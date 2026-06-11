# Phase 65 — SC1 Latency Evidence (DRV-04, D-01..D-04)

**Status:** ✅ **CAPTURED / GATE PASS** — measured on-VM 2026-06-11. Real `DbgPrint`
unload-dump output from the instrumented `.sys` (Task 1 instrumentation `af7cf3c5`),
captured via DebugView in an interactive Bastion desktop after 100 denied creates of
the watched target (deny harness reported `denied 100 / 100`). Values below are the
literal unload-dump lines — not fabricated.

**VM:** `nono-fltmgr-vm` (rg `rg-nono-fltmgr-spike`, IP `20.51.161.15`), Windows 11
26200, Standard security type, Secure Boot OFF, TESTSIGNING ON, HVCI OFF.
**Altitude:** 365678 (FSFilter Activity Monitor band, non-colliding — see 64-SC1).
**Test-sign cert:** `CN=NonoTestSign`, thumbprint
`C40C9572077EDBCEFE7BE51779D29F4BC0C074A7` (reuse if present; recreate via
`New-SelfSignedCertificate` per runbook §10b if the VM was rebuilt).
**QPC frequency (g_PerfFreq):** 10000000 (10 MHz; reported in both SPAN dump lines)

---

## Prep status (2026-06-10) — VM staged, capture needs Bastion

Driven via `az vm run-command` (operator session, az authed to "TWG Architecture POCs"):

- ✅ **Safety snapshot** `nono-fltmgr-snap-pre-65-latency` created (OS disk; the expected `nono-fltmgr-snap-testsigning-ready` was absent).
- ✅ **Instrumented source pushed** — the VM had the *uninstrumented* `nono-fltmgr.c` (0 QPC markers); pushed the committed instrumented copy (`af7cf3c5`, 35 063 B, sha256 `7DF1F95D…`, 17 QPC markers).
- ✅ **Rebuilt** `.sys` via EWDK (`MSBUILD_EXIT=0`), **re-signed** with `CN=NonoTestSign` (`signtool verify /pa` PASS, 17 136 B), **reloaded** the instrumented driver (`fltmc`: loaded, 6 instances, altitude 365678, no BSOD).
- ✅ **DebugView** downloaded to `C:\tools\DebugView`; **client** present at `C:\nono-fltmgr\nono_fltmgr_client.exe`; deny target `C:\nono-deny-test\secret.txt` created.

⛔ **Headless capture via `az vm run-command` is blocked (session-0 limits):** DebugView's kernel capture exits immediately (no interactive desktop → 0-byte log), and the P/Invoke `CreateFile` deny-harness does not execute under SYSTEM/session-0 (returns a null handle, `err=0` — results meaningless, NOT evidence the deny regressed). **The SPAN-A/SPAN-B capture must be run in an interactive Bastion desktop** (the proven Phase 63/64 method per `64-SC1-VM-RUNBOOK.md`). The VM is left fully prepped with the instrumented driver loaded; remaining steps = run DebugView + deny harness + `fltmc unload` interactively, then paste the SPAN lines below.

---

## What this measures

Two QPC spans the instrumented `nono-fltmgr.c` records over ~100 denied creates of
the watched target (`secret.txt`), dumped via `DbgPrint` at `fltmc unload`:

- **SPAN-A (D-02a)** — kernel-IPC round-trip: brackets the `FltSendMessage` call in
  `NonoWorkerThread`. Excludes ring-buffer enqueue + worker wakeup. Recorded only on
  the non-timeout path (a timeout measures the 500 ms fail-open envelope, not latency).
- **SPAN-B (D-02b)** — full pre-op → IRP completion: from the `NonoPreCreate` enqueue
  timestamp (`NONO_RING_ENTRY.EnqueueQpc`) to just after
  `FltCompletePendedPreOperation` on the deny path. Includes scheduling jitter.

Expected ordering: **SPAN-A median < SPAN-B median** (SPAN-B includes enqueue +
wakeup), and **both medians ≪ 500 000 µs** (the `FltSendMessage` fail-open envelope,
T-63-02). If SPAN-A ≈ SPAN-B or any median > ~10 ms, suspect logging perturbation
(Pitfall 1) and re-check that no `DbgPrint` slipped inside a span.

---

## VM run procedure (idempotent — D-04)

> Run from a host with `az` logged in to the spike subscription. The `.sys` stays
> **VM-local** and is never committed (T-63-05).

```powershell
# 1. Probe the VM idempotently (D-04, runbook §2 / §10b)
az vm get-instance-view -g rg-nono-fltmgr-spike -n nono-fltmgr-vm `
  --query "instanceView.statuses[?starts_with(code,'PowerState')].displayName" -o tsv
#   running      -> reuse
#   deallocated  -> az vm start -g rg-nono-fltmgr-spike -n nono-fltmgr-vm
#   gone/missing -> recreate via 64-SC1-VM-RUNBOOK.md §10b, then re-trust the cert

# 2. Push the instrumented source + rebuild the .sys on the VM (EWDK 26H1)
#    (use 64-vm-runcmd-ewdk-build-local.ps1 — headless build invocation)
az vm run-command invoke -g rg-nono-fltmgr-spike -n nono-fltmgr-vm `
  --command-id RunPowerShellScript --scripts @64-vm-runcmd-ewdk-build-local.ps1

# 3. Re-sign (machine store) + reload the instrumented driver
#    signtool sign /v /sm /fd sha256 /sha1 C40C9572077EDBCEFE7BE51779D29F4BC0C074A7 /t <ts> nono-fltmgr.sys
#    fltmc unload nono-fltmgr ; fltmc load nono-fltmgr

# 4. Run the deny harness (~100 denied creates of secret.txt) — runbook §9
#    (each create of the watched target round-trips + is denied -> one SPAN-A + one SPAN-B sample)

# 5. Trigger the unload dump and capture DbgPrint (DebugView / kd)
#    fltmc unload nono-fltmgr
#    -> two lines: "[nono-fltmgr] SPAN-A ... iterations=N min=.. median=.. p99=.."
#                  "[nono-fltmgr] SPAN-B ... iterations=N min=.. median=.. p99=.."
```

---

## SPAN-A — Kernel-IPC round-trip (FltSendMessage round-trip), D-02a

| Iterations | Min (µs) | Median (µs) | p99 (µs) |
|-----------|----------|-------------|----------|
| 100 | 387 | 553 | 1460 |

Raw `DbgPrint` output:

```
[nono-fltmgr] SPAN-A kernel-IPC round-trip: iterations=100 min=387 us median=553 us p99=1460 us (freq=10000000)
```

## SPAN-B — Full pre-op → IRP completion (STATUS_ACCESS_DENIED), D-02b

| Iterations | Min (µs) | Median (µs) | p99 (µs) |
|-----------|----------|-------------|----------|
| 100 | 486 | 569 | 1478 |

Raw `DbgPrint` output:

```
[nono-fltmgr] SPAN-B full pre-op->completion: iterations=100 min=486 us median=569 us p99=1478 us (freq=10000000)
```

---

## Acceptance gate (plan 65-01 Task 2)

- [x] SPAN-A + SPAN-B each report iterations (~100), min, median, p99 in µs + QPC freq + VM context — 100 iters each, freq 10 MHz, VM `nono-fltmgr-vm` @ altitude 365678
- [x] `git ls-files drivers/nono-fltmgr/*.sys` is empty (the `.sys` is VM-local — T-63-05) — verified empty 2026-06-11
- [x] SPAN-A median < SPAN-B median; both ≪ 500 000 µs (the 500 ms fail-open envelope) — 553 < 569 µs (ordered at every percentile: min 387<486, p99 1460<1478); both ~900× under the 500 ms envelope

**FINAL:** ✅ **PASS** (measured 2026-06-11). SPAN-A median 553 µs / p99 1460 µs; SPAN-B median
569 µs / p99 1478 µs; QPC freq 10 MHz; 100 iterations each. Ordering SPAN-A < SPAN-B holds at
min/median/p99; both medians ≈ 0.55 ms ≪ the 500 ms `FltSendMessage` fail-open envelope (T-63-02).
Feeds plan 65-03 `adr-65-latency-appendix.md`.

> Gate resume-signal (plan 65-01 Task 2): type **"approved"** with the SPAN-A and
> SPAN-B median+p99 numbers, or describe the VM/build issue. This evidence file is the
> data source for plan 65-03's `adr-65-latency-appendix.md`.
