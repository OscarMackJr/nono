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

**Action:** Open a native PowerShell window (`powershell.exe` or `Windows Terminal →
PowerShell`) — NOT a git-bash shell, NOT the Bash tool in this dev environment.

> **Are you in cmd.exe?** If your prompt looks like `C:\Users\<you>\nono-work>` (a bare path
> with `>` and no `PS` prefix) you are in **cmd.exe**, not PowerShell. Every command in this
> runbook is PowerShell (`$env:USERPROFILE`, `& $nono ...`, backtick line-continuations) and
> will not run in cmd. Launch `powershell.exe` and continue there.

### P-3: User-owned workspace directory (R-B3 — WRITE_OWNER pre-launch gate)

`try_set_mandatory_label` relabels the workspace to Low integrity (`SetNamedSecurityInfoW
LABEL_SECURITY_INFORMATION`). This requires WRITE_OWNER on the path. A directory created from
an elevated console is owned by `BUILTIN\Administrators`; the session user lacks WRITE_OWNER
even if they are the NTFS "owner" — so the relabel fails opaquely (Pitfall 2 in RESEARCH.md).

Plan 04 (commit `ddb335ab`) wires an R-B3 GATE A **before** spawn: `path_has_write_owner`
is called and nono refuses with a named diagnostic if WRITE_OWNER is absent. The recommended
workspace is `%USERPROFILE%\nono-work` (user-created, user-owned).

R-B3 checks **NTFS ownership** (owner SID == current-user SID), NOT ACL grants — so you do
**not** need to edit/`icacls`-grant any permissions. With default permissions, a folder you
create under `%USERPROFILE%` is owned by you and passes. nono deliberately does NOT auto-take
ownership (decision D-08) — it is your job to satisfy this precondition.

> **Admin-account trap (READ THIS if your account is a local Administrator):** When you `mkdir`
> from an **elevated** ("Run as Administrator") console, Windows sets the folder owner to
> `BUILTIN\Administrators`, NOT your user SID — and R-B3 then refuses the launch. The fix is not
> to change permissions; it is to **create the folder from a normal, non-elevated console**.
> You do not need elevation for anything in SC1 (lowering your own object's integrity label and
> creating an AppContainer profile are unprivileged), so run the entire UAT non-elevated.

**Action (in a NON-elevated PowerShell window):**

```powershell
mkdir $env:USERPROFILE\nono-work                  # user-owned; no elevation
# DO NOT use C:\nono-work or any dir created from an elevated prompt.

# Verify YOU own it (must print <MACHINE>\<you>, NOT BUILTIN\Administrators):
(Get-Acl $env:USERPROFILE\nono-work).Owner
```

If that prints `BUILTIN\Administrators` (you created it elevated earlier), either delete and
recreate it non-elevated, or reassign ownership to yourself:

```powershell
icacls $env:USERPROFILE\nono-work /setowner "$env:USERNAME"
(Get-Acl $env:USERPROFILE\nono-work).Owner        # re-confirm it now shows you
```

### P-4: Aider installed on the UAT host

Aider (`aider.exe`) is the engine under test. It is NOT a nono dependency — install it on the
UAT host only. `[ASSUMED]` — verify on PyPI at UAT time: https://pypi.org/project/aider-chat/

**`pipx` is commonly NOT installed** (e.g. this host has Python 3.12 + pip but no pipx). Do not
assume `pipx install aider-chat` will work — pick one of these, in order of least friction:

```powershell
# Option A (simplest — uses the pip you already have; installs aider.exe into
# <python>\Scripts which is already on PATH). Recommended for a one-off UAT:
python -m pip install aider-chat
aider --version          # confirm a version string; note where.exe aider for the capture table

# Option B (isolated via pipx — requires bootstrapping pipx first, then RESTART the shell
# so the updated PATH takes effect before the second command):
python -m pip install --user pipx
python -m pipx ensurepath
#   <-- close and reopen PowerShell here -->
pipx install aider-chat
```

> Note for the interpreter-coverage gate (D-07): whichever route you pick determines WHERE
> `python.exe` lives (Option A → `…\Programs\Python\Python3xx\python.exe`; Option B → a pipx
> venv under `…\.local\pipx\venvs\aider-chat\Scripts\python.exe`). If that location is not
> covered by the `aider` profile's `python_runtime` group, nono will refuse pre-spawn and
> **name the exact path plus the `--allow <dir>` fix** — apply it and re-run. That refusal is
> ENG-02 working as designed, not a UAT failure.

**Fallback — no Aider install needed (recommended if Aider fights you):** The `langchain-python`
profile with a bare `python.exe` call independently proves "engine is a variable" — spike-003
used raw python as the strongest confinement proof, and `python.exe` is already on this host.
Substitute `--profile langchain-python -- python.exe <args>` for the `aider.exe` steps below,
e.g. to prove an outside-write is denied:

```powershell
& $nono run --profile langchain-python --workspace $ws -- python.exe -c "open(r'C:\outside.txt','w').write('x')"
Test-Path C:\outside.txt    # must be False — write denied by the inherited Low-IL label
```

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

# Confirm workspace ownership (must show <MACHINE>\<you>, not BUILTIN\Administrators):
(Get-Acl $ws).Owner ; whoami
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

Filled in from the live run on a real Win11 host.

| Field | Value |
|-------|-------|
| **Date** | 2026-06-13 |
| **Host OS build** | Win11 Enterprise build 10.0.26200.0 (26200) |
| **nono version** | `nono 0.62.2` (dev-layout `.\target\release\nono.exe`, R-B4 satisfied) |
| **Proof engine** | `langchain-python` (raw `python.exe`) — "engine is a variable" proof; literal Aider run pending |
| **Aider version** | (pending — literal Aider run not yet executed; needs `python -m pip install aider-chat` + LLM API key) |
| **python.exe path** | `C:\Users\OMack\AppData\Local\Programs\Python\Python312\python.exe` (Python 3.12.10) |
| **Workspace used** | `C:\Users\OMack\nono-work` (user-owned, non-elevated) |

### Per-Step Outcome

| Step | Description | Result | Notes |
|------|-------------|--------|-------|
| SC1-1 | Inside-workspace write lands | **PASS** | `inside.txt` landed under `$ws` (relative path), child exit 0 |
| SC1-2a | Relative escape `..\outside.txt` is DENIED | PENDING | not yet run (the absolute case 2b was exercised instead) |
| SC1-2b | Absolute `C:\outside.txt` is DENIED | **PASS** | `PermissionError: [Errno 13] Permission denied: 'C:\outside.txt'`, child exit 1, `Test-Path → False` |
| SC1-3 | `python.exe` subprocess write DENIED (transitive) | **PASS** | grandchild python (spawned by the confined child via `subprocess`, NOT launched/validated by nono) got `PermissionError [Errno 13]` writing `C:/outside2.txt`; `grandchild_rc=1`, child exit 0, `Test-Path → False` — label inherited transitively (T-71-14) |
| SC2 | Relative write resolves INSIDE workspace (no C:\\ trap) | **PASS** | `inside.txt` resolved to `$ws`, not `C:\` — child CWD = absolute workspace |
| ENG-02-A | Admin-owned workspace → named R-B3 refusal pre-spawn | PENDING | optional spot-check, not yet run |
| ENG-02-B | Uncovered interpreter → named coverage refusal | **PASS** | first run refused pre-spawn naming `…\Python312\python.exe` + the exact `--allow` fix (D-07) |

**Run evidence (langchain-python, child in AppContainer `app_container=true`):**
- Negative: `& nono run --profile langchain-python --workspace $ws --allow $py -- python.exe -c "open(r'C:\outside.txt','w').write('x')"` → `PermissionError [Errno 13]`; `Test-Path C:\outside.txt = False`.
- Positive: `… -- python.exe -c "open('inside.txt','w').write('hi')"` → exit 0; `Test-Path $ws\inside.txt = True`.
- Transitive (T-71-14): `… -- python.exe transitive_test.py` where the script does `subprocess.run([sys.executable,'-c',"open('C:/outside2.txt','w')…"])` → `grandchild_rc=1`, `PermissionError [Errno 13]`, child exit 0, `Test-Path C:\outside2.txt = False`.
- Bonus fail-secure layer observed: the inline `-c` variant of the transitive probe was refused PRE-LAUNCH by `validate_windows_command_args` (the literal `C:\outside2.txt` in argv is an uncovered absolute-path argument) — defense-in-depth beyond the label.

**NO_WRITE_UP confirmation (SC1-2):** Did the OS denial message include `NO_WRITE_UP`,
`UnauthorizedAccessException`, or `Access is denied`? [x] YES — surfaced as Python `PermissionError [Errno 13] Permission denied` (the user-mode manifestation of the mandatory-label write-up block).

### Overall Verdict

[x] **PASS (langchain-python engine)** — SC1 core fully proven on real Win11 26200: inside-write LANDS (SC1-1), outside-write DENIED (SC1-2b), transitive subprocess DENIED (SC1-3, T-71-14), relative-write CWD inside workspace (SC2); ENG-02-B coverage refusal PASS. Confinement is OS-enforced (AppContainer + mandatory label), inherited transitively. Optional/pending: SC1-2a relative-escape, ENG-02-A admin-owned refusal, and the operator-requested **literal Aider** run (needs `pip install aider-chat` + LLM API key).
[ ] **FAIL** — one or more steps failed; describe below.

**Failure description (if FAIL):**

```
(paste error messages / unexpected behavior here)
```

### Resume Signal

**APPROVED — 2026-06-13 (operator: Oscar Mack Jr).** SC1 accepted as PASS on the strength of the
`langchain-python` (raw `python.exe`) live proof on Win11 26200: inside-write lands, outside-write
denied, transitive grandchild-subprocess denied (T-71-14), relative-write CWD inside the workspace,
and the ENG-02 interpreter-coverage + command-argument fail-secure gates fired as designed. The
engine-neutral launch path (ENG-01) is demonstrated sound on real hardware — "the engine is a
variable." The literal `aider.exe` run was deferred (requires `pip install aider-chat` + an LLM API
key, which was not configured on the host); the `langchain-python` engine is the documented,
sufficient SC1 proof (spike-003 precedent). Optional ENG-02-A admin-owned-workspace refusal and
SC1-2a relative-escape spot-checks were not exercised (non-blocking).
