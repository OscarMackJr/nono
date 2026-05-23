---
phase: 45
slug: source-migration-aipc-g-04-resl-native-re-validation
plan: 03
req: REQ-RESL-NIX-04
disposition: STRUCTURALLY-COMPLETE-PENDING-LIVE-RUN
created: 2026-05-23
verifier: oscarmackjr-twg
phase_46_handoff: true
---

# Plan 45-03 Native RESL Re-validation Protocol

## Purpose

REQ-RESL-NIX-04 closes the Phase 38 REQ-AAHX-HOST-01 native re-validation
deferral, which was folded into v2.6 at milestone-open per ROADMAP.md § Phase
38 number reservation. The requirement is a tactical confirmation pass that the
Phase 27.2 audit-attestation transitive closure (REQ-AAHX-01..03) holds on a
native Linux + macOS host; it was host-blocked from the Windows dev host since
v2.4 close.

This document — together with `.github/workflows/phase-45-resl-native-host.yml`
— discharges the structural half of REQ-RESL-NIX-04. The live workflow run is
deferred to the Phase 46 orchestrator action per D-45-D1 and SC#3 ("tactical
confirmation pass only — does not block phase close if no gap is found").

REQ-RESL-NIX-04 closes as STRUCTURALLY-COMPLETE-PENDING-LIVE-RUN per
`.planning/templates/cross-target-verify-checklist.md` PARTIAL semantics.

## Workflow Invocation (Phase 46 orchestrator)

The Phase 46 orchestrator triggers the workflow exactly once:

```
gh workflow run phase-45-resl-native-host.yml -f gh_runner_os=both
gh run list --workflow=phase-45-resl-native-host.yml --limit 1
gh run watch <run-id>
```

The default `gh_runner_os=both` runs both jobs in parallel. Operator may
select `ubuntu-24.04` or `macos-latest` individually if one host is
unavailable (SC#3 explicitly says "one or both per host availability").

After the run completes, record the verdict in § Closure Disposition below AND
in `46-VERIFICATION.md` § Linked Closures.

## Expected `cargo test` Output (Phase 27.2 closure baseline)

The Phase 27.2 audit-attestation test re-enablement closed at fix commit
`2b7425e7` (2026-05-09). Source:
`.planning/phases/27.2-audit-attestation-test-re-enablement/27.2-04-SUMMARY.md`
§ Post-execution closure.

Both jobs should produce this output on a clean native runner:

```
running 2 tests
test audit_verify_reports_signed_attestation_with_pinned_public_key ... ok
test rollback_signed_session_verifies_from_audit_dir_bundle ... ok

test result: ok. 2 passed; 0 failed; 0 ignored
```

If either test is missing from the output, the runner may be filtering
`#[ignore]`-tagged tests — verify that `--include-ignored` is present in the
invocation. The workflow uses the canonical form:

```
cargo test -p nono-cli --test audit_attestation -- --include-ignored
```

## SC#3 Decision Tree

ROADMAP Phase 45 § Success Criterion 3:
> Phase 38 REQ-AAHX-HOST-01 native re-validation runs on a Linux host
> (one or both per host availability) and reports either:
> (a) `audit-attestation` regression coverage matches the Phase 27.2
>     transitive closure, OR
> (b) a host-native gap is surfaced with a documented follow-up disposition.
> Tactical confirmation pass only — does not block phase close if no gap
> is found.

### Branch (a) — Coverage matches

- Both jobs (Linux + macOS) exit 0 with the expected 2-tests-pass output.
- REQ-RESL-NIX-04 flips from PARTIAL (STRUCTURALLY-COMPLETE-PENDING-LIVE-RUN)
  to VERIFIED.
- Phase 46 orchestrator records the verdict in
  `.planning/phases/45-source-migration-aipc-g-04-resl-native-re-validation/45-03-NATIVE-RESL-PROTOCOL.md`
  § Closure Disposition AND in `46-VERIFICATION.md` § Linked Closures.
- The workflow `.github/workflows/phase-45-resl-native-host.yml` is a
  candidate for deletion in v2.7 per D-45-D2.

### Branch (b) — Host-native gap surfaced

- One or both jobs report a failure that the Phase 27.2 transitive closure did
  NOT predict (e.g., a Linux-only symptom in
  `audit_verify_reports_signed_attestation_with_pinned_public_key` that did not
  surface on Windows host).
- Capture the failing test name + stderr verbatim.
- File a follow-up todo at
  `.planning/todos/pending/45-resl-nix-04-host-native-gap-<short-id>.md`
  with: (i) failing test name, (ii) host (Linux or macOS), (iii) stderr
  excerpt, (iv) hypothesis (host-native symptom vs Phase 27.2 fix regression).
- REQ-RESL-NIX-04 closes as PARTIAL with the explicit gap reference; Phase 46
  records the disposition in `46-VERIFICATION.md`.
- This is NOT a Phase 45 close blocker (SC#3 "does not block phase close if no
  gap is found"); however, the follow-up todo MUST be filed so the gap is not
  lost.

## Phase 27.2 Transitive-Closure Mapping

Phase 38 REQ-AAHX-HOST-01 was originally a "Phase 27 reopen" per
`.planning/PROJECT.md` § v2.4 archive. Phase 27.2 (audit-attestation test
re-enablement; commits closed at SHA `2b7425e7`) transitively closed
REQ-AAHX-01 + REQ-AAHX-02 + REQ-AAHX-03 via:

- `crates/nono-cli/tests/audit_attestation.rs::audit_verify_reports_signed_attestation_with_pinned_public_key`
  (REQ-AAHX-01 — audit-bundle verification + pinned-public-key validation)
- `crates/nono-cli/tests/audit_attestation.rs::rollback_signed_session_verifies_from_audit_dir_bundle`
  (REQ-AAHX-03 — rollback verification from audit dir bundle)

Both tests are `#[ignore]`-by-default on Windows host (they require a working
`signed_session` flow that depends on Unix-native filesystem semantics);
`--include-ignored` re-enables them. The Phase 27.2 closure was on Linux +
macOS via `2b7425e7`; Plan 45-03's workflow is the live re-validation of that
closure on native runners post-v2.6 quiet-baseline.

The Phase 27.2 fix at `2b7425e7` resolved a tracing-subscriber stderr capture
issue that caused the tests to incorrectly fail in CI despite correct audit
bundle logic. The fix was mechanical (subscriber init guard order) and does not
affect the underlying audit-attestation correctness invariants.

## Closure Disposition (Phase 46 orchestrator fills this in)

This section is left empty by Plan 45-03 close. Phase 46 orchestrator
populates after `gh workflow run` completes:

```
Disposition: [VERIFIED | PARTIAL | DEVIATED]
Linux job run-id: <id>
Linux job verdict: [pass | fail | skipped]
macOS job run-id: <id>
macOS job verdict: [pass | fail | skipped]
Closure branch: [(a) coverage matches | (b) gap surfaced]
Follow-up todo (if branch b): <path>
Recorded at: <YYYY-MM-DD>
Recorded by: orchestrator
```

## Deletion / Cleanup (v2.7 candidate)

Per D-45-D2, the workflow `.github/workflows/phase-45-resl-native-host.yml` is
a tactical artifact (NOT a permanent CI lane) and MAY be deleted in v2.7 once
the verdict is recorded in § Closure Disposition above. The cleanup procedure:

1. Delete `.github/workflows/phase-45-resl-native-host.yml`
2. Add a deletion note to the Phase 45 audit trail

**The cleanup MUST NOT delete this protocol doc** — `45-03-NATIVE-RESL-PROTOCOL.md`
is part of the v2.6 audit trail for REQ-RESL-NIX-04 and must be retained
regardless of whether the workflow is deleted.

## References

- `.planning/ROADMAP.md` § Phase 45 SC#3 — binding success criterion
- `.planning/REQUIREMENTS.md` § REQ-RESL-NIX-04 — binding requirement
- `.planning/phases/45-source-migration-aipc-g-04-resl-native-re-validation/45-CONTEXT.md`
  § D-45-D1 + D-45-D2 — locked decisions governing this plan
- `.planning/phases/45-source-migration-aipc-g-04-resl-native-re-validation/45-RESEARCH.md`
  § Plan 45-03 — research-supported invocation pattern + Phase 27.2 baseline citation
- `.planning/phases/27.2-audit-attestation-test-re-enablement/27.2-04-SUMMARY.md`
  § Post-execution closure — Phase 27.2 baseline (fix commit `2b7425e7`, 2026-05-09)
- `.github/workflows/phase-45-resl-native-host.yml` — the workflow this protocol orchestrates
- `.github/workflows/phase-37-linux-resl.yml` — layout precedent (SHA pins reused verbatim)
- `.planning/templates/cross-target-verify-checklist.md` — STRUCTURALLY-COMPLETE-PENDING-LIVE-RUN
  closure semantics inherited by REQ-RESL-NIX-04 per D-45-D1
