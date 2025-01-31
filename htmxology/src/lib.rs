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

mod caching;
mod controller;
mod route;
mod server;
mod state;
mod templating;

pub use caching::{
    Cache, CacheControl, CachingResponseExt, Controller as CachingController,
    ControllerExt as CachingControllerExt,
};
pub use controller::Controller;
pub use route::{decode_path_argument, replace_request_path, Route};
pub use server::{
    ServeError, Server, ServerBuilder, ServerInfo, ServerOptions, ServerOptionsFromEnvError,
};
pub use state::ServerState;
pub use templating::RenderIntoResponse;

#[cfg(feature = "derive")]
pub use htmxology_macros::{DisplayDelegate, Fragment, Route};
