//! The controller trait.

use std::future::Future;

/// The controller trait is responsible for rendering views in an application, based on a given
/// route and any associated model.
pub trait Controller: Send + Sync + Clone {
    /// The route type associated with the controller.
    type Route: super::Route + Send + axum::extract::FromRequest<Self>;

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
    /// the component type directly.
    fn get_component<'c, C>(&'c self) -> C
    where
        Self: AsComponent<'c, C>,
        C: Controller,
    {
        <Self as AsComponent<'c, C>>::as_component_controller(self)
    }
}

impl<T: Controller> ControllerExt for T {}

/// A trait for controllers that have sub-components.
pub trait AsComponent<'c, Component>: Controller
where
    Component: Controller,
{
    fn as_component_controller(&'c self) -> Component;
}
