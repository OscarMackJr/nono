# Phase 31: Broker-Process Architecture (SHELL-01) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-08
**Phase:** 31-broker-process-architecture-shell-01
**Areas discussed:** ConPTY ownership architecture, Broker binary placement + token-helper lift, Phase 31 scope boundary, Failure-mode response if A2 fails

---

## ConPTY ownership architecture

### Q1 → D-01: HPCON allocation and cross-boundary handling

| Option | Description | Selected |
|--------|-------------|----------|
| nono.exe owns HPCON; broker inherits (RESEARCH preferred) | nono.exe calls CreatePseudoConsole, spawns broker WITHOUT PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE — broker just inherits console handles. Broker spawns Low-IL child also WITHOUT the attribute. Mirrors PoC plain-inheritance shape exactly. Sidesteps A2. | ✓ |
| Broker owns HPCON from inherited pipe handles | nono.exe creates raw pipes only; broker calls CreatePseudoConsole itself; child spawned with PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE. Restructures pty_proxy::open_pty(). Adds ~1.5d. | |
| Broker spawns child with PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE pointing at nono's HPCON | Path the PoC explicitly did NOT test (RESEARCH §2c, Open Q1). Highest empirical risk: may re-trigger ConClntInitialize at Low-IL. | |

**User's choice:** nono.exe owns HPCON; broker inherits (RESEARCH preferred)
**Notes:** PoC pattern preserved verbatim; A2 untested but unused on this path.

### Q2 → D-02: Handle-inheritance discipline

| Option | Description | Selected |
|--------|-------------|----------|
| PROC_THREAD_ATTRIBUTE_HANDLE_LIST on every CreateProcess* call (Recommended) | Both nono→broker and broker→child spawns explicitly list ONLY the inherited console + ConPTY handles. Capability pipe + audit handles never inheritable past nono.exe. ~30 LOC; eliminates handle-leak class. | ✓ |
| bInheritHandles=FALSE on broker spawn; rebuild what broker needs via duplicate | More complex; little additional security beyond (a). | |
| Plain bInheritHandles=TRUE, no HANDLE_LIST (PoC shape) | Easy to validate but Low-IL child would inherit any inheritable HANDLE in nono.exe at spawn time. Fragile. | |

**User's choice:** PROC_THREAD_ATTRIBUTE_HANDLE_LIST on every CreateProcess* call
**Notes:** Capability pipe (Phase 11) + audit ledger handles never inheritable past nono.exe.

### Q3 → D-03: Broker lifetime model

| Option | Description | Selected |
|--------|-------------|----------|
| Broker waits for child; forwards exit code (RESEARCH §3d preferred) | Broker calls WaitForSingleObject, ExitProcess(child_exit_code). nono.exe monitors broker PID via existing WindowsSupervisedChild. Simple: no new IPC. | ✓ |
| Broker suspends child + signals nono; nono takes over; broker exits | Cleaner final supervision tree (nono→child) but new IPC surface. | |
| Broker stays alive AND nono monitors child via duplicated handle | Belt-and-braces but redundant exit-code reporting; ambiguous audit-ledger emission paths. | |

**User's choice:** Broker waits for child; forwards exit code
**Notes:** Broker is a thin shim; nono.exe monitors broker PID via existing WindowsSupervisedChild.

### Q4 → D-04: Job Object containment

| Option | Description | Selected |
|--------|-------------|----------|
| nono assigns broker to Job; rely on auto-inheritance for child (Recommended) | Standard Win32: child inherits Job membership automatically. JOB_OBJECT_LIMIT_*BREAKAWAY* must remain unset. One assertion test. | ✓ |
| nono assigns BOTH broker and child explicitly via IPC | Belt-and-braces but adds new IPC surface from broker to nono and races with child startup. | |
| Broker creates its own child Job and nests under nono's Job | Per-child Job lifecycle but adds Job hierarchy complexity. Overkill for SHELL-01. | |

**User's choice:** nono assigns broker to Job; rely on auto-inheritance for child
**Notes:** One test asserts child PID is in Job Object after spawn.

---

## Broker binary placement + token-helper lift

### Q1 → D-05: Broker binary location

| Option | Description | Selected |
|--------|-------------|----------|
| New workspace member: crates/nono-shell-broker/ (RESEARCH §3a preferred) | Standalone crate. Releases ship nono-shell-broker.exe alongside nono.exe. Cleanest separation. | ✓ |
| [[bin]] inside nono-cli with multi-call dispatch | One binary, two roles based on argv. Cuts release artifact count but mixes Medium-IL supervisor + broker code; review story muddier. | |
| Broker code lives in crates/nono as a library, fronted by a thin nono-cli [[bin]] | Library reusable from FFI / future bindings. Higher up-front cost; broker is deeply Windows-specific. | |

**User's choice:** New workspace member: crates/nono-shell-broker/
**Notes:** Cross-compile + signed-binary pipelines must add the new artifact.

### Q2 → D-06: create_low_integrity_primary_token() location

| Option | Description | Selected |
|--------|-------------|----------|
| Move to crates/nono/src/sandbox/windows.rs (RESEARCH preferred) | Lift to library, alongside try_set_mandatory_label / low_integrity_label_and_mask. Single source of truth. ~50 LOC moved. | ✓ |
| Extract to a new internal crate crates/nono-windows-tokens/ | New 4th workspace member; over-engineered for one function. | |
| Duplicate into broker crate; flag both copies as parity-locked | Drift risk explicitly flagged by RESEARCH. Not recommended. | |

**User's choice:** Move to crates/nono/src/sandbox/windows.rs
**Notes:** Library boundary preserved as long as function stays parameterless and policy-free.

### Q3 → D-07: Broker path resolution

| Option | Description | Selected |
|--------|-------------|----------|
| Sibling-binary lookup: std::env::current_exe() + parent dir (Recommended) | Fail-fast with NonoError::BrokerNotFound if missing. No new config surface. Mirrors how proxy is located today. | ✓ |
| Configurable via env NONO_SHELL_BROKER_PATH (override + sibling default) | Adds attack surface: env-poisoning could redirect to different binary. | |
| Bundle broker as a resource inside nono.exe; extract to %TEMP% on first shell launch | Single .exe ships but %TEMP%-extraction is a known smell (write-then-exec race; AV false positives). Not recommended for security tool. | |

**User's choice:** Sibling-binary lookup: std::env::current_exe() + parent dir
**Notes:** No env-var override surface.

### Q4 → D-08: Broker IPC shape

| Option | Description | Selected |
|--------|-------------|----------|
| argv only — simple flat command-line args (Recommended) | Inheritable handles via PROC_THREAD_ATTRIBUTE_HANDLE_LIST. Env via SetEnvironmentVariable. No JSON parsing surface in broker. | ✓ |
| argv + dedicated config pipe (anonymous pipe inherited by broker) | More structured but adds JSON parsing surface; opens vector for malformed-config crashes. | |
| argv + config file in well-known location | Persists past broker process; failure modes around cleanup, race conditions, ACL-on-temp. | |

**User's choice:** argv only — simple flat command-line args
**Notes:** Profile/CapabilitySet NOT passed to broker (RESEARCH §3a — labels applied supervisor-side).

---

## Phase 31 scope boundary

### Q1 → D-09: Acceptance criteria set

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 30 #1-#6 unchanged + harness Out-File→Set-Content fix (Recommended) | Most direct lift of Phase 30 work. Adds Acceptance #7 = harness fix verified by passing corrected Set-Content write-deny test. | ✓ |
| Phase 30 set + harness fix + AIPC-grandchild verification | Adds Acceptance #8 = smoke test verifying Phase 18 handle brokering still works for grandchildren. ~0.5d. | |
| Stripped: launch + write-deny only; cookbook/SHELL-01 flip in a follow-up | Keeps Phase 31 narrow but SHELL-01 stays in ⚠ status until follow-up. | |

**User's choice:** Phase 30 #1-#6 unchanged + harness Out-File→Set-Content fix
**Notes:** AIPC-grandchild verification is not a phase gate (smoke-tested informally at most).

### Q2 → D-10: Audit-ledger emissions

| Option | Description | Selected |
|--------|-------------|----------|
| No ledger emissions in Phase 31; track in v2.4 follow-up (Recommended) | Broker is structural enforcement. AuditEventPayload extension for shell launches stays out of scope. v2.4 deferred task. | ✓ |
| Add a single broker_spawn ledger event in Phase 31 | Reuses AuditRecorder plumbing. ~30 LOC + 1 test. Locks parity with Phase 23 AIPC emissions. | |
| Full per-write parity — every NO_WRITE_UP denial logged | Requires ETW or Win32 audit-log subscription. Belongs in dedicated audit-coverage milestone. | |

**User's choice:** No ledger emissions in Phase 31; track in v2.4 follow-up
**Notes:** Broker is a structural enforcement layer, not a per-decision gate.

### Q3 → D-11: AppliedLabelsGuard leak handling

| Option | Description | Selected |
|--------|-------------|----------|
| Stay as separate quick task (Recommended) | Phase 30 D-09 already deferred this. Phase 31 is the broker lift; mixing in a Drop-lifecycle bug expands scope. | ✓ |
| Fold into Phase 31 as a Wave-0 prerequisite | Adds ~0.5-1 day. 9 leaked labels make broker write-deny verification noisier. | |
| Fold in but as Wave-3 cleanup after broker lands | Risk: phase grows past 7-day estimate. | |

**User's choice:** Stay as separate quick task
**Notes:** Suggested slug 'nono-labels-guard-leak'. Phase 31 field-test treats label warnings as expected.

### Q4 → D-12: Milestone allocation

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 31 ships in v2.3 — reopen the milestone (Recommended) | PROJECT.md already lists SHELL-01 (Phase 31, ~7 days) as remaining v2.3 work. Bring v2.3 status back to in_flight. | ✓ |
| Phase 31 ships in v2.4 — v2.3 stays closed | Cleaner milestone arithmetic but PROJECT.md needs editing; pushes cookbook flip past v2.3 release date. | |
| Defer milestone decision — plan/execute Phase 31 standalone, decide at close time | Keeps options open but creates STATE.md drift. | |

**User's choice:** Phase 31 ships in v2.3 — reopen the milestone
**Notes:** Close v2.3 with /gsd-complete-milestone v2.3 once Phase 31 + Phase 25/26/27 follow-ups land.

---

## Failure-mode response if A2 fails during integration

### Q1 → D-13: Pivot policy on sub-A2 failure

| Option | Description | Selected |
|--------|-------------|----------|
| Hard timebox + ProcMon + replan checkpoint at day 5 (Recommended) | If TUI fails: ≤2d ProcMon. Day 5 unresolved → halt phase, paused finding, replan (split or descope). No silent slip. | ✓ |
| Open-ended investigation; phase ships when it ships | Phase 31 may slip past 7-9d estimate. Loses delivery predictability for v2.3. | |
| Auto-pivot to broker-allocates-ConPTY (option (b) from D-01) on first TUI failure | Skips ProcMon localization. May not solve actual root cause. | |
| Auto-descope to pipe-stdio fallback (no TUI) on first TUI failure | Phase 30 D-05 unlock required. Ships fastest but degrades Claude Code experience to text-only. | |

**User's choice:** Hard timebox + ProcMon + replan checkpoint at day 5
**Notes:** No silent slip past timebox.

### Q2 → D-14: Field-test scope

| Option | Description | Selected |
|--------|-------------|----------|
| Single-box validation on user's Windows test box (Recommended) | Match PoC validation discipline. CI matrix expansion is v2.4 follow-up. Aligns with Phase 15 / Phase 30 / PoC. | ✓ |
| Single-box validation + Windows 10 vs Windows 11 spot-check | De-risks version-specific behavior. ~0.5d if user has second box. | |
| CI matrix: Windows 10 22H2 + Windows 11 23H2 + Windows Server 2022 | Most thorough but ~2d for harness automation; pushes past 7-9d estimate. | |

**User's choice:** Single-box validation on user's Windows test box
**Notes:** CI matrix expansion is a v2.4 follow-up.

### Q3 → D-15: Phase 30 retained code disposition

| Option | Description | Selected |
|--------|-------------|----------|
| Replace with WindowsTokenArm::BrokerLaunch; delete the LowIlPrimary arm and its tests (Recommended) | Cleanest — retained code was scaffolding no longer needed. pty_token_gate_tests rewritten to assert BrokerLaunch dispatch. | ✓ |
| Keep LowIlPrimary as a fallback arm; BrokerLaunch is preferred but LowIlPrimary remains for Direct (non-PTY) launches | RESEARCH says LowIlPrimary stays for Direct path. Verify which call sites still hit it. | |
| Delete all Phase 30 token-arm code and tests; broker is the only Low-IL spawn path | Aggressively remove everything. Need to verify Direct never spawns Low-IL children. | |

**User's choice:** Replace with WindowsTokenArm::BrokerLaunch; delete the LowIlPrimary arm and its tests
**Notes:** Planner must verify no production path requires Low-IL spawn outside the broker before deletion lands. If Direct path needs Low-IL, re-evaluate D-15.

### Q4 → D-16: Rollback story on full timebox-failure

| Option | Description | Selected |
|--------|-------------|----------|
| SHELL-01 reverts to v3.0 deferral; cookbook reverts to Phase 30 final-state language (Recommended) | Same rollback shape Phase 30 prescribed. v2.3 closes WITHOUT SHELL-01. Phase 31 closes as failure-mode finding. | ✓ |
| Keep SHELL-01 in ⚠ Phase 31 candidate state; open Phase 31.1 to re-attempt with new evidence | SHELL-01 stays in limbo until Phase 31.1 closes. | |
| Ship pipe-stdio fallback shell as 'nono shell --no-tui' acceptance for SHELL-01 | Phase 30 D-05 acceptance unlocked. Ships some Windows coverage; meaningful degradation. | |

**User's choice:** SHELL-01 reverts to v3.0 deferral; cookbook reverts to Phase 30 final-state language
**Notes:** Phase 31 closes as failure-mode finding analogous to Phase 30.

---

## Claude's Discretion

- Wave structure (planner discretion). Natural shape from RESEARCH §5: Wave 0 = harness fix + Phase 30 token-arm code retirement; Wave 1 = crates/nono-shell-broker/ scaffolding + token-helper library lift; Wave 2 = launch.rs BrokerLaunch cascade arm + handle-list discipline + Job Object wiring; Wave 3 = field-test + cookbook + SHELL-01 bookkeeping flip.
- Exact `WindowsTokenArm::BrokerLaunch` enum variant placement and matcher arm position in `select_windows_token_arm`.
- Whether to pre-emit a broker `--smoke` self-test mode for CI.
- Whether `crates/nono-shell-broker/` includes a `#[cfg(not(windows))]` stub `main()` (PoC pattern) or refuses to compile on non-Windows.
- Tracing/log routing inside the broker (own stderr captured by nono.exe vs file vs structured forwarding).

## Deferred Ideas

- v2.4 follow-up: shell-launch audit-ledger emissions (parity with Phase 23 AIPC).
- v2.4 follow-up: AppliedLabelsGuard Drop-ordering bug fix (`nono-labels-guard-leak` quick task).
- v2.4 follow-up: CI matrix expansion (Windows 10 22H2 / Windows 11 23H2 / Server 2022).
- v2.4 follow-up: AIPC-grandchild verification under broker.
- v2.4 ergonomic: `nono shell --integrity <Untrusted|Low|Medium>`.
- v3.0: AppContainer-based isolation for `nono shell`.
- v3.0: Kernel mini-filter driver for FS deny enforcement.
- v3.0: AppContainer profile for the Claude Code child specifically.
- `nono shell` Linux/macOS broker port (separate work; different mechanisms).
- Phase 30 retained code retirement plan if D-15 keeps LowIlPrimary as fallback.
- Broker `--smoke` self-test mode (Claude's discretion for Phase 31).
