---
phase: 56-fine-grained-network-filtering
plan: "03"
subsystem: network-policy
tags: [allow_domain, AllowDomainEntry, profile_cmd, schema, manifest-endpoints, display-rendering]
dependency_graph:
  requires:
    - 56-01 (AllowDomainEntry enum + .domain() accessor)
  provides:
    - AllowDomainEntry::WithEndpoints display rendering in cmd_show
    - manifest::Network.endpoints population from WithEndpoints entries
    - nono-profile.schema.json AllowDomainWithEndpoints $defs + oneOf items
  affects:
    - profile_cmd.rs (cmd_show display, resolve_to_manifest endpoints)
    - nono-profile.schema.json (allow_domain items schema extended)
tech_stack:
  added: []
  patterns:
    - "match on AllowDomainEntry for format!() display rendering"
    - "filter_map with try_into().ok()? for manifest newtype conversion"
    - "oneOf(string, $ref) JSON Schema pattern for backward-compatible polymorphic arrays"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/profile_cmd.rs
    - crates/nono-cli/data/nono-profile.schema.json
decisions:
  - "Use manifest::NetworkEndpoint { host, rules } (not endpoint_rules — the plan had an incorrect field name; actual generated type uses 'rules' per capability-manifest.schema.json)"
  - "try_into().ok()? for host/method/path newtypes — filter_map silently skips invalid entries (T-56-10 accepted: manifest is diagnostic, enforcement is in proxy)"
  - "AllowDomainWithEndpoints schema added to $defs, not inlined in items — allows $ref from other locations if needed"
metrics:
  duration_minutes: 15
  completed_date: "2026-06-05"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 2
---

# Phase 56 Plan 03: Profile Display and Schema Update Summary

**One-liner:** profile_cmd.rs renders AllowDomainEntry::WithEndpoints as "domain (N endpoint rules)" and populates manifest endpoints; schema gains AllowDomainWithEndpoints $defs with oneOf allow_domain items.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | profile_cmd.rs allow_domain display, diff, and manifest endpoint mapping | cab43684 | crates/nono-cli/src/profile_cmd.rs |
| 2 | nono-profile.schema.json extend allow_domain items to oneOf | 6668db6d | crates/nono-cli/data/nono-profile.schema.json |

## What Was Built

### Task 1: profile_cmd.rs — three change sites

**Site 1 (cmd_show display, line ~1575):**
- Replaced flat `.domain()` map with a match on `AllowDomainEntry`
- `Plain(s)` renders as hostname string
- `WithEndpoints { domain, endpoints }` renders as `"domain (N endpoint rules)"`
- Satisfies D-11 WATCH-ITEM (profile show makes effective openness visible)

**Site 2 (cmd_diff, line ~2071):**
- Already using `.domain().to_string()` from Plan 01 adapters — no change required

**Site 3 (diff_to_json, line ~2606):**
- Already using `.domain().to_string()` from Plan 01 adapters — no change required

**Site 4 (resolve_to_manifest, line ~3250):**
- Added `manifest_endpoints` computation before `manifest::Network` construction
- `filter_map` over `WithEndpoints` entries with non-empty `endpoints`
- Maps `nono_proxy::config::EndpointRule.{method,path}` to `manifest::EndpointRuleMethod`/`manifest::EndpointRulePath` via `try_into().ok()?`
- Constructs `manifest::NetworkEndpoint { host, rules }` (field is `rules`, not `endpoint_rules`)
- Changed `endpoints: Vec::new()` to `endpoints: manifest_endpoints`

### Task 2: nono-profile.schema.json — schema extension

- Added `AllowDomainWithEndpoints` definition to `$defs`:
  - `required: ["domain"]`
  - `domain: string`
  - `endpoints: array of { method: string, path: string }` (optional)
- Changed `allow_domain.items` from `{ "type": "string" }` to `{ "oneOf": [{ "type": "string" }, { "$ref": "#/$defs/AllowDomainWithEndpoints" }] }`
- Backward-compatible: existing profiles with plain string arrays remain valid
- JSON validated with `python -m json.tool`
- `cargo build --bin nono` succeeds (schema embedded via build.rs)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan described wrong field name for manifest::NetworkEndpoint**

- **Found during:** Task 1, Site 4 implementation
- **Issue:** Plan 03 described `endpoint_rules` as the field name on `manifest::NetworkEndpoint`. The actual generated type (from `capability-manifest.schema.json`) uses `rules`.
- **Fix:** Used `rules` (the correct generated field name) in `manifest::NetworkEndpoint { host, rules }`
- **Verification:** Generated types confirmed at `target/debug/build/nono-*/out/capability_manifest_types.rs` line 1454: `pub rules: ::std::vec::Vec<EndpointRule>`
- **Files modified:** profile_cmd.rs (Site 4 only)
- **Commit:** cab43684

## Known Stubs

None introduced by this plan. The `manifest_endpoints` path is fully wired — `WithEndpoints` entries with non-empty `endpoints` produce `manifest::NetworkEndpoint` entries in the output manifest.

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes at trust boundaries beyond what the plan's threat model anticipated.

| Flag | File | Description |
|------|------|-------------|
| threat_flag: input-validation | crates/nono-cli/src/profile_cmd.rs | `try_into().ok()?` silently skips invalid NetworkEndpointHost/Method/Path values — accepted per T-56-10 (manifest is diagnostic, enforcement is in proxy) |

## Verification Results

- `cargo check --bin nono 2>&1 | grep "profile_cmd" | grep "^error"` — zero errors
- `grep "AllowDomainWithEndpoints" crates/nono-cli/data/nono-profile.schema.json` — $defs definition present
- `grep "oneOf" crates/nono-cli/data/nono-profile.schema.json` — allow_domain items updated
- `cargo build --bin nono` succeeds (schema embedded via build.rs; JSON parse errors fail the build)
- Both commits carry `Upstream-commit: 0ced085` trailer and `Signed-off-by: Oscar Mack Jr`

## Self-Check: PASSED

Files verified:
- `crates/nono-cli/src/profile_cmd.rs` — FOUND (modified)
- `crates/nono-cli/data/nono-profile.schema.json` — FOUND (modified)

Commits verified:
- `cab43684` — FOUND in git log (Task 1)
- `6668db6d` — FOUND in git log (Task 2)
