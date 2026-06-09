# Phase 64 — SC1 Driver Evidence (DRV-01 + DRV-02 + DRV-03)

**Status:** SC1 PASS (2026-06-09). Live end-to-end deny demonstrated on the Azure
Secure-Boot-OFF / HVCI-off test VM. DRV-01 (targeted deny), DRV-02 (user-mode policy
round-trip), and DRV-03 (full test-signing pipeline) satisfied.

**VM:** `nono-fltmgr-vm` (rg `rg-nono-fltmgr-spike`, IP `20.51.161.15`), Windows 11,
Standard security type, Secure Boot OFF, TESTSIGNING ON, HVCI OFF (per Phase 63 SC1).

**Chosen altitude:** **365678** — FSFilter Activity Monitor band (360000–389999),
non-colliding (sits between `WdFilter` 328010 and `UCPD` 385250.5; nothing else at
365678), and clear of the AV range 320000–329998 (D-08).

---

## Test-signing pipeline (DRV-03)

The Phase 63 pipeline was adapted to a current EWDK (26H1), which surfaced several
real deviations from the runbook (all now corrected):

- **Cert:** `makecert` is deprecated/absent on modern EWDK → created the test cert with
  PowerShell `New-SelfSignedCertificate -Type CodeSigningCert -Subject CN=NonoTestSign`
  in `Cert:\LocalMachine\My`, trusted in `Cert:\LocalMachine\Root` (thumbprint
  `C40C9572077EDBCEFE7BE51779D29F4BC0C074A7`).
- **Package layout:** `inf2cat /driver:.` resolves `[SourceDisksFiles]` relative to the
  INF, so the built `.sys` (in `x64\Release\`) was flattened next to the INF first.
- **Sign:** `signtool sign /v /sm /fd sha256 /sha1 <thumbprint> /t <timestamp>` — `/sm`
  required because the cert lives in the machine store; the `.sys` is **embedded-signed**
  directly (catalog-only signing hit `0x80070241` ERROR_INVALID_IMAGE_HASH on stale copies).
- **Install:** `pnputil /add-driver … /install` only *stages* a minifilter (it does not run
  the service section). Used `rundll32 setupapi.dll,InstallHinfSection DefaultInstall 132
  nono-fltmgr.inf` to create the service + `Instances\…\Altitude` registry + copy the `.sys`.
- **Load:** `fltmc load nono-fltmgr` (StartType = DEMAND, D-06 boot-loop safeguard).

INF fixes for inf2cat signability: added `DriverVer = 06/09/2026,1.0.0.0`,
`[SourceDisksNames]`/`[SourceDisksFiles]` declaring `nono-fltmgr.sys`, and set
`Instance1.Altitude = "365678"` (placeholder `370020` replaced).

---

## `fltmc filters` (altitude confirmed, no collision)

```
Filter Name                     Num Instances    Altitude    Frame
------------------------------  -------------  ------------  -----
bindflt                                 1       409800         0
FsDepends                               5       407000         0
UCPD                                    5       385250.5       0
nono-fltmgr                             5       365678         0
WdFilter                                5       328010         0
applockerfltr                           4       265000         0
storqosflt                              0       244000         0
wcifs                                   0       189900         0
CldFlt                                  0       180451         0
bfs                                     7       150000         0
FileCrypt                               0       141100         0
luafv                                   1       135000         0
UnionFS                                 0       130850         0
npsvctrig                               1        46000         0
Wof                                     2        40700         0
FileInfo                                5        40500         0
```

## `fltmc instances` (driver attached at 365678)

```
nono-fltmgr           E:                                         365678     nono-fltmgr Instance      0     0000000c
nono-fltmgr                                                      365678     nono-fltmgr Instance      0     0000000c
nono-fltmgr                                                      365678     nono-fltmgr Instance      0     0000000c
nono-fltmgr           C:                                         365678     nono-fltmgr Instance      0     0000000c
nono-fltmgr           \Device\Mup                                365678     nono-fltmgr Instance      0     0000000c
```

## Deny harness output (D-01) — SC1 PASS

User-mode policy client (`nono_fltmgr_client.exe C:\nono-deny-test\secret.txt`) running,
then the scripted Win32 `CreateFile` harness against the deny target:

```
SC1 PASS (attempt 1): ERROR_ACCESS_DENIED (5)
```

Client window (kernel→user round-trip, DRV-02) — the intercepted create being denied:

```
nono-fltmgr-client: connecting to \NonoPolicyPort (deny target: C:\nono-deny-test\secret.txt)
[DENY ] \Device\HarddiskVolume4\nono-deny-test\secret.txt
```

End-to-end chain proven: `IRP_MJ_CREATE` (secret.txt) → kernel pre-filter → ring buffer →
`FltSendMessage` over `\NonoPolicyPort` → Rust client path-match → `FilterReplyMessage`
(Decision=deny) → worker completes IRP `STATUS_ACCESS_DENIED` → caller `CreateFile` returns
`ERROR_ACCESS_DENIED (5)`.

---

## Defects found and fixed during live UAT

The live VM run flushed out ~18 real defects that no Windows dev-host build could catch
(no kernel C toolchain / no kernel runtime locally). Committed fixes:

| # | Area | Defect | Fix commit |
|---|------|--------|-----------|
| 1–3 | build | `_Static_assert` (use `C_ASSERT`); `POOL_FLAG_NON_PAGED_NX` (→ `POOL_FLAG_NON_PAGED`); `FltCompletePendingPreOp` (→ `FltCompletePendedPreOperation`) | `b4d2fef6` |
| 4–5 | INF | empty `DriverVer`; missing `[SourceDisksFiles]`/`[SourceDisksNames]` | `a81627b0` |
| 6 | load | `fltmc load` hang — `FltStartFiltering` ran before ring/port/worker init; + client-gate transparency | `39be6f19` |
| 7 | deadlock | `FLT_FILE_NAME_NORMALIZED` in pre-create → re-entrant deadlock (→ `FLT_FILE_NAME_OPENED`) | `08019735` |
| 8 | client | exact path match vs device-form name (→ tail match) | `08019735` |
| 9 | port | per-instance teardown closed the driver-wide comm port → `0x80070002` on connect | `97af1cec` |
| 10 | load | intercepting ALL system I/O made the desktop hang → kernel-scope to the deny-target leaf name | `d0bb64dc` |
| 11 | semantics | allowed create completed with `FLT_PREOP_COMPLETE`+SUCCESS → "parameter is incorrect" (→ `FLT_PREOP_SUCCESS_NO_CALLBACK`) | `f5ef9a74` |
| 12 | determinism | single-slot fail-open let denied creates slip through under contention → fail-closed on back-pressure for the watched file | `9267a131` |
| 13 | IPC | client sent `size_of::<ReplyBuf>()` (24, padded) to `FilterReplyMessage`; driver expects 20 → reply never delivered → timeout fail-open (the "[DENY] logged but file opens" bug) | `aec9d78b` |

Operational/procedural lessons (corporate-proxy/RBAC/transfer) also surfaced and are
captured in `64-SC1-VM-RUNBOOK.md` updates.

---

## SC1 RESULT

| Field | Value |
|-------|-------|
| Driver loaded | Yes — `nono-fltmgr` at altitude 365678, 5 instances |
| Altitude non-colliding | Yes (360000–389999 band; gap between 328010 and 385250.5) |
| Test-signed `.sys` | Yes (embedded-signed, `CN=NonoTestSign`, testsigning ON) |
| Deny harness | **SC1 PASS** — `ERROR_ACCESS_DENIED (5)` on the deny target |
| User-mode round-trip (DRV-02) | Yes — `[DENY ]` logged by `nono_fltmgr_client.exe` via `\NonoPolicyPort` |
| Allowed (non-target) creates | Open normally (allow path = `FLT_PREOP_SUCCESS_NO_CALLBACK`) |
| `.sys` committed to repo | No (VM-local only, T-63-05) |

**SC1 PASS** — DRV-01 + DRV-02 + DRV-03 satisfied.
