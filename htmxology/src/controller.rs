//! The controller trait.

use std::future::Future;

/// The controller trait is responsible for rendering views in an application, based on a given
/// route and any associated model.
///
/// Controllers support typed responses through the `Response` associated type, enabling semantic
/// composition where parent controllers can wrap or transform child controller responses in a
/// type-safe manner.
///
/// # Example
///
/// ```rust,ignore
/// // Root controller using axum::Response without Args
/// impl Controller for RootController {
///     type Route = MyRoute;
///     type Args = ();
///     type Response = Result<axum::response::Response, axum::response::Response>;
///
///     async fn handle_request(
///         &self,
///         route: Self::Route,
///         htmx: htmx::Request,
///         parts: http::request::Parts,
///         server_info: &ServerInfo,
///         args: Self::Args,
///     ) -> Self::Response {
///         Ok(my_response.into_response())
///     }
/// }
///
/// // Controller with session Args using Arc<RwLock<T>> for shared mutable state
/// struct AppContext {
///     session: Arc<RwLock<UserSession>>,
///     db: Arc<Database>,
/// }
///
/// impl Controller for BlogController {
///     type Route = BlogRoute;
///     type Args = AppContext;
///     type Response = Result<BlogResponse, BlogError>;
///
///     async fn handle_request(
///         &self,
///         route: Self::Route,
///         htmx: htmx::Request,
///         parts: http::request::Parts,
///         server_info: &ServerInfo,
///         args: Self::Args,
///     ) -> Self::Response {
///         // Can access and mutate session through Arc<RwLock<T>>
///         let mut session = args.session.write().await;
///         session.last_accessed = now();
///         Ok(BlogResponse { /* ... */ })
///     }
/// }
/// ```
///
/// For root controllers that directly serve HTTP responses, use
/// `Result<axum::response::Response, axum::response::Response>` as the `Response` type.
/// Intermediate controllers can use custom types that will be converted by parent controllers
/// using Rust's `Into`/`From` traits (or via the `convert_response` attribute in the
/// `RoutingController` macro for custom conversion logic).
pub trait Controller: Send + Sync + Clone {
    /// The route type associated with the controller.
    type Route: super::Route + Send + axum::extract::FromRequest<Self>;

    /// Arguments passed to the `handle_request` method.
    ///
    /// This type represents transient data that flows through the controller hierarchy,
    /// such as user sessions, database connections, or other request-scoped state.
    /// Args are created fresh for each request via the `args_factory` function and
    /// passed by value to `handle_request`.
    ///
    /// For controllers that don't require such data, set this to `()`.
    /// For controllers needing shared context, use a struct type like `AppContext`.
    ///
    /// **Note on mutability**: Args are passed by value (owned), not by reference.
    /// For shared mutable state across requests, use `Arc<RwLock<T>>` or `Arc<Mutex<T>>`.
    ///
    /// **Note on path parameters**: Path parameters (like `blog_id`) should remain in Route
    /// variants, not in Args.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // No args needed
    /// type Args = ();
    ///
    /// // Owned args with shared mutable state
    /// struct AppContext {
    ///     db: Arc<RwLock<Database>>,
    ///     user_id: u32,
    /// }
    /// type Args = AppContext;
    /// ```
    type Args: Send + Sync + 'static;

    /// The response type for this controller.
    ///
    /// This is the full `Result<T, E>` type returned by `handle_request()`. Root controllers
    /// should use `Result<axum::response::Response, axum::response::Response>`, while
    /// intermediate controllers can use custom types like `Result<MyResponse, MyError>`.
    ///
    /// Parent controllers convert child responses using Rust's `Into`/`From` traits, or via
    /// custom conversion functions specified with the `convert_response` attribute in the
    /// `RoutingController` macro.
    type Response: Send + 'static;

    /// Handle the request for a given route.
    ///
    /// Returns a typed `Response` which can be a `Result` with custom types for intermediate
    /// controllers, or `Result<axum::response::Response, axum::response::Response>` for root
    /// controllers. Parent controllers are responsible for converting child responses via
    /// the `HasSubcontroller` trait.
    ///
    /// # Arguments
    ///
    /// * `args` - Arguments passed by value through the controller hierarchy.
    ///   This enables passing transient data (like user sessions) through controllers.
    ///   Set `Args = ()` if not needed.
    fn handle_request(
        &self,
        route: Self::Route,
        htmx: super::htmx::Request,
        parts: http::request::Parts,
        server_info: &super::ServerInfo,
        args: Self::Args,
    ) -> impl Future<Output = Self::Response> + Send;
}

/// An extension trait for controllers that provides subcontroller access.
pub trait SubcontrollerExt: Controller {
    /// Get a subcontroller from the current controller.
    ///
    /// This is a convenience method that leverages the `HasSubcontroller` trait and allows specifying
    /// the subcontroller type directly.
    fn get_subcontroller<'c, C>(&'c self) -> C
    where
        Self: HasSubcontroller<'c, C>,
        C: Controller,
    {
        <Self as HasSubcontroller<'c, C>>::as_subcontroller(self)
    }
}

impl<T: Controller> SubcontrollerExt for T {}

/// A trait for controllers that have subcontrollers.
///
/// This trait enables composing controllers by allowing a parent controller to provide
/// subcontroller instances. Response conversion from subcontroller to parent is handled
/// via Rust's standard `Into`/`From` traits or can be customized using the
/// `convert_response` attribute in the `RoutingController` macro.
///
/// # Type Parameters
/// - `'c`: Lifetime of the controller reference
/// - `Subcontroller`: The subcontroller type
pub trait HasSubcontroller<'c, Subcontroller>: Controller
where
    Subcontroller: Controller,
{
    /// Get a subcontroller instance from this controller.
    ///
    /// Subcontrollers are constructed without arguments. Path parameters should be
    /// embedded in route variants, and transient data (like sessions) is passed via
    /// the `Args` parameter to `handle_request`.
    fn as_subcontroller(&'c self) -> Subcontroller;
}
