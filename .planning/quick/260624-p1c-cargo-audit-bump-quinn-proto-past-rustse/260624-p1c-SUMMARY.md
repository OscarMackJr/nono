---
phase: quick-260624-p1c
plan: "01"
subsystem: deps
tags: [security, cargo-audit, quinn-proto, RUSTSEC-2026-0185]
dependency_graph:
  requires: []
  provides: [clean-cargo-audit]
  affects: [Cargo.lock]
tech_stack:
  added: []
  patterns: []
key_files:
  created: []
  modified:
    - Cargo.lock
decisions:
  - "Lockfile-only update (primary path): quinn-proto 0.11.14 -> 0.11.15 via `cargo update -p quinn-proto`; no Cargo.toml edits needed."
  - "Task 2 (allow-list fallback) skipped: patched 0.11.x was available in the registry."
metrics:
  duration: "~10 minutes"
  completed: "2026-06-24"
---

# Phase quick-260624-p1c Plan 01: Cargo Audit — Bump quinn-proto Past RUSTSEC-2026-0185 Summary

**One-liner:** Lockfile-only bump of `quinn-proto` from 0.11.14 to 0.11.15 via `cargo update -p quinn-proto`, resolving the sole `cargo audit` hard error (RUSTSEC-2026-0185 remote memory exhaustion).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Confirm dep chain and attempt semver-compatible lockfile update | 78b50f04 | Cargo.lock |
| 2 | Allow-list fallback | SKIPPED (patched version available) | — |
| 3 | Verify workspace build and commit the fix | 78b50f04 | Cargo.lock |

## Outcome

**Path taken:** Primary (lockfile update). `cargo update -p quinn-proto` found `quinn-proto 0.11.15` in the registry, which contains the upstream fix. No `.cargo/audit.toml` changes were needed.

**Dependency chain confirmed:** `reqwest → quinn 0.11.9 → quinn-proto 0.11.14 → 0.11.15`

Note: `cargo tree -i quinn-proto --workspace` reported nothing to print on this Windows host (reqwest pulls quinn via a non-default feature gate that the current platform target doesn't compile). The version was confirmed directly in `Cargo.lock`.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo audit` — no `error:` lines | PASS |
| RUSTSEC-2026-0185 absent from output | PASS |
| `cargo build --workspace` | PASS (48s, all 5 crates) |
| 4 `unmaintained` warnings remain (async-std, fxhash, paste, rustls-pemfile) | Expected — out of scope |
| DCO sign-off on commit | PASS |
| Todo moved to `.planning/todos/done/` | PASS |

## Deviations from Plan

None — plan executed exactly as written. The primary path (Task 1 step 3a) applied: a patched `quinn-proto 0.11.x` was available, so `cargo update -p quinn-proto` advanced the lockfile. Task 2 (allow-list fallback) was correctly skipped.

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced. The change is a lockfile pin advancement that removes a known vulnerability.

## Self-Check: PASSED

- `Cargo.lock` modified: FOUND (verified via grep — `quinn-proto` at 0.11.15)
- Commit `78b50f04` exists: FOUND (`git log -1` confirms)
- `.planning/todos/done/20260624-cargo-audit-quinn-proto.md` exists: FOUND
- `cargo audit` zero `error:` lines: CONFIRMED
