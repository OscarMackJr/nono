# Phase 69: UPST8 Audit - Context

**Gathered:** 2026-06-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Produce a single `69-DIVERGENCE-LEDGER.md` that audits the **non-macOS** slice of upstream
`always-further/nono` across a corrected range (see D-01), dispositioning **every** relevant
commit (`will-sync` / `fork-preserve` / `won't-sync` / `split`) before any cherry-pick runs in
Phase 70. **Audit only — no code changes, no cherry-picks.** Mirrors the Phase 54 UPST7 audit
shape and the Phase 63 Track-B divergence-ledger shape.

**Not in this phase:** any cherry-pick / D-19 trailer work (Phase 70), any macOS re-audit beyond
cross-referencing Phase 63's ledger, any ADR rewrite (the cadence review *confirms* Phase 33
Option A `continue`, it does not supersede it). New capabilities belong in their own phases.

</domain>

<decisions>
## Implementation Decisions

### Range & upstream-tag collision (the load-bearing correction)
- **D-01:** **Audit range = upstream `v0.60.0..v0.62.0` = `9a05a4ff..52809dda`** — the user
  explicitly wants the audit to reach upstream's true highest release (v0.62.0), NOT stop at the
  roadmap SC's `v0.61.2` ceiling. By drift-tool count this is **14 non-merge cross-platform
  commits**, only **+3** of which are new beyond `v0.61.2` (the `v0.61.2..v0.62.0` tail). This
  **diverges from the locked ROADMAP/REQUIREMENTS SC range (`v0.60.0..v0.61.2`)** — the planner/
  executor MUST add a range-extension note to the ledger headline and flag the SC for a +3 update.
- **D-02:** **CRITICAL EXECUTOR LANDMINE — never use the local `v0.62.0` tag for the `--to` bound.**
  The fork's own v2.8/v2.9 leapfrog created **local** tags `v0.62.0` (`3c5e9025`), `v0.62.1`
  (`78bcdca8`), `v0.62.2` (`93a7390e`) pointing at *fork* releases on a **divergent history**.
  Upstream's real `v0.62.0` = **`52809dda3b9ec5d7a237c26ac5e90840052993d9`**. Verified 2026-06-12
  via `git ls-remote --tags upstream`: upstream's highest is v0.62.0 (no upstream v0.62.1/v0.62.2).
  The drift-tool `--to` bound MUST be the **SHA `52809dda`**, never the tag. (Confirmation that the
  collision is live: `git rev-list --count v0.61.2..v0.62.0` returned a garbage **1889** because it
  compared upstream-v0.61.2 against fork-v0.62.0 — different lines of history.)
- **D-03:** **Re-fetch at audit-open and record head SHA + refetch date** (SC4). Record
  `upstream_head_at_audit` (`git rev-parse upstream/main`; was `849cda42` on 2026-06-12) and the
  pinned `drift_tool_sh_sha` / `drift_tool_ps1_sha` in the ledger frontmatter, exactly as Phase 54/63.
  If the re-fetch surfaces a tag newer than upstream v0.62.0, lock the range at v0.62.0 and defer the
  newer set to a future UPST9 (mirrors Phase 54's out-of-range deferral).

### macOS-overlap scoping (the genuinely novel wrinkle vs Phase 54)
- **D-04:** **Cross-reference Phase 63 for the overlap range, flag the fresh tail.** Phase 63
  already dispositioned the **macOS** slice of `v0.57.0..v0.61.2` (`63-DIVERGENCE-LEDGER.md`,
  `macos-audit`, 63 commits). Within the Phase 69 range:
  - **Overlap range `v0.60.0..v0.61.2`:** for shared/cross-platform commits, audit the **non-macOS
    delta fresh** (a shared-code commit can still carry non-macOS sync work even if its macOS part
    was absorbed in v2.10) AND add a **pointer to the matching Phase 63 ledger row**. Pure
    **macOS-only** commits → `won't-sync` / out-of-scope (already absorbed in v2.10) **with a Phase
    63 pointer** — do not re-disposition.
  - **Tail range `v0.61.2..v0.62.0` (3 commits Phase 63 NEVER saw):** audit the non-macOS surface
    fresh, AND for any commit that is **macOS-relevant**, flag it explicitly in the ledger as
    **"macOS un-audited — needs a future macOS top-up"** so a macOS-only tail commit does not fall
    through the crack between the Phase 63 (≤v0.61.2) and Phase 69 (non-macOS) scopes.

### Ledger shape — locked by precedent (Phases 42/47/54/63), NOT re-discussed
- **D-05:** Disposition vocabulary = **`will-sync` / `fork-preserve` / `won't-sync` / `split`**.
  Cluster-summary table + per-commit rows. Reuse the Phase 54 ledger structure verbatim.
- **D-06:** Include the **`windows-touch` column** and a **re-export / cross-cluster diff-inspect
  note** per the `feedback_cluster_isolation_invalid` lesson — disposition by diff-inspecting
  re-export surfaces, **not** `git log --name-only` isolation.
- **D-07:** **ADR-cadence review per the Phase 33 Option A `continue` rule** — the ledger records
  the cadence-review outcome (expected: **confirm Option A `continue`**); it does **not** silently
  supersede the Phase 33 ADR.
- **D-08:** **Reproducibility:** drift-tool invocation is `make check-upstream-drift` (Windows-host
  fallback `bash scripts/check-upstream-drift.sh`) with `--from v0.60.0 --to 52809dda --format json`;
  assert the `drift_tool_sh_sha` pin before the run; JSON output to gitignored `ci-logs-local/drift/`
  (NOT committed), exactly as Phase 54/63.

### Claude's Discretion
- **Plan structure:** single plan vs multi — almost certainly a **single audit plan** mirroring
  Phase 54's `54-01-UPST7-AUDIT-PLAN.md`; planner's call.
- **Will-sync P1/P2 tiering** (like Phase 63's three P1 commits) — the auditor may tier the will-sync
  set by cherry-pick priority for Phase 70, or leave it flat; not required by the SC.
- **Cluster grouping granularity** — how the 14 commits group into themed clusters is the auditor's
  call (Phase 54 produced 14 clusters from 40 commits).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/REQUIREMENTS.md` § UPST8 — UPST8-01 (this audit) + UPST8-02 (Phase 70 sync), full acceptance language + out-of-scope table.
- `.planning/ROADMAP.md` § Phase 69 — goal, depends-on (Phase 55 cadence), success criteria 1–4. **Note the SC range says `v0.60.0..v0.61.2`; D-01 extends it to `v0.62.0`.**

### Prior ledger templates (audit shape to mirror — column structure + disposition vocab)
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` — **most directly mirrored**: range/headline/reproduction/cluster-summary/`windows-touch` column + ADR-cadence review outcome.
- `.planning/phases/54-upst7-audit/54-01-UPST7-AUDIT-PLAN.md` — the single-plan audit shape to mirror.
- `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` — origin of the `windows-touch` column.
- `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md` — backfill-style reference.

### macOS cross-reference (D-04 — overlap range already dispositioned here)
- `.planning/phases/63-minifilter-spike-groundwork-macos-divergence-ledger-audit/63-DIVERGENCE-LEDGER.md` — macOS audit of `v0.57.0..v0.61.2`; cross-ref its rows for the overlap range; it does NOT cover the `v0.61.2..v0.62.0` tail.

### Reproducibility tooling
- `scripts/check-upstream-drift.sh` / `scripts/check-upstream-drift.ps1` — drift tool (path-filtered to cross-platform Rust under `crates/{nono,nono-cli,nono-proxy}/src/`); SHA-pin both before running.
- `Makefile` § `check-upstream-drift` target — dispatches the platform-appropriate twin.
- `.planning/templates/cross-target-verify-checklist.md` — referenced by UPST8-02 (Phase 70), not this audit.

### Project memory / lessons (load-bearing)
- Memory `feedback_cluster_isolation_invalid` — diff-inspect re-export surfaces, not `--name-only` (drives D-06).
- Memory `project_v28_opened` / `project_upst7_gap` — the fork-vs-upstream tag-collision history that makes D-02 a real landmine (fork leapfrogged to v0.62.x to clear upstream's tag line).
- Phase 33 ADR (Option A `continue` cadence rule) — the cadence-review baseline (D-07).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Prior DIVERGENCE-LEDGER files (Phases 42/47/54/63):** directly reusable frontmatter schema, cluster-summary table, disposition vocabulary, and `windows-touch` column. Phase 69 keeps `windows-touch` (NOT a `macos-only` column — this is the non-macOS audit).
- **Drift tool (`check-upstream-drift.{sh,ps1}`):** read-only over `.git`, path-filtered to cross-platform Rust; takes `--from`/`--to` refs (accepts a raw SHA for `--to`, which D-02 requires).

### Established Patterns
- **Audit-only phase:** produces a markdown ledger + a plan/summary, no source changes. The "codebase" under audit is the upstream git history + the prior fork ledgers, not the working tree.
- **Cross-target-drift guard:** the ledger should FLAG any in-range commit touching cfg-gated shared code (relevant to Phase 70's cherry-picks), per `feedback_clippy_cross_target`.

### Integration Points
- Phase 69 produces no runtime integration. Its output (the will-sync disposition set) is the **direct input to Phase 70's cherry-pick wave** (UPST8-02). Depends-on Phase 55 only for cadence/linear-ordering, not code.

</code_context>

<specifics>
## Specific Ideas

- User directive (2026-06-12): **"audit everything to that release"** — upstream is now at v0.62.0; the audit must reach it, not the roadmap's v0.61.2 ceiling. Resolved to range `v0.60.0..v0.62.0` by SHA (D-01), with the +3-tail and SC-divergence noted.
- The tag-collision check was run live during discussion: `git ls-remote --tags upstream` proves upstream's real `v0.62.0` = `52809dda`, distinct from the local fork tag `3c5e9025` (D-02).

</specifics>

<deferred>
## Deferred Ideas

- **UPST9 (future):** any upstream tag newer than v0.62.0 surfaced at re-fetch → defer (lock range at v0.62.0), mirroring Phase 54's out-of-range deferral.
- **macOS top-up for the `v0.61.2..v0.62.0` tail:** if D-04's flag surfaces a macOS-relevant tail commit Phase 63 never saw, the actual macOS disposition/cherry-pick of it is a follow-on (macOS surface), NOT in Phase 69's non-macOS scope.
- **The cherry-pick wave itself (UPST8-02):** Phase 70 executes the will-sync set with D-19 trailers; Phase 69 only inventories.

### Reviewed Todos (not folded)
The 3 `todo.match-phase` hits were low-confidence keyword false positives belonging to other phases — reviewed, not folded:
- `20260611-msi-vcredist-prereq.md` — MSI VC++ prereq, belongs to Phase 67 (clean-host install).
- `20260611-poc-cert-broker-clean-host.md` — POC cert broker trust, belongs to Phase 67.
- `20260612-macos-rlimit-as-setrlimit-fails.md` — macOS RLIMIT_AS, belongs to Phase 68 (resl enforcement).

</deferred>

---

*Phase: 69-UPST8 Audit*
*Context gathered: 2026-06-12*
