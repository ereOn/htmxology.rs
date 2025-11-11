//! HTMX-related types.
//!
use std::{borrow::Cow, convert::Infallible, fmt::Display, str::FromStr};

use axum::response::IntoResponse;
use http::request::Parts;
use scraper::Html;

use crate::Route;

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
    pub(super) const HX_LOCATION: http::HeaderName = http::HeaderName::from_static("hx-location");
    pub(super) const HX_PUSH_URL: http::HeaderName = http::HeaderName::from_static("hx-push-url");
    pub(super) const HX_REDIRECT: http::HeaderName = http::HeaderName::from_static("hx-redirect");
    pub(super) const HX_RETARGET: http::HeaderName = http::HeaderName::from_static("hx-retarget");
}

/// An HTMX request header extractor.
#[derive(Debug, Clone)]
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
    /// The `hx-swap-oob` attribute is injected directly into the root element of each OOB fragment.
    oob_elements: Vec<(InsertStrategy, Cow<'static, str>, Box<dyn Display + Send>)>,
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

/// Inject the `hx-swap-oob` attribute into an HTML fragment.
///
/// This function parses the HTML, finds the root element, and adds the `hx-swap-oob` attribute.
/// If there are multiple root elements or if the fragment cannot be parsed, it wraps the content
/// in a `<template>` tag with the attribute.
///
/// # Arguments
///
/// * `html` - The HTML fragment to modify
/// * `strategy` - The swap strategy to use
/// * `target` - The CSS selector target for the swap
///
/// # Returns
///
/// The modified HTML with the `hx-swap-oob` attribute injected.
fn inject_oob_attribute(html: &str, strategy: &InsertStrategy, target: &str) -> String {
    // Parse the HTML fragment
    let fragment = Html::parse_fragment(html);

    // Try to find root elements - scraper puts fragment children directly under the root
    // We need to collect all element nodes at the root level
    let root_elements: Vec<_> = fragment
        .root_element()
        .children()
        .filter_map(scraper::ElementRef::wrap)
        .collect();

    // If we have exactly one root element, inject the attribute
    if root_elements.len() == 1 {
        let root = root_elements[0];
        let tag_name = root.value().name();

        // Build the hx-swap-oob attribute value
        let oob_value = if target.starts_with('#') && strategy.to_string() == "outerHTML" {
            // Simple case: if it's an ID selector and outerHTML, we can use "true"
            // But only if the element has a matching id attribute
            if let Some(id_attr) = root.value().attr("id") {
                if format!("#{}", id_attr) == target {
                    "true".to_string()
                } else {
                    format!("{}:{}", strategy, target)
                }
            } else {
                format!("{}:{}", strategy, target)
            }
        } else {
            format!("{}:{}", strategy, target)
        };

        // Get all attributes
        let mut attrs = Vec::new();
        for (name, value) in root.value().attrs() {
            // Skip existing hx-swap-oob attributes
            if name != "hx-swap-oob" {
                attrs.push(format!("{}=\"{}\"", name, value));
            }
        }

        // Add the hx-swap-oob attribute
        attrs.push(format!("hx-swap-oob=\"{}\"", oob_value));

        // Reconstruct the element with the new attribute
        let attrs_str = attrs.join(" ");
        let inner_html = root.inner_html();

        // Handle self-closing tags
        if inner_html.is_empty() && is_void_element(tag_name) {
            format!("<{} {} />", tag_name, attrs_str)
        } else {
            format!("<{} {}>{}</{}>", tag_name, attrs_str, inner_html, tag_name)
        }
    } else {
        // Multiple root elements or no root elements - wrap in template
        let oob_value = format!("{}:{}", strategy, target);
        format!(
            "<template hx-swap-oob=\"{}\">{}</template>",
            oob_value, html
        )
    }
}

/// Check if an HTML tag is a void element (self-closing).
fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
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

    /// Add an out-of-band insert to the response using the fragment's specified swap strategy.
    ///
    /// This method uses the element's ID (from the `Identity` trait) as the target selector
    /// and the swap strategy from the `Fragment` trait's `insert_strategy()` method.
    /// The `hx-swap-oob` attribute will be injected directly into the root element of the
    /// rendered HTML fragment.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Given a type that implements Fragment
    /// let notification = Notification::new("Saved!");
    ///
    /// // The strategy comes from notification.insert_strategy()
    /// response.with_oob(notification)
    /// ```
    pub fn with_oob(self, oob_element: impl Fragment + Send + 'static) -> Self {
        let target = format!("#{}", oob_element.id());
        let strategy = oob_element.insert_strategy();
        self.with_raw_oob(strategy, target, oob_element)
    }

    /// Add an out-of-band insert to the response using the specified insert strategy.
    ///
    /// This method lets you set the HTMX target as a raw string, which can be useful if the
    /// target is not a simple HTML ID (e.g. a class selector or an attribute selector).
    ///
    /// The `hx-swap-oob` attribute will be injected directly into the root element of the
    /// rendered HTML fragment. If the fragment has multiple root elements, it will be wrapped
    /// in a `<template>` tag.
    pub fn with_raw_oob(
        mut self,
        insert_strategy: InsertStrategy,
        target: impl Into<Cow<'static, str>>,
        oob_element: impl Display + Send + 'static,
    ) -> Self {
        self.oob_elements
            .push((insert_strategy, target.into(), Box::new(oob_element)));

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

    /// Trigger a client-side redirect to a new URL that does a full page reload.
    ///
    /// This uses the `HX-Redirect` header, which causes the browser to perform a complete
    /// page reload as if the user had manually entered the URL or clicked a non-boosted link.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Response::new(html)
    ///     .with_redirect("/login")
    /// ```
    ///
    /// # Panics
    ///
    /// If another `hx-redirect` header is already present, the call will panic.
    ///
    /// The URL must be a valid HTTP header value or the call will panic.
    pub fn with_redirect(mut self, url: impl AsRef<str>) -> Self {
        let header_value = http::HeaderValue::from_str(url.as_ref()).expect("invalid redirect URL");

        assert!(
            self.extra_headers
                .insert(header::HX_REDIRECT, header_value)
                .is_none(),
            "hx-redirect header already present"
        );

        self
    }

    /// Trigger a client-side redirect without a full page reload (AJAX-based navigation).
    ///
    /// This uses the `HX-Location` header with a simple path, which behaves like following
    /// an hx-boost link - creating a new history entry and issuing an AJAX request.
    ///
    /// For more control (e.g., specifying a target element), use `with_location_details`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Response::new(html)
    ///     .with_location("/dashboard")
    /// ```
    ///
    /// # Panics
    ///
    /// If another `hx-location` header is already present, the call will panic.
    ///
    /// The URL must be a valid HTTP header value or the call will panic.
    pub fn with_location(mut self, path: impl AsRef<str>) -> Self {
        let header_value =
            http::HeaderValue::from_str(path.as_ref()).expect("invalid location path");

        assert!(
            self.extra_headers
                .insert(header::HX_LOCATION, header_value)
                .is_none(),
            "hx-location header already present"
        );

        self
    }

    /// Trigger a client-side redirect with detailed configuration using JSON.
    ///
    /// This uses the `HX-Location` header with a JSON object that allows specifying
    /// additional options like the target element for the swap.
    ///
    /// # Arguments
    ///
    /// * `path` - The URL to load
    /// * `target` - Optional CSS selector for the element to swap (e.g., "#content", ".main")
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Redirect and swap into a specific element
    /// Response::new(html)
    ///     .with_location_details("/dashboard", Some("#main-content"))
    ///
    /// // Redirect without specifying a target (equivalent to with_location)
    /// Response::new(html)
    ///     .with_location_details("/dashboard", None)
    /// ```
    ///
    /// # Panics
    ///
    /// If another `hx-location` header is already present, the call will panic.
    ///
    /// The generated JSON must be a valid HTTP header value or the call will panic.
    pub fn with_location_details(
        mut self,
        path: impl AsRef<str>,
        target: Option<impl AsRef<str>>,
    ) -> Self {
        let location_value = if let Some(target) = target {
            // Use JSON format for advanced configuration
            serde_json::json!({
                "path": path.as_ref(),
                "target": target.as_ref()
            })
            .to_string()
        } else {
            // Simple path format
            path.as_ref().to_string()
        };

        let header_value =
            http::HeaderValue::from_str(&location_value).expect("invalid location value");

        assert!(
            self.extra_headers
                .insert(header::HX_LOCATION, header_value)
                .is_none(),
            "hx-location header already present"
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
            let oob_html = oob_element.to_string();
            let injected_html = inject_oob_attribute(&oob_html, &strategy, &target);
            body.push_str(&injected_html);
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

/// A type that represents a valid HTML identifier, as per RFC 1866.
///
/// As the content of `HtmlId` is checked, it is guaranteed to be a valid HTML identifier which
/// requires no escaping when used as the value of an `id` attribute.
#[derive(Clone)]
pub struct HtmlId(Cow<'static, str>);

/// An error that occurs when trying to create an `HtmlId` from an invalid string.
#[derive(Debug, thiserror::Error)]
#[error("invalid HTML id: {0}")]
pub struct InvalidHtmlId(String);

impl Display for HtmlId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for HtmlId {
    type Err = InvalidHtmlId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned().into())
    }
}

impl TryFrom<&'static str> for HtmlId {
    type Error = InvalidHtmlId;

    fn try_from(value: &'static str) -> Result<Self, Self::Error> {
        Self::from_static(value)
    }
}

impl TryFrom<String> for HtmlId {
    type Error = InvalidHtmlId;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value.into())
    }
}

impl TryFrom<Cow<'static, str>> for HtmlId {
    type Error = InvalidHtmlId;

    fn try_from(value: Cow<'static, str>) -> Result<Self, Self::Error> {
        match value {
            Cow::Borrowed(s) => Self::from_static(s),
            Cow::Owned(s) => Self::new(s.into()),
        }
    }
}

impl HtmlId {
    /// Create a new `HtmlId` from the given static string.
    pub fn from_static(id: &'static str) -> Result<Self, InvalidHtmlId> {
        Self::check_valid_html_id(id).map(|_| Self(Cow::Borrowed(id)))
    }

    /// Create a new `HtmlId` from the given string.
    ///
    /// # Errors
    ///
    /// Returns an `InvalidHtmlId` error if the string is not a valid HTML identifier.
    fn new(id: Cow<'static, str>) -> Result<Self, InvalidHtmlId> {
        Self::check_valid_html_id(&id).map(|_| Self(id))
    }

    /// Check if the given string is a valid HTML identifier.
    ///
    /// # Rules
    ///
    /// - Cannot be empty.
    /// - The first character must be a letter (A-Z or a-z) or an underscore (_).
    /// - The remaining characters can be letters, digits (0-9), hyphens (-), underscores (_),
    ///   colons (:), or periods (.).
    /// - Cannot contain spaces.
    /// - Cannot contain special characters other than hyphens, underscores, colons, or periods.
    fn check_valid_html_id(id: &str) -> Result<(), InvalidHtmlId> {
        let mut chars = id.chars();

        let first_char = match chars.next() {
            Some(c) => c,
            None => return Err(InvalidHtmlId("empty string".to_owned())),
        };

        if !first_char.is_ascii_alphanumeric() && first_char != '_' {
            return Err(InvalidHtmlId(format!(
                "invalid first character: '{first_char}' in '{id}'",
            )));
        }

        for c in chars {
            if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.') {
                return Err(InvalidHtmlId(
                    format!("invalid character: '{c}' in '{id}'",),
                ));
            }
        }

        Ok(())
    }
}

/// A type that represents a valid HTML attribute name, as per RFC 1866.
///
/// As the content of `HtmlName` is checked, it is guaranteed to be a valid HTML name which
/// requires no escaping when used as the value of a `name` attribute.
#[derive(Clone)]
pub struct HtmlName(Cow<'static, str>);

/// An error that occurs when trying to create an `HtmlName` from an invalid string.
#[derive(Debug, thiserror::Error)]
#[error("invalid HTML name: {0}")]
pub struct InvalidHtmlName(String);

impl Display for HtmlName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for HtmlName {
    type Err = InvalidHtmlName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned().into())
    }
}

impl TryFrom<&'static str> for HtmlName {
    type Error = InvalidHtmlName;

    fn try_from(value: &'static str) -> Result<Self, Self::Error> {
        Self::from_static(value)
    }
}

impl TryFrom<String> for HtmlName {
    type Error = InvalidHtmlName;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value.into())
    }
}

impl TryFrom<Cow<'static, str>> for HtmlName {
    type Error = InvalidHtmlName;

    fn try_from(value: Cow<'static, str>) -> Result<Self, Self::Error> {
        match value {
            Cow::Borrowed(s) => Self::from_static(s),
            Cow::Owned(s) => Self::new(s.into()),
        }
    }
}

impl HtmlName {
    /// Create a new `HtmlName` from the given static string.
    pub fn from_static(id: &'static str) -> Result<Self, InvalidHtmlName> {
        Self::check_valid_html_name(id).map(|_| Self(Cow::Borrowed(id)))
    }

    /// Create a new `HtmlName` from the given string.
    ///
    /// # Errors
    ///
    /// Returns an `InvalidHtmlName` error if the string is not a valid HTML identifier.
    fn new(id: Cow<'static, str>) -> Result<Self, InvalidHtmlName> {
        Self::check_valid_html_name(&id).map(|_| Self(id))
    }

    /// Check if the given string is a valid HTML name, as per RFC 1866
    ///
    /// # Rules
    ///
    /// - Cannot be empty.
    /// - The first character must be a letter (A-Z or a-z) or an underscore (_).
    /// - The remaining characters can be letters, digits (0-9), hyphens (-), underscores (_),
    ///   colons (:), or periods (.).
    /// - Cannot contain spaces.
    /// - Cannot contain special characters other than hyphens, underscores, colons, or periods.
    fn check_valid_html_name(id: &str) -> Result<(), InvalidHtmlName> {
        let mut chars = id.chars();

        let first_char = match chars.next() {
            Some(c) => c,
            None => return Err(InvalidHtmlName("empty string".to_owned())),
        };

        if !first_char.is_ascii_alphanumeric() && first_char != '_' {
            return Err(InvalidHtmlName(format!(
                "invalid first character: '{first_char}' in '{id}'",
            )));
        }

        for c in chars {
            if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.') {
                return Err(InvalidHtmlName(format!(
                    "invalid character: '{c}' in '{id}'",
                )));
            }
        }

        Ok(())
    }
}

/// A trait for HTML elements that have an identity.
///
/// Types that implement this trait MUST render as a HTML fragment with a root element that has
/// an `id` attribute matching the value returned by the `id` method.
///
/// A common way in most template engines is to use the `id_attribute` method to get the proper
/// declaration for the `id` attribute.
///
/// Here's an example implementation using the `askama` template engine:
///
/// ```ignore
/// <div {{ id_attribute()|safe }}>
/// ```
pub trait Identity: Display {
    /// Get the unique identifier for the HTML element.
    fn id(&self) -> HtmlId;

    /// Get the `id` attribute declaration for the HTML element.
    ///
    /// This is a convenience method that formats the `id` attribute for use in HTML.
    ///
    /// In most cases, this method should not be overridden.
    fn id_attribute(&self) -> String {
        format!(r#"id="{}""#, self.id())
    }
}

/// A trait for HTML fragments that can be used in out-of-band swaps.
///
/// This trait extends [`Identity`] and requires implementors to specify the HTMX swap strategy
/// that should be used when this fragment is added as an out-of-band element.
///
/// Types that implement this trait MUST render as a valid HTML fragment and include an `id`
/// attribute matching the value from [`Identity::id()`].
///
/// # Example
///
/// ```ignore
/// use htmxology::htmx::{Fragment, Identity, InsertStrategy, HtmlId};
///
/// struct Notification {
///     message: String,
/// }
///
/// impl Identity for Notification {
///     fn id(&self) -> HtmlId {
///         HtmlId::from_static("notification").unwrap()
///     }
/// }
///
/// impl Fragment for Notification {
///     fn insert_strategy(&self) -> InsertStrategy {
///         // This notification should replace the inner content
///         InsertStrategy::InnerHtml
///     }
/// }
///
/// impl Display for Notification {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "<div id=\"notification\">{}</div>", self.message)
///     }
/// }
/// ```
pub trait Fragment: Identity {
    /// Get the HTMX insert strategy for this fragment.
    ///
    /// This determines how the fragment will be swapped into the page when used as an
    /// out-of-band element.
    fn insert_strategy(&self) -> InsertStrategy;
}

/// A trait for HTML elements that have a form attribute name.
///
/// Types that implement this trait MUST render as a HTML fragment that contains an unique form
/// element with a `name` attribute matching the value returned by the `name` method.
///
/// A common way in most template engines is to use the `name_attribute` method to get the proper
/// declaration for the `name` attribute.
pub trait Named: Display {
    /// Get the name of the form element.
    fn name(&self) -> HtmlName;

    /// Get the `name` attribute declaration for the form element.
    ///
    /// This is a convenience method that formats the `name` attribute for use in HTML.
    ///
    /// In most cases, this method should not be overridden.
    fn name_attribute(&self) -> String {
        format!(r#"name="{}""#, self.name())
    }
}

/// A trait for HTML elements that contain a form.
pub trait HtmlForm: Display {
    /// The associated type for the route that handles the form submission.
    type Route: Route;

    /// Get the route that handles the form submission.
    fn action_route(&self) -> Self::Route;

    /// Get the `action` attribute declaration for the form element.
    fn action_attribute(&self) -> String {
        self.action_route().as_htmx_attribute()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_html_ids() {
        let valid_ids = [
            "validId", "valid-id", "valid_id", "valid:id", "valid.id", "valid123", "v", "_valid",
            "1valid",
        ];

        for id in valid_ids {
            assert!(
                HtmlId::from_str(id).is_ok(),
                "Expected '{}' to be a valid HTML id",
                id
            );
        }
    }

    #[test]
    fn test_invalid_html_ids() {
        let invalid_ids = [
            "",
            "-invalid",
            ".invalid",
            ":invalid",
            "invalid id",
            "invalid$id",
            "invalid/id",
            " invalid",
            "invalid ",
            "inva!lid",
        ];
        for id in invalid_ids {
            assert!(
                HtmlId::from_str(id).is_err(),
                "Expected '{}' to be an invalid HTML id",
                id
            );
        }
    }

    #[test]
    fn test_valid_html_names() {
        let valid_names = [
            "validName",
            "valid-name",
            "valid_name",
            "valid:name",
            "valid.name",
            "valid123",
            "v",
            "_valid",
            "1valid",
        ];

        for name in valid_names {
            assert!(
                HtmlName::from_str(name).is_ok(),
                "Expected '{}' to be a valid HTML name",
                name
            );
        }
    }

    #[test]
    fn test_invalid_html_names() {
        let invalid_names = [
            "",
            "-invalid",
            ".invalid",
            ":invalid",
            "invalid name",
            "invalid$name",
            "invalid/name",
            " invalid",
            "invalid ",
            "inva!lid",
        ];
        for name in invalid_names {
            assert!(
                HtmlName::from_str(name).is_err(),
                "Expected '{}' to be an invalid HTML name",
                name
            );
        }
    }

    #[test]
    fn test_inject_oob_attribute_single_element() {
        let html = r#"<div id="test">Content</div>"#;
        let result = inject_oob_attribute(html, &InsertStrategy::OuterHtml, "#test");

        // Should inject hx-swap-oob="true" since ID matches and strategy is outerHTML
        assert!(
            result.contains(r#"hx-swap-oob="true""#),
            "Expected hx-swap-oob=\"true\", got: {}",
            result
        );
        assert!(result.contains("Content"), "Content should be preserved");
    }

    #[test]
    fn test_inject_oob_attribute_with_different_target() {
        let html = r#"<div id="source">Content</div>"#;
        let result = inject_oob_attribute(html, &InsertStrategy::OuterHtml, "#target");

        // Should inject hx-swap-oob="outerHTML:#target" since IDs don't match
        assert!(
            result.contains(r#"hx-swap-oob="outerHTML:#target""#),
            "Expected hx-swap-oob=\"outerHTML:#target\", got: {}",
            result
        );
    }

    #[test]
    fn test_inject_oob_attribute_with_strategy() {
        let html = r#"<div id="test">Content</div>"#;
        let result = inject_oob_attribute(html, &InsertStrategy::InnerHtml, "#test");

        // Should inject hx-swap-oob="innerHTML:#test" since strategy is not outerHTML
        assert!(
            result.contains(r#"hx-swap-oob="innerHTML:#test""#),
            "Expected hx-swap-oob=\"innerHTML:#test\", got: {}",
            result
        );
    }

    #[test]
    fn test_inject_oob_attribute_multiple_elements() {
        let html = r#"<div>First</div><div>Second</div>"#;
        let result = inject_oob_attribute(html, &InsertStrategy::OuterHtml, "#target");

        // Should wrap in template tag
        assert!(
            result.starts_with("<template"),
            "Expected to wrap multiple elements in template, got: {}",
            result
        );
        assert!(
            result.contains(r#"hx-swap-oob="outerHTML:#target""#),
            "Template should have hx-swap-oob attribute"
        );
        assert!(
            result.contains("First") && result.contains("Second"),
            "Content should be preserved"
        );
    }

    #[test]
    fn test_inject_oob_attribute_preserves_attributes() {
        let html = r#"<div id="test" class="foo" data-value="bar">Content</div>"#;
        let result = inject_oob_attribute(html, &InsertStrategy::OuterHtml, "#test");

        // Should preserve existing attributes
        assert!(
            result.contains(r#"id="test""#),
            "id attribute should be preserved"
        );
        assert!(
            result.contains(r#"class="foo""#),
            "class attribute should be preserved"
        );
        assert!(
            result.contains(r#"data-value="bar""#),
            "data attribute should be preserved"
        );
        assert!(
            result.contains(r#"hx-swap-oob="true""#),
            "hx-swap-oob attribute should be added"
        );
    }

    #[test]
    fn test_inject_oob_attribute_class_selector() {
        let html = r#"<div class="notification">Alert</div>"#;
        let result = inject_oob_attribute(html, &InsertStrategy::BeforeEnd, ".notification");

        assert_eq!(
            result,
            r#"<div class="notification" hx-swap-oob="beforeend:.notification">Alert</div>"#
        );
    }

    // Tests for Fragment trait
    struct TestFragment {
        id: &'static str,
        strategy: InsertStrategy,
        content: &'static str,
    }

    impl Display for TestFragment {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, r#"<div id="{}">{}</div>"#, self.id, self.content)
        }
    }

    impl Identity for TestFragment {
        fn id(&self) -> HtmlId {
            HtmlId::from_static(self.id).expect("valid ID")
        }
    }

    impl Fragment for TestFragment {
        fn insert_strategy(&self) -> InsertStrategy {
            self.strategy.clone()
        }
    }

    #[test]
    fn test_fragment_with_inner_html() {
        let fragment = TestFragment {
            id: "notification",
            strategy: InsertStrategy::InnerHtml,
            content: "Alert!",
        };

        // Verify the fragment returns the correct strategy
        assert_eq!(fragment.insert_strategy().to_string(), "innerHTML");
        assert_eq!(fragment.id().to_string(), "notification");
    }

    #[test]
    fn test_fragment_with_before_end() {
        let fragment = TestFragment {
            id: "list",
            strategy: InsertStrategy::BeforeEnd,
            content: "<li>New item</li>",
        };

        // Verify the fragment returns the correct strategy
        assert_eq!(fragment.insert_strategy().to_string(), "beforeend");
    }

    #[test]
    fn test_response_with_fragment() {
        let fragment = TestFragment {
            id: "test-id",
            strategy: InsertStrategy::OuterHtml,
            content: "Test content",
        };

        let response = Response::new("Main content").with_oob(fragment);

        // Verify that the fragment is stored with the correct strategy
        assert_eq!(response.oob_elements.len(), 1);
        let (strategy, target, _) = &response.oob_elements[0];
        assert_eq!(strategy.to_string(), "outerHTML");
        assert_eq!(target.as_ref(), "#test-id");
    }

    #[test]
    fn test_multiple_fragments_with_different_strategies() {
        let fragment1 = TestFragment {
            id: "alert",
            strategy: InsertStrategy::OuterHtml,
            content: "Alert 1",
        };

        let fragment2 = TestFragment {
            id: "notification",
            strategy: InsertStrategy::InnerHtml,
            content: "Notification",
        };

        let response = Response::new("Main")
            .with_oob(fragment1)
            .with_oob(fragment2);

        // Verify that both fragments are stored
        assert_eq!(response.oob_elements.len(), 2);

        // Verify strategies
        assert_eq!(response.oob_elements[0].0.to_string(), "outerHTML");
        assert_eq!(response.oob_elements[1].0.to_string(), "innerHTML");

        // Verify targets
        assert_eq!(response.oob_elements[0].1.as_ref(), "#alert");
        assert_eq!(response.oob_elements[1].1.as_ref(), "#notification");
    }

    #[test]
    fn test_response_is_send() {
        // Compile-time assertion that Response<T> is Send when T is Send
        fn assert_send<T: Send>() {}
        assert_send::<Response<String>>();
    }

    #[test]
    fn test_with_redirect() {
        use axum::response::IntoResponse;

        let response = Response::new("test body").with_redirect("/login");
        let axum_response = response.into_response();

        // Check that the HX-Redirect header is set
        let redirect_header = axum_response
            .headers()
            .get("hx-redirect")
            .expect("hx-redirect header should be present");
        assert_eq!(redirect_header, "/login");
    }

    #[test]
    fn test_with_location_simple() {
        use axum::response::IntoResponse;

        let response = Response::new("test body").with_location("/dashboard");
        let axum_response = response.into_response();

        // Check that the HX-Location header is set
        let location_header = axum_response
            .headers()
            .get("hx-location")
            .expect("hx-location header should be present");
        assert_eq!(location_header, "/dashboard");
    }

    #[test]
    fn test_with_location_details_without_target() {
        use axum::response::IntoResponse;

        let response = Response::new("test body").with_location_details("/dashboard", None::<&str>);
        let axum_response = response.into_response();

        // Check that the HX-Location header is set to simple path
        let location_header = axum_response
            .headers()
            .get("hx-location")
            .expect("hx-location header should be present");
        assert_eq!(location_header, "/dashboard");
    }

    #[test]
    fn test_with_location_details_with_target() {
        use axum::response::IntoResponse;

        let response =
            Response::new("test body").with_location_details("/dashboard", Some("#main-content"));
        let axum_response = response.into_response();

        // Check that the HX-Location header is set to JSON format
        let location_header = axum_response
            .headers()
            .get("hx-location")
            .expect("hx-location header should be present");
        let location_str = location_header.to_str().unwrap();

        // Verify it's JSON with both path and target
        assert!(location_str.contains("\"path\":\"/dashboard\""));
        assert!(location_str.contains("\"target\":\"#main-content\""));
    }

    #[test]
    #[should_panic(expected = "hx-redirect header already present")]
    fn test_with_redirect_duplicate_panics() {
        Response::new("test body")
            .with_redirect("/login")
            .with_redirect("/logout");
    }

    #[test]
    #[should_panic(expected = "hx-location header already present")]
    fn test_with_location_duplicate_panics() {
        Response::new("test body")
            .with_location("/dashboard")
            .with_location("/settings");
    }

    // Test that htmx::Response can be used as a Controller response type
    #[cfg(test)]
    mod controller_response_tests {
        use super::*;
        use crate::{Controller, Route, ServerInfo};

        // Define a simple route for testing
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum TestRoute {
            Home,
        }

        impl std::str::FromStr for TestRoute {
            type Err = crate::route::ParseError;

            fn from_str(_s: &str) -> Result<Self, Self::Err> {
                Ok(TestRoute::Home)
            }
        }

        impl Route for TestRoute {
            fn method(&self) -> http::Method {
                http::Method::GET
            }
        }

        impl std::fmt::Display for TestRoute {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "/")
            }
        }

        // Implement FromRequest for TestRoute
        impl axum::extract::FromRequest<TestController> for TestRoute {
            type Rejection = (http::StatusCode, String);

            async fn from_request(
                _req: http::Request<axum::body::Body>,
                _state: &TestController,
            ) -> Result<Self, Self::Rejection> {
                Ok(Self::Home)
            }
        }

        // Define a test controller using htmx::Response as the response type
        #[derive(Clone)]
        struct TestController;

        impl Controller for TestController {
            type Route = TestRoute;
            type Args = ();
            type Response = Result<Response<String>, axum::response::Response>;

            async fn handle_request(
                &self,
                _route: Self::Route,
                _htmx: Request,
                _parts: http::request::Parts,
                _server_info: &ServerInfo,
                _args: Self::Args,
            ) -> Self::Response {
                Ok(Response::new("Hello, World!".to_string()))
            }
        }

        #[tokio::test]
        async fn test_controller_with_htmx_response() {
            let controller = TestController;
            let route = TestRoute::Home;
            let htmx = Request::Classic;

            // Create parts from a real request
            let req = http::Request::builder()
                .method("GET")
                .uri("/")
                .body(())
                .unwrap();
            let (parts, _body) = req.into_parts();

            let server_info = ServerInfo {
                base_url: "http://localhost:3000".parse().unwrap(),
            };

            let args = ();
            let response = controller
                .handle_request(route, htmx, parts, &server_info, args)
                .await;

            assert!(response.is_ok());
            let htmx_response = response.unwrap();

            // Verify the response can be converted to axum::response::Response
            use axum::response::IntoResponse;
            let axum_response = htmx_response.into_response();
            assert_eq!(axum_response.status(), http::StatusCode::OK);
        }

        #[test]
        fn test_controller_response_type_is_send() {
            // Compile-time assertion that the Controller::Response type is Send
            fn assert_send<T: Send>() {}
            assert_send::<<TestController as Controller>::Response>();
        }
    }
}
