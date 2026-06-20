# ADR-87: CR-02 Audit-Integrity Fix — `records_verified` Semantic Hardening

**Status:** Accepted
**Phase:** 87 — Security Sync
**Date:** 2026-06-20
**Authors:** Phase 87 execution

---

## Context

`verify_audit_log` in `crates/nono/src/audit.rs` returned `records_verified: true` regardless
of whether any records were processed. The upstream codebase contains the same hardcoded value
at commit `e9529312`.

When called with an empty log and `stored: None` (no stored integrity metadata to verify
against), the function returns:

- `event_count: 0`
- `records_verified: true`  (no records were verified — 0 iterations)
- `event_count_matches: true`  (vacuously, `unwrap_or(true)`)
- `chain_head_matches: true`  (vacuously)
- `merkle_root_matches: true`  (vacuously)

This caused `is_valid()` to return `true` for an empty log with no stored metadata — signalling
"all integrity checks passed" when in fact no integrity claim was made. Callers that use
`is_valid()` to gate security-relevant decisions (e.g., audit log freshness checks, session
attestation) could be misled.

This was identified during Phase 86 code review (86-REVIEW.md IN-02) as an upstream-inherited
issue and was explicitly deferred to Phase 87 via D-12.

---

## Decision

Set `records_verified` to `event_count > 0` in the `verify_audit_log` return value.

This is the **first intentional divergence** from upstream's `audit.rs` code since Phase 86
convergence. It is recorded here to ensure future upstream sync auditors recognize the
divergence as deliberate and preserve the fork's behavior when the upstream line reappears.

---

## Rationale

- **Security-relevant semantics:** Callers must not mistake "nothing checked" for "everything
  passed." The field name `records_verified` implies a positive claim; returning `true` when
  zero records were verified violates least-surprise security semantics.

- **`is_valid()` contract:** The public API contract of `is_valid()` is "all integrity checks
  passed." With an empty log and no stored metadata, no check was performed — `is_valid()`
  returning `true` in this case is a semantic bypass.

- **Minimal fix:** The change is a single expression substitution. All non-empty log paths are
  unaffected — the function returns `Err` on any record-level verification failure before
  reaching the return site, so `event_count > 0` will be `true` for all successful non-empty
  verifications.

- **Test coverage:** A regression test `verify_empty_log_with_no_stored_metadata_is_not_valid`
  documents the intended behavior and guards against future reversion.

---

## Consequences

- **Future upstream syncs:** The line `records_verified: true` (at upstream `e9529312`) will
  conflict with the fork's `records_verified: event_count > 0`. This is expected and
  intentional. Sync auditors MUST preserve the fork's expression.

- **Divergence tracking:** This divergence is recorded in
  `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` as a Phase 87 CR-02
  addendum. The ledger entry provides the upstream reference commit, fork behavior, and
  classification (deliberate fork-divergence / security hardening).

- **Caller behavior:** Any caller that previously relied on `is_valid()` returning `true` for
  empty logs will now receive `false`. Empty-log scenarios are not expected in production
  sessions (every session records at least `session_started` and `session_ended` events).
  Tests that call `verify_audit_log` with an empty file and expect `is_valid() = true` must
  be updated.

---

## References

- Upstream commit: `e9529312` — `records_verified: true` hardcode in `verify_audit_log`
- Phase 86 code review: 86-REVIEW.md IN-02 (deferred to Phase 87)
- Divergence ledger: `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md`
  § Phase 87 CR-02 Addendum
- RESEARCH.md §CR-02 Detailed Analysis (Phase 87)
