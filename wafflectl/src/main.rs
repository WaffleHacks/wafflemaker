use eyre::{Result, WrapErr};
use structopt::StructOpt;

mod args;

use args::Args;

fn main() -> Result<()> {
    // Setup traceback
    if std::env::var("RUST_SPANTRACE").is_err() {
        std::env::set_var("RUST_SPANTRACE", "0");
    }
    color_eyre::install()?;

    // Parse the CLI
    let cli = Args::from_args();
    println!("{:?}", cli);

    Ok(())
}
