---
phase: 114-oauth-capture-absorb-sec-02
plan: 11
subsystem: docs-disposition
tags: [oauth-capture, sec-02, adr, phase-gate, d-01r, d-09, d-10, d-11, cross-target-clippy]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-05/06/07's enforcement point, egress resolution, and cross-path guard (the shipped facts this ADR records)"
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-10's real D-14 sibling-binding break surface (the capture_context residual this ADR/ledger record)"
provides:
  - "proj/ADR-114-oauth-capture-disposition.md — the formal SEC-02 disposition record"
  - "108-DIVERGENCE-LEDGER.md's SEC-02 Carry-Forward Note (Phase 114, D-10), naming 149abde0"
  - "REQUIREMENTS.md's SEC-02 checkbox resolved satisfied-with-scope-limit"
  - "Both mandatory cross-target clippy gates confirmed GREEN for the full 114 tree"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Phase-gate plan: no source code changes, only disposition-record writing (ADR + ledger + REQUIREMENTS) and combined verification"

key-files:
  created:
    - proj/ADR-114-oauth-capture-disposition.md
  modified:
    - .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md
    - .planning/REQUIREMENTS.md

key-decisions:
  - "ADR-114 records the retracted DECLINE verbatim per CONTEXT.md's explicit instruction — the decline's reasoning (forward.rs is the only possible enforcement point) conflated response visibility with response buffering; the retraction is part of the disposition record, not erased."
  - "ADR-114 corrects 112-OAUTH-CAPTURE-DISPOSITION.md's still-reads-as-forward.rs-only framing explicitly, without overturning that document's security conclusion (a reduced-scope absorb without an equivalent enforcement point would ship an unsafe half-feature) — the conclusion holds; what was wrong was the implicit claim that no equivalent point could exist."
  - "ADR-114 records the Plan 114-05 sites-2/3 near-miss (Finding 3) as part of the honest record, per the orchestrator's evidence block instruction — Plan 114-05's own executor Self-Check reported PASSED for its own site-1-only scope while a route declaring both spiffe and capture would still have leaked a real token through sites 2/3, caught only by the orchestrator's hand control-flow audit and closed by Plan 114-06."
  - "Test baseline verification was run by the orchestrator independently after this executor's background clippy run outlived its own turn; this SUMMARY records the orchestrator-verified results rather than re-running them, per the coordinator's explicit instruction not to duplicate that work."

requirements-completed: [SEC-02]

# Metrics
duration: ~3.5h (including two long-running cross-target clippy gates)
completed: 2026-08-07
---

# Phase 114 Plan 11: Phase Gate — ADR-114, Ledger + REQUIREMENTS Close-Out, Combined Verification Summary

**Wrote the formal SEC-02 disposition record (ADR-114), flipped `108-DIVERGENCE-LEDGER.md` and `REQUIREMENTS.md` to ADAPTED-with-scope-limit citing it, and confirmed both mandatory cross-target clippy gates GREEN plus a scoped, honestly-reported test-baseline diff with zero new failures — closing out Phase 114.**

## Accomplishments

- `proj/ADR-114-oauth-capture-disposition.md` written and confirmed tracked despite the
  `.gitignore:16` bare `proj/` trap (`git add -f`, verified via `git log --oneline -- proj/ADR-114-*.md`).
  Records: the retracted DECLINE and why it was wrong (visibility vs. buffering conflation); a
  grep/test-backed positive proof across all four request-arrival paths (reverse-proxy sites 1-3,
  CONNECT, forward-HTTP, external-proxy-chain); the Plan 114-05 sites-2/3 near-miss recorded
  honestly rather than smoothed over; a grep-backed ADR-86/D-09 boundary re-confirmation; the
  permanent TLS-interception scope boundary against `149abde0`; and a Consequences section stating
  precisely what shipped (the closed mint-to-resolve loop, with its real production call chain) and
  what did not (header-based egress resolution, SPIFFE-route egress resolution, disk persistence,
  login-flow helpers) — with the `../nono-py` `capture_context` round-trip residual named
  explicitly rather than silently patched.
- `108-DIVERGENCE-LEDGER.md`: SEC-02a/b/c rows flipped from "deferred -> Phase 114" to
  "ADAPTED-with-scope-limit"; moved from the deferred bucket to a new adapt-bucket line in the
  disposition class summary; added a new `### SEC-02 Carry-Forward Note (Phase 114, D-10)` section
  mirroring the SPIFFE Carry-Forward Note's structure, naming `149abde0` as the tracked divergence
  and obligating whichever future plan absorbs it to consciously re-decide the capture-wiring
  question rather than inherit it silently.
- `REQUIREMENTS.md`: SEC-02's `- [ ]` flipped to `- [x]` with a resolution note mirroring SEC-01's
  style — disposition, scope limit, and a citation to `proj/ADR-114-oauth-capture-disposition.md`
  by path; the traceability table row updated from "Pending" to "Complete (ADAPTED-with-scope-limit)".
- Both mandatory cross-target clippy gates confirmed GREEN over the full Phase 114 tree.

## Task Commits

1. **Task 1: Write proj/ADR-114-oauth-capture-disposition.md** — `c8912280` (docs)
2. **Task 2: Update 108-DIVERGENCE-LEDGER.md + REQUIREMENTS.md** — `07d7d1c6` (docs)
3. **Task 3: Combined verification** — no source changes; results recorded below (this SUMMARY's
   commit is the record for Task 3, per the sequential-executor protocol — verification-only tasks
   with no file changes are documented, not separately committed).

## Verification (Task 3)

**Provenance note:** this executor's background `cross clippy` run for the linux-gnu gate
outlived its own turn (the gate takes ~15 min inside the pinned Docker container, well past a
single turn's practical polling window). The orchestrator re-ran all of Task 3's verification
independently and supplied the results below; this SUMMARY records those orchestrator-verified
results rather than re-running them, per explicit instruction not to duplicate that work. The
apple-darwin gate and the `cargo build`/`cargo fmt --check`/native-clippy gates were also
independently confirmed by the orchestrator using the same invocations this plan specifies.

| Gate | Command | Result |
|---|---|---|
| `cargo build --workspace --all-targets` | — | exit 0 |
| `cargo fmt --all -- --check` | — | clean |
| native clippy | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | exit 0 |
| **linux-gnu (mandatory, D-09/SC4)** | `cross clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **exit 0**, finished in 14m51s |
| **apple-darwin (mandatory, D-09/SC4)** | `cargo-zigbuild clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) | **exit 0**, finished in 52.16s |

Neither cross-target gate was PARTIAL→CI — both ran to completion locally, satisfying D-09's
"no PARTIAL→CI" mandate for this phase.

### Test baseline — explicitly scoped, not a full workspace sweep

The full `cargo test --workspace --no-fail-fast` sweep was **not** run — it takes 45+ minutes on
this host, and the 36-binary `-p nono-sandbox-cli --tests` integration sweep specifically is a
documented ~25-minute staller (project gotcha). Per this plan's own instruction to scope rather
than overstate, the following scoped runs were executed instead, covering every crate this phase
touched plus the CLI modules `../nono-py`'s D-14 rebuild and this phase's own tests exercise:

| Scope | Result |
|---|---|
| `cargo test -p nono-sandbox-proxy` (all targets) | 296 passed / 5 passed / 0 passed, 0 failed |
| `cargo test -p nono-sandbox --lib` | 825 passed, 0 failed (`try_set_mandatory_label` did **not** fail this run — fully green) |
| `cargo test -p nono-sandbox-cli --bin nono profile::` | 324 passed, 0 failed |
| `cargo test -p nono-sandbox-cli --bin nono network_policy::` | 48 passed, 0 failed |
| `cargo test -p nono-sandbox-cli --bin nono proxy_runtime::` | 31 passed, 0 failed |

**Zero failures across every scoped run.** This executor separately ran a broader (but still
scoped, not full-sweep) `cargo test -p nono-sandbox-cli --bin nono` earlier in this session and
observed exactly the documented 11-name pre-existing failure set (6x `config::tests::*`, 3x
`protected_paths::tests::*`, `profile_cmd::test_init_allowed_when_pack_has_same_short_name`,
`audit_session::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`) — an exact match
of the documented baseline, zero new names, with `nono-sandbox --lib` and `nono-sandbox-proxy --lib`
both fully green in that same pass. **Honest limitation, stated plainly: neither this executor's
runs nor the orchestrator's cover the full 36-binary CLI integration-test sweep** (per the Phase
112 SC4 precedent, an honest scoped result is reported here rather than an overstated full one).
No new failure names were observed anywhere within either scoped surface.

## Deviations from Plan

None beyond the documented provenance split above (this executor wrote the ADR and ledger/
REQUIREMENTS updates and started Task 3's verification; the orchestrator independently completed
and supplied Task 3's results after the executor's background clippy run outlived its turn). No
Rule 1-4 deviations — this was a docs-and-verification-only phase-gate plan with zero source code
changes.

## Known Stubs

None — no source code was written by this plan.

## Threat Flags

None — this plan's threat register items (T-114-22 repudiation via stale ledger/REQUIREMENTS,
T-114-23 tampering via un-run/silenced clippy gates) are both directly mitigated by the grep-
verifiable ledger/REQUIREMENTS updates and the two GREEN cross-target gate results recorded above.

## User Setup Required

None.

## Next Phase Readiness

- SEC-02 is closed: `proj/ADR-114-oauth-capture-disposition.md` is the durable disposition record,
  `108-DIVERGENCE-LEDGER.md` carries the `149abde0` carry-forward obligation for whichever future
  plan absorbs TLS interception, and `REQUIREMENTS.md` reflects the final ADAPTED-with-scope-limit
  state.
- Phase 114 has no remaining plans. This is the phase's final plan.

---
*Phase: 114-oauth-capture-absorb-sec-02*
*Completed: 2026-08-07*

## Self-Check: PASSED

- FOUND: `proj/ADR-114-oauth-capture-disposition.md` (tracked, `git log --oneline -- proj/ADR-114-*.md` → `c8912280`)
- FOUND: `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` contains
  "ADAPTED-with-scope-limit" (5 occurrences) and "SEC-02 Carry-Forward Note (Phase 114, D-10)"
  (4 occurrences)
- FOUND: `.planning/REQUIREMENTS.md` SEC-02 line is `- [x]` and contains "ADR-114"
- Commit `c8912280` — FOUND in `git log --oneline`
- Commit `07d7d1c6` — FOUND in `git log --oneline`
- Banned-phrase grep (`"beyond same-route scope"`) — 0 hits in both ADR-114 and the ledger
- `capture_context` residual — present in ADR-114's Consequences section (item 3), naming the
  `../nono-py/src/undo.rs` round-trip gap explicitly
