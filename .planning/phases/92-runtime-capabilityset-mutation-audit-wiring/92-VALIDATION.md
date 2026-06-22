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

> Filled per-plan during planning. Each row maps a task to its requirement, the secure behavior it must prove, and an automated command. Cross-target (Linux/macOS) clippy for cfg-gated code is PARTIAL→CI per CLAUDE.md when the cross-toolchain is absent on the Windows dev host.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 92-01-01 | 01 | 1 | AUD-03 | T-92-AUD | EventIDs 10006–10010 + `PolicyOverrideApplied` variant exist; match arms exhaustive | unit | `cargo test -p nono --lib audit::` | ❌ W0 | ⬜ pending |
| 92-02-01 | 02 | 1 | AUD-01, AUD-02, AUD-04 | T-92-FAILCLOSED | Override paths without committed audit event ⇒ abort before spawn; chain advances by exactly 1 | unit | `cargo test -p nono-cli --lib telemetry::override` | ❌ W0 | ⬜ pending |
| 92-03-01 | 03 | 2 | MUT-01..05, VFY-01 | T-92-ESCALATE | Verified grant appends exact `--allow` paths; `..`/non-absolute rejected; None ⇒ byte-identical args | unit | `cd ../nono-py && cargo test confined_run` | ❌ W0 | ⬜ pending |
| 92-04-01 | 04 | 3 | DF-01 | T-92-GATE | `verify-dark.ps1 --gate OVERRIDE-01` PASS incl. bad-sig/expired/out-of-scope/replay/`alg:none` | integration | `pwsh -File scripts/verify-dark.ps1 --gate OVERRIDE-01` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · Task IDs illustrative — planner finalizes against actual plan/wave layout.*

---

## Wave 0 Requirements

- [ ] `crates/nono/src/audit.rs` test module — assertions for `PolicyOverrideApplied` serialization + redaction (AUD-03)
- [ ] `crates/nono-cli/src/telemetry/` test module — `chain_sequence()`-based assertion that emit advances the HMAC chain by exactly 1 (AUD-01/SC4) and fail-closed abort path (AUD-04)
- [ ] nono-py `tests/` — regression test for byte-identical no-override args (MUT-05/SC2) + scope-sanitization rejection cases (MUT-04/SC3); reuse Phase 91 keypair fixtures
- [ ] `scripts/gates/override-01.ps1` — OVERRIDE-01 gate body following the `Test-Precondition`/`Invoke-Gate` contract (DF-01)

*Existing infrastructure (Rust test runner, pytest/cargo for nono-py, verify-dark.ps1 harness) covers all phase requirements — no new framework install needed.*

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
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
