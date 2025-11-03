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

    // Create a persistent session that will be shared across all requests
    let session = std::sync::Arc::new(tokio::sync::RwLock::new(controller::UserSessionImpl {
        user_name: "Guest".to_string(),
        is_authenticated: false,
        visit_count: 0,
    }));

    // Create a new server with auto-reload enabled by attempting to get a TCP listener from the
    // `listenfd` environment variable, falling back to binding to a local address if that fails.
    let server = htmxology::Server::builder_with_auto_reload("127.0.0.1:3000")
        .await?
        // Set the options on the server from the environment.
        .with_options_from_env()?
        .build();

    // The controller now stores the session and uses args_factory to clone it per request
    server
        .serve(MainController::new(session))
        .await
        .map_err(Into::into)
}

mod controller {

    use std::sync::Arc;

    use axum::response::IntoResponse;
    use htmxology::{Controller, htmx::Request as HtmxRequest};
    use htmxology::{Route, RoutingController, ServerInfo};
    use tokio::sync::RwLock;

    /// User session that flows through the controller hierarchy.
    /// This represents request-scoped state that can be accessed and modified
    /// by any controller in the hierarchy.
    #[derive(Debug, Clone, Default)]
    pub struct UserSessionImpl {
        pub user_name: String,
        pub is_authenticated: bool,
        pub visit_count: u32,
    }

    type UserSession = Arc<RwLock<UserSessionImpl>>;

    /// The main controller implementation.
    #[derive(Debug, Clone, RoutingController)]
    #[controller(AppRoute, args = UserSession, args_factory = "|controller: &MainController| -> _ { let session = controller.session.clone(); async move { session } }")]
    #[subcontroller(HelloWorldController, route=HelloWorld, path = "hello-world/", convert_response = "Self::convert_plain_response")]
    #[subcontroller(ImageGalleryController<'_>, route=ImageGallery, path = "image-gallery/", convert_with = "ImageGalleryController::from_main_controller")]
    #[subcontroller(UserPostController, route=UserPost, path = "user/{user_id}/post/{post_id}/", params(user_id: u32, post_id: String))]
    #[subcontroller(DelegatedController, route=Delegated)]
    pub struct MainController {
        image_gallery_base_url: String,
        session: UserSession,
    }

    impl MainController {
        pub fn new(session: UserSession) -> Self {
            Self {
                image_gallery_base_url: "https://picsum.photos/id/".to_string(),
                session,
            }
        }
    }

    impl MainController {
        #[expect(clippy::result_large_err)]
        fn convert_plain_response(
            _htmx: &htmxology::htmx::Request,
            response: axum::response::Response,
        ) -> Result<axum::response::Response, axum::response::Response> {
            Ok(response)
        }
    }

    impl From<&MainController> for UserPostController {
        fn from(_: &MainController) -> Self {
            Self
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
        type Args = UserSession;
        type Response = Result<axum::response::Response, axum::response::Response>;

        async fn handle_request(
            &self,
            route: Self::Route,
            htmx: HtmxRequest,
            parts: http::request::Parts,
            server_info: &ServerInfo,
            args: Self::Args,
        ) -> Self::Response {
            {
                let mut session = args.write().await;

                // Increment visit count on every request
                session.visit_count += 1;
            }

            match route {
                DelegatedRoute::Home => {
                    self.handle_home_request(htmx, parts, server_info, args)
                        .await
                }
                DelegatedRoute::Echo { message } => {
                    self.handle_echo_request(htmx, parts, server_info, args, &message)
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
            session: UserSession,
        ) -> Result<axum::response::Response, axum::response::Response> {
            let session = session.read().await;

            Ok((
                [(http::header::CONTENT_TYPE, "text/html")],
                format!(
                    r#"
<p>Welcome to the HTMX-SSR Components Example!</p>
<p>Session Info: User: {}, Authenticated: {}, Visits: {}</p>
<p>Visit the <a href="{}">Hello World Component</a>,
 the <a href="{}">Image Gallery Component</a>,
 or see <a href="{}">User Post Example</a>.</p>
"#,
                    session.user_name,
                    session.is_authenticated,
                    session.visit_count,
                    AppRoute::HelloWorld(HelloWorldRoute::Index),
                    AppRoute::ImageGallery(ImageGalleryRoute::Index),
                    AppRoute::UserPost {
                        user_id: 42,
                        post_id: "hello-world".to_string(),
                        subroute: UserPostRoute::Index,
                    }
                ),
            )
                .into_response())
        }

        async fn handle_echo_request(
            &self,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
            session: UserSession,
            message: &String,
        ) -> Result<axum::response::Response, axum::response::Response> {
            let session = session.read().await;

            Ok((
                [(http::header::CONTENT_TYPE, "text/html")],
                format!(
                    "<p>Message echoed: {message}</p><p>Visit count: {}</p>",
                    session.visit_count
                ),
            )
                .into_response())
        }
    }

    impl From<&'_ MainController> for DelegatedController {
        fn from(_main_controller: &'_ MainController) -> Self {
            Self
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
        type Args = UserSession;
        type Response = axum::response::Response;

        async fn handle_request(
            &self,
            route: Self::Route,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
            _session: Self::Args,
        ) -> Self::Response {
            match route {
                HelloWorldRoute::Index => (
                    [(http::header::CONTENT_TYPE, "text/html")],
                    "<p>Hello, World!</p>",
                )
                    .into_response(),
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
        type Args = UserSession;
        type Response = Result<axum::response::Response, axum::response::Response>;

        async fn handle_request(
            &self,
            route: Self::Route,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
            _session: Self::Args,
        ) -> Self::Response {
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

    // A parameterized sub-component that demonstrates route parameters.
    // Params are passed as Args to handle_request, not stored in the controller.

    #[derive(Debug, Clone, Default)]
    pub struct UserPostController;

    // Args struct for UserPostController - receives parent args + path parameters
    // Since MainController has Args = UserSession, we receive (UserSession, u32, String)
    pub struct UserPostArgs {
        pub session: UserSession,
        pub user_id: u32,
        pub post_id: String,
    }

    impl From<(UserSession, u32, String)> for UserPostArgs {
        fn from((session, user_id, post_id): (UserSession, u32, String)) -> Self {
            Self {
                session,
                user_id,
                post_id,
            }
        }
    }

    /// The user post routes.
    #[derive(Debug, Clone, Route)]
    pub enum UserPostRoute {
        /// The index route.
        #[route("")]
        Index,

        /// View comments on the post.
        #[route("comments")]
        Comments,

        /// Edit the post.
        #[route("edit")]
        Edit,
    }

    impl Controller for UserPostController {
        type Route = UserPostRoute;
        type Args = UserPostArgs;
        type Response = Result<axum::response::Response, axum::response::Response>;

        async fn handle_request(
            &self,
            route: Self::Route,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
            args: Self::Args,
        ) -> Self::Response {
            // Path parameters AND session are passed through Args!
            let user_id = args.user_id;
            let post_id = &args.post_id;
            let session = args.session.read().await;

            let base_route = AppRoute::UserPost {
                user_id,
                post_id: post_id.clone(),
                subroute: UserPostRoute::Index,
            };

            match route {
                UserPostRoute::Index => Ok((
                    [(http::header::CONTENT_TYPE, "text/html")],
                    format!(
                        r#"
                    <div class="user-post">
                        <h2>Post: {post_id}</h2>
                        <p>Author: User #{user_id}</p>
                        <p>Viewing as: {} (Authenticated: {})</p>
                        <p>Session visits: {}</p>
                        <p>Path parameters AND session flow through Args to handle_request!</p>
                        <nav>
                            <a href="comments">View Comments</a> |
                            <a href="edit">Edit Post</a>
                        </nav>
                    </div>"#,
                        session.user_name,
                        session.is_authenticated,
                        session.visit_count
                    ),
                )
                    .into_response()),
                UserPostRoute::Comments => Ok((
                    [(http::header::CONTENT_TYPE, "text/html")],
                    format!(
                        r#"
                    <div class="comments">
                        <h2>Comments on "{post_id}"</h2>
                        <p>User #{user_id} has written an amazing post!</p>
                        <p>Viewing as: {}</p>
                        <ul>
                            <li><strong>Alice:</strong> Great post, User #{user_id}!</li>
                            <li><strong>Bob:</strong> Very informative about "{post_id}"</li>
                        </ul>
                        <p><a href="{base_route}">Back to Post</a></p>
                    </div>"#,
                        session.user_name
                    ),
                )
                    .into_response()),
                UserPostRoute::Edit => Ok((
                    [(http::header::CONTENT_TYPE, "text/html")],
                    format!(
                        r#"
                    <div class="edit-post">
                        <h2>Edit Post: {post_id}</h2>
                        <p>Editing post by User #{user_id}</p>
                        <p>Editor: {} (Visits: {})</p>
                        <form>
                            <label>Post ID: <input type="text" value="{post_id}" readonly /></label><br/>
                            <label>Content: <textarea rows="5" cols="50">Content for user {user_id}'s post "{post_id}"</textarea></label><br/>
                            <button type="submit">Save Changes</button>
                        </form>
                        <p><a href="{base_route}">Cancel and go back</a></p>
                    </div>"#,
                        session.user_name,
                        session.visit_count
                    ),
                )
                    .into_response()),
            }
        }
    }
}
