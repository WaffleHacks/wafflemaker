use crate::config::Secrets;
use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use std::{collections::HashSet, sync::Arc};
use tokio::{
    select,
    sync::broadcast::Receiver,
    time::{self, Duration},
};
use tracing::{debug, error, info, instrument};
use url::Url;

mod error;
mod models;

use error::{Error, Result};
use models::*;

static STATIC_INSTANCE: Lazy<ArcSwap<Vault>> =
    Lazy::new(|| ArcSwap::from_pointee(Vault::default()));

/// A wrapper around the Hashicorp Vault API
#[derive(Debug)]
pub struct Vault {
    client: Client,
    url: Url,
}

impl Vault {
    /// Check that the token has the correct permissions
    #[instrument(skip(self), fields(url = %self.url))]
    async fn check_perms(&self) -> Result<()> {
        let response: Capabilities = self
            .client
            .post(format!("{}v1/sys/capabilities-self", self.url))
            .json(&Capabilities::paths())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        debug!("got permissions for token");

        for (path, expected) in Capabilities::mapping() {
            let got = match response.data.get(path) {
                Some(g) => g,
                None => {
                    error!("missing permissions for \"{}\"", path);
                    return Err(Error::InvalidPermissions);
                }
            };
            let matching = expected.intersection(got).collect::<HashSet<&Permission>>();

            if matching.len() != expected.len() {
                error!(expected = ?expected, got = ?got, "invalid permissions for \"{}\"", path);
                return Err(Error::InvalidPermissions);
            }

            debug!("valid permissions for \"{}\"", path);
        }

        info!("valid vault permissions");
        Ok(())
    }

    /// Renew the token for another period (24hr)
    #[instrument(skip(self))]
    async fn renew(&self) -> Result<()> {
        self.client
            .post(format!("{}v1/auth/token/renew-self", self.url))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

impl Default for Vault {
    fn default() -> Vault {
        Vault {
            client: Default::default(),
            url: Url::parse("http://127.0.0.1:8200").unwrap(),
        }
    }
}

/// Configure the vault service
pub async fn initialize(config: &Secrets, stop: Receiver<()>) -> Result<()> {
    let mut headers = HeaderMap::new();
    headers.insert("X-Vault-Token", HeaderValue::from_str(&config.token)?);

    let client = Client::builder()
        .default_headers(headers)
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;

    let renewal_period = config.period()?;

    let vault = Vault {
        client,
        url: Url::parse(&config.address)?,
    };
    vault.check_perms().await?;

    tokio::task::spawn(renewer(renewal_period, stop));

    STATIC_INSTANCE.swap(Arc::from(vault));
    Ok(())
}

/// Retrieve an instance of Vault
pub fn instance() -> Arc<Vault> {
    STATIC_INSTANCE.load().clone()
}

/// Automatically renew the token every 24hr
#[instrument]
async fn renewer(period: Duration, mut stop: Receiver<()>) {
    let mut interval = time::interval(period);

    loop {
        select! {
            _ = interval.tick() => {
                match instance().renew().await {
                    Ok(_) => info!("successfully renewed token"),
                    Err(e) => error!("failed to renew token: {}", e),
                }
            }
            _ = stop.recv() => {
                info!("stopping vault token renewer");
                break
            }
        }
    }
}
