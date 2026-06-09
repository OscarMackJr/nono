# Phase 64 Track A — VM Driver Test-Signing + Deny Harness Runbook

**Audience:** a junior dev/ops person with no prior nono context.
**Goal:** test-sign the `nono-fltmgr` minifilter on the Azure test VM, load it into the Windows
kernel, prove it denies a targeted file open, and capture the evidence that closes the Phase 64
Track A checkpoint.
**Time:** ~1.5–3 hours (most of it waiting on VM boots and a build).
**Danger level:** you are loading a kernel driver. A bad driver can **blue-screen (BSOD)** the VM.
That is expected and safe here — this VM is disposable and you take a snapshot first. **Never run
any of this on your own laptop or a production machine.**

> **How to use this doc:** run the steps top to bottom. Each command block is copy-paste ready.
> After each step there's an **Expected** line — if you don't see that, jump to the matching entry
> in §11 Troubleshooting. The **⚠ Phase 63 lesson** callouts are real foot-guns a previous run hit;
> read them, they save hours.

---

## 0. What you're actually doing (the 60-second mental model)

The driver (`nono-fltmgr.sys`) is a **minifilter**: Windows lets it sit in the file-system stack and
inspect every file-open. When something opens our target file, the driver pauses the open, asks a
small **user-mode program** (`nono_fltmgr_client.exe`, written in Rust) "allow or deny?", and the
program answers "deny" for our one secret file. The kernel then returns **Access Denied (error 5)**
to whoever tried to open it. Proving that round-trip works end-to-end is the whole job.

Pieces you'll touch:
| Piece | What it is | Where it comes from |
|-------|-----------|---------------------|
| `nono-fltmgr.sys` | the kernel driver (you build this on the VM) | `drivers/nono-fltmgr/` in the repo |
| `nono-fltmgr.inf` | install manifest; carries the driver's **altitude** number | repo — you'll edit one line |
| `nono_fltmgr_client.exe` | Rust user-mode "allow/deny" decider | build on the dev host, copy to VM |
| deny harness | a tiny PowerShell script that tries to open the secret file | §9 below |

---

## 1. Phase 63 gotchas — read this before you start

These already bit the last person. Don't relearn them the hard way.

1. **RDP (port 3389) is blocked by corporate egress.** You usually **cannot** Remote-Desktop
   straight to the VM. Use **Azure Bastion** (browser RDP over 443) for a desktop, or
   **`az vm run-command`** to run commands headless from your dev host. Both are shown below.
2. **Trusted Launch will silently ruin your day.** The VM **must** be `--security-type Standard`
   with Secure Boot OFF. Trusted Launch enforces Secure Boot, and then `bcdedit /set testsigning on`
   fails with *"The value is protected by Secure Boot policy."* The existing VM is already Standard —
   just don't reprovision it as Trusted Launch.
3. **`--security-type Standard` needs a one-time subscription feature flag**
   (`Microsoft.Compute/UseStandardSecurityType`). It's already registered on this subscription. You
   only care if you have to create a brand-new VM in a fresh subscription.
4. **DSv5/DASv5 had zero quota in eastus.** The VM is a `Standard_D4s_v4`. If you reprovision and the
   size is rejected, fall back to `D4s_v4` — don't fight the quota.
5. **VM create can fail once with `OSProvisioningTimedOut`** — it's transient. Delete and retry once.
6. **The build toolchain is the EWDK ISO (26H1), mounted on the VM** — not an installed Visual Studio.
   You open a build shell with `LaunchBuildEnv.cmd`. ⚠ That script **hangs if launched through
   `az vm run-command`** (it's interactive). Run the build from a **Bastion desktop**, not run-command.
7. **The `.sys` file is throwaible — never commit it.** Only `nono-fltmgr.c`, `.h`, and `.inf` live in
   the repo. The compiled `.sys`/`.cat`/`.obj` stay on the VM.
8. **The newer EWDK `signtool` rejects the WDK's auto test-sign** and then *deletes your unsigned
   `.sys`*. In Phase 63 the build was set to `SignMode=Off` to avoid this. In **this** runbook you
   sign manually (§6) with your own test cert, which is the supported path.

---

## 2. Confirm the VM is alive (and snapshot it first)

You run these from **your dev host** (where `az` is logged in). The VM already exists from Phase 63.

**Known resource names (from the Phase 63 evidence):**
| Thing | Value |
|------|------|
| Resource group | `rg-nono-fltmgr-spike` |
| VM name | `nono-fltmgr-vm` |
| Public IP | `20.51.161.15` |
| Safety snapshot to restore from | `nono-fltmgr-snap-testsigning-ready` |
| Subscription | `TWG Architecture POCs` (`98c2b71b-539f-4801-9c37-229efa10beda`) |

```powershell
# 2a. Make sure you're on the right subscription
az account show --query "name" -o tsv
# If it's not "TWG Architecture POCs":
#   az login
#   az account set --subscription "98c2b71b-539f-4801-9c37-229efa10beda"

# 2b. Is the VM there and running?
az vm get-instance-view -g rg-nono-fltmgr-spike -n nono-fltmgr-vm `
  --query "instanceView.statuses[?starts_with(code,'PowerState')].displayName" -o tsv
```
**Expected:** `VM running`. If it says `VM deallocated`, start it: `az vm start -g rg-nono-fltmgr-spike -n nono-fltmgr-vm`.
If the VM doesn't exist at all, jump to §10 (Reprovision from scratch).

```powershell
# 2c. CONFIRM the rollback snapshot exists. This is your undo button if the driver BSODs.
az snapshot show -g rg-nono-fltmgr-spike -n nono-fltmgr-snap-testsigning-ready --query "name" -o tsv
```
**Expected:** `nono-fltmgr-snap-testsigning-ready`.
**If it's missing — stop and make one now** (don't load a driver without an undo button):
```powershell
$diskName = az vm show -g rg-nono-fltmgr-spike -n nono-fltmgr-vm --query "storageProfile.osDisk.name" -o tsv
az snapshot create -g rg-nono-fltmgr-spike -n nono-fltmgr-snap-testsigning-ready --source $diskName
```

> **⚠ Phase 63 lesson (D-06):** the snapshot is the BSOD safeguard. If anything in §6–§9 blue-screens
> the VM and it won't boot, you restore from this snapshot (§11) rather than rebuilding everything.

---

## 3. Get onto the VM

You need an interactive desktop on the VM for the build and driver load. **Use Azure Bastion**
(works over 443, survives the corporate RDP block).

```powershell
# Open Bastion to the VM (browser-based RDP). In the Azure Portal:
#   VM 'nono-fltmgr-vm'  ->  Connect  ->  Bastion  ->  enter admin user 'nono-dev' + password.
# (CLI tunnel alternative, if Bastion host is provisioned:)
#   az network bastion rdp --name <bastion-name> -g rg-nono-fltmgr-spike `
#     --target-resource-id $(az vm show -g rg-nono-fltmgr-spike -n nono-fltmgr-vm --query id -o tsv)
```
**Expected:** a Windows 11 desktop in your browser.

**Headless alternative (for quick one-off checks only, NOT the build):** run a single command on the
VM without a desktop:
```powershell
az vm run-command invoke -g rg-nono-fltmgr-spike -n nono-fltmgr-vm `
  --command-id RunPowerShellScript --scripts "bcdedit /enum | Select-String testsigning"
```
> **⚠ Phase 63 lesson:** `az vm run-command` is great for one-liners but **wedges on interactive
> programs** (like `LaunchBuildEnv.cmd`). Do the build and driver-load from the **Bastion desktop**.

**Sanity check on the VM (Bastion desktop, elevated PowerShell — "Run as administrator"):**
```powershell
bcdedit /enum | Select-String testsigning      # expect: testsigning   Yes
Get-CimInstance Win32_DeviceGuard -Namespace root\Microsoft\Windows\DeviceGuard `
  | Select-Object VirtualizationBasedSecurityStatus   # expect: 0 (HVCI off)
```
**Expected:** `testsigning   Yes` and `VirtualizationBasedSecurityStatus : 0`. If `testsigning` is
not `Yes`, run `bcdedit /set testsigning on` then `shutdown /r /t 0` and reconnect.

---

## 4. Build the driver `.sys` on the VM

Copy the repo's `drivers/nono-fltmgr/` folder to the VM (drag-drop through the Bastion clipboard, or
`scp` if OpenSSH is up). Say you put it at `C:\nono-fltmgr`.

⚠ **Make sure you copied the latest version** — Phase 64 Plan 64-02 rewrote `nono-fltmgr.c` (the
pre-create callback, ring buffer, worker thread, and the `\NonoPolicyPort`). An old Phase 63 copy is
just an empty skeleton and will not deny anything.

In the **Bastion desktop**, open the EWDK build shell and build:
```cmd
:: Mount the EWDK ISO if it isn't already (drive letter may differ):
::   powershell Mount-DiskImage -ImagePath C:\path\to\ewdk.iso
:: Open the build environment (run from the EWDK mount, e.g. D:):
D:\LaunchBuildEnv.cmd

cd C:\nono-fltmgr
msbuild nono-fltmgr.vcxproj /p:Configuration=Release /p:Platform=x64
dir x64\Release\nono-fltmgr.sys
```
**Expected:**
```
Build succeeded.
    0 Warning(s)
    0 Error(s)
```
and a `nono-fltmgr.sys` (~5 KB) in `x64\Release\`.

> **⚠ Phase 63 lesson:** if you build through `az vm run-command` instead of Bastion, the build env
> launcher hangs forever. If `msbuild` errors on INF stamping (`stampinf exit 87`) or test-signing,
> you have an old `.vcxproj`; re-copy the current repo copy (those were fixed in Phase 63).

---

## 5. Pick a non-colliding altitude and update the INF

An **altitude** is the driver's fixed position in the filter stack. Two filters can't share one, and
you must avoid the antivirus band or you'll collide with a real EDR.

On the **VM**, elevated:
```powershell
fltmc filters
```
This lists every loaded filter and its altitude. Pick a number in the **FSFilter Activity Monitor
band 360000–389999** that **nothing else uses**.
- **Avoid** the anti-virus band **320000–329998**.
- **Avoid** known occupants like **385201 (Sysmon)** — and anything you actually see in the list.
- Example: if the 360000–389999 range looks empty in the listing, `365500` is a fine choice.

**Write down the number you chose.** Then edit the INF (line 71) to use it.

On your **dev host**, in the repo, change `nono-fltmgr.inf` line 71:
```
Instance1.Altitude  = "370020"      ->      Instance1.Altitude  = "<YOUR_CHOSEN_NUMBER>"
```
(`370020` is a placeholder; it must be replaced.) Then copy the updated INF to the VM (the driver
install in §6 reads it), **and commit the change back to the repo:**
```bash
git add drivers/nono-fltmgr/nono-fltmgr.inf
git commit -m "chore(64): set nono-fltmgr INF altitude to <YOUR_CHOSEN_NUMBER> (D-08)

Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>"
```

> **⚠ Phase 63 lesson (D-08):** the official Microsoft altitude (requested via `fsfcomm@microsoft.com`)
> can take ~30 business days. For this spike you self-pick from the Activity Monitor band — that's
> expected and fine; the official number replaces it later.

---

## 6. Test-sign and install the driver (the pipeline)

All on the **VM**, in an **elevated EWDK command prompt**, from `C:\nono-fltmgr`. Run these in order.

```cmd
:: a. Create a throwaway self-signed test certificate
makecert -r -pe -ss PrivateCertStore -n "CN=NonoTestSign" NonoTestSign.cer

:: b. Trust that cert as a root on THIS VM only
certmgr /add NonoTestSign.cer /s /r localMachine root

:: c. Generate the catalog (.cat) from the INF (declares what the package contains)
inf2cat /driver:. /os:10_x64 /uselocaltime

:: d. Sign the catalog with the test cert (SHA-256 + timestamp)
signtool sign /fd sha256 /s PrivateCertStore /n "CN=NonoTestSign" ^
  /t http://timestamp.digicert.com nono-fltmgr.cat

:: e. Confirm test-signing is still ON (it was set in Phase 63)
bcdedit /enum | findstr testsigning

:: f. Install + load the driver
pnputil /add-driver nono-fltmgr.inf /install
```
**Expected, step by step:**
- (a) `Succeeded` and a `NonoTestSign.cer` file.
- (c) produces `nono-fltmgr.cat`.
- (d) `Successfully signed: nono-fltmgr.cat`.
- (e) `testsigning   Yes`.
- (f) `Driver package added successfully.` (and an *Published name* / *Installed* line).

> **⚠ Phase 63 lesson:** the EWDK `signtool` won't accept the WDK's *automatic* test-sign during the
> build — that's why you sign **manually here** with your own `CN=NonoTestSign` cert. Don't re-enable
> auto test-sign in the `.vcxproj`.

---

## 7. Confirm the driver is loaded

On the **VM**, elevated:
```powershell
fltmc filters       # your driver should appear...
fltmc instances     # ...and have a live instance at your chosen altitude
```
**Expected:** a row for `nono-fltmgr` at the altitude you chose in §5 (e.g. `365500`), in the
360000–389999 band.

If it's **not** listed, see §11. Do **not** proceed to the deny test until it's loaded.

---

## 8. Build and stage the Rust policy client

The driver asks a user-mode program for the allow/deny decision. Build it on the **dev host** and copy
the `.exe` to the VM.

On the **dev host**:
```bash
cargo build -p nono-fltmgr-client --target x86_64-pc-windows-msvc
# produces: target/x86_64-pc-windows-msvc/debug/nono_fltmgr_client.exe
```
Copy `nono_fltmgr_client.exe` to the VM (e.g. `C:\nono-fltmgr\nono_fltmgr_client.exe`).

---

## 9. Run the deny harness (the actual proof)

On the **VM**:

**Terminal 1 (elevated)** — start the policy client, pointed at the file to protect. Leave it running.
```powershell
C:\nono-fltmgr\nono_fltmgr_client.exe C:\nono-deny-test\secret.txt
```
**Expected:** it prints `connecting to \NonoPolicyPort ...` and stays running (it's now answering the
driver's questions; it exits on Ctrl-C or when the port closes).

**Terminal 2 (elevated)** — create the target file and try to open it via the harness:
```powershell
# Create the deny target
New-Item -ItemType Directory -Force "C:\nono-deny-test" | Out-Null
Set-Content -Path "C:\nono-deny-test\secret.txt" -Value "deny-target"

# Try to open it through the raw Win32 CreateFile API and assert the error
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class FileTest {
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern IntPtr CreateFile(string lpFileName, uint dwDesiredAccess,
        uint dwShareMode, IntPtr lpSecurityAttributes, uint dwCreationDisposition,
        uint dwFlagsAndAttributes, IntPtr hTemplateFile);
    public static int LastError() { return Marshal.GetLastWin32Error(); }
}
'@

$h = [FileTest]::CreateFile("C:\nono-deny-test\secret.txt", 0x80000000, 0, [IntPtr]::Zero, 3, 0, [IntPtr]::Zero)
$err = [FileTest]::LastError()
if ($h.ToInt64() -eq -1 -and $err -eq 5) {
    Write-Output "SC1 PASS: CreateFile denied with ERROR_ACCESS_DENIED (5) as expected"
} else {
    Write-Output ("SC1 FAIL: h={0} err={1}" -f $h, $err)
}
```
**Expected:** `SC1 PASS: CreateFile denied with ERROR_ACCESS_DENIED (5) as expected`.

A quick sanity counter-check (optional): open a *different* file that isn't the target — it should
**succeed** (no deny). That confirms you're denying the one file, not everything.

---

## 10. Capture the evidence + take a post-load snapshot

Create `64-SC1-driver-evidence.md` in the Phase 64 folder and paste in the raw outputs. Copy the
template at the bottom of this runbook (§12). You need, at minimum:
- `fltmc filters` (shows your altitude, no collision)
- `pnputil /add-driver` output (`Driver package added successfully.`)
- `fltmc instances` (driver loaded at your altitude)
- the deny-harness line (`SC1 PASS: ... ERROR_ACCESS_DENIED (5)`)
- the altitude number you chose

Then snapshot the loaded state (from the **dev host**):
```powershell
$diskName = az vm show -g rg-nono-fltmgr-spike -n nono-fltmgr-vm --query "storageProfile.osDisk.name" -o tsv
az snapshot create -g rg-nono-fltmgr-spike -n nono-fltmgr-snap-loaded --source $diskName
```

Commit the evidence + the INF altitude change:
```bash
git add .planning/phases/64-minifilter-spike-implementation-macos-p1-cherry-pick-wave/64-SC1-driver-evidence.md
git add drivers/nono-fltmgr/nono-fltmgr.inf
git commit -m "test(64): SC1 driver evidence — nono-fltmgr loaded + deny harness PASS (DRV-01/DRV-03)

Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>"
```

**Then go back to the main session and reply `approved`** (the Phase 64 resume signal). Done.

---

## 11. Troubleshooting (mostly Phase 63 + DESIGN.md lessons)

| Symptom | Likely cause | Fix |
|--------|-------------|-----|
| `bcdedit /set testsigning on` → *"protected by Secure Boot policy"* | VM is Trusted Launch, not Standard | Reprovision Standard (§10). Don't fight it. |
| Can't RDP to `20.51.161.15:3389` | Corporate egress blocks 3389 | Use **Bastion** (443) or `az vm run-command`. |
| `LaunchBuildEnv.cmd` / build hangs forever | Run through `az vm run-command` (interactive) | Run the build in a **Bastion desktop**. |
| `msbuild` errors `stampinf exit 87` or deletes the `.sys` | Old `.vcxproj` / EWDK auto test-sign | Re-copy current repo `drivers/nono-fltmgr/`; sign manually (§6). |
| `pnputil /add-driver` fails altitude/collision | Altitude already in use | Pick another free number in 360000–389999 (§5). |
| Driver not in `fltmc filters` after install | Load failed or wrong INF | Check `pnputil /enum-drivers`; re-run §6 with the updated INF. |
| Harness prints `SC1 FAIL h=... err=2` (file not found) | Forgot to create the target file | Re-run the `New-Item`/`Set-Content` in §9 Terminal 2. |
| Harness prints `SC1 FAIL` but file exists | Policy client not running / wrong path | Make sure Terminal 1 client is running with the **exact same path** (case-insensitive). |
| **VM BSODs / won't boot** | Driver bug (recursion, IRQL, NULL timeout) | **Restore the snapshot** (below), then fix the C and rebuild. |

**BSOD recovery (restore the testsigning-ready snapshot):**
```powershell
# Swap the VM's OS disk back to a disk created from the safe snapshot.
$snapId = az snapshot show -g rg-nono-fltmgr-spike -n nono-fltmgr-snap-testsigning-ready --query id -o tsv
az disk create -g rg-nono-fltmgr-spike -n nono-fltmgr-osdisk-restored --source $snapId
az vm stop -g rg-nono-fltmgr-spike -n nono-fltmgr-vm
az vm update -g rg-nono-fltmgr-spike -n nono-fltmgr-vm --os-disk nono-fltmgr-osdisk-restored
az vm start -g rg-nono-fltmgr-spike -n nono-fltmgr-vm
```
**Common BSOD causes (from `drivers/nono-fltmgr/DESIGN.md` — for whoever fixes the C):**
- Any `ZwCreateFile`/`NtCreateFile` inside a callback → recursive-I/O stack-overflow BSOD (T-63-01).
- Allocating from `PagedPool` in callback context, or missing `NT_ASSERT(IRQL <= APC_LEVEL)` → IRQL BSOD (T-63-03). Use `ExAllocatePool2(POOL_FLAG_NON_PAGED_NX, ...)`.
- `NULL` timeout in `FltSendMessage` → system hang. It must be `-5000000LL` (500 ms).
- Holding a spinlock across `FltSendMessage` → IRQL-mismatch BSOD. Enqueue, release lock, then send.

---

## 12. Evidence template (paste into `64-SC1-driver-evidence.md`)

```markdown
# Phase 64 SC1 — Driver Evidence (DRV-01 + DRV-03)

**Captured:** <DATE>   **Operator:** <YOU>   **VM:** nono-fltmgr-vm (rg-nono-fltmgr-spike, 20.51.161.15)
**Chosen altitude:** <NUMBER>  (FSFilter Activity Monitor band 360000–389999, non-colliding)

## fltmc filters (altitude confirmed, no collision)
```
<paste>
```

## pnputil /add-driver output
```
<paste>   # expect: Driver package added successfully.
```

## fltmc instances (driver loaded)
```
<paste>   # expect: nono-fltmgr at <NUMBER>
```

## Deny harness output
```
SC1 PASS: CreateFile denied with ERROR_ACCESS_DENIED (5) as expected
```

## Notes / deviations
- <anything that differed; BSOD + recovery if it happened>

**SC1 PASS** — DRV-01 (end-to-end deny) + DRV-03 (test-signing pipeline) satisfied.
```

---

## 10b. Reprovision the VM from scratch (only if the VM is gone)

If §2 shows no VM, rebuild it (Standard security type is mandatory):
```powershell
az group create --name rg-nono-fltmgr-spike --location eastus   # if the RG is also gone
az vm create `
  --resource-group rg-nono-fltmgr-spike --name nono-fltmgr-vm `
  --image MicrosoftWindowsDesktop:windows-11:win11-24h2-pro:latest `
  --size Standard_D4s_v4 `
  --security-type Standard --enable-secure-boot false --enable-vtpm false `
  --admin-username nono-dev --admin-password "<YOUR_SECURE_PASSWORD>" `
  --public-ip-sku Standard --output json
```
Then re-run Phase 63's `63-preflight-azure.ps1` / `63-vm-runcmd-enable-testsigning.ps1` to turn on
TESTSIGNING, install the EWDK toolchain, and take the `nono-fltmgr-snap-testsigning-ready` snapshot
before you come back to §3. (If `D4s_v4` is rejected for quota, that's the size that worked last time —
escalate the quota rather than picking a Trusted-Launch-only size.)

> **⚠ Phase 63 lesson:** the first `az vm create` may fail once with `OSProvisioningTimedOut` — it's
> transient. `az vm delete` then retry once. And `--security-type Standard` needs the
> `Microsoft.Compute/UseStandardSecurityType` feature registered on the subscription (one-time, Owner
> self-serve).
