# Deferred Items — Phase 110

> **Wave-1 boundary resolution (orchestrator, 2026-07-30).** Both items below were
> CLOSED at the wave-1/wave-2 boundary rather than carried to the phase gate:
>
> - **Plan 02's rustfmt item** → closed by `b7abf218` (`cargo fmt --all`; the 4 diffs in
>   `profile/mod.rs` were the only ones in the workspace).
> - **Plan 03's `supervisor_linux.rs` item** → closed by `17992adb`, on an explicit
>   operator decision to fix it before Wave 2 rather than defer. `ApprovalRequest` →
>   `CapabilityRequest`, `request_approval` → `request_capability` (matching the real
>   trait at `crates/nono/src/supervisor/mod.rs:120`), and the phantom
>   `tool_sandbox_runtime: None` line removed — that identifier was confirmed to appear
>   nowhere else in the workspace, so it was pure Phase-96 leftover referencing the
>   v3.7-excluded tool-sandbox subsystem, not a field anyone had dropped.
>   `cross test … exec_strategy::supervisor_linux` now runs **48/48 green**, including
>   Plan 03's three previously-unverified range tests
>   (`bind_within_port_range_is_allowed`, `bind_outside_port_range_is_denied`,
>   `bind_allowed_by_individual_port_or_range`). `cross clippy --workspace
>   --target x86_64-unknown-linux-gnu --all-targets -- -D warnings` exits 0.
>
> The original entries are retained below as the diagnostic record.

## Plan 02

- **`crates/nono-cli/src/profile/mod.rs` has 4 pre-existing `cargo fmt --check` diffs**
  (lines ~9164, ~9254, ~9402, ~9560), all inside Plan 110-01's `platform_overrides` test
  additions. Discovered while running `cargo fmt -p nono-sandbox-cli` for Plan 02's own
  changes (`cargo fmt` reformats the whole package, not just touched files). Out of scope
  for Plan 02 (file not modified by this plan's tasks) — reverted via
  `git checkout -- crates/nono-cli/src/profile/mod.rs` to avoid an unrelated diff.
  Left for whichever plan/task next touches `profile/mod.rs`, or a dedicated `cargo fmt`
  pass at the phase gate.

## Plan 03

- **`crates/nono-cli/src/exec_strategy/supervisor_linux.rs`'s `mod tests::network_decision`
  has a pre-existing, dormant Linux-target-only compile break, unrelated to PROF-03/port
  ranges.** Discovered by running `cross test --target x86_64-unknown-linux-gnu -p
  nono-sandbox-cli exec_strategy::supervisor_linux` (a step beyond this plan's mandated
  gate — the CLAUDE.md/checklist-mandated gate is `cross clippy --workspace` without
  `--all-targets`, which does NOT compile `#[cfg(test)]` code and therefore never exercises
  this module). The break: `DenyAllBackend`'s `impl ApprovalBackend` uses a stale
  `request_approval(&self, _req: &ApprovalRequest)` shape (`nono::supervisor::ApprovalRequest`
  does not exist) instead of the current trait's `request_capability(&self, _req:
  &CapabilityRequest)`; separately, `make_config`'s `SupervisorConfig` literal sets a
  `tool_sandbox_runtime: None` field that does not exist on `SupervisorConfig` at all.
  `git log -L` on the affected lines traces this to commit `1a804977` ("fix(96-01): restore
  linux-gnu cfg-gated fork invariants surfaced by cross clippy gate", 2026-06-26) — that
  restore reintroduced the OLDER `ApprovalRequest`/`request_approval` shape from
  `ae77d198^` without reconciling it against a later trait rename
  (`ApprovalRequest`→`CapabilityRequest`, `request_approval`→`request_capability`) that had
  already landed elsewhere. `git stash` confirms this predates any Plan 03 edit — genuinely
  pre-existing, not caused by this plan.
  **Impact on Plan 03:** this plan's 3 new tests (`bind_within_port_range_is_allowed`,
  `bind_outside_port_range_is_denied`, `bind_allowed_by_individual_port_or_range`) and the
  new `make_config_with_ranges` helper live in the SAME `mod network_decision` and cannot be
  executed on Linux until this pre-existing break is fixed — they are unverified by direct
  test run, though correct by construction (mirroring the exact pattern of the passing
  `af_inet_bind_on_disallowed_port_denied` test immediately above them, and the surrounding
  `SupervisorConfig`/`proxy_bind_port_ranges` field wiring is proven correct via a clean
  `cross clippy` AND full `cross test -p nono-sandbox sandbox::linux` pass for the sibling
  library-side range tests).
  **Not fixed here** (Scope Boundary: out of scope, unrelated file section, would expand
  this plan beyond PROF-03). Left for the phase-gate plan (110-08) or a dedicated fix task —
  whoever next needs `mod network_decision` to actually execute on Linux. Fix shape: rename
  `ApprovalRequest`→`CapabilityRequest` and `request_approval`→`request_capability` in the
  `DenyAllBackend` impl (mirroring the trait's real signature), and remove the
  `tool_sandbox_runtime: None` line from `make_config`'s literal (or add the field to
  `SupervisorConfig` if it was meant to exist — needs an intentional decision, not a blind
  auto-fix).
