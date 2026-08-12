---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 43
subsystem: windows-labels
tags: [windows, cr-01, public-api, test-fixtures, gap-closure, round-4, wr-25, wr-34, wr-18, cint-02, cint-03]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-36's repaired SPEC drift gate; 117-39's labels_guard.rs as landed"
provides:
  - "low_integrity_label_and_mask carrying CR-01's INHERIT_ONLY_ACE rejection at the public API boundary"
  - "every_low_integrity_label_ace_consumer_filters_inherit_only — the enumeration made mechanical"
  - "plant_mandatory_label_helpers_plant_the_same_ace_shape — the cross-crate fixture invariant gated"
  - "A corrected WR-18 ledger row with an auditable Iteration 6 addendum"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A source-text gate that include_str!s its OWN file must anchor on a DECLARATION line, not
      any occurrence of a name — the test's own string literals otherwise match first"
    - "Scope a consumer-enumeration gate to the enclosing FUNCTION, not a fixed line window:
      compliant call sites legitimately apply the filter many lines later"

key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/windows.rs
    - proj/SPEC-windows-fail-direction-contract.md

key-decisions:
  - "WR-34 gates the SDDL (the ACE shape), not whole-body equality. The two helper bodies have
    ALREADY diverged — 53 vs 61 normalised lines — because the nono-cli copy declares its `use`
    statements inside the function. Body equality would have to be manufactured first and would
    then be brittle against exactly that harmless difference. The SDDL is what CR-01/WR-18
    actually depend on, and it is byte-identical."
  - "The class gate is scoped to the production half of each file: two test call sites legitimately
    read the raw 3-tuple to assert on flags, which is the entire point of the widened reader."
---

# Plan 117-43: WR-25 + WR-34 + WR-18 ledger — Execution Summary

**Executed:** 2026-08-12 · **Wave:** 17 · **Requirements:** CINT-02, CINT-03

## Task 1 — WR-25: harden the third consumer

### Enumeration, re-derived independently (required by the plan)

`grep -rn "low_integrity_label_ace(" crates/ bindings/ --include=*.rs` (2026-08-12) → 6 hits:

| Site | Kind | Filtered `INHERIT_ONLY_ACE`? |
|---|---|---|
| `windows.rs:2011` | the declaration (sole SACL walk) | n/a |
| `windows.rs:1995` — `low_integrity_label_rid` | production | yes (WR-18) |
| `labels_guard.rs:318` — `snapshot_and_apply`'s residue predicate | production | yes (CR-01) |
| `windows.rs:2111` — `low_integrity_label_and_mask` | production, **`pub`, re-exported** | **NO** |
| `windows.rs:3279`, `labels_guard.rs:1215` | tests reading the raw 3-tuple | n/a (legitimate) |

**One SACL walk, three production consumers, one unhardened** — confirming WR-25 and contradicting
WR-18's "exactly two readers, both hardened".

The wrapper now applies the same `.filter(|(_, _, flags)| (u32::from(*flags) & INHERIT_ONLY_ACE) == 0)`
as its sibling.

**Blast radius**, walked before editing: the twelve `low_integrity_label_and_mask` call sites in
`labels_guard.rs` tests all plant via `try_set_mandatory_label` (empty ACE-flags) or read after a
non-inherit-only plant, so none change. The one inherit-only planting site asserts on
`guard.entries[0]` and `coverage().application()`, not the wrapper. The production consumer
`hook_runtime_windows.rs::validate_hook_script_windows` warns when `rid > 0x2000`; after this change
an inert ACE no longer triggers that warning — **the intended operator-visible delta**, since a
warning about an "anomalous label" that the OS never evaluates is noise.

One existing assertion message called the wrapper "a thin wrapper over low_integrity_label_ace".
That is no longer unconditionally true; updated to say it agrees for an **effective** ACE, with a
pointer to the new rejection test. Leaving it would have been precisely the stale-claim pattern
this phase keeps finding.

### Class gate

`every_low_integrity_label_ace_consumer_filters_inherit_only` scans both files and requires each
production call site's **enclosing function** to reference `INHERIT_ONLY_ACE`.

Two scoping bugs were found and fixed by running it, not by reasoning:

1. **A fixed 6-line window false-positived** on `labels_guard.rs:318`, which is compliant but
   applies its filter ~20 lines later after a long explanatory comment. Replaced with
   enclosing-function scope.
2. **Test call sites false-positived** — `low_integrity_label_ace_reports_zero_flags_for_a_freshly_applied_label`
   reads the raw 3-tuple deliberately. The scan now stops at a line-exact `mod tests {` marker
   (not `#[cfg(test)]`, which also occurs inside doc comments — the trap hit in 117-41).

### Perturbation proofs

- Removing the wrapper's filter → `inherit_only_ace_is_rejected_by_low_integrity_label_and_mask`
  fails: *"an inherit-only mandatory-label ACE is inert on the object it sits on and must not be
  reported as an effective label by the public wrapper"*.
- Adding an unfiltered production consumer → the class gate fails naming it:
  *"windows.rs:2137 calls low_integrity_label_ace( without an INHERIT_ONLY_ACE filter"*.

The rejection test also carries a non-vacuity check: the SACL walk must still SEE the ACE and it
must carry `INHERIT_ONLY_ACE`, so the `None` assertion cannot pass because nothing was planted.

## Task 2 — WR-34: gate the ACE shape, not the body

**Route taken: the source-text gate, narrowed.** The plan's default was a normalised whole-body
comparison. Measured first: the two bodies are **53 vs 61 normalised lines and differ from the very
first line**, because the `nono-cli` copy declares its `use` statements inside the function. Body
equality is therefore not implementable without manufacturing it, and would then fail on exactly
that harmless placement difference.

What CR-01/WR-18 depend on is that both crates plant the **same ACE shape**. Both construct
`format!("S:(ML;{ace_flags_sddl};0x{mask:X};;;LW)")` — byte-identical. `plant_mandatory_label_helpers_plant_the_same_ace_shape`
pins that.

The `test-support` feature route was **not** taken, and the plan's three prerequisites are recorded
as the reason: `crates/nono/Cargo.toml` has no such feature (only `system-keyring`);
`layer-fault-injection`, the cited precedent, is declared in `crates/nono-cli/Cargo.toml:48`, a
different crate; and `nono-cli` consumes `nono` with `default-features = false`, so enabling it for
tests needs a duplicate dev-dependency entry and would add a `pub` test-scaffolding module to a
published security crate.

**Writing this gate produced the same blind-by-construction bug twice.** `include_str!("windows.rs")`
pulls in the test itself, whose scanning code mentions `fn plant_mandatory_label_with_flags` in a
string literal at line ~3373 — **earlier in the file than the real declaration at ~3544**. So
`src.find("fn plant_...")` matched the test, and the extracted "SDDL" was the test's own
`.find(|l| l.contains("let sddl = format!("))` line. Fixed by anchoring on a line that *declares*
the fn (`trim_start().starts_with("fn plant_…")`) and brace-matching its body.

**Perturbation:** changing the CLI helper's SDDL to `;;;ME` (Medium IL) instead of `;;;LW` fails
the gate with both strings printed side by side.

## Task 3 — WR-18's ledger row corrected

Appended an Iteration 6 addendum (the row's original text preserved, per this phase's
append-don't-rewrite convention) stating the true enumeration: one SACL walk, three production
consumers, all three now hardened, enforced by a gate rather than a reviewer's grep.

**The drift gate 117-36 repaired caught my own edit.** The first version cited bare
`windows.rs::low_integrity_label_ace`, which the gate resolves relative to `exec_strategy_windows/`
→ unreadable. It also surfaced that I had cited
`hook_runtime_windows.rs::validate_hook_script` — **a symbol that does not exist**; the real
enclosing function is `validate_hook_script_windows`. Both fixed; all citations now use
workspace-relative paths and resolve.

That is the gate doing exactly what CR-04 proved it must: catching a citation the author believed
was correct.

## Public API impact

`low_integrity_label_and_mask` is `pub` and re-exported at `crates/nono/src/lib.rs:97`, so this is a
behaviour change on the published surface.

- `bindings/c/src/` — **no reference**; the C FFI does not expose it.
- `../nono-py` — **0 references** to `low_integrity_label` (checked read-only, not edited).
- `../nono-ts` — scan did not complete on this host (directory walk timed out, likely
  `node_modules`); **unverified**, recorded rather than assumed.

No sibling repo was edited, per the plan.

## Verification

| Gate | Command | Result |
|---|---|---|
| Core library | `cargo test -p nono-sandbox --lib --no-fail-fast -- --test-threads=1` | **843 passed / 0 failed** (was 840; +3 new) |
| Labels guard | `cargo test -p nono-sandbox-cli --bin nono --features layer-fault-injection --no-fail-fast -- --test-threads=1 labels_guard::` | **13 passed / 1 failed** — the 1 is WR-20's host-blocked test, baseline #12 |
| SPEC drift gate | `cargo test -p nono-sandbox-cli --test layer_registry_selfcheck --no-fail-fast` | **14 passed / 0 failed** |
| Format | `cargo fmt --all -- --check` | clean |
| Cross-target clippy (linux-gnu) | `cross clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |
| Cross-target clippy (apple-darwin) | `cargo-zigbuild clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 errors (the 1 "warning" is the pre-existing `nono-shell-broker` manifest notice emitted on every build here, not a lint) |

`--all-targets` used per the plan, since this changes a `pub` item in the core library — the v3.0
durable finding that a `--bin nono` gate hid `nono-ffi` exhaustive-match breaks.
