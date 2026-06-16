# Shape B Born-Confined Soundness Spike — Win11 Host Report

## PASS

**Date:** 2026-06-14
**Host:** Win11 Build 26200, nono v0.62.2, dev-layout `target\release\nono.exe`
**Profile:** `langchain-python`
**Workspace:** `C:\Users\OMack\AppData\Local\Temp\nono-spike-b-ws` (user-owned, R-B3)
**Python dir allowed:** `C:\Users\OMack\AppData\Local\Programs\Python\Python312`
**Deny probe:** `C:\Users\OMack\nono_deny_probe.txt`
**Network:** `network.block:false` (file-only proof per D-04)

---

## Child Token Output (CONFINEMENT evidence)

The confined python child ran `whoami /groups` as a subprocess. Output confirms the child token
carries the Low Mandatory Level label and the AppContainer attribute:

```
broker: AppContainer profile registered
  app_container_name=nono.session.757ab9618ff84efc93e6628d9589f9b3
broker: token/AppContainer setup complete  app_container=true
broker: spawned child  child_pid=42292  app_container=true
broker: child exited  child_exit_code=0

--- whoami /groups (inside child) ---
Mandatory Label\Low Mandatory Level        Label            S-1-16-4096
```

**Observed IL label:** `Mandatory Label\Low Mandatory Level` (S-1-16-4096)
**AppContainer:** confirmed — broker logged `app_container=true`; child is BOTH Low-IL AND AppContainer-confined.

---

## Ordered Proof of Soundness Invariants

### (a) ORDERING — PASS (code review)

Verified by reading `.planning/spikes/003-daemon-as-launcher/spike_b_soundness.py` (commits
`5ab36ada` → `12e6c9f6`). In `run_spike()`, the first substantive operation is:

```python
result_b = subprocess.run([nono, "run", ...])   # FIRST operation
```

The only code that precedes this call is:
- `argparse` argument parsing (no file/socket/registry handles opened)
- `os.path.isfile` / `os.path.isdir` prereq checks (read-only metadata queries, not object handles
  in the privileged-handle sense; these are correct OS stat calls, not file descriptor opens)

No file handles, socket connections, registry keys, or other privileged kernel objects are opened
in the calling python process before the `nono.exe` invocation completes. The ordering invariant
(D-03) is satisfied.

### (b) CONFINEMENT — PASS

nono exit 0; `whoami /groups` run as a subprocess inside the confined python child shows:

```
Mandatory Label\Low Mandatory Level        Label            S-1-16-4096
```

Broker log confirms: `app_container=true`, `child_pid=42292`.

The resulting child runs under a Low-IL / AppContainer token. The broker minted the AppContainer
profile (`nono.session.757ab9618ff84efc93e6628d9589f9b3`), completed token/AppContainer setup,
and reported the child exited 0. Confinement is kernel-enforced (WFP + Low-IL token + AppContainer
SID), not advisory.

### (c) DENY — PASS

```
nono exit 1
child stdout: PermissionError: [Errno 13] Permission denied:
              'C:\Users\OMack\nono_deny_probe.txt'
deny_probe exists=False
broker: child_exit_code=1
```

The child was denied write access to a path outside the granted workspace. The deny probe file
was not created. nono returned exit 1 (child exited 1 — permission denied propagated correctly).

### (d) ALLOW — PASS

```
nono exit 0
ok.txt exists=True, contents='ALLOW_PROBE'
broker: child_exit_code=0
```

The child successfully wrote inside the granted workspace (`<ws>\ok.txt`). The parent confirmed
the file exists with expected contents. nono returned exit 0.

---

## Overall: PASS — exit code 0, all 4 invariants green

---

## Driver Evolution: 5ab36ada → 12e6c9f6 (3 Harness Bug Fixes)

The original driver (commit `5ab36ada`) timed out / crashed on the first operator run due to
three harness bugs — NOT soundness failures. Each gate fired fail-secure exactly as designed.
The fixes are documented here so the verdict is reproducible.

### Bug 1: Wrong confined command — `whoami.exe` instead of `python.exe`

**Original:** The driver passed `whoami.exe` as the confined command to `nono run ... -- whoami.exe`.

**Problem:** The `langchain-python` profile's Phase-71 `windows_interpreters` engine-coverage gate
evaluates the target executable at launch. `whoami.exe` is not a covered interpreter for the
`langchain-python` profile, so nono refused to spawn it with an engine-coverage denial.

**Fix (12e6c9f6):** The confined command must be `python.exe`. `whoami /groups` is run as a
subprocess INSIDE the confined python child (via `subprocess.run(["whoami", "/groups"])`), not
as the directly-invoked process.

**Security significance:** This gate firing is corroborating evidence that nono refuses to launch
engines that are not covered by the active profile's interpreter policy — fail-secure behavior.

### Bug 2: Missing `--allow <python-dir>` grant

**Original:** The driver invoked `nono run --profile langchain-python --allow <ws> -- python.exe`.

**Problem:** nono's R-B4 / capability resolution validates that the interpreter path itself is
covered by an allow grant. `python.exe` resolves to a path under `C:\Users\OMack\AppData\Local\
Programs\Python\Python312`, which was not included in any `--allow` argument. nono refused to
launch the child.

**Fix (12e6c9f6):** Added `--allow <python-dir>` (`C:\Users\OMack\AppData\Local\Programs\Python\
Python312`) to the nono invocation so the interpreter path is covered.

**Security significance:** This gate also fired fail-secure — nono enforces that every executable
path the child might load from must be explicitly granted.

### Bug 3: Missing cwd coverage (D-52-01)

**Original:** The driver did not set a working directory for the confined child, leaving cwd at
the script's invocation directory.

**Problem:** Per the D-52-01 cwd-coverage rule (documented in project memory from Phase 52): the
child's cwd must be a path covered by an `--allow` grant. An uncovered cwd causes nono to refuse
the spawn with a clear R-B3/cwd-coverage diagnostic.

**Fix (12e6c9f6):** Added `cwd=workspace` to the `subprocess.run` call so the child starts in
the user-owned, already-allowed workspace directory.

**Security significance:** A third fail-secure gate — nono refuses to start a child in an
uncovered working directory, preventing implicit filesystem escapes via relative paths.

### Timeout adjustment

**Original:** 60-second subprocess timeout.

**Problem:** Cold-start Windows Defender scanning of the unsigned dev-layout `nono.exe` binary
can add 60–120 seconds on first run after a build. The original 60s timeout expired before
nono completed the child spawn.

**Fix (12e6c9f6):** Increased to 180 seconds.

---

## Constraints Satisfied

| Constraint | Status |
|------------|--------|
| R-B3: workspace is user-owned (`%TEMP%`) | PASS — `C:\Users\OMack\AppData\Local\Temp\nono-spike-b-ws` |
| R-B4: dev-layout trust gate skip | PASS — `target\release\nono.exe` used; unsigned `Program Files` install skipped per project memory |
| D-04: file-only proof (network.block:false) | PASS — network not blocked; spike proves file-level confinement only |
| D-03: no post-hoc IL drop on self | PASS — spike uses re-exec (new process), not `SetTokenInformation` on the calling python process |
| D-02: no privileged handle before confinement | PASS — `nono.exe` is invoked as the FIRST operation in `run_spike()` |

---

## Wave 2 Gate

This PASS verdict unblocks Wave 2 plans:
- **72-02** — Rust `nono-py` binding crate (confined_run / confine wrappers around nono.exe)
- **72-03** — E1–E5 contract documentation (engine-agnostic Python API contract)

The proven invocation pattern for Wave 2 / 72-04 LangChain proof:
```
nono run \
  --profile langchain-python \
  --allow <workspace> \
  --allow <python-dir> \
  -- python.exe -c "<agent-code>"
```
Child cwd must equal `<workspace>` (D-52-01).
