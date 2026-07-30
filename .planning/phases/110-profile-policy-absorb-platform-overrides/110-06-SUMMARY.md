---
phase: 110-profile-policy-absorb-platform-overrides
plan: 06
subsystem: sandbox
tags: [windows, wfp, port-ranges, capability-set, fork-original]

# Dependency graph
requires:
  - phase: 110 (plan 03, wave 1)
    provides: CapabilitySet.localhost_port_ranges()/merge_port_ranges (library mechanism consumed by compile_network_policy)
  - phase: 110 (plans 04/05, wave 2)
    provides: NetworkConfig.open_port_range profile schema + capability_ext.rs/manifest_convert.rs wiring that populates CapabilitySet.localhost_port_ranges() end-to-end on Linux/macOS
provides:
  - WindowsNetworkPolicy.localhost_port_ranges field (library, sandbox/mod.rs) populated by compile_network_policy via merge_port_ranges
  - WfpRuntimeActivationRequest.localhost_port_ranges IPC contract field (CLI, windows_wfp_contract.rs), threaded from build_wfp_runtime_activation_request
  - nono-wfp-service.rs PortCondition::RemoteRange/LocalRange variants + build_policy_filter_specs range loops (one PolicyFilterSpec per range entry, never per port)
  - nono-wfp-service.rs add_policy_filter's FWP_MATCH_RANGE/FWP_RANGE0 construction arm, confirmed against the real windows-sys 0.59 generated bindings
affects: [110-08 (phase gate, PROF-03 completion — BLOCKED on this plan's pending checkpoint)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "WFP expresses a port range natively as ONE FWP_MATCH_RANGE filter object, unlike macOS's forced per-port Seatbelt unroll (D-06) -- no MACOS_PORT_RANGE_LIMIT cap needed on this backend"
    - "FWP_RANGE0's valueLow/valueHigh are inline FWP_VALUE0 structs (not pointers) confirmed by reading the vendored windows-sys 0.59 generated bindings directly, not assumed from the plan's own sketch"
    - "Delayed (uninitialized-until-assigned) local declaration for a raw-pointer-coerced POD struct, instead of a throwaway zeroed() default, to avoid an unused_assignments warning under -D warnings while preserving the file's existing outlives-the-call stack-local idiom (sd_blob)"

key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/mod.rs
    - crates/nono/src/sandbox/windows.rs
    - crates/nono-cli/src/windows_wfp_contract.rs
    - crates/nono-cli/src/exec_strategy_windows/network.rs
    - crates/nono-cli/src/bin/nono-wfp-service.rs
    - crates/nono-cli/src/agent_daemon/launch.rs

key-decisions:
  - "Compile-forced construction sites beyond the plan's own <interfaces> claim: exec_strategy_windows/network.rs's make_blocked_policy test fixture and 5 WfpRuntimeActivationRequest literals in agent_daemon/launch.rs (2 real call sites + 3 test fixtures) were NOT named in the plan's interfaces section but had to be updated for cargo build --workspace --all-targets to pass, since neither WindowsNetworkPolicy nor WfpRuntimeActivationRequest derives Default -- another instance of this milestone's D-14 'verify by behavior, not name' pattern"
  - "range: FWP_RANGE0 declared uninitialized (not zeroed()) at add_policy_filter's function scope, matching the file's existing sd_blob stack-local-then-borrow idiom so the &mut range raw-pointer coercion stays valid through the FwpmFilterAdd0 call, while avoiding an unused_assignments warning that a throwaway zeroed() default would have produced under -D warnings"
  - "describe_windows_network_runtime_target extended to surface localhost_port_ranges (Rule 2 auto-fix) -- the same operator-visibility gap (D-11: schema key present but invisible) this milestone has repeatedly caught, this time in the WFP activation diagnostic string rather than a schema/profile-display surface"

patterns-established:
  - "Windows-side FWP condition construction now has a documented, source-verified reference for FWP_RANGE0/FWP_VALUE0/FWP_MATCH_RANGE shapes (windows-sys 0.59 vendored source), closing 110-RESEARCH.md's previously-UNVERIFIED question for this backend"

requirements-completed: []  # PROF-03 remains INCOMPLETE -- Task 3 (live-kernel checkpoint) is pending operator action. Do NOT mark PROF-03 complete until this checkpoint is approved.

# Metrics
duration: 75min (Tasks 1-2 + Rule 2 follow-up; Task 3 checkpoint time excluded -- pending)
completed: 2026-07-30 (PARTIAL -- see Pending Checkpoint below)
---

# Phase 110 Plan 06: Windows WFP-Native Port-Range Emitter Summary (PARTIAL — checkpoint pending)

**Windows now expresses `CapabilitySet.localhost_port_ranges()` as native WFP `FWP_MATCH_RANGE` kernel filters (one filter per range, never unrolled per-port) through the general `nono run` pipeline; the live-kernel proof (Task 3) requires Administrator elevation this session does not have and is an unresolved `checkpoint:human-verify` — the plan is NOT complete.**

## Performance

- **Duration:** ~75 min (Tasks 1 + 2 + one Rule 2 follow-up commit)
- **Tasks:** 2 of 3 complete; Task 3 is an unresolved blocking checkpoint
- **Files modified:** 6

## Accomplishments
- `WindowsNetworkPolicy` (library, `sandbox/mod.rs`) gained `localhost_port_ranges: Vec<(u16, u16)>`, populated by `compile_network_policy` via `merge_port_ranges` (interval merging, not sort/dedup — the one place this pattern deliberately diverges from the sibling discrete-port code) and folded into the `requires_backend` OR-chain.
- `WfpRuntimeActivationRequest` (CLI IPC contract, `windows_wfp_contract.rs`) gained the same sibling field, threaded through `build_wfp_runtime_activation_request` as a straight clone-through (proxy-mode's port has no range analog).
- `nono-wfp-service.rs`'s `PortCondition` enum gained `RemoteRange(u16,u16)`/`LocalRange(u16,u16)`; `build_policy_filter_specs` gained two sibling loops (outbound `RemoteRange`, inbound `LocalRange`) producing exactly ONE `PolicyFilterSpec` per range entry — proven by a 1000-wide-range test asserting exactly 2 outbound + 2 inbound specs (one per applicable WFP layer), never 1000. `needs_outbound_block`/`needs_inbound_block` now recognize ranges alone as "network is scoped."
- `add_policy_filter`'s port match gained a `RemoteRange`/`LocalRange` arm building a real `FWP_MATCH_RANGE` condition. The exact `FWP_RANGE0`/`FWP_VALUE0`/`FWP_CONDITION_VALUE0_0` shapes were confirmed by reading the actual vendored `windows-sys-0.59.0` generated bindings (not assumed from the plan's own sketch, which flagged this as unverified): `FWP_RANGE0 { valueLow: FWP_VALUE0, valueHigh: FWP_VALUE0 }` are **inline structs**, not pointers, while `FWP_CONDITION_VALUE0_0.rangeValue` is `*mut FWP_RANGE0`. Per T-110-15/T-110-16, the `(start, end)` tuple is used as-received with no re-validation or reordering.
- 7 new tests added (2 in `sandbox/windows.rs`, 3 in `nono-wfp-service.rs`, 1 in `exec_strategy_windows/network.rs`, plus the pre-existing `discrete_port_and_range_coexist_in_same_request_without_regression` test) prove: range population and merging at the library layer; exactly-one-spec-per-range-per-layer (never per port); range-alone triggers the block fallback; a discrete port alongside a range in the same request is unregressed; and the WFP activation diagnostic string surfaces ranges.
- `grep -rn "MACOS_PORT_RANGE_LIMIT"` across all 4 Windows-side files touched returns 0 hits (T-110-17 proof — WFP needs no per-port-count cap, unlike Seatbelt).
- `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`, and `cargo fmt --all -- --check` all clean on the final tree. `cargo build --workspace --release` also succeeds (needed for the Task 3 checkpoint's manual verification).
- Full regression sweep: `cargo test -p nono-sandbox --lib` (819 passed, 1 pre-existing unrelated failure — `machine_policy::tests::windows_wrong_reg_type_returns_policy_load_failed`, a documented Windows registry-deletion-timing race, see 110-05-SUMMARY.md); `cargo test -p nono-sandbox-cli --bin nono` (1469 passed, 11 pre-existing baseline failures, unchanged count from the documented dev-host baseline); `cargo test -p nono-sandbox-cli --bin nono-wfp-service` (24/24 pass).

## Task Commits

Only Tasks 1 and 2 are committed. Task 3 (checkpoint:human-verify) has NOT been approved and is NOT committed as complete — see "Pending Checkpoint" below.

1. **Task 1: Library + IPC contract wiring** — `ff10f52f` (feat)
2. **Task 2: WFP filter-spec construction (RemoteRange/LocalRange, FWP_MATCH_RANGE)** — `ca8b3f25` (feat)
3. **Follow-up: surface localhost_port_ranges in the Windows runtime-target diagnostic (Rule 2)** — `8fe69f43` (fix)

_No TDD RED/GREEN gate applies formally (plan's `tdd="true"` on Task 2 followed the plan's own "add code and tests together" instruction, matching the pattern already established across 110-03/04/05), but Task 2's tests were written and verified passing alongside the implementation in the same commit._

## Files Created/Modified
- `crates/nono/src/sandbox/mod.rs` — `WindowsNetworkPolicy.localhost_port_ranges: Vec<(u16, u16)>` field
- `crates/nono/src/sandbox/windows.rs` — `compile_network_policy` wires `merge_port_ranges(caps.localhost_port_ranges())`, extends `requires_backend`; 2 new tests (`compile_network_policy_carries_port_ranges_into_wfp_policy`, `compile_network_policy_merges_overlapping_port_ranges`)
- `crates/nono-cli/src/windows_wfp_contract.rs` — `WfpRuntimeActivationRequest.localhost_port_ranges: Vec<(u16, u16)>` field
- `crates/nono-cli/src/exec_strategy_windows/network.rs` — `build_wfp_runtime_activation_request` clone-through; `make_blocked_policy` test-fixture compile-forced update; `describe_windows_network_runtime_target` extended (Rule 2); 1 new test (`describe_windows_network_runtime_target_surfaces_localhost_port_ranges`)
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — `PortCondition::RemoteRange`/`LocalRange` variants; `build_policy_filter_specs`'s 2 new sibling loops + `needs_outbound_block`/`needs_inbound_block` OR-chain extensions; `add_policy_filter`'s `FWP_MATCH_RANGE`/`FWP_RANGE0` construction arm; `sample_request()` + 1 direct literal compile-forced update; 3 new tests
- `crates/nono-cli/src/agent_daemon/launch.rs` — 5 `WfpRuntimeActivationRequest` construction-site compile-forced updates (2 real call sites in `wfp_filter_add`/`wfp_filter_remove`, 3 test fixtures) — all empty-vec clone-through; this daemon-only path never grants port ranges

## Decisions Made
- **`range: FWP_RANGE0` declared uninitialized, not `zeroed()`.** The file's established idiom (`sd_blob`) declares a POD struct at function scope with a real default so a later conditional `&mut`-coercion stays valid through `FwpmFilterAdd0`. For `range`, a `zeroed()` default produced an `unused_assignments` warning (the initial value is always overwritten before any read, in the one arm that uses it) which would fail under `-D warnings`. Delayed (uninitialized-until-assigned) declaration compiles cleanly because Rust's definite-assignment analysis proves every use is preceded by a direct assignment along all reachable paths, while preserving the same function-scope-outlives-the-call memory-safety property.
- **Confirmed the real windows-sys 0.59 binding shapes before writing the FFI code**, per the plan's own explicit instruction that its sketch was unverified. Read `$USERPROFILE/.cargo/registry/src/.../windows-sys-0.59.0/src/Windows/Win32/NetworkManagement/WindowsFilteringPlatform/mod.rs` directly: `FWP_RANGE0.valueLow`/`valueHigh` are inline `FWP_VALUE0` values (no `Box`/pointer indirection needed for the range bounds themselves), while `FWP_CONDITION_VALUE0_0.rangeValue` is the one pointer indirection (`*mut FWP_RANGE0`) — simpler than the plan's own `Box::into_raw` sketch anticipated.
- **Extended scope to 2 files not named in the plan's `<interfaces>` section** (`exec_strategy_windows/network.rs`'s `make_blocked_policy` fixture, `agent_daemon/launch.rs`'s 5 construction sites) because `cargo build --workspace --all-targets` — the plan's own Task 1 acceptance criterion — would not pass otherwise. Matches this milestone's now-repeated D-14 finding: the plan's own interfaces claims about which construction sites exist were incomplete.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Compile-forced construction sites beyond the plan's named interfaces**
- **Found during:** Task 1 verification (`cargo build --workspace --all-targets`)
- **Issue:** Neither `WindowsNetworkPolicy` nor `WfpRuntimeActivationRequest` derives `Default`. The plan's `<interfaces>` section named only 2 construction sites per struct, but the workspace actually has 6 total across `exec_strategy_windows/network.rs` (1 test fixture) and `agent_daemon/launch.rs` (5 sites: `wfp_filter_add`, `wfp_filter_remove`, and 3 test fixtures).
- **Fix:** Added `localhost_port_ranges: vec![]` (or the equivalent field) to every site. The `agent_daemon/launch.rs` daemon-only path never grants port ranges (it force-routes traffic through a single proxy port), so all 5 sites are a straight empty-vec clone-through with no behavior change.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/network.rs`, `crates/nono-cli/src/agent_daemon/launch.rs`
- **Verification:** `cargo build --workspace --all-targets` exits 0
- **Committed in:** `ff10f52f` (Task 1 commit)

**2. [Rule 1 - Bug] `unused_assignments` warning from a throwaway `zeroed()` default**
- **Found during:** Task 2 verification (`cargo build --workspace --all-targets`, then re-confirmed via `cargo clippy -- -D warnings`)
- **Issue:** `let mut range: FWP_RANGE0 = zeroed();` triggered `warning: value assigned to range is never read` because the initial value is always overwritten before any read that matters.
- **Fix:** Changed to a delayed (uninitialized-until-assigned) declaration `let mut range: FWP_RANGE0;`, relying on Rust's definite-assignment analysis to prove soundness.
- **Files modified:** `crates/nono-cli/src/bin/nono-wfp-service.rs`
- **Verification:** `cargo build --workspace --all-targets` produces zero warnings; `cargo clippy -- -D warnings -D clippy::unwrap_used` exits 0
- **Committed in:** `ca8b3f25` (Task 2 commit)

**3. [Rule 2 - Missing Critical] `describe_windows_network_runtime_target` did not surface port ranges**
- **Found during:** Post-Task-2 review, checking for the D-11 "schema key present but invisible to operator" pattern this milestone has repeatedly caught
- **Issue:** The function feeds the operator-facing `WfpRuntimeActivationRequest.runtime_target` field used in WFP-activation-required error diagnostics. It listed `tcp_connect_ports`/`tcp_bind_ports`/`localhost_ports` but not the new `localhost_port_ranges`, so an operator debugging an activation failure on a range-using profile would see no mention of the range.
- **Fix:** Added a sibling `restrictions.push(...)` branch for `localhost_port_ranges`, plus a new test.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/network.rs`
- **Verification:** New test `describe_windows_network_runtime_target_surfaces_localhost_port_ranges` passes; `cargo fmt --all -- --check` clean
- **Committed in:** `8fe69f43` (standalone follow-up commit)

---

**Total deviations:** 3 auto-fixed (1 blocking, 1 bug, 1 missing-critical)
**Impact on plan:** All three were necessary for correctness (compile completeness), lint cleanliness under the mandatory `-D warnings` gate, and closing a recurring observability gap this milestone specifically watches for. No scope creep — no filter-construction logic or security-relevant behavior was added beyond what the plan specified.

## Issues Encountered

**Stray leftover `nono-agentd.exe` process (PID 6256) blocked `cargo build --workspace --release`.** While preparing the environment for the Task 3 checkpoint's `how-to-verify` step 1 (`cargo build --workspace --release`), the linker failed with `error: failed to remove file ...\nono-agentd.exe: Access is denied (os error 5)`. A `Get-Process` check found a running `nono-agentd` process holding the file open — evidently a leftover daemon from an earlier, unrelated test/debug session on this dev host, unrelated to this plan's changes. Stopped the stray process (`Stop-Process -Force`) and re-ran; the release build then completed cleanly (`Finished release profile [optimized] target(s)`). This is host-state cleanup, not a source change, and is not part of this plan's `files_modified`.

## Known Stubs

None.

## Threat Flags

None — this plan implements only the mitigations already named in its own `<threat_model>` (T-110-15/T-110-16/T-110-17/T-110-SC): no re-validation at the WFP-service trust boundary (by design, matching the register), unit-tested 1:1 field mapping proving no reordering, and a grep-confirmed absence of `MACOS_PORT_RANGE_LIMIT` on this backend. No new network endpoints, auth paths, or schema changes at a trust boundary beyond what the plan's threat register already covers.

## User Setup Required

**Yes — Task 3's live-kernel checkpoint is unresolved and requires operator action. See "Pending Checkpoint" below and the structured checkpoint returned to the orchestrator.**

## Pending Checkpoint (Task 3 — PROF-03e, live-kernel verification)

**This plan is NOT complete.** Task 3 is a `checkpoint:human-verify` gated `blocking` that cannot be exercised from this non-elevated, non-Administrator dev session (`FwpmFilterAdd0` requires Administrator privileges and a running `nono-wfp-service`). All prerequisite automation is done:

- `cargo build --workspace --release` succeeds (after clearing a stray leftover `nono-agentd.exe` process holding a file lock — see Issues Encountered).
- `target\release\nono-wfp-service.exe` and `target\release\nono.exe` are both freshly built.

**What the operator must do** (from an Administrator PowerShell session):
1. Start `target\release\nono-wfp-service.exe` per the existing WFP-01 dark-gate procedure (non-elevated daemon via `runas /trustlevel:0x20000`, per project memory `wfp_confined_egress_and_daemon_gate`).
2. Author a test profile with `"network": {"open_port_range": [[49200, 49210]]}` and run `nono run --profile <that-profile> -- cmd /c "echo test"` (or an equivalent probe) from a non-elevated shell.
3. Confirm a connection attempt to a port INSIDE the range (e.g. 49205) succeeds and a connection attempt OUTSIDE the range (e.g. 49199 or 49211) is blocked.
4. Inspect the live WFP filter table (`netsh wfp show filters`) and confirm exactly ONE filter object exists for the range — not eleven individual port filters — proving D-06's "no per-port unroll" at the kernel level, not just in the unit-tested spec-construction function.

Resume signal: "approved" if the range filter enforces correctly with a single kernel filter object, or a description of what failed.

**Do NOT mark `PROF-03` complete in `REQUIREMENTS.md` or `ROADMAP.md`'s Phase 110 plan checklist for `110-06-PLAN.md` until this checkpoint is approved.**

## Next Phase Readiness

- Blocked. `110-08` (phase gate, cross-target clippy + `make ci` + binding rebuild + fork-invariant verify) depends on all Phase 110 plans being complete — this plan is not, pending the Task 3 checkpoint.
- `110-07` (bun/mise runtime presets, PROF-04) has no dependency on this plan's Task 3 and can proceed independently if the orchestrator chooses to parallelize, but the phase as a whole cannot close until Task 3 is resolved.

---
*Phase: 110-profile-policy-absorb-platform-overrides*
*Completed: 2026-07-30 (Tasks 1-2 only; PARTIAL, checkpoint pending)*

## Self-Check: PASSED

All 6 modified source files confirmed present on disk with the expected new symbols
(`localhost_port_ranges` in `sandbox/mod.rs`/`sandbox/windows.rs`/`windows_wfp_contract.rs`/
`exec_strategy_windows/network.rs`; `RemoteRange`/`LocalRange`/`FWP_MATCH_RANGE`/`FWP_RANGE0` in
`bin/nono-wfp-service.rs`; `localhost_port_ranges: vec![]` compile-forced sites in
`agent_daemon/launch.rs`). All 3 commit hashes (`ff10f52f`, `ca8b3f25`, `8fe69f43`) confirmed present
in `git log --oneline --all`.
