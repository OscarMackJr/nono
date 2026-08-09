# Phase 116: Tool-Sandbox Divergence Audit + Disposition ADR - Context

**Gathered:** 2026-08-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Produce two deliverables that turn upstream's never-absorbed `tool-sandbox/` subsystem from an
unexamined standing gap into a recorded decision:

1. **`116-DIVERGENCE-LEDGER.md`** — a per-commit divergence ledger covering the subsystem's full
   upstream history, in the Phase 108/98/94/85 shape (`windows-touch` flag, per-cluster
   dispositions, per-commit residue accounting, pinned-SHA Reproduction block).
2. **`proj/ADR-116-tool-sandbox-disposition.md`** — a standalone ADR returning an unambiguous
   **adopt vs. formalize-fork-native** verdict, with the fork's PR #4 path (PreToolUse hook →
   `nono run` + Low-IL primary-token broker) scored on its merits against the same criteria as
   upstream's, and Phase 120's scope sized from the verdict.

**This phase audits, evaluates and decides — only.** No absorb, no cherry-picks, no
`platform/windows.rs`, no code changes of any kind, no version bump. Executing the verdict is
Phase 120's job and is deliberately provisional until this phase lands.

Requirements: **TSBX-01** (ledger), **TSBX-02** (ADR + symbol-level grounding).

</domain>

<decisions>
## Implementation Decisions

### Audit Window & Ledger Surface

- **D-01: The window is the subsystem's full upstream history — `v0.64.1..v0.71.0`.**
  Not Phase 108's fenced `v0.66.0..v0.69.0`. Two reasons, both load-bearing:
  **(a)** PR #1105 — the thing the ADR is deciding about — landed at **v0.65.0**, *before* the
  fence, so a fence-only ledger would not contain its own subject; **(b)** upstream has shipped
  `v0.70.0`/`v0.71.0` since, and a verdict decided against a `v0.69.0` snapshot is stale on
  arrival while Phase 120 would inherit an undersized work-list.
  **Window tip is the `v0.71.0` tag (`0055bf3c`, 2026-07-31), not `upstream/main`** — a moving
  branch head breaks the pinned-SHA reproducibility the Phase 85/94/98/108 ledgers depend on.
  Record explicitly that `upstream/main` was at `4ede9ccc` (2026-08-05) at audit time, so the
  post-`v0.71.0` tail is a *named* boundary rather than silence.

  **Measured 2026-08-09 during discussion — hypothesis to re-confirm at audit time, never copy
  forward (Phase 108 D-04/D-21 rule; every prior audit's hypothesis was wrong somewhere):**

  | Measure (3-path union, inherited D-06 module set) | Value |
  |---|---|
  | Surface commits `v0.64.1..v0.71.0` | **38** |
  | — pre-fence (`v0.64.1..v0.66.0`, the #1105 intro) | 7 |
  | — Phase 108's fenced window (`v0.66.0..v0.69.0`) | 20 (9 pure / 11 split, per 108's ledger) |
  | — post-fence (`v0.69.0..v0.71.0`) | **11** (incl. #1476, #1492, #1467, #1453, #1521, #1550) |
  | `tool-sandbox/` at `v0.71.0` | 12 files, **18,433 LOC** |
  | — `platform/linux.rs` + `platform/macos.rs` | 6,586 + 7,041 = **13,627 (74% of subsystem)** |
  | — `platform/windows.rs` | **does not exist upstream** |
  | `command_policy.rs` at `v0.71.0` | **5,955 LOC** |

- **D-02: Every commit in the window gets a full disposition** (adopt / adapt / skip / split) with
  the `windows-touch` flag — including the 11 post-fence commits that sit outside the milestone's
  declared `v0.69.0` sync ceiling. **No separate `beyond-ceiling` flag.** FUT-08's declining of
  UPST13 constrains *absorb work*, not *knowledge*; a ledger that stops dispositioning at the
  ceiling hands Phase 120 (or FUT-09) 27 decided rows and 11 unexamined ones, which is the exact
  shape of gap this phase exists to close.

- **D-03: The module set is re-derived from the code at `v0.71.0`, then diffed against D-06's
  inherited three.** Follow `mod`/`use` edges out of `tool-sandbox/mod.rs` and `command_policy.rs`
  to establish what actually participates in the subsystem today; record the derived set and its
  delta from Phase 108's `tool-sandbox/` + `command_policy.rs` + `lineage_cgroup.rs` explicitly.
  Phase 108's D-06 was itself a *correction* of a boundary that had been measured wrong
  (directory-only = 18, union = 20 — **not equal**), and it warned in writing that the next sync
  MUST re-measure rather than inherit. Two more upstream releases have landed since.
  **Candidates the derivation must return an explicit in-or-out call on** (present upstream at
  `v0.71.0`, absent from the fork): `command_blocking_deprecation.rs`, `instruction_deny.rs`,
  `open_url_runtime.rs`, `hook_runtime.rs` — plus anything `tool-sandbox/` imports. An exclusion
  must be reasoned on the record, not implied by omission.

- **D-04: Residue is accounted per commit *and* routed.** Every path in every `split` commit is
  bucketed `absorb` / `defer` / `noise` (Phase 108 D-07 — a path in no bucket is a ledger defect).
  The module-scoped half always defers (the fork has no host module for it). The non-module half
  is real fork-relevant code and gets handled in two directions:
  - **Fenced-window residue** is reconciled against where Phases 109–112 *actually* landed it —
    **verified by grep against fork HEAD, not assumed from 108's routing note.** A routing note
    records an intent; only a symbol grep records an outcome.
  - **Post-fence residue mapping to no phase** is named as a new finding with a **proposed
    successor**, in the same operator-gated shape as Phase 108's D-19 Phase 112 amendment —
    proposed in the ledger, never applied to `ROADMAP.md` by this phase.

### The ADR's Option Space

- **D-05: The ADR is a two-pole comparison.**
  **Pole A — adopt + build `platform/windows.rs` from scratch** on the fork's existing
  AppContainer / Low-IL / WFP / broker primitives (the maximal reading of TSBX-03).
  **Pole B — formalize fork-native**: PR #4's PreToolUse hook → `nono run` + Low-IL
  primary-token broker becomes a permanent, named scope boundary in the ADR-111/ADR-113 shape.

- **D-06: The two intermediate shapes are recorded as considered-and-rejected, one paragraph
  each — not scored as full options.**
  *(i)* **Adopt Unix-only** (absorb for Linux/macOS where upstream's drivers land as-is, Windows
  stays fork-native) — leaves two tool-confinement architectures split by OS.
  *(ii)* **Adopt the policy model only** (`command_policy.rs`'s schema + the `command_policies`
  profile surface the fork already documents but never wired — docs-only, zero `.rs`, per D-06)
  enforced through the fork's existing Windows path.
  Naming them costs a paragraph and stops a future absorb re-proposing "just adopt it for Linux"
  as though it had never been examined. Precedent: ADR-113's OD-1 out-of-scope finding.

- **D-07: A capability feasibility matrix establishes what Pole A would actually take.**
  Read upstream's `platform/linux.rs` and `platform/macos.rs` and enumerate what the drivers
  *do* — per-command filesystem grants, exec shims, `url_shim`, `token_broker`,
  `dynamic_providers`, env scrubbing, per-command open-port mediation, sealed-shim runtime dir.
  Mark each capability **implementable-on-fork-primitives** / **implementable-but-new-work** /
  **structurally-blocked**, citing the fork symbol it would build on or the OS reason it cannot.
  This answers *"is it possible"* before *"what does it cost"* — the right order, because the
  `.NET`/PowerShell-CLR-cannot-start-under-`WRITE_RESTRICTED` finding proves some capabilities
  are blocked rather than merely expensive. Every row is independently re-checkable.

- **D-08: Minifilter-dependent capabilities are marked `structurally-blocked`, citing ADR-65.**
  Not "expensive", not "future work", and **not** framed as available-if-ADR-65-were-reversed.
  ADR-65's No-go/Conditional-go verdict stands and was re-affirmed at this milestone's open;
  per-file read policy inside one directory is explicitly not claimed. This makes Pole A's real
  Windows ceiling visible, and hands Phase 119's BOUND-02 a ready citation instead of a
  re-derivation.

- **D-09: One fixed criteria set, applied identically to both poles.** Symmetric scoring is what
  makes SC4's "on its merits rather than assumed inferior" checkable instead of asserted — and
  the reverse error is equally live, so the fork path does not get graded on a curve either.
  Criteria: **per-command granularity**; **engine-agnosticism** (the fork's honest weak spot — its
  entry point is Claude Code's PreToolUse contract, `claude_code_hook.rs` +
  `data/hooks/nono-tool-hook.ps1`, while upstream's subsystem is engine-neutral);
  **enforcement depth** (kernel-enforced vs. cooperative — the fork's strongest claim, and one the
  Unix drivers do not make on Windows at all); **fail-direction under layer failure**;
  **platform coverage** (Windows / Linux / macOS); **ADR-86 library-vs-CLI boundary impact**;
  **ongoing divergence + maintenance cost**.
  **No weighting is declared up front** — the ADR shows the scoring and argues from it.

### Verdict Posture

- **D-10: The verdict is genuinely open. No leaning is locked, and plans must not be written
  toward one.** This phase exists because the fork path was *assumed inferior* without
  examination; pre-locking the opposite assumption reproduces that error in reverse. This is a
  deliberate departure from Phase 108's D-11/D-12 ("the recommendation to argue for"), and the
  departure is the point. Planners: build the ledger, the feasibility matrix and the scoring, and
  let the verdict fall out. Do not seed either pole into task descriptions or success criteria.
  Consequence accepted: Phase 120's shape stays unknown until 116 lands.

- **D-11: The executor lands a decided ADR (`Status: Accepted`) — and Phase 120 planning is
  operator-gated on it.** TSBX-02 and SC3 both demand an unambiguous *recorded* verdict, so a
  `Proposed` ADR with the verdict left blank does not satisfy the requirement. But an *adopt*
  verdict would commit the milestone's remaining capacity to the largest build in v3.7, so the
  gate stays: **Phase 120 planning does not begin until the operator ratifies.** Same gate shape
  Phase 108's D-19 placed between its proposed amendment and Phase 109 planning.

- **D-12: SC3 is satisfied by named falsifiable triggers plus a named re-test point** — not by a
  reasoning paragraph. Each trigger is stated as a condition a future reader can *check*, e.g.
  "upstream ships `platform/windows.rs`", "a second agent engine needs confinement", "ADR-65's
  minifilter verdict is revisited", "the Unix drivers stop growing". Plus a named checkpoint —
  the next UPST sync — at which the triggers get re-tested. A trigger list nobody is obliged to
  look at is how this subsystem became a standing unexamined divergence in the first place.

### Deliverables, Evidence & Phase 120 Sizing

- **D-13: A new `116-DIVERGENCE-LEDGER.md`. Phase 108's ledger is not edited.**
  108's ledger declares itself closed (with one locked exception, the Phase 111 addendum);
  reopening it would make that closure statement unreliable and put v3.7 content inside the v3.6
  archive. The window differs anyway (D-01). The 116 ledger **cross-references** 108's
  tool-sandbox-pure/split rows rather than duplicating them.

- **D-14: `proj/ADR-116-tool-sandbox-disposition.md` carries a "Phase 120 Scope" section and
  proposes the corresponding `ROADMAP.md` amendment — operator-gated, written as a proposal, not
  applied.** One ratification moves both the decision and the roadmap. Phase-number ADR naming
  per ADR-98/108/111/113/114; independently citable; survives archival.

- **D-15: Phase 120's sizing derives from the feasibility matrix, row by row.** Each
  *implementable* / *implementable-but-new-work* row becomes a named deliverable with the fork
  primitive it builds on and a coarse **S / M / L** band. Bands stay coarse deliberately — LOC
  arithmetic off a 6.6k-line Landlock driver would be false precision. Commit counts are
  explicitly **not** the sizing basis (one commit is a 6k-line driver, another a one-line fix).
  The section states plainly what lands inside v3.7 versus what becomes **FUT-09**.

- **D-16: Every confidence rating carries its evidence inline — the literal command, its hit
  count, and the date measured.** A reviewer re-runs the cell without reconstructing what was
  checked. **File presence is never evidence**: Phase 112's disposition table was wrong in 3 of
  ~6 re-checked rows, every time from "the files exist" ≠ "the symbols exist".
  **And the greps must discover their targets, not confirm pre-named ones** — search the symbol
  across the tree rather than grepping a path someone already decided was the answer. Phase 115's
  V-01 found a verification test that was blind by construction because it scanned one function
  *by name*.

### Carried Forward (not re-litigated)

- **D-17: Ledger house shape** — pinned-SHA Reproduction block (SHAs, never tag names);
  cluster-first grouping with per-commit rows where dispositions bite; `windows-touch` flag;
  hypothesis-vs-measurement disagreements reconciled **in place**, with the superseded figure
  retained so the delta stays visible.
- **D-18: Fork invariants constrain every disposition** — the Windows security model
  (AppContainer / Low-IL / WFP / Job Object) and ADR-86's policy-free library boundary must not
  regress under either pole. Any pole implying cfg-gated Unix edits flags the cross-target clippy
  requirement for the executing phase (`cross` linux-gnu + `cargo-zigbuild` apple-darwin, both
  local, **no PARTIAL→CI**).
- **D-19: The carve-out re-touch check runs** (Phase 94 D-04/D-05, Phase 98 D-07):
  `git log <window> -- <exact path>` per accumulated fork carve-out, and a **zero-hit result is
  recorded explicitly** as "clean — no re-touch in window". Silence is not evidence.
- **D-20: No code changes.** Zero `.rs` edits, zero `Cargo.toml` edits, `ROADMAP.md` untouched
  (D-14's amendment is a proposal inside the ADR). `make ci` is not a gate for this phase; the
  gate is the two documents and their re-runnable evidence.

### Claude's Discretion

- Cluster naming and count within the D-17 scheme; ledger table layout and column ordering,
  provided `windows-touch` (TSBX-01), disposition, per-commit residue (D-04) and the D-16
  evidence column are all present.
- Whether to carry Phase 108's `security-relevant` flag forward as a second column — a lean
  toward *yes* (it made the D-18 security subset queryable rather than a judgment call), but not
  required by TSBX-01.
- The ADR's internal structure, provided both poles are scored on the D-09 criteria set, the D-06
  rejected shapes are named, and the D-12 triggers and D-14/D-15 sizing section are present.
- Plan and wave breakdown, and whether the feasibility matrix lives in the ADR or in a ledger
  appendix the ADR cites.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone definition
- `.planning/REQUIREMENTS.md` — v3.7 section: **TSBX-01 / TSBX-02** (this phase), TSBX-03
  (Phase 120), **FUT-08** (UPST13 declined — constrains absorb, not audit), **FUT-09**
  (conditional `platform/windows.rs` build-out), architecture invariants, and the Out-of-Scope
  table (minifilter, per-file read policy, TLS interception).
- `.planning/ROADMAP.md` — Phase 116 goal + SC1–SC5; Phase 120's provisional entry (the one
  D-14's proposed amendment would edit).

### Prior-ledger shape (the template to mirror)
- `.planning/phases/108-upst12-divergence-audit/108-CONTEXT.md` — D-05..D-09 (tool-sandbox
  fencing), **D-06 (the 3-path module set and its correction — the decision D-03 re-derives)**,
  D-07 (residue accounting), D-14/D-15 (granularity, noise floor), D-22 (carve-out re-touch).
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — the format to mirror;
  §"tool-sandbox-pure and tool-sandbox-split" holds the 20-commit / 9-pure-11-split baseline this
  ledger cross-references and extends.
- `.planning/milestones/v3.4-phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md` — the
  original cluster-table + per-cell-verdict shape.

### Disposition-ADR precedent (the ADR shape to mirror)
- `proj/ADR-113-spiffe-disposition.md` — closest analog: absorb disposition, positive-proof
  enumeration, OD-1 permanent named scope boundary, ROADMAP-SC-is-stale correction on the record.
- `proj/ADR-111-resource-limits-boundary.md` — the "formalize as a permanent named boundary" shape
  Pole B would take.
- `proj/ADR-114-oauth-capture-disposition.md`, `proj/ADR-108-deny-domain-posture.md` — the
  standalone phase-numbered ADR convention.

### Fork invariants
- `proj/ADR-86-library-boundary-convergence.md` — policy-free library boundary + the Windows
  denial-rendering carve-out (D-18; a criterion in D-09).
- `.planning/architecture/adr-65-minifilter-go-no-go.md` — the standing No-go/Conditional-go
  verdict D-08 cites for structurally-blocked capabilities.
  (`.planning/architecture/adr-65-latency-appendix.md` for the measured cost data.)
- `CLAUDE.md` — Library-vs-CLI boundary table; cross-target clippy MUST/NEVER; path-security and
  permission-scope rules.
- `.planning/templates/cross-target-verify-checklist.md` — the two cross-target clippy gates
  (relevant to D-18's forward-looking flag; not a gate for this phase per D-20).

### The fork-native path under evaluation (PR #4)
- `.planning/quick/260528-sch-spec-the-sandbox-the-tools-windows-tool-/260528-sch-SPEC.md` — the
  sandbox-the-tools design: PreToolUse hook → `nono run` per tool call.
- `crates/nono-cli/data/hooks/nono-tool-hook.ps1` — the shipped hook wrapper (57 lines, fail-closed
  deny JSON on any non-zero exit; stdout/stderr channel separation, finding R-A1).
- `crates/nono-cli/src/claude_code_hook.rs`, `crates/nono-cli/src/hooks.rs` — the handler and its
  installation path; `crates/nono-cli/src/hook_runtime.rs` equivalent upstream.
- `crates/nono-shell-broker/src/main.rs` — the Medium-IL broker that spawns Low-IL children.
- `crates/nono-cli/src/exec_strategy_windows/` — `restricted_token.rs`, `labels_guard.rs`,
  `dacl_guard.rs`, `network.rs`, `launch.rs`, `supervisor.rs`: the primitives a Pole-A Windows
  driver would have to build on, and the ones D-07's matrix maps capabilities against.
- `crates/nono-cli/src/execution_runtime.rs` — the `BrokerLaunchNoPty` **XOR** `WriteRestricted`
  token-arm split; the `.NET`/PowerShell-CLR-under-`WRITE_RESTRICTED` finding lives here and at
  `claude_code_hook.rs:1072`.
- `Skill("spike-findings-nono")` — engine-agnostic confinement patterns, landmines, and verified
  OS-behaviour facts (SEED-004); load before writing the feasibility matrix.
- `proj/DESIGN-engine-abstraction.md` — the engine-agnosticism criterion's fork-side context
  (D-09).

### Upstream surfaces this audit reasons about (read at `v0.71.0`, not fork HEAD)
- `crates/nono-cli/src/tool-sandbox/` — `mod.rs`, `policy.rs`, `env.rs`, `launch.rs`,
  `protocol.rs`, `token_broker.rs`, `url_shim.rs`, `credentials.rs`, `dynamic_providers.rs`,
  `audit_context.rs`, `platform/linux.rs`, `platform/macos.rs`. **No `platform/windows.rs`.**
- `crates/nono-cli/src/command_policy.rs` (5,955 LOC) and `crates/nono-cli/src/lineage_cgroup.rs`
  — the other two members of D-06's inherited module set.
- `crates/nono-cli/data/profile-authoring-guide.md` (fork) — the fork's only `command_policies`
  reference: **docs-only, zero `.rs` wiring** (verified Phase 108 D-06; re-verify per D-16).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **The Phase 108 ledger + ADR-108 pair, and ADR-113/ADR-114** — the exact two-deliverable shape
  this phase reproduces. Cluster table, `windows-touch` column, per-cell verdicts, pinned-SHA
  Reproduction block, operator-gated amendment proposal.
- **Phase 108's already-enumerated 20-commit surface** (9 pure / 11 split, with SHAs) — a verified
  starting point for 20 of the ~38 rows, cross-referenced rather than re-derived from zero.
- **`git log <window> -- <exact path>`** — the carve-out re-touch primitive from Phases 94/98/108,
  reused unchanged (D-19).
- **`exec_strategy_windows/` + `nono-shell-broker`** — a live, kernel-enforced Windows confinement
  stack. Pole A does not start from nothing; the feasibility matrix maps upstream capabilities
  onto these existing primitives.

### Established Patterns
- **Four dispositions:** adopt / adapt / skip / split. `split` is load-bearing here — the
  post-fence commits mix module-scoped work with fork-relevant residue (D-04).
- **Standalone phase-numbered ADR per headline decision** — ADR-86/98/108/111/113/114, now 116.
- **Hypothesis-vs-measurement reconciliation recorded in place** — every prior audit's
  discussion-time numbers were wrong somewhere and the ledger says so with the delta visible.
  D-01's table is explicitly a hypothesis on those terms.
- **Operator-gated roadmap amendment** — proposed in the artifact, never applied by the auditing
  phase (Phase 108 D-19).

### Integration Points
- **The ADR gates Phase 120 entirely** — its scope is provisional until the verdict lands, and
  planning is operator-gated on ratification (D-11).
- **D-08 feeds Phase 119's BOUND-02** — structurally-blocked rows citing ADR-65 are the citation
  BOUND-02 needs for "per-file read policy is not enforced".
- **D-09's fail-direction criterion touches Phase 117's CINT-01** — both poles have to answer what
  happens when a confinement layer cannot be established; 117 will enumerate those layers
  formally. 116 does not depend on 117 (ROADMAP: "depends on nothing"), but should not contradict
  it — flag any disagreement rather than reconciling it silently.
- **D-04's post-fence residue finding may propose a successor phase or FUT item** — operator-gated,
  and the natural home is UPST13/FUT-08 rather than v3.7 scope pressure.

</code_context>

<specifics>
## Specific Ideas

- **The single most decision-relevant measured fact: 74% of the subsystem is Unix platform drivers
  (`linux.rs` 6,586 + `macos.rs` 7,041 = 13,627 of 18,433 LOC) and upstream ships no
  `platform/windows.rs` at all.** "Adopt" is therefore not "take upstream's code" on the platform
  this milestone cares about — it is "take upstream's model and write its largest component from
  scratch". Keep this framing explicit in the ADR so neither pole is compared against a phantom.
- **The `.NET`/PowerShell-CLR-cannot-start-under-`WRITE_RESTRICTED` finding is a capability-class
  fact, not an anecdote** — it is why the fork has a `BrokerLaunchNoPty` arm at all. In D-07's
  matrix it is the worked example of `structurally-blocked` versus merely expensive.
- **Phase 108's D-06 correction is the model for D-03.** The original claim ("directory-only equals
  the 3-path union — true for this window") came from a substring-anywhere glob that matched a
  `.mdx` docs file. It was recorded as *a coincidence to re-test, not a rule to inherit*. That
  phrasing was written for whoever ran the next sync. This is that phase.
- The four borderline modules in D-03 (`command_blocking_deprecation.rs`, `instruction_deny.rs`,
  `open_url_runtime.rs`, `hook_runtime.rs`) were spotted by eyeballing upstream's `src/` listing,
  not by following imports — treat them as a *prompt for the derivation*, not as the answer.

</specifics>

<deferred>
## Deferred Ideas

- **Executing the verdict → Phase 120 (TSBX-03).** Absorb-with-driver or formalize-as-boundary,
  operator-gated on D-11; remainder that cannot land in v3.7 becomes **FUT-09**, sized from D-15's
  per-row bands rather than left as an implicit carry-forward.
- **Upstream sync past `v0.71.0` → FUT-08 / UPST13.** `upstream/main` was at `4ede9ccc`
  (2026-08-05) at audit time. This phase names the boundary (D-01) and does not audit past it.
- **Post-fence non-module residue that maps to no phase** (D-04) — surfaced as a finding with a
  proposed successor; the natural home is UPST13, not v3.7 scope.
- **The fork's engine-agnosticism gap** — scored as a criterion (D-09), not remediated here.
  Whatever the verdict, decoupling confinement from Claude Code's PreToolUse contract is its own
  work item and belongs with the engine-abstraction/`nono-agentd` line.
- **Gray areas raised but not discussed** (available if planning needs them): ledger granularity
  (per-commit for all ~38 vs. cluster-first with per-commit only where dispositions bite);
  whether the fork's docs-only `command_policies` profile surface needs its own disposition row;
  and how the carve-out re-touch check behaves when the fork has no host module for the carve-out.

### Reviewed Todos (not folded)
- `20260611-poc-cert-broker-clean-host.md` — matched at 0.6 on the keywords "broker / phase /
  finding", but the "broker" is the POC **certificate** broker for clean-host install, not the
  Low-IL shell broker. Host-gated v3.5 distribution item owned by v3.5 Phase 106. Phase 108
  reviewed and declined the same todo for the same reason.
- `20260611-msi-vcredist-prereq.md` — matched at 0.2 on the keyword "phase" only. MSI prerequisite
  work, v3.5 Phase 106. Unrelated to a tool-sandbox divergence audit.

</deferred>

---

*Phase: 116-tool-sandbox-divergence-audit-disposition-adr*
*Context gathered: 2026-08-09*
