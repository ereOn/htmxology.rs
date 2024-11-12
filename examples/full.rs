//! Run with
//!
//! ```not_rust
//! just example full
//! ```

use axum::{response::Redirect, routing::get, Router};
use htmxology::View;
use tracing::info;

#[derive(View)]
struct View {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let v = MyView {
        name: "v".to_string(),
        age: 42,
    };

    tracing::warn!("foo: {v:?}");

    info!("Starting example `{}`...", env!("CARGO_BIN_NAME"));

    // Create a new user state.
    //
    // This can be any type that you want to store in the server state.
    //
    // It just has to be `Send + Sync + 'static`.
    let user_state = models::AppState::default();

    // Create a new server with auto-reload enabled by attempting to get a TCP listener from the
    // `listenfd` environment variable, falling back to binding to a local address if that fails.
    let mut server = htmxology::Server::new_with_auto_reload("127.0.0.1:3000", user_state)
        .await?
        // Set the options on the server from the environment.
        .with_options_from_env()?;

    *server.router() = Router::new()
        // The root URL is redirected to the root page.
        .route("/", get(|| async { Redirect::permanent("/food") }))
        .route(
            "/food",
            get(handlers::food::list).post(handlers::food::create),
        )
        .route(
            "/food/:food_id",
            get(handlers::food::read)
                .put(handlers::food::update)
                .delete(handlers::food::delete),
        )
        .route(
            "/food/:food_id/comments",
            get(handlers::food::comments::list),
        )
        .route("/ws", get(handlers::ws_handler));

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
                        comments: vec![
                            Comment {
                                author: "Alice".to_string(),
                                content: "I love pancakes!".to_string(),
                            },
                            Comment {
                                author: "Bob".to_string(),
                                content: "I don't like pancakes.".to_string(),
                            },
                        ],
                    },
                    Food {
                        image_src: "https://images.pexels.com/photos/70497/pexels-photo-70497.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=2".to_string(),
                        description: "A burger and fries".to_string(),
                        title: "Burger".to_string(),
                        enjoyment: Enjoyment::Delicious,
                        comments: vec![],
                    },
                    Food {
                        image_src: "https://images.pexels.com/photos/1099680/pexels-photo-1099680.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=2".to_string(),
                        description: "A fruit-salad".to_string(),
                        title: "Fruit-salad".to_string(),
                        enjoyment: Enjoyment::Okay,
                        comments: vec![],
                    },
                    Food {
                        image_src: "https://images.pexels.com/photos/1279330/pexels-photo-1279330.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=2".to_string(),
                        description: "A plate of spaghettis".to_string(),
                        title: "Spaghettis".to_string(),
                        enjoyment: Enjoyment::Okay,
                        comments: vec![],
                    },
                    Food {
                        image_src: "https://images.pexels.com/photos/958545/pexels-photo-958545.jpeg?auto=compress&cs=tinysrgb&w=1260&h=750&dpr=2".to_string(),
                        description: "Various indian food: naan breads, pakora, baji onions".to_string(),
                        title: "Indian food".to_string(),
                        enjoyment: Enjoyment::Delicious,
                        comments: vec![],
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

        /// The comments.
        pub comments: Vec<Comment>,
    }

    /// Represents a comment.
    #[derive(Debug, Clone)]
    pub struct Comment {
        /// The author of the comment.
        pub author: String,

        /// The content of the comment.
        pub content: String,
    }
}

mod views {
    use std::fmt::Display;

    use askama::Template;

    /// The food view model.
    ///
    /// Not auto-generated.
    pub struct Food {
        pub image_src: String,
        pub description: String,
        pub title: String,
        pub width: u8,
        pub comments_count: usize,
    }

    impl From<crate::models::Food> for Food {
        fn from(food: crate::models::Food) -> Self {
            Self {
                image_src: food.image_src,
                description: food.description,
                title: food.title,
                width: match food.enjoyment {
                    crate::models::Enjoyment::Okay => 1,
                    crate::models::Enjoyment::Delicious => 2,
                },
                comments_count: food.comments.len(),
            }
        }
    }

    pub struct Comment {
        pub author: String,
        pub content: String,
    }

    impl From<crate::models::Comment> for Comment {
        fn from(comment: crate::models::Comment) -> Self {
            Self {
                author: comment.author,
                content: comment.content,
            }
        }
    }

    /// The debug panel.
    ///
    /// Auto-generated.
    #[derive(Template)]
    #[template(path = "full/debug-panel.html.jinja")]
    pub(super) struct DebugPanel {
        pub(super) unique_id: uuid::Uuid,
        pub(super) ws_connected: bool,
    }

    impl Default for DebugPanel {
        fn default() -> Self {
            Self {
                unique_id: uuid::Uuid::new_v4(),
                ws_connected: false,
            }
        }
    }

    /// The root page.
    ///
    /// Auto-generated.
    #[derive(Template)]
    #[template(path = "full/index.html.jinja")]
    pub(super) struct Root {
        pub(super) debug_panel: DebugPanel,
        pub(super) page: RootContent,
    }

    // Auto-generated.
    pub(super) enum RootContent {
        /// The food many page.
        ///
        /// Exposed behind the `/food` route.
        FoodList(FoodList),

        /// The food one page.
        ///
        /// Exposed behind the `/food/:food_id` route.
        FoodSingle(FoodSingle),
    }

    // This should be automatically generated by the `derive(HtmxSsrView)` macro.
    impl Display for RootContent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::FoodList(page) => write!(f, "{page}"),
                Self::FoodSingle(page) => write!(f, "{page}"),
            }
        }
    }

    // Auto-generated.
    #[derive(Template)]
    #[template(path = "full/food/list.html.jinja")]
    pub(super) struct FoodList {
        pub items: Vec<(usize, Food)>,
    }

    // Auto-generated.
    #[derive(Template)]
    #[template(path = "full/food/single.html.jinja")]
    pub(super) struct FoodSingle {
        // The id of the food.
        pub id: usize,

        // The food item.
        pub item: Food,

        // Auto-generated sub-field.
        pub comments: Option<FoodContent>,
    }

    pub(super) enum FoodContent {
        /// The food many page.
        ///
        /// Exposed behind the `/food/:food_id/comment` route.
        CommentList(FoodCommentList),

        /// The food one page.
        ///
        /// Exposed behind the `/food/:food_id/comment/:comment_id` route.
        CommentSingle(FoodCommentSingle),
    }

    // This should be automatically generated by the `derive(HtmxSsrView)` macro.
    impl Display for FoodContent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::CommentList(page) => write!(f, "{page}"),
                Self::CommentSingle(page) => write!(f, "{page}"),
            }
        }
    }
    #[derive(Template)]
    #[template(path = "full/food/comment/list.html.jinja")]
    pub(super) struct FoodCommentList {
        pub items: Vec<(usize, Comment)>,
    }

    #[derive(Template)]
    #[template(path = "full/food/comment/single.html.jinja")]
    pub(super) struct FoodCommentSingle {
        pub id: usize,
        pub item: Comment,
    }
}

mod handlers {
    use askama::Template;
    use askama_axum::IntoResponse;
    use axum::extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    };
    use tracing::{error, info};

    use crate::{models::AppState, views::DebugPanel};

    pub mod food {
        use super::super::{models::AppState, views};
        use askama_axum::IntoResponse;
        use axum::async_trait;
        use axum::extract::{Path, State};
        use htmxology::ArcState as HtmxState;
        use http::request::Parts;

        // This would be generated by the `derive(HtmxSsrView)` macro.
        pub async fn list(State(state): State<HtmxState<AppState>>) -> views::Root {
            let content = views::RootContent::FoodList(views::FoodList {
                items: state
                    .user_state
                    .food_items
                    .lock()
                    .await
                    .clone()
                    .into_iter()
                    .map(Into::into)
                    .enumerate()
                    .collect(),
            });

            views::Root {
                debug_panel: views::DebugPanel::default(),
                page: content,
            }
        }

        pub async fn create(State(state): State<HtmxState<AppState>>) -> views::Root {
            // TODO: Accept form arguments and create a new food item.

            let content = views::RootContent::FoodList(views::FoodList {
                items: state
                    .user_state
                    .food_items
                    .lock()
                    .await
                    .clone()
                    .into_iter()
                    .map(Into::into)
                    .enumerate()
                    .collect(),
            });

            views::Root {
                debug_panel: views::DebugPanel::default(),
                page: content,
            }
        }

        pub struct HxTarget(Option<String>);

        #[async_trait]
        impl<S> axum::extract::FromRequestParts<S> for HxTarget
        where
            S: Send + Sync,
        {
            type Rejection = (http::StatusCode, String);

            async fn from_request_parts(
                parts: &mut Parts,
                _state: &S,
            ) -> Result<Self, Self::Rejection> {
                parts
                    .headers
                    .get("Hx-Target")
                    .map(|v| v.to_str().map(str::to_owned))
                    .transpose()
                    .map(Self)
                    .map_err(|e| (http::StatusCode::BAD_REQUEST, e.to_string()))
            }
        }

        pub async fn read(
            State(state): State<HtmxState<AppState>>,
            HxTarget(hx_target): HxTarget,
            Path(id): Path<usize>,
        ) -> axum::response::Response {
            let item = state.user_state.food_items.lock().await.get(id).cloned();

            match item {
                Some(item) => {
                    // This is custom code.
                    let page = views::FoodSingle {
                        id,
                        item: item.into(),
                        comments: None,
                    };

                    match hx_target.as_deref() {
                        None => views::Root {
                            debug_panel: views::DebugPanel::default(),
                            page: views::RootContent::FoodSingle(page),
                        }
                        .into_response(),
                        Some("page") => page.into_response(),
                        _ => (http::StatusCode::BAD_REQUEST, "Bad request").into_response(),
                    }
                }
                None => (http::StatusCode::NOT_FOUND, "Not found").into_response(),
            }
        }

        pub async fn update(
            State(_state): State<HtmxState<AppState>>,
            Path(_id): Path<usize>,
        ) -> axum::response::Response {
            todo!()
        }

        pub async fn delete(
            State(state): State<HtmxState<AppState>>,
            Path(id): Path<usize>,
        ) -> axum::response::Response {
            let removed = {
                let mut items = state.user_state.food_items.lock().await;

                if id < items.len() {
                    items.remove(id);

                    true
                } else {
                    false
                }
            };

            if removed {
                list(State(state)).await.into_response()
            } else {
                (http::StatusCode::NOT_FOUND, "Not found").into_response()
            }
        }

        pub mod comments {
            use super::{
                super::super::{models::AppState, views},
                HxTarget,
            };
            use askama_axum::IntoResponse;
            use axum::extract::{Path, State};
            use htmxology::ArcState as HtmxState;

            // This would be generated by the `derive(HtmxSsrView)` macro.
            pub async fn list(
                State(state): State<HtmxState<AppState>>,
                Path(food_id): Path<usize>,
                HxTarget(hx_target): HxTarget,
            ) -> axum::response::Response {
                match hx_target.as_deref() {
                    None => {
                        let item = state
                            .user_state
                            .food_items
                            .lock()
                            .await
                            .get(food_id)
                            .cloned()
                            .unwrap();

                        let comments = item
                            .comments
                            .clone()
                            .into_iter()
                            .map(Into::into)
                            .enumerate()
                            .collect();

                        let content = views::RootContent::FoodSingle(views::FoodSingle {
                            id: food_id,
                            item: item.into(),
                            comments: Some(views::FoodContent::CommentList(
                                views::FoodCommentList { items: comments },
                            )),
                        });

                        views::Root {
                            debug_panel: views::DebugPanel::default(),
                            page: content,
                        }
                        .into_response()
                    }
                    Some("page") => {
                        let item = state
                            .user_state
                            .food_items
                            .lock()
                            .await
                            .get(food_id)
                            .cloned()
                            .unwrap();

                        let comments = item
                            .comments
                            .clone()
                            .into_iter()
                            .map(Into::into)
                            .enumerate()
                            .collect();

                        views::FoodSingle {
                            id: food_id,
                            item: item.into(),
                            comments: Some(views::FoodContent::CommentList(
                                views::FoodCommentList { items: comments },
                            )),
                        }
                        .into_response()
                    }
                    Some("comment") => {
                        let item = state
                            .user_state
                            .food_items
                            .lock()
                            .await
                            .get(food_id)
                            .cloned()
                            .unwrap();

                        let comments = item
                            .comments
                            .clone()
                            .into_iter()
                            .map(Into::into)
                            .enumerate()
                            .collect();

                        views::FoodCommentList { items: comments }.into_response()
                    }
                    _ => (http::StatusCode::BAD_REQUEST, "Bad request").into_response(),
                }
            }
        }
    }

    use htmxology::ArcState as HtmxState;

    pub async fn ws_handler(
        ws: WebSocketUpgrade,
        State(state): State<HtmxState<AppState>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(move |socket| ws_connection(socket, state))
    }

    async fn ws_connection(mut socket: WebSocket, state: HtmxState<AppState>) {
        info!("WebSocket connection established.");

        if let Err(err) = socket
            .send(Message::Text(
                DebugPanel {
                    unique_id: uuid::Uuid::new_v4(),
                    ws_connected: true,
                }
                .render()
                .unwrap(),
            ))
            .await
        {
            error!("Failed to send message: {}", err);
        }

        while let Some(msg) = socket.recv().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("Received message: {}", text);
                }
                Ok(_) => {
                    info!("Received non-text message");
                }
                Err(err) => {
                    error!("Failed to receive message: {}", err);
                }
            }
        }

        info!("WebSocket connection closed.");
    }
}
