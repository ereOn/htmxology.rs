//! HTMX-SSR macros

use syn::parse_macro_input;

mod components_controller;
mod display_delegate;
mod fragment;
mod identity;
mod named;
mod route;
mod utils;

/// Derive a route type.
///
/// Route types are enum types that represent the possible routes in an HTMX application.
#[proc_macro_derive(Route, attributes(route, subroute, catch_all, query, body))]
pub fn derive_route(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as syn::DeriveInput);

    route::derive(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implement the `Display` trait for an enum.
///
/// This derive macro simply implements the `Display` trait for the annotated enum type, by
/// delegating it to the `Display` implementation of the inner variants.
#[proc_macro_derive(DisplayDelegate)]
pub fn derive_display_delegate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as syn::DeriveInput);

    display_delegate::derive(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implement the `ComponentsController` trait for a controller.
///
/// This derive macro allows to automatically implement components conversions for a controller
/// type.
#[proc_macro_derive(ComponentsController, attributes(controller, component))]
pub fn derive_components_controller(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as syn::DeriveInput);

    components_controller::derive(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive the `Identity` trait for a type.
///
/// This macro implements the `Identity` trait, which provides a unique HTML ID for an element.
/// The ID is validated at compile time to ensure it follows HTML5 rules.
///
/// # Examples
///
/// Using a static ID:
///
/// ```ignore
/// use htmxology::htmx::Identity;
///
/// #[derive(Identity)]
/// #[identity("my-element")]
/// struct MyElement {
///     content: String,
/// }
/// ```
///
/// Using a function to compute the ID dynamically:
///
/// ```ignore
/// use htmxology::htmx::{Identity, HtmlId};
///
/// #[derive(Identity)]
/// #[identity(with_fn = "get_id")]
/// struct DynamicElement {
///     index: usize,
/// }
///
/// impl DynamicElement {
///     fn get_id(&self) -> HtmlId {
///         HtmlId::from_string(format!("element-{}", self.index))
///             .expect("valid ID")
///     }
/// }
/// ```
#[proc_macro_derive(Identity, attributes(identity))]
pub fn derive_identity(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as syn::DeriveInput);

    identity::derive(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive the `Named` trait for a type.
///
/// This macro implements the `Named` trait, which provides a unique HTML name attribute
/// for a form element. The name is validated at compile time to ensure it follows HTML5 rules.
///
/// # Examples
///
/// Using a static name:
///
/// ```ignore
/// use htmxology::htmx::Named;
///
/// #[derive(Named)]
/// #[named("user-email")]
/// struct EmailField {
///     value: String,
/// }
/// ```
///
/// Using a function to compute the name dynamically:
///
/// ```ignore
/// use htmxology::htmx::{Named, HtmlName};
///
/// #[derive(Named)]
/// #[named(with_fn = "get_name")]
/// struct DynamicField {
///     field_type: String,
/// }
///
/// impl DynamicField {
///     fn get_name(&self) -> HtmlName {
///         HtmlName::from_string(format!("field-{}", self.field_type))
///             .expect("valid name")
///     }
/// }
/// ```
#[proc_macro_derive(Named, attributes(named))]
pub fn derive_named(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as syn::DeriveInput);

    named::derive(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive the `Fragment` trait for a type.
///
/// This macro implements the `Fragment` trait, which extends `Identity` and specifies
/// the HTMX swap strategy to use for out-of-band swaps.
///
/// Note: The type must also implement `Identity` (either manually or via derive).
///
/// # Examples
///
/// Using a static strategy:
///
/// ```ignore
/// use htmxology::htmx::{Identity, Fragment};
///
/// #[derive(Identity, Fragment)]
/// #[identity("notification")]
/// #[fragment(strategy = "innerHTML")]
/// struct Notification {
///     message: String,
/// }
/// ```
///
/// Using a function to compute the strategy dynamically:
///
/// ```ignore
/// use htmxology::htmx::{Identity, Fragment, InsertStrategy};
///
/// #[derive(Identity, Fragment)]
/// #[identity("dynamic-element")]
/// #[fragment(with_fn = "get_strategy")]
/// struct DynamicElement {
///     should_replace: bool,
/// }
///
/// impl DynamicElement {
///     fn get_strategy(&self) -> InsertStrategy {
///         if self.should_replace {
///             InsertStrategy::OuterHtml
///         } else {
///             InsertStrategy::InnerHtml
///         }
///     }
/// }
/// ```
///
/// # Supported strategies
///
/// The macro accepts HTMX-standard strategy strings:
/// - `"innerHTML"` - Replace inner HTML
/// - `"outerHTML"` - Replace outer HTML
/// - `"textContent"` - Replace text content
/// - `"beforebegin"` - Insert before element
/// - `"afterbegin"` - Insert after opening tag
/// - `"beforeend"` - Insert before closing tag
/// - `"afterend"` - Insert after element
/// - `"delete"` - Delete the element
/// - `"none"` - Do nothing
/// - Any other string will be treated as a custom strategy
#[proc_macro_derive(Fragment, attributes(fragment))]
pub fn derive_fragment(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as syn::DeriveInput);

    fragment::derive(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
