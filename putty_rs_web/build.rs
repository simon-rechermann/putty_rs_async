//! Runs *once* whenever Cargo recompiles the binary.
//! 1. `npm ci`        – install packages (cached by GitHub Actions)
//! 2. `npm run build` – emit the static site into webui/dist
use std::{env, process::Command};

fn run_checked(cmd: &str, args: &[&str]) {
    let status = Command::new(cmd)
        .args(args)
        .current_dir("../webui")
        .status()
        .unwrap_or_else(|e| panic!("{cmd} {} failed to start: {e}", args.join(" ")));

    assert!(
        status.success(),
        "{cmd} {} exited with status {status}",
        args.join(" ")
    );
}

fn main() {
    // Watch the folder for changes by printing cargo:rerun-if-changed= line
    println!("cargo:rerun-if-changed=../webui");

    // › skip when the dev sets CARGO_FEATURE_SKIP_WEBUI
    if env::var_os("CARGO_FEATURE_SKIP_WEBUI").is_some() {
        return;
    }

    // 1. run `npm ci`   (installs exactly the versions in package-lock.json)
    // 2. run `npm run build`  (Vite → webui/dist)
    run_checked("npm", &["ci"]);
    run_checked("npm", &["run", "build"]);
}
