---
library_name: drift-proto
license_name: MPL-2.0 + Commons Clause 1.0
language: en
compatibility:
  - macos
  - linux
---

# drift-proto

Drift-proto defines the communication protocols and message types used by COVEN for distributed training coordination.

## Development Notes

Library provides:

- Protocol buffer definitions for training operations
- Message serialization (JSON/TOML)
- Request/response type definitions
- Integration with drift-auth for authenticated messages
- Async trait abstractions for protocol handlers

### Dependencies

Depends on drift-auth for authentication primitives and serde for serialization.

## Build

```
cd drift/drift-proto
cargo build --release
```

## Testing

```
cargo test
```