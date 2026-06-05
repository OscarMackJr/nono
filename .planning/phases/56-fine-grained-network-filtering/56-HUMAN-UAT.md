---
status: partial
phase: 56-fine-grained-network-filtering
source: [56-VERIFICATION.md]
started: 2026-06-05T14:43:56Z
updated: 2026-06-05T15:20:00Z
---

## Current Test

[testing paused — 1 item outstanding]

Test 1 (live proxy enforcement, SC1 end-to-end) is BLOCKED on Windows: supervised
proxy filtering is not implemented on this platform (fail-secure refusal). Re-run on
Linux/macOS to exercise it. Tests 2/3/4 resolved (1 pass + 2 policy-accepted).

## Tests

### 1. Live proxy enforcement (SC1 end-to-end)
expected: With a profile (or `--allow-domain https://api.github.com/repos/**`) that
restricts a host to specific method+path endpoints, a running nono proxy + sandboxed
child allows a matching request (e.g. GET /repos/...) and blocks a non-matching one
(e.g. POST /issues, or a path outside the rule). Static wiring is verified; this
confirms an actual HTTP request is filtered by `CompiledEndpointRules::is_allowed`.
result: blocked
blocked_by: other
reason: "Windows: nono refused with 'Platform not supported: ... supervised features
  not available yet: proxy filtering. Windows supervised execution does not implement
  proxy-filter-driven supervision yet.' This is correct fail-secure behavior — live
  proxy enforcement can only be exercised on Linux/macOS where supervised proxy
  filtering is available. Static wiring verified; nono-proxy is_allowed suite (161
  tests) covers enforcement on those platforms."

### 2. `nono why --host` terminal rendering
expected: `nono why --host api.github.com` (against a profile/state with endpoint
rules) renders an "Endpoint rules:" section listing the method+path rules, both for
the `--self` state-file path and the profile path.
result: pass
note: "Verified with the Phase-56 dev build (target/debug/nono.exe, Jun 5). Output for
  --profile <throwaway> --host api.github.com rendered 'Endpoint rules:' with both
  'GET /repos/**' and 'POST /repos/*/issues', plus 'restricted to 2 endpoint rules'.
  NOTE: PATH-installed nono v0.57.5 predates Phase 56 and rejects the WithEndpoints
  profile form ('invalid type: map, expected a string') — phase work is unreleased."

### 3. WR-02 policy decision — method ignored in `nono why` display
expected: Team confirms it is acceptable that `path_matches_endpoint_rules` matches
on path only (ignoring HTTP method) for the *display* approximation in `nono why`,
which can over-report "ALLOWED" relative to actual proxy enforcement. Accept pre-ship,
or escalate to a gap/follow-up.
result: pass
note: "Team ACCEPTED 2026-06-05. WR-02 is a known non-blocking follow-up: nono why
  verdict (path_matches_endpoint_rules) is method-blind/diagnostic-only and may
  over-report ALLOWED; proxy enforcement (nono-proxy is_allowed) is method-correct.
  Divergence documented in code comment. Not a ship blocker."

### 4. WR-03 policy decision — same-host Plain + WithEndpoints not merged at runtime
expected: Team confirms it is acceptable that a same-host `Plain` entry alongside a
`WithEndpoints` entry reaches `partition_allow_domain` without being merged. It stays
fail-secure via the cross-crate CONNECT block, but the state is fragile. Accept, or
escalate to a gap/follow-up (relates to IN-01 dedup in `merge_allow_domain`).
result: pass
note: "Team ACCEPTED 2026-06-05. WR-03 is a known non-blocking follow-up: same-host
  Plain + WithEndpoints entries are not merged at runtime; contradictory state is
  resolved fail-secure by the server.rs CONNECT block. Fragile (depends on the
  second layer), so track the merge fix (IN-01 / merge_allow_domain dedup) as future
  work. Not a ship blocker."

## Summary

total: 4
passed: 3
issues: 0
pending: 0
skipped: 0
blocked: 1

## Gaps
