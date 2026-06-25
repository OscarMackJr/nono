# Phase 94: UPST10 Divergence Audit - Context

**Gathered:** 2026-06-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Produce a complete, actionable `DIVERGENCE-LEDGER` for the `nolabs-ai/nono`
`v0.64.0..v0.65.1` window (covering releases v0.64.1, v0.65.0, v0.65.1) and
record the upstream relocation (`always-further/nono` → `nolabs-ai/nono`).

The ledger classifies **every** substantive commit into will-sync /
fork-preserve / won't-sync / split clusters, with a `windows-touch` flag per
commit and a per-cell ADR-review verdict (continue/escalate). The git
`upstream` remote and the PROJECT.md `## Upstream Parity Process` references
are repointed at the new canonical source, with a Future Cycles stub noting the
next sync trigger.

**This phase audits and classifies only.** Absorption (cherry-pick / manual
replay, DCO sign-off, fork-invariant verify, `make build`/`make test`) is
Phase 95. Cross-target toolchain stand-up is Phase 96. Release engineering
(crate leapfrog, pipeline, runbook) is Phase 97.

</domain>

<decisions>
## Implementation Decisions

### Window & Provenance (grounded this session)
- **D-01:** Audit window is `0153757001..1d1c88c9` (= `v0.64.0..v0.65.1`).
  Confirmed reachable tip SHAs via `git ls-remote --tags https://github.com/nolabs-ai/nono.git`:
  `v0.64.0` `0153757001…`, `v0.64.1` `0551eba27e…`, `v0.65.0` `137bb15c56…`,
  `v0.65.1` `1d1c88c9f9…`. v0.65.1 is the current tip.
- **D-02:** `nolabs-ai/nono` is a **clean continuation**, NOT a re-fork with
  rewritten history — its `v0.64.0` SHA (`0153757001…`) is byte-identical to the
  fork's last sync point (the UPST9/Phase 85 endpoint and the former
  `always-further/nono` head). The window is therefore directly comparable; no
  history-translation step is needed.
- **D-03:** Fork already carries **local** tags for v0.64.1 (`0551eba2`) and
  v0.65.0 (`137bb15c`) matching nolabs SHAs, but has **not absorbed** those
  commits. v0.65.1 (`1d1c88c9`) is not yet a local tag. The drift tool must run
  against explicit window SHAs, not tag names (SHA-not-tag guard, per Phase 85
  precedent — though here the local tags happen to match).

### Carve-out Re-touch Check (the fork-specific value-add)
- **D-04:** The ledger MUST include a dedicated **"Carve-out Re-touch Check"**
  section. For each of the three deliberate fork-divergence points recorded in
  the Phase 85 ledger addenda, run `git log <window> -- <exact path>` and record
  whether any window commit touches it:
  1. **CR-02** — `crates/nono/src/audit.rs` `records_verified: event_count > 0`
     (audit-integrity bypass hardening; `proj/ADR-87-cr02-audit-bypass.md`).
  2. **CR-01** — `bindings/c/src/` FFI entry points calling
     `clear_last_call_state()` at entry (stale-diagnostic-state fix; commit
     `db0f221d`). Cover `diagnostic.rs`, `lib.rs`, `capability_set.rs`,
     `fs_capability.rs`, `sandbox.rs`, `state.rs`, `query.rs`.
  3. **Cluster F** — the proxy fork model: `crates/nono-proxy/src/route.rs`,
     `connect.rs`, `reverse.rs`, `server.rs`, and
     `crates/nono-cli/src/proxy_runtime.rs`, plus the **absent**
     `crates/nono-proxy/src/tls_intercept/` directory and the
     `EffectiveProxySettings` model (Phase 89 reconciliation addendum).
- **D-05:** Each carve-out hit is flagged **"expected conflict — preserve fork
  expression"** and carried into the Phase 95 cherry-pick guidance. A *zero-hit*
  result is also explicitly recorded as "clean — no re-touch in window" (silence
  is not evidence).

### Upstream Remote Relocation
- **D-06:** Repoint `upstream` → `https://github.com/nolabs-ai/nono.git`.
  **Retain the old remote** by renaming it `upstream-legacy` →
  `https://github.com/always-further/nono.git` (provenance/forensics only;
  continuity is byte-clean so legacy is cheap insurance, not load-bearing).
- **D-07:** Update PROJECT.md `## Upstream Parity Process` to reference
  `nolabs-ai/nono` as the canonical source (replace `always-further/nono`).
- **D-08:** Future Cycles stub trigger = **nolabs-ai/nono ships any `v*` tag past
  v0.65.1** (observable via `git ls-remote --tags`). Matches the established
  drain-then-sync per-tag-window cadence. Not drift-count, not time-based.

### Inspection Depth
- **D-09:** Default to **uniform actual-diff** — run `git show` on *every*
  substantive commit in the window (not risk-tiered), including the cross-cluster
  re-export scan (`pub use` / `pub mod` / `extern crate` / `pub(crate)`
  additions). The window is small enough that full-diff cost is low, and this
  eliminates the risk of an additive-looking commit hiding a boundary/FFI/
  security change. (Phase 85 used risk-tiered depth because it had 90 commits;
  this window does not warrant that shortcut.)

### Release-Readiness Foreshadow
- **D-10:** Release/version commits (Cargo.toml bumps, CHANGELOG, release
  metadata) go in the **won't-sync** cluster as usual (fork uses its own crate
  leapfrog convention). Add a **one-line cross-ref** in that row noting which
  window commits carry the upstream version metadata, so Phase 97 knows the
  leapfrog floor (≥ `0.65.0`) without re-deriving it. **No** standalone release
  section — keep the audit sync-focused.

### Methodology Carried Forward (from Phase 85 UPST9 — NOT re-litigated)
- **D-11:** Use the drift tool (`scripts/check-upstream-drift.{sh,ps1}`) re-run +
  ~6-file empirical `git log` spot-check to validate cluster classification.
  Record the tool SHA pin and the exact invocation in the ledger's Reproduction
  block.
- **D-12:** Letter clusters **fresh** for this window (A, B, C…) — independent of
  Phase 85's A–M lettering.
- **D-13:** Reconcile noise: substantive + noise = total window commit count
  (`git log --oneline <window> | wc -l`); enumerate merge commits and
  out-of-filter commits so every commit is accounted for in exactly one place.
- **D-14:** Per-cluster ADR risk matrix across the five standard dimensions
  (security / windows / maintenance / divergence / contributor); no cluster left
  with a bare `TBD` verdict. Each disposition justified by ≥1 of: security
  impact, Windows-backend touch, or library-boundary relevance.
- **D-15:** For any will-sync/split commit touching `#[cfg(target_os = "linux")]`
  / `#[cfg(target_os = "macos")]` blocks, the ledger flags the CLAUDE.md
  cross-target clippy MUST for the Phase 95 executor (note it; the gate runs in
  95/96).

### Claude's Discretion
- Cluster naming/theme labels, the exact set of empirical spot-check files
  (beyond the carve-out paths in D-04, which are mandatory), and the ledger's
  internal section ordering — follow the Phase 85 ledger shape as the template.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Prior-cycle precedent (the ledger template + the carve-out source-of-truth)
- `.planning/milestones/v3.1-phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md`
  — the UPST9 ledger. Use as the structural template (frontmatter, Headline,
  Reproduction, Cluster Summary, per-cluster tables, ADR Review matrix, Empirical
  Cross-Check, Noise reconciliation). Its **three addenda** (Phase 87 CR-02,
  Phase 88 CR-01, Phase 89 Cluster F) are the authoritative definition of the
  carve-outs for D-04 — read them for the exact files/lines and "future sync
  note" wording.

### Upstream parity process & sync mechanics
- `.planning/PROJECT.md` §`## Upstream Parity Process` — the process to update
  (D-06/D-07/D-08); also the AIPC Unix-futures locked decision.
- `.planning/templates/upstream-sync-quick.md` — the cherry-pick scaffold (D-19
  trailer block shape) consumed in Phase 95, not 94.
- `docs/cli/development/upstream-drift.mdx` — long-form drift-tool runbook
  (output formats, categorization rules, fixture regen, fork-divergence catalog
  rationale). NOTE: this path is gitignored-but-tracked — `git add -f` if edited.
- `scripts/check-upstream-drift.sh` / `scripts/check-upstream-drift.ps1` — the
  drift tool; pin its SHA in the ledger Reproduction block (D-11).

### Fork-divergence ADRs / carve-out detail
- `proj/ADR-87-cr02-audit-bypass.md` — CR-02 `records_verified` hardening (D-04).
- `proj/ADR-86-library-boundary-convergence.md` — the audit/diagnostics
  library-boundary carve-out (relevant if any window commit re-touches the
  core-crate audit/diagnostic surface).

### Requirements / roadmap
- `.planning/REQUIREMENTS.md` — UPST10-01 (Phase 94) + UPST10-04 (Phase 94);
  UPST10-02/03 are Phase 95.
- `.planning/ROADMAP.md` §`### Phase 94` — the three success criteria this
  ledger must satisfy.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Drift tool** (`scripts/check-upstream-drift.{sh,ps1}`): re-run against the
  window SHAs for JSON output; path-inclusion filter is `crates/nono/src/`,
  `crates/nono-cli/src/`, `crates/nono-proxy/src/`, `crates/nono/Cargo.toml`;
  exclusion patterns `*_windows.rs` and `crates/nono-cli/src/exec_strategy_windows/`.
- **Phase 85 ledger** as a fill-in-the-blanks template (see canonical refs).
- **Local nolabs tags already fetched** (v0.64.1, v0.65.0) — but absorb step is
  Phase 95; do not cherry-pick in Phase 94.

### Established Patterns
- Ledger frontmatter records `range`, `upstream_head_at_audit`,
  `drift_tool_*_sha`, `drift_tool_invocation`, `fork_baseline`,
  `total_unique_commits`, `date` — reproduce this header shape.
- Carve-out "future sync note" pattern: each deliberate divergence has a
  guard-test name and an explicit "do not revert" instruction.

### Integration Points
- Git remotes (`git remote rename` / `git remote set-url`) for D-06.
- `.planning/PROJECT.md` for D-07/D-08.
- New ledger file at
  `.planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md`.

</code_context>

<specifics>
## Specific Ideas

- The ledger must mirror the Phase 85 ledger's section structure so the auditor
  trail is consistent across cycles.
- Carve-out check is the headline fork-specific deliverable — surface it
  prominently (its own section + a Headline mention), not buried in cluster prose.
- Re-fork-vs-relocation question is **settled** (D-02): clean continuation,
  byte-identical v0.64.0 anchor. Record this proof in the ledger Reproduction
  block so a future auditor doesn't re-investigate.

</specifics>

<deferred>
## Deferred Ideas

- **Standalone "Release Readiness" section** — considered for Phase 97 input;
  rejected in favor of a one-line won't-sync cross-ref (D-10) to avoid
  overlapping Phase 97 scope.
- **Drift-count / time-based sync triggers** — considered for the Future Cycles
  stub; rejected in favor of next-`v*`-tag (D-08).
- **Actual cherry-pick / absorption** — Phase 95 (UPST10-02/03).
- **Cross-target toolchain stand-up** (retire PARTIAL→CI) — Phase 96.
- **Crate leapfrog + release pipeline + runbook** — Phase 97.

None of these are in Phase 94 scope.

</deferred>

---

*Phase: 94-upst10-divergence-audit*
*Context gathered: 2026-06-25*
