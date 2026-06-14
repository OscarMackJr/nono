---
phase: 71-engine-agnostic-launch-productionization
doc: human-uat-script
status: PENDING OPERATOR — script authored, live SC1 run not yet executed
created: 2026-06-14
---

# Phase 71 — Human UAT Script (SC1: Aider End-to-End Confinement)

SC1 cannot be exercised by unit tests: the broker arm (`BrokerLaunchNoPty`) requires a real
Win11 host, a real Aider install, the Low-IL mandatory-label relabel, and the AppContainer
path. This document is the operator's runbook for the live proof on a real machine.

---

## Preconditions (ALL must be true before starting)

### P-1: Dev-layout `target\release\nono.exe` (R-B4 — broker Authenticode trust gate)

`BrokerLaunchNoPty` calls `verify_broker_authenticode` before spawning the broker unless
`is_dev_build_layout` returns true (`launch.rs:1375,1698`). An unsigned install from
`C:\Program Files\nono\` is CORRECTLY REFUSED with a `TrustVerification` error.

**Action:** Build and run from the source tree:

```powershell
cargo build --release -p nono-cli
# Use:  .\target\release\nono.exe  (dev-layout — bypasses the Authenticode gate)
# NOT:  "C:\Program Files\nono\nono.exe"  (refused at the broker trust gate)
```

### P-2: Real PowerShell console (not git-bash / MSYS)

`CreateProcessAsUserW` inside the broker fails with GLE=87 (`ERROR_INVALID_PARAMETER`) when
the caller inherits a git-bash/MSYS pseudo-console (no real Win32 console handle). This is not
a bug — the broker broker requires a proper Win32 console context.

**Action:** Open a native PowerShell 5 window (`powershell.exe` or `Windows Terminal →
PowerShell`) — NOT a git-bash shell, NOT the Bash tool in this dev environment.

### P-3: User-owned workspace directory (R-B3 — WRITE_OWNER pre-launch gate)

`try_set_mandatory_label` relabels the workspace to Low integrity (`SetNamedSecurityInfoW
LABEL_SECURITY_INFORMATION`). This requires WRITE_OWNER on the path. A directory created from
an elevated console is owned by `BUILTIN\Administrators`; the session user lacks WRITE_OWNER
even if they are the NTFS "owner" — so the relabel fails opaquely (Pitfall 2 in RESEARCH.md).

Plan 04 (commit `ddb335ab`) wires an R-B3 GATE A **before** spawn: `path_has_write_owner`
is called and nono refuses with a named diagnostic if WRITE_OWNER is absent. The recommended
workspace is `%USERPROFILE%\nono-work` (user-created, user-owned).

**Action:**

```powershell
mkdir $env:USERPROFILE\nono-work     # creates user-owned dir; no elevation
# DO NOT use C:\nono-work or any dir created from an elevated prompt
```

### P-4: Aider installed on the UAT host

Aider (`aider.exe`) is the engine under test. It is NOT a nono dependency — install it on the
UAT host only.

```powershell
pipx install aider-chat
# Verify on PyPI at UAT time: https://pypi.org/project/aider-chat/
# Confirm: aider --version  (should print a version string)
```

**Fallback (if Aider install is problematic):** The `langchain-python` profile with a bare
`python.exe` call independently proves "engine is a variable" — spike-003 used raw python as
the strongest confinement proof. Run `python.exe -c "open('test.py','w').write('x')"` in place
of `aider.exe` steps, substituting `--profile langchain-python`.

### P-5: AppContainer / CLR gotchas to watch

These are already handled by the broker arm; they are listed here as diagnostics if something
goes wrong:

- **CreateAppContainerProfile (not derive-only):** The per-run AppContainer must be created via
  `CreateAppContainerProfile`. A derive-only SID (`DeriveAppContainerSidFromAppContainerName`)
  without a registered profile causes `CreateProcessW ERROR_FILE_NOT_FOUND`. The broker arm
  already calls `CreateAppContainerProfile`; this fires only if the broker itself is broken or
  replaced.

- **CLR env baseline (`SystemRoot`/`windir`/`SystemDrive`):** `append_windows_runtime_env`
  (`launch.rs:681`) preserves these three env vars in the child's environment. Without them,
  .NET CLR startup fails with HRESULT `0xFFFF0000` (CLR init error). If the CLR fails and
  these vars appear missing from the child's env, the broker arm has regressed.

- **`\\?\` prefix:** `normalize_windows_launch_path` strips the `\\?\` verbatim prefix from
  `config.current_dir` before passing it as `lpCurrentDirectory`. If a child reports a CWD of
  `\\?\C:\...` instead of `C:\...`, something has bypassed the normalize step.

---

## SC1: Aider End-to-End Confinement (ENG-01)

### Setup

```powershell
# From the nono source tree, in a real PowerShell window:
$nono   = ".\target\release\nono.exe"
$ws     = "$env:USERPROFILE\nono-work"

# Clean slate inside the workspace
Remove-Item "$ws\*" -Recurse -Force -ErrorAction SilentlyContinue

# Confirm workspace ownership (should show your username, not Administrators):
icacls $ws | Select-String "OWNER" ; whoami
```

### SC1 Step 1 — Write inside the granted workspace LANDS

**Command:**

```powershell
& $nono run --profile aider --workspace $ws -- aider.exe `
    --no-git --no-check-update --yes `
    --message "Write the text 'hello from aider' to a file named result.txt in the current directory."
```

**Expected outcome:**

- nono prints the banner showing `r+w <workspace>` as the only write-capable grant.
- Aider starts (SC1 confirms the broker arm correctly spawns and degrades to Low IL).
- `result.txt` appears under `$ws\result.txt` (or similar — accept any write Aider chooses).
- `aider.exe` exits 0 (or a small positive code — Aider may exit 1 on `--no-git` warnings).

**Verify:**

```powershell
Get-Content "$ws\result.txt"   # must print "hello from aider" (or the model's output)
```

**PASS if:** the file exists under `$ws`. **FAIL if:** nono refuses before launch, Aider hangs,
or the file is not created.

### SC1 Step 2 — Write OUTSIDE the workspace is DENIED (NO_WRITE_UP)

Run a second aider session that explicitly writes to a path OUTSIDE the workspace. Use Aider's
`--message` flag to instruct it to write a file via a relative escape and an absolute path:

```powershell
& $nono run --profile aider --workspace $ws -- aider.exe `
    --no-git --no-check-update --yes `
    --message "Write the text 'should not appear' to ..\outside.txt and also to C:\outside.txt"
```

**Expected outcome (two sub-cases):**

a. **Relative escape (`..\outside.txt` from the child CWD = `$ws`):** The child CWD is `$ws`
   (`C:\Users\<user>\nono-work`), so `..\outside.txt` resolves to
   `C:\Users\<user>\outside.txt`. The OS-enforced mandatory integrity label blocks the write:
   `UnauthorizedAccessException` / `Access is denied` / `NO_WRITE_UP`.
   The file `$env:USERPROFILE\outside.txt` must NOT exist after the run.

b. **Absolute path (`C:\outside.txt`):** Mandatory label enforcement: `Access is denied`.
   `C:\outside.txt` must NOT exist after the run.

**Verify:**

```powershell
Test-Path "$env:USERPROFILE\outside.txt"   # must be False
Test-Path "C:\outside.txt"                 # must be False
```

**PASS if:** Aider (or its python.exe subprocess) attempts both writes and both are OS-denied;
neither file is created. The nono banner must show NO path outside `$ws` as writable.
**FAIL if:** either file is created (confinement leak).

### SC1 Step 3 — python.exe subprocess is confined transitively (T-71-14)

This step proves that nono is the PARENT that confines the engine process tree — not a per-tool
hook. `aider.exe` is a thin distlib wrapper that immediately spawns `python.exe`; the python
process inherits the Low-IL mandatory label. We exercise a python-level write to a path outside
the workspace.

```powershell
& $nono run --profile aider --workspace $ws -- aider.exe `
    --no-git --no-check-update --yes `
    --message "Run this exact Python code: `
      import subprocess; `
      subprocess.run(['python','-c',`
        'open(chr(67)+chr(58)+chr(92)+chr(112)+chr(121)+chr(116)+chr(104)+chr(111)+chr(110)+chr(45)+chr(111)+chr(117)+chr(116)+chr(115)+chr(105)+chr(100)+chr(101)+chr(46)+chr(116)+chr(120)+chr(116),chr(119)).write(chr(104)+chr(105))'])"
```

> Simpler alternative: ask Aider to run `python -c "open('C:/python-outside.txt','w').write('hi')"`.

**Expected outcome:** The python.exe process is confined at Low IL (inherited from the job).
The write to `C:\python-outside.txt` is denied by the OS mandatory-label enforcement
(`NO_WRITE_UP`). The file must NOT exist.

**Verify:**

```powershell
Test-Path "C:\python-outside.txt"   # must be False
```

**PASS if:** python.exe cannot write outside the workspace even though it is a separate process
launched by aider.exe — the OS label is inherited transitively (this is the "parent-and-confine"
proof; a per-tool hook cannot enforce this because it fires at tool call, not at process spawn).
**FAIL if:** the file is created (transitive confinement failure).

---

## SC2: Relative-Write CWD Assertion (D-05 — PowerShell-to-C:\ trap removed)

Spike-003 found that engines do not uniformly inherit the launcher's CWD, so relative writes
could resolve to `C:\` instead of the intended workspace. Plan 04 (commit `2001bf97`) sets the
child's `lpCurrentDirectory` to the canonicalized absolute workspace.

```powershell
& $nono run --profile aider --workspace $ws -- aider.exe `
    --no-git --no-check-update --yes `
    --message "Write the text 'cwd-check' to a file named cwd-result.txt using a relative path."
```

**Expected outcome:** `cwd-result.txt` appears under `$ws\cwd-result.txt`, NOT at
`C:\cwd-result.txt` or the PowerShell session's CWD.

**Verify:**

```powershell
Test-Path "$ws\cwd-result.txt"   # must be True
Test-Path "C:\cwd-result.txt"    # must be False — the C:\ trap is closed
```

**PASS if:** the file is under `$ws`. **FAIL if:** the file is at `C:\` or the PowerShell CWD
(the PowerShell→C:\ relative-write trap has re-opened).

---

## ENG-02 Fail-Secure Spot-Checks (recommended, not blocking SC1 pass/fail)

These exercise the two pre-launch gates from Plans 03/04. They are optional but provide
high-confidence evidence that ENG-02 is live.

### ENG-02 Spot-Check A — Admin-owned workspace triggers named R-B3 refusal (D-08)

From an **elevated** PowerShell window, create an admin-owned dir, then try to launch from the
**normal** PowerShell window:

```powershell
# In elevated window:
mkdir C:\nono-admin-ws   # owned by BUILTIN\Administrators

# In normal (non-elevated) window:
& $nono run --profile aider --workspace C:\nono-admin-ws -- aider.exe --no-git --version
```

**Expected outcome:** nono refuses BEFORE any spawn with a named `SandboxInit` error that:
- Names WRITE_OWNER (0x00080000) as the missing permission
- Mentions the elevated-console/admin-ownership cause
- Suggests `%USERPROFILE%` or `%TEMP%` alternatives
- States "nono will NOT take ownership automatically"

**PASS if:** the error message fires pre-spawn and names the cause. **FAIL if:** nono launches
(confinement proceeds on an unrelabelable path — opaque failure scenario).

### ENG-02 Spot-Check B — Uncovered interpreter triggers named coverage refusal (D-07)

This spot-check requires that `python.exe` is NOT in the `aider` profile's allow-groups for
the test host (or that the python dir is excluded from the grant). The easiest way to trigger
it: launch with a fake profile that omits `python_runtime`:

```powershell
# Pass a non-existent profile to force the interpreter coverage gate on the real aider.exe:
& $nono run --profile claude-code --workspace $ws -- aider.exe --no-git --version
# "claude-code" does not declare python.exe as an interpreter; the coverage gate fires.
```

**Expected outcome:** nono refuses with a named error that:
- Names the exact `python.exe` path (e.g. `C:\Users\<user>\.local\pipx\venvs\aider-chat\Scripts\python.exe`)
- Suggests adding `--allow <dir>` or extending the profile's `windows_interpreters` field
- Does NOT launch the engine (fail-secure)

**PASS if:** the exact python.exe path is named in the refusal. **FAIL if:** nono launches
aider.exe with python.exe uncovered (partial confinement).

---

## SC5: Per-Engine Fit Table (ENG-03 doc deliverable)

This table documents which confinement model fits each engine class. It satisfies the SC5 doc
deliverable ("per-engine fit is documented for multiple engines").

| Engine | Confinement Model | Mechanism | Notes |
|--------|------------------|-----------|-------|
| **Aider** | Launch-and-confine | `nono run --profile aider --workspace <abs> -- aider.exe` via `BrokerLaunchNoPty` | SC1 in this doc. Parent-process confinement; all engine subprocesses (python.exe) confined transitively. Network: `block:false` for Phase 71 proof (no WFP service needed). |
| **LangChain-Python** | Launch-and-confine | `nono run --profile langchain-python --workspace <abs> -- python.exe my_agent.py` | Same broker arm; `python.exe` IS the top-level process (program == interpreter). `windows_interpreters:["python.exe"]` self-satisfies the coverage gate. Fallback for SC1 if Aider install is problematic. |
| **GitHub Copilot CLI** | Launch-and-confine | `nono run --profile <copilot-profile> --workspace <abs> -- gh copilot <args>` | Same pattern; requires a `copilot` profile declaring the `gh.exe` interpreter chain. Not yet defined as a built-in profile (Phase 71 ships only `aider` + `langchain-python`). |
| **Claude Code (via PreToolUse hook)** | Per-tool hook (legacy path) | `nono claude-code-hook` mediation on every Write/Edit/Bash tool call | Defense-in-depth only (Phase 60 Q1 verdict). NOT isolation — hooks fire at tool-call level, not at process spawn; the engine process itself is unconfined. |
| **Cursor** | WSL-only | Engine limitation — Cursor for Windows runs as a GUI app; the broker arm cannot confine a GUI host process usefully | Cursor on WSL: treat as a Linux launch (Landlock). Native Windows Cursor confinement is out of scope. |

**Key distinction:** "Launch-and-confine" (rows 1-3) confines the engine at spawn time — ALL
descendants inherit the Low-IL label transitively. "Per-tool hook" (row 4) fires per-operation
and leaves the engine process itself unconfined between calls. The launch-and-confine model is
the sound choice for untrusted engines (ENG-01, Phase 71 proof).

---

## Operator Pass/Fail Capture

Fill in after running the UAT on a real Win11 host.

| Field | Value |
|-------|-------|
| **Date** | |
| **Host OS build** | (e.g. Win11 26200) |
| **nono version** | (e.g. `.\target\release\nono.exe --version` output) |
| **Aider version** | (e.g. `aider --version` output) |
| **python.exe path** | (e.g. `where.exe python` output) |
| **Workspace used** | (e.g. `C:\Users\<user>\nono-work`) |

### Per-Step Outcome

| Step | Description | Result | Notes |
|------|-------------|--------|-------|
| SC1-1 | Inside-workspace write lands | PASS / FAIL | |
| SC1-2a | Relative escape `..\outside.txt` is DENIED | PASS / FAIL | `NO_WRITE_UP` confirmed? Y/N |
| SC1-2b | Absolute `C:\outside.txt` is DENIED | PASS / FAIL | |
| SC1-3 | `python.exe` subprocess write DENIED (transitive) | PASS / FAIL | |
| SC2 | Relative write resolves INSIDE workspace (no C:\\ trap) | PASS / FAIL | |
| ENG-02-A | Admin-owned workspace → named R-B3 refusal pre-spawn | PASS / FAIL / SKIP | |
| ENG-02-B | Uncovered interpreter → named coverage refusal | PASS / FAIL / SKIP | |

**NO_WRITE_UP confirmation (SC1-2):** Did the OS denial message include `NO_WRITE_UP`,
`UnauthorizedAccessException`, or `Access is denied`? [ ] YES [ ] NO

### Overall Verdict

[ ] **PASS** — SC1 steps 1/2a/2b/3 and SC2 all pass; transitive confinement confirmed.
[ ] **FAIL** — one or more steps failed; describe below.

**Failure description (if FAIL):**

```
(paste error messages / unexpected behavior here)
```

### Resume Signal

After completing the UAT: type `"approved"` (with this table filled in and pasted into this
document) to continue, or describe which step failed and the observed behavior.
