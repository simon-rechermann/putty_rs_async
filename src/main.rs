use log::{info, error};
use clap::Parser;

mod core;
mod connections;
mod ui;
mod utils;

use utils::logging::init_logging;
use ui::cli::cli::Args;

fn main() {
    init_logging();

    // Parse arguments
    let args = Args::parse();

    if args.gui {
        // Launch GUI
        match ui::gui::window::launch_gui(args) {
            Ok(_) => info!("GUI closed gracefully."),
            Err(e) => error!("Failed to launch GUI: {:?}", e),
        }
    } else {
        // Run CLI
        if let Err(e) = ui::cli::cli::run_cli(args) {
            error!("CLI error: {:?}", e);
        } else {
            info!("CLI run completed successfully.");
        }
    }
}
