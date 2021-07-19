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
use tokio::sync::broadcast::Sender;
use tracing::{debug, error, info, instrument};
use url::Url;

mod error;
mod models;
mod renewal;

use error::{Error, Result};
use models::*;

pub use models::AWS;

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
    pub async fn aws_credentials(&self, role: &str) -> Result<(AWS, Lease)> {
        let response: BaseResponseWithLease<AWS> = self
            .client
            .get(format!("{}v1/aws/creds/{}", self.url, role))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        info!("generated AWS credentials");
        Ok((response.data, response.lease))
    }

    /// List all the roles for PostgreSQL
    pub async fn list_database_roles(&self) -> Result<Vec<String>> {
        let response = self
            .client
            .request(
                Method::from_str("LIST").unwrap(),
                format!("{}v1/database/roles", self.url),
            )
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            info!("gathered list of all database users");
            Ok(Vec::new())
        } else {
            let content: BaseResponse<DatabaseRole> = response.error_for_status()?.json().await?;
            info!("gathered list of all database users");
            Ok(content.data.keys)
        }
    }

    /// Create a role within PostgreSQL
    pub async fn create_database_role(&self, name: &str) -> Result<()> {
        self.client
            .post(format!("{}v1/database/roles/{}", self.url, name))
            .json(&DatabaseRole::new(name))
            .send()
            .await?
            .error_for_status()?;
        info!("created database user");
        Ok(())
    }

    /// Delete a static role from PostgreSQL
    pub async fn delete_database_role(&self, name: &str) -> Result<()> {
        self.client
            .delete(format!("{}v1/database/roles/{}", self.url, name))
            .send()
            .await?
            .error_for_status()?;
        info!("deleted database user");
        Ok(())
    }

    /// Get credentials for a static role from PostgreSQL
    pub async fn get_database_credentials(&self, role: &str) -> Result<(RoleCredentials, Lease)> {
        let response: BaseResponseWithLease<RoleCredentials> = self
            .client
            .get(format!("{}v1/database/creds/{}", self.url, role))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        info!("generated credentials for database user");
        Ok((response.data, response.lease))
    }

    /// Register credential leases to be renewed
    pub async fn register_leases(&self, id: &str, new_leases: Vec<Lease>) {
        let mut leases = renewal::LEASES.write().await;
        leases.insert(id.to_owned(), new_leases);
    }

    /// Revoke any releases if they existed
    pub async fn revoke_leases(&self, id: &str) -> Result<()> {
        let revoked = {
            let mut leases = renewal::LEASES.write().await;
            leases.remove(id)
        };

        // Revoke any leases if the existed
        if let Some(leases) = revoked {
            for lease in leases {
                self.client
                    .put(format!("{}v1/sys/leases/revoke", self.url))
                    .json(&LeaseRevocation {
                        lease_id: &lease.id,
                    })
                    .send()
                    .await?
                    .error_for_status()?;

                info!(id = %lease.id, "revoked lease");
            }
        }

        info!("revoked leases for container");

        Ok(())
    }

    /// Renew an individual lease for a new TTL
    async fn renew_lease(&self, lease: &Lease) -> Result<()> {
        self.client
            .put(format!("{}v1/sys/leases/renew", self.url))
            .json(&LeaseRenewal {
                lease_id: &lease.id,
                increment: lease.ttl,
            })
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
pub async fn initialize(config: &Secrets, stop: Sender<()>) -> Result<()> {
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

    let lease_interval = config.lease_interval()?;
    let token_interval = config.token_interval()?;

    let vault = Vault {
        client,
        url: Url::parse(&config.address)?,
    };
    vault.check_perms().await?;

    // Spawn the renewal tasks
    tokio::task::spawn(renewal::token(token_interval, stop.subscribe()));
    tokio::task::spawn(renewal::leases(
        lease_interval,
        config.lease_percent,
        stop.subscribe(),
    ));

    STATIC_INSTANCE.get_or_init(|| Arc::from(vault));
    Ok(())
}

/// Retrieve an instance of Vault
pub fn instance() -> Arc<Vault> {
    STATIC_INSTANCE.get().unwrap().clone()
}
