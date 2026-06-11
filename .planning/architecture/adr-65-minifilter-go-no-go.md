# Minifilter Spike Go/No-Go (Gap 6b FltMgr feasibility verdict)

**Status:** Accepted
<!-- D-06: shipped Proposed; flipped to Accepted 2026-06-11 after Oscar's human review (latency input confirmed populated per §2; see the D-06 gate under §6). Confirms the lean No-go / Conditional-go verdict: DRV-PROD-01 deferred to v2.11/v3.0. -->
**Date:** 2026-06-09
**Phase:** 65 (minifilter-adr-macos-live-re-validation)
**Decision IDs:** D-01..D-09 (Phase 65 CONTEXT)
**Related:** drivers/nono-fltmgr/DESIGN.md,
             .planning/phases/64-minifilter-spike-implementation-macos-p1-cherry-pick-wave/64-SC1-driver-evidence.md,
             .planning/architecture/adr-65-latency-appendix.md

## Context

Gap 6b asked whether a Windows filesystem **minifilter** (FltMgr) can enforce nono's
capability denies at the kernel boundary — the one isolation surface the current
Windows model (WFP for network + AppContainer/Low-IL for filesystem and process) does
not occupy with a dedicated driver. Phase 64 built and live-proved a spike minifilter
(`drivers/nono-fltmgr/`): DRV-01 (targeted deny), DRV-02 (user-mode policy round-trip
over `\NonoPolicyPort`), DRV-03 (full test-signing pipeline) all reached **SC1 PASS** on
a Secure-Boot-OFF / HVCI-OFF Azure test VM, denying a create with `ERROR_ACCESS_DENIED`
via a real kernel→user→kernel round-trip.

DRV-04 (this ADR) records the feasibility verdict: does the spike justify a **production**
minifilter milestone (DRV-PROD-01: EV/WHQL-signed, MSI-integrated, kernel-version-hardened)?
The verdict is **evidence-derived and NOT pre-committed** (D-05): it weighs five inputs
against the existing WFP+AppContainer security posture and is surfaced for Oscar's
review (D-06) — this ADR ships `Status: Proposed`.

## Goals

- Record the six DRV-04 topics as a single committed decision artifact (D-09).
- Tie the verdict to measured evidence (latency, spike-defect signal, cert cost,
  altitude-assignment status, maintenance burden) — not to a pre-chosen direction.
- Surface a recommendation for human review; do not silently lock the production
  go/no-go.

## 1. Interception design — how the spike intercepts

The spike registers an `IRP_MJ_CREATE` pre-operation callback (`NonoPreCreate`) at
altitude 365678. On a watched create it returns `FLT_PREOP_PENDING`, enqueues the
request into a single-slot ring buffer (`NONO_RING_ENTRY`, spinlock + event), and a
PASSIVE_LEVEL worker (`NonoWorkerThread`) round-trips the path/PID/desired-access to a
user-mode policy client via `FltSendMessage` over `\NonoPolicyPort`. The reply drives
`FltCompletePendedPreOperation` — `FLT_PREOP_COMPLETE` + `STATUS_ACCESS_DENIED` to deny,
`FLT_PREOP_SUCCESS_NO_CALLBACK` to allow (resume down the stack). Full design,
trust boundaries, and the BSOD-avoidance gate (T-63-01..05) are in
[`drivers/nono-fltmgr/DESIGN.md`](../../drivers/nono-fltmgr/DESIGN.md) and the deny chain
in [`64-SC1-driver-evidence.md`](../phases/64-minifilter-spike-implementation-macos-p1-cherry-pick-wave/64-SC1-driver-evidence.md)
(referenced, not duplicated).

## 2. Measured latency + finite-timeout fail-open

Phase 64 never instrumented an actual round-trip — the ~500 ms figure is the
`FltSendMessage` **finite-timeout fail-open envelope** (`timeout.QuadPart = -5000000LL`,
T-63-02), NOT the measured latency. Phase 65 added `KeQueryPerformanceCounter`
instrumentation (commit `af7cf3c5`) measuring two spans over ~100 denied creates:
SPAN-A (kernel-IPC round-trip) and SPAN-B (full pre-op → IRP completion). Raw min/median/p99
are in [`adr-65-latency-appendix.md`](adr-65-latency-appendix.md).

> **Latency input status: ✅ CAPTURED 2026-06-11 (plan 65-01 Task 2).** Measured on the
> spike VM over 100 denied creates: **SPAN-A median 0.553 ms** (p99 1.460 ms), **SPAN-B
> median 0.569 ms** (p99 1.478 ms), QPC freq 10 MHz. The measured median is **one input
> among five** — DRV-04 deliberately does NOT define a pass/fail latency threshold (Open
> Question 3). Both medians sit **~900× under** the 500 ms fail-open envelope and the
> SPAN-A < SPAN-B ordering holds at every percentile — the verdict's latency column is
> now **confirmed** (favorable). Raw tables: appendix.

## 3. `windows-drivers-rs` not viable (C/C++ driver decision)

The driver is an **out-of-workspace C/C++ WDK MSBuild project** (`drivers/nono-fltmgr/`),
NOT a Cargo member. `windows-drivers-rs` was ruled out: it is early-stage, KMDF-oriented,
and does not cover the FltMgr minifilter surface (`FLT_REGISTRATION`,
`FltCreateCommunicationPort`, `FltSendMessage`) the spike requires. The inverse holds
for user mode — the policy client is a `#[cfg(windows)]` Cargo workspace member over
`windows-sys`. (Ref: `64-CONTEXT.md`; REQUIREMENTS.md Out-of-Scope.)

## 4. FltMgr vs ETW rationale

FltMgr **enforces** (it can pend and deny an IRP); ETW only **observes** (it is a
telemetry/tracing surface that cannot block a create). A capability sandbox needs
enforcement at the create boundary, so ETW is not a substitute for the deny path. ETW
remains relevant to the separate EDR-integration concern (EDR-INTEG-01, deferred), not
to capability enforcement.

## 5. Altitude

The spike uses altitude **365678** in the FSFilter Activity-Monitor band
(360000–389999), live-confirmed non-colliding by `fltmc filters` (sits in the gap
between `WdFilter` 328010 and `UCPD` 385250.5; clear of the AV range 320000–329998).
**Official Microsoft altitude assignment** (fsfcomm@microsoft.com) is **PENDING /
not-yet-requested** — required before any production load-order guarantee, but not
needed for the spike. A production milestone must request and receive an assigned
altitude.

## 6. Decision — Go/No-Go verdict

### Verdict scoring table (D-05 — five inputs against the WFP+AppContainer gap)

| Option | Security gap closed vs WFP+AppContainer | Cert cost (EV/WHQL) | Maintenance burden (kernel-version) | Spike-defect signal (18 defects) | Verdict |
|--------|------------------------------------------|---------------------|-------------------------------------|----------------------------------|---------|
| **Go** (production driver now) | Adds kernel-enforced **file-create** deny that AppContainer expresses only coarsely — a real but **incremental** gain over the existing kernel-enforced model | **High** — EV cert + WHQL/attestation signing + Microsoft Partner Center; recurring | **High** — must track Windows kernel/WDK revisions; boot-loop + BSOD risk surface | **High-risk** — 18 live defects (re-entrant deadlock, port teardown, IPC size mismatch) on a *single-slot spike*; production hardening is materially larger | **Lean No** |
| **No-go** (WFP+AppContainer suffices) | Accepts the current model; no new kernel-create deny. The existing model is already kernel-enforced and structurally sound | **None** | **None** (no new kernel surface) | n/a | **Lean Yes** |
| **Conditional-go** (gated) | Defer Go until a **specific** capability gap is identified that AppContainer/WFP cannot express (e.g. content-level or per-handle file policy), AND latency lands well under envelope | Deferred | Deferred | Mitigated by re-scoping to a hardened multi-slot design | **Viable fallback** |

> Latency (the fifth input) is now CAPTURED (2026-06-11): SPAN-A/B medians 0.553/0.569 ms,
> ~900× under the 500 ms fail-open envelope — **favorable**, and it does not change the
> verdict (latency was never the limiting factor). Weighed as one input, not a gate; no
> pass/fail latency threshold is invented (D-05).

### Go/No-Go Recommendation

**Evidence-derived recommendation (for Oscar's review — D-06, NOT locked):** lean
**No-go / Conditional-go** for a *near-term* production-driver milestone. Rationale: the
spike **proved feasibility** (FltMgr can enforce a kernel deny via a real policy
round-trip), but the existing **WFP + AppContainer/Low-IL** model already delivers
kernel-enforced isolation, so a production minifilter is an **incremental** gain bought
at **high recurring cost** (EV/WHQL cert + Partner Center), **high maintenance burden**
(kernel-version tracking), and a **strong fragility signal** (18 live defects on a
single-slot spike). A **Conditional-go** is the recommended posture: keep DRV-PROD-01
deferred (v2.11/v3.0) and revive it only if a concrete capability gap is identified that
AppContainer/WFP cannot express, with an assigned altitude and a hardened multi-slot
redesign.

> **D-06 human-review gate:** this recommendation is **surfaced, not locked**. The ADR
> ships `Status: Proposed`. Flip to `Accepted` only after Oscar's sign-off — and confirm
> the latency input (appendix) is populated first. Do NOT mark Accepted silently.

## Non-goals / Deferred

- **Production fail-open vs fail-closed:** the spike uses **fail-open** to protect the
  test VM (a denied create slips to allow on timeout/error). The production
  fail-direction is a **separate production ADR** concern — mentioned here, **not
  decided** (CONTEXT Deferred).
- **EDR integration (EDR-INTEG-01)** and **DRV-PROD-01** packaging/signing are
  out-of-scope for this feasibility verdict.

---

**File path note:** This ADR lives at
`.planning/architecture/adr-65-minifilter-go-no-go.md` per D-46-A2 precedent
(`.planning/architecture/` is the v2.6+ ADR location; `docs/architecture/` holds
Phase-32-and-earlier ADRs). This is the **D-07 deviation** from the SC2 literal
`.planning/adr/` shorthand — recorded here so the close gate does not flag a wrong-path
miss. Raw latency tables live in the sibling
[`adr-65-latency-appendix.md`](adr-65-latency-appendix.md) (D-08).
