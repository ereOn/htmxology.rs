use std::net::SocketAddr;

use super::ServerOptions;

/// The HTMX state.
#[derive(Debug)]
pub struct State<T> {
    /// The base URL of the server.
    pub base_url: http::Uri,

    /// The user-defined state.
    pub user_state: T,
}

impl<T> State<T> {
    /// Get the base URL.
    fn base_url(options: ServerOptions, local_addr: SocketAddr) -> http::Uri {
        match options.base_url {
            Some(base_url) => base_url,
            None => {
                tracing::info!("No base URL set, guessing from local address `{local_addr}`...");

                if local_addr.ip().is_unspecified() {
                    // If the local address is unspecified, we have to enumerate the network
                    // interfaces and take an address from one of them.
                    tracing::warn!("Local address is unspecified, guessing from network interfaces... This is likely not what you want.");

                    let localhost_base_url = format!("http://localhost:{}", local_addr.port())
                        .parse()
                        .expect("hardcoded URL is valid");

                    #[cfg(feature = "interfaces")]
                    match netdev::get_default_interface() {
                        Ok(interface) => {
                            tracing::info!(
                                "Using default network interface `{}` for the base URL.",
                                interface.friendly_name.unwrap_or(interface.name)
                            );

                            match interface
                                .ipv4
                                .into_iter()
                                .map(|ip_v4| ip_v4.addr().to_string())
                                .chain(
                                    interface
                                        .ipv6
                                        .into_iter()
                                        .map(|ip_v6| ip_v6.addr().to_string()),
                                )
                                .next()
                            {
                                Some(ip) => {
                                    return format!("http://{}:{}", ip, local_addr.port())
                                        .parse()
                                        .expect("hardcoded URL is valid");
                                }
                                None => {
                                    tracing::error!(
                                        "No IP address found for the default network interface."
                                    );
                                }
                            };
                        }
                        Err(err) => {
                            tracing::error!("Failed to determine default network interface: {err}");
                        }
                    }

                    localhost_base_url
                } else {
                    format!("http://{}:{}", local_addr.ip(), local_addr.port())
                        .parse()
                        .expect("hardcoded URL is valid")
                }
            }
        }
    }

    /// Create a new server state from the given options and local address.
    pub(super) fn new(options: ServerOptions, local_addr: SocketAddr, user_state: T) -> Self {
        let base_url = Self::base_url(options, local_addr);

        Self {
            base_url,
            user_state,
        }
    }
}
