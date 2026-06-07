# Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-06
**Phase:** 63-Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit
**Areas discussed:** VM/WDK readiness, Microsoft altitude request, Design-doc artifact, DIVERGENCE-LEDGER scope

---

## Gray-area selection

All four offered gray areas selected for discussion: VM/WDK readiness, MS altitude request, Design-doc artifact, Ledger scope breadth. (Most of the WHAT was already locked by REQUIREMENTS.md, research SUMMARY, and the STATE Key Decisions table.)

---

## VM/WDK readiness

User clarification before answering: *"I have access to Azure and AWS. Where does this VM need to live?"* — Resolved that the spike does NOT require local Hyper-V; any disposable Windows box with Secure Boot OFF, HVCI off, reboot/brick freedom, and snapshots satisfies the requirement. Azure recommended over AWS (native Hyper-V, snapshot/serial-console recovery, WDK/VS fit; AWS nested virt needs `.metal`).

### VM plan (reformulated after clarification)

| Option | Description | Selected |
|--------|-------------|----------|
| Provision Azure VM in-phase | Stand up VM, capture msinfo32/bcdedit on it, full SC1+SC2 compile-to-.sys there | ✓ |
| Scaffold now, gate VM proof | Build project + design doc on dev host; defer VM provision + .sys proof to a host gate | |
| Provision Azure VM, defer to me | Produce recipe; user runs it; record proof once VM is up | |

**User's choice:** Provision Azure VM in-phase.

### Kdebug capability

| Option | Description | Selected |
|--------|-------------|----------|
| Snapshot + minidumps (lean) | Single VM, snapshots + !analyze -v; no nested virt | ✓ |
| Nested inner-VM live WinDbg | Nested-virt size, inner Hyper-V target, named-pipe COM live debug | |
| Provision both, decide in 64 | Stand up nested-capable VM, wire lean now, document WinDbg recipe | |

**User's choice:** Snapshot + minidumps (lean).

### VM image

| Option | Description | Selected |
|--------|-------------|----------|
| Match host (Win11 26200) | Same build as dev host for target parity | |
| Latest WDK-recommended pairing | Build the current WDK/VS is validated against; smoothest toolchain | ✓ |

**User's choice:** Latest WDK-recommended pairing.

**Notes:** Nested-virt live-WinDbg recorded as the documented escalation path for Phase 64 if BSOD iteration gets painful. `SERVICE_DEMAND_START` (already locked) is the boot-loop safeguard.

---

## Microsoft altitude request

| Option | Description | Selected |
|--------|-------------|----------|
| Send in-phase, record status | Email fsfcomm@microsoft.com to start the ~30-day clock; record date + pending as SC3 | ✓ |
| Prepare request, you send it | Draft the email; user sends; record drafted/awaiting | |
| Document plan, defer request | Record template + provisional altitude only; defer send to Phase 65 | |

**User's choice:** Send in-phase, record status.

### Provisional altitude

| Option | Description | Selected |
|--------|-------------|----------|
| FSFilter Activity Monitor band | Unused value in 360000–389999; matches observe/intercept role | ✓ |
| Microsoft-reserved test band | A documented dev/test-safe value if one applies | |
| You decide (researcher picks) | Record band + AV-range constraint only | |

**User's choice:** FSFilter Activity Monitor band (exact number → Phase 64 researcher; MUST avoid AV range 320000–329998).

**Notes:** Human action gate — the email needs company/contact/driver-purpose details; planner drafts, user sends and reports the date.

---

## Design-doc artifact

### Doc location

| Option | Description | Selected |
|--------|-------------|----------|
| drivers/nono-fltmgr/DESIGN.md | Co-located with the driver code it gates | |
| .planning/adr/ | With other ADRs/planning artifacts | |
| Both (stub + canonical) | Canonical in one place, one-line pointer from the other | ✓ |

**User's choice:** Both — recorded as canonical `drivers/nono-fltmgr/DESIGN.md` + pointer stub in `.planning/adr/` (planner may flip).

### ADR relationship

| Option | Description | Selected |
|--------|-------------|----------|
| Standalone now, seeds ADR later | Phase 63 doc standalone; Phase 65 writes DRV-04 ADR fresh | |
| Draft of the eventual ADR | Write design doc as an early draft of the DRV-04 ADR | |
| You decide | Planner chooses; record hard pre-code-gate constraint either way | ✓ |

**User's choice:** You decide (planner's call; hard pre-code gate either way).

---

## DIVERGENCE-LEDGER scope

### Scope breadth

| Option | Description | Selected |
|--------|-------------|----------|
| Backend + shared deps (diff-inspect) | macos.rs + shared profile-emission/capability/policy code; diff-inspect re-export surfaces | ✓ |
| Strict macos.rs/Seatbelt only | macOS-specific files only | |
| Broad sweep, macos-only column flags | Inventory all touched commits, flag macOS subset via column | |

**User's choice:** Backend + shared deps with re-export diff-inspect (per feedback_cluster_isolation_invalid).

### Dispositions

| Option | Description | Selected |
|--------|-------------|----------|
| Full disposition in 63 | Every macOS-relevant commit dispositioned + diff-inspect note in Phase 63 | ✓ |
| P1 firm, rest provisional | P1 firm; rest provisional, confirmed at Phase 64 plan-time | |
| You decide | Audit determines depth; P1 firm + ledger complete enough for Phase 64 | |

**User's choice:** Full disposition in Phase 63 — Phase 64 just executes the ledger.

---

## Claude's Discretion

- Design-doc ↔ DRV-04 ADR relationship (standalone-seeds vs draft-of).
- Exact provisional altitude number within the FSFilter Activity Monitor band (Phase 64 researcher).
- Whether the canonical design-doc location flips to `.planning/adr/`.

## Deferred Ideas

- Nested-virt inner-VM live WinDbg — escalation path only.
- Production EV/WHQL signing, MSI-bundling the driver, kernel-version-maintenance hardening (DRV-PROD-01, gated on DRV-04).
- `729697c2` `--trust-proxy-ca` (P2) and non-macOS UPST8 clusters (UPST8-NONMAC-01).
</content>
