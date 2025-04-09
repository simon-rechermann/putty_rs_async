use clap::Parser;
use putty_rs::ui::cli::cli_commands;
use putty_rs::utils::logging::init_logging;

#[tokio::main]
async fn main() {
    init_logging();
    let args = cli_commands::Args::parse();
    if let Err(e) = cli_commands::run_cli(args).await {
        eprintln!("CLI error: {:?}", e);
        std::process::exit(1);
    }
}
