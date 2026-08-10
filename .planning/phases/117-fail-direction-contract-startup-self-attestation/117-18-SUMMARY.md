---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 18
subsystem: testing
tags: [windows, ci, cargo-test, discovery-based-testing, layer-registry, CINT-03]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "layer_registry.rs (13-row LayerId registry), layer_registry_meta_test.rs's D-32 discovery scaffolding (Plan 12), the 7 pre-existing force-unavailable/negative unit tests (Plans 04/06/07), and the FirewallRulesEgress gate-level test added by sibling Plan 117-16"
provides:
  - "ALSO_AUTOMATED discovery list broadening every_registry_row_has_a_test to count 10/13 LayerId rows as tested (up from 2/13)"
  - "also_automated_entries_are_non_vacuous existence-check test, proven non-vacuous by a live counterfactual rename during this plan's own execution"
  - "contains_fn_exact word-boundary matcher, fixing a real substring false-positive bug discovered while proving non-vacuity"
  - "windows-layer-fault-injection CI job now builds nono-shell-broker and sets NONO_CI_HAS_WFP before running the fault-injection suites (NR-08)"
affects: [117-19, future-CINT-03-work]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Discovery-based coverage meta-test with a second, existence-checked allow-list (ALSO_AUTOMATED) alongside the loud-reason allow-list (MANUALLY_VERIFIED) — the second list is only trustworthy because a dedicated non-vacuity test reads every citation fresh from disk on every run"
    - "Word-boundary-exact source-text function-name search (contains_fn_exact) instead of str::contains, to avoid a renamed function silently matching its own former name as a prefix"

key-files:
  created: []
  modified:
    - crates/nono-cli/tests/layer_registry_meta_test.rs
    - .github/workflows/ci.yml

key-decisions:
  - "Broadened every_registry_row_has_a_test discovery via a new ALSO_AUTOMATED const rather than renaming the 8 existing test functions to the force_unavailable_<snake> convention — the plan's own investigation established these tests are a genuinely different shape (direct in-process unit/gate tests, not external-subprocess spawns), so forcing a naming convention onto them would have been cosmetic, not substantive."
  - "Added contains_fn_exact (word-boundary function-name match) instead of leaving the plan's specified str::contains check as-is, after live-reproducing a false-positive: renaming a cited function to a superstring of its old name (e.g. appending _RENAMED) still satisfied plain str::contains, which would have made the plan's own required non-vacuity proof silently pass instead of fail. This is a Rule 1 auto-fix — the bug was directly caused by (and discovered while proving) this task's own new mechanism."

requirements-completed: [CINT-03]

# Metrics
duration: 35min
completed: 2026-08-10
---

# Phase 117 Plan 18: SC3 Gap Closure — Broadened Discovery + CI Fix Summary

**Broadened `layer_registry_meta_test.rs`'s discovery mechanism from 2/13 to 10/13 automated `LayerId` rows via an existence-checked `ALSO_AUTOMATED` list, and fixed the `windows-layer-fault-injection` CI job to actually build its own test dependency.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-08-10 (session start)
- **Completed:** 2026-08-10
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `every_registry_row_has_a_test` now counts 10 of 13 `LayerId` rows as tested (2 pre-existing `force_unavailable_*` subprocess tests + 8 broadened-discovery `ALSO_AUTOMATED` entries), up from the 2/13 `117-VERIFICATION.md` found.
- `MANUALLY_VERIFIED` reduced to exactly the 3 rows that remain genuinely un-automatable on an ordinary host: `DaclSessionSidGrant`, `MinifilterAbsence`, `BrokerAuthenticodeTrustGate` — each keeps its existing, already-accurate reason string unchanged.
- New `also_automated_entries_are_non_vacuous` test existence-checks every `ALSO_AUTOMATED` citation's file+function fresh from disk on every run, closing the exact "second unchecked escape hatch" risk the plan's threat model (T-117-18-01) called out.
- Found and fixed a real bug in the citation-matching logic during the plan's own required non-vacuity counterfactual proof: plain `str::contains` false-positived when a renamed function's new name was a superstring of the old cited name. Added `contains_fn_exact`, a word-boundary matcher, and re-ran the counterfactual to confirm the fix actually catches the case.
- `windows-layer-fault-injection` CI job now runs `cargo build -p nono-shell-broker --release` before the fault-injection suites (closing NR-08 — broker-dependent tests previously `panic!`ed with a "pre-build" reminder rather than the CI job building it) and sets `NONO_CI_HAS_WFP: true`, matching the `windows-security` job's existing pattern.

## Task Commits

Each task was committed atomically:

1. **Task 1: ALSO_AUTOMATED list + broadened discovery in every_registry_row_has_a_test** - `16826a99` (test)
2. **Task 2: CI fix — build the broker + set NONO_CI_HAS_WFP (NR-08)** - `693dab46` (chore)

_Note: no additional plan-metadata commit hash beyond these two is included above; this SUMMARY's own commit is the plan-metadata commit._

## Files Created/Modified

- `crates/nono-cli/tests/layer_registry_meta_test.rs` - Added `ALSO_AUTOMATED` (8 entries), `also_automated_entries_are_non_vacuous` test, `contains_fn_exact` helper; broadened `every_registry_row_has_a_test`'s discovery fallback; reduced `MANUALLY_VERIFIED` to 3 entries.
- `.github/workflows/ci.yml` - Added a `cargo build -p nono-shell-broker --release` step and `NONO_CI_HAS_WFP: true` env to the `windows-layer-fault-injection` job only.

## Decisions Made

- Kept the plan's exact 8 `ALSO_AUTOMATED` triples (LayerId name, workspace-root-relative file path, function name) as specified in `<interfaces>` — verified each of the 8 cited functions exists in the current tree before wiring them in, since prior_work context flagged that citations elsewhere in this phase have drifted.
- See `key-decisions` above for the `contains_fn_exact` fix rationale.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed substring false-positive in citation matching**
- **Found during:** Task 1 (ALSO_AUTOMATED list + broadened discovery), while executing the plan's own mandated non-vacuity counterfactual proof
- **Issue:** The plan's `<interfaces>` reference shape used `src.contains(&format!("fn {fn_name}"))` to check a cited function exists. When the counterfactual proof (rename `firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate` to `..._RENAMED` without updating `ALSO_AUTOMATED`) was first run, `also_automated_entries_are_non_vacuous` and `every_registry_row_has_a_test` both incorrectly PASSED — because `"fn firewall_rules_..._gate_RENAMED"` still contains the substring `"fn firewall_rules_..._gate"` as a prefix. This would have made the plan's own required non-vacuity proof structurally unable to fail, defeating T-117-18-01's mitigation.
- **Fix:** Added `contains_fn_exact(src, fn_name)`, which requires the character immediately following the matched function name to be neither an identifier character nor absent-of-boundary (i.e., not alphanumeric/`_`), so `fn foo` matches `fn foo(` and `fn foo<T>` but not `fn foo2` or `fn foo_v2`. Replaced all three `.contains(&expected_fn)` call sites (the pre-existing `force_unavailable_*` check, the new `ALSO_AUTOMATED` fallback check, and `also_automated_entries_are_non_vacuous`) with this exact matcher for consistency.
- **Files modified:** `crates/nono-cli/tests/layer_registry_meta_test.rs`
- **Verification:** Re-ran the counterfactual (renamed the function again, confirmed both `every_registry_row_has_a_test` and `also_automated_entries_are_non_vacuous` FAILED with a message naming the missing function; reverted; confirmed all 6 tests pass again). `git status`/`git diff` confirmed the counterfactual edit and revert left `launch.rs` byte-identical to its committed state before Task 1's commit.
- **Committed in:** `16826a99` (part of Task 1's commit — the fix was made before committing, so no separate commit was needed)

---

**Total deviations:** 1 auto-fixed (1 bug fix, Rule 1)
**Impact on plan:** Necessary for the plan's own stated non-vacuity guarantee to be real rather than nominal. No scope creep — the fix is contained to the same file and same mechanism the plan specified.

## Issues Encountered

None beyond the deviation documented above.

## Non-Vacuity Proof (acceptance criterion evidence)

Per the plan's acceptance criteria, a live counterfactual was run twice (once before the `contains_fn_exact` fix, exposing the bug; once after, confirming the fix):

1. Renamed `firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate` to `firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate_RENAMED` in `crates/nono-cli/src/exec_strategy_windows/launch.rs`, WITHOUT updating `ALSO_AUTOMATED`.
2. Post-fix run: `also_automated_entries_are_non_vacuous` FAILED — `"FirewallRulesEgress: ...launch.rs does not contain \`fn firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate\` — the ALSO_AUTOMATED citation is stale (function renamed or removed)"`. `every_registry_row_has_a_test` also correctly failed, naming the same row.
3. Reverted the rename (`git checkout -- crates/nono-cli/src/exec_strategy_windows/launch.rs`, confirmed clean); re-ran the suite — all 6 tests pass.

## Verification Run Log

- `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` — 6/6 pass (`every_registry_row_has_a_test`, `host_gated_rows_are_loud`, `manual_verification_section_excludes_the_registry_table`, `security_assumptions_are_loud`, `pascal_to_snake_case_matches_expected_shapes`, `also_automated_entries_are_non_vacuous`).
- `cargo clippy -p nono-sandbox-cli --tests -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo fmt --all -- --check` — clean.
- `python -c "import yaml, ..."` YAML-parse assertion for the CI job — `OK` (broker-build step + `NONO_CI_HAS_WFP` both present, scoped to `windows-layer-fault-injection` only).
- `git diff --stat .github/workflows/ci.yml` — 12 insertions, 0 deletions, single job touched.
- `cargo build --workspace --all-targets` — clean.
- `cargo fmt --all -- --check` (workspace-wide) — clean.
- `cargo clippy --workspace --all-targets --features layer-fault-injection -- -D warnings -D clippy::unwrap_used` — clean.

**Cross-target clippy:** DOES NOT APPLY, as the plan states — `layer_registry_meta_test.rs` has no `#[cfg(target_os = "linux"|"macos")]` block (confirmed by reading the full file: its only `cfg` gate is `#![cfg(target_os = "windows")]`), and `.github/workflows/ci.yml` is not Rust source.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SC3's remaining gap after this plan: 3 of 13 `LayerId` rows (`DaclSessionSidGrant`, `MinifilterAbsence`, `BrokerAuthenticodeTrustGate`) still have no automated test, each with an accurate, already-loud, per-row reason recorded in `MANUALLY_VERIFIED` and cross-referenced in the SPEC's manual-verification section (enforced by `host_gated_rows_are_loud`, still green). This is the intended, plan-scoped remainder — the plan's objective explicitly targets 10/13, not 13/13.
- This plan did not touch NR3-01 (open BLOCKER, availability regression), NR3-02/NR3-03 (fail-open landmine risk), SC4's stale SPEC ledger rows, or NR3-08's citation drift — those remain open per `117-VERIFICATION.md` and are presumably addressed by sibling gap-closure plans 117-13 through 117-17 (already merged per `prior_work`) or later plans in this wave.
