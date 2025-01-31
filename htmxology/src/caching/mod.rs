//! Caching utilities.

mod controller;

use std::collections::BTreeSet;

use axum::response::IntoResponse;
pub use controller::{Controller, ControllerExt};
use tracing::{error, warn};

use crate::Route;

/// The default maximum body size.
const DEFAULT_MAX_BODY_SIZE: usize = 1024 * 1024; // 1 MB

/// The default caching duration.
const DEFAULT_CACHE_DURATION: std::time::Duration = std::time::Duration::from_secs(60);

/// A caching strategy.
pub struct Cache<R> {
    max_body_size: usize,
    cache_duration: std::time::Duration,
    _phantom: std::marker::PhantomData<R>,
}

impl<R> Default for Cache<R> {
    fn default() -> Self {
        Self {
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            cache_duration: DEFAULT_CACHE_DURATION,
            _phantom: Default::default(),
        }
    }
}

impl<R> Cache<R> {
    /// Create a new cache using in-memory storage.
    pub fn with_max_body_size(mut self, max_body_size: usize) -> Self {
        self.max_body_size = max_body_size;
        self
    }

    /// Set the cache duration.
    pub fn with_cache_duration(mut self, cache_duration: std::time::Duration) -> Self {
        self.cache_duration = cache_duration;
        self
    }
}

impl<R: Route> Cache<R> {
    /// Get the cache control for a request.
    pub fn get_cache_control(
        &self,
        route: &R,
        htmx: &crate::htmx::Request,
        parts: &http::request::Parts,
    ) -> CacheControl {
        // TODO: Allow customization of the cache key based on the request parameters.
        let _ = route;
        let _ = htmx;

        for cache_control_directive in parts
            .headers
            .get(http::header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim())
        {
            // This is a very simple implementation that only considers the `no-cache` and
            // `max-age=0` directives. But it'll do for now.
            if cache_control_directive == "no-cache" || cache_control_directive == "max-age=0" {
                return CacheControl::NoCache;
            }
        }

        let if_none_match = parts
            .headers
            .get_all(http::header::IF_NONE_MATCH)
            .into_iter()
            .flat_map(|value| match value.to_str() {
                Ok(value) => value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>(),
                Err(err) => {
                    warn!("Failed to parse If-None-Match header: {err}");

                    Vec::new()
                }
            })
            .collect();

        CacheControl::IfNoneMatch(if_none_match)
    }

    /// Decorates an HTTP response with caching headers, respecting the provided cache control
    /// directive.
    ///
    /// If the provided response already contains an ETag, it will be used directly. Otherwise, one
    /// will be computed from the response body, effectively disabling any streaming.
    ///
    /// If the provided response already contains a `Cache-Control` header, it will be left
    /// untouched.
    pub async fn check_cache_control(
        &self,
        cache_control: CacheControl,
        mut response: axum::response::Response,
    ) -> Result<axum::response::Response, axum::response::Response> {
        let mut response = match cache_control {
            CacheControl::IfNoneMatch(if_none_match) => {
                // If the response already has an ETag, we can use it directly.
                let etag = match response.headers().get(http::header::ETAG) {
                    Some(etag) => etag
                        .to_str()
                        .map_err(|err| {
                            error!("Failed to parse ETag header from response: {err}");

                            http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
                        })?
                        .to_string(),
                    None => {
                        let (parts, body) = response.into_parts();
                        let body = axum::body::to_bytes(body, self.max_body_size)
                            .await
                            .map_err(|err| {
                                error!("Failed to read response body: {err}");

                                http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
                            })?;

                        let etag = {
                            let mut hasher = blake3::Hasher::new();
                            hasher.update(&body);
                            hasher.finalize().to_hex().to_string()
                        };

                        response = http::Response::from_parts(parts, body.into());

                        etag
                    }
                };

                if if_none_match.contains(&etag.to_string()) {
                    let mut response = axum::response::Response::default();
                    *response.status_mut() = http::StatusCode::NOT_MODIFIED;

                    response
                } else {
                    response.with_etag(&etag)?
                }
            }
            CacheControl::NoCache => response,
        };

        // TODO: Implement real options here.

        if response
            .headers()
            .get(http::header::CACHE_CONTROL)
            .is_none()
        {
            response = response.with_caching(self.cache_duration);
        }

        Ok(response)
    }
}

/// An opaque cache key.
///
/// You should never need to instantiate this type directly nor should you store it across
/// requests.
///
/// The sole purpose of this type is to be used as a key for storing responses in the cache.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheControl {
    /// The request should be treated only if none of the provided ETags match.
    IfNoneMatch(BTreeSet<String>),

    /// The response should be treated as fresh.
    NoCache,
}

/// An extension trait for caching directives.
pub trait CachingResponseExt {
    /// Decorate the response with caching headers.
    fn with_caching_disabled(self) -> axum::response::Response;

    /// Decorate the response with a cache control directive.
    fn with_caching(self, duration: std::time::Duration) -> axum::response::Response;

    /// Add an ETag to the response.
    ///
    /// The etag value must be convertible to a valid HTTP header value or an error will be
    /// returned.
    fn with_etag(self, etag: &str) -> Result<axum::response::Response, axum::response::Response>;
}

impl CachingResponseExt for axum::response::Response {
    fn with_caching_disabled(mut self) -> axum::response::Response {
        self.headers_mut().insert(
            http::header::CACHE_CONTROL,
            http::header::HeaderValue::from_static("no-cache"),
        );

        self
    }

    fn with_caching(mut self, duration: std::time::Duration) -> axum::response::Response {
        let cache_control = http::header::HeaderValue::from_str(&format!(
            "private, max-age={}, must-revalidate",
            duration.as_secs()
        ))
        .expect("Failed to parse Cache-Control header");

        self.headers_mut()
            .insert(http::header::CACHE_CONTROL, cache_control);
        self.headers_mut().insert(
            http::header::VARY,
            http::header::HeaderValue::from_static("Hx-Request"),
        );

        self
    }

    fn with_etag(
        mut self,
        etag: &str,
    ) -> Result<axum::response::Response, axum::response::Response> {
        self.headers_mut().insert(
            http::header::ETAG,
            http::header::HeaderValue::from_str(etag).map_err(|err| {
                error!("Failed to parse ETag header: {err}");

                http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })?,
        );

        Ok(self)
    }
}
