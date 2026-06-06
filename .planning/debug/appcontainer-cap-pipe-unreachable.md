---
slug: appcontainer-cap-pipe-unreachable
status: investigating
trigger: "Under `nono run --profile claude-code` (broker/AppContainer arm) the supervised child cannot connect to the supervisor capability pipe; SupervisorSocket::connect fails reading the %TEMP% rendezvous file with Access is denied (os error 5). Surfaced by Phase 59 live UAT via the aipc-cap-child harness."
created: 2026-06-06
updated: 2026-06-06
related_phase: 59-supervisor-ipc-robustness
---

# Debug: appcontainer-cap-pipe-unreachable

## Symptoms

- **Expected:** A child launched by `nono run --profile claude-code` (the Low-IL broker / AppContainer arm) can connect to the supervisor capability pipe to request capability expansion (AIPC). Concretely, `aipc-cap-child.exe sc2`/`sc1` should connect via `SupervisorSocket::connect(NONO_CAP_FILE)` and drive the cap-pipe.
- **Actual:** The child STARTS cleanly under the broker (`broker: spawned child app_container=true`, no 0xC0000142), runs its code, and fails at `nono::supervisor::socket::SupervisorSocket::connect` with: `Failed to read Windows supervisor pipe rendezvous C:\Users\OMack\AppData\Local\Temp\.nono-<nonce>.json: Access is denied. (os error 5). Ensure the supervisor created the control channel before launching the child.` Both sc2 and sc1 fail identically at conn1. `child_exit_code=1`.
- **Error:** os error 5 (ERROR_ACCESS_DENIED) reading the rendezvous JSON file at `%TEMP%\.nono-<nonce>.json`.
- **Timeline:** Surfaced 2026-06-06 during Phase 59-03 live UAT (first time anything attempted to drive the cap pipe from a real AppContainer child). Not a regression of 59-03's code — the 59-03 read_frame bounded-read + disconnect_and_reconnect are verified by integration tests + the cap-pipe-live-repro multi-process helper (both PASS). The blocker is cap-pipe REACHABILITY for the AppContainer principal, which Phase 62 deferred.
- **Reproduction (operator, real Win11 console, cwd %USERPROFILE%\.claude, dev-layout target\release\nono.exe):**
  - `target\release\nono.exe run --profile claude-code --allow-cwd -- target\release\examples\aipc-cap-child.exe sc2`
  - `target\release\nono.exe run --profile claude-code --allow-cwd -- target\release\examples\aipc-cap-child.exe sc1`
  - PASS = child connects; sc1 prints `SC1 RESULT: PASS`; sc2 supervisor bounds the read without hanging.

## Root-cause hypothesis (seeded — verify, do not assume)

Two layers; FIX 1 is the immediate blocker, FIX 2 is the next one likely exposed after FIX 1:

- The broker arm is **AppContainer (lowbox)**, NOT the legacy WRITE_RESTRICTED restricting-SID token. Broker log: `AppContainer profile registered app_container_name=nono.session.<guid>` then `app_container=true`. The AppContainer child runs as a DIFFERENT PRINCIPAL (the per-run package SID), per Phase 62 ([[windows_appcontainer_wfp_validated]]: "package SID needs explicit read/traverse grants — different principal than the user"). Phase 62 EXPLICITLY DEFERRED the full read-grant model for the AppContainer principal.
- **FIX 1 (immediate):** the rendezvous file `%TEMP%\.nono-<nonce>.json` is written by `write_pipe_rendezvous` (crates/nono/src/supervisor/socket_windows.rs ~1150) with the USER's default ACL. The AppContainer package SID has no read access → os error 5 BEFORE the pipe is ever touched. Grant the package SID FILE_GENERIC_READ on the rendezvous file (and FILE_TRAVERSE to %TEMP% if needed).
- **FIX 2 (next):** the cap-pipe DACL is built by `SupervisorSocket::bind_low_integrity_with_session_sid` (socket_windows.rs:226) using an SDDL scoped to the per-session RESTRICTING SID (`session_sid`; CAPABILITY_PIPE_SDDL ~63 + the appended `(A;;0x120089;;;<session_sid>)` ACE). On the AppContainer arm the connecting principal is the PACKAGE SID, NOT in that DACL → connect would still fail ERROR_ACCESS_DENIED after FIX 1. Also admit the AppContainer package SID on the cap-pipe DACL.

## CRUX — ANSWERED (verified against production code)

**Question:** Does the supervisor/broker KNOW the AppContainer package SID at the time it (a) writes the rendezvous file and (b) binds the cap pipe?

**Answer: YES — the package SID is known to the supervisor BEFORE both the cap-pipe bind and the rendezvous write. No reordering or cross-broker round-trip is required.**

Trace (all `cfg(target_os = "windows")`):

1. `crates/nono-cli/src/execution_runtime.rs:490-495` — the supervisor derives the package SID DETERMINISTICALLY and EARLY, at `ExecConfig` build time, BEFORE any spawn:
   - `windows_app_container_name = generate_app_container_name()` (the per-run `nono.session.<uuid>` moniker)
   - `windows_package_sid = package_sid_to_string(&derive_app_container_sid(&windows_app_container_name)?)?` — pure derivation from the name; FAIL-CLOSED via `?`.
2. `execution_runtime.rs:497-522` — that SID is stored in `ExecConfig.package_sid: Some(windows_package_sid)` (mod.rs:151), alongside `session_token` + `cap_pipe_rendezvous_path` (lines 511-512). Same single source the broker's `SECURITY_CAPABILITIES.AppContainerSid` derives from (same name → identical package SID).
3. `crates/nono-cli/src/exec_strategy_windows/mod.rs:347-375` (`prepare_live_windows_launch`) — the package SID is ALREADY consumed pre-spawn to grant write (`AppliedDaclGrantsGuard`) + ancestor-traverse (`AppliedAncestorTraverseGuard`) on filesystem grants. So it is concretely available at supervisor-setup time.
4. **The gap:** `SupervisorConfig` (mod.rs:190-232) carries ONLY `session_sid` (line 218), NOT `package_sid`. `supervised_runtime.rs:375-429` builds `SupervisorConfig` from `ExecConfig` and threads `session_sid: config.session_sid.clone()` (line 420) but DROPS `config.package_sid`. Consequently the cap-pipe server thread (`exec_strategy_windows/supervisor.rs:465-524`) binds via `bind_low_integrity_with_session_sid(&rendezvous_path, session_sid.as_deref())` (line 511-513) using the SYNTHETIC `S-1-5-117-*` restricting SID — which is on NO AppContainer child token — and the rendezvous file is written by `write_pipe_rendezvous` (socket_windows.rs:1150) with the user's default ACL (no package-SID ACE).

**Conclusion:** Both FIX 1 and FIX 2 are confirmed and FEASIBLE with no reordering. The package SID just needs to be (a) threaded into `SupervisorConfig` and on into the cap-pipe server thread, then (b) granted READ on the rendezvous file + admitted in the cap-pipe SDDL.

## Confirmed root cause (two layers)

- **Layer 1 (FIX 1 — the immediate os error 5):** `write_pipe_rendezvous` (socket_windows.rs:1168-1191) creates `%TEMP%\.nono-<nonce>.json` with `OpenOptions::create_new`, inheriting the user's default ACL only. The AppContainer child (package-SID principal, `S-1-15-2-*`) is NOT in that ACL → `read_pipe_rendezvous`'s `std::fs::read_to_string` (socket_windows.rs:1195) fails ERROR_ACCESS_DENIED before the pipe is ever opened. This is exactly the observed error string.
- **Layer 2 (FIX 2 — the next blocker after FIX 1):** even with the rendezvous readable, `connect` → `connect_named_pipe` opens the pipe with `GENERIC_READ | GENERIC_WRITE`. The pipe's SDDL (`build_capability_pipe_sddl`, socket_windows.rs:1458-1473) only appends ACEs for the synthetic `session_sid` and the current logon SID (both `0x0012019F`). The AppContainer child's access-check participant is the PACKAGE SID, which is in NEITHER ACE → ERROR_ACCESS_DENIED on `CreateFileW`. The package SID must also be admitted to the cap-pipe DACL.

## Proposed fix (shape — security-critical, fail-secure)

FIX 1 — rendezvous-file read grant (library + CLI guard):
- Add a `grant_sid_read_on_path(path, sid)` primitive in `crates/nono/src/sandbox/windows.rs` (new narrow READ-only mask, e.g. `FILE_GENERIC_READ` 0x00120089, NO write/delete, `NO_INHERITANCE` for a single file), mirroring `grant_sid_write_on_path` / `grant_sid_traverse_on_path` exactly (same `edit_dacl_for_sid` core, same fail-closed discipline, `revoke_sid_on_path` already covers revert). Export it from `lib.rs`.
- Add an RAII guard (e.g. `AppliedRendezvousReadGuard` in `dacl_guard.rs`) that, when `package_sid` is `Some`, grants the package SID READ on the rendezvous file after the cap-pipe server has created it, and revokes on Drop — mirror `AppliedDaclGrantsGuard`. The rendezvous file is user-owned (`%TEMP%`), so `WRITE_DAC` is available (watch the existing "writable path not owned by current user" skip pattern — here ownership SHOULD hold; fail-closed if not).
- ORDERING NOTE: the rendezvous file only exists AFTER `bind_low_integrity_with_session_sid` runs (it calls `write_pipe_rendezvous` inside `bind_impl`). So the read-grant must be applied AFTER the bind (inside or just after the cap-pipe server thread's bind) and before the child can connect. Cleanest: have the library's bind path grant the package-SID read on the rendezvous file when a package SID is supplied, OR expose the bound rendezvous path so the CLI guard can grant it. Pick whichever keeps the library policy-free (prefer CLI-side guard — pass the resolved rendezvous path back, or grant right after bind in the server thread using a CLI helper).

FIX 2 — cap-pipe DACL admit (library + CLI plumbing):
- Thread `package_sid: Option<String>` through `SupervisorConfig` (mod.rs:190) ← `supervised_runtime.rs:375` (`package_sid: config.package_sid.clone()`) → `WindowsSupervisorRuntime` field (supervisor.rs ~244, next to `session_sid`) → cap-pipe server thread closure (supervisor.rs:486-ish) → into the bind call.
- Extend the bind path to admit the package SID in the SDDL. Two options: (a) add an `(A;;0x0012019F;;;<package_sid>)` ACE in `build_capability_pipe_sddl` when a package SID is supplied (add a `package_sid: Option<&str>` param + a `validate_package_sid_for_sddl` that allow-lists the `S-1-15-2-` shape, mirroring `validate_session_sid_for_sddl` — SDDL-injection defense-in-depth, fail-closed); OR (b) a new `bind_low_integrity_with_package_sid` entry point. Option (a) is more uniform. The package SID participates in the FIRST DACL pass (it is the child token's identity), so a single allow-ACE with the `0x0012019F` mask suffices (the WRITE_RESTRICTED double-check is NOT in play on the AppContainer arm — that arm uses no restricting SID).
- NEVER widen the DACL beyond the specific per-run package SID; NEVER fall back to a null/world ACL; validate before embedding.

## Key files

- crates/nono/src/supervisor/socket_windows.rs — write_pipe_rendezvous (1150), read_pipe_rendezvous (1194), bind_impl (233) / bind_low_integrity_with_session_sid (226), create_low_integrity_named_pipe (1507), build_capability_pipe_sddl (1458) + CAPABILITY_PIPE_SDDL (63) + CAPABILITY_PIPE_RESTRICTING_SID_MASK (90), validate_session_sid_for_sddl (1264). FIX 2 SDDL ACE + new validate_package_sid_for_sddl go here.
- crates/nono/src/sandbox/windows.rs — edit_dacl_for_sid (1561), grant_sid_write_on_path (1702), grant_sid_traverse_on_path (1748), revoke_sid_on_path (1775), path_is_owned_by_current_user (1205), SESSION_SID_WRITE_MASK (1413) / PACKAGE_SID_TRAVERSE_MASK (1445). FIX 1 new grant_sid_read_on_path + READ mask go here.
- crates/nono-cli/src/exec_strategy_windows/mod.rs — ExecConfig.package_sid (151), SupervisorConfig (190; ADD package_sid), prepare_live_windows_launch package-SID grant precedent (347-375).
- crates/nono-cli/src/supervised_runtime.rs — SupervisorConfig build (375-429; ADD package_sid: config.package_sid.clone()).
- crates/nono-cli/src/exec_strategy_windows/supervisor.rs — WindowsSupervisorRuntime fields (237-286; ADD package_sid), initialize copy (375-390), start_capability_pipe_server (465-524; clone package_sid into thread + pass to bind + apply rendezvous read-grant guard).
- crates/nono-cli/src/execution_runtime.rs — package SID derivation (490-495), ExecConfig build (497-522).
- crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs — AppliedDaclGrantsGuard / AppliedAncestorTraverseGuard precedent; ADD AppliedRendezvousReadGuard (FIX 1).
- crates/nono-cli/examples/aipc-cap-child.exe — repro harness (sc2/sc1).

## Constraints

- SECURITY-CRITICAL: the cap-pipe SDDL/DACL is the capability-expansion trust boundary. Fail-secure — validate the package SID before embedding in any SDDL (mirror validate_session_sid_for_sddl, allow-list the `S-1-15-2-` shape); never widen the DACL beyond the specific per-run package SID; never fall back to a null/world-readable ACL. Grant the rendezvous file read to the package SID ONLY, not broadly. Revert grants on Drop (mirror AppliedDaclGrantsGuard / AppliedAncestorTraverseGuard).
- WindowsToken-host ops: SetNamedSecurityInfoW / DACL edit needs WRITE_DAC; the %TEMP% rendezvous file is user-owned so WRITE_DAC is available (unlike the claude.json case — watch the "writable path not owned by current user" dacl-guard skip pattern).
- Verify on this Windows host where possible. Cross-target Unix clippy N/A (cfg(windows) files). Phase 59 stays OPEN until aipc-cap-child sc2 + sc1 PASS under real `nono run`.

## Current Focus

- hypothesis: CONFIRMED (both layers); cycle-1 (ordering inversion) and cycle-2 (grant moved inside bind_impl) both deployed. Cycle-2 still produced os error 5 with NO "Failed to bind capability pipe" error, meaning bind_impl returned Ok AND the grant ran Ok (or package_sid was None at bind). Static tracing of the code path says package_sid should reach bind as Some — but runtime disagrees.
- next_action: OPERATOR RUN-ONCE — run sc2 from a real PowerShell console (see repro commands below) with the cycle-3 diagnostic binary. Paste the full output including all `DIAG[appcontainer-cap-pipe]` lines. This discriminates two hypotheses:
  - **(A) package_sid is None at bind** — the `DIAG[appcontainer-cap-pipe]: NO package_sid at bind` line will appear, or the `cap-pipe server starting; package_sid_present=false` line; root cause is in the SupervisorConfig/WindowsSupervisorRuntime threading (the field is wired but may be None at runtime due to profile condition).
  - **(B) grant ran Ok but is insufficient** — the `rendezvous read-grant OK for <sid>` line will appear, and the child still sees os error 5; root cause is a DACL/traverse/inheritance issue (e.g. %TEMP% traverse not granted, or the inherited file ACL is overridden, or the child's access-check uses a different principal than the package SID we granted).

## Evidence

- timestamp: 2026-06-06 (cycle-3) — INSTRUMENTED; awaiting operator DIAG output. Cycle-2 relocation STILL failed with `Access is denied (os error 5)` reading the rendezvous file and NO "Failed to bind capability pipe" error. That error path only fires when `bind_low_integrity_with_session_and_package_sid` returns Err — it returned Ok. So either: (A) package_sid was None at bind (grant never ran), or (B) grant ran and returned Ok but was insufficient (traverse/DACL/principal mismatch). Six `DIAG[appcontainer-cap-pipe]:` `tracing::warn!` lines added at `start_capability_pipe_server` (package_sid clone site) and inside `bind_impl` (rendezvous written, before grant, grant Ok, grant Err, and None skip path). Operator must run sc2 once (see repro commands) with the cycle-3 `target\release\nono.exe` and paste the full DIAG output. Commit: `83fde87b`.

- timestamp: 2026-06-06 — Live Win11 run log: `broker: AppContainer profile registered app_container_name=nono.session.<guid>`; `broker: token/AppContainer setup complete app_container=true`; `broker: spawned child app_container=true`; then child prints `sc2: connect failed: ...Failed to read Windows supervisor pipe rendezvous C:\Users\OMack\AppData\Local\Temp\.nono-cb90a1393c9fcfd0.json: Access is denied. (os error 5)`; `broker: child exited child_exit_code=1`. Identical for sc1 conn1. Earlier in the same run: `dacl guard: writable path not owned by current user; skipping session-SID DACL grant ... path=C:\Users\OMack\.claude\claude.json` (shows the dacl-guard skip pattern is already active for non-owned paths).
- timestamp: 2026-06-06 — CODE TRACE (CRUX resolution): package SID derived at execution_runtime.rs:490-495 (derive_app_container_sid + package_sid_to_string, fail-closed), stored ExecConfig.package_sid (mod.rs:151), already consumed pre-spawn for FS write/traverse grants (mod.rs:347-375). SupervisorConfig (mod.rs:190-232) carries session_sid (218) but NOT package_sid; supervised_runtime.rs:420 threads session_sid only, dropping package_sid. Cap-pipe server (supervisor.rs:511-513) binds with the synthetic session_sid; rendezvous written by write_pipe_rendezvous (socket_windows.rs:1168-1191) with user-default ACL; SDDL (socket_windows.rs:1458-1473) admits only session_sid + logon SID. Package SID is in neither the rendezvous ACL nor the pipe DACL. CONFIRMS both fix layers; package SID is known pre-bind.
- timestamp: 2026-06-06 16:24/16:27 — CYCLE-1 LIVE RE-TEST STILL FAILED (os error 5): the same `Access is denied` error on the rendezvous file occurred AFTER deploying the cycle-1 fix. Root-caused to ordering inversion: cycle-1's `AppliedRendezvousReadGuard` was applied in `supervisor.rs` ~line 540 AFTER `bind_low_integrity_with_session_and_package_sid` returned. But `bind_impl` calls `write_pipe_rendezvous` then immediately calls `finalize_server_connection` which calls `ConnectNamedPipe` — which BLOCKS until the child connects. The child cannot connect until it has read the rendezvous file. So the child attempted the read (before the grant), received `ERROR_ACCESS_DENIED`, and exited — only then did `ConnectNamedPipe` return, `bind_impl` return, and the (now-useless) grant run. The grant was effectively dead code in the cycle-1 placement.
- timestamp: 2026-06-06 — SID IDENTITY CONFIRMED: supervisor package SID == broker child package SID. Both are derived via `derive_app_container_sid(app_container_name)` from the same `ExecConfig.windows_app_container_name` string. Only the TIMING of the grant was wrong, not the SID itself.

## Eliminated

- NOT a regression of Phase 59-03 (read_frame bounded-read / disconnect_and_reconnect) — those are integration-test + live-repro verified. The blocker is cap-pipe REACHABILITY for the AppContainer package-SID principal (a Phase 62 deferral), not the IPC robustness code.
- NOT a "supervisor created the channel too late" ordering bug (the error string's hint is misleading here): the channel exists; the AppContainer principal simply lacks READ on the rendezvous file. The package SID IS known before bind, so no reorder/broker-round-trip is needed.
- FIX 1 PLACED AFTER bind() IS INEFFECTIVE (CYCLE-1 INVERSION — CONFIRMED): `bind_impl` calls `ConnectNamedPipe` inside `finalize_server_connection`, which blocks until the child connects. The child reads the rendezvous to learn the pipe name; without the READ grant it fails immediately. Applying the grant AFTER `bind_impl` returns (as cycle-1's `AppliedRendezvousReadGuard` did) is therefore dead code — the child has already failed and exited before the grant runs.
- SUPERVISOR PACKAGE SID == BROKER CHILD PACKAGE SID (CONFIRMED): both are derived from the same `app_container_name` via `derive_app_container_sid`. The SID identity was never the issue; only the timing of the grant was wrong.

## Resolution

**Root cause (confirmed — both layers):**
- **Layer 1:** `write_pipe_rendezvous` creates `%TEMP%\.nono-<nonce>.json` with the user's default ACL. The AppContainer child (package-SID principal, `S-1-15-2-*`) has no read right → `ERROR_ACCESS_DENIED` (os error 5) in `SupervisorSocket::connect` → `read_pipe_rendezvous` → `std::fs::read_to_string`.
- **Layer 2:** Even after the rendezvous is readable, `build_capability_pipe_sddl` only included ACEs for the synthetic session restricting SID and the logon SID. The AppContainer child's access-check principal is the PACKAGE SID, absent from the pipe DACL → `ERROR_ACCESS_DENIED` on `CreateFileW(pipe, GENERIC_READ|GENERIC_WRITE)`.

**FIX 1 applied (cycle-2 ordering fix) — rendezvous-file READ grant:**
- New symbol: `grant_sid_read_on_path(path, sid)` in `crates/nono/src/sandbox/windows.rs` (new `PACKAGE_SID_READ_MASK` const = `FILE_GENERIC_READ` 0x00120089, `NO_INHERITANCE`, `edit_dacl_for_sid` core). Exported from `crates/nono/src/lib.rs`. Primitive and its tests retained in `sandbox/windows.rs`.
- **Cycle-2 relocation:** The grant is now applied inside `bind_impl` (socket_windows.rs), AFTER `write_pipe_rendezvous` (rendezvous file now exists on disk) and BEFORE `finalize_server_connection` (the blocking `ConnectNamedPipe`). The package SID is validated via `validate_package_sid_for_sddl` before the grant (fail-closed: on validation or grant error, the server handle is reclaimed and the error propagated — we never proceed to `ConnectNamedPipe` with an ungranted rendezvous on the AppContainer arm). Revert: the rendezvous file is deleted on `SupervisorSocket::Drop` via `cleanup_rendezvous_path → std::fs::remove_file`; file deletion destroys the leaf ACE with the file (no separate revoke needed). `None` package_sid → no grant, unchanged behavior for all existing callers.
- **Cycle-1 dead code removed:** The `AppliedRendezvousReadGuard` application in `supervisor.rs` (after `bind_low_integrity_with_session_and_package_sid` returns) has been replaced with a comment explaining the ordering fix. `AppliedRendezvousReadGuard` struct and its two tests removed from `dacl_guard.rs` (unused per CLAUDE.md § dead_code); `grant_sid_read_on_path` and `NonoError` removed from dacl_guard.rs production imports.

**FIX 2 applied — admit package SID in cap-pipe DACL:**
- New symbol: `validate_package_sid_for_sddl(sid)` in `socket_windows.rs` — allow-lists the `S-1-15-2-` prefix (AppContainer IL=15, base=2); rejects injection/garbage/over-length (same fail-closed discipline as `validate_session_sid_for_sddl`). New constant `PACKAGE_SID_MAX_LEN = 192`.
- `build_capability_pipe_sddl` extended with `package_sid: Option<&str>` parameter: validates before embedding; when `Some(valid)`, appends `(A;;0x0012019F;;;<package_sid>)` ACE (same `CAPABILITY_PIPE_RESTRICTING_SID_MASK`) before the SACL.
- `build_low_integrity_security_attributes` and `create_low_integrity_named_pipe` extended with `package_sid` parameter.
- `bind_impl` extended with `package_sid: Option<&str>` parameter.
- Back-compat entry point `bind_low_integrity_with_session_sid` preserved (passes `None` as `package_sid` — byte-identical pre-fix behavior for all existing callers).
- New entry point `bind_low_integrity_with_session_and_package_sid(path, session_sid, package_sid)` — used exclusively by the cap-pipe server thread.
- `package_sid: Option<String>` field added to `SupervisorConfig` (mod.rs) and `WindowsSupervisorRuntime` (supervisor.rs); threaded from `ExecConfig.package_sid` via `supervised_runtime.rs`; cloned into cap-pipe server thread closure; passed to `bind_low_integrity_with_session_and_package_sid`.

**Files changed (cycle-1, unchanged):**
- `crates/nono/src/sandbox/windows.rs` — `PACKAGE_SID_READ_MASK`, `grant_sid_read_on_path`, tests `grant_read_then_revoke_sid_round_trips_on_tempfile`, `grant_read_invalid_sid_fails_closed` (RETAINED)
- `crates/nono/src/lib.rs` — export `grant_sid_read_on_path` (RETAINED)
- `crates/nono/src/supervisor/socket_windows.rs` — `PACKAGE_SID_MAX_LEN`, `validate_package_sid_for_sddl`, `build_capability_pipe_sddl` extended, `bind_low_integrity_with_session_and_package_sid` added; tests for package-SID SDDL (RETAINED)
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — `SupervisorConfig.package_sid: Option<String>` field (RETAINED; doc updated)
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — `WindowsSupervisorRuntime.package_sid` field; `start_capability_pipe_server` uses `bind_low_integrity_with_session_and_package_sid` (RETAINED; doc updated)
- `crates/nono-cli/src/supervised_runtime.rs` — `package_sid: config.package_sid.clone()` (RETAINED)

**Files changed (cycle-2 ordering fix):**
- `crates/nono/src/supervisor/socket_windows.rs` — `bind_impl`: grant added AFTER `write_pipe_rendezvous` and BEFORE `finalize_server_connection` (blocking `ConnectNamedPipe`); uses `validate_package_sid_for_sddl` + `crate::sandbox::windows::grant_sid_read_on_path`; fail-closed (handle reclaim + error propagation on validation/grant failure).
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — removed dead-code `AppliedRendezvousReadGuard` application (replaced with comment explaining the ordering fix and pointing to `bind_impl`); updated `WindowsSupervisorRuntime.package_sid` doc.
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — updated `SupervisorConfig.package_sid` doc (FIX 1 now in `bind_impl`, not in CLI guard).
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` — removed `AppliedRendezvousReadGuard` struct + `impl` + `Drop` (unused in production code after relocation); removed associated imports (`grant_sid_read_on_path`, `NonoError`); removed 2 tests (`rendezvous_read_guard_*`). `grant_sid_read_on_path` primitive and its tests in `sandbox/windows.rs` RETAINED.

**Verification (cycle-2, this host — Windows 11 26200):**
- `cargo build -p nono -p nono-cli` — PASS
- `cargo clippy -p nono --all-targets -- -D warnings -D clippy::unwrap_used` — PASS (0 warnings)
- `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used` — PASS (0 warnings)
- `cargo test -p nono --lib supervisor::socket` — PASS (30/30)
- `cargo test -p nono --lib dacl_grant_tests` — PASS (6/6, includes 2 `grant_sid_read_on_path` tests)
- `cargo test -p nono-cli --bin nono dacl_guard` — PASS (5/5, 2 removed `AppliedRendezvousReadGuard` tests no longer present)
- `cargo test -p nono-cli --test aipc_handle_brokering_integration` — PASS (5/5)
- `cargo test -p nono-cli --test supervisor_ipc_robustness_windows` — PASS (4/4)
- Cross-target Unix clippy: N/A — changed files are all `cfg(target_os = "windows")` gated; marked PARTIAL per `.planning/templates/cross-target-verify-checklist.md`, deferred to live CI.
- Cycle-1 commits: `109ffc78` (FIX 1), `ece9b6dc` (FIX 2)
- Cycle-2 operator binaries rebuilt: `target\release\nono.exe`, `target\release\examples\aipc-cap-child.exe`

**OPERATOR VERIFICATION PENDING**

The full `aipc-cap-child sc2`/`sc1` PASS requires a real Win11 console under the broker/AppContainer arm — NOT automatable from this context.

**Exact repro commands** (run from `%USERPROFILE%\.claude` in a real PowerShell console using dev-layout binaries):

```powershell
# sc2: supervisor bounded-read test — child connects, sends oversized frame, supervisor reads bounded fragment, no hang
target\release\nono.exe run --profile claude-code --allow-cwd -- target\release\examples\aipc-cap-child.exe sc2

# sc1: full AIPC round-trip — child connects, sends RequestCapability, supervisor approves/denies, child prints result
target\release\nono.exe run --profile claude-code --allow-cwd -- target\release\examples\aipc-cap-child.exe sc1
```

**PASS criteria:**
- sc2: no `Access is denied (os error 5)` in child output; supervisor log shows `cap-pipe: bounded read` or similar; child exits 0.
- sc1: child prints `SC1 RESULT: PASS`; supervisor log shows capability request handled; child exits 0.

**If the repro still fails:** check that `target\release\nono.exe` is the post-fix binary (built after commit `ece9b6dc`). If a new `os error 5` appears on the PIPE (not the rendezvous file), a third DACL layer may be present — open a new debug session.
