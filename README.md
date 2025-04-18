# putty-rs

putty-rs is a rust clone of [putty](https://www.putty.org/).
The complete documentation is available in docs/index.adoc

## Usage

There is is command line interface (cli) and a graphical user interface (gui) available.

The gui does not expect any additional parameters.

```bash
cargo run --bin gui
```

The cli expects additional parameters. To get information about it you can run the following command.

```bash
cargo run --bin cli -- --help
```

## Lib dependencies

### Ubuntu

```bash
# For ssh2 crate
sudo apt-get install libssl-dev
# For tonic of you want to build the grpc server
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
