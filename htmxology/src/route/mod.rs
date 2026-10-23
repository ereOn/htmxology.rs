//! The route trait.

use std::fmt::Display;
use std::str::FromStr;

use axum::response::IntoResponse;
use de::PathArgumentDeserializer;
use http::uri::PathAndQuery;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};

mod de;
mod error;

pub use error::ParseError;

/// The set of bytes that must be percent-encoded in a single URL path segment.
///
/// This keeps the RFC 3986 unreserved characters and most sub-delimiters as-is, but encodes `/`
/// (so that a value can never span several segments), `?`, `#`, `%` and the characters that are
/// significant in HTML attributes (`"`, `'`, `<`, `>`, `&`).
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'!')
    .remove(b'$')
    .remove(b'(')
    .remove(b')')
    .remove(b'*')
    .remove(b'+')
    .remove(b',')
    .remove(b';')
    .remove(b'=')
    .remove(b':')
    .remove(b'@');

/// A wrapper that formats a path parameter as a percent-encoded URL path segment.
///
/// The `Route` derive macro uses this when rendering routes with `Display`, so that path
/// parameters containing `/`, `?`, `#`, whitespace or HTML-significant characters produce a
/// well-formed URL that round-trips through `FromStr`/`FromRequest` and is safe to embed in an
/// HTML attribute.
pub struct PathSegment<T>(pub T);

impl<T: Display> Display for PathSegment<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = self.0.to_string();

        percent_encoding::utf8_percent_encode(&raw, PATH_SEGMENT).fmt(f)
    }
}

/// Parse a path parameter from its raw (percent-encoded) URL path segment.
///
/// The `Route` derive macro uses this in its `FromStr` implementation so that parsing a URL
/// string decodes path parameters exactly like `FromRequest` does.
pub fn parse_path_param<T>(name: &str, raw: &str) -> Result<T, ParseError>
where
    T: FromStr,
    T::Err: Display,
{
    let value = percent_encoding::percent_decode_str(raw)
        .decode_utf8()
        .map_err(|err| ParseError::PathParamParse {
            param_name: name.to_owned(),
            value: raw.to_owned(),
            error: err.to_string(),
        })?;

    value
        .parse()
        .map_err(|err: T::Err| ParseError::PathParamParse {
            param_name: name.to_owned(),
            value: value.into_owned(),
            error: err.to_string(),
        })
}

/// The route trait can be implemented for types that represent a possible set of routes in an
/// application.
///
/// Typically implemented through the `Route` derive macro.
///
/// The `FromStr` implementation allows parsing URL paths into route instances, but only
/// for GET routes (routes without request bodies). Attempting to parse a URL that maps
/// to a non-GET route will return a `ParseError`.
pub trait Route: Display + FromStr<Err = ParseError> {
    /// Get the method for the route.
    fn method(&self) -> http::Method;

    /// Get a HTMX attribute for the route.
    fn as_htmx_attribute(&self) -> String {
        format!(r#"hx-{}="{self}""#, self.method().as_str().to_lowercase())
    }

    /// Get an absolute URL for the route.
    ///
    /// Routes always render with a leading `/`, so any trailing `/` on the base URL is dropped to
    /// avoid producing a `//` in the result.
    fn to_absolute_url(&self, base_url: &http::Uri) -> String {
        let base_url = base_url.to_string();

        format!("{}{}", base_url.trim_end_matches('/'), self)
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
#[expect(clippy::result_large_err)]
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

        impl FromStr for TestRoute {
            type Err = ParseError;

            fn from_str(_s: &str) -> Result<Self, Self::Err> {
                Ok(TestRoute)
            }
        }

        impl Route for TestRoute {
            fn method(&self) -> http::Method {
                http::Method::GET
            }
        }

        let route = TestRoute;
        assert_eq!(route.as_htmx_attribute(), r#"hx-get="/test/route""#);
        assert_eq!(
            route.to_absolute_url(&"http://localhost:3000".parse().unwrap()),
            "http://localhost:3000/test/route"
        );
        assert_eq!(
            route.to_absolute_url(&"http://localhost:3000/app/".parse().unwrap()),
            "http://localhost:3000/app/test/route"
        );
    }

    #[test]
    fn test_path_segment_encoding() {
        assert_eq!(
            PathSegment("plain-value_1.2~").to_string(),
            "plain-value_1.2~"
        );
        assert_eq!(PathSegment(42).to_string(), "42");
        assert_eq!(
            PathSegment(r#"a/b?c#d%e f"g'h<i>j&k"#).to_string(),
            "a%2Fb%3Fc%23d%25e%20f%22g%27h%3Ci%3Ej%26k"
        );
        assert_eq!(PathSegment("..").to_string(), "..");
        assert_eq!(PathSegment("héllo").to_string(), "h%C3%A9llo");
    }

    #[test]
    fn test_parse_path_param_round_trip() {
        for value in ["plain", r#"a/b?c#d%e f"g'h<i>j&k"#, "héllo", ".."] {
            let encoded = PathSegment(value).to_string();
            assert!(!encoded.contains('/') || value == "/");
            let decoded: String = parse_path_param("value", &encoded).unwrap();
            assert_eq!(decoded, value);
        }

        assert_eq!(parse_path_param::<u32>("id", "42").unwrap(), 42);

        // Percent-decoding happens before parsing.
        assert_eq!(parse_path_param::<u32>("id", "4%32").unwrap(), 42);

        let err = parse_path_param::<u32>("id", "abc").unwrap_err();
        assert!(matches!(err, ParseError::PathParamParse { ref value, .. } if value == "abc"));

        let err = parse_path_param::<String>("id", "%ff").unwrap_err();
        assert!(matches!(err, ParseError::PathParamParse { ref value, .. } if value == "%ff"));
    }
}
