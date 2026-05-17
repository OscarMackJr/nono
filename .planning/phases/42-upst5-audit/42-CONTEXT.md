---
phase: 42
phase_name: upst5-audit
gathered: 2026-05-17
status: Ready for planning
requirements_locked_via: REQUIREMENTS.md § REQ-UPST5-01 (no SPEC.md — audit-only phase mirrors Phase 33 + 39 shape)
---

# Phase 42: UPST5 audit - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 42 ships ONE artifact: a falsifiable, disposition-complete `DIVERGENCE-LEDGER.md` inventory of every upstream commit in `v0.53.0..v0.54.0` (1 tag — `v0.54.0` `6b00932f`; ~20 non-merge commits). Per-cluster disposition (`will-sync` / `fork-preserve` / `won't-sync`) + rationale + `windows-touch` column, sized identically to Phase 33 + 39 audit-shape template. **First audit cycle where the `windows-touch: yes` column actually fires** per D-39-C3 conservative-default fork-preserve disposition: 2 known windows-touching commits land in `v0.54.0~5^2` and `v0.54.0~5^2~1` (`5d821c12 fix(platform): correctly parse windows registry dword values` + `0748cced feat(platform): implement robust windows platform detection`).

Phase 42 ledger is the binding input for Phase 43 (UPST5 sync execution). Phase 42 also queues a UPST6 placeholder phase entry in `ROADMAP.md` (location TBD — see D-42-B4) so the cadence wheel keeps turning, per the Phase 33 ADR's "per upstream release, lazily-evaluated" rule (`docs/architecture/upstream-parity-strategy.md` § Future audit cadence).

**In scope:**
- Run `make check-upstream-drift ARGS="--from v0.53.0 --to v0.54.0 --format json"` at phase-start and curate themed clusters with per-cluster disposition + rationale (windows-host dispatch via `bash scripts/check-upstream-drift.sh` per Phase 33 precedent if `make` is not on PATH).
- Write `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` mirroring Phase 33 + 39 two-tier schema (cluster headers + nested commit-row tables).
- Inline `windows-touch: yes/no` column on commit-row tables (D-39-C1 inherited as D-42-E4).
- Apply D-39-C3 conservative-default fork-preserve to `windows-touch: yes` clusters unless empty fork-side; explicit per-commit disposition for `5d821c12` + `0748cced` with rationale (success criterion #2).
- **Explicit `## ADR review` section** (D-42-E10 below) that this cycle **MUST verdict** the Phase 33 Option A `continue` strategy — confirm with per-cell L/M/H verdicts (no implicit re-affirmation) or amend. First cycle with empirical `windows-touch: yes` evidence requires explicit posture.
- **Empirical cross-check:** spot-check ≥3 fork-shared files for any upstream path the drift tool missed (success criterion #4; Phase 39 empirical-cross-check pattern).
- Queue a UPST6 placeholder phase entry in `ROADMAP.md` (v2.6 backlog OR a v2.5 § Future Cycles holding section if v2.6 is not yet scoped — auditor's call at plan-phase per D-42-B4).
- Update `.planning/STATE.md` at phase close.

**Out of scope (route elsewhere or explicitly defer):**
- **Any actual cherry-picks, manual replays, or code changes** — Phase 43 is the execution phase by construction; Phase 42 is audit + queue only.
- **Post-v0.54.0 commits** (4 known at context-capture time: `66c69f86 fix(snapshot): validate restore targets against symlinks`, `803c6947 chore(deps): bump nix`, `fc965ccc chore(deps): bump tokio`, `089cf6a0 chore(deps): bump cosign-installer`). UPST6 absorbs per D-42-A4 silent-on-post-range rule (D-39-A3 inherited).
- **Strategic ADR rewrite** — Phase 33 ADR Option A `continue` stays Accepted until explicitly superseded. Phase 42's `## ADR review` section verdicts (confirm or amend) but does NOT supersede; a superseding ADR is a separate phase if the audit shape demands it.
- **Drift-tool fixes surfaced mid-audit** — documented inline in ledger + spawn `.planning/quick/` follow-up task per D-39-D3 (carried forward); Phase 42 itself stays untouched to preserve `drift_tool_sh_sha` reproducibility.
- **Fork-only-surface re-enumeration wholesale** — Phase 33 enumerated 6+ fork-only Windows seams; Phase 39 referenced unchanged. Phase 42 may add a § Delta-since-Phase-39 fork-only surface section ONLY if Phase 41 introduced new fork-only Windows surface that affects audit interpretation (e.g., broker CR-02/CR-03 null-handle + empty-list paths, HandleTarget API migration at 14 sites, env_vars guard migration). Auditor's discretion at audit walk.
- **G-XX-DRIFT gap closure** — Phase 33's G-25-DRIFT-01 closed in Phase 34. Phase 39 had no equivalent. Phase 42 has no equivalent.
- **Baseline-aware CI gate work** — already done by Phase 41 (REQ-CI-03 closed; baseline SHA `13cc0628` per `.planning/templates/upstream-sync-quick.md:102`). Phase 42 inherits the clean baseline; Phase 43 uses it as the gate reference.

</domain>

<decisions>
## Implementation Decisions

### Audit invocation, scope, and reproducibility (Area A — discussed)

- **D-42-A1:** **Upper bound = v0.54.0 release boundary, not upstream HEAD.** Audit range = `v0.53.0..v0.54.0` (sha `6b00932f`, ~20 non-merge commits — 1 tag spanned). Inherits Phase 33 + 39 pattern verbatim (D-33-A1 / D-39-A1). Clean reproducibility: any reader rerunning the audit against the same tag pair gets the same input set. Post-v0.54.0 commits (4 known: `66c69f86` snapshot symlink validation, `803c6947` / `fc965ccc` / `089cf6a0` dep bumps) roll into UPST6. **User explicitly rejected** `upstream/main HEAD at first-commit-of-plan` and `v0.54.0 + cherry-pick 66c69f86 inline` to preserve audit-boundary discipline.

- **D-42-A2:** **Frontmatter captures BOTH `range` AND `upstream_head_at_audit`.** Ledger frontmatter (D-39-A2 inheritance, verbatim):
  - `range: v0.53.0..v0.54.0`
  - `upstream_head_at_audit: <40-char sha captured at first commit of Plan 42-01>`
  - `drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559` (Phase 24 ship sha; unchanged through Phase 33 + 39)
  - `drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559`
  - `drift_tool_invocation: 'make check-upstream-drift ARGS="--from v0.53.0 --to v0.54.0 --format json"'`
  - `fork_baseline: v0.53.0 (Phase 40 UPST4 sync point — 2026-05-14)`
  - `date: 2026-MM-DD`
  Raw drift JSON is NOT committed (D-33-A2 inherited; output redirects to `ci-logs-local/drift/` per `.gitignore`); the ledger is the canonical artifact. **User explicitly rejected** the `Range only` and `Range + post-tag commit count` variants — the auxiliary HEAD-sha is the historical signal that lets UPST6 reconstruct what was punted.

- **D-42-A3:** **Lock at first commit of Plan 42-01.** Auditor runs `git fetch upstream --tags` then captures `upstream/main` sha into the ledger frontmatter (`upstream_head_at_audit`) as the FIRST act of Plan 42-01. Range = `v0.53.0..v0.54.0`; the lock records post-fetch HEAD for reproducibility against the historical fetch state. Matches Phase 33 D-33-A1+A2 / Phase 39 D-39-D1 cadence verbatim. New upstream commits landing during the audit week are ignored; they roll into UPST6. **User explicitly rejected** earlier lock points (CONTEXT-commit, plan-phase-open) — first-commit-of-plan preserves the discipline of fetching tags as an explicit auditable act in the plan's commit history.

- **D-42-A4:** **Strictly silent on post-v0.54.0 commits.** Ledger covers `v0.53.0..v0.54.0` only. Anything past `6b00932f` is UPST6's problem; mentioning it would muddy the audit boundary. Inherits D-39-A3 verbatim. The cadence rule is structural — each audit closes a defined range, next audit picks up where this one left off. **User explicitly rejected** the `audit-watch addendum` shape even for the security-flavored `66c69f86 fix(snapshot): validate restore targets against symlinks`. UPST6 absorbs it on the next cycle.

### Plan slicing, close-gate, and Phase 43 hand-off (Area B — inherits Phase 39 defaults; planner may refine)

- **D-42-B1:** **Single plan (`42-01-DIVERGENCE-AUDIT`).** Inherits D-39-B1 verbatim. One plan does: drift run → cluster curation → ledger write → empirical cross-check ≥3 files → ADR review verdict → ROADMAP UPST6 stub → STATE.md update. ~20 commits is small enough that splitting adds overhead without traceability benefit. **Planner may override to multi-plan only if** the empirical cross-check (success criterion #4) surfaces a wave-shape concern that warrants Plan-42-02; default = single plan.

- **D-42-B2:** **Close-gate = Phase 39 D-39-B2 + explicit per-cell L/M/H verdict requirement:**
  1. `make check-upstream-drift ARGS="--from v0.53.0 --to v0.54.0 --format json"` exits 0 (drift tool reproduces against the locked range).
  2. `DIVERGENCE-LEDGER.md` row count ≥ drift-tool `total_unique_commits` (exact coverage, zero gap).
  3. Every cluster has disposition (`will-sync` / `fork-preserve` / `won't-sync`) + one-line rationale.
  4. **`## ADR review` section present AND verdicts with per-cell L/M/H** — first cycle with `windows-touch: yes` evidence demands explicit posture, not implicit re-affirmation. Falsifiable via grep for the section header + per-cell verdict format.
  5. **Empirical cross-check: ≥3 fork-shared files spot-checked** for upstream paths the drift tool missed (success criterion #4 enforcement). Findings appended to ledger as a § Empirical cross-check subsection.
  6. ROADMAP UPST6 stub committed (location per D-42-B4).
  7. STATE.md updated.
  8. `make ci` substitute: `git diff --name-only HEAD~3..HEAD -- crates/ bindings/ scripts/ | wc -l` == 0 (Phase 42 ships zero `.rs` / `.toml` / `.sh` / `.ps1` / `Makefile` edits — structurally zero clippy/fmt/test risk; D-39-E5 invariant trivially honored).
  No cross-target clippy gate needed (Phase 25 CR-A lesson) — Phase 42 touches zero `.rs` files.

- **D-42-B3:** **Disposition-complete at Phase 42 close + foundation/dependency hints.** Inherits D-39-B3 verbatim. Every cluster's disposition is locked at Phase 42 close — Phase 43 inherits an immutable input. Phase 42 ledger MAY tag the largest/most-foundational cluster as `wave-hint: foundation` (analog of D-34-A2 "C7 first" / D-39-B3 "Cluster 2 + Cluster 6 foundation"). Phase 42 planner may flag cluster dependencies inline (e.g., `wave-hint: depends-on cluster-N final state`). Phase 43 planner has full discretion to refine wave membership; Phase 42 hints are advisory, not prescriptive.

- **D-42-B4:** **UPST6 ROADMAP queue location TBD — auditor decides at plan-phase.** v2.6 milestone is not yet scoped (v2.5 is the current active milestone per STATE.md). Options for the UPST6 stub:
  - **(a) v2.6 backlog stub** — add a `## v2.6 backlog` section to ROADMAP.md if not present; queue UPST6 there. Cleanest; mirrors D-39-B4 (which queued UPST5 in v2.5 backlog from a v2.4 ROADMAP).
  - **(b) v2.5 § Future Cycles** — add a holding section to ROADMAP.md § Future Cycles documenting "UPST6 cadence trigger: when v0.55.0+ ships". Lighter touch; doesn't presuppose v2.6.
  - **(c) Cross-milestone backlog file** — write to `.planning/backlog/upst6-stub.md` rather than ROADMAP.md; cleaner separation but breaks Phase 39's ROADMAP-resident precedent.
  Auditor judges at plan-phase based on whether v2.6 has been scoped. **Recommendation:** (a) v2.6 backlog stub if v2.6 scope exists by Plan 42-01 close; otherwise (b) v2.5 § Future Cycles holding section. Stub shape inherits D-39-B4:
  - Title: `UPST6 — Upstream v0.54.0… sync audit` (or `… sync execution` if the next cycle's commit set is small enough to skip a dedicated audit; auditor's call)
  - `Depends on: Phase 43`
  - `Plans: 0 / TBD`
  - Cross-reference to `docs/architecture/upstream-parity-strategy.md` § Future audit cadence

### Windows-touching upstream commits (Area C — inherits Phase 39 defaults; **first cycle with actual fire**)

- **D-42-C1:** **Inline `windows-touch: yes/no` column on commit-row tables.** Inherits D-39-C1 verbatim. Schema: `sha + subject + upstream-tag + categories + files-changed-count + windows-touch`. **Known fires for Phase 42:**
  - `5d821c12 fix(platform): correctly parse windows registry dword values` (reachable from `v0.54.0~5^2`)
  - `0748cced feat(platform): implement robust windows platform detection` (reachable from `v0.54.0~5^2~1`)
  Audit walk may surface more; auditor confirms.

- **D-42-C2:** **Detection methodology = mechanical filename heuristic + judgment override.** Inherits D-39-C2 verbatim:
  1. **Mechanical pass:** `windows-touch: yes` iff any file in `files_changed` matches `windows` substring, OR matches the pinned list `{platform.rs, registry.rs, wfp/*, win_*.rs}`, OR commit subject contains `windows` / `wfp` / `registry` / `wsa` / `ntdll` / `kernel32` keywords.
  2. **Judgment override:** For any cluster's lead commit AND any flagged commit whose subject is ambiguous re: Windows-touch, auditor reads the diff and confirms or overrides the mechanical flag.

- **D-42-C3:** **Windows-touch defaults to `fork-preserve` UNLESS empty fork-side.** Inherits D-39-C3 verbatim with **explicit Phase 42 application required for the 2 known fires:**
  - **`0748cced feat(platform): implement robust windows platform detection`** — upstream introduces (likely) `crates/nono/src/platform.rs` or similar new Windows-conditional module. Auditor MUST check whether fork has an analog (Phase 31 broker-process work + Phase 35/36 supervisor work may have introduced fork-side Windows platform-detection seams under `*_windows.rs` paths). If fork-side is empty → straight cherry-pick CAN be considered (will-sync); if fork-side has its own implementation → fork-preserve with D-20 manual-replay rationale.
  - **`5d821c12 fix(platform): correctly parse windows registry dword values`** — likely a small fix on top of upstream's `0748cced` platform module. Disposition typically follows `0748cced`'s disposition (cherry-pick together as a Windows-platform-detection cluster, or fork-preserve together if `0748cced` is fork-preserve).
  Auditor's disposition for each commit MUST be recorded explicitly with rationale (success criterion #2 enforcement).

- **D-42-C4:** **Explicit `## ADR review` section with per-cell L/M/H verdict.** Inherits D-39-C4 + Phase 33 ADR verdict-table shape, with **stricter Phase 42 requirement:** because this is the first cycle where `windows-touch: yes` actually fires, the ADR review section MUST contain explicit per-cell L/M/H verdicts for each of the 5 evaluation dimensions Phase 33 ADR enumerated (security posture / windows parity / maintenance cost / divergence risk / contributor velocity). No implicit re-affirmation. Possible outcomes:
  - **(a) Confirm Option A `continue`** with updated per-cell verdicts reflecting empirical evidence that `windows-touch: yes` clusters can be dispositioned safely via D-42-C3 conservative-default.
  - **(b) Amend Option A** — record specific carve-outs (e.g., "Windows-platform-detection commits default to D-20 manual-replay, not cherry-pick"). NOT a supersede; ADR stays Accepted.
  - **(c) Flag a future-supersede trigger** — record evidence that suggests Option B `split` or Option C `freeze` should be re-evaluated in UPST6/UPST7. Stays Accepted; documents the trigger threshold (e.g., "if 50%+ clusters become fork-preserve in a future cycle, supersede").
  Falsifiable: `grep -c "^## ADR review$" DIVERGENCE-LEDGER.md` returns 1 AND `grep -cE "^\| (security|windows|maintenance|divergence|contributor)" DIVERGENCE-LEDGER.md` returns ≥5.

### Re-audit posture and mid-phase drift (Area D — inherits Phase 39 defaults)

- **D-42-D1:** **Lock audit range at first commit of Plan 42-01.** Inherits D-39-D1 verbatim. Auditor runs `git fetch upstream --tags` then captures `upstream/main` sha into ledger frontmatter (`upstream_head_at_audit`) as FIRST act of Plan 42-01. Range = `v0.53.0..v0.54.0`. New upstream commits landing during audit week → UPST6.

- **D-42-D2:** **Post-lock upstream commits → UPST6 absorbs them.** Inherits D-39-D2 verbatim. If a security-relevant upstream commit lands between Phase 42 close and Phase 43 start, Phase 42 ledger stays frozen. Phase 43 plan-phase may re-run `make check-upstream-drift` if urgency demands faster turnaround — that's a Phase 43 scope re-evaluation, NOT a Phase 42 retroactive edit. Default: UPST6 is the absorption vehicle.

- **D-42-D3:** **Drift-tool bugs documented inline + spawn `.planning/quick/` follow-up task.** Inherits D-39-D3 verbatim. If the auditor discovers a drift-tool bug mid-phase (category miscategorized, file filter misses a cross-platform path, etc.), the audit ledger documents the bug inline AND the auditor creates a quick-task entry under `.planning/quick/YYMMDD-xxx-upstream-drift-tool-fix/`. Phase 42 stays untouched to preserve `drift_tool_sh_sha` reproducibility.

### Empirical cross-check (Area E — Phase 42-specific carry-forward)

- **D-42-E1:** **Empirical cross-check ≥3 fork-shared files (success criterion #4 enforcement).** Inherits Phase 39's empirical-cross-check pattern (which surfaced the ce06bd59 / 5d821c12 / 0748cced mis-attribution to v0.53.0 in 39-CONTEXT.md preview vs actual `v0.54.0~5^2` reachability). Phase 42 auditor selects ≥3 fork-shared files (representative: 1 from `crates/nono/`, 1 from `crates/nono-cli/` excluding `_windows.rs`, 1 from `crates/nono-proxy/`) and walks the file's git log against the upstream `v0.53.0..v0.54.0` range to confirm the drift tool's commit list covers every upstream commit touching that file. Findings appended to ledger as a § Empirical cross-check subsection.

- **D-42-E2:** **Phase 41 delta surface awareness.** Phase 41 landed substantial fork-side changes:
  - HandleTarget API migration at 14 sites in `crates/nono-cli/src/exec_strategy.rs` (Plan 41-01)
  - Broker CR-01..04: NonoError::BrokerNotFound FFI mapping, null-handle validation, empty-list path, job-object test skip policy (Plans 41-06 + 41-07)
  - env_vars guard migration via EnvVarGuard::set_all (Plan 41-05)
  - Dead-code dispositions across audit_ledger.rs, audit_integrity.rs, exec_identity.rs, etc. (Plan 41-02)
  - cross-target-verify-checklist template (Plan 41-10 Class F)
  Empirical cross-check should preferentially sample files Phase 41 touched, since those are the files most likely to have drifted from upstream in ways the drift tool's mechanical path filter may not catch.

### Carry-Forward From Phase 33 + 39 (still binding)

- **D-42-E3 (= Phase 33 D-33-A1/A2 / Phase 39 D-39-E1):** Drift-tool invocation in ledger frontmatter is the audit-of-record; raw JSON not committed; reproducible against tag pair + drift-tool sha.
- **D-42-E4 (= Phase 33 D-33-B1 / Phase 39 D-39-E2):** Phase-local ledger location (`.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md`). No cross-phase append.
- **D-42-E5 (= Phase 33 D-33-B2 / Phase 39 D-39-E3):** Two-tier structure (cluster headers + nested commit-row tables); reader sees strategic disposition at a glance via cluster headers; commit-level audit trail in nested tables.
- **D-42-E6 (= Phase 33 D-33-B3 / Phase 39 D-39-E4):** Standard row schema = `sha + subject + upstream-tag + categories + files-changed-count + windows-touch`. Disposition + rationale live at the CLUSTER level, not per-row.
- **D-42-E7 (= Phase 22 D-17 / Phase 34 D-34-E1 / Phase 39 D-39-E5):** Windows-only files structurally invariant. Phase 42 does not edit `*_windows.rs` or `exec_strategy_windows/`. Trivially honored (Phase 42 ships only docs + ROADMAP edits).
- **D-42-E8 (= Phase 33 ADR cadence rule / Phase 39 D-39-E6):** "Per upstream release, lazily-evaluated" — Phase 42 closes when v0.53.0..v0.54.0 is fully dispositioned; UPST6 fires when v0.55.0+ ships or maintainer decides cherry-pick labor warrants absorbing accumulated post-v0.54.0 commits.
- **D-42-E9 (= Phase 41 close gate baseline):** Baseline-aware CI gate baseline SHA = `13cc0628` per `.planning/templates/upstream-sync-quick.md:102`. All Linux/macOS Clippy + 5 Windows CI lanes green on this baseline. Phase 43 (NOT Phase 42 — Phase 42 ships zero source edits) inherits this as the gate reference for `success → failure` regression detection.
- **D-42-E10 (= Phase 33 D-33-C4 ADR review section convention, **upgraded** for Phase 42):** Explicit `## ADR review` section in ledger MANDATORY (D-42-C4 above). Phase 42 upgrade vs Phase 39 D-39-C4: this cycle MUST include per-cell L/M/H verdicts (not just confirmation-or-flag prose) because empirical `windows-touch: yes` evidence is available for the first time.

### Claude's Discretion

- **Cluster grouping heuristic.** D-33-B2 / D-39-E3 / D-42-E5 says cluster related commits, but cluster boundaries (e.g., "windows-platform-detection" as one 2-commit cluster vs. split between `0748cced` feature and `5d821c12` fix) are the auditor's judgment call during the audit walk. Recommendation: keep `0748cced` + `5d821c12` in one cluster since `5d821c12` is a direct fix on `0748cced`'s introduced module.
- **Per-cluster `wave-hint` granularity.** D-42-B3 allows but does not require wave hints on every cluster. The auditor decides whether a cluster's wave shape is interesting enough to flag.
- **UPST6 stub title wording.** D-42-B4 names two candidate titles (`… sync audit` vs `… sync execution`). Auditor picks based on Phase 42 ledger shape — if dispositions are simple and the next cycle could be a single-plan execution phase without a separate audit, title flips to `… sync execution`. Otherwise default to `… audit`.
- **Whether to capture a `Fork-only surface area` delta section.** Phase 33 enumerated 6+ fork-only seams; Phase 39 referenced unchanged. Phase 42 should add a § Delta-since-Phase-39 fork-only surface section IF Phase 41's changes (HandleTarget API migration, broker CR-01..04, env_vars guard migration) meaningfully affect audit interpretation. Auditor judges at audit walk.
- **Empirical cross-check file selection.** D-42-E1 requires ≥3 fork-shared files; auditor picks which (recommendation in D-42-E2 to preferentially sample Phase-41-touched files).
- **`make ci` re-run cadence.** Standard project gate (D-42-B2 step 8) — auditor may run the `git diff --name-only` substitute once at plan close OR per-commit if curation surfaces any tooling change concerns. Either is acceptable.
- **ADR review verdict outcome.** D-42-C4 offers three outcomes ((a) confirm, (b) amend, (c) flag-future-supersede). Auditor judges based on audit-walk evidence; default is (a) confirm if `windows-touch: yes` clusters dispositioned cleanly via D-42-C3 conservative-default.

### Reviewed Todos (not folded)

All 6 surfaced todos are off-topic for an upstream-audit phase:
- `41-10-linux-deny-overlap-regression.md` (score 0.6) — Linux deny-overlap CI test fires only under specific conditions. Phase 41 follow-up; unrelated to UPST5 upstream audit.
- `41-10-windows-integration-env-vars-flake.md` (score 0.6) — Windows Integration env_vars parallel flake. Phase 41 follow-up; unrelated.
- `41-10-windows-regression-temp-vars-flake.md` (score 0.6) — Windows Regression temp_vars sibling flake. Phase 41 follow-up; unrelated.
- `v24-cr-01-broker-not-found-ffi-mapping.md` through `v24-cr-04-*` (score 0.6 each) — Phase 31 broker CR carry-forwards CLOSED by Phase 41 Plans 41-06 + 41-07; pending-todo files may be stale (orchestrator can verify and move to `.planning/todos/done/` if so).

All 6 matched on generic "phase, review, source, plan, windows" keywords. None topical to upstream-audit scope. Surfaced here for future-phase scoping awareness.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 42 scope sources
- `.planning/REQUIREMENTS.md` § REQ-UPST5-01 — Acceptance criteria (DIVERGENCE-LEDGER produced; per-cluster rationale references fork-only surface; explicit ADR review section; empirical cross-check ≥3 fork-shared files; `5d821c12` + `0748cced` explicitly handled). REQ-UPST5-02 lives in Phase 43.
- `.planning/ROADMAP.md` § Phase 42 (lines 71–82) — Goal, depends-on Phase 41, success criteria (5 items including explicit empirical cross-check + ADR review requirements), reference list.
- `.planning/PROJECT.md` § v2.5 Backlog Drain + UPST5 — milestone context, key decisions.

### Phase 39 audit-shape template (PRIMARY reference — Phase 42 mirrors verbatim with `windows-touch` actual fires)
- `.planning/phases/39-upst4-audit/39-CONTEXT.md` — D-39-A1..D-39-E6 decision IDs. Phase 42 D-42-A1..A4 inherit D-39-A1..A3 + D-39-D1 verbatim; D-42-B1..B4 inherit D-39-B1..B4 (with D-42-B4 explicit-location-undecided refinement); D-42-C1..C4 inherit D-39-C1..C4 (with D-42-C4 upgraded to require per-cell L/M/H verdicts); D-42-D1..D3 inherit D-39-D1..D3 verbatim; D-42-E3..E8 are the D-39-E1..E6 invariants.
- `.planning/phases/39-upst4-audit/DIVERGENCE-LEDGER.md` — **the worked example with `windows-touch` column zero-fire.** Phase 42 mirrors this shape with actual `windows-touch: yes` fires for `5d821c12` + `0748cced`.
- `.planning/phases/39-upst4-audit/39-01-PLAN.md` — single-plan structure Phase 42 mirrors.
- `.planning/phases/39-upst4-audit/39-01-SUMMARY.md` — Phase 39 close-gate verification methodology (drift-tool re-run idempotence, ledger row count == drift-tool total_unique_commits, ADR review grep-confirmable, ROADMAP UPST5 stub committed, STATE.md updated).

### Phase 33 audit-shape template root (MANDATORY reading — Phase 42 inherits D-33-A1..D-33-D2 transitively)
- `.planning/phases/33-windows-parity-upstream-0-52-divergence/33-SPEC.md` — 5 requirements + acceptance criteria for the audit-shape template; Phase 42 mirrors REQ-1 (drift audit) inheritance.
- `.planning/phases/33-windows-parity-upstream-0-52-divergence/33-CONTEXT.md` — D-33-A1..D-33-D2 decision IDs.
- `.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md` — 300-line ledger with frontmatter + Headline + Reproduction + Cluster Summary table + 12 cluster sections + Fork-only surface area section + `## ADR review` section.

### Phase 40 + 34 execution-shape templates (inform disposition decisions; Phase 43 will use these)
- `.planning/phases/40-upst4-sync-execution/40-CONTEXT.md` — D-40-A1..E5 (wave structure, baseline-aware CI gate, D-40-E1 Windows-only-files invariant + 4-condition addendum exception rule). Phase 42's dispositions feed Phase 43's wave structure.
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-CONTEXT.md` — D-34-A1..E5 (per-cluster plan slicing, foundation gate, fork-preserve handling).

### Strategic ADR (LOCKED — Phase 42 MUST verdict but NOT supersede)
- `docs/architecture/upstream-parity-strategy.md` — **Phase 33 strategic ADR, `Status: Accepted` 2026-05-11, re-confirmed at v2.4 close per D-39-C4.** Option A `continue` chosen. § Future audit cadence defines the "per upstream release, lazily-evaluated" rule. Phase 42's § ADR review section MUST verdict with per-cell L/M/H (D-42-C4); first cycle with empirical `windows-touch: yes` evidence requires explicit posture.

### Drift-tool infrastructure (Phase 24)
- `scripts/check-upstream-drift.sh` + `scripts/check-upstream-drift.ps1` — Drift-tool twin scripts. Sha `0834aa664fbaf4c5e41af5debece292992211559` (Phase 24 ship sha; unchanged since 2026-04-29 through Phase 33 + 39). Phase 42 invokes via `make check-upstream-drift` or `bash scripts/check-upstream-drift.sh` if `make` is not on PATH (Phase 33 + 39 precedent).
- `Makefile` § `check-upstream-drift` target — dispatches platform-appropriate script.
- `.planning/phases/24-parity-drift-prevention/24-CONTEXT.md` — D-04..D-19 drift-tool decisions (categorization D-05, range auto-detect D-08, fork-only filter D-11, JSON schema D-07). D-11 path filter on `*_windows.rs` + `exec_strategy_windows/` is the key invariant Phase 42 honors when interpreting drift-tool output — but D-42-C1/C2/C3 add the `windows-touch: yes` detection for upstream commits adding NEW Windows code outside the D-11 filter.
- `docs/cli/development/upstream-drift.mdx` — long-form runbook.

### Sync execution mechanics (referenced by Phase 43, mentioned for context)
- `.planning/templates/upstream-sync-quick.md` — MANDATORY scaffold for every Phase 43 plan; D-19 cherry-pick trailer block (verbatim 6-line shape with lowercase `Upstream-author:`); **baseline SHA `13cc0628` per Phase 41 close** (line 102). Phase 42 does NOT use this directly (no cherry-picks); Phase 43 plans inherit it from the Phase 34 + 40 pattern.
- `.planning/templates/cross-target-verify-checklist.md` — Phase 41 Class F template; Phase 43 plan-phase references for cross-target clippy verification.

### Phase 41 close-gate context (Phase 42 inherits clean baseline)
- `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-SUMMARY.md` — Phase 41 close gate; baseline-aware CI gate reset, all CI lanes green on baseline `13cc0628`, broker CR-01..04 closed, HandleTarget API migration at 14 sites complete.
- `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-VERIFICATION.md` — 3rd-pass verifier confirming Phase 41 close-gate semantics. Phase 42 inherits this as the "clean baseline" precondition.

### Coding & security standards
- `CLAUDE.md` § Coding Standards — no `.unwrap()`, DCO sign-off (`Signed-off-by:` lines), `#[must_use]` on critical Results, env-var save/restore in tests. Phase 42 ships only docs; trivially honored.
- `CLAUDE.md` § Security Considerations — path component comparison, fail-secure on any unsupported shape. Phase 42's audit interpretation lens for any cluster that touches path canonicalization or trust scanning (e.g., post-range `66c69f86 fix(snapshot): validate restore targets against symlinks` will need explicit security-flavor consideration at UPST6).
- `CLAUDE.md` § Cross-target clippy verification — Phase 41 close-gate codifies this; Phase 42 trivially honors (zero `.rs` edits), Phase 43 must observe.

### Upstream source (git-resolvable from `upstream` remote at `https://github.com/always-further/nono.git`)
- Tag `v0.53.0` (`c4b25b82`) — Phase 40 UPST4 sync point; Phase 42 baseline.
- Tag `v0.54.0` (`6b00932f`) — Phase 42 upper bound.
- Upstream HEAD at context-capture time: `18c34f3a` (2026-05-17; 4 post-v0.54.0 commits visible). Phase 42 plan locks `upstream_head_at_audit` at first commit of Plan 42-01 (D-42-A3) — may shift from this value if upstream commits land before Plan 42-01 starts. Range stays `v0.53.0..v0.54.0` regardless.

### v2.5 milestone context
- `.planning/STATE.md` — current milestone v2.5 status; Phase 42 follows Phase 41 close.
- `.planning/milestones/v2.4-MILESTONE-CONTEXT.md` — v2.4 scope-themes captured 2026-05-12; Phase 39 was v2.4 Theme 3 audit half.
- `.planning/milestones/v2.4-MILESTONE-AUDIT.md` — v2.4 close audit; D-39-C4 re-confirmation of Phase 33 ADR Option A `continue` at v2.4 close.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`make check-upstream-drift` tooling (Phase 24)** — `scripts/check-upstream-drift.{sh,ps1}` (sha `0834aa66`, unchanged since Phase 24 ship 2026-04-29 through Phase 33 + 39). Phase 42 invokes once for the audit (D-42-A2 captures invocation verbatim in ledger frontmatter).
- **Phase 39 DIVERGENCE-LEDGER.md as the closest worked template.** ~150–200 lines covering 22 commits / 7 clusters / windows-touch column with zero fires. Phase 42 replicates with the column actually firing for `0748cced` + `5d821c12`.
- **Phase 33 ADR `docs/architecture/upstream-parity-strategy.md`** — locked Accepted, re-confirmed at v2.4 close. § Future audit cadence defines the cadence rule Phase 42 honors. Phase 42's § ADR review section MUST verdict with per-cell L/M/H (D-42-C4) since this is the first cycle with empirical `windows-touch: yes` evidence.
- **Phase 40 wave-hint precedent (D-40-A2 "Wave 0 foundation").** Phase 42 may tag the largest/most-foundational cluster the same way (D-42-B3).
- **Phase 41 close-gate baseline `13cc0628`** — all CI lanes green; Phase 42's CI is trivial (docs-only) so this baseline is inherited only for Phase 43 reference, not Phase 42 enforcement.

### Established Patterns

- **`upstream` git remote** at `https://github.com/always-further/nono.git`; tags v0.40.1..v0.54.0 fetched locally (verified 2026-05-17). No setup work.
- **Phase-local ledger convention (D-33-B1 / D-39-E2 / D-42-E4).** Each audit phase owns its own ledger artifact in its own phase dir. No cross-phase append.
- **D-11 fork-only Windows filter (Phase 24 D-08).** Drift tool excludes `*_windows.rs` and `crates/nono-cli/src/exec_strategy_windows/` from output. Phase 42 must STILL detect upstream commits adding NEW Windows code outside that filter (D-42-C1/C2) — D-11 is necessary but not sufficient. Phase 42 is the **first cycle where this insufficiency matters** because `0748cced` + `5d821c12` exemplify exactly the case D-11 doesn't catch.
- **Two-tier ledger structure (D-33-B2 / D-39-E3 / D-42-E5).** Cluster headers carry strategic disposition; nested commit-row tables carry audit trail. Phase 33 worked example shipped in 300 lines for 97 commits / 12 clusters; Phase 39 shipped in ~150–200 lines for 22 commits / 7 clusters; Phase 42 expects ~130–180 lines for ~20 commits / 5–7 clusters.
- **Lazily-evaluated cadence (D-39-E6 / D-42-E8).** ADR § Future audit cadence rule fires per upstream release; Phase 42 absorbs the v0.54.0 tag (1 minor release / ~20 commits) in one cycle. UPST6 fires when v0.55.0+ ships.

### Integration Points

- `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` — NEW file Phase 42 creates. Phase 43 reads this as its immutable input.
- `.planning/ROADMAP.md` — Phase 42 appends a UPST6 placeholder phase entry (D-42-B4; location TBD between v2.6 backlog OR v2.5 § Future Cycles holding section, auditor's call at plan-phase).
- `.planning/STATE.md` — Phase 42 plan-close appends a "Last activity" log entry.
- `.planning/phases/39-upst4-audit/DIVERGENCE-LEDGER.md` — READ-ONLY reference. Phase 42 does NOT modify Phase 39's ledger.
- `docs/architecture/upstream-parity-strategy.md` — READ-ONLY reference. Phase 42 verdicts but does NOT supersede this ADR.

### Drift signal preview (informational, NOT a disposition pre-commit)

Pre-audit commit listing for v0.53.0..v0.54.0 (20 non-merge commits):
- `6b00932f chore: release v0.54.0` (likely C7-style release-ride absorption — Phase 34/40 convention)
- `42601ed7 fix(pack-update-hint): treat unparsable installed as older in update check`
- `98c18f1f feat(pack-hints): document inline pack update hints`
- `18b03fa6 feat(pack_update_hint): refresh hints synchronously on first run`
- `317c97b7 style(cli): adjust line breaks and module order`
- `5098fc10 feat(packs): add pinning, outdated, and clarify publishing versioning`
- `be23d6df style(cli): improve formatting and simplify error handling`
- `a5985edd feat(cli): implement nono update command`
- `64d9f283 feat(package): add package pinning and outdated commands`
- `548bb800 fix: macos lint` (×3 commits — `021074c9`, `ff2d8b84`)
- `8b888a1c feat: upgrade to Rust edition 2024, centralize workspace dependencies` ⚠ **likely large cross-platform impact; possible foundation-cluster candidate**
- `66c69f86 fix(snapshot): validate restore targets against symlinks` — **WAIT: this commit's tag-reachability needs verification**; if it's actually pre-v0.54.0 it lands in Phase 42 scope; if post-v0.54.0 it's UPST6. Auditor confirms at audit walk.
- `5d821c12 fix(platform): correctly parse windows registry dword values` ⚠ **windows-touch: yes**
- `0748cced feat(platform): implement robust windows platform detection` ⚠ **windows-touch: yes**
- `803c6947 chore(deps): bump nix from 0.31.2 to 0.31.3` — likely post-v0.54.0; UPST6
- `fc965ccc chore(deps): bump tokio from 1.52.2 to 1.52.3` — likely post-v0.54.0; UPST6
- `089cf6a0 chore(deps): bump sigstore/cosign-installer from 4.1.1 to 4.1.2` — likely post-v0.54.0; UPST6
- `ce06bd59 feat(profile): add platform-conditional profile fields` — could intersect with fork's Phase 22 `unsafe_macos_seatbelt_rules` + Phase 36 canonical sections
- `(remaining ~3 commits surface during audit walk)`

Likely cluster themes (auditor confirms during audit-walk):
- **Pack management (nono update + pinning/outdated)** — ~5 commits (`42601ed7` / `98c18f1f` / `18b03fa6` / `5098fc10` / `a5985edd` / `64d9f283` / `18b03fa6`). New CLI surface; likely `will-sync` cluster.
- **Rust edition 2024 + workspace dependency centralization** (`8b888a1c`) — Likely foundation cluster; cross-platform impact; **wave-hint: foundation** candidate.
- **Snapshot symlink validation fix** (`66c69f86`) — security-flavored if in scope; **scope verification needed**.
- **Platform-conditional profile fields** (`ce06bd59`) — could touch fork's Phase 22 / 36 canonical schema work.
- **Windows platform detection** (`0748cced` + `5d821c12`) — TWO commits adding new Windows code outside D-11; trigger D-42-C3 fork-preserve default; **disposition explicitly required for success criterion #2**.
- **macOS lint fixes** (3 commits) — likely `will-sync` if they touch fork-shared paths; trivial absorption.
- **Release v0.54.0** (`6b00932f`) — Phase 34/40 release-ride convention; CHANGELOG-only absorption (drop Cargo.toml + Cargo.lock version bumps; fork tracks own version).

These are **informational only** — the audit walk produces the authoritative cluster grouping + disposition per the methodology in D-42-A1..D-42-C4. Phase 42 plan-phase or research-phase may refine.

</code_context>

<specifics>
## Specific Ideas

- **v0.54.0 tag boundary, not HEAD** (D-42-A1) — user explicitly chose clean tag-boundary reproducibility over HEAD-snapshot or hybrid (cherry-pick 66c69f86 inline) shapes. Mirrors Phase 33 + 39 pattern.
- **Frontmatter captures both range AND upstream_head_at_audit** (D-42-A2) — user explicitly rejected the range-only shape; the HEAD capture is the historical signal that lets UPST6 reconstruct what was punted.
- **First-commit-of-Plan-42-01 lock timing** (D-42-A3) — user chose first-commit-of-plan over phase-open or plan-phase-open. Preserves discipline of `git fetch upstream --tags` as an explicit auditable act in the plan's commit history.
- **Strictly silent on post-v0.54.0 commits** (D-42-A4) — user explicitly rejected the audit-watch addendum shape even for security-flavored `66c69f86` snapshot symlink validation. The cadence rule is structural; UPST6 absorbs.
- **Explicit per-cell L/M/H verdict required in ADR review** (D-42-C4 upgrade) — Phase 42 is the first cycle with empirical `windows-touch: yes` evidence; implicit re-affirmation insufficient.
- **Explicit per-commit disposition for `0748cced` + `5d821c12`** (D-42-C3 application) — success criterion #2 enforcement; auditor records rationale + windows-touch fork-side state check.
- **Empirical cross-check ≥3 fork-shared files** (D-42-E1) — success criterion #4 enforcement; preferentially sample Phase-41-touched files (D-42-E2).
- **UPST6 stub location TBD at plan-phase** (D-42-B4) — auditor's call between v2.6 backlog (if v2.6 scoped by Plan 42-01 close) or v2.5 § Future Cycles holding section.

</specifics>

<deferred>
## Deferred Ideas

- **Post-v0.54.0 commit absorption** — 4 known unreleased commits between `6b00932f` and upstream HEAD `18c34f3a` at context-capture time (`cff5a59f Change warning to note about API changes`, `83256ac0 docs(installation): add makepkg`, `01456d99 docs(installation): add Arch Linux (AUR) section`, `530306ee review fix`, plus security-flavored `66c69f86 fix(snapshot): validate restore targets against symlinks` if reachability confirms post-v0.54.0). UPST6 absorbs per the lazily-evaluated cadence rule (D-42-E8) when v0.55.0 ships or maintainer decides accumulated cherry-pick labor warrants firing.
- **Drift-tool fixes surfaced mid-audit** — if Phase 42 audit-walk reveals a drift-tool category miscategorization or file-filter gap, the fix lands as a `.planning/quick/YYMMDD-xxx-upstream-drift-tool-fix/` quick-task (D-42-D3), NOT folded into Phase 42.
- **Full wave-map for Phase 43** — D-42-B3 ships foundation flag + dependency hints only; Phase 43 planner decides full Wave 0/1/2/3 mapping.
- **Fork-only surface area delta enumeration** — Phase 33 enumerated 6+ fork-only Windows seams; Phase 39 referenced unchanged. Phase 42 may add a § Delta-since-Phase-39 section if Phase 41 introduced new fork-only Windows surface (broker CR-02/03 null-handle + empty-list paths, HandleTarget API migration). Auditor's discretion at audit walk.
- **Superseding ADR** — if Phase 42's `## ADR review` section surfaces evidence that Option A `continue` is no longer the right call (e.g., per-cell L/M/H verdicts shift dramatically), that's a Phase-NN superseding ADR, NOT a Phase 42 inline edit. Phase 33 ADR stays `Accepted` until explicitly superseded. D-42-C4 outcome (c) "flag a future-supersede trigger" is the deferral path.
- **UPST6 scope** — UPST6 audit/sync execution; auditor at Phase 42 close picks title (`audit` vs `sync execution`) and queues in v2.6 backlog or v2.5 § Future Cycles holding section per D-42-B4.
- **`66c69f86` reachability verification** — pre-audit commit listing shows this commit ambiguously; auditor must verify whether it's pre-v0.54.0 (in scope for Phase 42) or post-v0.54.0 (UPST6). If post-v0.54.0, its security flavor (symlink TOCTOU class) makes it a candidate for elevated UPST6 priority but does NOT trigger inline D-42-A4 audit-watch addendum.

### Reviewed Todos (not folded)

- `41-10-linux-deny-overlap-regression.md` (score 0.6, area: general) — Linux deny-overlap CI test pre-flight investigation. Off-topic: Phase 41 follow-up; unrelated to UPST5 upstream audit.
- `41-10-windows-integration-env-vars-flake.md` (score 0.6, area: general) — Windows Integration env_vars parallel flake. Off-topic: Phase 41 follow-up.
- `41-10-windows-regression-temp-vars-flake.md` (score 0.6, area: general) — Windows Regression temp_vars sibling flake. Off-topic: Phase 41 follow-up.
- `v24-cr-01-broker-not-found-ffi-mapping.md` (score 0.6, area: general) — Re-map `NonoError::BrokerNotFound` to FFI `ErrSandboxInit` (Phase 31 CR-01). Off-topic + likely CLOSED by Phase 41 Plan 41-06; orchestrator may verify and move to `.planning/todos/done/`.
- `v24-cr-02-broker-null-handle-validation.md` (score 0.6) — Reject `--inherit-handle 0x0` in broker argv parser (Phase 31 CR-02). Off-topic + likely CLOSED by Plan 41-06.
- `v24-cr-03-broker-empty-handle-list-path.md` (score 0.6) — Broker empty-handle-list path (Phase 31 CR-03). Off-topic + likely CLOSED by Plan 41-06.

All 6 matched on generic "phase, review, source, plan, windows" keywords. None topical to upstream-audit scope.

</deferred>

---

*Phase: 42-upst5-audit*
*Context gathered: 2026-05-17*
