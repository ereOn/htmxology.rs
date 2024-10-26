/// The HTMX state.
#[derive(Debug)]
pub struct State<T> {
    /// The base URL of the server.
    pub base_url: http::Uri,

    /// The user-defined state.
    pub user_state: T,
}
