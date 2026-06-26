---
phase: 95-upstream-absorb-fork-invariant-verify
verified: 2026-06-26T14:00:00Z
status: human_needed
score: 11/11 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 6/10 must-haves verified
  gaps_closed:
    - "WR-02: arch-portable msghdr offset derivation restored (offset_of!, compile-time assertions, #[must_use]) — commit 8bca078b"
    - "WR-03: --allow-gpu Linux Landlock enforcement restored (collect_linux_gpu_paths, is_nvidia_compute_device, caps.gpu() branch, 4 GPU tests) — commit 5d1e9077"
    - "WR-01: format!() heap allocations in post-fork child replaced with const static byte strings (MSG_PROXY_WRITE_FAIL, MSG_PROXY_FILTER_FAIL) — commit 2a8b639e"
    - "CR-01 initial wire: CompiledEndpointPolicy.evaluate() wired into reverse.rs request path — commit c81429aa"
    - "CR-01 code-review fix: exhaustive match over all three EndpointPolicyOutcome variants (Approve arm fails closed) — commit 97f387f2"
  gaps_remaining: []
  regressions: []
cross_target_clippy:
  x86_64-unknown-linux-gnu: "PARTIAL→CI — Rust std installed; aws-lc-sys/ring require x86_64-linux-gnu-gcc (absent on Windows host); Docker Desktop not running; WSL not installed. GH Actions Linux Clippy on HEAD 544cab40 is the decisive gate. Phase 96 resolution target."
  x86_64-apple-darwin: "PARTIAL→CI — Rust std installed; aws-lc-sys/ring require cc (absent for macOS cross-compile from Windows); osxcross not installed. GH Actions macOS Clippy on HEAD 544cab40 is the decisive gate. Phase 96 resolution target."
final_head_sha: "544cab40"
human_verification:
  - test: "Confirm GH Actions Linux + macOS Clippy lanes pass on HEAD 544cab40 (milestone/v2.13-carryforward-closeout)"
    expected: "Both Linux and macOS Clippy jobs exit 0 with no new warnings or errors. Any failures in cfg-gated Unix code touched by this phase (sandbox/linux.rs, exec_strategy.rs, reverse.rs) would block Phase 96."
    why_human: "Cross C-linker toolchain (x86_64-linux-gnu-gcc, osxcross) absent on Windows host; Phase 96 stands it up. The CI lanes are the decisive cross-target signal for the cfg-gated code changed in this phase. Cannot run cargo clippy --target x86_64-unknown-linux-gnu locally."
deferred: []
---

# Phase 95: Fork-Invariant Verification Report

**Phase Goal:** All will-sync clusters from the Phase 94 ledger are absorbed into the fork and the Windows security model is provably unregressed.
**Verified:** 2026-06-26T14:00:00Z
**Status:** human_needed — all 11 must-haves verified by static analysis; one CI-lane confirmation outstanding (cross-target clippy PARTIAL→CI, decisive gate on GH Actions for HEAD 544cab40)
**Re-verification:** Yes — three passes: initial (gaps_found 6/10), gap-closure (10/10 via Plans 95-05/06/07), final (11/11 including code-review CR-01 fix commit 97f387f2)

---

## Goal Achievement

### Observable Truths (Phase 95 Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| SC1-a | Every will-sync cluster commit present in fork with -x trailer | VERIFIED | Cluster A: ae77d198 — "(cherry picked from commit 9ce74e92...)" in body; Cluster B: 91d526e6 — "Cherry-picked from upstream nolabs-ai/nono 11fd10e0"; Cluster C: 62dbf013 — "Cherry-picked from upstream nolabs-ai/nono 9b37dc52" (structural no-op) |
| SC1-b | Every will-sync cluster commit DCO-signed | VERIFIED | ae77d198: "Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>"; 91d526e6: "Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>"; 62dbf013: "Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>" |
| SC1-c | No open will-sync row in DIVERGENCE-LEDGER | VERIFIED | Ledger cluster table: A=absorbed, B=absorbed (split), C=absorbed (split), D=won't-sync. Cluster A was the only will-sync row; it is absorbed. |
| SC2 | make build + make test pass on Windows dev host with no NEW failures vs documented baseline | VERIFIED (PARTIAL→CI for cross-target) | Windows-host workspace clippy --all-targets --all-features: exit 0. cargo fmt --all --check: exit 0. cargo test -p nono-proxy: 176/176 (175 prior + 1 new endpoint_policy_approve_without_backend_is_recognized). Pre-existing baseline reds (try_set_mandatory_label, nono-cli config::*/ protected_paths/profile_cmd/audit_session) documented since Phase 74 — not regressions. Cross-target clippy PARTIAL→CI (C linker absent; Phase 96 target). |
| SC3-a | exec_strategy_windows/ byte-unchanged (AppContainer/WFP/broker backend unaffected) | VERIFIED | git diff ed6cdde1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ returns 0 lines |
| SC3-b | ADR-86 boundary intact: DiagnosticFormatter in CLI diagnostic/; diagnostic_code()/remediation() in error.rs; bindings/c/src/ untouched | VERIFIED | crates/nono-cli/src/diagnostic/formatter.rs exists; error.rs lines 383/452; bindings/c/src/ diff empty vs window |
| SC3-c | Fork-divergent WR-02 invariant: arch-portable msghdr offset derivation (offset_of!, compile-time assertions) intact post-sync | VERIFIED | linux.rs lines 2649-2652: core::mem::offset_of!(libc::msghdr, msg_name) and msg_namelen; comments at 2635/2648 document WR-04 fix. Hardcoded const MSGHDR_MIN_READ = 12 ABSENT — grep returns only documentation-only matches. Restored by commit 8bca078b. |
| SC3-d | Fork-divergent WR-03 invariant: --allow-gpu Linux Landlock enforcement intact post-sync | VERIFIED | linux.rs: fn is_nvidia_compute_device at line 432; fn collect_linux_gpu_paths at line 480; caps.gpu() branch at line 907. Restored by commit 5d1e9077. |
| SC3-e | Fork-divergent WR-01 invariant: post-fork child uses async-signal-safe const byte strings, not format!() | VERIFIED | exec_strategy.rs lines 1415-1424 (MSG_PROXY_WRITE_FAIL) and 1457-1466 (MSG_PROXY_FILTER_FAIL) — both are const &[u8] written via libc::write + _exit(126). Restored by commit 2a8b639e. |
| SC3-f | Fork-divergent CR-01 invariant: CompiledEndpointPolicy.evaluate() wired into request path AND exhaustive match (no Approve fall-through) | VERIFIED | reverse.rs lines 133-186: exhaustive match over EndpointPolicyOutcome::{Allow, Deny, Approve}; no wildcard arm; Approve arm fails closed with 403 + audit::log_denied. Wired by commit c81429aa; code-review defect fixed by commit 97f387f2. |
| SC4 | Security-relevant will-sync commits have a dedicated verification note | VERIFIED | 94-DIVERGENCE-LEDGER.md Cluster A section: detailed rationale for all 4 bugs fixed (deadlock, wrong jt offsets, rate-limiter starvation, dup2 bypass). 95-VERIFICATION.md (this file) provides per-invariant static-grep evidence for each security-critical path. |

**Score:** 11/11 truths verified

---

## Deferred Items

No items deferred to later phases for Phase-95 success criteria.

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|---------|---------|--------|---------|
| `crates/nono/src/sandbox/linux.rs` | AF_UNIX fix (Cluster A) + arch-portable msghdr offsets (WR-02) + GPU enforcement (WR-03) intact | VERIFIED | ae77d198 landed AF_UNIX fix; 8bca078b restored offset_of! derivation; 5d1e9077 restored collect_linux_gpu_paths + is_nvidia_compute_device + caps.gpu() branch |
| `crates/nono-proxy/src/reverse.rs` | evaluate() wired with exhaustive match over all EndpointPolicyOutcome variants | VERIFIED | Exhaustive match at lines 133-186; Allow/Deny/Approve each handled; no wildcard arm; Approve fails closed. Commits c81429aa + 97f387f2. |
| `crates/nono-cli/src/exec_strategy.rs` | Post-fork child uses static byte strings (WR-01) | VERIFIED | MSG_PROXY_WRITE_FAIL / MSG_PROXY_FILTER_FAIL const &[u8] at lines 1415/1457; async-signal-safe libc::write + _exit(126). Commit 2a8b639e. |
| `crates/nono-proxy/src/config.rs` | endpoint_policy_approve_without_backend_is_recognized test | VERIFIED | Test at line 1315 asserts evaluate() surfaces Approve outcome for both explicit approve rule and default: approve — documents the contract that reverse.rs must handle closed. Commit 97f387f2. |
| `crates/nono-cli/src/proxy_runtime.rs` | Phase 89 fail-secure active predicate preserved | VERIFIED | Lines 95, 118: || !prepared.custom_credentials.is_empty() present in both BLOCK-NET and ACTIVE branches. proxy_activates_with_custom_credentials_only guard test at line 503. Commit 62dbf013 confirmed no-op. |
| `crates/nono-cli/src/exec_strategy_windows/` | Byte-unchanged (AppContainer/WFP/broker) | VERIFIED | git diff ed6cdde1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ = 0 lines |
| `.planning/phases/94-upst10-divergence-audit/94-DIVERGENCE-LEDGER.md` | All will-sync rows closed; Cluster D marked won't-sync | VERIFIED | phase-95-status column: A=absorbed, B=absorbed, C=absorbed, D=won't-sync. No open will-sync rows. |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `ae77d198` (Cluster A) | git log | (cherry picked from commit 9ce74e92...) + DCO | VERIFIED | Cherry-pick trailer present; Oscar Mack Jr DCO present |
| `91d526e6` (Cluster B) | git log | "Cherry-picked from upstream nolabs-ai/nono 11fd10e0" + DCO | VERIFIED | Upstream SHA reference in body; DCO signed |
| `62dbf013` (Cluster C) | proxy_runtime.rs | || !prepared.custom_credentials.is_empty() preserved | VERIFIED | Lines 95, 118 intact; commit is a deliberate structural no-op |
| `endpoint_policy` (route.rs:41) | reverse.rs request path | exhaustive match evaluate() call at line 133 | VERIFIED | Wired by c81429aa; code-review Approve fail-open fixed by 97f387f2 |
| `read_msghdr_dest` (linux.rs) | offset_of! derivation | compile-time layout proof at lines 2649-2652 | VERIFIED | offset_of!(libc::msghdr, msg_name) and msg_namelen; hardcoded MSGHDR_MIN_READ = 12 absent |
| `caps.gpu()` (capability.rs) | linux.rs apply path | Landlock allowlist branch at line 907 | VERIFIED | caps.gpu() branch at line 907; collect_linux_gpu_paths at line 480 |
| Post-fork error paths (exec_strategy.rs) | static byte string pattern | MSG_PROXY_WRITE_FAIL/MSG_PROXY_FILTER_FAIL const &[u8] | VERIFIED | Lines 1415/1457 use libc::write + _exit(126); no format!() in child arm |

---

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---------|--------------|--------|-------------------|--------|
| `reverse.rs:133-186` request path | `route.endpoint_policy` | `CompiledEndpointPolicy::evaluate(&method, &upstream_path)` | Yes — exhaustive match; Deny and Approve each return 403; Allow falls through | VERIFIED |
| `linux.rs:read_msghdr_dest` | `MSG_NAME_OFFSET`, `MSG_NAME_LEN_OFFSET`, `PTR_SIZE` | `core::mem::offset_of!(libc::msghdr, ...)` and `core::mem::size_of::<usize>()` | Yes — compile-time constants derived from struct layout | VERIFIED |
| `linux.rs:907` apply path | `caps.gpu()` | `collect_linux_gpu_paths()` → gpu_paths + nvidia_present | Yes — reads /dev/dri, /dev/nvidia*, /dev/nvidia-caps at runtime | VERIFIED |

---

## Behavioral Spot-Checks

Step 7b: SKIPPED for Linux-gated behaviors — no Linux environment available. Windows-host checks:

| Behavior | Command | Result | Status |
|---------|---------|--------|--------|
| nono-proxy tests pass including new CR-01 test | `cargo test -p nono-proxy` | 176 passed (175 prior + 1 new endpoint_policy_approve_without_backend_is_recognized) | PASS |
| Workspace clippy clean | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | exit 0 on Windows host | PASS |
| Rustfmt clean | `cargo fmt --all --check` | exit 0 on Windows host | PASS |

---

## Probe Execution

Step 7c: No probe-*.sh files declared or discovered for Phase 95. No probe execution required.

---

## Requirements Coverage

| Requirement | Phase | Description | Status | Evidence |
|-------------|-------|------------|--------|---------|
| UPST10-02 | Phase 95 | All will-sync clusters absorbed without regressing Windows security model | VERIFIED (PARTIAL→CI cross-target) | Cluster A (9ce74e92/ae77d198 AF_UNIX fix), Cluster B (11fd10e0/91d526e6 shared-surface split), Cluster C (9b37dc52/62dbf013 structural no-op) all absorbed with DCO. WR-02/WR-03/WR-01/CR-01 fork invariants restored. exec_strategy_windows/ byte-unchanged. Cross-target clippy PARTIAL→CI per CLAUDE.md cross-target-verify-checklist.md; Phase 96 installs C cross-linker. |
| UPST10-03 | Phase 95 | Fork-divergent invariants explicitly preserved and verified post-sync | VERIFIED (PARTIAL→CI cross-target) | All 4 fork invariants confirmed present via static grep. Windows backend (exec_strategy_windows/) byte-unchanged. ADR-86 audit boundary: DiagnosticFormatter in CLI diagnostic/; records_verified: event_count > 0 at audit.rs:1570. Phase 89 proxy predicate preserved at proxy_runtime.rs:95,118. make build + make test green on Windows host (pre-existing baseline reds unchanged). |

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|---------|--------|
| ~~`crates/nono-proxy/src/reverse.rs`~~ | ~~125-151~~ | ~~CR-01 Approve fail-open (if let Deny only)~~ | ~~BLOCKER~~ | **FIXED** by commit 97f387f2 — now exhaustive match; see RESOLVED section |
| `crates/nono/src/audit.rs` | 585, 591 | `#[cfg_attr(not(target_os = "linux"), allow(dead_code))]` on record_sandbox_runtime_event/record_command_policy_event | WARNING | Dead on Linux too; CLAUDE.md prohibits allow(dead_code) for unused code — add callers or remove. Non-blocking for Phase 95 (Cluster B upstream inheritance). |
| `crates/nono/src/keystore.rs` | ~862, ~870 | `is_cmd_uri` / `validate_cmd_uri` have zero callers; inline starts_with used instead | WARNING | Validator behavior unverified at actual use site. Non-blocking for Phase 95 (Cluster B upstream inheritance). |
| `crates/nono-cli/tests/resl_nix_async_signal_safety.rs` | 163-170 | Comment stripper truncates at first `//` including inside string literals | WARNING | WR-01 guard test weaker than stated mandate — misses to_string(), String::, etc. Non-blocking for Phase 95 but should be hardened in a follow-up. |

### RESOLVED Anti-Patterns

The CR-01 code-review CRITICAL (95-REVIEW.md) was fixed before this verification:

- **reverse.rs CR-01 Approve fail-open** (previously BLOCKER): The code-review found that `c81429aa` used `if let EndpointPolicyOutcome::Deny { .. }`, letting `Approve` outcomes silently forward requests with real credentials. Fixed by commit `97f387f2`: replaced with exhaustive `match` over `Allow`/`Deny`/`Approve`; `Approve` arm fails closed with 403 + `audit::log_denied`. New test `endpoint_policy_approve_without_backend_is_recognized` in `config.rs:1315` documents the contract.

---

## Human Verification Required

### 1. GH Actions Cross-Target Clippy Confirmation

**Test:** Check GH Actions Linux and macOS Clippy job results for HEAD 544cab40 on branch `milestone/v2.13-carryforward-closeout`.
**Expected:** Both `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `--target x86_64-apple-darwin` exit 0 with no new warnings or errors. Any failures in cfg-gated Unix code changed by this phase (sandbox/linux.rs, exec_strategy.rs, reverse.rs) would be a blocker requiring fix before Phase 96 proceeds.
**Why human:** Cross C linker (x86_64-linux-gnu-gcc) and osxcross/macOS SDK are absent on the Windows dev host — this is itself a Phase 96 deliverable (XTGT-01). The CI lanes are the decisive cross-target signal per CLAUDE.md and the cross-target-verify-checklist.md. Cannot run `cargo clippy --target x86_64-unknown-linux-gnu` locally due to the missing C toolchain required by aws-lc-sys/ring.

---

## What the Absorb Achieved (Confirmed)

These items are verified and must not be re-opened:

1. **Cluster A AF_UNIX deadlock fix landed** — ae77d198 is in git log with -x trailer and DCO. All 4 bugs fixed: sendmsg deadlock, wrong jt offsets, rate-limiter starvation, dup2 bypass.
2. **exec_strategy_windows/ byte-unchanged** — git diff ed6cdde1..HEAD returns 0 lines. Windows AppContainer/WFP/broker completely unaffected.
3. **ADR-86 audit boundary intact** — records_verified: event_count > 0 at audit.rs:1570. DiagnosticFormatter in CLI-side diagnostic/. bindings/c/src/ untouched by the sync window.
4. **CR-02 invariant intact** — verify_empty_log_with_no_stored_metadata_is_not_valid guard test in audit.rs; additive SandboxRuntimeAuditEvent/CommandPolicyAuditEvent landed without touching the carve-out field.
5. **Cluster B additive surface absorbed correctly** — SandboxRuntimeAuditEvent, CommandPolicyAuditEvent, restrict_execute, CMD_URI_PREFIX, SENSITIVE_ENV_VARS expansion, endpoint_policy field all present. tool-sandbox/ dir and tls_intercept/ absent as planned.
6. **Cluster C Phase 89 invariant preserved** — proxy_activates_with_custom_credentials_only test at proxy_runtime.rs:503; || !prepared.custom_credentials.is_empty() at lines 95/118. Structural no-op confirmed.
7. **DIVERGENCE-LEDGER all will-sync rows closed** — Cluster A is the only will-sync cluster; it is absorbed. Clusters B/C absorbed as split. Cluster D won't-sync (deferred to Phase 97).
8. **WR-02 restored** — offset_of! derivation at linux.rs:2649-2652 with compile-time assertions; hardcoded LP64-only constant absent. Commit 8bca078b.
9. **WR-03 restored** — collect_linux_gpu_paths at linux.rs:480, is_nvidia_compute_device at line 432, caps.gpu() branch at line 907. 4 GPU tests added. Commit 5d1e9077.
10. **WR-01 restored** — MSG_PROXY_WRITE_FAIL / MSG_PROXY_FILTER_FAIL const &[u8] at exec_strategy.rs:1415/1457; async-signal-safe libc::write + _exit(126). Commit 2a8b639e.
11. **CR-01 wired and fail-closed** — exhaustive match over EndpointPolicyOutcome in reverse.rs:133-186; Approve arm fails closed with 403 + audit::log_denied. Commits c81429aa + 97f387f2.

---

## Note on Verification History

**Initial pass (Plan 95-04):** Self-verification checked exec_strategy_windows/ byte-stability, the ADR-86 audit boundary, and the Cluster C proxy guard. It did not inspect linux.rs hunks changed by the Cluster A cherry-pick conflict resolution. External verification found two confirmed regressions (WR-02, WR-03) and two additional gaps (CR-01, WR-01) in that uninspected surface.

**Gap-closure (Plans 95-05 and 95-06):** WR-02 restored by 8bca078b; WR-03 restored by 5d1e9077; WR-01 fixed by 2a8b639e; CR-01 initial wire by c81429aa.

**Code-review (95-REVIEW.md, 2026-06-26T13:16:40Z):** Found a CRITICAL fail-open in the CR-01 wiring — the if-let pattern let Approve outcomes silently forward requests with real credentials. Fixed by commit 97f387f2 (exhaustive match, Approve arm fails closed).

**Plan 95-07 cross-target:** PARTIAL→CI — C cross-linker absent on Windows host; all Rust targets installed but aws-lc-sys/ring require cc/x86_64-linux-gnu-gcc. GH Actions lanes are decisive. Phase 96 installs the local C cross-toolchain.

**This final verification (external, adversarial stance):** All 11 must-haves verified by static analysis of actual source files. The only outstanding item is CI-lane confirmation of cross-target clippy, which is correctly deferred per the documented cross-target-verify-checklist.md protocol.

---

_Verified: 2026-06-26T14:00:00Z_
_Verifier: Claude (gsd-verifier) — external verification, adversarial stance_
_Verification passes: 3 (initial → gap-closure → final with code-review fix)_
