//! Run with
//!
//! ```not_rust
//! just example fuul
//! ```

use axum::{response::Redirect, routing::get, Router};
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
    let user_state = models::AppState::default();

    // Create a new server with auto-reload enabled by attempting to get a TCP listener from the
    // `listenfd` environment variable, falling back to binding to a local address if that fails.
    let mut server = htmx_ssr::Server::new_with_auto_reload("127.0.0.1:3000", user_state)
        .await?
        // Set the options on the server from the environment.
        .with_options_from_env()?;

    *server.router() = Router::new()
        // The root URL is redirected to the root page.
        .route("/", get(|| async { Redirect::permanent("/page") }))
        // The root page has its food page as the default.
        .route("/page", get(|| async { Redirect::permanent("/page/food") }))
        .route("/page/:page", get(handlers::page))
        // Experimental
        .route("/page/food/edit/:id", get(handlers::food_edit));

    server.serve().await.map_err(Into::into)
}

mod models {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    /// The application state.
    #[derive(Debug)]
    pub struct AppState {
        pub food_items: Arc<Mutex<Vec<Food>>>,
    }

    impl Default for AppState {
        fn default() -> Self {
            Self {
                food_items: Arc::new(Mutex::new(vec![
                    Food {
                        image_src: "https://images.pexels.com/photos/376464/pexels-photo-376464.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=2".to_string(),
                        description: "Delicious pancakes".to_string(),
                        title: "Pancakes".to_string(),
                        enjoyment: Enjoyment::Okay,
                    },
                    Food {
                        image_src: "https://images.pexels.com/photos/70497/pexels-photo-70497.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=2".to_string(),
                        description: "A burger and fries".to_string(),
                        title: "Burger".to_string(),
                        enjoyment: Enjoyment::Delicious,
                    },
                    Food {
                        image_src: "https://images.pexels.com/photos/1099680/pexels-photo-1099680.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=2".to_string(),
                        description: "A fruit-salad".to_string(),
                        title: "Fruit-salad".to_string(),
                        enjoyment: Enjoyment::Okay,
                    },
                    Food {
                        image_src: "https://images.pexels.com/photos/1279330/pexels-photo-1279330.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=2".to_string(),
                        description: "A plate of spaghettis".to_string(),
                        title: "Spaghettis".to_string(),
                        enjoyment: Enjoyment::Okay,
                    },
                    Food {
                        image_src: "https://images.pexels.com/photos/958545/pexels-photo-958545.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=2".to_string(),
                        description: "Various indian food: naan breads, pakora, baji onions".to_string(),
                        title: "Indian food".to_string(),
                        enjoyment: Enjoyment::Delicious,
                    },
                ])),
            }
        }
    }

    /// The enjoyment of the food.
    #[derive(Debug, Clone, Copy)]
    pub enum Enjoyment {
        /// The food is okay.
        Okay,

        /// The food is delicious.
        Delicious,
    }

    /// Represents a food.
    #[derive(Debug, Clone)]
    pub struct Food {
        /// A URL to the image of the food.
        pub image_src: String,

        /// A description for the food.
        pub description: String,

        /// The title of the food.
        pub title: String,

        /// The enjoyment of the food.
        pub enjoyment: Enjoyment,
    }
}

mod views {
    use std::{fmt::Display, str::FromStr};

    use askama::Template;
    use axum::async_trait;

    // #[derive(HtmxSsrView)]
    // // This would not be necessary as it is the default lowercased name of the struct.
    // // The root is a prefix added to the path of the templates.
    // // The template default name is `page.html.jinja`.
    // #[htmx_ssr(id="page", root="full/")]
    // pub(super) struct Page {
    //    /// Indicates the content of the page that will be rendered inside the HTML element with
    //    the ID `page`.
    //    #[content]
    //    pub(super) content: PageContent,
    // }
    //
    // #[derive(HtmxSsrView)]
    // // Implements the `Display` trait for the `PageContent` enum.
    // pub(super) enum PageContent {
    //    /// The food page.
    //    Food(PageFood),
    // }

    /// The root page.
    #[derive(Template)]
    #[template(path = "full/page.html.jinja")]
    pub(super) struct Page {
        pub(super) content: PageContent,
    }

    pub(super) enum PageContent {
        /// The food page.
        ///
        /// Exposed behind the `/page/food` route.
        Food(PageFood),
    }

    // This should be automatically generated by the `derive(HtmxSsrView)` macro.
    impl Display for PageContent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Food(page) => write!(f, "{page}"),
            }
        }
    }

    // This should be automatically generated by the `derive(HtmxSsrView)` macro.
    #[derive(Debug, Clone, Copy)]
    pub(super) enum PageView {
        Food,
    }

    impl PageView {
        const FOOD: &'static str = "food";
    }

    impl Display for PageView {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Food => write!(f, "{}", Self::FOOD),
            }
        }
    }

    // This should be automatically generated by the `derive(HtmxSsrView)` macro.
    #[derive(Debug, thiserror::Error)]
    pub(super) enum PageViewError {
        #[error("invalid page: {0}")]
        InvalidPage(String),

        #[error(transparent)]
        Path(#[from] axum::extract::rejection::PathRejection),
    }

    impl axum::response::IntoResponse for PageViewError {
        fn into_response(self) -> askama_axum::Response {
            (http::StatusCode::BAD_REQUEST, self.to_string()).into_response()
        }
    }

    impl FromStr for PageView {
        type Err = PageViewError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                Self::FOOD => Ok(Self::Food),
                _ => Err(PageViewError::InvalidPage(s.to_string())),
            }
        }
    }

    #[async_trait]
    impl<S: Send + Sync> axum::extract::FromRequestParts<S> for PageView {
        type Rejection = PageViewError;

        async fn from_request_parts(
            parts: &mut http::request::Parts,
            state: &S,
        ) -> Result<Self, Self::Rejection> {
            let page = axum::extract::Path::<String>::from_request_parts(parts, state)
                .await
                .map_err(PageViewError::Path)?;

            page.parse()
        }
    }

    #[derive(Template)]
    #[template(path = "full/page/food.html.jinja")]
    pub(super) struct PageFood {
        pub items: Vec<Food>,
    }

    #[derive(Template)]
    #[template(path = "full/partials/food.html.jinja")]
    pub(super) struct Food {
        pub id: usize,
        pub src: String,
        pub alt: String,
        pub title: String,
        pub width: u8,
    }

    #[derive(Template)]
    #[template(path = "full/food_edit.html.jinja")]
    pub(super) struct FoodEdit {
        pub id: usize,
        pub src: String,
        pub alt: String,
        pub title: String,
        pub width: u8,
    }

    impl From<(usize, super::models::Food)> for Food {
        fn from((id, picture): (usize, super::models::Food)) -> Self {
            Self {
                id,
                src: picture.image_src,
                alt: picture.description,
                title: picture.title,
                width: match picture.enjoyment {
                    super::models::Enjoyment::Okay => 1,
                    super::models::Enjoyment::Delicious => 2,
                },
            }
        }
    }
}

mod handlers {
    use super::{models::AppState, views};
    use askama_axum::IntoResponse;
    use axum::extract::{Path, State};
    use htmx_ssr::ArcState as HtmxState;

    // This would be generated by the `derive(HtmxSsrView)` macro.
    pub async fn page(
        State(state): State<HtmxState<AppState>>,
        view: views::PageView,
    ) -> views::Page {
        let content = match view {
            views::PageView::Food => views::PageContent::Food(page_food(State(state)).await),
        };

        views::Page { content }
    }

    pub async fn page_food(State(state): State<HtmxState<AppState>>) -> views::PageFood {
        let items = state
            .user_state
            .food_items
            .lock()
            .await
            .clone()
            .into_iter()
            .enumerate()
            .map(Into::into)
            .collect();

        views::PageFood { items }
    }

    pub async fn food_edit(
        State(state): State<HtmxState<AppState>>,
        Path(id): Path<usize>,
    ) -> axum::response::Response {
        // TODO: Implement a helper (like an option) to return the fact that some model does not
        // exist (404).
        let item = state.user_state.food_items.lock().await.get(id).cloned();

        match item {
            Some(item) => views::FoodEdit {
                id,
                src: item.image_src,
                alt: item.description,
                title: item.title,
                width: match item.enjoyment {
                    super::models::Enjoyment::Okay => 1,
                    super::models::Enjoyment::Delicious => 2,
                },
            }
            .into_response(),
            None => (http::StatusCode::NOT_FOUND, "Not found").into_response(),
        }
    }
}
