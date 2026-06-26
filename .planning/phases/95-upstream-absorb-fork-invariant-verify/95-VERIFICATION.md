---
phase: 95-upstream-absorb-fork-invariant-verify
verified: 2026-06-26T00:00:00Z
status: gaps_closed
score: 10/10 must-haves verified (gap-closure plans 95-05/06/07 complete)
re_verification:
  previous_status: gaps_found
  previous_score: 6/10 must-haves verified
  gaps_closed:
    - "WR-02: arch-portable msghdr offset derivation restored (offset_of!, compile-time assertions, #[must_use]) — commit 8bca078b"
    - "WR-03: --allow-gpu Linux Landlock enforcement restored (collect_linux_gpu_paths, is_nvidia_compute_device, caps.gpu() branch, 4 GPU tests) — commit 5d1e9077"
    - "WR-01: format!() heap allocations in post-fork child replaced with const static byte strings (MSG_PROXY_WRITE_FAIL, MSG_PROXY_FILTER_FAIL) — commit 2a8b639e"
    - "CR-01: CompiledEndpointPolicy.evaluate() wired into reverse.rs request path (additive, after endpoint_rules check) — commit c81429aa"
  gaps_remaining: []
  regressions: []
cross_target_clippy:
  x86_64-unknown-linux-gnu: "PARTIAL→CI — Rust std installed; aws-lc-sys/ring require x86_64-linux-gnu-gcc (absent); Docker Desktop not running; WSL not installed. GH Actions Linux Clippy on HEAD be42a5af is decisive. Phase 96 resolution target."
  x86_64-apple-darwin: "PARTIAL→CI — Rust std installed; aws-lc-sys/ring require cc (absent for macOS cross); osxcross not installed. GH Actions macOS Clippy on HEAD be42a5af is decisive. Phase 96 resolution target."
gap_closure_head_sha: "be42a5af"
gaps:
  - truth: "Fork-divergent invariants are explicitly preserved — the WR-04 arch-portable msghdr offset derivation (offset_of!/size_of, compile-time assertions, #[must_use]) is intact post-sync"
    status: closed
    closed_by: "commit 8bca078b (fix(95-05): restore WR-02 arch-portable msghdr offset derivation)"
    evidence: "grep 'core::mem::offset_of!(libc::msghdr, msg_name)' crates/nono/src/sandbox/linux.rs → line 2649 MATCH; const MSGHDR_MIN_READ: usize = 12 NO MATCH (gone)"

  - truth: "Fork-divergent invariants are explicitly preserved — --allow-gpu Linux Landlock enforcement (collect_linux_gpu_paths, is_nvidia_compute_device, caps.gpu() branch) is intact post-sync"
    status: closed
    closed_by: "commit 5d1e9077 (feat(95-05): restore WR-03 GPU enforcement)"
    evidence: "grep 'fn collect_linux_gpu_paths' linux.rs → line 480 MATCH; grep 'caps\\.gpu()' linux.rs → lines 478,907 MATCH"

  - truth: "Absorb does not introduce operator-configurable security controls that silently do nothing (endpoint_policy fail-open)"
    status: closed
    closed_by: "commit c81429aa (fix(95-06): wire CompiledEndpointPolicy.evaluate() into reverse.rs request path)"
    evidence: "grep 'endpoint_policy\\.evaluate' crates/nono-proxy/src/reverse.rs → line 126 MATCH"

  - truth: "Post-fork/pre-exec child code uses only async-signal-safe, non-heap-allocating operations (CR-01 fork-safety invariant)"
    status: closed
    closed_by: "commit 2a8b639e (fix(95-06): replace format!() heap alloc in post-fork child with const static byte strings)"
    evidence: "grep 'MSG_PROXY_WRITE_FAIL|MSG_PROXY_FILTER_FAIL' exec_strategy.rs → lines 1415,1457 MATCH (format!() gone)"

deferred: []
---

# Phase 95: Fork-Invariant Verification Report

**Phase Goal:** Absorb the UPST10 upstream sync clusters (Cluster A AF_UNIX deadlock fix, Cluster B tool-sandbox shared-surface, Cluster C credentials_intent) into the fork WITHOUT regressing any fork invariant or the fork's security model. Then produce a fork-invariant verification checklist.
**Verified:** 2026-06-26
**Status:** gaps_closed (updated by Plan 95-07)
**Score:** 10/10 must-haves verified (gap-closure plans 95-05/06/07 closed all 4 gaps)

**Note on prior self-verification (95-04):** The 95-VERIFICATION.md produced by Plan 95-04 was a self-verification that checked exec_strategy_windows/ byte-stability, the ADR-86 audit boundary, and the Cluster C proxy guard. It did NOT inspect the linux.rs hunks changed by the Cluster A cherry-pick conflict resolution. This external verification found two confirmed regressions (WR-02, WR-03) and two additional gaps (CR-01, WR-01) in that uninspected surface. All four gaps were closed by Plans 95-05 and 95-06. Plan 95-07 (this plan) ran the cross-target verification gate (PARTIAL→CI — C linker absent) and confirmed all gaps closed via static grep.

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | Cluster A (9ce74e92 AF_UNIX deadlock fix) is present in git log with -x trailer and DCO | VERIFIED | ae77d198 in log; Cherry-picked from commit 9ce74e92 in body; Signed-off-by: Oscar Mack Jr present |
| 2 | Cluster B shared-surface (11fd10e0) absorbed additively; CR-02 records_verified:event_count > 0 intact | VERIFIED | audit.rs:1570 byte-intact; SandboxRuntimeAuditEvent/CommandPolicyAuditEvent present; tool-sandbox/ absent |
| 3 | Cluster C (9b37dc52) absorbed; Phase 89 fail-secure proxy activation predicate preserved | VERIFIED | proxy_runtime.rs:95 has || !prepared.custom_credentials.is_empty(); proxy_activates_with_custom_credentials_only test at line 503 asserts opts.active |
| 4 | exec_strategy_windows/ is byte-for-byte unchanged (AppContainer/WFP/broker backend unaffected) | VERIFIED | git diff ed6cdde1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ returns 0 lines |
| 5 | ADR-86 boundary intact: DiagnosticFormatter in CLI-side; diagnostic_code()/remediation() in error.rs; bindings/c/src/ untouched | VERIFIED | formatter.rs in nono-cli/src/diagnostic/; error.rs lines 383/452; bindings/c/src/ diff empty |
| 6 | Cluster B restrict_execute/has_execute, CMD_URI_PREFIX, SENSITIVE_ENV_VARS, endpoint_policy field, shutdown_tx all absorbed | VERIFIED | restrict_execute at linux.rs:790, mod.rs:29; CMD_URI_PREFIX at keystore.rs:161; SENSITIVE_ENV_VARS expanded at scrub.rs:47; endpoint_policy at route.rs:41 |
| 7 | WR-04 fork hardening: arch-portable msghdr offset derivation (offset_of!, compile-time assertions, #[must_use]) intact post-sync | VERIFIED (gap-closed 95-05) | Restored by commit 8bca078b. grep 'core::mem::offset_of!(libc::msghdr, msg_name)' linux.rs → line 2649 MATCH. Hardcoded const MSGHDR_MIN_READ: usize = 12 confirmed ABSENT. |
| 8 | --allow-gpu Linux Landlock enforcement (collect_linux_gpu_paths, is_nvidia_compute_device, caps.gpu() branch) intact post-sync | VERIFIED (gap-closed 95-05) | Restored by commit 5d1e9077. grep 'fn collect_linux_gpu_paths' → line 480 MATCH; grep 'caps\.gpu()' → lines 478,907 MATCH. 4 GPU tests added. |
| 9 | No operator-configurable security controls that silently do nothing (endpoint_policy evaluator wired into request path) | VERIFIED (gap-closed 95-06) | Wired by commit c81429aa. grep 'endpoint_policy\.evaluate' reverse.rs → line 126 MATCH. evaluate() called after endpoint_rules.is_allowed() (additive). |
| 10 | Post-fork/pre-exec child uses only async-signal-safe non-heap-allocating operations (CR-01 fork-safety) | VERIFIED (gap-closed 95-06) | Fixed by commit 2a8b639e. grep 'MSG_PROXY_WRITE_FAIL\|MSG_PROXY_FILTER_FAIL' exec_strategy.rs → lines 1415,1457 MATCH. format!() gone. |

**Score:** 10/10 truths verified (gap-closure plans 95-05/06/07 complete)

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|---------|---------|--------|---------|
| `ci-logs-local/baseline-95/baseline-before-cherry-picks.txt` | D-04 baseline with 5 FAILED | VERIFIED | Exists; 95-01-SUMMARY confirms 5 failures captured pre-cherry-pick |
| `crates/nono/src/sandbox/linux.rs` | AF_UNIX fix + arch-portable msghdr offsets + GPU enforcement intact | VERIFIED | AF_UNIX fix landed (ae77d198); WR-02 msghdr restored (8bca078b); WR-03 GPU enforcement restored (5d1e9077) |
| `crates/nono-proxy/src/reverse.rs` | evaluate() wired into request path | VERIFIED | evaluate() called at line 126 (wired by c81429aa) |
| `crates/nono-cli/src/exec_strategy.rs` | Post-fork child uses static byte strings (CR-01) | VERIFIED | MSG_PROXY_WRITE_FAIL / MSG_PROXY_FILTER_FAIL const static byte strings at lines 1415/1457 (fixed by 2a8b639e) |
| `.planning/phases/95-upstream-absorb-fork-invariant-verify/95-VERIFICATION.md` | Fork-invariant checklist covering linux.rs regressions | VERIFIED | Updated by Plan 95-07 — all 4 gaps closed, status gaps_closed, score 10/10 |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `ae77d198` (Cluster A) | git log | -x trailer + DCO | VERIFIED | Cherry-picked from commit 9ce74e92; Oscar Mack Jr DCO present |
| `91d526e6` (Cluster B) | git log | upstream SHA in body + DCO | VERIFIED | "Cherry-picked from upstream nolabs-ai/nono 11fd10e0" in body |
| `62dbf013` (Cluster C) | proxy_runtime.rs | !prepared.custom_credentials.is_empty() preserved | VERIFIED | Line 95 and 118 of proxy_runtime.rs intact |
| `endpoint_policy` (route.rs) | reverse.rs request path | evaluate() call | WIRED (95-06) | reverse.rs:126 calls endpoint_policy.evaluate() after endpoint_rules.is_allowed() — commit c81429aa |
| `read_msghdr_dest` (linux.rs) | offset_of! derivation | compile-time layout proof | WIRED (95-05) | offset_of!(libc::msghdr, msg_name) at line 2649; compile-time assertions restored — commit 8bca078b |
| `caps.gpu()` (capability.rs) | linux.rs apply path | Landlock allowlist branch | WIRED (95-05) | caps.gpu() branch at line 907; collect_linux_gpu_paths at line 480 — commit 5d1e9077 |
| Post-fork error paths (exec_strategy.rs) | static byte string pattern | CR-01 invariant | WIRED (95-06) | Lines 1415/1457 use const MSG_PROXY_WRITE_FAIL/MSG_PROXY_FILTER_FAIL &[u8] — commit 2a8b639e |

---

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---------|--------------|--------|-------------------|--------|
| `reverse.rs` request path | `endpoint_policy` | `CompiledEndpointPolicy::evaluate()` | Yes — called at line 126 | WIRED (95-06, commit c81429aa) |
| `linux.rs:read_msghdr_dest` | `MSGHDR_MIN_READ` | `core::mem::offset_of!` | Yes — offset_of! derivation at line 2649 | WIRED (95-05, commit 8bca078b) |
| `linux.rs` apply path | `caps.gpu()` | `collect_linux_gpu_paths()` | Yes — function at line 480, branch at line 907 | WIRED (95-05, commit 5d1e9077) |

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|---------|--------|
| `crates/nono/src/sandbox/linux.rs` | 2427 | `const MSGHDR_MIN_READ: usize = 12` — hardcoded LP64 offset replacing offset_of! | BLOCKER | Security-critical: AF_UNIX sendmsg destination mediation reads wrong layout on non-LP64 targets; explicit fork hardening (WR-04) silently reverted |
| `crates/nono/src/sandbox/linux.rs` | ~476 (deleted) | `collect_linux_gpu_paths` and `is_nvidia_compute_device` deleted; caps.gpu() branch deleted | BLOCKER | Silent feature regression: --allow-gpu accepted on Linux with zero Landlock enforcement; macOS still enforces |
| `crates/nono-proxy/src/reverse.rs` | 96 | `endpoint_rules.is_allowed()` — endpoint_policy.evaluate() never called despite being operator-configurable | BLOCKER | Fail-open: configured deny rules silently not enforced on request path |
| `crates/nono-cli/src/exec_strategy.rs` | 1411, 1450 | `format!(...)` in post-fork/pre-exec child | WARNING | Heap allocation in child after fork — can deadlock if allocator lock held at fork time; violates established CR-01 pattern |
| `crates/nono/src/audit.rs` | 585, 591 | `#[cfg_attr(not(target_os = "linux"), allow(dead_code))]` on record_sandbox_runtime_event/record_command_policy_event | WARNING | Dead on Linux too; CLAUDE.md prohibits allow(dead_code) for unused code — add callers or remove until the consuming path lands |
| `crates/nono/src/keystore.rs` | 862, 870 | `is_cmd_uri` / `validate_cmd_uri` have zero callers — validator not used where cmd:// refs are accepted | WARNING | load_secret_by_ref uses inline starts_with check, not validate_cmd_uri; validator's behavior unverified |

---

## Requirements Coverage

| Requirement | Phase | Description | Status | Evidence |
|-------------|-------|------------|--------|---------|
| UPST10-02 | Phase 95 | All will-sync clusters absorbed without regressing Windows security model | VERIFIED (PARTIAL→CI) | Clusters A/B/C absorbed; WR-02/WR-03/WR-01/CR-01 regressions closed by Plans 95-05/06; cross-target clippy PARTIAL→CI (C linker absent on Windows host; Rust targets installed; GH Actions decisive on HEAD be42a5af) |
| UPST10-03 | Phase 95 | Fork-divergent invariants explicitly preserved and verified | VERIFIED (PARTIAL→CI) | All 4 fork invariants confirmed present via static grep (Plan 95-07); cross-target clippy PARTIAL→CI per cross-target-verify-checklist.md; Phase 96 resolves the C-linker gap |

---

## Behavioral Spot-Checks

Step 7b: SKIPPED — no runnable entry points verifiable without a Linux environment. The key invariants are verified via static grep/diff analysis. Cross-target clippy is PARTIAL→CI per D-03 and Phase 96 (C linker absent on Windows host).

---

## Probe Execution

Step 7c: No probe-*.sh files declared or discovered for Phase 95.

---

## What the Absorb DID Achieve (Confirmed)

These items are verified PASS and should NOT be re-opened in gap closure:

1. **Cluster A AF_UNIX deadlock fix landed** — ae77d198 is in git log with correct -x trailer and DCO. The BPF filter-install ordering fix, IPC handshake timing, and dup2 bypass closure all landed.
2. **exec_strategy_windows/ byte-unchanged** — `git diff ed6cdde1..HEAD -- crates/nono-cli/src/exec_strategy_windows/` returns empty. Windows AppContainer/WFP/broker unaffected.
3. **ADR-86 audit boundary intact** — records_verified: event_count > 0 at audit.rs:1570. DiagnosticFormatter in CLI-side diagnostic/. bindings/c/src/ untouched.
4. **CR-02 invariant intact** — verify_empty_log_with_no_stored_metadata_is_not_valid test passing per self-verification.
5. **Cluster B additive surface absorbed correctly** — SandboxRuntimeAuditEvent, CommandPolicyAuditEvent, restrict_execute, CMD_URI_PREFIX, SENSITIVE_ENV_VARS expansion, endpoint_policy field all present. Tool-sandbox dir and tls_intercept/ absent.
6. **Cluster C Phase 89 invariant preserved** — proxy_activates_with_custom_credentials_only test at proxy_runtime.rs:503; || !prepared.custom_credentials.is_empty() at lines 95/118.
7. **DIVERGENCE-LEDGER closed** — Clusters A/B/C marked absorbed per 95-04 bookkeeping.

---

## Gaps Summary (CLOSED — updated by Plan 95-07)

All four gaps that blocked the phase goal "absorb WITHOUT regressing any fork invariant" are now closed:

- **WR-02 CLOSED** — commit 8bca078b (Plan 95-05)
- **WR-03 CLOSED** — commit 5d1e9077 (Plan 95-05)
- **CR-01 CLOSED** — commit c81429aa (Plan 95-06)
- **WR-01 CLOSED** — commit 2a8b639e (Plan 95-06)

Original gap descriptions preserved below for historical context:

Four gaps blocked the phase goal "absorb WITHOUT regressing any fork invariant":

**WR-02 (BLOCKER):** The Cluster A cherry-pick conflict resolution silently reverted the WR-04 fork hardening in `read_msghdr_dest` (linux.rs:2427). The arch-portable `offset_of!` derivation was replaced with hardcoded `12`, compile-time assertions deleted, and `#[must_use]` stripped from both `read_msghdr_dest` and `read_mmsghdr_dests`. This is a security-critical path (AF_UNIX sendmsg destination mediation). The self-verification (95-04) only checked exec_strategy_windows/ and the ADR-86 boundary — it never ran `git diff ed6cdde1..HEAD -- crates/nono/src/sandbox/linux.rs` and compared the msghdr handling.

**WR-03 (BLOCKER):** The same Cluster A cherry-pick deleted the entire Linux GPU-path enforcement (`collect_linux_gpu_paths`, `is_nvidia_compute_device`, `caps.gpu()` Landlock branch). The `--allow-gpu` flag, `gpu()` capability, and macOS enforcement all remain — creating a silent fail-open where `nono run --allow-gpu` on Linux accepts the flag but applies zero Landlock rules for GPU devices.

**CR-01 (BLOCKER):** The Cluster B absorb added `CompiledEndpointPolicy` to `RouteConfig` as a live operator-configurable field, but `evaluate()` is never called on the request path. `reverse.rs:96` still uses the legacy `endpoint_rules.is_allowed()`. An operator who configures `endpoint_policy.deny` rules gets them silently ignored — a fail-open in a security proxy.

**WR-01 (WARNING):** Two new error paths in the post-fork/pre-exec child (exec_strategy.rs:1411, :1450) use `format!()` for heap allocation, contradicting the explicit CR-01 static-byte-string pattern established in adjacent code. This is an error-path-only issue but can deadlock if the allocator lock was held at fork time.

Root cause common to WR-02 and WR-03: the conflict resolution for the Cluster A cherry-pick in linux.rs accepted the upstream version of the conflicting sections, which did not have the fork's WR-04 hardening or GPU enforcement (those were fork-only additions absent in upstream). The executor resolved conflicts by "accepting upstream" without checking whether upstream's version was MISSING fork-only invariants.

Gap closure scope: WR-02 and WR-03 require restoring fork-specific linux.rs code that was lost in the conflict resolution. CR-01 requires either wiring endpoint_policy into the request path or adding a load-time guard that rejects configs using it. WR-01 requires replacing two format!() calls with static byte strings.

---

## Human Verification Required

None — all gap closures are statically verifiable via grep/diff (confirmed by Plan 95-07). Cross-target clippy is PARTIAL→CI per D-03 (C linker absent on Windows host; Phase 96 resolves). GH Actions Linux/macOS Clippy lanes on HEAD be42a5af are the decisive cross-target signal.

---

_Verified: 2026-06-26_
_Verifier: Claude (gsd-verifier) — external verification, adversarial stance_
