---
plan: 01
phase: 54-upst7-audit
status: complete
requirements: [REQ-UPST7-01]
date: 2026-06-04
must_haves_verified: 24
---

# Plan 54-01 Summary — UPST7 Audit (DIVERGENCE-LEDGER v0.57.0..v0.59.0)

## Summary

Produced `54-DIVERGENCE-LEDGER.md` — a falsifiable, disposition-complete divergence inventory of
the **40 unique upstream commits** across `v0.57.0..v0.59.0` (drift-tool source of truth; the
260527-sgo gap analysis under-counted at ~19), grouped into **14 clusters**. SC3 mandatory
re-fetch captured `upstream_head_at_audit=48d39f36` / `refetch_date=2026-06-04`. Every cluster
carries a 4-vocab disposition + windows-touch + rationale + commit-row table; the will-sync
clusters carry a cross-cluster re-export diff-inspect; the ledger includes the mandatory `## ADR
review`, `## Empirical cross-check`, `## Cross-cluster re-export deps detected`, and `## TLS-intercept
clean-apply assessment (Phase 34 C11)` sections. REQ-UPST7-01 satisfied. Zero source edits.

## Artifacts Created

- `.planning/phases/54-upst7-audit/54-01-LOCK-NOTES.md` — SC3 re-fetch lock (`7a7ad4a7`)
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` — the deliverable (`096375a2` scaffold, `07b0a23d` populated)
- `.planning/ROADMAP.md` — Phase 54 flipped `[x]` 1/1 + UPST8 stub (`0b49c697`)
- `.planning/STATE.md` + this SUMMARY — close (this commit)
- (drift JSON → `ci-logs-local/drift/*-v057-v059.json`, gitignored, not committed)

## Close-Gate Verification

| # | Check | Result |
|---|-------|--------|
| 1 | drift re-run idempotent (`bash scripts/check-upstream-drift.sh … --format json`) | PASS — exit 0, 40 commits |
| 2 | row-count >= drift total_unique_commits | PASS — 40 == 40 |
| 3 | every cluster has disposition (4-vocab) + rationale | PASS — 14/14 |
| 4 | `## ADR review` + 5-dim L/M/H | PASS — 1 + 5 |
| 5 | `## Empirical cross-check` >=4 files + `## Cross-cluster re-export deps detected` | PASS — 5 files + present |
| 6 | `## TLS-intercept clean-apply assessment (Phase 34 C11)` + `**Verdict:**` | PASS — fork-preserve |
| 7 | frontmatter `upstream_head_at_audit` (40-char) + `refetch_date` | PASS |
| 8 | ROADMAP UPST8 stub + Phase 54 `[x]` | PASS |
| 9 | STATE.md updated (Current/Accumulated/table/frontmatter) | PASS |
| 10 | zero-source-edits `git diff eb8c9b82..HEAD -- crates/ bindings/ scripts/ Makefile` | PASS — 0 |

## Disposition Breakdown

| disposition | clusters | count |
|-------------|----------|-------|
| will-sync | C3 allow_domain, C4 proxy-502, C6 bw://, C7 profile (JSONC/target_binary/opencode), C9 pack-hints, C10 diagnostic polish, C11 timeout constants, C12 policy-test | 8 |
| split | C2 supervisor IPC (→Ph59), C5 TLS-intercept ordering (→Ph56), C8 session hooks (→Ph58) | 3 |
| fork-preserve | — | 0 |
| won't-sync | C1 release commits, C13 sigstore (Cargo bump ok / scrub.rs verify), C14 macOS-only | 3 |

windows-touch:yes = 2 (C2, C8). Commit coverage: 40/40 (exact, zero-gap).

## ADR Review Outcome

**(a) Confirm** Phase 33 ADR Option A `continue`. Per-cell verdicts: security M, windows L,
maintenance M, divergence M, contributor L. No carve-outs; no future-supersede trigger. Phase 54
does NOT supersede Phase 33 ADR (stays Accepted).

## Cross-cluster Re-export Findings

`pub use`/`pub mod`/`extern crate`/`pub(crate)` diff-inspect on all 8 will-sync lead commits:
**clean** — only 3 intra-cluster `pub(crate) fn` definitions, no cross-cluster re-export
(Phase-43 `8b888a1c` trap does not recur). One **function-call** cross-cluster prereq:
**C5 → C3** (`partition_allow_domain` / `endpoint_routes`) — absorb allow_domain before the C5
`proxy_runtime.rs` port. `feedback_cluster_isolation_invalid` structurally closed for UPST7.

## SC4 TLS-intercept Verdict

**fork-preserve.** The fork's `RouteStore`/`CredentialStore` separation already enforces
endpoint-before-credential ordering; upstream's `tls_intercept/handle.rs` targets a module the fork
does not carry (do not import). `credential.rs` is untouched by `22e6c40` (Phase-09/11 rewrite
byte-identical, SHA `c9f25164` invariant preserved). Only portable artifact: the 12-line
`proxy_runtime.rs` filter-allowlist snippet — a small-additive-port rider coupled to Phase 56's
allow_domain absorption. `rcgen` bump (`8e78daf`) won't-sync (in absent `tls_intercept/`). This is
the diff-inspect note Phase 56 (REQ-NET-01) requires.

## Empirical Cross-Check Files

5 files walked (≥4): `crates/nono-proxy/src/route.rs` (0 — fork-original), `credential.rs`
(0 — fork-divergent), `crates/nono/src/keystore.rs` (2 → C6), `crates/nono-cli/src/profile/mod.rs`
(14: 8 non-merge in inventory + 6 merges correctly excluded), `crates/nono-cli/src/platform.rs`
(0 → confirms no java-dev cluster in range). Zero drift-tool gaps; no follow-up fix needed.

## v0.60.0 Scope Decision

Range kept `v0.57.0..v0.59.0` per the locked SC. **v0.60.0..v0.61.1 deferred to UPST8**
(human-confirmed 2026-06-04). The re-fetch surfaced v0.60.0 (`9a05a4ff`) + v0.61.0 + v0.61.1 —
larger than the v0.60.0-alone set the plan anticipated; the UPST8 stub records the wider deferred
set. Not the unrelated Feb-2026 v0.6.x line.

## Next Steps

Phase 55 (UPST7 Cherry-pick Wave) consumes these dispositions as immutable input. **Release-scope
guard:** hold Phase 55 *code* off `main` until v0.58.0 is tagged (per quick-260604-nue) — Phase 55
changes shipped binaries and would otherwise ride into the signed v2.9 release. Recommended: tag
v0.58.0 first (once the Azure signing cert lands), then do Phase 55 as v0.59.0/next.
