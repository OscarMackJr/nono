---
phase: 115-v3-6-carry-forward-drain
plan: 05
subsystem: api
tags: [nono-py, pyo3, serde, denial-classification, cross-repo, maturin]

# Dependency graph
requires:
  - phase: 115-v3-6-carry-forward-drain (plan 02)
    provides: "NetworkAuditDenialCategory::ALL self-enumerating const (post-InterceptHandshakeFailed-removal enum shape)"
provides:
  - "../nono-py compiles again — both hand-written NetworkAuditDenialCategory encode/decode matches replaced with serde-driven conversion (D-10)"
  - "nono::undo::NetworkAuditDenialCategory::ALL widened from pub(crate) to pub — reachable across the nono-py path-dep boundary"
  - "../nono-py's first Rust-level #[cfg(test)] mod tests block, a self-enumerating round-trip test over NetworkAuditDenialCategory::ALL (D-12)"
  - "maturin build verified GREEN against the new codec (5th consecutive occurrence of the only-a-real-wheel-build-catches-drift lesson, this time confirming absence of drift rather than catching it)"
affects: [118 (receipts work builds on a clean denial/audit spine across both the core crate and the nono-py binding)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Serde-driven cross-repo codec: instead of a second hand-written string-match vocabulary in a binding crate, encode via serde_json::to_value(&core_enum) and decode via serde_json::from_value(Value::String(..)), so the binding can never again drift from the core enum's own #[serde(rename_all = \"snake_case\")] derive"
    - "Self-enumerating cross-crate round-trip test: a #[cfg(test)] block in the downstream binding crate iterates the upstream core crate's own pub ALL const (reached via path-dep) rather than maintaining a second variant list in the binding"

key-files:
  created: []
  modified:
    - crates/nono/src/undo/types.rs
    - ../nono-py/src/proxy.rs
    - ../nono-py/src/undo.rs

key-decisions:
  - "D-10: both hand-written NetworkAuditDenialCategory matches (encoder in proxy.rs, decoder in undo.rs) deleted; both directions now route through serde_json::to_value/from_value against the core enum's own #[serde(rename_all = \"snake_case\")] derive, preserving the exact \"invalid denial_category: {}\" error-message shape for decode failures"
  - "D-12/visibility correction: NetworkAuditDenialCategory::ALL was shipped pub(crate) by Plan 115-02 (per that plan's own spec), which made it unreachable from ../nono-py across the crate boundary — exactly the consumer D-12 names. Widened to pub in this plan (crates/nono/src/undo/types.rs) as a Rule 3 blocking-issue fix, per the prior_wave_context's explicit guidance that widening visibility is acceptable when D-11/D-12's self-enumeration intent requires it. Confirmed the widening also removes the need for the #[cfg_attr(not(test), allow(dead_code))] escape hatch — pub items are reachable from the crate's public API so rustc's dead_code analysis does not flag them; cargo build -p nono-sandbox is warning-free without it."
  - "Round-trip test placement: added to ../nono-py/src/undo.rs (the decoder's file, and the crate's first Rust test module) rather than proxy.rs, matching the plan's key_links spec (\"../nono-py/src/undo.rs test module\" -> ALL)"

patterns-established:
  - "When a downstream binding crate needs a core enum's self-enumerating const across a path-dep boundary and the const was scoped pub(crate) by a prior plan, widen it rather than duplicating the enumeration in the downstream crate — the whole point of D-11/D-12 is that only one enumeration exists"

requirements-completed: [DRAIN-02]

# Metrics
duration: ~25min
completed: 2026-08-08
---

# Phase 115 Plan 05: nono-py Denial-Category Codec Serde Migration Summary

**Deleted both of `../nono-py`'s hand-written `NetworkAuditDenialCategory` encode/decode matches, replacing them with conversion through the core enum's own `#[serde(rename_all = "snake_case")]` derive, and pinned the fix with the crate's first Rust test — a round trip driven from the core enum's own (now `pub`) `ALL` const — with a real `maturin build` proving the wheel still compiles.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-08-08T21:50:00Z
- **Tasks:** 2
- **Files modified:** 3 (1 in `nono`, 2 in `../nono-py`)

## Accomplishments

- Closed DRAIN-02/NEW-06 (blocker-class): `../nono-py` compiles again. The prior hand-written decoder raised `ValueError` on `capture_unsupported_path` and `capture_buffer_or_rewrite_failed` even though the encoder already emitted them — both matches are now gone, replaced by a single serde-driven conversion path each direction can never drift from independently.
- Restored the build broken by Plan 115-02's `InterceptHandshakeFailed` removal (expected ripple, explicitly flagged in `115-02-SUMMARY.md` as this plan's precondition to discharge).
- Added `../nono-py`'s first Rust-level `#[cfg(test)] mod tests` block: a round-trip test self-enumerated from `nono::undo::NetworkAuditDenialCategory::ALL`, so a future core-enum variant is covered the moment `../nono-py` rebuilds against it, with zero second edit required in the binding.
- `maturin build` produced a real wheel (`nono_sandbox-0.70.0-cp312-cp312-win_amd64.whl`), the fifth consecutive occurrence of this project's "only a real wheel build catches binding struct drift" lesson — this time confirming the fix, not catching a new break.
- `cargo build --workspace --all-targets` and `cargo fmt --all -- --check` in the `nono` repo both remain clean after the `ALL` visibility widening; `cargo test -p nono-sandbox --lib` still shows 826 passed (unchanged from the 115-02 baseline).

## Task Commits

**nono repo** (`milestone/v2.13-carryforward-closeout` branch):
1. **Deviation fix: widen `NetworkAuditDenialCategory::ALL` to `pub`** - `c293c0ee` (fix)

**../nono-py repo** (`44-broker-ffi-lockstep` branch):
1. **Task 1: Delete hand-written matches; replace with serde-driven conversion (D-10)** - `01ac02a` (fix)
2. **Task 2: Self-enumerating round-trip test (D-12) + maturin build** - `a37f872` (test)

_Note: the `ALL`-visibility fix landed as its own commit in the `nono` repo, ahead of Task 1 in `../nono-py`, because Task 1's decoder/encoder replacement builds fine without it (serde conversion doesn't need `ALL`), but Task 2's round-trip test requires `ALL` reachable from `../nono-py` — sequencing it first kept both repos buildable at every commit boundary._

## Files Created/Modified

- `crates/nono/src/undo/types.rs` (nono repo) - `NetworkAuditDenialCategory::ALL` widened from `pub(crate)` to `pub`; removed the now-unnecessary `#[cfg_attr(not(test), allow(dead_code))]` escape hatch; doc comments updated to record the visibility correction and its rationale
- `../nono-py/src/proxy.rs` - deleted the 11-arm hand-written encoder match; added `pub(crate) fn denial_category_to_string` routing through `serde_json::to_value`
- `../nono-py/src/undo.rs` - deleted the 9-arm-plus-fallback hand-written decoder match (already missing 2 arms — the literal NEW-06 defect); added `pub(crate) fn denial_category_from_string` routing through `serde_json::from_value`; added the crate's first `#[cfg(test)] mod tests` block with `denial_category_round_trips_every_core_variant`

## Decisions Made

- **D-10 encoder/decoder shape:** used `serde_json::to_value`/`from_value` against `serde_json::Value::String`, not `serde_json::to_string`/`from_str` with manual quote-trimming — avoids any string-surgery footgun and keeps both directions symmetric (`Value::String` in, `Value::String` out).
- **Preserved exact error-message shape:** the decoder's `PyValueError` still reads `"invalid denial_category: {}"` with the original wire string interpolated, even though the matching mechanism is now `serde_json::from_value` mapped through `.map_err(|_| ...)` rather than an explicit `other => Err(...)` fallback arm — continuity for any caller pattern-matching on the message text.
- **`ALL` visibility widening over a second hand-written list:** per the prior-wave context's explicit instruction, when `ALL` proved unreachable from `../nono-py` (confirmed by reading `115-02-SUMMARY.md`'s own next-phase-readiness note before writing any code), the correct fix was widening the core crate's visibility, not hand-writing a second variant enumeration in `../nono-py` — the latter would have directly reproduced the drift class DRAIN-02/D-11/D-12 exist to close.
- **Test module location:** `undo.rs` (per the plan's `key_links` spec pointing at `"../nono-py/src/undo.rs test module"`), even though the test exercises both the encoder (`proxy.rs`) and decoder (`undo.rs`) — the round trip needs both, and `undo.rs` already had `use super::*;`-style access to its own decoder plus a `crate::proxy::` path to the encoder.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Widened `NetworkAuditDenialCategory::ALL` from `pub(crate)` to `pub`**
- **Found during:** Task 2 planning, before writing any nono-py code — read `115-02-SUMMARY.md`'s "Next Phase Readiness" section per the plan's own `<read_first>` instruction, which explicitly flagged: *"`NetworkAuditDenialCategory::ALL` is `pub(crate)` ... Plan 115-05 cannot consume it directly from `../nono-py`."*
- **Issue:** The plan's `key_links` and Task 2's `<action>` both specify the round-trip test must iterate `nono::undo::NetworkAuditDenialCategory::ALL` directly. `pub(crate)` visibility makes that item invisible outside the `nono` crate, and `../nono-py` reaches `nono` only via a path-dependency (a separate crate compilation), not an in-workspace module — so the literal instruction was not executable as written without a core-crate change.
- **Fix:** Changed `pub(crate) const ALL` to `pub const ALL` in `crates/nono/src/undo/types.rs`. Confirmed `NetworkAuditDenialCategory` itself and the `undo` module were already `pub`, so this is a pure visibility widening with no new type or module surface. Also removed the `#[cfg_attr(not(test), allow(dead_code))]` attribute that had shielded the (formerly internal-only) const from a dead_code lint — verified empirically via `cargo build -p nono-sandbox` (clean, no warnings) that `pub` items are exempted from that lint regardless of in-crate usage.
- **Files modified:** `crates/nono/src/undo/types.rs`
- **Verification:** `cargo build -p nono-sandbox` (clean) and `cargo test -p nono-sandbox --lib undo::types` (14/14 passed, including the pre-existing `all_denial_categories_present_and_guard_covers_every_entry` guard test, unaffected by the visibility change). Then confirmed `../nono-py`'s round-trip test (Task 2) compiles and passes against the widened `ALL`.
- **Committed in:** `c293c0ee` (nono repo, separate commit ahead of the `../nono-py` task commits)

---

**Total deviations:** 1 auto-fixed (1 blocking, Rule 3)
**Impact on plan:** Necessary for the plan's own D-12 requirement to be literally satisfiable — explicitly anticipated and pre-authorized by the prior-wave context ("widening visibility in the core crate is acceptable if the plan's D-11/D-12 intent requires it, do NOT hand-write a second variant list"). No scope creep: the change is a single-line visibility keyword plus removing an attribute that visibility change made unnecessary.

## Issues Encountered

- `cargo clippy -p nono-py --all-targets -- -D warnings` fails on pre-existing findings in `../nono-py/src/override.rs` (3 `clippy::unnecessary_map_or` + 3 stale `#[expect(dead_code, ...)]` attributes whose named construction sites appear to have since landed). Confirmed via `cargo clippy -p nono-py --lib -- -D warnings` (clean) that this is a test-target-only, pre-existing issue in a file this plan did not touch (`proxy.rs`/`undo.rs` only). Logged to `.planning/phases/115-v3-6-carry-forward-drain/deferred-items.md` per the scope-boundary rule rather than fixed here.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `../nono-py` builds, tests, and packages cleanly again — the DRAIN-02/NEW-06 blocker is closed at the class level (serde-driven, not a patched second list), and `maturin build` proved the wheel is unaffected.
- `NetworkAuditDenialCategory::ALL` is now `pub` and reachable from any future path-dep consumer, not just `../nono-py` — future binding work (or `../nono-ts` if it ever grows a `NetworkAuditDenialCategory` surface, which it currently does not per 115-CONTEXT.md D-12) can reuse the same self-enumeration pattern without a second visibility fix.
- Deferred, not blocking: `../nono-py/src/override.rs`'s pre-existing clippy debt (3 `map_or` simplifications + 3 stale `#[expect(dead_code)]` attributes) — see `deferred-items.md`.

---
*Phase: 115-v3-6-carry-forward-drain*
*Completed: 2026-08-08*

## Self-Check: PASSED

All modified/created files confirmed present on disk: `crates/nono/src/undo/types.rs`
(nono repo), `../nono-py/src/proxy.rs`, `../nono-py/src/undo.rs`, plus this SUMMARY.md,
`deferred-items.md`, and the built wheel
`../nono-py/target/wheels/nono_sandbox-0.70.0-cp312-cp312-win_amd64.whl`. All 4 commit
hashes confirmed present in `git log --oneline --all`: `c293c0ee`, `06810191` (nono
repo); `01ac02a`, `a37f872` (`../nono-py` repo, branch `44-broker-ffi-lockstep`).
Plan-level verification gate re-run clean: `cargo test -p nono-py --lib` (70 passed),
`maturin build` (wheel built), `cargo build --workspace --all-targets` (nono repo,
0 errors), `cargo fmt --all -- --check` (nono repo, clean), `cargo test -p nono-sandbox
--lib` (826 passed, matching the 115-02 baseline).
