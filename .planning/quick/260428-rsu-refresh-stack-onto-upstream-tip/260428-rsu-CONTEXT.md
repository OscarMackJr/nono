---
quick_id: 260428-rsu
slug: refresh-stack-onto-upstream-tip
description: Refresh the stack (PRs 725 + 726) onto upstream's new tip before a human reviewer engages
gathered: 2026-04-28
status: deferred — runbook captured, awaiting upstream trigger conditions
---

# Quick Task 260428-rsu: Refresh stack onto upstream's new tip — Context

**Gathered:** 2026-04-28
**Status:** Deferred (awaiting trigger conditions; see `<deferred>` block below)

<domain>
## Task Boundary

Refresh PRs 725 (v2.0-pr) and 726 (v2.1-pr) on `always-further/nono` onto upstream `main` (currently `30c0f76e chore: release v0.43.0`) so they are no longer `mergeStateStatus: DIRTY` / `mergeable: CONFLICTING`.

Both PRs currently:
- All review threads RESOLVED (8/8 on 725, 1/1 on 726)
- CI green on the limited gates (Conventional Commit Title + DCO)
- No human reviewer engagement since 2026-04-22 pause
- Conflicting with upstream/main due to v0.41 + v0.42 + v0.43 cumulative drift

</domain>

<decisions>
## Implementation Decisions (LOCKED 2026-04-28)

### Timing vs upstream PRs (LOCKED — Wait)

**Decision:** Wait for the following upstream PRs to land BEFORE rebasing:
- `#785` (lukehinds, claude-pack-migration) — directly overlaps PROF-04 + PKG-01..04 in our Phase 22 work; resolving conflicts twice if we rebase now.
- `#777` (dependabot, sigstore-verify 0.6.4 → 0.6.5) — unblocks the v2.3 backlog item "Audit-attestation hardening sweep" (sigstore-rs `KeyPair::from_pkcs8` re-enablement currently `#[ignore]`'d in 22-05a's 2 fixture-driven tests).
- `#778` (dependabot, sigstore-sign 0.6.4 → 0.6.5) — companion to #777.

**Rationale:** Each of these is a direct or close-overlap with our open PR diffs. Rebasing now and again later is double-work; rebasing once after they land is single-work. None of them blocks human review — review can begin on the existing PRs at any time.

**Trigger:** When #785 lands on `always-further/nono@main`, OR when #777+#778 land, OR when a human reviewer on the upstream repo engages with PR 725/726 and asks for refresh.

### Phase 22 stack disposition (LOCKED — Hold local)

**Decision:** Phase 22 + Phase 24 work (currently on local `main` at `85943d4`, NOT in PR 725 or PR 726) stays on local `main` only until PRs 725 and 726 merge. After they land on upstream/main, Phase 22 work submits as a separate v2.2-pr.

**Rationale:** Smallest reviewer surface. v2.0 and v2.1 PRs are already in a 2-PR stack reviewers are tracking; adding a third on top extends the review chain. Holding Phase 22 local doesn't lose any work — the commits are on `main` and tagged in STATE.md / ROADMAP.md.

### Rebase shape (LOCKED — Re-squash)

**Decision:** When the rebase fires, re-squash each PR into the existing 1-commit-per-milestone shape. New v2.0-pr tip = upstream/main + 1 commit; new v2.1-pr tip = new v2.0-pr + 1 commit. Preserve the DCO sign-off authorship line (`oscarmackjr-twg`) and full upstream-trailer block on the squashed commit message.

**Rationale:** Matches the existing PR shape that resolved the original PR 555 DCO block (the 2026-04-21 `/gsd-pr-branch` decision). Reviewers already have a mental model of "1 squashed commit per milestone" — preserving that shape minimizes review surprise. The per-commit history is preserved on local `windows-squash` branch (source of truth) for archaeology.

### Conflict resolution authority (LOCKED — Stop at each conflict)

**Decision:** When merge conflicts surface during the rebase (they will — v0.41/v0.42/v0.43 deltas overlap with OAuth2 wiring, AUD ledger shape, PROFILE struct, and exec_strategy_windows files), STOP at each conflict. Surface to the user:

1. The conflicting file path
2. The conflict region (both `<<<<<<< ours` and `>>>>>>> theirs` blocks)
3. A recommended resolution and a one-sentence justification
4. The user's decision options

Do NOT auto-resolve and force-push without user confirmation.

**Rationale:** Force-push to a public PR with active gemini-code-assist re-review and pending human review is destructive and visible to others. Mistakes during conflict resolution would land directly on a branch reviewers see. CLAUDE.md `<security_considerations>` mandate: "When in doubt, choose the more restrictive option." This is the more restrictive option.

### Claude's Discretion (NOT discussed)

- Branch naming for any temporary working branches during the rebase (free to invent `v2.0-pr-rebase-{date}` etc).
- Cherry-pick range to use for the squash regeneration (free to use `git rebase -i upstream/main` or `git diff main upstream/main | git apply --3way` per local convenience).
- Order of rebase: v2.0-pr first, then v2.1-pr onto v2.0-pr's new tip (sequencing is mechanical, no user input needed).
- Whether to delete the old origin tips (`origin/v2.0-pr@3ec7fdf`, `origin/v2.1-pr@b95a9e4`) after force-push or leave them tagged for archaeology — Claude's call, default delete since git reflog covers archaeology.

</decisions>

<specifics>
## Specific Ideas

**Trigger-watch checklist (run via `gh` to test trigger condition):**

```bash
# Triggers met? (run any time)
gh pr view 785 --repo always-further/nono --json state    # need state=MERGED
gh pr view 777 --repo always-further/nono --json state    # need state=MERGED
gh pr view 778 --repo always-further/nono --json state    # need state=MERGED

# OR human review engagement
gh pr view 725 --repo always-further/nono --json reviews  | jq '.reviews[] | select(.state == "APPROVED" or .state == "CHANGES_REQUESTED")'
gh pr view 726 --repo always-further/nono --json reviews  | jq '.reviews[] | select(.state == "APPROVED" or .state == "CHANGES_REQUESTED")'
```

**Rebase recipe (when triggers fire):** see `260428-rsu-PLAN.md`.

**Critical files to watch for conflicts (based on Phase 22 cherry-pick experience):**
- `crates/nono-cli/src/cli.rs` — Cmd enum, Subcommands ordering (#785 lukehinds will rewrite this section)
- `crates/nono-cli/src/profile/builtin.rs` — `claude-no-keychain` shape vs upstream's package-migrated form
- `crates/nono-cli/data/policy.json` — orphan override_deny semantics + deny rules
- `crates/nono-cli/src/package_cmd.rs` — entire file overlaps with #785
- `crates/nono-cli/Cargo.toml` — sigstore-sign / sigstore-verify version pins (will conflict with #777/#778)
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — Phase 23 (AUD-05) hasn't started yet but is teed up; rebase may not touch it
- `crates/nono-proxy/src/oauth2.rs` — fork-only, but reverse_proxy.rs upstream may conflict
- `crates/nono-proxy/src/reverse.rs` — OAuth2 strict-proxy composition could collide with new upstream additions

</specifics>

<deferred>
## Deferred — Trigger Conditions

This task is INTENTIONALLY deferred. Do not execute the rebase until ANY of the following triggers fires:

1. **PR #785 merges on upstream** — preferred trigger; biggest overlap.
2. **PRs #777 and #778 both merge on upstream** — sigstore bump trigger; unblocks v2.3 audit-attestation hardening alongside the rebase.
3. **A human reviewer on `always-further/nono` engages** — submits a `CHANGES_REQUESTED` or `APPROVED` review on PR 725 or PR 726, OR posts an inline comment requesting refresh. Refresh is responsive at that point.
4. **2026-05-12 (2 weeks from today)** — soft deadline. If no triggers fire by this date, escalate (re-evaluate timing decision; upstream may have stalled).

When a trigger fires:

```
/gsd-quick resume refresh-stack-onto-upstream-tip
```

This loads CONTEXT.md (decisions stay LOCKED), reads PLAN.md as the runbook, and proceeds to execution under the same decisions.

</deferred>

<canonical_refs>
## Canonical References

- 2026-04-21 `/gsd-pr-branch` decision (HANDOFF.json `decisions` block): "Pre-squash each milestone into single sign-off commit instead of amending old PRs 530/555/583" — establishes the squash-into-one-commit-per-milestone shape this rebase preserves.
- ROADMAP.md § Backlog (v2.3 carry-forward): "Audit-attestation D-13 fixtures re-enablement" — sigstore-rs `KeyPair::from_pkcs8` re-enablement is the trigger condition for the v2.3 audit-hardening sweep that #777/#778 unlock.
- `.planning/templates/upstream-sync-quick.md` — DRIFT-02 template for upstream-sync quick tasks; this rebase is NOT an upstream-sync task (no cherry-picks, just rebase) so the template doesn't apply directly, but its `Upstream-commit:` 6-line trailer convention should be preserved on the new squashed commit messages.

</canonical_refs>
