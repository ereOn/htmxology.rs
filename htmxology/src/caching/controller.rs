use std::{future::Future, sync::Arc};

use tracing::warn;

/// A controller that adds caching strategy support to another controller.
///
/// Only requests that result in a `Result::Ok` from the `handle_request` method will be considered
/// for caching. Requests that result in a `Result::Err` will bypass the cache and be handled
/// directly by the inner controller.
pub struct Controller<C: crate::Controller> {
    pub controller: C,
    pub cache: Arc<super::Cache<C::Route>>,
}

impl<C: crate::Controller> Clone for Controller<C> {
    fn clone(&self) -> Self {
        Self {
            controller: self.controller.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl<C> crate::Controller for Controller<C>
where
    C: crate::Controller<Response = Result<axum::response::Response, axum::response::Response>>,
    C::Route: crate::Route + Send + Sync + axum::extract::FromRequest<Self>,
{
    type Route = C::Route;
    type Args = C::Args;
    type Response = Result<axum::response::Response, axum::response::Response>;

    fn handle_request(
        &self,
        route: Self::Route,
        htmx: crate::htmx::Request,
        parts: http::request::Parts,
        server_info: &crate::ServerInfo,
    ) -> impl Future<Output = Self::Response> + Send {
        let cache_control = self.cache.get_cache_control(&route, &htmx, &parts);
        let url = route.to_string();

        async move {
            let response = self
                .controller
                .handle_request(route, htmx, parts, server_info)
                .await?;

            self.cache
                .check_cache_control(cache_control, response)
                .await
                .inspect_err(|_| warn!("Cache control failed for route: {url}"))
        }
    }
}

/// An extension trait for controllers that adds caching strategy support.
pub trait ControllerExt: crate::Controller {
    fn with_cache(self, cache: super::Cache<Self::Route>) -> Controller<Self>
    where
        Self: Sized;
}

impl<C: crate::Controller> ControllerExt for C {
    fn with_cache(self, cache: super::Cache<C::Route>) -> Controller<Self> {
        let cache = Arc::new(cache);

        Controller {
            controller: self,
            cache,
        }
    }
}
