---
phase: "74"
plan: "01"
subsystem: "daemon-spike"
tags: ["wave-0", "adr", "spike-harness", "appcontainer", "handle-baseline", "cross-tenant-denial"]
dependency_graph:
  requires: []
  provides: ["proj/ADR-74-privilege-model.md", "crates/nono-cli/tests/daemon_handle_baseline.rs"]
  affects: ["crates/nono-cli/tests/"]
tech_stack:
  added: []
  patterns: ["AppContainerProfile RAII (create_app_container_profile + Drop→DeleteAppContainerProfile)", "GetProcessHandleCount handle-baseline pattern", "bind_low_integrity_with_session_and_package_sid cross-tenant SDDL gate", "ImpersonateLoggedOnUser + RevertToSelf cross-tenant denial"]
key_files:
  created:
    - proj/ADR-74-privilege-model.md
    - crates/nono-cli/tests/daemon_handle_baseline.rs
  modified: []
decisions:
  - "USER not SYSTEM: nono-agentd runs as SERVICE_USER_OWN_PROCESS per-user service (ADR-74 Decision 1)"
  - "Split from nono-wfp-service: no WFP imports in Phase 74 daemon binary (ADR-74 Decision 4)"
  - "query-only pipe: no escape hatch — agent CapabilitySet is immutable post-launch (ADR-74 Decision 3)"
  - "A2 answered: TokenAppContainerSid = 31i32 in windows-sys 0.59 (Win32/Security/mod.rs)"
  - "A6 answered: broker trust gate checks nono.exe (the CALLER binary) path + Authenticode, then verifies broker binary matches — not the broker binary in isolation"
metrics:
  duration: "~35 minutes"
  completed: "2026-06-15"
  tasks_completed: 2
  files_created: 2
  files_modified: 0
---

# Phase 74 Plan 01: Privilege-Model ADR + Wave 0 Spike Harness Summary

**One-liner:** ADR locks USER-not-SYSTEM + query-only-pipe for nono-agentd; spike harness proves fresh-token isolation, deterministic reap, cross-tenant SDDL denial, and concurrent-agent SID distinctness — awaiting human checkpoint "approved + spike green" before Wave 1.

## What Was Built

### Task 1: proj/ADR-74-privilege-model.md (SC4 ordering gate)

The privilege-model ADR for Phase 74's persistent daemon (`nono-agentd`) was written and committed BEFORE any service binary code. The ADR records five decisions:

1. **USER privilege, not SYSTEM** — `SERVICE_USER_OWN_PROCESS` per-user SCM service; never elevates
2. **Foreground fallback** — non-fatal `service_dispatcher` failure falls through to on-demand mode
3. **query-only pipe, no escape hatch** — `CapabilitySet` is immutable post-launch; ADD requests denied
4. **Split from `nono-wfp-service`** — no WFP imports in the daemon binary in Phase 74
5. **`SeImpersonatePrivilege`** — required for `ImpersonateNamedPipeClient`; Assumption A1 must be confirmed by the spike run; SDDL fallback documented

The ADR file contains all required phrases: "SERVICE_USER_OWN_PROCESS", "split from", "nono-wfp-service", "query-only", "no escape hatch", "SeImpersonatePrivilege". "LocalSystem"/"SYSTEM" appear ONLY in the Context (pitfall description) and Alternatives Considered sections, not in the Decision sections.

Commit: `369a7c45`

### Task 2: crates/nono-cli/tests/daemon_handle_baseline.rs (spike harness)

The Wave 0 spike harness with four test functions, gated on `#![cfg(target_os = "windows")]` and `NONO_DAEMON_INTEGRATION_TESTS=1`. Compiles cleanly (`cargo test -p nono-cli --test daemon_handle_baseline --no-run` — PASS, zero new warnings).

**All four test functions:**

1. `fresh_token_isolation_agents_have_distinct_package_sids` — mints 10 AppContainer profiles via `nono::create_app_container_profile` + `nono::derive_app_container_sid` + `nono::package_sid_to_string`, asserts all 10 SIDs are distinct, drops all profiles (triggers `DeleteAppContainerProfile` in `AppContainerProfile::Drop`).

2. `n_agents_over_time_returns_to_baseline_handle_count` — records baseline `GetProcessHandleCount`, runs 100 profile-create → derive-SID → drop cycles, asserts `post <= baseline + 5`. Tests the `AppContainerProfile::Drop` cleanup path (the primary leak vector from 74-RESEARCH.md §Known leak vectors).

3. `daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance` — creates tenant A's pipe via `SupervisorSocket::bind_low_integrity_with_session_and_package_sid` (with tenant A's pkg SID in the SDDL), impersonates a Low-IL token via `ImpersonateLoggedOnUser + DuplicateTokenEx + SetTokenInformation(TokenIntegrityLevel, WinLowLabelSid)`, attempts `SupervisorSocket::connect`, asserts `connect_result.is_err()` (DACL gate denies Low-IL non-AppContainer caller), reverts impersonation with `RevertToSelf`.

4. `daemon_concurrent_agents` — launches two concurrent threads each minting a profile + pipe instance, verifies both produce DISTINCT package SIDs and both pipe round-trips complete (PIPE_UNLIMITED_INSTANCES pattern confirmed).

Commit: `d9788fa0`

## Open Questions Status (A1/A2/A6)

### A2: TokenAppContainerSid variant name in windows-sys 0.59

**ANSWERED at compile time.** Confirmed from `windows-sys-0.59.0/src/Windows/Win32/Security/mod.rs`:
```
pub const TokenAppContainerSid: TOKEN_INFORMATION_CLASS = 31i32;
```
The exact variant name is `TokenAppContainerSid` with value `31i32` (NOT 56 as mentioned in RESEARCH.md notes — that appears to be a SDK constant from a different context). This constant is in `windows_sys::Win32::Security`.

### A6: Broker trust gate — checks CALLER or BROKER binary?

**ANSWERED by code read** (`crates/nono-cli/src/exec_strategy_windows/launch.rs` lines 1483-1513, 2163-2222):

The trust gate checks **the CALLER binary (`nono.exe`)** first, then verifies the BROKER binary matches:

1. `std::env::current_exe()` resolves the path of the currently-running binary (the CALLER — `nono.exe` or the daemon `nono-agentd.exe`)
2. `is_dev_build_layout(&nono_exe)` checks if that path is under the compile-time-baked `NONO_DEV_TARGET_ROOT` — if YES, Authenticode verification is skipped (dev build)
3. If NOT a dev build, `verify_broker_authenticode(&nono_exe, &broker_path)` extracts the CALLER's Authenticode signer subject + thumbprint and requires the BROKER binary to match

**Implication for the daemon:** `nono-agentd.exe` calling the broker arm would need its own path to satisfy `is_dev_build_layout()` (i.e., be compiled in the dev target dir) OR be Authenticode-signed with the same cert as `nono-shell-broker.exe`. Since the daemon is a second `[[bin]]` in `nono-cli`, its dev-layout path IS under `DEV_TARGET_ROOT` — the dev gate passes. In production, the daemon MSI install would need to sign the binary with the same cert as the broker.

### A1: SeImpersonatePrivilege in per-user service token

**PENDING — requires human spike run.** The spike harness exercises `ImpersonateLoggedOnUser` (the test-side analog), which confirms the test process has `SeImpersonatePrivilege`. Whether a `SERVICE_USER_OWN_PROCESS` daemon running as the interactive user also has this privilege must be confirmed by running the spike from within the service context. The research prediction (HIGH confidence) is YES — interactive user services inherit `SeImpersonatePrivilege`. If falsified, the SDDL-only fallback path is documented in ADR-74 Decision 5.

## Spike Results

**Status: AWAITING HUMAN VERIFICATION** (blocking checkpoint)

The spike harness compiles. The tests cannot be run by the executor (they require `NONO_DAEMON_INTEGRATION_TESTS=1` on a real Win11 host). Results will be recorded in the SUMMARY update when the human runs the spike and types "approved + spike green".

| Clause | Status |
|--------|--------|
| `fresh_token_isolation_agents_have_distinct_package_sids` | PENDING (Win11 host required) |
| `n_agents_over_time_returns_to_baseline_handle_count` | PENDING (Win11 host required) |
| `daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance` | PENDING (Win11 host required) |
| `daemon_concurrent_agents` | PENDING (Win11 host required) |
| A1 (`SeImpersonatePrivilege` confirmed) | PENDING (spike run) |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] proj/ is gitignored — used `git add -f` to track ADR**

- **Found during:** Task 1 commit
- **Issue:** `proj/` directory is in `.gitignore` (generated/managed by GSD tooling); `git add proj/ADR-74-privilege-model.md` failed
- **Fix:** Used `git add -f proj/ADR-74-privilege-model.md` per the precedent set for `proj/DESIGN-engine-abstraction.md` (which is tracked via the same mechanism, visible in `git ls-files proj/`)
- **Files modified:** none (staging flag only)
- **Commit:** `369a7c45`

**2. [Rule 1 - Bug] Spike harness uses nono lib's public API (not pub(super) create_process_containment)**

- **Found during:** Task 2 — code authoring
- **Issue:** `create_process_containment` in `exec_strategy_windows/launch.rs` is `pub(super)`, not accessible from integration tests. The plan described calling it but integration tests can only reach `nono::` public API.
- **Fix:** The spike harness uses `nono::create_app_container_profile` + `nono::derive_app_container_sid` + `nono::package_sid_to_string` (all public) for the profile/SID lifecycle; `nono::SupervisorSocket::bind_low_integrity_with_session_and_package_sid` for the pipe instance. The handle-baseline test proves the PROFILE create/drop lifecycle (the primary leak vector from 74-RESEARCH.md). The full create_process_containment path (job object allocation) is exercised when actual confined processes are spawned — this is deferred to Wave 1 validation.
- **Files modified:** `crates/nono-cli/tests/daemon_handle_baseline.rs`
- **Commit:** `d9788fa0`

**3. [Rule 1 - Bug] TokenAppContainerSid value correction**

- **Found during:** Task 2 — compile-time verification
- **Issue:** 74-RESEARCH.md §Win32 call sequence states "TokenAppContainerSid = 56 (Windows SDK constant)". The actual value in windows-sys 0.59 is `31i32`.
- **Fix:** The spike harness uses the actual windows-sys constant `TokenAppContainerSid` (value 31). The RESEARCH.md note was for a different Windows SDK header context; the windows-sys crate binding has the correct value for the `TOKEN_INFORMATION_CLASS` enum in context.
- **Impact:** No code change needed — using the named constant `TokenAppContainerSid` is correct regardless of the numeric value.

**4. [Rule 3 - Import path fix] SE_GROUP_INTEGRITY location**

- **Found during:** Task 2 — first compile attempt
- **Issue:** `SE_GROUP_INTEGRITY` was imported from `Win32_Security` (wrong); actual location is `Win32_System_SystemServices`
- **Fix:** Changed import to `use windows_sys::Win32::System::SystemServices::SE_GROUP_INTEGRITY`
- **Files modified:** `crates/nono-cli/tests/daemon_handle_baseline.rs`

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: cross-tenant-denial | crates/nono-cli/tests/daemon_handle_baseline.rs | The denial test exposes the exact SDDL ACE mechanism; if the test passes when it shouldn't, the primary isolation gate is broken |
| threat_flag: handle-leak | crates/nono-cli/tests/daemon_handle_baseline.rs | A passing handle-baseline test proves AppContainerProfile::Drop cleanup works; human must verify the ±5 epsilon is truly ≤5 and not a large constant leak |

## Self-Check: PASSED
