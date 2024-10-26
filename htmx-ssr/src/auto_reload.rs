//! Auto-reload facilities.
//!
//! This module provides a way to automatically reload the server when the source code changes all
//! the while maintaining connections.
//!
//! This leverages [systemfd](https://github.com/mitsuhiko/systemfd) and its associated crate
//! [listenfd](https://github.com/mitsuhiko/listenfd).
//!
//! The code this wraps is by no means complex and is merely provided as a convenience.

use listenfd::ListenFd;
use tokio::net::{TcpListener, ToSocketAddrs};

/// An error that can occur when trying to get a TCP listener.
#[derive(Debug, thiserror::Error)]
pub enum GetTcpListenerError {
    /// An error occurred while trying to get a listener from `listenfd`.
    #[error("failed to get a listener from `listenfd`: {0}")]
    ListenFd(std::io::Error),

    /// An error occurred while trying to set the TCP listener to non-blocking mode.
    #[error("failed to set the TCP listener to non-blocking mode: {0}")]
    SetNonblocking(std::io::Error),

    /// An error occurred while trying to build a TCP listener from a standard listener.
    #[error("failed to build a TCP listener from a standard listener: {0}")]
    FromStd(std::io::Error),

    /// An error occurred while trying to bind to a local address.
    #[error("failed to bind to a local address: {0}")]
    Bind(std::io::Error),
}

/// Get a TCP listener from the environment by either taking it from the listen fd or binding to a
/// local address, as a fallback.
///
/// This is merely a convenience function that abstracts away the details of `listenfd`.
pub async fn get_or_bind_tcp_listener(
    addr: impl ToSocketAddrs,
) -> Result<TcpListener, GetTcpListenerError> {
    let mut listenfd = ListenFd::from_env();

    tracing::debug!("Attempting to get listener from `listenfd`...");

    match listenfd
        .take_tcp_listener(0)
        .map_err(GetTcpListenerError::ListenFd)?
    {
        Some(listener) => {
            tracing::debug!("Got listener from `listenfd`.");

            listener
                .set_nonblocking(true)
                .map_err(GetTcpListenerError::SetNonblocking)?;

            TcpListener::from_std(listener).map_err(GetTcpListenerError::FromStd)
        }
        None => {
            tracing::debug!("Got no listener from `listenfd`, falling back to binding.");

            TcpListener::bind(addr)
                .await
                .map_err(GetTcpListenerError::Bind)
        }
    }
}
