//! Test that FromStr can parse Display output for routes with subroutes and catch-all variants

#![cfg(feature = "derive")]

use htmxology::Route;
use std::str::FromStr;

// Subroute test
#[derive(Debug, Clone, PartialEq, Route)]
enum ApiRoute {
    #[route("users")]
    Users,

    #[route("posts/{id}")]
    Post { id: u32 },
}

#[derive(Debug, Clone, PartialEq, Route)]
enum AppRoute {
    #[route("")]
    Home,

    #[route("api/")]
    Api {
        #[subroute]
        route: ApiRoute,
    },
}

#[test]
fn test_subroute_simple() {
    let route = AppRoute::Api {
        route: ApiRoute::Users,
    };
    let route_str = route.to_string();
    assert_eq!(route_str, "/api/users");

    let parsed = AppRoute::from_str(&route_str).unwrap();
    assert_eq!(parsed, route);
}

#[test]
fn test_subroute_with_params() {
    let route = AppRoute::Api {
        route: ApiRoute::Post { id: 123 },
    };
    let route_str = route.to_string();
    assert_eq!(route_str, "/api/posts/123");

    let parsed = AppRoute::from_str(&route_str).unwrap();
    assert_eq!(parsed, route);
}

// Nested subroutes test
#[derive(Debug, Clone, PartialEq, Route)]
enum PostRoute {
    #[route("")]
    Show,

    #[route("edit")]
    Edit,
}

#[derive(Debug, Clone, PartialEq, Route)]
enum BlogRoute {
    #[route("")]
    Index,

    #[route("posts/{post_id}/")]
    Post {
        post_id: u32,
        #[subroute]
        route: PostRoute,
    },
}

#[derive(Debug, Clone, PartialEq, Route)]
enum SiteRoute {
    #[route("")]
    Home,

    #[route("blog/")]
    Blog {
        #[subroute]
        route: BlogRoute,
    },
}

#[test]
fn test_nested_subroute_index() {
    let route = SiteRoute::Blog {
        route: BlogRoute::Index,
    };
    let route_str = route.to_string();
    // BlogRoute::Index has route("") which displays as "/", so combined with "blog/" it's "/blog/"
    assert_eq!(route_str, "/blog/");

    let parsed = SiteRoute::from_str(&route_str).unwrap();
    assert_eq!(parsed, route);
}

#[test]
fn test_nested_subroute_deep() {
    let route = SiteRoute::Blog {
        route: BlogRoute::Post {
            post_id: 42,
            route: PostRoute::Edit,
        },
    };
    let route_str = route.to_string();
    assert_eq!(route_str, "/blog/posts/42/edit");

    let parsed = SiteRoute::from_str(&route_str).unwrap();
    assert_eq!(parsed, route);
}

// Catch-all test
#[derive(Debug, Clone, PartialEq, Route)]
enum NotFoundRoute {
    #[route("")]
    NotFound,

    #[route("{path}")]
    NotFoundWithPath { path: String },
}

#[derive(Debug, Clone, PartialEq, Route)]
enum MainRoute {
    #[route("")]
    Home,

    #[route("about")]
    About,

    #[catch_all]
    NotFound(NotFoundRoute),
}

#[test]
fn test_catch_all_simple() {
    let route = MainRoute::NotFound(NotFoundRoute::NotFound);
    let route_str = route.to_string();
    assert_eq!(route_str, "/");

    // This should parse as Home, not NotFound, since Home is more specific
    let parsed = MainRoute::from_str(&route_str).unwrap();
    assert_eq!(parsed, MainRoute::Home);
}

#[test]
fn test_catch_all_with_path() {
    let route = MainRoute::NotFound(NotFoundRoute::NotFoundWithPath {
        path: "unknown".to_string(),
    });
    let route_str = route.to_string();
    assert_eq!(route_str, "/unknown");

    let parsed = MainRoute::from_str(&route_str).unwrap();
    assert_eq!(parsed, route);
}

#[test]
fn test_main_routes_still_work() {
    let home = MainRoute::Home;
    let home_str = home.to_string();
    assert_eq!(home_str, "/");
    let parsed = MainRoute::from_str(&home_str).unwrap();
    assert_eq!(parsed, home);

    let about = MainRoute::About;
    let about_str = about.to_string();
    assert_eq!(about_str, "/about");
    let parsed = MainRoute::from_str(&about_str).unwrap();
    assert_eq!(parsed, about);
}
