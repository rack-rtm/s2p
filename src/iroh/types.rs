use std::time::Duration;

#[derive(Debug, Clone)]
pub struct S2pProtocol {
    pub proxy_timeouts: ProxyTimeouts,
}

#[derive(Debug, Clone)]
pub struct ProxyTimeouts {
    pub tcp_connection_timeout: Duration,
    pub dns_resolution_timeout: Duration,
    pub tcp_proxy_handshake_timeout: Duration,
}

impl S2pProtocol {
    pub fn new() -> Self {
        Self::with_timeouts(ProxyTimeouts::default())
    }
    pub fn with_timeouts(proxy_timeouts: ProxyTimeouts) -> Self {
        Self { proxy_timeouts }
    }
}

impl Default for ProxyTimeouts {
    fn default() -> Self {
        Self {
            tcp_connection_timeout: Duration::from_secs(10),
            dns_resolution_timeout: Duration::from_secs(5),
            tcp_proxy_handshake_timeout: Duration::from_secs(30),
        }
    }
}
