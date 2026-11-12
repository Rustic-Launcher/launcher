//! The seam the desktop app (`rusticmc-servers`) uses to expose a local server: a
//! [`TunnelProvider`] starts a tunnel for a server and returns its handle (events, status, stop).
//! The relay is one provider; others (a direct UPnP mapping, an invite-code tunnel) can implement
//! the same trait.
use crate::{TlsRoots, TokenSource, Tunnel, TunnelConfig, TunnelError, TunnelHandle};
use std::{net::SocketAddr, sync::Arc};

/// What the app asks for.
#[derive(Clone)]
pub struct TunnelRequest {
    /// The local server's game port, usually `127.0.0.1:<port>`.
    pub local_addr: SocketAddr,
    /// Tunnel tokens for this server (the app fetches them from Rustic Cloud).
    pub token_source: Arc<dyn TokenSource>,
    /// Whether the local server reads a PROXY protocol v2 header (Horizon does not).
    pub proxy_protocol: bool,
}

/// Starts tunnels; implemented by [`RelayTunnelProvider`].
pub trait TunnelProvider: Send + Sync {
    /// Start a tunnel on the current Tokio runtime.
    fn start(&self, request: TunnelRequest) -> Result<TunnelHandle, TunnelError>;
}

/// Tunnels through a Rustic Relay.
#[derive(Clone, Debug)]
pub struct RelayTunnelProvider {
    /// `host:port` of the relay's tunnel endpoint.
    pub relay_addr: String,
    pub tls_server_name: Option<String>,
    pub roots: TlsRoots,
}

impl RelayTunnelProvider {
    pub fn new(relay_addr: impl Into<String>) -> Self {
        Self {
            relay_addr: relay_addr.into(),
            tls_server_name: None,
            roots: TlsRoots::Native,
        }
    }
}

impl TunnelProvider for RelayTunnelProvider {
    fn start(&self, request: TunnelRequest) -> Result<TunnelHandle, TunnelError> {
        let mut config = TunnelConfig::new(
            self.relay_addr.clone(),
            request.local_addr,
            request.token_source,
        );
        config.tls_server_name = self.tls_server_name.clone();
        config.roots = self.roots.clone();
        config.proxy_protocol = request.proxy_protocol;
        Tunnel::connect(config)
    }
}
