---
quick_id: 260428-rsu
slug: refresh-stack-onto-upstream-tip
description: Refresh the stack (PRs 725 + 726) onto upstream's new tip before a human reviewer engages
started: 2026-04-28
status: deferred
deferred_until: trigger-fires (see CONTEXT.md `<deferred>` block)
runbook: 260428-rsu-PLAN.md
---

# Quick Task 260428-rsu — Summary: DEFERRED

**Status:** Deferred (NOT executed). Decisions captured + runbook ready.

## Outcome

`/gsd-quick --discuss` surfaced 4 gray areas; user locked the recommended option on each:

| Decision | Locked answer |
|---|---|
| Timing vs upstream PRs | Wait for #785 (claude-pack-migration) + #777/#778 (sigstore bumps) to land |
| Phase 22 stack disposition | Hold Phase 22 + 24 work on local `main` until 725+726 merge |
| Rebase shape | Re-squash on upstream/main (preserve 1-commit-per-milestone) |
| Conflict resolution authority | Stop at each conflict; surface to user before resolving |

The "Wait" timing decision means the rebase does NOT fire today. Instead, this task captures:
- `260428-rsu-CONTEXT.md` — locked decisions + trigger conditions + critical files-to-watch list
- `260428-rsu-PLAN.md` — 5-task runbook with concrete git commands, conflict-stop protocol, force-push gate

## Trigger conditions (from CONTEXT.md `<deferred>`)

The runbook executes when ANY of these fires:

1. **PR #785 merges on upstream** — preferred trigger; biggest overlap (PROF-04 + PKG-01..04)
2. **PRs #777 and #778 both merge on upstream** — sigstore bump trigger
3. **A human reviewer on always-further/nono engages** — submits CHANGES_REQUESTED or APPROVED on PR 725 or PR 726
4. **2026-05-12** — soft deadline; if no triggers fire, escalate (re-evaluate timing decision)

When triggered, run:

```
/gsd-quick resume refresh-stack-onto-upstream-tip
```

The resume path loads CONTEXT.md (decisions stay LOCKED) and reads PLAN.md as the runbook. The decisions made on 2026-04-28 do not need to be revisited unless triggers go stale.

## Files produced

- `260428-rsu-CONTEXT.md` — decisions locked; this is the durable artifact
- `260428-rsu-PLAN.md` — 5-task git runbook (Tasks 1-5: rebase v2.0-pr, rebase v2.1-pr, smoke-test, force-push, cleanup)
- `260428-rsu-SUMMARY.md` — this file

## No commits to source code

Per the deferred status, no source-code commits land in this quick task. Only `.planning/quick/260428-rsu-*` artifacts.

## Decisions deferred for the resume run (NOT pre-locked)

These are intentionally Claude's discretion at execute time, since they're mechanical and trigger-specific:

- Branch naming for working branches (`v2.0-pr-rebase-{date}` etc.)
- Cherry-pick vs `git apply --3way` for the squash regeneration mechanism
- Whether to delete old origin tips post-push (default: yes, git reflog covers archaeology)

## Watch-list (no action — informational)

Run any time to see if a trigger has fired:

```bash
gh pr view 785 --repo always-further/nono --json state,mergedAt
gh pr view 777 --repo always-further/nono --json state,mergedAt
gh pr view 778 --repo always-further/nono --json state,mergedAt
gh pr view 725 --repo always-further/nono --json reviews | jq '.reviews[] | select(.state == "APPROVED" or .state == "CHANGES_REQUESTED")'
gh pr view 726 --repo always-further/nono --json reviews | jq '.reviews[] | select(.state == "APPROVED" or .state == "CHANGES_REQUESTED")'
```
