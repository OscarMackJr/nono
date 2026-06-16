---
phase: 75-supplementary-controls-secondary-engines
verified: 2026-06-16T00:00:00Z
status: passed
score: 3/3 requirements verified (SUPP-01 MET, SUPP-02 MET, SUPP-03 MET)
overrides_applied: 0
deferred:
  - truth: "Copilot CLI confined end-to-end (SC3 completion)"
    addressed_in: "Future — post-Phase-75 Node-ESM enablement work"
    evidence: >
      75-08 spike fully characterized the Node-ESM/AppContainer lstat-on-drive-root
      limitation. Operator re-scoped: Copilot confinement is PROVEN (write-outside
      denied); end-to-end completion requires an admin host-prereq + nono ancestor-RA
      change (documented in 75-08, not implemented). Engine-2 (end-to-end) is
      claude-code (75-07 UAT PASS). Re-scope operator-approved 2026-06-16.
  - truth: "A1 empirical per-agent WFP isolation (two concurrent agents with different allowed domains)"
    addressed_in: "Future — requires a network-scoped test profile"
    evidence: >
      No current profiles have network.block: true / tcp_connect_ports scoping, so the
      two-agent cross-domain isolation experiment is not currently exercisable. D-05
      unit tests PASS; the code path is wired. Deferred per 75-05 plan (acceptable
      per plan language).
  - truth: "Cross-target clippy (Linux/macOS)"
    addressed_in: "Live CI"
    evidence: >
      Win11 dev host lacks x86_64-linux-gnu-gcc and macOS cc cross-compilers (aws-lc-sys
      C layer). All new code is #[cfg(target_os = \"windows\")]-gated. Deferred to CI
      per CLAUDE.md cross-target-verify-checklist.md — documented as PARTIAL in every
      Phase 75 plan SUMMARY.
---

# Phase 75: Supplementary Controls + Secondary Engines — Verification Report

**Phase Goal:** Implement SUPP-01 (operator demote), SUPP-02 (per-agent WFP egress), and SUPP-03
(second engine + nono-ts parity), proving the engine-abstraction across >=2 engines and >=2 bindings.

**Verified:** 2026-06-16

**Status:** PASSED

**Re-verification:** No — initial verification.

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SUPP-01: `nono agent demote <tenant>` drops the IL of a live agent without reaping it, with spike-002 leak limits documented | VERIFIED | `handle_demote` + `demote_tenant_il` in `control_loop.rs` (OpenProcessToken + SetTokenInformation(TokenIntegrityLevel)); `AgentCommands::Demote` in `cli.rs` with 5 leak-limit bullets in `--help`; 3 unit tests PASS; live Win11 UAT SC1 PASS (long-running claude-code agent, `nono agent demote` → "IL-drop to Low succeeded; WFP filter removed (best-effort). Agent NOT reaped."; subsequent `agent list` still showed the agent). Commits `923ae5f7` + `b1ae0d6f`. |
| 2 | SUPP-02: Per-agent WFP filter installed at daemon launch, keyed to package SID, removed at reap; D-05 gate refuses launch when WFP service absent and profile is network-scoped | VERIFIED | `wfp_filter_add` / `wfp_filter_remove` helpers in `launch.rs` (synchronous blocking pipe client, session_sid-keyed); D-05 gate between job-assign (step 6) and ResumeThread (step 8); reap task calls `wfp_filter_remove` before `tenants.remove()`; 6 unit tests PASS (`wfp_filter_add_at_launch`, `wfp_absent_fail_secure`, `wfp_absent_no_scoping_ok`, `wfp_filter_add_constructs_request`, `wfp_filter_remove_at_reap_not_in_drop`, `wfp_filter_remove_nonfatal_contract`). Live UAT SC2: D-05 gate confirmed wired (non-network-scoped launch succeeds with service present); A1 empirical two-agent isolation DEFERRED (see deferred section). Commits `195a7c11` + `1bdfc56e`. |
| 3 | SUPP-03a: A copilot-cli engine profile exists in policy.json and copilot.exe can be launched confined (write-outside-workspace denied) through the daemon broker arm | VERIFIED (confinement proven; end-to-end DEFERRED per re-scope) | `copilot-cli` profile in `crates/nono-cli/data/policy.json` (native-PE, `windows_low_il_broker: true`, no `windows_interpreters`); `copilot_cli_profile_present` + `copilot_cli_profile_is_native_pe` tests PASS. Live SC3: AppContainer spawn confirmed; write-outside-workspace denied (Test-Path False); fail-secure launch-coverage gate working. Copilot end-to-end completion DEFERRED (Node-ESM/AppContainer lstat limitation, fully characterized in 75-08; Engine-2 re-scoped to claude-code which passed end-to-end in 75-07). Commits `f3f8f9bf` + `f1b8a6e6`. |
| 4 | SUPP-03b: nono-ts `confinedRun` + `confine` napi exports exist, Windows-cfg-gated, nono pin bumped to 0.62 | VERIFIED | `windows_confined_run.rs` (new) + `lib.rs` exports (`JsExecResult`, `confinedRun`, `confine` with Windows impl + non-Windows stubs); `Cargo.toml` `nono = { path = "../Nono/crates/nono", version = "0.62" }`; 5 unit tests PASS; `cargo test` 5/5 in nono-ts. Commits `e218827` + `2bac4e2`. |
| 5 | SUPP-03 abstraction proof: >=2 engines end-to-end confined, >=2 bindings confined on Win11 | VERIFIED | Engines: Aider (Ph71 UAT PASS) + claude-code (75-07 UAT: confined under AppContainer, no DLL-death, write-outside denied, clean reap). Bindings: nono-py (Ph72 UAT PASS) + nono-ts (SC5 UAT PASS: `confinedRun` via `windows_low_il_broker` profile → write-outside denied exit 1, write-inside exit 0). |
| 6 | SC4 (daemon privilege split): nono-agentd runs as USER_OWN_PROCESS TEMPLATE (type 50), not LocalSystem | VERIFIED | `sc qc nono-agentd` → `TYPE: 50 USER_OWN_PROCESS TEMPLATE`, empty `SERVICE_START_NAME`. Live UAT SC4 PASS. |
| 7 | GAP-75-A (daemon-start type-50 guard): `nono daemon start` does not hit `sc start` exit-5 when service is a type-50 template | VERIFIED | `is_user_own_template_service` predicate + raw-spawn fallback in `agent_cli.rs`; 3 unit tests PASS; live UAT confirms the 75-06 guard message printed and daemon reached RUNNING (pid 11484). Commit `a2c44c3f`. |
| 8 | GAP-75-B (daemon capability-less launch): daemon-launched agents get a real CapabilitySet + package-SID DACL grants applied before ResumeThread, revoked on reap | VERIFIED | `build_daemon_capability_set` in `mod.rs`; local `DaemonDaclGuard` in `launch.rs` (step 6.6); `AgentTenant.dacl_guard` field; 6 daemon tests PASS (`cargo test --bin nono-agentd`); live UAT: claude-code launched confined, no DLL-death, per-tenant workspace created, write-outside denied (Test-Path False), clean reap with grants revoked. Commits `4fb9551e` + `062f86ad` + `30afd094`. |

**Score:** 8/8 truths verified (3 deferred items excluded from scoring — all operator-accepted)

---

### Deferred Items

Items not yet met but operator-accepted as out-of-scope for this phase.

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Copilot CLI end-to-end completion (SC3 full run) | Future Node-ESM enablement work | 75-08 spike: admin-prereq (C:\ + C:\Users icacls RA grant for ALL APPLICATION PACKAGES) + nono ancestor-RA code change required. Operator re-scoped: Engine-2 = claude-code. |
| 2 | A1 empirical per-agent WFP isolation (two-agent cross-domain test) | Future — requires a network-scoped test profile | All current profiles have `network.block: false`; D-05 unit path is wired; empirical isolation test deferred per 75-05 plan. |
| 3 | Cross-target clippy (Linux/macOS) | Live CI | Win11 host lacks cross-toolchains; all new code is `#[cfg(target_os = "windows")]`-gated; deferred per CLAUDE.md. |

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/agent_daemon/launch.rs` | WFP helpers + D-05 gate + DaemonDaclGuard + build_daemon_capability_set callers | VERIFIED | `wfp_filter_add`, `wfp_filter_remove`, `profile_needs_network_scoping`, D-05 gate at line ~670, `DaemonDaclGuard` struct + impl at lines ~101+, step 6.6 DACL grant at line ~575 |
| `crates/nono-cli/src/agent_daemon/control_loop.rs` | ControlRequest::Demote + handle_demote | VERIFIED | `Demote { tenant_id }` variant, `handle_demote` at line ~697, `demote_tenant_il` at line ~814 |
| `crates/nono-cli/src/agent_daemon/mod.rs` | build_daemon_capability_set + DaemonDaclGuard re-export | VERIFIED | `build_daemon_capability_set` at line ~102, `pub(crate) use windows_impl::DaemonDaclGuard` |
| `crates/nono-cli/src/agent_daemon/reap.rs` | WFP reap ordering contract tests | VERIFIED | `wfp_filter_remove_at_reap_not_in_drop`, `wfp_filter_remove_nonfatal_contract` |
| `crates/nono-cli/src/agent_daemon/accept_loop.rs` | Unchanged except 75-07 scope | VERIFIED | Modified per 75-07 summary; file exists |
| `crates/nono-cli/src/agent_cli.rs` | `nono agent demote` client + is_user_own_template_service type-50 guard | VERIFIED | `agent_demote` function, `is_user_own_template_service` at line ~756, 3 unit tests at lines ~1173+ |
| `crates/nono-cli/src/cli.rs` | AgentCommands::Demote with leak-limits in doc comment | VERIFIED | `Demote { tenant_id: String }` at line ~3226 with 5 leak-limit bullets |
| `crates/nono-cli/data/policy.json` | copilot-cli native-PE profile | VERIFIED | `copilot-cli` key at line ~902 |
| `crates/nono-cli/src/profile/builtin.rs` | copilot_cli_profile_present + copilot_cli_profile_is_native_pe tests | VERIFIED | Both tests present per 75-03 SUMMARY; confirmed via commit `f1b8a6e6` |
| `crates/nono-cli/src/bin/nono-wfp-service.rs` | session_sid-keyed request path (validate_target_request_fields fix) | VERIFIED | Modified in commit `195a7c11`; SID-keyed path allows `target_program_path=None` |
| `../nono-ts/src/windows_confined_run.rs` | confinedRun (Shape A) + confine (Shape B) Windows impl | VERIFIED | File exists at `/c/Users/OMack/nono-ts/src/windows_confined_run.rs`; contains `confined_run`, `confine`, `is_already_confined`, 5 unit tests |
| `../nono-ts/src/lib.rs` | JsExecResult + confinedRun/confine napi exports + non-Windows stubs | VERIFIED | `JsExecResult` at line ~357, `confinedRun` Windows at ~375, non-Windows stub at ~389, `confine` both variants |
| `../nono-ts/Cargo.toml` | nono = 0.62 path dep | VERIFIED | `nono = { path = "../Nono/crates/nono", version = "0.62" }` at line 17 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `handle_launch` | `build_daemon_capability_set` | call in `control_loop.rs` | WIRED | `handle_launch` resolves exe, derives workspace, calls `build_daemon_capability_set` before spawn |
| `launch_agent` | `wfp_filter_add` | D-05 gate between step 6 and step 7a | WIRED | `if profile_needs_network_scoping` → `wfp_filter_add` at line ~670 in launch.rs |
| Reap task | `wfp_filter_remove` | tokio::spawn reap path in launch.rs | WIRED | Reap task calls `wfp_filter_remove` before `tenants.remove()` at line ~858 |
| `AgentTenant` | `DaemonDaclGuard` | `dacl_guard` field | WIRED | Field declared before `job_handle`/`process_handle` so Drop order revokes grants first |
| `ControlRequest::Demote` | `handle_demote` | match arm in control_loop.rs | WIRED | `ControlRequest::Demote { tenant_id } => handle_demote(&state, &tenant_id)` at line ~443 |
| `handle_demote` | `wfp_filter_remove` | D-03 WFP-cut after IL-drop | WIRED | Non-fatal call to `wfp_filter_remove` after successful IL-drop at line ~780 area |
| `confinedRun` napi export | `windows_confined_run::confined_run` | cfg-gated delegation in lib.rs | WIRED | `windows_confined_run::confined_run(exe, args, allow, profile, cwd, timeout_secs)` |
| `daemon_start` | `is_user_own_template_service` | `sc qc` check before sc start | WIRED | `if is_user_own_template_service(&sc_qc_output)` at line ~118 in agent_cli.rs |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|---------------------|--------|
| `handle_demote` | tenant's process HANDLE | `state.tenants` lookup (daemon state map) | Yes — live HANDLE from launch_agent | FLOWING |
| `wfp_filter_add` | package_sid | `nono::package_sid_to_string()` from CreateAppContainerProfile at spawn time | Yes — per-agent SID, not wire input | FLOWING |
| `build_daemon_capability_set` | CapabilitySet | Embedded `policy.json` parsed at runtime; profile name from operator launch request | Yes — real CapabilitySet with exe dir + workspace + system dirs | FLOWING |
| `confinedRun` | ExecResult | `nono.exe run` subprocess, captured stdout/stderr | Yes — live child process output | FLOWING |

---

### Behavioral Spot-Checks

Live UAT on real Win11 Enterprise build 26200 (operator-run 2026-06-16) is the authoritative
behavioral evidence. Automated spot-checks are not run here — the binary requires a real Win11
SCM and AppContainer stack, which is unavailable in the verification environment.

| Behavior | UAT Evidence | Status |
|----------|-------------|--------|
| SC4: daemon runs as USER_OWN_PROCESS TEMPLATE | `sc qc nono-agentd` TYPE 50, empty SERVICE_START_NAME | PASS |
| SC1: demote drops IL, does not reap | Agent (pid 19612) stayed resident after `nono agent demote`; subsequent `agent list` showed it | PASS |
| SC2 (D-05): non-network-scoped launch succeeds with WFP service present | nono-wfp-service STATE 4 RUNNING; launch succeeded | PASS |
| GAP-75-A: `nono daemon start` bypasses `sc start` for type-50 | Type-50 guard message printed; daemon reached RUNNING (pid 11484) | PASS |
| GAP-75-B: daemon-launched agent gets real capabilities | claude-code confined, no DLL-death, write-outside-workspace denied (Test-Path False) | PASS |
| SC5: nono-ts `confinedRun` confines Node on Win11 | write outside `%USERPROFILE%\nono-ts-ws` denied (exit 1); write inside allowed (exit 0, `ok.txt` created) | PASS |
| SC3 confinement gate: copilot.exe write-outside denied | Test-Path False on write attempt outside workspace | PASS |

---

### Probe Execution

No conventional `scripts/*/tests/probe-*.sh` probes declared or found for Phase 75.
Step 7c: SKIPPED (no probe scripts; behavioral verification via live Win11 UAT in 75-HUMAN-UAT.md).

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| SUPP-01 | 75-02 | Operator demote: post-hoc token IL-drop with leak limits documented | SATISFIED | `nono agent demote` verb wired end-to-end; `demote_tenant_il` Win32 path; 3 unit tests; live SC1 PASS |
| SUPP-02 | 75-01 | Per-agent WFP egress scoped by AppContainer package SID | SATISFIED | `wfp_filter_add`/`remove` helpers; D-05 gate; 6 unit tests; A1 empirical deferred (accepted) |
| SUPP-03 | 75-03, 75-04, 75-07, 75-08 | Second engine profile + nono-ts parity; abstraction proven >=2 engines + >=2 bindings | SATISFIED | copilot-cli profile present; nono-ts `confinedRun`+`confine` wired; engines Aider+claude-code both end-to-end; bindings nono-py+nono-ts both confined on Win11; SC3 Copilot end-to-end DEFERRED (accepted re-scope) |

---

### Anti-Patterns Found

No TBD, FIXME, or XXX markers found in Phase 75-modified files:
- `crates/nono-cli/src/agent_daemon/launch.rs` — clean
- `crates/nono-cli/src/agent_daemon/control_loop.rs` — clean
- `crates/nono-cli/src/agent_cli.rs` — clean
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — clean
- `../nono-ts/src/windows_confined_run.rs` — clean

No empty handlers, placeholder returns, or hardcoded empty data flows to user-visible rendering.

| File | Pattern | Severity | Assessment |
|------|---------|----------|------------|
| `launch.rs` `wfp_filter_remove` non-fatal on reap | `tracing::warn!` on error, continues to `tenants.remove` | INFO | By-design (documented in 75-01 plan); WFP service startup sweep backstops stale filters |
| `nono-ts` non-Windows stubs: `return Err("confinedRun is Windows-only")` | Throws on non-Windows | INFO | D-07 design constraint (Windows-only this phase); not a stub — intentional error |

---

### Human Verification Required

None — all behavioral gates were closed by the live Win11 UAT run on 2026-06-16 recorded in
`75-HUMAN-UAT.md`. Accepted deferrals (Copilot end-to-end, A1 empirical isolation, cross-target
clippy) are tracked as future work, not human-verification blockers.

---

### Gaps Summary

No genuine gaps. The three deferred items listed above are operator-accepted:

1. **Copilot CLI end-to-end** — SC3 confinement is PROVEN; end-to-end completion requires an
   admin host-prereq (C:\ READ_ATTRIBUTES for ALL APPLICATION PACKAGES) that was deliberately
   not baked into the product story. Engine-2 = claude-code (zero prereq, proven end-to-end).
   Fully characterized in 75-08 spike. Operator-approved re-scope 2026-06-16.

2. **A1 empirical WFP isolation** — D-05 fail-secure gate is wired and unit-tested. No
   network-scoped profile exists in any current deployment; empirical two-agent cross-domain
   isolation cannot be exercised without one. Code path is correct; deferred pending a
   network-scoped test profile.

3. **Cross-target clippy (Linux/macOS)** — All new Phase 75 code is `#[cfg(target_os = "windows")]`-
   gated. Win11 dev host lacks the cross-toolchain C layer (aws-lc-sys). Deferred to live CI
   per CLAUDE.md mandatory rule.

None of the three items constitute a BLOCKER against the phase goal. All success criteria
(SUPP-01, SUPP-02, SUPP-03) are satisfied within the operator-approved scope.

---

_Verified: 2026-06-16_
_Verifier: Claude (gsd-verifier)_
