use crate::config::Secrets;
use once_cell::sync::OnceCell;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Method, StatusCode,
};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
};
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

pub use models::{StaticCredentials, AWS};

static STATIC_INSTANCE: OnceCell<Arc<Vault>> = OnceCell::new();

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

    /// Fetch all the static secrets for a service by name if they exist
    #[instrument(skip(self))]
    pub async fn fetch_static(&self, name: &str) -> Result<Option<HashMap<String, String>>> {
        let response = self
            .client
            .get(format!("{}v1/services/data/{}", self.url, name))
            .send()
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            info!("new secrets to be created");
            return Ok(None);
        }

        let content: BaseResponse<Secret> = response.error_for_status()?.json().await?;
        info!("found existing secrets");
        Ok(Some(content.data.data))
    }

    /// Save the static secrets for a service
    #[instrument(skip(self, secrets))]
    pub async fn put_static(&self, name: &str, secrets: HashMap<String, String>) -> Result<()> {
        self.client
            .post(format!("{}v1/services/data/{}", self.url, name))
            .json(&Secret { data: secrets })
            .send()
            .await?
            .error_for_status()?;
        info!("added secrets for service");
        Ok(())
    }

    /// Fetch AWS credentials using the given role
    #[instrument(skip(self))]
    pub async fn aws_credentials(&self, role: &str) -> Result<AWS> {
        let response: BaseResponse<AWS> = self
            .client
            .get(format!("{}v1/aws/creds/{}", self.url, role))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        info!("generated AWS credentials");
        Ok(response.data)
    }

    /// List all the static roles for PostgreSQL
    pub async fn list_database_roles(&self) -> Result<Vec<String>> {
        let response = self
            .client
            .request(
                Method::from_str("LIST").unwrap(),
                format!("{}v1/database/static-roles", self.url),
            )
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            info!("gathered list of all database users");
            Ok(Vec::new())
        } else {
            let content: BaseResponse<StaticRoles> = response.error_for_status()?.json().await?;
            info!("gathered list of all database users");
            Ok(content.data.keys)
        }
    }

    /// Create a static role within PostgreSQL
    pub async fn create_database_role(&self, name: &str) -> Result<()> {
        self.client
            .post(format!("{}v1/database/static-roles/{}", self.url, name))
            .json(&StaticRoles::new(name))
            .send()
            .await?
            .error_for_status()?;
        info!("created database user");
        Ok(())
    }

    /// Delete a static role from PostgreSQL
    pub async fn delete_database_role(&self, name: &str) -> Result<()> {
        self.client
            .delete(format!("{}v1/database/static-roles/{}", self.url, name))
            .send()
            .await?
            .error_for_status()?;
        info!("deleted database user");
        Ok(())
    }

    /// Get credentials for a static role from PostgreSQL
    pub async fn get_database_credentials(&self, role: &str) -> Result<StaticCredentials> {
        let response: BaseResponse<StaticCredentials> = self
            .client
            .get(format!("{}v1/database/static-creds/{}", self.url, role))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        info!("generated credentials for database user");
        Ok(response.data)
    }

    /// Rotate the credentials for a static role in PostgreSQL
    pub async fn rotate_database_credentials(&self, role: &str) -> Result<()> {
        self.client
            .post(format!("{}v1/database/rotate-role/{}", self.url, role))
            .send()
            .await?
            .error_for_status()?;
        info!("refreshed database user credentials");
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

    STATIC_INSTANCE.get_or_init(|| Arc::from(vault));
    Ok(())
}

/// Retrieve an instance of Vault
pub fn instance() -> Arc<Vault> {
    STATIC_INSTANCE.get().unwrap().clone()
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
