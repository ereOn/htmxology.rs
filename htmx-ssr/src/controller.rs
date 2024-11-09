//! The controller trait.

use axum::async_trait;

/// The controller trait is responsible for rendering views in an application, possibly from the
/// server state and associated model.
#[async_trait]
pub trait Controller: Send + Sync + 'static {
    /// The route type associated with the controller.
    type Route: super::Route;

    /// The model type associated with the controller.
    type Model: Send + Sync + Clone + 'static;

    /// Register the routes of the controller into the specified Axum router.
    async fn render_view(
        route: Self::Route,
        state: crate::State<Self::Model>,
        htmx: crate::htmx::Request,
    ) -> axum::response::Response;
}
