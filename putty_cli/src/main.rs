mod logging;
mod ui;

use crate::logging::init_logging;
use crate::ui::cli;
use clap::Parser;

#[tokio::main]
async fn main() {
    init_logging();
    let args = cli::Args::parse();
    if let Err(e) = cli::run_cli(args).await {
        eprintln!("CLI error: {e:?}");
        std::process::exit(1);
    }
}
