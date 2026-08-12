# Phase 117 — Gap-Closure Round 4 Test Baseline

**Captured:** 2026-08-12 (run 03:37:09Z → 04:16:38Z, 39m29s wall)
**Command:** `cargo test --workspace --no-fail-fast`
**HEAD at capture:** `9ef519cfe064b7fd87738b99e3c8d40581978533` (`docs(117): add gap-closure round 4 plans (35-45)`)
**Host:** Windows 11 Enterprise 10.0.26200, non-elevated domain account
**Plan:** 117-35 (wave 15) — runs before any round-4 code fix, so this is the true pre-fix state.

Produced by plan 117-35 so every later plan in this round (117-36 … 117-45) has a concrete,
dated, complete list of what is and is not round 4's responsibility. The raw untruncated capture
is at the end of this file under [Raw --no-fail-fast output](#raw---no-fail-fast-output-round-4-pre-fix).

---

## Result

```
error: 5 targets failed:
    `-p nono-sandbox-cli --bin nono`
    `-p nono-sandbox-cli --test audit_attestation`
    `-p nono-sandbox-cli --test env_vars`
    `-p nono-sandbox-cli --test layer_registry_selfcheck`
    `-p nono-sandbox-cli --test resl_nix_async_signal_safety`
```

**Five targets, 19 failing tests.** Every other workspace target — including all `nono` library
tests, `nono-proxy`, `nono-ffi`, and all doc-tests — passed.

### Two corrections to expectations recorded before this run

1. **The count is 5, not 6.** Plan 117-35's own `<objective>` states that
   `cargo test --workspace --no-fail-fast` "reports `error: 6 targets failed`". This run reports 5.
   The earlier figure came from an orchestrator investigation whose capture was not retained, so
   the sixth target cannot be identified and the delta cannot be explained. **This is not evidence
   that a target was fixed** — no round-4 code has landed. Treat the 5 targets below as the
   authoritative baseline and treat the "6" as unreconciled. If a sixth target appears in a later
   run, it is a regression against *this* file, not a return to a known state.

2. **The `=== cargo exit code: 0 ===` line in the raw capture is WRONG — ignore it.** The capture
   script emitted a blank line before reading `$?`, so that line records the exit status of an
   `echo`, not of cargo. Cargo's real verdict is the `error: 5 targets failed:` block, which is
   present and authoritative. Recorded here rather than silently deleted because a stray `exit
   code: 0` inside a baseline artifact is exactly the kind of false green this plan exists to
   eliminate.

---

## Failing-target classification

| Target | Failing test(s) | Disposition | Evidence |
|---|---|---|---|
| `-p nono-sandbox-cli --bin nono` | 12 (1624 passed / 12 failed / 2 ignored) — full list below | **pre-existing** | Count matches the documented Windows-host baseline exactly: 11 HOME/env-race failures + WR-20's host-blocked test = 12. No new name. |
| `-p nono-sandbox-cli --test audit_attestation` | `audit_verify_reports_signed_attestation_with_pinned_public_key`, `rollback_signed_session_verifies_from_audit_dir_bundle` (0 passed / 2 failed) | **pre-existing — host-environmental (Unix-only test on a Windows host)** | Both fail with `nono: Command execution failed: /bin/pwd: cannot find binary path`. The test hardcodes `/bin/pwd` at `crates/nono-cli/tests/audit_attestation.rs:147` and `:209`. `git log -- crates/nono-cli/tests/audit_attestation.rs` shows no Phase 117 commit; the four most recent are `fix(test):` commits predating this phase. |
| `-p nono-sandbox-cli --test env_vars` | `windows_run_allow_all_network_probe_connects` | **pre-existing — host-environmental (build-artifact size)** | `nono: Snapshot error: Rollback budget exceeded: 2153433890 bytes tracked (limit: 2147483648 bytes)`. The test grants `target\debug` r+w; that directory has grown past the 2 GiB rollback budget. Not a code defect and not fixable by round 4. **Caveat:** this test WAS touched by Phase 117 commit `76432e05` (CR-08), but the observed failure cause is the rollback budget, unrelated to that change. |
| `-p nono-sandbox-cli --test env_vars` | `windows_run_blocks_live_block_net_without_enforcement` | **UNCLASSIFIED — needs investigation** | Asserts "Windows preview should block live network restriction requests"; the run shows nono proceeding normally (`net outbound blocked`, `Applying sandbox...`). Reads as a stale preview-era expectation, but that cannot be established from the captured output alone. `env_vars.rs` was modified by Phase 117 commit `76432e05` (CR-08, "stop four Windows block-net tests from failing or passing vacuously"), so a 117 relationship is plausible and must be ruled in or out by reading, not assumed. |
| `-p nono-sandbox-cli --test env_vars` | `windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified` | **UNCLASSIFIED — needs investigation** | `nono: Sandbox initialization failed: Refusing to grant 'C:\Users\OMack\AppData\Local\Temp\.tmpJUlGb9' (source: CLI) because it overlaps protected nono state root '…\.tmpJUlGb9\fake-localappdata\nono'`. The fixture nests its fake LOCALAPPDATA inside the granted temp dir, which the protected-path overlap rule rejects. Whether the fixture or the policy is wrong is not determinable from the output. |
| `-p nono-sandbox-cli --test layer_registry_selfcheck` | `every_spec_symbol_citation_resolves_to_a_real_definition` (10 passed / 1 failed) | **round-4 regression (CR-04)** | The round's own SPEC drift gate, built by 117-32, failing on HEAD because 117-34 (a later wave, same round) wrote a ledger row with an unquoted illustrative `` `file.rs::Symbol` `` span. **Fixed by 117-36.** |
| `-p nono-sandbox-cli --test resl_nix_async_signal_safety` | `cr_01_no_format_macro_in_post_fork_child_branch` (4 passed / 1 failed) | **pre-existing** | Red since ~Phase 25. Asserts a `clear_close_on_exec` signature in `exec_strategy.rs` that no longer matches — itself an instance of "a test that NAMES its target instead of discovering it", the class this phase exists to close. |

Row count (5 distinct targets) matches the `error: 5 targets failed:` line. No failing target from
the capture is omitted.

### The 12 `--bin nono` failures, in full

Confirmed against the documented count rather than accepted silently:

```
audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty
config::tests::nono_home_dir_falls_through_when_unset
config::tests::nono_home_dir_rejects_non_absolute_override
config::tests::nono_home_dir_returns_override_when_set
config::tests::test_validated_home_falls_back_to_userprofile
config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists
config::tests::user_state_dir_uses_localappdata_on_windows
exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap
profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name
protected_paths::tests::blocks_child_directory_capability
protected_paths::tests::blocks_parent_directory_capability
protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root
```

11 HOME/env-race failures + `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`
(WR-20's intentionally-loud host-blocked test — this dev host's shell is a non-elevated domain
account with no `SeTakeOwnershipPrivilege`) = 12. **Count matches exactly; no new name appeared.**

---

## Round-4 responsibility

**Round 4 is responsible for exactly one target in this baseline:**
`layer_registry_selfcheck` (CR-04), fixed by 117-36.

**Round 4 is NOT responsible for:** the 12 `--bin nono` failures, the 2 `audit_attestation`
failures, `resl_nix_async_signal_safety`, or `env_vars::windows_run_allow_all_network_probe_connects`.
A round-4 plan whose verify run shows these MUST reconcile against this file and proceed, not chase
them as regressions.

**Two items are neither:** the two `UNCLASSIFIED` `env_vars` failures. They are not round-4's
responsibility to fix, but they must not be laundered into "pre-existing" either — they are
flagged as candidate operator-facing items in 117-35-SUMMARY.md and need a decision before this
phase closes.

---

## Verification rule for this round

**Every plan in gap-closure round 4 (117-36 through 117-45) whose `<verify>` asserts suite-level
state MUST use `--no-fail-fast`.** Never bare `cargo test --workspace`, and never
`cargo test -p <crate>` without it.

The reason is structural, not stylistic: **`cargo test` is fail-fast ACROSS targets.** On this
workspace `--bin nono` runs first and hits the 12 documented failures, and cargo then stops
without ever building or running a single `tests/*.rs` integration binary. Six review iterations,
eight executor self-checks, and multiple orchestrator passes all reported "matches baseline" while
never executing the integration targets. CR-04 shipped through 117-32's plan, 117-34's dependent
plan, and 117-34's own full-suite verify for exactly this reason — the gate could not see it.

This rule was audited across the round at capture time: all 20 `cargo test` commands in the
`<verify>` blocks of 117-36 … 117-45 carry `--no-fail-fast`. Any plan added to this round later
must match.

**Note on `make test` / `make ci`:** CLAUDE.md treats these as the canonical targets. They wrap
`cargo test` without `--no-fail-fast` and therefore carry the same blind spot. Round-4 plans use
the explicit `cargo test … --no-fail-fast` form instead. Changing the Makefile is out of scope for
this plan and is recorded as a follow-up candidate.

**Run cost:** ~40 minutes wall clock. The long pole is `env_vars` at 2061s (34m) on its own — it
spawns real supervised child processes, several individually exceeding 60s.

---

## Raw --no-fail-fast output (round 4, pre-fix)

Full capture, stdout and stderr interleaved, untruncated. The `=== cargo exit code: 0 ===` line
near the end is a capture-script artifact and does not reflect cargo's verdict — see
[the corrections above](#two-corrections-to-expectations-recorded-before-this-run).

```text
=== command: cargo test --workspace --no-fail-fast
=== started: 2026-08-12T03:37:09Z
=== HEAD: 9ef519cfe064b7fd87738b99e3c8d40581978533

warning: nono-sandbox-cli v0.70.0 (C:\Users\OMack\Nono\crates\nono-cli) ignoring invalid dependency `nono-shell-broker` which is missing a lib target
   Compiling nono-sandbox-cli v0.70.0 (C:\Users\OMack\Nono\crates\nono-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 33s
     Running unittests src\lib.rs (target\debug\deps\nono_ffi-23da6b230e80cdcc.exe)

running 49 tests
test capability_set::tests::test_allow_path_null_caps ... ok
test capability_set::tests::test_allow_path_null_path ... ok
test capability_set::tests::test_allow_path_nonexistent ... ok
test capability_set::tests::test_allow_path_invalid_mode ... ok
test capability_set::tests::test_capability_set_lifecycle ... ok
test capability_set::tests::test_free_null_safe ... ok
test capability_set::tests::test_commands ... ok
test capability_set::tests::test_allow_path_valid ... ok
test capability_set::tests::test_network_mode ... ok
test capability_set::tests::test_network_blocking ... ok
test capability_set::tests::test_network_mode_null_safe ... ok
test capability_set::tests::test_set_network_blocked_null ... ok
test capability_set::tests::test_summary ... ok
test capability_set::tests::test_summary_null_safe ... ok
test diagnostic::tests::diagnostic_code_is_cleared_between_calls ... ok
test diagnostic::tests::last_diagnostic_code_defaults_to_other ... ok
test diagnostic::tests::merge_diagnostic_report_json_rejects_null_session ... ok
test diagnostic::tests::merge_diagnostic_report_json_wraps_proxy_array ... ok
test diagnostic::tests::session_report_json_from_empty_arrays ... ok
test diagnostic::tests::string_getter_clears_stale_diagnostic_state ... ok
test diagnostic::tests::session_report_json_from_denial_array ... ok
test fs_capability::tests::test_fs_count_empty ... ok
test fs_capability::tests::test_fs_accessors_after_add ... ok
test fs_capability::tests::test_fs_count_null_safe ... ok
test fs_capability::tests::test_fs_out_of_bounds ... ok
test query::tests::test_query_context_lifecycle ... ok
test query::tests::test_query_context_null_safe ... ok
test query::tests::test_query_network_allowed ... ok
test query::tests::test_query_network_blocked ... ok
test query::tests::test_query_path_denied ... ok
test query::tests::test_query_path_granted ... ok
test sandbox::tests::test_apply_null_safe ... ok
test sandbox::tests::test_is_supported ... ok
test sandbox::tests::test_support_info ... ok
test state::tests::test_state_from_invalid_json ... ok
test state::tests::test_state_json_roundtrip ... ok
test state::tests::test_state_lifecycle ... ok
test state::tests::test_state_null_safe ... ok
test state::tests::test_state_to_caps ... ok
test tests::action_required_maps_to_err_config_parse ... ok
test tests::broker_not_found_maps_to_err_sandbox_init ... ok
test tests::map_error_unsupported_kernel_feature_returns_err_unsupported_platform ... ok
test tests::test_last_error_independent_copies ... ok
test tests::test_last_error_initially_null ... ok
test tests::test_rust_string_to_c_rejects_interior_nul ... ok
test tests::test_rust_string_to_c_roundtrip ... ok
test tests::test_set_and_get_error ... ok
test tests::test_string_free_null_safe ... ok
test tests::test_version_not_null ... ok

test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src\lib.rs (target\debug\deps\nono_fltmgr_client-37a6f8e3540b018a.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\main.rs (target\debug\deps\nono_fltmgr_client-b7ebfcab01ad546d.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\lib.rs (target\debug\deps\nono-dda21582704cb10e.exe)

running 840 tests
test attestation::tests::layer_attestation_status_derives_and_matches_exhaustively ... ok
test agent::tests::read_sid_current_process_returns_none ... ok
test agent::tests::classify_nonexistent_pid_not_agent ... ok
test agent::tests::insert_and_classify_requires_registry_membership ... ok
test agent::tests::classify_current_process_not_agent ... ok
test agent::remove_tests::remove_nonexistent_sid_is_idempotent ... ok
test agent::remove_tests::remove_decrements_registry_membership ... ok
test agent::remove_tests::insert_and_remove_leaves_registry_empty ... ok
test attestation::tests::process_handle_compiles_as_function_parameter_on_every_target ... ok
test attestation::windows_probe_tests::app_container_sid_probe_on_current_process_returns_ok_none ... ok
test attestation::windows_probe_tests::in_job_probe_on_invalid_handle_returns_layer_attestation_failed ... ok
test attestation::windows_probe_tests::in_job_probe_with_null_job_handle_is_rejected_fail_closed ... ok
test attestation::windows_probe_tests::app_container_sid_probe_on_invalid_handle_returns_layer_attestation_failed ... ok
test attestation::windows_probe_tests::integrity_level_probe_on_invalid_handle_returns_layer_attestation_failed ... ok
test attestation::windows_probe_tests::restricted_sids_probe_on_invalid_handle_returns_layer_attestation_failed ... ok
test audit::tests::inclusion_proof_rejects_tampered_leaf ... ok
test audit::tests::inclusion_proof_round_trips_each_leaf ... ok
test audit::tests::ledger_rejects_malformed_session_id ... ok
test attestation::windows_probe_tests::in_job_probe_distinguishes_the_assigned_job_from_another_job ... ok
test audit::tests::policy_override_applied_zt_audit_hash_absent_when_none ... ok
test audit::tests::policy_override_applied_round_trips ... ok
test audit::tests::recorder_produces_integrity_summary ... ok
test audit::tests::policy_override_applied_serializes_with_type_tag ... ok
test audit::tests::ledger_appends_and_verifies_session_digest ... ok
test audit::tests::record_session_started_scrubs_command_secrets ... ok
test audit::tests::session_digest_changes_when_protected_fields_change ... ok
test audit::tests::rust_compatibility_golden_vectors ... ok
test audit::tests::verify_empty_log_with_no_stored_metadata_is_not_valid ... ok
test capability::procfs_remap_tests::remap_procfs_self_rewrites_dev_fd_aliases ... ok
test audit::tests::audit_attestation_bundle_round_trips_in_core ... ok
test capability::procfs_remap_tests::remap_procfs_self_rewrites_proc_self_capability ... ok
test capability::tests::test_access_mode_contains ... ok
test capability::tests::test_allow_gpu_builder_sets_flag ... ok
test capability::tests::test_allow_gpu_does_not_affect_other_capabilities ... ok
test capability::tests::test_block_network_sets_blocked_mode ... ok
test capability::tests::test_allow_https_convenience ... ok
test capability::tests::test_capability_set_allow_unix_socket_accumulates ... ok
test audit::tests::verifier_round_trips_all_current_audit_event_payload_variants ... ok
test capability::tests::test_capability_set_builder ... ok
test capability::tests::test_capability_set_deduplicate ... ok
test capability::tests::test_capability_set_unix_socket_allowed_subtree_grant ... ok
test capability::tests::test_capability_set_unix_socket_allowed_directory_grant ... ok
test capability::tests::test_capability_set_unix_socket_allowed_mode_split ... ok
test capability::tests::test_deduplicate_identical_symlink_entries_collapsed ... ok
test capability::tests::test_deduplicate_merges_read_and_write_to_readwrite ... ok
test capability::tests::test_deduplicate_merges_write_then_read_to_readwrite ... ok
test capability::tests::test_capability_set_unix_socket_send_covered_by_connect_grant ... ok
test capability::tests::test_deduplicate_symlink_and_direct_are_kept_separately ... ok
test capability::tests::test_deduplicate_unix_sockets_does_not_widen_user_intent ... ok
test capability::tests::test_deduplicate_unix_sockets_keeps_file_and_dir_grants_separate ... ok
test capability::tests::test_deduplicate_user_upgrades_group_read_to_readwrite ... ok
test capability::tests::test_deduplicate_user_wins_over_system ... ok
test capability::tests::test_deduplicate_user_wins_over_system_reverse_order ... ok
test capability::tests::test_deduplicate_user_write_merges_with_group_read ... ok
test capability::tests::test_deduplicate_unix_sockets_promotes_connect_to_connect_bind ... ok
test capability::tests::test_deduplicate_unix_sockets_merges_identical_grants ... ok
test capability::tests::test_extensions_flag ... ok
test capability::tests::test_extensions_flag_mutable ... ok
test capability::tests::test_fs_capability_dir_as_file_error ... ok
test capability::tests::test_fs_capability_new_dir ... ok
test capability::tests::test_fs_capability_file_as_dir_error ... ok
test capability::tests::test_fs_capability_nonexistent ... ok
test capability::tests::test_fs_capability_new_file ... ok
test capability::tests::test_gpu_default_is_false ... ok
test capability::tests::test_ipc_mode_default_is_shared_memory_only ... ok
test capability::tests::test_ipc_mode_full ... ok
test capability::tests::test_ipc_mode_mutable_setter ... ok
test capability::tests::test_localhost_port_builder ... ok
test capability::tests::test_localhost_port_mutable ... ok
test capability::tests::test_localhost_port_range_builder ... ok
test capability::tests::test_localhost_port_range_mutable ... ok
test capability::tests::test_localhost_port_range_rejects_zero_start ... ok
test capability::tests::test_merge_port_ranges_adjacent ... ok
test capability::tests::test_merge_port_ranges_contained ... ok
test capability::tests::test_merge_port_ranges_empty ... ok
test capability::tests::test_merge_port_ranges_no_overlap ... ok
test capability::tests::test_merge_port_ranges_overlapping ... ok
test capability::tests::test_merge_port_ranges_three_overlap ... ok
test capability::tests::test_merge_port_ranges_unsorted_input ... ok
test capability::tests::test_network_mode_default_is_allow_all ... ok
test capability::tests::test_network_mode_display ... ok
test capability::tests::test_network_mode_serialization ... ok
test capability::tests::test_network_mode_serialization_with_bind_ports ... ok
test audit::tests::verifier_rejects_alpha_records_missing_event_json ... ok
test capability::tests::test_path_covered_basic ... ok
test capability::tests::test_path_covered_not_matching ... ok
test capability::tests::test_platform_rule_validation_rejects_comment_bypass ... ok
test capability::tests::test_path_covered_with_access_read_parent_does_not_satisfy_readwrite ... ok
test capability::tests::test_platform_rule_validation_rejects_malformed ... ok
test capability::tests::test_path_covered_with_access_readwrite_parent_satisfies_all ... ok
test capability::tests::test_path_covered_with_access_file_caps_ignored ... ok
test capability::tests::test_platform_rule_validation_rejects_root_access ... ok
test capability::tests::test_platform_rule_validation_rejects_unbalanced_parens ... ok
test capability::tests::test_platform_rule_validation_rejects_unterminated_constructs ... ok
test capability::tests::test_platform_rule_validation_rejects_whitespace_bypass ... ok
test capability::tests::test_platform_rule_validation_valid_deny ... ok
test capability::tests::test_process_info_mode_allow_all ... ok
test capability::tests::test_process_info_mode_default_is_isolated ... ok
test capability::tests::test_process_info_mode_allow_same_sandbox ... ok
test capability::tests::test_proxy_only_mode ... ok
test capability::tests::test_proxy_only_with_bind_ports ... ok
test capability::tests::test_set_gpu_toggles_flag ... ok
test capability::tests::test_set_network_blocked_backward_compat ... ok
test capability::tests::test_set_network_mode_builder ... ok
test capability::tests::test_signal_mode_allow_same_sandbox_roundtrip ... ok
test capability::tests::test_summary_includes_network_mode ... ok
test capability::tests::test_summary_includes_tcp_ports ... ok
test capability::tests::test_tcp_bind_ports ... ok
test capability::tests::test_tcp_connect_ports ... ok
test capability::tests::test_tcp_ports_mutable ... ok
test capability::tests::test_unix_socket_connect_bind_fails_when_parent_missing ... ok
test capability::tests::test_summary_includes_unix_sockets ... ok
test capability::tests::test_unix_socket_connect_on_existing_file ... ok
test capability::tests::test_unix_socket_covers_directory_one_level ... ok
test capability::tests::test_unix_socket_connect_bind_allows_nonexistent_path ... ok
test capability::tests::test_unix_socket_dir_nonexistent ... ok
test capability::tests::test_unix_socket_covers_file_exact_match ... ok
test capability::tests::test_unix_socket_covers_does_not_string_prefix ... ok
test capability::tests::test_unix_socket_dir_on_existing_directory ... ok
test capability::tests::test_unix_socket_dir_rejects_file_path ... ok
test capability::tests::test_unix_socket_covers_directory_subtree ... ok
test capability::tests::test_unix_socket_dir_rejects_filesystem_root ... ok
test capability::tests::test_unix_socket_op_display ... ok
test capability::tests::test_unix_socket_subtree_on_existing_directory ... ok
test diagnostic::observation::tests::network_hint_emits_sandbox_denied_network ... ok
test capability::tests::test_unix_socket_display ... ok
test capability::tests::test_unix_socket_file_rejects_directory_path ... ok
test diagnostic::records::tests::ipc_denial_record_keeps_legacy_suggested_flag_in_sync ... ok
test diagnostic::observation::tests::observation_input_builds_likely_sandbox_diagnostic ... ok
test diagnostic::records::tests::seatbelt_non_filesystem_operations_map_to_none ... ok
test diagnostic::records::tests::seatbelt_read_operations_map_to_read ... ok
test diagnostic::records::tests::seatbelt_write_operations_map_to_write ... ok
test diagnostic::report::tests::dedupe_denials_merges_access_modes ... ok
test diagnostic::report::tests::merge_with_proxy_json_wraps_session_and_proxy_arrays ... ok
test diagnostic::report::tests::insufficient_access_includes_grant_path_remediation ... ok
test diagnostic::report::tests::non_fs_violation_becomes_system_service_diagnostic ... ok
test diagnostic::report::tests::session_report_builds_diagnostics_from_denials ... ok
test diagnostic::report::tests::session_report_json_roundtrip ... ok
test diagnostic::report::tests::unix_socket_path_denial_includes_grant_remediation ... ok
test diagnostic::report::tests::unix_socket_path_denial_skipped_when_ipc_denials_present ... ok
test diagnostic::report::tests::violation_includes_grant_path_remediation ... ok
test error::broker_not_found_tests::broker_not_found_displays_path ... ok
test error::broker_not_found_tests::broker_not_found_is_debug ... ok
test error::diagnostic_tests::cwd_prompt_maps_to_structured_code_and_remediation ... ok
test error::diagnostic_tests::layer_attestation_failed_maps_to_clear_stale_layer_residue_remediation ... ok
test error::diagnostic_tests::layer_attestation_failed_maps_to_own_diagnostic_code ... ok
test error::diagnostic_tests::rollback_budget_error_maps_to_structured_code ... ok
test error::diagnostic_tests::secret_not_found_maps_to_credential_not_found ... ok
test error::tests::action_required_display_does_not_leak_env ... ok
test error::tests::action_required_display_format_base_hash ... ok
test error::tests::action_required_is_pattern_matchable ... ok
test error::tests::label_apply_failed_display_includes_path_hresult_and_hint ... ok
test error::tests::label_apply_failed_is_propagatable_via_result_alias ... ok
test error::unsupported_kernel_feature_tests::unsupported_kernel_feature_display_contains_cgroup_no_v1_hint ... ok
test error::unsupported_kernel_feature_tests::unsupported_kernel_feature_is_debug ... ok
test error::unsupported_kernel_feature_tests::unsupported_kernel_feature_is_pattern_matchable ... ok
test keystore::tests::keyring_timeout_tests::call_with_keyring_timeout_none_runs_inline ... ok
test keystore::tests::keyring_timeout_tests::keyring_timeout_invalid_falls_back_to_default ... ok
test keystore::tests::keyring_timeout_tests::keyring_timeout_unset_returns_default_120s ... ok
test keystore::tests::keyring_timeout_tests::keyring_timeout_zero_returns_none ... ok
test keystore::tests::system_keystore_label_mentions_windows_credential_manager ... ok
test keystore::tests::keyring_timeout_tests::keyring_timeout_valid_value_returns_some ... ok
test keystore::tests::keyring_timeout_tests::call_with_keyring_timeout_fast_call_returns_value ... ok
test keystore::tests::test_apply_keyring_decode_go_keyring_invalid_base64 ... ok
test keystore::tests::test_apply_keyring_decode_go_keyring_missing_prefix ... ok
test keystore::tests::test_apply_keyring_decode_go_keyring_valid ... ok
test keystore::tests::test_apply_keyring_decode_none_passthrough ... ok
test keystore::tests::test_build_mappings_apple_password_uri_legacy_equals_suffix_rejected ... ok
test keystore::tests::test_build_mappings_apple_password_uri_dangerous_target_rejected ... ok
test keystore::tests::test_build_mappings_apple_password_uri_rejected_in_list_mode ... ok
test keystore::tests::test_build_mappings_apple_password_uri_with_inline_var_rejected_in_list_mode ... ok
test keystore::tests::test_build_mappings_bw_uri_with_var ... ok
test keystore::tests::test_build_mappings_empty ... ok
test keystore::tests::test_build_mappings_env_uri_auto_derive ... ok
test keystore::tests::test_build_mappings_bw_uri_without_var ... ok
test keystore::tests::test_build_mappings_env_uri_dangerous_rejected ... ok
test keystore::tests::test_build_mappings_env_uri_empty_var_rejected ... ok
test keystore::tests::test_build_mappings_env_uri_explicit_dangerous_target_rejected ... ok
test keystore::tests::test_build_mappings_env_uri_with_explicit_var ... ok
test keystore::tests::test_build_mappings_file_uri_requires_explicit_var ... ok
test keystore::tests::test_build_mappings_file_uri_without_var_name_is_error ... ok
test keystore::tests::test_build_mappings_from_list ... ok
test keystore::tests::test_build_mappings_from_pairs_empty_credential_ref_rejected ... ok
test keystore::tests::test_build_mappings_from_pairs_keyring_and_uri ... ok
test keystore::tests::test_build_mappings_handles_whitespace ... ok
test keystore::tests::test_build_mappings_keyring_dangerous_autoderived_rejected ... ok
test keystore::tests::test_build_mappings_mixed_keyring_and_op ... ok
test keystore::tests::test_build_mappings_mixed_keyring_op_env ... ok
test keystore::tests::test_build_mappings_op_uri_dangerous_target_rejected ... ok
test keystore::tests::test_build_mappings_op_uri_empty_var_rejected ... ok
test keystore::tests::test_build_mappings_op_uri_invalid_uri_rejected ... ok
test keystore::tests::test_build_secret_mappings_explicit_pairs_take_precedence ... ok
test keystore::tests::test_build_mappings_op_uri_with_var_name ... ok
test keystore::tests::test_build_mappings_op_uri_without_var_rejected ... ok
test keystore::tests::test_classify_op_error_auth_required ... ok
test keystore::tests::test_classify_op_error_not_found ... ok
test keystore::tests::test_classify_op_error_session_expired ... ok
test keystore::tests::test_classify_op_error_unknown ... ok
test keystore::tests::test_extract_bw_field_custom ... ok
test keystore::tests::test_extract_bw_field_custom_not_found ... ok
test keystore::tests::test_extract_bw_field_empty_string ... ok
test keystore::tests::test_extract_bw_field_missing ... ok
test keystore::tests::test_is_apple_password_uri_negative ... ok
test keystore::tests::test_extract_bw_field_username ... ok
test keystore::tests::test_extract_bw_field_password ... ok
test keystore::tests::test_extract_bw_field_notes ... ok
test keystore::tests::test_is_apple_password_uri_positive ... ok
test keystore::tests::test_is_bw_uri_false ... ok
test keystore::tests::test_is_bw_uri_true ... ok
test keystore::tests::test_is_env_uri_negative ... ok
test keystore::tests::test_is_env_uri_positive ... ok
test keystore::tests::test_is_file_uri ... ok
test keystore::tests::test_is_keyring_uri_detects_scheme ... ok
test keystore::tests::test_is_op_uri_negative ... ok
test keystore::tests::test_is_op_uri_positive ... ok
test keystore::tests::test_keyring_uri_rejects_unknown_decode ... ok
test keystore::tests::test_keyring_uri_rejects_path_traversal ... ok
test keystore::tests::test_keyring_uri_roundtrip_no_decode ... ok
test keystore::tests::test_keyring_uri_roundtrip_with_go_keyring_decode ... ok
test keystore::tests::test_load_from_env_empty ... ok
test keystore::tests::test_load_from_env_not_set ... ok
test keystore::tests::test_load_from_bw_cli_not_found ... ok
test keystore::tests::test_load_from_env_set ... ok
test keystore::tests::test_load_from_bw_empty_session ... ok
test keystore::tests::test_load_from_bw_no_session ... ok
test keystore::tests::test_load_from_bws_cli_not_found ... ok
test keystore::tests::test_load_from_bws_no_token ... ok
test keystore::tests::test_load_from_file_empty_file_is_error ... ok
test keystore::tests::test_load_from_file_not_found ... ok
test keystore::tests::test_load_from_file_newline_only_is_error ... ok
test keystore::tests::test_load_from_file_multiline_reads_trimmed ... ok
test keystore::tests::test_load_secret_by_ref_dispatches_apple_passwords ... ok
test keystore::tests::test_load_secret_by_ref_dispatches_bw ... ok
test keystore::tests::test_load_from_file_trims_single_trailing_crlf ... ok
test keystore::tests::test_load_secret_by_ref_dispatches_env ... ok
test keystore::tests::test_redact_apple_password_uri_alias_prefix ... ok
test keystore::tests::test_load_from_file_preserves_significant_whitespace ... ok
test keystore::tests::test_load_from_file_whitespace_only_is_error ... ok
test keystore::tests::test_redact_apple_password_uri_malformed ... ok
test keystore::tests::test_redact_apple_password_uri_valid ... ok
test keystore::tests::test_redact_bw_uri_item ... ok
test keystore::tests::test_load_secret_by_ref_dispatches_op ... ok
test keystore::tests::test_load_from_file_reads_and_trims ... ok
test keystore::tests::test_redact_bw_uri_item_custom ... ok
test keystore::tests::test_redact_bw_uri_not_bw_prefix ... ok
test keystore::tests::test_redact_bw_uri_malformed ... ok
test keystore::tests::test_redact_bw_uri_secret ... ok
test keystore::tests::test_redact_file_uri ... ok
test keystore::tests::test_redact_file_uri_root_path ... ok
test keystore::tests::test_redact_keyring_uri_malformed ... ok
test keystore::tests::test_redact_keyring_uri_normal ... ok
test keystore::tests::test_redact_keyring_uri_with_decode_query ... ok
test keystore::tests::test_redact_op_uri_3_segments ... ok
test keystore::tests::test_redact_op_uri_4_segments ... ok
test keystore::tests::test_redact_op_uri_malformed ... ok
test keystore::tests::test_redact_op_uri_not_op ... ok
test keystore::tests::test_validate_apple_password_uri_empty_segment ... ok
test keystore::tests::test_validate_apple_password_uri_forbidden_char ... ok
test keystore::tests::test_validate_apple_password_uri_missing_account ... ok
test keystore::tests::test_validate_apple_password_uri_missing_prefix ... ok
test keystore::tests::test_validate_apple_password_uri_valid ... ok
test keystore::tests::test_validate_apple_password_uri_valid_alias_prefix ... ok
test keystore::tests::test_validate_bw_uri_forbidden_char ... ok
test keystore::tests::test_validate_bw_uri_fragment_rejected ... ok
test keystore::tests::test_validate_bw_uri_item_custom_field ... ok
test keystore::tests::test_validate_bw_uri_id_with_injection_char ... ok
test keystore::tests::test_validate_bw_uri_item_missing_selector ... ok
test keystore::tests::test_validate_bw_uri_item_notes ... ok
test keystore::tests::test_validate_bw_uri_item_password ... ok
test keystore::tests::test_validate_bw_uri_item_totp ... ok
test keystore::tests::test_validate_bw_uri_item_username ... ok
test keystore::tests::test_validate_bw_uri_query_rejected ... ok
test keystore::tests::test_validate_bw_uri_secret ... ok
test keystore::tests::test_validate_bw_uri_secret_no_field_selector ... ok
test keystore::tests::test_validate_bw_uri_unknown_first_segment ... ok
test keystore::tests::test_validate_bw_uri_unknown_selector ... ok
test keystore::tests::test_validate_destination_env_var_dangerous ... ok
test keystore::tests::test_validate_destination_env_var_dangerous_case_insensitive ... ok
test keystore::tests::test_validate_destination_env_var_empty ... ok
test keystore::tests::test_validate_destination_env_var_invalid_chars ... ok
test keystore::tests::test_validate_destination_env_var_valid ... ok
test keystore::tests::test_validate_env_uri_dangerous_case_insensitive ... ok
test keystore::tests::test_validate_env_uri_dangerous_dyld ... ok
test keystore::tests::test_load_secret_by_ref_dispatches_file_uri ... ok
test keystore::tests::test_validate_env_uri_dangerous_ld_preload ... ok
test keystore::tests::test_validate_env_uri_dangerous_node_options ... ok
test keystore::tests::test_validate_env_uri_dangerous_path ... ok
test keystore::tests::test_validate_env_uri_empty_name ... ok
test keystore::tests::test_validate_env_uri_invalid_chars ... ok
test keystore::tests::test_validate_env_uri_missing_prefix ... ok
test keystore::tests::test_validate_env_uri_valid ... ok
test keystore::tests::test_validate_file_uri_rejects_empty_path ... ok
test keystore::tests::test_validate_file_uri_rejects_forbidden_characters ... ok
test keystore::tests::test_validate_file_uri_rejects_relative_path ... ok
test keystore::tests::test_validate_file_uri_rejects_traversal ... ok
test keystore::tests::test_validate_file_uri_valid_absolute_path ... ok
test keystore::tests::test_validate_file_uri_valid_windows_absolute_path ... ok
test keystore::tests::test_validate_keyring_uri_empty_account ... ok
test keystore::tests::test_validate_keyring_uri_empty_service ... ok
test keystore::tests::test_validate_keyring_uri_forbidden_char ... ok
test keystore::tests::test_validate_keyring_uri_fragment_rejected ... ok
test keystore::tests::test_validate_keyring_uri_missing_account ... ok
test keystore::tests::test_validate_keyring_uri_missing_prefix ... ok
test keystore::tests::test_validate_keyring_uri_query_param_missing_value ... ok
test keystore::tests::test_validate_keyring_uri_too_long ... ok
test keystore::tests::test_validate_keyring_uri_too_many_segments ... ok
test keystore::tests::test_validate_keyring_uri_unknown_query_param ... ok
test keystore::tests::test_validate_keyring_uri_valid ... ok
test keystore::tests::test_validate_op_uri_empty_field ... ok
test keystore::tests::test_validate_op_uri_empty_item ... ok
test keystore::tests::test_validate_op_uri_forbidden_backtick ... ok
test keystore::tests::test_validate_op_uri_forbidden_dollar ... ok
test keystore::tests::test_validate_op_uri_forbidden_newline ... ok
test keystore::tests::test_validate_op_uri_empty_vault ... ok
test keystore::tests::test_validate_op_uri_forbidden_pipe ... ok
test keystore::tests::test_validate_op_uri_forbidden_semicolon ... ok
test keystore::tests::test_validate_op_uri_fragment ... ok
test keystore::tests::test_validate_op_uri_missing_prefix ... ok
test keystore::tests::test_validate_op_uri_query_string ... ok
test keystore::tests::test_validate_op_uri_single_segment ... ok
test keystore::tests::test_validate_op_uri_too_few_segments ... ok
test keystore::tests::test_validate_op_uri_valid_3_segments ... ok
test keystore::tests::test_validate_op_uri_valid_4_segments ... ok
test keystore::tests::test_validate_op_uri_valid_with_spaces_and_dashes ... ok
test machine_policy::expand_tests::empty_tokens_returns_empty ... ok
test machine_policy::expand_tests::group_with_suffixes_only_expands_hosts_not_suffixes ... ok
test machine_policy::expand_tests::known_single_token_returns_hosts ... ok
test machine_policy::expand_tests::malformed_json_returns_policy_load_failed ... ok
test machine_policy::expand_tests::multiple_tokens_returns_union_sorted_deduped ... ok
test machine_policy::expand_tests::unknown_token_returns_empty_not_error ... ok
test machine_policy::tests::cr01_leading_dot_suffix_matches_via_hostfilter ... ok
test machine_policy::tests::empty_policy_deserializes_and_raw_allowlist_is_empty ... ok
test machine_policy::tests::is_unconfigured_false_with_any_entry ... ok
test machine_policy::tests::is_unconfigured_ignores_required_layers_field ... ok
test machine_policy::tests::is_unconfigured_ignores_telemetry_field ... ok
test machine_policy::tests::is_unconfigured_true_for_empty_policy ... ok
test machine_policy::tests::policy_load_failed_display_contains_reason ... ok
test machine_policy::tests::policy_load_failed_is_pattern_matchable ... ok
test machine_policy::tests::policy_load_failed_propagates_via_result_alias ... ok
test machine_policy::tests::non_windows_stub_returns_ok_none ... ok
test machine_policy::tests::policy_serde_round_trip ... ok
test machine_policy::tests::policy_serde_round_trip_with_telemetry ... ok
test machine_policy::tests::policy_serde_without_telemetry_uses_defaults ... ok
test machine_policy::tests::raw_allowlist_concatenates_suffixes_then_hosts ... ok
test machine_policy::tests::telemetry_config_default_has_expected_values ... ok
test machine_policy::tests::raw_allowlist_normalizes_suffix_shapes ... ok
test machine_policy::tests::telemetry_config_invalid_display_contains_reason ... ok
test machine_policy::tests::telemetry_severity_orders_debug_to_error ... ok
test machine_policy::tests::telemetry_unavailable_display_contains_reason ... ok
test machine_policy::validate_tests::empty_host_entry_returns_err ... ok
test machine_policy::validate_tests::empty_policy_returns_ok ... ok
test machine_policy::validate_tests::preset_token_leading_dash_returns_err ... ok
test machine_policy::validate_tests::preset_token_underscore_returns_err ... ok
test machine_policy::validate_tests::preset_token_with_space_returns_err ... ok
test machine_policy::validate_tests::valid_alphanumeric_hyphen_token_is_ok ... ok
test machine_policy::validate_tests::valid_policy_returns_ok ... ok
test machine_policy::validate_tests::whitespace_only_suffix_returns_err ... ok
test net_filter::tests::sc4_dns_component_matrix ... ok
test net_filter::tests::test_allow_all_allows_192_168 ... ok
test net_filter::tests::test_allow_all_allows_private_networks ... ok
test net_filter::tests::test_allow_all_mode ... ok
test machine_policy::tests::windows_wrong_reg_type_returns_policy_load_failed ... ok
test machine_policy::tests::windows_configured_key_is_not_unconfigured ... ok
test net_filter::tests::test_allowed_count ... ok
test net_filter::tests::test_deny_cloud_metadata_hostname ... ok
test machine_policy::tests::windows_sentinel_only_key_is_unconfigured ... ok
test net_filter::tests::test_deny_google_metadata ... ok
test net_filter::tests::test_deny_ipv4_mapped_ipv6_link_local ... ok
test net_filter::tests::test_deny_ipv4_mapped_ipv6_other_link_local ... ok
test machine_policy::tests::windows_list_subkey_reads_reg_sz_values ... ok
test net_filter::tests::test_deny_link_local_ipv4 ... ok
test net_filter::tests::test_deny_link_local_ipv6 ... ok
test net_filter::tests::test_dns_rebinding_allow_all_blocked ... ok
test net_filter::tests::test_dns_rebinding_to_metadata_ip ... ok
test net_filter::tests::test_empty_resolved_ips_skips_link_local_check ... ok
test net_filter::tests::test_exact_host_allowed ... ok
test net_filter::tests::test_exact_host_case_insensitive ... ok
test net_filter::tests::test_filter_result_reason ... ok
test net_filter::tests::test_host_not_in_allowlist ... ok
test net_filter::tests::test_ipv4_mapped_ipv6_non_link_local_allowed ... ok
test net_filter::tests::test_multiple_ips_any_link_local_denied ... ok
test net_filter::tests::test_non_strict_empty_allowlist_allows ... ok
test net_filter::tests::test_strict_filter_empty_allowlist_denies ... ok
test net_filter::tests::test_strict_filter_respects_explicit_allowlist ... ok
test net_filter::tests::test_user_deny_beats_allowlist ... ok
test net_filter::tests::test_user_deny_host_does_not_affect_others ... ok
test net_filter::tests::test_user_deny_case_insensitive ... ok
test net_filter::tests::test_user_deny_host_exact ... ok
test net_filter::tests::test_user_deny_host_wildcard ... ok
test net_filter::tests::test_wildcard_does_not_match_bare_domain ... ok
test net_filter::tests::test_wildcard_subdomain_match ... ok
test query::tests::test_query_network ... ok
test path::tests::existing_path_canonicalizes ... ok
test path::tests::nonexistent_nested_uses_deepest_ancestor ... ok
test path::tests::nonexistent_leaf_uses_ancestor ... ok
test path::tests::existing_symlink_resolves_through ... ok
test sandbox::windows::app_container_tests::create_app_container_profile_empty_name_fails_closed ... ok
test query::tests::test_query_path_granted ... ok
test sandbox::windows::app_container_tests::derive_app_container_sid_rejects_empty_name ... ok
test sandbox::windows::create_low_integrity_primary_token_tests::create_low_integrity_primary_token_returns_low_il_token ... ok
test sandbox::windows::create_low_integrity_primary_token_tests::owned_handle_drop_is_safe_for_low_il_token ... ok
test sandbox::windows::create_low_integrity_primary_token_tests::owned_handle_drop_on_null_is_noop ... ok
test sandbox::windows::app_container_tests::derive_app_container_sid_is_deterministic_for_same_name ... ok
test sandbox::windows::app_container_tests::derive_app_container_sid_yields_package_sid_string ... ok
test sandbox::windows::dacl_grant_tests::grant_read_attributes_invalid_sid_fails_closed ... ok
test sandbox::windows::dacl_grant_tests::grant_read_invalid_sid_fails_closed ... ok
test sandbox::windows::dacl_grant_tests::grant_read_then_revoke_sid_round_trips_on_tempfile ... ok
test keystore::tests::keyring_timeout_tests::call_with_keyring_timeout_slow_call_fires ... ok
test sandbox::windows::dacl_grant_tests::grant_read_attributes_revoke_preserves_original_dacl ... ok
test sandbox::windows::dacl_grant_tests::grant_read_attributes_then_revoke_sid_round_trips_on_tempdir ... ok
test sandbox::windows::dacl_grant_tests::grant_single_file_and_invalid_sid_fails_closed ... ok
test sandbox::windows::dacl_grant_tests::grant_traverse_invalid_sid_fails_closed ... ok
test sandbox::windows::dacl_grant_tests::grant_then_revoke_sid_round_trips_on_tempdir ... ok
test sandbox::windows::tests::apply_accepts_network_blocked_capability_set ... ok
test sandbox::windows::dacl_grant_tests::grant_traverse_then_revoke_round_trips_on_tempdir ... ok
test sandbox::windows::tests::apply_accepts_port_level_wfp_caps ... ok
test sandbox::windows::tests::apply_accepts_minimal_supported_windows_subset ... ok
test sandbox::windows::tests::apply_error_message_remains_explicit_for_unsupported_subset ... ok
test sandbox::windows::tests::apply_accepts_single_file_grant_and_labels_low_integrity ... ok
test sandbox::windows::tests::apply_rejects_capability_expansion_shape ... ok
test sandbox::windows::tests::apply_labels_single_file_read_write_mode_with_correct_mask ... ok
test sandbox::windows::tests::apply_rejects_non_default_ipc_mode ... ok
test sandbox::windows::tests::apply_accepts_write_only_directory_grant_and_labels_low_integrity ... ok
test sandbox::windows::tests::classify_supervisor_support_tracks_supported_and_unsupported_features ... ok
test sandbox::windows::tests::compile_filesystem_policy_classifies_single_file_as_rule ... ok
test sandbox::windows::tests::apply_labels_single_file_write_mode_with_correct_mask ... ok
test sandbox::windows::tests::apply_labels_multiple_single_file_grants_all_succeed ... ok
test sandbox::windows::tests::compile_filesystem_policy_emits_rule_for_write_only_directory_grant ... ok
test sandbox::windows::tests::compile_filesystem_policy_emits_rule_for_single_file_read_grant ... ok
test sandbox::windows::tests::compile_filesystem_policy_emits_rule_for_single_file_write_grant ... ok
test sandbox::windows::tests::compile_filesystem_policy_emits_rule_for_single_file_read_write_grant ... ok
test sandbox::windows::tests::compile_filesystem_policy_classifies_write_only_directory_as_rule ... ok
test sandbox::windows::tests::compile_network_policy_allow_all_has_no_unsupported_shapes ... ok
test sandbox::windows::tests::compile_filesystem_policy_keeps_directory_rules ... ok
test sandbox::windows::tests::compile_network_policy_carries_port_ranges_into_wfp_policy ... ok
test sandbox::windows::tests::compile_network_policy_carries_port_filters_into_wfp_policy ... ok
test sandbox::windows::tests::compile_network_policy_merges_overlapping_port_ranges ... ok
test sandbox::windows::tests::compile_network_policy_tracks_blocked_mode ... ok
test sandbox::windows::tests::compile_network_policy_with_bind_ports_only_is_fully_supported ... ok
test sandbox::windows::tests::compile_network_policy_with_localhost_ports_only_is_fully_supported ... ok
test sandbox::windows::tests::filesystem_policy_covers_directory_path_case_insensitively ... ok
test sandbox::windows::tests::compile_network_policy_with_connect_ports_only_is_fully_supported ... ok
test sandbox::windows::tests::filesystem_policy_covers_directory_path_with_verbatim_prefix_case_insensitively ... ok
test sandbox::windows::tests::filesystem_policy_covers_single_file_with_verbatim_prefix ... ok
test sandbox::windows::tests::label_mask_for_access_mode_read_denies_write_and_execute_up ... ok
test sandbox::windows::tests::label_mask_for_access_mode_read_write_denies_only_execute_up ... ok
test sandbox::windows::tests::label_mask_for_access_mode_write_denies_read_and_execute_up ... ok
test sandbox::windows::tests::compile_filesystem_policy_accepts_git_config_shape ... ok
test sandbox::windows::tests::low_integrity_compatible_dir_matches_localappdata_temp_low ... ok
test sandbox::windows::tests::low_integrity_compatible_dir_matches_locallow ... ok
test sandbox::windows::tests::is_low_integrity_compatible_dir_rejects_an_inherit_only_label ... ok
test sandbox::windows::tests::low_integrity_compatible_dir_rejects_normal_tempdir ... ok
test sandbox::windows::tests::network_launch_support_allows_shell_hosts_for_wfp_backed_mode ... ok
test sandbox::windows::tests::network_launch_support_allows_standalone_binary_for_blocked_mode ... ok
test sandbox::windows::tests::low_integrity_label_ace_reports_zero_flags_for_a_freshly_applied_label ... ok
test sandbox::windows::tests::normalize_windows_path_strips_unc_verbatim_prefix ... ok
test sandbox::windows::tests::normalize_windows_path_strips_verbatim_prefix ... ok
test sandbox::windows::tests::path_is_owned_by_current_user_returns_false_for_system_windows_dir ... ok
test sandbox::windows::tests::path_has_write_owner_returns_true_for_userprofile_tempdir ... ok
test sandbox::windows::tests::preview_runtime_status_allows_blocked_network_mode ... ok
test sandbox::windows::tests::preview_runtime_status_allows_advisory_only_cwd_read_baseline ... ok
test sandbox::windows::tests::path_is_owned_by_current_user_returns_true_for_tempfile ... ok
test sandbox::windows::tests::preview_runtime_status_allows_advisory_only_for_write_only_directory ... ok
test sandbox::windows::tests::preview_runtime_status_allows_advisory_only_for_single_file_policy ... ok
test sandbox::windows::tests::preview_runtime_status_allows_deny_override_when_remaining_policy_is_supported ... ok
test sandbox::windows::tests::preview_runtime_status_blocks_extra_filesystem_grants ... ok
test sandbox::windows::tests::support_info_reports_supported_status_for_promoted_subset_contract ... ok
test sandbox::windows::tests::runtime_state_dir_prefers_writable_current_dir_inside_policy ... ok
test sandbox::windows::tests::preview_runtime_status_allows_supported_directory_allowlist ... ok
test sandbox::windows::tests::single_file_grant_does_not_label_parent_directory ... ok
test sandbox::windows::tests::runtime_state_dir_returns_none_for_file_only_policy ... ok
test sandbox::windows::tests::try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails ... ok
test sandbox::windows::tests::validate_command_args_accepts_absolute_path_inside_policy ... ok
test sandbox::windows::tests::validate_command_args_accepts_find_inside_policy ... ok
test sandbox::windows::tests::validate_command_args_accepts_cscript_paths_inside_policy ... ok
test sandbox::windows::tests::validate_command_args_accepts_findstr_inside_policy ... ok
test sandbox::windows::tests::validate_command_args_accepts_powershell_get_content_inside_policy ... ok
test sandbox::windows::tests::validate_command_args_rejects_absolute_path_outside_policy ... ok
test sandbox::windows::tests::validate_command_args_accepts_relative_cmd_copy_paths_inside_policy ... ok
test sandbox::windows::tests::validate_command_args_accepts_xcopy_inside_policy ... ok
test sandbox::windows::tests::validate_command_args_accepts_relative_cmd_type_path_inside_policy ... ok
test sandbox::windows::tests::validate_command_args_rejects_fc_file_outside_policy ... ok
test sandbox::windows::tests::validate_command_args_rejects_cmd_copy_destination_outside_policy ... ok
test sandbox::windows::tests::validate_command_args_rejects_comp_file_outside_policy ... ok
test sandbox::windows::tests::validate_command_args_rejects_cscript_destination_outside_policy ... ok
test sandbox::windows::tests::validate_command_args_rejects_relative_cmd_type_path_outside_policy ... ok
test sandbox::windows::tests::validate_command_args_rejects_powershell_copy_destination_outside_policy ... ok
test sandbox::windows::tests::validate_command_args_rejects_robocopy_destination_outside_policy ... ok
test sandbox::windows::tests::validate_command_args_rejects_relative_parent_escape_outside_policy ... ok
test sandbox::windows::tests::validate_launch_paths_accepts_single_file_policy_shapes ... ok
test sandbox::windows::tests::validate_launch_paths_accepts_covered_program_and_interpreter ... ok
test sandbox::windows::tests::low_integrity_compatible_dir_matches_dynamically_labeled_directory ... ok
test sandbox::windows::tests::validate_command_args_rejects_symlink_escape_inside_policy ... ok
test sandbox::windows::tests::validate_launch_paths_accepts_supported_executable ... ok
test sandbox::windows::tests::validate_launch_paths_empty_interpreter_slice_is_unchanged ... ok
test sandbox::windows::tests::validate_preview_entry_point_allows_wrap_with_empty_caps ... ok
test sandbox::windows::tests::validate_preview_entry_point_shell_fails_below_min_build ... ok
test sandbox::windows::tests::windows_paths_start_with_case_insensitive_matches_drive_case ... ok
test sandbox::windows::tests::validate_launch_paths_refuses_uncovered_interpreter ... ok
test scrub::tests::scrub_argv_redacts_sensitive_flag_pairs_and_equals_forms ... ok
test scrub::tests::scrub_argv_redacts_sensitive_header_arguments ... ok
test scrub::tests::scrub_header_redacts_sensitive_header_values ... ok
test scrub::tests::scrub_policy_can_remove_defaults_for_unsafe_debugging ... ok
test scrub::tests::scrub_policy_adds_local_sensitive_names ... ok
test scrub::tests::scrub_value_leaves_safe_strings_unchanged ... ok
test scrub::tests::scrub_value_redacts_sensitive_query_parameters ... ok
test scrub::tests::scrub_value_redacts_url_userinfo ... ok
test state::tests::test_state_roundtrip ... ok
test state::tests::test_to_caps_rejects_invalid_access_mode ... ok
test state::tests::test_to_caps_rejects_nonexistent_path ... ok
test sandbox::windows::tests::validate_launch_paths_rejects_executable_outside_policy ... ok
test state::tests::test_unix_socket_state_rejects_invalid_mode ... ok
test supervisor::aipc_sdk::tests::unsupported_platform_message_is_d09_locked_string ... ok
test supervisor::aipc_sdk::tests::unsupported_platform_message_starts_with_aipc_brokering ... ok
test state::tests::test_unix_socket_state_legacy_is_directory_maps_to_dir_children ... ok
test state::tests::test_unix_socket_state_roundtrip_preserves_original_and_resolved ... ok
test supervisor::aipc_sdk::tests::windows_real_broker_smoke_tests::ensure_winsock_initialized_is_idempotent ... ok
test supervisor::aipc_sdk::tests::windows_loopback_tests::helper_stamps_session_token_from_env ... ok
test supervisor::aipc_sdk::tests::windows_loopback_tests::request_pipe_returns_handle_on_granted ... ok
test supervisor::aipc_sdk::tests::windows_loopback_tests::request_event_returns_handle_on_granted ... ok
test supervisor::aipc_sdk::tests::windows_loopback_tests::request_event_propagates_denied_reason ... ok
test supervisor::policy::tests::all_bits_request_always_denied_unless_resolved_is_all_bits ... ok
test supervisor::aipc_sdk::tests::windows_real_broker_smoke_tests::sdk_request_event_round_trips_through_real_broker ... ok
test supervisor::policy::tests::default_masks_match_d05_lock ... ok
test supervisor::policy::tests::empty_request_is_trivially_subset ... ok
test supervisor::policy::tests::mask_subset_validates_correctly ... ok
test supervisor::policy::tests::mutex_modify_state_documented_as_reserved ... ok
test supervisor::policy::tests::standard_constants_match_microsoft_table ... ok
test supervisor::socket::tests::authenticate_pipe_client_reverts_on_error ... ok
test supervisor::socket::tests::build_capability_pipe_sddl_none_matches_constant ... ok
test supervisor::socket::tests::build_capability_pipe_sddl_package_sid_only_embeds_ace ... ok
test supervisor::socket::tests::build_capability_pipe_sddl_rejects_malformed_package_sid ... ok
test supervisor::socket::tests::build_capability_pipe_sddl_session_and_package_sid_embeds_all_aces ... ok
test supervisor::socket::tests::build_capability_pipe_sddl_some_embeds_session_and_logon_aces_before_sacl ... ok
test supervisor::socket::tests::impersonation_guard_reverts_on_drop ... ok
test supervisor::socket::tests::supervisor_message_no_tenant_id_field ... ok
test supervisor::socket::tests::test_bind_aipc_pipe_creates_pipe_with_low_integrity_sddl ... ok
test supervisor::aipc_sdk::tests::windows_real_broker_smoke_tests::sdk_request_pipe_round_trips_through_real_broker ... ok
test supervisor::socket::tests::test_broker_event_to_process_duplicates_handle ... ok
test supervisor::socket::tests::test_broker_event_to_process_propagates_duplicate_handle_failure ... ok
test supervisor::aipc_sdk::tests::windows_real_broker_smoke_tests::sdk_request_socket_round_trips_through_real_broker ... ok
test supervisor::socket::tests::test_broker_job_object_to_process_duplicates_with_query_mask ... ok
test supervisor::aipc_sdk::tests::windows_real_broker_smoke_tests::sdk_request_job_object_round_trips_through_real_broker ... ok
test supervisor::socket::tests::test_broker_job_object_to_process_propagates_duplicate_handle_failure ... ok
test supervisor::socket::tests::test_broker_mutex_to_process_duplicates_handle ... ok
test supervisor::socket::tests::test_broker_mutex_to_process_propagates_duplicate_handle_failure ... ok
test supervisor::socket::tests::test_broker_pipe_to_process_duplicates_with_read_access ... ok
test supervisor::socket::tests::test_broker_pipe_to_process_duplicates_with_readwrite_access ... ok
test supervisor::socket::tests::test_broker_pipe_to_process_propagates_duplicate_handle_failure ... ok
test supervisor::socket::tests::test_broker_socket_to_process_propagates_wsa_failure ... ok
test supervisor::socket::tests::test_broker_socket_to_process_serializes_proto_info_blob ... ok
test supervisor::socket::tests::test_connect_missing_pipe_returns_actionable_diagnostic ... ok
test supervisor::socket::tests::test_explicit_pipe_path_skips_server_pid_expectation ... ok
test supervisor::socket::tests::test_explicit_pipe_paths_are_preserved ... ok
test supervisor::socket::tests::test_message_too_large ... ok
test supervisor::socket::tests::test_pipe_pair_roundtrip ... ok
test supervisor::socket::tests::test_rendezvous_paths_publish_nonce_backed_pipe_names ... ok
test supervisor::aipc_sdk::tests::windows_real_broker_smoke_tests::sdk_request_mutex_round_trips_through_real_broker ... ok
test supervisor::socket::tests::validate_package_sid_for_sddl_accepts_valid_and_rejects_injection ... ok
test supervisor::socket::tests::validate_session_sid_for_sddl_rejects_injection ... ok
test supervisor::socket::tests::test_broker_file_handle_to_process_duplicates_handle ... ok
test supervisor::tests::test_approval_decision_methods ... ok
test supervisor::tests::test_capability_request_json_round_trip_preserves_session_token ... ok
test supervisor::tests::test_deny_backend ... ok
test supervisor::tests::test_grant_backend ... ok
test supervisor::tests::test_resource_grant_helpers ... ok
test supervisor::types::tests::handle_kind_discriminator_bytes_stable ... ok
test supervisor::types::tests::capability_request_json_round_trip_with_target ... ok
test supervisor::types::tests::phase11_request_deserializes_as_file_kind ... ok
test supervisor::types::tests::resource_grant_event_constructor_shape ... ok
test supervisor::types::tests::resource_grant_mutex_constructor_shape ... ok
test supervisor::types::tests::resource_grant_pipe_constructor_shape_read ... ok
test supervisor::types::tests::resource_grant_pipe_constructor_shape_readwrite ... ok
test supervisor::types::tests::resource_grant_socket_protocol_info_blob_shape ... ok
test supervisor::types::tests::resource_grant_socket_blob_json_round_trip_preserves_bytes ... ok
test supervisor::types::tests::socket_protocol_info_blob_variant_round_trips ... ok
test trust::base64::tests::base64_decode_skips_whitespace ... ok
test trust::base64::tests::base64_decode_without_padding ... ok
test trust::base64::tests::base64_empty ... ok
test trust::base64::tests::base64_has_padding ... ok
test trust::base64::tests::base64_known_value ... ok
test trust::base64::tests::base64_roundtrip ... ok
test trust::base64::tests::base64url_decode_invalid_char ... ok
test trust::base64::tests::base64url_decode_skips_whitespace ... ok
test trust::base64::tests::base64url_decode_standard_alphabet ... ok
test trust::base64::tests::base64url_decode_with_padding ... ok
test trust::base64::tests::base64url_empty ... ok
test trust::base64::tests::base64url_known_value ... ok
test trust::base64::tests::base64url_no_padding ... ok
test supervisor::socket::tests::capability_pipe_admits_restricted_token_child_with_session_sid ... ok
test trust::base64::tests::base64url_roundtrip ... ok
test trust::base64::tests::base64url_url_safe_chars ... ok
test trust::base64::tests::cross_alphabet_interop ... ok
test trust::bundle::tests::bundle_path_for_absolute_path ... ok
test trust::bundle::tests::bundle_path_for_appends_extension ... ok
test trust::bundle::tests::bundle_path_for_nested_path ... ok
test trust::bundle::tests::current_date_iso_prefix_pins_known_dates ... ok
test trust::bundle::tests::decode_utf8_extension_invalid_utf8 ... ok
test trust::bundle::tests::decode_utf8_extension_raw_bytes ... ok
test trust::bundle::tests::extract_all_subjects_multi ... ok
test trust::bundle::tests::extract_all_subjects_single ... ok
test trust::bundle::tests::extract_identity_empty_cert_chain ... ok
test trust::bundle::tests::extract_identity_from_real_fulcio_cert ... ok
test trust::bundle::tests::extract_identity_public_key_bundle ... ok
test trust::bundle::tests::load_bundle_invalid_json ... ok
test trust::bundle::tests::load_bundle_missing_fields ... ok
test trust::bundle::tests::load_bundle_nonexistent_file ... ok
test trust::bundle::tests::cache_round_trip ... ok
test sandbox::windows::tests::validate_command_args_rejects_junction_escape_inside_policy ... ok
test trust::bundle::tests::load_trusted_root_invalid_json ... ok
test trust::bundle::tests::load_trusted_root_nonexistent_file ... ok
test trust::bundle::tests::expired_cache_fails_closed_with_recovery_hint ... ok
test trust::bundle::tests::multi_subject_bundle_path_in_cwd ... ok
test trust::bundle::tests::missing_cache_fails_closed ... ok
test trust::bundle::tests::multi_subject_bundle_path_in_dir ... ok
test trust::bundle::tests::normalize_github_uri_non_github ... ok
test trust::bundle::tests::normalize_github_uri_passthrough_v1 ... ok
test trust::bundle::tests::normalize_github_uri_strips_prefix ... ok
test trust::bundle::tests::normalize_workflow_uri_full_v2 ... ok
test trust::bundle::tests::normalize_workflow_uri_no_ref_suffix ... ok
test trust::bundle::tests::normalize_workflow_uri_relative_passthrough ... ok
test trust::bundle::tests::verification_policy_default_enables_sct_verification ... ok
test trust::bundle::tests::real_fulcio_cert_matches_trust_policy ... ok
test trust::digest::tests::bytes_digest_deterministic ... ok
test trust::digest::tests::bytes_digest_different_inputs ... ok
test trust::digest::tests::bytes_digest_empty ... ok
test trust::digest::tests::bytes_digest_hello_world ... ok
test trust::digest::tests::bytes_digest_length ... ok
test trust::digest::tests::file_digest_empty_file ... ok
test trust::bundle::tests::load_production_trusted_root_succeeds ... ok
test trust::bundle::tests::load_test_trusted_root_smoke ... ok
test trust::bundle::tests::verify_bundle_with_invalid_digest ... ok
test trust::digest::tests::file_digest_nonexistent ... ok
test trust::digest::tests::hex_encode_correctness ... ok
test supervisor::socket::tests::test_read_pipe_rendezvous_rejects_missing_server_pid ... ok
test trust::dsse::tests::envelope_decode_payload ... ok
test supervisor::socket::tests::test_rendezvous_roundtrip_uses_published_pipe_name ... ok
test sandbox::windows::app_container_tests::create_app_container_profile_round_trips ... ok
test trust::digest::tests::file_digest_large_file ... ok
test trust::dsse::tests::envelope_extract_statement ... ok
test trust::dsse::tests::envelope_extract_statement_wrong_type ... ok
test trust::dsse::tests::envelope_pae_bytes ... ok
test trust::dsse::tests::envelope_parse_empty_payload ... ok
test trust::dsse::tests::envelope_parse_invalid_json ... ok
test trust::dsse::tests::envelope_parse_valid ... ok
test trust::dsse::tests::envelope_parse_no_signatures ... ok
test trust::digest::tests::file_digest_matches_bytes_digest ... ok
test trust::dsse::tests::envelope_to_json_roundtrip ... ok
test trust::dsse::tests::extract_ref_fallback ... ok
test trust::dsse::tests::extract_ref_heads ... ok
test trust::dsse::tests::instruction_and_policy_predicate_types_differ ... ok
test trust::dsse::tests::extract_ref_standard_format ... ok
test trust::dsse::tests::multi_subject_predicate_type_is_unique ... ok
test supervisor::socket::tests::test_bind_low_integrity_roundtrip ... ok
test trust::dsse::tests::multi_subject_statement_preserves_signer_predicate ... ok
test trust::dsse::tests::multi_subject_statement_roundtrips_through_envelope ... ok
test trust::dsse::tests::multi_subject_statement_single_subject ... ok
test trust::dsse::tests::multi_subject_statement_structure ... ok
test trust::dsse::tests::new_envelope_creates_valid_structure ... ok
test trust::dsse::tests::new_instruction_statement_structure ... ok
test trust::dsse::tests::new_policy_statement_uses_policy_predicate_type ... ok
test trust::dsse::tests::new_statement_accepts_custom_predicate_type ... ok
test trust::dsse::tests::pae_binary_payload ... ok
test trust::dsse::tests::pae_empty_payload ... ok
test trust::dsse::tests::pae_in_toto_type ... ok
test trust::dsse::tests::pae_spec_test_vector ... ok
test trust::dsse::tests::statement_empty_subjects ... ok
test trust::dsse::tests::statement_extract_keyed_signer ... ok
test trust::dsse::tests::statement_extract_keyless_signer ... ok
test trust::dsse::tests::signature_decode ... ok
test trust::dsse::tests::statement_extract_signer_missing ... ok
test trust::dsse::tests::statement_extract_signer_unknown_kind ... ok
test trust::dsse::tests::statement_first_subject_accessors ... ok
test trust::dsse::tests::statement_parse_valid ... ok
test trust::dsse::tests::statement_wrong_type ... ok
test trust::policy::tests::dropped_publishers_reports_the_discarded_set ... ok
test trust::policy::tests::evaluate_blocked_file ... ok
test trust::dsse::tests::statement_subject_missing_digest ... ok
test trust::policy::tests::evaluate_blocked_publisher ... ok
test trust::policy::tests::evaluate_blocked_publisher_by_repository ... ok
test trust::policy::tests::evaluate_blocklist_checked_before_signer ... ok
test trust::policy::tests::evaluate_result_contains_path_and_digest ... ok
test trust::policy::tests::evaluate_trusted_keyed ... ok
test trust::policy::tests::evaluate_trusted_keyless ... ok
test trust::policy::tests::evaluate_unsigned_file ... ok
test trust::policy::tests::evaluate_untrusted_publisher ... ok
test trust::policy::tests::find_included_files_empty_dir ... ok
test trust::policy::tests::find_included_files_in_claude_subdir ... ok
test trust::policy::tests::load_policy_from_file_not_found ... ok
test trust::policy::tests::find_included_files_in_directory ... ok
test trust::policy::tests::find_included_files_respects_extra_skip_dirs ... ok
test trust::policy::tests::find_included_files_in_non_special_hidden_dir ... ok
test trust::policy::tests::find_included_files_skips_bundle_sidecars ... ok
test trust::policy::tests::find_included_files_skips_git_dir ... ok
test trust::policy::tests::find_included_files_skips_well_known_heavy_dirs ... ok
test trust::policy::tests::load_policy_invalid_json ... ok
test trust::policy::tests::load_policy_missing_field ... ok
test trust::policy::tests::load_policy_with_publishers ... ok
test trust::policy::tests::load_valid_policy ... ok
test trust::policy::tests::merge_deduplicates_blocklist_by_digest ... ok
test trust::policy::tests::merge_deduplicates_patterns ... ok
test trust::policy::tests::merge_deduplicates_publishers_by_name ... ok
test trust::policy::tests::merge_empty_errors ... ok
test trust::policy::tests::merge_ignores_legacy_version_field ... ok
test trust::policy::tests::merge_policies_still_treats_every_input_as_a_trust_anchor ... ok
test trust::policy::tests::merge_project_cannot_weaken ... ok
test trust::policy::tests::merge_single_policy_unchanged ... ok
test trust::policy::tests::merge_strictest_enforcement_wins ... ok
test trust::policy::tests::merge_unions_blocklist_digests ... ok
test trust::policy::tests::merge_unions_includes ... ok
test trust::policy::tests::merge_unions_publishers ... ok
test trust::policy::tests::narrowing_only_layer_cannot_add_a_publisher ... ok
test trust::policy::tests::narrowing_only_layer_cannot_smuggle_an_inline_public_key ... ok
test trust::policy::tests::narrowing_only_layer_still_narrows ... ok
test trust::signing::tests::export_public_key_produces_valid_spki ... ok
test trust::signing::tests::export_public_key_to_pem ... ok
test trust::signing::tests::generate_signing_key_produces_valid_keypair ... ok
test trust::signing::tests::key_id_hex_differs_between_keys ... ok
test trust::signing::tests::key_id_hex_is_deterministic ... ok
test trust::signing::tests::sign_bytes_bundle_contains_correct_digest ... ok
test trust::signing::tests::sign_bytes_bundle_roundtrips_through_sigstore_bundle ... ok
test trust::signing::tests::sign_bytes_produces_valid_bundle_json ... ok
test trust::signing::tests::sign_bytes_signature_verifies ... ok
test trust::signing::tests::sign_files_produces_valid_multi_subject_bundle ... ok
test trust::signing::tests::sign_files_accepts_max_files ... ok
test trust::signing::tests::sign_files_roundtrips_through_sigstore_bundle ... ok
test trust::signing::tests::sign_instruction_file_nonexistent_returns_error ... ok
test trust::signing::tests::sign_files_signature_verifies ... ok
test trust::policy::tests::load_policy_from_file_success ... ok
test trust::signing::tests::sign_policy_bytes_differs_from_instruction_bytes ... ok
test trust::signing::tests::sign_policy_bytes_signature_verifies ... ok
test trust::signing::tests::sign_policy_bytes_uses_policy_predicate_type ... ok
test trust::signing::tests::sign_files_rejects_too_many_files ... ok
test trust::signing::tests::sign_policy_file_nonexistent_returns_error ... ok
test trust::signing::tests::test_github_id_token_happy_path ... ok
test trust::signing::tests::test_github_id_token_rejects_prefix_attack ... ok
test trust::signing::tests::test_gitlab_id_token_happy_path ... ok
test trust::signing::tests::test_gitlab_id_token_rejects_malformed_token ... ok
test trust::signing::tests::test_gitlab_id_token_rejects_port_mismatch ... ok
test trust::signing::tests::test_gitlab_id_token_rejects_prefix_matched_issuer ... ok
test trust::signing::tests::test_gitlab_id_token_rejects_scheme_mismatch ... ok
test trust::signing::tests::test_gitlab_id_token_rejects_wrong_issuer ... ok
test trust::signing::tests::test_gitlab_id_token_self_managed_happy_path ... ok
test trust::types::tests::blocklist_check_hit ... ok
test trust::types::tests::blocklist_check_miss ... ok
test trust::types::tests::enforcement_is_blocking ... ok
test trust::types::tests::enforcement_ordering ... ok
test trust::types::tests::includes_invalid_glob ... ok
test trust::types::tests::legacy_version_field_is_accepted_and_ignored ... ok
test trust::types::tests::publisher_is_keyed_and_keyless ... ok
test trust::types::tests::includes_match ... ok
test trust::types::tests::publisher_keyed_vs_keyless_no_cross_match ... ok
test trust::types::tests::includes_returns_originals ... ok
test trust::types::tests::publisher_matches_keyed ... ok
test trust::types::tests::publisher_matches_keyless ... ok
test trust::signing::tests::sign_instruction_file_works ... ok
test trust::types::tests::publisher_no_match_wrong_repo ... ok
test trust::types::tests::publisher_no_match_wrong_ref ... ok
test trust::signing::tests::sign_policy_file_works ... ok
test trust::types::tests::publisher_rejects_empty_identity_fields ... ok
test trust::types::tests::validate_version_accepts_policy ... ok
test trust::types::tests::verification_outcome_blocked_always_blocks ... ok
test trust::types::tests::trust_policy_serde_roundtrip ... ok
test trust::types::tests::verification_outcome_digest_mismatch ... ok
test trust::types::tests::verification_outcome_unsigned_respects_enforcement ... ok
test trust::types::tests::verification_outcome_untrusted_respects_enforcement ... ok
test trust::types::tests::verification_outcome_verified ... ok
test trust::types::tests::verification_result_serde_roundtrip ... ok
test trust::types::tests::wildcard_match_all ... ok
test trust::types::tests::wildcard_match_exact ... ok
test trust::types::tests::wildcard_match_interior ... ok
test trust::types::tests::wildcard_match_multiple ... ok
test trust::types::tests::wildcard_match_prefix ... ok
test trust::types::tests::wildcard_match_suffix ... ok
test trust::types::tests::wildcard_publisher_matching ... ok
test undo::exclusion::tests::force_include_matches_path_component ... ok
test undo::exclusion::tests::empty_patterns_excludes_nothing ... ok
test undo::exclusion::tests::component_pattern_matches ... ok
test undo::exclusion::tests::force_include_overrides_patterns ... ok
test undo::exclusion::tests::force_include_rejects_substring_match ... ok
test undo::exclusion::tests::normal_files_not_excluded ... ok
test undo::exclusion::tests::slash_pattern_matches_as_substring ... ok
test undo::merkle::tests::adding_file_changes_root ... ok
test undo::merkle::tests::empty_tree_has_deterministic_root ... ok
test undo::merkle::tests::deterministic_regardless_of_insertion_order ... ok
test undo::merkle::tests::root_changes_when_file_content_changes ... ok
test undo::merkle::tests::odd_number_of_files ... ok
test undo::merkle::tests::root_changes_when_file_path_changes ... ok
test undo::merkle::tests::single_file_tree ... ok
test undo::exclusion::tests::glob_pattern_matches_filename ... ok
test undo::object_store::tests::has_object_false_for_missing ... ok
test undo::object_store::tests::retrieve_missing_object_errors ... ok
test undo::object_store::tests::deduplication ... ok
test undo::object_store::tests::store_and_retrieve_roundtrip ... ok
test undo::exclusion::tests::gitignore_integration ... ok
test undo::object_store::tests::retrieve_to_atomic ... ok
test undo::object_store::tests::retrieve_to_replaces_existing_target_content ... ok
test undo::snapshot::tests::budget_checked_for_tracked_files ... ok
test undo::snapshot::tests::cleanup_new_atomic_temp_files_removes_only_new_files ... ok
test undo::object_store::tests::clone_or_copy_produces_identical_content ... ok
test undo::snapshot::tests::collect_atomic_temp_prunes_excluded_dirs ... ok
test undo::object_store::tests::verify_integrity ... ok
test undo::snapshot::tests::collect_atomic_temp_respects_budget ... ok
test undo::object_store::tests::store_file_roundtrip ... ok
test undo::object_store::tests::verify_detects_corruption ... ok
test undo::object_store::tests::store_file_streams_without_full_buffer ... ok
test undo::snapshot::tests::compute_merkle_root_changes_after_modification ... ok
test trust::signing::tests::write_bundle_creates_file ... ok
test undo::snapshot::tests::compute_restore_diff_shows_changes_without_modifying_disk ... ok
test undo::snapshot::tests::incremental_detects_creation ... ok
test undo::snapshot::tests::baseline_captures_all_files ... ok
test undo::snapshot::tests::compute_merkle_root_matches_baseline ... ok
test undo::snapshot::tests::incremental_detects_deletion ... ok
test undo::snapshot::tests::load_session_metadata_defaults_network_events_for_legacy_json ... ok
test undo::snapshot::tests::load_manifest_and_changes_static ... ok
test undo::snapshot::tests::load_session_metadata_static ... ok
test undo::snapshot::tests::incremental_detects_modification ... ok
test undo::snapshot::tests::merkle_root_differs_between_snapshots ... ok
test undo::snapshot::tests::session_metadata_save ... ok
test undo::snapshot::tests::manifest_roundtrip_via_disk ... ok
test undo::types::tests::change_type_display ... ok
test undo::types::tests::all_denial_categories_present_and_guard_covers_every_entry ... ok
test undo::types::tests::content_hash_invalid_hex ... ok
test undo::types::tests::content_hash_hex_roundtrip ... ok
test undo::snapshot::tests::validate_manifest_rejects_path_outside_tracked ... ok
test undo::snapshot::tests::restore_reverts_to_baseline ... ok
test undo::types::tests::content_hash_invalid_length ... ok
test undo::snapshot::tests::validate_manifest_rejects_parent_dir_traversal ... ok
test undo::types::tests::content_hash_prefix_suffix ... ok
test undo::types::tests::content_hash_serde_roundtrip ... ok
test undo::types::tests::rollback_status_available_is_available ... ok
test undo::types::tests::rollback_status_display_labels ... ok
test undo::types::tests::rollback_status_serde_roundtrip ... ok
test undo::types::tests::session_metadata_defaults_rollback_status_for_legacy_json ... ok
test undo::types::tests::session_metadata_rollback_status_failed_warning_roundtrip ... ok
test undo::types::tests::session_metadata_rollback_status_skipped_roundtrip ... ok
test undo::types::tests::snapshot_manifest_serde_roundtrip ... ok
test undo::snapshot::tests::walk_budget_entry_limit_exceeded ... ok
test undo::snapshot::tests::walk_budget_byte_limit_exceeded ... ok
test undo::snapshot::tests::validate_manifest_accepts_valid_paths ... ok

test result: ok. 840 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.87s

     Running tests\capability_manifest_schema.rs (target\debug\deps\capability_manifest_schema-fbe94f7d339b382c.exe)

running 40 tests
test filesystem_grant_readwrite_file ... ok
test credential_with_url_path_injection ... ok
test filesystem_grant_read_directory ... ok
test filesystem_deny_paths ... ok
test credential_minimal ... ok
test credential_with_endpoint_rules ... ok
test credential_with_header_injection ... ok
test credential_with_file_source ... ok
test filesystem_grant_with_home_tilde ... ok
test filesystem_grants_and_deny_combined ... ok
test manifest_with_empty_capabilities ... ok
test full_realistic_manifest ... ok
test network_blocked_mode ... ok
test minimal_manifest_with_version_only ... ok
test network_proxy_with_domains ... ok
test network_endpoint_without_rules_allows_all ... ok
test network_with_l7_endpoints ... ok
test process_with_all_fields ... ok
test rejects_credential_invalid_inject_mode ... ok
test rejects_credential_missing_source ... ok
test rejects_credential_missing_name ... ok
test network_with_ports ... ok
test rejects_credential_missing_upstream ... ok
test rejects_filesystem_grant_empty_path ... ok
test rejects_filesystem_grant_invalid_access_mode ... ok
test rejects_filesystem_grant_missing_access ... ok
test rejects_filesystem_grant_missing_path ... ok
test rejects_invalid_exec_strategy ... ok
test rejects_filesystem_grant_invalid_type ... ok
test rejects_invalid_signal_mode ... ok
test rejects_invalid_version_format ... ok
test rejects_missing_version ... ok
test rejects_network_port_above_max ... ok
test rejects_network_invalid_mode ... ok
test rejects_network_port_out_of_range ... ok
test schema_is_valid_json ... ok
test rejects_network_endpoint_missing_host ... ok
test rejects_wrong_version_value ... ok
test schema_compiles_as_json_schema ... ok
test rollback_config ... ok

test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests\manifest_types.rs (target\debug\deps\manifest_types-1075bbfb25d0f727.exe)

running 18 tests
test convert_network_localhost_range ... ok
test convert_minimal_manifest ... ok
test convert_network_ports ... ok
test convert_process_modes ... ok
test convert_nonexistent_path_fails ... ok
test convert_network_blocked ... ok
test convert_network_localhost_range_rejects_start_greater_than_end ... ok
test convert_filesystem_grants ... ok
test credential_inject_header_mode_passes_validation ... ok
test credential_inject_validation_url_path_requires_pattern ... ok
test credential_inject_validation_query_param_requires_name ... ok
test convert_validation_failure_propagates ... ok
test filesystem_grants_deserialize ... ok
test missing_version_rejected ... ok
test minimal_manifest_deserializes ... ok
test round_trip_minimal ... ok
test network_modes_deserialize ... ok
test invalid_version_rejected ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\main.rs (target\debug\deps\nono-50ea74d4aa0985bf.exe)

running 1638 tests
test agent_cli::tests::daemon_start_uses_sc_start_for_type10 ... ok
test agent_cli::tests::daemon_start_uses_raw_spawn_for_no_service ... ok
test agent_cli::tests::daemon_start_uses_raw_spawn_for_type50_template ... ok
test agent_cli::tests::control_pipe_name_consistency ... ok
test agent_cli::tests::is_pipe_not_found_returns_false_for_other_errors ... ok
test agent_cli::tests::is_pipe_not_found_recognizes_gle2 ... ok
test agent_cli::tests::agent_list_parses ... ok
test agent_cli::tests::agent_demote_parses ... ok
test agent_cli::tests::daemon_subcommand_parses_stop ... ok
test agent_cli::tests::agent_launch_parses_profile_and_cmd ... ok
test agent_cli::tests::no_agent_query_verb_exists ... ok
test agent_cli::tests::daemon_subcommand_parses_status ... ok
test agent_cli::tests::daemon_subcommand_parses_start ... ok
test audit_commands::tests::read_capability_decisions_returns_empty_on_missing_ledger ... ok
test audit_commands::tests::sanitize_for_terminal_removes_ansi_escape_sequences ... ok
test audit_attestation::tests::audit_attestation_file_uri_signer_loads ... ok
test audit_commands::tests::sanitize_for_terminal_removes_osc_sequences ... ok
test audit_commands::tests::sanitize_for_terminal_removes_carriage_return ... ok
test audit_commands::tests::read_capability_decisions_returns_filtered_records ... ok
test audit_integrity::tests::recorder_produces_integrity_summary ... ok
test audit_integrity::tests::record_session_started_scrubs_command_secrets ... ok
test audit_integrity::tests::recorder_tracks_event_count_without_needing_integrity_output ... ok
test audit_attestation::tests::audit_attestation_round_trips ... ok
test audit_integrity::tests::verifier_round_trips_all_current_audit_event_payload_variants ... ok
test audit_attestation::tests::audit_attestation_mismatch_is_reported_not_fatal ... ok
test audit_attestation::tests::audit_attestation_predicate_scrubs_command_secrets ... ok
test audit_attestation::tests::audit_attestation_predicate_records_redaction_policy_diff ... ok
test audit_integrity::tests::verifier_rejects_alpha_records_missing_event_json ... ok
test audit_integrity::tests::record_session_started_uses_configured_redaction_policy ... ok
test audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty ... FAILED
test audit_session::tests::discover_sessions_reads_legacy_audit_root ... ok
test capability_ext::tests::test_allow_unix_socket_adds_cap_and_implied_read_fs_grant ... ok
test capability_ext::tests::test_allow_unix_socket_bind_existing_grants_readwrite_fs ... ok
test capability_ext::tests::test_allow_unix_socket_dir_bind_directory_grants_readwrite_fs ... ok
test capability_ext::tests::test_allow_unix_socket_bind_accepts_nonexistent_path_and_widens_fs_to_parent ... ok
test capability_ext::tests::test_allow_unix_socket_dir_implies_read_fs_grant ... ok
test capability_ext::tests::test_allow_unix_socket_missing_skips_both_socket_and_fs_grants ... ok
test capability_ext::tests::test_allow_unix_socket_subtree_bind_implies_readwrite_fs_grant ... ok
test capability_ext::tests::test_allow_unix_socket_subtree_implies_read_fs_grant ... ok
test capability_ext::tests::test_from_args_allow_connect_port_populates_tcp_connect_ports ... ok
test capability_ext::tests::test_from_args_allow_gpu_is_noop_on_windows ... ok
test capability_ext::tests::test_cli_allow_connect_port_overrides_profile_connect_port ... ok
test capability_ext::tests::test_cli_allow_upgrades_profile_read_path ... ok
test capability_ext::tests::test_from_args_allow_proxy_does_not_open_raw_tcp_ports ... ok
test capability_ext::tests::test_cli_write_merges_with_profile_read_path ... ok
test capability_ext::tests::test_from_args_basic ... ok
test capability_ext::tests::test_from_args_network_blocked ... ok
test capability_ext::tests::test_from_args_rejects_protected_state_subtree ... ok
test capability_ext::tests::test_from_args_uses_default_profile_groups_for_runtime_policy ... ok
test capability_ext::tests::test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu ... ok
test capability_ext::tests::test_from_args_without_allow_gpu_never_sets_capability ... ok
test capability_ext::tests::test_from_args_with_commands ... ok
test capability_ext::tests::test_from_profile_allow_domain_does_not_open_raw_tcp_ports ... ok
test capability_ext::tests::test_from_loaded_profile_extends_default_respects_excluded_blocked_commands ... ok
test capability_ext::tests::test_from_profile_allow_net_overrides_proxy_mode ... ok
test capability_ext::tests::test_from_profile_allow_file_rejects_directory_when_exact_dir_unsupported ... ok
test capability_ext::tests::test_from_profile_commands_allow_deny_canonical_section ... ok
test capability_ext::tests::test_from_profile_filesystem_read_accepts_file_paths ... ok
test capability_ext::tests::test_from_profile_policy_add_allow_paths_add_capabilities ... ok
test capability_ext::tests::test_from_profile_allow_net_overrides_blocked_network ... ok
test capability_ext::tests::test_from_profile_ipc_mode_full ... ok
test capability_ext::tests::test_from_profile_policy_exclude_groups_removes_non_required_group ... ok
test capability_ext::tests::test_from_profile_connect_port_populates_tcp_connect_ports ... ok
test capability_ext::tests::test_from_profile_allowed_commands ... ok
test capability_ext::tests::test_from_profile_open_port_range_populates_localhost_port_ranges ... ok
test capability_ext::tests::test_from_profile_ipc_mode_shared_memory_only ... ok
test capability_ext::tests::test_from_profile_ipc_mode_default ... ok
test capability_ext::tests::test_from_profile_policy_override_deny_punches_through_deny_group ... ok
test capability_ext::tests::test_from_profile_with_groups ... ok
test capability_ext::tests::test_from_profile_process_info_mode_same_sandbox ... ok
test capability_ext::tests::test_profile_fs_allow_expands_env_var ... ok
test capability_ext::tests::test_from_profile_policy_override_deny_requires_matching_grant ... ok
test capability_ext::tests::test_profile_unix_socket_field_connect_bind_dir ... ok
test cert_trust::tests::test_cert_trust_api_compiles ... ok
test capability_ext::tests::test_profile_unix_socket_field_connect_bind_file ... ok
test claude_code_hook::tests::byte_array_literal_is_clm_safe_and_byte_faithful ... ok
test claude_code_hook::tests::byte_replace_statement_empty_new_is_a_deletion ... ok
test capability_ext::tests::test_profile_unix_socket_field_connect_bind_subtree ... ok
test claude_code_hook::tests::byte_replace_statement_is_clm_safe_and_embeds_byte_literals ... ok
test claude_code_hook::tests::confined_edit_cmd_is_clm_safe_byte_vehicle ... ok
test claude_code_hook::tests::confined_multiedit_cmd_is_clm_safe_single_read_write ... ok
test claude_code_hook::tests::confined_write_cmd_is_clm_safe ... ok
test claude_code_hook::tests::confined_write_empty_content_emits_empty_byte_array ... ok
test claude_code_hook::tests::non_pre_tool_use_event_is_silent ... ok
test claude_code_hook::tests::pre_tool_use_edit_returns_deny_with_ps_cmd ... ok
test claude_code_hook::tests::pre_tool_use_bash_rewrites_command ... ok
test claude_code_hook::tests::pre_tool_use_file_tools_deny ... ok
test claude_code_hook::tests::pre_tool_use_multiedit_returns_deny_with_ps_cmd ... ok
test claude_code_hook::tests::pre_tool_use_notebookedit_deny_no_ps_cmd ... ok
test capability_ext::tests::test_profile_override_deny_requires_matching_grant ... ok
test capability_ext::tests::test_profile_unix_socket_field_connect_dir ... ok
test claude_code_hook::tests::pre_tool_use_read_only_tools_allow ... ok
test capability_ext::tests::test_profile_unix_socket_field_connect_file ... ok
test claude_code_hook::tests::pre_tool_use_write_content_with_special_chars ... ok
test capability_ext::tests::test_profile_unix_socket_field_connect_subtree ... ok
test claude_code_hook::tests::pre_tool_use_write_returns_deny_with_ps_cmd ... ok
test claude_code_hook::tests::windows_cwd_guard_denies_home_claude_ancestor_absent_file ... ok
test claude_code_hook::tests::windows_cwd_guard_denies_home_claude_ancestor ... ok
test claude_code_hook::tests::windows_cwd_guard_denies_project_claude_child ... ok
test claude_code_hook::tests::windows_cwd_guard_uses_path_components ... ok
test claude_code_hook::tests::windows_wrapper_uses_native_powershell_child ... ok
test claude_code_hook::tests::windows_cwd_guard_denies_inside_home_claude ... ok
test claude_code_hook::tests::windows_write_arm_cwd_guard_fires_before_ps_cmd ... ok
test cli::parser_tests::allow_gpu_windows_warning_helper_is_callable ... ok
test cli::parser_tests::allow_gpu_default_is_false ... ok
test cli::parser_tests::allow_gpu_parses_on_run ... ok
test cli::parser_tests::allow_gpu_parses_on_shell ... ok
test cli::parser_tests::allow_gpu_coexists_with_phase16_and_env_filter_flags ... ok
test cli::parser_tests::allow_gpu_parses_on_wrap ... ok
test cli::parser_tests::env_allow_accepts_exact_and_glob_via_clap ... ok
test cli::parser_tests::env_deny_malformed_pattern_fails_closed_at_parse ... ok
test cli::parser_tests::env_allow_default_is_empty_vec ... ok
test cli::parser_tests::env_allow_malformed_pattern_fails_closed_at_parse ... ok
test cli::parser_tests::env_deny_accepts_exact_and_glob_via_clap ... ok
test cli::parser_tests::env_filter_flags_available_on_shell_and_wrap ... ok
test cli::parser_tests::parse_byte_size_accepts_raw_bytes ... ok
test cli::parser_tests::cpu_percent_range_enforced_by_clap ... ok
test cli::parser_tests::parse_byte_size_accepts_kmgt_suffixes ... ok
test cli::parser_tests::parse_duration_accepts_raw_seconds ... ok
test cli::parser_tests::parse_byte_size_rejects_overflow ... ok
test cli::parser_tests::memory_zero_rejected_by_parser ... ok
test cli::parser_tests::parse_byte_size_rejects_invalid ... ok
test cli::parser_tests::env_filter_flags_do_not_collide_with_phase16_flags ... ok
test cli::parser_tests::parse_duration_accepts_suffixes ... ok
test cli::parser_tests::parse_duration_rejects_invalid ... ok
test cli::parser_tests::parse_env_filter_pattern_accepts_bare_wildcard ... ok
test cli::parser_tests::parse_env_filter_pattern_accepts_prefix_glob ... ok
test cli::parser_tests::parse_env_filter_pattern_accepts_exact_names ... ok
test cli::parser_tests::parse_env_filter_pattern_rejects_middle_wildcard ... ok
test cli::parser_tests::parse_env_filter_pattern_rejects_leading_wildcard ... ok
test cli::parser_tests::parse_env_filter_pattern_rejects_empty ... ok
test cli::profile_resolver_args_tests::override_audit_meta_allows_null_zt_audit_hash ... ok
test cli::profile_resolver_args_tests::override_audit_meta_deserializes_valid_json ... ok
test cli::profile_resolver_args_tests::profile_resolver_args_default_no_auto_pull_is_false ... ok
test cli::parser_tests::wrap_to_sandbox_args_propagates_allow_gpu ... ok
test cli::parser_tests::max_processes_range_enforced_by_clap ... ok
test cli::profile_resolver_args_tests::override_audit_meta_rejects_unknown_fields ... ok
test cli::profile_resolver_args_tests::profile_resolver_args_cli_flag_overrides_env_var ... ok
test cli::profile_resolver_args_tests::setup_grant_ancestors_with_profile_parses ... ok
test cli::profile_resolver_args_tests::profile_resolver_args_cli_flag_sets_true ... ok
test cli::profile_resolver_args_tests::setup_grant_ancestors_without_profile_is_valid ... ok
test cli::profile_resolver_args_tests::setup_profile_without_grant_ancestors_is_rejected ... ok
test cli::profile_resolver_args_tests::profile_resolver_args_env_var_sets_true ... ok
test cli::tests::test_allow_endpoint_flag_parses ... ok
test cli::tests::test_allow_net_conflicts_with_allow_domain ... ok
test cli::tests::config_and_deny_domain_are_rejected_together ... ok
test cert_trust::tests::test_is_cert_present_absent_thumbprint_no_panic ... ok
test cli::profile_resolver_args_tests::profile_resolver_args_wrap_subcommand_accepts_flag ... ok
test cli::tests::test_allow_net_conflicts_with_block_net ... ok
test cli::tests::test_allow_net_conflicts_with_network_profile ... ok
test cli::profile_resolver_args_tests::pull_args_does_not_have_no_auto_pull_field ... ok
test cli::tests::test_allow_net_conflicts_with_deny_domain ... ok
test cli::tests::test_allow_net_parsing ... ok
test cli::tests::test_audit_list ... ok
test cli::tests::test_allow_port_parsing ... ok
test cli::tests::test_capability_elevation_flag_sets_true ... ok
test cli::tests::test_env_credential_map_repeatable_parses_pairs ... ok
test cli::tests::test_log_file_flag ... ok
test cli::tests::test_audit_show ... ok
test cli::tests::test_invalid_subcommand_is_parse_error ... ok
test cli::tests::test_no_audit_integrity_flag_parses ... ok
test cli::tests::test_log_file_flag_absent ... ok
test cli::tests::test_profile_diff_parses ... ok
test cli::tests::test_profile_groups_parses ... ok
test cli::tests::test_override_deny_multiple ... ok
test cli::tests::test_profile_groups_with_name ... ok
test cli::tests::test_override_deny_single ... ok
test cli::tests::test_profile_init_all_flags ... ok
test cli::tests::test_profile_guide ... ok
test cli::tests::test_profile_init_missing_name ... ok
test cli::tests::test_profile_list_parses ... ok
test cli::tests::test_profile_schema_default ... ok
test cli::tests::test_profile_no_subcommand ... ok
test cli::tests::test_profile_init_basic ... ok
test cli::tests::test_profile_show_parses_with_format_manifest ... ok
test cli::tests::test_profile_schema_with_output ... ok
test cli::tests::test_network_flag_aliases_still_parse ... ok
test cli::tests::test_profile_show_parses_with_json_and_raw ... ok
test cli::tests::test_rollback_cleanup_defaults ... ok
test cli::tests::test_profile_validate_parses ... ok
test cli::tests::test_rollback_list ... ok
test cli::tests::test_rollback_cleanup_with_options ... ok
test cli::tests::test_rollback_flags_with_no_rollback ... ok
test cli::tests::test_rollback_all_conflicts_with_include ... ok
test cli::tests::test_rollback_restore_defaults ... ok
test cli::tests::test_rollback_verify ... ok
test cli::tests::test_rollback_show ... ok
test cli::tests::test_rollback_list_recent_json ... ok
test cli::tests::test_rollback_restore_with_options ... ok
test cli::tests::test_root_help_mentions_windows_restricted_execution_surface ... ok
test cli::tests::test_root_help_lists_all_commands ... ok
test cli::tests::test_root_help_shows_all_flags ... ok
test cli::tests::test_root_help_short_circuits_in_parser ... ok
test cli::tests::test_run_basic ... ok
test cli::tests::test_run_multiple_paths ... ok
test cli::tests::test_root_version_short_circuits_in_parser ... ok
test cli::tests::test_run_with_separator ... ok
test cli::tests::test_shell_basic ... ok
test cli::tests::test_trust_init_with_includes ... ok
test cli::tests::test_trust_init_defaults ... ok
test cli::tests::test_trust_export_key_defaults ... ok
test cli::tests::test_trust_keygen ... ok
test cli::tests::test_trust_keygen_with_id ... ok
test cli::tests::test_trust_export_key_with_options ... ok
test cli::tests::test_trust_sign ... ok
test cli::tests::test_trust_list ... ok
test cli::tests::test_trust_sign_all ... ok
test cli::tests::test_trust_sign_multi_subject ... ok
test cli::tests::test_trust_sign_with_key ... ok
test cli::tests::test_trust_override_flag_sets_true ... ok
test cli::tests::test_trust_verify ... ok
test cli::tests::test_unix_socket_subtree_flags_parse ... ok
test cli::tests::test_why_positional_path_resolves ... ok
test cli::tests::test_why_positional_and_named_path_conflict ... ok
test cli::tests::test_subcommand_help_structure ... ok
test cli::tests::test_wrap_help_hides_proxy_flags ... ok
test cli::tests::test_wrap_basic ... ok
test cli::tests::test_wrap_rejects_proxy_flags_at_parse_time ... ok
test command_blocking_deprecation::tests::profile_warnings_include_allowed_and_denied_command_fields ... ok
test cli::tests::test_wrap_supports_direct_network_flags_only ... ok
test command_blocking_deprecation::tests::sandbox_arg_warnings_include_allow_and_block_flags ... ok
test command_blocking_deprecation::tests::test_deprecation_warning_does_not_block_allowed_commands ... ok
test command_blocking_deprecation::tests::test_deprecation_warning_does_not_unblock_commands ... ok
test command_blocking_deprecation::tests::test_deprecation_warning_emitted_for_deprecated_command ... ok
test command_blocking_deprecation::tests::test_print_warnings_silent_is_noop ... ok
test command_blocking_deprecation::tests::warning_for_surface_omits_empty_command_list_suffix ... ok
test command_display::tests::args_with_dollar_and_backslash_are_quoted ... ok
test command_display::tests::args_with_double_quotes_are_quoted ... ok
test cli_bootstrap::tests::log_target_is_private_tests::log_path_outside_every_granted_directory_is_private ... ok
test command_display::tests::args_with_single_quotes_are_quoted ... ok
test command_display::tests::args_with_spaces_are_quoted ... ok
test command_display::tests::empty_command_returns_empty_string ... ok
test command_display::tests::empty_arg_is_quoted ... ok
test command_display::tests::issue_660_repro ... ok
test command_display::tests::plain_args_unquoted ... ok
test command_blocking_deprecation::tests::manifest_warnings_include_process_command_fields ... ok
test command_display::tests::truncate_chars_max_len_smaller_than_ellipsis ... ok
test command_display::tests::truncate_chars_no_spurious_truncation_of_multibyte_string ... ok
test command_display::tests::truncate_chars_handles_multibyte_utf8_at_byte_boundary ... ok
test command_display::tests::truncate_chars_short_passes_through ... ok
test cli_bootstrap::tests::log_target_is_private_tests::log_path_inside_a_granted_directory_is_not_private ... ok
test cli_bootstrap::tests::log_target_is_private_tests::no_log_path_recorded_is_not_private ... ok
test command_display::tests::truncate_command_handles_multibyte_utf8 ... ok
test command_display::tests::truncate_command_max_len_smaller_than_ellipsis ... ok
test command_display::tests::truncate_command_short_passes_through ... ok
test cli_bootstrap::tests::log_target_is_private_tests::perturbed_granted_path_no_longer_covering_log_path_flips_result_to_private ... ok
test completions::tests::test_completions_args_parse_all_shells ... ok
test completions::tests::test_bash_completions_contain_subcommands ... ok
test cli_bootstrap::tests::shared_file_make_writer_appends_output ... ok
test completions::tests::test_bash_completions_contain_binary_name ... ok
test config::embedded::tests::test_load_embedded_network_policy ... ok
test completions::tests::test_completion_write_errors_are_returned ... ok
test config::embedded::tests::test_load_embedded_policy ... ok
test completions::tests::test_fish_completions_contain_binary_name ... ok
test config::tests::test_check_blocked_command_basic ... ok
test config::tests::test_check_blocked_command_extra_blocked ... ok
test config::tests::test_check_blocked_command_only_uses_resolved_policy ... ok
test config::tests::test_check_blocked_command_with_override ... ok
test config::tests::test_check_blocked_command_with_path ... ok
test config::tests::nono_home_dir_rejects_non_absolute_override ... FAILED
test config::tests::nono_home_dir_returns_override_when_set ... FAILED
test config::tests::nono_home_dir_falls_through_when_unset ... FAILED
test completions::tests::test_powershell_completions_non_empty ... ok
test config::user::tests::test_empty_user_config ... ok
test config::user::tests::test_redaction_settings_add_extra_patterns ... ok
test config::user::tests::test_parse_user_config ... ok
test config::user::tests::test_redaction_settings_can_remove_defaults_when_unsafe_enabled ... ok
test config::user::tests::test_redaction_settings_require_unsafe_override_for_removals ... ok
test config::user::tests::test_rollback_settings_custom ... ok
test config::user::tests::test_rollback_settings_defaults ... ok
test config::user::tests::test_ui_detach_sequence_parses_control_prefix ... ok
test config::user::tests::test_ui_detach_sequence_rejects_single_key ... ok
test config::version::tests::test_version_check_downgrade ... ok
test config::version::tests::test_update_version ... ok
test config::version::tests::test_version_check_new ... ok
test config::version::tests::test_version_check_upgrade ... ok
test deprecated_schema::tests::deprecation_counter_emits_separately_per_key ... ok
test deprecated_schema::tests::deprecation_counter_emits_once_per_key ... ok
test deprecated_schema::tests::legacy_policy_patch_passes_through_unknown_legacy_keys ... ok
test deprecated_schema::tests::legacy_override_deny_rewrites_to_bypass_protection ... ok
test diagnostic_formatter::diagnostic_footer_tests::diagnostic_footer_silent_for_unrelated_errors ... ok
test diagnostic_formatter::diagnostic_footer_tests::diagnostic_footer_notes_no_auto_pull_when_set ... ok
test diagnostic_formatter::diagnostic_footer_tests::diagnostic_footer_silent_when_flag_unset ... ok
test dynamic_tokens::tests::expand_dynamic_tokens_errors_on_unknown_git_query ... ok
test dynamic_tokens::tests::expand_dynamic_tokens_errors_on_unknown_provider ... ok
test config::tests::test_check_sensitive_path ... ok
test dynamic_tokens::tests::expand_dynamic_tokens_passes_literal_paths_through_unchanged ... ok
test config::tests::test_check_sensitive_path_component_wise ... ok
test config::tests::test_validated_home_falls_back_to_userprofile ... FAILED
test config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists ... FAILED
test config::tests::user_state_dir_uses_localappdata_on_windows ... FAILED
test completions::tests::test_zsh_completions_contain_binary_name ... ok
test dynamic_tokens::tests::git_read_common_dir_returns_empty_outside_git_repo ... ok
test dynamic_tokens::tests::git_read_main_worktree_returns_empty_outside_git_repo ... ok
test dynamic_tokens::tests::git_read_files_includes_non_firing_includeif_target ... ok
test dynamic_tokens::tests::git_read_paths_with_global_returns_config_file_and_path_values ... ok
test dynamic_tokens::tests::git_read_common_dir_returns_dot_git_in_regular_repo ... ok
test dynamic_tokens::tests::git_read_main_worktree_returns_empty_in_regular_repo ... ok
test dynamic_tokens::tests::git_read_paths_with_global_walks_include_chain ... ok
test dynamic_tokens::tests::git_read_toplevel_parent_returns_empty_outside_git_repo ... ok
[master (root-commit) ea4edab] init
test dynamic_tokens::tests::git_read_toplevel_returns_empty_outside_git_repo ... ok
test dynamic_tokens::tests::is_include_path_key_matches_include_and_includeif_only ... ok
test dynamic_tokens::tests::parse_paths_from_stdout_dedupes_repeated_file_origins ... ok
test dynamic_tokens::tests::parse_paths_from_stdout_drops_include_targets_in_untrusted_scopes ... ok
test dynamic_tokens::tests::parse_paths_from_stdout_drops_local_and_worktree_scopes ... ok
test dynamic_tokens::tests::parse_paths_from_stdout_extracts_config_file_paths_into_files ... ok
test dynamic_tokens::tests::parse_paths_from_stdout_extracts_include_and_includeif_targets ... ok
test dynamic_tokens::tests::parse_paths_from_stdout_extracts_path_valued_keys ... ok
test dynamic_tokens::tests::parse_paths_from_stdout_ignores_non_file_origins ... ok
test dynamic_tokens::tests::parse_paths_from_stdout_routes_hooks_path_to_dirs ... ok
test dynamic_tokens::tests::parse_token_recognises_at_provider_colon_query ... ok
test dynamic_tokens::tests::parse_token_returns_none_for_at_without_colon ... ok
test dynamic_tokens::tests::parse_token_returns_none_for_empty_string ... ok
test dynamic_tokens::tests::parse_token_returns_none_for_literal_paths ... ok
[master (root-commit) ea4edab] init
Preparing worktree (new branch 'worktree')
test dynamic_tokens::tests::git_read_paths_excludes_per_repo_local_config_overrides ... ok
test dynamic_tokens::tests::git_read_paths_includeif_hasconfig_matches_remote ... ok
test exec_identity::tests::compute_propagates_canonicalize_errors ... ok
test dynamic_tokens::tests::git_read_toplevel_parent_returns_parent_of_repo_root ... ok
test exec_identity_windows::tests::sanitize_for_terminal_strips_ansi_escape_sequences ... ok
test exec_identity_windows::tests::missing_path_returns_invalid_or_query_failed ... ok
Preparing worktree (new branch 'worktree')
test dynamic_tokens::tests::git_read_toplevel_returns_absolute_path_in_regular_repo ... ok
HEAD is now at ea4edab init
test exec_identity::tests::compute_hashes_canonical_binary_bytes ... ok
test exec_strategy::attestation::broker_wire_contract_tests::every_emitted_broker_required_layer_is_attestable_by_the_broker ... ok
test exec_strategy::attestation::broker_wire_contract_tests::pty_broker_arm_is_not_asked_to_attest_app_container_profile ... ok
test exec_strategy::attestation::broker_wire_contract_tests::required_layers_for_broker_includes_only_broker_expected_abort_rows_for_the_arm ... ok
test exec_strategy::attestation::broker_wire_contract_tests::required_layers_for_broker_never_includes_non_broker_only_rows ... ok
test exec_strategy::attestation::latency_measurement::attest_and_decide_direct_cli_write_restricted_write_restricted_latency ... ok
test exec_strategy::attestation::registry_tests::app_container_profile_is_never_expected_at_direct_cli ... ok
test exec_strategy::attestation::registry_tests::mandatory_integrity_label_cannot_confirm_from_an_unconfined_medium_il_token ... ok
test exec_strategy::attestation::registry_tests::minifilter_absence_always_not_applicable ... ok
test exec_strategy::attestation::registry_tests::not_expected_for_arm_classifies_not_applicable ... ok
test exec_strategy::attestation::registry_tests::restricted_token_requires_this_launch_own_session_sid ... ok
test exec_strategy::attestation::registry_tests::wfp_egress_filters_classifies_from_preconfirmed_flag ... ok
test exec_strategy::attestation::tests::applied_configured_only_row_proceeds_without_a_downgrade ... ok
test exec_strategy::attestation::tests::attest_and_decide_accepts_known_layer_names ... ok
test exec_strategy::attestation::tests::attest_and_decide_rejects_unrecognized_required_layer_name_from_cli_flag ... ok
test exec_strategy::attestation::tests::attest_and_decide_rejects_unrecognized_required_layer_name_from_machine_policy ... ok
test exec_strategy::attestation::tests::empty_vec_is_unconfirmed ... ok
test exec_strategy::attestation::tests::err_is_unconfirmed ... ok
test exec_strategy::attestation::tests::negative_bool_is_unconfirmed ... ok
test exec_strategy::attestation::tests::negative_option_is_unconfirmed ... ok
test exec_strategy::attestation::tests::network_row_for_an_unselected_backend_is_not_applicable ... ok
test exec_strategy::attestation::tests::partially_applied_configured_only_row_proceeds_downgraded ... ok
test exec_strategy::attestation::tests::positive_bool_confirms ... ok
test exec_strategy::attestation::tests::positive_option_confirms ... ok
test exec_strategy::attestation::tests::positive_vec_confirms ... ok
test exec_strategy::attestation::tests::some_production_row_can_still_produce_a_downgrade ... ok
test exec_strategy::attestation::tests::substitute_equivalent_mechanism_falls_through_to_abort_when_alternate_unconfirmed ... ok
test exec_strategy::attestation::tests::substitute_equivalent_mechanism_proceeds_when_alternate_confirmed ... ok
test exec_strategy::attestation::tests::tightened_required_layer_aborts_even_when_default_outcome_is_fail_open ... ok
test exec_strategy::attestation::tests::token_arm_matches_debug_format ... ok
test exec_strategy::attestation::tests::token_arm_matches_none_none ... ok
test exec_strategy::attestation::tests::token_arm_mismatched_option_shape_is_false ... ok
test exec_strategy::attestation::tests::unapplied_configured_only_row_aborts ... ok
test exec_strategy::attestation::tests::untightened_fail_open_row_never_downgrades ... ok
test exec_strategy::attestation_downgrade_event::tests::emit_attestation_event_advances_chain_by_one_disabled ... ok
[master (root-commit) 672792f] init
HEAD is now at ea4edab init
test exec_strategy::attestation_downgrade_event::tests::emit_attestation_event_advances_chain_by_one_enabled ... ok
test exec_strategy::attestation_downgrade_event::tests::emit_attestation_event_err_on_poisoned_mutex ... ok
test dynamic_tokens::tests::git_read_common_dir_returns_absolute_path_in_worktree ... ok
Preparing worktree (new branch 'worktree')
test exec_identity_windows::tests::authenticode_signed_records_subject ... ok
test exec_identity_windows::tests::signed_system_binary_extracts_40_char_hex_thumbprint ... ok
test exec_identity_windows::tests::signed_system_binary_extracts_cn_subject ... ok
test dynamic_tokens::tests::git_read_main_worktree_returns_main_repo_root_in_linked_worktree ... ok
test exec_strategy::dacl_guard::tests::ancestor_read_attrs_application_reports_not_applicable_for_a_rootless_walk ... ok
test exec_strategy::dacl_guard::tests::ancestor_read_attrs_application_reports_not_applicable_when_stopped_at_non_owned ... ok
test exec_strategy::dacl_guard::tests::ancestor_read_attrs_application_reports_not_applied_for_the_unreachable_third_state ... ok
test exec_strategy::dacl_guard::tests::ancestor_traverse_application_reports_not_applicable_for_a_rootless_walk ... ok
test exec_strategy::dacl_guard::tests::ancestor_traverse_application_reports_not_applicable_when_stopped_at_non_owned ... ok
test exec_strategy::dacl_guard::tests::ancestor_traverse_application_reports_not_applied_for_the_unreachable_third_state ... ok
test exec_identity_windows::tests::unsigned_temp_file_returns_unsigned_or_invalid ... ok
HEAD is now at 672792f init
test dynamic_tokens::tests::git_read_toplevel_returns_worktree_root_in_linked_worktree ... ok
test exec_strategy::dacl_guard::tests::mid_loop_grant_failure_reverts_already_applied ... ok
test exec_strategy::dacl_guard::tests::read_only_rule_is_skipped_no_dacl_change ... ok
test exec_strategy::dacl_guard::tests::writable_rule_applies_sid_ace_and_reverts_on_drop ... ok
test exec_strategy::env_sanitization::tests::test_allows_non_aws_unrelated_var ... ok
test exec_strategy::env_sanitization::tests::test_allows_unrelated_env_vars ... ok
test exec_strategy::env_sanitization::tests::test_blocks_aws_access_key_id ... ok
test exec_strategy::env_sanitization::tests::test_blocks_aws_prefix_arbitrary_suffix ... ok
test exec_strategy::env_sanitization::tests::test_blocks_aws_region ... ok
test exec_strategy::env_sanitization::tests::test_blocks_aws_secret_access_key ... ok
test exec_strategy::env_sanitization::tests::test_blocks_aws_session_token ... ok
test exec_strategy::env_sanitization::tests::test_blocks_interpreter_injection ... ok
test exec_strategy::env_sanitization::tests::test_blocks_linker_injection ... ok
test exec_strategy::env_sanitization::tests::test_blocks_op_connect_host ... ok
test exec_strategy::env_sanitization::tests::test_blocks_op_connect_token ... ok
test exec_strategy::env_sanitization::tests::test_blocks_op_service_account_token ... ok
test exec_strategy::env_sanitization::tests::test_blocks_op_session_prefix ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_bare_star ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_empty_list ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_exact_match ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_exact_no_match ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_mid_star_ignored ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_mixed_patterns ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_prefix_does_not_match_partial ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_prefix_match ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_prefix_matches_empty_suffix ... ok
test exec_strategy::env_sanitization::tests::test_env_var_allowed_prefix_no_match ... ok
test exec_strategy::env_sanitization::tests::test_env_var_denied_empty_list ... ok
test exec_strategy::env_sanitization::tests::test_env_var_denied_exact_match ... ok
test exec_strategy::env_sanitization::tests::test_env_var_denied_no_match ... ok
test exec_strategy::env_sanitization::tests::test_env_var_denied_overrides_allowed ... ok
test exec_strategy::env_sanitization::tests::test_env_var_denied_prefix_match ... ok
test exec_strategy::env_sanitization::tests::test_set_vars_accepts_dangerous_keys ... ok
test exec_strategy::env_sanitization::tests::test_set_vars_accepts_normal_keys ... ok
test exec_strategy::env_sanitization::tests::test_set_vars_empty_is_ok ... ok
test exec_strategy::env_sanitization::tests::test_set_vars_rejects_invalid_names ... ok
test exec_strategy::env_sanitization::tests::test_set_vars_rejects_nono_prefix ... ok
test exec_strategy::env_sanitization::tests::test_set_vars_rejects_path ... ok
test exec_strategy::env_sanitization::tests::test_should_skip_env_var_matches_windows_keys_case_insensitively ... ok
test exec_strategy::env_sanitization::tests::test_validate_accepts_bare_star ... ok
test exec_strategy::env_sanitization::tests::test_validate_deny_vars_field_name_in_error ... ok
test exec_strategy::env_sanitization::tests::test_validate_exact_name_no_star ... ok
test exec_strategy::env_sanitization::tests::test_validate_rejects_leading_star_with_suffix ... ok
test exec_strategy::env_sanitization::tests::test_validate_rejects_mid_star ... ok
test exec_strategy::env_sanitization::tests::test_validate_valid_patterns ... ok
test exec_strategy::env_sanitization::tests::test_windows_dangerous_vars_blocked ... ok
test exec_strategy::interpreter_resolve_tests::resolve_interpreter_paths_empty_slice_returns_empty ... ok
test exec_strategy::interpreter_resolve_tests::resolve_interpreter_paths_falls_back_to_bare_name_when_unresolvable ... ok
test exec_strategy::interpreter_resolve_tests::resolve_interpreter_paths_returns_candidate_paths_not_grants ... ok
test exec_strategy::interpreter_resolve_tests::shebang_read_extracts_interpreter_from_fixture ... ok
test exec_strategy::interpreter_resolve_tests::shebang_read_returns_none_for_missing_file ... ok
test exec_strategy::interpreter_resolve_tests::shebang_read_returns_none_when_no_shebang ... ok
test exec_strategy::labels_guard::tests::audit_flush_before_drop ... ok
test exec_strategy::labels_guard::tests::coverage_distinguishes_full_partial_and_zero_ace_launches ... ok
test exec_strategy::labels_guard::tests::guard_apply_then_drop_reverts_label_for_fresh_file ... ok
test exec_strategy::labels_guard::tests::guard_reverts_all_entries_if_mid_loop_apply_fails ... ok
test exec_strategy::labels_guard::tests::guard_skips_apply_and_revert_when_path_already_has_any_mandatory_label ... ok
test exec_strategy::labels_guard::tests::guard_skips_path_not_owned_by_current_user ... ok
test exec_strategy::labels_guard::tests::inherit_only_residue_is_not_treated_as_already_covered ... ok
test exec_strategy::labels_guard::tests::mismatched_prior_mask_still_records_a_coverage_gap ... ok
test exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap ... FAILED
test exec_strategy::labels_guard::tests::residue_is_not_reverted_on_drop ... ok
test exec_strategy::labels_guard::tests::stale_residue_from_an_abnormal_exit_does_not_self_lock_out_the_next_launch ... ok
test exec_strategy::launch::apply_resource_limits_tests::all_three_limits_coexist ... ok
test exec_strategy::launch::apply_resource_limits_tests::cpu_rate_control_readback_matches_applied_value ... ok
test exec_strategy::launch::apply_resource_limits_tests::empty_limits_is_noop ... ok
test exec_strategy::launch::apply_resource_limits_tests::idempotent_same_limits_twice ... ok
test exec_strategy::launch::apply_resource_limits_tests::max_processes_readback_matches_applied_value ... ok
test exec_strategy::launch::apply_resource_limits_tests::memory_readback_matches_applied_value ... ok
test exec_strategy::launch::apply_resource_limits_tests::preserves_kill_on_job_close ... ok
test exec_strategy::launch::assign_failure_tests::apply_process_handle_to_containment_invalid_job_returns_err ... ok
test exec_strategy::launch::assign_failure_tests::assign_failure_message_generic_gle_contains_gle_value ... ok
test exec_strategy::launch::assign_failure_tests::assign_failure_message_gle5_contains_did_not_create ... ok
test exec_strategy::launch::attestation_gate_tests::dacl_ancestor_traverse_row_reports_partially_applied_from_a_real_gate ... ok
test exec_strategy::launch::attestation_gate_tests::firewall_rules_guard_yields_preconfirmed_false ... ok
test exec_strategy::launch::attestation_gate_tests::firewall_rules_row_can_be_selected_and_unconfirmed_from_a_real_gate ... ok
test exec_strategy::launch::attestation_gate_tests::firewall_rules_row_reports_installation_evidence_not_the_discriminant ... ok
test exec_strategy::launch::attestation_gate_tests::fully_applied_launch_passes_the_gate ... ok
test exec_strategy::launch::attestation_gate_tests::launch_missing_an_expected_configured_only_layer_is_refused ... ok
test exec_strategy::launch::attestation_gate_tests::live_child_not_in_the_containment_job_aborts_naming_job_object_containment ... ok
test exec_strategy::launch::attestation_gate_tests::no_network_enforcement_yields_preconfirmed_false ... ok
test exec_strategy::launch::attestation_gate_tests::partially_applied_launch_is_downgraded_not_silently_passed ... ok
test exec_strategy::launch::attestation_gate_tests::proceed_downgraded_security_layer_absent_arm_withholds_layer_names_on_the_shared_console_channel ... ok
test exec_strategy::launch::attestation_gate_tests::proceed_downgraded_success_path_logs_downgraded_layers_field_on_private_log_channel ... ok
test exec_strategy::launch::attestation_gate_tests::proceed_downgraded_success_path_withholds_layer_names_on_the_shared_console_channel ... ok
test exec_strategy::launch::attestation_gate_tests::unconfirmed_abort_outcome_layer_returns_layer_attestation_failed ... ok
test exec_strategy::launch::attestation_gate_tests::wfp_row_can_be_selected_and_unconfirmed_from_a_real_guard_value ... ok
test exec_strategy::launch::attestation_gate_tests::wfp_service_managed_yields_preconfirmed_true ... ok
test exec_strategy::launch::broker_authenticode_layout_tests::attacker_lookalike_target_path_is_not_dev_build ... ok
test exec_strategy::launch::broker_authenticode_layout_tests::current_test_binary_is_dev_build_via_baked_root ... ok
test exec_strategy::launch::broker_authenticode_layout_tests::empty_baked_root_disables_dev_skip ... ok
test exec_strategy::launch::broker_authenticode_layout_tests::production_install_paths_are_not_dev_build ... ok
test exec_strategy::launch::broker_authenticode_layout_tests::real_baked_target_root_is_dev_build ... ok
test exec_strategy::launch::broker_authenticode_layout_tests::uncanonicalizable_exe_is_not_dev_build ... ok
test exec_strategy::launch::broker_dispatch_tests::broker_launch_assigns_child_to_job_object ... ok
test exec_strategy::launch::broker_dispatch_tests::broker_not_found_error_variant_is_constructible_and_displays_path ... ok
test exec_strategy::launch::broker_dispatch_tests::build_broker_command_line_appends_argv_args_with_quoting ... ok
test exec_strategy::launch::broker_dispatch_tests::build_broker_command_line_emits_quoted_broker_path ... ok
test exec_strategy::launch::broker_dispatch_tests::build_broker_command_line_terminates_with_null ... ok
test exec_strategy::launch::broker_dispatch_tests::normalize_windows_launch_path_strips_verbatim_prefix ... ok
test exec_strategy::launch::broker_dispatch_tests::sc4_classify_real_agent ... ignored, requires real Win11 host + CreateAppContainerProfile; run with --ignored on a real host
test exec_strategy::launch::broker_dispatch_tests::sc4_classify_spoof_not_agent ... ignored, requires real Win11 host + CreateAppContainerProfile; run with --ignored on a real host
test exec_strategy::launch::detached_stdio_tests::child_ends_are_inheritable ... ok
test exec_strategy::launch::detached_stdio_tests::close_child_ends_zeroes_them ... ok
test exec_strategy::launch::detached_stdio_tests::detached_stdio_pipes_create_succeeds ... ok
test exec_strategy::launch::detached_stdio_tests::drop_closes_all_remaining_handles_without_panic ... ok
test exec_strategy::launch::detached_stdio_tests::parent_ends_are_non_inheritable ... ok
test exec_strategy::launch::detached_token_gate_tests::returns_false_when_env_is_other_value ... ok
test exec_strategy::launch::detached_token_gate_tests::returns_false_when_env_unset ... ok
test exec_strategy::launch::detached_token_gate_tests::returns_true_when_env_is_one ... ok
test exec_strategy::launch::env_filter_tests::test_windows_allow_passes_only_matching_env_vars ... ok
test exec_strategy::launch::env_filter_tests::test_windows_deny_strips_matching_env_vars ... ok
test exec_strategy::launch::env_filter_tests::test_windows_empty_allow_denies_all_env_vars ... ok
test exec_strategy::launch::env_filter_tests::test_windows_nono_injected_credentials_bypass_both ... ok
test exec_strategy::launch::env_filter_tests::test_windows_read_only_granted_dir_appended_to_path ... ok
test exec_strategy::launch::job_hardening_tests::job_never_has_breakaway_ok ... ok
test exec_strategy::launch::job_hardening_tests::job_security_descriptor_denies_low_il ... ok
test exec_strategy::launch::job_hardening_tests::job_security_descriptor_with_package_sid ... ok
test exec_strategy::launch::low_integrity_primary_token_tests::low_integrity_primary_token_drop_is_safe ... ok
test exec_strategy::launch::low_integrity_primary_token_tests::low_integrity_primary_token_sets_low_il ... ok
test exec_strategy::launch::pty_token_gate_tests::broker_launch_takes_precedence_over_session_sid_on_pty_path ... ok
test exec_strategy::launch::pty_token_gate_tests::detach_dominates_other_signals ... ok
test exec_strategy::launch::pty_token_gate_tests::pty_none_caps_demand_low_il_selects_low_il ... ok
test exec_strategy::launch::pty_token_gate_tests::pty_none_no_session_sid_selects_null_fallback ... ok
test exec_strategy::launch::pty_token_gate_tests::pty_none_session_sid_with_broker_opt_in_selects_broker_launch_no_pty ... ok
test exec_strategy::launch::pty_token_gate_tests::pty_none_with_session_sid_selects_write_restricted ... ok
test exec_strategy::launch::pty_token_gate_tests::pty_some_no_detach_selects_broker_launch ... ok
test exec_strategy::launch::pty_token_gate_tests::pty_some_with_detach_selects_null ... ok
[2m2026-08-12T03:38:59.445333Z[0m [32m INFO[0m [2mnono_shell_broker::broker[0m[2m:[0m broker: console attach probe [3malloc_console_rc[0m[2m=[0m0
[2m2026-08-12T03:38:59.445496Z[0m [32m INFO[0m [2mnono_shell_broker::broker[0m[2m:[0m broker: Low-IL primary token constructed
[2m2026-08-12T03:38:59.460866Z[0m [32m INFO[0m [2mnono_shell_broker::broker[0m[2m:[0m broker: spawned Low-IL child [3mchild_pid[0m[2m=[0m8644
[2m2026-08-12T03:38:59.497246Z[0m [32m INFO[0m [2mnono_shell_broker::broker[0m[2m:[0m broker: child exited [3mchild_exit_code[0m[2m=[0m1
test exec_strategy::launch::write_deny_low_il_broker_no_pty_tests::write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file ... ok
test exec_strategy::layer_registry::tests::all_entries_covers_every_layer_id ... ok
test exec_strategy::layer_registry::tests::at_least_one_row_is_broker_expected ... ok
test exec_strategy::layer_registry::tests::broker_expected_rows_are_abort_only ... ok
test exec_strategy::layer_registry::tests::dacl_session_sid_grant_is_not_claimed_anywhere ... ok
test exec_strategy::layer_registry::tests::every_call_site_string_names_a_line_number ... ok
test exec_strategy::layer_registry::tests::every_consulting_row_with_an_expectancy_has_a_reported_field ... ok
test exec_strategy::layer_registry::tests::every_reported_layer_is_actually_consulted ... ok
test exec_strategy::layer_registry::tests::unreported_layer_application_defaults_to_not_applied ... ok
test exec_strategy::network::tests::build_wfp_target_activation_request_leaves_session_sid_none_for_appid_fallback ... ok
test exec_strategy::network::tests::build_wfp_target_activation_request_populates_session_sid ... ok
test exec_strategy::network::tests::cleanup_refuses_traversal_the_root_and_paths_outside_the_root ... ok
test exec_strategy::network::tests::describe_windows_network_runtime_target_surfaces_localhost_port_ranges ... ok
test exec_strategy::network::tests::install_wfp_network_backend_returns_error_on_prerequisites_missing ... ok
test exec_strategy::network::tests::install_wfp_network_backend_returns_guard_on_enforced_pending_cleanup ... ok
test exec_strategy::network::tests::test_wfp_autostart_fail_remediation_message ... ok
test exec_strategy::network::tests::test_wfp_autostart_on_stopped ... ok
test exec_strategy::network::tests::test_wfp_backend_binary_missing_is_fail_closed ... ok
test exec_strategy::network::tests::test_wfp_backend_service_missing_is_fail_closed ... ok
test exec_strategy::network::tests::test_wfp_backend_service_stopped_is_fail_closed ... ok
test exec_strategy::network::tests::test_wfp_non_stopped_status_unchanged ... ok
test exec_strategy::network::tests::test_wfp_platform_service_stopped_is_fail_closed ... ok
test exec_strategy::network::tests::test_wfp_ready_without_kernel_driver ... ok
test exec_strategy::network::tests::uninstall_deletes_stopped_service_without_stopping_it ... ok
test exec_strategy::network::tests::uninstall_is_noop_when_nothing_installed ... ok
test exec_strategy::network::tests::uninstall_propagates_delete_failure ... ok
test exec_strategy::network::tests::uninstall_purge_failure_is_fail_open ... ok
test exec_strategy::network::tests::uninstall_stops_and_deletes_both_running_services ... ok
test exec_strategy::network::tests::zero_installed_filters_fails_closed_even_when_service_reports_success ... ok
test exec_strategy::rb3_gate_tests::rb3_error_message_names_cause_and_fix_without_auto_takeown ... ok
test exec_strategy::rb3_gate_tests::system_dir_lacks_write_owner_for_standard_user ... ok
test exec_strategy::rb3_gate_tests::workspace_owned_by_current_user_passes_write_owner_check ... ok
test exec_strategy::restricted_token::tests::create_restricted_token_with_sid_applies_write_restricted_flag ... ok
test exec_strategy::restricted_token::tests::create_restricted_token_with_sid_returns_usable_handle_for_child_spawn ... ok
test exec_strategy::restricted_token::tests::generate_app_container_name_is_unique_and_well_formed ... ok
test exec_strategy::restricted_token::tests::generate_session_sid_produces_parsable_sddl_string ... ok
test exec_strategy::restricted_token::tests::restricted_token_drop_is_null_safe ... ok
test exec_strategy::supervisor::capability_handler_tests::audit_integrity_records_5_handle_kinds_in_ledger ... ok
test exec_strategy::supervisor::capability_handler_tests::default_allowlist_rejects_pipe_readwrite_after_prompt ... ok
test exec_strategy::supervisor::capability_handler_tests::dispatcher_flips_approved_to_denied_on_event_broker_failure ... ok
test exec_strategy::supervisor::capability_handler_tests::dispatcher_flips_approved_to_denied_on_job_object_broker_failure ... ok
test exec_strategy::supervisor::capability_handler_tests::dispatcher_flips_approved_to_denied_on_mutex_broker_failure ... ok
test exec_strategy::supervisor::capability_handler_tests::dispatcher_flips_approved_to_denied_on_pipe_broker_failure ... ok
test exec_strategy::supervisor::capability_handler_tests::dispatcher_flips_approved_to_denied_on_socket_broker_failure ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_brokers_event_with_default_mask ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_brokers_job_object_with_query_mask ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_brokers_mutex_with_default_mask ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_brokers_pipe_with_read_direction ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_brokers_socket_with_bind_role_when_profile_widens ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_brokers_socket_with_connect_role ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_consults_backend_for_valid_token ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_denies_event_with_mask_outside_allowlist ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_denies_job_object_brokering_of_containment_job ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_denies_job_object_brokering_of_containment_job_even_with_profile_widening ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_denies_job_object_with_terminate_mask_no_profile_widening ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_denies_mutex_with_mask_outside_allowlist ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_denies_pipe_with_invalid_target_shape ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_denies_socket_bind_role_without_profile_widening ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_denies_socket_with_privileged_port ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_denies_unknown_discriminator ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_redacts_token_in_audit_entry_json ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_redacts_token_in_audit_for_all_handle_kinds ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_redacts_token_in_serialized_audit ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_redacts_token_on_mismatch_audit ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_rejects_missing_token ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_rejects_replay_with_valid_token ... ok
test exec_strategy::supervisor::capability_handler_tests::handle_rejects_wrong_token ... ok
test exec_strategy::supervisor::capability_handler_tests::profile_widening_for_pipe_readwrite_reaches_backend ... ok
test exec_strategy::supervisor::capability_handler_tests::profile_widening_for_socket_bind_reaches_backend ... ok
test exec_strategy::supervisor::capability_handler_tests::recorded_ledger_redacts_session_token ... ok
test exec_strategy::supervisor::capability_handler_tests::recorder_does_not_abort_dispatcher_on_lock_poison ... ok
test exec_strategy::supervisor::capability_handler_tests::recorder_emission_is_optional_when_none ... ok
test exec_strategy::supervisor::capability_handler_tests::recorder_emits_one_capability_decision_per_dispatched_request ... ok
test exec_strategy::supervisor::capability_handler_tests::wr01_event_rejects_before_prompt_on_out_of_allowlist_mask ... ok
test exec_strategy::supervisor::capability_handler_tests::wr01_job_object_rejects_before_prompt_on_terminate_mask ... ok
test exec_strategy::supervisor::capability_handler_tests::wr01_mutex_rejects_before_prompt_on_out_of_allowlist_mask ... ok
test exec_strategy::supervisor::capability_handler_tests::wr01_pipe_rejects_after_prompt_on_readwrite_default_profile ... ok
test exec_strategy::supervisor::capability_handler_tests::wr01_socket_privileged_port_rejects_after_prompt_empirical ... ok
test exec_strategy::supervisor::timeout_deadline_tests::compute_deadline_adds_duration ... ok
test exec_strategy::supervisor::timeout_deadline_tests::compute_deadline_none_when_no_timeout ... ok
test exec_strategy::supervisor::timeout_deadline_tests::compute_deadline_rejects_overflow ... ok
test exec_strategy::supervisor::timeout_deadline_tests::deadline_never_returns_natural_exit ... ok
test exec_strategy::supervisor::timeout_deadline_tests::deadline_reached_terminates_job_and_returns_timeout_code ... ok
test exec_strategy::supervisor::timeout_deadline_tests::terminate_job_object_fails_on_invalid_handle ... ok
test execution_runtime::tests::compute_executable_identity_hashes_canonical_binary_bytes ... ok
test execution_runtime::tests::recommended_builtin_profile_ignores_unknown_commands ... ok
test execution_runtime::tests::recommended_builtin_profile_matches_known_agent_commands ... ok
test format_util::tests::format_bytes_short_falls_back_to_raw_on_non_round_values ... ok
test format_util::tests::format_bytes_short_handles_unit_boundaries ... ok
test format_util::tests::format_bytes_short_uses_largest_applicable_unit ... ok
test health::tests::test_aggregate_all_ok_is_healthy ... ok
test health::tests::test_aggregate_broken_wins ... ok
test health::tests::test_aggregate_degraded_without_broken ... ok
test health::tests::test_aggregate_empty_is_healthy ... ok
test health::tests::test_aggregate_multiple_broken ... ok
test health::tests::test_subsystem_state_broken_has_detail ... ok
test health::tests::test_subsystem_state_degraded_has_detail ... ok
test health::tests::test_subsystem_state_ok_has_no_detail ... ok
test hook_runtime_windows::tests::test_cr02_timeout_hook_exits_cleanly ... ok
test hook_runtime_windows::tests::test_env_file_create_new_prevents_clobber ... ok
test hook_runtime_windows::tests::test_execute_before_hook_powershell_does_not_clr_fail ... ok
test hook_runtime_windows::tests::test_strip_verbatim_prefix_deterministic ... ok
test hook_runtime_windows::tests::test_validate_hook_script_windows_rejects_non_file ... ok
test hook_runtime_windows::tests::test_validate_hook_script_windows_rejects_relative ... ok
test hook_runtime_windows::tests::test_validate_rejects_world_writable_parent ... ok
test hook_runtime_windows::tests::test_windows_dangerous_vars_filtered_from_env_file ... ok
test hooks::tests::test_claude_json_accepts_target_inside_root ... ok
test hooks::tests::test_claude_json_rejects_path_traversal ... ok
test hooks::tests::test_embedded_script_content ... ok
test hooks::tests::test_embedded_script_exists ... ok
test hooks::tests::test_embedded_tool_hook_fails_closed ... ok
test hooks::tests::test_embedded_tool_hook_separates_stderr_from_json_contract ... ok
test hooks::tests::test_install_claude_json_symlink_does_not_panic ... ok
test hooks::tests::test_update_claude_settings_moves_existing_hook_between_events ... ok
test hooks::tests::test_update_claude_settings_registers_powershell_hook ... ok
test hooks::tests::test_update_claude_settings_updates_existing_hook_matcher ... ok
test instruction_deny::tests::write_protect_with_no_paths_is_noop ... ok
test learn_windows::tests::classify_and_record_file_access_filters_zone_identifier ... ok
test learn_windows::tests::strip_ads_suffix_only_considers_final_segment ... ok
test learn_windows::tests::strip_ads_suffix_passthrough_no_stream ... ok
test learn_windows::tests::strip_ads_suffix_preserves_named_pipe_without_colon ... ok
test learn_windows::tests::strip_ads_suffix_removes_arbitrary_stream_name ... ok
test learn_windows::tests::strip_ads_suffix_removes_zone_identifier ... ok
test learn_windows::tests::test_build_volume_map_runs_without_panic ... ok
test learn_windows::tests::test_classify_deduplicates_repeated_paths ... ok
test learn_windows::tests::test_classify_descendant_pid_records_path ... ok
test learn_windows::tests::test_classify_multiple_distinct_paths ... ok
test learn_windows::tests::test_classify_tracked_pid_records_path ... ok
test learn_windows::tests::test_classify_unconvertible_path_is_noop ... ok
test learn_windows::tests::test_classify_untracked_pid_is_noop ... ok
test learn_windows::tests::test_non_admin_returns_learn_error ... ok
test learn_windows::tests::test_nt_to_win32_happy_path ... ok
test learn_windows::tests::test_nt_to_win32_named_pipe_returns_none ... ok
test learn_windows::tests::test_nt_to_win32_unknown_device_returns_none ... ok
test learn_windows::tests::test_nt_to_win32_volume_prefix_boundary ... ok
test learn_windows::tests::test_process_tree_add_child_of_tracked_parent ... ok
test learn_windows::tests::test_process_tree_double_add_idempotent ... ok
test learn_windows::tests::test_process_tree_exit_removes ... ok
test learn_windows::tests::test_process_tree_exit_untracked_is_noop ... ok
test learn_windows::tests::test_process_tree_grandchild_inherits ... ok
test learn_windows::tests::test_process_tree_reserved_pids_rejected ... ok
test learn_windows::tests::test_process_tree_root_seeded ... ok
test learn_windows::tests::test_process_tree_skip_child_of_untracked_parent ... ok
test learn_windows::tests::test_record_listening_deduplicates_same_port ... ok
test learn_windows::tests::test_record_listening_multiple_ports_accumulate ... ok
test learn_windows::tests::test_record_listening_tracked_pid_appends ... ok
test learn_windows::tests::test_record_listening_untracked_pid_is_noop ... ok
test learn_windows::tests::test_record_outbound_deduplicates_same_endpoint ... ok
test learn_windows::tests::test_record_outbound_multiple_connections_accumulate ... ok
test learn_windows::tests::test_record_outbound_tracked_pid_appends ... ok
test learn_windows::tests::test_record_outbound_untracked_pid_is_noop ... ok
test network_policy::tests::is_loopback_domain_does_not_match_127_example_com ... ok
test network_policy::tests::is_loopback_domain_does_not_match_public_host ... ok
test network_policy::tests::is_loopback_domain_matches_0_0_0_0 ... ok
test network_policy::tests::is_loopback_domain_matches_127_0_0_1 ... ok
test network_policy::tests::is_loopback_domain_matches_127_x_x_x_cidr ... ok
test network_policy::tests::is_loopback_domain_matches_ipv6_loopback ... ok
test network_policy::tests::is_loopback_domain_matches_localhost ... ok
test network_policy::tests::partition_allow_domain_127_example_com_uses_https_scheme ... ok
test network_policy::tests::partition_allow_domain_empty_domain_returns_error ... ok
test network_policy::tests::partition_allow_domain_empty_endpoints_treated_as_plain ... ok
test network_policy::tests::partition_allow_domain_loopback_uses_http_scheme ... ok
test network_policy::tests::partition_allow_domain_plain_entry_goes_to_plain_hosts ... ok
test network_policy::tests::partition_allow_domain_with_endpoints_creates_route ... ok
test network_policy::tests::test_build_proxy_config ... ok
test network_policy::tests::test_build_proxy_config_deny_only_sets_strict_filter ... ok
test network_policy::tests::test_build_proxy_config_propagates_denied_hosts ... ok
test network_policy::tests::test_claude_code_profile_does_not_enable_credentials_by_default ... ok
test network_policy::tests::test_codex_profile_does_not_enable_credentials_by_default ... ok
test network_policy::tests::test_collect_allow_domain_port_warnings_detects_host_port_entries ... ok
test network_policy::tests::test_collect_allow_domain_port_warnings_ignores_plain_hosts_and_groups ... ok
test network_policy::tests::test_custom_credential_http_0_0_0_0_allowed ... ok
test network_policy::tests::test_custom_credential_http_127_cidr_allowed ... ok
test network_policy::tests::test_custom_credential_http_localhost_allowed ... ok
test network_policy::tests::test_custom_credential_with_valid_header ... ok
test network_policy::tests::test_deduplication ... ok
test network_policy::tests::test_developer_profile_does_not_enable_github_credential_by_default ... ok
test network_policy::tests::test_developer_profile_does_not_enable_gitlab_credential_by_default ... ok
test network_policy::tests::test_embedded_network_profiles_do_not_enable_credentials_by_default ... ok
test network_policy::tests::test_load_embedded_network_policy ... ok
test network_policy::tests::test_resolve_credentials_builtin_without_env_var ... ok
test network_policy::tests::test_resolve_credentials_by_name ... ok
test network_policy::tests::test_resolve_credentials_custom_capture_threaded_to_route_config ... ok
test network_policy::tests::test_resolve_credentials_custom_overrides_builtin ... ok
test network_policy::tests::test_resolve_credentials_empty_returns_none ... ok
test network_policy::tests::test_resolve_credentials_filtered ... ok
test network_policy::tests::test_resolve_credentials_mixed_custom_and_builtin ... ok
test network_policy::tests::test_resolve_credentials_propagates_env_var ... ok
test network_policy::tests::test_resolve_credentials_rejects_dangerous_env_var ... ok
test network_policy::tests::test_resolve_credentials_unknown_service_fails ... ok
test network_policy::tests::test_resolve_credentials_with_custom ... ok
test network_policy::tests::test_resolve_credentials_with_oauth2_auth ... ok
test network_policy::tests::test_resolve_credentials_without_oauth2_has_none ... ok
test network_policy::tests::test_resolve_developer_profile ... ok
test network_policy::tests::test_resolve_enterprise_has_suffixes ... ok
test network_policy::tests::test_resolve_github_credential ... ok
test network_policy::tests::test_resolve_gitlab_credential ... ok
test network_policy::tests::test_resolve_minimal_profile ... ok
test network_policy::tests::test_resolve_nonexistent_profile ... ok
test output::attestation_marker_path_tests::accepts_the_generated_session_id_shapes ... ok
test output::attestation_marker_path_tests::rejects_traversal_and_separator_shapes ... ok
test output::tests::attestation_downgrade_banner_cold_vs_warm_dedup_marker_latency ... ok
test output::tests::dry_run_command_line_redacts_default_secrets ... ok
test output::tests::dry_run_command_line_uses_configured_redaction_policy ... ok
test output::tests::normalize_terminal_line_endings_uses_crlf ... ok
test output::tests::print_blocked_grants_collapsed_and_verbose_do_not_panic ... ok
test output::tests::print_capabilities_with_unix_socket_does_not_panic ... ok
test output::tests::print_profile_hint_is_noop_when_silent ... ok
test output::tests::render_diagnostic_footer_preserves_line_structure ... ok
test output::tests::render_diagnostic_footer_splits_path_on_last_paren_group ... ok
test output::tests::unix_socket_mode_badges_are_fixed_width_and_distinct ... ok
test override_audit_emit::tests::emit_event_returns_chain_head_hex_not_raw_material ... ok
test override_audit_emit::tests::emit_override_audit_event_advances_chain_by_one_via_fresh_layer ... ok
test override_audit_emit::tests::emit_revoked_event_advances_chain_by_one ... ok
test override_audit_emit::tests::rejected_kind_maps_to_event_id_10008 ... ok
test override_audit_emit::tests::revoked_kind_maps_to_event_id_10010 ... ok
test override_request::tests::bundle_json_has_required_keys ... ok
test override_request::tests::bundle_repo_context_absent_emits_empty_string ... ok
test override_request::tests::bundle_scope_contains_provided_paths_and_domains ... ok
test override_request::tests::nonce_is_32_hex_chars ... ok
test override_request::tests::nonce_is_distinct_across_invocations ... ok
test pack_update_hint::tests::is_newer_returns_false_on_downgrade_or_equal ... ok
test pack_update_hint::tests::is_newer_returns_true_on_genuine_upgrade ... ok
test pack_update_hint::tests::is_newer_suppresses_hint_on_prerelease_installed ... ok
test pack_update_hint::tests::pack_hint_env_var_opts_out ... ok
test pack_update_hint::tests::refresh_helper_args_reject_odd_values ... ok
test pack_update_hint::tests::refresh_helper_args_round_trip ... ok
test package::tests::artifact_type_plugin_round_trips ... ok
test package::tests::artifact_type_unknown_fails_closed ... ok
test package::tests::package_status_response_partial_deserialize ... ok
test package::tests::package_status_response_serde_roundtrip ... ok
test package::tests::parses_package_ref_with_version ... ok
test package::tests::profile_drafts_dir_resolves_under_config_dir ... ok
test package::tests::profile_drafts_dir_windows_appdata ... ok
test package::tests::rejects_invalid_package_ref ... ok
test package_cmd::tests::compare_versions_honors_prerelease_ordering ... ok
test package_cmd::tests::remove_external_artifacts_preserves_shared_hook_scripts ... ok
test package_cmd::tests::remove_external_artifacts_still_removes_non_hook_files ... ok
test package_cmd::tests::validate_path_within_rejects_symlink_escape ... ok
test package_cmd::tests::validate_relative_path_rejects_absolute_path ... ok
test package_cmd::tests::validate_relative_path_rejects_traversal ... ok
test package_status::tests::canonical_package_refs_target_official_packs ... ok
test package_status::tests::enforce_for_active_profile_no_profile_ok ... ok
test package_status::tests::enforce_for_active_profile_non_official_profile_ok ... ok
test package_status::tests::official_profile_names_include_claude_and_codex ... ok
test package_status::tests::yanked_message_pins_latest_when_available ... ok
test platform::tests::compare_versions_is_symmetric_on_non_numeric_segments ... ok
test platform::tests::linux_predicates_match_distro_like_version_and_variant ... ok
test platform::tests::macos_version_predicates_match ... ok
test platform::tests::negation_and_any_of_work ... ok
test platform::tests::parse_os_release_handles_quotes_and_id_like ... ok
test platform::tests::parse_windows_registry_value_accepts_case_mismatch ... ok
test platform::tests::parse_windows_registry_value_rejects_malformed_dword ... ok
test platform::tests::unknown_comparator_is_error ... ok
test platform::tests::unknown_os_is_false_not_error ... ok
test platform::tests::version_segments_compare_numerically_when_possible ... ok
test platform::tests::windows_build_predicate_parses_and_matches ... ok
test platform::tests::windows_registry_dword_values_are_decimalized ... ok
test policy::tests::policy_egress_groups_expand_anthropic_token ... ok
test policy::tests::policy_egress_groups_expand_github_api_token ... ok
test policy::tests::policy_egress_groups_expand_openai_token ... ok
test policy::tests::policy_egress_groups_present_in_network_policy ... ok
test policy::tests::policy_egress_groups_union_hosts ... ok
test policy::tests::policy_egress_groups_unknown_token_expands_to_empty ... ok
test policy::tests::test_all_groups_no_deny_within_allow_overlap ... ok
test policy::tests::test_apply_deny_overrides_does_not_remove_broader_deny ... ok
test policy::tests::test_apply_deny_overrides_empty_is_noop ... ok
test policy::tests::test_apply_deny_overrides_rejects_group_sourced_grant ... ok
test policy::tests::test_apply_deny_overrides_rejects_missing_grant ... ok
test policy::tests::test_apply_deny_overrides_removes_from_deny_paths ... ok
test policy::tests::test_apply_unlink_overrides_no_op_on_non_macos ... ok
test policy::tests::test_default_user_groups_do_not_grant_local_state ... ok
test policy::tests::test_deny_access_collects_path_and_generates_rules ... ok
test policy::tests::test_deny_access_non_symlink_no_duplicate ... ok
test policy::tests::test_embedded_claude_code_platform_groups_filter_by_os ... ok
test policy::tests::test_embedded_claude_code_platform_groups_have_expected_paths ... ok
test policy::tests::test_embedded_claude_code_profile_uses_platform_groups_for_os_paths ... ok
test policy::tests::test_embedded_policy_includes_windows_system_read_group ... ok
test policy::tests::test_embedded_policy_required_groups ... ok
test policy::tests::test_escape_seatbelt_path ... ok
test policy::tests::test_escape_seatbelt_path_injection_via_newline ... ok
test policy::tests::test_escape_seatbelt_path_injection_via_quote ... ok
test policy::tests::test_escape_seatbelt_path_rejects_control_chars ... ok
test policy::tests::test_expand_path_absolute ... ok
test policy::tests::test_expand_path_tilde ... ok
test policy::tests::test_expand_path_tmpdir ... ok
test policy::tests::test_find_denied_user_grants_detects_overlap ... ok
test policy::tests::test_find_denied_user_grants_ignores_non_user_grants ... ok
test policy::tests::test_find_denied_user_grants_profile_deny_without_group ... ok
test policy::tests::test_group_description ... ok
test policy::tests::test_linux_compat_groups_expose_expected_paths ... ok
test policy::tests::test_linux_core_excludes_runtime_state_sysfs_temp_and_nix ... ok
test policy::tests::test_list_groups ... ok
test policy::tests::test_load_embedded_policy ... ok
test policy::tests::test_load_policy ... ok
test policy::tests::test_nix_runtime_group_includes_nix_store ... ok
test policy::tests::test_platform_filtering ... ok
test policy::tests::test_profile_def_to_raw_profile_combines_exclude_groups ... ok
test policy::tests::test_resolve_command_group ... ok
test policy::tests::test_resolve_deny_group ... ok
test policy::tests::test_resolve_deny_group_collects_deny_paths ... ok
test policy::tests::test_resolve_parent_symlinks_existing_path ... ok
test policy::tests::test_resolve_read_group ... ok
test policy::tests::test_schema_has_session_hooks_defs ... ok
test policy::tests::test_schema_has_session_hooks_property ... ok
test policy::tests::test_should_skip_resolved_deny_target ... ok
test policy::tests::test_symlink_pairs ... ok
test policy::tests::test_system_read_linux_core_does_not_grant_bare_etc_or_proc ... ok
test policy::tests::test_to_raw_profile_includes_session_hooks ... ok
test policy::tests::test_unknown_group_error ... ok
test policy::tests::test_unlink_protection ... ok
test policy::tests::test_user_caches_macos_includes_dot_cache ... ok
test policy::tests::test_validate_deny_overlaps_detects_conflict ... ok
test policy::tests::test_validate_deny_overlaps_no_false_positive ... ok
test policy::tests::test_validate_group_exclusions_allows_non_required ... ok
test policy::tests::test_validate_group_exclusions_ignores_unknown ... ok
test policy::tests::test_validate_group_exclusions_rejects_required ... ok
test profile::allow_domain_tests::allow_domain_entry_domain_accessor ... ok
test profile::allow_domain_tests::allow_domain_entry_mixed_array_deserializes ... ok
test profile::allow_domain_tests::allow_domain_entry_plain_string_deserializes ... ok
test profile::allow_domain_tests::allow_domain_entry_with_empty_endpoints_deserializes ... ok
test profile::allow_domain_tests::allow_domain_entry_with_endpoints_deserializes ... ok
test profile::allow_domain_tests::merge_allow_domain_plain_and_with_endpoints_produces_with_endpoints ... ok
test profile::allow_domain_tests::merge_allow_domain_preserves_order ... ok
test profile::allow_domain_tests::merge_allow_domain_two_plain_same_domain_produces_single_plain ... ok
test profile::allow_domain_tests::merge_allow_domain_union_endpoints_same_domain ... ok
test profile::allow_domain_tests::merge_profiles_dedup_appends_deny_domain ... ok
test profile::allow_domain_tests::merge_profiles_dedup_appends_no_proxy ... ok
test profile::allow_domain_tests::network_config_allow_domain_field_is_vec_allow_domain_entry ... ok
test profile::allow_domain_tests::network_config_deny_domain_defaults_empty ... ok
test profile::allow_domain_tests::network_config_deny_domain_field_deserializes ... ok
test profile::allow_domain_tests::network_config_no_proxy_defaults_empty ... ok
test profile::allow_domain_tests::network_config_no_proxy_field_deserializes ... ok
test profile::allow_domain_tests::test_finalize_profile_rejects_inherited_no_proxy_allow_domain_overlap ... ok
test profile::allow_domain_tests::test_network_no_proxy_non_overlapping_allow_domain_is_valid ... ok
test profile::allow_domain_tests::test_network_no_proxy_rejects_allow_domain_overlap ... ok
test profile::builtin::tests::claude_no_keychain_loads ... ok
test profile::builtin::tests::copilot_cli_profile_declares_node_interpreter ... ok
test profile::builtin::tests::copilot_cli_profile_present ... ok
test profile::builtin::tests::test_all_profiles_signal_mode_resolves ... ok
test profile::builtin::tests::test_bare_name_claude_code_still_resolves ... ok
test profile::builtin::tests::test_default_profile_group_set_is_explicit ... ok
test profile::builtin::tests::test_embedded_profiles_extend_default ... ok
test profile::builtin::tests::test_get_builtin_aider ... ok
test profile::builtin::tests::test_get_builtin_claude_code ... ok
test profile::builtin::tests::test_get_builtin_claude_code_by_namespace_alias ... ok
test profile::builtin::tests::test_get_builtin_claude_code_uses_platform_groups_for_os_paths ... ok
test profile::builtin::tests::test_get_builtin_claude_no_kc ... ok
test profile::builtin::tests::test_get_builtin_claude_no_kc_does_not_relax_keychain_denies ... ok
test profile::builtin::tests::test_get_builtin_codex ... ok
test profile::builtin::tests::test_get_builtin_default ... ok
test profile::builtin::tests::test_get_builtin_langchain_python ... ok
test profile::builtin::tests::test_get_builtin_nonexistent ... ok
test profile::builtin::tests::test_get_builtin_nono_ts_default_by_namespace_alias ... ok
test profile::builtin::tests::test_get_builtin_nono_ts_wfp_test_blocked_by_namespace_alias ... ok
test profile::builtin::tests::test_get_builtin_nono_ts_wfp_test_open_by_namespace_alias ... ok
test profile::builtin::tests::test_get_builtin_openclaw ... ok
test profile::builtin::tests::test_get_builtin_swival ... ok
test profile::builtin::tests::test_get_builtin_swival_by_namespace_alias ... ok
test profile::builtin::tests::test_linux_host_compat_profile_groups ... ok
test profile::builtin::tests::test_linux_interactive_profiles_include_sysfs_but_not_runtime_state_or_temp ... ok
test profile::builtin::tests::test_list_builtin ... ok
test profile::builtin::tests::test_opencode_no_longer_inbuilt ... ok
test profile::builtin::tests::test_profile_exclusion_mechanism ... ok
test profile::builtin::tests::test_profile_extends_indirect_cycle_detected ... ok
test profile::builtin::tests::test_profile_extends_linear_chain_succeeds ... ok
test profile::builtin::tests::test_profile_extends_self_reference_detected ... ok
test profile::builtin::tests::test_profile_group_merging ... ok
test profile::canonical_schema_rename_tests::canonical_bypass_protection_key_deserializes_directly ... ok
test profile::canonical_schema_rename_tests::canonical_bypass_protection_key_does_not_match_legacy_detector ... ok
test profile::canonical_schema_rename_tests::commands_config_allow_and_deny_deserializes ... ok
test profile::canonical_schema_rename_tests::commands_config_rejects_unknown_fields ... ok
test profile::canonical_schema_rename_tests::filesystem_canonical_deny_and_bypass_protection_deserializes ... ok
test profile::canonical_schema_rename_tests::filesystem_empty_block_defaults_new_fields_to_empty ... ok
test profile::canonical_schema_rename_tests::filesystem_legacy_override_deny_alias_to_bypass_protection ... ok
test profile::canonical_schema_rename_tests::json_value_has_key_handles_arrays ... ok
test profile::canonical_schema_rename_tests::json_value_has_key_walks_nested_objects ... ok
test profile::canonical_schema_rename_tests::legacy_filesystem_override_deny_normalizes_to_bypass_protection ... ok
test profile::canonical_schema_rename_tests::legacy_override_deny_key_detected ... ok
test profile::canonical_schema_rename_tests::legacy_override_deny_key_still_deserializes ... ok
test profile::canonical_schema_rename_tests::malformed_json_returns_false_silently ... ok
test profile::canonical_schema_rename_tests::profile_canonical_sections_serialize_at_correct_nesting ... ok
test profile::canonical_schema_rename_tests::profile_empty_commands_block_defaults_correctly ... ok
test profile::canonical_schema_rename_tests::profile_round_trip_with_canonical_sections ... ok
test profile::canonical_schema_rename_tests::raw_profile_has_both_bypass_and_override_keys_canonical_only ... ok
test profile::canonical_schema_rename_tests::raw_profile_has_both_bypass_and_override_keys_detects_both ... ok
test profile::canonical_schema_rename_tests::raw_profile_has_both_bypass_and_override_keys_detects_both_in_jsonc ... ok
test profile::canonical_schema_rename_tests::raw_profile_has_both_bypass_and_override_keys_legacy_only ... ok
test profile::canonical_schema_rename_tests::raw_profile_has_legacy_override_deny_key_detects_in_jsonc ... ok
test profile::credential_provider::tests::double_dot_field_path_rejected ... ok
test profile::credential_provider::tests::empty_field_path_rejected ... ok
test profile::credential_provider::tests::empty_response_fields_rejected ... ok
test profile::credential_provider::tests::invalid_request_nonce_field_rejected ... ok
test profile::credential_provider::tests::leading_dot_field_path_rejected ... ok
test profile::credential_provider::tests::max_response_bytes_above_ceiling_rejected ... ok
test profile::credential_provider::tests::max_response_bytes_at_ceiling_accepted ... ok
test profile::credential_provider::tests::max_response_bytes_none_accepted ... ok
test profile::credential_provider::tests::trailing_dot_field_path_rejected ... ok
test profile::credential_provider::tests::valid_capture_config_passes ... ok
test profile::credential_provider::tests::zero_max_response_bytes_rejected ... ok
test profile::d08_deviation_tests::test_wsl2_proxy_policy_deviation_preserved ... ok
test profile::platform_overrides_tests::merge_profiles_unions_port_ranges ... ok
test profile::platform_overrides_tests::platform_overrides_custom_credential_collision_inherits_via_pipeline ... ok
test profile::platform_overrides_tests::platform_overrides_custom_credential_merge_exhaustive_over_every_field ... ok
test profile::platform_overrides_tests::platform_overrides_merge_adds_filesystem_paths ... ok
test profile::platform_overrides_tests::platform_overrides_merge_per_os_key_child_wins ... ok
test profile::platform_overrides_tests::platform_overrides_new_form_windows_low_il_broker_resolves_true ... ok
test profile::platform_overrides_tests::platform_overrides_on_base_survive_extends_resolution ... ok
test profile::platform_overrides_tests::platform_overrides_parses_and_serializes ... ok
test profile::platform_overrides_tests::platform_overrides_rejects_nested_extends ... ok
test profile::platform_overrides_tests::platform_overrides_rejects_nested_platform_overrides ... ok
test profile::platform_overrides_tests::platform_overrides_same_os_key_deep_merges_not_clobbers ... ok
test profile::platform_overrides_tests::platform_overrides_set_vars_validated_after_merge ... ok
test profile::platform_overrides_tests::platform_overrides_top_level_windows_low_il_broker_still_resolves_true ... ok
test profile::platform_overrides_tests::platform_overrides_valid_set_vars_merged_and_accepted ... ok
test profile::platform_overrides_tests::platform_overrides_validates_against_schema_alongside_existing_fields ... ok
test profile::platform_overrides_tests::platform_overrides_windows_interpreters_union_not_replace ... ok
test profile::platform_overrides_tests::platform_overrides_windows_low_il_broker_or_semantics_top_level_true_override_false_stays_true ... ok
test profile::platform_overrides_tests::port_ranges_parse_from_json ... ok
test profile::platform_overrides_tests::port_ranges_validate_against_schema ... ok
test profile::resolve_context_tests::load_profile_legacy_entry_uses_default_context ... ok
test profile::resolve_context_tests::load_profile_with_context_suppresses_auto_pull_when_flag_set ... ok
test profile::resolve_context_tests::resolve_context_default_does_not_suppress_auto_pull ... ok
test profile::resolve_context_tests::resolve_context_is_pattern_matchable ... ok
test profile::session_hooks_tests::test_merge_profiles_session_hooks_child_inherits_when_absent ... ok
test profile::session_hooks_tests::test_merge_profiles_session_hooks_child_overrides_per_field ... ok
test profile::session_hooks_tests::test_session_hooks_basic_deserialize ... ok
test profile::session_hooks_tests::test_session_hooks_rejects_unknown_field ... ok
test profile::session_hooks_tests::test_session_hooks_rejects_unknown_top_level_field ... ok
test profile::tests::aipc_capabilities_block_parses_valid_tokens ... ok
test profile::tests::aipc_default_inherits_when_block_absent ... ok
test profile::tests::aipc_unknown_token_rejected_at_parse_time ... ok
test profile::tests::built_in_profiles_load_with_aipc_block_present ... ok
test profile::tests::conditional_profile_entries_reject_unknown_fields ... ok
test profile::tests::custom_credential_aws_auth_rejected_unconditionally ... ok
test profile::tests::custom_credential_credential_key_and_auth_mutually_exclusive ... ok
test profile::tests::custom_credential_neither_key_nor_auth_rejected ... ok
test profile::tests::deserialize_custom_credentials_oauth2 ... ok
test profile::tests::deserialize_packs_and_command_args ... ok
test profile::tests::deserialize_packs_and_command_args_default_empty ... ok
test profile::tests::deserialize_seatbelt_rules_default_is_empty ... ok
test profile::tests::deserialize_seatbelt_rules_with_field ... ok
test profile::tests::draft_path_and_canonical_path_differ_in_parent ... ok
test profile::tests::get_user_profile_draft_base_path_ends_in_base ... ok
test profile::tests::get_user_profile_draft_path_ends_in_json ... ok
test profile::tests::merge_profiles_dedup_appends_packs_and_command_args ... ok
test profile::tests::merge_profiles_dedup_appends_seatbelt_rules ... ok
test profile::tests::oauth2_client_assertion_spiffe_still_validates_regression ... ok
test profile::tests::oauth2_empty_client_id_rejected ... ok
test profile::tests::oauth2_empty_client_secret_rejected ... ok
test profile::tests::oauth2_http_loopback_token_url_allowed ... ok
test profile::tests::oauth2_http_token_url_rejected ... ok
test profile::tests::oauth2_keystore_secret_resolves ... ok
test profile::tests::oauth2_plain_client_credentials_no_client_assertion_rejected ... ok
test profile::tests::plan_43_05_when_filters_filesystem_credentials_and_open_urls ... ok
test profile::tests::policy_json_validates_against_schema ... ok
test profile::tests::resolve_aipc_allowlist_widens_event_mask_with_signal_token ... ok
test profile::tests::resolve_aipc_allowlist_widens_job_object_with_terminate_token ... ok
test profile::tests::resolve_aipc_allowlist_widens_pipe_with_readwrite_token ... ok
test profile::tests::resolve_aipc_allowlist_widens_socket_role_with_bind_token ... ok
test profile::tests::test_credentials_deserialization_absent_vs_empty ... ok
test profile::tests::test_custom_credential_capture_composes_with_credential_key ... ok
test profile::tests::test_custom_credential_capture_only_is_valid ... ok
test profile::tests::test_custom_credential_invalid_capture_field_path_rejected ... ok
test profile::tests::test_custom_credential_none_of_credential_key_auth_aws_auth_spiffe_capture_rejected ... ok
test profile::tests::test_dedup_append_empty_vecs ... ok
test profile::tests::test_dedup_append_preserves_order ... ok
test profile::tests::test_empty_env_credentials_config ... ok
test profile::tests::test_env_credentials_config_parsing ... ok
test profile::tests::test_environment_config_allow_and_deny_vars_together ... ok
test profile::tests::test_environment_config_default ... ok
test profile::tests::test_environment_config_deny_unknown_fields ... ok
test profile::tests::test_environment_config_deny_vars_merge ... ok
test profile::tests::test_environment_config_deny_vars_merge_deduplicates ... ok
test profile::tests::test_environment_config_empty_allow_vars ... ok
test profile::tests::test_environment_config_with_allow_vars ... ok
test profile::tests::test_environment_config_with_deny_vars ... ok
test profile::tests::test_expand_vars ... ok
test profile::tests::test_expand_vars_uses_windows_home_and_appdata ... ok
test profile::tests::test_expand_vars_xdg_cache_home ... ok
test profile::tests::test_expand_vars_xdg_runtime_dir ... ok
test profile::tests::test_expand_vars_xdg_state_home ... ok
test profile::tests::test_extends_builtin_profile ... ok
test profile::tests::test_extends_can_clear_inherited_network_profile_with_null ... ok
test profile::tests::test_extends_chain_three_levels ... ok
test profile::tests::test_extends_circular_dependency_error ... ok
test profile::tests::test_extends_depth_limit_error ... ok
test profile::tests::test_extends_duplicate_base_deduplicates ... ok
test profile::tests::test_extends_empty_child_inherits_all ... ok
test profile::tests::test_extends_empty_string_in_array_rejected ... ok
test profile::tests::test_extends_field_deserialization ... ok
test profile::tests::test_extends_missing_base_error ... ok
test profile::tests::test_extends_multiple_bases ... ok
test profile::tests::test_extends_multiple_builtin_default ... ok
test profile::tests::test_extends_multiple_ordering ... ok
test profile::tests::test_extends_multiple_shared_transitive_base_deduplicates ... ok
test profile::tests::test_extends_resolves_sibling_in_same_directory ... ok
test profile::tests::test_extends_same_name_as_base_skips_self ... ok
test profile::tests::test_extends_same_name_still_resolves_other_siblings ... ok
test profile::tests::test_extends_self_reference_error ... ok
test profile::tests::test_extends_user_profile ... ok
test profile::tests::test_http_token_char_alphanumeric ... ok
test profile::tests::test_http_token_char_rejects_invalid ... ok
test profile::tests::test_http_token_char_special_chars ... ok
test profile::tests::test_list_profiles ... ok
test profile::tests::test_load_builtin_profile ... ok
test profile::tests::test_load_nonexistent_profile ... ok
test profile::tests::test_load_profile_extends_default_respects_excluded_groups ... ok
test profile::tests::test_load_profile_from_file_path ... ok
test profile::tests::test_load_profile_from_nonexistent_path ... ok
test profile::tests::test_merge_implicit_default_groups_into_user_profile ... ok
test profile::tests::test_merge_implicit_default_groups_respects_policy_exclude_groups ... ok
test profile::tests::test_merge_profiles_allow_launch_services_child_overrides_base ... ok
test profile::tests::test_merge_profiles_appends_filesystem_paths ... ok
test profile::tests::test_merge_profiles_appends_security_groups ... ok
test profile::tests::test_merge_profiles_credentials_empty_overrides_base ... ok
test profile::tests::test_merge_profiles_credentials_none_inherits_base ... ok
test profile::tests::test_merge_profiles_credentials_some_merges_with_base ... ok
test profile::tests::test_merge_profiles_custom_credentials_child_wins_on_collision ... ok
test profile::tests::test_merge_profiles_deduplicates_open_port ... ok
test profile::tests::test_merge_profiles_deduplicates_vecs ... ok
test profile::tests::test_merge_profiles_diagnostics_suppressions_append ... ok
test profile::tests::test_merge_profiles_env_credentials_child_wins ... ok
test profile::tests::test_merge_profiles_extends_consumed ... ok
test profile::tests::test_merge_profiles_inherits_linux_af_unix_mediation ... ok
test profile::tests::test_merge_profiles_inherits_network_block ... ok
test profile::tests::test_merge_profiles_interactive_or_semantics ... ok
test profile::tests::test_merge_profiles_merges_custom_credentials ... ok
test profile::tests::test_merge_profiles_merges_hooks ... ok
test profile::tests::test_merge_profiles_merges_policy_patches ... ok
test profile::tests::test_merge_profiles_network_profile_null_clears_base ... ok
test profile::tests::test_merge_profiles_network_profile_override ... ok
test profile::tests::test_merge_profiles_open_urls_child_absent_inherits_base ... ok
test profile::tests::test_merge_profiles_open_urls_child_narrows ... ok
test profile::tests::test_merge_profiles_open_urls_child_replaces_base ... ok
test profile::tests::test_merge_profiles_replaces_meta ... ok
test profile::tests::test_merge_profiles_workdir_inherit_from_base ... ok
test profile::tests::test_merge_profiles_workdir_override ... ok
test profile::tests::test_network_config_accepts_verb_noun_collection_aliases ... ok
test profile::tests::test_network_config_serializes_new_names ... ok
test profile::tests::test_network_profile_deserialization_distinguishes_absent_null_and_value ... ok
test profile::tests::test_policy_patch_deserialization ... ok
test profile::tests::test_profile_json_with_file_uri_custom_credential_parses ... ok
test profile::tests::test_profile_parses_linux_af_unix_mediation ... ok
test profile::tests::test_resolve_user_config_dir_uses_appdata ... ok
test profile::tests::test_schema_rejects_capture_empty_response_fields ... ok
test profile::tests::test_schema_rejects_capture_invalid_response_field_kind ... ok
test profile::tests::test_schema_rejects_extends_array_of_non_strings ... ok
test profile::tests::test_schema_rejects_extends_empty_array ... ok
test profile::tests::test_schema_rejects_extends_numeric ... ok
test profile::tests::test_schema_rejects_spiffe_missing_workload_api_socket ... ok
test profile::tests::test_schema_self_is_valid_json ... ok
test profile::tests::test_schema_validates_absent_extends ... ok
test profile::tests::test_schema_validates_builtin_profiles_in_policy_json ... ok
test profile::tests::test_schema_validates_capture_custom_credential ... ok
test profile::tests::test_schema_validates_extends_as_array ... ok
test profile::tests::test_schema_validates_extends_as_string ... ok
test profile::tests::test_schema_validates_extends_single_element_array ... ok
test profile::tests::test_schema_validates_full_profile ... ok
test profile::tests::test_schema_validates_oauth2_client_assertion ... ok
test profile::tests::test_schema_validates_spiffe_custom_credential ... ok
test profile::tests::test_secrets_alias_backward_compat ... ok
test profile::tests::test_security_config_allowed_commands_defaults_empty ... ok
test profile::tests::test_security_config_allowed_commands_deserializes ... ok
test profile::tests::test_security_config_ipc_mode_defaults_none ... ok
test profile::tests::test_security_config_ipc_mode_full_deserializes ... ok
test profile::tests::test_security_config_ipc_mode_shared_memory_only ... ok
test profile::tests::test_security_config_process_info_mode_allow_all ... ok
test profile::tests::test_security_config_process_info_mode_defaults_none ... ok
test profile::tests::test_security_config_process_info_mode_deserializes ... ok
test profile::tests::test_signal_mode_allow_same_sandbox_deserializes ... ok
test profile::tests::test_top_level_schema_field_allowed_in_profile ... ok
test profile::tests::test_unknown_fields_rejected_in_profile ... ok
test profile::tests::test_unknown_fields_rejected_in_top_level_profile ... ok
test profile::tests::test_valid_profile_names ... ok
test profile::tests::test_validate_basic_auth_mode_valid ... ok
test profile::tests::test_validate_custom_credential_empty_header_rejected ... ok
test profile::tests::test_validate_custom_credential_env_uri_accepted ... ok
test profile::tests::test_validate_custom_credential_env_uri_dangerous_var_rejected ... ok
test profile::tests::test_validate_custom_credential_file_uri_accepted ... ok
test profile::tests::test_validate_custom_credential_file_uri_invalid_rejected ... ok
test profile::tests::test_validate_custom_credential_file_uri_requires_env_var ... ok
test profile::tests::test_validate_custom_credential_file_uri_traversal_rejected ... ok
test profile::tests::test_validate_custom_credential_format_with_cr_rejected ... ok
test profile::tests::test_validate_custom_credential_format_with_lf_rejected ... ok
test profile::tests::test_validate_custom_credential_header_with_colon_rejected ... ok
test profile::tests::test_validate_custom_credential_header_with_space_rejected ... ok
test profile::tests::test_validate_custom_credential_http_0_0_0_0_rejected ... ok
test profile::tests::test_validate_custom_credential_http_ipv6_loopback_allowed ... ok
test profile::tests::test_validate_custom_credential_http_ipv6_unspecified_rejected ... ok
test profile::tests::test_validate_custom_credential_http_localhost_allowed ... ok
test profile::tests::test_validate_custom_credential_http_loopback_allowed ... ok
test profile::tests::test_validate_custom_credential_http_remote_rejected ... ok
test profile::tests::test_validate_custom_credential_invalid_format_rejected ... ok
test profile::tests::test_validate_custom_credential_invalid_header_rejected ... ok
test profile::tests::test_validate_custom_credential_invalid_key_rejected ... ok
test profile::tests::test_validate_custom_credential_none_of_credential_key_auth_aws_auth_spiffe_rejected ... ok
test profile::tests::test_validate_custom_credential_spiffe ... ok
test profile::tests::test_validate_custom_credential_spiffe_empty_inject_header_rejected ... ok
test profile::tests::test_validate_custom_credential_spiffe_with_aws_auth_rejected ... ok
test profile::tests::test_validate_custom_credential_spiffe_with_credential_key_rejected ... ok
test profile::tests::test_validate_custom_credential_valid ... ok
test profile::tests::test_validate_custom_credential_valid_special_header_chars ... ok
test profile::tests::test_validate_custom_credential_various_valid_formats ... ok
test profile::tests::test_validate_env_credentials_accepts_apple_password_uri ... ok
test profile::tests::test_validate_env_credentials_accepts_file_uri ... ok
test profile::tests::test_validate_env_credentials_rejects_invalid_apple_password_uri ... ok
test profile::tests::test_validate_env_credentials_rejects_invalid_file_uri ... ok
test profile::tests::test_validate_env_var_empty_rejected ... ok
test profile::tests::test_validate_env_var_invalid_chars_rejected ... ok
test profile::tests::test_validate_env_var_optional_for_keyring_keys ... ok
test profile::tests::test_validate_env_var_with_apple_password_uri_and_env_var_ok ... ok
test profile::tests::test_validate_env_var_with_apple_password_uri_requires_env_var ... ok
test profile::tests::test_validate_env_var_with_keyring_key_ok ... ok
test profile::tests::test_validate_env_var_with_op_uri_and_env_var_ok ... ok
test profile::tests::test_validate_env_var_with_op_uri_requires_env_var ... ok
test profile::tests::test_validate_oauth2_auth_client_assertion_valid ... ok
test profile::tests::test_validate_oauth2_auth_neither_client_assertion_nor_client_id_secret_rejected ... ok
test profile::tests::test_validate_query_param_mode_empty_param_name ... ok
test profile::tests::test_validate_query_param_mode_missing_param_name ... ok
test profile::tests::test_validate_query_param_mode_valid ... ok
test profile::tests::test_validate_url_path_mode_missing_pattern ... ok
test profile::tests::test_validate_url_path_mode_pattern_without_placeholder ... ok
test profile::tests::test_validate_url_path_mode_replacement_without_placeholder ... ok
test profile::tests::test_validate_url_path_mode_valid ... ok
test profile::tests::test_validate_url_path_mode_with_replacement ... ok
test profile::tests::test_workdir_config_default ... ok
test profile::tests::test_workdir_config_none ... ok
test profile::tests::test_workdir_config_read ... ok
test profile::tests::test_workdir_config_readwrite ... ok
test profile::windows_low_il_broker_tests::bun_dev_builtin_profile_resolves_and_carries_bun_runtime_group ... ok
test profile::windows_low_il_broker_tests::claude_code_builtin_profile_has_windows_low_il_broker_true ... ok
test profile::windows_low_il_broker_tests::codex_builtin_profile_does_not_have_windows_low_il_broker ... ok
test profile::windows_low_il_broker_tests::jsonc_comments_and_trailing_commas ... ok
test profile::windows_low_il_broker_tests::jsonc_resolve_prefers_jsonc_extension ... ok
test profile::windows_low_il_broker_tests::merge_profiles_or_semantics_base_false_child_true ... ok
test profile::windows_low_il_broker_tests::merge_profiles_or_semantics_base_true_child_false ... ok
test profile::windows_low_il_broker_tests::merge_profiles_or_semantics_both_false ... ok
test profile::windows_low_il_broker_tests::mise_dev_builtin_profile_resolves_and_carries_mise_manager_group ... ok
test profile::windows_low_il_broker_tests::profile_binary_field_parses_and_inherits ... ok
test profile::windows_low_il_broker_tests::set_vars_parses_valid_keys ... ok
test profile::windows_low_il_broker_tests::set_vars_rejects_reserved_nono_prefix ... ok
test profile::windows_low_il_broker_tests::set_vars_rejects_reserved_path_key ... ok
test profile::windows_low_il_broker_tests::windows_low_il_broker_default_is_false ... ok
test profile::windows_low_il_broker_tests::windows_low_il_broker_deserializes_without_unknown_field_error ... ok
test profile::windows_low_il_broker_tests::windows_low_il_broker_false_deserializes_correctly ... ok
test profile_cmd::tests::atomic_write_file_creates_target ... ok
test profile_cmd::tests::atomic_write_file_overwrites_existing ... ok
test profile_cmd::tests::atomic_write_file_uses_pid_suffix ... ok
test profile_cmd::tests::embedded_guide_contains_no_nono_policy_references ... ok
test profile_cmd::tests::network_section_shows_for_a_range_only_profile ... ok
test profile_cmd::tests::read_regular_file_reads_normal_file ... ok
test profile_cmd::tests::read_regular_file_rejects_directory ... ok
test profile_cmd::tests::read_regular_file_rejects_missing ... ok
test profile_cmd::tests::regular_file_exists_true_for_regular ... ok
test profile_cmd::tests::sha256_hex_of_known_bytes ... ok
test profile_cmd::tests::sha256_hex_produces_lowercase_hex ... ok
test profile_cmd::tests::test_diff_shows_differences ... ok
test profile_cmd::tests::test_force_overwrite ... ok
test profile_cmd::tests::test_full_skeleton_is_valid_profile ... ok
test profile_cmd::tests::test_full_vs_minimal_differences ... ok
test profile_cmd::tests::test_groups_lists_all ... ok
test profile_cmd::tests::test_groups_specific_known ... ok
test profile_cmd::tests::test_groups_unknown_errors ... ok
test profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name ... FAILED
test profile_cmd::tests::test_init_blocked_when_shadowing_builtin ... ok
test profile_cmd::tests::test_init_blocked_with_custom_output_when_shadowing_builtin ... ok
test profile_cmd::tests::test_invalid_extends_target ... ok
test profile_cmd::tests::test_invalid_group_name ... ok
test profile_cmd::tests::test_invalid_profile_name ... ok
test profile_cmd::tests::test_minimal_skeleton_is_valid_profile ... ok
test profile_cmd::tests::test_profiles_includes_builtins ... ok
test profile_cmd::tests::test_show_resolves_inheritance ... ok
test profile_cmd::tests::test_skeleton_omits_schema_url ... ok
test profile_cmd::tests::test_skeleton_with_groups ... ok
test profile_cmd::tests::test_validate_exclude_required ... ok
test profile_cmd::tests::test_validate_invalid_group ... ok
test profile_cmd::tests::test_validate_valid_profile ... ok
test profile_cmd::tests::verify_base_hash_match ... ok
test profile_cmd::tests::verify_base_hash_mismatch_returns_action_required ... ok
test profile_cmd::tests::verify_base_hash_rejects_invalid_hex ... ok
test profile_runtime::tests::absent_environment_block_returns_none ... ok
test profile_runtime::tests::build_listen_ports_rejects_range_starting_at_zero ... ok
test profile_runtime::tests::build_listen_ports_unrolls_range_alongside_discrete_ports ... ok
test profile_runtime::tests::check_macos_port_range_cap_accepts_exactly_at_limit ... ok
test profile_runtime::tests::check_macos_port_range_cap_rejects_over_limit_combined_ranges ... ok
test profile_runtime::tests::empty_allow_vars_fails_closed ... ok
test profile_runtime::tests::expand_profile_set_vars_none_when_absent ... ok
test profile_runtime::tests::validate_port_ranges_rejects_reversed_listen_port_range ... ok
test profile_runtime::tests::validate_port_ranges_rejects_reversed_open_port_range ... ok
test profile_runtime::tests::verify_profile_packs_requires_lockfile_entry_for_installed_pack ... ok
test profile_runtime::tests::verify_profile_packs_requires_trust_bundle_for_locked_pack ... ok
test protected_paths::tests::allows_unrelated_capability ... ok
test protected_paths::tests::blocks_child_directory_capability ... FAILED
test protected_paths::tests::blocks_parent_directory_capability ... FAILED
test protected_paths::tests::protected_roots_include_current_windows_state_dir ... ok
test protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root ... FAILED
test protected_paths::tests::windows_protected_path_check_handles_verbatim_prefix_and_case_insensitive_drive_letters ... ok
test provision_windows::tests::test_provision_idempotent_second_call ... ok
test provision_windows::tests::test_scratch_dir_fails_without_localappdata ... ok
test provision_windows::tests::test_scratch_dir_under_localappdata ... ok
test proxy_command::tests::allow_remote_does_not_unlock_no_auth_off_loopback ... ok
test proxy_command::tests::audit_sink_appends_events_as_json_lines ... ok
test proxy_command::tests::audit_sink_open_fails_closed_on_unwritable_path ... ok
test proxy_command::tests::audit_sink_without_path_is_a_tracing_only_sink ... ok
test proxy_command::tests::cli_deny_domain_flag_composes_with_profile_deny_domain ... ok
test proxy_command::tests::cli_deny_domain_without_allow_domain_is_rejected ... ok
test proxy_command::tests::non_loopback_listen_requires_allow_remote ... ok
test proxy_command::tests::plain_profile_leaves_deny_layer_empty_and_strict_filter_off ... ok
test proxy_command::tests::profile_deny_domain_without_allow_domain_is_rejected ... ok
test proxy_command::tests::profile_deny_layer_no_proxy_and_block_reach_launch_options ... ok
test proxy_runtime::tests::block_net_overrides_custom_credentials_activation ... ok
test proxy_runtime::tests::build_proxy_config_maps_upstream_proxy_to_external_proxy ... ok
test proxy_runtime::tests::build_proxy_config_propagates_deny_domain_with_allow_domain ... ok
test proxy_runtime::tests::build_proxy_config_propagates_non_overlapping_no_proxy ... ok
test proxy_runtime::tests::build_proxy_config_rejects_literal_no_proxy_allow_domain_overlap ... ok
test proxy_runtime::tests::deny_domain_with_allow_domain_activates_proxy ... ok
test proxy_runtime::tests::deny_only_state_is_rejected_before_reaching_prepare_proxy_launch_options ... ok
test proxy_runtime::tests::parse_allow_domain_host_port_443_yields_plain ... ok
test proxy_runtime::tests::parse_allow_domain_host_port_yields_plain ... ok
test proxy_runtime::tests::parse_allow_domain_plain_hostname ... ok
test proxy_runtime::tests::parse_allow_domain_unparseable_input_falls_back_to_plain ... ok
test proxy_runtime::tests::parse_allow_domain_url_no_path_produces_plain ... ok
test proxy_runtime::tests::parse_allow_domain_url_with_path_produces_with_endpoints ... ok
test proxy_runtime::tests::parse_allow_domain_url_with_root_path_produces_plain ... ok
test proxy_runtime::tests::proxy_activates_with_custom_credentials_only ... ok
test proxy_runtime::tests::resolve_effective_proxy_settings_preserves_with_endpoints ... ok
test proxy_runtime::tests::test_allow_endpoint_applied_to_credential_route ... ok
test proxy_runtime::tests::test_allow_endpoint_does_not_affect_other_routes ... ok
test proxy_runtime::tests::test_allow_endpoint_no_credential_errors ... ok
test proxy_runtime::tests::test_allow_endpoint_unknown_service_errors ... ok
test proxy_runtime::tests::test_build_proxy_config_propagates_strict_filter ... ok
test proxy_runtime::tests::test_build_proxy_config_rejects_group_expanded_no_proxy_overlap ... ok
test proxy_runtime::tests::test_build_proxy_config_strict_filter_off_when_no_block ... ok
test proxy_runtime::tests::test_compiled_endpoint_policy_compat_deviation_preserved ... ok
test proxy_runtime::tests::test_enforce_spiffe_socket_isolation_hard_errors_on_conflicting_grant ... ok
test proxy_runtime::tests::test_enforce_spiffe_socket_isolation_never_grants_the_socket ... ok
test proxy_runtime::tests::test_enforce_spiffe_socket_isolation_noop_when_no_spiffe_routes ... ok
test proxy_runtime::tests::test_parse_allow_endpoint_arg_missing_parts ... ok
test proxy_runtime::tests::test_parse_allow_endpoint_arg_path_must_start_with_slash ... ok
test proxy_runtime::tests::test_parse_allow_endpoint_arg_valid ... ok
test proxy_runtime::tests::test_parse_allow_endpoint_arg_wildcard_method ... ok
test query_ext::tests::test_parse_host_input_bare_hostname ... ok
test query_ext::tests::test_parse_host_input_url ... ok
test query_ext::tests::test_parse_host_input_url_root_path ... ok
test query_ext::tests::test_path_matches_empty_rules_allows_all ... ok
test query_ext::tests::test_path_matches_endpoint_rules_glob ... ok
test query_ext::tests::test_query_network_allowed ... ok
test query_ext::tests::test_query_network_bare_domain_with_endpoint_rules_shows_allowed ... ok
test query_ext::tests::test_query_network_blocked ... ok
test query_ext::tests::test_query_network_proxy_denies_cloud_metadata ... ok
test query_ext::tests::test_query_network_proxy_domain_filtering ... ok
test query_ext::tests::test_query_network_proxy_no_domain_filter ... ok
test query_ext::tests::test_query_network_proxy_wildcard_and_bare_domain ... ok
test query_ext::tests::test_query_network_url_extracts_domain ... ok
test query_ext::tests::test_query_network_url_with_endpoint_rules_path_denied ... ok
test query_ext::tests::test_query_network_url_with_endpoint_rules_path_matches ... ok
test query_ext::tests::test_query_path_denied ... ok
test query_ext::tests::test_query_path_granted ... ok
test query_ext::tests::test_query_path_prefers_more_specific_sufficient_capability ... ok
test query_ext::tests::test_query_path_reports_near_miss_with_source_and_fix ... ok
test query_ext::tests::test_query_path_sensitive_detected_despite_unc_canonicalization ... ok
test query_ext::tests::test_query_path_sensitive_policy_includes_policy_source ... ok
test query_ext::tests::test_query_scope_returns_structured_result ... ok
test registry_client::tests::download_artifact_to_path_computes_digest_of_streamed_bytes ... ok
test registry_client::tests::download_artifact_to_path_rejects_oversize_via_content_length ... ok
test registry_client::tests::enforce_content_length_passes_at_boundary ... ok
test registry_client::tests::enforce_content_length_passes_when_header_absent ... ok
test registry_client::tests::enforce_content_length_rejects_oversize ... ok
test registry_client::tests::fetch_package_status_url_encodes_installed ... ok
test registry_client::tests::registry_client_connect_timeout_fires_within_bounded_window ... ok
test registry_client::tests::registry_client_constructor_succeeds ... ok
test registry_client::tests::registry_client_normalizes_base_url ... ok
test registry_client::tests::tempdir_cleanup_runs_on_panic ... ok
test rollback_commands::tests::count_change_types_empty ... ok
test rollback_commands::tests::count_change_types_mixed ... ok
test rollback_commands::tests::format_absolute_timestamp_different_year ... ok
test rollback_commands::tests::format_absolute_timestamp_same_year ... ok
test rollback_commands::tests::format_change_summary_output ... ok
test rollback_commands::tests::format_session_timestamp_days_ago ... ok
test rollback_commands::tests::format_session_timestamp_different_year ... ok
test rollback_commands::tests::format_session_timestamp_epoch_seconds ... ok
test rollback_commands::tests::format_session_timestamp_future ... ok
test rollback_commands::tests::format_session_timestamp_hours_ago ... ok
test rollback_commands::tests::format_session_timestamp_invalid_string ... ok
test rollback_commands::tests::format_session_timestamp_just_now ... ok
test rollback_commands::tests::format_session_timestamp_minutes_ago ... ok
test rollback_commands::tests::format_session_timestamp_older_same_year ... ok
test rollback_commands::tests::group_by_project_multiple_paths ... ok
test rollback_commands::tests::group_by_project_single_path ... ok
test rollback_commands::tests::rollback_list_output_format_structure ... ok
test rollback_commands::tests::shorten_home_replaces_prefix ... ok
test rollback_commands::tests::windows_path_overlaps_filter_handles_verbatim_prefix_and_drive_case ... ok
test rollback_preflight::tests::detect_heavy_dirs_finds_known_names ... ok
test rollback_preflight::tests::detect_heavy_dirs_skips_already_excluded ... ok
test rollback_preflight::tests::empty_result_does_not_need_warning ... ok
test rollback_preflight::tests::heavy_dir_fields_accessible ... ok
test rollback_preflight::tests::preflight_empty_tracked_dir_no_warning ... ok
test rollback_preflight::tests::preflight_nonexistent_path_no_warning ... ok
test rollback_preflight::tests::preflight_result_fields ... ok
test rollback_runtime::tests::partial_restore_error_converts_to_nono_error ... ok
test rollback_runtime::tests::partial_restore_error_counts ... ok
test rollback_runtime::tests::partial_restore_error_display_names_failed_paths ... ok
test rollback_runtime::tests::plain_run_creates_audit_session_default_on_no_audit_opts_out ... ok
test rollback_runtime::tests::rollback_status_failed_warning_only_records_reason ... ok
test rollback_runtime::tests::rollback_status_skipped_when_rollback_disabled ... ok
test rollback_session::tests::calculate_dir_size_works ... ok
test rollback_session::tests::dead_process_not_alive ... ok
test rollback_session::tests::discover_sessions_empty_dir ... ok
test rollback_session::tests::discover_sessions_reads_legacy_rollback_root ... ok
test rollback_session::tests::format_bytes_display ... ok
test rollback_session::tests::is_current_process_alive ... ok
test rollback_session::tests::parse_pid_from_session_id_invalid ... ok
test rollback_session::tests::parse_pid_from_session_id_valid ... ok
test rollback_session::tests::rollback_root_uses_windows_state_dir ... ok
test rollback_session::tests::validate_session_id_accepts_valid ... ok
test rollback_session::tests::validate_session_id_rejects_traversal ... ok
test sandbox_prepare::tests::allow_endpoint_with_credential_is_valid ... ok
test sandbox_prepare::tests::allow_endpoint_without_credential_errors ... ok
test sandbox_prepare::tests::block_net_alone_is_valid ... ok
test sandbox_prepare::tests::block_net_with_allow_domain_errors ... ok
test sandbox_prepare::tests::block_net_with_credential_errors ... ok
test sandbox_prepare::tests::block_net_with_network_profile_errors ... ok
test sandbox_prepare::tests::deny_domain_from_profile_with_allow_domain_from_profile_is_valid ... ok
test sandbox_prepare::tests::deny_domain_from_profile_without_allow_domain_errors ... ok
test sandbox_prepare::tests::deny_domain_with_allow_domain_is_valid ... ok
test sandbox_prepare::tests::deny_domain_without_allow_domain_errors ... ok
test sandbox_prepare::tests::missing_directory_cli_grants_are_reported_as_skipped ... ok
test sandbox_prepare::tests::no_deny_domain_is_always_valid ... ok
test sandbox_prepare::tests::profile_network_block_with_credential_from_profile_errors ... ok
test sandbox_prepare::tests::proxy_port_with_credential_is_valid ... ok
test sandbox_prepare::tests::proxy_port_without_proxy_intent_errors ... ok
test sandbox_state::domain_endpoint_state_tests::domain_endpoint_state_serializes_and_deserializes ... ok
test sandbox_state::domain_endpoint_state_tests::from_caps_four_arg_populates_domain_endpoints ... ok
test sandbox_state::domain_endpoint_state_tests::from_caps_four_arg_signature_with_empty_domain_endpoints ... ok
test sandbox_state::domain_endpoint_state_tests::sandbox_state_domain_endpoints_backwards_compat_deserialize ... ok
test sandbox_state::domain_endpoint_state_tests::sandbox_state_domain_endpoints_field_skips_when_empty_in_json ... ok
test sandbox_state::tests::test_sandbox_state_roundtrip ... ok
test sandbox_state::tests::test_sandbox_state_roundtrip_preserves_source ... ok
test sandbox_state::tests::test_sandbox_state_write_and_read ... ok
test sandbox_state::tests::test_validate_cap_file_path_accepts_windows_runtime_temp_dir ... ok
test session::resource_limits_record_tests::from_resource_limits_maps_all_four ... ok
test session::resource_limits_record_tests::from_resource_limits_maps_cpu_only ... ok
test session::resource_limits_record_tests::from_resource_limits_maps_max_processes ... ok
test session::resource_limits_record_tests::from_resource_limits_maps_memory_only ... ok
test session::resource_limits_record_tests::from_resource_limits_maps_timeout_to_seconds ... ok
test session::resource_limits_record_tests::from_resource_limits_returns_none_when_empty ... ok
test session::resource_limits_record_tests::record_round_trip_preserves_set_fields ... ok
test session::resource_limits_record_tests::record_serialization_omits_unset_fields ... ok
test session::resource_limits_record_tests::session_record_deserializes_with_empty_limits_object ... ok
test session::resource_limits_record_tests::session_record_deserializes_with_populated_limits ... ok
test session::resource_limits_record_tests::session_record_deserializes_without_limits_field ... ok
test session::tests::is_prunable_all_exited_escape_hatch_matches_any_exited ... ok
test session::tests::is_prunable_at_exact_boundary ... ok
test session::tests::is_prunable_exited_older_than_retention_is_true ... ok
test session::tests::is_prunable_exited_within_retention_is_false ... ok
test session::tests::is_prunable_future_started_epoch_fails_closed ... ok
test session::tests::is_prunable_one_second_under_boundary_is_false ... ok
test session::tests::is_prunable_paused_is_never_true ... ok
test session::tests::is_prunable_running_is_never_true_even_if_ancient ... ok
test session::tests::sessions_dir_uses_localappdata_on_windows ... ok
test session::tests::test_generate_session_id_length ... ok
test session::tests::test_load_session_prefix_match ... ok
test session::tests::test_pid_recycling_dead_pid ... ok
test session::tests::test_process_matches_session_accepts_matching_start_time_on_eperm ... ok
test session::tests::test_process_matches_session_requires_start_time_on_eperm ... ok
test session::tests::test_process_matches_session_requires_start_time_when_accessible ... ok
test session::tests::test_session_guard_drop_marks_exited ... ok
test session::tests::test_session_record_roundtrip ... ok
test session::tests::test_session_status_serde ... ok
test session::tests::test_update_session_file ... ok
test session::tests::test_write_and_load_session_file ... ok
test session_commands::attach_busy_translation_tests::passes_through_arbitrary_io_errors ... ok
test session_commands::attach_busy_translation_tests::passes_through_other_errors ... ok
test session_commands::attach_busy_translation_tests::translates_pipe_busy_to_friendly_setup ... ok
test session_commands::inspect_formatting_tests::bytes_1_gib ... ok
test session_commands::inspect_formatting_tests::bytes_1_tib ... ok
test session_commands::inspect_formatting_tests::bytes_256_kib ... ok
test session_commands::inspect_formatting_tests::bytes_512_mib ... ok
test session_commands::inspect_formatting_tests::bytes_non_clean_multiple_falls_back_to_bytes ... ok
test session_commands::inspect_formatting_tests::bytes_zero_renders_as_zero_bytes ... ok
test session_commands::inspect_formatting_tests::duration_1_day_is_singular ... ok
test session_commands::inspect_formatting_tests::duration_1_hour_is_singular ... ok
test session_commands::inspect_formatting_tests::duration_1_minute_is_singular ... ok
test session_commands::inspect_formatting_tests::duration_1_second_is_singular ... ok
test session_commands::inspect_formatting_tests::duration_2_hours ... ok
test session_commands::inspect_formatting_tests::duration_45_seconds ... ok
test session_commands::inspect_formatting_tests::duration_5_minutes ... ok
test session_commands::inspect_formatting_tests::duration_90s_not_clean_minute ... ok
test session_commands::limits_block_format_tests::format_bytes_short_100_mebibytes_is_100m ... ok
test session_commands::limits_block_format_tests::format_bytes_short_1024_bytes_is_1k ... ok
test session_commands::limits_block_format_tests::format_bytes_short_1_gibibyte_is_1g ... ok
test session_commands::limits_block_format_tests::format_bytes_short_non_round_value_falls_back_to_bytes ... ok
test session_commands::limits_block_format_tests::limits_block_empty_returns_empty_string ... ok
test session_commands::limits_block_format_tests::limits_block_format_windows_retains_legacy_cpu_string ... ok
test session_commands::limits_block_format_tests::limits_block_format_windows_retains_legacy_memory_string ... ok
test session_commands::limits_block_format_tests::limits_block_format_windows_retains_legacy_procs_string ... ok
test session_commands::tests::auto_prune_is_noop_when_sandboxed ... ok
test setup::grant_ancestors_tests::grant_ancestors_idempotent ... ok
test setup::grant_ancestors_tests::grant_ancestors_non_destructive ... ok
test setup::tests::test_setup_profiles_loadable_by_name ... ok
test setup::tests::windows_storage_layout_uses_absolute_user_roots ... ok
test setup::windows_check_only_tests::check_only_summary_does_not_mention_intentionally_unavailable ... ok
test setup::windows_check_only_tests::check_only_summary_includes_canonical_wrap_availability_sentence ... ok
test setup::windows_check_only_tests::check_only_summary_is_internally_consistent_about_wrap ... ok
test startup_runtime::tests::startup_log_summary_collapses_exec_failure_path_details ... ok
test startup_runtime::tests::startup_log_summary_extracts_error_after_applying_sandbox_prefix ... ok
test startup_runtime::tests::startup_log_summary_prefers_exit_headline_over_footer_hints ... ok
test startup_runtime::tests::startup_log_summary_preserves_real_error_lines ... ok
test startup_runtime::tests::startup_log_summary_skips_capability_rows_in_detached_failures ... ok
test startup_runtime::tests::startup_log_summary_skips_network_badge_rows_in_detached_failures ... ok
test state_paths::tests::audit_discovery_roots_lists_primary_before_legacy ... ok
test state_paths::tests::protected_roots_include_both_legacy_and_xdg ... ok
test state_paths::tests::rollback_discovery_roots_lists_primary_before_legacy ... ok
test supervised_runtime::tests::interactive_sessions_always_allocate_pty ... ok
test supervised_runtime::tests::non_detached_non_interactive_never_allocates_pty ... ok
test supervised_runtime::tests::windows_detached_supervisor_does_not_allocate_pty ... ok
test supervised_runtime::tests::windows_low_il_broker_interactive_skips_pty ... ok
test supervised_runtime::tests::windows_non_low_il_interactive_still_allocates_pty ... ok
test telemetry::event::tests::classify_aws_path_is_credential ... ok
test telemetry::event::tests::classify_etc_is_system_path ... ok
test telemetry::event::tests::classify_keystore_path_is_credential ... ok
test telemetry::event::tests::classify_project_file_is_workspace ... ok
test telemetry::event::tests::classify_ssh_path_is_credential ... ok
test telemetry::event::tests::classify_system32_is_system_path ... ok
test telemetry::event::tests::classify_temp_is_temp ... ok
test telemetry::event::tests::classify_tmp_is_temp ... ok
test telemetry::event::tests::downgraded_layers_field_round_trips_through_serde ... ok
test telemetry::event::tests::downgraded_layers_omitted_when_none ... ok
test telemetry::event::tests::event_id_for_maps_all_five_types ... ok
test telemetry::event::tests::layer_attestation_downgraded_event_id_is_10011 ... ok
test telemetry::event::tests::override_event_ids_are_10006_through_10010 ... ok
test telemetry::event::tests::override_event_type_serde_roundtrip ... ok
test telemetry::event::tests::path_hash_differs_for_different_paths ... ok
test telemetry::event::tests::path_hash_differs_for_different_salts ... ok
test telemetry::event::tests::path_hash_for_does_not_contain_raw_path ... ok
test telemetry::event::tests::path_hash_for_is_deterministic ... ok
test telemetry::event::tests::security_event_serializes_with_pascal_case_sc1_fields ... ok
test telemetry::event::tests::security_event_type_serde_round_trip ... ok
test telemetry::tests::advance_and_emit_holds_lock_across_full_build_advance_emit_sequence ... ok
test telemetry::tests::advance_chain_changes_head_and_increments_sequence ... ok
test telemetry::tests::advance_chain_uses_hmac_not_sha2_placeholder ... ok
test telemetry::tests::advance_chain_uses_key_in_hash ... ok
test telemetry::tests::chain_head_hex_is_64_chars ... ok
test telemetry::tests::chain_sequence_genesis_is_zero ... ok
test telemetry::tests::chain_state_key_is_zeroizing_type ... ok
test telemetry::tests::emit_override_event_advances_chain_by_one ... ok
test telemetry::tests::emit_override_event_err_on_poisoned_mutex ... ok
test telemetry::tests::emit_override_event_none_zt_audit_hash_ok ... ok
test telemetry::tests::emit_override_event_two_calls_advance_by_two ... ok
test telemetry::tests::min_severity_filter_predicate_matches_policy_threshold ... ok
test telemetry::tests::new_produces_nonzero_key_and_salt ... ok
test telemetry::tests::poison_for_test_poisons_mutex_for_emit_override_event ... ok
test telemetry::tests::severity_for_all_denial_types_is_warning ... ok
test telemetry::tests::severity_for_override_lifecycle_events_is_warning ... ok
test telemetry::tests::telemetry_domains_differ_from_audit_domains ... ok
test telemetry::tests::telemetry_event_domain_value ... ok
test telemetry::tests::two_events_produce_different_chain_heads ... ok
test telemetry::windows::tests::emit_security_event_is_non_fatal ... ok
test telemetry::windows::tests::event_id_constants_are_correct ... ok
test telemetry::windows::tests::event_log_source_is_nono ... ok
test terminal_approval::tests::build_prompt_text_event_kind ... ok
test terminal_approval::tests::build_prompt_text_file_kind_preserves_legacy_block ... ok
test terminal_approval::tests::build_prompt_text_job_object_kind ... ok
test terminal_approval::tests::build_prompt_text_mutex_kind ... ok
test terminal_approval::tests::build_prompt_text_pipe_kind ... ok
test terminal_approval::tests::build_prompt_text_socket_kind ... ok
test terminal_approval::tests::format_capability_prompt_event_kind ... ok
test terminal_approval::tests::format_capability_prompt_file_kind ... ok
test terminal_approval::tests::format_capability_prompt_job_object_kind ... ok
test terminal_approval::tests::format_capability_prompt_job_object_kind_renders_terminate_widening ... ok
test terminal_approval::tests::format_capability_prompt_mutex_kind ... ok
test terminal_approval::tests::format_capability_prompt_pipe_kind ... ok
test terminal_approval::tests::format_capability_prompt_socket_kind_connect ... ok
test terminal_approval::tests::prompt_renders_kind_target_mismatch_safely ... ok
test terminal_approval::tests::prompt_sanitizes_socket_host_string ... ok
test terminal_approval::tests::prompt_sanitizes_untrusted_target_strings ... ok
test terminal_approval::tests::sanitize_for_terminal_strips_ansi ... ok
test terminal_approval::tests::test_format_access_mode ... ok
test terminal_approval::tests::test_sanitize_all_control_chars_replaced ... ok
test terminal_approval::tests::test_sanitize_ansi_escape_csi ... ok
test terminal_approval::tests::test_sanitize_ansi_escape_osc ... ok
test terminal_approval::tests::test_sanitize_apc_sequence ... ok
test terminal_approval::tests::test_sanitize_carriage_return_overwrite ... ok
test terminal_approval::tests::test_sanitize_clean_input ... ok
test terminal_approval::tests::test_sanitize_dcs_sequence ... ok
test terminal_approval::tests::test_sanitize_null_bytes ... ok
test terminal_approval::tests::test_sanitize_pm_sequence ... ok
test terminal_approval::tests::test_sanitize_sos_sequence ... ok
test terminal_approval::tests::test_sanitize_unterminated_csi ... ok
test terminal_approval::tests::test_terminal_approval_backend_name ... ok
test terminal_approval::tests::windows_no_console_denies_gracefully ... ok
test tests::render_error_for_operator_adds_remediation_for_layer_attestation_failed ... ok
test tests::render_error_for_operator_is_unchanged_for_other_variants ... ok
test tests::render_error_for_operator_names_the_event_log_for_non_label_layers ... ok
test tests::test_check_blocked_command_allow_override ... ok
test tests::test_check_blocked_command_basic ... ok
test tests::test_check_blocked_command_extra_blocked ... ok
test tests::test_check_blocked_command_uses_resolved_policy_only ... ok
test tests::test_check_blocked_command_with_path ... ok
test tests::test_dangerous_commands_defined ... ok
test tests::test_execution_start_dir_falls_back_to_root_when_not_covered ... ok
test tests::test_execution_start_dir_keeps_workdir_when_covered ... ok
test tests::test_pre_exec_update_check_disabled_for_completions ... ok
test tests::test_pre_exec_update_check_disabled_for_execution_commands ... ok
test tests::test_pre_exec_update_check_disabled_for_pack_update_hint_helper ... ok
test tests::test_pre_exec_update_check_enabled_for_non_exec_commands ... ok
test tests::test_resolve_effective_proxy_settings_allow_net_clears_profile_proxy_state ... ok
test tests::test_resolve_effective_proxy_settings_merges_cli_and_profile ... ok
test tests::test_resolve_requested_workdir_prefers_explicit_path ... ok
test tests::test_select_exec_strategy_uses_supervised_for_capability_elevation ... ok
test tests::test_select_exec_strategy_uses_supervised_for_detached_start ... ok
test tests::test_select_exec_strategy_uses_supervised_for_plain_run ... ok
test tests::test_select_exec_strategy_uses_supervised_for_proxy ... ok
test tests::test_select_exec_strategy_uses_supervised_for_rollback ... ok
test tests::test_select_exec_strategy_uses_supervised_for_trust_interception ... ok
test tests::test_select_threading_context_uses_crypto_for_trust_scan ... ok
test tests::test_select_threading_context_uses_keyring_for_secrets_only ... ok
test tests::test_sensitive_paths_defined ... ok
test tests::test_trust_interception_active_when_includes_exist ... ok
test tests::test_trust_interception_inactive_for_default_policy ... ok
test theme::tests::test_all_color_slots_used ... ok
test theme::tests::test_available_themes_not_empty ... ok
test theme::tests::test_current_before_init ... ok
test theme::tests::test_resolve_aliases ... ok
test theme::tests::test_resolve_known_themes ... ok
test theme::tests::test_resolve_unknown_falls_back ... ok
test timeouts::tests::supervisor_ipc_read_timeout_clamp ... ok
test timeouts::tests::supervisor_ipc_read_timeout_default ... ok
test timeouts::tests::supervisor_ipc_read_timeout_env_override ... ok
test timeouts::tests::supervisor_ipc_read_timeout_invalid_fallback ... ok
test trust_cmd::tests::base64_empty ... ok
test trust_cmd::tests::base64_known_value ... ok
test trust_cmd::tests::base64_roundtrip ... ok
test trust_cmd::tests::build_keyless_predicate_defaults_to_github_when_gitlab_ci_unset ... ok
test trust_cmd::tests::build_keyless_predicate_gitlab_honors_custom_port ... ok
test trust_cmd::tests::build_keyless_predicate_gitlab_tag_overrides_branch ... ok
test trust_cmd::tests::build_keyless_predicate_uses_gitlab_shape_when_gitlab_ci_true ... ok
test trust_cmd::tests::format_identity_keyed ... ok
test trust_cmd::tests::format_identity_keyless ... ok
test trust_cmd::tests::format_identity_keyless_gitlab ... ok
test trust_cmd::tests::format_identity_keyless_gitlab_custom_port ... ok
test trust_cmd::tests::gitlab_keyless_predicate_returns_none_when_gitlab_ci_not_true ... ok
test trust_cmd::tests::gitlab_keyless_predicate_returns_none_when_gitlab_ci_unset ... ok
test trust_cmd::tests::identity_regex_matches ... ok
test trust_cmd::tests::identity_regex_rejects_non_matching_san ... ok
test trust_cmd::tests::load_public_key_bytes_prefers_disk_cache ... ok
test trust_cmd::tests::load_trust_policy_returns_default_when_no_file ... ok
test trust_cmd::tests::oidc_error_suggests_keyref ... ok
test trust_cmd::tests::public_key_cache_path_encodes_key_id ... ok
test trust_cmd::tests::read_required_oidc_issuer_fails_closed_when_both_unset ... ok
test trust_cmd::tests::read_required_oidc_issuer_fails_closed_when_user_unset_and_env_whitespace_only ... ok
test trust_cmd::tests::read_required_oidc_issuer_rejects_malformed_env_url ... ok
test trust_cmd::tests::read_required_oidc_issuer_returns_env_value_when_user_unset_and_env_set ... ok
test trust_cmd::tests::read_required_oidc_issuer_returns_user_issuer_when_set ... ok
test trust_cmd::tests::user_trust_policy_path_is_some ... ok
test trust_intercept::tests::windows_trust_interceptor_reports_documented_limitation ... ok
test trust_keystore::backend_tests::backend_description_mentions_windows_credential_manager ... ok
test trust_refresh::tests::bad_signature_at_root_surfaces_as_nono_error_setup ... ok
test trust_refresh::tests::cache_bytes_match_baseline ... ok
test trust_refresh::tests::cache_file_loadable_by_load_production_trusted_root ... ok
test trust_refresh::tests::happy_path_walk_returns_trusted_root ... ok
test trust_refresh::tests::malformed_json_at_root_surfaces_as_nono_error_setup ... ok
test trust_refresh::tests::refresh_production_trusted_root_via_env_seam_returns_trusted_root ... ok
test trust_scan::tests::glob_pattern_with_no_matches_does_not_block ... ok
test trust_scan::tests::is_glob_pattern_classification ... ok
test trust_scan::tests::literal_pattern_present_on_disk_does_not_block ... ok
test trust_scan::tests::load_nono_policy_accepts_legacy_predicate_less_policy ... ok
test trust_scan::tests::load_nono_policy_in_toto_attestation_is_fatal_at_user_level ... ok
test trust_scan::tests::load_nono_policy_rejects_non_string_predicate ... ok
test trust_scan::tests::load_nono_policy_rejects_nono_shaped_policy_that_fails_to_load ... ok
test trust_scan::tests::load_nono_policy_rejects_unparseable_nono_shaped_json ... ok
test trust_scan::tests::load_nono_policy_rejects_unrecognised_predicate ... ok
test trust_scan::tests::load_nono_policy_skips_foreign_json_without_predicate ... ok
test trust_scan::tests::load_nono_policy_skips_in_toto_attestation_at_project_level ... ok
test trust_scan::tests::load_nono_policy_skips_unparseable_foreign_json_at_project_level ... ok
test trust_scan::tests::load_nono_policy_user_level_failure_is_always_fatal ... ok
test trust_scan::tests::load_scan_policy_aborts_on_broken_user_policy ... ok
test trust_scan::tests::load_scan_policy_drops_project_level_publishers ... ok
test trust_scan::tests::load_scan_policy_drops_project_publishers_without_user_policy ... ok
test trust_scan::tests::load_scan_policy_merges_user_policy_with_legacy_project_policy ... ok
test trust_scan::tests::load_scan_policy_skips_policy_verification_without_signed_artifacts ... ok
test trust_scan::tests::load_scan_policy_tolerates_in_toto_attestation_in_working_dir ... ok
test trust_scan::tests::load_scan_policy_with_trust_override_skips_verification ... ok
test trust_scan::tests::missing_literal_pattern_blocks_with_deny_enforcement ... ok
test trust_scan::tests::missing_literal_pattern_warns_with_warn_enforcement ... ok
test trust_scan::tests::multi_subject_bundle_detected_and_verified ... ok
test trust_scan::tests::multi_subject_bundle_detects_tampered_file ... ok
test trust_scan::tests::multi_subject_bundle_missing_file_fails ... ok
test trust_scan::tests::multi_subject_bundle_rejects_traversal_subject_name ... ok
test trust_scan::tests::multi_subject_untrusted_publisher_blocks ... ok
test trust_scan::tests::multi_subject_verified_paths_included ... ok
test trust_scan::tests::run_pre_exec_scan_respects_skip_dirs ... ok
test trust_scan::tests::safe_subject_path_accepts_plain_filename ... ok
test trust_scan::tests::safe_subject_path_accepts_subdirectory ... ok
test trust_scan::tests::safe_subject_path_rejects_absolute_path_windows ... ok
test trust_scan::tests::safe_subject_path_rejects_embedded_dotdot ... ok
test trust_scan::tests::safe_subject_path_rejects_relative_dotdot_traversal_windows ... ok
test trust_scan::tests::safe_subject_path_rejects_trailing_dotdot ... ok
test trust_scan::tests::scan_audit_enforcement_always_proceeds ... ok
test trust_scan::tests::scan_blocklisted_file_always_blocks ... ok
test trust_scan::tests::scan_empty_dir_returns_empty_result ... ok
test trust_scan::tests::scan_has_signed_artifacts_detects_per_file_bundle ... ok
test trust_scan::tests::scan_has_signed_artifacts_empty_policy_returns_false ... ok
test trust_scan::tests::scan_has_signed_artifacts_ignores_unsigned_matches ... ok
test trust_scan::tests::scan_nonmatching_files_ignored ... ok
test trust_scan::tests::scan_unsigned_file_deny_enforcement_blocks ... ok
test trust_scan::tests::scan_unsigned_file_warn_enforcement_proceeds ... ok
test trust_scan::tests::verified_paths_empty_when_none_verified ... ok
test trust_scan::tests::verified_paths_returns_only_verified ... ok
test trust_scan::tests::verify_policy_signature_missing_bundle_returns_error ... ok
test update_check::tests::test_detect_ci_environment_falsey_markers_are_ignored ... ok
test update_check::tests::test_detect_ci_environment_generic_ci ... ok
test update_check::tests::test_detect_ci_environment_github_actions ... ok
test update_check::tests::test_detect_ci_environment_no_ci ... ok
test update_check::tests::test_env_var_opt_out ... ok
test update_check::tests::test_generate_uuid_format ... ok
test update_check::tests::test_generate_uuid_uniqueness ... ok
test update_check::tests::test_is_newer_version ... ok
test update_check::tests::test_state_roundtrip ... ok
test update_check::tests::test_state_roundtrip_no_cached ... ok
test update_check::tests::test_update_info_deserialize ... ok
test update_check::tests::test_update_request_serializes_ci_context ... ok
test wiring::path_validation_tests::validate_target_path_accepts_valid_target ... ok
test wiring::path_validation_tests::validate_target_path_rejects_symlink_escape ... ok
test wiring::path_validation_tests::validate_target_path_rejects_traversal ... ok
test wiring::path_validation_tests::validate_target_path_rejects_unc_alias ... ok
test wiring::path_validation_tests::yaml_merge_apply_uses_validate_target_path ... ok
test wiring::tests::apply_yaml_merge_merges_overlay_into_target ... ok
test wiring::tests::apply_yaml_merge_rejects_self_merge ... ok
test wiring::tests::execute_options_default_is_non_force ... ok
test wiring::tests::execute_options_force_enables_adoption ... ok
test wiring::tests::yaml_merge_directive_parses ... ok
test exec_strategy::dacl_guard::tests::ancestor_traverse_stops_at_non_owned_ancestor ... ok
test exec_strategy::dacl_guard::tests::ancestor_guard_classification_matches_the_d37_table ... ok
test exec_strategy::dacl_guard::tests::ancestor_traverse_grants_owned_ancestors_and_reverts_on_drop ... ok
test exec_strategy::dacl_guard::tests::ancestor_read_attributes_stops_at_non_owned_ancestor ... ok
test exec_strategy::dacl_guard::tests::ancestor_read_attributes_grants_owned_ancestors_and_reverts_on_drop ... ok
test exec_strategy::dacl_guard::tests::ancestor_read_attributes_multi_target_covers_each_chain_and_stops_at_root ... ok
test exec_strategy::dacl_guard::tests::ancestor_read_attributes_dedups_shared_ancestor_across_targets ... ok

failures:

---- audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty stdout ----

thread 'audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty' (97892) panicked at crates\nono-cli\src\audit_session.rs:551:9:
assertion `left == right` failed
  left: 224
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- config::tests::nono_home_dir_rejects_non_absolute_override stdout ----

thread 'config::tests::nono_home_dir_rejects_non_absolute_override' (79440) panicked at crates\nono-cli\src\config\mod.rs:410:45:
env lock: PoisonError { .. }

---- config::tests::nono_home_dir_returns_override_when_set stdout ----

thread 'config::tests::nono_home_dir_returns_override_when_set' (17504) panicked at crates\nono-cli\src\config\mod.rs:397:45:
env lock: PoisonError { .. }

---- config::tests::nono_home_dir_falls_through_when_unset stdout ----

thread 'config::tests::nono_home_dir_falls_through_when_unset' (24520) panicked at crates\nono-cli\src\config\mod.rs:428:45:
env lock: PoisonError { .. }

---- config::tests::test_validated_home_falls_back_to_userprofile stdout ----

thread 'config::tests::test_validated_home_falls_back_to_userprofile' (68016) panicked at crates\nono-cli\src\config\mod.rs:370:45:
env lock: PoisonError { .. }

---- config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists stdout ----

thread 'config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists' (103044) panicked at crates\nono-cli\src\config\mod.rs:382:45:
env lock: PoisonError { .. }

---- config::tests::user_state_dir_uses_localappdata_on_windows stdout ----

thread 'config::tests::user_state_dir_uses_localappdata_on_windows' (16452) panicked at crates\nono-cli\src\config\mod.rs:459:45:
env lock: PoisonError { .. }

---- exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap stdout ----

thread 'exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap' (5280) panicked at crates\nono-cli\src\exec_strategy_windows\labels_guard.rs:1215:13:
test setup: could not reassign ownership of C:\Users\OMack\AppData\Local\Temp\.tmpiefNSY\foreign-labeled.txt away from the current user — icacls /setowner "NT AUTHORITY\SYSTEM" failed (C:\Users\OMack\AppData\Local\Temp\.tmpiefNSY\foreign-labeled.txt: This security ID may not be assigned as the owner of this object.), and the "BUILTIN\Administrators" fallback also failed (C:\Users\OMack\AppData\Local\Temp\.tmpiefNSY\foreign-labeled.txt: This security ID may not be assigned as the owner of this object.). This host's session almost certainly lacks SeRestorePrivilege (non-elevated / non-admin token) and cannot construct the non-owned + foreign-labeled combined condition this test pins. This is a documented, loud test-setup failure (D-31), not a silent skip — see 117-30-SUMMARY.md for the host-behavior record.

---- profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name stdout ----

thread 'profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name' (103764) panicked at crates\nono-cli\src\profile_cmd.rs:4369:9:
expected ok, got: Some(ProfileParse("Profile file already exists: \\\\?\\C:\\Users\\OMack\\AppData\\Roaming\\nono\\profiles\\my-agent.json\nUse --force to overwrite"))

---- protected_paths::tests::blocks_child_directory_capability stdout ----

thread 'protected_paths::tests::blocks_child_directory_capability' (3052) panicked at crates\nono-cli\src\protected_paths.rs:309:68:
blocked: ()

---- protected_paths::tests::blocks_parent_directory_capability stdout ----

thread 'protected_paths::tests::blocks_parent_directory_capability' (100340) panicked at crates\nono-cli\src\protected_paths.rs:289:78:
blocked: ()

---- protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root stdout ----

thread 'protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root' (91620) panicked at crates\nono-cli\src\protected_paths.rs:342:10:
blocked: ()


failures:
    audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty
    config::tests::nono_home_dir_falls_through_when_unset
    config::tests::nono_home_dir_rejects_non_absolute_override
    config::tests::nono_home_dir_returns_override_when_set
    config::tests::test_validated_home_falls_back_to_userprofile
    config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists
    config::tests::user_state_dir_uses_localappdata_on_windows
    exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap
    profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name
    protected_paths::tests::blocks_child_directory_capability
    protected_paths::tests::blocks_parent_directory_capability
    protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root

test result: FAILED. 1624 passed; 12 failed; 2 ignored; 0 measured; 0 filtered out; finished in 42.98s

error: test failed, to rerun pass `-p nono-sandbox-cli --bin nono`
     Running unittests src\bin\nono-agentd.rs (target\debug\deps\nono_agentd-0c8ec679c9109bae.exe)

running 93 tests
test agent_daemon::control_loop::tests::control_pipe_sddl_is_medium_il_only ... ok
test agent_daemon::control_loop::tests::classify_response_notanagent ... ok
test agent_daemon::control_loop::tests::classify_response_aiagent_omits_package_sid ... ok
test agent_daemon::control_loop::tests::list_returns_no_agents_when_empty ... ok
test agent_daemon::control_loop::tests::classify_non_appcontainer_pid_returns_not_an_agent ... ok
test agent_daemon::accept_loop::tests::accept_loop_denies_unknown_sid ... ok
test agent_daemon::control_loop::tests::classify_request_deserializes ... ok
test agent_daemon::control_loop::tests::demote_returns_err_for_unknown_tenant ... ok
test agent_daemon::control_loop::tests::no_escape_hatch_list_is_read_only ... ok
test agent_daemon::launch::tests::daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition ... ok
test agent_daemon::launch::tests::daemon_ancestor_skip_rationale_agrees_with_cli_notapplicable_classification ... ok
test agent_daemon::control_loop::tests::demote_does_not_reap_tenant_from_map ... ok
test agent_daemon::accept_loop::tests::session_id_is_routing_hint_not_authz ... ok
test agent_daemon::launch::tests::daemon_dacl_guard_apply_succeeds_when_immediate_ancestor_is_non_owned ... ok
test agent_daemon::launch::tests::daemon_attestation_decision_is_deliberately_two_state ... ok
test agent_daemon::control_loop::tests::list_returns_tenants_when_populated ... ok
test agent_daemon::launch::tests::daemon_dacl_guard_mid_loop_failure_reverts_already_applied ... ok
test agent_daemon::launch::tests::every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence ... ok
test agent_daemon::launch::tests::resolve_exe_path_bare_name_returns_absolute ... ok
test agent_daemon::launch::tests::launch_agent_fresh_profile_per_agent ... ok
test agent_daemon::launch::tests::launch_agent_inserts_into_daemon_state ... ok
test agent_daemon::launch::tests::reap_task_removes_tenant_on_exit ... ok
test agent_daemon::launch::tests::wfp_filter_add_constructs_request ... ok
test agent_daemon::launch::tests::wfp_proxy_only_port_is_parameterised ... ok
test agent_daemon::launch::tests::wfp_absent_fail_secure ... ok
test agent_daemon::launch::tests::wfp_proxy_only_constructs_proxy_mode_request ... ok
test agent_daemon::launch::tests::wfp_absent_no_scoping_ok ... ok
test agent_daemon::launch::windows_impl::attestation_gate_tests::daemon_attest_and_decide_result_matches_exhaustively ... ok
test agent_daemon::launch::windows_impl::attestation_gate_tests::null_handle_aborts_on_app_container_profile ... ok
test agent_daemon::launch::tests::wfp_filter_add_at_launch ... ok
test agent_daemon::launch::windows_impl::attestation_gate_tests::daemon_attest_and_decide_latency ... ok
test agent_daemon::reap::tests::agent_tenant_drop_does_not_panic_on_fake_profile ... ok
test agent_daemon::reap::tests::delete_profile_does_not_panic ... ok
test agent_daemon::reap::tests::wfp_filter_remove_at_reap_not_in_drop ... ok
test agent_daemon::reap::tests::wfp_filter_remove_nonfatal_contract ... ok
test agent_daemon::tests::daemon_state_arcs_share_same_mutex ... ok
test agent_daemon::tests::daemon_state_new_is_empty ... ok
test agent_daemon::tests::daemon_state_registry_insert_remove_roundtrip ... ok
test agent_daemon::tests::daemon_caps_non_empty_for_known_profile ... ok
test agent_daemon::launch::windows_impl::attestation_gate_tests::real_appcontainer_process_outside_the_named_job_aborts_on_job_containment ... ok
test agent_daemon::launch::windows_impl::attestation_gate_tests::real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable ... ok
test agent_daemon::tests::daemon_workspace_path_uses_userprofile ... ok
test agent_daemon::tests::machine_policy_handoff_absent_falls_through_to_per_user ... ok
test agent_daemon::tests::machine_policy_handoff_daemon_state_proxy_port_field ... ok
test agent_daemon::tests::machine_policy_handoff_wholesale_override_excludes_per_user ... ok
test agent_daemon::tests::wr04_systemroot_subpath_passes_allowlist_check ... ok
test telemetry::event::tests::classify_aws_path_is_credential ... ok
test telemetry::event::tests::classify_etc_is_system_path ... ok
test telemetry::event::tests::classify_keystore_path_is_credential ... ok
test agent_daemon::tests::wr04_user_writable_tempdir_rejected_by_allowlist ... ok
test telemetry::event::tests::classify_project_file_is_workspace ... ok
test telemetry::event::tests::classify_tmp_is_temp ... ok
test telemetry::event::tests::downgraded_layers_omitted_when_none ... ok
test telemetry::event::tests::classify_temp_is_temp ... ok
test telemetry::event::tests::classify_ssh_path_is_credential ... ok
test telemetry::event::tests::classify_system32_is_system_path ... ok
test telemetry::event::tests::downgraded_layers_field_round_trips_through_serde ... ok
test telemetry::event::tests::event_id_for_maps_all_five_types ... ok
test telemetry::event::tests::layer_attestation_downgraded_event_id_is_10011 ... ok
test telemetry::event::tests::override_event_ids_are_10006_through_10010 ... ok
test telemetry::event::tests::override_event_type_serde_roundtrip ... ok
test telemetry::event::tests::path_hash_differs_for_different_paths ... ok
test telemetry::event::tests::path_hash_differs_for_different_salts ... ok
test telemetry::event::tests::path_hash_for_does_not_contain_raw_path ... ok
test telemetry::event::tests::path_hash_for_is_deterministic ... ok
test telemetry::event::tests::security_event_type_serde_round_trip ... ok
test telemetry::event::tests::security_event_serializes_with_pascal_case_sc1_fields ... ok
test telemetry::tests::advance_chain_uses_hmac_not_sha2_placeholder ... ok
test telemetry::tests::advance_chain_changes_head_and_increments_sequence ... ok
test telemetry::tests::chain_head_hex_is_64_chars ... ok
test telemetry::tests::advance_chain_uses_key_in_hash ... ok
test telemetry::tests::chain_state_key_is_zeroizing_type ... ok
test telemetry::tests::chain_sequence_genesis_is_zero ... ok
test telemetry::tests::emit_override_event_err_on_poisoned_mutex ... ok
test telemetry::tests::min_severity_filter_predicate_matches_policy_threshold ... ok
test telemetry::tests::new_produces_nonzero_key_and_salt ... ok
test telemetry::tests::poison_for_test_poisons_mutex_for_emit_override_event ... ok
test telemetry::tests::severity_for_all_denial_types_is_warning ... ok
test telemetry::tests::severity_for_override_lifecycle_events_is_warning ... ok
test telemetry::tests::telemetry_domains_differ_from_audit_domains ... ok
test telemetry::tests::telemetry_event_domain_value ... ok
test telemetry::tests::two_events_produce_different_chain_heads ... ok
test telemetry::windows::tests::event_id_constants_are_correct ... ok
test telemetry::windows::tests::event_log_source_is_nono ... ok
test telemetry::windows::tests::emit_security_event_is_non_fatal ... ok
test telemetry_init::tests::d01_network_deny_advances_chain_sequence_to_one ... ok
test telemetry::tests::emit_override_event_advances_chain_by_one ... ok
test telemetry::tests::emit_override_event_none_zt_audit_hash_ok ... ok
test telemetry_init::tests::opt_out_disabled_layer_does_not_advance_chain ... ok
test telemetry::tests::emit_override_event_two_calls_advance_by_two ... ok
test telemetry::tests::advance_and_emit_holds_lock_across_full_build_advance_emit_sequence ... ok
test agent_daemon::launch::tests::daemon_dacl_guard_applies_and_reverts_write_grant ... ok
test agent_daemon::launch::tests::daemon_dacl_guard_reap_revokes_traverse_paths ... ok

test result: ok. 93 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.78s

     Running unittests src\bin\nono-wfp-service.rs (target\debug\deps\nono_wfp_service-e0049732f22c81f0.exe)

running 25 tests
test windows_impl::tests::deterministic_filter_keys_are_stable ... ok
test windows_impl::tests::blocked_mode_probe_reports_missing_target_program ... ok
test windows_impl::tests::event_log_message_format_includes_source_and_event_id ... ok
test windows_impl::tests::invalid_request_kind_returns_invalid_request_response ... ok
test windows_impl::tests::discrete_port_and_range_coexist_in_same_request_without_regression ... ok
test windows_impl::tests::port_range_only_request_produces_one_spec_per_range_per_applicable_layer_not_per_port ... ok
test windows_impl::tests::protocol_mismatch_returns_protocol_mismatch_response ... ok
test windows_impl::tests::port_range_alone_triggers_block_fallback_even_in_allow_all_mode ... ok
test windows_impl::tests::proxy_policy_filter_specs_include_loopback_permits_and_block_fallback ... ok
test windows_impl::tests::purge_covers_every_layer_enforcement_installs_into ... ok
test windows_impl::tests::purge_wfp_objects_arg_constant_is_correct ... ok
test windows_impl::tests::purge_wfp_objects_arg_is_distinct_from_other_mode_args ... ok
test windows_impl::tests::runtime_activation_probe_fails_closed ... ok
test windows_impl::tests::runtime_activation_request_size_limit_matches_protocol_guard ... ok
test windows_impl::tests::service_contract_output_is_stable ... ok
test windows_impl::tests::service_mode_fails_closed ... ok
test windows_impl::tests::startup_sweep_outcome_failed_message_is_deterministic ... ok
test windows_impl::tests::startup_sweep_outcome_removed_message_is_deterministic ... ok
test windows_impl::tests::startup_sweep_outcome_skipped_message_is_deterministic ... ok
test windows_impl::tests::startup_sweep_summary_is_empty_when_no_outcomes ... ok
test windows_impl::tests::startup_sweep_summary_reports_counts ... ok
test windows_impl::tests::sid_to_security_descriptor_fails_on_invalid_sid ... ok
test windows_impl::tests::sid_to_security_descriptor_works ... ok
test windows_impl::tests::test_wfp_pipe_sddl_includes_interactive_users ... ok
test windows_impl::tests::validate_target_request_fields_requires_program_and_rule_names ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src\bin\test-connector.rs (target\debug\deps\test_connector-108ec226c021d576.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\bin\windows-net-probe.rs (target\debug\deps\windows_net_probe-0b0d26cddd3c7258.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\adr_aipc_unix_futures.rs (target\debug\deps\adr_aipc_unix_futures-0a158b550ed60fa8.exe)

running 6 tests
test adr_exists_at_locked_path ... ok
test project_md_cross_links_the_adr ... ok
test adr_length_is_decision_only_not_implementation ... ok
test adr_decision_table_has_six_handlekind_rows ... ok
test adr_has_all_required_h2_sections ... ok
test adr_status_is_accepted ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests\aipc_handle_brokering_integration.rs (target\debug\deps\aipc_handle_brokering_integration-33b81bf977867ef8.exe)

running 5 tests
test integration_event_broker_round_trip ... ok
test integration_job_object_broker_round_trip ... ok
test integration_mutex_broker_round_trip ... ok
test integration_pipe_broker_round_trip ... ok
test integration_socket_broker_round_trip ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\attach_streaming_integration.rs (target\debug\deps\attach_streaming_integration-c81c48f0c78a180d.exe)

running 3 tests
test detached_child_stdout_reaches_session_log_via_anonymous_pipes ... ignored
test parse_session_id_recognizes_started_banner_line ... ok
test parse_session_id_returns_none_when_no_banner ... ok

test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\audit_attestation.rs (target\debug\deps\audit_attestation-af122516c146723e.exe)

running 2 tests
test audit_verify_reports_signed_attestation_with_pinned_public_key ... FAILED
test rollback_signed_session_verifies_from_audit_dir_bundle ... FAILED

failures:

---- audit_verify_reports_signed_attestation_with_pinned_public_key stdout ----

thread 'audit_verify_reports_signed_attestation_with_pinned_public_key' (86812) panicked at crates\nono-cli\tests\audit_attestation.rs:24:5:
expected success, stdout: , stderr: 
  nono v0.70.0
  Capabilities:
  ────────────────────────────────────────────────────
    r   \\?\C:\Users\OMack\Nono (dir)
       + 1 system/group paths (-v to show)
   net  outbound allowed
  ────────────────────────────────────────────────────

nono: Command execution failed: /bin/pwd: cannot find binary path

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- rollback_signed_session_verifies_from_audit_dir_bundle stdout ----

thread 'rollback_signed_session_verifies_from_audit_dir_bundle' (53868) panicked at crates\nono-cli\tests\audit_attestation.rs:24:5:
expected success, stdout: , stderr: 
  nono v0.70.0
  Capabilities:
  ────────────────────────────────────────────────────
    r   \\?\C:\Users\OMack\Nono (dir)
       + 1 system/group paths (-v to show)
   net  outbound allowed
  ────────────────────────────────────────────────────

nono: Command execution failed: /bin/pwd: cannot find binary path



failures:
    audit_verify_reports_signed_attestation_with_pinned_public_key
    rollback_signed_session_verifies_from_audit_dir_bundle

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.29s

error: test failed, to rerun pass `-p nono-sandbox-cli --test audit_attestation`
     Running tests\auto_pull_e2e_linux.rs (target\debug\deps\auto_pull_e2e_linux-63f7cf5484305387.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\broker_authenticode.rs (target\debug\deps\broker_authenticode-c9c6fcce4da837ec.exe)

running 6 tests
test broker_valid_signature_spawns ... ok
test dev_skip_does_not_bypass_release_layout ... ok
test broker_unsigned_release_refuses_spawn ... ok
test broker_signature_mismatch_refuses_spawn ... ok
test self_authenticode_extracts_subject_and_thumbprint ... ok
test each_dispatch_revalidates ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s

     Running tests\builtin_profile_load.rs (target\debug\deps\builtin_profile_load-d1c1a91cf83a40c8.exe)

running 4 tests
test test_builtin_profile_claude_code_loads_canonical_sections ... ok
test test_builtin_profile_codex_loads_canonical_sections ... ok
test test_builtin_profile_claude_no_keychain_loads_canonical_sections ... ok
test test_all_builtin_profiles_use_canonical_sections ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s

     Running tests\config_flag.rs (target\debug\deps\config_flag-c9a93fb16ca88cf8.exe)

running 7 tests
test config_conflicts_with_allow ... ok
test config_nonexistent_file_fails ... ok
test config_conflicts_with_profile ... ok
test config_with_invalid_json_fails ... ok
test config_with_valid_manifest_is_accepted ... ok
test config_semantic_validation_rejects_bad_inject ... ok
test config_with_missing_version_fails ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

     Running tests\daemon_handle_baseline.rs (target\debug\deps\daemon_handle_baseline-7ce47a529238e5e9.exe)

running 5 tests
test classify_pid_returns_verdict_from_daemon ... ok
test daemon_concurrent_agents ... ok
test daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance ... ok
test fresh_token_isolation_agents_have_distinct_package_sids ... ok
test n_agents_over_time_returns_to_baseline_handle_count ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\deny_overlap_run.rs (target\debug\deps\deny_overlap_run-8ccd9e137f7b489b.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\deprecated_policy.rs (target\debug\deps\deprecated_policy-9da41472d0ae1348.exe)

running 6 tests
test policy_help_top_level_labels_deprecated ... ok
test policy_diff_alias ... ok
test policy_groups_alias_prints_warning_and_matches_profile_groups ... ok
test policy_show_alias ... ok
test policy_validate_alias ... ok
test policy_profiles_alias_maps_to_profile_list ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s

     Running tests\env_vars.rs (target\debug\deps\env_vars-c47147dbf7fe3e85.exe)

running 72 tests
test env_conflict_allow_net_and_block_net ... ok
test cli_flag_overrides_env_var ... ok
test env_allow_net_conflicts_with_upstream_proxy ... ok
test env_nono_capability_elevation_accepts_truthy ... ok
test env_nono_block_net_accepts_true ... ok
test env_nono_block_net ... ok
test env_nono_allow_comma_separated ... ok
test allow_net_overrides_profile_external_proxy ... ok
test env_nono_network_profile ... ok
test env_nono_trust_override_accepts_truthy ... ok
test env_nono_upstream_proxy ... ok
test env_nono_upstream_bypass_comma_separated ... ok
test env_nono_upstream_bypass_requires_upstream_proxy ... ok
test env_nono_profile ... ok
test windows_attach_help_reports_documented_limitation ... ok
test windows_detach_help_reports_documented_limitation ... ok
test legacy_env_nono_net_block_still_works ... ok
test windows_inspect_help_reports_documented_limitation ... ok
test windows_allow_launch_services_reports_macos_only_limitation ... ok
test windows_run_allows_cmd_type_for_relative_file_inside_allowlist ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_logs_help_reports_documented_limitation ... ok
test windows_run_allows_direct_write_inside_dynamically_labeled_low_integrity_dir ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_run_allows_direct_write_inside_locallow_allowlisted_dir ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_run_allows_direct_write_inside_low_integrity_allowlisted_dir ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_run_allows_file_grants_in_preview_live_run ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_open_url_helper_reports_documented_limitation ... ok
test windows_run_allows_powershell_copy_into_redirected_tmp_runtime_dir ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_run_allows_powershell_get_content_inside_allowlist ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_prune_help_reports_documented_limitation ... ok
test windows_dry_run_reports_sandbox_validation ... ok
test windows_root_help_reports_session_management_as_unsupported_surface ... ok
test windows_root_help_reports_supported_command_surface_without_full_parity_claim ... ok
test windows_run_blocks_absolute_path_argument_outside_allowlist ... ok
test windows_run_blocks_cmd_copy_to_absolute_destination_outside_allowlist ... ok
test windows_run_blocks_cmd_type_for_absolute_file_outside_allowlist ... ok
test windows_run_blocks_comp_file_outside_allowlist ... ok
test windows_run_blocks_directory_allowlist_when_workdir_is_outside_supported_subset ... ok
test windows_run_blocks_fc_file_outside_allowlist ... ok
test windows_run_blocks_cscript_destination_outside_allowlist ... ok
test windows_run_blocks_xcopy_destination_outside_allowlist ... ok
test windows_run_blocks_powershell_copy_to_absolute_destination_outside_allowlist ... ok
test windows_run_allow_all_network_probe_connects has been running for over 60 seconds
test windows_run_allows_cmd_write_into_redirected_tmp_runtime_dir has been running for over 60 seconds
test windows_run_allows_findstr_inside_allowlist has been running for over 60 seconds
test windows_run_allows_supported_directory_allowlist_in_live_run has been running for over 60 seconds
test windows_run_blocks_live_block_net_without_enforcement has been running for over 60 seconds
test windows_run_blocks_workspace_write_even_with_writable_allowlist has been running for over 60 seconds
test windows_run_executes_basic_command has been running for over 60 seconds
test windows_run_filters_dangerous_env_vars_and_keeps_safe_ones has been running for over 60 seconds
test windows_run_allows_supported_directory_allowlist_in_live_run ... ok
test windows_run_blocks_workspace_write_even_with_writable_allowlist ... ok
test windows_run_allows_cmd_write_into_redirected_tmp_runtime_dir ... ok
test windows_run_allows_findstr_inside_allowlist ... ok
test windows_run_help_reports_supported_command_surface_and_backend_readiness ... ok
test windows_run_live_codex_profile_fails_intentionally_with_backend_reason ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified ... FAILED
test windows_run_prefers_managed_low_integrity_runtime_root_inside_allowlist ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_run_filters_host_toolchain_home_vars_without_runtime_dir has been running for over 60 seconds
test windows_run_honors_workdir has been running for over 60 seconds
test windows_run_live_default_profile_executes_command has been running for over 60 seconds
test windows_run_propagates_child_exit_code has been running for over 60 seconds
test windows_run_honors_workdir ... ok
test windows_run_read_only_allowlist_blocks_runtime_write_attempt has been running for over 60 seconds
test windows_run_read_only_allowlist_blocks_runtime_write_attempt ... ok
test windows_run_read_only_allowlist_still_reads_inside_policy ... ignored, requires live Windows AppContainer/low-integrity enforcement; run with --include-ignored on a configured Windows host
test windows_run_redirects_profile_state_vars_into_writable_allowlist has been running for over 60 seconds
test windows_run_redirects_profile_state_vars_into_writable_allowlist ... ok
test windows_run_redirects_temp_vars_into_writable_allowlist has been running for over 60 seconds
test windows_run_redirects_temp_vars_into_writable_allowlist ... ok
test windows_run_smoke_validates_stdout_stderr_and_exit_code has been running for over 60 seconds
test windows_run_allow_all_network_probe_connects ... FAILED
test windows_run_supervised_blocks_runtime_capability_elevation_with_actionable_diagnostic has been running for over 60 seconds
test windows_run_filters_dangerous_env_vars_and_keeps_safe_ones ... ok
test windows_run_executes_basic_command ... ok
test windows_run_blocks_live_block_net_without_enforcement ... FAILED
test windows_setup_check_only_reports_live_profile_subset ... ok
test windows_setup_check_only_reports_unified_support_status ... ok
test windows_shell_live_reports_documented_limitation ... ignored, interactive shell test; requires terminal; run manually
test windows_shell_live_reports_supported_alternative_without_preview_claim ... ignored, interactive shell test; requires terminal; run manually
test windows_shell_help_reports_documented_limitation ... ok
test windows_wrap_live_reports_supported_alternative_without_preview_claim ... ignored, wrap behavior semantics changed; see Phase 32 wrap availability update
test windows_wrap_reports_documented_limitation ... ignored, wrap behavior semantics changed; see Phase 32 wrap availability update
test windows_wrap_help_reports_documented_limitation ... ok
test windows_run_supervised_rollback_executes_command has been running for over 60 seconds
test windows_run_supervised_rollback_executes_command ... ok
test windows_run_live_default_profile_executes_command ... ok
test windows_run_filters_host_toolchain_home_vars_without_runtime_dir ... ok
test windows_run_propagates_child_exit_code ... ok
test windows_run_smoke_validates_stdout_stderr_and_exit_code ... ok
test windows_run_supervised_blocks_runtime_capability_elevation_with_actionable_diagnostic ... ok

failures:

---- windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified stdout ----

thread 'windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified' (7552) panicked at crates\nono-cli\tests\env_vars.rs:1556:5:
Windows preview should keep using the verified runtime root inside the writable allowlist, output:

  nono v0.70.0
nono: Sandbox initialization failed: Refusing to grant 'C:\Users\OMack\AppData\Local\Temp\.tmpJUlGb9' (source: CLI) because it overlaps protected nono state root 'C:\Users\OMack\AppData\Local\Temp\.tmpJUlGb9\fake-localappdata\nono'.

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- windows_run_allow_all_network_probe_connects stdout ----

thread 'windows_run_allow_all_network_probe_connects' (82464) panicked at crates\nono-cli\tests\env_vars.rs:816:5:
Windows allow-all network probe should connect successfully, output:

  nono v0.70.0
  Capabilities:
  ────────────────────────────────────────────────────
   r+w  \\?\C:\Users\OMack\Nono\target\debug (dir)
       + 2 system/group paths (-v to show)
   net  outbound allowed
  ────────────────────────────────────────────────────

  Applying sandbox...

nono: Snapshot error: Rollback budget exceeded: 2153433890 bytes tracked (limit: 2147483648 bytes). Consider adding exclusion patterns for large directories, or disable rollback with --no-rollback.


---- windows_run_blocks_live_block_net_without_enforcement stdout ----

thread 'windows_run_blocks_live_block_net_without_enforcement' (69900) panicked at crates\nono-cli\tests\env_vars.rs:2965:5:
Windows preview should block live network restriction requests, output:
test

  nono v0.70.0
  Skipping CWD prompt (non-interactive). Use --allow-cwd to include working directory.
  Capabilities:
  ────────────────────────────────────────────────────
       + 2 system/group paths (-v to show)
   net  outbound blocked
  ────────────────────────────────────────────────────

  Applying sandbox...

[2m2026-08-12T03:40:00.964806Z[0m [33m WARN[0m label guard: path not owned by current user; skipping mandatory label apply (system paths are Medium-IL by default and already readable by Low-IL subjects) [3mpath[0m[2m=[0mC:\Windows [3maccess[0m[2m=[0mRead



failures:
    windows_run_allow_all_network_probe_connects
    windows_run_blocks_live_block_net_without_enforcement
    windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified

test result: FAILED. 55 passed; 3 failed; 14 ignored; 0 measured; 0 filtered out; finished in 2061.29s

error: test failed, to rerun pass `-p nono-sandbox-cli --test env_vars`
     Running tests\exec_identity_windows.rs (target\debug\deps\exec_identity_windows-b319b3b9b0e37685.exe)

running 2 tests
test nono_binary_loads_without_unresolved_authenticode_symbols ... ok
test nono_prune_help_still_functions_post_authenticode_addition ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests\keyless_offline_invariant.rs (target\debug\deps\keyless_offline_invariant-1590116d0a44fea4.exe)

running 1 test
test verify_path_uses_no_async_network_io ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests\keyless_sign.rs (target\debug\deps\keyless_sign-783bb9daf085916a.exe)

running 3 tests
test keyless_sign_then_verify_roundtrip ... ignored, P32-DEFER-001: requires mock Fulcio/Rekor responses wired into the test binary; see keyless_sign.rs module-level doc for capture procedure
test frozen_tuf_fixture_is_present ... ok
test mock_servers_only_no_real_network ... ok

test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\keyless_verify.rs (target\debug\deps\keyless_verify-ba849c5924c34e18.exe)

running 5 tests
test verify_accepts_san_match ... ok
test verify_rejects_missing_issuer ... ok
test discover_oidc_token_error_suggests_keyref ... ok
test verify_rejects_missing_identity ... ok
test verify_rejects_san_mismatch ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.90s

     Running tests\layer_force_unavailable.rs (target\debug\deps\layer_force_unavailable-1086a0bf3226c2b4.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\layer_registry_meta_test.rs (target\debug\deps\layer_registry_meta_test-b2b92a8765bd7feb.exe)

running 9 tests
test contains_fn_exact_accepts_real_definition_after_rejecting_a_false_positive ... ok
test contains_fn_exact_rejects_bang_suffix ... ok
test pascal_to_snake_case_matches_expected_shapes ... ok
test contains_fn_exact_rejects_doc_comment_mention ... ok
test host_gated_rows_are_loud ... ok
test manual_verification_section_excludes_the_registry_table ... ok
test security_assumptions_are_loud ... ok
test also_automated_entries_are_non_vacuous ... ok
test every_registry_row_has_a_test ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests\layer_registry_selfcheck.rs (target\debug\deps\layer_registry_selfcheck-fab4e33362cd10ab.exe)

running 11 tests
test call_site_extraction_ignores_backtick_doc_comment_citations ... ok
test content_defines_symbol_accepts_a_qualified_type_method_citation ... ok
test content_defines_symbol_accepts_a_real_definition ... ok
test content_defines_symbol_rejects_a_doc_comment_mention ... ok
test content_defines_symbol_rejects_a_prefix_preserving_rename ... ok
test content_defines_symbol_distinguishes_same_named_methods_in_different_impls ... ok
test spec_call_site_cells_match_registry_call_sites ... ok
test spec_matches_registry ... ok
test symbol_citation_extraction_finds_the_eight_converted_citations ... ok
test registry_call_sites_exist ... ok
test every_spec_symbol_citation_resolves_to_a_real_definition ... FAILED

failures:

---- every_spec_symbol_citation_resolves_to_a_real_definition stdout ----

thread 'every_spec_symbol_citation_resolves_to_a_real_definition' (87260) panicked at crates\nono-cli\tests\layer_registry_selfcheck.rs:726:5:
proj/SPEC-windows-fail-direction-contract.md cites a file.rs::Symbol that does not resolve to a real definition anywhere in the document (Manual verification section, discrepancy ledger, or any other section — not only the Layer registry table):
"file.rs::Symbol" -> resolved to C:\Users\OMack\Nono\crates\nono-cli\src\exec_strategy_windows\file.rs (unreadable: The system cannot find the file specified. (os error 2))
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    every_spec_symbol_citation_resolves_to_a_real_definition

test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

error: test failed, to rerun pass `-p nono-sandbox-cli --test layer_registry_selfcheck`
     Running tests\learn_windows_integration.rs (target\debug\deps\learn_windows_integration-3d95352dcdbe6c48.exe)

running 1 test
test run_learn_against_dir_command_captures_files ... ignored, requires Windows host with administrator privileges (ETW)

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\manifest_roundtrip.rs (target\debug\deps\manifest_roundtrip-40565605245a5e6d.exe)

running 14 tests
test manifest_credential_env_var_accepted_and_round_trips ... ok
test manifest_credentials_accepted_in_config ... ok
test manifest_includes_workdir_grant ... ok
test manifest_includes_group_allow_paths ... ok
test manifest_proxy_mode_not_downgraded_to_blocked ... ok
test manifest_grants_are_deduplicated ... ok
test manifest_includes_group_blocked_commands ... ok
test manifest_includes_group_deny_paths ... ok
test manifest_uri_credential_without_env_var_fails_validation ... ok
test manifest_override_deny_removes_deny_from_export ... ok
test rollback_enabled_without_supervised_fails_validation ... ok
test rollback_enabled_with_supervised_is_accepted ... ok
test all_builtin_profiles_manifest_round_trip_is_complete ... ok
test profile_to_manifest_roundtrip_preserves_capabilities ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.82s

     Running tests\offline_verify_extended_trust_bundle.rs (target\debug\deps\offline_verify_extended_trust_bundle-5bfdd330526a7541.exe)

running 3 tests
test invalid_installed_path_values_are_rejected ... ok
test legacy_bundle_parses_and_falls_back_to_artifact_name ... ok
test extended_bundle_parses_and_fields_are_accessible ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\profile_cli.rs (target\debug\deps\profile_cli-653ea1abf932aba6.exe)

running 16 tests
test test_groups_list_output ... ok
test test_groups_detail_output ... ok
test test_diff_output ... ok
test test_groups_json ... ok
test test_groups_unknown_exits_error ... ok
test test_policy_diff_json_no_rust_debug_syntax ... ok
test test_list_output ... ok
test test_show_format_manifest_claude_code_profile ... ok
test test_show_format_manifest_default_profile ... ok
test test_show_profile_json ... ok
test test_show_format_manifest_round_trip ... ok
test test_validate_invalid_group ... ok
test test_validate_valid_profile ... ok
test test_show_profile_output ... ok
test test_policy_show_json_no_rust_debug_syntax ... ok
test test_show_format_manifest_all_builtins_succeed ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.62s

     Running tests\profile_cmd.rs (target\debug\deps\profile_cmd-0e40c1a2eebf05e9.exe)

running 10 tests
test test_init_invalid_name_exits_error ... ok
test test_guide_outputs_markdown ... ok
test test_init_rejects_existing_file_without_force ... ok
test test_init_invalid_group_exits_error ... ok
test test_init_invalid_extends_exits_error ... ok
test test_init_full_creates_all_additive_sections ... ok
test test_init_force_overwrites ... ok
test test_schema_output_to_file ... ok
test test_schema_outputs_valid_json ... ok
test test_init_creates_valid_profile ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s

     Running tests\profile_drafts_test.rs (target\debug\deps\profile_drafts_test-dd80484b18c79ce1.exe)

running 19 tests
test init_draft_invalid_name_rejected ... ok
test init_draft_refresh_errors_without_draft ... ok
test init_draft_writes_to_drafts_dir ... ok
test init_draft_refresh_errors_without_canonical ... ok
test init_draft_refresh_preserves_content ... ok
test init_draft_with_existing_canonical_writes_base_sidecar ... ok
test promote_base_hash_mismatch_action_required ... ok
test promote_invalid_name_rejected ... ok
test promote_shadow_builtin_refused ... ok
test promote_first_time_skips_base_hash ... ok
test promote_shadow_package_managed_refused ... ok
test promote_declined_does_not_modify_canonical ... ok
test draft_promote_roundtrip_serde ... ok
test init_draft_force_overwrites ... ok
test promote_yes_renames_draft_to_canonical ... ok
test promote_yes_with_diff_emits_summary_not_full_diff ... ok
test validate_draft_strict_fails_legacy_keys ... ok
test validate_draft_and_strict_compose ... ok
test validate_draft_resolves_drafts_dir ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.30s

     Running tests\profile_validate_strict.rs (target\debug\deps\profile_validate_strict-91ea914363c32090.exe)

running 3 tests
test test_profile_validate_non_strict_warns_and_continues ... ok
test test_profile_validate_strict_accepts_canonical_key ... ok
test test_profile_validate_strict_rejects_legacy_override_deny ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

     Running tests\proxy_command_run.rs (target\debug\deps\proxy_command_run-5f947ae6ebf2128e.exe)

running 3 tests
test test_proxy_help_exits_zero_and_documents_usage ... ok
test test_proxy_no_auth_with_non_loopback_listen_is_rejected ... ok
test test_proxy_non_loopback_listen_requires_allow_remote ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests\prune_alias_deprecation.rs (target\debug\deps\prune_alias_deprecation-02bbaf08c2b3b4c6.exe)

running 3 tests
test prune_alias_is_hidden_from_top_level_help ... ok
test prune_alias_surfaces_deprecation_note_in_help ... ok
test prune_alias_emits_stderr_deprecation_note_on_invocation has been running for over 60 seconds
test prune_alias_emits_stderr_deprecation_note_on_invocation ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 77.10s

     Running tests\resl_nix_async_signal_safety.rs (target\debug\deps\resl_nix_async_signal_safety-6e6f3f5fbc64d0ee.exe)

running 5 tests
test wr_04_no_pid_fallback_on_getpgid_failure ... ok
test cr_01_and_wr_02_const_msg_byte_strings_present ... ok
test wr_02_no_silent_setrlimit_discards ... ok
test cr_01_no_format_macro_in_post_fork_child_branch ... FAILED
test cr_02_direct_mode_timeout_emits_warn_macro ... ok

failures:

---- cr_01_no_format_macro_in_post_fork_child_branch stdout ----

thread 'cr_01_no_format_macro_in_post_fork_child_branch' (103976) panicked at crates\nono-cli\tests\resl_nix_async_signal_safety.rs:100:9:
expected function signature `fn clear_close_on_exec(fd: i32) -> std::io::Result<()>` in exec_strategy.rs — if this helper was renamed or its signature changed, update the strengthened CR-01-RESIDUAL test in resl_nix_async_signal_safety.rs
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    cr_01_no_format_macro_in_post_fork_child_branch

test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

error: test failed, to rerun pass `-p nono-sandbox-cli --test resl_nix_async_signal_safety`
     Running tests\resl_nix_linux.rs (target\debug\deps\resl_nix_linux-8d913eaeddb3c6fb.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\resl_nix_macos.rs (target\debug\deps\resl_nix_macos-62c3f9eccae854cc.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\resl_supervisor_drain.rs (target\debug\deps\resl_supervisor_drain-e89812bd99a7daae.exe)

running 2 tests
test every_supervisor_loop_exit_drains_network_notifications ... ok
test supervisor_loop_break_paths_drain_before_the_blocking_wait ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\rollback_audit_conflict.rs (target\debug\deps\rollback_audit_conflict-e828a23715d63de0.exe)

running 2 tests
test rollback_no_audit_conflict_rejected_at_parse ... ok
test no_audit_rollback_reverse_order_also_rejected ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests\setup_trust_root.rs (target\debug\deps\setup_trust_root-8c802c94053c4a07.exe)

running 10 tests
test from_file_with_refresh_rejected_by_clap ... ok
test from_file_malformed_quote_flipped_fails_closed ... ok
test setup_refresh_trust_root_writes_cache ... ignored, requires network access to https://tuf-repo-cdn.sigstore.dev (manual operator verification)
test from_file_missing_path_no_partial_cache ... ok
test from_file_happy_path_writes_byte_identical_cache_and_stdout_matches_shape ... ok
test from_file_malformed_truncated_fails_closed ... ok
test from_file_expired_fails_closed ... ok
test from_file_phase_index_uses_shared_slot ... ok
test setup_check_only_reports_stale_cache_with_recovery_hint ... ok
test setup_check_only_reports_uninitialized_cache ... ok

test result: ok. 9 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.48s

     Running tests\socket_access_run.rs (target\debug\deps\socket_access_run-dbe0ec99a33e500e.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\spiffe_run.rs (target\debug\deps\spiffe_run-d67a29eabb89bb78.exe)

running 2 tests
test spiffe_jwt_credential_injected_end_to_end ... ok
test spiffe_jwt_proxy_starts_with_live_agent ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\supervisor_ipc_robustness_unix.rs (target\debug\deps\supervisor_ipc_robustness_unix-aeb91edd09b052f3.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\supervisor_ipc_robustness_windows.rs (target\debug\deps\supervisor_ipc_robustness_windows-b18ce2e8d966750d.exe)

running 4 tests
test disconnect_and_reconnect_method_is_reachable ... ok
test scaffold_links_nono_lib ... ok
test capability_pipe_reconnect_named_pipe ... ok
test bounded_read_timeout_via_recv_message ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s

     Running tests\trust_policy_template.rs (target\debug\deps\trust_policy_template-90ba640e8fc1d4a4.exe)

running 1 test
test default_template_parses ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests\wfp_port_integration.rs (target\debug\deps\wfp_port_integration-5744852abe3910fb.exe)

running 3 tests
test wfp_port_permit_allows_real_tcp_connection ... ignored
test compile_network_policy_localhost_port_appears_in_policy ... ok
test allow_all_policy_with_only_a_port_range_has_port_rules ... ok

test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\yaml_merge_reversal.rs (target\debug\deps\yaml_merge_reversal-1a087aff9f974995.exe)

running 4 tests
test test_yaml_merge_preserves_validate_path_within ... ok
test test_yaml_merge_path_traversal_rejected_through_handler ... ok
test test_profile_patch_yaml_merge_directive_applied ... ok
test test_yaml_merge_reversal_failure ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s

     Running unittests src\lib.rs (target\debug\deps\nono_proxy-cd998347966e0b7b.exe)

running 307 tests
test audit::tests::log_denied_records_reason ... ok
test audit::tests::log_allowed_records_event ... ok
test auth::tests::extract_trust_domain_invalid ... ok
test auth::tests::extract_trust_domain_empty ... ok
test auth::tests::extract_trust_domain_valid ... ok
test capture::tests::capture_fails_closed_on_unrewritten_token_field ... ok
test auth::tests::extract_trust_domain_no_path ... ok
test capture::tests::capture_phantom_rejects_unadmitted_consumer ... ok
test capture::tests::capture_phantom_resolve_returns_real_for_admitted_consumer ... ok
test capture::tests::capture_phantom_resolve_unknown_phantom_returns_none ... ok
test capture::tests::capture_reject_ignores_empty_string_token_field ... ok
test capture::tests::capture_reject_passes_when_field_is_configured ... ok
test capture::tests::capture_reject_passes_when_no_token_fields_present ... ok
test capture::tests::capture_rewrite_gives_each_field_its_own_phantom ... ok
test capture::tests::capture_phantom_mint_returns_unique_nonempty_ids ... ok
test capture::tests::capture_store_debug_output_never_contains_real_token ... ok
test capture::tests::capture_rewrite_skips_missing_path_without_error ... ok
test capture::tests::capture_rewrites_configured_fields ... ok
test capture::tests::resolve_request_nonce_fields_leaves_unknown_value_unchanged ... ok
test capture::tests::jwt_shaped_phantom_has_three_dot_separated_parts_and_signature_is_the_phantom ... ok
test capture::tests::resolve_request_nonce_fields_leaves_unadmitted_phantom_unchanged ... ok
test capture::tests::resolve_request_nonce_fields_resolves_admitted_phantom_in_place ... ok
test config::tests::capture_config_rejects_unknown_keys ... ok
test config::tests::endpoint_policy_deny_rule_is_enforced ... ok
test config::tests::endpoint_policy_legacy_route_preserves_deny_semantics ... ok
test config::tests::test_aws_auth_config_all_fields_roundtrip ... ok
test config::tests::test_aws_auth_config_unknown_field_rejected ... ok
test config::tests::test_aws_auth_config_minimal_deserializes ... ok
test config::tests::oauth2_config_debug_redacts_client_id_and_secret ... ok
test config::tests::endpoint_policy_approve_without_backend_is_recognized ... ok
test config::tests::test_aws_auth_empty_object_sets_all_none ... ok
test config::tests::test_aws_auth_field_absent_is_none ... ok
test config::tests::test_compiled_endpoint_rules_empty_allows_all ... ok
test config::tests::test_compiled_endpoint_rules_invalid_pattern_rejected ... ok
test config::tests::test_config_serialization ... ok
test config::tests::test_compiled_endpoint_rules_hot_path ... ok
test config::tests::test_endpoint_allowed_empty_rules_allows_all ... ok
test config::tests::test_default_config ... ok
test config::tests::test_compiled_endpoint_rules_percent_encoded ... ok
test capture::tests::capture_store_holds_only_in_memory ... ok
test config::tests::test_endpoint_rule_double_slash_normalized ... ok
test config::tests::test_endpoint_allowed_multiple_rules ... ok
test config::tests::test_endpoint_rule_method_case_insensitive ... ok
test config::tests::test_endpoint_rule_double_wildcard ... ok
test config::tests::test_endpoint_rule_method_mismatch ... ok
test config::tests::test_endpoint_rule_double_wildcard_middle ... ok
test config::tests::test_endpoint_rule_percent_encoded_full_segment ... ok
test config::tests::test_endpoint_rule_exact_path ... ok
test config::tests::test_endpoint_rule_percent_encoded_invalid_utf8 ... ok
test config::tests::test_endpoint_rule_percent_encoded_path_decoded ... ok
test config::tests::test_endpoint_rule_serde_default ... ok
test config::tests::test_endpoint_rule_serde_roundtrip ... ok
test config::tests::test_endpoint_rule_method_wildcard ... ok
test config::tests::test_endpoint_rule_root_path ... ok
test config::tests::test_endpoint_rule_strips_query_string ... ok
test config::tests::test_endpoint_rule_single_wildcard ... ok
test config::tests::test_external_proxy_config_with_bypass_hosts ... ok
test config::tests::test_external_proxy_config_bypass_hosts_default_empty ... ok
test config::tests::test_no_proxy_entry_overlaps_host_pattern_wildcard_and_non_overlap ... ok
test config::tests::test_no_proxy_host_pattern_matches_only_explicit_suffix_semantics ... ok
test config::tests::test_no_proxy_overlap_models_common_client_bare_suffixes_for_protected_hosts ... ok
test config::tests::test_oauth2_config_default_scope ... ok
test config::tests::test_resolved_credential_format_authorization_case_insensitive ... ok
test config::tests::test_oauth2_config_deserialization ... ok
test config::tests::test_route_config_credential_format_omitted_is_none ... ok
test config::tests::test_route_config_explicit_bearer_on_custom_header_preserved ... ok
test config::tests::test_route_config_with_aws_auth_deserializes ... ok
test config::tests::test_route_config_with_oauth2 ... ok
test config::tests::test_route_config_with_full_aws_auth_deserializes ... ok
test config::tests::test_spiffe_absent_by_default ... ok
test config::tests::test_route_config_without_oauth2 ... ok
test config::tests::test_endpoint_rule_trailing_slash_normalized ... ok
test config::tests::test_spiffe_jwt_config_roundtrip ... ok
test config::tests::test_tls_ca_serde_roundtrip ... ok
test config::tests::test_validate_no_proxy_entry_accepts_host_patterns ... ok
test config::tests::test_validate_no_proxy_entry_rejects_non_host_values ... ok
test connect::tests::test_parse_connect_custom_port ... ok
test connect::tests::test_parse_connect_malformed ... ok
test connect::tests::test_parse_connect_without_port ... ok
test connect::tests::test_parse_connect_with_port ... ok
test connect::tests::test_validate_proxy_auth_invalid ... ok
test connect::tests::test_validate_proxy_auth_missing ... ok
test connect::tests::test_validate_proxy_auth_valid ... ok
test connect::tests::write_upstream_failure_round_trips_timeout_reason ... ok
test connect::tests::write_upstream_failure_records_audit_entry ... ok
test connect::tests::write_upstream_failure_sanitises_crlf_in_reason ... ok
test connect::tests::write_upstream_failure_without_audit_log_still_writes_response ... ok
test connect::tests::write_upstream_failure_sends_502_status_line ... ok
test credential::tests::test_empty_credential_store ... ok
test credential::tests::test_get_spiffe_assertion_none_when_unconfigured ... ok
test credential::tests::test_load_no_credential_routes ... ok
test credential::tests::test_load_non_authorization_header_omitted_format_injects_bare_secret ... ok
test credential::tests::test_load_non_authorization_header_explicit_bearer_format ... ok
test credential::tests::test_load_route_spiffe_only_is_visible_in_loaded_prefixes ... ok
test credential::tests::test_load_route_without_spiffe_absent_from_declared_spiffe_prefixes ... ok
test credential::tests::test_loaded_credential_debug_redacts_secrets ... ok
test diagnostic::tests::proxy_diagnostic_serializes_stable_code ... ok
test external::tests::test_bypass_matcher_bare_star_is_not_wildcard ... ok
test external::tests::test_bypass_matcher_empty ... ok
test external::tests::test_bypass_matcher_case_insensitive ... ok
test external::tests::test_bypass_matcher_exact ... ok
test external::tests::test_bypass_matcher_mixed ... ok
test external::tests::test_bypass_matcher_no_match ... ok
test external::tests::test_bypass_matcher_star_dot_only_is_ignored ... ok
test external::tests::test_bypass_matcher_star_without_dot_is_literal ... ok
test external::tests::test_bypass_matcher_wildcard ... ok
test external::tests::test_bypass_matcher_wildcard_case_insensitive ... ok
test external::tests::test_parse_connect_target ... ok
test external::tests::test_parse_status_code_200 ... ok
test external::tests::test_parse_status_code_403 ... ok
test external::tests::test_parse_status_code_malformed ... ok
test filter::tests::test_proxy_filter_allow_all ... ok
test filter::tests::test_proxy_filter_allows_private_networks ... ok
test filter::tests::test_proxy_filter_delegates_to_host_filter ... ok
test filter::tests::test_proxy_filter_denies_aws_ipv6_metadata_literals ... ok
test filter::tests::test_proxy_filter_denies_link_local ... ok
test filter::tests::test_proxy_filter_denies_resolved_aws_ipv6_metadata_ip ... ok
test filter::tests::test_proxy_filter_with_denied_hosts ... ok
test filter::tests::test_proxy_filter_with_denied_hosts_wildcard ... ok
test oauth2::tests::test_build_token_request_body ... ok
test oauth2::tests::test_build_token_request_body_no_scope ... ok
test oauth2::tests::test_parse_status_code_200 ... ok
test oauth2::tests::test_parse_status_code_401 ... ok
test oauth2::tests::test_parse_status_code_garbage ... ok
test oauth2::tests::test_parse_token_response_missing_access_token_errors ... ok
test oauth2::tests::test_parse_token_response_missing_expires_defaults ... ok
test oauth2::tests::test_parse_token_response_non_json_errors ... ok
test oauth2::tests::test_parse_token_response_success ... ok
test oauth2::tests::test_validate_token_url_scheme_accepts_https ... ok
test oauth2::tests::test_validate_token_url_scheme_accepts_loopback_http ... ok
test oauth2::tests::test_token_cache_returns_valid_token ... ok
test oauth2::tests::test_validate_token_url_scheme_rejects_non_loopback_http ... ok
test oauth2::tests::test_validate_token_url_scheme_rejects_other_scheme ... ok
test pool::tests::test_pinned_resolver_clone_shares_pins ... ok
test pool::tests::test_pinned_resolver_new_is_empty ... ok
test pool::tests::test_pinned_resolver_pin_and_retrieve ... ok
test reverse::capture_egress_tests::capture_egress_never_substitutes_real_token_for_unadmitted_consumer ... ok
test reverse::capture_egress_tests::capture_egress_resolves_admitted_phantom_in_request_body ... ok
test reverse::capture_egress_tests::resolve_capture_request_body_forwards_unchanged_when_not_applicable ... ok
test reverse::capture_relay_tests::capture_buffer_cap_exceeded_denies_response ... ok
test pool::tests::test_upstream_pool_new_creates_resolver ... ok
test reverse::capture_relay_tests::capture_decode_chunked_body_fails_closed_on_chunk_size_overflow ... ok
test pool::tests::test_upstream_pool_pin_host_stores_addresses ... ok
test reverse::capture_relay_tests::capture_decode_chunked_body_fails_closed_on_missing_terminating_crlf ... ok
test reverse::capture_relay_tests::capture_decode_chunked_body_fails_closed_on_non_hex_chunk_size ... ok
test reverse::capture_relay_tests::capture_decode_chunked_body_reassembles_two_chunks ... ok
test reverse::capture_egress_tests::capture_egress_resolution_reaches_upstream_on_live_dispatch_path ... ok
test reverse::capture_relay_tests::capture_denies_content_encoded_response ... ok
test reverse::capture_relay_tests::capture_fails_closed_on_unrewritten_token_field ... ok
test reverse::capture_relay_tests::capture_find_header_end_locates_boundary ... ok
test reverse::capture_relay_tests::capture_find_header_end_none_when_headers_incomplete ... ok
test reverse::capture_relay_tests::capture_parse_response_headers_detects_chunked ... ok
test reverse::capture_relay_tests::capture_parse_response_headers_detects_content_encoding_regardless_of_status ... ok
test reverse::capture_relay_tests::capture_deny_reason_never_contains_response_derived_bytes ... ok
test reverse::capture_relay_tests::capture_parse_response_headers_ignores_empty_content_encoding ... ok
test reverse::capture_relay_tests::capture_read_capped_response_denies_when_cap_exceeded_with_no_content_length ... ok
test reverse::capture_relay_tests::capture_read_capped_response_succeeds_within_cap ... ok
test reverse::capture_relay_tests::capture_rewrites_configured_fields ... ok
test reverse::capture_relay_tests::capture_rewrites_via_spiffe_route_site ... ok
test reverse::capture_relay_tests::capture_rewrites_via_spiffe_assertion_site ... ok
test reverse::capture_relay_tests::capture_relay_terminates_without_upstream_eof ... ok
test reverse::capture_relay_tests::non_capture_route_still_streams_unbuffered ... ok
test reverse::capture_relay_tests::relay_capture_if_declared_returns_none_when_capture_absent ... ok
test reverse::tests::test_extract_content_length ... ok
test reverse::tests::test_extract_content_length_missing ... ok
test reverse::tests::test_filter_headers_removes_custom_header ... ok
test reverse::tests::test_filter_headers_removes_host_auth ... ok
test reverse::tests::test_filter_headers_removes_x_api_key ... ok
test reverse::tests::test_parse_request_line ... ok
test reverse::tests::test_parse_request_line_malformed ... ok
test reverse::tests::test_parse_response_status_200 ... ok
test reverse::tests::test_parse_response_status_404 ... ok
test reverse::tests::test_parse_response_status_empty ... ok
test reverse::tests::test_parse_response_status_garbage ... ok
test reverse::tests::test_parse_response_status_partial ... ok
test reverse::tests::test_parse_service_prefix ... ok
test reverse::tests::test_parse_service_prefix_no_subpath ... ok
test reverse::tests::test_parse_upstream_url_http_with_port ... ok
test reverse::tests::test_parse_upstream_url_https ... ok
test reverse::tests::test_parse_upstream_url_invalid_scheme ... ok
test reverse::tests::test_parse_upstream_url_no_path ... ok
test reverse::tests::test_transform_query_param_add_to_existing_query ... ok
test reverse::tests::test_transform_query_param_add_to_no_query ... ok
test reverse::tests::test_transform_query_param_replace_existing ... ok
test reverse::tests::test_transform_query_param_url_encodes_special_chars ... ok
test reverse::tests::test_transform_url_path_basic ... ok
test reverse::tests::test_transform_url_path_different_replacement ... ok
test reverse::tests::test_transform_url_path_no_trailing_slash ... ok
test reverse::tests::test_validate_phantom_token_bearer_invalid ... ok
test reverse::tests::test_validate_phantom_token_bearer_valid ... ok
test reverse::tests::test_validate_phantom_token_case_insensitive_header ... ok
test reverse::tests::test_validate_phantom_token_in_path_invalid ... ok
test reverse::tests::test_validate_phantom_token_in_path_missing ... ok
test reverse::tests::test_validate_phantom_token_in_path_valid ... ok
test reverse::tests::test_validate_phantom_token_in_query_invalid ... ok
test reverse::tests::test_validate_phantom_token_in_query_missing_param ... ok
test reverse::tests::test_validate_phantom_token_in_query_no_query_string ... ok
test reverse::tests::test_validate_phantom_token_in_query_url_encoded ... ok
test reverse::tests::test_validate_phantom_token_in_query_valid ... ok
test reverse::tests::test_validate_phantom_token_missing ... ok
test reverse::tests::spiffe_route_denies_missing_session_token_before_credential_acquisition ... ok
test reverse::tests::test_validate_phantom_token_x_api_key_valid ... ok
test reverse::tests::denied_endpoint_returns_403_and_audit ... ok
test reverse::tests::test_validate_phantom_token_x_goog_api_key_valid ... ok
test route::tests::cr01_load_rejects_crlf_bearing_route_inject_header ... ok
test route::tests::boundary_validation_does_not_over_reject_valid_routes ... ok
test route::tests::cr01_load_rejects_crlf_bearing_spiffe_credential_format ... ok
test route::tests::cr01_load_rejects_crlf_bearing_spiffe_inject_header ... ok
test route::tests::host_port_matches_exact_mismatch ... ok
test route::tests::host_port_matches_exact_match ... ok
test route::tests::host_port_matches_wildcard_different_label ... ok
test route::tests::host_port_matches_no_wildcard_no_match ... ok
test route::tests::host_port_matches_wildcard_does_not_match_apex ... ok
test route::tests::host_port_matches_wildcard_matches_multi_label_prefix ... ok
test route::tests::host_port_matches_wildcard_port_mismatch ... ok
test route::tests::host_port_matches_wildcard_single_label ... ok
test route::tests::allow_domain_endpoint_route_does_not_shadow_credential_route ... ok
test route::tests::test_build_tls_connector_missing_file ... ok
test route::tests::test_capture_declared_for_upstream_false_for_non_capture_route ... ok
test route::tests::test_capture_declared_for_upstream_true_across_different_port ... ok
test route::tests::test_empty_route_store ... ok
test route::tests::test_extract_host_port_http ... ok
test route::tests::test_extract_host_port_https ... ok
test route::tests::test_extract_host_port_normalises_case ... ok
test route::tests::test_extract_host_port_preserves_wildcard_host ... ok
test route::tests::test_extract_host_port_with_port ... ok
test route::tests::test_host_only_matches_different_host_no_match ... ok
test route::tests::test_host_only_matches_same_host_different_port ... ok
test route::tests::test_host_only_matches_wildcard_does_not_match_apex ... ok
test route::tests::test_host_only_matches_wildcard_host_ignores_port ... ok
test route::tests::test_host_port_matches_wildcard_subdomain_only ... ok
test route::tests::test_is_route_upstream ... ok
test route::tests::test_load_routes_normalises_prefix ... ok
test route::tests::test_load_routes_rejects_malformed_or_unsupported_upstreams ... ok
test route::tests::test_load_routes_without_credentials ... ok
test route::tests::test_load_routes_without_spiffe_has_no_spiffe_source ... ok
test reverse::capture_relay_tests::capture_read_capped_response_denies_on_timeout ... ok
test route::tests::test_build_tls_connector_with_valid_ca ... ok
test route::tests::test_loaded_route_debug ... ok
test route::tests::test_route_upstream_hosts ... ok
test route::tests::test_spiffe_declared_for_upstream_false_for_non_spiffe_route ... ok
test route::tests::wr03_load_rejects_capture_max_response_bytes_above_ceiling ... ok
test server::tests::classify_request_target_detects_absolute_and_origin_forms ... ok
test route::tests::wr03_load_rejects_the_other_three_capture_gates ... ok
test route::tests::test_build_tls_connector_empty_pem ... ok
test server::tests::connect_denies_capture_declared_route_upstream_on_different_port ... ok
test server::tests::connect_denies_capture_declared_route_upstream ... ok
test pool::tests::test_upstream_pool_send_returns_error_for_invalid_host ... ok
test server::tests::d03_connect_denies_spiffe_declared_route_upstream ... ok
test server::tests::d03_forward_http_denies_spiffe_declared_route_upstream ... ok
test server::tests::forward_http_allowed_host_is_forwarded ... ok
test server::tests::forward_http_denies_capture_declared_route_upstream ... ok
test server::tests::forward_http_denies_capture_declared_route_upstream_on_different_port ... ok
test server::tests::forward_http_does_not_inject_managed_credential ... ok
test server::tests::forward_http_missing_proxy_auth_is_rejected_407 ... ok
test server::tests::connect_non_capture_route_on_different_port_is_not_blocked_by_the_guard ... ok
test server::tests::forward_http_non_capture_route_still_forwards ... ok
test server::tests::forward_https_absolute_form_is_rejected_with_connect_guidance ... ok
test server::tests::forward_http_requires_auth_by_default ... ok
test server::tests::no_proxy_pipeline_appends_validated_entry_and_emits_canonical_var ... ok
test server::tests::no_proxy_pipeline_clears_loopback_when_managed_loopback_upstream ... ok
test server::tests::no_proxy_pipeline_preserves_localhost_and_loopback_bypass ... ok
test server::tests::rewrite_absolute_to_origin_form_produces_origin_line ... ok
test server::tests::start_allows_no_auth_on_loopback_and_auth_on_any_bind ... ok
test server::tests::forward_http_denied_host_returns_403_and_audits ... ok
test server::tests::start_rejects_no_auth_on_non_loopback_bind ... ok
test server::tests::start_rejects_no_proxy_entry_conflicting_with_route_upstream ... ok
test server::tests::start_rejects_no_proxy_entry_overlapping_allowed_host ... ok
test server::tests::start_rejects_no_proxy_entry_with_invalid_grammar ... ok
test server::tests::strip_proxy_headers_removes_proxy_hop_by_hop_only ... ok
test server::tests::test_anthropic_credential_phantom_token_regression ... ok
test server::tests::origin_form_request_still_routes_to_reverse_proxy ... ok
test server::tests::test_no_proxy_empty_when_no_non_credential_hosts ... ok
test server::tests::test_no_proxy_excludes_credential_upstreams ... ok
test server::tests::test_no_proxy_includes_hosts_with_matching_connect_port ... ok
test server::tests::test_no_proxy_empty_without_direct_connect_ports ... ok
test server::tests::test_proxy_credential_env_vars_fallback_to_uppercase_key ... ok
test server::tests::test_proxy_credential_env_vars_skips_unloaded_routes ... ok
test server::tests::test_proxy_credential_env_vars_strips_slashes ... ok
test server::tests::test_proxy_credential_env_vars_with_explicit_env_var ... ok
test server::tests::test_proxy_credential_env_vars ... ok
test server::tests::test_proxy_env_vars ... ok
test server::tests::test_route_spiffe_visible_in_env_vars_and_diagnostics ... ok
test server::tests::test_smart_no_proxy_entry_filters_ambiguous_bare_domains ... ok
test server::tests::test_proxy_starts_and_binds ... ok
test server::tests::forward_http_honours_no_auth ... ok
test token::tests::test_constant_time_eq_different ... ok
test token::tests::test_constant_time_eq_different_length ... ok
test token::tests::test_constant_time_eq_empty ... ok
test token::tests::test_constant_time_eq_same ... ok
test token::tests::test_generate_token_is_hex ... ok
test token::tests::test_generate_token_length ... ok
test token::tests::test_generate_token_unique ... ok
test token::tests::test_validate_proxy_auth_basic ... ok
test token::tests::test_validate_proxy_auth_basic_any_username ... ok
test token::tests::test_validate_proxy_auth_basic_wrong_password ... ok
test token::tests::test_validate_proxy_auth_bearer ... ok
test token::tests::test_validate_proxy_auth_bearer_case_insensitive ... ok
test token::tests::test_validate_proxy_auth_bearer_invalid ... ok
test token::tests::test_validate_proxy_auth_missing ... ok
test server::tests::test_strict_filter_with_empty_allowlist_denies_connect ... ok
test spiffe::tests::test_jwt_source_fails_closed_on_missing_socket ... ok
test oauth2::tests::test_token_cache_detects_expiry ... ok
test connect::tests::connect_keeps_open_on_missing_proxy_auth ... ok
test credential::tests::test_load_spiffe_assertion_route_unreachable_socket_skips_route_not_startup ... ok
test route::tests::test_load_spiffe_route_unreachable_socket_fails_whole_load ... ok
test server::tests::start_fails_closed_on_unreachable_spiffe_socket ... ok

test result: ok. 307 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.19s

     Running tests\spiffe_integration.rs (target\debug\deps\spiffe_integration-493b377efd16c97b.exe)

running 6 tests
test d03_connect_and_forward_http_deny_spiffe_declared_route_upstream_end_to_end ... ok
test test_spiffe_jwt_live_fetch ... ok
test test_spiffe_jwt_live_delegation_none_on_plain_svid ... ok
test test_spiffe_jwt_live_proxy_startup ... ok
test d17_spiffe_dispatch_host_check_precedes_mint_structurally ... ok
test test_spiffe_jwt_fails_closed_on_missing_socket ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.02s

     Running unittests src\main.rs (target\debug\deps\nono_shell_broker-0d4787a724fa03b3.exe)

running 34 tests
test broker::broker_resume_gate_tests::app_container_required_on_a_non_app_container_shape_refuses_resume ... ok
test broker::broker_resume_gate_tests::app_container_sid_must_match_the_expected_per_run_value ... ok
test broker::broker_resume_gate_tests::tightened_confirmed_app_container_probe_permits_resume ... ok
test broker::broker_resume_gate_tests::mandatory_integrity_label_is_attested_and_can_fail ... ok
test broker::broker_resume_gate_tests::required_layers_list_is_comma_split_and_trimmed ... ok
test broker::broker_resume_gate_tests::tightened_unconfirmed_app_container_probe_refuses_resume ... ok
test broker::broker_resume_gate_tests::empty_required_layers_refuses_resume ... ok
test broker::broker_resume_gate_tests::broker_resume_gate_latency_with_real_probes ... ok
test broker::broker_resume_gate_tests::unattestable_required_layer_refuses_resume ... ok
test broker::build_command_line_tests::build_command_line_appends_simple_args ... ok
test broker::build_command_line_tests::build_command_line_doubles_embedded_quotes ... ok
test broker::build_command_line_tests::build_command_line_quotes_shell_path ... ok
test broker::build_command_line_tests::build_command_line_quotes_args_with_whitespace ... ok
test broker::build_command_line_tests::build_command_line_terminates_with_null ... ok
test broker::dedup_handles_tests::dedup_collapses_merged_stdout_stderr_duplicate ... ok
test broker::dedup_handles_tests::dedup_preserves_unique_list_unchanged ... ok
test broker::dedup_handles_tests::dedup_single_handle_unchanged ... ok
test broker::env_wire_contract_tests::absent_required_layers_env_var_is_fail_closed ... ok
test broker::env_wire_contract_tests::required_layers_env_var_tightens_the_gate ... ok
test broker::parse_args_tests::parse_args_empty_app_container_name_returns_error ... ok
test broker::parse_args_tests::parse_args_empty_inherit_handle_list_returns_error ... ok
test broker::parse_args_tests::parse_args_invalid_handle_value_inherit_handle_returns_error ... ok
test broker::parse_args_tests::parse_args_invalid_hex_inherit_handle_returns_error ... ok
test broker::parse_args_tests::parse_args_dangling_flag_value_returns_error ... ok
test broker::parse_args_tests::parse_args_missing_cwd_returns_error ... ok
test broker::parse_args_tests::parse_args_missing_shell_returns_error ... ok
test broker::parse_args_tests::parse_args_app_container_name_parsed ... ok
test broker::parse_args_tests::parse_args_multiple_inherit_handles_accumulate ... ok
test broker::parse_args_tests::parse_args_no_pty_absent_defaults_false ... ok
test broker::parse_args_tests::parse_args_no_pty_flag_accepted ... ok
test broker::parse_args_tests::parse_args_no_pty_without_app_container_name_returns_error ... ok
test broker::parse_args_tests::parse_args_null_inherit_handle_returns_error ... ok
test broker::parse_args_tests::parse_args_shell_arg_preserves_order ... ok
test broker::parse_args_tests::parse_args_unknown_flag_returns_error ... ok

test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src\main.rs (target\debug\deps\sign_fixture-8853eed6ba0f9ad2.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests nono_fltmgr_client

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests nono

running 10 tests
test crates\nono\src\lib.rs - (line 14) - compile ... ok
test crates\nono\src\keystore.rs - keystore::load_secrets (line 252) - compile ... ok
test crates\nono\src\supervisor\socket_windows.rs - supervisor::socket::SupervisorSocket::recv_message_with_timeout (line 421) - compile ... ok
test crates\nono\src\capability.rs - capability::CapabilitySet (line 909) - compile ... ok
test crates\nono\src\sandbox\mod.rs - sandbox::Sandbox (line 638) - compile ... ok
test crates\nono\src\trust\signing.rs - trust::signing::validate_oidc_issuer (line 60) - compile ... ok
test crates\nono\src\supervisor\mod.rs - supervisor::ApprovalBackend (line 88) ... ok
test crates\nono\src\keystore.rs - keystore::build_mappings_from_list (line 2161) ... ok
test crates\nono\src\manifest.rs - manifest (line 12) ... ok
test crates\nono\src\trust\types.rs - trust::types::TrustPolicy (line 53) ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.24s

   Doc-tests nono_proxy

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 5 targets failed:
    `-p nono-sandbox-cli --bin nono`
    `-p nono-sandbox-cli --test audit_attestation`
    `-p nono-sandbox-cli --test env_vars`
    `-p nono-sandbox-cli --test layer_registry_selfcheck`
    `-p nono-sandbox-cli --test resl_nix_async_signal_safety`

=== cargo exit code: 0 ===
=== finished: 2026-08-12T04:16:38Z
```
