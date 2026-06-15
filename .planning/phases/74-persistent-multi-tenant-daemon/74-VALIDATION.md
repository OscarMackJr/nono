---
phase: 74
slug: persistent-multi-tenant-daemon
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-14
---

# Phase 74 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `74-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | None — workspace-default |
| **Quick run command** | `cargo test -p nono-cli --lib` (unit tests, no broker/host needed) |
| **Full suite command** | `cargo test -p nono-cli` (integration tests; host-gated ones behind `NONO_DAEMON_INTEGRATION_TESTS=1` require a real Win11 host) |
| **Estimated runtime** | ~30s quick; integration suite host-gated |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli --lib` (unit only; < 30s)
- **After every plan wave:** Run `cargo test -p nono-cli` (includes integration; broker-arm tests host-gated)
- **Before `/gsd:verify-work`:** All SC1–SC5 green, including Win11 host UAT (concurrent agents + handle baseline)
- **Max feedback latency:** 30 seconds (unit tier)

---

## Per-Task Verification Map

| Req / SC | Behavior | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|----------|----------|-----------|-----------------|-----------|-------------------|-------------|--------|
| DMON-01 / SC1 | 2 concurrent confined agents, each served independently over one pipe, each scoped to own SID | T-tenant-isolation | Fresh token + fresh job per agent; no SID/workspace/WFP reuse | integration (Win11 host) | `NONO_DAEMON_INTEGRATION_TESTS=1 cargo test -p nono-cli daemon_concurrent_agents` | ❌ W0 | ⬜ pending |
| DMON-01 / SC3 | 100-agent launch/exit returns handle/job count to baseline (no leak) | T-handle-leak | Deterministic `Drop` reap closes every per-agent handle | integration (Win11 host, spike) | `NONO_DAEMON_INTEGRATION_TESTS=1 cargo test -p nono-cli daemon_handle_baseline` | ❌ W0 | ⬜ pending |
| DMON-02 / SC2 | Cross-tenant-denial: tenant B denied tenant A's grants | T-cross-tenant-theft | Per-tenant SDDL + impersonation + registry SID match; fail-secure deny | integration (in-process impersonation) | `cargo test -p nono-cli daemon_cross_tenant_denial` | ❌ W0 | ⬜ pending |
| DMON-03 / SC4 | Privilege-model ADR exists; daemon runs as USER not SYSTEM; split from WFP service | T-privilege-pivot | Least-privilege USER; query-only pipe (no escape hatch) | manual + doc check | Review `proj/ADR-74-privilege-model.md`; `sc qc nono-agentd` (manual) | ❌ W0 | ⬜ pending |
| DMON-01 / SC5 | Wire protocol reuses `session_id`; no net-new field in `SupervisorMessage` | T-protocol-spoof | `agent_id`/`session_id` = untrusted routing hint, never authz | unit | `cargo test -p nono supervisor_message_no_tenant_id_field` | ❌ W1 | ⬜ pending |
| DMON-02 | `ImpersonateNamedPipeClient` present in accept loop; `RevertToSelf` on all paths | T-identity-leak | RAII guard reverts impersonation on every path incl. error | unit (mock pipe) | `cargo test -p nono-cli authenticate_pipe_client_reverts_on_error` | ❌ W1 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `proj/ADR-74-privilege-model.md` — privilege-model ADR (MUST be first; SC4 ordering gate — ADR before service host is coded)
- [ ] Spike harness `crates/nono-cli/tests/daemon_handle_baseline.rs` — 100-agent reap + fresh-token isolation proof + cross-tenant denial (in-process impersonation variant)
- [ ] `crates/nono-cli/tests/daemon_cross_tenant_denial.rs` — or folded into the spike harness file

*The three Wave-0 spike clauses (fresh-token isolation, deterministic reap, cross-tenant denial) MUST pass on a real Win11 host before Wave 1 daemon-binary implementation begins.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| 2 concurrent confined agents served independently | DMON-01 / SC1 | Requires real Win11 host + broker trust gate (dev-layout/signed `nono.exe`) | Launch two agents via `nono agent launch`; confirm each gets a distinct package SID and confined FS posture |
| 100-agent handle-count-returns-to-baseline | DMON-01 / SC3 | Real-host Win32 handle accounting; reuse not observable in CI unit tier | Run host-gated reap test; compare process handle/job counts to pre-run baseline |
| Daemon runs as USER not SYSTEM | DMON-03 / SC4 | SCM account inspection | `sc qc nono-agentd` — confirm account is the user, not LocalSystem; confirm split from `nono-wfp-service` |

---

## Validation Sign-Off

- [ ] All SC1–SC5 have an `<automated>` verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (ADR, spike harness, cross-tenant test)
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s (unit tier)
- [ ] `nyquist_compliant: true` set in frontmatter once Wave 0 stubs exist

**Approval:** pending
