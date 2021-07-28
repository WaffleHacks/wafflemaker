use structopt::StructOpt;

mod args;

use args::Args;

fn main() {
    let cli = Args::from_args();
    println!("{:?}", cli);
}
