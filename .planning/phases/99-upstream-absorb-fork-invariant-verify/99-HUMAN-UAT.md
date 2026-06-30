---
status: partial
phase: 99-upstream-absorb-fork-invariant-verify
source: [99-VERIFICATION.md, 99-REVIEW.md]
started: 2026-06-30
updated: 2026-06-30
---

## Current Test

[awaiting human decision on WR-01 and WR-03]

## Tests

### 1. WR-01 — Unused sigstore-trust-root 0.9.0 direct dependency (Cluster F absorb fidelity)
expected: `crates/nono/Cargo.toml:54` pins `sigstore-trust-root = "=0.9.0"` as a direct dep, but no code in `crates/nono/src/` uses it; verification still runs on the transitive `0.8.0` (via `sigstore-verify = "0.8.0"`). Per 99-RESEARCH cascade assumption A1, the `sigstore-verify` 0.9 bump was intentionally deferred, so the 0.9.0 pin is a deferred-upgrade marker with no runtime effect. DECISION: keep the pin (documents the absorb + deferred cascade) vs remove it (cleaner, avoids dead-dep, but reverts Cluster F to cosmetic-only). The comment implying an active TUF-root security upgrade should be corrected to state the effective version regardless.
result: [pending]

### 2. WR-03 — validate_block_net_conflicts vs strict_filter semantic contradiction
expected: `sandbox_prepare.rs:204` folds `prepared.profile_network_block` into `block_net`, so the validator REJECTS a profile with `block: true` + `allow_domain`/proxy config as contradictory. But `proxy_runtime.rs:174` sets `strict_filter = prepared.profile_network_block` to RUN exactly that case in strict-filter mode. The two designs contradict; the validator preempts strict_filter. Test `profile_network_block_with_credential_from_profile_errors` locks in the reject behavior. DECISION (product): should a `block:true`+`allow_domain` profile error early, or run in strict-filter mode? Fail-closed today, so a consistency bug not a vuln.
result: [pending]

### 3. WR-02 — --allow-http2 / network.allow_http2 is a runtime no-op (Phase 100 tracking)
expected: `UpstreamPool::send()` is never called from `handle_reverse_proxy` (still uses the `tls_connector` path), so `--allow-http2` silently yields HTTP/1.1. The pooling infrastructure (pool.rs) was absorbed structurally from #983 but is not yet wired. ACTION before advertising H2 in Phase 100 release notes: either wire `UpstreamPool` into the reverse-proxy path or add a help-text note that H2 is not yet active.
result: [pending]

### 4. SC3 — 11 nono-cli test failures are pre-existing baseline, not Phase 99 regressions
expected: Both D-08 deviation tests pass; the 11 failures are confined to files Phase 99 never modified.
result: RESOLVED 2026-06-30 by orchestrator. `cargo test -p nono-cli` → 1387 passed, 11 failed. Both `test_wsl2_proxy_policy_deviation_preserved` and `test_compiled_endpoint_policy_compat_deviation_preserved` report `ok`. All 11 failures are in `config::tests::*` (6, env-lock PoisonError), `audit_session` (1), `profile_cmd` (1), `protected_paths` (3) — none of which Phase 99 touched. Count is 11 vs the prior memory snapshot of 4 because the 6 `config::tests` failures are the documented env-var parallel-pollution flakiness (CLAUDE.md). Not a regression.

## Summary

total: 4
passed: 1
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
