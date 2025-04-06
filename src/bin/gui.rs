use putty_rs::ui::gui::window;
use putty_rs::utils::logging::init_logging;

fn main() {
    init_logging();
    match window::launch_gui() {
        Ok(_) => println!("GUI closed gracefully."),
        Err(e) => {
            eprintln!("Failed to launch GUI: {:?}", e);
            std::process::exit(1);
        }
    }
}