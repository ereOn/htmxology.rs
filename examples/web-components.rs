//! Run with
//!
//! ```not_rust
//! just example web-components
//! ```

use controller::MainController;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    info!("Starting example `{}`...", env!("CARGO_BIN_NAME"));

    // Create a new server with auto-reload enabled by attempting to get a TCP listener from the
    // `listenfd` environment variable, falling back to binding to a local address if that fails.
    let server = htmxology::Server::builder_with_auto_reload("127.0.0.1:3000")
        .await?
        // Set the options on the server from the environment.
        .with_options_from_env()?
        .build();

    server
        .serve(MainController::default())
        .await
        .map_err(Into::into)
}

mod views {
    use askama::Template;

    /// The index page.
    #[derive(Template)]
    #[template(path = "web-components/index.html.jinja")]
    pub(super) struct Index;
}

mod controller {

    use super::views;
    use axum::response::IntoResponse;
    use htmxology::{Controller, htmx::Request as HtmxRequest};
    use htmxology::{RenderIntoResponse, Route, ServerInfo};

    /// The main application routes.
    #[derive(Debug, Clone, Route)]
    pub enum AppRoute {
        /// The home route.
        #[route("")]
        Home,
    }

    /// The main controller implementation.
    #[derive(Debug, Clone, Default)]
    pub struct MainController {}

    /// Custom implementation.
    impl Controller for MainController {
        type Route = AppRoute;

        async fn render_view(
            &self,
            route: AppRoute,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
        ) -> axum::response::Response {
            match route {
                AppRoute::Home => views::Index.render_into_response(),
            }
        }
    }
}
