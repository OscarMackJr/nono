---
phase: 103
slug: azure-clean-host-vm-iac-new-verify-dark-gates
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-03
---

# Phase 103 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None (no pytest/jest equivalent) — PowerShell gate scripts + `az`/`bicep` CLI checks are the test surface |
| **Config file** | none |
| **Quick run command** | `pwsh -File scripts/verify-dark.ps1 -Gate trusted-signed-assertion` / `-Gate broker-spawn-on-clean-host` (single gate); `az bicep build -f scripts/azure/clean-vm/main.bicep --stdout` (Bicep lint) |
| **Full suite command** | `pwsh -File scripts/verify-dark.ps1 -All` (exercises every pre-existing gate too — all must remain unaffected, zero harness changes) |
| **Estimated runtime** | single gate <30s; `-All` sweep ~1-2 min; `az bicep build` fast (local, after one-time Bicep binary install) |

---

## Sampling Rate

- **After every task commit:** `pwsh -File scripts/verify-dark.ps1 -Gate <new-gate-name>` (single gate) + `az bicep build -f scripts/azure/clean-vm/main.bicep --stdout` (when the bicep exists)
- **After every plan wave:** `pwsh -File scripts/verify-dark.ps1 -All` (full sweep — confirms no pre-existing gate regressed and both new gates SKIP correctly)
- **Before phase gate:** `az bicep build` exit 0 + `-All` sweep `overall` is `PASS` or `PASS_WITH_SKIPS` (never `FAIL`/`HARNESS_ERROR`) + `git diff --stat scripts/verify-dark.ps1` empty
- **Max feedback latency:** <30s per single-gate check

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 103-SC1a | TBD | — | CHOST-01 | NSG-scope / TLS | `main.bicep` compiles; Gen2+Trusted-Launch (secureBoot+vTPM) + live-SKU param shape; NSG scoped to single operator IP (no 0.0.0.0/0) | static/lint | `az bicep build -f scripts/azure/clean-vm/main.bicep --stdout` (exit 0) | ❌ W0 | ⬜ pending |
| 103-SC1b | TBD | — | CHOST-01 | ephemeral-lifecycle | deploy/teardown scripts exist + document ephemeral create→use→teardown (never persistent); no live deploy this phase | code-review | n/a (script/doc review) | ❌ W0 | ⬜ pending |
| 103-SC2 | TBD | — | CHOST-02 | false-PASS | `trusted-signed-assertion.ps1` exists, dot-sources verify-authenticode.ps1, asserts Status='Valid' (issuer INFORMATIONAL only — never a substring gate, per Phase 101 evidence), plugs in with zero harness changes | integration | `pwsh -File scripts/verify-dark.ps1 -Gate trusted-signed-assertion` → SKIP_HOST_UNAVAILABLE (exit 3) on dev host | ❌ W0 | ⬜ pending |
| 103-SC3 | TBD | — | CHOST-02 | order-dependence | `broker-spawn-on-clean-host.ps1` exists + SELF-CONTAINED (install→run→uninstall); order-safe under -All (sorts before clean-host-install) | integration + code-review | `pwsh -File scripts/verify-dark.ps1 -Gate broker-spawn-on-clean-host` → SKIP_HOST_UNAVAILABLE (exit 3); code-review confirms no cross-gate dependency | ❌ W0 | ⬜ pending |
| 103-SC4 | TBD | — | CHOST-02 | false-PASS | both new gates SKIP on dev host under a full sweep | integration | `pwsh -File scripts/verify-dark.ps1 -All` → both gates in gates[] with verdict SKIP_HOST_UNAVAILABLE; overall PASS or PASS_WITH_SKIPS | ❌ W0 | ⬜ pending |
| 103-REG | TBD | — | (regression) | harness-integrity | `verify-dark.ps1` byte-for-byte unchanged (zero harness changes) | static | `git diff --stat scripts/verify-dark.ps1` → empty | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*
*Task IDs are placeholders until the planner assigns plan/wave numbers.*

---

## Wave 0 Requirements

- [ ] `scripts/azure/clean-vm/main.bicep` — does not exist yet (CHOST-01)
- [ ] `scripts/azure/clean-vm/deploy.ps1` / `teardown.ps1` — do not exist yet (CHOST-01)
- [ ] `scripts/gates/trusted-signed-assertion.ps1` — does not exist yet (CHOST-02 SC2)
- [ ] `scripts/gates/broker-spawn-on-clean-host.ps1` — does not exist yet (CHOST-02 SC3)
- [ ] Bicep CLI install on the dev host — currently absent; the manual `curl` download + `~/.azure/bin/bicep.exe` placement (Pitfall 1) must run ONCE before `az bicep build` can be used as a verify step. Environment setup, not code — call out explicitly in the plan's first task so it isn't silently assumed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Actual live VM deploy + on-VM gate run | CHOST-03 (Phase 106) | Costs money + outward-facing Azure + needs a real release; explicitly OUT OF SCOPE for Phase 103 (author + skip-safe only) | Deferred to Phase 106 UAT |

*All in-phase behaviors have automated dev-host verification (bicep lint + gate SKIP). The live deploy is a Phase 106 concern, not gated here.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify (bicep lint / gate SKIP / git diff) — Wave 0 files created by the plans themselves
- [x] Sampling continuity: single-gate + bicep build after each task
- [x] Wave 0 covers all MISSING references (the 4 new files + Bicep binary setup)
- [x] No watch-mode flags
- [x] Feedback latency < 30s (single-gate)
- [ ] `nyquist_compliant: true` set once plans assign task IDs

**Approval:** pending
