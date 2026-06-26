# Release Runbook — v0.66.0 Operator Push Guide

**Status: PREPARE-ONLY**

This milestone has fully prepared the release workspace. All dry-runs have been validated,
the version has been leapfrogged, the pipeline has been audited, and the readiness gate is
green. The operator's single remaining action is to execute the push sequence documented
here. This document contains no command that this plan's execution runs live — every step
below is a future operator action.

---

## Pre-Push Checklist (PUBLIC-Repo Safety)

Before any `git push` or publish command, verify all three items:

- [ ] **Private paths check:** Repo is PUBLIC pending Microsoft minifilter-altitude
      approval. Verify `build_notes/` and `.gsd/` are NOT staged before any push.
      The release-readiness gate (Step 2 below) enforces this structurally, but a
      manual visual scan (`git status`) before Step 2 is also good practice.

- [ ] **Leapfrog version confirmed:** Crate version is 0.66.0, which is strictly
      greater than upstream highest 0.65.1. Confirmed by the readiness gate (Step 2).

- [ ] **Operator push is the sole remaining manual action:** All automation
      (dry-runs, gate, pipeline YAML) has been prepared by this milestone. Publishing
      to crates.io, PyPI, and npm — and the git push that triggers release.yml — are
      the ONLY remaining steps and are the operator's responsibility.

---

## Mandatory Pre-Push Gates (Must Both Be Green Before Any Push)

**STOP:** Do not proceed to the push sequence until both gates exit 0/PASS.

### Step 1 — Dry-Run Orchestrator

Run the full publish dry-run across all three registries:

```pwsh
pwsh -File scripts/release-dry-run.ps1
```

Expected outcome (current known state):

- `crates.nono` → PASS
- `crates.nono-proxy/nono-shell-broker/nono-cli` → `PRE_PUBLISH_REGISTRY_BLOCKED`
  (expected pre-publish state: nono 0.66.0 is not yet on crates.io)
- `pypi.maturin_build` → currently FAIL (see blocker below)
- `pypi.twine_check` → SKIP (twine absent on this host)
- `npm.dry_run` → PASS

**Pre-Release Blocker — PyPI:** `maturin build` exits 1 because two `RouteConfig`
struct initializers in nono-py are missing the `endpoint_policy` field added in the
phase 95 absorb. Fix required before the actual PyPI release:

```
C:\Users\OMack\nono-py\src\policy.rs:743   — add `endpoint_policy: None,`
C:\Users\OMack\nono-py\src\proxy.rs:206    — add `endpoint_policy: None,`
```

The dry-run script will exit 0 once all hard failures are resolved (BLOCKED statuses
are expected and do not block the exit code). Re-run after fixing nono-py and after
publishing nono 0.66.0 to verify downstream crates clear their BLOCKED status.

### Step 2 — Release Readiness Gate

Run the structured release-readiness gate (auto-discovered by verify-dark.ps1):

```pwsh
pwsh -File scripts/verify-dark.ps1 -Gate release-readiness
```

The gate asserts:

| Check | What It Verifies |
|-------|-----------------|
| version-family | nono, nono-cli, nono-proxy, nono-shell-broker, nono-fltmgr-client, nono-ffi all at 0.66.0 |
| no-stale-0.62.2 | No 0.62.2 version string in any tracked workspace Cargo.toml |
| leapfrog | 0.66.0 > upstream highest 0.65.1 |
| no-private-paths | No build_notes/ or .gsd/ path in staging area or tracked set |
| cargo-lock | Cargo.lock contains 0.66.0 workspace entries |

**Exit 0 = PASS (proceed to push sequence).**
Exit 2 = FAIL — investigate the failed check(s) in the JSON verdict before pushing.
Verdict file written to: `.nono-runtime/verdicts/release-readiness.json`

Both Step 1 (script exit 0) AND Step 2 (gate PASS) must be green. Do not push with
either failing.

---

## Push Sequence (Operator — Execute After Both Gates Are Green)

### Step 3 — Push Branch and Tag

```pwsh
# Push the current branch to origin
git push origin milestone/v2.13-carryforward-closeout

# Push the release tag — this triggers release.yml on GitHub Actions
git push origin v0.66.0
```

The `v0.66.0` tag push triggers the `release.yml` workflow (triggers on `push: tags: v*.*.*`).
That workflow builds and signs binaries on all five matrix legs (x86_64-linux-gnu,
x86_64-apple-darwin, aarch64-apple-darwin, aarch64-unknown-linux-gnu, x86_64-pc-windows-msvc),
packages two signed MSIs (machine and user scope) via `scripts/build-windows-msi.ps1`, runs
the admin-extract payload-signature gate, uploads all artifacts to the GitHub Release, and
runs the `publish-crates` and `update-homebrew-core` jobs.

Cross-target build gates: see `.planning/templates/cross-target-verify-checklist.md`
for the full cross-target clippy verification protocol (linux-gnu via `cross clippy`;
apple-darwin via direct-binary `cargo-zigbuild clippy`). Both gates are confirmed
LOCAL-RUNNABLE and both passed on this HEAD.

### Step 4 — crates.io Publish

The `publish-crates` job in release.yml runs this on tag push. If you need to publish
manually (e.g., the publish job failed after a successful build), publish in strict
dependency order with a 30-second gap between each to allow crates.io indexing:

```bash
# crates.io dependency order — nono must be published first
cargo publish -p nono
sleep 30

cargo publish -p nono-proxy
sleep 30

cargo publish -p nono-shell-broker
sleep 30

cargo publish -p nono-cli
```

**Publish set:** 4 crates (nono, nono-proxy, nono-shell-broker, nono-cli). The
`nono-fltmgr-client` and `nono-ffi` crates have `publish = false` and are
workspace-internal only. `nono-shell-broker` is included even though it is a Windows
dev-dependency of `nono-cli`, because it has no `publish = false` guard and can be
published independently; publishing it avoids future publish-order failures if its
dependency status changes.

Note: `cargo publish --dry-run -p nono-proxy/nono-shell-broker/nono-cli` will exit 101
(PRE_PUBLISH_REGISTRY_BLOCKED) until `nono 0.66.0` is on crates.io. Re-run the dry-run
after publishing nono to confirm all four crates package cleanly.

### Step 5 — PyPI (nono-py Wheel)

Fix the RouteConfig blocker in nono-py first (see Step 1 above). Then from the
nono-py repository directory:

```bash
cd ../nono-py
# Build the wheel with maturin
maturin publish
# Or, for a two-step approach:
maturin build --release
twine upload target/wheels/*.whl
```

The `nono-py` crate has `publish = false` in its `Cargo.toml` (Rust side) because it
is a PyO3 binding published to PyPI rather than crates.io. Publish only to PyPI via
maturin/twine.

### Step 6 — npm (nono-ts Package)

From the nono-ts repository directory:

```bash
cd ../nono-ts
# Verify the tarball manifest before publishing
npm publish --dry-run   # should show index.js + index.d.ts

# Publish to npm registry
npm publish
```

---

## What release.yml Does on Tag Push (Reference)

When `git push origin v0.66.0` triggers the workflow:

1. **Build matrix** (5 legs): Builds `nono-cli` for all platforms; additionally builds
   `nono-shell-broker` and `nono-wfp-service` on Windows (CRT-static).

2. **Sign Windows binaries (pre-package)**: Azure Trusted Signing (OIDC, keyless)
   signs `nono.exe`, `nono-shell-broker.exe`, `nono-wfp-service.exe` in-place
   before MSI harvesting (Phase 53 sign-before-harvest fix).

3. **Package**: tarballs for Unix; `scripts/build-windows-msi.ps1` (machine + user
   scope MSIs) for Windows. Both MSIs validated by `scripts/validate-windows-msi-contract.ps1`.

4. **Sign MSIs**: Azure Trusted Signing signs the MSI wrappers.

5. **Verify MSI payload signatures**: Admin-extract gate (`Verify MSI payload signatures`
   step) confirms embedded binaries are Valid Authenticode — fail-closed on NotSigned.

6. **Upload artifacts**: All tarballs, MSIs, and .deb packages uploaded to the
   GitHub Release created by the tag.

7. **publish-crates**: Publishes to crates.io in dependency order.

8. **update-homebrew-core**: Updates the Homebrew formula download URL
   (`OscarMackJr/nono` — corrected in Phase 97 Plan 02).

---

## Known Pre-Release Blockers

| # | Blocker | Location | Fix |
|---|---------|----------|-----|
| 1 | nono-py RouteConfig missing `endpoint_policy` | `../nono-py/src/policy.rs:743`, `../nono-py/src/proxy.rs:206` | Add `endpoint_policy: None,` to both initializers |
| 2 | twine absent on this dev host | host PATH | `pip install twine` or `uv add twine` in nono-py dev env |

Blocker 2 is moot if maturin build fails (blocker 1); fix blocker 1 first, then address
toolchain gaps.

---

## Reminder: This Milestone Is PREPARE-ONLY

None of the push commands above were executed by this milestone. The workspace has been:

- Version-leapfrogged to 0.66.0 (all six version-family crates)
- Pipeline-audited (sign-before-harvest order, admin-extract gate, 5 build legs)
- Dry-run validated (cargo PASS; PyPI blocked by nono-py RouteConfig bug; npm PASS)
- Cross-target clippy verified (linux-gnu + apple-darwin, both LOCAL-RUNNABLE, both PASS)
- Readiness gate written and PASS on the prepared tree

Tags remain LOCAL ONLY until the operator executes Step 3.
The repo stays PUBLIC (minifilter-altitude approval pending) — Step 2 structurally prevents
accidental `build_notes/` or `.gsd/` exposure.
