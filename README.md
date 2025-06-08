# putty-rs

putty-rs is a rust clone of [putty](https://www.putty.org/).
The complete documentation is available in docs/index.adoc

## Usage

There is is command line interface (cli) and a gRPC server to support language independet gRPC client
that implement the proto interface provides by the server.

The cli expects additional parameters. To get information about it you can run the following command.

```bash
cargo run --bin cli -- --help
```

## Dependencies

### Ubuntu

```bash
# For ssh2 crate
sudo apt-get install libssl-dev
# For tonic of you want to build the gRPC server
sudo apt install protobuf-compiler
```

## Test serial connection with putty or second putty-rs instance as other end of virtual serial device

```bash
socat -d -d pty,raw,echo=0 pty,raw,echo=0 # Create two connected virtual serial devices e.g. /dev/pts/2 and /dev/pts/3
# Connect a programm like putty to /dev/pts/2 or just launch putty-rs twice
cargo run --bin cli -- serial --port /dev/pts/2 # run putty-rs and connect it to /dev/pts/2
cargo run --bin cli -- serial --port /dev/pts/3 # run putty-rs and connect it to /dev/pts/3
```

## Test ssh connection

To be able to connect to a ssh server you need to specify some parameters.

```bash
cargo run --bin cli -- ssh --help
```

## Test the gRPC server

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
