use crate::config;
use once_cell::sync::OnceCell;
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT},
    Client,
};
use std::{error::Error as StdError, path::PathBuf, sync::Arc};
use tracing::{debug, error, instrument};
use url::Url;

mod error;
mod events;
mod services;

use error::{Error, Result};
pub use events::{Event, State};

// The GitHub API previews to enable
static GITHUB_API_PREVIEWS: &[&str; 2] = &[
    "application/vnd.github.ant-man-preview+json", // https://docs.github.com/rest/overview/api-previews#enhanced-deployments
    "application/vnd.github.flash-preview+json", // https://docs.github.com/rest/overview/api-previews#deployment-statuses
];
static NOTIFIERS: OnceCell<Arc<Vec<Notifier>>> = OnceCell::new();

/// Initialize the notifiers service
pub fn initialize() -> Result<()> {
    let cfg = &config::instance();

    let mut notifiers = Vec::new();
    for raw in &cfg.notifiers {
        notifiers.push(Notifier::extract(raw, &cfg.git.repository)?);
    }

    match NOTIFIERS.set(Arc::new(notifiers)) {
        Ok(_) => (),
        Err(_) => panic!("failed to initialize notifiers"),
    }
    Ok(())
}

/// Notify of an event that occurred. Can be treated as infallible as errors are
/// logged to the console directly.
#[instrument(skip(event), fields(event = ?event))]
pub async fn notify(event: Event<'_, '_>) {
    let notifiers = NOTIFIERS.get().unwrap().clone();

    for notifier in notifiers.iter() {
        match notifier.dispatch(&event).await {
            Ok(_) => debug!(r#type = %notifier.name(), "successfully dispatched notification"),
            Err(e) => match e.source() {
                Some(s) => {
                    error!(r#type = %notifier.name(), error = %e, source = %s, "failed to dispatch notification")
                }
                None => {
                    error!(r#type = %notifier.name(), error = %e, "failed to dispatch notification")
                }
            },
        }
    }
}

/// A validated notifier extracted from the configuration
enum Notifier {
    Discord {
        url: String,
        client: Client,
    },
    GitHub {
        owner: String,
        repository: String,
        client: Client,
        key: PathBuf,
        app_id: String,
        installation_id: String,
    },
}

impl Notifier {
    /// Dispatch an event to the service
    async fn dispatch(&self, event: &Event<'_, '_>) -> Result<()> {
        match self {
            Self::Discord { url, client } => services::discord(client, url, event).await,
            Self::GitHub {
                repository,
                owner,
                client,
                key,
                app_id,
                installation_id,
            } => {
                services::github(
                    client,
                    owner,
                    repository,
                    key,
                    app_id,
                    installation_id,
                    event,
                )
                .await
            }
        }
    }

    /// Get the name of the service
    fn name(&self) -> &str {
        match self {
            Self::Discord { .. } => "discord",
            Self::GitHub { .. } => "github",
        }
    }

    /// Extract and validate a notifier from the configuration
    fn extract(c: &config::Notifier, default_repo: &str) -> Result<Notifier> {
        use config::Notifier::*;

        let validated = match c {
            Discord { webhook } => {
                // Test the the URL is valid
                let parsed = Url::parse(webhook)?;

                Notifier::Discord {
                    url: parsed.to_string(),
                    client: Client::builder()
                        .user_agent(format!(
                            "{}/{}",
                            env!("CARGO_PKG_NAME"),
                            env!("CARGO_PKG_VERSION")
                        ))
                        .build()
                        .unwrap(),
                }
            }
            GitHub {
                app_id,
                installation_id,
                key,
                repository,
            } => {
                // Parse the repository owner and repo
                let mut parts = repository
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or(default_repo)
                    .split('/')
                    .take(2)
                    .map(String::from);
                let owner = parts.next().ok_or(Error::InvalidRepository)?;
                let repository = parts.next().ok_or(Error::InvalidRepository)?;

                // Ensure the private key exists
                if !key.exists() {
                    return Err(Error::InvalidKeyPath);
                }

                // Support GitHub preview apis
                let mut headers = HeaderMap::new();
                headers.insert(
                    ACCEPT,
                    HeaderValue::from_str(&GITHUB_API_PREVIEWS.join(","))?,
                );

                // Setup the client
                let client = Client::builder()
                    .user_agent(format!(
                        "{}/{}",
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_VERSION")
                    ))
                    .default_headers(headers)
                    .build()
                    .unwrap();

                Notifier::GitHub {
                    owner,
                    repository,
                    client,
                    key: key.to_owned(),
                    app_id: app_id.to_owned(),
                    installation_id: installation_id.to_owned(),
                }
            }
        };
        Ok(validated)
    }
}
