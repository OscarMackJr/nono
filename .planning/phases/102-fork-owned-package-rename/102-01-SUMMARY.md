---
phase: 102-fork-owned-package-rename
plan: 01
subsystem: infra
tags: [cargo, crates.io, package-rename, workspace-manifests, PUB-01]

# Dependency graph
requires:
  - phase: 101-verify-gate-hardening-azure-profile-confirmation
    provides: verify-gate hardening context (independent workstream; no direct code dependency)
provides:
  - crates.io [package] name renamed on the 3-crate publish set (nono, nono-proxy, nono-cli -> nono-sandbox, nono-sandbox-proxy, nono-sandbox-cli)
  - Every in-workspace dependent (nono-proxy, nono-cli, nono-shell-broker, nono-ffi) resolves the renamed packages via `package =` dependency keys, table keys and all `use nono::`/`use nono_proxy::` imports fully preserved
  - Live-confirmed registry-name availability (crates.io x3, PyPI, npm-scoped) as of 2026-07-03
  - Regenerated Cargo.lock with exactly 3 renamed [[package]] hunks, zero third-party drift
affects: [102-02-makefile-ci-reconciliation, 102-03-nono-py-rename, 102-04-nono-ts-rename, 102-05-phase-gate, 105-live-publish]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cargo package= rename key: dependency table KEY stays the pre-rename name (nono/nono-proxy); add package = \"<new-crates-io-name>\" to preserve every `use nono::`/`use nono_proxy::` import unchanged across all consumers, including publish=false crates"
    - "Explicit [lib] name pin required alongside a [package] name rename whenever the crate has its own tests/*.rs integration tests that reference the crate by its bare (pre-rename) name -- otherwise Cargo's default-derived lib name (hyphens->underscores of the new package name) silently breaks those in-crate tests, independent of and in addition to the package= consumer mechanism"

key-files:
  created: []
  modified:
    - crates/nono/Cargo.toml
    - crates/nono-proxy/Cargo.toml
    - crates/nono-cli/Cargo.toml
    - crates/nono-shell-broker/Cargo.toml
    - bindings/c/Cargo.toml
    - Cargo.lock

key-decisions:
  - "Registry availability gate (Task 1) run live before any manifest edit: all 5 names (crates.io nono-sandbox/nono-sandbox-proxy/nono-sandbox-cli, PyPI nono-sandbox, npm @oscarmackjr/nono-ts) confirmed 404 (available) on 2026-07-03, one day after the 102-RESEARCH.md 2026-07-02 pass -- re-confirms availability held across the gap"
  - "Rule 3 auto-fix: added explicit [lib] name = \"nono\" to crates/nono/Cargo.toml -- discovered empirically via cargo build --workspace --all-targets, NOT anticipated by 102-RESEARCH.md's scratch-workspace mechanism test (which only exercised cross-crate producer/consumer pairs, not a crate's own in-crate integration tests referencing itself by its pre-rename bare name). Verified this pin has zero effect on the package= consumer mechanism (consumer extern names are controlled solely by the dependency table key, per 102-RESEARCH.md's own empirically-verified finding, independent of producer [lib] name)."

patterns-established:
  - "Any future [package] name rename on a crate with tests/*.rs integration tests must also check whether those tests reference the crate by its pre-rename bare name, and pin [lib] name explicitly if so -- this is a distinct failure mode from the well-known dependency-key-vs-package= consumer pitfall"

requirements-completed: [PUB-01]

duration: 12min
completed: 2026-07-03
---

# Phase 102 Plan 01: Fork-Owned Package Rename (Crate Publish Set) Summary

**Renamed the 3-crate Cargo publish set (nono, nono-proxy, nono-cli) to the fork-owned nono-sandbox family via `[package] name` + `package =` dependency keys, after live-confirming all 5 target registry names are unclaimed -- zero `use nono::`/`use nono_proxy::` import changed anywhere in the workspace.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-07-03T13:28:57Z
- **Completed:** 2026-07-03T13:33:42Z
- **Tasks:** 3
- **Files modified:** 6 (5 Cargo.toml manifests + Cargo.lock)

## Accomplishments
- Live-verified all 5 new registry identities (crates.io nono-sandbox / nono-sandbox-proxy / nono-sandbox-cli, PyPI nono-sandbox, npm @oscarmackjr%2Fnono-ts) return HTTP 404 (available) before touching any manifest
- Renamed `[package] name` on the 3-crate publish set (nono -> nono-sandbox, nono-proxy -> nono-sandbox-proxy, nono-cli -> nono-sandbox-cli); zero dependency-table keys renamed
- Reconciled all 5 in-workspace dependents (nono-proxy, nono-cli, nono-shell-broker, nono-ffi, plus the newly-required `[lib]` pin on nono itself) with correct `package =` keys or lib-name pin
- `cargo build --workspace --all-targets` exits 0; Cargo.lock regenerated with exactly 3 renamed `[[package]]` hunks and zero third-party drift

## Task Commits

Each task was committed atomically:

1. **Task 1: Live registry availability gate (SC3)** - read-only network verification, no files changed, no commit (all 5 checks logged below)
2. **Task 2: Rename the 3-crate publish set + wire package= keys** - `4548d548` (feat)
3. **Task 3: Patch remaining in-workspace dependents + regenerate Cargo.lock + verify workspace compiles** - `ade0036e` (feat)

**Plan metadata:** (pending) `docs(102-01): complete plan`

## Task 1 Evidence (live registry checks, 2026-07-03)

| Registry | URL | Status |
|----------|-----|--------|
| crates.io | `https://crates.io/api/v1/crates/nono-sandbox` | 404 |
| crates.io | `https://crates.io/api/v1/crates/nono-sandbox-proxy` | 404 |
| crates.io | `https://crates.io/api/v1/crates/nono-sandbox-cli` | 404 |
| PyPI | `https://pypi.org/pypi/nono-sandbox/json` | 404 |
| npm | `https://registry.npmjs.org/@oscarmackjr%2Fnono-ts` | 404 |

All 5 available -- gate passed, rename proceeded.

## Files Created/Modified
- `crates/nono/Cargo.toml` - `[package] name` -> `nono-sandbox`; added explicit `[lib] name = "nono"` (Rule 3 auto-fix, see Deviations)
- `crates/nono-proxy/Cargo.toml` - `[package] name` -> `nono-sandbox-proxy`; `nono` dependency gets `package = "nono-sandbox"` (key unchanged)
- `crates/nono-cli/Cargo.toml` - `[package] name` -> `nono-sandbox-cli`; `nono`/`nono-proxy` dependencies get `package =` keys (keys unchanged); `[[bin]]` names `nono`/`nono-agentd` untouched; feature-forwarding (`nono/system-keyring`) untouched
- `crates/nono-shell-broker/Cargo.toml` - `nono` dependency gets `package = "nono-sandbox"` (publish=false, own name unchanged)
- `bindings/c/Cargo.toml` (nono-ffi) - `nono` dependency gets `package = "nono-sandbox"` (publish=false, own `[package]`/`[lib]` names unchanged)
- `Cargo.lock` - regenerated via `cargo build --workspace --all-targets`; exactly 3 renamed `[[package]] name` hunks, zero third-party drift

## Decisions Made
- Re-ran the live registry-availability checks fresh at execution time (2026-07-03) rather than trusting the 2026-07-02 research pass verbatim, since name availability is explicitly time-sensitive (102-RESEARCH.md: "Valid until: 7 days ... re-verify immediately before"). All 5 held available across the one-day gap.
- Followed the plan's `package =` mechanism exactly as documented for all 5 in-workspace dependency edits — no dependency-table key was ever renamed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added explicit `[lib] name = "nono"` to crates/nono/Cargo.toml**
- **Found during:** Task 3 (`cargo build --workspace --all-targets` verification step)
- **Issue:** `crates/nono/tests/manifest_types.rs` (the crate's own integration test) uses bare `use nono::capability::{...}`, `use nono::manifest::CapabilityManifest`, `use nono::CapabilitySet`. Integration tests under `tests/` link against the crate under test via its default-derived library crate name, which Cargo computes from `[package] name` with hyphens converted to underscores when no explicit `[lib]` section exists. After renaming `[package] name` to `nono-sandbox`, that default silently became `nono_sandbox`, breaking these three `use nono::` imports with `E0432`/`E0433` — a distinct failure mode from (and not covered by) the well-documented dependency-key-vs-`package=` consumer pitfall that 102-RESEARCH.md's scratch-workspace test exercised (that test only covered cross-crate producer/consumer pairs, not a crate's own in-crate integration tests referencing itself by its pre-rename bare name).
- **Fix:** Added an explicit `[lib]\nname = "nono"` section to `crates/nono/Cargo.toml`, pinning the library crate name to stay `nono` regardless of the `[package] name` rename. Verified this has zero effect on the `package=` consumer mechanism used by nono-proxy/nono-cli/nono-shell-broker/nono-ffi, since (per 102-RESEARCH.md's own empirically-verified finding) a consumer's extern crate name is controlled entirely by its OWN dependency-table key, independent of the producer's `[lib]` name.
- **Files modified:** `crates/nono/Cargo.toml`
- **Verification:** `cargo build --workspace --all-targets` exits 0 (previously failed with 3x E0432/E0433 in `crates/nono/tests/manifest_types.rs` before this fix); all Task 2/3 grep acceptance criteria re-checked and still pass with the `[lib]` section present (no acceptance criterion checks for its absence)
- **Committed in:** `ade0036e` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary for `cargo build --workspace --all-targets` to compile at all under the new package names — a hard Task 3 acceptance criterion. No scope creep: the fix is a single narrowly-scoped manifest addition with a documented zero-impact analysis on the consumer-side `package=` mechanism, and does not touch any `use nono::`/`use nono_proxy::` statement anywhere in the workspace (the plan's own core objective is fully preserved, arguably more completely than the plan anticipated).

## Issues Encountered
None beyond the deviation documented above.

## User Setup Required
None - no external service configuration required. (Note: the npm `@oscarmackjr` scope has no owning npm user/org account yet — this is a documented non-blocking Phase 105 precondition per 102-RESEARCH.md Pitfall 4, not a Phase 102 gap.)

## Next Phase Readiness
- The 3-crate publish-set rename (`nono`/`nono-proxy`/`nono-cli` -> `nono-sandbox`/`nono-sandbox-proxy`/`nono-sandbox-cli`) is complete and workspace-build-green, unblocking Plan 102-02 (Makefile/CI reconciliation), which will need to reference the new crate names in any hardcoded `-p nono`/`-p nono-cli` style invocations.
- Plans 102-03 (nono-py) and 102-04 (nono-ts) still need their own sibling-repo `Cargo.toml` `package=` edits (per 102-RESEARCH.md Pitfall 2 — invisible to this repo's own `cargo build --workspace`), and their own registry-facing identity changes (`pyproject.toml [project] name`, `napi rename --package-name`).
- No blockers. The `[lib]` name pin (Rule 3 deviation) should be called out explicitly to Plan 102-02/105 authors so a future contributor doesn't "clean it up" thinking it's redundant with the `package=` mechanism — it is not; it fixes a genuinely distinct problem.

---
*Phase: 102-fork-owned-package-rename*
*Completed: 2026-07-03*

## Self-Check: PASSED

- FOUND: `.planning/phases/102-fork-owned-package-rename/102-01-SUMMARY.md`
- FOUND: commit `4548d548`
- FOUND: commit `ade0036e`
