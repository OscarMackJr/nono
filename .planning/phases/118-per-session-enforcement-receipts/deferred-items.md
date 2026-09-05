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

## Discovered during Plan 118-10 (phase gate)

### 1. `cargo clippy --workspace` is RED — pre-existing, blocks `make ci`

Two `dead_code` errors, escalated by `-D warnings`, in BOTH the `nono` and `nono-agentd` binary
targets:

- `crates/nono-cli/src/receipt_sink.rs:315` — field `ReceiptWriter.session_id` is never read
- `crates/nono-cli/src/receipt_sink.rs:399,405` — methods `ReceiptWriter::session_id()` and
  `ReceiptWriter::file_path()` are never used

NOT caused by Plan 118-10 (which is docs-only) nor by the 260904-wkv quick task that first surfaced
it: nothing in `nono-cli`, `nono-proxy`, `nono-shell-broker`, or `bindings/` references
`session_credential`, and isolating clippy to `nono-sandbox-cli` alone reproduces it identically.

**Disposition needed at close-out, not deferred past it.** All six waves of Phase 118's code work
are complete, so these accessors are not pending a later plan — they appear genuinely unused.
CLAUDE.md's standard applies: remove them, or write tests that use them. `#[allow(dead_code)]` is
explicitly discouraged by the same rule. `make clippy` / `make ci` cannot pass until this is
settled.

### 2. Sink DENY ACEs accumulate without bound

`ensure_sink_guarded` adds a DENY ACE for each launch's synthetic session SID (and, on the daemon
arm, package SID) to the SHARED sink directory, and nothing ever revokes them. Observed growing
5 → 7 across two runs during the Task 3 checks; every entry is a distinct `S-1-5-117-*`.

Not a correctness defect today — a stale session SID is never reused, so the extra ACEs are inert.
But the DACL grows one ACE per session for the life of the host, and Windows caps an ACL at 64KB.
On a long-lived fleet machine running per-tool-call hook sessions, this is a real ceiling. No
cleanup path exists (`nono receipt cleanup` was deliberately not built — see `receipt_sink.rs`'s
retention-policy note).

### 3. Receipt read-permeability — RESOLVED as a scope narrowing, recorded here for cross-reference

Task 3 established that a confined child can READ every receipt in the sink on both arms. The
operator's disposition was to narrow D-08's claim to write integrity and amend the docs rather than
widen the guard. This is **not** an open item — it is a recorded, deliberate scope boundary (see
`118-10-SUMMARY.md` and the "D-08 SCOPE CORRECTION" section of `receipt_sink.rs`).

Logged here only so a future reader searching deferred items finds the pointer. If read
confidentiality is ever claimed, BOTH `ensure_sink_guarded` call sites must change together —
`exec_strategy_windows/launch.rs` passes `(session_sid, None)` and `agent_daemon/launch.rs` passes
`(None, Some(package_sid))`; neither passes both.
