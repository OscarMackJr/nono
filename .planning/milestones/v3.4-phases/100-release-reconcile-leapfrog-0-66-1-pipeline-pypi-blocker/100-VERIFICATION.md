---
phase: 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker
verified: 2026-07-02T16:20:21Z
status: passed
score: 8/8 must-haves verified
overrides_applied: 0
---

# Phase 100: Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker Verification Report

**Phase Goal:** The workspace is at crate version `0.66.1` (minimal collision-free bump above
upstream `0.66.0`), the prepare-only release pipeline is reconciled and gate-GREEN, the
nono-py PyPI blocker is closed, and a one-step operator push is the only remaining action.

**Verified:** 2026-07-02T16:20:21Z
**Status:** passed
**Re-verification:** No — initial verification

This phase spans three git repositories: `nono` (C:\Users\OMack\nono), `nono-py`
(C:\Users\OMack\nono-py), and `nono-ts` (C:\Users\OMack\nono-ts). All truths below were
verified by reading files, running `git log`, and **actually executing** the build/gate/dry-run
commands in this session — not by trusting SUMMARY.md prose.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | All 6 workspace-family crates (nono, nono-cli, nono-proxy, nono-shell-broker, nono-fltmgr-client, nono-ffi) report version 0.66.1; no lingering 0.66.0 in any workspace Cargo.toml | ✓ VERIFIED | Live `grep -n "^version"` on all 6 `Cargo.toml` files returned `0.66.1`; `grep -rn "0.66.0" crates/*/Cargo.toml bindings/c/Cargo.toml tools/sign-fixture/Cargo.toml Cargo.toml` returned zero matches; `tools/sign-fixture` correctly untouched at `0.1.0` |
| 2 | Cargo.lock regenerated with exactly the 6 workspace-member version bumps, zero third-party drift; `make build` / `cargo build --workspace --all-targets` passes clean | ✓ VERIFIED | `git diff 6cdfe149 c26e53bb -- Cargo.lock \| grep -E "^[-+]version"` shows exactly 6 `0.66.0`→`0.66.1` transitions, no other version line changed; live `cargo build --workspace --all-targets` in this session exited 0 (only a pre-existing unrelated warning about `nono-shell-broker` missing a lib target, not a version-bump artifact) |
| 3 | nono-py: Cargo.toml + pyproject.toml at 0.66.1; `endpoint_policy: None` present at both RouteConfig construction sites (src/proxy.rs:224, src/policy.rs:758); `enable_h2: false` added to ProxyConfig::new; `maturin build` succeeds; exactly one new DCO-signed commit, no 0.66.0 commit ever landed | ✓ VERIFIED | Live grep confirmed both fields at the cited sites; live `maturin build --release` in this session produced `nono_py-0.66.1-cp312-cp312-win_amd64.whl` (exit 0); `git log --all --oneline \| grep 0.66.0` in nono-py returned nothing; HEAD commit `84e8f18` is DCO-signed (`Signed-off-by: Oscar Mack Jr`) |
| 4 | nono-ts: Cargo.toml, package.json (+5 optionalDependencies), package-lock.json, and all 4 tracked npm/*/package.json files at 0.66.1; 3 scratch files remain untracked; one new DCO-signed commit, no 0.66.0 commit | ✓ VERIFIED | Live grep confirmed all manifests at 0.66.1; `git status --short` shows `decode-test.js`, `test-broker.js`, `test-confined.js` still untracked (`??`); live `npm publish --dry-run` in this session exited 0, tarball `nono-ts-0.66.1.tgz`; `git log --all --oneline \| grep 0.66.0` in nono-ts returned nothing; HEAD commit `dd7d416` is DCO-signed |
| 5 | proj/ADR-100-ci-pipeline-reconcile.md exists and records the ADAPT-not-ADOPT disposition for #1245/#1251; release.yml/ci.yml changes preserve sign-before-harvest MSI order and the release-readiness verify-dark gate | ✓ VERIFIED | File exists (8641 bytes), reads as a substantive ADR with Status/Context/Options-Considered/Decision sections explicitly rejecting verbatim adoption and explaining the `workflow_dispatch` trigger rewrite; `release.yml`'s `publish-crates` job (3× `cargo search` idempotency guards, confirmed live) is downstream of signing and does not touch the MSI build/sign job; `ci.yml`'s new `cross-compile:` job is gated on `github.event_name == 'workflow_dispatch'` (confirmed live, not the dead upstream PR-title trigger); both YAML files parse clean (`python -c "import yaml..."` exit 0 in this session) |
| 6 | release-readiness gate no longer hardcodes a stale 0.66.0 literal; both mandatory pre-push gates (release-readiness, release-dry-run.ps1) re-run GREEN at 0.66.1; RELEASE-RUNBOOK.md references 0.66.1 | ✓ VERIFIED | Live-executed `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` in this session → exit 0, verdict PASS, `cargo_lock` detail reports "0.66.1 found (6 occurrence(s))"; assertion (e) reads `[regex]::Escape($targetVersion)` not a literal (confirmed at `scripts/gates/release-readiness.ps1:215`); live-executed `pwsh -File scripts/release-dry-run.ps1` in this session → exit 0, `crates.nono` PASS, `crates.nono-proxy`/`crates.nono-cli` BLOCKED (expected pre-publish state), `pypi.maturin_build` PASS, `pypi.twine_check` SKIP (toolchain absent, correctly detected via exit-code not text-match), `npm.dry_run` PASS; `RELEASE-RUNBOOK.md` has 13 occurrences of `0.66.1` and exactly 1 occurrence of `0.66.0` (the intentional leapfrog-comparison row) |
| 7 | Both host-gated clean-host todos remain under .planning/todos/pending/ with a dated Phase 100 status note; neither marked resolved | ✓ VERIFIED | Both files present at `.planning/todos/pending/20260611-msi-vcredist-prereq.md` and `.planning/todos/pending/20260611-poc-cert-broker-clean-host.md`; each has a "## Phase 100 status (2026-07-01)" section recording verbatim gate verdicts / external-block status; both explicitly state "This item remains OPEN" |
| 8 | Requirements RLS-10/11/12/13 marked complete in REQUIREMENTS.md and traced to Phase 100 | ✓ VERIFIED | `.planning/REQUIREMENTS.md` lines 51-54 show `- [x]` for all four; traceability table (lines 89-92) maps all four to "Phase 100 / Complete" |

**Score:** 8/8 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/Cargo.toml` (+5 sibling Cargo.toml + bindings/c) | version = 0.66.1 | ✓ VERIFIED | Live-read, all 6 confirmed |
| `Cargo.lock` | 6 workspace-member entries at 0.66.1, no 3rd-party drift | ✓ VERIFIED | Live diff + live build |
| `.github/workflows/release.yml` | idempotent publish-crates (3× cargo search guards) | ✓ VERIFIED | Live grep confirmed 3 occurrences |
| `.github/workflows/ci.yml` | operator-invocable cross-compile job on workflow_dispatch | ✓ VERIFIED | Live grep confirmed trigger + YAML parse-clean |
| `proj/ADR-100-ci-pipeline-reconcile.md` | ADAPT-not-ADOPT disposition documented | ✓ VERIFIED | File exists, substantive content read |
| `C:/Users/OMack/nono-py/{Cargo.toml,pyproject.toml,src/proxy.rs,src/policy.rs}` | 0.66.1 + endpoint_policy/enable_h2 fields | ✓ VERIFIED | Live grep + live maturin build |
| `C:/Users/OMack/nono-ts/{Cargo.toml,package.json,package-lock.json,npm/*/package.json}` | 0.66.1 across all tracked manifests | ✓ VERIFIED | Live grep + live npm publish --dry-run |
| `scripts/gates/release-readiness.ps1` | $targetVersion=0.66.1 referenced (not hardcoded) throughout | ✓ VERIFIED | Live-read + live-executed, PASS |
| `scripts/release-dry-run.ps1` | re-run GREEN at 0.66.1, twine exit-code detection | ✓ VERIFIED | Live-executed, exit 0 |
| `.planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md` | updated for 0.66.1 tag | ✓ VERIFIED | Live grep: 13×0.66.1, 1×0.66.0 (intentional) |
| `.planning/todos/pending/20260611-msi-vcredist-prereq.md` + `20260611-poc-cert-broker-clean-host.md` | Phase 100 status appended, not resolved | ✓ VERIFIED | Live-read, both OPEN |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `crates/nono-cli/Cargo.toml` | `crates/nono/Cargo.toml` | internal path-dep version pin `nono = { version = "0.66.1", ... }` | WIRED | Live grep confirmed at `crates/nono-cli/Cargo.toml:47` |
| `crates/nono-cli/Cargo.toml` | `crates/nono-shell-broker/Cargo.toml` | Windows-only dev-dep pin | WIRED | Live grep confirmed at `crates/nono-cli/Cargo.toml:180` |
| `scripts/gates/release-readiness.ps1` | `.nono-runtime/verdicts/release-readiness.json` | verify-dark.ps1 persisted verdict | WIRED | Live-executed gate; verdict JSON confirmed present with matching PASS content and timestamp |
| `RELEASE-RUNBOOK.md` | `scripts/release-dry-run.ps1` | Step 1 pre-push gate reference | WIRED | Runbook references the script; script live-executed successfully |
| `nono-py::RouteConfig` construction sites | `nono_proxy::RouteConfig.endpoint_policy` field | struct-literal field addition | WIRED | `maturin build` compiled successfully against the wrapped `nono-proxy` crate, proving the field type/shape match is real, not just textually present |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace builds clean at 0.66.1 | `cargo build --workspace --all-targets` (nono repo) | exit 0 | ✓ PASS |
| cargo fmt clean | `cargo fmt --all -- --check` (nono repo) | exit 0 | ✓ PASS |
| release-readiness gate GREEN | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` | exit 0, verdict PASS | ✓ PASS |
| release-dry-run GREEN | `pwsh -File scripts/release-dry-run.ps1` | exit 0, 0 hard failures | ✓ PASS |
| nono-py wheel builds | `maturin build --release` (nono-py repo) | exit 0, wheel produced | ✓ PASS |
| nono-ts publish dry-run | `npm publish --dry-run` (nono-ts repo) | exit 0, tarball built | ✓ PASS |
| CI YAML parses | `python -c "import yaml; yaml.safe_load(...)"` on release.yml + ci.yml | exit 0 | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| RLS-10 | 100-01, 100-03 | Version-bump all workspace crates + both binding repos to 0.66.1, path-dep pins consistent, Cargo.lock regenerated, build clean | ✓ SATISFIED | Live build + live grep across both repos |
| RLS-11 | 100-02 | Reconcile upstream #1245/#1251 CI changes without breaking release-readiness gate or signed-MSI order | ✓ SATISFIED | ADR-100 + live YAML verification; gate re-run GREEN |
| RLS-12 | 100-03 | Close nono-py RouteConfig PyPI blocker (endpoint_policy field) | ✓ SATISFIED | Live maturin build success; fields present at cited lines |
| RLS-13 | 100-04 | Release-dry-run + release-readiness re-run GREEN at 0.66.1; RELEASE-RUNBOOK.md updated | ✓ SATISFIED | Both live-executed GREEN in this session; runbook content verified |

No orphaned requirements found — REQUIREMENTS.md's Phase 100 mapping (RLS-10..13) matches exactly the 4 requirements declared across the 5 plans' frontmatter.

### Anti-Patterns Found

None. Scanned all phase-touched files in both the `nono` repo (6× Cargo.toml, Cargo.lock, release.yml, ci.yml, release-readiness.ps1, release-dry-run.ps1, ADR-100, RELEASE-RUNBOOK.md) and the binding repos (nono-py: Cargo.toml, pyproject.toml, src/proxy.rs, src/policy.rs; nono-ts: Cargo.toml, package.json, package-lock.json) for `TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER` — zero real matches (the only regex hits were npm sha512 integrity hashes coincidentally containing the substring `XXX`, not debt markers).

### Human Verification Required

None. This phase is pure release-engineering (version strings, CI YAML, gate scripts, binding-repo manifests/struct-literal fields) — every truth was verifiable by direct file inspection and live command execution. The two host-gated clean-host UAT items (`msi-vcredist-prereq`, `poc-cert-broker-clean-host`) were explicitly folded as non-blocking, gated/deferred items per CONTEXT.md D-09 — Plan 100-05 correctly recorded their status without claiming resolution, and the phase's own success criteria never required them to close.

### Gaps Summary

No gaps found. All 8 derived observable truths (mapped from ROADMAP's 4 success criteria plus
the phase's cross-repo scope) were independently re-verified in this session via direct file
reads, `git log`/`git diff` inspection across all three repositories, and **live execution** of
`cargo build`, `cargo fmt --check`, `maturin build`, `npm publish --dry-run`,
`scripts/verify-dark.ps1 -Gate release-readiness`, and `scripts/release-dry-run.ps1` — all
matched or exceeded the SUMMARY.md claims. No stub code, no hardcoded-empty data, no unwired
artifacts, no debt markers were found in any phase-touched file across the three repositories.

One cosmetic (non-blocking) documentation drift was observed and is noted here for completeness,
not as a gap: `.planning/ROADMAP.md`'s top-level Phase 100 summary checkbox still reads
`- [ ] Phase 100 ... — 0/5 plans` (line 30) even though the detailed per-plan checklist further
down the same file (lines 116-120) correctly shows all 5 plans as `[x]` complete. This matches a
previously logged project-memory pattern ("SDK phase.complete leaves ROADMAP `## Phases`
checklist box unchecked") — a known SDK/tooling quirk, not a Phase 100 execution defect. It does
not affect any of the phase's observable truths and can be hand-corrected during milestone
close/archive.

---

_Verified: 2026-07-02T16:20:21Z_
_Verifier: Claude (gsd-verifier)_
