---
phase: 75-supplementary-controls-secondary-engines
plan: 03
subsystem: infra
tags: [policy, windows, copilot-cli, native-pe, engine-profile, supp-03]

# Dependency graph
requires:
  - phase: 75-01
    provides: per-agent WFP egress (SUPP-02) — copilot-cli profile uses same daemon launch path
  - phase: 71-engine-agnostic-launch-productionization
    provides: engine-neutral BrokerLaunchNoPty arm (windows_low_il_broker: true)
provides:
  - copilot-cli engine profile in policy.json (native PE, windows_low_il_broker: true)
  - copilot_cli_profile_present and copilot_cli_profile_is_native_pe unit tests
  - D-06 research finding baked into profile description (native PE, not node.exe wrapper)
affects: [75-05-uat, sc3-copilot-confined-end-to-end]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Native-PE engine profile pattern: windows_low_il_broker: true with NO windows_interpreters"
    - "TDD RED/GREEN for policy.json profile additions via get_builtin() unit tests"

key-files:
  created: []
  modified:
    - crates/nono-cli/data/policy.json
    - crates/nono-cli/src/profile/builtin.rs

key-decisions:
  - "D-06: copilot.exe is a native PE (not a node.exe wrapper); profile has no windows_interpreters field"
  - "groups: [] placeholder in copilot-cli profile; add node_runtime only if SC3 UAT shows node.exe grandchild"
  - "Description field encodes both the native-PE rationale and the SC3 UAT monitoring instruction (Pitfall 5)"

patterns-established:
  - "Native-PE engine profile: windows_low_il_broker: true + no windows_interpreters (vs aider/langchain-python which use python.exe)"
  - "copilot_cli_profile_is_native_pe test asserts empty windows_interpreters as the distinguishing gate"

requirements-completed:
  - SUPP-03

# Metrics
duration: 15min
completed: 2026-06-15
---

# Phase 75 Plan 03: Copilot CLI Engine Profile Summary

**GitHub Copilot CLI (copilot.exe) profiled as a native-PE engine in policy.json with windows_low_il_broker: true and no windows_interpreters field (D-06 native-PE finding), plus two copilot_cli_profile_present / copilot_cli_profile_is_native_pe unit tests**

## Performance

- **Duration:** 15 min
- **Started:** 2026-06-15T21:45:00Z
- **Completed:** 2026-06-15T21:59:52Z
- **Tasks:** 1 (TDD: RED + GREEN commits)
- **Files modified:** 2

## Accomplishments

- Added `copilot-cli` profile to `crates/nono-cli/data/policy.json` with the native-PE shape: `windows_low_il_broker: true`, no `windows_interpreters` field, `security.groups: []`, `signal_mode: isolated`, `network.block: false`, `workdir.access: readwrite`
- Profile description documents D-06 finding (native PE, not node.exe wrapper) and the SC3 UAT monitoring instruction for Pitfall 5 (if copilot.exe spawns node.exe as a grandchild, add `windows_interpreters: ["node.exe"]`)
- Added two unit tests in `profile/builtin.rs` via `get_builtin("copilot-cli")`: `copilot_cli_profile_present` (asserts profile existence, broker flag, workdir, signal_mode, network) and `copilot_cli_profile_is_native_pe` (asserts empty `windows_interpreters`)
- `cargo build -p nono-cli` succeeds (embedded JSON parse validated at build time); clippy clean

## Task Commits

1. **Task 1 RED: failing tests** - `f3f8f9bf` (test)
2. **Task 1 GREEN: copilot-cli profile in policy.json** - `f1b8a6e6` (feat)

## Files Created/Modified

- `crates/nono-cli/data/policy.json` - Added `copilot-cli` profile (17 lines, alphabetically placed between aider and langchain-python within the engine profiles section)
- `crates/nono-cli/src/profile/builtin.rs` - Added `copilot_cli_profile_present` and `copilot_cli_profile_is_native_pe` tests (52 lines)

## Decisions Made

- Profile inserted alphabetically (copilot-cli before langchain-python) within the engine profiles section of policy.json
- `groups: []` left as placeholder rather than adding `node_runtime`; the SC3 UAT gate (plan 75-05) determines whether node.exe coverage is needed
- No REFACTOR commit needed — no cleanup required beyond the GREEN implementation

## Deviations from Plan

None - plan executed exactly as written. The TDD RED/GREEN cycle matched the plan spec. No unexpected issues encountered.

## Issues Encountered

None.

## Known Stubs

None. The `copilot-cli` profile is a static JSON configuration; it does not wire up to any runtime data source. The `groups: []` placeholder is intentional and documented (filled at SC3 UAT time if node.exe coverage is needed). This is not a rendering stub — the profile is complete for its intended use via `nono agent launch --profile copilot-cli -- copilot [args]`.

## Threat Flags

None. The `copilot-cli` profile follows the same trust model as `aider` and `langchain-python`. No new trust boundaries are introduced beyond what the plan's `<threat_model>` documented (T-75-03-01 and T-75-03-02 are accepted/mitigated per plan).

## SC3 UAT Instructions (plan 75-05 gate)

On real Win11 host with `copilot.exe` installed (`winget install GitHub.Copilot` or MSI):

```
nono agent launch --profile copilot-cli -- copilot ask "What is 2+2?"
```

Monitor child processes spawned by the confined `copilot.exe`. If `node.exe` appears as a grandchild (Pitfall 5), add `"windows_interpreters": ["node.exe"]` to the copilot-cli profile and update `copilot_cli_profile_is_native_pe` accordingly.

## Next Phase Readiness

- SUPP-03a complete. The copilot-cli profile is in policy.json and embedded in the nono-cli binary
- SC3 UAT (plan 75-05) can now proceed: `nono agent launch --profile copilot-cli -- copilot ask "..."` on real Win11
- Cross-target clippy note: no cfg-gated code was added (policy.json is platform-agnostic JSON); no cross-target verification required for this plan

## Self-Check: PASSED

Files confirmed present:
- `crates/nono-cli/data/policy.json` - FOUND (copilot-cli profile added)
- `crates/nono-cli/src/profile/builtin.rs` - FOUND (copilot_cli_profile_present + copilot_cli_profile_is_native_pe tests added)

Commits confirmed:
- `f3f8f9bf` - FOUND (test RED)
- `f1b8a6e6` - FOUND (feat GREEN)

Both unit tests pass: `cargo test -p nono-cli copilot_cli` → 2 passed, 0 failed

---
*Phase: 75-supplementary-controls-secondary-engines*
*Completed: 2026-06-15*
