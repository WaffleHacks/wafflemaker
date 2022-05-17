use super::*;
use crate::http::service_path;
use serde::Serialize;

// wafflectl delete <lease {id} {service}|service {name}>
#[derive(Debug, StructOpt)]
pub enum Delete {
    /// Stop tracking a lease
    ///
    /// Removes a lease by its full ID from a particular service,
    /// preventing it from being renewed or managed by the
    /// service's lifecycle. Does not error if a particular
    /// lease cannot be found.
    Lease {
        /// The service to delete the lease from
        service: String,
        /// The ID of the lease to remove
        id: String,
    },
    /// Delete a service
    ///
    /// Remove a service's currently running deployment (if any).
    /// Does not modify any state in the source repository.
    Service {
        /// The name of the service
        name: String,
    },
}

impl Subcommand for Delete {
    /// Handle the subcommand call
    fn execute(&self, client: Client) -> Result<Option<Table>> {
        match self {
            Self::Lease { id, service } => {
                let params = Lease { id };
                client.delete(&["leases", service.as_str()], Some(params))?;
            }
            Self::Service { name } => {
                client.delete::<_, &str>(&service_path("services", &name), None)?;
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Serialize)]
struct Lease<'i> {
    id: &'i str,
}
