---
phase: 93
slug: live-zt-infra-integration-revocation-request-flow
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-22
---

# Phase 93 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from 93-RESEARCH.md § Validation Architecture. The planner populates
> the Per-Task Verification Map as plans are authored.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | pytest (nono-py live arm, apply console-script, AWS_* strip env-inspection) + `cargo test` (nono-cli `override audit-emit` subcommand, env_sanitization) + `verify-dark.ps1 --gate OVERRIDE-02` (live gate) |
| **Config file** | `nono-py/pyproject.toml` (pytest) · workspace `Cargo.toml` (cargo test) · `scripts/verify-dark.ps1` (gate harness) |
| **Quick run command** | `python -m pytest nono-py/tests/test_override_wiring.py -q` |
| **Full suite command** | `python -m pytest nono-py/tests -q && cargo test -p nono-cli && pwsh -File scripts/verify-dark.ps1 --gate OVERRIDE-02` |
| **Estimated runtime** | ~60–120 seconds (gate SKIPs fast when provisioner absent) |

---

## Sampling Rate

- **After every task commit:** Run the quick run command (Python live-arm + wiring tests)
- **After every plan wave:** Run the full suite command
- **Before `/gsd:verify-work`:** Full suite green; OVERRIDE-02 emits PASS or SKIP_HOST_UNAVAILABLE (never FAIL on a host without the provisioner)
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

*Populated by the planner during plan authoring. Each ZTL/CLI/DF requirement maps to at least one automated verify or a Wave 0 dependency. Live-AWS/provisioner paths are host-gated and verified via the OVERRIDE-02 gate's SKIP_HOST_UNAVAILABLE contract.*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | — | — | ZTL-01..05 / CLI-01/02 / DF-02 | — | (planner fills) | unit/integration/gate | (planner fills) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] nono-py live-arm test module (e.g. `nono-py/tests/test_live_arm.py`) — mockable `POST /actions` stub asserting allow→proceed, deny→LiveRevoked, timeout/non-200/malformed→LiveUnavailable (ZTL-01/02/03)
- [ ] AWS_* env-strip inspection test (ZTL-04, SC3) — child env contains no `AWS_*`, SystemRoot/windir baseline preserved
- [ ] `cargo test` coverage for the new `override audit-emit` subcommand (OQ-1 decision a) — kind→EventID mapping (rejected→10008, revoked→10010), HMAC chain advance
- [ ] `nono-override-apply` console-script smoke test (CLI-02, OQ-5 decision) — full offline+live verify-then-run one-shot
- [ ] `scripts/gates/override-02.ps1` — mirrors override-01 contract; mints via local provisioner + Phase 91 keypair; SKIP_HOST_UNAVAILABLE when absent

*Final Wave 0 list is the planner's; this captures the research-derived minimum.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end real-KMS token live `allow` against AWS control plane | ZTL-01/ZTL-05 | AWS KMS + DAAL unreachable from dev host (Dark Factory mandate; host-gated) | Run OVERRIDE-02 against a live ZT-Infra deployment with GPO-seeded HKLM trust roots; expect PASS, DAAL anchored async |
| Production HKLM trust-root read (GPO-seeded `Override\` key) | ZTL-01 (D-05/D-06) | Requires elevated host + GPO-seeded `HKLM\SOFTWARE\Policies\nono\Override` | Seed via ADMX/GPO, confirm fail-closed deny when trust material absent |
| Revocation honored on next live check | ZTL-03 | Requires running provisioner with a `deny[]` rule edit between checks | Add `override.apply:<jti>` deny rule; re-run apply; expect LiveRevoked |

*Live/provisioner paths are SKIP_HOST_UNAVAILABLE under the gate when the host lacks the dependency — consistent with prior milestone Dark Factory practice.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter (planner sets when map complete)

**Approval:** pending
