# How to use putty-rs

```bash
socat -d -d pty,raw,echo=0 pty,raw,echo=0 # Create two connected virtual serial devices e.g. /dev/pts/2 and /dev/pts/3
# connect a programm like putty to /dev/pts/2
cargo run -- --port /dev/pts/3 # run putty-rs and connect it to /dev/pts/3
```