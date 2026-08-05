---
phase: 112-security-residual-sync
plan: 05
subsystem: sandbox
tags: [upstream-sync, landlock, execute-restriction, refer, cross-target-verify]

# Dependency graph
requires:
  - phase: 112-security-residual-sync
    provides: "112-02's finalized restrict_execute() baseline in crates/nono/src/sandbox/linux.rs (SEC-03 landed first per wave/depends_on ordering, shifting line numbers but not the function's shape)"
provides:
  - "restrict_execute()'s stacked execute-restriction Landlock ruleset now grants bare Refer on / (gated on abi.has_refer()), fixing renames into subdirectories created after restrict_execute() is applied"
  - "Ported regression test test_restrict_execute_does_not_break_rename_into_new_subdir, verified live via cross test on Linux"
affects: [112-06-sec06-seccomp-ancestry]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Symbol-level verification before trusting a disposition table 'adopt' call: confirmed abi.has_refer(), PathFd, PathBeneath, AccessFs::Refer, FsCapability::new_dir, and CapabilitySet::add_fs all already exist and match upstream's usage exactly, and separately confirmed the general Refer grant at linux.rs:365 lives in a distinct stacked ruleset (apply()/apply_with_abi()) from restrict_execute()'s own ruleset, so this is an additive grant in a second layer, not deny-within-allow"

key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/linux.rs

key-decisions:
  - "Ported the Wave-0 regression test verbatim with exactly one naming adaptation: upstream's apply_landlock(&caps) call became apply(&caps) — grepped the fork and confirmed no function named apply_landlock exists anywhere; the fork's equivalent single entry point is apply() (crates/nono/src/sandbox/linux.rs:686), which the restrict_execute() doc comment itself already references"
  - "Confirmed via read_first that the general Refer grant at linux.rs:365 (inside access_to_landlock/apply's main ruleset) is a separate, independently-created landlock::Ruleset from restrict_execute()'s own ruleset — Landlock rulesets stack and each restrict_self() call is an intersection-narrowing layer, so the main ruleset's Refer grant does not propagate into this second, narrower execute-restriction layer. This confirms the plan's threat-model framing: an ADDITIONAL grant in a stacking layer, not a deny-within-allow expression, satisfying CLAUDE.md's Landlock allow-list constraint"
  - "Preserved the file's existing literal double-space typo (\"Tool Sandbox  execute restriction\") verbatim in all 3 new error-message strings, per the plan's substring-grep-stability requirement"

requirements-completed: [SEC-05]

# Metrics
duration: ~55min
completed: 2026-08-05
---

# Phase 112 Plan 05: SEC-05 Landlock Execute-Restriction Refer Grant Summary

**Ported upstream `d84b4818`'s Refer grant into `restrict_execute()`'s stacked Landlock ruleset — fixes renames into freshly-created subdirectories that the narrower execute-restriction layer previously denied even though the main sandbox ruleset already permitted them — verified live via `cross test` on Linux, both mandatory cross-target clippy gates GREEN.**

## Performance

- **Duration:** ~55min
- **Started:** 2026-08-05 (approx.)
- **Completed:** 2026-08-05 (approx.)
- **Tasks:** 2/2 completed
- **Files modified:** 1

## Accomplishments

- `restrict_execute()` (`crates/nono/src/sandbox/linux.rs`) now calls `.handle_access(AccessFs::Refer)` on its ruleset in addition to `AccessFs::Execute`, and — when `abi.has_refer()` — adds a bare `PathBeneath::new(root_fd, AccessFs::Refer)` rule on `/` after the per-path `Execute` rules and before `restrict_self()`.
- Added the upstream doc-comment explaining the grant's purpose ("Landlock requires it in every layer for rename/link... Bare Refer alone can't widen access").
- Ported `test_restrict_execute_does_not_break_rename_into_new_subdir` verbatim into the file's existing `mod tests` block, using the file's existing convention of gating individual `#[test]` fns via the test's own internal `detect_abi()`/`has_execute()` early-return guard rather than a file-level `cfg`. The only adaptation was `apply_landlock(&caps)` → `apply(&caps)`, since no function named `apply_landlock` exists in this fork — its equivalent is `apply()`.
- Verified live via `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox -- test_restrict_execute_does_not_break_rename_into_new_subdir`: `test result: ok. 1 passed; 0 failed`.
- Both mandatory cross-target clippy gates GREEN with `--all-targets`: `cross clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` (exit 0) and `cargo-zigbuild clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset, exit 0). `cargo fmt --all --check` clean.

## Task Commits

Each task was committed atomically:

1. **Task 1: Grant bare Refer in the execute-restriction Landlock layer + ported regression test** - `96398a3d` (fix)
2. **Task 2: Cross-target verification** - no code changes (pure verification task); both gates confirmed GREEN, no commit needed (working tree was clean after the run)

## Files Created/Modified

- `crates/nono/src/sandbox/linux.rs` - `restrict_execute()` gains `.handle_access(AccessFs::Refer)` + the `if abi.has_refer() { ... }` bare-Refer-on-`/` grant + doc comment; ported `test_restrict_execute_does_not_break_rename_into_new_subdir` regression test

## Decisions Made

See `key-decisions` in frontmatter for the full list. Summary: this was a clean **adopt** exactly as the disposition table predicted — every symbol the upstream diff references (`abi.has_refer()`, `PathFd`, `PathBeneath`, `AccessFs::Refer`, `FsCapability::new_dir`, `CapabilitySet::add_fs`) already exists in the fork with matching signatures, and the general/stacked-ruleset separation the plan's `must_haves` demanded was independently re-verified by reading `access_to_landlock`/`apply()` (a separate `landlock::Ruleset` construction from `restrict_execute()`'s own) rather than assumed from the disposition table alone.

## Deviations from Plan

None - plan executed exactly as written, with one pre-flagged adaptation the plan's own `<read_first>` anticipated: `apply_landlock(&caps)` → `apply(&caps)` in the ported test, since `apply_landlock` does not exist as a function name in this fork (confirmed via `grep -rn "apply_landlock" crates/nono/src` returning zero hits before writing any code).

## Disposition Table Re-Verification (per this plan's mandatory symbol-level check)

`112-DISPOSITION-TABLE.md` marked SEC-05 **adopt**. Symbol-level re-verification confirmed this was correct, unlike SEC-03 (112-02, became ADAPT) and the SEC item in 112-04 (became SKIP):

| Upstream symbol referenced by `d84b4818` | Fork status |
|---|---|
| `restrict_execute()` (target function) | present, byte-identical pre-patch shape at `crates/nono/src/sandbox/linux.rs:1224` (shifted from the plan's cited `:1105` due to 112-02's SEC-03 insertions earlier in the file, not a content divergence) |
| `abi.has_refer()` | present, `crates/nono/src/sandbox/linux.rs:52` |
| `landlock::{AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr, CompatLevel, Compatible}` | present, already imported at file top |
| `AccessFs::Refer` (general grant, main ruleset) | present, `crates/nono/src/sandbox/linux.rs:365`, confirmed to live in a **separate** `Ruleset`/`restrict_self()` layer from `restrict_execute()`'s own |
| `crate::capability::FsCapability::new_dir`, `AccessMode::ReadWrite` | present, `crates/nono/src/capability.rs:105`/`:56` |
| `CapabilitySet::new()`, `CapabilitySet::add_fs()` | present, `crates/nono/src/capability.rs:994`/`:1290` |
| `apply_landlock(&caps)` (test-only call) | **absent** — no function of this name exists anywhere in the fork (`grep -rn "apply_landlock" crates/nono/src` → 0 hits); fork's equivalent is `apply(&caps)` at `crates/nono/src/sandbox/linux.rs:686`, which the ported test now calls instead |
| `libc::fork/_exit/waitpid/WIFEXITED/WEXITSTATUS` | present, already used via fully-qualified `libc::` calls throughout the file's existing test suite (no import needed) |

Only one symbol required adaptation (`apply_landlock` → `apply`), and it was a pure rename with no behavioral difference — `apply()` is `Result<SeccompNetFallback>` where upstream's is presumably `Result<()>`, but the test only calls `.is_err()`, which is unaffected by the `Ok` payload type. No architectural gap, unlike 112-02/112-04.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. No new dependencies (`Cargo.toml` unchanged).

## Next Phase Readiness

- SEC-05 is functionally complete; `REQUIREMENTS.md`'s SEC-05 checkbox is marked complete by this plan (not split across other plans).
- `112-06` (SEC-06, seccomp supervisor-ancestry) also touches `linux.rs` — should be aware this plan added ~95 lines (grant + test) to the file, shifting line numbers again for any subsequent plan's cited ranges.
- No blockers for Wave 3's remaining plans.

---
*Phase: 112-security-residual-sync*
*Completed: 2026-08-05*

## Self-Check: PASSED

Confirmed `crates/nono/src/sandbox/linux.rs` present on disk; task commit `96398a3d`
confirmed present in `git log --oneline --all`. Both mandatory cross-target clippy
gates (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`, both
`--all-targets`) and `cross test` re-confirmed GREEN.
