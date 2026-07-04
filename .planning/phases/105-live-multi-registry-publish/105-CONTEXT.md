# Phase 105: Live Multi-Registry Publish — Context

**Gathered:** 2026-07-03
**Status:** Ready for planning
**Source:** AskUserQuestion (lightweight discuss) + 105-RESEARCH.md strategic answers

<domain>
## Phase Boundary

Publish `0.66.1` LIVE to crates.io / PyPI / npm under the fork-owned identities (`nono-sandbox` family, `@oscarmackjr/nono-ts`). Because live publishing is IRREVERSIBLE + operator-in-loop, this phase AUTHORS + HARDENS + DRY-RUNS the publish machinery autonomously; the actual live publishes are operator checkpoints. No live publish is performed by Claude.
</domain>

<decisions>
## Implementation Decisions (LOCKED)

### D-01 — No technical dependency on Phase 104's signed .exe (version-consistency only)
Research proved (`cargo publish --dry-run -p nono-sandbox` succeeded live) that crates.io/PyPI/npm publish source/wheels/native-addons, NEVER the signed Windows `.exe`. The ROADMAP's Phase 104 dependency is a **version-consistency safeguard** (don't burn permanent `0.66.1` if Phase 104's Microsoft-root blocker forces a later `0.66.2`), not a build requirement. → Author + harden + dry-run all machinery NOW; sequence the live-publish operator checkpoints only AFTER Phase 104's `v0.66.1` tag is confirmed.

### D-02 — Platform scope = locally-buildable platforms only (reduced coverage, documented)
The fork has NO `nono-py`/`nono-ts` GitHub repos (their remotes point at upstream; upstream `publish.yml` hard-guards on `github.repository == 'always-further/nono-py'`), so the fork can't use their CI for full multi-platform builds. → Phase 105 publishes what THIS Windows dev host can build (crates.io source = all-platform by nature; PyPI = host wheel; npm = the `win32-x64-msvc` native leg this host builds). Reduced platform coverage (Linux/macOS wheels + npm platform packages) is an explicit, DOCUMENTED known limitation, deferred to a future phase that forks the sibling repos + builds full CI. The npm `optionalDependencies` / published-platform set MUST be made CONSISTENT (only-published-platforms present, or the runtime-require gap documented) to avoid the "missing-platform-package failure" for the platform(s) actually shipped.

### D-03 — publish-crates job = workflow_dispatch-only gate (never auto on tag-push)
Given irreversibility, convert the neutralized `publish-crates` job to a manual `workflow_dispatch`-only gate (not re-enabled on tag-push). The operator explicitly triggers the live publish; a future tag push can never fire an unintended live publish.

### D-04 — npm `@oscarmackjr` scope = hard operator precondition
`www.npmjs.com/~oscarmackjr` + the org page both 404; `npm whoami` = `ENEEDAUTH`. Before any npm publish, the operator must create an npm user OR org named exactly `oscarmackjr` and authenticate, then `npm publish --access public` for the first scoped publish. Operator action item, NOT autonomous.

### Claude's Discretion
The exact index-visibility poll implementation (lean on cargo ≥1.66 built-in blocking + a defensive sparse-index poll), the dry-run harness structure (extend `scripts/release-dry-run.ps1`), and the post-publish resolve-check mechanics.
</decisions>

<canonical_refs>
## Canonical References
- `.planning/phases/105-live-multi-registry-publish/105-RESEARCH.md` — the registry mechanics + both strategic answers + the two structural findings
- `scripts/release-dry-run.ps1` — the existing crates.io dependency-order dry-run "always-runnable core" to extend
- `.github/workflows/release.yml` (publish-crates job, currently `if: false`) — the job to rewrite as a workflow_dispatch gate
- `.planning/phases/102-fork-owned-package-rename/102-RESEARCH.md` — availability-check commands + Pitfall 4 (npm scope)
</canonical_refs>

<deferred>
## Deferred Ideas
- Full multi-platform coverage (Linux/macOS PyPI wheels + npm platform packages) via forked `OscarMackJr/nono-py` + `nono-ts` repos with ported CI — a future phase (Open Question 1, deferred per D-02).
</deferred>

---
*Phase: 105-live-multi-registry-publish*
*Context gathered: 2026-07-03 via AskUserQuestion*
