//! Runs *once* whenever Cargo recompiles the binary.
//! 1. `npm ci`        – install packages (cached by GitHub Actions)
//! 2. `npm run build` – emit the static site into webui/dist
use std::{env, process::Command};

fn main() {
    // Watch the folder for changes by printing cargo:rerun-if-changed= line
    println!("cargo:rerun-if-changed=../webui");

    // › skip when the dev sets CARGO_FEATURE_SKIP_WEBUI
    if env::var_os("CARGO_FEATURE_SKIP_WEBUI").is_some() {
        return;
    }

    // 1. run `npm ci`   (installs exactly the versions in package-lock.json)
    // 2. run `npm run build`  (Vite → webui/dist)
    Command::new("npm").args(["ci"])
        .current_dir("../webui")
        .status().expect("npm ci failed");

    Command::new("npm").args(["run", "build"])
        .current_dir("../webui")
        .status().expect("npm run build failed");
}
