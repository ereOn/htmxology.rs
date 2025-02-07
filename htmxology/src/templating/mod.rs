//! Templating facilities.

use axum::response::IntoResponse;
use tracing::error;

/// Render a template into an Axum response.
pub trait RenderIntoResponse {
    /// Render the template into a response.
    fn render_into_response(self) -> axum::response::Response;
}

#[cfg(feature = "templating")]
impl<T: askama::Template> RenderIntoResponse for T {
    fn render_into_response(self) -> axum::response::Response {
        match self.render() {
            Ok(body) => {
                let mut headers = http::HeaderMap::new();
                headers.insert(
                    http::header::CONTENT_TYPE,
                    http::HeaderValue::from_static(T::MIME_TYPE),
                );

                (http::StatusCode::OK, headers, body).into_response()
            }
            Err(err) => {
                error!(
                    "Failed to render template `{}`: {err}",
                    std::any::type_name::<T>()
                );

                http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
