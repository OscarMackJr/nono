---
phase: 73
slug: ai-agent-marker
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-14
---

# Phase 73 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none — workspace-level `[lints]` in `Cargo.toml` |
| **Quick run command** | `cargo test -p nono --target x86_64-pc-windows-msvc` |
| **Full suite command** | `make test` (lib + cli + doc tests) |
| **Estimated runtime** | ~60 seconds (unit); real-child integration tests are `#[ignore]` (manual/UAT) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono --target x86_64-pc-windows-msvc` (unit tests; fast)
- **After every plan wave:** Run `cargo test -p nono -p nono-cli --target x86_64-pc-windows-msvc` + cross-target clippy (`--target x86_64-unknown-linux-gnu` and `--target x86_64-apple-darwin`, per CLAUDE.md MUST)
- **Before `/gsd:verify-work`:** Full `make test` green + real Win11 UAT (`nono classify <confined-pid>` outputs `AI_AGENT`)
- **Max feedback latency:** ~60 seconds (unit loop)

---

## Per-Task Verification Map

> Provisional — the planner finalizes task IDs. Mapped from RESEARCH.md §Validation Architecture.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | 01 | 1 | MARK-01 SC2/SC4 | T-73-spoof | Non-agent PID → `NotAnAgent` (fail-secure) | unit | `cargo test -p nono --target x86_64-pc-windows-msvc agent::tests::classify_current_process_not_agent` | ❌ W0 | ⬜ pending |
| TBD | 01 | 1 | MARK-01 SC4 | — | classify(nonexistent_pid) → `NotAnAgent` | unit | `cargo test -p nono --target x86_64-pc-windows-msvc agent::tests::classify_nonexistent_pid_not_agent` | ❌ W0 | ⬜ pending |
| TBD | 02 | 1 | MARK-01 SC3 | T-73-breakaway | `JOB_OBJECT_LIMIT_BREAKAWAY_OK` never set | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc launch::tests::job_never_has_breakaway_ok` | ❌ W0 | ⬜ pending |
| TBD | 02 | 1 | MARK-01 SC3 | T-73-job-acl | Job SD denies Low-IL / package SID | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc launch::tests::job_security_descriptor_denies_low_il` | ❌ W0 | ⬜ pending |
| TBD | 03 | 2 | MARK-01 SC1 | — | Launched agent classifies as `AI_AGENT` | integration (real child) | `cargo test -p nono-cli --target x86_64-pc-windows-msvc sc4_classify_real_agent -- --ignored` | ❌ W0 | ⬜ pending |
| TBD | 03 | 2 | MARK-01 SC2 | T-73-spoof | Self-made AppContainer (not in registry) → `NotAnAgent` | integration (real child) | `cargo test -p nono-cli --target x86_64-pc-windows-msvc sc4_classify_spoof_not_agent -- --ignored` | ❌ W0 | ⬜ pending |
| TBD | 03 | 2 | MARK-01 SC5 | — | `nono classify <pid>` prints "not an agent" for unrelated PID | smoke (manual UAT) | manual only | — | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono/src/agent.rs` — new module: `AgentRegistry`, `AgentClassification`, `read_process_appcontainer_sid` (Windows + non-Windows stub) with `#[cfg(all(test, target_os = "windows"))]` unit tests for SC2 + SC4
- [ ] `crates/nono-cli/src/exec_strategy_windows/launch.rs` test module additions — `job_never_has_breakaway_ok` + `job_security_descriptor_denies_low_il`
- [ ] `crates/nono-cli/src/exec_strategy_windows/launch.rs` broker integration tests — `sc4_classify_real_agent` + `sc4_classify_spoof_not_agent` (both `#[ignore]`, real-child / real-AppContainer)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `nono classify <confined-pid>` outputs `AI_AGENT` against a live confined agent | MARK-01 SC1/SC4/SC5 | Requires a real Win11 host, dev-layout/signed `nono.exe` (broker trust gate), and a live AppContainer confined child the unit harness cannot mint | Launch a confined agent via the Broker arm; capture its PID; run `nono classify <pid>` from the launcher; assert `AI_AGENT`. Run on a non-agent PID; assert "not an agent". |
| Real-child integration tests (`#[ignore]`) | MARK-01 SC1/SC2 | Spawn a real confined process + a real spoof AppContainer; CI Windows runner may lack AppContainer/dev-layout trust | Run with `-- --ignored` on a real Win11 host during UAT |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
