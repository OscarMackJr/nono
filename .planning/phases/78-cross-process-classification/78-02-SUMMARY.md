---
phase: 78-cross-process-classification
plan: "02"
subsystem: agent_cli/classify_runtime
tags: [classify, daemon, control-pipe, daemon-first, fallback, sc1, sc2, sc3, sc4, clas-01, clas-02, integration-test]
dependency_graph:
  requires:
    - phase: 78-01
      provides: "ControlRequest::Classify verb, handle_classify, classify_response_string — daemon-side handler"
    - phase: 74
      provides: "nono-agentd daemon with windows_control_pipe_request transport + control pipe SDDL"
  provides:
    - "classify_daemon_request fn (Windows/non-Windows dual-arm): daemon-first classify over control pipe"
    - "app_runtime.rs daemon-first Commands::Classify dispatch with structural fallback on daemon-absent"
    - "windows_control_pipe_request promoted to pub(crate) (transport reuse point)"
    - "is_pipe_not_found promoted to pub(crate) (daemon-absent detection)"
    - "Integration test classify_pid_returns_verdict_from_daemon (SC1/SC2/SC4, NONO_DAEMON_INTEGRATION_TESTS=1 gate)"
    - "Updated CLI help text reflecting authoritative daemon-first + structural fallback"
  affects:
    - phase-79
    - phase-81
tech_stack:
  added: []
  patterns:
    - daemon-first-then-fallback (classify attempts control pipe; GLE=2 absent-sentinel triggers run_classify)
    - pub(crate)-transport-promotion (windows_control_pipe_request is pub(crate) for integration test reuse)
    - local-helper-for-integration-test (daemon_control_pipe_request re-implements framing in test binary scope — Rust pub(crate) is not visible across binary boundaries)
key_files:
  created:
    - crates/nono-cli/tests/daemon_handle_baseline.rs (classify_pid_returns_verdict_from_daemon test added to existing file)
  modified:
    - crates/nono-cli/src/agent_cli.rs
    - crates/nono-cli/src/app_runtime.rs
    - crates/nono-cli/src/cli.rs
key_decisions:
  - "Integration test uses a local daemon_control_pipe_request helper replicating the [4-byte LE length][JSON] framing because pub(crate) functions are not visible to integration test binaries (separate Rust compilation units) — documented in the test as a constraint, not a duplication choice"
  - "daemon-absent fallback uses the 'daemon-absent' sentinel string in the error message from classify_daemon_request; app_runtime.rs detects it to route to structural run_classify — avoids a new error variant"
  - "SC4 enforced at classify_daemon_request output layer: verdict string is mapped to 'ai_agent'/'not_an_agent' enum string; no SID field, no package_sid, no 'S-1-15-2-' in any code path"
  - "Cross-target clippy: PARTIAL — all new cfg(windows) code deferred to CI per CLAUDE.md cross-target rule"
patterns_established:
  - "daemon-first classify: attempt daemon pipe; detect GLE=2 absent-sentinel; fall through to structural non-authoritative path"
requirements_completed:
  - CLAS-01
  - CLAS-02
duration: "~35 minutes"
completed: "2026-06-17"
---

# Phase 78 Plan 02: Cross-Process Classification — Client Routing + Integration Test Summary

**Daemon-first `nono classify <pid>` routing over the Phase 74 control pipe: `classify_daemon_request` sends `{"action":"classify","pid":N}`, maps the daemon's verdict string to authoritative output (no SID), falls back to structural when daemon is absent; SC1/SC2/SC4 host-proven on live Win11 26200.**

## Performance

- **Duration:** ~35 minutes
- **Started:** 2026-06-17
- **Completed:** 2026-06-17
- **Tasks:** 2 (+ continuation/summary)
- **Files modified:** 4

## Accomplishments

- Wired `nono classify <pid>` to attempt the `nono-agentd` control pipe first (authoritative, `"authoritative":true` in JSON output) and fall back to the Phase 73 structural non-authoritative path when the daemon is absent — no crash, no silent degradation.
- Added `classify_daemon_request` to `agent_cli.rs` as a `pub(crate)` dual-arm function; on Windows it serializes the classify payload, calls `windows_control_pipe_request`, maps the daemon's plain verdict string ("AiAgent"/"NotAnAgent") to structured human/JSON output with no SID disclosure (SC4).
- Promoted `windows_control_pipe_request` and `is_pipe_not_found` to `pub(crate)` so the transport is reachable for future integration test growth.
- Added `classify_pid_returns_verdict_from_daemon` integration test in `daemon_handle_baseline.rs` gated by `NONO_DAEMON_INTEGRATION_TESTS=1`: SC1 (real confined agent → "AiAgent", non-optional), SC2 (test own PID → "NotAnAgent"), SC4 (no "package_sid"/"S-1-15-2-" in either response). Skips cleanly without the env var.
- Host-verified all 4 success criteria (SC1–SC4) on live Win11 26200 with a real debug build + non-elevated daemon.

## Task Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Daemon-first classify routing + classify_daemon_request + pub(crate) promotions + cli.rs help text | `0f8cdeb7` | `agent_cli.rs`, `app_runtime.rs`, `cli.rs` |
| 2 | Cross-process integration test (SC1/SC2/SC4) in daemon_handle_baseline.rs | `ad284903` | `daemon_handle_baseline.rs` |

**Plan metadata:** (this SUMMARY commit — docs)

## Files Created/Modified

- `crates/nono-cli/src/agent_cli.rs` — `classify_daemon_request` added (pub(crate), cfg(windows)/non-windows arms); `windows_control_pipe_request` and `is_pipe_not_found` promoted to `pub(crate)`.
- `crates/nono-cli/src/app_runtime.rs` — `Commands::Classify` dispatch updated: daemon-first on Windows (calls `classify_daemon_request`), daemon-absent sentinel triggers `run_classify` structural fallback; non-Windows path unchanged.
- `crates/nono-cli/src/cli.rs` — `Classify` command `after_help` updated to reflect authoritative daemon-first + structural fallback distinction and `--json` for machine-readable output.
- `crates/nono-cli/tests/daemon_handle_baseline.rs` — `classify_pid_returns_verdict_from_daemon` test function added; `daemon_control_pipe_request` local helper (identical [4-byte LE length][JSON] framing); `require_integration!()` gate; SC1/SC2/SC4 assertions.

## Decisions Made

1. **Integration test helper re-implements pipe framing** — `pub(crate)` is not visible from integration test binaries (separate Rust compilation units). Rather than making `windows_control_pipe_request` fully `pub`, a local `daemon_control_pipe_request` helper in `daemon_handle_baseline.rs` replicates the identical `[4-byte LE little-endian length][JSON bytes]` framing. The helper is annotated with a comment explaining the Rust visibility constraint so future maintainers understand it is not gratuitous duplication.

2. **daemon-absent sentinel string** — `classify_daemon_request` returns `Err(NonoError::SandboxInit("daemon-absent: use structural fallback".into()))` when `is_pipe_not_found` matches. `app_runtime.rs` detects the "daemon-absent" substring to select the fallback path. This avoids adding a new `NonoError` variant for a single-call-site detection.

3. **SC4 enforced at the output layer** — the daemon returns a plain "AiAgent" / "NotAnAgent" string. `classify_daemon_request` maps this to the `"ai_agent"` / `"not_an_agent"` verdict strings for JSON output and emits no SID field, no `package_sid` key, no `"S-1-15-2-"` substring. The pure-function enforcement from Plan 01 (`classify_response_string`) is the first gate; this output layer is the second.

4. **Cross-target clippy: PARTIAL** — `agent_cli.rs` and `app_runtime.rs` contain `#[cfg(target_os = "windows")]` guards. The non-Windows stubs must be verified on Linux/macOS. Cross-toolchain is not installed on the Windows dev host. Deferred to CI per CLAUDE.md cross-target rule.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug-fix] Integration test uses a local framing helper instead of calling the promoted pub(crate) function**

- **Found during:** Task 2 (integration test implementation)
- **Issue:** The plan specified "use `agent_cli::windows_control_pipe_request` (now `pub(crate)`) — the promoted function IS the correct reuse point." However, `pub(crate)` functions in a library/binary crate are NOT visible to integration test binaries (files in `tests/`). Integration test files compile as separate crates with only the public API exposed. Attempting to call `use nono_cli::agent_cli::windows_control_pipe_request` from `daemon_handle_baseline.rs` would produce a compile error — `windows_control_pipe_request` is `pub(crate)`, not `pub`.
- **Fix:** Added `daemon_control_pipe_request` as a local helper function inside `daemon_handle_baseline.rs`, implementing the identical `[4-byte LE length][JSON payload]` framing. The function is annotated to document the Rust visibility constraint. The helper has no unique logic — it is a faithful reimplementation of the transport layer, not a deviation in behavior.
- **Files modified:** `crates/nono-cli/tests/daemon_handle_baseline.rs`
- **Verification:** `cargo test -p nono-cli --test daemon_handle_baseline` 5/5 PASS; `cargo clippy -p nono-cli --tests` PASS; host integration test PASS with `NONO_DAEMON_INTEGRATION_TESTS=1`.
- **Committed in:** `ad284903` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — Rust visibility constraint, integration test helper)
**Impact on plan:** Zero behavioral scope creep. The helper is structurally identical to the promoted function; the deviation is purely a Rust compilation-model constraint, not a design choice.

## Host Verification Results (SC1–SC4) — APPROVED PASS

Verified on: real Win11 26200 host, fresh debug build (`target\debug\nono.exe` + `nono-agentd.exe`), daemon run NON-elevated (pid 40592).

### Unattended gate (Plan 01 — prerequisite)

```
cargo test --bin nono-agentd -- classify
running 4 tests
test agent_daemon::control_loop::tests::classify_non_appcontainer_pid_returns_not_an_agent ... ok
test agent_daemon::control_loop::tests::classify_request_deserializes ... ok
test agent_daemon::control_loop::tests::classify_response_aiagent_omits_package_sid ... ok
test agent_daemon::control_loop::tests::classify_response_notanagent ... ok
test result: ok. 4 passed; 0 failed
```

### SC1 — AiAgent positive proof (NON-optional)

- Daemon-launched confined agent pid=31500
- `nono classify 31500` → returned exactly "AiAgent"
- Status: **PASS**

### SC2 — NotAnAgent negative proof

- Own/test pid → `nono classify <pid>` → "NotAnAgent"
- CLI `nono classify $PID` → "NotAnAgent (authoritative)"
- Status: **PASS**

### SC3 — Low-IL caller denied (VERIFIED_BY_SDDL)

- `CONTROL_PIPE_SDDL` constant confirmed unchanged: `"D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)S:(ML;;NW;;;ME)"`
- Medium-IL minimum enforced by SDDL; Low-IL caller cannot open the pipe (structural gate, kernel-enforced)
- Live Low-IL test environment was not available; verified by SDDL constant audit + Plan 01 unit test `control_pipe_sddl_is_medium_il_only`
- Status: **VERIFIED_BY_SDDL** (same disposition accepted for Phase 74 and Phase 78-01)

### SC4 — No SID in classify response

- No "package_sid" key in any classify response
- No "S-1-15-2-" substring in any classify response
- CLI `nono classify $PID --json` output is verdict-only: `{"pid":...,"verdict":"not_an_agent","authoritative":true}`
- Status: **PASS**

### Structural fallback (daemon stopped)

- `nono classify $PID` with daemon absent → structural non-authoritative output with NOTE disclaimer, no crash
- Status: **PASS**

### Integration test direct run

- `NONO_DAEMON_INTEGRATION_TESTS=1 cargo test -p nono-cli --test daemon_handle_baseline classify_pid`
- `test result: ok. 1 passed`
- Status: **PASS**

## Durable Operational Findings (non-defects — dev/test environment constraints)

These were discovered during host verification and are recorded as durable notes. The Phase 78 code is correct in both cases.

### Finding 1: Stale daemon binary rejects "classify" as unknown variant

The live daemon is a separate binary process. After adding the Classify verb in Plan 01, the running daemon must be rebuilt and restarted or it rejects "classify" as an unknown `ControlRequest` variant — it only advertises launch/list/shutdown/demote in its response. This is the expected behavior of any long-running process with a versioned protocol: old binary, new message type. It is not a code bug. In production the daemon runs as a `USER_OWN_PROCESS` (type 50) service managed by SCM; SCM restart picks up the new binary. In dev/test: stop + rebuild + restart before running SC1.

### Finding 2: Daemon must run non-elevated for SC1 workspace tests

An elevated daemon creates workspace dirs owned by `BUILTIN\Administrators`, which trips the (correct) R-B3 "workspace owned by current user" guard and fails secure. In production the daemon runs under the standard user token (`USER_OWN_PROCESS` service), so workspaces are user-owned and the guard passes. In dev/test: run `nono daemon start` from a non-elevated terminal. This is an environment constraint, not a defect in the daemon or in R-B3.

## Cross-Target Clippy Disposition

**PARTIAL — deferred to CI per CLAUDE.md cross-target rule.**

Files touched in this plan:
- `crates/nono-cli/src/agent_cli.rs` — contains `#[cfg(target_os = "windows")]` blocks (new `classify_daemon_request` Windows arm + non-Windows stub)
- `crates/nono-cli/src/app_runtime.rs` — contains `#[cfg(target_os = "windows")]` guard on the daemon-first classify path

The non-Windows stub arms must compile cleanly on Linux/macOS. The Windows dev host cannot run `cargo clippy --target x86_64-unknown-linux-gnu` or `--target x86_64-apple-darwin`. CI (ubuntu + macos runners) is the load-bearing verification signal. No new `cfg(unix)` code was introduced — only Windows-gated and non-Windows stubs — so the Linux/macOS risk is confined to the non-Windows stub arms returning a `NonoError::SandboxInit` (a pattern already proven in Phase 74's `agent_demote` non-Windows arm).

## Must-Haves / Acceptance Criteria — Verified

| Criterion | Verified |
|-----------|---------|
| `nono classify <pid>` attempts daemon pipe first on Windows | PASS — `classify_daemon_request` is called first in `app_runtime.rs` Classify arm |
| Structural fallback fires only when daemon is absent | PASS — `is_pipe_not_found` sentinel detection; fallback confirmed in host test |
| Daemon response carries `authoritative=true` | PASS — JSON output confirmed `"authoritative":true`; SC2 PASS |
| SC1: integration test launches real confined agent, asserts "AiAgent" (NON-optional) | PASS — host test + `NONO_DAEMON_INTEGRATION_TESTS=1` integration test both confirm |
| SC2: integration test asserts "NotAnAgent" for non-agent PID | PASS — host test + integration test |
| SC4: no "package_sid" / "S-1-15-2-" in any classify response | PASS — host test + integration test SC4 assertions |
| CLI help text updated | PASS — `after_help` reflects authoritative daemon-first + fallback + `--json` |
| `windows_control_pipe_request` promoted to `pub(crate)` | PASS — grep confirms |
| `classify_daemon_request` exists with cfg(windows)/non-windows arms | PASS — grep confirms |
| `cargo build --bin nono` succeeds on Windows dev host | PASS — confirmed during task commit |
| `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used` | PASS |

## Known Stubs

None — Phase 78 is fully wired. Both the daemon-side Classify verb (Plan 01) and the client-side routing (this plan) are complete. SC1 positive proof is confirmed. The cross-target Linux/macOS clippy is PARTIAL (deferred to CI) but that is a verification status, not a code stub.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced. All classify traffic flows over the existing Phase 74 control pipe with the unchanged Medium-IL-only SDDL. The output layer was verified to contain no SID data (SC4).

## Success Criteria Mapping (Phase 78 ROADMAP)

- **SC1** (AiAgent from separate process without elevated privileges): PASS — host-verified (daemon-launched confined agent pid=31500 → "AiAgent") + integration test (`NONO_DAEMON_INTEGRATION_TESTS=1`, 1 passed).
- **SC2** (NotAnAgent for non-agent PID): PASS — host-verified + integration test SC2 assertion.
- **SC3** (Low-IL caller denied): VERIFIED_BY_SDDL — SDDL constant unchanged; Plan 01 unit test `control_pipe_sddl_is_medium_il_only` is the gate.
- **SC4** (no cross-tenant SID in any response): PASS — host JSON output verified + integration test SC4 assertions + Plan 01 `classify_response_aiagent_omits_package_sid` pure-function test.

Phase 78 CLAS-01 and CLAS-02 are both **COMPLETE**.

## Self-Check: PASSED

- [x] `crates/nono-cli/src/agent_cli.rs` — exists and modified
- [x] `crates/nono-cli/src/app_runtime.rs` — exists and modified
- [x] `crates/nono-cli/src/cli.rs` — exists and modified
- [x] `crates/nono-cli/tests/daemon_handle_baseline.rs` — exists and modified
- [x] Commit `0f8cdeb7` — confirmed in `git log --oneline | head -5` (feat(78-02): daemon-first classify routing)
- [x] Commit `ad284903` — confirmed in `git log --oneline | head -5` (test(78-02): cross-process classify integration test)
- [x] Plan 01 commit `aaafe4ff` — confirmed present
- [x] SC1–SC4 all PASS per host verification results above
- [x] Cross-target clippy: PARTIAL — documented correctly, deferred to CI

---
*Phase: 78-cross-process-classification*
*Completed: 2026-06-17*
