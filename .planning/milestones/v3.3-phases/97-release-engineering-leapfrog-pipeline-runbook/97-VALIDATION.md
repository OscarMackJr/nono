---
phase: 97
slug: release-engineering-leapfrog-pipeline-runbook
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-26
---

# Phase 97 — Validation Strategy

> Per-phase validation contract. This is a release-engineering phase: its deliverables are
> version manifests, the release.yml pipeline, dry-run/readiness gate scripts, and an operator
> runbook. The automated verification surface is the dark-factory `verify-dark.ps1` gate harness
> plus structural grep assertions — NOT a unit-test framework. All requirement verifications were
> re-run green on 2026-06-26 during this audit (State B reconstruct-from-artifacts).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | dark-factory gate harness (`scripts/verify-dark.ps1`) + structural grep assertions + cargo build/publish dry-run |
| **Config file** | none — gates auto-discovered from `scripts/gates/*.ps1` |
| **Quick run command** | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` |
| **Full suite command** | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness && pwsh -File scripts/release-dry-run.ps1` |
| **Estimated runtime** | ~2 s (readiness gate) · ~minutes (full dry-run with cargo/maturin/npm) |

---

## Sampling Rate

- **After every task commit:** Run `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness`
- **After every plan wave:** Run the readiness gate + targeted structural greps
- **Before `/gsd:verify-work`:** Readiness gate must PASS (exit 0)
- **Max feedback latency:** ~2 seconds (readiness gate)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 97-01-01 | 01 | 1 | RLS-05 | T-97-01 | Six version-family crates at 0.66.0; no 0.62.2 / loose 0.62 pin survives | gate | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (version_family + stale_062_2 + leapfrog checks) | ✅ | ✅ green |
| 97-01-02 | 01 | 1 | RLS-05 | T-97-02 | Cargo.lock regenerated, workspace builds clean, no external crate drift | gate | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (cargo_lock) + `cargo build --workspace --all-targets` | ✅ | ✅ green |
| 97-02-01 | 02 | 1 | RLS-06 | T-97-04 / T-97-05 | Pre-package signing precedes Package; admin-extract payload gate + Authenticode wrapper gate both present | grep | `grep -n 'pre-package' .github/workflows/release.yml; grep -n 'Verify MSI payload signatures' .github/workflows/release.yml; grep -n 'Verify Authenticode' .github/workflows/release.yml` | ✅ | ✅ green |
| 97-02-01b | 02 | 1 | RLS-06 | T-97-06 | No reusable `image-build.yml` `uses:` job (0s startup_failure guard); 5 build legs | grep | `grep -nE '^\s*uses:\s*\./.github/workflows/image-build.yml' .github/workflows/release.yml` (want 0) | ✅ | ✅ green |
| 97-02-02 | 02 | 1 | RLS-08 | T-97-07 | Homebrew download-url at fork OscarMackJr/nono; no always-further/nolabs-ai refs | grep | `grep -c 'OscarMackJr/nono' release.yml` (≥1); `grep -c 'always-further/nono\|nolabs-ai/nono' release.yml` (0) | ✅ | ✅ green |
| 97-03-01 | 03 | 1 | RLS-07 | T-97-08 / T-97-09 | Dry-run only — no `twine upload`, no token/credential in script | grep | `grep -ci 'twine upload' scripts/release-dry-run.ps1` (0); `grep -ci 'token' scripts/release-dry-run.ps1` (0) | ✅ | ✅ green |
| 97-03-02 | 03 | 1 | RLS-06 | T-97-10 | Absent toolchains → documented SKIP signature, not silent pass; crates leg always-runs fail-closed | gate | `pwsh -File scripts/release-dry-run.ps1` (fail-closed orchestrator; SKIP_HOST_UNAVAILABLE signature) | ✅ | ✅ green (crates leg) · ⚠️ PyPI/npm host-gated → Manual-Only |
| 97-04-01 | 04 | 1 | RLS-09 | T-97-11 | release-readiness gate returns PASS on prepared tree, FAIL when build_notes/.gsd staged | gate | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (PASS exit 0 / FAIL exit 2) | ✅ | ✅ green |
| 97-04-02 | 04 | 1 | RLS-09 | T-97-12 / T-97-13 | RELEASE-RUNBOOK.md embeds pre-push checklist, names both mandatory gates, PREPARE-ONLY | grep | `grep -c 'build_notes' RELEASE-RUNBOOK.md` (≥1); `grep -c 'release-readiness' RELEASE-RUNBOOK.md` (≥1); `grep -c 'PREPARE-ONLY' RELEASE-RUNBOOK.md` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No test framework install needed — the
dark-factory `verify-dark.ps1` harness and the structural grep/build assertions are already present
and were re-run green during this audit. No `*.test.*` / `test_*` unit-test files are applicable to a
version-bump + pipeline + scripts + runbook phase.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Actual CI MSI signing run (Azure Trusted Signing) and signed-payload verification on a real release tag | RLS-06 | Requires a GitHub Actions runner + Azure Trusted Signing credentials; executes only on tag push. The phase verifies signing ORDER and gate EXISTENCE structurally (local-runnable); the live signed-artifact assertion is operator-push-time by design (PREPARE-ONLY). | Push tag `v0.66.0`; observe release.yml run; confirm "Verify Authenticode signatures" + "Verify MSI payload signatures" steps pass on the produced MSIs. |
| Live registry publish of the 3-crate set (nono → nono-proxy → nono-cli), nono-py wheel, nono-ts package | RLS-07 | A live `cargo publish` / `twine upload` / `npm publish` is irreversible and credentialed; the phase deliberately stops at `--dry-run`. PyPI/npm dry-run legs are also host-toolchain-gated (maturin/twine/npm may be absent → documented SKIP). | Execute RELEASE-RUNBOOK.md Steps 3–5 with operator credentials AFTER both mandatory gates pass. |
| nono-py / nono-ts sibling-repo version commits at 0.66.0 | RLS-05 | Sibling repos are separate git repos (`C:\Users\OMack\nono-py`, `C:\Users\OMack\nono-ts`); edits are on disk but committed by the operator in their own repos, outside this workspace's gate scope. | In each sibling repo: confirm `0.66.0` in manifests, commit, and tag in lockstep with the core release. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify (gate or structural grep) or are documented Manual-Only with reason
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (every task carries a gate/grep)
- [x] Wave 0 covers all MISSING references (none — existing harness covers all)
- [x] No watch-mode flags
- [x] Feedback latency < 3s (readiness gate ~2s)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-06-26
