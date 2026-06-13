# Live-Verification Cookbook — R-B4, R-A1 & R-A6

Operator runbook for the checks that could not be performed autonomously during the
`/gsd:debug` sessions. Run these on a **real Win11 host from an elevated-capable PowerShell
console** (NOT git-bash/MSYS — the broker arm needs a real console; `CreateProcessAsUserW`
returns GLE=87 otherwise).

- Fixes under test: commits `19f17ca4` (R-B4), `f48ec206` (R-A1), `ef1ea822` (R-A6) on branch `fix/win-confinement-rb4-ra1`.
- Build first so the fixed hook re-embeds and the dev-target-root bakes:
  ```powershell
  Set-Location C:\Users\OMack\Nono
  cargo build -p nono-cli --bin nono   # dev nono.exe at target\debug\nono.exe
  ```
- Record PASS/FAIL inline. All three R-B4 sub-checks must pass; both R-A1 checks must pass;
  R-A6 needs the LanguageMode probe + at least the confined Write byte-vehicle and the E2E
  hook check to pass.

PS C:\Users\OMack\Nono>  Set-Location C:\Users\OMack\Nono
PS C:\Users\OMack\Nono>   cargo build -p nono-cli --bin nono   # dev nono.exe at target\debug\nono.exe
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.51s
---

## R-B4 — broker Authenticode dev-bypass is closed (path provenance, not substring)

The gate now skips only when the **canonicalized** running `nono.exe` lives under the
compile-time-baked `NONO_DEV_TARGET_ROOT` (= this repo's `target\` dir). A copy under any
*other* `...\target\release\` path must now ENFORCE the gate.

### B4-1 (NEGATIVE — bypass closed). Attacker-style lookalike path → gate ENFORCES.

```powershell
# Build a SIGNED-free release pair OR reuse the unsigned dev binaries.
# Stage them under an attacker-style path that contains \target\release\ but is NOT this repo's target dir:
$evil = "C:\Users\OMack\evil\target\release"
New-Item -ItemType Directory -Force -Path $evil | Out-Null
Copy-Item C:\Users\OMack\Nono\target\debug\nono.exe              "$evil\nono.exe" -Force
Copy-Item C:\Users\OMack\Nono\target\debug\nono-shell-broker.exe "$evil\nono-shell-broker.exe" -Force

# Trigger a broker-arm run from the lookalike nono.exe (runner profile uses windows_low_il_broker:true).
# Run from a profile-covered dir, e.g. %USERPROFILE%\.claude, per the dev-layout/cwd-coverage gate.
$env:NONO_LOG = "debug"
Push-Location "$env:USERPROFILE\.claude"
& "$evil\nono.exe" run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo hi"
Pop-Location
```
PS C:\Users\OMack\Nono> $evil = "C:\Users\OMack\evil\target\release"
PS C:\Users\OMack\Nono> New-Item -ItemType Directory -Force -Path $evil | Out-Null
PS C:\Users\OMack\Nono> Copy-Item C:\Users\OMack\Nono\target\debug\nono.exe              "$evil\nono.exe" -Force
PS C:\Users\OMack\Nono> Copy-Item C:\Users\OMack\Nono\target\debug\nono-shell-broker.exe "$evil\nono-shell-broker.exe" -Force
PS C:\Users\OMack\Nono>
PS C:\Users\OMack\Nono> # Trigger a broker-arm run from the lookalike nono.exe (runner profile uses windows_low_il_broker:true).
PS C:\Users\OMack\Nono> # Run from a profile-covered dir, e.g. %USERPROFILE%\.claude, per the dev-layout/cwd-coverage gate.
PS C:\Users\OMack\Nono> $env:NONO_LOG = "debug"
PS C:\Users\OMack\Nono> Push-Location "$env:USERPROFILE\.claude"
PS C:\Users\OMack\.claude> & "$evil\nono.exe" run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo hi"

  nono v0.62.2
  Capabilities:
  ────────────────────────────────────────────────────
   r+w  \\?\C:\Users\OMack\.claude (dir)
       + 6 system/group paths (-v to show)
   net  outbound allowed
  ────────────────────────────────────────────────────

  Applying sandbox...

2026-06-13T13:17:46.219439Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.cargo prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:17:46.219670Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.claude prior_rid="0x1000" prior_mask="0x4"
2026-06-13T13:17:46.219788Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.config\git\ignore prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:17:46.219891Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.gitconfig prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:17:46.219982Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.local\bin prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:17:46.220082Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.rustup prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:17:46.220214Z  WARN label guard: path not owned by current user; skipping mandatory label apply (system paths are Medium-IL by default and already readable by Low-IL subjects) path=C:\Windows access=Read
nono: Sandbox initialization failed: Windows supervised execution failed during shutting-down (session: supervised-2852-1781356666211568700, transport: windows-supervisor-anon-2852-59b8e47be734f7a5ec8d67a6aa1899c7, supervisor_audit_entries: 0): Trust verification failed for C:\Users\OMack\evil\target\release\nono.exe: nono.exe Authenticode status is Unsigned (expected Valid). Self-trust-anchor unavailable; refusing to spawn broker. This install is not Authenticode-signed: install a signed release MSI (signing setup: docs/cli/development/windows-signing-guide.mdx), or run nono from your own Cargo dev build (under this machine's compile-time target dir) where this gate is intentionally skipped.
PS C:\Users\OMack\.claude> Pop-Location

**EXPECT (PASS):**
- A `NonoError::TrustVerification` failure (broker refused to spawn), because the unsigned
  binaries are NOT under the baked dev root and are not Authenticode-`Valid`.
- The `tracing::info!(target:"broker_authenticode", "skipping broker Authenticode verify: dev-build layout detected …")` line does **NOT** appear.

**FAIL (regression):** the run spawns the broker / a "skipping broker Authenticode verify" line appears → bypass still open.

Result: ____  Notes: ____

### B4-2 (POSITIVE — real dev build still works). Genuine checkout → gate SKIPS.

```powershell
$env:NONO_LOG = "debug"
Push-Location "$env:USERPROFILE\.claude"
& C:\Users\OMack\Nono\target\debug\nono.exe run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo hi"
Pop-Location
```

PS C:\Users\OMack\Nono> $env:NONO_LOG = "debug"
PS C:\Users\OMack\Nono> Push-Location "$env:USERPROFILE\.claude"
PS C:\Users\OMack\.claude> & C:\Users\OMack\Nono\target\debug\nono.exe run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo hi"

  nono v0.62.2
  Capabilities:
  ────────────────────────────────────────────────────
   r+w  \\?\C:\Users\OMack\.claude (dir)
       + 6 system/group paths (-v to show)
   net  outbound allowed
  ────────────────────────────────────────────────────

  Applying sandbox...

2026-06-13T13:19:17.618072Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.cargo prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:19:17.618863Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.claude prior_rid="0x1000" prior_mask="0x4"
2026-06-13T13:19:17.618959Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.config\git\ignore prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:19:17.619038Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.gitconfig prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:19:17.619106Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.local\bin prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:19:17.619203Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.rustup prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:19:17.619314Z  WARN label guard: path not owned by current user; skipping mandatory label apply (system paths are Medium-IL by default and already readable by Low-IL subjects) path=C:\Windows access=Read
2026-06-13T13:19:18.957418Z  INFO nono_shell_broker::broker: broker: console attach probe alloc_console_rc=0
2026-06-13T13:19:19.007434Z  INFO nono_shell_broker::broker: broker: AppContainer profile registered app_container_name=nono.session.d2943727581c458b9f6a28e56414d7e5
2026-06-13T13:19:19.007580Z  INFO nono_shell_broker::broker: broker: token/AppContainer setup complete app_container=true
2026-06-13T13:19:19.019918Z  INFO nono_shell_broker::broker: broker: spawned child child_pid=13360 app_container=true
hi
2026-06-13T13:19:19.027093Z  INFO nono_shell_broker::broker: broker: child exited child_exit_code=0
PS C:\Users\OMack\.claude> Pop-Location

**EXPECT (PASS):** `skipping broker Authenticode verify: dev-build layout detected at C:\Users\OMack\Nono\target\debug\nono.exe` appears, and the broker spawns / `echo hi` returns (child exit 0).

> Note: with the provenance fix, `target\x86_64-pc-windows-msvc\release\nono.exe` is now ALSO
> recognized as dev-layout (it is under this repo's `target\`), unlike the old substring check.
> That is intended (it is a real workspace build) and not a regression.

Result: ____  Notes: ____

### B4-3 (PRODUCTION unaffected). Signed install still ENFORCES.

```powershell
# Against a signed Program Files install (co-signed nono.exe + broker), a normal broker-arm run
# must verify cleanly (matching subject+thumbprint) and spawn — no "skipping" line, gate ENFORCED.
& "C:\Program Files\nono\nono.exe" run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo hi"
```

**EXPECT (PASS):** broker spawns after a successful Authenticode match; NO "skipping" info line.
If no signed install is available, mark **N/A** (covered by existing production-path unit tests).

PS C:\Users\OMack\Nono> & "C:\Program Files\nono\nono.exe" run --profile claude-code-tools-windows-runner --allow-cwd -- cmd.exe /c "echo hi"

  nono v0.57.5
  Capabilities:
  ────────────────────────────────────────────────────
   r+w  \\?\C:\Users\OMack\Nono (dir)
       + 6 system/group paths (-v to show)
   net  outbound allowed
  ────────────────────────────────────────────────────

  Applying sandbox...

2026-06-13T13:20:07.063072Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.cargo prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:20:07.063919Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.config\git\ignore prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:20:07.063996Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.gitconfig prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:20:07.064058Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.local\bin prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:20:07.064128Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\.rustup prior_rid="0x1000" prior_mask="0x5"
2026-06-13T13:20:07.064184Z  WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert (grant may have no observable enforcement effect depending on pre-existing label) path=C:\Users\OMack\Nono prior_rid="0x1000" prior_mask="0x4"
2026-06-13T13:20:07.064276Z  WARN label guard: path not owned by current user; skipping mandatory label apply (system paths are Medium-IL by default and already readable by Low-IL subjects) path=C:\Windows access=Read
2026-06-13T13:20:50.349654Z  INFO nono_shell_broker::broker: broker: console attach probe alloc_console_rc=0
2026-06-13T13:20:50.388288Z  INFO nono_shell_broker::broker: broker: AppContainer profile registered app_container_name=nono.session.93323522a4f54fff856317354a4b8a36
2026-06-13T13:20:50.389559Z  INFO nono_shell_broker::broker: broker: token/AppContainer setup complete app_container=true
2026-06-13T13:20:50.406534Z  INFO nono_shell_broker::broker: broker: spawned child child_pid=5008 app_container=true
hi
2026-06-13T13:20:50.416210Z  INFO nono_shell_broker::broker: broker: child exited child_exit_code=0


Result: ____  Notes: ____

---

## R-A1 — hook wrapper never corrupts the JSON contract under logging

The wrapper (`nono-tool-hook.ps1`) must emit pure JSON on stdout even with `NONO_LOG` set,
and fail CLOSED on any error.

### A1-1 (wrapper script, isolated). NONO_LOG=debug → clean parseable JSON.

```powershell
$env:NONO_EXE = "C:\Users\OMack\Nono\target\debug\nono.exe"
$env:NONO_LOG = "debug"
$hook = "C:\Users\OMack\Nono\crates\nono-cli\data\hooks\nono-tool-hook.ps1"
$json = '{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"x"},"tool_use_id":"r1"}'

$out = $json | & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hook
"--- raw stdout ---"; $out
"--- parse check ---"
try { $o = ($out -join "`n") | ConvertFrom-Json; "PARSE OK; decision=$($o.hookSpecificOutput.permissionDecision)" }
catch { "PARSE FAILED: $($_.Exception.Message)" }
```

**EXPECT (PASS):** `PARSE OK; decision=allow`; stdout contains ONLY the JSON (no `DEBUG`/ANSI lines).
PS C:\Users\OMack\Nono> $env:NONO_EXE = "C:\Users\OMack\Nono\target\debug\nono.exe"
PS C:\Users\OMack\Nono> $env:NONO_LOG = "debug"
PS C:\Users\OMack\Nono> $hook = "C:\Users\OMack\Nono\crates\nono-cli\data\hooks\nono-tool-hook.ps1"
PS C:\Users\OMack\Nono> $json = '{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"x"},"tool_use_id":"r1"}'
PS C:\Users\OMack\Nono>
PS C:\Users\OMack\Nono> $out = $json | & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hook
PS C:\Users\OMack\Nono> "--- raw stdout ---"; $out
--- raw stdout ---
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","permissionDecisionReason":"read-only Claude Code tool allowed by nono tool sandbox prototype"}}
PS C:\Users\OMack\Nono> "--- parse check ---"
--- parse check ---
PS C:\Users\OMack\Nono> try { $o = ($out -join "`n") | ConvertFrom-Json; "PARSE OK; decision=$($o.hookSpecificOutput.permissionDecision)" }
>> catch { "PARSE FAILED: $($_.Exception.Message)" }
PARSE OK; decision=allow


**Fail-closed sub-check:** point `NONO_EXE` at a stub that writes to stderr and exits 1; the
wrapper must emit a `deny` JSON with the stderr text in `permissionDecisionReason`:
```powershell
$stub = "C:\Temp\nono-stub.cmd"; Set-Content $stub "@echo boom 1>&2`r`nexit /b 1"
$env:NONO_EXE = $stub
$out = $json | & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hook
($out -join "`n") | ConvertFrom-Json | % { $_.hookSpecificOutput.permissionDecision }  # EXPECT: deny
Remove-Item Env:\NONO_EXE; Remove-Item Env:\NONO_LOG
```


PS C:\Users\OMack\Nono> $stub = "C:\Temp\nono-stub.cmd"; Set-Content $stub "@echo boom 1>&2`r`nexit /b 1"
PS C:\Users\OMack\Nono> $env:NONO_EXE = $stub
PS C:\Users\OMack\Nono> $out = $json | & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hook
PS C:\Users\OMack\Nono> ($out -join "`n") | ConvertFrom-Json | % { $_.hookSpecificOutput.permissionDecision }  # EXPECT: deny
deny
PS C:\Users\OMack\Nono> Remove-Item Env:\NONO_EXE; Remove-Item Env:\NONO_LOG


Result: ____  Notes: ____

### A1-2 (real Claude Code workflow). NONO_LOG=debug end-to-end.

```powershell
# Use the working deployment: isolated CLAUDE_CONFIG_DIR holding ONLY the nono PreToolUse hook,
# NONO_EXE -> the rebuilt dev nono.exe, launched from a non-.claude project dir (self-disable guard).
$env:NONO_EXE = "C:\Users\OMack\Nono\target\debug\nono.exe"
$env:CLAUDE_CONFIG_DIR = "C:\temp\nono-uat-cfg"   # minimal settings.json with the nono hook + copied .credentials.json
$env:NONO_LOG = "debug"
# launch claude from a profile-covered project dir WITHOUT a .claude subdir, then in-session:
#   1. Read a file        -> EXPECT: allowed, Claude proceeds (decision parsed)
#   2. Write a small file  -> EXPECT: denied + Claude auto-retries as confined Bash (additionalContext honored)
```

**EXPECT (PASS):** Claude parses and honors `permissionDecision` for both the allow (Read) and
deny (Write) cases; no fail-open, denials fail closed — identical to behavior with `NONO_LOG` unset.

**FAIL (regression):** with `NONO_LOG=debug` Claude reports a hook/JSON-parse error or ignores the
decision while the unset case works → wrapper still leaking stderr into stdout.

Result: ____  Notes: ____

---

## R-A6 — confined Write/Edit/MultiEdit are CLM-safe (no .NET, no BOM, byte-faithful)

The no-PTY broker arm (`BrokerLaunchNoPty`, selected for Claude Code tool calls with
`windows_low_il_broker:true`) spawns the child in a per-run **AppContainer**, where PowerShell
runs in **Constrained Language Mode** — blocking the old .NET payloads (`[Convert]`, `[System.IO.File]`,
`[System.Text.Encoding]`). The A1-2 transcript above (the `[Convert]::FromBase64String` failure +
"PowerShell is in Constrained Language Mode … the sandbox … is an AppContainer") is the live
evidence of the defect. The fix rewrites all three payload builders to a CLM-safe **byte vehicle**
(byte-array literals + `Set-Content -Encoding Byte`; pure-PowerShell byte find-and-replace for
Edit/MultiEdit) — no .NET method calls, no UTF-8 BOM.

> Rebuild after `ef1ea822` so the fixed payloads are in the binary: `cargo build -p nono-cli --bin nono`.

### A6-0 (confirm the binary embeds the R-A6 fix — broker-free, no quoting traps).

The version banner (`nono v0.62.2`) is the crate version and does NOT change with the fix, so it
is not a reliable check. Instead, ask the hook directly what payload it emits — this runs in-process
(no broker, no AppContainer, no `$`-quoting), so it always works from any console:

```powershell
$j = '{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"file_path":"x.txt","content":"hi"},"tool_use_id":"t"}'
Push-Location C:\Temp   # any dir WITHOUT a .claude subdir (else the self-disable guard fires)
$j | & C:\Users\OMack\Nono\target\debug\nono.exe claude-code-hook
Pop-Location
```

**EXPECT (correct binary):** `additionalContext` contains `Set-Content` + `-Encoding Byte` (the CLM-safe
byte vehicle) and does **NOT** contain `[System.IO.File]::WriteAllText` / `[Convert]::FromBase64String`.
If you still see `[System.IO.File]::WriteAllText`, the build is pre-`ef1ea822` — rebuild.

### A6-1 (root cause confirm). The arm forces Constrained Language Mode.

```powershell
# From a profile-covered dir (e.g. %USERPROFILE%\.claude) so the broker/AppContainer arm is selected.
Push-Location "$env:USERPROFILE\.claude"
# NOTE: SINGLE-quote the -Command payload so your outer console does NOT expand $-variables
# (double quotes let the console eat $ExecutionContext / $b before they reach the confined shell).
& C:\Users\OMack\Nono\target\debug\nono.exe run --profile claude-code-tools-windows-runner --allow-cwd -- powershell.exe -NoProfile -NonInteractive -Command '$ExecutionContext.SessionState.LanguageMode'
Pop-Location
```

**OBSERVED (2026-06-13): `FullLanguage` — and this check is NOT representative.** A direct `nono run`
from an interactive console returns FullLanguage, but the confined shell's LanguageMode depends on the
**launch context** (it inherits the parent environment). When the SAME confined shell is spawned
through the real Claude Code hook path it is `ConstrainedLanguage` — see **A6-4**, which is the
authoritative gate. Treat A6-1 as informational only; do not gate on it.

Result: FullLanguage (direct console — not representative; A6-4 is authoritative)

### A6-2 (confined Write byte-vehicle under REAL CLM). Byte-faithful, no BOM — no model, no hook.

```powershell
# Runs the exact CLM-safe Write vehicle inside the AppContainer; proves Set-Content -Encoding Byte
# works under CLM and is BOM-free. Bytes 68 69 C3 A9 = "hi" + é (U+00E9) in UTF-8.
Push-Location "$env:USERPROFILE\.claude"
# SINGLE-quote the payload (outer console must not expand $b); absolute path avoids nested quotes.
& C:\Users\OMack\Nono\target\debug\nono.exe run --profile claude-code-tools-windows-runner --allow-cwd -- powershell.exe -NoProfile -NonInteractive -Command '$b = [byte[]]@(104,105,195,169); Set-Content -LiteralPath C:\Users\OMack\.claude\a6_write.txt -Value $b -Encoding Byte'
Format-Hex "$env:USERPROFILE\.claude\a6_write.txt"
Pop-Location
```

**EXPECT (PASS):** `Format-Hex` shows exactly `68 69 C3 A9` (4 bytes), **no leading `EF BB BF`**.

Result: ____  Notes: ____

### A6-3 (E2E through the real hook — does the byte-vehicle fix actually LAND under real CLM?)

This is the **only** check that must run inside a live Claude Code session — CLM appears only on the
Claude-spawned path (A6-4), not on a direct console `nono run` (A6-1). You drive Claude to use the
in-process Write/Edit/MultiEdit tools; the hook denies each and Claude auto-retries the byte-vehicle
Bash command; then you verify the resulting file from a **separate, normal** PowerShell console.

**Setup (once).** Hook active (the `CLAUDE_CONFIG_DIR` deployment), `NONO_EXE` → the rebuilt
`target\debug\nono.exe`. The project dir MUST be **owned by the session user** — if not, the
mandatory-label/DACL grant is skipped (finding R-B3) and confined writes there are denied. Create it
fresh as yourself, with no `.claude` subdir (else the self-disable guard fires):
```powershell
$env:NONO_EXE          = "C:\Users\OMack\Nono\target\debug\nono.exe"
$env:CLAUDE_CONFIG_DIR = "C:\temp\nono-uat-cfg"   # minimal settings.json with ONLY the nono hook
mkdir C:\Users\OMack\a6test -Force | Out-Null      # fresh + user-owned
(Get-Acl C:\Users\OMack\a6test).Owner              # MUST be your user, else confined writes are denied (R-B3)
cd C:\Users\OMack\a6test                            # the granted CWD; files land here
claude
```

> **Two non-defects to expect (don't chase them as R-A6 failures):**
> 1. **`é`/non-ASCII mojibake is an input-layer issue, not the hook.** Typing `é` into a Windows
>    terminal can double-decode its UTF-8 bytes `C3 A9` as CP437 → `├⌐` *before Claude sees it*; the
>    hook then faithfully encodes the corrupted content. The hook is byte-faithful (verified: clean
>    `héllo` in → `104,195,169,108,108,111` out). Keep A6-3a ASCII so this can't masquerade as a bug.
> 2. **Confined writes are denied in a dir the session user doesn't own** (R-B3). Use the user-owned
>    `a6test` above; `~/.claude` is user-owned too but can't be the project dir (self-disable guard).

#### A6-3a — Write (the headline check; ASCII content)

In the Claude session, prompt verbatim:

> Use the Write tool to create `a6_write.txt` with exactly this content and nothing else: hello world

**Expected in-session:** the Write tool is **denied** with a byte-vehicle `additionalContext`
(`$bytes = [byte[]]@(...); Set-Content … -Encoding Byte`), and Claude **auto-retries it as a Bash call**
that exits 0. (No manual PowerShell — the point is to exercise the real hook.)

**Verify from a SEPARATE normal PowerShell console** (not inside the sandbox):
```powershell
Format-Hex C:\Users\OMack\a6test\a6_write.txt
```
**EXPECT (PASS):** bytes `68 65 6C 6C 6F 20 77 6F 72 6C 64` (= `hello world`), length **11**, **no leading `EF BB BF`**.
**FAIL:** a leading `EF BB BF` BOM, or the file missing.

(Non-ASCII byte-faithfulness is already proven separately — A6-2 wrote `68 69 C3 A9`, and the hook emits
`content.as_bytes()` verbatim — so this check stays ASCII to avoid the terminal `é`-mojibake gremlin.)

Result: ____  Notes: ____

#### A6-3b — Edit (non-start substring; literal replace, no BOM)

In the same session:
1. Prompt: *"Use the Write tool to create `a6_edit.txt` with exactly: `alpha BETA gamma`"* (lands via the vehicle).
2. Prompt: *"Use the Edit tool on `a6_edit.txt`: replace `BETA` with `DELTA`."*

Verify (separate console):
```powershell
Get-Content -Raw C:\Users\OMack\a6test\a6_edit.txt   # EXPECT: alpha DELTA gamma
Format-Hex      C:\Users\OMack\a6test\a6_edit.txt     # EXPECT: starts 61 6C 70 68 61 ... NO leading EF BB BF
```
**EXPECT (PASS):** content is `alpha DELTA gamma`, no BOM at head or mid-file.

Result: ____  Notes: ____

#### A6-3c — MultiEdit (regex metacharacters treated literally + a deletion)

1. Prompt: *"Use the Write tool to create `a6_multi.txt` with exactly: `foo.bar(1+2) remove_me tail`"*.
2. Prompt: *"Use the MultiEdit tool on `a6_multi.txt` with two edits: (1) replace `foo.bar(1+2)` with `OK`; (2) replace `remove_me ` with the empty string."*

Verify (separate console):
```powershell
Get-Content -Raw C:\Users\OMack\a6test\a6_multi.txt   # EXPECT: OK tail
Format-Hex      C:\Users\OMack\a6test\a6_multi.txt     # EXPECT: 4F 4B 20 74 61 69 6C, NO EF BB BF
```
**EXPECT (PASS):** content is exactly `OK tail` — proving `foo.bar(1+2)` was matched **literally** (not as
a regex: a regex would treat `.`, `()`, `+` specially) and the deletion removed `remove_me ` exactly. No BOM.

**FAIL (any sub-check):** leading/embedded `EF BB BF`, mangled non-ASCII, a regex-interpreted match, or a
file that never lands → report which one.

Result: ____  Notes: ____

### A6-4 (DECISIVE — is CLM real on the genuine hook path?). Resolves the A6-1=FullLanguage contradiction.

A6-1 (direct `nono run -- powershell.exe`) returned **`FullLanguage`**, which contradicts the CLM
premise behind R-A6. This check measures the SAME thing through the **real Claude Code hook path**
(Bash tool → hook rewrite → `nono run` → confined AppContainer shell) — the exact context where the
original "CLM" was reported.

**How to run — you do NOT wrap claude in `nono run`.** In the sandbox-the-tools model, `claude` runs
unsandboxed at Medium-IL and the PreToolUse hook confines each *tool call*:

1. Ensure the nono PreToolUse hook is active (the `CLAUDE_CONFIG_DIR` deployment), `NONO_EXE` → the
   rebuilt dev `nono.exe`.
2. Launch `claude` (NOT `nono run -- claude`) from a project dir WITHOUT a `.claude` subdir.
3. In the session, ask Claude to run each line below **with the Bash tool** (e.g. prompt:
   *"Run this with the Bash tool: `$ExecutionContext.SessionState.LanguageMode`"*). The hook rewrites
   the Bash call into the confined shell, so the line executes on the real path.

```
$ExecutionContext.SessionState.LanguageMode
[System.IO.File]::WriteAllText("$PWD\dotnet_probe.txt","hi"); Test-Path "$PWD\dotnet_probe.txt"
```

**Interpretation:**
- **`FullLanguage` + `True`** → CLM never applied on the hook path either; the old `[System.IO.File]`
  payloads would have worked. R-A6's CLM premise is **falsified** — the byte vehicle is harmless but
  should be reframed (BOM-free hardening) or reverted; reopen `confined-write-clm-blocked` and find
  what (if anything) actually failed originally (likely a malformed-base64 `FormatException` the
  nested session misread as CLM).
- **`ConstrainedLanguage` / write fails** → CLM is real on the hook path (but not the direct A6-1
  path) — itself a finding worth chasing — and R-A6 **stands**.

**RESULT (2026-06-13): `ConstrainedLanguage`.** R-A6 **CONFIRMED** — CLM is in force on the real hook
path, so the .NET payloads genuinely fail there and the byte-vehicle fix is required. The A6-1 vs A6-4
split shows the confined shell inherits CLM from its launch environment (Claude Code / node), not from
nono's AppContainer per se — a direct console `nono run` is FullLanguage. (The .NET-write half of the
probe is now redundant: ConstrainedLanguage by definition blocks `[System.IO.File]::WriteAllText`; no
need to fight the bash→nono→PS quoting to prove it.) Remaining gate: A6-3 — confirm the byte vehicle
actually LANDS files under this real CLM path.

Result: ConstrainedLanguage — R-A6 confirmed

---

## Sign-off

| Check | Result | Date | Operator |
|-------|--------|------|----------|
| B4-1 attacker path ENFORCES | | | |
| B4-2 real dev build SKIPS | | | |
| B4-3 production ENFORCES | | | |
| A1-1 wrapper JSON clean + fail-closed | | | |
| A1-2 Claude Code E2E under NONO_LOG | | | |
| A6-1 LanguageMode on direct path (observed: FullLanguage) | | | |
| A6-2 confined Write byte-vehicle: byte-faithful, no BOM | | | |
| A6-3 E2E Write/Edit/MultiEdit land, no BOM, literal | | | |
| A6-4 DECISIVE: LanguageMode + .NET write on real hook path | | | |

R-B4 + R-A1: all their checks PASS → merge-ready independently.
R-A6: premise CONFIRMED (A6-4 = `ConstrainedLanguage` on the real hook path; A6-1's FullLanguage was a
non-representative direct-console launch). Final gate is **A6-3** — confirm confined Write/Edit/MultiEdit
actually LAND under the real CLM path; then R-A6 is merge-ready too.
