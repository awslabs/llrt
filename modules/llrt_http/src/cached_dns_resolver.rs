use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::net::{SocketAddr, ToSocketAddrs};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{self, Poll};
use std::time::{Duration, Instant};
use std::{fmt, io, vec};

use hyper_util::client::legacy::connect::dns::Name;
use hyper_util::client::legacy::connect::HttpConnector;
use tokio::sync::Semaphore;
use tokio::{sync::RwLock, task::JoinHandle};
use tower_service::Service;

#[derive(Clone)]
struct CacheEntry {
    timestamp: Instant,
    gai_addrs: GaiAddrs,
}

struct CacheLock {
    semaphore: Arc<Semaphore>,
    entry: Option<CacheEntry>,
}

/// A resolver using blocking `getaddrinfo` calls in a threadpool.
#[derive(Clone)]
pub struct CachedDnsResolver {
    cache: Arc<RwLock<HashMap<Name, Arc<CacheLock>>>>,
    cache_duration: Duration,
}

/// An iterator of IP addresses returned from `getaddrinfo`.
#[derive(Clone)]
pub struct GaiAddrs {
    inner: SocketAddrs,
}

/// A future to resolve a name returned by `GaiResolver`.
pub struct GaiFuture {
    inner: JoinHandle<Result<GaiAddrs, io::Error>>,
}

/// Error indicating a given string was not a valid domain name.
#[derive(Debug)]
pub struct InvalidNameError(());

impl fmt::Display for InvalidNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Not a valid domain name")
    }
}

impl Error for InvalidNameError {}

impl Default for CachedDnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CachedDnsResolver {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_duration: Duration::from_secs(300),
        }
    }

    pub fn into_http_connector(self) -> HttpConnector<Self> {
        HttpConnector::<Self>::new_with_resolver(self)
    }
}

const DNS_PERMITS: usize = 5;

impl Service<Name> for CachedDnsResolver {
    type Response = GaiAddrs;
    type Error = io::Error;
    type Future = GaiFuture;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, name: Name) -> Self::Future {
        let cache = self.cache.clone();
        let duration = self.cache_duration;
        let handle = tokio::task::spawn(async move {
            let entry = {
                let cache_read = cache.read().await;
                if let Some(entry) = cache_read.get(&name) {
                    entry.clone()
                } else {
                    drop(cache_read); // Release read lock before acquiring write lock
                    let mut cache_write = cache.write().await;
                    cache_write
                        .entry(name.clone())
                        .or_insert_with(|| {
                            Arc::new(CacheLock {
                                semaphore: Arc::new(Semaphore::new(DNS_PERMITS)),
                                entry: None,
                            })
                        })
                        .clone()
                }
            };

            if let Some(cache) = &entry.entry {
                if cache.timestamp.elapsed() < duration {
                    return Ok(cache.gai_addrs.clone());
                }
            }

            let lock = entry.semaphore.acquire().await.unwrap();

            if let Some(entry) = cache.read().await.get(&name).and_then(|v| v.entry.clone()) {
                return Ok(entry.gai_addrs.clone());
            }

            let name2 = name.clone();

            let address = tokio::task::spawn_blocking(move || {
                (name2.as_str(), 0)
                    .to_socket_addrs()
                    .map(|i| SocketAddrs { iter: i })
            })
            .await;

            let addres = match address {
                Ok(Ok(addrs)) => {
                    let gai_addrs = GaiAddrs { inner: addrs };

                    let mut write = cache.write().await;
                    write.insert(
                        name,
                        Arc::new(CacheLock {
                            semaphore: Arc::new(Semaphore::new(DNS_PERMITS)),
                            entry: Some(CacheEntry {
                                timestamp: Instant::now(),
                                gai_addrs: gai_addrs.clone(),
                            }),
                        }),
                    );
                    Ok(gai_addrs)
                },
                Ok(Err(err)) => Err(err),
                Err(join_err) => {
                    if join_err.is_cancelled() {
                        Err(io::Error::new(io::ErrorKind::Interrupted, join_err))
                    } else {
                        panic!("gai background task failed: {:?}", join_err)
                    }
                },
            };
            drop(lock);
            addres
        });

        GaiFuture { inner: handle }
    }
}

impl fmt::Debug for CachedDnsResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("GaiResolver")
    }
}

impl Future for GaiFuture {
    type Output = Result<GaiAddrs, io::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|res| match res {
            Ok(Ok(addrs)) => Ok(addrs),
            Ok(Err(err)) => Err(err),
            Err(join_err) => {
                if join_err.is_cancelled() {
                    Err(io::Error::new(io::ErrorKind::Interrupted, join_err))
                } else {
                    panic!("gai background task failed: {:?}", join_err)
                }
            },
        })
    }
}

impl fmt::Debug for GaiFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("GaiFuture")
    }
}

impl Drop for GaiFuture {
    fn drop(&mut self) {
        self.inner.abort();
    }
}

impl Iterator for GaiAddrs {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl fmt::Debug for GaiAddrs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("GaiAddrs")
    }
}

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
