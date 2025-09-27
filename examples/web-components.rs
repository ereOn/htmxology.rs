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
    use htmxology::Route;

    use crate::controller::AppRoute;

    /// The index page.
    #[derive(Template)]
    #[template(path = "web-components/index.html.jinja")]
    pub(super) struct Index {
        /// The web components.
        pub web_components: htmxology::web_components::WebComponents,
    }

    /// The index page.
    #[derive(Template)]
    #[template(path = "web-components/web-components/my-element.html.jinja")]
    pub(super) struct MyElement;

    impl MyElement {
        /// Get the get route.
        pub fn get_route() -> AppRoute {
            AppRoute::WebComponents(crate::controller::WebComponentsRoute::MyElement(
                crate::controller::MyElementRoute::Get,
            ))
        }

        /// Render the view as a deferred component.
        pub fn render_deferred() -> String {
            format!(
                r#"<div {} hx-trigger="load" hx-swap="outerHtml">loading...</div>"#,
                Self::get_route().as_htmx_attribute()
            )
        }
    }
}

mod controller {

    use crate::views::MyElement;

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

        /// The web-components route.
        #[route("web-components/")]
        WebComponents(#[subroute] WebComponentsRoute),
    }

    /// The web-components sub-routes.
    #[derive(Debug, Clone, Route)]
    pub enum WebComponentsRoute {
        /// The my-element route.
        #[route("my-element/")]
        MyElement(#[subroute] MyElementRoute),
    }

    /// The my-element sub-routes.
    #[derive(Debug, Clone, Route)]
    pub enum MyElementRoute {
        /// The default route.
        #[route("")]
        Get,
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
            // TODO:
            // - Add the concept of loading visual elements?
            // - Allow for non-deferred rendering of web components?
            // - Make conventions for web components and their routes?

            let web_components = htmxology::web_components::WebComponents {
                web_components: vec![htmxology::web_components::WebComponent {
                    html_element_name: "my-element".to_string(),
                    js_component_name: "MyElement".to_string(),
                    shadow_dom_mode: htmxology::web_components::ShadowDomMode::Open,
                    html_content: views::MyElement::render_deferred(),
                }],
            };

            match route {
                AppRoute::Home => views::Index { web_components }.render_into_response(),
                AppRoute::WebComponents(WebComponentsRoute::MyElement(MyElementRoute::Get)) => {
                    MyElement.render_into_response()
                }
            }
        }
    }
}
