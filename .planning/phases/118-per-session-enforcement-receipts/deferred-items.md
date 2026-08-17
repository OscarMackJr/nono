# Deferred Items — Phase 118

Out-of-scope discoveries logged per the SCOPE BOUNDARY rule (only auto-fix issues directly caused
by the current task's changes; log everything else here without fixing).

## Discovered during Plan 118-04 (full `cargo test -p nono-sandbox-cli --all-targets --no-fail-fast` diligence run)

The plan's own mandatory verification gate (`cargo test -p nono-sandbox-cli --bin nono-agentd
agent_daemon::launch`, `cargo fmt --all -- --check`, `cargo clippy -p nono-sandbox-cli --all-targets
-- -D warnings -D clippy::unwrap_used`) is fully green. As additional diligence per critical_repo_constraints
#3 ("Any OTHER failing name is YOUR regression"), a full `--all-targets --no-fail-fast` run was
started. The primary `--bin nono` unit-test result matched the documented 12-failure baseline
EXACTLY (`1695 passed; 12 failed`, same 12 names: `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`,
6× `config::tests::*`, `exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`,
`profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`, 3× `protected_paths::tests::*`)
— zero regressions there.

Three ADDITIONAL failures surfaced in separate integration-test binaries, none previously enumerated
in the documented 12-name baseline list, and none touching `agent_daemon/`, `layer_registry.rs`,
`attestation.rs`, or `receipt.rs` (this plan's only modified file is `agent_daemon/launch.rs`):

- `audit_verify_reports_signed_attestation_with_pinned_public_key` (audit/sigstore integration test)
- `rollback_signed_session_verifies_from_audit_dir_bundle` (rollback/audit-dir integration test)
- `windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified` (live-run
  `LOCALAPPDATA` runtime-root integration test)

Not investigated or fixed — out of scope for Plan 118-04 per the SCOPE BOUNDARY rule (pre-existing,
unrelated files, not caused by this plan's changes). Flagging for the phase's verify-work pass
(Plan 118-10) to confirm/update the documented baseline figure, since these may represent additional
pre-existing host-environment-dependent failures not previously catalogued in
`nono_cli_windows_baseline_test_failures.md`.

The `--all-targets` run itself did not finish within a reasonable diligence window — the
`windows_run_*` live-run integration suite is documented elsewhere (project memory) as a ~25-minute
stall on this host (spawns real `nono.exe` child processes repeatedly). This is expected background
behavior, not a hang introduced by this plan.
