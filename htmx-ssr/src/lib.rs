//! HTMX-SSR
//!
//! Server-side rendering framework for Rust, using HTMX.
//!
//! # Features
//!
//! - `auto-reload`: Automatically reload the server when the source code changes. Useful for
//!   development. **Not enabled by default.**
//! - `interfaces`: Enrich the local base URL guessing logic with the ability to inspect the
//!   workstation's network interfaces. Useful for development. **Not enabled by default.**

mod server;
mod state;

pub use server::{ServeError, Server, ServerOptions, ServerOptionsFromEnvError};
pub use state::State;

/// A cloneable, thread-safe reference-counted pointer to the server state.
pub type ArcState<T> = std::sync::Arc<State<T>>;
