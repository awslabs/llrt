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

#[derive(Debug)]
struct Shared<T> {
    is_sent: AtomicBool,
    value: RwLock<Option<T>>,
    wakers: RwLock<Vec<Waker>>,
}

#[derive(Clone, Debug)]
pub struct Sender<T: Clone> {
    shared: Arc<Shared<T>>,
}

impl<'js> Trace<'js> for Sender<Value<'js>> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Ok(v) = self.shared.value.read() {
            if let Some(v) = v.as_ref() {
                tracer.mark(v)
            }
        }
    }
}

impl<T: Clone> Sender<T> {
    pub fn send(&self, value: T) {
        if !self.shared.is_sent.load(Ordering::Relaxed) {
            self.shared.value.write().unwrap().replace(value);
            self.shared.is_sent.store(true, Ordering::Release);
            if let Ok(wakers) = self.shared.wakers.read() {
                for waker in wakers.iter() {
                    waker.wake_by_ref();
                }
            }
        }
    }

    pub fn subscribe(&self) -> Receiver<T> {
        Receiver {
            shared: Arc::clone(&self.shared),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Receiver<T: Clone> {
    shared: Arc<Shared<T>>,
}

impl<T: Clone> Receiver<T> {
    pub fn recv(&self) -> ReceiverWaiter<T> {
        ReceiverWaiter {
            shared: Arc::clone(&self.shared),
        }
    }
}

pub struct ReceiverWaiter<T: Clone> {
    shared: Arc<Shared<T>>,
}

impl<T: Clone> Future for ReceiverWaiter<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.shared.is_sent.load(Ordering::Acquire) {
            let value = self.shared.value.read().unwrap().clone().unwrap();
            return Poll::Ready(value);
        }

        // Register waker only if value not ready
        if let Ok(mut wakers) = self.shared.wakers.write() {
            wakers.push(cx.waker().clone());
        }

        Poll::Pending
    }
}

pub fn channel<T: Clone>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        is_sent: AtomicBool::new(false),
        value: RwLock::new(None),
        wakers: RwLock::new(Vec::new()),
    });

    (
        Sender {
            shared: Arc::clone(&shared),
        },
        Receiver { shared },
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
