# RoutingController Extra Derives

This document describes the design for adding support for additional derive traits in the `RoutingController` macro (Issue #24).

## Problem Statement

The `RoutingController` macro automatically generates a `Route` enum with a fixed set of derives: `Debug`, `Clone`, and `htmxology::Route`. However, users often need additional trait implementations on their generated Route enums, such as:

- `PartialEq` and `Eq` - For equality comparisons
- `Hash` - For using routes as HashMap/HashSet keys
- `PartialOrd` and `Ord` - For ordering routes
- `serde::Serialize` and `serde::Deserialize` - For serialization
- Other custom derives

Currently, there's no way to add these derives without manually implementing them, which defeats the purpose of the macro.

## Proposed Solution

Add an optional `extra_derives` argument to the `#[controller(...)]` attribute that accepts a comma-separated list of derive traits to be added to the generated Route enum.

### Syntax

```rust
#[derive(RoutingController)]
#[controller(RouteEnumName, extra_derives(Trait1, Trait2, Trait3))]
#[subcontroller(...)]
struct MyController {
    // ...
}
```

### Example Usage

#### Basic Example

```rust
use htmxology::RoutingController;

#[derive(Debug, Clone, RoutingController)]
#[controller(AppRoute, extra_derives(PartialEq, Eq, Hash))]
#[subcontroller(BlogController, route = Blog, path = "blog/")]
#[subcontroller(UserController, route = User, path = "user/{user_id}/", params(user_id: u32))]
struct MainController {
    // controller fields
}
```

This would generate:

```rust
#[derive(Debug, Clone, htmxology::Route, PartialEq, Eq, Hash)]
pub enum AppRoute {
    #[route("blog/")]
    Blog(#[subroute] <BlogController as htmxology::Controller>::Route),

    #[route("user/{user_id}/")]
    User {
        user_id: u32,
        #[subroute]
        subroute: <UserController as htmxology::Controller>::Route,
    },
}
```

#### With Serialization Support

```rust
#[derive(RoutingController)]
#[controller(ApiRoute, extra_derives(serde::Serialize, serde::Deserialize))]
#[subcontroller(UsersController, route = Users, path = "api/users/")]
#[subcontroller(PostsController, route = Posts, path = "api/posts/")]
struct ApiController {
    // controller fields
}
```

This enables routes to be serialized/deserialized, useful for:
- Storing routes in databases
- Passing routes between services
- Caching routing information

#### Combined with Other Options

```rust
#[derive(RoutingController)]
#[controller(
    AppRoute,
    response = Result<MyResponse, MyError>,
    args = AppState,
    extra_derives(PartialEq, Eq, Hash, PartialOrd, Ord)
)]
#[subcontroller(HomeController, route = Home, path = "")]
#[subcontroller(BlogController, route = Blog, path = "blog/")]
struct MainController {
    state: AppState,
}
```

## Implementation Plan

### 1. Update ControllerSpec Structure

Location: `htmxology-macros/src/routing_controller/mod.rs` (around line 325)

Add a field to store the extra derives:

```rust
struct ControllerSpec {
    route_ident: syn::Ident,
    response_type: Option<syn::Type>,
    args_type: Option<syn::Type>,
    pre_handler: Option<syn::Path>,
    extra_derives: Vec<syn::Path>,  // NEW FIELD
}
```

### 2. Update Parser

Location: `htmxology-macros/src/routing_controller/mod.rs` (around line 350-395)

Extend the parser to recognize and parse `extra_derives(...)` syntax:

```rust
impl Parse for ControllerSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let route_ident: syn::Ident = input.parse()?;

        let mut response_type = None;
        let mut args_type = None;
        let mut pre_handler = None;
        let mut extra_derives = Vec::new();  // NEW

        while !input.is_empty() {
            input.parse::<syn::Token![,]>()?;

            if input.is_empty() {
                break;
            }

            let lookahead = input.lookahead1();

            // ... existing parsing logic ...

            // NEW: Parse extra_derives
            else if lookahead.peek(kw::extra_derives) {
                input.parse::<kw::extra_derives>()?;
                let content;
                syn::parenthesized!(content in input);

                loop {
                    extra_derives.push(content.parse()?);

                    if content.is_empty() {
                        break;
                    }

                    content.parse::<syn::Token![,]>()?;

                    if content.is_empty() {
                        break;
                    }
                }
            }
        }

        Ok(ControllerSpec {
            route_ident,
            response_type,
            args_type,
            pre_handler,
            extra_derives,  // NEW
        })
    }
}
```

### 3. Update Route Enum Generation

Location: `htmxology-macros/src/routing_controller/mod.rs` (around line 165-170)

Modify the enum generation to include extra derives:

```rust
let extra_derives = &controller_spec.extra_derives;

let route_decl = if extra_derives.is_empty() {
    quote_spanned! { route_ident.span() =>
        #[derive(Debug, Clone, htmxology::Route)]
        pub enum #route_ident {
            #(#route_variants)*
        }
    }
} else {
    quote_spanned! { route_ident.span() =>
        #[derive(Debug, Clone, htmxology::Route, #(#extra_derives),*)]
        pub enum #route_ident {
            #(#route_variants)*
        }
    }
};
```

### 4. Add Custom Keyword

Location: `htmxology-macros/src/routing_controller/mod.rs` (top of file with other keywords)

Add the `extra_derives` keyword:

```rust
mod kw {
    syn::custom_keyword!(route);
    syn::custom_keyword!(response);
    syn::custom_keyword!(args);
    syn::custom_keyword!(pre_handler);
    syn::custom_keyword!(extra_derives);  // NEW
    // ... other keywords
}
```

### 5. Testing Strategy

#### Snapshot Tests

Add new snapshot tests in `htmxology-macros/src/routing_controller/snapshot_tests.rs`:

- `test_extra_derives_single` - Single extra derive
- `test_extra_derives_multiple` - Multiple extra derives
- `test_extra_derives_with_paths` - Fully-qualified paths (e.g., `serde::Serialize`)
- `test_extra_derives_combined` - Combined with `response`, `args`, `pre_handler`
- `test_no_extra_derives` - Verify backward compatibility (no extra_derives specified)

#### Integration Tests

Verify the generated code compiles and works correctly with real trait implementations.

## Use Cases

### 1. Using Routes as HashMap Keys

```rust
use std::collections::HashMap;

#[derive(RoutingController)]
#[controller(AppRoute, extra_derives(PartialEq, Eq, Hash))]
#[subcontroller(BlogController, route = Blog, path = "blog/")]
struct MainController {}

// Now we can use routes as keys:
let mut cache: HashMap<AppRoute, CachedResponse> = HashMap::new();
cache.insert(route, response);
```

### 2. Comparing Routes

```rust
#[derive(RoutingController)]
#[controller(AppRoute, extra_derives(PartialEq, Eq))]
#[subcontroller(HomeController, route = Home, path = "")]
struct MainController {}

// Check if two routes are the same:
if route1 == route2 {
    // ...
}
```

### 3. Ordering Routes

```rust
#[derive(RoutingController)]
#[controller(AppRoute, extra_derives(PartialEq, Eq, PartialOrd, Ord))]
#[subcontroller(BlogController, route = Blog, path = "blog/")]
struct MainController {}

// Sort routes by priority:
let mut routes = vec![route1, route2, route3];
routes.sort();
```

### 4. Serializing Routes

```rust
#[derive(RoutingController)]
#[controller(ApiRoute, extra_derives(serde::Serialize, serde::Deserialize))]
#[subcontroller(UsersController, route = Users, path = "api/users/")]
struct ApiController {}

// Serialize route to JSON:
let json = serde_json::to_string(&route)?;

// Deserialize route from JSON:
let route: ApiRoute = serde_json::from_str(&json)?;
```

## Backward Compatibility

This feature is **fully backward compatible**:

- The `extra_derives` argument is **optional**
- If not specified, the macro behaves exactly as before
- Existing code requires no changes
- All existing tests continue to pass

## Alternatives Considered

### 1. Using a separate `#[derive_route(...)]` attribute

```rust
#[derive(RoutingController)]
#[controller(AppRoute)]
#[derive_route(PartialEq, Eq, Hash)]  // Separate attribute
#[subcontroller(...)]
struct MainController {}
```

**Rejected because:**
- Adds an extra attribute when one should suffice
- Less intuitive - the derives are logically part of the controller configuration
- Harder to discover (users expect all config in `#[controller(...)]`)

### 2. Using array syntax: `extra_derives = [...]`

```rust
#[controller(AppRoute, extra_derives = [PartialEq, Eq, Hash])]
```

**Rejected because:**
- Less idiomatic for Rust macros
- Harder to parse (requires bracket matching)
- The parenthesized list syntax is more common in derive macros

### 3. Auto-deriving common traits

Automatically derive `PartialEq`, `Eq`, and `Hash` without user input.

**Rejected because:**
- Not all routes can implement these traits (depends on field types)
- Violates principle of least surprise
- Forces users to implement traits on all subroute types
- Removes flexibility

## Documentation Updates

The following documentation should be updated:

1. **CLAUDE.md** - Add section explaining `extra_derives` with examples
2. **README.md** (if applicable) - Mention the feature in routing section
3. **API docs** - Add rustdoc examples showing common use cases
4. **examples/** - Consider adding an example showing HashMap usage

## Future Enhancements

Possible future extensions:

1. **Validation** - Warn if extra derives might not work (e.g., `Hash` without `Eq`)
2. **Presets** - Shorthand for common combinations (e.g., `extra_derives(eq_hash)` → `PartialEq, Eq, Hash`)
3. **Conditional derives** - Feature-gated derives (e.g., `extra_derives(#[cfg(feature = "serde")] serde::Serialize)`)

These are not part of this proposal and should be considered separately if needed.
