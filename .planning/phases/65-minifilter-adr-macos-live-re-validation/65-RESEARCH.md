# Phase 65: Minifilter ADR + macOS Live Re-validation - Research

**Researched:** 2026-06-09
**Domain:** Windows minifilter (FltMgr) latency instrumentation + ADR authoring + macOS Seatbelt HUMAN-UAT sequencing + cross-target drift guard
**Confidence:** HIGH

## Summary

Phase 65 is a **document + measure + re-validate** phase over code that already landed (Phase 63/64). It has exactly **one net-new code element** — driver latency instrumentation in `drivers/nono-fltmgr/nono-fltmgr.c` — and three process gates: writing the go/no-go ADR, sequencing the macOS HUMAN-UAT, and enforcing the cross-target drift guard. No new sandbox capability is built.

The instrumentation is small and well-bounded: `KeQueryPerformanceCounter` is callable at **any IRQL** [CITED: learn.microsoft.com/wdm/nf-wdm-kequeryperformancecounter], so it works unchanged in both measurement sites — the PASSIVE_LEVEL worker thread (kernel-IPC span, D-02a) and the ≤APC_LEVEL pre-create callback (full pre-op→completion span, D-02b). The frequency is fixed at boot and cacheable; the conversion idiom is `ms = delta_ticks * 1000 / freq`. The hard constraint is **don't perturb the measured path**: accumulate samples in a fixed kernel array and dump once at unload (or emit via `DbgPrint` only on the *non-timed* allow/deny branch tail), never log per-event inside the QPC span.

The ADR follows the established `.planning/architecture/` convention (D-07) — the `adr-58-windows-hook-executor.md` shape is the closest precedent (Status/Date/Phase/Decision-IDs header → Context → Goals → Decision Table → Alternatives), and the `v2.6-upstream-merge-deferral-ADR.md` per-option scoring table is the shape for the go/no-go verdict. The core decision narrative stays concise; raw latency tables (per-span min/median/p99, iteration counts, VM context) go in a sibling appendix file (D-08).

The macOS work is "code-ready now, UAT-gated" (D-10): the automatable parts (macOS CI `test` job on `macos-latest` + `x86_64-apple-darwin` clippy) can go green now; the live `sandbox_init()` deny assertions are host-gated and block close (gate 65-A). The cross-target drift guard (D-11) is non-negotiable: the v2.9 regression (two cfg-gated compile errors reached release tags because the Windows host never compiles macOS code) is the reason a **green macOS CI run link/SHA must be captured as literal gate evidence** before any release tag.

**Primary recommendation:** Add a cached-frequency QPC counter-pair to the worker `FltSendMessage`/`FilterReplyMessage` span and to the pre-create→`FltCompletePendedPreOperation` span, accumulate ~100 samples per span in a non-paged array, dump median+p99 at unload; write the ADR to `.planning/architecture/adr-65-minifilter-go-no-go.md` (+ sibling `adr-65-latency-appendix.md`) in the adr-58 shape; stage the macOS deny assertions as a close-blocking HUMAN-UAT checklist; and capture a green `macos-latest` CI SHA as gate evidence per the cross-target checklist.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Latency instrumentation (QPC counter pairs) | Kernel driver (`nono-fltmgr.c`) | — | Both spans are measured in kernel code; the QPC counter is a HAL primitive, not user-mode |
| Sample accumulation + median/p99 dump | Kernel driver | User-mode client (optional sink) | In-kernel accumulation avoids perturbing the measured path; emitting can be deferred to the allow/deny tail or unload |
| Go/no-go decision record | Planning doc (`.planning/architecture/`) | — | ADR is a project-governance artifact, not runtime code |
| macOS Seatbelt profile correctness | Library (`crates/nono/src/sandbox/macos.rs`) | — | Already landed (8f84d454); Phase 65 re-validates, does not modify |
| macOS deny assertions (live) | HUMAN-UAT (real macOS host) | CI (`macos-latest` automatable subset) | `sandbox_init()` enforcement is host-gated; ordering/clippy is automatable |
| Cross-target drift guard | CI (`macos-latest`) + dev-host clippy | Verification artifact (SHA evidence) | The Windows host cannot compile macOS code; CI is the decisive signal |

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Latency measurement (DRV-04 / SC1):**
- **D-01:** Capture a **real instrumented number**, not just the design budget. The ~500ms `FltSendMessage` timeout is the fail-open envelope, NOT the measured latency — Phase 64 never instrumented an actual round-trip. Phase 65 adds timing and re-runs on the VM.
- **D-02:** Measure **both layered spans**:
  - **(a) Kernel-only IPC span** — `KeQueryPerformanceCounter` immediately before `FltSendMessage` to immediately after `FilterReplyMessage` returns in the worker thread (pure kernel→user→kernel cost, excludes ring-buffer enqueue).
  - **(b) Full pre-op→completion span** — from pre-create callback entry (enqueue) through IRP completion with `STATUS_ACCESS_DENIED` (user-perceived deny path including ring-buffer + worker wakeup + scheduling jitter).
- **D-03:** Report **median + p99 over ~100 denied creates** for each span.
- **D-04:** **Track A scope** — driver rebuild → re-sign → reload cycle on the spike VM per `64-SC1-VM-RUNBOOK.md`. VM (`nono-fltmgr-vm`, rg `rg-nono-fltmgr-spike`, IP `20.51.161.15`) expected still alive; planner makes VM provisioning **idempotent** (reuse if present, recreate from runbook if gone). Per T-63-05 the `.sys` stays VM-local (not committed).

**Go/no-go verdict (DRV-04 / SC1):**
- **D-05:** **Let the evidence decide — the verdict is NOT pre-committed.** The ADR must weigh: measured latency (D-01..D-03) + 18 spike defects + EV/WHQL cert cost + official Microsoft altitude assignment + ongoing kernel-version maintenance burden, AGAINST the security gap (if any) that WFP+AppContainer cannot close.
- **D-06:** The written analysis drives a **recommended** direction (go / no-go / conditional-go-gated), but the final recommendation is a **HUMAN-review gate**: operator (Oscar) reviews before final. Surface the recommendation for review, don't silently lock it.

**ADR location + structure (DRV-04 / SC2):**
- **D-07:** **Location follows repo convention, NOT the SC's literal path.** Write the ADR to `.planning/architecture/` (next to `adr-58-windows-hook-executor.md`, `v2.6-upstream-merge-deferral-ADR.md`), suggested name `adr-65-minifilter-go-no-go.md`. The SC2 phrase "committed to `.planning/adr/`" is **descriptive shorthand** — this path deviation MUST be noted in verification so the close gate doesn't flag a "wrong path" miss.
- **D-08:** **Structure = core decision ADR (concise) + linked latency-data appendix/evidence file.** Raw measurement tables live in a separate appendix/evidence file. The ADR **references** `drivers/nono-fltmgr/DESIGN.md` and the Phase-64 SC1 evidence — does NOT duplicate them.
- **D-09:** Single ADR covers all six DRV-04 topics as sections: interception design, measured latency (+ finite-timeout fail-open), `windows-drivers-rs`-not-viable, FltMgr-vs-ETW rationale, chosen altitude (365678) + official-assignment request status, explicit go/no-go recommendation.

**macOS UAT sequencing (MACOS-03 / SC2 + SC3):**
- **D-10:** **"Code-ready now, UAT-gated."** Split:
  - **Automatable now:** macOS CI build leg green + `x86_64-apple-darwin` clippy + `sandbox::macos` ordering tests.
  - **Live HUMAN-UAT (host-gated):** SC2 live `sandbox_init()` assertions — `nono run --dry-run --profile claude-code` emits deny-after-allow ordering; `nono run --profile claude-code -- cat ~/.ssh/id_rsa` is **blocked**; **both** `/etc/hosts` **and** `/private/etc/hosts` are blocked; `make test-lib` green on host — staged as a **HUMAN-UAT checklist that BLOCKS phase close** until run on a real macOS host (gate 65-A). No macOS host confirmed available.
- **D-11:** **CI macOS green is a HARD gate, evidenced — not advisory.** Phase 65 MUST: (a) run `cargo clippy --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` from the dev host per CLAUDE.md, (b) scan the cherry-pick checklist for edition-2024 let-chains / E0716-class borrows / canonical-path (`/private/etc`, `/private/tmp`) coverage, AND (c) **capture a green macOS CI run link/SHA in the verification artifact as literal gate evidence**. No release tag until that run is green.

### Claude's Discretion
- Exact ADR section ordering/headings, the appendix filename, the precise instrumentation code (counter placement, log format) — provided D-01..D-03 spans + rigor are honored.
- VM recreate mechanics if the VM is gone — follow `64-SC1-VM-RUNBOOK.md`.

### Deferred Ideas (OUT OF SCOPE)
- **Production EV/WHQL-signed driver + kernel-version-maintenance hardening** (DRV-PROD-01) — gated on this ADR's verdict; future milestone (v2.11/v3.0).
- **EDR/ETW structured telemetry emission** (EDR-INTEG-01) — Phase 66 only validates *under* EDR.
- **Non-macOS UPST8 cherry-pick clusters** (UPST8-NONMAC-01).
- **`NonoIpcRequest` version / request-id ABI-insurance fields**.
- **Fail-direction for a production driver** (fail-open vs fail-closed) — note it in the ADR, don't decide it.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DRV-04 | A go/no-go ADR documenting interception design, **measured `FLT_PREOP_PENDING` round-trip latency**, the `windows-drivers-rs`-not-viable decision, the FltMgr-vs-ETW rationale, chosen altitude (365678 + official-assignment status), and an explicit go/no-go recommendation | QPC instrumentation idiom (§ Code Examples) + counter-pair placement (§ Architecture Patterns) + ADR format (§ ADR Authoring) + the six-topic mapping (§ ADR Authoring) all enable authoring. Decision inputs already exist: 64-SC1-driver-evidence.md (18 defects, deny chain, altitude), DESIGN.md (STRIDE, fail-open) |
| MACOS-03 | The Seatbelt layer re-validated **live on a real macOS host** (`sandbox_init()` allow/deny + absorbed ordering fix) AND the **macOS CI build leg confirmed green before any release tag** (HARD gate). Cherry-pick checklist scans for edition-2024 let-chains / E0716-class borrows / canonical-path coverage | macOS CI job topology (§ Validation Architecture), exact live deny commands + expected outputs (§ macOS UAT Sequencing), cross-target checklist scan items + location (§ Cross-Target Drift Guard), automatable-vs-host-gated split (§ macOS UAT Sequencing) |
</phase_requirements>

## Standard Stack

This phase introduces **no new external packages**. All tooling already exists in the repo or on the spike VM. The "stack" is:

| Component | Where | Purpose | Already present |
|-----------|-------|---------|-----------------|
| `KeQueryPerformanceCounter` | `<wdm.h>` (kernel, HAL) | High-res (<1µs) monotonic timestamp for span measurement | Yes — kernel header already `#include`d via `<fltKernel.h>` |
| EWDK 26H1 toolchain | spike VM (`C:\ewdk\ewdk.iso`) | Build/rebuild the `.sys` | Yes — proven in Phase 64 |
| `New-SelfSignedCertificate` / `inf2cat` / `signtool /sm` / `fltmc load` | spike VM | Re-sign + reload pipeline | Yes — corrected EWDK-26H1 path in `64-SC1-driver-evidence.md` |
| `64-vm-runcmd-ewdk-build-local.ps1` | Phase 64 folder | Headless build invocation (run-command, no Bastion hang) | Yes |
| `az vm` (idempotent reuse/recreate) | dev host | VM lifecycle per runbook §2 / §10b | Yes — Azure CLI |
| `cargo clippy --target x86_64-apple-darwin` | dev host | Cross-target macOS lint (D-11a) | Toolchain may need `rustup target add x86_64-apple-darwin` |
| GitHub Actions `ci.yml` `test` + clippy jobs on `macos-latest` | CI | macOS build/test/lint green gate (D-11c) | Yes — see § Validation Architecture |

**Installation:** Nothing to `npm install`/`cargo add`. The only setup step is the one-time cross-target toolchain add (if not present):
```bash
rustup target add x86_64-apple-darwin    # for the dev-host clippy gate (D-11a)
```

> **No Package Legitimacy Audit section** — this phase installs no external packages. Skipped per protocol (no registry dependency introduced).

## Architecture Patterns

### Instrumentation Data Flow

```
[Pre-create callback @ ≤APC_LEVEL]
   t0_full = KeQueryPerformanceCounter()         <-- SPAN-B start (D-02b)
        |  enqueue to ring buffer
        v
   [Ring buffer]  --KeSetEvent-->  [Worker thread @ PASSIVE_LEVEL]
                                        |
                  t0_ipc = KeQueryPerformanceCounter()   <-- SPAN-A start (D-02a)
                                        |  FltSendMessage(timeout=-5000000LL)
                                        |     -> user-mode client -> FilterReplyMessage
                  t1_ipc = KeQueryPerformanceCounter()   <-- SPAN-A end (D-02a)
                                        |  (record SPAN-A = t1_ipc - t0_ipc)
                                        v
                          FltCompletePendedPreOperation(STATUS_ACCESS_DENIED)
   t1_full = KeQueryPerformanceCounter()         <-- SPAN-B end (D-02b)
        |  (record SPAN-B = t1_full - t0_full)
        v
   [accumulator array in non-paged global] -- at unload --> median + p99 dump
```

**Critical placement note (SPAN-B end):** the full-span end timestamp (`t1_full`) is taken *after* `FltCompletePendedPreOperation`, but that call happens **inside the worker thread**, not the pre-create callback (the callback already returned `FLT_PREOP_PENDING`). So SPAN-B must carry its `t0_full` start timestamp **through the ring buffer** (store it in `NONO_RING_ENTRY` alongside `pRequest`/`Data`), and the worker computes `SPAN-B = t1_full - t0_full` after completing the IRP. This is the cleanest way to capture the user-perceived deny path without a post-op callback.

### Pattern 1: Cached frequency, computed once

**What:** The performance-counter frequency is fixed at boot and identical across processors [CITED: learn.microsoft.com/wdm/nf-wdm-kequeryperformancecounter]. Query it once in `DriverEntry` and cache it in a global; never re-query it inside the measured span.

**When to use:** Always — re-querying frequency per-sample adds a HAL call inside (or near) the span and is wasteful.

```c
// Source idiom: KeQueryPerformanceCounter docs [CITED]
LARGE_INTEGER g_PerfFreq;           // cached at DriverEntry
// in DriverEntry, before FltStartFiltering:
(void)KeQueryPerformanceCounter(&g_PerfFreq);   // ticks/second, fixed at boot
```

### Pattern 2: In-kernel accumulation, deferred emission

**What:** Record raw tick deltas into a fixed-size non-paged array; do NOT `DbgPrint`/log inside the span. Compute median + p99 once (at unload, or on a `fltmc unload`-triggered dump), after sorting the sample array.

**When to use:** Always for D-03 rigor. Per-event `DbgPrint` inside the timed span would dominate the measurement (DbgPrint is slow and serializes), corrupting the very number being measured.

```c
#define NONO_SAMPLE_MAX 128
LONG64 g_SpanA[NONO_SAMPLE_MAX];   LONG g_SpanACount = 0;   // kernel-IPC span ticks
LONG64 g_SpanB[NONO_SAMPLE_MAX];   LONG g_SpanBCount = 0;   // full pre-op->completion ticks
// record (worker thread, no lock needed if single-slot serializes):
LONG idx = g_SpanACount;
if (idx < NONO_SAMPLE_MAX) { g_SpanA[idx] = deltaA.QuadPart; g_SpanACount = idx + 1; }
```

> Single-slot ring means only one round-trip is in flight at a time, so the accumulator is effectively serialized by the existing design — no extra lock is required for the spike. If the planner widens the ring, add `InterlockedIncrement` on the count.

### Pattern 3: Tick→milliseconds conversion (report-time, user mode or unload)

**What:** `ms = delta_ticks * 1000 / freq`. Use 64-bit integer math; multiply before divide to preserve sub-ms resolution. For p99 you sort the deltas, take the 99th-percentile index, then convert.

```c
// delta in ticks, g_PerfFreq.QuadPart = ticks/sec
double ms = (double)delta_ticks * 1000.0 / (double)g_PerfFreq.QuadPart;
// integer alternative (avoid FP in kernel): microseconds = delta_ticks * 1000000 / freq
```

> **Kernel FP note:** floating-point in kernel mode requires `KeSaveFloatingPointState`/`KeRestoreFloatingPointState` guards on some paths. Prefer **integer microsecond math** in the driver (`us = delta * 1000000 / freq`) and do the final ms/percentile presentation in the user-mode client or in the ADR appendix. This sidesteps the FP-context issue entirely.

### Anti-Patterns to Avoid
- **`DbgPrint` inside the QPC span:** dominates the measurement; emit only outside the timed region (deny/allow tail) or batch at unload.
- **Re-querying frequency per sample:** wasteful HAL call; cache once in `DriverEntry`.
- **Kernel floating-point without context save:** can corrupt FP state or fault. Use integer microsecond math in-driver; convert to ms/percentile at report time.
- **Measuring only one span:** D-02 requires BOTH; SPAN-A attributes IPC cost, SPAN-B captures user-perceived latency including scheduling jitter. Reporting only one defeats the attribution goal.
- **`FLT_FILE_NAME_NORMALIZED` in pre-create:** unrelated to timing but a live deadlock the driver already avoids (uses `FLT_FILE_NAME_OPENED`) — don't reintroduce when editing the callback.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| High-res kernel timestamp | A custom RDTSC reader or `KeQueryTickCount` (15ms granularity) | `KeQueryPerformanceCounter` | <1µs resolution, monotonic, any-IRQL, frequency consistent across processors [CITED] |
| Frequency lookup | Hardcoded TSC frequency constant | `KeQueryPerformanceCounter(&freq)` once at init | Frequency is platform-dependent; the API reports it accurately and it's fixed at boot |
| ADR scaffolding | A fresh ADR template | Copy the `adr-58-windows-hook-executor.md` header + section skeleton | Convention is already established in `.planning/architecture/`; matching it avoids a "wrong format" close-gate flag |
| VM lifecycle | New provisioning script | `64-SC1-VM-RUNBOOK.md` §2 (reuse) / §10b (recreate) | Idempotent path with all Phase 63 gotchas (Trusted-Launch, EWDK ISO, RDP-block) already captured |
| macOS deny assertions | New test harness | Existing `nono run` CLI + `make test-lib` | The deny assertions ARE the live UAT; the unit tests already encode the ordering contract |
| Cross-target verification | Ad-hoc clippy invocation | `.planning/templates/cross-target-verify-checklist.md` decision tree | Codifies the PARTIAL-disposition + CI-evidence rule that the v2.9 regression proved necessary |

**Key insight:** Every hard part of this phase already has a proven asset. The only genuinely new authoring is ~30 lines of QPC instrumentation + the ADR prose. The risk is process discipline (don't perturb the span; capture the CI SHA), not invention.

## Runtime State Inventory

> This is a measure/document/re-validate phase, not a rename/refactor. A full Runtime State Inventory does not apply. The one stateful asset is the **spike VM**, covered explicitly:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — the `.sys` is VM-local (T-63-05); no datastore keys involved | None — verified by `git ls-files drivers/nono-fltmgr/*.sys` (empty, per 64-VERIFICATION) |
| Live service config | Spike VM `nono-fltmgr-vm` (rg `rg-nono-fltmgr-spike`, IP `20.51.161.15`); `nono-fltmgr` minifilter service at altitude 365678; snapshots `nono-fltmgr-snap-testsigning-ready` + `nono-fltmgr-snap-loaded` | Idempotent reuse/recreate per runbook §2/§10b; rebuild+reload after instrumentation edit |
| OS-registered state | Minifilter service (`SERVICE_DEMAND_START`) + `Instances\…\Altitude=365678` registry on the VM | Re-created by `rundll32 ... DefaultInstall` + `fltmc load` on each reload |
| Secrets/env vars | Test cert `CN=NonoTestSign` (thumbprint `C40C9572077EDBCEFE7BE51779D29F4BC0C074A7`) in VM machine store | Reuse if present; recreate via `New-SelfSignedCertificate` if VM was rebuilt |
| Build artifacts | VM-local `nono-fltmgr.sys`/`.cat`/`.obj` (never committed) | Rebuild on the VM after the instrumentation edit; stays VM-local |

## Common Pitfalls

### Pitfall 1: Measurement perturbs the measured path
**What goes wrong:** Logging each sample (`DbgPrint`) inside the QPC span makes the measured latency reflect logging cost, not IPC cost.
**Why it happens:** Natural instinct to "print as you go."
**How to avoid:** Accumulate raw tick deltas in a non-paged array; sort + compute median/p99 once at unload (or emit on the non-timed deny tail). Integer microsecond math in-kernel; ms/percentile presentation at report time.
**Warning signs:** SPAN-A ≈ SPAN-B (logging dominates both), or implausibly large medians (>10ms for a localhost round-trip).

### Pitfall 2: SPAN-B start lost across the ring buffer
**What goes wrong:** The full-span start is in the pre-create callback, but the end is in the worker thread — if `t0_full` isn't carried through the ring entry, SPAN-B can't be computed.
**Why it happens:** The callback returns `FLT_PREOP_PENDING` and exits before completion.
**How to avoid:** Add a `LARGE_INTEGER EnqueueQpc` field to `NONO_RING_ENTRY` (NOT to `NONO_IPC_REQUEST` — that struct has a `C_ASSERT(sizeof == 532)` ABI lock and a Rust mirror; changing it breaks the wire contract). The ring entry is kernel-internal and free to extend.
**Warning signs:** SPAN-B equals SPAN-A (the enqueue+wakeup delta got dropped).

### Pitfall 3: Touching the ABI-locked IPC struct
**What goes wrong:** Adding a timing field to `NONO_IPC_REQUEST` breaks `C_ASSERT(sizeof(NONO_IPC_REQUEST) == 532)` and the Rust-side layout assertion (`size_of - FILTER_MESSAGE_HEADER == 532`), causing reply-delivery failure (defect #13 class: size mismatch → timeout fail-open).
**Why it happens:** Conflating "I need to pass a timestamp around" with "I need to send it to user mode." The timing is kernel-internal; it never crosses the port.
**How to avoid:** Keep all timing state in kernel-only globals + the `NONO_RING_ENTRY`. The wire struct is untouched.
**Warning signs:** `[DENY ]` logged but the file opens anyway (the exact symptom of defect #13).

### Pitfall 4: macOS UAT marked "done" on Windows-host evidence
**What goes wrong:** Flipping MACOS-03 to VERIFIED based on `cargo check` from the Windows host — the exact v2.9 failure that shipped two cfg-gated compile errors to release tags.
**Why it happens:** The Windows host never compiles macOS code; `cargo check` doesn't run clippy.
**How to avoid:** Follow the cross-target checklist decision tree. If `x86_64-apple-darwin` clippy can't run on the dev host (ring/cc-rs link), mark PARTIAL and require the green `macos-latest` CI SHA as the decisive signal (D-11c).
**Warning signs:** Verification cites only Windows-host commands; no CI run link/SHA captured.

### Pitfall 5: ADR written to the SC's literal path
**What goes wrong:** Writing to `.planning/adr/` (the SC2 shorthand) instead of `.planning/architecture/` (repo convention, D-07), then the close gate flags a "wrong path" miss.
**How to avoid:** Write to `.planning/architecture/adr-65-minifilter-go-no-go.md`; explicitly note the deviation-from-SC-literal-path in the verification artifact.

## Code Examples

### Worker-thread SPAN-A instrumentation (kernel-IPC, D-02a)
```c
// In NonoWorkerThread, around the existing FltSendMessage call.
// g_PerfFreq cached in DriverEntry. Integer microsecond math (no kernel FP).
LARGE_INTEGER a0 = KeQueryPerformanceCounter(NULL);   // SPAN-A start
NTSTATUS sendStatus = FltSendMessage(
    gFilterHandle, &gClientPort, pRequest, sizeof(NONO_IPC_REQUEST),
    &reply, &replyLen, &timeout);
LARGE_INTEGER a1 = KeQueryPerformanceCounter(NULL);   // SPAN-A end

LONG64 us_A = (a1.QuadPart - a0.QuadPart) * 1000000LL / g_PerfFreq.QuadPart;
// record us_A into g_SpanA[] only on the timed (non-timeout) path
```

### SPAN-B carried through the ring entry (full pre-op→completion, D-02b)
```c
// NONO_RING_ENTRY (kernel-internal — safe to extend; NOT the wire struct):
//   + LARGE_INTEGER EnqueueQpc;
// In NonoPreCreate, just before enqueue:
g_RingEntry.EnqueueQpc = KeQueryPerformanceCounter(NULL);   // SPAN-B start
// In NonoWorkerThread, AFTER FltCompletePendedPreOperation(... STATUS_ACCESS_DENIED):
LARGE_INTEGER b1 = KeQueryPerformanceCounter(NULL);         // SPAN-B end
LONG64 us_B = (b1.QuadPart - enqueueQpc.QuadPart) * 1000000LL / g_PerfFreq.QuadPart;
// record us_B into g_SpanB[]
```

### macOS live deny assertions (HUMAN-UAT, run on a real macOS host)
```bash
# 1. Ordering visible in dry-run (deny-after-allow)
nono run --dry-run --profile claude-code        # expect: file-write* allows, THEN platform (deny ...) lines

# 2. SSH key read blocked
nono run --profile claude-code -- cat ~/.ssh/id_rsa
# expect: non-zero exit; "Operation not permitted" / sandbox deny (NOT the key contents)

# 3. BOTH /etc/hosts and the /private/etc canonical form blocked
nono run --profile claude-code -- cat /etc/hosts            # expect: blocked
nono run --profile claude-code -- cat /private/etc/hosts    # expect: blocked (dual-path coverage)

# 4. Library tests green on the host
make test-lib    # cargo test -p nono — expect: all sandbox::macos tests pass
```

### Cross-target clippy gate (dev host, D-11a)
```bash
rustup target add x86_64-apple-darwin   # one-time
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
# clean exit -> may flip; toolchain/link error -> PARTIAL + require green macos-latest CI SHA
```

## ADR Authoring (DRV-04 / SC2)

### Location + filename (D-07)
- **Write to:** `.planning/architecture/adr-65-minifilter-go-no-go.md`
- **Sibling appendix (D-08):** `.planning/architecture/adr-65-latency-appendix.md` (raw per-span min/median/p99, iteration counts, VM context, QPC frequency).
- **Convention source:** `adr-58-windows-hook-executor.md` (newest ADR, closest shape). The older `docs/architecture/*` ADRs use no `-adr` suffix and a different dir — `.planning/architecture/` is the v2.6+ location per D-46-A2.

### Header block (copy adr-58 shape)
```markdown
# Minifilter Spike Go/No-Go (Gap 6b FltMgr feasibility verdict)

**Status:** Accepted    (or Proposed until D-06 human review)
**Date:** 2026-06-XX
**Phase:** 65 (minifilter-adr-macos-live-re-validation)
**Decision IDs:** D-01..D-09 (Phase 65 CONTEXT)
**Related:** drivers/nono-fltmgr/DESIGN.md (BSOD-avoidance gate),
             64-SC1-driver-evidence.md (SC1 PASS evidence),
             .planning/architecture/adr-65-latency-appendix.md (latency data)
```

### Section skeleton — the six DRV-04 topics as sections (D-09)
The adr-58 body shape is: **Context → Goals → Decision Table → (Trust Boundary / Invariants) → Alternatives Considered**. Map the six topics onto:

| # | DRV-04 topic | ADR section | Source (reference, don't duplicate) |
|---|--------------|-------------|--------------------------------------|
| 1 | Interception design | Context + a "How the spike intercepts" section | `DESIGN.md` IPC ring-buffer+worker; `nono-fltmgr.c` deny chain |
| 2 | Measured latency + finite-timeout fail-open | A "Measured Latency" section → links to appendix | This phase's instrumentation run; T-63-02 fail-open envelope |
| 3 | `windows-drivers-rs`-not-viable | Decision Table row / Alternatives | 64-CONTEXT.md (early-stage/KMDF-v1.33-only); REQUIREMENTS.md Out-of-Scope row |
| 4 | FltMgr-vs-ETW rationale | Decision Table row / Alternatives | FltMgr enforces (deny) vs ETW observes-only; EDR-INTEG-01 deferral |
| 5 | Chosen altitude 365678 + official-assignment status | A short "Altitude" section | 64-SC1-driver-evidence.md `fltmc filters` non-collision; fsfcomm@microsoft.com request status (PENDING) |
| 6 | Go/no-go recommendation | A "Decision" + "Recommendation (human-review gate)" section | D-05 evidence weighing; D-06 surfaces, doesn't lock |

### Decision-Table shape for the verdict (use v2.6-ADR scoring style)
The `v2.6-upstream-merge-deferral-ADR.md` per-option scoring table is the precedent for a multi-criteria go/no-go. Columns to weigh per D-05:

| Option | Security gap closed vs WFP+AppContainer | Cert cost (EV/WHQL) | Maintenance burden (kernel-version) | Spike-defect signal | Verdict |
|--------|------------------------------------------|---------------------|--------------------------------------|---------------------|---------|
| Go (production driver) | … | … | … | … | … |
| No-go (WFP+AppContainer suffices) | … | … | … | … | … |
| Conditional-go (gated) | … | … | … | … | … |

> **D-06:** Fill the verdict from evidence, then surface for Oscar's review — mark `Status: Proposed` until human sign-off, flip to `Accepted` after. Do NOT silently lock the recommendation.

## macOS UAT Sequencing (MACOS-03 / SC2 + SC3)

There is **no precedent file named `*HUMAN-UAT*.md`** in the repo — prior Windows HUMAN-UAT (Phases 52, 56, 58, 60) embedded the live-UAT checklist inside the plan/verification artifacts and recorded results in the VERIFICATION `## Human-verification` section (see 64-VERIFICATION lines 41-48). Phase 65 should mirror that: a close-blocking checklist whose items map to the SC2 assertions, with a `pass/blocked` field per item and a host/date stamp. The 64-SC1 evidence-template shape (raw-output paste blocks + a final PASS/FAIL line) is the proven format.

### Automatable NOW (no macOS host)
| Item | Command | Where it runs |
|------|---------|---------------|
| macOS build leg | `cargo build --workspace` | CI `test` job, `macos-latest` (ci.yml:89-125) |
| macOS test leg | `cargo test --workspace` (incl. `sandbox::macos` ordering tests) | CI `test` job, `macos-latest` |
| `x86_64-apple-darwin` clippy | `cargo clippy --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | CI clippy job (ci.yml:408-412, matrix incl. `macos-latest`) + dev host (D-11a) |
| Ordering assertions | `test_generate_profile_platform_rules_after_writes`, `test_generate_profile_platform_rules_after_writes`-class (read<write<deny) | Already in `macos.rs` tests; run by the macOS `test` job |

### Live HUMAN-UAT, host-gated (BLOCKS close — gate 65-A)
The four assertions in § Code Examples (dry-run ordering, `~/.ssh/id_rsa` blocked, `/etc/hosts` + `/private/etc/hosts` both blocked, `make test-lib` green). Expected outputs are documented inline there. No macOS host is confirmed available at discuss time — the checklist stays OPEN until run.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Latency = design budget (500ms timeout) | Real instrumented median+p99 over ~100 creates | Phase 65 (D-01) | The ADR cites a measured number, not the fail-open envelope |
| `KeQueryTickCount` / RDTSC | `KeQueryPerformanceCounter` | Standard since Win2000 | <1µs resolution, any-IRQL, processor-consistent [CITED] |
| Windows-host `cargo check` trusted for macOS | `macos-latest` CI SHA as decisive gate evidence | v2.9 regression → D-11 | Two cfg-gated compile errors reached tags; CI evidence now mandatory |

**Deprecated/outdated:**
- `ExAllocatePoolWithTag(NonpagedPool, ...)` — superseded by `ExAllocatePool2(POOL_FLAG_NON_PAGED, ...)` (already used in the driver). No new allocations needed for timing (use stack-local `LARGE_INTEGER` + static global arrays).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Integer microsecond math in-kernel avoids the FP-context-save requirement; presentation math deferred to user mode/ADR | Architecture Pattern 3, Code Examples | If the planner does FP in-driver without `KeSaveFloatingPointState`, possible FP-state corruption — mitigation already given (integer math) |
| A2 | The single-slot ring serializes accumulation so no `InterlockedIncrement` is needed for the sample arrays | Architecture Pattern 2 | If the ring is widened to multi-slot, concurrent writers need interlocked counts; flagged inline |
| A3 | Median round-trip latency will be sub-millisecond to low-single-digit-ms (localhost kernel→user→kernel) | Summary | This is an expectation, not a measured fact — the whole point of D-01 is to replace it. The ADR must report the actual number |
| A4 | No `*HUMAN-UAT*.md` precedent file exists; prior UAT lived in plan/verification artifacts | macOS UAT Sequencing | Low — Glob over `.planning/phases` confirmed the pattern; if a dedicated file is desired, the 64-SC1 evidence-template shape is the model |

## Open Questions

1. **Is the spike VM still alive?**
   - What we know: It existed at Phase 64 close (`nono-fltmgr-vm`, IP `20.51.161.15`); snapshots exist.
   - What's unclear: Whether it's still running / not deallocated at Phase 65 execution time.
   - Recommendation: First plan task probes `az vm get-instance-view` (runbook §2); reuse if `VM running`, `az vm start` if deallocated, recreate via §10b if gone. Idempotent per D-04.

2. **Can `x86_64-apple-darwin` clippy run on this Windows dev host?**
   - What we know: `keyring`/`ring`/`aws-lc-sys` are C-linking crates; cross-clippy may fail to link (checklist § note).
   - What's unclear: Whether the macOS target links cleanly from this host.
   - Recommendation: Attempt it; if it errors, mark MACOS-03 PARTIAL and lean on the green `macos-latest` CI SHA as the decisive signal (D-11c) — the checklist explicitly authorizes this.

3. **What latency threshold makes the verdict "go" vs "no-go"?**
   - What we know: D-05 says evidence decides; there's no pre-committed number.
   - What's unclear: There is no fixed pass/fail latency bar — it's weighed against the WFP+AppContainer security gap.
   - Recommendation: The ADR presents the measured number as ONE input among five (latency, 18 defects, cert cost, altitude assignment, maintenance burden); the human-review gate (D-06) decides. Do not invent a threshold.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Azure CLI (`az`) | VM reuse/recreate | ✓ (dev host, Phase 64 used it) | — | None — required for Track A |
| Spike VM `nono-fltmgr-vm` | Driver rebuild/reload + latency run | ? (probe at run) | Win11 26200 | Recreate via runbook §10b |
| EWDK 26H1 ISO on VM | `.sys` rebuild | ✓ (VM-local, Phase 64) | 26H1 | Re-download via `63-vm-runcmd-ewdk-download.ps1` |
| `x86_64-apple-darwin` Rust target | D-11a dev-host clippy | ? (`rustup target add` may be needed) | — | Green `macos-latest` CI SHA (D-11c) |
| GitHub Actions `macos-latest` runner | D-11c CI gate | ✓ (ci.yml `test` + clippy jobs) | macos-latest | None — this IS the decisive gate |
| Real macOS host | Live `sandbox_init()` deny assertions (gate 65-A) | ✗ (none confirmed) | — | None — gate 65-A stays OPEN/blocking until a host is found |

**Missing dependencies with no fallback:**
- **Real macOS host** for the live deny assertions — there is no automatable substitute for `sandbox_init()` enforcement (CI runs tests but does NOT execute the `nono run -- cat ~/.ssh/id_rsa` deny path as a live assertion). Gate 65-A blocks phase close until a host runs the four assertions.

**Missing dependencies with fallback:**
- `x86_64-apple-darwin` clippy from the Windows host → fallback is the green `macos-latest` CI SHA (explicitly authorized by the cross-target checklist PARTIAL disposition).
- Spike VM (if deallocated/gone) → fallback is the idempotent recreate path in runbook §10b.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) for `sandbox::macos`; live CLI assertions for the deny UAT |
| Config file | none (workspace `Cargo.toml`) |
| Quick run command | `cargo test -p nono sandbox::macos` |
| Full suite command | `make test-lib` (`cargo test -p nono`) on a macOS host; `cargo test --workspace` in CI `macos-latest` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DRV-04 | Latency instrumented + ADR authored | manual (driver re-run + doc) | VM deny harness re-run (runbook §9) + ADR review | ❌ (instrumentation is new code; harness exists) |
| MACOS-03 (ordering) | deny-after-allow emission order (read<write<deny) | unit | `cargo test -p nono sandbox::macos::tests::test_generate_profile_platform_rules_after_writes` | ✅ `macos.rs` |
| MACOS-03 (CI gate) | macOS build+test+clippy green | CI | `macos-latest` `test` + clippy jobs in ci.yml | ✅ (capture SHA) |
| MACOS-03 (live deny) | `sandbox_init()` blocks SSH key + `/etc/hosts` + `/private/etc/hosts` | manual HUMAN-UAT | the four § Code Examples assertions | ✅ (CLI exists; host-gated) |

### Sampling Rate
- **Per task commit:** `cargo test -p nono` (the macOS-gated tests compile-skip on Windows but the contract is in-tree)
- **Per wave merge:** green `macos-latest` CI run
- **Phase gate:** green `macos-latest` CI SHA captured as evidence (D-11c) + gate-65-A HUMAN-UAT checklist all-pass before close; no release tag before the green SHA

### Wave 0 Gaps
- None for the macOS ordering tests — they already exist in `macos.rs` (`test_generate_profile_platform_rules_after_writes` and siblings).
- Net-new: the driver instrumentation code (not a test gap — it's the DRV-04 deliverable) and the latency-appendix file.

## Security Domain

> `security_enforcement` is enabled (absent = enabled). This is a security-critical codebase (CLAUDE.md). The driver instrumentation must not weaken the BSOD-avoidance gate; the macOS re-validation IS a security gate.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | n/a for this phase |
| V3 Session Management | no | n/a |
| V4 Access Control | yes | The whole point — minifilter deny + Seatbelt deny correctness re-validated |
| V5 Input Validation | yes | Driver path copy already bounded (T-63-04, 259 WCHAR clamp); Seatbelt path escaping already enforced (`escape_path` rejects control chars) — instrumentation adds no new input surface |
| V6 Cryptography | no | Test-sign cert is throwaway VM-local; production EV/WHQL is deferred (DRV-PROD-01) |

### Known Threat Patterns for {kernel minifilter + Seatbelt}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Recursive file I/O in callback → stack-overflow BSOD | DoS | No `Zw/Nt` file APIs (T-63-01) — instrumentation adds none; QPC is HAL-only |
| Infinite `FltSendMessage` → host hang | DoS | Finite 500ms timeout (T-63-02) preserved; timing wraps the call, doesn't change it |
| IRQL violation in callback | DoS | `KeQueryPerformanceCounter` is **any-IRQL** [CITED]; safe in both the ≤APC_LEVEL callback and PASSIVE_LEVEL worker. No new allocation/lock added |
| Altitude collision with EDR | Tampering/EoP | 365678 (Activity-Monitor band) proven non-colliding; unchanged by Phase 65 |
| ABI drift on the IPC wire struct | Tampering | `C_ASSERT(sizeof==532)` + Rust mirror — instrumentation stays kernel-internal (ring entry), never touches the wire struct |
| macOS deny-ordering regression (deny before write-allow) | EoP (sandbox escape) | Last-match-wins ordering (8f84d454) asserted by unit tests; live UAT confirms enforcement |
| Cross-target compile drift to release tag | Tampering (ships broken security code) | Cross-target checklist + green `macos-latest` CI SHA gate (D-11) |

## Sources

### Primary (HIGH confidence)
- `KeQueryPerformanceCounter` function (wdm.h) — learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-kequeryperformancecounter — signature, any-IRQL, frequency cacheable/fixed-at-boot, <1µs resolution, monotonic [CITED]
- `drivers/nono-fltmgr/nono-fltmgr.c` — the worker `FltSendMessage`/`FilterReplyMessage` span + pre-create enqueue (instrumentation sites) [VERIFIED: codebase]
- `drivers/nono-fltmgr/nono-fltmgr.h` — `NONO_IPC_REQUEST` `C_ASSERT(sizeof==532)` ABI lock + Rust mirror note [VERIFIED: codebase]
- `drivers/nono-fltmgr/DESIGN.md` — STRIDE T-63-01..05, ring-buffer+worker pattern, fail-open rationale [VERIFIED: codebase]
- `.planning/architecture/adr-58-windows-hook-executor.md` — ADR header + section convention [VERIFIED: codebase]
- `.planning/architecture/v2.6-upstream-merge-deferral-ADR.md` — per-option scoring-table convention; `.planning/architecture/` location rule (D-46-A2) [VERIFIED: codebase]
- `crates/nono/src/sandbox/macos.rs` — deny-after-allow ordering (last-match-wins) + `/private/etc`/`/private/tmp` dual-path + ordering unit tests [VERIFIED: codebase]
- `.planning/templates/cross-target-verify-checklist.md` — decision tree, PARTIAL disposition, CI-evidence rule, scan items [VERIFIED: codebase]
- `.github/workflows/ci.yml` — `test` job on `macos-latest` (lines 89-125), clippy matrix incl. `macos-latest` (408-412) [VERIFIED: codebase]
- `64-SC1-driver-evidence.md` / `64-SC1-VM-RUNBOOK.md` — altitude 365678 non-collision, 18 defects, deny chain, idempotent VM lifecycle [VERIFIED: codebase]
- `.planning/REQUIREMENTS.md` — DRV-04 / MACOS-03 acceptance text, DRV-PROD-01 deferral, Out-of-Scope rows [VERIFIED: codebase]

### Secondary (MEDIUM confidence)
- 64-VERIFICATION.md `## Human-verification` section — precedent that prior UAT/cross-target items were recorded as close-gated follow-ups (the HUMAN-UAT structure to mirror) [VERIFIED: codebase]

### Tertiary (LOW confidence)
- None — all claims are codebase-verified or cited from official Microsoft docs.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; QPC idiom cited from official docs; all tooling proven in Phase 64.
- Architecture (instrumentation placement): HIGH — driver code read in full; QPC any-IRQL property confirms both sites are valid; ABI-lock constraint identified.
- ADR format: HIGH — both precedent ADRs read in full; convention is explicit (D-46-A2).
- macOS UAT sequencing: HIGH — CI topology read directly; deny assertions taken verbatim from CONTEXT D-10/specifics; HUMAN-UAT precedent pattern confirmed via Glob.
- Pitfalls: HIGH — derived from the 18 documented Phase-64 defects + the v2.9 cross-target regression + driver-read constraints.

**Research date:** 2026-06-09
**Valid until:** 2026-07-09 (stable — kernel API + repo conventions are slow-moving; the spike VM state is the only volatile input, probed at execution time)
