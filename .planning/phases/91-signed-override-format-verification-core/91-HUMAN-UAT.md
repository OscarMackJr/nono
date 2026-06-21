---
status: partial
phase: 91-signed-override-format-verification-core
source: [91-VERIFICATION.md, 91-REVIEW.md]
started: 2026-06-21
updated: 2026-06-21
---

## Current Test

[awaiting human decision]

## Tests

### 1. W-01 / W-03 disposition (fail-secure hardening)
expected: Decide whether to fix the two code-review warnings the reviewer recommends addressing before Phase 92 consumes this verifier as a live trust anchor:
- **W-01** (`override.rs:872,877`): chrono `DateTime ± Duration` skew math **panics** on integer overflow with attacker-controlled `expires_at`/`not_before`/`iat`. CLAUDE.md mandates `checked_`/`saturating_` arithmetic and "libraries should almost never panic" + "fail secure on any error." A crafted token should DENY (return `Err`), not abort the interpreter (DoS). Fix = `checked_add_signed`/`checked_sub_signed` → `Err(OverrideErrorKind::...)`.
- **W-03** (`override.rs:782-786`): jti is consumed at step 9, before the (currently infallible) step 10. Safe today, but fragile for Phase 92 which may add fallible steps after — a non-replay failure could burn a legitimate one-time nonce. Fix = move jti consume to last-before-`Ok`.
result: [pending]

### 2. VFY-03 partial-coverage scope split
expected: Confirm the deliberate scope split is sanctioned. REQUIREMENTS.md maps VFY-03 to Phase 91, but the plans split it: clause (b) "no AWS SDK / no credentials in the offline verifier" is satisfied NOW (verified — zero AWS SDK in Cargo.toml); clause (a) "pubkey sourced from env/policy + key-ARN allowlist wired to live config" is deferred to Phase 93 with explicit `[BLOCKING-93]` markers.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
