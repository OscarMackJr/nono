# nono-fltmgr Minifilter Spike: ADR Pointer

**Status:** Accepted (pointer to canonical design doc)
**Date:** 2026-06-06
**Phase:** 63 (v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity)
**Requirement:** DRV-03 (partial)
**Related ADR:** [nono-fltmgr BSOD-Avoidance Pre-Code Gate](../../drivers/nono-fltmgr/DESIGN.md)

## Context

The canonical pre-code design document for the Gap 6b minifilter spike is co-located with the
driver source code at `drivers/nono-fltmgr/DESIGN.md` (per D-09: design docs should live
next to the code they govern so they stay in sync as the implementation evolves).

This stub exists so the `docs/architecture/` ADR index — where all six of the fork's existing
architecture decision records live — has a pointer to the minifilter design gate. Readers
browsing the ADR set will find this entry and be directed to the canonical document.

**Canonical document:** [`drivers/nono-fltmgr/DESIGN.md`](../../drivers/nono-fltmgr/DESIGN.md)

The canonical doc specifies the full BSOD-avoidance contract: ring-buffer + worker-thread IPC,
`NonPagedPoolNx` allocations, IRQL assertions, finite `FltSendMessage` timeout (~500 ms →
`STATUS_TIMEOUT`), the FSFilter Activity Monitor altitude band (360000–389999), and the AV-range
(320000–329998) avoidance constraint. It is the hard pre-code gate (D-10) that every Phase 64
implementer must read before writing any driver logic.
