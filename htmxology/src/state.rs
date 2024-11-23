use std::sync::Arc;

use axum::extract::FromRef;

use crate::server::ServerInfo;

/// The HTMX state.
#[derive(Debug, Clone)]
pub struct ServerState<Controller> {
    /// The server information.
    pub server_info: Arc<ServerInfo>,

    /// The user-defined state.
    pub controller: Controller,
}

impl<T> FromRef<ServerState<T>> for Arc<ServerInfo> {
    fn from_ref(state: &ServerState<T>) -> Arc<ServerInfo> {
        state.server_info.clone()
    }
}
