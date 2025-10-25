//! Run with
//!
//! ```not_rust
//! just example components
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

mod controller {

    use axum::response::IntoResponse;
    use htmxology::{ComponentsController, Route, ServerInfo};
    use htmxology::{Controller, ControllerExt, htmx::Request as HtmxRequest};

    // TODO: Derive the route variant names without the "Controller" suffix? Or allow/require to
    // specify it?

    /// The main controller implementation.
    #[derive(Debug, Clone, ComponentsController)]
    #[controller(AppRoute)]
    #[component(HelloWorldController, route = "hello-world/")]
    #[component(ImageGalleryController<'_>, route = "image-gallery", convert_with = "ImageGalleryController::from_main_controller")]
    #[component(DelegatedController, catch_all)]
    pub struct MainController {
        image_gallery_base_url: String,
    }

    impl Default for MainController {
        fn default() -> Self {
            Self {
                image_gallery_base_url: "https://picsum.photos/id/".to_string(),
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct DelegatedController;

    /// The delegated application routes.
    #[derive(Debug, Clone, Route)]
    pub enum DelegatedRoute {
        /// The home route.
        #[route("")]
        Home,

        /// The echo route.
        #[route("echo/{message}", method = "GET")]
        Echo { message: String },
    }

    impl Controller for DelegatedController {
        type Route = DelegatedRoute;

        async fn handle_request(
            &self,
            route: Self::Route,
            htmx: HtmxRequest,
            parts: http::request::Parts,
            server_info: &ServerInfo,
        ) -> Result<axum::response::Response, axum::response::Response> {
            match route {
                DelegatedRoute::Home => self.handle_home_request(htmx, parts, server_info).await,
                DelegatedRoute::Echo { message } => {
                    self.handle_echo_request(htmx, parts, server_info, &message)
                        .await
                }
            }
        }
    }

    impl DelegatedController {
        async fn handle_home_request(
            &self,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
        ) -> Result<axum::response::Response, axum::response::Response> {
            Ok((
                [(http::header::CONTENT_TYPE, "text/html")],
                format!(
                    r#"
<p>Welcome to the HTMX-SSR Components Example!</p>
<p>Visit the <a href="{}">Hello World Component</a> or The
 <a href="{}">Image Gallery Component</a>.</p>
"#,
                    AppRoute::HelloWorld(HelloWorldRoute::Index),
                    AppRoute::ImageGallery(ImageGalleryRoute::Index)
                ),
            )
                .into_response())
        }

        async fn handle_echo_request(
            &self,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
            message: &String,
        ) -> Result<axum::response::Response, axum::response::Response> {
            Ok((
                [(http::header::CONTENT_TYPE, "text/html")],
                format!("<p>Message echoed: {message}</p>"),
            )
                .into_response())
        }
    }

    impl From<&'_ MainController> for DelegatedController {
        fn from(_main_controller: &'_ MainController) -> Self {
            Self
        }
    }

    // TODO: Generate those with a derive?

    /// The main application routes.
    #[derive(Debug, Clone, Route)]
    pub enum AppRoute {
        /// Hello world component route.
        #[route("hello-world/")]
        HelloWorld(#[subroute] <HelloWorldController as Controller>::Route),

        /// The image gallery component route.
        #[route("image-gallery/")]
        ImageGallery(#[subroute] <ImageGalleryController<'static> as Controller>::Route),

        /// The delegated route.
        #[catch_all]
        Delegated(<DelegatedController as Controller>::Route),
    }

    impl Controller for MainController {
        type Route = AppRoute;

        async fn handle_request(
            &self,
            route: Self::Route,
            htmx: HtmxRequest,
            parts: http::request::Parts,
            server_info: &ServerInfo,
        ) -> Result<axum::response::Response, axum::response::Response> {
            match route {
                Self::Route::HelloWorld(route) => {
                    self.get_component::<HelloWorldController>()
                        .handle_request(route, htmx, parts, server_info)
                        .await
                }
                Self::Route::ImageGallery(route) => {
                    self.get_component::<ImageGalleryController>()
                        .handle_request(route, htmx, parts, server_info)
                        .await
                }
                Self::Route::Delegated(route) => {
                    self.get_component::<DelegatedController>()
                        .handle_request(route, htmx, parts, server_info)
                        .await
                }
            }
        }
    }

    // A simple sub-component.

    #[derive(Debug, Clone, Default)]
    pub struct HelloWorldController;

    impl From<&'_ MainController> for HelloWorldController {
        fn from(_main_controller: &'_ MainController) -> Self {
            Self
        }
    }

    /// The image gallery routes.
    #[derive(Debug, Clone, Route)]
    pub enum HelloWorldRoute {
        /// The index route.
        #[route("")]
        Index,
    }

    impl Controller for HelloWorldController {
        type Route = HelloWorldRoute;

        async fn handle_request(
            &self,
            route: Self::Route,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
        ) -> Result<axum::response::Response, axum::response::Response> {
            match route {
                HelloWorldRoute::Index => Ok((
                    [(http::header::CONTENT_TYPE, "text/html")],
                    "<p>Hello, World!</p>",
                )
                    .into_response()),
            }
        }
    }

    // A more complex sub-component.

    #[derive(Debug, Clone)]
    pub struct ImageGalleryController<'c> {
        main_controller: &'c MainController,
    }

    impl<'c> ImageGalleryController<'c> {
        pub fn from_main_controller(
            main_controller: &'c MainController,
        ) -> ImageGalleryController<'c> {
            ImageGalleryController { main_controller }
        }
    }

    /// The image gallery routes.
    #[derive(Debug, Clone, Route)]
    pub enum ImageGalleryRoute {
        /// The index route.
        #[route("")]
        Index,
    }

    impl<'c> Controller for ImageGalleryController<'c> {
        type Route = ImageGalleryRoute;

        async fn handle_request(
            &self,
            route: Self::Route,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
        ) -> Result<axum::response::Response, axum::response::Response> {
            let base_url = &self.main_controller.image_gallery_base_url;

            match route {
                ImageGalleryRoute::Index => Ok((
                    [(http::header::CONTENT_TYPE, "text/html")],
                    format!(
                        "
                    <div class=\"image-gallery\">
                        <img src=\"{base_url}1/200/200\" alt=\"Random Image 1\" />
                        <img src=\"{base_url}2/200/200\" alt=\"Random Image 2\" />
                        <img src=\"{base_url}3/200/200\" alt=\"Random Image 3\" />
                    </div>",
                    ),
                )
                    .into_response()),
            }
        }
    }
}
