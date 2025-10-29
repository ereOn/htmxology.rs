//! The controller trait.

use std::future::Future;

/// The controller trait is responsible for rendering views in an application, based on a given
/// route and any associated model.
///
/// Controllers now support typed responses through the `Output` and `ErrorOutput` associated types,
/// enabling semantic composition where parent controllers can wrap or transform child controller
/// responses in a type-safe manner.
///
/// # Example
///
/// ```rust,ignore
/// impl Controller for MyController {
///     type Route = MyRoute;
///     type Args = ();
///     type Output = MyResponse;  // Custom response type
///     type ErrorOutput = MyError;  // Custom error type
///
///     async fn handle_request(
///         &self,
///         route: Self::Route,
///         htmx: htmx::Request,
///         parts: http::request::Parts,
///         server_info: &ServerInfo,
///     ) -> Result<Self::Output, Self::ErrorOutput> {
///         // Return typed responses
///         Ok(MyResponse { /* ... */ })
///     }
/// }
/// ```
///
/// For root controllers that directly serve HTTP responses, use `axum::response::Response` for both
/// `Output` and `ErrorOutput` types.
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

    /// The successful output type for this controller.
    ///
    /// This enables semantic composition where parent controllers can work with typed responses
    /// from their children. Root controllers should use `axum::response::Response`.
    type Output: Send + 'static;

    /// The error output type for this controller.
    ///
    /// This enables type-safe error handling where parent controllers can transform or handle
    /// errors from their children. Root controllers should use `axum::response::Response`.
    type ErrorOutput: Send + 'static;

    /// Handle the request for a given route.
    ///
    /// Returns a `Result` with typed `Output` and `ErrorOutput` rather than opaque
    /// `axum::response::Response` values. This allows parent controllers to meaningfully
    /// compose and transform child controller responses.
    fn handle_request(
        &self,
        route: Self::Route,
        htmx: super::htmx::Request,
        parts: http::request::Parts,
        server_info: &super::ServerInfo,
    ) -> impl Future<Output = Result<Self::Output, Self::ErrorOutput>> + Send;
}

/// An extension trait for controllers that provides subcontroller access.
pub trait SubcontrollerExt: Controller {
    /// Get a subcontroller from the current controller.
    ///
    /// This is a convenience method that leverages the `AsSubcontroller` trait and allows specifying
    /// the subcontroller type directly. This method is for subcontrollers that don't require construction
    /// arguments (i.e., `Args = ()`).
    ///
    /// For subcontrollers that require arguments, use [`get_subcontroller_with`](Self::get_subcontroller_with).
    fn get_subcontroller<'c, C>(&'c self) -> C
    where
        Self: AsSubcontroller<'c, C, ()>,
        C: Controller<Args = ()>,
    {
        <Self as AsSubcontroller<'c, C, ()>>::as_subcontroller(self, ())
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
        Self: AsSubcontroller<'c, C, C::Args>,
        C: Controller,
    {
        <Self as AsSubcontroller<'c, C, C::Args>>::as_subcontroller(self, args)
    }
}

impl<T: Controller> SubcontrollerExt for T {}

/// A trait for controllers that have subcontrollers.
///
/// This trait enables composing controllers by converting a parent controller into a
/// subcontroller. The `Args` type parameter specifies what arguments are needed
/// for the conversion.
///
/// # Type Parameters
/// - `'c`: Lifetime of the controller reference
/// - `Subcontroller`: The subcontroller type
/// - `Args`: Arguments needed to construct the subcontroller (defaults to `()`)
pub trait AsSubcontroller<'c, Subcontroller, Args = ()>: Controller
where
    Subcontroller: Controller,
    Args: Send + Sync + 'static,
{
    /// Convert this controller into a subcontroller.
    ///
    /// # Arguments
    /// - `args`: Construction arguments for the subcontroller, typically extracted from route parameters
    fn as_subcontroller(&'c self, args: Args) -> Subcontroller;
}

/// Extension trait for converting typed controller results to axum responses.
///
/// This trait provides a convenient way to convert `Result<Output, ErrorOutput>` into
/// `Result<axum::response::Response, axum::response::Response>`, which is required at
/// the root controller boundary or when interacting with axum handlers.
///
/// # Example
///
/// ```rust,ignore
/// let result: Result<MyOutput, MyError> = subcontroller.handle_request(...).await;
/// let response: Result<Response, Response> = result.into_axum_result();
/// ```
pub trait IntoAxumResult {
    /// Convert this result into a result of axum responses.
    fn into_axum_result(self) -> Result<axum::response::Response, axum::response::Response>;
}

impl<O, E> IntoAxumResult for Result<O, E>
where
    O: axum::response::IntoResponse,
    E: axum::response::IntoResponse,
{
    fn into_axum_result(self) -> Result<axum::response::Response, axum::response::Response> {
        match self {
            Ok(output) => Ok(output.into_response()),
            Err(error) => Err(error.into_response()),
        }
    }
}
