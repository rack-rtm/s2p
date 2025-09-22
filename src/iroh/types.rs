use super::dns_resolver::DnsResolver;
use super::node_authenticator::NodeAuthenticator;
use super::socket_factory::SocketFactory;
use derive_builder::Builder;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct S2pProtocol {
    #[builder(default)]
    pub proxy_timeouts: ProxyTimeouts,
    #[builder(default = "super::socket_factory::DefaultSocketFactory::arc()")]
    pub socket_factory: Arc<dyn SocketFactory>,
    #[builder(default = "super::node_authenticator::AllowAllNodeAuthenticator::arc()")]
    pub node_authenticator: Arc<dyn NodeAuthenticator>,
    #[builder(default = "super::dns_resolver::DefaultDnsResolver::arc()")]
    pub dns_resolver: Arc<dyn DnsResolver>,
}

#[derive(Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct ProxyTimeouts {
    #[builder(default = "Duration::from_secs(10)")]
    pub tcp_connection_timeout: Duration,
    #[builder(default = "Duration::from_secs(5)")]
    pub dns_resolution_timeout: Duration,
    #[builder(default = "Duration::from_secs(30)")]
    pub tcp_proxy_handshake_timeout: Duration,
}

impl S2pProtocol {
    pub fn new() -> Self {
        Self::builder().build().unwrap()
    }

    pub fn builder() -> S2pProtocolBuilder {
        S2pProtocolBuilder::default()
    }

    pub fn with_timeouts(proxy_timeouts: ProxyTimeouts) -> Self {
        Self::builder()
            .proxy_timeouts(proxy_timeouts)
            .build()
            .unwrap()
    }

    pub fn with_socket_factory(
        proxy_timeouts: ProxyTimeouts,
        socket_factory: Arc<dyn SocketFactory>,
    ) -> Self {
        Self::builder()
            .proxy_timeouts(proxy_timeouts)
            .socket_factory(socket_factory)
            .build()
            .unwrap()
    }

    pub fn with_node_authenticator(
        proxy_timeouts: ProxyTimeouts,
        node_authenticator: Arc<dyn NodeAuthenticator>,
    ) -> Self {
        Self::builder()
            .proxy_timeouts(proxy_timeouts)
            .node_authenticator(node_authenticator)
            .build()
            .unwrap()
    }

    pub fn with_socket_factory_and_node_authenticator(
        proxy_timeouts: ProxyTimeouts,
        socket_factory: Arc<dyn SocketFactory>,
        node_authenticator: Arc<dyn NodeAuthenticator>,
    ) -> Self {
        Self::builder()
            .proxy_timeouts(proxy_timeouts)
            .socket_factory(socket_factory)
            .node_authenticator(node_authenticator)
            .build()
            .unwrap()
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
