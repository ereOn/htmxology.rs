//! Integration tests for FromRequest implementation with Vec query parameters.

#![cfg(feature = "derive")]

use axum::extract::FromRequest;
use htmxology::Route;
use http::{Request, StatusCode};
use serde::{Deserialize, Serialize};

// Test query structures

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
struct SimpleVecQuery {
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MixedQuery {
    #[serde(default)]
    tags: Vec<String>,
    category: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
struct NumericVecQuery {
    #[serde(default)]
    ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
struct OptionalFieldsQuery {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    limit: Option<usize>,
}

// Route definitions

#[derive(Debug, Clone, PartialEq, Route)]
enum SimpleVecRoute {
    #[route("search")]
    Search {
        #[query]
        query: SimpleVecQuery,
    },
}

#[derive(Debug, Clone, PartialEq, Route)]
enum MixedQueryRoute {
    #[route("search")]
    Search {
        #[query]
        query: MixedQuery,
    },
}

#[derive(Debug, Clone, PartialEq, Route)]
enum NumericRoute {
    #[route("items")]
    Items {
        #[query]
        query: NumericVecQuery,
    },
}

#[derive(Debug, Clone, PartialEq, Route)]
enum OptionalRoute {
    #[route("search")]
    Search {
        #[query]
        query: OptionalFieldsQuery,
    },
}

// Helper function to create a request with query string

fn make_request(path: &str, query: &str) -> Request<axum::body::Body> {
    let uri = if query.is_empty() {
        path.to_string()
    } else {
        format!("{}?{}", path, query)
    };

    Request::builder()
        .uri(uri)
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap()
}

// Tests for simple Vec<String> deserialization

#[tokio::test]
async fn test_parse_empty_vec() {
    let request = make_request("/search", "");
    let state = ();

    let result = SimpleVecRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        SimpleVecRoute::Search { query } => {
            assert_eq!(query.tags, Vec::<String>::new());
        }
    }
}

#[tokio::test]
async fn test_parse_single_tag() {
    let request = make_request("/search", "tags=rust");
    let state = ();

    let result = SimpleVecRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        SimpleVecRoute::Search { query } => {
            assert_eq!(query.tags, vec!["rust"]);
        }
    }
}

#[tokio::test]
async fn test_parse_multiple_tags() {
    let request = make_request("/search", "tags=rust&tags=web&tags=htmx");
    let state = ();

    let result = SimpleVecRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        SimpleVecRoute::Search { query } => {
            assert_eq!(query.tags, vec!["rust", "web", "htmx"]);
        }
    }
}

#[tokio::test]
async fn test_parse_url_encoded_tags() {
    let request = make_request("/search", "tags=hello+world&tags=foo%26bar");
    let state = ();

    let result = SimpleVecRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        SimpleVecRoute::Search { query } => {
            assert_eq!(query.tags, vec!["hello world", "foo&bar"]);
        }
    }
}

// Tests for mixed scalar + Vec deserialization

#[tokio::test]
async fn test_parse_mixed_query() {
    let request = make_request("/search", "tags=rust&tags=web&category=programming");
    let state = ();

    let result = MixedQueryRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        MixedQueryRoute::Search { query } => {
            assert_eq!(query.tags, vec!["rust", "web"]);
            assert_eq!(query.category, "programming");
        }
    }
}

#[tokio::test]
async fn test_parse_mixed_empty_vec() {
    let request = make_request("/search", "category=programming");
    let state = ();

    let result = MixedQueryRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        MixedQueryRoute::Search { query } => {
            assert_eq!(query.tags, Vec::<String>::new());
            assert_eq!(query.category, "programming");
        }
    }
}

#[tokio::test]
async fn test_parse_mixed_missing_required_field() {
    let request = make_request("/search", "tags=rust");
    let state = ();

    let result = MixedQueryRoute::from_request(request, &state).await;

    // Should fail because category is required
    assert!(result.is_err());
}

// Tests for Vec<u32> deserialization

#[tokio::test]
async fn test_parse_numeric_vec_empty() {
    let request = make_request("/items", "");
    let state = ();

    let result = NumericRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        NumericRoute::Items { query } => {
            assert_eq!(query.ids, Vec::<u32>::new());
        }
    }
}

#[tokio::test]
async fn test_parse_numeric_vec_single() {
    let request = make_request("/items", "ids=42");
    let state = ();

    let result = NumericRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        NumericRoute::Items { query } => {
            assert_eq!(query.ids, vec![42]);
        }
    }
}

#[tokio::test]
async fn test_parse_numeric_vec_multiple() {
    let request = make_request("/items", "ids=1&ids=2&ids=3&ids=100");
    let state = ();

    let result = NumericRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        NumericRoute::Items { query } => {
            assert_eq!(query.ids, vec![1, 2, 3, 100]);
        }
    }
}

#[tokio::test]
async fn test_parse_numeric_vec_invalid() {
    let request = make_request("/items", "ids=1&ids=not_a_number&ids=3");
    let state = ();

    let result = NumericRoute::from_request(request, &state).await;

    // Should fail due to invalid number
    assert!(result.is_err());
}

// Tests for optional fields with Vec

#[tokio::test]
async fn test_parse_optional_all_empty() {
    let request = make_request("/search", "");
    let state = ();

    let result = OptionalRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        OptionalRoute::Search { query } => {
            assert_eq!(query.tags, Vec::<String>::new());
            assert_eq!(query.limit, None);
        }
    }
}

#[tokio::test]
async fn test_parse_optional_vec_only() {
    let request = make_request("/search", "tags=a&tags=b");
    let state = ();

    let result = OptionalRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        OptionalRoute::Search { query } => {
            assert_eq!(query.tags, vec!["a", "b"]);
            assert_eq!(query.limit, None);
        }
    }
}

#[tokio::test]
async fn test_parse_optional_limit_only() {
    let request = make_request("/search", "limit=10");
    let state = ();

    let result = OptionalRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        OptionalRoute::Search { query } => {
            assert_eq!(query.tags, Vec::<String>::new());
            assert_eq!(query.limit, Some(10));
        }
    }
}

#[tokio::test]
async fn test_parse_optional_all_present() {
    let request = make_request("/search", "tags=x&tags=y&limit=20");
    let state = ();

    let result = OptionalRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        OptionalRoute::Search { query } => {
            assert_eq!(query.tags, vec!["x", "y"]);
            assert_eq!(query.limit, Some(20));
        }
    }
}

// Edge case tests

#[tokio::test]
async fn test_parse_empty_string_values() {
    let request = make_request("/search", "tags=&tags=valid&tags=");
    let state = ();

    let result = SimpleVecRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        SimpleVecRoute::Search { query } => {
            assert_eq!(query.tags, vec!["", "valid", ""]);
        }
    }
}

#[tokio::test]
async fn test_parse_duplicate_params_order_preserved() {
    let request = make_request("/search", "tags=first&tags=second&tags=third");
    let state = ();

    let result = SimpleVecRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        SimpleVecRoute::Search { query } => {
            assert_eq!(query.tags, vec!["first", "second", "third"]);
        }
    }
}

#[tokio::test]
async fn test_parse_numeric_vec_zero() {
    let request = make_request("/items", "ids=0&ids=0&ids=0");
    let state = ();

    let result = NumericRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        NumericRoute::Items { query } => {
            assert_eq!(query.ids, vec![0, 0, 0]);
        }
    }
}

#[tokio::test]
async fn test_parse_large_vec() {
    let params: Vec<String> = (0..100).map(|i| format!("ids={}", i)).collect();
    let query_string = params.join("&");
    let request = make_request("/items", &query_string);
    let state = ();

    let result = NumericRoute::from_request(request, &state).await;

    assert!(result.is_ok());
    let route = result.unwrap();
    match route {
        NumericRoute::Items { query } => {
            assert_eq!(query.ids.len(), 100);
            assert_eq!(query.ids[0], 0);
            assert_eq!(query.ids[99], 99);
        }
    }
}

// Test wrong path returns 404

#[tokio::test]
async fn test_wrong_path_returns_404() {
    let request = make_request("/wrong", "tags=test");
    let state = ();

    let result = SimpleVecRoute::from_request(request, &state).await;

    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Test wrong method returns 405

#[tokio::test]
async fn test_wrong_method_returns_405() {
    let request = Request::builder()
        .uri("/search?tags=test")
        .method("POST")
        .body(axum::body::Body::empty())
        .unwrap();
    let state = ();

    let result = SimpleVecRoute::from_request(request, &state).await;

    assert!(result.is_err());
    let response = result.unwrap_err();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
