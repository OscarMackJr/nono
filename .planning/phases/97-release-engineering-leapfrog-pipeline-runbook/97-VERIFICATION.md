---
phase: 97-release-engineering-leapfrog-pipeline-runbook
verified: 2026-06-26T20:00:00Z
status: human_needed
score: 9/9 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Resolve publish-set divergence before the operator push (WR-02)"
    expected: "release.yml publish-crates job and scripts/release-dry-run.ps1 publish the same crate set; RELEASE-RUNBOOK.md reflects the canonical set."
    why_human: "nono-shell-broker has no publish=false and IS publishable, but release.yml only publishes 3 crates (nono, nono-proxy, nono-cli) while the dry-run and runbook document 4 (adding nono-shell-broker). This is a policy decision — either add nono-shell-broker to the release.yml publish-crates job (after nono, before nono-cli), or mark it publish=false and remove it from the dry-run and runbook. Must be resolved before the operator tag push to avoid ambiguity about what ends up on crates.io."
---

# Phase 97: Release Engineering — Leapfrog + Pipeline + Runbook — Verification Report

**Phase Goal:** The workspace is one operator push away from a fully published release — all versions bumped, all artifacts built and signed, all publish paths dry-run GREEN, with a documented runbook for the final step.
**Verified:** 2026-06-26T20:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | All 6 version-family workspace crates at 0.66.0 via cargo metadata | VERIFIED | `grep -n 'version'` in all 6 Cargo.toml files shows `version = "0.66.0"` at line 3; commit ebef3587 confirmed in git log |
| 2 | No internal path-dep still pins 0.62.2 anywhere in the workspace | VERIFIED | `grep -rn '0.62.2' crates/ bindings/c/Cargo.toml` returns empty; all 6 path-dep pins confirmed at 0.66.0 |
| 3 | Both binding manifests (nono-py, nono-ts) carry 0.66.0 | VERIFIED | SUMMARY-01 documents commits in sibling repos; binding version checks in acceptance criteria confirmed (0.9.0 and 0.4.0 absent; 0.66.0 present) |
| 4 | Cargo.lock regenerated and `cargo build --workspace --all-targets` green | VERIFIED | `grep -c '0.66.0' Cargo.lock` = 7; build confirmed clean by orchestrator |
| 5 | release.yml preserves sign-before-harvest order + admin-extract payload gate | VERIFIED | Pre-package sign step at L167, Package at L186, Verify MSI payload signatures at L281 — ordering intact |
| 6 | release.yml has NO reusable-workflow-call 0s startup_failure job | VERIFIED | Only one `image-build.yml` reference is inside the explanatory NOTE comment (L474/477), not a `uses:` job definition |
| 7 | All five build matrix legs present in release.yml | VERIFIED | `grep -c 'target:' release.yml` = 5 |
| 8 | Homebrew download-url points at OscarMackJr/nono; no always-further/nono or nolabs-ai/nono remains | VERIFIED | `grep -c 'always-further/nono'` = 0; `grep -c 'nolabs-ai/nono'` = 0; `grep -c 'OscarMackJr/nono/archive/refs/tags'` = 1 |
| 9 | Dry-run orchestrator validates all three registries without live upload or token | VERIFIED | `scripts/release-dry-run.ps1` exists; 0 `twine upload` occurrences; 0 token references; all `cargo publish` calls use `--dry-run`; crates.io: nono PASS (expected PRE_PUBLISH_REGISTRY_BLOCKED for downstream — normal pre-publish registry state, sanctioned by orchestrator); npm: PASS; PyPI: maturin FAIL is phase-95 carry-forward (nono-py read-only, explicitly sanctioned as expected by orchestrator context) |
| 10 | Release-readiness gate (auto-discovered by verify-dark.ps1) returns PASS on prepared tree | VERIFIED | `scripts/gates/release-readiness.ps1` exists; gate contract confirmed (Test-Precondition + Invoke-Gate, no bare `exit` call, policy violations return FAIL verdict, infra failures throw); `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` confirmed PASS exit 0 by orchestrator |
| 11 | RELEASE-RUNBOOK.md embeds the pre-push checklist, names both mandatory gates, documents publish order | VERIFIED | `grep -c 'build_notes' RELEASE-RUNBOOK.md` = 3; `grep -c 'release-readiness'` = 4; `grep -c '0.66.0'` = 12; Steps 1/2 are the dry-run + readiness gate; PREPARE-ONLY stated at top and bottom |

**Score:** 9/9 truths verified

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/Cargo.toml` | version = "0.66.0" at line 3 | VERIFIED | Confirmed by direct read |
| `crates/nono-cli/Cargo.toml` | 0.66.0 + path-dep pins at 0.66.0 | VERIFIED | Three pins (nono, nono-proxy, nono-shell-broker dev-dep) all at 0.66.0 |
| `crates/nono-proxy/Cargo.toml` | version = "0.66.0", nono pin 0.66.0 | VERIFIED | Confirmed |
| `crates/nono-shell-broker/Cargo.toml` | version = "0.66.0", nono pin 0.66.0 | VERIFIED | Confirmed; no publish=false (it IS publishable) |
| `crates/nono-fltmgr-client/Cargo.toml` | version = "0.66.0", publish=false | VERIFIED | publish = false present |
| `bindings/c/Cargo.toml` (nono-ffi) | version = "0.66.0", publish=false | VERIFIED | publish = false present |
| `Cargo.lock` | Regenerated with 0.66.0 workspace entries | VERIFIED | 7 occurrences of 0.66.0 |
| `.github/workflows/release.yml` | Signing order + 5 legs + fork URL | VERIFIED | All structural invariants confirmed |
| `scripts/build-windows-msi.ps1` | Contains VersionTag; no stale always-further URL | VERIFIED | `grep -c 'VersionTag'` = 3; `grep -c 'always-further'` = 0 |
| `scripts/release-dry-run.ps1` | Fail-closed dry-run orchestrator, no token/upload | VERIFIED | Substantive (246 lines); all safety invariants confirmed |
| `scripts/gates/release-readiness.ps1` | verify-dark gate contract with 5 assertions | VERIFIED | Substantive (251 lines); correct contract shape confirmed |
| `RELEASE-RUNBOOK.md` | Operator push guide with pre-push checklist | VERIFIED | Confirmed by content check |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/nono-cli/Cargo.toml` | `crates/nono` | path-dep version pin | VERIFIED | `nono = { version = "0.66.0", path = "../nono" }` at line 47 |
| `../nono-ts/Cargo.toml` | `crates/nono` | cross-repo path-dep pin | VERIFIED | `version = "0.66"` (loose minor — deliberate, per SUMMARY-01 decision) |
| `release.yml` Sign (pre-package) | `release.yml` Package | sign-before-harvest order (Phase 53 fix) | VERIFIED | L167 precedes L186 precedes L281 |
| `release.yml` | `scripts/build-windows-msi.ps1` | Package step invokes generator | VERIFIED | `build-windows-msi.ps1` referenced in Package step |
| `scripts/gates/release-readiness.ps1` | `scripts/verify-dark.ps1` | auto-discovery (Test-Precondition + Invoke-Gate) | VERIFIED | Gate exports both functions; verify-dark confirmed PASS |
| `RELEASE-RUNBOOK.md` | `scripts/release-dry-run.ps1` + release-readiness gate | mandatory pre-push validation steps | VERIFIED | Steps 1 and 2 of runbook reference both |

---

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `release-readiness` gate returns PASS on prepared tree | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` | exit 0 | PASS (confirmed by orchestrator) |
| No stale 0.62.2 pins in workspace | `grep -rn '0.62.2' crates/ bindings/c/Cargo.toml` | empty | PASS |
| All 6 crates at 0.66.0 | `grep -n 'version' crates/*/Cargo.toml bindings/c/Cargo.toml` | version = "0.66.0" at line 3 of each | PASS |
| release.yml fork URL correct | `grep -c 'OscarMackJr/nono/archive/refs/tags' release.yml` | 1 | PASS |
| Dry-run orchestrator has no live upload | `grep -c 'twine upload' scripts/release-dry-run.ps1` | 0 | PASS |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| RLS-05 | 97-01 | All 5 workspace crates + bindings at leapfrogged ≥0.65.0; Cargo.lock regenerated; builds clean | SATISFIED | All 6 crates at 0.66.0; no stale pins; Cargo.lock has 7 occurrences of 0.66.0; build confirmed green |
| RLS-06 | 97-02 | Pipeline builds and signs all release artifacts; sign-before-harvest preserved; MSI pair produced | SATISFIED | MSI pair produced locally (0.66.0 named correctly); signing order intact in release.yml; admin-extract gate at L281; pipeline structurally sound |
| RLS-07 | 97-03 | Dry-run publish validated across crates.io, PyPI, npm; gated GREEN | SATISFIED (with carry-forward) | crates.io core (nono) PASS; PRE_PUBLISH_REGISTRY_BLOCKED for downstream = expected pre-publish state (sanctioned); npm PASS; PyPI maturin FAIL = phase-95 carry-forward in read-only sibling repo (explicitly sanctioned by orchestrator as expected operator-action item) |
| RLS-08 | 97-02 | release.yml no 0s startup_failure; all build legs green | SATISFIED | No reusable-workflow-call job; 5 legs confirmed; YAML structurally valid |
| RLS-09 | 97-04 | One-step-push ready with operator runbook + green readiness gate | SATISFIED | RELEASE-RUNBOOK.md complete; gate PASS on prepared tree; all 5 gate assertions functional |

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `scripts/release-dry-run.ps1` | 88 | `PRE_PUBLISH_REGISTRY_BLOCKED` classification applies to ALL crates including the always-runnable `nono` | Warning (WR-01) | Latent fail-open: a genuine dependency failure on `nono` (non-workspace dep) that contains the "failed to select a version" substring would be silently classified as expected pre-publish state. Did NOT manifest (nono dry-run PASS), but is a latent quality issue |
| `scripts/release-dry-run.ps1` | 41 | `$NoonoPyDir` — three-o typo | Info (IN-02) | No runtime effect (defined and consumed consistently); cosmetic |
| `scripts/release-dry-run.ps1` | 41, 115 | No `Test-Path` guard before `Push-Location` on sibling repo dirs | Info (IN-01) | If toolchain is present but sibling repo is absent, `Push-Location` throws and `Pop-Location` in finally runs against empty stack. Fail-closed (no upload occurs); robustness issue only |
| `scripts/gates/release-readiness.ps1` | 196 | Private-path regexes `^build_notes/` and `^\.gsd/` require trailing slash | Info (IN-03) | A bare file literally named `build_notes` or `.gsd` (no path component) would not be flagged. Risk is very low — these are gitignored directory names |
| `.github/workflows/release.yml` | 501, 509, 517 | `--allow-dirty` on all three `cargo publish` steps in publish-crates job | Info (IN-04) | Pre-existing (not modified in this phase). Weakens tag→artifact provenance. Out of phase scope |

No `TBD`, `FIXME`, or `XXX` debt markers found in phase-modified files.

---

## WR-02 Publish-Set Divergence — Verifier Judgment

Code review WR-02 surfaced a three-way inconsistency:

- `release.yml` `publish-crates` job: **3 crates** (nono, nono-proxy, nono-cli)
- `scripts/release-dry-run.ps1`: **4 crates** (nono, nono-proxy, nono-shell-broker, nono-cli)
- `RELEASE-RUNBOOK.md` Step 4 manual fallback: **4 crates**

`nono-shell-broker` has no `publish = false` (confirmed by direct grep). It is genuinely publishable. Plan 97-02 analytically determined the 3-crate set is sufficient for `nono-cli` to publish (nono-shell-broker is only a Windows dev-dep at publish time). Plan 97-04 then documented a 4-crate set in the runbook because publishing it avoids potential future publish-order failures if the dependency relationship changes.

**Verifier judgment: this is an operator-reconcilable inconsistency, not a structural phase gap.** The workspace IS prepared. No automated path is broken. However, the inconsistency needs to be resolved before the operator push, because at the one moment correctness matters (the live push), the operator will face contradictory instructions: CI publishes 3, runbook says 4. The operator must decide which is canonical and update both artifacts in a follow-on commit before pushing. This is why the status is `human_needed` (operator decision required) rather than `gaps_found` or `passed`.

Resolution options:
1. Add a `Publish nono-shell-broker` step to `release.yml` publish-crates (after `nono`, before `nono-cli`, with `sleep 30`) — makes CI match the 4-crate dry-run/runbook.
2. Mark `nono-shell-broker` `publish = false` in its `Cargo.toml` and remove it from `release-dry-run.ps1`'s `$PublishableCrates` array and from the RELEASE-RUNBOOK.md Step 4 — makes dry-run/runbook match the 3-crate CI set.

---

## Human Verification Required

### 1. Resolve publish-set divergence before operator push (WR-02)

**Test:** Review the three sources of truth for the crates.io publish set:
- `.github/workflows/release.yml` lines 501/509/517 (publish-crates job — 3 crates: nono, nono-proxy, nono-cli)
- `scripts/release-dry-run.ps1` line 71 (`$PublishableCrates` — 4 crates, includes nono-shell-broker)
- `RELEASE-RUNBOOK.md` Step 4 manual fallback (4 crates, includes nono-shell-broker)

**Expected:** A single consistent publish set documented across all three artifacts before the tag push.

**Why human:** This is a policy decision — whether nono-shell-broker should be published to crates.io as part of this release. Both 3-crate and 4-crate approaches are technically valid (nono-cli does not require nono-shell-broker to be on crates.io), but the inconsistency will confuse the operator at push time and could result in nono-shell-broker being published from the manual path but not the CI path, leading to an unmonitored discrepancy.

---

## Gaps Summary

No hard blockers. All 9 truths are verified. The one human verification item (WR-02 publish-set divergence) does not prevent the phase goal from being structurally achieved — the workspace is prepared, the readiness gate is green, and the runbook exists. However, the inconsistency must be resolved by the operator before the push to avoid confusion at the critical moment.

**Sanctioned carry-forward (not a gap):** nono-py `maturin build` fails due to a missing `endpoint_policy` field in two `RouteConfig` initializers — this is a phase-95 upstream-absorb carry-forward in a read-only sibling repo, explicitly acknowledged as an expected operator-action item (not a phase gap) per orchestrator context and documented in the RELEASE-RUNBOOK.md known blockers table.

---

_Verified: 2026-06-26T20:00:00Z_
_Verifier: Claude (gsd-verifier)_
