//! Web-components support.

use std::fmt::Display;

/// A container that deals with web components registration.
#[derive(Debug, Default)]
pub struct WebComponents {
    /// The registered web components.
    pub web_components: Vec<WebComponent>,
}

impl Display for WebComponents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "<script>")?;

        for component in &self.web_components {
            writeln!(f, "{component}")?;
        }

        writeln!(f, "</script>")?;

        Ok(())
    }
}

#[derive(Debug, askama::Template)]
#[template(path = "web-component.js.jinja", escape = "none")]
pub struct WebComponent {
    /// The name of the HTML element.
    pub html_element_name: String,

    /// The name of the JS component.
    pub js_component_name: String,

    /// The shadow DOM attachment mode.
    pub shadow_dom_mode: ShadowDomMode,

    /// The HTML content of the component.
    pub html_content: String,
}

/// The shadow DOM attachment mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ShadowDomMode {
    /// Open shadow DOM.
    #[default]
    Open,

    /// Closed shadow DOM.
    Closed,
}

impl std::fmt::Display for ShadowDomMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShadowDomMode::Open => write!(f, "open"),
            ShadowDomMode::Closed => write!(f, "closed"),
        }
    }
}
