---
quick_id: 260508-m99
slug: broker-process-poc-minimal-rust-binary-t
type: research-code
completed: 2026-05-08T20:11:30Z
tasks_completed: 3/3
commits:
  - 2cb4071b: scaffold standalone poc-broker crate (Task 1)
  - 0095ab4a: implement Win32 broker mechanism (Task 2)
  - f5eebfc3: add user-runnable README (Task 3)
key_files:
  created:
    - .planning/quick/260508-m99-broker-process-poc-minimal-rust-binary-t/poc-broker/Cargo.toml
    - .planning/quick/260508-m99-broker-process-poc-minimal-rust-binary-t/poc-broker/src/main.rs
    - .planning/quick/260508-m99-broker-process-poc-minimal-rust-binary-t/README.md
  modified: []
---

# Quick Task 260508-m99: Broker-Process PoC Summary

## One-liner

Standalone Rust binary (`poc-broker`) that sequences `AllocConsole` + `DuplicateTokenEx(SecurityAnonymous, TokenPrimary)` + `SetTokenInformation(TokenIntegrityLevel, Low)` + `CreateProcessAsUserW(dwCreationFlags=0)` to validate whether a Low-IL child inherits the broker's console without retriggering CSRSS ALPC at Low IL (RESEARCH.md Assumption A1).

## What Was Built

**Crate:** `.planning/quick/260508-m99-broker-process-poc-minimal-rust-binary-t/poc-broker/`
**Binary name:** `poc-broker.exe`
**Source:** `src/main.rs` — 196 lines including comments

**Structural isolation:**
- `Cargo.toml` contains `[workspace]` empty section — prevents Cargo crawling up to the parent nono workspace
- `windows-sys = "0.59"` under `[target.'cfg(windows)'.dependencies]` — matches workspace pin
- Parent workspace `Cargo.toml` `[workspace.members]` is unchanged (nono, nono-cli, nono-proxy, bindings/c only)

**Implementation steps:**
1. `AllocConsole()` — attaches to console at Medium IL; non-fatal if parent already has one
2. `OpenProcessToken(GetCurrentProcess(), ...)` — opens current token for duplication
3. `DuplicateTokenEx(SecurityAnonymous, TokenPrimary)` — mirrors launch.rs:1103-1108 (CR-01 hygiene)
4. `CreateWellKnownSid(WinLowLabelSid)` + `TOKEN_MANDATORY_LABEL` inline construction
5. `SetTokenInformation(TokenIntegrityLevel, Low)` — lowers duplicate token to Low IL
6. `CreateProcessAsUserW` with `dwCreationFlags=0` — no CREATE_NEW_CONSOLE; child inherits broker console
7. Wait + exit code decode: PASS (0) / FAIL-A (0xC0000142 STATUS_DLL_INIT_FAILED) / FAIL-B (other)
8. `CloseHandle` cleanup on all four handles

All `unsafe` blocks carry `// SAFETY:` comments. Non-Windows stub included for Linux/macOS builds.

## Build Status

**Build:** clean — `cargo build --release --target x86_64-pc-windows-msvc` from `poc-broker/` succeeds on Windows host. Initial build had 2 import-path errors (`OpenProcessToken` lives in `Win32::System::Threading`, `SE_GROUP_INTEGRITY` lives in `Win32::System::SystemServices`) — fixed in commit `17d87c7f`.

**Cargo.toml feature additions:** `Win32_System_SystemServices` added alongside `Win32_System_Threading` / `Win32_Security` / `Win32_Foundation` / `Win32_System_Console` per windows-sys 0.59 module layout.

## Field-Test Result: ✅ PASS — A1 EMPIRICALLY VALIDATED (2026-05-08)

User ran the PoC on the Windows test box (Windows 10/11, normal Medium-IL PowerShell). Verbatim output:

```
[POC] AllocConsole rc=0 (0=inherited parent console, non-zero=new console)
[POC] Mechanism: AllocConsole + DuplicateTokenEx(SecurityAnonymous,TokenPrimary) + SetTokenInformation(Low) + CreateProcessAsUserW(dwCreationFlags=0)
[POC] Child PID: 29996
[POC] Waiting for child...
PS C:\...\poc-broker> $PID
29996
PS C:\...\poc-broker> whoami /groups | Select-String "Mandatory Label"
Mandatory Label\Low Mandatory Level        Label            S-1-16-4096
PS C:\...\poc-broker> exit
[POC] Child exit code: 0x00000000 (0)
[POC] PASS — broker pattern viable; child survived KernelBase DllMain at Low-IL
```

### Diagnostic evidence

| Probe | Outcome | Significance |
|---|---|---|
| `[POC] Child PID: 29996` | child spawned successfully | broker → child CreateProcessAsUserW worked |
| `$PID` (in spawned shell) = `29996` | matches reported child PID | user IS interacting with the spawned Low-IL child, not the outer Medium-IL shell |
| `whoami /groups` Mandatory Label | `Low Mandatory Level S-1-16-4096` | child is at Low IL — token integrity drop succeeded and persisted to the child |
| Spawned shell renders prompt + accepts input | interactive | KernelBase DllMain completed (no `STATUS_DLL_INIT_FAILED`) and the console attach worked via inheritance |
| PSReadLine `Access to the path '...ConsoleHost_history.txt' is denied` | OS-level write-deny | mandatory-label NO_WRITE_UP enforcement active — Low-IL child cannot write to Medium-IL `AppData\Roaming` path. **This is the security envelope working as designed.** |
| `[POC] Child exit code: 0x00000000 (0)` | clean exit | child ran to completion (user typed `exit`); no crash, no NTSTATUS abort |

### What this validates

**RESEARCH.md Assumption A1** — *"KernelBase's `ConClntInitialize` skips the CSRSS ALPC connect when the child inherits the parent's console (no CREATE_NEW_CONSOLE flag)"* — is **empirically confirmed on this Windows test box**. The Phase 30 ProcMon evidence localized the failure to CSRSS console-subsystem ALPC denial during KernelBase DllMain at Low-IL; the broker pattern (Medium-IL parent holds console, Low-IL child inherits) bypasses that denial path because KernelBase short-circuits the CSRSS attach when a console handle is already inherited.

### Architecture confirmed

```
outer PowerShell (Medium IL — holds CSRSS console attachment from session start)
  └── poc-broker.exe (Medium IL, inherits console — does NOT need to call AllocConsole; rc=0)
       └── powershell.exe -NoLogo (Low IL via duplicated token, inherits broker's console;
            KernelBase DllMain detects inherited console → skips CSRSS attach → survives)
```

For Phase 31 production lift: the broker MUST be a Medium-IL process spawned by `nono.exe` (which is itself Medium-IL). Only the broker's CHILD gets the Low-IL token. A Low-IL broker would re-trigger the CSRSS denial.

## Next Step (UPDATED post-PASS)

**Phase 31 broker-process pattern is now de-risked.** Recommended path:

1. **Today:** Update `30-WAVE-2-PROCMON.md` and the resolved debug session with the A1 validation evidence (this commit). Update `PROJECT.md` to flag SHELL-01 ✘→Phase 31 candidate (was v3.0 deferral).
2. **Next session:** `/gsd-phase add 31 "Windows nono shell broker-process pattern"` then `/gsd-discuss-phase 31` with this PoC outcome + RESEARCH.md as locked scoping inputs.
3. **Phase 31 execution:** ~7 days per RESEARCH.md effort estimate (PoC eliminates the 1.5-day de-risking step). Lift mechanism into `crates/nono-shell-broker/`, wire `launch.rs` to dispatch, fix harness `Out-File` false-PASS, re-run field smoke for Acceptance #1-#4, ship cookbook update flipping SHELL-01 ✘→✔.

## Deviations from Plan

1. Build encountered 2 import-path errors not anticipated by planner; fixed inline (commit `17d87c7f`).
2. 3 unsafe blocks lacked `// SAFETY:` annotations from initial executor pass; added in commit `9282cd34` for hygiene parity with production code Phase 31 will lift.

Both deviations are documentation/correctness fixes, not architectural changes. The mechanism the plan specified is exactly what shipped and what the user ran successfully.

## Self-Check: PASSED

- poc-broker/Cargo.toml: FOUND (with corrected feature list)
- poc-broker/src/main.rs: FOUND (196 lines + 2-line SAFETY hygiene)
- README.md: FOUND
- Build clean on Windows: VERIFIED
- Field-test outcome: **PASS** (child exit code 0; Low-IL confirmed; write-deny confirmed)
- Parent Cargo.toml members: UNCHANGED (no poc-broker added)
- A1 assumption status: **EMPIRICALLY VALIDATED**

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- poc-broker/Cargo.toml: FOUND
- poc-broker/src/main.rs: FOUND (196 lines)
- README.md: FOUND
- Commit 2cb4071b (scaffold): FOUND
- Commit 0095ab4a (Win32 impl): FOUND
- Commit f5eebfc3 (README): FOUND
- Parent Cargo.toml members: UNCHANGED (no poc-broker added)
