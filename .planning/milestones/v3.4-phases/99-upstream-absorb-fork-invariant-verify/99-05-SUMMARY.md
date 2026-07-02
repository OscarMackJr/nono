---
phase: 99-upstream-absorb-fork-invariant-verify
plan: "05"
subsystem: deps
tags: [sigstore, cargo, proxy, docs, upstream-sync]

# Dependency graph
requires:
  - phase: 99-03
    provides: Clusters D and E absorbed (linux.rs 9P warning; org-ref migration)
provides:
  - "sigstore-trust-root =0.9.0 pinned in crates/nono/Cargo.toml (Cluster F)"
  - "Cargo.lock regenerated with sigstore-trust-root/crypto/tuf/types 0.9.0 added"
  - "X-Nono-Token stale claim removed from nono-proxy README and token.rs (Cluster G)"
  - "Deprecated flag warnings restructured with remove_by metadata in cli_bootstrap.rs"
affects: [99-06, phase-100]

# Tech tracking
tech-stack:
  added:
    - "sigstore-trust-root 0.9.0 (direct dep pin in crates/nono/Cargo.toml)"
    - "sigstore-crypto 0.9.0, sigstore-tuf 0.9.0, sigstore-types 0.9.0 (transitive via trust-root 0.9.0)"
  patterns:
    - "Dual-version sigstore resolution: trust-root 0.9.0 (nono direct) + 0.8.0 (sigstore-verify 0.8.0 transitive) coexist"
    - "3-tuple (legacy, replacement, remove_by) pattern for legacy flag warnings in cli_bootstrap.rs"

key-files:
  created: []
  modified:
    - "crates/nono/Cargo.toml — sigstore-trust-root =0.9.0 direct dep pin added"
    - "Cargo.lock — regenerated with sigstore-trust-root 0.9.0 and 3 new sigstore-{crypto,tuf,types} 0.9.0 packages"
    - "crates/nono-proxy/README.md — stale X-Nono-Token/NONO_PROXY_TOKEN claims corrected"
    - "crates/nono-proxy/src/token.rs — module doc updated to accurately describe transparent credential delivery"
    - "crates/nono-cli/src/cli_bootstrap.rs — deprecated flag warnings restructured with remove_by field"

key-decisions:
  - "Cluster F cascade evaluation: sigstore-trust-root 0.9.0 resolves alongside sigstore-verify 0.8.0 via dual-version (resolver kept 0.8.0 for sigstore-verify; 0.9.0 is a new direct pin for nono). make build exits 0. sigstore-verify bump deferred to a future sync phase."
  - "cli.rs ALIAS annotation change skipped: fork's cli.rs lacks the ALIAS annotation convention; the behavioral change (remove_by metadata in warning messages) is fully captured in cli_bootstrap.rs."
  - "cargo audit result: 5 pre-existing UNMAINTAINED/UNSOUND warnings (async-std, fxhash, paste, rustls-pemfile, anyhow); no HIGH/CRITICAL advisories for any new sigstore packages."

patterns-established:
  - "Cluster F cascade procedure: add trust-root pin, run cargo update -p, inspect lock diff, build, audit — before deciding whether to co-bump sigstore-verify"

requirements-completed:
  - UPST11-03

# Metrics
duration: 13min
completed: "2026-06-30"
---

# Phase 99 Plan 05: Clusters F+G Upstream Absorb Summary

**sigstore-trust-root bumped to =0.9.0 with dual-version cascade resolution; X-Nono-Token stale documentation removed from proxy README, token.rs, and cli_bootstrap.rs deprecated-flag warnings updated with removal timeline metadata**

## Performance

- **Duration:** 13 min
- **Started:** 2026-06-30T15:24:04Z
- **Completed:** 2026-06-30T15:37:40Z
- **Tasks:** 2
- **Files modified:** 5 (+ Cargo.lock)

## Accomplishments

- Replayed upstream 2e64798d (Cluster F): added `sigstore-trust-root = "=0.9.0"` direct dep pin to `crates/nono/Cargo.toml`; `cargo update -p sigstore-trust-root` resolved via dual versioning (trust-root 0.9.0 alongside 0.8.0); `make build` exits 0; `cargo audit` shows no HIGH/CRITICAL advisories.
- Replayed upstream a4d68189 (Cluster G): removed stale X-Nono-Token/NONO_PROXY_TOKEN claims from README.md and token.rs module doc; updated `cli_bootstrap.rs` deprecated flag warnings to 3-tuple with `remove_by` field (`--proxy-credential` and `NONO_PROXY_CREDENTIAL` now warn "Will be removed in v1.0.0").
- All proxy tests pass: `cargo test -p nono-proxy --lib` exits 0, 176/176 passed.

## Task Commits

1. **Task 1: Cluster F sigstore-trust-root bump** - `91b3bdc3` (chore)
2. **Task 2: Cluster G proxy docs + deprecated flag fix** - `18719f11` (docs)

## Files Created/Modified

- `crates/nono/Cargo.toml` — added `sigstore-trust-root = "=0.9.0"` direct dep with Phase 99 comment explaining dual-version rationale
- `Cargo.lock` — regenerated: 4 new packages (sigstore-trust-root 0.9.0, sigstore-crypto 0.9.0, sigstore-tuf 0.9.0, sigstore-types 0.9.0) added alongside existing 0.8.0 versions
- `crates/nono-proxy/README.md` — corrected "Session token authentication" bullet (removed X-Nono-Token reference; clarified transparent credential shadowing mechanism); updated code comment (removed NONO_PROXY_TOKEN reference)
- `crates/nono-proxy/src/token.rs` — module-level doc updated: title now "...validation, and nonce resolution"; description now accurately describes transparent credential route delivery and Proxy-Authorization for CONNECT tunnels
- `crates/nono-cli/src/cli_bootstrap.rs` — `collect_legacy_network_warnings()` restructured from 2-tuple `(legacy, replacement)` to 3-tuple `(legacy, replacement, remove_by)` for both CLI flag and env var loops; `--proxy-credential`/`NONO_PROXY_CREDENTIAL` carry `Some("v1.0.0")` in `remove_by`

## Decisions Made

- **Cluster F cascade evaluation:** `cargo update -p sigstore-trust-root` showed the resolver added sigstore-trust-root 0.9.0 as a new direct dep without bumping sigstore-verify from 0.8.0. This dual-version resolution (nono uses trust-root 0.9.0 directly; sigstore-verify 0.8.0 continues to use 0.8.0 transitively) is confirmed compatible — `make build` exits 0, `cargo audit` clean. sigstore-verify 0.8.0→0.9.0 (upstream commit 9e084cbb) was absorbed out-of-window and is deferred to a future sync phase.
- **cli.rs ALIAS annotation absent:** The upstream a4d68189 diff changes `remove_by="indefinite"` to `remove_by="v1.0.0"` on an ALIAS annotation line that does not exist in the fork's diverged cli.rs. Skipped — the behavioral change (warning message with removal date) is fully captured in cli_bootstrap.rs.
- **Pre-existing nono-cli test failures:** `cargo test -p nono-cli` shows 11 failures (config, profile_cmd, protected_paths, audit_session) — all confirmed pre-existing Windows host env failures per project memory, unrelated to Task 2 changes.

## Deviations from Plan

### Auto-fixed Issues

None — plan steps followed exactly as specified.

### Clarifications / Manual Replay Decisions

**1. Cluster F cascade: dual-version accepted (not a co-bump)**
- The plan specified "If YES (sigstore-verify pulled to 0.9.0 by resolver): also edit... If NO (resolver keeps sigstore-verify 0.8.0): confirm trust-root 0.9.0 / verify 0.8.0 coexist cleanly."
- Resolver kept sigstore-verify 0.8.0; dual-version resolution accepted. Build clean. Assumption A1 from research confirmed favorable.

**2. cli.rs ALIAS annotation: absent in fork — skipped**
- Plan says "Apply the +2/-2 changes (deprecated flag metadata)." Those lines do not exist in the fork's cli.rs (diverged). No equivalent line to change. Behavioral intent fully captured in cli_bootstrap.rs.

---

**Total deviations:** 0 auto-fixed; 2 clarifications (both within plan's described decision tree).

## Issues Encountered

None — both commits applied cleanly.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. Cluster G changes are doc/metadata only. Cluster F adds a new crate version to the dep graph but introduces no new callable surface in production code.

## Known Stubs

None — no placeholder values introduced.

## Next Phase Readiness

- Clusters F and G are fully absorbed; both will-sync rows closed in the Phase 98 ledger.
- `sigstore-trust-root = "=0.9.0"` is pinned; `make build` is GREEN; `cargo audit` is clean.
- Plan 99-06 (Cluster C — proxy HTTP/2 + endpoint routing split) can proceed. Plans 99-04 and 99-05 are both complete (no file overlap; Wave 4 parallel execution done).

---

*Phase: 99-upstream-absorb-fork-invariant-verify*
*Completed: 2026-06-30*
