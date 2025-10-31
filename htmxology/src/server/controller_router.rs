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

    /// Create a new controller router from a controller with an args factory.
    ///
    /// The factory function is called on each request with a reference to the controller
    /// to create the Args for that request.
    pub fn new<C, F, Fut>(controller: C, args_factory: F) -> Self
    where
        C: Controller<Response = Result<axum::response::Response, axum::response::Response>>
            + 'static,
        F: Fn(&C) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = C::Args> + Send,
    {
        let router = Router::new()
            .fallback(
                move |axum::extract::State(controller): axum::extract::State<C>,
                      htmx: crate::htmx::Request,
                      parts: http::request::Parts,
                      route: C::Route| {
                    let args_factory = args_factory.clone();
                    async move {
                        let server_info: Arc<ServerInfo> = parts.extensions.get().cloned().expect(
                            "server info was not found in request extensions: this is not expected",
                        );

                        // Call the factory to create args for this request
                        let args = args_factory(&controller).await;
                        C::handle_request(&controller, route, htmx, parts, &server_info, args).await
                    }
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

impl<C> From<crate::caching::Controller<C>> for ControllerRouter
where
    C: crate::Controller<Response = Result<axum::response::Response, axum::response::Response>>
        + 'static,
    C::Route:
        crate::Route + Send + Sync + axum::extract::FromRequest<crate::caching::Controller<C>>,
    C::Args: Default,
{
    fn from(controller: crate::caching::Controller<C>) -> Self {
        ControllerRouter::new(controller, |_| async { C::Args::default() })
    }
}
