# Plan: Make `--repo` Optional in `drift-cli` Train Command

## Context

The `drift train` command currently requires `--repo` as a mandatory argument. The goal is to make it optional, allowing users to run training without specifying a repository (e.g., `drift train --peers ...`).

---

## Stage 1: Make `--repo` Optional in CLI Definition

**Objective:** Change the `repo` field from required `String` to optional `Option<String>`.

| Step | Action | Line(s) | Pct |
|------|--------|--------|-----|
| 1.1 | Read `drift-cli/src/main.rs` to understand current `Train` command definition | 29-73 | 0% |
| 1.2 | Change `repo: String` → `repo: Option<String>` at CLI argument level | 32 | 0% |
| 1.3 | Update `coord::train(...)` call to pass `repo.clone()` only when Some | 105-118 | 0% |
| 1.4 | Add validation: if `repo` is None, use a default or cached value | — | 0% |
| 1.5 | Mark stage complete | — | 0% |

**Verification:** `drift train --peers id1,id2` does not error on missing `--repo`.

---

## Stage 2: Update `coord::train` Signature and Logic

**Objective:** Propagate `Option<String>` through the coordinate module.

| Step | Action | Line(s) | Pct |
|------|--------|--------|-----|
| 2.1 | Read `drift-cli/src/coord.rs` to understand `train()` function signature | 12-24 | 0% |
| 2.2 | Change `repo: String` → `repo: Option<String>` in function signature | 13 | 0% |
| 2.3 | Update `TrainConfig` construction to use `train_repo_url: repo` (Option) | 54-67 | 0% |
| 2.4 | Update all `broadcast_training_cancel(...)` calls to handle optional repo | 129, 141, 155, 169 | 0% |
| 2.5 | Add fallback logic: if `repo` is None, use a default value or error | — | 0% |
| 2.6 | Mark stage complete | — | 0% |

**Verification:** `cargo build -p drift-cli` succeeds without errors.

---

## Stage 3: Handle Optional Repo in Node/Protocol

**Objective:** Ensure nodes can operate without a repo URL.

| Step | Action | Line(s) | Pct |
|------|--------|--------|-----|
| 3.1 | Read `drift-cli/src/node.rs` - `handle_connection()` uses `config.train_repo_url` | 264-266 | 0% |
| 3.2 | Update `handle_connection()` to handle `train_repo_url: Option<String>` | 265 | 0% |
| 3.3 | If `train_repo_url` is None, skip RepoCommit verification | 264-275 | 0% |
| 3.4 | Update `broadcast_training_cancel` in `coord.rs` to handle None | 412-427 | 0% |
| 3.5 | Mark stage complete | — | 0% |

**Verification:** Full integration test: coordinator starts without `--repo`, node connects, training proceeds.

---

## Stage 4: Add Repo Caching (Optional Enhancement)

**Objective:** Cache the last-used repo URL to use as default when `--repo` is omitted.

| Step | Action | Line(s) | Pct |
|------|--------|--------|-----|
| 4.1 | Implement cache read/write in `coord.rs` (similar to `cli.rs` in nocturne-cli) | — | 0% |
| 4.2 | On train start, if `repo` is None, try to read from cache | — | 0% |
| 4.3 | On successful train, cache the repo URL | — | 0% |
| 4.4 | Mark stage complete | — | 0% |

**Verification:** `drift train --peers id1,id2 --repo exo/foo` followed by `drift train --peers id1,id2` uses cached repo.

---

## Summary

- **Files to edit:** `drift-cli/src/main.rs`, `drift-cli/src/coord.rs`, `drift-cli/src/node.rs`
- **Core change:** `repo: String` → `repo: Option<String>` with fallback logic
- **Optional:** Repo caching for better UX

---

## Notes

- The `drift_proto` crate defines `TrainConfig` with `train_repo_url: Option<String>` — already compatible with optional repo.
- When `repo` is None and no default/cache exists: **FAIL with message:**
  `"no repo specified. Use --repo <url> or run a training session first to cache a repo"`