# Phase 55: UPST7 Cherry-pick Wave - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-04
**Phase:** 55-upst7-cherry-pick-wave
**Areas discussed:** Scope reconciliation, C13 sigstore bump, Release-scope timing, Plan slicing

---

## Scope reconciliation (REQ/SC vs audit routing)

| Option | Description | Selected |
|--------|-------------|----------|
| Ledger routing + amend artifacts | Execute the ledger's Phase-55 set (C4/C7/C9/C10/C11/C12 + C13-Cargo); edit REQ-UPST7-02 + ROADMAP SC1 to drop phantom java-dev (0 commits in range) and add omitted C9/C12/C13 | ✓ |
| Ledger routing; java-dev N/A in SUMMARY only | Same execution; leave REQ/SC text as-is, document drift in SUMMARY only | |
| Hunt java-dev in a later range now | Pull java_runtime from v0.60.0+ into this wave (scope creep; violates audit immutability) | |

**User's choice:** Ledger routing + amend artifacts (D-55-01)
**Notes:** Maintainer wants the written acceptance criteria to track the audit-of-record, not the older 260527-sgo gap-analysis prose. The `54-DIVERGENCE-LEDGER.md` itself stays immutable; the amendment edits REQUIREMENTS.md + ROADMAP.md only.

---

## C13 sigstore 0.8.0 split handling

| Option | Description | Selected |
|--------|-------------|----------|
| Diff-inspect-first, then port+verify | Mirror Phase 48 C9: scrub.rs vs Phase-49 trust-root diff-inspection artifact; port Cargo bump + scrub.rs if clean, else D-20 replay/defer scrub.rs | ✓ |
| Cargo bump only; defer scrub.rs | Port the 0.8.0 dep bump now, defer scrub.rs to a follow-up todo | |
| Defer entire C13 to UPST8 / post-release | Don't perturb the signing/trust surface during the wave | |

**User's choice:** Diff-inspect-first, then port+verify (D-55-02)
**Notes:** Conservative-by-default with upgrade authority. The Cargo bump ripples Cargo.lock workspace-wide (5 crates). Upgrade to port both Cargo + scrub.rs only if diff-inspection clears the Phase-49 surface + the D-32-15 offline-verify invariant. Resolution captured in `55-NN-C13-DISPOSITION-RESOLUTION.md`.

---

## Release-scope timing

| Option | Description | Selected |
|--------|-------------|----------|
| Execute now on held branch; merge after v0.58.0 tag | Do the work now on a feature branch; merge-to-main gated on v0.58.0 tag + sign; land as v0.59.0/next | ✓ |
| Block Phase 55 until v0.58.0 ships first | Don't start execution until the tag lands (hard sequencing gate) | |
| Proceed straight to main now | Accept the cherry-picks ride into the next signed release | |

**User's choice:** Execute now on held branch; merge after v0.58.0 tag (D-55-03)
**Notes:** Honors the Phase 54 release-scope guard (`quick-260604-nue`). Work proceeds now so it isn't stalled on the Azure signing cert; the merge-to-main is the v0.58.0 gate so the cherry-picks don't ride into the signed v2.9 release.

---

## Plan slicing

| Option | Description | Selected |
|--------|-------------|----------|
| One plan per cluster (~7 plans) | 55-01..55-07, Phase 48 precedent; max traceability + per-cluster rollback; surface-disjoint parallel waves | ✓ |
| Consolidate the polish clusters | Merge C9/C10/C12 into one POLISH-BATCH plan (~5 plans total) | |
| Planner's call at plan-phase | Defer slicing entirely to gsd-planner after surface-overlap analysis | |

**User's choice:** One plan per cluster (D-55-04)
**Notes:** Planner still owns wave grouping + the surface-overlap analysis within the one-per-cluster default (known overlaps: C7+C12 on `policy.rs`; C10/C11 possible `exec_strategy.rs`/`diagnostic.rs` intersection).

---

## Claude's Discretion

- Plan numbering + cluster-theme names (55-01..55-07 suggested; planner may refine).
- Wave grouping within one-plan-per-cluster + surface-overlap analysis at plan-open.
- C13 diff-inspection artifact name + the upgrade-or-replay outcome.
- Per-plan close-gate composition (inherits Phase 34 D-34-D2 8-check; skip/add with categorization).
- Exact mechanism of the REQ/SC amendment (standalone docs commit vs folded into first plan).
- Cherry-pick chronological order within each cluster (verified at plan-open via `git log`).

## Deferred Ideas

- `java-dev` / `java_runtime` profile (0 commits in v0.57.0..v0.59.0) → UPST8 if upstream ships it.
- C3 (allow_domain) + C5 (TLS-intercept rider) → Phase 56 (REQ-NET-01).
- C6 (`bw://` Bitwarden) → Phase 57 (REQ-CRED-01).
- C8 (session hooks) → Phase 58 (REQ-HOOK-01; Windows ADR).
- C2 (supervisor named-socket IPC) → Phase 59 (REQ-IPC-01; Windows AIPC fork-preserve).
- rcgen 0.13.2→0.14.8 (`8e78daf`) → won't-sync (absent `tls_intercept/` module).
- UPST8 audit (`v0.60.0..v0.61.1`, growing) → fires after Phase 55 closes.
