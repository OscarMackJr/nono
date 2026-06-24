---
quick_id: 260624-q9j
description: Fix red Docs Checks for orphan windows-win-1706-option-1-workstream.mdx
date: 2026-06-24
status: complete
commit: 3475b470
---

# Quick Task 260624-q9j — SUMMARY

## Outcome

`Docs Checks` CI red resolved by **force-adding the already-in-nav page** to git
(not by excluding the dir, as the todo originally proposed — see PLAN for the
corrected diagnosis).

## What changed

- `git add -f docs/cli/development/windows-win-1706-option-1-workstream.mdx`
  (674 lines), committed as `3475b470`.
- No change to `docs.json` (the nav entry was already present at line 104).
- No change to the CI scan (the dir stays under nav-integrity coverage).

## Key finding (verify-the-hypothesis)

The todo's root cause was **backwards**. Production check (`ci.yml` → "Verify docs
nav pages exist") is nav→file, not file→nav. The page was in nav but untracked in
git (gitignored dir, never `git add -f`'d). Surfaced the discrepancy to the
operator before acting; operator chose force-add.

## Verification

- After force-add, `git ls-files` includes the page.
- Local simulation of the CI nav→file check over `git ls-files docs`: **0 missing
  nav pages** → `Docs Checks` will go green in CI.
- Commit carries DCO sign-off.

## Commit

- `3475b470` — `docs(cli): track windows-win-1706-option-1-workstream.mdx (fix red Docs Checks)`
