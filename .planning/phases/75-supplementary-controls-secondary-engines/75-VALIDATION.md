---
phase: 75
slug: supplementary-controls-secondary-engines
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-15
---

# Phase 75 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `75-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`); nono-ts uses napi cargo tests + `npm test` |
| **Config file** | `Makefile` targets (`make test`, `make test-cli`); `../nono-ts/package.json` |
| **Quick run command** | `cargo test -p nono-cli --target x86_64-pc-windows-msvc` (Windows host) |
| **Full suite command** | `make ci` (clippy + fmt + tests; Windows host for nono-cli) + `npm test` in `../nono-ts/` |
| **Estimated runtime** | ~120 seconds (nono-cli) + ~30 seconds (nono-ts) |

---

## Sampling Rate

- **After every task commit:** `cargo test -p nono-cli` (Windows host) for daemon changes; `cargo test` in `../nono-ts/` for binding changes
- **After every plan wave:** `make ci` on nono repo; `npm test` in nono-ts repo
- **Before `/gsd:verify-work`:** Full suite green + all live Win11 UAT gates (SC3, SC5)
- **Max feedback latency:** ~150 seconds

---

## Per-Task Verification Map

> Populated during Wave 0 / per-plan authoring. Requirement→behavior map below is the seed (from RESEARCH.md § Phase Requirements → Test Map).

| Req ID | Behavior | Test Type | Automated Command | File Exists |
|--------|----------|-----------|-------------------|-------------|
| SUPP-01 | `nono agent demote <id>` drops IL of tenant's process to Low; does NOT reap | unit + live Win11 UAT | `cargo test -p nono-cli demote` | ❌ W0 |
| SUPP-01 | Demoting an unknown `tenant_id` returns a clear error | unit | `cargo test -p nono-cli demote_unknown_tenant` | ❌ W0 |
| SUPP-01 | Leak-limits paragraph present in `nono agent demote` verb/help output | static / unit | inspect help text | ❌ W0 |
| SUPP-02 | WFP filter added at launch (profile with network scoping) | unit (mock pipe) + live UAT | `cargo test -p nono-cli wfp_filter_add_at_launch` | ❌ W0 |
| SUPP-02 | WFP filter removed at reap | unit (mock pipe) | `cargo test -p nono-cli wfp_filter_remove_at_reap` | ❌ W0 |
| SUPP-02 | D-05: launch refuses + names missing service when wfp-service absent & profile network-scoped | unit | `cargo test -p nono-cli wfp_absent_fail_secure` | ❌ W0 |
| SUPP-02 | D-05: launch proceeds when wfp-service absent & profile has no network scoping | unit | `cargo test -p nono-cli wfp_absent_no_scoping_ok` | ❌ W0 |
| SUPP-02 | Per-agent isolation: two agents, different allowed domains, no cross-bleed (A1 gate) | live Win11 UAT | manual two-agent WFP test | N/A — UAT |
| SUPP-03a | `copilot-cli` profile present in policy.json (`windows_low_il_broker: true`, native PE, no `windows_interpreters`) | unit (policy parse) | `cargo test -p nono-cli copilot_cli_profile_present` | ❌ W0 |
| SUPP-03a | Copilot CLI confined end-to-end on real Win11 (SC3) | live Win11 UAT | manual: `nono agent launch --profile copilot-cli` | N/A — UAT |
| SUPP-03b | nono-ts `confinedRun` exported + callable on Windows | unit (nono-ts) | `cargo test --target x86_64-pc-windows-msvc` in `../nono-ts/` | ❌ W0 |
| SUPP-03b | nono-ts `confine` exported + callable on Windows | unit | same | ❌ W0 |
| SUPP-03b | nono-ts non-Windows stubs throw `"…is Windows-only"` | cross-target unit | `cargo test --target x86_64-unknown-linux-gnu` in `../nono-ts/` | ❌ W0 |
| SUPP-03b | nono-ts `confinedRun` confines a node/JS process on real Win11 (SC5) | live Win11 UAT | manual TS test calling `confinedRun` | N/A — UAT |
| SUPP-03b | Cross-target clippy (Linux + macOS) green on nono-ts | static | `cargo clippy --target x86_64-unknown-linux-gnu` in `../nono-ts/` | N/A — CI |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono-cli/` (inline or `tests/`): `wfp_filter_add_at_launch`, `wfp_filter_remove_at_reap`, `wfp_absent_fail_secure`, `wfp_absent_no_scoping_ok` — mock wfp-service pipe tests
- [ ] `control_loop.rs` (inline or `tests/`): `demote_returns_ok_for_known_tenant`, `demote_returns_err_for_unknown_tenant`
- [ ] policy parse test: `copilot_cli_profile_present`
- [ ] `../nono-ts/src/windows_confined_run.rs` — unit tests mirroring nono-py `test_find_nono_exe_*`
- [ ] `../nono-ts/src/lib.rs` — non-Windows stub test (`confined_run_windows_only_stub`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Copilot CLI confined end-to-end | SUPP-03 (SC3) | Requires real Win11 host + installed Copilot CLI + AppContainer launch | `nono agent launch --profile copilot-cli -- copilot ...`; confirm confinement + native-PE coverage |
| Per-agent WFP isolation (A1) | SUPP-02 (SC2) | WFP kernel enforcement only observable on real Win11 with two live AppContainer agents | Launch 2 agents with distinct allowed domains; confirm each can only reach its own domain |
| nono-ts `confinedRun` confines node/JS | SUPP-03 (SC5) | Broker-arm launch needs real host / dev-layout-or-signed `nono.exe` (R-B4 trust gate) | Run TS script calling `confinedRun`; confirm write/network denial |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 150s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
