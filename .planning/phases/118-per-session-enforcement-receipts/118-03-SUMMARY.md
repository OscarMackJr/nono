---
phase: 118-per-session-enforcement-receipts
plan: 03
subsystem: windows-attestation-census
tags: [windows, enforcement-receipts, layer-registry, census, tdd, source-scan]
dependency-graph:
  requires:
    - "crates/nono::LayerId / EntryPath / TokenArm / LayerReceiptRow / SessionOutcome / EnforcementReceipt (118-01)"
  provides:
    - "crates/nono-cli/src/exec_strategy_windows/attestation.rs::census_from_entries (pub(crate), non-early-return 13-row pass)"
    - "crates/nono-cli/src/exec_strategy_windows/attestation.rs::build_enforcement_receipt (pub(crate))"
    - "crates/nono-cli/src/exec_strategy_windows/attestation.rs::token_arm_to_receipt (pub(crate) WindowsTokenArm -> nono::TokenArm map)"
    - "layer_registry::LayerId / layer_registry::EntryPath as re-exports of nono::LayerId / nono::EntryPath"
  affects:
    - "crates/nono/src/lib.rs (EntryPath/TokenArm added to the flat re-export)"
    - "crates/nono-cli/tests/layer_registry_meta_test.rs and layer_registry_selfcheck.rs (ALL-const discovery marker repointed at core)"
tech-stack:
  added: []
  patterns:
    - "census pass built as a SEPARATE, non-early-return loop alongside an existing early-return decision function (Research Finding 1 / Pitfall 4)"
    - "discovery-based source-scan proof for a property (no early return, registry-driven) that a runtime call cannot check from outside the crate (tests/*.rs has no [lib] target to link against)"
    - "thin tests/*.rs documentation-shim file pointing at an inline #[cfg(test)] test, with its own non-vacuous signpost assertion"
key-files:
  created:
    - crates/nono-cli/tests/receipt_sentinel_roundtrip.rs
  modified:
    - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
    - crates/nono-cli/src/exec_strategy_windows/attestation.rs
    - crates/nono-cli/tests/layer_registry_meta_test.rs
    - crates/nono-cli/tests/layer_registry_selfcheck.rs
    - crates/nono/src/lib.rs
decisions:
  - "layer_registry::EntryPath is now also a pub(crate) use nono::EntryPath re-export (orchestrator directive extending Task 1's LayerId-only plan text) — same-shaped variant set, zero call-site breakage, and the operator explicitly rejected keeping two copies of a receipt-shaped type in sync."
  - "layer_registry::ALL resolves directly to nono::LayerId::ALL (a const pointing at the core associated const) instead of a second hand-maintained 13-entry array, per the plan's explicit 'do not leave a second, possibly-divergent ALL const' instruction."
  - "build_enforcement_receipt does NOT use a string-literal entry_path_label helper as the plan's original interfaces text specified — that instruction predates 118-01's post-plan correction, which retyped EnforcementReceipt.entry_path/.token_arm from &'static str to the core EntryPath/TokenArm enums. entry_path needs no conversion (same type, post Task 1's re-export); token_arm goes through a small exhaustive token_arm_to_receipt(WindowsTokenArm) -> nono::TokenArm match instead."
metrics:
  duration: "~2h"
  completed: 2026-08-17
---

# Phase 118 Plan 03: Windows Attestation Census Summary

Gave `nono.exe` a real, complete 13-row per-session census: `census_from_entries` walks the same
registry `decide_from_entries` walks, but with no `return` inside its loop, so a row that would
make the decision function abort on the spot is still recorded — and every row after it is still
probed. Paired with `build_enforcement_receipt` (the census -> `EnforcementReceipt` assembler), the
D-14 sentinel round-trip proof, and a census-completeness drift guard. `LayerId` and `EntryPath` are
now consumed from `crates/nono` core rather than locally redefined in `layer_registry.rs`.

## What Was Built

**Task 1 (`49118105`)** — `feat(118-03): LayerId/EntryPath core re-export + census_from_entries pass`

- `layer_registry.rs`: replaced the local `pub(crate) enum LayerId { .. }` (13 variants) and
  `pub(crate) enum EntryPath { DirectCli, Broker, Daemon }` with `pub(crate) use nono::LayerId;` /
  `pub(crate) use nono::EntryPath;` re-exports. `layer_registry::ALL` now resolves directly to
  `nono::LayerId::ALL` (`pub(crate) const ALL: &[LayerId] = LayerId::ALL;`) instead of a second,
  hand-maintained 13-entry array. Verified every existing `LayerId::Variant` / `EntryPath::Variant`
  call site across the crate (123 / 118 occurrences respectively, post-change) keeps compiling
  unchanged — confirmed by `cargo check -p nono-sandbox-cli --all-targets` passing clean.
- `crates/nono/src/lib.rs`: added `EntryPath`, `TokenArm` to the flat `pub use receipt::{...}`
  re-export (Rule 3 fix — `layer_registry.rs`'s new `nono::EntryPath` re-export could not resolve
  without it; `nono::EntryPath` existed as a type in `receipt.rs` but was never flat-exported from
  `lib.rs` by 118-01's post-plan correction).
- `attestation.rs`: `census_from_entries(entries, input) -> Vec<nono::LayerReceiptRow>` — copies
  `decide_from_entries`'s `for entry in entries { let verdict = classify_row(entry, input); ... }`
  loop shape exactly, but with **no `return` anywhere inside the loop**: every row is pushed
  unconditionally, every iteration, for all 13 rows every time. `build_enforcement_receipt(census,
  session_id, pid, entry_path, token_arm, outcome) -> nono::EnforcementReceipt` assembles the final
  receipt; `token_arm_to_receipt(WindowsTokenArm) -> nono::TokenArm` is a small, exhaustive
  (no-wildcard-arm) match mirroring `assert_all_layer_ids_covered`'s drift-guard style. Neither
  function is wired into `apply_startup_attestation_gate` (Plan 118-07's job).
- 8 new tests in `attestation.rs`'s existing `#[cfg(test)] mod tests` block (the file has a `mod
  tests { .. }` / `mod registry_tests { .. }` / `mod broker_wire_contract_tests { .. }` / `mod
  latency_measurement { .. }` sibling-module layout, not a `tests`-nests-everything shape — the new
  tests were placed inside `mod tests`, matching where their fixtures/imports already live):
  - `census_from_entries_returns_all_13_rows_even_when_the_first_row_would_abort` — the real
    13-row registry, `(EntryPath::DirectCli, WriteRestricted)`, a null `child_process` handle. This
    makes `decide_from_entries` abort at the very first row (`RestrictedToken`,
    `LiveTokenOrJobQuery`, `Abort`) since `probe_restricted_sids` on a null handle fails closed —
    `census_from_entries` still returns 13 rows.
  - `decide_from_entries_still_aborts_while_census_from_entries_stays_complete` — the perturbation
    proof (Pitfall 4): for the IDENTICAL input, `decide_from_entries` still returns
    `Abort { layer: RestrictedToken, status: Unconfirmed }` (byte-identical to its pre-plan
    behavior) while `census_from_entries` stays a full 13-row census.
  - `every_census_row_matches_an_independent_classify_row_call` — every census row's `status`
    equals a freshly, independently computed `classify_row` result — no stale/cached values.
  - `census_from_entries_never_returns_early_on_a_synthetic_single_row_abort` — same property on a
    minimal synthetic fixture, not only the full production registry.
  - `token_arm_to_receipt_maps_every_windows_token_arm_variant` — all 5 `WindowsTokenArm` variants.
  - `build_enforcement_receipt_assembles_a_full_receipt_from_a_census` — full field-by-field
    assembly check.
  - `sentinel_seeded_session_sid_never_leaks_into_the_serialized_receipt` — the D-14 positive
    sentinel proof (see Task 2).

**Task 2 (`1815c93`)** — `test(118-03): repoint ALL-const discovery scans at core + census meta-test`

- **Sentinel round-trip (D-14 half 2), positive proof:** placed inline in `attestation.rs`'s
  `mod tests` (per this plan's `<interfaces>` guidance — `nono-cli` has no `[lib]` target, so a
  `tests/*.rs` file cannot call the `pub(crate)` `census_from_entries`/`build_enforcement_receipt`
  directly). Seeds `AttestationInput::expected_session_sid` with
  `"C:\Users\SENTINEL-TOKEN-7f3a\secret-project"`, builds a full receipt through the real
  `census_from_entries` -> `build_enforcement_receipt` pipeline, serializes it with
  `serde_json::to_string`, and asserts neither `"SENTINEL-TOKEN-7f3a"` nor `"secret-project"`
  appears anywhere in the output.
- **Negative-control demonstration (not committed — per this task's own instruction and
  VALIDATION.md: "demonstrated in review, not committed"):** temporarily added
  `pub raw_session_sid: String` to `crates/nono/src/receipt.rs`'s `EnforcementReceipt`, temporarily
  threaded `input.expected_session_sid` into it via a temporary extra parameter on
  `build_enforcement_receipt`, and re-ran the sentinel test. It **FAILED** as expected — the
  serialized JSON output included `"raw_session_sid":"C:\\Users\\SENTINEL-TOKEN-7f3a\\secret-project"`
  verbatim (full panic transcript captured during execution). Both the temporary field and the
  temporary parameter/call-site changes (in `receipt.rs` and `attestation.rs`, plus the 3 struct
  literals in `receipt.rs`'s own test module that needed the new field populated to keep compiling)
  were then reverted in full — confirmed via `git diff` showing zero changes to `receipt.rs` after
  the revert, and the full 47-test `attestation` suite passing again unchanged.
- **Census-completeness meta-test extension — Rule 1 bug fix first.** Task 1's change to
  `layer_registry.rs`'s `ALL` const (now `= LayerId::ALL`, no literal array) broke
  `layer_registry_meta_test.rs`'s and `layer_registry_selfcheck.rs`'s `extract_all_layer_id_names`,
  which searched for the now-nonexistent literal marker `"const ALL: &[LayerId] = &["` in
  `layer_registry.rs`'s source text (`layer_registry_selfcheck.rs::spec_matches_registry` FAILED
  with exactly this panic before the fix, confirmed live). Both functions now read
  `crates/nono/src/receipt.rs` (D-12's real source of truth for the array) via a new
  `read_core_layer_id_module()` helper, with the marker updated to
  `"pub const ALL: &'static [LayerId] = &["`. `layer_registry_meta_test.rs`'s now-unused
  `read_layer_registry()` was removed (its only 3 call sites all moved to the new reader; CLAUDE.md
  forbids `#[allow(dead_code)]`).
- **Net-new meta-test:** `census_from_entries_has_no_early_return_and_is_registry_driven` in
  `layer_registry_meta_test.rs` — a discovery-based source scan (brace-depth-tracked extraction of
  `census_from_entries`'s function body from `attestation.rs`'s source text, read fresh on every
  run) asserting the body contains no `return` keyword and does iterate `entries` directly (not a
  hardcoded per-`LayerId` match). This complements, rather than duplicates, the pre-existing
  `all_entries_covers_every_layer_id` registry-side guard — the net-new claim is that the census
  *pass itself* stays exhaustive, not just the registry data it walks. **Perturbation-proved:** a
  temporary `return census;` inserted inside the loop made this test FAIL with the exact expected
  panic message (captured during execution), then was reverted; the test passed again afterward.
- **`crates/nono-cli/tests/receipt_sentinel_roundtrip.rs`:** created as the thin documentation shim
  the plan's `<interfaces>` block anticipated (rather than leaving the frontmatter-listed path
  unaddressed). Not a duplicate of the real test — one non-vacuous signpost test
  (`the_real_sentinel_round_trip_test_lives_in_attestation_rs`) source-scans `attestation.rs` and
  confirms the real sentinel test function still exists, immediately preceded by a real `#[test]`
  attribute (not merely a doc-comment mention), and that the literal sentinel substring is present
  — so this file cannot pass vacuously if the real test is ever renamed or removed.

## Verification

- `cargo test -p nono-sandbox-cli --bin nono exec_strategy::attestation` — **47 passed**, 0 failed
  (full pre-existing suite + 9 new tests; `decide_from_entries`'s own pre-existing tests are
  byte-unchanged, only new tests were added).
- `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` — **12 passed**, 0 failed (11
  pre-existing + 1 new `census_from_entries_has_no_early_return_and_is_registry_driven`).
- `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck` — **21 passed**, 0 failed
  (confirms the `read_core_layer_id_module()` repoint fixed `spec_matches_registry`, which failed
  before this plan's fix).
- `cargo test -p nono-sandbox-cli --test receipt_sentinel_roundtrip` — **1 passed**, 0 failed.
- `cargo test -p nono-sandbox --lib receipt::` — **5 passed**, 0 failed (unchanged from 118-01;
  confirms `receipt.rs` itself is byte-identical to 118-01's shipped state after the negative-control
  revert).
- `cargo check -p nono-sandbox-cli --all-targets` — exit 0.
- `cargo fmt --check` — clean (workspace-wide, after `cargo fmt --all`).
- `cargo clippy -p nono-sandbox -p nono-sandbox-cli --all-targets -- -D warnings -D
  clippy::unwrap_used` — clean.
- `grep -c "enum LayerId" crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` — **0**.
- `grep -c "enum EntryPath" crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` — **0**.
- Full `cargo test -p nono-sandbox-cli --all-targets` (background run, ~133s): **1695 passed, 12
  failed, 2 ignored**. The 12 failures are pre-existing baseline noise unrelated to this plan's
  files (`audit_session`, `config::tests::*` env-lock `PoisonError` from parallel test-thread
  pollution, `exec_strategy::labels_guard` host-privilege-gated, `profile_cmd`, `protected_paths`) —
  none touch `layer_registry.rs`, `attestation.rs`, `receipt.rs`, or `lib.rs`, and this class is
  already documented as pre-existing in `nono_cli_windows_baseline_test_failures.md` (prior figure
  11; today's run measured 12 under this host's current thread-timing — logged as an updated data
  point, not chased as a regression, per the SCOPE BOUNDARY rule).

## TDD Gate Compliance

Task 1 was tagged `tdd="true"`, but the executor committed the new `census_from_entries` /
`build_enforcement_receipt` / `token_arm_to_receipt` production code together with their tests in
one commit (`49118105`) rather than a separate RED (failing test against a stub) / GREEN
(implementation) commit pair. **No `test(...)` commit precedes a `feat(...)` commit for Task 1** —
the RED/GREEN gate sequence was not followed as a literal two-commit protocol.

Mitigating context: every new assertion in this plan carries its own perturbation proof performed
live during execution (not merely asserted) — the census-vs-decision cross-check, the sentinel
round-trip's negative control, and the meta-test's `return census;` injection all failed as
expected before being fixed/reverted — so the "prove the test can fail" discipline was honored in
substance even though the commit-boundary literal RED/GREEN gate was not. Flagging this per the
TDD compliance instruction rather than silently claiming full compliance.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] `nono::EntryPath`/`TokenArm` missing from the flat `lib.rs` re-export**
- **Found during:** Task 1, immediately after adding `layer_registry.rs`'s
  `pub(crate) use nono::EntryPath;`.
- **Issue:** `cargo check -p nono-sandbox-cli --bin nono` failed with `E0432: unresolved import
  nono::EntryPath — no EntryPath in the root`. `crates/nono/src/receipt.rs` defines `pub enum
  EntryPath` and `pub enum TokenArm` (118-01's post-plan correction), but `lib.rs`'s `pub use
  receipt::{...}` only flat-exported `EnforcementReceipt, LayerId, LayerReceiptRow,
  SessionOutcome` — `EntryPath`/`TokenArm` were left off that list.
- **Fix:** Added `EntryPath, TokenArm` to `lib.rs`'s `pub use receipt::{...}` list.
- **Files modified:** `crates/nono/src/lib.rs`
- **Commit:** `49118105`

**2. [Rule 1 - Bug] Task 1's `ALL`-const change broke two pre-existing discovery-scan tests**
- **Found during:** Task 2, running the full test suite after Task 1's changes settled.
- **Issue:** `layer_registry_meta_test.rs` and `layer_registry_selfcheck.rs` both define an
  `extract_all_layer_id_names(src: &str)` helper that searches `layer_registry.rs`'s source text
  for the literal marker `"const ALL: &[LayerId] = &["` and parses the `LayerId::Variant,` entries
  between it and `];`. Task 1 replaced that literal array with
  `pub(crate) const ALL: &[LayerId] = LayerId::ALL;` (per the plan's own explicit "do not leave a
  second, possibly-divergent ALL const" instruction), so the marker no longer matches and both
  functions panic. Confirmed live: `layer_registry_selfcheck.rs::spec_matches_registry` failed with
  `expected to find "const ALL: &[LayerId] = &[" in layer_registry.rs ...` before this fix.
- **Fix:** Added `read_core_layer_id_module()` to both files, reading
  `crates/nono/src/receipt.rs` (D-12's real source of truth for the array) instead, and updated the
  marker to `"pub const ALL: &'static [LayerId] = &["` matching that file's declaration. Updated
  all 4 call sites (3 in `layer_registry_meta_test.rs`, 1 in `layer_registry_selfcheck.rs`) and
  removed `layer_registry_meta_test.rs`'s now-unused `read_layer_registry()` (CLAUDE.md forbids
  `#[allow(dead_code)]`; `layer_registry_selfcheck.rs`'s own `read_layer_registry()` stayed — it is
  still used by 5 other tests in that file that need the whole registry, not just `ALL`).
- **Files modified:** `crates/nono-cli/tests/layer_registry_meta_test.rs`,
  `crates/nono-cli/tests/layer_registry_selfcheck.rs`
- **Commit:** `1815c93`

### Orchestrator-Directed Deviations

**3. `layer_registry::EntryPath` also re-exported from core (not only `LayerId`)**
- **Directive:** the orchestrator's prompt explicitly extended Task 1's plan text (which only
  named `LayerId` for the core re-export) to also cover `EntryPath` — "Do the same for `EntryPath`
  in the same task": delete the local `pub(crate) enum EntryPath { DirectCli, Broker, Daemon }` and
  replace it with `pub(crate) use nono::EntryPath;`.
- **Rationale (per the orchestrator):** the operator explicitly rejected keeping two copies of a
  receipt-shaped type in sync, and core's `EntryPath` has an identical variant set, so every
  existing `EntryPath::Variant` call site keeps compiling unchanged.
- **Scope respected:** `WindowsTokenArm` in `launch.rs` was explicitly NOT touched, per the same
  directive — that unification belongs to Plan 118-07, which owns that file.
- **Verified:** grepped every `EntryPath::`/`layer_registry::EntryPath` use site
  (`crates/nono-cli/src`) before and confirmed `cargo check -p nono-sandbox-cli --all-targets`
  compiles clean after.

### Plan-Text Supersession (not a deviation from intent — the plan's own instruction predates a later correction)

**4. No `entry_path_label` string-literal helper**
- The plan's `<interfaces>`/`<tasks>` text (written before 118-01's post-plan correction) specified
  writing `fn entry_path_label(p: layer_registry::EntryPath) -> &'static str` because
  `EnforcementReceipt.entry_path`/`.token_arm` were originally typed `&'static str`/`Option<&'static
  str>`. 118-01's SUMMARY.md documents an operator-decided correction (taken at orchestration time,
  before this plan executed) retyping both fields to the core `EntryPath`/`Option<TokenArm>` enums.
  This plan's prior-wave-context block confirmed this explicitly. `build_enforcement_receipt`
  therefore assigns `entry_path` directly (identical type, after Task 1's re-export makes
  `layer_registry::EntryPath` literally `nono::EntryPath`) and maps `token_arm` through
  `token_arm_to_receipt(WindowsTokenArm) -> nono::TokenArm` instead of building a string literal.

## Cross-Target Clippy Inventory (for Plan 118-10)

Files touched by this plan that carry a `#[cfg(target_os = ...)]` gate (CLAUDE.md's cross-target
clippy MUST applies to the whole file, not just the touched lines):

| File | Gate(s) present | What this plan changed there |
|------|------------------|-------------------------------|
| `crates/nono/src/lib.rs` | `#[cfg(target_os = "windows")]` (x3), `#[cfg(target_os = "linux")]` (x1) | Added `EntryPath, TokenArm` to a platform-neutral `pub use` list (not itself cfg-gated, but the file as a whole qualifies) |
| `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` | `#[cfg(target_os = "windows")]` on `all_entries()`; doc-comment mentions of linux/macos cfg (not real gates) | `LayerId`/`EntryPath` re-exports and `ALL` const — all platform-neutral declarations, not cfg-gated themselves |
| `crates/nono-cli/src/exec_strategy_windows/attestation.rs` | No real `#[cfg(target_os)]` gate in this file (one doc-comment mention only); the whole `exec_strategy_windows` module tree is `#[cfg(target_os = "windows")] #[path = ...]`-included from `main.rs` | `census_from_entries`, `build_enforcement_receipt`, `token_arm_to_receipt`, 9 new tests |
| `crates/nono-cli/tests/layer_registry_meta_test.rs` | `#![cfg(target_os = "windows")]` (whole-file gate) | `read_core_layer_id_module()`, marker fix, new meta-test |

`crates/nono-cli/tests/layer_registry_selfcheck.rs` and `crates/nono-cli/tests/receipt_sentinel_roundtrip.rs`
carry no `cfg(target_os)` gate. Neither local cross-target gate (`cross` linux-gnu,
`cargo-zigbuild` apple-darwin) was run by this plan — deferred to Plan 118-10 per the phase's
verification strategy (`118-VALIDATION.md` line 53: "Before `/gsd:verify-work`: ... both cross-target
clippy gates").

## Self-Check: PASSED

```
FOUND: crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
FOUND: crates/nono-cli/src/exec_strategy_windows/attestation.rs
FOUND: crates/nono-cli/tests/receipt_sentinel_roundtrip.rs
FOUND: crates/nono-cli/tests/layer_registry_meta_test.rs
FOUND: crates/nono-cli/tests/layer_registry_selfcheck.rs
FOUND: crates/nono/src/lib.rs
FOUND commit: 49118105
FOUND commit: 1815c93
```

## Threat Flags

None. This plan's changes are exactly the surface its own `<threat_model>` names
(T-118-08/T-118-09/T-118-10) — `census_from_entries` calling `classify_row` fresh per row (not
reusing `decide_from_entries`'s discarded verdicts), the sentinel-leak surface on
`expected_session_sid`, and the two independent-loop design keeping `census_from_entries` and
`decide_from_entries` from sharing mutable state. No new network endpoint, auth path,
file-access pattern, or schema change at a trust boundary was introduced. The `lib.rs` re-export
addition (Deviation 1) widens *visibility* of an already-existing, already-reviewed core type
(118-01 shipped `EntryPath`/`TokenArm` as `pub` in `receipt.rs`); it does not introduce new
attack surface.

## Known Stubs

None. `census_from_entries` and `build_enforcement_receipt` are complete, functioning
implementations — not wired into `apply_startup_attestation_gate` yet (explicitly out of scope for
this plan, assigned to Plan 118-07), but that is a documented forward-reference to a later plan's
deliverable, not a stub left unfinished in this plan's own scope.
