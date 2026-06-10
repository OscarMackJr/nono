# 65-01 SUMMARY — Minifilter latency instrumentation (DRV-04)

**Status:** Task 1 COMPLETE; Task 2 (on-VM measurement) **GATE OPEN / blocking-human**.

## What was built

Added `KeQueryPerformanceCounter` latency instrumentation to the Phase-64 minifilter
spike driver (`drivers/nono-fltmgr/nono-fltmgr.c`), the net-new code for DRV-04
(D-01..D-04). Two spans are measured over denied creates and dumped at unload:

- **SPAN-A (D-02a)** — kernel-IPC round-trip, brackets `FltSendMessage`; recorded only
  on the non-timeout path.
- **SPAN-B (D-02b)** — full pre-op → IRP completion; start rides a new kernel-internal
  `NONO_RING_ENTRY.EnqueueQpc` field, lifted at dequeue, end taken after
  `FltCompletePendedPreOperation` on the deny path.

QPC frequency cached once in `DriverEntry`; raw ticks accumulated lock-free (single-slot
ring serializes writers); `NonoDumpSpan` helper sorts + computes min/median/p99 via
integer-microsecond math and `DbgPrint`s at unload (outside any timed span).

## Key files

- created/modified: `drivers/nono-fltmgr/nono-fltmgr.c` (+108 lines) — commit `af7cf3c5`
- created: `.planning/.../65-SC1-latency-evidence.md` (staged, PENDING) — commit `29e5503e`

## Verification (Task 1, automated — all PASS)

- `KeQueryPerformanceCounter` present in: freq cache, SPAN-A (×2), SPAN-B start, SPAN-B end (grep count 7 incl. comments; ≥4 required) ✓
- `EnqueueQpc` in struct + enqueue + dequeue-lift + SPAN-B delta ✓
- `nono-fltmgr.h` `C_ASSERT(sizeof(NONO_IPC_REQUEST) == 532)` byte-for-byte unchanged ✓
- `g_SpanA`/`g_SpanB` accumulators + `NonoFltUnload` median/p99 dump (integer-µs) ✓
- No `DbgPrint`/`Zw`/`Nt` file API inside any QPC span ✓
- `timeout.QuadPart = -5000000LL` (500 ms) + `FLT_FILE_NAME_OPENED` unchanged ✓
- `git ls-files drivers/nono-fltmgr/*.sys` empty (T-63-05) ✓

## Deviations

- **Task 2 not executed (blocking-human gate OPEN):** `az` CLI is unavailable in the
  execution session, so the VM rebuild→re-sign→reload→deny-harness→latency-capture
  cycle cannot be driven autonomously. The exact idempotent procedure + PENDING
  SPAN-A/SPAN-B tables are staged in `65-SC1-latency-evidence.md` for Oscar to run.
  **No measurement values were fabricated** (fail-secure).

## Self-Check: PASSED (Task 1) — Task 2 deferred to human/VM gate

## Downstream

`65-SC1-latency-evidence.md` is the data source for plan 65-03's
`adr-65-latency-appendix.md`. Until the VM run fills it, 65-03's appendix latency cells
remain PENDING and the ADR ships `Status: Proposed` (which it does regardless, per D-06).
