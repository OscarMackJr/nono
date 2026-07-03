---
phase: 104-smoke-green-cut-the-trusted-signed-release
plan: 02
subsystem: infra
tags: [github-actions, release-yml, powershell, certutil, azure-trusted-signing, requirements-md, roadmap-md]

# Dependency graph
requires:
  - phase: 104-01
    provides: verify-authenticode.ps1 D-04 flush fix + NoCheck-mode untrusted-root classification
provides:
  - "release.yml publish-crates job neutralized via job-level if: false (Pitfall 104-A closed)"
  - "scripts/verify-release-yml-publish-selectors.ps1 — regression guard against stale -p selectors reappearing outside the neutralized job"
  - "scripts/azure/check-trusted-signing-root.ps1 — free local pre-check for Microsoft root-CTL propagation, avoids burning CI dispatches"
  - "REQUIREMENTS.md REL-01 / ROADMAP.md Phase 104 SC3 corrected to the Finding-B-accurate Verified-publisher condition"
affects: [104-03, 105-pub-02]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Job-level `if: false` (not step-level no-op) to fully neutralize a stale/deferred CI job without deleting its body"
    - "Bounded-range static regression guard: locate a job's line range by header markers, assert a required pattern inside it, scan for a forbidden pattern outside it"
    - "Start-Process -NoNewWindow -Wait -PassThru for native tool invocation (never bare `&` + $LASTEXITCODE), matching Invoke-SignToolVerify's existing convention"

key-files:
  created:
    - scripts/verify-release-yml-publish-selectors.ps1
    - scripts/azure/check-trusted-signing-root.ps1
  modified:
    - .github/workflows/release.yml
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "Neutralized publish-crates via job-level if: false rather than deleting/rewriting it — Phase 105 (PUB-02) owns the real selector rewrite; the 3 stale cargo publish lines stay textually present as documented deferred work"
  - "Regression guard script placed at scripts/ (not scripts/gates/) so it is NOT auto-discovered by verify-dark.ps1's Get-ChildItem scripts/gates harness — it's a standalone static check, not a Test-Precondition/Invoke-Gate contract implementer"
  - "certutil pre-check script is read-only/informational only — its PRESENT/ABSENT result is never wired into any gate or Assert-TrustedSignature; only the operator's own judgment in Plan 104-03 consumes it"
  - "Corrected REL-01/SC3 wording avoids repeating the literal disproven 'Microsoft ID Verified CS' string so the corrected line itself no longer matches the stale-heuristic grep (kept the untouched SIGN-03/CHOST-02/Phase-101-SC4/Phase-103-SC2 historical mentions intact)"

requirements-completed: [REL-01]

# Metrics
duration: 24min
completed: 2026-07-03
---

# Phase 104 Plan 02: Neutralize Stale Publish Job + Root Pre-Check + Doc Correction Summary

**Neutralized release.yml's stale `publish-crates` job with a job-level `if: false` guard (closing Pitfall 104-A), added a static regression guard and a local certutil-based Microsoft root-CTL pre-check, and corrected the disproven issuer-substring wording in REQUIREMENTS.md/ROADMAP.md to the Finding-B-accurate Verified-publisher condition.**

## Performance

- **Duration:** 24 min
- **Started:** 2026-07-03T21:17:00Z
- **Completed:** 2026-07-03T21:41:10Z
- **Tasks:** 3
- **Files modified:** 5 (1 workflow file, 2 new scripts, 2 planning docs)

## Accomplishments
- `publish-crates` job in `release.yml` can no longer redden the `v0.66.1` tag-push `Release` workflow run — its job-level condition is now `if: false`, with a dated comment explaining the Phase-102-rename mismatch and the Phase-105 re-enable plan. Its 3 `cargo publish -p nono*` lines and the pre-existing Phase-105 deferral comment are untouched.
- `scripts/verify-release-yml-publish-selectors.ps1` created and regression-proven this session: it locates the `publish-crates` job's bounded line range, asserts `if: false` is inside it, and fails if any OTHER enabled job (outside that range, ignoring comment lines) references the stale `-p nono`/`-p nono-proxy`/`-p nono-cli` selectors. Proven by temporarily injecting a throwaway `cargo publish -p nono --allow-dirty` line into `update-homebrew-core`, confirming the guard failed (exit 1, correct offending line number), then removing it and confirming PASS (exit 0) again.
- `scripts/azure/check-trusted-signing-root.ps1` created and live-run this session: pulls the current Microsoft Trusted Root Program CTL via `certutil -generateSSTFromWU` and checks for thumbprint `991D364E97882715B80ED978F53E1F35DC2F07C2` (Microsoft Enterprise ID Root CA 2021). Live result: **ABSENT** (exit 1), 554 certificates checked — matching 104-RESEARCH.md's own live-confirmed finding. No TLS bypass; uses `Start-Process -NoNewWindow -Wait -PassThru` per the repo's established non-bare-`&` convention.
- `REQUIREMENTS.md`'s REL-01 line and `ROADMAP.md`'s Phase 104 SC3 line no longer state the disproven `Microsoft ID Verified CS` issuer-substring heuristic — both now read: Verified publisher is gated on `Get-AuthenticodeSignature.Status -eq 'Valid'` plus a non-test signer (rejecting `CN=nono Test Signing`/`PublicTrustTest`), with the issuer captured informationally only, citing `101-SIGN03-SMOKE-VERDICT.md` Finding B. All other mentions (SIGN-03, CHOST-02, Phase 101 SC4, Phase 103 SC2 — all historical/already-closed) are untouched.

## Task Commits

Each task was committed atomically:

1. **Task 1: Neutralize the stale publish-crates job** - `bd18e928` (fix)
2. **Task 2a: Add the publish-selector regression guard** - `0ece6717` (test)
2. **Task 2b: Add the local root-availability pre-check** - `b77c6763` (feat)
3. **Task 3: Correct the disproven issuer-substring wording** - `0370f43e` (docs)

_Note: Task 2's two scripts were committed as two separate atomic commits (guard script, then pre-check script) since they are independently verifiable deliverables with distinct verification evidence (a regression-proof FAIL/PASS cycle for the first; a live ABSENT result for the second)._

## Files Created/Modified
- `.github/workflows/release.yml` - `publish-crates` job's condition replaced with `if: false` + dated deferral comment; publish steps/selectors and `update-homebrew-core` left untouched
- `scripts/verify-release-yml-publish-selectors.ps1` - new: static, local, fast regression guard against stale `-p nono`/`-p nono-proxy`/`-p nono-cli` selectors reappearing outside the neutralized job
- `scripts/azure/check-trusted-signing-root.ps1` - new: local, free, no-CI-minutes pre-check for Microsoft root-CTL propagation via `certutil -generateSSTFromWU`
- `.planning/REQUIREMENTS.md` - REL-01 line corrected to the Finding-B-accurate Verified-publisher condition
- `.planning/ROADMAP.md` - Phase 104 SC3 line corrected identically

## Decisions Made
- Job-level `if: false` chosen over deletion/rewrite of `publish-crates`, per 104-RESEARCH.md Pattern 3 and the plan's explicit constraint — guarantees the entire job body, including every `CARGO_REGISTRY_TOKEN` secret reference, never executes, while leaving Phase 105's rewrite scope untouched.
- The two new scripts were deliberately placed outside `scripts/gates/` so `verify-dark.ps1`'s `Get-ChildItem scripts/gates -Filter *.ps1` auto-discovery does not pick them up and does not require them to implement the `Test-Precondition`/`Invoke-Gate` two-function contract — they are standalone scripts with their own explicit exit codes, following `scripts/release-dry-run.ps1`/`scripts/verify-trust-root-cached.ps1` convention.
- Reworded the REL-01/SC3 correction to avoid repeating the literal disproven `Microsoft ID Verified CS` string verbatim in the corrected line itself, so the corrected line no longer matches the "stale heuristic" grep used to verify the fix (while still citing Finding B and describing the disproof in plain language) — a deviation in *wording precision* from the plan's illustrative example text, not in *substance* (the corrected condition matches the plan's intent exactly: Status=Valid + non-test signer + issuer informational-only).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Comment near `if: false` in release.yml initially caused a self-match in its own acceptance grep**
- **Found during:** Task 1 verification
- **Issue:** The first draft of the neutralization comment quoted `` `if: false` `` verbatim inside a prose sentence, which made `grep -n "if: false" .github/workflows/release.yml` return 2 matches (the comment sentence plus the real directive) instead of the required exactly-1 match.
- **Fix:** Reworded the comment to describe the mechanism ("the job-level condition below") without repeating the literal substring `if: false`, so only the actual YAML `if: false` directive matches.
- **Files modified:** `.github/workflows/release.yml`
- **Verification:** `grep -n "if: false" .github/workflows/release.yml` now returns exactly 1 match (line 485); YAML still parses (`python -c "import yaml; yaml.safe_load(...)"` → `YAML_OK`).
- **Committed in:** `bd18e928` (Task 1 commit)

**2. [Rule 1 - Bug] Doc correction initially re-introduced the literal disproven substring it was meant to remove**
- **Found during:** Task 3 verification
- **Issue:** The first draft of the corrected REL-01/SC3 wording cited the disproven heuristic by quoting its literal string (`` `Microsoft ID Verified CS` ``) as part of explaining *what was disproved* — which meant the corrected line still matched the acceptance grep for the stale heuristic, breaking the required exactly-2-matches-per-file acceptance criteria (the corrected line would have made it 3, not 2).
- **Fix:** Reworded to reference "the prior issuer-naming heuristic" generically instead of quoting the disproven string literally, while still preserving the Finding B cross-reference and the plain-language explanation of what was disproved.
- **Files modified:** `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`
- **Verification:** `grep -c "Microsoft ID Verified CS"` returns exactly 2 for each file (the untouched historical/already-closed lines only); `grep -c "Finding B" .planning/REQUIREMENTS.md` returns 1; `git diff --stat` touches exactly these 2 files.
- **Committed in:** `0370f43e` (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs in the executor's own first-draft wording caught by re-running the plan's own acceptance-criteria greps before committing)
**Impact on plan:** Both auto-fixes are purely textual/wording corrections caught during self-verification before commit; no scope creep, no change to the plan's intended mechanism or acceptance semantics.

## Issues Encountered
None beyond the two self-caught wording deviations documented above.

## User Setup Required
None - no external service configuration required. The `check-trusted-signing-root.ps1` pre-check currently (and correctly) reports ABSENT — this is expected, not a setup gap; Plan 104-03's poll-until-green operator loop is how this will eventually resolve.

## Next Phase Readiness
- Plan 104-03 (the operator-in-loop smoke re-run + tag push + Verified-publisher confirmation) is now safe to attempt against the tag-push job graph — `publish-crates` will not redden the run, and REL-01's own locked acceptance wording now matches the gate Phase 104 actually implements.
- The operator can run `pwsh -File scripts/azure/check-trusted-signing-root.ps1` at any time, for free, to check whether Microsoft's root-CTL propagation has resolved before burning a CI dispatch — currently ABSENT, so the external blocker (Microsoft's root-certificate propagation) remains open and is the sole remaining precondition for Plan 104-03's SC1 (smoke green).
- `scripts/verify-release-yml-publish-selectors.ps1` should be re-run any time `release.yml` is edited before Phase 105 lands, to catch any accidental re-introduction of the stale selectors outside the neutralized job.

## Self-Check: PASSED

All created/modified files confirmed present on disk; all 4 task commit hashes (`bd18e928`, `0ece6717`, `b77c6763`, `0370f43e`) confirmed present in `git log --oneline --all`.

---
*Phase: 104-smoke-green-cut-the-trusted-signed-release*
*Completed: 2026-07-03*
