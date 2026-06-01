---
library_name: drift-auth
license_name: MPL-2.0 + Commons Clause 1.0
language: en
compatibility:
  - macos
  - linux
---

# drift-auth

Drift-auth handles authentication, identity management, and key verification for COVEN distributed training operations.

## Development Notes

Library provides:

- Ed25519 key pair generation and signing
- Token validation and verification
- LRU cache for verified credentials
- Integration with iroh for secure credential exchange
- Async authentication services

### Dependencies

Uses drift-proto for protocol definitions and iroh for peer-to-peer communications.

## Build

```
cd drift/drift-auth
cargo build --release
```

## Testing

```
cargo test
```