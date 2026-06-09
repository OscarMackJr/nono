# Phase 65: Minifilter ADR + macOS Live Re-validation - Context

**Gathered:** 2026-06-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 65 delivers two independent workstreams over the Phase-64 spike + macOS code that already landed:

1. **DRV-04 — Go/no-go ADR** formalizing the Phase-64 minifilter spike verdict, backed by a **freshly instrumented `FLT_PREOP_PENDING` round-trip latency measurement** captured on the spike VM (Track A — driver instrumentation + re-run).
2. **MACOS-03 — macOS live re-validation HUMAN-UAT** of the already-landed Seatbelt code (Phase 63 MACOS-01 + Phase 64 MACOS-02 ordering/dual-path fixes), with the **macOS CI build leg green as a HARD close gate** before any release tag.

**This phase clarifies HOW to write the ADR and HOW to sequence the macOS UAT — it does NOT add new sandbox capabilities.** The minifilter driver and macOS Seatbelt code already exist; Phase 65 measures, documents, decides, and re-validates.

**Explicitly NOT in this phase:**
- Building a production EV/WHQL-signed driver (DRV-PROD-01 — deferred, gated on this ADR's verdict).
- New macOS Seatbelt features or cherry-picks beyond re-validating what Phase 64 absorbed (non-macOS UPST8 slice is UPST8-NONMAC-01, deferred).
- WR-02 EDR HUMAN-UAT (Phase 66).

</domain>

<decisions>
## Implementation Decisions

### Latency measurement (DRV-04 / SC1)
- **D-01:** Capture a **real instrumented number**, not just the design budget. The ~500ms `FltSendMessage` timeout is the fail-open envelope, NOT the measured latency — Phase 64 never instrumented an actual round-trip. Phase 65 adds timing and re-runs on the VM.
- **D-02:** Measure **both layered spans** so the ADR can attribute where time goes:
  - **(a) Kernel-only IPC span** — `KeQueryPerformanceCounter` immediately before `FltSendMessage` to immediately after `FilterReplyMessage` returns in the worker thread (pure kernel→user→kernel cost, excludes ring-buffer enqueue).
  - **(b) Full pre-op→completion span** — from pre-create callback entry (enqueue) through IRP completion with `STATUS_ACCESS_DENIED` (user-perceived deny path including ring-buffer + worker wakeup + scheduling jitter).
- **D-03:** Statistical rigor: report **median + p99 over ~100 denied creates** for each span.
- **D-04:** **Track A scope** — this requires a driver rebuild → re-sign → reload cycle on the spike VM per `64-SC1-VM-RUNBOOK.md`. The VM (`nono-fltmgr-vm`, rg `rg-nono-fltmgr-spike`, IP `20.51.161.15`) is expected **still alive / operator will confirm**; planner should make VM provisioning **idempotent** (reuse if present, recreate from runbook if gone) so the measurement step is self-contained. Per T-63-05 the `.sys` stays VM-local (not committed).

### Go/no-go verdict (DRV-04 / SC1)
- **D-05:** **Let the evidence decide — the verdict is NOT pre-committed.** The ADR must weigh: the freshly measured latency numbers (D-01..D-03) + the 18 spike defects + EV/WHQL cert cost + official Microsoft altitude assignment + ongoing kernel-version maintenance burden, AGAINST the security gap (if any) that WFP+AppContainer — the already-shipping kernel-enforced layer — cannot close.
- **D-06:** The written analysis drives a **recommended** direction (go / no-go / conditional-go-gated), but the final recommendation is a **HUMAN-review gate**: the operator (Oscar) reviews the ADR's recommendation before it is considered final. Plan/execute should surface the recommendation for review, not silently lock it.

### ADR location + structure (DRV-04 / SC2)
- **D-07:** **Location follows repo convention, NOT the SC's literal path.** Write the ADR to `.planning/architecture/` (next to existing `adr-58-windows-hook-executor.md`, `v2.6-upstream-merge-deferral-ADR.md`), suggested name `adr-65-minifilter-go-no-go.md`. The SC2 phrase "committed to `.planning/adr/`" is treated as **descriptive shorthand** — **this path deviation MUST be noted in verification** so the close gate doesn't flag a "wrong path" miss.
- **D-08:** **Structure = core decision ADR (concise) + linked latency-data appendix/evidence file.** Raw measurement tables (per-span min/median/p99, iteration counts, VM context) live in a separate appendix/evidence file so they don't bloat the decision narrative. The ADR **references** `drivers/nono-fltmgr/DESIGN.md` and the Phase-64 SC1 evidence — it does **not** duplicate them.
- **D-09:** Single ADR covers all six DRV-04 topics as sections: interception design, measured latency (+ finite-timeout fail-open behavior), `windows-drivers-rs`-not-viable decision, FltMgr-vs-ETW rationale, chosen altitude (365678) + official-assignment request status, and the explicit go/no-go recommendation.

### macOS UAT sequencing (MACOS-03 / SC2 + SC3)
- **D-10:** **"Code-ready now, UAT-gated."** The Seatbelt code already landed (Phase 63/64); split the work:
  - **Automatable now:** macOS CI build leg green + `x86_64-apple-darwin` clippy + `sandbox::macos` ordering tests.
  - **Live HUMAN-UAT (host-gated):** the SC2 live `sandbox_init()` assertions — `nono run --dry-run --profile claude-code` emits deny-after-allow ordering; `nono run --profile claude-code -- cat ~/.ssh/id_rsa` is **blocked**; **both** `/etc/hosts` **and** `/private/etc/hosts` are blocked; `make test-lib` green on the host — are staged as a **HUMAN-UAT checklist that BLOCKS phase close** until run on a real macOS host (gate 65-A). Mirrors prior Windows HUMAN-UAT phase structure. No macOS host is confirmed available at discuss time.
- **D-11:** **CI macOS green is a HARD gate, evidenced — not advisory.** Enforcement is the v2.9 cross-target-drift guard (two cfg-gated compile errors reached tags because the Windows host never compiles macOS code). Phase 65 MUST: (a) run `cargo clippy --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` from the dev host per CLAUDE.md, (b) scan the cherry-pick checklist for edition-2024 let-chains / E0716-class borrows / canonical-path (`/private/etc`, `/private/tmp`) coverage, AND (c) **capture a green macOS CI run link/SHA in the verification artifact as literal gate evidence**. No release tag until that run is green.

### Claude's Discretion
- Exact ADR section ordering/headings, the appendix filename, the precise instrumentation code (counter placement, log format) — provided D-01..D-03 spans + rigor are honored.
- VM recreate mechanics if the VM is gone — follow `64-SC1-VM-RUNBOOK.md`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### DRV-04 / minifilter spike (ADR source material)
- `.planning/REQUIREMENTS.md` — DRV-04 (go/no-go ADR contents), DRV-PROD-01 (deferred production driver this ADR gates), and the v2 "Out of Scope" table rows (windows-drivers-rs, IRP_MJ_ACQUIRE, EDR telemetry).
- `drivers/nono-fltmgr/DESIGN.md` — interception design, STRIDE threat register (T-63-01..05), ring-buffer+worker IPC pattern, finite-timeout fail-open rationale (T-63-02). ADR references, does not duplicate.
- `.planning/phases/64-minifilter-spike-implementation-macos-p1-cherry-pick-wave/64-SC1-driver-evidence.md` — SC1 PASS evidence: chosen altitude 365678 + non-collision `fltmc` output, the 18 live-UAT defects + fix commits, end-to-end deny chain. The ADR's "what the spike proved" backbone.
- `.planning/phases/64-.../64-SC1-VM-RUNBOOK.md` — VM provisioning + test-sign + load runbook for the latency re-run (idempotent recreate path).
- `.planning/phases/64-.../64-CONTEXT.md` — Phase-64 decisions that carry forward (windows-drivers-rs not viable, C/C++ MSBuild driver, dedicated `\NonoPolicyPort`, ABI-insurance deferral).

### ADR convention (location + format)
- `.planning/architecture/adr-58-windows-hook-executor.md` — existing ADR format/location precedent (the convention D-07 follows).
- `.planning/architecture/v2.6-upstream-merge-deferral-ADR.md` — second existing-ADR precedent.

### MACOS-03 / Seatbelt re-validation
- `.planning/REQUIREMENTS.md` — MACOS-03 (live re-validation + HARD CI gate), the v2.9 cross-target-drift rationale.
- `crates/nono/src/sandbox/macos.rs` — Seatbelt profile generation; deny-after-allow ordering + `/private/etc`÷`/private/tmp` dual-path handling (8f84d454, Phase 64). Re-validation target.
- `.planning/phases/64-.../64-VERIFICATION.md` — MACOS-02 cherry-pick evidence (8f1b0b74, 362ada22, 8f84d454) + the macOS-CI-must-run note.
- `CLAUDE.md` § Coding Standards — the cross-target clippy MUST/NEVER rule (`x86_64-apple-darwin` verification).
- `.planning/templates/cross-target-verify-checklist.md` — the cross-target verification checklist D-11 scans.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`drivers/nono-fltmgr/nono-fltmgr.c`** + worker-thread loop: the latency instrumentation (D-02) hooks directly into the existing `FltSendMessage`/`FilterReplyMessage` worker span — add `KeQueryPerformanceCounter` counter pairs, no architectural change.
- **`64-SC1-VM-RUNBOOK.md` + `64-vm-runcmd-ewdk-build-local.ps1`**: the build/sign/load pipeline is already captured and proven (with the EWDK 26H1 deviations corrected); reuse for the re-run.
- **`crates/nono/src/sandbox/macos.rs`**: the dual-path + ordering logic to re-validate is already present — UAT confirms behavior, no code change expected.

### Established Patterns
- **HUMAN-UAT-gated phase close** (prior Windows UAT phases): automatable parts green now, live host assertions block close — directly mirrored by D-10.
- **Cross-target clippy MUST** (CLAUDE.md, promoted at v2.5 / hardened at v2.9): the macOS leg cannot be trusted from a Windows host; D-11 enforces dev-host `x86_64-apple-darwin` clippy + CI-green evidence.
- **ADR-in-`.planning/architecture/`** convention (D-07).

### Integration Points
- ADR verdict (D-05/D-06) gates DRV-PROD-01 (future milestone) — the ADR is the decision record that unlocks or shelves production-driver work.
- macOS CI green (D-11) gates the release tag — no v2.10 tag until the macOS leg is green.

</code_context>

<specifics>
## Specific Ideas

- Suggested ADR filename: `.planning/architecture/adr-65-minifilter-go-no-go.md` + a sibling latency-data appendix/evidence file.
- Altitude already chosen and proven: **365678** (FSFilter Activity Monitor band 360000–389999, non-colliding). Official Microsoft altitude assignment (fsfcomm@microsoft.com) status is documented as a precondition, not yet requested.
- Latency target spans: kernel-IPC-only AND full pre-op→IRP-completion, median + p99 over ~100 denied creates.
- macOS SC2 deny assertions to run live: `cat ~/.ssh/id_rsa` blocked; `/etc/hosts` AND `/private/etc/hosts` both blocked; dry-run profile shows deny-after-allow.

</specifics>

<deferred>
## Deferred Ideas

- **Production EV/WHQL-signed driver + kernel-version-maintenance hardening** (DRV-PROD-01) — gated on this ADR's verdict; future milestone (v2.11/v3.0).
- **EDR/ETW structured telemetry emission** (EDR-INTEG-01) — building EDR integrations is out of scope; Phase 66 only validates *under* EDR.
- **Non-macOS UPST8 cherry-pick clusters** (UPST8-NONMAC-01) — Windows/Linux upstream `v0.60.0..v0.61.2` on normal sync cadence; v2.10 absorbs only the macOS slice.
- **`NonoIpcRequest` version / request-id ABI-insurance fields** — production-ADR consideration, not the spike.
- **Fail-direction for a production driver** (fail-open vs fail-closed) — the spike uses fail-open to avoid locking the test VM; the production decision is a separate ADR concern (note it in the ADR, don't decide it).

None of the above are in Phase 65 scope — discussion stayed within the phase boundary.

</deferred>

---

*Phase: 65-minifilter-adr-macos-live-re-validation*
*Context gathered: 2026-06-09*
