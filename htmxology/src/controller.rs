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
/// // Root controller using axum::Response
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
///     ) -> Self::Response {
///         Ok(my_response.into_response())
///     }
/// }
///
/// // Intermediate controller with custom types
/// impl Controller for BlogController {
///     type Route = BlogRoute;
///     type Args = ();
///     type Response = Result<BlogResponse, BlogError>;
///
///     async fn handle_request(...) -> Self::Response {
///         Ok(BlogResponse { /* ... */ })
///     }
/// }
/// ```
///
/// For root controllers that directly serve HTTP responses, use
/// `Result<axum::response::Response, axum::response::Response>` as the `Response` type.
/// Intermediate controllers can use custom types that will be converted by parent controllers
/// via the `HasSubcontroller::convert_response()` method.
pub trait Controller: Send + Sync + Clone {
    /// The route type associated with the controller.
    type Route: super::Route + Send + axum::extract::FromRequest<Self>;

    /// Arguments required to construct this controller from a parent controller.
    ///
    /// This is used when a controller is a sub-component of another controller and requires
    /// parameters from the parent route (e.g., path parameters like `blog_id`).
    ///
    /// For controllers that don't require construction arguments, set this to `()`.
    /// For parameterized controllers, use a tuple type like `(u32,)` or `(u32, String)`.
    type Args: Send + Sync + 'static;

    /// The response type for this controller.
    ///
    /// This is the full `Result<T, E>` type returned by `handle_request()`. Root controllers
    /// should use `Result<axum::response::Response, axum::response::Response>`, while
    /// intermediate controllers can use custom types like `Result<MyResponse, MyError>`.
    ///
    /// Parent controllers convert child responses using `HasSubcontroller::convert_response()`.
    type Response: Send + 'static;

    /// Handle the request for a given route.
    ///
    /// Returns a typed `Response` which can be a `Result` with custom types for intermediate
    /// controllers, or `Result<axum::response::Response, axum::response::Response>` for root
    /// controllers. Parent controllers are responsible for converting child responses via
    /// the `HasSubcontroller` trait.
    fn handle_request(
        &self,
        route: Self::Route,
        htmx: super::htmx::Request,
        parts: http::request::Parts,
        server_info: &super::ServerInfo,
    ) -> impl Future<Output = Self::Response> + Send;
}

/// An extension trait for controllers that provides subcontroller access.
pub trait SubcontrollerExt: Controller {
    /// Get a subcontroller from the current controller.
    ///
    /// This is a convenience method that leverages the `HasSubcontroller` trait and allows specifying
    /// the subcontroller type directly. This method is for subcontrollers that don't require construction
    /// arguments (i.e., `Args = ()`).
    ///
    /// For subcontrollers that require arguments, use [`get_subcontroller_with`](Self::get_subcontroller_with).
    fn get_subcontroller<'c, C>(&'c self) -> C
    where
        Self: HasSubcontroller<'c, C, ()>,
        C: Controller<Args = ()>,
    {
        <Self as HasSubcontroller<'c, C, ()>>::as_subcontroller(self, ())
    }

    /// Get a subcontroller from the current controller with construction arguments.
    ///
    /// This is a convenience method for subcontrollers that require arguments to be constructed.
    /// The arguments typically come from path parameters in the parent route.
    ///
    /// # Example
    /// ```rust,ignore
    /// // In handle_request for route Blog { blog_id, subroute }
    /// self.get_subcontroller_with::<BlogController>((blog_id,))
    ///     .handle_request(subroute, htmx, parts, server_info)
    ///     .await
    /// ```
    fn get_subcontroller_with<'c, C>(&'c self, args: C::Args) -> C
    where
        Self: HasSubcontroller<'c, C, C::Args>,
        C: Controller,
    {
        <Self as HasSubcontroller<'c, C, C::Args>>::as_subcontroller(self, args)
    }

    /// Handle a request using a subcontroller and convert its response.
    ///
    /// This is a convenience method that combines getting a subcontroller, calling
    /// `handle_request` on it, and converting the response to the parent controller's
    /// response type. This method is for subcontrollers that don't require construction
    /// arguments (i.e., `Args = ()`).
    ///
    /// For subcontrollers that require arguments, use [`handle_subcontroller_request_with`](Self::handle_subcontroller_request_with).
    ///
    /// # Example
    /// ```rust,ignore
    /// // In handle_request for route Blog(subroute)
    /// self.handle_subcontroller_request::<BlogController>(subroute, htmx, parts, server_info)
    ///     .await
    /// ```
    fn handle_subcontroller_request<'c, C>(
        &'c self,
        route: C::Route,
        htmx: super::htmx::Request,
        parts: http::request::Parts,
        server_info: &super::ServerInfo,
    ) -> impl std::future::Future<Output = Self::Response> + Send
    where
        Self: HasSubcontroller<'c, C, ()>,
        C: Controller<Args = ()>,
    {
        async move {
            let subcontroller = self.get_subcontroller::<C>();
            let response = subcontroller
                .handle_request(route, htmx.clone(), parts, server_info)
                .await;
            <Self as HasSubcontroller<'c, C, ()>>::convert_response(&htmx, response)
        }
    }

    /// Handle a request using a subcontroller with construction arguments and convert its response.
    ///
    /// This is a convenience method that combines getting a subcontroller with arguments,
    /// calling `handle_request` on it, and converting the response to the parent controller's
    /// response type.
    ///
    /// # Example
    /// ```rust,ignore
    /// // In handle_request for route Blog { blog_id, subroute }
    /// self.handle_subcontroller_request_with::<BlogController>((blog_id,), subroute, htmx, parts, server_info)
    ///     .await
    /// ```
    fn handle_subcontroller_request_with<'c, C>(
        &'c self,
        args: C::Args,
        route: C::Route,
        htmx: super::htmx::Request,
        parts: http::request::Parts,
        server_info: &super::ServerInfo,
    ) -> impl std::future::Future<Output = Self::Response> + Send
    where
        Self: HasSubcontroller<'c, C, C::Args>,
        C: Controller,
    {
        async move {
            let subcontroller = self.get_subcontroller_with::<C>(args);
            let response = subcontroller
                .handle_request(route, htmx.clone(), parts, server_info)
                .await;
            <Self as HasSubcontroller<'c, C, C::Args>>::convert_response(&htmx, response)
        }
    }
}

impl<T: Controller> SubcontrollerExt for T {}

/// A trait for controllers that have subcontrollers.
///
/// This trait enables composing controllers by allowing a parent controller to provide
/// subcontroller instances. The `Args` type parameter specifies what arguments are needed
/// for constructing the subcontroller.
///
/// The `convert_response` method handles converting the subcontroller's `Response` type
/// to the parent controller's `Response` type, enabling flexible composition without
/// forcing all controllers to use the same response types.
///
/// # Type Parameters
/// - `'c`: Lifetime of the controller reference
/// - `Subcontroller`: The subcontroller type
/// - `Args`: Arguments needed to construct the subcontroller (defaults to `()`)
pub trait HasSubcontroller<'c, Subcontroller, Args = ()>: Controller
where
    Subcontroller: Controller,
    Args: Send + Sync + 'static,
{
    /// Get a subcontroller instance from this controller.
    ///
    /// # Arguments
    /// - `args`: Construction arguments for the subcontroller, typically extracted from route parameters
    fn as_subcontroller(&'c self, args: Args) -> Subcontroller;

    /// Convert the subcontroller's response to the parent controller's response type.
    ///
    /// This method is called after the subcontroller handles a request, allowing the parent
    /// to transform or wrap the child's response. For cases where both parent and child use
    /// the same `Response` type, this can be an identity function.
    ///
    /// The `htmx` parameter provides access to the HTMX request context, which can be used
    /// to determine whether to return a full page or a fragment based on the request type.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // When both use axum::Response (identity conversion)
    /// fn convert_response(htmx: &super::htmx::Request, response: Subcontroller::Response) -> Self::Response {
    ///     response
    /// }
    ///
    /// // When converting custom types to axum::Response
    /// fn convert_response(htmx: &super::htmx::Request, response: Result<BlogResponse, BlogError>) -> Self::Response {
    ///     response
    ///         .map(|r| r.into_response())
    ///         .map_err(|e| e.into_response())
    /// }
    /// ```
    fn convert_response(
        htmx: &super::htmx::Request,
        response: Subcontroller::Response,
    ) -> Self::Response;
}
