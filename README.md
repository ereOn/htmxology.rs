# HTMXOLOGY

[![Crates.io](https://img.shields.io/crates/v/htmxology.svg)](https://crates.io/crates/htmxology)
[![Documentation](https://docs.rs/htmxology/badge.svg)](https://docs.rs/htmxology)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/ereOn/htmxology.rs/blob/main/LICENSE-MIT)

A server-side rendering framework for [Rust](https://www.rust-lang.org/), built on
[Axum](https://crates.io/crates/axum) and designed for [HTMX](https://htmx.org/).

HTMXOLOGY assumes your application has a finite set of routes and makes that set a Rust `enum`.
From there, the compiler keeps your URLs, forms, redirects and `hx-*` attributes in sync with
your handlers: a route that renders as a link is the same value that is matched when the request
comes back.

- **Routes are enums.** `#[derive(Route)]` turns an enum into a URL parser, a URL renderer and an
  Axum extractor. Path, query and body parameters are typed fields.
- **Controllers are plain structs.** A `Controller` handles one route enum. Controllers nest, and
  `#[derive(RoutingController)]` writes the dispatch code for you.
- **HTMX is first-class.** Tell full-page loads from HTMX requests, return fragments with
  out-of-band swaps, drive `hx-push-url` / `HX-Redirect` / `HX-Location` from the server.
- **It is still Axum.** Handlers return `axum::response::Response`; templates, caching and
  anything from the Axum ecosystem keep working.

## Installation

```toml
[dependencies]
htmxology = { version = "0.28", features = ["derive", "templating"] }

askama = "0.16"
axum = "0.8"
http = "1"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

The `derive` feature enables the macros. `templating` adds the [Askama](https://askama.readthedocs.io/)
integration used below; it is optional, any way of producing HTML works.

## Quick start

A complete application: a form on `/`, a `POST` that redirects, and a page with a path parameter.

```rust,no_run
use askama::Template;
use axum::response::{Html, IntoResponse};
use htmxology::{Controller, RenderIntoResponse, Route, RouteExt, Server, ServerInfo, htmx};

/// Every route of the application, as one enum.
#[derive(Debug, Clone, Route)]
enum AppRoute {
    /// `GET /`
    #[route("")]
    Home,

    /// `GET /hello/{name}`
    #[route("hello/{name}")]
    Hello { name: String },

    /// `POST /greet`, with a URL-encoded form body.
    #[route("greet", method = "POST")]
    Greet {
        #[body("application/x-www-form-urlencoded")]
        form: GreetForm,
    },
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct GreetForm {
    name: String,
}

/// An Askama template. Values are HTML-escaped for you.
#[derive(Template)]
#[template(source = "<h1>Hello, {{ name }}!</h1>", ext = "html")]
struct HelloPage {
    name: String,
}

#[derive(Debug, Clone)]
struct AppController;

impl Controller for AppController {
    type Route = AppRoute;
    type Args = ();
    type Response = Result<axum::response::Response, axum::response::Response>;

    async fn handle_request(
        &self,
        route: AppRoute,
        _htmx: htmx::Request,
        _parts: http::request::Parts,
        _server_info: &ServerInfo,
        _args: (),
    ) -> Self::Response {
        match route {
            AppRoute::Home => {
                // Routes render as URLs: no hand-written paths, no broken links.
                let action = AppRoute::Greet {
                    form: GreetForm::default(),
                };

                Ok(Html(format!(
                    r#"<form method="post" action="{action}">
                        <input name="name" placeholder="Your name">
                        <button>Greet</button>
                    </form>"#
                ))
                .into_response())
            }
            AppRoute::Hello { name } => Ok(HelloPage { name }.render_into_response()),
            AppRoute::Greet { form } => {
                // `303 See Other` to `/hello/{name}`, with `name` percent-encoded.
                Ok(AppRoute::Hello { name: form.name }.as_redirect_response())
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    Server::builder(listener).build().serve(AppController).await?;

    Ok(())
}
```

Run it and open <http://127.0.0.1:3000/>. Requests that match no route get a `404`; a known
path with the wrong method gets a `405`; a path parameter or query that fails to parse gets a
`400`, and a form body that fails to deserialize gets a `422`.

## Routes

A route is a URL pattern plus an HTTP method plus typed parameters. Paths are written without a
leading `/`; the method defaults to `GET`.

```rust
use htmxology::Route;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct SearchQuery {
    q: String,
    page: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Route)]
enum AdminRoute {
    #[route("")]
    Dashboard,

    #[route("users/{id}", method = "DELETE")]
    DeleteUser(u32),
}

#[derive(Debug, Clone, PartialEq, Route)]
enum BlogRoute {
    /// `GET /`
    #[route("")]
    Index,

    /// `GET /posts/{id}`: a named path parameter.
    #[route("posts/{id}")]
    Post { id: u32 },

    /// `GET /search?q=...&page=...`: query parameters, deserialized with serde.
    #[route("search")]
    Search {
        #[query]
        query: SearchQuery,
    },

    /// `/admin/...`: every `AdminRoute`, nested under a prefix.
    #[route("admin/")]
    Admin(#[subroute] AdminRoute),
}

// Routes render as URLs...
assert_eq!(BlogRoute::Post { id: 42 }.to_string(), "/posts/42");
assert_eq!(
    BlogRoute::Admin(AdminRoute::DeleteUser(7)).to_string(),
    "/admin/users/7"
);

// ...know their HTTP method...
assert_eq!(
    BlogRoute::Admin(AdminRoute::DeleteUser(7)).method(),
    http::Method::DELETE
);

// ...render as HTMX attributes...
assert_eq!(
    BlogRoute::Admin(AdminRoute::DeleteUser(7)).as_htmx_attribute(),
    r#"hx-delete="/admin/users/7""#
);

// ...and parse back from a path (for routes without a body).
let route: BlogRoute = "/search?q=rust&page=2".parse().unwrap();
assert_eq!(
    route,
    BlogRoute::Search {
        query: SearchQuery {
            q: "rust".to_owned(),
            page: Some(2),
        },
    }
);
```

| Attribute | On | Meaning |
|---|---|---|
| `#[route("path/{param}", method = "POST")]` | variant | URL pattern and method. Each `{param}` must match a field (by name for struct variants, by position for tuple variants). |
| `#[route("prefix/")]` + `#[subroute]` | variant + field | A trailing `/` makes the variant a prefix for another `Route` enum held in the marked field. |
| `#[query]` | field | The field is deserialized from the query string (`serde::Deserialize`; `serde::Serialize` to render). |
| `#[body]` | field | The field is the request body, deserialized as a URL-encoded form (`serde::Deserialize`). A content type may be written for documentation, as in `#[body("application/x-www-form-urlencoded")]`, but only forms are supported today. |
| `#[catch_all]` | variant | A fallback variant, tried after every other route. It holds another `Route` enum. |

Path parameters are percent-encoded when a route is rendered and decoded when it is parsed, so
a `String` parameter can hold `/`, `?`, spaces or `<` safely. See
[`design/app_routes.md`](https://github.com/ereOn/htmxology.rs/blob/main/design/app_routes.md)
for the full grammar.

## Controllers

A `Controller` handles every variant of one route enum. It receives the parsed route, the
[HTMX request headers](#htmx), the raw request `Parts`, the server's base URL and a per-request
`Args` value (sessions, database handles, anything you build in the args factory).

Large applications split into subcontrollers, one per route prefix. `#[derive(RoutingController)]`
generates the parent's route enum and its `handle_request` dispatch:

```rust
use axum::response::{Html, IntoResponse};
use htmxology::{Controller, Route, RoutingController, ServerInfo, htmx};

type Response = Result<axum::response::Response, axum::response::Response>;

/// Request-scoped data, handed to every controller.
#[derive(Debug, Clone, Default)]
struct Session {
    user: Option<String>,
}

// A leaf controller.

#[derive(Debug, Clone, Route)]
enum BlogRoute {
    #[route("")]
    Index,

    #[route("posts/{id}")]
    Post { id: u32 },
}

#[derive(Debug, Clone)]
struct BlogController;

impl Controller for BlogController {
    type Route = BlogRoute;
    type Args = Session;
    type Response = Response;

    async fn handle_request(
        &self,
        route: BlogRoute,
        _htmx: htmx::Request,
        _parts: http::request::Parts,
        _server_info: &ServerInfo,
        _session: Session,
    ) -> Response {
        Ok(match route {
            BlogRoute::Index => Html("<h1>Blog</h1>".to_owned()),
            BlogRoute::Post { id } => Html(format!("<h1>Post {id}</h1>")),
        }
        .into_response())
    }
}

// A leaf controller mounted under a parameterized prefix.

#[derive(Debug, Clone, Route)]
enum ProfileRoute {
    #[route("")]
    Show,

    #[route("settings")]
    Settings,
}

/// Built from the parent's `Args` followed by the path parameters, in order.
struct ProfileArgs {
    session: Session,
    user_id: u32,
}

impl From<(Session, u32)> for ProfileArgs {
    fn from((session, user_id): (Session, u32)) -> Self {
        Self { session, user_id }
    }
}

#[derive(Debug, Clone)]
struct ProfileController;

impl Controller for ProfileController {
    type Route = ProfileRoute;
    type Args = ProfileArgs;
    type Response = Response;

    async fn handle_request(
        &self,
        route: ProfileRoute,
        _htmx: htmx::Request,
        _parts: http::request::Parts,
        _server_info: &ServerInfo,
        args: ProfileArgs,
    ) -> Response {
        let _ = (route, args.session);

        Ok(Html(format!("<h1>User {}</h1>", args.user_id)).into_response())
    }
}

// The root controller: it only routes.

#[derive(Debug, Clone, RoutingController)]
#[controller(AppRoute, args = Session)]
#[subcontroller(BlogController, route = Blog, path = "blog/")]
#[subcontroller(
    ProfileController,
    route = Profile,
    path = "users/{user_id}/",
    params(user_id: u32)
)]
struct AppController;

// Subcontrollers are built from the parent on each request.
impl From<&AppController> for BlogController {
    fn from(_: &AppController) -> Self {
        Self
    }
}

impl From<&AppController> for ProfileController {
    fn from(_: &AppController) -> Self {
        Self
    }
}

// The macro generated `AppRoute` with one variant per subcontroller.
let route = AppRoute::Profile {
    user_id: 42,
    subroute: ProfileRoute::Settings,
};
assert_eq!(route.to_string(), "/users/42/settings");
```

A controller is served through a `ControllerRouter`, an Axum router whose fallback is the
controller. Controllers with `Args = ()` (or any `Default` `Args`) convert into one directly;
others need an args factory, called once per request. Anything else your application serves goes
next to the controller, and `into_router` gives you a plain `axum::Router` for tests or for
serving it yourself:

```rust
use axum::routing::get;
use htmxology::{ControllerRouter, ServerInfo};
# use htmxology::{Controller, Route, htmx};
# #[derive(Debug, Clone, Route)]
# enum AppRoute {
#     #[route("")]
#     Home,
# }
# #[derive(Debug, Clone)]
# struct AppController {
#     session: String,
# }
# impl Controller for AppController {
#     type Route = AppRoute;
#     type Args = String;
#     type Response = Result<axum::response::Response, axum::response::Response>;
#     async fn handle_request(
#         &self,
#         _route: AppRoute,
#         _htmx: htmx::Request,
#         _parts: http::request::Parts,
#         _server_info: &ServerInfo,
#         _args: String,
#     ) -> Self::Response {
#         Ok(axum::response::Html("ok").into_response())
#     }
# }
# use axum::response::IntoResponse;
# let controller = AppController { session: "s".to_owned() };

async fn log_requests(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    println!("{} {}", request.method(), request.uri());
    next.run(request).await
}

let router = ControllerRouter::new(controller, |c: &AppController| {
    let session = c.session.clone();
    async move { session }
})
.route("/health", get(|| async { "ok" }))
.layer(axum::middleware::from_fn(log_requests));

// `server.serve(router)` would do this with the base URL it determined.
let _app: axum::Router = router.into_router(ServerInfo {
    base_url: "https://example.com/".parse().unwrap(),
});
```

`#[controller(...)]` also accepts `pre_handler = "Self::func"` (an async hook that runs before
dispatch and can short-circuit with a response, for authentication or rate limiting) and
`extra_derives = (PartialEq, Eq)` for the generated enum. `#[subcontroller(...)]` accepts
`convert_with = "func"` to build the subcontroller from something other than `From<&Parent>`,
and `convert_response = "func"` to map a subcontroller's `Response` type into the parent's.

## HTMX

`htmx::Request` is an Axum extractor for the `HX-*` request headers. Match on it to decide
between a full document and a fragment:

```rust
use std::fmt::Display;

use axum::response::{Html, IntoResponse};
use htmxology::htmx::{Request, ResponseExt};
use htmxology::{Fragment, Identity};

/// A fragment that can be swapped out-of-band: it knows its own `id` and swap strategy.
#[derive(Identity, Fragment)]
#[identity(id = "flash")]
#[fragment(strategy = "innerHTML")]
struct Flash(&'static str);

impl Display for Flash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<div {}>{}</div>", self.id_attribute(), self.0)
    }
}

fn page(htmx: &Request, content: &'static str) -> axum::response::Response {
    match htmx {
        // A regular navigation: send the whole document.
        Request::Classic => Html(format!(
            r#"<html><body><div id="flash"></div><main>{content}</main></body></html>"#
        ))
        .into_response(),

        // An HTMX request: send the fragment, plus an out-of-band update of `#flash`.
        Request::Htmx { .. } => content
            .into_htmx_response()
            .with_oob(Flash("Saved!"))
            .into_response(),
    }
}

assert_eq!(page(&Request::Classic, "<p>Hello</p>").status(), 200);
```

`htmx::Response` wraps any `Display` body and adds the HTMX response headers: `with_push_url`,
`with_redirect` (`HX-Redirect`), `with_location` (`HX-Location`), `with_retarget` and
`with_header`. Out-of-band fragments get their `hx-swap-oob` attribute injected into their root
element, so templates stay free of HTMX plumbing.

`Identity`, `Fragment` and `Named` are small traits for HTML elements that carry an `id`, a swap
strategy or a form `name`; `HtmlId` and `HtmlName` validate those values. `htmx::ResultExt` and
`htmx::OptionExt` turn errors and missing values into HTTP error responses
(`.ok_or_not_found("no such post")`).

## Templates

With the `templating` feature, any Askama template gets `RenderIntoResponse`:

```rust
use askama::Template;
use htmxology::RenderIntoResponse;

#[derive(Template)]
#[template(source = "<ul>{% for item in items %}<li>{{ item }}</li>{% endfor %}</ul>", ext = "html")]
struct List {
    items: Vec<String>,
}

let response = List {
    items: vec!["a".to_owned(), "b".to_owned()],
}
.render_into_response();

assert_eq!(response.status(), 200);
assert_eq!(response.headers()["content-type"], "text/html; charset=utf-8");
```

A template that fails to render is logged and answered with a `500`. `#[derive(DisplayDelegate)]`
implements `Display` for an enum by delegating to its variants, which is handy for a `Page` enum
whose variants are templates.

## Caching

`CachingControllerExt::with_cache` wraps a controller so that `GET`/`HEAD` responses get an
`ETag` and `If-None-Match` requests get a `304`:

```rust
use std::time::Duration;

use htmxology::{Cache, CachingControllerExt};
# use htmxology::{Controller, Route, ServerInfo, htmx};
# #[derive(Debug, Clone, Route)]
# enum AppRoute {
#     #[route("")]
#     Home,
# }
# #[derive(Debug, Clone, Default)]
# struct AppController;
# impl Controller for AppController {
#     type Route = AppRoute;
#     type Args = ();
#     type Response = Result<axum::response::Response, axum::response::Response>;
#     async fn handle_request(
#         &self,
#         _route: AppRoute,
#         _htmx: htmx::Request,
#         _parts: http::request::Parts,
#         _server_info: &ServerInfo,
#         _args: (),
#     ) -> Self::Response {
#         Ok(axum::response::Html("ok").into_response())
#     }
# }
# use axum::response::IntoResponse;

let controller = AppController.with_cache(
    Cache::default()
        .with_cache_duration(Duration::from_secs(300))
        .with_max_body_size(512 * 1024),
);

// The wrapper is a controller like any other: `server.serve(controller)` works.
let _router: htmxology::ControllerRouter = controller.into();
```

Responses can also carry cache directives directly. `with_caching` sets a private `max-age` and
extends `Vary` with the HTMX headers, so a cached fragment is never served in place of a full
page:

```rust
use std::time::Duration;

use axum::response::IntoResponse;
use htmxology::CachingResponseExt;

let response = axum::response::Html("<p>ok</p>")
    .into_response()
    .with_caching(Duration::from_secs(60));

assert_eq!(
    response.headers()["cache-control"],
    "private, max-age=60, must-revalidate"
);
assert!(response.headers()["vary"].to_str().unwrap().contains("hx-request"));

let response = axum::response::Html("<p>ok</p>")
    .into_response()
    .with_caching_disabled();

assert_eq!(response.headers()["cache-control"], "no-cache");
```

## Development server

With the `auto-reload` feature, `Server::builder_with_auto_reload` picks up a listening socket
passed by [`systemfd`](https://crates.io/crates/systemfd) (falling back to binding the given
address) and shuts down gracefully on `ctrl-c`. Combined with a file watcher, the socket survives
recompilations, so the browser never sees a connection refused:

```rust,no_run
# use htmxology::{Controller, Route, ServerInfo, htmx};
# #[derive(Debug, Clone, Route)]
# enum AppRoute {
#     #[route("")]
#     Home,
# }
# #[derive(Debug, Clone, Default)]
# struct AppController;
# impl Controller for AppController {
#     type Route = AppRoute;
#     type Args = ();
#     type Response = Result<axum::response::Response, axum::response::Response>;
#     async fn handle_request(
#         &self,
#         _route: AppRoute,
#         _htmx: htmx::Request,
#         _parts: http::request::Parts,
#         _server_info: &ServerInfo,
#         _args: (),
#     ) -> Self::Response {
#         Ok(axum::response::Html("ok").into_response())
#     }
# }
# use axum::response::IntoResponse;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = htmxology::Server::builder_with_auto_reload("127.0.0.1:3000")
        .await?
        // Reads `HTMXOLOGY_BASE_URL`.
        .with_options_from_env()?
        .build();

    server.serve(AppController).await?;

    Ok(())
}
```

```sh
cargo install systemfd cargo-watch
systemfd --no-pid -s tcp::3000 -- cargo watch -x run
```

`HTMXOLOGY_BASE_URL` sets the public base URL (`ServerInfo::base_url`, used for absolute links
behind a reverse proxy). When unset, it is guessed from the listening address; with the
`interfaces` feature the guess uses the default network interface when listening on `0.0.0.0`.

In production, build the server from your own `TcpListener` and pass a shutdown future with
`with_graceful_shutdown`.

## Feature flags

| Feature | Enables |
|---|---|
| `derive` | The `Route`, `RoutingController`, `Identity`, `Fragment`, `Named` and `DisplayDelegate` macros. You want this. |
| `templating` | `RenderIntoResponse` for Askama templates. |
| `auto-reload` | `Server::builder_with_auto_reload` and `listenfd` socket inheritance. |
| `interfaces` | Base URL guessing from network interfaces. |
| `ws` | Axum's WebSocket support. HTMXOLOGY has no WebSocket helpers of its own yet. |
| `full` | All of the above. |

No feature is enabled by default.

## Examples

The [`examples`](https://github.com/ereOn/htmxology.rs/tree/main/examples) directory has two
runnable applications:

- `blocks`: a dashboard with Askama templates, a boosted menu updated out-of-band, forms and
  response caching.
- `components`: nested controllers, path parameters, per-request session `Args` and a form
  that saves through a `POST` + redirect.

```sh
cargo install just
just dev-setup
just example blocks   # http://localhost:3000
```

## Documentation

- [API reference](https://docs.rs/htmxology)
- [Route grammar](https://github.com/ereOn/htmxology.rs/blob/main/design/app_routes.md)
- [Changelog](https://github.com/ereOn/htmxology.rs/blob/main/CHANGELOG.md)

## Contributing

See [CONTRIBUTING.md](https://github.com/ereOn/htmxology.rs/blob/main/CONTRIBUTING.md).

## License

MIT. See [LICENSE-MIT](https://github.com/ereOn/htmxology.rs/blob/main/LICENSE-MIT).
