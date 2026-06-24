# TODO: Docs Checks — orphan `windows-win-1706-option-1-workstream.mdx` not in `docs.json` nav

**Captured:** 2026-06-24 (PR #12 CI triage)
**Severity:** low — docs-only; `Docs Checks` CI check is red
**Source:** PR #12 `Docs Checks` job; pre-existing (fails on `main` too — NOT a v3.x regression)

## Problem
`Docs Checks` fails because a doc page exists on disk but is absent from `docs.json` navigation:
```
Missing page from docs.json navigation: docs/cli/development/windows-win-1706-option-1-workstream.mdx
```
The check scans `docs/**/*.mdx` and requires each to be referenced in `docs.json`. `docs/cli/development/` is also `.gitignore`'d-but-tracked (editing there needs `git add -f`).

## Fix (pick one)
- **Remove** the stale dev doc if it's obsolete (it's an old Windows-1706 workstream note), OR
- **Add** it to `docs.json` navigation, OR
- **Exclude** `docs/cli/development/**` from the Docs Checks navigation scan if dev docs are intentionally unpublished.

Decide based on whether `docs/cli/development/` is meant to be part of the published docs site at all.

## Acceptance
`Docs Checks` CI check is green.
