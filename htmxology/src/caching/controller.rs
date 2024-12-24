use std::sync::Arc;

use axum::async_trait;
use tracing::warn;

/// A controller that adds caching strategy support to another controller.
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

#[async_trait]
impl<C> crate::Controller for Controller<C>
where
    C: crate::Controller,
    C::Route: crate::Route + Send + Sync + axum::extract::FromRequest<crate::ServerState<Self>>,
{
    type Route = C::Route;

    async fn render_view(
        &self,
        route: Self::Route,
        htmx: crate::htmx::Request,
        parts: http::request::Parts,
        server_info: &crate::ServerInfo,
    ) -> axum::response::Response {
        let cache_control = self.cache.get_cache_control(&route, &htmx, &parts);
        let url = route.to_string();
        let response = self
            .controller
            .render_view(route, htmx, parts, server_info)
            .await;

        match self
            .cache
            .check_cache_control(cache_control, response)
            .await
        {
            Ok(response) => response,
            Err(response) => {
                warn!("Cache control failed for route: {url}");

                response
            }
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
