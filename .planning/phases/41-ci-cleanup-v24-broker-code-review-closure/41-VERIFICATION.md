---
phase: 41-ci-cleanup-v24-broker-code-review-closure
verified: 2026-05-16T20:30:00Z
status: gaps_found
score: 4/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 5/5
  previous_verified: 2026-05-16T19:30:00Z
  trigger: "CI run 25972316892 (push of b284bc63) surfaced 6 new -Dwarnings dead-code errors on Linux Test + Linux Clippy + macOS Clippy lanes after the HandleTarget import fix landed"
  gaps_closed: []
  gaps_remaining:
    - "REQ-CI-01 SC#1+#3+#4: cross-target Linux/macOS clippy clean — re-opened after CI surfaced 6 dead-code errors"
  regressions:
    - "REQ-CI-01 falls back from VERIFIED to PARTIAL: Linux Test, Linux Clippy, and macOS Clippy GH Actions lanes still red on b284bc63 due to escalated dead-code lints"
must_haves:
  truths:
    - "REQ-CI-01: cross-target Linux clippy clean; no new raw #[allow(dead_code)]; orphans deleted or cfg-gated"
    - "REQ-CI-02: 5 Windows CI jobs (Build, Integration, Regression, Security, Packaging) green; MSI validator -BrokerPath mismatch resolved; no unjustified #[ignored]"
    - "REQ-CI-03: baseline-aware CI gate baseline SHA + skipped-gates convention + STATE.md ## Deferred Items cleanup"
    - "REQ-BROKER-CR-01..03: BrokerNotFound FFI remap + broker null/INVALID + empty-list rejects"
    - "REQ-BROKER-CR-04: Job-object test silent-SKIP→FAIL resolved; STATE.md v24 CR-A entries cleared"
gaps:
  - truth: "REQ-CI-01: cross-target Linux/macOS clippy clean; no new raw #[allow(dead_code)]; orphans deleted or cfg-gated"
    status: partial
    reason: "CI run 25972316892 on commit b284bc63 surfaced 6 new -Dwarnings dead-code errors that the prior verification missed because cross-target Linux/macOS clippy was SKIPPED locally. Three lanes RED (Linux Test, Linux Clippy, macOS Clippy). REQ-CI-01 SC#1 and SC#3 cannot pass until these clear; SC#4 forbids #[allow(dead_code)] so the fix must cfg-gate or wire/delete."
    artifacts:
      - path: "crates/nono-cli/src/exec_strategy/env_sanitization.rs"
        line: 127
        issue: "`fn validate_env_var_patterns` is `pub(crate)` but never invoked on Linux/macOS. Closely related to WR-06 (profile_runtime.rs:290 declares byte-identical `validate_env_var_patterns_local` — neither calls the canonical fn). Linux Test job error: 'function `validate_env_var_patterns` is never used'."
      - path: "crates/nono-cli/src/launch_runtime.rs"
        line: 170
        issue: "Field `ExecutionFlags.interactive_shell: bool` is read only inside `#[cfg(target_os = \"windows\")]` blocks (execution_runtime.rs:411 → exec_strategy_windows). On Linux/macOS, the field is set (defaults.rs:204) but never read. Linux Test job error: 'field `interactive_shell` is never read'."
      - path: "crates/nono-cli/src/setup.rs"
        line: "14-18"
        issue: "Fields `register_wfp_service, install_wfp_service, install_wfp_driver, start_wfp_service, start_wfp_driver: bool` on `SetupRunner` struct. Read sites at lines 28-32, 52-72, 650-787 are all WFP-specific Windows code paths NOT cfg-gated; on Linux/macOS the field readers compile but the readers themselves should be Windows-only. Linux Test job error: 'fields `register_wfp_service, install_wfp_service, install_wfp_driver, start_wfp_service, start_wfp_driver` are never read'."
      - path: "crates/nono-cli/src/setup.rs"
        line: "737-771"
        issue: "Methods `register_phase_index (737), install_phase_index (741), start_phase_index (745), install_driver_phase_index (752), start_driver_phase_index (756), recheck_wfp_phase_index (771)` on `SetupRunner`. Called only from Windows-only WFP setup flow (lines 152, 172, 192, 212, 232, 277); these callers must be Windows-only too. Linux Test job error: 'methods `register_phase_index, install_phase_index, start_phase_index, install_driver_phase_index, start_driver_phase_index, recheck_wfp_phase_index` are never used'."
      - path: "crates/nono-cli/tests/common/test_env.rs"
        line: "23-37"
        issue: "`EnvVarGuard::set_all` (line 26) is invoked at `crates/nono-cli/tests/env_vars.rs:1047` inside a `#[cfg(target_os = \"windows\")]` test (line 1039). On Linux/macOS the test compiles out, so `set_all` becomes unused. Linux Clippy job error: 'associated function `set_all` is never used'."
      - path: "crates/nono/src/keystore.rs"
        line: "1074-1078"
        issue: "`.map_err(|e| { let _ = child.kill(); let _ = child.wait(); e })?` is the canonical clippy `manual_inspect` pattern (side-effect-only closure returning `e` unchanged). Inside `#[cfg(target_os = \"macos\")] load_from_apple_password`. macOS Clippy job error: 'using `map_err` over `inspect_err`'."
    missing:
      - "cfg-gate `fn validate_env_var_patterns` to Windows OR wire profile_runtime.rs:290 to call this canonical fn (closes WR-06 simultaneously per the deferred-backlog cross-reference)"
      - "cfg-gate `ExecutionFlags.interactive_shell` to Windows on the field declaration (launch_runtime.rs:170) OR add `#[cfg_attr(not(target_os = \"windows\"), allow(dead_code))]` per the existing precedent at launch_runtime.rs:180 for `allowed_env_vars`"
      - "cfg-gate the 5 WFP-related `SetupRunner` fields (setup.rs:14-18) AND their reader sites (lines 28-32, 52-72, 650-787) to Windows OR migrate to a `#[cfg(target_os = \"windows\")] mod wfp_setup;` extraction"
      - "cfg-gate the 6 `phase_index` methods (setup.rs:737-771) AND their call sites (lines 152, 172, 192, 212, 232, 277) to Windows"
      - "cfg-gate the duplicated `EnvVarGuard` in `tests/common/test_env.rs` to Windows OR drop the mirror entirely and lift `tests/env_vars.rs::windows_run_*` into a Windows-only integration test file (e.g. tests/env_vars_windows.rs)"
      - "Replace `crates/nono/src/keystore.rs:1074-1078` `.map_err(|e| { ...; e })` with `.inspect_err(|_| { let _ = child.kill(); let _ = child.wait(); })` per the existing precedent at keystore.rs:1006 inside `load_from_op` (already uses `.inspect_err(|_e| { ... })`)"
human_verification:
  - test: "Verify CI run 25972316892 + successor runs no longer hit -Dwarnings dead-code errors after the gap-closure plan lands"
    expected: "GH Actions Linux Test, Linux Clippy, macOS Clippy lanes on the SHA carrying the dead-code gap-closure commits all PASS. No occurrence of 'is never used', 'is never read', or 'using `map_err` over `inspect_err`' in the lane logs."
    why_human: "Live CI signal; not reproducible locally without C cross-compiler for Linux clippy from this Windows dev host. NEW item for this re-verification."
  - test: "Verify the windows-build CI lane no longer fails at PowerShell parameter binding on the next PR push after Plan 41-08 lands"
    expected: "ci-logs/windows-build.log contains NO 'Cannot process command because of one or more missing mandatory parameters: BrokerPath' line; the build suite progresses past 'validate windows msi contract' label; cargo build -p nono-shell-broker step appears and succeeds; Test-Path guard passes silently"
    why_human: "Plan 41-08 closed the gap at the codebase level (verified by grep + PowerShell syntax check), but the decisive live signal — GH Actions windows-build job green on PR head SHA — lives in CI and is not reproducible from this dev host. Carried forward from prior verification."
  - test: "Verify all 7 GitHub Actions CI lanes green on Phase 41 close SHA (post-gap-closure head)"
    expected: "Linux Clippy + Linux Test + macOS Clippy + Windows Build + Windows Integration + Windows Regression + Windows Security + Windows Packaging all PASS on the same head SHA"
    why_human: "Lives in GitHub Actions; not reproducible locally. REQ-CI-01/02 SC require GH Actions green on Phase 41 close SHA. Carried forward from prior verification."
  - test: "Verify the env_vars parallel flake fix on a real Windows host (cargo test -p nono-cli --test env_vars windows_run_redirects_profile_state_vars_into_writable_allowlist run 10x in parallel)"
    expected: "0 failures across 10 parallel runs"
    why_human: "Plan 41-05 used Windows-host-only verification; current dev host could not execute the flake check (10x runs). CI Integration job covers this on Windows-latest. Carried forward from prior verification."
  - test: "Verify the block-net probe tests pass on a Windows host with NONO_CI_HAS_WFP=true (elevated, WFP service installed)"
    expected: "windows_run_block_net_blocks_probe_connection + windows_run_block_net_blocks_probe_through_cmd_host both PASS with 'connect failed' or 'exit code 42' markers in stderr"
    why_human: "Plan 41-04 short-circuits on non-elevated dev hosts; full probe path runs only on elevated CI runner. Carried forward from prior verification."
  - test: "Verify cross-binding nono-py / nono-ts impact of CR-01 FFI remap"
    expected: "No integer-mapping of -1 (ErrPathNotFound) as broker-discovery-failure in downstream bindings — or follow-up todo filed for lockstep"
    why_human: "../nono-py/ and ../nono-ts/ are sibling repositories not present in this working directory; D-10 manual verification was deferred per Plan 41-06 SUMMARY. Carried forward from prior verification."
---

# Phase 41: CI cleanup + v24 broker code-review closure Verification Report

**Phase Goal:** Reset every CI lane to green and clear the v24 Windows broker code-review backlog so Phases 42 + 43 inherit a clean baseline.

**Verified:** 2026-05-16T20:30:00Z (re-verification triggered by CI run 25972316892)
**Status:** gaps_found
**Re-verification:** Yes — supersedes 2026-05-16T19:30:00Z verification

## Re-verification Summary

The prior verification (2026-05-16T19:30:00Z, post-Plan-41-08) returned `status: human_needed` with 5/5 must-haves VERIFIED at the codebase level. The single outstanding REQ-CI-01 risk was explicitly documented as "cross-target Linux clippy from Windows host: SKIPPED — load-bearing per CI Linux native lane" (Behavioral Spot-Checks row, prior 41-VERIFICATION.md line 154).

After Phase 41 commits pushed to `oscarmackjr-twg:main`:

1. **CI run 25970910911** (initial post-push run) failed Linux Test with `error[E0432]: unresolved import 'nono::HandleTarget'` at the Plan 41-01 `request_path()` helper.
2. **Quick task 260516-mxw** landed two commits fixing the import path:
   - `3c1ddc40` — `fix(quick): correct nono::HandleTarget import path`
   - `b284bc63` — `fix(quick): use nono::supervisor::HandleTarget for request_path helper`
3. **CI run 25972316892** on commit `b284bc63` cleared the HandleTarget error but **surfaced 6 NEW errors** on Linux/macOS lanes, all escalated to build failures via `-Dwarnings`:
   - Linux Test job `76346400920`: 4 dead-code errors
   - Linux Clippy job `76346400927`: 1 additional dead-code error (set_all)
   - macOS Clippy job `76346400923`: 1 map_err → inspect_err clippy lint

These 6 errors are real, blocking, and code-level — not pending CI signals. REQ-CI-01 falls back from VERIFIED to **PARTIAL**: the API migration helper + audit_ledger deletion + cfg-gate dispositions remain landed (those parts of SC#1 are achieved), but SC#1's "cross-target Linux/macOS Clippy green from GH Actions" + SC#3 "Linux Clippy + macOS Clippy jobs green on the head of Phase 41" cannot pass with these errors live. SC#4 ("No `#[allow(dead_code)]` added — orphans either deleted or wired") is non-negotiable, so the fix must cfg-gate or wire/delete; bulk `#[allow(dead_code)]` is forbidden.

The other 4 must-haves (REQ-CI-02, REQ-CI-03, REQ-BROKER-CR-01..03, REQ-BROKER-CR-04) remain VERIFIED at the codebase level — none of the new CI errors touch their artifacts. Status transitions: `human_needed` (5/5) → `gaps_found` (4/5).

## CI Evidence — Run 25972316892

**Trigger commit:** `b284bc63` (push of HandleTarget import fix on `oscarmackjr-twg:main`).
**Run URL pattern:** `https://github.com/{org}/nono/actions/runs/25972316892` (specific job IDs below).

### Linux Test job 76346400920 (4 errors)

| # | Error | File | Line | Notes |
|---|-------|------|------|-------|
| 1 | `function \`validate_env_var_patterns\` is never used` | `crates/nono-cli/src/exec_strategy/env_sanitization.rs` | 127 | `pub(crate) fn` declared but no Linux/macOS caller. Closely related to WR-06 — `crates/nono-cli/src/profile_runtime.rs:290` declares a byte-identical `validate_env_var_patterns_local` without delegating. The canonical fix simultaneously closes WR-06: wire profile_runtime.rs:290 to call the canonical `env_sanitization::validate_env_var_patterns` AND ensure at least one Linux test (or Linux-reachable runtime path) reaches it. |
| 2 | `field \`interactive_shell\` is never read` | `crates/nono-cli/src/launch_runtime.rs` | 170 | `ExecutionFlags.interactive_shell: bool`. Read sites all gated to `#[cfg(target_os = "windows")]`: execution_runtime.rs:411 (cfg block 402-416), exec_strategy_windows/mod.rs:669,743, exec_strategy_windows/supervisor.rs:373,434. Fix: add `#[cfg_attr(not(target_os = "windows"), allow(dead_code))]` to the field per the established precedent at launch_runtime.rs:180 (allowed_env_vars) and :185 (denied_env_vars). |
| 3 | `fields \`register_wfp_service, install_wfp_service, install_wfp_driver, start_wfp_service, start_wfp_driver\` are never read` | `crates/nono-cli/src/setup.rs` | 14-18 | All 5 `SetupRunner` fields are WFP-specific and read by 30+ sites (lines 28-32, 52-72, 650-787, 1219-1223) that are themselves Windows-only WFP setup code. Linux/macOS does not invoke these branches, so neither field nor reader is exercised on non-Windows. Fix: either `#[cfg(target_os = "windows")]` on the field declarations + readers, OR extract the WFP setup into a `#[cfg(target_os = "windows")] mod wfp_setup;` submodule. |
| 4 | `methods \`register_phase_index, install_phase_index, start_phase_index, install_driver_phase_index, start_driver_phase_index, recheck_wfp_phase_index\` are never used` | `crates/nono-cli/src/setup.rs` | 737, 741, 745, 752, 756, 771 | Methods on `SetupRunner`; called from WFP setup flow at lines 152, 172, 192, 212, 232, 277 — same Windows-only call surface as the fields above. Fix: same scope as #3 (cfg-gate the entire WFP setup surface together). |

### Linux Clippy job 76346400927 (1 additional error)

| # | Error | File | Line | Notes |
|---|-------|------|------|-------|
| 5 | `associated function \`set_all\` is never used` | `crates/nono-cli/tests/common/test_env.rs` | 26 (within `impl EnvVarGuard` at lines 23-37) | This is the integration-test mirror of `crates/nono-cli/src/test_env.rs::EnvVarGuard` introduced by Plan 41-05. Sole caller: `crates/nono-cli/tests/env_vars.rs:1047`, inside a `#[cfg(target_os = "windows")]` test (line 1039: `windows_run_redirects_profile_state_vars_into_writable_allowlist`). On Linux/macOS the test compiles out, leaving `set_all` orphaned. Fix: either (a) cfg-gate the EnvVarGuard mirror behind `#[cfg(target_os = "windows")]` (since it currently has no non-Windows callers), OR (b) lift the entire `windows_run_*` test cluster into a separate `tests/env_vars_windows.rs` file with module-level `#![cfg(target_os = "windows")]` so the common helper is only compiled when reachable. Option (a) is the minimal-touch fix. |

### macOS Clippy job 76346400923 (1 different error)

| # | Error | File | Line | Notes |
|---|-------|------|------|-------|
| 6 | `using \`map_err\` over \`inspect_err\`` (clippy::manual_inspect) | `crates/nono/src/keystore.rs` | 1074-1078 | `.map_err(\|e\| { let _ = child.kill(); let _ = child.wait(); e })?` is the canonical clippy::manual_inspect pattern — side-effect-only closure that returns the input error unchanged. Inside `#[cfg(target_os = "macos")] load_from_apple_password`. ubuntu-latest clippy did not lift this lint to error level in CI run 25972316892's Linux lane (different toolchain channel or different `-W → -D` escalation list), but macOS Clippy did. Fix (one-line): replace with `.inspect_err(\|_\| { let _ = child.kill(); let _ = child.wait(); })?` — established precedent already in this same file at line 1006 inside `load_from_op` for an identical kill+wait child-cleanup pattern. |

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth (Success Criterion) | Status | Evidence |
|---|---------------------------|--------|----------|
| 1 | REQ-CI-01 SC: cross-target Linux clippy clean from Windows host + GH Actions Linux/macOS Clippy green; no `#[allow(dead_code)]` added — every orphan deleted or wired | **PARTIAL** (regressed from VERIFIED via CI run 25972316892) | API migration helper + audit_ledger deletion + cfg-gate dispositions REMAIN landed (those parts of SC#1 hold). However, CI surfaced 6 new dead-code findings on Linux Test, Linux Clippy, macOS Clippy lanes. See § CI Evidence above. SC#1 cannot pass until all 6 close; SC#3 ("GH Actions Linux Clippy + macOS Clippy green on Phase 41 head") is RED. SC#4 forbids bulk `#[allow(dead_code)]`; fix must cfg-gate or wire/delete each item. |
| 2 | REQ-CI-02 SC: All 5 Windows CI jobs green; MSI validator -BrokerPath mismatch resolved; no [ignored] markers | VERIFIED (code-level; CI green = human-verify) | Unchanged from prior verification. Plan 41-08's `scripts/windows-test-harness.ps1:151-170` fix landed. Live CI confirmation pending next PR push. |
| 3 | REQ-CI-03 SC: Baseline SHA in upstream-sync-quick.md updated to Phase 41 close SHA; SUMMARY frontmatter convention documented; STATE.md ## Deferred Items cleared of v24 CR-A | VERIFIED | Unchanged. `.planning/templates/upstream-sync-quick.md` baseline SHA `13cc0628`; `.planning/phases/41-.../41-SUMMARY.md` `skipped_gates_convention` frontmatter present; `.planning/STATE.md` v24 CR-A row removed and resolution clause appended. |
| 4 | REQ-BROKER-CR-01..03 SC: BrokerNotFound→ErrSandboxInit FFI remap; broker argv rejects null/INVALID/empty handle inputs | VERIFIED | Unchanged. `bindings/c/src/lib.rs:138` mapping; `crates/nono-shell-broker/src/main.rs:103-107` (CR-02) + `:132-136` (CR-03); 4 tests. |
| 5 | REQ-BROKER-CR-04 SC: Job-object test silent-SKIP→FAIL resolved with explicit decision; STATE.md ## Deferred Items cleared of v24 CR-A | VERIFIED | Unchanged. `crates/nono-cli/src/exec_strategy_windows/launch.rs:2450-2458` panic!; `crates/nono-cli/Cargo.toml:109-115` cfg-windows dev-dep; STATE.md updated. |

**Score:** 4/5 truths verified (was 5/5; REQ-CI-01 regressed to PARTIAL via CI evidence).

### Required Artifacts

Carried forward from prior verification — artifact-existence rows unchanged. Net-new artifact concerns introduced by CI run 25972316892:

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/exec_strategy/env_sanitization.rs` | `validate_env_var_patterns` callable from Linux + macOS code paths OR cfg-gated to Windows | FAILED | Line 127: `pub(crate) fn validate_env_var_patterns` has no caller on Linux/macOS (CI 25972316892 Linux Test). Crosscuts WR-06 (profile_runtime.rs:290 byte-identical local copy). |
| `crates/nono-cli/src/launch_runtime.rs` | `ExecutionFlags.interactive_shell` read on Linux/macOS OR cfg-gated | FAILED | Line 170: reader sites all `#[cfg(target_os = "windows")]`. Fix should follow the precedent at line 180 (`#[cfg_attr(not(target_os = "windows"), allow(dead_code))]`). |
| `crates/nono-cli/src/setup.rs` | WFP setup surface (5 fields + 6 methods + 6 readers) reachable on Linux/macOS OR cfg-gated to Windows | FAILED | Lines 14-18 (fields); 28-32, 52-72 (constructor + run-time reads); 152, 172, 192, 212, 232, 277 (phase-index call sites); 650-787 (counter expressions); 737-771 (phase_index methods); 1219-1223 (test fixture). All WFP-specific; Linux/macOS path never exercises them. |
| `crates/nono-cli/tests/common/test_env.rs` | `EnvVarGuard::set_all` reachable on Linux/macOS test runs OR cfg-gated | FAILED | Line 23 (`impl EnvVarGuard`) declares `set_all` at line 26. Sole caller (tests/env_vars.rs:1047) is inside `#[cfg(target_os = "windows")]` test. On Linux/macOS, helper is orphaned. |
| `crates/nono/src/keystore.rs` | Idiomatic `inspect_err` for side-effect-only error handling | FAILED | Lines 1074-1078: `.map_err(\|e\| { let _ = child.kill(); let _ = child.wait(); e })` triggers `clippy::manual_inspect` on macOS Clippy lane. Existing precedent at line 1006 inside `load_from_op` already uses `.inspect_err(\|_e\| { ... })` — apply same shape. |

All other artifact rows from the prior verification remain VERIFIED (`request_path()` helper, audit_ledger deletion, EnvGuard disallowed_methods fences, dangerous_force_wfp_ready wiring, validate-windows-msi-contract.ps1, scripts/windows-test-harness.ps1, bindings/c/src/lib.rs::map_error, crates/nono-shell-broker/src/main.rs argv guards, launch.rs panic!, Cargo.toml dev-dep, upstream-sync-quick.md baseline, 41-SUMMARY.md frontmatter, STATE.md edits). Those rows are not re-listed here for brevity — see `41-VERIFICATION.md` post-41-08 superseded section in git history (commit prior to this re-verification).

### Key Link Verification

Carried forward unchanged from prior verification — no key links broken by the CI run 25972316892 findings. The 6 new gaps are all dead-code / lint findings, not link breakages.

### Data-Flow Trace (Level 4)

Carried forward unchanged from prior verification.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All Plan 41-08 grep checks | (see prior 41-VERIFICATION.md) | unchanged | PASS |
| Cross-target Linux clippy from Windows host | Documented blocked locally; CI Linux Test + Linux Clippy lanes are decisive | CI 25972316892 RED on Linux Test, Linux Clippy, macOS Clippy | **FAIL** (was SKIP — now confirmed FAIL via CI evidence; load-bearing per phase convention) |
| `fn validate_env_var_patterns` location confirmed | `grep -n "fn validate_env_var_patterns" crates/nono-cli/src/exec_strategy/env_sanitization.rs` | matches line 127 | PASS (location confirmed) |
| `interactive_shell` field location confirmed | `grep -n "interactive_shell" crates/nono-cli/src/launch_runtime.rs` | matches line 170 (field), 204, 320 (defaults) | PASS (location confirmed) |
| WFP-service field locations confirmed | `grep -n "register_wfp_service\|install_wfp_service" crates/nono-cli/src/setup.rs` | matches lines 14-18 (fields), readers at 28-32, 52-72, 650-787 | PASS (location confirmed) |
| `phase_index` method locations confirmed | `grep -n "fn register_phase_index\|fn install_phase_index" crates/nono-cli/src/setup.rs` | matches lines 737, 741, 745, 752, 756, 771; call sites at 152, 172, 192, 212, 232, 277 | PASS (location confirmed) |
| `set_all` mirror location confirmed | `grep -n "fn set_all\|impl EnvVarGuard" crates/nono-cli/tests/common/test_env.rs` | matches lines 23, 26 | PASS (location confirmed) |
| macOS `map_err` candidate location confirmed | `grep -nB2 -A5 "let _ = child.kill" crates/nono/src/keystore.rs` | matches lines 1074-1078 inside `#[cfg(target_os = "macos")] load_from_apple_password` | PASS (location confirmed) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| REQ-CI-01 | 41-01, 41-02 | Linux/macOS Clippy lints resolved | **PARTIAL** (regression) | API migration + cfg dispositions landed; 6 NEW dead-code/clippy errors discovered via CI 25972316892. SC#1 + SC#3 RED; SC#4 forbids `#[allow(dead_code)]` so fix must cfg-gate or wire/delete. |
| REQ-CI-02 | 41-03, 41-04, 41-05, 41-08 | Windows CI jobs green (5 jobs) | SATISFIED (code-level); CI green is human-verify | Plan 41-08 closure landed; pending live CI signal |
| REQ-CI-03 | 41-07 | Baseline-aware gate reset + skipped-gates convention + STATE.md cleanup | SATISFIED | Three D-16 commits landed |
| REQ-BROKER-CR-01 | 41-06 | BrokerNotFound FFI not-found mapping | SATISFIED | bindings/c/src/lib.rs:138 |
| REQ-BROKER-CR-02 | 41-06 | Broker null-handle validation | SATISFIED | crates/nono-shell-broker/src/main.rs:103-107 + 2 tests |
| REQ-BROKER-CR-03 | 41-06 | Broker empty-handle-list path | SATISFIED | crates/nono-shell-broker/src/main.rs:132-136 + flipped test |
| REQ-BROKER-CR-04 | 41-07 | Job-object test skip policy | SATISFIED | launch.rs:2450-2458 panic! + Cargo.toml dev-dep (WR-07 deferred) |

### Anti-Patterns Found

Re-classifying the table to reflect the 6 new CI-surfaced findings (BLOCKER class) and carry forward the 7 deferred WARNINGS unchanged.

| File | Line | Pattern | Severity | Status |
|------|------|---------|----------|--------|
| `crates/nono-cli/src/exec_strategy/env_sanitization.rs` | 127 | `pub(crate) fn` never used on Linux/macOS; orphan public-to-crate API | 🛑 BLOCKER (CI-surfaced) | OPEN — requires gap-closure plan |
| `crates/nono-cli/src/launch_runtime.rs` | 170 | Struct field set but never read on Linux/macOS (reader sites all `#[cfg(target_os = "windows")]`) | 🛑 BLOCKER (CI-surfaced) | OPEN — requires gap-closure plan |
| `crates/nono-cli/src/setup.rs` | 14-18 (fields) + 737-771 (methods) | WFP setup surface (5 fields + 6 methods + 6 call sites + 30+ counter expressions) never exercised on Linux/macOS | 🛑 BLOCKER (CI-surfaced) | OPEN — requires gap-closure plan; recommend extracting to `#[cfg(target_os = "windows")] mod wfp_setup;` to cluster the change |
| `crates/nono-cli/tests/common/test_env.rs` | 23-37 (impl + set_all) | Helper used only by `#[cfg(target_os = "windows")]` integration test; orphan on Linux/macOS | 🛑 BLOCKER (CI-surfaced) | OPEN — requires gap-closure plan |
| `crates/nono/src/keystore.rs` | 1074-1078 | `map_err(\|e\| { side-effect; e })` returning input unchanged → clippy::manual_inspect lint | 🛑 BLOCKER (CI-surfaced; macOS Clippy) | OPEN — one-line fix to `.inspect_err(\|_\| { ... })`, precedent at keystore.rs:1006 |
| `crates/nono-cli/Cargo.toml` | 109-115 | Dev-dep builds DEBUG but test only checks RELEASE paths | ⚠️ WARNING (WR-07) | DEFERRED (backlog) — unchanged |
| `crates/nono-cli/tests/common/test_env.rs` | 5-10 | Doc-comment claims "verbatim mirror" but omits `lock_env()` and `EnvVarGuard::remove()` | ⚠️ WARNING (WR-08) | DEFERRED (backlog) — unchanged |
| `crates/nono-cli/src/command_runtime.rs` | 26-29 | `--dangerous-force-wfp-ready` silently dropped on `nono shell`/`nono wrap` | ⚠️ WARNING (WR-01) | DEFERRED (backlog) — unchanged |
| `crates/nono-shell-broker/src/main.rs` | 103-107 | INVALID_HANDLE_VALUE guard misses 32-bit `0xFFFFFFFF` sentinel | ⚠️ WARNING (WR-03) | DEFERRED (backlog) — unchanged |
| `crates/nono-shell-broker/src/main.rs` | 150-167 | `build_command_line` does not reject argv values with interior NUL bytes | ⚠️ WARNING (WR-02) | DEFERRED (backlog) — unchanged |
| `bindings/c/src/lib.rs` | 80-82 | `NoCapabilities \| NoCommand => ErrNoCapabilities` conflates distinct semantics | ⚠️ WARNING (WR-04) | DEFERRED (backlog) — unchanged |
| `bindings/c/src/lib.rs` | 116-119 | `HashMismatch` → `ErrIo` (should be `ErrTrustVerification`); `SessionNotFound` → `ErrIo` (should be `ErrPathNotFound`) | ⚠️ WARNING (WR-05) | DEFERRED (backlog) — unchanged |
| `crates/nono-cli/src/profile_runtime.rs` | 289-306 (now :290) | `validate_env_var_patterns_local` byte-identical to canonical; no lockstep test | ⚠️ WARNING (WR-06) | DEFERRED (backlog) — **CROSS-REFERENCE: see CI-surfaced BLOCKER row 1 above. The gap-closure planner SHOULD consider closing WR-06 simultaneously: wiring profile_runtime.rs:290 to delegate to env_sanitization.rs:127 simultaneously eliminates the orphan AND the duplication.** |

## Deferred (Backlog)

The 7 WARNINGS (WR-01..WR-08) deferred per the prior verification remain deferred. **Important cross-reference:** WR-06 (`validate_env_var_patterns_local` byte-identical to canonical, profile_runtime.rs:290) is closely related to BLOCKER row 1 above (`validate_env_var_patterns` never used, env_sanitization.rs:127). A single edit at profile_runtime.rs:290 — replacing the local copy with a delegation to `env_sanitization::validate_env_var_patterns(patterns, field_name)` — closes WR-06 AND wires the canonical function so the dead-code finding clears without needing a cfg-gate. **The gap-closure planner should consider closing both together** (one PR, one diff).

| Item | File | Brief | Disposition |
|------|------|-------|-------------|
| WR-01 | crates/nono-cli/src/command_runtime.rs:26-29 | `--dangerous-force-wfp-ready` silently dropped on `nono shell`/`nono wrap` | Backlog — defense-in-depth UX hardening |
| WR-02 | crates/nono-shell-broker/src/main.rs:150-167 | `build_command_line` does not reject argv values with interior NUL bytes | Backlog — minimal-attack-surface |
| WR-03 | crates/nono-shell-broker/src/main.rs:103-107 | INVALID_HANDLE_VALUE guard misses 32-bit sentinel `0xFFFFFFFF` | Backlog — defense-in-depth |
| WR-04 | bindings/c/src/lib.rs:80-82 | `NoCapabilities \| NoCommand` conflates distinct semantics | Backlog — FFI error precision |
| WR-05 | bindings/c/src/lib.rs:116-119 | `HashMismatch`/`SessionNotFound` routed to `ErrIo` instead of precise codes | Backlog — FFI error routing |
| WR-06 | crates/nono-cli/src/profile_runtime.rs:290 | byte-identical local copy of `validate_env_var_patterns`; no lockstep test | **Backlog with cross-reference** — closing this WR simultaneously closes CI-surfaced BLOCKER row 1; planner should fold together |
| WR-07 | crates/nono-cli/Cargo.toml:109-115 | Dev-dep builds DEBUG but test only checks RELEASE | Backlog — dev-loop UX |
| WR-08 | crates/nono-cli/tests/common/test_env.rs:5-10 | Mirror omits `lock_env()`/`EnvVarGuard::remove()` | Backlog — doc freshness |

## Human Verification Required

#### 1. CI run 25972316892 + successor runs no longer hit -Dwarnings dead-code errors after the gap-closure plan lands (NEW)

**Test:** After the next gap-closure plan lands the 6 fixes documented in § CI Evidence, observe the next push's GH Actions runs.
**Expected:** Linux Test, Linux Clippy, macOS Clippy lanes all PASS. No occurrence of any of the following in lane logs: `function \`validate_env_var_patterns\` is never used`, `field \`interactive_shell\` is never read`, `fields \`register_wfp_service`, `methods \`register_phase_index`, `associated function \`set_all\` is never used`, `using \`map_err\` over \`inspect_err\``.
**Why human:** Live CI signal; not reproducible locally without C cross-compiler for Linux clippy from this Windows dev host. NEW for this re-verification.

#### 2. windows-build CI lane no longer fails at PowerShell parameter binding after Plan 41-08 lands

**Test:** On the next push to the Phase 41 PR branch carrying commit `c0a89227` (the Plan 41-08 fix), inspect the GH Actions `windows-build` job's `Run Windows build harness` step output (`ci-logs/windows-build.log`).
**Expected:** NO line matching `Cannot process command because of one or more missing mandatory parameters: BrokerPath`; the new `==> build nono-shell-broker` label appears followed by a successful `cargo build -p nono-shell-broker`; the `==> validate windows msi contract` label is followed by NO Test-Path failure.
**Why human:** Decisive live signal lives in GH Actions; not reproducible locally. Carried forward from prior verification.

#### 3. All 7 (now 8 with Linux Test) GH Actions CI lanes green on Phase 41 close SHA

**Test:** Open / refresh the Phase 41 PR and inspect CI status for all lanes (Linux Clippy, Linux Test, macOS Clippy, Windows Build, Windows Integration, Windows Regression, Windows Security, Windows Packaging) on the head SHA after the gap-closure plan lands.
**Expected:** All lanes PASS on the same head commit.
**Why human:** REQ-CI-01 SC#3 + REQ-CI-02 SC#1 require GH Actions green on Phase 41 close SHA; not reproducible locally. Carried forward.

#### 4. env_vars parallel flake fix (Plan 41-05) on Windows host

**Test:** On a Windows host, run `cargo test -p nono-cli --test env_vars windows_run_redirects_profile_state_vars_into_writable_allowlist` 10 times back-to-back in parallel mode.
**Expected:** 0 failures across 10 runs.
**Why human:** Plan 41-05 did not execute the 10x verification on the current dev host. Carried forward.

#### 5. Block-net probe tests on elevated Windows CI runner

**Test:** Verify `windows_run_block_net_blocks_probe_connection` + `windows_run_block_net_blocks_probe_through_cmd_host` pass on a Windows runner with `NONO_CI_HAS_WFP=true` and WFP service installed.
**Expected:** Both tests pass with "connect failed" or "exit code 42" markers in stderr.
**Why human:** Local dev host short-circuits the probe path. Carried forward.

#### 6. Cross-binding (nono-py / nono-ts) D-10 verification of CR-01 FFI remap

**Test:** `grep -rn 'ErrPathNotFound\|errorCode.*-1' ../nono-py/ ../nono-ts/` from a workspace with both sibling repos checked out.
**Expected:** No integer-mapping of `-1` (ErrPathNotFound) as broker-discovery-failure semantics.
**Why human:** Sibling repos not present in this working directory. Carried forward.

## Gaps Summary

CI run 25972316892 surfaced 6 BLOCKER-class findings on Linux Test, Linux Clippy, and macOS Clippy lanes after the HandleTarget import fix landed. All 6 are -Dwarnings escalated dead-code or `clippy::manual_inspect` lints that the prior local verification could not catch because cross-target clippy from Windows host was SKIPPED (the documented load-bearing risk). The 6 findings are concentrated in 4 files:

1. `crates/nono-cli/src/exec_strategy/env_sanitization.rs:127` — `validate_env_var_patterns` orphan (closely related to WR-06)
2. `crates/nono-cli/src/launch_runtime.rs:170` — `interactive_shell` field orphan on non-Windows
3. `crates/nono-cli/src/setup.rs:14-18, 737-771` — WFP setup surface orphan on non-Windows (largest single fix; group with field readers at 28-32, 52-72, 650-787, call sites at 152-277, fixture at 1219-1223)
4. `crates/nono-cli/tests/common/test_env.rs:23-37` — `EnvVarGuard::set_all` mirror orphan on non-Windows
5. `crates/nono/src/keystore.rs:1074-1078` — `map_err` → `inspect_err` macOS-only clippy lint (one-line fix, precedent at line 1006)

REQ-CI-01 falls back from VERIFIED to **PARTIAL**; status transitions from `human_needed` (5/5) to `gaps_found` (4/5). The other 4 must-haves (REQ-CI-02, REQ-CI-03, REQ-BROKER-CR-01..03, REQ-BROKER-CR-04) remain VERIFIED at the codebase level.

Recommendation for the next gap-closure plan: fold the env_sanitization fix together with WR-06 (one diff: wire `profile_runtime.rs:290` to delegate to `env_sanitization.rs:127`) so the orphan clears and WR-06 closes simultaneously. The setup.rs WFP-surface fix is the largest single change; extracting to `#[cfg(target_os = "windows")] mod wfp_setup;` may be cleaner than line-by-line cfg-gating. The keystore.rs macOS fix is one-line. Bulk `#[allow(dead_code)]` is NOT acceptable per REQ-CI-01 SC#4.

---

_Verified: 2026-05-16T20:30:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification supersedes: 2026-05-16T19:30:00Z (post-Plan-41-08 verification, status: human_needed)_
_CI trigger: run 25972316892 on commit b284bc63 surfaced 6 -Dwarnings dead-code errors on Linux Test (job 76346400920), Linux Clippy (job 76346400927), and macOS Clippy (job 76346400923)_
