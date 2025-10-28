# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **BREAKING**: Renamed `ComponentsController` derive macro to `RoutingController` for better clarity
- **BREAKING**: Renamed `#[component(...)]` attribute to `#[subcontroller(...)]` for consistency with the controller/subcontroller terminology
- **BREAKING**: Renamed `ControllerExt` trait to `SubcontrollerExt` for better alignment with subcontroller terminology
- **BREAKING**: Renamed `AsComponent` trait to `AsSubcontroller` for consistency
- **BREAKING**: Renamed `get_component()` method to `get_subcontroller()`
- **BREAKING**: Renamed `get_component_with()` method to `get_subcontroller_with()`
- **BREAKING**: Renamed `as_component_controller()` method to `as_subcontroller()`

## [0.14.0] - 2025-10-27

### Added
- **`Fragment` trait**: New trait for HTML fragments that can specify their own HTMX swap strategy
  - Extends `Identity` trait to maintain ID-based targeting
  - Requires implementors to provide an `InsertStrategy` via `insert_strategy()` method
  - Allows different fragments to use different swap strategies (innerHTML, outerHTML, beforeend, etc.)
  - See `htmxology::htmx::Fragment` documentation for usage examples
- **Derive macros for Identity, Named, and Fragment traits** (#6)
  - `#[derive(Identity)]` with `#[identity("html-id")]` attribute
  - `#[derive(Named)]` with `#[named("field-name")]` attribute
  - `#[derive(Fragment)]` with `#[fragment(strategy = "...")]` attribute
  - Compile-time validation of HTML IDs and names
  - Supports all standard HTMX swap strategies plus custom strategies
  - **Dynamic value support**: All three macros now support `with_fn` for computing values at runtime
    - `#[identity(with_fn = "get_id")]` - Call a function to get the HTML ID
    - `#[named(with_fn = "get_name")]` - Call a function to get the HTML name
    - `#[fragment(with_fn = "get_strategy")]` - Call a function to get the swap strategy
    - The function should be a method on the type that returns the appropriate type (HtmlId, HtmlName, or InsertStrategy)
  - Examples:
    ```rust
    // Static values
    #[derive(Identity, Fragment)]
    #[identity("notification")]
    #[fragment(strategy = "innerHTML")]
    struct Notification {
        message: String,
    }

    #[derive(Named)]
    #[named("user-email")]
    struct EmailField {
        value: String,
    }

    // Dynamic values
    #[derive(Identity, Fragment)]
    #[identity(with_fn = "get_id")]
    #[fragment(with_fn = "get_strategy")]
    struct DynamicElement {
        index: usize,
    }

    impl DynamicElement {
        fn get_id(&self) -> HtmlId {
            HtmlId::from_string(format!("element-{}", self.index))
                .expect("valid ID")
        }

        fn get_strategy(&self) -> InsertStrategy {
            InsertStrategy::InnerHtml
        }
    }
    ```
- **License checking with cargo-deny**
  - Added `deny.toml` configuration to ensure only permissive licenses (MIT, Apache-2.0, BSD, ISC, MPL-2.0, Unicode-3.0)
  - Security advisory checks via RustSec Advisory Database
  - Added `just deny` and `just check` commands

### Changed
- **BREAKING**: `Response::with_oob()` now requires `Fragment` trait instead of just `Identity`
  - Elements must implement `Fragment` and specify their swap strategy
  - Migration: Use derive macro or implement `Fragment` trait manually
  - Example with derive:
    ```rust
    #[derive(Identity, Fragment)]
    #[identity("my-element")]
    #[fragment(strategy = "outerHTML")]
    struct MyElement;
    ```
  - Example manual implementation:
    ```rust
    impl Fragment for MyElement {
        fn insert_strategy(&self) -> InsertStrategy {
            InsertStrategy::OuterHtml
        }
    }
    ```
- **BREAKING**: `Fragment` derive macro now only accepts HTMX-standard strategy strings
  - Snake_case variants (e.g., `"inner_html"`, `"outer_html"`) are no longer supported
  - Use exact HTMX strings instead: `"innerHTML"`, `"outerHTML"`, `"beforebegin"`, `"afterbegin"`, `"beforeend"`, `"afterend"`, `"textContent"`, `"delete"`, `"none"`
  - Custom strategies (any other string) are still supported and will be treated as `InsertStrategy::Custom`
  - Migration: Replace snake_case variants with camelCase HTMX-standard strings
    - `"inner_html"` → `"innerHTML"`
    - `"outer_html"` → `"outerHTML"`
    - `"text_content"` → `"textContent"`
    - `"before_begin"` → `"beforebegin"`
    - `"after_begin"` → `"afterbegin"`
    - `"before_end"` → `"beforeend"`
    - `"after_end"` → `"afterend"`

### Fixed
- Fixed `with_oob()` implementation to properly inject `hx-swap-oob` attributes
  - The `hx-swap-oob` attribute is now injected directly into the root element instead of wrapping in a `<div>`
  - Multiple root elements are automatically wrapped in a `<template>` tag
  - Added `scraper` dependency (ISC license) for HTML parsing and manipulation
  - Supports all HTMX swap-oob use cases as documented at https://htmx.org/attributes/hx-swap-oob/
- Refactored derive macro code to eliminate duplication
  - Created shared utility functions for HTML identifier validation and `with_fn` attribute parsing
  - Reduced code duplication across Identity, Named, and Fragment macros

## [0.13.0] - 2025-10-27

### Added
- Added test for combined query and body parameters in routes
- Routes can now have both `#[query]` and `#[body]` parameters simultaneously

### Changed
- **BREAKING**: Route derive macro errors now point to the correct source location instead of `call_site()`

### Fixed
- Error spans in derive macros now correctly point to the problematic code location
- Removed artificial restriction that prevented routes from having both query and body parameters

## [0.12.0] - 2025-10-26

### Added

#### New Features
- **`#[derive(ComponentsController)]` macro**: Automatically generate controller hierarchies with sub-components
  - Supports parameterized routes with path parameters
  - Automatic `AsComponent` implementations
  - Support for `convert_with` to customize component construction
- **Catch-all routes**: Added `#[catch_all]` attribute for routes that handle any unmatched paths
- **Controller `Args` type**: Controllers can now specify construction arguments via the `Args` associated type
- **`ControllerExt` trait**: Convenience methods `get_component()` and `get_component_with()` for accessing sub-controllers
- **`AsComponent` trait**: Enables composing controllers with parent-to-child conversions
- **`Identity` trait**: New trait for HTML elements with unique identifiers
  - Automatic ID extraction for OOB swaps
  - Includes `id_attribute()` helper method for templates
- **`Response::with_raw_oob()` method**: Low-level OOB method for custom swap strategies and targets
- New HTML form traits and ID/name types for better type safety

#### Dependencies
- Added `serde_html_form` (replaces `serde_urlencoded`)
- Added `axum-extra` as required dependency for the `derive` feature

### Changed

#### Breaking Changes

##### Controller Trait
- **BREAKING**: The `Controller` trait has been significantly refactored:
  ```rust
  // Before (v0.11.0)
  pub trait Controller: Send + Sync + Clone + 'static {
      type Route: Route + Send + axum::extract::FromRequest<Self>;

      fn render_view(
          &self,
          route: Self::Route,
          htmx: super::htmx::Request,
          parts: http::request::Parts,
          server_info: &super::ServerInfo,
      ) -> impl Future<Output = axum::response::Response> + Send;
  }

  // After (v0.12.0)
  pub trait Controller: Send + Sync + Clone {
      type Route: Route + Send + axum::extract::FromRequest<Self>;
      type Args: Send + Sync + 'static;  // NEW

      fn handle_request(  // RENAMED from render_view
          &self,
          route: Self::Route,
          htmx: super::htmx::Request,
          parts: http::request::Parts,
          server_info: &super::ServerInfo,
      ) -> impl Future<Output = Result<axum::response::Response, axum::response::Response>> + Send;  // NEW: returns Result
  }
  ```

##### Query Parameter Serialization
- **BREAKING**: Query parameters now use `serde_html_form` instead of `serde_urlencoded`
  - This affects how query parameters are serialized in route URLs
  - The new implementation properly handles empty query strings (no `?` is appended when empty)
  - Better support for HTML form serialization standards

##### Out-of-Band (OOB) Swap Behavior
- **BREAKING**: The `Response::with_oob()` method signature and behavior has changed:
  ```rust
  // Before (v0.11.0)
  response.with_oob(
      "#my-element",           // Manual target selector
      my_element,              // Any Display type
  )
  // Default swap: InnerHtml

  // After (v0.12.0)
  response.with_oob(my_element)  // Element must implement Identity trait
  // Default swap: OuterHtml
  // Target is automatically extracted from element.id()
  ```
  - Default swap method changed from `InnerHtml` to `OuterHtml`
  - The `with_oob()` method now requires elements to implement the `Identity` trait
  - The target selector is automatically generated from the element's ID
  - For custom targets or swap methods, use the new `with_raw_oob()` method

### Fixed
- Query parameters no longer append `?` when the query string is empty
- Various internal refactoring to improve code organization and maintainability

---

## Migration Guide: v0.11.0 → v0.12.0

### 1. Update Controller Implementations

**Change 1: Rename `render_view` to `handle_request`**

```rust
// Before
impl Controller for MyController {
    type Route = MyRoute;

    async fn render_view(
        &self,
        route: Self::Route,
        htmx: htmxology::htmx::Request,
        parts: http::request::Parts,
        server_info: &htmxology::ServerInfo,
    ) -> axum::response::Response {
        // ... implementation
    }
}

// After
impl Controller for MyController {
    type Route = MyRoute;
    type Args = ();  // NEW: Add this if your controller doesn't need construction args

    async fn handle_request(  // RENAMED
        &self,
        route: Self::Route,
        htmx: htmxology::htmx::Request,
        parts: http::request::Parts,
        server_info: &htmxology::ServerInfo,
    ) -> Result<axum::response::Response, axum::response::Response> {  // NEW: Returns Result
        // ... implementation
        Ok(response)  // Wrap your response in Ok()
    }
}
```

**Change 2: Return `Result` instead of `Response`**

Update all your `handle_request` implementations to return `Result<Response, Response>`:
- Success case: `Ok(response)`
- Error case: `Err(error_response)`

### 2. Update Dependencies in `Cargo.toml`

If you were using `serde_urlencoded` for query parameters:

```toml
# Before
serde_urlencoded = "0.7"

# After
serde_html_form = "0.2"
```

Add `axum-extra` if using the `derive` feature:

```toml
[dependencies]
axum-extra = { version = "0.9", features = ["query"] }
```

### 3. Update Out-of-Band (OOB) Swaps

The `with_oob()` method now requires elements to implement the `Identity` trait and uses `OuterHtml` by default.

**Option 1: Implement the `Identity` trait** (Recommended)

```rust
use htmxology::htmx::Identity;
use std::borrow::Cow;

// Your template/component
struct MyElement {
    id: String,
    // ... other fields
}

impl Identity for MyElement {
    fn id(&self) -> Cow<'static, str> {
        Cow::Owned(self.id.clone())
    }
}

impl Display for MyElement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Your HTML must include: <div {{ id_attribute()|safe }}>...</div>
        write!(f, "<div {}>Content</div>", self.id_attribute())
    }
}

// Before (v0.11.0)
response.with_oob("#my-element", my_element)

// After (v0.12.0)
response.with_oob(my_element)  // Target and ID are handled automatically
```

**Option 2: Use `with_raw_oob()` for custom behavior**

If you need a custom target selector or swap method:

```rust
// Before (v0.11.0)
response.with_oob("#my-element", my_element)

// After (v0.12.0) - for backward compatibility
use htmxology::htmx::InsertStrategy;
response.with_raw_oob(
    InsertStrategy::InnerHtml,  // Use InnerHtml like v0.11.0
    "#my-element",              // Custom target selector
    my_element
)
```

**Important**: Note that the default swap changed from `InnerHtml` to `OuterHtml`. If you rely on the old behavior, use `with_raw_oob()` with `InsertStrategy::InnerHtml`.

### 4. Update Query Parameter Handling

If you were manually handling query parameters, update from `serde_urlencoded` to `serde_html_form`:

```rust
// Before
use serde_urlencoded;
let query = serde_urlencoded::to_string(&params)?;

// After
use serde_html_form;
let query = serde_html_form::to_string(&params)?;
```

### 5. Use New ComponentsController Macro (Optional)

If you have complex controller hierarchies, consider using the new `ComponentsController` derive macro:

```rust
#[derive(ComponentsController)]
#[controller(AppRoute)]
#[component(HomeController, route = Home, path = "")]
#[component(BlogController, route = Blog, path = "blog/")]
struct AppController {
    home: HomeController,
    blog: BlogController,
}
```

This automatically generates:
- The `AppRoute` enum with all sub-routes
- `AsComponent` implementations for conversions
- `handle_request` implementation that delegates to sub-controllers

### 5. Parameterized Controllers (Optional)

If you need to pass parameters from parent routes to child controllers:

```rust
#[derive(ComponentsController)]
#[controller(AppRoute)]
#[component(
    BlogController,
    route = Blog,
    path = "blog/{blog_id}/",
    params(blog_id: u32)  // Extract blog_id from path
)]
struct AppController {
    state: AppState,
}

impl Controller for BlogController {
    type Route = BlogRoute;
    type Args = (u32,);  // Accepts blog_id as argument

    // ... implementation
}

// In AppController, implement From to construct BlogController
impl From<(&AppController, u32)> for BlogController {
    fn from((app, blog_id): (&AppController, u32)) -> Self {
        BlogController::new(&app.state, blog_id)
    }
}
```

### 6. Catch-All Routes (Optional)

To handle unmatched routes:

```rust
#[derive(Route)]
enum MyRoute {
    #[route("")]
    Home,

    #[route("about")]
    About,

    #[catch_all]
    NotFound(NotFoundRoute),  // Handles any other path
}
```

### Breaking Changes Checklist

- [ ] Rename all `render_view` methods to `handle_request`
- [ ] Add `type Args = ();` to all `Controller` implementations
- [ ] Change return type from `Response` to `Result<Response, Response>`
- [ ] Wrap all response returns in `Ok()`
- [ ] Update all `with_oob()` calls to either:
  - Implement `Identity` trait for OOB elements (recommended), or
  - Use `with_raw_oob()` for backward compatibility
- [ ] Verify OOB swap behavior (default changed from `InnerHtml` to `OuterHtml`)
- [ ] Update `serde_urlencoded` to `serde_html_form` in dependencies
- [ ] Update any manual query parameter serialization code
- [ ] Add `axum-extra` dependency if using `derive` feature

[Unreleased]: https://github.com/your-repo/htmxology.rs/compare/v0.13.0...HEAD
[0.13.0]: https://github.com/your-repo/htmxology.rs/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/your-repo/htmxology.rs/releases/tag/v0.12.0
