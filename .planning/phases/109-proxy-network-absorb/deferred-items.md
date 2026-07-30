# Deferred Items — Phase 109

## 109-01: pre-existing test failures unrelated to `deny_domain` (#1374)

Captured while running `cargo test -p nono-sandbox-cli --bin nono` after 109-01 Task 3.
These 11 failures are in files NOT touched by 109-01's `files_modified` list
(`audit_session.rs`, `config/mod.rs`, `profile_cmd.rs`, `protected_paths.rs`) and are
out of scope per the executor's SCOPE BOUNDARY rule. Not fixed; logged only.

- `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`
- `config::tests::nono_home_dir_falls_through_when_unset`
- `config::tests::nono_home_dir_rejects_non_absolute_override`
- `config::tests::nono_home_dir_returns_override_when_set`
- `config::tests::test_validated_home_falls_back_to_userprofile`
- `config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists`
- `config::tests::user_state_dir_uses_localappdata_on_windows`
- `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`
- `protected_paths::tests::blocks_child_directory_capability`
- `protected_paths::tests::blocks_parent_directory_capability`
- `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root`

Root cause class: parallel-test-execution races on this Windows dev host —
`config::tests::*` fail with `env lock: PoisonError { .. }` (a prior panicked test holding
the env-var mutex, per CLAUDE.md's "Environment variables in tests" guidance);
`profile_cmd`/`protected_paths` match the durable memory note
`nono_cli_windows_baseline_test_failures.md` ("4 pre-existing `cargo test -p nono-cli` fails
(profile_cmd init + 3 protected_paths)... env-specific, fail at phase-base too; don't chase
as regressions"). `audit_session` appears to be the same parallel-execution class.

Every 109-01-touched test module (`net_filter`, `filter`, `config` in `nono-proxy`,
`network_policy`, `sandbox_prepare`, `profile`, `proxy_runtime`, `cli`, `launch_runtime`)
passes 0 failures when run in isolation (see 109-01-SUMMARY.md verification section).
