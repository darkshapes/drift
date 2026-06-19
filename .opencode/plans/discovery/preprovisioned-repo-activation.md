# Plan: Pre-provisioned Repo Discovery & Venv Activation for \_ati_plug

## Context

When drift training runs with a provided repo:

1. Repo already exists in `~/.local/share/covn/<repo-suffix>` (pre-provisioned by nocturne-cli or other tool)
2. Node should discover `_ati_plug` entrypoint from that repo's `pyproject.toml`
3. Spawn command must source `.venv/bin/activate` then run the entrypoint

**Design Decisions:**

- Venv activation is **required** (error if `.venv/bin/activate` missing)
- Do **not** modify PYTHONPATH (entrypoint resolution alone is sufficient)
- Implement **stage-by-stage** with verification after each stage

## Current Flow (Broken)

In `drift-node/src/network.rs:154-168` (TrainingReady handler):

```rust
match crate::script_discovery::clone_repo_to_drift_cache(&url, &base).await {
    Ok(cloned_path) => {
        match crate::script_discovery::discover_script_entrypoint(&cloned_path) {
            Ok(entrypoint) => { ... }
        }
    }
}
```

**Problem**: Code clones to `~/.local/share/drift/<suffix>` instead of checking `~/.local/share/covn/<suffix>` first.

## Required Modifications

### Stage 1: Modify Repo Discovery Logic

**File**: `drift-node/src/script_discovery.rs`

**Changes**:

1. Add new function `find_preprovisioned_repo(url: &str, base: &Path)` that:
   - Extracts repo suffix from URL (reuse `repo_suffix_from_url`)
   - Checks `~/.local/share/covn/<suffix>` first
   - Falls back to `~/.local/share/drift/<suffix>` if not found
   - Returns error if neither exists

2. Modify `clone_repo_to_drift_cache` to:
   - First check if repo already exists in `~/.local/share/covn/<suffix>`
   - Only clone if not found anywhere
   - Return existing path if found

| Step | Description                                            | %   | Line |
| :--- | :----------------------------------------------------- | :-- | :--- |
| 1.1  | Add `find_preprovisioned_repo` function                | 0%  |      |
| 1.2  | Modify `clone_repo_to_drift_cache` to check covn first | 0%  |      |

- [ ] Stage 1 Complete

**Completion State**: `script_discovery.rs` can locate pre-provisioned repos in `~/.local/share/covn/` before falling back to cloning.

---

### Stage 2: Update Proto Definitions

**File**: `drift-proto/src/lib.rs` (line 259-300)

**Changes**:

1. Add field to `TrainConfig`:
   ```rust
   #[serde(default)]
   pub repo_path: Option<String>,
   ```

| Step | Description                                      | %   | Line |
| :--- | :----------------------------------------------- | :-- | :--- |
| 2.1  | Add `repo_path: Option<String>` to `TrainConfig` | 0%  |      |

- [ ] Stage 2 Complete

**Completion State**: `TrainConfig` includes `repo_path` field with serde defaults.

---

### Stage 3: Modify Network Handler

**File**: `drift-node/src/network.rs` (line 154-168)

**Changes**:

1. After discovering entrypoint, set both `script_entrypoint` AND `repo_path` in `cached_config`

| Step | Description                                             | %   | Line |
| :--- | :------------------------------------------------------ | :-- | :--- |
| 3.1  | Store discovered repo path in `cached_config.repo_path` | 0%  |      |

- [ ] Stage 3 Complete

**Completion State**: `network.rs` persists discovered repo path in cached config for later use.

---

### Stage 4: Modify Python Spawn Logic

**File**: `drift-node/src/training.rs` (line 72-180)

**Changes**:

1. Modify `spawn_training_with_progress` signature to accept `repo_path: Option<&str>`

2. Modify spawn logic (line 92-100) to source venv:
   ```rust
   // Build spawn command with venv activation if repo_path provided
   let use_shell = script.contains(' ') || repo_path.is_some();
   let mut base_cmd = tokio::process::Command::new("sh");
   if use_shell {
       let venv_activate = repo_path
           .map(|p| format!("source \"{}\"/.venv/bin/activate && ", p))
           .unwrap_or_default();
       base_cmd.arg("-c").arg(format!("{}python {}", venv_activate, script));
   } else {
       base_cmd.arg(script);
   }
   ```

| Step | Description                                     | %   | Line |
| :--- | :---------------------------------------------- | :-- | :--- |
| 4.1  | Update `spawn_training_with_progress` signature | 0%  |      |
| 4.2  | Add venv activation to spawn command            | 0%  |      |

- [ ] Stage 4 Complete

**Completion State**: `training.rs` spawns Python with venv activation when `repo_path` is provided.

---

### Stage 5: Update Resume Flow

**File**: `drift-node/src/main.rs` (line 97-118)

**Changes**:

1. When resuming from cache, extract `repo_path` from `config.repo_path`
2. Pass it to `spawn_training_with_progress`

| Step | Description                            | %   | Line |
| :--- | :------------------------------------- | :-- | :--- |
| 5.1  | Extract `repo_path` from cached config | 0%  |      |
| 5.2  | Pass `repo_path` to spawn function     | 0%  |      |

- [ ] Stage 5 Complete

**Completion State**: Resume flow passes `repo_path` through to training spawn.

---

## Verification Steps

1. `cargo check drift-proto && cargo check drift-node && cargo check drift-cli`
2. Test with pre-provisioned repo: `ls ~/.local/share/covn/<test-repo>` exists
3. Verify `.venv/bin/activate` is sourced before python spawn
4. Verify entrypoint runs with correct PYTHONPATH

## Notes

- Venv activation is **required** (error if `.venv/bin/activate` missing)
- Do **not** modify PYTHONPATH (entrypoint resolution alone is sufficient)
- Keep backward compatibility: if `repo_path` is None, spawn without venv activation
- Implement **stage-by-stage** with verification after each stage
