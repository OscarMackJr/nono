---
phase: 51
plan: 04
subsystem: verification
tags: [windows, verification, clippy, cross-target, ci-suite, non-regression, repro-a, checkpoint]
dependency_graph:
  requires: ["51-01", "51-02", "51-03"]
  provides: ["Phase 51 verification record", "cross-target clippy disposition", "repro A non-regression disposition"]
  affects: []
tech_stack:
  added: []
  patterns: ["cross-target clippy PARTIAL deferral", "pre-spawn fail-secure policy gate confirms DLL-init regression absent"]
key_files:
  created:
    - .planning/phases/51-no-pty-low-il-broker-token-routing-write-deny-preservation/51-04-SUMMARY.md
  modified: []
decisions:
  - "Checkpoint A (cross-target clippy): approved-partial-both — host clippy clean; Linux + macOS fail only on missing C cross-compiler (not lints); deferred to live GH Actions CI per cross-target-verify-checklist.md"
  - "Checkpoint B (repro A): accepted as non-regression PASS — 0xC0000142 DLL-init regression NOT reproduced (run fails at pre-spawn policy gate); literal cmd /c echo hi failure is the documented Phase 27 cmd-needs-C:\\ launch-path limitation, not a Phase 51 regression; positive heavy-runtime spawn deferred to Phase 52 HUMAN-UAT"
  - "Executed inline by the orchestrator (continuation of the 51-03 subagent-Bash-permission fallback); both blocking human-verify checkpoints adjudicated by the user via AskUserQuestion"
metrics:
  duration: "~25 minutes (inline)"
  completed: "2026-05-26"
  tasks_completed: 3
  tasks_total: 3
  files_modified: 0
---

# Phase 51 Plan 04: Verification Sweep

**One-liner:** Windows host clippy + full CI suite green; cross-target clippy (Linux + macOS) PARTIAL on missing C cross-compiler (deferred to CI per checklist); repro A confirms the `0xC0000142` DLL-init regression is gone (run fails at the pre-spawn Phase 27 policy gate, not at DLL init). Both blocking checkpoints user-approved.

## Tasks Completed

| Task | Name | Type | Disposition |
|------|------|------|-------------|
| 1 | Windows host clippy + full CI suite | auto | PASS (host clippy clean; suites green modulo 4 pre-existing host failures) |
| 2 | Cross-target clippy (Linux + macOS) | checkpoint:human-verify (blocking) | approved-partial-both |
| 3 | Repro A live non-regression | checkpoint:human-verify (blocking) | approved (regression gone) |

## Verification Results

### Task 1 — Windows host CI

| Check | Command | Result |
|-------|---------|--------|
| Host workspace clippy | `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` | **PASS** (exit 0) |
| nono-cli suite | `cargo test -p nono-cli --target x86_64-pc-windows-msvc` | 1091 pass / 4 pre-existing host failures (see note) |
| broker suite | `cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc` | **PASS** (17/17) |
| pty_token_gate_tests | targeted | **PASS** (8/8) |
| broker_dispatch_tests | targeted | **PASS** (5/5) |
| write_deny_low_il_broker_no_pty_tests | targeted | **PASS** (1/1, exit_code==1 non-vacuous gate) |

**Pre-existing host failures (4) — NOT Phase 51 regressions:** `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` and 3× `protected_paths::tests::*`. Proven to fail identically at the base commit `808e4b33` (pre-Wave-1): `0 passed; 4 failed`. Root causes are Windows-host environment issues — `\\?\` extended-length canonicalization breaking path-component overlap detection, and real-`%APPDATA%` profile-dir test-isolation pollution (the CLAUDE.md-documented env-isolation class). Independent of this phase; left for the existing host-test-hygiene backlog.

### Task 2 — Cross-target clippy (REQ-WSRH-05) → approved-partial-both

| Target | Result |
|--------|--------|
| `x86_64-pc-windows-msvc` (host) | **PASS** (clean) |
| `x86_64-unknown-linux-gnu` | **PARTIAL** — `cargo clippy` aborts at `cc-rs: failed to find tool "x86_64-linux-gnu-gcc"` (C cross-compiler absent). Not a clippy lint. |
| `x86_64-apple-darwin` | **PARTIAL** — `cargo clippy` aborts at `cc-rs: failed to find tool "cc"` (C cross-compiler absent). Not a clippy lint. |

Both rustup targets ARE installed, but a C-dependent dependency (aws-lc-sys/ring via the sigstore stack) requires a target C cross-compiler that is not present on this Windows dev host. The failure is in dependency C compilation, never reaching clippy on the workspace crates. Per `.planning/templates/cross-target-verify-checklist.md`, the decisive cross-target signal is the live GH Actions Linux/macOS clippy lanes on the HEAD SHA. The only cross-platform file modified in Phase 51 is `crates/nono-cli/src/profile/mod.rs` (`windows_low_il_broker: bool`), which follows the same cross-platform-field pattern as existing fields that compile cleanly on all targets. **User-approved disposition: approved-partial-both.**

### Task 3 — Repro A live non-regression → approved (regression gone)

Command: `nono run --profile claude-code [--allow-cwd] -- cmd /c "echo hi"` → exits 1.

The failure is the **pre-spawn fail-secure policy gate**, NOT the `0xC0000142` STATUS_DLL_INIT_FAILED regression:
- Without `--allow-cwd`: `execution directory outside supported allowlist`.
- With `--allow-cwd`: `Windows filesystem policy does not cover the absolute path argument required for launch: C:\`.

This second refusal is a **documented Phase 27 discovery** explicitly recorded in `crates/nono-cli/tests/audit_attestation.rs:118-123`: launching `cmd` via the Windows supervisor requires `C:\` in the launch-path policy, which the default policy intentionally withholds (fail-secure). It reproduces regardless of the launched binary's location (confirmed by launching an in-CWD `cmd.exe` copy — same refusal), proving the requirement is intrinsic to `cmd.exe`, not the launch path.

**Key non-regression conclusions:**
1. The `0xC0000142` DLL-init regression is **absent** — every invocation fails at policy validation, before any process is spawned / DLL is loaded.
2. Phase 51 modified only token-arm routing and the broker no-PTY path; it did not touch launch-path or filesystem policy, so the Phase 27 `cmd`/`C:\` gate is pre-existing and unrelated.
3. The profile field addition did not break `claude-code` profile loading (capabilities compiled and displayed correctly).

**Positive end-to-end heavy-runtime spawn validation** (a real `claude.exe` exiting 0 through `BrokerLaunchNoPty`, write-deny enforced live) is the explicit charter of **Phase 52 HUMAN-UAT (REQ-WSRH-04 / REQ-WSRH-06)**. **User-approved disposition: accept (regression gone; positive spawn → Phase 52).**

## Requirement Dispositions

| REQ | Verification | Status |
|-----|--------------|--------|
| REQ-WSRH-01 | `BrokerLaunchNoPty` cascade routing (51-01) + spawn arm (51-03); `pty_token_gate_tests` (8) prove arm selection | VERIFIED |
| REQ-WSRH-02 | `WriteRestricted` arm preserved (51-01); cascade test asserts it still fires for session-SID-without-opt-in | VERIFIED |
| REQ-WSRH-03 | `write_deny_low_il_broker_no_pty_tests` real-spawn proof (51-03), exit_code==1 non-vacuous gate | VERIFIED |
| REQ-WSRH-05 | Host clippy clean; PTY path regression-proofed (asserts BrokerLaunch not BrokerLaunchNoPty); repro A confirms 0xC0000142 gone; cross-target clippy PARTIAL → live CI | VERIFIED (host) / PARTIAL (cross-target, deferred to CI) |

## Deviations from Plan

- **Inline execution:** continuation of the 51-03 fallback (gsd-executor subagents could not obtain Bash). Both blocking `human-verify` checkpoints were adjudicated by the user via AskUserQuestion rather than free-text resume signals; recorded dispositions are equivalent to `approved-partial-both` (Task 2) and `approved` (Task 3).
- **Repro A acceptance not literally met:** the plan's acceptance ("`cmd /c echo hi` exits 0") is unreachable on Windows by design (Phase 27 `cmd`/`C:\` policy gate). The meaningful intent — confirming the `0xC0000142` regression is gone — is satisfied; the literal exit-0 smoke was an over-specified acceptance. Positive spawn validation correctly belongs to Phase 52.

## Security Analysis (Threat Model Coverage)

| Threat ID | Status |
|-----------|--------|
| T-51D-01 | MITIGATED — cross-target clippy gate explicitly adjudicated via blocking checkpoint; approved-partial-both with CI deferral documented (Phase 41 anti-pattern not recurred) |
| T-51D-02 | MITIGATED — repro A run live; `0xC0000142` regression confirmed absent (pre-spawn gate); positive spawn deferred to Phase 52 per user approval |
| T-51D-03 | MITIGATED — no `#[allow(...)]` added to silence lints; the 51-03 rust-1.95 fixes were structural (`is_some_and`, doc separator) |
| T-51D-SC | ACCEPTED — no new dependencies (verification-only plan) |

## Known Stubs

None. This is a verification plan; no code stubs. The deferred items (cross-target clippy → CI; positive heavy-runtime spawn → Phase 52) are tracked dispositions, not stubs.

## Threat Flags

None.

## Self-Check: PASSED

- [x] Host workspace clippy clean
- [x] broker suite 17/17; nono-cli targeted suites (8+5+1) all pass
- [x] nono-cli full suite: only 4 pre-existing host failures (proven at base), no Phase 51 regressions
- [x] Cross-target clippy: approved-partial-both (C cross-compiler absent; deferred to CI per checklist)
- [x] Repro A: 0xC0000142 regression confirmed absent; user-approved non-regression disposition
- [x] Both blocking human-verify checkpoints adjudicated by the user
