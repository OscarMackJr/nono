---
phase: 97-release-engineering-leapfrog-pipeline-runbook
reviewed: 2026-06-26T17:56:25Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - scripts/release-dry-run.ps1
  - scripts/gates/release-readiness.ps1
  - scripts/build-windows-msi.ps1
  - .github/workflows/release.yml
  - crates/nono/Cargo.toml
  - crates/nono-cli/Cargo.toml
  - crates/nono-proxy/Cargo.toml
  - crates/nono-shell-broker/Cargo.toml
  - crates/nono-fltmgr-client/Cargo.toml
  - bindings/c/Cargo.toml
findings:
  critical: 0
  warning: 2
  info: 4
  total: 6
status: issues_found
---

# Phase 97: Code Review Report

**Reviewed:** 2026-06-26T17:56:25Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Reviewed the release-engineering artifacts for the 0.66.0 leapfrog/prepare-only milestone:
the dry-run orchestrator, the verify-dark readiness gate, the MSI build script, the release
workflow, and the six workspace `Cargo.toml` manifests.

The high-value safety invariants all hold:

- **`release-dry-run.ps1` is upload-free.** Every `cargo publish` call carries `--dry-run`;
  the PyPI leg uses `maturin build` + `twine check` (never `maturin publish`/`twine upload`);
  the npm leg uses `npm publish --dry-run`. No registry token or credential appears anywhere
  in the script. Sibling-repo `cd`s use `Push-Location`/`Pop-Location` in `try/finally`, so the
  working directory is restored on every path including error.
- **`release-readiness.ps1` honors its gate contract.** It RETURNS an ordered verdict object
  and never calls `exit`; policy violations return `verdict = FAIL` (fail-closed: a missing or
  wrong-version crate is reported as a mismatch, never silently passed), while infrastructure
  faults `throw` for the runner to map to a harness error. The private-path assertion uses
  trailing-slash-anchored regexes (`^build_notes/`, `^\.gsd/`), i.e. component-boundary matching
  rather than a bare `starts_with`, satisfying the CLAUDE.md path rule.
- **`release.yml` invariants are intact.** Sign-before-harvest ordering is preserved (pre-package
  binary sign at L167 → Package at L186 → MSI wrapper sign at L241 → Authenticode verify at L259
  → admin-extract payload verify at L281). No reusable-workflow-call job exists (only the
  explanatory NOTE comment at L474). The homebrew `download-url` points solely at
  `OscarMackJr/nono`; no `always-further`/`nolabs-ai` references remain in either changed file.
- **Cargo manifests are internally consistent.** All six crates are at `0.66.0`; every internal
  path dependency pins `version = "0.66.0"`; `publish = false` is set on exactly the two
  non-published crates (`nono-fltmgr-client`, `nono-ffi`); no stale `0.62.2` strings remain.

Two Warnings concern fail-open / consistency defects in the prepare tooling (not the live
pipeline). No Critical findings.

## Warnings

### WR-01: Dry-run masks genuine dependency-resolution failures as "pre-publish blocked" (fail-open)

**File:** `scripts/release-dry-run.ps1:88`
**Issue:** The crates.io leg downgrades ANY non-zero `cargo publish --dry-run` whose output
contains the substring `failed to select a version for the requirement` from a hard `FAIL` to
`PRE_PUBLISH_REGISTRY_BLOCKED`, which is treated as non-failing (script exits 0). This match is
applied uniformly to all four crates, including the base crate `nono`, which has no unpublished
workspace dependency — so any real resolution error for `nono` (e.g. a yanked or unsatisfiable
third-party dep) is silently classified as an expected pre-publish state and the orchestrator
reports PASS. The same masking can hide a third-party (non-`nono`) resolution failure on the
downstream crates, since the pattern does not check *which* requirement failed. For a
release-readiness validator this is a fail-open: a genuinely broken package can pass the gate.
**Fix:** Scope the lenient classification to (a) downstream crates only — never the always-runnable
core `nono` — and (b) only when the unresolved requirement is a workspace crate. For example:
```powershell
# Only downstream crates may be PRE_PUBLISH_REGISTRY_BLOCKED, and only on the workspace dep.
$isDownstream = $crate -ne 'nono'
$blockedOnWorkspaceDep = $outStr -match 'failed to select a version for the requirement `?nono'
if ($isDownstream -and $blockedOnWorkspaceDep) {
    Add-Result "crates.$crate" "PRE_PUBLISH_REGISTRY_BLOCKED" "nono ^0.66.0 not yet on crates.io"
} else {
    Add-Result "crates.$crate" "FAIL" "cargo publish --dry-run -p $crate exited $exitCode"
}
```

### WR-02: Publish-set divergence — 3 crates in CI vs 4 in dry-run/runbook

**File:** `scripts/release-dry-run.ps1:71`, `.github/workflows/release.yml:500-519`
**Issue:** Three sources disagree on the publishable crate set:
- `release.yml` `publish-crates` publishes **3** crates: `nono` → `nono-proxy` → `nono-cli`.
- `release-dry-run.ps1` dry-runs **4**: `nono`, `nono-proxy`, `nono-shell-broker`, `nono-cli`.
- `RELEASE-RUNBOOK.md` (Plan 04) documents a **4**-crate operator publish order including
  `nono-shell-broker`; the Plan 02 summary, by contrast, argues 3 is sufficient.

`nono-shell-broker` has no `publish = false`, so it is genuinely publishable, but `nono-cli`
depends on it only as a Windows `dev-dependency`, so omitting it from CI does not break the
`nono-cli` publish. The result is nonetheless an inconsistency: an operator following the runbook
publishes a crate the CI never publishes, and the CI-published set omits a crate the runbook and
dry-run validate. This will confuse the operator at the one moment correctness matters (the live
push) and can leave `nono-shell-broker` at 0.66.0 unpublished on crates.io.
**Fix:** Pick one source of truth. Either add a `Publish nono-shell-broker` step to
`release.yml` (after `nono`, before `nono-cli`, with the same `sleep 30` indexing gap) so CI
matches the 4-crate dry-run/runbook, or mark `nono-shell-broker` `publish = false` and drop it
from both the dry-run set and the runbook. Reconcile the Plan 02 (3-crate) vs Plan 04 (4-crate)
narrative accordingly.

## Info

### IN-01: Sibling-repo directories not guarded before `Push-Location`

**File:** `scripts/release-dry-run.ps1:41-42,115,182`
**Issue:** `$NoonoPyDir`/`$NonoTsDir` are computed but never `Test-Path`-checked. With
`$ErrorActionPreference = "Stop"`, if a toolchain (maturin/npm) is present but the sibling repo
(`../nono-py`, `../nono-ts`) is absent, `Push-Location` throws and the `finally { Pop-Location }`
then runs against an empty location stack, producing a confusing secondary error instead of a
clean SKIP. Behavior is fail-closed (no upload occurs), so this is robustness, not safety.
**Fix:** Guard each leg with `if (-not (Test-Path -LiteralPath $NoonoPyDir)) { Add-Result ... "SKIP" "nono-py repo not found: SKIP_HOST_UNAVAILABLE"; }` before `Push-Location`, mirroring the toolchain-absent SKIP convention already used for maturin/twine/npm.

### IN-02: Variable name typo `$NoonoPyDir`

**File:** `scripts/release-dry-run.ps1:41,115`
**Issue:** The nono-py directory variable is spelled `$NoonoPyDir` (three `o`s). It is defined
and consumed consistently, so there is no runtime effect, but the typo is confusing next to the
correctly spelled `$NonoTsDir`.
**Fix:** Rename to `$NonoPyDir` at both sites.

### IN-03: Private-path assertion misses a bare file named `build_notes` / `.gsd`

**File:** `scripts/gates/release-readiness.ps1:196`
**Issue:** The regexes `^build_notes/` and `^\.gsd/` require a trailing slash (a directory
prefix). A tracked or staged file at the repo root literally named `build_notes` or `.gsd`
(no trailing component) would not be flagged. Risk is very low — these are gitignored directory
names, not files — and the trailing-slash anchoring is the correct component-boundary choice for
the common case.
**Fix (optional):** Broaden to `^(build_notes|\.gsd)(/|$)` to also catch the bare-name edge case.

### IN-04: `cargo publish --allow-dirty` in the live publish job (pre-existing, adjacent)

**File:** `.github/workflows/release.yml:501,509,517`
**Issue:** The `publish-crates` steps pass `--allow-dirty`, which publishes even when the working
tree differs from the committed/tagged state. This job was not modified in this phase, but since
the phase hardens the release path it is worth flagging: `--allow-dirty` weakens the
tag→artifact provenance guarantee the rest of the workflow (trusted_root byte-identity, payload
signature verify) works to establish.
**Fix (optional, out of phase scope):** Drop `--allow-dirty` and ensure the checkout at the tag
ref is clean, or document why a dirty tree is expected at publish time.

---

_Reviewed: 2026-06-26T17:56:25Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
