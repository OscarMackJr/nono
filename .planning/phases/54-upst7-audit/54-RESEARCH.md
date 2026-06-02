# Phase 54: UPST7 Audit - Research

**Researched:** 2026-06-01
**Domain:** Upstream-parity divergence audit (analysis/doc-producing phase) for upstream `v0.57.0..v0.59.0`
**Confidence:** HIGH (methodology + touchpoints repo-local-verified; commit landscape MEDIUM-HIGH pending the mandatory re-fetch)

## Summary

Phase 54 is an **audit/analysis phase**, not a code-implementation phase. It produces exactly one durable artifact: `DIVERGENCE-LEDGER.md` covering upstream `v0.57.0..v0.59.0`, with per-cluster dispositions, a `windows-touch` column, an `## ADR review` section, an `## Empirical cross-check` section (diff-inspect, not `--name-only`), and frontmatter recording a fresh upstream HEAD SHA + re-fetch date. The phase is the binding input for Phase 55 (the cherry-pick wave). It ships **zero** `.rs`/`.toml`/`.sh`/`.ps1`/`Makefile` edits — only `.planning/` artifacts.

The fork has an exceptionally well-established methodology for this exact task: Phase 33 (UPST3), Phase 39 (UPST4), Phase 42 (UPST5 audit), and **Phase 47 (UPST6 audit)** are direct precedents. Phase 47's `DIVERGENCE-LEDGER.md` and its `47-01-UPST6-AUDIT-PLAN.md` are the verbatim template Phase 54 should mirror — same frontmatter schema, same 8-task structure (mechanical preamble → scaffold → human audit-walk → re-export scan → ADR review → empirical cross-check → ROADMAP stub → SUMMARY), same disposition vocabulary, same close-gates. The single substantive change versus Phase 47 is the range (`v0.57.0..v0.59.0` instead of `v0.54.0..v0.57.0`) and two SC-mandated additions: (SC3) a mandatory re-fetch because the v0.58/v0.59 tags are **not yet local**, and (SC4) an explicit diff-inspect note on the fork-divergent TLS-interception surface (Phase 34 Cluster C11).

**Primary recommendation:** Clone the Phase 47 UPST6 audit plan structure verbatim, retarget the range to `v0.57.0..v0.59.0`, set `fork_baseline: v0.57.0`, and add two SC-driven tasks/sub-sections: (1) a re-fetch step in the mechanical preamble that captures the post-fetch `upstream/main` HEAD SHA + date into frontmatter (and surfaces the v0.60.0 scope question to the human), and (2) a dedicated TLS-interception diff-inspect note in the ledger addressing whether the v0.59 "endpoint-rules-before-credential-selection" ordering fix applies cleanly to the fork's `route.rs`/`connect.rs`/`credential.rs` surface or needs manual replay. This is a **single analysis plan** (one PLAN.md), human-in-the-loop (`autonomous: false`), mirroring Phase 47 Plan 47-01.

## Architectural Responsibility Map

This is a documentation/analysis phase; "tiers" map to artifact-production responsibilities rather than runtime tiers.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Enumerate upstream commit set | Drift tool (`scripts/check-upstream-drift.{sh,ps1}`) | git CLI (`git log/show`) | Tool is the locked source of truth for "what diverged"; D-11 path filter applied |
| Re-fetch upstream + capture HEAD SHA | git CLI (`git fetch upstream --tags`) | Ledger frontmatter | SC3 mandatory; local upstream is stale at v0.57.0 era |
| Cluster grouping + disposition | Human auditor judgment | drift JSON metadata | Substantive judgment; auto-runner cannot decide will-sync vs split |
| Re-export surface verification | git CLI (`git show <sha>:<file>` + grep) | Human interpretation | Diff-inspect per `feedback_cluster_isolation_invalid`; not `--name-only` |
| ADR re-confirm verdict | Human auditor judgment | Phase 33 ADR doc | L/M/H per-cell verdict on 5 dimensions; does NOT supersede |
| TLS-intercept clean-apply assessment (SC4) | Human auditor + git diff-inspect | fork `nono-proxy` source | Fork-preserve surface; needs replay-vs-cherry-pick judgment |

## Standard Stack

This phase installs **no external packages**. It uses only repo-local tooling and git. The Package Legitimacy Audit and Environment Availability sections below reflect that.

### Core (repo-local tooling)
| Tool | Version/SHA | Purpose | Why Standard |
|------|-------------|---------|--------------|
| `scripts/check-upstream-drift.sh` | content SHA `0834aa66...` (current `git log` SHA `0834aa664fbaf4c5e41af5debece292992211559`) [VERIFIED: repo grep] | Enumerate unabsorbed upstream commits in a range, grouped by category, with D-11 path filter | Locked reproducibility pin used by every prior UPST audit (Phase 33/39/42/47) |
| `scripts/check-upstream-drift.ps1` | twin script | Windows-host dispatch (Makefile `check-upstream-drift` target auto-selects) | Cross-platform twin; same JSON output |
| `make check-upstream-drift ARGS="..."` | Makefile target line 80-88 [VERIFIED: repo grep] | Invocation wrapper; dispatches to `.sh` or `.ps1` | The D-47-A2-locked invocation form |
| `git` | system | `fetch --tags`, `rev-parse`, `log`, `show` for re-export diff-inspect | Standard |

**Drift tool category set (fixed, deterministic):** `profile`, `policy`, `package`, `proxy`, `audit`, `other` [VERIFIED: repo grep of `.sh` line 285 JSON emitter]. Multi-category commits double-count across categories.

**D-11 path filter (drift tool EXCLUDES):** `*_windows.rs` and `crates/nono-cli/src/exec_strategy_windows/` [VERIFIED: repo grep `.sh` lines 120-121]. The tool reports cross-platform Rust under `crates/{nono,nono-cli,nono-proxy}/src/` plus `crates/nono/Cargo.toml`. This is why the `windows-touch` column and the empirical cross-check exist — they cover what the tool is structurally blind to.

### Supporting (precedent artifacts to clone)
| Artifact | Purpose | When to Use |
|----------|---------|-------------|
| `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-UPST6-AUDIT-PLAN.md` | The verbatim 8-task plan template | Clone task structure; retarget range |
| `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md` | The worked ledger output template | Mirror frontmatter + all section shapes |
| `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` | Earlier worked template (cited by 47 plan) | Cross-reference for cluster/ADR shape |
| `docs/architecture/upstream-parity-strategy.md` | Phase 33 ADR (Option A `continue`, `Status: Accepted`) | ADR review section input; do NOT modify |
| `.planning/quick/260527-sgo-upstream-v044-v059-gap-analysis/GAP-ANALYSIS.md` | The ~19-commit v0.58/v0.59 starting inventory + 6 phase buckets | Seed for cluster grouping; cross-check against drift JSON |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Drift tool (locked SHA) | Raw `git log v0.57.0..v0.59.0 --oneline` | Loses category grouping + D-11 filter + deterministic JSON; breaks reproducibility pin. Use raw git only as a *cross-check* (e.g., for PR-number traceability per gap analysis Section 4), not as the source of truth. |
| Single analysis plan | Multi-plan/multi-wave | Unjustified — Phase 47 audit was a single plan (47-01); the v0.58/v0.59 commit set (~19 commits per gap analysis) is smaller than Phase 47's 42. One plan is correct. |

**No installation step.** All tooling is repo-local and already present.

## Package Legitimacy Audit

**Not applicable.** Phase 54 installs **zero** external packages. It is a doc-producing audit phase that runs only repo-local scripts (`check-upstream-drift.{sh,ps1}`) and git. No npm/PyPI/crates registry interaction occurs. slopcheck not run because there are no package candidates.

## Architecture Patterns

### System Architecture Diagram

```
                       Phase 54 UPST7 Audit (analysis only)
                       ====================================

  [git fetch upstream --tags]  ──SC3 MANDATORY──►  upstream/main HEAD + v0.58/v0.59/(v0.60) tags
            │                                          │
            │                                  capture HEAD SHA + date
            ▼                                          ▼
  ┌──────────────────────┐                  ┌─────────────────────────┐
  │ check-upstream-drift  │── --from v0.57.0 │  Ledger frontmatter      │
  │  (SHA 0834aa66...)    │   --to v0.59.0   │  range/upstream_head/    │
  │  D-11 path filter      │   --format json  │  drift_tool_sha/date     │
  └──────────┬───────────┘                  └─────────────────────────┘
             │ JSON (ci-logs-local/, NOT committed)
             │ total_unique_commits + per-commit {sha,subject,files_changed,categories}
             ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │ HUMAN AUDIT-WALK                                                  │
  │  • cluster grouping (themed)                                      │
  │  • per-cluster disposition: will-sync / fork-preserve /           │
  │       won't-sync / split                                          │
  │  • windows-touch column (yes/no) — D-47-A5 heuristic + judgment   │
  │  • commit-row tables (sha|subject|tag|categories|files|win-touch) │
  └──────────┬────────────────────────────────────────────┬─────────┘
             │                                              │
   per will-sync cluster:                          fork-divergent surfaces:
   diff-inspect re-export surface                  • Phase 33 ADR review (5-dim L/M/H)
   (git show <sha>:<file> | grep pub use/mod)      • Phase 34 C11 TLS-intercept (SC4
   flip to `split` on cross-cluster dep              diff-inspect note: clean-apply
             │                                        vs manual-replay)
             ▼                                              ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │ DIVERGENCE-LEDGER.md  (the ONLY durable code-adjacent output)     │
  │  Headline · Reproduction · Cluster Summary · per-Cluster ·        │
  │  ## ADR review · ## Empirical cross-check ·                       │
  │  ## Cross-cluster re-export deps detected                         │
  └──────────┬────────────────────────────────────────────────────────┘
             │ immutable input
             ▼
        Phase 55 cherry-pick wave (consumes dispositions)
```

### Recommended Phase Directory Structure
```
.planning/phases/54-upst7-audit/
├── 54-RESEARCH.md                  # this file
├── 54-CONTEXT.md                   # (if discuss-phase runs)
├── 54-01-UPST7-AUDIT-PLAN.md       # single analysis plan (clone of 47-01)
├── 54-01-LOCK-NOTES.md             # re-fetch HEAD SHA + tag-assert holding file (Task 1)
├── DIVERGENCE-LEDGER.md            # the deliverable
└── 54-01-SUMMARY.md                # plan close summary
```
Raw drift JSON goes to `ci-logs-local/drift/<timestamp>-v057-v059.json` and is **NOT committed** (`ci-logs-local/` is gitignored) — per D-47-E1 / D-33-A2 inherited.

### Pattern 1: Mechanical preamble → human audit-walk split
**What:** Tasks 1-3 (fetch, drift-run, scaffold) are `type: auto`; Tasks 4-7 (cluster grouping, re-export scan, ADR verdict, empirical cross-check) are `type: checkpoint:human-action`; Task 8 (ROADMAP stub) + SUMMARY are `auto`.
**When to use:** Every UPST audit. Disposition/grouping/ADR-verdict require human judgment; scaffolding and grep-verification are auto.
**Example:** See `47-01-UPST6-AUDIT-PLAN.md` task `type` attributes verbatim. Set plan frontmatter `autonomous: false`.

### Pattern 2: Reproducible frontmatter (D-47-A2 / D-47-A3)
**What:** Ledger frontmatter captures the exact inputs so a fresh auditor regenerates the inventory deterministically.
**Example (retargeted for Phase 54):**
```yaml
---
phase: 54-upst7-audit
plan: 01
ledger_type: upst7-audit
range: v0.57.0..v0.59.0
upstream_head_at_audit: <40-char post-fetch upstream/main SHA — captured at first commit of Plan 54-01>
refetch_date: <UTC date of the SC3 git fetch>          # SC3 NEW field
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'make check-upstream-drift ARGS="--from v0.57.0 --to v0.59.0 --format json"'
fork_baseline: v0.57.0 (Phase 48 UPST6 sync point — 42 commits across v0.55.0..v0.57.0 absorbed 2026-05-25)
date: <ship date YYYY-MM-DD>
---
```
**Source:** Phase 47 frontmatter [VERIFIED: read of `47-.../DIVERGENCE-LEDGER.md` lines 1-13]. The `refetch_date` field is the SC3 addition (Phase 47 folded the fetch date into the lock-notes file; Phase 54 SC3 says the *ledger frontmatter* records it — add it explicitly).

### Pattern 3: Diff-inspect re-export surfaces (NOT `--name-only`)
**What:** For every `will-sync` cluster's lead commit, run `git show <sha>:<file> | grep -nE "^pub use |^pub mod |^extern crate |pub\(crate\) "` and trace each re-exported symbol to its definition. If the definition lives in another cluster's commits within range, flip disposition to `split`.
**Why mandatory:** `feedback_cluster_isolation_invalid` — Phase 43 proved cluster isolation can be **empirically false** (Cluster 2's `8b888a1c` re-exported `public_key_id_hex` + `sign_statement_bundle` from upstream commits the fork hadn't absorbed). A `--name-only` diff would have missed it.
**Example:** Phase 47's only re-export edge was `c2c6f2ca`'s `pub use sandbox::{DetectedAbi, LandlockScopePolicy, ...}` — verified intra-cluster (symbols introduced in the same commit) via `git show c2c6f2ca -- crates/nono/src/sandbox/linux.rs | grep '^+pub'`. [VERIFIED: read of `47-.../DIVERGENCE-LEDGER.md` lines 23, 307-313]

### Anti-Patterns to Avoid
- **Trusting `--name-only` for isolation:** the whole point of SC2. Always `git show <sha>:<file>` and trace symbols.
- **Skipping the re-fetch:** local `upstream/main` is at `807fca38` (v0.57.0 era, 2026-05-22). The v0.58/v0.59 tags are NOT local. Running the drift tool without fetching first will fail or produce an empty/wrong range.
- **Modifying the Phase 33 ADR:** the ADR review *verdicts* but does NOT supersede. `docs/architecture/upstream-parity-strategy.md` must not be edited by this phase.
- **Bumping the drift tool SHA:** the reproducibility pin (`0834aa66...`) is an invariant. If a drift-tool gap is found, spawn a separate quick-task (D-47-E10) — do not edit the tool inside this plan.
- **Blind cherry-pick assumption on TLS-intercept:** SC4 — the fork has NO `tls_intercept/` module (D-34-B1 fork-preserve). Assess clean-apply explicitly.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Enumerate divergent upstream commits | Custom `git log` parsing + ad-hoc grouping | `make check-upstream-drift ARGS="--from v0.57.0 --to v0.59.0 --format json"` | Locked reproducibility pin; D-11 filter; deterministic categorized JSON; every prior UPST used it |
| Ledger schema / section layout | New artifact format | Clone `47-.../DIVERGENCE-LEDGER.md` structure verbatim | Phase 55 + future audits expect the established shape; plan-checker verifies grep-falsifiable headers |
| Plan task breakdown | New task taxonomy | Clone `47-01-UPST6-AUDIT-PLAN.md` 8-task shape | Proven auto/human split; close-gates already encoded |
| ADR review dimensions | New scoring rubric | The 5 fixed dimensions (security/windows/maintenance/divergence/contributor) with L/M/H | D-47-E8 falsifiability gate; grep-verified |

**Key insight:** Phase 54 is a *clone-and-retarget* of Phase 47, not a novel design. The single source of truth for "what diverged" is the locked drift tool; the single source of truth for "how to present it" is the Phase 47 ledger. The new work is judgment (dispositions, ADR verdict, TLS-intercept assessment), not invention.

## Runtime State Inventory

> This phase is **not** a rename/refactor/migration of code. It produces a `.planning/` document and ships zero `crates/`/`bindings/`/`scripts/` edits. A full runtime-state inventory is not applicable. The relevant "state" is git/upstream refs:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no datastore touched | None |
| Live service config | None | None |
| OS-registered state | None | None |
| Secrets/env vars | None | None |
| Build artifacts | None — zero code edits, so no rebuild | None |
| **git/upstream refs (the one stateful thing)** | Local `upstream/main` is stale at `807fca38` (v0.57.0 era); v0.58.0/v0.59.0/v0.60.0 tags are NOT fetched locally [VERIFIED: `git rev-parse v0.58.0` fails locally; `git ls-remote --tags upstream` shows them remotely] | **MANDATORY `git fetch upstream --tags`** at audit-open (SC3); capture post-fetch HEAD SHA + date into ledger frontmatter |

## Common Pitfalls

### Pitfall 1: Stale local upstream refs (the SC3 trap)
**What goes wrong:** Auditor runs the drift tool against `v0.57.0..v0.59.0` but `v0.59.0` doesn't resolve locally; tool errors or silently produces wrong output.
**Why it happens:** Local `upstream/main` was last fetched 2026-05-22 at `807fca38` (v0.57.0 era). The drift tool's `--to` auto-detect uses `git describe --tags --abbrev=0 upstream/main`, which would return `v0.57.0` without a fetch.
**How to avoid:** Task 1 (mechanical preamble) runs `git fetch upstream --tags` FIRST, asserts `git rev-parse v0.58.0` and `v0.59.0` resolve, captures `git rev-parse upstream/main` into the lock-notes + frontmatter. [VERIFIED: drift tool `.sh` lines 106-107 require `upstream/main` fetched]
**Warning signs:** `git rev-parse v0.59.0` returns "unknown revision"; drift JSON `total_unique_commits` is 0 or implausibly low.

### Pitfall 2: v0.60.0 (and v0.6.x) scope creep / version-string confusion
**What goes wrong:** Upstream has cut **v0.60.0** since the 2026-05-27 gap analysis [VERIFIED: `git ls-remote --tags upstream` shows `v0.60.0` at `9a05a4ff`]. Also note an EARLIER, unrelated `v0.6.0`/`v0.6.1` tag line (Feb 2026) — do NOT confuse `v0.6.x` (old) with `v0.60.0` (new). The SC locks the range to `v0.57.0..v0.59.0`.
**Why it happens:** "capture any v0.59.x patch releases cut after 2026-05-27" (SC3) is about patch releases *within* v0.59, not the v0.60.0 minor. But v0.60.0's existence is a real scope question.
**How to avoid:** Keep the range `v0.57.0..v0.59.0` as the SC mandates. Surface v0.60.0's existence to the human in Task 1 (and note it in the ledger Headline / a "post-v0.59.0 deferred to UPST8" line, mirroring Phase 47's "strictly silent on post-v0.57.0 per D-47-A4"). The auditor decides whether to expand scope to v0.60.0 — that is a discuss-phase / human decision, not a research assumption. **[ASSUMED]** that the range stays `v0.57.0..v0.59.0` per the locked SC; flag for confirmation if the team wants v0.60.0 folded in.
**Warning signs:** Drift `--to v0.59.0` but commits reference v0.60.0 features; gap-analysis features not appearing because they landed in v0.60.0.

### Pitfall 3: Cluster isolation false-positive (the SC2 / Phase 43 trap)
**What goes wrong:** A `will-sync` cluster is marked clean by name, but its lead commit re-exports a symbol defined in an unabsorbed prerequisite commit → Phase 55 cherry-pick aborts mid-wave.
**Why it happens:** `git diff --name-only` shows file overlap but not symbol provenance.
**How to avoid:** SC2 mandates diff-inspect (`git show <sha>:<file> | grep pub use/mod`) on every will-sync lead commit; flip to `split` on any cross-cluster dep. Document in `## Cross-cluster re-export deps detected`.
**Warning signs:** A cluster touches `crates/nono/src/lib.rs` or `mod.rs` re-export blocks; `pub use` lines reference symbols not introduced in the same cluster.

### Pitfall 4: TLS-interception clean-apply assumption (the SC4 trap)
**What goes wrong:** The v0.59 "endpoint-rules-before-credential-selection" ordering fix is dispositioned `will-sync` and queued for blind cherry-pick — but cherry-picking it deletes the fork's Phase 09/11 Windows credential-injection rewrite (exactly what happened with `9300de9` in Phase 34, which escalated to a D-20 manual replay after 9 conflicted files + 4 modify/delete on files the fork doesn't have).
**Why it happens:** Upstream has a `tls_intercept/` module; the fork does NOT (D-34-B1 fork-preserve). Upstream uses `forward.rs`/`audit_ledger.rs`; the fork uses `route.rs`+`reverse.rs`+`audit_integrity.rs`. [VERIFIED: `find crates -name "*tls*"` returns no tls_intercept module; `34-10-FP-PROXY-TLS-SUMMARY.md` documents the divergence]
**How to avoid (SC4):** Write a dedicated diff-inspect note in the ledger: `git show <v0.59-ordering-commit> -- crates/nono-proxy/` against the fork's `route.rs`/`connect.rs`/`credential.rs`/`reverse.rs`. **Good news for the fork:** `route.rs` ALREADY decouples L7 endpoint filtering from credential injection — its module doc says "a route can enforce endpoint restrictions without injecting any credential ... Credential injection is handled separately" and `RouteStore`/`CredentialStore` are separate keyed stores [VERIFIED: read of `crates/nono-proxy/src/route.rs` lines 1-129]. So the *ordering intent* of the v0.59 fix may already be structurally satisfied by the fork's architecture; the diff-inspect note should confirm whether it's a no-op, a small additive port, or a manual replay. Flag `fork-preserve` or `split` if the upstream commit is entangled with the `tls_intercept/` module the fork doesn't carry.
**Warning signs:** The v0.59 commit touches `tls_intercept/`, `forward.rs`, or `audit_ledger.rs`; cherry-pick produces modify/delete conflicts.

### Pitfall 5: Drift-tool SHA drift (reproducibility pin)
**What goes wrong:** The drift tool was edited since Phase 47, invalidating the `0834aa66...` pin.
**How to avoid:** Task 2 asserts the tool's content SHA equals `0834aa664fbaf4c5e41af5debece292992211559` before running; if mismatch, ABORT and surface to human (D-47-E10). [VERIFIED: `git log -1` SHA on the tool is currently `0834aa664fbaf4c5e41af5debece292992211559` — pin holds]
**Note:** Phase 47 used `sha256sum scripts/check-upstream-drift.sh` for the pin, then recorded the value as `drift_tool_sh_sha`. The Phase 47 ledger frontmatter value `0834aa66...` happens to match the file's last-commit SHA; treat the recorded `drift_tool_sh_sha` as the invariant to assert against (carry it forward verbatim from Phase 47).

## Code Examples

These are git/drift-tool invocations, not application code. All verified against repo state.

### Mechanical preamble: re-fetch + assert tags + lock HEAD (Task 1)
```bash
# Source: 47-01-UPST6-AUDIT-PLAN.md Task 1 (retargeted)
git fetch upstream --tags
git rev-parse upstream/main                 # -> upstream_head_at_audit (40-char SHA)
git rev-parse v0.57.0                        # assert resolves (== 10cec984...)
git rev-parse v0.58.0                        # assert resolves (== 54c4deb6... per ls-remote)
git rev-parse v0.59.0                        # assert resolves (== e61814f8... per ls-remote)
git rev-parse v0.60.0                        # exists remotely (9a05a4ff...) — note for human, NOT in range
```
Expected post-fetch tag SHAs (from `git ls-remote --tags upstream`, pre-fetch) [VERIFIED: ls-remote output]:
- `v0.57.0` → `10cec9845e14db24a50bf8e4a0fdda30c8395359`
- `v0.58.0` → `54c4deb6fbc14ea751b65f73d697d2d6aa191873`
- `v0.59.0` → `e61814f8a70a53346a1e9d0bcf7ba4f52e0e4d1d`
- `v0.60.0` → `9a05a4ff1a4cc8944ccd1da880432b3efe86a051` (post-v0.59, out of locked range)

### Drift tool run (Task 2)
```bash
# Source: 47-.../DIVERGENCE-LEDGER.md Reproduction block (retargeted range)
mkdir -p ci-logs-local/drift
make check-upstream-drift ARGS="--from v0.57.0 --to v0.59.0 --format json" \
  > ci-logs-local/drift/$(date -u +%Y%m%dT%H%M%SZ)-v057-v059.json
# Windows-host fallback if `make` not on PATH:
bash scripts/check-upstream-drift.sh --from v0.57.0 --to v0.59.0 --format json \
  > ci-logs-local/drift/<timestamp>-v057-v059.json
# JSON is gitignored (ci-logs-local/); capture total_unique_commits as the row-count gate target.
```

### Re-export diff-inspect on a will-sync lead commit (Task 5)
```bash
# Source: 47-01-UPST6-AUDIT-PLAN.md Task 5 (verbatim pattern)
git show --stat <lead-commit-sha>
git show <lead-commit-sha>:<file> | grep -nE "^pub use |^pub mod |^extern crate |pub\(crate\) "
# Then trace each re-exported symbol's definition; if defined in ANOTHER cluster
# within v0.57.0..v0.59.0 -> CROSS-CLUSTER DEP -> flip disposition to `split`.
git show <lead-commit-sha> -- <file> | grep '^+pub'   # confirm intra- vs cross-cluster origin
```

### TLS-intercept clean-apply assessment (SC4, new note in Task 4/5)
```bash
# Identify the v0.59 "endpoint rules before credential selection" ordering commit, then:
git show <v0.59-ordering-sha> -- crates/nono-proxy/
# Compare against the fork's already-decoupled surface:
#   crates/nono-proxy/src/route.rs       (RouteStore — L7 endpoint rules, credential-independent)
#   crates/nono-proxy/src/connect.rs     (CONNECT path)
#   crates/nono-proxy/src/credential.rs  (CredentialStore — Phase 09/11 Windows rewrite; preserve byte-identical)
#   crates/nono-proxy/src/reverse.rs     (audit-context call sites)
# Record verdict: clean-apply | small-additive-port | manual-replay(D-20) | fork-preserve
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| 3-disposition vocab (will-sync/fork-preserve/won't-sync) | **4-disposition** vocab adding `split` | v2.5 close (Phase 43 `feedback_cluster_isolation_invalid`) | `split` is a valid disposition for clusters with cross-cluster re-export deps; mechanically-resolvable portion ships now, source migration deferred |
| Re-export check by file-overlap | Diff-inspect symbol provenance (`git show <sha>:<file>`) | Phase 47 (D-47-D1..D4) | SC2 mandate; structural prevention of Phase 43-style mid-wave abort |
| ADR review optional | ADR review MANDATORY with 5-dim L/M/H verdict | Phase 42 (D-42-C4) → Phase 47 (D-47-E8) | Grep-falsifiable; SC1 mandate |
| ≥3 fork-shared files in empirical cross-check | ≥4 files | Phase 47 (D-47-D1, SC#3) | Phase 54 should keep ≥4; preferentially sample fork-divergence hot spots |

**Deprecated/outdated:**
- Treating the drift tool's category labels as sufficient for isolation: superseded by the diff-inspect requirement (the empirical cross-check exists precisely because category labels don't flag fork-divergence hot spots like `profile/mod.rs`'s `From<ProfileDeserialize>` exhaustive match).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The audit range stays `v0.57.0..v0.59.0` per the locked SC, even though upstream has since cut v0.60.0 | Pitfall 2, Summary | If the team wants v0.60.0 folded in, the range/`--to` and `fork_baseline`-forward expectations change; Phase 55 scope shifts. Surface to human in discuss-phase / Task 1. |
| A2 | Exactly one analysis plan (single PLAN.md), `autonomous: false` | Summary, Pattern 1 | If the commit set is far larger than the ~19 estimated, a second plan could help — but Phase 47's 42 commits fit one plan, so risk is low. |
| A3 | The v0.59 TLS-intercept ordering fix's *intent* is likely already satisfied by the fork's decoupled `route.rs`/`CredentialStore` split | Pitfall 4 | The diff-inspect (SC4) is what actually decides this; the note must verify, not assume. If the upstream commit is entangled with `tls_intercept/`, disposition is fork-preserve/split. |
| A4 | The drift tool's `drift_tool_sh_sha` to record is `0834aa664fbaf4c5e41af5debece292992211559` (carried verbatim from Phase 47) | Pitfall 5, Standard Stack | If Phase 47 used a content-hash (sha256sum) that differs from the last-commit SHA, Task 2's assertion form must match Phase 47's exactly. Both happen to be `0834aa66...` here; carry forward verbatim. |
| A5 | v0.58.0/v0.59.0 tag SHAs are `54c4deb6.../e61814f8...` | Code Examples | From pre-fetch `git ls-remote`; confirmed post-fetch in Task 1. Low risk. |

**Note:** The v0.58/v0.59 *feature content* (Bitwarden `bw://`, JSONC, `target_binary`, session hooks, allow_domain path/method, proxy 502, etc.) in the gap analysis is tagged MEDIUM/MEDIUM-HIGH confidence by its own Section 4 and was retrieved from upstream CHANGELOG + Releases page — the drift tool run in Task 2 is what authoritatively enumerates the actual commits. Do not treat the gap-analysis feature list as the commit inventory; use it as a cross-check.

## Open Questions

1. **Does the audit scope expand to v0.60.0?**
   - What we know: SC locks `v0.57.0..v0.59.0`; v0.60.0 exists upstream (`9a05a4ff`), cut after the 2026-05-27 gap analysis.
   - What's unclear: whether the team wants UPST7 to absorb v0.60.0 too, or defer it to UPST8.
   - Recommendation: Keep range at `v0.57.0..v0.59.0` per SC; record v0.60.0 as "deferred to next cycle" in the ledger Headline (mirroring Phase 47's post-v0.57.0 deferral). Surface the choice to the human at discuss-phase or Task 1. Do NOT silently expand.

2. **Are there v0.59.x patch releases (post-2026-05-27)?**
   - What we know: SC3 specifically asks to capture any. Pre-fetch `ls-remote` showed only `v0.59.0` (no `v0.59.1+`), but v0.60.0 landed.
   - What's unclear: settled only by the Task 1 re-fetch.
   - Recommendation: The re-fetch resolves this; if a `v0.59.x` exists, include it (range `v0.57.0..v0.59.x`).

3. **Which exact v0.59 commit is the "endpoint-rules-before-credential-selection" ordering fix?**
   - What we know: gap analysis flags it as `partial/verify`, cross-platform-core, touching the proxy CONNECT route.
   - What's unclear: the precise SHA (gap analysis had no PR numbers).
   - Recommendation: Identify it from the Task 2 drift JSON (proxy category), then run the SC4 diff-inspect.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git | re-fetch, rev-parse, log, show | ✓ | system | — |
| `upstream` remote | drift tool, fetch | ✓ | `https://github.com/always-further/nono.git` [VERIFIED: `git remote -v`] | — |
| `scripts/check-upstream-drift.sh` | commit enumeration | ✓ | content/SHA `0834aa66...` [VERIFIED: `ls -la`] | `.ps1` twin |
| `scripts/check-upstream-drift.ps1` | Windows-host dispatch | ✓ | present [VERIFIED: `ls -la`] | `.sh` via bash |
| `make` | invocation wrapper | conditional on PATH | — | direct `bash scripts/check-upstream-drift.sh` (Windows-host fallback per Phase 47 Task 2) |
| `bash` | run `.sh` on Windows | ✓ | Git Bash / WSL | `.ps1` directly |
| Network access to GitHub | `git fetch upstream --tags` (SC3) | ✓ (assumed; required) | — | none — SC3 is blocking without it |

**Missing dependencies with no fallback:** None — but the **`git fetch upstream --tags` (SC3) requires network access to GitHub at audit-open**; the local upstream is stale at v0.57.0 era and the v0.58/v0.59 tags are not yet local. Without the fetch, the audit cannot run.

**Missing dependencies with fallback:** `make` (use direct `bash scripts/check-upstream-drift.sh` per the Phase 47 Windows-host precedent).

## Validation Architecture

> `.planning/config.json` was not located in this research pass; treating `nyquist_validation` as enabled by default. **However, this is a doc-producing audit phase with zero code/test changes** — there is no test framework to exercise. "Validation" here is grep-falsifiable ledger-structure gates, not unit tests.

### "Test" Framework (falsifiability gates, not code tests)
| Property | Value |
|----------|-------|
| Framework | grep-based structure assertions (per Phase 47 plan `<verify><automated>` blocks) |
| Config file | none — gates live in the PLAN.md task `verify` blocks |
| Quick run command | `grep -c "^### Cluster " DIVERGENCE-LEDGER.md` etc. |
| Full suite command | `make check-upstream-drift ARGS="--from v0.57.0 --to v0.59.0 --format json"` idempotency re-run (D-47-B4 close-gate step 1) + the grep gates below |

### Phase Requirements → Falsifiability Gate Map
| Req/SC | Behavior | Gate Type | Automated Command | Exists? |
|--------|----------|-----------|-------------------|---------|
| SC1 | Ledger has per-cluster dispositions (4-vocab) + windows-touch column | grep | `grep -cE "^\*\*Disposition:\*\* (will-sync|fork-preserve|won't-sync|split)$" DIVERGENCE-LEDGER.md` ≥1 | ✅ pattern from 47 plan |
| SC1 | `## ADR review` with 5-dim L/M/H confirming/revising Option A | grep | `grep -c "^## ADR review$"` ==1 AND `grep -cE "^\| (security\|windows\|maintenance\|divergence\|contributor) "` ≥5 | ✅ |
| SC2 | `## Empirical cross-check` via diff-inspect, ≥4 fork-shared files | grep | `grep -c "^## Empirical cross-check$"` ==1 AND `grep -c "^### File: "` ≥4 | ✅ |
| SC2 | `## Cross-cluster re-export deps detected` summary present | grep | `grep -c "^## Cross-cluster re-export deps detected$"` ==1 | ✅ |
| SC3 | Frontmatter records re-fetch HEAD SHA + date | grep | `grep -qE "^upstream_head_at_audit: [a-f0-9]{40}$"` AND `grep -q "^refetch_date:"` | ✅ (add refetch_date) |
| SC4 | TLS-intercept diff-inspect note present | grep | `grep -iq "tls.intercept" DIVERGENCE-LEDGER.md` AND a `### ` or `**` note referencing route.rs/credential.rs clean-apply vs replay | NEW — add to plan |
| REQ-UPST7-01 | Row count ≥ drift `total_unique_commits` (zero coverage gap) | count | sum commit-rows ≥ JSON `total_unique_commits` | ✅ (D-47-B4 step 2) |
| invariant | Zero `crates/`/`bindings/`/`scripts/`/Makefile edits | git | `git diff --name-only <base>..HEAD -- crates/ bindings/ scripts/ Makefile` empty | ✅ (D-47-E5) |

### Wave 0 Gaps
- [ ] None requiring code/test infra. The only "new" gate vs Phase 47 is the **SC4 TLS-intercept diff-inspect note** — add a grep-falsifiable assertion for it to the plan's verify blocks.
- [ ] Add `refetch_date:` frontmatter field assertion (SC3 makes the date a ledger-frontmatter requirement, where Phase 47 kept it in lock-notes).

## Security Domain

> `security_enforcement` config not located; treating as enabled. This is a doc-producing audit phase — it introduces no runtime code, no new attack surface, and no dependencies. ASVS application categories are largely N/A. The security-relevant content is the **audit's job of routing upstream security fixes into the fork** and not regressing fork-divergent security surfaces.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No auth code touched |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | no (this phase) | The *downstream* Phase 55 cherry-picks must run schema-collision checks; this phase only disposition-flags them |
| V6 Cryptography | no (this phase) | Phase 34 C11 preserved the credential-injection rewrite byte-identical; SC4 note must not propose changes that regress it |
| V14 Configuration | indirect | The ledger's job includes flagging the proxy-502/TLS-intercept ordering security fixes for correct downstream absorption |

### Known Threat Patterns for this phase (audit-integrity oriented)
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Mis-disposition routes a security fix to `won't-sync`, leaving the fork's cross-platform surface unpatched | Information Disclosure / Elevation | ADR review's security cell + per-cluster rationale must justify any non-`will-sync` disposition on a security-relevant commit (the ADR's whole premise is upstream security fixes flowing in on cadence) |
| Cluster-isolation false positive aborts Phase 55 mid-wave, leaving a partially-synced security surface | Tampering | SC2 diff-inspect + `split` disposition (structural prevention) |
| Blind cherry-pick of the TLS-intercept ordering commit deletes the fork's Windows credential-injection rewrite | Tampering / Elevation | SC4 diff-inspect note; preserve `credential.rs` byte-identical (Phase 34 precedent — SHA `c9f25164...` invariant) |

## Sources

### Primary (HIGH confidence — repo-local, this session)
- `.planning/ROADMAP.md` lines 63-72 — Phase 54 Goal + 4 Success Criteria (authoritative scope)
- `.planning/REQUIREMENTS.md` lines 21-23 — REQ-UPST7-01 text
- `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md` — the methodology + output template (frontmatter, clusters, ADR review, empirical cross-check, re-export scan)
- `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-UPST6-AUDIT-PLAN.md` — the 8-task plan template (auto/human split, close-gates)
- `.planning/quick/260527-sgo-upstream-v044-v059-gap-analysis/GAP-ANALYSIS.md` — v0.58/v0.59 starting inventory + 6 phase buckets + Section 4 confidence flags
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-10-FP-PROXY-TLS-SUMMARY.md` — Phase 34 C11 TLS-interception fork-preserve precedent (the `9300de9` D-20 manual-replay escalation)
- `docs/architecture/upstream-parity-strategy.md` — Phase 33 ADR (Option A `continue`, Accepted), Decision Table, Future audit cadence rule
- `scripts/check-upstream-drift.sh` — drift tool source (D-11 filter lines 120-121; category JSON line 285; `--to` auto-detect requires `upstream/main` fetched, lines 106-107)
- `crates/nono-proxy/src/route.rs` lines 1-129 — fork's L7-endpoint/credential decoupling (RouteStore vs CredentialStore) — directly relevant to SC4
- `git remote -v`, `git ls-remote --tags upstream`, `git rev-parse`, `git describe upstream/main` — verified upstream tag SHAs + stale-local-ref state

### Secondary (MEDIUM confidence)
- Memory `feedback_cluster_isolation_invalid` — diff-inspect mandate, `split` as 4th disposition
- Memory `feedback_clippy_cross_target` — cross-target rule (informs the windows-touch column rationale, though this phase ships no code)
- v0.58/v0.59 feature content in GAP-ANALYSIS Section 2 (self-flagged MEDIUM by its Section 4; authoritatively re-enumerated by the Task 2 drift run)

### Tertiary (LOW confidence — to be settled by execution)
- Exact v0.58.0/v0.59.0 commit count and per-commit metadata (settled by the Task 2 drift JSON after the SC3 re-fetch)
- Whether any `v0.59.x` patch release exists (settled by the Task 1 re-fetch)

## Metadata

**Confidence breakdown:**
- Methodology / plan structure: HIGH — direct clone of the verified Phase 47 precedent
- Touchpoints (Phase 33 ADR, Phase 34 C11 TLS-intercept, drift tool, fork proxy surface): HIGH — all read in-session
- Commit landscape (`v0.57.0..v0.59.0` contents): MEDIUM-HIGH — gap analysis + tag SHAs verified; exact commit set pending the mandatory re-fetch
- Scope boundary (v0.60.0 question): MEDIUM — flagged as an open question / assumption for human decision

**Research date:** 2026-06-01
**Valid until:** ~2026-06-15 (the upstream tag landscape is actively moving — v0.60.0 already exists; re-verify the re-fetch state at plan execution).
