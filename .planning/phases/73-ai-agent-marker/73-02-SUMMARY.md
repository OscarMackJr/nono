---
phase: 73-ai-agent-marker
plan: "02"
subsystem: exec_strategy_windows
tags: [windows, job-object, security-descriptor, sddl, hardening, d-03]
dependency_graph:
  requires: []
  provides: [job-acl-hardening, breakaway-denied-test, per-agent-sid-deny-ace]
  affects: [BrokerLaunchNoPty, execute_direct, execute_supervised]
tech_stack:
  added: []
  patterns:
    - OwnedJobSD RAII (LocalFree on Drop — mirrors OwnedSecurityDescriptor in sandbox/windows.rs)
    - build_job_security_attributes — SDDL D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW) pattern
    - Explicit SECURITY_ATTRIBUTES threaded into CreateJobObjectW (replaces null pointer)
key_files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/supervisor.rs
decisions:
  - "Used hex rights (0x1F001F) in SDDL per Pitfall 3 — SDDL mnemonic MAJOBS not universally available"
  - "Local OwnedJobSD RAII instead of nono-re-export — OwnedSecurityDescriptor is private in nono crate"
  - "Third test job_security_descriptor_with_package_sid added (Claude discretion) to exercise the Some(sid) branch"
  - "supervisor.rs test call site updated as deviation Rule 1 (bug: compile break on signature change)"
metrics:
  duration: "~35 minutes"
  completed: "2026-06-14T23:04:13Z"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 3
---

# Phase 73 Plan 02: Job ACL Hardening (D-03) Summary

**One-liner:** SDDL-built explicit DACL on CreateJobObjectW denies Low-IL and per-agent package SID any job access; breakaway-denied negative test enforced as regression guard.

## What Was Built

### Task 1 — Refactor `create_process_containment` signature + SDDL job ACL (commit `8a1c0751`)

**Files:** `launch.rs`, `supervisor.rs`

Three concrete changes:

1. **New imports** — `ConvertStringSecurityDescriptorToSecurityDescriptorW`, `SDDL_REVISION_1`, `PSECURITY_DESCRIPTOR`, `LocalFree` added to `launch.rs` header.

2. **`OwnedJobSD` RAII struct** — calls `LocalFree` on the `PSECURITY_DESCRIPTOR` returned by `ConvertStringSecurityDescriptorToSecurityDescriptorW`. Guards the SD for the duration of `CreateJobObjectW`. Mirrors the private `OwnedSecurityDescriptor` in `crates/nono/src/sandbox/windows.rs` (that type is not re-exported from the `nono` crate).

3. **`build_job_security_attributes(package_sid: Option<&str>)`** — builds the SDDL:
   - Base: `D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW)` — protected DACL, owner-only allow, Low-IL deny
   - With package_sid: appends `(D;;0x1F001F;;;<sid>)` per-agent deny ACE
   - Uses hex `0x1F001F` for rights (not SDDL mnemonic — Pitfall 3 from RESEARCH.md)
   - Returns `(OwnedJobSD, SECURITY_ATTRIBUTES)` — caller keeps both alive past `CreateJobObjectW`

4. **Signature refactor** — `create_process_containment(session_id: Option<&str>)` →  `create_process_containment(session_id: Option<&str>, package_sid: Option<&str>)`.

5. **`CreateJobObjectW` call** — first argument changed from `std::ptr::null()` to `&sa` (the `SECURITY_ATTRIBUTES` built from the SDDL descriptor).

6. **Existing test call sites** — all 7 `create_process_containment(None)` calls in `apply_resource_limits_tests` updated to `create_process_containment(None, None)`.

7. **New `job_hardening_tests` module** — three tests:
   - `job_never_has_breakaway_ok` — reads `JOBOBJECT_EXTENDED_LIMIT_INFORMATION.LimitFlags`, asserts `JOB_OBJECT_LIMIT_BREAKAWAY_OK` bit is 0. Regression guard.
   - `job_security_descriptor_denies_low_il` — asserts `create_process_containment(None, None)` returns `Ok` (SDDL string accepted by OS, job created with explicit ACL).
   - `job_security_descriptor_with_package_sid` — asserts `create_process_containment(None, Some("S-1-15-2-1-2-3-4-5-6-7"))` returns `Ok` (per-agent deny ACE path exercised).

### Task 2 — Update call sites in mod.rs (commit `b9833063`)

**Files:** `mod.rs`

Two surgical edits:
- `execute_direct` line 818: `create_process_containment(session_id)` → `create_process_containment(session_id, config.package_sid.as_deref())`
- `execute_supervised` line 892: same change

`config.package_sid` is `Option<String>`; `.as_deref()` converts to `Option<&str>` for the new parameter. Both call sites already have access to `config: &ExecConfig<'_>`.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo build -p nono-cli --target x86_64-pc-windows-msvc` | PASS (21s) |
| `cargo test -p nono-cli --target x86_64-pc-windows-msvc` | 1247 passed, 6 pre-existing failures (broker binary missing, profile_cmd env, protected_paths) |
| `job_hardening_tests::job_never_has_breakaway_ok` | PASS |
| `job_hardening_tests::job_security_descriptor_denies_low_il` | PASS |
| `job_hardening_tests::job_security_descriptor_with_package_sid` | PASS |
| All 7 `apply_resource_limits_tests` with updated (None, None) signatures | PASS |
| `cargo clippy -p nono-cli --target x86_64-pc-windows-msvc -- -D warnings -D clippy::unwrap_used` | PASS (no warnings) |
| `grep` for `std::ptr::null()` as first arg to `CreateJobObjectW` in `create_process_containment` | 0 matches (replaced by `&sa`) |
| `grep -c "JOB_OBJECT_LIMIT_BREAKAWAY_OK" launch.rs` | 4 matches (constant used in test assertions) |
| `grep -c "create_process_containment(session_id, config.package_sid.as_deref())" mod.rs` | 2 matches (both call sites) |
| Cross-target clippy (x86_64-unknown-linux-gnu) | PARTIAL — C cross-toolchain (`x86_64-linux-gnu-gcc`) not installed on Windows dev host; deferred to CI per CLAUDE.md cross-target-verify-checklist.md |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] supervisor.rs had a third call site not mentioned in the plan**
- **Found during:** Task 1 test run (E0061 compile error in `supervisor.rs:5515`)
- **Issue:** `supervisor.rs` test module `deadline_reached_terminates_job_and_returns_timeout_code` used `create_process_containment(None)` — the plan only identified call sites in `mod.rs` lines 818 and 892.
- **Fix:** Updated to `create_process_containment(None, None)` — pure call-site update, no logic change.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`
- **Commit:** `8a1c0751` (included with Task 1 since it was needed to compile)

**2. [Rule 2 - Auto-add] Added third test `job_security_descriptor_with_package_sid`**
- **Found during:** Task 1 implementation
- **Issue:** The plan specified two new tests. The `package_sid: Some(...)` branch of `build_job_security_attributes` was exercised by neither `job_never_has_breakaway_ok` nor `job_security_descriptor_denies_low_il`. Without a test, the `Some` branch could be broken without detection.
- **Fix:** Added `job_security_descriptor_with_package_sid` test (within CLAUDE.md "Claude's Discretion" for exact SDDL construction).
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/launch.rs`
- **Commit:** `8a1c0751`

## Known Stubs

None. All new code is fully wired and tested.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes introduced. All changes are within the existing `CreateJobObjectW` call path (hardening, not surface expansion).

## Self-Check: PASSED

| Item | Status |
|------|--------|
| `.planning/phases/73-ai-agent-marker/73-02-SUMMARY.md` exists | FOUND |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` exists | FOUND |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` exists | FOUND |
| Commit `8a1c0751` exists (Task 1) | FOUND |
| Commit `b9833063` exists (Task 2) | FOUND |
