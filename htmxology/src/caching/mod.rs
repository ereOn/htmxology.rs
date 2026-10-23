//! Caching utilities.

mod controller;

use std::collections::BTreeSet;

use axum::response::IntoResponse;
pub use controller::{Controller, ControllerExt};
use md5::Digest;
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

        // Only safe, idempotent requests are candidates for validation and caching: decorating
        // the response to a `POST`/`DELETE` with an ETag (and possibly turning it into a `304`)
        // makes no sense and could hide the result of the side effect from the client.
        if !matches!(parts.method, http::Method::GET | http::Method::HEAD) {
            return CacheControl::Bypass;
        }

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

        // Entity tags are compared with the weak comparison function (RFC 9110 §8.8.3.2), so the
        // `W/` prefix is dropped and `*` matches any representation.
        let if_none_match = parts
            .headers
            .get_all(http::header::IF_NONE_MATCH)
            .into_iter()
            .flat_map(|value| match value.to_str() {
                Ok(value) => value
                    .split(',')
                    .map(|s| s.trim().trim_start_matches("W/").to_string())
                    .filter(|s| !s.is_empty())
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
    /// Only successful (2xx) responses that do not set cookies are decorated: redirects, error
    /// pages and responses carrying a `Set-Cookie` header are returned untouched.
    ///
    /// If the provided response already contains an ETag, it will be used directly. Otherwise, one
    /// will be computed from the response body, effectively disabling any streaming. Responses
    /// whose body is larger than the configured maximum body size, or whose size is unknown
    /// (streaming bodies), are returned untouched rather than buffered.
    ///
    /// If the provided response already contains a `Cache-Control` header, it will be left
    /// untouched. `Vary` is extended (not replaced) with the HTMX request headers that may affect
    /// the rendered output.
    pub async fn check_cache_control(
        &self,
        cache_control: CacheControl,
        mut response: axum::response::Response,
    ) -> Result<axum::response::Response, axum::response::Response> {
        if !response.status().is_success()
            || response.headers().contains_key(http::header::SET_COOKIE)
        {
            return Ok(response);
        }

        let mut response = match cache_control {
            CacheControl::Bypass => return Ok(response),
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
                        use axum::body::HttpBody as _;

                        // Never buffer bodies that are too large or of unknown size: pass them
                        // through without validators instead of failing the request.
                        match response.body().size_hint().upper() {
                            Some(upper) if upper as usize <= self.max_body_size => {}
                            _ => return Ok(response),
                        }

                        let (parts, body) = response.into_parts();
                        let body = axum::body::to_bytes(body, self.max_body_size)
                            .await
                            .map_err(|err| {
                                error!("Failed to read response body: {err}");

                                http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
                            })?;

                        let etag = {
                            let mut hasher = md5::Md5::new();

                            hasher.update(&body);
                            format!("\"{}\"", hex::encode(hasher.finalize()))
                        };

                        response = http::Response::from_parts(parts, body.into());

                        etag
                    }
                };

                let matches = if_none_match.contains("*")
                    || if_none_match.contains(etag.trim_start_matches("W/"));

                if matches {
                    // A `304` must carry the same metadata headers as the `200` would have
                    // (`ETag`, `Cache-Control`, `Vary`, ...), so only the body is dropped.
                    let (mut parts, _) = response.into_parts();
                    parts.status = http::StatusCode::NOT_MODIFIED;
                    parts.headers.remove(http::header::CONTENT_LENGTH);
                    parts.headers.remove(http::header::CONTENT_TYPE);

                    http::Response::from_parts(parts, axum::body::Body::empty()).with_etag(&etag)?
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

    /// The request is not a candidate for caching (e.g. a non-`GET`/`HEAD` request) and its
    /// response must be returned untouched.
    Bypass,
}

/// An extension trait for caching directives.
pub trait CachingResponseExt {
    /// Decorate the response with caching headers, instructing clients and intermediate proxies to
    /// not cache the response.
    fn with_caching_disabled(self) -> axum::response::Response;

    /// Decorate the response with a cache control directive.
    ///
    /// The response will be cached for the provided duration, using the `private` directive.
    ///
    /// The `Vary` header is extended with the HTMX request headers that commonly change the
    /// rendered output, so that a cached fragment is never served in place of a full page (or
    /// vice-versa). Any existing `Vary` value is preserved.
    fn with_caching(self, duration: std::time::Duration) -> axum::response::Response;

    /// Add an ETag to the response.
    ///
    /// The etag value must be convertible to a valid HTTP header value or an error will be
    /// returned.
    #[expect(clippy::result_large_err)]
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

        const HTMX_VARY: [&str; 4] = ["hx-request", "hx-target", "hx-boosted", "hx-current-url"];

        let existing: Vec<String> = self
            .headers()
            .get_all(http::header::VARY)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();

        let vary = if existing.iter().any(|value| value == "*") {
            "*".to_string()
        } else {
            existing
                .iter()
                .map(String::as_str)
                .chain(
                    HTMX_VARY
                        .into_iter()
                        .filter(|name| !existing.iter().any(|v| v.eq_ignore_ascii_case(name))),
                )
                .collect::<Vec<_>>()
                .join(", ")
        };

        self.headers_mut().insert(
            http::header::VARY,
            http::header::HeaderValue::from_str(&vary).expect("Vary header values are valid"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct TestRoute;

    impl std::fmt::Display for TestRoute {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "/test")
        }
    }

    impl std::str::FromStr for TestRoute {
        type Err = crate::route::ParseError;

        fn from_str(_s: &str) -> Result<Self, Self::Err> {
            Ok(TestRoute)
        }
    }

    impl Route for TestRoute {
        fn method(&self) -> http::Method {
            http::Method::GET
        }
    }

    fn parts(method: http::Method, headers: &[(http::HeaderName, &str)]) -> http::request::Parts {
        let mut builder = http::Request::builder().method(method).uri("/test");

        for (name, value) in headers {
            builder = builder.header(name, *value);
        }

        builder.body(()).unwrap().into_parts().0
    }

    fn html(status: http::StatusCode, body: &'static str) -> axum::response::Response {
        (
            status,
            [(http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            body,
        )
            .into_response()
    }

    async fn body_string(response: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn header(response: &axum::response::Response, name: http::HeaderName) -> Option<&str> {
        response.headers().get(name).map(|v| v.to_str().unwrap())
    }

    #[tokio::test]
    async fn get_response_gets_quoted_etag_and_vary() {
        let cache = Cache::<TestRoute>::default();
        let cache_control = cache.get_cache_control(
            &TestRoute,
            &crate::htmx::Request::Classic,
            &parts(http::Method::GET, &[]),
        );

        let response = cache
            .check_cache_control(cache_control, html(http::StatusCode::OK, "<p>hello</p>"))
            .await
            .unwrap();

        let etag = header(&response, http::header::ETAG).unwrap().to_owned();
        assert!(etag.starts_with('"') && etag.ends_with('"'), "etag: {etag}");
        assert_eq!(
            header(&response, http::header::CACHE_CONTROL),
            Some("private, max-age=60, must-revalidate")
        );
        assert_eq!(
            header(&response, http::header::VARY),
            Some("hx-request, hx-target, hx-boosted, hx-current-url")
        );
        assert_eq!(body_string(response).await, "<p>hello</p>");

        // A matching `If-None-Match` (with or without a weak prefix) yields a `304` that keeps
        // the validator and caching headers but drops the body.
        for if_none_match in [
            etag.clone(),
            format!("W/{etag}"),
            format!("\"other\", {etag}"),
            "*".to_owned(),
        ] {
            let cache_control = cache.get_cache_control(
                &TestRoute,
                &crate::htmx::Request::Classic,
                &parts(
                    http::Method::GET,
                    &[(http::header::IF_NONE_MATCH, &if_none_match)],
                ),
            );

            let response = cache
                .check_cache_control(cache_control, html(http::StatusCode::OK, "<p>hello</p>"))
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                http::StatusCode::NOT_MODIFIED,
                "{if_none_match}"
            );
            assert_eq!(header(&response, http::header::ETAG), Some(etag.as_str()));
            assert!(header(&response, http::header::CACHE_CONTROL).is_some());
            assert!(header(&response, http::header::VARY).is_some());
            assert_eq!(header(&response, http::header::CONTENT_LENGTH), None);
            assert_eq!(body_string(response).await, "");
        }
    }

    #[tokio::test]
    async fn existing_vary_is_preserved() {
        let response = html(http::StatusCode::OK, "x");
        let mut response = response;
        response.headers_mut().insert(
            http::header::VARY,
            http::HeaderValue::from_static("Cookie, HX-Request"),
        );

        let response = response.with_caching(std::time::Duration::from_secs(10));

        assert_eq!(
            header(&response, http::header::VARY),
            Some("Cookie, HX-Request, hx-target, hx-boosted, hx-current-url")
        );
    }

    #[tokio::test]
    async fn non_get_and_non_success_and_cookie_responses_are_untouched() {
        let cache = Cache::<TestRoute>::default();

        // POST requests bypass the cache entirely.
        let cache_control = cache.get_cache_control(
            &TestRoute,
            &crate::htmx::Request::Classic,
            &parts(http::Method::POST, &[]),
        );
        assert_eq!(cache_control, CacheControl::Bypass);
        let response = cache
            .check_cache_control(cache_control, html(http::StatusCode::OK, "x"))
            .await
            .unwrap();
        assert_eq!(header(&response, http::header::ETAG), None);
        assert_eq!(header(&response, http::header::CACHE_CONTROL), None);

        // Redirects and errors are untouched, even on GET.
        for status in [http::StatusCode::SEE_OTHER, http::StatusCode::NOT_FOUND] {
            let cache_control = cache.get_cache_control(
                &TestRoute,
                &crate::htmx::Request::Classic,
                &parts(http::Method::GET, &[]),
            );
            let response = cache
                .check_cache_control(cache_control, html(status, "x"))
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            assert_eq!(header(&response, http::header::ETAG), None);
            assert_eq!(header(&response, http::header::CACHE_CONTROL), None);
        }

        // Responses that set cookies are untouched.
        let cache_control = cache.get_cache_control(
            &TestRoute,
            &crate::htmx::Request::Classic,
            &parts(http::Method::GET, &[]),
        );
        let mut response = html(http::StatusCode::OK, "x");
        response.headers_mut().insert(
            http::header::SET_COOKIE,
            http::HeaderValue::from_static("a=b"),
        );
        let response = cache
            .check_cache_control(cache_control, response)
            .await
            .unwrap();
        assert_eq!(header(&response, http::header::ETAG), None);
        assert_eq!(header(&response, http::header::CACHE_CONTROL), None);
    }

    #[tokio::test]
    async fn oversized_body_is_passed_through_instead_of_failing() {
        let cache = Cache::<TestRoute>::default().with_max_body_size(4);
        let cache_control = cache.get_cache_control(
            &TestRoute,
            &crate::htmx::Request::Classic,
            &parts(http::Method::GET, &[]),
        );

        let response = cache
            .check_cache_control(
                cache_control,
                html(http::StatusCode::OK, "this is too large"),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), http::StatusCode::OK);
        assert_eq!(header(&response, http::header::ETAG), None);
        assert_eq!(body_string(response).await, "this is too large");
    }
}
