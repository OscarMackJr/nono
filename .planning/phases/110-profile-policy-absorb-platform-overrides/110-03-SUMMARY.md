---
phase: 110-profile-policy-absorb-platform-overrides
plan: 03
subsystem: sandbox
tags: [landlock, seatbelt, seccomp, port-ranges, capability-set, cross-target-clippy]

# Dependency graph
requires:
  - phase: 110 (plans 01/02, same wave)
    provides: platform_overrides field, $VAR/@git:* token expansion (unrelated files, no code overlap)
provides:
  - CapabilitySet.localhost_port_ranges mechanism (merge_port_ranges, MACOS_PORT_RANGE_LIMIT, builder/accessor pair)
  - macOS Seatbelt range-aware localhost port emission with a cumulative 16,384-port cap
  - Linux Landlock range-aware localhost port emission with no cap
  - Linux seccomp proxy-only fallback range-based bind allowance (proxy_bind_port_ranges)
affects: [110-04 (profile schema/NetworkConfig open_port_range/listen_port_range), 110-05 (capability_ext.rs/manifest layer wiring), 110-06 (Windows WFP-native range emitter)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CapabilitySet additive-field mechanism (Default-derived struct, no exhaustive-literal site forced)"
    - "Cumulative-cap-checked-before-string-built pattern for macOS Seatbelt profile generation"
    - "Fork-a-child-then-apply_with_abi()-then-probe-syscall-outcome test pattern for Landlock enforcement proof (never call apply_with_abi in the parent test process - restrict_self() is irreversible)"

key-files:
  created: []
  modified:
    - crates/nono/src/capability.rs
    - crates/nono/src/sandbox/macos.rs
    - crates/nono/src/sandbox/linux.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
    - crates/nono-cli/src/supervised_runtime.rs

key-decisions:
  - "landlock v0.4.4 confirmed (vendored source, net.rs:109) to have no native range rule type - Linux emitter unrolls per-port with no cap, matching upstream d5803b99 exactly"
  - "macOS cumulative-cap check computed and errored BEFORE any profile string is built, preserving the sandbox_init() SIGILL rationale (~17,770 rules) verbatim in both the error message and a code comment"
  - "SupervisorConfig.proxy_bind_port_ranges added as a Linux-only cfg-gated sibling field to proxy_bind_ports, paired into all 9 exhaustive test-fixture literals (self-consistent count-gate: 9 == 9)"

patterns-established:
  - "Wave-0 gap PROF-03c closed: original test coverage for Linux Landlock range expansion (no upstream test existed to port)"

requirements-completed: [PROF-03]

# Metrics
duration: 55min
completed: 2026-07-30
---

# Phase 110 Plan 03: CapabilitySet Port-Range Mechanism + Unix Emitters Summary

**Ported upstream `d5803b99`'s port-range mechanism into `CapabilitySet` (mechanism-only, ADR-86-compliant) and extended both Unix sandbox backends (Seatbelt unroll-with-cap, Landlock unroll-with-no-cap) plus the Linux seccomp proxy-only fallback's range-based bind check.**

## Performance

- **Duration:** ~55 min
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments
- `CapabilitySet` gained a policy-free `localhost_port_ranges: Vec<(u16,u16)>` mechanism (additive sibling to `localhost_ports`, no exhaustive-literal site forced since `CapabilitySet` derives `Default`) with `MACOS_PORT_RANGE_LIMIT`, `merge_port_ranges`, builder/mutable/accessor methods, and 10 ported tests.
- macOS Seatbelt backend expands ranges into individual `localhost:N` outbound rules, enforcing a cumulative 16,384-port cap checked BEFORE any profile string is built (preserving the `sandbox_init()` SIGILL rationale in both the error and a code comment); 6 ported tests.
- Linux Landlock backend expands ranges into per-port `NetPort` ConnectTcp+BindTcp rules with **no artificial cap** (confirmed via the vendored `landlock-0.4.4` crate source that no native range rule type exists); 2 original tests close Wave-0 gap PROF-03c, using a fork-a-child-then-probe-bind()-outcome pattern to avoid ever calling the irreversible `restrict_self()` in the parent test process.
- Linux seccomp proxy-only fallback (`supervisor_linux.rs`) now accepts `bind()` on a port inside any configured `proxy_bind_port_ranges` entry, in addition to the existing discrete `proxy_bind_ports` list (union semantics); `SupervisorConfig` gained the new field across all 9 existing exhaustive test-fixture literals plus the real `supervised_runtime.rs` construction site.
- Both mandatory cross-target clippy gates (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`) ran clean (exit 0) on the final, fmt-clean tree. The Linux `cross test` run additionally *executed* (not just compiled) both new `sandbox::linux` port-range tests successfully.

## Task Commits

Each task was committed atomically:

1. **Task 1: CapabilitySet mechanism** - `a0bf9076` (feat)
2. **Task 2: macOS Seatbelt range emitter + cumulative cap** - `0c747739` (feat)
3. **Task 3: Linux Landlock range emitter + seccomp proxy-bind ranges** - `5d0c8e34` (feat)
4. **Follow-up: cargo fmt sweep on Task 2/3 additions** - `2102f538` (style)

_No TDD gate applies to this plan (`type="auto" tdd="true"` tasks used add-code-then-add-tests-together, matching the plan's own `<action>` instructions to port tests verbatim alongside the implementation, not a separate RED/GREEN cycle)._

## Files Created/Modified
- `crates/nono/src/capability.rs` - `MACOS_PORT_RANGE_LIMIT`, `merge_port_ranges`, `localhost_port_ranges` field + `allow_localhost_port_range`/`add_localhost_port_range`/`localhost_port_ranges()`, 10 tests
- `crates/nono/src/sandbox/macos.rs` - range-aware `push_localhost_tcp_outbound_seatbelt_rules`, cumulative-cap check in `generate_profile`, `has_localhost_tcp` gating fix, byte-count debug logging, 6 tests
- `crates/nono/src/sandbox/linux.rs` - range-unroll `NetPort` loop in `apply_with_abi`, 2 original tests (`test_port_range_expands_landlock_bind_rules_for_every_port`, `test_port_range_no_artificial_cap_on_large_range`) plus 2 test helpers
- `crates/nono-cli/src/exec_strategy.rs` - `SupervisorConfig.proxy_bind_port_ranges` field, 9 test-fixture literal updates
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` - `SYS_BIND` arm union check, `make_config_with_ranges` helper, 3 tests
- `crates/nono-cli/src/supervised_runtime.rs` - `proxy_bind_port_ranges: caps.localhost_port_ranges().to_vec()` at the real `SupervisorConfig` construction site

## Decisions Made
- Confirmed via the vendored `landlock-0.4.4/src/net.rs:109` source (not assumed) that `NetPort::new` takes only a single `u16` — no native range API — settling the previously-UNVERIFIED question from `110-PATTERNS.md`/`110-RESEARCH.md` and confirming the Linux emitter must unroll per-port with no cap, exactly matching upstream's own implementation shape.
- Cumulative macOS cap check placed as the first statement of `generate_profile`, before any profile string construction, so the error path never wastes work building a profile that will be rejected.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing `#[cfg(target_os = "linux")]` on 9 new `proxy_bind_port_ranges: Vec::new(),` test-fixture lines**
- **Found during:** Task 3 verification (mandatory cross-target gate: `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin --all-targets`)
- **Issue:** The mechanical sibling-line insertion (matching each existing `proxy_bind_ports: Vec::new(),`) omitted the `#[cfg(target_os = "linux")]` attribute that guards the existing line, causing `E0560: struct has no field named proxy_bind_port_ranges` on any non-Linux target's test compilation (the field itself is Linux-only-declared on `SupervisorConfig`).
- **Fix:** Added the matching `#[cfg(target_os = "linux")]` attribute above each of the 9 new lines via a targeted regex substitution; re-verified with a self-consistent count gate (`grep -c` for both field names both return 9) and a clean re-run of `cargo-zigbuild clippy --all-targets` (exit 0).
- **Files modified:** `crates/nono-cli/src/exec_strategy.rs`
- **Verification:** `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin --all-targets -- -D warnings -D clippy::unwrap_used` exits 0 (previously exited 101 with 8 `E0560` errors)
- **Committed in:** `5d0c8e34` (Task 3 commit — caught and fixed before that commit landed)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Necessary correctness fix for cross-platform compilation; caught by the plan's own mandated cross-target gate exactly as intended. No scope creep.

## Issues Encountered

**Pre-existing, unrelated compile break discovered in `supervisor_linux.rs`'s `mod tests::network_decision` (out of scope, not fixed — see `deferred-items.md`).** While going beyond the plan's mandated verification (running `cross test`, which compiles `#[cfg(test)]` code, vs. the checklist-mandated `cross clippy` which does not), the module's `DenyAllBackend` test helper was found to implement a stale `ApprovalBackend` trait shape (`request_approval(&self, _req: &ApprovalRequest)`) that predates a later rename to `request_capability(&self, _req: &CapabilityRequest)`, plus a `tool_sandbox_runtime: None` field on `SupervisorConfig` that doesn't exist at all. `git log -L`/`git stash` trace this to commit `1a804977` ("fix(96-01): restore linux-gnu cfg-gated fork invariants...", 2026-06-26) and confirm it predates this plan entirely. This blocks *execution* (not compilation-via-clippy) of this plan's 3 new `supervisor_linux` tests on Linux, though the surrounding production code (the `SupervisorConfig` field and the `SYS_BIND` arm) is proven correct via a clean `cross clippy` pass on both mandatory targets. Per the Scope Boundary rule, this pre-existing defect (unrelated to PROF-03) was documented in `deferred-items.md` and NOT fixed in this plan.

## Known Stubs

None.

## Threat Flags

None — this plan implements the mitigations already named in its own `<threat_model>` (T-110-07/T-110-08/T-110-09/T-110-SC), introduces no new network endpoints, auth paths, or schema changes at a trust boundary beyond what the plan's threat register already covers.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `CapabilitySet.localhost_port_ranges()` is ready for plan 110-04 (profile schema `NetworkConfig.open_port_range`/`listen_port_range`) and 110-05 (`capability_ext.rs`/manifest layer wiring) to drive it — the mechanism and both Unix emitters are proven end-to-end.
- Plan 110-06 (Windows WFP-native range emitter) can now read `caps.localhost_port_ranges()` the same way `caps.localhost_ports()` is read today in `compile_network_policy`.
- **Carry-forward for whoever next touches `supervisor_linux.rs`'s test module:** the pre-existing `mod network_decision` compile break (see Issues Encountered / `deferred-items.md`) should be fixed before relying on `cargo test`/`cross test` executing any test in that module on Linux.

---
*Phase: 110-profile-policy-absorb-platform-overrides*
*Completed: 2026-07-30*

## Self-Check: PASSED

All 6 modified source files + SUMMARY.md + deferred-items.md confirmed present on disk. All 5
commit hashes (`a0bf9076`, `0c747739`, `5d0c8e34`, `2102f538`, `a6ac7d46`) confirmed present in
`git log --oneline --all`.
