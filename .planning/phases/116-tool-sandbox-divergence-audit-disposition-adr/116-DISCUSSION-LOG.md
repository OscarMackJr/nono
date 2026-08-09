# Phase 116: Tool-Sandbox Divergence Audit + Disposition ADR - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-09
**Phase:** 116-tool-sandbox-divergence-audit-disposition-adr
**Areas discussed:** Audit window boundary, What "adopt" concretely means, Verdict posture, Deliverables + Phase 120 sizing

---

## Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| Audit window boundary | Where the ledger's window starts/ends given 38 surface commits vs. Phase 108's fenced 20 | ✓ |
| What "adopt" concretely means | Upstream ships no `platform/windows.rs`; 74% of the subsystem is Unix drivers | ✓ |
| Verdict posture | Pre-locked leaning (Phase 108 D-12 precedent) vs. genuinely open | ✓ |
| Deliverables + Phase 120 sizing | File layout, ADR naming, where SC5's sizing lands | ✓ |

**User's choice:** All four.

---

## Audit window boundary

### Q1 — What commit window does the 116 ledger cover?

| Option | Description | Selected |
|--------|-------------|----------|
| Full subsystem history → v0.71.0 | `v0.64.1..v0.71.0`, 38 commits; verdict decided against the subsystem as it exists today | ✓ |
| Full history → v0.69.0 fence only | 27 commits; respects FUT-08's declared ceiling, names the 11-commit tail as unaudited | |
| Phase 108's fenced 20 only | Cheapest; but PR #1105 itself lands at v0.65.0, before the fence | |
| Full history → upstream main tip | Maximum currency; moving branch head breaks pinned-SHA reproducibility | |

**User's choice:** Full subsystem history → v0.71.0.
**Notes:** Window tip is the `v0.71.0` tag (`0055bf3c`), not `upstream/main` (`4ede9ccc`, 2026-08-05) — the main tip is recorded as a named boundary. → **D-01**

### Q2 — Disposition status of the 11 post-fence commits

| Option | Description | Selected |
|--------|-------------|----------|
| Full dispositions, same as the rest | Auditing is not syncing; FUT-08 constrains absorb work, not knowledge | ✓ |
| Dispositioned but flagged out-of-ceiling | Same dispositions plus a `beyond-v0.69.0-ceiling` flag per row | |
| Evidence-only rows, no disposition | Listed but undispositioned; violates D-07's spirit | |

**User's choice:** Full dispositions, same as the rest — no separate ceiling flag. → **D-02**

### Q3 — How the module set is derived

| Option | Description | Selected |
|--------|-------------|----------|
| Re-derive from the code, then compare | Follow `mod`/`use` edges at v0.71.0; record delta vs. D-06's inherited 3 | ✓ |
| Inherit D-06's 3 paths verbatim | Diff-comparable with 108; risks the exact failure D-06 was corrected for | |
| 3 paths + a named expansion review | D-06 as spine, explicit in/out call on 4 named candidates | |

**User's choice:** Re-derive from the code, then compare.
**Notes:** The four borderline modules named in the question (`command_blocking_deprecation.rs`, `instruction_deny.rs`, `open_url_runtime.rs`, `hook_runtime.rs`) carry forward as a prompt for the derivation, not as its answer. → **D-03**

### Q4 — Split-commit residue handling

| Option | Description | Selected |
|--------|-------------|----------|
| Account it, and route the unabsorbed | Full D-07 bucketing; fenced residue grep-reconciled against 109-112; post-fence orphans get a proposed successor | ✓ |
| Account it, don't route it | Full bucketing, observation only, no proposed phase | |
| Module-scoped accounting only | Smallest ledger; reproduces the silent-loss failure D-07 exists to catch | |

**User's choice:** Account it, and route the unabsorbed. → **D-04**

---

## What "adopt" concretely means

### Q1 — Which adopt shapes must the ADR evaluate? (multi-select)

| Option | Description | Selected |
|--------|-------------|----------|
| Adopt + new Windows driver | Absorb the subsystem and write `platform/windows.rs` from scratch on fork primitives | ✓ |
| Adopt Unix-only | Linux/macOS absorb, Windows stays fork-native; two architectures split by OS | |
| Adopt the policy model only | `command_policy.rs` schema + `command_policies` surface, enforced via the fork's Windows path | |
| Formalize fork-native | PR #4's hook + Low-IL broker as a permanent named boundary | |

**User's choice:** Adopt + new Windows driver — the only adopt shape scored as a full option.
**Notes:** Formalize-fork-native is the verdict's other pole by construction (TSBX-02/SC3), so the ADR is a two-pole comparison. → **D-05**

### Q2 — Treatment of the unselected intermediate shapes

| Option | Description | Selected |
|--------|-------------|----------|
| Recorded as considered-and-rejected | One paragraph each; stops a future absorb re-proposing them as unexamined | ✓ |
| Not mentioned at all | Cleanest verdict; leaves SC3 unable to distinguish rejected from unseen | |
| Full third/fourth options after all | Most complete; quadruples the comparison and risks a split verdict | |

**User's choice:** Recorded as considered-and-rejected. → **D-06**

### Q3 — How the ADR establishes what a Windows driver would take

| Option | Description | Selected |
|--------|-------------|----------|
| Capability feasibility matrix | Enumerate upstream driver capabilities; mark implementable / new-work / structurally-blocked with a cited fork symbol or OS reason | ✓ |
| Matrix + LOC-derived size estimate | Same plus a number grounded in the 6.6k/7.0k driver LOC | |
| Structural comparison, no sizing | Architecture fit only; leaves Phase 120 open-ended | |

**User's choice:** Capability feasibility matrix (feasibility, not LOC).
**Notes:** This left SC5's sizing without a basis — resolved in the Deliverables area Q3. → **D-07**

### Q4 — Criteria the two poles are scored against

| Option | Description | Selected |
|--------|-------------|----------|
| Fixed criteria set, both poles scored | One list applied identically: granularity, engine-agnosticism, enforcement depth, fail-direction, coverage, ADR-86 impact, divergence cost | ✓ |
| Criteria set + explicit weighting | Same list with decisive criteria declared up front | |
| Narrative comparison | Prose; makes "on its merits" hard to confirm | |

**User's choice:** Fixed criteria set, both poles scored — no declared weighting. → **D-09**

---

## Verdict posture

### Q1 — Is a leaning locked now?

| Option | Description | Selected |
|--------|-------------|----------|
| Verdict genuinely open | No leaning; the verdict falls out of the evidence | ✓ |
| Lean formalize fork-native | D-12-shaped recommendation to argue for, given LOC and no upstream Windows driver | |
| Lean adopt | Argue toward absorbing; permanent divergence compounds | |

**User's choice:** Verdict genuinely open.
**Notes:** A deliberate departure from Phase 108's D-11/D-12 precedent — the phase exists because the fork path was assumed inferior without examination, and pre-locking the opposite assumption repeats the error in reverse. Plans must not seed either pole. → **D-10**

### Q2 — Who lands the verdict?

| Option | Description | Selected |
|--------|-------------|----------|
| Executor lands it, operator gate before 120 | ADR written `Status: Accepted`; Phase 120 planning gated on operator ratification | ✓ |
| Executor lands it, no gate | Fastest; risks silently committing the milestone tail to the largest build in v3.7 | |
| ADR proposes, operator decides | Max control; a verdict-blank `Proposed` ADR fails TSBX-02/SC3 as written | |

**User's choice:** Executor lands it, operator gate before 120. → **D-11**

### Q3 — Concreteness of SC3's reconciliation surface

| Option | Description | Selected |
|--------|-------------|----------|
| Named falsifiable triggers + review point | Checkable conditions plus a named re-test at the next UPST sync | ✓ |
| Named triggers, no review point | Concrete but nothing forces anyone to look | |
| Reasoning paragraph | Lighter to write, harder to act on later | |

**User's choice:** Named falsifiable triggers + review point. → **D-12**

### Q4 — Minifilter-dependent capabilities in the feasibility matrix

| Option | Description | Selected |
|--------|-------------|----------|
| Marked structurally-blocked, ADR-65 cited | Makes Pole A's Windows ceiling explicit; hands Phase 119's BOUND-02 a ready citation | ✓ |
| Marked blocked, no ADR-65 dependency | Cleaner separation, weaker traceability | |
| Treated as conditional-on-ADR-65 | Shows the theoretical ceiling; reads as reopening a re-affirmed decision | |

**User's choice:** Marked structurally-blocked, ADR-65 cited. → **D-08**

---

## Deliverables + Phase 120 sizing

### Q1 — Where the 116 ledger lives

| Option | Description | Selected |
|--------|-------------|----------|
| New `116-DIVERGENCE-LEDGER.md` | Own file, cross-references 108 rather than duplicating; respects 108's closure | ✓ |
| New ledger + a pointer stub in 108 | Same plus a successor note appended to 108's tool-sandbox section | |
| Append to 108's ledger | One record; makes 108's closure statement unreliable and puts v3.7 content in the v3.6 archive | |

**User's choice:** New `116-DIVERGENCE-LEDGER.md`; 108's ledger is not edited. → **D-13**

### Q2 — Where Phase 120's sizing lives

| Option | Description | Selected |
|--------|-------------|----------|
| ADR section + ROADMAP amendment, gated | Sizing in the ADR plus a proposed roadmap edit in the D-19 shape, not applied | ✓ |
| ADR section only | Fewer moving parts; leaves the milestone tail visibly open-ended in ROADMAP | |
| Separate scoping doc | Cleanest separation; a third artifact to keep in sync | |

**User's choice:** ADR section + ROADMAP amendment, operator-gated. → **D-14**

### Q3 — What the sizing derives from

| Option | Description | Selected |
|--------|-------------|----------|
| Per-row deliverable list + size band | Each feasibility-matrix row becomes a named deliverable with a coarse S/M/L band | ✓ |
| Verdict-branch outlines | Both poles costed before the verdict lands, removing pressure to shade toward the cheaper branch | |
| Ledger disposition counts | Mechanical; commit counts are a poor proxy when one commit is a 6k-line driver | |

**User's choice:** Per-row deliverable list + size band. → **D-15**

### Q4 — How symbol-level evidence is recorded

| Option | Description | Selected |
|--------|-------------|----------|
| Evidence column: literal command + hit count | Inline per rating, re-runnable without reconstruction; greps discover targets rather than confirming pre-named paths | ✓ |
| Evidence column + Reproduction block | Same plus a top-level pinned-SHA block for whole-ledger re-measurement | |
| Footnoted evidence appendix | Readable at 38+ rows; the indirection is where grounding quietly gets lost | |

**User's choice:** Evidence column with literal command + hit count. → **D-16**
**Notes:** The pinned-SHA Reproduction block was already locked as a carried-forward ledger convention (D-17) before this question was asked, so this choice governs the per-rating evidence format and does not remove it.

---

## Claude's Discretion

- Cluster naming and count; ledger table layout and column ordering (subject to the mandatory
  `windows-touch`, disposition, residue and evidence columns).
- Whether to carry Phase 108's `security-relevant` flag forward as a second column (leaning yes).
- The ADR's internal structure, given the D-09 criteria set, D-06 rejected shapes, D-12 triggers
  and D-14/D-15 sizing section are all present.
- Plan and wave breakdown; whether the feasibility matrix lives in the ADR or a cited ledger
  appendix.

## Deferred Ideas

- Executing the verdict → Phase 120 (TSBX-03); un-landable remainder → FUT-09.
- Upstream sync past `v0.71.0` → FUT-08 / UPST13 (`upstream/main` at `4ede9ccc`, 2026-08-05).
- Post-fence non-module residue mapping to no phase → proposed successor, natural home UPST13.
- The fork's engine-agnosticism gap → scored as a criterion, remediation belongs with the
  engine-abstraction / `nono-agentd` line.
- Undiscussed gray areas available if planning needs them: ledger granularity (per-commit for all
  ~38 vs. cluster-first); whether the fork's docs-only `command_policies` surface needs its own
  disposition row; carve-out re-touch behaviour when the fork has no host module.

## Reviewed Todos (not folded)

- `20260611-poc-cert-broker-clean-host.md` (score 0.6) — the "broker" is the POC **certificate**
  broker for clean-host install, not the Low-IL shell broker. v3.5 Phase 106. Phase 108 declined
  the same todo for the same reason.
- `20260611-msi-vcredist-prereq.md` (score 0.2) — keyword-only match on "phase". v3.5 Phase 106.
