use eyre::{Result, WrapErr};
use structopt::StructOpt;
use tabled::{Alignment, Indent, Modify, Row, Style};

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

    // Build the HTTP client
    let client = http::Client::new(cli.address, &cli.token).wrap_err("failed to build client")?;

    let content = cli.cmd.subcommand().execute(client)?;
    println!(
        "{}",
        content.with(Style::psql()).with(
            Modify::new(Row(1..))
                .with(Alignment::left())
                .with(Indent::new(1, 1, 0, 0))
        )
    );

    Ok(())
}
