//! A route URL.

use std::{collections::BTreeMap, fmt::Display, str::FromStr};

use itertools::{Itertools, Position};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Ident;

/// A route URL.
///
/// Route URLs are used to match incoming requests to the appropriate controller method.
///
/// They can *NEVER* start with a slash except for the single slash URL `/`, where the slash is
/// considered being at the end of the URL.
///
/// If they end with a slash, they will be considered as a path prefix.
///
/// Route URLs can be empty, usually to indicate the `index` resource.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteUrl(Vec<RouteUrlSegment>);

/// A route URL segment.
///
/// This type represents a segment of a route URL path, which can be either a slash separator, a
/// static segment, or a path parameter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RouteUrlSegment {
    /// A slash separator.
    Separator,

    //// A path parameter.
    Parameter {
        /// The name of the parameter and its identifier.
        name: String,
    },

    /// A static segment.
    Literal(String),
}

/// An error that can occur when parsing a route URL.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// The route URL starts with a slash.
    #[error("the route URL starts with a slash which is not allowed")]
    LeadingSlash,

    /// The path contains an invalid character.
    #[error("the path contains an unexpected character (`{character}`)")]
    UnexpectedCharacter {
        /// The position at which the invalid character was found.
        position: usize,

        /// The invalid character.
        character: char,
    },

    /// A path parameter is not allowed here.
    #[error("a path parameter can only appear directly after a slash separator")]
    ParameterNotAllowed {
        /// The position at which the path parameter was found.
        position: usize,
    },

    /// A path parameter contains an invalid character.
    #[error("the path parameter contains an invalid character (`{character}`)")]
    InvalidParameterCharacter {
        /// The position at which the path parameter was opened.
        start: usize,

        /// The position at which the invalid character was found.
        position: usize,

        /// The invalid character.
        character: char,
    },

    /// A path parameter is not closed.
    #[error("the path parameter is not closed")]
    UnclosedParameter {
        /// The position at which the path parameter was opened.
        start: usize,

        /// The end position.
        end: usize,
    },
}

impl ParseError {
    /// Returns the range of characters that caused the error.
    pub fn range(&self) -> std::ops::RangeInclusive<usize> {
        match self {
            Self::LeadingSlash => 0..=1,
            Self::UnexpectedCharacter { position, .. } => *position..=*position,
            Self::ParameterNotAllowed { position } => *position..=*position,
            Self::InvalidParameterCharacter {
                start, position, ..
            } => *start..=*position,
            Self::UnclosedParameter { start, end } => *start..=*end,
        }
    }

    /// Get a detailled error message with the specific position of the error.
    pub fn detail(&self, s: &str) -> String {
        let range = self.range();
        let mut result = String::with_capacity(s.len() + 16);

        result.push_str(&s[..*range.start()]);
        result.push('^');
        result.push_str(&s[*range.start()..=*range.end()]);
        result.push('^');
        result.push_str(&s[*range.end() + 1..]);

        result
    }
}

impl RouteUrl {
    /// Check if the route URL is a prefix.
    pub fn is_prefix(&self) -> bool {
        matches!(self.0.last(), Some(RouteUrlSegment::Separator) if self.0.len() > 1)
    }

    /// Get an Axum router path from the route URL path.
    pub fn to_path_regex(&self) -> String {
        // As good a guess as any...
        let mut result = String::with_capacity(64);

        result.push('^');

        let segments = self.0.iter().with_position();

        for (position, segment) in segments {
            match (position, &segment) {
                (Position::Last, RouteUrlSegment::Separator) => {
                    result.push_str("(?P<subroute>/.*)");
                }
                (_, RouteUrlSegment::Separator) => result.push('/'),
                (_, RouteUrlSegment::Literal(s)) => result.push_str(s),
                (_, RouteUrlSegment::Parameter { name }) => {
                    result.push_str("(?P<");
                    result.push_str(name);
                    result.push_str(">[^/]+)");
                }
            }
        }

        result.push('$');

        result
    }

    /// Get the URL as a list of format statements, failing if there are any required path parameters.
    pub fn to_unparameterized_string(&self, ctx: impl ToTokens) -> syn::Result<Vec<TokenStream>> {
        let mut statements = Vec::with_capacity(self.0.len());

        for segment in &self.0 {
            match segment {
                RouteUrlSegment::Separator => {
                    statements.push(quote! {std::fmt::Write::write_char(f, '/')?;})
                }
                RouteUrlSegment::Literal(s) => statements.push(quote! {f.write_str(#s)?;}),
                RouteUrlSegment::Parameter { name } => {
                    return Err(syn::Error::new_spanned(ctx, format!("the URL contains a required path parameter `{name}`, which cannot be formatted without parameters")));
                }
            }
        }

        Ok(statements)
    }

    /// Get the URL as a list of format statements, with its name path parameters resolved.
    pub fn to_named_parameters_format(
        &self,
        ctx: &impl ToTokens,
        name_params: impl IntoIterator<Item = (String, Ident)>,
    ) -> syn::Result<Vec<TokenStream>> {
        let mut statements = Vec::with_capacity(self.0.len());
        let mut name_params = name_params.into_iter().collect::<BTreeMap<_, _>>();

        for element in &self.0 {
            match element {
                RouteUrlSegment::Separator => {
                    statements.push(quote! {std::fmt::Write::write_char(f, '/')?;})
                }
                RouteUrlSegment::Literal(s) => statements.push(quote! {f.write_str(#s)?;}),
                RouteUrlSegment::Parameter { name } => {
                    let ident = name_params.remove(name.as_str()).ok_or_else(|| {
                        syn::Error::new_spanned(
                            ctx,
                            format!("missing named path parameter `{name}`"),
                        )
                    })?;

                    statements.push(quote! {#ident.fmt(f)?;});
                }
            }
        }

        if let Some((name, ident)) = name_params.into_iter().next() {
            return Err(syn::Error::new_spanned(
                ident,
                format!("the URL does not contain all named path parameters (missing: {name})"),
            ));
        }

        Ok(statements)
    }

    /// Get the URL as a list of format statements, with its unnamed path parameters resolved.
    pub fn to_unnamed_parameters_format(
        &self,
        ctx: &impl ToTokens,
        unnamed_params: impl IntoIterator<Item = Ident>,
    ) -> syn::Result<Vec<TokenStream>> {
        let mut statements = Vec::with_capacity(self.0.len());
        let mut unnamed_params = unnamed_params.into_iter();

        for element in &self.0 {
            match element {
                RouteUrlSegment::Separator => {
                    statements.push(quote! {std::fmt::Write::write_char(f, '/')?;})
                }
                RouteUrlSegment::Literal(s) => statements.push(quote! {f.write_str(#s)?;}),
                RouteUrlSegment::Parameter { .. } => {
                    let ident = unnamed_params.next().ok_or_else(|| {
                        syn::Error::new_spanned(
                            ctx,
                            "the URL contains more path parameters than route has arguments",
                        )
                    })?;

                    statements.push(quote! {#ident.fmt(f)?;});
                }
            }
        }

        if let Some(ident) = unnamed_params.next() {
            return Err(syn::Error::new_spanned(
                ident,
                "the URL does not contain all unnamed path parameters",
            ));
        }

        Ok(statements)
    }
}

impl FromStr for RouteUrl {
    type Err = ParseError;

    /// Parses a route URL from a static string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars().enumerate().peekable();

        // Special case: a single slash is a valid route URL prefix.
        if s == "/" {
            return Ok(Self(vec![
                RouteUrlSegment::Separator,
                RouteUrlSegment::Separator,
            ]));
        }

        if chars.peek() == Some(&(0, '/')) {
            return Err(ParseError::LeadingSlash);
        }

        // Even though we don't want the string to start with a slash, we still want to add one in
        // the internal representation.
        let mut segments = vec![RouteUrlSegment::Separator];
        let mut start = None;

        while let Some((i, c)) = chars.next() {
            match c {
                '/' => {
                    if let Some(start) = start.take() {
                        segments.push(RouteUrlSegment::Literal(s[start..i].to_string()));
                    }

                    segments.push(RouteUrlSegment::Separator);
                }
                '{' => {
                    // A path parameter is only allowed after a slash separator.
                    if start.take().is_some() {
                        return Err(ParseError::ParameterNotAllowed { position: i });
                    }

                    if **segments.last().as_ref().unwrap() != RouteUrlSegment::Separator {
                        return Err(ParseError::ParameterNotAllowed { position: i });
                    }

                    let start = i + 1;
                    let mut stop = None;

                    for (i, c) in chars.by_ref() {
                        if c == '}' {
                            stop = Some(i);

                            break;
                        }

                        if !c.is_alphanumeric() && c != '_' {
                            return Err(ParseError::InvalidParameterCharacter {
                                start,
                                position: i,
                                character: c,
                            });
                        }
                    }

                    let stop = stop.ok_or(ParseError::UnclosedParameter {
                        start: i,
                        end: s.len() - 1,
                    })?;

                    segments.push(RouteUrlSegment::Parameter {
                        name: s[start..stop].to_string(),
                    });
                }
                c if is_valid_url_path_character(c) => {
                    if start.is_none() {
                        start = Some(i);
                    }
                }
                c => {
                    return Err(ParseError::UnexpectedCharacter {
                        position: i,
                        character: c,
                    });
                }
            }
        }

        // If we have a start, we have a literal segment left to add.
        if let Some(start) = start.take() {
            segments.push(RouteUrlSegment::Literal(s[start..].to_string()));
        }

        Ok(Self(segments))
    }
}

impl Display for RouteUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let segments = self.0.iter().with_position();

        for (position, segment) in segments {
            match (position, &segment) {
                (Position::Last, RouteUrlSegment::Separator) => {}
                (_, RouteUrlSegment::Separator) => f.write_str("/")?,
                (_, RouteUrlSegment::Literal(s)) => f.write_str(s)?,
                (_, RouteUrlSegment::Parameter { name }) => {
                    f.write_str("{")?;
                    f.write_str(name)?;
                    f.write_str("}")?;
                }
            }
        }

        Ok(())
    }
}

/// Returns whether a character is a valid URL path character.
///
/// Valid URL path characters, as per
/// [RFC3986](https://datatracker.ietf.org/doc/html/rfc3986#section-3.3) are: A–Z, a–z, 0–9, -, .,
/// _, ~, !, $, &, ', (, ), *, +, ,, ;, =, :, @, as well as % and /.
fn is_valid_url_path_character(c: char) -> bool {
    matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' | '!' | '$' | '&' | '\''
        | '(' | ')' | '*' | '+' | ',' | ';' | '=' | ':' | '@' | '%' | '/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_route_url() {
        let u: RouteUrl = "".parse().unwrap();
        assert_eq!(u.to_string(), "/");
        assert!(!u.is_prefix());
        assert_eq!(u.to_path_regex(), "^/$");

        let u: RouteUrl = "/".parse().unwrap();
        assert_eq!(u.to_string(), "/");
        assert!(u.is_prefix());
        assert_eq!(u.to_path_regex(), "^/(?P<subroute>/.*)$");

        let u: RouteUrl = "foo".parse().unwrap();
        assert_eq!(u.to_string(), "/foo");
        assert!(!u.is_prefix());
        assert_eq!(u.to_path_regex(), "^/foo$");

        let u: RouteUrl = "foo/{bar}".parse().unwrap();
        assert_eq!(u.to_string(), "/foo/{bar}");
        assert!(!u.is_prefix());
        assert_eq!(u.to_path_regex(), "^/foo/(?P<bar>[^/]+)$");

        let u: RouteUrl = "user/{uid}/comment/{cid}".parse().unwrap();
        assert_eq!(u.to_string(), "/user/{uid}/comment/{cid}");
        assert!(!u.is_prefix());
        assert_eq!(
            u.to_path_regex(),
            "^/user/(?P<uid>[^/]+)/comment/(?P<cid>[^/]+)$"
        );
    }

    #[test]
    fn test_subroute_regex_match() {
        let u: RouteUrl = "foo/{bar}/".parse().unwrap();
        assert!(u.is_prefix());
        let rx = u.to_path_regex();
        assert_eq!(rx, "^/foo/(?P<bar>[^/]+)(?P<subroute>/.*)$");
        let re = regex::Regex::new(&rx).unwrap();

        // Subroute regex should not match as it requires a `/` as the first character of the
        // subroute.
        assert!(re.captures("/foo/123").is_none());

        let caps = re.captures("/foo/123/").unwrap();
        assert_eq!(caps.name("bar").unwrap().as_str(), "123");
        assert_eq!(caps.name("subroute").unwrap().as_str(), "/");

        let caps = re.captures("/foo/123/").unwrap();
        assert_eq!(caps.name("bar").unwrap().as_str(), "123");
        assert_eq!(caps.name("subroute").unwrap().as_str(), "/");

        let caps = re.captures("/foo/123/bar").unwrap();
        assert_eq!(caps.name("bar").unwrap().as_str(), "123");
        assert_eq!(caps.name("subroute").unwrap().as_str(), "/bar");
    }

    #[test]
    fn test_parse_route_url_leading_slash() {
        let err = "/foo".parse::<RouteUrl>().unwrap_err();

        match err {
            ParseError::LeadingSlash => {}
            _ => panic!("unexpected error: {err:?}"),
        }
    }

    #[test]
    fn test_parse_route_url_unexpected_character() {
        let err = "foo</bar".parse::<RouteUrl>().unwrap_err();

        match err {
            ParseError::UnexpectedCharacter {
                position,
                character,
            } => {
                assert_eq!(position, 3);
                assert_eq!(character, '<');
            }
            _ => panic!("unexpected error: {err:?}"),
        }

        assert_eq!(err.range(), 3..=3);
    }

    #[test]
    fn test_parse_route_url_invalid_parameter_character() {
        let err = "foo/{bar<}".parse::<RouteUrl>().unwrap_err();

        match err {
            ParseError::InvalidParameterCharacter {
                start,
                position,
                character,
            } => {
                assert_eq!(start, 5);
                assert_eq!(position, 8);
                assert_eq!(character, '<');
            }
            _ => panic!("unexpected error: {err:?}"),
        }

        assert_eq!(err.range(), 5..=8);
    }

    #[test]
    fn test_parse_route_url_parameter_not_allowed() {
        let err = "foo/prefix-{bar}".parse::<RouteUrl>().unwrap_err();

        match err {
            ParseError::ParameterNotAllowed { position } => {
                assert_eq!(position, 11);
            }
            _ => panic!("unexpected error: {err:?}"),
        }

        assert_eq!(err.range(), 11..=11);
    }

    #[test]
    fn test_parse_route_url_parameter_not_allowed_twice() {
        let err = "foo/{foo}{bar}".parse::<RouteUrl>().unwrap_err();

        match err {
            ParseError::ParameterNotAllowed { position } => {
                assert_eq!(position, 9);
            }
            _ => panic!("unexpected error: {err:?}"),
        }

        assert_eq!(err.range(), 9..=9);
    }

    #[test]
    fn test_parse_route_url_unclosed_parameter() {
        let err = "foo/{bar".parse::<RouteUrl>().unwrap_err();

        match err {
            ParseError::UnclosedParameter { start, end } => {
                assert_eq!(start, 4);
                assert_eq!(end, 7);
            }
            _ => panic!("unexpected error: {err:?}"),
        }

        assert_eq!(err.range(), 4..=7);
    }
}
