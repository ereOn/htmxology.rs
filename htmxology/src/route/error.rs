//! Error types for route parsing.

use std::fmt;

/// Error that can occur when parsing a route from a string.
///
/// This error is only relevant for GET routes, as only GET routes can be
/// parsed from URL strings (since they don't have request bodies).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The URL doesn't match any known GET route pattern.
    NoMatchingRoute { url: String },

    /// The URL matches a route, but that route is not a GET route.
    /// FromStr only works for GET routes since other methods may have request bodies.
    NotAGetRoute { url: String, method: String },

    /// Failed to parse a path parameter.
    PathParamParse {
        param_name: String,
        value: String,
        error: String,
    },

    /// Failed to parse the query string.
    QueryStringParse { error: String },

    /// A path parameter was missing from the URL.
    MissingPathParam { param_name: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMatchingRoute { url } => {
                write!(f, "No matching GET route found for URL: {}", url)
            }
            Self::NotAGetRoute { url, method } => {
                write!(
                    f,
                    "URL '{}' matches a {} route, but FromStr only works for GET routes",
                    url, method
                )
            }
            Self::PathParamParse {
                param_name,
                value,
                error,
            } => {
                write!(
                    f,
                    "Failed to parse path parameter '{}' from '{}': {}",
                    param_name, value, error
                )
            }
            Self::QueryStringParse { error } => {
                write!(f, "Failed to parse query string: {}", error)
            }
            Self::MissingPathParam { param_name } => {
                write!(f, "Missing required path parameter: {}", param_name)
            }
        }
    }
}

impl std::error::Error for ParseError {}
