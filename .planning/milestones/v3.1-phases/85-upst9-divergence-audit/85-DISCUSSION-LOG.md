# Phase 85: UPST9 Divergence Audit - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-19
**Phase:** 85-upst9-divergence-audit
**Areas discussed:** Ledger granularity, Disposition pre-commitment, Diff-inspection scope, Noise-commit handling

---

## Ledger granularity

| Option | Description | Selected |
|--------|-------------|----------|
| Cluster-level + commit inventory | One disposition + risk verdict per theme A–M, with a nested SHA inventory of every commit in the cluster. Matches Phase 42/47/48 shape. | ✓ |
| Cluster-level, sub-disposition where split | Default to one disposition per cluster but allow per-commit sub-dispositions inside mixed-fate clusters (e.g. M). | |

**User's choice:** Cluster-level + commit inventory
**Notes:** Mixed-fate themes are handled via the `split` cluster disposition + per-commit inventory annotations (D-02), so no separate per-commit disposition column is needed.

---

## Disposition pre-commitment

| Option | Description | Selected |
|--------|-------------|----------|
| Pre-lean the obvious, audit confirms | Record leanings for the clear clusters (C→will-sync; F & M→split/diff-careful) and let the diff-inspection confirm or overturn. | ✓ |
| Only A&B locked, rest fully empirical | Lock only A&B; every other cluster left genuinely open until diff-inspection determines it. | |

**User's choice:** Pre-lean the obvious, audit confirms
**Notes:** A&B remain locked to will-sync/adopt-upstream (milestone Key Decision). Leanings recorded in D-04 MUST be validated by the audit, not assumed.

---

## Diff-inspection scope

| Option | Description | Selected |
|--------|-------------|----------|
| Targeted: shared-surface clusters | Full actual-diff re-export inspection for A, B, diagnostic-touching surfaces, and F (proxy TLS); --name-only sufficient for additive D/H/I/K + dep bumps. | ✓ |
| Exhaustive: every cluster | Actual-diff re-export inspection for all A–M regardless of apparent isolation. | |

**User's choice:** Targeted: shared-surface clusters
**Notes:** Ledger must state per-cluster which inspection depth was applied so hazard-closure completeness is auditable (D-05).

---

## Noise-commit handling

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit excluded-noise bucket | Dedicated ledger section: exclusion filter criteria + count + SHAs/ranges excluded as noise. Completeness claim independently verifiable. | ✓ |
| Silent filter to substantive-only | Ledger contains only substantive commits; noise filtered without enumeration. | |

**User's choice:** Explicit excluded-noise bucket
**Notes:** Supports the "every substantive commit classified" completeness claim being independently verifiable (D-06).

---

## Claude's Discretion

- Exact ledger table column layout / format (follow Phase 42/47/48 convention as reconstructable; prior ledger files archived out of the live tree).
- Bucketing/ordering of excluded noise SHAs (ranges vs enumerated).

## Deferred Ideas

- Full boundary-convergence ADR → Phase 86 (BND-03).
- Actual cherry-pick / code relocation → Phases 86–89 per disposition.
- Crate version leapfrog to ≥ 0.65.0 → release-time, post-sync.
- Four weak-match unrelated todos reviewed and not folded (MSI VC++ prereq, poc-cert-broker, macOS rlimit, phase-83 code-review) — left for their own cadence.

## Pre-locked (not discussed — carried from roadmap/seed/milestone)

- Window = `v0.62.0..v0.64.0`; upstream highest tag verified `v0.64.0` (no v0.65.0 yet).
- Themes A & B → will-sync / adopt-upstream (milestone Key Decision; changes the policy-free-library boundary).
- Risk dimensions = 5 standard (security, windows, maintenance, divergence, contributor).
- Diff-inspect actual diffs, not `--name-only` (closes feedback_cluster_isolation_invalid).
