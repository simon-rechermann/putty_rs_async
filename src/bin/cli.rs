use clap::Parser;
use putty_rs::ui::cli::cli_commands;
use putty_rs::utils::logging::init_logging;

fn main() {
    init_logging();
    // Parse CLI-specific arguments (you might remove the gui flag now)
    let args = cli_commands::Args::parse();

    if let Err(e) = cli_commands::run_cli(args) {
        eprintln!("CLI error: {:?}", e);
        std::process::exit(1);
    }
}