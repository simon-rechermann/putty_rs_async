use log::{info, error};

mod core;
mod connections;
mod ui;
mod utils;

use utils::logging::init_logging;

fn main() {
    init_logging();

    // For now, we just delegate to a CLI run:
    // If you’d like to parse command-line arguments, do that in `ui/cli/commands`.
    if let Err(e) = ui::cli::commands::run_cli() {
        error!("Error: {:?}", e);
    } else {
        info!("CLI run completed successfully.");
    }
}
