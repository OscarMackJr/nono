---
phase: 95-upstream-absorb-fork-invariant-verify
plan: "06"
subsystem: proxy-security
tags: [fork-safety, async-signal-safe, endpoint-policy, reverse-proxy, seccomp]

requires:
  - phase: 95-05
    provides: Gap-closure context; 95-04 fork-invariant verify findings (WR-01/CR-01 gaps)

provides:
  - const MSG_PROXY_WRITE_FAIL and MSG_PROXY_FILTER_FAIL static byte strings in exec_strategy.rs post-fork child paths (WR-01 closed)
  - CompiledEndpointPolicy.evaluate() wired into reverse.rs request handler (CR-01 closed)
  - Two new config.rs tests proving endpoint_policy deny enforcement and legacy-route semantics

affects:
  - Phase 96 cross-target clippy (exec_strategy.rs unix cfg-gated code)
  - Any future audit of post-fork child safety
  - Any future endpoint_policy feature additions to nono-proxy

tech-stack:
  added: []
  patterns:
    - "CR-01 const MSG_* pattern: all post-fork child error messages use const static byte strings + libc::write, never format!()"
    - "evaluate() additive: endpoint_policy.evaluate() runs AFTER endpoint_rules.is_allowed(); legacy routes preserve backward compat"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/config.rs

key-decisions:
  - "WR-01: Dynamic errno lost in static message — accepted tradeoff: message conveys failure type; errno is inaccessible safely in post-fork context"
  - "CR-01: evaluate() placed AFTER endpoint_rules check (additive), not replacing it — preserves backward compat for legacy routes that use only endpoint_rules"
  - "Test placed in config.rs unit tests (not reverse.rs handler test) — evaluate() is a pure function on CompiledEndpointPolicy; unit test proves enforcement without a stream harness"

patterns-established:
  - "Post-fork child safety pattern: const MSG_* static byte string + libc::write(STDERR_FILENO) + libc::_exit(126); no format!() or heap allocation"

requirements-completed:
  - UPST10-02
  - UPST10-03

duration: 18min
completed: 2026-06-26
---

# Phase 95 Plan 06: Fork-Invariant Gap Closure (WR-01 + CR-01) Summary

**Replaced two format!() heap allocations in the post-fork child with const static byte strings (WR-01), and wired CompiledEndpointPolicy.evaluate() into the reverse proxy request path to close an operator fail-open (CR-01)**

## Performance

- **Duration:** 18 min
- **Started:** 2026-06-26T13:00:00Z
- **Completed:** 2026-06-26T13:18:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- WR-01 closed: both post-fork/pre-exec error paths in exec_strategy.rs now use `const MSG_PROXY_WRITE_FAIL` and `const MSG_PROXY_FILTER_FAIL` static byte strings with `libc::write()` instead of `format!()` heap allocation; CR-01 comment block added at each site explaining the async-signal-safety invariant
- CR-01 closed: `route.endpoint_policy.evaluate()` is now called in `handle_reverse_proxy()` after the legacy `endpoint_rules.is_allowed()` check; explicit `EndpointPolicyOutcome::Deny` returns 403 with audit event before any credential operations
- Two new unit tests in config.rs prove (a) a configured endpoint_policy deny rule for "DELETE /admin/*" returns `Deny` for "DELETE /admin/users" and (b) legacy routes wrapped via `compile()` preserve their default-deny semantics

## Task Commits

1. **Task 1: Replace format!() heap allocation in post-fork child (WR-01)** - `2a8b639e` (fix)
2. **Task 2: Wire CompiledEndpointPolicy.evaluate() into reverse.rs + add enforcement test (CR-01)** - `c81429aa` (fix)

## Files Created/Modified

- `crates/nono-cli/src/exec_strategy.rs` — Two `format!()` calls at post-fork error paths replaced with `const MSG_PROXY_WRITE_FAIL` and `const MSG_PROXY_FILTER_FAIL` static byte strings; `Err(e)` renamed to `Err(_e)` to satisfy unused-variable lint
- `crates/nono-proxy/src/reverse.rs` — Added `use crate::config::EndpointPolicyOutcome;` import; inserted `endpoint_policy.evaluate()` check block after the `endpoint_rules.is_allowed()` block
- `crates/nono-proxy/src/config.rs` — Added two unit tests: `endpoint_policy_deny_rule_is_enforced` and `endpoint_policy_legacy_route_preserves_deny_semantics`

## Decisions Made

- Errno detail lost when converting to static strings: accepted. The message type ("failed to write proxy seccomp notify fd number" / "seccomp proxy filter not available") is sufficient for operator diagnosis; the exact OS error is inaccessible safely in a post-fork context anyway.
- evaluate() placed AFTER the legacy endpoint_rules check (additive, not replacing): operator-configured `endpoint_policy` deny rules now add a stricter layer without removing backward compat for legacy `endpoint_rules`-only routes.
- Test at config.rs unit level (not reverse.rs handler level): `CompiledEndpointPolicy::evaluate()` is a pure function; a unit test proving the Deny outcome is a sufficient proof of enforcement since reverse.rs calls evaluate() directly. No stream harness needed.

## Deviations from Plan

None — plan executed exactly as written. Both violations were at the expected code locations; cfg gating was confirmed correct; the evaluate() wiring and test match the plan's specified approach.

## Issues Encountered

Second `Err(e)` replacement failed on first attempt due to indentation mismatch (grep context confirmed the correct whitespace); fixed on second attempt with exact indentation. No impact.

## Known Stubs

None.

## Threat Flags

None — changes close documented threats T-95-06-01 (format!() deadlock) and T-95-06-02 (endpoint_policy fail-open). No new network endpoints, auth paths, or trust boundaries introduced.

## Self-Check

Files exist:
- `crates/nono-cli/src/exec_strategy.rs` — FOUND
- `crates/nono-proxy/src/reverse.rs` — FOUND
- `crates/nono-proxy/src/config.rs` — FOUND

Commits:
- `2a8b639e` — FOUND (Task 1: WR-01 static byte strings)
- `c81429aa` — FOUND (Task 2: CR-01 evaluate() wiring)

Grep acceptance:
- `MSG_PROXY_WRITE_FAIL` in exec_strategy.rs — MATCH (line 1415)
- `MSG_PROXY_FILTER_FAIL` in exec_strategy.rs — MATCH (line 1457)
- `endpoint_policy.evaluate` in reverse.rs — MATCH (line 126)
- `EndpointPolicyOutcome::Deny` in reverse.rs — MATCH (line 125)

Tests: 175 nono-proxy tests pass (2 new added). 1 pre-existing baseline fail (`try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails` — env-sensitive Phase 74 code, not a regression).

## Self-Check: PASSED

## Next Phase Readiness

- WR-01 and CR-01 gap items from the 95-04 verification checklist are now closed
- Phase 96 cross-target clippy can run against the updated exec_strategy.rs (unix cfg-gated code); no new cfg-gated branches were added
- No blockers

## Code-Review Fix

**Finding:** CR-01 (95-REVIEW.md, 2026-06-26) identified a fail-open in the `endpoint_policy`
wiring added by this plan. `CompiledEndpointPolicy::evaluate()` returns three variants —
`Deny`, `Allow`, `Approve` — but `reverse.rs` matched only `Deny` with `if let`. An `Approve`
outcome (reachable via `approve:` rules or `default: approve` in operator config) silently
fell through and forwarded the request upstream with the real injected credential and no audit
event, violating the Fail-Secure principle.

**Fix (commit `fix(95-06): fail Approve outcome closed in reverse.rs endpoint_policy (code-review CR-01)`):**

- Replaced the `if let EndpointPolicyOutcome::Deny { .. }` block in `reverse.rs` with an
  exhaustive `match` over all three arms (no `_` wildcard — compiler-enforced exhaustiveness
  prevents any future variant from silently falling open).
- `Allow` arm: explicit empty body — request proceeds to credential lookup.
- `Deny` arm: existing 403 + `audit::log_denied` path, unchanged.
- `Approve` arm: fails closed — 403 + `audit::log_denied` with a distinct reason string making
  clear no approval backend is wired ("endpoint_policy requires approval but no approval backend
  is configured — failing closed: ...").
- Added `endpoint_policy_approve_without_backend_is_recognized` test in `config.rs` asserting
  that an explicit `approve:` rule and a `default: approve` both produce `Approve` outcome from
  `evaluate()`, documenting the contract that reverse.rs must handle.
- `cargo test -p nono-proxy`: 176 passed (175 prior + 1 new). Clippy and fmt clean.

---
*Phase: 95-upstream-absorb-fork-invariant-verify*
*Completed: 2026-06-26*
