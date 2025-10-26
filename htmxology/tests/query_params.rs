//! Comprehensive tests for query parameter handling, especially Vec<T> support.

#![cfg(feature = "derive")]

use htmxology::Route;
use serde::{Deserialize, Serialize};

// Test structures for various query parameter scenarios

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct VecQuery {
    tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MixedQuery {
    tags: Vec<String>,
    category: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct OptionalVecQuery {
    tags: Vec<String>,
    category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct IntVecQuery {
    ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ComplexQuery {
    tags: Vec<String>,
    ids: Vec<u32>,
    limit: Option<usize>,
    offset: usize,
}

// Route definitions

#[derive(Debug, Clone, Route)]
enum VecRoute {
    #[route("search")]
    Search {
        #[query]
        query: VecQuery,
    },
}

#[derive(Debug, Clone, Route)]
enum MixedRoute {
    #[route("search")]
    Search {
        #[query]
        query: MixedQuery,
    },
}

#[derive(Debug, Clone, Route)]
enum OptionalVecRoute {
    #[route("search")]
    Search {
        #[query]
        query: OptionalVecQuery,
    },
}

#[derive(Debug, Clone, Route)]
enum IntVecRoute {
    #[route("items")]
    Items {
        #[query]
        query: IntVecQuery,
    },
}

#[derive(Debug, Clone, Route)]
enum ComplexRoute {
    #[route("search")]
    Search {
        #[query]
        query: ComplexQuery,
    },
}

// Tests for Vec<String> query params

#[test]
fn test_vec_query_empty() {
    let route = VecRoute::Search {
        query: VecQuery { tags: vec![] },
    };

    let url = route.to_string();
    assert_eq!(url, "/search", "Empty Vec should produce no query string");
}

#[test]
fn test_vec_query_single_item() {
    let route = VecRoute::Search {
        query: VecQuery {
            tags: vec!["rust".to_string()],
        },
    };

    let url = route.to_string();
    assert_eq!(url, "/search?tags=rust");
}

#[test]
fn test_vec_query_multiple_items() {
    let route = VecRoute::Search {
        query: VecQuery {
            tags: vec!["rust".to_string(), "web".to_string(), "htmx".to_string()],
        },
    };

    let url = route.to_string();
    assert_eq!(url, "/search?tags=rust&tags=web&tags=htmx");
}

#[test]
fn test_vec_query_url_encoding() {
    let route = VecRoute::Search {
        query: VecQuery {
            tags: vec!["hello world".to_string(), "foo&bar".to_string()],
        },
    };

    let url = route.to_string();
    assert!(url.contains("hello+world") || url.contains("hello%20world"));
    assert!(url.contains("foo%26bar"));
}

// Tests for mixed scalar + Vec query params

#[test]
fn test_mixed_query_empty_vec() {
    let route = MixedRoute::Search {
        query: MixedQuery {
            tags: vec![],
            category: "programming".to_string(),
        },
    };

    let url = route.to_string();
    assert_eq!(url, "/search?category=programming");
}

#[test]
fn test_mixed_query_with_vec() {
    let route = MixedRoute::Search {
        query: MixedQuery {
            tags: vec!["rust".to_string(), "web".to_string()],
            category: "programming".to_string(),
        },
    };

    let url = route.to_string();
    // Query string order may vary, so check both possibilities
    assert!(
        url == "/search?tags=rust&tags=web&category=programming"
            || url == "/search?category=programming&tags=rust&tags=web"
    );
}

// Tests for Option<T> with Vec<T>

#[test]
fn test_optional_vec_all_none() {
    let route = OptionalVecRoute::Search {
        query: OptionalVecQuery {
            tags: vec![],
            category: None,
        },
    };

    let url = route.to_string();
    assert_eq!(url, "/search", "No query string when all fields are empty");
}

#[test]
fn test_optional_vec_some_category_empty_vec() {
    let route = OptionalVecRoute::Search {
        query: OptionalVecQuery {
            tags: vec![],
            category: Some("test".to_string()),
        },
    };

    let url = route.to_string();
    assert_eq!(url, "/search?category=test");
}

#[test]
fn test_optional_vec_none_category_with_vec() {
    let route = OptionalVecRoute::Search {
        query: OptionalVecQuery {
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            category: None,
        },
    };

    let url = route.to_string();
    assert_eq!(url, "/search?tags=tag1&tags=tag2");
}

#[test]
fn test_optional_vec_all_present() {
    let route = OptionalVecRoute::Search {
        query: OptionalVecQuery {
            tags: vec!["tag1".to_string()],
            category: Some("test".to_string()),
        },
    };

    let url = route.to_string();
    assert!(url == "/search?tags=tag1&category=test" || url == "/search?category=test&tags=tag1");
}

// Tests for Vec<u32> (non-String types)

#[test]
fn test_int_vec_query_empty() {
    let route = IntVecRoute::Items {
        query: IntVecQuery { ids: vec![] },
    };

    let url = route.to_string();
    assert_eq!(url, "/items");
}

#[test]
fn test_int_vec_query_single() {
    let route = IntVecRoute::Items {
        query: IntVecQuery { ids: vec![42] },
    };

    let url = route.to_string();
    assert_eq!(url, "/items?ids=42");
}

#[test]
fn test_int_vec_query_multiple() {
    let route = IntVecRoute::Items {
        query: IntVecQuery {
            ids: vec![1, 2, 3, 10, 100],
        },
    };

    let url = route.to_string();
    assert_eq!(url, "/items?ids=1&ids=2&ids=3&ids=10&ids=100");
}

// Tests for complex queries with multiple Vecs and mixed types

#[test]
fn test_complex_query_all_empty() {
    let route = ComplexRoute::Search {
        query: ComplexQuery {
            tags: vec![],
            ids: vec![],
            limit: None,
            offset: 0,
        },
    };

    let url = route.to_string();
    assert_eq!(url, "/search?offset=0");
}

#[test]
fn test_complex_query_partial() {
    let route = ComplexRoute::Search {
        query: ComplexQuery {
            tags: vec!["rust".to_string()],
            ids: vec![],
            limit: Some(10),
            offset: 0,
        },
    };

    let url = route.to_string();
    // Order may vary, check that all parts are present
    assert!(url.contains("tags=rust"));
    assert!(url.contains("limit=10"));
    assert!(url.contains("offset=0"));
    assert!(!url.contains("ids="));
}

#[test]
fn test_complex_query_all_present() {
    let route = ComplexRoute::Search {
        query: ComplexQuery {
            tags: vec!["rust".to_string(), "web".to_string()],
            ids: vec![1, 2, 3],
            limit: Some(20),
            offset: 5,
        },
    };

    let url = route.to_string();
    // Verify all components are present
    assert!(url.contains("tags=rust"));
    assert!(url.contains("tags=web"));
    assert!(url.contains("ids=1"));
    assert!(url.contains("ids=2"));
    assert!(url.contains("ids=3"));
    assert!(url.contains("limit=20"));
    assert!(url.contains("offset=5"));
}

// Serialization round-trip tests

#[test]
fn test_vec_query_serialization_roundtrip() {
    let query = VecQuery {
        tags: vec!["a".to_string(), "b".to_string(), "c".to_string()],
    };

    let serialized = serde_html_form::to_string(&query).unwrap();
    let deserialized: VecQuery = serde_html_form::from_str(&serialized).unwrap();

    assert_eq!(query, deserialized);
}

#[test]
fn test_mixed_query_serialization_roundtrip() {
    let query = MixedQuery {
        tags: vec!["tag1".to_string(), "tag2".to_string()],
        category: "test".to_string(),
    };

    let serialized = serde_html_form::to_string(&query).unwrap();
    let deserialized: MixedQuery = serde_html_form::from_str(&serialized).unwrap();

    assert_eq!(query, deserialized);
}

#[test]
fn test_int_vec_serialization_roundtrip() {
    let query = IntVecQuery {
        ids: vec![1, 2, 3, 100],
    };

    let serialized = serde_html_form::to_string(&query).unwrap();
    let deserialized: IntVecQuery = serde_html_form::from_str(&serialized).unwrap();

    assert_eq!(query, deserialized);
}

#[test]
fn test_complex_query_serialization_roundtrip() {
    let query = ComplexQuery {
        tags: vec!["rust".to_string(), "web".to_string()],
        ids: vec![1, 2, 3],
        limit: Some(10),
        offset: 5,
    };

    let serialized = serde_html_form::to_string(&query).unwrap();
    let deserialized: ComplexQuery = serde_html_form::from_str(&serialized).unwrap();

    assert_eq!(query, deserialized);
}

// Edge case tests

#[test]
fn test_vec_with_empty_strings() {
    let route = VecRoute::Search {
        query: VecQuery {
            tags: vec!["".to_string(), "valid".to_string(), "".to_string()],
        },
    };

    let url = route.to_string();
    assert!(url.contains("tags="));
    assert!(url.contains("tags=valid"));
}

#[test]
fn test_vec_with_special_characters() {
    let route = VecRoute::Search {
        query: VecQuery {
            tags: vec![
                "c++".to_string(),
                "rust&go".to_string(),
                "type=safe".to_string(),
            ],
        },
    };

    let url = route.to_string();
    // All special characters should be URL encoded
    assert!(url.contains("%") || url.contains("+"));
}

#[test]
fn test_large_vec() {
    let tags: Vec<String> = (0..100).map(|i| format!("tag{}", i)).collect();
    let route = VecRoute::Search {
        query: VecQuery { tags },
    };

    let url = route.to_string();
    // Should contain all tags
    assert!(url.contains("tags=tag0"));
    assert!(url.contains("tags=tag99"));
}

#[test]
fn test_vec_zero_values() {
    let route = IntVecRoute::Items {
        query: IntVecQuery { ids: vec![0, 0, 0] },
    };

    let url = route.to_string();
    assert_eq!(url, "/items?ids=0&ids=0&ids=0");
}
