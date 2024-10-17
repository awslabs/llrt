// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::Notify;

#[derive(Default)]
pub struct Latch {
    count: AtomicUsize,
    notify: Notify,
    inited: AtomicBool,
}

impl Latch {
    pub fn increment(&self) {
        self.inited.store(true, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement(&self) {
        let previous = self.count.fetch_sub(1, Ordering::Relaxed);
        if previous == 1 {
            self.notify.notify_waiters();
        }
    }

    pub async fn wait(&self) {
        if !self.inited.load(Ordering::Relaxed) || self.count.load(Ordering::Relaxed) > 0 {
            self.notify.notified().await;
        }
    }
}
