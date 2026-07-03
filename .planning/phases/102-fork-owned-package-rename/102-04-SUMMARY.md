---
phase: 102-fork-owned-package-rename
plan: 04
subsystem: infra
tags: [cargo, napi, npm, package-rename, sibling-repo, PUB-01]

# Dependency graph
requires:
  - phase: 102-fork-owned-package-rename plan 01
    provides: renamed [package] name on the 3-crate publish set (nono/nono-proxy/nono-cli -> nono-sandbox/nono-sandbox-proxy/nono-sandbox-cli) in this workspace
provides:
  - "../nono-ts/Cargo.toml nono path-dependency carries package = \"nono-sandbox\" (dependency table key unchanged)"
  - "../nono-ts/package.json name renamed to @oscarmackjr/nono-ts; napi.binaryName left \"nono\"; all 4 existing platform subpackages + optionalDependencies consistently rescoped"
  - "Green napi build --platform --release in ../nono-ts under the new names; cargo tree confirms the resolved dependency graph points at the renamed core crate via the correct relative path"
affects: [102-05-phase-gate, 105-live-publish]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Sibling-repo package= propagation: a repo outside this Cargo workspace that path-deps into it by relative path must have its OWN Cargo.toml dependency entries patched with package= keys after an upstream rename -- this repo's own cargo build --workspace never surfaces that breakage (per 102-RESEARCH.md Pitfall 2)"
    - "napi-rs scoped subpackage convention: @scope/pkgname-platform-arch mirrors the unscoped pkgname-platform-arch pattern already used for nono-ts-darwin-x64 etc."

key-files:
  created: []
  modified:
    - ../nono-ts/Cargo.toml
    - ../nono-ts/package.json
    - ../nono-ts/npm/darwin-arm64/package.json
    - ../nono-ts/npm/darwin-x64/package.json
    - ../nono-ts/npm/linux-arm64-gnu/package.json
    - ../nono-ts/npm/linux-x64-gnu/package.json
    - ../nono-ts/Cargo.lock

key-decisions:
  - "Rule 1/3 auto-fix: the installed @napi-rs/cli 3.6.0 `rename` subcommand's ACTUAL implementation diverges from 102-RESEARCH.md's documentation-derived description. It does not touch npm/*/package.json subpackages or optionalDependencies at all (confirmed by reading node_modules/@napi-rs/cli/src/api/rename.ts), it maps the package.json `name` field to a DIFFERENT flag (`--name`, not `--package-name`), it is fully interactive even with all documented flags supplied (both @inquirer/prompts prompts route through a real TTY and cannot be satisfied by piped stdin), and -- most importantly -- it OVERWRITES the crate's own `Cargo.toml [package].name` with a sanitized form of `--binary-name`, which would have silently renamed nono-ts's own crate identity from `nono-node` away from its required unchanged value. Given the tool provides zero benefit (doesn't rename the subpackages/optionalDependencies) and carries an active hazard (corrupts [package].name), all 6 JSON files were hand-edited directly instead of invoking `napi rename`. Every cross-reference (optionalDependencies keys/versions vs. each npm/*/package.json name field) was verified individually to mitigate exactly the risk the plan's anti-pattern warned about for manual editing."

patterns-established: []

requirements-completed: [PUB-01]

# Metrics
duration: 12min
completed: 2026-07-03
---

# Phase 102 Plan 04: nono-ts Sibling-Repo Package Rename Summary

**Patched `../nono-ts`'s Cargo.toml path-dependency with `package = "nono-sandbox"` and hand-rescoped npm identity to `@oscarmackjr/nono-ts` (main package.json + 4 platform subpackages + optionalDependencies) after discovering the official `napi rename` tool's installed version neither performs the subpackage rewrite nor can run non-interactively, then proved `napi build --platform --release` green and committed in the nono-ts repo.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-07-03 (session start)
- **Completed:** 2026-07-03
- **Tasks:** 2
- **Files modified:** 7 (all in `../nono-ts`: Cargo.toml, package.json, 4x npm/*/package.json, Cargo.lock)

## Accomplishments
- `../nono-ts/Cargo.toml`: added `package = "nono-sandbox"` to the `nono` path-dependency entry; dependency key left `nono`; `[package] name = "nono-node"` untouched (not in the 3-crate publish set, never published to crates.io)
- `../nono-ts/package.json`: `name` renamed from `nono-ts` to `@oscarmackjr/nono-ts`; `napi.binaryName` left as `nono` (compiled `.node` artifact prefix unchanged); `optionalDependencies` rescoped to the 4 renamed platform subpackages plus the pre-existing (never-generated) `win32-x64-msvc` entry
- 4 existing `npm/*/package.json` files (`darwin-arm64`, `darwin-x64`, `linux-arm64-gnu`, `linux-x64-gnu`) each had their `name` field rescoped to `@oscarmackjr/nono-ts-<platform>-<arch>`, matching the main package's new scope
- `npx napi build --platform --release` run from `../nono-ts`: compiled `nono-sandbox v0.66.1` from the renamed core-crate path, produced the release `.node` artifact, exited 0
- `cargo tree` in `../nono-ts`, grepped for `nono-sandbox`, returned `nono-sandbox v0.66.1 (C:\Users\OMack\Nono\crates\nono)` — confirms the resolved dependency graph points at the renamed crate via the expected relative path, not a stray registry hit (T-102-02 mitigated)
- `Cargo.lock` regenerated by the build; diff confirms `nono-node`'s own `[[package]] name` is unchanged and now depends on `nono-sandbox v0.66.1` (previously a stale `nono v0.62.2`/`nono-node v0.4.0` pin — the lockfile had not been regenerated in some time, unrelated to this rename)
- All 7 files committed together in `../nono-ts` with a DCO sign-off, as a commit SEPARATE from this repo's and nono-py's commit history

## Task Commits

Task execution in `../nono-ts` (a separate git repository from this one):

1. **Task 1 + Task 2 combined: Patch Cargo.toml/package.json identity + prove napi build green + DCO-signed commit** - `c2f5aaa` (feat), committed in `../nono-ts` on its `44-broker-ffi-lockstep` branch

**This repo's plan-metadata commit:** (this SUMMARY.md + STATE.md + ROADMAP.md, committed separately below)

## Files Created/Modified
- `../nono-ts/Cargo.toml` - `nono` dependency: `+ package = "nono-sandbox"`; key and `[package] name = "nono-node"` unchanged
- `../nono-ts/package.json` - `name`: `nono-ts` -> `@oscarmackjr/nono-ts`; `optionalDependencies` keys rescoped to `@oscarmackjr/nono-ts-*`; `napi.binaryName` untouched
- `../nono-ts/npm/darwin-arm64/package.json` - `name`: `nono-ts-darwin-arm64` -> `@oscarmackjr/nono-ts-darwin-arm64`
- `../nono-ts/npm/darwin-x64/package.json` - `name`: `nono-ts-darwin-x64` -> `@oscarmackjr/nono-ts-darwin-x64`
- `../nono-ts/npm/linux-arm64-gnu/package.json` - `name`: `nono-ts-linux-arm64-gnu` -> `@oscarmackjr/nono-ts-linux-arm64-gnu`
- `../nono-ts/npm/linux-x64-gnu/package.json` - `name`: `nono-ts-linux-x64-gnu` -> `@oscarmackjr/nono-ts-linux-x64-gnu`
- `../nono-ts/Cargo.lock` - regenerated via `napi build --platform --release`

## Decisions Made
- Abandoned the plan's directed `napi rename --package-name @oscarmackjr/nono-ts` invocation after empirically confirming (via both live execution and reading the installed tool's source) that it cannot achieve the required end state safely or non-interactively — see the Deviation below for the full reasoning. Hand-editing was the only path that satisfied every `must_haves.truths`/`acceptance_criteria` line without the corruption risk.
- Combined Task 1 (manifest/JSON edits) and Task 2 (build-proof + commit) into a single commit in `../nono-ts`, matching the pattern already established in Plan 102-03's nono-py commit, since the plan's Task 2 acceptance criteria require Cargo.toml, package.json, all npm/*/package.json, and Cargo.lock all present together.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1/3 - Bug in tooling assumption / Blocking] Official `napi rename` tool does not do what the plan/research assumed, and is actively hazardous**
- **Found during:** Task 1, npm rename step
- **Issue:** Running `npx napi rename --package-name @oscarmackjr/nono-ts` (per the plan's exact instruction) produced an interactive `@inquirer/prompts` session that could not be satisfied via piped stdin (`Exit Prompt Error: User force closed the prompt`) even though the documented flag was supplied. Reading the installed `@napi-rs/cli@3.6.0` source (`src/commands/rename.ts`, `src/api/rename.ts`, `src/def/rename.ts`) revealed three material divergences from 102-RESEARCH.md's documentation-derived description: (1) the `RenameCommand.execute()` gate checks `options.name` (populated only by `--name`/`-n`), not `options.packageName` (populated by `--package-name`) — so the flag actually passed by the plan never satisfies the prompt-skip condition; (2) `renameProject()` contains NO code path that touches `npm/*/package.json` subpackages or the main `package.json`'s `optionalDependencies` block — contrary to the plan's/research's core assumption that "this single invocation... rewrite[s] all 4 platform subpackages... and optionalDependencies"; (3) if `--binary-name` is supplied (required to skip the second interactive prompt), the tool OVERWRITES `Cargo.toml [package].name` with a sanitized form of the binary name (`cargoToml.package.name = sanitizedName`), which would have silently renamed nono-ts's own crate identity away from the plan's explicitly required-unchanged `nono-node`.
- **Fix:** Did not run `napi rename`. Hand-edited all 6 JSON manifests directly (`package.json` name + optionalDependencies; the 4 `npm/*/package.json` name fields), leaving `Cargo.toml [package].name` and `napi.binaryName` completely untouched by any tool invocation. Verified every optionalDependencies key against its corresponding subpackage's renamed `name` field individually, mitigating exactly the cross-reference-miss risk the plan's own anti-pattern section warned about for manual editing.
- **Files modified:** `../nono-ts/package.json`, `../nono-ts/npm/darwin-arm64/package.json`, `../nono-ts/npm/darwin-x64/package.json`, `../nono-ts/npm/linux-arm64-gnu/package.json`, `../nono-ts/npm/linux-x64-gnu/package.json`
- **Verification:** All 6 acceptance-criteria checks from the plan (grep/node one-liners) passed as specified; `napi build --platform --release` exits 0 and `cargo tree | grep nono-sandbox` resolves correctly, proving the hand-edited manifests are functionally equivalent to what the (unavailable) correct tool behavior would have produced.
- **Commit:** `c2f5aaa` (in `../nono-ts`)

---

**Total deviations:** 1 auto-fixed (tooling-assumption bug + blocking issue, resolved together)
**Impact on plan:** Necessary — the plan's directed tool invocation was not executable as written (interactive-only) and, if forced through with the required flags, would have corrupted a value the plan explicitly required to stay unchanged. The hand-edit achieves the identical, plan-specified end state (verified against every acceptance criterion) with no scope creep beyond the 6 files the plan already named for this task.

## Issues Encountered
- `npx napi rename` requires a real TTY for its two `@inquirer/prompts` prompts regardless of which documented flags are passed on this installed version; piped/heredoc stdin cannot satisfy it. Not pursued further once the tool's actual rewrite scope (see deviation above) was found to exclude the npm subpackage/optionalDependencies work anyway, making the tool unnecessary for this task.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `../nono-ts`'s half of PUB-01 SC2 (npm package name renamed to the scoped `@oscarmackjr/nono-ts`) and PUB-01 SC4 (napi build green) is complete.
- This was the last plan of Wave 2. Plan 102-05 (phase gate) should re-verify `napi build --platform --release` fresh in `../nono-ts` alongside `maturin build` in `../nono-py` and `make build` in this repo, per 102-RESEARCH.md's "three separate, sequential verification steps" guidance (Pitfall 2).
- Flag for future reference (any later phase touching `../nono-ts`'s napi tooling): the installed `@napi-rs/cli` version's `rename` subcommand should not be relied on for automated/non-interactive re-scoping — its actual behavior does not match its own upstream documentation on this point, and it has a hazardous side effect on the crate's own `[package].name`.
- No blockers. Note: `../nono-ts`'s current git branch is `44-broker-ffi-lockstep` (pre-existing, not created by this plan) — the rename commit landed on that branch, consistent with the prior commit history already present there. Pre-existing untracked files (`decode-test.js`, `test-broker.js`, `test-confined.js`) in `../nono-ts` were left untouched — out of scope for this plan.

---
*Phase: 102-fork-owned-package-rename*
*Completed: 2026-07-03*
