use std::{
    cmp::min,
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use tokio::sync::{Notify, Semaphore};

#[derive(Clone)]
pub struct BytearrayBuffer {
    inner: Arc<Mutex<VecDeque<u8>>>,
    max_capacity: Arc<AtomicUsize>,
    len: Arc<AtomicUsize>,
    notify: Arc<Notify>,
    closed: Arc<AtomicBool>,
    write_semaphore: Arc<Semaphore>,
}

impl BytearrayBuffer {
    pub fn new(capacity: usize) -> Self {
        let queue = VecDeque::with_capacity(capacity);
        let capacity = queue.capacity();
        Self {
            inner: Arc::new(Mutex::new(queue)),
            len: Arc::new(AtomicUsize::new(0)),
            max_capacity: Arc::new(AtomicUsize::new(capacity)),
            notify: Arc::new(Notify::new()),
            closed: Arc::new(AtomicBool::new(false)),
            write_semaphore: Arc::new(Semaphore::new(1)),
        }
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    pub fn write_forced(&self, item: &[u8]) {
        let mut inner = self.inner.lock().unwrap();
        inner.extend(item);
        let capacity = inner.capacity();
        self.len.fetch_add(item.len(), Ordering::Relaxed);
        self.max_capacity.store(capacity, Ordering::Relaxed);
    }

    pub async fn write(&self, item: &mut [u8]) -> usize {
        let _ = self.write_semaphore.acquire().await.unwrap();
        let mut slice_index = 0;
        loop {
            let max_capacity = self.max_capacity.load(Ordering::Relaxed);
            if self.closed.load(Ordering::Relaxed) {
                return max_capacity;
            }

            let len = self.len.load(Ordering::Relaxed);

            let available = max_capacity - len;

            if available > 0 {
                let end_index = min(item.len() - 1, slice_index + available - 1);
                let sub_slice = &item[slice_index..=end_index];
                let slice_length = sub_slice.len();
                slice_index += slice_length;

                self.inner.lock().unwrap().extend(sub_slice);
                self.len.fetch_add(slice_length, Ordering::Relaxed);

                if slice_index == item.len() {
                    return max_capacity;
                }
            }
            self.notify.notified().await;
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    pub async fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
        self.notify.notify_one();
        //wait for write to finish
        let _ = self.write_semaphore.acquire().await.unwrap();
    }

    pub async fn clear(&self) {
        self.closed.store(false, Ordering::Relaxed);
        self.notify.notify_one();
        //wait for write to finish
        let _ = self.write_semaphore.acquire().await.unwrap();
        self.len.store(0, Ordering::Relaxed);
        self.inner.lock().unwrap().clear();
        self.closed.store(false, Ordering::Relaxed);
    }

    pub fn read(&self, desired_size: Option<usize>) -> Option<Vec<u8>> {
        let mut inner = self.inner.lock().unwrap();
        let done = self.closed.load(Ordering::Relaxed);

        let items = if done {
            Some(inner.drain(0..).collect())
        } else if let Some(desired_len) = desired_size {
            let max_capacity = self.max_capacity.load(Ordering::Relaxed);
            if desired_len > max_capacity {
                let diff = desired_len - max_capacity;
                inner.reserve(diff - 1);
                let mut max_capacity = inner.capacity();
                if desired_len > max_capacity {
                    inner.reserve(desired_len - max_capacity);
                    max_capacity = inner.capacity();
                }
                drop(inner);
                self.max_capacity.store(max_capacity, Ordering::Relaxed);
                self.notify.notify_one();
                return None;
            }

            let len = self.len.load(Ordering::Relaxed);
            if desired_len > len {
                self.notify.notify_one();
                return None;
            }

            Some(inner.drain(0..desired_len).collect())
        } else {
            Some(inner.drain(0..).collect())
        };
        self.len.store(inner.len(), Ordering::Relaxed);
        drop(inner);
        self.notify.notify_one();
        items
    }
}

#[cfg(test)]
mod tests {
    use crate::bytearray_buffer::BytearrayBuffer;

    #[tokio::test]
    async fn clear_while_writing() {
        let queue = BytearrayBuffer::new(8);
        let queue2 = queue.clone();

        tokio::task::spawn(async move {
            let mut vec: Vec<u8> = (0..=255).collect();
            queue.write(&mut vec).await;
        });

        queue2.clear().await
    }

    #[tokio::test]
    async fn write_one_at_a_time() {
        let queue = BytearrayBuffer::new(8);
        let queue2 = queue.clone();
        let queue3 = queue.clone();

        tokio::task::spawn(async move {
            let mut vec: Vec<u8> = (0..=127).collect();
            queue.write(&mut vec).await;
        });

        tokio::task::spawn(async move {
            let mut vec: Vec<u8> = (128..=255).collect();
            queue2.write(&mut vec).await;
        });

        let mut data = Vec::<u8>::new();

        loop {
            tokio::task::yield_now().await;
            if let Some(bytes) = queue3.read(Some(256)) {
                data.extend(bytes);
                break;
            }
        }

        //assert that data in vec is increment from 0 to 255
        for i in 0..=255 {
            assert_eq!(data[i as usize], i);
        }
    }

    #[tokio::test]
    async fn queue() {
        let queue = BytearrayBuffer::new(8);
        let queue2 = queue.clone();

        let write_task = tokio::task::spawn(async move {
            for _ in 0..=255 {
                let mut vec: Vec<u8> = (0..=255).collect();
                queue.write(&mut vec).await;
            }
            queue.close().await;
        });

        let mut data = Vec::<u8>::new();

        loop {
            let done = queue2.is_closed();

            tokio::task::yield_now().await;
            if let Some(bytes) = queue2.read(Some(9)) {
                data.extend(bytes);
            }
            if done {
                break;
            }
        }

        let _ = write_task.await;

        assert_eq!(data.len(), 256 * 256)
    }
}
