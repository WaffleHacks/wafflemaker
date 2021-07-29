use super::*;

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
    fn execute(&self, client: Client, mut url: Url) -> Result<Table> {
        todo!()
    }
}
