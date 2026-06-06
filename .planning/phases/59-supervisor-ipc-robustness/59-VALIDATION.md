---
phase: 59
slug: supervisor-ipc-robustness
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-06
---

# Phase 59 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`#[test]`), `tempfile`, `proptest` (cli) |
| **Config file** | none — Cargo-native; integration tests under `crates/nono-cli/tests/` |
| **Quick run command** | `cargo test -p nono --lib supervisor::socket` |
| **Full suite command** | `make test` (or `cargo test --workspace`) |
| **Estimated runtime** | ~60–120 seconds (full suite); <10s for the targeted IPC tests |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono --lib supervisor::socket` + `cargo build --workspace`
- **After every plan wave:** Run `make test` (baseline-aware: the documented pre-existing Windows env failures are red→red carry-forwards, NOT regressions — see [[nono_cli_windows_baseline_test_failures]])
- **Before `/gsd:verify-work`:** Full suite green (modulo documented baseline) AND the Windows live-repro signed off
- **Max feedback latency:** ~10 seconds for the targeted IPC tests (set `NONO_SUPERVISOR_IPC_READ_TIMEOUT` low in tests to keep CI fast)

---

## Per-Task Verification Map

> Task IDs are placeholders pending the planner's wave/plan breakdown; the planner MUST keep every REQ-IPC-01 row backed by an automated `<verify>` or a Wave 0 dependency.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 59-01-* | 01 | 1 | REQ-IPC-01 (D-01) | T-59-02 | new read-timeout const honors `NONO_*` env override + `MAX_TIMEOUT` clamp; invalid env → safe default, never unbounded | unit | `cargo test -p nono-cli timeouts::` | ⚠️ extend existing | ⬜ pending |
| 59-02-* | 02 | 2 | REQ-IPC-01 (SC1/SC2 Unix) | T-59-01 | child closes IPC then reconnects → supervisor survives & re-accepts; partial-frame stall → bounded timeout fires (keep-alive, not fatal) | integration | `cargo test -p nono-cli --test supervisor_ipc_robustness` | ❌ W0 | ⬜ pending |
| 59-03-* | 03 | 2 | REQ-IPC-01 (SC4 Windows) | T-59-01 | AIPC `read_frame` bounded by `PeekNamedPipe` deadline; transient close → re-accept, capability pipe not permanently disabled | integration + live-repro | `cargo test -p nono-cli --test supervisor_ipc_robustness` (compile-all-platforms) + documented Win11 UAT | ❌ W0 | ⬜ pending |
| 59-* | regression | — | REQ-IPC-01 | — | existing round-trip framing unchanged | integration | `cargo test -p nono-cli --test aipc_handle_brokering_integration` | ✅ exists | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono-cli/tests/supervisor_ipc_robustness.rs` — net-new integration tests for SC1 (reconnect survival) and SC2 (bounded read timeout). Must be cfg-structured to **compile on all 3 platforms** (Unix uses socketpair fork; Windows uses the AIPC pipe) — mirror the `aipc_handle_brokering_integration.rs` cfg/empty-binary pattern.
- [ ] **Update** the existing fork test at `crates/nono-cli/src/exec_strategy.rs:4057-4067` — it asserts the OLD break-on-close (POLLHUP returns) behavior, which SC1 inverts. It must be updated to assert keep-alive/re-accept for the URL/direct-IPC listener path.
- [ ] Extend `crates/nono-cli/src/timeouts.rs` unit tests to cover `NONO_SUPERVISOR_IPC_READ_TIMEOUT` parse + clamp (save/restore env per CLAUDE.md env-var test rule).

*Existing framework (Cargo test runner) covers all phase requirements — no new test framework needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Windows AIPC bounded-read + re-accept under real named-pipe timing | REQ-IPC-01 (SC4) | CI cannot deterministically exercise named-pipe `PIPE_WAIT` timing; project Windows-UAT pattern requires a real Win11 console | Run a slow-child (holds partial frame) and a disconnect-then-reconnect script against a supervised `nono run` from a real Win11 console; observe (a) supervisor not blocked past the deadline and (b) capability pipe re-accepts after the transient close. Document repro + result in the plan SUMMARY. |

*Sampling note: the 5s timeout + ~200ms poll tick set temporal resolution — a reconnect test must wait > one poll tick (200ms) to observe re-accept; a timeout test must hold a partial frame > the configured timeout (override `NONO_SUPERVISOR_IPC_READ_TIMEOUT` to ~1s in CI with save/restore).*

---

## Validation Sign-Off

- [ ] All tasks have an automated `<verify>` or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (the net-new integration test file + the updated break-on-close test)
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s for targeted IPC tests
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
