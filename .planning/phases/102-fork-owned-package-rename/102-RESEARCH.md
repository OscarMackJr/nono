# Phase 102: Fork-Owned Package Rename - Research

**Researched:** 2026-07-02
**Domain:** Cargo workspace manifest mechanics (package-name vs. lib-name decoupling), PyPI/maturin packaging, npm/napi-rs scoped packaging, cross-registry name-availability verification
**Confidence:** HIGH (Cargo mechanism empirically verified on this machine; registry availability empirically verified live; napi/maturin claims corroborated by official docs)

## Summary

PUB-01 is a **pure identity rename**: three already-published-nowhere crates get new `[package] name` values, two sibling binding repos get new distribution names, and the workspace's own internal Rust code (`use nono::`, `use nono_proxy::`) must not change one character. The naive approach — renaming the dependency table key to match the new package name — silently produces `error[E0433]: cannot find module or crate` unless paired with Cargo's `package = "..."` rename key. This was empirically verified in this research session (see Standard Stack) and is the single most important fact the planner needs: **the fix is `package = "<new-name>"` added to each existing `nono = {...}` / `nono-proxy = {...}` dependency entry, with the dependency table KEY left as `nono`/`nono-proxy` unchanged.** This requires zero edits to any `use nono::` statement, zero edits to feature-forwarding strings (`"nono/system-keyring"`), and zero new `[lib]` section in `crates/nono/Cargo.toml`.

Critically, this rename is **not confined to this repo**. Two sibling repos (`../nono-py`, `../nono-ts`) declare their OWN `nono = { path = "../Nono/crates/nono" }` / `nono-proxy = { path = "../Nono/crates/nono-proxy" }` dependencies pointing at this workspace by relative path — outside Cargo's workspace-member mechanism entirely. The moment `crates/nono/Cargo.toml`'s `[package] name` changes, BOTH sibling repos' builds break (`no matching package found`) until their Cargo.toml dependency stanzas are patched with the same `package = "nono-sandbox"` fix. This is a hard cross-repo dependency of PUB-01's own success criterion 4 ("both binding builds are green") — the plan must touch all three repos in the same phase, not just the primary workspace.

On the Python/npm side, the two binding manifests are simpler: only `pyproject.toml [project] name` (PyPI distribution identity) and `package.json name` (npm identity) change. The importable Python module stays `nono_py` (not `nono` as the phase objective loosely implies — the actual importable name in this repo has always been `nono_py`, confirmed by reading `python/nono_py/__init__.py`; this is a correction to record for the planner) and the compiled `.node` binary prefix stays `nono` (`napi.binaryName`) if the official `napi rename --package-name` tool is used with `-n`/`-b` left unset.

All three new registry names were live-checked against the real registries in this session (crates.io API with a User-Agent header, PyPI JSON endpoint, npm registry) and are confirmed available as of 2026-07-02. One operator-relevant gap surfaced: the npm scope `@oscarmackjr` requires an npm account (or org) named exactly `oscarmackjr` to exist before Phase 105 can publish under it — no such npm user/org profile currently exists (`www.npmjs.com/~oscarmackjr` returns 404). This does not block Phase 102 (naming/availability only) but is a hard precondition for Phase 105 and should be flagged now.

**Primary recommendation:** Use `package = "<new-name>"` renaming (not dependency-key renaming, not new `[lib]` sections) for all three workspace crates; use the official `napi rename --package-name @oscarmackjr/nono-ts` command for nono-ts (leaving `--name`/`--binary-name` unset); hand-edit `pyproject.toml [project] name` for nono-py; patch the two sibling repos' own Cargo.toml dependency stanzas with matching `package =` keys; regenerate all three affected Cargo.lock files via `cargo build`/`cargo check`.

## Architectural Responsibility Map

This phase has no browser/API/DB tiers — its "tiers" are packaging/registry layers. Mapped for planner sanity-checking:

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| crates.io registry identity (`[package] name`) | Cargo workspace manifests (this repo) | crates.io registry (external, read-only in this phase) | Manifest edit is local; registry only consulted for availability, never written to in Phase 102 (prepare-only) |
| Internal Rust import surface (`use nono::`) | Cargo dependency-table `package =` key | — | Must NOT change; owned entirely by how dependents declare the dependency, not by the renamed crate itself |
| PyPI distribution identity | `../nono-py/pyproject.toml [project]` | maturin build backend | `[tool.maturin] module-name` is a separate, unrelated identity — must NOT change |
| Python import surface (`import nono_py`) | `../nono-py/python/nono_py/` package dir + `[tool.maturin] module-name` | — | Unaffected by PUB-01; already independent of the Rust crate name and the PyPI name |
| npm package identity | `../nono-ts/package.json` + `../nono-ts/npm/*/package.json` | `@napi-rs/cli` (`napi rename`) | Renaming tool exists precisely for this; hand-editing risks missing the `optionalDependencies` cross-references |
| `.node` binary artifact naming | `napi.binaryName` in `../nono-ts/package.json` | — | Must NOT change (parallels `[[bin]] name` invariant); leave `--binary-name` unset when running `napi rename` |
| Cross-repo path-dependency resolution | Both sibling repos' own `Cargo.toml` (`../nono-py/Cargo.toml`, `../nono-ts/Cargo.toml`) | This repo's renamed `[package] name` | Sibling repos are OUTSIDE this Cargo workspace — renaming here has zero automatic propagation; each sibling's dependency stanza needs its own `package =` fix |
| Cargo.lock regeneration | `cargo build`/`cargo check --workspace` (this repo) + each sibling repo's own lockfile | — | Three separate lockfiles must each be regenerated after their respective manifest edits |

## Standard Stack

### Core mechanism (verified empirically this session)

| Mechanism | Verified Behavior | Confidence |
|-----------|-------------------|------------|
| Cargo `[dependencies] key = { path = "...", package = "actual-name" }` | The extern crate name used in the CONSUMER's `use` statements equals the dependency table KEY (`key`), regardless of the producer's `[package] name` or `[lib] name`. Empirically confirmed: producer package renamed, consumer kept key `nono`/`prod` + added `package = "..."`, `use nono::`/`use prod::` compiled unchanged. | **VERIFIED** (empirical `cargo build` on this host, isolated scratch workspace, see Sources) |
| Cargo `[dependencies] key = { path = "..." }` with NO `package =`, where key ≠ actual package name | Hard error: `error: no matching package found / searched package name: 'key' / perhaps you meant: <actual-name>`. Cargo REQUIRES the table key to equal the resolved package name unless `package =` is present. | **VERIFIED** (empirical) |
| Cargo `[dependencies] key = { path = "...", package = "actual-name" }` where the PRODUCER's `[lib] name` is left at its default (no explicit `[lib]` section) | Still works — the producer's `[lib] name` is irrelevant once `package =` is used; the extern name is 100% controlled by the consumer's key. | **VERIFIED** (empirical) |
| Feature-forwarding syntax `"dep-key/feature-name"` (e.g. `system-keyring = ["dep:keyring", "nono/system-keyring", "nono-proxy/system-keyring"]`) after a `package =` rename | Continues to reference the dependency table KEY, not the package name — [CITED: doc.rust-lang.org/cargo/reference/specifying-dependencies.html] "names of features take after the name of the dependency, not the package name, when renamed." Not independently re-verified with a feature-flag scratch test in this session (time-boxed), but directly stated by the official doc and consistent with the empirical key-based behavior above. | CITED (HIGH) |
| Cargo.lock effect | `[[package]] name = "..."` entries and their cross-references are rewritten automatically on the next `cargo build`/`cargo check`/`cargo generate-lockfile` after a manifest rename — no manual Cargo.lock editing required. Path-dependency (workspace-member) lockfile entries carry no `source`/checksum field, so there is no checksum-mismatch risk from the rename itself. | CITED (Cargo Book, general lockfile behavior) — not independently re-verified against this specific lockfile in this session |

### Applying the mechanism to this workspace

| File | Current | Change |
|------|---------|--------|
| `crates/nono/Cargo.toml` | `[package] name = "nono"` | `[package] name = "nono-sandbox"` — **no other change**; no new `[lib]` section needed |
| `crates/nono-proxy/Cargo.toml` | `[package] name = "nono-proxy"`, `nono = { version = "0.66.1", path = "../nono", default-features = false }` | `[package] name = "nono-sandbox-proxy"`; dependency becomes `nono = { version = "0.66.1", path = "../nono", package = "nono-sandbox", default-features = false }` (key unchanged, `package =` added) |
| `crates/nono-cli/Cargo.toml` | `[package] name = "nono-cli"`; `nono = {...}`, `nono-proxy = {...}` deps; `[[bin]] name = "nono"` (×2 bins: `nono`, `nono-agentd`) | `[package] name = "nono-sandbox-cli"`; both dependency entries get `package = "nono-sandbox"` / `package = "nono-sandbox-proxy"` added (keys `nono`/`nono-proxy` unchanged); `[[bin]]` sections **untouched** |
| `crates/nono-shell-broker/Cargo.toml` | `publish = false`; `nono = { version = "0.66.1", path = "../nono" }` | NOT part of publish set (see Publish-Set Boundary below) — but its `nono` dependency still needs `package = "nono-sandbox"` added, or it will fail to build the moment `crates/nono`'s package name changes (it is compiled by `make build`/`cargo build --workspace`, publish-status is irrelevant to whether it needs to resolve the path dep) |
| `bindings/c/Cargo.toml` (`nono-ffi`) | `publish = false`; `nono = { version = "0.66.1", path = "../../crates/nono", default-features = false }` | Same as above — not in the publish set, but its `nono` dependency needs `package = "nono-sandbox"` added or `cargo build --workspace` breaks |
| `crates/nono-cli/Cargo.toml` windows dev-dep on `nono-shell-broker` | `nono-shell-broker = { path = "../nono-shell-broker", version = "0.66.1" }` | Untouched — `nono-shell-broker`'s OWN package name is not being renamed (only nono/nono-proxy/nono-cli are) |
| `Cargo.lock` (workspace root) | Contains `nono`, `nono-proxy`, `nono-cli` package entries | Regenerated via `cargo build --workspace --all-targets` (or `cargo check --workspace`) after all manifest edits — expect exactly 3 renamed `[[package]] name` hunks + their cross-reference updates, zero third-party drift |

**Installation:** No new packages installed — this phase edits existing manifests only.

**Version verification:** N/A (no new dependency versions introduced; `version = "0.66.1"` fields are preserved as-is, referring to the crate's own existing version, now under its new package identity).

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `package =` rename (recommended) | Rename dependency table key to match new package name (`nono-sandbox = { path = "../nono" }`) + add explicit `[lib] name = "nono"` in the producer | Requires editing EVERY consumer's key (bigger diff across 5 dependent manifests) AND requires the producer to carry an explicit `[lib]` section it doesn't have today. Functionally equivalent once done correctly, but larger surface area for a paste-error, and the phrase "internal imports untouched" is satisfied either way — `package =` is strictly less invasive. |
| Renaming table key + relying on default `[lib]` name derived from new package name | (not viable) | Empirically proven to break: default lib name would become `nono_sandbox`, silently changing every `use nono::` to a compile error across 5 consumer crates + 2 sibling repos. Rejected. |

## Package Legitimacy Audit

**Not applicable.** This phase installs zero new external packages — it renames the `[package]`/`[project]`/`package.json` identity of first-party crates/packages that already exist in this repo and its two sibling repos. No `cargo add`, `pip install`, or `npm install` of any new third-party dependency occurs. The slopcheck/registry-verification gate is for NEW package adoption and does not apply here.

## Architecture Patterns

### System flow: name change propagation

```
crates/nono/Cargo.toml               crates/nono-proxy/Cargo.toml         crates/nono-cli/Cargo.toml
[package] name: nono → nono-sandbox  [package] name: nono-proxy →         [package] name: nono-cli →
        │                             nono-sandbox-proxy                  nono-sandbox-cli
        │                                    │                                   │
        │  (path dep, package= added)        │  (path dep, package= added)      │
        ▼                                    ▼                                   │
nono-proxy's `nono = {..., package =    nono-cli's `nono-proxy = {...,           │
"nono-sandbox"}` ──────────────────────────► package = "nono-sandbox-proxy"}`◄───┘
        │                                    │
        ▼                                    ▼
nono-cli's `nono = {..., package = "nono-sandbox"}`
        │
        ├──► crates/nono-shell-broker/Cargo.toml `nono = {..., package = "nono-sandbox"}` (publish=false, still must resolve)
        ├──► bindings/c/Cargo.toml `nono = {..., package = "nono-sandbox"}` (publish=false, still must resolve)
        │
        └──► OUTSIDE this workspace (relative-path siblings, NOT workspace members):
             ../nono-py/Cargo.toml  `nono = {path="../Nono/crates/nono", package="nono-sandbox"}`
                                    `nono-proxy = {path="../Nono/crates/nono-proxy", package="nono-sandbox-proxy"}`
             ../nono-ts/Cargo.toml `nono = {path="../Nono/crates/nono", package="nono-sandbox", version="0.66"}`

Independently, in the two binding repos, the REGISTRY-FACING identity changes with NO Cargo-level coupling:
../nono-py/pyproject.toml  [project] name: nono-py → nono-sandbox         (PyPI identity only)
../nono-ts/package.json     name: nono-ts → @oscarmackjr/nono-ts          (npm identity only, via `napi rename --package-name`)
                             + npm/*/package.json subpackage names follow (napi rename handles this)
                             + optionalDependencies in main package.json follow (napi rename handles this)
```

### Recommended edit sequence (dependency order matters for a clean single-pass build)

1. `crates/nono/Cargo.toml`: `[package] name = "nono-sandbox"` only.
2. `crates/nono-proxy/Cargo.toml`: `[package] name = "nono-sandbox-proxy"`; patch its `nono` dependency to add `package = "nono-sandbox"`.
3. `crates/nono-cli/Cargo.toml`: `[package] name = "nono-sandbox-cli"`; patch both `nono` and `nono-proxy` dependency entries with `package = "nono-sandbox"` / `package = "nono-sandbox-proxy"`.
4. `crates/nono-shell-broker/Cargo.toml` and `bindings/c/Cargo.toml`: patch their `nono` dependency entries with `package = "nono-sandbox"` (their OWN `[package] name` is unchanged — they are outside the publish set).
5. `cargo build --workspace --all-targets` (or at minimum `cargo check --workspace`) to regenerate `Cargo.lock` and prove the workspace compiles under the new names.
6. `../nono-py/Cargo.toml`: patch `nono`/`nono-proxy` dependency entries the same way; `../nono-py/pyproject.toml`: `[project] name = "nono-sandbox"`; `maturin build` (or `maturin develop`) to prove green and regenerate `../nono-py/Cargo.lock`.
7. `../nono-ts/Cargo.toml`: patch its `nono` dependency entry the same way; run `napi rename --package-name @oscarmackjr/nono-ts` from the `nono-ts` repo root (updates `package.json`, all `npm/*/package.json` subpackages, and the `optionalDependencies` block in one step); `npm run build` (`napi build --platform --release`) to prove green and regenerate `../nono-ts/Cargo.lock`.

### Anti-Patterns to Avoid

- **Renaming the dependency table key instead of using `package =`:** silently breaks every `use nono::`/`use nono_proxy::` statement across 7 files/crates (5 in this workspace, 2 sibling repos) with a compile error, not a warning. Always prefer `package =` over a key rename for this specific "keep imports stable" requirement.
- **Forgetting `nono-shell-broker`/`nono-ffi` because they're `publish = false`:** publish status is irrelevant to whether `cargo build --workspace` needs to resolve their `nono` path dependency. Both will fail to build (not just fail to publish) if their dependency entry isn't patched.
- **Forgetting the two sibling repos entirely:** they are NOT workspace members (`../nono-py`, `../nono-ts` are separate git repos with their own `Cargo.toml`/`Cargo.lock`, referencing this repo only via relative path). `cargo build --workspace` in THIS repo will never surface their breakage — only `maturin build` / `napi build`, run explicitly in those repos, will.
- **Hand-editing `nono-ts`'s `npm/*/package.json` files and `optionalDependencies` instead of using `napi rename`:** four platform subpackages already exist (`darwin-arm64`, `darwin-x64`, `linux-arm64-gnu`, `linux-x64-gnu`) plus one referenced-but-never-generated (`win32-x64-msvc`, present in `optionalDependencies` and `napi.targets` but missing its own `npm/win32-x64-msvc/` directory — a pre-existing gap, not introduced by this phase). Manual editing risks missing a cross-reference; the official tool is purpose-built for this.
- **Changing `[tool.maturin] module-name` or the `python/nono_py/` directory name:** PUB-01 only requires the PyPI *distribution* name change; the importable module has always been `nono_py`, is unrelated to the distribution name, and changing it would be a genuine breaking change to every existing `import nono_py` user — explicitly out of scope.
- **Changing `napi.binaryName` or passing `--binary-name` to `napi rename`:** this would rename the compiled `.node` artifact prefix (currently `nono`), which has no analog requirement in PUB-01 (the requirement only names PyPI/npm/crates.io identities, not binary artifact names) — leave it unset.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Renaming an npm package + its napi-rs platform subpackages + `optionalDependencies` cross-references | A shell script / manual multi-file sed | `napi rename --package-name @oscarmackjr/nono-ts` (already installed: `@napi-rs/cli` 3.6.0 present in `node_modules`) | Official tool exists specifically for this; hand-rolling risks missing one of the 4 (soon 5) platform subpackage files or leaving a stale `optionalDependencies` version/name pair |
| Verifying registry name availability | Assuming a name is free because it "sounds unused" | `curl -A "<ua>" https://crates.io/api/v1/crates/<name>` (needs a User-Agent — crates.io returns `403` without one, which can be misread as "taken"), `curl https://pypi.org/pypi/<name>/json` (404 = available), `curl https://registry.npmjs.org/<name-or-%40scope%2Fname>` (404 = available) | Both crates.io and PyPI/npm publish would fail loudly if actually taken, but the *research* value is confirming BEFORE committing to the rename across 3 manifests + 2 repos, per PUB-01 success criterion 3 |

**Key insight:** Cargo's `package =` rename key and napi-rs's `rename` subcommand exist precisely because "keep the code's internal name stable while changing the published identity" is a common, well-solved packaging problem — this phase is not a novel scenario.

## Common Pitfalls

### Pitfall 1: The "obvious" rename breaks the build silently until `cargo check`
**What goes wrong:** Renaming `[package] name` and then updating dependents' table keys to match (the naive symmetric rename) compiles fine at the TOML level (no TOML syntax error) but fails at the Rust compilation step with `error[E0433]: cannot find module or crate`.
**Why it happens:** The dependency table key, not the package name, is what Rust's `use` statements resolve against (absent `package =`).
**How to avoid:** Use `package =` exclusively; never rename the table key for the 3 renamed crates.
**Warning signs:** Any `use nono::` / `use nono_proxy::` compile error immediately after a manifest edit, in ANY of the 7 dependent files (5 in-workspace + 2 sibling repos).

### Pitfall 2: Sibling-repo breakage is invisible to `make build`
**What goes wrong:** `make build` / `cargo build --workspace` in THIS repo reports success, giving false confidence that PUB-01 success criterion 4 ("both binding builds are green") is met.
**Why it happens:** `../nono-py` and `../nono-ts` are separate git repositories with their own Cargo.toml/Cargo.lock, invisible to this workspace's build graph. They only get exercised by explicitly running `maturin build` in `../nono-py` and `napi build --platform --release` (or `npm run build`) in `../nono-ts`.
**How to avoid:** Treat "workspace build green" and "both binding builds green" as three SEPARATE, sequential verification steps, per this repo's own memory precedent (`project_v34_opened.md`: "re-run maturin+napi build in nono-py/nono-ts after every UPST absorb touching nono-proxy structs — only `maturin build` catches it").
**Warning signs:** A plan that only lists `make build` as its verification step and treats the phase as done.

### Pitfall 3: `cargo publish --dry-run` on the renamed crates will still fail today (unrelated to this phase)
**What goes wrong:** Attempting `cargo publish --dry-run -p nono-sandbox-proxy` (or `-cli`) after the rename may still exit non-zero.
**Why it happens:** Per this repo's own recorded history (`97-03` decision log), `cargo publish --dry-run` for downstream workspace crates resolves dependencies from the LIVE crates.io index at packaging time — until `nono-sandbox` is actually published (Phase 105), `nono-sandbox-proxy`/`nono-sandbox-cli` dry-run publishes will report the dependency as unresolvable on the registry. This is expected and is NOT a regression introduced by Phase 102; it is deferred to Phase 105's dependency-ordered publish.
**How to avoid:** Do not treat a `cargo publish --dry-run` failure on the two downstream crates as a Phase 102 blocker; Phase 102's own success criterion 4 only requires `make build` (workspace compile), not a registry dry-run.
**Warning signs:** Conflating "workspace compiles" (Phase 102 scope) with "publishes cleanly" (Phase 105 scope).

### Pitfall 4: The `@oscarmackjr` npm scope has no owning account yet
**What goes wrong:** Assuming "the npm name is available" (404 on the package registry lookup) also means "the scope is ready to publish into."
**Why it happens:** A scoped package `@scope/name` requires either an npm user account named exactly `scope`, or an npm Organization named `scope`, to exist and be owned by the publishing credential — this is a separate precondition from the package name itself being unclaimed. `https://www.npmjs.com/~oscarmackjr` currently 404s (no such npm user profile exists).
**How to avoid:** Flag this as an operator action item for Phase 105 (create the npm user/org named `oscarmackjr` before attempting `npm publish --access public`), not something Phase 102 needs to resolve (Phase 102 only needs name AVAILABILITY confirmed, which is true regardless of scope ownership).
**Warning signs:** Phase 105 planning assuming the scope already exists without an explicit operator checkpoint.

### Pitfall 5: `nono-ts`'s own CLAUDE.md documents a path that doesn't match its actual Cargo.toml
**What goes wrong:** `../nono-ts/CLAUDE.md` states `Cargo.toml references nono = { path = "../nono/crates/nono" }`, but the actual file reads `path = "../Nono/crates/nono"` (capital N). Similarly `../nono-py/Cargo.toml` uses `../Nono/crates/nono`.
**Why it happens:** Pre-existing drift, likely tolerated because Windows filesystem paths are case-insensitive (this repo's actual directory is lowercase `nono`, and `../Nono/...` silently resolves correctly on Windows/macOS default filesystems but would NOT resolve on a case-sensitive Linux filesystem/CI runner).
**How to avoid:** Not this phase's job to fix (out of scope for PUB-01), but the planner should NOT be surprised if a Linux CI runner building `nono-py`/`nono-ts` from a fresh clone fails on this unrelated pre-existing path-casing bug, and should not misattribute it to the rename.
**Warning signs:** A Linux-runner build failure with "No such file or directory: ../Nono" — this is NOT a Phase 102 regression.

## Code Examples

### `crates/nono-proxy/Cargo.toml` — dependency-table diff (representative pattern for all 5 in-workspace + 2 sibling-repo dependency edits)
```toml
# Before
[dependencies]
nono = { version = "0.66.1", path = "../nono", default-features = false }

# After
[dependencies]
nono = { version = "0.66.1", path = "../nono", package = "nono-sandbox", default-features = false }
```
Source: empirically verified this session (isolated scratch Cargo workspace, `cargo build` exit 0 with `use <key>::` compiling against a differently-named `[package] name`).

### `crates/nono-cli/Cargo.toml` — feature-forwarding stays unchanged
```toml
# UNCHANGED — dependency KEYS ("nono", "nono-proxy") are what feature-forwarding
# syntax references, not package names, so no edit needed here even though
# the underlying packages are renamed.
system-keyring = ["dep:keyring", "nono/system-keyring", "nono-proxy/system-keyring"]
```
Source: [CITED: doc.rust-lang.org/cargo/reference/specifying-dependencies.html] — "names of features take after the name of the dependency, not the package name, when renamed."

### `../nono-ts` — official rename tool invocation
```bash
cd /c/Users/OMack/nono-ts
npx napi rename --package-name @oscarmackjr/nono-ts
# Deliberately NOT passing --name or --binary-name (-n/-b) — leaves the
# napi.binaryName ("nono") and any unrelated project display name untouched.
```
Source: [CITED: raw GitHub `napi-rs/website` `legacy_pages/docs/cli/rename.en.mdx`] — official flag table: `--package-name` = "The new package name of the project" (distinct from `--name` and `--binary-name`).

### Registry availability checks (all run live this session, 2026-07-02)
```bash
# crates.io — MUST include a User-Agent or you get a misleading 403, not 404
curl -s -A "nono-research/1.0 (oscar.mack.jr@gmail.com)" -o /dev/null -w "%{http_code}\n" \
  https://crates.io/api/v1/crates/nono-sandbox        # -> 404 (available)
curl -s -A "nono-research/1.0" -o /dev/null -w "%{http_code}\n" \
  https://crates.io/api/v1/crates/nono-sandbox-proxy  # -> 404 (available)
curl -s -A "nono-research/1.0" -o /dev/null -w "%{http_code}\n" \
  https://crates.io/api/v1/crates/nono-sandbox-cli    # -> 404 (available)

# PyPI
curl -s -o /dev/null -w "%{http_code}\n" https://pypi.org/pypi/nono-sandbox/json  # -> 404 (available)

# npm (scoped names must be percent-encoded: @ -> %40, / -> %2F)
curl -s -o /dev/null -w "%{http_code}\n" https://registry.npmjs.org/@oscarmackjr%2Fnono-ts  # -> 404 (available)

# npm scope ownership check (separate from name availability)
curl -s -o /dev/null -w "%{http_code}\n" https://www.npmjs.com/~oscarmackjr  # -> 404 (no npm user/org "oscarmackjr" exists yet)
```
All results verified live in this session via the Bash tool — **VERIFIED**, not assumed.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `cargo search <name>` to check crates.io availability | `cargo search` is disabled/deprecated on crates.io (returns nothing/errors); use the registry HTTP API (`crates.io/api/v1/crates/<name>`) or the sparse index (`index.crates.io/<p1>/<p2>/<name>`) directly, always with a `User-Agent` header | crates.io policy change (pre-dates this milestone; confirmed empirically this session — `cargo search` returned nothing while the API/index returned the expected 404) | Planner must not rely on `cargo search` in a verification task; use `curl` with the API endpoint instead |

**Deprecated/outdated:** `cargo search` for availability checks (empirically returned empty output in this session against `nono-sandbox`, while the direct API call correctly returned `404`).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Cargo.lock regeneration will produce "exactly 3 renamed `[[package]] name` hunks + cross-reference updates, zero third-party drift" for the main workspace | Standard Stack table | LOW — this is a standard, mechanical Cargo behavior; if wrong, `git diff Cargo.lock` after the build will simply show more/different hunks than expected, easily caught by a diff-review verification step |
| A2 | Feature-forwarding syntax (`"nono/system-keyring"`) continues to work unchanged after the `package =` rename, without an independent empirical re-test in THIS specific workspace | Standard Stack table, Code Examples | LOW-MEDIUM — backed by official Cargo Book text and consistent with the empirically-verified key-based extern-name behavior, but not independently re-verified with a scratch feature-flag test in this session (time-boxed). If wrong, `cargo build -p nono-cli --no-default-features --features system-keyring` would fail to enable `nono`'s `system-keyring` feature — easily caught by the existing `make build`/`make ci` feature-matrix coverage |
| A3 | `napi rename --package-name` correctly patches ALL FOUR existing `npm/*/package.json` files' `name` fields AND the main `package.json`'s `optionalDependencies` block in one invocation, without needing separate `--npm-dir`/manual per-file edits | Standard Stack, Code Examples | MEDIUM — based on official flag documentation and the tool's stated purpose, but the exact scope of what one invocation touches was not empirically run in this session (would require executing the rename against the real nono-ts working tree, which the planner/executor should do, not research). If the tool doesn't cover the pre-existing missing `win32-x64-msvc` npm subdirectory, that gap (Pitfall 5) was already present before this phase and is not a regression |
| A4 | Cargo.lock entries for local workspace path-member packages carry no `source`/checksum field, so the rename introduces no checksum-mismatch risk | Standard Stack table | LOW — standard, well-known Cargo.lock format behavior, not independently re-verified by inspecting this repo's actual `Cargo.lock` bytes in this session |

**If this table is empty:** N/A — see above; all four assumptions are LOW-to-MEDIUM risk, none blocking, and each has an obvious detection mechanism baked into the phase's own build-green success criterion.

## Open Questions (RESOLVED)

**RESOLVED (2026-07-03, during planning):** Q1 resolved → sibling-repo edits are scoped INTO Phase 102 (Plans 102-03 for nono-py, 102-04 for nono-ts, re-verified in 102-05), each a separate DCO-signed commit in its own repo. Q2 resolved → npm `@oscarmackjr` scope ownership is captured as a non-blocking Phase 105 precondition (see `102-VALIDATION.md` Manual-Only Verifications), NOT a Phase 102 gate.

1. **Should the sibling repos' Cargo.toml edits be scoped INTO Phase 102, or handed off as a dependency to Phase 105?** *(RESOLVED: scoped into Phase 102 — Plans 102-03/04/05.)*
   - What we know: PUB-01 success criterion 4 explicitly requires "both binding builds are green," which is impossible without patching `../nono-py/Cargo.toml` and `../nono-ts/Cargo.toml`'s own `nono`/`nono-proxy` dependency stanzas (Pitfall 2).
   - What's unclear: Whether the phase's task list should physically edit files in `../nono-py`/`../nono-ts` (separate git repos, separate commit history) within the same `/gsd:execute-phase 102` run, or whether that requires a distinct commit/PR flow per repo.
   - Recommendation: Scope it INTO Phase 102 — the success criterion is unambiguous, and deferring it would mean Phase 102 "completes" with green in this repo alone while silently leaving two repos broken. Use three separate DCO-signed commits (one per repo), matching this project's existing pattern for cross-repo binding bumps (see `100-03` decision log: "cross-repo binding bump ... one DCO-signed commit each").

2. **Does the operator need to create the `oscarmackjr` npm user/org before Phase 102 closes, or only before Phase 105?** *(RESOLVED: only before Phase 105 — non-blocking note here, hard gate there.)*
   - What we know: Phase 102 success criterion 3 only requires confirming the NAME is available (done, live-verified). Actually reserving/owning the `@oscarmackjr` scope requires npm account/org creation, which is an operator action with no `curl`-based verification path (requires an authenticated npm login).
   - What's unclear: Whether "confirmed live against the actual registry" in PUB-01's wording implies the scope-ownership precondition should also be raised NOW as an operator checkpoint, even though it isn't strictly required to complete Phase 102's manifest-rename work.
   - Recommendation: Raise it as a non-blocking operator note in this phase's plan (informational, not a checkpoint gate) — the real gate belongs in Phase 105 (PUB-02, actual publish), where it would hard-block `npm publish`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | Workspace manifest edits + `make build` verification | ✓ | cargo 1.95.0, rustc 1.95.0 | — |
| Python 3 | maturin build for nono-py | ✓ | 3.12.10 | — |
| maturin | PyPI-facing build verification (`maturin build`) | ✓ | 1.14.1 (in nono-py's `.venv`) | — |
| Node.js / npm | npm-facing build verification | ✓ | Node v24.15.0, npm 11.15.0 | — |
| `@napi-rs/cli` (`napi` command) | `napi rename`, `napi build --platform` | ✓ | 3.6.0 (via `npx napi --version` in nono-ts) | — |
| Network access to crates.io / pypi.org / registry.npmjs.org | Live name-availability checks (success criterion 3) | ✓ | — | Corporate-proxy TLS interception noted elsewhere in project memory as a risk for `az` CLI specifically; NOT observed as a problem for `curl` against these three registries in this session (all three returned clean 200/404 responses) |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None — all required tooling is present and working on this host.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None applicable — this phase is a manifest/config rename with no new logic to unit-test. Verification is build-green + live registry checks, mirroring the phase's own stated success criteria. |
| Config file | N/A |
| Quick run command | `cargo check --workspace --all-targets` (fast compile-only check after each manifest edit) |
| Full suite command | `make ci` (this repo) + `maturin build` (nono-py) + `npx napi build --platform --release` (nono-ts) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|--------------------|-------------|
| PUB-01 (SC1) | crates.io `[package] name` renamed on 3-crate publish set; internal path-dep names/pins reconciled; `[[bin]]`/`[lib]` unchanged | build/manifest | `cargo check --workspace --all-targets && grep -c 'name = "nono"' crates/nono-cli/Cargo.toml` (expect the `[[bin]] name = "nono"` line still present, package name changed) | ✅ — verifiable via existing `Cargo.toml`/`Cargo.lock`, no new file needed |
| PUB-01 (SC2) | `nono-py` PyPI project name → `nono-sandbox`; `nono-ts` npm name → `@oscarmackjr/nono-ts` in both sibling repos | manifest | `grep '^name' ../nono-py/pyproject.toml` and `node -e "console.log(require('../nono-ts/package.json').name)"` | ✅ |
| PUB-01 (SC3) | Each new registry identity's availability confirmed live before committing | integration (network) | `curl -s -A "ua" -o /dev/null -w "%{http_code}" https://crates.io/api/v1/crates/<name>` (×3), `curl .../pypi/nono-sandbox/json`, `curl .../@oscarmackjr%2Fnono-ts` — all expect `404` | ✅ — already run live in this research session, see Code Examples |
| PUB-01 (SC4) | Workspace build (`make build`) and both binding builds (`maturin build`, napi build) green under new names | build | `make build` (this repo); `cd ../nono-py && maturin build`; `cd ../nono-ts && npx napi build --platform --release` | ✅ |

### Sampling Rate
- **Per task commit:** `cargo check --workspace --all-targets` after each manifest edit (fast, catches E0433-class import breakage immediately).
- **Per wave merge:** `make build` (this repo) + `maturin build` (nono-py) + `napi build --platform --release` (nono-ts).
- **Phase gate:** All three full builds green, plus a fresh live re-check of the three registry availability endpoints (in case another party claimed a name between research and execution).

### Wave 0 Gaps
None — existing build tooling (`cargo`, `maturin`, `napi`) fully covers this phase's requirements; no new test files or fixtures are needed since there is no new runtime logic, only manifest identity changes.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | No | N/A — no auth surface touched |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A |
| V5 Input Validation | No | N/A — no user input parsing added |
| V6 Cryptography | No | N/A — no crypto touched |

### Known Threat Patterns for this phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| Name-squatting / dependency confusion — a third party registers `nono-sandbox`/`nono-sandbox-proxy`/`nono-sandbox-cli`/`@oscarmackjr/nono-ts` on the public registry between this research and the actual Phase 105 publish, then ships a malicious package under the name users expect | Spoofing | Re-run the live availability checks (Code Examples section) immediately before Phase 105's actual publish step, not just once during Phase 102 research — this repo's own PUB-02 success criteria already require dependency-ordered, index-polled publishing, which naturally re-surfaces a "already exists and isn't ours" failure early |
| Sibling-repo Cargo.toml edits accidentally reintroducing a stale/incorrect path or dependency version that resolves to an unintended crate | Tampering | Verify via `cargo tree -p nono-py` / `cargo metadata` (or the simpler `maturin build` success itself) that the resolved dependency graph points at the expected local path, not a stray registry hit |

## Sources

### Primary (HIGH confidence)
- Empirical `cargo build` verification in an isolated scratch Cargo workspace this session (producer/consumer crates with differing `[package] name`/`[lib] name`/dependency-key/`package =` combinations) — 4 build runs, all outcomes as documented above.
- Live HTTP checks against `crates.io/api/v1/crates/*`, `pypi.org/pypi/*/json`, `registry.npmjs.org/*`, `www.npmjs.com/~oscarmackjr` this session (2026-07-02).
- Direct reads of this repo's actual `Cargo.toml` files (workspace root + 6 crates), `../nono-py/Cargo.toml` + `pyproject.toml` + `python/nono_py/__init__.py`, `../nono-ts/Cargo.toml` + `package.json` + `npm/*/package.json` + `CLAUDE.md`.
- [CITED: doc.rust-lang.org/cargo/reference/specifying-dependencies.html] — the `package` key rename mechanism and feature-forwarding-follows-dependency-key behavior.
- [CITED: raw.githubusercontent.com napi-rs/website legacy_pages/docs/cli/rename.en.mdx] — official `napi rename` flag table (`--name`, `--binary-name`, `--package-name` are three distinct, independent flags).

### Secondary (MEDIUM confidence)
- WebSearch summary of napi-rs scoped-package conventions (`@napi-rs/cli` appends platform suffixes to scoped names, e.g. `@napi-rs/snappy-darwin-x64`) — consistent with, but not a byte-for-byte quote of, the official docs; corroborated by this repo's own existing (unscoped) `nono-ts-<platform>-<arch>` naming pattern already present in `npm/*/package.json`.

### Tertiary (LOW confidence)
- None flagged — all findings in this research were either empirically verified in this session or cited from an official/authoritative source.

## Metadata

**Confidence breakdown:**
- Standard stack (Cargo `package =` mechanism): HIGH — empirically verified by direct `cargo build` execution in this session, not merely recalled from training data.
- Architecture (cross-repo propagation, edit sequence): HIGH — derived directly from reading the actual manifests and grepping actual import statements in all affected files.
- Pitfalls: HIGH — each pitfall is either an empirically demonstrated failure mode (Pitfall 1) or a directly-observed pre-existing repo condition (Pitfalls 2-5, all confirmed by reading real files/history in this session).
- Registry availability: HIGH — live-checked this session, not assumed from training data.

**Research date:** 2026-07-02
**Valid until:** 7 days (registry name availability is a live, time-sensitive fact — re-verify immediately before Phase 105's actual publish, per the Security Domain note above)
