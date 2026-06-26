---
phase: 95-upstream-absorb-fork-invariant-verify
plan: 03
subsystem: proxy
tags: [upstream-sync, proxy-activation, fork-invariant, cluster-c, fail-secure]

# Dependency graph
requires:
  - phase: 95-02
    provides: Cluster B shared-surface absorb committed (91d526e6)
provides:
  - Cluster C 9b37dc52 absorb record (structural no-op confirmed and documented)
  - Phase 89 active predicate preserved (|| !prepared.custom_credentials.is_empty() at lines 95, 118)
  - proxy_activates_with_custom_credentials_only guard test intact (opts.active == true)
affects:
  - Phase 96 (cross-target clippy verification — D-03 deferred items)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Structural no-op absorb: upstream commit structurally incompatible with fork shape; no code change; empty commit records the analysis"
    - "D-02 fork-preserve rule: keep || !prepared.custom_credentials.is_empty() in active predicate; upstream inversion blocked"

key-files:
  created: []
  modified: []

key-decisions:
  - "D-02 confirmed: Cluster C (9b37dc52) is Outcome B (structural no-op) — upstream uses CredentialProxyIntent/EndpointFilterIntent/UpstreamProxyIntent struct shape incompatible with fork's flat ProxyLaunchOptions; no hunk applicable"
  - "D-03 confirmed: cross-target clippy deferred to Phase 96; no cfg-gated Unix code touched in this plan"
  - "Environmental flakiness: audit_session + config::tests ENV_LOCK cascade (same root cause as 95-02 SUMMARY) — not a Cluster C regression; Cluster C is a no-op empty commit"

requirements-completed:
  - UPST10-02

# Metrics
duration: 30min
completed: 2026-06-26
---

# Phase 95 Plan 03: Cluster C Credentials-Intent Absorb Summary

**Cluster C (9b37dc52) is a structural no-op in the fork: upstream's CredentialProxyIntent struct refactor is incompatible with the fork's flat ProxyLaunchOptions shape; the Phase 89 fail-secure proxy-activation divergence is preserved intact.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-06-26T04:00:00Z
- **Completed:** 2026-06-26T05:00:00Z
- **Tasks:** 2 (Task 1: absorb commit; Task 2: make ci gate)
- **Files modified:** 0 (empty commit)

## Cluster C Outcome

**Outcome B: Structural No-Op**

The upstream commit 9b37dc52 (`refactor(credentials): require explicit activation for custom credentials #1215`) uses a `CredentialProxyIntent` wrapper struct in `ProxyLaunchOptions.credentials: Option<CredentialProxyIntent>`. The fork's `ProxyLaunchOptions` uses a flat structural shape with `active: bool` and `custom_credentials: HashMap<...>` as separate fields.

**Structural evidence:**

Attempting `git cherry-pick --no-commit 9b37dc52` produced conflicts in all 3 upstream files:
- `crates/nono-cli/src/proxy_runtime.rs` — CONFLICT: upstream introduces `CredentialProxyIntent`, `EndpointFilterIntent`, `UpstreamProxyIntent`, `domain_filter`, `endpoint_filter` variables; none present in fork
- `crates/nono-cli/src/network_policy.rs` — CONFLICT: upstream removes `custom_credentials.keys()` from `all_names`; fork never had this pattern
- `crates/nono-cli/src/launch_runtime.rs` — CONFLICT: upstream adds `is_active()` method on `CredentialProxyIntent`; fork has `active: bool` flat field

**Grep confirmation:** `grep "credentials_intent\|CredentialProxyIntent\|has_custom_credentials\|has_credentials\|would_activate"` returned 0 matches in fork's `proxy_runtime.rs` — none of the upstream's refactored variables exist in the fork.

**The fork's equivalent behavior is already correct:** The active predicate at lines 90-119 of proxy_runtime.rs retains `|| !prepared.custom_credentials.is_empty()` at lines 95 and 118 — this is the Phase 89 fail-secure divergence (0c08e5d2) that must NOT be removed.

**The credentials_intent fix that IS compatible:** The upstream's `credentials_intent` fix (`if has_credentials || !prepared.custom_credentials.is_empty()`) maps to the fork's existing `custom_credentials: prepared.custom_credentials.clone()` at line 126 — already present and correct.

**The test inversion:** Upstream renames `test_proxy_is_active_when_only_custom_credentials_are_set` to `test_proxy_is_inactive_when_only_custom_credentials_are_set` and inverts to `opts.is_active() == false`. The fork's guard test `proxy_activates_with_custom_credentials_only` with `assert!(opts.active)` is preserved intact and continues to serve as the regression sentinel.

## Accomplishments

- Confirmed Cluster C is structurally incompatible (Outcome B) via `git cherry-pick --no-commit 9b37dc52` producing CONFLICT in all 3 files
- Reset all 3 conflicting files to HEAD; working tree clean
- Committed empty `--allow-empty` commit `62dbf013` recording the structural no-op with upstream SHA `9b37dc52` in body + DCO sign-off
- Verified Phase 89 active predicate intact: `|| !prepared.custom_credentials.is_empty()` at lines 95 and 118
- Verified `proxy_activates_with_custom_credentials_only` test name preserved at line 503
- Verified `opts.active == true` assertion at line 562 (NOT inverted)
- Verified 0 matches for `CredentialProxyIntent`, `test_proxy_is_inactive`, in fork's proxy_runtime.rs
- All 3 Cluster F guard tests pass: `proxy_activates_with_custom_credentials_only`, `block_net_overrides_custom_credentials_activation`, `build_proxy_config_maps_upstream_proxy_to_external_proxy`

## Task Commits

1. **Task 1: Cluster C structural no-op** - `62dbf013` (empty commit, refactor)

## Files Created/Modified

None. Cluster C is a structural no-op — no code files changed.

## Decisions Made

- Outcome B (structural no-op) confirmed by live `git cherry-pick --no-commit` producing merge conflicts in all 3 upstream files
- Empty `--allow-empty` commit chosen per plan's Outcome B template to record the analysis in git history with upstream SHA reference
- `exec_strategy_windows/` not touched (ADR-86 D-03); confirmed by `git diff HEAD~1 -- crates/nono-cli/src/exec_strategy_windows/` returning empty

## Deviations from Plan

None — plan executed exactly as written. Outcome B was the predicted outcome per 95-RESEARCH.md Cluster C analysis ("This hunk may be a no-op in the fork structurally"). Confirmed at execution time.

## Make CI Gate Results

| Check | Result |
|-------|--------|
| `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | PASS (0 warnings) |
| `cargo fmt --all -- --check` | PASS (clean) |
| `cargo test -p nono` | 1 FAIL (pre-existing D-04 baseline: `try_set_mandatory_label`) |
| `cargo test -p nono-cli` | 4 canonical baseline FAILs (profile_cmd + 3 protected_paths) + ENV_LOCK cascade from `audit_session` flakiness (documented in 95-02; not a Cluster C regression — empty commit changes no code) |
| `cargo test -p nono-ffi` | PASS (49/49) |
| `cargo audit` | PASS (0 errors; 4 allowed warnings) |
| `proxy_activates_with_custom_credentials_only` guard test | PASS |
| `block_net_overrides_custom_credentials_activation` guard test | PASS |
| `build_proxy_config_maps_upstream_proxy_to_external_proxy` guard test | PASS |

**No new failures introduced by Cluster C.** Cluster C is an empty commit with no code changes, so the test result set is identical to post-Cluster-B.

## Security Invariant Verification

| Invariant | Check | Result |
|-----------|-------|--------|
| Phase 89 active predicate preserved (D-02) | Lines 95 and 118 both retain `\|\| !prepared.custom_credentials.is_empty()` | PASS |
| Guard test name preserved | `grep "proxy_activates_with_custom_credentials_only"` → line 503 | PASS |
| Guard test assertion NOT inverted | `grep "opts.active,"` → line 562 (true assertion; NOT false) | PASS |
| Upstream test rename NOT applied | `grep "test_proxy_is_inactive"` → 0 matches | PASS |
| CredentialProxyIntent NOT introduced | `grep "CredentialProxyIntent"` → 0 matches | PASS |
| exec_strategy_windows/ untouched | `git diff HEAD~1 -- crates/nono-cli/src/exec_strategy_windows/` → empty | PASS |
| Upstream SHA in commit body | `git log --format="%B" HEAD \| grep "9b37dc52"` → match in body | PASS |
| DCO sign-off | `git log --format="%B" HEAD \| grep "Signed-off-by: Oscar Mack Jr"` → match | PASS |

## PARTIAL Deferrals

**Cross-target clippy DEFERRED to Phase 96 per D-03:**
This plan makes no code changes (empty commit). No cfg-gated Unix blocks touched by Cluster C. The PARTIAL→96 deferral from Clusters A and B (Phases 95-01 and 95-02) carries forward unchanged.

## Environmental Flakiness (Not a Regression)

The `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty` test continues to flake when `LOCALAPPDATA/nono/audit/` contains many real sessions. This poisons `ENV_LOCK`, cascading to 6-7 `config::tests` failures. This is the same root cause documented in 95-02 SUMMARY (unchanged; audit_session.rs is unmodified by any Cluster A/B/C change). Not a Cluster C regression — this plan makes zero code changes.

## Known Stubs

None.

## Threat Flags

None. No new network endpoints, auth paths, file access patterns, or schema changes introduced — Cluster C is a zero-code-change empty commit.

## Self-Check: PASSED

- `62dbf013` exists in git log: CONFIRMED
- `crates/nono-cli/src/proxy_runtime.rs` unchanged from HEAD~1: CONFIRMED (empty commit)
- `|| !prepared.custom_credentials.is_empty()` at lines 95 and 118: CONFIRMED
- `proxy_activates_with_custom_credentials_only` at line 503: CONFIRMED
- `opts.active == true` assertion at line 562: CONFIRMED
- 0 matches for `CredentialProxyIntent`: CONFIRMED
- 0 matches for `test_proxy_is_inactive`: CONFIRMED
- Upstream SHA `9b37dc52` in commit body: CONFIRMED
- `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` in commit: CONFIRMED

---
*Phase: 95-upstream-absorb-fork-invariant-verify*
*Completed: 2026-06-26*
