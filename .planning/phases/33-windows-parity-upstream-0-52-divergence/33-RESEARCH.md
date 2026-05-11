# Phase 33: Windows parity with upstream 0.52 features and divergence decision — Research

**Researched:** 2026-05-10
**Domain:** Upstream-parity audit + strategic ADR (docs/ledger only — no code mutation)
**Confidence:** HIGH on contracts (drift-tool JSON shape, ADR convention, file shapes); MEDIUM on audit-walk size (precise commit counts pending tag fetch); HIGH on PROJECT.md / G-25-DRIFT-01 / ROADMAP shapes.

## Summary

Phase 33 ships two artifacts on top of an immutable codebase: `DIVERGENCE-LEDGER.md` (the falsifiable audit) and `docs/architecture/upstream-parity-strategy.md` (the scored strategic ADR). Five locked requirements (33-SPEC.md REQ-1..5) plus eleven locked decisions (33-CONTEXT.md D-33-A1..A3, B1..B3, C1..C4, D1..D2) constrain almost every shape; the planner's residual freedom is around ledger header wording, per-cluster grouping heuristic, ROADMAP placeholder fill, and the `Proposed → Accepted` vs `Accepted-first-commit` ADR commit pattern.

Three reality-check findings the planner needs:
1. **Drift-tool JSON shape is verified** (script-source-confirmed) — `{sha, subject, author, date, additions, deletions, files_changed:[...], categories:[...]}` per commit; outer envelope has `range / from / to / total_unique_commits / by_category{6 keys} / commits[]`. CONTEXT.md's field list matches script exactly.
2. **Upstream tags v0.43.1..v0.52.0 are NOT fetched locally**. Only v0.40.x, v0.41.0, v0.42.0, v0.43.0 are present in the working tree. `git fetch upstream --tags` is REQUIRED before D-33-A1's invocation can succeed. `upstream/main` is at SHA `34725154` (v0.44.0 territory) — also stale.
3. **PROJECT.md "key-decisions row" target needs clarification**. The 3-column `| Decision | Rationale | Outcome |` table (lines 158-183) is the Key Decisions table the planner appends to. CONTEXT.md / SPEC.md say "matches existing rows for REQ-WRU-01 and SHELL-01" — but those identifiers are bullet entries in the `## Requirements § Validated` list (line 65+), NOT the Key Decisions table. The planner should treat the 3-column Key Decisions table as the canonical target (it's the more-discoverable structural artifact and matches "key-decisions" naming) and write a Decision/Rationale/Outcome row, NOT a REQ-style bullet.

**Primary recommendation:** Plan as 4 sequential waves: (W0) prep — `git fetch upstream --tags`, run drift tool, capture sha-of-tool. (W1) ledger curation — two-tier markdown structure with cluster headers + nested commit tables + manual fork-only enumeration. (W2) ADR — write `docs/architecture/upstream-parity-strategy.md` with locked scoring matrix (3 options × 5 criteria × Low/Med/High). (W3) integration edits — PROJECT.md Key Decisions row + G-25-DRIFT-01 update section + ROADMAP UPST3 placeholder. Each wave is one commit, atomic, reviewable in isolation; final wave triggers `make ci` gate.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Drift inventory generation | Build tooling (Bash/PowerShell) | — | `make check-upstream-drift` is the source of truth (SPEC.md constraint); Phase 33 is read-only consumer |
| Ledger curation | Planning artifacts (Markdown in `.planning/`) | — | Per D-33-B1 phase-local, parallel to PLAN/CONTEXT/SUMMARY pattern |
| Strategic decision documentation | Architecture docs (`docs/architecture/*.md`) | Planning artifact references | Per D-33-C4 + SPEC constraint, follows 4-ADR convention in `docs/architecture/` |
| Gap-tracking integration | Phase artifacts (`25-HUMAN-UAT.md`) | — | Append-only update section per D-33-D2; preserves gap audit trail |
| Roadmap queueing | Project roadmap (`.planning/ROADMAP.md`) | — | Mirrors existing Phase-X-stub placeholder pattern |
| Decision discoverability | Project artifact (`.planning/PROJECT.md` Key Decisions table) | — | Append-only 3-column row; grep-discoverable from project root |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-1 | Drift audit + `DIVERGENCE-LEDGER.md` against v0.52 with 3-value disposition per cluster row | Drift-tool JSON shape verified from `scripts/check-upstream-drift.sh:243-294`; two-tier cluster+row structure spec'd in D-33-B2; row schema in D-33-B3 |
| REQ-2 | Scored strategic ADR at `docs/architecture/upstream-parity-strategy.md` (3 options × ≥4 criteria, picked option, `status: accepted`) | 5 criteria locked in D-33-C1; Low/Med/High scoring in D-33-C2; section structure mirrors `audit-bundle-target.md` per D-33-C4 |
| REQ-3 | PROJECT.md key-decisions row | 3-column `\| Decision \| Rationale \| Outcome \|` table at `PROJECT.md:158-183`; planner appends one row |
| REQ-4 | G-25-DRIFT-01 cross-reference (Update section appended) | Existing gap at `25-HUMAN-UAT.md:62-87` has `## Gaps` → `### G-25-DRIFT-01` → 5 subsections (What/Where/Impact/Why-not-caught/Recommended/Cross-references); D-33-D2 specifies a NEW `**Update (Phase 33, YYYY-MM-DD):**` section appended below the existing structure |
| REQ-5 | UPST3-sync placeholder queued in ROADMAP | Phase-stub pattern verified from existing Phase 33 entry at `ROADMAP.md:394-409`: title-line + Goal + Trigger + Requirements:TBD + Depends-on + Plans:0 + (optional) Reference; planner picks next available number (Phase 34) |

## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-33-A1** — Explicit drift-tool range. Invoke as `make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json"`. Auto-detect would mis-resolve because the fork carries v0.41.0/v0.42.0/v0.43.0 LOCAL tags (mirrors fetched via `git fetch upstream --tags`, NOT genuine sync points). v0.40.1 is the actual last-synced state from Phase 22 UPST2. Ledger header records this invocation verbatim.

**D-33-A2** — Raw drift JSON is NOT committed. `DIVERGENCE-LEDGER.md` is canonical; raw JSON regenerable from the header invocation. Ledger header records: exact command, upstream HEAD sha at audit time, `make check-upstream-drift` script version sha.

**D-33-A3** — Fork-only surfaces enumerated in BOTH ledger and ADR. Drift tool's D-11 filter (`*_windows.rs` / `exec_strategy_windows/` excluded) masks fork-only-new-code. Ledger has a separate `## Fork-only surface area` section enumerating: `crates/nono-shell-broker/`, Phase 27.1 `NONO_TEST_HOME` seam, Phase 28 Authenticode chain-walker (`parse_signer_subject`/`parse_thumbprint`), Phase 31 broker dispatch (`WindowsTokenArm::BrokerLaunch`), Phase 32 Sigstore TUF cached-root + broker self-trust-anchor. (Note: `crates/nono-wfp-service/` is NOT a crate in the workspace as of audit — see "Verification gap" below.) ADR Decision section quotes highlights as evidence.

**D-33-B1** — Phase-local ledger location: `.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md`.

**D-33-B2** — Two-tier structure. Cluster headers group related commits; each cluster has disposition + rationale; commit-row table nested beneath each cluster. Example shape locked in CONTEXT.md.

**D-33-B3** — Standard row schema: `sha + subject + upstream-tag + categories + files-changed-count`. Disposition + rationale at CLUSTER level, not per-row.

**D-33-C1** — Five equal-weighted criteria: maintenance cost, security posture, user clarity, contributor velocity, roadmap optionality.

**D-33-C2** — Low/Med/High qualitative scoring; each cell carries a 1-2 sentence rationale.

**D-33-C3** — Equal weight is the weight choice. Tiebreaker: PROJECT.md core value (security posture leans).

**D-33-C4** — ADR header: plain-text `**Status:** Accepted` line, NOT YAML frontmatter. Matches `audit-bundle-target.md`, `broker-trust-anchor.md`, `sigstore-tuf-cache.md`, `aipc-unix-futures.md`.

**D-33-D1** — UPST3-sync placeholder phase number TBD; planner picks slot. Placeholder title: `UPST3 — Upstream v0.41–v0.52 Sync Execution`. Title flips if ADR picks B/C.

**D-33-D2** — G-25-DRIFT-01 update is a full `**Update (Phase 33, YYYY-MM-DD):**` section appended (NOT frontmatter edit). Subsections: drift audit summary, parity-strategy ADR decision, closure handoff, audit-walk note.

### Claude's Discretion

- ADR file template specifics beyond locked sections (Context, Goals, Non-goals, Decision, Consequences, Alternatives)
- Ledger header exact wording (must include invocation, upstream HEAD sha, drift-tool script sha, date)
- Per-cluster grouping heuristic for the ledger (cluster boundaries are maintainer judgment)
- ROADMAP placeholder content beyond title (mirror existing TBD-stub shape)
- ADR commit pattern: `Proposed → Accepted` two-step OR `Accepted` first-commit (both valid)

### Deferred Ideas (OUT OF SCOPE)

- Automated ledger-regeneration tooling (skeleton from drift JSON)
- CI-side drift watchdog (periodic GHA workflow posting issues)
- Cross-fork divergence user-facing doc (`docs/cli/development/fork-vs-upstream.mdx`)
- `jq`-based ledger schema validation

## Standard Stack

### Core (drift tool)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `scripts/check-upstream-drift.sh` | Phase 24 ship sha (Phase 33 captures at audit time) | Read-only commit walker over `v0.40.1..v0.52.0` with D-11 path filter | Single source of truth for "what diverged" per SPEC.md constraint |
| `make check-upstream-drift` target | as shipped Phase 24 | Platform-aware dispatch to twin script | Convention from Phase 24 D-02; CONTEXT.md D-33-A1 leans on it |

### Supporting (no new dependencies for Phase 33)
| Asset | Purpose | When to Use |
|-------|---------|-------------|
| `docs/architecture/audit-bundle-target.md` | Closest ADR convention match | Mirror its section structure for `upstream-parity-strategy.md` |
| `.planning/templates/upstream-sync-quick.md` | UPST3-sync follow-up scaffold | Referenced from UPST3 placeholder (Phase 33 does NOT use directly) |
| `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` | Previous audit cycle example | Reference for what "audit cycle" looks like + cluster naming heuristic |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Inline drift JSON in ledger | Committed `drift.json` artifact | D-33-A2 rejects — raw JSON is regenerable; ledger is canonical |
| YAML frontmatter `status: accepted` | Plain-text `**Status:** Accepted` line | D-33-C4 rejects — fork convention is plain-text per 4 prior ADRs |
| Per-row disposition | Per-cluster disposition | D-33-B2 picks cluster-level (strategic view) with row-level commit detail nested |

**Installation:**
```bash
# No new tooling — Phase 33 uses what Phase 24 shipped + Markdown editing.
# Prep step (REQUIRED before drift-tool can target v0.52):
git fetch upstream --tags
# Verify result:
git tag --list 'v0.5*' --sort=v:refname  # should show v0.50.0, v0.50.1, v0.51.0, v0.52.0
```

**Version verification (drift tool):**
```bash
# Capture script sha for ledger header at audit time:
git log -1 --format=%H scripts/check-upstream-drift.sh
git log -1 --format=%H scripts/check-upstream-drift.ps1
```
`[VERIFIED: scripts/check-upstream-drift.sh:243-294 read 2026-05-10]` — JSON schema field order, exit codes, and D-11 filter confirmed against source.

## Architecture Patterns

### System Architecture Diagram

```
                       Phase 33 — read-only audit + decision
                       ───────────────────────────────────────

  upstream/main                                                   docs/architecture/
  (v0.40.1..v0.52.0)                                              upstream-parity-strategy.md
        │                                                                  │
        │                                                                  │
        ▼                                                                  ▼
  ┌────────────────────────────┐    ┌─────────────────────┐    ┌────────────────────────┐
  │ git fetch upstream --tags  │───▶│  DIVERGENCE-LEDGER  │───▶│ Strategic ADR          │
  │ (Wave 0 — prep)            │    │  Wave 1             │    │ Wave 2                 │
  └────────────────────────────┘    │  - cluster headers  │    │ - 3 options scored     │
                                    │  - nested commits   │    │ - 5 criteria L/M/H     │
        ┌─────────────────┐         │  - fork-only enum   │    │ - Decision section     │
        │ make check-     │────────▶│  - dispositions     │    │ - status: accepted     │
        │ upstream-drift  │  JSON   │    (3-value enum)   │    └──────────┬─────────────┘
        │ --format json   │         └──────────┬──────────┘               │
        └─────────────────┘                    │                          │
                                               │                          ▼
                                               ▼               ┌─────────────────────────┐
                                    ┌────────────────────────┐ │ Wave 3 integration:     │
                                    │ Wave 3 integration:    │ │ PROJECT.md Key Decisions│
                                    │ G-25-DRIFT-01 Update   │ │ ROADMAP UPST3 stub      │
                                    │ section appended       │ │ (depends on this Wave)  │
                                    └────────────────────────┘ └──────────┬──────────────┘
                                                                          │
                                                                          ▼
                                                                ┌──────────────────┐
                                                                │ make ci          │
                                                                │ (clippy+fmt+test)│
                                                                │ trivial — no     │
                                                                │ code changes     │
                                                                └──────────────────┘

  Boundary: Phase 33 is READ-ONLY against the codebase.
            crates/nono/ byte-identical D-19 invariant holds trivially.
            All artifacts are docs / planning / ROADMAP edits.
            Sync execution (cherry-picks) deferred to UPST3-sync follow-up phase.
```

### Recommended Project Structure (artifacts written by this phase)
```
.planning/phases/33-windows-parity-upstream-0-52-divergence/
├── 33-CONTEXT.md                # Already shipped
├── 33-SPEC.md                   # Already shipped
├── 33-DISCUSSION-LOG.md         # Already shipped
├── 33-RESEARCH.md               # ← This file
├── 33-PLAN.md                   # ← planner output
├── DIVERGENCE-LEDGER.md         # ← REQ-1 deliverable
├── 33-VERIFICATION.md           # ← post-plan output
└── 33-SUMMARY.md                # ← post-plan output

docs/architecture/
└── upstream-parity-strategy.md  # ← REQ-2 deliverable (NEW FILE)

.planning/PROJECT.md             # ← REQ-3 edit (append Key Decisions row)
.planning/ROADMAP.md             # ← REQ-5 edit (append UPST3 stub + flip Phase 33 row to complete)
.planning/phases/25-cross-platform-resl-aipc-unix-design/
└── 25-HUMAN-UAT.md              # ← REQ-4 edit (append Update section)
```

### Pattern 1: Drift-Tool JSON Schema (VERIFIED)
**What:** Exact JSON shape emitted by `check-upstream-drift.sh --format json` (source-confirmed).
**When to use:** Ledger curation — extract sha, subject, files_changed (for count), categories per commit.
**Example (verified shape):**
```json
{
  "range": "v0.40.1..v0.52.0",
  "from": "v0.40.1",
  "to": "v0.52.0",
  "total_unique_commits": <N>,
  "by_category": {
    "profile": <int>,
    "policy":  <int>,
    "package": <int>,
    "proxy":   <int>,
    "audit":   <int>,
    "other":   <int>
  },
  "commits": [
    {
      "sha": "abcd1234...",
      "subject": "feat(profile): ...",
      "author": "Name",
      "date": "2026-04-15T12:34:56+00:00",
      "additions": 123,
      "deletions": 45,
      "files_changed": ["crates/nono-cli/src/profile/mod.rs", ...],
      "categories": ["profile", "policy"]
    }
  ]
}
```
Field order is locked (see `finalize_commit` at `scripts/check-upstream-drift.sh:179-241`). `categories` are deduplicated and ordered: `audit, other, package, policy, profile, proxy` (see L194). `by_category` key order in the outer envelope: `profile, policy, package, proxy, audit, other` (SUMMARY.md narrative order, see L286-288). `[VERIFIED: scripts/check-upstream-drift.sh L228-241 + L285-288]`

### Pattern 2: Drift-Tool Category Lookup (VERIFIED)
**What:** First-match-wins path-prefix categorizer; load-bearing ORDER.
**Verified table** (from `categorize_file()` at `scripts/check-upstream-drift.sh:131-147`):
| Match | Category |
|-------|----------|
| `crates/nono-cli/src/profile/*` OR `profile.rs` OR `data/profile-authoring-guide.md` | `profile` |
| `crates/nono-cli/src/policy.rs` OR `data/policy.json` | `policy` |
| `crates/nono-cli/src/package*` OR `package_cmd.rs` OR `crates/nono/src/package*` | `package` |
| `crates/nono-proxy/*` | `proxy` |
| `crates/nono/src/audit/*` OR `audit_attestation*` OR `crates/nono-cli/src/audit*` | `audit` |
| anything else under filtered paths | `other` |

This MATCHES the CONTEXT.md D-33 citation of "6 categories: profile/policy/package/proxy/audit/other" exactly. `[VERIFIED: scripts/check-upstream-drift.sh L131-147]`

### Pattern 3: D-11 Path Filter (VERIFIED)
**What:** Drift tool reports cross-platform paths only — `*_windows.rs` and `exec_strategy_windows/` are EXCLUDED.
**Confirmed paths** (from `GITLOG_PATHS` array at `scripts/check-upstream-drift.sh:115-122`):
```
crates/nono/src/
crates/nono-cli/src/
crates/nono-proxy/src/
crates/nono/Cargo.toml
:(exclude)*_windows.rs
:(exclude)crates/nono-cli/src/exec_strategy_windows/
```
**Implication for D-33-A3 (fork-only enum):** the filter masks ALL fork-only Windows surface. The "Fork-only surface area" section in the ledger MUST enumerate the items the filter hides because the drift tool will never surface them. `[VERIFIED: scripts/check-upstream-drift.sh L115-122]`

### Pattern 4: ADR Section Structure (VERIFIED across 4 ADRs)
Common structural skeleton observed in all four prior ADRs:
```
# <Title>

**Status:** Accepted
**Date:** YYYY-MM-DD
**Phase:** N (vX.Y description)
**Requirement:** REQ-XXX-NN   ← optional; aipc-unix-futures + audit-bundle-target have this
**Decision IDs:** D-NN-XX..YY ← optional; sigstore-tuf-cache + broker-trust-anchor have this
**Supersedes:** ...            ← optional; audit-bundle-target has this
**Related ADR:** [link]        ← optional; sigstore-tuf-cache + broker-trust-anchor have this

## Context
  (prose: why this decision now)
  ### Goals
  ### Non-goals

## Decision Table          ← shape: option | dimensions | verdict
## Decision                ← prose explaining chosen option
  ### Sub-sections specific to ADR (e.g., "Skip Mechanism", "Frozen Test Fixture")
## Consequences
  ### Positive
  ### Negative
  ### (optional) Backward-compat shim contract
## References
  ### Internal
  ### Source code           ← may omit if no code
  ### Related ADRs          ← cross-link to companion ADRs
```
`[VERIFIED: docs/architecture/{audit-bundle-target.md, broker-trust-anchor.md, sigstore-tuf-cache.md, aipc-unix-futures.md} header pattern + section headers extracted 2026-05-10]`

**Structural variation note:** `aipc-unix-futures.md` has additional sections — `## Per-HandleKind Rationale`, `## Alternate Mechanisms`, `## Reversibility`, `### Glossary`, `### Frequently-asked questions` — but these are domain-specific, not template requirements. The minimum compliance set is: header block + Context (with Goals + Non-goals) + Decision Table + Decision + Consequences (with Positive + Negative) + References.

**Recommended ADR skeleton for `upstream-parity-strategy.md`:**
```
# Upstream Parity Strategy (continue / split-windows / freeze-at-v0.52)

**Status:** Accepted
**Date:** 2026-05-XX
**Phase:** 33 (v2.4 windows-parity-upstream-0-52-divergence)
**Related artifact:** [DIVERGENCE-LEDGER.md](../../.planning/phases/33-.../DIVERGENCE-LEDGER.md)

## Context
  (narrative: 12 minor versions of divergence, fork-only Windows surface size, G-25-DRIFT-01)
  ### Goals
  ### Non-goals (e.g., does NOT execute the sync — UPST3 follow-up does)

## Decision Table
  3 options × 5 criteria (L/M/H) — verdict per row; chosen option starred

## Decision
  Prose for chosen option; tiebreaker rationale per D-33-C3 if applicable

  ### Future audit cadence    ← Specifics suggestion #3 from CONTEXT.md
  ### Fork-only surface area  ← D-33-A3 evidence quote

## Consequences
  ### Positive
  ### Negative
  ### Cadence / commitments

## Alternatives Considered
  Brief restatement of rejected options with reasons (mirrors Decision Table)

## References
  ### Internal (DIVERGENCE-LEDGER, G-25-DRIFT-01, PROJECT.md row)
  ### Related ADRs (audit-bundle-target, broker-trust-anchor, sigstore-tuf-cache, aipc-unix-futures)
```

### Pattern 5: ROADMAP TBD-Stub Phase Entry (VERIFIED)
**Source:** `.planning/ROADMAP.md` § Phase 33 (lines 394-409), the canonical example CONTEXT.md cites.
**Shape:**
```markdown
### Phase NN: <title>

**Goal:** [To be planned] — <one-line problem statement>.

**Trigger:** <upstream event or gap that opens this work>.

**Requirements:** TBD — to be locked at `/gsd-spec-phase NN` / `/gsd-discuss-phase NN`.

**Depends on:** Phase XX (<short rationale>), ...

**Plans:** 0 plans

Plans:
- [ ] TBD (run `/gsd-spec-phase NN` then `/gsd-plan-phase NN`)

**Reference:** <optional: upstream repo URL, dependent artifact paths>
```
`[VERIFIED: .planning/ROADMAP.md:394-409]`

**Phase numbering** — next available slot is **Phase 34**. Current ROADMAP phase ladder ends at Phase 33. The planner writes UPST3-sync as Phase 34 unless cross-phase pressure surfaces a 33.x sub-phase. (Phase numbers 27.1 and 27.2 are INSERTED sub-phases — that pattern is reserved for "carved out of an existing phase" scenarios, which UPST3 is not.)

### Pattern 6: G-25-DRIFT-01 Update-Section Shape (VERIFIED)
**Source:** `.planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md:62-87`.
**Current structure of G-25-DRIFT-01:**
```markdown
### G-25-DRIFT-01 — Upstream parity drift on all 4 RESL flag names (v0.52)
severity: warning
status: open
discovered: 2026-05-10
discovered_in: 25-HUMAN-UAT (test 1 attempt)

**What:** ...
**Where:** ...
**Impact:** ...
**Why not caught earlier:** ...
**Recommended follow-up:** ...
**Cross-references:** ...
```
**Append shape** (per D-33-D2) — add below `**Cross-references:**` block:
```markdown
**Update (Phase 33, 2026-MM-DD):**

1. **Drift audit summary:** "Confirmed N commits in upstream cluster <name> (v0.4X) covering the RESL-flag-rename surface. See `DIVERGENCE-LEDGER.md` for the full row table."
2. **Parity-strategy ADR decision:** "The strategic ADR landed at `docs/architecture/upstream-parity-strategy.md` picked option {A/B/C}: {short option name}. Implication for this gap: {RESL renames will sync in UPST3 / RESL flags stay fork-named with documented divergence / RESL flags stay frozen at v0.40 verbatim}."
3. **Closure handoff:** "Gap stays `status: open` until Phase TBD-NN (UPST3-sync follow-up) lands the actual renames. Phase 33 does NOT close G-25-DRIFT-01."
4. **Audit-walk note** (if applicable): "Audit surfaced N additional RESL-flag-rename commits beyond the 4 originally suspected from Phase 25 HUMAN-UAT; see ledger cluster <name>."
```
**Precedent check:** the existing G-25-DRIFT-01 entry has no prior `Update` section. There are no other gap entries in `25-HUMAN-UAT.md` with `Update` sections to use as precedent — Phase 33 establishes the convention. The shape suggested in CONTEXT.md D-33-D2 is internally consistent and the planner should follow it as-spec'd. `[VERIFIED: 25-HUMAN-UAT.md L60-87]`

### Pattern 7: Previous Audit Cycle Cluster Naming (REFERENCE)
**Source:** `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md`.
**78 non-merge commits over v0.37.1..v0.40.1**, grouped by the maintainer into **5 feature groups**:

| # | Feature group | Upstream LOC | Cluster commit grouping example |
|---|---------------|--------------|----------------------------------|
| 1 | Audit integrity + attestation | ~1,400 | `4f9552ec` → `4ec61c29` → `02ee0bd1` → `7b7815f7` → `0b1822a9` → `6ecade2e` → `9db06336` (7 commits sequenced together) |
| 2 | Package manager / packs | ~1,500 new | `8b46573d`, `55fb42b8`, `71d82cd0`, `088bdad7`, `115b5cfa`, `ec49a7af`, etc. (~10 commits) |
| 3 | OAuth2 proxy credential injection | ~900 | `fbf5c06e` + `9546c879` + `b1ecbc02` + `0c7fb902` + `19a0731f` + `2244dd73` (6 commits) |
| 4 | `override_deny` + `--rollback` fail-closed | ~200 | `5c301e8d` + `b83da813` (2 commits) |
| 5 | Env-var filtering | ~600 | already-ported (5 commits flagged but disposition = won't-sync because shipped) |

Also organized by release tag in the SUMMARY:
- **v0.37.1 → v0.38.0** (24 commits) — Theme: Package manager v1
- **v0.38.0 → v0.39.0** (23 commits) — Theme: OAuth2 + profile polish
- **v0.39.0 → v0.40.0** (28 commits) — Theme: Audit integrity + attestation + reverse-proxy
- **v0.40.0 → v0.40.1** (3 commits) — minor

**Implication for Phase 33 ledger clustering:** the prior audit shows TWO orthogonal grouping axes worked: (a) by feature theme (Audit, Package, OAuth2 — these are the user-facing change semantics) and (b) by release tag (v0.37.1→v0.38.0 etc — temporal). D-33-B2 favors the feature-theme axis for cluster headers; D-33-B3 puts upstream-tag as a row column so the reader can still see temporal locality at the row level. This dual presentation is what the v0.37.1..v0.40.1 audit shipped, and it's the proven shape. `[VERIFIED: .planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md L11-100]`

### Anti-Patterns to Avoid
- **Committing raw drift.json**: D-33-A2 forbids; ledger is canonical, JSON regenerable.
- **Per-row dispositions**: D-33-B2 puts disposition at cluster level. Per-row would balloon the ledger with no strategic-view benefit.
- **YAML frontmatter on the ADR**: D-33-C4 forbids. Fork convention is plain-text `**Status:** Accepted`.
- **Re-litigating disposition decisions in the ADR**: ledger holds dispositions; ADR holds the strategic choice. Don't duplicate.
- **Closing G-25-DRIFT-01 in Phase 33**: SPEC out-of-scope. Gap stays `open` until UPST3-sync lands the renames.
- **Numbered-scoring matrix in the ADR**: D-33-C2 forbids "false-precision integer scale". Low/Med/High only with rationale.
- **Bulk-merge of v0.41-v0.52 implied or recommended**: Phase 33 is audit + decision only. ANY implementation guidance in the ADR is properly scoped to UPST3-sync; Phase 33 does NOT cherry-pick.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Commit walking + categorization | A bash one-liner over `git log` | `make check-upstream-drift ARGS=...` | Phase 24 ships this; D-33-A1 mandates it; consistency with future audits |
| ADR scoring framework | A new templating system | Plain markdown table per D-33-C1 + D-33-C2 | 4 prior ADRs prove the convention; no tooling needed |
| Fork-divergence catalog | A new schema | Plain markdown enumeration in the ledger's `## Fork-only surface area` section | One-shot for this audit; if it becomes recurring (UPST4+), tooling lands then |
| Per-cluster grouping algorithm | A heuristic engine | Maintainer judgment + theme-axis + upstream-tag-axis (Pattern 7 above) | v0.37.1..v0.40.1 audit cycle proved this works; consistency matters more than automation |

**Key insight:** Phase 33 is a one-shot audit + decision artifact set. Any tool the audit reveals is missing (e.g., automated cluster grouping) belongs in Phase 24 extension work or v2.5+ scope, NOT Phase 33.

## Runtime State Inventory

**Skipped** — Phase 33 is a docs/ledger/audit phase. No rename, refactor, migration, or string replacement against runtime state. No databases, no live services, no OS-registered state, no secrets/env-vars, no build artifacts touched.

**Verified by:** SPEC.md constraint "No interactive merges in this phase. Audit is read-only against upstream tags; no `git pull --no-commit` or merge commits land." + "`crates/nono/` byte-identical (D-19 invariant) continues. No library changes ride along with the audit/decision artifacts."

## Common Pitfalls

### Pitfall 1: Stale local upstream tags
**What goes wrong:** Running `make check-upstream-drift ARGS="--to v0.52.0"` without first fetching upstream tags. The tool fails because `v0.44.0`..`v0.52.0` aren't fetched locally.
**Why it happens:** Local repo only has tags v0.40.1, v0.41.0, v0.42.0, v0.43.0. `upstream/main` is at SHA `34725154` (v0.44.0 territory) — also stale.
**How to avoid:** Wave 0 task explicitly runs `git fetch upstream --tags` BEFORE the drift-tool invocation. Validate with `git tag --list 'v0.5*' --sort=v:refname` returning `v0.50.0, v0.50.1, v0.51.0, v0.52.0`.
**Warning signs:** `Error: cannot resolve latest upstream tag` from the script, or "unknown revision" on tag references.
`[VERIFIED: git ls-remote --tags upstream → 79 tags exist remotely; git tag --list 'v0.5*' locally → only v0.5.0 (a v0.5 release, NOT v0.50). git rev-parse v0.52.0 → fatal: unknown revision]`

### Pitfall 2: Ledger header drift between audit time and review time
**What goes wrong:** Ledger header records "current upstream HEAD sha at audit time" but the planner runs the drift tool, commits the ledger days later, and by then `upstream/main` has moved. Header becomes inaccurate.
**Why it happens:** Audit walk + ledger curation are separate cognitive activities; the maintainer may forget to re-capture sha at commit time.
**How to avoid:** Wave 0 (drift tool invocation) AND Wave 1 (ledger curation) happen back-to-back in one task. Header captures sha at Wave 0 time and is treated as historical fact, not "current."
**Warning signs:** Multi-day gap between drift-tool run and ledger commit.

### Pitfall 3: D-11 filter blindness for fork-only surface
**What goes wrong:** The drift tool's `*_windows.rs` + `exec_strategy_windows/` exclusion means the tool will NEVER surface fork-only Windows code. A reader of the ledger could mistakenly conclude "the fork has nothing upstream doesn't" because the audit shows only upstream→fork direction.
**Why it happens:** D-33-A3 anticipates this — the LEDGER must have a manual `## Fork-only surface area` section enumerating what the filter hides. Easy to forget under "the tool tells me what's diverged" instinct.
**How to avoid:** D-33-A3 spec'd surface enumeration — `crates/nono-shell-broker/`, Phase 27.1 `NONO_TEST_HOME` seam, Phase 28 Authenticode chain-walker, Phase 31 broker dispatch, Phase 32 Sigstore TUF cached-root + broker self-trust-anchor. Plus `*_windows.rs` files: `exec_identity_windows.rs`, `learn_windows.rs`, `open_url_runtime_windows.rs`, `pty_proxy_windows.rs`, `session_commands_windows.rs`, `trust_intercept_windows.rs`, `windows_wfp_contract.rs`, and the `exec_strategy_windows/` subdir.
**Warning signs:** Ledger has zero entries that talk about Windows-only surface. If true, the manual fork-only section is missing.

### Pitfall 4: ADR header `status:` form mismatch
**What goes wrong:** Writer uses YAML frontmatter `---\nstatus: accepted\n---` (which SPEC.md mentions in passing) instead of plain-text `**Status:** Accepted` line. Result: `grep -l '^\*\*Status:\*\*' docs/architecture/` won't find the new ADR, breaking the discoverability invariant.
**Why it happens:** SPEC.md REQ-2 line uses the phrase "frontmatter `status: accepted`" — CONTEXT.md D-33-C4 explicitly corrects this to plain-text form, but the SPEC wording is stickier on first read.
**How to avoid:** Follow CONTEXT.md D-33-C4 verbatim. The 4 prior ADRs all use plain-text — Phase 33's ADR matches.
**Warning signs:** ADR doesn't appear in `grep -l '^\*\*Status:\*\*' docs/architecture/*.md`.
`[VERIFIED: all 4 prior ADRs use plain-text Status line at L3]`

### Pitfall 5: "Cluster boundary" judgment paralysis
**What goes wrong:** The maintainer hits 80 commits and freezes on "where do I cut between cluster A and cluster B?" — leading to either too-fine clusters (loses strategic view) or too-coarse clusters (loses commit-level audit trail).
**Why it happens:** D-33-B2 + CONTEXT.md Claude's Discretion says "cluster boundaries are maintainer judgment call." No formal rule.
**How to avoid:** Use the v0.37.1..v0.40.1 audit cycle precedent (Pattern 7): 5 feature-themed clusters covering 78 commits. Aim for "you can describe each cluster as one feature or one upstream PR-equivalent." If a cluster has > ~15 commits or covers > 2 unrelated themes, split it.
**Warning signs:** Either (a) >10 clusters with <5 commits each (too fine), or (b) <3 clusters covering everything (too coarse).

### Pitfall 6: PROJECT.md row destination confusion
**What goes wrong:** SPEC REQ-3 says "PROJECT.md key-decisions row matching shape for REQ-WRU-01 / SHELL-01." But REQ-WRU-01 and SHELL-01 are NOT entries in the Key Decisions table — they're bullets in `## Requirements § Validated` (line 65+) and bullets in `### Active (v2.3)` (line 110+).
**Why it happens:** CONTEXT.md refers to two different sections of PROJECT.md, and SPEC.md inherits the confusion.
**How to avoid:** Append to the 3-column **Key Decisions table** at `PROJECT.md:160-183` (header: `| Decision | Rationale | Outcome |`). This IS what "key-decisions row" naturally means and is the more-discoverable structural artifact. Sample row from the existing table (line 178 — WR-01 reject-stage):
```
| Decision | Rationale | Outcome |
|----------|-----------|---------|
| ... | ... | ... |
| Phase 33 Upstream parity strategy (continue / split / freeze) | <chosen option's rationale, one paragraph> | <one-line outcome with ADR link> |
```
**Warning signs:** New row added to the Requirements bullet-list rather than the Key Decisions table. If the planner does the bullet form by mistake, it's not grep-discoverable as a "decision" and breaks the SPEC acceptance criterion ("PROJECT.md grep finds the row; row's ADR link resolves").
`[VERIFIED: .planning/PROJECT.md L158-183 = Key Decisions 3-column table; L65-101 = Requirements § Validated bullets; L105-112 = Active v2.3 bullets]`

### Pitfall 7: ROADMAP placeholder phase number collision
**What goes wrong:** Planner picks Phase 34 for UPST3-sync, but in the meantime someone has inserted a Phase 33.x or claimed 34. Result: phase-number conflict.
**Why it happens:** D-33-D1 lets the planner pick the slot. ROADMAP can change between research and plan time.
**How to avoid:** Plan-phase task re-checks `grep '^### Phase ' .planning/ROADMAP.md` immediately before writing the placeholder. Highest current number + 1 (or `XX.1` insert if that fits) is the slot.
**Warning signs:** Phase 34 appears multiple times in ROADMAP after the edit.

### Pitfall 8: Forgetting Phase 33 ROW completion in ROADMAP
**What goes wrong:** Phase 33 ships the UPST3 placeholder but forgets to flip Phase 33's own ROADMAP row from in-progress to complete. Result: ROADMAP still shows Phase 33 as the current phase.
**Why it happens:** Two ROADMAP edits required: (a) append UPST3 stub (new content), (b) update Phase 33's existing entry (mark complete). It's easy to do (a) and skip (b).
**How to avoid:** Wave 3 task explicitly lists BOTH edits. Use `grep -n '### Phase 33' .planning/ROADMAP.md` to confirm Phase 33 entry exists and update its status indicator.
**Warning signs:** After phase close, ROADMAP shows Phase 33 with `Plans: 0 plans` instead of `complete`.

## Code Examples

### Verified Drift-Tool Invocation
```bash
# Wave 0 prep (REQUIRED — local upstream tags are stale):
git fetch upstream --tags

# Verify v0.52.0 is now resolvable:
git rev-parse v0.52.0
# expected: 5d15b50e2fbb60de9fdf69379bcaaf5bc1109e59

# Wave 0 audit invocation (D-33-A1 — verbatim):
make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json" > /tmp/drift-v0.52.json

# Capture invocation metadata for ledger header:
echo "Drift tool sha: $(git log -1 --format=%H scripts/check-upstream-drift.sh)"
echo "Upstream HEAD sha at audit: $(git rev-parse upstream/main)"
echo "Date: $(date -Iseconds)"

# Sanity check: by_category sums (verified field order from script L286-288):
jq '.by_category' /tmp/drift-v0.52.json
jq '.total_unique_commits' /tmp/drift-v0.52.json
```
`[VERIFIED: scripts/check-upstream-drift.sh L266-268 git log invocation; L286-294 JSON emission]`

### Verified Ledger Cluster Header Shape
```markdown
### Cluster: RESL flag renames (introduced in v0.45.0)

- **Disposition:** will-sync
- **Rationale:** Closes G-25-DRIFT-01; user-facing CLI surface must match upstream documentation. The 4 RESL flags (`--memory` / `--cpu-percent` / `--max-processes` / `--timeout`) are renamed in upstream v0.45.0; the rename surface is the most user-visible part of the v0.41-v0.52 divergence.
- **Target phase:** UPST3-sync (Phase 34)

| sha | subject | upstream-tag | categories | files-changed |
|-----|---------|--------------|------------|---------------|
| abc123 | feat(cli): rename --memory to --mem-limit | v0.45.0 | profile,policy | 4 |
| def456 | ... | v0.45.0 | profile | 2 |
```
Shape verified against D-33-B2 spec + CONTEXT.md example block. Categories cell uses comma-separated form (matching JSON `categories[]` content).

### Verified ADR Decision Table Shape
```markdown
## Decision Table

| Option | Maint cost | Security posture | User clarity | Contributor velocity | Roadmap optionality | Verdict |
|--------|-----------|------------------|--------------|---------------------|---------------------|---------|
| **A (chosen) — Continue bidirectional parity** | Med — per-sync labor sustains; 78-commit precedent | High — kernel-enforced Windows hardening evolves alongside upstream | High — single CLI surface | Med — Windows PRs go through one repo | High — keeps all v2.4+ doors open | **Accepted** |
| B — Split Windows into nono-windows fork | Low — fork only pulls from upstream periodically | High — Windows-only hardening unchanged | Low — "which `nono` am I running?" confusion | Low — Windows PRs land in separate repo, cross-platform PRs need 2 reviews | Low — structurally hard to reverse | Rejected: <reason> |
| C — Freeze fork at v0.52, stop chasing upstream | Low — zero sync labor | Med — Windows hardening static; upstream security fixes don't flow in | Med — divergence documented but expected | High — fork becomes its own thing | Low — forecloses re-merge | Rejected: <reason> |
```
The chosen option's rationale moves into a `## Decision` prose block below the table; rejected options' reasons summarize in the Verdict cell.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Auto-detect last-synced tag via `git tag --merged HEAD` | Explicit `--from v0.40.1` for Phase 33 | Phase 33 D-33-A1 | Fork carries mirror tags (v0.41/v0.42/v0.43) that are NOT actual sync points — auto-detect would mis-resolve |
| Inline ADR in `.planning/phases/NN/PLAN.md` | Standalone `docs/architecture/*.md` | Phase 25 Plan 25-02 (aipc-unix-futures) onward | ADRs need to be discoverable from project root, not buried in a phase plan |
| YAML frontmatter `status:` on ADRs | Plain-text `**Status:** Accepted` line | Phase 27.2 (audit-bundle-target onward) | Convention chosen for grep-discoverability without YAML parser; locked in D-33-C4 |
| Single ROADMAP entry per upstream sync | Audit + decision phase separate from sync execution phase | Phase 33 (THIS phase) | Audit + ADR are reviewable in one PR; sync execution gets its own scope-boundary |

**Deprecated/outdated:**
- Phase 24 D-08 auto-detect — sound default, but Phase 33 D-33-A1 documents the exception (fork-mirrored tags break the heuristic).
- Bulk-merge upstream into fork — never the fork's convention; per-commit cherry-pick with D-19 trailer block has been canon since Phase 22.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The actual commit count of `v0.40.1..v0.52.0` (filtered) is large enough to need cluster grouping but small enough for one-week audit walk | Summary, Pattern 7 | If commit count > ~400, audit walk becomes multi-week; planner may want to split Phase 33 into 33.a (audit) + 33.b (decision). `[ASSUMED — actual count requires `git fetch upstream --tags` followed by drift-tool run, which Phase 33 itself does; v0.37.1..v0.40.1 baseline was 78 commits; 12 minor versions vs 4 implies ~200-300 range, but this is an estimate, not a count]` |
| A2 | UPST3-sync placeholder phase number = Phase 34 | Pattern 5 | If ROADMAP gains a phase between research and plan-phase time, planner re-checks `grep '^### Phase ' .planning/ROADMAP.md` for next-available slot. Low risk; Pitfall 7 covers. `[ASSUMED — current highest phase is 33 as of 2026-05-10]` |
| A3 | PROJECT.md row target is the 3-column Key Decisions table at L158-183, NOT the Requirements bullets | Pitfall 6 | If CONTEXT.md intent was actually the Requirements bullets, the planner picks the wrong target and the new row doesn't appear "key-decisions"-grep-discoverable. Recommend confirming this at plan-phase. `[ASSUMED — D-33-C1/D-33-D1/SPEC REQ-3 phrasing is ambiguous; 3-column table is the more-natural fit for "key-decisions row" semantics]` |

## Open Questions

1. **PROJECT.md target section: Key Decisions table vs Requirements bullets?**
   - What we know: The 3-column table at L158-183 is titled `## Key Decisions`; the bullets at L65-101 are under `## Requirements § Validated`. CONTEXT.md/SPEC.md reference "REQ-WRU-01 and SHELL-01 row shape" — those identifiers ARE in the bullets, NOT the table.
   - What's unclear: Whether the writers of CONTEXT.md/SPEC.md meant the 3-column Key Decisions table (more semantically apt) or the Requirements-style bullet entries (where REQ-WRU/SHELL-01 actually live).
   - Recommendation: Plan-phase confirms with the user. If still unclear, default to the 3-column Key Decisions table (Pitfall 6).

2. **ADR commit pattern: `Proposed → Accepted` two-step vs `Accepted` first-commit?**
   - What we know: D-33 Claude's Discretion explicitly allows either pattern.
   - What's unclear: Whether the ADR needs review before acceptance.
   - Recommendation: If the planner expects code-review on the ADR PR, use the two-step pattern (commit as `Proposed`, flip to `Accepted` after review-approval in a follow-up commit). If the ADR is treated as "finalized at first commit because Phase 33 doesn't ship until acceptance lands anyway," use one-step. Either is valid; plan-phase picks.

3. **Audit-walk depth: how much commit-level inspection per cluster?**
   - What we know: D-33-B2 puts disposition + rationale at cluster level. Per-row inspection isn't required for disposition assignment.
   - What's unclear: How deep the maintainer dives into each cluster — read every commit subject, or read every diff?
   - Recommendation: Read subject + author + files-changed-count for every commit (free from JSON); read full diff for the lead commit in each cluster (the one introducing the feature) and for any commit whose subject is ambiguous re: disposition. Document this in the ledger header as the inspection methodology.

4. **Audit cadence post-Phase-33?**
   - What we know: Specifics section 3 ("Future audit cadence subsection in Consequences") flags this as ADR content.
   - What's unclear: Per-release? Per-milestone? Triggered by gap?
   - Recommendation: Plan-phase records a tentative cadence in the ADR; opens for revision in UPST3-sync close.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `git` | drift tool, fetch, log walks | ✓ | (Git-for-Windows MSYS bash, working) | — |
| `bash` (MSYS) | `scripts/check-upstream-drift.sh` | ✓ | (Git-for-Windows ships) | PowerShell twin available |
| `make` | `make check-upstream-drift` target | ✓ assumed (used throughout repo) | — | Direct script invocation |
| `upstream` git remote | tag fetch + commit walk | ✓ | configured to `https://github.com/always-further/nono.git` | n/a — required |
| Upstream tags v0.41.0..v0.52.0 | `--from v0.40.1 --to v0.52.0` resolution | ✗ partial | v0.40.1, v0.41.0, v0.42.0, v0.43.0 fetched; v0.43.1..v0.52.0 missing locally | Run `git fetch upstream --tags` (Wave 0 prep task) |
| `jq` (optional) | sanity-check drift JSON | unknown — not blocking | — | Skip JSON sanity check; raw drift JSON is throwaway per D-33-A2 |
| `cargo` / Rust toolchain | `make ci` final gate | ✓ assumed | 1.77+ per CLAUDE.md | n/a — required by SPEC.md acceptance "make ci passes" |

**Missing dependencies with fallback:**
- Upstream tags v0.43.1..v0.52.0 not local — fallback: `git fetch upstream --tags` (Wave 0 first task; takes <30s on broadband).

**Missing dependencies with no fallback:**
- None.

**Verification command** (Wave 0):
```bash
git remote get-url upstream  # must succeed
git fetch upstream --tags --dry-run | grep -E 'v0\.(4[3-9]|5[0-2])' | head  # confirms fetchable
```
`[VERIFIED: git fetch upstream --tags --dry-run on 2026-05-10 shows v0.43.1, v0.44.0..v0.52.0 as new tag refs]`

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | n/a for unit tests — Phase 33 ships docs/ledger only |
| Config file | `.planning/config.json` (workflow.nyquist_validation = true; included per default) |
| Quick run command | `make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json" > /dev/null` (exit 0 = audit reproduces) |
| Full suite command | `make ci` (clippy + fmt + tests; trivial since no code change) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-1 | `make check-upstream-drift --to v0.52.0` exits 0 after ledger lands | smoke | `make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json" > /dev/null` | ✅ (Phase 24 shipped) |
| REQ-1 | Every cluster has a non-empty disposition from `{will-sync, fork-preserve, won't-sync}` enum | structural / grep | `grep -E '^\- \*\*Disposition:\*\* (will-sync\|fork-preserve\|won.t-sync)$' DIVERGENCE-LEDGER.md \| wc -l` (count = cluster count) | ❌ Wave 0 (ledger doesn't exist yet) |
| REQ-1 | Every cluster has a non-empty rationale | structural / grep | `grep -E '^\- \*\*Rationale:\*\* .+$' DIVERGENCE-LEDGER.md \| wc -l` matches cluster count | ❌ Wave 0 |
| REQ-1 | Row count ≥ count of items the drift tool flagged | numeric | Compare commit-row count in ledger vs `jq '.total_unique_commits'` on the drift JSON | ❌ Wave 0 |
| REQ-2 | ADR file exists at `docs/architecture/upstream-parity-strategy.md` | file existence | `test -f docs/architecture/upstream-parity-strategy.md` | ❌ Wave 2 |
| REQ-2 | ADR header has plain-text `**Status:** Accepted` | grep | `grep -l '^\*\*Status:\*\* Accepted' docs/architecture/upstream-parity-strategy.md` | ❌ Wave 2 |
| REQ-2 | Scoring matrix lists all 3 options × ≥4 criteria | structural | Manual or `grep -c 'continue\|split\|freeze' docs/architecture/upstream-parity-strategy.md` ≥ 3 | ❌ Wave 2 |
| REQ-2 | Decision section names chosen option + rationale | structural | `grep -A 5 '^## Decision$' docs/architecture/upstream-parity-strategy.md \| head` | ❌ Wave 2 |
| REQ-3 | PROJECT.md Key Decisions has new row referencing ADR | grep | `grep 'upstream-parity-strategy' .planning/PROJECT.md` finds 1+ match | ❌ Wave 3 |
| REQ-3 | ADR link in row resolves to file | path check | `grep -oE '\[.*\]\(.*upstream-parity-strategy.*\)' .planning/PROJECT.md \| ...` resolves | ❌ Wave 3 |
| REQ-4 | G-25-DRIFT-01 has Update section appended | grep | `grep -A 2 'Update (Phase 33' .planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md` | ❌ Wave 3 |
| REQ-4 | Gap status remains `open` (Phase 33 does NOT close it) | grep | `grep '^status: open' .planning/phases/25-.../25-HUMAN-UAT.md` (in gap context) still present | ✅ (currently open; must remain so) |
| REQ-4 | Update section cross-references DIVERGENCE-LEDGER.md + ADR | grep | `grep -E 'DIVERGENCE-LEDGER.md.*upstream-parity-strategy.md' <update-section>` | ❌ Wave 3 |
| REQ-5 | ROADMAP has new placeholder phase entry for UPST3-sync | grep | `grep -E '### Phase 34.*UPST3' .planning/ROADMAP.md` | ❌ Wave 3 |
| REQ-5 | Placeholder's Depends-on references Phase 33 | grep | `grep -A 10 '### Phase 34' .planning/ROADMAP.md \| grep -E 'Depends on:.*Phase 33'` | ❌ Wave 3 |
| REQ-5 | Phase 33 ROADMAP row flipped to complete state | grep | `grep -A 5 '### Phase 33' .planning/ROADMAP.md` shows complete-status indicators | ❌ Wave 3 |
| (constraint) | `make ci` passes after final commit | full CI | `make ci` (clippy + fmt + tests) | ✅ existing tests don't change |
| (constraint) | `crates/nono/` byte-identical (D-19 invariant) | git diff | `git diff --stat <pre-phase>..<post-phase> -- crates/nono/` shows zero files | ✅ trivially (no code changes) |

### Sampling Rate
- **Per task commit:** `grep`-based structural checks listed in the table above; trivial (~5 seconds per check).
- **Per wave merge:** Full structural pass over the artifacts modified in the wave (ledger structure for W1, ADR structure for W2, multi-file edits for W3).
- **Phase gate (`/gsd-verify-work`):** Full structural sweep + `make ci` + drift-tool reproduction (`make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json"` exits 0).

### Wave 0 Gaps
- [ ] No new test files needed — Phase 33 ships no Rust code.
- [ ] No framework install — drift tool ships from Phase 24.
- [ ] One-time prep task: `git fetch upstream --tags` (required for drift tool to resolve `v0.43.1..v0.52.0`).

*Framework note: Since Phase 33 ships zero `crates/*/src/` changes, the "test framework" is structural grep + drift-tool reproduction. `make ci` runs anyway as a sanity gate for any incidental whitespace damage in the ROADMAP / PROJECT.md edits.*

## Security Domain

> Required by default; Phase 33 is docs-only, so most ASVS categories don't apply, but the analysis is included for completeness.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | n/a — no auth surface in this phase |
| V3 Session Management | no | n/a |
| V4 Access Control | no | n/a |
| V5 Input Validation | partial | Drift tool already validates `--from` / `--to` refs against `^[A-Za-z0-9._/-]+$` (T-24-01 V5 BLOCKING); Phase 33 inherits this fix and uses it verbatim |
| V6 Cryptography | no | n/a — no crypto operations in this phase |

### Known Threat Patterns for Phase 33

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Tampered/incomplete drift inventory leading to missed divergence | Tampering / Repudiation | Ledger header records drift-tool sha + upstream HEAD sha + invocation — audit is reproducible; per D-33-A2 |
| Stale ADR decision (decision rationale wrong as upstream evolves) | Repudiation | ADR includes "Future audit cadence" subsection (Specifics #3); revisitable per ADR convention with Supersedes link |
| Cluster-disposition leakage (a "will-sync" cluster contains a fork-preserve commit) | Tampering | Per-cluster disposition is acceptably coarse for strategic view; UPST3-sync's per-commit cherry-pick re-validates individual commits at execution time |
| Misclassification of fork-only Windows surface as "upstream-not-touched" | Information disclosure (false positive in audit) | D-33-A3 mandates manual `## Fork-only surface area` section; defends against D-11 filter blindness (Pitfall 3) |

**Security posture for Phase 33 itself:** ADR decision impacts the fork's LONG-TERM security posture (continue parity = absorb upstream security fixes; freeze = stuck on v0.52 security state; split = fork-specific hardening evolves independently). The ADR's "Security posture" scoring criterion (D-33-C1 #2) is where this surfaces.

## Project Constraints (from CLAUDE.md)

- **`crates/nono/` byte-identical D-19 invariant** — Phase 33 trivially honors (no code changes).
- **`make ci` passes after every commit** — Phase 33 ships docs/markdown only; CI should be uneventful.
- **No `.unwrap()` / `.expect()`** — n/a (no Rust code).
- **Conventional commit format with DCO sign-off** — applies to every commit Phase 33 writes; existing repo convention.
- **GSD workflow enforcement** — Phase 33 work is happening under `/gsd-plan-phase 33` (integrated) per CLAUDE.md project section; this RESEARCH.md is the input to `/gsd-plan-phase 33` planner.

## Sources

### Primary (HIGH confidence)
- `scripts/check-upstream-drift.sh` (L1-321) — JSON schema, D-11 filter, category lookup, exit semantics. Read 2026-05-10. `[VERIFIED]`
- `docs/architecture/audit-bundle-target.md` — ADR header + section convention. `[VERIFIED]`
- `docs/architecture/broker-trust-anchor.md` — ADR header + scoring-table pattern. `[VERIFIED]`
- `docs/architecture/sigstore-tuf-cache.md` — ADR header + decision-table pattern. `[VERIFIED]`
- `docs/architecture/aipc-unix-futures.md` — ADR header + extended section structure (per-domain rationale, reversibility). `[VERIFIED]`
- `.planning/PROJECT.md` (L65-101 Requirements, L158-183 Key Decisions table) — both candidate row destinations. `[VERIFIED]`
- `.planning/ROADMAP.md` (L394-409 Phase 33 stub example) — TBD-stub shape. `[VERIFIED]`
- `.planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md` (L60-87 G-25-DRIFT-01) — existing gap entry shape. `[VERIFIED]`
- `.planning/phases/24-parity-drift-prevention/24-CONTEXT.md` (D-04 / D-05 / D-08 / D-11) — drift-tool design decisions Phase 33 inherits. `[VERIFIED]`
- `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` — previous audit-cycle precedent (cluster naming, scope structure). `[VERIFIED]`
- `.planning/phases/33-windows-parity-upstream-0-52-divergence/33-CONTEXT.md` — locked decisions D-33-A1..D2. `[VERIFIED]`
- `.planning/phases/33-windows-parity-upstream-0-52-divergence/33-SPEC.md` — locked requirements REQ-1..REQ-5. `[VERIFIED]`

### Secondary (MEDIUM confidence)
- `git ls-remote --tags upstream` output 2026-05-10 — upstream tags v0.41.0..v0.52.0 confirmed available remotely. `[VERIFIED via tool 2026-05-10]`
- `git log v0.40.1..upstream/main` (37 unfiltered commits / 33 D-11-filtered) — partial commit count using current `upstream/main` (v0.44.0-era SHA `34725154`); REAL v0.40.1..v0.52.0 count requires `git fetch upstream --tags` first. `[PARTIAL — needs full fetch]`

### Tertiary (LOW confidence)
- Estimated v0.40.1..v0.52.0 commit count: ~200-300 filtered. Based on 78-commit v0.37.1..v0.40.1 baseline × ~3-4× release count. `[ASSUMED — confirm at Wave 0]`

## Metadata

**Confidence breakdown:**
- Drift-tool contracts (JSON shape, filter, categories): HIGH — source-verified at the script level.
- ADR convention: HIGH — 4-ADR pattern, all 4 cross-checked.
- PROJECT.md target: MEDIUM — Pitfall 6 documents the ambiguity; planner should confirm at plan-phase.
- G-25-DRIFT-01 update shape: HIGH — current shape verified + D-33-D2 spec is internally consistent.
- ROADMAP placeholder shape: HIGH — Phase 33's own stub is the template.
- Audit walk size: MEDIUM — depends on `git fetch upstream --tags` outcome, which Phase 33 itself produces.
- Common pitfalls: HIGH — 7 of 8 are directly evidenced from CONTEXT.md / SPEC.md / source; only Pitfall 5 (cluster boundary judgment) is judgment-call inherent.

**Research date:** 2026-05-10
**Valid until:** 2026-06-10 (30 days for stable shape research); REVALIDATE if upstream ships v0.53+ before Phase 33 plan-phase runs, or if `.planning/PROJECT.md` Key Decisions table structure changes.

## RESEARCH COMPLETE

**Phase:** 33 — windows-parity-upstream-0-52-divergence
**Confidence:** HIGH on shapes/contracts; MEDIUM on commit-count estimate (resolves at Wave 0)

### Key Findings
- Drift-tool JSON shape verified at the script source — exact field list, order, and category enum confirmed. CONTEXT.md's claims about the tool match the actual implementation.
- Upstream tags v0.43.1..v0.52.0 are NOT fetched locally. `git fetch upstream --tags` is REQUIRED as a Wave 0 prep task before D-33-A1's invocation can succeed.
- ADR convention is plain-text `**Status:** Accepted` line — confirmed across all 4 prior ADRs; D-33-C4 is consistent with reality.
- PROJECT.md "key-decisions row" target is the 3-column Key Decisions table at L158-183; the SPEC/CONTEXT reference to "REQ-WRU-01 / SHELL-01" identifiers points at a DIFFERENT section (Requirements bullets). Planner should append to the table, not the bullets. Pitfall 6 covers.
- ROADMAP placeholder shape: 6-7 line stub mirroring the current Phase 33 stub itself. Next slot is Phase 34.
- The v0.37.1..v0.40.1 audit precedent (78 commits, 5 themed clusters) provides a proven cluster-grouping heuristic — theme axis for cluster headers, upstream-tag axis at row level. The planner can lift the dual-axis pattern.
- D-11 filter blindness is the highest-risk audit pitfall — the drift tool will not surface fork-only Windows surface; D-33-A3's manual enumeration in the ledger's `## Fork-only surface area` section is the only defense.

### File Created
`.planning/phases/33-windows-parity-upstream-0-52-divergence/33-RESEARCH.md`

### Confidence Assessment
| Area | Level | Reason |
|------|-------|--------|
| Drift-tool JSON schema | HIGH | Source-verified at script line level |
| Category lookup table | HIGH | Source-verified — 6 categories match CONTEXT.md exactly |
| D-11 filter scope | HIGH | Source-verified path globs |
| ADR convention | HIGH | 4 prior ADRs cross-checked |
| G-25-DRIFT-01 entry shape | HIGH | Existing gap entry inspected; append target unambiguous |
| ROADMAP placeholder | HIGH | Phase 33 itself is the template; next slot = 34 |
| PROJECT.md target section | MEDIUM | Pitfall 6 — ambiguity in CONTEXT/SPEC wording; planner confirms |
| v0.40.1..v0.52.0 commit count | MEDIUM | Local tags incomplete; Wave 0 fetch resolves |
| Cluster-grouping heuristic | MEDIUM | Maintainer judgment; v0.37.1..v0.40.1 precedent is guideline, not rule |
| Audit walk depth (per-commit inspection) | LOW | Open question for plan-phase |

### Open Questions
1. Confirm PROJECT.md target: 3-column Key Decisions table at L158-183 (recommended) vs Requirements bullet entries.
2. ADR commit pattern: `Proposed → Accepted` two-step or `Accepted` first-commit.
3. Audit-walk methodology (per-commit subject-only vs lead-commit-diff).
4. Future audit cadence (post-Phase-33) — record in the ADR Consequences.

### Ready for Planning
Research complete. Planner can produce PLAN.md splitting the work into 4 sequential waves (W0 prep / W1 ledger curation / W2 ADR / W3 integration edits) with grep-checkable validation per acceptance criterion. The 4 open questions are surface-level only; none block plan-phase.
