// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{self, Poll},
    time::{Duration, Instant},
    vec,
};

use hyper_util::client::legacy::connect::{dns::Name, HttpConnector};
use quick_cache::sync::Cache;
use tokio::sync::Semaphore;
use tower_service::Service;

#[derive(Clone)]
pub struct SocketAddrs {
    iter: vec::IntoIter<SocketAddr>,
}

impl Iterator for SocketAddrs {
    type Item = SocketAddr;
    #[inline]
    fn next(&mut self) -> Option<SocketAddr> {
        self.iter.next()
    }
}

#[derive(Clone)]
struct CacheEntry {
    ttl: Instant,
    addrs: SocketAddrs,
}

#[derive(Clone)]
struct CacheConcurrencyGuard {
    semaphore: Arc<Semaphore>,
    entry: Option<CacheEntry>,
}
impl CacheConcurrencyGuard {
    fn new(permits: u8) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits as usize)),
            entry: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CachedDnsResolver {
    cache: Arc<Cache<Name, CacheConcurrencyGuard>>,
    concurrency: u8,
    ttl: Duration,
}

impl Service<Name> for CachedDnsResolver {
    type Response = SocketAddrs;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = std::io::Result<Self::Response>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, name: Name) -> Self::Future {
        let cache = self.cache.clone();
        let permits = self.concurrency;
        let ttl = self.ttl;

        Box::pin(async move {
            let guard = match cache.get_value_or_guard_async(&name).await {
                Ok(guard) => guard,
                Err(placeholder) => {
                    let guard = CacheConcurrencyGuard::new(permits);
                    _ = placeholder.insert(guard.clone());
                    guard
                },
            };
            if let Some(entry) = guard.entry {
                if entry.ttl > Instant::now() {
                    return Ok(entry.addrs);
                }
            };

            let semaphore = guard.semaphore;
            let semaphore2 = semaphore.clone();
            let lock = semaphore2.acquire().await.unwrap();

            if let Some(item) = cache.get(&name).and_then(|guard| guard.entry) {
                return Ok(item.addrs);
            }

            let addrs = tokio::net::lookup_host((name.as_str(), 0)).await?;
            let addrs = addrs.collect::<Vec<_>>();
            let addrs = SocketAddrs {
                iter: addrs.into_iter(),
            };
            let addrs2 = addrs.clone();
            let entry = CacheEntry {
                ttl: Instant::now() + ttl,
                addrs,
            };
            cache.insert(
                name,
                CacheConcurrencyGuard {
                    semaphore,
                    entry: Some(entry),
                },
            );
            drop(lock);
            Ok(addrs2)
        })
    }
}

impl Default for CachedDnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CachedDnsResolver {
    pub fn new() -> Self {
        Self::with_options(128, 2, 300)
    }

    pub fn with_options(size: usize, concurrency: u8, ttl: u64) -> Self {
        Self {
            cache: Arc::new(Cache::new(size)),
            concurrency,
            ttl: Duration::from_secs(ttl),
        }
    }

    pub fn into_http_connector(self) -> HttpConnector<Self> {
        HttpConnector::<Self>::new_with_resolver(self)
    }
}
