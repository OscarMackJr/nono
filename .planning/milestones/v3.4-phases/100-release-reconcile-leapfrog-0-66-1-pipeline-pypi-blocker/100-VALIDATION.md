---
phase: 100
slug: release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-07-01
---

# Phase 100 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> **Release/config phase:** no new Rust logic is introduced, so validation is
> gate re-runs + build checks, not new unit tests. `cargo test` is a
> no-op-relative-to-baseline regression check, not a targeted new-test requirement.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Fork's PowerShell "dark-factory" harness (`scripts/verify-dark.ps1`), auto-discovering gate scripts under `scripts/gates/*.ps1` — not Pester, not a unit-test framework. Plus `make ci` (clippy + fmt + tests) for the Rust workspace. |
| **Config file** | None — gates are plain PowerShell functions auto-discovered by filename (`scripts/verify-dark.ps1:130-146`). |
| **Quick run command** | `pwsh -File scripts/gates/release-readiness.ps1` (fast local check after each version-string edit) |
| **Full suite command** | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (canonical; writes JSON verdict to `.nono-runtime/verdicts/release-readiness.json`) + `pwsh -File scripts/release-dry-run.ps1` + `make ci` |
| **Estimated runtime** | ~gate seconds; dry-run + make ci minutes |

> ⚠️ Per project memory: invoke verify-dark/gate scripts via `-File`/direct, **NEVER** `pwsh -Command "<bare path>"` (swallows exit N→1).

---

## Sampling Rate

- **After every task commit:** Run `pwsh -File scripts/gates/release-readiness.ps1`
- **After every plan wave:** Run `pwsh -File scripts/release-dry-run.ps1` + `make ci`
- **Before `/gsd:verify-work`:** Both `release-dry-run.ps1` and the `release-readiness` gate must be GREEN at `0.66.1` (RLS-13 / D-09)
- **Max feedback latency:** gate ~seconds

---

## Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists |
|--------|----------|-----------|-------------------|-------------|
| RLS-10 | All 6 version-family crates report `0.66.1`; workspace builds clean | gate + build | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (version assertion) + `cargo build --workspace` | ✅ gate exists; needs 2-constant edit |
| RLS-11 | Reconciled CI hunks don't break the prepare-only pipeline / signed-MSI order / verify-dark gate | manual YAML review + optional `actionlint` lint | `actionlint .github/workflows/release.yml` (if available); otherwise reviewed-only | ❌ CI-config — inherently smoke-verified via an actual push, not a local unit test |
| RLS-12 | `nono-py` wheel builds; `twine check` (or documented SKIP) passes | integration (external repo build) | `cd C:/Users/OMack/nono-py && maturin build` then `pwsh -File scripts/release-dry-run.ps1` (shells into the binding build) | ✅ dry-run PyPI leg exists |
| RLS-13 | Both `release-dry-run.ps1` and `release-readiness` gate exit GREEN at `0.66.1`; runbook updated | gate + script | `pwsh -File scripts/release-dry-run.ps1` (exit 0) + `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (PASS) | ✅ both scripts exist |

---

## Per-Task Verification Map

*Populated from the 5 committed plans. Known gotcha: `cargo publish --dry-run` for downstream crates exits 101 until `nono` is published (v3.3 97-03) — expected, not a failure.*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 100-01-01 | 01 | 1 | RLS-10 | version-collision | No `0.66.0` version string remains in the 6 members | source | `grep -rn 'version = "0.66.0"' crates/*/Cargo.toml bindings/c/Cargo.toml \| wc -l` (→ 0) | ✅ | ⬜ pending |
| 100-01-02 | 01 | 1 | RLS-10 | — | Workspace builds clean at 0.66.1 | build | `cargo build --workspace --all-targets` (exit 0) | ✅ | ⬜ pending |
| 100-02-01 | 02 | 1 | RLS-11 | unsigned/wrong-order MSI | publish-crates idempotency adapted; sign-order preserved | source | `grep -c 'cargo search' .github/workflows/release.yml` | ✅ | ⬜ pending |
| 100-02-02 | 02 | 1 | RLS-11 | — | cross-compile job adapted (not dead upstream trigger) | source | `grep -n 'cross-compile\|workflow_dispatch' .github/workflows/ci.yml` | ✅ | ⬜ pending |
| 100-02-03 | 02 | 1 | RLS-11 | — | ADR-100 records adopt-vs-adapt disposition | source | `grep -c '^## ' proj/ADR-100-ci-pipeline-reconcile.md` | ✅ | ⬜ pending |
| 100-03-01 | 03 | 1 | RLS-10, RLS-12 | intermediate-0.66.0 leak | nono-py at 0.66.1 + `endpoint_policy: None`; wheel builds | integration | `cd C:/Users/OMack/nono-py && maturin build` | ✅ | ⬜ pending |
| 100-03-02 | 03 | 1 | RLS-10 | intermediate-0.66.0 leak | nono-ts at 0.66.1; package validates | integration | `cd C:/Users/OMack/nono-ts && npm publish --dry-run` | ✅ | ⬜ pending |
| 100-04-01 | 04 | 2 | RLS-13 | private-path leak | release-readiness gate PASS at 0.66.1 | gate | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` | ✅ | ⬜ pending |
| 100-04-02 | 04 | 2 | RLS-13 | — | dry-run orchestrator exits 0 at 0.66.1 | script | `pwsh -File scripts/release-dry-run.ps1` | ✅ | ⬜ pending |
| 100-04-03 | 04 | 2 | RLS-13 | — | runbook updated for 0.66.1 (no stale 0.66.0) | source | `grep -c '0.66.0' .../RELEASE-RUNBOOK.md` | ✅ | ⬜ pending |
| 100-05-01 | 05 | 3 | RLS-13 | — | clean-host gate returns SKIP_HOST_UNAVAILABLE (host-gated) | gate (manual) | `pwsh -File scripts/gates/clean-host-install.ps1` | ✅ | ⬜ pending (host-gated) |
| 100-05-02 | 05 | 3 | RLS-13 | — | both folded todos record Phase 100 status | source | `grep -c 'Phase 100' .planning/todos/pending/20260611-*.md` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

None — the release-readiness gate script and the dry-run orchestrator already exist and passed
once at `0.66.0` (v3.3 Phase 97). No new test infrastructure is needed; this phase edits two
hard-coded constants in an existing gate (`scripts/gates/release-readiness.ps1:76-77`) and
re-runs existing scripts.

*Existing infrastructure covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Reconciled `release.yml` CI actually runs correctly on a release PR | RLS-11 | GH Actions YAML has no local unit-test story; only an actual push exercises it | Operator-gated: on the eventual release push, confirm the `publish-crates` + compile-step jobs behave; out of this phase's local scope |
| Clean-host MSI install (folded todos) | RLS-13 (folded) | Needs a clean Win11 VM; poc-cert one additionally blocked on Azure Trusted Signing verify-gate | Host-gated — SKIP_HOST_UNAVAILABLE if no clean VM; document status |

---

## Validation Sign-Off

- [x] All tasks have an automated gate/build verify or a documented manual/host-gated reason
- [x] Sampling continuity: gate re-run after each version-string edit (no 3-consecutive-task gap)
- [x] Wave 0 covers all MISSING references (none — existing infra)
- [x] No watch-mode flags
- [x] Both release scripts GREEN at `0.66.1` before verify-work (per-task map rows 100-04-01/02)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-07-01 (plan-checker VERIFICATION PASSED; per-task map synced from committed plans)
