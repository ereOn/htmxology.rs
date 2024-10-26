//! Run with
//!
//! ```not_rust
//! just example fuul
//! ```

use axum::{response::Html, routing::get, Router};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    info!("Starting example `{}`...", env!("CARGO_BIN_NAME"));

    // Create a new server with auto-reload enabled by attempting to get a TCP listener from the
    // `listenfd` environment variable, falling back to binding to a local address if that fails.
    let mut server = htmx_ssr::Server::new_with_auto_reload("127.0.0.1:3000")
        .await?
        // Set the options on the server from the environment.
        .with_options_from_env()?;

    *server.router() = Router::new().route("/", get(handler));

    server.serve().await.map_err(Into::into)
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
