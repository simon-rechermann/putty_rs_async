# putty-rs

`putty-rs` is a terminal client with feature-gated serial, SSH, and optional profile storage support.

## Install

Install the default CLI with serial, SSH, and storage support:

```bash
cargo install putty-rs
```

Install a smaller serial-only build without storage:

```bash
cargo install putty-rs --no-default-features --features serial
```

Install an SSH-only build without storage:

```bash
cargo install putty-rs --no-default-features --features ssh
```

Install a serial-only build with storage support:

```bash
cargo install putty-rs --no-default-features --features serial,storage
```

Install an SSH-only build with storage support:

```bash
cargo install putty-rs --no-default-features --features ssh,storage
```

## CLI Usage

Show help:

```bash
putty-rs --help
```

Show transport-specific help:

```bash
putty-rs serial --help
putty-rs ssh --help
```

Show storage help when the CLI was built with the `storage` feature:

```bash
putty-rs storage --help
```

Open a serial connection:

```bash
putty-rs serial --port /dev/ttyUSB0 --baud 115200
```

### Example: Test With Virtual Serial Devices

On Unix-like systems, `socat` can create a connected pair of pseudo terminals. This is useful for testing `putty-rs` without physical serial hardware.

```bash
socat -d -d pty,raw,echo=0 pty,raw,echo=0
# Create two connected virtual serial devices, e.g. /dev/pts/2 and /dev/pts/3

# Connect a program such as PuTTY to one side, or run putty-rs twice:
putty-rs serial --port /dev/pts/2
putty-rs serial --port /dev/pts/3
```

Open an SSH connection:

```bash
putty-rs ssh --host 127.0.0.1 --username user
```

## Profiles

These commands are only available when the CLI was built with the `storage` feature.

List saved profiles:

```bash
putty-rs storage list
```

Save a serial profile:

```bash
putty-rs storage save-serial --name lab --port /dev/ttyUSB0 --baud 115200
```

Save an SSH profile:

```bash
putty-rs storage save-ssh --name pi --host 192.168.1.20 --username simon
```

Use a saved profile:

```bash
putty-rs storage use-profile --profile pi
```

Delete a saved profile:

```bash
putty-rs storage delete --name pi
```

## Terminal Controls

Exit an active session with:

```text
Ctrl+A, then x
```
