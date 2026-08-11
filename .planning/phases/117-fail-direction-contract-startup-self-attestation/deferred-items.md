# Deferred Items — Phase 117

Out-of-scope discoveries logged during plan execution, not fixed (per executor scope-boundary rule).

## 117-28

- **`crates/nono-cli/src/exec_strategy_windows/launch.rs:4227-4308`
  (`dacl_ancestor_traverse_row_reports_partially_applied_from_a_real_gate`)** — this
  test's docstring (lines 4227-4240) narrates that Task 1 of Plan 117-23 (WR-07) made
  the ancestor-traverse guard's `application()` reach `PartiallyApplied` for a walk
  that ran and granted zero owned ancestors. After D-37 (Plan 117-28), that specific
  scenario (stopped at the first non-owned ancestor) now classifies `NotApplicable`,
  not `PartiallyApplied` — see `dacl_guard.rs`'s
  `ancestor_traverse_application_reports_not_applicable_when_stopped_at_non_owned`.
  The test itself still passes and remains valid: it directly sets
  `applied.dacl_ancestor_traverse = PartiallyApplied` on a hand-built `AppliedLayers`
  to exercise `attestation::attest_and_decide`'s `ProceedDowngraded` gate behavior for
  a `PartiallyApplied` row in the abstract (which remains a legal, reachable value for
  OTHER layers, e.g. `mandatory_integrity_label` per `labels_guard.rs`) — it is not
  actually driven through the real guard, so the D-37 behavior change does not make it
  incorrect. Only the doc comment's narrative is now stale (it implies the real guard
  still produces `PartiallyApplied` for this scenario, which is no longer true).
  `exec_strategy_windows/launch.rs` is outside Plan 117-28's `files_modified` scope
  (`dacl_guard.rs`, `exec_strategy_windows/mod.rs`, `agent_daemon/launch.rs` only), so
  left as-is here. A future plan touching this file should update the docstring to
  cite D-37 and clarify the scenario now exercises the abstract `PartiallyApplied`
  gate path rather than a real WR-07-reachable guard outcome.
