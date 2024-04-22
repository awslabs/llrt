// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    task::{Context, Poll},
};

#[derive(Clone, Debug)]
pub struct Sender<T: Clone> {
    is_sent: Arc<AtomicBool>,
    value: Arc<RwLock<Option<T>>>,
}

impl<T: Clone> Sender<T> {
    pub fn send(&self, value: T) {
        if !self.is_sent.load(Ordering::Relaxed) {
            self.value.write().unwrap().replace(value);
            self.is_sent.store(true, Ordering::Relaxed);
        }
    }

    pub fn subscribe(&self) -> Receiver<T> {
        Receiver {
            is_sent: self.is_sent.clone(),
            value: self.value.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Receiver<T: Clone> {
    is_sent: Arc<AtomicBool>,
    value: Arc<RwLock<Option<T>>>,
}

impl<T: Clone> Receiver<T> {
    pub fn recv(&self) -> ReceiverWaiter<T> {
        ReceiverWaiter {
            is_sent: self.is_sent.clone(),
            value: self.value.clone(),
        }
    }
}

pub struct ReceiverWaiter<T: Clone> {
    is_sent: Arc<AtomicBool>,
    value: Arc<RwLock<Option<T>>>,
}

impl<T: Clone> Future for ReceiverWaiter<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_sent.load(Ordering::Relaxed) {
            let a = self.get_mut().value.read().unwrap().clone().unwrap();
            return Poll::Ready(a);
        }

        cx.waker().wake_by_ref();

        Poll::Pending
    }
}

pub fn channel<T: Clone>() -> (Sender<T>, Receiver<T>) {
    let is_sent = Arc::new(AtomicBool::new(false));
    let value = Arc::new(RwLock::new(None));

    (
        Sender {
            is_sent: is_sent.clone(),
            value: value.clone(),
        },
        Receiver {
            is_sent: is_sent.clone(),
            value: value.clone(),
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

        // tx.send(false);

        let a = tokio::spawn(async move {
            let val = rx1.recv().await; //wait for value to become false
            assert!(val)
        });

        let b = tokio::spawn(async move {
            let val = rx2.recv().await; //wait for value to become false
            assert!(val)
        });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        tx.send(false);

        rx3.recv().await;

        let _ = join!(a, b);
    }
}
