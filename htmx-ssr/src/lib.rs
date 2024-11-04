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

pub mod htmx;

mod controller;
mod server;
mod state;

pub use controller::{Controller, ViewMapper};
pub use server::{ServeError, Server, ServerOptions, ServerOptionsFromEnvError};
pub use state::State;

#[cfg(feature = "derive")]
pub use htmx_ssr_macros::{Controller, DisplayDelegate, Fragment};
