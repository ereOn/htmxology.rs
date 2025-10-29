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
    use htmxology::{Controller, SubcontrollerExt, htmx::Request as HtmxRequest};
    use htmxology::{Route, RoutingController, ServerInfo};

    /// The main controller implementation.
    #[derive(Debug, Clone, RoutingController)]
    #[controller(AppRoute)]
    #[subcontroller(HelloWorldController, route=HelloWorld, path = "hello-world/", convert_response = "Ok")]
    #[subcontroller(ImageGalleryController<'_>, route=ImageGallery, path = "image-gallery/", convert_with = "ImageGalleryController::from_main_controller")]
    #[subcontroller(UserPostController, route=UserPost, path = "user/{user_id}/post/{post_id}/", params(user_id: u32, post_id: String), convert_with = "Self::make_user_post_controller")]
    #[subcontroller(DelegatedController, route=Delegated)]
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

    impl MainController {
        fn make_user_post_controller(&self, user_id: u32, post_id: String) -> UserPostController {
            UserPostController { user_id, post_id }
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
        type Args = ();
        type Response = Result<axum::response::Response, axum::response::Response>;

        async fn handle_request(
            &self,
            route: Self::Route,
            htmx: HtmxRequest,
            parts: http::request::Parts,
            server_info: &ServerInfo,
        ) -> Self::Response {
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
<p>Visit the <a href="{}">Hello World Component</a>,
 the <a href="{}">Image Gallery Component</a>,
 or see <a href="{}">User #42's Post "hello-world"</a>.</p>
"#,
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
        type Args = ();
        type Response = axum::response::Response;

        async fn handle_request(
            &self,
            route: Self::Route,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
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
        type Args = ();
        type Response = Result<axum::response::Response, axum::response::Response>;

        async fn handle_request(
            &self,
            route: Self::Route,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
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

    #[derive(Debug, Clone)]
    pub struct UserPostController {
        user_id: u32,
        post_id: String,
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
        type Args = (u32, String);
        type Response = Result<axum::response::Response, axum::response::Response>;

        async fn handle_request(
            &self,
            route: Self::Route,
            _htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
        ) -> Self::Response {
            let user_id = self.user_id;
            let post_id = &self.post_id;

            match route {
                UserPostRoute::Index => Ok((
                    [(http::header::CONTENT_TYPE, "text/html")],
                    format!(
                        r#"
                    <div class="user-post">
                        <h2>Post: {post_id}</h2>
                        <p>Author: User #{user_id}</p>
                        <p>This is a demonstration of parameterized routes in htmxology.</p>
                        <p>The controller received user_id={user_id} and post_id="{post_id}" as parameters.</p>
                        <nav>
                            <a href="{}">View Comments</a> |
                            <a href="{}">Edit Post</a>
                        </nav>
                    </div>"#,
                        AppRoute::UserPost {
                            user_id,
                            post_id: post_id.clone(),
                            subroute: UserPostRoute::Comments,
                        },
                        AppRoute::UserPost {
                            user_id,
                            post_id: post_id.clone(),
                            subroute: UserPostRoute::Edit,
                        }
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
                        <ul>
                            <li><strong>Alice:</strong> Great post, User #{user_id}!</li>
                            <li><strong>Bob:</strong> Very informative about "{post_id}"</li>
                            <li><strong>Charlie:</strong> Thanks for sharing!</li>
                        </ul>
                        <p><a href="{}">Back to Post</a></p>
                    </div>"#,
                        AppRoute::UserPost {
                            user_id,
                            post_id: post_id.clone(),
                            subroute: UserPostRoute::Index,
                        }
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
                        <form>
                            <label>Post ID: <input type="text" value="{post_id}" readonly /></label><br/>
                            <label>Content: <textarea rows="5" cols="50">This demonstrates how parameters (user_id={user_id}, post_id="{post_id}") flow through the routing system.</textarea></label><br/>
                            <button type="submit">Save Changes</button>
                        </form>
                        <p><a href="{}">Cancel and go back</a></p>
                    </div>"#,
                        AppRoute::UserPost {
                            user_id,
                            post_id: post_id.clone(),
                            subroute: UserPostRoute::Index,
                        }
                    ),
                )
                    .into_response()),
            }
        }
    }
}
