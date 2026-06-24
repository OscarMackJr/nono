# TODO: `crates/nono-cli/src/audit_ledger.rs` is an orphan module (never compiled)

**Captured:** 2026-06-24 (surfaced during PR #12 dead_code triage + audit_attestation investigation)
**Severity:** medium — latent: an append-only audit-ledger feature exists in source but is dead code, and the divergence weakens `nono audit verify` coverage vs upstream
**Source:** PR #12 debug session `.planning/debug/resolved/ci-linux-cfg-compile-errors.md`

## Problem
`crates/nono-cli/src/audit_ledger.rs` has **no `mod audit_ledger;` declaration anywhere** in the crate, so it is never compiled. Consequences observed during PR #12:
- Its items (`AUDIT_LEDGER_FILENAME`, `maybe_migrate_legacy_audit_ledger`, `append_session`, …) are dead on every platform — they had to be `#[allow(dead_code)]`'d (commit `c64c5977`), not cfg-gated, because no live caller exists.
- `append_session` is never invoked in the run path, so the upstream **append-only audit ledger** feature is effectively NOT wired into this fork.
- This is why `nono audit verify --json` emits the fork's flat `{integrity, attestation_present, attestation_valid}` shape rather than upstream's richer `{session, ledger, attestation}` envelope (the `audit_attestation` integration tests were updated to the flat shape in PR #12, commit `5b2fdad6`).

## Decision needed
- **Wire it in** — add `mod audit_ledger;`, call `append_session` at session finalization, cfg-gate the `nix::fcntl::Flock` usage for Windows, and enrich `cmd_verify` to report ledger-chain verification (reaching upstream parity), OR
- **Remove it** — delete `audit_ledger.rs` and the now-`allow(dead_code)`'d `AUDIT_LEDGER_FILENAME` / `maybe_migrate_legacy_audit_ledger` if the fork deliberately does not want the append-only ledger.

## Acceptance
Either `audit_ledger.rs` is declared + wired into the run/verify path (with cross-platform cfg handling) and exercised by a test, OR it and its dead helpers are removed and the `#[allow(dead_code)]` markers added in `c64c5977` are no longer needed.
