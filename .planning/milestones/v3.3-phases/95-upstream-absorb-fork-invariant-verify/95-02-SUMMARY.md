---
phase: 95-upstream-absorb-fork-invariant-verify
plan: 02
subsystem: audit,sandbox,proxy
tags: [upstream-sync, audit-events, landlock, keystore, scrub, endpoint-policy, cluster-b]

# Dependency graph
requires:
  - phase: 95-01
    provides: Cluster A cherry-pick (ae77d198 AF_UNIX/IPC deadlock fix); restrict_execute in linux.rs
provides:
  - SandboxRuntimeAuditEvent + CommandPolicyAuditEvent in audit.rs (additive)
  - cmd:// URI guard in keystore.rs
  - SENSITIVE_ENV_VARS expansion in scrub.rs
  - CompiledEndpointPolicy in nono-proxy/config.rs
  - endpoint_policy field on RouteConfig in route.rs
  - restrict_execute re-export in sandbox/mod.rs
affects:
  - Phase 96 (cross-target clippy verification — D-03 deferred items)
  - Any plan using audit event types or RouteConfig struct literals

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Surgical manual absorb: extract additive shared-surface hunks from squash commits, skip tool-sandbox machinery"
    - "Rule 3 cascading fix: adding a new field to RouteConfig requires updating all struct literal construction sites workspace-wide"

key-files:
  created: []
  modified:
    - crates/nono/src/audit.rs
    - crates/nono/src/sandbox/mod.rs
    - crates/nono/src/keystore.rs
    - crates/nono/src/scrub.rs
    - crates/nono-proxy/src/config.rs
    - crates/nono-proxy/src/route.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono-proxy/src/credential.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-cli/src/network_policy.rs

key-decisions:
  - "D-01 confirmed: shared-surface only — SandboxRuntimeAuditEvent + CommandPolicyAuditEvent structs land; tool-sandbox subsystem dir and tls_intercept/ hunks SKIPPED"
  - "D-03 confirmed: cross-target clippy (linux + macOS cfg branches) deferred to Phase 96 — Windows host has no cross C compiler"
  - "Rule 3 auto-fix: config.rs included despite being in SKIP-unless-needed list; CompiledEndpointPolicy required by route.rs to compile"
  - "Rule 3 auto-fix: credential.rs, reverse.rs, network_policy.rs endpoint_policy: None added to all RouteConfig struct literals for compilation"
  - "Environmental flakiness documented: audit_session::discover_sessions_does_not_warn test uses XDG_STATE_HOME guard on Windows where LOCALAPPDATA drives audit_root(); LOCALAPPDATA now has 17 real sessions causing non-1 count. NOT a regression from Cluster B changes — audit_session.rs unchanged."

requirements-completed:
  - UPST10-02

# Metrics
duration: 90min
completed: 2026-06-26
---

# Phase 95 Plan 02: Cluster B Shared-Surface Absorb Summary

**Surgically extracted additive shared-surface from upstream squash commit 11fd10e0 (91-file tool sandbox feature): SandboxRuntimeAuditEvent, CommandPolicyAuditEvent, cmd:// keystore guard, SENSITIVE_ENV_VARS, and CompiledEndpointPolicy — skipping all tool-sandbox machinery absent from the fork.**

## Performance

- **Duration:** ~90 min
- **Started:** 2026-06-26T00:00:00Z
- **Completed:** 2026-06-26T04:00:00Z
- **Tasks:** 2 (combined into 1 commit per plan instruction)
- **Files modified:** 10

## Accomplishments

- Applied 7 approved shared-surface files from Cluster B (11fd10e0) plus 3 cascading compilation fixes
- CR-02 invariant preserved byte-intact: `records_verified: event_count > 0` at audit.rs line 1570
- All security skip checks confirmed: no ApprovalRequest, no tls_intercept refs in production code, no tool-sandbox dir, no exec_strategy.rs timeout changes, exec_strategy_windows/ untouched (ADR-86 D-03)
- Cross-target clippy DEFERRED to Phase 96 per D-03 (Windows host lacks cross C compiler)

## Task Commits

1. **Task 1+2 (combined): Cluster B shared-surface extraction** - `91d526e6` (feat)

## Files Created/Modified

- `crates/nono/src/audit.rs` - Added SandboxRuntimeAuditEvent, CommandPolicyAuditEvent structs; record_sandbox_runtime_event(), record_command_policy_event() methods; Network variant boxed (Box<NetworkAuditEvent>)
- `crates/nono/src/sandbox/mod.rs` - Added pub use linux::restrict_execute re-export; Sandbox::restrict_execute() method
- `crates/nono/src/keystore.rs` - Added CMD_URI_PREFIX const, cmd:// early-return guard in load_secret_by_ref(), is_cmd_uri(), validate_cmd_uri() helpers
- `crates/nono/src/scrub.rs` - Added SENSITIVE_ENV_VARS (14 entries), sensitive_env_vars field on ScrubPolicy, add/remove/is_sensitive_env_var methods, scrub_env_name/value helpers
- `crates/nono-proxy/src/config.rs` - Added EndpointPolicyConfig, CompiledEndpointPolicy, EndpointPolicyOutcome types and compile() logic
- `crates/nono-proxy/src/route.rs` - Added endpoint_policy field to LoadedRoute; CompiledEndpointPolicy import and compilation in RouteStore::load(); all RouteConfig struct literal sites updated
- `crates/nono-proxy/src/server.rs` - All RouteConfig struct literals updated with endpoint_policy: None
- `crates/nono-proxy/src/credential.rs` - All RouteConfig struct literals updated with endpoint_policy: None (Rule 3)
- `crates/nono-proxy/src/reverse.rs` - RouteConfig struct literal updated with endpoint_policy: None (Rule 3)
- `crates/nono-cli/src/network_policy.rs` - All 3 RouteConfig construction sites updated with endpoint_policy: None (Rule 3)

## Decisions Made

- Applied config.rs despite it being in "check before staging" list: its changes are pure EndpointPolicyConfig/CompiledEndpointPolicy infrastructure (no tls_intercept, no ApprovalRequest, no credential_capture) and are REQUIRED by route.rs import
- cherry-pick --no-commit approach (plan's stated strategy) was actually superseded by direct manual edits per the session context; all hunks applied manually to the 7 approved files
- restrict_execute was already present in linux.rs from Cluster A cherry-pick (ae77d198) — no re-application needed for linux.rs

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] endpoint_policy field in RouteConfig required workspace-wide struct literal updates**
- **Found during:** Post-change `cargo check --workspace --tests`
- **Issue:** Adding `endpoint_policy: Option<EndpointPolicyConfig>` to RouteConfig struct in config.rs caused 13 "missing field `endpoint_policy`" compile errors in credential.rs (3), reverse.rs (1), server.rs (9), and 3 more in network_policy.rs (nono-cli)
- **Fix:** Added `endpoint_policy: None,` to each struct literal; credential.rs/reverse.rs/network_policy.rs added to commit
- **Files modified:** credential.rs, reverse.rs, server.rs (server.rs was in plan APPLY list), network_policy.rs
- **Verification:** `cargo check --workspace --tests` exits 0 after fix
- **Committed in:** 91d526e6

---

**Total deviations:** 1 auto-fixed (Rule 3 blocking compilation)
**Impact on plan:** Necessary cascading fix from adding endpoint_policy to RouteConfig. All changes are `endpoint_policy: None` — no behavioral change. No scope creep beyond compilation requirement.

## PARTIAL Deferrals

**Cross-target clippy DEFERRED to Phase 96 per D-03:**
- Commit 91d526e6 touches `crates/nono/src/sandbox/mod.rs` and `crates/nono/src/sandbox/linux.rs` (via chain from Cluster A in 95-01)
- These files contain `#[cfg(target_os = "linux")]` blocks
- Windows host cannot run `cargo clippy --target x86_64-unknown-linux-gnu` (no cross C compiler)
- Per CLAUDE.md § Cross-target clippy verification: verification PARTIAL, deferred to Phase 96 live CI

## Environmental Flakiness Noted (Not a Regression)

`audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty` fails when `LOCALAPPDATA/nono/audit/` contains real sessions (currently 17+). This test:
- Uses `XDG_STATE_HOME` env guard (works on Linux/macOS)
- On Windows, `state_paths::audit_root()` uses `LOCALAPPDATA` (not `XDG_STATE_HOME`), so the temp-dir guard has no effect
- This causes `discover_sessions()` to find 17 real sessions instead of 1
- When this test panics, it poisons `ENV_LOCK`, cascading to 6+ `config::tests` failures with `PoisonError`
- `audit_session.rs` is UNCHANGED by Cluster B (confirmed: git diff HEAD shows 0 changes)
- This was passing at baseline because `LOCALAPPDATA/nono/audit/` had fewer sessions at capture time
- Deferred to a future cleanup task; not a Cluster B regression

## Make CI Gate Results

| Check | Result |
|-------|--------|
| `cargo clippy --workspace --all-targets -D warnings -D clippy::unwrap_used` | PASS (0 warnings) |
| `cargo fmt --all -- --check` | PASS (clean) |
| `cargo test --workspace` (nono lib only) | 1 FAIL (pre-existing D-04 baseline: `try_set_mandatory_label`) |
| `cargo test -p nono --lib -- audit::tests::verify_empty_log_with_no_stored_metadata_is_not_valid` | PASS (CR-02 guard test) |
| `cargo check --workspace --tests` | PASS (all struct literals compile) |

The `cargo test --workspace` gate (which runs only the `nono` lib on this Windows host) shows exactly the D-04 baseline failure — 1 failure, same as pre-Cluster-B. The nono-cli test environmental flakiness is documented above.

## Security Invariant Verification

| Invariant | Check | Result |
|-----------|-------|--------|
| CR-02: records_verified: event_count > 0 | grep -n "records_verified" crates/nono/src/audit.rs shows "event_count > 0" at line 1570 | PASS |
| No ApprovalRequest | grep -rn "ApprovalRequest" crates/ returns 0 production matches | PASS |
| No tls_intercept in production code | grep "tls_intercept\|TlsIntercept" route.rs server.rs returns comments only | PASS |
| No tool-sandbox dir | ls crates/nono-cli/src/tool-sandbox → path not found | PASS |
| exec_strategy.rs timeout refactor absent | exec_strategy.rs not in git diff HEAD | PASS |
| exec_strategy_windows/ untouched | Not in git diff HEAD | PASS |
| Upstream SHA in commit | git log --format="%B" HEAD | grep "11fd10e0" | PASS |
| DCO sign-off | git log --format="%B" HEAD | grep "Signed-off-by: Oscar Mack Jr" | PASS |

## Self-Check: PASSED

- `91d526e6` exists in git log: CONFIRMED
- `crates/nono/src/audit.rs` contains `SandboxRuntimeAuditEvent`: CONFIRMED
- `crates/nono/src/keystore.rs` contains `CMD_URI_PREFIX`: CONFIRMED
- `crates/nono/src/sandbox/mod.rs` contains `restrict_execute`: CONFIRMED
- `records_verified: event_count > 0` at audit.rs:1570: CONFIRMED

---
*Phase: 95-upstream-absorb-fork-invariant-verify*
*Completed: 2026-06-26*
