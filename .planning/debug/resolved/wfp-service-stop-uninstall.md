---
slug: wfp-service-stop-uninstall
status: resolved
trigger: "Windows: nono WFP service cannot be stopped (stop returns an error), AND the MSI uninstall from Apps & Features completes but leaves files/service behind. Likely one linked root cause."
created: 2026-05-27
updated: 2026-05-27
---

# Debug Session: wfp-service-stop-uninstall

## Symptoms

**Expected behavior:**
- Stopping the WFP service (`sc stop nono-wfp-service` / Services.msc) cleanly stops it.
- Uninstalling nono from the Windows "Apps & Features" menu (MSI uninstall) stops + deletes
  the service, removes the install dir + binaries, and removes the Apps & Features entry — no
  leftovers, no reboot required.

**Actual behavior (user-confirmed 2026-05-27):**
- **Service stop RETURNS AN ERROR** (fast fail with an error code/message — NOT a hang on
  "Stopping").
- **MSI uninstall COMPLETES BUT LEAVES FILES/SERVICE behind** — reports success, yet the
  install dir / binary / service registration remains.

**Error messages / codes:** exact `sc stop` error text/code not yet captured. Code analysis
predicts SCM error 1053 (ERROR_SERVICE_REQUEST_TIMEOUT) presenting fast (see root cause).

**Platform:** Windows 11 Enterprise build 26200. nono v0.57.3. Windows WFP backend + Job Objects.

## Goal

Find the root cause, then propose/apply a fix after a checkpoint. Find-and-fix mode.
SECURITY: WFP service is a kernel-level network-enforcement component — any stop/teardown
fix must preserve fail-secure semantics (no filter leak on stop failure, no half-applied
enforcement) and must not weaken the install/uninstall trust posture.

## Current Focus

- hypothesis: The two symptoms are TWO related defects in the stop/teardown path. (1) The
  service control handler ACKs STOP but never transitions to STOPPED and never breaks its
  infinite pipe-accept loop, so SCM stop fails. (2) Because WiX `ServiceControl Stop="both"
  Wait="yes"` cannot stop the (broken) service on uninstall, the running host process keeps
  the binary locked → MSI cannot delete it (delete-pending → reboot leftover); additionally
  the post-install-registered kernel driver service (`nono-wfp-driver`) has NO uninstall
  counterpart anywhere in the codebase, so it persists regardless.
- next_action: Present root cause + ranked fix options at the fix checkpoint; get approval.
- test: (deferred to fix) cargo build/clippy on Windows host; live stop/uninstall requires
  elevated PowerShell run by the user.
- expecting: A stop-channel + STOP_PENDING state transition makes `sc stop` succeed; adding
  driver-service removal on uninstall closes the leftover.

## Evidence

- timestamp: 2026-05-27 — User confirmed: service stop RETURNS AN ERROR (not a hang); MSI uninstall COMPLETES but LEAVES files/service behind.
- timestamp: 2026-05-27 — CODE: `crates/nono-cli/src/bin/nono-wfp-service.rs` `run_service()` (L489-537). The control handler (L490-496) handles `ServiceControl::Stop` by returning `ServiceControlHandlerResult::NoError` ONLY — it does NOT signal the pipe loop to exit and does NOT call `set_service_status(Stopped)`. The main thread is blocked forever in `rt.block_on(run_named_pipe_server())` (L520-524), which is an infinite `loop {}` (L551-664) with no shutdown signal. The `set_service_status(Stopped)` at L526-534 is therefore unreachable. `wait_hint: Duration::default()` (=0) at L506 means SCM gets no time hint → SCM reports a STOP failure/timeout fast rather than sitting in STOP_PENDING. ROOT CAUSE for symptom #1.
- timestamp: 2026-05-27 — CODE: WiX service control is CORRECT in definition. `scripts/build-windows-msi.ps1` (L221-246, the real MSI source) and the reference snapshot `dist/windows/nono-machine.wxs` (L93-100) both emit `<ServiceControl ... Start="install" Stop="both" Remove="uninstall" Wait="yes" />`. So MSI removal of the *user-mode* service registration is wired — but `Wait="yes"` means uninstall blocks on the SCM stop, which fails per the above. A running service host holds `nono-wfp-service.exe` open → MSI delete becomes delete-pending → file/dir leftover needing reboot. CHAINED CAUSE for symptom #2.
- timestamp: 2026-05-27 — CODE: The kernel DRIVER service (`nono-wfp-driver`) is registered POST-install via `sc create ... type= kernel` (`exec_strategy_windows/network.rs` L253-295 build args; `setup.rs` L200-218 `install_windows_wfp_driver`). WiX deliberately does NOT model it (comment L277-283 in the ps1 / L101-108 in the wxs). Grep for `fn (stop|delete|uninstall|remove)_windows_wfp` across `crates/nono-cli/src` returns ZERO matches — there is NO uninstall/delete counterpart for either the driver service OR a CLI-registered user-mode service. So even after a clean MSI uninstall, a driver service registered by `nono setup --install-wfp-driver` persists. SECOND INDEPENDENT GAP for symptom #2.
- timestamp: 2026-05-27 — CODE: `build_wfp_service_create_args` (network.rs L253-266) registers the user-mode service with `binPath= "<svc.exe>" --service-mode`, `type= own`, `start= demand` — identical contract to the MSI's `<ServiceInstall>`. Confirms a CLI `--install-wfp-service` and the MSI register the SAME service name with the SAME broken host binary, so the stop defect applies to both registration paths.

## Eliminated

- "Stop hangs indefinitely in STOP_PENDING" — ELIMINATED by user report (stop returns an error fast). Consistent with `wait_hint=0` causing SCM to report failure immediately rather than wait.
- "WiX ServiceControl Stop/Remove not configured" — ELIMINATED. The WiX IS configured correctly (`Stop="both" Remove="uninstall" Wait="yes"`). The defect is the service binary never honoring the stop, not a missing WiX directive.

## Resolution

**Decision (user, 2026-05-27):** "Fix both now", then at the discovered-scope re-checkpoint:
"2a now, flag 2b".

**Fix #1 — service stop handler (commit `0cbeb3be`).** `crates/nono-cli/src/bin/nono-wfp-service.rs`:
added a `tokio::sync::Notify` shutdown signal. The `SERVICE_CONTROL_STOP` handler now calls
`notify_one()` (permit-stored, no lost-wakeup race) and the named-pipe accept loop `select!`s
(biased) on it around `connect()`, breaking out to reach the previously-unreachable `Stopped`
transition. On break the dynamic-session WFP engine drops, removing the `nono` sublayer —
fail-secure teardown (no filters left applied). Enabled tokio's `sync` feature.
**This fixes BOTH reported symptoms:** `sc stop` now succeeds, and because the MSI's
`ServiceControl Stop="both" Remove="uninstall" Wait="yes"` can now stop the service, the
user-mode service + its binary are removed instead of left delete-pending.

**Fix #2a — driver-service removal command (commit `b852826b`).** Added
`nono setup --uninstall-wfp`: stops (best-effort) + deletes both the `nono-wfp-driver` kernel
service (which has no WiX representation) and the `nono-wfp-service` user-mode service. Only the
two well-known nono-owned service names are ever touched (cannot delete an unrelated service);
idempotent; requires elevated admin. New args builders + `uninstall_windows_wfp_with_runner` +
public wrapper + `WindowsWfpUninstallReport` + 4 mock-runner unit tests, wired through
`setup.rs` (early short-circuit) and `cli.rs`.

**Fix #2b — DEFERRED (flagged follow-up).** A WiX custom action to invoke `--uninstall-wfp`
automatically during MSI uninstall was NOT authored: it runs at elevation during uninstall and
cannot be validated from the non-elevated git-bash shell; authoring it blind risks regressing
uninstall worse than today. Recommended as a small follow-up executed WITH an elevated
live-uninstall test. Until then, the driver service (only present if the user ran
`nono setup --install-wfp-driver`) is removed via the new `nono setup --uninstall-wfp` command.

- root_cause: see Root Cause / Evidence above — control handler never signalled the infinite
  accept loop, so STOPPED was unreachable (SCM fast-fail); chained to the MSI file leftover.
  Plus the independent gap: no removal path for the post-install `sc create`-registered driver.
- fix: Fix #1 (`0cbeb3be`) + Fix #2a (`b852826b`); Fix #2b deferred.
- verification: build green; 4 new unit tests pass; production clippy clean
  (`-D warnings -D clippy::unwrap_used`). LIVE verification (elevated `sc stop nono-wfp-service`
  + MSI uninstall leaves nothing) is the user's to run from an elevated PowerShell — the
  Claude Code Bash tool is non-elevated MSYS. Pre-existing `--tests` clippy debt
  (`offline_verify_extended_trust_bundle.rs` unwrap_err, `profile/mod.rs` oauth2_cred_builder
  dead_code) is unrelated to this fix and untouched.
- files_changed:
  - crates/nono-cli/Cargo.toml (tokio `sync` feature)
  - crates/nono-cli/src/bin/nono-wfp-service.rs (Fix #1: wakeable stop)
  - crates/nono-cli/src/exec_strategy_windows/mod.rs (report struct + re-export)
  - crates/nono-cli/src/exec_strategy_windows/network.rs (Fix #2a: removal fns + tests)
  - crates/nono-cli/src/setup.rs (--uninstall-wfp wiring)
  - crates/nono-cli/src/cli.rs (--uninstall-wfp flag)

**Follow-up to track:** Fix #2b (WiX auto-uninstall custom action) + the elevated live-uninstall
UAT for both fixes.
