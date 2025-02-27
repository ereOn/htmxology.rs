//! The controller trait.

use std::future::Future;

/// The controller trait is responsible for rendering views in an application, based on a given
/// route and any associated model.
pub trait Controller: Send + Sync + Clone + 'static {
    /// The route type associated with the controller.
    type Route: super::Route + Send + axum::extract::FromRequest<Self>;

    /// Render a view for a given route.
    fn render_view(
        &self,
        route: Self::Route,
        htmx: super::htmx::Request,
        parts: http::request::Parts,
        server_info: &super::ServerInfo,
    ) -> impl Future<Output = axum::response::Response> + Send;
}
