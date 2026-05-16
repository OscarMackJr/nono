---
phase: 41-ci-cleanup-v24-broker-code-review-closure
verified: 2026-05-16T19:30:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  previous_verified: 2026-05-16T00:00:00Z
  gaps_closed:
    - "REQ-CI-02 SC#2: MSI validator -BrokerPath mandatory-parameter mismatch at scripts/windows-test-harness.ps1:147 (BLOCKER from prior Anti-Patterns row 1)"
  gaps_remaining: []
  regressions: []
must_haves:
  truths:
    - "REQ-CI-01: cross-target Linux clippy clean; no new raw #[allow(dead_code)]; orphans deleted or cfg-gated"
    - "REQ-CI-02: 5 Windows CI jobs (Build, Integration, Regression, Security, Packaging) green; MSI validator -BrokerPath mismatch resolved; no unjustified #[ignored]"
    - "REQ-CI-03: baseline-aware CI gate baseline SHA + skipped-gates convention + STATE.md ## Deferred Items cleanup"
    - "REQ-BROKER-CR-01..03: BrokerNotFound FFI remap + broker null/INVALID + empty-list rejects"
    - "REQ-BROKER-CR-04: Job-object test silent-SKIP→FAIL resolved; STATE.md v24 CR-A entries cleared"
human_verification:
  - test: "Verify the windows-build CI lane no longer fails at PowerShell parameter binding on the next PR push after Plan 41-08 lands"
    expected: "ci-logs/windows-build.log contains NO 'Cannot process command because of one or more missing mandatory parameters: BrokerPath' line; the build suite progresses past 'validate windows msi contract' label; cargo build -p nono-shell-broker step appears and succeeds; Test-Path guard passes silently"
    why_human: "Plan 41-08 closed the gap at the codebase level (verified by grep + PowerShell syntax check), but the decisive live signal — GH Actions windows-build job green on PR head SHA — lives in CI and is not reproducible from this dev host (cargo build --workspace takes ~10 minutes; full validator run requires WiX tooling). REQ-CI-02 SC#2 fully closes only after this lane is observed green on the post-Plan-41-08 SHA."
  - test: "Verify all 7 GitHub Actions CI lanes green on Phase 41 close SHA (post-Plan-41-08 head)"
    expected: "Linux Clippy + macOS Clippy + Windows Build + Windows Integration + Windows Regression + Windows Security + Windows Packaging all PASS on the same head SHA"
    why_human: "Lives in GitHub Actions; not reproducible locally. REQ-CI-01/02 SC require GH Actions green on Phase 41 close SHA. Carried forward from prior verification — was item #1."
  - test: "Verify the env_vars parallel flake fix on a real Windows host (cargo test -p nono-cli --test env_vars windows_run_redirects_profile_state_vars_into_writable_allowlist run 10x in parallel)"
    expected: "0 failures across 10 parallel runs"
    why_human: "Plan 41-05 used Windows-host-only verification; current dev host could not execute the flake check (10x runs). CI Integration job covers this on Windows-latest. Carried forward from prior verification — was item #2."
  - test: "Verify the block-net probe tests pass on a Windows host with NONO_CI_HAS_WFP=true (elevated, WFP service installed)"
    expected: "windows_run_block_net_blocks_probe_connection + windows_run_block_net_blocks_probe_through_cmd_host both PASS with 'connect failed' or 'exit code 42' markers in stderr"
    why_human: "Plan 41-04 short-circuits on non-elevated dev hosts; full probe path runs only on elevated CI runner. Carried forward from prior verification — was item #3."
  - test: "Verify cross-binding nono-py / nono-ts impact of CR-01 FFI remap"
    expected: "No integer-mapping of -1 (ErrPathNotFound) as broker-discovery-failure in downstream bindings — or follow-up todo filed for lockstep"
    why_human: "../nono-py/ and ../nono-ts/ are sibling repositories not present in this working directory; D-10 manual verification was deferred per Plan 41-06 SUMMARY. Carried forward from prior verification — was item #4."
---

# Phase 41: CI cleanup + v24 broker code-review closure Verification Report

**Phase Goal:** Reset every CI lane to green and clear the v24 Windows broker code-review backlog so Phases 42 + 43 inherit a clean baseline. This is the v2.5 prerequisite phase: subsequent baseline-aware CI gates (REQ-UPST5-02) become unambiguously real regression detectors rather than baseline-drift trackers.

**Verified:** 2026-05-16T19:30:00Z (re-verification after Plan 41-08 gap closure)
**Status:** human_needed
**Re-verification:** Yes — supersedes 2026-05-16T00:00:00Z verification

## Re-verification Summary

The prior verification (2026-05-16T00:00:00Z) returned `status: gaps_found` (4/5 must-haves) with ONE BLOCKER: `scripts/windows-test-harness.ps1:147` invoked the MSI validator without the mandatory `-BrokerPath` parameter introduced by Plan 41-03, causing the GitHub Actions `windows-build` job to fail at PowerShell parameter binding every run.

Plan 41-08 (Wave 3, gap_closure: true) landed two commits closing this gap:
- `c0a89227` — `fix(41-08): close REQ-CI-02 BrokerPath gap in windows-test-harness build suite`
- `55640d72` — `docs(41-08): record gap closure SUMMARY for REQ-CI-02 BrokerPath fix`

Verification of the gap closure at the codebase level: **CONFIRMED**. The fix landed exactly as described in 41-08-PLAN.md:
1. Explicit `Invoke-LoggedCargo` for `cargo build -p nono-shell-broker` inserted at `scripts/windows-test-harness.ps1:151-155` immediately after the workspace build.
2. Fail-secure `Test-Path -LiteralPath $brokerPath` guard with `Write-Error` + `throw` at lines 163-167.
3. Validator invocation expanded to multi-line with backtick continuation at lines 168-170, threading both `-BinaryPath` AND `-BrokerPath $brokerPath` (pointing at `target\debug\nono-shell-broker.exe`).

Repo-wide audit (Grep across all non-`.md` files): only **two** validator-invocation sites exist — `.github/workflows/ci.yml:343` (windows-packaging, fixed by Plan 41-03) and `scripts/windows-test-harness.ps1:168` (windows-build, fixed by Plan 41-08). Both thread `-BrokerPath`. No third caller exists.

Truth #2 transitions FAILED → VERIFIED. The other 4 truths are re-confirmed unchanged. Score: **5/5**. Status would be `passed` were it not for 5 outstanding `human_verification` items that all live on CI lanes / Windows-host hardware / sibling repos and cannot be locally executed — per the verification process Step 9 decision tree, human-verification items take priority and yield `status: human_needed`.

The 7 WARNINGS (WR-01..WR-08 from `41-REVIEW.md`) are **explicitly deferred** per user "Blocker only" scope decision on the gap closure (2026-05-16). See § Deferred (Backlog).

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth (Success Criterion) | Status | Evidence |
|---|---------------------------|--------|----------|
| 1 | REQ-CI-01 SC: cross-target Linux clippy clean from Windows host + GH Actions Linux/macOS Clippy green; no `#[allow(dead_code)]` added — every orphan deleted or wired | VERIFIED (code-level; CI green = human-verify) | Unchanged from prior verification. `crates/nono-cli/src/audit_ledger.rs` deleted (file gone, `mod audit_ledger;` removed from `main.rs`); `audit_integrity.rs:217`, `session.rs:827-ish`, `exec_strategy.rs:376` carry `#[cfg_attr(not(target_os = "windows"), allow(dead_code))]`; `wait_for_child(child)` unreachable trailer deleted; `profile_runtime.rs` `EnvGuard` carries per-block `#[allow(clippy::disallowed_methods)]` with rationale; `request_path()` helper added at `exec_strategy.rs:2634` with 14 migrated call sites |
| 2 | REQ-CI-02 SC: All 5 Windows CI jobs green; MSI validator -BrokerPath mismatch resolved; no [ignored] markers | **VERIFIED** (transitioned FAILED → VERIFIED via Plan 41-08) | `scripts/windows-test-harness.ps1:151-155` pre-builds the broker via `Invoke-LoggedCargo` with `"build", "-p", "nono-shell-broker"` CargoArgs; lines 163-167 guard with `Test-Path -LiteralPath $brokerPath` + `Write-Error` + `throw` on missing artifact (CLAUDE.md Fail Secure); lines 168-170 invoke `validate-windows-msi-contract.ps1` with both `-BinaryPath (Join-Path $PWD "target\debug\nono.exe")` AND `-BrokerPath $brokerPath` via backtick line-continuation. Plan 41-08 commits `c0a89227` + `55640d72` on the branch. Repo-wide grep audit: exactly 2 validator-invocation sites exist (`.github/workflows/ci.yml:343` + `scripts/windows-test-harness.ps1:168`); both pass `-BrokerPath`. Live CI-lane confirmation pending next PR push (see human_verification item #1). Block-net probe wiring + env_vars EnvVarGuard pinning remain in place from Plans 41-04 + 41-05. |
| 3 | REQ-CI-03 SC: Baseline SHA in upstream-sync-quick.md updated to Phase 41 close SHA; SUMMARY frontmatter convention documented; STATE.md ## Deferred Items cleared of v24 CR-A | VERIFIED | Unchanged from prior verification. `.planning/templates/upstream-sync-quick.md:96-115` contains `## Baseline-aware CI gate` section with baseline SHA `13cc0628`; `.planning/phases/41-.../41-SUMMARY.md` contains `skipped_gates_convention` frontmatter with `load_bearing` + `environmental` sub-keys; `.planning/STATE.md:233` rewritten to remove `+ 4 v24 CR todos` and append "v24 CR-A class (4 todos) resolved by Phase 41; cleared 2026-05-16" |
| 4 | REQ-BROKER-CR-01..03 SC: BrokerNotFound→ErrSandboxInit FFI remap; broker argv rejects null/INVALID/empty handle inputs | VERIFIED | Unchanged from prior verification. `bindings/c/src/lib.rs:138` maps `BrokerNotFound { .. } => NonoErrorCode::ErrSandboxInit` with D-09 doc-comment block at 132-137; `crates/nono-shell-broker/src/main.rs:103-107` rejects `raw_value == 0 \|\| raw_value == usize::MAX` (CR-02); lines 132-136 reject `inherit_handles.is_empty()` (CR-03); 4 tests added/flipped (RED→GREEN via commits 2bf6f4b5, 00b75939, be26ce3a) |
| 5 | REQ-BROKER-CR-04 SC: Job-object test silent-SKIP→FAIL resolved with explicit decision; STATE.md ## Deferred Items cleared of v24 CR-A | VERIFIED | Unchanged from prior verification. `crates/nono-cli/src/exec_strategy_windows/launch.rs:2450-2458` replaces `eprintln!` + `return;` with `panic!("nono-shell-broker.exe missing at {} and {}; ...")`; `crates/nono-cli/Cargo.toml:109-115` adds `[target.'cfg(target_os = "windows")'.dev-dependencies] nono-shell-broker = { path = "../nono-shell-broker", version = "0.53.0" }`; STATE.md v24 CR-A row removed (see Truth 3) |

**Score:** 5/5 truths verified (was 4/5 in prior verification)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/exec_strategy.rs` | `request_path()` helper + 14 migrated call sites | VERIFIED | Helper at line 2634; 14 `request_path(&request)` call sites |
| `crates/nono-cli/src/audit_ledger.rs` | DELETED | VERIFIED | File absent; `mod audit_ledger;` removed from `main.rs` |
| `crates/nono-cli/src/profile_runtime.rs` | EnvGuard + Drop with `#[allow(clippy::disallowed_methods)]` per-block rationale | VERIFIED | Matches `test_env.rs:24,56` D-08 precedent |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | `set_windows_wfp_test_force_ready` ungated + NONO_TEST_HARNESS runtime guard | VERIFIED | Lines 397-417 |
| `crates/nono-cli/src/cli.rs` | `dangerous_force_wfp_ready` field with no `#[cfg(debug_assertions)]` gate; `hide = true` preserved | VERIFIED | Lines 1643-1645 |
| `crates/nono-cli/src/command_runtime.rs` | Wiring from `args.dangerous_force_wfp_ready` to setter | VERIFIED | Lines 26-29 |
| `crates/nono-cli/tests/env_vars.rs` | EnvVarGuard::set_all pinning 6 Windows env vars in flaky test | VERIFIED | Line 1047 |
| `crates/nono-cli/tests/common/test_env.rs` | Mirror of EnvVarGuard for integration test boundary | VERIFIED (with WR-08 deferred) | File created; 50 lines; mirrors `set_all` + Drop with disallowed_methods fences. WR-08 (mirror omits `lock_env()` + `EnvVarGuard::remove()`) deferred to backlog |
| `scripts/validate-windows-msi-contract.ps1` | Mandatory `-BrokerPath` threaded through `Get-WixDocumentForScope` | VERIFIED | Top-level param block (line 8), function param, `$buildArgs["BrokerPath"]`, both call sites, `Resolve-Path` for `$BrokerPath` all in place |
| `scripts/windows-test-harness.ps1` | All validator invocations pass mandatory `-BrokerPath` | **VERIFIED** (Plan 41-08) | Lines 151-155: explicit `Invoke-LoggedCargo` pre-builds broker. Lines 163-167: `Test-Path -LiteralPath $brokerPath` + `Write-Error` + `throw` fail-secure guard. Lines 168-170: multi-line validator invocation with backtick continuation passing `-BinaryPath` AND `-BrokerPath $brokerPath`. Confirmed in commit `c0a89227`. |
| `bindings/c/src/lib.rs` | BrokerNotFound → ErrSandboxInit + new FFI test | VERIFIED | Line 138 mapping; inline test `broker_not_found_maps_to_err_sandbox_init` |
| `bindings/c/src/types.rs` | `ErrSandboxInit` doc-comment enumerates BrokerNotFound | VERIFIED | Plan 41-06 SUMMARY (Rule 2 auto-fix: cbindgen reads variant doc-comment, not match-arm comment) |
| `bindings/c/include/nono.h` | Auto-regenerated by cbindgen reflecting D-09 remap | VERIFIED (per SUMMARY) | Plan 41-06 `git diff` non-empty after cbindgen regen (5 new lines in NONO_ERROR_CODE_ERR_SANDBOX_INIT comment) |
| `crates/nono-shell-broker/src/main.rs` | Argv parser rejects null/INVALID + empty-list | VERIFIED | Lines 103-107 (CR-02); 132-136 (CR-03); tests at end of file |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | `broker_launch_assigns_child_to_job_object` panics on missing artifact | VERIFIED | Lines 2450-2458: panic! replaces eprintln!+return; |
| `crates/nono-cli/Cargo.toml` | `[target.'cfg(target_os = "windows")'.dev-dependencies]` declares nono-shell-broker | VERIFIED (with WR-07 deferred) | Lines 109-115: dev-dep present with Phase 41 D-14 rationale. WR-07 (dev-dep builds DEBUG; test checks RELEASE paths) deferred to backlog |
| `.planning/templates/upstream-sync-quick.md` | `## Baseline-aware CI gate` section with Phase 41 close SHA | VERIFIED | Section at lines 96-115; baseline SHA `13cc0628` |
| `.planning/phases/41-.../41-SUMMARY.md` | Frontmatter contains `skipped_gates_convention` block with load_bearing + environmental keys | VERIFIED | Lines 1-14: both keys present |
| `.planning/STATE.md` | v24-cr-0[1-4] row removed; summary line updated | VERIFIED | Line 233 updated; v24 CR-A resolution clause appended |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `scripts/windows-test-harness.ps1:168-170` (build suite) | `scripts/validate-windows-msi-contract.ps1` | direct invocation passing `-BinaryPath` AND `-BrokerPath` via backtick continuation | **WIRED** (transitioned BROKEN → WIRED via Plan 41-08) | Multi-line `&` invocation at lines 168-170 threads both arguments; PowerShell parameter binding now succeeds. Defense-in-depth: `Test-Path` guard at 164 + `cargo build -p nono-shell-broker` pre-step at 151-155 ensure the artifact exists before invocation. |
| `scripts/windows-test-harness.ps1:151-155` (build suite) | `target\debug\nono-shell-broker.exe` | explicit `Invoke-LoggedCargo` with `"build", "-p", "nono-shell-broker"` CargoArgs | WIRED | New artifact-production step ensures broker binary exists for the validator call below |
| `.github/workflows/ci.yml:151-153` (windows-build job) | `scripts/windows-test-harness.ps1 -Suite build` | pwsh invocation | WIRED | Unchanged — CI driver of the now-fixed harness |
| `scripts/validate-windows-msi-contract.ps1` (top param) | `Get-WixDocumentForScope -BrokerBinary` | function param thread-through | WIRED | Plan 41-03 |
| `Get-WixDocumentForScope` | `scripts/build-windows-msi.ps1` | `$buildArgs["BrokerPath"]` splat | WIRED | Plan 41-03 |
| `.github/workflows/ci.yml:343-346` (windows-packaging job) | `scripts/validate-windows-msi-contract.ps1` | direct invocation with `-BinaryPath`, `-BrokerPath`, `-ServiceBinaryPath` | WIRED | Plan 41-03 (validator's only other invocation site; repo-wide audit confirms 2 callers total, both fixed) |
| `SandboxArgs.dangerous_force_wfp_ready` (clap field) | `set_windows_wfp_test_force_ready` (atomic setter) | `command_runtime.rs` cfg(windows) block | WIRED (with WR-01 deferred) | Lines 26-29; WR-01 deferred (silently ignored on `shell`/`wrap`) |
| `set_windows_wfp_test_force_ready` | `WINDOWS_WFP_TEST_FORCE_READY` atomic | `NONO_TEST_HARNESS` runtime guard then store | WIRED | exec_strategy_windows/mod.rs:404-417 |
| `bindings/c/src/lib.rs::map_error` | `NonoErrorCode::ErrSandboxInit` (-6) | `BrokerNotFound { .. } => ErrSandboxInit` match arm | WIRED | Line 138 |
| `broker argv parser` | `NonoError::SandboxInit` (rejection) | `if raw_value == 0 \|\| raw_value == usize::MAX` + `if inherit_handles.is_empty()` | WIRED | crates/nono-shell-broker/src/main.rs:103-107 + 132-136 |
| `broker_launch_assigns_child_to_job_object` test | `panic!` on missing artifact | else branch of candidate_triple/candidate_default check | WIRED | launch.rs:2450-2458 |
| `crates/nono-cli` test build | broker artifact | cargo dev-dependency | PARTIAL (WR-07 deferred) | Dev-dep declared (Cargo.toml:109-115); dev-dep builds DEBUG but the test only searches `target/release/` paths — test still panics without prior release-mode pre-build. WR-07 deferred to backlog per user scope decision. |

### Data-Flow Trace (Level 4)

Phase 41 is a CI cleanup + hardening phase, not a feature phase rendering dynamic data. The data-flow concerns are:

| Artifact | Data Variable | Source | Produces Real Effect | Status |
|----------|---------------|--------|----------------------|--------|
| `dangerous_force_wfp_ready` flag | `WINDOWS_WFP_TEST_FORCE_READY` atomic | Clap parse → command_runtime wiring → setter (guarded by NONO_TEST_HARNESS) | Yes (when env var set; only on `run` subcommand per WR-01) | FLOWING |
| MSI validator `-BrokerPath` | `$brokerPath` → `Get-WixDocumentForScope -BrokerBinary` → `$buildArgs["BrokerPath"]` → build script | Plan 41-03 thread-through + Plan 41-08 caller fix | Yes from BOTH callers (windows-packaging + windows-build) | **FLOWING from both caller sites** (was FLOWING from packaging only / HOLLOW_PROP from windows-build harness; Plan 41-08 closed the gap) |
| Broker pre-build artifact | `target\debug\nono-shell-broker.exe` | `Invoke-LoggedCargo` step at windows-test-harness.ps1:151-155 with `Test-Path` guard at 164 | Yes — artifact produced by explicit cargo build step; existence verified before validator call | FLOWING (new in Plan 41-08) |
| Broker FFI error code | `NonoError::BrokerNotFound` → `map_error` → `ErrSandboxInit` (-6) | bindings/c/src/lib.rs:138 | Yes | FLOWING |
| Broker argv null/INVALID/empty rejects | argv → parse_args → SandboxInit error → broker exits non-zero | crates/nono-shell-broker/src/main.rs:103-107 + 132-136 | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| audit_ledger.rs deleted | `ls crates/nono-cli/src/audit_ledger.rs` | "No such file or directory" | PASS |
| `mod audit_ledger;` removed | `grep -c "mod audit_ledger" crates/nono-cli/src/main.rs` | 0 | PASS |
| baseline-aware section present in template | `grep -c "^## Baseline-aware CI gate$" .planning/templates/upstream-sync-quick.md` | 1 | PASS |
| Phase 41 close SHA stamped | `grep "Current baseline SHA: \`13cc0628\`" .planning/templates/upstream-sync-quick.md` | match | PASS |
| `skipped_gates_convention` frontmatter present | `head -14 .planning/phases/41-.../41-SUMMARY.md` shows load_bearing + environmental | match | PASS |
| v24 CR-A row removed from STATE.md | `grep -c "v24-cr-0" .planning/STATE.md` | 0 | PASS |
| v24 CR-A resolution clause present | `grep "v24 CR-A class.*resolved by Phase 41" .planning/STATE.md` | 1 hit | PASS |
| BrokerNotFound→ErrSandboxInit | `grep "BrokerNotFound .* ErrSandboxInit" bindings/c/src/lib.rs` | match at :138 | PASS |
| Broker null/INVALID reject in parser | `grep "is null or INVALID_HANDLE_VALUE" crates/nono-shell-broker/src/main.rs` | match | PASS |
| Broker empty-list reject in parser | `grep "inherit_handles.is_empty" crates/nono-shell-broker/src/main.rs` | match | PASS |
| Launch.rs SKIP→FAIL | `grep "nono-shell-broker.exe missing" crates/nono-cli/src/exec_strategy_windows/launch.rs` | match in panic! | PASS |
| **Test harness build suite calls validator with -BrokerPath** | Multi-line grep `validate-windows-msi-contract\.ps1[\s\S]{0,200}-BrokerPath` on scripts/windows-test-harness.ps1 | matches lines 168 (invocation) → 170 (`-BrokerPath $brokerPath`) | **PASS** (was FAIL in prior verification) |
| Test harness pre-builds broker | `grep -nE "Label \"build nono-shell-broker\"" scripts/windows-test-harness.ps1` | match at line 151 | PASS (new in Plan 41-08) |
| Test harness Test-Path guard on broker | `grep -n "Test-Path -LiteralPath \$brokerPath" scripts/windows-test-harness.ps1` | match at line 164 | PASS (new in Plan 41-08) |
| Test harness fail-secure throw | `grep -n "throw \"MSI validator pre-check failed" scripts/windows-test-harness.ps1` | match at line 166 | PASS (new in Plan 41-08) |
| Repo-wide validator caller inventory | `grep -rn "validate-windows-msi-contract" .github/ scripts/ tests/` (excluding `.md`) | exactly 2 invocations: `.github/workflows/ci.yml:343` + `scripts/windows-test-harness.ps1:168` (plus 1 comment ref at :157); both invocations pass `-BrokerPath` | PASS |
| Cross-target Linux clippy from Windows host | Documented blocked by missing C cross-compiler in plan SUMMARYs | SKIPPED — load-bearing per CI Linux native lane | SKIP (load-bearing per phase convention) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| REQ-CI-01 | 41-01, 41-02 | Linux/macOS Clippy lints resolved (33 errors per tracker) | SATISFIED (code-level); CI green is human-verify | API migration helper + audit_ledger deletion + cfg-gate dispositions + EnvGuard disallowed_methods fence + unreachable delete |
| REQ-CI-02 | 41-03, 41-04, 41-05, **41-08** | Windows CI jobs green (5 jobs) | **SATISFIED** (code-level; CI green is human-verify) | Plans 41-03 (validator + windows-packaging caller), 41-04 (dangerous-force-wfp-ready ungate + runtime guard), 41-05 (env_vars EnvVarGuard flake pin), **41-08 (windows-test-harness build-suite BrokerPath gap closure)** — all landed at code level. The PowerShell parameter binding failure that previously broke the windows-build CI lane every run is resolved. CI-side verification pending next PR push (human_verification item #1). |
| REQ-CI-03 | 41-07 | Baseline-aware gate reset + skipped-gates convention + STATE.md cleanup | SATISFIED | Three D-16 commits landed (baseline SHA stamped, frontmatter convention, STATE.md edits) |
| REQ-BROKER-CR-01 | 41-06 | BrokerNotFound FFI not-found mapping | SATISFIED | bindings/c/src/lib.rs:138 + types.rs doc-comment + inline FFI test |
| REQ-BROKER-CR-02 | 41-06 | Broker null-handle validation | SATISFIED | crates/nono-shell-broker/src/main.rs:103-107 + 2 tests (null + INVALID_HANDLE_VALUE) |
| REQ-BROKER-CR-03 | 41-06 | Broker empty-handle-list path | SATISFIED | crates/nono-shell-broker/src/main.rs:132-136 + flipped test (_is_ok → _returns_error) |
| REQ-BROKER-CR-04 | 41-07 | Job-object test skip policy | SATISFIED | launch.rs:2450-2458 panic! replaces silent SKIP; Cargo.toml dev-dep automates pre-build (WR-07 deferred) |

### Anti-Patterns Found

The Anti-Patterns table from the prior verification has been re-classified: the BLOCKER (Plan 41-08's target) is resolved; the 7 WARNINGS are explicitly deferred — see § Deferred (Backlog).

| File | Line | Pattern | Severity | Status |
|------|------|---------|----------|--------|
| `scripts/windows-test-harness.ps1` | 147 (pre-edit) | Call to validator missing mandatory `-BrokerPath` argument | 🛑 BLOCKER | **RESOLVED by Plan 41-08** — lines 168-170 now pass `-BrokerPath $brokerPath` via backtick line-continuation; lines 151-155 pre-build the broker; lines 163-167 fail-secure on missing artifact |
| `crates/nono-cli/Cargo.toml` | 109-115 | Comment claims dev-dep eliminates manual pre-build, but cargo test builds DEBUG profile while the test only checks `target/release/` paths | ⚠️ WARNING (WR-07) | DEFERRED (backlog) |
| `crates/nono-cli/tests/common/test_env.rs` | 5-10 | Doc-comment claims "verbatim mirror" but omits `lock_env()` and `EnvVarGuard::remove()` | ⚠️ WARNING (WR-08) | DEFERRED (backlog) |
| `crates/nono-cli/src/command_runtime.rs` | 26-29 | `--dangerous-force-wfp-ready` wired only on `run_sandbox`; `nono shell` and `nono wrap` silently drop the flag | ⚠️ WARNING (WR-01) | DEFERRED (backlog) |
| `crates/nono-shell-broker/src/main.rs` | 103-107 | INVALID_HANDLE_VALUE guard uses `usize::MAX` but does NOT reject the 32-bit sentinel `0xFFFFFFFF` on 64-bit hosts | ⚠️ WARNING (WR-03) | DEFERRED (backlog) |
| `crates/nono-shell-broker/src/main.rs` | 150-167 | `build_command_line` does not reject argv values containing interior NUL bytes | ⚠️ WARNING (WR-02) | DEFERRED (backlog) |
| `bindings/c/src/lib.rs` | 80-82 | `NoCapabilities \| NoCommand => ErrNoCapabilities` conflates two semantically distinct errors | ⚠️ WARNING (WR-04) | DEFERRED (backlog) |
| `bindings/c/src/lib.rs` | 116-119 | `HashMismatch` mapped to generic `ErrIo` instead of `ErrTrustVerification`; `SessionNotFound` mapped to `ErrIo` instead of `ErrPathNotFound` | ⚠️ WARNING (WR-05) | DEFERRED (backlog) |
| `crates/nono-cli/src/profile_runtime.rs` | 289-306 | `validate_env_var_patterns_local` is byte-identical to canonical in env_sanitization.rs:127-143; no test asserts lockstep | ⚠️ WARNING (WR-06) | DEFERRED (backlog) |

## Deferred (Backlog)

The 7 WARNINGS (WR-01..WR-08, equivalently 8 distinct entries with WR-07 + WR-08 as Cargo.toml + test_env.rs items) flagged in the prior `41-VERIFICATION.md` Anti-Patterns table and originating in `41-REVIEW.md` are explicitly deferred.

**Per user scope decision on 41-08 gap closure (2026-05-16). Carry forward to v2.5 milestone backlog or future hardening phase. Not blocking Phase 41 close.**

| Item | File | Brief | Disposition |
|------|------|-------|-------------|
| WR-01 | crates/nono-cli/src/command_runtime.rs:26-29 | `--dangerous-force-wfp-ready` silently dropped on `nono shell` / `nono wrap` subcommands; clap accepts the flag without effect | Backlog — defense-in-depth UX hardening |
| WR-02 | crates/nono-shell-broker/src/main.rs:150-167 | `build_command_line` does not reject argv values with interior NUL bytes | Backlog — minimal-attack-surface contract item; broker trusts parent today |
| WR-03 | crates/nono-shell-broker/src/main.rs:103-107 | INVALID_HANDLE_VALUE guard misses 32-bit sentinel `0xFFFFFFFF` on 64-bit hosts | Backlog — defense-in-depth |
| WR-04 | bindings/c/src/lib.rs:80-82 | `NoCapabilities \| NoCommand => ErrNoCapabilities` conflates distinct semantics | Backlog — FFI error precision |
| WR-05 | bindings/c/src/lib.rs:116-119 | `HashMismatch` → `ErrIo` (should be `ErrTrustVerification`); `SessionNotFound` → `ErrIo` (should be `ErrPathNotFound`) | Backlog — FFI error routing (security-relevant for C consumers) |
| WR-06 | crates/nono-cli/src/profile_runtime.rs:289-306 | `validate_env_var_patterns_local` byte-identical to canonical; no lockstep test | Backlog — drift-risk hardening |
| WR-07 | crates/nono-cli/Cargo.toml:109-115 | Dev-dep builds DEBUG but `broker_launch_assigns_child_to_job_object` only checks RELEASE paths; misleading Cargo.toml comment | Backlog — dev-loop UX (CI release.yml workaround masks the issue) |
| WR-08 | crates/nono-cli/tests/common/test_env.rs:5-10 | Mirror omits `lock_env()` + `EnvVarGuard::remove()`; doc-comment claims "verbatim" | Backlog — doc freshness + test ergonomics |

These items are tracked here for visibility and should surface in `/gsd-audit-milestone` for the v2.5 milestone or be folded into a future hardening phase. None are blocking for Phase 41 close: each is either a code-hygiene item or a defense-in-depth gap that does not contradict any REQ-CI-01/02/03 or REQ-BROKER-CR-01..04 success criterion.

## Human Verification Required

#### 1. windows-build CI lane no longer fails at PowerShell parameter binding after Plan 41-08 lands

**Test:** On the next push to the Phase 41 PR branch carrying commit `c0a89227` (the Plan 41-08 fix), inspect the GH Actions `windows-build` job's `Run Windows build harness` step output (`ci-logs/windows-build.log`).
**Expected:**
- NO line matching `Cannot process command because of one or more missing mandatory parameters: BrokerPath`.
- The new `==> build nono-shell-broker` label appears in the log followed by a successful `cargo build -p nono-shell-broker` step.
- The `==> validate windows msi contract` label is followed by NO Test-Path failure; the multi-line validator invocation succeeds at parameter binding (the validator may still fail downstream for content reasons — that is a SEPARATE bug, not this gap closure's concern).
**Why human:** The decisive live signal lives in GH Actions and is not reproducible locally — `cargo build --workspace --verbose` takes ~10 minutes on Windows, and full validator content checks require WiX tooling. Plan 41-08's PowerShell parser check + grep audit verified the codebase fix; only the CI lane verifies the end-to-end runtime behavior. REQ-CI-02 SC#2 fully closes once this lane is green.

#### 2. All 7 GH Actions CI lanes green on Phase 41 close SHA (post-Plan-41-08 head)

**Test:** Open / refresh the Phase 41 PR and inspect CI status for all 7 lanes (Linux Clippy, macOS Clippy, Windows Build, Windows Integration, Windows Regression, Windows Security, Windows Packaging) on the head SHA carrying `c0a89227` + `55640d72`.
**Expected:** All lanes PASS on the same head commit.
**Why human:** REQ-CI-01 SC#3 + REQ-CI-02 SC#1 require GH Actions green on Phase 41 close SHA; not reproducible locally. Carried forward from prior verification.

#### 3. env_vars parallel flake fix (Plan 41-05) on Windows host

**Test:** On a Windows host, run `cargo test -p nono-cli --test env_vars windows_run_redirects_profile_state_vars_into_writable_allowlist` 10 times back-to-back in parallel mode.
**Expected:** 0 failures across 10 runs.
**Why human:** Plan 41-05 did not execute the 10x verification on the current dev host (skipped due to availability). CI Integration job verifies on PR.

#### 4. Block-net probe tests on elevated Windows CI runner

**Test:** Verify `windows_run_block_net_blocks_probe_connection` + `windows_run_block_net_blocks_probe_through_cmd_host` pass on a Windows runner with `NONO_CI_HAS_WFP=true` and WFP service installed.
**Expected:** Both tests pass with "connect failed" or "exit code 42" markers in stderr.
**Why human:** Local dev host short-circuits the probe path; full WFP enforcement runs only on elevated CI runner.

#### 5. Cross-binding (nono-py / nono-ts) D-10 verification of CR-01 FFI remap

**Test:** `grep -rn 'ErrPathNotFound\|errorCode.*-1' ../nono-py/ ../nono-ts/` from a workspace with both sibling repos checked out.
**Expected:** No integer-mapping of `-1` (ErrPathNotFound) as broker-discovery-failure semantics. If any matches: file a follow-up todo for cross-binding lockstep.
**Why human:** Sibling repos not present in this working directory; D-10 manual verification deferred per Plan 41-06 SUMMARY.

## Gaps Summary

**No gaps remain at the codebase level.** Plan 41-08 (commits `c0a89227` + `55640d72`) closed the single BLOCKER from the prior verification. All 5 must-haves are now VERIFIED at the codebase level (was 4/5).

The phase still requires HUMAN VERIFICATION on 5 items — 4 carried forward from the prior verification (all 7 CI lanes green / env_vars 10x parallel flake / block-net probes on elevated runner / cross-binding D-10) plus 1 new item specifically for the Plan 41-08 gap closure (windows-build lane progresses past PowerShell parameter binding on the post-Plan-41-08 SHA). All 5 items live on CI lanes / Windows-host hardware / sibling repos that cannot be locally executed from this dev host.

Per the verification process Step 9 decision tree, when human-verification items are non-empty the status is `human_needed` regardless of an otherwise-clean 5/5 score. This is the correct closure status for Phase 41: code-level work complete, awaiting live CI / human signals to fully confirm REQ-CI-01 SC#3 (Linux/macOS Clippy green on GH Actions) + REQ-CI-02 SC#1 (5 Windows CI jobs green) + Plan 41-05 / 41-04 / 41-06 production-environment behaviors.

The 7 WARNINGS (WR-01..WR-08) are explicitly deferred per user "Blocker only" scope decision on the gap closure — see § Deferred (Backlog) for the disposition table. They surface for the v2.5 milestone audit but do not block Phase 41 close.

---

_Verified: 2026-05-16T19:30:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification supersedes: 2026-05-16T00:00:00Z (initial verification, status: gaps_found)_
