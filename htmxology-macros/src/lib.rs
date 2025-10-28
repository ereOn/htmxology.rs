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
/// # Example
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
/// # Example
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
/// # Example
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
/// Supported strategies:
/// - `"innerHTML"` or `"inner_html"` - Replace inner HTML
/// - `"outerHTML"` or `"outer_html"` - Replace outer HTML
/// - `"textContent"` or `"text_content"` - Replace text content
/// - `"beforebegin"` or `"before_begin"` - Insert before element
/// - `"afterbegin"` or `"after_begin"` - Insert after opening tag
/// - `"beforeend"` or `"before_end"` - Insert before closing tag
/// - `"afterend"` or `"after_end"` - Insert after element
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
