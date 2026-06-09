# nono drivers

Out-of-workspace WDK/MSBuild driver projects for the nono Windows kernel-enforcement
spike (Gap 6b minifilter feasibility). These are **C/WDK** drivers, not Rust (see
`nono-fltmgr/DESIGN.md` for the windows-drivers-rs rationale) and are intentionally
**outside the Cargo workspace** (built with `msbuild`, not `cargo`).

> **The spike `.sys` is VM-local throwaway — never commit it, never MSI-bundle it**
> (`nono-fltmgr/DESIGN.md` §Security, T-63-05). Only the source (`.c`, `.h`, `.inf`,
> `.vcxproj`, `.vcxproj.filters`) lives in the repo.
>
> **`crates/nono-cli/data/windows/nono-wfp-driver.sys` is a separate WFP placeholder and
> is UNTOUCHED by this spike** — do not confuse it with `nono-fltmgr.sys`, and do not
> modify it or the MSI as part of minifilter work.

`nono-fltmgr/` — minifilter (FltMgr) that intercepts `IRP_MJ_CREATE` and consults a
user-mode policy client (`crates/nono-fltmgr-client`) over `\NonoPolicyPort` to allow/deny
a file open. Proven end-to-end on a live VM — see
`.planning/phases/64-.../64-SC1-driver-evidence.md` (SC1 PASS, `ERROR_ACCESS_DENIED`) and
the step-by-step operator guide `.planning/phases/64-.../64-SC1-VM-RUNBOOK.md`.

---

## Prerequisites (test VM)

- **Azure VM**: Windows 11, **Standard** security type (NOT Trusted Launch),
  **Secure Boot OFF**, **HVCI OFF**, **TESTSIGNING ON**. Provision per Phase 63
  (`63-SC1-vm-state.md`); Trusted Launch blocks `bcdedit /set testsigning on`.
- **Toolchain on the VM**: EWDK 26H1 ISO (VS Build Tools 18.3.0 + SDK/WDK 10.0.28000) —
  open the build env with `LaunchBuildEnv.cmd` (interactive shell) or `SetupBuildEnv.cmd`
  (configures the current shell; use this for headless `az vm run-command`). Alternatively
  WDK 28000.1761 + VS 2026. `msbuild`, `signtool`, `inf2cat`, `certmgr` must be on PATH
  inside the build env.
- **Altitude**: run `fltmc filters` on the VM and pick a **non-colliding** number in the
  FSFilter Activity Monitor band **360000–389999**, avoiding the AV range 320000–329998.
  The validated spike value is **365678**. Set it in `nono-fltmgr/nono-fltmgr.inf`
  (`Instance1.Altitude`). The official Microsoft altitude (request to `fsfcomm@microsoft.com`)
  is pending; the test-signed spike uses the temporary non-colliding number.
- **BSOD safeguard**: take an OS-disk snapshot before first load (the driver is
  `StartType = DEMAND`, so a reboot recovers without auto-loading). See
  `nono-fltmgr/DESIGN.md` for the BSOD-avoidance contract (no `ZwCreateFile`/`NtCreateFile`
  in callbacks, `NonPagedPoolNx`/`ExAllocatePool2`, finite `FltSendMessage` timeout, IRQL
  asserts).

---

## Pipeline 1 — C minifilter: build → test-sign → load

All on the VM, in an **elevated** EWDK build shell, from the driver directory (e.g.
`C:\nono-fltmgr`). Stage the `drivers/nono-fltmgr/` source onto the VM first.

```cmd
:: 1. Build
cd C:\nono-fltmgr
msbuild nono-fltmgr.vcxproj /p:Configuration=Release /p:Platform=x64
::    -> x64\Release\nono-fltmgr.sys   (DO NOT commit this file)

:: 2. Flatten the package: inf2cat resolves [SourceDisksFiles] relative to the INF,
::    so the .sys must sit next to the .inf.
copy /Y x64\Release\nono-fltmgr.sys .

:: 3. Create a test code-signing cert and trust it as a machine root.
::    NOTE: the classic `makecert` tool is deprecated and absent from modern EWDKs;
::    use PowerShell New-SelfSignedCertificate instead. (Legacy equivalent, for older
::    kits, was: makecert -r -pe -ss PrivateCertStore -n "CN=NonoTestSign" NonoTestSign.cer
::    then certmgr /add NonoTestSign.cer /s /r localMachine root.)
powershell -Command "$c = New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=NonoTestSign' -CertStoreLocation Cert:\LocalMachine\My -KeyUsage DigitalSignature -FriendlyName 'NonoTestSign' -NotAfter (Get-Date).AddYears(3); Export-Certificate -Cert $c -FilePath NonoTestSign.cer | Out-Null; Import-Certificate -FilePath NonoTestSign.cer -CertStoreLocation Cert:\LocalMachine\Root | Out-Null; $c.Thumbprint"
::    -> copy the printed <THUMBPRINT>

:: 4. Generate the catalog from the INF
inf2cat /driver:. /os:10_x64 /uselocaltime
::    -> nono-fltmgr.cat

:: 5. Sign. The cert is in the MACHINE store, so signtool needs /sm. Embedded-sign the
::    .sys directly (most robust; catalog-only signing can hit 0x80070241 on stale copies).
signtool sign /v /sm /fd sha256 /sha1 <THUMBPRINT> /t http://timestamp.digicert.com nono-fltmgr.cat
signtool sign /v /sm /fd sha256 /sha1 <THUMBPRINT> /t http://timestamp.digicert.com x64\Release\nono-fltmgr.sys

:: 6. Ensure test-signing is on (one-time per VM; needs a reboot to take effect)
bcdedit /set testsigning on
bcdedit /enum | findstr testsigning      :: expect: testsigning   Yes

:: 7. Install. pnputil /add-driver only STAGES a minifilter (it does not run the service
::    section), so use rundll32 DefaultInstall — it creates the service, writes the
::    Instances\...\Altitude registry, and copies the .sys to System32\drivers.
::    (pnputil /add-driver nono-fltmgr.inf /install stages the package but does not load it.)
rundll32.exe setupapi.dll,InstallHinfSection DefaultInstall 132 C:\nono-fltmgr\nono-fltmgr.inf

:: 8. Force-deploy the freshly signed .sys (DefaultInstall may skip the copy if DriverVer
::    is unchanged), then load (StartType = DEMAND -> must load explicitly).
copy /Y x64\Release\nono-fltmgr.sys C:\Windows\System32\drivers\nono-fltmgr.sys
fltmc load nono-fltmgr

:: 9. Verify
fltmc filters       :: nono-fltmgr at the chosen altitude (e.g. 365678)
fltmc instances     :: attached to C: / E: / \Device\Mup
```

To replace a rebuilt driver: `fltmc unload nono-fltmgr` (or reboot — demand-start) before
copying the new `.sys`; verify with `certutil -hashfile` that the deployed `.sys` matches
the build before `fltmc load`.

---

## Pipeline 2 — Rust user-mode policy client (`nono-fltmgr-client`)

`crates/nono-fltmgr-client` is the user-mode policy client that connects to
`\NonoPolicyPort`, receives `NonoIpcRequest` messages from the driver via `FilterGetMessage`,
and replies allow/deny via `FilterReplyMessage`. The minifilter is transparent until this
client connects (fail-open with no client); once connected it evaluates opens of the
deny-target.

```powershell
# Build on the Windows dev host. Use +crt-static so the .exe is self-contained
# (a dynamically-linked build needs the VC++ redist, which a clean VM lacks -> 0xc000007b).
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release -p nono-fltmgr-client --target x86_64-pc-windows-msvc
Remove-Item Env:\RUSTFLAGS
# -> target\x86_64-pc-windows-msvc\release\nono_fltmgr_client.exe
```

```cmd
:: On the VM (elevated), start the client BEFORE triggering any open of the deny-target.
:: It runs until Ctrl-C or the port disconnects.
nono_fltmgr_client.exe C:\nono-deny-test\secret.txt
```

Then, in a second terminal, attempt to open the deny-target (e.g. via a Win32 `CreateFile`
harness — see `64-SC1-VM-RUNBOOK.md` §9). Expect `ERROR_ACCESS_DENIED (5)` and a `[DENY ]`
line in the client window. (`cargo build -p nono-fltmgr-client` without `--target` also
works on a Windows host; it compiles to an empty crate on Linux/macOS CI.)

---

## Out-of-scope artifacts (do not touch)

- **`crates/nono-cli/data/windows/nono-wfp-driver.sys`** — WFP driver **placeholder**,
  UNTOUCHED by the minifilter spike. Not the same as `nono-fltmgr.sys`. The MSI is likewise
  untouched.
- **`nono-fltmgr.sys` / `.cat` / `.obj`** — VM-local throwaway build artifacts; never
  committed (`DESIGN.md` §Security, T-63-05).
- **Official altitude** — pending `fsfcomm@microsoft.com` assignment; the spike uses a
  temporary non-colliding Activity Monitor number (see `64-SC1-driver-evidence.md`).

## See also

- `nono-fltmgr/DESIGN.md` — BSOD-avoidance pre-code gate, IPC design rules, altitude table.
- `.planning/phases/64-.../64-SC1-VM-RUNBOOK.md` — full operator runbook (provisioning →
  build → sign → load → deny test → evidence), with the corporate-proxy / file-transfer
  workarounds.
- `.planning/phases/64-.../64-SC1-driver-evidence.md` — SC1 PASS evidence + the defect log
  from the live bring-up.
