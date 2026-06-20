# Plan: Environment Variables for `ati plug` Spawn Command

## Overview
Add support for environment variables when launching `ati_plug` spawn commands. Drift should load environment variables from:
1. `.env` file in current working directory (automatic)
2. Optional `--env` flag specifying a custom env file path

Environment variables should be prepended inline to the spawn command using shell syntax.

## Requirements
- **Env file format**: Simple `KEY=value` pairs (one per line)
- **Merging**: Merge `.env` from cwd with `--env` file (both apply)
- **Spawn command format**: Inline shell syntax: `VAR1=val1 VAR2=val2 command`

---

## Stage 1: Parse Environment Files

### Tasks
| Step | Description | Status | Location |
|------|-------------|--------|----------|
| 1.1 | Create `parse_env_file(path)` function to read KEY=value pairs | ✅ 100% | `drift-cli/src/node.rs:748-767` |
| 1.2 | Handle comments (lines starting with #) | ✅ 100% | `drift-cli/src/node.rs:752` |
| 1.3 | Handle empty lines | ✅ 100% | `drift-cli/src/node.rs:752` |
| 1.4 | Return list of (key, value) pairs | ✅ 100% | `drift-cli/src/node.rs:753-764` |

**Stage Completion**: [x] 100%

---

## Stage 2: Add `--env` Flag to CLI

### Tasks
| Step | Description | Status | Location |
|------|-------------|--------|----------|
| 2.1 | Add `env_file: Option<String>` field to `Train` command in `cli.rs` | ✅ 100% | `drift-cli/src/cli.rs:48` |
| 2.2 | Pass `env_file` parameter through to `coord::train()` | ✅ 100% | `drift-cli/src/main.rs:34-51` |
| 2.3 | Update `coord::train()` signature to accept `env_file` parameter | ✅ 100% | `drift-cli/src/coord.rs:25` |

**Stage Completion**: [x] 100%

---

## Stage 3: Load and Merge Environment Variables

### Tasks
| Step | Description | Status | Location |
|------|-------------|--------|----------|
| 3.1 | In `coord::train()`, load `.env` from cwd if exists | ✅ 100% | `drift-cli/src/node.rs:406-408` |
| 3.2 | Load custom env file if `--env` flag provided | ✅ 100% | `drift-cli/src/node.rs:409-411` |
| 3.3 | Merge both sources (--env overrides .env) | ✅ 100% | `drift-cli/src/node.rs:406-411` |
| 3.4 | Store merged env vars in `TrainConfig` struct | ✅ 100% | `drift-proto/src/lib.rs:313` |

**Stage Completion**: [x] 100%

---

## Stage 4: Apply Environment Variables to Spawn Command

### Tasks
| Step | Description | Status | Location |
|------|-------------|--------|----------|
| 4.1 | Modify `resolve_entrypoint_to_spawn_cmd()` to accept env vars | ✅ 100% | `drift-cli/src/node.rs:406-417` |
| 4.2 | Prepend env vars inline: `VAR1=val1 VAR2=val2 command` | ✅ 100% | `drift-cli/src/node.rs:412-417` |
| 4.3 | Copy same logic to `drift-cli/src/node.rs` if needed | ✅ 100% | `drift-cli/src/node.rs:402-420` |

**Stage Completion**: [x] 100%

---

## Stage 5: Testing

### Tasks
| Step | Description | Status | Location |
|------|-------------|--------|----------|
| 5.1 | Test: `.env` file in cwd applies to spawn_cmd | ✅ 100% | Manual verification |
| 5.2 | Test: `--env` flag loads custom env file | ✅ 100% | Manual verification |
| 5.3 | Test: merge both sources correctly | ✅ 100% | Manual verification |
| 5.4 | Test: no env file = no env vars in command | ✅ 100% | Manual verification |

**Stage Completion**: [x] 100%

---

## Verification State
All stages complete when:
- [x] `.env` file in cwd automatically loaded
- [x] `--env <path>` flag loads additional env vars
- [x] Both sources merged correctly
- [x] Spawn command includes inline env vars: `STEP=20 DTYPE=torch.float32 ati_plug`
- [x] All tests pass
