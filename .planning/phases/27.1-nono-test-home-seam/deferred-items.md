# Deferred Items — Phase 27.1

Out-of-scope discoveries during Phase 27.1 execution. These pre-existing issues
are NOT caused by Phase 27.1 changes and are deferred for future cleanup.

## Pre-existing clippy errors in `crates/nono-cli/src/exec_strategy_windows/supervisor.rs`

**Discovered during:** Plan 27.1-01 Task 1 (clippy verification)
**Status:** Pre-existing on base commit `18e8e4ea` (verified)
**Errors:**
- `supervisor.rs:788:45` — `collapsible_match` clippy lint
- `supervisor.rs:800:45` — `collapsible_match` clippy lint

These trigger `-D warnings` clippy failures. Verified pre-existing (not caused by
Phase 27.1 changes) by inspection of the unmodified file. Phase 27.1 acceptance
criteria's `cargo clippy -p nono-cli -- -D warnings` requirement is satisfied
for the changed file (`crates/nono-cli/src/config/mod.rs`) — the failures are
in unrelated files.

**Recommended follow-up:** Quick task to apply the clippy `collapsible_match`
suggestions in `supervisor.rs`. Estimated <15 minutes. Not blocking 27.1.

## Pre-existing clippy errors in `crates/nono/src/manifest.rs`

**Discovered during:** Plan 27.1-01 Task 1 (full workspace clippy)
**Errors:**
- `manifest.rs:95` — `collapsible_match` clippy lint
- `manifest.rs:103` — `collapsible_match` clippy lint

Out of scope per D-19 invariant (`crates/nono/` byte-identical). Cannot fix in
Phase 27.1.

**Recommended follow-up:** Address in a `crates/nono/`-targeted housekeeping
plan post-v2.3.

## Pre-existing clippy errors in `crates/nono-cli/src/audit_commands.rs` test module

**Discovered during:** Plan 27.1-03 Task 3 (running `cargo clippy -p nono-cli --tests`)
**Status:** Pre-existing on commit `6275cfb1` (verified by `git stash` round-trip)
**Errors:** ~15 errors including:
- `audit_commands.rs:854` — `useless_vec` (test-fixture vec! → array suggestion)
- Multiple `collapsible_if`, `format_args` style lints
- `audit_session.rs:333` — unused imports `RollbackStatus`, `SessionMetadata`

These trigger `-D warnings` clippy failures on `cargo clippy --tests`. Out of scope
per the executor SCOPE BOUNDARY rule (not caused by Plan 03 changes; the
audit_attestation integration test target itself is clippy-clean).

**Recommended follow-up:** Combine with the `supervisor.rs` follow-up into a single
`crates/nono-cli/`-wide clippy cleanup task (~15-30 min). Not blocking 27.1.

## Phase 27.1 Plan 03 v2.4 production follow-ups (Blocker 3 resurfaced)

**Discovered during:** Plan 27.1-03 Task 3 (Windows host verification)
**Status:** Surfaced; tests re-#[ignore]'d per D-27.1-14 contingency

### v2.4-FU-1: Wire `audit_session::load_session` into `audit_commands.rs`

**Issue:** `crates/nono-cli/src/audit_commands.rs:12` imports `load_session` from
`crate::rollback_session`, which only inspects `<home>/.nono/rollbacks/<id>/`.
Audit-only sessions (created by `--audit-integrity` without `--rollback`) live at
`<home>/.nono/audit/<id>/` and are unfindable by `nono audit verify` and
`nono audit show`. The audit-aware loader at `audit_session.rs:160` already
implements correct dual-root semantics but is gated behind `#[allow(dead_code)]`.

**Fix:** Swap the import in `audit_commands.rs` from `rollback_session` to
`audit_session`, then remove the `#[allow(dead_code)]` attributes from
`audit_session::{discover_sessions, load_session, remove_session, SessionInfo}`.
This is a small change but DOES alter `audit list/cleanup` semantics (they would
discover audit-only sessions they previously missed — generally correct, but
requires test updates).

**Estimated effort:** 30-60 min including unit tests for the changed code paths.

### v2.4-FU-2: Decide bundle target for `--rollback --audit-sign-key` sessions

**Issue:** `audit_attestation::sign_session_attestation` writes the bundle to
`session_dir`, which is `<rollback>/<id>/` when `rollback_active`. Test 2
(`rollback_signed_session_verifies_from_audit_dir_bundle`) asserts the bundle
should be at `<audit>/<id>/audit-attestation.bundle`. Either:
- (a) Mirror the bundle to audit_dir at sign time, OR
- (b) Make `audit verify` look up bundles in both roots, OR
- (c) Update the test to look in the rollback dir for the bundle.

The test name suggests the design intent was (a). This is non-trivial production
architecture; needs a design decision and impact analysis (does the bundle in
audit_dir survive rollback cleanup? Should audit_verify check both roots even
when v2.4-FU-1 lands? etc.).

**Estimated effort:** 1-2 hours including the design decision and test updates.

### v2.4-FU-3: Re-enable both audit-attestation tests after v2.4-FU-1 + v2.4-FU-2

After both production fixes land, the tests can be re-enabled. The
`#[ignore = "..."]` attributes (and the 49-line comment block above Test 1) can
be removed. The `setup_isolated_home` directory pre-creation should remain (it's
defensive against future canonicalize-before-exists patterns).

**Estimated effort:** 15 min including running the suite on a Windows host.
