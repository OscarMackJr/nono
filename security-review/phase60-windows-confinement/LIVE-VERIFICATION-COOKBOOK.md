# Live-Verification Cookbook — R-B4 & R-A1

Operator runbook for the two checks that could not be performed autonomously during the
`/gsd:debug` session. Run these on a **real Win11 host from an elevated-capable PowerShell
console** (NOT git-bash/MSYS — the broker arm needs a real console; `CreateProcessAsUserW`
returns GLE=87 otherwise).

- Fixes under test: commits `19f17ca4` (R-B4), `f48ec206` (R-A1) on branch `fix/win-confinement-rb4-ra1`.
- Build first so the fixed hook re-embeds and the dev-target-root bakes:
  ```powershell
  Set-Location C:\Users\OMack\Nono
  cargo build -p nono-cli --bin nono   # dev nono.exe at target\debug\nono.exe
  ```
- Record PASS/FAIL inline. All three R-B4 sub-checks must pass; both R-A1 checks must pass.

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

**Fail-closed sub-check:** point `NONO_EXE` at a stub that writes to stderr and exits 1; the
wrapper must emit a `deny` JSON with the stderr text in `permissionDecisionReason`:
```powershell
$stub = "C:\Temp\nono-stub.cmd"; Set-Content $stub "@echo boom 1>&2`r`nexit /b 1"
$env:NONO_EXE = $stub
$out = $json | & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $hook
($out -join "`n") | ConvertFrom-Json | % { $_.hookSpecificOutput.permissionDecision }  # EXPECT: deny
Remove-Item Env:\NONO_EXE; Remove-Item Env:\NONO_LOG
```

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

## Sign-off

| Check | Result | Date | Operator |
|-------|--------|------|----------|
| B4-1 attacker path ENFORCES | | | |
| B4-2 real dev build SKIPS | | | |
| B4-3 production ENFORCES | | | |
| A1-1 wrapper JSON clean + fail-closed | | | |
| A1-2 Claude Code E2E under NONO_LOG | | | |

All PASS → the R-B4 and R-A1 commits are field-verified; safe to merge `fix/win-confinement-rb4-ra1`.
