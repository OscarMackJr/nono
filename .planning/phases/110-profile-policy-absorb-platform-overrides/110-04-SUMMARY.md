---
phase: 110-profile-policy-absorb-platform-overrides
plan: 04
subsystem: profile
tags: [port-ranges, profile-schema, network-config, capability-set, cross-target-clippy]

# Dependency graph
requires:
  - phase: 110 (plan 03, same wave)
    provides: CapabilitySet.localhost_port_ranges mechanism (merge_port_ranges, MACOS_PORT_RANGE_LIMIT) consumed by profile_runtime.rs's macOS cumulative-cap pre-check
provides:
  - NetworkConfig.open_port_range/listen_port_range fields (Vec<[u16; 2]>, array-of-pairs JSON shape)
  - merge_profiles' 3rd exhaustive-literal site for both fields (dedup_append union)
  - nono-profile.schema.json companion properties (closes Pitfall 4 / T-110-12)
  - profile_runtime.rs range validation (start<=end + macOS-only cumulative cap pre-check)
  - listen_port_range unroll into PreparedProfile.listen_ports (discrete carry-through, not a new CapabilitySet field)
affects: [110-05 (capability_ext.rs/manifest wiring consumes open_port_range), 110-06 (Windows WFP-native range emitter), 111 (v3.7 profile-schema follow-on work)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Standalone, platform-independent helper functions (validate_port_ranges, build_listen_ports, check_macos_port_range_cap) extracted ahead of the exhaustive PreparedProfile struct literal so range validation is directly unit-testable without needing a full on-disk profile load"
    - "#[cfg(any(target_os = \"macos\", test))] gate on a platform-specific helper so its error path has direct unit coverage on every dev host, not just macOS CI"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - crates/nono-cli/src/profile_runtime.rs

key-decisions:
  - "Extracted validate_port_ranges/build_listen_ports/check_macos_port_range_cap as standalone fns above prepare_profile_with_context (not inlined in the Ok(PreparedProfile{...}) literal as the plan's interfaces section sketched) so each has direct, isolated unit test coverage without requiring a full profile::load_profile_with_context disk-backed fixture"
  - "check_macos_port_range_cap gated #[cfg(any(target_os = \"macos\", test))] rather than plain #[cfg(target_os = \"macos\")], so the 'exceeds the macOS limit' error path is unit-tested on every host (Windows dev host included) per the plan's own allowance for an OS-independent test exercising the pre-check function directly"

requirements-completed: [PROF-03]  # PROF-03 spans plans 110-03/04/05/06; see Requirement Tracking Note below — REQUIREMENTS.md deliberately NOT flipped by this plan

# Metrics
duration: 45min
completed: 2026-07-30
---

# Phase 110 Plan 04: Profile Schema Port-Range Fields + profile_runtime.rs Validation Summary

**Absorbed the profile-facing half of upstream `d5803b99`: `NetworkConfig.open_port_range`/`listen_port_range` (array-of-pairs JSON), a matching `nono-profile.schema.json` companion edit, and `profile_runtime.rs` range validation (start<=end + macOS cumulative cap) with `listen_port_range` unrolling into the existing discrete `listen_ports` carry-through.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- `NetworkConfig` gained `open_port_range`/`listen_port_range: Vec<[u16; 2]>` fields (array-of-pairs, matching upstream's exact JSON shape), wired through `merge_profiles`' 3rd exhaustive-literal site via the same `dedup_append` union pattern as `open_port`/`listen_port`, and through both existing exhaustive test-fixture literals.
- `nono-profile.schema.json` gained matching `open_port_range`/`listen_port_range` properties — a fork-only companion edit (upstream's own commit never touched this file) that closes the Pitfall 4 / T-110-12 gap where a profile using the new fields would `serde`-parse but fail strict schema validation.
- `profile_runtime.rs`'s `prepare_profile_with_context` now validates both range fields before constructing any `PreparedProfile`: a hard parse-time error for any reversed `[start, end]` entry, and (macOS only) a cumulative combined-port-count check against `nono::capability::MACOS_PORT_RANGE_LIMIT` — a deliberate defense-in-depth duplicate of the plan 110-03 sandbox-layer check, giving operators the clearer error earlier.
- `listen_port_range` correctly unrolls into the existing discrete `PreparedProfile.listen_ports` carry-through (not a new `CapabilitySet` field), with a hard rejection for any range starting at port 0 (port 0 has no defined meaning in a range, unlike the discrete `open_port` field where 0 is macOS's `localhost:*` wildcard).
- Both mandatory cross-target clippy gates (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`) ran clean given the new `#[cfg(target_os = "macos")]` block Task 2 introduced in `profile_runtime.rs`.

## Task Commits

Each task was committed atomically:

1. **Task 1: NetworkConfig fields + merge_profiles 3rd exhaustive-literal site + schema** - `f7008b72` (feat)
2. **Task 2: profile_runtime.rs validation (start<=end, macOS cumulative cap) + listen_port_range unroll** - `4fe0dcde` (feat)

_No TDD gate applies to this plan (`type="auto" tdd="true"` tasks added code and tests together within the same commit, matching the plan's own `<action>`/`<acceptance_criteria>` shape, not a separate RED/GREEN cycle)._

## Files Created/Modified
- `crates/nono-cli/src/profile/mod.rs` - `NetworkConfig.open_port_range`/`listen_port_range` fields, `merge_profiles`'s `dedup_append` arm for both, 2 test-fixture literal updates, 3 new tests (schema validation, serde round-trip, merge union)
- `crates/nono-cli/data/nono-profile.schema.json` - `network.open_port_range`/`listen_port_range` properties (array of `[start, end]` integer pairs, `minItems`/`maxItems: 2`)
- `crates/nono-cli/src/profile_runtime.rs` - `validate_port_ranges` (start<=end check), `check_macos_port_range_cap` (macOS-only cumulative cap, `#[cfg(any(target_os = "macos", test))]`), `build_listen_ports` (range unroll + port-0 rejection), `to_range_tuples` helper; wired into `prepare_profile_with_context`; 6 new tests

## Decisions Made
- Extracted the plan's inline-sketched validation/unroll logic into three standalone functions (`validate_port_ranges`, `check_macos_port_range_cap`, `build_listen_ports`) placed immediately above `prepare_profile_with_context`, rather than inlining them inside the `if let Some(profile) = ...` block and the `Ok(PreparedProfile { ... })` literal as the plan's `<interfaces>` section sketched. `prepare_profile_with_context` itself requires a full on-disk profile load (`profile::load_profile_with_context`, pack verification, hook installation) to exercise, which would make direct unit testing of just the range-validation logic impractical. The extracted functions take a `&profile::Profile` directly and are independently unit-tested without any of that machinery.
- Gated `check_macos_port_range_cap` with `#[cfg(any(target_os = "macos", test))]` instead of a plain `#[cfg(target_os = "macos")]`, so its "exceeds the macOS limit" error path has direct unit coverage on this Windows dev host (and any non-macOS CI lane), per the plan's own allowance for "an OS-independent test exercising the pre-check function directly if it's split out as a standalone fn." On a real macOS build the function is used both in production (called from `validate_port_ranges` under the `target_os = "macos"` half of the `any(...)`) and in tests — no duplicate compilation, no dead-code warning on any platform.

## Deviations from Plan

None - plan executed exactly as written, with the one structural refactor (inline logic extracted to standalone functions) noted above under Decisions Made — this is a testability improvement, not a behavioral deviation; the validation/unroll semantics match the plan's `<interfaces>` section and upstream `d5803b99` exactly.

## Issues Encountered

One pre-existing, unrelated test failure was observed while running the plan's own verification command (`cargo test -p nono-sandbox-cli --bin nono profile`): `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` fails with `ProfileParse("Profile file already exists: ...my-agent.json")`. This is one of the 11 documented pre-existing Windows dev-host baseline failures (memory `nono_cli_windows_baseline_test_failures.md`, "profile_cmd init" category) caused by dev-host state (`my-agent.json` already present under `%APPDATA%\nono\profiles`), not a regression introduced by this plan. Not fixed, per Scope Boundary.

## Known Stubs

None.

## Threat Flags

None — this plan implements the mitigations already named in its own `<threat_model>` (T-110-10/T-110-11/T-110-12/T-110-SC): the start<=end check, the macOS cumulative-cap pre-check, and the `nono-profile.schema.json` companion edit. No new network endpoints, auth paths, or schema changes at a trust boundary beyond what the plan's threat register already covers.

## User Setup Required

None - no external service configuration required.

## Requirement Tracking Note

`PROF-03` is intentionally **NOT** marked complete in `REQUIREMENTS.md`. Plans 110-03/04/05/06 all
declare the same requirement ID `PROF-03` (a single requirement split across 4 sub-plans, confirmed
in 110-03-SUMMARY.md). This plan satisfies the profile-schema + `profile_runtime.rs` validation half;
`capability_ext.rs`/manifest wiring (110-05) and the Windows WFP emitter (110-06) remain before
`open_port_range` actually reaches a sandbox capability end-to-end. PROF-03 should be marked complete
only after 110-06 lands.

## Next Phase Readiness

- `NetworkConfig.open_port_range`/`listen_port_range` are parseable, schema-valid, merge correctly
  through `extends`, and are validated (start<=end, macOS cumulative cap) at profile-prepare time.
  Plan 110-05 can now read `profile.network.open_port_range` from a resolved `Profile` and drive
  `CapabilitySet.allow_localhost_port_range`/`add_localhost_port_range` (landed in plan 110-03) in
  `capability_ext.rs`.
- `PreparedProfile.listen_ports` already contains every unrolled `listen_port_range` port, so no
  further plan needs to touch the listen-port carry-through — 110-05/110-06 only need to wire
  `open_port_range` into the `CapabilitySet` range mechanism.

---
*Phase: 110-profile-policy-absorb-platform-overrides*
*Completed: 2026-07-30*

## Self-Check: PASSED

All 3 modified source files confirmed present on disk with the expected new symbols
(`open_port_range`/`listen_port_range` in `profile/mod.rs` and `nono-profile.schema.json`;
`validate_port_ranges`/`check_macos_port_range_cap`/`build_listen_ports` in `profile_runtime.rs`).
Both commit hashes (`f7008b72`, `4fe0dcde`) confirmed present in `git log --oneline --all`.
