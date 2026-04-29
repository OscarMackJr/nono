---
quick_id: 260428-rsu
slug: refresh-stack-onto-upstream-tip
plan_status: deferred — runbook ready, awaiting trigger conditions (see CONTEXT.md `<deferred>` block)
locked_decisions:
  - timing: wait-for-785-and-sigstore
  - phase22_disposition: hold-local-until-merge
  - rebase_shape: re-squash
  - conflict_authority: stop-at-each
created: 2026-04-28
context_file: 260428-rsu-CONTEXT.md
---

# Quick Task 260428-rsu — Plan: Refresh stack onto upstream's new tip

**Mode:** quick (deferred runbook)
**Description:** Refresh PRs 725 (v2.0-pr) + 726 (v2.1-pr) on `always-further/nono` onto upstream `main` (currently `30c0f76e v0.43.0`).

This plan is a **runbook** to execute when a trigger condition from `260428-rsu-CONTEXT.md` `<deferred>` fires. It is NOT executed today.

## Must-haves (locked from CONTEXT.md)

1. PRs 725 + 726 transition `mergeStateStatus: DIRTY` → `CLEAN` after rebase
2. The 1-commit-per-milestone shape is preserved (re-squash decision)
3. DCO sign-off authorship (`oscarmackjr-twg`) preserved on each squashed commit
4. NO conflicts auto-resolved — every conflict surfaces to user before resolution
5. Phase 22 work stays on local `main` only — NOT included in either rebased PR
6. `windows-squash` source-of-truth branch is untouched
7. Old origin tips (`origin/v2.0-pr@3ec7fdf`, `origin/v2.1-pr@b95a9e4`) replaced via `--force-with-lease` (NOT plain `--force`)

## Execution prerequisites (verify before starting)

```bash
# 1. Confirm a trigger from CONTEXT.md `<deferred>` fired:
gh pr view 785 --repo always-further/nono --json state,mergedAt
gh pr view 777 --repo always-further/nono --json state,mergedAt
gh pr view 778 --repo always-further/nono --json state,mergedAt
gh pr view 725 --repo always-further/nono --json reviews
gh pr view 726 --repo always-further/nono --json reviews

# 2. Working tree clean
git status --short    # must be empty (no uncommitted, no unstaged)

# 3. Upstream remote exists and is up to date
git remote -v | grep upstream || git remote add upstream https://github.com/always-further/nono.git
git fetch upstream main

# 4. Local v2.0-pr / v2.1-pr branches track origin
git fetch origin v2.0-pr v2.1-pr
git branch -f v2.0-pr-staging origin/v2.0-pr
git branch -f v2.1-pr-staging origin/v2.1-pr

# 5. Capture pre-rebase state for diagnostic comparison
git rev-parse origin/v2.0-pr origin/v2.1-pr upstream/main main > /tmp/pre-rebase-state.txt
```

## Task 1: Rebase v2.0-pr onto upstream/main (re-squash)

**Files / surfaces:**
- Branch: `v2.0-pr-staging` (working branch)
- Target base: `upstream/main` (latest)
- Old base: parent of `origin/v2.0-pr@3ec7fdf` (or whatever upstream/main was when 725 was first squashed)

**Action:**

```bash
# 1.1 — Snapshot the v2.0 squashed-commit message for re-application
git log -1 --format='%B' origin/v2.0-pr > /tmp/v2.0-pr-msg.txt
# Extract authorship + DCO sign-off
git log -1 --format='%an <%ae>' origin/v2.0-pr > /tmp/v2.0-pr-author.txt

# 1.2 — Get the diff that v2.0-pr applies (the entire squashed delta)
# This is the canonical "what does PR 725 add to its base" — independent of base movement.
git diff "$(git merge-base origin/v2.0-pr upstream/main)..origin/v2.0-pr" > /tmp/v2.0-pr.patch

# 1.3 — Reset working branch to upstream/main and apply
git checkout -B v2.0-pr-rebase-260428 upstream/main
git apply --3way --index /tmp/v2.0-pr.patch
# ⚠️ CONFLICTS WILL FIRE HERE — per CONTEXT.md decision, STOP at each.
# When conflict surfaces:
#   git status                         # list conflicted files
#   git diff <file>                    # see ours-vs-theirs
#   <surface to user, await decision>
#   <apply user's decision>
#   git add <file>
# Continue when all resolved.

# 1.4 — Single squashed commit. Recompose message:
#   - Original v2.0-pr commit body (from /tmp/v2.0-pr-msg.txt)
#   - Append `Upstream-tag: v0.43.0` (or whichever upstream tip was used)
#   - Append `Upstream-commit: <upstream/main SHA>` if appropriate
#   - DCO sign-off line preserved verbatim
git commit --author="$(cat /tmp/v2.0-pr-author.txt)" --file=/tmp/v2.0-pr-msg.txt
# Then `git commit --amend` to inject the upstream trailer block.

# 1.5 — Verify: single commit ahead of upstream/main
[ "$(git rev-list --count upstream/main..HEAD)" = "1" ] || { echo "FAIL: not single commit"; exit 1; }
```

**Verify:**
- `git log upstream/main..v2.0-pr-rebase-260428 --oneline` shows exactly 1 commit
- `git diff origin/v2.0-pr v2.0-pr-rebase-260428 -- crates/ docs/` shows ONLY upstream-drift-driven changes (no logic regressions vs the original PR 725)
- DCO sign-off line present in commit message body

**Done when:** v2.0-pr-rebase-260428 holds 1 commit on top of upstream/main with PR 725's full delta cleanly merged.

## Task 2: Rebase v2.1-pr onto v2.0-pr-rebase-260428 (re-squash)

**Files / surfaces:**
- Branch: `v2.1-pr-staging` (working branch)
- Target base: `v2.0-pr-rebase-260428` (output of Task 1)

**Action:** Mirror Task 1 with v2.1-pr inputs.

```bash
git log -1 --format='%B' origin/v2.1-pr > /tmp/v2.1-pr-msg.txt
git log -1 --format='%an <%ae>' origin/v2.1-pr > /tmp/v2.1-pr-author.txt

# Diff = v2.1-only delta (NOT v2.0+v2.1 — anchor at the v2.0/v2.1 boundary)
git diff "$(git merge-base origin/v2.1-pr origin/v2.0-pr)..origin/v2.1-pr" > /tmp/v2.1-pr.patch

git checkout -B v2.1-pr-rebase-260428 v2.0-pr-rebase-260428
git apply --3way --index /tmp/v2.1-pr.patch
# CONFLICTS — same stop-at-each protocol as Task 1.

git commit --author="$(cat /tmp/v2.1-pr-author.txt)" --file=/tmp/v2.1-pr-msg.txt
# Amend to inject Upstream-tag / Upstream-commit trailers if appropriate.

[ "$(git rev-list --count v2.0-pr-rebase-260428..HEAD)" = "1" ] || { echo "FAIL: not single commit"; exit 1; }
```

**Verify:**
- `git log v2.0-pr-rebase-260428..v2.1-pr-rebase-260428 --oneline` shows exactly 1 commit
- `git diff origin/v2.1-pr v2.1-pr-rebase-260428 -- crates/ docs/` shows ONLY upstream-drift-driven changes
- DCO sign-off preserved

**Done when:** v2.1-pr-rebase-260428 holds 1 commit on top of v2.0-pr-rebase-260428 with PR 726's full delta cleanly merged.

## Task 3: Smoke-test both rebased branches

```bash
# 3.1 — Build clean on each branch
git checkout v2.0-pr-rebase-260428
make build || { echo "FAIL: v2.0-pr-rebase build broken"; exit 1; }

git checkout v2.1-pr-rebase-260428
make build || { echo "FAIL: v2.1-pr-rebase build broken"; exit 1; }

# 3.2 — Run the lightweight test suite (full ci optional, gated by user preference)
git checkout v2.1-pr-rebase-260428
cargo test --workspace --lib   # ~3-5 min on local Windows host

# 3.3 — Optional: full make ci if user requests it
# make ci   # disabled by default; user can request explicitly
```

**Done when:** both branches build clean. Running tests only on v2.1-pr-rebase (it is a strict superset of v2.0).

## Task 4: Force-push and verify PR state

**⚠️ STOP — final user confirmation gate before force-push.** Per LOCKED conflict-authority decision, force-push is a "stop and confirm" point even if conflicts were clean.

Surface to user:
- 1-commit diff for PR 725: `git log --stat origin/v2.0-pr..v2.0-pr-rebase-260428`
- 1-commit diff for PR 726: `git log --stat origin/v2.1-pr..v2.1-pr-rebase-260428`
- Pre-rebase state: `cat /tmp/pre-rebase-state.txt`

After user confirmation:

```bash
# 4.1 — Force-push v2.0-pr (uses --force-with-lease for collision protection)
git push origin v2.0-pr-rebase-260428:v2.0-pr --force-with-lease=v2.0-pr:"$(grep ^origin/v2.0-pr /tmp/pre-rebase-state.txt | awk '{print $2}')"

# 4.2 — Force-push v2.1-pr
git push origin v2.1-pr-rebase-260428:v2.1-pr --force-with-lease=v2.1-pr:"$(grep ^origin/v2.1-pr /tmp/pre-rebase-state.txt | awk '{print $2}')"

# 4.3 — Verify PR mergeStateStatus flipped CLEAN
sleep 30   # GitHub needs a moment to recompute
gh pr view 725 --repo always-further/nono --json mergeStateStatus,mergeable,number
gh pr view 726 --repo always-further/nono --json mergeStateStatus,mergeable,number
# Expected: mergeStateStatus="CLEAN" or "BLOCKED" (waiting for review approval), mergeable="MERGEABLE"
```

**Done when:** both PRs report `mergeable: MERGEABLE` (mergeStateStatus may stay BLOCKED awaiting review approval, but conflicts are gone).

## Task 5: Cleanup + reply on PR threads

```bash
# 5.1 — Delete temporary branches
git branch -D v2.0-pr-staging v2.1-pr-staging v2.0-pr-rebase-260428 v2.1-pr-rebase-260428
rm -f /tmp/v2.0-pr.patch /tmp/v2.1-pr.patch /tmp/v2.0-pr-msg.txt /tmp/v2.1-pr-msg.txt /tmp/v2.0-pr-author.txt /tmp/v2.1-pr-author.txt /tmp/pre-rebase-state.txt

# 5.2 — Post a reply on each PR noting the rebase + new tip + which upstream PRs/tag we re-anchored on
gh pr comment 725 --repo always-further/nono --body "Rebased onto \`upstream/main@${UPSTREAM_TIP}\` (v0.43.0). Re-anchored after #785 + sigstore bumps landed. No logic changes; only upstream-drift conflicts resolved. Re-running tests locally; ready for further review."
gh pr comment 726 --repo always-further/nono --body "Rebased onto v2.0-pr's new tip after upstream refresh. Same as #725 — drift-only conflicts resolved, logic unchanged."
```

**Done when:** branches cleaned, PR comments posted, both PRs visible as MERGEABLE on upstream.

## Out of scope for this task

- Phase 22 work (`main` branch local commits) — held local per LOCKED phase22_disposition decision
- Submitting v2.2-pr (separate quick task to be created when 725+726 merge)
- Any code logic changes — this task is purely upstream-drift conflict resolution
- Touching `windows-squash` (the source-of-truth branch — read-only here)
