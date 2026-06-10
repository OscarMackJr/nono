# Phase 65 — SC1 Latency Evidence (DRV-04, D-01..D-04)

**Status:** ⛔ **GATE OPEN / PENDING VM RUN** — staged 2026-06-09. This is the
blocking-human gate for plan 65-01 Task 2. The instrumentation (Task 1) is committed
(`af7cf3c5`); this file captures the on-VM measured latency once the deny harness is
re-run against the instrumented `.sys`. **No values below may be filled without real
on-VM `DbgPrint` output** (Pitfall 4 / fail-secure: never fabricate measurement data).

**VM:** `nono-fltmgr-vm` (rg `rg-nono-fltmgr-spike`, IP `20.51.161.15`), Windows 11
26200, Standard security type, Secure Boot OFF, TESTSIGNING ON, HVCI OFF.
**Altitude:** 365678 (FSFilter Activity Monitor band, non-colliding — see 64-SC1).
**Test-sign cert:** `CN=NonoTestSign`, thumbprint
`C40C9572077EDBCEFE7BE51779D29F4BC0C074A7` (reuse if present; recreate via
`New-SelfSignedCertificate` per runbook §10b if the VM was rebuilt).
**QPC frequency (g_PerfFreq):** _<fill from the unload dump line>_

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
| _PENDING_ | _PENDING_ | _PENDING_  | _PENDING_ |

Raw `DbgPrint` output:

```
<paste the SPAN-A unload-dump line here>
```

## SPAN-B — Full pre-op → IRP completion (STATUS_ACCESS_DENIED), D-02b

| Iterations | Min (µs) | Median (µs) | p99 (µs) |
|-----------|----------|-------------|----------|
| _PENDING_ | _PENDING_ | _PENDING_  | _PENDING_ |

Raw `DbgPrint` output:

```
<paste the SPAN-B unload-dump line here>
```

---

## Acceptance gate (plan 65-01 Task 2)

- [ ] SPAN-A + SPAN-B each report iterations (~100), min, median, p99 in µs + QPC freq + VM context
- [ ] `git ls-files drivers/nono-fltmgr/*.sys` is empty (the `.sys` is VM-local — T-63-05)
- [ ] SPAN-A median < SPAN-B median; both ≪ 500 000 µs (the 500 ms fail-open envelope)

**FINAL:** _PASS / FAIL — fill after the VM run with the median+p99 for both spans._

> Gate resume-signal (plan 65-01 Task 2): type **"approved"** with the SPAN-A and
> SPAN-B median+p99 numbers, or describe the VM/build issue. This evidence file is the
> data source for plan 65-03's `adr-65-latency-appendix.md`.
