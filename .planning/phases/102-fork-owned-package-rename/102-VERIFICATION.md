---
phase: 102-fork-owned-package-rename
verified: 2026-07-03T00:00:00Z
status: passed
score: 4/4
overrides_applied: 0
---

# Phase 102: Fork-Owned Package Rename Verification Report

**Phase Goal:** The published package identities are renamed to fork-owned `nono-sandbox` family names across all three registries, with the `nono` binary/lib/repo names left unchanged, so a later live publish (Phase 105) has an unblocked, owned target.
**Verified:** 2026-07-03
**Status:** passed
**Re-verification:** No — initial verification

This verification re-ran every build (`cargo build --workspace --all-targets`, the two `make build` constituent `cargo build -p` invocations, `cargo fmt --all -- --check`, `maturin build` in `../nono-py`, `npx napi build --platform --release` in `../nono-ts`), re-checked all 5 live registry endpoints, and read every touched manifest/workflow file directly against the plan's grep/node acceptance criteria — none of this relied on SUMMARY.md narrative alone.

## Goal Achievement

### Observable Truths (PUB-01 SC1-SC4, roadmap-sourced)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: crates.io `[package] name` renamed on the 3-crate publish set (`nono`→`nono-sandbox`, `nono-proxy`→`nono-sandbox-proxy`, `nono-cli`→`nono-sandbox-cli`); internal path-dep names/pins reconciled; `[[bin]] name = "nono"`/`"nono-agentd"` and `[lib] name = "nono"` unchanged | VERIFIED | Read `crates/nono/Cargo.toml` (`name = "nono-sandbox"`, explicit `[lib]\nname = "nono"`), `crates/nono-proxy/Cargo.toml` (`name = "nono-sandbox-proxy"`, `nono` dep carries `package = "nono-sandbox"`), `crates/nono-cli/Cargo.toml` (`name = "nono-sandbox-cli"`, both `[[bin]]` sections unchanged: `name = "nono"` / `name = "nono-agentd"`, `nono`/`nono-proxy` deps carry matching `package=` keys, dependency-table keys unchanged, `nono/system-keyring` feature-forwarding line intact). `crates/nono-shell-broker/Cargo.toml` and `bindings/c/Cargo.toml` (`nono-ffi`, `[lib] name = "nono_ffi"`) both carry `package = "nono-sandbox"` on their `nono` dep with their own names untouched. `git diff` on the `Cargo.lock` rename commit shows exactly 3 renamed `[[package]] name` hunks, zero checksum changes anywhere (`grep checksum` diff returns empty) — zero third-party drift. 95 files under `crates/nono-cli/src/` still contain `use nono::` imports, confirmed unaffected. |
| 2 | SC2: `nono-py` binding's PyPI project name renamed to `nono-sandbox`; `nono-ts` binding's npm package renamed to scoped `@oscarmackjr/nono-ts` — in both sibling repos | VERIFIED | `../nono-py/pyproject.toml`: `name = "nono-sandbox"` (line 6); `module-name = "nono_py._nono_py"` unchanged; `python/nono_py/` import dir present and untouched; own `Cargo.toml [package] name = "nono-py"` untouched, `nono`/`nono-proxy` deps carry `package = "nono-sandbox"`/`"nono-sandbox-proxy"`. `../nono-ts/package.json`: `name` resolves to `@oscarmackjr/nono-ts` via `node -e`; `napi.binaryName` still `"nono"`; all 4 existing `npm/*/package.json` subpackages (`darwin-arm64`, `darwin-x64`, `linux-arm64-gnu`, `linux-x64-gnu`) individually cross-checked and match `optionalDependencies` keys exactly (`@oscarmackjr/nono-ts-<platform>`); `../nono-ts/Cargo.toml [package] name = "nono-node"` untouched, `nono` dep carries `package = "nono-sandbox"`. |
| 3 | SC3: each new registry identity's availability confirmed live before committing | VERIFIED | Re-ran all 5 checks live in this verification session (not reused from SUMMARY): `crates.io/nono-sandbox` → 404, `nono-sandbox-proxy` → 404, `nono-sandbox-cli` → 404 (all with `User-Agent` header, avoiding the documented 403-without-UA trap), `pypi.org/pypi/nono-sandbox/json` → 404, `registry.npmjs.org/@oscarmackjr%2Fnono-ts` → 404. All 5 still unclaimed as of this verification pass. |
| 4 | SC4: workspace build (`make build`) and both binding builds (`maturin build`, `napi build`) green under the new names | VERIFIED | `cargo build --workspace --all-targets` → exit 0 (re-run live in this session). `make` itself is confirmed absent from this host's PATH (`which make`, `cmd.exe /c where make` both empty) — substituted its two constituent commands directly: `cargo build -p nono-sandbox` → exit 0, `cargo build -p nono-sandbox-cli` → exit 0. `cargo fmt --all -- --check` → exit 0. `maturin build` in `../nono-py` → exit 0, produced `nono_sandbox-0.66.1-cp312-cp312-win_amd64.whl`. `npx napi build --platform --release` in `../nono-ts` → exit 0, compiled `nono-node v0.66.1` against the renamed core crate. `cargo tree` in both sibling repos confirms resolution against `nono-sandbox`/`nono-sandbox-proxy` via the local relative path `C:\Users\OMack\Nono\crates\...` (not a stray registry hit). Makefile grep counts match plan exactly: 0 stale `-p nono`/`-p nono-cli`, 4 `-p nono-sandbox`, 8 `-p nono-sandbox-cli`, 2 unchanged `-p nono-ffi`. 4 permanent CI workflows (ci.yml, image-build.yml, release.yml build steps, phase-37-linux-resl.yml) all show the expected renamed selector counts; release.yml's 3 `cargo publish -p nono`/`-nono-proxy`/`-nono-cli` lines are confirmed still present/unrenamed with an in-file "Phase 105" deferral comment; phase-45/phase-46 workflows confirmed untouched (no diff since phase base). |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/Cargo.toml` | `[package] name = "nono-sandbox"` | VERIFIED | Confirmed; explicit `[lib] name = "nono"` pin also present (documented deviation, see below) |
| `crates/nono-proxy/Cargo.toml` | `[package] name = "nono-sandbox-proxy"`; `nono` dep carries `package = "nono-sandbox"` | VERIFIED | Confirmed |
| `crates/nono-cli/Cargo.toml` | `[package] name = "nono-sandbox-cli"`; `nono`/`nono-proxy` deps carry `package=`; `[[bin]]` unchanged | VERIFIED | Confirmed |
| `crates/nono-shell-broker/Cargo.toml` | `nono` dep carries `package = "nono-sandbox"`; own name unchanged | VERIFIED | Confirmed |
| `bindings/c/Cargo.toml` | `nono` dep carries `package = "nono-sandbox"`; own `[package]`/`[lib]` unchanged | VERIFIED | Confirmed |
| `Cargo.lock` | Regenerated, exactly 3 renamed `[[package]]` hunks, zero third-party drift | VERIFIED | Confirmed via `git diff` — zero checksum changes anywhere |
| `Makefile` | All `-p nono`/`-p nono-cli` selectors renamed | VERIFIED | Grep counts exactly match plan spec |
| `.github/workflows/{ci,image-build,release,phase-37-linux-resl}.yml` | Build selectors renamed; release.yml publish lines deliberately untouched + commented | VERIFIED | All grep counts match; Phase 105 deferral comment present |
| `../nono-py/Cargo.toml` + `pyproject.toml` | `package=` keys added; `[project] name = "nono-sandbox"` | VERIFIED | Confirmed; own crate name + module-name untouched |
| `../nono-ts/Cargo.toml` + `package.json` + 4 `npm/*/package.json` | `package=` key added; npm identity rescoped consistently | VERIFIED | Confirmed; cross-referenced individually against `optionalDependencies` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `crates/nono-proxy` / `crates/nono-cli` / `crates/nono-shell-broker` / `bindings/c` `[dependencies].nono` | `crates/nono [package].name` | `package = "nono-sandbox"` key | WIRED | `cargo build --workspace --all-targets` exits 0; `use nono::` imports unaffected (95 files in nono-cli alone) |
| `../nono-py [dependencies].nono`/`.nono-proxy` | this repo's renamed crates | `package=` key + relative path | WIRED | `cargo tree -p nono-py` resolves `nono-sandbox`/`nono-sandbox-proxy` via `C:\Users\OMack\Nono\crates\...`; `maturin build` exit 0 |
| `../nono-ts [dependencies].nono` | this repo's renamed `nono-sandbox` crate | `package=` key + relative path | WIRED | `cargo tree` resolves via local path; `napi build --platform --release` exit 0 |
| Makefile / CI workflow `-p` selectors | renamed `[package]` names | string selector match | WIRED | Live re-run confirms all selectors resolve; grep counts match plan exactly |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace compiles under new names | `cargo build --workspace --all-targets` | exit 0 | PASS |
| `make build` equivalent (make absent from PATH) | `cargo build -p nono-sandbox && cargo build -p nono-sandbox-cli` | both exit 0 | PASS |
| No formatting drift from manifest edits | `cargo fmt --all -- --check` | exit 0 | PASS |
| nono-py binding build | `maturin build` (in `../nono-py`) | exit 0, wheel `nono_sandbox-0.66.1-cp312-cp312-win_amd64.whl` produced | PASS |
| nono-ts binding build | `npx napi build --platform --release` (in `../nono-ts`) | exit 0, `nono-node v0.66.1` compiled | PASS |
| Both sibling deps resolve to renamed local crates, not registry | `cargo tree` in both repos, grep `nono-sandbox` | resolves via local relative path in both | PASS |
| 5 registry names still unclaimed (live) | `curl` x5 (crates.io x3 w/ UA, PyPI, npm) | all 404 | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| PUB-01 | 102-01 through 102-05 | Rename published package identities to fork-owned `nono-sandbox` family across crates.io/PyPI/npm, leaving bin/lib/repo names + internal imports unchanged, with availability confirmed live and all builds green | SATISFIED | All 4 roadmap Success Criteria independently re-verified live in this session (see Observable Truths table above) |

No orphaned requirements found for Phase 102 in REQUIREMENTS.md.

### Anti-Patterns Found

None. Scanned all touched manifests (`crates/*/Cargo.toml`, `bindings/c/Cargo.toml`, `Makefile`, the 4 touched CI workflow files, `../nono-py/{Cargo.toml,pyproject.toml}`, `../nono-ts/{Cargo.toml,package.json,npm/*/package.json}`) for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` — zero matches.

### Noted Deviations (documented, assessed as non-blocking)

1. **`[lib] name = "nono"` explicit pin added to `crates/nono/Cargo.toml` (102-01 Task 3 deviation).** Without this pin, Cargo's default lib-name derivation (`nono-sandbox` → `nono_sandbox`) would have broken `crates/nono/tests/manifest_types.rs`'s own `use nono::...` integration-test imports. Verified this pin has zero effect on the `package=` consumer mechanism (a consumer's extern crate name is controlled solely by its own dependency-table key) by confirming the full workspace build is green and all 95 `use nono::` call sites in `nono-cli` compile unaffected. Assessed: correct and necessary fix, not a compromise of PUB-01's "leave `[lib] name = "nono"` unchanged" requirement — the requirement is satisfied more explicitly than before (an implicit default is now an explicit, self-documented pin).
2. **`napi rename` tool abandoned in favor of hand-editing 6 JSON files (102-04 deviation).** Verified by reading the installed `@napi-rs/cli@3.6.0` source that the tool (a) requires a real TTY and cannot run non-interactively regardless of flags passed, (b) does not touch `npm/*/package.json` subpackages or `optionalDependencies` at all, and (c) would have corrupted `Cargo.toml [package].name` away from the required-unchanged `nono-node`. This verification independently cross-referenced all 4 hand-edited subpackage `name` fields against the main `package.json`'s `optionalDependencies` keys — all match exactly. Assessed: correct call, no gap.
3. **`make` binary absent from this host's PATH** (noted independently in 102-02 and 102-05 SUMMARYs, and reconfirmed in this verification session). All `make build`-equivalent behavior verified via direct `cargo build -p` invocations instead. This is a host/environment condition, not a rename defect — the Makefile's targets themselves are correctly renamed (confirmed by direct grep), so a host with `make` installed would produce the identical result.

### Human Verification Required

None. All 4 success criteria are objectively checkable via file content, live registry HTTP status, and command exit codes — no visual, UX, or subjective judgment call is needed for this phase's goal.

### Gaps Summary

No gaps found. All 4 of PUB-01's roadmap Success Criteria were independently re-verified against live repo and registry state in this session (not merely trusted from SUMMARY.md prose):

- SC1: 3-crate publish-set rename + internal reconciliation — confirmed via direct file reads + a live `cargo build --workspace --all-targets` (exit 0) + zero-third-party-drift Cargo.lock diff.
- SC2: both sibling binding identities renamed — confirmed via direct file reads + cross-referenced npm subpackage/optionalDependencies consistency.
- SC3: live registry availability — confirmed via 5 fresh `curl` checks in this session, all still 404.
- SC4: all 3 builds green — confirmed via live re-runs of `cargo build --workspace --all-targets`, the `make build` constituent commands, `cargo fmt --all -- --check`, `maturin build`, and `npx napi build --platform --release`, all exiting 0 in this verification session.

The phase's own documented deviations (the `[lib]` pin and the abandoned `napi rename` tool) were independently assessed against the codebase, not merely accepted from the SUMMARY narrative, and both hold up as correct, non-compromising fixes.

Phase 102 is genuinely, fully achieved. Ready for Phase 105 to build on this rename.

---
*Verified: 2026-07-03*
*Verifier: Claude (gsd-verifier)*
