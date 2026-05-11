# Phase 33: windows-parity-upstream-0-52-divergence - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-10
**Phase:** 33-windows-parity-upstream-0-52-divergence
**Areas discussed:** Audit invocation & scope, DIVERGENCE-LEDGER schema + location, ADR scoring methodology, G-25-DRIFT-01 + UPST3 placeholder

---

## Audit invocation & scope

### Q1: Drift-tool invocation range (fork-local mirror tags break auto-detect)

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit `--from v0.40.1 --to v0.52.0` | Pin the range explicitly using upstream's tag names. v0.40.1 is the genuine last-synced state from Phase 22 UPST2. Falsifiable + reproducible from the ledger header. Recommended. | ✓ |
| `--from upstream/v0.40.1 --to upstream/v0.52.0` | Same range but qualify with `upstream/` remote prefix. Slightly more verbose. | |
| Delete local v0.41–v0.43 tags first, then auto-detect | Clean up the misleading local tag mirrors. Risk: side effects on other tools/scripts. | |

**User's choice:** Explicit `--from v0.40.1 --to v0.52.0`
**Notes:** Tag names are unambiguous in this context; remote-prefix is unnecessary. Ledger header records the invocation verbatim for reproducibility.

### Q2: Raw drift JSON archival policy

| Option | Description | Selected |
|--------|-------------|----------|
| Commit at `${phase_dir}/33-drift-raw.json` | Phase-local archived artifact. Reproducibility: a reader can re-run the tool and diff. ~10s of KB likely. Falsifiable evidence. | |
| Don't commit — ledger is canonical | Ledger rows ARE the dispositioned view of drift output. Raw JSON is regenerable; committing it bloats the planning dir. Ledger header records the invocation command so it's reproducible. | ✓ |
| Commit BOTH raw JSON + a one-line `drift-command.sh` in phase dir | Belt-and-suspenders. Heaviest option. | |

**User's choice:** Don't commit — ledger is canonical
**Notes:** Ledger header captures the exact invocation + upstream HEAD sha + drift-tool script version. Raw JSON is regenerable; archiving it is bloat.

### Q3: Where do fork-only surfaces (broker, WFP, Sigstore, NONO_TEST_HOME) get enumerated?

| Option | Description | Selected |
|--------|-------------|----------|
| Separate FORK-ONLY-SURFACES.md section in the ADR | ADR has a `## Fork-only surface area` section. Self-contained; reader sees both sides of the divergence ledger when reading the decision. | |
| Second section in DIVERGENCE-LEDGER.md | Ledger has TWO parts: upstream→fork unabsorbed (drift-tool output) and fork-only surface area (manual enum). One artifact, two directions. ADR references both. | |
| Both — enumerated in ledger, summarized in ADR | Ledger has the full fork-only enumeration as its own section; ADR's decision quotes the highlights. Recommended: most discoverable for downstream agents. | ✓ |

**User's choice:** Both — enumerated in ledger, summarized in ADR
**Notes:** Phase 24 D-11 filters fork-only paths out of drift-tool output by design; manual enum lives in ledger so the audit artifact is complete, ADR summary keeps the strategic decision self-contained.

---

## DIVERGENCE-LEDGER schema + location

### Q1: Ledger location — phase-local vs repo-level

| Option | Description | Selected |
|--------|-------------|----------|
| Phase-local: `.planning/phases/33-.../DIVERGENCE-LEDGER.md` | Self-contained per audit cycle. Future UPST4 audit creates its own ledger in its phase dir. Cleaner archival semantics; matches Phase 24's per-phase-artifacts pattern. | ✓ |
| Repo-level: `.planning/DIVERGENCE-LEDGER.md` | Single durable artifact across audit cycles. Each new audit appends a section. Risks staleness. | |
| Phase-local PLUS a repo-level INDEX | Ledger lives in phase dir; thin index lives at repo root. Discoverable + archival. Heaviest. | |

**User's choice:** Phase-local: `.planning/phases/33-.../DIVERGENCE-LEDGER.md`
**Notes:** Phase 33's `DIVERGENCE-LEDGER.md` slots in alongside the standard per-phase artifact set.

### Q2: Row granularity — one row per commit, clustered, or two-tier

| Option | Description | Selected |
|--------|-------------|----------|
| One row per commit | Direct map from drift-tool JSON. Mechanically generable; repetitive when many commits cluster into one logical change. | |
| One row per logical change (clustered) | Maintainer groups commits into logical rows. Easier strategic reasoning; less mechanically verifiable. | |
| Two-tier: commit-rows under cluster headers | Cluster headers with disposition + rationale; nested commit-row tables. Both views in one artifact. Recommended for readability + verifiability. | ✓ |

**User's choice:** Two-tier: commit-rows under cluster headers
**Notes:** Reader sees strategic disposition at a glance via cluster headers; commit-level audit-trail remains in the nested table.

### Q3: Row schema fields

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal: sha + subject + disposition + rationale | Four fields. Drift-tool provides sha + subject; maintainer adds disposition + rationale. Tightest schema. | |
| Standard: + upstream tag + categories + files-changed-count | Adds upstream-tag (which release introduced it), categories from D-05 lookup, delta size indicator. Lets ADR scoring use 'size' as evidence. | ✓ |
| Full: + author + date + assigned-follow-up-phase | Standard fields plus author/date for archival completeness AND a `target-phase` field. Heaviest schema. | |

**User's choice:** Standard: + upstream tag + categories + files-changed-count
**Notes:** Disposition + rationale live at cluster level (per Q2), not per-row. Row schema is per-commit identification only.

---

## ADR scoring methodology

### Q1: Criteria set

| Option | Description | Selected |
|--------|-------------|----------|
| Lock the four interview-named criteria verbatim | Maintenance cost, Security posture, User clarity, Contributor velocity. | |
| Lock those four PLUS 'Fork-only surface preservation cost' | Adds a criterion for how much fork-only Windows work would have to be re-implemented/re-tested per upstream sync. Directly tied to the strategic question. | |
| Lock those four PLUS 'Roadmap optionality' | Adds a criterion for which option keeps the most v2.4+ doors open vs forecloses options (split is structurally hard to reverse). Strategy-aware. | ✓ |

**User's choice:** Lock the four PLUS 'Roadmap optionality'
**Notes:** Five criteria, equal-weighted (see Q3).

### Q2: Scoring scale + derivation

| Option | Description | Selected |
|--------|-------------|----------|
| 1–5 qualitative scale; maintainer scores from audit findings | Each option×criterion gets a 1–5 score using ledger row counts + fork-only surface size as evidence. Score-line rationale beside each cell. | |
| Low / Med / High; maintainer scores from audit findings | Three-tier qualitative scoring (no false precision). Same evidence basis. | ✓ |
| 1–5 quantitative; tied to ledger metrics | Scores derive from explicit formulas. Most rigorous, most rigid. Bias risk: formulas themselves encode a stance. | |

**User's choice:** Low / Med / High; maintainer scores from audit findings
**Notes:** Each cell carries a 1–2 sentence rationale beside the verdict.

### Q3: Weighting

| Option | Description | Selected |
|--------|-------------|----------|
| Equal weight | All criteria contribute equally. Cleanest. Risk: doesn't reflect stated priority. | ✓ |
| Differential, maintainer-set with rationale | Each criterion has explicit weight; ADR documents WHY. Defensible. Higher cognitive load. | |
| Equal weight in the matrix, with a separate 'tiebreaker' paragraph | Score equally, then if two options within 1 point, ADR names tiebreaker. Pragmatic middle. | |

**User's choice:** Equal weight
**Notes:** If options tie, ADR's "Decision" section names tiebreaker explicitly with rationale grounded in PROJECT.md core value (OS-enforced isolation leans security posture).

---

## G-25-DRIFT-01 + UPST3 placeholder

### Q1: UPST3-sync follow-up phase number

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 34, title `UPST3 — Upstream v0.41–v0.52 Sync Execution` | Next sequential number. Locks the slot now. Title flips if ADR picks split or freeze. | |
| Phase 33.1, title `UPST3 — Upstream v0.41–v0.52 Sync Execution` | Decimal numbering signals tight follow-up (Phase 27.1/27.2 pattern). Locks the slot now. | |
| Leave number as `TBD-NN` placeholder | Per SPEC verbatim — plan-phase decides slot. Back-link cites 'the UPST3-sync follow-up phase (TBD-NN)'. Less concrete but matches SPEC. | ✓ |

**User's choice:** Leave number as `TBD-NN` placeholder
**Notes:** Per SPEC, plan-phase decides slot. ROADMAP placeholder uses title `UPST3 — Upstream v0.41–v0.52 Sync Execution` (flips if ADR picks split/freeze).

### Q2: G-25-DRIFT-01 cross-reference back-link detail level

| Option | Description | Selected |
|--------|-------------|----------|
| Two-line back-link | Adds two lines pointing at LEDGER cluster + ADR + closure handoff. Minimal but discoverable. | |
| Full 'Update (Phase 33)' section under the gap | Adds a paragraph spelling out: drift audit summary, ADR decision, explicit closure handoff, audit-walk note if applicable. Richer archival record. | ✓ |
| Update the YAML frontmatter only | Adds `cross_references` + `closure_phase` to metadata block. Programmatic but invisible to casual reader. | |

**User's choice:** Full 'Update (Phase 33)' section under the gap
**Notes:** Four sub-points: drift audit summary, parity-strategy ADR decision, closure handoff, audit-walk note if applicable.

---

## Claude's Discretion

- ADR file template specifics beyond the locked sections (Context, Goals, Non-goals, Decision, Consequences, Alternatives) — match existing ADRs in `docs/architecture/`.
- Ledger header exact wording (must include invocation + upstream HEAD sha + drift-tool script version + date).
- Per-cluster grouping heuristic for the ledger (cluster boundaries are maintainer judgment during the audit walk).
- ROADMAP placeholder content beyond the title (mirror existing TBD-stub phase entries).
- Whether to commit the ADR with `status: Proposed` first then flip to `Accepted` in a follow-up commit, or write `Accepted` from the first commit — both valid.

## Deferred Ideas

- Automated ledger regeneration tooling — could be a Phase 24 extension; manual curation is acceptable for one audit cycle.
- CI-side drift watchdog — Phase 24 D-03 deferred this; lands in v2.5+ once 2+ audits prove the JSON shape.
- Cross-fork divergence documentation for end users (`docs/cli/development/fork-vs-upstream.mdx`).
- DIVERGENCE-LEDGER schema validation (jq or similar CI-side check); defer until 2+ ledgers exist to see if drift is real.
