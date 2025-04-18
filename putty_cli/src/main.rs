mod ui;

use clap::Parser;
use crate::ui::cli;
use putty_core::utils::logging::init_logging;

#[tokio::main]
async fn main() {
    init_logging();
    let args = cli::Args::parse();
    if let Err(e) = cli::run_cli(args).await {
        eprintln!("CLI error: {e:?}");
        std::process::exit(1);
    }
}