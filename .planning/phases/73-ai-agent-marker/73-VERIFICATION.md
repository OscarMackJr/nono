---
phase: 73
slug: ai-agent-marker
status: passed
requirement: MARK-01
verified: 2026-06-16
verifier: gsd-execute-phase milestone-audit gap-closure (operator-run host-gated SC4)
---

# Phase 73 — AI_AGENT Marker — Verification (MARK-01)

**Status: PASSED.** Verified 2026-06-16 during the v2.12 milestone-audit gap-closure (the phase was
code-complete since 2026-06-14 but its verification gate had never been closed — no VERIFICATION.md,
VALIDATION draft, SC4 `#[ignore]` tests never run). All success criteria now have passing evidence.

## Requirement

**MARK-01** — Each confined agent carries an unforgeable `AI_AGENT` identity bound to its
daemon-minted token SID; a non-agent process cannot claim the identity and a confined agent cannot
shed it. (Named job objects are kill-group/enumeration only — never authorization.)

## Scope (settled by 73-03-PLAN, not re-litigated)

The authoritative AI_AGENT predicate is the **in-process/in-daemon `AgentRegistry`** (the agent's
per-run AppContainer package SID is minted at spawn and inserted into the registry by the trusted
launcher; the confined Low-IL child cannot reach the launcher's memory to forge it). Standalone
cross-process `nono classify <pid>` is **structural/non-authoritative by design** (its registry is
empty); registry-backed cross-process authoritative classification is explicitly Phase 74+ daemon
scope. Therefore the `sc4_classify_*` in-process integration tests are the load-bearing MARK-01
proof, and the absence of a cross-process Classify verb is intended, not a gap.

## Success criteria — evidence

| SC | Criterion | Evidence | Verdict |
|----|-----------|----------|---------|
| SC1 | Launched agent's package SID inserted into AgentRegistry on the shipping path | `execution_runtime.rs` mint→`registry.insert` (`AgentRegistry` ×3, `#[cfg(windows)]`); daemon path inserts at `launch.rs:745`, removes at reap `launch.rs:846`; proven behaviorally by sc4_classify_real_agent | ✅ MET |
| SC2 | Non-agent / nonexistent PID → NotAnAgent | `cargo test -p nono classify_` → **8 passed** (incl. classify_current_process_not_agent, classify_nonexistent_pid_not_agent) | ✅ MET |
| SC3 | Job never sets `JOB_OBJECT_LIMIT_BREAKAWAY_OK`; job SD denies Low-IL | `job_never_has_breakaway_ok` + `job_security_descriptor_denies_low_il` → **2 passed** | ✅ MET |
| SC4 | In-process integration: real confined child → AiAgent; spoof AppContainer (not in registry) → NotAnAgent | `cargo test -p nono-cli --bin nono -- --ignored sc4_classify_real_agent sc4_classify_spoof_not_agent` → **2 passed** on real Win11 26200, dev-layout broker (2026-06-16) | ✅ MET (authoritative) |
| SC5 | Adopted-agent best-effort + standalone-classify-non-authoritative documented | `proj/DESIGN-engine-abstraction.md`: adopted=5, non-authoritative=4 matches | ✅ MET |

## Quality gates

- `cargo clippy -p nono -- -D warnings -D clippy::unwrap_used` → 0 warnings.
- `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used` → 0 warnings.
- Cross-target clippy (Linux/macOS): PARTIAL — deferred to CI per CLAUDE.md cross-target-verify-checklist.

## Unforgeability argument (MARK-01 core)

- **Bound to daemon-minted SID:** the registry key is the per-run AppContainer package SID minted by
  the launcher at `BrokerLaunchNoPty`/daemon spawn — `sc4_classify_real_agent` confirms a registry-
  inserted child classifies as `AiAgent { package_sid }`.
- **Non-agent cannot claim it:** `sc4_classify_spoof_not_agent` spawns a *self-made* AppContainer
  (structurally identical) NOT in the registry → `NotAnAgent`. Registry membership, not structure, is
  the predicate, so a process cannot forge the identity by mimicking the AppContainer shape.
- **Cannot shed it:** the SID is the token's AppContainer SID, fixed for the process lifetime; the
  confined child cannot alter the launcher-held registry (separate Low-IL process, no access to
  launcher memory — T-73-13).

## Known limitations (documented, non-blocking)

- No authoritative cross-process classify path (by design — Phase 74+ daemon scope; standalone
  `nono classify` is structural/non-authoritative with the disclaimer on every output path).
- Post-demote (SUPP-01) does not eagerly `agent_registry.remove`; the entry clears at reap. No
  exploitable surface today (control pipe has no Classify verb). Tracked as a milestone-audit warning.

## Verdict

**MARK-01: SATISFIED.** Phase 73 verification gate closed.
