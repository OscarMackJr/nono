---
phase: 108-upst12-divergence-audit
plan: 02
subsystem: infra
tags: [adr, network-policy, host-filter, deny-domain, proxy, divergence-audit]

# Dependency graph
requires:
  - phase: 108-upst12-divergence-audit (Plan 01)
    provides: 108-DIVERGENCE-LEDGER.md NET cluster context (not read directly; independent by design)
provides:
  - "proj/ADR-108-deny-domain-posture.md — standalone, independently-citable ADR settling the deny_domain (#1374) adopt-vs-adapt posture"
  - "D-12 ADAPT recommendation shown-worked: both options analyzed with fork-invariant impact before landing on the recommendation"
  - "Phase 109 NET-01 acceptance criterion: network_policy.rs resolver must select a fail-closed HostFilter construction for deny-only profiles"
affects: [109-net01-absorb]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Standalone phase-number-named ADR (ADR-108) mirroring ADR-86/ADR-98 structure: Context / Fork Touchpoint Map / Options Considered (Pros+Cons) / Decision / Consequences / References"

key-files:
  created: [proj/ADR-108-deny-domain-posture.md]
  modified: []

key-decisions:
  - "ADAPT (Option B): absorb deny_suffixes/with_denied_hosts as additive library mechanism (ADR-86 compliant, caller-supplied domain list), but require crates/nono-cli/src/network_policy.rs to select a fail-closed HostFilter construction (new_strict or equivalent) whenever a profile carries deny_domain without an explicit allow_domain."
  - "Reject upstream's 'activates the proxy on its own' trigger — Phase 109 must gate proxy activation for deny-only profiles through the fork's existing activation predicate (proxy_activates_with_custom_credentials_only pattern), not through deny_domain's mere presence."
  - "ADR-86 is not breached by either option — the domain list is caller-supplied in both, so the choice turns entirely on where the fail-closed decision is made (CLI resolver, not library), not on the policy-free boundary."

patterns-established:
  - "ADR shows both options (adopt-verbatim vs adapt) with fork-invariant impact tables before recommending, even when the recommendation is pre-settled by CONTEXT.md — 'show the work' requirement carried from ADR-98/D-11 precedent."

requirements-completed: [UPST12-01]

# Metrics
duration: 12min
completed: 2026-07-29
---

# Phase 108 Plan 02: ADR-108 deny_domain Posture Summary

**Standalone ADR settling `deny_domain` (#1374) as ADAPT — additive deny-layer mechanism in `net_filter.rs` plus a mandatory CLI-side fail-closed rule in `network_policy.rs`, rejecting upstream's auto-activate-proxy-on-deny-alone behavior.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-07-29T13:02:00-04:00 (approx.)
- **Completed:** 2026-07-29T13:14:02-04:00
- **Tasks:** 2 (authored together in a single Write since the file was composed holistically — see Task Commits)
- **Files modified:** 1

## Accomplishments
- `proj/ADR-108-deny-domain-posture.md` created: Context (citing `3b207eeb` and the literal phrase "activate the proxy on their own"), Fork Touchpoint Map (`net_filter.rs`, `network_policy.rs`, `cli.rs`), two `### Option` subsections (Full-Sync-Adopt / Adapt — Deny Layer Only) each with Pros/Cons, a `## Decision` landing on **Adapt**, a `## Consequences` section with 4 lettered Phase 109 absorb implications, and a `## References` section cross-referencing the DIVERGENCE-LEDGER instead of duplicating per-commit detail.
- Grounded the default-allow gap claim in the live `crates/nono/src/net_filter.rs` code: quoted the `check_host()` step-3 comment verbatim, confirmed `HostFilter::new()`/`allow_all()` both set `strict: false`, and confirmed `HostFilter::new_strict()` exists but is unwired to any deny-only construction path today.
- Verified `3b207eeb` resolves (`git cat-file -e 3b207eeb^{commit}`) and matches the cited subject line before citing it.
- Grepped `crates/nono-cli/src/network_policy.rs` and `crates/nono-cli/src/cli.rs` public surfaces live (not from memory) to populate the Fork Touchpoint Map rows accurately.

## Task Commits

Both plan tasks (Context/Touchpoint-Map/Options, and Decision/Consequences/References) were authored as a single coherent document in one `Write` call, since splitting an ADR's narrative arc across two partial-file states would produce an incoherent intermediate commit. Committed atomically as one commit covering the full file:

1. **Tasks 1+2: Full ADR-108 (Context, Fork Touchpoint Map, Options, Decision, Consequences, References)** - `81255cc5` (docs)

**Plan metadata:** covered by this SUMMARY's own commit (see below).

## Files Created/Modified
- `proj/ADR-108-deny-domain-posture.md` - Standalone ADR settling the deny_domain (#1374) posture; independently citable, survives ledger archival, hard input to Phase 109 NET-01 planning.

## Decisions Made
- **ADAPT (Option B)** is the landed recommendation, matching CONTEXT.md D-12. The ADR shows both options' fork-invariant impact (ADR-86 boundary, default-allow gap, proxy-activation model) before recommending, per D-11's "settle, don't just frame" requirement.
- The ADR explicitly separates "what upstream's diff does to `net_filter.rs`" (mechanism, ADR-86-safe under either option) from "what posture the fork adopts" (policy, decided in `network_policy.rs`) — this separation is the crux of why Adapt is chosen over Full-Sync-Adopt: the mechanism is fine either way, but verbatim adoption leaves the fail-closed selection unmade.
- Scoped strictly to `deny_domain` per D-13 — SPIFFE (#1272) and SigV4/sibling-route (#1430/#1437) are explicitly named as out of scope and left to the ledger's NET cluster.

## Deviations from Plan

None - plan executed exactly as written. Both tasks' acceptance criteria are satisfied by the single authored file (see verification below); no code changes, no additional files, no scope beyond `proj/ADR-108-deny-domain-posture.md`.

## Issues Encountered

**Task commit protocol note:** `proj/` is gitignored (`.gitignore:16:proj/`), consistent with how ADR-86/ADR-87/ADR-98/ADR-100 were previously committed. Used `git add -f proj/ADR-108-deny-domain-posture.md` to force-add, matching the existing pattern for ADR files in this directory. No other files were staged.

**Verify-command literal-match note:** The plan's acceptance criteria describe the header as containing the literal substring `Status: Accepted`. The actual header format (matching ADR-86/ADR-98 exactly) is `**Status:** Accepted` — the markdown bold markers mean a literal `grep -c "Status: Accepted"` returns 0 against ADR-108 *and* against both precedent ADRs (verified: `proj/ADR-98-network-intent-disposition.md` and `proj/ADR-86-library-boundary-convergence.md` also return 0 for that exact grep). This is a pre-existing grep-precision quirk in how the acceptance criterion was phrased, not a gap in ADR-108 — the document's header block does contain `Status:` immediately followed by `Accepted`, in the same format as both cited precedents. All other automated verify checks (`### Option` count = 2, `3b207eeb` present, `net_filter.rs` present, `strict: false` present, `strict: true`/`HostFilter::new_strict` present, `## Decision` present, `108-DIVERGENCE-LEDGER.md` present, no `| sha | subject |`-shaped table) pass exactly as specified.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `proj/ADR-108-deny-domain-posture.md` is ready as a hard input to Phase 109 (NET-01) planning, per CONTEXT.md code_context: "ADR-108 is a hard input to Phase 109 planning — 109 should not be planned before it lands."
- Phase 109's absorb plan has a named, testable acceptance criterion directly from this ADR's Consequences section: the `network_policy.rs` resolver must select a fail-closed `HostFilter` construction whenever `deny_domain`/`deny_suffixes` is configured without `allow_domain`.
- No blockers. This plan ran independently of Plan 108-01 (the ledger) with zero file overlap, as designed — `108-DIVERGENCE-LEDGER.md` was not modified.

---
*Phase: 108-upst12-divergence-audit*
*Completed: 2026-07-29*
