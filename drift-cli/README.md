---
library_name: drift-cli
license_name: MPL-2.0 + Commons Clause 1.0
language: en
compatibility:
  - macos
  - linux
---

# drift-cli

Drift-cli is the command-line interface for joining and managing COVEN distributed training sessions as a peer node.

## Development Notes

Library provides:

- CLI entry point (`drift`) for training coordination
- Commands for joining, leaving, and monitoring sessions
- Configuration of model serving parameters
- Integration with iroh for p2p networking
- Status reporting and session management

### Dependencies

Uses drift-proto for protocol definitions, clap for argument parsing, and libc for system integration.

## Requirements

### Hardware

- Any computer capable of running Rust with network connectivity.

### Software

- [`just`](https://github.com/casey/just#packages) to build library
- [cargo](https://rust-lang.org) Rust toolchain

### Experience

- Familiarity with command-line interfaces (CLI)

## Build

```
cd drift/drift-cli
cargo build --release
```

## Testing

```
cargo test
```