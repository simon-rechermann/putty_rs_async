use log::{info, error};
use std::env;

mod core;
mod connections;
mod ui;
mod utils;

use utils::logging::init_logging;

fn main() {
    init_logging();

    // Check for --gui
    let args: Vec<String> = env::args().collect();
    let use_gui = args.iter().any(|arg| arg == "--gui");

    if use_gui {
        // Launch GUI
        match ui::gui::window::launch_gui() {
            Ok(_) => info!("GUI closed gracefully."),
            Err(e) => error!("Failed to launch GUI: {:?}", e),
        }
    } else {
        // Run CLI
        if let Err(e) = ui::cli::cli::run_cli() {
            error!("CLI error: {:?}", e);
        } else {
            info!("CLI run completed successfully.");
        }
    }
}
