//! Run with
//!
//! ```not_rust
//! just example fuul
//! ```

use std::sync::Arc;

use askama::Template;
use axum::{extract::State, routing::get, Router};
use htmx_ssr::ArcState as HtmxState;
use tokio::sync::Mutex;
use tracing::info;

struct CustomState {
    current_text: Arc<Mutex<usize>>,
    texts: Vec<&'static str>,
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
        current_text: Default::default(),
        texts: vec!["Alpha", "Beta", "Gamma", "Delta"],
    };

    // Create a new server with auto-reload enabled by attempting to get a TCP listener from the
    // `listenfd` environment variable, falling back to binding to a local address if that fails.
    let mut server = htmx_ssr::Server::new_with_auto_reload("127.0.0.1:3000", user_state)
        .await?
        // Set the options on the server from the environment.
        .with_options_from_env()?;

    *server.router() = Router::new()
        .route("/", get(handler))
        .route("/api/toggle", get(toggle_handler));

    server.serve().await.map_err(Into::into)
}

#[derive(Template)]
#[template(path = "full/index.html.jinja")]
struct IndexView {
    button: ButtonView,
}

#[derive(Template)]
#[template(path = "full/button.html.jinja")]
struct ButtonView {
    text: String,
}

async fn handler(State(state): State<HtmxState<CustomState>>) -> IndexView {
    let text = {
        let current_text = state.user_state.current_text.lock().await;
        state.user_state.texts[*current_text % state.user_state.texts.len()]
    }
    .to_string();

    IndexView {
        button: ButtonView { text },
    }
}

async fn toggle_handler(State(state): State<HtmxState<CustomState>>) -> ButtonView {
    let text = {
        let mut current_text = state.user_state.current_text.lock().await;
        let text = state.user_state.texts[*current_text % state.user_state.texts.len()];
        *current_text += 1;
        text
    }
    .to_string();

    ButtonView { text }
}
