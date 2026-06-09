---
phase: 64
plan: "03"
subsystem: fltmgr-client + macos-cwd-capability
tags: [windows, fltmgr, spike, macos, seatbelt, cherry-pick, upstream-port, cwd]
dependency_graph:
  requires:
    - nono-fltmgr-client crate scaffold + run_policy_client (Plan 64-01)
    - nono-fltmgr.c \NonoPolicyPort + 532-byte NONO_IPC_REQUEST (Plan 64-02)
  provides:
    - non-Windows run_policy_client stub (cross-platform callable; DRV-02)
    - resolved_workdir() helper in sandbox_prepare.rs (MACOS-02)
    - macOS dual-path CWD FsCapability grant (symlink + canonical)
    - $PWD preference in resolved_workdir (symlink-form CWD without --workdir)
  affects:
    - crates/nono-fltmgr-client/src/lib.rs
    - crates/nono-cli/src/sandbox_prepare.rs
tech_stack:
  added: []
  patterns:
    - "#[cfg(target_os = \"macos\")] platform-gated second FsCapability grant"
    - "or_else chain with $PWD env preference before current_dir()"
    - "verbatim D-19 Upstream-commit: trailer per cherry-pick (D-10)"
key_files:
  created: []
  modified:
    - crates/nono-fltmgr-client/src/lib.rs (non-Windows run_policy_client stub)
    - crates/nono-cli/src/sandbox_prepare.rs (resolved_workdir + dual-path CWD + $PWD)
decisions:
  - "Plan 64-01 over-delivered the full Windows run_policy_client (FilterConnectCommunicationPort + FilterGetMessage loop + FilterReplyMessage); Plan 64-03 Task 1 therefore only added the missing #[cfg(not(windows))] fn stub so run_policy_client is callable on any platform (D-03)"
  - "Fork CWD block has a prompt/decline branch upstream 8f1b0b74 lacked; the single upstream macOS grant is split into the two branches that add the canonical cap (--allow-cwd, confirmed-prompt) so a declined prompt grants neither path (more restrictive — CLAUDE.md security rule; D-10 manual-port)"
  - "x86_64-apple-darwin clippy for nono-cli is PARTIAL: target installed but cc-rs (ring/aws-lc-sys transitive C deps of nono-proxy) needs a cross-compile C toolchain absent on the Windows dev host; the macOS dual-path block is a structural mirror of the adjacent compiling canonical-cap lines and is deferred to live CI per CLAUDE.md cross-target rule"
  - "Plan executed inline on main (sequential mode) after the background gsd-executor agent twice failed to provision Bash; no worktree, commits straight to main"
metrics:
  duration: "~25 minutes"
  completed: "2026-06-08"
  tasks: 2
  files: 2
---

# Phase 64 Plan 03: Rust policy client + macOS CWD cherry-picks Summary

One-liner: Completed the cross-platform `run_policy_client` surface and ported the first two P1 macOS cherry-picks (`8f1b0b74` resolved_workdir + dual-path CWD grant, `362ada22` $PWD preference) into `sandbox_prepare.rs`.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | non-Windows run_policy_client stub (DRV-02) | a1dbc65b | crates/nono-fltmgr-client/src/lib.rs |
| 2a | Cherry-pick 8f1b0b74 (resolved_workdir + macOS dual-path CWD) | 0a131bae | crates/nono-cli/src/sandbox_prepare.rs |
| 2b | Cherry-pick 362ada22 ($PWD preference) | 57b280b4 | crates/nono-cli/src/sandbox_prepare.rs |

## What Was Built

### Task 1 — non-Windows `run_policy_client` stub (DRV-02)

Plan 64-01 already over-delivered the **full** Windows `run_policy_client` implementation (the complete `FilterConnectCommunicationPort` + blocking `FilterGetMessage` loop + `FilterReplyMessage` round-trip, including two real windows-sys 0.59 API corrections — HRESULT out-param HANDLE and `ptr::read_unaligned` for packed-field access). The only remaining gap against this plan's Task 1 was the cross-platform stub: lib.rs exposed `run_policy_client` **only** under `#[cfg(windows)]` (a `const _NON_WINDOWS_STUB: () = ()` placeholder otherwise).

This task replaced that placeholder with a real `#[cfg(not(windows))] pub fn run_policy_client(_deny_path: &str) -> Result<(), Box<dyn Error>>` returning `"nono-fltmgr-client is Windows-only"`, so the function is callable from build scripts / integration tests on any platform (D-03). The non-Windows branch now actually compiles on apple-darwin and is clippy-clean.

### Task 2 — macOS CWD cherry-picks (MACOS-02)

**`8f1b0b74` (commit 0a131bae):** Extracted a `resolved_workdir(args: &SandboxArgs) -> PathBuf` helper (replacing the inline `args.workdir.clone().or_else(current_dir).unwrap_or_else(".")` expression in `prepare_sandbox_with_context`). Added a `#[cfg(target_os = "macos")]` dual-path grant: when CWD is reached via a symlink (e.g. `/tmp -> /private/tmp`), the canonical path differs, so the **symlink form** is emitted as a second `FsCapability::new_dir(workdir.clone(), access)?` alongside the canonical-path cap, letting Seatbelt allow traversal via either form.

**`362ada22` (commit 57b280b4):** Added a `$PWD` preference to `resolved_workdir` — `std::env::var("PWD").ok().map(PathBuf::from)` is tried **between** `--workdir` and `current_dir()`. `$PWD` preserves the symlink path where `current_dir()` resolves it, so `nono run` without `--workdir` captures the symlink form of the CWD. `$PWD` is still validated by the subsequent `canonicalize()` call (T-64-P1: no new privilege-escalation vector).

## Verification

| Gate | Result |
|------|--------|
| `cargo build -p nono-fltmgr-client` (Windows host) | PASS |
| `cargo clippy -p nono-fltmgr-client -- -D warnings -D clippy::unwrap_used` | PASS |
| `cargo clippy -p nono-fltmgr-client --target x86_64-apple-darwin` | PASS (non-Windows stub now compiles + clippy-clean) |
| `cargo build -p nono-cli` (Windows host) | PASS |
| `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` | PASS |
| `cargo test -p nono-cli` | 1211 passed; 4 failed (pre-existing Windows baseline: profile_cmd init + 3 protected_paths — fail at phase-base too, not regressions) |
| `resolved_workdir` function present in sandbox_prepare.rs | PASS |
| `#[cfg(target_os = "macos")]` dual-path block present | PASS (two branches) |
| Both commits carry verbatim `Upstream-commit:` trailers + DCO | PASS (8f1b0b74, 362ada22) |
| `cargo clippy -p nono-cli --target x86_64-apple-darwin` | PARTIAL — blocked by cc-rs (ring/aws-lc-sys, transitive via nono-proxy) needing a cross-compile C toolchain absent on the Windows dev host; macOS dual-path block deferred to live CI per CLAUDE.md cross-target rule |
| aarch64-apple-darwin / x86_64-unknown-linux-gnu | PARTIAL → live CI (targets not installed) |

## Deviations from Plan

**1. [Process] Inline execution instead of worktree subagent**
- The orchestrator's background `gsd-executor` agent failed twice in a row to provision the Bash tool (returned immediately requesting Bash access, no worktree created, no commits). To avoid burning further cycles, this single 2-task plan was executed inline on `main` (the workflow's sanctioned sequential-inline fallback). Commits landed directly on `main`; no worktree was used.

**2. [Scope] Task 1 was already substantially complete**
- Plan 64-01's executor implemented the full Windows `run_policy_client` rather than a scaffold stub (its own SUMMARY's "Known Stubs" table is therefore stale). Task 1 here reduced to adding the non-Windows fn stub. The plan's Task 1 must_have ("lib.rs has a pub fn run_policy_client that connects to \NonoPolicyPort, loops on FilterGetMessage, returns allow/deny via FilterReplyMessage") was already satisfied; this task completed the cross-platform-callability done-criterion.

**3. [Security adaptation] macOS grant split across grant branches (D-10 manual-port)**
- Upstream 8f1b0b74's single `#[cfg(target_os = "macos")]` block sat in a call-site with no decline branch. The fork's CWD block has `--allow-cwd` / silent-error / interactive-prompt branches. Placing one block before `caps.deduplicate()` would have added the symlink cap even when the user **declined** CWD sharing (granting refused access — less restrictive). The grant was therefore split into the two branches that actually add the canonical cap, so a declined prompt grants neither path. Noted in the 0a131bae commit body.

## Threat Surface Scan

No new network endpoints, auth paths, schema changes, or packages. `$PWD` becomes an input to CWD capability resolution but is still validated by the existing `canonicalize()` (T-64-P1, accepted in the plan threat model — same risk class as `--workdir` injection, no escalation). The macOS dual-path grant only widens CWD access to the symlink form of an already-granted directory. No unmodeled surface found.

## Self-Check: PASSED

- `crates/nono-fltmgr-client/src/lib.rs` non-Windows `run_policy_client` fn: FOUND
- `resolved_workdir` in `crates/nono-cli/src/sandbox_prepare.rs`: FOUND
- `#[cfg(target_os = "macos")]` dual-path blocks: FOUND (2)
- `std::env::var("PWD")` in resolved_workdir: FOUND
- Commit `a1dbc65b` (Task 1): FOUND
- Commit `0a131bae` (Upstream-commit: 8f1b0b74): FOUND
- Commit `57b280b4` (Upstream-commit: 362ada22): FOUND
- All three commits carry `Signed-off-by: Oscar Mack Jr`: FOUND
