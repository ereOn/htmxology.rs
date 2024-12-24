//! The controller trait.

use axum::async_trait;

/// The controller trait is responsible for rendering views in an application, based on a given
/// route and any associated model.
#[async_trait]
pub trait Controller: Send + Sync + Clone + 'static {
    /// The route type associated with the controller.
    type Route: super::Route + Send + axum::extract::FromRequest<super::ServerState<Self>>;

    /// Render a view for a given route.
    async fn render_view(
        &self,
        route: Self::Route,
        htmx: super::htmx::Request,
        parts: http::request::Parts,
        server_info: &super::ServerInfo,
    ) -> axum::response::Response;
}
