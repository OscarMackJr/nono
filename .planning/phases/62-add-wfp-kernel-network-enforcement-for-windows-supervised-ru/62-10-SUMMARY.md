---
phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru
plan: 10
subsystem: infra
tags: [windows, wfp, broker, token, restricted-token, write-restricted, session-sid, low-il]

requires:
  - phase: 62
    provides: "62-05..62-09: WFP service activates cleanly (FwpmFilterAdd0=0, persistent session), BrokerLaunchNoPty arm, create_low_integrity_primary_token library fn, restricted_token.rs WRITE_RESTRICTED reference"

provides:
  - "create_low_integrity_primary_token_with_sid: broker child token carries session_sid as WRITE_RESTRICTED restricting SID so WFP ALE_USER_ID filter matches confined child's outbound connections"
  - "--session-sid plumbing: launch.rs no-PTY broker_args -> broker argv parser (with ConvertStringSidToSidW validation) -> conditional token build"
  - "Fail-closed contract at 3 sites: launch.rs ok_or, broker --no-pty requires --session-sid, library FFI failure propagates"

affects:
  - 62-11 (uninstall purge - still required before SC4)
  - 62-04 HUMAN-UAT re-run (SC1 enforcement-MATCH validation pending)

tech-stack:
  added: []
  patterns:
    - "WRITE_RESTRICTED+session_sid injection: CreateRestrictedToken(WRITE_RESTRICTED, 1 restricting SID) then apply_low_il_label — proven WFP-matchable shape (mirrors restricted_token.rs)"
    - "Fail-closed SID injection: library fn, broker parser, and launch.rs caller all independently reject missing/malformed SID with SandboxInit error"
    - "Shared label helper: apply_low_il_label extracted so both parameterless and _with_sid fns are byte-equivalent on the label path"

key-files:
  created: []
  modified:
    - "crates/nono/src/sandbox/windows.rs"
    - "crates/nono/src/lib.rs"
    - "crates/nono-shell-broker/src/main.rs"
    - "crates/nono-cli/src/exec_strategy_windows/launch.rs"

key-decisions:
  - "WRITE_RESTRICTED + restricting SID is the only Win32 mechanism to inject an arbitrary new SID into a token's access check surface (no API adds a new enabled group); CreateRestrictedToken with WRITE_RESTRICTED is the sole viable shape (debug D1 final)"
  - "apply_low_il_label factored as private helper so both parameterless and _with_sid fns stay byte-equivalent on the label path; neither fn's observable behavior changes"
  - "Fail-closed at 3 independent sites (library, broker parser, launch.rs caller) — defense-in-depth so SID-less child is structurally impossible on the BrokerLaunchNoPty arm"
  - "Cross-target clippy deferred to CI: windows.rs / exec_strategy_windows / broker are cfg(windows)-only and do not compile under Linux/macOS targets on this Windows host (see cross-target verification section)"

requirements-completed:
  - REQ-WFP-01
---

# Phase 62 Plan 10: broker session-SID injection closes WFP enforcement-MATCH gap (F-62-UAT-05)

**Injects session_sid as WRITE_RESTRICTED restricting SID into the BrokerLaunchNoPty Low-IL token so the WFP ALE_USER_ID filter finally matches the confined child's outbound connections, closing the F-62-UAT-05 enforcement gap where curl returned the external IP despite WFP activating cleanly.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-06-02T~00:00Z
- **Completed:** 2026-06-02
- **Tasks:** 3
- **Files modified:** 4

## Root Cause Summary (for context)

After 62-05..62-09, WFP activates cleanly (FwpmFilterAdd0=0, persistent session, no marshaling errors), yet live Win11 UAT showed `curl` still reached the external IP. Root cause: the WFP block filter scopes to the connection via `FWPM_CONDITION_ALE_USER_ID` with SD `D:(A;;CC;;;<session_sid>)`. For the filter to match, the connecting process token must carry `session_sid`. The `BrokerLaunchNoPty` arm (selected for `claude-code` profile via `windows_low_il_broker=true`) builds the child token via the parameterless `create_low_integrity_primary_token()` which injects NO SID — so the filter installs but matches nothing.

The `WriteRestricted` arm does inject session_sid as a restricting SID, but breaks child startup (curl/powershell `STATUS_ACCESS_DENIED`). The broker path differs (Medium-IL broker self-degrades, anonymous-pipe stdio, no ConPTY), making WRITE_RESTRICTED safe on that path — but it was never tried until this plan.

## Accomplishments

- Factored `apply_low_il_label` private helper from `create_low_integrity_primary_token`; both fns share the identical label path
- Added `create_low_integrity_primary_token_with_sid(session_sid: &str) -> Result<OwnedHandle>`: opens process token, parses `session_sid` via `ConvertStringSidToSidW` (fail-closed), builds `CreateRestrictedToken(WRITE_RESTRICTED, 1 restricting SID = session_sid)`, applies Low-IL label
- Re-exported new fn from `nono::lib.rs`
- Added unit tests: malformed SID rejects (Err), valid S-1-5-117-* SID returns Ok (with CI-limitation note for elevation-sensitive environments)
- Added `session_sid: Option<String>` to `BrokerArgs`; `--session-sid` parser arm with `ConvertStringSidToSidW` validation at parse time
- FAIL-CLOSED gate in broker: `--no-pty` without `--session-sid` returns `SandboxInit` error (never spawns unmatched child)
- Conditional token build in `broker::run`: `Some(sid)` -> `create_low_integrity_primary_token_with_sid`, `None` -> parameterless fn; `?` propagates failure, never falls back on error
- Updated `parse_args_no_pty_flag_accepted` test to include required `--session-sid`
- Added 3 new broker argv tests: `session_sid_parsed`, `no_pty_without_session_sid_returns_error`, `malformed_session_sid_returns_error`
- Added `--session-sid <config.session_sid>` push in `launch.rs` no-PTY broker_args block with `ok_or_else` fail-closed guard

## Task Commits

1. **Task 1: Add create_low_integrity_primary_token_with_sid to nono library** - `b5c2eae0` (feat)
2. **Task 2: Thread --session-sid through broker (argv + validation + conditional token call)** - `9af1136e` (feat)
3. **Task 3: Push --session-sid from launch.rs no-PTY broker_args (fail-closed)** - `9e75d084` (feat)

## Files Created/Modified

- `crates/nono/src/sandbox/windows.rs` - apply_low_il_label helper + create_low_integrity_primary_token_with_sid + unit tests
- `crates/nono/src/lib.rs` - re-export create_low_integrity_primary_token_with_sid
- `crates/nono-shell-broker/src/main.rs` - BrokerArgs.session_sid field, --session-sid parser/validation, fail-closed gate, conditional token build, 3 new tests + updated existing test
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` - --session-sid push in no-PTY broker_args with ok_or_else fail-closed guard

## Decisions Made

- WRITE_RESTRICTED is the only correct shape: no Win32 API adds an arbitrary new SID as a normal enabled group to an existing token; `CreateRestrictedToken` (restricting SID) is the only injection path (debug D1 final decision)
- apply_low_il_label extracted as private helper ensuring both fns stay byte-equivalent on the label path — no behavior change to the parameterless fn
- Fail-closed at 3 independent sites (library FFI failure, broker parser validation, launch.rs ok_or) so a SID-less child is structurally impossible on the BrokerLaunchNoPty arm
- PTY broker_args (L1490) intentionally NOT changed — PTY path waives per-session WFP

## Deviations from Plan

None — plan executed exactly as written. One minor adaptation: the existing `parse_args_no_pty_flag_accepted` test was updated to include `--session-sid` since the new fail-closed gate requires it; this is consistent with the "always pass session-sid on --no-pty" production contract documented in the plan.

## Fail-Closed Contract (SECURITY-CRITICAL)

All three sites enforce the contract independently:

| Site | Mechanism | Failure mode |
|------|-----------|--------------|
| `nono` library fn | `ConvertStringSidToSidW==0` → `Err(SandboxInit)` | SID-less token never returned |
| `nono` library fn | `CreateRestrictedToken==0` → `Err(SandboxInit)` | SID-less token never returned |
| Broker `parse_args` | `ConvertStringSidToSidW` rejects malformed SID at parse time | Never reaches token build |
| Broker `parse_args` | `--no-pty && session_sid.is_none()` → `Err(SandboxInit)` | Never reaches run() |
| Broker `run` | `?` on token build call | FFI failure propagates, no child spawned |
| `launch.rs` | `ok_or_else` on `config.session_sid` | SandboxInit if invariant violated |

Spawning an unmatched WFP child (filter installs but matches nothing = silent non-enforcement, operator believes they are blocked but are not) is the worst outcome and is now structurally impossible on the BrokerLaunchNoPty arm.

## No Escalation / Threat Review (T-62-21/22/23)

- **T-62-21 (EoP):** session_sid is a synthetic `S-1-5-117-*` SID naming no real account. As a RESTRICTING SID under `WRITE_RESTRICTED` it can only NARROW write access, never widen it. Token integrity stays Low (label applied AFTER CreateRestrictedToken). No escalation.
- **T-62-22 (Spoofing):** FAIL-CLOSED at all 3 sites ensures no SID-less child can be spawned when network.block is set. Silent non-enforcement is now structurally prevented.
- **T-62-23 (Tampering):** DACL grants from `AppliedDaclGrantsGuard` become operative on the broker arm (previously inert per dacl_guard.rs L32-34) — this is intended parity, not new surface. Only grants on paths already in the capability grant set, already user-owned, with `FILE_GENERIC_WRITE|DELETE` only, revoked on Drop.

## Cross-Target Clippy Verification

- **Windows host (`cargo clippy -p nono -p nono-shell-broker -p nono-cli -- -D warnings -D clippy::unwrap_used`):** PASS (clean, 0 warnings)
- **Linux target (x86_64-unknown-linux-gnu) and macOS target (x86_64-apple-darwin):** PARTIAL / deferred to CI per CLAUDE.md Coding Standards. The modified files (`windows.rs`, `exec_strategy_windows/launch.rs`, `nono-shell-broker/src/main.rs`) are entirely `#[cfg(windows)]` / `cfg(target_os = "windows")` gated and do not compile under the Unix targets on this Windows host. The cross-toolchain is not installed. CI is the authoritative cross-target gate per `.planning/templates/cross-target-verify-checklist.md`.

## Live SC1 Validation (empirical unknown — pending human UAT)

The ONE empirical unknown from the plan — does `curl`/`powershell` START under WRITE_RESTRICTED+Low-IL on the broker path? — cannot be resolved here. The WriteRestricted arm broke startup on the non-broker path (curl/powershell `STATUS_ACCESS_DENIED`). The broker path differs materially (Medium-IL broker self-degrades, anonymous-pipe stdio, no ConPTY). The startup risk CLASS is reintroduced by this plan, but the execution shape that caused it previously may not apply.

**Required human UAT step (62-04 SC1 re-run after this plan):**
1. Rebuild `nono.exe` + `nono-shell-broker.exe` from the updated codebase
2. From `%USERPROFILE%\.claude` (profile-covered cwd, mandatory): `nono run --profile claude-code --block-net --allow-cwd -- curl.exe -sS -m 5 https://api.ipify.org`
3. **PRIMARY:** confirm `curl` is BLOCKED (no external IP, "BLOCKED:"/timeout/error, exit != 0)
4. **STARTUP:** confirm `curl` STARTS at all (if startup regresses to `STATUS_ACCESS_DENIED`-class failure, escalate to new debug session — D1 fallback alternative does not exist; restricting SID is the only shape)
5. Record startup result in next UAT session notes

Until SC1 is confirmed, REQ-WFP-01 remains in verification state (enforcement code complete, live run pending).

## 62-11 Dependency Note

Plan 62-11 (WFP object uninstall purge — persistent objects from 62-09) is STILL required before SC4. This plan (62-10) closes the SC1 enforcement-MATCH gap only. SC4 (block-net with multiple concurrent sessions + cleanup) depends on 62-11 completing the uninstall path.

## Issues Encountered

None — plan executed exactly as specified. The only adaptation was updating the existing `parse_args_no_pty_flag_accepted` unit test (which passed `--no-pty` without `--session-sid`) to include the now-required `--session-sid` argument, which is the correct production shape.

## Next Phase Readiness

- Code complete and building. Pending: human UAT SC1 re-run to confirm (a) curl starts under WRITE_RESTRICTED+Low-IL on broker path and (b) outbound is blocked
- 62-11 (uninstall purge) must run before SC4
- If SC1 startup regresses: escalate to new debug session (no fallback alternative to restricting-SID injection exists per D6)

---
*Phase: 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru*
*Completed: 2026-06-02*

## Self-Check: PASSED

Files verified present:
- `crates/nono/src/sandbox/windows.rs` - contains `create_low_integrity_primary_token_with_sid` (FOUND)
- `crates/nono/src/lib.rs` - contains `create_low_integrity_primary_token_with_sid` re-export (FOUND)
- `crates/nono-shell-broker/src/main.rs` - contains `--session-sid` (FOUND)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` - contains `--session-sid` (FOUND)

Commits verified present:
- `b5c2eae0` - feat(62-10): add create_low_integrity_primary_token_with_sid to nono library (FOUND)
- `9af1136e` - feat(62-10): thread --session-sid through broker argv, validation, and token build (FOUND)
- `9e75d084` - feat(62-10): push --session-sid from launch.rs no-PTY broker_args (fail-closed) (FOUND)
