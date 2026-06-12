---
phase: 68
slug: macos-resl-enforcement-fix
status: in-progress
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-12
updated: 2026-06-12
---

# Phase 68 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner + `cargo test` |
| **Config file** | None (standard Cargo) |
| **Quick run command** | `cargo test -p nono-cli --test resl_nix_async_signal_safety` |
| **Full suite command** | `cargo test -p nono-cli` |
| **Estimated runtime** | ~5s (quick source-scan) / ~60s (full suite, Windows host) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli --test resl_nix_async_signal_safety` (fast source-scan tests)
- **After every plan wave:** Run `cargo test -p nono-cli` (full suite incl. skip-path for env-gated host tests)
- **Before `/gsd:verify-work`:** Full suite green **AND** real macOS host UAT with `NONO_RESL_HOST_VALIDATED=1` (both gated tests PASS on `Oscars-MacBook-Pro`)
- **Max feedback latency:** ~60 seconds (host-gated enforcement tests are out-of-band, run on the macOS host)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 68-01-T1 | 01 | 1 | RESL-MAC-02 | T-68-01, T-68-05 | `uid_process_count()` + `baseline_uid_count` field + `install_pre_exec` RLIMIT_NPROC via `libc::setrlimit` (Direct path); fail-closed on sysctl/setrlimit failure | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety` | ✅ | ✅ |
| 68-01-T2 | 01 | 1 | RESL-MAC-01, RESL-MAC-02 | T-68-02, T-68-06 | `setpgid(0,0)` in child arm + `MSG_RLIMIT_NPROC_FAIL` + real `libc::setrlimit(RLIMIT_NPROC)` in supervised child arm; CR-01/WR-02/WR-04 preserved | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety` | ✅ | ✅ |
| 68-01-T3 | 01 | 1 | RESL-MAC-01, RESL-MAC-02 (D-08, D-09) | — | D-09 bonus test added; Windows host suite green (4 pre-existing failures); cross-target deferred to CI; real macOS host UAT gate (awaiting user) | integration (env-gated) | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos -- --nocapture` | ✅ | 🔲 in-progress (awaiting macOS host UAT) |
| 68-01-T3-cr01 | 01 | 1 | CR-01 (async-signal-safety) | T-68-06 | No `format!()`/alloc in child arm; `MSG_RLIMIT_NPROC_FAIL` const present (≥11 such message consts) | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety` | ✅ | ✅ |
| 68-01-T3-wr04 | 01 | 1 | WR-04 (no PID fallback) | — | watchdog uses `match getpgid` with skip-on-Err (no PID fallback under PID reuse) | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety wr_04_no_pid_fallback_on_getpgid_failure` | ✅ | ✅ |
| 68-01-T3-ci | 01 | 1 | RESL-MAC-01+02 (CI gate) | — | Build/clippy stays green; gated tests skip cleanly off the runner | build + clippy | `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` | ✅ | 🔲 deferred-to-CI (cross-target) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · 🔲 in-progress*

*Task IDs updated from 68-XX-XX placeholders to 68-01-T1/T2/T3.*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* No new test files needed. The two gated tests (`macos_timeout_kills_at_deadline`, `macos_max_processes_blocks_on_rlimit_nproc`), the `host_enforcement_validated()` gate, the `run_bounded` harness, and the `resl_nix_async_signal_safety` source-scan suite all already exist.

- [x] (Conditional, D-09 bonus only) One `--memory` / RLIMIT_AS live assertion appended to `resl_nix_macos.rs`, gated on `host_enforcement_validated()` — secondary, no new requirement. **DONE: `macos_memory_limit_kills_at_rlimit_as` added (commit 3583bacc).**

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real-host enforcement of `--timeout` and `--max-processes` | RESL-MAC-01, RESL-MAC-02 | CI runners hang on these tests (they are env-gated OFF the runner by design); kernel `setrlimit`/process-group enforcement requires a real macOS host | On `Oscars-MacBook-Pro` (Apple Silicon): `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos` — both gated tests must PASS |
| Cross-target clippy (Linux + macOS) | RESL-MAC-01, RESL-MAC-02 (D-10) | Windows dev host cannot cross-compile (ring/aws-lc-sys C toolchain); CI is the load-bearing signal | Per `.planning/templates/cross-target-verify-checklist.md`; mark cross-target REQ PARTIAL/deferred-to-CI if cross-toolchain unavailable on dev host; macOS CI build leg must stay green |

---

## Automated Verification Results (Windows Dev Host — 2026-06-12)

| Check | Result | Notes |
|-------|--------|-------|
| `cargo test -p nono-cli --test resl_nix_async_signal_safety` | ✅ 5/5 pass | After Task 1, Task 2, and Task 3 automated steps |
| `cargo test -p nono-cli` | ✅ 1211 pass, 4 pre-existing failures | Pre-existing: profile_cmd init + 3 protected_paths (env-specific, documented) |
| `cargo check -p nono-cli` | ✅ clean | Windows host compile gate |
| `grep MSG_RLIMIT_NPROC_FAIL exec_strategy.rs` | ✅ >= 1 | Found 3 occurrences (const decl + write + comment) |
| `grep RLIMIT_NPROC_UNAVAILABLE exec_strategy.rs` | ✅ 0 | Old no-op removed |
| `grep uid_process_count supervisor_macos.rs` | ✅ 1 | Function present |
| `grep setpgid exec_strategy.rs` | ✅ 5 | setpgid call in child arm present |
| `grep "match getpgid(" exec_strategy.rs` | ✅ 1 | WR-04 preserved |
| `grep "unwrap_or(child)" exec_strategy.rs` | ✅ 0 | WR-04 no PID fallback |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** Tasks 1 & 2 complete; Task 3 awaiting macOS host UAT (human-verify gate)
