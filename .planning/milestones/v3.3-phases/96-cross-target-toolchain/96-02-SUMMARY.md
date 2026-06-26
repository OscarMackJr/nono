---
phase: 96-cross-target-toolchain
plan: 02
subsystem: build-tooling / cross-target-verification
tags: [cross-target, clippy, apple-darwin, cargo-zigbuild, zig, XTGT-03]
requires:
  - "96-01 linux-gnu gate GREEN (sibling; post-Phase-95 synced tree)"
provides:
  - "apple-darwin clippy gate = LOCAL-RUNNABLE (cargo-zigbuild clippy, exit 0, SDKROOT unset)"
  - "XTGT-03 resolved via the D-04 clean-exit branch (not the hard-blocker branch)"
  - "Plan-03 handoff: apple-darwin Q3 branch flips to MUST-run-locally; PARTIAL→CI default retired"
affects:
  - ".planning/templates/cross-target-verify-checklist.md (Plan 03 rewrite consumes this)"
  - "CLAUDE.md cross-target bullet (Plan 03 one-line pointer)"
tech-stack:
  added:
    - "zig 0.16.0 (host tool, winget zig.zig)"
    - "cargo-zigbuild 0.23.0 (host tool, cargo install --locked)"
  patterns:
    - "cargo-zigbuild clippy --target x86_64-apple-darwin (direct-binary form; cargo zigbuild clippy mis-parses)"
key-files:
  created:
    - ".planning/phases/96-cross-target-toolchain/96-02-XTGT-APPLE-DARWIN-RECORD.md"
  modified: []
decisions:
  - "apple-darwin LOCAL-RUNNABLE (D-04 clean-exit branch) — bounded attempt exited 0, no SDK extraction"
metrics:
  duration_min: 14
  completed: 2026-06-26
---

# Phase 96 Plan 02: apple-darwin Cross-Target Clippy Gate Summary

apple-darwin clippy is now provably runnable locally on this Windows host via `cargo-zigbuild clippy` (zig 0.16.0 + cargo-zigbuild 0.23.0) — the single bounded attempt exited 0 with `SDKROOT` unset, so XTGT-03 closes through the D-04 clean-exit branch rather than the expected SDK-licensing hard-blocker.

## What Was Built

- Installed the Wave 2 host tools and recorded resolved versions: **zig 0.16.0** (`winget install zig.zig`) and **cargo-zigbuild 0.23.0** (`cargo install --locked cargo-zigbuild`).
- Ran **exactly one** bounded apple-darwin clippy attempt (D-03 one-shot cap):
  `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`,
  with `SDKROOT` confirmed unset and no macOS SDK extraction.
- Outcome: **exit 0** — `aws-lc-sys 0.41.0` / `aws-lc-rs 1.17.0` / `ring 0.17.14` C build-dep probes
  compiled under zig's bundled macOS target support, and all workspace crates (incl. the
  `#[cfg(target_os = "macos")]` surface: `core-foundation`, `security-framework`) linted clean.
- Wrote `96-02-XTGT-APPLE-DARWIN-RECORD.md`: tool versions, the verbatim single invocation, the
  captured clean-exit outcome, the invocation-form correction, and the disposition flipping
  apple-darwin to **LOCAL-RUNNABLE** with the Plan 03 handoff flag.

## Outcome vs. Expectation

The plan and research (assumption A3, Pitfall 5) flagged the very likely **D-04(b) SDK-licensing wall**
at the `aws-lc-sys` C feature probe. It did not materialize — zig 0.16.0 + cargo-zigbuild 0.23.0 bundled
enough macOS C target support to satisfy the probe without the proprietary SDK. Both D-04 branches were
pre-authorized as complete, passing outcomes; this plan landed on the favorable clean-exit branch (a).
Net result: **both** cross-targets (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`)
are now local-runnable on this host.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Corrected the apple-darwin invocation form**
- **Found during:** Task 1
- **Issue:** The plan's literal `cargo zigbuild clippy …` mis-parses. Under cargo's
  external-subcommand mechanism, `cargo zigbuild …` invokes `cargo-zigbuild` with `zigbuild` as
  argv[1], which collides with the binary's own `zigbuild` subcommand, leaving `clippy` as a stray
  argument. `cargo-zigbuild 0.23.0` also rejected `--version` on the `zigbuild` subcommand.
- **Fix:** Used the documented direct-binary form `cargo-zigbuild clippy …` (the binary exposes
  `clippy` as a first-class subcommand) and `cargo-zigbuild --version` for the version probe. This is
  the same clippy run the plan intended — one bounded attempt, same lints, same target.
- **Files modified:** none (invocation only; recorded in § 2 of the record for Plan 03 reproducibility)
- **Commit:** a3bbb564

No source code changed — the gate was clean on the first attempt, so no structural drift fixes and
no `#[allow(...)]` were needed (D-05 trivially satisfied).

## Authentication Gates

None.

## Known Stubs

None — this is a toolchain + record plan; no code or UI stubs introduced.

## Commits

- `a3bbb564` docs(96): record apple-darwin bounded clippy attempt — clean exit
- `4274f544` docs(96): resolve XTGT-03 — flip apple-darwin to local-runnable

## Self-Check: PASSED
- FOUND: .planning/phases/96-cross-target-toolchain/96-02-XTGT-APPLE-DARWIN-RECORD.md
- FOUND: commit a3bbb564
- FOUND: commit 4274f544
