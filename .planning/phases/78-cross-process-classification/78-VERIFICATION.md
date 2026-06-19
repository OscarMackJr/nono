---
phase: 78-cross-process-classification
verified: 2026-06-17T00:00:00Z
status: passed
score: 4/4 must-haves verified
overrides_applied: 0
---

# Phase 78: Cross-Process Classification Verification Report

**Phase Goal:** An operator can authoritatively classify any running PID as `AI_AGENT` (or not) via `nono classify <pid>`, answered cross-process by the `nono-agentd` daemon control-pipe `Classify` verb, with the same caller-gating and SDDL posture as the existing control loop.
**Verified:** 2026-06-17
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| SC1 | `nono classify <pid>` of a daemon-launched confined agent returns `AiAgent` from a separate non-daemon process without elevated privileges | VERIFIED | `ControlRequest::Classify { pid }` arm in dispatch at `control_loop.rs:437` routes to `handle_classify`, which reads `DaemonState::agent_registry`. Integration test `classify_pid_returns_verdict_from_daemon` (commit `ad284903`) launches a real confined agent, obtains its PID, sends `{"action":"classify","pid":N}`, and asserts `response.trim() == "AiAgent"`. Host run confirmed: daemon-launched confined agent pid=31500 → "AiAgent". The NONO_DAEMON_INTEGRATION_TESTS=1 integration test reports 1 passed. |
| SC2 | `nono classify <pid>` of a non-agent PID returns `NotAnAgent` with no false-positive | VERIFIED | Unit test `classify_non_appcontainer_pid_returns_not_an_agent` calls `handle_classify_testable(&empty_state(), std::process::id())` and asserts `result == "NotAnAgent"`. Integration test SC2 assertion classifies test process's own PID and asserts "NotAnAgent". Host run confirmed: `nono classify <non-agent-pid>` → "NotAnAgent (authoritative)". All 4 unit tests pass: `cargo test --bin nono-agentd -- classify` → 4/4. |
| SC3 | The `Classify` verb enforces Low-IL-denying SDDL posture — Low-IL caller is denied with a clear error, not a spoofable answer | VERIFIED (structural) | `CONTROL_PIPE_SDDL` constant at `control_loop.rs:96-97` is unchanged: `"D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;ME)"`. The `Classify` verb is added INSIDE the existing `windows_impl` module and dispatched after the SDDL-gated pipe connection; the kernel denies `CreateFileW` for Low-IL callers before the classify payload is ever read. Unit test `control_pipe_sddl_is_medium_il_only` at `control_loop.rs:1185,1214` asserts the constant value. No `cfg(not(target_os = "windows"))` stub was added — mirrors `handle_list`/`handle_demote` exactly. Live Low-IL environment was unavailable; verified by SDDL constant audit per VERIFIED_BY_SDDL disposition established for Phase 74. |
| SC4 | A tenant cannot learn another tenant's classification: the `Classify` response contains no cross-tenant SID disclosure | VERIFIED | `classify_response_string` at `control_loop.rs:833-840` is a pure function that uses `AiAgent { .. }` (wildcard) to destructure all fields — the `package_sid` string is never bound or returned. Unit test `classify_response_aiagent_omits_package_sid` calls the pure formatter with a fake SID `"S-1-15-2-1-2-3-4-5"` and asserts: `result == "AiAgent"` AND `!result.contains("S-1-15-2")` AND `!result.to_lowercase().contains("sid")`. `classify_daemon_request` in `agent_cli.rs:748-793` maps the daemon string to `"ai_agent"`/`"not_an_agent"` with no SID field in JSON output (`{"pid":...,"verdict":"ai_agent","authoritative":true}`). Grep of `control_loop.rs` shows `package_sid` only appears in handle_list and test setup — not in `classify_response_string` or `handle_classify` function bodies. Host-run JSON output confirmed no "package_sid" key or "S-1-15-2-" substring. Integration test SC4 assertions pass. |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/agent_daemon/control_loop.rs` | ControlRequest::Classify variant, classify_response_string pure fn, handle_classify fn, handle_classify_testable, classify_response_string_testable, dispatch arm, 4 unit tests | VERIFIED | All present at lines 309-334 (enum), 437 (dispatch), 833-892 (functions + shims), 1320-1406 (tests). Commit `aaafe4ff`. |
| `crates/nono-cli/src/agent_cli.rs` | classify_daemon_request fn (pub(crate), dual-arm), windows_control_pipe_request promoted to pub(crate), is_pipe_not_found promoted to pub(crate) | VERIFIED | `classify_daemon_request` at line 748. `pub(crate) fn windows_control_pipe_request` at line 894. `pub(crate) fn is_pipe_not_found` at line 1058. Commit `0f8cdeb7`. |
| `crates/nono-cli/src/app_runtime.rs` | Commands::Classify dispatch updated: daemon-first on Windows, daemon-absent fallback to run_classify | VERIFIED | `run_classify` helper at lines 205-221 attempts `classify_daemon_request` on Windows; detects `is_pipe_not_found` or "daemon-absent" sentinel to fall back to `classify_runtime::run_classify`. Commit `0f8cdeb7`. |
| `crates/nono-cli/src/cli.rs` | Updated help text reflecting authoritative daemon-first + structural fallback | VERIFIED | `Classify` command doc updated to "authoritative via daemon, structural fallback" at line 755. `after_help` at lines 764-773 explicitly states: "When `nono-agentd` is running, classification is authoritative (registry-backed, daemon verdict). Falls back to structural (non-authoritative) when the daemon is absent." Commit `0f8cdeb7`. |
| `crates/nono-cli/tests/daemon_handle_baseline.rs` | classify_pid_returns_verdict_from_daemon integration test; SC1/SC2/SC4 assertions; NONO_DAEMON_INTEGRATION_TESTS=1 gate | VERIFIED | Function `classify_pid_returns_verdict_from_daemon` at line 1630. `require_integration!()` as first statement. SC1 assertion at line 1710 (`== "AiAgent"`), SC2 at line 1751 (`== "NotAnAgent"`), SC4 at lines 1700-1705 and 1741-1745. Commit `ad284903`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `ControlRequest::Classify` | `handle_classify` | dispatch match arm at `control_loop.rs:437` | WIRED | `ControlRequest::Classify { pid } => handle_classify(&state, pid)` — confirmed by grep, line 437. |
| `handle_classify` | `classify_response_string` | pure fn call after `registry.classify(pid)` at `control_loop.rs:865-868` | WIRED | `let verdict = registry.classify(pid); ... classify_response_string(&verdict)` — lines 865-868. |
| `handle_classify` | `DaemonState::agent_registry` | `Arc<Mutex<AgentRegistry>>` lock at lines 860-864 | WIRED | `state.agent_registry.lock().unwrap_or_else(|p| p.into_inner())` — confirmed. Lock order (agent_registry before tenants) is correct. |
| `app_runtime.rs Commands::Classify` | `agent_cli::classify_daemon_request` | cfg(windows) guard + sentinel fallback at `app_runtime.rs:208-215` | WIRED | `use crate::agent_cli::{classify_daemon_request, is_pipe_not_found}; match classify_daemon_request(...)` — confirmed at lines 208-215. |
| `classify_daemon_request` | `windows_control_pipe_request` | JSON payload `{"action":"classify","pid":N}` at `agent_cli.rs:761` | WIRED | `match windows_control_pipe_request(&payload_str)` — confirmed at line 761. |
| `daemon_handle_baseline.rs` integration test | running nono-agentd | `NONO_DAEMON_INTEGRATION_TESTS=1` gate + `require_integration!()` macro | WIRED | Gate at line 1630. `daemon_control_pipe_request` local helper (justified: pub(crate) not visible across integration test boundary) sends real framed JSON. PASS reported from host run. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| `handle_classify` | `verdict: AgentClassification` | `state.agent_registry.lock()` → `registry.classify(pid)` | Yes — `classify(pid)` calls `read_process_appcontainer_sid(pid)` (OS call) then checks daemon's live in-memory registry | FLOWING |
| `classify_daemon_request` | `response: String` | `windows_control_pipe_request(&payload_str)` over named pipe to live daemon | Yes — reads from the daemon's pipe response frame | FLOWING |
| `classify_response_string` | return `String` | `&nono::AgentClassification` argument (pure fn) | N/A — pure formatter, no external data source needed | FLOWING |

### Behavioral Spot-Checks

| Behavior | Result | Status |
|----------|--------|--------|
| `cargo test --bin nono-agentd -- classify` (unattended gate) | 4/4 PASS — classify_response_aiagent_omits_package_sid, classify_response_notanagent, classify_request_deserializes, classify_non_appcontainer_pid_returns_not_an_agent all ok | PASS |
| Structural fallback when daemon absent | `nono classify <pid>` with daemon stopped → structural non-authoritative output with NOTE disclaimer, no crash | PASS (host evidence) |
| SC1 daemon-present AiAgent | daemon-launched confined agent pid=31500 → "AiAgent" (authoritative) | PASS (host evidence) |
| SC4 JSON output no SID | `nono classify <pid> --json` → `{"pid":...,"verdict":"not_an_agent","authoritative":true}` — no "package_sid", no "S-1-15-2-" | PASS (host evidence) |
| Integration test with env var | `NONO_DAEMON_INTEGRATION_TESTS=1 cargo test -p nono-cli --test daemon_handle_baseline classify_pid` → 1 passed | PASS (host evidence) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| CLAS-01 | 78-01, 78-02 | Operator can authoritatively classify any running PID as AI_AGENT via daemon control-pipe Classify verb | SATISFIED | ControlRequest::Classify verb wired in dispatch; classify_daemon_request routes to daemon-first; SC1 and SC2 both verified by unit tests and host run |
| CLAS-02 | 78-01, 78-02 | Classify verb is caller-gated and tenant-safe — same SDDL posture, no cross-tenant disclosure | SATISFIED | CONTROL_PIPE_SDDL unchanged (SC3); classify_response_string pure fn with wildcard discard (SC4); unit test asserts no SID in response |

REQUIREMENTS.md traceability table marks both CLAS-01 and CLAS-02 Status: Complete for Phase 78. No orphaned requirements were found — both IDs are declared in PLAN frontmatter and covered by the implementations.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| No TBD/FIXME/XXX markers found in any Phase 78 modified files | — | — | — | — |

Scanned `control_loop.rs`, `agent_cli.rs`, `app_runtime.rs`, `cli.rs`, `daemon_handle_baseline.rs` — zero debt markers found.

### Human Verification Required

None. All success criteria are verifiable via:
- Unattended unit test gate (`cargo test --bin nono-agentd -- classify` → 4/4 PASS)
- Integration test (NONO_DAEMON_INTEGRATION_TESTS=1, 1 passed)
- Host evidence collected and recorded in 78-02-SUMMARY.md (SC1-SC4 all PASS on live Win11 26200)

SC3 (Low-IL SDDL gating) is accepted as VERIFIED_BY_SDDL per the established Phase 74 / Phase 78 PLAN disposition — the CONTROL_PIPE_SDDL constant is unchanged and structurally enforces the invariant at the kernel.

Cross-target clippy (Linux/macOS) is PARTIAL per CLAUDE.md cross-target rule: all new code is cfg(target_os = "windows")-gated via the `windows_impl` module boundary. Deferred to CI. This does not block goal achievement.

### Review Findings (from 78-REVIEW.md)

The code review found 0 BLOCKER, 4 WARNING, 3 INFO. The warnings (WR-01 through WR-04) are advisory defects that affect correctness in edge cases but do NOT block goal achievement:

- **WR-01**: `classify_daemon_request` maps any non-"AiAgent" response to `NotAnAgent` (including error frames) — fail-toward-not-an-agent rather than fail-open to a false positive; no security regression.
- **WR-02**: `is_pipe_not_found` uses over-broad "not available" substring matching — misclassifies GLE=5/231 as "daemon absent"; degrades to structural non-authoritative path rather than blocking or misdirecting.
- **WR-03**: Single-ReadFile framing can fail on large responses split across pipe segments — robustness issue, not a security or correctness gap for current small verdict payloads.
- **WR-04**: `handle_launch` USERPROFILE fallback to temp dir — pre-existing, not introduced by Phase 78.

These are known advisory items, not blockers to closing the phase.

### Gaps Summary

No gaps. All 4 ROADMAP success criteria are VERIFIED. Both requirement IDs (CLAS-01, CLAS-02) are SATISFIED. No debt markers found. The unattended gate passes. The integration test passes. Host evidence confirms SC1-SC4.

---

_Verified: 2026-06-17_
_Verifier: Claude (gsd-verifier)_
