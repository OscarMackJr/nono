# Phase 100: Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker - Research

**Researched:** 2026-07-01
**Domain:** Release engineering (Cargo workspace version bump, GitHub Actions CI/CD reconcile, PyO3/maturin PyPI packaging, cross-repo binding version sync)
**Confidence:** HIGH (all four requirement areas verified directly against local repo state, upstream commit objects, and the two separate binding-repo working trees — no unverifiable claims)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Cross-repo binding scope (RLS-10 + RLS-12)**
- D-01: Both bindings fully in-phase. Execute the `0.66.1` version bump in **both** separate binding repos (`C:/Users/OMack/nono-py`, `C:/Users/OMack/nono-ts`) AND the `nono-py` `endpoint_policy` fix, each as **separate DCO-signed commits in their own git repos** this phase. These are distinct repositories from the `nono` workspace — plan for cross-repo commits, not a single-repo diff.
- D-02: Binding **publish** (PyPI wheel, npm package) stays **operator-gated** — same prepare-only posture as the crate registry publish. Phase 100 makes them build/validate clean and version-consistent; it does not push.

**Version-bump membership (RLS-10)**
- D-03: All members lockstep to `0.66.1`. RLS-10's enumeration ("5 crates") is **stale** — the workspace actually has 7 members. Bump **every** member currently carrying `0.66.0`: `crates/nono`, `crates/nono-cli`, `crates/nono-proxy`, `crates/nono-shell-broker`, `bindings/c` (nono-ffi), **`crates/nono-fltmgr-client`** (omitted from RLS-10), plus `tools/sign-fixture` if it carries a version. No intentional version skew.
- D-04: Keep **all internal path-dep `version` pins consistent** across every `Cargo.toml` (each member hard-codes `version = "..."` — there is **no** `version.workspace = true` inheritance, so every file is edited individually). Regenerate `Cargo.lock`; `make build` must pass clean. Publish set remains 3 crates (nono → nono-proxy → nono-cli) per the v3.3 WR-02 decision — bumping non-published members is for path-dep consistency, not publish.

**CI reconciliation (RLS-11)**
- D-05: Adapt + short ADR. Cherry-pick/adapt only the applicable hunks of #1245 and #1251; **preserve** the fork's prepare-only posture, the signed-MSI build order (sign-before-harvest), and the `release-readiness` verify-dark gate. Record the adopt-vs-adapt disposition in a **brief ADR** (mirrors the #1225 / ADR-98 pattern) so the divergence is auditable.
- D-06: The `release-readiness` verify-dark gate and the signed-MSI order are **invariants** — any CI change that would break them is rejected, not worked around.

**PyPI blocker depth (RLS-12)**
- D-07: Minimal `endpoint_policy: None` stub + tracked follow-up. Add `endpoint_policy: None` at the two `nono-py` `RouteConfig` initializers so the wheel builds and `twine`/maturin validation passes (RLS-12 literal). This unblocks PyPI now; Python users cannot yet *set* endpoint policy — document that limitation.
- D-08: The "fully thread `endpoint_policy` through `nono-py` `RouteConfig`" work is reserved as a **named future phase** (see Deferred Ideas) — not attempted in Phase 100.

**Release posture (RLS-13)**
- D-09: Phase 100 = **PREPARE ONLY**. `release-dry-run.ps1` + `release-readiness` re-run GREEN at `0.66.1`; `RELEASE-RUNBOOK.md` updated for the `0.66.1` tag with the PUBLIC-repo pre-push checklist (no `build_notes/`/`.gsd/` staged; `0.66.1` > upstream `0.66.0` confirmed). **Repo stays PUBLIC** (go-private cancelled 2026-07-01 — push no longer altitude-gated, but still operator-gated). The actual tag push + registry publish are the single remaining operator step, outside this phase.

### Claude's Discretion
- Wave/plan breakdown is the planner's call. A natural shape: (1) workspace version bump + path-dep pins + Cargo.lock + build; (2) CI reconcile + ADR; (3) nono-py PyPI stub + nono-py/nono-ts version bumps (cross-repo); (4) release-dry-run + release-readiness re-green + runbook update; (5) host-gated clean-host UAT (folded, may SKIP_HOST_UNAVAILABLE).
- ADR filename/location is the planner's call (convention: `proj/ADR-NN-*.md`).
- Whether `RELEASE-RUNBOOK.md` is updated in place at its archived v3.3 path or brought forward is the planner's call (note the current path in canonical refs).
- Version is **exactly `0.66.1`** — not `0.67.x`. Minimal collision-free bump above upstream's own `0.66.0` (Phase 98 ledger anchor). Do not skip ahead.
- Preserve fork identity: any org-ref touched during the bump keeps `OscarMackJr/nono` (Cluster E precedent). Flag — but don't silently rewrite — the workspace `repository = always-further/nono` field; whether to correct it to the fork is a planner decision within RLS-10 reconcile.
- The ADR (D-05) should explicitly state which #1245/#1251 hunks were taken vs dropped and why (prepare-only + signed-MSI-order preservation), like ADR-98 named its deviations.

### Deferred Ideas (OUT OF SCOPE)
- **Named future phase — nono-py `endpoint_policy` binding completeness.** Fully thread `endpoint_policy` through `nono-py`'s `RouteConfig` so Python callers can set endpoint routing end-to-end (beyond the D-07 `None` stub). Reserve as an explicit future phase; not Phase 100.
- **Actual tag push + registry publish (PyPI/npm/crates.io).** The single remaining operator-gated step after Phase 100 makes everything push-ready. Outside this phase by design (D-09).
- **Correcting `[workspace.package] repository` to the fork** — if the planner decides it's out of RLS-10 scope, capture as a small follow-up rather than expanding Phase 100.

### Folded Todos (in-phase, host-gated)
- `20260611-msi-vcredist-prereq.md` — clean-Win11-host machine-MSI install with no VC++ runtime. Code fix DONE (`+crt-static`, commit `a517284b`). Remaining = host-gated UAT only; may SKIP_HOST_UNAVAILABLE.
- `20260611-poc-cert-broker-clean-host.md` — supervised-path broker spawn on a clean host with no manual cert-trust step. Signing-pipeline fix DONE (commit `20cd68d9`). Remaining = cut a trusted-signed release + clean-host UAT; externally blocked on the Azure Trusted Signing verify-gate (`UnknownError`) — plan as gated/deferred, not a hard exit criterion.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RLS-10 | All 6 (not 5 — D-03 correction) workspace version-family crates (`nono`, `nono-cli`, `nono-proxy`, `nono-shell-broker`, `nono-fltmgr-client`, `nono-ffi`) plus `nono-py`/`nono-ts` bindings version-bumped to `0.66.1`, internal path-dep pins consistent, `Cargo.lock` regenerated, workspace builds clean | Architecture Patterns "RLS-10 — Version Bump Surface" enumerates all 6 `[package]` lines + 6 internal path-dep pins + both binding repos' exact files/lines, including the critical uncommitted-working-tree finding (Pitfall 1) and the `tools/sign-fixture` exclusion |
| RLS-11 | Upstream CI #1245 (idempotent `publish-crates` + cross-compile check) and #1251 (compile-step mapping fix) reconciled against the fork's prepare-only pipeline without breaking `release-readiness` or the signed-MSI build order | Summary + Code Examples give the full verified diffs of both commits; Pitfalls 3/4 identify the dead-trigger risk and the correct pre-fixed `if:` form; recommends ADAPT (not ADOPT) for the cross-compile job trigger, clean ADAPT for the idempotency hunk |
| RLS-12 | `nono-py` `RouteConfig` PyPI blocker closed — missing `endpoint_policy` field added (`src/policy.rs:743` + `src/proxy.rs:206`) so the wheel builds and `twine check`/maturin validation passes | Code Examples "RLS-12 — exact nono-py insertion points" gives verbatim before/after snippets at the exact lines, confirms field type `Option<EndpointPolicyConfig>` and `None` validity via ~15 existing `nono-proxy` usages |
| RLS-13 | Release is one-step-push ready at `0.66.1` — `release-dry-run.ps1` + `release-readiness` gate re-run GREEN, `RELEASE-RUNBOOK.md` updated for the `0.66.1` tag with PUBLIC-repo pre-push checklist | Code Examples "RLS-13 — gate constants to update" + Validation Architecture section give the exact 2-constant edit and both invocation commands; Environment Availability confirms all required tooling present (twine absence already gracefully handled) |
</phase_requirements>


## Summary

Phase 100 is a mechanical reconciliation phase, not a design phase — the pipeline, the gate
script, and the binding repos already exist and already passed once at `0.66.0` (v3.3 Phase 97).
The work is: (1) shift six hard-coded `version = "0.66.0"` strings + six internal path-dep pins
to `0.66.1` in the `nono` workspace, plus mirror the bump into two **separate git repos**
(`nono-py`, `nono-ts`); (2) reconcile two small upstream CI commits (`ebd94275` / PR #1245,
`84b5e7ce` / PR #1251) against the fork's tag-push-only release model; (3) insert one missing
struct field at two exact line numbers in `nono-py`; (4) re-point two hard-coded version
constants in `scripts/gates/release-readiness.ps1` and re-run both gate scripts.

The single most important finding this research surfaced: **both binding repos already have
uncommitted, in-progress version-bump edits sitting in their working trees** — `nono-ts` at
`0.4.0 → 0.66.0` and `nono-py` at `0.9.0 → 0.66.0` — left over from a prior session and never
committed. These are not clean starting points; the planner must treat "finish editing these
files to `0.66.1` and commit once" as the task, not "start a fresh bump from a clean tree."
Committing the stale `0.66.0` value as an intermediate commit would fabricate a release that
never shipped and collides with upstream's real `0.66.0` — it must never land, not even as a
DCO-signed commit.

The second most important finding: upstream PR #1245's `cross-compile` CI job is gated on
`startsWith(github.event.pull_request.title, 'chore: release v')` — a release-please-bot PR
title convention. That convention is **100% upstream-only**; every `chore: release v...` commit
in `git log --all` traces to `nolabs-ai/nono`, none to this fork. The fork releases via
`push: tags: v*.*.*` / `workflow_dispatch` (confirmed in `release.yml:1-13`). Adopting #1245's
job verbatim would add a job that **never fires** on this fork. The `publish-crates` idempotency
half of #1245, by contrast, patches the exact same job structure the fork's `release.yml` already
has (`cargo publish -p nono/-proxy/-cli`) and is a clean, low-risk ADAPT. #1251 is a pure YAML
quoting fix for a bug in #1245's own `if:` expression — apply it together with #1245, not
separately.

**Primary recommendation:** Treat RLS-11 as two independent hunks with different dispositions —
ADAPT the `publish-crates` idempotency check (safe, drop-in, apply #1251's quoting fix on top),
and ADAPT (not ADOPT) the `cross-compile` job by rewriting its trigger to something the fork's
branching model actually fires (e.g. `workflow_dispatch` and/or `push`/`pull_request` on
`milestone/*` branches) — record both dispositions in a short `proj/ADR-100-*.md` per D-05.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Workspace crate version bump (RLS-10) | Build/Release tooling (Cargo.toml manifests) | — | Pure manifest edit; no runtime tier owns crate versioning |
| CI/CD pipeline reconcile (RLS-11) | CI/CD (GitHub Actions) | Release/Build tooling | `.github/workflows/*.yml` is infra-as-code, not application code |
| PyPI RouteConfig field fix (RLS-12) | API/Backend (PyO3 binding layer) | — | `nono-py` is a thin FFI wrapper over `nono-proxy`'s `RouteConfig`; the fix is a struct-literal field addition in the binding, not a policy or business-logic change |
| Release gate re-run + runbook (RLS-13) | Release/Build tooling (PowerShell gate scripts) | CI/CD | `scripts/gates/release-readiness.ps1` + `release-dry-run.ps1` are local orchestration, invoked manually by the operator, not triggered by CI |
| Clean-host MSI/broker UAT (folded todos) | OS/Host (Windows installer + service layer) | — | Host-gated; verifies OS-level install/service behavior outside any application tier |

## Standard Stack

This phase does not introduce new libraries. It edits existing, already-approved manifests and
CI YAML. No `Package Legitimacy Audit` section is required — no new external package name is
introduced anywhere in this phase's scope (confirmed by inspecting every diff site below; every
touched dependency is an existing internal workspace crate or an already-vetted binding-repo
dependency).

### Core (existing tools this phase invokes, not installs)

| Tool | Verified Version (this host) | Purpose | Why Standard |
|------|-------------------------------|---------|---------------|
| `cargo` | 1.95.0 `[VERIFIED: cargo --version]` | Workspace build, publish dry-run, `cargo metadata` (used by the gate script) | Already the workspace toolchain |
| `maturin` | 1.14.1 `[VERIFIED: pip show maturin]` | Builds the `nono-py` wheel (PyO3) | Already wired into `release-dry-run.ps1` |
| `twine` | **NOT FOUND on this host** `[VERIFIED: command -v twine]` | Wheel metadata validation before PyPI upload | `release-dry-run.ps1` already has a graceful SKIP + `python -m twine` fallback probe for this exact gap |
| `npm` | 11.15.0 `[VERIFIED: npm --version]` | `nono-ts` package validation (`npm publish --dry-run`) | Already wired into `release-dry-run.ps1` |
| `cargo-audit` | 0.22.1 `[VERIFIED: cargo audit --version]` | Existing `audit` CI job; unaffected by this phase | No change needed |
| PowerShell (`pwsh`) | host default | Runs `release-dry-run.ps1`, `verify-dark.ps1 -Gate release-readiness` | Fork's dark-factory verification harness (auto-discovered gates, no Pester) |

**Installation:** None required. `twine` absence is a pre-existing, already-documented,
non-blocking gap (RELEASE-RUNBOOK.md "Known Pre-Release Blockers" #2) — install via
`pip install twine` only if the operator wants the PyPI leg to go past SKIP; not required for
RLS-13's gate-GREEN exit criterion (the dry-run script's exit code does not require twine to be
present — SKIP is not a failing status).

### Package Legitimacy Audit

**Not applicable this phase.** RLS-10/11/12/13 touch only version strings in existing manifests
and two small CI-YAML hunks from a commit already present in this repo's git history (fetched
from the `upstream` remote). No new crate, npm package, or pip package name is introduced.

## Architecture Patterns

### RLS-10 — Version Bump Surface (verified exhaustive)

**Six `[package] version = "0.66.0"` lines** (workspace members) `[VERIFIED: grep across every Cargo.toml]`:

| File | Crate name | Notes |
|------|-----------|-------|
| `crates/nono/Cargo.toml:3` | `nono` | Publish set member 1/3 |
| `crates/nono-cli/Cargo.toml:3` | `nono-cli` | Publish set member 3/3 |
| `crates/nono-proxy/Cargo.toml:3` | `nono-proxy` | Publish set member 2/3 |
| `crates/nono-shell-broker/Cargo.toml:3` | `nono-shell-broker` | `publish = false` — bump for path-dep consistency only |
| `crates/nono-fltmgr-client/Cargo.toml:3` | `nono-fltmgr-client` | `publish = false`; omitted from RLS-10's stale text but IS one of the release-readiness gate's 6 tracked crates |
| `bindings/c/Cargo.toml:3` | `nono-ffi` | `publish = false` |

**`tools/sign-fixture/Cargo.toml:3` is `version = "0.1.0"` and is NOT part of this family** —
confirmed by git history (`c8b3eacf`, `4aaa0508`): it has always versioned independently as a CI
fixture tool, never tracked the workspace release version. **Do not bump it.** This directly
corrects the CONTEXT.md canonical-refs hedge ("`tools/sign-fixture/Cargo.toml` — per-crate
hard-coded version … if it carries a version") — it does carry one, but bumping it would be
wrong, not merely optional.

**Six internal path-dep `version = "0.66.0"` pins** `[VERIFIED: grep -B2 across every Cargo.toml]`:

| File:line | Dependency edge |
|-----------|-----------------|
| `bindings/c/Cargo.toml:18` | `nono-ffi` → `nono` |
| `crates/nono-shell-broker/Cargo.toml:20` | `nono-shell-broker` → `nono` |
| `crates/nono-cli/Cargo.toml:47` | `nono-cli` → `nono` |
| `crates/nono-cli/Cargo.toml:48` | `nono-cli` → `nono-proxy` |
| `crates/nono-cli/Cargo.toml:180` | `nono-cli` → `nono-shell-broker` (Windows-only `[target.'cfg(target_os = "windows")'.dev-dependencies]`, Phase 41 CR-04) |
| `crates/nono-proxy/Cargo.toml:21` | `nono-proxy` → `nono` |

`crates/nono-fltmgr-client/Cargo.toml` has no internal path-dep edges (self-contained; not
depended on by any other workspace member, confirmed by absence in the grep above).

**`[workspace.package]` `repository`/`homepage`** (`Cargo.toml:14-15`) are both
`https://github.com/always-further/nono` `[VERIFIED: read Cargo.toml]` — stale, matches
CONTEXT.md's flag. Per D-09/Deferred Ideas this is the planner's call whether to fix in-phase
or spin off as a follow-up; it is metadata-only (does not affect build/publish correctness) so
low risk either way.

**`scripts/gates/release-readiness.ps1`** (lines 76-85) hard-codes the exact same six-crate list
under variable `$versionFamilyCrates` and two version constants:
```powershell
$targetVersion       = '0.66.0'
$upstreamHighest     = '0.65.1'
```
`[VERIFIED: read release-readiness.ps1]` — this is independent, cross-checking confirmation that
the six-crate enumeration above is exactly what the gate itself already expects (nono, nono-cli,
nono-proxy, nono-shell-broker, nono-fltmgr-client, nono-ffi). **Both constants must move**:
`$targetVersion` → `'0.66.1'`, `$upstreamHighest` → `'0.66.0'` (the D-09 "0.66.1 > upstream 0.66.0
confirmed" checklist item is this exact assertion — ASSERTION (c) `leapfrog` at line ~163-172).
The existing `no-stale-0.62.2` check (ASSERTION (b), lines 132-160) is a historical guard from an
earlier bump; it is unaffected — do not need to add an analogous `no-stale-0.66.0` check to pass
the gate, though the planner may choose to as defense-in-depth (optional, not required).

**`scripts/release-dry-run.ps1`** references `v0.66.0` only in comments/docstrings (lines 3, 20,
85, 91) describing expected dry-run behavior text — these are **not gate-enforced assertions**,
just human-readable status strings; update for accuracy but they do not block a GREEN exit.

**Binding repos — separate git repositories, both already mid-edit (see Common Pitfalls below
for the critical uncommitted-state finding):**

| Repo | File | Current committed value | Uncommitted working-tree value | Notes |
|------|------|--------------------------|----------------------------------|-------|
| `C:/Users/OMack/nono-py` | `Cargo.toml:3` | `0.9.0` | `0.66.0` (uncommitted) `[VERIFIED: git diff]` | Rust/PyO3 crate version |
| `C:/Users/OMack/nono-py` | `pyproject.toml:7` | `0.9.0` | `0.66.0` (uncommitted) `[VERIFIED: git diff]` | PyPI package version |
| `C:/Users/OMack/nono-py` | `python/nono_py/__init__.py:277` | `__version__ = "0.9.0"` | **unchanged, still `0.9.0`** `[VERIFIED: grep]` | ⚠️ Independent version marker — NOT part of the 0.66.x family; do not touch |
| `C:/Users/OMack/nono-ts` | `Cargo.toml:3` | `0.4.0` | `0.66.0` (uncommitted) `[VERIFIED: git diff]` | napi-rs crate; `name = "nono-node"` |
| `C:/Users/OMack/nono-ts` | `Cargo.toml:18` (`nono` path-dep) | `version = "0.62"` | `version = "0.66"` (uncommitted) `[VERIFIED: git diff]` | Caret-style minor-version req — `"0.66"` already satisfies `0.66.1`; **no further edit needed** for this specific line |
| `C:/Users/OMack/nono-ts` | `package.json:3` | `0.4.0` | `0.66.0` (uncommitted) `[VERIFIED: git diff]` | Main npm package |
| `C:/Users/OMack/nono-ts` | `package.json:78-82` (`optionalDependencies`) | five `0.4.0` entries (4 present + `win32-x64-msvc` added fresh) | five `0.66.0` entries (uncommitted) `[VERIFIED: git diff]` | See pitfall below re: `win32-x64-msvc` |
| `C:/Users/OMack/nono-ts` | `npm/darwin-arm64/package.json:3`, `npm/darwin-x64/package.json:3`, `npm/linux-arm64-gnu/package.json:3`, `npm/linux-x64-gnu/package.json:3` | `0.4.0` each | `0.66.0` each (uncommitted) `[VERIFIED: git diff]` | 4 of the 5 optional-dependency targets have a tracked package.json in this repo |

`nono-py`'s `Cargo.toml` dependency on `nono-proxy` is **path-only, no version string**
(`nono-proxy = { path = "../Nono/crates/nono-proxy" }`) `[VERIFIED: grep]` — no version-pin edit
needed there.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Detecting version drift across 6+ manifests | A new ad-hoc grep/sed script | The existing `scripts/gates/release-readiness.ps1` ASSERTION (a) `version-family` check (already does `cargo metadata` + compares against `$versionFamilyCrates`) | Already exists, already tested, already the canonical gate — just update its two constants |
| Verifying the three-registry publish path | New bespoke dry-run tooling | `scripts/release-dry-run.ps1` (already covers crates.io / PyPI / npm with graceful SKIPs for absent tools) | Purpose-built for exactly this in Phase 97; re-running it is the RLS-13 literal ask |
| Deciding CI-hunk disposition ad hoc | Free-form commit message explaining the choice | A short `proj/ADR-NN-*.md` (pattern: ADR-74, ADR-86, ADR-87, ADR-98 — one ADR per phase-number, `Status/Context/Decision` shape) | D-05 explicitly calls for "adapt + short ADR", mirroring the ADR-98 precedent this same milestone already used |

## Common Pitfalls

### Pitfall 1: Binding repos are NOT clean-tree starting points

**What goes wrong:** A plan that says "edit `nono-py/Cargo.toml` version from `0.9.0` to
`0.66.1`" will fail — `git diff` in that repo right now shows `Cargo.toml` and `pyproject.toml`
already modified in the working tree, with the version already sitting at the intermediate,
uncommitted value `0.66.0` (not the last-committed `0.9.0`). Same for `nono-ts`: `Cargo.toml`,
`package.json`, `package-lock.json`, and all four tracked `npm/*/package.json` files are already
modified in-place from `0.4.0` to `0.66.0`, uncommitted.
**Why it happens:** A prior session (almost certainly v3.3 Phase 97's version-leapfrog work)
started the binding-repo bump but the commits were never made — the edits were left in the
working tree across a session boundary.
**How to avoid:** Plan tasks as "finish editing the already-modified files from `0.66.0` →
`0.66.1`, then make ONE DCO-signed commit per repo" — not "start from committed HEAD." Never
commit the intermediate `0.66.0` value as its own commit (it would fabricate a release that
collides with upstream's real, already-published `0.66.0`).
**Warning signs:** `git status` in either binding repo showing modified files before any Phase
100 edit has been made is the tell — confirm with `git diff` before writing new edits, don't
assume a clean tree.
**Also note (nono-ts only):** `git status` additionally shows three **untracked** scratch files
(`decode-test.js`, `test-broker.js`, `test-confined.js`) at repo root, unrelated to this phase —
do not `git add -A`; stage only the specific version-bump files.

### Pitfall 2: `win32-x64-msvc` npm subpackage has no local `package.json` to edit

**What goes wrong:** `nono-ts/package.json`'s `optionalDependencies` block references
`"nono-ts-win32-x64-msvc": "0.66.0"` (already present in the uncommitted diff) but there is
**no** `npm/win32-x64-msvc/` directory in this repo (`git ls-files npm/` lists only
`darwin-arm64`, `darwin-x64`, `linux-arm64-gnu`, `linux-x64-gnu`).
**Why it happens:** The Windows npm subpackage is apparently generated/published from a
different location (CI artifact, not checked into `nono-ts` git history) — this is a pre-existing
repo-structure gap, not something Phase 100 introduced.
**How to avoid:** Bump the `optionalDependencies` string for `win32-x64-msvc` in the main
`package.json` (already done, uncommitted, at `0.66.0` → needs `0.66.1`) but do not expect (or
create) a `npm/win32-x64-msvc/package.json` file — there isn't one to edit in this repo, and
manufacturing one is out of this phase's scope (RLS-10 is a version-string sync, not a
napi-target-completeness fix).
**Warning signs:** A task that says "bump all 5 `npm/*/package.json` files" will fail to find a
5th directory — confirm the count is 4 before planning file-edit tasks.

### Pitfall 3: `#1245`'s cross-compile CI job trigger will never fire on the fork

**What goes wrong:** Cherry-picking `ebd94275` verbatim adds a `cross-compile` job to `ci.yml`
gated `if: startsWith(github.event.pull_request.title, 'chore: release v') && ...`. The fork has
**zero** history of any PR or commit titled `chore: release v...` — every such commit in
`git log --all` traces to the `upstream` remote (`nolabs-ai/nono`'s release-please-style bot).
The fork's own release trigger is `on: push: tags: v*.*.*` / `workflow_dispatch`
(`release.yml:1-13`) — a completely different flow (no bot-authored release PR ever exists).
**Why it happens:** Upstream uses a release-please-like bot workflow (opens a PR titled
`chore: release vX.Y.Z` before tagging); the fork does not use that pattern at all.
**How to avoid:** ADAPT, not ADOPT — rewrite the `if:` condition to a trigger the fork's actual
branching model produces. Two workable options: (a) `workflow_dispatch` only (the operator runs
it on-demand right before pushing a tag — fits the prepare-only/operator-gated posture cleanly),
or (b) trigger on `pull_request`/`push` where `github.head_ref` (or `github.ref_name`) matches
the fork's `milestone/*` branch convention (confirmed live: current branch is
`milestone/v2.13-carryforward-closeout`). Record whichever is chosen, and why, in the ADR (D-05).
**Warning signs:** A "cross-compile check passed" claim based on a job that has a
`pull_request.title` condition and was never actually invoked in a real PR run is a false
verification — check the Actions run history for an actual execution, not just YAML syntax
validity.

### Pitfall 4: `#1251` is a bugfix for `#1245`'s own code, not an independent hunk

**What goes wrong:** Treating #1245 and #1251 as two separately-schedulable absorb tasks risks
landing #1245's buggy `if:` expression (`${{ startsWith(...) && ... }}` — invalid boolean-context
usage of the `${{ }}` wrapper inside an `if:` that GitHub Actions evaluates as a job condition)
without its own follow-on fix.
**Why it happens:** #1251 (`84b5e7ce`, one day younger) exists **specifically** to fix the exact
`if:` line #1245 added — `git show 84b5e7ce` touches nothing except that one line, converting
`if: ${{ EXPR }}` to `if: "EXPR"` (quoted-string form, the GitHub Actions–recommended way to
write a boolean job condition without the double-evaluation footgun).
**How to avoid:** When adapting #1245's `cross-compile` job (per Pitfall 3, with a rewritten
trigger), write the `if:` in the already-corrected quoted-string style from the start — there is
no reason to reproduce the historical bug in the fork just because upstream shipped it and fixed
it one day later.
**Warning signs:** Any `if: ${{ ... }}` (double-braced) job condition freshly written into
`ci.yml` in this phase is worth a second look — the recommended GitHub Actions style for a
boolean job-level `if:` is the bare/quoted-string form, not the `${{ }}` wrapper.

### Pitfall 5: Cargo.lock regen from a pure version bump should not pull dependency changes — verify it doesn't

**What goes wrong:** `cargo build`/`cargo generate-lockfile` after bumping 6 workspace member
versions is expected to touch only the workspace members' own `version` fields inside
`Cargo.lock` (their internal `dependencies = [...]` graphs are unaffected — no external crate's
resolved version should move, since no `Cargo.toml` `[dependencies]` entry for a *third-party*
crate is being touched by RLS-10).
**Why it happens (if it does happen):** A `cargo update` (not `cargo build`) run instead of a
plain rebuild could pull newer semver-compatible third-party releases that happened to ship since
the lockfile was last regenerated, silently broadening the diff beyond the version bump.
**How to avoid:** Regenerate via `cargo build --workspace` (or `cargo check --workspace`), not
`cargo update`. After regen, diff `Cargo.lock` and confirm the only changed lines are the 6
workspace-member `version = "0.66.1"` entries (each member's own `[[package]]` block) — any
third-party crate version change in the diff is unexpected and should be investigated, not
silently accepted.
**Warning signs:** A `Cargo.lock` diff wider than ~6-12 hunks (6 members × ~1-2 lines each) after
a pure version bump is a signal something pulled in more than intended.

### Pitfall 6: `cargo search` (used by #1245's idempotency check) is a live network call

**What goes wrong:** The idempotency hunk (`if cargo search nono --limit 1 | grep -q "\"$VERSION\""`)
makes a real HTTP call to crates.io's search API during every `publish-crates` CI run. This is
confirmed **currently functional** on this host (`cargo search nono --limit 1` returned
`nono = "0.66.0"` live during this research session) but crates.io's search API has a documented
history of rate-limiting/deprecation churn — it is not guaranteed stable indefinitely.
**Why it happens:** Upstream chose the simplest idempotency check available at the time
(`cargo search` + grep) rather than the more robust `cargo info <pkg>@<version>` or a direct
`https://crates.io/api/v1/crates/<pkg>` `curl` call.
**How to avoid:** Adopt the hunk as-is (it works today, verified live) but note in the ADR that
this is a `[VERIFIED: cargo search nono --limit 1, 2026-07-01]` MEDIUM-durability check — if it
ever breaks, the fix is swapping to `cargo info` or a direct registry API call, not re-litigating
whether idempotent publish is worth having.
**Warning signs:** A `publish-crates` job failure with an error mentioning search API
deprecation/auth is the signal to swap the check method, not to remove idempotency entirely.

## Code Examples

### RLS-11 — exact upstream diffs (already present in this repo's git objects)

Both commits exist locally via the `upstream` remote (`nolabs-ai/nono`) — no fetch needed,
confirmed by `git show`:

```bash
# Source: local git object, upstream remote nolabs-ai/nono
git show ebd94275   # PR #1245 — idempotent publish-crates + cross-compile check
git show 84b5e7ce    # PR #1251 — fixes #1245's own if: expression
```

`ebd94275` touches `.github/workflows/ci.yml` (+57 lines, new `cross-compile` job) and
`.github/workflows/release.yml` (+20/-3 lines, wraps each of the 3 `cargo publish -p ...` calls
in `cargo search <pkg> --limit 1 | grep -q "\"$VERSION\""` idempotency guards). The
`release.yml` hunk maps cleanly onto the fork's existing `publish-crates` job
(`release.yml:483-520`) — same 3 crates, same publish order, same `--allow-dirty --token`
invocation — this half is a clean ADAPT with no structural conflict.

`84b5e7ce` touches only `.github/workflows/ci.yml`, one line: converts
`if: ${{ startsWith(github.event.pull_request.title, 'chore: release v') && ... }}` to
`if: "startsWith(github.event.pull_request.title, 'chore: release v') && ..."` (quoted-string
form). Apply this correction *inline* when writing the fork's rewritten trigger condition
(Pitfall 3/4) rather than reproducing then re-fixing the bug.

### RLS-12 — exact nono-py insertion points

```rust
// Source: C:/Users/OMack/nono-py/src/proxy.rs:206-224 (RouteConfig::new constructor)
// Insert `endpoint_policy: None,` as a new field inside this literal, e.g. after `tls_ca,` (line 223):
Self {
    inner: RustRouteConfig {
        prefix,
        upstream,
        // ...existing fields...
        endpoint_rules: endpoint_rules
            .into_iter()
            .map(|(method, path)| RustEndpointRule { method, path })
            .collect(),
        tls_ca,
        endpoint_policy: None,   // <-- ADD (RLS-12 / D-07)
    },
}
```

```rust
// Source: C:/Users/OMack/nono-py/src/policy.rs:741-760 (impl From<PolicyRouteConfig> for RustRouteConfig)
impl From<PolicyRouteConfig> for RustRouteConfig {
    fn from(route: PolicyRouteConfig) -> Self {
        Self {
            prefix: route.prefix,
            // ...existing fields...
            endpoint_rules: route.endpoint_rules.into_iter().map(Into::into).collect(),
            tls_ca: route.tls_ca,
            endpoint_policy: None,   // <-- ADD (RLS-12 / D-07)
        }
    }
}
```

Field-type confirmation: `crates/nono-proxy/src/config.rs:195` declares
`pub endpoint_policy: Option<EndpointPolicyConfig>` on the Rust-side `RouteConfig` that `nono-py`
wraps — `None` is unambiguously valid; the core `nono-proxy` crate itself uses `endpoint_policy:
None,` at ~15 other construction sites (`credential.rs`, `route.rs`, `server.rs`)
`[VERIFIED: grep across crates/nono-proxy/src]`.

### RLS-13 — gate constants to update

```powershell
# Source: scripts/gates/release-readiness.ps1:76-77
$targetVersion       = '0.66.0'    # -> '0.66.1'
$upstreamHighest     = '0.65.1'    # -> '0.66.0'
```

## Runtime State Inventory

Not applicable — this is a version-bump/CI-reconcile phase, not a rename/refactor/migration
phase. No stored data, live service config, OS-registered state, or secrets are touched.
(The binding-repo uncommitted-edit finding in Common Pitfalls #1 is a session-continuity hazard,
not runtime state in the sense this section covers.)

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Recommended fix for #1245's cross-compile trigger (`workflow_dispatch` and/or `milestone/*` branch match) is the best adaptation | Common Pitfalls #3 | Low — this is explicitly flagged as the planner's/D-05's call, not a locked decision; any working trigger satisfies RLS-11's literal text |
| A2 | `cargo search`'s crates.io API will remain stable enough for the idempotency check's continued use | Common Pitfalls #6 | Low — verified working today; if crates.io deprecates the search API, the fix is a mechanical swap to `cargo info`, not a design change |
| A3 | Correcting `[workspace.package] repository`/`homepage` from `always-further` to the fork is out of RLS-10's literal scope | Architecture Patterns / RLS-10 | Low — CONTEXT.md D-09/Deferred Ideas already flags this as the planner's discretion; metadata-only, no build/publish impact either way |

No claim above required a package-name provenance check (no new packages introduced this phase).

## Open Questions

1. **Should `CHANGELOG.md` get a `0.66.1` (or a retroactive `0.66.0`) entry?**
   - What we know: `CHANGELOG.md`'s newest entry is `[0.62.2]` (2026-06-06) — it has not been
     updated across v3.0, v3.1, v3.2, v3.3, or the current `0.66.0` leapfrog. This drift predates
     Phase 100 and is not called out in CONTEXT.md's decisions or canonical refs.
   - What's unclear: Whether RLS-13's "`RELEASE-RUNBOOK.md` updated for the `0.66.1` tag" implies
     `CHANGELOG.md` too, or whether that file has been deliberately deprioritized.
   - Recommendation: Out of scope unless the planner decides otherwise — CONTEXT.md's D-09 names
     only `RELEASE-RUNBOOK.md`; treat `CHANGELOG.md` as a pre-existing, separate gap (flag to
     the operator, do not silently expand Phase 100 to fix multi-milestone changelog debt).

2. **Does the release-readiness gate need a new `no-stale-0.66.0` assertion (mirroring its
   existing `no-stale-0.62.2` check)?**
   - What we know: The existing ASSERTION (b) hard-codes `0.62.2` as the one historical stale
     value to guard against; there is no generic "no stale prior version" check.
   - What's unclear: Whether a `0.66.0` residue anywhere in a tracked `Cargo.toml` after the bump
     would be caught by any *other* existing assertion (ASSERTION (a) `version-family` would
     catch it in the 6 tracked crates specifically, since it checks `-ne $targetVersion`; a
     stray `0.66.0` string in a *non*-version-family file, e.g. a stale comment, would not be
     caught by anything).
   - Recommendation: Not required to satisfy RLS-13's literal GREEN-exit criterion (ASSERTION (a)
     already catches the crates that matter); optional defense-in-depth the planner may add.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | RLS-10 build verification, RLS-13 dry-run | ✓ | 1.95.0 | — |
| `maturin` | RLS-12 wheel build, RLS-13 PyPI dry-run leg | ✓ | 1.14.1 | — |
| `twine` | RLS-13 PyPI `twine check` leg | ✗ | — | `release-dry-run.ps1` already SKIPs gracefully + has a `python -m twine` fallback probe; install via `pip install twine` only if operator wants the leg to go past SKIP |
| `npm` | RLS-10 nono-ts bump, RLS-13 npm dry-run leg | ✓ | 11.15.0 | — |
| `cargo-audit` | Existing `audit` CI job (unaffected by this phase) | ✓ | 0.22.1 | — |
| PowerShell (`pwsh`) | RLS-13 gate scripts (`release-dry-run.ps1`, `verify-dark.ps1`) | ✓ | host default | — |
| Docker (`cross`) + zig (`cargo-zigbuild`) | Cross-target clippy gate — only if any Rust source (not just manifests/YAML) is touched | ✓ (per STATE.md Phase 96) | — | Pure version-bump/manifest edits do not require this gate; only invoke if a code-level change sneaks into scope |
| Clean Win11 VM (no VC++ redist) | Folded todo `20260611-msi-vcredist-prereq` UAT | Unknown / likely ✗ on this dev host | — | `SKIP_HOST_UNAVAILABLE` per CONTEXT.md — host-gated, may be skipped |

**Missing dependencies with no fallback:** None — `twine`'s absence has a documented,
already-implemented fallback (SKIP + `python -m twine` probe) that does not block RLS-13's exit
code.

**Missing dependencies with fallback:** `twine` (see above). Clean Win11 VM (folded UAT — may
`SKIP_HOST_UNAVAILABLE`).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Fork's custom PowerShell "dark-factory" harness (`scripts/verify-dark.ps1`), auto-discovering gate scripts under `scripts/gates/*.ps1` — not Pester, not a unit-test framework |
| Config file | None — gates are plain PowerShell functions, auto-discovered by filename (`scripts/verify-dark.ps1:130-146`) |
| Quick run command | `pwsh -File scripts/gates/release-readiness.ps1` (direct dot-source invocation for fast iteration while editing the two version constants) |
| Full suite command | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (canonical invocation — writes JSON verdict to `.nono-runtime/verdicts/release-readiness.json`) |

Supplementary: `pwsh -File scripts/release-dry-run.ps1` is not a "gate" in the verify-dark
auto-discovery sense but is the other mandatory-green script per RLS-13/D-09. `make ci`
(clippy + fmt + tests) remains the standard Rust-level regression check for RLS-10's "workspace
builds clean" requirement — no new Rust logic is introduced, so `cargo test` should be a
no-op-relative-to-baseline regression check, not a targeted new-test requirement.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RLS-10 | All 6 version-family crates report `0.66.1`; workspace builds clean | gate + build | `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (ASSERTION a) + `cargo build --workspace` | ✅ (gate script exists; needs 2-constant edit) |
| RLS-11 | Reconciled CI hunks don't break existing pipeline | manual YAML review + (optional) a dry-run `act`/push-to-fork-branch smoke, since GH Actions YAML has no local unit-test story | N/A — no automated command; `actionlint` (if available) as a lint-only sanity check | ❌ — no test file; this is CI-config, inherently smoke-verified only via an actual push |
| RLS-12 | `nono-py` wheel builds; `twine check` (or SKIP) passes | integration (external repo build) | `cd C:/Users/OMack/nono-py && maturin build` then `pwsh -File <nono-repo>/scripts/release-dry-run.ps1` (re-run from the nono repo, which shells out into the binding repo's build) | ✅ — `release-dry-run.ps1`'s PyPI leg already exists |
| RLS-13 | Both `release-dry-run.ps1` and `release-readiness` gate exit GREEN at `0.66.1` | gate + script | `pwsh -File scripts/release-dry-run.ps1` (exit 0) + `pwsh -File scripts/verify-dark.ps1 -Gate release-readiness` (PASS) | ✅ — both scripts exist |

### Sampling Rate
- **Per task commit:** `pwsh -File scripts/gates/release-readiness.ps1` (fast local check after each version-string edit)
- **Per wave merge:** `pwsh -File scripts/release-dry-run.ps1` full run + `make ci`
- **Phase gate:** Both scripts GREEN before `/gsd:verify-work`, per RLS-13/D-09

### Wave 0 Gaps

None — the release-readiness gate script and the dry-run orchestrator already exist and already
passed once at `0.66.0` (v3.3 Phase 97). No new test infrastructure is needed; this phase edits
two hard-coded constants in an existing gate and re-runs existing scripts.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Fork crate version `0.66.0` (collides with upstream's now-also-`0.66.0`) | Fork crate version `0.66.1` (strictly above upstream) | This phase (RLS-10) | Restores a collision-free, publishable version; the `0.66.0` value must never be published from the fork |
| `publish-crates` job publishes unconditionally (fails loudly on retry if a crate already landed) | Idempotent publish — checks `cargo search` before each `cargo publish` | Upstream PR #1245 (2026-06-24), adapted here | Safe to re-run `release.yml` after a partial failure without manual crates.io state inspection |

**Deprecated/outdated:** Nothing in this phase's scope is being deprecated — this is additive
reconciliation, not removal.

## Sources

### Primary (HIGH confidence)
- `git show ebd94275` / `git show 84b5e7ce` — local git objects, upstream `nolabs-ai/nono` remote, full diffs read directly
- `crates/*/Cargo.toml`, `bindings/c/Cargo.toml`, `tools/sign-fixture/Cargo.toml` — read/grepped directly, this repo
- `scripts/gates/release-readiness.ps1`, `scripts/release-dry-run.ps1` — read directly, this repo
- `.github/workflows/ci.yml`, `.github/workflows/release.yml` — read/grepped directly, this repo
- `C:/Users/OMack/nono-py/src/proxy.rs`, `.../src/policy.rs`, `.../Cargo.toml`, `.../pyproject.toml` — read directly, separate repo, including live `git diff`/`git status`
- `C:/Users/OMack/nono-ts/package.json`, `.../Cargo.toml`, `.../npm/*/package.json` — read directly, separate repo, including live `git diff`/`git status`
- `crates/nono-proxy/src/config.rs`, `credential.rs`, `route.rs`, `server.rs` — grepped for `endpoint_policy` field type + usage precedent
- Live tool probes on this host: `cargo --version`, `maturin` (via `pip show`), `twine` (absent), `npm --version`, `cargo-audit --version`, `cargo search nono --limit 1`
- `.planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md` — prior-phase runbook, cross-checked against live gate script constants (fully consistent)
- `proj/ADR-98-network-intent-disposition.md` — ADR format/naming precedent

### Secondary (MEDIUM confidence)
- None — every claim in this document was directly verified against a local file, a local git object, or a live command probe during this research session.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new tools; every existing tool's presence/version verified live
- Architecture (version surface + CI reconcile): HIGH — every file/line enumerated via direct grep/read, cross-checked against the independent `release-readiness.ps1` crate list
- Pitfalls: HIGH — the uncommitted-binding-repo-edits finding and the PR-title-trigger-dead-code finding were both discovered via direct `git diff`/`git log` inspection, not inferred

**Research date:** 2026-07-01
**Valid until:** Short-lived — this research describes the *current* uncommitted state of two
external repos (`nono-py`, `nono-ts`). If those repos are touched by any other process before
Phase 100 executes, the "already at `0.66.0` uncommitted" finding may be stale; re-run
`git status`/`git diff` in both before starting execution.

## RESEARCH COMPLETE

**Phase:** 100 - Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker
**Confidence:** HIGH — every RLS-10/11/12/13 claim verified directly against local files, git objects (including the fetched upstream commits `ebd94275`/`84b5e7ce`), and live tool probes; no unverifiable claims remain.

### Key Findings
- Both `nono-py` and `nono-ts` binding repos already have **uncommitted** version-bump edits sitting in their working trees (`nono-ts`: `0.4.0→0.66.0`; `nono-py`: `0.9.0→0.66.0`) — the plan must finish these edits to `0.66.1` and commit once, never landing the intermediate `0.66.0` value.
- Upstream PR #1245's `cross-compile` CI job triggers on a `chore: release v...` PR-title convention that is 100% upstream-only (release-please-bot pattern) — verbatim adoption would add dead CI code; the trigger must be rewritten (e.g. `workflow_dispatch` and/or `milestone/*` branch match) as the ADAPT half of D-05.
- PR #1245's `publish-crates` idempotency hunk is a clean, low-risk ADAPT (same job structure the fork already has); PR #1251 is a one-line quoting fix for a bug in #1245's own code — apply both together, pre-corrected.
- RLS-10's version surface is exactly 6 `[package]` version lines + 6 internal path-dep pins in the `nono` workspace (verified exhaustively), plus `tools/sign-fixture` which must explicitly NOT be touched (independent 0.1.0 versioning).
- RLS-12's exact insertion points (`nono-py/src/proxy.rs` after line 223, `nono-py/src/policy.rs` after line 757) and field type (`Option<EndpointPolicyConfig>`, `None` confirmed valid via ~15 existing usages in `nono-proxy`) are fully confirmed.
- RLS-13's gate script (`scripts/gates/release-readiness.ps1:76-77`) needs exactly two constant edits (`$targetVersion` and `$upstreamHighest`); no new test infrastructure required.

### File Created
`.planning/phases/100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker/100-RESEARCH.md`

### Confidence Assessment
| Area | Level | Reason |
|------|-------|--------|
| Standard Stack | HIGH | No new tools; every existing tool verified live on this host |
| Architecture | HIGH | Every version-surface file/line enumerated via direct grep, cross-checked against the independent gate script's own crate list |
| Pitfalls | HIGH | Both critical findings (uncommitted binding-repo state, dead CI trigger) discovered via direct git inspection, not inference |

### Open Questions
1. Whether `CHANGELOG.md`'s multi-milestone staleness (last entry `0.62.2`) is in scope — recommended: out of scope, flag to operator separately.
2. Whether the release-readiness gate should gain a `no-stale-0.66.0` assertion mirroring its existing `no-stale-0.62.2` check — recommended: optional, not required for GREEN.

### Ready for Planning
Research complete. Planner can now create PLAN.md files for Phase 100.
