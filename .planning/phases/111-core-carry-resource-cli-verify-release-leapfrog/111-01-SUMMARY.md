---
phase: 111-core-carry-resource-cli-verify-release-leapfrog
plan: 01
subsystem: policy
tags: [macos, landlock, seatbelt, policy.json, exec_strategy, upstream-sync, cross-target-clippy]

# Dependency graph
requires:
  - phase: 108-core-carry-resource-cli-verify-release-leapfrog
    provides: "UPST12 divergence ledger (108-DIVERGENCE-LEDGER.md) identifying #1378 and #1424 as will-sync carries for the v3.6 UPST12 window"
provides:
  - "user_caches_macos.allow.readwrite now includes ~/.cache (upstream #1378, ca888108)"
  - "MAX_CRYPTO_THREADS raised from 7 to 12 with upstream's corrected doc comment (upstream #1424, 099237da)"
  - "Two new Wave-0 by-value unit tests closing the gap 111-RESEARCH.md's validation architecture identified"
affects: [111-04-fork-invariant-verify, 111-05-release-leapfrog]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Hand-edit instead of cherry-pick when an upstream commit's other files don't exist under matching names/paths in the fork"]

key-files:
  created: []
  modified:
    - crates/nono-cli/data/policy.json
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/src/exec_strategy.rs

key-decisions:
  - "Did not run git cherry-pick ca888108 — 4 of its 6 touched files don't exist under those names in this fork (macos_trust.rs, proxy_command.rs, tls_intercept/ca.rs) and a 5th (learn.rs) is an unrelated clippy cosmetic; hand-edited the single relevant policy.json line instead"
  - "Verified MAX_CRYPTO_THREADS test live on x86_64-unknown-linux-gnu via cross test, since exec_strategy.rs is #[cfg(not(target_os = \"windows\"))] and does not compile natively on this Windows dev host"

patterns-established: []

requirements-completed: [CORE-01]

# Metrics
duration: 35min
completed: 2026-08-04
---

# Phase 111 Plan 01: Core Carry (#1378 ~/.cache + #1424 MAX_CRYPTO_THREADS) Summary

**Absorbed two independent upstream macOS/core carries — a one-line `~/.cache` policy grant and a `MAX_CRYPTO_THREADS` 7→12 bump — each backed by a new Wave-0 by-value test, with both mandatory cross-target clippy gates re-confirmed GREEN.**

## Performance

- **Duration:** 35 min
- **Started:** 2026-08-04T23:00:00Z (approx)
- **Completed:** 2026-08-04T23:20:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- `user_caches_macos.allow.readwrite` in `policy.json` now contains `["~/Library/Caches", "~/Library/Logs", "~/.cache"]`, closing upstream #1378's fix for tools (uv, Corepack) that write to `~/.cache` on macOS despite the platform convention being `~/Library/Caches`
- `MAX_CRYPTO_THREADS` raised from 7 to 12 in `crates/nono-cli/src/exec_strategy.rs`, with upstream #1424's corrected doc comment explaining the libdispatch workqueue-thread headroom Security.framework spawns for `SecTrustSettings*` XPC during proxy CA setup on macOS (closes the fork thread-count flake)
- Two new Wave-0 by-value tests added, closing the gap `111-RESEARCH.md`'s validation architecture flagged: existing tests only proved the policy group *loads*, not its contents, and the crypto-thread count was only exercised indirectly through threading-guard match arms
- Both mandatory cross-target clippy gates (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`, both `-D warnings -D clippy::unwrap_used`) confirmed GREEN with zero findings on the touched `exec_strategy.rs`
- `max_crypto_threads_raised_to_12` confirmed passing live via `cross test --target x86_64-unknown-linux-gnu` (the module is `#[cfg(not(target_os = "windows"))]` and does not compile on this Windows dev host natively)

## Task Commits

Each task was committed atomically:

1. **Task 1: Absorb #1378 — add ~/.cache to user_caches_macos + Wave-0 test** - `1d0c8eb5` (feat)
2. **Task 2: Absorb #1424 — MAX_CRYPTO_THREADS 7→12 + Wave-0 test + cross-target gates** - `74146367` (feat)

**Plan metadata:** this commit (docs: complete plan)

## Files Created/Modified
- `crates/nono-cli/data/policy.json` - Appended `"~/.cache"` as the third entry in `user_caches_macos.allow.readwrite`
- `crates/nono-cli/src/policy.rs` - Added `test_user_caches_macos_includes_dot_cache`, mirroring the existing `test_embedded_claude_code_platform_groups_have_expected_paths` pattern
- `crates/nono-cli/src/exec_strategy.rs` - Raised `MAX_CRYPTO_THREADS` from 7 to 12 with upstream's corrected doc comment; added `max_crypto_threads_raised_to_12` test

## Decisions Made
- Did not attempt `git cherry-pick ca888108fe5983be866e8d1a6eccf96edc2a8dd5` (#1378) — of its 6 touched files, `macos_trust.rs`, `proxy_command.rs`, and `tls_intercept/ca.rs` don't exist under those names/paths in this fork, `learn.rs`'s touched lines are an unrelated `clippy::question_mark` cosmetic, and the `nono-proxy/src/server.rs` `&*self.token` line was not found via search (fork has diverged past it). Hand-edited the single relevant `policy.json` array entry instead, per the plan's explicit action.
- Ran the new `max_crypto_threads_raised_to_12` test via `cross test --target x86_64-unknown-linux-gnu` rather than relying solely on the native Windows `cargo test` invocation, because `exec_strategy.rs` is gated `#[cfg(not(target_os = "windows"))]` (Windows uses `exec_strategy_windows/mod.rs` instead) — the native command reports "0 tests" for this specific filter, which would otherwise look like a silent pass. The live cross-target run confirms `ok. 1 passed; 0 failed`.

## Deviations from Plan

None - plan executed exactly as written. Both tasks' acceptance criteria were met without needing Rule 1-4 auto-fixes.

## Issues Encountered

None. The plan's own read_first and action sections correctly anticipated that Task 2's verification command (`cargo test -p nono-sandbox-cli --bin nono -- max_crypto_threads_raised_to_12`) would not exercise the test natively on this Windows host, since the plan mandates both cross-target clippy gates as the primary verification for this task; the cross-target `cross test` run was added as extra positive confirmation the test itself is correct (not just clippy-clean) beyond what the plan's `<verify>` block strictly required.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- CORE-01 requirement satisfied at the codebase level for this plan's scope (the `~/.cache` grant and the `MAX_CRYPTO_THREADS` bump).
- Plan 111-04 (VERIFY-01 combined 108-111 fork-invariant pass) can build on these two confirmed-GREEN cross-target clippy runs for `exec_strategy.rs`.
- No blockers for Wave 1's remaining parallel plans (111-02 CORE-02 help-text correction, 111-03 ADR-111 + ledger addendum).

---
*Phase: 111-core-carry-resource-cli-verify-release-leapfrog*
*Completed: 2026-08-04*
