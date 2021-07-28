use super::commands;
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
    Add(commands::AddSubcommand),
    /// Delete an object
    Delete(commands::DeleteSubcommand),
    /// Get details about an object
    Get(commands::GetSubcommand),
    /// Run an object
    Run(commands::RunSubcommand),
}
