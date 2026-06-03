## 1. Workspace Setup

### Add drift-auth to workspace

**File:** `/coven/drift/Cargo.toml`

```toml
[workspace]
members = ["drift-cli", "drift-node", "drift-coord", "drift-proto", "drift-auth"]  # Add "drift-auth"
resolver = "2"
```

**Checklist:** Setup for all items
