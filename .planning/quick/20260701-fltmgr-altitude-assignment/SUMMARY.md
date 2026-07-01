---
type: quick
slug: fltmgr-altitude-assignment
created: 2026-07-01
completed: 2026-07-01
status: complete
commit: e8ea5508
---

# Summary: Wire official Microsoft altitude 377813.5 for nono-fltmgr.sys

**Status: complete ✓**

## What was requested

The Microsoft File System Filter team (fsfcomm@microsoft.com) allocated altitude
**377813.5** for the `nono-fltmgr.sys` minifilter. Do the required updates.

## What was found

The **live driver artifacts were already wired** in commit `2041fc62` (same day):
`nono-fltmgr.inf` (`Instance1.Altitude = "377813.5"`), `DESIGN.md` altitude table
(ASSIGNED), and `drivers/README.md`. The old placeholder `365678` was fully purged
from `drivers/`. Verified there is **no** other artifact carrying an altitude value
needing change — `.c`/`.h`, `crates/nono-fltmgr-client/`, `scripts/`, `tests/`, and
`docs/` are clean (the `.c` FLT_REGISTRATION carries no altitude; it comes from the
INF/registry).

Two **tracking** references still described the old PENDING state and were reconciled.

## What was changed (commit `e8ea5508`)

1. `.planning/architecture/adr-65-minifilter-go-no-go.md` §5 — dated amendment note
   recording the 377813.5 assignment; the spike's historical use of 365678 (Phase 64)
   left intact (ADR amended in place per D-06 precedent).
2. `.planning/STATE.md` Blockers/Concerns — minifilter-altitude approval marked
   RECEIVED 2026-07-01; safety invariant (repo stays PUBLIC, no `build_notes/`/`.gsd/`
   staged before push, LOCAL-only tags, operator-gated push) preserved verbatim.

## Operator decision — go-private CANCELLED (2026-07-01)

- **PUBLIC → PRIVATE repo flip: CANCELLED, not deferred.** Altitude approval was the
  *gate condition* for the deferred go-private decision (memory: go-private commit
  `74a47742` was cancelled pending approval). With the gate cleared, the operator
  decided the repo will **remain PUBLIC permanently**; the go-private idea is retired.
  Recorded in `STATE.md` Blockers/Concerns (commit below) and in persistent memory.
  The operational invariant persists *because* the repo stays public: never stage
  `build_notes/`/`.gsd/` before a push.

## Not touched (intentional)

- `STATE.md` line 64 — under a "historical" decisions table; "pending" was true at
  v3.3 time.
- `DESIGN.md` line 107 dangling ref to `63-altitude-request.md` — pre-existing,
  unrelated to the altitude value.

## Verification

- `git grep 365678` → no hits in `drivers/` (placeholder fully purged).
- Assigned value `377813.5` present in INF, DESIGN.md, README.md, and now ADR-65.
- No source/test/CI artifact asserts an altitude value, so no build/test gate applies
  to this docs-only change.
