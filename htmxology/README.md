# HTMXOLOGY

A type-safe, full-stack web framework for Rust that brings together the power of [HTMX](https://htmx.org/) and [Axum](https://crates.io/crates/axum) for server-side rendering.

[![Crates.io](https://img.shields.io/crates/v/htmxology.svg)](https://crates.io/crates/htmxology)
[![Documentation](https://docs.rs/htmxology/badge.svg)](https://docs.rs/htmxology)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

## Why HTMXOLOGY?

HTMXOLOGY makes building interactive web applications with HTMX in Rust a joy. It provides:

- **Type-safe routing** - Routes are Rust enums, eliminating typos and broken links
- **Compile-time guarantees** - Know your routes work before running your app
- **Zero-cost abstractions** - Macros generate efficient code with no runtime overhead
- **First-class HTMX support** - Built-in helpers for HTMX attributes and responses
- **Flexible architecture** - Compose controllers for clean, maintainable code

## Quick Start

Add HTMXOLOGY to your `Cargo.toml`:

```toml
[dependencies]
htmxology = { version = "0.18", features = ["full"] }
tokio = { version = "1", features = ["full"] }
```

Create a simple app:

```rust
use htmxology::{Route, Controller, Server};

// Define your routes as an enum
#[derive(Route)]
enum AppRoute {
    #[route(GET, "/")]
    Home,
    #[route(GET, "/about")]
    About,
    #[route(GET, "/users/{id}")]
    User(u32),
}

// Implement a controller
struct AppController;

#[htmxology::async_trait]
impl Controller for AppController {
    type Route = AppRoute;
    type Args = ();
    type Response = Result<axum::response::Response, axum::response::Response>;

    async fn handle_request(
        &self,
        route: AppRoute,
        htmx: htmxology::htmx::Request,
        parts: http::request::Parts,
        server_info: &htmxology::ServerInfo,
    ) -> Self::Response {
        match route {
            AppRoute::Home => Ok(axum::response::Html("<h1>Home</h1>").into_response()),
            AppRoute::About => Ok(axum::response::Html("<h1>About</h1>").into_response()),
            AppRoute::User(id) => {
                Ok(axum::response::Html(format!("<h1>User {}</h1>", id)).into_response())
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::builder("127.0.0.1:3000")
        .await?
        .build()
        .serve(AppController)
        .await?;
    Ok(())
}
```

## Key Features

### Type-Safe Routes

Routes are defined as Rust enums, making your routing logic type-safe and refactor-friendly:

```rust
#[derive(Route)]
enum BlogRoute {
    #[route(GET, "/")]
    Index,

    #[route(GET, "/posts/{id}")]
    Post(u32),

    #[route(POST, "/posts", body("application/x-www-form-urlencoded"))]
    CreatePost(#[body] PostForm),

    #[route(GET, "/search", query)]
    Search(#[query] SearchQuery),
}

// Generate URLs with Display
let url = BlogRoute::Post(42).to_string(); // "/posts/42"

// Use in HTMX templates
let attr = BlogRoute::Search(query).as_htmx_attribute(); // hx-get="/search?q=..."
```

### Composable Controllers

Build modular applications with nested controllers:

```rust
#[derive(RoutingController)]
#[controller(AppRoute)]
#[subcontroller(BlogController, route = Blog, path = "blog/")]
#[subcontroller(ApiController, route = Api, path = "api/")]
struct AppController {
    blog: BlogController,
    api: ApiController,
}
```

### HTMX Integration

First-class support for HTMX features:

```rust
// Access HTMX headers
if htmx.is_htmx_request() {
    // Return partial HTML
} else {
    // Return full page
}

// Generate HTMX attributes from routes
BlogRoute::Post(42).as_htmx_attribute() // "hx-get=\"/posts/42\""

// Type-safe HTML IDs
let id = HtmlId::new("my-element");
```

### Built-in Caching

Add caching to any controller:

```rust
use htmxology::{Cache, CachingControllerExt};

let cached_controller = my_controller.with_cache(Cache::default());
```

### Templating Support

Integrate with [Askama](https://github.com/djc/askama) templates:

```rust
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    title: String,
    items: Vec<Item>,
}

// Use RenderIntoResponse trait
impl Controller for MyController {
    async fn handle_request(&self, ...) -> Self::Response {
        let template = IndexTemplate { ... };
        Ok(template.render_into_response()?)
    }
}
```

## Feature Flags

- `derive` - Enable derive macros (Route, DisplayDelegate, RoutingController)
- `templating` - Askama template integration
- `auto-reload` - Development hot-reload support
- `interfaces` - Network interface detection
- `ws` - WebSocket support
- `full` - Enable all features

## Examples

Check out the [examples directory](https://github.com/ereOn/htmxology.rs/tree/main/examples) for complete applications:

- `blocks` - Component-based UI with HTMX
- `components` - Reusable component patterns

Run an example:

```bash
cargo install just
just example blocks
```

## Documentation

- [API Documentation](https://docs.rs/htmxology)
- [GitHub Repository](https://github.com/ereOn/htmxology.rs)

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](https://github.com/ereOn/htmxology.rs/blob/main/CONTRIBUTING.md) for details.

## License

Licensed under the MIT License. See [LICENSE-MIT](https://github.com/ereOn/htmxology.rs/blob/main/LICENSE-MIT) for details.
