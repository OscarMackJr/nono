---
phase: 85-upst9-divergence-audit
plan: "01"
subsystem: planning/audit
tags: [upst9, divergence-audit, ledger, v0.62.0, v0.64.0]
dependency_graph:
  requires: []
  provides:
    - "85-DIVERGENCE-LEDGER.md: cluster A-M dispositions + ADR risk verdicts + re-export scan results — gating input for Phases 86/87/88/89"
  affects:
    - Phase 86 (cherry-pick clusters A and B — boundary convergence)
    - Phase 87 (cherry-pick cluster C — AF_UNIX security fix)
    - Phase 88 (cherry-pick clusters D/E/G/H/I/J/K/L/M + dep-bumps)
    - Phase 89 (cherry-pick cluster F — proxy hardening split)
tech_stack:
  added: []
  patterns:
    - git-fetch-upstream-tip-check
    - drift-tool-SHA-safe-invocation
    - actual-diff-re-export-inspection
    - cluster-level-disposition-ledger
key_files:
  created:
    - .planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md
  modified: []
decisions:
  - "upstream_head_at_audit: 0153757001d21805a8218213e32add462d3322a1 (v0.64.0; no v0.65.0 at audit-open)"
  - "7f319b9e folded into Cluster B (diagnostic CLI surface: crates/nono-cli/src/diagnostic/formatter.rs; not in SEED-006)"
  - "Cluster A: will-sync (D-03 locked); no cross-cluster re-export dep; lib.rs gains pub mod audit; a5b2a516 must land before aed35bec/0b27cfc2 in Phase 86"
  - "Cluster B: will-sync (D-03 locked); intra-B ordering constraint: 4ad8ba92 -> f867aba2 -> a6aa5995 -> 7f319b9e; a6aa5995 adds 3 pub extern C fns in bindings/c/src/diagnostic.rs (Pitfall 3 closed)"
  - "Cluster C: will-sync; security=H (AF_UNIX bypass); Phase 87 must run cross-target clippy on cfg(linux) edits"
  - "Cluster F: split; 76b7b695 entirely won't-apply (tls_intercept/ absent); b5f8db5c/b0b2c743 partial-sync (tls_intercept hunks won't-apply); Phase 89"
  - "Cluster M: split; e54cf9cb applies to hook_runtime.rs (Unix only); hook_runtime_windows.rs retains env_clear + SystemRoot/windir/SystemDrive baseline"
  - "Re-export scan verdict: clean for cross-cluster deps; intra-B ordering constraint in Phase 86; tls_intercept re-exports in F won't-apply"
  - "ADR overall: A=H, B=H, C=H (security dominant), D=L, E=M, F=M, G=L, H=L, I=L, J=L, K=L, L=L, M=M; phase-level H driven by A/B boundary convergence"
  - "5bb098cd (Cluster G AWS auth) also touches tls_intercept/handle.rs; that hunk won't-apply in fork"
metrics:
  duration: "~45 minutes"
  completed: "2026-06-19"
  tasks_completed: 2
  files_created: 1
---

# Phase 85 Plan 01: UPST9 Divergence Audit Summary

## One-liner

UPST9 divergence ledger for upstream v0.62.0..v0.64.0: 35 commits classified into 14 rows (11 will-sync / 2 split / 1 won't-sync) with actual-diff re-export inspection on A/B/F and FFI Pitfall 3 closure.

## What Was Built

The complete `85-DIVERGENCE-LEDGER.md` for upstream `always-further/nono` commit window
`52809dda..01537570` (v0.62.0..v0.64.0). This is the gating input for Phases 86-89 cherry-pick
decisions.

**Ledger contents:**
- YAML frontmatter (13 keys) with D-07 tip-check result recorded
- Cluster Summary table (14 rows: A through M plus release/dep-bumps)
- 14 per-cluster sections each with: disposition, windows-touch, inspection depth statement, rationale, cross-cluster re-export check result, per-commit table
- ADR review section with 5-dimension (security/windows/maintenance/divergence/contributor) matrix for all 14 clusters
- Empirical cross-check section with 6 file-level git log walk spot-checks
- Cross-cluster re-export deps section with intra-B ordering constraint and F split finding
- TLS-intercept assessment section with per-commit split verdict for Phase 89
- Excluded as Noise section: 55 commits (23 merges + 32 out-of-filter), all enumerated

**Drift tool execution:** `bash scripts/check-upstream-drift.sh --from 52809dda --to 01537570 --format json` confirmed 35 substantive commits. SHA collision guard verified (local fork v0.62.0 = 3c5e9025 is NOT upstream v0.62.0 = 52809dda; explicit SHAs used throughout).

## Key Decisions Made

1. **7f319b9e folded into Cluster B** (not SEED-006 — touches `crates/nono-cli/src/diagnostic/formatter.rs`; standalone deprecated-command string fix in diagnostic CLI surface).

2. **Cluster F split finding confirmed:** `76b7b695` is entirely won't-apply (tls_intercept/ only). `b5f8db5c` and `b0b2c743` are partial-sync (tls_intercept hunks won't-apply; server.rs/route.rs hunks need reconciliation). `a5d623fd`, `7c9abd3b`, `724bb207` are sync-safe. `bd4b6b7f`'s `TlsInterceptIntent` struct needs Phase 89 assessment.

3. **Cluster B intra-ordering constraint:** Phase 86 cherry-pick sequence must be `4ad8ba92` → `f867aba2` → `a6aa5995` → `7f319b9e`. `4ad8ba92` migrates existing diagnostic surface out of core (prerequisite); `a6aa5995` adds the new module layout and 3 FFI functions.

4. **FFI Pitfall 3 closure:** `git show a6aa5995 -- bindings/c/src/` confirmed `bindings/c/src/diagnostic.rs` is net-new with `nono_last_diagnostic_code()`, `nono_last_remediation_json()`, and `nono_session_diagnostic_report_to_json()`. These reference types from the new `nono::diagnostic` module.

5. **5bb098cd (Cluster G) additional surface:** Also touches `tls_intercept/handle.rs` (AWS auth header injection path). That hunk is won't-apply in the fork; core AWS auth on `route.rs`/`credential.rs`/`server.rs` syncs cleanly.

6. **Atomic task execution:** Both Task 1 (initial classification) and Task 2 (actual-diff inspection) were executed in a single pass to eliminate placeholder debt. All actual-diff commands were run before writing the ledger. Single commit covers both tasks.

## Deviations from Plan

### Execution order adjustment (Rule 3 — auto-fix blocking pattern)

**Task 1 and Task 2 executed atomically.** The plan specified Task 1 writes the ledger with "Deferred to Task 2" placeholders, then Task 2 fills them in via Edit. Instead, all actual-diff inspection (git show for clusters A, B, F) was performed before writing the file, allowing a single clean Write with no placeholders. This eliminated a write-then-edit cycle that would require reading back the file between tasks.

- **Impact:** Final output is identical to the two-task output; fewer I/O operations.
- **AUDIT-02 compliance:** All acceptance criteria met in the single commit.
- **Commit:** `17fbaa3b` covers both Task 1 and Task 2 deliverables.

## Threat Surface Scan

No new source code written. This is a documentation-only phase. No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries. No threat flags.

## Known Stubs

None. All 14 cluster sections have complete dispositions, inspection results, and per-commit tables. No placeholder text remains.

## Self-Check

**Files created:**
- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` — 803 lines (above 400 minimum)

**Commits:**
- `17fbaa3b` — `docs(85-01): create UPST9 divergence ledger for v0.62.0..v0.64.0`

**SHA completeness:** All 35 substantive SHAs verified present via grep loop (all returned count ≥1).

**Key requirement checks:**
- total_unique_commits: 35 — present in frontmatter
- Cluster Summary table — present
- Clusters A and B both will-sync (D-03 honored)
- Clusters F and M both split
- e54cf9cb annotated Unix-path-only with hook_runtime_windows.rs note
- 7f319b9e classified with "Not in SEED-006" note
- 55 noise commits documented (23 merges enumerated)
- ADR review present with 5-dimension table
- Empirical cross-check present with 6 spot-checks
- TLS-intercept assessment present
- Cross-cluster re-export deps section present
- No "Deferred to Task 2" text (grep confirmed: 0 matches)
- upstream_head_at_audit in frontmatter (D-07)
- DCO sign-off on commit

## Self-Check: PASSED
