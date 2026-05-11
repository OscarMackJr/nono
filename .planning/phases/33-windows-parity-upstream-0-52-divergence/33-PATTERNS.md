# Phase 33: Windows parity with upstream 0.52 features and divergence decision — Pattern Map

**Mapped:** 2026-05-10
**Files analyzed:** 5 (2 new, 3 modified)
**Analogs found:** 5 / 5
**Phase character:** Docs/audit only — no Rust code, no FFI, no library mutation. Five Markdown deliverables across `.planning/` and `docs/architecture/`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md` | new audit artifact | transform (drift JSON → curated two-tier Markdown) | `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` | role-match (audit precedent; ledger schema is novel per D-33-B2) |
| `docs/architecture/upstream-parity-strategy.md` | new ADR (scored decision) | request-response (problem → scored options → decision) | `docs/architecture/audit-bundle-target.md` | exact (closest of 4 ADRs by header convention + decision-table shape) |
| `.planning/PROJECT.md` (Key Decisions table append) | new row in existing 3-col table | append-only | `.planning/PROJECT.md` § Key Decisions (L160–183) | exact (verbatim shape — appending to the same table) |
| `.planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md` (Update section append) | append section to G-25-DRIFT-01 entry | append-only | `25-HUMAN-UAT.md` § G-25-DRIFT-01 (L62–87) | exact target; no prior `Update` precedent — Phase 33 establishes the convention per D-33-D2 |
| `.planning/ROADMAP.md` (UPST3-sync placeholder entry) | new phase stub | append-only | `.planning/ROADMAP.md` § Phase 33 stub (L394–409) | exact (the existing Phase 33 stub IS the template per D-33-D1) |

## Pattern Assignments

---

### `.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md` (new audit artifact)

**Analog:** `.planning/quick/260424-upr-review-upstream-037-to-040/SUMMARY.md` (the v0.37.1..v0.40.1 audit precedent — Phase 22 UPST2's source data)

**Header pattern** (lines 1–13 of analog — YAML frontmatter + headline + headline-impact table):
```markdown
---
slug: upr-review-upstream-037-to-040
status: complete
type: research-only
date: 2026-04-24
range: v0.37.1..v0.40.1
tag_head_upstream: v0.40.1 (79154fe0)
fork_baseline: v2.1 shipped (windows-squash) — UPST-01..04 = v0.37.1 parity
---

# Upstream v0.37.1 → v0.40.1 review — Windows-native impact

## Headline

**78 non-merge commits, ~9k insertions / ~600 deletions.** Zero `*_windows.rs` files touched upstream. All impact flows in through cross-platform files the fork has also modified — so the risk is merge conflicts and missing parity in Windows paths, not direct Windows regressions.
```

**Adaptation for Phase 33:**
- Keep YAML frontmatter (slug, status, type, date, range, tag_head_upstream, fork_baseline) — proven shape.
- Per D-33-A1/A2, header MUST also record verbatim invocation `make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json"`, upstream HEAD sha at audit time, AND `git log -1 --format=%H scripts/check-upstream-drift.sh` script-version sha. Add these as additional frontmatter fields OR as a "Reproduction" subsection under the headline.
- Range becomes `v0.40.1..v0.52.0`; date is 2026-05-XX (audit run date).

**Theme-cluster pattern** (lines 17–25 of analog — the impact-priority table):
```markdown
Five feature groups dominate. In priority order for Windows follow-up:

| # | Feature group                               | Upstream LOC | Windows impact                                                                              |
|---|---------------------------------------------|--------------|---------------------------------------------------------------------------------------------|
| 1 | Audit integrity + attestation               | ~1,400       | **High.** Supervisor event recording + DSSE/in-toto signing; Windows supervisor must match. |
| 2 | Package manager / packs                     | ~1,500 new   | **Medium.** New subcommands (pull/remove/search/list) + hook registration; Windows CI gap.  |
| 3 | OAuth2 proxy credential injection           | ~900         | **Medium.** `nono-proxy` is cross-platform; Windows proxy-cred flow affected.               |
```

**Adaptation:** Replace "Windows impact" column with `Disposition` (`will-sync` / `fork-preserve` / `won't-sync` per REQ-1 enum). This becomes the strategic-view summary at the top of the ledger; per-cluster details follow.

**Per-cluster header + nested commit-table pattern** — synthesize from analog L29–96 (release-tagged commit lists) PLUS the D-33-B2 example block (CONTEXT L60–71). Verbatim cluster shape locked by RESEARCH Pattern 1 + Code Example "Verified Ledger Cluster Header Shape" (RESEARCH L502–514):
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

**Adaptation notes:**
- Cluster header MUST use `### Cluster: <name> (introduced in <upstream-tag>)` form per RESEARCH Specifics #2 — sortable top-to-bottom by introducing tag.
- `categories` cell uses comma-separated form matching the drift-tool JSON `categories[]` array content (RESEARCH Pattern 2 verified — values come from the lookup `audit, other, package, policy, profile, proxy`).
- `files-changed` is an integer count, NOT a per-file list (D-33-B3).
- Disposition + rationale + target-phase at CLUSTER level only, never per-row (D-33-B2).
- Cluster grouping heuristic: maintainer judgment, but the analog proved 5 themed clusters covering 78 commits works (RESEARCH Pattern 7); aim for "one feature theme per cluster" with 2–15 commits each.

**Fork-only surface section** — no analog (this section is novel per D-33-A3, defense against D-11 filter blindness). Skeleton:
```markdown
## Fork-only surface area

Surface added since v0.40.1 with NO upstream analog. The drift tool's D-11 filter (`*_windows.rs` + `exec_strategy_windows/` excluded) hides ALL of this from the audit walk; this section is the manual enumeration.

- `crates/nono-shell-broker/` — Phase 31 Low-IL broker process (Windows-only by design)
- Phase 27.1 `NONO_TEST_HOME` seam — `crates/nono-cli/src/cli_bootstrap.rs`
- Phase 28 Authenticode chain-walker — `parse_signer_subject` / `parse_thumbprint` in `crates/nono-cli/src/exec_strategy_windows/...`
- Phase 31 broker dispatch — `WindowsTokenArm::BrokerLaunch` in `crates/nono-cli/src/exec_strategy_windows/launch.rs:1246-1438`
- Phase 32 Sigstore TUF cached-root — `crates/nono/src/trust/bundle.rs::load_production_trusted_root` (cross-platform per D-32-15 but introduced post-v0.40)
- Phase 32 broker self-trust-anchor — verify gate at `launch.rs:1246+` (Windows-only)
- `*_windows.rs` files: `exec_identity_windows.rs`, `learn_windows.rs`, `open_url_runtime_windows.rs`, `pty_proxy_windows.rs`, `session_commands_windows.rs`, `trust_intercept_windows.rs`, `windows_wfp_contract.rs`
```

**Adaptation notes:**
- Per CONTEXT D-33-A3 + RESEARCH Pitfall 3: planner MUST NOT omit this section. If the cluster table has zero "Windows-only" mentions, this section is the only place fork-only surface is visible.
- Note: CONTEXT D-33-A3 mentions `crates/nono-wfp-service/` but RESEARCH flagged this is NOT a crate in the workspace as of audit time — planner SHOULD verify against `Cargo.toml` and omit if absent.

---

### `docs/architecture/upstream-parity-strategy.md` (new ADR — scored strategic decision)

**Analog:** `docs/architecture/audit-bundle-target.md` (Phase 27.2 ADR — closest convention match per CONTEXT D-33-C4 + RESEARCH Pattern 4)

**Header pattern** (lines 1–7 of analog — plain-text header block, NOT YAML frontmatter):
```markdown
# Audit Bundle Target

**Status:** Accepted
**Date:** 2026-05-05
**Phase:** 27.2 (v2.3 Audit-Attestation Test Re-Enablement)
**Requirement:** REQ-AAHX-02
**Supersedes:** Plan 22-05a Decision 5 ("for backward compatibility" dual-location rationale)
```

**Adaptation for Phase 33:**
```markdown
# Upstream Parity Strategy (continue / split-windows / freeze-at-v0.52)

**Status:** Accepted
**Date:** 2026-05-XX
**Phase:** 33 (v2.4 windows-parity-upstream-0-52-divergence)
**Related artifact:** [DIVERGENCE-LEDGER.md](../../.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md)
```

**Adaptation notes:**
- **CRITICAL — D-33-C4:** Plain-text `**Status:** Accepted` line at L3, NOT YAML frontmatter. SPEC.md says "frontmatter `status: accepted`" but D-33-C4 corrects this; all 4 prior ADRs use plain-text. RESEARCH Pitfall 4 documents this gotcha.
- `Requirement:` field is optional; aipc-unix-futures + audit-bundle-target have it (`REQ-AIPC-NIX-01`, `REQ-AAHX-02`), broker-trust-anchor + sigstore-tuf-cache use `Decision IDs:` instead. Phase 33 has no formal REQ-ID at scope-lock (post-milestone phase) — use `Decision IDs: D-33-A1..A3, B1..B3, C1..C4, D1..D2` if wanting traceability, OR omit and link to SPEC.md / CONTEXT.md inline. Planner's call.
- `Related ADR:` field appears on `broker-trust-anchor.md` + `sigstore-tuf-cache.md` (cross-links between Phase 32's two ADRs). Phase 33's ADR is solo; replace with `Related artifact:` pointing to DIVERGENCE-LEDGER.md.

**Context section pattern** (analog L9–17 — prose narrative motivating the decision):
```markdown
## Context

Phase 22-05a Decision 5 (v2.2) split audit-only sessions to `<audit_root>/<id>/` "for namespace separation" while keeping rollback-active sessions writing the audit attestation bundle to `<rollback_root>/<id>/audit-attestation.bundle`. The split was a partial migration: audit-only flows landed in the new namespace; `--rollback` flows did not. The disposition was recorded as "for backward compatibility" — meaning, in practice, that v2.2 deferred the question of whether the dual-location shape was the desired endpoint.

[... 2 more prose paragraphs telling the story up to the decision point ...]

This ADR finishes the namespace-separation migration started in v2.2: [...one-sentence summary of the chosen path...].
```

**Adaptation:** Phase 33 Context narrates: (1) last upstream sync was Phase 22 UPST2 (v0.38–v0.40, 2026-04-28); (2) v0.41–v0.52 = 12 minor releases of unabsorbed divergence; (3) G-25-DRIFT-01 surfaced the RESL-flag tip of this; (4) fork has accumulated Windows-only surface (enumerate per D-33-A3); (5) this ADR resolves whether continued parity is sustainable.

**Goals / Non-goals pattern** (analog L19–35 — bulleted commits + explicit non-commits):
```markdown
### Goals

This ADR commits to:

- One canonical bundle location independent of `--rollback` flag state.
- Audit attestation bundle outlives rollback cleanup (`rm -rf ~/.nono/rollbacks/<id>/` no longer destroys it).
- Backward-compatible verification of bundles written by older `nono` versions, with a one-shot deprecation warning so operators can re-sign at their leisure.
- A documented v2.5 milestone for hard-cutover removal of the back-compat shim.

### Non-goals

This ADR explicitly does NOT commit to:

- Migrating existing on-disk bundles. Verification still works via the shim; re-signing is operator-driven.
- Changing `cmd_verify`'s JSON output schema in this phase. [...]
```

**Adaptation:** Goals include: a 3-option strategic verdict; locked criteria; per-option rationale capture; future-audit-cadence subsection. Non-goals MUST list: "Executing any cherry-picks (UPST3-sync follow-up does that)"; "Closing G-25-DRIFT-01 (still open after Phase 33)"; "Touching `crates/nono/` (D-19 invariant holds trivially)"; "Per-row will-sync vs fork-preserve dispositions (ledger handles)".

**Decision Table pattern** (analog L37–45 — option | dimensions | verdict):
```markdown
## Decision Table

| Option | Bundle Target on `--rollback` | Verification Path | Verdict |
|--------|-------------------------------|-------------------|---------|
| **A (chosen)** | `<audit_root>/<id>/audit-attestation.bundle` always | Audit-first lookup; rollback-root fallback shim with one-shot `tracing::warn!` until v2.5 | **Accepted** |
| B | `<rollback_root>/<id>/...` when `--rollback`; `<audit_root>/<id>/...` otherwise; verify learns dual-root | Permanent dual-root | Rejected: codifies the dual-location complexity instead of finishing the namespace migration. Permanent verifier complexity for no security gain. |
| C | Test rewrite — accept current production behavior (bundle at `<rollback>/<id>/` when rollback-active) | Unchanged | Rejected: codifies the non-repudiation hole. Test name `rollback_signed_session_verifies_from_audit_dir_bundle` would have to be renamed to assert wrong intent. |
```

**Adaptation for Phase 33** (verbatim shape from RESEARCH L520–525 — locked by D-33-C1/C2/C3):
```markdown
## Decision Table

| Option | Maint cost | Security posture | User clarity | Contributor velocity | Roadmap optionality | Verdict |
|--------|-----------|------------------|--------------|---------------------|---------------------|---------|
| **A (chosen) — Continue bidirectional parity** | Med — per-sync labor sustains; 78-commit precedent | High — kernel-enforced Windows hardening evolves alongside upstream | High — single CLI surface | Med — Windows PRs go through one repo | High — keeps all v2.4+ doors open | **Accepted** |
| B — Split Windows into nono-windows fork | Low — fork only pulls from upstream periodically | High — Windows-only hardening unchanged | Low — "which `nono` am I running?" confusion | Low — Windows PRs land in separate repo, cross-platform PRs need 2 reviews | Low — structurally hard to reverse | Rejected: <reason> |
| C — Freeze fork at v0.52, stop chasing upstream | Low — zero sync labor | Med — Windows hardening static; upstream security fixes don't flow in | Med — divergence documented but expected | High — fork becomes its own thing | Low — forecloses re-merge | Rejected: <reason> |
```

**Adaptation notes:**
- **CRITICAL — D-33-C2:** Each cell carries Low/Med/High verdict + 1-2 sentence rationale. NO numeric scoring (RESEARCH Anti-Pattern: "false-precision integer scale").
- **D-33-C1:** Exactly 5 criteria, no more, no less. Order matters per D-33-C3 tiebreaker (security-posture-leans).
- **D-33-C3:** If two options tie on aggregate Low/Med/High shape, named tiebreaker = PROJECT.md core value ("OS-enforced isolation, structurally impossible bypass") favors security-posture column.
- "Verdict" cell for chosen = `**Accepted**`; rejected = `Rejected: <one-sentence reason>` (mirror analog L42–43 verbatim wording).

**Decision + Consequences + Alternatives** (analog L47–73 — prose + structured sub-sections):
- `## Decision` — prose for chosen option; mirrors analog L47–51.
- `## Consequences` with `### Positive` + `### Negative` sub-sections (analog L53–66).
- Phase-33-specific addition per CONTEXT Specifics #3: `### Future audit cadence` sub-section under Consequences ("whichever option wins, downstream maintainers need to know whether `make check-upstream-drift` runs every release, every milestone, or only when triggered by a specific gap").
- Per D-33-A3: `### Fork-only surface area` sub-section under Decision (quotes the ledger's enumeration as evidence supporting the chosen option).
- `## Alternatives Considered` — brief restatement of rejected options with reasons (verdict cells were one-liners; this section gets 1 short paragraph each).

**References section** (analog L75–93):
```markdown
## References

### Internal

- `.planning/REQUIREMENTS.md` § AAHX (REQ-AAHX-02 acceptance criteria)
- `.planning/phases/27.2-audit-attestation-test-re-enablement/27.2-CONTEXT.md` (decisions D-27.2-01, D-27.2-02, D-27.2-03, D-27.2-04, D-27.2-06)
[...]

### Source code

- `crates/nono-cli/src/audit_attestation.rs:155` (bundle write site: [...])
[...]

### Related ADRs

- `docs/architecture/aipc-unix-futures.md` (Phase 25-02 ADR convention reference; this ADR mirrors its structure)
```

**Adaptation:** Internal references = `33-SPEC.md`, `33-CONTEXT.md` D-33-A1..D2, `DIVERGENCE-LEDGER.md`, `25-HUMAN-UAT.md` G-25-DRIFT-01, `PROJECT.md` Key Decisions row. Source code = OMIT (no code changes in Phase 33). Related ADRs = the 4 prior ADRs (`audit-bundle-target.md`, `aipc-unix-futures.md`, `broker-trust-anchor.md`, `sigstore-tuf-cache.md`) as convention references.

---

### `.planning/PROJECT.md` — Key Decisions table append (new row)

**Analog:** `.planning/PROJECT.md` § Key Decisions (lines 158–183) — the SAME file being modified; the existing table IS the analog

**Header + first few rows** (lines 158–164 verbatim):
```markdown
## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Supervisor Parity as Priority | Essential for "attach/detach" workflow used by long-running agents. | ✔ Good — attach/detach/ps/stop shipped in v1.0; v2.0 extended with `nono shell`, `nono wrap`, session commands; v2.1 added live-stream attach on detached path (Phase 17) |
| WFP over Temporary Firewall | Kernel-level enforcement is the "nono way"; temporary rules are a stopgap. | ✔ Complete — Phase 06 wired SID end-to-end, removed driver gate, cleaned duplicate activation path |
| Intentional `shell`/`wrap` omission | Lack of credible enforcement model on Windows; avoiding security over-claims. | ↶ Reversed in v2.0 — both now shipped with Job Object + WFP + ConPTY enforcement |
```

**Recent precedent row** (line 183 — most-recent Phase 32 entry showing "(chosen-option link)" pattern):
```markdown
| Phase 22 audit-integrity verification upgraded to cryptographic DSSE (HG-01-H, commit cffb43b1) | Initial 22-05a Plan implementation only verified the *structural shape* of the `audit-attestation.bundle` [...]. HG-01-H reviewer caught that this would silently accept a forged bundle whose signature was wrong. Cryptographic DSSE verification fail-closed on any signature mismatch. | ✔ Critical fix — landed via /gsd-code-review-fix flow; 2 fixture-driven tests `#[ignore]`'d pending sigstore-rs `KeyPair::from_pkcs8` re-enablement |
```

**Adaptation for Phase 33** (verbatim shape from RESEARCH Pitfall 6 L457–460 — locked target is the 3-column Key Decisions table at L158-183, NOT the Requirements bullets):
```markdown
| Phase 33 Upstream parity strategy (continue / split / freeze) | <chosen option's rationale, one paragraph including: 5-criteria score summary, tiebreaker if applicable per D-33-C3, evidence from DIVERGENCE-LEDGER cluster counts + fork-only surface size> | ✔ Decided — [docs/architecture/upstream-parity-strategy.md](../docs/architecture/upstream-parity-strategy.md); UPST3-sync follow-up queued in ROADMAP § Phase 34 |
```

**Adaptation notes:**
- **CRITICAL — RESEARCH Pitfall 6 + Assumption A3:** The target IS the 3-column `| Decision | Rationale | Outcome |` table at L158-183, NOT the Requirements bullets at L65-101. CONTEXT.md/SPEC.md mention "REQ-WRU-01 / SHELL-01 shape" but those live in the Requirements section. The planner appends to the table.
- Row is APPENDED (after L183, the current last row) — preserves chronological-by-phase ordering of existing entries.
- "Outcome" column uses convention seen across the table: status glyph (`✔` complete, `↶` reversed, `⚠️` revisit, `✓` locked) + `—` separator + outcome prose. Phase 33 uses `✔ Decided` since the ADR ships `Accepted`.
- ADR link MUST use relative path `../docs/architecture/upstream-parity-strategy.md` (PROJECT.md is at `.planning/`, ADR is at `docs/architecture/`).

---

### `.planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md` — G-25-DRIFT-01 Update section (append)

**Analog:** `25-HUMAN-UAT.md` § G-25-DRIFT-01 (lines 62–87) — the EXISTING gap entry being extended

**Existing structure** (verbatim L62–87, the append target's current shape):
```markdown
### G-25-DRIFT-01 — Upstream parity drift on all 4 RESL flag names (v0.52)
severity: warning
status: open
discovered: 2026-05-10
discovered_in: 25-HUMAN-UAT (test 1 attempt)

**What:** All four RESL flags shipped by Phase 25 (`--memory`, `--cpu-percent`, `--max-processes`, `--timeout`) have been deprecated or renamed in upstream nono v0.52. [...]

**Where:** `crates/nono-cli/src/cli.rs` — flag definitions at lines ~1966 (`--memory`), the `--cpu-percent` parser around line 83, plus `--max-processes` and `--timeout` declarations elsewhere in the same file. [...]

**Impact:**
- Phase 25's source-level closure is INTACT [...]
- The user-facing CLI surface diverges from upstream v0.52 [...]
- All 6 HUMAN-UAT tests cannot be re-validated until either (a) upstream sync brings flag names current, or (b) the tests are rewritten [...]

**Why not caught earlier:** Phase 22 UPST2 was scoped as v0.38–v0.40 only. The DRIFT-01/DRIFT-02 tooling from Phase 24 (`check-upstream-drift` + GSD quick-task template) is the right machinery for this — it just hasn't been run against v0.52 yet.

**Recommended follow-up:**
- New phase or quick-task: **UPST3 — Upstream v0.41–v0.52 Parity Sync** (RESL flag rename surface specifically; may surface other drift areas worth folding in).
- Use the Phase 24 DRIFT tooling (`check-upstream-drift` + 260428-rsu-style quick-task template) as the entry point.
- Do NOT block Phase 25 milestone close on this — Phase 25's source-level deliverables are correct against the v0.40 baseline. The drift is a separate concern.

**Cross-references:**
- Phase 22 UPST2 SUMMARY (last upstream sync — through v0.40)
- Phase 24 DRIFT-01 (`check-upstream-drift` tooling) + DRIFT-02 (quick-task template)
- 260428-rsu deferred runbook (upstream-stack rebase pattern)
```

**Append target:** AFTER the `**Cross-references:**` bullet list (after line 87). No prior `Update` section exists in this file — Phase 33 establishes the convention per D-33-D2.

**Append pattern** (verbatim shape from RESEARCH Pattern 6 L361–368 + CONTEXT D-33-D2):
```markdown
**Update (Phase 33, 2026-MM-DD):**

1. **Drift audit summary:** "Confirmed N commits in upstream cluster <name> (v0.4X) covering the RESL-flag-rename surface. See `DIVERGENCE-LEDGER.md` for the full row table."
2. **Parity-strategy ADR decision:** "The strategic ADR landed at `docs/architecture/upstream-parity-strategy.md` picked option {A/B/C}: {short option name}. Implication for this gap: {RESL renames will sync in UPST3 / RESL flags stay fork-named with documented divergence / RESL flags stay frozen at v0.40 verbatim}."
3. **Closure handoff:** "Gap stays `status: open` until Phase TBD-NN (UPST3-sync follow-up) lands the actual renames. Phase 33 does NOT close G-25-DRIFT-01."
4. **Audit-walk note** (if applicable): "Audit surfaced N additional RESL-flag-rename commits beyond the 4 originally suspected from Phase 25 HUMAN-UAT; see ledger cluster <name>."
```

**Adaptation notes:**
- **CRITICAL — D-33-D2:** This is a FULL APPENDED SECTION, NOT a frontmatter-only edit. The frontmatter `status: open` field at L64 stays `open` (do NOT flip to `closed`).
- Audit-walk note (item 4) is conditional — include only if the actual audit surfaces additional RESL-rename commits beyond the 4 originally suspected.
- Phase number in item 3 — fill in the actual ROADMAP slot (likely Phase 34 per RESEARCH Assumption A2) at write time.
- ADR option name in item 2 — fill in whichever option the ADR picks (A continue / B split / C freeze).

---

### `.planning/ROADMAP.md` — UPST3-sync placeholder phase entry (append)

**Analog:** `.planning/ROADMAP.md` § Phase 33 (lines 394–409) — the EXISTING Phase 33 stub IS the template per D-33-D1 + RESEARCH Pattern 5

**Verbatim template** (L394–409 of the same file being modified):
```markdown
### Phase 33: Windows parity with upstream 0.52 features and divergence decision

**Goal:** [To be planned] — close the v0.52 upstream-parity gap surfaced after v0.41 baseline (Phase 25's G-25-DRIFT-01 + RESL-flag rename in upstream v0.52) AND decide whether continued parity is sustainable in this repo or warrants splitting Windows off into a dedicated repo (`always-further/nono`).

**Trigger:** Upstream baseline at v0.52 has accumulated feature divergence we have not yet absorbed; Phase 25 surfaced G-25-DRIFT-01 (RESL flags renamed in upstream v0.52) as an UPST3 follow-up. The repo-split question is a strategic decision, not just a code merge.

**Requirements:** TBD — to be locked at `/gsd-spec-phase 33` / `/gsd-discuss-phase 33`.

**Depends on:** Phase 25 (RESL Unix backends + G-25-DRIFT-01), Phase 32 (Sigstore Integration; closes Windows-only trust-anchor surface).

**Plans:** 0 plans

Plans:
- [ ] TBD (run `/gsd-spec-phase 33` then `/gsd-plan-phase 33`)

**Reference:** Upstream repo — https://github.com/always-further/nono
```

**Adaptation for Phase 33 (writing the UPST3-sync stub)** — verbatim copy with substitutions per D-33-D1 + RESEARCH Pattern 5:
```markdown
### Phase 34: UPST3 — Upstream v0.41–v0.52 Sync Execution

**Goal:** [To be planned] — execute the cherry-picks and manual replays catalogued in Phase 33's `DIVERGENCE-LEDGER.md` per the parity-strategy ADR (`docs/architecture/upstream-parity-strategy.md`), closing G-25-DRIFT-01 once the RESL flag renames land.

**Trigger:** Phase 33 audit produced disposition-complete ledger with `will-sync` clusters queued for execution; ADR locked continue-parity option (or pivot to split/freeze if ADR picks B/C).

**Requirements:** TBD — to be locked at `/gsd-spec-phase 34` / `/gsd-discuss-phase 34`.

**Depends on:** Phase 33 (audit ledger + parity-strategy ADR).

**Plans:** 0 plans

Plans:
- [ ] TBD (run `/gsd-spec-phase 34` then `/gsd-plan-phase 34`)

**Reference:** `.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md`, `docs/architecture/upstream-parity-strategy.md`, `.planning/templates/upstream-sync-quick.md`
```

**Adaptation notes:**
- **D-33-D1 conditional rename:** If the ADR picks option B (`split-windows`) or C (`freeze-at-v0.52`) rather than option A (`continue`), the placeholder title flips at plan-phase to match — e.g., `Phase 34: Windows-fork split execution` or `Phase 34: v0.52 freeze-bookkeeping`. Per RESEARCH Assumption A2 + Pitfall 7: planner re-checks `grep '^### Phase ' .planning/ROADMAP.md` immediately before writing to confirm Phase 34 is still the next free slot.
- **D-33-D1 verbatim title (option A path):** `UPST3 — Upstream v0.41–v0.52 Sync Execution`.
- **RESEARCH Pitfall 8 — TWO ROADMAP edits required:** (1) APPEND the new UPST3 stub after Phase 33; (2) UPDATE Phase 33's own existing row (L394–409) status from in-progress/TBD to complete. Don't ship one without the other. The Plans bullet under Phase 33 currently reads `- [ ] TBD (run /gsd-spec-phase 33 then /gsd-plan-phase 33)`; flip to `- [x]` entries naming the actual plans Phase 33 ships.

---

## Shared Patterns

### Plain-text ADR header (NOT YAML frontmatter)
**Source:** All 4 prior ADRs — `docs/architecture/{audit-bundle-target.md, aipc-unix-futures.md, broker-trust-anchor.md, sigstore-tuf-cache.md}` line 1–7 each
**Apply to:** `docs/architecture/upstream-parity-strategy.md` only (Phase 33's lone ADR deliverable)
**Pattern:**
```markdown
# <Title>

**Status:** Accepted
**Date:** YYYY-MM-DD
**Phase:** N (vX.Y description)
[optional fields: **Requirement:** | **Decision IDs:** | **Supersedes:** | **Related ADR:**]
```
**Why locked:** D-33-C4 + RESEARCH Pitfall 4 — grep-discoverability via `grep -l '^\*\*Status:\*\*' docs/architecture/*.md` is the acceptance gate. YAML frontmatter form (SPEC.md L80 phrasing) is explicitly rejected.

### Markdown table append convention
**Source:** `.planning/PROJECT.md` § Key Decisions (L158–183, 23+ existing rows show the convention)
**Apply to:** PROJECT.md Key Decisions row + DIVERGENCE-LEDGER.md per-cluster commit-row tables
**Pattern:**
- Header row + alignment row are fixed (`|----|----|----|`).
- New rows appended at the end of the table (chronological where applicable).
- No need to pad column widths — Markdown renders fine without alignment whitespace.
- Inline links use `[label](path)` form, paths relative to the file being edited.

### Phase-stub shape (ROADMAP.md)
**Source:** `.planning/ROADMAP.md` § Phase 33 (L394–409) — the file's own canonical stub
**Apply to:** UPST3-sync stub append
**Pattern:** 6-block structure — `### Phase NN: <title>`, `**Goal:**`, `**Trigger:**`, `**Requirements:**`, `**Depends on:**`, `**Plans:**`, `Plans:` bullet list, optional `**Reference:**`. Each block on its own line with blank-line separators.

### Cross-file linking conventions
**Source:** RESEARCH § Architecture Patterns + verified examples in audit-bundle-target.md L80–89
- From `docs/architecture/<adr>.md` → `.planning/` artifacts: relative `../../.planning/phases/.../FILE.md`
- From `.planning/PROJECT.md` → `docs/architecture/<adr>.md`: relative `../docs/architecture/<adr>.md`
- From `.planning/phases/NN/HUMAN-UAT.md` → cross-phase artifacts: relative `../../phases/MM/FILE.md` OR absolute repo-root form
- Apply to: every cross-reference in the 5 deliverables. Planner picks the form already used in the file being edited.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | — | — | All 5 deliverables have proven analogs; no novel file shapes. |

The two NEW files (DIVERGENCE-LEDGER.md, upstream-parity-strategy.md) have STRONG analogs (the v0.37.1..v0.40.1 audit precedent + the 4-ADR convention); the three MODIFIED files extend EXISTING shapes within the same file (append-only). Novel-within-Phase-33 content (the 3-option scored table with 5 criteria L/M/H, the fork-only-surface ledger section, the `**Update (Phase 33, ...):**` section pattern in HUMAN-UAT) all have verbatim shape excerpts spec'd in CONTEXT.md / RESEARCH.md and are reproduced above so the planner writes against locked text rather than re-deriving.

## Metadata

**Analog search scope:**
- `docs/architecture/` — 4 ADRs (all 4 read for header convention; audit-bundle-target.md selected as closest)
- `.planning/quick/260424-upr-review-upstream-037-to-040/` — 1 audit precedent (SUMMARY.md read for ledger shape)
- `.planning/PROJECT.md` — Key Decisions table (L158–183 read for row shape)
- `.planning/phases/25-cross-platform-resl-aipc-unix-design/25-HUMAN-UAT.md` — G-25-DRIFT-01 (L62–87 read for append-target shape)
- `.planning/ROADMAP.md` — Phase 33 stub (L394–409 read for placeholder shape)

**Files scanned:** 8 (5 analogs read end-to-end where ≤300 lines; PROJECT.md targeted-read at L140–185; ROADMAP.md targeted-read at L340–410; aipc-unix-futures.md / broker-trust-anchor.md / sigstore-tuf-cache.md targeted-read at L1–60 each for header pattern only)

**Pattern extraction date:** 2026-05-10

## PATTERN MAPPING COMPLETE

**Phase:** 33 — windows-parity-upstream-0-52-divergence
**Files classified:** 5 (2 new artifacts, 3 modified-in-place)
**Analogs found:** 5 / 5

### Coverage
- Files with exact analog: 4 (audit-bundle-target.md → upstream-parity-strategy.md; PROJECT.md Key Decisions table → row append; 25-HUMAN-UAT.md G-25-DRIFT-01 → Update section append; ROADMAP.md Phase 33 stub → UPST3 stub)
- Files with role-match analog: 1 (260424-upr SUMMARY.md → DIVERGENCE-LEDGER.md; precedent for "audit cycle" but D-33-B2 two-tier shape is novel — RESEARCH Pattern 7 + Code Example excerpts pin the verbatim cluster header form)
- Files with no analog: 0

### Key Patterns Identified
- ADR header is plain-text `**Status:** Accepted` line at L3, NOT YAML frontmatter — 4-of-4 prior ADRs prove the convention; D-33-C4 + RESEARCH Pitfall 4 lock it.
- Decision Table shape is `option | dimensions... | verdict` with chosen row's verdict cell = `**Accepted**` and rejected rows' verdict cell = `Rejected: <one-liner>` — verbatim from `audit-bundle-target.md` L37–45 + RESEARCH L520–525.
- DIVERGENCE-LEDGER two-tier structure (cluster header + nested commit-row table) has no direct analog; RESEARCH Pattern 1 + Code Example "Verified Ledger Cluster Header Shape" pin the exact form per D-33-B2/B3.
- Three modified-in-place files (PROJECT.md, 25-HUMAN-UAT.md, ROADMAP.md) all extend existing shapes within their own files — the analog IS the same file's pre-existing content. Append-only edits; no re-shaping of existing rows.
- Phase 33's ROADMAP placeholder write requires TWO edits, not one (Pitfall 8): append UPST3 stub + flip Phase 33's own row from in-progress to complete.

### File Created
`.planning/phases/33-windows-parity-upstream-0-52-divergence/33-PATTERNS.md`

### Ready for Planning
Pattern mapping complete. Planner can write PLAN.md action blocks with verbatim excerpts copy-pasteable from this file. All 5 deliverables have concrete code excerpts with file paths and line numbers; novel-shape sections (Decision Table scoring, two-tier ledger clusters, fork-only surface enumeration, G-25-DRIFT-01 Update section) all reference locked CONTEXT/RESEARCH excerpts so the planner writes against pinned text rather than re-deriving from prose.
