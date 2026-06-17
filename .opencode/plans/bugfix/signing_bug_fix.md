# Fix: Signing Bug in Phase 1 - RepoCommit Signing with Iroh Keypair

## Bug Diagnosis

In `drift-node/src/network.rs:92-97`, the code incorrectly converts `iroh::SecretKey` to `ed25519_dalek::SigningKey`:

```rust
let secret_key = endpoint.secret_key();
let key_bytes = secret_key.to_bytes();
let mut seed = [0u8; 32];
seed.copy_from_slice(&key_bytes);
let keypair = SigningKey::from_bytes(&seed);  // BUG: returns Result, not SigningKey
let signature = sign_repo_commit(node_id, &commit, &repo_url, &keypair).to_bytes().to_vec();
```

**Problems:**

1. `SigningKey::from_bytes()` returns `Result<SigningKey, Error>` but is treated as returning `SigningKey` directly
2. Unnecessary conversion - iroh's `SecretKey` has its own `sign()` method
3. The `sign_repo_commit` function expects a generic `Signer` trait, but the conversion breaks the type system

---

## Stage 1: Fix RepoCommit signing to use iroh SecretKey directly

### Step-by-step

1. In `drift-node/src/network.rs`, locate the `handle_connection` function
2. Find the TrainConfig handling block (lines ~88-112)
3. Replace the incorrect SigningKey conversion with direct iroh signing:
   - Get `secret_key = endpoint.secret_key()`
   - Build message: `format!("{}|{}|{}", node_id, commit, repo_url)`
   - Sign: `secret_key.sign(message.as_bytes())`
   - Convert signature to vec: `.to_bytes().to_vec()`
4. Update imports if needed
5. Remove unused `ed25519_dalek::SigningKey` import if no longer needed

### Completion tracking

| Step | Description                        | %    | Line refs                          |
| ---- | ---------------------------------- | ---- | ---------------------------------- |
| 1.1  | Use iroh SecretKey.sign() directly | 100% | `drift-node/src/network.rs:92-97`  |
| 1.2  | Remove unnecessary dalek import    | 100% | `drift-node/src/network.rs:6`      |
| 1.3  | Verify build compiles              | 0%   | `cargo build --package drift-node` |

**Stage 1 complete when:** `cargo build --package drift-node` succeeds and RepoCommit messages are signed using `iroh::SecretKey.sign()` directly.

---

## Stage 2: Verify coordinator-side verification still works

### Step-by-step

1. In `drift-coord/src/main.rs`, verify `verify_repo_commit` handles `Vec<u8>` signature
2. Check that iroh's `Signature.to_bytes()` output is compatible
3. Run `cargo test --package drift-auth` to verify crypto tests

### Completion tracking

| Step | Description                           | %    | Line refs                         |
| ---- | ------------------------------------- | ---- | --------------------------------- |
| 2.1  | Verify signature format compatibility | 100% | `drift-auth/src/crypto.rs:56-72`  |
| 2.2  | Run crypto tests                      | 100% | `cargo test --package drift-auth` |

**Stage 2 complete when:** All `drift-auth` crypto tests pass with the updated signing.

---

## Stage 3: Integration verification

### Step-by-step

1. Build entire workspace: `cargo build --workspace`
2. Run all tests: `cargo test --workspace`
3. Verify end-to-end TrainConfig→RepoCommit flow works

### Completion tracking

| Step | Description             | %   | Line refs                 |
| ---- | ----------------------- | --- | ------------------------- |
| 3.1  | Full workspace build    | 0%  | `cargo build --workspace` |
| 3.2  | Full workspace test     | 0%  | `cargo test --workspace`  |
| 3.3  | Manual integration test | 0%  | Coordinator + node test   |

**Stage 3 complete when:** Build succeeds, all tests pass, and end-to-end flow works.

---

## Overall Progress

- [ ] Stage 1: Fix signing to use iroh SecretKey directly
- [ ] Stage 2: Verify coordinator verification works
- [ ] Stage 3: Full integration verification
