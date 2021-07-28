use structopt::StructOpt;
use url::Url;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "wafflectl",
    about = "Manages the WaffleMaker deployment engine"
)]
pub struct Args {
    /// The address where the WaffleMaker management interface is located
    #[structopt(
        short,
        long,
        default_value = "http://127.0.0.1:8001",
        env = "WAFFLECTL_ADDRESS"
    )]
    pub address: Url,
    /// The token to authenticate with
    #[structopt(short, long, env = "WAFFLECTL_TOKEN", hide_env_values = true)]
    pub token: String,

    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Add an instance of an object
    Add(AddSubcommand),
    /// Delete an object
    Delete(DeleteSubcommand),
    /// Get details about an object
    Get(GetSubcommand),
    /// Run an object
    Run(RunSubcommand),
}

// wafflectl add lease {service} {id} {ttl} [last updated]
#[derive(Debug, StructOpt)]
pub enum AddSubcommand {
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

// wafflectl delete <lease {id} {service}|service {name}>
#[derive(Debug, StructOpt)]
pub enum DeleteSubcommand {
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

// wafflectl get <deployments|leases|services|service {name}>
#[derive(Debug, StructOpt)]
pub enum GetSubcommand {
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

// wafflectl run <deployment {commit}|service {name}>
#[derive(Debug, StructOpt)]
pub enum RunSubcommand {
    /// Run a deployment
    ///
    /// Run a deployment given the commit hash of the before
    /// state to compare with what is currently on disk.
    Deployment {
        /// The commit hash of the before state
        before: String,
    },
    /// Run a deploy of a service
    ///
    /// Deploy a service using the current configuration stored on disk.
    Service {
        /// The name of the service
        name: String,
    },
}
