---
quick_id: 260619-duw
title: Scope upstream v0.62..v0.64 delta and plant SEED-006
date: 2026-06-19
---

# Quick Task 260619-duw: Scope upstream `v0.62..v0.64` delta → SEED-006

## Task

Upstream `always-further/nono` is now at `v0.64.0`. Scope the delta from `v0.62.0`
(commit list, changed files, new/modified functions) and plant a `SEED-006` seed
file documenting it so the future UPST9 upstream-sync milestone has a function-level
inventory ready.

## Approach (research-then-document; no code change)

1. Verify premise against ground truth — confirm upstream tags + the `v0.62.0..v0.64.0`
   window via `gh api repos/always-further/nono/compare/...`.
2. Extract the real new/modified function signatures from the compare patches
   (filter dependabot/docs/merge noise; group substantive work by theme).
3. Write `.planning/seeds/SEED-006-upst9-v0.62-v0.64-sync-window.md` matching the
   existing SEED template, with commit SHAs, PR numbers, file paths, and a
   fork-conflict-risk assessment against the library/CLI boundary invariant.

## Done

- [x] `SEED-006` exists in `.planning/seeds/` with themed new/modified-function inventory
- [x] Library-boundary conflict risk (audit + diagnostics moved into core crate) flagged
- [x] Breadcrumbs link the divergence-ledger process + relevant memories
