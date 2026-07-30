---
phase: 110-profile-policy-absorb-platform-overrides
plan: 05
subsystem: sandbox
tags: [port-ranges, capability-set, manifest-convert, profile-cmd, cross-target-clippy]

# Dependency graph
requires:
  - phase: 110 (plan 03, same wave)
    provides: CapabilitySet.localhost_port_ranges mechanism (add_localhost_port_range/allow_localhost_port_range, merge_port_ranges, MACOS_PORT_RANGE_LIMIT)
  - phase: 110 (plan 04, same wave)
    provides: NetworkConfig.open_port_range (Vec<[u16; 2]>), profile_runtime.rs start<=end + macOS cumulative-cap pre-validation
provides:
  - capability_ext.rs profile pathway wiring (open_port_range -> add_localhost_port_range)
  - manifest_convert.rs independent manifest pathway (PortConfig.localhost_range -> allow_localhost_port_range, own start<=end + macOS cumulative-cap validation)
  - capability-manifest.schema.json PortConfig.localhost_range typify-source property
  - profile_cmd.rs display of open_port_range/listen_port_range in cmd_show, resolve_to_manifest PortConfig.localhost_range field
  - output.rs --dry-run -v display of localhost_port_ranges() alongside discrete localhost_ports()
affects: [110-06 (Windows WFP-native range emitter reads the same CapabilitySet.localhost_port_ranges()), 110-08 (phase gate, PROF-03 completion)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two independent, non-overlapping consumer pathways (profile-driven capability_ext.rs vs manifest-driven manifest_convert.rs) each own their own validation of the same underlying CapabilitySet mechanism, matching ADR-86's policy-free-library boundary: the mechanism validates nothing itself (allow_localhost_port_range only rejects start==0), each CLI-facing/library-facing pathway is responsible for its own start<=end and macOS cumulative-cap checks"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/capability_ext.rs
    - crates/nono/schema/capability-manifest.schema.json
    - crates/nono/src/manifest_convert.rs
    - crates/nono/tests/manifest_types.rs
    - crates/nono-cli/src/profile_cmd.rs
    - crates/nono-cli/src/output.rs

key-decisions:
  - "manifest_convert.rs's localhost_range conversion duplicates profile_runtime.rs's start<=end and macOS cumulative-cap validation logic verbatim in shape (not by reuse) because the manifest pathway (--config launch) never touches profile_runtime.rs at all -- this is the plan's own documented intentional dual-pathway design, not redundant code"
  - "Tests for the manifest pathway added to the existing crates/nono/tests/manifest_types.rs integration-test file (no mod tests existed inside manifest_convert.rs itself) to match the established test-location convention for TryFrom<&CapabilityManifest> conversions"

requirements-completed: [PROF-03]  # PROF-03 spans plans 110-03/04/05/06; see Requirement Tracking Note below -- REQUIREMENTS.md deliberately NOT flipped by this plan

# Metrics
duration: 50min
completed: 2026-07-30
---

# Phase 110 Plan 05: capability_ext.rs + manifest_convert.rs Port-Range Wiring Summary

**Wired `open_port_range` to `CapabilitySet.localhost_port_ranges()` through both the profile pathway (`capability_ext.rs::CapabilitySet::from_profile`) and the independent manifest pathway (`manifest_convert.rs::TryFrom<&CapabilityManifest>`), each with its own validation, plus display support in `nono profile show` and `--dry-run -v` output.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- `capability_ext.rs`'s `CapabilitySet::from_profile` gained a 3-line sibling loop next to the existing `open_port` loop, calling `caps.add_localhost_port_range(start, end)?` for each `profile.network.open_port_range` entry (already validated start<=end and macOS-capped by plan 110-04's `profile_runtime.rs` before this point is ever reached).
- `capability-manifest.schema.json`'s `PortConfig` gained a `localhost_range` property (array of `[start, end]` integer pairs, `minItems`/`maxItems: 2`), typify-regenerated at build time into `manifest::PortConfig { localhost_range: Vec<[NonZeroU64; 2]> }` — confirmed by the schema-only edit producing a clean `cargo build --workspace --all-targets`.
- `manifest_convert.rs`'s `TryFrom<&CapabilityManifest> for CapabilitySet` gained its own independent conversion for `ports.localhost_range`: a hard rejection for any reversed `[start, end]` entry (`"start must be <= end"`), a macOS-only cumulative cap check against `nono::capability::MACOS_PORT_RANGE_LIMIT` (via `merge_port_ranges`), then the `allow_localhost_port_range` loop — the manifest pathway never touches `profile_runtime.rs`, so it needed its own equivalent protection rather than reusing plan 110-04's validation.
- `profile_cmd.rs`'s `cmd_show` gained display blocks for `open_port_range`/`listen_port_range`, formatting each entry as `"{s}..={e}"`; `resolve_to_manifest`'s `ports` guard now also checks `open_port_range.is_empty()`, and the `PortConfig` literal gains a `localhost_range` field built from `open_port_range` (matching upstream: `listen_port_range` is intentionally NOT threaded into the manifest, mirroring the existing `listen_port`/`open_port` asymmetry).
- `output.rs`'s `print_capabilities` localhost-ports block now also triggers on `!caps.localhost_port_ranges().is_empty()`, appending each range as `"{start}..={end}"` to the same displayed list as the discrete ports.
- Both mandatory cross-target clippy gates (`cross clippy` linux-gnu, `cargo-zigbuild clippy` apple-darwin) ran clean given the new `#[cfg(target_os = "macos")]` block introduced in `manifest_convert.rs`. Beyond the mandatory clippy gate, `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox --test manifest_types` additionally *executed* (not just compiled) both new manifest-pathway tests on real Linux: 18/18 tests pass, including `convert_network_localhost_range` and `convert_network_localhost_range_rejects_start_greater_than_end`.

## Task Commits

Each task was committed atomically:

1. **Task 1: capability_ext.rs profile-pathway wiring + manifest-pathway conversion** - `4d635305` (feat)
2. **Task 2: Display wiring (profile_cmd.rs, output.rs)** - `07a45e22` (feat)

_No TDD gate applies to this plan (`type="auto" tdd="true"` on Task 1 added code and tests together within the same commit, matching the plan's own `<action>`/`<acceptance_criteria>` shape, not a separate RED/GREEN cycle)._

## Files Created/Modified
- `crates/nono-cli/src/capability_ext.rs` - `open_port_range` sibling loop in `CapabilitySet::from_profile`; new test `test_from_profile_open_port_range_populates_localhost_port_ranges`
- `crates/nono/schema/capability-manifest.schema.json` - `PortConfig.localhost_range` property (typify source)
- `crates/nono/src/manifest_convert.rs` - independent `ports.localhost_range` conversion (start<=end check, macOS cumulative-cap check via `merge_port_ranges`, `allow_localhost_port_range` loop); `#[cfg(target_os = "macos")] use crate::capability::merge_port_ranges;`
- `crates/nono/tests/manifest_types.rs` - new tests `convert_network_localhost_range`, `convert_network_localhost_range_rejects_start_greater_than_end`
- `crates/nono-cli/src/profile_cmd.rs` - `cmd_show` display blocks for `open_port_range`/`listen_port_range`; `resolve_to_manifest`'s `ports` guard + `PortConfig.localhost_range` field construction
- `crates/nono-cli/src/output.rs` - `print_capabilities`'s localhost-ports block extended to include `localhost_port_ranges()` entries

## Decisions Made
- Placed the new manifest-pathway tests in the existing `crates/nono/tests/manifest_types.rs` integration test file (there is no `mod tests` inside `manifest_convert.rs` itself) rather than adding an inline test module, matching the file's established convention for testing `TryFrom<&CapabilityManifest>` conversions (`convert_network_ports`, `convert_process_modes`, etc. all live there).
- The manifest-pathway validation block in `manifest_convert.rs` intentionally duplicates the shape of plan 110-04's `profile_runtime.rs` validation (start<=end check, then macOS-gated cumulative cap check) rather than calling into `profile_runtime.rs` — this is the plan's own documented intentional dual-pathway design (a `--config` manifest launch never goes through `profile_runtime.rs`), not incidental duplication.

## Deviations from Plan

None - plan executed exactly as written. The `<interfaces>` section's sketched code for both the `capability_ext.rs` loop and the `manifest_convert.rs` conversion block matched the live fork's actual call sites and field names verbatim (no drift discovered, unlike several earlier phase-110 plans' `<specifics>`-flagged assumption mismatches).

## Issues Encountered

**Formatting:** `cargo fmt --all -- --check` initially reported 2 diffs (the new `#[cfg(target_os = "macos")] use` ordering in `manifest_convert.rs`, and a multi-line collect chain in `output.rs`). Resolved with `cargo fmt --all` before the final commit; re-verified clean.

**Pre-existing, unrelated test failures observed during verification (not caused by this plan, not fixed, per Scope Boundary):**
- `cargo test -p nono-sandbox-cli --bin nono -- --test-threads=1` reproduces exactly the documented 11-failure Windows dev-host baseline (`profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`, 3 `protected_paths::tests::*`, 6 `config::tests::*`, 1 `audit_session::tests::*`) — zero new failures introduced by this plan's changes.
- `cargo test -p nono-sandbox --lib` surfaced one additional pre-existing, host-specific failure not previously in the documented list: `machine_policy::tests::windows_wrong_reg_type_returns_policy_load_failed` fails with `Os { code: 1018, ... "Illegal operation attempted on a registry key that has been marked for deletion." }` — a Windows registry-deletion-timing race in an unrelated test file (`crates/nono/src/machine_policy.rs`), never touched by this plan. Not fixed (out of scope, unrelated file).

## Known Stubs

None.

## Threat Flags

None — this plan implements only the mitigations already named in its own `<threat_model>` (T-110-13/T-110-14/T-110-SC): independent manifest-pathway validation, and a typify-source-only schema edit whose generated type is compiler-verified on every build. No new network endpoints, auth paths, or schema changes at a trust boundary beyond what the plan's threat register already covers.

## User Setup Required

None - no external service configuration required.

## Requirement Tracking Note

`PROF-03` is intentionally **NOT** marked complete in `REQUIREMENTS.md`. Plans 110-03/04/05/06 all
declare the same requirement ID `PROF-03` (a single requirement split across 4 sub-plans, confirmed
in 110-03-SUMMARY.md and 110-04-SUMMARY.md). This plan satisfies the CLI-side wiring half —
`open_port_range` now reaches `CapabilitySet.localhost_port_ranges()` end-to-end on Linux (Landlock)
and macOS (Seatbelt) via both the profile and manifest pathways. Only plan 110-06 (Windows WFP-native
range emitter) remains before `PROF-03` is complete across all three platforms.

## Next Phase Readiness

- `CapabilitySet.localhost_port_ranges()` is now reachable end-to-end from both a profile
  (`--profile`) and a manifest (`--config`) launch, on Linux and macOS, with independent
  validation on each pathway.
- Plan 110-06 (Windows WFP-native range emitter) can now read `caps.localhost_port_ranges()`
  exactly the way `caps.localhost_ports()` is already read today in `compile_network_policy` —
  no further CLI-side wiring is needed before the Windows emitter lands.
- `nono profile show` and `--dry-run -v` output now surface the new fields, closing the
  "schema key that deserializes but reaches no enforcement path AND is invisible to the operator"
  gap this milestone has repeatedly caught.
- Both mandatory cross-target clippy gates (linux-gnu `cross clippy`, apple-darwin `cargo-zigbuild
  clippy`) ran clean on the final tree.

---
*Phase: 110-profile-policy-absorb-platform-overrides*
*Completed: 2026-07-30*

## Self-Check: PASSED

All 6 modified source files confirmed present on disk with the expected new symbols
(`add_localhost_port_range` in `capability_ext.rs`; `localhost_range` in
`capability-manifest.schema.json`; `allow_localhost_port_range` in `manifest_convert.rs`;
`open_port_range`/`localhost_port_ranges` in `profile_cmd.rs`/`output.rs`). Both commit hashes
(`4d635305`, `07a45e22`) confirmed present in `git log --oneline --all`.
