# putty-rs

`putty-rs` is a terminal client with serial and SSH support.

## Install

Install the default CLI with both serial and SSH support:

```bash
cargo install putty-rs
```

Install a smaller serial-only build:

```bash
cargo install putty-rs --no-default-features --features serial
```

Install an SSH-only build:

```bash
cargo install putty-rs --no-default-features --features ssh
```

## Usage

Show help:

```bash
putty-rs --help
```

Open a serial connection:

```bash
putty-rs serial --port /dev/ttyUSB0 --baud 115200
```

Open an SSH connection:

```bash
putty-rs ssh --host 127.0.0.1 --username user
```

List saved profiles:

```bash
putty-rs storage list
```
