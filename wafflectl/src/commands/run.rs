use super::*;

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

impl RunSubcommand {
    /// Handle the subcommand call
    pub fn handle(self, client: Client, url: Url) -> Result<()> {
        todo!()
    }
}
