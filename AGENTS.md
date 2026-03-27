# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust workspace with four crates: `putty_core/` for connection logic, storage, and tests; `putty_cli/` for the terminal client; `putty_grpc_server/` for the gRPC API and proto build; and `putty_rs_web/` for the Rust web host. The React frontend lives in `webui/`. Protocol definitions are in `putty_grpc_server/proto/`, and architecture/config docs are under `docs/`.

## Build, Test, and Development Commands
Use workspace commands from the repository root unless noted otherwise.

- `cargo build --workspace`: build all Rust crates.
- `cargo test --all`: run the default unit and integration test suite.
- `cargo test -p putty_core --features hw-tests -- --nocapture`: run hardware-backed serial/SSH tests.
- `cargo fmt --all`: format Rust sources.
- `cargo clippy --all-targets --all-features -- -D warnings`: enforce lint-clean Rust builds.
- `cargo run --bin cli -- --help`: inspect CLI usage.
- `cargo run -p putty_grpc_server`: start the backend server.
- `cd webui && npm ci && npm run dev`: install frontend dependencies and start Vite.
- `cd webui && npm run build && npm run lint`: build and lint the React app.

## Coding Style & Naming Conventions
Rust uses the standard `rustfmt` style with 4-space indentation, `snake_case` for modules/functions, and `CamelCase` for types. Keep async boundaries explicit and prefer small modules below `putty_core/src/` grouped by domain (`connections/`, `storage/`, `utils/`). TypeScript in `webui/src/` follows ESLint defaults, 2-space indentation, and `PascalCase` component filenames such as `ProfilesModal.tsx`. Regenerate frontend proto bindings with `cd webui && npm run proto` after changing `putty_interface.proto`.

## Testing Guidelines
Integration tests live in `putty_core/tests/` and are named for the behavior they cover, for example `profile_store.rs` or `roundtrip_and_write.rs`. Keep new tests deterministic by using the helpers in `putty_core/tests/common/`. Reserve `hw_*` tests for cases that require serial devices, `socat`, or SSH services, and gate them behind the `hw-tests` feature.

## Commit & Pull Request Guidelines
Recent history favors short, imperative commit subjects such as `cargo clippy`, `de2fb55 Fix CORS issue`, and `Only info logging`. Keep commit titles concise, present tense, and focused on one change. Pull requests should summarize the user-visible impact, note any cross-crate or proto changes, list verification commands run, and include screenshots only when `webui/` behavior changes.
