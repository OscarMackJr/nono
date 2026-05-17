# Phase 42: UPST5 audit - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 42-upst5-audit
**Areas discussed:** Audit range upper bound

---

## Audit range upper bound

### Q1 — What's the upper bound of the audit range — a clean tag boundary, or a snapshot of upstream/main HEAD at audit-open?

| Option | Description | Selected |
|--------|-------------|----------|
| v0.54.0 tag boundary | Range = v0.53.0..v0.54.0 (~20 non-merge commits). Clean reproducibility: any reader rerunning gets the same input set. Mirrors Phase 33 + 39 D-A1 pattern. Post-v0.54.0 commits (4 known: 66c69f86 snapshot symlink validation, 803c6947 nix bump, fc965ccc tokio bump, 089cf6a0 cosign bump) roll into UPST6. | ✓ |
| upstream/main HEAD at first-commit-of-plan | Range = v0.53.0..<sha captured at plan start>. Catches the 4 post-v0.54.0 commits (66c69f86 looks security-relevant: validates restore targets against symlinks). Breaks 'tag-boundary-only' reproducibility convention. Phase 39 explicitly rejected this (D-39-A1). | |
| v0.54.0 + cherry-pick 66c69f86 inline | Tag-boundary base for the bulk; carve out 66c69f86 (snapshot symlink validation — security-relevant) as an explicit single-commit addendum cluster in the ledger with rationale. Preserves reproducibility for the v0.54.0 audit while not deferring a likely-important security fix to UPST6. | |

**User's choice:** v0.54.0 tag boundary
**Notes:** Inherits Phase 33 + 39 audit-boundary discipline verbatim. The 4 post-v0.54.0 commits (incl. security-flavored 66c69f86) roll into UPST6 per D-42-A4.

---

### Q2 — Frontmatter capture for the upper bound — do we still record upstream/main HEAD at audit-open, or just the tag?

| Option | Description | Selected |
|--------|-------------|----------|
| Both: range=v0.53.0..v0.54.0 + upstream_head_at_audit | Inherits D-39-A2 pattern verbatim. Range is the audit boundary; upstream_head_at_audit records the post-`git fetch upstream --tags` HEAD as historical signal for what UPST6 will absorb. Falsifiable: grep 'upstream_head_at_audit' returns 1. | ✓ |
| Range only (drop upstream_head_at_audit) | Tag boundary alone is the reproducibility anchor; HEAD capture adds noise. Diverges from Phase 39 D-39-A2 (which captured both). Cleaner frontmatter, slightly less context for UPST6. | |
| Range + upstream_head_at_audit + post-tag commit count | Both fields, plus a 'post_v0540_commits: 4' counter line documenting the deferred-to-UPST6 surface size. Phase 39 didn't do this. Pro: surfaces UPST6 absorption load at v2.6 scoping time. Con: slight frontmatter inflation. | |

**User's choice:** Both: range + upstream_head_at_audit
**Notes:** Phase 39 D-39-A2 inheritance verbatim. HEAD-sha is the historical signal that lets UPST6 reconstruct what was punted.

---

### Q3 — When exactly does the auditor capture upstream_head_at_audit?

| Option | Description | Selected |
|--------|-------------|----------|
| First commit of Plan 42-01, post-fetch | Inherits D-39-D1 verbatim. Auditor runs `git fetch upstream --tags` then captures `upstream/main` sha into ledger frontmatter as the FIRST act of Plan 42-01. Matches Phase 33 + 39 lock cadence; reproducibility falsifiable against historical fetch state. | ✓ |
| At phase-open (CONTEXT.md commit) | Capture earlier — at Phase 42 open (this discuss-phase commit). Stable across replanning. But context-capture moment lacks `git fetch upstream --tags` discipline; risks staleness if user's local clone hadn't fetched recently. | |
| At plan-phase open (before any execution) | Capture during Plan 42-01 plan-phase generation, not at first execution commit. Slightly earlier than D-39-D1 but still post-fetch. Lower risk of drift between plan-phase and execution-phase. Diverges from D-39-D1 cadence. | |

**User's choice:** First commit of Plan 42-01, post-fetch
**Notes:** D-39-D1 inheritance preserves discipline of `git fetch upstream --tags` as an explicit auditable act in the plan's commit history.

---

### Q4 — How do we handle post-v0.54.0 commits (4 known: 66c69f86 snapshot symlink validation + 3 dep bumps)?

| Option | Description | Selected |
|--------|-------------|----------|
| Strictly silent | Inherits D-39-A3 verbatim. Ledger covers v0.53.0..v0.54.0 only; anything past c4b25b82..6b00932f is UPST6's problem. Mentioning post-range commits muddies the audit boundary. Cadence rule is structural — each audit closes a defined range. | ✓ |
| Silent on commits + UPST6 stub records sentinel post-range head | Ledger body silent on post-range commits (D-39-A3 honored). UPST6 ROADMAP stub captures upstream/main HEAD at Phase 42 close as the next audit's lower-bound sentinel + 4-commit count. Preserves audit-boundary discipline; gives UPST6 a concrete starting point. | |
| Note security-flavored commits in an audit-watch addendum | Ledger body silent on routine bumps, but a `## Audit watch (post-v0.54.0)` section flags 66c69f86 (snapshot symlink validation — security-relevant) by sha+subject only, no disposition. Diverges from D-39-A3. Pro: maintainer sees the security signal early. Con: muddies audit boundary. | |

**User's choice:** Strictly silent
**Notes:** D-39-A3 inheritance verbatim — security-flavored 66c69f86 absorbed into UPST6 next cycle, not surfaced inline.

---

## Areas not selected for discussion

The following three gray areas were surfaced but not selected; CONTEXT.md captures them with Phase 39 defaults + phase-42-specific notes for the planner/auditor to refine:

- **windows-touch handling for `5d821c12` + `0748cced`** — inherits D-39-C3 conservative-default fork-preserve unless empty fork-side. Explicit per-commit disposition required at audit time (success criterion #2).
- **ADR review verdict (Option A continue)** — first cycle where windows-touch: yes actually fires; Phase 42 upgrade to D-39-C4 requires explicit per-cell L/M/H verdicts (D-42-C4).
- **Empirical cross-check + UPST6 stub queue location** — success criterion #4 demands ≥3 fork-shared file spot-checks (D-42-E1); UPST6 stub location TBD between v2.6 backlog or v2.5 § Future Cycles holding section per D-42-B4.

## Claude's Discretion

- Cluster grouping heuristic (per-walk auditor judgment).
- Per-cluster wave-hint granularity (advisory only, Phase 43 planner refines).
- UPST6 stub title wording (`… audit` vs `… sync execution` based on Phase 42 ledger shape).
- Whether to capture a § Delta-since-Phase-39 fork-only surface section (audit-walk discretion based on Phase 41 surface changes).
- Empirical cross-check file selection (recommendation: preferentially sample Phase-41-touched files).
- `make ci` substitute re-run cadence (per-commit or once at plan close).
- ADR review verdict outcome ((a) confirm with updated L/M/H, (b) amend with carve-outs, (c) flag future-supersede trigger).

## Deferred Ideas

- Post-v0.54.0 commit absorption — 4+ known unreleased commits past `6b00932f`. UPST6 absorbs per lazily-evaluated cadence rule.
- `66c69f86` reachability verification — pre-audit listing showed ambiguously; auditor confirms at audit walk whether it's pre-v0.54.0 (in scope) or post-v0.54.0 (UPST6 deferral). Security flavor (symlink TOCTOU class) makes it elevated UPST6 priority but does NOT trigger inline audit-watch.
- Drift-tool fixes surfaced mid-audit — `.planning/quick/` follow-up task, NOT folded into Phase 42 (D-42-D3).
- Full wave-map for Phase 43 — Phase 43 planner decides.
- Fork-only surface area delta enumeration — auditor's discretion at audit walk.
- Superseding ADR — separate phase, not Phase 42 inline edit.
- UPST6 scope — auditor picks title + location at Phase 42 close.
