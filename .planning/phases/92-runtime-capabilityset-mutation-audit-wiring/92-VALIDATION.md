---
phase: 92
slug: runtime-capabilityset-mutation-audit-wiring
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-21
---

# Phase 92 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) for core/CLI; pytest for nono-py; PowerShell (`verify-dark.ps1`) for the DF gate |
| **Config file** | none — workspace `Cargo.toml` + nono-py `pyproject.toml` already configured |
| **Quick run command** | `cargo test -p nono --lib audit:: && cargo test -p nono-cli --lib telemetry::` |
| **Full suite command** | `make test` (Rust workspace) + `cd ../nono-py && cargo test` + `pwsh -File scripts/verify-dark.ps1 --gate OVERRIDE-01` |
| **Estimated runtime** | ~90 seconds (Rust unit tests) + ~30s (gate) |

---

## Sampling Rate

- **After every task commit:** Run `{quick run command}`
- **After every plan wave:** Run `make test` (Rust) / `cargo test` (nono-py) for the touched repo
- **Before `/gsd:verify-work`:** Full suite must be green AND `verify-dark.ps1 --gate OVERRIDE-01` emits PASS
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

> Maps each task to its requirement, the secure behavior it must prove, and an automated command.
> Wave 0 test-first: each plan embeds inline TDD tasks (`tdd="true"`) so tests are written before
> production code. This satisfies the Nyquist Wave-0 requirement — there is no separate Wave 0 plan;
> Wave 0 tests are delivered as the first half of each TDD task within the plan. The `wave_0_complete`
> field flips to `true` after the first committed task in Wave 1 includes its RED test.
> Cross-target (Linux/macOS) clippy for cfg-gated code is PARTIAL→CI per CLAUDE.md when the
> cross-toolchain is absent on the Windows dev host.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 92-01-T1 | 92-01 | 1 | AUD-01, AUD-03 | T-92-AUD, T-92-02 | `PolicyOverrideApplied` variant exists with correct fields; serde snake_case; zt_audit_hash absent when None | unit (TDD inline) | `cargo test -p nono --lib audit::` | ❌ pre-exec | ⬜ pending |
| 92-01-T2 | 92-01 | 1 | AUD-01, AUD-03 | T-92-03 | EventIDs 10006–10010 + `SecurityEventType` variants exist; `event_id_for` and `severity_for` exhaustive | unit (TDD inline) | `cargo test -p nono-cli --lib telemetry::event` | ❌ pre-exec | ⬜ pending |
| 92-02-T1 | 92-02 | 1 | MUT-01, AUD-02 | T-92-TOCTOU | `OverrideGrant.zt_audit_hash()` getter exists; construction site populates from `token.current_hash` | unit (TDD inline) | `cd ../nono-py && cargo test --lib override::` | ❌ pre-exec | ⬜ pending |
| 92-02-T2 | 92-02 | 1 | MUT-01..05, AUD-04 | T-92-SCOPE, T-92-D02-PROBE | `append_override_args` appends correct flags; `sanitize_override_path` rejects `..`/relative; `probe_override_support` raises `NonoOverrideError` on version mismatch; MUT-05 regression: None→no --override-audit | unit (TDD inline) | `cd ../nono-py && cargo test --lib windows_confined_run::` | ❌ pre-exec | ⬜ pending |
| 92-03-T1 | 92-03 | 2 | AUD-01, AUD-02, AUD-04 | T-92-DENY_UNKNOWN | `OverrideAuditMeta` deserializes valid JSON; rejects unknown fields; allows null zt_audit_hash | unit (TDD inline) | `cargo test -p nono-cli --lib cli::` | ❌ pre-exec | ⬜ pending |
| 92-03-T2 | 92-03 | 2 | AUD-01, AUD-04 | T-92-FAILCLOSED, T-92-MUTEX | `emit_override_event` advances chain by 1; returns Err on poisoned mutex (AUD-04 fail-closed); two-call advances by 2 | unit (TDD inline) | `cargo test -p nono-cli --lib telemetry::` | ❌ pre-exec | ⬜ pending |
| 92-04-T1 | 92-04 | 3 | MUT-01..05, AUD-04 | T-92-VACUOUS-MOCK | 6 pytest tests green using stub injection (NOT subprocess.Popen mock); MUT-05 regression, AUD-04 flag assertion | integration (pytest) | `cd ../nono-py && python -m pytest tests/test_override_wiring.py -x -q` | ❌ pre-exec | ⬜ pending |
| 92-04-T2 | 92-04 | 3 | DF-01, MUT-01, MUT-05, VFY-01 | T-92-GATE-EXIT, T-92-SC2-INCOMPLETE | `verify-dark.ps1 --gate override-01` PASS; SC1/SC2/SC3 proven; no subprocess.Popen mock | integration (dark-factory) | `pwsh -File scripts/verify-dark.ps1 --gate OVERRIDE-01` | ❌ pre-exec | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*`nyquist_compliant` and `wave_0_complete` flip to `true` after Wave 1 first-task RED tests are committed (TDD inline — tests precede implementation within each TDD task). Set these manually in this frontmatter after confirming the first RED commit exists.*

---

## Wave 0 Requirements

Wave 0 tests are embedded as the RED phase of each TDD task (`tdd="true"`) within each plan. There is no
separate Wave 0 plan. The following files must gain new test coverage before (or as part of) the first
production code commit in each plan:

- [ ] `crates/nono/src/audit.rs` — 3 inline tests for `PolicyOverrideApplied` serialization + zt_audit_hash None-omission + round-trip (AUD-03); embedded in Plan 92-01 Task 1
- [ ] `crates/nono-cli/src/telemetry/event.rs` — 2 inline tests for EventID mapping + serde round-trip (AUD-03); embedded in Plan 92-01 Task 2
- [ ] `crates/nono-cli/src/telemetry/mod.rs` — 4 inline tests for `emit_override_event` chain-advance + mutex-poison + two-call + None-zt (AUD-01/AUD-04); embedded in Plan 92-03 Task 2
- [ ] `nono-py/src/override.rs` — 2 inline Rust tests for `zt_audit_hash` getter (Some/None); embedded in Plan 92-02 Task 1
- [ ] `nono-py/src/windows_confined_run.rs` — 4 inline Rust tests for sanitize_override_path + MUT-05 regression + probe_cache; embedded in Plan 92-02 Task 2
- [ ] `scripts/gates/override-01.ps1` — new gate file for DF-01 (Plan 92-04 Task 2)

*(No new test framework install needed — all frameworks already present)*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Cross-target (Linux/macOS) clippy on cfg-gated audit code | CLAUDE.md cross-target rule | Windows dev host lacks cross C toolchain | PARTIAL→CI: live CI runs `cargo clippy --target x86_64-unknown-linux-gnu`/`x86_64-apple-darwin` |
| End-to-end live `nono.exe run` with a real override metadata flag against AppContainer+WFP | SC1 (OS confinement still applies) | Needs elevated Win11 host + nono-wfp-service; daemon-path telemetry only | Run from real PowerShell console per [[windows_supervised_needs_real_console]]; verify expanded `--allow` set still kernel-confined |

*VFY-01 live `POST /actions` arm is Phase 93 `[BLOCKING-93]` — NOT a Phase 92 manual gap.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (satisfied by TDD-inline approach — see Wave 0 section above)
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter (set AFTER first RED commit in Wave 1)

**Approval:** pending
