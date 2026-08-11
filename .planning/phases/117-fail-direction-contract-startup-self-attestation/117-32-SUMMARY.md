---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 32
subsystem: testing
tags: [rust, integration-tests, source-scanning, markdown-parsing, wr-19]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "Plan 117-31's hardened content_defines_symbol / is_ident_boundary / nearest_preceding_impl_names, reused unchanged by this plan's whole-document citation resolver"
provides:
  - "spec_call_site_cells_match_registry_call_sites — a discovery-based test comparing each SPEC Layer-registry-table row's citation cell against layer_registry.rs's own per-entry call_sites array"
  - "every_spec_symbol_citation_resolves_to_a_real_definition — a whole-document scan resolving every file.rs::Symbol-shaped backtick citation anywhere in the SPEC, not only the Layer registry table"
affects: [117-gap-closure-round-4, any-future-plan-editing-proj/SPEC-windows-fail-direction-contract.md-or-layer_registry.rs]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-entry source segmentation (split on a repeated marker, e.g. \"id: LayerId::\") to scope an extraction helper to one registry row's own text, preventing a later row's citations from being misattributed to an earlier one"
    - "Markdown table cell extraction by header-row anchor + positional split('|') indexing, reusing existing no-regex backtick/quote-pair scanning helpers rather than a table parser"
    - "Citation-shape validator (looks_like_a_spec_citation) that distinguishes real file.rs::Symbol citations from the document's own two escape hatches: quoted illustrative examples and English-prose trailing-call-parens mentions"

key-files:
  created: []
  modified:
    - proj/SPEC-windows-fail-direction-contract.md
    - crates/nono-cli/tests/layer_registry_selfcheck.rs

key-decisions:
  - "The plan text said to resolve whole-document citations 'via resolve_citation_path' — that function rsplit_once(':')s on the citation string, which mis-splits a \"file.rs::Symbol\" citation containing an embedded \"::\" (the two-colon pair's second colon wins the rsplit, producing a garbage \"file.rs:\" file part). Used the already-established split_symbol_citation + resolve_file_part pair instead — the exact pattern registry_call_sites_exist already uses for symbol-form citations — documented here as a Rule 1 correction to the plan's own wording, not a deviation from its intent."
  - "Building the whole-document scan surfaced 4 non-registry-table SPEC citations that did not already resolve cleanly: two carried a trailing call-parens English-prose convention (apply(), applied_layers()) excluded on purpose by the shape validator (a real citation names a bare symbol, never Symbol()); one (main.rs) used a bare filename that is NOT under exec_strategy_windows/ (main.rs lives at the crate root); one (broker_authenticode.rs) named a test file under crates/nono-cli/tests/, a location resolve_file_part's four-way prefix convention does not cover for bare filenames. Rather than widening resolve_file_part's resolution heuristics (more surface for a future ambiguous match), normalized the citations themselves to the document's own established conventions — strip the parens, and use the already-supported workspace-relative crates/ prefix for both the main.rs and the test-file case — keeping the resolver's four branches exactly as Plan 117-31 left them."
  - "A second unquoted 'file.rs::Symbol' illustrative-example instance (WR-08 row) was found one paragraph below the one already correctly quoted at NR3-08 — evidence this document's own quoting convention for illustrative examples has itself drifted inconsistently before. Quoted it to match, closing a second, smaller instance of the same citation-hygiene issue this whole plan exists to prevent."

patterns-established:
  - "Shape-validate before scanning: a naive 'does this backtick/quote span contain .rs::' substring test is not sufficient for a hand-written prose document (unlike layer_registry.rs's own machine-consistent call_sites arrays) — English prose legitimately contains near-miss shapes (Symbol(), quoted illustrative examples) that a stricter character-class + segment-count validator must reject before the resolver ever runs, or the gate produces false failures on valid document text rather than true failures on drifted citations."

requirements-completed: [CINT-01]

# Metrics
duration: ~50min
completed: 2026-08-11
---

# Phase 117 Plan 32: SPEC Citation Drift Gate (Call-Site Cells + Whole-Document Symbol Scan) Summary

**Fixed WR-19's stale `BrokerAuthenticodeTrustGate` citation and closed the structural hole that let it (and two prior citation-drift rounds) survive undetected: a new drift gate compares every SPEC Layer-registry-table row's citation cell against `layer_registry.rs`'s own `call_sites` array, and a second gate content-verifies every `file.rs::Symbol` citation anywhere in the SPEC document, not only inside that one table.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-08-11 (first Read)
- **Completed:** 2026-08-11
- **Tasks:** 2/2 completed
- **Files modified:** 2 (`proj/SPEC-windows-fail-direction-contract.md`, `crates/nono-cli/tests/layer_registry_selfcheck.rs`)

## Accomplishments

- Fixed the `BrokerAuthenticodeTrustGate` Manual-verification row's stale `launch.rs:2190`/`:2194` citation (unrelated `InitializeProcThreadAttributeList` buffer-sizing code) to the symbol form `launch.rs::is_dev_build_layout`, confirmed by direct read against the real call sites (`launch.rs:1822`, `:2151`) and declaration (`launch.rs:2537`) — line numbers had drifted from the plan text's own cited `:1800`/`:2129`/`:2515` by ~20 lines from intervening work in this same gap-closure round, re-derived live rather than trusted.
- Added `spec_call_site_cells_match_registry_call_sites`: segments `layer_registry.rs`'s source per entry (split on `"id: LayerId::"`, 13 occurrences, one per row) and extracts each entry's OWN `call_sites: &[...]` array content from its own narrowed slice; separately parses the SPEC's `## Layer registry` Markdown table by anchoring on its header row and reading each data row's `LayerId`-name and "Enforcing call site(s)" cells; asserts the two sets are equal per row, naming the specific `LayerId` on mismatch. All 13 rows matched on first run — the table was already accurate; the gate itself was the missing artifact, not a hidden drift in the table (WR-19 was scoped to the Manual-verification section, which the table-comparison gate does not cover — that is the second gate's job).
- Added `every_spec_symbol_citation_resolves_to_a_real_definition`: scans the FULL SPEC document text (not scoped to the Layer registry table) for every backtick-wrapped citation matching a `file.rs::Symbol` shape, via a new `looks_like_a_spec_citation` validator that requires the file half to be `/`-separated path segments of identifier/hyphen/dot characters ending in `.rs`, and the symbol half to be one or two `::`-separated identifier segments with no other characters — rejecting the document's two legitimate near-miss shapes (quoted illustrative examples like `` `"file.rs::Symbol"` ``, and English-prose mentions with trailing call-parens like `` `apply()` ``) before they reach the resolver. Resolves each surviving citation via the existing `split_symbol_citation` + `resolve_file_part` + `content_defines_symbol` pipeline (Plan 117-31's hardened matcher, reused unchanged).
- Found and fixed 4 non-registry-table citations that the new whole-document scan's stricter shape check exposed as needing normalization (not previously checked by anything): `crates/nono/src/sandbox/windows.rs::apply()` → `::apply` (SC4-1), `mod.rs::applied_layers()` → `::applied_layers` (NR3-02 row), `main.rs::render_error_for_operator()` → `crates/nono-cli/src/main.rs::render_error_for_operator` (WR-04 row — bare `main.rs` is not under `exec_strategy_windows/`), and the Manual-verification section's `broker_authenticode.rs::broker_signature_mismatch_refuses_spawn` → `crates/nono-cli/tests/broker_authenticode.rs::...` (a test file, not a src file — bare-filename resolution would have pointed at a nonexistent `exec_strategy_windows/broker_authenticode.rs`).
- Found and fixed a second unquoted `file.rs::Symbol` illustrative-example instance (WR-08 row, one paragraph below the already-correctly-quoted NR3-08 instance) by wrapping it in the document's own established quoted-example convention, so the new whole-document scan does not false-fail against a placeholder name that was never meant to be a real citation.

## Task Commits

1. **Task 1: Fix the stale Manual-verification citation** - `49151c0e` (docs)
2. **Task 2: Build the SPEC-vs-registry call-site drift gate and widen the symbol scan to the whole document** - `ca982654` (test)

_No separate plan-metadata commit — this SUMMARY.md commit follows immediately below (STATE.md/ROADMAP.md are orchestrator-owned per this repo's project-specific override and are not touched by this executor)._

## Files Created/Modified

- `proj/SPEC-windows-fail-direction-contract.md` - Fixed the `BrokerAuthenticodeTrustGate` Manual-verification citation (Task 1); normalized 4 non-registry-table citations and quoted a second illustrative-example instance so the new whole-document scan resolves cleanly (Task 2, all necessary corrections surfaced by building the test).
- `crates/nono-cli/tests/layer_registry_selfcheck.rs` - Added `looks_like_a_spec_citation`, `extract_spec_citations`, `registry_citations_by_layer_id`, `spec_layer_registry_table_citations` (helpers), and the two new tests `spec_call_site_cells_match_registry_call_sites` / `every_spec_symbol_citation_resolves_to_a_real_definition`. Reformatted by `cargo fmt` (two multi-line expressions).

## Decisions Made

- **Deviated from the plan's literal "resolves each via `resolve_citation_path`" instruction.** `resolve_citation_path` splits on the LAST `:` character, which for a `"file.rs::Symbol"` citation lands on the second colon of the `::` pair, producing a garbage `"file.rs:"` file part. Used the pre-existing `split_symbol_citation` + `resolve_file_part` pair instead — the exact pattern `registry_call_sites_exist` already uses for symbol-form citations in the registry-source scan. [Rule 1 — plan-text correction, verified before implementing]
- **Normalized 4 out-of-table SPEC citations rather than widening `resolve_file_part`'s resolution heuristics.** Two carried an English-prose trailing-`()` convention that the new shape validator deliberately excludes (a real `file.rs::Symbol` citation names a bare symbol); the other two used bare filenames outside `resolve_file_part`'s four covered locations (crate-root `main.rs`; a `tests/`-directory file). Fixing the citations to the document's own established conventions (strip parens; use the already-supported `crates/`-prefix form) kept the resolver's proven four-branch shape exactly as Plan 117-31 left it, rather than adding new prefix-matching surface for a one-off case. [Rule 1 — bug the new test's construction surfaced]
- **Quoted a second stray unquoted `file.rs::Symbol` illustrative example** (WR-08 row) to match the document's own established quoted-example convention (already correct one paragraph above, at NR3-08) — confirms this specific citation-hygiene lapse had already happened twice independently before this plan, reinforcing the round-3-discipline premise that a name-only drift gate cannot see this class of error. [Rule 1]

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan-text bug] `resolve_citation_path` cannot resolve symbol-form citations**
- **Found during:** Task 2 design (before writing the test)
- **Issue:** The plan's `<action>` text says the new whole-document test "resolves each via `resolve_citation_path`" — that function is line-form-only (`rsplit_once(':')`); applying it to a `"file.rs::Symbol"` string mis-splits on the second colon of the `::` pair.
- **Fix:** Used `split_symbol_citation` (splits on the FIRST `::`) + `resolve_file_part`, matching the pattern `registry_call_sites_exist` already established for symbol-form citations.
- **Files modified:** `crates/nono-cli/tests/layer_registry_selfcheck.rs`
- **Verification:** `every_spec_symbol_citation_resolves_to_a_real_definition` passes against all 37 real citations in the live document.
- **Committed in:** `ca982654`

**2. [Rule 1 - Bug surfaced by the new test] Four non-registry-table SPEC citations did not resolve cleanly**
- **Found during:** Task 2, first run of `every_spec_symbol_citation_resolves_to_a_real_definition` against the unmodified SPEC text (dry run via a standalone Node.js reproduction before writing the Rust test — see Perturbation Proofs below for why this matters)
- **Issue:** `crates/nono/src/sandbox/windows.rs::apply()` and `mod.rs::applied_layers()` carry a trailing-`()` English-prose convention my shape validator (correctly) rejects as not a `file.rs::Symbol` citation; `main.rs::render_error_for_operator()` used a bare filename that resolves to the wrong directory (`main.rs` is not under `exec_strategy_windows/`); the Manual-verification section's `broker_authenticode.rs::broker_signature_mismatch_refuses_spawn` named a test file, which bare-filename resolution incorrectly pointed at a nonexistent src-tree path.
- **Fix:** Stripped the trailing `()` from the first two; added the `crates/nono-cli/src/` and `crates/nono-cli/tests/` workspace-relative prefixes (both already-supported forms) to the latter two.
- **Files modified:** `proj/SPEC-windows-fail-direction-contract.md`
- **Verification:** All 37 citations resolve; confirmed via a standalone Node.js reproduction of the resolver logic before porting to Rust, then confirmed again by the passing Rust test.
- **Committed in:** `ca982654`

**3. [Rule 1 - Documentation-hygiene bug] Second unquoted illustrative-example citation**
- **Found during:** Task 2, same dry-run pass as #2 above
- **Issue:** The WR-08 discrepancy-ledger row contained an unquoted `` `file.rs::Symbol` `` illustrative example (a placeholder, not a real citation) — a second instance of the exact citation-hygiene lapse the already-correct NR3-08 row (one paragraph above) had already been fixed for.
- **Fix:** Wrapped it in the document's own established quoting convention: `` `"file.rs::Symbol"` ``.
- **Files modified:** `proj/SPEC-windows-fail-direction-contract.md`
- **Verification:** `every_spec_symbol_citation_resolves_to_a_real_definition` no longer attempts to resolve `file.rs`/`Symbol` (a nonexistent placeholder pair).
- **Committed in:** `ca982654`

---

**Total deviations:** 3 auto-fixed (1 plan-text correction, 2 bugs the new test's own construction surfaced)
**Impact on plan:** All three were necessary for the new whole-document scan to be correct rather than either vacuous (silently skipping real citations) or falsely failing (rejecting the document's own legitimate prose conventions). No scope creep — every touched line was a citation the plan's own stated scope ("every `file.rs::Symbol` citation appearing ANYWHERE in the SPEC document") already covered.

## Perturbation Proofs

**Proof 1 — line-form reintroduction (WR-19's original stale citation) confirms line-form drift is genuinely out of THIS gate's scope, by design, not by accident.**

Per the plan's stated contingency: reintroduced the exact pre-fix stale citation shape into the `BrokerAuthenticodeTrustGate` row and re-ran the new whole-document test:

```
$ node -e "... replace \`launch.rs::is_dev_build_layout\` with \`launch.rs:2190\`/\`:2194\`) ..."
reverted to stale line-form citation
$ cargo test -p nono-sandbox-cli --test layer_registry_selfcheck -- every_spec_symbol_citation_resolves_to_a_real_definition spec_call_site_cells_match_registry_call_sites
running 2 tests
test spec_call_site_cells_match_registry_call_sites ... ok
test every_spec_symbol_citation_resolves_to_a_real_definition ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.07s
```

Confirmed: line-form citations (`` `launch.rs:2190` ``, no `::`) do not match the `file.rs::Symbol` shape this gate targets — they are a structurally different, pre-existing citation convention `registry_call_sites_exist`'s own TODO comment already documents as a known, separately-scoped risk (file-existence-only, not content-verified). This is not a gap in this plan's gate; it is the documented reason `registry_call_sites_exist`'s line-form branch and this plan's symbol-form whole-document scan are two different tests with two different scopes. Reverted immediately after confirming:

```
$ node -e "... restore \`launch.rs::is_dev_build_layout\` ..."
restored correct symbol-form citation
$ grep -c "launch.rs:2190" proj/SPEC-windows-fail-direction-contract.md
0
```

**Proof 2 — synthetic malformed `file.rs::NoSuchSymbol` citation, per the plan's fallback instruction, confirms the gate DOES fire on a real symbol-form drift.**

Injected a synthetic malformed citation into the document (under the Latency budget heading, clearly marked):

```
$ grep -n NoSuchSymbolXyz proj/SPEC-windows-fail-direction-contract.md
90:PERTURBATION-PROOF: a synthetic malformed citation, `launch.rs::NoSuchSymbolXyz`, for this plan's perturbation proof.
$ cargo test -p nono-sandbox-cli --test layer_registry_selfcheck -- every_spec_symbol_citation_resolves_to_a_real_definition
running 1 test
test every_spec_symbol_citation_resolves_to_a_real_definition ... FAILED

thread 'every_spec_symbol_citation_resolves_to_a_real_definition' panicked at crates\nono-cli\tests\layer_registry_selfcheck.rs:724:5:
proj/SPEC-windows-fail-direction-contract.md cites a file.rs::Symbol that does not resolve to a real definition anywhere in the document (Manual verification section, discrepancy ledger, or any other section — not only the Layer registry table):
"launch.rs::NoSuchSymbolXyz" -> resolved to C:\Users\OMack\Nono\crates\nono-cli\src\exec_strategy_windows\launch.rs, but its content does not define "NoSuchSymbolXyz" at a real definition site (only a comment, string, or macro literal mention, if any)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.02s
```

Reverted immediately after confirming:

```
$ grep -c NoSuchSymbolXyz proj/SPEC-windows-fail-direction-contract.md
0
```

**Proof 3 — registry `call_sites` array edited without updating the SPEC table cell — confirms `spec_call_site_cells_match_registry_call_sites` fires naming the specific `LayerId`.**

Added an extra citation to `JobObjectContainment`'s `call_sites` array in `layer_registry.rs` without touching the SPEC's corresponding table cell:

```
$ cargo test -p nono-sandbox-cli --test layer_registry_selfcheck -- spec_call_site_cells_match_registry_call_sites
running 1 test
test spec_call_site_cells_match_registry_call_sites ... FAILED

thread 'spec_call_site_cells_match_registry_call_sites' panicked at crates\nono-cli\tests\layer_registry_selfcheck.rs:671:5:
SPEC Layer registry table citations have drifted from layer_registry.rs's own call_sites (WR-19's structural fix — the prior spec_matches_registry test compared LayerId names only and could never have caught this):
JobObjectContainment: registry call_sites ["launch.rs::create_process_containment", "launch.rs::apply_process_handle_to_containment", "agent_daemon/launch.rs::create_agent_job", "agent_daemon/launch.rs::assign_process_to_agent_job", "mod.rs::prepare_live_windows_launch"] does not match the SPEC's Enforcing call site(s) cell ["launch.rs::create_process_containment", "launch.rs::apply_process_handle_to_containment", "agent_daemon/launch.rs::create_agent_job", "agent_daemon/launch.rs::assign_process_to_agent_job"]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
```

Reverted immediately after confirming; re-ran the full file to confirm no residual diff:

```
$ git diff --stat
 crates/nono-cli/tests/layer_registry_selfcheck.rs | 276 ++++++++++++++++++++++
 proj/SPEC-windows-fail-direction-contract.md      |  10 +-
 2 files changed, 281 insertions(+), 5 deletions(-)
$ cargo test -p nono-sandbox-cli --test layer_registry_selfcheck
running 11 tests
... (all 11 ok, see Full Test Run below)
```

The `git diff --stat` above confirms only the intended Task 1 + Task 2 changes remain — no leftover perturbation.

## Full Test Run (this plan's test file, after all perturbations reverted)

```
$ cargo test -p nono-sandbox-cli --test layer_registry_selfcheck
running 11 tests
test call_site_extraction_ignores_backtick_doc_comment_citations ... ok
test content_defines_symbol_accepts_a_qualified_type_method_citation ... ok
test content_defines_symbol_accepts_a_real_definition ... ok
test content_defines_symbol_distinguishes_same_named_methods_in_different_impls ... ok
test content_defines_symbol_rejects_a_doc_comment_mention ... ok
test content_defines_symbol_rejects_a_prefix_preserving_rename ... ok
test spec_call_site_cells_match_registry_call_sites ... ok
test spec_matches_registry ... ok
test symbol_citation_extraction_finds_the_eight_converted_citations ... ok
test registry_call_sites_exist ... ok
test every_spec_symbol_citation_resolves_to_a_real_definition ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

`layer_registry_meta_test.rs` (untouched by this plan, re-run for regression confirmation): 9/9 passed.

## Verification Gate Results

- `cargo build --workspace --all-targets` — clean (only the pre-existing, unrelated `nono-shell-broker` "missing a lib target" advisory warning).
- `cargo fmt --check` — one deviation: `cargo fmt --all` reformatted two multi-line boolean expressions in `layer_registry_selfcheck.rs`'s new `looks_like_a_spec_citation` and `registry_citations_by_layer_id` helpers to match `rustfmt`'s canonical wrapping; re-ran `cargo fmt --check` afterward — clean.
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — clean (exit 0), no new `#[allow]` needed.
- `cargo test -p nono-sandbox-cli --workspace` — **1624 passed, 12 failed, 2 ignored.** All 12 failures match the documented baseline exactly:
  - 11 pre-existing baseline failures (`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`, `config::tests::nono_home_dir_rejects_non_absolute_override`, `config::tests::nono_home_dir_falls_through_when_unset`, `config::tests::nono_home_dir_returns_override_when_set`, `config::tests::test_validated_home_falls_back_to_userprofile`, `config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists`, `config::tests::user_state_dir_uses_localappdata_on_windows`, `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`, `protected_paths::tests::blocks_child_directory_capability`, `protected_paths::tests::blocks_parent_directory_capability`, `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root`).
  - 1 host-blocked failure from Plan 117-30's intentional D-31 test: `exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap` (this shell lacks `SeTakeOwnershipPrivilege`, per the dependency note's stated baseline).
  - No new regressions. This plan's own changed test binary (`layer_registry_selfcheck`) is 100% green (11/11); `layer_registry_meta_test` (untouched) remains 100% green (9/9).
- No cross-target clippy gate applies to this plan — both modified files (`proj/SPEC-windows-fail-direction-contract.md`, a Markdown document, and `crates/nono-cli/tests/layer_registry_selfcheck.rs`, a `tests/*.rs` file) fall outside D-35's canonical gate commands, which do not pass `--all-targets`/`--tests` and therefore never compile `tests/*.rs` files regardless of `cfg` gates (same rationale 117-31-PLAN.md's `<cross_target_rationale>` already established for this same test file).

## Issues Encountered

None beyond the three deviations documented above, all resolved inline during Task 2's construction.

## Next Phase Readiness

WR-19 is closed: the stale `BrokerAuthenticodeTrustGate` citation is fixed, and the structural hole that let it (and NR3-08/WR-08's two prior citation-drift rounds) survive is closed by two new gates — one comparing the Layer registry table's citation cells against the registry's own source of truth, one content-verifying every `file.rs::Symbol` citation anywhere in the document. Both proven load-bearing via live perturbation, in both directions (SPEC-side drift and registry-side drift), plus a documented scope boundary (line-form citations remain a separately-scoped, pre-existing risk). No further action needed on this item for gap-closure round 3; STATE.md/ROADMAP.md updates are owned by the orchestrator per this repo's project-specific override and are not touched by this executor.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*

## Self-Check: PASSED

- FOUND: `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-32-SUMMARY.md`
- FOUND: `proj/SPEC-windows-fail-direction-contract.md`
- FOUND: `crates/nono-cli/tests/layer_registry_selfcheck.rs`
- FOUND: commit `49151c0e` (Task 1)
- FOUND: commit `ca982654` (Task 2)
- FOUND: commit `bbae0235` (SUMMARY.md)
