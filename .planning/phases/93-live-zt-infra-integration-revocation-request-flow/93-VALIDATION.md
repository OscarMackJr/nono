---
phase: 93
slug: live-zt-infra-integration-revocation-request-flow
status: planned
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-22
---

# Phase 93 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from 93-RESEARCH.md § Validation Architecture. The planner populated
> the Per-Task Verification Map from the authored plans (93-01..93-06).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | pytest (nono-py live arm, apply console-script) + `cargo test` (nono-cli `override audit-emit` + `env_sanitization` + `override_request`; nono-py `override::`) + `verify-dark.ps1 --gate override-02` (live gate) |
| **Config file** | `nono-py/pyproject.toml` (pytest) · workspace `Cargo.toml` (cargo test) · `scripts/verify-dark.ps1` (gate harness) |
| **Quick run command** | per-repo touched: `python -m pytest nono-py/tests/test_live_arm.py -x -q` OR the relevant `cargo test -p <crate> --lib <filter>` |
| **Full suite command** | `python -m pytest nono-py/tests -q && cargo test -p nono-cli && cargo test -p nono-py && pwsh -File scripts/verify-dark.ps1 --gate override-02` |
| **Estimated runtime** | ~60–120 seconds (gate SKIPs fast when provisioner absent) |

---

## Sampling Rate

- **After every task commit:** Run the quick run command for the touched repo (Python live-arm/apply tests or the `cargo test -p` filter)
- **After every plan wave:** Run the full suite command (both repos green)
- **Before `/gsd:verify-work`:** Full suite green; override-02 emits PASS or SKIP_HOST_UNAVAILABLE (never FAIL on a host without the provisioner)
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

*Every ZTL/CLI/DF requirement maps to at least one automated verify or a Wave 0 test created by the plan that owns it. Live-AWS/provisioner paths are host-gated and verified via override-02's SKIP_HOST_UNAVAILABLE contract.*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 93-01-T1 | 01 | 1 | VFY-01 (kind vocab) | T-93-01-03 | LiveRevoked/LiveUnavailable kinds map 1:1 to stable codes | unit (Rust) | `cargo test -p nono-py --lib override::` | ❌ W0 (test added in task) | ⬜ pending |
| 93-01-T2 | 01 | 1 | ZTL-01 (trust sourcing) | T-93-01-01/02/04 | HKLM trust read fail-secure; absent -> deny; non-Win stub fail-closed | build+clippy (Rust) | `cargo build -p nono-py && cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` | ❌ W0 | ⬜ pending |
| 93-01-T3 | 01 | 1 | VFY-01 (VFY-03a seam) | T-93-01-01 | unknown key_id -> Err(KeyNotAllowlisted); per-key_id cache | unit (Rust) | `cargo test -p nono-py --lib override::` | ❌ W0 | ⬜ pending |
| 93-02-T1 | 02 | 1 | ZTL-04 | T-93-02-01 | child env has no AWS_*; SystemRoot/windir baseline intact | unit (Rust) | `cargo test -p nono-cli --lib exec_strategy::env_sanitization` | ❌ W0 | ⬜ pending |
| 93-02-T2 | 02 | 1 | ZTL-02/ZTL-03/AUD-01 | T-93-02-02/03/04 | reject/revoke -> 10008/10010 HMAC emit; chain advance; fail-closed | unit (Rust) | `cargo test -p nono-cli --lib override_audit_emit` | ❌ W0 | ⬜ pending |
| 93-03-T1 | 03 | 2 | CLI-01 | T-93-03-01/03 | request bundle {scope,repo_context,reason,nonce}; fresh nonce | unit (Rust) | `cargo test -p nono-cli --lib override_request` | ❌ W0 | ⬜ pending |
| 93-03-T2 | 03 | 2 | CLI-01 | T-93-03-02 | override group = request + audit-emit (not apply); builds | build (Rust) | `cargo build -p nono-cli --bin nono` | n/a | ⬜ pending |
| 93-04-T1 | 04 | 2 | ZTL-01/02/03/05 | T-93-04-01/03/05 | fail-closed mapping; no flush_daal; env-proxy off; body shape | unit (pytest, mocked urllib) | `python -m pytest tests/test_live_arm.py -x` | ❌ W0 | ⬜ pending |
| 93-04-T2 | 04 | 2 | VFY-01 (clause b) | T-93-04-02 | live pre-step before Rust spawn; no-override path unchanged | unit+build | `python -m pytest tests/test_live_arm.py -x && cargo build -p nono-py` | ❌ W0 | ⬜ pending |
| 93-05-T1 | 05 | 3 | CLI-02/VFY-01 | T-93-05-01/02 | offline+live verify before run; failures block exec | unit (pytest, mocked) | `python -m pytest tests/test_override_apply.py -x` | ❌ W0 | ⬜ pending |
| 93-05-T2 | 05 | 3 | CLI-02 | T-93-05-04 | console-script registered; resolves to _cli_apply:main | config-parse (Python) | `python -c "import tomllib; ..."` | n/a | ⬜ pending |
| 93-06-T1 | 06 | 3 | DF-02/ZTL-03/ZTL-01 | T-93-06-01..05 | precondition SKIP; allow+revoke live proof; no exit/Persist-Verdict | gate (pwsh) | `pwsh -File scripts/verify-dark.ps1 --gate override-02` (SKIP_HOST_UNAVAILABLE off-host) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

> **Sampling continuity:** no 3 consecutive tasks lack an automated verify — every task above carries an `<automated>` command. Live/provisioner end-to-end is the host-gated exception (override-02 SKIP contract).

---

## Wave 0 Requirements

Each test file is created by the plan/task that owns the behavior (Wave 0 scaffolds are inline in the owning task, not a separate plan):

- [ ] `nono-py/tests/test_live_arm.py` (Plan 04 T1) — mockable `POST /actions`: allow→proceed, deny→LiveRevoked, timeout/non-200/malformed→LiveUnavailable, no-flush_daal, body-shape (ZTL-01/02/03/05)
- [ ] `nono-py/tests/test_override_apply.py` (Plan 05 T1) — apply path: full-pass-runs, offline-fail-blocks, live-fail-blocks, `--` argv split (CLI-02)
- [ ] `nono-py` `override::` unit tests (Plan 01 T1/T3) — kind string codes + unknown-key_id fail-closed
- [ ] `crates/nono-cli/.../env_sanitization.rs` test module (Plan 02 T1) — AWS_* dangerous cases + unrelated-var-allowed (ZTL-04, SC3)
- [ ] `crates/nono-cli/src/override_audit_emit.rs` test module (Plan 02 T2) — kind→EventID 10008/10010 + chain-advance-by-one
- [ ] `crates/nono-cli/src/override_request.rs` test module (Plan 03 T1) — bundle JSON shape + distinct nonces (CLI-01)
- [ ] `scripts/gates/override-02.ps1` (Plan 06 T1) — mirrors override-01; mints via local provisioner + Phase 91 keypair; SKIP_HOST_UNAVAILABLE when absent (DF-02)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end real-KMS token live `allow` against AWS control plane | ZTL-01/ZTL-05 | AWS KMS + DAAL unreachable from dev host (Dark Factory mandate; host-gated) | Run override-02 against a live ZT-Infra deployment with GPO-seeded HKLM trust roots; expect PASS, DAAL anchored async |
| Production HKLM trust-root read (GPO-seeded `Override\` key) | ZTL-01 (D-05/D-06) | Requires elevated host + GPO-seeded `HKLM\SOFTWARE\Policies\nono\Override` | Seed via ADMX/GPO, confirm fail-closed deny when trust material absent |
| Revocation honored on next live check | ZTL-03 | Requires running provisioner with a `deny[]` rule edit between checks | Add `override.apply:<jti>` deny rule; re-run apply; expect LiveRevoked |
| Windows-marked pytest (compiled extension) | ZTL-01..05 / CLI-02 | Tests requiring `confined_run`/compiled `nono_py` skip off-Windows by platform guard | On Win10/11 with `maturin develop`: `python -m pytest nono-py/tests -v` |

*Live/provisioner paths are SKIP_HOST_UNAVAILABLE under the gate when the host lacks the dependency — consistent with prior milestone Dark Factory practice.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter (map complete across plans 93-01..93-06)

**Approval:** planner-approved 2026-06-22 (pending execution)
