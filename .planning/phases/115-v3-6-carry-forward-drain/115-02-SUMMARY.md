---
phase: 115-v3-6-carry-forward-drain
plan: 02
subsystem: api
tags: [audit, rust, denial-classification, nono-proxy, serde, compile-time-guard]

# Dependency graph
requires:
  - phase: 115-v3-6-carry-forward-drain (plan 01)
    provides: CustomCredentialDef.inject_mode/inject_header as Option<T> (no file overlap with this plan)
provides:
  - "audit::log_denied requires a NetworkAuditDenialCategory as a non-Option positional argument — omission is a compile error"
  - "All 6 deny_domain dispatch paths (connect.rs, external.rs, reverse.rs x3, server.rs) emit HostDenied"
  - "external.rs's enterprise-proxy-rejected site emits ExternalProxyRejected"
  - "NetworkAuditDenialCategory::InterceptHandshakeFailed removed (zero production constructors, permanently unconstructable per ADR-113 D-01)"
  - "NetworkAuditDenialCategory::ALL self-enumerating const + assert_all_variants_covered compile-time guard"
affects: [115-05 (nono-py DRAIN-02 codec — must delete both hand-written matches that still reference the removed variant), 118 (receipts work builds on this denial/audit spine)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Required-argument-over-optional-struct-field: security-relevant classification moved out of a #[derive(Default)] struct into a function's own non-Option parameter, so omission is E0061 not a silent default"
    - "Zero-new-dependency exhaustive-match self-enumeration guard (const ALL + private exhaustive match, no wildcard arm) as the fallback to strum::EnumIter"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/audit.rs
    - crates/nono-proxy/src/connect.rs
    - crates/nono-proxy/src/external.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono/src/undo/types.rs

key-decisions:
  - "D-06: EventContext.denial_category removed; log_denied's 3rd positional parameter is now non-Option NetworkAuditDenialCategory"
  - "D-07: connect.rs's HTTPS deny_domain enforcement point and external.rs's cloud-metadata host-check both emit HostDenied"
  - "D-08: external.rs's enterprise-proxy status!=200 rejection emits ExternalProxyRejected"
  - "D-09: InterceptHandshakeFailed removed (not reserved); ledger-safety precondition re-verified empirically before removal"
  - "D-11: zero-new-dependency fallback (const ALL + exhaustive-match guard) chosen over strum::EnumIter — pub(crate) per plan spec, so Plan 115-05's nono-py test cannot consume ALL directly"

patterns-established:
  - "Bite-proof live-verification: every compile-time guard added in this plan was empirically broken (argument removed / variant added without guard update) and the exact rustc error code captured before being reverted, not merely asserted by reading code"

requirements-completed: [DRAIN-02, DRAIN-03]

# Metrics
duration: ~25min
completed: 2026-08-08
---

# Phase 115 Plan 02: Denial Spine Required-Category + Self-Enumerating Enum Summary

**`audit::log_denied` now takes `NetworkAuditDenialCategory` as a required 3rd positional argument (not an optional `EventContext` field), all 28 call sites migrated with 3 previously-uncategorised sites now emitting real categories, and `InterceptHandshakeFailed` is removed from the core enum in favor of a self-enumerating `ALL` const + compile-time exhaustive-match guard.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-08-08T21:04:00Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- Closed DRAIN-03/NEW-01: a new `log_denied` call site that omits a category is now a compile error (E0061), not a silent uncategorised denial — bite-proof empirically verified twice (missing-argument error during Task 1's transitional state, and again as the documented counterexample).
- All 6 `deny_domain` dispatch paths (`connect.rs`, `external.rs`, `reverse.rs` x3, `server.rs`) now emit `HostDenied` — closing the 2-of-6 under-reporting blind spot the audit named.
- `external.rs`'s enterprise-proxy-rejected site now emits `ExternalProxyRejected` — the variant's first production constructor.
- Removed `InterceptHandshakeFailed`, the last denial variant with zero production constructors and no possible future one (ADR-113 D-01's standing no-TLS-interception decision).
- Closed the D-11 half of DRAIN-02: `NetworkAuditDenialCategory` is now self-enumerating via `ALL` + a compile-time exhaustive-match guard, with the strum-vs-fallback choice decided and documented in code.

## Task Commits

1. **Task 1: log_denied required-category signature (D-06) with bite-proof** - `3fd95b8b` (fix)
2. **Task 2: Migrate 28 call sites; D-07/D-08 category assignment** - `83b8c504` (fix)
3. **Task 3: Remove InterceptHandshakeFailed (D-09); self-enumerating guard (D-11)** - `73617620` (fix)

_Note: Task 1 and Task 2 are structurally coupled — Task 1's signature change breaks all 28 call sites in the same crate until Task 2 migrates them, since `EventContext`'s field removal and the function's new required argument affect every existing call site simultaneously. Both commits are self-contained (each is buildable on its own for its own file scope's logical intent), but only after Task 2 lands does `cargo build -p nono-sandbox-proxy` succeed again. This is documented, not accidental: Task 1's own verification step demonstrated the compile failure as the required bite-proof evidence before Task 2 resolved it._

## Files Created/Modified

- `crates/nono-proxy/src/audit.rs` - `EventContext.denial_category` field removed; `log_denied` gained a required 3rd positional `category: NetworkAuditDenialCategory` parameter; the one `#[cfg(test)]` call site (`log_denied_records_reason`) updated
- `crates/nono-proxy/src/connect.rs` - 3 `log_denied` call sites migrated; the HTTPS `deny_domain` enforcement point now emits `HostDenied`
- `crates/nono-proxy/src/external.rs` - 2 `log_denied` call sites migrated; cloud-metadata host-check emits `HostDenied`, status!=200 rejection emits `ExternalProxyRejected`
- `crates/nono-proxy/src/reverse.rs` - 17 `log_denied` call sites migrated (mechanical positional-argument move; two `HostDenied` sites, in `handle_spiffe_route` and `handle_spiffe_assertion_credential`, were resolved via one `replace_all` since their surrounding text was byte-identical)
- `crates/nono-proxy/src/server.rs` - 6 `log_denied` call sites migrated, including one that resolves its category from a local variable (`is_spiffe_route`/`is_capture_route` dispatch)
- `crates/nono/src/undo/types.rs` - `InterceptHandshakeFailed` variant removed with a documented precondition-discharge comment; `NetworkAuditDenialCategory::ALL` const + `assert_all_variants_covered` guard added; one new `#[cfg(test)]` test (`all_denial_categories_present_and_guard_covers_every_entry`)

## Decisions Made

- **D-06 signature shape:** followed RESEARCH's locked recommendation exactly — `category` inserted as the 3rd positional parameter (after `mode`, before `ctx`), not a struct-splitting alternative. `log_allowed` was left untouched (confirmed by inspection it never reads `denial_category`).
- **D-09 removal is unconditional, no escape hatch:** RESEARCH Q1 (re-verified via fresh grep before removal — `InterceptHandshakeFailed` appears only in its own enum definition and in `../nono-py`'s two dead hand-written matches) confirmed zero persisted-ledger risk, so no `#[serde(other)]` tolerance was added.
- **D-11 fallback over strum:** implemented the zero-new-dependency `const ALL` + exhaustive-match guard exactly as the plan specified `pub(crate)` visibility, even though this means Plan 115-05's `../nono-py` round-trip test cannot reach `ALL` directly (it lives in a different crate) — this was the plan's own explicit choice, not a deviation; the rationale (workspace-wide absence of `strum`, `thiserror` as derive-only-dependency precedent, equal structural strength at lower dependency cost) is recorded in a doc comment above `ALL`.
- **dead_code lint on `ALL` and `assert_all_variants_covered`:** both items are only consumed by this module's own `#[cfg(test)]` test today, which triggers rustc's per-target dead_code warning on the non-test `lib` build (the test target itself sees no warning, since it uses them). Resolved with `#[cfg_attr(not(test), allow(dead_code))]`, following the exact precedent already in this codebase at `crates/nono-cli/src/policy.rs`'s `expand_egress_preset_tokens` — not a bare unconditional `#[allow(dead_code)]`, consistent with CLAUDE.md's "avoid lazy dead_code allows" guidance.

## Deviations from Plan

None — plan executed exactly as written. The Task 1/Task 2 coupling described above is an artifact of the plan's own task decomposition (a crate-wide signature change split across two tasks whose files necessarily interact), not a deviation from it; both tasks' individual acceptance criteria were met and independently verified.

## Issues Encountered

None during execution. One expected, plan-scoped consequence to flag:

- **`../nono-py` no longer builds** (`cargo build` there now fails with `error[E0599]: no variant or associated item named InterceptHandshakeFailed found for enum NetworkAuditDenialCategory`, at `src/proxy.rs:85` and `src/undo.rs:604`). This is the exact ripple CONTEXT.md's D-09 text predicted ("Removal also ripples into `../nono-py`'s decoder — which under D-10 stops being hand-written anyway, so sequence D-09 with D-10") and this plan's own `<verification_notes>` explicitly instructed NOT to fix here ("Do NOT edit `../nono-py` in this plan — Plan 115-05 owns that. But DO note in SUMMARY.md any variant removal so 115-05 can act on it."). Verified read-only via `cargo build` in `../nono-py` (no files there were modified — confirmed via `git status` unaffected in that repo). **Plan 115-05 must delete both hand-written matches (`../nono-py/src/proxy.rs:70-105` encoder, `../nono-py/src/undo.rs:587-623` decoder) referencing `InterceptHandshakeFailed` as part of its D-10 work**, restoring the build.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The denial/audit spine (`log_denied`'s required-category signature, `NetworkAuditDenialCategory`'s self-enumerating `ALL`) is now the clean foundation DRAIN-02/Plan 115-05 and Phase 117/118's receipt work build on, per this phase's stated purpose.
- **Blocker for Plan 115-05 (DRAIN-02):** `../nono-py` will not build until Plan 115-05's D-10 task deletes both hand-written matches referencing the now-removed `InterceptHandshakeFailed` variant. This is expected and already scoped into that plan — flagging here so it is not mistaken for a regression introduced by this plan.
- `NetworkAuditDenialCategory::ALL` is `pub(crate)` (per this plan's own spec) — Plan 115-05 cannot consume it directly from `../nono-py`; its round-trip test will need its own enumeration mechanism (likely serde-driven, consistent with D-10's broader "use the enum's own serde" direction) rather than reusing `ALL`.

---
*Phase: 115-v3-6-carry-forward-drain*
*Completed: 2026-08-08*

## Self-Check: PASSED

All 6 modified source files confirmed present on disk; all 4 commit hashes
(`3fd95b8b`, `83b8c504`, `73617620`, `7afe1583`) confirmed present in
`git log --oneline --all`. Full-plan verification gate re-run clean:
`cargo build --workspace --all-targets` (0 warnings/errors),
`cargo test -p nono-sandbox-proxy --lib` (301 passed), `cargo test -p
nono-sandbox --lib` (826 passed, including the new
`all_denial_categories_present_and_guard_covers_every_entry` test),
`cargo fmt --all -- --check` (clean, no diff).
