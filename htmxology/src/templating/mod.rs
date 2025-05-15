//! Templating facilities.

/// Render a template into an Axum response.
pub trait RenderIntoResponse: Sized {
    /// Render the template into a response.
    fn render_into_response(self) -> axum::response::Response {
        self.render_into_response_with_values(&())
    }

    /// Render the template into a response, with values.
    fn render_into_response_with_values(
        self,
        values: &dyn askama::Values,
    ) -> axum::response::Response;
}

#[cfg(feature = "templating")]
impl<T: askama::Template> RenderIntoResponse for T {
    fn render_into_response_with_values(
        self,
        values: &dyn askama::Values,
    ) -> axum::response::Response {
        use axum::response::IntoResponse;
        use tracing::error;

        match self.render_with_values(values) {
            Ok(body) => {
                let mut headers = http::HeaderMap::new();
                headers.insert(
                    http::header::CONTENT_TYPE,
                    http::HeaderValue::from_static("text/html; charset=utf-8"),
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
