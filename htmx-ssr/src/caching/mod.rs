//! Caching utilities.

/// A caching strategy.
#[derive(Debug, Clone, Default)]
pub struct CachingStrategy {}

impl CachingStrategy {
    /// Decorates an HTTP response with caching headers.
    pub fn add_caching_headers(
        &self,
        response: impl axum::response::IntoResponse,
    ) -> axum::response::Response {
        let mut response = response.into_response();

        // TODO: Implement real options here.

        response.headers_mut().insert(
            http::header::CACHE_CONTROL,
            http::header::HeaderValue::from_static("private, max-age=60"),
        );

        response
    }
}
