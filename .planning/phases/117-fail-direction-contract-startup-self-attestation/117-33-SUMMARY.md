---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 33
subsystem: audit
tags: [rust, mutex, telemetry, hmac-chain, tamper-evidence, clippy, windows]

# Dependency graph
requires:
  - phase: 117 (Plan 25)
    provides: "SecurityEventLayerInner fields private; advance_and_snapshot as the sole cross-module chain-advancement accessor (WR-09)"
  - phase: 117 (Plan 17)
    provides: "emit_attestation_event moved to exec_strategy_windows::attestation_downgrade_event (NR3-05 follow-up)"
provides:
  - "advance_and_emit: holds SecurityEventLayer's chain mutex across the full build+advance+emit sequence for emit_attestation_event, matching emit_override_event's and on_event's existing atomicity"
  - "SecurityEventLayer::inner narrowed back to private; poison_for_test as the sole documented cross-module mutex-poisoning accessor"
affects: [117-gap-closure-round-3, WR-21]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Single-critical-section chain emitters: every SecurityEventLayer method that advances the HMAC chain now holds inner's lock across BOTH the chain-advance AND the corresponding Event Log write (on_event, emit_override_event, and now emit_attestation_event via advance_and_emit)"
    - "Test-only cross-module mutex access goes through a named, documented, individually-allowed accessor (poison_for_test) instead of raw field access, keeping the field itself private"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/telemetry/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs

key-decisions:
  - "advance_and_emit replaces advance_and_snapshot as a generic FnOnce/FnOnce pair (build_event_bytes, emit) rather than returning a snapshot tuple, so the caller's emit logic runs while the MutexGuard is still in scope"
  - "poison_for_test lives in the same impl block as the inner field (module-local access), is #[cfg(test)] with its own #[allow(clippy::unwrap_used)] (not covered by the file's mod-tests-scoped allow), and is exercised by a same-file test so it has a real caller in both nono and nono-agentd's independent #[path]-copies"

requirements-completed: [CINT-02]

# Metrics
duration: 55min
completed: 2026-08-11
---

# Phase 117 Plan 33: Chain-Mutex Atomicity Parity + inner Field Privacy Summary

**Closed WR-21: `emit_attestation_event` now holds the chain mutex across its full build-advance-emit sequence via a new `advance_and_emit` accessor, and `SecurityEventLayer::inner` is private again with a dedicated `poison_for_test` helper replacing the only remaining raw cross-module `.inner.lock()` call.**

## Performance

- **Duration:** 55 min
- **Started:** 2026-08-11T15:57:00Z
- **Completed:** 2026-08-11T16:52:25Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Replaced `advance_and_snapshot` (locked, advanced, dropped lock, returned a snapshot for the caller to build/emit separately) with `advance_and_emit` (locks once, advances the chain, then runs the caller's `emit` closure — including any Event Log write — WHILE the lock is still held). `emit_attestation_event` now shares the exact atomicity guarantee `emit_override_event` and `on_event` already had.
- Narrowed `SecurityEventLayer::inner` from `pub(crate)` back to plain private, deleting the stale comment that pointed at an unrelated `SecurityEventLayerInner` follow-up note.
- Added `poison_for_test`, a `#[cfg(test)]` method living in the production `impl SecurityEventLayer` block (not `mod tests`), as the sole documented substitute for the direct `.inner.lock()` field access that `attestation_downgrade_event.rs`'s poisoning test previously performed.
- Enumerated all three production chain-advancing emitters (`on_event`, `emit_override_event`, `emit_attestation_event` via `advance_and_emit`) — all three now hold the lock across chain-advance-then-emit; no fourth emitter exists (confirmed via `grep -n "advance_chain(" crates/nono-cli/src`).
- Proved both hardened behaviors with live perturbations (see below), each reverted before its commit.

## Task Commits

Each task was committed atomically:

1. **Task 1: Replace advance_and_snapshot with advance_and_emit (lock held across the whole sequence)** - `072d324a` (fix)
2. **Task 2: Narrow SecurityEventLayer::inner to private; replace the direct-field-access poisoning test with poison_for_test** - `526f4ed5` (fix)
3. **Task 3: Cross-target clippy verification** - no source changes (verification-only task; results recorded below)

**Plan metadata:** (this commit, docs: complete plan)

## Files Created/Modified

- `crates/nono-cli/src/telemetry/mod.rs` - `advance_and_snapshot` deleted; `advance_and_emit<R>` added (locks once, holds the guard across `build_event_bytes` + `advance_chain` + the caller's `emit` closure); `inner` field visibility narrowed to private; `poison_for_test` added with its own `#[allow(clippy::unwrap_used)]`; two new tests (`advance_and_emit_holds_lock_across_full_build_advance_emit_sequence`, `poison_for_test_poisons_mutex_for_emit_override_event`)
- `crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs` - `emit_attestation_event` rewritten to call `self.advance_and_emit(build_event_bytes, emit)`, moving `SecurityEvent` construction/emission inside the `emit` closure (identical field values, same D-14 degrade-not-abort semantics); `emit_attestation_event_err_on_poisoned_mutex` rewritten to call `layer.poison_for_test()` instead of a manual `Arc`/`catch_unwind` dance against `layer_clone.inner.lock()`; module doc comment updated to describe the Plan 33 changes

## Decisions Made

- **Chain-advancing emitter enumeration (round-3 class-coverage discipline):** grepped every `advance_chain(` call site in `crates/nono-cli/src` — exactly three production call sites exist: `emit_override_event` (mod.rs:363, already atomic pre-plan), `on_event`'s tracing-intercept path (mod.rs:524, already atomic pre-plan — the `MutexGuard` stays in scope across its own `windows::emit_security_event` call), and `advance_and_emit`/former `advance_and_snapshot` (mod.rs:425→453, the one this plan fixes). No fourth emitter exists; the audit trail's separate ledger mechanism (`crates/nono/src/audit/`, mentioned in CLAUDE.md) is a structurally distinct system with its own append/verify path, not a `SecurityEventLayer` chain emitter, and is out of this plan's scope (its files are not in `files_modified`).
- **`agent_daemon/telemetry_init.rs`'s `self.inner.lock()`/`self.inner.on_event()`/`self.inner.chain_sequence()` calls (lines 163/166) are NOT cross-module access to `SecurityEventLayer::inner`:** they belong to a distinct, test-only `SpyLayer { inner: SecurityEventLayer }` wrapper struct whose own field happens to share the name `inner`. Confirmed by reading the struct definition before editing — this is unrelated to the field this plan narrows and required no change.
- Kept `advance_and_emit` generic over `emit`'s return type (`impl FnOnce(&str, &str, bool) -> R`) rather than hardcoding `String`, so the accessor stays reusable for a future emitter with a different emit-closure return shape, matching the interface the plan's `<action>` specified.

## Deviations from Plan

None beyond what the plan's own `<action>`/`<interfaces>` anticipated. No Rule 1/2/3/4 auto-fixes were needed — the plan's prescribed shape compiled and passed on the first implementation attempt for both tasks.

## Perturbation Proofs

**Task 1 (atomicity):** Added `advance_and_emit_holds_lock_across_full_build_advance_emit_sequence` — two threads call `advance_and_emit` concurrently; thread A's `emit` closure sleeps 150ms after starting (simulating a slow Event Log write) and sets an `a_emit_finished` flag on completion; thread B starts ~30ms later and its `build_event_bytes` closure (which runs INSIDE the lock, immediately on acquisition) records whether `a_emit_finished` was already `true`. With the fix, B cannot reach that point until A's full sequence (including the 150ms emit) completes, so the assertion always passes.

Ran clean against the fixed implementation:
```
cargo test -p nono-sandbox-cli --bin nono telemetry::tests::advance_and_emit_holds_lock_across_full_build_advance_emit_sequence -- --nocapture
...
test telemetry::tests::advance_and_emit_holds_lock_across_full_build_advance_emit_sequence ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1636 filtered out; finished in 0.15s
```

Then temporarily perturbed `advance_and_emit` to `drop(inner)` before calling `emit` (simulating the pre-fix `advance_and_snapshot` shape) and re-ran the same test:
```
cargo test -p nono-sandbox-cli --bin nono telemetry::tests::advance_and_emit_holds_lock_across_full_build_advance_emit_sequence -- --nocapture
...
thread 'telemetry::tests::advance_and_emit_holds_lock_across_full_build_advance_emit_sequence' (76964) panicked at crates\nono-cli\src\telemetry\mod.rs:1057:9:
thread B's build_event_bytes (which runs under the lock) must not be reachable until thread A's full build+advance+emit sequence has completed — this is the atomicity advance_and_emit must guarantee (WR-21 point 1)
test telemetry::tests::advance_and_emit_holds_lock_across_full_build_advance_emit_sequence ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1636 filtered out; finished in 0.16s
```

Reverted the perturbation; re-ran — passed again (shown above as the first run, re-confirmed identical after revert).

**Task 2a (clippy::unwrap_used gate on poison_for_test):** Temporarily removed `poison_for_test`'s own `#[allow(clippy::unwrap_used)]`:
```
cargo clippy -p nono-sandbox-cli --all-targets -- -D warnings -D clippy::unwrap_used
...
error: used `unwrap()` on a `Result` value
   --> crates\nono-cli\src\bin\..\telemetry\mod.rs:299:26
    |
299 |             let _guard = this.inner.lock().unwrap();
    |                          ^^^^^^^^^^^^^^^^^^^^^^^^^^
error: could not compile `nono-sandbox-cli` (bin "nono-agentd" test) due to 1 previous error
error: could not compile `nono-sandbox-cli` (bin "nono" test) due to 1 previous error
```
Failed in BOTH binaries' test targets, naming exactly `poison_for_test`'s `.unwrap()` — confirming this function is not accidentally covered by the file's `mod tests`-scoped allow. Restored the allow; re-ran `cargo clippy -p nono-sandbox-cli --all-targets -- -D warnings -D clippy::unwrap_used` — clean again.

**Task 2b (visibility non-load-bearing check):** Temporarily re-added `pub(crate)` to `inner` and confirmed `cargo build -p nono-sandbox-cli --bin nono` and `--bin nono-agentd` both still succeed. This direction is expected to always succeed (widening a field's visibility can never break compilation) — the meaningful evidence is that `inner` compiles clean as **private** (already proven by the steady-state build before and after this perturbation), which in turn confirms no other cross-module direct-field-access site was missed: had one existed, the private-narrowing edit itself would have failed to compile before this perturbation was even attempted. Reverted `inner` to private; re-verified `cargo clippy -p nono-sandbox-cli --all-targets -- -D warnings -D clippy::unwrap_used` clean.

## Deadlock Analysis (explicit, per round-3 discipline)

`advance_and_emit` holds `self.inner`'s `MutexGuard` across the caller-supplied `emit` closure. This is safe from self-deadlock because:
- `emit_attestation_event` (the only caller) passes an `emit` closure whose body calls exactly one function: `crate::telemetry::windows::emit_security_event`, a free function (not a `SecurityEventLayer` method) that writes to the Windows Application Log / ETW and touches no `SecurityEventLayer` state — it cannot re-enter `self.inner`'s mutex, whether via this clone or any other clone (all clones share the same underlying `Arc<Mutex<...>>`).
- No other `SecurityEventLayer` method (nor any code reachable from `windows::emit_security_event`) is called from within the `emit` closure.
- This mirrors the existing, already-safe shape of `emit_override_event` and `on_event`, both of which hold the same lock across their own `windows::emit_security_event` calls with no prior deadlock reports.

## Chain-Advancing Emitter Enumeration (round-3 class-coverage discipline)

All production call sites of `advance_chain(` in `crates/nono-cli/src`, confirmed by grep:
| Emitter | Location | Lock held across emit? |
|---|---|---|
| `on_event` (tracing intercept) | `telemetry/mod.rs:524` (post-edit line) | Yes — pre-existing, unchanged by this plan |
| `emit_override_event` | `telemetry/mod.rs:363` (post-edit line) | Yes — pre-existing, unchanged by this plan |
| `emit_attestation_event` via `advance_and_emit` | `telemetry/mod.rs:453` → `attestation_downgrade_event.rs` | Yes — **fixed by this plan** (was No, via `advance_and_snapshot`) |

No fourth emitter exists. All three now share the same atomicity guarantee.

## Cross-Target Verification (Task 3)

Per the checklist's Decision Tree:
- `telemetry/mod.rs`: **D-35 gates apply.** No `#[cfg(target_os = ...)]` gate anywhere in the file; wired unconditionally via `main.rs`'s ungated `pub(crate) mod telemetry;` and also `#[path]`-included into `nono-agentd.rs`. Compiles on every target `cargo clippy --workspace --target ...` builds, including linux-gnu and apple-darwin. Both edits in this plan (Task 1's `advance_and_emit`, Task 2's `inner`/`poison_for_test`) are squarely in that blast radius.
- `attestation_downgrade_event.rs`: **D-35 gates do NOT apply.** Declared via `mod attestation_downgrade_event;` inside `exec_strategy_windows/mod.rs`, which `main.rs` only wires behind `#[cfg(target_os = "windows")] #[path = "exec_strategy_windows/mod.rs"] mod exec_strategy;` — no Unix counterpart, matching the checklist's explicit Windows-only exemption.

Both gates run to completion with zero errors/warnings:

```
$ docker info 2>&1 | grep "Server Version"
 Server Version: 29.6.2

$ cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
...
    Checking nono-sandbox-cli v0.70.0 (/project/crates/nono-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 19s

$ cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
    Checking nono-sandbox-cli v0.70.0 (C:\Users\OMack\Nono\crates\nono-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.08s
```

No errors on either gate; no `PARTIAL` fallback needed.

## Issues Encountered

- **`poison_for_test` initially tripped `-D dead-code` under `--all-targets` on both binaries** — its only cross-module caller (`attestation_downgrade_event.rs`) compiles into the `nono` binary only, so `nono-agentd`'s independent `#[path]`-copy of `telemetry/mod.rs` had zero callers for it. Resolved by adding a same-file test (`poison_for_test_poisons_mutex_for_emit_override_event`) in `telemetry/mod.rs`'s own `mod tests`, giving the method a real caller compiled into BOTH binaries' test targets — the same multi-binary-compilation pattern already documented for `chain_sequence`, `emit_override_event`, and `advance_and_emit` in this file. No `#[allow(dead_code)]` used.
- **`cargo fmt --check` initially failed** on `poison_for_test`'s justification comment, which was originally written as a trailing `// ...` comment chain directly after `#[allow(clippy::unwrap_used)]` (rustfmt re-indents multi-line trailing comments after an attribute inconsistently). Resolved by moving the justification into the method's doc comment (a `# clippy::unwrap_used justification` section) and leaving the attribute on its own line — matching the style already used at the bottom of this file above `mod tests`.

## Verification

- `cargo build --workspace --all-targets` — clean.
- `cargo fmt --check` — clean (workspace-wide).
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` — clean (workspace-wide).
- `cargo clippy -p nono-sandbox-cli --all-targets -- -D warnings -D clippy::unwrap_used` — clean.
- `cargo test -p nono-sandbox-cli --bin nono exec_strategy::attestation_downgrade_event` — 3/3 pass (unchanged assertions).
- `cargo test -p nono-sandbox-cli --bin nono telemetry::` — 42/42 pass, including the two new tests.
- `cargo test -p nono-sandbox-cli --bin nono-agentd telemetry::` — 42/42 pass (confirms the daemon's independent `#[path]`-copy compiles and passes identically).
- `cargo build -p nono-sandbox-cli --bin nono-agentd` — succeeds (confirms `advance_and_snapshot`'s removal does not break the daemon binary).
- `cargo test -p nono-sandbox-cli --workspace` — **1624 passed, 12 failed, 2 ignored.** All 12 failures match the documented 12-item baseline exactly (the 11 pre-existing baseline failures plus Plan 117-30's intentional D-31 host-blocked test `exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`). No new regressions.
- Both cross-target clippy gates (Task 3) pass clean for `telemetry/mod.rs`'s blast radius.
- `grep -c "fn advance_and_snapshot" crates/nono-cli/src/telemetry/mod.rs` → 0.
- `grep -n "pub(crate) inner" crates/nono-cli/src/telemetry/mod.rs` → no matches.
- `grep -n "\.inner\.lock()" crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs` → no matches.

## Next Phase Readiness

WR-21 is closed: both sibling chain emitters (`emit_override_event`, `emit_attestation_event`) now hold the lock across their full build-advance-emit sequence (as does `on_event`, the third emitter, already atomic pre-plan), and `SecurityEventLayer::inner` is private with the one legitimate cross-module need (test-only mutex poisoning) served by `poison_for_test` — itself clean under the workspace's full `--all-targets -D clippy::unwrap_used` gate. STATE.md/ROADMAP.md updates are owned by the orchestrator per this repo's project-specific override and are not touched by this executor.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-11*
