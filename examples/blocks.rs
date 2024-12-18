//! Run with
//!
//! ```not_rust
//! just example blocks
//! ```

use controller::MainController;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    info!("Starting example `{}`...", env!("CARGO_BIN_NAME"));

    // Create a new server with auto-reload enabled by attempting to get a TCP listener from the
    // `listenfd` environment variable, falling back to binding to a local address if that fails.
    let server = htmxology::Server::builder_with_auto_reload("127.0.0.1:3000")
        .await?
        // Set the options on the server from the environment.
        .with_options_from_env()?
        .build();

    server
        .serve(MainController::default())
        .await
        .map_err(Into::into)
}

/// The views.
///
/// This modules contains types whose sole goal is to render HTML and HTTP responses.
///
/// For some of the types, the `Fragment` derive macro is used to implement the `Fragment` trait
/// which eases the rendering of the views in an HTMX context.
mod views {
    use askama::Template;
    use htmxology::{DisplayDelegate, Fragment, Route};

    use crate::controller::AppRoute;

    /// The index page.
    #[derive(Template)]
    #[template(path = "blocks/index.html.jinja")]
    pub(super) struct Index {
        pub menu: Menu,
        pub page: Page,
        pub base_url: http::Uri,
    }

    /// The page fragment.
    ///
    /// This defines a fragment that can contain the top-level pages.
    ///
    /// As the fragment itself must implement `Display`, we do it by using the `DisplayDelegate`
    /// derive macro, which simply calls the `Display` implementation of the inner enum variants.
    ///
    /// The `target` attribute is optional (and by default derived from the type name) and
    /// indicates the HTMX target in which the fragment will be inserted.
    #[derive(Debug, Fragment, DisplayDelegate)]
    #[htmx(target = "#page")]
    pub(super) enum Page {
        /// The dashboard page.
        Dashboard(PageDashboard),
        /// The messages page.
        Messages(PageMessages),
        /// The message detail page.
        MessageDetail(PageMessageDetail),
        /// The settings page.
        Settings(PageSettings),
        /// The advanced settings page.
        AdvancedSettings(PageAdvancedSettings),
    }

    #[derive(Debug, Fragment, Template)]
    #[htmx(target = "#menu")]
    #[template(path = "blocks/menu.html.jinja")]
    pub(super) struct Menu {
        /// The menu.
        pub menu: Vec<MenuItem>,

        /// The active menu item.
        pub active: usize,
    }

    #[derive(Debug)]
    pub(super) struct MenuItem {
        /// The URL of the menu item.
        pub url: AppRoute,

        /// The title of the menu item.
        pub title: String,

        /// The icon.
        pub icon: MenuItemIcon,
    }

    impl From<super::model::MenuItem> for MenuItem {
        fn from(item: super::model::MenuItem) -> Self {
            Self {
                url: item.url,
                title: item.title,
                icon: item.icon.into(),
            }
        }
    }

    #[derive(Debug, Clone, Copy, Template)]
    #[template(path = "blocks/menu_item_icon.html.jinja")]
    pub(super) struct MenuItemIcon {
        icon: super::model::MenuItemIcon,
    }

    impl From<super::model::MenuItemIcon> for MenuItemIcon {
        fn from(icon: super::model::MenuItemIcon) -> Self {
            Self { icon }
        }
    }

    #[derive(Debug, Template)]
    #[template(path = "blocks/page/dashboard.html.jinja")]
    pub(super) struct PageDashboard {}

    #[derive(Debug, Template)]
    #[template(path = "blocks/page/messages.html.jinja")]
    pub(super) struct PageMessages {
        pub messages: Vec<(AppRoute, super::model::Message)>,
    }

    #[derive(Debug, Template)]
    #[template(path = "blocks/page/message_detail.html.jinja")]
    pub(super) struct PageMessageDetail {
        pub message_id: u8,
        pub red: bool,
        pub save_url: AppRoute,
        pub form: crate::controller::MessageSaveBody,
    }

    #[derive(Debug, Template)]
    #[template(path = "blocks/page/settings.html.jinja")]
    pub(super) struct PageSettings {}

    #[derive(Debug, Template)]
    #[template(path = "blocks/page/advanced_settings.html.jinja")]
    pub(super) struct PageAdvancedSettings {}
}

mod model {
    use crate::controller::{AppRoute, SettingsRoute};

    /// The model.
    ///
    /// This can be anything you need, and would typically hold one or many database-access layer
    /// implementations or even in-memory data.
    ///
    /// Go crazy!
    #[derive(Debug)]
    pub struct Model {
        /// The menu.
        pub menu: Menu,

        /// The messages.
        pub messages: Vec<Message>,
    }

    impl Default for Model {
        fn default() -> Self {
            let menu = Menu::default();
            let messages = vec![
                Message {
                    id: 1,
                    title: "Message 1".to_string(),
                    content: "This is message 1.".to_string(),
                },
                Message {
                    id: 2,
                    title: "Message 2".to_string(),
                    content: "This is message 2.".to_string(),
                },
                Message {
                    id: 3,
                    title: "Message 3".to_string(),
                    content: "This is message 3.".to_string(),
                },
            ];

            Self { menu, messages }
        }
    }

    #[derive(Debug, Clone)]
    pub(super) struct Menu {
        /// The menu items.
        pub items: Vec<MenuItem>,
    }

    impl Default for Menu {
        fn default() -> Self {
            let items = vec![
                MenuItem {
                    url: AppRoute::Dashboard,
                    title: "Dashboard".to_string(),
                    icon: MenuItemIcon::Dashboard,
                },
                MenuItem {
                    url: AppRoute::Messages,
                    title: "Messages".to_string(),
                    icon: MenuItemIcon::Messages,
                },
                MenuItem {
                    url: AppRoute::Settings(SettingsRoute::Home),
                    title: "Settings".to_string(),
                    icon: MenuItemIcon::Settings,
                },
                MenuItem {
                    url: AppRoute::Settings(SettingsRoute::Advanced),
                    title: "Advanced settings".to_string(),
                    icon: MenuItemIcon::Settings,
                },
            ];

            Self { items }
        }
    }

    #[derive(Debug, Clone)]
    pub(super) struct MenuItem {
        /// The URL of the menu item.
        pub url: AppRoute,

        /// The title of the menu item.
        pub title: String,

        /// The icon.
        pub icon: MenuItemIcon,
    }

    #[derive(Debug, Clone, Copy)]
    pub(super) enum MenuItemIcon {
        /// The dashboard icon.
        Dashboard,
        /// The messages icon.
        Messages,
        /// The settings icon.
        Settings,
    }

    /// A message.
    #[derive(Debug, Clone)]
    pub struct Message {
        /// The ID of the message.
        pub id: u8,

        /// The title of the message.
        pub title: String,

        /// The content of the message.
        pub content: String,
    }
}

mod controller {
    use std::ops::Deref;
    use std::sync::Arc;

    use super::views;
    use askama_axum::IntoResponse;
    use axum::async_trait;
    use htmxology::{
        htmx::{FragmentExt, Request as HtmxRequest},
        Controller,
    };
    use htmxology::{Route, ServerInfo};
    use serde::{Deserialize, Serialize};
    use tokio::sync::Mutex;

    /// The main application routes.
    #[derive(Debug, Clone, Route)]
    pub enum AppRoute {
        /// The home route.
        #[route("/")]
        Home,

        /// The dashboard route.
        #[route("/dashboard")]
        Dashboard,

        /// The messages route.
        #[route("/messages")]
        Messages,

        /// The message detail route.
        #[route("/messages/{id}")]
        MessageDetail {
            /// The message ID.
            id: u8,

            #[query]
            query: MessageDetailQuery,
        },

        /// The message save route.
        #[route("/messages/{id}/save", method = "POST")]
        MessageSave(
            u8,
            /// The message content.
            #[body]
            MessageSaveBody,
        ),

        /// The settings route.
        #[subroute("/settings")]
        Settings(#[subroute] SettingsRoute),
    }

    /// The message save body.
    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    pub struct MessageSaveBody {
        /// The message title.
        pub title: String,

        /// The message content.
        pub content: String,
    }

    /// The settings application routes.
    #[derive(Debug, Clone, Route)]
    pub enum SettingsRoute {
        /// The general settings route.
        #[route("/")]
        Home,

        /// The advanced settings route.
        #[route("/advanced")]
        Advanced,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct MessageDetailQuery {
        /// Show the message in red.
        red: Option<bool>,
    }

    /// The main controller implementation.
    #[derive(Debug, Clone, Default)]
    pub struct MainController {
        model: Arc<Mutex<super::model::Model>>,
    }

    /// Custom implementation.
    #[async_trait]
    impl Controller for MainController {
        type Route = AppRoute;

        async fn render_view(
            &self,
            route: AppRoute,
            htmx: HtmxRequest,
            server_info: &ServerInfo,
        ) -> axum::response::Response {
            let caching = htmxology::caching::CachingStrategy::default();
            let base_url = server_info.base_url.clone();

            match route {
                AppRoute::Home | AppRoute::Dashboard => {
                    let menu = Self::make_menu(self.model.lock().await.deref(), 0);
                    let page = views::Page::Dashboard(views::PageDashboard {});
                    match htmx {
                        HtmxRequest::Classic => caching.add_caching_headers(views::Index {
                            menu,
                            page,
                            base_url,
                        }),
                        HtmxRequest::Htmx { .. } => {
                            caching.add_caching_headers(page.into_htmx_response().with_oob(menu))
                        }
                    }
                }
                AppRoute::Messages => {
                    let model = self.model.lock().await;
                    let menu = Self::make_menu(model.deref(), 1);
                    let messages = model.messages.clone();
                    let page = views::Page::Messages(views::PageMessages {
                        messages: messages
                            .into_iter()
                            .map(|message| {
                                (
                                    AppRoute::MessageDetail {
                                        id: message.id,
                                        query: MessageDetailQuery { red: Some(true) },
                                    },
                                    message,
                                )
                            })
                            .collect(),
                    });

                    match htmx {
                        HtmxRequest::Classic => caching.add_caching_headers(views::Index {
                            menu,
                            page,
                            base_url,
                        }),
                        HtmxRequest::Htmx { .. } => {
                            caching.add_caching_headers(page.into_htmx_response().with_oob(menu))
                        }
                    }
                }
                AppRoute::MessageDetail { id, query } => {
                    let model = self.model.lock().await;
                    let menu = Self::make_menu(model.deref(), 1);
                    let message = match model
                        .messages
                        .iter()
                        .find(|message| message.id == id)
                        .cloned()
                    {
                        Some(message) => message,
                        None => {
                            return (http::StatusCode::NOT_FOUND, "Message not found".to_string())
                                .into_response();
                        }
                    };

                    let page = views::Page::MessageDetail(views::PageMessageDetail {
                        message_id: message.id,
                        red: query.red.unwrap_or_default(),
                        save_url: AppRoute::MessageSave(id, Default::default()),
                        form: MessageSaveBody {
                            title: message.title,
                            content: message.content,
                        },
                    });

                    match htmx {
                        HtmxRequest::Classic => caching.add_caching_headers(views::Index {
                            menu,
                            page,
                            base_url,
                        }),
                        HtmxRequest::Htmx { .. } => {
                            caching.add_caching_headers(page.into_htmx_response().with_oob(menu))
                        }
                    }
                }
                AppRoute::MessageSave(id, form) => {
                    println!("{id} => {form:#?}");

                    http::StatusCode::NO_CONTENT.into_response()
                }
                AppRoute::Settings(settings) => {
                    let (page, active_idx) = match settings {
                        SettingsRoute::Home => (views::Page::Settings(views::PageSettings {}), 2),
                        SettingsRoute::Advanced => (
                            views::Page::AdvancedSettings(views::PageAdvancedSettings {}),
                            3,
                        ),
                    };

                    let menu = Self::make_menu(self.model.lock().await.deref(), active_idx);

                    match htmx {
                        HtmxRequest::Classic => caching.add_caching_headers(views::Index {
                            menu,
                            page,
                            base_url,
                        }),
                        HtmxRequest::Htmx { .. } => {
                            caching.add_caching_headers(page.into_htmx_response().with_oob(menu))
                        }
                    }
                }
            }
        }
    }

    impl MainController {
        fn make_menu(model: &super::model::Model, active: usize) -> views::Menu {
            views::Menu {
                menu: model
                    .menu
                    .items
                    .clone()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                active,
            }
        }
    }
}
