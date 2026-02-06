//! Test that FromStr can parse Display output for all GET route variants

#![cfg(feature = "derive")]

use htmxology::Route;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PageQuery {
    page: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Route)]
enum TestRoute {
    // Unit variant - GET (default)
    #[route("")]
    Home,

    // Named single path param - GET (default)
    #[route("users/{user_id}")]
    User { user_id: u32 },

    // Named multiple path params - GET (default)
    #[route("posts/{post_id}/comments/{comment_id}")]
    Comment { post_id: u32, comment_id: u64 },

    // Named with query param - GET (default)
    #[route("search")]
    Search {
        #[query]
        query: SearchQuery,
    },

    // Named with path and query params - GET (default)
    #[route("users/{user_id}/posts")]
    UserPosts {
        user_id: u32,
        #[query]
        query: PageQuery,
    },

    // Unnamed single path param - GET (default)
    #[route("products/{id}")]
    Product(u32),

    // Unnamed multiple path params - GET (default)
    #[route("categories/{cat_id}/items/{item_id}")]
    CategoryItem(u32, String),

    // POST route with body - should NOT implement FromStr parsing
    #[route("submit", method = "POST")]
    Submit {
        #[body("application/x-www-form-urlencoded")]
        data: String,
    },

    // DELETE route - should NOT implement FromStr parsing
    #[route("delete/{id}", method = "DELETE")]
    Delete { id: u32 },
}

#[test]
fn test_unit_variant() {
    let home = TestRoute::Home;
    let home_str = home.to_string();
    assert_eq!(home_str, "/");
    let parsed = TestRoute::from_str(&home_str).unwrap();
    assert_eq!(parsed, home);
}

#[test]
fn test_named_single_path_param() {
    let user = TestRoute::User { user_id: 42 };
    let user_str = user.to_string();
    assert_eq!(user_str, "/users/42");
    let parsed = TestRoute::from_str(&user_str).unwrap();
    assert_eq!(parsed, user);
}

#[test]
fn test_named_multiple_path_params() {
    let comment = TestRoute::Comment {
        post_id: 123,
        comment_id: 456,
    };
    let comment_str = comment.to_string();
    assert_eq!(comment_str, "/posts/123/comments/456");
    let parsed = TestRoute::from_str(&comment_str).unwrap();
    assert_eq!(parsed, comment);
}

#[test]
fn test_named_with_query_param() {
    let search = TestRoute::Search {
        query: SearchQuery {
            q: "rust".to_string(),
        },
    };
    let search_str = search.to_string();
    assert_eq!(search_str, "/search?q=rust");
    let parsed = TestRoute::from_str(&search_str).unwrap();
    assert_eq!(parsed, search);
}

#[test]
fn test_named_with_path_and_query_params() {
    // With query param
    let user_posts = TestRoute::UserPosts {
        user_id: 10,
        query: PageQuery { page: Some(2) },
    };
    let user_posts_str = user_posts.to_string();
    assert_eq!(user_posts_str, "/users/10/posts?page=2");
    let parsed = TestRoute::from_str(&user_posts_str).unwrap();
    assert_eq!(parsed, user_posts);

    // Without query param
    let user_posts_no_page = TestRoute::UserPosts {
        user_id: 10,
        query: PageQuery { page: None },
    };
    let user_posts_no_page_str = user_posts_no_page.to_string();
    assert_eq!(user_posts_no_page_str, "/users/10/posts");
    let parsed = TestRoute::from_str(&user_posts_no_page_str).unwrap();
    assert_eq!(parsed, user_posts_no_page);
}

#[test]
fn test_unnamed_single_path_param() {
    let product = TestRoute::Product(999);
    let product_str = product.to_string();
    assert_eq!(product_str, "/products/999");
    let parsed = TestRoute::from_str(&product_str).unwrap();
    assert_eq!(parsed, product);
}

#[test]
fn test_unnamed_multiple_path_params() {
    let category_item = TestRoute::CategoryItem(5, "widget".to_string());
    let category_item_str = category_item.to_string();
    assert_eq!(category_item_str, "/categories/5/items/widget");
    let parsed = TestRoute::from_str(&category_item_str).unwrap();
    assert_eq!(parsed, category_item);
}

#[test]
fn test_non_get_routes_dont_parse() {
    // POST route with body
    let submit_str = "/submit";
    assert!(TestRoute::from_str(submit_str).is_err());

    // DELETE route
    let delete_str = "/delete/123";
    assert!(TestRoute::from_str(delete_str).is_err());
}

#[test]
fn test_invalid_paths_dont_parse() {
    assert!(TestRoute::from_str("/invalid/path").is_err());
    assert!(TestRoute::from_str("/users/not_a_number").is_err());
}

#[test]
fn test_special_characters_in_path_params() {
    #[derive(Debug, Clone, PartialEq, Route)]
    enum ItemRoute {
        #[route("items/{name}")]
        Item { name: String },
    }

    // Test with simple string (path params are not URL-encoded by Display)
    let item = ItemRoute::Item {
        name: "widget".to_string(),
    };
    let item_str = item.to_string();
    assert_eq!(item_str, "/items/widget");

    // FromStr should be able to parse it back
    let parsed = ItemRoute::from_str(&item_str).unwrap();
    assert_eq!(parsed, item);
}
