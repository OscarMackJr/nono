---
phase: 94-upst10-divergence-audit
plan: 01
subsystem: infra
tags: [upstream-sync, divergence-ledger, audit, security, proxy, carve-out]

# Dependency graph
requires:
  - phase: 85-upst9-divergence-audit
    provides: ledger template structure, three carve-out addenda (CR-02, CR-01, Cluster F)
  - phase: 87-security-hardening
    provides: CR-02 records_verified fork-hardening (ADR-87-cr02-audit-bypass.md)
  - phase: 88-feat-deps-wave
    provides: CR-01 FFI clear_last_call_state fork-hardening (commit db0f221d)
  - phase: 89-proxy-hardening
    provides: Cluster F proxy fork-preserve model (EffectiveProxySettings, guard tests)
provides:
  - "Complete DIVERGENCE-LEDGER for nolabs-ai/nono v0.64.0..v0.65.1 (8 substantive commits, 4 clusters)"
  - "Carve-out Re-touch Check: CR-02 additive-only hit, CR-01 clean, Cluster F two-commit hit with preserve-fork guidance"
  - "Phase 95 cherry-pick contract: Cluster A will-sync, B/C split with explicit guidance, D won't-sync"
  - "Phase 97 leapfrog floor >= 0.65.0 documented via Cluster D release cross-ref"
  - "New conflict discovered: 9b37dc52 inverts Phase 89 deliberate divergence (split guidance authored)"
affects: [95-upst10-absorb, 97-upst10-release]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Uniform actual-diff inspection (D-09) on every substantive commit (window small enough, no risk-tiering)"
    - "Fresh cluster lettering per window (A-D, independent of Phase 85 A-M)"
    - "Split disposition with explicit APPLY/SKIP guidance per hunk surface"

key-files:
  created:
    - ".planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md"
    - "ci-logs-local/drift/20260626T003610Z-v064-v0651-upst10.json (gitignored)"
  modified: []

key-decisions:
  - "9b37dc52 (upstream custom-credentials refactor) inverts Phase 89 deliberate divergence (0c08e5d2): classified as split — apply credentials_intent block fix only, preserve proxy_activates_with_custom_credentials_only guard test"
  - "11fd10e0 (tool sandbox) classified as split: tool-sandbox dir absent in fork (skip), tls_intercept/ hunks won't-apply, audit.rs additive additions are CR-02-safe and syncable, sandbox/linux.rs+mod.rs syncable after cross-target clippy"
  - "9ce74e92 (AF_UNIX mediation deadlock fix) classified as will-sync: closes 4 bugs in Phase 87 SEC-01 code path including dup2 bypass — security critical"
  - "Window has zero merge commits (17 total = 8 substantive + 9 noise, all non-merge noise)"

patterns-established:
  - "D-02 clean-continuation proof: nolabs-ai/nono v0.64.0 SHA is byte-identical to fork's UPST9 endpoint — record in Reproduction so future auditors skip re-investigation"
  - "Carve-out Re-touch Check: each of CR-02/CR-01/Cluster F must have an explicit hit OR 'clean — no re-touch in window' statement (silence is not evidence, D-05)"

requirements-completed: [UPST10-01]

# Metrics
duration: 90min
completed: 2026-06-26
---

# Phase 94 Plan 01: UPST10 Divergence Ledger Summary

**DIVERGENCE-LEDGER for nolabs-ai/nono v0.64.0..v0.65.1: 8 substantive commits in 4 clusters (A will-sync, B/C split, D won't-sync); new conflict found — upstream 9b37dc52 inverts Phase 89 custom-credentials fail-secure divergence**

## Performance

- **Duration:** ~90 min
- **Started:** 2026-06-26T00:30Z
- **Completed:** 2026-06-26T02:00Z
- **Tasks:** 3
- **Files modified:** 1 (94-DIVERGENCE-LEDGER.md created)

## Accomplishments

- Fetched nolabs-ai/nono window non-destructively, verified all four tip SHAs reachable, confirmed D-02 clean-continuation proof (byte-identical v0.64.0 anchor)
- Ran drift tool against explicit window SHAs (SHA-not-tag guard, D-03); 8 substantive commits identified from 17 total (0 merges, 9 out-of-filter noise); arithmetic balances
- Authored complete ledger with Headline, Reproduction, Cluster Summary, 4 per-cluster sections, mandatory Carve-out Re-touch Check, ADR Review, Empirical Cross-Check (8 spot-checks), and noise reconciliation
- Discovered and documented a new active conflict: upstream `9b37dc52` directly inverts the Phase 89 deliberate divergence (`0c08e5d2`); split guidance with APPLY/SKIP/PRESERVE instructions authored for Phase 95

## Task Commits

All three tasks produced content in the single ledger file and were committed together:

1. **Task 1: Fetch, drift tool, frontmatter + Reproduction + Headline** - `ef73adb3` (docs)
2. **Task 2: Cluster Summary, per-cluster sections, Carve-out Re-touch Check, ADR Review** - `ef73adb3` (same commit, Tasks 1-3 combined)
3. **Task 3: Empirical Cross-Check, noise reconciliation, finalize counts** - `ef73adb3` (same commit)

**Plan metadata:** (committed in final metadata commit below)

## Files Created/Modified

- `.planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md` - Complete UPST10 divergence ledger (562 lines)
- `ci-logs-local/drift/20260626T003610Z-v064-v0651-upst10.json` - Drift tool JSON output (gitignored)

## Decisions Made

1. `9b37dc52` classified as **split** (not fork-preserve): upstream removes `has_custom_credentials` from proxy activation, but the fork's Phase 89 deliberate divergence (`0c08e5d2`) adds it for fail-secure credential injection. Phase 95 must apply only the `credentials_intent` block fix and preserve the activation predicate + guard test.

2. `11fd10e0` classified as **split** (not fork-preserve): the tool-sandbox subsystem itself is absent from the fork and is a large new feature requiring its own introduction, but the additive audit.rs event types, sandbox/linux.rs fixes, and shared proxy surface changes are syncable with carve-out guidance.

3. `9e084cbb` (sigstore-verify dep bump, Cargo.lock + Cargo.toml only) classified under Cluster D **won't-sync** with note that the dep version target should be absorbed in Phase 95 DEPS review rather than cherry-picked.

4. `137bb15c` (one-line tool-sandbox/platform/linux.rs fix) classified under Cluster D **won't-sync** because the file it patches (`crates/nono-cli/src/tool-sandbox/platform/linux.rs`) is absent from the fork.

## Deviations from Plan

None - plan executed exactly as written. This is a docs/audit plan; no source/build/test files were modified; no cherry-pick occurred.

## Issues Encountered

None.

## Carve-out Re-touch Check Summary

| Carve-out | Verdict | Phase 95 guidance |
|-----------|---------|-------------------|
| CR-02 (`crates/nono/src/audit.rs`) | HIT — additive-only | Apply additive struct additions; CR-02 `records_verified: event_count > 0` not touched |
| CR-01 (`bindings/c/src/` FFI files) | clean — no re-touch in window | No action; `clear_last_call_state()` invariant not threatened |
| Cluster F (proxy + proxy_runtime) | HIT — two commits | 11fd10e0: split tls_intercept/ out; 9b37dc52: preserve Phase 89 activation predicate |

## Known Stubs

None — this is a docs/audit plan; no UI rendering or data wiring involved.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced. This plan is read-only (git fetch + inspection + documentation).

## Next Phase Readiness

- **Phase 95 (absorb):** Ledger is the cherry-pick contract. Clusters A (will-sync), B (split — exclude tool-sandbox dir + tls_intercept/ + preserve CR-02 + Cluster F guard tests), C (split — apply credentials_intent fix only; preserve proxy activation predicate), D sigstore dep → DEPS review.
- **Phase 97 (release):** Leapfrog floor >= 0.65.0 documented in Cluster D cross-ref.
- **Blockers:** None. New conflict (9b37dc52 vs 0c08e5d2) is fully documented with Phase 95 resolution guidance; no escalation required.

---
*Phase: 94-upst10-divergence-audit*
*Completed: 2026-06-26*

## Self-Check: PASSED

- [x] `.planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md` exists (562 lines)
- [x] `.planning/phases/94-upst10-divergence-audit/94-01-SUMMARY.md` exists (this file)
- [x] Commit `ef73adb3` exists (ledger creation)
- [x] Commit `29bf215c` exists (SUMMARY creation)
- [x] No source/build/test files modified; no cherry-pick
- [x] STATE.md and ROADMAP.md not modified (orchestrator-owned)
