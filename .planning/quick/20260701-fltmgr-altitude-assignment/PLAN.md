---
type: quick
slug: fltmgr-altitude-assignment
created: 2026-07-01
status: in-progress
---

# Quick Task: Wire official Microsoft altitude 377813.5 for nono-fltmgr.sys

## Task

The Microsoft File System Filter team (fsfcomm@microsoft.com) allocated altitude
**377813.5** for the `nono-fltmgr.sys` minifilter. Complete the required updates so
every artifact reflects the assigned (no longer PENDING) altitude.

## Investigation findings

The live driver artifacts were **already wired** by commit `2041fc62` (2026-07-01):
- `drivers/nono-fltmgr/nono-fltmgr.inf` — `Instance1.Altitude = "377813.5"`
- `drivers/nono-fltmgr/DESIGN.md` — altitude table reframed PENDING → ASSIGNED
- `drivers/README.md` — official altitude 377813.5

The old placeholder `365678` is fully purged from `drivers/`. Verified clean:
`drivers/nono-fltmgr/*.c`, `*.h`, `crates/nono-fltmgr-client/`, `scripts/`, `tests/`,
`docs/` hold **no** altitude value needing change (the `.c` FLT_REGISTRATION carries
no altitude — it comes from the INF/registry, correctly).

Two references still describe the OLD `PENDING / not-yet-requested` state:
1. `.planning/architecture/adr-65-minifilter-go-no-go.md` §5 "Altitude" — says the
   official assignment is "PENDING / not-yet-requested" and "A production milestone
   must request and receive an assigned altitude."
2. `.planning/STATE.md` line 105 (Blockers/Concerns) — "(minifilter-altitude approval
   pending)".

## Plan

1. **ADR-65 §5** — append a dated amendment note recording the 377813.5 assignment.
   Do NOT rewrite the spike's historical use of 365678 (the spike genuinely used it);
   ADR is a decision record, amended in place per the D-06 precedent.
2. **STATE.md line 105** — update the stale parenthetical to "approval received
   2026-07-01" while PRESERVING the safety invariant (repo stays PUBLIC, no
   `build_notes/`/`.gsd/` staged before push, tags LOCAL, push operator-gated).

## Explicitly out of scope (flagged to operator)

- **Going private.** The altitude approval was the *gate condition* for the deferred
  PUBLIC → PRIVATE decision. Flipping repo visibility is a consequential, hard-to-reverse,
  operator-gated action — NOT a "required update" for the altitude itself. Left for an
  explicit operator decision.
- `.planning/STATE.md` line 64 lives under a "**historical**" decisions table; its
  "pending" wording was true at v3.3 time — not rewritten.
- `DESIGN.md` line 107 dangling ref to `63-altitude-request.md` (archived phase artifact)
  is pre-existing and unrelated to the altitude value.
