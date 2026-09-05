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

## ADJUDICATION of the three integration failures (Plan 118-10 close-out, 2026-09-05)

**Verdict: all three are PRE-EXISTING and none is a Phase 118 regression.** Adjudicated by
structural evidence rather than a phase-base rebuild — the reasoning below is stronger than a
single cold-build comparison because it identifies each failure's actual cause.

### Why they were absent from the documented baseline (the thing that made them look new)

The "12 known failures" baseline is scoped to `-p nono-sandbox-cli --bin nono` — unit tests
compiled INTO the bin target. All three of these live in separate integration-test binaries
(`--test audit_attestation`, `--test env_vars`) that no prior baseline sweep ever reached, and
`cargo test` is fail-fast ACROSS targets, so a plain run stops before them. 118-04's
`--no-fail-fast` sweep was simply the first run that got that far. Their absence from the baseline
reflects never having been measured, not having recently broken.

### 1 + 2. `audit_verify_reports_signed_attestation_with_pinned_public_key` and `rollback_signed_session_verifies_from_audit_dir_bundle`

**Cause: unportable test fixture. Both invoke `/bin/pwd`** (`audit_attestation.rs:147,209`), a
Unix path that cannot exist on Windows. Failure is
`nono: Command execution failed: /bin/pwd: cannot find binary path`, both panicking at the same
shared `assert_success` helper (`audit_attestation.rs:24`).

Both are plain `#[test]` with **zero platform gating** (`cfg(unix)`/`cfg(target_os)` count in that
file: 0). They therefore run on Windows and **cannot ever have passed here**. The file was last
modified 2026-06-24 and was NOT touched during the phase window (`git log 2359c841..HEAD` on it is
empty). No rebuild is needed to establish pre-existence — it follows from construction.

Same defect class as Cluster B of quick task `260815-gfd` (hardcoded `/tmp` → `C:\tmp`), which
fixed that class selectively and did not reach these two.

### 3. `windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified`

**Cause: the test's own fixture layout now trips a fail-closed guard.** It grants a temp dir while
placing `fake-localappdata\nono` INSIDE it, so nono correctly refuses:
`Refusing to grant '...\.tmpToYBj8' (source: CLI) because it overlaps protected nono state root
'...\.tmpToYBj8\fake-localappdata\nono'.` The product is behaving correctly and fail-closed; the
fixture is what is wrong.

Neither side changed during the phase: `env_vars.rs` is untouched in `2359c841..HEAD`, and all
three files that can emit that refusal (`capability_ext.rs`, `exec_strategy.rs`,
`protected_paths.rs`) are likewise untouched in that window.

**HYPOTHESIS, not established:** the test was added 2026-04-09/10, and `protected_paths.rs` last
changed 2026-06-20 (`de553185`, "apply Windows CI fixes for XDG cherry-picks") — i.e. the guard
moved AFTER the test was written. That ordering would explain the test having passed once and
breaking silently in June, unnoticed for ~2.5 months because no sweep reached integration binaries.
Confirming this needs a run at `de553185^` vs `de553185`; it has NOT been run, and the verdict
above does not depend on it.

### Disposition — NOT fixed by Phase 118, and the fix is a scope decision

None of the three is fixed here: all predate the phase, none touches its surface, and repairing
them is test-fixture work outside a receipts phase. Options for whoever picks them up:

- **1 + 2:** either `#[cfg(unix)]`-gate them (honest, cheap, but silently drops Windows
  audit-attestation integration coverage) or make the invoked binary portable (restores coverage,
  more work). Gating without recording the coverage loss would repeat the "documented scope
  decision hiding a gap" anti-pattern this phase already hit once.
- **3:** move `fake-localappdata` OUTSIDE the granted directory so the fixture stops overlapping
  the protected state root.
