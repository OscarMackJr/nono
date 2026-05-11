---
phase: 33
phase_name: windows-parity-upstream-0-52-divergence
gathered: 2026-05-10
status: Ready for planning
requirements_locked_via: 33-SPEC.md (5 requirements)
---

# Phase 33: Windows parity with upstream 0.52 features and divergence decision - Context

**Gathered:** 2026-05-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 33 ships two artifacts: a falsifiable inventory of every fork-vs-upstream divergence between v0.41 and v0.52 (`DIVERGENCE-LEDGER.md` with disposition-complete rows) AND a scored strategic ADR (`docs/architecture/upstream-parity-strategy.md`, `status: accepted`) deciding between three named options for the fork's relationship with upstream `always-further/nono` going forward (continue parity / split Windows / freeze-at-v0.52). Sync execution — the actual cherry-picks and manual replays the audit and ADR imply — is OUT of scope by construction; it lands in a follow-up phase (UPST3-sync, slot TBD-NN). The phase also updates Phase 25's open gap G-25-DRIFT-01 with a cross-reference and queues the UPST3-sync placeholder in ROADMAP.

</domain>

<spec_lock>
## Requirements (locked via SPEC.md)

**5 requirements are locked.** See `33-SPEC.md` for full requirements, boundaries, and acceptance criteria.

Downstream agents MUST read `33-SPEC.md` before planning or implementing. Requirements are not duplicated here.

**In scope (from SPEC.md):**
- Run `make check-upstream-drift` against v0.52 upstream tag and produce `DIVERGENCE-LEDGER.md` with full coverage of flagged items.
- Write `docs/architecture/upstream-parity-strategy.md` with scored options, picked option, rationale, `status: accepted`.
- Add the PROJECT.md key-decisions row.
- Cross-reference Phase 25's G-25-DRIFT-01 gap entry to point at this phase's outputs.
- Queue the UPST3-sync follow-up phase placeholder in ROADMAP.md.

**Out of scope (from SPEC.md):**
- Any actual cherry-picks, manual replays, or code changes that close divergences (always a separate phase).
- Closing G-25-DRIFT-01 (the renames are sync work).
- Phase 25 HUMAN-UAT re-validation (blocked until UPST3-sync runs).
- Cross-platform parity sweep beyond the audit.
- Per-row will-sync vs fork-preserve decisions (plan/execute handles; SPEC only requires every row gets one of three dispositions).
- Mock-Fulcio fixture / Phase 32 carry-forward items.

</spec_lock>

<decisions>
## Implementation Decisions

### Audit invocation & scope (Area A)

- **D-33-A1:** **Explicit drift-tool range** — invoke as `make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json"`. The auto-detect logic from Phase 24 D-08 would mis-resolve because the fork carries v0.41.0/v0.42.0/v0.43.0 LOCAL tags (mirrors of upstream tags fetched via `git fetch upstream --tags`, NOT genuine sync points). v0.40.1 is the actual last-synced state from Phase 22 UPST2. The ledger header records this invocation verbatim so a reader can reproduce it.

- **D-33-A2:** **Raw drift JSON is NOT committed.** `DIVERGENCE-LEDGER.md` is the canonical artifact; raw JSON is regenerable from the invocation in the header. Keeps the planning dir lean. The ledger header has the exact command, current upstream HEAD sha at the time of the audit, and `make check-upstream-drift` script version (whatever is checked-in at audit time) so the audit is reproducible.

- **D-33-A3:** **Fork-only surfaces enumerated in BOTH the ledger and the ADR.** The drift tool's D-11 filter (`*_windows.rs` / `exec_strategy_windows/` excluded) means upstream→fork direction is asymmetric: drift tool sees only upstream changes the fork hasn't absorbed; it does NOT see fork code with no upstream analog. Strategic decision requires both directions, so the LEDGER has a separate `## Fork-only surface area` section (manual enum of crates/seams added since v0.40: `crates/nono-shell-broker/`, `crates/nono-wfp-service/`, Phase 27.1 `NONO_TEST_HOME` seam, Phase 28 Authenticode chain-walker `parse_signer_subject`/`parse_thumbprint`, Phase 31 broker dispatch `WindowsTokenArm::BrokerLaunch`, Phase 32 Sigstore TUF cached-root + broker self-trust-anchor), and the ADR's Decision section quotes the highlights as evidence.

### DIVERGENCE-LEDGER schema + location (Area B)

- **D-33-B1:** **Phase-local ledger location** at `.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md`. Matches Phase 24's per-phase-artifacts pattern. Future audits (UPST4 for v0.53+) create their own ledger in their phase dir; no cross-phase append.

- **D-33-B2:** **Two-tier structure** — cluster headers group related commits into logical changes; each cluster has its own disposition + rationale; commit-row table sits nested beneath each cluster header. Example shape:
  ```markdown
  ### Cluster: RESL flag renames (v0.45.0)
  - Disposition: will-sync
  - Rationale: closes G-25-DRIFT-01; user-facing CLI surface must match upstream
  - Target phase: UPST3-sync (TBD-NN)

  | sha | subject | upstream-tag | categories | files-changed |
  |-----|---------|--------------|------------|---------------|
  | abc123 | rename --memory to --mem-limit | v0.45.0 | profile,policy | 4 |
  | def456 | ...                            |  ...         |  ...           | ... |
  ```
  Reader sees strategic disposition AT a glance via cluster headers; commit-level audit-trail remains in the nested table. Both views in one artifact.

- **D-33-B3:** **Standard row schema** = `sha + subject + upstream-tag + categories + files-changed-count`. Categories derive from Phase 24 D-05 lookup table (`profile/policy/package/proxy/audit/other`). `upstream-tag` is the upstream release that introduced the commit (e.g., `v0.45.0`) so the reader sees which release each cluster lives in. `files-changed-count` (integer, no per-file list) lets the ADR scoring use 'size' as evidence without bloating the ledger. Disposition + rationale live at the CLUSTER level, not per-row (per D-33-B2).

### ADR scoring methodology (Area C)

- **D-33-C1:** **Five criteria, equal-weighted:**
  1. **Maintenance cost** — per-sync labor, cherry-pick conflict count, manual-replay rate (D-20 pattern from Phase 26 Plan 26-01).
  2. **Security posture** — Windows-only hardening (broker IL ladder, WFP kernel-enforcement, Authenticode chain-walker, Sigstore broker self-trust-anchor) vs upstream's threat model coverage.
  3. **User clarity** — single-CLI-surface (one tool, one docs URL) vs split-surface confusion (which `nono` am I running?).
  4. **Contributor velocity** — Windows-PR latency, cross-platform-PR latency, PR review burden.
  5. **Roadmap optionality** — which option keeps the most v2.4+ doors open vs forecloses options (split is structurally hard to reverse).

- **D-33-C2:** **Low / Med / High qualitative scoring** — each option×criterion gets one of {Low, Med, High}; no false-precision integer scale. Maintainer scores from audit findings (LEDGER row counts, fork-only surface size as evidence). Each cell carries a 1–2 sentence rationale line beside the verdict.

- **D-33-C3:** **Equal weight, no per-criterion weights.** SPEC.md uses "weighted" loosely — equal weight IS a weight choice, documented as such. If two options tie within one criterion-level (e.g., both score "Med Med High High Med"), the ADR's "Decision" section names the tiebreaker explicitly with rationale grounded in PROJECT.md core value ("OS-enforced isolation, structurally impossible bypass" leans security posture).

- **D-33-C4:** **ADR header convention follows fork standard** — plain-text `**Status:** Accepted` line (not YAML frontmatter) matching `docs/architecture/audit-bundle-target.md` (Phase 27.2 Plan 27.2-03), `docs/architecture/broker-trust-anchor.md` (Phase 32 Plan 32-05), `docs/architecture/sigstore-tuf-cache.md` (Phase 32 Plan 32-05), `docs/architecture/aipc-unix-futures.md` (Phase 25 Plan 25-02). SPEC.md's "frontmatter `status: accepted`" wording was loose — the structural requirement is "discoverable header that grep finds at the top of the file"; plain-text matches the convention.

### G-25-DRIFT-01 + UPST3 placeholder (Area D)

- **D-33-D1:** **UPST3-sync follow-up phase number = TBD-NN** in ROADMAP placeholder. Per SPEC verbatim, plan-phase decides the slot. Back-link from G-25-DRIFT-01 cites "the UPST3-sync follow-up phase (TBD-NN)". Placeholder title in ROADMAP: `UPST3 — Upstream v0.41–v0.52 Sync Execution`. If the ADR picks `split-windows-into-fork` or `freeze-at-v0.52` rather than `continue parity`, the placeholder title flips at plan-phase to match the chosen direction (e.g., `Phase TBD-NN — Windows-fork split execution`).

- **D-33-D2:** **G-25-DRIFT-01 update is a full `**Update (Phase 33, 2026-MM-DD):**` section** appended to the existing gap entry (NOT a frontmatter-only edit). Section contents:
  1. **Drift audit summary:** "Confirmed N commits in upstream v0.4X cluster `RESL flag renames` (see DIVERGENCE-LEDGER.md for full row table)."
  2. **Parity-strategy ADR decision:** "The strategic ADR landed at `docs/architecture/upstream-parity-strategy.md` picked option {A/B/C}: {short option name}. Implication for this gap: {one-line per option — continue means RESL renames sync in UPST3; split means RESL flags stay fork-named with upstream divergence documented; freeze means RESL flags stay frozen at v0.40 verbatim}."
  3. **Closure handoff:** "Gap stays `status: open` until Phase TBD-NN (UPST3-sync follow-up) lands the actual renames. Phase 33 does NOT close G-25-DRIFT-01."
  4. **Audit-walk note** (if applicable): if the audit surfaces RESL-flag-rename commits beyond the 4 originally suspected, note the actual count.

### Claude's Discretion

- **ADR file template specifics** beyond the locked sections (Context, Goals, Non-goals, Decision, Consequences, Alternatives) — match the existing ADRs in `docs/architecture/` for any structural choices not pinned in D-33-C4.
- **Ledger header exact wording** — must include the invocation command (D-33-A1), upstream HEAD sha at audit time, drift-tool script version (commit sha of `scripts/check-upstream-drift.sh` at audit time), and the date. Wording is the planner's call.
- **Per-cluster grouping heuristic for the ledger** — D-33-B2 says cluster related commits, but the cluster boundaries (e.g., "RESL flag renames" vs "RESL flag rename + completion commit + tests") are the maintainer's judgment call during the audit walk.
- **ROADMAP placeholder content beyond the title** — the description, depends-on line, and "Plans: 0 plans / TBD" boilerplate should mirror existing TBD-stub phase entries (Phase 33 itself is a good template; replicate that shape).
- **Whether to commit the ADR with `status: Proposed` first then flip to `Accepted` in a follow-up commit, or write `Accepted` from the first commit** — both are valid; planner picks based on whether the ADR needs review before acceptance.

### Folded Todos

None — no pending todos matched Phase 33 scope.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 33 contracts
- `.planning/phases/33-windows-parity-upstream-0-52-divergence/33-SPEC.md` — **Locked requirements (5 reqs). MUST read before planning.**
- `.planning/REQUIREMENTS.md` — v2.3 milestone requirements (Phase 33 is post-milestone, no REQ-IDs assigned at scope-lock; the milestone context informs strategic priorities)
- `.planning/ROADMAP.md` § Phase 33 (lines 394–409) — Goal, trigger, dependencies, the strategic question, upstream repo reference

### Inputs for the audit
- `.planning/phases/24-parity-drift-prevention/24-CONTEXT.md` — Drift-tool architecture (D-01..D-19), categorization heuristics (D-05), range-detection logic (D-08), fork-only filter (D-11). Phase 33 LEANS on this; downstream agents must understand the tool before invoking it.
- `scripts/check-upstream-drift.sh` + `scripts/check-upstream-drift.ps1` — Twin scripts; the actual tool Phase 33 runs.
- `Makefile` — `check-upstream-drift` target (line 1 of Makefile target list per existing convention)
- `.planning/templates/upstream-sync-quick.md` — Template for the UPST3-sync follow-up phase plan (Phase 33 does NOT use this directly; the placeholder it queues in ROADMAP will reference it)

### Inputs for the strategic decision
- `.planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md` § G-25-DRIFT-01 (lines 62–87) — The originating gap; Phase 33 REQ-4 updates this entry.
- `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` — v0.37.1..v0.40.1 inventory (Phase 22 UPST2's source data); reference for what an "audit cycle" looks like in practice.
- `.planning/quick/260424-upr-review-upstream-037-to-040/PLAN.md` — Companion plan (helps the maintainer judge what cluster grouping looked like at v0.40 audit time)
- `.planning/quick/260428-rsu-...` — The 260428-rsu quick-task deferred runbook for upstream-stack rebase pattern (referenced from Phase 22-05 + ROADMAP § Backlog as "windows-squash → main merge gated on PR-583 maintainer response"). Reference only — Phase 33 doesn't depend on its outcome.

### ADR convention models
- `docs/architecture/audit-bundle-target.md` — Closest convention match (header style, Context/Goals/Non-goals/Decision/Consequences/Alternatives structure). Phase 33 mirrors this shape.
- `docs/architecture/broker-trust-anchor.md` — Phase 32 ADR; reference for the "options + scoring + decision + consequences" pattern.
- `docs/architecture/sigstore-tuf-cache.md` — Phase 32 companion ADR; reference for how cross-cutting decisions are documented.
- `docs/architecture/aipc-unix-futures.md` — Phase 25 Plan 25-02 ADR; reference for the per-option scoring/verdict pattern.

### PROJECT.md integration
- `.planning/PROJECT.md` § Key Decisions (lines 158–183) — 3-column markdown table (`| Decision | Rationale | Outcome |`); Phase 33 REQ-3 adds one row here.
- `.planning/PROJECT.md` § Upstream Parity Process (lines 185–197) — Process Phase 24 added; Phase 33 may extend this section with a `Sync Strategy` subsection citing the new ADR.

### Project standards
- `CLAUDE.md` — Tech stack constraints, security non-negotiables, the byte-identical `crates/nono/` D-19 invariant (which Phase 33 trivially honors — no library changes).
- `.planning/STATE.md` — Current milestone state; Phase 33 closes after v2.3 ship, so Phase 33 may be a v2.4 or v2.3-trailing phase depending on milestone timing (the `phase_dir` slug doesn't reveal milestone; ROADMAP entry says `v2.4`).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`make check-upstream-drift` target** (Phase 24) — already wired into the Makefile; dispatches platform-appropriate twin script. Phase 33 invokes it once for the audit, captures JSON output (per D-33-A1), discards after ledger curation (per D-33-A2).
- **Twin script CLI surface** — `--from <ref> --to <ref> --format <table|json>` flags from Phase 24 D-04/D-09 are battle-tested. Phase 33 uses `--from v0.40.1 --to v0.52.0 --format json` exactly once.
- **JSON schema from Phase 24 D-07** — `{sha, subject, author, date, additions, deletions, files_changed: [...], categories: [...]}` per commit. Maps directly to the ledger row schema in D-33-B3 (drop `author/date/additions/deletions/files_changed` per-file list; keep sha/subject/categories; derive `files-changed-count = len(files_changed)`; add `upstream-tag` via `git describe --tags --contains <sha>`).
- **Existing ADR pattern in `docs/architecture/`** — 4 ADRs (`audit-bundle-target.md`, `broker-trust-anchor.md`, `sigstore-tuf-cache.md`, `aipc-unix-futures.md`) provide structural templates. Phase 33 mirrors the closest match.

### Established Patterns

- **`upstream` git remote** is configured (`https://github.com/always-further/nono.git`); upstream tags v0.37.1 → v0.52.0 are fetched locally. No setup work.
- **Phase-local artifact convention** — Each phase dir owns its own artifacts (PLAN.md, CONTEXT.md, SUMMARY.md, VERIFICATION.md, HUMAN-UAT.md, etc.). Phase 33's `DIVERGENCE-LEDGER.md` slots in alongside this set.
- **ROADMAP § Phase Details placeholder pattern** — Existing TBD-stub entries (e.g., Phase 33 itself before /gsd-spec-phase ran) follow the shape: title, goal/trigger lines, "Requirements: TBD", "Depends on: ...", "Plans: 0 plans". UPST3-sync placeholder follows the same shape.
- **D-19 cherry-pick trailer** — Locked block (Upstream-commit/Upstream-tag/Upstream-author/Co-Authored-By/Signed-off-by × 2). Phase 33 does NOT use this directly (no cherry-picks in this phase); the UPST3-sync follow-up will.

### Integration Points

- `docs/architecture/upstream-parity-strategy.md` — NEW file Phase 33 creates.
- `.planning/phases/33-.../DIVERGENCE-LEDGER.md` — NEW file Phase 33 creates.
- `.planning/PROJECT.md` § Key Decisions — Phase 33 adds one row.
- `.planning/PROJECT.md` § Upstream Parity Process — Phase 33 may extend with a Sync Strategy subsection citing the new ADR (planner decides).
- `.planning/phases/25-.../25-HUMAN-UAT.md` § G-25-DRIFT-01 — Phase 33 appends an "Update (Phase 33, YYYY-MM-DD)" section.
- `.planning/ROADMAP.md` § Phase Details + Progress Table — Phase 33 adds the UPST3-sync placeholder entry AND updates Phase 33's own row to `Complete` after the phase closes.

</code_context>

<specifics>
## Specific Ideas

- **Ledger header MUST record the invocation verbatim**, including upstream HEAD sha at audit time AND the `scripts/check-upstream-drift.sh` commit sha. The drift tool can evolve; capturing tool version makes the audit reproducible against the historical tool, not just the current one.
- **Cluster header convention in the ledger** — use `### Cluster: <name> (introduced in <upstream-tag>)` so the v0.41–v0.52 progression is readable top-to-bottom. Sort clusters by their earliest introducing tag.
- **The strategic ADR should include a "Future audit cadence" subsection in Consequences** — whichever option wins, downstream maintainers need to know whether `make check-upstream-drift` runs every release, every milestone, or only when triggered by a specific gap. This is a downstream-of-decision rule, not a Phase 33 requirement, but the ADR is the right place to document it.
- **`fork-preserve` rationale lines should reference the precedent** — if a cluster is preserved because cherry-pick would delete a fork-only security check (D-20 pattern from Phase 26 Plan 26-01 PKGS-02), the rationale cites that decision. If preserved because the file is `*_windows.rs` (fork-only by definition), the rationale notes that. Each `fork-preserve` should be auditable back to a known principle, not maintainer fiat.
- **`won't-sync` is the catch-all for "upstream churn not relevant to fork"** — e.g., dependabot bumps for crates the fork doesn't use, refactors of files the fork has rewritten, etc. Rationale should be specific ("hyper bump — fork pins hyper at workspace level, upstream version-bump unnecessary") not generic ("not relevant").

</specifics>

<deferred>
## Deferred Ideas

- **Automated ledger regeneration tooling** — A script that ingests drift-tool JSON output and generates a draft `DIVERGENCE-LEDGER.md` skeleton (cluster grouping by upstream-tag, empty disposition columns for maintainer fill-in). Could be a Phase 24 extension. Not Phase 33 scope — manual curation is acceptable for one audit cycle.
- **CI-side drift watchdog** — A periodic GitHub Action that runs `make check-upstream-drift` and posts an issue when drift exceeds N commits. Phase 24 D-03 explicitly deferred this; Phase 33 is the audit that proves the JSON shape out, so CI could land in v2.5+ once we've done 2+ audits.
- **Cross-fork divergence documentation** — A standalone `docs/cli/development/fork-vs-upstream.mdx` for end users (not just maintainers) explaining what the fork does differently. Not Phase 33 scope; the strategic ADR is for maintainers; a user-facing doc is its own work.
- **DIVERGENCE-LEDGER schema validation** — A small `jq` or similar check that ensures every cluster header has a disposition + rationale and every row has the standard schema. Could be CI-side; defer until we have 2+ ledgers to see if drift in shape is a real risk.

### Reviewed Todos (not folded)
None — no pending todos matched Phase 33 scope.

</deferred>

---

*Phase: 33-windows-parity-upstream-0-52-divergence*
*Context gathered: 2026-05-10*
