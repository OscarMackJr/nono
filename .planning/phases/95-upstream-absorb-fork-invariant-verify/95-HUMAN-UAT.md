---
status: resolved
phase: 95-upstream-absorb-fork-invariant-verify
source: [95-VERIFICATION.md]
started: 2026-06-26
updated: 2026-06-26
---

## Current Test

[resolved — superseded by Phase 96 local cross-target toolchain]

## Tests

### 1. Cross-target Clippy confirmation on GitHub Actions (HEAD 544cab40)
expected: The GH Actions Linux (`x86_64-unknown-linux-gnu`) and macOS (`x86_64-apple-darwin`) Clippy lanes on the gap-closure HEAD `544cab40` run `cargo clippy --workspace --target <triple> -- -D warnings -D clippy::unwrap_used` to completion and exit 0, confirming the restored cfg-gated Unix code (linux.rs WR-02/WR-03, exec_strategy.rs WR-01, reverse.rs CR-01) compiles clean on both targets.
result: RESOLVED — superseded by Phase 96. The CI lanes were the decisive gate only *until* the local cross C toolchain existed; Phase 96 (XTGT-01/02/03) stood up that toolchain on the Windows dev host and ran BOTH gates GREEN locally — `cross clippy --target x86_64-unknown-linux-gnu` (Docker + cross 0.2.5) and `cargo-zigbuild clippy --target x86_64-apple-darwin` (zig 0.16.0 + cargo-zigbuild 0.23.0) — on the synced tree carrying the restored cfg-gated Unix code (linux.rs WR-02/WR-03, exec_strategy.rs WR-01, reverse.rs CR-01). The local Linux gate even caught a *separate* regression (dropped SEC-01 AF_UNIX filter), proving it genuinely exercises the cfg(linux) branches. The intent of this scenario — confirm the restored Unix code compiles clean on both targets — is satisfied by Phase 96's local runs; the PARTIAL→CI default is now retired per the updated cross-target-verify-checklist.
note: Closed at v3.3 milestone completion 2026-06-26.

## Summary

total: 1
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
