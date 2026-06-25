# TODO: Cargo Audit — bump `quinn-proto` past RUSTSEC-2026-0185 (remote memory exhaustion)

**Captured:** 2026-06-24 (PR #12 CI triage)
**Severity:** medium — `Cargo Audit` CI check is red; transitive dep, not in a hot path, but it is the only hard `error` from `cargo audit`
**Source:** PR #12 `Cargo Audit` job; pre-existing/time-based advisory (fails on `main` too — NOT a v3.x regression)

## Problem
`cargo audit` fails on **RUSTSEC-2026-0185** — *Remote memory exhaustion in `quinn-proto` from unbounded out-of-order stream reassembly*. `quinn-proto` is a transitive dependency.
```
Crate:  quinn-proto
Title:  Remote memory exhaustion in quinn-proto from unbounded out-of-order stream reassembly
ID:     RUSTSEC-2026-0185
error: 1 vulnerability found!
warning: 4 allowed warnings found
```
Advisory: https://rustsec.org/advisories/RUSTSEC-2026-0185

## Fix
1. `cargo tree -i quinn-proto` — identify the parent pulling it in.
2. `cargo update -p quinn-proto` to a patched version (or bump the parent if pinned).
3. Confirm `cargo audit` passes and the workspace builds across targets.

## Also flagged in the same job (lower priority — `unmaintained` warnings, not vulns)
- RUSTSEC-2025-0052 `async-std` (discontinued) · RUSTSEC-2025-0057 `fxhash` · RUSTSEC-2024-0436 `paste` · RUSTSEC-2025-0134 `rustls-pemfile`

## Acceptance
`Cargo Audit` CI check is green (or the residual advisories are explicitly allow-listed in `audit.toml`/`deny.toml` with a documented rationale).
