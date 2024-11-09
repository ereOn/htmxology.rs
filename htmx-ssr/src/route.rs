//! The route trait.

/// The route trait can be implemented for types that represent a possible set of routes in an
/// application.
pub trait Route {
    /// Register the routes of the controller into the specified Axum router.
    fn register_routes<Controller: super::Controller<Route = Self>>(
        router: axum::Router<crate::State<Controller::Model>>,
    ) -> axum::Router<crate::State<Controller::Model>>;
}
