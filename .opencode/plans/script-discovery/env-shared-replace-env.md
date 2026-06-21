# Plan: Replace .env with .env.shared for Spawn Command Environment Variables

## Overview
Change drift coordinator to load environment variables from `.env.shared` in current working directory instead of `.env`. The `--env-file` argument remains available to override the default.

## Requirements
- **Default file**: `.env.shared` in current working directory (where `drift train` is run)
- **Override**: `--env-file <path>` argument to specify custom path
- **Fallback**: If `.env.shared` doesn't exist and no `--env-file` provided → no env vars (graceful)
- **Scope**: Coordinator only (`drift-cli`), node receives env vars via `TrainConfig` (no file reading on node side)

---

## Stage 1: Update Environment File Resolution in Coordinator

### Tasks
| Step | Description | Status | Location |
|------|-------------|--------|----------|
| 1.1 | Modify `coord::train()` to check for `.env.shared` when `--env-file` not provided | ☐ 0% | `drift-cli/src/coord.rs:35-46` |
| 1.2 | Update comment/docstring to reference `.env.shared` | ☐ 0% | `drift-cli/src/coord.rs:654` |
| 1.3 | Update warning message to reference `.env.shared` as default | ☐ 0% | `drift-cli/src/coord.rs:40` |

**Stage Completion**: [ ] 0%

---

## Stage 2: Update Documentation

### Tasks
| Step | Description | Status | Location |
|------|-------------|--------|----------|
| 2.1 | Update README.md to document `.env.shared` default | ☒ 100% | `README.md` |
| 2.2 | Update CONTRIBUTING.md if env var docs exist | ☒ 100% | `CONTRIBUTING.md` |

**Stage Completion**: [x] 100%

---

## Stage 3: Testing

### Tasks
| Step | Description | Status | Location |
|------|-------------|--------|----------|
| 3.1 | Test: `.env.shared` in cwd loads automatically | ☐ 0% | Manual verification |
| 3.2 | Test: `--env-file` overrides `.env.shared` | ☐ 0% | Manual verification |
| 3.3 | Test: no `.env.shared` + no `--env-file` = no env vars (no error) | ☐ 0% | Manual verification |
| 3.4 | Test: env vars passed to node via `TrainConfig` | ☐ 0% | Manual verification |

**Stage Completion**: [ ] 0%

---

## Verification State
All stages complete when:
- [ ] Coordinator loads `.env.shared` from cwd by default
- [ ] `--env-file` argument overrides default
- [ ] Graceful fallback when no env file exists
- [ ] Env vars transmitted to nodes via `TrainConfig.env_vars`
- [ ] All tests pass

---

## Implementation Notes

### Current Flow
```
drift train --env-file <path> 
  → coord::train() parses env file 
  → env_vars stored in TrainConfig 
  → sent to nodes via DriftMessage::TrainConfig 
  → node uses env_vars in spawn_cmd
```

### New Default Behavior
```
drift train 
  → check --env-file argument
    → if provided: use that path
    → if not: check .env.shared in cwd
      → if exists: parse it
      → if not: no env vars (graceful)
  → continue as before...
```

### Files to Modify
1. **drift-cli/src/coord.rs** (lines 35-46): Add default `.env.shared` resolution
2. **drift-cli/src/coord.rs** (line 40): Update warning message
3. **drift-cli/src/coord.rs** (line 654): Update docstring

### Files NOT to Modify
- **drift-node/src/**: Node already receives env_vars via TrainConfig, no file reading needed
- **drift-proto/src/lib.rs**: TrainConfig struct already has env_vars field, no change needed

---

## Stage Completion Status
- [ ] Stage 1: Coordinator env file resolution
- [ ] Stage 2: Documentation updates
- [ ] Stage 3: Testing and verification
