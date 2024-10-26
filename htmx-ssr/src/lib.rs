//! HTMX-SSR
//!
//! Server-side rendering framework for Rust, using HTMX.

use std::{future::Future, pin::Pin};

#[cfg(feature = "auto-reload")]
pub mod auto_reload;

/// The main struct for the HTMX-SSR framework.
///
/// Represents a running HTMX-SSR server.
pub struct Server {
    /// The TCP listener that the server is using.
    listener: tokio::net::TcpListener,

    /// The Axum router that the server is using.
    router: axum::Router,

    /// The graceful shutdown signal.
    graceful_shutdown: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

/// An error that can occur when instantiating a new HTMX-SSR server with auto-reload features.
#[cfg(feature = "auto-reload")]
#[derive(Debug, thiserror::Error)]
pub enum NewWithAutoReloadError {
    /// An error occurred while trying to get a TCP listener.
    #[error("failed to get a TCP listener: {0}")]
    GetTcpListener(#[from] auto_reload::GetTcpListenerError),

    /// An error occurred while trying to get the local address of the listener.
    #[error("failed to get the local address of the listener: {0}")]
    LocalAddr(#[from] std::io::Error),
}

/// An error that can occur when trying to serve the application.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// An error occurred while trying to serve the application.
    #[error("failed to serve the application: {0}")]
    Io(#[from] std::io::Error),
}

impl Server {
    /// Instantiate a new HTMX-SSR server.
    pub fn new(listener: tokio::net::TcpListener, router: axum::Router) -> Self {
        let graceful_shutdown = None;

        Self {
            listener,
            router,
            graceful_shutdown,
        }
    }

    /// Instantiate a new HTMX-SSR server with all the auto-reload features enabled.
    ///
    /// Attempts to get a TCP listener from the environment if run through `listenfd`, falling
    /// back to binding to a local address if that fails.
    ///
    /// Also sets the graceful shutdown signal to `ctrl-c`.
    #[cfg(feature = "auto-reload")]
    pub async fn new_with_auto_reload(
        addr: impl tokio::net::ToSocketAddrs,
        router: axum::Router,
    ) -> Result<Self, NewWithAutoReloadError> {
        let listener = auto_reload::get_or_bind_tcp_listener(addr).await?;

        let local_addr = listener.local_addr()?;

        tracing::info!("HTMX SSR server listening on {local_addr}.");

        Ok(Self::new(listener, router).with_ctrl_c_graceful_shutdown())
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

    /// Serve the application.
    pub async fn serve(self) -> Result<(), ServeError> {
        let serve = axum::serve(self.listener, self.router);

        match self.graceful_shutdown {
            Some(signal) => serve.with_graceful_shutdown(signal).await,
            None => serve.await,
        }
        .map_err(Into::into)
    }
}
