# Phase 63 — SC1: Azure Test-Signing VM State + SC2: Scaffold Compile Proof

**Status:** AWAITING EXECUTION — runbook drafted; human must provision the VM, paste captures, and
record the compile result.

**Purpose:** Reproducibility evidence for DRV-03 (partial). Proves the Azure Standard-security-type
VM has TESTSIGNING on, Secure Boot off, HVCI off (SC1), and that the `drivers/nono-fltmgr/` scaffold
compiles to `x64\Release\nono-fltmgr.sys` with no MSBuild errors (SC2).

---

## RUNBOOK (executor-drafted; human executes and pastes outputs)

All steps run in Azure (PowerShell/cmd on the VM or from the dev host for provisioning).
The `.sys` binary is a throwaway compile artifact — DO NOT commit it to the repo.

---

### Step 0: Prerequisites (dev host)

Ensure the Azure CLI is installed and you are logged in:

```powershell
az account show        # confirms your active subscription
az version             # confirm CLI >= 2.55 (2024+)
```

If not logged in:

```powershell
az login
az account set --subscription "<YOUR_SUBSCRIPTION_ID>"
```

---

### Step 1: Create the resource group

```powershell
az group create --name nono-fltmgr-rg --location eastus
```

Adjust `--location` to your preferred region. Record the location used in the evidence section below.

---

### Step 2: Provision the VM — Standard security type, Secure Boot OFF

This is the critical step (D-02 / Pitfall A). Use `--security-type Standard`, NOT Trusted Launch.
Trusted Launch enforces Secure Boot and rejects `bcdedit /set testsigning on`.

**Recommended VM size:** Standard_D4s_v5 (4 vCPU, 16 GiB RAM) — sufficient for VS 2026 + WDK.
If unavailable, use Standard_D4s_v4 or Standard_D4_v5 (equivalent generation).

**Image:** Windows 11 Pro 24H2 Gen2. The URN below is the stable `latest` alias; at provision time
Azure resolves it to the current image version. Capture the exact resolved URN in the evidence section.

```powershell
# Primary command — paste this verbatim (adjust admin password to your standard):
az vm create `
  --resource-group nono-fltmgr-rg `
  --name nono-fltmgr-vm `
  --image MicrosoftWindowsDesktop:windows-11:win11-24h2-pro:latest `
  --size Standard_D4s_v5 `
  --security-type Standard `
  --enable-secure-boot false `
  --enable-vtpm false `
  --admin-username nono-dev `
  --admin-password "<YOUR_SECURE_PASSWORD>" `
  --public-ip-sku Standard `
  --output json
```

> **Why `--security-type Standard` and `--enable-secure-boot false`:**  
> Azure Gen2 images default to Trusted Launch (Secure Boot + vTPM enforced). Standard security type
> disables this enforcement. Without it, `bcdedit /set testsigning on` returns:
> "The value is protected by Secure Boot policy and cannot be modified or deleted."  
> (See RESEARCH.md Pitfall A.)

Save the JSON output — it contains the VM's public IP. Record the **exact `imageReference`**
line from the output (or run `az vm show` after creation) in the evidence section below.

If Standard_D4s_v5 is unavailable in your region, fall back to:

```powershell
# Fallback: D4s_v4
az vm create `
  --resource-group nono-fltmgr-rg `
  --name nono-fltmgr-vm `
  --image MicrosoftWindowsDesktop:windows-11:win11-24h2-pro:latest `
  --size Standard_D4s_v4 `
  --security-type Standard `
  --enable-secure-boot false `
  --enable-vtpm false `
  --admin-username nono-dev `
  --admin-password "<YOUR_SECURE_PASSWORD>" `
  --output json
```

---

### Step 3: Open RDP (3389) and connect

```powershell
# Open RDP port if not already open (NSG rule):
az vm open-port --resource-group nono-fltmgr-rg --name nono-fltmgr-vm --port 3389

# Get the public IP:
az vm show --resource-group nono-fltmgr-rg --name nono-fltmgr-vm `
  --show-details --query publicIps -o tsv
```

Connect via Remote Desktop to `<public_ip>:3389`, username `nono-dev`.

---

### Step 4: Take a pre-configuration snapshot (recommended)

Before making any system changes, take a snapshot of the OS disk as a rollback point (D-04 lean
debugging strategy). From your dev host (while connected to the VM is fine):

```powershell
# Get the OS disk name:
$diskName = $(az vm show --resource-group nono-fltmgr-rg --name nono-fltmgr-vm `
  --query "storageProfile.osDisk.name" -o tsv)

# Create snapshot:
az snapshot create `
  --resource-group nono-fltmgr-rg `
  --name nono-fltmgr-snap-baseline `
  --source $diskName
```

---

### Step 5: Enable TESTSIGNING and reboot (on the VM)

Open an elevated Command Prompt or PowerShell **on the VM** (Run as Administrator):

```cmd
bcdedit /set testsigning on
```

Expected output:

```
The operation completed successfully.
```

If you see `"The value is protected by Secure Boot policy"` — STOP. The VM was provisioned as
Trusted Launch, not Standard. Deprovision and reprovision with `--security-type Standard`.

Then reboot:

```cmd
shutdown /r /t 0
```

Reconnect via RDP after the reboot (typically 1-2 minutes).

---

### Step 6: Capture SC1 — msinfo32 export (on the VM)

Run `msinfo32` on the VM (Win + R → `msinfo32`). Verify the following before exporting:

- **Secure Boot State:** Off
- **Kernel DMA Protection:** Off (or "Not applicable")
- **Virtualization-based security (VBS) / Memory Integrity (HVCI):** Off

Export the full msinfo32 report:
File → Save → save as `msinfo32-nono-fltmgr-vm.nfo` or `.txt`.

**Paste the relevant lines from the msinfo32 export into the evidence section below.** At minimum,
paste the lines containing: `Secure Boot State`, `Kernel DMA Protection`,
`Virtualization-based security`, `Memory Integrity`, `BIOS Mode` (should be UEFI for Gen2).

---

### Step 7: Capture SC1 — bcdedit /enum all (on the VM)

Run in an elevated Command Prompt **on the VM**:

```cmd
bcdedit /enum all
```

**Paste the full output into the evidence section below.** The `testsigning` line should show `Yes`.

---

### Step 8: Verify HVCI is off before Phase 64 driver loading

After the reboot, from an elevated PowerShell **on the VM**:

```powershell
# Check VBS/HVCI status via PowerShell (supplements msinfo32):
Get-CimInstance -ClassName Win32_DeviceGuard -Namespace root\Microsoft\Windows\DeviceGuard |
  Select-Object -Property VirtualizationBasedSecurityStatus, SecurityServicesRunning,
  AvailableSecurityProperties, SecurityServicesConfigured
```

If `VirtualizationBasedSecurityStatus` = 0 (Off) and HVCI-related service IDs are absent from
`SecurityServicesRunning`, the VM is safe for test-signed driver loading in Phase 64. Record the
output in the evidence section.

---

### Step 9: Install the WDK toolchain (on the VM)

**Option A: WDK 28000.1761 + VS 2026 (recommended for IDE iteration)**

1. Install VS 2026 Community or Professional with the following workloads and components:
   - Workload: "Desktop development with C++"
   - Individual components: All six "MSVC … Spectre-mitigated libs" for x64/x86
   - Individual component: "Windows Driver Kit" (VSIX for VS 2026)
2. Install the matching Windows SDK 10.0.28000.1
3. Install WDK 28000.1761 from:
   `https://learn.microsoft.com/en-us/windows-hardware/drivers/download-the-wdk`

**Option B: EWDK ISO (faster; self-contained command-line environment)**

1. Download the EWDK ISO that includes VS 2026 Build Tools 18.3.0 + SDK + WDK:
   `https://learn.microsoft.com/en-us/windows-hardware/drivers/download-the-wdk` (see EWDK section)
2. Mount the ISO:
   ```powershell
   Mount-DiskImage -ImagePath "C:\path\to\ewdk.iso"
   ```
3. Open the EWDK build environment:
   ```cmd
   D:\LaunchBuildEnv.cmd
   ```
   (drive letter may differ; check the mount point)

**Option A fallback:** If WDK 28000.1761 + VS 2026 is unavailable, use WDK 26100.6584 + VS 2022
(D-05 alternative). The SDK and WDK build numbers MUST match.

**Version verification (run in the build environment on the VM):**

```powershell
# Confirm msbuild is on PATH and can target the Driver platform:
where.exe msbuild
msbuild -version

# Confirm WDK signing tools are present (needed in Phase 64):
where.exe signtool
where.exe makecert
where.exe certmgr
where.exe inf2cat
```

Paste the `msbuild -version` output into the evidence section.

---

### Step 10: Copy the driver scaffold to the VM

From your dev host (or use a file share / RDP clipboard), copy the entire `drivers/nono-fltmgr/`
directory from the repo to the VM. A convenient approach:

**Via RDP clipboard / file transfer:** drag-drop or use `robocopy` from a network share.

**Via PowerShell SCP (if OpenSSH is installed):**

```powershell
scp -r C:\path\to\nono\drivers\nono-fltmgr nono-dev@<vm_public_ip>:C:\nono-fltmgr
```

The scaffold files required are:
- `nono-fltmgr.vcxproj`
- `nono-fltmgr.vcxproj.filters`
- `nono-fltmgr.inf`
- `nono-fltmgr.c`

---

### Step 11: Compile the scaffold (SC2 proof) — on the VM

From the EWDK command prompt (Option B) or a VS 2026 Developer Command Prompt (Option A),
`cd` to the directory where you copied the scaffold:

```cmd
cd C:\nono-fltmgr

msbuild nono-fltmgr.vcxproj /p:Configuration=Release /p:Platform=x64
```

**Expected outcome (SC2 success):**

```
Build succeeded.
    0 Warning(s)
    0 Error(s)
```

And the file `x64\Release\nono-fltmgr.sys` is produced. Verify:

```cmd
dir x64\Release\nono-fltmgr.sys
```

**Paste the msbuild tail (last ~20 lines, including "Build succeeded" and "0 Error(s)")
AND the `dir x64\Release\nono-fltmgr.sys` listing into the evidence section below.**

Do NOT copy `nono-fltmgr.sys` back to the dev host and do NOT commit it to the repo.
It is a throwaway compile artifact (Pitfall 6: spike `.sys` must not land in main).

---

### Step 12: Record the exact Azure image URN used

After provisioning, run on your dev host:

```powershell
az vm show --resource-group nono-fltmgr-rg --name nono-fltmgr-vm `
  --query "storageProfile.imageReference" -o json
```

Paste the JSON output into the evidence section. It will look like:

```json
{
  "exactVersion": "22621.XXXXX.XXXXXXXX",
  "offer": "windows-11",
  "publisher": "MicrosoftWindowsDesktop",
  "sku": "win11-24h2-pro",
  "version": "22621.XXXXX.XXXXXXXX"
}
```

Record the full `publisher:offer:sku:version` URN for reproducibility.

---

### Step 13: Take a post-TESTSIGNING snapshot (recommended)

After TESTSIGNING is on and the toolchain is verified (but before the driver is installed in Phase 64):

```powershell
az snapshot create `
  --resource-group nono-fltmgr-rg `
  --name nono-fltmgr-snap-testsigning-ready `
  --source $diskName
```

This snapshot is the Phase 64 starting point — if the driver install BSODs, restore from here.

---

## SC1 EVIDENCE (paste here after running the runbook)

> **Instructions:** After running Steps 5-8 and 11-12, paste the captured outputs in the
> corresponding subsections below. Commit this file with the pasted evidence.

### VM Provisioning Details

| Field | Value |
|-------|-------|
| Azure region | *(paste region, e.g. eastus)* |
| VM size | *(paste size, e.g. Standard_D4s_v5)* |
| Image URN (publisher:offer:sku:version) | *(paste from `az vm show` imageReference output)* |
| Exact image version (az exactVersion) | *(paste exact version string)* |
| VM creation date | *(paste date)* |
| Security type | Standard (NOT Trusted Launch) |
| Secure Boot at create | Disabled (`--enable-secure-boot false`) |

### msinfo32 Captures (Secure Boot / HVCI state)

Paste the relevant msinfo32 lines here:

```
[Paste msinfo32 Secure Boot State, Kernel DMA Protection, Virtualization-based security,
 Memory Integrity lines here]
```

Expected: `Secure Boot State: Off`, `Virtualization-based security: Not enabled` or `Off`.

### bcdedit /enum all output (TESTSIGNING state)

Paste the full `bcdedit /enum all` output here. The TESTSIGNING line must read `Yes`:

```
[Paste full bcdedit /enum all output here]
```

Expected: `testsigning                Yes` in the Windows Boot Loader section.

### PowerShell VBS/HVCI status (Get-CimInstance)

Paste the Get-CimInstance DeviceGuard output here:

```
[Paste Get-CimInstance Win32_DeviceGuard output here]
```

Expected: `VirtualizationBasedSecurityStatus: 0` (Off).

### Toolchain Version Verification

Paste `msbuild -version` output and the `where.exe` results here:

```
[Paste msbuild -version + where signtool/makecert/certmgr/inf2cat output here]
```

---

## SC2 EVIDENCE — Scaffold Compile Proof (paste here after running Step 11)

### msbuild Output Tail

Paste the last ~20 lines of `msbuild nono-fltmgr.vcxproj /p:Configuration=Release /p:Platform=x64`:

```
[Paste msbuild output tail here — must include "Build succeeded." and "0 Error(s)"]
```

### dir x64\Release\nono-fltmgr.sys listing

```
[Paste dir x64\Release\nono-fltmgr.sys output here — proves the .sys was produced]
```

---

## SC2 RESULT (fill after running the runbook)

| Field | Value |
|-------|-------|
| msbuild exit code | *(0 = success; non-zero = failure with error description)* |
| `x64\Release\nono-fltmgr.sys` produced | *(Yes / No)* |
| Build warnings | *(count — 0 expected for a clean scaffold build)* |
| Errors | *(count — must be 0 for SC2 pass)* |
| TESTSIGNING confirmed | *(Yes / No — from bcdedit evidence above)* |
| Secure Boot State | *(Off / On — must be Off)* |
| HVCI / Memory Integrity | *(Off / On — must be Off for Phase 64 driver loading)* |

---

## Notes and Deviations

*(Record any deviations from the runbook, fallback choices made (e.g. D4s_v4 instead of D4s_v5,
VS 2022 + WDK 26100 instead of VS 2026 + WDK 28000), or issues encountered here.)*
