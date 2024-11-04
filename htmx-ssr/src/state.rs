use std::sync::Arc;

use axum::extract::FromRef;

use crate::server::ServerInfo;

/// The HTMX state.
#[derive(Debug, Clone)]
pub struct State<Model> {
    /// The server information.
    pub server: Arc<ServerInfo>,

    /// The user-defined state.
    pub model: Model,
}

impl<T> FromRef<State<T>> for Arc<ServerInfo> {
    fn from_ref(state: &State<T>) -> Arc<ServerInfo> {
        state.server.clone()
    }
}
