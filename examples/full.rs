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

    let router = Router::new().route("/", get(handler));
    let server = htmx_ssr::Server::new_with_auto_reload("127.0.0.1:3000", router).await?;

    server.serve().await.map_err(Into::into)
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
