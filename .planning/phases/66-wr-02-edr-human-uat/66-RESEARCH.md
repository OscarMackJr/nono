# Phase 66: WR-02 EDR HUMAN-UAT - Research

**Researched:** 2026-06-11
**Domain:** Windows EDR observation methodology — Sysmon + Defender AV, MIC boundary verification, T1134.002 behavioral detection, two-pass exclusion mechanics
**Confidence:** HIGH (core MIC and Defender API mechanics), MEDIUM (Sysmon event field corpus), LOW (Defender behavioral heuristic thresholds for T1134.002 specifically)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **EDR runner:** Sysmon v15.20, schema 4.91, SwiftOnSecurity config + built-in Microsoft Defender Antivirus (Normal mode, real-time + quarantine validated). MDE is NOT available.
- **Host:** Azure VM `nono-fltmgr-vm`, Win11 build 26200, production-signed v0.62.2 machine MSI (Authenticode Valid). Machine MSI is REQUIRED — broker trust gate D-32-12 only spawns from a signed Program-Files install.
- **Exercising command:** `nono run --profile claude-code -- cmd /c whoami /groups` — triggers BrokerLaunchNoPty arm → broker `create_low_integrity_primary_token` + `CreateProcessAsUserW(low_il_token)` = MITRE T1134.002 sequence → Low-IL child with NO_WRITE_UP mandatory label.
- **Two-pass structure:** no-exclusion first (false-positive exposure), then with-exclusion (`Add-MpPreference`). Security invariant: exclusions must NOT weaken the OS MIC enforcement.
- **EDR-proxy caveat:** Sysmon + Defender AV is a representative proxy, NOT full cloud-EDR. WR-02 closes as "validated under a representative EDR-proxy"; MDE re-run is future optional.

### Claude's Discretion
- Exact observation methodology for each boundary (which events, which queries, how to read them)
- Precise wording of ~10 assertions
- Publisher-trust-state recording methodology

### Deferred Ideas (OUT OF SCOPE)
- MDE (Defender for Endpoint) run — re-run same matrix under MDE if/when tenant available
- EDR telemetry emission / EDR-evasion-resistance hardening (EDR-INTEG-01)
- CI-runner EDR UAT — requires real host
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EDR-01 | HUMAN-UAT artifact records ~10 pass/fail assertions in two passes (no-exclusion then with-exclusion), each recording EDR product + version + policy mode; distinguishes alert from quarantine | Research provides exact PowerShell queries for Defender, Sysmon event corpus, and assertion methodology for both passes |
| EDR-02 | Validates (a) whether EDR DLL-injection into Low-IL children fails at NO_WRITE_UP MIC boundary, and (b) whether broker's CreateProcessAsUserW + SetTokenInformation(IntegrityLevel) sequence (T1134.002) triggers EDR alerts/quarantine; WR-02 closed or re-scoped | Research provides MIC directional rules, correct observation commands for (a) and (b), alert-vs-quarantine disambiguation, and close/re-scope decision criteria |
</phase_requirements>

---

## Summary

Phase 66 is a pure HUMAN-UAT phase: no source changes, no new tests. The entire deliverable is a `66-HUMAN-UAT.md` checklist with ~10 pass/fail assertions executed by a human operator. This research answers the hard methodological question: **how do you concretely observe, with specific commands, what the EDR sees when nono's broker-pattern Low-IL launch runs?**

Three interconnected problem areas require precise methodology before the planner can write useful assertions:

**EDR-02(a) — MIC boundary direction (the most commonly mis-stated nuance).** The `NO_WRITE_UP` policy on the Low-IL child governs what the child can *write up to*. It does NOT block a Medium-IL process (such as an EDR monitoring agent) from injecting *down* into the Low-IL child. MIC is directional: low-to-high writes are blocked; high-to-low injection is permitted by the kernel. Therefore "EDR DLL-injection fails at NO_WRITE_UP" is actually the wrong framing — Defender/Sysmon can inject their monitoring DLLs into the Low-IL child because they operate from Medium or System IL. What the assertion actually validates is that the **child cannot escalate writes back upward** (the structural containment guarantee), NOT that the EDR is blind. The observation method must therefore be precise about what "MIC boundary holds" actually means empirically.

**EDR-02(b) — T1134.002 surface.** The `nono-shell-broker.exe` broker runs at the caller's identity (Medium IL), calls `OpenProcessToken` + `DuplicateTokenEx` on its own process, then `SetTokenInformation(TokenIntegrityLevel, WinLowLabelSid)` to lower the duplicated token, then passes it to `CreateProcessAsUserW`. This is the textbook T1134.002 sequence (create process with integrity-manipulated token). Sysmon Event 10 can log the OpenProcess/DuplicateTokenEx aspect; Sysmon Event 1 logs the resulting Low-IL child with an IntegrityLevel=Low field. Windows Security Event 4688 also records the new process with its Mandatory Label SID. Defender's behavioral engine may or may not flag this pattern depending on whether it is in its heuristic set — research indicates this is LOW confidence without live testing; the UAT itself resolves the question.

**Publisher trust confounder.** Defender's reputation engine (SmartScreen + cloud-file-reputation) can alert on a binary purely because its Authenticode signer is not yet in the reputation whitelist (a new or POC cert), independently of T1134.002 behavior. The UAT must record the signer's trust status upfront so that any Defender alerts in Pass 1 can be categorized as reputation-driven vs. behavioral.

**Primary recommendation:** Each assertion in the 66-HUMAN-UAT.md must specify the exact PowerShell or Event Viewer command to run, the exact field to check, and the concrete pass criterion — not just "check Sysmon." The research below provides those commands at assertion-resolution.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Low-IL child spawn (T1134.002) | nono-shell-broker.exe (Medium-IL process) | nono.exe (supervisor) | Broker calls create_low_integrity_primary_token + CreateProcessAsUserW; nono.exe only dispatches via BrokerLaunchNoPty arm |
| MIC NO_WRITE_UP enforcement | Windows kernel (MIC pre-DACL check) | — | Mandatory label evaluation happens in the kernel before DACL; no user-mode code path can bypass it |
| T1134.002 behavioral detection | Defender AV behavioral engine (cbengine.sys / mpengine.dll) | AMSI | Defender's scan engine inspects API call sequences; AMSI is for script content, not relevant here |
| Token sequence telemetry | Sysmon driver (sysmondrv.sys) via ETW | Windows Security Audit log | Sysmon Event 10 captures OpenProcess on the broker's own process; Event 1 captures the Low-IL child create |
| DLL injection into Low-IL child | Medium-IL injectors (EDR agents, Sysmon itself) | — | MIC direction: Medium IL can inject DOWN into Low IL; only low-to-high writes are blocked |
| Alert-vs-quarantine state | Defender AV (mpscv.exe / MpEngine) | WMI root\Microsoft\Windows\Defender | Get-MpThreatDetection reads from the WMI namespace; ThreatStatusID is the authoritative field |
| Exclusion scoping | Defender AV (mpscv.exe) | Windows Registry | Add-MpPreference writes to HKLM\SOFTWARE\Microsoft\Windows Defender; exclusions are AV-scoping only, no effect on kernel MIC |

---

## Methodology: The Six Research Questions

### RQ1 — EDR-02(a): What "MIC boundary holds" actually means and how to observe it

**The directional rule (VERIFIED from official docs [CITED: learn.microsoft.com/en-us/windows/win32/secauthz/mandatory-integrity-control]):**

MIC's `SYSTEM_MANDATORY_LABEL_NO_WRITE_UP` policy is the default on all objects. It means:
- A **Low-IL subject** cannot write to (or WriteProcessMemory into, or CreateRemoteThread into) a **Medium-IL or higher object**. The kernel blocks this BEFORE the DACL check.
- A **Medium-IL or higher subject** CAN write to a **Low-IL object**. This is not blocked by MIC.

**Therefore the correct framing of EDR-02(a) is:**

> "The Low-IL child cannot write back up to the Medium-IL supervisor or any Medium-IL system object — the `NO_WRITE_UP` label protects the rest of the system from the contained child."

NOT: "The EDR cannot inject into the Low-IL child." (That injection is permitted and expected — the EDR's monitoring DLL can load into the Low-IL child because the EDR agent runs at Medium/System IL.)

**Concrete observation methods for EDR-02(a):**

**(a1) Verify the child's integrity level via `whoami /groups` output:**
```powershell
# The exercising command itself is the proof:
nono run --profile claude-code -- cmd /c whoami /groups
# Pass criterion: output contains "Mandatory Label\Low Mandatory Level" SID S-1-16-4096
```

**(a2) Sysmon Event 1 — IntegrityLevel field on the grandchild cmd.exe:**
```powershell
# Query Sysmon event log for the cmd.exe ProcessCreate with IntegrityLevel=Low
Get-WinEvent -LogName "Microsoft-Windows-Sysmon/Operational" |
  Where-Object { $_.Id -eq 1 } |
  Select-Object -Last 20 |
  ForEach-Object {
    $xml = [xml]$_.ToXml()
    $data = $xml.Event.EventData.Data
    [PSCustomObject]@{
      TimeCreated    = $_.TimeCreated
      Image          = ($data | Where-Object Name -eq 'Image').'#text'
      IntegrityLevel = ($data | Where-Object Name -eq 'IntegrityLevel').'#text'
      CommandLine    = ($data | Where-Object Name -eq 'CommandLine').'#text'
      ParentImage    = ($data | Where-Object Name -eq 'ParentImage').'#text'
    }
  } | Where-Object { $_.IntegrityLevel -eq 'Low' }
# Pass criterion: a record appears where Image ends in \cmd.exe, IntegrityLevel = "Low",
# CommandLine contains "whoami /groups", ParentImage ends in \nono-shell-broker.exe
```

**(a3) Controlled write-up probe — verifying NO_WRITE_UP blocks the child (optional depth-of-defense):**
```powershell
# From WITHIN the Low-IL cmd.exe child (pipe output back, do not run as operator):
# whoami /groups shows Low Mandatory Level — structural proof.
# A deeper probe: attempt to write to a Medium-IL temp file from the Low-IL child.
# If blocked with ACCESS_DENIED, NO_WRITE_UP is confirmed.
# This is advisory — the mandatory-label on the child token IS the structural proof.
# The assertion does not require this extra step; whoami /groups + Sysmon Event 1 are sufficient.
```

**(a4) EDR DLL observation — is the EDR's monitoring DLL loaded into the Low-IL child?**

This is NOT a failure — it is expected behavior. A Medium-IL EDR injecting its monitoring DLL down into a Low-IL process is MIC-legal. The assertion records WHAT happened, not a pass/fail on the injection itself.

```powershell
# Method: Sysmon Event 7 (ImageLoad) with the child cmd.exe PID as the target
# First, get the PID of the cmd.exe child from Event 1 above
# Then query Event 7 entries logged during the run that loaded into that PID:
Get-WinEvent -LogName "Microsoft-Windows-Sysmon/Operational" |
  Where-Object { $_.Id -eq 7 } |
  ForEach-Object {
    $xml = [xml]$_.ToXml()
    $data = $xml.Event.EventData.Data
    [PSCustomObject]@{
      TimeCreated = $_.TimeCreated
      ProcessId   = ($data | Where-Object Name -eq 'ProcessId').'#text'
      Image       = ($data | Where-Object Name -eq 'Image').'#text'
      ImageLoaded = ($data | Where-Object Name -eq 'ImageLoaded').'#text'
      Signed      = ($data | Where-Object Name -eq 'Signed').'#text'
    }
  } | Where-Object { $_.ProcessId -eq '<child_cmd_pid>' }
# Expected: system DLLs (ntdll.dll, kernel32.dll, etc.) are present.
# Defender/Sysmon monitoring DLL (MpClient.dll, SysmonDrv callbacks) may appear.
# Key field to record: whether any unsigned DLL loaded — Signed = 'false' is notable.
# The assertion records the DLL list, not a binary pass/fail on EDR injection.
```

**Summary for assertion wording:** Assertion EDR-02(a) should be: "Sysmon Event 1 shows the child cmd.exe process has IntegrityLevel=Low AND `whoami /groups` output contains `Mandatory Label\Low Mandatory Level`." The MIC boundary holding means the child token is correctly labelled; NO_WRITE_UP prevents the child from writing up. Whether the EDR's monitoring DLL loaded into the child is RECORDED as a finding (not a pass/fail criterion), because Medium-IL injection down is MIC-legal.

---

### RQ2 — EDR-02(b): T1134.002 detection — Sysmon events and Defender alerts

**The token-manipulation sequence in nono-shell-broker.exe:**

1. `OpenProcessToken(GetCurrentProcess(), TOKEN_DUPLICATE|TOKEN_QUERY|TOKEN_ADJUST_DEFAULT|TOKEN_ASSIGN_PRIMARY)` — broker opens its own token
2. `DuplicateTokenEx(...)` → primary token copy
3. `SetTokenInformation(primary_token, TokenIntegrityLevel, WinLowLabelSid)` — integrity downgrade
4. `CreateProcessAsUserW(low_il_token, cmd.exe, ...)` — spawn child with lowered token

**Sysmon events that surface this sequence:**

**(b1) Sysmon Event 10 — ProcessAccess on the broker's own process:**

The broker calls `OpenProcessToken(GetCurrentProcess(), ...)`. When a process opens its own process handle for token operations, Sysmon may or may not log Event 10 (the SwiftOnSecurity config's default may filter self-access). Sysmon Event 10 is primarily designed to catch CROSS-process opens (one process opening another). Self-OpenProcess for token duplication may not appear in Sysmon Event 10 under the SwiftOnSecurity config. [ASSUMED — the SwiftOnSecurity config filtering logic for self-opens is not verified in this session.]

**(b2) Sysmon Event 1 — the Low-IL grandchild is the primary observable signal:**

```powershell
# The most reliable T1134.002 observable: the grandchild cmd.exe has a DIFFERENT
# IntegrityLevel than its parent nono-shell-broker.exe.
# nono-shell-broker.exe runs at Medium IL (caller's identity = nono.exe's token)
# cmd.exe child runs at Low IL (CreateProcessAsUserW(low_il_token))
# This mismatch IS the T1134.002 behavioral signature Sysmon Event 1 captures.
Get-WinEvent -LogName "Microsoft-Windows-Sysmon/Operational" |
  Where-Object { $_.Id -eq 1 } |
  ForEach-Object {
    $xml = [xml]$_.ToXml()
    $data = $xml.Event.EventData.Data
    [PSCustomObject]@{
      TimeCreated    = $_.TimeCreated
      Image          = ($data | Where-Object Name -eq 'Image').'#text'
      IntegrityLevel = ($data | Where-Object Name -eq 'IntegrityLevel').'#text'
      ParentImage    = ($data | Where-Object Name -eq 'ParentImage').'#text'
    }
  } | Where-Object { $_.Image -match 'cmd\.exe' -and $_.ParentImage -match 'broker' }
# Pass criterion: ParentImage has IntegrityLevel=Medium AND child has IntegrityLevel=Low
# (You infer parent IL from the broker's own earlier ProcessCreate event)
```

**(b3) Windows Security Event 4688 — process creation with Mandatory Label:**
```powershell
# WinSec Audit must have "Audit Process Creation" enabled (check first):
# auditpol /get /subcategory:"Process Creation"
# If enabled, Event 4688 records the Mandatory Label SID of the new process.
# The Low-IL cmd.exe shows: MandatoryLabel = S-1-16-4096 (Low Mandatory Level)
Get-WinEvent -LogName Security |
  Where-Object { $_.Id -eq 4688 } |
  Select-Object -Last 30 |
  ForEach-Object {
    $xml = [xml]$_.ToXml()
    $data = $xml.Event.EventData.Data
    [PSCustomObject]@{
      TimeCreated      = $_.TimeCreated
      NewProcessName   = ($data | Where-Object Name -eq 'NewProcessName').'#text'
      CommandLine      = ($data | Where-Object Name -eq 'CommandLine').'#text'
      MandatoryLabel   = ($data | Where-Object Name -eq 'MandatoryLabel').'#text'
    }
  } | Where-Object { $_.NewProcessName -match 'cmd\.exe' }
# Pass criterion: MandatoryLabel = S-1-16-4096 (or "Mandatory Label\Low Mandatory Level" text)
# NOTE: Audit Process Creation subcategory must be enabled. If not enabled, this event won't fire.
# Sysmon Event 1 is the backup observable and does NOT require audit policy changes.
```

**(b4) Defender AV behavioral alert — T1134.002 specific:**

Defender AV's real-time protection engine may or may not fire on the token-downgrade sequence. Research finds [ASSUMED]:
- Defender's behavioral engine does observe `SetTokenInformation(TokenIntegrityLevel)` calls (it has ETW hooks at the kernel)
- Whether it generates an alert on integrity-DOWN sequences (vs. UP sequences which are more suspicious) is NOT documented in public Defender signatures
- The sandbox pattern (Medium-IL spawning Low-IL via CreateProcessAsUserW) is used by Chrome, IE Protected Mode, Adobe Reader, etc. — Defender is likely trained to NOT alert on this pattern to avoid massive false positives

**Practical implication for the assertion:** The T1134.002 behavioral detection assertion should be: "Run `Get-MpThreatDetection` after the exercising command; record whether any threat was logged. An absence of a Defender alert is a VALID result and is expected for a well-known sandboxing pattern." The assertion is NOT "Defender must alert" — it is "record what Defender does and whether it distinguishes alert from quarantine."

```powershell
# BEFORE the exercising command, capture a baseline:
$before = Get-MpThreatDetection | Select-Object ThreatID, InitialDetectionTime
nono run --profile claude-code -- cmd /c whoami /groups
# AFTER, compare:
$after = Get-MpThreatDetection | Select-Object ThreatID, InitialDetectionTime, ThreatStatusID, CleaningActionID, ActionSuccess, Resources
$new = Compare-Object -ReferenceObject $before -DifferenceObject $after -Property ThreatID |
  Where-Object { $_.SideIndicator -eq '=>' }
if ($new) {
  Write-Host "NEW THREATS DETECTED: $($new.ThreatID -join ', ')"
  $after | Where-Object { $_.ThreatID -in $new.ThreatID }
} else {
  Write-Host "No new Defender threats detected (expected for sandboxing pattern)"
}
```

---

### RQ3 — Publisher-trust confounder recording

The v0.62.2 MSI's Authenticode signer may be a POC/organization certificate (not a publicly-trusted CA-chained signer in Microsoft's reputation database). If Defender fires in Pass 1, it may be a reputation alert on the binary, NOT a T1134.002 behavioral alert. These must be distinguished.

**Baseline capture commands (run once, before any test pass):**

```powershell
# Check signature status on both nono.exe and nono-shell-broker.exe:
$nonoExe = "C:\Program Files\nono\nono.exe"
$brokerExe = "C:\Program Files\nono\nono-shell-broker.exe"

$sig_nono = Get-AuthenticodeSignature -FilePath $nonoExe
$sig_broker = Get-AuthenticodeSignature -FilePath $brokerExe

[PSCustomObject]@{
  Binary          = $nonoExe
  Status          = $sig_nono.Status           # Valid / NotSigned / HashMismatch / UnknownError
  StatusMessage   = $sig_nono.StatusMessage
  SignerSubject   = $sig_nono.SignerCertificate.Subject
  Thumbprint      = $sig_nono.SignerCertificate.Thumbprint
  Issuer          = $sig_nono.SignerCertificate.Issuer
  NotAfter        = $sig_nono.SignerCertificate.NotAfter
}

[PSCustomObject]@{
  Binary          = $brokerExe
  Status          = $sig_broker.Status
  StatusMessage   = $sig_broker.StatusMessage
  SignerSubject   = $sig_broker.SignerCertificate.Subject
  Thumbprint      = $sig_broker.SignerCertificate.Thumbprint
  Issuer          = $sig_broker.SignerCertificate.Issuer
  NotAfter        = $sig_broker.SignerCertificate.NotAfter
}
```

**Interpretation:**
- `Status = Valid` AND `Issuer` chains to a public CA (DigiCert, Sectigo, etc.) → reputation is likely positive; any Defender alert is behavioral
- `Status = Valid` AND `Issuer` is a self-signed or org-only CA not in Microsoft's trust store → reputation signal may dominate in Pass 1; SmartScreen/cloud-file-rep may flag the binary; behavioral vs. reputation alerts cannot be cleanly separated without MDE cloud context
- Record the full Issuer string in the UAT baseline — it is a confounder for every Pass 1 Defender finding

[CITED: learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-authenticodesignature]

---

### RQ4 — Two-pass exclusion mechanics and the security invariant

**Pass 1 commands (no exclusions — run before any exclusion is added):**
```powershell
# Verify no existing exclusions for nono paths:
(Get-MpPreference).ExclusionPath | Where-Object { $_ -match 'nono' }
(Get-MpPreference).ExclusionProcess | Where-Object { $_ -match 'nono' }
# If empty: proceed to Pass 1 exercising command.
```

**Pass 2 exclusion setup:**
```powershell
# Add exclusions for the nono install directory and both executables:
Add-MpPreference -ExclusionPath "C:\Program Files\nono"
Add-MpPreference -ExclusionProcess "C:\Program Files\nono\nono.exe"
Add-MpPreference -ExclusionProcess "C:\Program Files\nono\nono-shell-broker.exe"
# Verify:
(Get-MpPreference).ExclusionPath
(Get-MpPreference).ExclusionProcess
```

**Pass 2 cleanup (restore after both passes are complete):**
```powershell
Remove-MpPreference -ExclusionPath "C:\Program Files\nono"
Remove-MpPreference -ExclusionProcess "C:\Program Files\nono\nono.exe"
Remove-MpPreference -ExclusionProcess "C:\Program Files\nono\nono-shell-broker.exe"
# Verify clean:
(Get-MpPreference).ExclusionPath | Where-Object { $_ -match 'nono' }  # must be empty
(Get-MpPreference).ExclusionProcess | Where-Object { $_ -match 'nono' }  # must be empty
```

[CITED: learn.microsoft.com/en-us/powershell/module/defender/add-mppreference, learn.microsoft.com/en-us/powershell/module/defender/remove-mppreference]

**The security invariant assertion:**

AV exclusions are AV-scoping only. They do NOT touch the Windows kernel's MIC enforcement. The mandatory label on the Low-IL child's access token is set by `SetTokenInformation(TokenIntegrityLevel)` BEFORE `CreateProcessAsUserW` is called — no AV exclusion can alter this. To assert the security invariant:

```powershell
# After adding Pass 2 exclusions AND running the exercising command:
# Repeat the whoami /groups check and Sysmon Event 1 query.
# Pass criterion: IntegrityLevel=Low is still present DESPITE the AV exclusion.
# (The MIC boundary is OS-level; AV exclusions only affect scan behavior.)
nono run --profile claude-code -- cmd /c whoami /groups
# Expected: still outputs "Mandatory Label\Low Mandatory Level"
# Re-run the Sysmon Event 1 query and confirm IntegrityLevel=Low still appears.
```

---

### RQ5 — Alert vs. quarantine disambiguation and WR-02 close/re-scope criteria

**The alert vs. quarantine distinction (VERIFIED from official WMI docs [CITED: learn.microsoft.com/en-us/previous-versions/windows/desktop/defender/msft-mpthreatdetection]):**

The authoritative fields in `Get-MpThreatDetection` output:

| Field | Type | Meaning |
|-------|------|---------|
| `ThreatStatusID` | uint8 | Current state: 1=Detected(alert only), 3=Quarantined, 4=Removed, 6=Blocked, 102=QuarantineFailed, 103=RemoveFailed |
| `ActionSuccess` | boolean | Whether the cleaning action completed successfully |
| `CleaningActionID` | uint8 | Action taken: 0=Unknown, 1=Clean, 2=Quarantine, 3=Remove, 4=Allow, 5=UserDefined, 6=NoAction, 7=Block [CITED: community docs via learn.microsoft.com/en-us/answers/questions/4110692/] |
| `Resources` | string[] | The file(s) or process(es) affected |

**Diagnostic query for each test scenario:**
```powershell
Get-MpThreatDetection | Select-Object @{N='Threat';E={
    (Get-MpThreat -ThreatID $_.ThreatID).ThreatName
  }}, ThreatStatusID, CleaningActionID, ActionSuccess, Resources, InitialDetectionTime |
  Sort-Object InitialDetectionTime -Descending | Select-Object -First 10
```

**Decision matrix for per-assertion verdict:**

| `ThreatStatusID` | `ActionSuccess` | Classification | Verdict for UAT |
|------------------|-----------------|----------------|-----------------|
| 0 (Unknown) or absent | — | No detection | "EDR did not alert" |
| 1 (Detected) | false | Alert logged, no action taken | "EDR alerted (no quarantine)" |
| 3 (Quarantined) | true | File quarantined | "EDR quarantined binary" |
| 4 (Removed) | true | File deleted | "EDR removed binary" |
| 6 (Blocked) | true | Execution blocked | "EDR blocked execution" |
| 3 or 4 | false | Quarantine/remove attempted, failed | "EDR attempted quarantine (failed)" |

**WR-02 close vs. re-scope decision tree:**

| Scenario | Verdict |
|----------|---------|
| Pass 1: no Defender alert, no Sysmon anomaly; Pass 2: same, exclusion confirmed not needed for normal operation | WR-02 CLOSED — nono runs cleanly under a real EDR-proxy with no false positives |
| Pass 1: Defender alerts on binary (reputation), no behavioral alert; Pass 2: alert suppressed with exclusion, MIC still holds | WR-02 CLOSED with note: "reputation-driven alert suppressed by install-path exclusion; T1134.002 behavior not independently triggered" |
| Pass 1: Defender behavioral alert (ThreatName = something), ActionSuccess=false; Pass 2: exclusion suppresses, MIC still holds | WR-02 CLOSED with note: "behavioral alert present; excluded for production use; structural boundary intact" |
| Pass 1: Defender quarantines nono.exe or nono-shell-broker.exe (ActionSuccess=true, binary removed) | WR-02 RE-SCOPED: "quarantine on install path" — requires operator to restore binary and add exclusion before proceeding to Pass 2; note incident as F-66-XXX |
| Sysmon Event 1 does NOT show IntegrityLevel=Low on the cmd.exe child | WR-02 RE-SCOPED: "MIC boundary not exercised" — investigate whether BrokerLaunchNoPty arm was reached; check Windows Event 4688 for MandatoryLabel |
| TESTSIGNING=ON causes kernel integrity flags that confuse AV heuristics | Record TESTSIGNING posture in baseline; tag any alert that fires ONLY under TESTSIGNING as "TESTSIGNING confounder, not behavioral" |

---

### RQ6 — Host hygiene, baseline capture, TESTSIGNING posture

The VM has `TESTSIGNING ON` from the Phase 63 minifilter spike. The CONTEXT.md already documented two options (turn off or record the posture). The research recommendation:

**Option A (preferred):** `bcdedit /set testsigning off` + reboot before the EDR UAT. This eliminates the TESTSIGNING confounder entirely and gives cleaner Defender behavior.

**Option B (if reboot is impractical):** Record TESTSIGNING posture in the UAT baseline stamp. Tag any Defender alert that fires with a `[TESTSIGNING-confounder?]` note and re-verify with TESTSIGNING off if the alert seems anomalous.

**Host baseline capture commands (run once, before Pass 1):**
```powershell
# 1. Defender AV status
Get-MpComputerStatus | Select-Object AMProductVersion, AMEngineVersion, NISEngineVersion,
  AntispywareEnabled, AntivirusEnabled, RealTimeProtectionEnabled,
  IoavProtectionEnabled, BehaviorMonitorEnabled

# 2. Sysmon version and config
sysmon -c  # shows current config loaded (or check: Get-Service Sysmon)
(Get-Item "C:\Windows\Sysmon.exe" -ErrorAction SilentlyContinue)?.VersionInfo.ProductVersion

# 3. MSI signature (see RQ3 commands above)

# 4. TESTSIGNING posture
bcdedit /enum current | Select-String "testsigning"

# 5. No existing nono Defender exclusions
(Get-MpPreference).ExclusionPath | Where-Object { $_ -match 'nono' }

# 6. Audit policy: Process Creation
auditpol /get /subcategory:"Process Creation"

# 7. Defender threat history cleared (optional — prevents pollution from prior tests):
# CAUTION: only run if intentional; this removes all detection history
# Remove-Item -Path "C:\ProgramData\Microsoft\Windows Defender\Scans\History\Service\DetectionHistory" -Recurse -Force -ErrorAction SilentlyContinue
# Alternative: just record timestamps before and after each pass (the $before/$after pattern in RQ2)
```

---

## Standard Stack (Observation Tools)

### Core Observation Commands

| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| `Get-MpThreatDetection` | Defender PS module (built-in) | Primary Defender alert/quarantine query | Use before/after delta pattern |
| `Get-MpComputerStatus` | Defender PS module | Baseline: version, real-time, behavior monitor | Run once at session start |
| `Get-MpPreference` | Defender PS module | Verify exclusions | Check before Pass 1, after Pass 2 exclusion add |
| `Get-AuthenticodeSignature` | Microsoft.PowerShell.Security | Publisher-trust recording | Run on nono.exe and nono-shell-broker.exe |
| `Get-WinEvent -LogName "Microsoft-Windows-Sysmon/Operational"` | Built-in PS | Sysmon event corpus | Filter by event ID 1, 7, 8, 10 |
| `Get-WinEvent -LogName Security -Id 4688` | Built-in PS | Process creation with MandatoryLabel field | Requires Audit Process Creation enabled |
| `auditpol /get /subcategory:"Process Creation"` | Built-in CLI | Verify audit policy active | If disabled, 4688 won't fire; Sysmon Event 1 is the backup |
| `bcdedit /enum current` | Built-in CLI | TESTSIGNING posture | Record before every pass |
| `Add-MpPreference` / `Remove-MpPreference` | Defender PS module | Pass 2 exclusion add/revert | Requires admin; verify with Get-MpPreference after each |
| `sysmon -c` | Sysinternals Sysmon | Verify config is loaded | Confirm SwiftOnSecurity config active |

### Sysmon Event Reference for This Phase

| Event ID | Name | Fields Used | What It Proves |
|----------|------|-------------|----------------|
| 1 | ProcessCreate | Image, IntegrityLevel, ParentImage, CommandLine | Low-IL child created by broker (T1134.002 observable) |
| 7 | ImageLoad | Image, ImageLoaded, Signed | What DLLs loaded into the Low-IL child (EDR injection) |
| 8 | CreateRemoteThread | SourceImage, TargetImage | If anyone tries to inject into the child via CreateRemoteThread |
| 10 | ProcessAccess | SourceImage, TargetImage, GrantedAccess | Cross-process token access (may not fire for self-OpenProcess) |

**Sysmon Event 10 note on T1134.002:** The broker calls `OpenProcessToken(GetCurrentProcess(), ...)` — a self-access on its own process. Sysmon Event 10 is optimized for CROSS-process access. The SwiftOnSecurity config may filter self-access events. The most reliable T1134.002 signal in Sysmon is Event 1 (the integrity-downgraded child), not Event 10. [ASSUMED based on SwiftOnSecurity config's exclusion behavior for self-access; verify empirically.]

---

## Architecture Patterns

### Two-Pass Test Architecture

```
[Host Baseline Capture]
       |
       v
[Pass 1: No Exclusions]
  nono run --profile claude-code -- cmd /c whoami /groups
       |
       +---> [Sysmon Event 1: IntegrityLevel=Low?]       (EDR-02a MIC proof)
       +---> [Defender: Get-MpThreatDetection delta]      (EDR-02b alert/quarantine)
       +---> [Sysmon Event 7: DLLs in Low-IL child?]     (EDR injection observation)
       +---> [whoami /groups: "Low Mandatory Level"?]     (child stdout proof)
       |
       v
[Record Pass 1 Verdicts]
       |
       v
[Pass 2: Add Exclusions]
  Add-MpPreference (path + 2x process)
  Verify via Get-MpPreference
       |
       v
[Pass 2: With Exclusions]
  nono run --profile claude-code -- cmd /c whoami /groups
       |
       +---> [Defender: new threats?]                     (exclusion suppression check)
       +---> [Sysmon Event 1: IntegrityLevel=Low STILL?]  (MIC survives exclusion)
       +---> [whoami /groups: still Low?]                 (structural invariant)
       |
       v
[Pass 2 Cleanup: Remove-MpPreference]
[Verify exclusions removed]
       |
       v
[Sign-off + WR-02 close or re-scope]
```

### Recommended UAT Artifact Structure (to mirror 65-HUMAN-UAT.md)

```
66-HUMAN-UAT.md
├── ## Host Baseline Stamp            (pre-flight, recorded once)
│   ├── Defender AV version + mode
│   ├── Sysmon version + config
│   ├── TESTSIGNING posture
│   ├── MSI signature + publisher
│   └── Audit policy status
├── ## Pass 1 — No Exclusions
│   ├── Assertion 1: Exercising command succeeds (nono exits 0)
│   ├── Assertion 2: whoami /groups shows Low Mandatory Level
│   ├── Assertion 3: Sysmon Event 1 — Low-IL grandchild present
│   ├── Assertion 4: Sysmon Event 7 — DLL load inventory for Low-IL child
│   ├── Assertion 5: Defender alert/quarantine delta (Get-MpThreatDetection)
│   └── Assertion 6: Sysmon Event 8/10 — any injection or token access events?
├── ## Pass 2 — With Exclusions
│   ├── Assertion 7: Exclusions confirmed added (Get-MpPreference)
│   ├── Assertion 8: Exercising command still succeeds with exclusions
│   ├── Assertion 9: IntegrityLevel=Low STILL present despite AV exclusion
│   └── Assertion 10: Defender delta clean (no new threats after exclusion)
├── ## WR-02 Verdict
│   ├── Per-scenario close/re-scope decision (using decision matrix above)
│   └── EDR-proxy caveat recorded
└── ## Sign-off
    └── Resume-signal: "approved" with pasted outputs
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Alert vs. quarantine distinction | Custom log parser | `Get-MpThreatDetection \| Select ThreatStatusID, CleaningActionID, ActionSuccess` | Official WMI class; ThreatStatusID is the authoritative field |
| Sysmon event parsing | PowerShell regex on raw log text | `Get-WinEvent \| ToXml()` + XPath on EventData | Sysmon events are structured XML; ToXml() + named Data nodes is the correct pattern |
| Integrity level check | P/Invoke to GetTokenInformation | `whoami /groups` + Sysmon Event 1 IntegrityLevel field | Built-in; no code needed; both surfaces agree |
| Defender exclusion management | Registry edits | `Add-MpPreference` / `Remove-MpPreference` | Defender PS cmdlets are the official API; registry edits can be overwritten or not immediately effective |
| Publisher trust verification | Manual cert inspection | `Get-AuthenticodeSignature` | Returns structured Signature object with Status enum |

---

## Common Pitfalls

### Pitfall 1: Misreading MIC direction (EDR-02a)
**What goes wrong:** Asserter writes "EDR DLL-injection into Low-IL child was blocked" and marks PASS because they believe NO_WRITE_UP protects the child from EDR injection.
**Why it happens:** NO_WRITE_UP sounds like "nobody can write to this process," but MIC is asymmetric: LOW-to-HIGH writes are blocked; HIGH-to-LOW injection is explicitly permitted.
**How to avoid:** The assertion is "child has Low IntegrityLevel" (the containment proof), not "EDR cannot inject." Sysmon Event 7 records what DLLs loaded into the Low-IL child — EDR monitoring DLLs appearing there is NOT a failure.
**Warning signs:** Assertion wording says "injection blocked" — rewrite to "integrity label confirmed Low."

### Pitfall 2: Treating TESTSIGNING alerts as behavioral T1134.002 detections
**What goes wrong:** Defender fires on nono-shell-broker.exe in Pass 1; UAT records "T1134.002 detected"; actually it's a heuristic on test-signed binaries.
**Why it happens:** TESTSIGNING ON means the OS will load test-signed kernel drivers; Defender's behavior monitor treats the TESTSIGNING state as a risk indicator.
**How to avoid:** Record TESTSIGNING posture in baseline. If a Defender alert fires and TESTSIGNING is ON, add `[TESTSIGNING-confounder?]` tag and note in UAT. Preferred fix: turn TESTSIGNING off before the UAT.
**Warning signs:** Defender fires on the binary file itself (not a process behavior), Resources field shows the .exe path, ThreatName contains "Unknown" or "Trojan:Win32/Wacatac" style generic detection.

### Pitfall 3: Missing the before/after delta pattern for Defender
**What goes wrong:** Operator runs Get-MpThreatDetection after the test and sees old detections from prior sessions; concludes "Defender detected T1134.002."
**Why it happens:** Get-MpThreatDetection returns ALL historical detections, not just today's.
**How to avoid:** Always capture `$before = Get-MpThreatDetection` BEFORE the exercising command; compare with $after; only new entries (using Compare-Object on ThreatID) are relevant to this test run.
**Warning signs:** Detection dates in Get-MpThreatDetection predate the current session.

### Pitfall 4: Forgetting that Pass 1 must run BEFORE Pass 2 exclusions
**What goes wrong:** Operator adds exclusions first, then runs both passes — Pass 1 is actually "with exclusions."
**Why it happens:** Sequence error; the false-positive characterization purpose of Pass 1 is destroyed.
**How to avoid:** Verify Get-MpPreference shows no nono-related exclusions before starting. The CONTEXT.md explicitly locks "no-exclusion first."
**Warning signs:** Pass 1 shows zero Defender activity; would be suspicious without verifying no exclusions existed.

### Pitfall 5: Assuming `auditpol` is enabled for 4688 without checking
**What goes wrong:** Operator queries Security log for Event 4688; finds nothing; concludes "no process creation recorded."
**Why it happens:** Audit Process Creation is disabled by default on many Azure VMs.
**How to avoid:** Run `auditpol /get /subcategory:"Process Creation"` in the host baseline. If "No Auditing" → 4688 won't fire. Use Sysmon Event 1 as the primary observable; 4688 is a backup.
**Warning signs:** Security event log contains no 4688 events at all.

### Pitfall 6: SwiftOnSecurity config may suppress some events
**What goes wrong:** No Sysmon Event 10 fires for the broker's self-OpenProcess; asserter concludes "Sysmon can't see T1134.002."
**Why it happens:** SwiftOnSecurity config has exclusions; self-process access may be filtered.
**How to avoid:** Use Sysmon Event 1 (IntegrityLevel on the child) as the primary T1134.002 observable. If Event 10 is absent, note it as "Sysmon config suppressed self-OpenProcess event (expected)" rather than "event missing."
**Warning signs:** Zero Sysmon Event 10 entries despite nono running.

---

## Runtime State Inventory

*Not applicable — this is a HUMAN-UAT phase with no code changes and no rename/refactor operations.*

---

## Environment Availability

| Dependency | Required By | Available | Notes |
|------------|------------|-----------|-------|
| Azure VM `nono-fltmgr-vm` | All assertions | Per CONTEXT.md kickoff | Host validated: Win11 26200, EICAR quarantine proven |
| Production-signed v0.62.2 machine MSI | Broker trust gate (D-32-12) | Per CONTEXT.md | REQUIRED — dev-layout install bypasses broker Authenticode check and never exercises the Production path |
| Sysmon v15.20 (SwiftOnSecurity config) | Sysmon events (A1, A3, A4, A6) | Per CONTEXT.md | Events flowing; schema 4.91 |
| Defender AV 4.18.26050.15 Normal mode | Alert/quarantine assertions (A5, A10) | Per CONTEXT.md | EICAR quarantine proven with ActionSuccess=True |
| PowerShell (admin) | All Get-Mp* cmdlets, Get-WinEvent Security | Built-in Win11 | Requires elevation for Get-MpPreference writes and ExclusionPath changes |
| Audit Process Creation policy | 4688 events (backup observable) | Unknown — verify in baseline | If disabled, 4688 absent; Sysmon Event 1 is primary |

**Missing with no fallback:** None that block the UAT — Sysmon Event 1 is the primary MIC + T1134.002 observable and does not require audit policy changes.

---

## Validation Architecture

`workflow.nyquist_validation` is not explicitly set in `.planning/config.json` (or the file does not exist for this phase-level research) — treat as enabled.

However, this phase has **no automated test components**: it is a pure human-executed UAT checklist. There are no Rust tests to run, no `cargo test` commands, and no CI gates.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated? | Assertion |
|--------|----------|-----------|------------|-----------|
| EDR-01 | Two-pass UAT artifact with ~10 assertions, each recording EDR product+version+mode | Manual HUMAN-UAT | No — requires real EDR host | 66-HUMAN-UAT.md completion |
| EDR-01 | Distinguishes "did not alert" from "did not quarantine" per assertion | Manual | No | ThreatStatusID + ActionSuccess recorded per assertion |
| EDR-02(a) | MIC NO_WRITE_UP boundary verified on Low-IL child | Manual (whoami /groups + Sysmon Event 1) | No | Operator verifies output and pastes into checklist |
| EDR-02(b) | T1134.002 sequence triggers or does not trigger Defender alert/quarantine | Manual (Get-MpThreatDetection delta) | No | Operator runs before/after query and records |
| EDR-02 | WR-02 closed or explicitly re-scoped | Planning artifact | N/A | Sign-off block in 66-HUMAN-UAT.md |

### Wave 0 Gaps
None — no new automated test files are needed. The entire deliverable is the `66-HUMAN-UAT.md` checklist written by the planner and executed by the operator.

---

## Security Domain

`security_enforcement` is implied enabled (this is a security-critical codebase per CLAUDE.md).

### Applicable ASVS Categories for This Phase

| ASVS Category | Applies | Control |
|---------------|---------|---------|
| V2 Authentication | No | UAT only, no auth code changes |
| V3 Session Management | No | UAT only |
| V4 Access Control | Yes | The LOW-IL MIC boundary IS an access control verification |
| V5 Input Validation | No | No new code |
| V6 Cryptography | No | No new code |

### Security Considerations for the UAT Itself

- The exercising command `cmd /c whoami /groups` is read-only (no filesystem writes, no network). Pass 1 and Pass 2 both run only this command.
- Defender exclusions added in Pass 2 are explicitly cleaned up with Remove-MpPreference. If the cleanup is not run (e.g., operator session terminates), a stale exclusion remains on the VM. The UAT artifact should note this as a cleanup requirement.
- The UAT does NOT test Defender evasion — it tests whether nono's normal operation triggers false positives. If Defender quarantines nono.exe, that is a FINDING, not a success condition.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | SwiftOnSecurity Sysmon config may suppress self-OpenProcess access events (Event 10 may not fire for broker self-access) | RQ2 Sysmon events | Low — Sysmon Event 1 is the primary observable; Event 10 is a supplementary check |
| A2 | Defender AV is unlikely to generate a behavioral T1134.002 alert on a well-known sandboxing pattern (Medium-IL spawning Low-IL via CreateProcessAsUserW) | RQ2 T1134.002 detection | Medium — if Defender DOES alert, it changes the WR-02 verdict interpretation; the UAT itself resolves this empirically |
| A3 | CleaningActionID values (2=Quarantine, 3=Remove, 7=Block) come from community-documented mappings, not official Microsoft enumeration documentation | RQ5 | Low — ThreatStatusID is the authoritative field and IS officially documented; CleaningActionID is supplementary |
| A4 | TESTSIGNING ON state on the VM may influence Defender heuristics on unsigned/test-signed DLLs loaded into the broker process | RQ6 host hygiene | Medium — if TESTSIGNING-driven alerts fire and are mistaken for T1134.002 behavioral detections, the pass/fail verdict is wrong; prefer Option A (turn TESTSIGNING off before UAT) |

---

## Open Questions

1. **Is Audit Process Creation enabled on `nono-fltmgr-vm`?**
   - What we know: Azure VMs do not always have it enabled by default.
   - What's unclear: Whether the host was configured for it during Phase 63/64 work.
   - Recommendation: Include `auditpol /get /subcategory:"Process Creation"` in the host baseline capture (Assertion 0 / pre-flight). If not enabled, Sysmon Event 1 is the sole primary observable for the Low-IL child — that is sufficient.

2. **Does Defender AV behavioral engine alert on T1134.002 integrity-DOWN sequences?**
   - What we know: Defender alerts on integrity-UP manipulation (privilege escalation). Integrity-DOWN is the sandboxing direction used by Chrome, IE, Adobe Reader — Defender is historically trained NOT to alert on this pattern.
   - What's unclear: Whether the v0.62.2 binary's reputation state changes the heuristic threshold.
   - Recommendation: The UAT itself resolves this. The assertion is "record what happens" — both "no alert" and "alert" are valid findings with different WR-02 implications.

3. **Is TESTSIGNING still ON on `nono-fltmgr-vm` after Phase 65 completion?**
   - What we know: CONTEXT.md notes the minifilter test driver was unloaded post-latency-capture; TESTSIGNING state was not changed.
   - What's unclear: Whether the operator turned it off as part of Phase 65 cleanup.
   - Recommendation: Add `bcdedit /enum current | Select-String "testsigning"` to the pre-flight baseline capture; turn off if ON (Option A).

---

## Sources

### Primary (HIGH confidence)
- `learn.microsoft.com/en-us/windows/win32/secauthz/mandatory-integrity-control` — MIC directional rules, NO_WRITE_UP definition, process creation integrity rules
- `learn.microsoft.com/en-us/previous-versions/windows/desktop/defender/msft-mpthreatdetection` — ThreatStatusID enumeration (official WMI class; authoritative for alert vs. quarantine)
- `learn.microsoft.com/en-us/powershell/module/defender/add-mppreference` — ExclusionPath/ExclusionProcess syntax
- `learn.microsoft.com/en-us/powershell/module/defender/remove-mppreference` — exclusion revert syntax
- `learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-authenticodesignature` — publisher trust check
- `crates/nono/src/sandbox/windows.rs` — `create_low_integrity_primary_token` source (the exact T1134.002 sequence: OpenProcessToken + DuplicateTokenEx + SetTokenInformation + CreateProcessAsUserW)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — BrokerLaunchNoPty arm, cascade ordering, CreateProcessAsUserW call site
- `crates/nono-cli/data/policy.json` — `claude-code` profile `windows_low_il_broker: true`

### Secondary (MEDIUM confidence)
- `learn.microsoft.com/en-us/answers/questions/4110692/windows-defender-get-mpthreatdetection-cleaningact` — CleaningActionID community-documented mapping (2=Quarantine, 3=Remove, 7=Block)
- `attack.mitre.org/techniques/T1134/002/` — T1134.002 definition and detection strategies
- `blog.didierstevens.com/2010/09/07/integrity-levels-and-dll-injection/` — empirical confirmation that Low-IL process cannot inject into Medium-IL; Medium-IL CAN inject into Low-IL
- `github.com/trustedsec/SysmonCommunityGuide` — Sysmon Event 1 IntegrityLevel field, Event 7 ImageLoad, Event 8 CreateRemoteThread, Event 10 ProcessAccess semantics

### Tertiary (LOW confidence)
- Defender behavioral heuristic sensitivity to T1134.002 integrity-DOWN pattern: not documented; must be resolved empirically by the UAT

---

## Metadata

**Confidence breakdown:**
- MIC boundary direction: HIGH — official Microsoft docs
- ThreatStatusID (alert vs. quarantine): HIGH — official WMI class
- CleaningActionID mapping: MEDIUM — community-sourced; cross-referenced with ThreatStatusID
- Sysmon event corpus for this scenario: MEDIUM — Sysmon event schema is authoritative; SwiftOnSecurity config filtering is ASSUMED
- Defender behavioral alert on T1134.002: LOW — requires live test; no public signature database confirms or denies

**Research date:** 2026-06-11
**Valid until:** 2026-07-11 (Defender engine versions change; re-verify engine version in host baseline before UAT)
