---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 40
subsystem: windows-attestation
tags: [windows, d-26, d-37, fail-open, gap-closure, round-4, wr-29, cint-02]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-35's --no-fail-fast baseline"
provides:
  - "decide_from_entries consults the D-26 tighten set before the NotApplicable short-circuit"
  - "tightened_not_applicable_row_aborts + its untightened discriminating sibling"
  - "tightened_fully_applied_row_still_proceeds — pins the arm a naive restructuring breaks"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A fix that reorders a guard must be traced against EVERY branch the reorder newly exposes;
      the minimal change (one status value) beats the general restructuring when the general one
      widens blast radius"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy_windows/attestation.rs

key-decisions:
  - "Did NOT implement the plan's prescribed restructuring. It would make a tightened, genuinely
    Confirmed row fall into the per-outcome match, where ContractOutcome::Abort's `if tightened`
    branch aborts it — tightening would turn a success into a failure. Implemented the minimal
    fix instead: the Confirmed && !partially_established short-circuit stays unconditional, and
    the tighten check goes INSIDE the NotApplicable branch only."
  - "Added tightened_fully_applied_row_still_proceeds specifically to pin the arm the plan's
    version would have broken, so a future author cannot reintroduce it."
---

# Plan 117-40: WR-29 — Execution Summary

**Executed:** 2026-08-12 · **Wave:** 16 · **Requirement:** CINT-02

## The fail-open

`decide_from_entries` short-circuited on
`status == NotApplicable || (Confirmed && !partially_established)` with a bare `continue`
**before** computing `let tightened = required_names.contains(&layer_name);`. A row that classified
`NotApplicable` therefore never consulted the D-26 tighten set at all.

Before Plan 117-28 (D-37), an ancestor walk that stopped at the first non-owned directory
classified `PartiallyApplied` → `EstablishedNotIndependentlyObservable` → reached the tighten check
→ aborted when tightened. D-37 widened the `NotApplicable` arm, and the short-circuit swallowed it:
an operator (or an `HKLM\...\RequiredLayers` fleet policy, once RF-13's enforcement wiring lands)
that explicitly REQUIRED the layer got a silent `Proceed`. The SPEC's RF-16 paragraph was false for
this case.

Latent today only because `apply_startup_attestation_gate` hardcodes empty
`required_layers_override`/`machine_required_layers`. Treated as fail-open regardless of the
finding's Warning severity, per CLAUDE.md's non-negotiable fail-secure rule.

## Deviation: the plan's prescribed restructuring introduces a NEW bug

The plan specified: compute `tightened` first, then `continue` only when
`!tightened && (NotApplicable || (Confirmed && !partially_established))`, plus an early
`if tightened && status != Confirmed { return Abort }`.

Traced against every arm, that breaks a case that works today. A **tightened row whose status is
genuinely `Confirmed` with `partially_established == false`** no longer hits the `continue` (because
`!tightened` is false) and is not caught by the early check (because `status == Confirmed`). It
falls through into the per-outcome `match`, where `ContractOutcome::Abort`'s existing
`if tightened || status == Unconfirmed` branch **aborts it**. Tightening would turn a success into
a failure — the opposite of D-26's intent.

Implemented the minimal fix instead:

1. `tightened` is computed before any short-circuit (this is the actual WR-29 defect).
2. The `Confirmed && !partially_established` short-circuit stays **unconditional** — tightening
   raises the bar for layers that fall short; it cannot invalidate one that succeeded.
3. The tighten check lives **inside** the `NotApplicable` branch only.

Blast radius is exactly one status value, and no existing per-outcome `if tightened` arm becomes
dead code (the plan's version would have made all three unreachable).

## Per-`ContractOutcome`-arm trace (required by acceptance criteria)

| Row status | Before | After | Changed? |
|---|---|---|---|
| `NotApplicable`, untightened | `continue` (Proceed) | `continue` (Proceed) | no |
| `NotApplicable`, **tightened** | `continue` → **silent Proceed** | **`Abort`** | **YES — the fix** |
| `Confirmed`, `!partially`, tightened or not | `continue` | `continue` | no |
| `Confirmed`, `partially_established`, tightened | match → `Abort` arm → `if tightened` → Abort | unchanged (still reaches match) | no |
| `Unconfirmed`, any outcome | match → per-arm handling | unchanged | no |
| `EstablishedNotIndependentlyObservable`, tightened | match → `Abort`/`Degrade`/`FailOpen` arm → `if tightened` → Abort | unchanged | no |

`NotApplicable` rows never reached the `match` before and still do not.

## Tests

- **`tightened_not_applicable_row_aborts`** — the fix. Includes a **discriminating sibling** in the
  same test: the identical row with an empty `required_names` must still `Proceed`. Without it the
  primary assertion is satisfiable by aborting on every `NotApplicable` row, which would break
  every launch whose policy does not exercise a given layer.
- **`tightened_fully_applied_row_still_proceeds`** — pins the arm the plan's restructuring breaks.

Writing the second test surfaced a distinction worth recording: **`ProbeKind::ConfiguredOnly` with
`LayerApplication::Applied` classifies `EstablishedNotIndependentlyObservable`, not `Confirmed`**,
and a tightened requirement on that is *designed* to abort (the operator asked for independent
observability the layer does not provide — this is what the pre-existing
`tightened_required_layer_aborts_even_when_default_outcome_is_fail_open` pins). Only
`ProbeKind::ConfirmedByEnforcingComponentReport` with `wfp_preconfirmed: true` yields a real
`Confirmed`. The first draft of the test asserted `Proceed` against a `ConfiguredOnly` fixture and
failed, which is how the distinction was found rather than assumed.

## Perturbation proof

Removed only the new `if tightened { return Abort }` from the `NotApplicable` branch:

```
test ...tightened_fully_applied_row_still_proceeds ... ok
test ...tightened_not_applicable_row_aborts ... FAILED
test ...tightened_required_layer_aborts_even_when_default_outcome_is_fail_open ... ok

assertion `left == right` failed: WR-29: a tightened requirement on a NotApplicable row must
abort — the tighten set must be consulted before the NotApplicable short-circuit
  left: Proceed
 right: Abort { layer: MandatoryIntegrityLabel, status: NotApplicable }
```

The silent fail-open reproduced exactly, with the other two tightened tests still green — the
perturbation is narrowly scoped to the arm under test. Reverted; re-run green.

## Verification

| Gate | Command | Result |
|---|---|---|
| Module | `cargo test -p nono-sandbox-cli --bin nono --no-fail-fast -- --test-threads=1 exec_strategy::attestation::` | **35 passed / 0 failed** |
| Format | `cargo fmt --all -- --check` | clean |
| Cross-target clippy (linux-gnu) | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |
| Cross-target clippy (apple-darwin) | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |

`attestation.rs` is under `exec_strategy_windows/`, so CLAUDE.md's cross-target MUST applies; both
gates run locally, neither deferred.
