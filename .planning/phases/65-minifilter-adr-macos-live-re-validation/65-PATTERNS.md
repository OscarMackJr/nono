# Phase 65: Minifilter ADR + macOS Live Re-validation - Pattern Map

**Mapped:** 2026-06-09
**Files analyzed:** 5 (2 driver MODIFY, 2 ADR CREATE, 1 macOS READ-ONLY re-validation target)
**Analogs found:** 5 / 5

> **Phase shape:** document + measure + re-validate over already-landed code. Net-new code is ~30 lines of QPC instrumentation in the driver; the rest is ADR prose + a re-validation of existing macOS behavior. Every analog is in-tree and read in full below.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `drivers/nono-fltmgr/nono-fltmgr.c` (MODIFY) | kernel driver (minifilter) | event-driven (IRP pre-op → ring → worker IPC) | itself — existing `NonoWorkerThread` span + `NonoPreCreate` enqueue + `NONO_RING_ENTRY` struct | exact (in-file extension) |
| `drivers/nono-fltmgr/nono-fltmgr.h` (likely NO CHANGE) | C header (ABI wire contract) | request-response (kernel↔user wire struct) | itself — `NONO_IPC_REQUEST` `C_ASSERT(sizeof==532)` | exact — **the struct that must NOT change** |
| `.planning/architecture/adr-65-minifilter-go-no-go.md` (CREATE) | planning doc (ADR) | n/a (governance artifact) | `adr-58-windows-hook-executor.md` (header+section skeleton) + `v2.6-upstream-merge-deferral-ADR.md` (scoring-table verdict) | exact (convention precedent) |
| `.planning/architecture/adr-65-latency-appendix.md` (CREATE) | planning doc (data appendix) | n/a (raw data tables) | no direct analog — derive simple data-table shape; sibling-of the ADR above | role-match |
| `crates/nono/src/sandbox/macos.rs` (READ-ONLY) | library (Seatbelt profile gen) | transform (CapabilitySet → profile string) | itself — existing ordering + dual-path tests (re-validation target, no edit) | exact (target, not modified) |

## Pattern Assignments

### `drivers/nono-fltmgr/nono-fltmgr.c` (kernel driver, event-driven) — MODIFY

**Analog:** itself. The instrumentation extends three existing sites; no architectural change. All line numbers below are current as of the Phase-64 file.

#### CRITICAL constraint — where timing state lives

`NONO_RING_ENTRY` is **defined inline in the `.c` file** (lines 44–55), NOT in the header. It is kernel-internal and free to extend. Add the SPAN-B start timestamp here:

```c
// nono-fltmgr.c lines 44-55 (current struct — add EnqueueQpc field):
typedef struct _NONO_RING_ENTRY {
    PNONO_IPC_REQUEST pRequest;   // heap payload (ExAllocatePool2)
    PFLT_CALLBACK_DATA Data;      // pended IRP
    BOOLEAN Occupied;             // slot busy flag
    // + LARGE_INTEGER EnqueueQpc;   // SPAN-B start (D-02b) — kernel-internal, safe to add
} NONO_RING_ENTRY, *PNONO_RING_ENTRY;
```

**DO NOT** add timing fields to `NONO_IPC_REQUEST` (the header struct) — that struct carries `C_ASSERT(sizeof(NONO_IPC_REQUEST) == 532)` and a Rust mirror (`crates/nono-fltmgr-client/src/lib.rs`). Changing it is the defect-#13 class (size mismatch → reply-delivery fail → timeout fail-open). See the `nono-fltmgr.h` assignment below.

**Globals pattern** (where to declare cached frequency + accumulators) — mirror the existing globals block at lines 33–70 (filter handle, port handles, ring entry, lock, event). Add alongside:

```c
// new globals (mirror the lines 33-70 style: file-scope, zero/NULL-init):
LARGE_INTEGER g_PerfFreq;                                  // cached in DriverEntry
#define NONO_SAMPLE_MAX 128
LONG64 g_SpanA[NONO_SAMPLE_MAX]; LONG g_SpanACount = 0;    // kernel-IPC span ticks (D-02a)
LONG64 g_SpanB[NONO_SAMPLE_MAX]; LONG g_SpanBCount = 0;    // full pre-op->completion ticks (D-02b)
```

**SPAN-B start** — in `NonoPreCreate`, alongside the existing enqueue at lines 260–266 (between populating the ring slot and `KeReleaseSpinLock`):

```c
// nono-fltmgr.c lines 260-266 (existing enqueue — capture SPAN-B start here):
g_RingEntry.pRequest  = pReq;
g_RingEntry.Data      = Data;
g_RingEntry.Occupied  = TRUE;
// + g_RingEntry.EnqueueQpc = KeQueryPerformanceCounter(NULL);   // SPAN-B start (D-02b)
KeReleaseSpinLock(&g_RingLock, oldIrql);
```

> Note the **dequeue copy site** at lines 319–324 already snapshots `pRequest`/`Data` out of the slot before release — extend that copy to also lift `EnqueueQpc` into a local so SPAN-B end can compute against it after the slot is cleared.

**SPAN-A start/end** — wrap the existing `FltSendMessage` call at lines 341–348 in `NonoWorkerThread`. This is the kernel→user→kernel round-trip (D-02a):

```c
// nono-fltmgr.c lines 341-348 (existing FltSendMessage — bracket with QPC):
// + LARGE_INTEGER a0 = KeQueryPerformanceCounter(NULL);   // SPAN-A start
NTSTATUS sendStatus = FltSendMessage(
    gFilterHandle, &gClientPort, pRequest, sizeof(NONO_IPC_REQUEST),
    &reply, &replyLen, &timeout);
// + LARGE_INTEGER a1 = KeQueryPerformanceCounter(NULL);   // SPAN-A end
// + record (a1-a0) into g_SpanA[] ONLY on the non-timeout path (sendStatus != STATUS_TIMEOUT)
```

**SPAN-B end** — after `FltCompletePendedPreOperation` in the deny branch at lines 368–374. The worker (not the callback) owns this site, which is exactly why SPAN-B's start had to ride the ring entry:

```c
// nono-fltmgr.c lines 368-374 (existing deny completion — capture SPAN-B end after):
if (deny) {
    pendingData->IoStatus.Status      = STATUS_ACCESS_DENIED;
    pendingData->IoStatus.Information  = 0;
    FltCompletePendedPreOperation(pendingData, FLT_PREOP_COMPLETE, NULL);
    // + LARGE_INTEGER b1 = KeQueryPerformanceCounter(NULL);   // SPAN-B end (D-02b)
    // + record (b1 - localEnqueueQpc) into g_SpanB[]  -- localEnqueueQpc lifted at dequeue (lines 319-324)
}
```

**Frequency cache** — in `DriverEntry`, alongside the ring-primitive init at lines 580–582 (`KeInitializeEvent` / `KeInitializeSpinLock` / `RtlZeroMemory`), BEFORE `FltStartFiltering` (line 650):

```c
// nono-fltmgr.c near lines 580-582 (init block — add cached-frequency query):
KeInitializeEvent(&g_RingBufferEvent, SynchronizationEvent, FALSE);
KeInitializeSpinLock(&g_RingLock);
RtlZeroMemory(&g_RingEntry, sizeof(g_RingEntry));
// + (void)KeQueryPerformanceCounter(&g_PerfFreq);   // ticks/sec, fixed at boot — cache once
```

**Sample dump** — at unload, in `NonoFltUnload` (lines 500–548), AFTER the worker is joined (lines 506–527) so no concurrent writer remains, BEFORE `FltUnregisterFilter` (line 542). Sort `g_SpanA`/`g_SpanB`, compute median + p99 over `g_Span*Count` samples, integer-microsecond math (`us = delta * 1000000 / g_PerfFreq.QuadPart`). Emit via `DbgPrint` here (outside any timed span — safe).

#### Constraints to preserve (DESIGN.md BSOD-avoidance gate, from this file's header lines 14–24)
- **T-63-01:** No `Zw*`/`Nt*` file APIs. `KeQueryPerformanceCounter` is HAL-only — adds none.
- **T-63-02:** `FltSendMessage` keeps `timeout.QuadPart = -5000000LL` (500ms). QPC wraps the call; does not change it.
- **T-63-03:** `KeQueryPerformanceCounter` is **any-IRQL** — valid in both the ≤APC_LEVEL callback and the PASSIVE_LEVEL worker. No new allocation/lock.
- **No kernel FP:** use integer microsecond math in-driver; defer ms/percentile presentation to the ADR appendix (CLAUDE.md panics/security aside; FP-context save not needed if avoided).
- **No `DbgPrint` inside the QPC span** (Pitfall 1) — accumulate raw ticks, dump at unload only.
- Single-slot ring serializes accumulation → no `InterlockedIncrement` needed (Assumption A2). If the ring is widened, add it.

---

### `drivers/nono-fltmgr/nono-fltmgr.h` (C header, ABI wire contract) — LIKELY NO CHANGE

**Analog:** itself. This file is the **negative pattern** — the struct the instrumentation must NOT touch.

```c
// nono-fltmgr.h lines 34-60 — THE ABI-LOCKED WIRE STRUCT (do not extend for timing):
#pragma pack(push, 1)
typedef struct _NONO_IPC_REQUEST {
    WCHAR PathBuffer[260];       // 520 bytes
    ULONG ProcessId;             // 4 bytes
    ACCESS_MASK DesiredAccess;   // 4 bytes
    ULONG Reserved;              // 4 bytes
} NONO_IPC_REQUEST, *PNONO_IPC_REQUEST;
#pragma pack(pop)
// The Rust mirror is crates/nono-fltmgr-client/src/lib.rs NonoIpcRequest (header lines 9-11).
C_ASSERT(sizeof(NONO_IPC_REQUEST) == 532);   // line 60 — breaks if you add a timing field
```

**Planner action:** the `<action>` for the `.h` file should be a **no-op assertion** ("confirm `NONO_IPC_REQUEST` is unchanged; timing rides `NONO_RING_ENTRY` in the `.c` file"). The only header edit that could ever be justified is moving `NONO_RING_ENTRY` out of the `.c` — NOT recommended for the spike (it currently lives at `.c` lines 44–55 and stays there).

---

### `.planning/architecture/adr-65-minifilter-go-no-go.md` (ADR) — CREATE

**Analog:** `.planning/architecture/adr-58-windows-hook-executor.md` (header + section skeleton) and `.planning/architecture/v2.6-upstream-merge-deferral-ADR.md` (per-option scoring table for the verdict).

**Location (D-07):** `.planning/architecture/` — NOT the SC2 literal `.planning/adr/`. The path deviation MUST be noted in verification (Pitfall 5). Both precedent ADRs carry an explicit `**File path note:**` footer documenting their `.planning/architecture/` location per D-46-A2 — copy that footer idiom.

**Header block** (copy adr-58 lines 1–7 shape exactly):

```markdown
# Minifilter Spike Go/No-Go (Gap 6b FltMgr feasibility verdict)

**Status:** Proposed    <!-- D-06: Proposed until Oscar's human review, then flip to Accepted -->
**Date:** 2026-06-XX
**Phase:** 65 (minifilter-adr-macos-live-re-validation)
**Decision IDs:** D-01..D-09 (Phase 65 CONTEXT)
**Related:** drivers/nono-fltmgr/DESIGN.md, 64-SC1-driver-evidence.md,
             .planning/architecture/adr-65-latency-appendix.md
```

> **D-06 gate:** start at `Status: Proposed` (adr-58 line 3 uses `Accepted`; v2.6-ADR line 3 uses `Accepted` — but those were post-human-review). Phase 65's verdict is a human-review gate, so the ADR ships `Proposed` and the plan surfaces the recommendation for Oscar; flip to `Accepted` only after sign-off. Do NOT silently lock.

**Body section skeleton** — adr-58 body shape is `Context → Goals → Decision Table → (Trust Boundary / Invariants) → Alternatives Considered` (lines 10, 46, 117, 126/179, 238). Map the six DRV-04 topics (D-09) onto:

| # | DRV-04 topic | ADR section | Reference (don't duplicate) |
|---|--------------|-------------|------------------------------|
| 1 | Interception design | Context + "How the spike intercepts" | `DESIGN.md` ring-buffer+worker; `nono-fltmgr.c` deny chain |
| 2 | Measured latency + finite-timeout fail-open | "Measured Latency" → links appendix | this phase's run; T-63-02 envelope (`.c` lines 333–337) |
| 3 | `windows-drivers-rs` not viable | Decision Table row / Alternatives | `64-CONTEXT.md`; REQUIREMENTS.md Out-of-Scope |
| 4 | FltMgr vs ETW rationale | Decision Table row / Alternatives | FltMgr enforces (deny) vs ETW observes-only |
| 5 | Altitude 365678 + assignment status | short "Altitude" section | `64-SC1-driver-evidence.md` non-collision; fsfcomm@ PENDING |
| 6 | Go/no-go recommendation | "Decision" + "Recommendation (human-review gate)" | D-05 evidence weighing; D-06 surfaces |

**Verdict scoring table** (copy `v2.6-upstream-merge-deferral-ADR.md` Decision Table shape, lines 38–42 — per-option rows, criteria columns, `Verdict` column):

```markdown
| Option | Security gap vs WFP+AppContainer | Cert cost (EV/WHQL) | Maintenance burden | Spike-defect signal | Verdict |
|--------|----------------------------------|---------------------|--------------------|--------------------|---------|
| Go (production driver)            | … | … | … | … | … |
| No-go (WFP+AppContainer suffices) | … | … | … | … | … |
| Conditional-go (gated)            | … | … | … | … | … |
```

> Fill from evidence (D-05: latency + 18 defects + cert cost + altitude assignment + maintenance burden, AGAINST the WFP+AppContainer security gap). Do NOT invent a pass/fail latency threshold (Open Question 3) — present the measured number as ONE input among five.

**Note the fail-direction deferral** (CONTEXT Deferred): the spike uses fail-open to protect the test VM; the production fail-open-vs-fail-closed decision is a separate ADR concern — mention it, don't decide it (mirror adr-58's "Non-goals" section, lines 94–116).

---

### `.planning/architecture/adr-65-latency-appendix.md` (data appendix) — CREATE

**Analog:** no direct precedent. Derive a simple data-appendix shape; it is the **sibling** the core ADR references (D-08, keeps raw tables out of the decision narrative). Match the ADR's `.planning/architecture/` location.

Suggested shape (raw measurement tables, integer-µs in-driver → ms presentation here):

```markdown
# Minifilter Spike — Latency Measurement Appendix (Phase 65 DRV-04)

**Captured:** 2026-06-XX  |  **VM:** nono-fltmgr-vm (rg-nono-fltmgr-spike, 20.51.161.15, Win11 26200)
**Altitude:** 365678  |  **QPC frequency:** <ticks/sec from g_PerfFreq>  |  **Driver build:** <commit / .sys hash, VM-local per T-63-05>

## SPAN-A — Kernel-IPC round-trip (FltSendMessage → FilterReplyMessage), D-02a
| Iterations | Min (µs) | Median (µs) | p99 (µs) |
|-----------|----------|-------------|----------|
| ~100      |          |             |          |

## SPAN-B — Full pre-op → IRP completion (STATUS_ACCESS_DENIED), D-02b
| Iterations | Min (µs) | Median (µs) | p99 (µs) |
|-----------|----------|-------------|----------|
| ~100      |          |             |          |

## Notes
- SPAN-A excludes ring-buffer enqueue + worker wakeup; SPAN-B includes them (scheduling jitter).
- Fail-open envelope: 500ms FltSendMessage timeout (T-63-02) — measured median should be << this.
```

> The 64-SC1 evidence-template shape (raw-output paste blocks + final PASS/FAIL line, per RESEARCH §macOS UAT Sequencing) is the proven model for the surrounding evidence framing.

---

### `crates/nono/src/sandbox/macos.rs` (library, transform) — READ-ONLY RE-VALIDATION TARGET

**Analog:** itself. **No code change expected** — Phase 65 re-validates the already-landed 8f84d454 behavior. Map the existing test analogs the live UAT confirms.

**Deny-after-allow ordering** (the security invariant being re-validated) — generated at lines 708–716:

```rust
// macos.rs lines 708-716 — platform deny rules emitted AFTER write-allows (last-match-wins):
// SECURITY: Platform deny rules are placed AFTER write rules (last-match-wins:
// deny overrides preceding write-allows). Port of upstream 8f84d454.
for rule in caps.platform_rules() {
    profile.push_str(rule);
    profile.push('\n');
}
```

**Dual-path emission** (`/tmp`↔`/private/tmp`, `/etc`↔`/private/etc`) — lines 239–246 in `path_filters_for_cap`:

```rust
// macos.rs lines 239-246 — emit a rule for BOTH original and resolved when they differ:
if cap.original != cap.resolved {
    if let Some(original_str) = cap.original.to_str() {
        let escaped_original = escape_path(original_str)?;
        filters.push(format!("{} \"{}\"", kind, escaped_original));
    }
}
```

**Existing unit-test analogs the macOS CI `test` job runs (D-10 automatable subset):**

| Test | Line | What it asserts |
|------|------|-----------------|
| `test_generate_profile_platform_rules_after_writes` | 997 | `read_pos < write_pos < deny_pos` (deny AFTER write) |
| `test_platform_rules_after_write_allows` | 1880 | same ordering invariant (D-11 sibling) |
| `test_platform_deny_symlink_and_canonical_path` | 1918 | both `/etc/passwd` AND `/private/etc/passwd` deny rules appear (dual-path) |
| `test_generate_profile_extensions_before_platform_deny_rules` | 1128 | extension allows precede platform denies |

```rust
// macos.rs lines 1026-1030 — the canonical ordering assert the UAT dry-run mirrors:
assert!(read_pos < write_pos, "read rules must come before write rules");
assert!(
    write_pos < deny_pos,
    "platform deny rules must come AFTER write rules (last-match-wins)"
);
```

**Live HUMAN-UAT mapping (gate 65-A, host-gated — see Shared Patterns):** the four `nono run` assertions confirm at runtime what these tests assert at compile time:
- `nono run --dry-run --profile claude-code` → deny-after-allow visible (mirrors lines 1026–1030)
- `cat ~/.ssh/id_rsa` blocked (deny enforcement)
- `cat /etc/hosts` AND `cat /private/etc/hosts` both blocked (mirrors `test_platform_deny_symlink_and_canonical_path`, line 1918)
- `make test-lib` green on host (runs all the above tests natively)

---

## Shared Patterns

### Cross-target clippy gate (D-11a) — applies to the macOS re-validation
**Source:** `CLAUDE.md` § Coding Standards (cross-target MUST/NEVER) + `.planning/templates/cross-target-verify-checklist.md`
**Apply to:** `crates/nono/src/sandbox/macos.rs` re-validation; any commit touching cfg-gated Unix code.

```bash
rustup target add x86_64-apple-darwin   # one-time
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
# clean exit -> may flip; link/toolchain error -> mark PARTIAL + require green macos-latest CI SHA (D-11c)
```

The Windows host cannot compile macOS code; `cargo check` does NOT run clippy (Pitfall 4 — the exact v2.9 regression). Verification MUST cite a green `macos-latest` CI run link/SHA as literal gate evidence (D-11c) — no release tag before it.

### HUMAN-UAT-gated phase close (gate 65-A)
**Source:** prior Windows UAT phases (52, 56, 58, 60) — recorded in the VERIFICATION `## Human-verification` section (no dedicated `*HUMAN-UAT*.md` file exists; Assumption A4). 64-VERIFICATION lines 41–48 are the precedent.
**Apply to:** the four macOS live `sandbox_init()` deny assertions.
Pattern: a close-blocking checklist with a `pass/blocked` field per item + host/date stamp; the 64-SC1 evidence-template (raw-output paste + final PASS/FAIL line) is the proven format. Checklist stays OPEN until a real macOS host runs it (none confirmed at plan time).

### ADR `.planning/architecture/` location footer (D-07 / D-46-A2)
**Source:** adr-58 lines 301–304, v2.6-ADR line 12.
**Apply to:** both new ADR files. Each existing ADR carries an explicit file-path note explaining the `.planning/architecture/` (v2.6+) vs `docs/architecture/` (Phase-32-and-earlier) split. Copy that footer so the close gate does not flag a "wrong path" miss against the SC2 `.planning/adr/` shorthand.

### Idempotent VM lifecycle (D-04, Track A)
**Source:** `64-SC1-VM-RUNBOOK.md` §2 (reuse) / §10b (recreate) + `64-vm-runcmd-ewdk-build-local.ps1`.
**Apply to:** the driver rebuild→re-sign→reload→latency-run cycle. Probe `az vm get-instance-view` first; reuse if running, `az vm start` if deallocated, recreate via §10b if gone. The `.sys` stays VM-local (T-63-05) — never committed.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `.planning/architecture/adr-65-latency-appendix.md` | data appendix | n/a | No prior latency-appendix file in the repo. Derived a simple table shape above; it is structurally the sibling-of `adr-65-minifilter-go-no-go.md` (D-08). The 64-SC1 evidence-template framing is the nearest stylistic model. |

## Metadata

**Analog search scope:** `drivers/nono-fltmgr/`, `.planning/architecture/`, `crates/nono/src/sandbox/`
**Files scanned (read in full or targeted):** `nono-fltmgr.c` (676 lines, full), `nono-fltmgr.h` (77 lines, full), `adr-58-windows-hook-executor.md` (305 lines, full), `v2.6-upstream-merge-deferral-ADR.md` (116 lines, full), `macos.rs` (1944 lines — targeted: ordering 700–716, dual-path 222–249, tests 997–1031 + 1880–1943)
**Pattern extraction date:** 2026-06-09
