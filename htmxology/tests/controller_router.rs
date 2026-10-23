//! End-to-end tests of `ControllerRouter`.
//!
//! Requests are driven through the Axum router with `tower::ServiceExt::oneshot`, without a TCP
//! server, which is exactly what `ControllerRouter::into_router` is for.

#![cfg(feature = "derive")]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use htmxology::{Controller, ControllerRouter, Route, ServerInfo, htmx};
use http::{HeaderMap, Request, StatusCode};
use tower::ServiceExt as _;

#[derive(Debug, Clone, Route)]
enum TestRoute {
    #[route("")]
    Home,

    #[route("items/{id}")]
    Item { id: u32 },

    #[route("items", method = "POST")]
    Create {
        #[body]
        form: CreateForm,
    },

    #[route("info")]
    Info,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CreateForm {
    name: String,
}

#[derive(Debug, Clone, Default)]
struct TestController;

impl Controller for TestController {
    type Route = TestRoute;
    type Args = ();
    type Response = Result<axum::response::Response, axum::response::Response>;

    async fn handle_request(
        &self,
        route: TestRoute,
        _htmx: htmx::Request,
        _parts: http::request::Parts,
        server_info: &ServerInfo,
        _args: (),
    ) -> Self::Response {
        Ok(match route {
            TestRoute::Home => Html("home").into_response(),
            TestRoute::Item { id } => Html(format!("item {id}")).into_response(),
            TestRoute::Create { form } => (StatusCode::CREATED, form.name).into_response(),
            TestRoute::Info => server_info.base_url.to_string().into_response(),
        })
    }
}

fn server_info() -> ServerInfo {
    ServerInfo {
        base_url: "https://example.test/".parse().unwrap(),
    }
}

fn router() -> Router {
    ControllerRouter::from(TestController).into_router(server_info())
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, HeaderMap, String) {
    let response = router.oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let body = to_bytes(body, usize::MAX).await.unwrap();

    (
        parts.status,
        parts.headers,
        String::from_utf8(body.to_vec()).unwrap(),
    )
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn form_request(uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .header(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(Body::from(body.to_owned()))
        .unwrap()
}

#[tokio::test]
async fn controller_handles_matching_routes() {
    let (status, _, body) = send(router(), get_request("/")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "home");

    let (status, _, body) = send(router(), get_request("/items/42")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "item 42");
}

#[tokio::test]
async fn unknown_path_is_not_found() {
    let (status, _, _) = send(router(), get_request("/nope")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn wrong_method_is_not_allowed() {
    let (status, _, _) = send(router(), form_request("/", "")).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn invalid_path_parameter_is_bad_request() {
    let (status, _, _) = send(router(), get_request("/items/not-a-number")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn form_body_is_deserialized() {
    let (status, _, body) = send(router(), form_request("/items", "name=widget")).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body, "widget");
}

#[tokio::test]
async fn invalid_form_body_is_unprocessable() {
    let (status, _, _) = send(router(), form_request("/items", "nope=1")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn server_info_is_available_to_the_controller() {
    let (status, _, body) = send(router(), get_request("/info")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "https://example.test/");
}

#[tokio::test]
async fn explicit_args_factory_works() {
    let router = ControllerRouter::new(TestController, |_: &TestController| async {})
        .into_router(server_info());

    let (status, _, body) = send(router, get_request("/")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "home");
}

async fn add_test_header(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("x-test", http::HeaderValue::from_static("1"));
    response
}

#[tokio::test]
async fn extra_routes_and_layers_live_next_to_the_controller() {
    let router = ControllerRouter::from(TestController)
        .route("/health", get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(add_test_header))
        .into_router(server_info());

    let (status, headers, body) = send(router.clone(), get_request("/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
    assert_eq!(headers["x-test"], "1");

    // The layer also wraps the controller fallback.
    let (status, headers, body) = send(router.clone(), get_request("/")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "home");
    assert_eq!(headers["x-test"], "1");

    // Unknown paths still reach the controller and get its 404.
    let (status, _, _) = send(router, get_request("/nope")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn nest_merge_and_map_router_work() {
    let router = ControllerRouter::from(TestController)
        .nest(
            "/nested",
            Router::new().route("/x", get(|| async { "nested" })),
        )
        .merge(Router::new().route("/merged", get(|| async { "merged" })))
        .map_router(|router| router.route("/mapped", get(|| async { "mapped" })))
        .into_router(server_info());

    for (path, expected) in [
        ("/nested/x", "nested"),
        ("/merged", "merged"),
        ("/mapped", "mapped"),
        ("/", "home"),
    ] {
        let (status, _, body) = send(router.clone(), get_request(path)).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(body, expected, "{path}");
    }
}
