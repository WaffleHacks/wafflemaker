use super::*;
use crate::http::service_path;
use std::collections::HashMap;
use tabled::{Disable, Header};

// wafflectl get <deployments|leases|services|service {name}>
#[derive(Debug, StructOpt)]
pub enum Get {
    /// Get the most recently deployed version
    ///
    /// Gets the commit hash of the most recently deployed version
    /// and some simple statistics about the current services.
    Deployments,
    /// Get all the currently registered leases
    ///
    /// Get a map of services to the leases that are currently
    /// registered with WaffleMaker and being renewed.
    Leases,
    /// Get a list of all the deployed services
    ///
    /// While the services are deployed, they are not necessarily
    /// running as an error could have occurred while deploying
    /// or the underlying container was manually stopped.
    Services,
    /// Get details about a service
    ///
    /// Get the configuration for a service as WaffleMaker sees it
    /// and the ID of the container running it (if any).
    Service {
        /// The name of the service
        name: String,
    },
}

impl Subcommand for Get {
    /// Handle the subcommand call
    fn execute(&self, client: Client) -> Result<Option<Table>> {
        let table = match self {
            Self::Deployments => {
                let response: DeploymentsResponse = client.get(&["deployments"])?;
                Table::new(&[response])
            }
            Self::Leases => {
                let response: LeasesResponse = client.get(&["leases"])?;
                Table::new(&response.into_table())
            }
            Self::Services => {
                let response: Vec<String> = client.get(&["services"])?;
                Table::new(response)
                    .with(Header("services"))
                    .with(Disable::Row(1..=1))
            }
            Self::Service { name } => {
                let response: Service = client.get(&service_path("services", &name))?;
                Table::new(&[response])
            }
        };

        Ok(Some(table))
    }
}

#[derive(Deserialize, Tabled)]
struct DeploymentsResponse {
    commit: String,
    services: u64,
    running: u64,
}

#[derive(Deserialize, Tabled)]
struct Lease {
    #[serde(skip)]
    service: String,
    id: String,
    ttl: u64,
    updated_at: u64,
}

#[derive(Deserialize)]
struct LeasesResponse {
    leases: HashMap<String, Vec<Lease>>,
    services: HashMap<String, String>,
}

impl LeasesResponse {
    /// Convert the response into a format that can be a table
    fn into_table(mut self) -> Vec<Lease> {
        let mut table = Vec::new();

        for (service, container) in self.services {
            if let Some(mut leases) = self.leases.remove(&container) {
                while let Some(mut lease) = leases.pop() {
                    lease.service = service.clone();
                    table.push(lease);
                }
            }
        }

        table
    }
}

#[derive(Debug, Deserialize, Tabled)]
struct Service {
    image: String,
    automatic_updates: bool,
    #[field(display_with = "display_option")]
    domain: Option<String>,
    #[field(display_with = "display_option")]
    deployment_id: Option<String>,
    #[header(inline("dependency."))]
    dependencies: ServiceDependencies,
}

#[derive(Debug, Deserialize, Tabled)]
struct ServiceDependencies {
    postgres: bool,
    redis: bool,
}

fn display_option(o: &Option<String>) -> String {
    match o {
        Some(s) => s.to_owned(),
        None => "[none]".to_owned(),
    }
}
