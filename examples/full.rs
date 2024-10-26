//! Run with
//!
//! ```not_rust
//! just example fuul
//! ```

use askama::Template;
use axum::{extract::State, routing::get, Router};
use htmx_ssr::ArcState as HtmxState;
use tracing::info;

struct CustomState {
    foo: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    info!("Starting example `{}`...", env!("CARGO_BIN_NAME"));

    // Create a new user state.
    //
    // This can be any type that you want to store in the server state.
    //
    // It just has to be `Send + Sync + 'static`.
    let user_state = CustomState {
        foo: "bar".to_string(),
    };

    // Create a new server with auto-reload enabled by attempting to get a TCP listener from the
    // `listenfd` environment variable, falling back to binding to a local address if that fails.
    let mut server = htmx_ssr::Server::new_with_auto_reload("127.0.0.1:3000", user_state)
        .await?
        // Set the options on the server from the environment.
        .with_options_from_env()?;

    *server.router() = Router::new().route("/", get(handler));

    server.serve().await.map_err(Into::into)
}

#[derive(Template)]
#[template(path = "full.html.jinja")]
struct CustomTemplate {
    state: HtmxState<CustomState>,
}

async fn handler(State(state): State<HtmxState<CustomState>>) -> CustomTemplate {
    CustomTemplate {
        state: state.clone(),
    }
}
