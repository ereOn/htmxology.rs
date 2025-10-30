//! HTMX-SSR macros

use syn::parse_macro_input;

mod display_delegate;
mod fragment;
mod identity;
mod named;
mod route;
mod routing_controller;
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

/// Implement the `RoutingController` trait for a controller.
///
/// This derive macro automatically implements sub-controller routing for a controller type
/// by generating the necessary `Controller` and `HasSubcontroller` trait implementations.
///
/// # Attributes
///
/// - `#[controller(RouteType)]` - Specifies the route enum type for this controller
/// - `#[subcontroller(...)]` - Defines a subcontroller with the following options:
///   - `route = VariantName` - The route variant name (required)
///   - `path = "path/"` - URL path for this subcontroller (optional)
///   - `params(name: Type, ...)` - Path parameters to extract (optional)
///   - `convert_with = "function"` - Custom function to create the subcontroller (optional)
///   - `convert_response = "function"` - Custom function to convert the subcontroller's response (optional)
///   - `doc = "description"` - Documentation for the route variant (optional)
///
/// # Response Type Conversion
///
/// The macro generates a `convert_response` method in the `HasSubcontroller` implementation
/// to convert the subcontroller's `Response` type to the parent controller's `Response` type.
///
/// By default, the generated conversion uses `.into()`, assuming the parent's response type
/// implements `From<SubcontrollerResponse>`. For custom conversions, use the `convert_response`
/// attribute.
///
/// ```ignore
/// #[subcontroller(
///     MyController,
///     route = MyRoute,
///     path = "my-path/",
///     convert_response = "Ok"
/// )]
/// ```
///
/// # Example
///
/// ```ignore
/// use htmxology::{Controller, RoutingController, Route};
///
/// #[derive(RoutingController)]
/// #[controller(AppRoute)]
/// #[subcontroller(BlogController, route = Blog, path = "blog/")]
/// #[subcontroller(
///     AdminController,
///     route = Admin,
///     path = "admin/",
///     convert_response = "Ok"
/// )]
/// struct MainController {
///     // ... fields
/// }
/// ```
#[proc_macro_derive(RoutingController, attributes(controller, subcontroller))]
pub fn derive_routing_controller(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as syn::DeriveInput);

    routing_controller::derive(&mut input)
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
