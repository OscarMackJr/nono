---
phase: 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker
plan: 03
subsystem: release-bindings
tags: [nono-py, nono-ts, pyo3, napi-rs, version-bump, pypi, npm, cross-repo]

# Dependency graph
requires:
  - phase: 100-01
    provides: "nono workspace leapfrogged to crate version 0.66.1 across all 6 members + path-dep pins"
provides:
  - "nono-py repo committed at 0.66.1 with the endpoint_policy PyPI blocker (RLS-12) closed"
  - "nono-ts repo committed at 0.66.1 across all tracked manifests + regenerated package-lock.json"
affects: [100-04, 100-05, release-runbook]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Finish-in-place pattern for stale uncommitted binding-repo edits: never land the intermediate 0.66.0 value as its own commit, edit straight through to 0.66.1"
    - "Rule 3 blocking-issue auto-fix applied to a second missing-field drift (ProxyConfig.enable_h2) discovered only by actually running maturin build, not by the plan's static grep"

key-files:
  created: []
  modified:
    - C:/Users/OMack/nono-py/Cargo.toml
    - C:/Users/OMack/nono-py/pyproject.toml
    - C:/Users/OMack/nono-py/src/proxy.rs
    - C:/Users/OMack/nono-py/src/policy.rs
    - C:/Users/OMack/nono-ts/Cargo.toml
    - C:/Users/OMack/nono-ts/package.json
    - C:/Users/OMack/nono-ts/package-lock.json
    - C:/Users/OMack/nono-ts/npm/darwin-arm64/package.json
    - C:/Users/OMack/nono-ts/npm/darwin-x64/package.json
    - C:/Users/OMack/nono-ts/npm/linux-arm64-gnu/package.json
    - C:/Users/OMack/nono-ts/npm/linux-x64-gnu/package.json

key-decisions:
  - "D-01: both binding repos got the 0.66.1 bump AND the nono-py endpoint_policy fix, each as one separate DCO-signed commit in its own git repo (nono-py 84e8f18, nono-ts dd7d416) — no commit ever landed the intermediate 0.66.0 value"
  - "D-02: publish stayed operator-gated — only maturin build and npm publish --dry-run were run; no push/publish to PyPI or npm"
  - "D-07: minimal endpoint_policy: None stub added at both nono-py RouteConfig construction sites (src/proxy.rs, src/policy.rs), not full policy threading"
  - "Rule 3 (blocking issue) applied beyond the plan's literal scope: nono-py's ProxyConfig::new was also missing the enable_h2 field added to nono-proxy's ProxyConfig (same class of binding-drift as endpoint_policy) — maturin build failed with E0063 until fixed; added enable_h2: false alongside the two planned endpoint_policy additions"

requirements-completed: [RLS-10, RLS-12]

# Metrics
duration: 20min
completed: 2026-07-02
---

# Phase 100 Plan 03: Cross-Repo Binding Version Bump + nono-py PyPI Blocker Summary

**Finished the already-in-progress 0.66.0 -> 0.66.1 version bump in both separate binding repos (nono-py, nono-ts) and closed the nono-py RouteConfig/ProxyConfig PyPI blocker, landing exactly one DCO-signed commit per repo with the intermediate 0.66.0 value never committed.**

## Performance

- **Duration:** ~20 min (task work; excludes file-read/context-load overhead)
- **Tasks:** 2 completed
- **Files modified:** 4 in nono-py, 7 in nono-ts (11 total across two external repos)

## Accomplishments
- `nono-py` (`C:/Users/OMack/nono-py`): `Cargo.toml` + `pyproject.toml` finished from the
  stale uncommitted `0.66.0` to `0.66.1`; `endpoint_policy: None` added at both `RouteConfig`
  construction sites (`src/proxy.rs`'s `RouteConfig::new`, `src/policy.rs`'s
  `impl From<PolicyRouteConfig> for RustRouteConfig`), closing the carried-forward RLS-12
  PyPI blocker. `maturin build` exits 0, producing
  `nono_py-0.66.1-cp312-cp312-win_amd64.whl`. `python/nono_py/__init__.py` left untouched
  (`__version__ = "0.9.0"`, an independent marker per the plan).
- `nono-ts` (`C:/Users/OMack/nono-ts`): `Cargo.toml`, `package.json` (main version + all 5
  `optionalDependencies` entries), and all 4 tracked `npm/*/package.json` files
  (darwin-arm64, darwin-x64, linux-arm64-gnu, linux-x64-gnu) finished from `0.66.0` to
  `0.66.1`. `package-lock.json` regenerated via `npm install --package-lock-only` (previously
  stale at `0.4.0`, now syncs root name/version + `optionalDependencies` to `0.66.1`).
  `npm publish --dry-run` exits 0 with `index.js` + `index.d.ts` in the tarball manifest. The
  3 untracked scratch files (`decode-test.js`, `test-broker.js`, `test-confined.js`) and the
  `nono` path-dep's already-correct `version = "0.66"` caret pin were left untouched.
- One DCO-signed commit landed in each repo (`nono-py` `84e8f18`, `nono-ts` `dd7d416`); `git
  log -p` in both repos confirms the version string transitions directly `0.9.0`/`0.4.0` ->
  `0.66.1` with no intermediate `0.66.0` commit ever created.

## Task Commits

Each task was committed atomically, in its own external repository (not the nono repo):

1. **Task 1: Finish nono-py version bump to 0.66.1 + close the RouteConfig/ProxyConfig endpoint_policy/enable_h2 PyPI blocker** - `nono-py@84e8f18` (chore)
2. **Task 2: Finish nono-ts version bump to 0.66.1 across all tracked manifests** - `nono-ts@dd7d416` (chore)

## Files Created/Modified
- `C:/Users/OMack/nono-py/Cargo.toml` - `version = "0.9.0"` -> `"0.66.1"`
- `C:/Users/OMack/nono-py/pyproject.toml` - `version = "0.9.0"` -> `"0.66.1"`
- `C:/Users/OMack/nono-py/src/proxy.rs` - `endpoint_policy: None,` added to `RouteConfig::new`'s struct literal (after `tls_ca,`); `enable_h2: false,` added to `ProxyConfig::new`'s struct literal (after `direct_connect_ports: Vec::new(),`) — the latter is a Rule 3 auto-fix beyond the plan's literal scope, discovered only when `maturin build` failed with E0063
- `C:/Users/OMack/nono-py/src/policy.rs` - `endpoint_policy: None,` added to `impl From<PolicyRouteConfig> for RustRouteConfig` (after `tls_ca: route.tls_ca,`)
- `C:/Users/OMack/nono-ts/Cargo.toml` - `version = "0.4.0"` -> `"0.66.1"` (the `nono` path-dep's `version = "0.66"` caret pin was already correct, untouched)
- `C:/Users/OMack/nono-ts/package.json` - main `"version"` + all 5 `optionalDependencies` entries `"0.66.0"` -> `"0.66.1"`
- `C:/Users/OMack/nono-ts/package-lock.json` - regenerated via `npm install --package-lock-only`; root `name`/`version` and `optionalDependencies` block now `0.66.1` (was stale at `0.4.0`)
- `C:/Users/OMack/nono-ts/npm/darwin-arm64/package.json`, `npm/darwin-x64/package.json`, `npm/linux-arm64-gnu/package.json`, `npm/linux-x64-gnu/package.json` - each `"version": "0.66.0"` -> `"0.66.1"`

## Decisions Made
- The plan's exact insertion points and field-type confirmations (from RESEARCH.md and a
  fresh `crates/nono-proxy/src/config.rs:195` re-read) held: `Option<EndpointPolicyConfig>`,
  `None` is unambiguously valid.
- A second, unplanned missing-field build error surfaced only by actually invoking `maturin
  build` (not caught by the plan's static line-number research): `ProxyConfig::new`'s struct
  literal in `nono-py/src/proxy.rs` was missing `enable_h2` — a field added to
  `nono-proxy::ProxyConfig` in the same Phase 95 upstream absorb wave that introduced
  `endpoint_policy`, but never mirrored into this second `nono-py` construction site. Applied
  Rule 3 (blocking issue — "something prevents completing current task") rather than
  expanding scope: added `enable_h2: false,` (the same default the core `nono-proxy` crate's
  `ProxyConfig::default()` uses) as a single additional line in the same commit, since it is
  the same class of binding-drift as the plan's own `endpoint_policy` fix and was required for
  the task's own acceptance criterion (`maturin build` exits 0).
- Stayed within D-02 (publish operator-gated): both repos' validation commands
  (`maturin build`, `npm publish --dry-run`) build/validate only, no push or registry publish
  was invoked in either repo.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] nono-py ProxyConfig missing `enable_h2` field**
- **Found during:** Task 1, first `maturin build` attempt (immediately after the planned `endpoint_policy` edits)
- **Issue:** `cargo rustc` failed with `error[E0063]: missing field 'enable_h2' in initializer of 'nono_proxy::ProxyConfig'` at `src/proxy.rs:383` — a field added to the wrapped Rust `ProxyConfig` (same struct family as the plan's `RouteConfig.endpoint_policy` fix, both introduced in the Phase 95 upstream absorb) was never mirrored into `nono-py`'s `ProxyConfig::new` construction site
- **Fix:** Added `enable_h2: false,` to the `RustProxyConfig { ... }` struct literal in `ProxyConfig::new`
- **Files modified:** `C:/Users/OMack/nono-py/src/proxy.rs`
- **Commit:** `nono-py@84e8f18` (same commit as the planned Task 1 changes — discovered and fixed within the same task before the acceptance-criteria build check)

## Issues Encountered
None beyond the auto-fixed `enable_h2` build blocker above. Both `git status`/`git diff`
re-verifications at task start matched the RESEARCH.md snapshot exactly — no drift in either
external repo's working-tree state since the research was authored.

## User Setup Required

None — no external service configuration required. Publish to PyPI/npm remains a manual,
operator-gated step outside this plan (D-02); this plan only built/validated locally.

## Next Phase Readiness
- `nono-py` is committed at `0.66.1` with `maturin build` passing; `nono-ts` is committed at
  `0.66.1` with `npm publish --dry-run` passing. Both binding repos are now consistent with
  the `nono` workspace's own `0.66.1` leapfrog (Phase 100-01).
- No blockers carried forward from this plan. The remaining Phase 100 work
  (release-dry-run + release-readiness re-green, `RELEASE-RUNBOOK.md` update, host-gated UAT)
  can proceed against a fully version-consistent tree.

---
*Phase: 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker*
*Completed: 2026-07-02*

## Self-Check: PASSED

All modified files verified present on disk at `0.66.1` (nono-py `Cargo.toml`/`pyproject.toml`,
`src/proxy.rs`/`src/policy.rs` `endpoint_policy` fields; nono-ts `Cargo.toml`,
`package-lock.json`); both external-repo commit hashes verified present in their respective
`git log` (`nono-py@84e8f18`, `nono-ts@dd7d416`); this SUMMARY.md verified present on disk.
