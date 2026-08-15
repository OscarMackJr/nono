---
phase: 117-fail-direction-contract-startup-self-attestation
verified: 2026-08-15T00:00:00Z
status: human_needed
score: >
  3/4 truths verified on evidence, 4/4 satisfied. SC1 VERIFIED; SC2 VERIFIED-with-disclosed-limits;
  SC3 ACCEPTED VIA OPERATOR OVERRIDE (factually still 10/13 — the override accepts a disclosed
  shortfall, it does not erase it); SC4 VERIFIED-with-disclosed-limits — all four of its `missing`
  bullets were closed on 2026-08-15 by quick task 260815-b0s (commits 511e2ab1, fba34606), AFTER
  this verification pass. The limits SC4 carries are stated in its gap entry below: the new
  list-assignment gate verifies 5 module-doc claims and holds 13 further row mentions constant by
  an equality pin without verifying them, and `every_open_marker_in_code_has_a_ledger_row` still
  has no ledger->code direction. Residual human verification is unchanged: the WR-20 pin
  (`non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`) is host-blocked, and
  `cargo test --bin nono` is RED at HEAD (1688 passed / 12 failed).
overrides_applied: 1
overrides:
  - must_have: "Every entry in the contract has a test that forces that layer unavailable and asserts the contracted outcome — a contract row without a test is not counted as satisfied"
    reason: "10/13 rows are automated and verified. The 3 remaining rows are named, justified and SPEC-documented: DaclSessionSidGrant and MinifilterAbsence are rows for layers that do not exist in this tree (nothing to force unavailable), and BrokerAuthenticodeTrustGate is inert outside a signed production install. Accepted in the 2026-08-10 gap-closure session; enforced loud by host_gated_rows_are_loud."
    accepted_by: "Oscar Mack Jr"
    accepted_at: "2026-08-15T00:00:00Z"
re_verification:
  previous_status: gaps_found
  previous_score: 0/4 truths fully verified (SC1 partial-unchanged, SC2 failed-differently, SC3 substantially-improved-but-partial, SC4 failed-freshly)
  gaps_closed:
    - "SC1 / CINT-01 citation drift (iter2's headline SC1 gap): CLOSED. Every registry and SPEC call-site citation is now symbol-form (`file.rs::Symbol`), zero raw `file:line` citations remain (`no_line_number_citations_remain_in_the_registry_surface` passes), and the citations are CONTENT-verified against a real definition site. Independently proved discriminating: perturbing one citation to `AppliedDaclGrantsGuard::snapshot_and_apply_v2` made BOTH `registry_call_sites_exist` and `spec_call_site_cells_match_registry_call_sites` FAIL with precise messages. iter2's two spot-checked stale citations (restricted_token.rs:55, launch.rs:1403) no longer exist as citations at all."
    - "SC2 / CR-01 BLOCKER (AceFlags-blind residue predicate): CLOSED. `low_integrity_label_ace` now returns `AceFlags`; `labels_guard.rs:336-339` rejects `INHERIT_ONLY_ACE` before counting toward `applied`; the ownership gate (`path_is_owned_by_current_user`, :292) now runs BEFORE the residue check (iter2 WR-01); a negative test (`inherit_only_residue_is_not_treated_as_already_covered`) exists and runs in the DEFAULT build. The class was closed mechanically, not by grep: `every_low_integrity_label_ace_consumer_filters_inherit_only` (windows.rs:3468) scans the production half of both consumer files with a `checked > 0` non-vacuity floor."
    - "SC2 / CR-02 BLOCKER-adjacent (D-28 leak): CLOSED. All three `ProceedDowngraded` emission sites now compute ONE shared `layer_detail` gated on `cli_bootstrap::log_target_is_private()` (launch.rs:1617-1650); the banner (`output.rs:139-193`) is count-only on every arm; the per-session dedup marker's CONTENT is now a domain-separated digest (`attestation_downgrade_marker_content`, output.rs:361-374), not the plaintext layer set."
    - "SC4 / iter2's unrecorded CR-01+CR-02: CLOSED. Both are present in the SPEC's D-15 ledger (`proj/SPEC-windows-fail-direction-contract.md:275`, `:276`) with re-runnable evidence and iteration-5 addenda, alongside CR-03 and WR-01..WR-21."
    - "Round-8 CR-01 (`every_markdown_file_gated_by_a_test_runs_the_code_jobs` failing in the real repo): CLOSED and re-proved IN THE REAL TREE at `C:\\Users\\OMack\\Nono`, which does carry the defect condition (15 gitignored worktrees under `.claude/worktrees/` plus `.gsd/`). 20 passed / 0 failed."
    - "Round-8 WR-01 (`#[cfg(test)]`-gated non-`mod` items landing in the production half): CLOSED, proved by the decisive two-part perturbation rather than by reading — I injected `#[cfg(test)] pub(crate) fn verifier_probe_helper() -> &'static str { \"DaclAncestorTraverse\" }` into `agent_daemon/launch.rs`'s production region AND renamed the real production site; the daemon gate FAILED, proving the cfg-test helper cannot satisfy it."
  gaps_remaining:
    - "SC3's literal wording ('a contract row without a test is not counted as satisfied'): 3/13 rows still have no forced-unavailable test. Composition unchanged from iter2, but the rows are now individually justified and SPEC-documented, and I verified the 10 covered rows' test BODIES rather than accepting the name-existence check. — STILL FACTUALLY TRUE. ACCEPTED 2026-08-15 by operator override (see `overrides` above); the 10/13 finding stands unaltered."
    - "SC4: no new contract/code divergence of iter2's kind, but the SPEC ledger has not been touched since before the round-4 review, and one live doc/code disagreement is unrecorded (see gaps). — CLOSED 2026-08-15 by quick task 260815-b0s (commits 511e2ab1, fba34606): the doc/code disagreement is corrected and machine-checked, the stale TODO(117-12) is replaced, the SPEC states the D-15 scoping rule and indexes rounds 4/6/8 with finding counts and dispositions, and both operator-deferred items now carry greppable `OPEN` code markers. See the SC4 gap entry below for what is and is not covered."
  regressions: []
gaps:
  - truth: "SC3 / CINT-03 — every entry in the contract has a test that forces that layer unavailable and asserts the contracted outcome; a contract row without a test is not counted as satisfied"
    status: accepted_via_override
    override_ref: "overrides[0] — accepted_by Oscar Mack Jr, accepted_at 2026-08-15T00:00:00Z. The override ACCEPTS the disclosed 10/13 shortfall; it does not erase it. Every word of the `reason` below is the evidence for that acceptance and is unchanged."
    reason: >
      10 of 13 rows are covered by a real, running, substantive test — I verified this by reading
      every one of the 10 test BODIES rather than trusting the meta-test's name-existence check,
      and by proving the discovery machinery discriminates (renaming
      `ancestor_traverse_snapshot_and_apply_fails_when_forced_unavailable` made BOTH
      `every_registry_row_has_a_test` and `also_automated_entries_are_non_vacuous` FAIL). All 10
      run in a real gate: 6 in the default `cargo test --bin nono`, 6 seam tests under
      `--features layer-fault-injection` (verified live, 6 passed), the 2 external-subprocess
      tests under the same feature (verified live, 2 passed), and the broker row under
      `cargo test -p nono-shell-broker --features layer-fault-injection`. CI job
      `windows-layer-fault-injection` (`.github/workflows/ci.yml:343-388`) runs the feature legs,
      so the SPEC's "or that test fails the build" claim is backed by an actual build.
      The remaining 3 rows have no such test: `DaclSessionSidGrant` and `MinifilterAbsence` are
      rows for layers that do not exist in this tree (empty expectancy / structural absence per
      ADR-65), so there is nothing to force unavailable; `BrokerAuthenticodeTrustGate` is a real
      layer whose gate is inert outside a signed production install. By CINT-03's own literal
      wording those 3 rows are not satisfied. This is an operator-accepted remainder recorded in
      the SPEC's Manual verification section and mechanically kept loud by `host_gated_rows_are_loud`
      — it is a disclosed partial, not a hidden one.
      Separately: `exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`
      (the WR-20 pin) FAILS on this host and is one of the 12 failures in `cargo test --bin nono`.
      It fails LOUDLY with a full explanatory message per D-31 rather than skipping. It pins the
      `MandatoryIntegrityLabel` row's CLASSIFICATION RULE for a non-owned + foreign-labelled path,
      not that row's forced-unavailable test (which passes), so it does not reduce SC3's 10/13 row
      coverage — but it does mean one contracted classification rule is authored-and-unverified
      until an elevated/CI Windows runner with `SeTakeOwnershipPrivilege`/`SeRestorePrivilege`
      executes it, and it means `cargo test --bin nono` is RED at HEAD with a phase-117-authored
      test among the failures.
    artifacts:
      - path: "crates/nono-cli/tests/layer_registry_meta_test.rs"
        issue: "MANUALLY_VERIFIED (:153-184) holds 3 of 13 rows. The discovery gate checks a test FUNCTION NAME exists in a cited file; it does not check the body forces the layer unavailable or asserts the contracted outcome. (I closed that by reading all 10 bodies; the mechanism itself remains name-based.)"
      - path: "crates/nono-cli/src/exec_strategy_windows/labels_guard.rs"
        issue: "`non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` cannot construct its non-owned + foreign-labelled precondition on a non-elevated account and fails loudly; needs an elevated/CI Windows runner."
    missing:
      - "CLOSED 2026-08-15 (the second alternative was taken): 'Either an automated forced-unavailable test for `BrokerAuthenticodeTrustGate` (the only one of the 3 that is a live layer), or an explicit VERIFICATION override recording the operator's 2026-08-10 acceptance of the 3-row remainder against SC3's un-hedged wording.' — the override is applied in this file's frontmatter, signed by the operator. No test was added for `BrokerAuthenticodeTrustGate`; it remains inert outside a signed production install, and the 3-row remainder (`DaclSessionSidGrant`, `MinifilterAbsence`, `BrokerAuthenticodeTrustGate`) stays named and disclosed."
      - "STILL OPEN: an elevated/CI Windows RUN of `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` to convert WR-20 from authored-but-unverified to verified. What is missing is a RUN, not CI wiring — no new CI wiring is required. The test is NOT feature-gated: it is a plain `#[test]` at `labels_guard.rs:1188` inside `#[cfg(test)] mod tests` (declared :567-570), so it lives in the `nono` bin's ordinary unit tests. CI job `windows-layer-fault-injection` (`.github/workflows/ci.yml:343-388`) therefore ALREADY runs it by construction: its invocation at `ci.yml:387` carries no `--bin`/`--test`/`--lib` filter, so it runs every target in the package. The blocker is that the job has never executed this test — branch `milestone/v2.13-carryforward-closeout` is 684 commits ahead of origin (0 behind) and the last CI run on it, 2026-07-29, predates Phases 115/116/117 entirely. Pushing is an operator decision, which is why this stays a human item. Bounding the job's evidentiary value so it is not overstated: the `test` job matrix (`ci.yml:129-131`) is `[ubuntu-latest, macos-latest]`, making `windows-layer-fault-injection` the ONLY job that runs Rust unit tests on Windows anywhere in CI; and because it runs the whole package suite, the other 11 documented Windows-host baseline failures will surface there too. The 6 `config::tests` env races may well pass under its `--test-threads=1`, but that is a PREDICTION, NOT A MEASUREMENT. The override does not touch this — it is about SC3's ROW COVERAGE, not about that test, which continues to FAIL loudly per D-31 on this non-elevated host and keeps `cargo test --bin nono` RED at HEAD (1688 passed / 12 failed)."
  - truth: "SC4 — where the contract and the code disagree, the code is changed or the contract is corrected in the same phase, with the discrepancy recorded rather than quietly reconciled"
    status: closed_after_verification
    closure: >
      All FOUR `missing` bullets below were closed on 2026-08-15 by quick task 260815-b0s
      (commits 511e2ab1 and fba34606), after this verification pass. The three residuals recorded
      in `reason` are the finding of record and are left verbatim; what follows is what actually
      closed each, and — as importantly — what is still NOT covered.
      (1) CLOSED. `layer_force_unavailable.rs`'s module doc now assigns both rows to
      `ALSO_AUTOMATED` and names each row's in-process test, and the assignment is machine-checked
      by a new sibling gate, `module_doc_assigns_each_claimed_row_to_the_list_it_is_on`
      (`layer_registry_meta_test.rs`). The rule is SENTENCE-scoped with nearest-preceding-list-name
      association, discovery-based over `LayerId::ALL` (it names no layer literally). Proved by TWO
      recorded perturbations, not by inspection: flipping the corrected sentence's list name FAILED
      naming both rows, the claimed list and the real list; rewording the claim back to the
      anaphoric "both rows" shape FAILED on both pins (verified claims 5 -> 3, unverified mentions
      13 -> 15). Both reverted.
      (2) CLOSED, and BOTH alternatives were taken rather than one. The SPEC's
      `## Contract vs. code discrepancies` section now states the D-15 scoping rule
      (contract-vs-code divergences here; verification-machinery defects in the committed
      `117-REVIEW*` / `117-REVIEW-FIX*` artifacts) AND indexes rounds 4, 6 and 8 with their finding
      COUNTS (8 / 7 / 5, one Critical) and per-finding dispositions read from those artifacts. All
      20 findings are verification-machinery; none was reclassified as a contract-vs-code
      divergence, so none earned a ledger row. Round 6's WR-04 — the one with a plausible claim to
      being production-side — is dispositioned explicitly in the SPEC rather than swept into the
      index: its `collect_files` is a nested helper inside the `#[test] fn
      downgrade_marker_files_never_contain_a_layer_name` D-28 leak scan, i.e. the gate, and the
      round-7 fix report states "No production behaviour changed this round".
      (3) CLOSED. The stale `TODO(117-12)` is gone; `registry_call_sites_exist`'s doc now describes
      what it does at HEAD (symbol-form citations, content-verified) and states the narrower
      residual risk that genuinely remains (a symbol can survive while no longer performing the
      enforcement its row describes — a semantic claim no source-text scan settles).
      (4) CLOSED via the marker alternative. `CR-02` and `RF-13` now carry greppable
      `OPEN (NOT FIXED ...)` markers at the three sites an operator would grep, and both ledger
      rows carry the literal `OPEN` token, so all FOUR open items (WR-10, WR-14, CR-02, RF-13)
      resolve through `every_open_marker_in_code_has_a_ledger_row`. Exactly one `| CR-02 ` ledger
      row carries `OPEN` and it is the Iteration-6 one (the id collides with an Iteration-4 row the
      gate cannot distinguish). `marker_sources()` was hoisted to module scope (one definition, two
      callers) and a new discovery gate,
      `every_marker_carrying_source_file_is_in_marker_sources`, closes the file class: every
      tracked `crates/*/src/**.rs` file carrying a marker line must be listed. That gate
      discriminated for real — run before the list was extended, it FAILED naming both new marker
      files by path and line.
      WHAT IS STILL NOT COVERED, stated so the record does not overclaim: (a) the new
      list-assignment gate verifies the 5 module-doc claims made in sentences that name a list; it
      does NOT verify the 13 further row mentions (10 in the "reasons below cover rows on BOTH
      lists" sentence, 2 in a section heading, 1 inside a quoted diagnostic). Those make no
      assignment claim and are held CONSTANT by an equality pin, not checked. (b)
      `every_open_marker_in_code_has_a_ledger_row` still enforces only the code -> ledger
      direction: a ledger row marked `OPEN` with no corresponding code marker would still pass.
      (c) markers placed in `tests/` are out of the discovery gate's reach by design.
    reason: >
      The mechanism is real and strong: the SPEC's D-15 ledger carries ~50 rows through Iteration 6
      with re-runnable evidence, later findings are folded in as in-place addenda (WR-25, WR-27,
      WR-34, CR-06), and contract/code sync is MECHANICALLY enforced by gates I proved
      discriminating (`spec_call_site_cells_match_registry_call_sites` and `registry_call_sites_exist`
      both fail on a single perturbed citation; `every_open_marker_in_code_has_a_ledger_row` passes
      and is non-vacuity-floored). iter2's specific SC4 gap is genuinely closed.
      Three residuals stop this being a full pass.
      (1) A LIVE, unrecorded doc/code disagreement inside the phase's own coverage-declaring file:
      `crates/nono-cli/tests/layer_force_unavailable.rs:110-112` states "Both rows are therefore
      on the `MANUALLY_VERIFIED` list with their manual reproduction steps" about `RestrictedToken`
      and `JobObjectContainment`. They are on `ALSO_AUTOMATED`
      (`layer_registry_meta_test.rs:198-239`), not `MANUALLY_VERIFIED` (`:153-184`). This is
      verbatim the class Iteration 4's WR-07 fixed once already in this same file ("a reader
      auditing coverage from that doc went looking for a test that does not exist"), and the gate
      WR-07 added (`coverage_split_accounts_for_every_layer_id`) checks the ARITHMETIC and
      DISJOINTNESS but structurally cannot see which list the prose names. The direction is
      under-claim (both rows are in fact automated), so it is not a security defect — but it is a
      contract/code disagreement left standing, which is what SC4 forbids.
      (2) The SPEC has not been edited since `860d4772`, which precedes the round-4 code review
      (`62892678`). Three review/fix rounds since then (4->5, 6->7, 8->9), including one Critical
      (round-8 CR-01), produced no ledger row. I judged those findings to be verification-machinery
      defects rather than contract-vs-code divergences, and each IS recorded in a committed review
      and fix artifact — so this is materially weaker than iter2's finding. But the phase's own
      precedent (Waves 11 and 14 were dedicated "record the whole round in the ledger" plans) was
      not followed for the last three rounds, and nothing mechanically requires it.
      (3) `crates/nono-cli/tests/layer_registry_selfcheck.rs:354-358` still carries
      `// TODO(117-12): this asserts file EXISTENCE ... it does not verify the cited line NUMBER
      still points at the right code`. Both halves are now false: no line-number citations remain,
      and symbol citations ARE content-verified. Stale record on the gate that carries SC1.
    artifacts:
      - path: "crates/nono-cli/tests/layer_force_unavailable.rs"
        issue: "Lines 110-112 assign RestrictedToken/JobObjectContainment to MANUALLY_VERIFIED; they are on ALSO_AUTOMATED. No gate can see it. — RESOLVED 2026-08-15 (511e2ab1): the claim now names ALSO_AUTOMATED and both rows in ONE sentence, and module_doc_assigns_each_claimed_row_to_the_list_it_is_on can see it."
      - path: "proj/SPEC-windows-fail-direction-contract.md"
        issue: "Ledger last edited at 860d4772, before the round-4 review (62892678). Rounds 4/6/8 findings — including round-8's Critical — have no ledger row. — RESOLVED 2026-08-15 (fba34606): the section states the D-15 scoping rule and indexes all three rounds with finding counts and per-finding dispositions. No round produced a contract-vs-code divergence, so none earned a row."
      - path: "crates/nono-cli/tests/layer_registry_selfcheck.rs"
        issue: "Line 354 TODO(117-12) describes a gate that no longer behaves that way (raw file:line citations are gone; symbol content IS verified). — RESOLVED 2026-08-15 (511e2ab1): replaced with what the gate does at HEAD plus the narrower residual risk that really remains."
    missing:
      - "CLOSED 2026-08-15 (511e2ab1): 'Correct layer_force_unavailable.rs:110-112 to name ALSO_AUTOMATED, and extend coverage_split_accounts_for_every_layer_id (or add a sibling) so the prose's list assignment is machine-checked, not just the totals.' — a sibling was added, not an extension; two perturbation proofs recorded."
      - "CLOSED 2026-08-15 (fba34606): 'Either a ledger pass recording rounds 4/6/8 (with their dispositions), or an explicit statement in the SPEC that the D-15 ledger scopes to contract-vs-code divergences and that verification-machinery findings live in the committed 117-REVIEW*/117-REVIEW-FIX* artifacts.' — BOTH were done: the scoping rule AND a per-round index carrying finding counts and dispositions."
      - "CLOSED 2026-08-15 (511e2ab1): 'Refresh or delete the stale TODO(117-12) at layer_registry_selfcheck.rs:354.' — refreshed, not deleted; no TODO( marker remains in that file."
      - "CLOSED 2026-08-15 (fba34606) via the marker alternative: 'Optional, for symmetry with WR-10/WR-14: greppable `OPEN` markers in code for the two operator-deferred items (CR-02 daemon wiring, RF-13/WR-12 fleet control), or a ledger->code direction on every_open_marker_in_code_has_a_ledger_row. Today an operator grepping the code finds 2 of the 4 open items.' — an operator now finds 4 of 4 (5 marker sites; RF-13 is annotated at both its reader and its gate call site). The ledger->code direction was NOT built and remains absent."
deferred:
  - truth: "CR-02 — the EntryPath::Daemon half of the registry drives no decision; nono-agentd cannot link the registry"
    addressed_in: "v3.7 carry-forward (operator decision, 2026-08-14)"
    evidence: "SPEC row proj/SPEC-windows-fail-direction-contract.md:299 — 'Partial: the drift gate is closed; the wiring decision is NOT taken', with Option A / Option B stated and deliberately not guessed. Rationale re-verified accurate at HEAD: crates/nono-cli/src/bin/nono-agentd.rs #[path]-includes only ../agent_daemon/mod.rs, ../telemetry/mod.rs and ../agent_daemon/telemetry_init.rs — it never declares exec_strategy_windows. The binding gate daemon_expected_rows_are_all_named_by_the_daemon_gate exists (layer_registry.rs:1425) with a `checked >= 5` non-vacuity floor, and I proved it discriminating by perturbation."
  - truth: "WR-12 / RF-13 — fleet-control RequiredLayers plumbing is read and warned about but never enforced"
    addressed_in: "v3.7 carry-forward (operator decision, 2026-08-14)"
    evidence: "SPEC row proj/SPEC-windows-fail-direction-contract.md:262 — 'Enforcement is NOT implemented', with the open operator decision stated. Re-verified accurate at HEAD: machine_policy.rs:716-739 reads the sub-key and emits a loud `RequiredLayersNotEnforced` warning; the production gate call site launch.rs:1585-1586 passes `required_layers_override: &[]` and `machine_required_layers: &[]` with a comment explaining that this is tighten-only, so the deferral cannot fail open."
human_verification:
  - test: "A RUN, not new CI wiring. Either (a) push branch `milestone/v2.13-carryforward-closeout` so CI job `windows-layer-fault-injection` (`.github/workflows/ci.yml:343-388`) executes it — the job already runs it BY CONSTRUCTION, because its cargo invocation at `ci.yml:387` carries no `--bin`/`--test`/`--lib` filter and therefore runs every target in the package, and the test is NOT feature-gated (a plain `#[test]` at `labels_guard.rs:1188` inside `#[cfg(test)] mod tests` declared at :567-570, i.e. one of the `nono` bin's ordinary unit tests); or (b) run `cargo test -p nono-sandbox-cli --bin nono non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` from an ELEVATED session that holds SeTakeOwnershipPrivilege / SeRestorePrivilege / SeBackupPrivilege."
    expected: "The test constructs a non-owned + foreign-labelled path and passes, confirming the WR-20 classification rule (such a path is contract-exempt, not a coverage gap). Via route (a), expect the other 11 documented Windows-host baseline failures to surface in the same job as well, because it runs the whole package suite. The 6 `config::tests` env races may well pass under the job's `--test-threads=1` — that is a PREDICTION, NOT A MEASUREMENT."
    why_human: "No new CI wiring is required: the coverage already exists and what is missing is a RUN, which is blocked on an operator decision to push. Branch `milestone/v2.13-carryforward-closeout` is 684 commits ahead of origin (0 behind), and the last CI run on it — 2026-07-29 — predates Phases 115/116/117 entirely, so `windows-layer-fault-injection` has never executed this test. It also cannot be closed locally: this dev host runs a non-elevated domain account; `icacls /setowner` fails for both SYSTEM and BUILTIN\\Administrators. The test fails LOUDLY with a full explanatory message per D-31 rather than skipping, so the gap is disclosed — but it cannot be closed from this host, and WR-20 stays authored-but-unverified until a real run exists, with `cargo test --bin nono` RED at HEAD (1688 passed / 12 failed). Bounding the job's evidentiary value so it is not overstated: the `test` job matrix (`ci.yml:129-131`) is `[ubuntu-latest, macos-latest]`, making `windows-layer-fault-injection` the ONLY job that runs Rust unit tests on Windows anywhere in CI; and because it runs the whole package suite it will also inherit the other 11 documented Windows-host baseline failures. The 6 `config::tests` env races may well pass under its `--test-threads=1`, but that is a PREDICTION, NOT A MEASUREMENT."
  - test: "Execute the 3 MANUALLY_VERIFIED rows' documented steps per crates/nono-cli/tests/layer_registry_meta_test.rs:153-184 — DaclSessionSidGrant (a code read: `grep -rn \"session_sid\" crates/nono-cli/src` must still show no DACL grant of config.session_sid), MinifilterAbsence (structural, ADR-65), BrokerAuthenticodeTrustGate (from a SIGNED production install outside target/, stage a nono-shell-broker.exe signed by a different identity and confirm the broker-arm spawn refuses)."
    expected: "Each row's documented steps produce the contracted result."
    why_human: "One is a structural code read, one is a documented absence, and the third needs a signed, non-dev-layout production install that no `cargo build` host can produce."
  - test: "Run the full `cargo test --workspace --no-fail-fast` to completion on a quiet machine and reconcile the extras against the documented Windows-host baseline."
    expected: "`--bin nono` shows 1688 passed / 12 failed (the documented baseline list, including the host-blocked WR-20 pin); `-p nono-sandbox` shows 844 passed / 0 failed; the workspace extras are pre-existing and untouched by this phase (e.g. audit_attestation::* hardcodes `/bin/pwd` at crates/nono-cli/tests/audit_attestation.rs:147 and :209, which cannot work on Windows)."
    why_human: "The `--bin nono` and `-p nono-sandbox` figures were measured live in this pass. The full workspace `--no-fail-fast` sweep is known to run long on this host (the 36-binary CLI sweep stalls ~25 min) and did not complete inside this verification window."
---

# Phase 117: Fail-Direction Contract + Startup Self-Attestation — Verification Report

**Phase Goal:** The composite's fail-direction stops being decided per-layer-in-isolation and
becomes one system-level answer — and nono can no longer report "enforcing" while a layer is
silently inert.

**Verified:** 2026-08-15
**Status:** gaps_found (2 PARTIAL truths, 0 FAILED, 0 BLOCKER)
**Re-verification:** Yes — third full pass. `117-VERIFICATION.iter1.md` and
`117-VERIFICATION.iter2.md` are preserved. iter2 (2026-08-10) is nine review/fix rounds stale; I
read it for its specific gaps and then re-derived every verdict against HEAD `28c866f4` rather
than carrying its findings forward.

## Where the runs happened

Per the method requirement, and because round 8 caught a test that passed in a clean worktree and
failed in the real repository:

| Run | Where | Result |
|---|---|---|
| `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` | **REAL TREE** `C:\Users\OMack\Nono` | **20 passed, 0 failed** |
| `cargo test -p nono-sandbox-cli --test layer_registry_meta_test --test layer_force_unavailable` | **REAL TREE** | 10 passed / 0 (feature-filtered) |
| `cargo test -p nono-sandbox-cli --features layer-fault-injection --test layer_force_unavailable` | **REAL TREE** | **2 passed, 0 failed** |
| `cargo test -p nono-sandbox-cli --features layer-fault-injection --bin nono -- fails_when_forced_unavailable` | **REAL TREE** | **6 passed, 0 failed** |
| `cargo test -p nono-sandbox-cli --bin nono` | **REAL TREE** | **1688 passed, 12 failed, 2 ignored** |
| `cargo test --workspace` (fail-fast) | **REAL TREE** | `-p nono-sandbox` 844 passed / 0 failed; stopped at `--bin nono`'s 12 |
| `cargo fmt --all -- --check` | **REAL TREE** | exit 0 |
| 3 adversarial perturbations (below) | **REAL TREE**, each reverted with `git checkout --` | all FAILED as required |

The real tree **does** carry the round-8 CR-01 defect condition: 15 gitignored agent worktrees
under `.claude/worktrees/` plus `.gsd/`. `every_markdown_file_gated_by_a_test_runs_the_code_jobs`
passes here, which is the environment where round 8 caught it failing. `git status --porcelain`
after every perturbation showed only the untracked `117-VERIFICATION.iter2.md`.

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|---|---|---|
| SC1 | One document names every layer, states its behaviour when it cannot be established, citing the enforcing call site | ✓ VERIFIED | `proj/SPEC-windows-fail-direction-contract.md:50-64` + `layer_registry.rs:876-1098` carry all 13 rows, covering every layer SC1 names by name: restricted token, mandatory integrity label, AppContainer profile, package-SID DACL grant (+ 3 more DACL rows), WFP egress filters, and the minifilter's absence (`ContractOutcome::FailOpen { justification: "ADR-65: no minifilter exists; per-file read policy inside one directory is explicitly not claimed" }`). Every row states an explicit `ContractOutcome`. **iter2's headline gap is closed:** zero raw `file:line` citations remain, every citation is `file.rs::Symbol`, and each is CONTENT-verified against a real definition site (definition-line prefix + identifier boundary + enclosing-`impl` scoping). Proved discriminating by perturbation. |
| SC2 | Forcing a layer unavailable produces abort or a visibly downgraded claim; no path presents a confinement guarantee it did not confirm | ✓ VERIFIED (with disclosed, bounded limits) | The gate is applied unconditionally at all three spawn paths, each fail-closed with `TerminateProcess` **before** resume: `launch.rs:2558` (DirectCli), `agent_daemon/launch.rs:925` (Daemon), `nono-shell-broker/src/main.rs:927` (Broker). Both iter2 BLOCKERs are closed and I re-proved each against source (see below). End-to-end live proof: two real `nono.exe` subprocess launches with a layer forced unavailable both aborted with `LayerAttestationFailed`. `applied_layers()` (`mod.rs:391-449`) derives from each guard's own `coverage().application()` with fail-secure `NotApplied` defaults. Limits, all disclosed in-code: the per-session banner dedup marker is forgeable by a same-user process on the `Null`/`WriteRestricted` arms (`output.rs:75-101` states this explicitly and names two independent channels it does not control); and on the Daemon arm the decision is a hand-written mirror bound to the registry only by a source-text drift gate — an operator-deferred item (CR-02), recorded. |
| SC3 | Every entry in the contract has a test that forces that layer unavailable and asserts the contracted outcome; a row without a test is not satisfied | ✓ ACCEPTED VIA OPERATOR OVERRIDE (factually 10/13 — unchanged) | 2 direct + 8 `ALSO_AUTOMATED` + 3 `MANUALLY_VERIFIED` = 13, arithmetic and disjointness machine-enforced. I read all 10 covered tests' BODIES — every one forces its layer unavailable (or drives the real gate to an Unconfirmed state) and asserts `LayerAttestationFailed` naming that layer. All 10 run in a real gate (default build, the `--features layer-fault-injection` legs, or `-p nono-shell-broker`), and CI job `windows-layer-fault-injection` runs the feature legs. Discovery proved discriminating by perturbation. **The literal bar is still short by 3:** two of those rows are for layers that do not exist in this tree (nothing to force unavailable); one, `BrokerAuthenticodeTrustGate`, is a live layer whose gate is inert outside a signed install. Operator-accepted and SPEC-recorded — a disclosed partial. |
| SC4 | Contract/code disagreements are fixed or the contract corrected in-phase, with the discrepancy recorded, never quietly reconciled | ✓ VERIFIED (with disclosed, bounded limits) — **closed 2026-08-15, after this pass** | The mechanism is real: ~50 D-15 ledger rows with re-runnable evidence, in-place addenda for later findings, `OPEN` rows backed by greppable code markers enforced by `every_open_marker_in_code_has_a_ledger_row`, and contract↔code sync gates I proved discriminating. iter2's specific gap is closed. The three residuals this pass found were closed by quick task 260815-b0s (`511e2ab1`, `fba34606`): the `layer_force_unavailable.rs` module doc now names `ALSO_AUTOMATED` for both rows and its list assignment is machine-checked by a new sentence-scoped sibling gate proved by two perturbations; the SPEC states the D-15 scoping rule and indexes rounds 4/6/8 with finding counts (8/7/5, one Critical) and per-finding dispositions, none of which was a contract-vs-code divergence; and the stale `TODO(117-12)` is replaced with what the gate does at HEAD. **Bounded limits, stated so this row does not overclaim:** the new gate verifies the 5 module-doc claims made in sentences that name a coverage list and does NOT verify 13 further row mentions (it holds them constant by an equality pin instead); and `every_open_marker_in_code_has_a_ledger_row` still has no ledger→code direction, so an `OPEN` ledger row with no code marker would pass. |

**Score:** 3/4 truths verified on evidence, 4/4 satisfied — SC1, SC2 and SC4 verified (SC2 and SC4
with disclosed, bounded limits); SC3 accepted via an explicit operator override that leaves its
10/13 finding intact. 0 FAILED. No BLOCKER. At the time this pass was written the score was 2/4;
SC4 was closed the same day by the follow-up quick task, and this row records that rather than
re-verifying it from scratch.

### Deferred Items (operator decisions, 2026-08-14 — not counted as gaps)

| # | Item | Addressed In | Rationale still accurate at HEAD? | Greppable `OPEN` marker? | SPEC ledger row |
|---|---|---|---|---|---|
| 1 | **CR-02** — `EntryPath::Daemon` registry rows are documentation-only; `nono-agentd` does not link the registry | v3.7 carry-forward | **Yes, re-verified.** `crates/nono-cli/src/bin/nono-agentd.rs:42-57` `#[path]`-includes only `../agent_daemon/mod.rs`, `../telemetry/mod.rs`, `../agent_daemon/telemetry_init.rs` — `exec_strategy_windows` is never declared. Production `attest_and_decide` is never called with `EntryPath::Daemon`. | **Yes, as of 2026-08-15** — `⚠ CR-02 OPEN (NOT FIXED …)` at the `daemon_attest_and_decide` call site in `agent_daemon/launch.rs`. The registry binding is still only the source-text drift gate; the marker records that, it does not change it | ✓ the Iteration-6 row, disposition still "Partial: the drift gate is closed; the wiring decision is NOT taken", with Option A / Option B named and deliberately not guessed — the row now also carries the literal `OPEN` token so the code marker resolves against it, and it is the ONLY `\| CR-02 ` row that does (the Iteration-4 row shares the id and the gate cannot distinguish them) |
| 2 | **WR-12 / RF-13** — fleet-control `RequiredLayers` plumbing | v3.7 carry-forward | **Yes, re-verified.** `machine_policy.rs:716-739` reads the sub-key and emits a loud `RequiredLayersNotEnforced` warning; production gate call site `launch.rs:1585-1586` passes `&[]` for both slices, with an in-place comment explaining the union is tighten-only so the deferral cannot fail open. | **Yes, as of 2026-08-15** — `⚠ RF-13 OPEN (NOT FIXED …)` at BOTH ends: the reader in `machine_policy.rs` that emits the warning, and the gate call site in `exec_strategy_windows/launch.rs` that passes the empty slices | ✓ the `RF-13` row, "Enforcement is NOT implemented", open operator decision stated — the row now also carries the literal `OPEN` token |

Both rationales are accurate. **Updated 2026-08-15 (`fba34606`):** both items now carry greppable
`OPEN` code markers, so an operator grepping the source finds **4 of the 4** open items (WR-10,
WR-14, CR-02, RF-13) across 5 marker sites — RF-13 is annotated at both its reader and its gate
call site. The convention previously covered WR-10 (`layer_registry.rs`) and WR-14 (`error.rs`)
only. `marker_sources()` is now module-scope with a single definition and 7 entries, and
`every_marker_carrying_source_file_is_in_marker_sources` makes the file set a closed class over
`git ls-files`, so a future marker in an unlisted production file fails the build.
`every_open_marker_in_code_has_a_ledger_row` still enforces only code→ledger and has **no**
ledger→code direction: a ledger row marked `OPEN` with no code marker would still pass. That
residual is disclosed, not closed.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `proj/SPEC-windows-fail-direction-contract.md` (was 344 lines; 372 after 2026-08-15) | The one document (SC1) + D-15 discrepancy ledger (SC4) | ✓ VERIFIED for SC1, ⚠ PARTIAL for SC4 | 13-row registry table with symbol-form call sites, outcomes, probes; Manual verification (D-31) section; 52-row ledger. **Updated 2026-08-15 (`fba34606`):** the `## Contract vs. code discrepancies` section now states the D-15 scoping rule and indexes rounds 4/6/8 with finding counts and dispositions, and the `RF-13` and `CR-02 (Iteration 6)` rows carry the literal `OPEN` token. |
| `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` (1632 lines) | 13-row registry, expectancy matrix, drift gates | ✓ VERIFIED | All 13 `LayerId` rows with explicit `ContractOutcome`; `ALL` const with exhaustive-match drift trap; `daemon_expected_rows_are_all_named_by_the_daemon_gate` (:1425) with `checked >= 5` floor, proved discriminating. |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | D-21 gate + D-27/D-28 downgrade channels | ✓ VERIFIED | `apply_startup_attestation_gate` (:1552), one production call site (:2558) with fail-closed terminate; one shared `layer_detail` D-28 gate (:1617) feeding all three emission sites; `downgrade_detail_pointer` (:1477) is an exhaustive match with no `_` arm. |
| `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` | CR-01/WR-01 residue predicate + coverage accessor | ✓ VERIFIED (1 host-blocked test) | Ownership gate first (:292), `INHERIT_ONLY_ACE` rejected (:338), `AlreadyAtRequiredLevel` correctly non-reverting (:422-428), `application()` excludes the contract-exempt category from the denominator (:178-190). `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` fails loudly on this host — disclosed, needs an elevated runner. |
| `crates/nono/src/sandbox/windows.rs` | `AceFlags`-aware SACL reader + class gate | ✓ VERIFIED | `low_integrity_label_ace` returns flags; all 3 production consumers filter `INHERIT_ONLY_ACE`; `every_low_integrity_label_ace_consumer_filters_inherit_only` (:3468) scans the production half of both files with a `checked > 0` non-vacuity floor. |
| `crates/nono-cli/tests/layer_registry_selfcheck.rs` (2245 lines at this pass; 2418 after 2026-08-15) | Citation content-verification + CI sync gate + OPEN-marker gate | ✓ VERIFIED (stale comment resolved 2026-08-15) | 20/20 pass IN THE REAL TREE including the round-8 CR-01 target. `tracked_files()` resolves against `git ls-files -z` and fails CLOSED. **Updated 2026-08-15 (`511e2ab1`, `fba34606`): 21/21 in the real tree.** The stale `TODO(117-12)` is replaced with what `registry_call_sites_exist` does at HEAD; `marker_sources()` is hoisted to module scope (one definition, 7 entries, two callers); and the new `every_marker_carrying_source_file_is_in_marker_sources` closes the marker file class over `git ls-files`, with the detected-file count pinned by equality at 5. |
| `crates/nono-cli/tests/layer_registry_meta_test.rs` (639 lines at this pass; 944 after 2026-08-15) | D-32 discovery + coverage-split arithmetic + module-doc list assignment | ✓ VERIFIED as a mechanism, ⚠ 10/13 as coverage (operator-accepted) | 10/10 pass; discovery reads `LayerId::ALL` fresh from source and names no layer; `coverage_split_accounts_for_every_layer_id` enforces totals AND disjointness. Checks name existence, not test semantics. **Updated 2026-08-15 (`511e2ab1`): 11/11.** The new `module_doc_assigns_each_claimed_row_to_the_list_it_is_on` adds what the arithmetic gate is structurally blind to — WHICH list the prose names — sentence-scoped, discovery-based, 5 claims verified and 13 unverifiable mentions pinned by equality. |
| `crates/nono-cli/tests/layer_force_unavailable.rs` (207 lines at this pass; 216 after 2026-08-15) | 2 external-subprocess forced-unavailable tests | ✓ VERIFIED (module doc corrected 2026-08-15) | Both pass live under `--features layer-fault-injection`. The module doc's misassignment of 2 rows to `MANUALLY_VERIFIED` is **corrected in `511e2ab1`**: both rows are now assigned to `ALSO_AUTOMATED` in one sentence that also names each row's in-process test, and the assignment is machine-checked — see SC4. |
| `crates/nono-cli/src/cfg_test_regions.rs` (2048 lines, 81 tests) | Shared cfg-test classifier for all 3 gates | ✓ VERIFIED | 3 `scan_production` call sites, all 3 call `assert_split_is_correct`. Round-8 WR-01 fix proved discriminating by the decisive two-part perturbation. No mechanical gate forces a FUTURE consumer to call `assert_split_is_correct` — latent, noted below. |
| `crates/nono-cli/src/output.rs` | Coarse D-27 banner + D-28 digest marker | ✓ VERIFIED | Banner is count-only on every arm and takes no `silent` parameter; marker content is a domain-separated digest (:361-374); forgeability limit disclosed in the function doc. |
| `.github/workflows/ci.yml` | Gate that actually runs the fault-injection suites | ✓ VERIFIED | `windows-layer-fault-injection` (:343-388) builds `nono-shell-broker --release` then runs both feature legs with `--test-threads=1`. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `spawn_windows_child` | `apply_startup_attestation_gate` | direct call before `resume_contained_process` | ✓ WIRED | `launch.rs:2558-2575`, fail-closed with `terminate_suspended_process`. |
| `agent_daemon::launch_agent` | `daemon_attest_and_decide` | direct call | ✓ WIRED | `:925`; two-state Proceed/Abort by design (WR-06 rationale re-verified). |
| `nono-shell-broker::run` | `broker_resume_gate` | direct call before `ResumeThread` | ✓ WIRED | `main.rs:927-944`, `TerminateProcess` on `Err`. |
| Guard coverage accessors | `AppliedLayers` | `coverage().application()` / `application()` | ✓ WIRED | `mod.rs:391-449`; `map_or(NotApplied, ..)` fail-secure default when a guard never ran. |
| `low_integrity_label_ace` | residue predicate `applied` count | `(rid, mask, flags)` with `INHERIT_ONLY_ACE` rejected | ✓ WIRED, sound | iter2's unsound link is repaired and the class is gated. |
| `ProceedDowngraded` arm | operator channels | one shared `layer_detail` + `emit_downgrade_diagnostics` + banner | ✓ WIRED, D-28-safe | Layer names reach only `PrivateLogFile`; every shared-console arm is count-only. |
| Registry `(Daemon, expected:true)` rows | daemon decision | **source-text drift gate only** | ⚠ PARTIAL (deferred) | The daemon binary cannot link the registry; binding is `daemon_expected_rows_are_all_named_by_the_daemon_gate`. Operator-deferred, recorded at SPEC `:299`. |
| Machine policy `RequiredLayers` | gate input | `machine_required_layers` | ⚠ NOT WIRED (deferred) | `launch.rs:1586` passes `&[]`. Tighten-only, so not fail-open. Recorded at SPEC `:262`. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `AppliedLayers::mandatory_integrity_label` | `LabelCoverage` | Real SACL reads (`AceType`+`rid`+`mask`+`AceFlags`), residue branch now flag-aware | Yes; the over-broad branch iter2 found is closed | ✓ FLOWING |
| `AppliedLayers::dacl_ancestor_traverse` / `..._read_attrs` | `LayerApplication` | Guards' own three-arm `application()` over real walk state (D-37) | Yes | ✓ FLOWING |
| `AppliedLayers::wfp_egress_filters` | `Option<bool>` | `wfp_composition_report` over the guard's recorded `installed_filter_count` | Yes — count 0 ⇒ not confirmed, proved by a real-gate test | ✓ FLOWING |
| `AppliedLayers::firewall_rules_egress` | `Option<bool>` | `firewall_rules_report` over the guard's recorded rule count | Yes — 1-of-2 rules ⇒ not confirmed | ✓ FLOWING |
| `AttestationInput::machine_required_layers` | `&[String]` | hardcoded `&[]` at the production call site | **No** — deferred (WR-12/RF-13) | ⚠ STATIC, disclosed and tighten-only |

### Adversarial Perturbations (the anti-"passes by absence" checks)

| # | Perturbation | Target gate | Result |
|---|---|---|---|
| P1 | Change one registry citation to `AppliedDaclGrantsGuard::snapshot_and_apply_v2` (a symbol that does not exist) | `registry_call_sites_exist`, `spec_call_site_cells_match_registry_call_sites` | **BOTH FAILED** with precise messages naming the citation and the resolved file. SC1's gate is not vacuous. |
| P2 | Rename `ancestor_traverse_snapshot_and_apply_fails_when_forced_unavailable` → `..._RENAMED` | `every_registry_row_has_a_test`, `also_automated_entries_are_non_vacuous` | **BOTH FAILED**, the first quoting CINT-03's own wording. SC3's discovery is not vacuous. |
| P3 | Inject `#[cfg(test)] pub(crate) fn verifier_probe_helper() -> &'static str { "DaclAncestorTraverse" }` into `agent_daemon/launch.rs`'s production region AND rename the real production `layer = "DaclAncestorTraverse"` site | `daemon_expected_rows_are_all_named_by_the_daemon_gate` | **FAILED** — the cfg-test helper did NOT satisfy the gate. This is the decisive proof of round-9's WR-01 fix: before it, a `#[cfg(test)]`-gated non-`mod` item landed in the production half and could satisfy exactly this gate. |

All three reverted; `git status --porcelain` clean afterwards apart from the untracked
`117-VERIFICATION.iter2.md`.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Forcing MandatoryIntegrityLabel unavailable aborts a real `nono run` | `cargo test --features layer-fault-injection --test layer_force_unavailable` | 2 passed (both spawn a real `nono.exe` and assert the `LayerAttestationFailed` diagnostic) | ✓ PASS |
| The six in-crate seam regression tests fire | `cargo test --features layer-fault-injection --bin nono -- --test-threads=1 fails_when_forced_unavailable` | 6 passed | ✓ PASS |
| CI-sync gate resolves against the repository, not the disk | `cargo test --test layer_registry_selfcheck` IN THE REAL TREE (with 15 `.claude/worktrees/` + `.gsd/` present) | 20 passed / 0 failed | ✓ PASS |
| `AceFlags` is now read by the SACL reader | `grep -c AceFlags crates/nono/src/sandbox/windows.rs` | `8` (was `0` at iter2) | ✓ PASS |
| Every SACL-reader consumer filters `INHERIT_ONLY_ACE` | `every_low_integrity_label_ace_consumer_filters_inherit_only` | passes, with `checked > 0` floor | ✓ PASS |
| Debt markers in the phase's 144 changed `.rs` files | `grep -nE "\bTBD\b\|\bFIXME\b\|\bXXX\b"` | 1 hit, and it is prose *about* removing a SPEC `TBD` (`attestation.rs:1703`), not a marker | ✓ PASS |
| Formatting | `cargo fmt --all -- --check` | exit 0 | ✓ PASS |
| Full CLI binary suite | `cargo test -p nono-sandbox-cli --bin nono` | **1688 passed, 12 failed, 2 ignored** | ⚠ RED (see below) |
| Core library suite | `cargo test --workspace` (`-p nono-sandbox` leg) | 844 passed, 0 failed | ✓ PASS |

**On the 12 failures.** They are exactly the documented Windows-host baseline: 6 `config::tests`
env races, 3 `protected_paths::tests`, `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`,
`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`, and
`exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`.
No phase-117 gate failed. But I want the twelfth on the record plainly rather than folded into
"baseline": **it is a Phase 117 deliverable (the WR-20 pin), it is authored-but-unverified by
design, and it fails LOUDLY per D-31 rather than skipping.** Its consequence for SC3 is bounded
and specific — it pins the `MandatoryIntegrityLabel` row's *classification rule* for a non-owned +
foreign-labelled path, not that row's forced-unavailable test, which passes. So it does not reduce
the 10/13 row coverage. What it does mean is that `cargo test --bin nono` is RED at HEAD in this
repository, and one contracted classification rule stays unverified until an elevated or CI
Windows runner executes it.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| CINT-01 | 117-01, 03, 15, 19, 24, 26, 32 | Single fail-direction contract, derived from code, citing call sites | ✓ SATISFIED | SC1 verified. `.planning/REQUIREMENTS.md:119` still shows `[ ]`/Pending and `:186` Pending — the marking now UNDER-states the state; safe direction, worth updating at phase close. |
| CINT-02 | 117-02, 05, 08–11, 13–17, 20, 21, 23, 27–29 | Startup self-attestation; never presents an unconfirmed guarantee | ✓ SATISFIED (with the two recorded operator deferrals) | SC2 verified. `.planning/REQUIREMENTS.md:187` shows Pending — iter2's recommendation to revert the premature `[x]` was taken, and the underlying defect is now fixed. |
| CINT-03 | 117-04, 06, 07, 12, 16, 18, 22, 30, 31, 44 | Per-layer forced-unavailable test for every row; untested row = unsatisfied | ✓ SATISFIED VIA OPERATOR OVERRIDE (factually 10/13 by its own literal wording) | SC3's shortfall is accepted in this file's frontmatter `overrides` block (Oscar Mack Jr, 2026-08-15T00:00:00Z); the 10/13 finding and the three named remainder rows are unchanged. `.planning/REQUIREMENTS.md:188` Pending — deliberately not edited by the 2026-08-15 closure task; worth updating at phase close. |

**Orphaned requirements check:** `grep -n "CINT-0" .planning/REQUIREMENTS.md` returns only
CINT-01/02/03, all mapped to Phase 117. No orphans.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `crates/nono-cli/tests/layer_force_unavailable.rs` | 110-112 | Stale prose: assigns `RestrictedToken`/`JobObjectContainment` to `MANUALLY_VERIFIED` when they are on `ALSO_AUTOMATED` | ✓ RESOLVED 2026-08-15 (`511e2ab1`) | Was: under-claim direction (both rows ARE automated), so no security effect — but a live doc/code disagreement, in the same file and of the same class Iteration 4's WR-07 already fixed once, and no gate could see it. Now: the claim names `ALSO_AUTOMATED` first and both rows after it in ONE sentence, cites each row's in-process test by symbol, and is machine-checked by `module_doc_assigns_each_claimed_row_to_the_list_it_is_on`. The "no gate can see it" half is what the new gate closes. |
| `crates/nono-cli/tests/layer_registry_selfcheck.rs` | 354-358 | Stale `TODO(117-12)`: "asserts file EXISTENCE … does not verify the cited line NUMBER" | ✓ RESOLVED 2026-08-15 (`511e2ab1`) | Both halves were false at HEAD: no line-number citations remain and symbol citations ARE content-verified. Replaced with an accurate description of what `registry_call_sites_exist` does now, plus the narrower residual risk that genuinely remains (a symbol can survive while no longer performing the enforcement its row describes). No `TODO(` marker remains in the file. |
| `crates/nono-cli/src/cfg_test_regions.rs` | — | No mechanical gate requires a FUTURE `scan_production` consumer to call `assert_split_is_correct` | ℹ Info | All 3 current consumers do (verified). Latent instance of the same "guard narrower than the class" pattern this phase kept producing — in the phase's own verification machinery. Not scored. |
| `crates/nono/src/sandbox/windows.rs` | 3482-3488 | `every_low_integrity_label_ace_consumer_filters_inherit_only` scans two hardcoded files | ℹ Info | The reader is `pub`, so an out-of-workspace consumer is out of the gate's reach. The SPEC's WR-25 addendum states the enumeration explicitly (1 walk, 3 production consumers, all hardened). Not scored. |

No `TBD`/`FIXME`/`XXX` debt markers in the phase's 144 changed Rust files.

### Human Verification Required

See `human_verification` in frontmatter. Three items:

1. **A RUN of the WR-20 pin — not CI wiring** — `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`
   needs `SeTakeOwnershipPrivilege`/`SeRestorePrivilege`/`SeBackupPrivilege`. The coverage already
   exists and **no new CI wiring is required**. The test is NOT feature-gated — it is a plain
   `#[test]` at `labels_guard.rs:1188` inside `#[cfg(test)] mod tests` (declared :567-570), so it is
   one of the `nono` bin's ordinary unit tests — and CI job `windows-layer-fault-injection`
   (`.github/workflows/ci.yml:343-388`) already runs it **by construction**: its invocation at
   `ci.yml:387` carries no `--bin`/`--test`/`--lib` filter, so it runs every target in the package.
   What is missing is a RUN. The branch is **684 commits ahead of origin** (0 behind) and the last
   CI run on it, 2026-07-29, predates Phases 115/116/117 entirely, so the job has never executed
   this test. Pushing is an operator decision, which is why this stays a human item.
   **Bounding the job's evidentiary value, so it is not overstated:** the `test` job matrix
   (`ci.yml:129-131`) is `[ubuntu-latest, macos-latest]`, making `windows-layer-fault-injection` the
   ONLY job that runs Rust unit tests on Windows anywhere in CI; and because it runs the whole
   package suite, the other 11 documented Windows-host baseline failures will surface there too. The
   6 `config::tests` env races may well pass under its `--test-threads=1`, but that is a
   **PREDICTION, NOT A MEASUREMENT**. Disclosed host-blocked item, not a silent gap; it fails loudly
   with a full explanation per D-31 rather than skipping, and WR-20 remains
   **authored-but-unverified** until a real run exists, with `cargo test --bin nono` RED at HEAD
   (1688 passed / 12 failed).
2. **The 3 `MANUALLY_VERIFIED` rows' documented steps** — a code read (`DaclSessionSidGrant`), a
   structural absence (`MinifilterAbsence`), and a signed-production-install round-trip
   (`BrokerAuthenticodeTrustGate`).
3. **A completed `cargo test --workspace --no-fail-fast`** on a quiet machine, to confirm the
   workspace extras are the documented pre-existing ones (e.g. `audit_attestation::*` hardcodes
   `/bin/pwd`). The two figures that matter for this phase were measured live here.

### Suggested Override (SC3) — APPLIED 2026-08-15

SC3's shortfall is a **recorded operator decision**, not an implementation gap. The block below is
the one this report suggested; it is now **applied verbatim** in this file's frontmatter, with
`accepted_by: "Oscar Mack Jr"` and `accepted_at: "2026-08-15T00:00:00Z"`:

```yaml
overrides:
  - must_have: "Every entry in the contract has a test that forces that layer unavailable and asserts the contracted outcome — a contract row without a test is not counted as satisfied"
    reason: "10/13 rows are automated and verified. The 3 remaining rows are named, justified and SPEC-documented: DaclSessionSidGrant and MinifilterAbsence are rows for layers that do not exist in this tree (nothing to force unavailable), and BrokerAuthenticodeTrustGate is inert outside a signed production install. Accepted in the 2026-08-10 gap-closure session; enforced loud by host_gated_rows_are_loud."
    accepted_by: "<name>"
    accepted_at: "<ISO timestamp>"
```

This report deliberately did **not** apply it — SC3's wording is un-hedged in the ROADMAP, and
inventing the acceptance would have been the verifier writing its own pass. **The operator, Oscar
Mack Jr, accepted it on 2026-08-15**, and quick task 260815-b0s applied it (the `accepted_by` /
`accepted_at` fields above are the operator's, not the verifier's). What the override does and does
not do: it accepts a **disclosed** shortfall against SC3's literal wording. It does not change the
finding. SC3 is still factually 10/13; the three remaining rows — `DaclSessionSidGrant`,
`MinifilterAbsence` and `BrokerAuthenticodeTrustGate` — are still named individually here, in the
SPEC's Manual verification section, and in `MANUALLY_VERIFIED`, and are still kept loud by
`host_gated_rows_are_loud`. It is also not about the WR-20 pin: that test remains host-blocked and
`cargo test --bin nono` remains RED at HEAD.

### Gaps Summary

**The phase goal is substantially achieved.** The composite's fail-direction is now one
system-level answer: a single 13-row registry with an explicit outcome per row, one document that
transcribes it with content-verified symbol citations, one gate applied fail-closed at all three
spawn paths before any child resumes, and a layer's claim derived from what each guard actually
did rather than from whether the guard object exists. The second half of the goal — "nono can no
longer report enforcing while a layer is silently inert" — I verified end to end, live: forcing a
layer unavailable aborts a real `nono.exe` launch with a diagnostic naming the layer, and a WFP
backend that reports zero installed filters is refused by the real gate.

Both of iter2's BLOCKERs are genuinely closed, and I confirmed each against source rather than
against the fix reports. The `AceFlags` blindness that let a structurally-inert `INHERIT_ONLY_ACE`
count as coverage is fixed at the reader, at the predicate, and — the part that matters for this
phase's recurring failure mode — at the *class*, by a gate with a non-vacuity floor rather than by
a reviewer repeating a grep. The D-28 leak is closed at all three emission sites through one
shared predicate, and the marker that carried the plaintext layer set now carries a
domain-separated digest.

I also ran three adversarial perturbations rather than accepting "the test exists and passes",
because this phase's signature failure is machinery that passes by absence. All three gates failed
as required, including the decisive one: a `#[cfg(test)]`-gated helper naming a layer no longer
satisfies the daemon drift gate, which is precisely what round 8 found it doing.

Two truths were honestly partial when this pass was written. Both have since been dispositioned —
SC3 by an operator override, SC4 by closure work. The original findings are preserved below,
followed by what happened to each.

1. **SC3 is 10/13 by its own literal bar.** The three remaining rows are named, justified,
   SPEC-documented and kept loud by a test — and two of them are rows for layers that do not exist
   in this tree, so there is genuinely nothing to force unavailable. I am scoring it partial
   because the ROADMAP's wording is un-hedged and because the honest thing to do with an
   operator-accepted shortfall is surface it for an explicit override, not absorb it. Related and
   stated plainly: `cargo test --bin nono` is RED at HEAD (1688/12), and one of the 12 is a Phase
   117 test that cannot construct its precondition on this non-elevated host. It fails loudly by
   design, it does not reduce SC3's row coverage, and it needs an elevated/CI runner.

   **Disposition (2026-08-15):** the operator, Oscar Mack Jr, accepted the shortfall; the override
   is applied in this file's frontmatter. Nothing above is retracted. SC3 remains factually 10/13,
   the three named rows remain named, `cargo test --bin nono` remains RED at HEAD (1688/12), and
   the elevated/CI run of the WR-20 pin is still outstanding. The override changes the SCORING of
   a disclosed shortfall, not the shortfall.

2. **SC4's mechanism is strong but has a live instance of the very thing it forbids.** The
   phase's own coverage-declaring file states that two rows are on the manual list when they are
   on the automated one. That is the same class WR-07 fixed once in this same file, the direction
   is under-claim so nothing is over-promised, and the gate WR-07 added structurally cannot see
   it. Alongside it: the SPEC ledger has not been touched since before the round-4 review, so the
   last three review rounds — one of them Critical — have no ledger row, even though the phase set
   its own precedent with two dedicated ledger-recording waves. I judged those findings to be
   verification-machinery defects rather than contract/code divergences, and every one is recorded
   in a committed review artifact, so this is materially weaker than what iter2 found. But
   "recorded in the review artifact" is a different claim from "recorded in the standing contract",
   and SC4 names the latter.

   **Disposition (2026-08-15, quick task 260815-b0s — commits `511e2ab1` and `fba34606`):** all
   four `missing` bullets are closed. The coverage-declaring file now names the right list and its
   assignment is machine-checked by a sentence-scoped sibling gate, proved by two perturbations
   (a wrong claim FAILS naming both rows, the claimed list and the real list; a claim reworded
   back to the anaphoric "both rows" shape FAILS on both pins). The standing contract now carries
   the claim SC4 asks for: the SPEC states the D-15 scoping rule and indexes rounds 4, 6 and 8
   with their finding counts (8 / 7 / 5, one Critical) and per-finding dispositions read from the
   committed artifacts — all 20 verification-machinery, none reclassified, with round 6's WR-04
   dispositioned explicitly rather than swept in. The stale `TODO(117-12)` is replaced. And all
   four open items now carry greppable `OPEN` markers resolving to `OPEN` ledger rows.
   **What the closure does not claim:** the new gate verifies 5 module-doc claims and holds 13
   further row mentions constant by an equality pin WITHOUT verifying them (they make no
   assignment claim); `every_open_marker_in_code_has_a_ledger_row` still has no ledger→code
   direction; and markers in `tests/` are outside the new discovery gate's reach.

Neither gap was a BLOCKER, and neither prevents proceeding to Phase 118, whose dependency on this
phase is the layer enumeration — which is complete, verified, and mechanically drift-guarded. The
closure work this report scoped as "small and specific" was carried out the same day, except for
the last item: **the elevated-runner pass for WR-20 is still outstanding** and remains the phase's
one genuinely open verification task.

---

_Verified: 2026-08-15_
_Verifier: Claude (gsd-verifier)_
_HEAD: 28c866f4 · branch: milestone/v2.13-carryforward-closeout · all runs in the real working tree_
