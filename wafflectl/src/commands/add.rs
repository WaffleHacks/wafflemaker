use structopt::StructOpt;

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
