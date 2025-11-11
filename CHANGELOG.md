# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.22.0] - 2025-11-11

### Fixed
- **FromStr implementation for subroutes and catch-all routes**
  - `FromStr` now correctly handles routes with subroute variants
  - Subroutes parse their path parameters and delegate to the inner route's `FromStr`
  - Catch-all routes are now used as a fallback when no other routes match
  - Enables complete URL-to-Route parsing for all GET route types
  - Added comprehensive tests for subroutes and catch-all FromStr behavior

## [0.21.0] - 2025-11-11

### Added
- **Extra derives support** in `RoutingController` macro (Issue #24)
  - New optional `extra_derives` parameter in `#[controller(...)]` attribute
  - Allows specifying additional derive traits for the generated Route enum
  - Syntax: `#[controller(AppRoute, extra_derives = (PartialEq, Eq, Hash))]`
  - The extra derives are appended to the default derives (Debug, Clone, htmxology::Route)
  - Useful for adding `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`, etc.
  - Example:
    ```rust
    #[derive(RoutingController)]
    #[controller(AppRoute, extra_derives = (PartialEq, Eq, Hash))]
    #[subcontroller(HomeController, route = Home, path = "")]
    struct AppController { ... }
    // Generates: #[derive(Debug, Clone, htmxology::Route, PartialEq, Eq, Hash)]
    ```
- **Pre-handler support** in `RoutingController` macro (Issue #20)
  - New optional `pre_handler` parameter in `#[controller(...)]` attribute
  - Async function called before routing to enable authentication, rate limiting, and request validation
  - Signature: `async fn(&self, &Route, &htmx::Request, &http::request::Parts, &ServerInfo, &mut Args) -> Option<Response>`
  - Returns `Some(response)` to short-circuit routing and return immediately
  - Returns `None` to proceed with normal routing
  - Example:
    ```rust
    #[controller(AppRoute, args = Session, pre_handler = "Self::authenticate")]
    struct MyController { ... }

    impl MyController {
        async fn authenticate(&self, route: &AppRoute, ..., args: &mut Session) -> Option<Response> {
            if !args.is_authenticated {
                return Some(Ok(Redirect::to("/login").into_response()));
            }
            None
        }
    }
    ```
- **`FromStr` implementation for Route types** (Issue #26)
  - The `Route` derive macro now automatically implements `std::str::FromStr` for GET routes
  - Allows parsing URL strings into route instances: `"/users/123".parse::<MyRoute>()`
  - Only works for GET routes (routes without request bodies) since they can be fully represented by a URL
  - Parses both path parameters and query parameters from the URL string
  - Returns `htmxology::ParseError` if:
    - No matching GET route is found
    - URL matches a non-GET route
    - Path or query parameters fail to parse
  - Example:
    ```rust
    #[derive(Route)]
    enum MyRoute {
        #[route("users/{id}")]
        User { id: u32, #[query] filters: Filters },
    }

    let route: MyRoute = "/users/123?sort=asc".parse()?;
    ```
- **`ParseError` type for route parsing errors**
  - New error type `htmxology::ParseError` with detailed error variants
  - Replaces String errors in `FromStr` implementation
  - Provides clear error messages for debugging route parsing issues

### Changed
- **BREAKING**: Simplified `HasSubcontroller` trait by removing `convert_response` method (Issue #22)
  - The `convert_response` trait method has been removed
  - Response conversion is now inlined directly in the generated `handle_request` method
  - Default behavior uses `.into()` (same as before)
  - Custom conversion functions can still be specified via `convert_response = "fn"` attribute in `RoutingController` macro
  - **For users of `RoutingController` macro**: No code changes needed - the macro generates everything automatically
  - **For manual `HasSubcontroller` implementations** (rare): Remove the `convert_response` method from your impl block
  - Benefits:
    - Simpler trait with less boilerplate
    - More idiomatic use of Rust's `Into`/`From` traits
    - Better ergonomics as `htmx` context is naturally available in generated code
    - No double indirection
- Generated `handle_request` now uses `mut args: Self::Args` when `pre_handler` is configured
  - Only applies when using the `pre_handler` parameter
  - Controllers without `pre_handler` continue to use `args: Self::Args` (no breaking change)
  - Manual `Controller` implementations don't need to change unless using `pre_handler`

## [0.20.0] - 2025-11-04

### Changed
- **BREAKING**: Removed automatic `From<Controller> for ControllerRouter` implementation from `RoutingController` macro (Issue #21)
  - The generated `From` implementation was too restrictive, requiring `Controller::Response = Result<axum::response::Response, axum::response::Response>`
  - This prevented using custom response types for semantic composition between controllers
  - Users must now manually create `ControllerRouter` instances using `ControllerRouter::new(controller, args_factory)`
- **BREAKING**: Removed `args_factory` parameter from `#[controller(...)]` attribute (Issue #21)
  - The parameter is no longer needed since the `From` implementation is not generated
  - Args factory is now specified directly when calling `ControllerRouter::new()`
  - Migration example:
    ```rust
    // Before (v0.19.0):
    #[controller(AppRoute, args = UserSession, args_factory = "|c: &Controller| async { c.session.clone() }")]
    struct MyController { session: UserSession }

    server.serve(MyController::new(session)).await?; // Automatic conversion

    // After (v0.20.0):
    #[controller(AppRoute, args = UserSession)]
    struct MyController { session: UserSession }

    let controller = MyController::new(session);
    let router = ControllerRouter::new(controller, |c| async { c.session().clone() });
    server.serve(router).await?;
    ```

## [0.19.0] - 2025-11-03

### Added
- **Custom Args type support** in `RoutingController` macro: The `#[controller(...)]` attribute now accepts an optional `args` parameter to specify custom Args types (Issue #19)
  - Default: `#[controller(AppRoute)]` uses `Args = ()`
  - Custom: `#[controller(AppRoute, args = Session)]`
  - Enables passing custom request-scoped state (like user sessions) through the controller hierarchy
- **Async args_factory support**: The `#[controller(...)]` attribute now accepts an optional `args_factory` parameter to specify how Args are created for each request
  - Factory is called per-request with a reference to the controller: `Fn(&Controller) -> impl Future<Output = Args>`
  - Example: `#[controller(AppRoute, args = UserSession, args_factory = "|controller: &MainController| -> _ { let session = controller.session.clone(); async move { session } }")]`
  - Enables async initialization of request-scoped state (e.g., database queries, authentication checks)

### Changed
- **BREAKING**: `Controller::Args` now represents transient request data passed to `handle_request()` by value instead of construction parameters (Issue #19)
  - `Args` is now passed as `args: Self::Args` to `handle_request()` method (not `&mut Self::Args`)
  - Args are created fresh for each request via the args_factory function
  - Enables passing session data and other request-scoped state through the controller hierarchy
  - **Important**: Args must be owned types (`'static` bound). For shared mutable state, use `Arc<RwLock<T>>` or similar patterns
  - **Args inheritance**: Child controllers receive parent Args merged with path parameters
    - Path parameters declared with `params()` in `#[subcontroller]` are combined with parent Args
    - Generated code: `ChildArgs::from((parent_args, param1, param2, ...))`
    - If parent has `Args = ()`, child receives `((), param1, param2, ...)`
    - If parent has `Args = AppContext`, child receives `(AppContext, param1, param2, ...)`
  - Migration:
    - Change `args: &mut Self::Args` parameter to `args: Self::Args` in all `handle_request()` implementations
    - Update `From` implementations for parameterized subcontrollers to take values instead of mutable references
    - Example with parent `Args = ()`: `impl From<((), u32, String)> for UserPostArgs { ... }`
    - Example with parent `Args = AppContext`: `impl From<(AppContext, u32, String)> for UserPostArgs { ... }`
    - Controllers without params continue to receive parent args directly (no change needed)
- **BREAKING**: `ControllerRouter::new()` now requires an `args_factory` parameter
  - Factory function signature: `Fn(&C) -> impl Future<Output = C::Args> + Send`
  - Called once per request to create Args for that request
  - Example: `ControllerRouter::new(controller, |_| async { () })`

## [0.18.0] - 2025-10-30

### Added
- **Made `htmx::Request` clonable**: Added `Clone` derive to `htmx::Request` enum to enable easier request handling (Issue #15)
- **Convenience methods in `SubcontrollerExt`**: Added `handle_subcontroller_request()` and `handle_subcontroller_request_with()` methods that combine getting a subcontroller, calling `handle_request`, and converting the response in a single method call (Issue #18)
  - Significantly reduces boilerplate when delegating requests to subcontrollers
  - Example: `self.handle_subcontroller_request::<BlogController>(route, htmx, parts, server_info).await`

### Changed
- **BREAKING**: `HasSubcontroller::convert_response()` now accepts `htmx: &htmx::Request` as its first parameter (Issue #16)
  - Enables parent controllers to access HTMX request context when converting child responses
  - Allows decisions between full pages vs fragments based on request type
  - Migration: Add `htmx: &htmx::Request` as first parameter to all `convert_response` implementations
- **BREAKING**: Renamed `AsSubcontroller` trait to `HasSubcontroller` (Issue #17)
  - The new name better reflects that the trait is implemented on parent controllers to indicate they "have" or "provide" a subcontroller
  - Migration: Replace all `AsSubcontroller` references with `HasSubcontroller`

## [0.17.0] - 2025-10-29

### Added
- **Custom Response type support** in `RoutingController` macro: The `#[controller(...)]` attribute now accepts an optional `response` parameter to specify custom Response types (Issue #13)
  - Default: `#[controller(AppRoute)]` uses `Result<axum::response::Response, axum::response::Response>`
  - Custom: `#[controller(AppRoute, response = Result<MyResponse, MyError>)]`
  - Enables better type safety and semantic composition with domain-specific error and response types

### Fixed
- **RoutingController macro no longer requires SubcontrollerExt import**: The macro now uses fully-qualified trait syntax (`htmxology::SubcontrollerExt::get_subcontroller()`) eliminating the need for explicit trait imports (Issue #12)

## [0.16.0] - 2025-10-29

### Added
- Re-exported `Identity`, `Named`, and `Fragment` traits at root level as `IdentityTrait`, `NamedTrait`, and `FragmentTrait` for convenience (Issue #7)
- **`wrap_response` helper function**: Converts `axum::response::Response` to `Result<axum::response::Response, axum::response::Response>` for common controller composition scenarios
- **`convert_response` attribute** for `RoutingController` macro: Allows specifying custom response conversion functions when composing controllers with different response types
  - Example: `#[subcontroller(MyController, route = MyRoute, path = "my-path/", convert_response = "Ok")]`
  - Default behavior uses `.into()` trait-based conversion

### Changed
- **BREAKING**: `Controller` trait now has a `Response` associated type instead of separate `Output`/`ErrorOutput` types (Issue #11)
  - Root controllers should use `Response = Result<axum::response::Response, axum::response::Response>`
  - Intermediate controllers can use custom response types for semantic composition
  - Migration: Change `type Output = T; type ErrorOutput = E;` to `type Response = Result<T, E>;`
- **BREAKING**: `AsSubcontroller` trait now includes a `convert_response()` method to enable flexible response type conversion between parent and child controllers (Issue #11)
  - Automatically generated by the `RoutingController` macro
  - Uses `.into()` by default, or custom function specified via `convert_response` attribute
- **BREAKING**: `#[identity(...)]` now requires `id = "value"` syntax instead of positional `#[identity("value")]` (Issue #9)
- **BREAKING**: `#[named(...)]` now requires `name = "value"` syntax instead of positional `#[named("value")]` (Issue #9)
- **BREAKING**: All `with_fn` attributes now require full function paths (e.g., `Foo::method` or `Self::method`) instead of just method names (Issue #9)
- `#[fragment(...)]` attribute is now optional and defaults to `outerHTML` strategy (Issue #8)

### Fixed
- Fixed `Identity`, `Named`, and `Fragment` derive macros to support generic type parameters with default values (Issue #10)
  - Previously, types like `struct Foo<T: Display = Bar>` would generate invalid syntax
  - Added `extract_generic_param_idents()` utility function to separate parameter declarations from usage

## [0.15.0] - 2025-10-28

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
