# RepoCommit Signature Verification Failure - Diagnosis

## Problem Summary

When running `drift train --peers <node1>,<node2> --repo <url>`, the coordinator receives `RepoCommit` messages from nodes, decrypts them successfully (logs show matching commit hashes), but then **fails signature verification** and sends `TrainingCancel` with error `SIG_VERIFY_FAILED`.

## Root Cause

### Empty Signature in drift-cli

**File:** `drift-cli/src/node.rs:267-291`

```rust
let mut signing_key = Vec::new(); // placeholder for forwarded signing key from drift-node
let mut signature = if signing_key.len() == 32 {
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&signing_key);
    let keypair = SigningKey::from_bytes(&seed);
    sign_repo_commit(&node_id_str, &commit_hash, &repo_url, &keypair).to_bytes().to_vec()
} else {
    Vec::new()  // ← Empty signature sent to coordinator
};
```

**Problem:** `signing_key` is hardcoded as an **empty vector**, so `signing_key.len() == 32` is always `false`, resulting in an **empty signature** being sent.

### Coordinator Verification Failure

**File:** `drift-cli/src/coord.rs:576-581`

```rust
fn verify_repo_commit(commit: &drift_proto::RepoCommit, node_id: &str) -> Result<()> {
    let pubkey = PublicKey::from_str(node_id)
        .map_err(|_| anyhow::anyhow!("Invalid node ID: {}", node_id))?;
    drift_auth::crypto::verify_repo_commit(&pubkey, node_id, &commit.commit, &commit.repo_url, &commit.signature)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
```

**File:** `drift-auth/src/crypto.rs:56-72`

```rust
pub fn verify_repo_commit(
    pubkey: &iroh::PublicKey,
    node_id: &str,
    commit: &str,
    repo_url: &str,
    signature: &[u8],
) -> Result<(), CryptoError> {
    let pk_bytes = pubkey.as_bytes();
    let pk = VerifyingKey::from_bytes(pk_bytes)
        .map_err(|_| CryptoError::InvalidKey)?;

    let message = format!("{}|{}|{}", node_id, commit, repo_url);
    let sig = Signature::from_slice(signature)  // ← Fails here with empty signature
        .map_err(|_| CryptoError::VerificationFailed)?;
    pk.verify(message.as_bytes(), &sig)
        .map_err(|_| CryptoError::VerificationFailed)
}
```

**Failure point:** `Signature::from_slice(signature)` with an empty byte array returns an error, which gets mapped to `VerificationFailed`.

## Why Logs Show Matching Commits

The coordinator logs show:
```
INFO drift::coord: decrypted repocommit hash node=c1702b6171f1... commit=bb8f3f42483c9c0d1fe1426248d70a30ad08cac6
INFO drift::coord: decrypted repocommit hash node=7e69a6ad8e75... commit=bb8f3f42483c9c0d1fe1426248d70a30ad08cac6
```

This is **before** signature verification - it's just logging the `commit` field from the decrypted `RepoCommit` struct. The commit hashes match because both nodes correctly read the git commit. The **signature verification** is a separate cryptographic step that proves the node's identity.

## Correct Implementation (drift-node)

**File:** `drift-node/src/network.rs:95-104`

```rust
let secret_key = endpoint.secret_key();
let message = format!("{}|{}|{}", node_id, commit, repo_url);
let signature = secret_key.sign(message.as_bytes()).to_bytes().to_vec();
tracing::info!(signature_len = signature.len(), "signature length");
let repo_commit = RepoCommit {
    commit,
    repo_url,
    signature,
};
write_message(&mut send, &DriftMessage::RepoCommit(repo_commit)).await?;
```

**Key difference:** `drift-node` correctly accesses `endpoint.secret_key()` to sign the message.

## Why drift-auth Doesn't Have the Signing Key

`drift-auth` is a **library crate** that provides authentication primitives (signing, verification, aggregation functions). It does **not** own or create iroh endpoints.

The signing key belongs to the **iroh endpoint**, which is created in:
- `drift-node/src/network.rs` - correctly accesses via `endpoint.secret_key()`
- `drift-cli/src/node.rs` - **fails** to access the key, uses empty placeholder

## Solution

### Option 1: Sign with endpoint's secret key (Recommended)

Modify `drift-cli/src/node.rs:267-291` to match `drift-node`'s implementation:

```rust
DriftMessage::TrainConfig(config) => {
    info!(model = %config.model_path, epochs = config.epochs, "received config");
    let repo_url = config.train_repo_url.as_ref().ok_or_else(|| anyhow::anyhow!("No train_repo_url in config"))?;
    let repo_path = find_local_repo(repo_url).ok_or_else(|| anyhow::anyhow!("Repo not found locally"))?;
    let commit_hash = run_git_ls_remote(&repo_path).ok_or_else(|| anyhow::anyhow!("git ls-remote failed"))?;
    
    // Get signing key from endpoint (like drift-node does)
    let secret_key = endpoint.secret_key();
    let message = format!("{}|{}|{}", endpoint.id(), commit_hash, repo_url);
    let signature = secret_key.sign(message.as_bytes()).to_bytes().to_vec();
    
    let repo_commit = RepoCommit {
        commit: commit_hash,
        repo_url: repo_url.to_string(),
        signature,
    };
    write_message(&mut send, &DriftMessage::RepoCommit(repo_commit)).await?;
    info!("sent RepoCommit to coordinator");
    train_config = Some(config);
}
```

### Option 2: Use NodeIdentity from drift-auth

If `drift-cli` wants to use the `NodeIdentity` abstraction from `drift-auth`:

```rust
// At startup, create NodeIdentity
let node_identity = NodeIdentity::new(&endpoint.id().to_string())?;

// Then in handle_connection:
let signature = sign_repo_commit(&node_id_str, &commit_hash, &repo_url, &node_identity.keypair).to_bytes().to_vec();
```

**Tradeoff:** This creates a separate keypair from the endpoint's key, which may complicate identity management. Option 1 is simpler and consistent with `drift-node`.

## Files to Modify

1. **drift-cli/src/node.rs** (lines 267-291) - Fix signature generation in `handle_connection`

## Verification Steps

After fix:
1. Run `drift train --peers <node1>,<node2> --repo <url>`
2. Verify logs show successful signature verification (no `SIG_VERIFY_FAILED`)
3. Training should proceed past "Consistency Verification" stage
4. Optional: Add assertion that `signature.len() > 0` before sending `RepoCommit`

## Additional Notes

- The `SigningKey::from_bytes(&seed)` call on line 279 is dead code and should be removed
- The `sign_repo_commit` import on line 13 may no longer be needed if using `endpoint.secret_key()`
- Consider adding a guard/assertion that signature is non-empty before sending to coordinator
