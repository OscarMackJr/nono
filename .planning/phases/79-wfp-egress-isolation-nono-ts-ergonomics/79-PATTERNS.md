# Phase 79: WFP Egress Isolation + nono-ts Ergonomics - Pattern Map

**Mapped:** 2026-06-17
**Files analyzed:** 4 (2 new, 2 modified)
**Analogs found:** 4 / 4

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `scripts/gates/wfp-egress-isolation.ps1` | gate script | request-response (two-agent spawn + verdict) | `scripts/gates/copilot-e2e.ps1` | exact (same two-function contract) |
| `C:\Users\OMack\nono-ts\tests\test_confined_run_default.js` | integration test | request-response (spawn + exit-code assert) | `C:\Users\OMack\nono-ts\tests\test_broker_ffi_mapping.js` | role-match |
| `crates/nono-cli/data/policy.json` (3 new profile blocks) | profile data / config | n/a (static JSON) | lines 884-937 (same file: `aider`, `copilot-cli` profile blocks) | exact |
| `crates/nono-ts/src/windows_confined_run.rs` (modified) | napi binding / service | request-response | same file (self-analog: `find_nono_exe` resolution pattern + existing `confined_run` body) | exact |

---

## Pattern Assignments

---

### `scripts/gates/wfp-egress-isolation.ps1` (gate script, request-response)

**Analog:** `scripts/gates/copilot-e2e.ps1` (lines 1-422) — VERIFIED on disk.
Secondary analog: `scripts/gates/harness-self-check.ps1` (lines 1-126) — simpler gate, cleaner read of the minimal contract.

#### File header comment pattern (from `harness-self-check.ps1` lines 1-19)

```powershell
# scripts/gates/harness-self-check.ps1
#
# Phase 76 Plan 02 — harness-self-check gate
#
# CONTRACT (D-05): exports exactly two functions dot-sourced by scripts/verify-dark.ps1.
# The gate RETURNS its verdict object — it MUST NOT call exit (D-02, PATTERNS exit/return
# convention). Only the runner owns exit-code mapping.
#
#   Test-Precondition -> $null (preconditions met) | "reason string" (SKIP_HOST_UNAVAILABLE)
#   Invoke-Gate       -> verdict object (PASS / FAIL / SKIP_HOST_UNAVAILABLE)
```

The new gate should open with the analogous header — naming the phase/plan, echoing the two-function contract, and documenting what the gate proves and what causes SKIP vs FAIL.

#### `Test-Precondition` pattern — return `$null` or reason string (from `copilot-e2e.ps1` lines 84-158)

The full function structure to copy:

```powershell
function Test-Precondition {
    # Return $null when all preconditions met; return a reason string -> SKIP_HOST_UNAVAILABLE.

    # Fast check for a required tool (throw is for harness-internal; missing tool = SKIP)
    if (-not (Get-Command nono -ErrorAction SilentlyContinue)) {
        # nono absence is harness-internal (always throw, not skip) per copilot-e2e:94-95
    }

    # Probe a named pipe (new for wfp-egress-isolation — not in copilot-e2e):
    # if $pipe does not exist -> return 'reason string' (SKIP_HOST_UNAVAILABLE)
    $pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', 'nono-wfp-control',
        [System.IO.Pipes.PipeDirection]::InOut)
    try {
        $pipe.Connect(2000)
        $pipe.Close()
    } catch {
        return 'nono-wfp-service is not running (pipe \\.\pipe\nono-wfp-control absent) — install and start nono-wfp-service'
    }

    return $null   # all preconditions met
}
```

Key rule (copilot-e2e line 94-95): nono's absence on PATH is NOT a SKIP — it is a harness-internal error (throw inside `Invoke-Gate`, never inside `Test-Precondition`).

#### `Invoke-Gate` pattern — process spawn with timeout (from `copilot-e2e.ps1` lines 160-422)

**ProcessStartInfo spawn pattern** (copilot-e2e.ps1 lines 259-295 — use this verbatim for the two `nono run` jobs):

```powershell
$psi = [System.Diagnostics.ProcessStartInfo]::new()
$psi.FileName = $nono.Source
foreach ($a in $nonoArgs) { [void]$psi.ArgumentList.Add([string]$a) }
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError  = $true
$psi.UseShellExecute        = $false
$psi.CreateNoWindow         = $true

$proc = [System.Diagnostics.Process]::new()
$proc.StartInfo = $psi
try {
    [void]$proc.Start()
    $outTask = $proc.StandardOutput.ReadToEndAsync()
    $errTask = $proc.StandardError.ReadToEndAsync()

    if (-not $proc.WaitForExit($script:GateTimeoutSeconds * 1000)) {
        $timedOut = $true
        try { $proc.Kill($true) } catch { }
        try { [void]$proc.WaitForExit(5000) } catch { }
    } else {
        $exitCode = $proc.ExitCode
    }

    $out = ''; $err = ''
    try { $out = $outTask.GetAwaiter().GetResult() } catch { }
    try { $err = $errTask.GetAwaiter().GetResult() } catch { }
    $output = (@($out, $err) -join "`n").Trim()
} finally {
    $proc.Dispose()
}
```

**WFP gate deviation from copilot-e2e:** The wfp gate launches TWO processes concurrently (Start-Job or two parallel ProcessStartInfo instances), not one. The verdict logic is simpler: `agentA.exitCode -eq 0 -AND agentB.exitCode -ne 0`.

**`Start-Job` parallel launch pattern (preferred for the two-agent shape):**

```powershell
$jobA = Start-Job -ScriptBlock {
    param($nonoPath, $port)
    $result = & $nonoPath run --profile nono-ts-wfp-test-open -- `
        curl.exe -s --max-time 5 "http://127.0.0.1:$port/probe" 2>&1
    return $LASTEXITCODE
} -ArgumentList $nono.Source, $port

$jobB = Start-Job -ScriptBlock {
    param($nonoPath, $port)
    $result = & $nonoPath run --profile nono-ts-wfp-test-blocked -- `
        curl.exe -s --max-time 5 "http://127.0.0.1:$port/probe" 2>&1
    return $LASTEXITCODE
} -ArgumentList $nono.Source, $port

$null = Wait-Job $jobA, $jobB -Timeout 60
$exitA = Receive-Job $jobA
$exitB = Receive-Job $jobB
Remove-Job $jobA, $jobB -Force -ErrorAction SilentlyContinue
```

#### Verdict object pattern (from `harness-self-check.ps1` lines 72-79, and `copilot-e2e.ps1` lines 309-315)

```powershell
# PASS verdict shape
return [ordered]@{
    gate      = 'wfp-egress-isolation'
    verdict   = 'PASS'
    reason    = 'agent A egress succeeded (exit 0) and agent B egress denied (exit non-zero) — per-SID WFP isolation confirmed'
    detail    = [ordered]@{
        agentAExitCode = $exitA
        agentBExitCode = $exitB
        mockPort       = $port
        agentAProfile  = 'nono-ts-wfp-test-open'
        agentBProfile  = 'nono-ts-wfp-test-blocked'
    }
    timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
}

# FAIL verdict shape (copy for each failure branch)
return [ordered]@{
    gate      = 'wfp-egress-isolation'
    verdict   = 'FAIL'
    reason    = 'agent A egress succeeded but agent B egress also succeeded — WFP per-SID filter did not deny B'
    detail    = [ordered]@{ agentAExitCode = $exitA; agentBExitCode = $exitB; mockPort = $port }
    timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
}

# SKIP_HOST_UNAVAILABLE returned from Invoke-Gate (e.g. nono-wfp-service went down mid-gate)
return [ordered]@{
    gate      = 'wfp-egress-isolation'
    verdict   = 'SKIP_HOST_UNAVAILABLE'
    reason    = 'nono run --profile nono-ts-wfp-test-blocked stderr indicates WFP service unreachable'
    detail    = [ordered]@{ agentBStderr = $errB }
    timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
}
```

**Contract invariants (from `harness-self-check.ps1` line 6 and `copilot-e2e.ps1` line 12):**
- Gate MUST NOT call `exit`. Only the runner owns exit-code mapping.
- Gate MUST NOT call `Persist-Verdict`. Only the runner owns persistence (WR-04).
- A `throw` inside `Invoke-Gate` = harness-internal error (runner maps to exit 4). Use `throw` ONLY for "could not run the gate at all" (e.g. nono not on PATH), never for a confinement result.
- Confinement result (A/B exit codes) is always a `return` of a verdict object, never a `throw`.

#### Assert-True helper pattern (from `copilot-e2e.ps1` lines 66-78)

```powershell
function Assert-True {
    param(
        [Parameter(Mandatory = $true)][bool]$Condition,
        [Parameter(Mandatory = $true)][string]$Message
    )
    if (-not $Condition) { throw $Message }
}
```

#### Mock TCP server pattern (from `79-RESEARCH.md` verified PowerShell snippet)

```powershell
$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
$listener.Start()
$port = $listener.LocalEndpoint.Port

$listenerJob = [System.Threading.Tasks.Task]::Run([Action]{
    for ($i = 0; $i -lt 2; $i++) {
        $client = $listener.AcceptTcpClient()
        $stream = $client.GetStream()
        $response = [Text.Encoding]::ASCII.GetBytes(
            "HTTP/1.1 200 OK`r`nContent-Length: 2`r`n`r`nOK")
        $stream.Write($response, 0, $response.Length)
        $stream.Close()
        $client.Close()
    }
    $listener.Stop()
})
```

The mock server MUST accept at least 2 connections (one per agent). Bind to `127.0.0.1` (loopback) — the Medium-IL gate process is not an AppContainer, so AppContainer client → non-AppContainer loopback server is allowed without LoopbackExempt (confirmed in 79-RESEARCH.md "Revised conclusion").

#### WFP service detection: false-PASS guard (from `79-RESEARCH.md` Pitfall 1)

When nono-wfp-service is absent, `nono run --profile nono-ts-wfp-test-blocked` exits non-zero before curl even runs (wfp_filter_add fails → launch_agent terminates the suspended process). This would produce `agentB.exitCode != 0` — a vacuous PASS. Mitigation:

1. `Test-Precondition` probes `\\.\pipe\nono-wfp-control` → SKIP if absent.
2. In `Invoke-Gate`, after jobs complete, check agent B's stderr for "WFP network scope required" / "nono-wfp-service is not reachable" and reclassify the verdict as SKIP_HOST_UNAVAILABLE if found (matches the fail-secure language from `launch.rs`).

---

### `C:\Users\OMack\nono-ts\tests\test_confined_run_default.js` (integration test, request-response)

**Analog:** `C:\Users\OMack\nono-ts\tests\test_broker_ffi_mapping.js` (lines 1-173) — VERIFIED on disk.
Secondary analog: `C:\Users\OMack\nono-ts\tests\test_sandbox_policy.js` (lines 1-233) — richer `assert`/`pass`/section helpers.

**Note on test.js:** `package.json` line 53 shows `"test": "node test.js"`. The file `test.js` does NOT exist in `C:\Users\OMack\nono-ts\` (no matches from Glob). The existing test files in `tests/` are run individually, not via `npm test`. This means the plan must either create `test.js` as a dispatcher OR update `package.json` `"test"` to point at the new integration test directly.

#### Require/import pattern (from `test_broker_ffi_mapping.js` line 30, `test_sandbox_policy.js` line 11-17)

```javascript
// Minimal: require only what you use (test_platform.js line 1)
const { isSupported, supportInfo } = require('../index.js');

// Full: multiple named imports (test_sandbox_policy.js lines 11-17)
const {
  CapabilitySet,
  QueryContext,
  SandboxState,
  AccessMode,
  isSupported,
  supportInfo,
} = require('../index.js');

// For confinedRun:
const { confinedRun } = require('../index.js');
const os   = require('os');
const path = require('path');
const fs   = require('fs');
```

#### Platform-skip pattern

`test_platform.js` has NO platform skip (it's cross-platform). `test_broker_ffi_mapping.js` uses the `skip()` helper for items that are conditionally unavailable. The new test needs a hard skip+exit(0) for non-Windows:

```javascript
if (process.platform !== 'win32') {
    console.log('SKIP: confinedRun integration test is Windows-only');
    process.exit(0);
}
```

This pattern is NOT in any existing test file verbatim — it is described in `79-RESEARCH.md` lines 418-421, modeled on the general napi "non-Windows stub returns error" pattern from `lib.rs` line 388-403.

#### Assert/pass/fail helpers (from `test_broker_ffi_mapping.js` lines 41-57)

```javascript
let failures = 0;
let skipped  = 0;

function assert(condition, msg) {
  if (!condition) {
    failures += 1;
    console.error(`  ${BG_RED}${WHITE}${BOLD} FAIL ${RESET} ${RED}${msg}${RESET}`);
  } else {
    console.log(`  ${GREEN}*${RESET} ${msg}`);
  }
}

function skip(msg, reason) {
  skipped += 1;
  console.log(`  ${YELLOW}- SKIP${RESET} ${msg}`);
  console.log(`         ${YELLOW}reason: ${reason}${RESET}`);
}
```

**Simpler variant** — `test_sandbox_policy.js` uses `assert` with `process.exit(1)` on failure (lines 35-40), making failures immediate rather than accumulating. Either pattern is valid; the `test_broker_ffi_mapping.js` accumulate-then-exit approach is preferable for integration tests that have multiple sequential assertions.

#### Summary/exit pattern (from `test_broker_ffi_mapping.js` lines 162-172)

```javascript
if (failures === 0) {
  console.log(`${BG_GREEN}${WHITE}${BOLD} PASS ${RESET} ${GREEN}` +
    `all SC4 assertions pass.${RESET}\n`);
  process.exit(0);
} else {
  console.error(`${BG_RED}${WHITE}${BOLD} FAIL ${RESET} ${RED}` +
    `${failures} assertion(s) failed.${RESET}\n`);
  process.exit(1);
}
```

#### The confinedRun integration test core (from `79-RESEARCH.md` lines 426-441)

```javascript
const ws = path.join(os.homedir(), 'nono-ts-default-gate-ws');
fs.mkdirSync(ws, { recursive: true });

console.log('--- SC4: confinedRun with no profile flags ---');
const result = confinedRun(
    'node.exe',
    ['-e', 'process.exit(0)'],
    undefined,   // allow: undefined → D-04 auto-covers node.exe dir
    undefined,   // profile: undefined → D-03 injects nono-ts-default
    ws,          // cwd: workspace under %USERPROFILE%
    30           // timeout: 30 seconds
);
if (result.exitCode !== 0) {
    console.error('FAIL: confinedRun with no profile exited', result.exitCode);
    console.error('stderr:', Buffer.from(result.stderr || []).toString());
    process.exit(1);
}
console.log('PASS: confinedRun default-broker-arm path succeeded (exit 0)');
```

**Critical constraint:** Workspace must be under `os.homedir()` (i.e. `%USERPROFILE%`), NOT `C:\poc\*` or a drive root — the AppContainer R-B3 label grant fails on drive-root dirs (from memory `feedback_windows_mandatory_label_write_owner` and `project_v212_phase71`).

---

### `crates/nono-cli/data/policy.json` — 3 new profile blocks

**Analog:** same file, lines 884-937 (`aider` at line 884 and `copilot-cli` at line 902) — VERIFIED on disk.

RESEARCH.md named the profile names correctly. The live file confirms:
- `"network": { "block": false }` is the existing notation (NOT `"block: false"` or any other shape). RESEARCH.md cited this correctly.
- `"windows_low_il_broker": true` is a top-level key at the profile object level (not nested under any sub-key). Lines 899, 917, 935 confirm.
- `"filesystem": {}` is valid (empty object, no sub-keys required). Lines 896, 914, 932 confirm.
- `"workdir": { "access": "readwrite" }` is the existing notation for writable workdir. Lines 898, 916, 934 confirm.
- `"security": { "groups": [], "signal_mode": "isolated" }` is the minimal-footprint security block. Lines 910-913 confirm for `copilot-cli`.

#### Template: copy `copilot-cli` profile shape (lines 902-919) for all 3 new profiles

```json
"copilot-cli": {
  "extends": "default",
  "meta": {
    "name": "copilot-cli",
    "version": "1.0.0",
    "description": "...",
    "author": "nono-project"
  },
  "security": {
    "groups": [],
    "signal_mode": "isolated"
  },
  "filesystem": {},
  "network": { "block": false },
  "workdir": { "access": "readwrite" },
  "windows_low_il_broker": true,
  "windows_interpreters": ["node.exe"]
}
```

**Deviations for the 3 new profiles:**

1. **`nono-ts-wfp-test-open`** (Agent A — non-blocked):
   - `"network": { "block": false }` (same as copilot-cli)
   - `"filesystem": { "allow": ["C:\\Windows\\System32"] }` — `curl.exe` lives there; R-B3 requires the launch target to be covered
   - `"workdir": { "access": "none" }` — test agent needs no writeable cwd
   - `"windows_interpreters"` — omit (curl.exe is a native PE, no interpreter needed)
   - `"interactive": false` — add to signal non-interactive/unattended mode

2. **`nono-ts-wfp-test-blocked`** (Agent B — WFP-blocked):
   - `"network": { "block": true }` — the ONLY new use of block:true in the file
   - `"filesystem": { "allow": ["C:\\Windows\\System32"] }` — same curl.exe coverage
   - `"workdir": { "access": "none" }`
   - `"interactive": false`

3. **`nono-ts-default`** (default for confinedRun no-profile path):
   - `"network": { "block": false }`
   - `"filesystem": {}` — empty; all coverage comes from `--allow` paths injected by D-04
   - `"workdir": { "access": "readwrite" }` — child needs to operate in its cwd
   - `"interactive": false`

**Insertion point:** after `langchain-python` (line 937 `}`) and before `python-dev` (line 938). All three new profiles are siblings at the same JSON level as existing profiles.

**Confirmation that `"network": { "block": true }` is schema-valid:** `nono-profile.schema.json` §471-490 defines `network.block` as a boolean (cited in CONTEXT.md canonical refs). No existing profile uses `block: true`, but the schema supports it. The WFP enforcement is triggered by `profile_needs_network_scoping()` in `launch.rs` which reads `profiles[name]["network"]["block"]` from `EMBEDDED_POLICY_JSON`.

---

### `crates/nono-ts/src/windows_confined_run.rs` — D-03/D-04 wiring

**Analog:** same file (self-analog). VERIFIED on disk (all line references match).

#### `find_nono_exe` PATH-resolution pattern (lines 34-66) — copy for `resolve_exe_dir`

```rust
fn find_nono_exe() -> napi::Result<PathBuf> {
    // 1. Check NONO_EXE env var
    if let Some(val) = std::env::var_os("NONO_EXE") {
        let path = PathBuf::from(val);
        if path.is_file() {
            return Ok(path);
        }
        return Err(Error::new(
            Status::GenericFailure,
            format!(
                "NONO_EXE is set to '{}' but that path is not an existing file",
                path.display()
            ),
        ));
    }

    // 2. Search PATH
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("nono.exe");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err(Error::new(
        Status::GenericFailure,
        "nono.exe not found. Set NONO_EXE to its path or add it to PATH.",
    ))
}
```

The new `resolve_exe_dir` helper follows the same PATH-walk pattern but: (a) does NOT check a custom env var (exe is caller-provided, not a fixed binary), (b) returns `Option<String>` (the parent directory), not `PathBuf`, and (c) on resolution failure returns `Ok(None)` rather than `Err` (auto-cover is best-effort; a missing exe-dir does not fail the call).

#### `confined_run` current body (lines 108-144) — what D-03/D-04 slot into

```rust
pub(crate) fn confined_run(
    exe: String,
    args: Vec<String>,
    allow: Option<Vec<String>>,
    profile: Option<String>,
    cwd: Option<String>,
    timeout_secs: Option<f64>,
) -> napi::Result<JsExecResult> {
    // Validate: at least one of profile or allow must be provided.
    if profile.is_none() && allow.as_ref().map_or(true, |v| v.is_empty()) {
        return Err(Error::new(
            Status::InvalidArg,
            "confined_run: at least one of 'profile' or 'allow' must be provided",
        ));
    }

    let nono_path = find_nono_exe()?;

    let mut cmd = Command::new(&nono_path);
    cmd.arg("run");
    build_nono_run_args(
        &mut cmd,
        profile.as_deref(),
        allow.as_deref(),
        cwd.as_deref(),
    );
    cmd.arg("--").arg(&exe).args(&args);

    if let Some(ref d) = cwd {
        cmd.current_dir(d);
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    do_spawn_and_wait(cmd, timeout_secs)
}
```

**D-03 injection:** Insert `let profile = profile.or_else(|| Some("nono-ts-default".to_string()));` as the **first line** of `confined_run`, BEFORE the validation check (per Pitfall 5 in RESEARCH.md: the validation guard `profile.is_none() && allow.is_empty()` must not fire before the default injection). After D-03, the guard becomes `profile.is_none() && allow.is_empty()` — since profile is now always `Some`, the guard fires only if the caller explicitly opts out via future API surface and passes an empty allow too.

**D-04 injection:** Resolve `exe` to its parent directory and append to `allow` before calling `build_nono_run_args`. The resolution is best-effort (returns `None` on failure). Pattern mirrors `find_nono_exe`'s PATH walk but for a caller-supplied exe name.

#### `build_nono_run_args` (lines 76-94) — unchanged by D-03/D-04

```rust
fn build_nono_run_args(
    cmd: &mut Command,
    profile: Option<&str>,
    allow: Option<&[String]>,
    cwd_allow: Option<&str>,
) {
    if let Some(p) = profile {
        cmd.arg("--profile").arg(p);
    }
    if let Some(paths) = allow {
        for path in paths {
            cmd.arg("--allow").arg(path);
        }
    }
    if let Some(cwd) = cwd_allow {
        cmd.arg("--allow").arg(cwd).arg("--allow-cwd");
    }
}
```

`build_nono_run_args` itself does NOT change. D-03/D-04 changes happen in `confined_run` BEFORE the call to `build_nono_run_args`.

#### Unit test pattern (lines 349-502) — model for new D-03/D-04 unit tests

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize all env-var-mutating tests (CLAUDE.md env-var isolation rule).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_confined_run_requires_profile_or_allow() {
        let result = confined_run(
            "test.exe".to_string(), vec![],
            None, None, None, None,
        );
        assert!(result.is_err(), "confined_run must require profile or allow");
    }
}
```

New unit tests to add (modeled on existing test at lines 479-501):
- `test_confined_run_default_profile_injected`: call `confined_run` with `profile=None`, `allow=None`, verify the command would emit `--profile nono-ts-default` (test the injection without actually spawning).
- `test_resolve_exe_dir_absolute`: given an absolute path, `resolve_exe_dir` returns the parent dir.
- `test_resolve_exe_dir_not_found_returns_none`: given a nonexistent exe, returns `Ok(None)`.

These tests follow the save/restore env-var pattern in the existing tests (lines 371-384, 394-420).

---

## Shared Patterns

### Verdict object key order

**Source:** `scripts/gates/harness-self-check.ps1` lines 72-79 (`[ordered]@{}` with keys `gate`, `verdict`, `reason`, `detail`, `timestamp`)
**Source:** `scripts/verify-dark.ps1` lines 36-43 (`Build-Verdict` function — same key order)
**Apply to:** `wfp-egress-isolation.ps1` — every `return` statement must use this exact key order.

```powershell
[ordered]@{
    gate      = '<gate-name>'
    verdict   = 'PASS' | 'FAIL' | 'SKIP_HOST_UNAVAILABLE'
    reason    = '...'
    detail    = [ordered]@{ ... }
    timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ')
}
```

### No `exit` / no `Persist-Verdict` in gates

**Source:** `scripts/gates/harness-self-check.ps1` lines 6-7 (D-02, D-05 contract)
**Apply to:** `wfp-egress-isolation.ps1` exclusively — the runner (`verify-dark.ps1`) owns both.

### `$ErrorActionPreference = 'Continue'` inside `Invoke-Gate`

**Source:** `scripts/gates/copilot-e2e.ps1` line 168
**Apply to:** `wfp-egress-isolation.ps1` `Invoke-Gate` body — native tools write progress to stderr; prevent that from becoming terminating errors.

### napi error handling pattern (Rust)

**Source:** `crates/nono-ts/src/windows_confined_run.rs` lines 42-49, 62-65
**Apply to:** Any new helper function in `windows_confined_run.rs`

```rust
return Err(Error::new(
    Status::GenericFailure,
    format!("descriptive message: {}", detail),
));
```

For best-effort helpers (like `resolve_exe_dir`), use `Ok(None)` instead of `Err` on soft failures.

### Env-var save/restore in Rust unit tests

**Source:** `crates/nono-ts/src/windows_confined_run.rs` lines 371-384
**Apply to:** Any new unit test that touches `NONO_EXE`, `PATH`, or `NONO_ALREADY_CONFINED`

```rust
let prev = std::env::var_os("NONO_EXE");
unsafe { std::env::set_var("NONO_EXE", &test_exe_str) };
let result = find_nono_exe();
unsafe {
    match prev {
        Some(v) => std::env::set_var("NONO_EXE", v),
        None    => std::env::remove_var("NONO_EXE"),
    }
}
```

### `#[cfg(windows)]` gating

**Source:** `crates/nono-ts/src/windows_confined_run.rs` line 12 (`#![cfg(windows)]`)
**Apply to:** The entire `windows_confined_run.rs` module. Any new helper added to this file is automatically Windows-only via the module-level `#![cfg(windows)]`. No per-function `#[cfg]` needed.

---

## No Analog Found

None. All 4 files have at least a role-match analog in the codebase.

---

## RESEARCH.md Staleness Notes

All line references in RESEARCH.md were verified against live code:

| RESEARCH.md Claim | Live File | Status |
|---|---|---|
| `windows_confined_run.rs §76-94` = `build_nono_run_args` | Confirmed lines 76-94 | CURRENT |
| `windows_confined_run.rs §108-145` = `confined_run` | Confirmed lines 108-145 | CURRENT |
| `lib.rs §375-403` = `confinedRun` napi export | Confirmed lines 375-403 | CURRENT |
| `copilot-e2e.ps1` / `harness-self-check.ps1` = gate contract | Both files confirmed on disk | CURRENT |
| `policy.json`: `aider`, `copilot-cli`, `langchain-python` have `windows_low_il_broker: true` | Confirmed lines 899, 917, 935 | CURRENT |
| `policy.json`: no existing profile uses `network.block: true` | Grep confirmed zero matches | CURRENT |
| `nono-ts/tests/` has no `confinedRun` integration test | Glob confirmed no confinedRun test | CURRENT |
| `package.json §53`: `"test": "node test.js"` | Confirmed line 53 | CURRENT |
| `test.js` does not exist | Glob `**/*.js` confirms absent | CURRENT |

**One deviation from RESEARCH.md summary (line 53-54):** RESEARCH.md states "but `test.js` does not exist. `test-confined.js` exists but is NOT wired to `npm test`." Glob found no `test-confined.js` either — that file does not appear to exist. The planner should treat the `npm test` wiring as purely a `package.json` update (change `"test"` to point to the new file or create a `test.js` dispatcher), with no existing `test-confined.js` to consult.

---

## Metadata

**Analog search scope:** `C:\Users\OMack\Nono\scripts\gates\`, `C:\Users\OMack\nono-ts\tests\`, `C:\Users\OMack\nono-ts\src\`, `C:\Users\OMack\Nono\crates\nono-cli\data\`
**Files scanned:** 6 (2 gate scripts, 2 nono-ts test files, 1 Rust module, 1 policy.json)
**Pattern extraction date:** 2026-06-17
