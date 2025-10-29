use std::{future::Future, net::SocketAddr, pin::Pin, sync::Arc};

#[cfg(feature = "auto-reload")]
pub mod auto_reload;

mod controller_router;
mod options;

use axum::Router;
pub use controller_router::ControllerRouter;
pub use options::{ServerOptions, ServerOptionsFromEnvError};

/// The server information.
///
/// This information is made available in the controller through request extensions.
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// The base URL of the server.
    pub base_url: http::Uri,
}

/// A server builder.
pub struct ServerBuilder {
    /// The TCP listener that the server is using.
    listener: tokio::net::TcpListener,

    /// The graceful shutdown signal.
    graceful_shutdown: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,

    /// The options for the server.
    options: ServerOptions,
}

/// The main struct for the HTMX-SSR framework.
///
/// Represents a running HTMX-SSR server.
pub struct Server {
    /// The TCP listener that the server is using.
    listener: tokio::net::TcpListener,

    /// The graceful shutdown signal.
    graceful_shutdown: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,

    /// The options for the server.
    options: ServerOptions,
}

/// An error that can occur when instantiating a new HTMX-SSR server with auto-reload features.
#[cfg(feature = "auto-reload")]
#[derive(Debug, thiserror::Error)]
pub enum NewWithAutoReloadError {
    /// An error occurred while trying to get a TCP listener.
    #[error("failed to get a TCP listener: {0}")]
    GetTcpListener(#[from] auto_reload::GetTcpListenerError),
}

/// An error that can occur when trying to serve the application.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// An error occurred while trying to serve the application.
    #[error("failed to serve the application: {0}")]
    Io(#[from] std::io::Error),

    /// An error occurred while trying to get the local address of the listener.
    #[error("failed to get the local address of the listener: {0}")]
    LocalAddr(std::io::Error),
}

impl ServerBuilder {
    /// Set the options on the server.
    pub fn with_options(mut self, options: ServerOptions) -> Self {
        self.options = options;
        self
    }

    /// Set the options on the server from the environment.
    pub fn with_options_from_env(mut self) -> Result<Self, ServerOptionsFromEnvError> {
        self.options = ServerOptions::from_env()?;

        Ok(self)
    }

    /// Set the graceful shutdown signal.
    pub fn with_graceful_shutdown(
        mut self,
        signal: impl Future<Output = ()> + Send + 'static,
    ) -> Self {
        self.graceful_shutdown = Some(Box::pin(signal));
        self
    }

    /// Set the graceful shutdown signal to `ctrl-c`.
    #[cfg(feature = "auto-reload")]
    pub fn with_ctrl_c_graceful_shutdown(self) -> Self {
        self.with_graceful_shutdown(async move {
            tracing::info!("Listening for `ctrl-c` signal for graceful shutdown...");

            if let Err(err) = tokio::signal::ctrl_c().await {
                tracing::error!("Failed to register for `ctrl-c` signal: {err}");
            }

            tracing::info!("Received `ctrl-c` signal, shutting down gracefully.");
        })
    }

    /// Build the server.
    pub fn build(self) -> Server {
        Server {
            listener: self.listener,
            graceful_shutdown: self.graceful_shutdown,
            options: self.options,
        }
    }
}

impl Server {
    /// Get a builder for the server.
    pub fn builder(listener: tokio::net::TcpListener) -> ServerBuilder {
        ServerBuilder {
            listener,
            graceful_shutdown: None,
            options: Default::default(),
        }
    }

    /// Instantiate a new HTMX-SSR server with all the auto-reload features enabled.
    ///
    /// Attempts to get a TCP listener from the environment if run through `listenfd`, falling
    /// back to binding to a local address if that fails.
    ///
    /// Also sets the graceful shutdown signal to `ctrl-c`.
    #[cfg(feature = "auto-reload")]
    pub async fn builder_with_auto_reload(
        addr: impl tokio::net::ToSocketAddrs,
    ) -> Result<ServerBuilder, NewWithAutoReloadError> {
        let listener = auto_reload::get_or_bind_tcp_listener(addr).await?;

        Ok(Self::builder(listener).with_ctrl_c_graceful_shutdown())
    }

    /// Serve the specified controller.
    pub async fn serve<C>(self, controller: C) -> Result<(), ServeError>
    where
        C: super::Controller<Response = Result<axum::response::Response, axum::response::Response>>
            + 'static,
    {
        self.serve_with_router(ControllerRouter::new(controller))
            .await
    }

    /// Serve the specified controller router.
    ///
    /// Use this method to add custom routes to the server before serving it.
    pub async fn serve_with_router(self, router: ControllerRouter) -> Result<(), ServeError> {
        let local_addr = self.listener.local_addr().map_err(ServeError::LocalAddr)?;

        tracing::info!("HTMX SSR server listening on TCP/{local_addr}.");

        let base_url = match self.options.base_url {
            Some(base_url) => base_url,
            None => Self::guess_base_url(local_addr),
        };

        let server_info = Arc::new(ServerInfo { base_url });

        tracing::info!(
            "Now serving HTMX SSR server at `{}`...",
            server_info.base_url
        );

        let router: Router = router.into();
        let router = router.layer(axum::extract::Extension(server_info));

        let serve = axum::serve(self.listener, router);

        match self.graceful_shutdown {
            Some(signal) => serve.with_graceful_shutdown(signal).await,
            None => serve.await,
        }
        .map_err(Into::into)
    }

    /// Guess the base URL from the local address.
    fn guess_base_url(local_addr: SocketAddr) -> http::Uri {
        tracing::info!("No base URL set, guessing from local address `{local_addr}`...");

        if local_addr.ip().is_unspecified() {
            // If the local address is unspecified, we have to enumerate the network
            // interfaces and take an address from one of them.
            tracing::warn!(
                "Local address is unspecified, guessing from network interfaces... This is likely not what you want."
            );

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
