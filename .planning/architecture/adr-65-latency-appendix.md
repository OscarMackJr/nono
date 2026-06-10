# Minifilter Spike — Latency Measurement Appendix (Phase 65 DRV-04)

**Captured:** 2026-06-09 (tables PENDING the on-VM run — see status below)
**VM:** `nono-fltmgr-vm` (rg `rg-nono-fltmgr-spike`, `20.51.161.15`, Win11 26200)
**Altitude:** 365678 (FSFilter Activity-Monitor band, non-colliding)
**QPC frequency (`g_PerfFreq`):** _<ticks/sec — fill from the unload dump>_
**Driver build:** instrumented `nono-fltmgr.c` (commit `af7cf3c5`); `.sys` is VM-local
and never committed (T-63-05).

> **Status — PENDING on-VM run.** This appendix is the presentation layer (milliseconds)
> of the raw integer-microsecond data captured in
> [`65-SC1-latency-evidence.md`](../phases/65-minifilter-adr-macos-live-re-validation/65-SC1-latency-evidence.md).
> That evidence file is the source of truth; its gate is OPEN until the instrumented
> `.sys` is rebuilt and the deny harness re-run on the spike VM (plan 65-01 Task 2,
> `az`-driven, blocking-human). No values are filled here without real `DbgPrint`
> output (fail-secure — never fabricate measurement data).

This appendix exists so the core ADR
([`adr-65-minifilter-go-no-go.md`](adr-65-minifilter-go-no-go.md)) stays concise and
references the raw tables here rather than inlining them (D-08).

## What is measured

- **SPAN-A (D-02a)** — kernel-IPC round-trip: brackets `FltSendMessage` in
  `NonoWorkerThread`. **Excludes** ring-buffer enqueue + worker wakeup.
- **SPAN-B (D-02b)** — full pre-op → IRP completion: from the `NonoPreCreate` enqueue
  (`NONO_RING_ENTRY.EnqueueQpc`) to just after `FltCompletePendedPreOperation` on the
  deny path. **Includes** enqueue + wakeup (scheduling jitter).

## SPAN-A — Kernel-IPC round-trip (`FltSendMessage` round-trip), D-02a

| Iterations | Min (ms) | Median (ms) | p99 (ms) |
|-----------|----------|-------------|----------|
| _PENDING_ | _PENDING_ | _PENDING_  | _PENDING_ |

## SPAN-B — Full pre-op → IRP completion (`STATUS_ACCESS_DENIED`), D-02b

| Iterations | Min (ms) | Median (ms) | p99 (ms) |
|-----------|----------|-------------|----------|
| _PENDING_ | _PENDING_ | _PENDING_  | _PENDING_ |

> Values are converted from the in-driver integer-microsecond measurements
> (`us = ticks * 1000000 / g_PerfFreq`) to milliseconds for presentation here.

## Notes

- **Ordering expectation:** SPAN-A median < SPAN-B median (SPAN-B includes the
  enqueue + worker-wakeup that SPAN-A excludes).
- **Fail-open envelope:** the `FltSendMessage` finite timeout is `-5000000LL`
  (500 ms, T-63-02). Both measured medians should sit **well under** this envelope —
  the 500 ms figure is the fail-open ceiling, **not** the measured latency (the exact
  Phase-64 ambiguity DRV-04 resolves with a real number, D-01).
- **Perturbation guard:** if SPAN-A ≈ SPAN-B, or any median exceeds ~10 ms, suspect
  logging perturbation (Pitfall 1) — no `DbgPrint` may sit inside a timed span — and
  re-run.

---

**File path note:** This appendix lives at
`.planning/architecture/adr-65-latency-appendix.md` per D-46-A2 precedent
(`.planning/architecture/` is the v2.6+ ADR location; `docs/architecture/` holds
Phase-32-and-earlier ADRs). It is the sibling data file of
`adr-65-minifilter-go-no-go.md` (D-08), and deviates from the SC2 literal
`.planning/adr/` shorthand by repo convention (D-07).
