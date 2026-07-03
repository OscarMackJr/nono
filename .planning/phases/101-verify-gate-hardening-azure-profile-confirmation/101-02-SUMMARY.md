---
phase: 101-verify-gate-hardening-azure-profile-confirmation
plan: 02
subsystem: infra
tags: [ci, github-actions, authenticode, signtool, powershell, code-signing]

# Dependency graph
requires:
  - phase: 101-01
    provides: "scripts/verify-authenticode.ps1 — dot-sourceable Assert-TrustedSignature helper (Mode Strict|Informational)"
provides:
  - "release.yml: 3 verify sites (loose .exe/broker/MSI loop, MSI-payload extract, zip-payload verify) now dot-source and call Assert-TrustedSignature"
  - "trusted-signing-smoke.yml: verify site now dot-sources and calls Assert-TrustedSignature -Mode Strict"
  - "Zero inline Get-AuthenticodeSignature/Status-ne-Valid gating logic remains in either workflow YAML — the fail-closed check lives only inside scripts/verify-authenticode.ps1"
affects: [101-03, 104]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CI verify steps dot-source a shared PowerShell helper as the first line of their run: block, then call Assert-TrustedSignature with an explicit -Mode argument per call site"
    - "Extension-based Strict/Informational grouping stays at the YAML call site (which files get which -Mode), never inferred inside the helper"

key-files:
  created: []
  modified:
    - .github/workflows/release.yml
    - .github/workflows/trusted-signing-smoke.yml

key-decisions:
  - "Also wired a third, previously-unenumerated inline Status -ne 'Valid' gate in release.yml's \"Verify signed binaries inside zip (Windows)\" step (~line 329) — the plan's read_first/action sections only named Site 1 (~259) and Site 2 (~281), but this plan's own security_notes and acceptance criterion (grep -c \"sig.Status -ne\" returns 0, whole-file) require zero duplicated inline gates anywhere in release.yml. Fixing it is squarely SIGN-02's own stated goal (collapse duplicated gating logic into the one shared helper), not scope creep."
  - "Kept the pre-existing $sig = Get-AuthenticodeSignature smoke\\nono-smoke.exe call in trusted-signing-smoke.yml purely to preserve the ambient Status:/Signer:/Issuer: echo lines the task explicitly required to survive; the actual fail-closed gate now runs entirely inside Assert-TrustedSignature -Mode Strict, called immediately after."

requirements-completed: [SIGN-02]

# Metrics
duration: 2min
completed: 2026-07-02
---

# Phase 101 Plan 02: Verify-Gate Hardening — Wire Assert-TrustedSignature into CI Summary

**Collapsed 4 duplicated inline `Get-AuthenticodeSignature`/`Status -ne 'Valid'` fail-closed gates across `release.yml` (3 sites) and `trusted-signing-smoke.yml` (1 site) into calls to the shared `Assert-TrustedSignature -Mode Strict|Informational` helper, with the `.sys` WHQL carve-out preserved verbatim.**

## Performance

- **Duration:** 2 min (commit-to-commit)
- **Started:** 2026-07-02T20:21:39-04:00 (Task 1 commit)
- **Completed:** 2026-07-02T20:22:06-04:00 (Task 2 commit)
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `release.yml` Site 1 (loose `.exe`/broker/MSI-wrapper loop, ~line 259): dot-sources the helper, calls `Assert-TrustedSignature -Path $artifact -Mode Strict` inside the unchanged `foreach` loop.
- `release.yml` Site 2 (MSI-payload administrative extract, ~line 281): dot-sources the helper; `.exe` payloads call `-Mode Strict`; the `.sys` WHQL/cross-sign carve-out calls `-Mode Informational`; the carve-out comment block ("SEPARATE signing regime") and the `msiexec /a` admin-extract block are both untouched, byte-for-byte.
- `release.yml` third site ("Verify signed binaries inside zip (Windows)", ~line 329, not enumerated in the plan's `read_first`): also dot-sources the helper and calls `-Mode Strict` — closing a duplicated inline gate the plan's file inventory missed, required by this plan's own whole-file grep acceptance criterion.
- `trusted-signing-smoke.yml`'s verify step: dot-sources the helper, calls `Assert-TrustedSignature -Path "smoke\nono-smoke.exe" -Mode Strict`; the pre-existing `Status:`/`Signer:`/`Issuer:` ambient echo lines are preserved unchanged.
- Verified via grep: `sig.Status -ne` returns `0` occurrences across both files; `SEPARATE signing regime` carve-out comment still present and unmoved; `msiexec` admin-extract line unchanged; both files pass `yaml.safe_load`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire release.yml Site 1 and Site 2** - `5bd7567c` (feat)
2. **Task 2: Wire trusted-signing-smoke.yml verify site** - `51d75786` (feat)

## Files Created/Modified
- `.github/workflows/release.yml` - 3 verify `run:` blocks (Site 1, Site 2, zip-payload verify) now dot-source `scripts/verify-authenticode.ps1` and call `Assert-TrustedSignature` with an explicit `-Mode` per call site; `.sys` carve-out comment + `msiexec` admin-extract block untouched.
- `.github/workflows/trusted-signing-smoke.yml` - verify step dot-sources the helper and calls `Assert-TrustedSignature -Mode Strict`; ambient `Status:`/`Signer:`/`Issuer:` echo lines preserved.

## Decisions Made
- Wired the previously-unenumerated third inline gate (zip-payload verify step) in `release.yml` — see `key-decisions` above; required by the plan's own explicit whole-file acceptance criterion, not by the task's `read_first`/`action` text, which only named two sites.
- Retained the ambient `Get-AuthenticodeSignature` call in the smoke workflow solely to preserve its `Signer:`/`Issuer:` echo lines, per the task's explicit instruction; this call's `.Status` is no longer what gates anything — `Assert-TrustedSignature` performs the actual (dual-engine, retrying) fail-closed check immediately after.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Wired a third, plan-unenumerated inline fail-closed gate in release.yml**
- **Found during:** Task 1 (post-edit acceptance-criteria verification)
- **Issue:** The plan's `read_first`/`action` for Task 1 named only Site 1 (~L259) and Site 2 (~L281-321), and explicitly said "the zip-creation step at line 325+ is out of scope." However, immediately after that zip-*creation* step is a separate "Verify signed binaries inside zip (Windows)" step with its own inline `Get-AuthenticodeSignature`/`if ($sig.Status -ne "Valid")`/`exit 1` block — a fourth duplicate of the same fail-closed logic this plan exists to collapse. Running the plan's own acceptance-criteria grep (`grep -c "sig.Status -ne" .github/workflows/release.yml` must return `0`) after only fixing Sites 1/2 would have returned `1`, failing the plan's stated success criterion, and this plan's `<security_notes>` explicitly required zero occurrences file-wide.
- **Fix:** Dot-sourced the helper as the first line of that step's `run:` block and replaced its inline gate with `Assert-TrustedSignature -Path $extractedBinary -Mode Strict`, identical in shape to Site 1's refactor.
- **Files modified:** `.github/workflows/release.yml`
- **Verification:** `grep -c "sig.Status -ne" .github/workflows/release.yml` returns `0`; `yaml.safe_load` still exits 0.
- **Committed in:** `5bd7567c` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 — missing critical functionality required by this plan's own explicit whole-file acceptance criterion).
**Impact on plan:** Strictly in-scope with SIGN-02's stated goal (no duplicated inline Authenticode gate remains anywhere; the check lives only in `scripts/verify-authenticode.ps1`). No architectural change, no weakening of any fail-closed condition — same shape as the two enumerated sites. No scope creep beyond the plan's own acceptance bar.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required. (SIGN-01's Azure `az trustedsigning certificate-profile show` operator checkpoint remains scoped to a later plan in this phase per `101-CONTEXT.md` D-05 — not required for this plan's SIGN-02 wiring deliverable.)

## Next Phase Readiness
- All three CI verify sites named in the phase's `must_haves` (`release.yml` Site 1, `release.yml` Site 2, `trusted-signing-smoke.yml`), plus the plan-unenumerated fourth zip-payload site, now call `Assert-TrustedSignature` with an explicit `-Mode` — no inline duplicate of the fail-closed check remains in either YAML file.
- SIGN-03 (smoke workflow GREEN on `windows-latest`, proving a real Trusted Signing chain resolves to `Valid`) still depends on SIGN-01 (Azure `PublicTrust`-vs-`PublicTrustTest` profile confirmation, operator checkpoint) per `101-CONTEXT.md` D-05 — that checkpoint is scoped to a later plan in this phase and is unaffected by this plan's wiring work.
- No blockers. This plan only edited two CI workflow YAML files; no new dependencies, no Rust/library-side changes, no cross-target clippy exposure.

---
*Phase: 101-verify-gate-hardening-azure-profile-confirmation*
*Completed: 2026-07-02*

## Self-Check: PASSED

- FOUND: .github/workflows/release.yml (dot-source + Assert-TrustedSignature calls present, verified via grep)
- FOUND: .github/workflows/trusted-signing-smoke.yml (dot-source + Assert-TrustedSignature call present, verified via grep)
- FOUND: commit 5bd7567c (feat, Task 1)
- FOUND: commit 51d75786 (feat, Task 2)
