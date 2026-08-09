---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 03
subsystem: windows-composite-integrity
tags: [windows, fail-direction, layer-registry, spec-drift-check, cargo-test, cint-01]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-01's LayerId enum, ArmExpectancy/ContractOutcome/ProbeKind vocabulary, and the 13-row REGISTRY_ENTRIES in crates/nono-cli/src/exec_strategy_windows/layer_registry.rs; 117-02's NonoError::LayerAttestationFailed / RequiredLayersPolicy"
provides:
  - "proj/SPEC-windows-fail-direction-contract.md — the human-readable Windows fail-direction contract, one row per LayerId, drift-checked against the registry"
  - "crates/nono-cli/tests/layer_registry_selfcheck.rs — registry_call_sites_exist and spec_matches_registry discovery-based tests"
  - "the broker-abort-only structural constraint and the downgrade-banner per-session dedup policy, both now contract-visible, not just enforced-but-unexplained"
affects: [117-04, 117-08, 117-09, 117-10, 117-11, 117-12, 118, 119]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CARGO_MANIFEST_DIR source-scan tests (no regex, no include_str!) for self-enforcing code/doc drift checks, matching crates/nono-cli/tests/resl_supervisor_drain.rs"
    - "proj/ SPEC documents are gitignored by default (.gitignore:16) — require `git add -f`, same as the docs/cli/development/ .mdx precedent"

key-files:
  created:
    - proj/SPEC-windows-fail-direction-contract.md
    - crates/nono-cli/tests/layer_registry_selfcheck.rs
  modified: []

key-decisions:
  - "Corrected CONTEXT.md's D-12 nearest-analog citation to the verified line numbers (linux.rs:184 ABI_PROBE_ORDER, :209-230 detect_abi_uncached) rather than the plan's unverified linux.rs:308 pointer, which resolves to unrelated WSL2-detection code"
  - "Used precise anchored greps (^pub fn apply\\() rather than the plan's literal grep text where the naive pattern would have matched unrelated test function names, recording the accurate re-run hit count per D-16 instead of the plan's pre-decided number"
  - "SC4-2 and SC4-4 hit counts recorded as actually re-run on 2026-08-09 (6 and 4 respectively) rather than the RESEARCH document's stale counts (2 and 2), since code has shifted since RESEARCH was authored"

patterns-established:
  - "SPEC drift-check tests read both the registry source and the SPEC prose fresh at test-run time via CARGO_MANIFEST_DIR, and are verified non-vacuous by a live manual drift injection (documented in this SUMMARY) before being trusted"

requirements-completed: [CINT-01]

# Metrics
duration: ~35min
completed: 2026-08-09
---

# Phase 117 Plan 03: Windows Fail-Direction Contract + Drift-Check Tests Summary

**Authored `proj/SPEC-windows-fail-direction-contract.md` (13-row human-readable contract transcribed from Plan 01's registry) plus `layer_registry_selfcheck.rs`'s two discovery-based tests that keep it honest against the registry and the tree.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 2
- **Files modified:** 2 (both new)

## Accomplishments
- `proj/SPEC-windows-fail-direction-contract.md`: preamble (D-09 Windows-only), a labeled D-20 startup-only/mid-session-removal subsection, all 13 `LayerId` rows with call sites/expectancy/outcome/probe, a D-12 Unix-boundary section citing Landlock's verified best-effort ABI-downgrade mechanism, a D-24 latency-budget placeholder table (one row per D-23 gated path), a D-15 "Contract vs. code discrepancies" section with 5 SC4 rows each carrying re-runnable D-16 evidence, a "Structural constraints" section naming the Item-1 broker-abort-only invariant and its enforcing test, and a "Downgrade banner behavior" section naming the Item-2 per-session dedup policy.
- `crates/nono-cli/tests/layer_registry_selfcheck.rs`: `registry_call_sites_exist` (every `call_sites` citation resolves to a real file in the tree) and `spec_matches_registry` (every `LayerId` in the registry's `ALL` const appears by name in the SPEC — the D-01 drift gate). Both pass; both verified discovery-based by live drift injection (see Issues Encountered).

## Task Commits

Each task was committed atomically:

1. **Task 1: Author proj/SPEC-windows-fail-direction-contract.md** - `17df1f6f` (docs)
2. **Task 2: Registry self-check + SPEC drift-check tests** - `5c733b9d` (test)

_Note: Task 2 was tagged `tdd="true"` but has no companion production/behavior file — see "TDD Gate Compliance" below._

## Files Created/Modified
- `proj/SPEC-windows-fail-direction-contract.md` - the living Windows fail-direction contract (13 layer rows, D-12/D-15/D-20/D-24 sections, structural-constraints and downgrade-banner sections)
- `crates/nono-cli/tests/layer_registry_selfcheck.rs` - the registry↔SPEC drift gate and call-site existence check

## Decisions Made
- **D-12 citation correction:** CONTEXT.md's canonical_refs cited `crates/nono/src/sandbox/linux.rs:308` as the Landlock best-effort-ABI-downgrade analog. Reading that line live showed it is inside `detect_wsl2`'s env-var-trust warning, unrelated to ABI selection. The actual mechanism is `ABI_PROBE_ORDER` (`linux.rs:184`, highest-to-lowest ABI list) and `detect_abi_uncached()` (`linux.rs:209-230`, which walks that list and accepts the first supported ABI). The SPEC cites the verified lines, not the plan's unverified pointer — consistent with D-16's "greps must discover their targets, not confirm pre-named ones."
- **SC4-1 grep precision:** the plan's literal `grep -n "fn apply" crates/nono/src/sandbox/windows.rs` matches every `fn apply_accepts_*`/`fn apply_labels_*` test function name too (12 hits), not the 1 hit the plan's action text asserted. Used an anchored `^pub fn apply\(` pattern to get the accurate 1-hit citation the discrepancy row actually needs, and recorded that precision choice in the SPEC row itself.
- **SC4-2/SC4-4 hit-count re-verification:** re-running the RESEARCH document's cited greps live on 2026-08-09 produced different counts than RESEARCH recorded (`reverse-of-declaration`: 6 hits now vs. RESEARCH's 2; `NONO_TEST_HARNESS` in `mod.rs`: 4 hits now vs. RESEARCH's 2) — the code has accrued more doc-comment references since RESEARCH was authored. Recorded the live counts per D-16, not the stale RESEARCH numbers.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `extract_call_site_citations` false-positived on a doc-comment example string**
- **Found during:** Task 2, first `cargo test` run
- **Issue:** `layer_registry.rs`'s doc comment for `LayerRegistryEntry::call_sites` illustrates the citation shape with `` `"file:line"` `` — backticks wrapping an actual double-quoted string literal. The naive "any double-quoted string containing `.rs:`" extraction picked up the literal text `"file:line"` as a citation, then failed to resolve it to a real file.
- **Fix:** Narrowed the extraction to require the character immediately after the citation's final `:` to be an ASCII digit (every real citation ends in `<digits>` or `<digits>-<digits>`; the illustrative example ends in the literal word `line`). Added a regression test, `call_site_extraction_ignores_backtick_doc_comment_citations`, that includes this exact doc-comment shape in its sample input.
- **Files modified:** `crates/nono-cli/tests/layer_registry_selfcheck.rs`
- **Verification:** `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` — all 3 tests pass.
- **Committed in:** `5c733b9d` (Task 2 commit)

**2. [Rule 3 - Blocking] `proj/` is gitignored — required `git add -f` for the SPEC**
- **Found during:** Task 1 commit
- **Issue:** `.gitignore:16` excludes `proj/` wholesale; the existing `proj/ADR-*.md`/`DESIGN-*.md` files were previously force-added (matching the durable project lesson for `docs/cli/development/`). A plain `git add proj/SPEC-windows-fail-direction-contract.md` would have silently no-op'd, and `git commit` would have committed nothing.
- **Fix:** Used `git add -f proj/SPEC-windows-fail-direction-contract.md`.
- **Files modified:** none (staging-only fix)
- **Verification:** `git status --short` showed `A  proj/SPEC-windows-fail-direction-contract.md` before commit; `git log --oneline -1` and `git show --stat` confirmed the file landed in the commit.
- **Committed in:** `17df1f6f` (Task 1 commit)

**3. [Rule 3 - Blocking] Cargo package name is `nono-sandbox-cli`, not `nono-cli`**
- **Found during:** Task 2, first `cargo test -p nono-cli` invocation
- **Issue:** The plan's `<acceptance_criteria>` literally specifies `cargo test -p nono-cli --test layer_registry_selfcheck`, but the fork-owned rename (Phase 102) changed the crate's Cargo package `name` to `nono-sandbox-cli` while the directory stayed `crates/nono-cli/`. `-p nono-cli` fails with "package ID specification `nono-cli` did not match any packages."
- **Fix:** Used `-p nono-sandbox-cli` (confirmed against `Makefile`'s own `cargo test -p nono-sandbox-cli` usage). No code change — this is a command-syntax correction, not a source fix.
- **Files modified:** none
- **Verification:** `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` — 3/3 pass.
- **Committed in:** N/A (command-only, no commit needed)

---

**Total deviations:** 3 auto-fixed (1 bug, 2 blocking)
**Impact on plan:** All three were necessary corrections to make the plan's own verification commands and the test's extraction logic actually work; none altered scope. No architectural changes.

## Issues Encountered

**Live drift-injection verification (acceptance criteria, Task 2):** to confirm `spec_matches_registry` is discovery-based and not vacuously true, `LayerId::RestrictedToken` was temporarily renamed in `layer_registry.rs`. That approach was abandoned mid-verification because renaming only the `LayerId::RestrictedToken,` occurrences (not the enum variant declaration or the exhaustive match arm) breaks compilation — `cargo test` cannot even build the crate, which would test a compile failure, not the intended assertion failure. Reverted that edit (`git checkout -- crates/nono-cli/src/exec_strategy_windows/layer_registry.rs`, confirmed zero diff) and instead temporarily stripped every occurrence of the string `RestrictedToken` from `proj/SPEC-windows-fail-direction-contract.md` (a pure-text edit, no compilation impact). Re-ran `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck spec_matches_registry`: it failed with `proj/SPEC-windows-fail-direction-contract.md is missing a row for the following LayerId variant(s) ... ["RestrictedToken"]` — confirming the test discovers its target and names it, not a pre-named check. Restored the SPEC file from the pre-edit copy and confirmed `git status --short` / `git diff` show no residual change before re-running the full suite green.

## TDD Gate Compliance

Task 2 is tagged `tdd="true"` in the plan, but its `<files>` list contains only the test file itself (`crates/nono-cli/tests/layer_registry_selfcheck.rs`) — there is no companion non-test production file for this task; the task's entire deliverable *is* the self-check test suite, verifying artifacts (`layer_registry.rs`, the SPEC) that Task 1 and Plan 01 already produced. A literal RED phase (write a failing test, then write production code to make it pass) does not apply: there is no new behavior to implement, only an invariant over already-correct artifacts to assert. This matches the MVP+TDD gate's own "Behavior-Adding Task" predicate (`checks.has_source_files` requires non-test source files in `<files>`), which this task does not satisfy — it is not a behavior-adding task by that definition. A single `test(...)` commit was made; the tests passed on first run once the doc-comment false-positive (Deviation 1) was fixed. Non-vacuousness was independently verified via the manual drift injection documented above, standing in for a literal RED/GREEN cycle.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `proj/SPEC-windows-fail-direction-contract.md` and `layer_registry_selfcheck.rs` are landed and green; Plans 04/08/09/10/12 can now cite the SPEC directly and rely on `spec_matches_registry` catching any future registry/SPEC divergence.
- SC4-2 (documentation-accuracy defect in `PreparedWindowsLaunch`'s drop-order comments) and SC4-4 (the `NONO_TEST_HARNESS` runtime-gated toggle) are recorded in the SPEC as deferred to Plan 117-10 and Plan 117-04 respectively — those plans should treat the SPEC's discrepancy rows as their own acceptance criteria, not re-derive the finding.
- The latency-budget table's `TBD` placeholders are explicitly Plan 117-12's responsibility to fill in; no blocker for phases in between.
- No cross-target clippy gate applies to either file in this plan (neither is cfg-gated Unix code, under `exec_strategy/`, or under `bindings/c/src/`) — confirmed by inspection, not run, since neither file references `target_os = "linux"`/`"macos"`.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-09*
