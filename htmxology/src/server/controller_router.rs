use std::{convert::Infallible, sync::Arc};

use axum::{
    Router,
    extract::Request,
    response::IntoResponse,
    routing::{MethodRouter, Route},
};
use tower_layer::Layer;
use tower_service::Service;

use crate::Controller;

use super::ServerInfo;

/// An Axum router whose fallback handler dispatches requests to a [`Controller`].
///
/// Create one with [`ControllerRouter::new`], or with [`From`] for a controller whose `Args`
/// implement `Default`. Routes, services and middleware can be added next to the controller with
/// the builder methods; they take precedence over the controller, which only sees the requests
/// that no explicit route matched.
///
/// A `ControllerRouter` is either handed to [`Server::serve`](crate::Server::serve), or turned
/// into a plain [`Router`] with [`into_router`](Self::into_router) to be served, tested or merged
/// into another application by other means.
///
/// # Example
///
/// ```rust,ignore
/// let router = ControllerRouter::from(AppController)
///     .route("/health", axum::routing::get(|| async { "ok" }))
///     .layer(tower_http::trace::TraceLayer::new_for_http());
///
/// server.serve(router).await?;
/// ```
#[derive(Debug, Clone)]
pub struct ControllerRouter(Router);

impl ControllerRouter {
    /// Create a new controller router from a controller with an args factory.
    ///
    /// The factory function is called on each request with a reference to the controller
    /// to create the `Args` for that request.
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
                        let Some(server_info) = parts.extensions.get::<Arc<ServerInfo>>().cloned()
                        else {
                            tracing::error!(
                                "`ServerInfo` is missing from the request extensions: build the \
                                 router with `ControllerRouter::into_router` or serve it through \
                                 `Server::serve`"
                            );

                            return Err(http::StatusCode::INTERNAL_SERVER_ERROR.into_response());
                        };

                        // Call the factory to create args for this request
                        let args = args_factory(&controller).await;
                        C::handle_request(&controller, route, htmx, parts, &server_info, args).await
                    }
                },
            )
            .with_state(controller);

        Self(router)
    }

    /// Add a route next to the controller.
    ///
    /// See [`Router::route`].
    pub fn route(self, path: &str, method_router: MethodRouter) -> Self {
        Self(self.0.route(path, method_router))
    }

    /// Add a service at a path, next to the controller.
    ///
    /// See [`Router::route_service`].
    pub fn route_service<T>(self, path: &str, service: T) -> Self
    where
        T: Service<Request, Error = Infallible> + Clone + Send + Sync + 'static,
        T::Response: IntoResponse,
        T::Future: Send + 'static,
    {
        Self(self.0.route_service(path, service))
    }

    /// Nest a router under a path prefix, next to the controller.
    ///
    /// See [`Router::nest`].
    pub fn nest(self, path: &str, router: Router) -> Self {
        Self(self.0.nest(path, router))
    }

    /// Nest a service under a path prefix, next to the controller.
    ///
    /// This is the usual way to serve static files (for instance with `tower_http::services::ServeDir`).
    ///
    /// See [`Router::nest_service`].
    pub fn nest_service<T>(self, path: &str, service: T) -> Self
    where
        T: Service<Request, Error = Infallible> + Clone + Send + Sync + 'static,
        T::Response: IntoResponse,
        T::Future: Send + 'static,
    {
        Self(self.0.nest_service(path, service))
    }

    /// Merge the routes of another router into this one.
    ///
    /// The other router must not have a fallback of its own: the controller is the fallback.
    ///
    /// See [`Router::merge`].
    pub fn merge<R>(self, other: R) -> Self
    where
        R: Into<Router>,
    {
        Self(self.0.merge(other))
    }

    /// Apply a middleware layer to every route, including the controller.
    ///
    /// See [`Router::layer`].
    pub fn layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        Self(self.0.layer(layer))
    }

    /// Apply a middleware layer to the explicitly added routes only, not to the controller.
    ///
    /// See [`Router::route_layer`].
    pub fn route_layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        Self(self.0.route_layer(layer))
    }

    /// Transform the underlying router with any [`Router`] method not exposed here.
    ///
    /// The closure must return a router that still contains the controller fallback (that is, a
    /// router derived from the one it was given); otherwise the controller is never reached.
    pub fn map_router(self, f: impl FnOnce(Router) -> Router) -> Self {
        Self(f(self.0))
    }

    /// Turn this into a plain Axum [`Router`], making `server_info` available to the controller.
    ///
    /// Use this to serve the application with `axum::serve` directly, to merge it into an
    /// existing Axum application, or to drive it in tests with `tower::ServiceExt::oneshot`.
    /// [`Server::serve`](crate::Server::serve) calls this for you with the base URL it determined.
    pub fn into_router(self, server_info: ServerInfo) -> Router {
        self.0
            .layer(axum::extract::Extension(Arc::new(server_info)))
    }
}

/// Any controller with the root response type and `Args` that can be defaulted is a router.
///
/// For other `Args`, use [`ControllerRouter::new`] with an args factory.
impl<C> From<C> for ControllerRouter
where
    C: Controller<Response = Result<axum::response::Response, axum::response::Response>> + 'static,
    C::Args: Default,
{
    fn from(controller: C) -> Self {
        ControllerRouter::new(controller, |_: &C| async { C::Args::default() })
    }
}
