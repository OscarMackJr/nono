---
phase: 108-upst12-divergence-audit
plan: 01
ledger_type: upst12-audit
range: d817ed53663c6bba4669ee7a5bfb41b35971fd1b..59bdace7e905c05c127f480dc6d2a8c3a3331392 (v0.66.0..v0.69.0)
upstream_head_at_audit: 59bdace7e905c05c127f480dc6d2a8c3a3331392
refetch_date: 2026-07-29
fork_baseline: v0.66.1 (crate version; Phase 100 leapfrog endpoint, strictly above upstream v0.66.0)
total_unique_commits: 100
date: 2026-07-29
---

## Headline

100 non-merge commits in window `d817ed53..59bdace7` (upstream `v0.66.0..v0.69.0`, tags
`v0.67.0`, `v0.67.1`, `v0.68.0`, `v0.69.0`), plus 4 merge commits (excluded from all counts
below per D-15/task instructions). This plan populates the Reproduction block, the full
CODE/DEPS/CI/DOCS accounting, and a 5-cluster Cluster Summary skeleton. Dispositions,
windows-touch, and security-relevant flags in the Cluster Summary are left as literal `TBD` —
Plans 108-03/108-04/108-05 fill those in.

**Bucket totals (measured live 2026-07-29, this plan): CODE 62 / DEPS 19 / CI 11 / DOCS 8 = 100.**
This **disagrees** with CONTEXT.md's recorded hypothesis of **CODE 68 / DEPS 15 / CI 11 / DOCS 6**
(same 100 total, same CI count, different CODE/DEPS/DOCS split). Per D-04/D-21 this disagreement
is exactly the outcome re-measurement exists to catch — see "Discrepancy vs. CONTEXT.md
Hypothesis" under Full Commit Accounting for the reconciliation and the exact rule that produces
each split.

**Tool-sandbox surface:** 20 commits (live-reconfirmed, matches CONTEXT.md D-05/D-09 exactly).
**D-06 re-measurement finding:** the directory-only set (`tool-sandbox/`) is **18** commits, not
20 — it does **not** equal the 3-path union this window. See "Tool-Sandbox Surface — D-06
Re-measurement" below.

---

## Reproduction

```bash
# The `upstream` remote already points at nolabs-ai/nono (repointed in v3.3 Phase 94).
# No remote relocation work this cycle.

# Fetch upstream with tags (non-destructive)
git fetch upstream --tags

# Resolve window tip + intermediate tag SHAs (D-02: pin SHAs, not tag names)
git ls-remote --tags upstream v0.66.0 v0.67.0 v0.67.1 v0.68.0 v0.69.0
```

**Live output (2026-07-29):**

```
d817ed53663c6bba4669ee7a5bfb41b35971fd1b  refs/tags/v0.66.0
19b43f300b9d2e257e161b808440f0573abfd422  refs/tags/v0.67.0
6118b79aeda1365da213d85457b4d3cf1201d575  refs/tags/v0.67.1
00692e8c7846c6ee00ad6239d1be3b9e9b8d5dea  refs/tags/v0.68.0
59bdace7e905c05c127f480dc6d2a8c3a3331392  refs/tags/v0.69.0
```

All 5 SHAs match the pinned values given in `108-CONTEXT.md`'s interfaces block verbatim.
**No escalation needed** — upstream has not moved past v0.69.0 since the 2026-07-29 baseline.

```bash
# SHA guard: verify both window-tip SHAs are reachable commit objects
git cat-file -t d817ed53663c6bba4669ee7a5bfb41b35971fd1b   # -> commit (v0.66.0, window base)
git cat-file -t 59bdace7e905c05c127f480dc6d2a8c3a3331392   # -> commit (v0.69.0, window tip)

# Full 5-tag resolvability guard (all SHAs written into this ledger)
for sha in d817ed53663c6bba4669ee7a5bfb41b35971fd1b \
           19b43f300b9d2e257e161b808440f0573abfd422 \
           6118b79aeda1365da213d85457b4d3cf1201d575 \
           00692e8c7846c6ee00ad6239d1be3b9e9b8d5dea \
           59bdace7e905c05c127f480dc6d2a8c3a3331392; do
  git cat-file -e "${sha}^{commit}" && echo "OK: $sha"
done
# -> OK for all 5 (verified 2026-07-29)

# Full commit counts across the window
RANGE="d817ed53663c6bba4669ee7a5bfb41b35971fd1b..59bdace7e905c05c127f480dc6d2a8c3a3331392"
git log --no-merges --oneline $RANGE | wc -l   # -> 100
git log --merges --oneline $RANGE | wc -l      # -> 4
git log --merges --oneline $RANGE
# -> 8f4b1613 Merge pull request #1343 from nolabs-ai/feat/provider-plugins
# -> bda29e86 Merge branch 'main' into feat/provider-plugins
# -> a86f3886 Merge branch 'main' into feat/provider-plugins
# -> 06e9f2e9 Merge pull request #1324 from kipz/kipz/per-intercept-sandbox
```

**Merge-commit handling (unlike UPST11, which had zero merges):** all 4 merge SHAs above are
explicitly excluded from the 100-commit accounting below — they carry no independent diff content
of their own (both are `Merge branch 'main' into ...` / `Merge pull request ...` merge commits;
their substantive changes are already present as the non-merge commits they merge in).

```bash
# Tool-sandbox surface: D-06 three-path union (authoritative — see finding below)
git log --no-merges --oneline $RANGE -- \
  'crates/nono-cli/src/tool-sandbox/' \
  'crates/nono-cli/src/command_policy.rs' \
  'crates/nono-cli/src/lineage_cgroup.rs' | wc -l
# -> 20 (matches CONTEXT.md D-05/D-09 exactly)
```

**Auditor rerun:** the `git log --no-merges --oneline $RANGE | wc -l` command above, run verbatim
against these exact pinned SHAs, must return 100. Any other number means upstream has moved and
the audit must be re-run from scratch.

### Tool-Sandbox Surface — D-06 Re-measurement

D-06 hypothesized "the union of these three [paths] equals the directory-only set (true for this
window — 20 either way)". **Live re-measurement finds this claim is false for this window:**

```bash
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/tool-sandbox/' | wc -l
# -> 18  (directory-only, NOT 20)
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/command_policy.rs' | wc -l
# -> 9
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/lineage_cgroup.rs' | wc -l
# -> 1
```

Set-difference shows exactly 2 commits touch `command_policy.rs` but **not** `tool-sandbox/`:
`5a7447d3ed30835bd9bd647b7812ee18ea80a155` and `ebd51cbb9546aec872136301249d048074a05e38`. The
`lineage_cgroup.rs`-touching commit (`a519ee62e6d571330caa6a9a7d033fb10bff5939`) is already inside
the directory-only set. So: `18 (directory-only) + 2 (command_policy.rs-only) + 0 (new from
lineage_cgroup.rs) = 20 (union)`.

**Finding:** the directory-only set (18) and the 3-path union (20) are **not** equal this window
— they only happened to be numerically restated as "20 either way" in D-06's phrasing, which this
re-measurement shows is imprecise: the directory-only count is 18, not 20. A directory-only filter
would silently drop `5a7447d3` and `ebd51cbb`. This is the concrete validation of D-06's own
caveat that the equality "holds by coincidence, not by rule" — in this window the two sets are
close but not identical, and the union (not the directory) is the authoritative filter. **Future
syncs must use the explicit 3-path union, never the directory alone.**

---

## Full Commit Accounting (100 non-merge commits)

**Classification rule (D-15):** file-set based, not prefix-based. A commit lands in **CODE** if
any touched path is under `crates/*/src/` or `bindings/c/src/` (regardless of what else it
touches). Otherwise: **DEPS** if every touched path is `Cargo.toml` and/or `Cargo.lock` (any
crate); **CI** if every touched path is under `.github/`, is `Makefile`, or is under `scripts/`;
**DOCS** if every touched path is under `docs/`, starts with `CHANGELOG`, or ends in `.md`/`.mdx`.

**Bucket totals: CODE 62, DEPS 19, CI 11, DOCS 8. Sum = 62+19+11+8 = 100 = total non-merge count.**
Every one of the 100 SHAs appears in exactly one bucket table below (verified: `git log --no-merges
--format='%H' $RANGE | sort` deduped against the concatenated, sorted, deduped bucket SHA lists —
zero SHAs in zero buckets, zero SHAs in 2+ buckets).

### Bucket Classification Edge Cases

9 of the 100 commits do not touch `src/` yet also fail the strict "ONLY <pattern>" test for
DEPS/CI/DOCS (they mix a DEPS-pattern file with a DOCS-pattern file, or touch a path the base
rule doesn't name at all). Each is resolved below with an explicit, reproducible rule so a future
auditor gets the same split:

1. **Release-cut commits** (`59bdace7…` v0.69.0, `00692e8c…` v0.68.0, `6118b79a…` v0.67.1,
   `19b43f30…` v0.67.0) — each touches `CHANGELOG.md` (DOCS-pattern) **and** `Cargo.lock` +
   4× `Cargo.toml` (DEPS-pattern), no `src/`. **Rule applied: DEPS.** Rationale: the version bump
   is the substantive, non-generated content; `CHANGELOG.md` in a release-cut commit is
   auto-generated release notes accompanying that bump, not independently authored documentation.
   → 4 commits classified DEPS.
2. **`f016b2d5…` (mise policy #1387) and `b620ed8e…` (bun preset #1305)** — each touches only
   `crates/nono-cli/data/policy.json` (embedded-at-build-time security policy data per
   `build.rs`/CLAUDE.md, physically outside `src/`) and a `crates/nono-cli/tests/*.rs` file.
   **Rule applied: CODE.** Rationale: `data/policy.json` is compiled into the binary as the
   single source of truth for built-in groups/deny-rules/profiles — functionally source, not
   descriptive documentation, even though it lives outside the literal `src/` prefix.
   → 2 commits classified CODE.
3. **`9840a16f…` (test: suppress save prompt in socket access tests #1279)** — touches only
   `crates/nono-cli/tests/socket_access_run.rs`. **Rule applied: CODE.** Rationale: Rust test
   files under `crates/*/tests/` are compiled Rust code with no natural home in DEPS/CI/DOCS;
   test-only commits are grouped with CODE so they are reviewed alongside the feature they test
   in Phases 109-112, rather than silently dropped into a documentation bucket.
   → 1 commit classified CODE.
4. **`d7406333…` (fix: permanent community link #1349) and `2f547703…` (docs: community health
   files #1348)** — each touches `.github/ISSUE_TEMPLATE/config.yml` (a CI-pattern path under
   `.github/`) together with several `.md` files (`CONTRIBUTING.md`, `MAINTAINERS.md`,
   `README.md`, `CODE_OF_CONDUCT.md`, `CONTRIBUTORS.md`, `GOVERNANCE.md`) and, in one case,
   `docs/docs.json`. **Rule applied: DOCS.** Rationale: GitHub issue-template YAML is
   contributor-facing content (not build/release pipeline configuration like
   `.github/workflows/*.yml`); grouping it with the accompanying community-health `.md` files as
   DOCS is more accurate than forcing a false CI classification.
   → 2 commits classified DOCS.

Net effect of the edge-case rules on the strict pre-override counts (CODE 59 / DEPS 15 / CI 11 /
DOCS 6, sum 91, 9 unclassified): **+3 CODE (mise, bun, test-suppress), +4 DEPS (4 releases), +2
DOCS (2 community-health)** → final **CODE 62 / DEPS 19 / CI 11 / DOCS 8 = 100**.

### Discrepancy vs. CONTEXT.md Hypothesis

CONTEXT.md's `108-CONTEXT.md` records a baseline hypothesis of **CODE 68 / DEPS 15 / CI 11 /
DOCS 6** "taken 2026-07-29 during discussion; re-verify at audit time, do not copy forward
blindly." This plan's live re-measurement produces **CODE 62 / DEPS 19 / CI 11 / DOCS 8** — CI
matches exactly; CODE, DEPS, and DOCS do not.

**Both numbers are recorded here; this ledger's measured split (62/19/11/8) is authoritative for
all downstream work in Phases 109-112. CONTEXT.md's 68/15/11/6 is superseded.**

The arithmetic explains the gap precisely: CONTEXT.md's strict pre-override counts (59/15/11/6,
sum 91) are identical to this ledger's, modulo the same 9 ambiguous commits. If **all 9** ambiguous
commits are folded into CODE (59+9=68), DEPS/CI/DOCS stay at their strict values (15/11/6) —
which is exactly CONTEXT.md's number. This ledger instead applies the **per-commit rule above**
(release commits → DEPS; policy-data + test-only commits → CODE; community-health-with-YAML →
DOCS) rather than a blanket "fold ambiguous into CODE" rule. **The one-line reason the splits
differ: CONTEXT.md's hypothesis treated all 9 path-set edge cases as CODE; this ledger classifies
each edge case by its dominant semantic content instead (4 release→DEPS, 3 policy/test→CODE, 2
community-docs→DOCS).** Per D-04/D-21, this is exactly the outcome re-measurement exists to catch
— it is recorded as a finding, not silently reconciled to either number.

### Prefix-vs-Path-Set Cross-Check (D-15)

Conventional-commit-prefix guess: commits whose subject starts with `chore:`/`ci:`/`docs:`/
`build:` (optionally scoped, e.g. `chore(deps):`) = **38**. Path-set-based non-CODE bucket count
(DEPS 19 + CI 11 + DOCS 8) = **38**. The totals coincide, but the *sets* are not identical —
**4 commits disagree**:

| sha | prefix-guess says | path-set bucket says | why they disagree |
|-----|--------------------|-----------------------|--------------------|
| `9ef5918169d719821aaec757a9eea715c3e8c44d` | non-CODE (`docs(cli/profile):` prefix) | **CODE** | subject is `docs(cli/profile): simplify credential provider def doc comment`, but the change is a doc-comment edit *inside* `crates/nono-cli/src/profile/credential_provider.rs` — a `.rs` source file, not documentation |
| `f050643479d0dab8c86d7322a9a283a4f2d64322` | non-CODE (`chore:` prefix) | **CODE** | subject is `chore: migrate registry namespace references...`, but the commit touches 20+ `.rs` files under `crates/nono-cli/src/` and `crates/nono/src/trust/bundle.rs` |
| `002fe4986e9b03c61d0a90d78bbd535108a9b698` | CODE (no chore/ci/docs/build prefix — subject is `fix(release): ...`) | **CI** | touches only `.github/workflows/release.yml`; the `fix:` prefix type doesn't signal "non-CODE" under the D-15 prefix heuristic, but the path-set is pure CI |
| `d74063335e8584f9667496e25967e1c87e1d38e1` | CODE (subject is `fix: use permanent community link...`) | **DOCS** | touches `.github/ISSUE_TEMPLATE/config.yml` + 4 `.md` files, no `src/`; the `fix:` prefix type doesn't signal "non-CODE" but the path-set is DOCS per the edge-case rule above |

**Final disagreement count: 4.** Both directions occur (2 prefix-flagged-noise-but-actually-CODE;
2 prefix-flagged-CODE-but-actually-noise) — this validates D-15's warning that prefix alone is
unreliable in both directions, not just the "chore that touches source" direction it names.

### CODE bucket (62 commits)

| sha | subject | touched-path-summary |
|-----|---------|----------------------|
| 23d93fc96abf795d672c712e9c1834a8f97aa0aa | fix(proxy): don't cross-deny sibling routes sharing an upstream (#1437) | 1 file: crates/nono-proxy/src/tls_intercept/handle.rs |
| 6fb7ecbf36e5d18760d8084b7f4900e93582004c | bug: Fix SigV4 URI generation errors for uri's that have encoded characters in them (#1430) | 1 file: crates/nono-proxy/src/aws/sign.rs |
| 1619275caa32b7b96e3eee56c33c71dfce777bbf | feat: add profile-declared no_proxy bypass support (#1415) | 17 files: crates/nono-cli/data/nono-profile.schema.json, crates/nono-cli/data/profile-authoring-guide.md, crates/nono-cli/src/launch_runtime.rs, crates/nono-cli/src/main.rs, crates/nono-cli/src/profile/mod.rs, crates/nono-cli/src/profile_cmd.rs, +11 more |
| a519ee62e6d571330caa6a9a7d033fb10bff5939 | fix(tool-sandbox): attribute daemonized callers to their command (#1417) | 10 files: crates/nono-cli/data/nono-profile.schema.json, crates/nono-cli/src/command_policy.rs, crates/nono-cli/src/lineage_cgroup.rs, crates/nono-cli/src/main.rs, crates/nono-cli/src/profile/mod.rs, crates/nono-cli/src/resource_cgroup.rs, +4 more |
| c831dade422f2bdf37d7429af0423cafa0a60c06 | feat(proxy): add SPIFFE/SPIRE workload identity auth for upstream routes (#1272) | 33 files: .github/workflows/spire.yml, .gitignore, Cargo.lock, Makefile, crates/nono-cli/src/audit_integrity.rs, crates/nono-cli/src/audit_ledger.rs, +27 more |
| 099237da94d33752ca5617db3f4902a216f9f84c | fix(exec): raise MAX_CRYPTO_THREADS to 12 for macOS libdispatch workqueue threads (#1424) | 1 file: crates/nono-cli/src/exec_strategy.rs |
| cafc39eb3104e789cee2b663dc04b8aa7ef0062a | fix(sandbox): allow exec in writable grant-dirs under command policies (#1391) | 2 files: crates/nono-cli/src/tool-sandbox/platform/linux.rs, crates/nono-cli/tests/execution_strategy_run.rs |
| 34c2c975d649844923cf1515be94de624c689c6c | feat(resources): cap sandbox process count with --max-processes (#1403) | 15 files: crates/nono-cli/src/cli.rs, crates/nono-cli/src/command_runtime.rs, crates/nono-cli/src/output.rs, crates/nono-cli/src/resource_cgroup.rs, crates/nono-cli/src/sandbox_prepare.rs, crates/nono-cli/src/sandbox_state.rs, +9 more |
| d5803b994b416ad07a73907143ca169c408917f3 | feat: add port range support to sandbox profiles (#1398) | 17 files: crates/nono-cli/data/profile-authoring-guide.md, crates/nono-cli/src/capability_ext.rs, crates/nono-cli/src/exec_strategy.rs, crates/nono-cli/src/exec_strategy/supervisor_linux.rs, crates/nono-cli/src/output.rs, crates/nono-cli/src/profile/mod.rs, +11 more |
| ab93cf44e5706b49b1aa5f8ecc1b3b59f29596f8 | fix(tool-sandbox): preserve argv[0] for symlink-dispatched commands (#1413) | 3 files: crates/nono-cli/src/tool-sandbox/env.rs, crates/nono-cli/src/tool-sandbox/platform/linux.rs, crates/nono-cli/src/tool-sandbox/platform/macos.rs |
| f050643479d0dab8c86d7322a9a283a4f2d64322 | chore: migrate registry namespace references from always-further to nolabs-ai (#1405) | 24 files: .github/ISSUE_TEMPLATE/onboarding_issue.yml, README.md, crates/nono-cli/README.md, crates/nono-cli/data/policy.json, crates/nono-cli/src/cli.rs, crates/nono-cli/src/legacy_cleanup.rs, +18 more |
| ac5ccd70712782f4a97c54916ee18b03c650f56f | fix(sandbox): keep orphaned descendants in supervisor ancestry for seccomp-notify mediation (#1401) | 2 files: crates/nono-cli/src/exec_strategy.rs, crates/nono-cli/tests/socket_access_run.rs |
| 0374e454b9424e8b77c46fe0948a4e109d37b8da | fix(profile): omit inheritable Option fields when None on save (#1400)(#1402) | 1 file: crates/nono-cli/src/profile/mod.rs |
| d84b4818f824b7660d1a4d95ec2e56f322b6fa63 | fix(sandbox): grant Refer in execute-restriction layer on Linux (#1397) | 1 file: crates/nono/src/sandbox/linux.rs |
| ca888108fe5983be866e8d1a6eccf96edc2a8dd5 | fix: missing ~/.cache on macOS (#1378) | 6 files: crates/nono-cli/data/policy.json, crates/nono-cli/src/learn.rs, crates/nono-cli/src/macos_trust.rs, crates/nono-cli/src/proxy_command.rs, crates/nono-proxy/src/server.rs, crates/nono-proxy/src/tls_intercept/ca.rs |
| bf7ea3cb7faa241de13fe549152e28333b1f0068 | fix(tool-sandbox): grant env-shebang scripts their re-exec interpreter (#1394) | 3 files: crates/nono-cli/src/tool-sandbox/env.rs, crates/nono-cli/src/tool-sandbox/platform/linux.rs, crates/nono-cli/src/tool-sandbox/platform/macos.rs |
| f016b2d56dcbe4aad2bb322acd7574d38b5c13f7 | mise policy (#1387) | 2 files: crates/nono-cli/data/policy.json, crates/nono-cli/tests/manifest_roundtrip.rs |
| b620ed8e4572359afa4ed9cff4d803c8e9064f26 | feat(policy): add bun runtime preset (#1305) | 2 files: crates/nono-cli/data/policy.json, crates/nono-cli/tests/manifest_roundtrip.rs |
| 7fe0c8283810c4ca8c246d1b651f3f31578fdf55 | fix(registry): add X-Nono-Pull-Reason header to distinguish pull triggers (#1383)(#1386) | 5 files: crates/nono-cli/src/app_runtime.rs, crates/nono-cli/src/migration.rs, crates/nono-cli/src/package_cmd.rs, crates/nono-cli/src/profile/mod.rs, crates/nono-cli/src/registry_client.rs |
| 726ac1f1b5fd7b6de2d86b9fccf1d72660d3e32a | feat(proxy): support plain HTTP forward-proxying via HTTP_PROXY (#1335) | 1 file: crates/nono-proxy/src/server.rs |
| 72a988309974f426f0da8f507a4f05df9738ce92 | feat(tool-sandbox): add per-command exec_paths for multi-call binaries (#1384) | 4 files: crates/nono-cli/src/command_policy.rs, crates/nono-cli/src/profile/mod.rs, crates/nono-cli/src/tool-sandbox/platform/linux.rs, docs/cli/features/tool-sandbox.mdx |
| 719975cf03ea303f835061089279eb6c50245ccf | fix(profile): preserve platform_overrides through extends resolution (#1380) | 1 file: crates/nono-cli/src/profile/mod.rs |
| 1f54f4ae91870de02c1fabfda554278d49c99bd9 | fix(command-policy): resolve command_policies binaries once, in parallel, with caching (#1373) | 10 files: crates/nono-cli/src/command_policy.rs, crates/nono-cli/src/execution_runtime.rs, crates/nono-cli/src/launch_runtime.rs, crates/nono-cli/src/main.rs, crates/nono-cli/src/profile_runtime.rs, crates/nono-cli/src/proxy_runtime.rs, +4 more |
| 3b207eeb884bd71b0fc10f0123fa89fffe9f7955 | feat(proxy): add deny_domain to block domains through the proxy (#1374) | 15 files: crates/nono-cli/data/profile-authoring-guide.md, crates/nono-cli/src/cli.rs, crates/nono-cli/src/launch_runtime.rs, crates/nono-cli/src/main.rs, crates/nono-cli/src/network_policy.rs, crates/nono-cli/src/profile/mod.rs, +9 more |
| ae1c513e6ed1f1a0628dbb690c3f1c984fb7a225 | feat(profile): add platform_overrides field for per-OS profile patches (#1371) | 3 files: crates/nono-cli/data/profile-authoring-guide.md, crates/nono-cli/src/policy.rs, crates/nono-cli/src/profile/mod.rs |
| c344efb006365ba596b20b843f29fabdbbdc847e | fix(why): respect proxy domain filter in --profile and --self host queries (#1372) | 2 files: crates/nono-cli/src/execution_runtime.rs, crates/nono-cli/src/query_ext.rs |
| 4192bfa58101d0ef9737b60243a39695fc1e0fa8 | fix(proxy): skip credential_capture entries with missing helper binaries (#1368) | 1 file: crates/nono-cli/src/proxy_runtime.rs |
| 7c20dc75f4f155d64601719437980a0e232c8bd8 | fix(tool-sandbox): resolve command policy paths against the live cwd (#1339) | 10 files: crates/nono-cli/src/command_runtime.rs, crates/nono-cli/src/execution_runtime.rs, crates/nono-cli/src/launch_runtime.rs, crates/nono-cli/src/main.rs, crates/nono-cli/src/proxy_runtime.rs, crates/nono-cli/src/sandbox_prepare.rs, +4 more |
| 42161620ec68da9c4b09b2283e6584ecbf51d3a2 | fix(cli): match intercept args after global options (#1344) | 4 files: crates/nono-cli/data/nono-profile.schema.json, crates/nono-cli/src/command_policy.rs, crates/nono-cli/src/tool-sandbox/policy.rs, docs/cli/features/tool-sandbox.mdx |
| 0158d52f0a2a7542993ae48fda664ea7ce3359ff | feat(registry-client): attach installation context headers to registry requests (#1341) | 2 files: crates/nono-cli/src/registry_client.rs, crates/nono-cli/src/update_check.rs |
| 762eb05bd3d00557228b6ba0fa8e9bb21ef13cfb | feat(update-check): emit install_source on update check requests (#1340) | 3 files: .github/workflows/release.yml, crates/nono-cli/build.rs, crates/nono-cli/src/update_check.rs |
| eb2d61a7abc5a354b8f8085b782cd11afa6babb8 | feat(tool-sandbox): add exec intercept action (#1322) | 8 files: crates/nono-cli/data/nono-profile.schema.json, crates/nono-cli/src/command_policy.rs, crates/nono-cli/src/policy.rs, crates/nono-cli/src/tool-sandbox/env.rs, crates/nono-cli/src/tool-sandbox/platform/linux.rs, crates/nono-cli/src/tool-sandbox/platform/macos.rs, +2 more |
| 9ef5918169d719821aaec757a9eea715c3e8c44d | docs(cli/profile): simplify credential provider def doc comment | 1 file: crates/nono-cli/src/profile/credential_provider.rs |
| d033c63111472711e242f4067eb4be04aeaf618a | test(oauth): consume provider stdin in header fixture | 1 file: crates/nono-cli/src/proxy_runtime.rs |
| 3c59c62e7ddcc2d3a1f1c161bd039b5f2b0b5f7b | fix(oauth): harden capture security boundaries | 4 files: crates/nono-cli/src/profile_runtime.rs, crates/nono-proxy/src/forward.rs, crates/nono-proxy/src/oauth_capture/mod.rs, crates/nono-proxy/src/oauth_capture/persist.rs |
| 9b692e07ee4dd156d85588b12b63691778d9e9be | feat(oauth): add declarative sandboxed OAuth capture | 26 files: crates/nono-cli/data/nono-profile.schema.json, crates/nono-cli/data/profile-authoring-guide.md, crates/nono-cli/src/launch_runtime.rs, crates/nono-cli/src/main.rs, crates/nono-cli/src/policy.rs, crates/nono-cli/src/profile/credential_provider.rs, +20 more |
| 2663e9900fe1317cec0949c18b7a0e6b39321602 | feat(cli): add standalone `nono proxy` command (#1261) | 15 files: crates/nono-cli/src/app_runtime.rs, crates/nono-cli/src/cli.rs, crates/nono-cli/src/cli_bootstrap.rs, crates/nono-cli/src/main.rs, crates/nono-cli/src/proxy_command.rs, crates/nono-cli/src/proxy_runtime.rs, +9 more |
| f943fb5a0721352e85d92ef48e3e4d5b36747486 | fix(trust): add predicate field to distinguish nono trust policies from foreign JSON (#1333) | 7 files: crates/nono-cli/src/trust_cmd.rs, crates/nono-cli/src/trust_scan.rs, crates/nono/src/trust/mod.rs, crates/nono/src/trust/policy.rs, crates/nono/src/trust/types.rs, docs/cli/features/trust.mdx, +1 more |
| 5a7447d3ed30835bd9bd647b7812ee18ea80a155 | fix(tool-sandbox): strip untrusted unsafe_macos_seatbelt_rules before emission | 4 files: crates/nono-cli/src/command_policy.rs, crates/nono-cli/src/command_runtime.rs, crates/nono-cli/src/sandbox_prepare.rs, docs/cli/features/tool-sandbox.mdx |
| ebd51cbb9546aec872136301249d048074a05e38 | fix(tool-sandbox): warn on unsafe_macos_seatbelt_rules nested in command/intercept sandboxes | 3 files: crates/nono-cli/src/command_policy.rs, crates/nono-cli/src/profile_save_runtime.rs, docs/cli/features/tool-sandbox.mdx |
| e2c87fd5e82b1dd7c1cf9a70d9632f2b0310afb4 | feat(tool-sandbox): per-command unsafe_macos_seatbelt_rules escape hatch | 5 files: crates/nono-cli/data/nono-profile.schema.json, crates/nono-cli/src/command_policy.rs, crates/nono-cli/src/tool-sandbox/platform/macos.rs, crates/nono-cli/tests/schema_shape.rs, docs/cli/features/tool-sandbox.mdx |
| 261bbd68d984f58c5b7376143fdacbeaffef1ccc | fix(tests): raise credential-capture test timeout to reduce macOS CI flakiness | 1 file: crates/nono-cli/src/proxy_runtime.rs |
| 052b8374d2381ca4efdc6a51d9eee54d292a54da | feat(tool-sandbox): per-intercept sandbox override | 5 files: crates/nono-cli/src/command_policy.rs, crates/nono-cli/src/tool-sandbox/platform/linux.rs, crates/nono-cli/src/tool-sandbox/platform/macos.rs, crates/nono-cli/src/tool-sandbox/policy.rs, docs/cli/features/tool-sandbox.mdx |
| ea334d2bbdcb332c3a1c4164843667b0d2153cb1 | fix(linux): use u64 for fs_type_unsupported to fix musl build (#1332) | 3 files: .github/workflows/ci.yml, crates/nono-cli/src/tool-sandbox/platform/linux.rs, crates/nono/src/sandbox/linux.rs |
| e6d26871f0498e7dc7a867e67af5c6136b84f91c | feat: resource limiting (#1269) | 23 files: Cargo.lock, crates/nono-cli/Cargo.toml, crates/nono-cli/src/cli.rs, crates/nono-cli/src/command_runtime.rs, crates/nono-cli/src/exec_strategy.rs, crates/nono-cli/src/main.rs, +17 more |
| 0ecc476bf0db3c9509bcc2bd8efae43832c764b8 | feat: implement aws authentication for the MiTM proxy (#1195) | 11 files: Cargo.lock, crates/nono-proxy/Cargo.toml, crates/nono-proxy/src/aws/endpoints.rs, crates/nono-proxy/src/aws/mod.rs, crates/nono-proxy/src/aws/route.rs, crates/nono-proxy/src/aws/sign.rs, +5 more |
| d4927f95a37863cf0ba534b054e28f48f002ad21 | feat(profile): expand @git:* dynamic tokens in top-level filesystem paths (#1298) | 5 files: crates/nono-cli/src/capability_ext.rs, crates/nono-cli/src/tool-sandbox/dynamic_providers.rs, crates/nono-cli/src/tool-sandbox/mod.rs, crates/nono-cli/src/tool-sandbox/platform/linux.rs, crates/nono-cli/src/tool-sandbox/platform/macos.rs |
| 3672ea10c7d45b340bc389e1d5cf0a26dfa5db34 | fix(tests): share stdin-manipulation lock between capture_helper stdin tests (#1327) | 1 file: crates/nono-cli/src/proxy_runtime.rs |
| 503045801a9c54e7b1f5e4b6004766e7f9317b42 | fix(pty): drain late terminal query reply on teardown (#1258) | 2 files: crates/nono-cli/src/pty_proxy.rs, crates/nono-cli/src/timeouts.rs |
| 676a042f3c1e3877f1d1770abd19b0fea0345750 | fix(tool-sandbox): ack frame before SCM_RIGHTS send to prevent EMSGSIZE on macOS (#1325) | 3 files: crates/nono-cli/src/tool-sandbox/platform/linux.rs, crates/nono-cli/src/tool-sandbox/platform/macos.rs, crates/nono-cli/src/tool-sandbox/protocol.rs |
| 2cbaa9a017b14ddc45219474b6eebf94727787ef | feat(profile): expand $VAR tokens from process env in profile paths and capture commands (#1296) | 5 files: crates/nono-cli/src/capability_ext.rs, crates/nono-cli/src/policy.rs, crates/nono-cli/src/proxy_runtime.rs, crates/nono-cli/src/wiring.rs, docs/cli/features/credential-injection.mdx |
| 199bb26699661e8a36de141adc4767617e0a1036 | feat(tool-sandbox): add git worktree tokens; fold include-files into @git:config-files (#1280) | 3 files: crates/nono-cli/data/profile-authoring-guide.md, crates/nono-cli/src/tool-sandbox/dynamic_providers.rs, docs/cli/features/tool-sandbox.mdx |
| f58c7c2242d3790f9c390d4bf0dd808ba1261c77 | feat(profile): support CLI profile extends (#1320) | 9 files: CHANGELOG.md, crates/nono-cli/data/profile-authoring-guide.md, crates/nono-cli/src/cli.rs, crates/nono-cli/src/command_runtime.rs, crates/nono-cli/src/learn.rs, crates/nono-cli/src/profile/mod.rs, +3 more |
| a32439074a61eea21eb374a91fdbcebb91b0afe6 | feat(gpu): harden NVIDIA procfs mediation (#1284) | 15 files: crates/nono-cli/src/command_runtime.rs, crates/nono-cli/src/exec_strategy.rs, crates/nono-cli/src/exec_strategy/supervisor_linux.rs, crates/nono-cli/src/execution_runtime.rs, crates/nono-cli/src/launch_runtime.rs, crates/nono-cli/src/main.rs, +9 more |
| 8255a27a1d8bedf878a7c8c67f9007ca592962d1 | refactor load_with_diagnostics to be async (#1287) | 4 files: crates/nono-proxy/src/credential.rs, crates/nono-proxy/src/oauth2.rs, crates/nono-proxy/src/server.rs, crates/nono-proxy/src/tls_intercept/h2_forward.rs |
| a5a441c25769ecec663aed1d7038ab2b8814d43f | fix(profile): empty allow_vars no longer strips all env vars (#1204) | 5 files: crates/nono-cli/data/profile-authoring-guide.md, crates/nono-cli/src/profile/mod.rs, crates/nono-cli/src/profile_runtime.rs, docs/cli/features/environment.mdx, docs/cli/features/profile-authoring.mdx |
| 7d23bba683036789163385afa7f5c2f5888886a5 | fix(proxy): separate stdin and stderr inheritance for credential helpers (#1300) | 5 files: crates/nono-cli/data/nono-profile.schema.json, crates/nono-cli/data/profile-authoring-guide.md, crates/nono-cli/src/profile/mod.rs, crates/nono-cli/src/proxy_runtime.rs, docs/cli/features/credential-injection.mdx |
| f6f027511f7899f1735f1188747086696e40412e | fix(execution-runtime): allow env_credentials + command_policies on non-shim entry (#1301) | 3 files: crates/nono-cli/src/execution_runtime.rs, crates/nono-cli/src/profile/mod.rs, crates/nono-cli/tests/execution_strategy_run.rs |
| de8a54b25c3c26f68f6d51f768be904cee0d0c41 | fix(dynamic-providers): run git config from repo root to honour hasconfig: includeIf (#1313) | 1 file: crates/nono-cli/src/tool-sandbox/dynamic_providers.rs |
| 4cc0af2c52119c3794843416b292c2ed88d22c8c | fix(tests): use /tmp for socket test dirs to stay under SUN_LEN limit (#1303) | 3 files: crates/nono-cli/src/open_url_runtime.rs, crates/nono-cli/tests/url_open_integration.rs, crates/nono/src/supervisor/socket.rs |
| 8a4237f2ee0dc33bc1e5afdd39c0db8ca6f5ed38 | refactor(seccomp): introduce SeccompPolicy struct and client-driven selection (#1283) | 21 files: bindings/c/src/sandbox.rs, crates/nono-cli/src/cli.rs, crates/nono-cli/src/command_runtime.rs, crates/nono-cli/src/exec_strategy.rs, crates/nono-cli/src/exec_strategy/supervisor_linux.rs, crates/nono-cli/src/execution_runtime.rs, +15 more |
| 9840a16f354f9bdd79ed504b56467e1743a7e609 | test: suppress save prompt in socket access tests (#1279) | 1 file: crates/nono-cli/tests/socket_access_run.rs |

### DEPS bucket (19 commits)

| sha | subject | touched-path-summary |
|-----|---------|----------------------|
| 59bdace7e905c05c127f480dc6d2a8c3a3331392 | chore: release v0.69.0 | 6 files: CHANGELOG.md, Cargo.lock, bindings/c/Cargo.toml, crates/nono-cli/Cargo.toml, crates/nono-proxy/Cargo.toml, crates/nono/Cargo.toml |
| 00692e8c7846c6ee00ad6239d1be3b9e9b8d5dea | chore: release v0.68.0 (#1418) | 6 files: CHANGELOG.md, Cargo.lock, bindings/c/Cargo.toml, crates/nono-cli/Cargo.toml, crates/nono-proxy/Cargo.toml, crates/nono/Cargo.toml |
| ce3d6724ed172226b17ec2ec1a22ea4b21fb7858 | chore(deps): bump sigstore-trust-root from 0.9.0 to 0.11.0 (#1410) | 2 files: Cargo.lock, crates/nono/Cargo.toml |
| 2f7a6747b8e12e71838cd22def743dc1bd789cef | chore(deps): bump bytes from 1.12.0 to 1.12.1 (#1408) | 1 file: Cargo.lock |
| b6e7ec43474ca57d075f19c8ca318690719ef185 | chore(deps): bump regex from 1.12.4 to 1.13.0 (#1406) | 1 file: Cargo.lock |
| bc947b0bdb8d693a704c3cf9af6d098c2aa12805 | chore(deps): bump sigstore-sign from 0.10.0 to 0.11.0 (#1407) | 2 files: Cargo.lock, crates/nono-cli/Cargo.toml |
| 373a67ae65fdb94898b49469d562a263ba2083ee | chore(deps): bump crossbeam-epoch from 0.9.18 to 0.9.20 (#1369) | 1 file: Cargo.lock |
| 0bfb5c54621d933e28ff4c2800a0f2629ff648de | chore(deps): bump clap_complete from 4.6.5 to 4.6.7 (#1360) | 1 file: Cargo.lock |
| 0f2f635bcd547b52b8e86632c5b2ad753bb147c3 | chore(deps): bump ignore from 0.4.26 to 0.4.27 (#1363) | 1 file: Cargo.lock |
| 3cff37337ea60f1c5a10ebd335676575dd0ecd10 | chore(deps): bump time from 0.3.52 to 0.3.53 (#1358) | 1 file: Cargo.lock |
| b59477a56e47e91539ebfcadf4d374e26a66f3fa | chore(deps): bump rand from 0.10.1 to 0.10.2 (#1362) | 1 file: Cargo.lock |
| 18bbb5eb1fbbe1167c1eef8f99d748b40a65715e | chore(deps): bump sigstore-sign from 0.8.0 to 0.10.0 (#1361) | 2 files: Cargo.lock, crates/nono-cli/Cargo.toml |
| 6118b79aeda1365da213d85457b4d3cf1201d575 | chore: release v0.67.1 (#1353) | 6 files: CHANGELOG.md, Cargo.lock, bindings/c/Cargo.toml, crates/nono-cli/Cargo.toml, crates/nono-proxy/Cargo.toml, crates/nono/Cargo.toml |
| 19b43f300b9d2e257e161b808440f0573abfd422 | chore: release v0.67.0 (#1350) | 6 files: CHANGELOG.md, Cargo.lock, bindings/c/Cargo.toml, crates/nono-cli/Cargo.toml, crates/nono-proxy/Cargo.toml, crates/nono/Cargo.toml |
| 2af5495cda0fc0067360c4b99ffdebc8e3c68755 | chore(deps): bump h2 from 0.4.14 to 0.4.15 (#1312) | 1 file: Cargo.lock |
| cf7a40392a0011e22c4d11bdaf4b16add46a9e55 | chore(deps): bump webpki-roots from 1.0.7 to 1.0.8 (#1311) | 1 file: Cargo.lock |
| d8187fb17c44869bdd4bd9670f0070320f7c10f4 | chore(deps): bump rustls from 0.23.40 to 0.23.41 (#1310) | 1 file: Cargo.lock |
| b8f00cefd811ff3b950314963c85f4456df0206c | chore(deps): bump time from 0.3.49 to 0.3.51 (#1307) | 1 file: Cargo.lock |
| 4b889c953f2473f5a39a69126fc17eacc50c000e | chore(deps): bump jsonschema from 0.46.5 to 0.46.6 (#1306) | 1 file: Cargo.lock |

### CI bucket (11 commits)

| sha | subject | touched-path-summary |
|-----|---------|----------------------|
| bffe8134ee4b68752dc1b2ef514a8ae58f3b6123 | chore(deps): bump docker/setup-buildx-action from 4.1.0 to 4.2.0 (#1359) | 1 file: .github/workflows/image-build.yml |
| cbe9bdc28f842157c95cbb6ff2983ab0797bda32 | chore(deps): bump docker/login-action from 4.2.0 to 4.4.0 (#1357) | 1 file: .github/workflows/image-build.yml |
| 2485430a0ac821bffda784a5cea8590f2fca5efb | chore(deps): bump nolabs-ai/agent-sign from 0.0.11 to 0.1.0 (#1355) | 1 file: .github/workflows/sign-instruction-files.yml |
| 168efa2bac7f0dedddcb43a089e15f1022dc0f17 | chore(deps): bump docker/setup-qemu-action from 4.1.0 to 4.2.0 (#1354) | 1 file: .github/workflows/image-build.yml |
| 3fc3743dd1d551a91f54f075f683a30b32d7d67a | chore(deps): bump docker/build-push-action from 7.2.0 to 7.3.0 (#1356) | 1 file: .github/workflows/image-build.yml |
| 002fe4986e9b03c61d0a90d78bbd535108a9b698 | fix(release): strip ./ prefix from SHA256SUMS.txt entries (#1352) | 1 file: .github/workflows/release.yml |
| a0a279079bba8057085d45503dee681a51db99f6 | chore(deps): bump actions/attest from 4.1.0 to 4.1.1 (#1309) | 2 files: .github/workflows/attest-release.yml, .github/workflows/release.yml |
| ade1bb128b2e2d28054bfe62aa44abd0cf52b416 | chore(deps): bump actions/cache from 5.0.5 to 6.1.0 (#1308) | 1 file: .github/workflows/ci.yml |
| 40080d04991cc28cd9f99d409dd42b2c4bcbe45b | chore(ci): remove homebrew bump workflow (#1294) | 2 files: .github/workflows/homebrew-bump.yml, .github/workflows/release.yml |
| 92ef76ffa5fd5ead3bbaf19ac5a928a48b63ae03 | ci: run actionlint (#1273) | 3 files: .github/workflows/ci.yml, .github/workflows/image-build.yml, .github/workflows/release.yml |
| 4fd9144525c4b748bd78a5c861678957ac5c776b | chore(ci): refine automation workflow (#1292) | 2 files: .github/workflows/label-new-issues.yml, .github/workflows/project-automation.yml |

### DOCS bucket (8 commits)

| sha | subject | touched-path-summary |
|-----|---------|----------------------|
| 55fd3825c4ea9471a207313f74aa71e6e4a96014 | docs(profiles): fix codeblock (#1426) | 1 file: docs/cli/features/profiles-groups.mdx |
| 99b8fa4fbe7da26c332e69e53f44fd84c39b4eb9 | docs(codex): clarify codex docs around the optional login-shell hardening (#1381) | 1 file: docs/cli/clients/codex.mdx |
| d74063335e8584f9667496e25967e1c87e1d38e1 | fix: use permanent community link across project (#1349) | 5 files: .github/ISSUE_TEMPLATE/config.yml, CONTRIBUTING.md, MAINTAINERS.md, README.md, docs/docs.json |
| 2f547703517d0c5436f57aad341a3a51af18e116 | docs: add community health files (#1348) | 6 files: .github/ISSUE_TEMPLATE/config.yml, CODE_OF_CONDUCT.md, CONTRIBUTING.md, CONTRIBUTORS.md, GOVERNANCE.md, MAINTAINERS.md |
| 73952cd4e1762a835b47b5f4c23c7abd030fb0f9 | docs(readme): explain tool sandboxing for agents (#1342) | 1 file: README.md |
| 2375aeb6adb81d1d896336328cf61e04b2a4b19e | docs(profiles): clarify predefined vs user profiles scope (#1331) | 1 file: docs/cli/features/profiles-groups.mdx |
| 63c9589fa30f0244c542c026faf27d517aaa9b39 | docs(credential-injection): document AWS SigV4 proxy signing (#1329) | 1 file: docs/cli/features/credential-injection.mdx |
| da3a90f247bf80ffe8bcc470154e83f764033803 | docs(nogent): add nogent markdown file (#1288) | 1 file: NOGENT.md |

---

## Cluster Summary

Every CODE-bucket SHA (62 total) is assigned to exactly one of 5 clusters. `disposition`,
`windows-touch`, and `security-relevant` are **finalized below (Plan 108-05)** — rolled up from
the per-commit tables in Plans 108-03/108-04, plus a grep-verified `cfg(windows)` check and a
reasoned security-relevant classification for the two clusters (tool-sandbox-surface,
security-residual-and-misc) whose 108-04/108-01 per-commit tables did not carry an explicit
windows-touch/security-relevant column. Full derivation and per-commit support data for those two
clusters is in "Cluster Summary Rollup Support Notes" immediately below the table. All prior
placeholder cells in this table have now been replaced with finalized values.

**Arithmetic check:** NET 12 + PROF 8 + CORE 4 + tool-sandbox-surface 20 +
security-residual-and-misc 18 = **62 = CODE bucket total** (verified above).

| cluster_id | theme | commit_count | disposition | windows-touch | security-relevant | phase-target |
|------------|-------|--------------|-------------|----------------|--------------------|---------------|
| NET | proxy/network (deny_domain, SPIFFE, SigV4, sibling-route, no_proxy, HTTP_PROXY, credential-capture plumbing) | 12 | will-sync (12/12 commits — 11 adopt + 1 adapt: `3b207eeb` #1374 `deny_domain`, per ADR-108) | no (0/12 — grep-confirmed, zero `cfg(target_os = "windows")`/`cfg(windows)` hits, per Plan 108-03's NET table) | yes (9/12, per Plan 108-03's NET table security-relevant column) | 109 |
| PROF | profile/policy (platform_overrides, extends, $VAR tokens, bun/mise presets, inheritable-field save) | 8 | will-sync (8/8 commits, all adopt) | yes (2/8 — `ae1c513e`, `719975cf`; cited runtime `Os::Windows` match-arm evidence in Plan 108-03's PROF table, not a `cfg(windows)` compile gate) | yes (6/8, per Plan 108-03's PROF table security-relevant column) | 110 |
| CORE | macOS/resource-CLI (~/.cache fix, MAX_CRYPTO_THREADS, resource limiting, --max-processes) | 4 | split (2/4 adopt — macOS-only carry, no fork Windows counterpart to reconcile; 2/4 adapt — CORE-02 requires reconciling with the fork's existing Job Object mechanism) | no (0/4 — zero Windows mentions of any kind in any of the 4 diffs, per Plan 108-03's CORE table finding) | yes (3/4, per Plan 108-03's CORE table security-relevant column) | 111 |
| tool-sandbox-surface | tool-sandbox subsystem (D-05/D-06 3-path union; per-commit pure/split residue accounting in Plan 108-04) | 20 | DEFERRED->v3.7 (20/20 commits — 9/20 pure fully deferred; 11/20 split, module-set-scoped paths deferred while non-module absorb-worthy residue is routed to PROF-02/PROF-03/CORE-cluster-residual per the Plan 108-04 residue tables; 2 of the 20 — `d5803b99` PROF-03, `d4927f95` PROF-02 — carry an actual v3.6 requirement mapping via that residue routing even though the commit itself stays DEFERRED->v3.7) | no (0/20 — Plan 108-05 grep-verified: zero `cfg(target_os = "windows")`/`cfg(windows)` hits across all 20 diffs; one incidental case-insensitive `windows` string match in `42161620` is `.windows(N)`, Rust's slice-windowing method, a false positive — see Rollup Support Notes) | yes (16/20 — Plan 108-05 reasoned classification per commit, see Rollup Support Notes below; the 4 remaining are build/CLI/config fixes with no sandbox-boundary implication) | v3.7 DEFERRED (per-commit split in Plan 108-04) |
| security-residual-and-misc | security-relevant + unrouted residual commits with no v3.6 requirement home (D-18) | 18 | DEFERRED->proposed Phase 112 (18/18 commits pending operator approval — no adopt/adapt/skip/split disposition assigned since no v3.6 requirement maps to this cluster; see Requirement Coverage Gap section) | no (0/18 — Plan 108-05 grep-verified, zero `cfg(target_os = "windows")`/`cfg(windows)` hits across all 18 diffs) | yes (11/18 — the 10 D-18-named anchors + `f6f027511f` #1301 credential-guard relaxation; the remaining 7/18 are registry/update-check/test-only residual commits with no security-boundary implication — see Rollup Support Notes) | proposed Phase 112 pending operator approval |

### Cluster Summary Rollup Support Notes

**NET/PROF/CORE rollups** are a direct roll-up of Plan 108-03's per-commit `windows-touch` and
`security-relevant` columns (counted directly from the tables in "NET/PROF/CORE Cluster — Per-Commit
Table" above) — no new verification was needed for these 3 clusters. All prior placeholder cells
in those 3 rows are now replaced with finalized values.

**tool-sandbox-surface and security-residual-and-misc** did not receive an explicit per-commit
`windows-touch`/`security-relevant` column in Plans 108-01/108-04 (those tables carry
disposition/PR#/residue markers instead, per D-07's own accounting scheme). Plan 108-05 closes this
gap as follows, so the Cluster Summary row above is traceable rather than asserted:

**windows-touch, both clusters (38 commits total):** every SHA in both clusters was re-run through
`git show <sha> | grep -icE 'cfg\(target_os = "windows"\)|cfg\(windows\)'`; **all 38 returned 0.**
A secondary case-insensitive `grep -ic windows` sweep (looking for any Windows mention at all, cfg
or otherwise — the same style of check Plan 108-03 used for the CORE cluster) found exactly one
hit, in `42161620ec68da9c4b09b2283e6584ecbf51d3a2` (tool-sandbox-pure): `shim_args.windows(expected_args.len())`
— Rust's `[T]::windows(size)` slice-iteration method, not an OS reference. Confirmed a false
positive by reading the line in context. **Result: windows-touch: no for all 20 tool-sandbox-surface
commits and all 18 security-residual-and-misc commits.**

**security-relevant, tool-sandbox-surface (20 commits) — reasoned per-commit classification:**

| sha | subject | security-relevant | rationale |
|-----|---------|:---:|-----------|
| `a519ee62` | attribute daemonized callers to their command (#1417) | yes | policy-enforcement attribution for daemonized command execution |
| `cafc39eb` | allow exec in writable grant-dirs under command policies (#1391) | yes | widens an exec grant under sandbox command policy |
| `d5803b99` | port range support (#1398) | yes | new kernel-enforced network capability (D-05 worked example) |
| `ab93cf44` | preserve argv[0] for symlink-dispatched commands (#1413) | yes | symlink-dispatch exec-identity handling inside the sandbox |
| `bf7ea3cb` | grant env-shebang scripts their re-exec interpreter (#1394) | yes | sandbox exec grant for re-exec'd interpreters |
| `72a98830` | per-command exec_paths for multi-call binaries (#1384) | yes | exec-path allow-listing mechanism |
| `1f54f4ae` | resolve command_policies binaries once, parallel+cached (#1373) | no | performance/caching fix, no policy-boundary change |
| `7c20dc75` | resolve command policy paths against the live cwd (#1339) | yes | path-resolution correctness bug that affects which policy matches |
| `42161620` | match intercept args after global options (#1344) | no | CLI argument-parsing bug fix |
| `eb2d61a7` | add exec intercept action (#1322, named) | yes | new interception/mediation mechanism |
| `5a7447d3` | strip untrusted unsafe_macos_seatbelt_rules before emission | yes | sanitizes untrusted input before Seatbelt-profile emission (injection risk) |
| `ebd51cbb` | warn on unsafe_macos_seatbelt_rules nested in sandboxes | yes | same untrusted-Seatbelt-rule theme as `5a7447d3` |
| `e2c87fd5` | per-command unsafe_macos_seatbelt_rules escape hatch | yes | introduces a sandbox escape-hatch mechanism |
| `052b8374` | per-intercept sandbox override | yes | sandbox-override mechanism |
| `ea334d2b` | musl build fix (#1332, D-05 worked example) | no | build-target compatibility fix, no security-boundary change |
| `d4927f95` | @git:* dynamic tokens in filesystem paths (#1298, D-05 worked example) | yes | path-expansion into filesystem grants — can widen access |
| `676a042f` | ack frame before SCM_RIGHTS send (#1325, named) | yes | file-descriptor-passing (SCM_RIGHTS) correctness/security |
| `199bb266` | add git worktree tokens (#1280, named) | yes | same path-expansion theme as `d4927f95` |
| `de8a54b2` | run git config from repo root (#1313) | no | git-config-resolution correctness fix, not a sandbox-boundary change |
| `8a4237f2` | SeccompPolicy struct refactor (#1283, D-05 worked example) | yes | sandbox-enforcement-selection mechanism (highest cross-crate surface in this window) |

Tally: 16 yes, 4 no (`1f54f4ae`, `42161620`, `ea334d2b`, `de8a54b2`) = 20. ✓

**security-relevant, security-residual-and-misc (18 commits):** the 10 D-18-named anchors are
security-relevant by construction (D-18 explicitly names them as the security-relevant subset).
Of the 8 "additional" residual commits, one was hand-checked and found security-relevant:
`f6f027511f7899f1735f1188747086696e40412e` (#1301, "allow env_credentials + command_policies on
non-shim entry") removes a hard guard that previously blocked brokered-credential injection
outside a policy shim — diff-verified (`git show f6f027511f`) to touch the credential-broker
nonce-injection path directly. The remaining 7 additional commits (`f050643479` registry-namespace
migration, `7fe0c8283810`/`0158d52f0a`/`762eb05bd3d` registry/update-check diagnostic headers,
`503045801a` PTY-teardown bug fix, `4cc0af2c52`/`9840a16f35` test-only) have no sandbox/credential/
network-boundary implication. Tally: 11 yes (10 named + `f6f027511f`), 7 no = 18. ✓

### NET cluster commits (12)

**Cross-reference note (PROF rationale):** `d5803b994b416ad07a73907143ca169c408917f3` (#1398,
port-range) and `d4927f95a37863cf0ba534b054e28f48f002ad21` (#1298, `@git:*` tokens) are
thematically PROF (profile-schema features) but are filed under **tool-sandbox-surface** per the
plan's explicit instruction, since both also touch the tool-sandbox module set (D-05/D-06). Their
split/residue accounting happens in Plan 108-04, not here.

| sha | subject |
|-----|---------|
| 3b207eeb884bd71b0fc10f0123fa89fffe9f7955 | feat(proxy): add deny_domain to block domains through the proxy (#1374) |
| c831dade422f2bdf37d7429af0423cafa0a60c06 | feat(proxy): add SPIFFE/SPIRE workload identity auth for upstream routes (#1272) |
| 6fb7ecbf36e5d18760d8084b7f4900e93582004c | bug: Fix SigV4 URI generation errors for uri's that have encoded characters in them (#1430) |
| 23d93fc96abf795d672c712e9c1834a8f97aa0aa | fix(proxy): don't cross-deny sibling routes sharing an upstream (#1437) |
| 1619275caa32b7b96e3eee56c33c71dfce777bbf | feat: add profile-declared no_proxy bypass support (#1415) |
| 726ac1f1b5fd7b6de2d86b9fccf1d72660d3e32a | feat(proxy): support plain HTTP forward-proxying via HTTP_PROXY (#1335) |
| c344efb006365ba596b20b843f29fabdbbdc847e | fix(why): respect proxy domain filter in --profile and --self host queries (#1372) |
| 4192bfa58101d0ef9737b60243a39695fc1e0fa8 | fix(proxy): skip credential_capture entries with missing helper binaries (#1368) |
| 261bbd68d984f58c5b7376143fdacbeaffef1ccc | fix(tests): raise credential-capture test timeout to reduce macOS CI flakiness |
| 3672ea10c7d45b340bc389e1d5cf0a26dfa5db34 | fix(tests): share stdin-manipulation lock between capture_helper stdin tests (#1327) |
| 8255a27a1d8bedf878a7c8c67f9007ca592962d1 | refactor load_with_diagnostics to be async (#1287) |
| 7d23bba683036789163385afa7f5c2f5888886a5 | fix(proxy): separate stdin and stderr inheritance for credential helpers (#1300) |

### PROF cluster commits (8)

**Cross-reference note:** see NET cluster note above — `d5803b99`/`d4927f95` are thematically PROF
but filed under tool-sandbox-surface.

| sha | subject |
|-----|---------|
| ae1c513e6ed1f1a0628dbb690c3f1c984fb7a225 | feat(profile): add platform_overrides field for per-OS profile patches (#1371) |
| 719975cf03ea303f835061089279eb6c50245ccf | fix(profile): preserve platform_overrides through extends resolution (#1380) |
| 2cbaa9a017b14ddc45219474b6eebf94727787ef | feat(profile): expand $VAR tokens from process env in profile paths and capture commands (#1296) |
| b620ed8e4572359afa4ed9cff4d803c8e9064f26 | feat(policy): add bun runtime preset (#1305) |
| f016b2d56dcbe4aad2bb322acd7574d38b5c13f7 | mise policy (#1387) |
| 0374e454b9424e8b77c46fe0948a4e109d37b8da | fix(profile): omit inheritable Option fields when None on save (#1400)(#1402) |
| 9ef5918169d719821aaec757a9eea715c3e8c44d | docs(cli/profile): simplify credential provider def doc comment |
| f58c7c2242d3790f9c390d4bf0dd808ba1261c77 | feat(profile): support CLI profile extends (#1320) |

### CORE cluster commits (4)

| sha | subject |
|-----|---------|
| ca888108fe5983be866e8d1a6eccf96edc2a8dd5 | fix: missing ~/.cache on macOS (#1378) |
| 099237da94d33752ca5617db3f4902a216f9f84c | fix(exec): raise MAX_CRYPTO_THREADS to 12 for macOS libdispatch workqueue threads (#1424) |
| e6d26871f0498e7dc7a867e67af5c6136b84f91c | feat: resource limiting (#1269) |
| 34c2c975d649844923cf1515be94de624c689c6c | feat(resources): cap sandbox process count with --max-processes (#1403) |

### tool-sandbox-surface cluster commits (20)

Per D-05/D-09, this is the raw 3-path-union surface (not subdivided pure/split — Plan 108-04's
job). Includes the 4 D-05 worked-example commits (`d5803b99`, `d4927f95`, `ea334d2b`, `8a4237f2`)
proving PR-level deferral is unsafe, plus the 2 commits (`5a7447d3`, `ebd51cbb`) the D-06
re-measurement above shows a directory-only filter would silently drop.

| sha | subject |
|-----|---------|
| a519ee62e6d571330caa6a9a7d033fb10bff5939 | fix(tool-sandbox): attribute daemonized callers to their command (#1417) |
| cafc39eb3104e789cee2b663dc04b8aa7ef0062a | fix(sandbox): allow exec in writable grant-dirs under command policies (#1391) |
| d5803b994b416ad07a73907143ca169c408917f3 | feat: add port range support to sandbox profiles (#1398) |
| ab93cf44e5706b49b1aa5f8ecc1b3b59f29596f8 | fix(tool-sandbox): preserve argv[0] for symlink-dispatched commands (#1413) |
| bf7ea3cb7faa241de13fe549152e28333b1f0068 | fix(tool-sandbox): grant env-shebang scripts their re-exec interpreter (#1394) |
| 72a988309974f426f0da8f507a4f05df9738ce92 | feat(tool-sandbox): add per-command exec_paths for multi-call binaries (#1384) |
| 1f54f4ae91870de02c1fabfda554278d49c99bd9 | fix(command-policy): resolve command_policies binaries once, in parallel, with caching (#1373) |
| 7c20dc75f4f155d64601719437980a0e232c8bd8 | fix(tool-sandbox): resolve command policy paths against the live cwd (#1339) |
| 42161620ec68da9c4b09b2283e6584ecbf51d3a2 | fix(cli): match intercept args after global options (#1344) |
| eb2d61a7abc5a354b8f8085b782cd11afa6babb8 | feat(tool-sandbox): add exec intercept action (#1322) |
| 5a7447d3ed30835bd9bd647b7812ee18ea80a155 | fix(tool-sandbox): strip untrusted unsafe_macos_seatbelt_rules before emission |
| ebd51cbb9546aec872136301249d048074a05e38 | fix(tool-sandbox): warn on unsafe_macos_seatbelt_rules nested in command/intercept sandboxes |
| e2c87fd5e82b1dd7c1cf9a70d9632f2b0310afb4 | feat(tool-sandbox): per-command unsafe_macos_seatbelt_rules escape hatch |
| 052b8374d2381ca4efdc6a51d9eee54d292a54da | feat(tool-sandbox): per-intercept sandbox override |
| ea334d2bbdcb332c3a1c4164843667b0d2153cb1 | fix(linux): use u64 for fs_type_unsupported to fix musl build (#1332) |
| d4927f95a37863cf0ba534b054e28f48f002ad21 | feat(profile): expand @git:* dynamic tokens in top-level filesystem paths (#1298) |
| 676a042f3c1e3877f1d1770abd19b0fea0345750 | fix(tool-sandbox): ack frame before SCM_RIGHTS send to prevent EMSGSIZE on macOS (#1325) |
| 199bb26699661e8a36de141adc4767617e0a1036 | feat(tool-sandbox): add git worktree tokens; fold include-files into @git:config-files (#1280) |
| de8a54b25c3c26f68f6d51f768be904cee0d0c41 | fix(dynamic-providers): run git config from repo root to honour hasconfig: includeIf (#1313) |
| 8a4237f2ee0dc33bc1e5afdd39c0db8ca6f5ed38 | refactor(seccomp): introduce SeccompPolicy struct and client-driven selection (#1283) |

### security-residual-and-misc cluster commits (18)

The 10 named security-relevant anchor SHAs from `108-CONTEXT.md` D-18 appear first (verbatim),
followed by 8 additional CODE-bucket commits with no NET/PROF/CORE/tool-sandbox-surface home.

| sha | subject | anchor? |
|-----|---------|---------|
| 0ecc476bf0db3c9509bcc2bd8efae43832c764b8 | feat: implement aws authentication for the MiTM proxy (#1195) | named (D-18) |
| 9b692e07ee4dd156d85588b12b63691778d9e9be | feat(oauth): add declarative sandboxed OAuth capture | named (D-18) |
| 3c59c62e7ddcc2d3a1f1c161bd039b5f2b0b5f7b | fix(oauth): harden capture security boundaries | named (D-18) |
| d033c63111472711e242f4067eb4be04aeaf618a | test(oauth): consume provider stdin in header fixture | named (D-18) |
| a32439074a61eea21eb374a91fdbcebb91b0afe6 | feat(gpu): harden NVIDIA procfs mediation (#1284) | named (D-18) |
| f943fb5a0721352e85d92ef48e3e4d5b36747486 | fix(trust): add predicate field to distinguish nono trust policies from foreign JSON (#1333) | named (D-18) |
| d84b4818f824b7660d1a4d95ec2e56f322b6fa63 | fix(sandbox): grant Refer in execute-restriction layer on Linux (#1397) | named (D-18) |
| ac5ccd70712782f4a97c54916ee18b03c650f56f | fix(sandbox): keep orphaned descendants in supervisor ancestry for seccomp-notify mediation (#1401) | named (D-18) |
| 2663e9900fe1317cec0949c18b7a0e6b39321602 | feat(cli): add standalone `nono proxy` command (#1261) | named (D-18) |
| a5a441c25769ecec663aed1d7038ab2b8814d43f | fix(profile): empty allow_vars no longer strips all env vars (#1204) | named (D-18) |
| f050643479d0dab8c86d7322a9a283a4f2d64322 | chore: migrate registry namespace references from always-further to nolabs-ai (#1405) | additional |
| 7fe0c8283810c4ca8c246d1b651f3f31578fdf55 | fix(registry): add X-Nono-Pull-Reason header to distinguish pull triggers (#1383)(#1386) | additional |
| 0158d52f0a2a7542993ae48fda664ea7ce3359ff | feat(registry-client): attach installation context headers to registry requests (#1341) | additional |
| 762eb05bd3d00557228b6ba0fa8e9bb21ef13cfb | feat(update-check): emit install_source on update check requests (#1340) | additional |
| 503045801a9c54e7b1f5e4b6004766e7f9317b42 | fix(pty): drain late terminal query reply on teardown (#1258) | additional |
| f6f027511f7899f1735f1188747086696e40412e | fix(execution-runtime): allow env_credentials + command_policies on non-shim entry (#1301) | additional |
| 4cc0af2c52119c3794843416b292c2ed88d22c8c | fix(tests): use /tmp for socket test dirs to stay under SUN_LEN limit (#1303) | additional |
| 9840a16f354f9bdd79ed504b56467e1743a7e609 | test: suppress save prompt in socket access tests (#1279) | additional |

---

## Downstream Notes (non-normative — full disposition work is Plans 108-03/04/05)

- **ADR-108** (D-10/D-11/D-12) settles the `deny_domain` (#1374, in NET cluster) posture
  separately; this ledger cross-references it but does not duplicate the analysis.
- **Carve-out re-touch check** (D-22) and **per-commit requirement mapping** (D-21) are explicitly
  out of scope for this plan (108-01) — they land in later plans in this phase.
- **D-19 note carried forward:** the security-residual-and-misc cluster (18 commits, phase-target
  "proposed Phase 112") is exactly the kind of unmapped cluster D-19 anticipates. This ledger
  records it; the roadmap amendment proposal itself is Plan 108-05's deliverable, subject to
  operator approval before Phase 109 planning begins.

---

## NET Cluster — Per-Commit Table (Plan 108-03)

Hand-verified per D-21 (read the actual `git show <sha>` diff, not the commit subject) against
`NET-01`/`NET-02`/`NET-03` (`.planning/REQUIREMENTS.md`). Re-export scan command run per row:
`git show <sha> | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'`.
`git show <sha> | grep -c 'cfg(target_os = "windows")\|cfg(windows)'` confirms windows-touch for
every row below returned **0** — the NET cluster (proxy/`nono-proxy` crate) is not
Windows-cfg-gated anywhere in this window; all 12 rows are `windows-touch: no`.

**Row count check:** 12 rows below == NET cluster `commit_count` (12) from Plan 108-01's Cluster
Summary. No additional commits found by re-running the re-sweep (`git log --no-merges --oneline
$RANGE -- crates/nono-proxy/src/ crates/nono-cli/src/network_policy.rs` reproduces the same 12
plus the tool-sandbox-filed `d5803b99`/`d4927f95`, which stay out of this table per the
Cross-reference note above).

| sha | subject | files-changed | windows-touch | security-relevant | requirement-mapping | disposition | re-export scan |
|-----|---------|----------------|:---:|:---:|:---:|-----------|-----------------|
| `3b207eeb884bd71b0fc10f0123fa89fffe9f7955` | feat(proxy): add deny_domain to block domains through the proxy (#1374) | 15 files (cli.rs, launch_runtime.rs, main.rs, network_policy.rs, profile/mod.rs, profile_runtime.rs, proxy_command.rs, proxy_runtime.rs, sandbox_prepare.rs, nono-proxy/config.rs, filter.rs, server.rs, nono/net_filter.rs, +2 docs), 311+/26- | no | yes | **NET-01** | see ADR-108 (`proj/ADR-108-deny-domain-posture.md`) — ADAPT | `pub(crate) deny_domain: Vec<String>` field ×4 (intra-CLI/proxy structs); no cross-crate `pub mod`/`pub use` |
| `c831dade422f2bdf37d7429af0423cafa0a60c06` | feat(proxy): add SPIFFE/SPIRE workload identity auth for upstream routes (#1272) | 33 files (`.github/workflows/spire.yml`, `nono-proxy/src/{auth,spiffe,route,reverse,credential,oauth2,server,config}.rs`, `tls_intercept/{h2_forward,handle}.rs`, `tests/spiffe_{run,integration}.rs`, `nono-cli/src/network_policy.rs`+`profile/mod.rs`+`proxy_runtime.rs`, `nono/src/{audit.rs,undo/types.rs}`, docs/scripts/testdata), 4354+/545- | no | yes | **NET-02** | adopt | `pub mod auth;` `pub mod spiffe;` (new intra-`nono-proxy` modules) + several `pub(crate)` helper fns (`filter_headers_multi`, `is_loopback_host_port`, `add_ca_file_to_store`); no re-export crossing into `nono` core or `nono-cli` |
| `6fb7ecbf36e5d18760d8084b7f4900e93582004c` | bug: Fix SigV4 URI generation errors for uri's that have encoded characters in them (#1430) | 1 file (`nono-proxy/src/aws/sign.rs`), 16+/4- | no | yes | **NET-03** | adopt | Clean — diff confined to `aws/sign.rs` internal URI-encoding logic; no new `pub` items |
| `23d93fc96abf795d672c712e9c1834a8f97aa0aa` | fix(proxy): don't cross-deny sibling routes sharing an upstream (#1437) | 1 file (`nono-proxy/src/tls_intercept/handle.rs`), 87+/0- | no | yes | **NET-03** | adopt | Clean — hand-verified: diff modifies `select_intercept_route`'s `EndpointPolicyOutcome::Deny` arm to treat a non-explicit default-deny as "route does not apply" instead of a terminal 403, exactly the sibling-route cross-deny logic NET-03 names; adds a regression test, no new `pub` surface |
| `1619275caa32b7b96e3eee56c33c71dfce777bbf` | feat: add profile-declared no_proxy bypass support (#1415) | 17 files (schema.json, profile-authoring-guide.md, launch_runtime.rs, main.rs, profile/{mod,cmd,runtime}.rs, proxy_runtime.rs, sandbox_prepare.rs, nono-proxy/{config,filter,route,server}.rs, tls_intercept/handle.rs, +3 docs), 2065+/99- | no | yes | **NET-03** | adopt | `pub(crate) no_proxy: Vec<String>` field ×3 + `pub(crate) fn` helpers (`validate_no_proxy_allow_domain_conflicts`, `no_proxy_host_pattern_matches`, `strip_no_proxy_port`, `normalise_no_proxy_host_pattern`, `normalise_no_proxy_env_entry`, `parse_host_ip_literal`, `is_proxy_denied_metadata_ip`, `extract_host_port`, `format_host_port`) — all intra-crate `pub(crate)`, no cross-crate re-export |
| `726ac1f1b5fd7b6de2d86b9fccf1d72660d3e32a` | feat(proxy): support plain HTTP forward-proxying via HTTP_PROXY (#1335) | 1 file (`nono-proxy/src/server.rs`), 815+/5- | no | yes | **NET-03** | adopt | Clean — single-file `server.rs` addition, no new `pub` items in the diff |
| `c344efb006365ba596b20b843f29fabdbbdc847e` | fix(why): respect proxy domain filter in --profile and --self host queries (#1372) | 2 files (`execution_runtime.rs`, `query_ext.rs`), 131+/13- | no | yes | none | adopt | Clean — hand-verified: diagnostic-command bugfix (`nono why --profile`/`--self`) making the `why` reporter consult the same `HostFilter`/`network_policy` resolution the proxy already enforces; does not itself implement any NET-01/02/03-named mechanism, so mapped `none` rather than force-mapped to NET-01 |
| `4192bfa58101d0ef9737b60243a39695fc1e0fa8` | fix(proxy): skip credential_capture entries with missing helper binaries (#1368) | 1 file (`nono-cli/src/proxy_runtime.rs`), 263+/16- | no | yes | none | adopt | Clean — includes a portable path-separator security fix (`resolve_capture_command` previously checked only the native `MAIN_SEPARATOR`, letting a relative command containing the non-native separator bypass bare-command validation on Windows); credential_capture plumbing, not a NET-01/02/03-named item |
| `261bbd68d984f58c5b7376143fdacbeaffef1ccc` | fix(tests): raise credential-capture test timeout to reduce macOS CI flakiness | 1 file (`nono-cli/src/proxy_runtime.rs`), 1+/1- | no | no | none | adopt | Clean — test-only timeout constant change |
| `3672ea10c7d45b340bc389e1d5cf0a26dfa5db34` | fix(tests): share stdin-manipulation lock between capture_helper stdin tests (#1327) | 1 file (`nono-cli/src/proxy_runtime.rs`), 7+/5- | no | no | none | adopt | Clean — test-only lock-sharing fix |
| `8255a27a1d8bedf878a7c8c67f9007ca592962d1` | refactor load_with_diagnostics to be async (#1287) | 4 files (`credential.rs`, `oauth2.rs`, `server.rs`, `tls_intercept/h2_forward.rs`), 67+/51- | no | no | none | adopt | Clean — mechanical `fn` → `async fn` signature conversion (`pub fn` becomes `pub async fn`, not a new `pub` item), no behavior change per commit message |
| `7d23bba683036789163385afa7f5c2f5888886a5` | fix(proxy): separate stdin and stderr inheritance for credential helpers (#1300) | 5 files (schema.json, profile-authoring-guide.md, profile/mod.rs, proxy_runtime.rs, credential-injection.mdx), 190+/6- | no | yes | none | adopt | Clean — hand-verified: fixes a terminal-keypress-theft security bug (credential helpers inheriting stdin unintentionally) by splitting `interaction.stdio` into separate `stdio`(stderr)/`stdin` schema keys; not a NET-01/02/03-named item |

**NET disposition summary:** 1 ADAPT (`3b207eeb` #1374, per ADR-108) + 11 adopt. All 12 rows map
to exactly one of `NET-01`/`NET-02`/`NET-03`/`none` (5 map `none` — diagnostic/test/refactor/
credential-plumbing commits that ride alongside the network absorb but do not themselves
implement a named NET-0X mechanism). Zero blank cells. `windows-touch: no` for all 12 (grep-
confirmed, zero `cfg(target_os = "windows")`/`cfg(windows)` hits in any of the 12 diffs).

---

## PROF Cluster — Per-Commit Table (Plan 108-03)

**Row-count reconciliation note:** Task 2's action text names 5 PROF commits for full-row
treatment (`ae1c513e`, `719975cf`, `2cbaa9a0`, `b620ed8e`, `f016b2d5`) plus the 2 tool-sandbox
pointer rows (`d5803b99`, `d4927f95`) — 7 rows. Plan 108-01's own Cluster Summary, however,
already established **8** confirmed PROF-cluster members (the 5 above plus `0374e454`,
`9ef5918169`, `f58c7c2242`). Per this phase's own re-verification discipline (D-04/D-21 —
"record the disagreement explicitly rather than silently adopting either"), omitting 3
already-confirmed cluster members from the per-commit table would leave them with no
hand-verified requirement mapping, contradicting this plan's own must-have truth ("every
NET/PROF/CORE cluster commit has a hand-verified requirement mapping"). Resolution: **all 8**
Cluster-Summary-confirmed PROF commits are given full rows below, plus the 2 required pointer
rows — **10 rows total**, not 7. This is recorded as a plan/data discrepancy (Rule 1 — a table
that silently drops established cluster members is a correctness bug), not a silent deviation.

Hand-verified per D-21 against `PROF-01`/`PROF-02`/`PROF-03`/`PROF-04`
(`.planning/REQUIREMENTS.md`). Re-export scan command: `git show <sha> | grep '^+' | grep -E
'^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'`.

**`ae1c513e` windows-touch evidence (cited, per Task 2 requirement):** `git show ae1c513e | grep
-in windows` returns 5 hits, including `+    pub windows: Option<Box<PlatformOverride>>,` (new
struct field) and `+            crate::platform::Os::Windows => self.windows,` (runtime
platform-match arm) — this is the exact field the fork's `windows_low_il_broker`/
`windows_interpreters` migration (PROF-01) will populate. No `cfg(target_os = "windows")`/
`cfg(windows)` compile-time gate exists in the diff (0 hits) — the Windows-relevance is a
runtime `Os::Windows` match arm on a cross-platform enum, not a cfg-gated code path. **Windows-
touch: yes**, on the strength of this cited grep evidence, not asserted without it.

**`719975cf` windows-touch evidence:** `git show 719975cf | grep -in windows` returns 6 hits,
including `+            windows: merge_platform_override_slot(b.windows, c.windows),` (the
extends-merge logic PROF-01 requires to preserve `platform_overrides` through inheritance) and
two test assertions (`po.windows.is_some()`, `windows: Some(Box::new(...))`). **Windows-touch:
yes**, cited.

| sha | subject | files-changed | windows-touch | security-relevant | requirement-mapping | disposition | re-export scan |
|-----|---------|----------------|:---:|:---:|:---:|-----------|-----------------|
| `ae1c513e6ed1f1a0628dbb690c3f1c984fb7a225` | feat(profile): add platform_overrides field for per-OS profile patches (#1371) | 3 files (profile-authoring-guide.md, policy.rs, profile/mod.rs), 242+/0- | **yes** (see evidence above) | yes | **PROF-01** | adopt | Clean — new `pub struct PlatformOverrides`/`PlatformOverride` and `pub platform_overrides` field are all intra-`nono-cli`; no cross-crate `pub mod`/`pub use` |
| `719975cf03ea303f835061089279eb6c50245ccf` | fix(profile): preserve platform_overrides through extends resolution (#1380) | 1 file (profile/mod.rs), 294+/1- | **yes** (see evidence above) | yes | **PROF-01** | adopt | Clean — single-file `profile/mod.rs` change to `merge_profiles`/`merge_platform_override_slot`; no new `pub` surface |
| `2cbaa9a017b14ddc45219474b6eebf94727787ef` | feat(profile): expand $VAR tokens from process env in profile paths and capture commands (#1296) | 5 files (capability_ext.rs, policy.rs, proxy_runtime.rs, wiring.rs, credential-injection.mdx), 215+/68- | no | yes | **PROF-02** | adopt | `pub(crate) fn substitute_vars<E>(...)`, `pub(crate) fn expand_env_vars(s: &str) -> String` — intra-CLI, no cross-crate re-export |
| `b620ed8e4572359afa4ed9cff4d803c8e9064f26` | feat(policy): add bun runtime preset (#1305) | 2 files (policy.json, manifest_roundtrip.rs), 30+/0- | no | yes | **PROF-04** | adopt | Clean — pure `data/policy.json` embedded-policy addition + test; no Rust `pub` surface change |
| `f016b2d56dcbe4aad2bb322acd7574d38b5c13f7` | mise policy (#1387) | 2 files (policy.json, manifest_roundtrip.rs), 31+/0- | no | yes | **PROF-04** | adopt | Clean — same shape as `b620ed8e` |
| `0374e454b9424e8b77c46fe0948a4e109d37b8da` | fix(profile): omit inheritable Option fields when None on save (#1400)(#1402) | 1 file (profile/mod.rs), 47+/7- | no | no | none | adopt | Clean — hand-verified: fixes `Profile` struct save serialization for 7 `Option` fields (`environment`, `command_policies`, `open_urls`, `allow_launch_services`, `allow_gpu`, `allow_parent_of_protected`, `binary`), explicitly "matching `platform_overrides` (which already had it)" per the commit message — adjacent to the `ae1c513e` struct but does not itself implement `platform_overrides`; mapped `none` rather than force-mapped to PROF-01, though Phase 110 should land it alongside PROF-01 for round-trip correctness (Rule 2 candidate) |
| `9ef5918169d719821aaec757a9eea715c3e8c44d` | docs(cli/profile): simplify credential provider def doc comment | 1 file (credential_provider.rs), 1+/3- | no | no | none | adopt | Clean — pure doc-comment trim, no logic change |
| `f58c7c2242d3790f9c390d4bf0dd808ba1261c77` | feat(profile): support CLI profile extends (#1320) | 9 files (CHANGELOG.md, profile-authoring-guide.md, cli.rs, command_runtime.rs, learn.rs, profile/mod.rs, profile_runtime.rs, why_runtime.rs, flags.mdx), 418+/30- | no | yes | none | adopt | Clean — hand-verified: adds a **CLI** `--extends <PROFILE>` flag (prepends to the JSON `extends` list); NOT the profile-schema `extends` field itself, which the fork already carries independently (fork commit `91c3b1a0`, "feat: profile inheritance via `extends` field (#203)", predates this window — confirmed via `git log -S "pub extends: Option<Vec<String>>"`). No conflict: different surface (CLI flag vs. JSON field), composes with the fork's existing JSON-extends resolution path. The authoring guide itself flags "Inherited grants can widen sandbox permissions" — security-relevant, but implements no PROF-0X-named mechanism, so mapped `none` |

**Tool-sandbox-entangled PROF pointer rows** (Task 2 — cross-reference only, no duplicated
analysis; full split/residue accounting is Plan 108-04's job per D-05):

| sha | subject | requirement-mapping | disposition |
|-----|---------|----------------------|--------------|
| `d5803b994b416ad07a73907143ca169c408917f3` | feat: add port range support to sandbox profiles (#1398) | **PROF-03** | see tool-sandbox-split section (Plan 108-04) — entangled with tool-sandbox module set per D-05 |
| `d4927f95a37863cf0ba534b054e28f48f002ad21` | feat(profile): expand @git:* dynamic tokens in top-level filesystem paths (#1298) | **PROF-02** | see tool-sandbox-split section (Plan 108-04) — entangled with tool-sandbox module set per D-05 |

**PROF disposition summary:** 10 rows total (8 full + 2 pointer), all adopt (no adapt/skip/split
needed for the 8 full-analysis commits — none of them touch a fork carve-out surface). 4 rows map
`none` (`0374e454`, `9ef5918169`, `f58c7c2242` — adjacent/ergonomic commits with no PROF-0X-named
mechanism of their own). 2 rows (`ae1c513e`, `719975cf`) are `windows-touch: yes` on cited
`Os::Windows` runtime-match evidence, not a `cfg(windows)` compile gate. Zero blank cells.

---

## CORE Cluster — Per-Commit Table (Plan 108-03)

Hand-verified per D-21 against `CORE-01`/`CORE-02` (`.planning/REQUIREMENTS.md`). All 4 rows
below == CORE cluster `commit_count` (4) from Plan 108-01's Cluster Summary — no additional
commits found by re-running `git log --no-merges --oneline $RANGE -- crates/nono/src/sandbox/
macos.rs crates/nono-cli/src/resource_cgroup.rs` (the latter file is new in this window, created
by `e6d26871`).

**Windows-relevance finding (all 4 rows):** `git show <sha> | grep -in windows` returns **zero**
hits for all 4 CORE commits — none of `ca888108`/`099237da`/`e6d26871`/`34c2c975` mention Windows
anywhere in their diffs, not even in doc comments. `e6d26871` (#1269, resource limiting) and
`34c2c975` (#1403, `--max-processes`) implement their enforcement entirely via
`crates/nono-cli/src/resource_cgroup.rs` (Linux cgroup v2) with no Windows Job Object
counterpart authored upstream at all. This confirms CORE-02's framing ("reconciled with the
fork's existing kernel-enforced Job Object implementation") is not an adopt-then-verify task —
Phase 111 is authoring the Windows reconciliation from a zero-Windows-precedent starting point,
not adapting an upstream Windows code path.

| sha | subject | files-changed | windows-touch | security-relevant | requirement-mapping | disposition | re-export scan |
|-----|---------|----------------|:---:|:---:|:---:|-----------|-----------------|
| `ca888108fe5983be866e8d1a6eccf96edc2a8dd5` | fix: missing ~/.cache on macOS (#1378) | 6 files (policy.json, learn.rs, macos_trust.rs, proxy_command.rs, nono-proxy/server.rs, tls_intercept/ca.rs), 12+/12- | no | yes | **CORE-01** | adopt | Clean — corrects an existing `~/.cache`-adjacent macOS path reference; no new `pub` items |
| `099237da94d33752ca5617db3f4902a216f9f84c` | fix(exec): raise MAX_CRYPTO_THREADS to 12 for macOS libdispatch workqueue threads (#1424) | 1 file (exec_strategy.rs), 7+/4- | no | no | **CORE-01** | adopt | Clean — single-constant tuning change, no security-boundary implication |
| `e6d26871f0498e7dc7a867e67af5c6136b84f91c` | feat: resource limiting (#1269) | 23 files (cli.rs, command_runtime.rs, exec_strategy.rs, main.rs, output.rs, profile_cmd.rs, `resource_cgroup.rs` [new], sandbox_prepare.rs, sandbox_state.rs, supervised_runtime.rs, nono/capability.rs, lib.rs, `manifest.rs` [new], manifest_convert.rs, `resource/mod.rs` [new], state.rs, +schema/docs), 2517+/14- | no | yes | **CORE-02** | adapt | **`pub mod resource;` + `pub use resource::ResourceLimits;` added to `crates/nono/src/lib.rs`** — a new cross-crate re-export from the core library's public API surface (not intra-crate `pub(crate)` like every other row in this ledger). Flagged below under Threat Flags — Phase 111 must confirm this re-export is policy-free mechanism (ADR-86-compliant) before adopting verbatim. |
| `34c2c975d649844923cf1515be94de624c689c6c` | feat(resources): cap sandbox process count with --max-processes (#1403) | 15 files (cli.rs, command_runtime.rs, output.rs, resource_cgroup.rs, sandbox_prepare.rs, sandbox_state.rs, supervised_runtime.rs, nono/schema, capability.rs, manifest.rs, manifest_convert.rs, resource/mod.rs, state.rs, +docs), 1251+/181- | no | yes | **CORE-02** | adapt | Clean — extends the `resource` module `e6d26871` created; no additional `pub mod`/`pub use` beyond what `e6d26871` already added |

**CORE disposition summary:** 2 adopt (`ca888108`, `099237da` — macOS-only carry, no fork
Windows counterpart to reconcile) + 2 adapt (`e6d26871`, `34c2c975` — CORE-02 explicitly requires
reconciling with the fork's existing Job Object mechanism, not verbatim adoption). All 4 rows map
to `CORE-01`/`CORE-02`, zero `none`, zero blank cells. `windows-touch: no` for all 4 (zero
Windows mentions of any kind, per the finding above — a materially different result from the
PROF cluster's `Os::Windows`-runtime-match findings).

---

## Threat Flags

| Flag | File | Description |
|------|------|--------------|
| threat_flag: cross-crate-reexport | `crates/nono/src/lib.rs` (via `e6d26871`, CORE-02) | Upstream's resource-limiting feature adds `pub mod resource;` + `pub use resource::ResourceLimits;` to the core `nono` library's public API — the first CORE-cluster commit in this window to cross the library/CLI boundary rather than stay `pub(crate)`/CLI-internal. Not itself a threat_model item in `108-03-PLAN.md` (that threat_model covers ledger-classification tampering, not library-boundary re-exports) but flagged here per the SUMMARY template's mandatory threat-surface scan: Phase 111 must confirm `ResourceLimits` is caller-supplied mechanism (ADR-86-compliant), not embedded policy, before absorbing verbatim. |

---

## tool-sandbox-pure and tool-sandbox-split (Plan 108-04, Task 1)

**Method:** for each of the 20 SHAs in the D-06 3-path union (`crates/nono-cli/src/tool-sandbox/`,
`crates/nono-cli/src/command_policy.rs`, `crates/nono-cli/src/lineage_cgroup.rs`), the definitive
touched-path list was pulled via `git show --name-only --format='' <sha>`. A commit is **pure** if
every touched path is inside the module set OR is non-production (`docs/`,
`crates/nono-cli/data/`, `crates/nono-cli/tests/`). A commit is **split** if it touches the module
set **and** at least one other production source path (anything under `crates/*/src/` or
`bindings/c/src/` outside the module set). This reproduces the plan's own `interfaces` block
methodology exactly (re-run independently in this task, not copied from the plan).

### Pure/entangled count reconciliation

CONTEXT.md D-05 states "The 12 commits touching both tool-sandbox and other code get a split
verdict" — an aggregate estimate implying 8 pure by subtraction (20 − 12 = 8), **with no
per-commit enumeration recorded to diff against**. This task's exhaustive, independently-run
per-path classification (command output captured above the per-commit tables below) finds exactly
**9 pure / 11 split** — one fewer split commit than D-05's "12" figure. This **exactly matches**
the plan's own "Planner-found" reconnaissance hypothesis recorded in `108-04-PLAN.md`'s
`interfaces` block (9 pure / 11 entangled, commit-for-commit identical to the SHAs found here),
which itself already flagged disagreement with CONTEXT's "8/12" figure as something this task must
resolve, not silently inherit.

Because CONTEXT.md D-05 gives no SHA-level breakdown for its "12", the exact commit responsible
for the 12-vs-11 gap cannot be attributed with certainty from CONTEXT.md's own text. The most
plausible candidate, based on touched-path shape, is
**`e2c87fd5e82b1dd7c1cf9a70d9632f2b0310afb4`** (feat(tool-sandbox): per-command
`unsafe_macos_seatbelt_rules` escape hatch): it touches 5 paths —
`crates/nono-cli/data/nono-profile.schema.json`, `crates/nono-cli/src/command_policy.rs`,
`crates/nono-cli/src/tool-sandbox/platform/macos.rs`, `crates/nono-cli/tests/schema_shape.rs`,
`docs/cli/features/tool-sandbox.mdx` — of which **two** (the schema JSON and the Rust test file)
are non-module "other" paths. A coarse "touches tool-sandbox plus more than one other file"
heuristic (the kind of quick eyeball pass that would produce an unenumerated aggregate like "12"
during a discussion) would plausibly flag this as entangled/split. Hand-verification shows both
extra paths are non-production per this task's own rule (a JSON schema file under
`crates/nono-cli/data/` and a Rust test file under `crates/nono-cli/tests/`, neither
`crates/*/src/` nor `bindings/c/src/`), so it classifies **pure**.

**Resolved count: 9 pure / 11 split (sum = 20). This table set is authoritative; CONTEXT.md D-05's
"12" is superseded** — exactly the outcome D-04/D-21 re-verification exists to catch, consistent
with every other hypothesis-vs-measurement reconciliation already recorded in this ledger (108-01's
CODE/DEPS/DOCS split, 108-03's PROF row-count).

## tool-sandbox-pure (9 commits)

Every touched path is inside the D-06 module set or is non-production (`docs/`,
`crates/nono-cli/data/`, `crates/nono-cli/tests/`). Named refinement PRs are marked explicitly.

| sha | subject | PR# | disposition |
|-----|---------|-----|--------------|
| `cafc39eb3104e789cee2b663dc04b8aa7ef0062a` | fix(sandbox): allow exec in writable grant-dirs under command policies | #1391 | DEFERRED->v3.7 (base subsystem absent — D-06) |
| `ab93cf44e5706b49b1aa5f8ecc1b3b59f29596f8` | fix(tool-sandbox): preserve argv[0] for symlink-dispatched commands | **#1413 (named)** | DEFERRED->v3.7 (base subsystem absent — D-06) |
| `bf7ea3cb7faa241de13fe549152e28333b1f0068` | fix(tool-sandbox): grant env-shebang scripts their re-exec interpreter | **#1394 (named)** | DEFERRED->v3.7 (base subsystem absent — D-06) |
| `42161620ec68da9c4b09b2283e6584ecbf51d3a2` | fix(cli): match intercept args after global options | #1344 | DEFERRED->v3.7 (base subsystem absent — D-06) |
| `e2c87fd5e82b1dd7c1cf9a70d9632f2b0310afb4` | feat(tool-sandbox): per-command unsafe_macos_seatbelt_rules escape hatch | (none) | DEFERRED->v3.7 (base subsystem absent — D-06) — see reconciliation note above (candidate for the 8-vs-9 discrepancy) |
| `052b8374d2381ca4efdc6a51d9eee54d292a54da` | feat(tool-sandbox): per-intercept sandbox override | (none) | DEFERRED->v3.7 (base subsystem absent — D-06) |
| `676a042f3c1e3877f1d1770abd19b0fea0345750` | fix(tool-sandbox): ack frame before SCM_RIGHTS send to prevent EMSGSIZE on macOS | **#1325 (named)** | DEFERRED->v3.7 (base subsystem absent — D-06) |
| `199bb26699661e8a36de141adc4767617e0a1036` | feat(tool-sandbox): add git worktree tokens; fold include-files into @git:config-files | **#1280 (named)** | DEFERRED->v3.7 (base subsystem absent — D-06) |
| `de8a54b25c3c26f68f6d51f768be904cee0d0c41` | fix(dynamic-providers): run git config from repo root to honour hasconfig: includeIf | #1313 | DEFERRED->v3.7 (base subsystem absent — D-06) |

**Named-PR coverage in this table: 4 of 7** (#1280, #1325, #1394, #1413). The remaining 3 named PRs
(#1322, #1384, #1417) are **split** commits, not pure — see the note immediately below and their
full residue tables in `## tool-sandbox-split`.

**Named refinement PRs that are split, not pure — explicit DEFERRED->v3.7 marking for their
module-scoped content:**

| PR# | sha | disposition |
|-----|-----|--------------|
| **#1322** | `eb2d61a7abc5a354b8f8085b782cd11afa6babb8` | DEFERRED->v3.7 for its tool-sandbox-module-scoped rows (base subsystem absent — D-06); commit is `split`, not `pure` — see residue table below |
| **#1384** | `72a988309974f426f0da8f507a4f05df9738ce92` | DEFERRED->v3.7 for its tool-sandbox-module-scoped rows (base subsystem absent — D-06); commit is `split`, not `pure` — see residue table below |
| **#1417** | `a519ee62e6d571330caa6a9a7d033fb10bff5939` | DEFERRED->v3.7 for its tool-sandbox-module-scoped rows (base subsystem absent — D-06); commit is `split`, not `pure` — see residue table below |

All 7 named refinement PRs (#1280, #1322, #1325, #1384, #1394, #1413, #1417) are now accounted for
with an explicit DEFERRED->v3.7 disposition — 4 as fully-pure commits, 3 as split commits whose
module-set-scoped paths are deferred while their non-module paths are separately accounted for
below. Recording all 3 split PRs as blanket-pure (which the plan's literal acceptance-criteria
phrasing might suggest) would itself have been the exact "blanket defer" anti-pattern D-05
forbids — silently hiding that #1417/#1322/#1384 also carry absorbable-or-wiring content.

## tool-sandbox-split (11 commits — full D-07 residue accounting)

For each split commit: every touched path from `git show --name-only --format='' <sha>` is listed
with a marker of exactly `absorb` / `defer` / `noise`. Row count per commit is spot-checked against
`git show --name-only --format='' <sha> | wc -l`.

---

#### `a519ee62e6d571330caa6a9a7d033fb10bff5939` — #1417 fix(tool-sandbox): attribute daemonized callers to their command (10 paths)

**Finding:** structurally `split` (touches `main.rs`/`profile/mod.rs`/`resource_cgroup.rs`,
production paths outside the module set), but on inspection **zero** of those non-module paths
carry independently-absorbable content — `main.rs` only registers `mod lineage_cgroup;` behind
`#[cfg(target_os = "linux")]`, `profile/mod.rs` only adds unit tests for the new
`daemon_pid_source` field merging through `platform_overrides`, and `resource_cgroup.rs` only
widens 3 functions from private to `pub(crate)` so `lineage_cgroup.rs` can call them. All three are
wiring for the deferred `lineage_cgroup.rs`/`command_policy.rs` feature, not standalone value. This
differs from the 4 D-05-named worked examples, which is exactly why D-05 calls those 4 out by name
as the commits proving unsafe deferral — not every split commit has genuine absorb content.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/data/nono-profile.schema.json` | noise | schema data file |
| `crates/nono-cli/src/command_policy.rs` | defer | module set |
| `crates/nono-cli/src/lineage_cgroup.rs` | defer | module set |
| `crates/nono-cli/src/main.rs` | defer | wiring-only: `#[cfg(target_os = "linux")] mod lineage_cgroup;` registration, no independent value without the deferred module |
| `crates/nono-cli/src/profile/mod.rs` | defer | wiring-only: unit tests for `daemon_pid_source` field merge (a `command_policy.rs`/tool-sandbox concept) |
| `crates/nono-cli/src/resource_cgroup.rs` | defer | wiring-only: 3 fns widened `fn`→`pub(crate) fn` solely so `lineage_cgroup.rs` can call them; no behavior change of its own |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set |
| `crates/nono-cli/tests/schema_shape.rs` | noise | test file |
| `docs/cli/features/tool-sandbox.mdx` | noise | docs file |

Row count: 3 noise + 7 defer + 0 absorb = 10 = touched-path count (10). ✓

---

#### `d5803b994b416ad07a73907143ca169c408917f3` — #1398 feat: add port range support to sandbox profiles (17 paths) — **D-05 worked example (PROF-03)**

**Finding:** genuinely split with substantial absorb content. The commit message states ranges
"expand to individual rules on both platforms — Seatbelt rules on macOS, Landlock NetPort objects
on Linux," implemented entirely in `crates/nono/src/sandbox/linux.rs`/`sandbox/macos.rs` +
`crates/nono/src/capability.rs` (the port-range `CapabilitySet` mechanism) + CLI plumbing
(`profile/mod.rs`, `profile_cmd.rs`, `profile_runtime.rs`, `capability_ext.rs`, `exec_strategy.rs`,
`exec_strategy/supervisor_linux.rs`, `output.rs`, `supervised_runtime.rs`,
`manifest_convert.rs`, `capability-manifest.schema.json`) — none of that is tool-sandbox-specific;
it is a general profile network-config feature (**PROF-03**). Only the 3 `tool-sandbox/*` files
(how a per-command sandbox would *consume* the new range grants) are module-set / deferred.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/data/profile-authoring-guide.md` | noise | docs/data |
| `crates/nono-cli/src/capability_ext.rs` | absorb | **PROF-03** — port-range CLI/profile plumbing |
| `crates/nono-cli/src/exec_strategy.rs` | absorb | **PROF-03** |
| `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` | absorb | **PROF-03** |
| `crates/nono-cli/src/output.rs` | absorb | **PROF-03** |
| `crates/nono-cli/src/profile/mod.rs` | absorb | **PROF-03** |
| `crates/nono-cli/src/profile_cmd.rs` | absorb | **PROF-03** |
| `crates/nono-cli/src/profile_runtime.rs` | absorb | **PROF-03** |
| `crates/nono-cli/src/supervised_runtime.rs` | absorb | **PROF-03** |
| `crates/nono/schema/capability-manifest.schema.json` | absorb | **PROF-03** — manifest schema for the new port-range capability shape |
| `crates/nono/src/capability.rs` | absorb | **PROF-03** — core `CapabilitySet` port-range type |
| `crates/nono/src/manifest_convert.rs` | absorb | **PROF-03** |
| `crates/nono/src/sandbox/linux.rs` | absorb | **PROF-03** — Landlock `NetPort` object emission (kernel-enforced) |
| `crates/nono/src/sandbox/macos.rs` | absorb | **PROF-03** — Seatbelt rule emission |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set — per-command consumption of range grants |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/protocol.rs` | defer | module set |

Row count: 1 noise + 13 absorb + 3 defer = 17 = touched-path count (17). ✓ At least one
`absorb`-marked row present (13, in fact) — satisfies the D-05 worked-example requirement.

---

#### `72a988309974f426f0da8f507a4f05df9738ce92` — #1384 (named) feat(tool-sandbox): add per-command exec_paths for multi-call binaries (4 paths)

**Finding:** the `profile/mod.rs` touch is unit-test-only (two new tests asserting
`platform_overrides`-declared `exec_paths` merge into a command's sandbox) — testing a
tool-sandbox-only field, no independent value.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/command_policy.rs` | defer | module set |
| `crates/nono-cli/src/profile/mod.rs` | defer | wiring-only: unit tests for `exec_paths` merge through `platform_overrides`, a `command_policy.rs` concept |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set |
| `docs/cli/features/tool-sandbox.mdx` | noise | docs file |

Row count: 1 noise + 3 defer + 0 absorb = 4 = touched-path count (4). ✓

---

#### `1f54f4ae91870de02c1fabfda554278d49c99bd9` — #1373 fix(command-policy): resolve command_policies binaries once, in parallel, with caching (10 paths)

**Finding:** all 6 non-module files are small (1-8 line) call-site wiring for the new
`command_policy::ResolvedCommandBinaries` caching type (e.g. `profile_runtime.rs` threads a
`resolved_command_binaries: Option<crate::command_policy::ResolvedCommandBinaries>` field through
`PreparedProfile` purely so tool-sandbox plan-building can reuse it) — no independent value absent
`command_policy.rs`.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/command_policy.rs` | defer | module set |
| `crates/nono-cli/src/execution_runtime.rs` | defer | wiring-only, 2-line call-site (verified via diff) |
| `crates/nono-cli/src/launch_runtime.rs` | defer | wiring-only, 4-line call-site |
| `crates/nono-cli/src/main.rs` | defer | wiring-only, 2-line call-site |
| `crates/nono-cli/src/profile_runtime.rs` | defer | wiring-only: threads `ResolvedCommandBinaries` (a `command_policy.rs` type) through `PreparedProfile` (verified via diff) |
| `crates/nono-cli/src/proxy_runtime.rs` | defer | wiring-only, 1-line call-site |
| `crates/nono-cli/src/sandbox_prepare.rs` | defer | wiring-only, 8-line call-site |
| `crates/nono-cli/src/tool-sandbox/mod.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set |

Row count: 10 defer, 0 noise, 0 absorb = touched-path count (10). ✓

---

#### `7c20dc75f4f155d64601719437980a0e232c8bd8` — #1339 fix(tool-sandbox): resolve command policy paths against the live cwd (10 paths)

**Finding:** same shape as `1f54f4ae` — all 6 non-module files are 1-7 line call-site wiring
(module registration / passthrough plumbing) for the cwd-resolution fix, which lives entirely in
`tool-sandbox/dynamic_providers.rs` and `tool-sandbox/platform/{linux,macos}.rs`.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/command_runtime.rs` | defer | wiring-only, 2-line call-site |
| `crates/nono-cli/src/execution_runtime.rs` | defer | wiring-only, 3-line call-site |
| `crates/nono-cli/src/launch_runtime.rs` | defer | wiring-only, 5-line call-site |
| `crates/nono-cli/src/main.rs` | defer | wiring-only, 2-line call-site |
| `crates/nono-cli/src/proxy_runtime.rs` | defer | wiring-only, 1-line call-site |
| `crates/nono-cli/src/sandbox_prepare.rs` | defer | wiring-only, 7-line call-site |
| `crates/nono-cli/src/tool-sandbox/dynamic_providers.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/mod.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set |

Row count: 10 defer, 0 noise, 0 absorb = touched-path count (10). ✓

---

#### `eb2d61a7abc5a354b8f8085b782cd11afa6babb8` — #1322 (named) feat(tool-sandbox): add exec intercept action (8 paths)

**Finding:** touches **`crates/nono-cli/src/policy.rs`** (the fork's general CLI policy-resolver
file — distinct from `crates/nono-cli/src/tool-sandbox/policy.rs`, which is also touched in the
same commit). Diff-verified: adds `expand_env_vars_strict`, a strict `$VAR`-expansion helper
alongside the already-adopted `expand_env_vars`/`substitute_vars` (PROF-02, `2cbaa9a0` in the PROF
cluster table). Its **sole caller in this commit** is `resolve_exec_helper` in
`tool-sandbox/policy.rs` — no caller outside the deferred exec-intercept feature — so it is
wiring for the deferred subsystem, not independently absorbable value, despite living in a
non-module file.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/data/nono-profile.schema.json` | noise | schema data file |
| `crates/nono-cli/src/command_policy.rs` | defer | module set |
| `crates/nono-cli/src/policy.rs` | defer | adds `expand_env_vars_strict`; sole caller is `tool-sandbox/policy.rs::resolve_exec_helper` (verified via diff) — wiring for the deferred exec-intercept action, not standalone |
| `crates/nono-cli/src/tool-sandbox/env.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/policy.rs` | defer | module set |
| `docs/cli/features/tool-sandbox.mdx` | noise | docs file |

Row count: 2 noise + 6 defer + 0 absorb = touched-path count (8). ✓

---

#### `5a7447d3ed30835bd9bd647b7812ee18ea80a155` — fix(tool-sandbox): strip untrusted unsafe_macos_seatbelt_rules before emission (4 paths)

**Finding:** this is one of the 2 commits the D-06 re-measurement shows a directory-only
(`tool-sandbox/`) filter would silently drop — its only tool-sandbox-named path is the docs file;
its module-set membership comes solely from `command_policy.rs`. Diff-verified:
`command_runtime.rs` adds `strip_untrusted_unsafe_seatbelt_rules`, whose only call site (added in
the same commit) is `sandbox_prepare.rs`; both exist purely to call into
`crate::command_policy::nested_unsafe_seatbelt_rules` (module set).

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/command_policy.rs` | defer | module set |
| `crates/nono-cli/src/command_runtime.rs` | defer | wiring-only: `strip_untrusted_unsafe_seatbelt_rules`, calls `command_policy::nested_unsafe_seatbelt_rules` (verified via diff) |
| `crates/nono-cli/src/sandbox_prepare.rs` | defer | wiring-only: sole call site for the function above |
| `docs/cli/features/tool-sandbox.mdx` | noise | docs file — **this is the ONLY tool-sandbox-named path; directory-only filter would drop this commit entirely (D-06)** |

Row count: 1 noise + 3 defer + 0 absorb = touched-path count (4). ✓

---

#### `ebd51cbb9546aec872136301249d048074a05e38` — fix(tool-sandbox): warn on unsafe_macos_seatbelt_rules nested in command/intercept sandboxes (3 paths)

**Finding:** the second D-06 directory-only-filter-drop commit (see `5a7447d3` above — same shape).
Diff-verified: `profile_save_runtime.rs` changes call
`crate::command_policy::nested_unsafe_seatbelt_rules` in 3 places to extend existing warning/patch-
preview logic; no independent value absent `command_policy.rs`.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/command_policy.rs` | defer | module set |
| `crates/nono-cli/src/profile_save_runtime.rs` | defer | wiring-only: calls `command_policy::nested_unsafe_seatbelt_rules` (verified via diff), no independent logic |
| `docs/cli/features/tool-sandbox.mdx` | noise | docs file — **this is the ONLY tool-sandbox-named path; directory-only filter would drop this commit entirely (D-06)** |

Row count: 1 noise + 2 defer + 0 absorb = touched-path count (3). ✓

---

#### `ea334d2bbdcb332c3a1c4164843667b0d2153cb1` — #1332 fix(linux): use u64 for fs_type_unsupported to fix musl build (3 paths) — **D-05 worked example**

**Finding:** genuinely split with independent absorb content. Diff-verified: the substantive fix
is entirely in `crates/nono/src/sandbox/linux.rs` (`V9FS_MAGIC`/`fs_type_unsupported` retyped
`libc::c_long` → `u64` to fix an Alpine/musl build break — musl defines `statfs::f_type` as `u64`
vs glibc's `c_long`), a portable Landlock-detection fix with **zero tool-sandbox dependency** and
no requirement ID (it is a build-target compatibility fix, not a feature). The
`.github/workflows/ci.yml` hunk only adds a musl CI matrix leg + `musl-tools` package — CI-cluster
noise for this task's purposes (its CI-cluster disposition is handled in Task 2, cross-referenced,
not double-counted).

| path | marker | note |
|------|--------|------|
| `.github/workflows/ci.yml` | noise | CI matrix leg + `musl-tools` install; own disposition lives in the CI Cluster table (Task 2) — **not** double-counted as a CI-bucket commit since this SHA is itself CODE-bucket/tool-sandbox-split |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set |
| `crates/nono/src/sandbox/linux.rs` | absorb | musl build fix — portable, no requirement ID; `V9FS_MAGIC`/`fs_type_unsupported` `c_long`→`u64` retype (verified via diff) |

Row count: 1 noise + 1 defer + 1 absorb = touched-path count (3). ✓ At least one `absorb`-marked
row present — satisfies the D-05 worked-example requirement.

---

#### `d4927f95a37863cf0ba534b054e28f48f002ad21` — #1298 feat(profile): expand @git:* dynamic tokens in top-level filesystem paths (5 paths) — **D-05 worked example (PROF-02)**

**Finding:** genuinely split, but with a load-bearing cross-dependency worth flagging explicitly.
`capability_ext.rs` is where the top-level `filesystem.allow`/`read`/`write` fields gain `@git:*`
token expansion (**PROF-02** — already has a pointer row in the PROF cluster table from Plan
108-03). Diff-verified: the fix works by calling
`crate::tool_sandbox::dynamic_providers::expand_dynamic_tokens` — a function that lives in
`tool-sandbox/dynamic_providers.rs`, itself part of the deferred module set. **This means PROF-02's
absorb work in Phase 110 cannot be a clean lift of `capability_ext.rs` alone** — it depends on
`expand_dynamic_tokens` existing, which today only exists inside the deferred tool-sandbox
subsystem. Phase 110 must either port a minimal standalone `expand_dynamic_tokens` (not the full
tool-sandbox module) or explicitly scope PROF-02 down to exclude `@git:*` top-level expansion until
the tool-sandbox subsystem lands in v3.7. This is exactly the kind of coupling D-07 residue
accounting exists to surface — a naive `absorb → PROF-02` marking without this note would silently
promise Phase 110 a self-contained cherry-pick that in fact is not self-contained.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/capability_ext.rs` | absorb | **PROF-02** — BUT calls `tool_sandbox::dynamic_providers::expand_dynamic_tokens` (deferred module); see cross-dependency note above, Phase 110 must resolve before treating this as a clean absorb |
| `crates/nono-cli/src/tool-sandbox/dynamic_providers.rs` | defer | module set — defines `expand_dynamic_tokens`, the function `capability_ext.rs` above depends on |
| `crates/nono-cli/src/tool-sandbox/mod.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set |

Row count: 0 noise + 1 absorb + 4 defer = touched-path count (5). ✓ At least one `absorb`-marked
row present — satisfies the D-05 worked-example requirement.

---

#### `8a4237f2ee0dc33bc1e5afdd39c0db8ca6f5ed38` — #1283 refactor(seccomp): introduce SeccompPolicy struct and client-driven selection (21 paths) — **D-05 worked example**

**Finding:** genuinely split, largest and highest-risk residue table in this window — 18 files
incl. `bindings/c/src/sandbox.rs`, the highest-risk cross-crate public-surface boundary in this
audit. The refactor is a general Linux seccomp/sandbox-enforcement-selection mechanism
(`SeccompPolicy` struct, `apply_auto`/`apply_landlock`/`apply_external` entry points, new
`--sandbox-policy` CLI flag) — **not tool-sandbox-specific**; the commit message states "Update all
in-tree consumers (nono-ffi C bindings, tool-sandbox, exec_strategy, doctests) to use the new entry
points," meaning `tool-sandbox/platform/{linux,macos}.rs` are mere consumer updates, not the
mechanism itself. No existing CORE-0X requirement ID names this seccomp-policy-selection mechanism
directly; it is filed as a **CORE-cluster / Phase 111 residual item** (same disposition class as
the security-residual-and-misc cluster, pending requirement-ID confirmation at absorb time) rather
than force-mapped to CORE-01/02.

**Re-export scan (SC2, mandatory for `bindings/c/src/sandbox.rs`):**
`git show 8a4237f2ee0dc33bc1e5afdd39c0db8ca6f5ed38 | grep '^+' | grep -E '^\+\s*(pub use|pub mod|extern crate|pub\(crate\))'`
returns **only 4 `pub(crate)` hits, zero `pub mod`/`pub use`/`extern crate`**:
`pub(crate) sandbox_policy: crate::profile::LinuxSandboxPolicy,` (×2, on different structs),
`pub(crate) fn from_prepared(`, `pub(crate) sandbox_policy: profile::LinuxSandboxPolicy,` — all
intra-`nono-cli` fields/fns, **zero cross-crate re-exports anywhere in this 18-file diff**. The
`bindings/c/src/sandbox.rs` hunk itself is a 1-line call-site rename
(`nono::Sandbox::apply` → `nono::Sandbox::apply_auto`), diff-verified, no new `pub` item in that
file. **Finding: Clean.**

| path | marker | note |
|------|--------|------|
| `bindings/c/src/sandbox.rs` | absorb | CORE-cluster/Phase-111 residual (seccomp-policy-selection mechanism). **Re-export scan: Clean** — 1-line call-site rename only, zero `pub mod`/`pub use` (see scan above) |
| `crates/nono-cli/src/cli.rs` | absorb | CORE-cluster/Phase-111 residual — new `--sandbox-policy` flag |
| `crates/nono-cli/src/command_runtime.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/exec_strategy.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/execution_runtime.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/launch_runtime.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/main.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/profile/mod.rs` | absorb | CORE-cluster/Phase-111 residual — `LinuxSandboxPolicy` profile-schema field |
| `crates/nono-cli/src/profile_runtime.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/proxy_runtime.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/sandbox_prepare.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/supervised_runtime.rs` | absorb | CORE-cluster/Phase-111 residual |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set — mere consumer update per commit message |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set — mere consumer update |
| `crates/nono/README.md` | noise | docs (`.md`) |
| `crates/nono/src/lib.rs` | absorb | CORE-cluster/Phase-111 residual — doctest-comment-only change (`Sandbox::apply`→`apply_auto`), no new `pub` item (verified via diff) |
| `crates/nono/src/sandbox/linux.rs` | absorb | CORE-cluster/Phase-111 residual — `apply_auto`/`apply_landlock`/`apply_external` entry points |
| `crates/nono/src/sandbox/mod.rs` | absorb | CORE-cluster/Phase-111 residual |
| `docs/cli/usage/flags.mdx` | noise | docs |
| `issues/sandbox-policy-split.md` | noise | design-rationale doc, not source (`.md`, outside `src/`) |

Row count: 3 noise + 2 defer + 16 absorb = touched-path count (21). ✓ At least one `absorb`-marked
row present — satisfies the D-05 worked-example requirement.

---

### tool-sandbox-split summary

11 split commits, 95 total touched-path rows across their residue tables (10+17+4+10+10+8+4+3+3+5+21
= 95), zero unbucketed paths (every commit's residue-table row count spot-checked equal to its
`git show --name-only --format='' <sha> | wc -l` output above). 4 of the 11 (`d5803b99`,
`ea334d2b`, `d4927f95`, `8a4237f2`) are the D-05-named worked examples and each carries at least one
`absorb`-marked row; the other 7 split commits (`a519ee62`, `72a98830`, `1f54f4ae`, `7c20dc75`,
`eb2d61a7`, `5a7447d3`, `ebd51cbb`) are structurally split (touch a production path outside the
module set) but carry **zero** absorb-worthy content on inspection — every non-module path in
those 7 is small wiring (module registration, field threading, or a helper whose only caller is
inside the deferred subsystem). This asymmetry is itself a finding: D-05's forbidden "blanket
defer" anti-pattern is about silently dropping the *rare* commits with genuine absorb content, not
a claim that most split commits carry absorb content — 7 of 11 do not, and recording that
explicitly (rather than omitting the "why" and just marking `defer`) is what keeps this table
audit-safe.

**tool-sandbox arithmetic check: 9 pure + 11 split = 20 = full tool-sandbox surface.** ✓

---

## DEPS, CI, and DOCS Clusters (Plan 108-04, Task 2)

Source SHA lists: Plan 108-01's "Full Commit Accounting" DEPS (19)/CI (11)/DOCS (8) bucket tables
(reproduced above). Each is individually reviewed per D-15/D-16/D-17 — none blanket-adopted or
blanket-skipped.

### `cargo audit` invocation and cross-reference (D-16)

`cargo-audit` is installed in this environment (`cargo-audit-audit 0.22.1`). Ran verbatim against
the fork's current `Cargo.lock`:

```
cargo audit
# Scanning Cargo.lock for vulnerabilities (567 crate dependencies)
# -> 1 vulnerability found, 5 allowed warnings found
```

**6 findings total, verbatim:**

| Crate | Version | ID | Kind | Solution |
|-------|---------|----|----|----------|
| `crossbeam-epoch` | 0.9.18 | RUSTSEC-2026-0204 | vulnerability (error) | Upgrade to >=0.9.20 |
| `async-std` | 1.13.2 | RUSTSEC-2025-0052 | unmaintained (warning) | — |
| `fxhash` | 0.2.1 | RUSTSEC-2025-0057 | unmaintained (warning) | — |
| `paste` | 1.0.15 | RUSTSEC-2024-0436 | unmaintained (warning) | — |
| `rustls-pemfile` | 2.2.0 | RUSTSEC-2025-0134 | unmaintained (warning) | — |
| `anyhow` | 1.0.102 | RUSTSEC-2026-0190 | unsound (warning) | — |

**Cross-reference against the 19 DEPS-bucket commits: 1 direct hit.** `373a67ae65fdb94898b49469d562a263ba2083ee`
(#1369, `chore(deps): bump crossbeam-epoch from 0.9.18 to 0.9.20`) is the **exact fix** for
RUSTSEC-2026-0204 (`Solution: Upgrade to >=0.9.20`) — this DEPS-bucket commit is **not yet
absorbed** into the fork, and the fork's own `Cargo.lock` currently carries `crossbeam-epoch
0.9.18`, the vulnerable version, live today. This is precisely the scenario D-16 exists to catch:
a dependency bump nearly classified as routine "noise" that in fact closes a live RUSTSEC advisory
the fork currently ships with. **Flagged for priority absorption in whichever phase lands the DEPS
cluster (currently unassigned — see D-19 gap note).** The other 5 findings (`async-std`, `fxhash`,
`paste`, `rustls-pemfile`, `anyhow`) do not correspond to any crate bumped by a DEPS-bucket commit
in this window — they are pre-existing transitive dependencies (`async-std`/`fxhash` via
`httpmock`/`tracing-etw` respectively — dev-only or Windows-ETW-only deps; `rustls-pemfile` via
`nono-sandbox-proxy`; `anyhow` via a long `wit-parser`/`sigstore` transitive chain) not touched by
this window's upstream sync at all, so they are out of scope for this ledger and are recorded here
only for completeness, not as DEPS-bucket findings.

## DEPS Cluster (19 commits)

| sha | subject | Cargo.toml/Cargo.lock delta summary | cargo-audit-flag |
|-----|---------|--------------------------------------|-------------------|
| `59bdace7e905c05c127f480dc6d2a8c3a3331392` | chore: release v0.69.0 | version bump v0.68.0->v0.69.0 across all 4 crate `Cargo.toml`s + `Cargo.lock` regen | none |
| `00692e8c7846c6ee00ad6239d1be3b9e9b8d5dea` | chore: release v0.68.0 (#1418) | version bump v0.67.1->v0.68.0 across all 4 crate `Cargo.toml`s + `Cargo.lock` regen | none |
| `ce3d6724ed172226b17ec2ec1a22ea4b21fb7858` | chore(deps): bump sigstore-trust-root 0.9.0->0.11.0 (#1410) | bumps `sigstore-trust-root` 0.9.0 -> 0.11.0 | none (not in the 6-finding list) |
| `2f7a6747b8e12e71838cd22def743dc1bd789cef` | chore(deps): bump bytes 1.12.0->1.12.1 (#1408) | bumps `bytes` 1.12.0 -> 1.12.1 | none |
| `b6e7ec43474ca57d075f19c8ca318690719ef185` | chore(deps): bump regex 1.12.4->1.13.0 (#1406) | bumps `regex` 1.12.4 -> 1.13.0 | none |
| `bc947b0bdb8d693a704c3cf9af6d098c2aa12805` | chore(deps): bump sigstore-sign 0.10.0->0.11.0 (#1407) | bumps `sigstore-sign` 0.10.0 -> 0.11.0 | none |
| `373a67ae65fdb94898b49469d562a263ba2083ee` | chore(deps): bump crossbeam-epoch 0.9.18->0.9.20 (#1369) | bumps `crossbeam-epoch` 0.9.18 -> 0.9.20 | **RUSTSEC-2026-0204 — fork's Cargo.lock currently carries the vulnerable 0.9.18; this commit is the fix. See cross-reference above.** |
| `0bfb5c54621d933e28ff4c2800a0f2629ff648de` | chore(deps): bump clap_complete 4.6.5->4.6.7 (#1360) | bumps `clap_complete` 4.6.5 -> 4.6.7 | none |
| `0f2f635bcd547b52b8e86632c5b2ad753bb147c3` | chore(deps): bump ignore 0.4.26->0.4.27 (#1363) | bumps `ignore` 0.4.26 -> 0.4.27 | none — `ignore` itself has no advisory; note `ignore`'s own transitive `crossbeam-deque`->`crossbeam-epoch` chain is a *different* edge than the direct `373a67ae` bump, both ultimately need `crossbeam-epoch` >=0.9.20 to fully clear the lockfile |
| `3cff37337ea60f1c5a10ebd335676575dd0ecd10` | chore(deps): bump time 0.3.52->0.3.53 (#1358) | bumps `time` 0.3.52 -> 0.3.53 | none |
| `b59477a56e47e91539ebfcadf4d374e26a66f3fa` | chore(deps): bump rand 0.10.1->0.10.2 (#1362) | bumps `rand` 0.10.1 -> 0.10.2 | none |
| `18bbb5eb1fbbe1167c1eef8f99d748b40a65715e` | chore(deps): bump sigstore-sign 0.8.0->0.10.0 (#1361) | bumps `sigstore-sign` 0.8.0 -> 0.10.0 | none |
| `6118b79aeda1365da213d85457b4d3cf1201d575` | chore: release v0.67.1 (#1353) | version bump v0.67.0->v0.67.1 across all 4 crate `Cargo.toml`s + `Cargo.lock` regen | none |
| `19b43f300b9d2e257e161b808440f0573abfd422` | chore: release v0.67.0 (#1350) | version bump v0.66.0->v0.67.0 across all 4 crate `Cargo.toml`s + `Cargo.lock` regen | none |
| `2af5495cda0fc0067360c4b99ffdebc8e3c68755` | chore(deps): bump h2 0.4.14->0.4.15 (#1312) | bumps `h2` 0.4.14 -> 0.4.15 | none |
| `cf7a40392a0011e22c4d11bdaf4b16add46a9e55` | chore(deps): bump webpki-roots 1.0.7->1.0.8 (#1311) | bumps `webpki-roots` 1.0.7 -> 1.0.8 | none |
| `d8187fb17c44869bdd4bd9670f0070320f7c10f4` | chore(deps): bump rustls 0.23.40->0.23.41 (#1310) | bumps `rustls` 0.23.40 -> 0.23.41 | none (distinct from the unrelated `rustls-pemfile` unmaintained warning above) |
| `b8f00cefd811ff3b950314963c85f4456df0206c` | chore(deps): bump time 0.3.49->0.3.51 (#1307) | bumps `time` 0.3.49 -> 0.3.51 | none |
| `4b889c953f2473f5a39a69126fc17eacc50c000e` | chore(deps): bump jsonschema 0.46.5->0.46.6 (#1306) | bumps `jsonschema` 0.46.5 -> 0.46.6 | none |

**DEPS disposition summary:** 19/19 individually reviewed (not blanket-adopted). 16 are routine
semver-compatible bumps with no advisory hit. 3 are release-cut commits (version-bump-only,
already captured under D-16's own "own cluster" framing, not noise). 1
(`373a67ae`) is a **priority absorb** — the direct fix for a live RUSTSEC advisory in the fork's
current `Cargo.lock`.

## CI Cluster (11 commits)

Fork's `.github/workflows/` inventory checked directly (`ls`) for each commit's touched
workflow file(s) before assigning adapt-reasoning, per D-17 (never adopt verbatim without
checking the fork's actual file).

| sha | subject | adapt-reasoning |
|-----|---------|------------------|
| `bffe8134ee4b68752dc1b2ef514a8ae58f3b6123` | chore(deps): bump docker/setup-buildx-action 4.1.0->4.2.0 (#1359) | **portable** — fork's `image-build.yml` exists and currently pins `docker/setup-buildx-action@...#v4.0.0`, older than upstream's bump target; routine mechanical action-version bump |
| `cbe9bdc28f842157c95cbb6ff2983ab0797bda32` | chore(deps): bump docker/login-action 4.2.0->4.4.0 (#1357) | **portable** — fork pins `docker/login-action@...#v4.1.0`, older; routine bump |
| `2485430a0ac821bffda784a5cea8590f2fca5efb` | chore(deps): bump nolabs-ai/agent-sign 0.0.11->0.1.0 (#1355) | **fork-specific conflict** — verified: fork's `sign-instruction-files.yml` pins `always-further/agent-sign@...#v0.0.8`, a **different action reference** (pre-relocation org, per Phase-94 remote-relocation precedent) at a different version scheme entirely. Verbatim adoption would point at an action the fork's workflow doesn't reference — ADR-100-style bot/action-identity mismatch, not a mechanical bump |
| `168efa2bac7f0dedddcb43a089e15f1022dc0f17` | chore(deps): bump docker/setup-qemu-action 4.1.0->4.2.0 (#1354) | **portable** — fork pins `docker/setup-qemu-action@...#v4.0.0`, older; routine bump |
| `3fc3743dd1d551a91f54f075f683a30b32d7d67a` | chore(deps): bump docker/build-push-action 7.2.0->7.3.0 (#1356) | **portable** — fork pins `docker/build-push-action@...#v7.0.0`, older; routine bump |
| `002fe4986e9b03c61d0a90d78bbd535108a9b698` | fix(release): strip ./ prefix from SHA256SUMS.txt entries (#1352) | **portable** (moot for current fork structure) — verified: fork's `release.yml` already emits `sha256sum *.tar.gz > SHA256SUMS.txt` (bare filenames, no `./` prefix) via a different, already-diverged per-artifact-type generation block (tar.gz/zip/msi/exe/trusted_root.json — the fork's own Windows-artifact structure); the specific `./`-prefix bug this commit fixes does not reproduce in the fork's current script, but the underlying `sha256sum -- <globs>` pattern (avoiding SC2035) is worth carrying forward if the fork's structure is ever consolidated |
| `a0a279079bba8057085d45503dee681a51db99f6` | chore(deps): bump actions/attest 4.1.0->4.1.1 (#1309) | **fork-specific conflict** — verified: fork has no `attest-release.yml` and zero `actions/attest` references anywhere in `.github/workflows/`; the fork uses Azure Trusted Signing + Sigstore (`sigstore-sign`/`sigstore-verify`) for release provenance instead of GitHub's native artifact-attestation action. Not applicable, not merely skippable-as-noise |
| `ade1bb128b2e2d28054bfe62aa44abd0cf52b416` | chore(deps): bump actions/cache 5.0.5->6.1.0 (#1308) | **portable** — verified: fork's `ci.yml` uses `actions/cache@...#v5` in 8 separate steps; bumping the pinned major version to v6.1.0 is a routine mechanical upgrade with no fork-specific dependency |
| `40080d04991cc28cd9f99d409dd42b2c4bcbe45b` | chore(ci): remove homebrew bump workflow (#1294) | **fork-specific conflict** — verified: fork has no standalone `homebrew-bump.yml` to remove; the fork's homebrew publishing is an inline `update-homebrew-core` job inside `release.yml` using `mislav/bump-homebrew-formula-action`, architecturally distinct from what upstream removed. Not applicable |
| `92ef76ffa5fd5ead3bbaf19ac5a928a48b63ae03` | ci: run actionlint (#1273) | **portable** — verified: fork has `ci.yml`/`image-build.yml`/`release.yml`, all 3 files this commit touches; adds an additive `actionlint` linting job, no removal or fork-specific-workflow dependency in the diff |
| `4fd9144525c4b748bd78a5c861678957ac5c776b` | chore(ci): refine automation workflow (#1292) | **fork-specific conflict** — verified: renames `label-new-issues.yml` (which the fork DOES have) into a 237-line `project-automation.yml` that polls a `nolabs-ai` org GitHub Projects v2 board via GraphQL, gated on a `PROJECTS_TOKEN` secret the fork does not provision. Upstream community/triage infrastructure, not portable without re-scoping to a fork-owned board |

**`ea334d2b` (musl build fix) is NOT in this table** — verified: it is a tool-sandbox-split
**CODE**-bucket commit (its own touched-path list includes `crates/nono-cli/src/tool-sandbox/
platform/linux.rs` and `crates/nono/src/sandbox/linux.rs`, both under `crates/*/src/`), already
fully accounted for in `## tool-sandbox-split` above (its `.github/workflows/ci.yml` hunk is noted
there as CI-cluster-adjacent noise, not double-counted as a CI-bucket commit).

**CI disposition summary:** 11/11 individually reviewed. 7 portable (4 docker/* action bumps +
actions/cache bump + actionlint + the SHA256SUMS fix, moot-but-harmless). 4 fork-specific conflict
(agent-sign org mismatch, actions/attest not used, homebrew mechanism architecturally different,
Projects-v2 board automation tied to upstream's own org). This validates D-17's "adapt-by-default,
never verbatim, never blanket-skip" framing precisely: portable commits (7) outnumber
fork-specific-conflicting ones (4), so a blanket-skip-all-CI rule would have missed 7 legitimate,
low-risk version bumps and one useful linting addition.

## DOCS Cluster (8 commits)

Fork's docs tree (`docs/cli/features/`, `docs/cli/clients/`) and root community-health files
checked directly for each commit's target file(s) before assigning a reconcile-note.

| sha | subject | reconcile-note |
|-----|---------|------------------|
| `55fd3825c4ea9471a207313f74aa71e6e4a96014` | docs(profiles): fix codeblock (#1426) | needs-doc-follow-up — verified: fork carries `docs/cli/features/profiles-groups.mdx`, likely inherited the same codeblock formatting bug (fork's copy predates this fix); low-risk cosmetic doc fix worth applying in whichever phase reconciles docs |
| `99b8fa4fbe7da26c332e69e53f44fd84c39b4eb9` | docs(codex): clarify codex docs around the optional login-shell hardening (#1381) | needs-doc-follow-up — verified: fork carries `docs/cli/clients/codex.mdx` |
| `d74063335e8584f9667496e25967e1c87e1d38e1` | fix: use permanent community link across project (#1349) | safe-to-ignore — verified: fork has none of the touched files (`CONTRIBUTING.md`, `MAINTAINERS.md`, `README.md`-community-link-section, `docs/docs.json`, `.github/ISSUE_TEMPLATE/config.yml`); fork does not carry a community-governance file set at all |
| `2f547703517d0c5436f57aad341a3a51af18e116` | docs: add community health files (#1348) | safe-to-ignore — verified: fork has none of `CODE_OF_CONDUCT.md`/`CONTRIBUTING.md`/`CONTRIBUTORS.md`/`GOVERNANCE.md`/`MAINTAINERS.md`; the fork is solo-maintained, not community-governed like upstream |
| `73952cd4e1762a835b47b5f4c23c7abd030fb0f9` | docs(readme): explain tool sandboxing for agents (#1342) | needs-doc-follow-up, **deferred** — this documents the tool-sandbox subsystem itself (`command_policies` JSON example, "Sandboxed Tool Execution" link); adopting it now into the fork's `README.md` would describe a feature the fork does not have until v3.7 (D-06/D-09 — tool-sandbox subsystem absent). Hold until the v3.7 Windows Tool-Sandbox Parity milestone lands, then reconcile alongside it |
| `2375aeb6adb81d1d896336328cf61e04b2a4b19e` | docs(profiles): clarify predefined vs user profiles scope (#1331) | needs-doc-follow-up — verified: fork carries `docs/cli/features/profiles-groups.mdx` (same file as `55fd3825` above) |
| `63c9589fa30f0244c542c026faf27d517aaa9b39` | docs(credential-injection): document AWS SigV4 proxy signing (#1329) | needs-doc-follow-up — verified: fork carries `docs/cli/features/credential-injection.mdx`, **and** the fork already adopted the code this documents (`6fb7ecbf`, SigV4 URI-encoding fix, NET cluster, Plan 108-03 — dispositioned `adopt`); this doc should land alongside NET absorb in Phase 109, not independently |
| `da3a90f247bf80ffe8bcc470154e83f764033803` | docs(nogent): add nogent markdown file (#1288) | safe-to-ignore — verified: fork has no `NOGENT.md`; this is upstream's own GitHub-App-onboarding document ("Once this is in place, we can add the repo to the GitHub App"), not relevant to the fork's separate identity/tooling |

**DOCS disposition summary:** 8/8 individually reviewed. 4 needs-doc-follow-up (2 for
`profiles-groups.mdx`, 1 for `codex.mdx`, 1 for `credential-injection.mdx` — the last explicitly
tied to Phase 109's NET absorb). 1 needs-doc-follow-up-but-deferred (README tool-sandboxing
section, held for v3.7). 3 safe-to-ignore (community-health files + NOGENT.md the fork does not
carry and does not need, being solo-maintained rather than community-governed).

## Bucket-Count Reconciliation

**DEPS: 19 rows. CI: 11 rows. DOCS: 8 rows. Sum: 38 = the non-CODE-bucket total from Plan 108-01's
Full Commit Accounting (DEPS 19 + CI 11 + DOCS 8 = 38, out of the ledger's 100-commit total).**

This task's own per-commit review (checking every commit against the fork's *actual* current
`.github/workflows/`/`docs/`/root files, not just the commit subject) reproduces Plan 108-01's
bucket totals **exactly** — zero commits moved buckets. Note the plan's Task 2 action text cites
"Plan 108-01's recorded bucket counts (15/11/6)" as the comparison baseline; that figure is
**CONTEXT.md's original, already-superseded hypothesis** (see 108-01's own "Discrepancy vs.
CONTEXT.md Hypothesis" section), not Plan 108-01's actual measured/authoritative ledger numbers
(DEPS 19 / CI 11 / DOCS 8, recorded in the "Bucket totals" line of the Headline section and the
per-bucket table headers above). This task reconciles against the **authoritative** 19/11/8
figures, which is the correct baseline — comparing against the stale 15/11/6 hypothesis here would
have reintroduced exactly the kind of hypothesis-vs-measurement error D-04/D-21 exist to prevent.
**Result: no reclassification needed; DEPS/CI/DOCS table row counts match the ledger's own
authoritative totals with zero disagreement.**
