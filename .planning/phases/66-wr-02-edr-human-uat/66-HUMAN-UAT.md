# Phase 66 — WR-02 EDR HUMAN-UAT (gate 66-WR02, EDR-01 + EDR-02)

**Status:** OPEN / BLOCKS PHASE CLOSE. This checklist must be executed on the real EDR
host `nono-fltmgr-vm` by a human operator. It is close-blocking: gate 66-WR02 stays OPEN
until all 10 assertions are run and the WR-02 Verdict section is filled in. **Do NOT
mark any item `pass` without real host output** — this is a live EDR observation; no
simulation substitutes.

**Host:** _\<hostname\>_  |  **Windows version / build:** _\<winver\>_  |  **Date:** _\<run date\>_

**Scope:** This phase has NO production code changes. The entire deliverable is this
checklist and the operator-recorded verdicts. The exercising command is
`nono run --profile claude-code -- cmd /c whoami /groups`, run from an elevated
PowerShell on `nono-fltmgr-vm` with the production-signed v0.62.2 machine MSI installed.

**EDR-proxy caveat:** This UAT uses Sysmon v15.20 (SwiftOnSecurity config, schema 4.91) +
Microsoft Defender Antivirus 4.18.26050.15 (Normal mode, real-time protection ON) as a
representative EDR proxy. This is NOT Microsoft Defender for Endpoint (cloud-behavioral
engine, XDR correlation). The results validate load-bearing OS boundaries (MIC NO_WRITE_UP
enforcement + Defender real-time quarantine), but not MDE-specific cloud behavioral
detections. WR-02 closes as "validated under a representative EDR-proxy". A future MDE
re-run of the same matrix is optional and does not block close.

> **Pre-requisite:** The production-signed v0.62.2 *machine* MSI must be installed. The
> broker self-trust gate (D-32-12) only spawns nono-shell-broker.exe from a signed
> Program-Files install. Dev-layout builds bypass this gate and do NOT exercise the
> EDR-02(b) T1134.002 path.

---

## Host Baseline Stamp (run once, BEFORE Pass 1)

Complete all seven items and paste results into each area before starting Pass 1. These
records establish the ground truth against which every assertion is interpreted.

### Baseline Item (a) — TESTSIGNING posture

```powershell
bcdedit /enum current | Select-String "testsigning"
```

**Action required:** If output shows `testsigning Yes`, run the following and reboot before
continuing the UAT:

```powershell
bcdedit /set testsigning off
# Then reboot: Restart-Computer
```

If TESTSIGNING is ON and you do NOT reboot, any Defender alerts must be tagged
`[TESTSIGNING-confounder?]` throughout the UAT (see RESEARCH RQ6 and Pitfall 2).

```
<paste bcdedit output here — confirm "testsigning" line value (Yes or absent/No)>
```

**TESTSIGNING posture at UAT run time:** _\<ON / OFF\>_

---

### Baseline Item (b) — Defender AV status

```powershell
Get-MpComputerStatus | Select-Object AMProductVersion, AMEngineVersion, AntivirusEnabled, RealTimeProtectionEnabled, BehaviorMonitorEnabled
```

```
<paste output here — confirm AntivirusEnabled=True, RealTimeProtectionEnabled=True, BehaviorMonitorEnabled=True>
```

**Defender AV version / policy mode at UAT run time:** _\<AMProductVersion / AMEngineVersion\>_

---

### Baseline Item (c) — Sysmon status

```powershell
Get-Service Sysmon | Select-Object Status, DisplayName
(Get-Item 'C:\Windows\Sysmon.exe' -ErrorAction SilentlyContinue)?.VersionInfo.ProductVersion
```

```
<paste output here — confirm Status=Running and version string>
```

**Sysmon version / config at UAT run time:** _\<version / SwiftOnSecurity config\>_

---

### Baseline Item (d) — MSI publisher trust (publisher-trust confounder recording)

This is the publisher-trust confounder recording per RESEARCH RQ3. If the Issuer does NOT
chain to a public CA (DigiCert, Sectigo, Microsoft, etc.), any Pass 1 Defender alert must
be tagged `[publisher-reputation-confounder?]` — it may be a reputation alert, not a
T1134.002 behavioral detection.

```powershell
Get-AuthenticodeSignature -FilePath 'C:\Program Files\nono\nono.exe' |
  Select-Object Status, StatusMessage,
    @{N='SignerSubject';E={$_.SignerCertificate.Subject}},
    @{N='Thumbprint';E={$_.SignerCertificate.Thumbprint}},
    @{N='Issuer';E={$_.SignerCertificate.Issuer}},
    @{N='NotAfter';E={$_.SignerCertificate.NotAfter}} | Format-List

Get-AuthenticodeSignature -FilePath 'C:\Program Files\nono\nono-shell-broker.exe' |
  Select-Object Status, StatusMessage,
    @{N='SignerSubject';E={$_.SignerCertificate.Subject}},
    @{N='Thumbprint';E={$_.SignerCertificate.Thumbprint}},
    @{N='Issuer';E={$_.SignerCertificate.Issuer}},
    @{N='NotAfter';E={$_.SignerCertificate.NotAfter}} | Format-List
```

```
<paste output for BOTH binaries here — record Status, SignerSubject, Issuer, Thumbprint>
```

**Publisher trust assessment:** _\<Issuer chains to public CA: YES / NO — if NO, tag Pass 1 alerts [publisher-reputation-confounder?]\>_

---

### Baseline Item (e) — Audit Process Creation policy

```powershell
auditpol /get /subcategory:"Process Creation"
```

If result is "No Auditing", Windows Security Event 4688 will not fire. Sysmon Event 1 is
the sole primary MIC observable in that case (that is sufficient — no action needed).

```
<paste auditpol output here — record Success/Failure or "No Auditing">
```

**Audit Process Creation status:** _\<Success and Failure / Success / No Auditing\>_

---

### Baseline Item (f) — Existing nono exclusions pre-check (MUST be empty before Pass 1)

```powershell
(Get-MpPreference).ExclusionPath | Where-Object { $_ -match 'nono' }
(Get-MpPreference).ExclusionProcess | Where-Object { $_ -match 'nono' }
```

**If either returns any entry: STOP.** Remove the existing nono-related exclusions with
`Remove-MpPreference` before proceeding to Pass 1. Pass 1 is only meaningful when no
exclusions exist — otherwise false-positive exposure cannot be characterized.

```
<paste output here — must be empty (no output) to proceed>
```

**Existing exclusions:** _\<None / [list any found]\>_

---

### Baseline Item (g) — Defender threat history baseline count

This captures the count of pre-existing Defender detections so the Pass 1 and Pass 2
before/after delta comparisons are clean. Do NOT clear the detection history.

```powershell
$baseline_count = (Get-MpThreatDetection).Count
Write-Host "Baseline detection count: $baseline_count"
```

```
<paste count only — e.g., "Baseline detection count: 3">
```

**Baseline detection count:** _\<N\>_

---

## Pass 1 — No Exclusions

**Pre-condition:** Baseline Item (f) confirmed no nono-related exclusions exist. If
TESTSIGNING is still ON, tag all Defender findings `[TESTSIGNING-confounder?]`.

Run assertions A-P1-01 through A-P1-06 in sequence. Use a single elevated PowerShell
session throughout Pass 1.

---

### A-P1-01 — Exercising command completes successfully (nono exits 0)

**What this validates:** The broker-path (BrokerLaunchNoPty arm, windows_low_il_broker:true)
reaches `CreateProcessAsUserW` and the child completes normally. This is the baseline
functional confirmation before EDR observations are interpreted.

**EDR product / version / policy mode:** _\<Defender AMProductVersion / Normal mode / RealTimeProtectionEnabled=True\>_

```powershell
nono run --profile claude-code -- cmd /c whoami /groups
# If nono is not on PATH:
& 'C:\Program Files\nono\nono.exe' run --profile claude-code -- cmd /c whoami /groups
```

**Pass criterion:** Command exits 0. No error output from nono itself. The `whoami /groups`
output (group listing with Mandatory Label line) appears before nono exits.

**Result:** [ ] pass  /  [ ] blocked

```
<paste the full nono + whoami /groups output here — include any nono banner lines and the complete group listing>
```

---

### A-P1-02 — Low-IL child confirmed via whoami /groups output (EDR-02(a) primary proof)

**What this validates:** The child token carries the Low mandatory integrity label
(`S-1-16-4096`). This is the primary structural proof of EDR-02(a): the child runs at
Low IL with `NO_WRITE_UP`, which means it cannot write back up to Medium-IL or higher
objects.

**IMPORTANT — MIC direction note:** "IntegrityLevel=Low on the child" means the child
CANNOT write up to Medium-IL or higher objects (`NO_WRITE_UP`). It does NOT mean the EDR
cannot inject monitoring DLLs downward into this process. Medium-IL injectors (EDR agents,
Sysmon) are MIC-permitted to write down into Low-IL processes. Defender/Sysmon monitoring
DLLs loading into the Low-IL child is expected behavior, NOT a failure — see A-P1-04 for
the DLL injection observation. The assertion here is "child has IntegrityLevel=Low", NOT
"EDR cannot inject".

**EDR product / version / policy mode:** _\<same as A-P1-01\>_

**Command:** Use the output already captured in A-P1-01. Paste it again here for clarity.

**Pass criterion:** Output from A-P1-01 contains both of the following:
- The text `Mandatory Label\Low Mandatory Level`
- The SID `S-1-16-4096`

**Result:** [ ] pass  /  [ ] blocked

```
<paste the whoami /groups output from A-P1-01 here — confirm "Mandatory Label\Low Mandatory Level" and SID S-1-16-4096 are present>
```

---

### A-P1-03 — Sysmon Event 1 — Low-IL grandchild confirmed (T1134.002 primary Sysmon signal)

**What this validates:** This is the primary T1134.002 Sysmon observable. Sysmon Event 1
records the cmd.exe grandchild process created by nono-shell-broker.exe with
IntegrityLevel=Low. The parent-child IntegrityLevel mismatch (broker at Medium IL, child
at Low IL) is the T1134.002 behavioral signature that Sysmon captures. This corroborates
A-P1-02 from the kernel telemetry layer.

**EDR product / version / policy mode:** _\<Sysmon v15.20 / SwiftOnSecurity config / schema 4.91\>_

```powershell
Get-WinEvent -LogName "Microsoft-Windows-Sysmon/Operational" |
  Where-Object { $_.Id -eq 1 } |
  Select-Object -Last 30 |
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
  } | Where-Object { $_.IntegrityLevel -eq 'Low' } | Format-Table -AutoSize
```

**Pass criterion:** At least one record appears where:
- `Image` ends in `\cmd.exe`
- `IntegrityLevel` = "Low"
- `CommandLine` contains "whoami /groups"
- `ParentImage` ends in `\nono-shell-broker.exe`

**Note on missing Event 10:** If Sysmon Event 10 (ProcessAccess on broker self-open) is
absent elsewhere, that is expected — the SwiftOnSecurity config may suppress
self-OpenProcess events. Note as "Sysmon config suppressed self-OpenProcess event
(expected per RESEARCH RQ2 Pitfall 6)" rather than a gap.

**Note:** Record the `ProcessId` field of the cmd.exe Low-IL entry — you will need it for A-P1-04.

**Result:** [ ] pass  /  [ ] blocked

```
<paste the Format-Table output here — include all columns; highlight the cmd.exe row with IntegrityLevel=Low and ParentImage=nono-shell-broker.exe>
```

**ProcessId of the Low-IL cmd.exe child:** _\<PID\>_

---

### A-P1-04 — Sysmon Event 7 — DLL load inventory for the Low-IL child (EDR injection observation)

**What this validates:** This assertion records what DLLs loaded into the Low-IL cmd.exe
child. This is an observational record, NOT a binary pass/fail on injection. EDR monitoring
DLLs loading down into the Low-IL child is MIC-legal (Medium-IL injector to Low-IL target
is permitted). Unsigned DLLs are notable and should be recorded.

**EDR product / version / policy mode:** _\<Sysmon v15.20 / SwiftOnSecurity config / schema 4.91\>_

Replace `<paste_child_pid_here>` with the ProcessId recorded in A-P1-03:

```powershell
$childPid = '<paste_child_pid_here>'
Get-WinEvent -LogName "Microsoft-Windows-Sysmon/Operational" |
  Where-Object { $_.Id -eq 7 } |
  Select-Object -Last 200 |
  ForEach-Object {
    $xml = [xml]$_.ToXml()
    $data = $xml.Event.EventData.Data
    $pid = ($data | Where-Object Name -eq 'ProcessId').'#text'
    if ($pid -eq $childPid) {
      [PSCustomObject]@{
        TimeCreated = $_.TimeCreated
        Image       = ($data | Where-Object Name -eq 'Image').'#text'
        ImageLoaded = ($data | Where-Object Name -eq 'ImageLoaded').'#text'
        Signed      = ($data | Where-Object Name -eq 'Signed').'#text'
      }
    }
  } | Where-Object { $_ -ne $null } | Format-Table -AutoSize
```

**Record field:** List all ImageLoaded entries. Note any where `Signed = "false"` — these
are unsigned DLLs and are notable. EDR monitoring DLLs (e.g., MpClient.dll,
SysmonDrv callbacks) appearing in this list is expected and NOT a failure.

**Pass criterion:** Command runs without error. Paste the full DLL list, even if long.

**Result:** [ ] pass  /  [ ] blocked

```
<paste the full DLL load list here — include all columns; note any Signed=false entries>
```

**Notable unsigned DLLs (Signed=false):** _\<list or "None"\>_

---

### A-P1-05 — Defender alert/quarantine delta — T1134.002 Defender response (EDR-02(b))

**What this validates:** The before/after Defender threat detection comparison answers the
open LOW-confidence question from RESEARCH RQ2: does Defender AV alert or quarantine on
the broker's integrity-DOWN sandboxing sequence (T1134.002)? This assertion records the
authoritative ThreatStatusID + ActionSuccess fields per the alert-vs-quarantine decision
matrix. "No new threats detected" is a VALID and EXPECTED outcome (Defender is historically
trained not to alert on well-known sandboxing patterns like Chrome, IE Protected Mode,
Adobe Reader).

**EDR product / version / policy mode:** _\<Defender AMProductVersion / AMEngineVersion / Normal mode\>_

```powershell
$before = Get-MpThreatDetection | Select-Object ThreatID, InitialDetectionTime
# (If you have not run nono yet since the baseline stamp, re-run it now:)
nono run --profile claude-code -- cmd /c whoami /groups
Start-Sleep -Seconds 5  # allow Defender scan to complete
$after = Get-MpThreatDetection | Select-Object ThreatID, InitialDetectionTime,
  ThreatStatusID, CleaningActionID, ActionSuccess, Resources
$new = Compare-Object -ReferenceObject $before -DifferenceObject $after -Property ThreatID |
  Where-Object { $_.SideIndicator -eq '=>' }
if ($new) {
  Write-Host "NEW THREATS DETECTED:"
  $after | Where-Object { $_.ThreatID -in $new.ThreatID } |
    Select-Object ThreatStatusID, CleaningActionID, ActionSuccess, Resources |
    Format-List
} else {
  Write-Host "No new Defender threats detected (expected for sandboxing pattern)"
}
```

**Interpret result using this matrix (record ThreatStatusID + ActionSuccess):**

| ThreatStatusID | ActionSuccess | Classification | Verdict |
|----------------|---------------|----------------|---------|
| Absent / no new entries | — | No detection | "EDR did not alert" (expected) |
| 1 | false | Alert logged, no action | "EDR alerted (no quarantine)" |
| 3 | true | File quarantined | "EDR quarantined binary" — STOP; see WR-02 re-scope in decision table; restore binary before Pass 2 |
| 6 | true | Execution blocked | "EDR blocked execution" |
| 3 or 4 | false | Action attempted, failed | "EDR attempted quarantine (failed)" |

**Confounder tagging:**
- If a Defender alert fires AND the Host Baseline Stamp (item d) shows the Issuer does NOT chain to a public CA: tag the alert `[publisher-reputation-confounder?]`
- If TESTSIGNING was ON during this run: tag any alert `[TESTSIGNING-confounder?]`

**Pass criterion:** Record whatever happened. Paste the full output. Pass/blocked for this
assertion is determined by the WR-02 decision table (any outcome can be recorded here —
the verdict table in the WR-02 section maps it to Close/Re-scope).

**Result:** [ ] pass  /  [ ] blocked

```
<paste the full Compare-Object output here — include the "No new Defender threats detected" line OR the full NEW THREATS DETECTED block with ThreatStatusID, CleaningActionID, ActionSuccess, Resources>
```

**ThreatStatusID recorded:** _\<value or "No new threats"\>_
**ActionSuccess recorded:** _\<value or "N/A"\>_
**Confounder tags applied:** _\<[publisher-reputation-confounder?] / [TESTSIGNING-confounder?] / None\>_

---

### A-P1-06 — Sysmon Event 8/10 — CreateRemoteThread / ProcessAccess supplementary scan

**What this validates:** Supplementary scan for cross-process injection events that fired
during the nono run. The absence of Event 10 for the broker's self-OpenProcess is expected
(see RESEARCH RQ2 Pitfall 6 / SwiftOnSecurity config filtering).

**EDR product / version / policy mode:** _\<Sysmon v15.20 / SwiftOnSecurity config / schema 4.91\>_

```powershell
# Event 8: CreateRemoteThread (injection via thread creation)
Get-WinEvent -LogName "Microsoft-Windows-Sysmon/Operational" -ErrorAction SilentlyContinue |
  Where-Object { $_.Id -eq 8 } | Select-Object -Last 10 |
  ForEach-Object {
    $xml = [xml]$_.ToXml(); $data = $xml.Event.EventData.Data
    [PSCustomObject]@{
      Time   = $_.TimeCreated
      Source = ($data | Where-Object Name -eq 'SourceImage').'#text'
      Target = ($data | Where-Object Name -eq 'TargetImage').'#text'
    }
  } | Format-Table -AutoSize

# Event 10: ProcessAccess (cross-process handle open — broker self-access may not appear)
Get-WinEvent -LogName "Microsoft-Windows-Sysmon/Operational" -ErrorAction SilentlyContinue |
  Where-Object { $_.Id -eq 10 } | Select-Object -Last 10 |
  ForEach-Object {
    $xml = [xml]$_.ToXml(); $data = $xml.Event.EventData.Data
    [PSCustomObject]@{
      Time          = $_.TimeCreated
      Source        = ($data | Where-Object Name -eq 'SourceImage').'#text'
      Target        = ($data | Where-Object Name -eq 'TargetImage').'#text'
      GrantedAccess = ($data | Where-Object Name -eq 'GrantedAccess').'#text'
    }
  } | Format-Table -AutoSize
```

**Record all entries.** Note whether `nono-shell-broker.exe` appears in `SourceImage` of
Event 10. If absent: record as "Sysmon config suppressed self-OpenProcess (expected)."

**Pass criterion:** Command runs without error. Paste all entries (or "No entries" if none).

**Result:** [ ] pass  /  [ ] blocked

```
<paste Event 8 output here — all columns>
```

```
<paste Event 10 output here — all columns; note if nono-shell-broker.exe appears in SourceImage>
```

**nono-shell-broker.exe in Event 10 SourceImage:** _\<Yes / No — if No, record "Sysmon config suppressed self-OpenProcess (expected)"\>_

---

## Pass 2 — With Exclusions

**Pre-condition:** Pass 1 is fully recorded. If A-P1-05 resulted in ThreatStatusID=3
(quarantine, ActionSuccess=true): restore the quarantined binary from the MSI or backup
before proceeding. Record this as a finding in the WR-02 Verdict section.

Run assertions A-P2-07 through A-P2-10 in sequence in the same elevated PowerShell session.

---

### A-P2-07 — Exclusions added and verified

**What this validates:** Pass 2 exclusions are correctly configured and in effect before
the exercising command runs.

**EDR product / version / policy mode:** _\<Defender AMProductVersion / Normal mode\>_

```powershell
Add-MpPreference -ExclusionPath "C:\Program Files\nono"
Add-MpPreference -ExclusionProcess "C:\Program Files\nono\nono.exe"
Add-MpPreference -ExclusionProcess "C:\Program Files\nono\nono-shell-broker.exe"
# Verify:
(Get-MpPreference).ExclusionPath
(Get-MpPreference).ExclusionProcess
```

**Pass criterion:** `ExclusionPath` output includes `C:\Program Files\nono` and
`ExclusionProcess` output includes both `C:\Program Files\nono\nono.exe` and
`C:\Program Files\nono\nono-shell-broker.exe`.

**Result:** [ ] pass  /  [ ] blocked

```
<paste the ExclusionPath and ExclusionProcess output here — confirm both entries are present>
```

---

### A-P2-08 — Exercising command still succeeds with exclusions applied

**What this validates:** nono functions normally when excluded from AV scanning. Confirms
that Pass 2 can proceed with reliable binary execution (not scanner-induced latency or
block).

**EDR product / version / policy mode:** _\<Defender AMProductVersion / Normal mode / exclusions active\>_

```powershell
nono run --profile claude-code -- cmd /c whoami /groups
```

**Pass criterion:** Command exits 0. `whoami /groups` output printed. The Mandatory Label
line should still be present (A-P2-09 will re-assert this formally).

**Result:** [ ] pass  /  [ ] blocked

```
<paste the full nono + whoami /groups output here>
```

---

### A-P2-09 — SECURITY INVARIANT — IntegrityLevel=Low still present despite AV exclusion

**What this validates:** This is the critical security assertion. AV exclusions are
AV-scoping only. The Windows kernel's MIC enforcement is orthogonal to Defender — mandatory
labels on process tokens are set by `SetTokenInformation(TokenIntegrityLevel)` at process
creation; no AV exclusion can alter them. The Low-IL boundary MUST survive the exclusion.

**If this assertion fails** (IntegrityLevel absent or "Medium" with exclusions applied):
this is a CRITICAL FINDING — the MIC boundary is not what we believe it to be; WR-02 must
be re-scoped immediately.

**EDR product / version / policy mode:** _\<Sysmon v15.20 / SwiftOnSecurity config / with Defender exclusions active\>_

```powershell
# After A-P2-08, re-run the Sysmon Event 1 query for the latest cmd.exe Low-IL record:
Get-WinEvent -LogName "Microsoft-Windows-Sysmon/Operational" |
  Where-Object { $_.Id -eq 1 } |
  Select-Object -Last 30 |
  ForEach-Object {
    $xml = [xml]$_.ToXml()
    $data = $xml.Event.EventData.Data
    [PSCustomObject]@{
      TimeCreated    = $_.TimeCreated
      Image          = ($data | Where-Object Name -eq 'Image').'#text'
      IntegrityLevel = ($data | Where-Object Name -eq 'IntegrityLevel').'#text'
      ParentImage    = ($data | Where-Object Name -eq 'ParentImage').'#text'
    }
  } | Where-Object { $_.IntegrityLevel -eq 'Low' -and $_.Image -match 'cmd\.exe' } |
  Sort-Object TimeCreated -Descending | Select-Object -First 1 | Format-Table -AutoSize
```

**Pass criterion:** A record with `IntegrityLevel = "Low"` appears from the A-P2-08 run
(verify `TimeCreated` is after A-P2-08 started). Confirms: AV exclusions are AV-scoping
only; no AV exclusion can alter the kernel MIC mandatory label set by
`SetTokenInformation`.

**Result:** [ ] pass  /  [ ] blocked

```
<paste the single most-recent cmd.exe Low-IL record here — include TimeCreated, Image, IntegrityLevel, ParentImage; confirm TimeCreated is after A-P2-08 start time>
```

**IntegrityLevel with exclusions active:** _\<Low (PASS) / absent or Medium (CRITICAL FINDING)\>_

---

### A-P2-10 — Defender delta clean after exclusion — no new detections

**What this validates:** The exclusion suppresses Defender scan activity on the nono
install path. If Defender still alerts with the exclusion applied, that is anomalous and
warrants a re-scope note.

**EDR product / version / policy mode:** _\<Defender AMProductVersion / Normal mode / exclusions active\>_

```powershell
$before2 = Get-MpThreatDetection | Select-Object ThreatID, InitialDetectionTime
nono run --profile claude-code -- cmd /c whoami /groups
Start-Sleep -Seconds 5
$after2 = Get-MpThreatDetection | Select-Object ThreatID, InitialDetectionTime
$new2 = Compare-Object -ReferenceObject $before2 -DifferenceObject $after2 -Property ThreatID |
  Where-Object { $_.SideIndicator -eq '=>' }
if ($new2) {
  Write-Host "NEW THREATS DETECTED (unexpected with exclusion): $($new2.ThreatID -join ', ')"
} else {
  Write-Host "No new Defender threats detected (expected — exclusion applied)"
}
```

**Pass criterion:** No new detections. If Defender still alerts with the exclusion applied,
note the ThreatName and ThreatStatusID — this is anomalous and warrants a re-scope note in
the WR-02 Verdict section.

**Result:** [ ] pass  /  [ ] blocked

```
<paste the Compare-Object output here — "No new Defender threats detected" is the expected result>
```

---

## Pass 2 Cleanup (REQUIRED — do not skip)

Run the following cleanup commands **before ending the session**. If the session terminates
before cleanup runs, a stale exclusion remains on the VM — it MUST be removed before the
next EDR test session.

```powershell
Remove-MpPreference -ExclusionPath "C:\Program Files\nono"
Remove-MpPreference -ExclusionProcess "C:\Program Files\nono\nono.exe"
Remove-MpPreference -ExclusionProcess "C:\Program Files\nono\nono-shell-broker.exe"
# Verify clean:
(Get-MpPreference).ExclusionPath | Where-Object { $_ -match 'nono' }    # must be empty
(Get-MpPreference).ExclusionProcess | Where-Object { $_ -match 'nono' } # must be empty
```

**Cleanup result:** [ ] cleanup complete — both filters returned empty

```
<paste the ExclusionPath and ExclusionProcess filter output here — must produce no output (empty)>
```

---

## WR-02 Verdict

Match your observed outcomes to the decision table below. Record the selected scenario and
verdict in the Sign-off block.

**EDR-proxy caveat:** This UAT used Sysmon v15.20 (SwiftOnSecurity config, schema 4.91) +
Microsoft Defender Antivirus 4.18.26050.15 (Normal mode, real-time protection ON) as a
representative EDR proxy. This is NOT Microsoft Defender for Endpoint (cloud-behavioral
engine, XDR correlation). The results validate load-bearing OS boundaries (MIC NO_WRITE_UP
enforcement + Defender real-time quarantine), but not MDE-specific cloud behavioral
detections. WR-02 closes as "validated under a representative EDR-proxy". A future MDE
re-run of the same matrix is optional and does not block close.

| Scenario | Pass 1 Defender (A-P1-05) | Pass 2 MIC invariant (A-P2-09) | WR-02 Verdict | Next Step |
|----------|---------------------------|--------------------------------|---------------|-----------|
| No Defender alert in either pass; MIC holds | No new threats | IntegrityLevel=Low confirmed | CLOSED — nono runs cleanly under a representative EDR-proxy with no false positives | Tag WR-02 closed in REQUIREMENTS.md + sign-off below |
| Pass 1: reputation alert (ThreatStatusID=1), no behavioral; Pass 2: alert suppressed by exclusion; MIC holds | ThreatStatusID=1 [publisher-reputation-confounder?] | IntegrityLevel=Low confirmed | CLOSED with note: reputation-driven alert suppressed by install-path exclusion; T1134.002 behavior not independently triggered | Record [publisher-reputation-confounder?] in verdict; tag WR-02 closed |
| Pass 1: behavioral alert (ThreatStatusID=1); Pass 2: exclusion suppresses; MIC holds | ThreatStatusID=1 (behavioral) | IntegrityLevel=Low confirmed | CLOSED with note: behavioral alert present; excluded for production use; structural MIC boundary intact | Record alert ThreatName; recommend exclusion guidance in docs; tag WR-02 closed |
| Pass 1: Defender quarantines nono.exe or broker (ThreatStatusID=3, ActionSuccess=true) | ThreatStatusID=3, ActionSuccess=true | N/A — cannot run Pass 2 until binary restored | RE-SCOPED: F-66-QUARANTINE — restore binary from backup; add exclusion; re-run Pass 1 with exclusion; schedule re-run without exclusion on clean reputation baseline | File F-66-QUARANTINE; do not close WR-02 |
| Sysmon Event 1 does NOT show IntegrityLevel=Low on cmd.exe child | N/A | N/A | RE-SCOPED: F-66-MIC-NOT-EXERCISED — BrokerLaunchNoPty arm may not have been reached; check Windows Event 4688 MandatoryLabel; verify machine MSI install (not dev-layout) | File F-66-MIC-NOT-EXERCISED; investigate broker dispatch |
| TESTSIGNING ON and a Defender alert fires that looks heuristic | Alert present [TESTSIGNING-confounder?] | — | INCONCLUSIVE: turn TESTSIGNING off + reboot; re-run Pass 1 to determine if alert was a confounder; do not close WR-02 until clean run | Record confounder tag; schedule re-run with TESTSIGNING off |

**Selected scenario:** _\<paste the Scenario text from the matching row above\>_

**Observed Pass 1 Defender result (A-P1-05):** _\<ThreatStatusID value or "No new threats"\>_

**Observed Pass 2 MIC invariant result (A-P2-09):** _\<IntegrityLevel=Low confirmed / CRITICAL FINDING\>_

**WR-02 Verdict:** _\<CLOSED / RE-SCOPED as F-66-XXX / INCONCLUSIVE\>_

---

## Sign-off

- **Gate 66 (WR-02 EDR UAT verdict):** _PASS / FAIL / INCONCLUSIVE_
- **Host / Windows version / build / date:** _\<stamp\>_
- **Defender AV version / mode at run time:** _\<AMProductVersion / AMEngineVersion / Normal mode\>_
- **Sysmon version / config at run time:** _\<v15.20 / SwiftOnSecurity config schema 4.91\>_
- **TESTSIGNING posture at run time:** _\<ON / OFF\>_
- **WR-02 disposition:** _CLOSED / RE-SCOPED (scenario: F-66-XXX)_
- **Confounder tags applied:** _\<[TESTSIGNING-confounder?] / [publisher-reputation-confounder?] / None\>_
- **Commit hash of this file after verdicts pasted:** _\<git short hash\>_

> **This checklist BLOCKS phase close.** It ships OPEN (no operator results at authoring
> time, 2026-06-11). Resume-signal (plan 66-01 Task 2): type one of the following alongside
> the filled-in sign-off block above:
>
> - **"approved: WR-02 CLOSED"** — all 10 assertions ran, the WR-02 decision table maps to
>   a CLOSED scenario, 66-HUMAN-UAT.md committed with pasted verdicts
> - **"approved: WR-02 RE-SCOPED as F-66-XXX"** — a WR-02 re-scope scenario triggered
>   (quarantine, MIC not exercised, or TESTSIGNING inconclusive); 66-HUMAN-UAT.md records
>   the finding; describe the next step
> - **"blocked: \<reason\>"** — UAT could not be run (e.g., VM unavailable, binary missing
>   from Program Files, TESTSIGNING not cleared in time); WR-02 remains open; describe the
>   blocker
