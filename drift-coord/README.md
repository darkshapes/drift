---
library_name: drift-coord
license_name: MPL-2.0 + Commons Clause 1.0
language: en
compatibility:
  - macos
  - linux
---

# drift-coord

Drift-coord is the coordination layer that manages peer discovery, session negotiation, and distributed training orchestration across COVEN nodes.

## Development Notes

Library provides:

- Peer-to-peer node coordination via iroh
- Session management and participant tracking
- Time-synchronized scheduling for training rounds
- Cryptographic verification of peer identities
- Tracing and logging for distributed operations

### Dependencies

Uses drift-proto for message protocols, drift-auth for identity verification, and clap for CLI argument parsing.

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
cd drift/drift-coord
cargo build --release
```

## Testing

```
cargo test
```