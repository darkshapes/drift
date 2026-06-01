---
library_name: drift-node
license_name: MPL-2.0 + Commons Clause 1.0
language: en
compatibility:
  - macos
  - linux
---

# drift-node

Drift-node implements the COVEN node runtime that participates in distributed training sessions as a compute peer.

## Development Notes

Library provides:

- Node initialization and lifecycle management
- Protocol handling for training coordination messages
- Integration with iroh for peer-to-peer networking
- VRAM and memory tracking for model allocation
- Status reporting and health monitoring

### Dependencies

Uses drift-proto for protocol definitions and clap for CLI configuration.

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
cd drift/drift-node
cargo build --release
```

## Testing

```
cargo test
```