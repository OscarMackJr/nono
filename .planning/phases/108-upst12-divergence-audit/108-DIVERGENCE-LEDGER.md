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

**This ledger is complete (finalized by Plan 108-05).** 100 non-merge commits in window
`d817ed53..59bdace7` (upstream `v0.66.0..v0.69.0`, tags `v0.67.0`, `v0.67.1`, `v0.68.0`,
`v0.69.0`), plus 4 merge commits (excluded from all counts below per D-15). Every commit is
classified into exactly one of 9 non-overlapping buckets/clusters:

| bucket/cluster | count | disposition tally |
|----------------|------:|--------------------|
| NET | 12 | 9 adopt + 1 adapt + 2 won't-sync (target subsystem absent) (`3b207eeb` #1374 `deny_domain`, per `proj/ADR-108-deny-domain-posture.md`) |
| PROF | 8 | 8 adopt |
| CORE | 4 | 2 adopt + 2 adapt |
| tool-sandbox-pure | 9 | 9 DEFERRED->v3.7 |
| tool-sandbox-split | 11 | 11 DEFERRED->v3.7 (module-scoped portions); non-module absorb-worthy residue routed to PROF-02/PROF-03/CORE-residual |
| security-residual-and-misc | 18 | 18 DEFERRED->proposed Phase 112 pending operator approval |
| DEPS | 19 | 19 individually reviewed (16 routine + 3 release-cut; 1 priority-absorb flag: `373a67ae` closes live RUSTSEC-2026-0204) |
| CI | 11 | 7 portable + 4 fork-specific-conflict |
| DOCS | 8 | 4 needs-doc-follow-up + 1 needs-doc-follow-up-deferred-to-v3.7 + 3 safe-to-ignore |
| **Total** | **100** | overall: adopt 19 (9 NET + 8 PROF + 2 CORE) / adapt 3 (1 NET + 2 CORE) / won't-sync (target subsystem absent) 2 (2 NET) / DEFERRED->v3.7 20 (9 tool-sandbox-pure + 11 tool-sandbox-split) / DEFERRED->proposed-Phase-112 18 / individually-reviewed-non-CODE 38 (19 DEPS + 11 CI + 8 DOCS, each dispositioned in its own bucket, not adopt/adapt/skip/split). Arithmetic: 19+3+2+20+18+38 = 100. |

**Requirement Coverage Gap:** 27 non-tool-sandbox CODE commits map to none of v3.6's 12
requirements — ROADMAP.md Phase 108 SC4 is unsatisfiable as originally written. See
"Requirement Coverage Gap (D-18/D-19)" immediately below for the exact count, the full SHA list,
and the proposed **Phase 112 "Security + Residual Sync"** roadmap amendment (operator-approval
gated, not applied to ROADMAP.md by this ledger).

**Tool-sandbox subsystem:** the full 20-commit 3-path-union surface (9 pure + 11 split) is
recorded per D-09 so v3.7 (Windows Tool-Sandbox Parity) inherits the real work-list rather than
the 7-PR-level approximation; all 7 named refinement PRs (#1280/#1322/#1325/#1384/#1394/#1413/
#1417) are explicitly DEFERRED->v3.7 (4 as pure-table rows, 3 as split commits whose
module-scoped portions defer — see "tool-sandbox-pure and tool-sandbox-split" below).

**`deny_domain` (#1374) posture:** settled ADAPT in the standalone
`proj/ADR-108-deny-domain-posture.md` (D-10/D-11/D-12) — an additional deny layer only, evaluated
before the allowlist; this ledger cross-references it rather than duplicating the analysis.

**Bucket totals (CODE/DEPS/CI/DOCS coarse view, measured live 2026-07-29):
CODE 62 / DEPS 19 / CI 11 / DOCS 8 = 100.** This **disagrees** with CONTEXT.md's recorded
hypothesis of **CODE 68 / DEPS 15 / CI 11 / DOCS 6** (same 100 total, same CI count, different
CODE/DEPS/DOCS split). Per D-04/D-21 this disagreement is exactly the outcome re-measurement
exists to catch — see "Discrepancy vs. CONTEXT.md Hypothesis" under Full Commit Accounting for
the reconciliation and the exact rule that produces each split.

**Tool-sandbox surface:** 20 commits (live-reconfirmed, matches CONTEXT.md D-05/D-09 exactly).
**D-06 re-measurement finding:** the directory-only set (`tool-sandbox/`) is **18** commits, not
20 — it does **not** equal the 3-path union this window. See "Tool-Sandbox Surface — D-06
Re-measurement" below.

---

## Requirement Coverage Gap (D-18/D-19)

**Exact hand-verified count (D-21 — not the crude keyword-matcher estimate CONTEXT.md records):
27 non-tool-sandbox CODE-bucket commits map to none of v3.6's 12 requirements**
(`UPST12-01`/`NET-01..03`/`PROF-01..04`/`CORE-01..02`/`VERIFY-01`/`RLS-14`, per
`.planning/REQUIREMENTS.md`'s v3.6 Traceability table). This count is compiled by summing every
`requirement-mapping = none` row across the NET/PROF/CORE per-commit tables (Plan 108-03) plus the
entire security-residual-and-misc cluster (Plan 108-01), **excluding** the tool-sandbox-surface
cluster's own 18 unmapped commits — those are a *different*, already-resolved gap (D-09
DEFERRED→v3.7, not an oversight):

| source | unmapped (`none`) count | SHAs |
|--------|--------------------------|------|
| NET cluster (Plan 108-03) | 6 | `c344efb0`, `4192bfa5`, `261bbd68`, `3672ea10`, `8255a27a`, `7d23bba6` |
| PROF cluster (Plan 108-03) | 3 | `0374e454`, `9ef5918169`, `f58c7c2242` |
| CORE cluster (Plan 108-03) | 0 | (none — all 4 CORE commits map to CORE-01/CORE-02) |
| security-residual-and-misc (Plan 108-01) | 18 | the 10 D-18-named anchors + 8 additional (see cluster table) |
| **Total** | **27** | |

**Completeness-sweep finding (recorded, not silently corrected — see Completeness Verification
below):** the NET per-commit table's own summary prose (Plan 108-03) states "5 map `none`" but the
table itself contains **6** `none` rows (listed above) — a one-off undercount in that plan's own
arithmetic. The PROF per-commit table's summary prose similarly states "4 rows map `none`" but the
table contains **3** `none` rows (listed above) — a one-off overcount. The two errors cancel
(5+4=9, 6+3=9), so neither previously invalidated a downstream total, but each is a genuine
arithmetic defect in its source plan's prose, caught here by the D-21 hand-recount this task
performs rather than trusting either plan's stated tally. The **27** figure above uses the
corrected per-table counts (6 + 3), not the plans' stated prose (5 + 4) — both arrive at the same
combined total (9) either way, so the final gap count (27) is unaffected, but the discrepancy
itself is a finding worth flagging for whoever revisits Plans 108-03/108-04.

All 10 D-18-named SHAs from `108-CONTEXT.md` appear in the 27-commit gap list above, inside the
security-residual-and-misc row: `0ecc476bf0db3c9509bcc2bd8efae43832c764b8`,
`9b692e07ee4dd156d85588b12b63691778d9e9be`, `3c59c62e7ddcc2d3a1f1c161bd039b5f2b0b5f7b`,
`d033c63111472711e242f4067eb4be04aeaf618a`, `a32439074a61eea21eb374a91fdbcebb91b0afe6`,
`f943fb5a0721352e85d92ef48e3e4d5b36747486`, `d84b4818f824b7660d1a4d95ec2e56f322b6fa63`,
`ac5ccd70712782f4a97c54916ee18b03c650f56f`, `2663e9900fe1317cec0949c18b7a0e6b39321602`,
`a5a441c25769ecec663aed1d7038ab2b8814d43f` — all with `security-relevant=yes` per the D-20 rollup
above.

**ROADMAP.md Phase 108 SC4 ("the ledger maps each will-sync cluster onto Phase 109/110/111")
cannot be satisfied as written -- 27 commits map to none of v3.6's 12 requirements. This is a
legitimate audit finding, not a phase failure.**

### Proposed Phase 112: Security + Residual Sync

**Goal:** Absorb the security-relevant and residual commits from the v0.66.0..v0.69.0 window that
no existing v3.6 phase covers, without mixing security review into a release-cut phase — mirrors
the v3.1 Phase 87 precedent.

**Depends on:** Phase 108 (this ledger's security-residual-and-misc cluster + Requirement Coverage
Gap section as the work-list)

**Draft requirement-ID list** (one per distinct theme in the 27-commit gap list; at minimum one ID
per D-18-named security theme, plus additional IDs for the non-security residual themes found by
this task's own review):

- SEC-01: AWS SigV4 authentication for the MiTM proxy (#1195, `0ecc476b`)
- SEC-02: Declarative sandboxed OAuth capture (`9b692e07`) + capture-boundary hardening (`3c59c62e`) + stdin fixture test (`d033c631`)
- SEC-03: NVIDIA procfs mediation hardening (#1284, `a3243907`)
- SEC-04: Trust-policy `predicate` field to distinguish nono trust policies from foreign JSON (#1333, `f943fb5a`)
- SEC-05: Linux execute-restriction `Refer` grant (#1397, `d84b4818`)
- SEC-06: Seccomp supervisor-ancestry for orphaned descendants (#1401, `ac5ccd70`)
- SEC-07: Standalone `nono proxy` command (#1261, `2663e990`)
- SEC-08: `allow_vars` empty-list env-strip fix (#1204, `a5a441c25769ecec663aed1d7038ab2b8814d43f`)
- SEC-09: Credential-broker non-shim-entry guard relaxation (#1301, `f6f027511f7899f1735f1188747086696e40412e` — found security-relevant during this task's Cluster Summary rollup, not itself a D-18-named anchor)
- RES-01: Registry/update-check header residual (#1405/#1383/#1386/#1341/#1340 — `f050643479`, `7fe0c8283810`, `0158d52f0a`, `762eb05bd3d`)
- RES-02: PTY-teardown + test-infra residual (#1258, `503045801a`; test-only `4cc0af2c52`/`9840a16f35`)

**Draft Success Criteria** (ROADMAP.md phase-detail style):
  1. All 10 D-18-named security-relevant commits (SEC-01..SEC-08 above) are absorbed with a
     fork-invariant review distinct from any release-cut phase, mirroring the v3.1 Phase 87
     separation of security review from feature absorb.
  2. The registry/update-check header residual (RES-01) and PTY-teardown/test-infra residual
     (RES-02) are individually reviewed and absorbed or explicitly skipped with reasoning, not
     silently dropped.
  3. `373a67ae65fdb94898b49469d562a263ba2083ee` (#1369, crossbeam-epoch 0.9.18→0.9.20) is
     prioritized ahead of routine DEPS-cluster absorb — it is the direct fix for the live
     RUSTSEC-2026-0204 vulnerability the fork's `Cargo.lock` currently carries (see DEPS Cluster
     cargo-audit cross-reference above).
  4. Both cross-target clippy gates (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) and
     `make ci` are GREEN locally after the Phase 112 absorb, consistent with VERIFY-01's framing
     in Phase 111.

**This proposal requires operator approval before Phase 109 planning begins -- Phase 108 does not
apply this amendment to ROADMAP.md.** No edit to `.planning/ROADMAP.md` was made by this task (see
Completeness Verification's SC4 statement below for the diff-empty confirmation).

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
| NET | proxy/network (deny_domain, SPIFFE, SigV4, sibling-route, no_proxy, HTTP_PROXY, credential-capture plumbing) | 12 | 10/12 will-sync (9 adopt + 1 adapt: `3b207eeb` #1374 `deny_domain`, per ADR-108) + 2/12 won't-sync (target subsystem absent): `6fb7ecbf` #1430, `23d93fc9` #1437, per `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` | no (0/12 — grep-confirmed, zero `cfg(target_os = "windows")`/`cfg(windows)` hits, per Plan 108-03's NET table) | yes (9/12, per Plan 108-03's NET table security-relevant column) | 109 |
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

### SEC-09 Carry-Forward Note (Phase 112, D-01)

**Filed:** 2026-08-05 (Phase 112, Plan 112-08 close-out; premise re-confirmed live in Plan 112-01
and `112-DISPOSITION-TABLE.md`).

Upstream `f6f027511f7899f1735f1188747086696e40412e` (#1301, "fix(execution-runtime): allow
env_credentials + command_policies on non-shim entry") relaxes a guard block in
`execution_runtime.rs` gated on `tool_sandbox_runtime.is_some() && !loaded_secrets.is_empty() &&
tool_sandbox_initial_shim.is_none()` — removing the restriction so `env_credentials`/
`command_policies` become usable even when the initial process is not the tool-sandbox shim entry
point.

**This fork does not currently have that guard to relax.**
`grep -rn "command_policies\|tool_sandbox_runtime\|tool_sandbox_initial_shim"
crates/nono-cli/src/execution_runtime.rs` returns 0 hits (re-confirmed live 2026-08-05, Plan
112-01) because the fork never absorbed the base `tool_sandbox_runtime`/`command_policies`
subsystem (PR #1105, v0.65.0) — the entire tool-sandbox-surface cluster immediately above (20
commits) is the standing divergence deferred to the **v3.7 Windows Tool-Sandbox Parity**
milestone.

**Obligation for whichever future plan absorbs the tool-sandbox base subsystem:** when
`tool_sandbox_runtime`/`command_policies` are eventually absorbed into this fork, the fork MUST
consciously decide whether to bring `f6f02751`'s relaxed non-shim-entry posture along with it, or
preserve a stricter guard equivalent to upstream's pre-`f6f02751` behavior. This is **not an
automatic inheritance** — absorbing the base subsystem at its current (post-`f6f02751`) upstream
state would silently adopt the relaxed posture without a fork-side security review ever having
happened. A future v3.7 planner reading this cluster's work-list should treat `f6f02751` as a
named line item requiring its own disposition call, not a hunk that rides along for free with the
base absorb.

**Disposition (Phase 112): won't-sync (target subsystem absent) — LOCKED per CONTEXT.md D-01.** No
code change. This note is the deliverable.

### SPIFFE Carry-Forward Note (Phase 113, D-02)

**Filed:** 2026-08-06 (Phase 113, Plan 113-08 close-out; premise re-confirmed live via direct
`grep`/`git log -S` reads during Plans 113-01 through 113-08).

Upstream `c831dade422f2bdf37d7429af0423cafa0a60c06` (#1272, "feat(proxy): add SPIFFE/SPIRE
workload identity auth for upstream routes") includes two `tls_intercept/` hunks —
`h2_forward.rs` (+192) and `handle.rs` (+286), 478 lines total — implementing
`handle_spiffe_intercept_request`, a SPIFFE-authenticated request handler for TLS-intercepted
streams, parallel to (not a prerequisite of) the reverse-proxy SPIFFE handlers this fork built in
`reverse.rs`.

**This fork does not currently have a `tls_intercept/` module for that handler to land on.**
`grep -rn "mod tls_intercept|struct.*TlsIntercept|fn.*tls_intercept" crates/nono-proxy/src/`
returns 0 hits (re-confirmed live 2026-08-06, Plan 113-08) — no TLS-interception subsystem exists
anywhere in this fork's proxy. `proj/ADR-113-spiffe-disposition.md` (D-01) records the full
positive proof that dropping these two hunks does not leave any *reachable* fork route type
silently unauthenticated, since the fork cannot reach a TLS-intercepted request path at all today.

**Obligation for whichever future plan introduces a `tls_intercept/` module:** when TLS
interception is eventually absorbed into this fork (the fork's own `ProxyHandle::intercept_ca_path()`
returning `None` unconditionally, per `112-07`, is the standing marker that this has not yet
happened), the fork MUST consciously decide whether to bring `c831dade`'s
`handle_spiffe_intercept_request` SPIFFE-authentication path along with it, or whether the
reverse-proxy SPIFFE handlers this phase built (`handle_spiffe_route`,
`handle_spiffe_assertion_credential` in `reverse.rs`) already provide sufficient coverage for
whatever request shapes the new TLS-intercept module exposes. This is **not an automatic
inheritance** — absorbing a `tls_intercept/` module without re-examining this decision risks either
silently re-introducing an unauthenticated SPIFFE-declared-route gap on the newly-reachable
intercepted-TLS path, or duplicating enforcement logic that already exists in `reverse.rs`. A
future planner reading this cluster's work-list should treat `c831dade`'s `tls_intercept/` hunks as
a named line item requiring its own disposition call against the reverse-proxy SPIFFE handlers'
actual shape at that time, not a hunk that rides along for free.

**A second, newly-identified divergence surfaced by this phase's research, not previously recorded
in any ledger cluster:** upstream commit `b1ecbc02` ("feat(profile): support OAuth2 auth config in
custom_credentials", `git describe` = `v0.38.0-3-gb1ecbc02`, 974 lines / 8 files, landed shortly
after upstream `v0.38.0` — well before this milestone's `v0.66.0..v0.69.0` sync window and never
referenced by any prior Phase 108 cluster). `113-RESEARCH.md`'s symbol-level verification found
that `c831dade`'s `handle_spiffe_assertion_credential`/`handle_spiffe_route` machinery references
`CredentialStore.oauth2_routes`/`get_oauth2()`/`struct OAuth2Route`,
`LoadedRoute.requires_managed_credential`/`.managed_auth_mechanism`/`.managed_injection_mode`/
`.missing_managed_credential()`, and `crate::forward` (`UpstreamSpec`/`UpstreamStrategy`/
`forward_request`) — a general (non-SPIFFE) OAuth2 `client_credentials` route-wiring layer that
traces to `b1ecbc02`, not to any commit in this milestone's window, and has never been absorbed
into this fork. Phase 113 built a SPIFFE-only slice of equivalent functionality
(`CredentialStore.spiffe_assertion_routes`/`get_spiffe_assertion()`,
`LoadedRoute.managed_auth`/`has_spiffe_source()`) without absorbing `b1ecbc02`'s general layer —
see `proj/ADR-113-spiffe-disposition.md`'s OD-1 section for the full disposition and rationale
(mirroring `proj/ADR-111-resource-limits-boundary.md`'s "reject the core-module absorb" shape).

**Obligation for whichever future plan absorbs `b1ecbc02` proper:** that plan must read
`proj/ADR-113-spiffe-disposition.md`'s OD-1 section before designing the general
`client_credentials` route-wiring layer, to understand that Phase 113's SPIFFE-scoped symbols
(`spiffe_assertion_routes`, `get_spiffe_assertion()`, `managed_auth`, `has_spiffe_source()`) were
built independently and must not be silently renamed, removed, or restructured by an absorb that
assumes it is starting from a clean slate.

**Disposition (Phase 113): ADAPT-DOWN for the `tls_intercept/` hunks (won't-sync, target module
absent) — LOCKED per `113-CONTEXT.md` D-01; scope-boundary decline for `b1ecbc02`'s general
OAuth2-route-wiring layer — LOCKED per `113-CONTEXT.md`'s discretionary OD-1 grant, recorded
permanently in `proj/ADR-113-spiffe-disposition.md`.** No code change for either. This note and
the ADR are the deliverables.

### SEC-02 Carry-Forward Note (Phase 114, D-10)

**Filed:** 2026-08-07 (Phase 114, Plan 114-11 close-out).

Upstream `9b692e07` (SEC-02a, "feat(oauth): add declarative sandboxed OAuth capture"),
`3c59c62e` (SEC-02b, "fix(oauth): harden capture security boundaries"), and `d033c631` (SEC-02c,
"test(oauth): consume provider stdin in header fixture") were carved out to Phase 114 by Phase
112 (`112-OAUTH-CAPTURE-DISPOSITION.md`) as reality-check evidence with no adopt/adapt/decline
verdict made. Phase 114 makes that verdict: **ADAPT — a fork-native response buffer-and-rewrite
enforcement point** (`relay_response_with_capture()`, `crates/nono-proxy/src/reverse.rs`), built
against the fork's own `reverse.rs`/`server.rs` shape rather than importing upstream's
`forward.rs`/`oauth_capture/` subsystem, which remains absent from the fork. Full disposition
record: `proj/ADR-114-oauth-capture-disposition.md`.

**The scope limit, stated plainly:** capture works only for OAuth token endpoints reachable as
**configured reverse-proxy routes**. It does not cover agent-initiated flows to arbitrary hosts
via CONNECT — those are denied outright (D-06's cross-path fail-closed guard,
`114-07-SUMMARY.md`), not rewritten, because no buffer-and-rewrite point exists on that path.

**`149abde0` ("feat(proxy): add tls interception for l7-bearing connect routes") is named here as
the tracked divergence behind this scope limit**, mirroring how Phase 113 filed `b1ecbc02` above.
`149abde0` is the sole commit that creates `forward.rs` and brings TLS interception into
upstream's proxy; it is not absorbed, and the fork's no-MITM stance
(`ProxyHandle::intercept_ca_path()` returns `None` unconditionally) is a standing decision this
phase reaffirmed rather than reopened. TLS interception is the named condition under which
SEC-02's scope limit could be lifted — see `proj/ADR-114-oauth-capture-disposition.md`'s "OD: The
TLS-Interception Boundary Against `149abde0`" section for the full reasoning, including the
corrected finding that the original blocker was response **buffering**, not response
**visibility** (the fork's reverse proxy already had plaintext visibility into upstream responses
for configured routes; `149abde0`/TLS interception was never required for that subset).

**Obligation for whichever future plan absorbs `149abde0`:** when TLS interception is eventually
absorbed into this fork, the fork MUST consciously decide whether to wire
`relay_response_with_capture()` (or an equivalent) into whatever new intercepted-TLS response path
that absorb creates, in order to lift SEC-02's scope limit for agent-initiated CONNECT flows. This
is **not an automatic inheritance** — absorbing TLS interception without re-examining this decision
would leave capture-declared routes' CONNECT-path guard (D-06) silently denying traffic that a
rewrite point could otherwise have served safely. A future planner reading this note should treat
`149abde0` as a named line item requiring its own disposition call against
`relay_response_with_capture()`'s actual shape at that time, not a hunk that rides along for free.

**Disposition (Phase 114): ADAPT — fork-native enforcement point built, real code shipped, with a
named permanent scope limit — LOCKED per `114-CONTEXT.md` D-01r/D-10/D-11, recorded permanently in
`proj/ADR-114-oauth-capture-disposition.md`.** This note and the ADR are the deliverables.

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
| `6fb7ecbf36e5d18760d8084b7f4900e93582004c` | bug: Fix SigV4 URI generation errors for uri's that have encoded characters in them (#1430) | 1 file (`nono-proxy/src/aws/sign.rs`), 16+/4- | no | yes | **NET-03** | won't-sync (target subsystem absent) | Target file `aws/sign.rs` is absent from the fork's tree, per `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` (Phase 109 re-verification, 2026-07-29) |
| `23d93fc96abf795d672c712e9c1834a8f97aa0aa` | fix(proxy): don't cross-deny sibling routes sharing an upstream (#1437) | 1 file (`nono-proxy/src/tls_intercept/handle.rs`), 87+/0- | no | yes | **NET-03** | won't-sync (target subsystem absent) | Target file `tls_intercept/handle.rs` is absent from the fork's tree, per `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` (Phase 109 re-verification, 2026-07-29) |
| `1619275caa32b7b96e3eee56c33c71dfce777bbf` | feat: add profile-declared no_proxy bypass support (#1415) | 17 files (schema.json, profile-authoring-guide.md, launch_runtime.rs, main.rs, profile/{mod,cmd,runtime}.rs, proxy_runtime.rs, sandbox_prepare.rs, nono-proxy/{config,filter,route,server}.rs, tls_intercept/handle.rs, +3 docs), 2065+/99- | no | yes | **NET-03** | adopt | `pub(crate) no_proxy: Vec<String>` field ×3 + `pub(crate) fn` helpers (`validate_no_proxy_allow_domain_conflicts`, `no_proxy_host_pattern_matches`, `strip_no_proxy_port`, `normalise_no_proxy_host_pattern`, `normalise_no_proxy_env_entry`, `parse_host_ip_literal`, `is_proxy_denied_metadata_ip`, `extract_host_port`, `format_host_port`) — all intra-crate `pub(crate)`, no cross-crate re-export |
| `726ac1f1b5fd7b6de2d86b9fccf1d72660d3e32a` | feat(proxy): support plain HTTP forward-proxying via HTTP_PROXY (#1335) | 1 file (`nono-proxy/src/server.rs`), 815+/5- | no | yes | **NET-03** | adopt | Clean — single-file `server.rs` addition, no new `pub` items in the diff |
| `c344efb006365ba596b20b843f29fabdbbdc847e` | fix(why): respect proxy domain filter in --profile and --self host queries (#1372) | 2 files (`execution_runtime.rs`, `query_ext.rs`), 131+/13- | no | yes | none | adopt | Clean — hand-verified: diagnostic-command bugfix (`nono why --profile`/`--self`) making the `why` reporter consult the same `HostFilter`/`network_policy` resolution the proxy already enforces; does not itself implement any NET-01/02/03-named mechanism, so mapped `none` rather than force-mapped to NET-01 |
| `4192bfa58101d0ef9737b60243a39695fc1e0fa8` | fix(proxy): skip credential_capture entries with missing helper binaries (#1368) | 1 file (`nono-cli/src/proxy_runtime.rs`), 263+/16- | no | yes | none | adopt | Clean — includes a portable path-separator security fix (`resolve_capture_command` previously checked only the native `MAIN_SEPARATOR`, letting a relative command containing the non-native separator bypass bare-command validation on Windows); credential_capture plumbing, not a NET-01/02/03-named item |
| `261bbd68d984f58c5b7376143fdacbeaffef1ccc` | fix(tests): raise credential-capture test timeout to reduce macOS CI flakiness | 1 file (`nono-cli/src/proxy_runtime.rs`), 1+/1- | no | no | none | adopt | Clean — test-only timeout constant change |
| `3672ea10c7d45b340bc389e1d5cf0a26dfa5db34` | fix(tests): share stdin-manipulation lock between capture_helper stdin tests (#1327) | 1 file (`nono-cli/src/proxy_runtime.rs`), 7+/5- | no | no | none | adopt | Clean — test-only lock-sharing fix |
| `8255a27a1d8bedf878a7c8c67f9007ca592962d1` | refactor load_with_diagnostics to be async (#1287) | 4 files (`credential.rs`, `oauth2.rs`, `server.rs`, `tls_intercept/h2_forward.rs`), 67+/51- | no | no | none | adopt | Clean — mechanical `fn` → `async fn` signature conversion (`pub fn` becomes `pub async fn`, not a new `pub` item), no behavior change per commit message |
| `7d23bba683036789163385afa7f5c2f5888886a5` | fix(proxy): separate stdin and stderr inheritance for credential helpers (#1300) | 5 files (schema.json, profile-authoring-guide.md, profile/mod.rs, proxy_runtime.rs, credential-injection.mdx), 190+/6- | no | yes | none | adopt | Clean — hand-verified: fixes a terminal-keypress-theft security bug (credential helpers inheriting stdin unintentionally) by splitting `interaction.stdio` into separate `stdio`(stderr)/`stdin` schema keys; not a NET-01/02/03-named item |

**NET disposition summary:** 1 ADAPT (`3b207eeb` #1374, per ADR-108) + 9 adopt + 2 won't-sync
(target subsystem absent) = 12. All 12 rows map to exactly one of `NET-01`/`NET-02`/`NET-03`/`none`
(5 map `none` — diagnostic/test/refactor/credential-plumbing commits that ride alongside the
network absorb but do not themselves implement a named NET-0X mechanism). Zero blank cells.
`windows-touch: no` for all 12 (grep-confirmed, zero `cfg(target_os = "windows")`/`cfg(windows)`
hits in any of the 12 diffs).

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
| `63c9589fa30f0244c542c026faf27d517aaa9b39` | docs(credential-injection): document AWS SigV4 proxy signing (#1329) | needs-doc-follow-up, **deferred** — verified: fork carries `docs/cli/features/credential-injection.mdx`, but `6fb7ecbf` (SigV4 URI-encoding fix, NET cluster) is now dispositioned `won't-sync (target subsystem absent)` — the fork has no `aws/sign.rs` SigV4-signing implementation for this doc to describe (per `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`). Hold this doc unless/until full AWS SigV4 signing is ever built in a future phase, then reconcile alongside it |
| `da3a90f247bf80ffe8bcc470154e83f764033803` | docs(nogent): add nogent markdown file (#1288) | safe-to-ignore — verified: fork has no `NOGENT.md`; this is upstream's own GitHub-App-onboarding document ("Once this is in place, we can add the repo to the GitHub App"), not relevant to the fork's separate identity/tooling |

**DOCS disposition summary:** 8/8 individually reviewed. 3 needs-doc-follow-up (2 for
`profiles-groups.mdx`, 1 for `codex.mdx`). 2 needs-doc-follow-up-but-deferred (README
tool-sandboxing section, held for v3.7; and `credential-injection.mdx`'s AWS SigV4 doc, held
pending a future AWS SigV4 signing phase). 3 safe-to-ignore (community-health files + NOGENT.md
the fork does not carry and does not need, being solo-maintained rather than community-governed).

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

---

## Carve-out Re-touch Check (D-22/D-23)

Per D-22 (carried forward from Phase 94 D-04/D-05, expanded in 98 D-07): for each of the 7
accumulated fork carve-out surfaces named in `108-05-PLAN.md`'s `interfaces` block, run
`git log --no-merges --oneline <window> -- <exact-path(s)>` and record the literal command plus
its literal output. **A zero-hit result is recorded explicitly as "clean — no re-touch in
window" — silence is never treated as evidence.** `RANGE` below is the pinned window
`d817ed53663c6bba4669ee7a5bfb41b35971fd1b..59bdace7e905c05c127f480dc6d2a8c3a3331392`.

### 1. CR-02 (audit bypass invariant) — `crates/nono/src/audit.rs`

```bash
git log --no-merges --oneline $RANGE -- crates/nono/src/audit.rs
```
```
c831dade feat(proxy): add SPIFFE/SPIRE workload identity auth for upstream routes (#1272)
```
**HIT (1 commit)** — `c831dade422f2bdf37d7429af0423cafa0a60c06` is already classified in the
**NET cluster per-commit table** (Plan 108-03), dispositioned `adopt`/`NET-02`. That row's own
re-export scan (hand-verified) found only intra-`nono-proxy` `pub mod auth;`/`pub mod spiffe;` and
`pub(crate)` helpers touching `audit.rs`/`undo/types.rs` — no CR-02 audit-bypass-invariant
regression identified. Not re-analyzed here; routed to its existing table.

### 2. CR-01 (FFI `clear_last_call_state`) — `bindings/c/src/{lib,diagnostic,capability_set,fs_capability,sandbox,state,query}.rs`

```bash
git log --no-merges --oneline $RANGE -- bindings/c/src/lib.rs bindings/c/src/diagnostic.rs \
  bindings/c/src/capability_set.rs bindings/c/src/fs_capability.rs bindings/c/src/sandbox.rs \
  bindings/c/src/state.rs bindings/c/src/query.rs
```
```
8a4237f2 refactor(seccomp): introduce SeccompPolicy struct and client-driven selection (#1283)
```
**HIT (1 commit)** — `8a4237f2ee0dc33bc1e5afdd39c0db8ca6f5ed38` is already classified in the
**tool-sandbox-split residue table** (Plan 108-04). Its `bindings/c/src/sandbox.rs` row is
diff-verified there as a 1-line call-site rename (`nono::Sandbox::apply` → `apply_auto`) with an
explicit re-export scan result of "Clean" (zero `pub mod`/`pub use`/`extern crate`, 4 intra-`nono-cli`
`pub(crate)` hits only). No CR-01 regression. Not re-analyzed here.

### 3. Proxy fork-preserve surface — `crates/nono-proxy/src/{route,connect,reverse,server}.rs`, `crates/nono-cli/src/proxy_runtime.rs`

```bash
git log --no-merges --oneline $RANGE -- crates/nono-proxy/src/route.rs crates/nono-proxy/src/connect.rs \
  crates/nono-proxy/src/reverse.rs crates/nono-proxy/src/server.rs crates/nono-cli/src/proxy_runtime.rs
```
```
1619275c feat: add profile-declared no_proxy bypass support (#1415)
c831dade feat(proxy): add SPIFFE/SPIRE workload identity auth for upstream routes (#1272)
ca888108 fix: missing ~/.cache on macOS (#1378)
726ac1f1 feat(proxy): support plain HTTP forward-proxying via HTTP_PROXY (#1335)
1f54f4ae fix(command-policy): resolve command_policies binaries once, in parallel, with caching (#1373)
3b207eeb feat(proxy): add deny_domain to block domains through the proxy (#1374)
4192bfa5 fix(proxy): skip credential_capture entries with missing helper binaries (#1368)
7c20dc75 fix(tool-sandbox): resolve command policy paths against the live cwd (#1339)
d033c631 test(oauth): consume provider stdin in header fixture
9b692e07 feat(oauth): add declarative sandboxed OAuth capture
2663e990 feat(cli): add standalone `nono proxy` command (#1261)
261bbd68 fix(tests): raise credential-capture test timeout to reduce macOS CI flakiness
3672ea10 fix(tests): share stdin-manipulation lock between capture_helper stdin tests (#1327)
2cbaa9a0 feat(profile): expand $VAR tokens from process env in profile paths and capture commands (#1296)
a3243907 feat(gpu): harden NVIDIA procfs mediation (#1284)
8255a27a refactor load_with_diagnostics to be async (#1287)
7d23bba6 fix(proxy): separate stdin and stderr inheritance for credential helpers (#1300)
8a4237f2 refactor(seccomp): introduce SeccompPolicy struct and client-driven selection (#1283)
```
**HIT (18 commits)** — every SHA above is already classified in an existing table; none is
un-routed:
- **NET cluster (Plan 108-03):** `c831dade`, `3b207eeb`, `1619275c`, `726ac1f1`, `4192bfa5`,
  `261bbd68`, `3672ea10`, `8255a27a`, `7d23bba6`
- **CORE cluster (Plan 108-03):** `ca888108`
- **PROF cluster (Plan 108-03):** `2cbaa9a0`
- **tool-sandbox-split (Plan 108-04):** `1f54f4ae`, `7c20dc75`, `8a4237f2`
- **security-residual-and-misc (Plan 108-01, D-18-named anchors):** `d033c631`, `9b692e07`,
  `2663e990`, `a3243907`

### 4. Endpoint-policy surface (Phase 95) — `crates/nono-cli/src/network_policy.rs`, `crates/nono-proxy/src/{config,credential,route,server}.rs`

```bash
git log --no-merges --oneline $RANGE -- crates/nono-cli/src/network_policy.rs \
  crates/nono-proxy/src/config.rs crates/nono-proxy/src/credential.rs \
  crates/nono-proxy/src/route.rs crates/nono-proxy/src/server.rs
```
```
1619275c feat: add profile-declared no_proxy bypass support (#1415)
c831dade feat(proxy): add SPIFFE/SPIRE workload identity auth for upstream routes (#1272)
ca888108 fix: missing ~/.cache on macOS (#1378)
726ac1f1 feat(proxy): support plain HTTP forward-proxying via HTTP_PROXY (#1335)
3b207eeb feat(proxy): add deny_domain to block domains through the proxy (#1374)
9b692e07 feat(oauth): add declarative sandboxed OAuth capture
2663e990 feat(cli): add standalone `nono proxy` command (#1261)
0ecc476b feat: implement aws authentication for the MiTM proxy  (#1195)
8255a27a refactor load_with_diagnostics to be async (#1287)
```
**HIT (9 commits)** — all already routed: **NET cluster** (`1619275c`, `c831dade`, `726ac1f1`,
`3b207eeb`, `8255a27a`); **CORE cluster** (`ca888108`); **security-residual-and-misc**
(`9b692e07`, `2663e990`, `0ecc476b` — the last two D-18-named anchors).

### 5. `linux.rs` restored invariants — `crates/nono/src/sandbox/linux.rs`

```bash
git log --no-merges --oneline $RANGE -- crates/nono/src/sandbox/linux.rs
```
```
d5803b99 feat: add port range support to sandbox profiles (#1398)
d84b4818 fix(sandbox): grant Refer in execute-restriction layer on Linux (#1397)
ea334d2b fix(linux): use u64 for fs_type_unsupported to fix musl build (#1332)
a3243907 feat(gpu): harden NVIDIA procfs mediation (#1284)
8a4237f2 refactor(seccomp): introduce SeccompPolicy struct and client-driven selection (#1283)
```
**HIT (5 commits)** — `d5803b99`, `ea334d2b`, `8a4237f2` are already classified in
**tool-sandbox-split** (Plan 108-04; all 3 are D-05-named worked examples with dedicated finding
notes). `d84b4818`, `a3243907` are D-18-named anchors in **security-residual-and-misc**
(Plan 108-01).

### 6. ADR-86 Windows denial-rendering carve-out — `crates/nono-cli/src/exec_strategy_windows/`

```bash
git log --no-merges --oneline $RANGE -- crates/nono-cli/src/exec_strategy_windows/
```
```
(no output)
```
**clean — no re-touch in window.** No upstream commit in this sync touches the fork's D-02
Windows denial-rendering carve-out (`proj/ADR-86-library-boundary-convergence.md`).

### 7. v3.2 signed-override surface (Rust core side) — `crates/nono/src/audit.rs` (`PolicyOverrideApplied` + EventIDs 10006-10010)

```bash
git log --no-merges --oneline $RANGE -- crates/nono/src/audit.rs
```
```
c831dade feat(proxy): add SPIFFE/SPIRE workload identity auth for upstream routes (#1272)
```
**HIT (1 commit)** — the identical result to surface #1 (CR-02) above, since both carve-outs live
in the same file. Already routed to the **NET cluster** (Plan 108-03) per surface #1's analysis;
not re-analyzed twice.

### Carve-out re-touch summary

All 7 surfaces have an explicit stated result: 6 HIT (with every named SHA cross-referenced to an
existing table by section name, none un-routed) + 1 clean ("no re-touch in window" —
`exec_strategy_windows/`). Zero surfaces are left without a stated result.

---

## Security-Relevant Rollup (D-20)

Consolidated table of every row flagged `security-relevant=yes` in Plan 108-03's NET/PROF/CORE
per-commit tables, plus the 10 D-18-named anchor SHAs from the security-residual-and-misc cluster
(Plan 108-01). **Note on method:** the 10 D-18-named SHAs are not individually flagged with an
explicit `security-relevant=yes` *cell* in any 108-03/108-04 table — `108-CONTEXT.md` D-18 itself
designates them security-relevant, and Plan 108-01's security-residual-and-misc table records that
designation via its "anchor: named (D-18)" column rather than a `security-relevant` column. This
task treats the D-18 naming as equivalent evidence and includes all 10 here — omitting them because
they use a differently-labeled source column would fail this plan's own explicit must-have that
all 10 D-18-named SHAs appear in this rollup.

| sha | subject | source cluster | requirement-mapping |
|-----|---------|-----------------|----------------------|
| `3b207eeb884bd71b0fc10f0123fa89fffe9f7955` | feat(proxy): add deny_domain to block domains through the proxy (#1374) | NET | NET-01 |
| `c831dade422f2bdf37d7429af0423cafa0a60c06` | feat(proxy): add SPIFFE/SPIRE workload identity auth for upstream routes (#1272) | NET | NET-02 |
| `6fb7ecbf36e5d18760d8084b7f4900e93582004c` | bug: Fix SigV4 URI generation errors (#1430) | NET | NET-03 |
| `23d93fc96abf795d672c712e9c1834a8f97aa0aa` | fix(proxy): don't cross-deny sibling routes sharing an upstream (#1437) | NET | NET-03 |
| `1619275caa32b7b96e3eee56c33c71dfce777bbf` | feat: add profile-declared no_proxy bypass support (#1415) | NET | NET-03 |
| `726ac1f1b5fd7b6de2d86b9fccf1d72660d3e32a` | feat(proxy): support plain HTTP forward-proxying via HTTP_PROXY (#1335) | NET | NET-03 |
| `c344efb006365ba596b20b843f29fabdbbdc847e` | fix(why): respect proxy domain filter in --profile and --self host queries (#1372) | NET | none |
| `4192bfa58101d0ef9737b60243a39695fc1e0fa8` | fix(proxy): skip credential_capture entries with missing helper binaries (#1368) | NET | none |
| `7d23bba683036789163385afa7f5c2f5888886a5` | fix(proxy): separate stdin and stderr inheritance for credential helpers (#1300) | NET | none |
| `ae1c513e6ed1f1a0628dbb690c3f1c984fb7a225` | feat(profile): add platform_overrides field for per-OS profile patches (#1371) | PROF | PROF-01 |
| `719975cf03ea303f835061089279eb6c50245ccf` | fix(profile): preserve platform_overrides through extends resolution (#1380) | PROF | PROF-01 |
| `2cbaa9a017b14ddc45219474b6eebf94727787ef` | feat(profile): expand $VAR tokens from process env (#1296) | PROF | PROF-02 |
| `b620ed8e4572359afa4ed9cff4d803c8e9064f26` | feat(policy): add bun runtime preset (#1305) | PROF | PROF-04 |
| `f016b2d56dcbe4aad2bb322acd7574d38b5c13f7` | mise policy (#1387) | PROF | PROF-04 |
| `f58c7c2242d3790f9c390d4bf0dd808ba1261c77` | feat(profile): support CLI profile extends (#1320) | PROF | none |
| `ca888108fe5983be866e8d1a6eccf96edc2a8dd5` | fix: missing ~/.cache on macOS (#1378) | CORE | CORE-01 |
| `e6d26871f0498e7dc7a867e67af5c6136b84f91c` | feat: resource limiting (#1269) | CORE | CORE-02 |
| `34c2c975d649844923cf1515be94de624c689c6c` | feat(resources): cap sandbox process count with --max-processes (#1403) | CORE | CORE-02 |
| `0ecc476bf0db3c9509bcc2bd8efae43832c764b8` | feat: implement aws authentication for the MiTM proxy (#1195) | security-residual-and-misc (D-18 named) | none |
| `9b692e07ee4dd156d85588b12b63691778d9e9be` | feat(oauth): add declarative sandboxed OAuth capture | security-residual-and-misc (D-18 named) | none |
| `3c59c62e7ddcc2d3a1f1c161bd039b5f2b0b5f7b` | fix(oauth): harden capture security boundaries | security-residual-and-misc (D-18 named) | none |
| `d033c63111472711e242f4067eb4be04aeaf618a` | test(oauth): consume provider stdin in header fixture | security-residual-and-misc (D-18 named) | none |
| `a32439074a61eea21eb374a91fdbcebb91b0afe6` | feat(gpu): harden NVIDIA procfs mediation (#1284) | security-residual-and-misc (D-18 named) | none |
| `f943fb5a0721352e85d92ef48e3e4d5b36747486` | fix(trust): add predicate field to distinguish nono trust policies from foreign JSON (#1333) | security-residual-and-misc (D-18 named) | none |
| `d84b4818f824b7660d1a4d95ec2e56f322b6fa63` | fix(sandbox): grant Refer in execute-restriction layer on Linux (#1397) | security-residual-and-misc (D-18 named) | none |
| `ac5ccd70712782f4a97c54916ee18b03c650f56f` | fix(sandbox): keep orphaned descendants in supervisor ancestry for seccomp-notify mediation (#1401) | security-residual-and-misc (D-18 named) | none |
| `2663e9900fe1317cec0949c18b7a0e6b39321602` | feat(cli): add standalone nono proxy command (#1261) | security-residual-and-misc (D-18 named) | none |
| `a5a441c25769ecec663aed1d7038ab2b8814d43f` | fix(profile): empty allow_vars no longer strips all env vars (#1204) | security-residual-and-misc (D-18 named) | none |

**Rollup row count: 28** (9 NET + 6 PROF + 3 CORE + 10 security-residual-and-misc anchors). All 10
D-18-named SHAs (`0ecc476b`, `9b692e07`, `3c59c62e`, `d033c63111472711e242f4067eb4be04aeaf618a`,
`a3243907`, `f943fb5a`, `d84b4818`, `ac5ccd70`, `2663e990`, `a5a441c25769ecec663aed1d7038ab2b8814d43f`)
appear verbatim above. 28 ≥ 10 — acceptance threshold met.

---

## Completeness Verification

**Per-cluster/bucket arithmetic (finest non-overlapping partition of all 100 commits, 9
buckets):**

```
NET 12 + PROF 8 + CORE 4 + tool-sandbox-pure 9 + tool-sandbox-split 11 +
security-residual-and-misc 18 + DEPS 19 + CI 11 + DOCS 8 = 100
```

`12 + 8 + 4 + 9 + 11 + 18 + 19 + 11 + 8 = 100`. ✓ (The coarser CODE/DEPS/CI/DOCS bucket view in
the Full Commit Accounting section — CODE 62 = NET+PROF+CORE+tool-sandbox-pure+tool-sandbox-split+
security-residual-and-misc = 12+8+4+9+11+18 = 62 — is a rollup of this same finest partition, not
an independent count; the two views are reconciled, not duplicated.)

**Unique-SHA dedup pass:** every SHA appearing as a **primary row** (the commit each of the 9
finest-partition tables above is directly enumerating) was extracted, sorted, and deduplicated,
then cross-checked against a fresh `git log --no-merges --format='%H' $RANGE | sort` of the pinned
window:

```bash
RANGE="d817ed53663c6bba4669ee7a5bfb41b35971fd1b..59bdace7e905c05c127f480dc6d2a8c3a3331392"
git log --no-merges --format='%H' $RANGE | sort > git_shas.txt   # 100 lines
# (100 primary-row SHAs from the 9 finest-partition tables, sorted+deduped) > ledger_shas.txt
comm -23 git_shas.txt ledger_shas.txt   # in git log but not ledger  -> empty
comm -13 git_shas.txt ledger_shas.txt   # in ledger but not git log  -> empty
sort -u ledger_shas.txt | wc -l          # -> 100
```

**Result: 100 unique primary-row SHAs, exact 1:1 match against the git log's 100 non-merge commits
in the window. Zero missing, zero extra, zero duplicate primary rows.** Cross-reference mentions
(carve-out re-touch HIT lines, the Security-Relevant Rollup's `source cluster` column, the
Requirement Coverage Gap's SHA lists, the CODE/DEPS/CI/DOCS coarse bucket tables reproducing the
same 62/19/11/8 rows in a different grouping) are *not* counted as additional primary rows — per
this task's own defect definition, only the same SHA appearing as a primary row in two different
**cluster** tables would be a defect, and none was found.

**Prior-plan arithmetic discrepancy caught by this sweep (recorded, not silently fixed):** Plan
108-03's NET cluster summary prose states "5 map `none`" (actual: 6) and its PROF cluster summary
prose states "4 rows map `none`" (actual: 3) — see "Requirement Coverage Gap (D-18/D-19)" above
for the full detail. Both are one-off counting errors in that plan's own summary sentence, not in
its table data; the errors happen to cancel in the combined total (9 either way), so no downstream
total was corrupted by them, but they are genuine defects worth flagging for anyone revisiting
108-03.

### ROADMAP.md Phase 108 Success Criteria — final disposition

1. **SC1** ("classifies every substantive commit adopt/adapt/skip/split with a windows-touch flag
   and an ADR-review verdict per cluster") — **satisfied.** All 5 CODE-bucket clusters (NET, PROF,
   CORE, tool-sandbox-surface, security-residual-and-misc) carry a finalized `disposition`,
   `windows-touch`, and `security-relevant` cell in the Cluster Summary table (zero `TBD`
   remaining, per Plan 108-05's first task — see "Cluster Summary" + "Cluster Summary Rollup
   Support Notes" above). The `deny_domain` ADR-review verdict lives in
   `proj/ADR-108-deny-domain-posture.md` (D-10/D-11/D-12); every other cluster's disposition
   verdict is recorded per-commit in the NET/PROF/CORE tables (Plan 108-03) and the
   tool-sandbox-pure/split + security-residual-and-misc rollups (Plans 108-04/108-05).
2. **SC2** ("Re-export/public-surface diffs are inspected (not just `git diff --name-only`)") —
   **satisfied.** Every NET/PROF/CORE row carries an explicit "re-export scan" cell (Plan 108-03,
   hand-verified `pub mod`/`pub use`/`extern crate` greps). The tool-sandbox-split residue table
   (Plan 108-04) diff-verifies every absorb-marked row's actual mechanism, not just its touched
   path. The highest-risk cross-crate surface in this window (`8a4237f2`'s `bindings/c/src/sandbox.rs`
   touch) got a dedicated re-export scan finding "Clean — 1-line call-site rename, zero
   `pub mod`/`pub use`." `e6d26871`'s `pub mod resource; pub use resource::ResourceLimits;` addition
   to `crates/nono/src/lib.rs` was flagged as a genuine cross-crate re-export in the Threat Flags
   table (not a false negative).
3. **SC3** ("The 7 tool-sandbox refinement PRs ... explicitly recorded DEFERRED->v3.7") —
   **satisfied.** All 7 named PRs (#1280, #1322, #1325, #1384, #1394, #1413, #1417) are
   DEFERRED->v3.7 in "tool-sandbox-pure and tool-sandbox-split" above — 4 as pure-table rows
   (#1280, #1325, #1394, #1413) and 3 as split commits whose module-scoped portions are marked
   DEFERRED->v3.7 in a dedicated supplementary table (#1322, #1384, #1417), with their non-module
   residue separately accounted for rather than blanket-marked pure.
4. **SC4** ("the ledger maps each will-sync cluster onto Phase 109/110/111") — **not satisfied as
   originally written; this is the corrected outcome CONTEXT.md D-19 anticipates, not a gap left
   open by omission.** 27 non-tool-sandbox CODE commits (the entire security-residual-and-misc
   cluster plus 9 ride-along NET/PROF commits) map to none of Phase 109/110/111's requirements.
   The corrected SC (per ROADMAP.md's own D-19 annotation) is satisfied instead: the gap is
   surfaced with an exact count (27, hand-verified per D-21 — see "Requirement Coverage Gap
   (D-18/D-19)" above), the unmapped clusters/commits are named, and a **Phase 112 "Security +
   Residual Sync"** roadmap amendment is proposed in writing with an explicit operator-approval
   gate before Phase 109 planning begins. `.planning/ROADMAP.md` is unedited by this ledger —
   confirmed by `git diff --stat -- .planning/ROADMAP.md` showing zero changes throughout Plan
   108-05's execution.

**Ledger closed.** All 5 plans in Phase 108 (108-01 through 108-05) have now contributed to this
document; no further plan is expected to append to `108-DIVERGENCE-LEDGER.md` — **except the one
deliberate, locked exception recorded immediately below** (Phase 111 CONTEXT.md D-02).

---

## Phase 111 Standing Divergence Addendum

Despite the "Ledger closed" declaration above, CONTEXT.md's Phase 111 D-02 directs one explicit
exception: record the CORE cluster's `adapt`-dispositioned resource-limiting commits as a
**standing divergence**, so the next upstream sync auditor who encounters them finds a documented
decision instead of re-litigating this one from scratch.

**Commits:** `e6d26871f0498e7dc7a867e67af5c6136b84f91c` (#1269, "feat: resource limiting") and its
sibling `34c2c975d649844923cf1515be94de624c689c6c` (#1403, "feat(resources): cap sandbox process
count with --max-processes") — both already listed above in "CORE Cluster — Per-Commit Table"
with disposition `adapt`, and both flagged there under "Threat Flags" as
`threat_flag: cross-crate-reexport` against `crates/nono/src/lib.rs`.

**Disposition: STANDING DIVERGENCE.** The fork **ADAPTS** (does not **ADOPT**) upstream's
core-module relocation, per `proj/ADR-111-resource-limits-boundary.md`'s Decision:
`crates/nono/src/lib.rs` will never gain `pub mod resource;` or `pub use
resource::ResourceLimits;`. All resource-limit code — flag parsing, per-platform dispatch, and
enforcement (Job Object on Windows, cgroup v2 on Linux, `RLIMIT_AS`+`RLIMIT_NPROC` on macOS) —
stays `nono-cli`-side, unchanged from its pre-Phase-111 shape. Only flag names/help text/semantics
are aligned with upstream where they already match (no rename was needed — see ADR-111 D-04).
Every future upstream sync touching the `resource` module in `nolabs-ai/nono`'s core `nono` crate
must **re-affirm this decision by citing `proj/ADR-111-resource-limits-boundary.md`**, rather than
treat the module's absence from the fork's `crates/nono/src/lib.rs` as an unabsorbed gap to close.

**Distinguished from the `tool-sandbox-surface` cluster's `DEFERRED->v3.7` treatment
(above, "Cluster Summary" table):** that cluster's 20 commits are a **deferral** — explicitly
routed to a **named future phase** (v3.7 Windows Tool-Sandbox Parity) that is expected to pick the
work back up, adopt or formalize it, and close the gap. This CORE-cluster divergence is different
in kind: it is a **permanent** decision with **no future phase this hands off to**. There is no
v3.7-style "come back and finish the absorb" expectation attached to `e6d26871`/`34c2c975` — the
fork's own implementation already exceeds upstream's in enforcement completeness, and the ADR's
Decision is final unless a future ADR explicitly reopens it. A future sync auditor should treat
this row the way they would treat Decision 3 of `proj/ADR-86-library-boundary-convergence.md` (the
Windows denial-path carve-out): a durable, standing architectural choice, not an open TODO.

---

## Phase 112 Security + Residual Sync Addendum

Despite the "Ledger closed" declaration above (and despite the Phase 111 Standing Divergence
Addendum immediately preceding this one, which is itself an explicit exception to that
declaration), Phase 112 CONTEXT.md's D-05 directs a second, likewise-locked exception: record all
18 `security-residual-and-misc` cluster dispositions as they actually **landed** — not as
`112-DISPOSITION-TABLE.md`'s pre-execution plan text framed them — so a future upstream-sync
auditor inherits the executed reality, not the plan.

**Scope:** the 18-commit `security-residual-and-misc` cluster listed above (10 named D-18 anchors
+ `f6f02751` (SEC-09) + 4 RES-01 + 3 RES-02 = 18, per D-08's closed arithmetic — every row below
appears exactly once; no invented remainder bucket).

### 18-SHA Final Disposition (as executed, Phase 112 close-out 2026-08-05)

| SHA (short) | Req | Table's planned disposition (`112-DISPOSITION-TABLE.md`) | **Disposition as shipped** | Evidence |
|---|---|---|---|---|
| `0ecc476b` | SEC-01 | won't-sync, HIGH confidence | **won't-sync (target subsystem absent) — CONFIRMED** | `112-AWS-SIGV4-PROXY-AUTH-FINDING.md`; `aws/`/`tls_intercept/` absent from `crates/nono-proxy/src/`; fork's own prior D-15 501-stub guard (`reverse.rs:260-268`) unchanged |
| `9b692e07` | SEC-02a | deferred -> Phase 114 (ROADMAP Amendment framing) | **ADAPTED-with-scope-limit — see "SEC-02 Carry-Forward Note (Phase 114, D-10)"** | `proj/ADR-114-oauth-capture-disposition.md`; fork-native `relay_response_with_capture()` enforcement point built on `reverse.rs` (all three relay sites), superseding the `forward.rs`-hook absence `112-OAUTH-CAPTURE-DISPOSITION.md` §1c identified; scope limit: capture works only for OAuth token endpoints reachable as configured reverse-proxy routes |
| `3c59c62e` | SEC-02b | deferred -> Phase 114 | **ADAPTED-with-scope-limit — see "SEC-02 Carry-Forward Note (Phase 114, D-10)"** | rides SEC-02a's disposition; hardening intent (fail-closed on unsafe token responses, cross-path denial) realized via `reject_unrewritten_token_fields()` (D-07 backstop) and the D-06 cross-path fail-closed guard (`114-07-SUMMARY.md`) |
| `d033c631` | SEC-02c | deferred -> Phase 114 | **ADAPTED-with-scope-limit — see "SEC-02 Carry-Forward Note (Phase 114, D-10)"** | rides SEC-02a's disposition; test-fixture intent realized via this phase's own fork-original behavior tests (Plans 114-05/06/07), not a ported fixture |
| `a3243907` | SEC-03 | **adopt, HIGH confidence** | **ADAPT — DIVERGED FROM PLAN** | `112-02-SUMMARY.md`: symbol-level forensics (not the table's file-presence check) proved `a3243907` depends on the unabsorbed antecedent `fa21a004`/`8a4237f2` (#1283, "introduce SeccompPolicy struct and client-driven selection", 21 files) — zero grep hits for `SeccompPolicy`/`LinuxSandboxPolicy`/`apply_landlock`/`apply_auto`/`apply_external`/`TcpNetworkEnforcement` anywhere in the pre-absorb fork tree. Implemented the security-hardening intent (procfs read-only mediation + fatal seccomp-notify failures + new `apply_seccomp`/`apply_seccomp_with_abi`/`apply_external` API) against the fork's existing simpler `apply()`/`apply_with_abi()` architecture. `fa21a004`/`8a4237f2`'s 21-file `LinuxSandboxPolicy` CLI-policy-selection feature remains a **standing, unabsorbed gap** after Phase 112 closes — already independently flagged above ("tool-sandbox-surface cluster commits (20)" table) as a CORE-cluster/Phase-111 residual, never picked up by Phase 111; not this plan's to close. |
| `f943fb5a` | SEC-04 | adopt, HIGH confidence | **adopt — CONFIRMED** | `112-03-SUMMARY.md`: ported verbatim, additive `predicate`/`TRUST_POLICY_PREDICATE` discriminator, `TRUST_POLICY_VERSION` deprecated not removed |
| `d84b4818` | SEC-05 | adopt, HIGH confidence | **adopt — CONFIRMED** | `112-05-SUMMARY.md`: symbol-level re-verification confirmed every referenced symbol present with matching signatures; `restrict_execute()` gains bare `Refer` on `/`, gated `abi.has_refer()`; live-tested via `cross test` |
| `ac5ccd70` | SEC-06 | adapt | **adapt — CONFIRMED** | `112-06-SUMMARY.md`: upstream's `tool_sandbox_runtime.is_some() \|\| seccomp_policy.child_requires_dumpable()` gate symbols absent; reused the fork's existing `linux_child_requires_dumpable()` predicate; `PR_SET_CHILD_SUBREAPER` + `reap_reparented_orphans()` ported, RED/GREEN TDD-verified live via `cross test` |
| `2663e990` | SEC-07 | adopt | **adopt (adapted API) — CONFIRMED, plus additional Rule-2 hardening** | `112-07-SUMMARY.md`: `nono proxy` standalone subcommand built on the fork's real `ProxyLaunchOptions`/`build_proxy_config_from_flags` API (upstream's Intent-struct imports absent); see "New Information" below for the `require_auth`/`strict_connect_auth` fix this plan additionally delivered |
| `a5a441c2` | SEC-08 | won't-sync-verbatim / adapt-with-fail-closed-preservation, ADR-112-flagged | **won't-sync/PRESERVE — CONFIRMED, ADR written** | `112-03-SUMMARY.md`; `proj/ADR-112-allow-vars-fail-closed-preserved.md`: upstream's `Option<Vec<String>>` reinterpretation would flip an omitted `allow_vars` from "strip all" (fork's fail-closed default, D-20/Plan 34-08a) to "allow everything" — rejected outright, zero source-code change (`git diff -- crates/nono-cli/src/profile_runtime.rs` empty) |
| `f6f02751` | SEC-09 | won't-sync (target subsystem absent), LOCKED per D-01 | **won't-sync + carry-forward filed — CONFIRMED** | premise re-verified live (`grep` returns 0 hits for `command_policies`/`tool_sandbox_runtime`/`tool_sandbox_initial_shim` in `execution_runtime.rs`); carry-forward note filed above in this ledger's tool-sandbox-surface cluster section ("SEC-09 Carry-Forward Note") |
| `f0506434` | RES-01 | skip, with recorded reasoning | **skip — CONFIRMED** | `112-OAUTH-CAPTURE-DISPOSITION.md` Part 2; HKLM machine-policy-spine collision; flagged-but-out-of-scope `always-further/*` pack-registry-namespace UX staleness (16 grep hits) noted, not acted on |
| `7fe0c828` | RES-01 | skip, with recorded reasoning | **skip — CONFIRMED** | same HKLM-spine collision rationale; `X-Nono-Pull-Reason` telemetry header, no fork-side gap |
| `0158d52f` | RES-01 | skip, with recorded reasoning | **skip — CONFIRMED** | same rationale; installation-context headers, upstream-service-facing only |
| `762eb05b` | RES-01 | skip, with recorded reasoning | **skip — CONFIRMED** | same rationale, plus `release.yml`'s heavy Trusted-Signing (v3.5) customization makes a verbatim upstream hunk out-of-proportion regression risk |
| `503045801a` | RES-02 | adopt | **adopt — CONFIRMED** | `112-04-SUMMARY.md`: `CprReplyParse`/`discard_late_terminal_input()` ported verbatim, wired only into the final `release_terminal_for_prompt` teardown path; 38/38 `pty_proxy` tests pass live via `cross test` |
| `4cc0af2c52` | RES-02 | "adapt with caution, or skip", MEDIUM confidence | **SKIP — DIVERGED FROM PLAN (narrowed, not reversed)** | `112-04-SUMMARY.md`: symbol-level re-verification found the target helper `socket_test_dir()` absent from `crates/nono/src/supervisor/socket.rs` AND from `open_url_runtime.rs`; the commit's third file (`tests/url_open_integration.rs`) does not exist in the fork at all. The fork's `socket.rs` tests use `SupervisorSocket::pair()` (anonymous `socketpair()`, no filesystem path) — no SUN_LEN-limited tempdir choice remains to switch. Recorded via a comment-only disposition-amendment note (`git diff --stat` confirms zero functional change). |
| `9840a16f35` | RES-02 | adopt, HIGH confidence | **adopt (adapted, not verbatim) — CONFIRMED, disposition narrowed** | `112-04-SUMMARY.md`: `NONO_NO_SAVE_PROMPT=1` ported as-is; the denial-marker assertion was rebuilt around the bare socket path rather than upstream's literal `"send {path}"` string, because this fork's `diagnostic/formatter.rs` never emits that marker (it renders a block header + one denied-path line instead) — a verbatim port would have made the tightened assertion vacuously pass |

**Arithmetic check (D-08):** 10 named anchors (`0ecc476b`, `9b692e07`, `3c59c62e`, `d033c631`,
`a3243907`, `f943fb5a`, `d84b4818`, `ac5ccd70`, `2663e990`, `a5a441c2`) + 1 SEC-09 (`f6f02751`) +
4 RES-01 (`f0506434`, `7fe0c828`, `0158d52f`, `762eb05b`) + 3 RES-02 (`503045801a`, `4cc0af2c52`,
`9840a16f35`) = **18**, matching this cluster's itemization above exactly. Every SHA appears in
the table exactly once; no remainder bucket.

### Divergences between the planned disposition table and what shipped

`112-DISPOSITION-TABLE.md` (Wave 1's finalized-but-pre-execution disposition table) called 3 of
the 18 rows correctly at the ADOPT/SKIP level but incorrectly at the ADAPT-vs-confidence level.
Recorded here because the corrected reasoning — not the table's original text — is what a future
auditor should trust:

1. **SEC-03 (`a3243907`) — table said "adopt, HIGH confidence"; shipped as ADAPT.** The table's
   confidence was derived from file-presence evidence (`ls`/`Read` confirming every target file
   exists) — not symbol-level verification of whether the specific types/functions the diff's
   context lines assume already exist. `112-02`'s Task 1 `read_first` step found zero grep hits
   for `SeccompPolicy`/`LinuxSandboxPolicy`/`apply_landlock`/`apply_auto`/`apply_external`
   anywhere in the fork, tracing the gap to the unabsorbed antecedent commits `fa21a004`/
   `8a4237f2` (#1283) — already independently flagged in this ledger's tool-sandbox-surface
   cluster table (above) as a "CORE-cluster/Phase-111 residual," never picked up by Phase 111.
   This 21-file refactor remains a **standing, unabsorbed gap** after Phase 112 closes — flagged
   for whichever future plan eventually absorbs it.
2. **RES-02/`4cc0af2c52` — table said "adapt with caution, or skip" (MEDIUM confidence); shipped
   as SKIP.** `112-04`'s symbol-level re-verification found the target helper `socket_test_dir()`
   does not exist anywhere in the fork, and the commit's third file does not exist at all —
   narrower than "risky to adapt," the target is simply absent.
3. **RES-02/`9840a16f35` — table said "adopt, HIGH confidence" (implying a near-verbatim port);
   shipped as adopt-but-adapted.** Upstream's `"send {path}"` denial marker string is never
   emitted by this fork's diagnostic formatter (`crates/nono-cli/src/diagnostic/formatter.rs`
   renders a block-header-plus-per-line format instead); a verbatim port would have made the
   tightened test assertion vacuously pass. `112-04` rebuilt the assertion around the bare socket
   path instead.

**Methodological lesson (recorded for future UPST-sync planning):**
`112-DISPOSITION-TABLE.md`'s confidence column was derived from file-presence evidence (`ls`/
`git show --stat`), not symbol-level verification (`grep` for the specific types/functions/fields
the diff's own context lines assume already exist) — and was wrong in 2 of the 3 dispositions this
addendum re-checked at symbol level during execution (SEC-03, RES-02/`4cc0af2c52`; the third,
RES-02/`9840a16f35`, was directionally correct but needed a fork-specific adaptation the table
didn't anticipate). Future UPST-sync Wave-1 reality-check passes should require symbol-level
verification (grep for the diff's referenced types/functions/struct-fields, not just `ls`/
file-presence) before assigning a HIGH-confidence "adopt" call — `112-02`, `112-04`, `112-05`, and
`112-06` each independently re-ran this check this phase (`112-05`'s SUMMARY documents running the
same check and confirming the table's call WAS correct for SEC-05, in contrast to SEC-03/
RES-02-`4cc0af2c52` where the same check found a real gap).

### New information generated during Phase 112 execution (not upstream-sync items)

**Pre-existing fork security defect closed by SEC-07 (`112-07`, not an upstream sync item):**
before this plan, `nono-proxy` had no `--no-auth` / auth-disable concept at all. `reverse.rs`'s two
auth checks were unconditionally fatal (correct only for the sandboxed-run-only world), and
`connect.rs`'s CONNECT-tunnel auth check was unconditionally lenient (never rejecting, by design
for the OS-sandbox-is-the-trust-boundary sandboxed path) — a genuine session-token-boundary bypass
for a **standalone, non-loopback `nono proxy`** server with no OS sandbox behind it: an
unauthenticated client could tunnel arbitrary traffic through CONNECT regardless of the advertised
session token. Closed via `ProxyConfig.require_auth` (default `true`, matches pre-existing
sandboxed-run behavior) and `ProxyConfig.strict_connect_auth` (default `false`, i.e. every
standalone invocation except explicit `--no-auth` sets it `true` and CONNECT auth becomes fatal —
407). Both defaults preserve `nono run`/`shell`/`wrap`'s behavior byte-for-byte (confirmed:
218/218 pre-existing `nono-sandbox-proxy` unit tests pass unchanged). See `112-07-SUMMARY.md`'s
"Security Note" section for the full trace.

### Disposition class summary (this addendum)

- **won't-sync (target subsystem absent):** SEC-01, SEC-09 (2)
- **won't-sync/PRESERVE (ADR-112, zero code change):** SEC-08 (1)
- **adapt (ADAPTED-with-scope-limit, Phase 114):** SEC-02a/b/c (3)
- **adopt:** SEC-04, SEC-05, RES-02/`503045801a`, RES-02/`9840a16f35` (4)
- **adopt (adapted API/marker):** SEC-07 (1)
- **adapt:** SEC-03, SEC-06 (2)
- **skip, with recorded reasoning:** RES-01 x4, RES-02/`4cc0af2c52` (5)

2 + 1 + 3 + 4 + 1 + 2 + 5 = **18**, matching the 18-SHA table above and D-08's closed arithmetic.

**Distinguished from the Phase 111 Standing Divergence Addendum above:** that addendum records ONE
permanent architectural decision (CORE-cluster resource-limiting stays `nono-cli`-side, no future
phase hands off to it). This addendum instead closes out an entire phase's disposition work across
18 independently-adjudicated commits — three disposition classes of which (SEC-01/SEC-09
won't-sync, SEC-08 PRESERVE) are themselves durable/standing in the same sense as the Phase 111
entry, while the remaining 13 (SEC-03..07, RES-01 x4, RES-02 x3) represent completed, code-landed
absorb/adapt/skip work with no further action owed. SEC-09's entry additionally carries a live
future obligation — see "SEC-09 Carry-Forward Note" in the tool-sandbox-surface cluster section
above (not repeated here). **SEC-02a/b/c, originally deferred here pending Phase 114's verdict, is
now itself resolved (ADAPTED-with-scope-limit) and closed out in the "SEC-02 Carry-Forward Note
(Phase 114, D-10)" below** — the deferral this addendum recorded in 2026-08-05 is no longer open.
