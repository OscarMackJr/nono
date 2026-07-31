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

## Plan 08

Discovered while running `cargo test --workspace --no-fail-fast` for the phase-gate's
`make ci` constituent (Task 1). All three items below are confirmed pre-existing and
**unrelated to any file touched by any 110-0X plan** — verified via `git log --oneline --all
-- <file>` returning zero `110-0X` commits for each file named below. None were fixed here
per Scope Boundary (none intersect this phase's `files_modified`). They were not visible in
any prior phase-gate run because `cargo test --workspace` (default fail-fast) always aborted
earlier, at the pre-existing `-p nono-sandbox-cli --bin nono` failure (11 tests, documented
baseline) — `--no-fail-fast` was needed to see past it for the first time in this project's
history of phase-gate verifications.

- **`crates/nono-cli/tests/audit_attestation.rs` — 2 failures, hardcoded Unix path.**
  `audit_verify_reports_signed_attestation_with_pinned_public_key` and
  `rollback_signed_session_verifies_from_audit_dir_bundle` both spawn the sandboxed child
  command via a hardcoded `"/bin/pwd"` literal (lines 147, 209) with no `#[cfg(windows)]`
  fallback — `/bin/pwd` does not exist on Windows, so `nono` fails with `Command execution
  failed: /bin/pwd: cannot find binary path` before the test's actual assertion is ever
  reached. Reproduced standalone (`cargo test -p nono-sandbox-cli --test audit_attestation
  -- --test-threads=1`, clean process state, 2/2 fail identically both times). `git log
  --oneline --all -- crates/nono-cli/tests/audit_attestation.rs` shows no `110-0X` commit.
  Fix shape: a `#[cfg(windows)]` alternate command (e.g. `cmd /c cd`, matching the pattern
  already used elsewhere in this test suite per the file's own historical comment about
  cross-platform `pwd` substitutes) — needs an intentional decision, not a blind auto-fix.
- **`crates/nono-cli/tests/env_vars.rs` — 10 failures, live `windows_run_*` integration
  tests.** All 10 failing tests spawn a real `nono run` child and assert on its exit
  code/stdout; failures include exit-code mismatches (e.g. expected `Some(7)`, got
  `Some(1)`) and unexpanded Windows env-var placeholders (`CARGO_HOME=!CARGO_HOME!`
  appearing literally instead of expanded). Full list: `windows_run_allow_all_network_probe_connects`,
  `windows_run_blocks_live_block_net_without_enforcement`, `windows_run_executes_basic_command`,
  `windows_run_filters_dangerous_env_vars_and_keeps_safe_ones`,
  `windows_run_filters_host_toolchain_home_vars_without_runtime_dir`,
  `windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified`,
  `windows_run_live_default_profile_executes_command`, `windows_run_propagates_child_exit_code`,
  `windows_run_smoke_validates_stdout_stderr_and_exit_code`,
  `windows_run_supervised_blocks_runtime_capability_elevation_with_actionable_diagnostic`.
  This test binary took 986s (16+ min) for 62 tests — anomalously slow, suggesting host-state
  contention (the binary's own log output shows `label guard: path has pre-existing
  mandatory-label ACE; skipping apply + revert` against real host paths like
  `C:\Users\OMack\.local\bin`, meaning some prior session left non-default mandatory-label
  ACEs on real system paths this test binary probes). `git log --oneline --all -- crates/nono-cli/tests/env_vars.rs`
  shows no `110-0X` commit. Not re-diagnosed further (would require an isolated/clean host
  state and is orthogonal to any file this phase touches) — left as a host-environment
  finding for whoever next investigates Windows live-execution test flakiness.
- **`crates/nono-cli/tests/resl_nix_async_signal_safety.rs` — 1 failure, stale
  text-signature-match test.** `cr_01_no_format_macro_in_post_fork_child_branch` does a raw
  source-text search in `exec_strategy.rs` for the literal string
  `fn clear_close_on_exec(fd: i32) -> std::io::Result<()>`; the real, current signature (line
  3817 of `exec_strategy.rs`) is `fn clear_close_on_exec(fd: i32) -> Result<()>` (the crate's
  own `Result` type alias, not the spelled-out `std::io::Result`). Purely a stale assertion
  string, unrelated to any actual regression — the function itself is unchanged in behavior.
  `git log --oneline --all -- crates/nono-cli/tests/resl_nix_async_signal_safety.rs` shows no
  `110-0X` commit, and `git log --oneline -- crates/nono-cli/src/exec_strategy.rs` confirms no
  `110-0X` commit touches that file either. Fix shape: update the expected literal in the test
  to match the current `Result<()>` alias spelling.
