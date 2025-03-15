# putty-rs

putty-rs is a rust clone of [putty](https://www.putty.org/).
The complete documentation is available in docs/index.adoc

## Usage

```bash
cargo run -- --help
```

## Lib dependencies

### Ubuntu

sudo apt-get install libssl-dev

## Test program serial connection with putty as other end of virtual serial device

```bash
socat -d -d pty,raw,echo=0 pty,raw,echo=0 # Create two connected virtual serial devices e.g. /dev/pts/2 and /dev/pts/3
# Connect a programm like putty to /dev/pts/2 or just launch putty-rs twice
cargo run -- serial --port /dev/pts/2 # run putty-rs and connect it to /dev/pts/2
cargo run -- serial --port /dev/pts/3 # run putty-rs and connect it to /dev/pts/3
```
