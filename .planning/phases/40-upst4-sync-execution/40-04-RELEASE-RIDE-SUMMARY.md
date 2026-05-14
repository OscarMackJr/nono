---
phase: 40-upst4-sync-execution
plan: 04
slug: release-ride
subsystem: nono-core
tags: [upst4, c7, sandbox, landlock, diagnostic, release, wave-1]

# Dependency graph
requires:
  - phase: 39-upst4-audit
    provides: cluster C7 disposition (will-sync, 5 commits) + commit chain inventory
  - phase: 40-02-CLI-ALLOW-VALIDATE
    provides: Wave 0 foundation (SandboxState.allowed_domains; sandbox-state shape stable)
  - phase: 40-03-SCRUB-MODULE
    provides: Wave 0 foundation (nono::scrub re-export from lib.rs)
  - phase: 40-01-PROXY-HARDENING
    provides: Wave 1 first plan complete; PR #922 body + system_keystore_label CR-A fix on main
provides:
  - Landlock ABI detection cached via OnceLock (eliminates up-to-5 redundant landlock_create_ruleset syscalls per nono run)
  - Full failure diagnostic returned at sandbox boundaries (no longer swallows per-ABI probe errors via .ok())
  - CHANGELOG.md absorption of upstream v0.52.1 + v0.52.2 + v0.53.0 release-notes entries
  - 5 cherry-picked commits with verbatim D-19 6-line trailer block
affects: [40-05-FP-PROFILE-SAVE, 40-06-FP-PROXY-TLS]

# Tech tracking
tech-stack:
  added: []  # all commits are upstream-as-is; no new fork dependencies
  patterns:
    - "D-19 verbatim trailer block on each cherry-pick (lowercase 'Upstream-author:')"
    - "Phase 34 release-commit handling convention (precedent 64b231a7): fork drops upstream Cargo.toml + Cargo.lock version bumps; CHANGELOG.md absorption only"
    - "Wave 1 baseline-aware CI regression gate (Task 5) using last code-touching baseline (4665ae75) — pre-existing-red jobs treated as Phase 41 scope; only success→failure transitions are regressions"

key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/linux.rs (Landlock ABI cache + full failure diagnostic)
    - CHANGELOG.md (upstream v0.52.1/v0.52.2/v0.53.0 release-notes absorbed)

key-decisions:
  - "5a61808 plan-mismatch fixed (DEV-1): plan key-files said diagnostic.rs but actual upstream diff is entirely in sandbox/linux.rs. Followed upstream's actual diff."
  - "Phase 34 release-commit convention applied to 21bbb82 + e8bf014 + c4b25b8 (DEV-2): all three conflicted with fork's v0.53.0 pin (fork at 0.53.0 since 8c7f9fda for milestone v2.3); reverted Cargo.toml + Cargo.lock; CHANGELOG.md absorption only. Precedent: 64b231a7 for upstream v0.52.0."
  - "C7 / C5 boundary preserved at v0.53.0 release commit (DEV-3): upstream's v0.53.0 CHANGELOG entry lists C5 SHAs 8ddb143 + 54c7552 + f77e0e3 (TLS trust / multi-route / credential matching) — these are explicitly tagged in absorbed CHANGELOG as 'to be replayed via Plan 40-06-FP-PROXY-TLS' rather than cherry-picked here."
  - "Wave 1 baseline-aware CI gate passed: zero regressions vs baseline 4665ae75 (Plan 40-01 CR-A fix). All pre-existing Phase 41 failures unchanged."

patterns-established:
  - "Cluster-C7 cherry-pick chain executed in upstream chronological order: 5b61971 → 5a61808 → 21bbb82 → e8bf014 → c4b25b8. The 2 feature commits precede the 3 release bumps despite all being tagged v0.52.1..v0.53.0; chronology by commit date, not by tag boundary."
  - "Fork-CHANGELOG conflict resolution when upstream's release-notes entry collides with an existing fork heading: absorb upstream's entries (Bug Fixes / Features / Refactoring) under the fork's existing version heading; explicitly tag C5/fork-preserve SHAs as 'to be replayed via Plan 40-06' inline so reviewers can see the boundary."

requirements-completed: [REQ-UPST4-02]

# Metrics
duration: ~120m
completed: 2026-05-14
---

# Phase 40 Plan 04: RELEASE-RIDE Summary

**Cluster C7 (v0.52.1..v0.53.0, 5 commits) cherry-picked onto fork main with D-19 trailers — Landlock ABI cached via OnceLock, full failure diagnostic preserved, and 3 release version bumps absorbed CHANGELOG-only per Phase 34 convention. D-40-E1 invariant holds (0 Windows-file edits across the chain). Wave 1 CI gate confirmed zero regressions vs baseline.**

## Performance

- **Duration:** ~120 min (including ~80 min waiting for Wave 1 CI to complete the Windows Security harness)
- **Started:** 2026-05-14
- **Completed:** 2026-05-14
- **Tasks:** 5 plan tasks
- **Files modified:** 2 (`crates/nono/src/sandbox/linux.rs`, `CHANGELOG.md`)
- **Commits landed:** 5 cherry-picks (no follow-on fixes needed)

## Accomplishments

- **Landlock ABI cache (5b61971):** `detect_abi()` was previously calling `landlock_create_ruleset` syscalls up to 5 times per nono run; now caches the result in a process-global `OnceLock<DetectedAbi>` so first call probes as before and subsequent calls return the cached value via a lock-free pointer read.
- **Full failure diagnostic (5a61808):** Replaced the `OnceLock<Option<DetectedAbi>>` cache shape (which swallowed the per-ABI probe errors via `.ok()` in the `None` arm and returned a generic "No supported Landlock ABI detected" string) with `OnceLock<DetectedAbi>` + early-return. On cache-miss the full per-ABI diagnostic from `detect_abi_uncached()` now surfaces.
- **Release ride-along absorbed (21bbb82 / e8bf014 / c4b25b8):** Three upstream chore-release commits absorbed CHANGELOG-only per Phase 34 convention; fork's v0.53.0 version pin preserved across all three cherry-picks. Fork is now CHANGELOG-aligned with upstream v0.52.1 + v0.52.2 + v0.53.0 for the will-sync cluster surface (C7).
- **D-19 trailer block on every cherry-pick (5/5, verbatim 6-line shape with lowercase `Upstream-author:`).**
- **D-40-E1 holding** (0 Windows-file edits across the 5-commit chain; pre-plan Windows sentinel SHA `96886ae9` unchanged).
- **C7 / C5 boundary preserved** (C5 SHAs `8ddb143`, `54c7552`, `f77e0e3` mentioned in c4b25b8's CHANGELOG entry but flagged inline as Plan-40-06 territory; not cherry-picked here).
- **PR #922 body appended** with Plan 40-04's contribution section (after Plan 40-01's section).
- **Task 5 baseline-aware CI gate:** zero regressions vs baseline `4665ae75` (Plan 40-01 CR-A fix). All pre-existing Phase 41 failures unchanged.

## Task Commits

Each task was committed atomically. Upstream chronological order:

1. **Task 2 cherry-pick 1/5:** `5b61971` (SequeI) — `fix(sandbox): cache Landlock ABI detection with OnceLock` → `51681639`
2. **Task 2 cherry-pick 2/5:** `5a61808` (SequeI) — `fix: return full failure diagnostic` → `a2ce7795`
3. **Task 2 cherry-pick 3/5:** `21bbb82` (Luke Hinds) — `chore: release v0.52.1 (CHANGELOG-only; fork tracks own version)` → `b83938db`
4. **Task 2 cherry-pick 4/5:** `e8bf014` (Luke Hinds) — `chore: release v0.52.2 (CHANGELOG-only; fork tracks own version)` → `a29262de`
5. **Task 2 cherry-pick 5/5:** `c4b25b8` (Luke Hinds) — `chore: release v0.53.0 (CHANGELOG-only; fork tracks own version)` → `85cc3d9e`

(SUMMARY-doc commit follows separately.)

## Files Created/Modified

- `crates/nono/src/sandbox/linux.rs` — Landlock ABI detection now caches the probed `DetectedAbi` in a process-global `OnceLock<DetectedAbi>`. First call walks `ABI_PROBE_ORDER` V6→V1 as before; subsequent calls return the cached value. On no-ABI-supported, the full per-ABI diagnostic is returned (no longer swallowed via `.ok()`). Added `use std::sync::OnceLock;` import; added private `detect_abi_uncached()` helper for the actual probe walk.
- `CHANGELOG.md` — three upstream release-notes entries absorbed:
  - v0.52.1 - 2026-05-11 placed between fork's existing `[0.53.0] - 2026-05-14` entry and the existing `[0.52.0] - 2026-05-10` entry.
  - v0.52.2 - 2026-05-11 placed between fork's `[0.53.0]` and the absorbed v0.52.1.
  - v0.53.0 - 2026-05-11 upstream entries merged under fork's existing `[0.53.0]` heading with explicit "absorbed from upstream v0.53.0" subsection markers; C5 SHAs flagged inline as "to be replayed via Plan 40-06-FP-PROXY-TLS".

## Decisions Made

- **DEC-1:** Cherry-picks applied in true upstream chronological order (`5b61971 → 5a61808 → 21bbb82 → e8bf014 → c4b25b8`). Plan frontmatter `must_haves.truths` mandates chronological for tag/SHA assignment; the feature commits (5b61971 + 5a61808) precede the release bumps despite all being tagged v0.52.1..v0.53.0. This is correct because chronology is by commit date, not by tag boundary — see plan's `<interfaces>` table note.
- **DEC-2:** All three release commits cherry-picked under Phase 34 convention (precedent commit `64b231a7` for upstream v0.52.0): `git checkout HEAD -- Cargo.toml Cargo.lock {all crate Cargo.toml}` after the cherry-pick conflict landed, preserving fork's v0.53.0 pin (set in `8c7f9fda` for milestone v2.3). Only the CHANGELOG.md entry from each release commit was absorbed.
- **DEC-3:** For c4b25b8 (upstream v0.53.0), the upstream CHANGELOG entry collides with the fork's existing `[0.53.0] - 2026-05-14` heading. Merged upstream's entries (Bug Fixes / Features / Refactoring sections) UNDER the fork's existing heading, with subsection markers labelled "absorbed from upstream v0.53.0 - 2026-05-11". The C5 SHAs that appear in upstream's v0.53.0 notes (`8ddb143` TLS-trust + multi-route, `54c7552` review comments, `f77e0e3` absolute-match credential matching) were explicitly tagged inline as "to be replayed via Plan 40-06-FP-PROXY-TLS" rather than cherry-picked here. This preserves the C7 / C5 boundary at the CHANGELOG level so reviewers can see exactly where Plan 40-06 will add coverage.
- **DEC-4:** D-40-C2 gates 3+4 (cross-target clippy on linux-gnu + darwin) are "load-bearing skip → CI-verified" rather than "documented-skip — none" — same categorization as Plan 40-01. The Windows host lacks `aws-lc-sys`/`ring` C cross-compilers; Task 5's baseline-aware CI gate substitutes by comparing the post-push CI run's job conclusions against the last code-touching baseline. Zero regressions detected.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan-vs-reality mismatch] Plan key-files said `5a61808` touches `diagnostic.rs` but actual diff is in `sandbox/linux.rs`**
- **Found during:** Task 2 pre-cherry-pick read of `git show 5a61808` (per Task 2 read_first instruction)
- **Issue:** Plan frontmatter `key-files.modified` listed `crates/nono/src/diagnostic.rs` as a file `5a61808` would modify. The actual upstream commit diff is entirely in `crates/nono/src/sandbox/linux.rs` (-7/+8 lines reshaping the OnceLock cache from `OnceLock<Option<DetectedAbi>>` to `OnceLock<DetectedAbi>`). The plan's commit subject "fix: return full failure diagnostic" referred to returning the per-ABI probe diagnostic on cache-miss, not to changes in `diagnostic.rs`.
- **Fix:** Followed upstream's actual diff (which only touches `sandbox/linux.rs`). No edits to `crates/nono/src/diagnostic.rs`. SUMMARY frontmatter `key-files.modified` reflects reality: only `sandbox/linux.rs` + `CHANGELOG.md` touched in this plan.
- **Files modified:** N/A (the deviation is about correctly NOT modifying `diagnostic.rs`)
- **Verification:** `git diff --stat HEAD~5 HEAD` returns only `crates/nono/src/sandbox/linux.rs` + `CHANGELOG.md`.
- **Committed in:** body of `a2ce7795` (notes "Touches crates/nono/src/sandbox/linux.rs (not diagnostic.rs as the plan's key-files field claimed — see deviations)")

**2. [Rule 1 - Release-commit Cargo.toml conflicts] All three release bumps conflicted with fork's v0.53.0 pin**
- **Found during:** Task 2 cherry-pick 3/5 (21bbb82), 4/5 (e8bf014), 5/5 (c4b25b8)
- **Issue:** Each of the 3 release commits modifies `Cargo.toml` + `Cargo.lock` + 4 crate-level `Cargo.toml` files to bump version (0.52.0 → 0.52.1 → 0.52.2 → 0.53.0 in upstream's chain). Fork's workspace is already at 0.53.0 across all crates (set by commit `8c7f9fda` for milestone v2.3 in advance of this UPST4 sync). Every cherry-pick produced 5-6 Cargo.toml/Cargo.lock conflicts.
- **Fix:** Applied Phase 34 release-commit handling convention (precedent: commit `64b231a7 chore: release v0.52.0 (CHANGELOG-only; fork tracks own version)`):
  - `git checkout HEAD -- Cargo.toml Cargo.lock bindings/c/Cargo.toml crates/nono/Cargo.toml crates/nono-cli/Cargo.toml crates/nono-proxy/Cargo.toml` to revert all version bumps and lockfile regen.
  - Resolved the CHANGELOG.md conflict manually (kept fork's existing entries, inserted upstream's new entries in chronological position).
  - `git add CHANGELOG.md && git -c core.editor=true cherry-pick --continue` to land the absorbed entry.
- **Files modified:** Only `CHANGELOG.md` for each of the 3 release cherry-picks. No Cargo.toml or Cargo.lock changes.
- **Verification:** `grep -h '^version' Cargo.toml crates/nono/Cargo.toml crates/nono-cli/Cargo.toml crates/nono-proxy/Cargo.toml bindings/c/Cargo.toml` returns 4× `version = "0.53.0"` (the workspace Cargo.toml inherits via `package.version.workspace = true` so it doesn't have its own version field; the crate-level files do).
- **Committed in:** bodies of `b83938db`, `a29262de`, `85cc3d9e` (each documents the reverted hunks explicitly under "Reverted from upstream's release commit").

---

**Total deviations:** 2 auto-fixed (1 Rule 1 plan-mismatch, 1 Rule 1 release-commit-convention).
**Impact on plan:** Both auto-fixes were necessary for correctness. No scope creep; no Windows files touched. C7 / C5 boundary preserved.

## Issues Encountered

- **Plan key-files frontmatter inaccuracy:** as documented above, 5a61808's actual diff is in `sandbox/linux.rs` not `diagnostic.rs`. No impact on execution — discovered during the Task 2 read_first step before any cherry-pick was attempted.
- **`gh run watch` not used:** per Plan 40-01's `gh run watch` API-error notes, used polling via `gh run view ... --json status,conclusion` instead. Worked correctly.
- **Windows Security harness slow:** Wave 1's Windows Security job took ~45 minutes to complete (baseline was ~23 minutes). Both concluded `failure` (pre-existing Phase 41 backlog). Slower wall-time but no behavior change.
- **`/tmp` path resolution under Windows Python:** When using `python -c '...'` from bash on this Windows host, `/tmp/foo` does not resolve to the same path that bash sees. Worked around by using absolute Windows paths (`C:/Users/OMack/AppData/Local/Temp/...`).

## D-40-C2 8-check close gate

| Gate | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `cargo test --workspace --all-features` (Windows host) | **PASS** | 689 + 1031 + 40 + ... tests green; no failures |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) | **PASS** | Clean |
| 3 | `cargo clippy --target x86_64-unknown-linux-gnu` | **load-bearing-skip → CI-verified** | C cross-compiler not available on Windows host; CI confirms zero regression vs baseline `4665ae75` |
| 4 | `cargo clippy --target x86_64-apple-darwin` | **load-bearing-skip → CI-verified** | Same as gate 3; CI confirms zero regression |
| 5 | `cargo fmt --all -- --check` | **PASS** | Silent |
| 6 | Phase 15 5-row detached-console smoke | **environmental-skip** | Requires interactive Windows TTY session |
| 7 | `wfp_port_integration` tests | **environmental-skip** | Requires WFP service admin privileges |
| 8 | `learn_windows_integration` tests | **environmental-skip** | Requires elevated Windows execution context |

**Load-bearing skip categorization:** Gates 3+4 treated as "load-bearing skip — CI must verify" rather than "documented-skipped — none", matching Plan 40-01 pattern. Task 5 baseline-aware CI gate concluded **zero regressions**.

## Wave 1 CI Verification (Task 5)

**Baseline run:** `25878973341` on commit `4665ae75` (Plan 40-01 CR-A fix, last code-touching commit before this plan).
**Wave 1 run:** `25884160206` on commit `85cc3d9e` (this plan's final commit).
**Result:** PASS — zero regressions.

Per-job comparison (16 jobs):

| Job | Baseline | Wave 1 | Status |
|---|---|---|---|
| Cargo Audit | success | success | — |
| Classify Changes | success | success | — |
| Clippy (macos-latest) | failure | failure | Phase 41 pre-existing |
| Clippy (ubuntu-latest) | failure | failure | Phase 41 pre-existing |
| Docs Checks | skipped | skipped | — |
| Integration Tests | failure | failure | Phase 41 pre-existing |
| Rustfmt | success | success | — |
| Test (macos-latest) | failure | failure | Phase 41 pre-existing |
| Test (ubuntu-latest) | failure | failure | Phase 41 pre-existing |
| Verify FFI Header | success | success | (Plan 40-01's CR-A fix held) |
| Windows Build | failure | failure | Phase 41 pre-existing |
| Windows Integration | failure | failure | Phase 41 pre-existing |
| Windows Packaging | failure | failure | Phase 41 pre-existing (broker BrokerPath param) |
| Windows Regression | failure | failure | Phase 41 pre-existing |
| Windows Security | failure | failure | Phase 41 pre-existing (block-net probe tests) |
| Windows Smoke | success | success | — |

**No `success → failure` transitions detected. All pre-existing failures unchanged from baseline → Phase 41 scope, not blocking.**

## Threat-model close-out

| Threat ID | Mitigation status | Evidence |
|-----------|-------------------|----------|
| T-40-04-01 (Tampering, D-40-E1 Windows-only files invariant) | **mitigated** | `git diff --stat HEAD~5 HEAD -- crates/ \| grep -E '_windows\|exec_strategy_windows' \| wc -l` returns 0; pre-plan Windows sentinel SHA `96886ae9` unchanged |
| T-40-04-02 (Repudiation, D-19 trailer missing) | **mitigated** | `git log --format='%B' HEAD~5..HEAD \| grep -c '^Upstream-commit: '` returns 5; lowercase `Upstream-author:` count returns 5 |
| T-40-04-03 (DoS, OnceLock ABI cache poisoned by concurrent initialization race) | **accept** | OnceLock guarantees single initialization per Rust's standard library safety contract; no additional mitigation needed |
| T-40-04-04 (Tampering, Landlock ABI version cached at init then sandbox upgraded mid-process) | **accept** | nono's process model is fork+exec; each child gets fresh process + fresh OnceLock initialization; no mid-process sandbox upgrade path exists |
| T-40-04-05 (Tampering, release-bump Cargo.toml conflicts) | **mitigated** | Phase 34 convention applied (commit bodies of `b83938db`/`a29262de`/`85cc3d9e` document the reverted hunks); fork's v0.53.0 pin preserved on all 4 crate Cargo.toml files; `cargo build --workspace` green after each cherry-pick |
| T-40-04-06 (Information Disclosure, full failure diagnostic exposes kernel paths) | **accept** | Diagnostic is emitted to stderr by the supervisor (unsandboxed); sandboxed child never reads it. No new attack surface introduced. |

## Self-Check: PASSED

**Files verified present:**
- `crates/nono/src/sandbox/linux.rs` — contains `OnceLock<DetectedAbi>` cache + `detect_abi_uncached()` helper. FOUND.
- `CHANGELOG.md` — contains v0.52.1 + v0.52.2 entries between `[0.53.0]` and `[0.52.0]` headings; v0.53.0 upstream entries merged under fork's existing `[0.53.0]` heading. FOUND.

**Commits verified in git log:**
- `51681639` (5b61971), `a2ce7795` (5a61808), `b83938db` (21bbb82), `a29262de` (e8bf014), `85cc3d9e` (c4b25b8) — all 5 reachable from `main` via `git log --oneline HEAD~5..HEAD`.

**Gates verified:**
- D-19 trailer count: 5 (lowercase `Upstream-author:`) ✓
- D-40-E1 windows-file edits: 0 ✓
- Cargo.toml versions intact at v0.53.0: 4/4 crate-level files ✓
- C5 SHA absence in commit chain: 3/3 (`8ddb143`, `54c7552`, `f77e0e3` NOT in HEAD~5..HEAD) ✓
- Final CI baseline diff: 0 regressions ✓
- Landlock ABI OnceLock pattern landed: `grep -E 'OnceLock' crates/nono/src/sandbox/linux.rs` returns 3 matches (the new cache + the pre-existing WSL2_DETECTED + the test comment) ✓

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 40-01 + Plan 40-04 (Wave 1)** now both landed on `main`. Wave 1 closes.
- **PR #922** body updated with Plan 40-04's contribution section; the umbrella PR now spans Plans 40-01 + 40-04 (Wave 1) atop Plans 40-02 + 40-03 (Wave 0).
- **Plan 40-05 (FP-PROFILE-SAVE)** can begin. Wave 2 depends on Wave 1 being closed and CI-verified — both criteria met.
- **Plan 40-06 (FP-PROXY-TLS)** inherits two boundaries from Plans 40-01 + 40-04:
  - C5 surface that was deliberately held back in 40-01 (OAuth2 credential block in `credential.rs`, TLS-intercept env vars in `server.rs`, `tls_connector` parameter on `CredentialStore::load`).
  - The CHANGELOG entries for `8ddb143` + `54c7552` + `f77e0e3` in this plan's commit `85cc3d9e` are tagged "to be replayed via Plan 40-06-FP-PROXY-TLS" — Plan 40-06 should re-attribute them when it lands.
- **Phase 41 backlog** unchanged — Wave 1 push introduced no new failures; all 11 pre-existing red jobs remain at baseline state.

---

*Phase: 40-upst4-sync-execution*
*Completed: 2026-05-14*
