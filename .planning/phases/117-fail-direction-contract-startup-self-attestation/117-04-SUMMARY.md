---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 04
subsystem: infra
tags: [windows, wfp, cargo-feature, fault-injection, clippy, cross-target]

# Dependency graph
requires:
  - phase: 117-01
    provides: layer registry inventory (LayerId, LayerRegistryEntry) that Plans 06/07 will drive per-layer force-unavailable hooks off of
provides:
  - "layer-fault-injection Cargo feature on crates/nono-cli (default-off, not part of `default = [...]`)"
  - "WFP force-ready toggle (WINDOWS_WFP_TEST_FORCE_READY, set_windows_wfp_test_force_ready, windows_wfp_test_force_ready, --dangerous-force-wfp-ready) fully compiled out of default builds"
  - "SC4-4 resolved: no runtime-gated confinement-disabling toggle remains in the release binary"
  - "Reusable #[cfg(feature = \"layer-fault-injection\")] pattern for Plans 06/07 to gate the remaining layers' force-unavailable hooks behind"
affects: [117-06, 117-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Compiled-out (not runtime-refused) fault-injection seam via a default-off Cargo feature — mirrors the existing test-trust-overrides precedent in crates/nono-cli/src/trust_cmd.rs"
    - "clap derive field cfg-gating: #[cfg(feature = \"...\")] directly above a struct field compiles cleanly with clap's derive macro (same pattern already used for the macOS-only trust_proxy_ca field)"

key-files:
  created: []
  modified:
    - crates/nono-cli/Cargo.toml
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/exec_strategy_windows/network.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/command_runtime.rs

key-decisions:
  - "Removed dangerous_force_wfp_ready from the default build entirely (cfg-gated the struct field) rather than leaving a present-but-inert flag, per the plan's own reconnaissance-surface reasoning."
  - "windows_wfp_test_force_ready()'s force-ready branch in network.rs is itself #[cfg(feature = \"layer-fault-injection\")]-gated at the call site, so the reader function does not need a no-op fallback stub for the default build — it simply does not exist."

patterns-established:
  - "Pattern: any later layer's force-unavailable hook (Plans 06/07) should follow this exact shape — static + setter + reader all under #[cfg(feature = \"layer-fault-injection\")], the call site wrapped in the same cfg rather than calling through a runtime no-op, and any CLI-flag plumbing (cli.rs struct field + command_runtime.rs wiring + any From/struct-literal conversions) cfg-gated identically."

requirements-completed: [CINT-03]

# Metrics
duration: 30min
completed: 2026-08-09
---

# Phase 117 Plan 04: Compiled-Out WFP Fault-Injection Seam Summary

**Migrated the shipped `WINDOWS_WFP_TEST_FORCE_READY` / `--dangerous-force-wfp-ready` toggle from a runtime `NONO_TEST_HARNESS` env-var gate to a `#[cfg(feature = "layer-fault-injection")]` compiled-out shape, closing SC4-4.**

## Performance

- **Duration:** ~30 min
- **Completed:** 2026-08-09T23:41:39Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Added a default-off `layer-fault-injection = []` Cargo feature to `crates/nono-cli/Cargo.toml`, documented as never-enable-in-release (D-30).
- Removed the runtime `NONO_TEST_HARNESS` env-var check entirely — `grep -c "NONO_TEST_HARNESS" crates/nono-cli/src/exec_strategy_windows/mod.rs` is now 0. The feature gate is the only gate.
- Compiled the static `WINDOWS_WFP_TEST_FORCE_READY`, `set_windows_wfp_test_force_ready`, `windows_wfp_test_force_ready`, the `network.rs` force-ready branch, and the `--dangerous-force-wfp-ready` CLI flag (`cli.rs`) out of the default build entirely — not merely refused at runtime. In a default build, none of these symbols exist in the compiled artifact.
- Verified both build configurations (`cargo build`/`cargo clippy`, with and without `--features layer-fault-injection`) compile and lint clean, plus `cargo fmt --all -- --check`.
- Ran both cross-target clippy gates (`cross clippy` for `x86_64-unknown-linux-gnu`, `cargo-zigbuild clippy` for `x86_64-apple-darwin`) in both feature configurations, since this plan's changes touch `cli.rs` and `command_runtime.rs`, both of which carry `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` blocks elsewhere in the file (in-scope per `.planning/templates/cross-target-verify-checklist.md`) — all four runs clean.
- Confirmed `crates/nono-cli/tests/layer_registry_selfcheck.rs`'s discovery-based drift tests (from Plan 03) still pass unmodified.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add layer-fault-injection Cargo feature** - `eee8bf50` (feat)
2. **Task 2: Migrate the WFP toggle to the compiled-out feature; remove NONO_TEST_HARNESS gate** - `b6f52386` (fix)

## Files Created/Modified
- `crates/nono-cli/Cargo.toml` - New `layer-fault-injection = []` feature, default-off, documented never-enable-in-release
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` - `WINDOWS_WFP_TEST_FORCE_READY` static, `set_windows_wfp_test_force_ready`, `windows_wfp_test_force_ready` all gated `#[cfg(feature = "layer-fault-injection")]`; `AtomicBool`/`Ordering` import gated to match; module-doc dead_code comment updated
- `crates/nono-cli/src/exec_strategy_windows/network.rs` - The force-ready branch in `probe_wfp_backend_status_with_config` gated `#[cfg(feature = "layer-fault-injection")]`; falls through unchanged to the real `nono-wfp-service` IPC check on the default path
- `crates/nono-cli/src/cli.rs` - `dangerous_force_wfp_ready` field on `SandboxArgs` gated `#[cfg(feature = "layer-fault-injection")]` (absent from default build, not left as a hidden no-op); the `WrapSandboxArgs -> SandboxArgs` conversion's field assignment gated to match
- `crates/nono-cli/src/command_runtime.rs` - Wiring block gated `#[cfg(all(target_os = "windows", feature = "layer-fault-injection"))]`

## Decisions Made
- Removed the CLI flag from the default build's struct definition entirely (rather than leaving a dead no-op flag), per the plan's own reasoning that a present-but-inert flag is still discoverable reconnaissance for an attacker probing `--help`/binary strings.
- Kept `windows_wfp_test_force_ready()`'s consumption site in `network.rs` cfg-gated inline at the call site (rather than providing an always-false fallback function for the default build), so the function itself is fully absent — not present-but-returning-false — when the feature is off.

## Deviations from Plan

None beyond mechanical follow-through required by the plan's own acceptance criteria: the plan's `<interfaces>` cited `network.rs` lines "~1690-1800" for the readiness-check dispatch, but the actual call site was at `network.rs:381-390` (`probe_wfp_backend_status_with_config`) — confirmed by grep before editing, not a deviation in approach, just a stale line citation in the plan's interface notes.

## Issues Encountered
- Two build-time errors on first pass (E0560 missing struct field in the `WrapSandboxArgs -> SandboxArgs` conversion at `cli.rs:2799`, and an unused-import warning for `AtomicBool`/`Ordering` under `-D warnings`) were found and fixed inline while iterating toward the default-features build, both structurally required to satisfy the plan's own acceptance criteria (clean `cargo build`/`clippy` in both configurations) — not scope additions.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- The `#[cfg(feature = "layer-fault-injection")]` pattern established here (static + setter + reader all feature-gated, call site wrapped in the same cfg, CLI/wiring plumbing cfg-gated identically) is ready for Plans 06/07 to replicate for the remaining registry layers.
- No blockers identified.

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-09*
