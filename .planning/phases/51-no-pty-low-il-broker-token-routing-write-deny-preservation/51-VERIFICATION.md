---
phase: 51-no-pty-low-il-broker-token-routing-write-deny-preservation
verified: 2026-05-26T18:35:00Z
status: passed
reconciled: 2026-05-26T19:30:00Z
score: 10/10 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run nono run --profile claude-code -- cmd /c \"echo hi\" on a Windows 11 host and observe whether it exits 0"
    expected: "Child prints 'hi' and exits 0, confirming no regression to plain console-app paths"
    result: passed
    closed_by: "Phase 52 HUMAN-UAT repro A — live PASS on Windows 11 build 26200, operator-attested 2026-05-26 (see 52-HUMAN-UAT.md § Phase 51 SC-4 Closure Note). The dev-host blocker was a misattribution: the cmd/echo form fails only the cwd-coverage gate from an uncovered cwd, NOT the Phase 27 launch-path gate (which fires only for cmd shapes resolving C:\\, e.g. cmd /c cd). Run from the profile-covered cwd C:\\Users\\OMack\\.claude, cmd /c echo hi printed 'hi' and exited 0. Repro B (claude --version) also exited 0."
---

# Phase 51: No-PTY Low-IL broker + token routing + write-deny preservation — Verification Report

**Phase Goal:** The non-PTY `nono run` supervised path launches heavy-runtime children (e.g. `claude.exe`) through a Low-IL primary token with no synthetic restricting SID, eliminating the `STATUS_DLL_INIT_FAILED (0xC0000142)` failure class while preserving mandatory-label `NO_WRITE_UP` write-deny at the OS level.
**Verified:** 2026-05-26T18:35:00Z
**Status:** passed (SC-4 reconciled 2026-05-26T19:30:00Z via Phase 52 HUMAN-UAT repro A)
**Re-verification:** No — initial verification + SC-4 reconciliation

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | `nono-shell-broker` accepts `--no-pty` invocation mode and binds anonymous-pipe stdio via `STARTF_USESTDHANDLES` | VERIFIED | `BrokerArgs.no_pty: bool` field exists in `main.rs:67`; `--no-pty` arm in `parse_args` at `main.rs:126-131`; `STARTF_USESTDHANDLES` branch in `run()` at `main.rs:299-315`. `parse_args_no_pty_flag_accepted` + `parse_args_no_pty_absent_defaults_false` unit tests pass (17/17 broker tests green). |
| 2 | `select_windows_token_arm` dispatches non-detached, non-PTY, session-SID launches to `BrokerLaunchNoPty` when `prefers_low_il_broker=true`; `WriteRestricted` arm remains reachable when `prefers_low_il_broker=false` | VERIFIED | Cascade in `launch.rs:1139-1175`: arm 3 `prefers_low_il_broker && has_session_sid → BrokerLaunchNoPty`; arm 4 `has_session_sid → WriteRestricted` unchanged. Tests: `pty_none_session_sid_with_broker_opt_in_selects_broker_launch_no_pty` + `pty_none_with_session_sid_selects_write_restricted` both pass (8/8 `pty_token_gate_tests`). |
| 3 | A regression test asserts that a Low-IL child write to a Medium-IL path is denied by OS kernel MIC pre-DACL; test passes with `exit_code != 0 && exit_code != 2` AND fixture unchanged | VERIFIED | `write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file` in `launch.rs:3225-3431`. Test observed `BROKER_EXIT_CODE=1`, fixture content `b"sentinel"` unchanged. Runs live (1/1 pass). `#[ignore]` attribute absent — confirmed in code. |
| 4 | `nono run --profile claude-code -- cmd /c "echo hi"` still exits 0, confirming no regression to plain console-app paths | VERIFIED | Closed by Phase 52 HUMAN-UAT repro A (operator-attested live PASS, Windows 11 build 26200, 2026-05-26): run from the profile-covered cwd `C:\Users\OMack\.claude`, the command printed `hi` and exited 0. The earlier "UNCERTAIN" disposition misattributed the failure to the Phase 27 launch-path gate; the actual obstacle was the cwd-coverage gate (fires only from an uncovered cwd). `cmd /c echo hi` does not resolve `C:\` and so does not trip the Phase 27 gate. Repro B (`claude --version`) also exited 0. See `52-HUMAN-UAT.md` § "Phase 51 SC-4 Closure Note". |
| 5 | Cross-target clippy is clean or PARTIAL per checklist; Windows CI lanes remain green; existing broker/detached paths produce no new failures | VERIFIED (host) / PARTIAL (cross-target) | Host `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` exits 0. Cross-target (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`) aborts at C cross-compiler absence (`cc-rs` failure, not a lint), consistent with known dev-host constraint documented in `.planning/templates/cross-target-verify-checklist.md`. User-approved disposition `approved-partial-both`. Existing `broker_dispatch_tests` (5/5), `pty_token_gate_tests` (8/8), `nono-shell-broker` (17/17) all pass — no regression to Phase 31 paths. |

**Score:** 10/10 must-haves verified (SC-4 reconciled 2026-05-26 via Phase 52 HUMAN-UAT repro A — live PASS)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | `BrokerLaunchNoPty` variant + cascade + spawn arm + write-deny test | VERIFIED | Variant at line 1113; cascade branch at 1152-1159; spawn arm at 1631-1851; write-deny test module at 3197-3432. All 4+ occurrences of `BrokerLaunchNoPty` confirmed. |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | `ExecConfig.prefers_low_il_broker: bool` field | VERIFIED | Field at line 172 with doc comment. |
| `crates/nono-cli/src/execution_runtime.rs` | `prefers_low_il_broker` wired from `loaded_profile.windows_low_il_broker` | VERIFIED | `prefers_low_il_broker: loaded_profile.as_ref().is_some_and(|p| p.windows_low_il_broker)` at line ~389. |
| `crates/nono-cli/src/profile/mod.rs` | `windows_low_il_broker: bool` in Profile, ProfileDeserialize, From impl, merge_profiles (4 locations) | VERIFIED | Profile struct at 2081; ProfileDeserialize at 2143; From impl at 2176; merge_profiles OR semantics at 3048. |
| `crates/nono-cli/data/policy.json` | `"windows_low_il_broker": true` in claude-code profile only | VERIFIED | Single occurrence at line 729 in the `claude-code` profile block. No other profile has the field. |
| `crates/nono-cli/data/nono-profile.schema.json` | `windows_low_il_broker` property entry | VERIFIED | Property at line 104-107 with `"type": "boolean"` and description. |
| `crates/nono-shell-broker/src/main.rs` | `no_pty` field + `--no-pty` parse arm + `STARTF_USESTDHANDLES` branch + WR-01 fail-closed guard + 2 new unit tests | VERIFIED | `no_pty` field at line 67; `--no-pty` match arm at 126-131; WR-01 fail-closed guard at 299-305; `STARTF_USESTDHANDLES` binding at 306-315. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `policy.json` claude-code profile | `profile/mod.rs ProfileDeserialize` | serde deserialization at startup | VERIFIED | `"windows_low_il_broker": true` in policy.json; field exists in `ProfileDeserialize` with `#[serde(deny_unknown_fields)]` constraint — struct literal completeness enforced by Rust compiler. |
| `profile/mod.rs Profile` | `exec_strategy_windows/mod.rs ExecConfig` | `execution_runtime.rs` Windows ExecConfig literal | VERIFIED | `prefers_low_il_broker: loaded_profile.as_ref().is_some_and(|p| p.windows_low_il_broker)` wires `Profile.windows_low_il_broker` → `ExecConfig.prefers_low_il_broker`. |
| `ExecConfig.prefers_low_il_broker` | `select_windows_token_arm` | `spawn_windows_child` call site at `launch.rs:1218-1224` | VERIFIED | `config.prefers_low_il_broker` passed as 5th argument at line 1223. |
| `BrokerLaunchNoPty arm` | `DetachedStdioPipes::create()` | anonymous-pipe stdio creation in spawn arm | VERIFIED | `let pipes = DetachedStdioPipes::create()?` at `launch.rs:1677`; stderr merged into stdout (`child_stdio: [HANDLE; 3] = [pipes.stdin_read, pipes.stdout_write, pipes.stdout_write]`) at 1688-1689 per CR-01 fix. |
| `spawn_windows_child return value` | `execute_supervised` detached_stdio relay | `detached_stdio = Some(pipes)` at `launch.rs:1850` | VERIFIED | `detached_stdio = Some(pipes)` confirmed at line 1850; PATTERNS.md + 51-03 SUMMARY confirm `attach_detached_stdio` is unconditional in `execute_supervised`. |
| `nono-cli launch.rs BrokerLaunchNoPty arm` | `nono-shell-broker --no-pty flag` | broker command line constructed with `--no-pty` | VERIFIED | `broker_args.push(OsString::from("--no-pty"))` at `launch.rs:1789`. |
| `args.inherit_handles[0..2]` | `startup_info_ex.StartupInfo.hStdInput/Output/Error` | `STARTF_USESTDHANDLES` when `args.no_pty` is true | VERIFIED | `main.rs:306-315` sets `dwFlags = STARTF_USESTDHANDLES` and binds hStd* to handles 0/1/2. WR-01 fail-closed guard at lines 299-305 (reject `< 3` handles with `Err`). |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces Windows system-call sequences and token routing logic, not web components rendering dynamic data. The key data flows (profile field → token arm selection → process spawn) are verified through unit tests and the integration write-deny test.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `pty_token_gate_tests` — 8 tests including BrokerLaunchNoPty selection | `cargo test -p nono-cli --bin nono --target x86_64-pc-windows-msvc pty_token_gate_tests` | `8 passed; 0 failed` | PASS |
| `broker_dispatch_tests` — 5 Phase 31 no-regression tests | `cargo test -p nono-cli --bin nono --target x86_64-pc-windows-msvc broker_dispatch_tests` | `5 passed; 0 failed` | PASS |
| `write_deny_low_il_broker_no_pty_tests` — real-spawn MIC write-deny proof | `cargo test -p nono-cli --bin nono --target x86_64-pc-windows-msvc write_deny_low_il_broker_no_pty_tests` | `1 passed; 0 failed` (exit_code=1, fixture=b"sentinel") | PASS |
| `nono-shell-broker` full suite | `cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc` | `17 passed; 0 failed` | PASS |
| Host workspace clippy | `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` | exit 0, no warnings | PASS |

### Probe Execution

No phase-conventional `scripts/*/tests/probe-*.sh` probes defined. The behavioral spot-checks above serve as the phase verification probes.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| REQ-WSRH-01 | 51-02, 51-03 | nono-shell-broker can launch a child without ConPTY/PTY, inheriting stdio via anonymous pipes | SATISFIED | Broker accepts `--no-pty`; `STARTF_USESTDHANDLES` binds 3 pipe handles as child stdio; `BrokerLaunchNoPty` spawn arm in `spawn_windows_child` creates and passes anonymous pipes. 17/17 broker tests pass. |
| REQ-WSRH-02 | 51-01 | Non-PTY supervised path routes through broker Low-IL arm instead of `WriteRestricted` for affected case; `WriteRestricted` branch still reachable | SATISFIED | `select_windows_token_arm` cascade arm 3 (BrokerLaunchNoPty) inserted BEFORE arm 4 (WriteRestricted); WriteRestricted remains reachable when `prefers_low_il_broker=false`. `pty_none_with_session_sid_selects_write_restricted` test pins this. |
| REQ-WSRH-03 | 51-03 | Regression test asserts write attempt by Low-IL child to Medium-IL path is denied by kernel MIC | SATISFIED | `write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file` passes with `exit_code==1`, fixture unchanged. cfg-gated `#[cfg(all(test, target_os = "windows"))]`. No `#[ignore]`. |
| REQ-WSRH-05 | 51-01, 51-04 | No regression to existing paths; Windows CI green; cross-target clippy clean or PARTIAL per checklist | SATISFIED (host) / PARTIAL (cross-target) | Host clippy clean; broker/pty/dispatch tests all pass; cross-target clippy PARTIAL — C cross-compiler absent on dev host, deferred to live GH Actions CI per `.planning/templates/cross-target-verify-checklist.md`. User-approved `approved-partial-both`. |

**Note:** REQ-WSRH-04 and REQ-WSRH-06 are Phase 52 scope (not Phase 51). REQUIREMENTS.md traceability table maps them to Phase 52.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/nono-cli/src/profile/mod.rs` | ~4668 | `fn oauth2_cred_builder()` triggers `dead_code` warning in `cargo test` profile build | Info | Pre-existing warning unrelated to Phase 51; present before this phase. Does NOT appear in `cargo clippy` (clean), only in `--warn(dead_code)` of the test binary build. Not introduced by Phase 51. |

No `TBD`, `FIXME`, or `XXX` debt markers found in Phase 51 modified files. No stub return patterns in production code. All `unsafe` blocks have `// SAFETY:` comments.

### CR-01 Deadlock Fix Verification

The code review found a BLOCKER (CR-01: undrained stderr pipe deadlock). The fix is present in the codebase:

- `launch.rs:1688-1689`: `child_stdio: [HANDLE; 3] = [pipes.stdin_read, pipes.stdout_write, pipes.stdout_write]` — stderr merged into stdout (NOT `pipes.stderr_write`).
- `launch.rs:1692`: `gated_handles: [HANDLE; 2] = [pipes.stdin_read, pipes.stdout_write]` — only 2 unique handles gated via HANDLE_LIST.
- This mirrors the Phase 17 detached path merge at `launch.rs:1879`: `startup_info.hStdError = pipes.stdout_write`.

CR-01 is FIXED and confirmed in code. WR-01 (fail-closed < 3 handles) is FIXED at `main.rs:299-305`. WR-04 (exit code assertion) is FIXED at `launch.rs:3419-3425` (`exit_code != 0 && exit_code != 2`).

### Human Verification — CLOSED

#### 1. Repro A: cmd /c "echo hi" exits 0 (ROADMAP SC-4) — PASSED

**Test:** `nono run --profile claude-code -- cmd /c "echo hi"`

**Expected:** Child prints `hi` and exits 0

**Result:** PASS — operator-attested live on Windows 11 build 26200 (2026-05-26), Phase 51 `nono 0.57.0` BrokerLaunchNoPty binary. Run from the profile-covered cwd `C:\Users\OMack\.claude`, the command printed `hi` and exited 0; no `0xC0000142` / STATUS_DLL_INIT_FAILED. Recorded in `52-HUMAN-UAT.md` repro A + § "Phase 51 SC-4 Closure Note".

**Correction to the original "why human" rationale:** the earlier disposition wrongly attributed the dev-host failure to the Phase 27 launch-path gate. The actual obstacle was the **cwd-coverage gate** (`crates/nono/src/sandbox/windows.rs:1304-1309`), which fires only when the working directory is outside the profile allowlist. `cmd /c echo hi` does NOT resolve `C:\` and therefore never trips the Phase 27 launch-path gate; from a profile-covered cwd it passes cleanly. Repro B (`claude --version`, 234 MB self-contained `claude.exe`) also exited 0 — stronger confirmation of the same invariant.

### Gaps Summary

No open gaps. All must-haves verified:
1. The `BrokerLaunchNoPty` variant, cascade, spawn arm, and profile field threading are fully implemented and wired.
2. The write-deny integration test proves kernel MIC `NO_WRITE_UP` enforcement with a real subprocess spawn.
3. The CR-01 stderr-deadlock blocker found in code review was fixed before phase completion; the fix is present in the production code.
4. SC-4 (repro A literal exit-0) was reconciled 2026-05-26 via Phase 52 HUMAN-UAT — live PASS. Status advanced to `passed`.

---

_Verified: 2026-05-26T18:35:00Z_
_Verifier: Claude (gsd-verifier)_
