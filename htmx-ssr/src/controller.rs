//! The controller trait.

use axum::async_trait;

pub trait Controller {
    /// The model type associated with the controller.
    type Model: Send + Sync + Clone + 'static;

    /// Register the routes of the controller into the specified Axum router.
    fn register_routes(
        router: axum::Router<crate::State<Self::Model>>,
    ) -> axum::Router<crate::State<Self::Model>>;
}

/// The view mapper trait is responsible for mapping the controller routes to the views, possibly
/// by using the model.
#[async_trait]
pub trait ViewMapper {
    /// The model type associated with the controller.
    type Model: Send + Sync + Clone + 'static;

    /// Register the routes of the controller into the specified Axum router.
    async fn render_view(
        self,
        state: crate::State<Self::Model>,
        htmx: crate::htmx::Request,
    ) -> axum::response::Response;
}
