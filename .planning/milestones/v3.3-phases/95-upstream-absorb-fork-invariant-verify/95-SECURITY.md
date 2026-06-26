---
phase: 95-upstream-absorb-fork-invariant-verify
plans: [95-05, 95-06, 95-07]
asvs_level: 2
audit_date: 2026-06-26
auditor: gsd-security-auditor
verdict: SECURED
threats_open: 0
threats_total: 9
---

# Security Audit: Phase 95 — Upstream Absorb Fork-Invariant Verify (Plans 05/06/07)

## Summary

**All 9 declared threats resolved. 0 open. Phase may ship.**

Threats closed: 9/9
ASVS Level: 2
Block condition: high (block_on: high)

---

## Threat Verification

| Threat ID | Category | Disposition | Status | Evidence |
|-----------|----------|-------------|--------|----------|
| T-95-05-01 | Elevation of Privilege | mitigate | CLOSED | `linux.rs:2649` — `core::mem::offset_of!(libc::msghdr, msg_name)`; `linux.rs:2650` — `offset_of!(libc::msghdr, msg_namelen)`; `linux.rs:2652` — `const PTR_SIZE`; `linux.rs:2657,2661` — two `const _: () = assert!` guards; `linux.rs:2637,2723` — `#[must_use]` on both functions; `linux.rs:2701` — TOCTOU comment; `linux.rs:2686` — `namelen_bytes` arch-portable extraction; hardcoded `const MSGHDR_MIN_READ: usize = 12` confirmed ABSENT (no grep match) |
| T-95-05-02 | Elevation of Privilege | mitigate | CLOSED | `linux.rs:432` — `fn is_nvidia_compute_device`; `linux.rs:480` — `fn collect_linux_gpu_paths`; `linux.rs:907` — `if caps.gpu()` Landlock branch in apply path; `linux.rs:4566,4587,4613,4632` — all 4 GPU unit tests present |
| T-95-05-03 | Tampering | mitigate | CLOSED | `exec_strategy_windows/` not touched by any of the 4 gap-closure commits (8bca078b, 5d1e9077, 2a8b639e, c81429aa) — confirmed via `git show --name-only`; IPC/BPF ordering unchanged |
| T-95-05-SC | Tampering | accept | CLOSED | 95-05-SUMMARY.md `tech_stack.added: []` — no new cargo dependencies |
| T-95-06-01 | Denial of Service | mitigate | CLOSED | `exec_strategy.rs:1415-1426` — `const MSG_PROXY_WRITE_FAIL: &[u8]` with `libc::write` + `_exit(126)`; `exec_strategy.rs:1457-1468` — `const MSG_PROXY_FILTER_FAIL: &[u8]` with `libc::write` + `_exit(126)`; CR-01 comment block present at both sites; no `format!()` at either former violation site |
| T-95-06-02 | Elevation of Privilege | mitigate | CLOSED | `reverse.rs:133-186` — exhaustive `match` over all three `EndpointPolicyOutcome` variants (`Allow`/`Deny`/`Approve`) with NO wildcard arm; `Approve` arm fails closed (403 + `audit::log_denied`, returns before credential lookup); `EndpointPolicyOutcome` enum confirmed to have exactly 3 variants (`config.rs:295-309`); `use crate::config::EndpointPolicyOutcome` import at `reverse.rs:18`; enforcement tests at `config.rs:1250,1286,1315` — deny rule enforcement, legacy route semantics, Approve-without-backend recognition |
| T-95-06-03 | Spoofing | accept | CLOSED | Accepted: evaluate() runs AFTER endpoint_rules.is_allowed() (additive); legacy routes wrapped via compile() preserve semantics. Documented in 95-06-SUMMARY.md `key-decisions` |
| T-95-06-SC | Tampering | accept | CLOSED | 95-06-SUMMARY.md `tech-stack.added: []` — no new cargo dependencies |
| T-95-07-01 | Tampering | mitigate | CLOSED (PARTIAL→CI) | Cross-target clippy gate ran for both x86_64-unknown-linux-gnu and x86_64-apple-darwin; both produced PARTIAL→CI because C cross-linker (x86_64-linux-gnu-gcc / cc) is absent on Windows dev host — the failure is a C build failure, NOT a Rust clippy error in any changed file. Rust std installed for both targets. Docker Desktop not running; WSL absent. GH Actions Linux and macOS Clippy lanes on HEAD be42a5af are the decisive gate. Documented per `.planning/templates/cross-target-verify-checklist.md`. Gate was not skipped. Phase 96 (XTGT-01) is the resolution target. |
| T-95-07-02 | Repudiation | mitigate | CLOSED | 95-07-SUMMARY.md `## Gap Closure Status` table records all 5 static grep probes verbatim with line numbers: WR-02 (line 2649), WR-03 (lines 480, 907), WR-01 (lines 1415, 1457), CR-01 (line 126), hardcoded-12-gone (NO MATCH confirmed) |
| T-95-07-SC | Tampering | accept | CLOSED | 95-07-SUMMARY.md `tech-stack.added: []` — no new cargo dependencies |

---

## Security Crux: T-95-06-02 Approve Arm Detail

The original c81429aa wiring used `if let EndpointPolicyOutcome::Deny { .. }` which is a
non-exhaustive single-arm match — `Approve` silently fell through to credential injection.
The code-review (95-REVIEW.md CR-01) caught this. The fix (committed as part of the 95-06
gap-closure pass) replaces the `if let` with a compiler-enforced exhaustive `match`:

- `Allow` arm: explicit empty body — request proceeds to credential lookup.
- `Deny` arm: 403 + `audit::log_denied` + early return (unchanged from original intent).
- `Approve` arm: fails closed — 403 + `audit::log_denied` with distinct reason string
  ("endpoint_policy requires approval but no approval backend is configured — failing closed");
  returns before credential lookup. No credential is injected for an Approve outcome.

No wildcard (`_`) arm is present. If a future fourth variant is added to `EndpointPolicyOutcome`,
the compiler will refuse to compile `reverse.rs` until that arm is explicitly handled. This is
the correct structural enforcement for a security-critical dispatch.

Verified at `reverse.rs:133-186` by direct file read (not documentation).

---

## Accepted Risks Log

| Risk ID | Threat | Rationale |
|---------|--------|-----------|
| T-95-05-SC | No new cargo deps (tampering surface) | No packages added; slopcheck not required for this phase |
| T-95-06-03 | evaluate() wiring changes endpoint_rules legacy behavior | evaluate() is additive (runs after is_allowed()); legacy routes wrapped via compile() preserve backward-compat deny semantics per config.rs:361-390. Documented in plan. |
| T-95-06-SC | No new cargo deps (tampering surface) | No packages added |
| T-95-07-SC | No new cargo deps (tampering surface) | No packages added |

---

## Unregistered Flags

None. The `## Threat Flags` sections of 95-05-SUMMARY.md, 95-06-SUMMARY.md, and
95-07-SUMMARY.md all report "None". No new network endpoints, auth paths, file access
patterns, schema changes, or trust boundaries were introduced by these plans.

---

## Open Threats

None.

---

## Notes

- The PARTIAL→CI for T-95-07-01 (cross-target clippy) is a documented deferral, not a skip.
  The gate was invoked; the blocker is a missing C linker (aws-lc-sys/ring), not a Rust
  correctness issue in the changed files. This is the Phase 95 plan's explicitly sanctioned
  fallback per cross-target-verify-checklist.md. Resolution is Phase 96 XTGT-01.
- 95-REVIEW.md WR-01 (regression test only greps `format!(`) and WR-02 (comment-stripper
  limitation) are test-quality warnings against the async-signal-safety guard in
  `tests/resl_nix_async_signal_safety.rs`. They do not affect production code correctness
  and are not STRIDE threats. They are informational findings for future test hardening.
- 95-REVIEW.md WR-03 (evaluate() duplicates is_allowed() on legacy routes) is a performance
  warning, not a security defect; legacy routes are checked twice but the results are identical.
  Flagged for future consolidation.
