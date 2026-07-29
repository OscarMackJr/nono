---
phase: 108-upst12-divergence-audit
plan: 01
subsystem: upstream-divergence-audit
tags: [upst12, divergence-ledger, audit, reproduction]
dependency-graph:
  requires: []
  provides:
    - "108-DIVERGENCE-LEDGER.md Reproduction block (pinned v0.66.0..v0.69.0 SHAs)"
    - "Full 100-commit CODE/DEPS/CI/DOCS accounting"
    - "5-cluster Cluster Summary skeleton (NET/PROF/CORE/tool-sandbox-surface/security-residual-and-misc)"
  affects:
    - "Phase 108 Plans 108-03/108-04/108-05 (disposition/windows-touch/security-relevant fill-in)"
    - "Phase 109/110/111 planning (blocked on this ledger + ADR-108)"
tech-stack:
  added: []
  patterns:
    - "File-set-based noise classification (D-15) with prefix cross-check as secondary signal only"
    - "Documented per-commit edge-case overrides where the base 4-bucket rule doesn't cleanly partition"
key-files:
  created:
    - .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md
  modified: []
decisions:
  - "Measured bucket split (CODE 62 / DEPS 19 / CI 11 / DOCS 8) recorded as authoritative, superseding CONTEXT.md's 68/15/11/6 hypothesis, per D-04/D-21"
  - "D-06's tool-sandbox directory-vs-union equality does NOT hold this window (18 vs 20) -- union is authoritative going forward"
  - "Release-cut commits (CHANGELOG.md + Cargo.toml/lock, no src/) classified DEPS; data/policy.json + tests/-only commits classified CODE; community-health commits mixing .github/ISSUE_TEMPLATE with .md classified DOCS"
metrics:
  duration: "~35 minutes"
  completed: 2026-07-29
---

# Phase 108 Plan 01: UPST12 Divergence Ledger Reproduction + Full Commit Accounting Summary

Built the foundation ledger for the v0.66.0..v0.69.0 upstream divergence audit: live-reconfirmed
pinned SHAs, a documented 100-commit CODE/DEPS/CI/DOCS classification with explicit edge-case
rules, and a 5-cluster Cluster Summary skeleton (dispositions left TBD for Plans 108-03/04/05).

## What Was Built

`.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`:

1. **Reproduction block** — `git fetch upstream --tags` + `git ls-remote --tags` output for all
   5 window tags (v0.66.0, v0.67.0, v0.67.1, v0.68.0, v0.69.0), all matching CONTEXT.md's pinned
   SHAs verbatim (no upstream movement since 2026-07-29, no escalation needed). `git cat-file -t`
   / `-e` guard checks for all 5 SHAs. Live-reconfirmed counts: 100 non-merge commits, 4 merge
   commits (this window has merges, unlike UPST11's zero — all 4 explicitly named and excluded).
2. **Full Commit Accounting** — all 100 non-merge commits classified into CODE (62) / DEPS (19)
   / CI (11) / DOCS (8) by touched-path-set (D-15), summing to exactly 100. 9 commits needed
   documented edge-case rules because the base "touches src/" / "ONLY Cargo.toml+lock" / "ONLY
   .github+Makefile+scripts" / "ONLY docs+CHANGELOG+md+mdx" definitions don't cleanly partition
   them (4 release-cut commits mixing CHANGELOG.md with Cargo.toml/lock; 2 policy-data commits
   touching `data/policy.json` + `tests/`; 1 test-only commit; 2 community-health commits mixing
   `.github/ISSUE_TEMPLATE/` with `.md` files). Each rule and its rationale is spelled out so a
   future auditor reproduces the identical split.
3. **Prefix-vs-path-set cross-check** — 38 commits carry a `chore:`/`ci:`/`docs:`/`build:`
   (optionally scoped) prefix; 38 commits landed in a non-CODE bucket; but the *sets* disagree on
   4 SHAs (2 each direction), all individually named and explained.
4. **Cluster Summary skeleton** — 5 clusters covering all 62 CODE-bucket commits: NET (12,
   proxy/network), PROF (8, profile/policy), CORE (4, macOS/resource-CLI), tool-sandbox-surface
   (20, the D-05/D-09 3-path-union set, not yet subdivided pure/split), security-residual-and-misc
   (18, includes all 10 D-18-named security-relevant anchors plus 8 additional unrouted commits).
   Arithmetic check: 12+8+4+20+18 = 62 = CODE total. `disposition` / `windows-touch` /
   `security-relevant` columns are literal `TBD` per plan instruction — Plans 108-03/04/05 own
   filling those in.

## Key Findings (recorded in the ledger, not silently resolved)

**Finding 1 — bucket-split discrepancy vs. CONTEXT.md hypothesis.** This plan's live measurement
produced CODE 62 / DEPS 19 / CI 11 / DOCS 8, which disagrees with CONTEXT.md's recorded hypothesis
of CODE 68 / DEPS 15 / CI 11 / DOCS 6 (same 100 total, same CI count). Root cause, worked out and
documented in the ledger's "Discrepancy vs. CONTEXT.md Hypothesis" subsection: CONTEXT.md's
hypothesis implicitly folded all 9 path-set edge-case commits into CODE (59 strict + 9 = 68,
leaving DEPS/CI/DOCS at their strict values 15/11/6). This ledger instead classifies each of the
9 edge cases individually by dominant semantic content (4 release commits -> DEPS, 3 policy-data/
test-only commits -> CODE, 2 community-health commits -> DOCS). Both numbers are recorded in the
ledger; **this ledger's 62/19/11/8 is authoritative** for all downstream Phase 109-112 work per
D-04/D-21 (re-measurement supersedes the CONTEXT.md hypothesis, which was explicitly flagged as
"re-verify at audit time, do not copy forward blindly").

**Finding 2 — D-06's tool-sandbox union-vs-directory equality does not hold this window.** D-06
states "the union of these three [paths] equals the directory-only set (true for this window --
20 either way)". Live re-measurement: `crates/nono-cli/src/tool-sandbox/` alone (directory-only)
= **18** commits, not 20. The 3-path union (directory + `command_policy.rs` + `lineage_cgroup.rs`)
= 20, matching CONTEXT.md's count. Two commits (`5a7447d3ed30835bd9bd647b7812ee18ea80a155`,
`ebd51cbb9546aec872136301249d048074a05e38`) touch `command_policy.rs` but not the `tool-sandbox/`
directory and would be silently dropped by a directory-only filter. This validates D-06's own
caveat that the equality "holds by coincidence, not by rule" — this window shows it does not even
hold numerically for the directory-only count. The ledger records this as an explicit finding and
states the 3-path union (not the directory alone) is authoritative for all future syncs.

## Deviations from Plan

### Auto-fixed Issues

None — no code changes, no bugs to fix. This is a planning-artifact-only plan.

### Judgment Calls (Rule 2-adjacent: filling a gap the plan's literal rules didn't cover)

**1. Bucket classification edge cases for 9 commits not cleanly covered by the plan's literal
4-bucket definitions.** The plan defines CODE/DEPS/CI/DOCS via strict "touches src/" / "ONLY
<pattern>" tests. 9 commits (4 release-cut, 2 policy-data+test, 1 test-only, 2 community-health)
fail all four strict tests because they mix patterns from two buckets or touch a path (`data/`,
`tests/`, `.github/ISSUE_TEMPLATE/`) the base rule doesn't name. Rather than force an arbitrary
choice silently, each is resolved with an explicit, named rule in the ledger's "Bucket
Classification Edge Cases" subsection, and the resulting discrepancy against CONTEXT.md's
hypothesis is called out as Finding 1 above. This is the exact "re-measurement catches drift"
behavior D-04/D-21 exist to produce — not a defect, a finding.

No other deviations. Plan executed as written otherwise.

## Self-Check

- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` exists: FOUND
- Commit `8cf5ed92` exists in `git log`: FOUND (verified below)
- All 5 pinned tag SHAs resolve via `git cat-file -e <sha>^{commit}`: verified live during
  execution (all 5 returned `OK`)
- Reproduction section commit counts (100 non-merge / 4 merge) verified live against the pinned
  range before being written into the ledger
- Bucket table row counts (62+19+11+8) sum to 100, verified programmatically before writing
- Cluster Summary commit_count column (12+8+4+20+18) sums to 62 (the CODE bucket total), verified
  before writing
- `.planning/STATE.md` and `.planning/ROADMAP.md`: NOT modified (verified via `git status
  --short` before final commit — only the ledger and this SUMMARY are staged)

## Self-Check: PASSED
