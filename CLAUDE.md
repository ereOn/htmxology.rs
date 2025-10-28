# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

HTMXOLOGY is a Server Side Rendering (SSR) framework written in Rust using HTMX and Axum. It provides a type-safe way to build web applications with server-side rendering, leveraging HTMX for dynamic interactions.

## Development Setup

### Prerequisites

Install required tools:
```bash
cargo install just bacon systemfd cargo-deny
```

Or run the automated setup:
```bash
just dev-setup
```

This installs:
- `just` - Command runner
- `bacon` - Background code checker with live reloading
- `systemfd` - Socket activation for hot-reload during development
- `cargo-deny` - License and security checker

### Common Commands

Build the project:
```bash
just build
# or
cargo build --all-targets --all-features
```

Run all checks (build, format, tests, licenses):
```bash
just check
```

Check licenses and security:
```bash
just deny
```

Run a specific example:
```bash
just example <example-name>
# e.g., just example blocks
# e.g., just example components
```

Build documentation:
```bash
just doc
# Uses bacon for live-reloading documentation
```

Run tests:
```bash
cargo test
```

Run a specific test:
```bash
cargo test <test-name>
```

Run tests for a specific package:
```bash
cargo test -p htmxology
cargo test -p htmxology-macros
```

### Code Formatting

**IMPORTANT**: Always run `cargo fmt` after creating or modifying files to ensure consistent code formatting.

```bash
# Format all code in the workspace
cargo fmt --all

# Check formatting without modifying files
cargo fmt --all -- --check
```

This should be done:
- After creating new files
- After making significant changes to existing files
- Before running tests to verify changes
- Before committing code

## Architecture

### Workspace Structure

This is a Cargo workspace with two main packages:

1. **`htmxology`** - The main framework library
2. **`htmxology-macros`** - Procedural macros (`Route`, `DisplayDelegate`, `ComponentsController`)

### Core Concepts

#### Routes (`Route` trait and derive macro)

Routes are represented as Rust enums with variants for each endpoint. The `#[derive(Route)]` macro generates routing logic automatically.

Key characteristics:
- Routes combine URL paths, HTTP methods, and parameters
- Path parameters can be positional (tuple variants) or named (struct variants)
- Query parameters use `#[query]` attribute
- Request bodies use `#[body("content-type")]` attribute
- Sub-routes use `#[subroute]` for nested routing hierarchies
- Catch-all routes can handle wildcard paths
- Routes implement `Display` to render URLs and `method()` to expose HTTP method
- Helper methods: `as_htmx_attribute()`, `to_absolute_url()`, `as_redirect_response()`

See `design/app_routes.md` for comprehensive routing examples.

#### Controllers (`Controller` trait)

Controllers handle requests for specific routes. They:
- Are associated with a `Route` type and an `Args` type
- The `Args` associated type specifies what parameters are needed to construct the controller
  - Set to `()` for controllers that don't need construction parameters
  - Use tuple types like `(u32,)` or `(u32, String)` for parameterized controllers
- Implement `handle_request()` to process incoming requests
- Receive HTMX request context, HTTP parts, and server info
- Can be composed using the `AsComponent` trait for sub-components
- Use `ControllerExt::get_component()` to access sub-controllers without parameters
- Use `ControllerExt::get_component_with(args)` for parameterized sub-controllers

The `#[derive(ComponentsController)]` macro helps implement component relationships.

**Parameterized Routes**: The `ComponentsController` macro supports path parameters:
```rust
#[derive(ComponentsController)]
#[controller(AppRoute)]
#[component(BlogController, route = Blog, path = "blog/{blog_id}/", params(blog_id: u32))]
#[component(PostController, route = Post, path = "blog/{blog_id}/post/{post_id}/", params(blog_id: u32, post_id: String))]
struct AppController {
    state: AppState,
}
```

This generates route variants with typed parameters and automatically extracts them for sub-controller construction:
- Path parameters are declared with `params(name: Type, ...)`
- Parameters are extracted from the URL and passed to the sub-controller via `get_component_with(tuple)`
- Use `convert_with` to specify a custom function that accepts the parameters

#### Caching

The framework provides built-in caching via:
- `Cache` - Storage for cacheable content
- `CachingController` - Wrapper that adds caching to any controller
- `CachingControllerExt::with_cache()` - Extension method to enable caching
- `CacheControl` and `CachingResponseExt` - For cache control headers

#### Server Setup

Servers are built using a builder pattern:
```rust
htmxology::Server::builder_with_auto_reload("127.0.0.1:3000")
    .await?
    .with_options_from_env()?
    .with_ctrl_c_graceful_shutdown()
    .build()
    .serve(controller)
    .await
```

Environment variables:
- `HTMXOLOGY_BASE_URL` - Base URL for the server (default: "http://localhost:3000")
- `SYSTEMFD_LISTEN_ADDR` - Socket address for systemfd (default: "tcp::3000")

### Feature Flags

- `auto-reload` - Development auto-reload using systemfd/listenfd
- `interfaces` - Network interface detection for base URL guessing
- `ws` - WebSocket support
- `derive` - Enable derive macros (`Route`, `DisplayDelegate`, `ComponentsController`)
- `templating` - Askama template integration via `RenderIntoResponse`
- `full` - Enables all features
- `examples` - Additional dependencies for running examples

### Templating

When using the `templating` feature:
- Use Askama templates (Jinja-style syntax)
- Templates are placed in a `templates/` directory
- The `RenderIntoResponse` trait converts templates to HTTP responses
- The `DisplayDelegate` derive macro simplifies enum variant rendering

### HTMX Integration

The framework provides HTMX-specific helpers:
- `htmx::Request` - HTMX request headers and context
- `Route::as_htmx_attribute()` - Generate HTMX attributes (e.g., `hx-get="/path"`)
- `HtmlId` and `Identity` - Type-safe HTML element IDs
- Path manipulation via `replace_request_path()` and `decode_path_argument()`

## Development Workflow

When running examples with `just example <name>`:
- Uses `systemfd` for socket activation (allows recompilation without losing connections)
- Uses `bacon` for live-reloading on file changes
- Server automatically restarts on code changes during development

## Publishing

```bash
just publish
```

This publishes both packages in order: `htmxology-macros` first, then `htmxology`.

## License Checking

The project uses `cargo-deny` to ensure all dependencies use permissive licenses that are safe for commercial use.

### Running License Checks

```bash
# Using just
just deny

# Or directly with cargo
cargo deny check
```

This validates:
- **Licenses**: Only permissive licenses are allowed (MIT, Apache-2.0, BSD, ISC, MPL-2.0)
- **Security Advisories**: Checks for known vulnerabilities in dependencies
- **Bans**: Warns about duplicate dependency versions
- **Sources**: Ensures dependencies come from trusted sources (crates.io)

### Allowed Licenses

The following licenses are permitted (see `deny.toml`):
- **MIT** - Most permissive
- **Apache-2.0** - Permissive with patent grant
- **BSD-3-Clause** - Permissive
- **ISC** - Similar to MIT
- **Unicode-3.0** - For Unicode data tables
- **MPL-2.0** - Weak copyleft (safe for commercial use)
  - Only requires source disclosure for modified MPL-licensed files
  - Does not force your entire application to be open source
  - Used by the `scraper` crate for HTML parsing

**Note**: GPL, LGPL, and AGPL licenses are explicitly denied as they require derivative works to be open source.

**Duplicate Dependencies**: The configuration allows multiple versions of `windows-sys` as these are pulled in by different dependencies in the tokio ecosystem and are automatically resolved.

### Configuration

License and security policies are configured in `deny.toml` at the repository root.
