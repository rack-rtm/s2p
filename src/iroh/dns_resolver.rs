use std::future::Future;
use std::io;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;

pub trait DnsResolver: Send + Sync + std::fmt::Debug {
    fn lookup_host<'a>(
        &'a self,
        host: &'a str,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<IpAddr>>> + Send + 'a>>;
}

#[derive(Debug, Clone)]
pub struct DefaultDnsResolver;

impl DefaultDnsResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn arc() -> Arc<dyn DnsResolver> {
        Arc::new(Self::new())
    }
}

impl DnsResolver for DefaultDnsResolver {
    fn lookup_host<'a>(
        &'a self,
        host: &'a str,
    ) -> Pin<Box<dyn Future<Output = io::Result<Vec<IpAddr>>> + Send + 'a>> {
        Box::pin(async move {
            let addrs = tokio::net::lookup_host(host).await?;
            Ok(addrs.map(|addr| addr.ip()).collect())
        })
    }
}

impl Default for DefaultDnsResolver {
    fn default() -> Self {
        Self::new()
    }
}
