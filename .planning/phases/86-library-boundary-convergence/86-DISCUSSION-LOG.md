# Phase 86: Library-Boundary Convergence - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-19
**Phase:** 86-library-boundary-convergence
**Areas discussed:** Port mechanism, Windows diagnostic reconciliation, CLI boundary line, Test relocation

---

## Port mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| Manual re-port to end-state | Hand-write fork files to match upstream end-state; cleaner, loses commit provenance | |
| Cherry-pick + resolve conflicts | git cherry-pick the 8 commits in locked order, resolve conflicts; preserves provenance, heavy churn on B | ✓ |
| Hybrid: cherry-pick A, manual B | Mechanism per-theme by conflict density | |

**User's choice:** Cherry-pick + resolve conflicts
**Notes:** Resolve toward upstream end-state on shared surfaces; honor intra-B ordering (4ad8ba92 → … → a6aa5995). Windows conflict hunks get the preserve-and-bridge resolution.

---

## Windows diagnostic reconciliation

| Option | Description | Selected |
|--------|-------------|----------|
| Preserve-and-bridge (conservative) | Keep fork Windows paths; bridge to new model only at surface; lowest regression risk | ✓ |
| Route Windows through new model | Rewrite Windows consumers end-to-end; max convergence, higher regression risk | |
| Defer Windows decision to research | Lock no-regression; let researcher recommend bridge-vs-route per-path | |

**User's choice:** Preserve-and-bridge (conservative)
**Notes:** SC#3 requires no regression in Windows denial output, not Windows convergence. Accept short-term duplication; this is the milestone's focus platform.

---

## CLI boundary line (BND-03 ADR substance)

| Option | Description | Selected |
|--------|-------------|----------|
| Match upstream's line exactly | Move what upstream moved; leave CLI-side what upstream leaves; ADR documents upstream's line verbatim | ✓ |
| Upstream line + fork carve-outs | Adopt upstream's boundary but carve out fork-specific bits per item | |
| Defer exact line to research/planning | Lock intent + ADR requirement; map the split during research | |

**User's choice:** Match upstream's line exactly
**Notes:** Minimizes the reconciliation surface for the next upstream sync. The one sanctioned deviation is D-02's Windows carve-out, documented in the ADR.

---

## Test relocation

| Option | Description | Selected |
|--------|-------------|----------|
| Move tests to core with the code | Audit unit tests move into crates/nono/src/audit.rs; CLI keeps thin wrapper/integration tests | ✓ |
| Keep testing through CLI wrappers | Leave tests CLI-side, exercise core indirectly | |
| Mirror upstream's test placement | Put fork tests wherever upstream put them | |

**User's choice:** Move tests to core with the code
**Notes:** Tests live with unit-under-test, prove core behavior directly; aligns with upstream's placement (low future-sync friction).

---

## Claude's Discretion

- Per-hunk cherry-pick conflict-resolution tactics; whether to squash or retain the 8 cherry-picked commits (provenance is the goal, final commit shape is executor's call within DCO convention).
- ADR file location/format (follow fork's existing ADR convention).
- Precise field-by-field shape of the Windows surface bridge (research/planner to map).

## Deferred Ideas

- Full Windows convergence (route denial output end-to-end through the new core model) — deferred; D-02 is bridge-only. A future phase could converge once the bridge is proven stable.
- Themes C / D–M / F absorption + crate-version leapfrog — Phases 87/88/89 + release-time.
- Four weak-match unrelated todos (MSI VC++ prereq, POC-cert broker, Phase 83 code-review, macOS rlimit) reviewed, not folded — same set Phase 85 rejected.
