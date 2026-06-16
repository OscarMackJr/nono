---
phase: 73
slug: ai-agent-marker
status: passed
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-14
verified: 2026-06-16
verification_note: "All 6 validation rows pass (run 2026-06-16 during v2.12 milestone audit gap-closure): classify_current_process_not_agent + classify_nonexistent_pid_not_agent (SC2, in the 8-test classify_ set), job_never_has_breakaway_ok + job_security_descriptor_denies_low_il (SC3), and the two #[ignore] SC4 authoritative integration tests sc4_classify_real_agent + sc4_classify_spoof_not_agent (real Win11 host, dev-layout broker). See 73-VERIFICATION.md."
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
- **Before `/gsd:verify-work`:** Full `make test` green + real Win11 UAT (`nono classify <confined-pid>` outputs "structural match (non-authoritative)" with NOTE; SC4 `--ignored` tests pass with in-process registry)
- **Max feedback latency:** ~60 seconds (unit loop)

---

## Per-Task Verification Map

> Provisional — the planner finalizes task IDs. Mapped from RESEARCH.md §Validation Architecture.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|------|--------|
| TBD | 01 | 1 | MARK-01 SC2/SC4 | T-73-spoof | Non-agent PID → `NotAnAgent` (fail-secure) | unit | `cargo test -p nono --target x86_64-pc-windows-msvc agent::tests::classify_current_process_not_agent` | `crates/nono/src/agent.rs` | ✅ passed |
| TBD | 01 | 1 | MARK-01 SC4 | — | classify(nonexistent_pid) → `NotAnAgent` | unit | `cargo test -p nono --target x86_64-pc-windows-msvc agent::tests::classify_nonexistent_pid_not_agent` | `crates/nono/src/agent.rs` | ✅ passed |
| TBD | 02 | 1 | MARK-01 SC3 | T-73-breakaway | `JOB_OBJECT_LIMIT_BREAKAWAY_OK` never set | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc launch::tests::job_never_has_breakaway_ok` | `crates/nono-cli/src/exec_strategy_windows/launch.rs` | ✅ passed |
| TBD | 02 | 1 | MARK-01 SC3 | T-73-job-acl | Job SD denies Low-IL / package SID | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc launch::tests::job_security_descriptor_denies_low_il` | `crates/nono-cli/src/exec_strategy_windows/launch.rs` | ✅ passed |
| TBD | 03 | 2 | MARK-01 SC1/SC4 | — | In-process: same process mints SID, inserts, classifies confined child → `AiAgent` (authoritative) | integration (real child, `#[ignore]`) | `cargo test -p nono-cli --target x86_64-pc-windows-msvc -- --ignored sc4_classify_real_agent` | `crates/nono-cli/src/exec_strategy_windows/launch.rs` broker_dispatch_tests | ✅ passed |
| TBD | 03 | 2 | MARK-01 SC2 | T-73-spoof | In-process: self-made AppContainer (not in registry) → `NotAnAgent` | integration (real child, `#[ignore]`) | `cargo test -p nono-cli --target x86_64-pc-windows-msvc -- --ignored sc4_classify_spoof_not_agent` | `crates/nono-cli/src/exec_strategy_windows/launch.rs` broker_dispatch_tests | ✅ passed |
| TBD | 03 | 2 | MARK-01 SC5 | — | `nono classify <pid>` prints structural/non-authoritative verdict; never "AI_AGENT" from standalone | smoke (manual UAT) | manual + `cargo run -p nono-cli -- classify --help \| grep -i pid` | `crates/nono-cli/src/classify_runtime.rs` | ✅ passed |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**SC4 test location note:** `sc4_classify_real_agent` and `sc4_classify_spoof_not_agent` are canonical in `crates/nono-cli/src/exec_strategy_windows/launch.rs` **`broker_dispatch_tests`** module (Wave 0 item below). This module already has real-child spawn infrastructure (BrokerLaunchNoPty arm, IsProcessInJob, `#[ignore]` pattern). NOT in `classify_runtime.rs`.

---

## Wave 0 Requirements

- [ ] `crates/nono/src/agent.rs` — new module: `AgentRegistry`, `AgentClassification`, `read_process_appcontainer_sid` (Windows + non-Windows stub) with `#[cfg(all(test, target_os = "windows"))]` unit tests for SC2 + SC4 fail-secure paths
- [ ] `crates/nono-cli/src/exec_strategy_windows/launch.rs` test module additions — `job_never_has_breakaway_ok` + `job_security_descriptor_denies_low_il`
- [ ] `crates/nono-cli/src/exec_strategy_windows/launch.rs` **broker_dispatch_tests** additions — `sc4_classify_real_agent` + `sc4_classify_spoof_not_agent` (both `#[ignore]`, real-child / real-AppContainer; canonical SC4 location)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `nono classify <confined-pid>` outputs "structural match (non-authoritative)" against a live confined agent (NOT "AI_AGENT") | MARK-01 SC5/D-04 | Requires a real Win11 host + live confined child; standalone classify has empty registry so the authoritative "AI_AGENT" verdict is impossible cross-process | Launch a confined agent via the Broker arm; capture its PID; run `nono classify <pid>` from a second terminal; assert "structural match (non-authoritative)" + NOTE. Note: "AI_AGENT" is NOT expected from standalone classify. |
| SC4 in-process integration tests (`#[ignore]`) pass | MARK-01 SC1/SC2 | Spawn a real confined process + a real spoof AppContainer; CI Windows runner may lack AppContainer/dev-layout trust | Run with `-- --ignored` on a real Win11 host during UAT; these are the AUTHORITATIVE AI_AGENT proofs |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
