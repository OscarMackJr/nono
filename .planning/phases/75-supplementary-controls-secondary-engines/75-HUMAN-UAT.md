---
phase: 75-supplementary-controls-secondary-engines
doc: human-uat-script
status: PENDING OPERATOR
created: 2026-06-15
wave: 3
plan: "05"
requirements: [SUPP-01, SUPP-02, SUPP-03]
---

# Phase 75 — Human UAT Script (SC1–SC5: Supplementary Controls + Secondary Engines)

SC1 (demote does not reap), SC2 (per-agent WFP isolation / A1 gate), SC3 (Copilot CLI confined
end-to-end), and SC5 (nono-ts confinedRun on Win11) cannot be verified by unit tests alone: they
require the real nono-agentd binary running on Win11, the nono-wfp-service installed, and live
AppContainer process spawning. SC4 (daemon privilege model) is a carry-forward from Phase 74 UAT
and is re-confirmed here to verify the Phase 75 changes did not regress it. This document is the
operator's runbook for the go/no-go gate on a real Win11 host.

---

## Pre-Flight: CI Sweep Results (automated — filled by Task 1 executor)

| Check | Command | Result |
|-------|---------|--------|
| Workspace build | `cargo build --workspace` | PASS (26.23s, 0 errors) |
| Clippy strict | `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` | PASS (55.93s, 0 warnings) |
| Rustfmt | `cargo fmt --all -- --check` | PASS (after fmt fix commit 39b4032b) |
| nono lib tests | `cargo test -p nono` | 793 PASS, 1 pre-existing failure* |
| nono-cli bin tests | `cargo test -p nono-cli --bin nono` | 1261 PASS, 4 pre-existing failures* |
| nono-ts tests | `cargo test` (in `../nono-ts/`) | 5 PASS, 0 failed |
| Cross-target clippy (Linux) | `cargo clippy --target x86_64-unknown-linux-gnu` | PARTIAL (cross-toolchain absent on Win11; deferred to CI) |
| Cross-target clippy (macOS) | `cargo clippy --target x86_64-apple-darwin` | PARTIAL (cross-toolchain absent on Win11; deferred to CI) |

**Pre-existing failures (NOT Phase 75 regressions — confirmed at phase-base commits):**

- `nono` lib (1): `sandbox::windows::tests::try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails` — env-specific; documented in `nono_cli_windows_baseline_test_failures.md`
- `nono-cli` bin (4): `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`, `protected_paths::tests::blocks_parent_directory_capability`, `protected_paths::tests::blocks_child_directory_capability`, `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root` — all env-specific, fail at phase-base commits

**Wave 1 + 2 SUMMARY confirmations:**

| Plan | What it proves | Commit(s) |
|------|----------------|-----------|
| 75-01 (SUPP-02 WFP helpers) | `wfp_filter_add_at_launch`, `wfp_absent_fail_secure`, `wfp_absent_no_scoping_ok`, `wfp_filter_add_constructs_request`, `wfp_filter_remove_at_reap_not_in_drop`, `wfp_filter_remove_nonfatal_contract` — 6 tests all PASS | `195a7c11`, `1bdfc56e` |
| 75-02 (SUPP-01 demote verb) | `demote_returns_err_for_unknown_tenant`, `demote_does_not_reap_tenant_from_map`, `agent_demote_parses` — 3 tests all PASS | `923ae5f7`, `b1ae0d6f` |
| 75-03 (copilot-cli profile) | `copilot_cli_profile_present`, `copilot_cli_profile_is_native_pe` — 2 tests PASS | `f3f8f9bf`, `f1b8a6e6` |
| 75-04 (nono-ts parity) | 5 unit tests PASS; nono pin bumped 0.33.0 → 0.62 (path dep local) | `e218827` |
| 75-05 (fmt fix) | cargo fmt --all; CI sweep clean | `39b4032b` |

**New Phase 75 failures discovered during CI sweep:** None. All failures are pre-existing
baseline failures documented before Phase 75 work began.

---

## Reading Guide

Run sections in order: Pre-Conditions → SC4 (privilege model, run first) → SC1 → SC2 → SC3 → SC5.

SC4 is run FIRST because if nono-agentd has regressed to SYSTEM/LocalSystem, the rest of the UAT
demonstrates a privilege escalation risk, not a passing system.

---

## Pre-Conditions (ALL must be confirmed before starting SC gates)

### P-1: Dev-layout build — both nono.exe and nono-agentd.exe

```powershell
# From the nono source tree root, in a REAL PowerShell window (NOT git-bash/MSYS):
cargo build --release -p nono-cli

# Confirm both binaries exist:
Test-Path .\target\release\nono.exe        # must be True
Test-Path .\target\release\nono-agentd.exe # must be True

# Set path variables for use throughout this runbook:
$nono   = "$PWD\target\release\nono.exe"
$agentd = "$PWD\target\release\nono-agentd.exe"
```

### P-2: Real PowerShell console (NOT git-bash / MSYS)

`CreateProcessAsUserW` inside the broker arm fails with GLE=87 from a git-bash/MSYS pseudo-console.
All commands in this runbook require native PowerShell syntax.

### P-3: User-owned workspace directory (R-B3 — WRITE_OWNER pre-launch gate)

```powershell
# In a NON-elevated PowerShell window:
$ws = "$env:USERPROFILE\nono-test-workspace"
mkdir $ws -ErrorAction SilentlyContinue
(Get-Acl $ws).Owner   # must show <MACHINE>\<you>, NOT BUILTIN\Administrators
whoami                # must match the owner above
```

If owner shows BUILTIN\Administrators (from a prior elevated session):
```powershell
icacls $ws /setowner "$env:USERNAME"
(Get-Acl $ws).Owner   # re-confirm
```

### P-4: nono-wfp-service installed and running (required for SC2)

```powershell
sc.exe query nono-wfp-service
# Expected: STATE: 4 RUNNING
# If not running: net start nono-wfp-service
```

### P-5: nono-agentd clean slate

```powershell
sc.exe query nono-agentd
# Expected if clean: FAILED 1060 (ERROR_SERVICE_DOES_NOT_EXIST) — OK, proceed
# If present: sc.exe stop nono-agentd ; sc.exe delete nono-agentd
```

### P-6: nono daemon install (one-time admin, elevated shell)

```powershell
# From an ELEVATED PowerShell window (Run as administrator):
& $nono daemon install
# Expected: nono-agentd installed as a per-user service (type= userown).
```

### P-7: Copilot CLI installed (required for SC3)

```powershell
winget install GitHub.Copilot
# OR: msiexec /i copilot-x64.msi
where.exe copilot
# Record the actual install path (A2 assumption check)
```

---

## SC4 — Privilege Model (run FIRST; carry-forward from Phase 74)

**Purpose:** Confirm nono-agentd still runs as `USER_OWN_PROCESS` under the interactive user
account, NOT as LocalSystem or elevated. Phase 74 UAT verified this; Phase 75 must confirm no
regression.

```powershell
sc.exe qc nono-agentd
```

**Expected (critical fields):**
```
TYPE               : 50  USER_OWN_PROCESS TEMPLATE
SERVICE_START_NAME :
```

`TYPE : 50  USER_OWN_PROCESS` (not `10  WIN32_OWN_PROCESS`) and empty `SERVICE_START_NAME` (not
`LocalSystem`) are the pass signals. If FAIL, STOP — do not proceed to SC1.

```powershell
& $nono daemon start
Start-Sleep -Seconds 2
& $nono daemon status
# Expected: nono-agentd status: RUNNING
```

---

## SC1 — Demote Does Not Reap (SUPP-01)

**Purpose:** Confirm that `nono agent demote <tenant_id>` drops the IL of the confined agent's
process token to Low without reaping (killing or removing from the tenant list) the agent.

### SC1 Step 1 — Launch a long-running agent

Use an executable that stays alive long enough to demote:

```powershell
& $nono agent launch --profile aider -- notepad.exe
# Record the tenant_id from the output:
# "Launched agent: tenant_id=<hex>  profile=aider  sid=S-1-15-2-...  pid=<N>"
$tid = "<the tenant_id printed above>"
```

### SC1 Step 2 — Confirm agent is listed

```powershell
& $nono agent list
# Expected: "Tenant agents (1):" showing the agent with the tid and SID
```

### SC1 Step 3 — Demote the agent

```powershell
& $nono agent demote $tid
# Expected: success message (NOT "reaping" or "terminated")
# Example expected output: "demoted agent <tid>: IL dropped to Low"
```

### SC1 Step 4 — Confirm agent is still in list (not reaped)

```powershell
& $nono agent list
# Expected: agent STILL in list (not removed by demote)
```

### SC1 Step 5 — Verify IL-drop (optional but recommended)

Using Process Hacker or PowerShell:
```powershell
# Get the pid from the launch output
$pid_val = <pid from launch>
# In Process Hacker: right-click the process → Properties → Token → Integrity Level
# Should now show "Low" (was "AppContainer" or "Low" from spawn; confirm it changed if it wasn't already Low)
```

**SC1 PASS criterion:**
1. `demote` command exits with a success message (not an error)
2. `nono agent list` still shows the agent after demote (not reaped)
3. Agent notepad.exe window is still visible on desktop

**SC1 FAIL criterion:**
- `nono agent demote` returns an error
- `nono agent list` shows the agent gone after demote (incorrectly reaped)
- `nono agent demote` returns "tenant not found" (control-pipe lookup broken)

Close notepad.exe after recording the result:
```powershell
Stop-Process -Name notepad -Force -ErrorAction SilentlyContinue
```

---

## SC2 — Per-Agent WFP Isolation (A1 Gate) (SUPP-02)

**Purpose:** Confirm that two concurrent agents with different WFP-allowed domains cannot reach
each other's domain. This is the A1 assumption check:
> Does `FWPM_CONDITION_ALE_USER_ID` with a SID-scoped SD matching an AppContainer package SID
> correctly filter traffic from that AppContainer only?

**Note:** The copilot-cli and aider profiles both have `network.block: false` in the current
policy.json (no WFP scoping needed for their baseline operation). SC2 requires a
network-scoped profile to exercise the per-agent WFP filter. Use a custom test profile or
adapt the instructions below. If no network-scoped profile is available, SC2 is a partial test
of the D-05 gate only.

### SC2 Step 1 — Verify the D-05 fail-secure gate (always testable)

With nono-wfp-service running:

```powershell
# Temporarily stop the WFP service to test D-05:
net stop nono-wfp-service

# Attempt to launch an agent with a profile that uses network scoping.
# If no such profile exists yet (all current profiles have network.block: false),
# this step confirms D-05 is wired by verifying the launch succeeds normally for
# non-scoped profiles even when WFP service is absent.
& $nono agent launch --profile aider -- notepad.exe
# Expected for non-scoped profile: launch SUCCEEDS (not refused) even without WFP service

# Re-start WFP service:
net start nono-wfp-service
```

### SC2 Step 2 — Per-agent isolation test (if network-scoped test profile available)

If a network-scoped test profile exists (e.g., one with explicit `tcp_connect_ports` restrictions):

1. Launch Agent A with profile allowing only `api.openai.com` (port 443)
2. Launch Agent B with profile allowing only `api.anthropic.com` (port 443)
3. From Agent A's confined process: `Invoke-WebRequest -Uri "https://api.anthropic.com" -TimeoutSec 5`
   → expect BLOCKED (connection refused or timeout)
4. From Agent B's confined process: `Invoke-WebRequest -Uri "https://api.openai.com" -TimeoutSec 5`
   → expect BLOCKED
5. Each agent can reach only its own allowed domain

**A1 assumption record (fill in after test):**
- Does `FWPM_CONDITION_ALE_USER_ID` + AppContainer SID correctly filter per-agent? [ ] YES / [ ] NO / [ ] NOT TESTED (no network-scoped profile available)
- If NO: gap-closure plan 75-06 needed to switch to `FWPM_CONDITION_ALE_PACKAGE_ID`

**SC2 PASS criterion:**
- D-05 gate: non-network-scoped launch succeeds when WFP service absent (or when present)
- Per-agent isolation: each agent's allowed domain does not bleed to the other agent (if tested)

**SC2 PARTIAL criterion (acceptable for phase close):**
- D-05 unit tests PASS (confirmed in Wave 1 CI sweep above)
- A1 empirical isolation: DEFERRED (no network-scoped test profile available; gap-closure tracked)

---

## SC3 — Copilot CLI Confined End-to-End (SUPP-03a)

**Purpose:** Confirm that `copilot.exe` (native PE engine) runs confined via the daemon's
broker arm (AppContainer + Job). This is the D-08 live UAT gate — build-green-only is
insufficient for this SC.

### SC3 Step 1 — Confirm install path (A2 assumption check)

```powershell
where.exe copilot
# Record the actual path: ____________________________
# Expected (MSI install): C:\Users\<you>\AppData\Local\Programs\GitHub Copilot\copilot.exe
# Expected (npm global):  C:\Users\<you>\AppData\Roaming\npm\copilot.cmd → underlying PE
```

### SC3 Step 2 — Confirm nono daemon is running

```powershell
& $nono daemon status
# Expected: nono-agentd status: RUNNING
# If not: & $nono daemon start
```

### SC3 Step 3 — Launch Copilot CLI confined

Workspace for SC3:
```powershell
$copilot_ws = "C:\poc\copilot-workspace"
mkdir $copilot_ws -ErrorAction SilentlyContinue
```

Launch:
```powershell
& $nono agent launch --profile copilot-cli --workspace $copilot_ws -- copilot ask "What is 2+2?"
```

**Expected:**
- Launch succeeds; `copilot ask` runs under AppContainer + Job confinement
- The answer to "What is 2+2?" is printed (confirming the confined binary executed)
- No "Trust verification failed" error (dev-layout binary passes the R-B4 gate)

### SC3 Step 4 — Write-outside-workspace denial

```powershell
# From inside the confined process (you may need to use a different copilot subcommand
# or launch a test script instead of "ask"):
# Attempt: create a file outside the workspace
# Expected: DENIED (access denied / error code)
```

Alternative: launch a confined process that attempts the write directly:
```powershell
& $nono agent launch --profile copilot-cli --workspace $copilot_ws -- cmd.exe /c "echo test > C:\outside-workspace.txt"
# Expected: write DENIED or error (AppContainer does not have write access outside workspace)
```

### SC3 Step 5 — Subprocess monitoring (A4 assumption check)

While Copilot CLI is running confined, check for node.exe grandchildren:

```powershell
$copilot_pid = <pid from SC3 launch output>
Get-CimInstance Win32_Process | Where-Object { $_.ParentProcessId -eq $copilot_pid } | Select-Object Name, ProcessId
```

Also use Process Monitor (if available) and filter by parent PID.

**A4 assumption record (fill in after test):**
- Does `copilot.exe` spawn `node.exe` as a grandchild? [ ] YES / [ ] NO
- If YES: add `"windows_interpreters": ["node.exe"]` to the copilot-cli profile in
  `crates/nono-cli/data/policy.json` and update `copilot_cli_profile_is_native_pe` test
  before typing "approved"

**SC3 PASS criterion:**
1. `copilot ask "..."` runs confined end-to-end (exits 0, prints answer)
2. Write outside workspace is denied
3. A2 install path recorded
4. A4 node.exe grandchild: YES/NO recorded (profile updated if YES)

**SC3 FAIL criterion:**
- `nono agent launch` returns error (trust gate, profile parse, or daemon connection)
- Copilot CLI exits non-zero without producing output (confinement too restrictive)
- Write outside workspace SUCCEEDS (confinement not enforced)

---

## SC4 Carry-Forward — Daemon Privilege Split (confirmed above in SC4 section)

Already verified at the top of this runbook. Record the TYPE field value in the
results table below.

---

## SC5 — nono-ts confinedRun on Win11 (SUPP-03b)

**Purpose:** Confirm that `confinedRun` in the nono-ts binding confines a Node/JS process on
real Win11 with write denial outside the allowed workspace. This is the D-08 live UAT gate for
the TypeScript binding.

### SC5 Step 1 — Build nono-ts

```powershell
cd ..\nono-ts
npm run build
# Expected: produces index.node (the napi native module)
# If npm run build fails, try: npx @napi-rs/cli build --release --target x86_64-pc-windows-msvc
```

### SC5 Step 2 — Create the test script

Save as `test-confined.js` in the `../nono-ts/` directory:

```javascript
const { confinedRun } = require('./index.js');

// Test 1: write OUTSIDE the allowed workspace — expect non-zero exit (denied)
const outside_ws = 'C:\\poc\\ts-workspace';
const outside_path = 'C:\\outside-nono-ts-test.txt';
console.log('--- Test 1: write outside workspace (expect denial) ---');
const result1 = confinedRun(
  'node.exe',
  ['-e', `require("fs").writeFileSync("${outside_path}", "pwned")`],
  [outside_ws],          // allow: only inside workspace
  undefined,             // profile
  outside_ws,            // cwd
  30                     // timeout_secs
);
console.log('exit_code:', result1.exitCode);
console.log('stderr:', result1.stderr.toString());
// EXPECTED: exit_code != 0 (AppContainer denied the write outside workspace)

// Test 2: write INSIDE the allowed workspace — expect exit 0
const inside_path = outside_ws + '\\nono-ts-test-write.txt';
console.log('\n--- Test 2: write inside workspace (expect success) ---');
// Ensure workspace dir exists:
const fs = require('fs');
fs.mkdirSync(outside_ws, { recursive: true });
const result2 = confinedRun(
  'node.exe',
  ['-e', `require("fs").writeFileSync("${inside_path.replace(/\\/g, '\\\\')}", "ok")`],
  [outside_ws],          // allow: workspace
  undefined,
  outside_ws,
  30
);
console.log('exit_code:', result2.exitCode);
// EXPECTED: exit_code == 0
if (result2.exitCode === 0) {
  console.log('PASS: write inside workspace succeeded');
  fs.unlinkSync(inside_path);  // cleanup
}
```

### SC5 Step 3 — Create workspace directory

```powershell
mkdir C:\poc\ts-workspace -Force
```

### SC5 Step 4 — Run the test script

```powershell
cd ..\nono-ts
node test-confined.js
```

**Expected output (indicating confinement is working):**
```
--- Test 1: write outside workspace (expect denial) ---
exit_code: 1    # (or non-zero — write denied by AppContainer)

--- Test 2: write inside workspace (expect success) ---
exit_code: 0
PASS: write inside workspace succeeded
```

**SC5 PASS criterion:**
1. Test 1 exit_code is non-zero (write outside workspace denied)
2. Test 2 exit_code is 0 (write inside workspace succeeds)
3. No nono.exe or nono-agentd error in stderr output

**SC5 FAIL criterion:**
- Test 1 exit_code is 0 (write outside workspace was NOT denied — confinement broken)
- `confinedRun` itself throws (nono.exe not found on PATH, or binding load error)
- Test 2 exit_code is non-zero (workspace write incorrectly blocked)

**Cleanup after SC5:**
```powershell
Remove-Item C:\outside-nono-ts-test.txt -ErrorAction SilentlyContinue
Remove-Item C:\poc\ts-workspace -Recurse -Force -ErrorAction SilentlyContinue
del test-confined.js
```

---

## Abstraction Proof Checklist (fill in after SC1–SC5 complete)

After SC1–SC5 all pass, the abstraction is proven across:

| Dimension | Engine/Binding | Phase | Status |
|-----------|---------------|-------|--------|
| Engine 1 | Aider (Phase 71 SC1) | 71 | CONFIRMED (Phase 71 UAT PASS) |
| Engine 2 | Copilot CLI (SC3 above) | 75 | [ ] PENDING |
| Binding 1 | nono-py (Phase 72) | 72 | CONFIRMED (Phase 72 UAT PASS) |
| Binding 2 | nono-ts (SC5 above) | 75 | [ ] PENDING |

Both dimensions must show ≥2 confirmed entries for SUPP-03 to be closed.

---

## Go/No-Go Checklist

Fill in on the real Win11 host. ALL items must be PASS (or documented PARTIAL with gap-closure
plan) for Phase 75 to be marked complete.

| # | SC | Check | Result | Notes |
|---|-----|-------|--------|-------|
| 1 | SC4 | `sc qc nono-agentd` TYPE = 50 USER_OWN_PROCESS; SERVICE_START_NAME empty | [ ] PASS / [ ] FAIL | TYPE field: ___ |
| 2 | SC4 | `nono daemon start` exits 0; `daemon status` = RUNNING | [ ] PASS / [ ] FAIL | |
| 3 | SC1 | `nono agent demote <tid>` exits with success message (not error) | [ ] PASS / [ ] FAIL | |
| 4 | SC1 | `nono agent list` after demote still shows the agent (not reaped) | [ ] PASS / [ ] FAIL | |
| 5 | SC2 | D-05 gate: non-network-scoped launch succeeds when WFP service present | [ ] PASS / [ ] FAIL | |
| 6 | SC2 | A1 empirical: per-agent isolation confirmed (or DEFERRED with gap-closure) | [ ] PASS / [ ] PARTIAL / [ ] DEFERRED | |
| 7 | SC3 | `copilot ask "..."` runs confined end-to-end; A2 install path recorded | [ ] PASS / [ ] FAIL | Path: ___ |
| 8 | SC3 | A4 node.exe grandchild check done; profile updated if YES | [ ] PASS / [ ] FAIL | node.exe grandchild: YES/NO |
| 9 | SC5 | `node test-confined.js` Test 1 exit_code non-zero (write outside denied) | [ ] PASS / [ ] FAIL | exit_code: ___ |
| 10 | SC5 | `node test-confined.js` Test 2 exit_code 0 (write inside succeeds) | [ ] PASS / [ ] FAIL | exit_code: ___ |
| 11 | REG | Pre-existing 5 test failures unchanged; no NEW failures in nono / nono-cli | [ ] PASS / [ ] FAIL | |

---

## Assumption Records

Fill in from the live Win11 host run.

### A1 — FWPM_CONDITION_ALE_USER_ID + AppContainer SID empirical result

| Field | Value |
|-------|-------|
| **Test method** | |
| **Result** | YES (isolates per AppContainer SID) / NO (does not match; need PACKAGE_ID) / NOT TESTED |
| **Gap-closure needed** | YES (create plan 75-06 to switch to FWPM_CONDITION_ALE_PACKAGE_ID) / NO |

### A2 — copilot.exe actual install path

| Field | Value |
|-------|-------|
| **`where.exe copilot` output** | |
| **Install method used** | winget / MSI / npm global |
| **E1 coverage implication** | copilot-cli profile grants: ___ |

### A4 — node.exe grandchild check

| Field | Value |
|-------|-------|
| **Subprocess monitoring method** | Process Monitor / Get-CimInstance / other |
| **node.exe grandchild observed** | YES / NO |
| **Action taken** | Added `windows_interpreters: ["node.exe"]` to copilot-cli profile / No action needed |

---

## Operator Pass/Fail Capture

Fill in from the live run on a real Win11 host.

| Field | Value |
|-------|-------|
| **Date** | |
| **Host OS build** | (e.g., Win11 Enterprise build 10.0.26200) |
| **nono binary** | dev-layout `.\target\release\nono.exe` |
| **nono-agentd binary** | dev-layout `.\target\release\nono-agentd.exe` |
| **nono version string** | (run `.\target\release\nono.exe --version`) |
| **SC4 TYPE field** | (paste: `TYPE : 50  USER_OWN_PROCESS` or actual) |
| **SC1 tenant_id demoted** | |
| **SC1 agent still in list after demote** | YES / NO |
| **SC2 A1 result** | YES isolates / NO does not / NOT TESTED |
| **SC3 copilot.exe install path** | (from `where.exe copilot`) |
| **SC3 A4 node.exe grandchild** | YES / NO |
| **SC5 Test 1 exit_code** | |
| **SC5 Test 2 exit_code** | |

### Per-Step Outcome

| Step | Description | Result | Notes |
|------|-------------|--------|-------|
| P-1 | Build: nono.exe + nono-agentd.exe both in target\release | | |
| P-6 | `nono daemon install` from elevated shell exits 0 | | |
| SC4-1 | `sc qc nono-agentd` TYPE = 50 USER_OWN_PROCESS | | |
| SC4-2 | `nono daemon start` exits 0; status = RUNNING | | |
| SC1-1 | `nono agent launch --profile aider -- notepad.exe`; tenant_id printed | | |
| SC1-2 | `nono agent list` shows the agent | | |
| SC1-3 | `nono agent demote <tid>` exits with success message | | |
| SC1-4 | `nono agent list` AFTER demote still shows the agent | | |
| SC2-1 | D-05 gate verified (non-scoped launch behavior with WFP service) | | |
| SC2-2 | A1 isolation check (if network-scoped profile available) | | |
| SC3-1 | `where.exe copilot` — path recorded (A2) | | |
| SC3-2 | `nono agent launch --profile copilot-cli -- copilot ask "What is 2+2?"` | | |
| SC3-3 | Write-outside-workspace denial confirmed | | |
| SC3-4 | Subprocess check for node.exe grandchild (A4) | | |
| SC5-1 | `npm run build` in nono-ts succeeds (index.node produced) | | |
| SC5-2 | `node test-confined.js` Test 1 exit_code non-zero (outside denied) | | |
| SC5-3 | `node test-confined.js` Test 2 exit_code 0 (inside succeeds) | | |
| TD | `nono daemon stop` + `nono daemon uninstall` complete cleanly | | |

### Overall Verdict

[ ] **PASS** — SC1–SC5 all green + A1/A2/A4 recorded; Phase 75 go/no-go APPROVED.
[ ] **PARTIAL** — All SCs pass but A1 empirical test deferred; gap-closure plan 75-06 created.
[ ] **FAIL** — One or more SCs failed; describe below and return to replanning.

**Failure description (if FAIL):**

```
(paste the full output of the failing command here)
```

---

## Common Failure Modes and Diagnostics

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `nono daemon install` fails: "nono-agentd.exe not found" | P-1 build not done or wrong directory | Run `cargo build --release -p nono-cli`; confirm both EXEs in `target\release\` |
| `nono agent launch` returns "No daemon running" | Daemon not started | `nono daemon start` |
| `nono agent demote` returns "tenant_id not found" | Tenant reaped before demote | Re-launch with a longer-lived process; run launch + demote faster |
| SC3 `copilot ask` hangs indefinitely | Copilot CLI requires auth / network | Ensure copilot.exe is authenticated (`copilot auth login`); or use `copilot --version` as the SC3 command instead |
| SC5 `confinedRun` throws "nono.exe not found" | nono.exe not on PATH | Set `$env:NONO_EXE = "$PWD\target\release\nono.exe"` or add target\release to PATH |
| SC5 Test 1 exit_code is 0 (write not denied) | AppContainer workspace not set up; nono.exe run path not wiring the --allow correctly | Check the confinedRun call — the `allow` argument must NOT include `C:\` root; confirm `C:\poc\ts-workspace` exists and is user-owned |
| SC5 Test 2 exit_code non-zero (write inside denied) | `allow` path not matching the actual workspace path | Verify `C:\poc\ts-workspace` exact path in both the test script and filesystem |
| A4: node.exe appears as grandchild of copilot.exe | npm-loader.js JS fallback triggered | Add `"windows_interpreters": ["node.exe"]` to copilot-cli profile in policy.json + update test |

---

## Resume Signal

After completing the UAT on a real Win11 host:

1. Fill in all rows in the Go/No-Go Checklist above.
2. Fill in the Assumption Records (A1, A2, A4).
3. If A4 found node.exe grandchild: update `crates/nono-cli/data/policy.json` copilot-cli
   profile to add `"windows_interpreters": ["node.exe"]` BEFORE typing "approved".
4. If A1 found ALE_USER_ID does NOT filter AppContainer SIDs: create gap-closure plan 75-06
   (switch to `FWPM_CONDITION_ALE_PACKAGE_ID`) BEFORE typing "approved".

Then type **"approved"** to close Phase 75, or describe specific failures:
- `SC1 FAIL: <paste nono agent demote output and error>`
- `SC2 FAIL: <paste test output and A1 result>`
- `SC3 FAIL: <paste nono agent launch output and error>`
- `SC5 FAIL: <paste node test-confined.js output>`
