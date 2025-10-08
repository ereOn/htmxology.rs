//! HTMX-related types.
//!
use std::{borrow::Cow, convert::Infallible, fmt::Display};

use axum::response::IntoResponse;
use http::request::Parts;

mod header {
    /// Request headers.
    pub(super) const HX_BOOSTED: http::HeaderName = http::HeaderName::from_static("hx-boosted");
    pub(super) const HX_CURRENT_URL: http::HeaderName =
        http::HeaderName::from_static("hx-current-url");
    pub(super) const HX_HISTORY_RESTORE_REQUEST: http::HeaderName =
        http::HeaderName::from_static("hx-history-restore-request");
    pub(super) const HX_PROMPT: http::HeaderName = http::HeaderName::from_static("hx-prompt");
    pub(super) const HX_REQUEST: http::HeaderName = http::HeaderName::from_static("hx-request");
    pub(super) const HX_TARGET: http::HeaderName = http::HeaderName::from_static("hx-target");
    pub(super) const HX_TRIGGER_NAME: http::HeaderName =
        http::HeaderName::from_static("hx-trigger-name");
    pub(super) const HX_TRIGGER: http::HeaderName = http::HeaderName::from_static("hx-trigger");

    // Response headers.
    pub(super) const HX_RETARGET: http::HeaderName = http::HeaderName::from_static("hx-retarget");
    pub(super) const HX_PUSH_URL: http::HeaderName = http::HeaderName::from_static("hx-push-url");
}

/// An HTMX request header extractor.
#[derive(Debug)]
pub enum Request {
    /// A classic request, with no HTMX headers.
    Classic,

    /// An HTMX request, with the HTMX headers.
    Htmx {
        /// Whether the request was boosted.
        boosted: bool,

        /// The current URL.
        current_url: String,

        /// The history restore request flag.
        history_restore_request: bool,

        /// The prompt.
        prompt: String,

        /// The target of the request, if one was provided.
        target: Option<http::HeaderValue>,

        /// The trigger name, if one was provided.
        trigger_name: Option<http::HeaderValue>,

        /// The trigger, if one was provided.
        trigger: Option<http::HeaderValue>,
    },
}

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for Request {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        if headers.get(header::HX_REQUEST).is_some() {
            Ok(Self::Htmx {
                boosted: headers.get(header::HX_BOOSTED).is_some(),
                current_url: headers
                    .get(header::HX_CURRENT_URL)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_owned(),
                history_restore_request: headers.get(header::HX_HISTORY_RESTORE_REQUEST).is_some(),
                prompt: headers
                    .get(header::HX_PROMPT)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_owned(),
                target: headers.get(header::HX_TARGET).cloned(),
                trigger_name: headers.get(header::HX_TRIGGER_NAME).cloned(),
                trigger: headers.get(header::HX_TRIGGER).cloned(),
            })
        } else {
            Ok(Self::Classic)
        }
    }
}

/// An HTMX response, as returned by an Axum handler.
///
/// A `Response` typically consists of a main insert and an optional list of out-of-band inserts.
///
/// Responses contain the `Hx-Retarget` header, which specifies the target for the main insert.
pub struct Response<T> {
    /// The main insert that constitutes the body of the response.
    body: T,

    /// The content-type of the response.
    content_type: http::HeaderValue,

    /// The HTMX target for the main insert.
    htmx_retarget: Option<http::HeaderValue>,

    /// Some extra HTTP headers that are added to the response.
    extra_headers: http::HeaderMap,

    /// The out-of-band inserts that are added to the body of the response, but will be processed
    /// out-of-band by HTMX.
    ///
    /// The swap method strategy used for these are `innerHTML`, as the elements are automatically wrapped in a
    /// `div` element.
    oob_elements: Vec<(InsertStrategy, Cow<'static, str>, Box<dyn Display>)>,
}

/// An HTMX insert strategy.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InsertStrategy {
    /// Replace the target element's inner HTML.
    InnerHtml,
    /// Replace the target element's outer HTML.
    OuterHtml,
    /// Replace the target element's text content without interpreting it as HTML.
    TextContent,
    /// Insert the element before the target element.
    BeforeBegin,
    /// Insert the element after the target element.
    AfterBegin,
    /// Insert the element before the target element's end tag.
    BeforeEnd,
    /// Insert the element after the target element's end tag.
    AfterEnd,
    /// Delete the target element.
    Delete,
    /// Do not insert the element.
    None,
    /// Custom insert strategy.
    ///
    /// This is a catch-all variant for custom insert strategies that are not (yet) covered by the
    /// other variants.
    Custom(String),
}

impl Display for InsertStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InnerHtml => write!(f, "innerHTML"),
            Self::OuterHtml => write!(f, "outerHTML"),
            Self::TextContent => write!(f, "textContent"),
            Self::BeforeBegin => write!(f, "beforebegin"),
            Self::AfterBegin => write!(f, "afterbegin"),
            Self::BeforeEnd => write!(f, "beforeend"),
            Self::AfterEnd => write!(f, "afterend"),
            Self::Delete => write!(f, "delete"),
            Self::None => write!(f, "none"),
            Self::Custom(custom) => write!(f, "{custom}"),
        }
    }
}

impl<T> Response<T> {
    /// Create a new HTMX response with the given body.
    pub fn new(body: T) -> Self {
        Self {
            body,
            content_type: http::HeaderValue::from_static("text/html"),
            htmx_retarget: None,
            extra_headers: http::HeaderMap::new(),
            oob_elements: vec![],
        }
    }

    /// Set the content type of the response.
    pub fn with_content_type(mut self, content_type: http::HeaderValue) -> Self {
        self.content_type = content_type;
        self
    }

    /// Forward the optional target from the request to the response.
    pub fn with_forwarded_target(mut self, target: Option<http::HeaderValue>) -> Self {
        self.htmx_retarget = target;
        self
    }

    /// Retarget the response to the given target.
    ///
    /// # Panics
    ///
    /// If the target is not a valid HTTP header value, the call will panic.
    pub fn with_retarget(mut self, htmx_retarget: http::HeaderValue) -> Self {
        self.htmx_retarget = Some(htmx_retarget);
        self
    }

    /// Add an out-of-band insert to the response using the `innerHTML` swap method.
    pub fn with_oob(
        mut self,
        target: impl Into<Cow<'static, str>>,
        oob_element: impl Display + 'static,
    ) -> Self {
        self.oob_elements.push((
            InsertStrategy::InnerHtml,
            target.into(),
            Box::new(oob_element),
        ));

        self
    }

    /// Add an extra HTTP header to the response.
    pub fn with_header(mut self, name: http::HeaderName, value: http::HeaderValue) -> Self {
        self.extra_headers.append(name, value);
        self
    }

    /// Indicate that the response should not push the URL to the browser history.
    ///
    /// # Panics
    ///
    /// If another `hx-push-url` header is already present, the call will panic.
    pub fn without_push_url(mut self) -> Self {
        assert!(
            self.extra_headers
                .insert(header::HX_PUSH_URL, http::HeaderValue::from_static("false"))
                .is_none(),
            "hx-push-url header already present"
        );

        self
    }

    /// Indicate that the response should push the URL to the browser history.
    ///
    /// # Panics
    ///
    /// If another `hx-push-url` header is already present, the call will panic.
    ///
    /// The URL to push must be a valid HTTP header value or the call will panic.
    pub fn with_push_url(mut self, url: &http::Uri) -> Self {
        let header_value = http::HeaderValue::from_str(&url.to_string()).expect("invalid URL");

        assert!(
            self.extra_headers
                .insert(header::HX_PUSH_URL, header_value,)
                .is_none(),
            "hx-push-url header already present"
        );

        self
    }
}

impl<T: Display> axum::response::IntoResponse for Response<T> {
    fn into_response(self) -> axum::response::Response {
        let headers: http::HeaderMap = [(http::header::CONTENT_TYPE, self.content_type)]
            .into_iter()
            .chain(
                self.htmx_retarget
                    .map(|htmx_target| (header::HX_RETARGET, htmx_target)),
            )
            .chain(
                self.extra_headers
                    .into_iter()
                    .filter_map(|(name, value)| name.map(|name| (name, value))),
            )
            .collect();

        let mut body = self.body.to_string();

        for (strategy, target, oob_element) in self.oob_elements {
            body.push_str(&format!(
                "<div hx-swap-oob=\"{strategy}:{target}\">{oob_element}</div>"
            ));
        }

        (headers, body).into_response()
    }
}

/// An extension trait for responses.
pub trait ResponseExt: Sized {
    /// Turn the insert into an HTMX `Response`.
    ///
    /// Note: `htmx_retarget` can be `None` to indicate that the response should not be retargeted.
    ///
    /// This is a convenience method.
    fn into_htmx_response(self) -> Response<Self> {
        Response::new(self)
    }
}

impl<T> ResponseExt for T where T: Sized {}

/// An extension trait for providing convenience methods on `Result<T, E>`.
pub trait ResultExt<T> {
    /// Turn the result into an HTTP response with the specified status code.
    ///
    /// If the status code is in the 5xx range, the error message will be logged at the error
    /// level and a generic "internal server error" message will be returned to the client.
    #[expect(clippy::result_large_err)]
    fn map_error_into_response(
        self,
        status_code: http::StatusCode,
    ) -> Result<T, axum::response::Response>;
}

impl<T, E: Display> ResultExt<T> for Result<T, E> {
    fn map_error_into_response(
        self,
        status_code: http::StatusCode,
    ) -> Result<T, axum::response::Response> {
        self.map_err(|err| {
            if status_code.is_server_error() {
                tracing::error!("Internal server error: {err}");
                (status_code, "internal server error").into_response()
            } else {
                (status_code, err.to_string()).into_response()
            }
        })
    }
}

/// An extension trait for providing convenience methods on `Option<T>`.
pub trait OptionExt<T>: Sized {
    /// Turn the option into an HTMX `Response`, mapping the `None` case to a response with the
    /// specified status and message as the body.
    #[expect(clippy::result_large_err)]
    fn ok_or_status(
        self,
        status_code: http::StatusCode,
        message: impl Into<Cow<'static, str>>,
    ) -> Result<T, axum::response::Response>;

    /// Turn the option into an HTMX `Response`, mapping the `None` case to a response with a 404 status
    /// code and the specified message as the body.
    #[expect(clippy::result_large_err)]
    fn ok_or_not_found(
        self,
        message: impl Into<Cow<'static, str>>,
    ) -> Result<T, axum::response::Response> {
        self.ok_or_status(http::StatusCode::NOT_FOUND, message)
    }

    /// Turn the option into an HTMX `Response`, mapping the `None` case to a response with a 400
    /// status code and the specified message as the body.
    #[expect(clippy::result_large_err)]
    fn ok_or_bad_request(
        self,
        message: impl Into<Cow<'static, str>>,
    ) -> Result<T, axum::response::Response> {
        self.ok_or_status(http::StatusCode::BAD_REQUEST, message)
    }

    /// Turn the option into an HTMX `Response`, mapping the `None` case to a response with a 500
    /// status code and "internal server error" as the body.
    #[expect(clippy::result_large_err)]
    fn ok_or_internal_server_error(self) -> Result<T, axum::response::Response> {
        self.ok_or_status(
            http::StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error",
        )
    }
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_status(
        self,
        status_code: http::StatusCode,
        message: impl Into<Cow<'static, str>>,
    ) -> Result<T, axum::response::Response> {
        self.ok_or_else(|| (status_code, message.into()).into_response())
    }
}
