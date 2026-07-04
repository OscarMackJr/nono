# Phase 105: Live Multi-Registry Publish - Research

**Researched:** 2026-07-03 (current session date; ROADMAP/REQUIREMENTS timestamped 2026-07-02)
**Domain:** crates.io dependency-ordered publishing + sparse-index semantics, PyPI (maturin/twine) multi-platform wheel publishing, npm/napi-rs scoped multi-package publishing, cross-repo GitHub/CI topology for two sibling binding repos that are NOT currently forked
**Confidence:** HIGH for crates.io mechanics (empirically verified live against the real registry + official cargo PR history); MEDIUM-HIGH for PyPI/npm mechanics (official docs + empirically verified local tooling availability); the two strategic questions are both resolved with HIGH confidence evidence

## Summary

PUB-02 is a three-registry live publish, but the single most consequential discovery this session is **structural, not procedural**: the two sibling binding repos (`../nono-py`, `../nono-ts`) have **no fork-owned GitHub repository at all**. Their `git remote -v` origins still point at the upstream org (`always-further/nono-py` and `always-further/nono-ts`, which GitHub now redirects to the relocated `nolabs-ai` org), and `gh repo view OscarMackJr/nono-py` / `OscarMackJr/nono-ts` both fail with "Could not resolve to a Repository" — **verified live this session**. This means the Phase 102 rename commits (`787e2dd`, `c2f5aaa`) sit only in local git history on a local branch, unpushed anywhere the fork controls, and each sibling repo's own `publish.yml` (which builds macOS/Linux wheels or native addons via GitHub-hosted runners) is upstream-owned CI the fork cannot dispatch or rely on. `../nono-py/publish.yml` even hard-guards every job with `if: github.repository == 'always-further/nono-py'`, which would never be true for a fork regardless. **Practical consequence for the plan:** genuine multi-platform coverage for PyPI and npm requires either (a) creating real `OscarMackJr/nono-py` and `OscarMackJr/nono-ts` GitHub repos and repairing+re-enabling their CI (recommended — reuses this project's own already-proven 5-target CI pattern from the main `nono` repo), or (b) accepting a reduced, honestly-documented local-only platform matrix (Windows-native guaranteed; Linux via experimental Docker-`cross`/zig cross-compilation, already installed on this host from Phase 96; macOS not achievable at all without new infrastructure). This is a first-order planning decision, not a footnote.

The second major finding closes research focus #3 concretely: `../nono-ts/package.json` already declares `napi.targets` including `x86_64-pc-windows-msvc` and `optionalDependencies` already references `@oscarmackjr/nono-ts-win32-x64-msvc` — but **`npm/win32-x64-msvc/` does not exist on disk**, and neither `.github/workflows/ci.yml` nor `.github/workflows/publish.yml` in `nono-ts` build a Windows leg at all (both matrices cover only macOS x64/arm64 + Linux x64/aarch64-gnu). This is the exact "documented missing-platform-package failure" the phase description warns about, reproduced concretely in this repo today, not hypothetical: if Phase 105 published today using the existing CI machinery unmodified, `npm install @oscarmackjr/nono-ts` would succeed on macOS/Linux but **fail to resolve a working native binding on Windows** (the optionalDependency would 404, napi-rs's JS loader has no fallback, `require()` throws). Any publish plan must add a Windows build leg before npm publish, not treat the 4 existing platform folders as complete.

Third, the crates.io mechanics are simpler than the phase description implies: **`cargo publish` has blocked on index-visibility by default since Cargo 1.66 (Dec 2022)** — verified via the merged upstream cargo PR history (`rust-lang/cargo#11062`, `#11356`) — and this host runs cargo 1.95.0. The existing (neutralized) `release.yml` job's `sleep 30` + `cargo search`-based skip-check is both unnecessary (cargo already blocks) and broken (`cargo search` is deprecated/returns empty against crates.io — already documented in Phase 102 research, re-confirmed this session). The correct design leans on cargo's built-in blocking as the primary mechanism, adds an explicit, observable sparse-index poll as a defensive/audit-trail layer (satisfying SC1's literal wording), and replaces the broken `cargo search` idempotency check with a direct sparse-index or registry-API version lookup.

Both strategic questions are resolved below with live evidence. **Primary recommendation:** author and harden the full three-registry publish machinery now (scripts, CI diffs, verification tooling, dry-runs) with zero live publish action — the machinery has no technical dependency on Phase 104's signed `.exe` — but leave the actual operator-executed live-publish checkpoints sequenced after Phase 104's tag confirmation, for version-consistency/irreversibility risk management (a process reason, not a technical one). Separately and urgently: flag the npm scope-ownership gap (still true, re-verified live) and the missing sibling-repo hosting as blocking operator action items that sit outside code the assistant can author.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| crates.io source-crate publish (`cargo publish`) | This repo's Cargo workspace (`OscarMackJr/nono`, real fork) | crates.io registry (external) | Fully local + this repo's own CI; no sibling-repo dependency |
| PyPI wheel/sdist build (`maturin build`) | `../nono-py` local working tree | PyPI registry (external) | Sibling repo has NO fork-owned GitHub host — build must happen on whatever machine runs it (this operator's Windows host, or a to-be-created fork's CI) |
| PyPI publish (`twine upload`) | Operator's local machine (or a to-be-created fork's CI with OIDC trusted publishing) | PyPI registry | `twine` is the phase-mandated mechanism; not yet installed on this host (verified) |
| npm native addon build (`napi build`) | `../nono-ts` local working tree (Windows-native) / GitHub-hosted runners (macOS, Linux) | — | Same sibling-repo hosting gap as PyPI; Windows leg is 100% locally buildable today, macOS/Linux legs need either a real fork+CI or experimental local cross-compilation |
| npm publish (`npm publish`) | Operator's local machine (or a to-be-created fork's CI) | npm registry | Requires `npm login`/token (not configured — verified) AND the `@oscarmackjr` scope to exist (not created — verified) |
| Publish sequencing / idempotency / index-polling | New/extended local script (`scripts/release-dry-run.ps1` counterpart) | `release.yml` `publish-crates` job (crates.io only, this repo's own CI) | Only the crates.io leg has a genuinely fork-owned CI surface to re-enable; PyPI/npm legs cannot be CI-driven until sibling repos are actually forked |

## User Constraints

No CONTEXT.md exists for this phase (verified: `.planning/phases/105-live-multi-registry-publish/` is empty apart from this file). No `/gsd:discuss-phase` decisions to carry forward — this research is the first artifact for the phase.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PUB-02 | `0.66.1` published LIVE to crates.io (dependency order + index polling), PyPI (maturin + twine `--skip-existing`), npm (`@oscarmackjr/nono-ts` + all platform packages); each registry's own idempotency respected; `cargo install`/`pip install`/`npm i` all resolve post-publish | See Standard Stack, Architecture Patterns, Common Pitfalls, Validation Architecture below — covers all 4 ROADMAP success criteria (SC1 crates.io dependency-order+polling, SC2 PyPI+coverage-verification, SC3 npm+platform-completeness, SC4 post-publish resolve checks) |

## Strategic Question 1 — Does the live registry publish genuinely require Phase 104's signed release?

**Answer: No technical dependency. The ROADMAP's Phase 104 dependency is a process/version-consistency safeguard, not a build requirement. Recommendation: author + harden + dry-run all Phase 105 machinery NOW, in parallel with Phase 104 remaining blocked; keep the actual live-publish operator checkpoints sequenced after Phase 104's tag confirmation.**

Evidence:
- `cargo publish -p nono-sandbox` was run live (`--dry-run`) this session against the actual current source tree and succeeded cleanly (packaged 54 files, compiled, "aborting upload due to dry run"). Cargo packages **the current Cargo.toml/source tree at the version already set (`0.66.1`)** — nothing about this depends on a Windows Trusted-Signing certificate, a signed `.exe`, or a cut GitHub Release. `[VERIFIED: local cargo publish --dry-run run this session]`
- PyPI's artifact is a maturin-built wheel/sdist containing a compiled Python extension module (`_nono_py.pyd`/`.so`) — never the signed `nono.exe`/MSI. `[VERIFIED: read ../nono-py/pyproject.toml + Cargo.toml this session]`
- npm's artifact is a napi-rs native `.node` addon — same reasoning, no coupling to the signed CLI binary. `[VERIFIED: read ../nono-ts/package.json this session]`
- The actual git tag `v0.66.1` **has not been pushed yet** — only `v0.66.0` exists (`git tag -l "v0.66*"` this session) — and Phase 104's own plan (`104-03-PLAN.md`) documents the tag push as still gated behind an **external, unresolved Microsoft root-certificate propagation blocker** (not a code defect in this repo). `[VERIFIED: read 104-03-PLAN.md + git tag this session]`
- ROADMAP.md's own Phase 106 entry proves the project already treats "needs the release" and "needs the live publish" as two independently satisfiable conditions in an analogous case: *"Depends on: Phase 103 ... + Phase 104 ... — does NOT require Phase 105's live registry publish; the GitHub Release artifact is sufficient."* The same reasoning applies in reverse to Phase 105: Phase 105 needs *a version number and source tree to package*, which already exist at `0.66.1` in this repo's Cargo.toml — it does not need the *signed artifact* Phase 104 produces.

Why the ROADMAP dependency should nonetheless stay as a **process gate** (not be removed): crates.io publishes are permanent — a version, once uploaded, can never be replaced or deleted, only yanked (which still leaves the number burned). If Phase 105 published `nono-sandbox 0.66.1` today and Phase 104's blocker forces the eventual signed release to become `0.66.2` (e.g. if further code changes are needed while waiting on Microsoft), the registries would carry a permanently-orphaned `0.66.1` with no corresponding signed GitHub Release — a provenance/trust problem for anyone auditing "does this crates.io version correspond to a real signed release." **Recommendation given to the planner:** structure Phase 105 as two layers — (1) an autonomous, fully-buildable-now layer (author scripts, fix CI, run every dry-run leg, fix the missing npm Windows platform package, create sibling-repo forks if that path is chosen) that has zero dependency on Phase 104, and (2) a small number of `checkpoint:human-action` tasks for the actual `cargo publish`/`twine upload`/`npm publish` invocations, each of which the operator should only execute after Phase 104's tag `v0.66.1` is confirmed released (matching the ROADMAP's stated intent), not before. This lets Phase 105 execution proceed today without idling on Phase 104's external blocker, while preserving the version-consistency safety property the ROADMAP dependency exists to protect.

## Strategic Question 2 — npm `@oscarmackjr` scope precondition

**Answer: STILL UNRESOLVED as of this session (re-verified live, 2026-07-03/current date). This remains a hard, non-code, operator-only precondition.**

Re-verification performed this session (values identical to Phase 102's finding):
```bash
curl -s -A "Mozilla/5.0 (Windows NT 10.0; Win64; x64) research/1.0" -o /dev/null -w "%{http_code}\n" https://www.npmjs.com/~oscarmackjr        # -> 404
curl -s -A "Mozilla/5.0 (Windows NT 10.0; Win64; x64) research/1.0" -o /dev/null -w "%{http_code}\n" https://www.npmjs.com/org/oscarmackjr    # -> 404
curl -s "https://registry.npmjs.org/-/v1/search?text=scope:oscarmackjr&size=5"   # -> {"objects":[],"total":0,...}
npm whoami   # -> npm error code ENEEDAUTH / need auth (not logged in on this host at all)
```
`[VERIFIED: live HTTP + npm CLI checks this session]` — no npm user or org named `oscarmackjr` exists, and this host isn't even logged into npm yet, which is a second, separate precondition (distinct from scope ownership).

**Exact operator action items** (cannot be performed by the assistant — requires interactive account creation/auth):
1. Create an npm account (if the operator doesn't already have a personal npm account) via `npm adduser` or the npmjs.com signup flow, choosing/using a username, OR create an npm **Organization** named exactly `oscarmackjr` via `npm org create oscarmackjr` (requires an existing personal npm login first) or the npmjs.com "Add Organization" UI — a scoped package `@scope/name` can be owned by either a same-named user or an org.
2. Authenticate this (or the publishing) machine: `npm login` (interactive) or configure an `.npmrc` with an npm access token (`//registry.npmjs.org/:_authToken=...`) — currently absent on this host (`npm whoami` fails).
3. Because this will be the **first publish** of every package under the `@oscarmackjr` scope, every `npm publish` invocation (main package + all platform subpackages) **must** pass `--access public` explicitly — npm defaults new scoped packages to `restricted` (private, which requires a paid plan) unless told otherwise. `nono-ts`'s existing (upstream) `publish.yml` already does this correctly (`npm publish "$dir" --provenance --access public`), a pattern to preserve.
4. This is a `checkpoint:human-action` in the plan, not an autonomous task — flag it prominently as a Wave 0 blocker for the npm leg specifically (crates.io and PyPI have no equivalent scope-ownership precondition).

## Standard Stack

### Core (already present, verified this session)
| Tool | Version (this host) | Purpose | Confidence |
|------|---------------------|---------|------------|
| cargo | 1.95.0 | crates.io publish; blocks on index-visibility by default since 1.66 | VERIFIED |
| maturin | 1.14.1 | PyPI wheel/sdist build | VERIFIED (Phase 102 research + re-confirmed present) |
| napi (`@napi-rs/cli`) | 3.6.0 | npm native addon build + `napi artifacts`/`napi rename` | VERIFIED (Phase 102 research) |
| zig | 0.16.0 | Present via WinGet; enables `maturin build --zig` / `napi build -x` cross-compilation | VERIFIED this session |
| cargo-zigbuild | installed (`~/.cargo/bin/cargo-zigbuild`) | Cross-compilation backend for maturin/napi `--zig`/`-x` flags | VERIFIED this session |
| cross (Docker-based) | 29.5.3 | Docker-based cross-compilation for Linux targets (Phase 96 toolchain) | VERIFIED this session; Docker daemon confirmed running |

### Missing (must be installed/configured before the live legs — autonomous, non-live actions)
| Tool/Credential | Status | Action |
|------------------|--------|--------|
| `twine` | **NOT installed** (`twine: command not found`; `python -m twine` -> `No module named twine`) | `pip install twine` — safe, autonomous, no live-registry action |
| npm auth | **NOT logged in** (`npm whoami` -> `ENEEDAUTH`) | Operator must `npm login` or configure a token — this is itself part of Strategic Question 2's operator checklist |
| cargo registry token | No `~/.cargo/credentials.toml` found on this host | Operator must `cargo login <token>` (or supply `CARGO_REGISTRY_TOKEN` env var) before any live `cargo publish` |
| `OscarMackJr/nono-py`, `OscarMackJr/nono-ts` GitHub repos | **Do not exist** (`gh repo view` -> "Could not resolve to a Repository") | See Strategic Decision below — fork or accept local-only build scope |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled sparse-index poll loop as the PRIMARY crates.io wait mechanism | Rely on `cargo publish`'s built-in blocking (default since 1.66), add an explicit poll only as an observable/defensive corroboration | Hand-rolling the poll as primary duplicates already-solved cargo behavior and risks a subtly different timeout/backoff than cargo's own; the recommended design treats cargo's block as authoritative and the explicit poll as an audit-trail/SC1-wording-literal addition |
| Local-only cross-compilation for all 5 npm platforms + all PyPI wheel platforms | Fork `nono-py`/`nono-ts` to real `OscarMackJr/*` repos and reuse their existing (upstream-authored, currently misconfigured) GitHub Actions matrices | Forking reuses already-proven, already-authored CI (macOS/Linux runners, matrix structure) instead of experimental `--use-cross`/`--zig` local cross-compilation for artifact types (PyO3 extension modules, native Node addons) that were never validated for full builds on this host (only `cargo clippy` cross-target was proven in Phase 96, not full link+build) |
| PyPI OIDC "Trusted Publisher" (as `nono-py`'s own inherited CI already uses) | Classic API-token + `twine upload --skip-existing` (as PUB-02's literal wording specifies) | Trusted Publisher requires a PyPI-dashboard-side binding to a specific GitHub repo+workflow, which requires the fork repo to exist first; token+twine works today from the operator's local machine with zero new infrastructure — matches the phase's explicit instruction |

## Architecture Patterns

### Publish sequence (system flow)

```
 ┌─ Pre-flight (autonomous, repeatable) ───────────────────────────────────────┐
 │  1. Re-check name availability (crates.io ×3, PyPI, npm) — live HTTP        │
 │  2. Re-check npm scope ownership (~oscarmackjr, org/oscarmackjr) — 404 gate │
 │  3. Environment check: twine installed? npm/cargo logged in? repo exists?   │
 └───────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
 ┌─ crates.io leg (dependency order, this repo's own workspace) ──────────────┐
 │  For each crate in [nono-sandbox, nono-sandbox-proxy, nono-sandbox-cli]:    │
 │    a. Idempotency check: GET index.crates.io/{p1}/{p2}/{name}              │
 │       -> if "0.66.1" already present in the version list, SKIP publish     │
 │    b. cargo publish -p <crate> --allow-dirty --token $CARGO_REGISTRY_TOKEN │
 │       (blocks internally until index-visible — cargo >=1.66 default)       │
 │    c. Explicit corroborating poll: re-GET the same index URL, confirm      │
 │       "0.66.1" present, bounded timeout (e.g. 5 min / 10s interval)        │
 │    d. Only then proceed to the next dependent crate                       │
 └───────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
 ┌─ PyPI leg (../nono-py, LOCAL machine — no fork CI exists) ─────────────────┐
 │  1. maturin build --release              (Windows-native wheel, guaranteed)│
 │  2. maturin sdist                        (universal source dist, guaranteed)│
 │  3. [best-effort, dry-run first] maturin build --release --zig             │
 │       --target x86_64-unknown-linux-gnu  (needs zig; UNVERIFIED for this   │
 │       crate's actual PyO3 extension in this session — test before relying) │
 │  4. twine upload --skip-existing dist/*.whl dist/*.tar.gz                  │
 │  5. Verify: GET pypi.org/pypi/nono-sandbox/json -> enumerate               │
 │     releases["0.66.1"] filenames; confirm expected platform tags present   │
 └───────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
 ┌─ npm leg (../nono-ts, LOCAL machine + CI gap) ──────────────────────────────┐
 │  1. FIX FIRST: add a Windows build leg to ci.yml/publish.yml OR build      │
 │     win32-x64-msvc locally: napi build --platform --release               │
 │     (host-native on this Windows machine — guaranteed buildable)           │
 │  2. napi artifacts --output-dir <dir> --npm-dir npm  (populates npm/*/     │
 │     including the previously-missing npm/win32-x64-msvc/)                 │
 │  3. [best-effort] Linux leg via `napi build -x --use-cross                │
 │     --target x86_64-unknown-linux-gnu` (Docker cross, already installed;  │
 │     UNVERIFIED for full napi build in this session — test before relying) │
 │  4. macOS leg: NOT locally achievable — requires either a real Mac or a   │
 │     forked repo's GitHub-hosted macos-latest runner                       │
 │  5. For each existing npm/*/package.json: npm publish <dir> --access      │
 │     public  (idempotent per-version — npm rejects a re-publish of an      │
 │     existing version, safe to re-run)                                     │
 │  6. npm publish --access public   (main package, LAST — its               │
 │     optionalDependencies must already resolve to published versions)      │
 │  7. Verify: for each platform actually published, confirm via             │
 │     GET registry.npmjs.org/@oscarmackjr%2Fnono-ts-<platform> — 200        │
 └───────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
 ┌─ Post-publish resolve verification (SC4, side-effect-free) ────────────────┐
 │  cargo install nono-sandbox-cli --root <tmpdir>      ; rm -rf <tmpdir>     │
 │  python -m venv <tmpvenv>; <tmpvenv>/pip install nono-sandbox; rm -rf ...  │
 │  npm i @oscarmackjr/nono-ts --prefix <tmpdir>        ; rm -rf <tmpdir>     │
 └───────────────────────────────────────────────────────────────────────────┘
```

### Recommended project structure (new/changed files)

```
scripts/
├── release-dry-run.ps1                 # EXISTING — unchanged, still the safe default
├── release-publish-live.ps1            # NEW — the machinery this phase authors; every
│                                        #   live-publish action gated behind explicit
│                                        #   -Confirm/-Registry <crates|pypi|npm> flags,
│                                        #   never a bare "run everything live" default
├── crates-index-poll.ps1               # NEW — reusable helper: GET index.crates.io/
│                                        #   {p1}/{p2}/{name}, parse JSON-lines, check
│                                        #   for a specific "vers", bounded timeout
└── verify-post-publish-resolve.ps1     # NEW — SC4: isolated cargo install / pip install /
                                         #   npm i in temp dirs, cleans up after itself

.github/workflows/
└── release.yml                         # publish-crates job REWRITTEN (correct names +
                                         # dependency order + index-poll corroboration +
                                         # fixed idempotency check replacing `cargo search`);
                                         # if: false REMOVED only once the operator has
                                         # confirmed Phase 104's tag exists (see Pitfall 6)
```

### Anti-Patterns to Avoid

- **Re-enabling `publish-crates` unconditionally on every future `v*.*.*` tag push.** The job currently triggers via `needs: release` with no independent gate — since crates.io publishes are permanent, an accidental future tag push (e.g. a hotfix `v0.66.2` cut before the operator is ready to also publish it to all three registries) would silently attempt a live, irreversible crates.io publish as a side effect of an unrelated release cut. Recommend adding an explicit `workflow_dispatch`-only trigger (or a `vars`/`inputs.confirm_publish == 'true'` gate) rather than relying purely on the tag-push trigger, even after un-neutralizing.
- **Trusting `cargo search` for idempotency.** Already broken (deprecated by crates.io, confirmed empty output against `nono-sandbox` in Phase 102 research). Use `index.crates.io/{p1}/{p2}/{name}` or `crates.io/api/v1/crates/{name}` directly.
- **Treating the existing 4 `npm/*/` platform folders as "the platform set."** They reflect what upstream's CI has historically built (macOS ×2, Linux-gnu ×2) — NOT what this fork's `package.json` now declares (5 targets including Windows). Always diff `napi.targets`/`optionalDependencies` against the actual `npm/*/` directory listing before publishing.
- **Assuming `maturin build --zig`/`napi build -x --use-cross` "just works" because the underlying tools (zig, cross) are installed.** Both are explicitly marked `[experimental]` by their own CLIs and were only proven in this repo for `cargo clippy` (Phase 96), not for producing a fully linked PyO3 extension or full napi cdylib. Dry-run and inspect the actual artifact (load it, run a smoke import/require) before trusting it as a publishable platform wheel/addon.
- **Publishing the main npm package before its platform subpackages.** `optionalDependencies` pins exact versions; if the main package publishes first and a subpackage publish fails/is delayed, `npm install` on that platform will report an unmet optional dependency (usually silently ignored by npm as "optional," but the native binding then fails to load at runtime) — always publish platform subpackages first, main package last.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Waiting for a just-published crate to become resolvable by a dependent | A sleep-based or ad-hoc retry loop as the PRIMARY mechanism | `cargo publish`'s own built-in index-visibility blocking (default since Cargo 1.66) | Already solves exactly this problem, upstream-tested across the whole crates.io ecosystem; re-implementing it independently risks a subtly different (and less battle-tested) timeout/backoff |
| Moving built `.node` artifacts into per-platform npm package folders | Hand-copying files + hand-editing each `npm/*/package.json` | `napi artifacts --output-dir <dir> --npm-dir npm` | Official tool already used by this repo's own inherited `publish.yml`; matches file naming (`nono.<platform>.node`) and package.json shape exactly |
| Renaming/adding a new platform subpackage (Windows) | Manually authoring `npm/win32-x64-msvc/package.json` from scratch | `napi artifacts` after a successful `napi build --platform --release` on a Windows host (auto-generates the folder + package.json from `napi.targets`) | Avoids drift between `napi.targets`, `optionalDependencies`, and the actual folder structure — the exact class of bug already discovered in this session (Pitfall 1 below) |

**Key insight:** every genuinely hard part of this phase (index-visibility timing, multi-package npm publishing, platform-wheel packaging) already has an official, purpose-built solution — the phase's real work is *sequencing* those solutions correctly and *closing the infrastructure gaps* (missing repo forks, missing Windows CI leg, missing local credentials) that would silently make the official tools produce an incomplete result.

## Common Pitfalls

### Pitfall 1: The Windows npm platform package doesn't exist yet (reproduced live, not hypothetical)
**What goes wrong:** `npm install @oscarmackjr/nono-ts` on a Windows machine fails to load a native binding (no matching optionalDependency package exists to satisfy `win32-x64-msvc`).
**Why it happens:** `package.json`'s `napi.targets` and `optionalDependencies` already list `x86_64-pc-windows-msvc` / `@oscarmackjr/nono-ts-win32-x64-msvc`, but neither `ci.yml` nor `publish.yml`'s build matrix includes a Windows runner, so `npm/win32-x64-msvc/` was never generated. `[VERIFIED: directory listing + both workflow YAMLs read this session]`
**How to avoid:** Add a Windows build leg (this operator's own machine is Windows-native — no new infra needed for this one platform) before running `napi artifacts`, and verify all 5 declared targets have a corresponding `npm/*/` folder before any publish.
**Warning signs:** `optionalDependencies` entry count (5) not matching `ls npm/ | wc -l` (currently 4).

### Pitfall 2: Sibling repos have no fork-owned GitHub host (structural, not a code bug)
**What goes wrong:** Planning assumes `../nono-py`/`../nono-ts`'s own `publish.yml` can simply be "re-enabled," the way Phase 104 is re-enabling `release.yml`'s `publish-crates` job.
**Why it happens:** Those workflows live in repos whose `origin` remote is still the upstream org (`always-further`/`nolabs-ai`), confirmed via `git remote -v` and `gh repo view OscarMackJr/nono-py` failing with "Could not resolve to a Repository" this session. The fork has no push access, no Actions minutes, and no ability to configure secrets on those repos.
**How to avoid:** Treat "does this sibling repo need to be forked to GitHub first" as an explicit Wave 0 decision, not an assumption. If forked: budget for fixing `publish.yml`'s `if: github.repository == 'always-further/...'` guards, package-name/URL staleness (`nono-ts` on npmjs.com badge URL, `nono-py`'s PyPI project URL in the `environment:` block), and adding the missing Windows matrix leg. If not forked: scope PyPI/npm publishing to what this single Windows operator machine can build (guaranteed: Windows wheel + Windows napi addon + universal sdist; best-effort/experimental: Linux via zig/cross; not achievable: macOS).
**Warning signs:** A plan step that says "re-enable nono-py's publish.yml" without first confirming a fork-owned repo exists to run it on.

### Pitfall 3: `cargo search`-based idempotency check is silently broken
**What goes wrong:** The existing (neutralized) `release.yml` `publish-crates` job uses `cargo search nono --limit 1 | grep -q "\"$VERSION\""` to decide "already published, skip" — `cargo search` is deprecated by crates.io and returns empty output, so this check ALWAYS evaluates false, meaning the job would always attempt to re-publish even an already-published version (which crates.io then rejects with a hard error, breaking resumability).
**Why it happens:** `cargo search` querying crates.io has been effectively non-functional for some time (confirmed empirically in Phase 102 research, re-confirmed by this phase's own reasoning about the sparse-index/registry-API alternatives).
**How to avoid:** Replace the check with a direct query against `index.crates.io/{p1}/{p2}/{name}` (parse the newline-delimited JSON for a `"vers":"0.66.1"` entry) or `crates.io/api/v1/crates/{name}` (parse the `versions` array) — both empirically confirmed working this session.
**Warning signs:** A publish script whose "already published, skipping" log line never actually fires even on a genuine re-run.

### Pitfall 4: crates.io sparse-index path scheme gets the wrong prefix for these specific crate names
**What goes wrong:** Using the wrong index URL shape (e.g. treating `nono-sandbox` as a short name) returns a 404/error that gets misread as "crate doesn't exist" rather than "wrong URL shape."
**Why it happens:** The sparse index uses length-dependent prefixing: 1-char -> `1/{name}`, 2-char -> `2/{name}`, 3-char -> `3/{first-char}/{name}`, 4+ char -> `{name[0:2]}/{name[2:4]}/{name}`. All three publish-set names (`nono-sandbox`, `nono-sandbox-proxy`, `nono-sandbox-cli`) are 4+ characters, so the correct path is `no/no/{name}` for all three.
**How to avoid:** Empirically verified this session: `curl -A "<ua>" https://index.crates.io/no/no/nono-sandbox` correctly returns `404 NoSuchKey` (meaning: valid path shape, crate genuinely not yet published) — use this exact prefix scheme, not a fixed 2-segment or name-length-agnostic guess.
**Warning signs:** An index URL returning a CDN/S3-level 403 instead of the expected `NoSuchKey` 404 — usually a missing `User-Agent` header (crates.io API returns 403 without one; the sparse index CDN was tolerant of this in testing but a User-Agent should still always be sent as a courtesy/future-proofing measure).

### Pitfall 5: `--zig`/`--use-cross` cross-compilation flags are unproven for this project's actual build artifacts
**What goes wrong:** Assuming that because `zig`, `cargo-zigbuild`, and `cross` (Docker) are already installed and proven for `cargo clippy --target aarch64-unknown-linux-gnu`/`--target x86_64-apple-darwin` (Phase 96), the same tooling will transparently produce a working PyO3 extension module or napi cdylib for a different target.
**Why it happens:** Phase 96's cross-target toolchain was explicitly scoped to `clippy` (lint-only, no final link step) — CLAUDE.md itself documents this ("apple-darwin via the direct-binary cargo-zigbuild **clippy**"). Producing an actual linked, loadable extension module involves linking against the target's Python/Node ABI, which is a materially harder problem than lint-checking source.
**How to avoid:** Treat any `maturin build --zig --target ...` or `napi build -x --use-cross --target ...` invocation as an experimental leg requiring its own dry-run + artifact-load smoke test (e.g. actually `import`/`require` the built artifact, ideally inside a container matching the target) before it is trusted as a publishable wheel/addon — never assume success from a clean exit code alone, matching SC2's explicit "verified post-publish, not just exit code" wording.
**Warning signs:** A cross-compiled wheel/addon that builds successfully but produces an `ImportError`/`ERR_DLOPEN_FAILED` when actually loaded on the target platform.

### Pitfall 6: Re-enabling `publish-crates` unconditionally risks an accidental live publish on a future unrelated tag push
**What goes wrong:** Once the `if: false` guard is removed and correct names/logic are in place, the job runs automatically on every future `v*.*.*` tag push (the workflow's existing `on: push: tags:` trigger) — including tags cut for reasons unrelated to a deliberate "publish now" decision (e.g. a hotfix release before the operator is ready to also touch PyPI/npm).
**Why it happens:** The job's only current gate is `needs: release` (i.e., "did the build+sign+GitHub-release job succeed"), not an explicit human-confirmation gate.
**How to avoid:** Add an explicit opt-in gate — e.g. require `workflow_dispatch` with an `inputs.confirm_live_publish` string that must exactly equal a confirmation phrase, or split crates.io publishing into its own `workflow_dispatch`-only workflow decoupled from the tag-push-triggered `release.yml` entirely. Given the idempotency/index-poll logic makes re-running safe for already-published versions, the risk is specifically about a NEW, not-yet-decided-to-publish version being published as an unintended side effect.
**Warning signs:** A plan that "re-enables" the job by only removing `if: false` and fixing the crate names, without adding any new confirmation gate.

## Code Examples

### crates.io sparse-index idempotency + poll (verified path shape this session)
```bash
# 4+ char crate name prefix scheme: {name[0:2]}/{name[2:4]}/{name}
UA="nono-release/1.0 (oscar.mack.jr@gmail.com)"
NAME="nono-sandbox"
PREFIX="${NAME:0:2}/${NAME:2:4}"
URL="https://index.crates.io/${PREFIX}/${NAME}"

# Idempotency check (does 0.66.1 already exist?)
if curl -s -A "$UA" "$URL" | grep -q '"vers":"0.66.1"'; then
  echo "$NAME 0.66.1 already published — skipping"
else
  cargo publish -p "$NAME" --allow-dirty --token "$CARGO_REGISTRY_TOKEN"
  # cargo publish already blocks internally until index-visible (Cargo >=1.66,
  # this host runs 1.95.0) — the loop below is a defensive/audit-trail
  # corroboration, not the primary wait mechanism.
  for i in $(seq 1 30); do
    if curl -s -A "$UA" "$URL" | grep -q '"vers":"0.66.1"'; then
      echo "Confirmed indexed after $((i * 10))s"
      break
    fi
    sleep 10
  done
fi
```
Source: URL shape + `NoSuchKey` 404 response — VERIFIED live this session against `https://index.crates.io/no/no/nono-sandbox`. `cargo publish` blocking-by-default — [CITED: github.com/rust-lang/cargo PR #11062, #11356 — merged, shipped Cargo 1.66.0].

### PyPI post-publish coverage verification (not just exit code — SC2)
```bash
curl -s https://pypi.org/pypi/nono-sandbox/json | python -c "
import json,sys
d = json.load(sys.stdin)
files = d['releases'].get('0.66.1', [])
print([f['filename'] for f in files])
"
```
Source: standard PyPI JSON API shape — [CITED: warehouse (PyPI) JSON API docs, `releases.<version>` array of file dicts with `filename` key].

### npm platform-completeness check before publish (SC3)
```bash
cd ../nono-ts
node -e "
const pkg = require('./package.json');
const fs = require('fs');
const declared = Object.keys(pkg.optionalDependencies);
const present = fs.readdirSync('npm').map(d => '@oscarmackjr/nono-ts-' + d);
const missing = declared.filter(d => !present.includes(d));
if (missing.length) { console.error('MISSING platform packages:', missing); process.exit(1); }
console.log('All', declared.length, 'declared platform packages present locally.');
"
```
This exact check, run this session, reports `MISSING platform packages: [ '@oscarmackjr/nono-ts-win32-x64-msvc' ]` against the current tree — reproduces Pitfall 1 mechanically.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Manual `sleep 30` between dependent crate publishes (the current neutralized job's own logic) | `cargo publish` blocks internally until the version is index-visible by default | Cargo 1.66.0, Dec 2022 | The existing job's `sleep 30` calls are now redundant with cargo's own built-in behavior — safe to remove/replace with an explicit corroborating poll instead of a blind sleep |
| `cargo search <name>` for registry-availability/idempotency checks | Direct `index.crates.io/{p1}/{p2}/{name}` sparse-index query or `crates.io/api/v1/crates/{name}` | crates.io policy change (pre-dates this milestone; re-confirmed empty output this session) | Any idempotency check in the rewritten `publish-crates` job MUST use one of these, not `cargo search` |

**Deprecated/outdated:** `cargo search` for any registry-state check (confirmed non-functional against `nono-sandbox` this session, consistent with Phase 102's finding).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `maturin build --zig --target x86_64-unknown-linux-gnu` will successfully produce a loadable Linux wheel for this specific PyO3 crate (not just a clean exit code) | Architecture Patterns (PyPI leg), Pitfall 5 | MEDIUM — if wrong, the PyPI publish falls back to Windows-wheel + sdist only; SC2's wording ("platform coverage verified post-publish") tolerates a smaller, honestly-documented coverage, so this is not a hard blocker, just a scope-reduction risk |
| A2 | `napi build -x --use-cross --target x86_64-unknown-linux-gnu` will successfully produce a loadable Linux `.node` addon for this specific napi-rs crate | Architecture Patterns (npm leg), Pitfall 5 | MEDIUM — same reasoning as A1; falls back to Windows-only npm platform coverage if unproven, which itself must then be honestly documented as a known gap rather than silently shipped incomplete |
| A3 | Creating real `OscarMackJr/nono-py` and `OscarMackJr/nono-ts` GitHub repos (forking) is in-scope for Phase 105 rather than a separate future phase | Summary, Pitfall 2 | MEDIUM-HIGH — this is the single biggest scope question for the planner; if descoped, PyPI/npm publishing in this phase is necessarily Windows-machine-limited (+ best-effort Linux), and full multi-platform parity becomes a deferred follow-up, which should be explicitly logged as a new FUT-item rather than silently assumed complete |
| A4 | The existing `CARGO_REGISTRY_TOKEN` GitHub Actions secret (referenced in the neutralized `publish-crates` job) is already configured on the `OscarMackJr/nono` repo from a prior phase | Environment Availability, Architecture Patterns | LOW-MEDIUM — cannot be verified without repo-admin API access in this session; if absent, the operator must create it before any CI-driven crates.io publish (irrelevant to the local-script path, which uses the operator's own `cargo login`/env var instead) |

**If this table is empty:** N/A — see above.

## Open Questions (RESOLVED during planning)

**RESOLVED (2026-07-03, via AskUserQuestion → 105-CONTEXT.md):** Q1 → RESOLVED by D-02 (scope to LOCALLY-BUILDABLE platforms only; forking `OscarMackJr/nono-py`/`nono-ts` for full multi-platform CI is DEFERRED; reduced Linux/macOS coverage documented as a known limitation). Q2 → RESOLVED by D-03 (the `publish-crates` job becomes a `workflow_dispatch`-only decoupled workflow, never auto on tag-push).

1. **RESOLVED (D-02) — Should Phase 105 create real fork-owned `OscarMackJr/nono-py`/`OscarMackJr/nono-ts` GitHub repos, or explicitly scope this phase's npm/PyPI publish to what's locally buildable on this one Windows operator machine?**
   - What we know: no such repos exist today (verified); the sibling repos' own inherited CI would solve multi-platform coverage cleanly and reuses proven patterns, but repo creation + CI repair is nontrivial, unbudgeted work not implied by PUB-01/PUB-02's wording.
   - What's unclear: whether "publish `0.66.1` live to ... npm ... all platform-specific native packages present" (PUB-02) is meant to require full 5-platform parity in THIS phase, or whether a documented, reduced Windows(+best-effort-Linux) coverage satisfies the requirement's intent given the infrastructure gap discovered.
   - Recommendation: raise this explicitly to the user/planner as a scoping decision before task authoring — do not silently pick either path. If deferred, log a new `FUT-XX` entry for full multi-platform parity, matching this project's existing pattern (see REQUIREMENTS.md `FUT-04`..`FUT-07`).

2. **RESOLVED (D-03) — Should the rewritten `publish-crates` CI job be re-enabled to run automatically on tag push, or converted to a `workflow_dispatch`-only, separately-triggered workflow?**
   - What we know: crates.io publishes are permanent; the existing trigger (`needs: release`, itself triggered by `on: push: tags: 'v*.*.*'`) offers no independent human-confirmation gate once un-neutralized.
   - What's unclear: whether decoupling it into its own `workflow_dispatch`-only workflow is worth the added file/complexity versus simply requiring a `workflow_dispatch` input on the existing job (keeping it in `release.yml`).
   - Recommendation: favor the `workflow_dispatch`-only approach (matching `nono-ts`'s own inherited `publish.yml` pattern, which already uses an `inputs.publish_target` choice of `dry-run`/`npm` as its live-vs-dry gate) — it is a smaller, proven pattern already present elsewhere in this project's dependency tree.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo | crates.io publish leg | ✓ | 1.95.0 | — |
| maturin | PyPI build leg | ✓ | 1.14.1 | — |
| twine | PyPI upload leg | ✗ | — | `pip install twine` (autonomous, safe) |
| napi CLI | npm build leg | ✓ | 3.6.0 | — |
| zig / cargo-zigbuild | Experimental cross-compile legs | ✓ | zig 0.16.0 | Skip cross-compile legs, document reduced coverage |
| Docker (`cross`) | Experimental Linux cross-compile leg | ✓ (daemon running) | cross 29.5.3 | Same as above |
| npm auth | npm publish leg | ✗ | — | Operator `npm login`/token — **blocking, no fallback** |
| cargo registry credentials | crates.io publish leg | ✗ | — | Operator `cargo login`/`CARGO_REGISTRY_TOKEN` — **blocking, no fallback** |
| `@oscarmackjr` npm scope ownership | npm publish leg (first-ever scoped publish) | ✗ | — | Operator creates npm user/org `oscarmackjr` — **blocking, no fallback, no code workaround** |
| `OscarMackJr/nono-py`/`nono-ts` GitHub repos | Multi-platform CI-driven PyPI/npm builds | ✗ | — | Local-machine-only build scope (Windows + best-effort Linux); see Open Question 1 |

**Missing dependencies with no fallback:** npm authentication, cargo registry credentials, `@oscarmackjr` scope ownership — all three are operator-only actions with zero code-side workaround, and must be resolved before any live-publish checkpoint task can execute (not before the autonomous authoring/dry-run tasks).

**Missing dependencies with fallback:** `twine` (pip-installable autonomously); cross-compilation tooling absence would just reduce documented platform coverage, not block the phase.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None applicable — this phase is publish-pipeline tooling, not application logic. Verification is dry-run-green + live registry-state checks, mirroring the phase's own ROADMAP success criteria. |
| Config file | N/A |
| Quick run command | `pwsh -File scripts/release-dry-run.ps1` (existing, safe, no live action) |
| Full suite command | `pwsh -File scripts/release-dry-run.ps1` + the new index-poll/platform-completeness checks below, all runnable with zero registry credentials |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|--------------------|-------------|
| PUB-02 SC1 (crates.io order+poll) | Dependency-order publish with index-visibility confirmation | integration (network, read-only pre-live) | `curl -A "<ua>" https://index.crates.io/no/no/nono-sandbox` (×3 names) — dry-run-testable today against the "not yet published" 404 state | ✅ — verified this session |
| PUB-02 SC2 (PyPI coverage) | maturin build + twine check/upload; post-publish coverage verified | build + integration | `maturin build` (host-native, already proven Phase 102); `curl pypi.org/pypi/nono-sandbox/json` (verifiable shape today against a 404-not-yet-published state) | ⚠️ — `twine` itself needs installing first (Wave 0 gap) |
| PUB-02 SC3 (npm platform completeness) | All declared platform packages present + published | manifest/build | The Node.js one-liner in Code Examples (already run this session — reports the missing win32-x64-msvc gap concretely) | ✅ — verified this session, currently FAILS (expected — this is the gap the phase must close) |
| PUB-02 SC4 (post-publish resolve) | `cargo install`/`pip install`/`npm i` all resolve | integration (isolated temp env) | `cargo install nono-sandbox-cli --root <tmp>`; `pip install nono-sandbox` in a fresh venv; `npm i @oscarmackjr/nono-ts --prefix <tmp>` — all runnable post-live-publish only (cannot be dry-run meaningfully before the package exists) | ❌ Wave 0 — needs a small wrapper script (`verify-post-publish-resolve.ps1`) that isolates + cleans up each check |

### Sampling Rate
- **Per task commit (authoring the machinery):** `pwsh -File scripts/release-dry-run.ps1` after every script/CI change — must stay green (crates.io leg) and produce the expected `PRE_PUBLISH_REGISTRY_BLOCKED`/SKIP states for the still-not-live PyPI/npm legs.
- **Per wave merge:** the platform-completeness Node.js check (Code Examples) + a fresh live re-check of all name-availability/scope-ownership endpoints (per Phase 102's own Security Domain precedent — a name can be squatted between research and execution).
- **Phase gate (before any live-publish checkpoint is even offered to the operator):** all of the above green, PLUS explicit confirmation that Phase 104's tag `v0.66.1` exists and is Verified-publisher (per Strategic Question 1's recommended sequencing), PLUS the npm scope-ownership + auth + cargo credentials operator checklist confirmed complete.

### Wave 0 Gaps
- [ ] `pip install twine` — currently absent on this host, blocks the PyPI upload leg's dry-run-adjacent testing (twine check already works via `release-dry-run.ps1`'s existing SKIP path, but live `twine upload --skip-existing` needs the real tool)
- [ ] `scripts/crates-index-poll.ps1` — new reusable helper (sparse-index query + bounded poll), no existing equivalent in this repo
- [ ] `scripts/verify-post-publish-resolve.ps1` — new SC4 wrapper (isolated temp-dir installs + cleanup), no existing equivalent
- [ ] Windows npm platform build/publish leg — `npm/win32-x64-msvc/` folder + CI matrix leg, concretely missing today
- [ ] Decision + (if chosen) execution of forking `nono-py`/`nono-ts` to `OscarMackJr/*` — see Open Question 1

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | Yes (narrowly) | Registry credentials (`CARGO_REGISTRY_TOKEN`, npm token, PyPI token) must never be logged/echoed by any new script; existing `release.yml` job already threads the cargo token via `env:`/`secrets:` correctly — replicate that pattern, never inline a token into a command string that could appear in a log |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A — no new access-control surface; registry permissions are entirely external (npm org membership, crates.io ownership) |
| V5 Input Validation | No (narrowly) | Version-string parsing from index/API JSON responses should use a real JSON parser (not regex-only) to avoid a crafted/partial response producing a false "already published" or "not yet published" verdict — low severity (read-only registry data, not user input) but still worth doing correctly |
| V6 Cryptography | No | N/A — no crypto touched by this phase (Trusted Signing is Phase 101/104's domain, not this one) |

### Known Threat Patterns for this phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| Name-squatting between research and live publish — a third party claims `nono-sandbox`/`nono-sandbox-proxy`/`nono-sandbox-cli`/`nono-sandbox` (PyPI)/`@oscarmackjr/nono-ts` on the public registry before this phase's operator checkpoint executes | Spoofing | Re-run the live availability checks (Code Examples / re-verification commands in Strategic Question 2) immediately before EVERY live-publish checkpoint, not just once during research — this phase's own dependency-ordered/index-polled design naturally re-surfaces an "already exists and isn't ours" failure early for crates.io; PyPI/npm need the same explicit re-check as a task precondition |
| Accidental live publish of an unintended version via an unconfirmed CI trigger | Tampering / Elevation of scope | Pitfall 6's recommended `workflow_dispatch`-only (or explicit confirmation-input) gate, decoupled from the tag-push trigger that Phase 104 already owns |
| Publishing the main npm package before all platform subpackages succeed, leaving a broken optionalDependency reference live and installable | Denial of Service (availability, for end users) | Strict publish ordering (platform packages first, main package last) + the platform-completeness check (Code Examples) as a hard pre-publish gate, not just an informational log line |
| Leaking a registry token via a script that echoes its command line (including the token) into CI/local logs | Information Disclosure | Thread tokens exclusively via environment variables consumed by the tool's own token-flag mechanism (`cargo publish --token "$VAR"` already does this correctly in the existing job — never `echo`/print the token, never embed it in a URL) |

## Sources

### Primary (HIGH confidence)
- Empirical `cargo publish --dry-run -p nono-sandbox` run this session against the real repo — packaged, compiled, and correctly aborted only at upload (dry-run).
- Live HTTP checks this session against `crates.io/api/v1/crates/*`, `index.crates.io/no/no/*`, `pypi.org/pypi/nono-sandbox/json`, `registry.npmjs.org/@oscarmackjr%2Fnono-ts`, `www.npmjs.com/~oscarmackjr`, `www.npmjs.com/org/oscarmackjr`, `registry.npmjs.org/-/v1/search?text=scope:oscarmackjr`.
- Direct reads of `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `scripts/release-dry-run.ps1`, `.github/workflows/release.yml` (publish-crates job + trigger config), `.planning/phases/102-fork-owned-package-rename/102-RESEARCH.md`, `.planning/phases/104-smoke-green-cut-the-trusted-signed-release/104-03-PLAN.md`, `../nono-py/{pyproject.toml,Cargo.toml,.github/workflows/publish.yml}`, `../nono-ts/{package.json,.github/workflows/{ci,publish,release}.yml,npm/*/package.json}`.
- `git remote -v` + `gh repo view` for `OscarMackJr/nono`, `OscarMackJr/nono-py`, `OscarMackJr/nono-ts` this session (confirms the fork-repo-existence gap).
- [CITED: github.com/rust-lang/cargo PR #11062 "Block until it is in index"] and [CITED: github.com/rust-lang/cargo PR #11356 "Fix wait-for-publish with sparse registry"] — official, merged, shipped in Cargo 1.66.0 (Dec 2022) — via WebFetch of the PR content this session.
- Local tool version checks this session (`cargo --version`, `maturin --version`, `npx napi --version`, `zig version`, `cargo-zigbuild` presence, `cross --version`, `docker info`, `wsl --list`, `rustup target list --installed`, `twine`/`npm whoami`/cargo credentials absence).

### Secondary (MEDIUM confidence)
- WebSearch summary corroborating the cargo blocking-publish history (multiple GitHub issue/PR titles consistent with the PR #11062 WebFetch content) — [github.com/rust-lang/cargo/issues/10297](https://github.com/rust-lang/cargo/issues/10297), [github.com/rust-lang/cargo/issues/11314](https://github.com/rust-lang/cargo/issues/11314), [rust-lang.github.io/rfcs/2789-sparse-index.html](https://rust-lang.github.io/rfcs/2789-sparse-index.html).

### Tertiary (LOW confidence)
- Whether `maturin build --zig`/`napi build -x --use-cross` will succeed for THIS project's actual PyO3/napi crates — not tested in this session (would require actually running a full cross-compile, time-boxed out of research scope); explicitly flagged as Assumptions A1/A2 and Pitfall 5, not asserted as fact.

## Metadata

**Confidence breakdown:**
- crates.io mechanics (index shape, publish-blocking-by-default, idempotency-check redesign): HIGH — empirically verified live against the real registry plus official, merged upstream cargo PR history.
- Sibling-repo hosting gap + npm scope-ownership gap (both strategic questions): HIGH — directly verified via `git remote`, `gh repo view`, and live HTTP/`npm whoami` checks this session, not inferred.
- PyPI/npm multi-platform cross-compilation feasibility: MEDIUM — tooling presence verified, but actual success for this project's specific build artifacts is unverified (explicitly logged as Assumptions A1/A2, not overstated as fact).
- Architecture/sequencing recommendations: HIGH — derived directly from reading the actual manifests, workflows, and registry state, cross-referenced against official Cargo/PyPI/npm documentation and this project's own prior-phase precedents.

**Research date:** 2026-07-03
**Valid until:** 3-7 days (registry name availability, npm scope ownership, and the "does Phase 104's tag exist yet" state are all live, time-sensitive facts — re-verify every item in this file immediately before any live-publish operator checkpoint, not just once during planning)
