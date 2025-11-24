# Enhanced `convert_response` Signature

**Issue:** [#28](https://github.com/ereOn/htmxology.rs/issues/28)
**Status:** Planned
**Author:** Claude
**Date:** 2025-11-24

## Summary

This document describes the planned enhancement to the `convert_response` attribute in the `RoutingController` macro. The enhancement will provide full request context to response conversion functions by passing all `handle_request` parameters by reference.

## Motivation

Currently, the `convert_response` attribute receives only two parameters:
- The HTMX request (`&htmx::Request`)
- The child controller's response

This limits the conversion logic to decisions based only on the response itself and basic HTMX context. Many real-world scenarios require access to:
- The specific route that was matched
- HTTP request parts (headers, method, URI, etc.)
- Server information
- Controller arguments (session, database connections, etc.)

## Current State

### Current Signature

```rust
fn convert_response(
    htmx: &htmxology::htmx::Request,
    response: ChildResponse,
) -> ParentResponse
```

### Usage Example

```rust
impl MainController {
    fn convert_plain_response(
        _htmx: &htmxology::htmx::Request,
        response: axum::response::Response,
    ) -> Result<axum::response::Response, axum::response::Response> {
        Ok(response)
    }
}
```

## Proposed Changes

### New Signature

```rust
fn convert_response(
    route: &Self::Route,
    htmx: &htmxology::htmx::Request,
    parts: &http::request::Parts,
    server_info: &htmxology::ServerInfo,
    args: &Self::Args,
    response: ChildResponse,
) -> ParentResponse
```

### Parameters

1. **`route: &Self::Route`** - The matched route variant, enabling route-specific conversion logic
2. **`htmx: &htmxology::htmx::Request`** - HTMX request context (existing parameter)
3. **`parts: &http::request::Parts`** - HTTP request metadata (method, URI, headers, etc.)
4. **`server_info: &htmxology::ServerInfo`** - Server configuration and base URL
5. **`args: &Self::Args`** - Controller arguments (session, database, etc.)
6. **`response: ChildResponse`** - The child controller's response (existing parameter)

## Implementation Plan

### Phase 1: Update Macro Code Generation

**File:** `htmxology-macros/src/routing_controller/mod.rs`

#### Step 1: Update Conversion Logic Generation (Lines 109-119)

Modify the `conversion_logic` quote block to pass all parameters:

```rust
let conversion_logic = if let Some(fn_expr) = convert_response_fn {
    // Custom function specified
    quote! {
        #fn_expr(&route, &htmx, &parts, server_info, &args, response)
    }
} else {
    // Default: use Into trait
    quote! {
        response.into()
    }
};
```

#### Step 2: Update Match Arm Generation (Lines 122-148)

For **parameterless routes**:
```rust
quote_spanned! { spec.route_variant.span() =>
    Self::Route::#route_variant(route) => {
        let response = htmxology::SubcontrollerExt::get_subcontroller::<#controller_type>(self)
            .handle_request(route, htmx.clone(), parts, server_info, args)
            .await;
        // Pass the parent route variant reference
        let parent_route = Self::Route::#route_variant(route.clone());
        #conversion_logic  // Now has access to parent_route
    }
}
```

For **parameterized routes**:
```rust
quote_spanned! { spec.route_variant.span() =>
    Self::Route::#route_variant { #(#param_names,)* subroute } => {
        let sub_args = <#controller_type as htmxology::Controller>::Args::from((args, #(#param_names_for_construction,)*));
        let response = htmxology::SubcontrollerExt::get_subcontroller::<#controller_type>(self)
            .handle_request(subroute, htmx.clone(), parts, server_info, sub_args)
            .await;
        // Capture parent route for conversion
        let parent_route = &Self::Route::#route_variant {
            #(#param_names,)*
            subroute: subroute.clone()
        };
        #conversion_logic  // Now has access to parent_route
    }
}
```

### Phase 2: Update Example Code

**File:** `examples/components.rs` (Lines 87-94)

```rust
impl MainController {
    #[expect(clippy::result_large_err)]
    fn convert_plain_response(
        _route: &AppRoute,
        _htmx: &htmxology::htmx::Request,
        _parts: &http::request::Parts,
        _server_info: &htmxology::ServerInfo,
        _args: &UserSession,
        response: axum::response::Response,
    ) -> Result<axum::response::Response, axum::response::Response> {
        Ok(response)
    }
}
```

### Phase 3: Regenerate Snapshot Tests

**Files:** `htmxology-macros/src/routing_controller/snapshots/*.snap`

Run snapshot test updates:
```bash
cargo test -p htmxology-macros
cargo insta review
```

Expected changes in all snapshot files:
- `convert_response` function signatures updated
- Call sites pass all required parameters
- Both parameterized and non-parameterized route variants handled correctly

### Phase 4: Update Documentation

#### 4.1 CLAUDE.md (Line 205-210)

```markdown
fn convert_response(
    route: &MyRoute,
    htmx: &htmx::Request,
    parts: &http::request::Parts,
    server_info: &ServerInfo,
    args: &(),
    response: BlogController::Response
) -> Self::Response {
    // Convert Result<BlogResponse, BlogError> to Result<Response, Response>
    response
        .map(|r| r.into_response())
        .map_err(|e| e.into_response())
}
```

#### 4.2 htmxology-macros/src/lib.rs (Lines 60-80)

Update the `convert_response` attribute documentation to show the new signature with all parameters.

#### 4.3 htmxology/src/controller.rs (Lines 63, 107, 155)

Update trait documentation references to mention the expanded parameter list.

## Use Cases

### 1. Route-Specific Error Handling

```rust
fn convert_api_response(
    route: &AppRoute,
    htmx: &htmx::Request,
    parts: &http::request::Parts,
    server_info: &ServerInfo,
    args: &Session,
    response: Result<ApiResponse, ApiError>,
) -> Result<Response, Response> {
    match route {
        AppRoute::Api(ApiRoute::Public(_)) => {
            // Public API: return JSON errors
            response
                .map(|r| r.into_response())
                .map_err(|e| e.to_json_response())
        }
        AppRoute::Api(ApiRoute::Internal(_)) => {
            // Internal API: return detailed errors
            response
                .map(|r| r.into_response())
                .map_err(|e| e.to_detailed_response())
        }
        _ => unreachable!(),
    }
}
```

### 2. Conditional Wrapping Based on Headers

```rust
fn convert_with_layout(
    _route: &AppRoute,
    htmx: &htmx::Request,
    parts: &http::request::Parts,
    _server_info: &ServerInfo,
    _args: &(),
    response: Html<String>,
) -> Result<Response, Response> {
    // Check if request wants partial content
    if htmx.is_boosted() || parts.headers.contains_key("X-Partial") {
        // Return unwrapped content
        Ok(response.into_response())
    } else {
        // Wrap in full page layout
        Ok(wrap_with_layout(response).into_response())
    }
}
```

### 3. Session-Aware Response Transformation

```rust
fn convert_with_auth(
    _route: &AppRoute,
    _htmx: &htmx::Request,
    _parts: &http::request::Parts,
    _server_info: &ServerInfo,
    args: &Session,
    response: Result<ProtectedContent, AuthError>,
) -> Result<Response, Response> {
    match response {
        Ok(content) => {
            // Add user info from session to response headers
            Ok((
                [(header::X_USER_ID, args.user_id.to_string())],
                content
            ).into_response())
        }
        Err(AuthError::Unauthorized) if !args.is_authenticated => {
            // Redirect to login
            Err(Redirect::to("/login").into_response())
        }
        Err(e) => Err(e.into_response()),
    }
}
```

## Testing Strategy

1. **Unit Tests**: Macro code generation with various route configurations
2. **Snapshot Tests**: Regenerate and verify all existing snapshots
3. **Integration Tests**: Build and run all examples, especially `components` example
4. **Full Suite**: Run `just check` to ensure all tests, formatting, and licenses pass

### Test Commands

```bash
# Run macro tests with snapshot updates
cargo test -p htmxology-macros
cargo insta review

# Build and test all examples
cargo check --all-targets --all-features
just example components

# Run full test suite
cargo test --all-features

# Format check
cargo fmt --all

# Complete validation
just check
```

## Breaking Changes

This is a **breaking change** for any code that uses the `convert_response` attribute.

### Migration Guide

For all `convert_response` functions, update the signature from:

```rust
// OLD
fn convert_response(
    htmx: &htmx::Request,
    response: ChildResponse,
) -> ParentResponse {
    // ...
}
```

To:

```rust
// NEW
fn convert_response(
    _route: &ParentRoute,        // Add
    htmx: &htmx::Request,
    _parts: &http::request::Parts, // Add
    _server_info: &ServerInfo,    // Add
    _args: &ParentArgs,           // Add
    response: ChildResponse,
) -> ParentResponse {
    // Prefix unused parameters with _ to avoid warnings
    // ...
}
```

## Benefits

1. **More Context**: Full access to request context for sophisticated conversion logic
2. **Better Decisions**: Route-specific, header-based, or session-aware response transformations
3. **Flexibility**: Enables advanced patterns like conditional layouts, custom error handling
4. **Consistency**: Matches the parameter list of `handle_request`, making the API more intuitive
5. **Type Safety**: All parameters are properly typed and checked at compile time

## Implementation Notes

- The `convert_response` function is NOT a trait method (removed in v0.22.0)
- It's generated as a static function in the `HasSubcontroller` impl block
- Called as: `<Self as HasSubcontroller<_, Child>>::convert_response(...)`
- Since it's not part of a trait, changing the signature is safe (no trait compatibility concerns)
- All parameters are passed by reference (except `response` which is moved)

## Timeline

- **Planning**: 2025-11-24
- **Implementation**: TBD
- **Testing**: TBD
- **Release**: TBD (next minor version due to breaking change)

## Related Issues

- Issue #16: Added `htmx` parameter to `convert_response` (v0.18.0)
- Issue #22: Removed `convert_response` from trait, inlined in macro (v0.22.0)
