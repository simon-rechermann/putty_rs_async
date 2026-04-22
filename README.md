# putty-rs

putty-rs is a rust clone of [putty](https://www.putty.org/).
The complete documentation is available in docs/index.adoc

## Components

- `putty-rs`: CLI binary with feature-gated serial, SSH, and profile management/storage support
- `putty_core`: core transport and connection management logic
- `putty_storage`: profile storage and keyring integration
- `putty_grpc_server`: gRPC server exposing the backend for other clients
- `putty_rs_web`: single-binary web launcher that starts the backend and serves the web UI
- `webui`: React frontend for the browser-based UI

## CLI

CLI install, build, usage, profile handling, and terminal controls are documented in [putty_cli/README.md](putty_cli/README.md).

## Build CLI From Source

Default build with serial, SSH, and storage support:

```bash
cargo build -p putty-rs
```

Serial + SSH without storage:

```bash
cargo build --manifest-path putty_cli/Cargo.toml --no-default-features --features serial,ssh
```

Serial-only build without storage:

```bash
cargo build --manifest-path putty_cli/Cargo.toml --no-default-features --features serial
```

SSH-only build without storage:

```bash
cargo build --manifest-path putty_cli/Cargo.toml --no-default-features --features ssh
```

Serial-only build with storage:

```bash
cargo build --manifest-path putty_cli/Cargo.toml --no-default-features --features serial,storage
```

SSH-only build with storage:

```bash
cargo build --manifest-path putty_cli/Cargo.toml --no-default-features --features ssh,storage
```

## Dependencies

### Ubuntu

```bash
# For tonic if you want to build the gRPC server
sudo apt install protobuf-compiler
```

SSH is provided by the pure-Rust `russh` crate, so no native TLS/SSL libraries are required.

## Test the gRPC server

### With python CLI client

To test the gRPC server you can generate the python gRPC stubs and use the python_client,
which utilizes the proto interface to provide the same CLI like the rust cli.

First we create the python client stubs:

```bash
cd python_grpc_client
# Created and activate a venv to not make any global pip installations
python3 -m venv .venv
source .venv/bin/activate
# Install the required dependencies with pip
pip install grpcio grpcio-tools protoletariat

mkdir generated
# Generate stubs
python -m grpc_tools.protoc -I ../putty_grpc_server/proto --python_out=generated --grpc_python_out=generated putty_interface.proto
# Modify stubs to using protoletariat to make the imports relative see: https://github.com/protocolbuffers/protobuf/issues/1491
protol --create-package --in-place --python-out generated protoc --proto-path=../putty_grpc_server/proto putty_interface.proto

```

Now we can start the server in one terminal and connect with the python client it.
The python client provides the same cli as the rust cli.

```bash
# Run the server which listens for clients to connect
cargo run --bin putty_grpc_server
# In a new termial we can connect to the server with the python client
cd python_grpc_client
python grpc_cli_client.py serial --port /dev/pts/3
```

### With react webUI

For development of the webUI the following flow is usefull.

```bash
# ------------------------------------------------------------------
# 1) one-time project setup
# ------------------------------------------------------------------
cd webui                        # enter the UI workspace

npm ci                          # install *exactly* the versions in package-lock.json
                                # (→ reproducible, faster, and safer than `npm install`)

npm run proto                   # generate TypeScript stubs from putty_interface.proto
                                # (re-run this ONLY if the proto changes)

# ------------------------------------------------------------------
# 2) start the live-reload dev server (frontend) …
# ------------------------------------------------------------------
npm run dev                     # Vite dev server → http://localhost:5173
                                # proxies /rpc/* to the Rust backend

                                # ------------------------------------------------------------------
# 2b) Build and preview the release files
# ------------------------------------------------------------------
npm run build                     
npm run preview                 # Vite preview → http://localhost:4173

# ------------------------------------------------------------------
# 3) …and in another terminal start the backend
# ------------------------------------------------------------------
cargo run -p putty_grpc_server  # gRPC-Web + REST UI → http://127.0.0.1:50051

```

## Run integration tests

Here are some examples of how you can run the integration tests.
The hw-tests (test having this feature configured #![cfg(feature = "hw-tests")])
will only be compiled and run on unix machines.

Run all tests:

```bash
cargo test --features hw-tests
```

Run all test excluding the ones maked with #![cfg(feature = "hw-tests")]

```bash
cargo test
# As only the putty_core package(which consists out of the putty_core lib crate and the tests binary crates)
# has integration tests the following command does the same
cargo test -p putty_core
```

Run ssh hw-test and enable logging output

```bash
cargo test -p putty_core --test hw_ssh --features hw-tests -- --nocapture
```
