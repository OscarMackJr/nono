---
phase: 68
slug: macos-resl-enforcement-fix
status: in-progress
nyquist_compliant: false
wave_0_complete: false
t1_committed: true
t2_committed: true
t3_committed: true
t4_host_uat: pending
created: 2026-06-12
---

# Phase 68 — Validation Strategy (re-scoped: D1+D2+D3)

> Per-phase validation contract. The LOAD-BEARING gate is a real macOS build+test —
> Windows `cargo check` cannot compile the Apple cfg arms (this gap already shipped 2 compile
> errors + fmt debt this phase). See 68-RESEARCH.md "Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none |
| **Quick run command (Windows dev host — text-scan only)** | `cargo test -p nono-cli --test resl_nix_async_signal_safety` |
| **Full suite (Windows dev host)** | `cargo test -p nono-cli` (gated macOS tests skip without `NONO_RESL_HOST_VALIDATED=1`) |
| **LOAD-BEARING gate (real macOS host, Oscars-MacBook-Pro)** | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos -- --nocapture` |
| **Cross-target signal** | GH Actions macOS + Linux Clippy/build lanes on the head SHA |

---

## Sampling Rate

- **After every task commit (Windows dev host):** `cargo test -p nono-cli --test resl_nix_async_signal_safety` (text-scan invariants: CR-01 no-format, MSG_* const count, WR-02/WR-04).
- **Before `/gsd:verify-work`:** the real-macOS-host gate MUST pass — `macos_timeout_kills_at_deadline` + `macos_max_processes_blocks_on_rlimit_nproc` PASS with `NONO_RESL_HOST_VALIDATED=1`; the D-09 `macos_memory_limit_kills_at_rlimit_as` assertion is updated per the D2 downgrade.
- **Cross-target:** macOS + Linux CI lanes green on the head SHA (Windows host cannot cross-compile — PARTIAL/deferred-to-CI per `.planning/templates/cross-target-verify-checklist.md`).

---

## Per-Task Verification Map

*Populated by the planner (68-02-PLAN.md). Each defect maps to a proof:*

| Defect | Proof | Status |
|--------|-------|--------|
| P-A reaping | host probe: `time nono run --allow-cwd --read=... -- sleep 3` exits at ~3s | PASS (user-reported 2026-06-12) |
| D3 (watchdog/pgrp) | `macos_timeout_kills_at_deadline` PASS on host; `macos_max_processes_blocks_on_rlimit_nproc` PASS | pending T4 |
| D1 (set_read_timeout) | `--memory`/supervised runs no longer print `Failed to set socket read timeout`; supervised run completes on host | pending T4 |
| D2 (RLIMIT_AS) | no `_exit(126)` abort on `--memory`; D-09 test assertion flipped + passes/skips cleanly | code committed; pending T4 host |
| async-signal-safety invariants | `cargo test -p nono-cli --test resl_nix_async_signal_safety` 5/5 PASS (Windows dev host) | PASS (commits 924b4d60, c3cf3855, 648c5856) |

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| macOS resl enforcement fires | RESL-MAC-01/02 | CI runners hang on these (env-gated off CI); needs real Apple hardware | `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos` on Oscars-MacBook-Pro |
| P-A reaping baseline | (diagnostic) | needs real macOS host | `time nono run --allow-cwd --read=/bin --read=/usr --read=/private -- sleep 3` |

---

## Validation Sign-Off

- [x] D1/D2/D3 code committed (T1: 924b4d60, T2: c3cf3855, T3: 648c5856); async-signal-safety 5/5 PASS on dev host
- [ ] Real-macOS-host gate (Checkpoint T4): `NONO_RESL_HOST_VALIDATED=1 cargo test -p nono-cli --test resl_nix_macos` 5/5 PASS on Oscars-MacBook-Pro
- [ ] Cross-target macOS+Linux CI lanes green on head SHA (deferred to GH Actions — PARTIAL per cross-target-verify-checklist.md)
- [ ] `nyquist_compliant: true` set in frontmatter (after T4 PASS)

**Approval:** pending T4 host UAT
