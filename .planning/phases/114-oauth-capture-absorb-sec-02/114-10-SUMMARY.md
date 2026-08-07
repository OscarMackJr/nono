---
phase: 114-oauth-capture-absorb-sec-02
plan: 10
subsystem: bindings
tags: [pyo3, maturin, napi-rs, rust, struct-drift, oauth-capture]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "RouteConfig.capture (114-02), NetworkAuditEvent/EventContext.capture_context (114-01), NetworkAuditDenialCategory::CaptureUnsupportedPath/CaptureBufferOrRewriteFailed (114-01/114-05)"
provides:
  - "../nono-py rebuilt clean against the phase's final nono/nono-proxy struct surface (cargo build + clippy + maturin build all green)"
  - "../nono-ts re-confirmed structurally immune to this phase's struct changes (zero nono_proxy references, napi build green, no source edits)"
  - "Real (not predicted) break-surface evidence for D-14: 4 compiler errors across 3 files in ../nono-py"
affects: [115-oauth-capture-remaining-plans, future-upst-sync-phases, nono-py, nono-ts]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-14 discipline: run the real build first, fix exactly what the compiler reports, never predict the break surface"
    - "Exhaustive match arms on NetworkAuditDenialCategory must be checked alongside struct literals whenever the enum gains a variant"

key-files:
  created: []
  modified:
    - "../nono-py/src/proxy.rs"
    - "../nono-py/src/policy.rs"
    - "../nono-py/src/undo.rs"

key-decisions:
  - "Confirmed the plan's own predicted break surface (proxy.rs, policy.rs) was INCOMPLETE: the real compiler output added a third file (undo.rs, NetworkAuditEvent struct literal) and a fourth distinct error class (non-exhaustive match in proxy.rs's denial_category dict encoder) that the plan's <interfaces> section had flagged as a risk but not confirmed live."
  - "capture_context: None in undo.rs's dict->NetworkAuditEvent round-trip mirrors the pre-existing spiffe_context: None residual — no dict encoder exists yet for either rich payload, so both are lost on a round-trip. Documented as a named residual, not silently patched with a fabricated encoding."
  - "../nono-ts required zero source changes — the previously-established structural immunity (zero nono_proxy references) held after this phase's changes, confirmed by a fresh grep before the build, not assumed from the 113-08 precedent."

patterns-established: []

requirements-completed: [SEC-02]

# Metrics
duration: 25min
completed: 2026-08-07
---

# Phase 114 Plan 10: Sibling Binding Repo Rebuild (D-14) Summary

**Rebuilt `../nono-py` against phase 114's final `capture` struct surface — real compiler output found 4 errors across 3 files (proxy.rs, policy.rs, undo.rs), one more file and one more error class than the plan predicted; `../nono-ts` re-confirmed structurally immune with zero source changes.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-08-07 (session start)
- **Completed:** 2026-08-07
- **Tasks:** 2
- **Files modified:** 3 (all in `../nono-py`)

## Accomplishments
- `../nono-py` builds clean (`cargo build`), lints clean (`cargo clippy -- -D warnings`), and packages clean (`python -m maturin build` — real wheel built: `nono_sandbox-0.70.0-cp312-cp312-win_amd64.whl`)
- `../nono-ts` re-confirmed structurally immune to this phase's changes: `grep -rln "nono_proxy" ../nono-ts/src` returns zero files, `npx napi build --platform --release` succeeded with no source edits
- The real (not predicted) break surface is fully documented below, per D-14

## Task Commits

Each task was committed atomically, in the correct sibling repo:

1. **Task 1: Rebuild ../nono-py — fix every real compiler-reported break** - `21ab6c2` (fix, in `../nono-py` repo, branch `44-broker-ffi-lockstep`)
2. **Task 2: Rebuild ../nono-ts — confirm structural immunity** - no commit (zero source changes; nothing to commit)

**Plan metadata:** (this commit, in the `nono` repo)

## Files Created/Modified

### `../nono-py` (separate git repository, commit `21ab6c2`)
- `src/proxy.rs` — added `capture: None,` to the `RustRouteConfig` exhaustive struct literal in `RouteConfig::new()` (E0063); added two new match arms (`CaptureUnsupportedPath` → `"capture_unsupported_path"`, `CaptureBufferOrRewriteFailed` → `"capture_buffer_or_rewrite_failed"`) to the `denial_category` → Python-string exhaustive match inside `audit_event_to_py_dict()` (E0004)
- `src/policy.rs` — added `capture: None,` to the `From<PolicyRouteConfig> for RustRouteConfig` conversion's exhaustive struct literal (E0063)
- `src/undo.rs` — added `capture_context: None,` to the dict → `NetworkAuditEvent` struct literal inside `SessionMetadata.set_network_events()` (E0063), with a doc comment noting the same "no dict encoder yet" residual that already applied to `spiffe_context`

### `../nono-ts`
- No files modified — build succeeded as-is

## The Real Break Surface (D-14, compared against the plan's prediction)

**Plan predicted:** 2 files (`../nono-py/src/proxy.rs`, `../nono-py/src/policy.rs`), both for the `capture: None,` struct-literal field only. The plan's `<interfaces>` section separately flagged (not confirmed) that `capture_context` might also require a fix somewhere, and separately flagged (not confirmed) that a `match` on `NetworkAuditDenialCategory` might need new arms.

**Real compiler output** (`cargo build` in `../nono-py`, first run, before any fix):

```
error[E0063]: missing field `capture` in initializer of `nono_proxy::config::RouteConfig`
   --> src\policy.rs:743:9

error[E0063]: missing field `capture` in initializer of `nono_proxy::config::RouteConfig`
   --> src\proxy.rs:212:20

error[E0063]: missing field `capture_context` in initializer of `NetworkAuditEvent`
   --> src\undo.rs:475:34

error[E0004]: non-exhaustive patterns: `&NetworkAuditDenialCategory::CaptureUnsupportedPath` and
`&NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed` not covered
   --> src\proxy.rs:78:54
```

**Delta vs. prediction:** the real surface was **3 files, 4 distinct errors** — one more file (`undo.rs`) than predicted, and one error class (the non-exhaustive `match` in `proxy.rs`) that the plan's own author flagged as "check for it, not confirmed" rather than predicted as certain. This is consistent with every prior occurrence of this pattern in this phase and Phase 113: static prediction undercounted the real surface. The `undo.rs` dict→`NetworkAuditEvent` reverse conversion (string → enum, lines 587-623) was correctly NOT flagged by the compiler — it is not an exhaustive match (it has an `other => Err(...)` catch-all), so no fix was needed there, confirming the plan's implicit assumption that only the forward (enum → string) direction needed new arms.

After fixes: `cargo build` exit 0, `cargo clippy -- -D warnings` exit 0 (no new warnings; `cargo fmt --check` diffs that remain are pre-existing in `windows_confined_run.rs`, untouched by this plan — out of scope per the deviation rules' scope boundary), `python -m maturin build` succeeded:

```
📦 Built wheel for CPython 3.12 to C:\Users\OMack\nono-py\target\wheels\nono_sandbox-0.70.0-cp312-cp312-win_amd64.whl
```

`../nono-ts`: `grep -rln "nono_proxy" ../nono-ts/src` → zero files (re-confirmed, not assumed). `npx napi build --platform --release`:

```
   Compiling nono-node v0.70.0 (C:\Users\OMack\nono-ts)
   Compiling nono-sandbox v0.70.0 (C:\Users\OMack\Nono\crates\nono)
    Finished `release` profile [optimized] target(s) in 54.95s
```
Exit 0, no source changes required.

## Decisions Made

- Used `python -m maturin build` instead of `uv run maturin build` — `uv` is not on this host's PATH (project gotcha not previously documented for this exact command); `maturin` 1.14.1 is installed directly into the host Python 3.12 environment and invoked via `python -m maturin`. Functionally identical real build output, not a prediction.
- Did not touch `windows_confined_run.rs`'s pre-existing `cargo fmt --check` diffs — out of scope per the deviation rules' scope boundary (not caused by this task's changes).
- `capture_context: None,` documented with the same "residual, no dict encoder yet" framing as the pre-existing `spiffe_context: None,` comment immediately above it in `undo.rs`, rather than either fabricating an encoding or leaving the asymmetry unexplained.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed non-exhaustive match on `NetworkAuditDenialCategory` in `../nono-py/src/proxy.rs`**
- **Found during:** Task 1, first `cargo build` run
- **Issue:** `audit_event_to_py_dict()`'s `denial_category` → Python-string match did not cover the two new `NetworkAuditDenialCategory` variants (`CaptureUnsupportedPath`, `CaptureBufferOrRewriteFailed`) added by Plans 114-01/114-05, causing E0004
- **Fix:** Added two match arms mapping to `"capture_unsupported_path"` and `"capture_buffer_or_rewrite_failed"`, following the existing naming convention (snake_case mirror of the Rust variant name) used for every other arm in this match
- **Files modified:** `../nono-py/src/proxy.rs`
- **Verification:** `cargo build` exit 0 after fix; `cargo clippy -- -D warnings` exit 0
- **Committed in:** `21ab6c2` (nono-py repo)

**2. [Rule 1 - Bug] Fixed missing `capture_context` field in `../nono-py/src/undo.rs`'s `NetworkAuditEvent` struct literal**
- **Found during:** Task 1, first `cargo build` run
- **Issue:** `SessionMetadata.set_network_events()` constructs `nono::undo::NetworkAuditEvent` exhaustively from a Python dict; missing the new `capture_context` field (E0063). This was the third file the plan's own `<interfaces>` section explicitly warned might exist but had not confirmed live (matching the Phase 113 3-file precedent it cited).
- **Fix:** Added `capture_context: None,` with a doc comment referencing the identical pre-existing `spiffe_context: None,` residual immediately above it (no dict encoder for the rich payload exists yet for either field)
- **Files modified:** `../nono-py/src/undo.rs`
- **Verification:** `cargo build` exit 0 after fix
- **Committed in:** `21ab6c2` (nono-py repo)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — compiler-reported breaks in files/error-classes the plan flagged as possible but did not confirm or fully predict)
**Impact on plan:** Both auto-fixes were required for `../nono-py` to compile at all; no scope creep — fixes are the minimal correct values (`None` / new match arms) with no behavior change beyond restoring exhaustiveness.

## Issues Encountered

- `uv` is not installed/on PATH on this host. Worked around by invoking the already-installed `maturin` 1.14.1 via `python -m maturin build` instead of `uv run maturin build`. Same real build, same real output — not a prediction substitute.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both sibling binding repos (`../nono-py`, `../nono-ts`) are green against phase 114's final `nono`/`nono-proxy` struct surface. `../nono-py`'s wheel was actually built (`maturin build`); `../nono-ts`'s native addon was actually built (`napi build --platform --release`).
- `cargo build --workspace --all-targets` remains exit 0 in the `nono` repo — no core-repo changes were needed or made by this plan.
- No architectural or API changes were surfaced by either sibling build (Rule 4 / gotcha 8 did not trigger) — this was purely additive struct-literal/match-arm maintenance.
- This confirms, for the fifth-plus consecutive time this phase and the sixth+ time across Phases 113-114, that D-14-style "run the actual build" tasks are necessary infrastructure, not redundant process overhead: the real surface (3 files, 4 errors, including a non-exhaustive match) again exceeded the plan's own static prediction (2 files, 1 field).

---
*Phase: 114-oauth-capture-absorb-sec-02*
*Completed: 2026-08-07*

## Self-Check: PASSED

- FOUND: `../nono-py/src/proxy.rs` (contains `capture: None,` at line 232, and the two new denial-category match arms)
- FOUND: `../nono-py/src/policy.rs` (contains `capture: None,` at line 757)
- FOUND: `../nono-py/src/undo.rs` (contains `capture_context: None,`)
- FOUND: nono-py commit `21ab6c2` in `../nono-py`'s git history
- FOUND: this SUMMARY.md at `.planning/phases/114-oauth-capture-absorb-sec-02/114-10-SUMMARY.md`
- Re-ran `cargo build --workspace --all-targets` in the `nono` repo: exit 0 (unchanged, no core-repo edits made by this plan)
