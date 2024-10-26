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

    let app = Router::new().route("/", get(handler));

    let listener = htmx_ssr::auto_reload::get_or_bind_tcp_listener("127.0.0.1:3000").await?;
    let local_addr = listener.local_addr()?;

    info!("Listening on {local_addr}.");

    axum::serve(listener, app).await.map_err(Into::into)
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
