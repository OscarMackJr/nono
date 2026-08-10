---
phase: 117-fail-direction-contract-startup-self-attestation
verified: 2026-08-10T00:00:00Z
status: gaps_found
score: 1/4 must-haves verified (SC1 partial, SC2 partial, SC3 failed, SC4 partial)
overrides_applied: 0
gaps:
  - truth: "SC3 — every contract entry has a test that forces that layer unavailable and asserts the contracted outcome"
    status: failed
    reason: >
      Only 2 of the 13 `LayerId` rows (`MandatoryIntegrityLabel`, `DaclPackageSidGrant`) have an
      automated `force_unavailable_*` test in `crates/nono-cli/tests/layer_force_unavailable.rs`.
      The other 11 rows are entries in `MANUALLY_VERIFIED` — written justifications enforced-loud
      by `host_gated_rows_are_loud`, but not tests that force the layer unavailable and assert the
      contracted outcome. SC3's own wording, reproduced verbatim in CINT-03 and in the SPEC itself
      ("a contract entry with no such test is not satisfied"), means 11/13 rows are explicitly not
      satisfied by the phase's own stated bar.
    artifacts:
      - path: "crates/nono-cli/tests/layer_force_unavailable.rs"
        issue: "Contains exactly 2 `fn force_unavailable_*` test functions (verified by grep: lines 160, 189) against 13 registry rows."
      - path: "crates/nono-cli/tests/layer_registry_meta_test.rs"
        issue: "MANUALLY_VERIFIED const lists 11 of 13 LayerId names (lines 151-273) — loud, but not a test."
    missing:
      - "Automated forced-unavailable tests (or a materially larger share of them) for the remaining 11 rows, or an explicit operator-accepted downgrade of SC3's bar for host-gated rows via an override entry per row."
  - truth: "SC2 — there is no path on which nono presents a confinement guarantee it did not confirm"
    status: failed
    reason: >
      True for the row iteration 3 traced end-to-end (MandatoryIntegrityLabel / CR-14: a
      zero-ACE launch now hard-aborts, confirmed live by `labels_guard::tests::coverage_distinguishes_full_partial_and_zero_ace_launches`
      and `guard_skips_path_not_owned_by_current_user`, both passing on this host). Not
      universally true across the registry: (1) `dacl_ancestor_traverse` and
      `dacl_ancestor_read_attrs` derive `LayerApplication` from `.is_some()` on an
      unconditionally-`Some` field (`mod.rs:413-414`), so both rows are the constant `Applied`
      and structurally cannot report a negative on a shipped `DirectCli` launch (NR3-02). (2)
      `DaclSessionSidGrant`'s row is hardcoded `NotApplicable` in `AppliedLayers::status()`
      while its doc comment cites a guard test — `dacl_session_sid_grant_is_not_claimed_anywhere`
      — that does not exist anywhere in the tree (verified: `grep -rn` returns one hit, the
      comment itself). If the row's expectancy is ever restored (an explicitly open operator
      decision, RF-03), the row would silently drop out of every decision rather than aborting —
      fail-open, unguarded (NR3-03). (3) A newly introduced availability defect (NR3-01, open
      BLOCKER, unfixed as of the latest commit `ff4fc7af`) means nono's own abnormal-exit residue
      — or a second concurrent session over the same workspace — now hard-aborts every subsequent
      launch with no remediation arm on `NonoError::LayerAttestationFailed` (confirmed: grep of
      `remediation()` shows no `LayerAttestationFailed` arm, falls to `_ => None`), no sweep, and
      no CLI affordance. This is an availability regression, not a false-positive confinement
      claim, but it is a direct, unresolved consequence of the CR-14 fix this criterion credits.
    artifacts:
      - path: "crates/nono-cli/src/exec_strategy_windows/mod.rs"
        issue: "Lines 413-414: dacl_ancestor_traverse / dacl_ancestor_read_attrs derive Applied from Option::is_some() on a field that is unconditionally Some (execution_runtime.rs:608 sets package_sid: Some(..) unconditionally) — can never report a negative."
      - path: "crates/nono-cli/src/exec_strategy_windows/layer_registry.rs"
        issue: "Line 686 doc comment cites dacl_session_sid_grant_is_not_claimed_anywhere as build-failing; that test does not exist anywhere in crates/ (verified by grep)."
      - path: "crates/nono-cli/src/exec_strategy_windows/labels_guard.rs"
        issue: "Lines 106-118: applied == 0 is a hard NotApplied -> Abort, but nono's own SkipPreExistingLabel residue from an abnormal exit (Drop does not run on Ctrl-C/kill/crash) reproduces applied == 0 on the NEXT launch of the same workspace, and on the second of two concurrent sessions."
      - path: "crates/nono/src/error.rs"
        issue: "No remediation() arm for LayerAttestationFailed (verified: falls through to catch-all _ => None at line 500) — operator has no CLI-surfaced recovery path for the NR3-01 lockout."
    missing:
      - "A coverage accessor for AppliedAncestorTraverseGuard / AppliedAncestorReadAttributesGuard that reads guard effect rather than Option::is_some() (NR3-02 fix)."
      - "The dacl_session_sid_grant_is_not_claimed_anywhere test the code comment already promises, or removal of the false citation (NR3-03)."
      - "Self-healing recovery for nono's own stale-label residue plus a remediation() arm for LayerAttestationFailed (NR3-01 — the one open BLOCKER from iteration 3, unfixed as of HEAD)."
  - truth: "SC4 — where the contract and the code disagree, the discrepancy is recorded rather than quietly reconciled"
    status: failed
    reason: >
      Well executed for the iteration-1-era discrepancies (SC4-1 through SC4-5, RF-01 through
      RF-13 all present in proj/SPEC-windows-fail-direction-contract.md's ledger with re-runnable
      grep evidence). Not extended to the iteration-2 fix pass: verified directly that the SPEC's
      "Contract vs. code discrepancies" and "Review-fix pass" tables contain rows for RF-15 (CR-14)
      and RF-16 (NR-02) only — no row exists for NR-04, NR-05, or NR-06. Worse, the existing RF-14
      row's own text, read verbatim from the SPEC, still describes the guard as it existed BEFORE
      NR-06's fix ("refuses any path that is not a strict subdirectory of `%TEMP%/nono-net-block`,
      compared by path components") when the shipped code now has three ordered defenses
      (traversal-component rejection, the original component test, and canonicalize-both-sides).
      This is a stale contract claim about shipped behavior that nothing catches — `spec_matches_registry`
      only checks LayerId names, not prose accuracy — which is the "quietly reconciled" failure
      mode SC4 exists to prevent, now reproduced one review cycle later.
    artifacts:
      - path: "proj/SPEC-windows-fail-direction-contract.md"
        issue: "Lines 240-266 (Review-fix pass table): no row for NR-04/NR-05/NR-06; RF-14 row's description is the pre-NR-06 single-defense guard, not the shipped three-defense guard."
      - path: "crates/nono-cli/src/exec_strategy_windows/layer_registry.rs"
        issue: "Registry call-site line citations (mod.rs, dacl_guard.rs, labels_guard.rs) drifted a further ~145 lines in the iteration-2 fix pass and were not re-run (NR3-08, third consecutive pass this has been deferred)."
    missing:
      - "SPEC rows for NR-04, NR-05, NR-06 documenting what changed and their honest-limits caveats (both already written up in 117-REVIEW-FIX.md; porting them into the SPEC's standing discrepancy ledger is the missing step)."
      - "RF-14's description updated to match the shipped three-defense staging guard."
      - "Registry call-site citations re-run against current line numbers, or converted to symbol citations as NR3-08 recommends."
human_verification:
  - test: "Run the full nono-sandbox-cli bin test suite (--features layer-fault-injection, --test-threads=1) including exec_strategy:: on an elevated/real-console Windows host, and confirm the pre-existing-failure count is still 11, not more."
    expected: "Same 11 known pre-existing Windows failures (documented in project memory nono_cli_windows_baseline_test_failures.md); no new regressions from this phase's changes."
    why_human: "This verification session hit a hang in exec_strategy::dacl_guard::tests::ancestor_read_attributes_multi_target_covers_each_chain_and_stops_at_root when running the full labels_guard::/dacl_guard::/attestation:: slice together on this non-elevated dev host (isolated labels_guard:: subset ran clean, 7/7 pass). This matches known host/harness fragility documented in REVIEW-FIX.md (RestrictedToken/JobObjectContainment teardown stalls) rather than a code defect, but a full run on a clean host is needed to confirm the failure count has not grown."
  - test: "Exercise the WfpEgressFilters and DaclSessionSidGrant/DaclPackageSidGrant manual-verification rows per the steps written in layer_registry_meta_test.rs's MANUALLY_VERIFIED entries (elevated nono-wfp-service + non-elevated daemon session; real PowerShell console for the broker arm)."
    expected: "Each manually-verified row produces the contracted Abort outcome when forced unavailable, matching the written manual-verification steps."
    why_human: "These 11 rows require elevated services, real (non-git-bash) console sessions, or signed production installs that this verification pass could not stand up. They are the exact rows SC3 counts as unsatisfied without an automated test — human execution of the documented manual steps is the only way to close that gap today, and doing so does not change the SC3 scoring (a manual pass is still not 'a test' per CINT-03's own wording), but it is needed before relying on those rows operationally."
---

# Phase 117: Fail-Direction Contract + Startup Self-Attestation Verification Report

**Phase Goal:** The composite's fail-direction stops being decided per-layer-in-isolation and
becomes one system-level answer — and nono can no longer report "enforcing" while a layer is
silently inert.

**Verified:** 2026-08-10
**Status:** gaps_found
**Re-verification:** No — initial verification (this is the first VERIFICATION.md for phase 117)

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|---|---|---|
| SC1 | One document names every layer, states behaviour on failure, citing the enforcing call site | ⚠ PARTIAL | `layer_registry.rs` (13-row `LayerId` enum, code-resident, compiles clean) + `proj/SPEC-windows-fail-direction-contract.md` (313 lines) both exist and are substantive — every layer named, every row has an outcome. But call-site citations have drifted ~145 lines in the current tree (NR3-08, confirmed unfixed) and the SPEC's own review-fix ledger is stale for 3 of the 5 iteration-2 fixes (see SC4). "Citing the enforcing call site" is degraded, not absent. |
| SC2 | Forcing a layer unavailable produces abort or visible downgrade; no path presents an unconfirmed guarantee | ✗ FAILED | Resolved and test-verified for the row iteration 3 traced end-to-end (`MandatoryIntegrityLabel`/CR-14 — confirmed live by re-running `labels_guard::` tests, 7/7 pass). NOT universal: two DACL-ancestor rows are a constant `Applied` and cannot deny (NR3-02, confirmed by source read); `DaclSessionSidGrant`'s row is fail-open if its expectancy is ever restored, and the guard test its own comment cites does not exist (NR3-03, confirmed by grep — zero hits besides the comment); and the CR-14 fix itself introduced an unresolved availability BLOCKER (NR3-01, open, confirmed unfixed as of `ff4fc7af`) with no remediation arm (confirmed: `NonoError::remediation()` has no `LayerAttestationFailed` arm). |
| SC3 | Every contract entry has a test forcing that layer unavailable, asserting the contracted outcome | ✗ FAILED | Confirmed by direct source count: 2 of 13 `LayerId` rows have an automated `force_unavailable_*` test (`crates/nono-cli/tests/layer_force_unavailable.rs`, functions at lines 160 and 189). The remaining 11 are `MANUALLY_VERIFIED` entries — loud, written, and mechanically enforced to stay loud, but not tests. Per SC3's own wording ("a contract row without a test is not counted as satisfied"), 11/13 = 85% of rows are not satisfied. |
| SC4 | Contract/code disagreements are fixed or recorded in-phase, never quietly reconciled | ✗ FAILED | Well executed for the iteration-1-era discrepancies (SC4-1..SC4-5, RF-01..RF-13, all present with re-runnable grep evidence — verified present in the SPEC). Not extended to iteration-2 (NR-04/NR-05/NR-06 have no SPEC row) and RF-14's existing row description is now stale — it describes the pre-NR-06 single-defense guard while the shipped code has three ordered defenses (confirmed by reading both the SPEC text and `network.rs`'s current `cleanup_network_enforcement_staging`). This is the exact "quietly reconciled" failure mode SC4 exists to prevent, reproduced one cycle later and not yet caught by any test (`spec_matches_registry` only checks `LayerId` names). |

**Score:** 0/4 truths fully verified (SC1 partial-with-real-defects, SC2/SC3/SC4 failed against their own literal wording).

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` | Code-resident 13-layer registry, source of truth (D-01) | ✓ VERIFIED (exists, substantive, wired) | 1274 lines; compiles clean (`cargo check -p nono-sandbox-cli --all-targets --features layer-fault-injection` succeeded); `LayerId` enum has exactly 13 variants; consumed by `attestation.rs`, `mod.rs`, both meta-test files. |
| `proj/SPEC-windows-fail-direction-contract.md` | Human-readable, drift-checked contract | ⚠ ORPHANED CONTENT (stale sections) | 313 lines, tracked despite `proj/` gitignore per repo convention; `spec_matches_registry` test passes (names only); prose for RF-14 and the missing NR-04/05/06 rows is stale (see SC4). |
| `crates/nono-cli/tests/layer_force_unavailable.rs` | Per-layer forced-unavailable tests (CINT-03) | ✗ STUB relative to CINT-03's bar | File exists, both its 2 tests pass on this host, but covers 2/13 rows — the file's own scope is far short of "every registry row" (see SC3). |
| `crates/nono-cli/tests/layer_registry_meta_test.rs` | D-32 discovery-based meta-test | ✓ VERIFIED, but its own scope is honest-not-satisfying | 5/5 tests pass on this host (`every_registry_row_has_a_test`, `host_gated_rows_are_loud`, etc.) — it correctly enforces that every row has EITHER a test or a loud manual-verification entry, which is a weaker bar than CINT-03's literal wording. The test suite is internally consistent; the bar it enforces is not the bar the roadmap states. |
| `crates/nono-cli/src/exec_strategy_windows/attestation.rs` | Startup self-attestation gate (CINT-02) | ✓ VERIFIED core logic, ⚠ gaps in coverage | `classify_row`/`decide_from_entries`/`RowVerdict::from_application` all present, wired, and covered by passing unit tests (`attestation::tests::*`, confirmed passing on this host). Structural gaps (NR3-02, NR3-03) are in the DATA the gate consumes, not in the gate's decision logic itself. |
| `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` | CR-14 fix: report guard effect not construction | ✓ VERIFIED | `LabelCoverage::application()` (lines 106-118) confirmed present and matches the described 4-state contract; `coverage_distinguishes_full_partial_and_zero_ace_launches` and `guard_skips_path_not_owned_by_current_user` both pass live on this host. |
| `crates/nono/src/error.rs` | `LayerAttestationFailed` + remediation | ⚠ HOLLOW | `LayerAttestationFailed` variant exists and is used (`:388`, `:458`, `:736`), but `remediation()` has no arm for it — confirmed by reading the full match arm list, falls to `_ => None`. Operator gets a diagnostic but no recovery path, which is the direct mechanism behind the open NR3-01 blocker. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `AppliedLabelsGuard::snapshot_and_apply` | `LayerApplication` (registry) | `LabelCoverage::application()` | ✓ WIRED, confirmed correct | Traced the full chain per the review and re-confirmed by running the tests live: `AppliedLabel::{Skip*, Applied}` → `coverage()` → `application()` → `AppliedLayers::mandatory_integrity_label` → `classify_row` → `decide_from_entries` → `Abort` on zero coverage. |
| `mod.rs::applied_layers()` (dacl_ancestor_traverse/read_attrs) | `LayerApplication` | `Option::is_some()` | ✗ NOT_WIRED to real guard effect | Confirmed by direct source read (`mod.rs:413-414`): derives from an `Option` that is unconditionally `Some` (`execution_runtime.rs:608`), not from the guard's own coverage accessor — the same shape CR-14 fixed for the other two fields, un-fixed here. |
| `layer_registry.rs` `DaclSessionSidGrant` row | `AppliedLayers::status()` | hardcoded match arm | ✗ FAIL-OPEN landmine, confirmed | `status()` hardcodes `NotApplicable` for this row regardless of expectancy; the cited guard test (`dacl_session_sid_grant_is_not_claimed_anywhere`) does not exist (`grep -rn` returns exactly the doc-comment hit, zero test hits). |
| `daemon_attest_and_decide` | `DaemonAttestationDecision::ProceedDowngraded` | constructor call | ✗ NEVER CONSTRUCTED, confirmed | `grep -n "ProceedDowngraded" agent_daemon/launch.rs` shows the variant declared (`:1304`) and matched in a dead arm (`:915`) but no production call site constructs it — daemon path is abort-or-proceed only, divergent from the CLI's three-state model (NR3-05, unfixed). |
| D-27 downgrade banner | operator-visible diagnostic detail | `tracing::warn!` | ✗ NOT WIRED on success path, confirmed | Read `launch.rs:1480-1524` directly: the `downgraded_layers=` field is only logged inside the `None`/`Err` arms of audit emission (failure paths), never unconditionally. The banner text ("see diagnostic output for details") points at a channel that carries nothing on the success path (NR3-04, unfixed). |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `AppliedLayers::mandatory_integrity_label` | `LabelCoverage` | `AppliedLabelsGuard::snapshot_and_apply` against real filesystem ACEs | Yes | ✓ FLOWING (confirmed by live test run) |
| `AppliedLayers::dacl_ancestor_traverse` / `dacl_ancestor_read_attrs` | `Option<Vec<PathBuf>>.is_some()` | `execution_runtime.rs:608`, unconditional `Some` | No — constant, not conditioned on the walk's actual outcome | ✗ STATIC (NR3-02) |
| `NetworkEnforcementGuard::WfpServiceManaged.installed_filter_count` | `u32` | `network.rs`, elevated service IPC report | Yes, but gated earlier by `assert_wfp_activation_installed_filters` so the negative value is production-unreachable | ⚠ FLOWING BUT UNREACHABLE NEGATIVE (NR-04 partial, confirmed via review + code read) |
| Daemon `dacl_guard_applied` / `wfp_filters_installed` | `bool` | `DaemonDaclGuard::granted_write_access()`, `wfp_filter_add` result | Yes, but both remain dynamically identical to the gate condition on every reachable path because earlier gates already fail-closed | ⚠ FLOWING BUT UNREACHABLE NEGATIVE (NR-05 partial, confirmed via review) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Registry + meta-test compile and pass | `cargo check -p nono-sandbox-cli --all-targets --features layer-fault-injection` | `Finished dev profile`, clean | ✓ PASS |
| D-32 discovery gate + drift check run and pass | `cargo test -p nono-sandbox-cli --test layer_registry_meta_test --test layer_registry_selfcheck -- --test-threads=1` | 5/5 + 3/3 pass | ✓ PASS |
| CR-14 fix behaves as claimed (zero-coverage aborts) | `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection -- --test-threads=1 labels_guard::` | 7/7 pass | ✓ PASS |
| Full `exec_strategy::` unit slice (labels_guard/dacl_guard/attestation together) | `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection -- --test-threads=1 labels_guard:: dacl_guard:: attestation::` | Hung/timed out inside `dacl_guard::tests::ancestor_read_attributes_multi_target_covers_each_chain_and_stops_at_root` (exit 143 on 180s timeout) | ? SKIP — routed to human verification (host/harness fragility documented in REVIEW-FIX.md for other DACL/token tests on this same class of host) |
| `NonoError::remediation()` has no arm for `LayerAttestationFailed` | `grep -n "fn remediation" -A 40 crates/nono/src/error.rs` | Confirmed: falls to `_ => None`, no `LayerAttestationFailed` arm | ✓ PASS (confirms the NR3-01 gap, not a positive result) |
| CI job runs the fault-injection suites with no pre-built broker / no WFP env (NR-08) | Read `.github/workflows/ci.yml:311-346` | Confirmed: `windows-layer-fault-injection` job runs `cargo test -p nono-sandbox-cli --features layer-fault-injection` with no `NONO_CI_HAS_WFP` and no `cargo build -p nono-shell-broker` step | ✓ PASS (confirms NR-08 is still open) |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| CINT-01 | 117-01, 117-03 | Single fail-direction contract naming every layer, derived from code, citing call sites | ⚠ PARTIAL | Registry + SPEC exist and are substantive (13/13 layers named, each with an outcome), but call-site citations are stale by ~145 lines (NR3-08, unfixed) and the SPEC's discrepancy ledger itself has drifted from the shipped code (see CINT-01's dependency on SC4 below). |
| CINT-02 | 117-02, 05, 08, 09, 10, 11 | Startup self-attestation; never presents an unconfirmed guarantee | ✗ BLOCKED | Core decision logic (`classify_row`, `decide_from_entries`) is correct and tested. Blocked by: NR3-01 (open BLOCKER — CR-14's fix causes self-lockout after abnormal exit / concurrent session, no remediation); NR3-02 (2 rows structurally cannot deny); NR3-03 (fail-open landmine on `DaclSessionSidGrant` guarded by a test that does not exist); NR3-05 (daemon mirror has no downgrade state at all, diverging from the CLI's three-state model). |
| CINT-03 | 117-04, 06, 07, 12 | Per-layer forced-unavailable test for every row; untested row = unsatisfied | ✗ BLOCKED | 2/13 rows have an automated test; 11/13 rely on written manual-verification steps enforced loud but not executed automatically. By CINT-03's own literal wording this requirement is not met for 85% of rows. |

**Orphaned requirements check:** `grep -n "Phase 117" .planning/REQUIREMENTS.md` returns no rows beyond CINT-01/02/03 already covered above — no orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `crates/nono-cli/src/exec_strategy_windows/attestation.rs` | 1558 | `TBD` in a comment | ℹ️ INFO | Benign — refers to the SPEC's now-filled-in latency placeholder that this exact test replaced (D-24 measurements are present in the SPEC, confirmed read). Not a debt marker. |
| (none found) | — | `FIXME` / `XXX` | — | Grep across the 9 phase-touched files under `exec_strategy_windows/`, `agent_daemon/launch.rs`, `output.rs`, `crates/nono/src/error.rs` returned zero hits. |

No blocking debt markers found in the files this phase touched.

### Human Verification Required

See `human_verification` in frontmatter — two items:
1. Full-suite re-run on a clean/elevated host to confirm the pre-existing-11-Windows-failures baseline has not grown (this session's own run hit a host-specific hang unrelated to the phase's logic, isolated `labels_guard::` subset ran clean).
2. Manual execution of the 11 `MANUALLY_VERIFIED` rows' documented steps (elevated WFP service, real console, signed install) — needed operationally, but does not change the SC3 scoring since CINT-03 explicitly requires a *test*, not a manual pass.

### Gaps Summary

Phase 117 shipped substantial, genuinely-verified value: a 13-row code-resident registry that
compiles and is consumed by the attestation gate; a resolved BLOCKER (CR-14) proven by tests that
drive real filesystem state, not synthetic values; a reachable downgrade channel (NR-02) proven by
a real suspended child; independently-sourced network facts (NR-04) and a real daemon predicate
fix (NR-05) that are honestly documented as defense-in-depth rather than exercised negatives; and a
traversal-and-symlink-safe cleanup guard (NR-06) proven by a counterfactual that actually deleted a
victim directory before the fix.

But the phase does not clear its own bar. Three of the four roadmap Success Criteria fail when
measured against their own literal wording, not against SUMMARY.md's narrative:

- **SC3 is failed outright** — 2 of 13 rows have an automated forced-unavailable test; CINT-03's
  own text says an untested row is not satisfied, so 11/13 rows are, by the requirement's own
  definition, unsatisfied. This is not a close call.
- **SC2 is failed as a universal claim**, though resolved for the one row (mandatory integrity
  label) that was the review's headline concern. Two DACL-ancestor rows structurally cannot deny
  (NR3-02); one row (`DaclSessionSidGrant`) is a live fail-open landmine guarded by a test that
  does not exist anywhere in the tree (NR3-03) — this is the single most concerning finding in
  this verification pass, because it means the contract's own self-defense claim is fictional; and
  the fix that resolved the review's headline concern introduced an **open, unfixed BLOCKER**
  (NR3-01) with no remediation path, confirmed still open at the current HEAD commit
  (`ff4fc7af`, 2026-08-10, iteration-3 review — no fix commits follow it in `git log`).
- **SC4 is failed on inspection** — the SPEC's own discrepancy ledger, which exists specifically to
  prevent "quietly reconciled" drift, was itself not updated for 3 of the 5 iteration-2 fixes, and
  one existing row (RF-14) now describes behavior the code no longer has. This is the exact failure
  mode SC4 was written to close, reproduced one review cycle later, inside the very artifact meant
  to catch it.
- **SC1 is partial** — the document exists and names every layer, but "citing the enforcing call
  site" is degraded by a ~145-line citation drift that has now been deferred across three
  consecutive review passes (NR3-08 / WR-06-R).

None of this was invented by this verification pass — every finding above was independently
re-confirmed against the current source tree (grep hit counts, live test runs, direct line reads)
rather than taken from the review documents' word. The review documents' own iteration-3 verdict
(1 Blocker + 9 Warning, `status: issues_found`) is consistent with what this pass found, and no fix
commits have landed since that review was written. The phase should not be marked passed; the gap
closure plan should prioritize NR3-01 (the open blocker — an availability regression with no
recovery path), then the SC3 test-coverage shortfall (the requirement's own explicit bar), then the
SC4 SPEC-ledger and NR3-02/NR3-03 residuals.

---

_Verified: 2026-08-10_
_Verifier: Claude (gsd-verifier)_
