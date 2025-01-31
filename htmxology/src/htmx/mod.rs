//! HTMX-related types.
//!
use std::{convert::Infallible, fmt::Display};

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
        target: Option<String>,

        /// The trigger name, if one was provided.
        trigger_name: Option<String>,

        /// The trigger, if one was provided.
        trigger: Option<String>,
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
                target: headers
                    .get(header::HX_TARGET)
                    .and_then(|value| value.to_str().ok())
                    .map(|value| value.to_owned()),
                trigger_name: headers
                    .get(header::HX_TRIGGER_NAME)
                    .and_then(|value| value.to_str().ok())
                    .map(|value| value.to_owned()),
                trigger: headers
                    .get(header::HX_TRIGGER)
                    .and_then(|value| value.to_str().ok())
                    .map(|value| value.to_owned()),
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

    /// Some extra HTTP headers that are added to the response.
    extra_headers: http::HeaderMap,

    /// The out-of-band inserts that are added to the body of the response, but will be processed
    /// out-of-band by HTMX.
    ///
    /// The swap method strategy used for these are `innerHTML`, as the elements are automatically wrapped in a
    /// `div` element.
    oob_elements: Vec<(InsertStrategy, Box<dyn Fragment>)>,
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
    /// Add an out-of-band insert to the response using the `innerHTML` swap method.
    pub fn with_oob(mut self, oob_element: impl Fragment + 'static) -> Self {
        self.oob_elements
            .push((InsertStrategy::InnerHtml, Box::new(oob_element)));
        self
    }

    /// Add an extra HTTP header to the response.
    pub fn with_header(mut self, name: http::HeaderName, value: http::HeaderValue) -> Self {
        self.extra_headers.append(name, value);
        self
    }
}

impl<T: Fragment> axum::response::IntoResponse for Response<T> {
    fn into_response(self) -> axum::response::Response {
        let headers: http::HeaderMap = [
            (
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("text/html"),
            ),
            (
                header::HX_RETARGET,
                http::HeaderValue::from_static(self.body.htmx_target()),
            ),
        ]
        .into_iter()
        .chain(
            self.extra_headers
                .into_iter()
                .filter_map(|(name, value)| name.map(|name| (name, value))),
        )
        .collect();

        let mut body = self.body.to_string();

        for (strategy, oob_element) in self.oob_elements {
            let target = oob_element.htmx_target();

            body.push_str(&format!(
                "<div hx-swap-oob=\"{strategy}:{target}\">{oob_element}</div>"
            ));
        }

        (headers, body).into_response()
    }
}

/// A trait for types that can be inserted into an HTMX response.
///
/// Fragments are associated to a static HTMX target, which should ideally be the element whose
/// content will be replaced by the fragment.
pub trait Fragment: Display {
    /// Get the HTMX target for the insert, as used in a `hx-target` attribute or the `Hx-Retarget`
    /// HTTP header.
    ///
    /// This is the target element that will be replaced by the insert.
    fn htmx_target(&self) -> &'static str;
}

impl<T: Fragment> From<T> for Response<T> {
    fn from(body: T) -> Self {
        Self {
            body,
            extra_headers: http::HeaderMap::new(),
            oob_elements: vec![],
        }
    }
}

/// An extension trait for fragments.
pub trait FragmentExt: Fragment + Sized {
    /// Turn the insert into an HTMX `Response`.
    ///
    /// This is a convenience method.
    fn into_htmx_response(self) -> Response<Self> {
        self.into()
    }
}

impl<T> FragmentExt for T where T: Fragment {}
