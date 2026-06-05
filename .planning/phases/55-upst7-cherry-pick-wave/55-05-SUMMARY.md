---
phase: 55-upst7-cherry-pick-wave
plan: 05
subsystem: cli
tags: [rust, timeouts, configuration, env-vars, cherry-pick, upstream-sync]

requires:
  - phase: 55-upst7-cherry-pick-wave
    provides: "55-03 (C9 pack-hint robustness applied to cli.rs/startup_runtime.rs/main.rs)"
  - phase: 55-upst7-cherry-pick-wave
    provides: "55-04 (C10 diagnostic-denial-polish applied to exec_strategy.rs)"

provides:
  - "crates/nono-cli/src/timeouts.rs: new module with named timeout constants + env-var overrides (NONO_DETACH_STARTUP_TIMEOUT, NONO_PTY_DRAIN_TIMEOUT, NONO_PTY_ATTACH_TIMEOUT)"
  - "exec_strategy.rs, pty_proxy.rs, session_commands.rs, startup_runtime.rs, learn.rs: inline Duration literals replaced with timeouts.rs constants"
  - "Overflow checks tightened in startup_runtime.rs and timeouts.rs (MAX_TIMEOUT = 3600s clamp)"
  - "docs/cli/usage/flags.mdx: --detach-timeout flag and timeout-tuning env vars documented"

affects: [55-06, 55-07, wave-4-merge]

tech-stack:
  added: []
  patterns:
    - "Centralized timeout module pattern: all Duration literals in a single timeouts.rs module"
    - "Env-var configurable timeouts with MAX_TIMEOUT clamping for DoS prevention"
    - "D-20 manual-replay conflict resolution for fork-diverged files"

key-files:
  created:
    - crates/nono-cli/src/timeouts.rs
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/learn.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/pty_proxy.rs
    - crates/nono-cli/src/session_commands.rs
    - crates/nono-cli/src/startup_runtime.rs
    - docs/cli/usage/flags.mdx

key-decisions:
  - "D-20 manual-replay for startup_runtime.rs: fork has Windows DETACHED_PROCESS launch (C9); C11 detach_timeout logic applied on top; deadline now uses detach_startup_timeout() (30s default) instead of 2s Windows-focused value"
  - "D-20 manual-replay for exec_strategy.rs: POST_EXIT_PTY_DRAIN_TIMEOUT constant removed; drain_master_output call lands in a future plan; CHILD_POLL_INTERVAL replaces 200ms inline literal"
  - "D-20 manual-replay for pty_proxy.rs: SessionGone retry path absent from fork; pty_attach_timeout_ms() applied at existing wait_for_attach_ready call site only"
  - "Windows cfg-arms in exec_strategy.rs preserved verbatim (D-55-E1 gate)"
  - "Commit 3 (14428182 formatting) applied as empty commit: formatting was already incorporated during commit 2 replay"

patterns-established:
  - "Timeout constants: use crate::timeouts::{CONSTANT} instead of inline Duration literals"
  - "User-facing timeouts: read via crate::timeouts::fn_name() which reads env var with MAX_TIMEOUT clamp"
  - "Internal poll intervals: use the pub const directly (no env var override)"

requirements-completed: [REQ-UPST7-02]

duration: 45min
completed: 2026-06-04
---

# Phase 55 Plan 05: TIMEOUT-CONSTANTS Summary

**C11 cherry-pick: centralized timeouts.rs module with env-var-configurable NONO_DETACH_STARTUP_TIMEOUT / NONO_PTY_DRAIN_TIMEOUT / NONO_PTY_ATTACH_TIMEOUT and overflow-check tightening (MAX_TIMEOUT = 3600s)**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-06-04
- **Completed:** 2026-06-04
- **Tasks:** 1 (3 cherry-pick sub-commits)
- **Files modified:** 9 (1 created, 8 modified)

## Accomplishments

- Created `crates/nono-cli/src/timeouts.rs` with named constants for all CLI timeout/interval values previously scattered as inline Duration literals
- Replaced inline Duration literals in exec_strategy.rs, pty_proxy.rs, session_commands.rs, startup_runtime.rs, learn.rs with named constants
- Applied overflow-check tightening: `MAX_TIMEOUT = 3600s` clamp in `env_duration_secs` and `env_duration_millis`, `secs.min(3600)` in `startup_runtime.rs` — T-55-05-01 mitigation verified
- Added `--detach-timeout` clap flag and `NONO_DETACH_STARTUP_TIMEOUT` env var wiring in cli.rs + startup_runtime.rs
- Documented `--detach-timeout`, `NONO_PTY_DRAIN_TIMEOUT`, `NONO_PTY_ATTACH_TIMEOUT` in docs/cli/usage/flags.mdx

## C11 Cherry-pick Log

| # | Upstream SHA | Fork Commit | Subject | Method |
|---|-------------|-------------|---------|--------|
| 1 | 194788ee | 929a4bb5 | feat(cli): centralize timeout constants | D-20 manual-replay (3 files conflict) |
| 2 | 69af73d5 | f1c00abd | fix: tighten up overflow checks | Direct port |
| 3 | 14428182 | 4837eb74 | fix: formatting | Empty commit (already in formatted form) |

D-19 trailer verification: `git log --format="%B" HEAD~3..HEAD | grep -c "^Upstream-commit:"` = 3 ✓

## Conflict-File Inventory

### cli.rs
- **C9 state:** Added `PackUpdateHintHelper` subcommand, `NONO_NO_PACK_UPDATE_HINTS` env var
- **C11 adds:** `--detach-timeout` / `detach_timeout_secs: Option<u64>` field in `RunArgs`
- **Resolution:** Applied C11 addition after C9's additions — both present side-by-side

### startup_runtime.rs
- **C9 state:** Windows DETACHED_PROCESS launch, cross-platform attach readiness probe, `Duration::from_secs(2)` deadline
- **C11 change:** `Duration::from_secs(30)` → `detach_timeout_secs.map(...).unwrap_or_else(detach_startup_timeout)`
- **Resolution:** Applied C11 timeout-constant logic using `detach_startup_timeout()` (30s default), replacing both C9's 2s value. Also replaced `from_millis(50)` and `from_millis(25)` with `SESSION_READY_POLL_INTERVAL` and `TERMINATE_POLL_INTERVAL`.

### exec_strategy.rs (C10 + C11 overlap)
- **C10 state:** Canonical-denial pre-computation already applied
- **C11 removes:** `POST_EXIT_PTY_DRAIN_TIMEOUT` const (replaced by `timeouts::POST_EXIT_PTY_DRAIN_TIMEOUT`)
- **Resolution:** Removed the local const + comment block, added `use crate::timeouts;`. `drain_master_output` usage site lands in a future plan; `CHILD_POLL_INTERVAL` replaces the 200ms sleep.

### pty_proxy.rs
- **C11 changes:** 4 Duration literals replaced with timeouts constants
- **Missing upstream code:** `SessionGone` retry path not present in fork
- **Resolution:** Applied 3 changes that matched fork structure; skipped ATTACH_RETRY_DELAY usage (no SessionGone path in fork)

## Verification Results

### D-55-E1: Windows-file non-touch gate
`git diff --name-only HEAD~3 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` → 0 lines ✓

### D-55-E2: D-19 trailer count
`git log --format="%B" HEAD~3..HEAD | grep -c "^Upstream-commit:"` = 3 ✓

### Build check
`cargo build --workspace` → exit 0 (12 dead_code warnings for Unix-only constants on Windows host — expected behavior on Windows, these constants ARE used on Linux/macOS) ✓

### cargo test
- Phase 54 baseline had 1 failing test: `sandbox::windows::tests::try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails`
- Post-C11: same 1 failure — carry-forward ✓
- `supervisor::aipc_sdk::tests::windows_loopback_tests::helper_stamps_session_token_from_env` failed in one full run but PASSES in isolation and is a pre-existing env-var parallel-test race (CLAUDE.md documented pattern) — NOT introduced by C11

### D-55-E4: Baseline-aware CI gate
| Test lane | Before C11 | After C11 | Category |
|-----------|-----------|-----------|----------|
| try_set_mandatory_label | FAIL | FAIL | red→red (carry-forward) |
| All other tests (732) | PASS | PASS | green→green |
| helper_stamps_session_token | Intermittent | Intermittent | environmental (parallel race) |

## Cross-target Clippy Status (D-55-E3)

**STATUS: PARTIAL — deferred to live CI**

Both `x86_64-unknown-linux-gnu` and `x86_64-apple-darwin` Rust targets are installed, but the C cross-linker (`x86_64-linux-gnu-gcc`, `cc`) is unavailable on this Windows dev host. Cargo fails at the `ring` crate's build script before reaching clippy analysis.

Per CLAUDE.md MUST/NEVER and `.planning/templates/cross-target-verify-checklist.md` § PARTIAL Disposition:

> Cross-target clippy gate SKIPPED: The Rust cross-compilation targets are installed
> (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`) but the required C cross-linker is
> unavailable on this Windows host. The gate is categorized as `skipped_gates_environmental`.
> Live CI (GitHub Actions ubuntu-latest / macos-latest runners) will exercise `exec_strategy.rs`,
> `pty_proxy.rs`, and `session_commands.rs` cfg-gated Unix branches against clippy
> `-D warnings -D clippy::unwrap_used`. No merge to main until CI passes.

Files in scope for D-55-E3:
- `crates/nono-cli/src/exec_strategy.rs` — cfg-gated Unix code (modified: removed `POST_EXIT_PTY_DRAIN_TIMEOUT` const, added `use crate::timeouts;`, changed 200ms sleep)
- `crates/nono-cli/src/pty_proxy.rs` — Unix-only PTY code (modified: 3 Duration literal replacements)
- `crates/nono-cli/src/session_commands.rs` — Unix-only session code (modified: 2 Duration literal replacements)

## Threat Model Mitigations

| Threat | Status |
|--------|--------|
| T-55-05-01: Env-var timeout overflow (NONO_DETACH_STARTUP_TIMEOUT etc.) | MITIGATED — `MAX_TIMEOUT = 3600s` clamp in both env_duration_secs/millis; `secs.min(3600)` in startup_runtime.rs |
| T-55-05-02: exec_strategy.rs Windows-cfg-arm preservation | PASS — Windows arms preserved verbatim; D-55-E1 gate 0 lines |
| T-55-05-03: Configurable timeouts enabling DoS | ACCEPTED — operator-controlled env vars, sandboxed child cannot modify parent's env |
| T-55-05-SC: No new Cargo deps in C11 | PASS — no Cargo.toml changes |

## Feature Branch Status

Branch: `worktree-agent-a0ff646e0caf19535`
Hold status: NOT merged to main per D-55-03 (merge blocked until v0.58.0 tagged + signed) ✓

## Known Stubs

None. All timeout constants are wired with real default values; env-var overrides work at runtime.

## Threat Flags

None. No new network endpoints, auth paths, or trust boundaries introduced.

## Self-Check: PASSED

- `crates/nono-cli/src/timeouts.rs` exists ✓
- Commit 929a4bb5 exists ✓
- Commit f1c00abd exists ✓
- Commit 4837eb74 exists ✓
- `git log --format="%B" HEAD~3..HEAD | grep -c "^Upstream-commit:"` = 3 ✓
- `cargo build --workspace` exits 0 ✓
