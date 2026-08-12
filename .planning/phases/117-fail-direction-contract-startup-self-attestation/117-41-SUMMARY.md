---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 41
subsystem: telemetry
tags: [windows, deadlock, wr-21, gap-closure, round-4, wr-30, cint-02]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-35's --no-fail-fast baseline"
provides:
  - "advance_and_emit's Deadlock analysis states the REAL invariant (target-string mismatch), not
    the false premise that emit touches no layer state"
  - "emit_security_event_target_must_not_match_on_event_prefix — the invariant enforced mechanically"
  - "on_event_prefix_test_must_keep_its_trailing_colons — the sibling half, from the other side"
affects: [117-44]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A source-text gate that greps for a literal its OWN doc comment contains passes vacuously;
      anchor on a full statement form that occurs only in production code"
    - "Prose that recommends what a new gate forbids is a defect: fix the doc in the same change,
      or the next reader follows the doc and breaks the build"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/telemetry/mod.rs
    - crates/nono-cli/src/telemetry/windows.rs

key-decisions:
  - "Extended scope to windows.rs's two doc comments, which described the emit target as
    `nono_security::*` while the code uses the bare `nono_security`. Leaving prose that invites
    exactly the edit the new gate rejects would have manufactured the next round's blocker."
  - "Rejected two failed designs for the sibling test before landing one that bites: a whole-file
    `contains` (matched its own text) and truncation at the first `#[cfg(test)]` (that marker
    first appears inside a doc comment ~300 lines above on_event)."
---

# Plan 117-41: WR-30 — Execution Summary

**Executed:** 2026-08-12 · **Wave:** 16 · **Requirement:** CINT-02

## The false premise

`advance_and_emit` runs its `emit` closure **while holding `self.inner`'s lock** (WR-21's
atomicity requirement). Its "Deadlock analysis" section justified that as:

> `emit_security_event` — a free function that writes to the Windows Application log / ETW and
> **touches no `SecurityEventLayer` state**, so it cannot re-enter this mutex.

**That is false.** Verified by reading the call chain:

1. `emit_security_event` ends in `tracing::warn!(target: "nono_security", …)`
   (`telemetry/windows.rs:265`).
2. That event is dispatched through the subscriber stack to
   `SecurityEventLayer::on_event` — a `SecurityEventLayer` method.
3. `on_event` calls `self.inner.lock()` at `telemetry/mod.rs:551`, **the same mutex**.
4. `std::sync::Mutex` is not reentrant.

What actually prevents the deadlock is a **string mismatch**: `on_event` early-returns unless
`target.starts_with("nono_security::")` (note the trailing colons), and the emit uses the bare
`"nono_security"`, which fails that test. Safety is incidental, not designed.

## The trap was baited by the code's own documentation

`telemetry/windows.rs:9` (module doc) and `:233` (`emit_security_event`'s doc) both described the
target as `` `tracing::warn!(target: "nono_security::*", …)` `` — the `nono_security::*` convention
the `path_deny`/`network_deny` events genuinely use. So "aligning the literal with the documented
convention" looks like a tidy-up and is in fact a supervisor hang before the child is ever resumed.

Both doc comments corrected, and `emit_security_event`'s now carries an explicit warning naming the
gate. **This is a scope extension beyond the plan's `files_modified`** (which listed only
`telemetry/mod.rs`), taken deliberately: shipping a gate while leaving prose that recommends
violating it is how this phase has repeatedly produced its own next blocker.

## Tests

**`emit_security_event_target_must_not_match_on_event_prefix`** — scans every `target: "..."`
literal in `telemetry/windows.rs`, asserts none starts with `nono_security::`, and asserts the bare
`"nono_security"` is still present (non-vacuity). Source-text by necessity: the deadlock cannot be
exercised at runtime without hanging the test process.

**`on_event_prefix_test_must_keep_its_trailing_colons`** — the sibling half. Dropping the `::` from
`on_event`'s prefix test produces the identical deadlock from the other side.

### Two failed test designs, both caught by perturbation

1. **Whole-file `contains("starts_with(\"nono_security::\")")`** — passed with `on_event` broken,
   because the test's OWN doc comment and assertion message contain that literal. Blind by
   construction, in the exact shape the P115 V-01 lesson names.
2. **Truncating at the first `#[cfg(test)]`** — failed even when the code was CORRECT: the first
   occurrence of that marker is inside a doc comment at `mod.rs:252`, roughly 300 lines above
   `on_event` at `:551`, so the scan excluded the production code entirely.

The landed version anchors on the full statement form
`if !event.metadata().target().starts_with("nono_security::") {`, which occurs only in `on_event`'s
body. Verified clean → fails under perturbation → clean again.

Recording this because it is the second time in this plan that the perturbation step caught a
defect in the TEST rather than the code — the argument for round 4's rule that every discovery test
carry one.

## Perturbation proofs

**Emit side** — target changed to `"nono_security::layer_attestation_downgraded"` (the exact
"cleanup" the old docs invited):

```
test telemetry::tests::emit_security_event_target_must_not_match_on_event_prefix ... FAILED
DEADLOCK: telemetry/windows.rs emits on target "nono_security::layer_attestation_downgraded",
which satisfies SecurityEventLayer::on_event's `nono_security::` prefix test. on_event takes the
same mutex advance_and_emit holds across its emit closure, and std::sync::Mutex is not reentrant —
this hangs the supervisor before the child is resumed. The target must stay the BARE
"nono_security" (WR-30).
```

**Listener side** — trailing `::` dropped from `on_event`'s prefix test:

```
test telemetry::tests::on_event_prefix_test_must_keep_its_trailing_colons ... FAILED
SecurityEventLayer::on_event must gate on the `nono_security::` prefix INCLUDING the trailing
double colon; without it the bare "nono_security" emit target matches and advance_and_emit
deadlocks (WR-30)
```

Both reverted; suite re-run green.

## Verification

| Gate | Command | Result |
|---|---|---|
| Module | `cargo test -p nono-sandbox-cli --bin nono --no-fail-fast -- --test-threads=1 telemetry::` | **45 passed / 0 failed** (was 43; +2) |
| Format | `cargo fmt --all -- --check` | clean |
| Cross-target clippy (linux-gnu) | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |
| Cross-target clippy (apple-darwin) | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |

Cross-target gates run despite neither file being under `exec_strategy_windows/`, since
`telemetry/mod.rs` compiles into both the `nono` and `nono-agentd` binaries.
