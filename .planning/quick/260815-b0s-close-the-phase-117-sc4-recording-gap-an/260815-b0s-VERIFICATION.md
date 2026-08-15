---
phase: quick/260815-b0s-close-the-phase-117-sc4-recording-gap-an
verified: 2026-08-15T00:00:00Z
status: human_needed
score: 8/8 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Decide whether `117-VERIFICATION.md`'s frontmatter `status: passed_with_accepted_risk` stands, or is changed to `human_needed` (or reverted to `gaps_found`)."
    expected: "An operator-chosen value. Evidence for the decision: (a) `passed_with_accepted_risk` appears nowhere else as a PHASE-VERIFICATION status in this repo — the other 11 phase VERIFICATION.md files use only `passed` (7) or `human_needed` (4); every existing use of the token is a MILESTONE-AUDIT status (`v3.6-MILESTONE-AUDIT.md`, `MILESTONES.md`, `STATE.md`). (b) The file's own frontmatter still carries 3 non-empty `human_verification` items and a `missing` bullet reading 'STILL OPEN: an elevated/CI Windows run of non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap'. Under the GSD verifier decision tree, a non-empty human_verification section forces `human_needed`."
    why_human: "This is a records/scoring judgement the operator owns, not a codebase fact. The executor self-flagged it as its one out-of-plan edit and explicitly invited reversion. The file's PROSE is honest either way — only the machine-readable token is at issue."
  - test: "Run `cargo test -p nono-sandbox-cli --bin nono non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` from an ELEVATED Windows session or CI runner holding SeTakeOwnershipPrivilege / SeRestorePrivilege / SeBackupPrivilege."
    expected: "The test constructs a non-owned + foreign-labelled path and passes, converting the WR-20 pin from authored-but-unverified to verified."
    why_human: "Host-blocked on this non-elevated domain account. Pre-existing; explicitly OUT OF SCOPE for this task and correctly left disclosed as STILL OPEN in both `gaps` and `human_verification`."
---

# Quick Task 260815-b0s Verification Report

**Task Goal:** Close the Phase 117 SC4 recording gap (all four `missing` bullets) and apply the operator-accepted SC3 override — without softening any disclosed finding.
**Verified:** 2026-08-15
**Status:** human_needed (8/8 must-haves verified; two operator decisions surfaced)
**Tree:** REAL TREE `C:\Users\OMack\Nono`, no worktree. Base `1ded1130`, HEAD `868cd5f4` on `milestone/v2.13-carryforward-closeout`. `git status --porcelain` at start and end: only the untracked quick-task directory.

## Goal Achievement

### Observable Truths (plan `must_haves.truths`)

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | Module doc names `ALSO_AUTOMATED` (not `MANUALLY_VERIFIED`) for `RestrictedToken`/`JobObjectContainment`, states the real coverage shape, keeps the host/harness writeup + PowerShell repro | ✓ VERIFIED | `layer_force_unavailable.rs:109-127` read in full. Claim sentence names the list FIRST, both rows AFTER, in one sentence, and cites each row's in-process test by symbol. The investigation writeup (4-attempt × 45s stall) and both isolated PowerShell reproductions survive verbatim, reframed as "supplementary manual evidence … not the rows' coverage of record". |
| 2 | A machine gate FAILS on a wrong list assignment — sentence-scoped, nearest-preceding, 5 claims at HEAD, 3 of them in the `Of the 13 LayerId rows` paragraph; proved by TWO perturbations | ✓ VERIFIED — reproduced independently, plus a third of my own | See "Perturbations I ran myself" below. Rule written once in the test's doc comment and implemented once. Discovery-based: rows from `extract_all_layer_id_names(&read_layer_registry())`, list names from `stringify!` paired with the consts' own members — no layer named literally. |
| 3 | The unverifiable row mentions are pinned at their exact HEAD count by equality | ✓ VERIFIED — count derived independently, not read out of the test | `assert_eq!(unverified.len(), 13, …)`. My own hand-derivation of the `//!` region against `LayerId::ALL` gives 13 (10 + 2 + 1); the gate's own dump under perturbation (c) enumerated exactly that composition. Pin discriminates in BOTH directions (I proved the increase direction; the executor proved the decrease direction, and I reproduced its cause). |
| 4 | Every tracked `crates/*/src/**.rs` carrying an `OPEN` marker is in `marker_sources()`, asserted by repo-wide discovery over `tracked_files` | ✓ VERIFIED — count derived independently | `every_marker_carrying_source_file_is_in_marker_sources` present and passing. I re-implemented `line_carries_marker_tokens` + `is_finding_id` in Python over `git ls-files -z -- crates/` and got **194 candidates, 5 detected**, exactly the five the gate pins. `marker_sources()` has ONE definition, at module scope (`layer_registry_selfcheck.rs:1075`); `grep -c 'fn marker_sources'` = 1. `/src/`-only scope limit and the `tests/`-out-of-reach consequence are stated in the test's own doc comment. |
| 5 | No stale `TODO(117-12)`; `registry_call_sites_exist`'s doc describes HEAD | ✓ VERIFIED | Zero `TODO(` / `TBD` / `FIXME` / `XXX` / `HACK` / `PLACEHOLDER` in all six modified source files. New doc describes the raw-form residual branch, `content_defines_symbol`'s three checks, both non-vacuity floors, and states a NARROWER residual risk (a symbol can survive while no longer enforcing) rather than restating the old wrong one. |
| 6 | SPEC states the D-15 scoping rule and indexes rounds 4/6/8 with finding COUNTS and real per-finding dispositions | ✓ VERIFIED | Scoping statement + bulleted index added before the `\| # \| Finding \|` table. Counts **8 / 7 / 5 = 20** independently confirmed by enumerating `### WR-\|CR-` headings in the three review artifacts. Honesty condition checked in depth — see below. |
| 7 | All FOUR `OPEN` marker ids resolve to a real ledger row; CR-02's `OPEN` on the Iteration-6 row only | ✓ VERIFIED | 5 marker sites / 4 ids (WR-14, WR-10, CR-02, RF-13×2), zero `WR-12` markers. Per-row `OPEN` counts measured programmatically: `\| CR-02 ` line 331 = **0**, line 354 (Iteration 6) = **2**; `\| WR-10 ` 341 = 0, 355 = 2; `\| WR-14 ` 346 = 0, 356 = 2; `\| RF-13 ` 317 = 2 (single row); `\| WR-12 ` 344 = 0. Exactly one row per id carries the token, and in every colliding case it is the Iteration-6 one. |
| 8 | `117-VERIFICATION.md` records the SC3 override without softening the 10/13 finding, the named 3-row remainder or the WR-20 disclosure, and reports SC4 honestly | ✓ VERIFIED on substance (see WARNING on the frontmatter `status:` token) | Per-token before/after, one `grep -c` each: no count decreased; `10/13` went 8 → 14. Override applied **byte-identical** to the file's own Suggested Override block (`must_have` MATCH, `reason` MATCH, verified by YAML-parsing both and comparing). SC4's three residual limits stated in four places. |

**Score:** 8/8 must-haves verified.

### Perturbations I ran myself (independent of the executor's)

Reproduced in the real tree, each reverted with `git checkout --` and `git status --porcelain` confirmed clean of it afterwards.

| # | Perturbation | Result | Discrimination observed |
|---|---|---|---|
| (a) | `layer_force_unavailable.rs:110`, `ALSO_AUTOMATED` → `MANUALLY_VERIFIED` | **FAILED** at `layer_registry_meta_test.rs:730` | Named **both** rows, the **claimed** list, the **real** list, and quoted the offending sentence — all four. Reproduces the executor's report exactly. |
| (b) | Same line, `list:` → `list.` (splits the claim into a list-name-only sentence + a row-names-only sentence — the anaphoric fail-open shape) | **FAILED** at `layer_registry_meta_test.rs:741` | Claims floor fired: `only 3 … expected at least 5`, listing the surviving 3. Confirms verified 5 → 3. |
| (c) | **Mine, not the executor's** — inserted `` `WfpEgressFilters` `` into a listless sentence at line 120 | **FAILED** on the equality pin | `left: 14 / right: 13`, and dumped the full unverified list, which matched my hand-derived composition exactly (10 in the both-lists sentence, 2 in the section heading, 1 in the quoted `LayerAttestationFailed` diagnostic) + my injected one. This proves the pin discriminates in the INCREASE direction too, which neither perturbation (a) nor (b) exercises. |

**The pinned numbers are the real numbers.** I derived 5 and 13 by hand from the `//!` region against `LayerId::ALL` (13 variants, read from `layer_registry.rs:1107-1121`) and against the two consts' actual membership (`MANUALLY_VERIFIED` = `DaclSessionSidGrant`, `MinifilterAbsence`, `BrokerAuthenticodeTrustGate`; `ALSO_AUTOMATED` contains `RestrictedToken` and `JobObjectContainment`) BEFORE reading the pin. Both matched. Neither pin is set to "whatever the implementation emitted".

### Item 4's honesty condition — checked against every one of the 20 findings

The scoping statement was **not** used to sweep a contract-vs-code divergence out of the ledger. I resolved the target of all 20 findings to a compile region, rather than accepting the SUMMARY's table:

| Target | Findings | Compile region at HEAD | Disposition |
|---|---|---|---|
| `crates/nono-cli/src/cfg_test_regions.rs` | R4 WR-01, WR-03; R6 WR-01, WR-02, WR-03; R8 WR-01, WR-02, WR-04 (**8**) | `main.rs:162` declares `#[cfg(test)] mod cfg_test_regions;` — the **entire module** is test-only, compiled out of release | machinery ✓ |
| `crates/nono-cli/tests/*` | R4 WR-05; R6 WR-05, WR-06; R8 CR-01, WR-03 (**5**) | integration-test crate | machinery ✓ |
| `layer_registry.rs:1425-1468` | R4 WR-02 | inside `#[cfg(all(test, target_os = "windows"))] mod tests` at `:1167` | machinery ✓ |
| `main.rs:465-487` | R4 WR-04 | inside `#[cfg(test)] mod tests` at `:322`; the fn is now at `main.rs:602` | machinery ✓ |
| `output.rs:1560-1631`, `:1666-1706`, `:2139-2147` | R4 WR-06, WR-08; R6 WR-07 (sweep) | inside `#[cfg(all(test, target_os = "windows"))] mod attestation_marker_path_tests` at `:1530` and `#[cfg(test)]` at `:1820` | machinery ✓ |
| `.github/workflows/ci.yml` | R4 WR-07 | CI wiring | machinery ✓ |
| `exec_strategy_windows/launch.rs` `collect_files` | **R6 WR-04** | **`fn collect_files` at `:4701`, nested inside `#[test] fn downgrade_marker_files_never_contain_a_layer_name` at `:4680`** — the D-28 leak-scan gate, not the confinement path | machinery ✓ |

Round-6 **WR-04** verified on evidence, not on the executor's word: the review's own text says "in the same test, thirty lines up" and "the gate reports a clean D-28 result over a subtree it never opened", and at HEAD the enclosing `#[test]` is three lines above the fn. It is a machinery defect with real security significance — which is still a machinery defect. (Executor cited `:4690`; actual `:4701`. Immaterial line drift.)

Round-8 **CR-01** verified: the finding is wholly about `every_markdown_file_gated_by_a_test_runs_the_code_jobs` in `tests/layer_registry_selfcheck.rs` resolving discovery against the disk instead of `git ls-files`. Its own text closes with "this is a broken developer/`make ci` build rather than a broken pipeline".

Round-6 **WR-07** verified: its two production-line citations (`output.rs:274-282`, `:303-312`) are **doc-comment claims about a test's reach** — the finding is literally "the *record* is once more stronger than the *check*".

**No finding among the 20 is a contract-vs-code divergence.** Corroborating evidence that the ledger was not being used defensively: the CR-02 (Iteration 6) row **does** carry a real contract-vs-code divergence in its own text ("`DaclAncestorTraverse` at `(Daemon, None)` declares `Abort`; the daemon warns and proceeds") — it has a row, not an index line. The scoping statement itself closes with "A finding that is **both** gets a row here as well."

### Item 6 — every disclosure survived, checked per token

One `grep -c` per token against `1ded1130` and HEAD. **No count decreased.**

| Token | Before | After |
|---|---|---|
| `Oscar Mack Jr` | 0 | 5 |
| `2026-08-15T00:00:00Z` | 1 | 5 |
| `DaclSessionSidGrant` | 4 | 7 |
| `MinifilterAbsence` | 4 | 7 |
| `BrokerAuthenticodeTrustGate` | 6 | 8 |
| `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` | 7 | 8 |
| `1688` | 4 | 6 |
| `10/13` | 8 | 14 |
| `overrides_applied: 1` | 0 | 1 |

- The 10/13 finding survives and is **restated as still factually true** in five places (score, SC3 gap `reason`, SC3 truths row, CINT-03 requirements row, Gaps Summary item 1's Disposition paragraph).
- All three remainder rows are named **individually**, not as a count.
- The WR-20 pin is disclosed in **both** `gaps` (SC3 `missing` bullet 2, "STILL OPEN") and `human_verification` (item 1).
- The RED statement survives: "`cargo test --bin nono` is RED at HEAD (1688 passed / 12 failed)" in `score`, and "1688 passed / 12 failed" in `human_verification`.
- Frontmatter parses as valid YAML; `overrides[0].accepted_by = "Oscar Mack Jr"`, `accepted_at = 2026-08-15T00:00:00Z`, `overrides_applied: 1`.

### Item 7 — SC4's new status is honest

All three residual limits the executor claims to disclose are actually present, in four independent locations (frontmatter `score`, the SC4 gap `closure` block's "WHAT IS STILL NOT COVERED" paragraph, the SC4 row of the truths table, and Gaps Summary item 2's "What the closure does not claim"):

1. the gate verifies 5 claims and only PINS the other 13;
2. `every_open_marker_in_code_has_a_ledger_row` still has **no ledger→code direction**;
3. markers in `tests/` are out of the discovery gate's reach by design.

Bullet 4's closure is also correctly qualified as "CLOSED via the marker alternative … The ledger→code direction was NOT built and remains absent". SC4 is not overclaimed.

### Item 9 — no production confinement behaviour changed

```
git diff 1ded1130..HEAD -U0 -- crates/nono/src/machine_policy.rs \
  crates/nono-cli/src/exec_strategy_windows/launch.rs \
  crates/nono-cli/src/agent_daemon/launch.rs \
  | grep -E '^[+-]' | grep -vE '^(\+\+\+|---)' | grep -vE '^[+-][[:space:]]*//'
```
→ **0 lines**; the negated form exits **0**. I also read the full (non-`-U0`) diff of all three files: every added line is a `//` or `///` comment, no line sits inside a string literal or block comment. No guard, gate decision, or attestation path touched. Cross-target clippy not applicable — `grep -c 'target_os = "linux"'` and `'target_os = "macos"'` are both **0** in all three files, none is under `exec_strategy/` or `bindings/c/src/`, and comment-only edits cannot alter a cfg branch regardless.

### Test Gates

| Gate | Result | Status |
|---|---|---|
| `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` | 21 passed / 0 failed | ✓ (includes `every_marker_carrying_source_file_is_in_marker_sources`, `every_open_marker_in_code_has_a_ledger_row`, `the_wr14_open_record_matches_the_actual_swallow_sites`, `every_spec_symbol_citation_resolves_to_a_real_definition`, `every_markdown_file_gated_by_a_test_runs_the_code_jobs`) |
| `… --test layer_registry_meta_test` | 11 passed / 0 failed | ✓ (includes `module_doc_assigns_each_claimed_row_to_the_list_it_is_on`, `host_gated_rows_are_loud`) |
| `… --test layer_force_unavailable` (default) | 0 tests (feature-filtered) | ✓ expected |
| `… --features layer-fault-injection --test layer_force_unavailable` | 2 passed / 0 failed | ✓ |
| `cargo fmt --all -- --check` | exit 0 | ✓ |
| `cargo clippy -p nono-sandbox-cli --test layer_registry_meta_test --test layer_registry_selfcheck -- -D warnings -D clippy::unwrap_used` | exit 0 | ✓ |
| SPEC ledger integrity | 52 contiguous rows after the single `\| # \| What was wrong` header (floor 40); exactly one such header | ✓ |
| `git diff --stat 1ded1130..HEAD -- .planning/REQUIREMENTS.md .planning/ROADMAP.md .planning/STATE.md` | empty | ✓ |

**Not run, stated plainly:** `cargo test -p nono-sandbox-cli --bin nono` (the documented 1688/12 Windows-host baseline) and `-p nono-sandbox`. The plan does not require them, no production code changed, and long suites stall ~25 min on this host. I did **not** verify the 1688/12 figure myself — I verified only that the claim survives unsoftened in the record.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| all six modified source files | — | `TBD` / `FIXME` / `XXX` / `TODO(` / `HACK` / `PLACEHOLDER` | none — **0 occurrences** | Debt-marker gate clean. The `TODO(117-12)` this task was to remove is gone. |
| `117-VERIFICATION.md` | frontmatter `status:` | `passed_with_accepted_risk` on a PHASE verification | ⚠ Warning | See below. |

## WARNING — the one item that needs an operator call

**`117-VERIFICATION.md`'s `status:` was changed from `gaps_found` to `passed_with_accepted_risk`.** The executor self-flagged this as its one out-of-plan edit and invited reversion. My judgement, stated plainly as asked:

**It overstates, and its stated justification does not hold.** Two independent reasons:

1. **The token is not phase-verification vocabulary in this project.** `grep -rn "passed_with_accepted_risk" .planning/` returns 9 hits: `v3.6-MILESTONE-AUDIT.md`, `v3.6-REQUIREMENTS.md`, `v3.6-ROADMAP.md`, `MILESTONES.md`, `PROJECT.md`, `RETROSPECTIVE.md`, `STATE.md` (all **milestone-audit** contexts), this file, and the SUMMARY. Across the repo's 12 phase `*-VERIFICATION.md` files the status distribution is `passed` ×7, `human_needed` ×4, and this one. The SUMMARY's claim that it is "a value already used in this project's vocabulary" is true only for a *different artifact type*.
2. **The file's own contents force a different value.** The frontmatter still carries **3 non-empty `human_verification` items** and a `gaps[0].missing[1]` bullet that literally begins "STILL OPEN". Under the GSD verifier decision tree, a non-empty human-verification section makes `human_needed` mandatory and `passed`-family values invalid.

**Severity: WARNING, not BLOCKER.** No disclosure was softened — the prose is scrupulous, and the WR-20 pin, the RED suite and the 10/13 shortfall are all stated more prominently after the edit than before. The risk is confined to a machine-readable token that a downstream consumer keying off `status:` alone could read as "no action required" while `gaps` and `human_verification` both still hold live items.

**Recommendation:** change `status:` to `human_needed`. That is consistent with `overrides_applied: 1`, with four closed SC4 bullets, and with the honest fact that one verification task (the elevated WR-20 run) is still outstanding. Reverting to `gaps_found` would be defensible but slightly understates, since all four SC4 bullets really are closed.

## Gaps Summary

No must-have gaps. All eight plan truths verified against the codebase, with the two most stub-prone claims (the list-assignment gate's discrimination, and the marker-source discovery count) re-derived independently rather than read out of the implementation, and the highest-risk honesty claim (item 4's "all 20 are machinery") resolved to a compile region for every one of the 20 findings rather than spot-checked.

Two items require a human decision: the frontmatter `status:` token above, and the pre-existing, out-of-scope elevated-runner pass for the WR-20 pin.

---

_Verified: 2026-08-15_
_Verifier: Claude (gsd-verifier) — all commands run in the REAL TREE at `C:\Users\OMack\Nono`_
