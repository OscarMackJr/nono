# Phase 108: UPST12 Divergence Audit - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-29
**Phase:** 108-upst12-divergence-audit
**Areas discussed:** tool-sandbox entanglement, deny_domain disposition + ADR, ledger granularity + noise floor, overflow / unmapped commits

**Selection:** all 4 offered gray areas were selected. Every question resolved to the
recommended option; no free-text overrides, no scope creep raised.

---

## Scouting findings that reframed the discussion

Measured live before the questions were posed — several contradicted the milestone's own
planning assumptions:

| Assumption | Source | Measured |
|---|---|---|
| "~40 substantive commits" | `260727-jkn` map | **100 non-merge commits** (68 CODE / 15 DEPS / 11 CI / 6 DOCS) |
| "7 tool-sandbox refinement PRs" | REQUIREMENTS.md | **20 commits** touch the tool-sandbox surface (8 pure, 12 entangled) |
| deny_domain is a proxy feature | jkn parity matrix | Touches `crates/nono/src/net_filter.rs` — a **core library** file |
| 12 requirements cover the window | ROADMAP SC4 | **~28 code commits map to no requirement**, several security-relevant |

---

## tool-sandbox Entanglement

Evidence presented: a PR-level "defer anything touching tool-sandbox" rule would silently
drop `d5803b99` (port ranges #1398 = PROF-03), `d4927f95` (`@git:*` tokens = PROF-02),
`ea334d2b` (musl build fix), `8a4237f2` (seccomp refactor, 18 files incl. `bindings/c`).

### Q1 — Classification of the 12 entangled commits

| Option | Description | Selected |
|--------|-------------|----------|
| Split — per-commit hunk-level disposition | `split` verdict naming exactly which paths absorb vs defer; most audit effort, provably drops nothing | ✓ |
| Case-by-case by dominant intent | Judge by primary purpose; accepts that non-dominant hunks fail to apply and get dropped | |
| Defer whole, re-derive features independently | Reimplement PROF-02/PROF-03 from scratch in Phase 110 citing upstream as reference | |

**User's choice:** Split — per-commit hunk-level disposition.

### Q2 — Definition of the deferral boundary

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit module set, re-measured each sync | Name `tool-sandbox/`, `command_policy.rs`, `lineage_cgroup.rs`; record that the union equals the directory-only set; re-measure next sync | ✓ |
| Directory prefix only | Correct for this window but holds only by coincidence — `command_policy.rs` already lives outside it | |
| Symbol/feature-level | Define by the `command_policies` feature surface; most accurate semantically, hardest to grep-verify | |

**User's choice:** Explicit module set, re-measured each sync.
**Notes:** the union was measured at 20 either way — recorded as a coincidence to re-test, not a rule to inherit.

### Q3 — Anti-drop proof obligation

| Option | Description | Selected |
|--------|-------------|----------|
| Per-commit residue accounting | Every path in a split/deferred commit marked absorb/defer/noise; unbucketed path = ledger defect | ✓ |
| Re-export + public-surface diff only | Satisfies SC2 literally; catches API-surface drops but not arbitrary dropped hunks | |
| Post-absorb reconciliation in Phase 111 | Diff fork vs upstream after absorbing; catches everything but finds it late | |

**User's choice:** Per-commit residue accounting.

### Q4 — Audit-vs-absorb boundary

| Option | Description | Selected |
|--------|-------------|----------|
| Flag + name paths; absorb phases do the hunks | Keeps the audit an audit; hunk extraction happens where it can be compiled and tested | ✓ |
| Pre-compute hunk-level patches in Phase 108 | Ready-to-apply filtered patches; can't be compile-verified until later, goes stale on reorder | |

**User's choice:** Flag + name paths; absorb phases do the hunks.

---

## deny_domain Disposition + ADR

Evidence presented: the fork's `HostFilter` already has `deny_hosts`, `DENY_HOSTS`, and
`FilterResult::Deny`, so #1374 is a mechanism extension with a caller-supplied list —
ADR-86 is not breached. The real risk is posture: upstream's commit message advertises
"allow everything except these domains… activate the proxy on their own", and
`HostFilter.strict` defaults false ("empty allowlist allows"), so a deny-only profile
becomes default-allow.

### Q1 — Standalone ADR?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — `proj/ADR-108-deny-domain-posture.md` | Mirrors ADR-98/ADR-86; independently citable; survives ledger archival | ✓ |
| No — inline cluster verdict | Lighter; appropriate only if the call is considered predetermined | |
| Defer the ADR to Phase 109 | Risks planning 109 before its central decision is made | |

**User's choice:** Yes — standalone ADR-108.

### Q2 — Settle or frame?

| Option | Description | Selected |
|--------|-------------|----------|
| Settle it in 108, with a recommendation | v3.4 Phase 98 pattern; audit already has the diff in hand | ✓ |
| Frame only — 109 decides with code in hand | Better only if the answer depends on detail invisible in the diff | |

**User's choice:** Settle it in 108.

### Q3 — Substantive posture

| Option | Description | Selected |
|--------|-------------|----------|
| Adapt — deny layer only, never an allowlist substitute | Absorb the feature, reject the "activates proxy on its own" path; deny-only profiles still fail closed | ✓ |
| Adopt verbatim, document the divergence risk | Max parity, lowest future merge friction; accepts default-allow as user choice | |
| Adapt — adopt but force `strict=true` | Strongest fail-closed guarantee; silently diverges for identical config | |
| You decide — recommend in the ADR | Leave open for the audit to recommend | |

**User's choice:** Adapt — deny layer only, never an allowlist substitute.

### Q4 — ADR scope

| Option | Description | Selected |
|--------|-------------|----------|
| `deny_domain` only | SPIFFE and SigV4/sibling-route are additive, no fork-invariant conflict | ✓ |
| Whole NET cluster in one ADR | Fewer files, but buries the posture decision among routine absorbs | |

**User's choice:** `deny_domain` only.

---

## Ledger Granularity + Noise Floor

Evidence presented: 68 CODE / 15 DEPS / 11 CI / 6 DOCS. 38 commits carry
`chore:`/`ci:`/`docs:`/`build:` prefixes but only 32 are non-CODE — prefix and file-set
classification disagree on ~6 commits.

### Q1 — Granularity at 68 code commits

| Option | Description | Selected |
|--------|-------------|----------|
| Cluster-first, per-commit only for will-sync + split | Per-commit rigor where cherry-picks happen, without 100 hand-written rows | ✓ |
| Per-commit for all 100 | Most uniform; 5× ledger volume, most rows read "noise — skip" | |
| Cluster-only, no per-commit rows | Contradicts the carried-forward commits-not-PRs rule | |

**User's choice:** Cluster-first, per-commit only for will-sync + split.

### Q2 — Noise classification method

| Option | Description | Selected |
|--------|-------------|----------|
| File-set based, prefix as cross-check only | Prefix flags disagreements for review; a `chore:` touching source is the dangerous case | ✓ |
| File-set based only | Simple; discards a cheap second signal | |
| Manual review of all 100 subjects | Most accurate, slowest | |

**User's choice:** File-set based, prefix as cross-check only.

### Q3 — The 15 dependency-only commits

| Option | Description | Selected |
|--------|-------------|----------|
| Own cluster — security-relevant, needs `cargo audit` | v3.4 Cluster F precedent (sigstore pin → dual-version cascade); fork carries its own RUSTSEC pin | ✓ |
| Noise — regenerate Cargo.lock post-absorb | Less bookkeeping; loses upstream's explicit pin intent | |
| Split — audit-relevant clustered, dependabot skipped | Accurate; requires per-commit judgment on all 15 | |

**User's choice:** Own cluster — security-relevant.

### Q4 — The 11 CI-only commits

| Option | Description | Selected |
|--------|-------------|----------|
| Cluster as ADAPT-by-default, never adopt verbatim | ADR-100 precedent (upstream assumed a release-please bot the fork lacks) | ✓ |
| Skip entirely — fork CI fully divergent | Would miss portable fixes like the musl build fix `ea334d2b` | |
| Case-by-case with no default posture | No blanket rule | |

**User's choice:** Cluster as ADAPT-by-default.

---

## Overflow / Unmapped Commits

Evidence presented: ~28 non-tool-sandbox code commits map to no v3.6 requirement,
including AWS MiTM proxy auth (#1195), declarative sandboxed OAuth capture + boundary
hardening, NVIDIA procfs mediation (#1284), a trust-policy `predicate` field, the Linux
execute-restriction `Refer` grant (#1397), seccomp supervisor-ancestry for orphaned
descendants, the standalone `nono proxy` command (#1261), and the `allow_vars` env-strip
fix (#1204).

### Q1 — Destination for the ~28 unmapped commits

| Option | Description | Selected |
|--------|-------------|----------|
| Add Phase 112 — Security + Residual Sync | v3.1 Phase 87 precedent; own fork-invariant review rather than riding in a release phase | ✓ |
| Fold into Phase 111 | Would make 111 by far the largest phase and mix security review with a release cut | |
| Split — security to a new phase, rest to v3.7 | Only the security subset absorbed this milestone | |
| Defer all 28 to a v3.7 backlog | Smallest change; leaves known upstream security fixes unabsorbed another cycle | |

**User's choice:** Add Phase 112 — Security + Residual Sync.

### Q2 — Handling the unsatisfiable SC4

| Option | Description | Selected |
|--------|-------------|----------|
| Ledger reports the gap; roadmap amended as a Phase 108 output | Under-scoping is a legitimate audit finding; amendment gated on operator approval | ✓ |
| Amend the roadmap now, before planning 108 | Cleaner ordering; commits to a shape before the audit has run | |
| Relax SC4 to "…or the deferred backlog" | Minimal disruption; risks normalizing a growing backlog | |

**User's choice:** Ledger reports the gap; roadmap amended as a Phase 108 output.

### Q3 — Security flag in the ledger

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — `security-relevant` column alongside `windows-touch` | Makes the security subset queryable; gives Phase 112 a precise work-list | ✓ |
| No — cluster membership conveys it | Redundant if security commits cluster together; cross-cluster fixes get harder to spot | |

**User's choice:** Yes — add the column.

### Q4 — Precision of the requirement mapping

| Option | Description | Selected |
|--------|-------------|----------|
| Hand-verify every code commit's mapping | Automated split is a hypothesis only; false-positive "mapped" is the dangerous direction | ✓ |
| Spot-check mapped, hand-verify unmapped only | Faster; a mis-mapped commit stays invisible | |

**User's choice:** Hand-verify every code commit's mapping.

---

## Claude's Discretion

- Cluster naming and count within the cluster-first scheme (D-14).
- Ledger table layout and column ordering, provided `windows-touch`, `security-relevant`, disposition, and residue accounting are all present.
- Internal structure of ADR-108, provided both options and their fork-invariant impact are shown before landing on the recommendation.

## Deferred Ideas

- tool-sandbox subsystem + its 7 refinement PRs → **v3.7**; ledger records the full 20-commit surface so v3.7 starts from real data.
- Windows `tool-sandbox/platform/windows.rs` driver → **v3.7** (needs the SCM_RIGHTS→handle-passing + peer-auth spike and an adopt-vs-formalize ADR).
- WFP daemon-path-only reachability / filter-leak hardening — fork-internal, tracked separately.
- **Gray areas offered but not discussed** (user chose "ready for context"): the `platform_overrides` adoption call, which the `260727-jkn` map pre-declares as full-adopt; how the carve-out re-touch check scales at 100 commits; whether the SPIFFE/SPIRE `testdata/spire/` fixtures and `scripts/spiffe-mock-server.py` come across with NET-02; and how CORE-02's "fork is already ahead" claim gets verified rather than asserted.
- **Reviewed todos, not folded:** `20260611-msi-vcredist-prereq.md` and `20260611-poc-cert-broker-clean-host.md` — both matched at score 0.2 on the keyword "phase" only; host-gated v3.5 distribution items owned by v3.5 Phase 106.
