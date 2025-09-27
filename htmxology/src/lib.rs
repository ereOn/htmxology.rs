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

#[cfg(feature = "templating")]
mod templating;

#[cfg(feature = "web-components")]
pub mod web_components;

pub use caching::{
    Cache, CacheControl, CachingResponseExt, Controller as CachingController,
    ControllerExt as CachingControllerExt,
};
pub use controller::Controller;
pub use route::{Route, RouteExt, decode_path_argument, replace_request_path};
#[cfg(feature = "auto-reload")]
pub use server::auto_reload::get_or_bind_tcp_listener;
pub use server::{
    ControllerRouter, ServeError, Server, ServerBuilder, ServerInfo, ServerOptions,
    ServerOptionsFromEnvError,
};

#[cfg(feature = "templating")]
pub use templating::RenderIntoResponse;

#[cfg(feature = "derive")]
pub use htmxology_macros::{DisplayDelegate, Route};
