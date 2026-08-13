---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 44
subsystem: windows-attestation
tags: [windows, d-27, d-28, operator-guidance, gap-closure, round-4, wr-26, wr-22, cint-02, cint-03]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-37's write_security_event_log (CR-05); 117-39's main.rs (WR-27); 117-35's baseline"
provides:
  - "DowngradeDetailChannel — the pointer an operator is given is derived from what actually received the record"
  - "EventLogDelivery — write_security_event_log reports where the write landed"
  - "emit_downgrade_diagnostics — the three emission sites as a pure, individually-testable function"
  - "every_operator_detail_pointer_is_conditional — the class gate, covering WR-27's main.rs site too"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "When a test cannot reach an arm because of a process-global OnceLock, take
      Option<&T> in a pure function rather than trying to set the global"
    - "An assertion that scans ALL captured events for a needle is satisfied by whichever
      site happens to fire; require a site-DISCRIMINATING marker in the SAME event"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/telemetry/windows.rs
    - crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/output.rs

key-decisions:
  - "WR-22 folded in (operator-approved) rather than planned separately: the review's own fallback
    fix — factor the three-site block into a pure function taking Option<&SecurityEventLayer> — is
    exactly the seam WR-26 needs, and a separate plan would have collided on launch.rs."
  - "PrivateLogFile takes precedence over EventLog when both apply: it is the channel the operator
    asked for and the only one permitted to carry LayerId names (D-28)."
  - "The banner's dedup marker deliberately does NOT key on channel — the same layer-set on the
    same session is still a repeat, whatever carried it."
---

# Plan 117-44 (+ WR-22 folded): Execution Summary

**Executed:** 2026-08-12/13 · **Wave:** 17 · **Requirements:** CINT-02, CINT-03

## Why the two findings are one refactor

WR-26 needs to know which channel received the downgrade record. WR-22 needs the three emission
sites to be individually observable. The review's own fallback fix for WR-22 — *"factor the
three-site emission block out of `apply_startup_attestation_gate` into a pure function taking
`Option<&SecurityEventLayer>`"* — is precisely the seam WR-26's channel derivation requires. A
separate plan would also have collided on `launch.rs`.

## Task 1 — the emission outcome made observable

117-37 deliberately left `write_security_event_log` returning `()`, and recorded that 117-44 would
need to change it. Added `EventLogDelivery { EventLog, StderrFallback, NotAvailable }`, returned by
`write_security_event_log` and `emit_security_event`.

`emit_attestation_event` now returns `Result<(String, DowngradeDetailChannel), &'static str>`. The
`enabled` flag and the delivery are captured through `Cell`s read *after* `advance_and_emit`
returns — **not** by widening that method's signature, because `telemetry/mod.rs` is 117-41's file
and WR-21/WR-30 constrain its locking discipline. No borrow outlives the closure; the chain mutex is
untouched.

All three emptying conditions are now representable:

| Condition | Result |
|---|---|
| `SECURITY_LAYER` absent | `None` |
| telemetry `enabled == false` | `None` (chain still advances — D-14 — so `Ok` alone never proved an emission) |
| `RegisterEventSourceW` NULL | `Stderr` (redacted, per CR-05) |
| emission succeeded | `EventLog` |

Each `EventLogDelivery` variant is annotated per-platform: `NotAvailable` is dead on Windows, the
other two on non-Windows.

## Task 2 — render only the channel that received it

`downgrade_detail_pointer` (operator warn) and the banner both `match` the channel with **no `_`
arm**, so a future variant is a compile error rather than a silently wrong pointer.

**A regression I introduced and caught:** the first extraction replaced site 3's suffix with
`downgrade_detail_pointer(channel)` unconditionally, which returns empty for `PrivateLogFile` — so
on the private channel the operator's own log would have stopped receiving the layer names, a
D-27/NR3-04 regression. Restored the original private-vs-shared semantics (`layer_detail` when
private, else a channel-correct destination sentence).

## Task 4 — WR-22

### (a) The private-channel assertion was vacuous; it now discriminates

The perturbation (deleting `layer_detail` from site 3) produced the finding's evidence verbatim:

```
[("message", "attestation downgrade audit emission unavailable (AUD-04: SecurityEventLayer not
  initialized) — proceeding per AUD-04's non-fatal contract; downgraded_layers=MandatoryIntegrityLabel")]
[("message", "startup self-attestation: 1 confinement layer(s) could not be fully confirmed at
  startup — proceeding with a downgraded confinement claim"), ("downgraded_count", "1")]
```

Site 1 carries the LayerId name and no `downgraded_count`; site 3 carries the marker. The old
assertion scanned **any** event for the name, so site 1 alone satisfied it. It now requires the
`downgraded_count` marker and the LayerId name **in the same captured event**, and **fails** under
exactly that perturbation — which it would not have done before this task.

### (b) Sites 2 and 3 had zero coverage; they now have direct tests

`SECURITY_LAYER` is never `set` in a test binary — confirmed: the only `set` call sites are
`cli_bootstrap.rs:400` and `agent_daemon/telemetry_init.rs:75`, neither reachable from a unit test.
Driving `emit_downgrade_diagnostics` directly avoids the `OnceLock` entirely:

- `emit_downgrade_diagnostics_site_1_absent_layer_reports_no_channel`
- `emit_downgrade_diagnostics_site_2_emission_error_reports_no_channel` (via `poison_for_test`)
- `emit_downgrade_diagnostics_site_3_success_reports_the_real_channel`

Sites 2 and 3 each assert both the shared-console and private-log cases. The site-3 shared-console
assertion is a **disjunction** (`EventLog | Stderr`) because the answer depends on whether this
host's `nono` event source is registered; encoding one host's configuration would make the test
assert the environment rather than the behaviour.

### (c) The false doc claim corrected

`..._withholds_layer_names_on_the_shared_console_channel` claimed to drive the
`SECURITY_LAYER`-present arm. It does not, and never did — both shared-console tests exercise the
identical absent arm. Corrected, with a pointer to the tests that do cover the present arms.

## Task 3 — the class gate

Enumerated first (per the plan), then gated. All five production pointers now sit inside
`match channel` arms:

| Site | Conditional? |
|---|---|
| `launch.rs` `downgrade_detail_pointer` — EventLog / Stderr / None arms | yes |
| `output.rs` banner — EventLog / None arms | yes |
| `main.rs` `render_error_for_operator` `_` arm | WR-27, fixed by 117-39 — names `--log-file`, and states the event log is empty on this path |

`every_operator_detail_pointer_is_conditional` scans the production halves of `output.rs`,
`launch.rs` and `main.rs`, requiring every mention of the Application event log to sit within a
`DowngradeDetailChannel::EventLog` arm, and requiring `main.rs` never to name it as a destination at
all. Doc comments and test modules are excluded — prose *about* the rule is not an instance of it,
and 117-39's tests legitimately assert on these strings.

**Perturbation:** replacing the `None` arm's text with an event-log pointer fails the gate naming
`output.rs:133` and the offending arm.

## Deviations

1. **`attestation_downgrade_event` made `pub(crate)`** so `output.rs` can name the channel type in
   the banner's signature.
2. **Site 3's suffix logic preserved** rather than replaced (see Task 2) — the plan's wording
   implied replacing `operator_suffix` wholesale, which drops the private-channel names.

## Verification

| Gate | Result |
|---|---|
| `output::` | **13 passed / 0 failed** |
| `exec_strategy::launch` | **67 passed / 0 failed**, 2 ignored |
| `attestation_downgrade_event::` | **5 passed / 0 failed** |
| `telemetry::` | **45 passed / 0 failed** |
| `render_error_for_operator` | **3 passed / 0 failed** |
| `cargo fmt --all -- --check` | clean |
| Cross-target clippy (apple-darwin, `--all-targets`) | **PASS** — exit 0, 0 errors |
| Cross-target clippy (linux-gnu, `--all-targets`) | **PASS** — exit 0, 0 errors (on re-run; see note below) |

**linux-gnu runner note:** the first attempt exited 1 with **zero lint diagnostics** — Docker could
not open its context metadata (`open ...meta.json: The process cannot access the file because it is
being used by another process`), a transient Docker Desktop file lock. That is a runner failure, not
a lint failure, and per CLAUDE.md a transient lock does not qualify for PARTIAL→CI, so the gate was
re-run rather than deferred.
