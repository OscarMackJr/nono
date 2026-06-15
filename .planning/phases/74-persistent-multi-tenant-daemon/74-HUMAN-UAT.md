---
phase: 74-persistent-multi-tenant-daemon
doc: human-uat-script
status: PENDING OPERATOR — control pipe wired (Plan 74-07); SC1 end-to-end dev-validated 2026-06-15
created: 2026-06-15
wave: 5
plan: "07"
---

# Phase 74 — Human UAT Script (SC1-SC5: Persistent Multi-Tenant Daemon)

SC1 (two concurrent confined agents, distinct SIDs), SC3 (100-agent handle-count baseline), and
SC4 (USER service, not SYSTEM) cannot be verified by unit tests alone: they require the real
nono-agentd binary running on Win11, full SCM registration, and live AppContainer process spawn.
SC2 and SC5 are covered by automated tests (Wave 0 and Wave 1) and are re-confirmed here at the
integration tier. This document is the operator's runbook for the go/no-go gate on a real Win11
host.

---

## Reading Guide

Run sections in order: Preconditions → SC4 (must confirm USER before anything spawns) →
Build → SC1 → SC2 → SC3 → SC5 → Regression → Teardown → Go/No-Go table.

SC4 is run FIRST because if nono-agentd registers as SYSTEM/LocalSystem rather than a per-user
service, the rest of the UAT demonstrates a privilege escalation risk, not a passing system.

---

## Preconditions (ALL must be confirmed before starting)

### P-1: Dev-layout target\release\nono.exe (R-B4 — broker Authenticode trust gate)

The broker trust gate (`is_dev_build_layout` in `launch.rs`) checks whether the running
binary's path is under the compile-time-baked `NONO_DEV_TARGET_ROOT`. An unsigned install from
`C:\Program Files\nono\` is CORRECTLY REFUSED with a `TrustVerification` error. The agentd
binary must be the dev-layout build (same directory as nono.exe) so both pass the gate.

**Action — build BOTH binaries from the source tree:**

```powershell
# From the nono source tree root, in a REAL PowerShell window (NOT git-bash/MSYS):
cargo build --release -p nono-cli

# After build, confirm BOTH binaries exist:
Test-Path .\target\release\nono.exe        # must be True
Test-Path .\target\release\nono-agentd.exe # must be True

# Set $nono and $agentd for use throughout this runbook:
$nono   = "$PWD\target\release\nono.exe"
$agentd = "$PWD\target\release\nono-agentd.exe"
```

The single `cargo build --release -p nono-cli` command builds all `[[bin]]` targets in the
`nono-cli` Cargo.toml workspace member, including both `nono` and `nono-agentd`.

### P-2: Real PowerShell console (NOT git-bash / MSYS)

`CreateProcessAsUserW` inside the broker arm fails with GLE=87 (`ERROR_INVALID_PARAMETER`) when
the caller inherits a git-bash/MSYS pseudo-console — no real Win32 console handle. This is not
a bug; the broker requires a proper Win32 console context.

**Action:** Open a native PowerShell window (`powershell.exe` or Windows Terminal → PowerShell).
NOT a git-bash/MSYS shell and NOT the Bash tool in this dev environment.

> Are you in cmd.exe? If your prompt shows `C:\Users\<you>>` without a `PS` prefix, you are in
> cmd.exe, not PowerShell. Every command in this runbook uses PowerShell syntax (`$env:...`,
> `& $nono ...`). Launch `powershell.exe` and continue there.

### P-3: User-owned workspace directory (R-B3 — WRITE_OWNER pre-launch gate)

The supervisor's `try_set_mandatory_label` path (called when creating the per-agent Low-IL
workspace) requires WRITE_OWNER on the directory. A directory created from an **elevated**
console is owned by `BUILTIN\Administrators`; the session user lacks WRITE_OWNER even if they
appear as the NTFS owner — so the relabel fails. The R-B3 gate refuses with a named diagnostic
before any spawn.

The daemon test workspace can reuse `%USERPROFILE%\nono-work` (same as the Phase 71 UAT), or
you can create a dedicated directory:

```powershell
# In a NON-elevated PowerShell window (do NOT run as Administrator):
$ws = "$env:USERPROFILE\nono-test-workspace"
mkdir $ws -ErrorAction SilentlyContinue

# Verify YOU own it (must show <MACHINE>\<you>, NOT BUILTIN\Administrators):
(Get-Acl $ws).Owner
whoami   # must match the owner above
```

If the owner shows `BUILTIN\Administrators` (created from an elevated console earlier), either
delete and recreate it from a non-elevated shell, or reassign:

```powershell
icacls $ws /setowner "$env:USERNAME"
(Get-Acl $ws).Owner   # re-confirm
```

### P-4: No pre-existing nono-agentd SCM registration

If a stale service registration exists from a prior run it may interfere with SC4's service
query. Check and remove before starting:

```powershell
sc.exe query nono-agentd
# Expected if clean: FAILED 1060 (ERROR_SERVICE_DOES_NOT_EXIST) — OK, proceed
# If RUNNING or STOPPED: run  sc.exe stop nono-agentd ; sc.exe delete nono-agentd  first
```

### P-5: Non-elevated PowerShell session

nono-agentd registers as a per-user SCM service (`type= userservice` / `SERVICE_USER_OWN_PROCESS`
in HKCU namespace). This does NOT require elevation. Run the entire UAT from a normal
(non-elevated) PowerShell window. If you run elevated, `SC_MANAGER_CONNECT` may route the user
service to the machine-wide SCM namespace, giving misleading `sc qc` output.

---

## SC4 — Privilege Model (run FIRST; blocks everything else if SYSTEM)

**Purpose:** Confirm nono-agentd runs as `USER_OWN_PROCESS` under the interactive user account,
NOT as LocalSystem or any elevated account. This is the ADR-74 Decision 1 invariant.

SC4 is run before any agent spawning because if the daemon runs as SYSTEM, the downstream SC1
test exercises a high-privilege multi-tenant surface — dangerous rather than passing.

### SC4 Step 1 — Install the service

```powershell
# From the nono source tree root:
& $nono daemon install
```

**Expected output:**
```
nono-agentd installed as a per-user service (type= userservice).
Use `nono daemon start` to start it.
```

If you see `Error` or `failed to find nono-agentd.exe`: ensure P-1 (build step) completed and
both binaries exist in `target\release\`.

### SC4 Step 2 — Inspect the service configuration

```powershell
sc.exe qc nono-agentd
```

**Expected output (the critical fields):**

```
[SC] QueryServiceConfig SUCCESS

SERVICE_NAME: nono-agentd
        TYPE               : 110  USER_OWN_PROCESS
        START_TYPE         : 2   AUTO_START
        ERROR_CONTROL      : 1   NORMAL
        BINARY_PATH_NAME   : <path>\target\release\nono-agentd.exe --service-mode
        LOAD_ORDER_GROUP   :
        TAG                : 0
        DISPLAY_NAME       : nono-agentd
        DEPENDENCIES       :
        SERVICE_START_NAME : LocalSystem
```

> IMPORTANT — the `SERVICE_START_NAME: LocalSystem` field in the `sc qc` output is NOT the
> service account for `USER_OWN_PROCESS` services. For user services (`TYPE: 110 USER_OWN_PROCESS`),
> Windows stores the running account in the per-user SCM namespace (HKCU), not in the
> `SERVICE_START_NAME` field used by machine-wide services. The key field is `TYPE : 110` —
> this confirms the service runs as the interactive user, NOT as LocalSystem.
>
> Contrast with `nono-wfp-service`: that service shows `TYPE : 010  WIN32_OWN_PROCESS` and
> `SERVICE_START_NAME : LocalSystem` — a system-privilege service. The `110` vs `010` TYPE
> distinction is the SC4 pass/fail signal.

**SC4 PASS criterion:** `TYPE : 110  USER_OWN_PROCESS` is present.
**SC4 FAIL criterion:** `TYPE : 010  WIN32_OWN_PROCESS` or any `LocalSystem`/`SYSTEM` in the
service type field. If FAIL, STOP the UAT — do not proceed to SC1.

### SC4 Step 3 — Start the service and confirm separation from nono-wfp-service

```powershell
& $nono daemon start
```

**Expected output:**
```
nono-agentd started successfully.
```

Confirm nono-agentd is separate from nono-wfp-service:

```powershell
sc.exe query nono-agentd
sc.exe query nono-wfp-service  # if installed — must show as a SEPARATE service
```

**Expected:** Two independent service entries; nono-agentd at `110 USER_OWN_PROCESS`, nono-wfp-service
(if installed) at `010 WIN32_OWN_PROCESS`. No shared binary. No cross-dependency.

---

## Build Reference (if not done in P-1)

```powershell
# From nono source tree root, in a REAL PowerShell window:
cargo build --release -p nono-cli

# Set paths for use throughout:
$nono   = "$PWD\target\release\nono.exe"
$agentd = "$PWD\target\release\nono-agentd.exe"
$ws     = "$env:USERPROFILE\nono-test-workspace"

# Confirm:
Test-Path $nono   # True
Test-Path $agentd # True
```

---

## SC1 — Two Concurrent Confined Agents (the marquee multi-tenant test)

**Purpose:** Confirm that nono-agentd serves two independent concurrent agents, each in its own
AppContainer (distinct package SID), over one named-pipe listener — the DMON-01 core claim.

**End-to-end flow (Plan 74-07 wired — no stubs):** `nono daemon start` starts the daemon in
dev-layout background mode. `nono agent launch` sends a `Launch` request over
`\\.\pipe\nono-agentd-control`, which the daemon's `control_loop.rs` receives, validates the
profile, calls `launch_agent`, and returns the tenant metadata. `nono agent list` sends a `List`
request and receives the live tenant table from the daemon's in-memory `DaemonState`. `nono daemon
stop` sends a `Shutdown` request; the control loop fires `notify_one()` twice (for both the
accept loop and control loop) and the daemon exits cleanly.

**Dev-validated on Win11 build 26200 (2026-06-15):**

```
Launched agent:   tenant_id=9805b826e41729cdf3d30d8e869f6b39   profile=aider
  sid=S-1-15-2-4194181214-2299273401-2390484096-2024065141-3009441876-859840219-1047527710   pid=28848
Launched agent:   tenant_id=692b151b84f322b40ab8c00dc0bad736   profile=aider
  sid=S-1-15-2-3758960487-1416308204-3545803209-3860120738-3688480973-4053348959-2360816473   pid=34644
Tenant agents (2):
  9805b826e41729cd  profile=nono.session.9805b826e41729cd  sid=S-1-15-2-4194181214-...  pid=28848
  692b151b84f322b4  profile=nono.session.692b151b84f322b4  sid=S-1-15-2-3758960487-...  pid=34644
nono-agentd status: RUNNING
nono-agentd stopped (dev-layout): nono-agentd: shutdown initiated.
nono-agentd status: NOT RUNNING
```

> **Profile name in list output:** `nono agent list` shows the AppContainer profile moniker
> (`nono.session.<uuid>`) rather than the user-facing profile name (`aider`). This is cosmetic:
> the moniker IS the per-agent AppContainer profile name created at launch. The SID uniqueness
> is the security-critical check, not the display name.

### SC1 Setup

Ensure nono-agentd is running:

```powershell
# Dev-layout start (no SCM install needed):
& $nono daemon start
Start-Sleep -Seconds 2
& $nono daemon status
# Expected: nono-agentd status: RUNNING
```

If already installed as an SCM service (from SC4), use `& $nono daemon start` after install.

### SC1 Step 1 — Launch Agent A

In a NEW PowerShell window (Window 2), from the nono source tree root:

```powershell
$nono = "$PWD\target\release\nono.exe"

& $nono agent launch --profile aider -- C:\Windows\System32\notepad.exe
```

> Using `C:\Windows\System32\notepad.exe` (full path required — the daemon runs detached without
> the system PATH entries). `notepad.exe` is a GUI application that survives in AppContainer
> because it does not require network or console access.
>
> Alternatively, use any other long-running executable available with its full path.

**Expected output:**
```
Launched agent:
  tenant_id=<uuid>
  profile=aider
  sid=S-1-15-2-<A-octets>
  pid=<A-pid>
```

Keep this window open (notepad.exe is running in the background as the confined agent).

### SC1 Step 2 — Launch Agent B (while Agent A is still running)

In a THIRD PowerShell window (Window 3):

```powershell
$nono = "$PWD\target\release\nono.exe"

& $nono agent launch --profile aider -- C:\Windows\System32\notepad.exe
```

**Expected output:**
```
Launched agent:
  tenant_id=<uuid2 — DIFFERENT from Agent A>
  profile=aider
  sid=S-1-15-2-<B-octets — DIFFERENT from Agent A>
  pid=<B-pid>
```

**Expected:** Agent B starts. Window 2 (Agent A) and Window 3 (Agent B) are both running
simultaneously as distinct notepad processes, each in a separate AppContainer.

### SC1 Step 3 — Confirm both agents with distinct SIDs (the key check)

In a FOURTH PowerShell window (Window 4), while both notepad windows are open:

```powershell
$nono = "$PWD\target\release\nono.exe"
& $nono agent list
```

**Expected output (format illustrative; actual SIDs will differ):**

```
Tenant agents (2):
  <uuid-A-prefix>  profile=nono.session.<uuid-A-prefix>  sid=S-1-15-2-<A-octets>  pid=<A-pid>
  <uuid-B-prefix>  profile=nono.session.<uuid-B-prefix>  sid=S-1-15-2-<B-octets>  pid=<B-pid>
```

**SC1 PASS criterion (all three sub-criteria required):**

1. `nono agent list` shows **exactly 2** tenants (not 0, not 1).
2. The two package SIDs (`S-1-15-2-...`) are **distinct** — each agent has its own AppContainer.
3. Both notepad windows are visible and alive on the desktop.

**SC1 FAIL criterion (any of):**

- `nono agent list` shows 0 or 1 tenants while both agents are supposed to be running
- The two SIDs are IDENTICAL (same AppContainer reused — token isolation violated)
- `nono agent launch` returns `error: failed to launch agent: ...`
- `nono agent launch` returns "No daemon running" (daemon not started or crashed)

### SC1 Step 4 — Confirm tenant list returns to zero after both agents exit

Close both notepad windows (killing the confined agents):

```powershell
# Or via PowerShell (optional — closing the windows manually is also fine):
Stop-Process -Name notepad -Force
```

Wait ~2 seconds for the daemon to reap the processes, then:

```powershell
& $nono agent list
```

**Expected:**
```
No agents running.
```

**SC1 PASS criterion:** Zero tenants after both agents exit (no zombie entries, no handle leak
visible at the CLI level).

### SC1 Step 5 — Graceful daemon stop

```powershell
& $nono daemon stop
Start-Sleep -Seconds 3
& $nono daemon status
```

**Expected:**
```
nono-agentd stopped (dev-layout): nono-agentd: shutdown initiated.
nono-agentd status: NOT RUNNING (not in SCM; use `nono daemon start` ...)
```

**SC1 PASS criterion:** Daemon exits cleanly within 5 seconds of `daemon stop`. `daemon status`
returns NOT RUNNING (not RUNNING).

---

## SC2 — Cross-Tenant Denial (in-process integration tier)

**Purpose:** Confirm that tenant B cannot access tenant A's capability pipe instance. The OS-level
SDDL DACL gate proven in the Wave 0 spike (74-01) is confirmed here at the integration test tier.

SC2 is covered by the `daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance`
test in the Wave 0 spike harness. The manual cross-tenant attempt (attaching to A's pipe with B's
token) would require a custom tool not shipped in Phase 74; the automated test is the SC2 proof.

### SC2 Step 1 — Run the spike integration test

```powershell
# From the nono source tree root, in a REAL PowerShell window:
$env:NONO_DAEMON_INTEGRATION_TESTS = "1"
cargo test -p nono-cli --test daemon_handle_baseline `
    daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance `
    -- --nocapture
```

**Expected output (abridged):**
```
running 1 test
[spike74][denial] tenant A pkg_sid: S-1-15-2-<A-octets>
[spike74][denial] tenant B pkg_sid: S-1-15-2-<B-octets>
[spike74][denial] spawned AppContainer B child pid=<pid>
[spike74][denial] obtained real AppContainer B impersonation token
[spike74][denial] PASS: real AppContainer B token (SID: S-1-15-2-<B-octets>)
  was correctly denied access to tenant A's pipe instance (A-SID: S-1-15-2-<A-octets>).
  SDDL DACL gate confirmed at OS level.
test daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out
```

**SC2 PASS criterion:** Test reports `ok`. The diagnostic line explicitly states the SDDL DACL
gate denied tenant B's real AppContainer token.

**SC2 FAIL criterion:** Test reports `FAILED`. Record the full test output including any
`[spike74][denial]` diagnostic lines in the results table.

> Note on what SC2 proves: The test uses a REAL AppContainer B child process (not a synthetic
> Low-IL token) to obtain the impersonation token. This is the actual threat model — an AppContainer
> process trying to reach another tenant's named pipe. The OS DACL gate denies it at the kernel
> level. This is stronger proof than a synthetic token because it exercises the exact Win32
> `ImpersonateLoggedOnUser` + `DuplicateTokenEx` path the daemon's accept loop uses.

---

## SC3 — Handle Baseline (100-agent create/drop, no leak)

**Purpose:** Confirm that 100 cycles of AppContainerProfile create + SID derive + drop returns
the process handle count to the post-warmup plateau with zero steady-state growth (DMON-01 reap
invariant). This proves the `AgentTenant::Drop` cleanup path (`KILL_ON_JOB_CLOSE` +
`DeleteAppContainerProfile`) works correctly at scale.

### SC3 Step 1 — Run the handle-baseline spike test

For the most accurate (cold-process) characterization, run the test ALONE (not alongside siblings):

```powershell
$env:NONO_DAEMON_INTEGRATION_TESTS = "1"
cargo test -p nono-cli --test daemon_handle_baseline `
    n_agents_over_time_returns_to_baseline_handle_count `
    -- --nocapture
```

**Expected output (key lines):**

```
running 1 test
[spike74][handles] cold baseline handle count: <N>
[spike74][handles] post-warmup handle count: <N+warmup> (one-time delta=<warmup>)
[spike74][handles] after cycle 35: handle count = <plateau> (plateau-delta=0)
[spike74][handles] after cycle 60: handle count = <plateau> (plateau-delta=0)
[spike74][handles] after cycle 85: handle count = <plateau> (plateau-delta=0)
[spike74][handles] PASS: 100 cycles — one-time warmup cost=<warmup> handles
  (expected: RPC/threadpool infra); steady-state delta=0 (target: <= 5)
test n_agents_over_time_returns_to_baseline_handle_count ... ok
```

**Record these numbers in the results table:** `cold_baseline`, `post_warmup`, `one-time warmup
cost`, `steady-state delta`.

**SC3 PASS criterion:**

1. Test reports `ok`.
2. `steady-state delta=0` (or `<= 5`; the `EPSILON_STEADY` guard in the test).
3. The plateau handle count is stable across cycle checkpoints (35, 60, 85 all print the same count).

**SC3 FAIL criterion:**

- Test reports `FAILED`.
- `steady-state delta` is growing across cycle checkpoints (handle leak per cycle).
- Any `WARN: per-type sum N != absolute M — snapshot inconsistency` line (indicates measurement
  error; if this appears, try rerunning with `-- --test-threads=1` to ensure serial isolation).

> Reference (from Wave 0 spike run on Win11 build 26200):
> `cold_baseline=81, post_warmup=152, one-time warmup cost=71 (RPC/ETW/AppX infra), steady-state delta=0 over 90 cycles.`
>
> When run as part of the full test suite (sibling tests pre-pay the AppX warmup), the
> one-time cost appears much smaller (3-4 handles) because the RPC pool is already initialized.
> Both are correct outcomes; the PASS signal is `steady-state delta <= 5`.

---

## SC5 — Wire Protocol No-Tenant-ID Guard

**Purpose:** Confirm that `SupervisorMessage` serializes to JSON without any `tenant_id` or
`agent_id` field. Tenant identity must derive ONLY from the kernel-vouched AppContainer SID
obtained via `ImpersonateNamedPipeClient` + `TokenAppContainerSid` extraction, never from a
wire field an agent could forge.

### SC5 Step 1 — Run the unit test

```powershell
cargo test -p nono supervisor_message_no_tenant_id_field -- --nocapture
```

**Expected output:**

```
running 1 test
test supervisor::socket_windows::tests::supervisor_message_no_tenant_id_field ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; ...
```

**SC5 PASS criterion:** Test reports `ok`.

**SC5 FAIL criterion:** Test reports `FAILED`. This would mean a `SupervisorMessage` variant
now carries a `tenant_id` or `agent_id` field — a wire-level regression that breaks the
no-wire-authz invariant (SC5 / DMON-03). Record the failure message and do not mark SC5 green.

---

## Regression Check — Existing Tests Unbroken

**Purpose:** Confirm that Waves 0-4 did not introduce regressions into the existing test suite.

```powershell
# Unit tier (lib only; no host-gated integration tests; < 30 seconds):
cargo test -p nono-cli --lib
```

**Expected outcome:** All non-integration tests pass. The 4 pre-existing baseline failures
documented in `nono_cli_windows_baseline_test_failures.md` may still appear and are NOT
regressions:

```
IGNORED (pre-existing, env-specific):
  protected_paths::tests::blocks_parent_directory_capability
  protected_paths::tests::blocks_child_directory_capability
  protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root
  profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name
```

**Regression PASS criterion:** All tests pass except the 4 pre-existing baseline failures above.
No new failures introduced by Phase 74.

**Regression FAIL criterion:** Any test failure beyond those 4 pre-existing ones. Record the
failing test name and error message.

---

## Teardown

After completing all SC checks, stop and optionally remove the service:

```powershell
# Stop the daemon:
& $nono daemon stop
# Expected: nono-agentd stopped.

# Confirm stopped:
& $nono daemon status
# Expected: nono-agentd status: STOPPED

# Remove the SCM registration (optional — clean slate for next run):
& $nono daemon uninstall
# Expected: nono-agentd service registration removed.

# Confirm removed:
sc.exe query nono-agentd
# Expected: FAILED 1060 (does not exist)

# Clean up env var set for integration tests:
Remove-Item env:NONO_DAEMON_INTEGRATION_TESTS -ErrorAction SilentlyContinue
```

---

## Go/No-Go Checklist

Fill in on the real Win11 host. ALL six items must be PASS for Phase 74 to be marked complete.

| # | SC | Check | Result | Notes |
|---|-----|-------|--------|-------|
| 1 | SC4 | `sc qc nono-agentd` shows `TYPE : 110  USER_OWN_PROCESS` | [ ] PASS / [ ] FAIL | |
| 2 | SC1 | `nono agent list` shows 2 tenants with DISTINCT package SIDs while both agents alive | [ ] PASS / [ ] FAIL | Record SIDs |
| 3 | SC1 | Both agents closed; `nono agent list` returns to 0; `nono daemon stop` exits cleanly; `daemon status` shows NOT RUNNING | [ ] PASS / [ ] FAIL | |
| 4 | SC2 | `daemon_cross_tenant_denial` test: `ok. 1 passed; 0 failed` | [ ] PASS / [ ] FAIL | |
| 5 | SC3 | `n_agents_over_time` test: `ok`; steady-state delta <= 5; record baseline + post counts | [ ] PASS / [ ] FAIL | baseline=___ post=___ delta=___ |
| 6 | SC5 | `supervisor_message_no_tenant_id_field` test: `ok` | [ ] PASS / [ ] FAIL | |
| 7 | REG | `cargo test -p nono-cli --lib` — no NEW failures beyond the 4 pre-existing ones | [ ] PASS / [ ] FAIL | |

---

## Operator Pass/Fail Capture

Fill in from the live run on a real Win11 host.

| Field | Value |
|-------|-------|
| **Date** | |
| **Host OS build** | (e.g., Win11 Enterprise build 10.0.26200) |
| **nono binary** | dev-layout `.\target\release\nono.exe` (R-B4 satisfied) |
| **nono-agentd binary** | dev-layout `.\target\release\nono-agentd.exe` |
| **nono version string** | (run `.\target\release\nono.exe --version`) |
| **Workspace used** | (e.g., `C:\Users\<you>\nono-test-workspace`) |
| **SC4 TYPE field** | (paste: `TYPE : 110  USER_OWN_PROCESS` or actual value) |
| **SC1 Agent A SID** | S-1-15-2-... |
| **SC1 Agent B SID** | S-1-15-2-... (must differ from A) |
| **SC3 cold baseline handles** | |
| **SC3 post-warmup handles** | |
| **SC3 steady-state delta** | |

### Per-Step Outcome

| Step | Description | Result | Notes |
|------|-------------|--------|-------|
| P-1 | Build succeeds; nono.exe + nono-agentd.exe both in target\release | | |
| SC4-1 | `nono daemon install` exits 0 with success message | | |
| SC4-2 | `sc qc nono-agentd` TYPE = 110 USER_OWN_PROCESS | | |
| SC4-3 | `nono daemon start` exits 0; `daemon status` = RUNNING | | |
| SC1-1 | `nono daemon start` exits 0; daemon status RUNNING | | |
| SC1-2 | Agent A launched via `nono agent launch`; notepad running; shows tenant_id + SID | | |
| SC1-3 | Agent B launched concurrently; notepad running; DIFFERENT SID | | |
| SC1-4 | `nono agent list` shows 2 tenants, 2 distinct SIDs | | |
| SC1-5 | Both notepad closed; `nono agent list` returns to 0; `nono daemon stop` clean | | |
| SC2-1 | `daemon_cross_tenant_denial` integration test: PASS | | |
| SC3-1 | `n_agents_over_time` integration test: PASS, delta <= 5 | | |
| SC5-1 | `supervisor_message_no_tenant_id_field` unit test: PASS | | |
| REG | `cargo test -p nono-cli --lib` — no new failures | | |
| TD | `nono daemon stop` + `uninstall` complete cleanly | | |

### Overall Verdict

[ ] **PASS** — SC1-SC5 all green + regression PASS; Phase 74 go/no-go APPROVED.
[ ] **FAIL** — One or more SC failed; describe below and return to replanning.

**Failure description (if FAIL):**

```
(paste the full output of the failing command here)
```

---

## Common Failure Modes and Diagnostics

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `nono daemon install` fails: "nono-agentd.exe not found" | P-1 build not done or wrong directory | Run `cargo build --release -p nono-cli` from source root; confirm both EXEs in `target\release\` |
| `sc qc nono-agentd` shows `TYPE : 010  WIN32_OWN_PROCESS` | Wrong service type registered | Uninstall (`nono daemon uninstall`), rebuild, reinstall — ADR-74 D1 regression |
| `nono agent launch` returns "nono-agentd is not running" | Daemon not started or crashed | Run `nono daemon status`; if STOPPED run `nono daemon start`; check Windows Event Log for crash |
| `nono agent list` shows 0 during SC1 while agents appear to be running | Daemon not serving the control pipe, or agents already exited | Verify daemon is running (`nono daemon status`). Confirm notepad windows are still open. If daemon was started with an OLD binary before Plan 74-07, rebuild (`cargo build --release -p nono-cli`) and restart the daemon. |
| SC1 hangs: `nono agent launch` blocks indefinitely | Daemon is not accepting connections | Run `nono daemon status`; verify `daemon_capability_pipe` is open with `NONO_DAEMON_INTEGRATION_TESTS=1 cargo test daemon_concurrent_agents` |
| SC3 test: `WARN: per-type sum N != absolute M` | Parallel sibling tests interfered | Rerun with `-- --test-threads=1` for serial isolation |
| SC3 `steady-state delta > 5` | Handle leak in AppContainerProfile::Drop or job cleanup | Record full `[spike74][characterize]` output; look for growing handle types (Token, Job, Process, File = actual leak; EtwRegistration/Event/Semaphore = warmup only) |
| `CreateAppContainerProfile` fails or `nono agent launch` returns profile error | AppContainer profile namespace conflict | Run `nono daemon stop`; inspect HKCU\...\AppContainer\Storage for stale profiles; `nono daemon start` again |

---

## Security Assertions (inline with test evidence)

**No-escape-hatch invariant (SC4 / ADR-74 Decision 4):** The daemon's capability pipe is
query-only. The accept loop (`accept_loop.rs::serve_frames`) denies any wire request that does not
correspond to a read-only capability query. An agent cannot send an ADD request to expand its own
`CapabilitySet`. The ADR (`proj/ADR-74-privilege-model.md`) predates all daemon code in the git
history — the SC4 ordering gate is satisfied.

**Tenant isolation (SC2 / DMON-02):** The SDDL DACL gate at the pipe level plus the
`authenticate_pipe_client` post-auth registry SID check provide defense-in-depth. SC2 proves
the OS-level gate (SDDL) via a real AppContainer B process. A synthetic token approach would
require `SE_TCB_PRIVILEGE` (not held by interactive users on Win11); using a real spawned child
is both feasible and a stronger security proof.

**Privilege model (SC4 / ADR-74 Decision 1):** `TYPE : 110  USER_OWN_PROCESS` is the SCM
service type code for `SERVICE_USER_OWN_PROCESS`. The split from `nono-wfp-service` (which runs
as `TYPE : 010  WIN32_OWN_PROCESS` at LocalSystem) means an escaped agent reaching the daemon
gains only interactive-user-level access, not SYSTEM-level access.

**No wire tenant-id (SC5 / DMON-03):** `SupervisorMessage` carries no `tenant_id` or `agent_id`
field. The `session_id` field present in the message is a routing HINT only; authorization
derives from `authenticate_pipe_client`'s kernel-vouched `TokenAppContainerSid` output. This
is verified by the `supervisor_message_no_tenant_id_field` unit test, which serializes every
message variant to JSON and asserts the absence of those keys.

---

## Resume Signal

After completing the UAT on a real Win11 host, respond with ONE of:

**"UAT PASS SC1-SC5"** — if all 7 rows in the Go/No-Go Checklist are PASS (SC4+SC1+SC2+SC3+SC5+REG+TD).

or describe specific failures per SC:
- `SC1 FAIL: <paste nono agent list output and error>`
- `SC2 FAIL: <paste test output>`
- `SC3 FAIL: <paste handle baseline output with delta>`
- `SC4 FAIL: <paste sc qc output>`
- `SC5 FAIL: <paste test output>`
- `REG FAIL: <paste failing test names>`
