//! Run with
//!
//! ```not_rust
//! just example components
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

mod components {
    use axum::response::IntoResponse;
    use htmxology::{
        RouteExt, ServerInfo,
        htmx::{Request as HtmxRequest, ResponseExt},
    };

    use crate::controller::{AppRoute, TodoListElementCreateRoute};

    pub struct TodoListElementCreateForm;

    impl TodoListElementCreateForm {
        /// Handle a request to the form.
        pub async fn handle_request(
            controller: &crate::controller::MainController,
            route: TodoListElementCreateRoute,
            htmx: HtmxRequest,
            _parts: http::request::Parts,
            _server_info: &ServerInfo,
        ) -> Result<axum::response::Response, axum::response::Response> {
            match route {
                TodoListElementCreateRoute::Submit { backend_name, body } => {
                    let backend = controller.backends.get_or_create(backend_name).await;

                    backend.add_todo_element(body.title, body.description).await;

                    match htmx {
                        HtmxRequest::Classic => Err(AppRoute::Home.as_redirect_response()),
                        HtmxRequest::Htmx { .. } => {
                            let todo_list =
                                crate::views::TodoList::new_for_backend(backend_name, &backend)
                                    .await;

                            Ok(crate::views::TodoListElementCreateForm { backend_name }
                                .into_htmx_response()
                                .with_oob(todo_list)
                                .into_response())
                        }
                    }
                }
            }
        }
    }
}

mod views {

    use askama::Template;
    use htmxology::htmx::{HtmlForm, HtmlId, Identity};

    use crate::{
        backend::{Backend, BackendName, Backends, TodoElement},
        controller::AppRoute,
    };

    /// The index page.
    #[derive(Template)]
    #[template(path = "components/index.html.jinja")]
    pub(super) struct Index {
        /// The todo list for backend A.
        todo_list_a: TodoList,

        /// The todo list for backend B.
        todo_list_b: TodoList,
    }

    impl Index {
        /// Create a new index page.
        pub async fn new(backends: &Backends) -> Result<Self, axum::response::Response> {
            Ok(Self {
                todo_list_a: TodoList::new(backends, BackendName::A).await,
                todo_list_b: TodoList::new(backends, BackendName::B).await,
            })
        }

        /// Get a todo-list element create form for the specified backend.
        fn todo_list_element_create_form(
            &self,
            backend_name: BackendName,
        ) -> TodoListElementCreateForm {
            TodoListElementCreateForm { backend_name }
        }
    }

    /// The todo-list.
    #[derive(Template)]
    #[template(path = "components/forms/todo-list/index.html.jinja")]
    pub(super) struct TodoList {
        /// The backend to use.
        backend_name: BackendName,

        /// The list of todo elements.
        items: Vec<TodoElement>,
    }

    impl Identity for TodoList {
        fn id(&self) -> HtmlId {
            let Self {
                backend_name: backend,
                ..
            } = self;

            format!("todo-list-{backend}")
                .parse()
                .expect("valid HTML id")
        }
    }

    impl TodoList {
        /// Create a new todo list.
        pub async fn new(backends: &Backends, backend_name: BackendName) -> Self {
            let backend = backends.get_or_create(backend_name).await;

            Self::new_for_backend(backend_name, &backend).await
        }
        /// Create a new todo list for the specified backend.
        pub async fn new_for_backend(backend_name: BackendName, backend: &Backend) -> Self {
            let todo_elements = backend.get_todo_elements().await;

            Self {
                backend_name,
                items: todo_elements,
            }
        }
    }

    /// The todo-list element create form.
    #[derive(Template)]
    #[template(path = "components/forms/todo-list-element-create/form.html.jinja")]
    pub(super) struct TodoListElementCreateForm {
        /// The backend to use.
        pub backend_name: BackendName,
    }

    impl Identity for TodoListElementCreateForm {
        fn id(&self) -> HtmlId {
            let Self { backend_name } = self;

            format!("todo-list-element-create-form-{backend_name}")
                .parse()
                .expect("valid HTML id")
        }
    }

    impl HtmlForm for TodoListElementCreateForm {
        type Route = AppRoute;

        fn action_route(&self) -> AppRoute {
            AppRoute::Forms(crate::controller::FormsRoute::TodoListElementCreate(
                crate::controller::TodoListElementCreateRoute::Submit {
                    backend_name: self.backend_name,
                    body: Default::default(),
                },
            ))
        }
    }

    impl TodoListElementCreateForm {}
}

mod backend {
    use std::{
        collections::{BTreeMap, HashMap},
        sync::Arc,
    };

    use serde::{Deserialize, Serialize};
    use tokio::sync::Mutex;

    /// The name of a backend.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
    pub enum BackendName {
        /// The A backend.
        A,

        /// The B backend.
        B,
    }

    /// Display implementation.
    impl std::fmt::Display for BackendName {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                BackendName::A => write!(f, "A"),
                BackendName::B => write!(f, "B"),
            }
        }
    }

    /// A priority for a todo element.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
    pub enum Priority {
        /// A low priority.
        Low,

        /// A medium priority.
        #[default]
        Medium,

        /// A high priority.
        High,
    }

    /// Display implementation.
    impl std::fmt::Display for Priority {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Low => write!(f, "Low"),
                Self::Medium => write!(f, "Medium"),
                Self::High => write!(f, "High"),
            }
        }
    }

    /// A list of backends.
    #[derive(Debug, Clone, Default)]
    pub struct Backends(Arc<Mutex<HashMap<BackendName, Backend>>>);

    impl Backends {
        /// Get a backend by name, creating it if it doesn't exist.
        pub async fn get_or_create(&self, name: BackendName) -> Backend {
            let mut backends = self.0.lock().await;

            backends.entry(name).or_default().clone()
        }
    }

    /// A simple todo list backend.
    #[derive(Debug, Clone, Default)]
    pub struct Backend {
        /// The list of todo elements, ordered by ID.
        todo_elements: Arc<Mutex<BTreeMap<u32, TodoElement>>>,
    }

    impl Backend {
        /// Add a new todo element.
        pub async fn add_todo_element(
            &self,
            title: impl Into<String>,
            description: impl Into<String>,
        ) {
            let mut todo_elements = self.todo_elements.lock().await;

            let id = todo_elements.keys().last().map_or(1, |last_id| last_id + 1);

            let todo_element = TodoElement::new(id, title, description);

            todo_elements.insert(id, todo_element);
        }

        /// Get all todo elements.
        pub async fn get_todo_elements(&self) -> Vec<TodoElement> {
            let todo_elements = self.todo_elements.lock().await;

            todo_elements.values().cloned().collect()
        }
    }

    /// A single todo element.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct TodoElement {
        /// The ID of the todo element.
        pub id: u32,

        /// The title of the todo element.
        pub title: String,

        /// The description of the todo element.
        pub description: String,

        /// The priority of the todo element.
        pub priority: Priority,

        /// Whether the todo element is completed.
        pub completed: bool,
    }

    impl TodoElement {
        /// Create a new todo element.
        fn new(id: u32, title: impl Into<String>, description: impl Into<String>) -> Self {
            Self {
                id,
                title: title.into(),
                description: description.into(),
                priority: Priority::default(),
                completed: false,
            }
        }
    }
}

mod controller {

    use crate::backend::{BackendName, Backends};

    use super::views;
    use axum::response::IntoResponse;
    use htmxology::{Controller, htmx::Request as HtmxRequest};
    use htmxology::{RenderIntoResponse, Route, ServerInfo};
    use serde::Deserialize;

    /// The main application routes.
    #[derive(Debug, Clone, Route)]
    pub enum AppRoute {
        /// The home route.
        #[route("")]
        Home,

        /// The forms route.
        ///
        /// This is the base route for all form-related routes.
        #[route("forms/")]
        Forms(#[subroute] FormsRoute),
    }

    /// The forms sub-routes.
    #[derive(Debug, Clone, Route)]
    pub enum FormsRoute {
        /// The form that creates a new todo list element.
        #[route("todo-list-element-create/")]
        TodoListElementCreate(#[subroute] TodoListElementCreateRoute),
    }

    /// The base route for the todo list element create routes.
    #[derive(Debug, Clone, Route)]
    pub enum TodoListElementCreateRoute {
        /// The route to submit the form.
        #[route("{backend_name}", method = "POST")]
        Submit {
            /// The backend to use.
            backend_name: BackendName,

            /// The body of the request.
            #[body]
            body: TodoListElementCreateFormBody,
        },
    }

    /// The body of the todo list element create form.
    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TodoListElementCreateFormBody {
        /// The title of the todo element.
        pub title: String,

        /// The description of the todo element.
        pub description: String,
    }

    /// The main controller implementation.
    #[derive(Debug, Clone, Default)]
    pub struct MainController {
        /// The backends
        pub backends: Backends,
    }

    /// Custom implementation.
    impl Controller for MainController {
        type Route = AppRoute;

        async fn handle_request(
            &self,
            route: AppRoute,
            htmx: HtmxRequest,
            parts: http::request::Parts,
            server_info: &ServerInfo,
        ) -> Result<axum::response::Response, axum::response::Response> {
            match route {
                AppRoute::Home => Ok(views::Index::new(&self.backends)
                    .await?
                    .render_into_response()),
                AppRoute::Forms(FormsRoute::TodoListElementCreate(route)) => {
                    crate::components::TodoListElementCreateForm::handle_request(
                        self,
                        route,
                        htmx,
                        parts,
                        server_info,
                    )
                    .await
                }
            }
        }
    }
}
