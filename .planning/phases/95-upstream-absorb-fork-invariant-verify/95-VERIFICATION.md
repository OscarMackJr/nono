---
phase: 95-upstream-absorb-fork-invariant-verify
verified: 2026-06-26T00:00:00Z
status: gaps_found
score: 6/10 must-haves verified
re_verification:
  previous_status: passed  # self-verification produced SC3 PASS — this is the first external verification
  previous_score: 3/3 invariants (self-reported)
  gaps_closed: []
  gaps_remaining:
    - "WR-02: arch-portable msghdr offset derivation reverted to hardcoded LP64 layout"
    - "WR-03: --allow-gpu Linux Landlock enforcement deleted (silent fail-open regression)"
    - "CR-01: CompiledEndpointPolicy.evaluate() is never called (operator deny rules silently ignored)"
    - "WR-01: format!() heap allocation in post-fork/pre-exec child reverts CR-01 fork-safety invariant"
  regressions:
    - "WR-02 and WR-03 are both confirmed regressions introduced by the Cluster A cherry-pick conflict resolution that the self-verification (95-04) did not examine"
gaps:
  - truth: "Fork-divergent invariants are explicitly preserved — the WR-04 arch-portable msghdr offset derivation (offset_of!/size_of, compile-time assertions, #[must_use]) is intact post-sync"
    status: failed
    reason: "cherry-pick ae77d198 (Cluster A) reverted the WR-04 fork hardening in read_msghdr_dest. Current linux.rs:2427 has const MSGHDR_MIN_READ: usize = 12 (hardcoded LP64 layout). The phase-base (ed6cdde1) had core::mem::offset_of!(libc::msghdr, msg_name/msg_namelen), compile-time overlap assertions, and #[must_use] on both functions. All three elements are now absent."
    artifacts:
      - path: "crates/nono/src/sandbox/linux.rs"
        issue: "Line 2427: const MSGHDR_MIN_READ: usize = 12 (hardcoded). Missing: offset_of!/size_of derivation, compile-time assertions, #[must_use] on read_msghdr_dest (line 2419) and read_mmsghdr_dests (line 2470), TOCTOU comment at msg_name==0 branch"
    missing:
      - "Restore const MSG_NAME_OFFSET/MSG_NAME_LEN_OFFSET/PTR_SIZE via core::mem::offset_of! and size_of"
      - "Restore const _: () = assert!() compile-time layout assertions"
      - "Re-apply #[must_use = \"...\"] to read_msghdr_dest and read_mmsghdr_dests"
      - "Restore the WR-01 TOCTOU comment at the msg_name == 0 branch"

  - truth: "Fork-divergent invariants are explicitly preserved — --allow-gpu Linux Landlock enforcement (collect_linux_gpu_paths, is_nvidia_compute_device, caps.gpu() branch) is intact post-sync"
    status: failed
    reason: "cherry-pick ae77d198 + fix 61689ef8 deleted the entire Linux GPU-path enforcement. collect_linux_gpu_paths, is_nvidia_compute_device, and the caps.gpu() Landlock allowlist branch around former line 915 are all absent from linux.rs. The capability.rs gpu()/allow_gpu() API, the --allow-gpu CLI flag, and macOS enforcement all still exist. Net effect: on Linux, nono run --allow-gpu silently grants no GPU device paths."
    artifacts:
      - path: "crates/nono/src/sandbox/linux.rs"
        issue: "collect_linux_gpu_paths and is_nvidia_compute_device functions deleted, caps.gpu() branch deleted. Zero GPU references in file. Phase-base had these at lines 417-476, 915-916."
      - path: "crates/nono/src/capability.rs"
        issue: "gpu()/allow_gpu() capability still present (line 1181/1451) — API contract now silent fail-open on Linux"
    missing:
      - "Re-apply collect_linux_gpu_paths() and is_nvidia_compute_device() helper functions"
      - "Re-apply the caps.gpu() conditional Landlock allowlist branch in the apply path"
      - "Alternatively: if GPU support is deliberately dropped, remove the --allow-gpu flag, gpu() capability, and macOS enforcement too, and record in DIVERGENCE-LEDGER"

  - truth: "Absorb does not introduce operator-configurable security controls that silently do nothing (endpoint_policy fail-open)"
    status: failed
    reason: "CompiledEndpointPolicy.evaluate() has zero callers anywhere in the workspace. The reverse.rs request path (line 96) still calls route.endpoint_rules.is_allowed(). An operator who configures endpoint_policy deny rules gets silent non-enforcement. Because endpoint_policy is exposed as a live serde-deserializable field in RouteConfig with deny_unknown_fields, this is a real operator-facing fail-open."
    artifacts:
      - path: "crates/nono-proxy/src/reverse.rs"
        issue: "Line 96 calls route.endpoint_rules.is_allowed() -- endpoint_policy.evaluate() never invoked on request path"
      - path: "crates/nono-proxy/src/config.rs"
        issue: "evaluate() defined at line 419, allows_all_without_l7() at line 404, zero callers in workspace"
    missing:
      - "Either wire endpoint_policy.evaluate() into reverse.rs replacing the endpoint_rules.is_allowed check, OR reject configs that set endpoint_policy with ProxyError::Config at load time so the knob cannot be silently relied upon"

  - truth: "Post-fork/pre-exec child code uses only async-signal-safe, non-heap-allocating operations (CR-01 fork-safety invariant)"
    status: failed
    reason: "exec_strategy.rs lines 1411-1414 and 1450-1451 use format!() (heap allocation) in the post-fork/pre-exec child. Adjacent code at lines 1064-1139 has explicit CR-01 comments forbidding format!() and uses static byte strings. The new error paths for proxy notify fd handling use format!() then convert to bytes, violating the established pattern."
    artifacts:
      - path: "crates/nono-cli/src/exec_strategy.rs"
        issue: "Lines 1411-1414: let detail = format!(\"nono: failed to write proxy seccomp notify fd number: {}\\n\", std::io::Error::last_os_error()); Lines 1450-1451: let detail = format!(\"nono: seccomp proxy filter not available: {}\\n\", e);"
    missing:
      - "Replace both format!() allocations with const &[u8] static byte strings, matching the adjacent CR-01 pattern"
      - "Use libc::write directly with the static slice as the adjacent code does"
deferred: []
---

# Phase 95: Fork-Invariant Verification Report

**Phase Goal:** Absorb the UPST10 upstream sync clusters (Cluster A AF_UNIX deadlock fix, Cluster B tool-sandbox shared-surface, Cluster C credentials_intent) into the fork WITHOUT regressing any fork invariant or the fork's security model. Then produce a fork-invariant verification checklist.
**Verified:** 2026-06-26
**Status:** gaps_found
**Score:** 6/10 must-haves verified

**Note on prior self-verification (95-04):** The 95-VERIFICATION.md produced by Plan 95-04 was a self-verification that checked exec_strategy_windows/ byte-stability, the ADR-86 audit boundary, and the Cluster C proxy guard. It did NOT inspect the linux.rs hunks changed by the Cluster A cherry-pick conflict resolution. This external verification found two confirmed regressions (WR-02, WR-03) and two additional gaps (CR-01, WR-01) in that uninspected surface.

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
| 7 | WR-04 fork hardening: arch-portable msghdr offset derivation (offset_of!, compile-time assertions, #[must_use]) intact post-sync | FAILED | linux.rs:2427 has const MSGHDR_MIN_READ: usize = 12 (hardcoded LP64). Phase-base ed6cdde1 had offset_of!(libc::msghdr, msg_name/msg_namelen), compile-time assertions, and #[must_use] on both functions. All reverted by Cluster A cherry-pick. |
| 8 | --allow-gpu Linux Landlock enforcement (collect_linux_gpu_paths, is_nvidia_compute_device, caps.gpu() branch) intact post-sync | FAILED | Zero GPU references in linux.rs. Functions deleted by Cluster A cherry-pick. capability.rs gpu()/allow_gpu() and --allow-gpu CLI flag still exist — silent fail-open on Linux. |
| 9 | No operator-configurable security controls that silently do nothing (endpoint_policy evaluator wired into request path) | FAILED | CompiledEndpointPolicy.evaluate() has zero callers in workspace. reverse.rs:96 still calls endpoint_rules.is_allowed(). Endpoint_policy deny rules silently ignored. |
| 10 | Post-fork/pre-exec child uses only async-signal-safe non-heap-allocating operations (CR-01 fork-safety) | FAILED | exec_strategy.rs:1411-1414 and :1450-1451 use format!() for heap-allocated error messages in post-fork child, reverting the established static-byte-string CR-01 pattern |

**Score:** 6/10 truths verified

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|---------|---------|--------|---------|
| `ci-logs-local/baseline-95/baseline-before-cherry-picks.txt` | D-04 baseline with 5 FAILED | VERIFIED | Exists; 95-01-SUMMARY confirms 5 failures captured pre-cherry-pick |
| `crates/nono/src/sandbox/linux.rs` | AF_UNIX fix + arch-portable msghdr offsets + GPU enforcement intact | PARTIAL/STUB | AF_UNIX fix landed (ae77d198); msghdr hardcoded (WR-02); GPU deleted (WR-03) |
| `crates/nono-proxy/src/config.rs` | evaluate() wired into request path | STUB | evaluate() defined but never called — zero callers workspace-wide |
| `crates/nono-cli/src/exec_strategy.rs` | Post-fork child uses static byte strings (CR-01) | STUB | Lines 1411/1450 use format!() heap allocation |
| `.planning/phases/95-upstream-absorb-fork-invariant-verify/95-VERIFICATION.md` | Fork-invariant checklist covering linux.rs regressions | MISSING | Prior self-verification did not examine linux.rs hunks; missed WR-02 and WR-03 |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `ae77d198` (Cluster A) | git log | -x trailer + DCO | VERIFIED | Cherry-picked from commit 9ce74e92; Oscar Mack Jr DCO present |
| `91d526e6` (Cluster B) | git log | upstream SHA in body + DCO | VERIFIED | "Cherry-picked from upstream nolabs-ai/nono 11fd10e0" in body |
| `62dbf013` (Cluster C) | proxy_runtime.rs | !prepared.custom_credentials.is_empty() preserved | VERIFIED | Line 95 and 118 of proxy_runtime.rs intact |
| `endpoint_policy` (route.rs) | reverse.rs request path | evaluate() call | NOT_WIRED | reverse.rs:96 calls endpoint_rules.is_allowed() — evaluate() never invoked |
| `read_msghdr_dest` (linux.rs) | offset_of! derivation | compile-time layout proof | NOT_WIRED | Hardcoded const 12 replaces offset_of!; no compile-time assertions |
| `caps.gpu()` (capability.rs) | linux.rs apply path | Landlock allowlist branch | NOT_WIRED | GPU branch deleted; --allow-gpu flag still accepted silently |
| Post-fork error paths (exec_strategy.rs) | static byte string pattern | CR-01 invariant | NOT_WIRED | Lines 1411, 1450 use format!() contrary to established pattern |

---

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---------|--------------|--------|-------------------|--------|
| `reverse.rs` request path | `endpoint_policy` | `CompiledEndpointPolicy::evaluate()` | No — never called | DISCONNECTED |
| `linux.rs:read_msghdr_dest` | `MSGHDR_MIN_READ` | `core::mem::offset_of!` | No — hardcoded 12 | STATIC (fork-hardening reverted) |
| `linux.rs` apply path | `caps.gpu()` | `collect_linux_gpu_paths()` | No — function deleted | DISCONNECTED |

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
| UPST10-02 | Phase 95 | All will-sync clusters absorbed without regressing Windows security model | PARTIAL | Clusters A/B/C absorbed (commits present, DCO signed); but Cluster A cherry-pick introduced WR-02 (msghdr hardcoded) and WR-03 (GPU deleted) — fork-invariant regressions introduced, not just Windows model |
| UPST10-03 | Phase 95 | Fork-divergent invariants explicitly preserved and verified | FAILED | Self-verification (95-04) produced PASS but did not examine linux.rs hunks changed by Cluster A. WR-02 and WR-03 are confirmed violations of fork invariants that this requirement is chartered to preserve |

---

## Behavioral Spot-Checks

Step 7b: SKIPPED — no runnable entry points verifiable without a Linux environment. The key invariants are verified via static grep/diff analysis. Cross-target clippy is PARTIAL->CI per D-03 and Phase 96.

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

## Gaps Summary

Four gaps block the phase goal "absorb WITHOUT regressing any fork invariant":

**WR-02 (BLOCKER):** The Cluster A cherry-pick conflict resolution silently reverted the WR-04 fork hardening in `read_msghdr_dest` (linux.rs:2427). The arch-portable `offset_of!` derivation was replaced with hardcoded `12`, compile-time assertions deleted, and `#[must_use]` stripped from both `read_msghdr_dest` and `read_mmsghdr_dests`. This is a security-critical path (AF_UNIX sendmsg destination mediation). The self-verification (95-04) only checked exec_strategy_windows/ and the ADR-86 boundary — it never ran `git diff ed6cdde1..HEAD -- crates/nono/src/sandbox/linux.rs` and compared the msghdr handling.

**WR-03 (BLOCKER):** The same Cluster A cherry-pick deleted the entire Linux GPU-path enforcement (`collect_linux_gpu_paths`, `is_nvidia_compute_device`, `caps.gpu()` Landlock branch). The `--allow-gpu` flag, `gpu()` capability, and macOS enforcement all remain — creating a silent fail-open where `nono run --allow-gpu` on Linux accepts the flag but applies zero Landlock rules for GPU devices.

**CR-01 (BLOCKER):** The Cluster B absorb added `CompiledEndpointPolicy` to `RouteConfig` as a live operator-configurable field, but `evaluate()` is never called on the request path. `reverse.rs:96` still uses the legacy `endpoint_rules.is_allowed()`. An operator who configures `endpoint_policy.deny` rules gets them silently ignored — a fail-open in a security proxy.

**WR-01 (WARNING):** Two new error paths in the post-fork/pre-exec child (exec_strategy.rs:1411, :1450) use `format!()` for heap allocation, contradicting the explicit CR-01 static-byte-string pattern established in adjacent code. This is an error-path-only issue but can deadlock if the allocator lock was held at fork time.

Root cause common to WR-02 and WR-03: the conflict resolution for the Cluster A cherry-pick in linux.rs accepted the upstream version of the conflicting sections, which did not have the fork's WR-04 hardening or GPU enforcement (those were fork-only additions absent in upstream). The executor resolved conflicts by "accepting upstream" without checking whether upstream's version was MISSING fork-only invariants.

Gap closure scope: WR-02 and WR-03 require restoring fork-specific linux.rs code that was lost in the conflict resolution. CR-01 requires either wiring endpoint_policy into the request path or adding a load-time guard that rejects configs using it. WR-01 requires replacing two format!() calls with static byte strings.

---

## Human Verification Required

None — all findings are statically verifiable via grep/diff. Cross-target clippy remains PARTIAL->CI per D-03 (deferred to Phase 96 by design).

---

_Verified: 2026-06-26_
_Verifier: Claude (gsd-verifier) — external verification, adversarial stance_
