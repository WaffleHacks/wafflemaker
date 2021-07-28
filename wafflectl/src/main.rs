use eyre::{Result, WrapErr};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
};
use structopt::StructOpt;

mod args;
mod commands;

use args::{Args, Command};

fn main() -> Result<()> {
    // Setup traceback
    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }
    color_eyre::install()?;

    // Parse the CLI
    let cli = Args::from_args();

    // Build the HTTP client
    let headers = {
        let mut map = HeaderMap::new();
        map.insert(AUTHORIZATION, HeaderValue::from_str(&cli.token)?);
        map
    };
    let _client = Client::builder()
        .default_headers(headers)
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .wrap_err("failed to build client")?;

    // Run the desired command
    match cli.cmd {
        Command::Add(_) => {}
        Command::Delete(_) => {}
        Command::Get(_) => {}
        Command::Run(_) => {}
    }

    Ok(())
}
