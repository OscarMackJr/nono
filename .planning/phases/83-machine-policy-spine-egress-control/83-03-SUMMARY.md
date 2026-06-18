---
phase: 83
plan: 03
subsystem: egress-policy
tags: [egress, admx, gpo, preset-tokens, network-policy, tdd]
dependency_graph:
  requires: [83-01]
  provides: [expand_egress_preset_tokens, network-policy-presets, admx-named-toggles]
  affects: [83-02, 83-04]
tech_stack:
  added: []
  patterns:
    - "Token-indirection: ADMX writes stable group TOKEN; CLI expands token->FQDN (D-11/D-12)"
    - "Embedded network-policy.json groups as preset token->FQDN map (D-12)"
    - "#[allow(dead_code)] with explanatory comment for forward-declared Plan-02 API"
key_files:
  created: []
  modified:
    - crates/nono-cli/data/network-policy.json
    - crates/nono-cli/src/policy.rs
    - scripts/build-windows-msi.ps1
    - scripts/validate-windows-msi-contract.ps1
    - dist/windows/nono.admx
    - dist/windows/nono.adml
decisions:
  - "D-11: ADMX named toggles write group TOKENS (anthropic/openai/github-api), not literal FQDNs; nono expands token->FQDN at runtime"
  - "D-12: preset token->FQDN map lives in embedded network-policy.json group mechanism (CLI-side, not core lib)"
  - "expand_egress_preset_tokens placed in policy.rs (not network_policy.rs) to mirror the policy resolver pattern and to be co-located with the group-resolution tests"
  - "Unknown tokens expand to empty list (T-83-token-widen fail-secure); never silently widen to all hosts"
metrics:
  duration: 25m
  completed: "2026-06-18"
  tasks: 2
  files: 6
---

# Phase 83 Plan 03: AI-Provider Egress Presets + ADMX Named Toggles Summary

**One-liner:** Wildcard FQDN preset groups for Anthropic/OpenAI/GitHub-API in network-policy.json with token->FQDN expansion in policy.rs and ADMX named-toggle here-strings that write stable group tokens (not literal FQDNs) per D-11.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| RED | Add failing policy_egress_groups tests | `8745ef38` | crates/nono-cli/src/policy.rs |
| GREEN (Task 1) | Add preset groups + expand_egress_preset_tokens | `51b66205` | network-policy.json, policy.rs |
| Task 2 | ADMX named-toggle policies + contract assertion | `e165abca` | build-windows-msi.ps1, validate-windows-msi-contract.ps1, nono.admx, nono.adml |

## What Was Built

### Task 1: AI-Provider Preset Groups + Token Expansion (TDD)

**TDD RED:** `8745ef38` — 5 failing tests added to `policy.rs`:
- `policy_egress_groups_present_in_network_policy` — verifies 3 preset groups in network-policy.json
- `policy_egress_groups_expand_anthropic_token` — token "anthropic" → "*.anthropic.com"
- `policy_egress_groups_expand_openai_token` — token "openai" → "*.openai.com"
- `policy_egress_groups_expand_github_api_token` — token "github-api" → "api.github.com"
- `policy_egress_groups_unknown_token_expands_to_empty` — T-83-token-widen fail-secure
- `policy_egress_groups_union_hosts` — all three tokens expand to their respective FQDNs

**TDD GREEN:** `51b66205` — implementation:

`crates/nono-cli/data/network-policy.json`: Added 3 new preset groups to the `groups` object:
- `"anthropic"`: hosts `["*.anthropic.com"]` with description mentioning EGRESS-04/D-11
- `"openai"`: hosts `["*.openai.com"]`
- `"github-api"`: hosts `["api.github.com"]`

`crates/nono-cli/src/policy.rs`: Added `expand_egress_preset_tokens(tokens: &[String]) -> Result<Vec<String>>`:
- Parses `embedded_network_policy_json()` via `crate::network_policy::load_network_policy()`
- Looks each token up in the `groups` map
- Unknown tokens → empty slice (fail-secure / T-83-token-widen)
- Deduplicates results (future-proof for overlapping presets)
- `#[allow(dead_code)]` with documented comment: used by Plan 83-02 wiring (transitional)

### Task 2: ADMX Named-Toggle Preset Policies + Contract Assertion

`scripts/build-windows-msi.ps1`: Added ADMX/ADML generation from here-strings:
- **`AllowAnthropicPreset`** policy: writes enabledValue `"anthropic"` to `SOFTWARE\Policies\nono\PresetTokens` as REG_SZ
- **`AllowOpenAIPreset`** policy: writes enabledValue `"openai"`
- **`AllowGitHubAPIPreset`** policy: writes enabledValue `"github-api"`
- Existing `AllowedSuffixes` / `AllowedHosts` `<list>` policies retained (N×REG_SZ, Pitfall 1 maintained)
- Both `nono.admx` and `nono.adml` emitted to `dist/windows/` via `Write-Utf8NoBomCompat`

`scripts/validate-windows-msi-contract.ps1`: Contract assertions added:
- Verifies "Allow Anthropic", "Allow OpenAI", "Allow GitHub API" are in the build script source
- Verifies token values `"anthropic"`, `"openai"`, `"github-api"` are present
- Verifies `AllowedSuffixes` / `AllowedHosts` still present (shape regression guard)

`dist/windows/nono.admx` + `nono.adml`: Regenerated to include the Phase 83 named-toggle policies.

## Verification Results

```
cargo test -p nono-cli policy_egress_groups
running 6 tests
test policy::tests::policy_egress_groups_present_in_network_policy ... ok
test policy::tests::policy_egress_groups_expand_anthropic_token ... ok
test policy::tests::policy_egress_groups_expand_openai_token ... ok
test policy::tests::policy_egress_groups_expand_github_api_token ... ok
test policy::tests::policy_egress_groups_union_hosts ... ok
test policy::tests::policy_egress_groups_unknown_token_expands_to_empty ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1292 filtered out
```

```
grep -Eq 'Allow Anthropic' scripts/build-windows-msi.ps1 && 
grep -Eq 'Allow OpenAI' ... && grep -Eq 'Allow GitHub API' ... && 
grep -Eq '"anthropic"' ... => ALL_SOURCE_ASSERTIONS_PASS
```

```
cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used => Finished (no errors)
```

## Deviations from Plan

**1. [Rule 2 - Missing critical functionality] Suppressed dead_code lint with documented allow**
- **Found during:** Task 1 — clippy -D warnings fails with E0 on `expand_egress_preset_tokens`
- **Issue:** `cargo clippy --bin nono -- -D warnings` promotes the dead_code warning to error because the function is not yet called from the binary (Plan 02 wiring will add the call)
- **Fix:** Added `#[allow(dead_code)]` with an explanatory comment documenting the Plan 02 dependency; the function is fully tested
- **Files modified:** crates/nono-cli/src/policy.rs
- **Commit:** `e165abca`

## Known Stubs

None — all data is wired. The preset groups in network-policy.json have real FQDN host entries; `expand_egress_preset_tokens` does real lookups. The function will be called from Plan 02's daemon-startup wiring.

## Threat Flags

None — no new network endpoints or auth paths introduced. The expansion function is read-only (embedded JSON parse + map lookup). The fail-secure unknown-token behavior (T-83-token-widen) is asserted by test.

## Self-Check: PASSED

- `crates/nono-cli/data/network-policy.json` — FOUND (contains anthropic/openai/github-api groups)
- `crates/nono-cli/src/policy.rs` — FOUND (contains expand_egress_preset_tokens)
- `scripts/build-windows-msi.ps1` — FOUND (contains Allow Anthropic/OpenAI/GitHub API toggles)
- `scripts/validate-windows-msi-contract.ps1` — FOUND (contains ADMX contract assertions)
- `dist/windows/nono.admx` — FOUND (contains AllowAnthropicPreset/AllowOpenAIPreset/AllowGitHubAPIPreset)
- `dist/windows/nono.adml` — FOUND (contains allow_anthropic/allow_openai/allow_github_api string resources)
- Commits: `8745ef38` (RED), `51b66205` (GREEN), `e165abca` (Task 2) — all in git log
