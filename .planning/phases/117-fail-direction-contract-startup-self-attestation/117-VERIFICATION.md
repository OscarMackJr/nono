---
phase: 117-fail-direction-contract-startup-self-attestation
verified: 2026-08-10T00:00:00Z
status: gaps_found
score: 0/4 truths fully verified (SC1 partial-unchanged, SC2 failed-differently, SC3 substantially-improved-but-partial, SC4 failed-freshly)
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 1/4 must-haves verified (SC1 partial, SC2 partial, SC3 failed, SC4 partial) — 0/4 truths fully verified
  gaps_closed:
    - "NR3-01 (iter1's open BLOCKER): abnormal-exit/concurrent-session residue no longer hard-locks the operator out — confirmed live: labels_guard.rs:210-224 now recognizes exact-match residue, and NonoError::remediation() gained a ClearStaleLayerResidue arm (error.rs:505-509)."
    - "NR3-02: dacl_ancestor_traverse / dacl_ancestor_read_attrs now derive LayerApplication from the guards' own coverage().application() accessors, not Option::is_some() on an unconditionally-Some field — confirmed at mod.rs:416-423."
    - "NR3-03: layer_registry.rs's citation of a nonexistent test is fixed — dacl_session_sid_grant_is_not_claimed_anywhere now exists and is confirmed present at layer_registry.rs:1184."
    - "SC3 test coverage: automated force-unavailable coverage grew from 2/13 to 10/13 rows (2 direct layer_force_unavailable.rs tests + 8 ALSO_AUTOMATED entries, all 8 target functions confirmed present in the cited files), with the remaining 3 rows (DaclSessionSidGrant, MinifilterAbsence, BrokerAuthenticodeTrustGate) individually named and justified rather than silently skipped."
    - "SC4 iteration-2 backlog: SPEC-windows-fail-direction-contract.md now carries rows for NR-04, NR-05, NR-06, and NR3-01 through NR3-08 (confirmed present), and RF-14's row was rewritten to describe the shipped three-defense staging guard rather than the pre-NR-06 single-defense guard (confirmed by reading both the SPEC text and the intent of network.rs's cleanup_network_enforcement_staging)."
  gaps_remaining:
    - "SC1 / CINT-01 call-site citations: unchanged from iter1. Independently re-confirmed 2 of WR-08's 15 sampled citations (restricted_token.rs:55, launch.rs:1403) both point at unrelated code (D-29/D-30 comment; a doc comment, not the cited logic). 9 of 13 rows are still uncited-by-symbol."
    - "SC3's literal wording ('a contract entry with no such test is not satisfied'): 3/13 rows remain untested by design, not by test-gap — a smaller, honestly-disclosed remainder than iter1's 11/13, but still short of the roadmap's un-hedged bar."
  regressions:
    - "SC2 / CINT-02 (NEW, confirmed independently, BLOCKER): the very fix that closed NR3-01's availability lockout (Plan 117-13) reopens CR-14's exact failure class through a narrower predicate. AlreadyAtRequiredLevel compares only (rid, mask); low_integrity_label_and_mask (crates/nono/src/sandbox/windows.rs) never reads header.AceFlags, so a structurally-ineffective INHERIT_ONLY_ACE is indistinguishable from an effective label and counts toward LabelCoverage::applied — a launch with zero effective mandatory-label ACEs can now report the layer Confirmed. Confirmed independently: `grep -c AceFlags crates/nono/src/sandbox/windows.rs` returns 0."
    - "D-28 (NEW, confirmed independently, BLOCKER-adjacent, affects CINT-02/CINT-03 plans): Plan 117-16's unconditional tracing::warn! at launch.rs:1516-1533 carries specific downgraded LayerId names, justified by a comment claiming tracing never reaches the confined child's stderr. Confirmed false: cli_bootstrap.rs's default arm (no --log-file) and its file-open-failure fallback arm both build the fmt layer with .with_writer(std::io::stderr) (cli_bootstrap.rs:161-186), and this is the overwhelmingly common invocation path (nono run, the per-tool-call hook)."
gaps:
  - truth: "SC2 / CINT-02 — no path on which nono presents a confinement guarantee it did not confirm; nono never proceeds while presenting a confinement guarantee it cannot substantiate"
    status: failed
    reason: >
      Independently confirmed (code review CR-01, orchestrator-corroborated, and re-verified here
      directly against source): `AppliedLabelsGuard::snapshot_and_apply`'s residue-equivalence
      predicate (`labels_guard.rs:210-224`) treats a prior mandatory-label ACE as
      `AlreadyAtRequiredLevel` — counted toward `LabelCoverage::applied` — whenever `(prior_rid,
      prior_mask)` matches this launch's own wanted values. The reader it depends on,
      `low_integrity_label_and_mask` in `crates/nono/src/sandbox/windows.rs`, never reads
      `header.AceFlags` (confirmed: `grep -c AceFlags crates/nono/src/sandbox/windows.rs` → 0). An
      `INHERIT_ONLY_ACE` — which the OS does not evaluate against the object it sits on — is
      therefore indistinguishable from the effective ACE nono itself writes. A launch whose only
      on-disk ACE is that structurally-ineffective one reports `MandatoryIntegrityLabel` fully
      Confirmed, with zero effective mandatory-label enforcement in force. This is verbatim the
      CR-14 defect this phase exists to close, reopened by the plan (117-13) that closed NR3-01.
      Two compounding defects (WR-01, WR-02), also independently re-confirmed by direct code read:
      the residue check runs BEFORE the ownership gate (`path_is_owned_by_current_user`,
      `labels_guard.rs:258`), so it can fire on paths nono itself could never have labelled; and the
      residue-adopting session's own `Drop` never reverts what it adopted, so the session that
      first applied the label can tear it down (via its own `Drop`) while a second session that
      adopted it is still relying on the attestation — first-session-out-restores, not
      last-session-out-restores as the module doc and the new code comment both claim.
    artifacts:
      - path: "crates/nono-cli/src/exec_strategy_windows/labels_guard.rs"
        issue: "Lines 210-228: equivalence predicate compares (rid, mask) only, no AceFlags check. Lines 246-281: ownership gate runs AFTER the residue check, so residue is accepted on paths nono could not have labelled. Lines 331-342: AlreadyAtRequiredLevel's revert arm is a no-op, so the applying session's own Drop can remove a label an adopting session is relying on."
      - path: "crates/nono/src/sandbox/windows.rs"
        issue: "low_integrity_label_and_mask (~2114-2165) reads header.AceType to filter but never reads header.AceFlags, so INHERIT_ONLY_ACE (structurally inert on the object it sits on) is indistinguishable from an effective label at the predicate's input."
    missing:
      - "Widen the reader to also return AceFlags, and require the residue predicate to reject INHERIT_ONLY_ACE (masking out INHERITED_ACE, which IS effective) before counting toward applied."
      - "Move the residue check to run after the ownership gate so it can only fire on a path nono itself could have labelled."
      - "A negative-direction test: plant an inherit-only ACE via SetNamedSecurityInfoW and assert the guard records SkipPreExistingLabel, not AlreadyAtRequiredLevel."
      - "Revert REQUIREMENTS.md's CINT-02 row from [x] Complete back to [ ] Pending until this is fixed and re-verified — the Complete marking (set by 117-17-SUMMARY.md's requirements-completed: [CINT-02]) predates this finding and is a plan (117-17) downstream of the plan (117-13) that introduced the regression; it does not account for CR-01."
  - truth: "D-28 — layer-specific downgrade detail stays off channels the confined child can read (surfaced inside CINT-02/CINT-03 gap-closure plan 117-16)"
    status: failed
    reason: >
      Independently confirmed (code review CR-02, re-verified here directly). The new unconditional
      `tracing::warn!(downgraded_layers = %dedup_key, ...)` at `launch.rs:1516-1533` is justified by
      an adjacent comment stating "tracing::warn! at the CLI routes to the log file / Event Log
      subscriber, never to the confined child's stderr". Read directly: `init_tracing_with_security`
      (`cli_bootstrap.rs:155-186`) has three arms; only the `--log-file` arm writes to a file. The
      default arm (no `--log-file` — the common `nono run ...` / per-tool-call hook invocation) and
      the file-open-failure fallback arm both call `.with_writer(std::io::stderr)`. The confined
      child shares that console on the non-detached-stdio path. This makes every downgraded launch
      print specific layer names to a console the confined process can read back — the exact
      reconnaissance D-28 exists to prevent, undoing the coarseness `print_attestation_downgrade_banner`
      (`output.rs:114-124`, same eprintln sink, count-only, two lines below) deliberately preserves.
    artifacts:
      - path: "crates/nono-cli/src/exec_strategy_windows/launch.rs"
        issue: "Lines 1516-1533: unconditional tracing::warn! carrying downgraded_layers, justified by a comment that misstates where 2 of 3 init_tracing arms route."
      - path: "crates/nono-cli/src/cli_bootstrap.rs"
        issue: "Lines 161-186: default arm and file-open-failure fallback arm both use .with_writer(std::io::stderr) — the comment's premise is false for the majority invocation path."
    missing:
      - "Gate the layer-name-carrying warn behind a check that tracing is NOT routing to a channel the child can read (e.g. only when --log-file is set), or route the detail through ETW/ Event Log only; keep a count-only warn on the default path."
      - "Correct the SPEC's NR3-04 row (proj/SPEC-windows-fail-direction-contract.md) to state the channel accurately, or explicitly amend D-28 in 117-CONTEXT.md if console exposure is an accepted risk."
  - truth: "SC4 — where the contract and the code disagree, the discrepancy is recorded rather than quietly reconciled"
    status: failed
    reason: >
      The iter1-era gap (SPEC ledger missing NR-04/NR-05/NR-06 rows, RF-14 describing pre-fix
      behaviour) is genuinely closed — confirmed by reading proj/SPEC-windows-fail-direction-contract.md
      directly: rows for NR-04, NR-05, NR-06, and NR3-01 through NR3-08 are present with re-runnable
      evidence, and RF-14 now describes the shipped three-defense staging guard. But the gap-closure
      round that fixed this immediately produced two new BLOCKER-level contract/code discrepancies
      (CR-01, CR-02, both independently re-confirmed above) plus 11 warnings, none of which are yet
      recorded in the SPEC's discrepancy ledger as of the current HEAD — they exist only in
      117-REVIEW.md, an ephemeral review artifact, not in the standing contract document SC4 names.
      This reproduces, inside this same gap-closure round, the exact "quietly reconciled" failure
      pattern SC4 exists to prevent — the phase is being submitted for closure with two known,
      unrecorded, unfixed BLOCKER-level discrepancies between what the contract implies (a
      MandatoryIntegrityLabel Confirmed status is trustworthy; a downgrade warn respects D-28) and
      what the code does.
    artifacts:
      - path: "proj/SPEC-windows-fail-direction-contract.md"
        issue: "No row exists yet for CR-01, CR-02, or any of WR-01 through WR-11 (117-REVIEW.md, iteration 4) — the ledger stops at NR3-08."
    missing:
      - "SPEC rows for CR-01 and CR-02 (at minimum) recording the finding and its disposition — either the fix, or an explicit operator-accepted-risk entry — before the phase closes."
  - truth: "SC1 / CINT-01 — every layer's contract entry cites the enforcing call site accurately"
    status: failed
    reason: >
      Unchanged from iter1, independently re-confirmed (not merely taken from the review): read
      restricted_token.rs:55 and launch.rs:1403 directly, both cited call sites for layer registry
      rows, and both are comment/doc-comment lines unrelated to the enforcing logic the row claims —
      matching WR-08's finding that 15/15 sampled remaining citations (of 9/13 uncited-by-symbol
      rows) point at unrelated code. 4/13 rows were converted to content-verified symbol-form
      citations by Plan 117-15; the remaining 9 were not addressed by this gap-closure round (no
      117-1x plan targeted this beyond the 4 already done).
    artifacts:
      - path: "crates/nono-cli/src/exec_strategy_windows/layer_registry.rs"
        issue: "9 of 13 rows still cite raw file:line pairs that have drifted from the code they describe (verified 2 independently, 15 more verified by the code review this session incorporates as established input)."
    missing:
      - "Convert the remaining 9 rows to content-verified file.rs::Symbol citations, matching the pattern Plan 117-15 already established for the other 4."
human_verification:
  - test: "Manually execute the 3 remaining MANUALLY_VERIFIED rows' documented steps (DaclSessionSidGrant code-read; MinifilterAbsence; BrokerAuthenticodeTrustGate — elevated/signed-install dependent) per crates/nono-cli/tests/layer_registry_meta_test.rs's MANUALLY_VERIFIED entries."
    expected: "Each row's manual-verification steps produce the documented, contracted result."
    why_human: "These are the genuinely host-gated/structural rows this gap-closure round named explicitly (down from 11 in iter1) — they require elevated services, signed production installs, or a code-read of an intentionally-empty layer, none of which this verification pass can execute. This does not change SC3/CINT-03 scoring (CINT-03 requires a test, not a manual pass) but is needed operationally."
---

# Phase 117: Fail-Direction Contract + Startup Self-Attestation Verification Report

**Phase Goal:** The composite's fail-direction stops being decided per-layer-in-isolation and
becomes one system-level answer — and nono can no longer report "enforcing" while a layer is
silently inert.

**Verified:** 2026-08-10
**Status:** gaps_found
**Re-verification:** Yes — this is the second full verification pass (117-VERIFICATION.iter1.md
preserved). Plans 117-13..117-19 closed the majority of iter1's structural gaps, but the code
review this pass treats as established input (`117-REVIEW.md`, iteration 4) found 2 new BLOCKERs
that this report independently re-confirmed against the current source tree rather than accepting
on the review's word.

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|---|---|---|
| SC1 | One document names every layer, states behaviour on failure, citing the enforcing call site | ✗ FAILED (unchanged from iter1) | Registry + SPEC exist, every layer named, every row has an outcome. But 9/13 rows still cite call sites that have drifted from the code they describe. Independently re-confirmed 2 of the review's 15 sampled citations directly: `restricted_token.rs:55` and `launch.rs:1403` both land on unrelated comment lines. |
| SC2 | Forcing a layer unavailable produces abort or visible downgrade; no path presents an unconfirmed guarantee | ✗ FAILED (regressed in kind, not merely unresolved) | iter1's headline row (MandatoryIntegrityLabel / CR-14) was fixed for the true-zero-coverage case, but the SAME fix (Plan 117-13) reopens the identical failure class through a narrower door: `AlreadyAtRequiredLevel` counts a structurally-ineffective `INHERIT_ONLY_ACE` as coverage because `low_integrity_label_and_mask` never reads `AceFlags` (confirmed: 0 hits for `AceFlags` in `crates/nono/src/sandbox/windows.rs`). A zero-effective-enforcement launch can again report Confirmed. |
| SC3 | Every contract entry has a test forcing that layer unavailable, asserting the contracted outcome | ⚠ PARTIAL (substantially improved) | Automated coverage grew from 2/13 (iter1) to 10/13, confirmed by reading both `layer_force_unavailable.rs` (2 direct `fn force_unavailable_*`) and `layer_registry_meta_test.rs`'s `ALSO_AUTOMATED` list (8 entries, all 8 target functions confirmed present at their cited paths — spot-checked `ancestor_traverse_snapshot_and_apply_fails_when_forced_unavailable` and its RA twin, both genuinely force the guard unavailable and assert `LayerAttestationFailed`). 3/13 rows remain named-and-justified manual entries (down from 11), which is real progress but still short of SC3's un-hedged "a contract row without a test is not counted as satisfied." |
| SC4 | Contract/code disagreements are fixed or recorded in-phase, never quietly reconciled | ✗ FAILED (closed old debt, opened new debt in the same round) | iter1's specific gap (missing NR-04/05/06 rows, stale RF-14) is genuinely closed — confirmed by reading the SPEC directly, all rows present, RF-14 rewritten to match the shipped three-defense guard. But this same gap-closure round's own code review (iteration 4) found 2 new BLOCKER-level discrepancies (CR-01, CR-02) that are not yet recorded anywhere in the standing SPEC ledger — reproducing the exact pattern SC4 exists to prevent, one cycle later, inside the very round meant to close it. |

**Score:** 0/4 truths fully verified (unchanged from iter1's headline number, though the underlying
composition improved substantially for SC3 and for iter1's specific SC4/SC2 line items — offset by
2 new BLOCKER-level regressions this pass independently confirmed).

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` | CR-14 fix, now also NR3-01's residue self-heal | ✗ REGRESSED | `coverage()`/`application()` shape (CR-14) is sound. But the NR3-01 residue predicate added in this round (`:210-228`) reopens CR-14's exact class via a narrower door (CR-01, independently confirmed). |
| `crates/nono/src/sandbox/windows.rs` | ACE reader used by the residue predicate | ⚠ INCOMPLETE | `low_integrity_label_and_mask` reads `AceType`+`rid`+`mask` but never `AceFlags` — confirmed via `grep -c AceFlags` → 0. This is the root cause of CR-01. |
| `crates/nono/src/error.rs` | `LayerAttestationFailed` remediation arm (NR3-01) | ⚠ PARTIAL | `ClearStaleLayerResidue` arm now exists (`:505-509`, confirmed present) — closes the "no remediation at all" half of iter1's SC2 gap. But `main.rs`'s error-printing path (`:204-223`) never calls `e.remediation()` for non-`ActionRequired` errors — confirmed by reading `main.rs` directly — so the new arm is still not operator-visible on the primary CLI path (WR-04, unresolved). |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | NR3-02 real coverage accessors | ✓ VERIFIED | `applied_layers()` (`:391-423`) now derives `dacl_ancestor_traverse`/`dacl_ancestor_read_attrs` from `AppliedAncestorTraverseGuard::application`/`AppliedAncestorReadAttributesGuard::application`, not `Option::is_some()` — confirmed by direct read. |
| `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` | NR3-03 landmine fix + NR3-08 citation conversion | ⚠ PARTIAL | `dacl_session_sid_grant_is_not_claimed_anywhere` test confirmed present (`:1184`); the previously-nonexistent citation is fixed. But 9/13 rows' call-site citations remain uncited-by-symbol and drifted (SC1, unresolved this round). |
| `crates/nono-cli/tests/layer_force_unavailable.rs` + `layer_registry_meta_test.rs` (`ALSO_AUTOMATED`) | Per-layer forced-unavailable tests (CINT-03) | ⚠ PARTIAL, substantially improved | 10/13 rows automated (confirmed: 2 direct + 8 ALSO_AUTOMATED functions all present at cited locations), 3/13 named-manual. `contains_fn_exact`'s documented `!`-boundary rule is not implemented (WR-10, not independently re-verified in this pass but accepted as established review input; classified as a warning on the verification mechanism, not on row coverage). |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | D-27/D-28 downgrade channel (NR3-04) | ✗ REGRESSED against D-28 | The banner's "see diagnostic output for details" claim is now backed by a real unconditional warn (closes iter1's NR3-04) — but that warn violates D-28 by carrying layer names onto the shared console on the majority invocation path (CR-02, independently confirmed). |
| `crates/nono-cli/src/agent_daemon/launch.rs` | NR3-05 daemon dead-state removal | ✓ VERIFIED (with a documented honest limit) | `DaemonAttestationDecision` shrunk to `Proceed`/`Abort`; discovery test confirmed to exist. WR-06 (the stated premise for deletion is technically false — passes 1/3 do have skip arms) is accepted as established review input; the deletion's *outcome* (fail direction is safe) is not disputed. |
| `proj/SPEC-windows-fail-direction-contract.md` | Drift-checked contract, discrepancy ledger | ⚠ PARTIAL | iter1's specific debt (NR-04/05/06, stale RF-14) confirmed closed. This round's own new findings (CR-01, CR-02, WR-01..WR-11) are not yet recorded (SC4 gap). |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `AppliedLabelsGuard::snapshot_and_apply` residue predicate | `LabelCoverage::applied` | `(prior_rid, prior_mask)` equality only | ✗ UNSOUND, confirmed | No `AceFlags` check anywhere in the read path — an inherit-only, structurally-ineffective ACE is wired into `applied` exactly like an effective one. |
| `mod.rs::applied_layers()` (ancestor rows) | `LayerApplication` | `AppliedAncestorTraverseGuard::application` / `...ReadAttributesGuard::application` | ✓ WIRED, confirmed fixed | Direct read confirms real coverage accessors replace `Option::is_some()` — NR3-02 genuinely closed. |
| `layer_registry.rs`'s `DaclSessionSidGrant` doc comment | guard test | `dacl_session_sid_grant_is_not_claimed_anywhere` | ✓ WIRED, confirmed fixed | Test confirmed present at `layer_registry.rs:1184` — NR3-03's citation-lie is closed. |
| `apply_startup_attestation_gate`'s `ProceedDowngraded` arm | operator-visible detail | unconditional `tracing::warn!` | ⚠ WIRED BUT LEAKY | The warn now fires unconditionally (closes NR3-04's dead-channel gap) but routes layer names to the confined child's shared console on the default arm (D-28 violation, CR-02). |
| `main.rs` error path | `NonoError::remediation()` | `e.remediation()` call | ✗ NOT WIRED, confirmed | `main.rs:204-223` prints only `Display`; `remediation()` is never called for `LayerAttestationFailed`, so the new `ClearStaleLayerResidue` arm is unreachable from the CLI (WR-04, unresolved). |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `AppliedLayers::mandatory_integrity_label` | `LabelCoverage` | Real filesystem ACE reads, now including a residue-equivalence branch | Yes, but the residue branch accepts a broader ACE-state set than it should | ⚠ FLOWING BUT OVER-BROAD (CR-01) |
| `AppliedLayers::dacl_ancestor_traverse` / `dacl_ancestor_read_attrs` | `LayerApplication` | Guards' own `application()` accessor over real walk state | Yes | ✓ FLOWING (NR3-02 fix confirmed) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| `AceFlags` is read anywhere in the ACE reader used by the residue predicate | `grep -c AceFlags crates/nono/src/sandbox/windows.rs` | `0` | ✓ PASS (confirms CR-01) |
| `init_tracing` writer arms | Read `cli_bootstrap.rs:155-186` directly | Default arm and error-fallback arm both use `.with_writer(std::io::stderr)`; only the `--log-file` arm does not | ✓ PASS (confirms CR-02) |
| Registry citation spot-check #1 | Read `restricted_token.rs:55` | `// D-29/D-30 (Phase 117-06): generalizes the shipped WFP force-unavailable...` — a comment, not the cited enforcing logic | ✓ PASS (confirms WR-08 sample) |
| Registry citation spot-check #2 | Read `launch.rs:1403` | `/// Phase 117 D-21 (CINT-02): apply the startup self-attestation gate...` — a doc comment | ✓ PASS (confirms WR-08 sample) |
| `main.rs` calls `remediation()` for `LayerAttestationFailed` | Read `main.rs:204-223` | Only `ActionRequired` is special-cased; all other errors print `Display` only | ✓ PASS (confirms WR-04) |
| SPEC ledger has rows for CR-01/CR-02 | `grep -n "CR-01\|CR-02" proj/SPEC-windows-fail-direction-contract.md` | No hits | ✓ PASS (confirms SC4 gap) |
| SPEC ledger has rows for NR-04/NR-05/NR-06/NR3-01..08 | Read `proj/SPEC-windows-fail-direction-contract.md:265-274` | All present with re-runnable evidence | ✓ PASS (confirms iter1 SC4 gap closed) |
| `dacl_session_sid_grant_is_not_claimed_anywhere` exists | `grep -rn` in `crates/nono-cli/` | Present at `layer_registry.rs:1184` | ✓ PASS (confirms NR3-03 closed) |
| ALSO_AUTOMATED target functions exist | Read `dacl_guard.rs:1063`, `:1205` | Both `..._fails_when_forced_unavailable` functions present, force the guard unavailable via `force_dacl_grant_unavailable(true)`, and assert `NonoError::LayerAttestationFailed` | ✓ PASS (confirms SC3 progress is real for these 2 rows) |
| Debt markers (TBD/FIXME/XXX) in the 13 review-scoped files | `grep -n "TBD\|FIXME\|XXX"` across all 13 files | No hits | ✓ PASS (no debt-marker blocker) |
| Workspace build/clippy/fmt | Given as established fact for this session (`known_environment_facts`) | Clean with and without `--features layer-fault-injection`; 1611 tests pass; 11 pre-existing Windows-host baseline failures (sanctioned, documented) | ✓ PASS (not re-run this pass; accepted per task framing) |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| CINT-01 | 117-01, 03, 15 (partial), 19 | Single fail-direction contract, derived from code, citing call sites | ✗ BLOCKED | 9/13 rows still cite drifted call sites (SC1). REQUIREMENTS.md correctly shows this as Pending — no change needed. |
| CINT-02 | 117-02, 05, 08, 09, 10, 11, 13, 14, 15, 16, 17 | Startup self-attestation; never presents an unconfirmed guarantee | ✗ BLOCKED — **REQUIREMENTS.md marking not defensible** | REQUIREMENTS.md currently shows `[x] Complete`, set by Plan 117-17's `requirements-completed: [CINT-02]`. CR-01 (independently confirmed, see gaps) is a direct violation of this requirement's own text, introduced by Plan 117-13 — which is itself inside CINT-02's declared span. The Complete marking predates and does not account for this finding. **Recommend reverting to `[ ]` Pending** until CR-01 (and ideally CR-02, which affects the same plan wave's downgrade-channel half of CINT-02) is fixed and re-verified. |
| CINT-03 | 117-04, 06, 07, 12, 16, 18 | Per-layer forced-unavailable test for every row; untested row = unsatisfied | ✗ BLOCKED (substantially closer) | 10/13 rows now automated (up from 2/13); 3/13 remain named-manual. By CINT-03's own literal wording ("a contract entry with no such test is not satisfied") this is still not fully met, but the gap has shrunk from 85% of rows to 23% of rows. |

**Orphaned requirements check:** `grep -n "Phase 117" .planning/REQUIREMENTS.md` returns no rows beyond CINT-01/02/03 — no orphaned requirements. All 19 plans' `requirements:` frontmatter fields resolve to CINT-01/02/03 only.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| (none found) | — | `TBD`/`FIXME`/`XXX` | — | Grepped all 13 files the iteration-4 code review scoped (labels_guard.rs, dacl_guard.rs, layer_registry.rs, launch.rs, mod.rs, attestation_downgrade_event.rs, agent_daemon/launch.rs, telemetry/mod.rs, query_ext.rs, diagnostic/codes.rs, error.rs, layer_registry_selfcheck.rs, layer_registry_meta_test.rs) — zero hits. No debt-marker blocker. |

The material defects in this phase are not debt markers but a fail-open predicate (CR-01) and a
confidentiality-channel violation (CR-02) — both classified under Gaps, not here.

### Human Verification Required

See `human_verification` in frontmatter. One item (down from two in iter1 — the full-suite-hang
item is resolved per this session's `known_environment_facts`, which confirms 1611 passed / 11
sanctioned pre-existing failures, i.e. a full clean run occurred):

1. Manual execution of the 3 remaining `MANUALLY_VERIFIED` rows' documented steps
   (`DaclSessionSidGrant`, `MinifilterAbsence`, `BrokerAuthenticodeTrustGate`) — needed
   operationally, does not change SC3/CINT-03 scoring.

### Gaps Summary

Plans 117-13 through 117-19 closed the majority of what iter1 flagged: NR3-01's lockout is fixed
with a real remediation arm; NR3-02's two constant-`Applied` rows now derive from real guard
coverage; NR3-03's fail-open landmine is closed by a test that actually exists; SC3's automated
test coverage nearly quintupled (2/13 → 10/13); and SC4's specific iter1 debt (missing NR-04/05/06
rows, a stale RF-14 description) is genuinely recorded in the SPEC now. This is real, verified
progress — confirmed by direct source reads in this pass, not taken from SUMMARY.md claims.

But the round does not clear the roadmap's bar, for two independent reasons:

1. **A new BLOCKER (CR-01) reopens CR-14's exact class of defect.** The residue predicate that
   fixes NR3-01's availability problem is unsound: it cannot distinguish a structurally-ineffective
   inherit-only ACE from the effective ACE nono itself writes, because the ACE reader it depends on
   never inspects `AceFlags` — confirmed by direct grep (`0` hits) and by reading the predicate at
   `labels_guard.rs:210-228`. This means SC2 and CINT-02, both of which this phase exists to
   satisfy, are violated in a new way even though the row iter1 traced end-to-end (the pure
   zero-ACE case) is fixed. CINT-02's `[x] Complete` marking in `REQUIREMENTS.md` — set by Plan
   117-17, a plan downstream of and blind to this finding — is not defensible until CR-01 is fixed.

2. **A second BLOCKER-adjacent finding (CR-02) violates D-28**, the decision this same gap-closure
   wave's NR3-04 fix was supposed to satisfy without leaking layer names to the confined child's
   shared console. Confirmed directly: 2 of `init_tracing`'s 3 arms — including the default,
   overwhelmingly common path — route `tracing::warn!` to stderr, contradicting the new warn's own
   justifying comment.

3. **SC4, the criterion meant to prevent exactly this kind of unrecorded drift, itself has an open
   instance**: neither CR-01 nor CR-02 (nor the 11 accompanying warnings) is yet recorded in
   `proj/SPEC-windows-fail-direction-contract.md`'s discrepancy ledger. The phase is being submitted
   for closure with two known, independently-confirmed, unfixed BLOCKER-level discrepancies between
   contract and code.

4. **SC1/CINT-01's citation drift is unchanged** — independently re-confirmed on 2 of the review's
   15 sampled citations (both land on unrelated comment lines) — no plan in this gap-closure round
   targeted the remaining 9/13 rows.

None of this was taken on the review documents' word alone: `AceFlags` absence, the `cli_bootstrap.rs`
writer arms, 2 of the stale citations, the SPEC ledger's current contents, the `main.rs` remediation
gap, and the `mod.rs`/`layer_registry.rs` fixes were each independently re-read against the current
source tree in this verification pass. The phase should not be marked passed. Priority for the next
gap-closure round: (1) fix CR-01 (the predicate's ACE-flags blindness — this is the security-relevant
one, since it converts an integrity claim into a false positive, not merely an availability problem);
(2) fix or gate CR-02 (the D-28 leak); (3) record both in the SPEC ledger regardless of fix status;
(4) finish the remaining 9/13 SC1 citation conversions; (5) resolve WR-04 (wire `main.rs` to call
`remediation()`) so NR3-01's fix is actually operator-visible.

---

_Verified: 2026-08-10_
_Verifier: Claude (gsd-verifier)_
