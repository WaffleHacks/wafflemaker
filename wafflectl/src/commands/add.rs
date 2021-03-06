use super::*;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

// wafflectl add lease {service} {id} {ttl} [last updated]
#[derive(Debug, StructOpt)]
pub enum Add {
    /// Add a lease to track
    ///
    /// When the service it is registered with gets deleted, the lease will
    /// be revoked as well. If no deployments exist for the specified service,
    /// the lease will not be tracked.
    Lease {
        /// The name of the service to register the lease with
        service: String,
        /// The ID of the lease within Vault
        id: String,
        /// How long the lease is valid for
        ttl: u64,
        #[structopt(short, long)]
        /// When the lease was last updated, defaults to now
        updated_at: Option<u64>,
    },
}

impl Subcommand for Add {
    /// Handle the subcommand call
    fn execute(&self, client: Client) -> Result<Option<Table>> {
        match self {
            Self::Lease {
                service,
                id,
                ttl,
                updated_at,
            } => {
                let body = Lease {
                    id,
                    ttl: *ttl,
                    updated_at: updated_at.unwrap_or_else(now),
                };
                client.put(&["leases", service.as_str()], Some(body))?;
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Serialize)]
struct Lease<'i> {
    id: &'i str,
    ttl: u64,
    updated_at: u64,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
