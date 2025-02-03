//! Server options.

/// The options for the server.
#[derive(Debug, Clone, Default)]
pub struct ServerOptions {
    /// The base HTTP URL of the server.
    ///
    /// If the server is running behind a reverse proxy, this should be set to the base URL of the
    /// proxy.
    ///
    /// If no base URL is set, the server will attempt to determine the base URL from its own TCP
    /// listener address.
    ///
    /// If `HTMXOLOGY_BASE_URL` is set in the environment, it will be read and used as the base URL
    /// when calling `ServerOptions::from_env`.
    pub base_url: Option<http::Uri>,
}

/// An error that can occur when trying to get the server options from the environment.
#[derive(Debug, thiserror::Error)]
pub enum ServerOptionsFromEnvError {
    /// An environment variable was not unicode.
    #[error("environment variable {name} was not unicode")]
    NotUnicode {
        /// The name of the environment variable.
        name: &'static str,
    },

    /// An error occurred while trying to get the base URL from the environment.
    #[error("failed to parse the base URL from environment variable {name} (was `{url}`): {err}")]
    BaseUrl {
        /// The name of the environment variable.
        name: &'static str,

        /// The URL that was attempted to be parsed.
        url: String,

        /// The error that occurred.
        #[source]
        err: http::uri::InvalidUri,
    },
}

impl ServerOptions {
    /// The environment variable name for the base URL.
    pub const HTMXOLOGY_BASE_URL: &'static str = "HTMXOLOGY_BASE_URL";

    fn env_var(name: &'static str) -> Result<Option<String>, ServerOptionsFromEnvError> {
        match std::env::var(name) {
            Ok(value) => Ok(if value.is_empty() { None } else { Some(value) }),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => {
                Err(ServerOptionsFromEnvError::NotUnicode { name })
            }
        }
    }

    /// Get the server options from the environment.
    pub fn from_env() -> Result<Self, ServerOptionsFromEnvError> {
        tracing::info!("Reading HTMX SSR server options from the environment...");

        let base_url = Self::env_var(Self::HTMXOLOGY_BASE_URL)?
            .map(|url| {
                url.parse()
                    .map_err(|err| ServerOptionsFromEnvError::BaseUrl {
                        name: Self::HTMXOLOGY_BASE_URL,
                        url: url.clone(),
                        err,
                    })
            })
            .transpose()?;

        match &base_url {
            Some(base_url) => {
                tracing::info!(
                    "{} was set: using `{base_url}` as the base URL.",
                    Self::HTMXOLOGY_BASE_URL
                );
            }
            None => {
                tracing::warn!(
                    "{} was not set: base URL will be determined from the TCP listener address. This may not be what you want.",
                    Self::HTMXOLOGY_BASE_URL
                );
            }
        };

        Ok(Self { base_url })
    }
}
