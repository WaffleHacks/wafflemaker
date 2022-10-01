use eyre::{Result, WrapErr};
use sentry::{ClientOptions, IntoDsn};
use structopt::StructOpt;
use tabled::{object::Rows, Alignment, Modify, Padding, Style};

mod args;
mod commands;
mod http;

use args::Args;

fn main() -> Result<()> {
    // Setup traceback
    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }
    color_eyre::install()?;

    // Parse the CLI
    let cli = Args::from_args();

    // Initialize sentry
    let _guard = sentry::init(ClientOptions {
        dsn: option_env!("SENTRY_DSN").into_dsn()?,
        release: sentry::release_name!(),
        attach_stacktrace: true,
        ..Default::default()
    });

    // Build the HTTP client
    let client = http::Client::new(cli.address, &cli.token).wrap_err("failed to build client")?;

    let content = cli.cmd.subcommand().execute(client)?;
    if let Some(mut content) = content {
        println!(
            "{}",
            content.with(Style::psql()).with(
                Modify::new(Rows::new(1..))
                    .with(Alignment::left())
                    .with(Padding::new(1, 1, 0, 0))
            )
        );
    }

    Ok(())
}
