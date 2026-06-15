---
gsd_state_version: 1.0
milestone: v2.12
milestone_name: AI Agent Abstraction
status: executing
last_updated: "2026-06-15T12:45:41.023Z"
last_activity: 2026-06-15
progress:
  total_phases: 5
  completed_phases: 3
  total_plans: 18
  completed_plans: 15
  percent: 83
---

# Project State: nono — v2.12 AI Agent Abstraction

## Project Reference

See: `.planning/PROJECT.md` (v2.12 milestone started 2026-06-13; v2.11 Phases 68/69/70 complete, Phase 67 host-gated carry-forward). Phase numbering continues from Phase 70 (Phases 71-75 — NOT reset to 1).

**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms — and that confinement must apply to *any* AI agent engine, not just Claude Code.

**Current Focus:** Phase 74 — persistent-multi-tenant-daemon

## Current Position

Phase: 74 (persistent-multi-tenant-daemon) — EXECUTING
Plan: 3 of 6 — COMPLETE
Status: Ready to execute
Last activity: 2026-06-15

### v2.12 Phase Summary (active)

| Phase | Goal | Requirements | SC | Status | Host gate |
|-------|------|--------------|----|--------|-----------|
| 71 | Engine-Agnostic Launch Productionization — parent-and-confine any covered engine (Aider + LangChain-Python) through one engine-neutral path; de-spike the validated 003 path; fail-secure coverage + R-B3 diagnostic | ENG-01, ENG-02, ENG-03 | 5 | ⬜ Not started | Real Win11 host (Aider end-to-end) |
| 72 | nono-py Binding + In-Process-Exec Proof — confine a real LangChain agent with NO Claude hook via `confined_run` (Shape A) + `confine` (Shape B); document the E1-E5 contract | ABI-01, ABI-02 | 5 | ✅ Complete (2026-06-14) | Win11 host w/ Python; nono-py build |
| 73 | AI_AGENT Marker — unforgeable spawn-time token SID (not a named job); deny breakaway; daemon-only job ACL; classify arbitrary PID | MARK-01 | 5 | 🔶 Code-complete (UAT pending) | Win11 host |
| 74 | Persistent Multi-Tenant Daemon (RISKIEST — former spike 004) — least-priv USER daemon, multi-client tenant-isolated pipe, fresh token+job per agent, deterministic reap | DMON-01, DMON-02, DMON-03 | 5 | 🔵 In-progress (Plan 01 at checkpoint) | Win11 host; **research-flag 74** |
| 75 | Supplementary Controls + Secondary Engines — demote (demote-only), per-agent WFP egress, Copilot CLI profile, nono-ts parity | SUPP-01, SUPP-02, SUPP-03 | 5 | ⬜ Not started | Win11 host; node/nono-ts build |

**Dependencies:** **71 FIRST** (foundation — everything sits on top). **72 ∥ 73** (parallel — both depend only on 71). **74** depends on 71 (working single-launch path — HARD GATE) + 73 (marker). **75** depends on 74 (demote/WFP are daemon-keyed) + 72 (nono-ts mirrors nono-py). The daemon (74) MUST NOT precede a solid single-launch path — that ordering IS the quality gate.

### Research / spike flags (v2.12)

| Phase | Flag | Notes |
|-------|------|-------|
| 74 | `/gsd:plan-phase 74 --research-phase 74` | Two unspiked/net-new mechanisms: (a) **token/job reuse-vs-fresh across many tenants** — the explicitly UNSPIKED part of spike 003/004, the milestone's highest-risk unknown; scope a spike INSIDE the phase gated on fresh-token isolation + deterministic reap + cross-tenant denial. (b) whether server-side **`ImpersonateNamedPipeClient`** (NOT in `socket_windows.rs` today — it verifies the *server* PID from the client side) composes with the existing Low-IL/AppContainer cap-pipe SDDL DACL handshake. Privilege-model ADR written BEFORE coding the service host. |
| 72 | (optional, in-phase validation) | Shape B (`Sandbox::apply` on the CURRENT process at startup) is a usage pattern the bindings have not exercised; soundness boundary = must precede any privileged handle open. |
| 71, 73, 75 | skip `--research-phase` | Standard patterns: spike 003 VALIDATED (71); named-job Win32 documented + unnamed-job lifecycle already in `exec_strategy_windows/` (73); demote (spike 002) + WFP (Phase 62) + node-engine profiles (Claude) all proven shapes (75). |

### Host-availability gates (v2.12)

| Phase | Gate | Notes |
|-------|------|-------|
| 71 | Real Win11 host | Aider confined end-to-end; the broker-arm launch (`windows_low_il_broker:true`) only works from a real host. Re-assert: AppContainer SID needs `CreateAppContainerProfile` (not derive-only → `ERROR_FILE_NOT_FOUND`); preserve `SystemRoot`/`windir`/`SystemDrive` env baseline (else CLR `0xFFFF0000`). |
| 72 | Win11 host + Python; nono-py build | LangChain `PythonREPLTool` exec() write-deny test; internal nono pin bump 0.57.0 → 0.62.x. |
| 74 | Win11 host | 2 concurrent agents over one pipe; cross-tenant-denial negative test; 100-agent launch/exit handle-baseline test. |
| 75 | Win11 host + node; nono-ts build | Copilot CLI profile (node engine); nono-ts pin bump 0.33.0 → 0.62.x (napi 2 kept). |

<details>
<summary>v2.11 Phase Summary (Phases 67-70; 68/69/70 complete, 67 host-gated — archived at `milestones/v2.11-ROADMAP.md`)</summary>

| Phase | Goal | Requirements | Status |
|-------|------|--------------|--------|
| 67 | Clean-Host Windows Install (MSI VC++ handled, service-start non-fatal, interim broker-trust helper) | DIST-01, DIST-02, TRUST-01, TRUST-02 | ⬜ Host-gated UAT pending (clean Win11) |
| 68 | macOS Resource-Limit Enforcement Fix (`--timeout` + `--max-processes` fire on real macOS) | RESL-MAC-01, RESL-MAC-02 | ✅ Complete 2026-06-12 |
| 69 | UPST8 Audit (DIVERGENCE-LEDGER `v0.60.0..v0.62.0` non-macOS) | UPST8-01 | ✅ Complete 2026-06-13 |
| 70 | UPST8 Cherry-pick Sync (5 will-sync commits absorbed) | UPST8-02 | ✅ Complete 2026-06-13 |

</details>

## Key Decisions

### v2.12 decisions

| Decision | Phase | Rationale |
|----------|-------|-----------|
| Phase order is dependency-driven A→(B∥C)→D→E (= 71→(72∥73)→74→75) | all | Research SUMMARY § Implications: launch productionization is the foundation everything consumes; daemon is launch+marker+pipe+unspiked-reuse-risk and cannot start until 71 is a gated working path + 73 exists. Quality gate enforced structurally. |
| ENG-01/02/03 all in Phase 71 (one engine-agnostic launch phase) | 71 | They form one coherent capability (the productionized launch-and-confine path + profiles + fail-secure gates); splitting them would create partial-feature boundaries. |
| Daemon (74) is the riskiest piece and is research-flagged | 74 | Token/job reuse across tenants is the one genuinely unspiked mechanism (former spike 004); `ImpersonateNamedPipeClient` is net-new to `socket_windows.rs`. Spike-inside-the-phase. |
| Demote (SUPP-01) is demote-only, NOT a standalone boundary | 75 | Spike 002 PARTIAL: post-hoc IL-drop is leaky; it stays an IR lever layered on a proven launch-time default — never the "detect-and-confine-as-primary" anti-feature. |
| AI_AGENT marker = unforgeable token SID, NOT a named job | 73 | Pitfall 2: a named job is openable by name (rendezvous, not secret); env/argv markers are forgeable. Named job is kill-group/enumeration/resource-caps only; the token SID is the authorization signal. |
| Least-privilege USER daemon SPLIT from the elevated WFP service; privilege-model ADR FIRST | 74 | Pitfall 4: a SYSTEM-level always-on launcher gives an escaped agent a pivot to all tenants + the host. ADR written before the service host is coded. |
| Composition over green-field: no new wire protocol, windows-sys 0.59 / pyo3 0.28 / napi 2 kept | all | Research STACK.md: every new dep already lives in-tree pinned; deltas are features + net-new Win32 (named job objects). Deliberate non-bumps avoid gratuitous cross-target-drift churn and a napi-3 scope balloon. |
| A2: TokenAppContainerSid = 31i32 in windows-sys 0.59 (not 56 as noted in RESEARCH.md) | 74 Plan 01 | Confirmed from windows-sys-0.59.0/src/Windows/Win32/Security/mod.rs at compile time. |
| A6: broker trust gate checks nono.exe CALLER path + Authenticode; broker binary must match | 74 Plan 01 | Code read of launch.rs is_dev_build_layout() + verify_broker_authenticode(); daemon binary (nono-agentd.exe) needs same trust-gate treatment as nono.exe in production. |
| authenticate_pipe_client is pub unsafe fn (not pub(crate)) — daemon accept loop in nono-cli calls it cross-crate | 74 Plan 02 | `pub(crate)` cannot be re-exported from a `pub mod`; the daemon binary in nono-cli requires `pub` for cross-crate access. `unsafe fn` is semantically correct for a raw-HANDLE parameter API per clippy `not_unsafe_ptr_arg_deref`. |
| ImpersonationGuard RAII guarantees RevertToSelf on ALL exit paths (including panic) | 74 Plan 02 | STRIDE T-74-02-01 (EoP) mitigation: Drop calls RevertToSelf unconditionally; even when not impersonating (safe no-op). Full end-to-end test in daemon_handle_baseline.rs (nono-cli); unit test verifies RAII mechanism without requiring SeImpersonatePrivilege. |

<details>
<summary>v2.11 decisions (archived)</summary>

See `.planning/milestones/v2.11-ROADMAP.md`. Key: DIST+TRUST kept together in Phase 67; real Azure Trusted Signing OUT OF SCOPE (cert-gated → enterprise milestone); macOS resl fix is a real supervisor/setrlimit bug fix (D-01 baseline+N RLIMIT_NPROC, D-04 setpgid); UPST8 scoped to non-macOS slice, D-70-01 extended range to v0.62.0.

</details>

### Key Decisions (carried from v1.0 — still load-bearing for v2.12)

- **Supervisor-Broker Pattern:** the only way to manage elevated tasks (WFP) while keeping a user-level CLI; the daemon (74) reuses this split.
- **WFP as Primary Network Backend:** kernel-level enforcement; SUPP-02 per-agent egress keys on it via the elevated `nono-wfp-service`.
- **Named Job Objects:** agent lifecycle (atomic stop/list) — but for v2.12 the marker's authorization signal is the token SID, NOT the job name (see v2.12 decisions).
- **SID-Based Filtering:** child processes inherit network restrictions; per-tenant SID is the v2.12 isolation primitive.
- **Restricted Tokens / Low-IL broker arm (`windows_low_il_broker:true`):** the validated confining primitive the launcher uses unchanged.

## Accumulated Context

### Constraints active this milestone

- **User-mode only — no kernel driver / minifilter / `PsSetCreateProcessNotifyRoutine`** (ADR-65 No-go). Composition over existing subsystems.
- **Isolation ≥ the per-invocation `nono run` model** — `NO_WRITE_UP` write-deny + deny-network-unless-granted must be preserved by every new path (launcher, daemon, binding).
- **Repo MUST stay PUBLIC** until Microsoft approves the minifilter altitude — verify no `build_notes/`/`.gsd/` staged before any `git push` (the "go-private" commit `74a47742` was cancelled 2026-06-11).
- **Cross-target clippy (Linux + macOS) is a MUST** per CLAUDE.md for any cfg-gated Unix code touched; Windows dev host can't cross-compile (ring/aws-lc-sys C-toolchain) → CI is the load-bearing signal. (The binding work in 72/75 is mostly Windows-surfaced, but scan any `cfg`-gated changes.)

### Pitfall guards carried into v2.12 (per research PITFALLS.md)

The per-invocation pitfalls carry forward UNCHANGED (post-hoc IL-drop demote-only, R-B3 user-owned workspace, exe-coverage gate, absolute grants, broker dev-layout/signing). Each NEW pitfall is owned by exactly one phase as a success-criterion-with-negative-test:

| Pitfall | Owner | Bake-in |
|---------|-------|---------|
| P6 nested-job collisions / silent confinement loss | 71 | spawn suspended, assign before any code runs, fail-secure on assign failure, no UI limits |
| R-B3 user-owned workspace + exe-coverage fail-secure | 71 | fail-secure coverage gate + R-B3 ownership diagnostic |
| P3 in-process-exec() cannot be confined post-hoc | 72 | `confine()` self-confine at startup before any privileged handle |
| P2 AI_AGENT marker forge/shed | 73 | unforgeable token SID (not a named job); deny breakaway; daemon-only job ACL |
| P1 cross-tenant capability theft (load-bearing) | 74 | server-side `ImpersonateNamedPipeClient` + per-tenant SID; cross-tenant-denial negative test |
| P4 daemon attack surface / privilege | 74 | least-priv USER daemon split from elevated WFP service; privilege-model ADR first |
| P5 token & job-object handle lifetime | 74 | fresh token+job per agent; deterministic reap; 100-agent handle-baseline test |
| "detect-and-confine as primary model" anti-feature | 75 | demote stays an IR lever, never the boundary |

### Re-assert per phase (banked AppContainer/CLR gotchas)

- **AppContainer per-agent SID:** `CreateAppContainerProfile`, NOT derive-only (else `CreateProcessW` `ERROR_FILE_NOT_FOUND`). (memory `windows_appcontainer_wfp_validated`)
- **Env baseline for CLR/PowerShell children:** preserve `SystemRoot`/`windir`/`SystemDrive` (else CLR `0xFFFF0000`). (memory `windows_hook_interpreter_spawn_gotchas`)
- **Cap-pipe DACL handshake:** the Low-IL/AppContainer rendezvous (package-SID READ grant before the blocking `ConnectNamedPipe`; child learns the pipe via `NONO_SUPERVISOR_PIPE`). (memory `windows_appcontainer_cap_pipe_reachability`)

## Deferred Items

### v2.11 carry-forward (open)

- **Phase 67 clean-host Windows install** (DIST-01/02, TRUST-01/02) — host-gated UAT pending a clean Win11 host (no VC++, no pre-trusted cert); production-signed MSI, not dev-layout. Independent of v2.12 work; can be exercised when a clean host is available.

### v2 (deferred from v2.12 scope — not yet milestone-scoped)

- **Signed-policy / decentralized attestation** (SEED-005 / R-T1) — X-Large; its own milestone.
- **Sound adoption of an already-running agent** nono did not launch — blocked by the post-hoc-IL-drop leak (spike 002); needs a different mechanism.
- **Cursor native-Windows confinement** — Cursor's agent CLI is Linux/macOS/WSL-only (engine limitation, not nono's).

### Historical (acknowledged at prior closes — see git history / MILESTONES.md)

Prior-close audit-open backlogs (v2.10: 65 items; v2.9/v2.8: 55; v2.7: 45) — mostly pre-v2.5 `missing` quick-task slugs + historical UAT/verification bookkeeping. Carried, none blocking. Detail in MILESTONES.md per-milestone close notes.

## Session Continuity

**Last session:** 2026-06-15T12:45:41.008Z

**Phase 74 Plan 01 executed (2026-06-15):** Wave 0 — ADR + spike harness. `proj/ADR-74-privilege-model.md` committed first (SC4 ordering gate; 369a7c45). `crates/nono-cli/tests/daemon_handle_baseline.rs` committed with 4 test functions (d9788fa0). Harness compiles cleanly on Windows host. AWAITING human checkpoint "approved + spike green" before Wave 1. A2 answered: `TokenAppContainerSid = 31i32` in windows-sys 0.59. A6 answered: trust gate checks CALLER (nono.exe) not broker. A1 pending spike run.

**v2.12 roadmap complete (2026-06-13):** Phases 71-75 defined, 12/12 reqs mapped (100% coverage, no orphans, no duplicates). ROADMAP.md + REQUIREMENTS.md traceability + STATE.md updated. Build order is dependency-driven: 71 (foundation) → (72 ∥ 73 parallel) → 74 (riskiest daemon; hard-gated behind working 71 + 73; research-flagged) → 75 (supplementary). Composition milestone over broker-arm launch + `socket_windows.rs` cap pipe + `nono-wfp-service` shape; user-mode only (ADR-65 No-go); isolation ≥ `nono run`.

**Predecessor context (carried):** v2.11 Phases 68/69/70 complete (macOS resl fix shipped; UPST8 audited + synced to v0.62.0). Phase 67 (clean-host Win install) host-gated, carries forward. v2.10 shipped tag `v2.10` 2026-06-11; ADR-65 Accepted (No-go/Conditional-go on the kernel driver — the constraint anchoring v2.12's user-mode-only rule). Repo STAYS PUBLIC.

## Operator Next Steps

- **IMMEDIATE:** Run `NONO_DAEMON_INTEGRATION_TESTS=1 cargo test -p nono-cli daemon_handle_baseline -- --nocapture` on a real Win11 host with dev-layout nono.exe. Type "approved + spike green" when all 4 clauses pass, or report failures for replanning.
- After spike green: `/gsd:execute-phase 74` will continue with Plan 74-02 (daemon binary skeleton) as Wave 1.
- `/gsd:plan-phase 71` — engine-agnostic launch productionization (the FOUNDATION; spike-003 VALIDATED, skip `--research-phase`). Needs a real Win11 host for the Aider end-to-end gate.
- `/gsd:plan-phase 72` and `/gsd:plan-phase 73` — parallel-safe once 71 lands (binding proof ∥ marker; both standard-pattern).
- `/gsd:plan-phase 75` — supplementary controls + Copilot profile + nono-ts parity (proven shapes).
- Before any push: confirm no `build_notes/`/`.gsd/` staged — repo stays PUBLIC pending Microsoft minifilter-altitude approval.
