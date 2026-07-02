# Phase 98: UPST11 Divergence Audit - Context

**Gathered:** 2026-06-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Produce a complete, commit-level `DIVERGENCE-LEDGER` for the `nolabs-ai/nono`
`v0.65.1..v0.66.0` window — the ~19 PRs referenced by upstream release-cut
**#1293** (`chore: release v0.66.0`, MERGED 2026-06-29) — and settle the
**#1225 `NetworkIntent`-vs-`ProxyOnly`** adopt-vs-fork-divergence call in a
standalone ADR.

The ledger classifies **every** substantive commit into will-sync /
fork-preserve / won't-sync / split clusters, with a `windows-touch` flag per
commit and a per-cell ADR-review verdict (continue/escalate). It refines the
`260629-toe` quick-task's preliminary **per-PR** dispositions to **per-commit**
resolution.

**This phase audits, classifies, and decides #1225 — only.** No cherry-picks,
no version bump, no pipeline work. The ledger + the #1225 ADR are the sole
deliverables and gate Phase 99 (absorb) and Phase 100 (release reconcile).

The git `upstream` remote is **already** repointed at `nolabs-ai/nono` (done in
v3.3 Phase 94); `upstream-legacy` retains `always-further/nono`. No remote
relocation work this cycle.

</domain>

<decisions>
## Implementation Decisions

### Window & Provenance
- **D-01:** Audit window is upstream `v0.65.1..v0.66.0`. Fork's confirmed
  upstream high-water mark is **v0.65.1** (absorbed in v3.3 / UPST10, Phases
  94-97). The window is the work behind release-cut **#1293** (`605bb6c5`,
  Luke Hinds) — itself a release-cut PR only (CHANGELOG + version bump
  `0.65.1→0.66.0`); the substantive code is in the ~19 referenced PRs.
- **D-02:** The audit MUST `git fetch upstream` and pin the window tip SHAs in
  the ledger Reproduction block (`git ls-remote --tags` → `v0.65.1` and
  `v0.66.0`). Run the drift tool against explicit window **SHAs, not tag
  names** (SHA-not-tag guard, Phase 85/94 precedent).
- **D-03:** Cherry-pick works at the commit level even though the CHANGELOG
  groups PRs — so the ledger resolves to commits, not PRs. The 260629-toe
  per-PR dispositions are a **starting hypothesis to re-confirm**, not a
  ground truth to copy.

### #1225 NetworkIntent Disposition (the headline decision)
- **D-04:** The #1225 call is settled by an **open, diff-grounded ADR** — no
  pre-commitment to adopt or diverge. The audit MUST:
  1. `git show` the actual #1225 commit(s) to capture the real upstream shape
     of `NetworkIntent` (not the CHANGELOG summary).
  2. Map **every** fork touchpoint of the surface it replaces:
     `crates/nono/src/capability.rs` (`NetworkMode::ProxyOnly` constructors
     ~1045/1065, match arms ~843/1386/2711+, doc ~707),
     `crates/nono/src/manifest_convert.rs:47`,
     `crates/nono/src/sandbox/linux.rs:386/620`, plus the Windows WFP/
     AppContainer backends and the proxy-filter post-fork path that key off the
     network-capability enum.
  3. Then render an adopt-vs-diverge **recommendation** in the ADR, with the
     rationale tied to which fork invariants each option affects.
- **D-05:** Decision options the ADR weighs (per ROADMAP SC2): **full-sync-adopt**
  (precedent: v3.1 Phase 86 adopted upstream's high-conflict boundary
  refactors) vs a **written fork-divergence carve-out**. Whichever wins MUST
  NOT regress the policy-free library boundary (ADR-86) or the Windows network
  model, and MUST be cross-target-clippy-clean (verified in Phase 99, but the
  ADR flags the cfg-gated surfaces).
- **D-06:** The ADR is a **standalone file**: `proj/ADR-98-network-intent-disposition.md`
  (mirrors `ADR-86`/`ADR-87` phase-number naming) — independently citable by
  Phase 99 cherry-pick guidance and future auditors, surviving ledger archival.
  The ledger cross-references it; it does not duplicate the analysis inline.

### Carve-out Re-touch Check (expanded this cycle)
- **D-07:** The mandatory carve-out re-touch check (Phase 94 D-04/D-05) is
  **expanded** beyond the original three to cover the carve-outs that have
  accumulated since. For each, run `git log <window> -- <exact path>` and
  record whether any window commit touches it; a **zero-hit** result is
  recorded explicitly as "clean — no re-touch in window" (silence is not
  evidence). The set:
  1. **CR-02** — `crates/nono/src/audit.rs` `records_verified: event_count > 0`
     (`proj/ADR-87-cr02-audit-bypass.md`).
  2. **CR-01** — `bindings/c/src/` FFI entry points calling
     `clear_last_call_state()` at entry (`diagnostic.rs`, `lib.rs`,
     `capability_set.rs`, `fs_capability.rs`, `sandbox.rs`, `state.rs`,
     `query.rs`).
  3. **Cluster F (proxy fork model)** — `crates/nono-proxy/src/{route,connect,reverse,server}.rs`,
     `crates/nono-cli/src/proxy_runtime.rs`, the **absent**
     `crates/nono-proxy/src/tls_intercept/` dir, and the `EffectiveProxySettings`
     model (Phase 89 reconciliation).
  4. **v3.3 Phase 95 endpoint-policy wiring** — `CompiledEndpointPolicy` /
     `endpoint_policy.evaluate()` across `crates/nono-cli/src/network_policy.rs`
     and `crates/nono-proxy/src/{config,credential,reverse,route,server}.rs`.
     **This is the surface #1225 restructures** — highest re-touch exposure
     this window.
  5. **v3.3 Phase 95 restored fork invariants** — arch-portable `offset_of!`
     msghdr derivation, GPU Landlock enforcement, post-fork fork-safety
     (`crates/nono/src/sandbox/linux.rs` and the AF_UNIX/seccomp paths).
  6. **v3.2 override surface** — `PolicyOverrideApplied` audit variant /
     SecurityEventLayer EventIDs 10006-10010 (lives in `nono-py`, but flag any
     core-crate `nono` audit-variant re-touch).
- **D-08:** Each carve-out hit is flagged **"expected conflict — preserve fork
  expression"** and carried into the Phase 99 cherry-pick guidance.

### Inspection Depth
- **D-09:** **Uniform actual-diff** — `git show` every substantive commit in the
  window (not risk-tiered), including the cross-cluster re-export scan
  (`pub use` / `pub mod` / `extern crate` / `pub(crate)` additions). The window
  (~19 PRs, est. tens of commits) is small enough to afford it, and uniform
  inspection eliminates the risk of an additive-looking commit hiding a
  boundary/FFI/security change. (Phase 85 used risk-tiered for 90 commits;
  Phase 94 used uniform for 8; this window does not warrant the Phase 85
  shortcut.) Re-confirm each 260629-toe disposition at commit granularity.

### Methodology Carried Forward (from Phase 85/94 — NOT re-litigated)
- **D-10:** Use the drift tool (`scripts/check-upstream-drift.{sh,ps1}`) re-run +
  empirical `git log` spot-check to validate cluster classification. Pin the
  tool SHA and exact invocation in the ledger Reproduction block.
- **D-11:** Letter clusters **fresh** for this window (A, B, C…) — independent
  of Phase 85's A–M and Phase 94's lettering.
- **D-12:** Reconcile noise: substantive + noise = total window commit count
  (`git log --oneline <window> | wc -l`); enumerate merge commits and
  out-of-filter commits so every commit lands in exactly one place.
- **D-13:** Per-cluster ADR risk matrix across the five standard dimensions
  (security / windows / maintenance / divergence / contributor); no cluster left
  with a bare `TBD`. Each disposition justified by ≥1 of: security impact,
  Windows-backend touch, or library-boundary relevance.
- **D-14:** For any will-sync/split commit touching `#[cfg(target_os = "linux")]`
  / `#[cfg(target_os = "macos")]` blocks, flag the CLAUDE.md cross-target clippy
  MUST for the Phase 99 executor (note it; the gate runs in 99, not 98).
- **D-15:** Release/version commits (Cargo.toml bumps, CHANGELOG, the #1293
  release-cut) go in the **won't-sync** cluster. Add a one-line cross-ref noting
  the upstream version metadata so Phase 100 knows the leapfrog floor. **Floor
  is settled at the milestone level: `0.66.1`** (collision-free above upstream's
  now-`0.66.0`; the fork already spent `0.66.0` in v3.3) — NOT 0.67, despite the
  260629-toe SUMMARY's earlier "≥0.67.0" note. No standalone release section in
  the ledger; keep the audit sync-focused.

### Claude's Discretion
- Cluster naming/theme labels, the exact set of empirical spot-check files
  (beyond the carve-out paths in D-07, which are mandatory), and the ledger's
  internal section ordering — follow the Phase 94/85 ledger shape as the
  template.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Prior-cycle precedent (the ledger template + the carve-out source-of-truth)
- `.planning/milestones/v3.3-phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md`
  — the UPST10 ledger; the **direct structural template** for this phase
  (frontmatter, Reproduction, Cluster Summary, per-cluster tables, ADR Review
  matrix, Carve-out Re-touch Check, Noise reconciliation).
- `.planning/milestones/v3.3-phases/94-upst10-divergence-audit/94-CONTEXT.md`
  — the Phase 94 decision set this phase mirrors and extends (D-04 carve-out
  list is the base D-07 expands on).
- `.planning/milestones/v3.1-phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md`
  — the UPST9 ledger; its **three addenda** (Phase 87 CR-02, Phase 88 CR-01,
  Phase 89 Cluster F) are the authoritative definition of carve-outs 1-3 in
  D-07 (exact files/lines + "future sync note" wording).

### The audit input (preliminary dispositions to re-confirm)
- `.planning/quick/260629-toe-v066-parity/PLAN.md` — the 260629-toe per-PR
  divergence ledger + fork-invariant gate list + recommended wave structure.
  **Preliminary** (fast collision scan) — D-03/D-09 re-confirm per-commit.
- `.planning/quick/260629-toe-v066-parity/SUMMARY.md` — the #1293 / #1225
  findings summary (note: its "≥0.67.0" floor is superseded by D-15's `0.66.1`).

### #1225 / NetworkIntent surface (for the ADR)
- `crates/nono/src/capability.rs` — `NetworkMode::ProxyOnly` (the surface
  #1225 replaces); map every touchpoint per D-04.
- `crates/nono/src/manifest_convert.rs`, `crates/nono/src/sandbox/linux.rs` —
  secondary ProxyOnly consumers.
- `crates/nono-cli/src/network_policy.rs`,
  `crates/nono-proxy/src/{config,credential,reverse,route,server}.rs` —
  `CompiledEndpointPolicy`/`endpoint_policy` (the v3.3 Phase 95 surface #1225
  collides with).
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library
  boundary the #1225 disposition must not regress; also the **adopt precedent**
  (Phase 86 adopted upstream's high-conflict refactor).
- `proj/ADR-98-network-intent-disposition.md` — **to be created this phase**
  (D-06).

### Fork-divergence ADRs / carve-out detail
- `proj/ADR-87-cr02-audit-bypass.md` — CR-02 `records_verified` hardening (D-07).

### Upstream parity process & sync mechanics
- `.planning/PROJECT.md` §`## Upstream Parity Process` — the process (remote
  already relocated in Phase 94; this cycle confirms only).
- `.planning/templates/upstream-sync-quick.md` — the cherry-pick scaffold
  (D-19 trailer block); consumed in Phase 99, not 98.
- `docs/cli/development/upstream-drift.mdx` — long-form drift-tool runbook
  (gitignored-but-tracked — `git add -f` if edited).
- `scripts/check-upstream-drift.sh` / `scripts/check-upstream-drift.ps1` — the
  drift tool; pin its SHA in the ledger Reproduction block (D-10).

### Requirements / roadmap
- `.planning/REQUIREMENTS.md` — UPST11-01 (Phase 98); UPST11-02/03/04 are
  Phase 99.
- `.planning/ROADMAP.md` §`### Phase 98` — the four success criteria this
  ledger + ADR must satisfy.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Drift tool** (`scripts/check-upstream-drift.{sh,ps1}`): re-run against the
  window SHAs for JSON output; path-inclusion filter is `crates/nono/src/`,
  `crates/nono-cli/src/`, `crates/nono-proxy/src/`, `crates/nono/Cargo.toml`;
  exclusion patterns `*_windows.rs` and `crates/nono-cli/src/exec_strategy_windows/`.
- **Phase 94 ledger** as the fill-in-the-blanks template (see canonical refs).
- **Upstream remote already set** — `upstream → nolabs-ai/nono`,
  `upstream-legacy → always-further/nono` (verified `git remote -v`); only a
  `git fetch upstream` is needed to populate the window.

### Established Patterns
- Ledger frontmatter records `range`, `upstream_head_at_audit`,
  `drift_tool_*_sha`, `drift_tool_invocation`, `fork_baseline`,
  `total_unique_commits`, `date` — reproduce this header shape.
- Carve-out "future sync note" pattern: each deliberate divergence has a
  guard-test name and an explicit "do not revert" instruction.
- Fork has **no** `NetworkIntent` (confirmed: `grep -rl NetworkIntent crates/`
  empty); `NetworkMode::ProxyOnly` and `CompiledEndpointPolicy` are both live
  and deep — the #1225 collision is real, not theoretical.

### Integration Points
- New ledger file:
  `.planning/phases/98-upst11-divergence-audit/98-DIVERGENCE-LEDGER.md`.
- New ADR file: `proj/ADR-98-network-intent-disposition.md` (D-06).
- `git fetch upstream` + `git ls-remote --tags` for window SHAs (D-02).

</code_context>

<specifics>
## Specific Ideas

- The ledger must mirror the Phase 94/85 ledger structure so the auditor trail
  stays consistent across cycles.
- The **#1225 disposition** and the **expanded carve-out re-touch check** are
  the two headline fork-specific deliverables — surface both prominently (the
  ADR is its own file; the carve-out check is its own ledger section), not
  buried in cluster prose.
- The version-collision fact (fork already at `0.66.0`, upstream now also
  `0.66.0`) is **release-blocking, not merge-blocking** — record the `0.66.1`
  floor in the won't-sync cross-ref (D-15) so Phase 100 doesn't re-derive it.

</specifics>

<deferred>
## Deferred Ideas

- **Actual cherry-pick / absorption + fork-invariant verify** — Phase 99
  (UPST11-02/03/04).
- **#1225 *implementation* of whichever disposition the ADR picks** — Phase 99.
  Phase 98 only *decides* and writes the ADR.
- **Crate leapfrog to `0.66.1` + pipeline reconcile + PyPI `RouteConfig`
  blocker + runbook** — Phase 100 (RLS-10..13).
- **#1245 idempotent-publish / #1251 CI compile-step** reconciliation against
  the prepare-only pipeline — classified in the ledger here, applied in
  Phase 99/100.

None of these are in Phase 98 scope.

</deferred>

---

*Phase: 98-upst11-divergence-audit*
*Context gathered: 2026-06-29*
