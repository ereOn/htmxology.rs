//! Run with
//!
//! ```not_rust
//! just example blocks
//! ```

use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    info!("Starting example `{}`...", env!("CARGO_BIN_NAME"));

    // Create a new user state.
    //
    // This can be any type that you want to store in the server state.
    //
    // It just has to be `Send + Sync + 'static`.
    let model: Arc<Mutex<model::Model>> = Arc::default();

    // Create a new server with auto-reload enabled by attempting to get a TCP listener from the
    // `listenfd` environment variable, falling back to binding to a local address if that fails.
    let server = htmx_ssr::Server::new_with_auto_reload("127.0.0.1:3000", model)
        .await?
        // Set the options on the server from the environment.
        .with_options_from_env()?;

    // Register the main controller as a controller.
    //
    // This registers the routes in the server with handlers that render the views.
    let server = server.with_controller::<controller::MainController>();

    server.serve().await.map_err(Into::into)
}

/// The views.
///
/// This modules contains types whose sole goal is to render HTML and HTTP responses.
///
/// For some of the types, the `Fragment` derive macro is used to implement the `Fragment` trait
/// which eases the rendering of the views in an HTMX context.
mod views {
    use askama::Template;
    use htmx_ssr::{DisplayDelegate, Fragment};

    /// The index page.
    #[derive(Template)]
    #[template(path = "blocks/index.html.jinja")]
    pub(super) struct Index {
        pub menu: Menu,
        pub page: Page,
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
        pub url: String,

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
        pub messages: Vec<super::model::Message>,
    }

    #[derive(Debug, Template)]
    #[template(path = "blocks/page/message_detail.html.jinja")]
    pub(super) struct PageMessageDetail {
        pub message: super::model::Message,
        pub red: bool,
    }

    #[derive(Debug, Template)]
    #[template(path = "blocks/page/settings.html.jinja")]
    pub(super) struct PageSettings {}
}

mod model {
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
                    url: "/dashboard".to_string(),
                    title: "Dashboard".to_string(),
                    icon: MenuItemIcon::Dashboard,
                },
                MenuItem {
                    url: "/messages".to_string(),
                    title: "Messages".to_string(),
                    icon: MenuItemIcon::Messages,
                },
                MenuItem {
                    url: "/settings".to_string(),
                    title: "Settings".to_string(),
                    icon: MenuItemIcon::Settings,
                },
            ];

            Self { items }
        }
    }

    #[derive(Debug, Clone)]
    pub(super) struct MenuItem {
        /// The URL of the menu item.
        pub url: String,

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
    use htmx_ssr::Controller;
    use htmx_ssr::{
        htmx::{FragmentExt, Request as HtmxRequest},
        ViewMapper,
    };
    use tokio::sync::Mutex;

    /// The main controller.
    #[derive(Debug, Clone, Controller)]
    pub enum MainController {
        /// The dashboard route.
        #[url("/")]
        #[url("/dashboard")]
        Dashboard,

        /// The messages route.
        #[url("/messages")]
        Messages,

        /// The message detail route.
        #[url("/messages/:id")]
        MessageDetail {
            /// The message ID.
            id: u8,

            /// Show the message in red.
            #[query]
            red: bool,
        },

        /// The settings route.
        #[url("/settings")]
        Settings,
    }

    /// Custom implementation.
    #[async_trait]
    impl ViewMapper for MainController {
        type Model = Arc<Mutex<super::model::Model>>;

        async fn render_view(
            self,
            state: htmx_ssr::State<Self::Model>,
            htmx: HtmxRequest,
        ) -> axum::response::Response {
            match self {
                Self::Dashboard => {
                    let menu = Self::make_menu(state.model.lock().await.deref(), 0);
                    let page = views::Page::Dashboard(views::PageDashboard {});
                    match htmx {
                        HtmxRequest::Classic => views::Index { menu, page }.into_response(),
                        HtmxRequest::Htmx { .. } => {
                            page.into_htmx_response().with_oob(menu).into_response()
                        }
                    }
                }
                Self::Messages => {
                    let model = state.model.lock().await;
                    let menu = Self::make_menu(model.deref(), 1);
                    let messages = model.messages.clone();
                    let page = views::Page::Messages(views::PageMessages { messages });

                    match htmx {
                        HtmxRequest::Classic => views::Index { menu, page }.into_response(),
                        HtmxRequest::Htmx { .. } => {
                            page.into_htmx_response().with_oob(menu).into_response()
                        }
                    }
                }
                Self::MessageDetail { id, red } => {
                    let model = state.model.lock().await;
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

                    let page =
                        views::Page::MessageDetail(views::PageMessageDetail { message, red });

                    match htmx {
                        HtmxRequest::Classic => views::Index { menu, page }.into_response(),
                        HtmxRequest::Htmx { .. } => {
                            page.into_htmx_response().with_oob(menu).into_response()
                        }
                    }
                }
                Self::Settings => {
                    let menu = Self::make_menu(state.model.lock().await.deref(), 2);
                    let page = views::Page::Settings(views::PageSettings {});

                    match htmx {
                        HtmxRequest::Classic => views::Index { menu, page }.into_response(),
                        HtmxRequest::Htmx { .. } => {
                            page.into_htmx_response().with_oob(menu).into_response()
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
