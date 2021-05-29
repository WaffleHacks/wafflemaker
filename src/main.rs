use anyhow::{Context, Result};
use structopt::StructOpt;
use tokio::fs;

mod args;
mod config;
mod http;

use args::Args;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the cli
    let cli = Args::from_args();

    // Get the configuration
    let configuration = config::parse(cli.config)
        .await
        .context("Failed to load configuration")?;
    let address = cli.address.unwrap_or(configuration.server.address);

    // Ensure the clone directory exists
    if !configuration.github.clone_to.exists() {
        fs::create_dir_all(&configuration.github.clone_to)
            .await
            .context("Failed to create configuration clone directory")?;
    }

    // Setup the routes and launch the server
    let routes = http::routes();
    warp::serve(routes).run(address).await;

    Ok(())
}
