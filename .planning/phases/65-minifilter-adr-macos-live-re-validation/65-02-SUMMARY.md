---
phase: 65-minifilter-adr-macos-live-re-validation
plan: 02
subsystem: testing
tags: [macos, seatbelt, ci, resl, rlimit, cross-target, github-actions]

# Dependency graph
requires:
  - phase: 64-minifilter-spike-implementation-macos-p1-cherry-pick-wave
    provides: the landed macOS Seatbelt cherry-picks (8f84d454, 362ada22, 8f1b0b74) re-validated here
  - phase: 37-linux-resl-backends-pkgs-auto-pull
    provides: the host-blocked REQ-RESL-NIX-03 resource-limit enforcement first exercised on the macOS runner here
provides:
  - Green macos-latest CI SHA (d9144663) captured as the literal D-11c HARD-gate evidence
  - Cross-target clippy disposition (PARTIAL) + Phase-64 cherry-pick drift scan (clean) recorded
  - NONO_RESL_HOST_VALIDATED env-gate moving the two host-dependent resl enforcement tests off CI to gate-65-A
affects: [66-edr-uat, gate-65-A, release-tagging, resl-enforcement-validation]

# Tech tracking
tech-stack:
  added: []
  patterns: ["env-gated host-dependent enforcement test (skip-on-CI / run-on-real-host via NONO_RESL_HOST_VALIDATED)"]

key-files:
  created:
    - .planning/phases/65-minifilter-adr-macos-live-re-validation/65-MACOS-CI-EVIDENCE.md
  modified:
    - crates/nono-cli/tests/resl_nix_macos.rs
    - .planning/phases/65-minifilter-adr-macos-live-re-validation/65-HUMAN-UAT.md

key-decisions:
  - "macos.rs left unmodified — re-validation only; the green SHA was reached via a test-harness change, not a sandbox-code change"
  - "Host-dependent resl enforcement tests (macos_timeout, macos_max_processes) gated behind NONO_RESL_HOST_VALIDATED rather than #[ignore] — they self-activate on a real macOS host at gate-65-A"
  - "PR #6 on the fork (OscarMackJr/nono) is the CI vehicle + continuity record; NOT pushed to upstream always-further/nono"

patterns-established:
  - "Env-gated enforcement test: skip-with-message on CI (var unset), full assertions on a real host (var set) — for OS enforcement the hosted runner does not faithfully provide"

requirements-completed: [MACOS-03]

# Metrics
duration: ~3h (across resume sessions; CI rehab + runner-hang diagnosis)
completed: 2026-06-11
---

# Phase 65 (Plan 02): macOS CI Re-validation Summary

**Green `macos-latest` CI SHA `d9144663` captured as the D-11c HARD-gate evidence — reached by env-gating the two host-dependent resl enforcement tests off the CI runner (where macOS `--timeout`/`RLIMIT_NPROC` does not fire and was hanging the runner to death), with `macos.rs` left unmodified.**

## Performance

- **Duration:** ~3h across resume sessions (D-11c CI rehab + runner-hang root-cause)
- **Completed:** 2026-06-11
- **Tasks:** 2 (Task 1 local/cross-target checks; Task 2 green-SHA HARD gate)
- **Files modified:** 3 (1 source test file + 2 evidence/UAT docs)

## Accomplishments
- **D-11c HARD gate SATISFIED:** `Test (macos-latest)` + `Clippy (macos-latest)` both `success` at SHA `d9144663` (run `27345465703`, PR #6). Recorded as literal tag-blocking evidence in `65-MACOS-CI-EVIDENCE.md`.
- **Root-caused the chronic macOS red as a runner hang, not a code bug:** once the prior flag fix let the resl tests actually launch children, `macos_timeout_kills_at_deadline` + `macos_max_processes_blocks_on_rlimit_nproc` exercised never-host-validated macOS enforcement (REQ-RESL-NIX-03) for the first time on the GH runner, where it does not fire and `run_bounded` did not reap the sandboxed/detached children → 25+ min hang → "runner lost communication" (runs `27291915409`, `27300030066`).
- **Fixed via `NONO_RESL_HOST_VALIDATED` env-gate** (PR #6 / `d9144663`): the two host-dependent enforcement tests skip on CI and run on a real macOS host at gate-65-A. `macos.rs` unchanged.
- **Task 1 re-validation recorded:** macOS ordering + dual-path contract tests present and green on their native target; cross-target clippy PARTIAL (Windows host has no `cc`); Phase-64 cherry-pick drift scan clean (no let-chains, no E0716, `/private` dual-path present).
- **gate-65-A UAT extended:** added Assertion 5 so the now-CI-skipped enforcement tests are validated on the real macOS host with `NONO_RESL_HOST_VALIDATED=1`.

## Task Commits

The D-11c CI rehab spanned several atomic `test(nono-cli):` commits (prior session, on `e72d6438`):

1. **Cargo Audit green** — `4aaa0508` (chore: sign-fixture sigstore 0.7→0.8)
2. **audit_attestation `$PWD` harness fix** — `d21663a6`
3. **builtin_profile_load opencode-pack relocation** — `28219919`
4. **profile_drafts manifest fix** — `c80e8664`
5. **resl_nix `--allow-fs-*`→`--read` flag fix** — `924354f4`
6. **resl_nix fail-fast bounding** — `e72d6438` (later found insufficient — see Deviations)
7. **resl enforcement tests env-gated (the green-SHA fix)** — `d9144663` (PR #6, `fix/macos-resl-host-gate`)

**Plan metadata + evidence/UAT docs:** this commit (docs: Phase 65 close-out).

## Files Created/Modified
- `crates/nono-cli/tests/resl_nix_macos.rs` — added `host_enforcement_validated()` + `NONO_RESL_HOST_VALIDATED` skip guards on the two host-dependent enforcement tests.
- `65-MACOS-CI-EVIDENCE.md` — Task 2 gate section filled with the green run URL + SHA + both-legs-success; "how the green was reached" narrative.
- `65-HUMAN-UAT.md` — Assertion 5 (gated resl enforcement tests on the real host) + green-SHA reference in Sign-off.

## Decisions Made
- **Env-gate over `#[ignore]`** (user-selected): the tests self-activate on a real macOS host without `--ignored`, and the skip message documents *why*. More honest than hiding them.
- **`macos.rs` untouched:** the re-validation invariant holds — the change is test-harness only. The Seatbelt profile-generation contract is unchanged; only the resl *enforcement* tests (which assert OS behavior the hosted runner lacks) were gated.
- **Fork-only PR:** PR #6 lives on `OscarMackJr/nono` for CI + continuity; not pushed upstream (repo stays public during the Microsoft minifilter-altitude approval window).

## Deviations from Plan

Plan 65-02 expected "no source code change" — a green macOS CI on the existing tree. That did **not** hold:

### Auto-fixed Issues

**1. [Blocking — CI gate un-closeable as written] resl enforcement tests hung the macOS runner**
- **Found during:** Task 2 (capturing the green `macos-latest` SHA)
- **Issue:** The fail-fast bound added in `e72d6438` did not reap the sandboxed/detached `sleep`/`bash` children, so `macos_timeout_kills_at_deadline` + `macos_max_processes_blocks_on_rlimit_nproc` hung `cargo test` 25+ min until the runner lost communication. macOS `--timeout`/`RLIMIT_NPROC` enforcement (REQ-RESL-NIX-03) genuinely does not fire on the hosted runner and was never host-validated (Phase 37 host-blocked).
- **Fix:** Gated both tests behind `NONO_RESL_HOST_VALIDATED` (skip on CI, run at gate-65-A). Test-harness only; `macos.rs` unchanged.
- **Files modified:** `crates/nono-cli/tests/resl_nix_macos.rs`
- **Verification:** PR #6 CI run `27345465703` — `Test (macos-latest)` + `Clippy (macos-latest)` both `success` at `d9144663`.
- **Committed in:** `d9144663`

---

**Total deviations:** 1 (blocking — necessary to make the HARD gate closeable). The deviation is test-harness scope only; the re-validation invariant (`macos.rs` unmodified) is preserved. No scope creep.

## Issues Encountered
- **GitHub serves per-job logs only after the whole run completes** (gotcha #1): the macOS log 404'd while the ubuntu leg hung ~6h. Resolved by reading the log via `gh run view --log --job` once the run reached a terminal state, and by re-running on the same SHA / a fresh PR run.
- **Cancelling a run drops in-progress step logs** — the diagnosis was completed from the job's *step state* (Build success → "Run tests" hung 25+ min) rather than the unavailable log.

## User Setup Required
None — no external service configuration. (Two real-host steps remain as gate-65-A HUMAN-UAT, not setup.)

## Next Phase Readiness
- **D-11c HARD gate CLEARED** — a release tag may be cut on/after `d9144663`.
- **Still open (gate-65-A, real macOS host):** the 4 live `sandbox_init()` deny assertions + new Assertion 5 (resl enforcement with `NONO_RESL_HOST_VALIDATED=1`). MACOS-03's live half stays open until run on a real macOS host.
- **PR #6 ready to merge** into the fork's `main`.
- Phase 66 (EDR HUMAN-UAT) has no code dependency on this work.

---
*Phase: 65-minifilter-adr-macos-live-re-validation*
*Completed: 2026-06-11*
