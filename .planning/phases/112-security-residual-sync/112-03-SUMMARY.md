---
phase: 112-security-residual-sync
plan: 03
subsystem: security
tags: [trust-policy, sigstore, attestation, profile, env-vars, adr, upstream-sync]

# Dependency graph
requires:
  - phase: 112-01
    provides: The Wave 1 reality-check disposition table (112-DISPOSITION-TABLE.md) that finalized SEC-04 -> adopt and SEC-08 -> won't-sync-verbatim/ADR-112-flagged
provides:
  - "TrustPolicy predicate discriminator (TRUST_POLICY_PREDICATE + predicate: Option<String> field) so foreign trust-policy.json files (e.g. AWS IAM) are skipped with a warning instead of crashing nono's trust loader"
  - "TRUST_POLICY_VERSION deprecated (since 0.66.0) and dropped from the crate::trust re-export surface; version field is now Option<u32>, informational only"
  - "load_nono_policy() peek-before-full-parse loader in nono-cli's trust_scan.rs, used by both trust_scan's load_scan_policy and trust_cmd's load_trust_policy/run_sign_policy"
  - "proj/ADR-112-allow-vars-fail-closed-preserved.md: durable record that upstream a5a441c2's allow_vars Option<Vec<String>> reinterpretation is explicitly rejected, preserving the fork's fail-closed default"
affects: [113-oauth-capture-deferred, future-upstream-syncs-touching-trust-or-allow_vars]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Peek-before-full-parse: read raw JSON as serde_json::Value first, check a discriminator field, only attempt full struct deserialization if the discriminator matches — avoids confusing schema-parse errors on identically-named foreign files"
    - "Contingent ADR escalation (D-05 pattern): when a disposition table finds an upstream adopt that would loosen an existing fork security guard, escalate to a standalone ADR that records PRESERVE with zero code change, rather than silently skip or silently adopt"

key-files:
  created:
    - proj/ADR-112-allow-vars-fail-closed-preserved.md
  modified:
    - crates/nono/src/trust/types.rs
    - crates/nono/src/trust/policy.rs
    - crates/nono/src/trust/mod.rs
    - crates/nono-cli/src/trust_cmd.rs
    - crates/nono-cli/src/trust_scan.rs
    - docs/cli/features/trust.mdx
    - tests/integration/test_trust_cli.sh

key-decisions:
  - "SEC-04 ported verbatim from upstream f943fb5a: predicate field added, TRUST_POLICY_VERSION deprecated (kept, not removed, for backward-compat parsing of existing files), validate_version() only rejects a present-but-wrong predicate (absent predicate is legal at the library layer; the CLI's load_nono_policy is what skips foreign files)"
  - "validate_version()'s predicate check uses nested if-let, not upstream's let-chain (if let ... && ...) syntax, to stay compatible with this fork's MSRV 1.82 / edition 2021 without relying on let-chains"
  - "SEC-08: PRESERVE, do not adopt upstream a5a441c2 in any form (verbatim or adapted) — recorded in proj/ADR-112-allow-vars-fail-closed-preserved.md per CLAUDE.md's fail-secure/most-restrictive-option mandate; zero change to crates/nono-cli/src/profile_runtime.rs"
  - "docs/cli/features/trust.mdx: updated all 4 trust-policy.json examples (including the fork-only GitLab-signed-files section upstream doesn't have) for consistency, not just the 3 upstream's own diff touched"

patterns-established:
  - "Peek-before-full-parse loader pattern (load_nono_policy) for any future JSON config format that risks filename collision with a foreign tool's config"

requirements-completed: [SEC-04, SEC-08]

# Metrics
duration: ~25min
completed: 2026-08-05
---

# Phase 112 Plan 03: Trust-Policy Predicate Discriminator + Preserve Fail-Closed allow_vars Summary

**Ported upstream's trust-policy predicate discriminator (SEC-04) so foreign `trust-policy.json` files no longer crash nono, and closed SEC-08 by writing ADR-112 to explicitly reject upstream's allow_vars reinterpretation that would have silently switched the fork's default from fail-closed to fail-open.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-08-05 (session start, not separately timestamped)
- **Completed:** 2026-08-05T13:52:40-04:00
- **Tasks:** 2
- **Files modified:** 8 (7 code/doc files + 1 new ADR)

## Accomplishments
- `TrustPolicy` gains `predicate: Option<String>` and `TRUST_POLICY_PREDICATE` const; `nono trust`'s loader (`load_nono_policy` in `trust_scan.rs`) now peeks at raw JSON before full deserialization, skipping foreign files (e.g. AWS IAM's identically-named `trust-policy.json`) with a warning suggesting `nono trust init --force` instead of crashing on a schema-parse error.
- `nono trust init` no longer emits a `"version"` key in generated policy files; `TRUST_POLICY_VERSION` is `#[deprecated(since = "0.66.0")]` and dropped from `crate::trust`'s public re-export (kept internally so existing files carrying `"version"` still parse for backward compatibility).
- Wrote `proj/ADR-112-allow-vars-fail-closed-preserved.md`, the D-05 contingent-escalation ADR: upstream `a5a441c2` would make an omitted `allow_vars` key mean "inherit every parent env var" — the opposite of this fork's deliberate fail-closed default (Plan 34-08a / D-20). Decision: PRESERVE, zero source-code change, `empty_allow_vars_fails_closed` remains the durable regression guard.

## Task Commits

Each task was committed atomically:

1. **Task 1: SEC-04 trust-policy predicate discriminator** - `62d9e050` (feat)
2. **Task 2: SEC-08 — ADR-112, preserve the fork's fail-closed allow_vars default** - `9cdef2f5` (docs)

_No TDD-cycle multi-commit tasks in this plan — Task 1 was implemented and verified against the existing `trust::` test suite (189 tests) plus new/renamed tests ported from upstream's diff; Task 2 produced no source-code change by design._

## Files Created/Modified
- `crates/nono/src/trust/types.rs` - `TRUST_POLICY_PREDICATE` const, `predicate: Option<String>` field, deprecated `TRUST_POLICY_VERSION`, `version: Option<u32>`, `has_nono_predicate()`, `validate_version()` rewritten to check predicate (nested if-let, MSRV-safe)
- `crates/nono/src/trust/policy.rs` - `merge_policies` sets `predicate`/`version: None` on the merged result instead of `version.max()`; `merge_rejects_unsupported_version` renamed `merge_ignores_legacy_version_field`
- `crates/nono/src/trust/mod.rs` - re-export swaps `TRUST_POLICY_VERSION` for `TRUST_POLICY_PREDICATE`
- `crates/nono-cli/src/trust_cmd.rs` - `run_init` emits `predicate` instead of `version`; `run_sign_policy` and `load_trust_policy` route through `trust_scan::load_nono_policy`
- `crates/nono-cli/src/trust_scan.rs` - new `pub(crate) fn load_nono_policy()` (peek-before-full-parse); `load_scan_policy` uses it; new `write_test_policy` test helper replaces two hand-written JSON literal test fixtures
- `docs/cli/features/trust.mdx` - 4 `trust-policy.json` examples updated to show `predicate` instead of `version`
- `tests/integration/test_trust_cli.sh` - 2 raw `trust-policy.json` heredocs gain a `predicate` line (kept `version` too, matching upstream, to test backward-compat parsing)
- `proj/ADR-112-allow-vars-fail-closed-preserved.md` (new) - the SEC-08 decision record

## Decisions Made
- Ported SEC-04 as a clean `adopt` per `112-DISPOSITION-TABLE.md`'s finding — all 7 target files present, additive discriminator field, no removal of fork-specific trust types.
- Used nested `if let` instead of upstream's let-chain (`if let ... && ...`) in `validate_version()` to avoid any MSRV/edition ambiguity on this fork's Rust 1.82/edition 2021 baseline; behavior is identical.
- Updated all 4 `trust-policy.json` examples in `docs/cli/features/trust.mdx`, including the fork-only "Trust Policy for GitLab-Signed Files" section that upstream's own diff didn't touch (upstream has no GitLab support) — left stale would have shown an inconsistent mix of `version`/`predicate` across otherwise-parallel examples.
- SEC-08: rejected upstream `a5a441c2` outright (not adapted, not partially adopted) because the underlying semantic — "omitted `allow_vars` means inherit all parent env vars" — is irreconcilable with the fork's deliberate, previously-litigated (D-20, Plan 34-08a) fail-closed default. Recorded in `proj/ADR-112-allow-vars-fail-closed-preserved.md` per CLAUDE.md's "when in doubt, choose the more restrictive option."

## Deviations from Plan

None — plan executed exactly as written. Both tasks matched their `must_haves`/acceptance criteria on the first implementation pass; no auto-fixes (Rules 1-3) or architectural escalations (Rule 4) beyond the plan's own explicit D-05 escalation (which is the plan's designed outcome, not a deviation).

## Issues Encountered

None. One pre-emptive check: verified `TRUST_POLICY_VERSION` had no other internal consumers (`grep -rn TRUST_POLICY_VERSION crates/`) before dropping it from the `crate::trust` re-export, to confirm removing the re-export (matching upstream's own diff exactly) wouldn't break any caller — confirmed zero other consumers in `nono-cli` or the C FFI bindings.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SEC-04 and SEC-08 are both closed. `112-DISPOSITION-TABLE.md`'s tally for this plan's two rows is now fully executed (SEC-04 `adopt` — done; SEC-08 `won't-sync-verbatim/ADR-112-flagged` — done, ADR written).
- `proj/ADR-112-allow-vars-fail-closed-preserved.md` is the durable record any future upstream sync must cite (per its own Consequences section) if it re-encounters `a5a441c2` or a descendant commit touching `allow_vars` semantics — it should re-affirm, not re-litigate.
- No blockers for the remaining Phase 112 plans (SEC-01 finding doc, SEC-03/05/06/07 absorbs, RES-01/RES-02 dispositions, SEC-02a/b/c deferral to Phase 114).

## Verification

- `cargo test -p nono-sandbox --lib -- trust::` → 189 passed, 0 failed (SEC-04)
- `cargo test -p nono-sandbox-cli --bin nono -- trust_scan:: trust_cmd::` → 57 passed, 0 failed
- `cargo test -p nono-sandbox-cli --bin nono -- empty_allow_vars_fails_closed` → 1 passed (SEC-08 regression guard, before and after this plan)
- `git diff -- crates/nono-cli/src/profile_runtime.rs` → empty (0 lines) — confirmed zero source-code change for SEC-08
- `cargo build --workspace --all-targets` → clean
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows-host, informational — not the mandatory cross-target gate; see below) → clean
- `cargo fmt --all -- --check` → clean

**Cross-target gate applicability:** This plan's files (`crates/nono/src/trust/{types,policy,mod}.rs`, `crates/nono-cli/src/{trust_cmd,trust_scan}.rs`) contain no `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, or `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks, and are not under `exec_strategy/` or `bindings/c/src/`. Per `CLAUDE.md`'s cross-target clippy rule, the mandatory `cross clippy`/`cargo-zigbuild clippy` gates do not apply to this plan's changes (`trust_scan.rs` does contain pre-existing, untouched `#[cfg(unix)]`/`#[cfg(windows)]` test-helper blocks for path-safety tests — none of this plan's edits are near or touch those blocks). The Windows-host `cargo clippy --workspace --all-targets` run above is a supplementary sanity check, not a substitute for the mandatory gate — it was run because it is fast and free, not because it satisfies the CLAUDE.md requirement.

## Self-Check: PASSED

All claimed files exist on disk; all claimed commit hashes (`62d9e050`, `9cdef2f5`, `296b0a04`) resolve via `git log --oneline --all`.

---
*Phase: 112-security-residual-sync*
*Completed: 2026-08-05*
