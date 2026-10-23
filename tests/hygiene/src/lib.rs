//! Macro hygiene check.
//!
//! Every derive macro of `htmxology` is used here, with every attribute that changes the code it
//! generates. This crate depends on `htmxology`, `axum`, `http` and `serde` only, so it compiles
//! only if the generated code reaches `regex`, `serde_html_form`, `axum-extra` and friends through
//! `htmxology::__private` rather than by bare crate name.

use std::fmt::Display;

use axum::response::{Html, IntoResponse};
use htmxology::{
    Controller, DisplayDelegate, Fragment, Identity, Named, Route, RoutingController, ServerInfo,
    htmx,
};

type Response = Result<axum::response::Response, axum::response::Response>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ItemForm {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Route)]
pub enum AdminRoute {
    #[route("")]
    Dashboard,

    #[route("users/{id}", method = "DELETE")]
    DeleteUser(u32),
}

#[derive(Debug, Clone, PartialEq, Route)]
pub enum NotFoundRoute {
    #[route("")]
    Root,

    #[route("{path}")]
    Path { path: String },
}

/// Every kind of variant the `Route` derive supports.
#[derive(Debug, Clone, PartialEq, Route)]
pub enum BlogRoute {
    #[route("")]
    Index,

    #[route("posts/{id}")]
    Post { id: u32 },

    #[route("tags/{tag}/{page}")]
    Tag(String, u32),

    #[route("search")]
    Search {
        #[query]
        query: SearchQuery,
    },

    #[route("items", method = "POST")]
    CreateItem {
        #[body]
        form: ItemForm,
    },

    #[route("api/items", method = "POST")]
    CreateItemJson(#[body("application/json")] ItemForm),

    #[route("admin/")]
    Admin(#[subroute] AdminRoute),

    #[catch_all]
    NotFound(NotFoundRoute),
}

#[derive(Identity, Fragment)]
#[identity(id = "flash")]
#[fragment(strategy = "innerHTML")]
pub struct Flash(pub &'static str);

impl Display for Flash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<div {}>{}</div>", self.id_attribute(), self.0)
    }
}

#[derive(Named)]
#[named(name = "email")]
pub struct EmailField(pub &'static str);

impl Display for EmailField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<input {} value=\"{}\">", self.name_attribute(), self.0)
    }
}

#[derive(DisplayDelegate)]
pub enum Fragments {
    Flash(Flash),
    Email(EmailField),
}

#[derive(Debug, Clone, Default)]
pub struct Session;

#[derive(Debug, Clone)]
pub struct BlogController;

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
        Ok(Html(format!("<p>{route}</p>")).into_response())
    }
}

pub struct ProfileArgs {
    pub session: Session,
    pub user_id: u32,
}

impl From<(Session, u32)> for ProfileArgs {
    fn from((session, user_id): (Session, u32)) -> Self {
        Self { session, user_id }
    }
}

#[derive(Debug, Clone)]
pub struct ProfileController;

impl Controller for ProfileController {
    type Route = AdminRoute;
    type Args = ProfileArgs;
    type Response = Response;

    async fn handle_request(
        &self,
        _route: AdminRoute,
        _htmx: htmx::Request,
        _parts: http::request::Parts,
        _server_info: &ServerInfo,
        args: ProfileArgs,
    ) -> Response {
        let _ = args.session;
        Ok(Html(format!("<p>{}</p>", args.user_id)).into_response())
    }
}

#[derive(Debug, Clone, RoutingController)]
#[controller(AppRoute, args = Session, pre_handler = "Self::pre_handle")]
#[subcontroller(BlogController, route = Blog, path = "blog/")]
#[subcontroller(
    ProfileController,
    route = Profile,
    path = "users/{user_id}/",
    params(user_id: u32)
)]
pub struct AppController;

impl AppController {
    async fn pre_handle(
        &self,
        _route: &AppRoute,
        _htmx: &htmx::Request,
        _parts: &http::request::Parts,
        _server_info: &ServerInfo,
        _session: &mut Session,
    ) -> Option<Response> {
        None
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_render_and_parse() {
        let route = BlogRoute::Search {
            query: SearchQuery { q: "rust".into() },
        };
        assert_eq!(route.to_string(), "/search?q=rust");
        assert_eq!("/search?q=rust".parse::<BlogRoute>().unwrap(), route);

        assert_eq!(
            BlogRoute::Admin(AdminRoute::DeleteUser(7)).to_string(),
            "/admin/users/7"
        );
        assert_eq!(
            BlogRoute::Admin(AdminRoute::DeleteUser(7)).method(),
            http::Method::DELETE
        );

        let route = AppRoute::Profile {
            user_id: 42,
            subroute: AdminRoute::Dashboard,
        };
        assert_eq!(route.to_string(), "/users/42/");
    }

    #[test]
    fn html_derives_work() {
        assert_eq!(Flash("hi").id().to_string(), "flash");
        assert_eq!(EmailField("a@b").name().to_string(), "email");
        assert_eq!(
            Fragments::Flash(Flash("hi")).to_string(),
            r#"<div id="flash">hi</div>"#
        );
    }
}
