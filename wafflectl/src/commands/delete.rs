use super::*;

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
        /// The ID of the lease to remove
        id: String,
        /// The service to delete the lease from
        service: String,
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
    fn execute(&self, client: Client, url: Url) -> Result<Table> {
        todo!()
    }
}
