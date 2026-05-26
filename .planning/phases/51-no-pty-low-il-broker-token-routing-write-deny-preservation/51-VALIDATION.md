---
phase: 51
slug: no-pty-low-il-broker-token-routing-write-deny-preservation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-26
---

# Phase 51 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `51-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none — Cargo.toml `[dev-dependencies]` (no separate config) |
| **Quick run command** | `cargo test -p nono-cli pty_token_gate_tests` |
| **Full suite command** | `cargo test -p nono-cli --target x86_64-pc-windows-msvc && cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc` |
| **Estimated runtime** | ~30 seconds (quick) / ~3 min (full Windows suite incl. real-spawn) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli pty_token_gate_tests` (fast pure-logic check)
- **After every plan wave:** Run `cargo test -p nono-cli --target x86_64-pc-windows-msvc && cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc`
- **Before `/gsd:verify-work`:** Full Windows suite green + cross-target clippy (or PARTIAL with CI deferral per `.planning/templates/cross-target-verify-checklist.md`)
- **Max feedback latency:** 30 seconds (quick) — real-spawn integration runs at wave boundaries

---

## Per-Task Verification Map

> Requirement-level map from RESEARCH. Task IDs (`51-NN-NN`) are bound during planning;
> the planner MUST attach `<automated>` verify commands to each task that map to these rows.

| Requirement | Behavior | Test Type | Automated Command | File Exists |
|-------------|----------|-----------|-------------------|-------------|
| REQ-WSRH-01 | Broker accepts `--no-pty` and launches child with pipe stdio | Integration (real-spawn) | `cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc` (new `parse_args_no_pty` + `run_no_pty_launch`) | ❌ W0 |
| REQ-WSRH-02 | `select_windows_token_arm(prefers_low_il_broker=true)` → `BrokerLaunchNoPty` | Unit | `cargo test -p nono-cli pty_token_gate_tests::pty_none_session_sid_with_broker_opt_in_selects_broker_launch_no_pty` | ❌ W0 |
| REQ-WSRH-02 | `prefers_low_il_broker=false` still → `WriteRestricted` (regression-safe) | Unit (existing) | `cargo test -p nono-cli pty_token_gate_tests::pty_none_with_session_sid_selects_write_restricted` | ✅ launch.rs:1934 |
| REQ-WSRH-03 | Low-IL child write to Medium-IL-labeled path kernel-denied (MIC pre-DACL) | Integration (real-spawn) | `cargo test -p nono-cli --target x86_64-pc-windows-msvc write_deny_low_il_broker_no_pty_tests` | ❌ W0 |
| REQ-WSRH-05 | No regression: PTY path still selects `BrokerLaunch` | Unit (existing) | `cargo test -p nono-cli pty_token_gate_tests::pty_some_no_detach_selects_broker_launch` | ✅ launch.rs:1896 |
| REQ-WSRH-05 | All existing `broker_dispatch_tests` pass | Integration (existing) | `cargo test -p nono-cli broker_dispatch_tests` | ✅ launch.rs:2346 |
| REQ-WSRH-05 | Cross-target Linux clippy clean | Clippy | `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | N/A |
| REQ-WSRH-05 | Cross-target macOS clippy clean | Clippy | `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | N/A |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] New unit test `pty_none_session_sid_with_broker_opt_in_selects_broker_launch_no_pty` in `pty_token_gate_tests` (launch.rs) — REQ-WSRH-02
- [ ] New integration test module `write_deny_low_il_broker_no_pty_tests` in launch.rs — REQ-WSRH-03
- [ ] New unit tests `parse_args_no_pty_flag_accepted` + `run_no_pty_pipes_bound` in `nono-shell-broker/src/main.rs` — REQ-WSRH-01 broker side
- [ ] Pre-build step for `nono-shell-broker.exe` artifact before the D-07 real-spawn test can run

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `claude --version` survives on a real Windows host (end-to-end field validation) | REQ-WSRH-04/06 (Phase 52) | Requires the fixed binary on a real Windows host with the 234 MB `claude.exe` — out of scope for Phase 51 | Deferred to Phase 52 |
| Cross-target clippy when dev-host toolchains absent | REQ-WSRH-05 | If `x86_64-unknown-linux-gnu` / `x86_64-apple-darwin` toolchains are not installed on the dev host, mark PARTIAL and defer to live CI | Per `.planning/templates/cross-target-verify-checklist.md` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (4 new test scaffolds above)
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
