---
quick_id: 260527-sgo
slug: upstream-v044-v059-gap-analysis
date: 2026-05-27
status: complete
mode: research
---

# Summary: Upstream v0.44→v0.59 gap analysis (research)

## What was produced

`GAP-ANALYSIS.md` (216 lines) in this task dir — a themed, cited gap matrix of upstream nono
v0.44→v0.59 vs the Windows fork, with proposed phase buckets and a confidence/flags section.
Research + one-document write only; no production code touched.

## Headline finding (CORRECTS the pre-research framing)

**The fork's confirmed upstream-sync high-water mark is `v0.57.0`, not ~v0.43.** The
`v0-41-v0-43` in the Phase 47 ledger directory name is a *backfill paper-trail*, not the sync
frontier. Triangulated + cited from three repo-local sources:
- Phase 47 UPST6 ledger frontmatter: `range: v0.54.0..v0.57.0`, `fork_baseline: v0.54.0`.
- Phase 48 `48-VERIFICATION.md`: 42 v0.55–v0.57 upstream commits accounted for (REQ-UPST6-02 SATISFIED).
- `20260512-upstream-fork-release-grid/RESULT.md`: coverage chain Phase 20→22→34→42/43 (v0.37→v0.54).

So the **genuine forward gap is upstream v0.58.0 + v0.59.0** (~19 commits, deferred as "UPST7" per
D-47-A4). Most v0.44–v0.57 absorption-time deferrals have since been closed (profile drafts,
deprecated_schema, yaml_merge wiring, Windows env-filter, env sanitization — all verified present
in the current tree).

## Gap counts (the v0.58/v0.59 net-new set)

- **no (11):** bw:// credential source, session lifecycle hooks, JSONC config, `target_binary`,
  allow_domain path scoping, allow_domain method+path, named-socket IPC cluster, java_runtime
  group, proxy 502 handling, suppressed-denial annotations, canonical denial-path precompute
  (+ access-mode/overflow + macOS CWD-symlink fixes, PR-hygiene CI). Confirmed via zero-match grep.
- **partial / verify:** opencode relocation, TLS-intercept ordering, timeout-constant
  generalization, release attestation.
- **yes (boundary markers):** v0.55–v0.57 pack/profile verification tail.

## Proposed phase buckets (for roadmap mapping)

1. UPST7 audit + straight cherry-pick wave (small/med)
2. Network: fine-grained `allow_domain` path + method restrictions (med — marquee)
3. Bitwarden `bw://` credential source (small)
4. Session lifecycle hooks — Windows-equivalent design (med/large — highest design risk; upstream
   `hook_runtime` is unix-only)
5. Supervisor IPC robustness ported to Named-Pipe AIPC (med)
6. (Optional/CI) Release attestation alignment (small, low priority)

## Flagged needs-manual-review (honored verify-before-asserting)

- v0.59.0 was cut **today (2026-05-27)** — any later v0.59.x patches wouldn't appear yet.
- Upstream CHANGELOG anchored to tags, not PR numbers (refs are tag-level).
- opencode "extraction", proxy-502, and TLS-intercept-ordering fork status marked
  verify-not-asserted. No features fabricated.

## Notes

- Lives in `.planning/quick/` (normal `git add`, not the docs gitignore quirk).
- Next step is the team's: lift the phase buckets into roadmap planning (e.g. `/gsd:new-milestone`
  or phase-add). This doc is the seed.
