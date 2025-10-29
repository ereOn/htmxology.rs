use std::sync::Arc;

use axum::Router;

use crate::Controller;

use super::ServerInfo;

/// A router that is associated to a controller.
#[derive(Debug, Clone, Default)]
pub struct ControllerRouter(Router);

impl ControllerRouter {
    /// Create a new controller router from an existing router.
    ///
    /// # Safety
    ///
    /// The router must have been created with the correct fallback handler referencing a
    /// controller, likely through the `ControllerRouter::new` constructor.
    ///
    /// The router may contain layers or additional routes that are not controller-related.
    pub unsafe fn from_router(router: Router) -> Self {
        Self(router)
    }

    /// Create a new controller router from a controller.
    pub fn new<C>(controller: C) -> Self
    where
        C: Controller + 'static,
        C::Output: axum::response::IntoResponse,
        C::ErrorOutput: axum::response::IntoResponse,
    {
        let router = Router::new()
            .fallback(
                |axum::extract::State(controller): axum::extract::State<C>,
                 htmx: crate::htmx::Request,
                 parts: http::request::Parts,
                 route: C::Route| async move {
                    let server_info: Arc<ServerInfo> = parts.extensions.get().cloned().expect(
                        "server info was not found in request extensions: this is not expected",
                    );

                    C::handle_request(&controller, route, htmx, parts, &server_info).await
                },
            )
            .with_state(controller);

        Self(router)
    }
}

impl From<ControllerRouter> for Router {
    fn from(controller_router: ControllerRouter) -> Self {
        controller_router.0
    }
}
