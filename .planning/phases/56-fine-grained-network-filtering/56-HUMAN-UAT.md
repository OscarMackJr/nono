---
status: partial
phase: 56-fine-grained-network-filtering
source: [56-VERIFICATION.md]
started: 2026-06-05T14:43:56Z
updated: 2026-06-05T14:43:56Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Live proxy enforcement (SC1 end-to-end)
expected: With a profile (or `--allow-domain https://api.github.com/repos/**`) that
restricts a host to specific method+path endpoints, a running nono proxy + sandboxed
child allows a matching request (e.g. GET /repos/...) and blocks a non-matching one
(e.g. POST /issues, or a path outside the rule). Static wiring is verified; this
confirms an actual HTTP request is filtered by `CompiledEndpointRules::is_allowed`.
result: [pending]

### 2. `nono why --host` terminal rendering
expected: `nono why --host api.github.com` (against a profile/state with endpoint
rules) renders an "Endpoint rules:" section listing the method+path rules, both for
the `--self` state-file path and the profile path.
result: [pending]

### 3. WR-02 policy decision — method ignored in `nono why` display
expected: Team confirms it is acceptable that `path_matches_endpoint_rules` matches
on path only (ignoring HTTP method) for the *display* approximation in `nono why`,
which can over-report "ALLOWED" relative to actual proxy enforcement. Accept pre-ship,
or escalate to a gap/follow-up.
result: [pending]

### 4. WR-03 policy decision — same-host Plain + WithEndpoints not merged at runtime
expected: Team confirms it is acceptable that a same-host `Plain` entry alongside a
`WithEndpoints` entry reaches `partition_allow_domain` without being merged. It stays
fail-secure via the cross-crate CONNECT block, but the state is fragile. Accept, or
escalate to a gap/follow-up (relates to IN-01 dedup in `merge_allow_domain`).
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps
