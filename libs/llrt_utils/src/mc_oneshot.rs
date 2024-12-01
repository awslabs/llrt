// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    task::{Context, Poll, Waker},
};

use rquickjs::{
    class::{Trace, Tracer},
    Value,
};

#[derive(Clone, Debug)]
pub struct Sender<T: Clone> {
    is_sent: Arc<AtomicBool>,
    value: Arc<RwLock<Option<T>>>,
    wakers: Arc<RwLock<Vec<Waker>>>,
}

impl<'js> Trace<'js> for Sender<Value<'js>> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Ok(v) = self.value.as_ref().read() {
            if let Some(v) = v.as_ref() {
                tracer.mark(v)
            }
        }
    }
}

impl<T: Clone> Sender<T> {
    pub fn send(&self, value: T) {
        if !self.is_sent.load(Ordering::Relaxed) {
            self.value.write().unwrap().replace(value);
            self.is_sent.store(true, Ordering::Release);
            if let Ok(wakers) = self.wakers.read() {
                for waker in wakers.iter() {
                    waker.wake_by_ref();
                }
            }
        }
    }

    pub fn subscribe(&self) -> Receiver<T> {
        Receiver {
            is_sent: self.is_sent.clone(),
            value: self.value.clone(),
            wakers: self.wakers.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Receiver<T: Clone> {
    is_sent: Arc<AtomicBool>,
    value: Arc<RwLock<Option<T>>>,
    wakers: Arc<RwLock<Vec<Waker>>>,
}

impl<T: Clone> Receiver<T> {
    pub fn recv(&self) -> ReceiverWaiter<T> {
        ReceiverWaiter {
            is_sent: self.is_sent.clone(),
            value: self.value.clone(),
            wakers: self.wakers.clone(),
        }
    }
}

pub struct ReceiverWaiter<T: Clone> {
    is_sent: Arc<AtomicBool>,
    value: Arc<RwLock<Option<T>>>,
    wakers: Arc<RwLock<Vec<Waker>>>,
}

impl<T: Clone> Future for ReceiverWaiter<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_sent.load(Ordering::Acquire) {
            let a = self.get_mut().value.read().unwrap().clone().unwrap();
            return Poll::Ready(a);
        }

        // Register waker only if value not ready
        if let Ok(mut wakers) = self.wakers.write() {
            wakers.push(cx.waker().clone());
        }

        Poll::Pending
    }
}

pub fn channel<T: Clone>() -> (Sender<T>, Receiver<T>) {
    let is_sent = Arc::new(AtomicBool::new(false));
    let value = Arc::new(RwLock::new(None));
    let wakers = Arc::new(RwLock::new(Vec::new()));

    (
        Sender {
            is_sent: is_sent.clone(),
            value: value.clone(),
            wakers: wakers.clone(),
        },
        Receiver {
            is_sent: is_sent.clone(),
            value: value.clone(),
            wakers: wakers.clone(),
        },
    )
}

#[cfg(test)]
mod tests {
    use tokio::join;

    #[tokio::test]
    async fn test() {
        let (tx, rx1) = super::channel::<bool>();

        let rx2 = tx.subscribe();
        let rx3 = tx.subscribe();

        let a = tokio::spawn(async move {
            let val = rx1.recv().await; //wait for value to become false
            assert!(val)
        });

        let b = tokio::spawn(async move {
            let val = rx2.recv().await; //wait for value to become false
            assert!(val)
        });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        tx.send(true);

        let val = rx3.recv().await;
        assert!(val);

        let (a, b) = join!(a, b);
        a.unwrap();
        b.unwrap();
    }
}
