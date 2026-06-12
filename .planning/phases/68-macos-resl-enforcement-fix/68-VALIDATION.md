---
phase: 68
slug: macos-resl-enforcement-fix
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-12
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
| 68-XX-XX | XX | 1 | RESL-MAC-02 | — | `--max-processes N` → EAGAIN past cap on real macOS host (`setrlimit(RLIMIT_NPROC, baseline+N)` applied pre-exec) | integration (env-gated) | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos macos_max_processes_blocks_on_rlimit_nproc` | ✅ | ⬜ pending |
| 68-XX-XX | XX | 1 | RESL-MAC-01 | — | `--timeout D` SIGKILLs child at deadline on real macOS host (own-pgrp + watchdog group-kill, supervisor `wait()` returns) | integration (env-gated) | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos macos_timeout_kills_at_deadline` | ✅ | ⬜ pending |
| 68-XX-XX | XX | 1 | CR-01 (async-signal-safety) | — | No `format!()`/alloc in child arm; `MSG_RLIMIT_NPROC_FAIL` const present (≥11 such message consts) | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety` | ✅ | ⬜ pending |
| 68-XX-XX | XX | 1 | WR-04 (no PID fallback) | — | watchdog uses `match getpgid` with skip-on-Err (no PID fallback under PID reuse) | unit (source scan) | `cargo test -p nono-cli --test resl_nix_async_signal_safety wr_04_no_pid_fallback_on_getpgid_failure` | ✅ | ⬜ pending |
| 68-XX-XX | XX | 1 | RESL-MAC-01+02 (CI gate) | — | Build/clippy stays green; gated tests skip cleanly off the runner | build + clippy | `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*Task IDs (`68-XX-XX`) are placeholders — the planner assigns concrete plan/task IDs; the executor updates this map.*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* No new test files needed. The two gated tests (`macos_timeout_kills_at_deadline`, `macos_max_processes_blocks_on_rlimit_nproc`), the `host_enforcement_validated()` gate, the `run_bounded` harness, and the `resl_nix_async_signal_safety` source-scan suite all already exist.

- [ ] (Conditional, D-09 bonus only) One `--memory` / RLIMIT_AS live assertion appended to `resl_nix_macos.rs`, gated on `host_enforcement_validated()` — secondary, no new requirement.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real-host enforcement of `--timeout` and `--max-processes` | RESL-MAC-01, RESL-MAC-02 | CI runners hang on these tests (they are env-gated OFF the runner by design); kernel `setrlimit`/process-group enforcement requires a real macOS host | On `Oscars-MacBook-Pro` (Apple Silicon): `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos` — both gated tests must PASS |
| Cross-target clippy (Linux + macOS) | RESL-MAC-01, RESL-MAC-02 (D-10) | Windows dev host cannot cross-compile (ring/aws-lc-sys C toolchain); CI is the load-bearing signal | Per `.planning/templates/cross-target-verify-checklist.md`; mark cross-target REQ PARTIAL/deferred-to-CI if cross-toolchain unavailable on dev host; macOS CI build leg must stay green |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
