# Requirements: nono — v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity

**Defined:** 2026-06-06
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms; every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees, or be explicitly documented as intentionally unsupported with a clear rationale.

**Trigger:** v2.8 + v2.9 shipped 2026-06-06 with the Windows confined-tools + out-of-box WFP-enforcement story complete. The two heaviest standing deferrals — **Gap 6b** (kernel file-open/trust interception, no user-mode equivalent on Windows) and **WR-02** (EDR-instrumented validation, deferred every milestone since v2.1) — plus the **dormant macOS Seatbelt backend** (last exercised before the recent Windows-focused milestones; high-water upstream sync `v0.57.0`) are the next frontier. Per the user's scoping: the kernel driver is a **de-risking feasibility spike** (test-signed POC, not a production driver), the EDR theme is **running the deferred UAT** (not building EDR integrations), and macOS = **upstream feature parity through `v0.61.2`**. Research: `.planning/research/SUMMARY.md` (HIGH confidence).

## v1 Requirements (v2.10 Scope)

### Driver — Gap 6b Minifilter Feasibility Spike (DRV)

- [x] **DRV-01**: A **test-signed Windows FltMgr minifilter** intercepts `IRP_MJ_CREATE` pre-operation and **denies a targeted file open** (`FLT_PREOP_COMPLETE` returning `STATUS_ACCESS_DENIED`), demonstrated end-to-end on a Secure-Boot-OFF / HVCI-off VM (a process attempting to open the targeted path is refused at the kernel boundary). POC-depth: a single deterministic deny target is sufficient; production policy breadth is out of scope.
- [x] **DRV-02**: The minifilter performs a **user-mode policy round-trip** — `FLT_PREOP_PENDING` + `FltSendMessage` over a dedicated `FilterCommunicationPort` (`\NonoPolicyPort`) with a **finite timeout** — and a Rust `#[cfg(windows)]` user-mode client (`fltmgr_client.rs`, via the `Win32_Storage_InstallableFileSystems` `windows-sys` feature) receives the request and returns an allow/deny decision that the driver enforces. The `#[repr(C)]` message struct carries a static layout assertion.
- [x] **DRV-03**: A **reproducible driver build + test-signing pipeline** exists and is documented: an out-of-workspace `drivers/nono-fltmgr/` WDK MSBuild project, built and test-signed (`makecert → inf2cat → signtool → certmgr → bcdedit /set testsigning on → pnputil /add-driver`, `SERVICE_DEMAND_START`). The existing `nono-wfp-driver.sys` placeholder and the MSI are **untouched**; the spike binary is for manual test-VM use only and is not production-signed.
- [x] **DRV-04** *(satisfied 2026-06-11 — ADR committed w/ latency + lean No-go/Conditional-go recommendation; `Status: Proposed` pending Oscar's Accepted-flip)*: A **go/no-go ADR** is committed documenting the interception design, the **measured `FLT_PREOP_PENDING` round-trip latency**, the `windows-drivers-rs`-not-viable (C/C++ driver) decision, the FltMgr-vs-ETW rationale, the chosen altitude (Activity-Monitor/FSFilter range, official-assignment request status), and an explicit recommendation for (or against) a production-driver milestone.

### EDR — WR-02 HUMAN-UAT (EDR)

- [x] **EDR-01** *(satisfied 2026-06-11)*: A **HUMAN-UAT artifact** records ~10 pass/fail assertions of nono's behavior and visibility under a **real EDR runner** (Sysmon as EDR-proxy and/or Microsoft Defender for Endpoint), run on existing v2.9 binaries with **no new code**. The matrix is executed in **two passes — no-exclusion first** (to characterize false-positive exposure) **then with-exclusion** (to confirm suppression is sufficient); each assertion records the EDR product, version, and policy mode. → `66-HUMAN-UAT.md`, 10 assertions live on `nono-fltmgr-vm` (Sysmon v15.20 + Defender AV 4.18.26050.15), two passes executed.
- [x] **EDR-02** *(satisfied 2026-06-11)*: The UAT explicitly validates the two load-bearing boundaries: (a) whether **EDR DLL-injection into Low-IL children fails at the `NO_WRITE_UP` MIC boundary** as designed, and (b) whether the broker's `CreateProcessAsUserW` + `SetTokenInformation(IntegrityLevel)` sequence (**MITRE T1134.002**) triggers EDR alerts/quarantine. **WR-02 is closed or explicitly re-scoped** in the planning artifacts with the recorded findings. → (a) Low-IL/AppContainer child confirmed (`S-1-16-4096`), survives AV exclusions; finding: confined child invisible to Sysmon telemetry. (b) T1134.002 integrity-drop captured at the broker chain; no Defender alert/quarantine. **WR-02 CLOSED** (validated under a representative EDR-proxy).

### macOS — Seatbelt Upstream Parity through v0.61.2 (MACOS)

- [x] **MACOS-01**: A **`DIVERGENCE-LEDGER.md`** audits upstream `always-further/nono` `v0.57.0..v0.61.2` **scoped to the macOS surface** (a `macos-only` column, mirroring the Phase 42/47 `windows-touch` audit shape), inventorying every macOS-relevant commit — including the UPST7-deferred items (`$PWD` symlink-CWD capture, platform-rules-after-user-write-allows ordering) — with per-commit dispositions (will-sync / fork-preserve / won't-sync) and a diff-inspect note per the `feedback_cluster_isolation_invalid` lesson.
- [x] **MACOS-02**: The **P1 macOS security/correctness commits** are absorbed with verbatim D-19 `Upstream-commit:` trailers: `8f84d454` (platform rules evaluated after user write-allows — security ordering defect), `362ada22` and `8f1b0b74` (symlink / `$PWD` CWD capture correctness). **Seatbelt rule ordering is asserted by unit tests** (last-match-wins emission order), not merely rule presence; the fork's profile-emission call site is diff-inspected before each cherry-pick (the upstream fix may apply at a different site).
- [x] **MACOS-03** *(satisfied 2026-06-11)*: The **Seatbelt layer is re-validated live on a real macOS host** (a `sandbox_init()`-backed `nono` run confirming allow/deny + the absorbed ordering fix), and the **macOS CI build leg is confirmed green before any release tag** — a HARD close gate, not advisory (the v2.9 cross-target-drift guard: two cfg-gated compile errors reached release tags because the Windows host never compiles macOS code). The cherry-pick checklist scans for edition-2024 let-chains / E0716-class borrows and canonical-path (`/private/etc`, `/private/tmp`) coverage.

## v2 Requirements (Deferred)

- **DRV-PROD-01** *(deferred — gated on DRV-04 go/no-go)*: A **production EV/WHQL-signed** Gap 6b minifilter integrated into the runtime-trust path, MSI-packaged with the clean-uninstall invariant preserved, with kernel-version-maintenance hardening. Future milestone (v2.11/v3.0).
- **EDR-INTEG-01** *(deferred)*: nono **emits structured EDR/ETW security telemetry** (sandbox events, denials, broker spawns) for EDR consumption — an integration, distinct from the v2.10 validation UAT.
- **UPST8-NONMAC-01** *(deferred)*: The **non-macOS** UPST8 cherry-pick clusters (Windows/Linux upstream `v0.60.0..v0.61.2`) on the fork's normal sync cadence — v2.10 absorbs only the macOS-relevant slice.

## Out of Scope (Explicit Exclusions)

| Feature | Reason |
|---------|--------|
| Production EV/WHQL driver signing + kernel-version-maintenance hardening | Gated on the DRV-04 spike go/no-go; a production driver is a separate, cert-and-maintenance-heavy milestone. |
| Rust kernel-mode minifilter bindings (`windows-drivers-rs`) | Early-stage / KMDF-v1.33-only / not production-recommended; the spike driver is C/C++ + WDK. |
| `IRP_MJ_ACQUIRE_FOR_SECTION_SYNCHRONIZATION` pre-exec deny | Can only return `STATUS_INSUFFICIENT_RESOURCES`, not `STATUS_ACCESS_DENIED`; pre-create (`IRP_MJ_CREATE`) is the spike's interception point. |
| EDR telemetry emission / EDR-evasion-resistance hardening | v2.10 *validates under* EDR (WR-02 UAT); building EDR integrations is deferred (EDR-INTEG-01). |
| Non-macOS UPST8 cherry-picks (Windows/Linux upstream sync) | Stays on the fork's own cadence; v2.10's upstream sync is scoped to the macOS surface through `v0.61.2`. |
| macOS `sandbox-exec` migration | Deprecated with no public replacement API; continue using `sandbox_init()`. |
| WR-02 EDR HUMAN-UAT on a CI runner | Requires a real host with a real EDR agent — cannot run in CI (re-affirmed since v2.1). |

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| DRV-01 | Phase 64 | Complete |
| DRV-02 | Phase 64 | Complete |
| DRV-03 | Phase 63 (partial groundwork) + Phase 64 (complete) | Complete |
| DRV-04 | Phase 65 | Satisfied (2026-06-11) — ADR Proposed, sign-off pending |
| EDR-01 | Phase 66 | Satisfied (2026-06-11) |
| EDR-02 | Phase 66 | Satisfied (2026-06-11) — WR-02 CLOSED |
| MACOS-01 | Phase 63 | Complete |
| MACOS-02 | Phase 64 | Complete |
| MACOS-03 | Phase 65 | Satisfied (2026-06-11) — Seatbelt re-validated; resl A5 failure filed as separate defect |

**Coverage:**
- v1 requirements: 9 total
- Mapped to phases: 9/9 (100%)

---
*Requirements defined: 2026-06-06*
*Traceability filled: 2026-06-06 (roadmap creation)*
