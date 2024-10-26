//! Run with
//!
//! ```not_rust
//! just example fuul
//! ```

use axum::{extract::State, response::Html, routing::get, Router};
use htmx_ssr::ServerState;
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

async fn handler(State(state): State<ServerState<CustomState>>) -> Html<String> {
    Html(format!(
        r#"<h1>Hello, HTMX SSR!</h1>
<p>Our base URL is: <a href="{0}">{0}</a></p>
<p>My <code>foo</code> is: <code>{1}</code></p>
"#,
        state.base_url, state.user_state.foo,
    ))
}
