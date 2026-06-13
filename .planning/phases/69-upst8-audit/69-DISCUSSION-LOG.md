# Phase 69: UPST8 Audit - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-12
**Phase:** 69-UPST8 Audit
**Areas discussed:** Audit range / upstream-tag collision, macOS-overlap scoping, discussion depth

---

## Audit range / upstream-tag collision

Initial framing assumed the roadmap SC range `v0.60.0..v0.61.2`. User redirected: upstream
`always-further/nono` is now at **v0.62.0** and the audit should cover everything to that release.

Live verification during discussion:
- `git ls-remote --tags upstream` → upstream `v0.62.0` = `52809dda…`; upstream's highest is v0.62.0
  (no upstream v0.62.1/v0.62.2).
- Local tags `v0.62.0/.1/.2` are the **fork's** releases (`3c5e9025`/`78bcdca8`/`93a7390e`) on a
  divergent history — `git rev-list --count v0.61.2..v0.62.0` returned a garbage **1889** proving
  the collision.
- Real upstream `v0.60.0..v0.62.0` (by SHA) = **14** non-merge commits; **+3** beyond v0.61.2.

| Option | Description | Selected |
|--------|-------------|----------|
| Lock to v0.61.2, defer newer | Keep SC range, defer v0.62.0 to UPST9 | |
| Extend to upstream's latest (v0.62.0) | Audit full backlog to upstream's true highest, by SHA | ✓ |

**User's choice:** Extend to v0.62.0 — "audit everything to that release."
**Notes:** Resolved to range `v0.60.0..52809dda` (D-01); fork-tag landmine locked as D-02 (use the
SHA, never the local `v0.62.0` tag); SC range now needs a +3 note.

---

## macOS-overlap scoping

Phase 63 already dispositioned the **macOS** slice of `v0.57.0..v0.61.2`; the new range overlaps it
on `v0.60.0..v0.61.2` and extends past it on the 3-commit tail `v0.61.2..v0.62.0` (which Phase 63
never saw).

| Option | Description | Selected |
|--------|-------------|----------|
| Cross-ref 63 + flag fresh tail | Audit non-macOS delta fresh in overlap (pointer to 63 rows; macOS-only → out-of-scope w/ pointer); for the 3 tail commits flag any macOS-relevant one as "needs future macOS top-up" | ✓ |
| Non-macOS only, ignore macOS gap | Strictly non-macOS; don't track macOS gap in tail | |
| Include macOS for tail only | Cross-ref overlap, but fully audit both surfaces for the 3 fresh tail commits | |

**User's choice:** Cross-ref 63 + flag fresh tail (D-04).
**Notes:** Closes the crack where a macOS-only tail commit would be unaudited by both phases.

---

## Discussion depth

| Option | Description | Selected |
|--------|-------------|----------|
| Enough — write CONTEXT, go to plan | Remaining items locked by Phase 54/63 precedent | ✓ |
| Discuss disposition/cadence details | Deeper dive on ADR-cadence, P1/P2 tiering, split routing | |

**User's choice:** Enough — write CONTEXT, proceed to plan.

---

## Claude's Discretion

- Single-plan vs multi-plan structure (almost certainly single, mirroring Phase 54).
- Will-sync P1/P2 tiering for Phase 70 priority (optional).
- Cluster grouping granularity for the 14 commits.

## Deferred Ideas

- UPST9: any upstream tag newer than v0.62.0 surfaced at re-fetch → defer.
- macOS top-up for any macOS-relevant `v0.61.2..v0.62.0` tail commit (non-macOS scope excludes it).
- The cherry-pick wave itself → Phase 70 (UPST8-02).
- 3 reviewed-not-folded todos (belong to Phases 67/68).
