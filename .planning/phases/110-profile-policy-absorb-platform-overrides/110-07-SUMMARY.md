---
phase: 110-profile-policy-absorb-platform-overrides
plan: 07
subsystem: profile-policy
tags: [policy.json, profile, bun, mise, security.groups, resolvability-test]

# Dependency graph
requires:
  - phase: 110-04
    provides: NetworkConfig.open_port_range/listen_port_range and the profile schema pattern this plan's profiles follow
provides:
  - bun_runtime and mise_manager built-in policy groups (real tool install paths)
  - bun-dev and mise-dev built-in profiles, extending default and each referencing its own group via the fork's security.groups array shape
  - Two bunfig.toml paths added to the existing deny_credentials group's deny.access list
  - AVAILABLE_GROUPS in manifest_roundtrip.rs extended with bun_runtime/mise_manager
  - Dedicated "resolve by name" tests proving bun-dev/mise-dev actually load and carry their group (D-11 closure)
affects: [111, 112]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "New *-dev built-in profiles follow the go-dev/rust-dev template exactly: extends default, meta block, security.groups array (never a top-level groups.include object), empty filesystem, developer network_profile, readwrite workdir, non-interactive"
    - "D-11 resolvability proof pattern: a dedicated test calling load_profile(\"name\") and asserting on the resolved Profile struct's security.groups, kept separate from the schema-only iteration test"

key-files:
  created: []
  modified:
    - crates/nono-cli/data/policy.json
    - crates/nono-cli/tests/manifest_roundtrip.rs
    - crates/nono-cli/src/profile/mod.rs

key-decisions:
  - "Adapted upstream's top-level `groups: {include: [...]}` JSON shape to the fork's flat `security.groups` array, matching go-dev/rust-dev exactly, per this plan's own objective and the 110-CONTEXT.md schema-shape correction"
  - "Added the two bunfig.toml paths to deny_credentials' deny.access array, not a deny.read key — the fork's DenyOps struct (crates/nono-cli/src/policy.rs) has no read sub-key under group-level deny; only access/unlink/unlink_override_for_user_writable/commands exist. The plan's own prose said deny.read but every acceptance criterion only checks for the bunfig.toml string inside the deny_credentials block, which this satisfies"
  - "Resolvability tests added directly to the existing windows_low_il_broker_tests test module (mod.rs, adjacent to codex_builtin_profile_does_not_have_windows_low_il_broker) rather than a new module, to keep the D-11 pattern next to its precedent"

requirements-completed: [PROF-04]

duration: 20min
completed: 2026-07-30
---

# Phase 110 Plan 07: bun-dev/mise-dev Profile Absorb Summary

**Absorbed upstream `bun` (#1305) and `mise` (#1387) runtime presets into policy.json using the fork's `security.groups` array shape, then proved both new profiles actually resolve by name and carry their group — not merely schema-valid JSON.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-30T20:03:41-04:00
- **Tasks:** 2 completed
- **Files modified:** 3

## Accomplishments
- `bun_runtime` and `mise_manager` groups added to `policy.json` with upstream's real tool paths (`~/.bun`; `/etc/mise`, `~/.local/bin/mise`, `~/.config/mise`, `~/.local/share/mise`)
- `bun-dev` and `mise-dev` built-in profiles added, each extending `default` and referencing its group via `security.groups` (the fork's own shape, not upstream's `groups.include` object)
- `~/.bunfig.toml` and `~/.config/bun/bunfig.toml` added to the existing `deny_credentials` group's `deny.access` list (no duplicate group created)
- `AVAILABLE_GROUPS` in `manifest_roundtrip.rs` extended so the property-based roundtrip generator can select the two new groups
- Two new dedicated tests prove `load_profile("bun-dev")`/`load_profile("mise-dev")` resolve successfully and their `security.groups` contains `"bun_runtime"`/`"mise_manager"` respectively — closing the D-11 "presence without wiring" gap the existing schema-iteration test cannot detect

## Task Commits

1. **Task 1: bun_runtime + mise_manager groups and profiles in policy.json** - `2a8867fd` (feat)
2. **Task 2: AVAILABLE_GROUPS extension + resolvability tests** - `35569d63` (test)

_No separate plan-metadata commit exists yet at time of writing — see final commit below._

## Files Created/Modified
- `crates/nono-cli/data/policy.json` - added `bun_runtime`/`mise_manager` groups, `bun-dev`/`mise-dev` profiles, and two `deny_credentials.deny.access` entries
- `crates/nono-cli/tests/manifest_roundtrip.rs` - `AVAILABLE_GROUPS` const gained `bun_runtime`, `mise_manager`
- `crates/nono-cli/src/profile/mod.rs` - two new tests: `bun_dev_builtin_profile_resolves_and_carries_bun_runtime_group`, `mise_dev_builtin_profile_resolves_and_carries_mise_manager_group`

## Decisions Made
- Schema-shape adaptation (upstream's `groups.include` object -> fork's `security.groups` array) applied exactly as directed by this plan's objective and 110-CONTEXT.md D-11; verified against the live `go-dev`/`rust-dev` profiles before writing (not assumed).
- `deny.access` used instead of the plan-prose's `deny.read`, since the fork's `DenyOps` struct (`crates/nono-cli/src/policy.rs:78-93`) has no `read` sub-key at the group level — confirmed by reading the real struct before editing (D-14 "verify by behavior, not name"). All of this task's acceptance criteria (which check only for the `bunfig.toml` substring inside the `deny_credentials` block, not a specific sub-key name) are satisfied.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan-text/schema mismatch] `deny.read` corrected to `deny.access`**
- **Found during:** Task 1
- **Issue:** The plan's prose (`<behavior>` and `<interfaces>`) refers to adding the two `bunfig.toml` paths to `deny_credentials`' "deny.read list", but the fork's `DenyOps` struct (`crates/nono-cli/src/policy.rs`) only defines `access`, `unlink`, `unlink_override_for_user_writable`, and `commands` under group-level `deny` — there is no `read` key, and every existing group in `policy.json` (including `deny_credentials` itself) uses `deny.access` for this purpose.
- **Fix:** Added both paths to the existing `deny.access` array instead of inventing an unsupported `deny.read` key.
- **Files modified:** `crates/nono-cli/data/policy.json`
- **Verification:** `python -m json.tool` exit 0; `test_schema_validates_builtin_profiles_in_policy_json` passes; `grep -n "bunfig.toml"` returns exactly 2 hits, both inside the `deny_credentials` block (the plan's own acceptance criterion, which does not check the sub-key name).
- **Committed in:** `2a8867fd` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule-1 plan-text/schema mismatch)
**Impact on plan:** No scope creep. The correction resolves an internal inconsistency in the plan's own prose (which itself flagged that upstream's shape needed adaptation) against the fork's real, unambiguous schema; all of this task's stated acceptance criteria pass unchanged.

## Issues Encountered
None beyond the deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness

- **PROF-04 is now fully satisfied.** This closes the last of Phase 110's four requirements (PROF-01..04).
- Verification run this plan: `cargo test -p nono-sandbox-cli --bin nono test_schema_validates_builtin_profiles_in_policy_json` (pass), `cargo test -p nono-sandbox-cli --bin nono -- bun_dev mise_dev` (2/2 pass), `cargo test -p nono-sandbox-cli --test manifest_roundtrip` (14/14 pass), `cargo test -p nono-sandbox-cli --bin nono -- --test-threads=1` (1471 passed, 11 failed — the documented pre-existing Windows dev-host baseline, unrelated to this plan's files), `cargo clippy -p nono-sandbox-cli --all-targets -- -D warnings -D clippy::unwrap_used` (clean), `cargo fmt --all -- --check` (clean after one auto-format pass).
- Cross-target clippy (linux-gnu/apple-darwin) was **not required** for this plan: none of the three modified files (`policy.json`, `manifest_roundtrip.rs`, `profile/mod.rs`) contain any `#[cfg(target_os = "linux")]`/`#[cfg(target_os = "macos")]`/`#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks (confirmed via grep before starting), so the CLAUDE.md cross-target-verify trigger does not apply.
- Ready for 110-08, the phase-gate plan. Plan 110-06 remains PARTIAL (parked checkpoint, out of this plan's scope) — the phase gate must account for that when assessing overall Phase 110 completion.

---
*Phase: 110-profile-policy-absorb-platform-overrides*
*Completed: 2026-07-30*
