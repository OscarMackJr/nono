---
phase: 102-fork-owned-package-rename
plan: 02
subsystem: infra
tags: [make, github-actions, ci, cargo, package-rename, PUB-01]

# Dependency graph
requires:
  - phase: 102-fork-owned-package-rename plan 01
    provides: renamed [package] name on the 3-crate publish set (nono/nono-proxy/nono-cli -> nono-sandbox/nono-sandbox-proxy/nono-sandbox-cli)
provides:
  - Makefile build/test/run/watch/doc-lib targets invoking cargo with the renamed -p nono-sandbox / -p nono-sandbox-cli selectors
  - 4 permanent/build-facing CI workflows (ci.yml, image-build.yml, release.yml build steps, phase-37-linux-resl.yml) reconciled to the renamed selectors
  - In-file Phase 105 (PUB-02) deferral comment above release.yml's untouched cargo publish steps
  - Green cargo build --workspace --all-targets, make-build-equivalent (cargo build -p nono-sandbox + -p nono-sandbox-cli), and cargo fmt --all -- --check under the new package names
affects: [102-05-phase-gate, 104-release-cut, 105-live-publish]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "cargo -p selector rename: rename ONLY the package-name token after -p; leave --bin, --lib, --workspace, and unrenamed sibling package selectors (nono-ffi, nono-shell-broker) untouched"
    - "Deferred cargo publish selectors get an in-file comment naming the phase that will rewrite them, so the deferral is visible in the workflow file itself, not only in planning docs"

key-files:
  created: []
  modified:
    - Makefile
    - .github/workflows/ci.yml
    - .github/workflows/image-build.yml
    - .github/workflows/release.yml
    - .github/workflows/phase-37-linux-resl.yml

key-decisions:
  - "make binary itself is not installed/on PATH in this execution session (contrary to the environment note); verified make build's actual behavior instead by directly running the exact commands make build invokes (cargo build -p nono-sandbox && cargo build -p nono-sandbox-cli), both of which exited 0 -- this is a session tooling gap, not a rename defect, and is out of scope to fix per the deviation SCOPE BOUNDARY"

patterns-established: []

requirements-completed: [PUB-01]

# Metrics
duration: 9min
completed: 2026-07-03
---

# Phase 102 Plan 02: Makefile + CI Workflow Package-Selector Reconciliation Summary

**Renamed every build/test/run `-p nono`/`-p nono-cli` cargo selector in the Makefile and the 4 permanent CI workflows to `-p nono-sandbox`/`-p nono-sandbox-cli`, leaving `nono-ffi`/`nono-shell-broker` selectors and release.yml's 3 deferred `cargo publish` lines untouched (with a new Phase 105 deferral comment).**

## Performance

- **Duration:** 9 min
- **Started:** 2026-07-03T13:39:38Z
- **Completed:** 2026-07-03T13:48:36Z
- **Tasks:** 3 (Task 3 verification-only, no files modified)
- **Files modified:** 5

## Accomplishments
- Makefile: 4 bare `-p nono` selectors -> `-p nono-sandbox` (build-lib, build-release-lib, test-lib, doc-lib); 8 `-p nono-cli` selectors -> `-p nono-sandbox-cli` (build-cli, build-release-cli, build-arm64, test-cli, run, run-setup, run-dry, watch); `-p nono-ffi` (2 occurrences) and all `--workspace`-scoped targets left untouched
- 4 CI workflows reconciled: ci.yml (3 `-p nono-cli` build-step occurrences renamed), image-build.yml (2), release.yml (3 build-step occurrences renamed; 3 `cargo publish` lines deliberately untouched + new Phase 105 deferral comment added), phase-37-linux-resl.yml (1 `-p nono` + 4 `-p nono-cli` renamed; `--bin nono` args and the `-p nono-ffi` occurrence left untouched)
- phase-45-resl-native-host.yml and phase-46-uat-backlog.yml confirmed untouched (zero diff)
- `cargo build --workspace --all-targets` and `cargo fmt --all -- --check` both exit 0; make build's two constituent commands (`cargo build -p nono-sandbox`, `cargo build -p nono-sandbox-cli`) both exit 0

## Task Commits

Each task was committed atomically:

1. **Task 1: Reconcile Makefile package selectors** - `f25fc706` (fix)
2. **Task 2: Reconcile CI workflow build/test package selectors** - `89388037` (fix)
3. **Task 3: Build-green verification under the new package names** - verification-only, no files modified, no commit

**Plan metadata:** (pending) `docs(102-02): complete plan`

## Files Created/Modified
- `Makefile` - all `-p nono`/`-p nono-cli` build/test/run/watch/doc-lib selectors renamed to `-p nono-sandbox`/`-p nono-sandbox-cli`; `-p nono-ffi` and `--workspace` targets unchanged
- `.github/workflows/ci.yml` - 3 `-p nono-cli` build-step selectors (Windows release build, native + cross matrix builds) renamed to `-p nono-sandbox-cli`; `nono-shell-broker` and `nono-ffi` selectors unchanged
- `.github/workflows/image-build.yml` - 2 `-p nono-cli` selectors (native + cross matrix builds) renamed to `-p nono-sandbox-cli`
- `.github/workflows/release.yml` - 3 build-step `-p nono-cli` selectors renamed to `-p nono-sandbox-cli`; `nono-shell-broker` build step unchanged; 3 `cargo publish -p nono`/`-nono-proxy`/`-nono-cli` lines deliberately unchanged, with a new comment above the first publish step documenting the Phase 105 (PUB-02) deferral
- `.github/workflows/phase-37-linux-resl.yml` - 1 `-p nono` selector renamed to `-p nono-sandbox`; 4 `-p nono-cli` selectors renamed to `-p nono-sandbox-cli`; `-p nono-ffi` occurrence and `--bin nono` arguments unchanged

## Decisions Made
- Verified `make build`'s functional intent by directly invoking the exact two commands the target runs (`cargo build -p nono-sandbox`, `cargo build -p nono-sandbox-cli`) since the `make` binary itself was not resolvable on PATH in this execution session (checked via Bash `which`/`command -v`, `cmd.exe /c where make`, and a filesystem search of common install locations — all came back empty). This is a session/host tooling-availability gap unrelated to the rename edits themselves; the Makefile's targets are correctly renamed and both underlying cargo invocations succeed, which is the substantive thing Task 3's acceptance criteria are checking.
- Followed the plan's exact acceptance-criteria grep gates verbatim (including the `grep -vc 'cargo publish'` exclusion and the `($| )` anchored variants) rather than substituting equivalent-looking checks.

## Deviations from Plan

None - plan executed exactly as written. (The `make` binary unavailability noted above is an environment/tooling condition encountered during verification, not a deviation from the plan's instructions — the plan's own acceptance criteria for Task 3 name `cargo build --workspace --all-targets`, `make build`, and `cargo fmt --all -- --check`; the first and third ran directly and exited 0, and `make build`'s two constituent cargo invocations were verified directly to exit 0 in `make`'s absence.)

## Issues Encountered
- `make` executable not found on PATH in this Bash (Git Bash) session, nor via `cmd.exe /c where make`, despite the task's `<environment>` note stating make is on PATH. Resolved by running `make build`'s two constituent commands directly (`cargo build -p nono-sandbox` && `cargo build -p nono-sandbox-cli`), both of which exited 0. Flagging for awareness in case a future plan needs `make` specifically (e.g. `make ci`, `make check-upstream-drift`'s platform dispatch) rather than an equivalent direct cargo/rustfmt invocation.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- PUB-01's `make build` verification command is now satisfiable under the renamed package selectors (Makefile targets renamed; underlying cargo invocations proven green).
- The always-on required-check `phase-37-linux-resl.yml` workflow and the other 3 permanent/build-facing CI workflows now reference the correct renamed package names for their build/test steps, so future PRs/pushes to main will not fail with "package ID specification ... did not match any packages" once this branch merges.
- release.yml's 3 live `cargo publish` lines remain deliberately pointed at the pre-rename package names (`nono`, `nono-proxy`, `nono-cli`) until Phase 105 (PUB-02) rewrites them with dependency-ordered, index-visibility-polled publish logic; the in-file comment makes this deferral visible to any future reader of the workflow, not just to planning docs.
- No blockers for Plan 103 (nono-py rename) or Plan 104 (nono-ts rename), which operate on sibling repos independent of this repo's build automation.
- One tooling note for future plan authors: `make` itself is not resolvable in this Bash session — plans requiring literal `make <target>` execution (as opposed to the equivalent direct cargo/rustfmt commands) should verify tool availability first or provide a fallback verification path.

---
*Phase: 102-fork-owned-package-rename*
*Completed: 2026-07-03*

## Self-Check: PASSED

- FOUND: `Makefile`
- FOUND: `.github/workflows/ci.yml`
- FOUND: `.github/workflows/image-build.yml`
- FOUND: `.github/workflows/release.yml`
- FOUND: `.github/workflows/phase-37-linux-resl.yml`
- FOUND: `.planning/phases/102-fork-owned-package-rename/102-02-SUMMARY.md`
- FOUND: commit `f25fc706`
- FOUND: commit `89388037`
