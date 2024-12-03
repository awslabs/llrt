// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use rquickjs::{
    class::{Trace, Tracer},
    Value,
};
use std::ops::Deref;
use tokio::sync::Notify;

#[derive(Debug)]
pub struct Shared<T> {
    is_sent: AtomicBool,
    value: RwLock<Option<T>>,
    notify: Notify,
}

#[derive(Clone, Debug)]
pub struct Sender<T: Clone>(Arc<Shared<T>>);

impl<T: Clone> Deref for Sender<T> {
    type Target = Arc<Shared<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js> Trace<'js> for Sender<Value<'js>> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Ok(v) = self.0.value.read() {
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
            self.notify.notify_waiters();
        }
    }

    pub fn subscribe(&self) -> Receiver<T> {
        Receiver(Arc::clone(&self.0))
    }
}

#[derive(Clone, Debug)]
pub struct Receiver<T: Clone>(Arc<Shared<T>>);

impl<T: Clone> Deref for Receiver<T> {
    type Target = Arc<Shared<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> Receiver<T> {
    pub async fn recv(&self) -> T {
        if !self.is_sent.load(Ordering::Acquire) {
            self.notify.notified().await;
        }
        self.value.read().unwrap().clone().unwrap()
    }
}

pub fn channel<T: Clone>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        is_sent: AtomicBool::new(false),
        value: RwLock::new(None),
        notify: Notify::new(),
    });

    (Sender(Arc::clone(&shared)), Receiver(shared))
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
