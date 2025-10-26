//! The controller trait.

use std::future::Future;

/// The controller trait is responsible for rendering views in an application, based on a given
/// route and any associated model.
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

    /// Handle the request for a given route.
    fn handle_request(
        &self,
        route: Self::Route,
        htmx: super::htmx::Request,
        parts: http::request::Parts,
        server_info: &super::ServerInfo,
    ) -> impl Future<Output = Result<axum::response::Response, axum::response::Response>> + Send;
}

/// An extension trait for controllers.
pub trait ControllerExt: Controller {
    /// Get a sub-component controller from the current controller.
    ///
    /// This is a convenience method that leverages the `AsComponent` trait and allows specifying
    /// the component type directly. This method is for components that don't require construction
    /// arguments (i.e., `Args = ()`).
    ///
    /// For components that require arguments, use [`get_component_with`](Self::get_component_with).
    fn get_component<'c, C>(&'c self) -> C
    where
        Self: AsComponent<'c, C, ()>,
        C: Controller<Args = ()>,
    {
        <Self as AsComponent<'c, C, ()>>::as_component_controller(self, ())
    }

    /// Get a sub-component controller from the current controller with construction arguments.
    ///
    /// This is a convenience method for components that require arguments to be constructed.
    /// The arguments typically come from path parameters in the parent route.
    ///
    /// # Example
    /// ```rust,ignore
    /// // In handle_request for route Blog { blog_id, subroute }
    /// self.get_component_with::<BlogController>((blog_id,))
    ///     .handle_request(subroute, htmx, parts, server_info)
    ///     .await
    /// ```
    fn get_component_with<'c, C>(&'c self, args: C::Args) -> C
    where
        Self: AsComponent<'c, C, C::Args>,
        C: Controller,
    {
        <Self as AsComponent<'c, C, C::Args>>::as_component_controller(self, args)
    }
}

impl<T: Controller> ControllerExt for T {}

/// A trait for controllers that have sub-components.
///
/// This trait enables composing controllers by converting a parent controller into a
/// sub-component controller. The `Args` type parameter specifies what arguments are needed
/// for the conversion.
///
/// # Type Parameters
/// - `'c`: Lifetime of the controller reference
/// - `Component`: The sub-component controller type
/// - `Args`: Arguments needed to construct the component (defaults to `()`)
pub trait AsComponent<'c, Component, Args = ()>: Controller
where
    Component: Controller,
    Args: Send + Sync + 'static,
{
    /// Convert this controller into a sub-component controller.
    ///
    /// # Arguments
    /// - `args`: Construction arguments for the component, typically extracted from route parameters
    fn as_component_controller(&'c self, args: Args) -> Component;
}
