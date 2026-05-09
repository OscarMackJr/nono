---
phase: 31
slug: broker-process-architecture-shell-01
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-09
---

# Phase 31 — Validation Strategy

> Reconstructed from completed phase artifacts (State B). Plans 31-01..31-06 already shipped; this document audits and pins the verification surface.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` |
| **Config file** | `Cargo.toml` (workspace root) |
| **Quick run command** | `cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc` |
| **Full suite command** | `cargo test --workspace --target x86_64-pc-windows-msvc` |
| **Estimated runtime** | ~60s (broker tests <5s; full workspace ~60s on cold cache) |

Notes:
- Windows-specific tests are gated `#[cfg(all(test, target_os = "windows"))]` and only compile on `--target x86_64-pc-windows-msvc`.
- Plan 31-04 (release.yml + WiX MSI) and Plan 31-05 (operator field-test) and Plan 31-06 (bookkeeping/cookbook) have no Cargo unit-test surface — verified via grep gates and operator attestation.

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <changed-crate> --target x86_64-pc-windows-msvc`
- **After every plan wave:** Run `cargo test --workspace --target x86_64-pc-windows-msvc`
- **Before `/gsd-verify-work`:** Full suite green + `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` clean
- **Max feedback latency:** ~60s

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 31-01-01 | 01 | 1 | D-06 lift | T-31-01, T-31-06 | Library-side `create_low_integrity_primary_token` produces token with `SECURITY_MANDATORY_LOW_RID` (0x1000) | unit | `cargo test -p nono --target x86_64-pc-windows-msvc create_low_integrity_primary_token_tests` | ✅ | ✅ green |
| 31-01-01 | 01 | 1 | D-06 lift | T-31-03 | `OwnedHandle::Drop` is null-safe and idempotent (single CloseHandle) | unit | `cargo test -p nono --target x86_64-pc-windows-msvc owned_handle_drop` | ✅ | ✅ green |
| 31-01-01 | 01 | 1 | D-15 fallback | — | Lifted token-arm preserved via re-export — `WindowsTokenArm::LowIlPrimary` cascade still passes | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc low_integrity_primary_token_tests` | ✅ | ✅ green |
| 31-01-02 | 01 | 1 | D-07 variant | T-31-04 | `NonoError::BrokerNotFound { path }` displays path payload via Debug formatting; rejects env-var override | unit | `cargo test -p nono broker_not_found_tests` | ✅ | ✅ green |
| 31-01-03 | 01 | 0 | D-09 #7 | T-31-05 | `Set-Content -ErrorAction Stop` invocation distinguishes OS-level deny from PowerShell parse error | static + field | `grep -c "Set-Content -Path '" scripts/test-windows-shell-write-deny.ps1` (>= 1) AND Plan 31-05 Acceptance #7 | ✅ | ✅ green |
| 31-02-01 | 02 | 2 | D-05 scaffold | — | Workspace member `nono-shell-broker` registered with windows-sys 0.59 + 5 features; cross-compile parity stub on non-Windows | build | `cargo build --workspace --target x86_64-pc-windows-msvc` AND `cargo build --workspace` | ✅ | ✅ green |
| 31-02-02 | 02 | 2 | D-08 argv parser | T-31-09, T-31-13 | Missing/unknown flags fail-fast with `NonoError::SandboxInit`; hex parsing accepts `0x`/`0X` prefixes; multi-arg accumulation preserves order | unit | `cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc parse_args_tests` | ✅ | ✅ green |
| 31-02-02 | 02 | 2 | D-08 quoting | T-31-20 | `build_command_line` quotes shell_path always; quotes args with whitespace; doubles embedded quotes; null-terminates UTF-16 | unit | `cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc build_command_line_tests` | ✅ | ✅ green |
| 31-02-02 | 02 | 2 | D-01/D-02/D-03 8-step sequence | T-31-07, T-31-08, T-31-10, T-31-11, T-31-12 | Broker mechanism byte-equivalent to validated PoC; HANDLE_LIST + EXTENDED_STARTUPINFO_PRESENT; no CREATE_NEW_CONSOLE, no PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE | static + field | `grep -v '^[[:space:]]*//' crates/nono-shell-broker/src/main.rs \| grep -c CREATE_NEW_CONSOLE` (== 0) AND Plan 31-05 Acceptance #1/#2/#3 | ✅ | ✅ green |
| 31-03-01 | 03 | 2 | D-15 cascade order | — | `select_windows_token_arm` returns `BrokerLaunch` for `has_pty=true && !is_detached`; precedence over `session_sid` and `caps_demand_low_il` arms | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc pty_token_gate_tests` | ✅ | ✅ green |
| 31-03-02 | 03 | 2 | D-07 sibling resolution | T-31-04, T-31-16 | `BrokerNotFound` variant constructible + Display includes path; broker resolved as sibling of `current_exe()` | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc broker_dispatch_tests::broker_not_found_error_variant_is_constructible_and_displays_path` | ✅ | ✅ green |
| 31-03-02 | 03 | 2 | D-04 Job Object containment | T-31-19 | `IsProcessInJob(broker_handle, job, &mut in_job)` returns `in_job != 0` after `AssignProcessToJobObject` BEFORE `ResumeThread` | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc broker_dispatch_tests::broker_launch_assigns_child_to_job_object` | ✅ | ✅ green |
| 31-03-02 | 03 | 2 | D-08 argv emitter | T-31-20 | `build_broker_command_line` quotes paths/args containing whitespace; null-terminates UTF-16; matches Plan 31-02 parser contract | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc broker_dispatch_tests::build_broker_command_line` | ✅ | ✅ green |
| 31-04-01 | 04 | 3 | D-13 release pipeline | T-31-22, T-31-25 | Broker.exe signed with same Authenticode key as nono.exe; bundled in zip + machine MSI + user MSI | manual | Manual workflow_dispatch on a release tag (deferred) | ✅ | manual-only |
| 31-04-02 | 04 | 3 | WiX MSI broker component | T-31-24 | `build-windows-msi.ps1 -BrokerPath` mandatory; `<Component Id="cmpNonoShellBrokerExe">` ships in both scopes | static | `grep -c "BrokerPath" scripts/build-windows-msi.ps1` (>= 4) AND PowerShell mandatory-param runtime check | ✅ | manual-only |
| 31-05-01 | 05 | 4 | D-14 field-test runbook | — | 31-FIELD-SMOKE.md operator runbook documents acceptance harnesses + decision matrix | static | `grep -cE "OUTCOME: SUCCESS\|OUTCOME: FAILURE" 31-FIELD-SMOKE.md` (>= 1) | ✅ | ✅ green |
| 31-05-02 | 05 | 4 | D-04 lift | T-31-19 | `broker_launch_assigns_child_to_job_object` no longer `#[ignore]`'d; runs against real broker artifact with SKIP gate | unit | (covered above by 31-03-02 D-04 row) | ✅ | ✅ green |
| 31-05-03 | 05 | 4 | Acceptance #1–#7 | T-31-01..T-31-13 | Operator reproduces all acceptance criteria on Windows test box | manual | Operator field-test on user's Windows test box | ✅ | manual-only |
| 31-06-01 | 06 | 5 | Cookbook security envelope | — | `docs/cli/development/windows-poc-handoff.mdx` describes broker → Low-IL child + NO_WRITE_UP + defense-in-depth; zero active "deferred to v3.0" refs | static | `grep -c "broker" docs/cli/development/windows-poc-handoff.mdx` (>= 3) AND `grep -c "deferred to v3.0" ...` (== 0) | ✅ | manual-only |
| 31-06-02 | 06 | 5 | Bookkeeping flip | — | PROJECT/STATE/ROADMAP rows reflect SHELL-01 ✔ validated v2.3 Phase 31 | static | `grep -c "validated v2.3 Phase 31" .planning/PROJECT.md` (>= 1) AND `grep -c "⚠ Phase 31 candidate" .planning/PROJECT.md` (== 0) | ✅ | manual-only |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · manual-only*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* No new test framework installs needed — Rust's built-in `#[test]` runner drives every automated check; PowerShell parser drives the harness-syntax check; grep drives the structural acceptance gates.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end broker dispatch with TUI rendering | D-01, D-05 (Phase 30 carry), D-14 | Requires real Windows test box, Authenticode-signed broker artifact, real ConPTY session, and `claude` TUI host. CI matrix expansion deferred to v2.4 (CONTEXT.md `<deferred>`). | Run `.\nono.exe shell --profile claude-code --allow-cwd` on a Windows 10/11 box; verify shell prompt appears with no `STATUS_DLL_INIT_FAILED` and `whoami /groups` reports `Low Mandatory Level S-1-16-4096`; launch `claude` and confirm alternate-screen + cursor + raw-mode input all functional. |
| Mandatory-label NO_WRITE_UP enforcement on Low-IL grandchild | D-06, Acceptance #3 | Mandatory Integrity Control (MIC) enforcement is a kernel-level boundary; cannot be unit-tested without spawning a real Low-IL child. | Run `pwsh -File scripts\test-windows-shell-write-deny.ps1`; assert exit 0 with `Acceptance #3 result: PASS` log line (inner shell exit 42 sentinel proves `Set-Content` raised `UnauthorizedAccessException` at OS level). |
| Authenticode chain-walker validates broker | D-13, T-31-25 | Requires release.yml run with signing secrets; verified post-tag, not at unit-test time. | Run `Get-AuthenticodeSignature target\x86_64-pc-windows-msvc\release\nono-shell-broker.exe` after release pipeline run; assert `Status -eq "Valid"`. Verified pre-upload by the workflow's "Verify Authenticode signatures (Windows)" step. |
| Broker ships in machine + user MSI under `INSTALLFOLDER` | D-07 deployment | Requires actual MSI install + filesystem inspection. | Install machine MSI to `Program Files\nono\`; confirm `nono-shell-broker.exe` exists alongside `nono.exe`. Repeat for user-scope MSI in `LocalAppData\Programs\nono\`. |
| `nono shell --profile claude-code` cookbook recommendation | D-16 (success path) | Documentation correctness; no automated structural test for prose accuracy. | Manual review of `docs/cli/development/windows-poc-handoff.mdx` Phase 31 security-envelope section against the actual binary chain behavior. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (or are explicitly classified manual-only with rationale)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (every plan has at least one automated test except 31-04 / 31-06 which are CI/docs by nature)
- [x] Wave 0 covers all MISSING references (all argv parser + command-line gaps closed via `parse_args_tests`, `build_command_line_tests`, and `broker_dispatch_tests::build_broker_command_line` extensions)
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-09

---

## Validation Audit 2026-05-09

State B reconstruction — VALIDATION.md generated retroactively from completed phase artifacts.

| Metric | Count |
|--------|-------|
| Gaps found | 3 |
| Resolved | 3 |
| Escalated | 0 |

Resolved gaps:
1. `broker::parse_args` argv parser — 8 unit tests added in `crates/nono-shell-broker/src/main.rs` `parse_args_tests` mod (commit `787179bb`).
2. `broker::build_command_line` quoting — 5 unit tests added in `crates/nono-shell-broker/src/main.rs` `build_command_line_tests` mod (commit `787179bb`).
3. `build_broker_command_line` argv emitter — 3 unit tests appended to `crates/nono-cli/src/exec_strategy_windows/launch.rs` `broker_dispatch_tests` mod (commit `0172a05e`).

All 16 new tests passed on the Windows target on 2026-05-09 (`cargo test --target x86_64-pc-windows-msvc`).

No implementation-level escalations. One adversarial finding logged in the auditor return: `quote_windows_arg` only quotes when the arg contains whitespace/tab/quote (not unconditional) — Gap 3 tests pin this only-quote-when-needed policy as a future-regression guard.
