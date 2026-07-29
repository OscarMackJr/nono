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
`windows-touch`, and `security-relevant` are literal `TBD` placeholders in this plan — Plans
108-03/108-04/108-05 fill them in per-commit/per-cluster.

**Arithmetic check:** NET 12 + PROF 8 + CORE 4 + tool-sandbox-surface 20 +
security-residual-and-misc 18 = **62 = CODE bucket total** (verified above).

| cluster_id | theme | commit_count | disposition | windows-touch | security-relevant | phase-target |
|------------|-------|--------------|-------------|----------------|--------------------|---------------|
| NET | proxy/network (deny_domain, SPIFFE, SigV4, sibling-route, no_proxy, HTTP_PROXY, credential-capture plumbing) | 12 | TBD — Plan 108-03/04/05 | TBD | TBD | 109 |
| PROF | profile/policy (platform_overrides, extends, $VAR tokens, bun/mise presets, inheritable-field save) | 8 | TBD — Plan 108-03/04/05 | TBD | TBD | 110 |
| CORE | macOS/resource-CLI (~/.cache fix, MAX_CRYPTO_THREADS, resource limiting, --max-processes) | 4 | TBD — Plan 108-03/04/05 | TBD | TBD | 111 |
| tool-sandbox-surface | tool-sandbox subsystem (D-05/D-06 3-path union; NOT subdivided pure/split here — Plan 108-04's job) | 20 | TBD — Plan 108-03/04/05 | TBD | TBD | v3.7 DEFERRED (per-commit split in Plan 108-04) |
| security-residual-and-misc | security-relevant + unrouted residual commits with no v3.6 requirement home (D-18) | 18 | TBD — Plan 108-03/04/05 | TBD | TBD | proposed Phase 112 pending operator approval |

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
