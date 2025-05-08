//! The route trait.

use std::fmt::Display;

use axum::response::IntoResponse;
use de::PathArgumentDeserializer;
use http::uri::PathAndQuery;

mod de;

/// The route trait can be implemented for types that represent a possible set of routes in an
/// application.
///
/// Typically implemented through the `Route` derive macro.
pub trait Route: Display {
    /// Get the method for the route.
    fn method(&self) -> http::Method;

    /// Get a HTMX attribute for the route.
    fn as_htmx_attribute(&self) -> String {
        format!(r#"hx-{}="{self}""#, self.method().as_str().to_lowercase())
    }

    /// Get an absolute URL for the route.
    fn to_absolute_url(&self, base_url: &http::Uri) -> String {
        format!("{}/{}", base_url, self)
    }
}

/// An extension trait for routes.
pub trait RouteExt: Route {
    /// Turn the route into a redirect response.
    fn as_redirect_response(&self) -> axum::response::Response {
        http::Response::builder()
            .status(http::StatusCode::SEE_OTHER)
            .header(http::header::LOCATION, self.to_string())
            .body(axum::body::Body::empty())
            .expect("failed to create redirect response")
    }
}

impl<T: Route> RouteExt for T {}

/// Decode a path argument into a value.
pub fn decode_path_argument<T: serde::de::DeserializeOwned>(
    key: &'static str,
    value: &str,
) -> Result<T, axum::response::Response> {
    let value = T::deserialize(PathArgumentDeserializer::new(value)).map_err(|err| {
        (
            http::StatusCode::BAD_REQUEST,
            format!("error while deserializing argument `{key}`: {err}"),
        )
            .into_response()
    })?;

    Ok(value)
}

/// Replace the path in a request.
pub fn replace_request_path<B>(req: http::Request<B>, path: String) -> http::Request<B> {
    let (mut parts, body) = req.into_parts();
    let mut uri_parts = parts.uri.into_parts();
    let path_and_query = uri_parts
        .path_and_query
        .expect("URI must have a path and query");
    uri_parts.path_and_query = PathAndQuery::from_maybe_shared(match path_and_query.query() {
        Some(query) => format!("{path}?{query}"),
        None => path.to_string(),
    })
    .map(Some)
    .expect("failed to create new path and query");

    parts.uri = http::Uri::from_parts(uri_parts).expect("failed to create new URI");

    http::Request::from_parts(parts, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_as_htmx_attribute() {
        #[derive(Debug, Clone, Copy)]
        struct TestRoute;

        impl Display for TestRoute {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "/test/route")
            }
        }

        impl Route for TestRoute {
            fn method(&self) -> http::Method {
                http::Method::GET
            }
        }

        let route = TestRoute;
        assert_eq!(route.as_htmx_attribute(), r#"hx-get="/test/route""#);
    }
}
