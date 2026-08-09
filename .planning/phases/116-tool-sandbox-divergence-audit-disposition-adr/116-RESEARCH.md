# Phase 116: Tool-Sandbox Divergence Audit + Disposition ADR - Research

**Researched:** 2026-08-09
**Domain:** Git archaeology / upstream-fork divergence audit (docs-only phase) + Windows OS-confinement capability inventory (informational, not judgment)
**Confidence:** HIGH (all git commands live-executed on this host against the real `upstream` remote; all symbol inventories are live greps, not recollection)

## Summary

This phase produces two markdown documents and zero code. The hard part is not writing prose —
it is (a) getting the ledger's *structure* right so it matches the house shape from Phases
108/98/94/85 and the ADR shape from ADR-113/111/114/108, and (b) making every git command in the
plan's task list something that has actually been run on this Windows host against this repo's
real `upstream` remote, so the plan's task descriptions can cite exact, reproducible commands
rather than approximated ones.

Both are now done. Section 1 below distills the house ledger/ADR format into a concrete template
(section order, table shapes, the pinned-SHA Reproduction block, the D-07 residue-accounting
convention, the D-04/addendum "ledger closed except for named exceptions" pattern). Section 2
reports every git command the plan will need, live-run on this host, with exact output — including
a **reproduction of Phase 108's own D-06 substring-glob hazard**, caught live in this window too
(two commits, `5a7447d3` and `ebd51cbb`, whose only "tool-sandbox" hit is
`docs/cli/features/tool-sandbox.mdx`). Section 3 inventories what the fork's Windows confinement
stack exposes as greppable symbols — the raw material D-07's feasibility matrix will need, with
one genuine methodology finding: `exec_strategy_windows/supervisor.rs`'s public surface is
entirely `pub(super)`, invisible to a `pub|pub(crate)` grep pattern. Section 4 inventories
upstream's `tool-sandbox/` at `v0.71.0` by file and by mod/use edge, with one structural finding:
the `mod`/`use` edges for the D-03 candidate modules live in `crates/nono-cli/src/main.rs`, not
inside `tool-sandbox/mod.rs` or `command_policy.rs` themselves — so "follow mod/use edges out of
tool-sandbox/mod.rs" (as CONTEXT.md phrases it) requires also reading `main.rs`'s `mod`/`use`
block, not just the subsystem's own files. Section 5 adapts Validation Architecture to a
docs-only, git-archaeology deliverable.

**Primary recommendation:** Build the plan's task list directly around the Reproduction block
this research already ran (Section 2) — every command below is copy-paste-ready and was verified
against the pinned SHAs `0551eba2` (v0.64.1) / `0055bf3c` (v0.71.0). Do not re-derive the commands
from scratch; re-run these exact ones at execution time and diff the counts against what's
recorded here (per D-01/D-04's re-measurement discipline, a different count at execution time is
itself a finding, not an error).

## Architectural Responsibility Map

This phase produces no runtime code, so a tier-ownership map in the usual sense does not apply.
The closest analog — which *tier* each deliverable's claims are about — is still useful for the
planner sanity-checking task scope:

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Per-commit divergence ledger (TSBX-01) | Documentation / git-archaeology | — | Pure historical record; no runtime tier |
| ADR verdict + feasibility matrix (TSBX-02) | Documentation / decision record | — | Evaluates two *future* implementation poles without building either |
| Pole A's hypothetical `platform/windows.rs` driver (evaluated, not built) | OS / Kernel (AppContainer, Low-IL, WFP) | CLI (`nono-cli` process dispatch) | Matches upstream's `platform/linux.rs`/`platform/macos.rs` tier — a CLI-owned per-tool launch mediator sitting on top of OS primitives, per CLAUDE.md's Library-vs-CLI boundary table |
| Pole B's fork-native PreToolUse hook + broker (evaluated, not extended) | CLI (`hooks.rs`, `claude_code_hook.rs`) + OS (Low-IL primary-token broker, AppContainer, WFP) | Claude Code's own hook contract (external, not fork-owned) | Entry point is an external agent-engine contract (PreToolUse), which is D-09's named "engine-agnosticism" weak spot |

No code touches the library (`crates/nono/src/`) in this phase; both poles under evaluation would,
if executed by Phase 120, live entirely in `nono-cli` per ADR-86's policy-free-library boundary
(confirmed unaffected by this phase — see Section 3).

## User Constraints (from CONTEXT.md)

<user_constraints>

### Locked Decisions

All 20 decisions (D-01..D-20) from `116-CONTEXT.md` are locked and binding on the plan. The full
text is long (see the file directly); the load-bearing ones for planning are:

- **D-01:** Window is `v0.64.1..v0.71.0` (NOT Phase 108's fenced `v0.66.0..v0.69.0`). Tip pinned
  at tag `v0.71.0` (`0055bf3c`), not `upstream/main`. `upstream/main` was at `4ede9ccc`
  (2026-08-05) — recorded as a named boundary. The D-01 measured-hypothesis table (38 surface
  commits / 12 files / 18,433 LOC / etc.) is explicitly "a hypothesis to re-confirm at audit time,
  never copy forward" — see Section 2 below for the live re-confirmation this research already ran.
- **D-02:** Every commit in the window (all 38, including the 11 post-fence ones) gets a full
  disposition (adopt/adapt/skip/split) with `windows-touch`. No `beyond-ceiling` flag.
- **D-03:** The module set is **re-derived** from `mod`/`use` edges at `v0.71.0`, then diffed
  against D-06's inherited three (`tool-sandbox/`, `command_policy.rs`, `lineage_cgroup.rs`).
  Explicit in-or-out call required on 4 named candidates: `command_blocking_deprecation.rs`,
  `instruction_deny.rs`, `open_url_runtime.rs`, `hook_runtime.rs`.
- **D-04:** Residue per split commit bucketed `absorb`/`defer`/`noise` (Phase 108 D-07 pattern).
  Fenced-window residue verified by grep against fork HEAD, not assumed from 108's routing note.
  Post-fence residue mapping to no phase is a named finding with a proposed successor.
- **D-05/D-06:** ADR is a two-pole comparison (Pole A: adopt + build `platform/windows.rs` from
  scratch; Pole B: formalize fork-native PR#4). Two intermediate shapes (adopt-Unix-only,
  adopt-policy-model-only) recorded as considered-and-rejected, one paragraph each.
- **D-07/D-08:** Capability feasibility matrix rates each upstream driver capability
  implementable-on-fork-primitives / implementable-but-new-work / structurally-blocked, citing the
  fork symbol or the OS/ADR-65 reason. Minifilter-dependent rows are `structurally-blocked` citing
  ADR-65, never framed as available-if-ADR-65-reversed.
- **D-09:** One fixed criteria set applied identically to both poles: per-command granularity,
  engine-agnosticism, enforcement depth, fail-direction under layer failure, platform coverage,
  ADR-86 boundary impact, ongoing divergence/maintenance cost. No weighting declared up front.
- **D-10:** **The verdict is genuinely open.** No leaning locked. Plans must not be written toward
  either pole — build the ledger, matrix, and scoring, let the verdict fall out.
- **D-11:** Executor lands `Status: Accepted`. Phase 120 planning is operator-gated on ratification.
- **D-12:** SC3 satisfied by named falsifiable triggers + a named re-test checkpoint (next UPST
  sync), not a reasoning paragraph.
- **D-13/D-14:** New `116-DIVERGENCE-LEDGER.md` (108's ledger not edited, cross-referenced only).
  ADR carries a "Phase 120 Scope" section + a proposed (not applied) `ROADMAP.md` amendment.
- **D-15:** Phase 120 sizing derives row-by-row from the feasibility matrix, coarse S/M/L bands.
  Commit counts are explicitly NOT the sizing basis.
- **D-16:** Every confidence rating carries inline evidence — literal command, hit count, date
  measured. File presence is never evidence (Phase 112's 3-of-6 wrong-disposition lesson). Greps
  must discover their targets, not confirm pre-named ones (Phase 115 V-01 lesson).
- **D-17 (carried forward):** Ledger house shape — pinned-SHA Reproduction block (SHAs never tag
  names), cluster-first grouping with per-commit rows where dispositions bite, `windows-touch`
  flag, hypothesis-vs-measurement disagreements reconciled in place with the superseded figure
  retained.
- **D-18 (carried forward):** Fork invariants (Windows security model, ADR-86 boundary) constrain
  every disposition. Any pole implying cfg-gated Unix edits flags cross-target clippy for the
  *executing* phase (Phase 120), not this one.
- **D-19 (carried forward):** Carve-out re-touch check runs: `git log <window> -- <exact path>`
  per fork carve-out, zero-hit recorded explicitly as "clean — no re-touch in window."
- **D-20:** No code changes. Zero `.rs`/`Cargo.toml` edits. `ROADMAP.md` untouched. `make ci` is
  NOT a gate for this phase.

### Claude's Discretion

- Cluster naming/count within the D-17 scheme; ledger table layout/column ordering, provided
  `windows-touch`, disposition, per-commit residue (D-04), and the D-16 evidence column are
  present.
- Whether to carry Phase 108's `security-relevant` flag forward as a second column (leaned yes,
  not required).
- The ADR's internal structure, provided both poles are scored on the D-09 set, D-06 rejected
  shapes are named, and D-12 triggers + D-14/D-15 sizing are present.
- Plan/wave breakdown; whether the feasibility matrix lives in the ADR or a ledger appendix.

### Deferred Ideas (OUT OF SCOPE)

- Executing the verdict → Phase 120 (TSBX-03), operator-gated on D-11 ratification.
- Upstream sync past `v0.71.0` → FUT-08/UPST13.
- Post-fence non-module residue mapping to no phase (D-04) — surfaced as a finding with a proposed
  successor only.
- The fork's engine-agnosticism gap — scored as a D-09 criterion, not remediated here.
- Ledger granularity gray area (per-commit for all ~38 vs. cluster-first); whether the docs-only
  `command_policies` profile surface needs its own row; carve-out re-touch check behavior when the
  fork has no host module for the carve-out.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TSBX-01 | Per-commit divergence ledger covering PR #1105 + the 7 fenced refinement PRs, `windows-touch` flags, per-cluster dispositions, Phase 108/98/94/85 ledger shape | Section 1 (house format), Section 2 (all Reproduction-block commands live-verified against pinned SHAs, including the D-06-style substring-glob hazard re-demonstrated in this window) |
| TSBX-02 | ADR recording adopt-vs-formalize verdict, fork's PR#4 evaluated as genuine alternative, every confidence rating symbol-level-grounded | Section 3 (fork symbol inventory — greppable public surface of `exec_strategy_windows/`, hook/broker files), Section 4 (upstream `tool-sandbox/` file/symbol inventory at `v0.71.0`), spike-findings-nono skill content (engine-agnosticism criterion grounding) |

</phase_requirements>

## Standard Stack

Not applicable in the conventional sense — this phase installs no packages and writes no code.
The "stack" is entirely the git CLI already present on this host (`git version` confirmed via the
commands below) plus the existing project convention of hand-written markdown ledgers/ADRs. No
package legitimacy audit is needed (no external packages are installed by this phase).

## Package Legitimacy Audit

**Not applicable.** This phase installs zero packages (D-20: no `Cargo.toml` edits). Skipped per
the package-legitimacy-protocol's own scope (only required "whenever this phase installs external
packages").

## Section 1: House Format — Ledger + ADR Template

### 1a. Ledger section order (from `108-DIVERGENCE-LEDGER.md`, the authoritative template)

The 108 ledger's actual top-level section order, sampled structurally (not read end-to-end):

```
---
(YAML frontmatter: phase, plan, ledger_type, range, upstream_head_at_audit, refetch_date,
 fork_baseline, total_unique_commits, date)
---

## Headline                                    <- bucket/cluster summary table + arithmetic check
## Requirement Coverage Gap (D-18/D-19)         <- 116 analog: N/A unless 116 finds an unmapped set
### Proposed Phase NNN: <name>                  <- only if a gap needing a new phase is found
## Reproduction                                 <- pinned SHAs, live command output, re-run guard
### <Subsystem> Surface — D-06-style Re-measurement   <- if a hypothesis disagrees with live count
## Full Commit Accounting (N non-merge commits)
### Bucket Classification Edge Cases
### Discrepancy vs. CONTEXT.md Hypothesis       <- 116 MUST have this: D-01's table is explicit hypothesis
### Prefix-vs-Path-Set Cross-Check
### <BUCKET> bucket (N commits)                 <- repeated per coarse bucket
## Cluster Summary                              <- thematic clusters (NET/PROF/CORE/tool-sandbox/...)
### Cluster Summary Rollup Support Notes
### <cluster> commits (N)                       <- repeated per cluster
### <Carry-Forward Note> (Phase NNN, D-NN)       <- forward pointer to where residue landed, if known
## Downstream Notes (non-normative)
## <CLUSTER> — Per-Commit Table (Plan NN-0N)     <- repeated per will-sync/split cluster
## Threat Flags
## <subsystem>-pure and <subsystem>-split        <- 116's tool-sandbox-pure/split section, this ledger's core
### Pure/entangled count reconciliation          <- reconcile any hypothesis (e.g. D-05's estimate) vs. measured
## <subsystem>-pure (N commits)
## <subsystem>-split (N commits — full D-07 residue accounting)
  #### <sha> — #NNNN <subject> (N paths)
  **Finding:** <split verdict rationale>
  | path | marker | note |                      <- absorb / defer / noise per D-04/D-07
### <subsystem>-split summary
## <other clusters: DEPS, CI, DOCS> (Plan NN-0N)
## Bucket-Count Reconciliation
## Carve-out Re-touch Check (D-19)
### N. <carve-out name> — <exact path(s)>
  ```bash
  git log --no-merges --oneline $RANGE -- <exact path>
  ```
  <literal output, or "(no output)">
  **HIT (N commits)** — routed to existing table | **clean — no re-touch in window.**
### Carve-out re-touch summary
## Security-Relevant Rollup (D-20-equivalent — optional per Claude's Discretion)
## Completeness Verification
### ROADMAP.md Phase NNN Success Criteria — final disposition
## <Phase NNN Standing Divergence Addendum>      <- only added post-execution, by a LATER phase citing this one
```

**116-specific adaptations required by CONTEXT.md:**
- No `## Requirement Coverage Gap` section is expected to be load-bearing the way 108's was —
  TSBX-01/TSBX-02 are the only two requirements this ledger maps to, and both map to the whole
  document, not a subset of commits. If the D-03 re-derivation or D-04 post-fence residue check
  turns up genuinely unmapped work, name it as a finding (per D-04) rather than force-fitting the
  108-style "Requirement Coverage Gap" section — the shape doesn't transfer 1:1.
- The `Discrepancy vs. CONTEXT.md Hypothesis` section is NOT optional for 116 — D-01's table is
  explicitly flagged as a hypothesis, and 108's own ledger required this exact section when its
  own measurements disagreed. Section 2 below already found agreement on every number tested; the
  plan should still include the section, stating "confirmed, no discrepancy" if that holds at
  execution time, or the actual delta if it does not.
- 108's ledger cross-references — not duplicates — the 20-commit fenced-window surface. The
  planner should structure tasks so the 116 ledger's tables for the 9 pure + 11 split fenced-window
  commits are genuinely short cross-reference rows pointing at `108-DIVERGENCE-LEDGER.md`'s
  existing tables (per D-13), with full new-table treatment reserved for the 7 pre-fence + 11
  post-fence commits that 108 never touched.

### 1b. ADR section order (from ADR-113/111/114/108, cross-checked)

All four sampled ADRs share this skeleton:

```
# ADR-NNN: <Title> — <Verdict-class subtitle>

**Status:** Accepted
**Phase:** NNN — <phase name>
**Date:** <date>
**Authors:** Phase NNN execution

---

## Context
  <upstream commit(s)/PR(s), file/line diff stats, live re-measurement of any stale prior claim>

## Decision
  <the verdict, stated in the first sentence, in bold>
  ### <sub-decision positive-proof section, e.g. ADR-113's D-01>
  <symbol-level evidence: every code path enumerated with grep/test proof, not assertion>

## D-0N: <Named Concern> (dependency review / boundary re-check / residual-honesty section)
  <ADR-113 has "D-05: Dependency Review", "D-08/SC3: Re-confirming ADR-86", "D-07: Loud-Skip Reality">
  <ADR-111 has "Why the ADR-86 carve-out does NOT extend here">
  <ADR-114 has "D-09/SC3: Re-confirming the ADR-86 Boundary">

## OD-N: <Permanent Scope Boundary>   <- "Out-of-scope Decision", the Pole-B / considered-rejected shape
  <named, permanent, citable by future absorbs — this is the shape D-06's two rejected
   intermediate poles and (if the verdict is Pole B) the whole ADR's outcome should take>

## Consequences
  <numbered list: what other documents get updated (ledger addendum), what's permanent vs.
   deferred, what future syncs must re-affirm>

## References
  <bulleted: upstream commit SHAs, sibling ADRs, ledger sections, phase CONTEXT.md, phase
   RESEARCH.md, per-plan SUMMARY.md files>
```

**116-specific fit:** D-05's two-pole comparison + D-06's two rejected intermediates map cleanly
onto this skeleton — `## Decision` states the verdict; a `## D-0N: Feasibility Matrix` section
(D-07/D-08) plays the "named concern" role ADR-113's D-05/D-07/D-08 play; a `## Pole [A|B]:
Considered and Rejected — <the other 2 shapes>` section plays the OD-N role (naming Unix-only-adopt
and policy-model-only-adopt as permanently-considered, not scored). D-09's symmetric scoring table
is new relative to the four precedent ADRs — none of them do head-to-head criteria scoring because
none of them had a live, already-built fork-native alternative to weigh against upstream's code.
This is a genuine template extension, not a deviation — CONTEXT.md's Claude's-Discretion note ("The
ADR's internal structure, provided both poles are scored on the D-09 criteria set...") already
anticipates this.

### 1c. The `## <Phase N> Carry-Forward Note` and `## Standing Divergence Addendum` pattern

Two distinct shapes exist in 108's ledger, both worth the planner distinguishing when writing
116's own forward-pointing sections:

1. **Carry-forward note** (e.g. "SEC-02 Carry-Forward Note (Phase 114, D-10)") — written by 108
   itself, pointing at a *future* phase that is expected to resolve an open item. This is the shape
   for 116's own D-04 post-fence-residue-with-no-phase finding.
2. **Standing Divergence Addendum** (e.g. "Phase 111 Standing Divergence Addendum") — written
   *after the fact*, by the LATER phase, appended to the EARLIER phase's already-"closed" ledger,
   recording a PERMANENT decision (not a deferral). This is the shape a hypothetical future Phase
   120 (or a future UPST13 audit) would use to append to `116-DIVERGENCE-LEDGER.md` once the
   verdict is executed — 116 itself does not write this section (it doesn't exist yet), but the
   plan should leave the ledger's closing language open to this pattern (108's own closing line —
   "Ledger closed" — is explicitly not literally true; it's "closed except for named exceptions,"
   and 116 should phrase its own closing statement the same way to leave room for a future
   addendum).

## Section 2: Tooling Reproducibility — Live-Verified Commands

All commands below were executed live on this Windows host (`git-bash`) on 2026-08-09 against the
real `nolabs-ai/nono` `upstream` remote, from `C:\Users\OMack\nono`, on branch
`milestone/v2.13-carryforward-closeout`, clean working tree. **These are the exact strings to put
in the plan's task descriptions** — no translation needed at execution time.

### 2a. Remote and tag resolution

```bash
git remote -v
# origin           https://github.com/OscarMackJr/nono.git (fetch/push)
# upstream         https://github.com/nolabs-ai/nono.git (fetch/push)
# upstream-legacy  https://github.com/always-further/nono.git (fetch/push)

git ls-remote --tags upstream v0.64.1 v0.71.0
# 0551eba27ea53b0eda20f4f796f757dd168adb92  refs/tags/v0.64.1
# 0055bf3c686de6fa665decd9831dc5686b282cb0  refs/tags/v0.71.0
```

Both SHAs **match CONTEXT.md D-01's pinned values exactly**. `upstream` already points at
`nolabs-ai/nono` (no repointing needed, consistent with Phase 108's note).

### 2b. Window commit counts

```bash
RANGE="0551eba27ea53b0eda20f4f796f757dd168adb92..0055bf3c686de6fa665decd9831dc5686b282cb0"

git log --no-merges --oneline $RANGE | wc -l
# -> 187

git log --merges --oneline $RANGE | wc -l
# -> 4
```

**Note for the planner:** this is the FULL non-merge commit count across the ENTIRE window
(v0.64.1..v0.71.0) — not the tool-sandbox-scoped subset. CONTEXT.md's D-01 table does not state
this figure (it only states the tool-sandbox-scoped 38 and its pre/fenced/post-fence breakdown).
187 total non-merge commits is far larger than the 38-commit tool-sandbox surface, confirming
D-02's framing that the ledger's *dispositioned* set is the 38-commit tool-sandbox surface, not
the full window — the full window is `v0.66.0..v0.69.0`'s superset, already audited by Phase 108
for its non-tool-sandbox content. **This phase does not re-audit the 149 non-tool-sandbox commits**
(187 − 38 = 149) — TSBX-01/02 scope is the tool-sandbox subsystem only; re-confirm this reading in
the plan's own scope statement to avoid an executor accidentally re-running Phase 108's job.

### 2c. The 3-path union surface (D-06 inherited set, D-03's starting point)

```bash
git log --no-merges --oneline $RANGE -- \
  'crates/nono-cli/src/tool-sandbox/' \
  'crates/nono-cli/src/command_policy.rs' \
  'crates/nono-cli/src/lineage_cgroup.rs' | wc -l
# -> 38
```

**Matches D-01's hypothesis exactly (38).** This is the *inherited* set D-03 must re-derive from
first principles and diff against — do not treat this command's output as the re-derivation
itself; D-03 requires following `mod`/`use` edges (Section 4) to arrive at the module set
independently, then comparing to this number.

### 2d. Reproducing Phase 108's D-06 substring-glob hazard — CONFIRMED live in this window too

This is a genuine, reproducible finding, not a hypothetical warning. The durable project lesson
("`git log -- '*name*'` is substring-anywhere and has previously matched docs paths") is not
theoretical for this window — it recurs with the *same two commits* Phase 108 already flagged:

```bash
# THE HAZARD FORM — do not use this as the ledger's authoritative count:
git log --no-merges --oneline $RANGE -- '*tool-sandbox*' | wc -l
# -> 37

# THE CORRECT DIRECTORY-ONLY FORM:
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/tool-sandbox/' | wc -l
# -> 35

# Isolate exactly which commits the hazard form over-counts:
comm -23 <(git log --no-merges --format='%H' $RANGE -- '*tool-sandbox*' | sort) \
         <(git log --no-merges --format='%H' $RANGE -- 'crates/nono-cli/src/tool-sandbox/' | sort)
# -> 5a7447d3ed30835bd9bd647b7812ee18ea80a155
# -> ebd51cbb9546aec872136301249d048074a05e38

git show --name-only --format='' 5a7447d3ed30835bd9bd647b7812ee18ea80a155 | grep -i tool-sandbox
# -> docs/cli/features/tool-sandbox.mdx
git show --name-only --format='' ebd51cbb9546aec872136301249d048074a05e38 | grep -i tool-sandbox
# -> docs/cli/features/tool-sandbox.mdx
```

Both commits touch `docs/cli/features/tool-sandbox.mdx` and nothing else matching the substring —
these are precisely the same commits Phase 108 already named (`108-DIVERGENCE-LEDGER.md`'s D-06
finding cites `5a7447d3` and `ebd51cbb` as the docs-only false matches from that window's own
hazard reproduction). **Use the directory-only pathspec (`'crates/nono-cli/src/tool-sandbox/'`,
trailing slash, single-quoted) or the explicit 3-path union — never a `*substring*` glob — anywhere
in this phase's git log invocations.**

### 2e. Union arithmetic breakdown (confirms 38 = 35 + 3, matching D-01)

```bash
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/command_policy.rs' | wc -l
# -> 17
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/lineage_cgroup.rs' | wc -l
# -> 1

# Commits touching command_policy.rs or lineage_cgroup.rs but NOT the tool-sandbox/ directory:
comm -23 <(git log --no-merges --format='%H' $RANGE -- \
             'crates/nono-cli/src/command_policy.rs' 'crates/nono-cli/src/lineage_cgroup.rs' \
             | sort -u) \
         <(git log --no-merges --format='%H' $RANGE -- 'crates/nono-cli/src/tool-sandbox/' \
             | sort -u)
# -> 5a7447d3ed30835bd9bd647b7812ee18ea80a155
# -> c808f000db582ad69a10c1a2a1b8221b020fff94
# -> ebd51cbb9546aec872136301249d048074a05e38
```

35 (directory-only) + 3 (the above, unique to command_policy.rs/lineage_cgroup.rs) = **38**,
matching D-01's hypothesis exactly. Note `5a7447d3`/`ebd51cbb` appear in BOTH the substring-hazard
list (2e) and this command_policy.rs list — they are genuinely `command_policy.rs`-touching
commits (not docs-only in the union sense), which is exactly Phase 108's original finding about
these two SHAs reproduced verbatim in this larger window.

### 2f. Reading file content at a tag without checkout (no working-tree pollution)

```bash
git show v0.71.0:crates/nono-cli/src/tool-sandbox/mod.rs
git show v0.71.0:crates/nono-cli/src/command_policy.rs
git show v0.71.0:crates/nono-cli/src/lineage_cgroup.rs
```

Confirmed working — `git show <ref>:<path>` prints file content to stdout without touching the
working tree or index. This is the form to use for all "read upstream source at v0.71.0" tasks.
Combine with `| wc -l` for LOC counts and `| grep -n <pattern>` for symbol search.

### 2g. Listing a directory tree at a tag without checkout

```bash
git ls-tree -r --name-only v0.71.0 -- crates/nono-cli/src/tool-sandbox/
```
Returns all 12 files (confirmed): `audit_context.rs`, `credentials.rs`, `dynamic_providers.rs`,
`env.rs`, `launch.rs`, `mod.rs`, `platform/linux.rs`, `platform/macos.rs`, `policy.rs`,
`protocol.rs`, `token_broker.rs`, `url_shim.rs`.

Per-file LOC (via `git show v0.71.0:<path> | wc -l` in a loop) sums to **exactly 18,433** —
matching D-01's hypothesis precisely, with `platform/linux.rs` = 6,586 and `platform/macos.rs` =
7,041 (sum 13,627, confirmed 74% of 18,433). `command_policy.rs` at v0.71.0 = **5,955 LOC**,
`lineage_cgroup.rs` = **556 LOC** (not previously stated in CONTEXT.md's table — new datum).

**No `platform/windows.rs`** — confirmed by the absence from the `ls-tree` output above and by
`platform/` containing exactly two files (`linux.rs`, `macos.rs`).

### 2h. Following mod/use edges out of `tool-sandbox/mod.rs` — the structural finding

`tool-sandbox/mod.rs` itself declares only its own child modules:
```bash
git show v0.71.0:crates/nono-cli/src/tool-sandbox/mod.rs | grep -n "^\s*\(pub(crate)\s*\)\?mod \|^\s*pub mod "
```
Result: `audit_context`, `credentials`, `dynamic_providers` (pub(crate)), `env`, `launch`,
`policy`, `protocol`, `token_broker` (pub(crate)), `url_shim`, plus `platform::{linux, macos}` at
the bottom (behind `#[cfg(any(target_os = "linux", target_os = "macos"))]`).

**None of the four D-03 candidate modules (`command_blocking_deprecation.rs`, `instruction_deny.rs`,
`open_url_runtime.rs`, `hook_runtime.rs`) are declared or referenced inside `tool-sandbox/mod.rs`
or `command_policy.rs`** — zero grep hits for any of the four names in either file. The `mod`
declarations for all four (plus `command_policy` and `lineage_cgroup` and the `tool_sandbox`
re-alias) live at the **crate root, `crates/nono-cli/src/main.rs`**:

```bash
git show v0.71.0:crates/nono-cli/src/main.rs | grep -n \
  "^mod \|^use \|#\[path"
```
Confirms (line numbers from the live v0.71.0 tree):
```
17:mod command_blocking_deprecation;
19:mod command_policy;
31:mod hook_runtime;
32:mod instruction_deny;
37:mod lineage_cgroup;
42:mod open_url_runtime;
82:#[path = "tool-sandbox/mod.rs"]
83:mod tool_sandbox;
103:use command_blocking_deprecation::{...};
```

Further, grepping all 12 `tool-sandbox/*.rs` files plus `command_policy.rs`/`lineage_cgroup.rs` for
each of the four candidate names returns **zero hits in both directions** — the four candidates
never import from the tool-sandbox module set, and the tool-sandbox module set never imports from
them. Their only structural relationship (as visible from static `mod`/`use` grep) is being
sibling top-level modules registered in the same `main.rs`.

**Methodology finding for the plan's D-03 task:** "follow mod/use edges out of `tool-sandbox/mod.rs`
and `command_policy.rs`" (CONTEXT.md's phrasing) will find nothing pointing at the four candidates,
because the wiring — if any exists beyond sibling registration — is not expressed as a direct
`mod`/`use` edge between the files themselves. The re-derivation task should explicitly also grep
`main.rs`'s own `mod`/`use` block (shown above) as the second, necessary half of "following the
edges," and should state plainly in the ledger whether the four candidates are considered in-scope
based on this absence of any cross-reference — that judgment call is D-03's, not this research's,
but the underlying grep facts (zero cross-references in either direction) are now established and
reproducible.

### 2i. Carve-out re-touch check form (D-19)

```bash
RANGE="0551eba27ea53b0eda20f4f796f757dd168adb92..0055bf3c686de6fa665decd9831dc5686b282cb0"
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/exec_strategy_windows/'
# -> (no output)
```

Confirmed the exact form (`git log --no-merges --oneline <range> -- <exact-path>`, directory paths
trailing-slash and single-quoted) works and correctly returns nothing for a path upstream has no
counterpart to — record explicitly as **"clean — no re-touch in window; upstream has no
Windows-specific exec-strategy directory at all"** (a stronger statement than 108's plain "clean,"
since in 116's case the reason for zero hits is upstream never having the surface, not merely not
touching it this window — worth distinguishing in the ledger since it's informative for the ADR's
Pole-A feasibility read).

### 2j. Working tree / worktree hazard check

```bash
git status --short   # clean at session start, no changes
git worktree list
```
`git worktree list` shows several **stale, unrelated worktrees** registered under
`C:/Users/omack/Nono/.claude/worktrees/agent-*` (lowercase `omack`, a different casing/path than
the primary `C:/Users/OMack/Nono` — the exact class of divergence the durable "Windows worktree CWD
divergence" lesson warns about) plus one fix-branch worktree
(`C:/Users/OMack/AppData/Local/Temp/nono-fix3`). **None of these affect this phase**: every command
this research and the plan will run (`git log`, `git show`, `git ls-tree`, `git ls-remote`) is
read-only and does not require a clean working tree or touch the index — they operate on commit
objects directly regardless of worktree state. `workflow.use_worktrees=false` is already the
project's own setting per STATE.md. **No worktree hazard for this phase's actual commands**, but if
the plan's own executor is spawned via a worktree-based orchestration path despite the config
setting, it should verify `pwd` and `git rev-parse --show-toplevel` resolve to the intended
checkout before running any command in this section, per the durable lesson.

## Section 3: Fork Symbol Inventory — Windows Confinement Stack (Greppable Surface Only)

Per the scope fence, this is **inventory, not judgment** — no capability rating is assigned here;
D-07's matrix (executor work) will map upstream capabilities onto this list.

### 3a. `crates/nono-cli/src/exec_strategy_windows/` — file sizes and public surface

| File | Size | Grep pattern needed | Public items found |
|------|------|---------------------|---------------------|
| `restricted_token.rs` | 12.1 KB | `pub\|pub(crate)` | `generate_session_sid()`, `generate_app_container_name()` |
| `labels_guard.rs` | 26.5 KB | `pub\|pub(crate)` | `AppliedLabelsGuard` (struct), `::snapshot_and_apply()` |
| `dacl_guard.rs` | 43.2 KB | `pub\|pub(crate)` | `AppliedDaclGrantsGuard`, `AppliedAncestorTraverseGuard`, `AppliedAncestorReadAttributesGuard` (all 3 with `::snapshot_and_apply[_targets]()`) |
| `network.rs` | 106.2 KB | `pub\|pub(crate)` | `install_windows_wfp_service/driver()`, `start_windows_wfp_driver/service()`, `uninstall_windows_wfp()`, `probe_windows_wfp_readiness()` |
| `launch.rs` | 190.4 KB | `pub\|pub(crate)` | `create_process_containment()`, `apply_process_handle_to_containment()`, `terminate_job_object()`, `verify_broker_authenticode()`, `is_windows_detached_launch()`, plus `JobObjectHandle::create()` |
| `supervisor.rs` | 241.6 KB | **`pub(super)` — NOT `pub`/`pub(crate)`** | `SendableHandle`, `WindowsSupervisorLifecycleState`, `WindowsSupervisedChild`, `WindowsSupervisorRuntime`, `compute_deadline()`, `initialize()`, `attach_detached_stdio()`, `start_streaming()`, `set_child_broker_target()`, `run_child_event_loop()`, `startup_failure()`/`command_failure()`, `shutdown()`, `pty()`, `detached_stdio()`, `initialize_supervisor_control_channel()`, `open_windows_supervisor_path()`, `handle_windows_supervisor_message()` — **23 items total** |
| `mod.rs` | 58.0 KB | `pub\|pub(crate)` | `ExecStrategy`/`ThreadingContext` (enums), `ExecConfig`/`SupervisorConfig` (structs), `WindowsSupervisorDenyAllApprovalBackend`, `ProcessContainment`, the 5 `WindowsWfp*Report` structs, `execute_direct()`, `execute_supervised()`, `resolve_program()`, `is_admin_process()`, `probe_job_object_permissions()`, `probe_integrity_level_support()`, `probe_bfe_service_status()`, `read_distlib_shebang()` — 26 items |

**Methodology finding worth carrying into the plan's D-07/D-16 task instructions:** a naive
`grep -nE "^\s*(pub|pub\(crate\))\s+..." supervisor.rs` returns **zero** hits despite the file
being 241 KB and load-bearing — because its entire public API is `pub(super)`, visible only to
`exec_strategy_windows/mod.rs`, not to the crate at large. The grep pattern used for D-16's
evidence column MUST include `pub(super)` (and ideally scan for any `pub` variant — `pub`,
`pub(crate)`, `pub(super)`, `pub(in ...)`) or it will silently under-report a file's real surface,
which is precisely the "greps must discover their targets, not confirm pre-named ones" failure
mode D-16 already warns against (Phase 115 V-01's lesson, generalized to visibility modifiers, not
just function names).

### 3b. PR#4 fork-native hook + broker path — file inventory

| File | Public/pub(crate) symbols found |
|------|----------------------------------|
| `crates/nono-cli/src/claude_code_hook.rs` | `pub(crate) fn run()` |
| `crates/nono-cli/src/hooks.rs` | `HookInstallResult` (enum), `install_hooks()`, `install_profile_hooks()`, plus 2 embedded-script `pub const` items (`NONO_HOOK_SH`, `NONO_TOOL_HOOK_PS1`) |
| `crates/nono-cli/src/execution_runtime.rs` | `pub(crate) fn execution_start_dir()`, `pub(crate) fn execute_sandboxed()` |
| `crates/nono-shell-broker/src/main.rs` | `BrokerArgs` (struct, `pub`), `parse_args()`, `build_command_line()`, `run()` |
| `crates/nono-cli/data/hooks/nono-tool-hook.ps1` | exists (57 lines per prior project memory; the shipped hook script) |

### 3c. Confirmed: fork has zero tool-sandbox files at HEAD

```bash
ls crates/nono-cli/src/tool-sandbox/    # No such file or directory
ls crates/nono-cli/src/command_policy.rs   # No such file or directory
ls crates/nono-cli/src/lineage_cgroup.rs   # No such file or directory
```
Confirms CONTEXT.md's framing verbatim — the fork has no counterpart to any of the D-06 inherited
module set, on any platform, not just Windows.

### 3d. spike-findings-nono skill — what it contains for the engine-agnosticism criterion (D-09)

Located at `.claude/skills/spike-findings-nono/SKILL.md` + 2 reference files
(`windows-confinement-model.md`, `engine-agnostic-confinement.md`). Content directly relevant to
D-09's engine-agnosticism criterion and D-07's feasibility matrix:

- **"Sandbox-the-tools, not sandbox-the-TUI"** — the achievable Windows confinement model is
  per-tool-invocation confinement (what PR#4 already does), not confining an interactive
  agent-engine process itself. A Low-integrity client cannot register with the Windows console
  subsystem (`cmd.exe` on ConPTY dies `0xC0000142` under both raw Low-IL and AppContainer,
  Spike 001 INVALIDATED) — relevant context for why Pole A's hypothetical `platform/windows.rs`
  would also have to be a per-tool-launch mediator, not a TUI-level sandbox, mirroring upstream's
  own `platform/linux.rs`/`platform/macos.rs` shape.
- **"Daemon-as-launcher is the sound primary model" (Spike 003, VALIDATED)** — a single persistent
  launcher confining arbitrary engines (`cmd.exe`, `powershell.exe`, `python.exe`) identically via
  `nono run --profile <...> -- <engine.exe>`, proven on Win11 26200.8390. This is direct evidence
  for D-09's engine-agnosticism criterion: the fork's *underlying primitive* (`nono run`) is
  already engine-neutral; only the **entry point** (Claude Code's PreToolUse hook) is
  Claude-Code-specific. This distinction — primitive vs. entry point — is worth the ADR making
  explicit when scoring D-09, since it changes what "the fork's engine-agnosticism gap" actually
  means (a hook-contract problem, not a confinement-primitive problem).
- **Requirements/constraints documented as spike findings, not yet built:** executable-coverage
  contract (nono fail-secure refuses to launch an engine whose binary isn't covered by an
  `--allow`), grants-are-absolute (engines don't uniformly inherit launcher CWD), user-mode only
  (no kernel driver — consistent with ADR-65's standing verdict), CLM-safe payload requirement for
  confined writes under the real Claude-Code hook path (Constrained Language Mode is inherited from
  the launch environment).
- **Explicitly NOT yet spiked:** persistent token/job reuse across many agents + multi-tenant
  `AI_AGENT` marker + one persistent multi-client capability pipe (was spike 004, never run); the
  `nono-py`-binding-driven real Python/LangChain agent proof (was spike 005, never run). These are
  named gaps in the skill's own metadata, relevant if the ADR's feasibility matrix or Phase 120
  sizing needs to note "engine-agnosticism beyond Claude Code" as still partially unvalidated even
  for the fork-native path, not just for a hypothetical `platform/windows.rs`.

`proj/DESIGN-engine-abstraction.md` also exists (confirmed present) — canonical refs point to it
as "the engine-agnosticism criterion's fork-side context"; not read in full here per the research
scope (it's canonical-ref material for the ADR author to read directly, not something this
research needs to summarize further).

## Section 4: Upstream Surface Inventory — `tool-sandbox/` at `v0.71.0`

(File listing and mod/use edges already given in Section 2f–2h; consolidated here per the
scope-fence's own "Upstream surface inventory" requirement.)

**12 files, 18,433 LOC total** (verified by summing live `git show v0.71.0:<path> | wc -l` per
file):

| File | LOC |
|------|-----|
| `audit_context.rs` | 19 |
| `credentials.rs` | 118 |
| `dynamic_providers.rs` | 1,291 |
| `env.rs` | 586 |
| `launch.rs` | 98 |
| `mod.rs` | 491 |
| `platform/linux.rs` | 6,586 |
| `platform/macos.rs` | 7,041 |
| `policy.rs` | 1,334 |
| `protocol.rs` | 295 |
| `token_broker.rs` | 474 |
| `url_shim.rs` | 100 |

`platform/linux.rs` + `platform/macos.rs` = 13,627 = **73.9% of 18,433** (matches D-01's "74%"
framing). `platform/windows.rs` does not exist (confirmed by directory listing — `platform/`
contains exactly these two files).

`command_policy.rs` at `v0.71.0`: **5,955 LOC** (matches D-01 exactly).
`lineage_cgroup.rs` at `v0.71.0`: **556 LOC** (new datum, not in D-01's table).

**Platform driver entry points** (the names D-07's matrix will need to cite): `mod.rs` declares
`mod linux;` / `mod macos;` under `#[cfg(any(target_os = "linux", target_os = "macos"))]` at lines
475/485 — i.e. the platform dispatch is compile-time cfg-gated exactly like the fork's own
`crates/nono/src/sandbox/{linux,macos}.rs` split, a structurally familiar pattern for whoever
writes a hypothetical `platform/windows.rs` counterpart. The non-cfg-gated top of `mod.rs` (lines
1–27, shown in Section 2f) defines a **no-op stub** (`PreparedToolSandboxRuntime`,
`maybe_run_internal_tool_sandbox_entrypoint() -> false`, etc.) for any platform NOT linux/macos —
meaning upstream's own crate **already compiles cleanly on Windows today**, just with the entire
subsystem inert. This is a directly relevant, previously-unstated fact for the ADR: adopting Pole
A does not require modifying upstream's own cfg-gate structure to "make room" for Windows — the
room is already there, deliberately, as a no-op. Building `platform/windows.rs` is additive to
upstream's own existing shape, not a fork against it.

**D-03 candidate modules — confirmed present at `v0.71.0`, zero cross-reference to the module set
in either direction** (see Section 2h for the full grep evidence): `command_blocking_deprecation.rs`
(192 LOC), `instruction_deny.rs` (159 LOC), `open_url_runtime.rs` (210 LOC), `hook_runtime.rs`
(601 LOC). All four are declared as sibling `mod` statements in `main.rs` alongside
`tool_sandbox`/`command_policy`/`lineage_cgroup`, with no `use` edges connecting any of the four to
any of the three module-set members, in either direction.

## Architecture Patterns

Not applicable in the runtime-code sense (D-20: no code changes). The relevant "pattern" is
purely documentary: see Section 1's ledger/ADR template. No project structure, no diagram, no
code examples beyond the git commands already given in Section 2.

## Don't Hand-Roll

Not applicable — no library/algorithm choice exists in a docs-only phase. The closest analog: do
not hand-roll a new ledger/ADR format. Reuse the exact section skeleton in Section 1, which is
already a proven, four-times-repeated house convention (Phases 85/94/98/108 for ledgers;
ADR-86/98/108/111/113/114 for ADRs).

## Common Pitfalls

### Pitfall 1: Substring-anywhere pathspec globs
**What goes wrong:** `git log <range> -- '*tool-sandbox*'` over-counts by matching
`docs/cli/features/tool-sandbox.mdx` and any other path containing the substring anywhere.
**Why it happens:** git pathspec globs match the whole path, not just the basename or a directory
boundary; `*X*` matches `X` wherever it appears, including in doc paths having nothing to do with
the source module.
**How to avoid:** always use the directory-only form with a trailing slash
(`'crates/nono-cli/src/tool-sandbox/'`) or the explicit file path, single-quoted so the shell
doesn't expand it, and use the 3-path union form for D-03's inherited-set baseline.
**Warning signs:** a commit count for the tool-sandbox surface that includes commits whose ONLY
matching path is under `docs/`.

### Pitfall 2: `pub(super)` invisible to a `pub|pub(crate)` grep
**What goes wrong:** D-16's evidence column requires citing "the actual grep run"; a grep pattern
that only matches `pub`/`pub(crate)` silently reports zero symbols for files (like
`exec_strategy_windows/supervisor.rs`) whose entire API surface is `pub(super)`.
**Why it happens:** Rust visibility has more forms than the two most common ones; a
narrowly-written regex looks successful (it runs, it returns *something* for most files) while
being wrong for specific files.
**How to avoid:** always test the grep pattern against a file known to have symbols before trusting
a zero-hit result as "this file exports nothing" — a zero count is itself a signal to broaden the
pattern (`pub`, `pub(crate)`, `pub(super)`, `pub(in ...)`), not to conclude the file is symbol-free.
**Warning signs:** a 200+ KB file reporting zero public symbols in a confidence-rating grep.

### Pitfall 3: Treating "no mod/use edge from the subsystem's own files" as "no relationship"
**What goes wrong:** the D-03 candidate modules have zero direct `mod`/`use` edges to/from
`tool-sandbox/mod.rs` or `command_policy.rs` — but that is a fact about *static import syntax*,
not necessarily about *runtime coupling*. `main.rs` calls `tool_sandbox::maybe_run_internal_tool_
sandbox_entrypoint()` unconditionally at the very top of `fn main()`, before any CLI parsing —
whether `command_blocking_deprecation`/`hook_runtime`/etc. participate in the SAME early-dispatch
control flow (rather than importing each other's types) is a question this research's grep-only
method cannot answer and D-03's re-derivation should check by reading `main.rs`'s control flow, not
just its `mod`/`use` block.
**Why it happens:** "follow mod/use edges" is a necessary but not sufficient check for "does this
module participate in the subsystem" when a language allows call-without-import-of-types (calling
a sibling module's free function needs no `use`, only the `mod` declaration already present at the
crate root).
**How to avoid:** for the four D-03 candidates, read `main.rs`'s actual body (not just its `mod`/
`use` header) to check whether any of the four functions get called adjacent to, or gated by,
tool-sandbox entrypoint logic — Section 2h already found `command_blocking_deprecation`'s
`collect_cli_warnings`/`print_warnings` running in the same `fn main()` body as `tool_sandbox`'s
entrypoint checks, though calling neither module's functions on the other's types.

### Pitfall 4: Conflating "the full 187-commit window" with "the 38-commit dispositioned surface"
**What goes wrong:** D-01's window is `v0.64.1..v0.71.0` (187 non-merge commits total), but D-02's
per-commit disposition obligation applies to the 38-commit tool-sandbox surface within that window,
not all 187. A plan or ledger that tries to disposition all 187 has silently expanded scope past
TSBX-01/02 and duplicated Phase 108's already-completed non-tool-sandbox audit work for the
`v0.66.0..v0.69.0` portion, plus taken on unscoped new work for the `v0.69.0..v0.71.0` portion that
belongs to a future UPST13/FUT-08 audit, not this phase.
**Why it happens:** "the window" is used to describe both the git range (D-01) and the scope of
work (D-02) in CONTEXT.md's prose; they are not the same set.
**How to avoid:** the plan should state explicitly, in its own scope section, that the 38-commit
3-path-union surface is the dispositioned set, and that the remaining 149 commits in the full
window are out of scope for this phase (they belong to Phase 108's already-closed audit for the
fenced portion, and to a future sync for the post-`v0.69.0` non-tool-sandbox portion).

## Validation Architecture

`workflow.nyquist_validation` is `true` in `.planning/config.json` (present, not absent), so this
section is required. There is no test framework, no code, and `make ci` is explicitly not a gate
(D-20) — "verification" here means: can a reader independently reproduce every factual claim in
the two deliverables using only the commands recorded in them?

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None — this phase produces documentation, not code |
| Config file | N/A |
| Quick run command | Re-run any single Reproduction-block command from Section 2 and diff against the recorded output |
| Full suite command | Re-run the entire Reproduction block (Section 2a–2i) end-to-end and diff every count against the ledger's stated figures |

### Phase Requirements → Evidence Map

| Req ID | Behavior | Verification Type | Reproducible Command | Evidence exists at research time? |
|--------|----------|--------------------|-----------------------|----------------------------------|
| TSBX-01 | Ledger covers the full 38-commit surface with `windows-touch` + disposition per commit | git-archaeology re-run | `git log --no-merges --oneline $RANGE -- <3-path-union>` then per-commit `git show --name-only --format='' <sha>` | ✅ commands verified live this session (Section 2c) |
| TSBX-01 | Pinned-SHA Reproduction block matches live upstream state | tag/SHA re-resolution | `git ls-remote --tags upstream v0.64.1 v0.71.0` | ✅ verified live, matches CONTEXT.md exactly (Section 2a) |
| TSBX-02 | Every confidence rating cites a grep with hit count | symbol-level grep re-run | the specific `grep -nE "pub..."` pattern the ledger/ADR cites, against the specific file, at the specific ref | ✅ demonstrated live for both fork (Section 3a) and upstream (Section 2h) surfaces; the `pub(super)` gap (Pitfall 2) is itself proof this check catches real errors |
| TSBX-02 | Feasibility matrix rows cite fork symbol or ADR-65 | citation-format check | grep the ADR text for each matrix row's citation, confirm the cited symbol exists via `grep -n <symbol> <file>` | Not yet run (executor work — matrix rows don't exist until the ADR is written) |
| TSBX-02 | ADR carve-out re-touch checks are all-clean or all-routed | re-run per-surface `git log` | Section 2i's exact form, once per accumulated fork carve-out | ✅ form verified live for one example carve-out (Section 2i); the executor must run it for all accumulated carve-outs (7+ per Phase 108's precedent, likely more by Phase 115) |

### Sampling Rate

- **Per plan/task commit:** re-run the specific git command(s) that task's evidence table cites,
  confirm the count matches what's written into the ledger/ADR before committing the task.
- **Per wave merge:** re-run the FULL Reproduction block (all of Section 2's commands) and confirm
  every count still matches — `upstream` is a live remote and could theoretically move between
  task execution and wave close (D-01 already names this risk: "any other number means upstream
  has moved and the audit must be re-run from scratch," mirroring 108's own Reproduction-block
  language).
- **Phase gate:** before `/gsd:verify-work 116`, a reviewer re-runs every single command that
  appears literally in either deliverable (not a sample) and confirms the output matches — this
  is checkable in full because the total command count is small (git log/show/ls-tree/ls-remote
  invocations, dozens not hundreds) and every one is deterministic given the pinned SHAs.

### Wave 0 Gaps

None — there is no test framework to stand up. The "Wave 0" equivalent for this phase is
confirming the pinned SHAs still resolve (Section 2a) and the working tree is clean (Section 2j)
before task execution begins; both already confirmed true at research time.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The four D-03 candidate modules' lack of static `mod`/`use` cross-reference to the tool-sandbox module set means they are likely OUT of the re-derived module set — stated as a lean, not a locked call, per the scope fence | Section 2h, Section 4 | If the executor's deeper control-flow read (Pitfall 3) finds runtime coupling this research's static grep missed, the in-or-out call could flip; this research explicitly does not make that call, only supplies the grep evidence |
| A2 | Upstream's `command_policy.rs`/`lineage_cgroup.rs` LOC figures (5,955 / 556) and the D-03 candidates' LOC figures are stable — measured once, at v0.71.0, this session | Section 2g, Section 4 | Low risk — these are exact `git show \| wc -l` counts against pinned SHAs, not estimates; a re-run against the same SHAs will reproduce them exactly |
| A3 | The `pub(super)` visibility gap generalizes as a real risk across the rest of the fork's Windows stack (i.e. other files might also hide symbols behind less-common visibility modifiers not yet grepped for) | Section 3a, Pitfall 2 | If other fork files (not yet exhaustively swept) have similarly-hidden surfaces, D-07's matrix could under-cite the fork's actual capabilities; mitigated by the Pitfall 2 warning-sign guidance (treat zero-hit as suspicious, not conclusive) |

**No claim in this research is tagged `[ASSUMED]` in the source-provenance sense** — every factual
claim about commit counts, file existence, LOC, and symbol names was verified live via git commands
against the pinned SHAs during this research session (`[VERIFIED: git — <command>]` implicitly, per
command, throughout Sections 2–4). The three items above are the genuine open judgment calls this
research surfaces for the executor, not unverified facts.

## Open Questions

1. **Does the D-03 candidate-module in-or-out call affect the D-01 hypothesis table's 38-commit
   figure?**
   - What we know: the four candidates have zero static cross-reference to the current 3-module
     inherited set (Section 2h).
   - What's unclear: whether including any of them in the re-derived module set would change the
     commit-surface count from 38 to something larger (their own commit histories were not
     enumerated by this research — out of scope per the scope fence, since that's the exact
     "compute a number that would end up in the ledger" boundary).
   - Recommendation: the plan should have D-03's task compute
     `git log --no-merges --oneline $RANGE -- crates/nono-cli/src/command_blocking_deprecation.rs
     crates/nono-cli/src/instruction_deny.rs crates/nono-cli/src/open_url_runtime.rs
     crates/nono-cli/src/hook_runtime.rs` as one of its first steps, to see whether the question is
     even live (if these four files were untouched in the window, the in-or-out call is moot for
     TSBX-01's commit surface either way, though it may still matter for TSBX-02's Pole-A scoping).

2. **Does upstream's `platform/` cfg-gate stub (Section 4) simplify Pole A's actual build, or is it
   a red herring?**
   - What we know: upstream's own `mod.rs` already compiles a no-op stub for non-linux/macos
     targets — the crate does not need forking to "make room" for a Windows arm.
   - What's unclear: whether the *shape* of the stub (specifically its function signatures —
     `maybe_run_internal_tool_sandbox_entrypoint() -> bool`, `record_main_start()`,
     `log_main_total()`, `cleanup_runtime_dir()`) constrains what a real Windows implementation
     would need to match, or whether upstream would need its own PR to accept a
     `platform/windows.rs` (i.e. is this an upstream-contribution question or a pure fork-internal
     build question, given the fork never syncs `tool-sandbox/` at all per D-06/CONTEXT.md).
   - Recommendation: D-07's feasibility matrix should note the stub's existence as a factual
     grounding point (it already does not need to be argued from scratch), but the "who builds
     this and where does it live" question is a Phase 120 sizing question (D-15), not this phase's.

## Deliberately Not Researched

Per the scope fence, the following were intentionally NOT computed, even though they are directly
adjacent to what this research did compute — flagging explicitly so the planner does not read
their absence as an oversight:

- **Per-commit disposition (adopt/adapt/skip/split) for any of the 38 surface commits** — D-02's
  job, executor work.
- **Pure vs. split classification for the 11 post-fence commits** — the D-05/D-05-analog exercise
  Phase 108 ran for its 20-commit fenced surface (finding 9 pure / 11 split); this research did not
  run the equivalent `git show --name-only` per-path classification for the 11 post-fence commits
  named in D-01's table (`#1476`, `#1492`, `#1467`, `#1453`, `#1521`, `#1550`, plus unnamed others),
  because doing so would produce exactly the kind of disposition-adjacent number the scope fence
  forbids pre-computing.
- **Any capability rating for D-07's feasibility matrix** — this research supplied the greppable
  symbol inventory (Section 3) both sides of the matrix will cite, but assigned zero
  implementable/new-work/blocked ratings.
- **Any lean toward Pole A or Pole B** — Section 3d's spike-findings summary is presented as
  neutral grounding for the engine-agnosticism criterion, not as an argument for either pole; the
  "primitive vs. entry point" framing noted there is offered as a scoring nuance, not a conclusion.
- **The full commit-count breakdown for the 149 non-tool-sandbox commits in the 187-commit full
  window** — out of scope per Pitfall 4; not touched beyond confirming the total (187) and the
  scoped subset (38) are different numbers that must not be conflated.

## Sources

### Primary (HIGH confidence — live command execution this session)
- `git` CLI against `upstream` remote (`nolabs-ai/nono`) — every command in Sections 2, 3c, 4,
  run live on 2026-08-09 from `C:\Users\OMack\nono`.
- `crates/nono-cli/src/exec_strategy_windows/{mod,restricted_token,labels_guard,dacl_guard,
  network,launch,supervisor}.rs` — fork HEAD, read/grepped directly.
- `crates/nono-cli/src/{claude_code_hook,hooks,execution_runtime}.rs`,
  `crates/nono-shell-broker/src/main.rs` — fork HEAD, read/grepped directly.
- `.claude/skills/spike-findings-nono/{SKILL.md,references/windows-confinement-model.md,
  references/engine-agnostic-confinement.md}` — read directly.
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — read structurally
  (section headers via grep, then targeted reads of Reproduction, tool-sandbox-pure/split, and
  Carve-out Re-touch Check sections).
- `.planning/phases/108-upst12-divergence-audit/108-CONTEXT.md`,
  `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-CONTEXT.md` — read in
  full.
- `proj/ADR-113-spiffe-disposition.md`, `proj/ADR-111-resource-limits-boundary.md` — read in full.
- `proj/ADR-108-deny-domain-posture.md`, `proj/ADR-114-oauth-capture-disposition.md`,
  `proj/ADR-86-library-boundary-convergence.md` — section structure confirmed via grep.
- `.planning/architecture/adr-65-minifilter-go-no-go.md` — read (partial, through the Go/No-Go
  verdict table).
- `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md` (partial — first page),
  `CLAUDE.md`, `.planning/config.json` — read directly.

### Secondary (MEDIUM confidence)
- None — every claim in this document is either a direct file read or a live-executed, recorded
  command.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- House format (Section 1): HIGH — directly sampled from the four precedent ledgers/ADRs, not
  inferred.
- Tooling reproducibility (Section 2): HIGH — every command live-executed this session against the
  pinned SHAs; all numbers cross-checked for internal arithmetic consistency.
- Fork/upstream symbol inventories (Sections 3–4): HIGH for existence/LOC/grep-hit facts; the
  Pitfall 2 (`pub(super)`) finding is itself evidence the inventory method is sound (it caught its
  own initial gap).
- Feasibility/verdict content: N/A — deliberately not researched, per scope fence.

**Research date:** 2026-08-09
**Valid until:** ~7 days for the pinned-SHA reproduction commands (upstream is a live, actively-
released remote — re-verify `git ls-remote --tags upstream v0.71.0` before executing the plan if
more than a few days have passed); the house-format/template findings (Section 1) are stable for
the life of this ADR-naming convention (effectively indefinite, tied to the project's own
established pattern, not to upstream's release cadence).
