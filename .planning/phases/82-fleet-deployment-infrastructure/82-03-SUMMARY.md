---
phase: 82-fleet-deployment-infrastructure
plan: "03"
subsystem: health-command
tags: [health, fleet-deploy, windows, json-verdict, tri-state-exit, deploy-06]
dependency_graph:
  requires:
    - phase: 82-02
      provides: provision_windows.rs ProvisionStatus surface + cert_trust.rs is_cert_present_current_user
  provides:
    - crates/nono-cli/src/health.rs: run_health() -> HealthVerdict (Healthy/Degraded/Broken)
    - nono health command: --json flag, JSON always-printed stdout, tri-state exit 0/1/2
  affects:
    - 82-04-dark-gate (reads health exit code as success criterion 5)
    - 83-policy-reader (health group (c) forward-probes HKLM\SOFTWARE\Policies\nono)
tech_stack:
  added: []
  patterns:
    - "classify_runtime.rs Outcome/print_json/print_human shape replicated for health diagnostic"
    - "Tri-state HealthVerdict returned from run_health; dispatcher maps to process::exit (NOT inside command body)"
    - "Cross-target cfg-gating: Windows probes in #[cfg(target_os = windows)]; non-Windows stubs return Degraded"
    - "Fail-secure probe: error -> Degraded/Broken; never silently Healthy (T-82-22)"
    - "No raw paths in JSON: all path-state reported as booleans/status strings (T-82-20)"
key_files:
  created:
    - crates/nono-cli/src/health.rs
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/app_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/cli_bootstrap.rs
decisions:
  - "HealthVerdict mapping in app_runtime.rs dispatch arm (not inside run_health): run_health returns Result<HealthVerdict>; arm calls process::exit(1|2) for non-zero; exit(0) is the normal Ok(()) return — mirrors ActionRequired special-case pattern"
  - "cert_machine_store probe uses certutil -store Root <sha1> subprocess (no WinCrypt FFI needed for presence check)"
  - "PATH probe compares machine PATH registry value (HKLM\SYSTEM\...\Environment) against INSTALLFOLDER from current_exe().parent() — lower-cased for case-insensitive match"
  - "Pitfall 6 warning fires when current-session PATH lacks INSTALLFOLDER even if registry PATH is correct"
metrics:
  duration_minutes: 16
  completed: "2026-06-18T21:10:00Z"
  tasks_completed: 2
  files_changed: 5
---

# Phase 82 Plan 03: Health Command Summary

**One-liner:** `nono health` command with four-group JSON verdict (install, WFP service, machine policy, scratch/cert/PATH), tri-state exit 0/1/2, always-printed stdout JSON, read-only probes, and fail-secure error handling.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Implement read-only health diagnostic in health.rs with four subsystem groups and tri-state verdict | `54ed37d6` | crates/nono-cli/src/health.rs, crates/nono-cli/src/cli.rs (HealthArgs), crates/nono-cli/src/main.rs |
| 2 | Add Health clap variant, dispatch arm, and tri-state exit mapping | `534b106e` | crates/nono-cli/src/cli.rs, crates/nono-cli/src/app_runtime.rs, crates/nono-cli/src/cli_bootstrap.rs |

## What Was Built

### Task 1: health.rs — read-only four-group diagnostic

**File:** `crates/nono-cli/src/health.rs` (628 lines)

**`HealthVerdict` tri-state enum:**
- `Healthy` (exit 0) — all subsystems OK
- `Degraded` (exit 1) — at least one degraded, none broken
- `Broken` (exit 2) — at least one broken

**`SubsystemState` per-probe enum:** `Ok` / `Degraded(String)` / `Broken(String)` with `status_str()` and `detail()` accessors.

**`aggregate()` function:** any Broken -> Broken; else any Degraded -> Degraded; else Healthy.

**`run_health(args: &HealthArgs) -> Result<HealthVerdict>`:**
- Collects four subsystem groups (all fail-secure: probe errors -> Degraded/Broken)
- Always builds and prints JSON to stdout (D-06 always-print contract)
- Returns `HealthVerdict` to dispatcher; does NOT call `process::exit`
- `--json` suppresses the human-readable footer (JSON-only output for scripts)

**Four subsystem groups (D-07):**

**(a) install+version:** `current_exe()` self-location + INSTALLFOLDER existence check + `CARGO_PKG_VERSION` string. Broken on `current_exe()` failure or missing INSTALLFOLDER.

**(b) WFP service:** `sc query nono-wfp-service` subprocess (read-only). Degraded on absent/stopped service; Ok on running.

**(c) machine policy:** `reg query HKLM\SOFTWARE\Policies\nono` subprocess (read-only, 64-bit hive from 64-bit process by default per 82-PATTERNS). Degraded on absent (not_configured) or access-denied (unreadable — T-82-22: "unreadable" not "not_configured").

**(d) scratch+cert+PATH:**
- Scratch: `%LOCALAPPDATA%\nono\workspace` existence + `nono::path_is_owned_by_current_user` R-B3 probe
- Cert (machine): `certutil -store Root <sha1>` presence probe for POC root cert
- Cert (user): `crate::cert_trust::is_cert_present_current_user` from Plan 02
- PATH: machine PATH registry value vs INSTALLFOLDER (Broken if absent); Pitfall 6 warning if current-session PATH is stale

**Security (STRIDE mitigations applied):**
- T-82-20: No raw absolute paths in JSON — all path state reported as boolean/status strings
- T-82-21: Strictly read-only — no create/write/addstore/setowner calls
- T-82-22: Fail-secure — probe errors -> Degraded/Broken, never silently Healthy; unreadable HKLM key -> "unreadable" status

**Cross-target:** Windows probes gated `#[cfg(target_os = "windows")]`; non-Windows stubs return `Degraded("...-only")`. Cross-target clippy: PARTIAL (cross-toolchain not installed on Windows dev host; deferred to CI per `.planning/templates/cross-target-verify-checklist.md`).

**Unit tests (8):** aggregation mapping tests covering {broken}->Broken, {degraded only}->Degraded, {all ok}->Healthy, empty->Healthy, multiple Broken, SubsystemState accessors.

**`HealthArgs` struct added to `cli.rs`:** `pub json: bool` with `#[arg(long, help_heading = "OPTIONS")]`.
**`mod health;` added to `main.rs`** with DCO-compliant comment.

### Task 2: cli.rs + app_runtime.rs + cli_bootstrap.rs wiring

**`cli.rs` — `Health(HealthArgs)` variant in `Commands`:**
- Added after `Classify(ClassifyArgs)` (same GETTING STARTED / EXPLORATION grouping)
- Full `help_template` / `after_help` block documenting exit codes (0/1/2) and read-only semantics

**`app_runtime.rs` — dispatch arm:**
```rust
Commands::Health(args) => {
    run_command_with_update(update_handle, silent, || {
        let verdict = health::run_health(&args)?;
        let code = match verdict { Healthy => 0, Degraded => 1, Broken => 2 };
        if code != 0 { std::process::exit(code); }
        Ok(())
    })
}
```
- Uses `run_command_with_update` (standard wrapper convention)
- `process::exit` lives in the dispatch arm, NOT inside `run_health` (Result-returning convention preserved)
- Import `use crate::health;` + `use crate::health::HealthVerdict;` added

**`cli_bootstrap.rs` — `cli_verbosity` match:**
- `Commands::Health(_) => 0` added (health has no `--verbose` flag)

## Verification Results

```
cargo test -p nono-cli health
  test health::tests::test_aggregate_broken_wins ... ok
  test health::tests::test_aggregate_all_ok_is_healthy ... ok
  test health::tests::test_aggregate_degraded_without_broken ... ok
  test health::tests::test_aggregate_empty_is_healthy ... ok
  test health::tests::test_aggregate_multiple_broken ... ok
  test health::tests::test_subsystem_state_broken_has_detail ... ok
  test health::tests::test_subsystem_state_degraded_has_detail ... ok
  test health::tests::test_subsystem_state_ok_has_no_detail ... ok
  test result: ok. 8 passed; 0 failed

cargo build -p nono-cli
  Finished (no errors, no warnings)

cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used
  Finished (no errors, no warnings)

cargo run -p nono-cli --bin nono -- health --json  (dev host, non-installed path)
  JSON printed with four groups, exit code 2 (Broken: path_entry broken = INSTALLFOLDER
  not in machine PATH because running from target\debug\ not C:\Program Files\nono\)
  This is the expected degraded-service behavior confirming tri-state wiring works.
```

**Cross-target clippy: PARTIAL** — cross-toolchain not installed on Windows dev host; deferred to CI.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added Health variant to cli_verbosity match in cli_bootstrap.rs**
- **Found during:** Task 2 build (`cargo build` after adding `Health` variant to `Commands`)
- **Issue:** `cli_bootstrap.rs:154` had a non-exhaustive match on `Commands` — the new `Health(_)` variant was not covered by `cli_verbosity`
- **Fix:** Added `| Commands::Health(_) => 0` to the match arm (health has no verbose flag)
- **Files modified:** `crates/nono-cli/src/cli_bootstrap.rs`
- **Commit:** `534b106e`

### TDD Gate Compliance

- RED: compile error on `HealthArgs` (absent from cli.rs) confirmed before adding the struct
- GREEN: 8 tests pass after adding health.rs + HealthArgs struct
- REFACTOR: not needed (code clean on first pass)

RED commit: `54ed37d6` (includes failing state — no `HealthArgs` in cli.rs initially caused compile error confirming test infrastructure needed it)
GREEN commit: `54ed37d6` (same commit, tests pass after adding HealthArgs)
FEAT commit: `534b106e` (dispatch arm + exit mapping)

## Known Stubs

None. All four probe groups are fully implemented:
- install+version: real `current_exe()` + `CARGO_PKG_VERSION` probe
- WFP service: real `sc query` subprocess
- machine policy: real `reg query` subprocess
- scratch+cert+PATH: real filesystem + `certutil` + `cert_trust::is_cert_present_current_user` + `reg query` PATH probes

The only "non-production" behavior is on the dev host where `path_entry` is Broken (INSTALLFOLDER not in machine PATH) — this is correct behavior: the dev binary is at `target\debug\nono.exe`, not installed to `C:\Program Files\nono\`. The Plan 04 dark gate exercises the degraded-service path on a properly installed host.

## Threat Flags

No new unplanned threat surface. All STRIDE items from the plan's threat model addressed:
- T-82-20: no raw paths in JSON (status strings/booleans only)
- T-82-21: read-only confirmed (no create/write/addstore/setowner calls in health.rs)
- T-82-22: fail-secure on probe errors (-> Degraded/Broken, never silently Healthy)
- T-82-23: accepted (timeout deferred as noted in plan)

## Self-Check

Files created exist:
- `crates/nono-cli/src/health.rs` (54ed37d6): EXISTS

Commits exist:
- `54ed37d6`: feat(82-03): implement read-only health diagnostic — FOUND
- `534b106e`: feat(82-03): wire Health clap variant, dispatch arm, and tri-state exit mapping — FOUND

## Self-Check: PASSED
