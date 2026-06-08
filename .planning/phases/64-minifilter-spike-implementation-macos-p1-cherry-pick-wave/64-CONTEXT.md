# Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave - Context

**Gathered:** 2026-06-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Two tracks that turn Phase 63's groundwork into a working spike + the absorbed macOS fixes:

- **Track A — Minifilter spike implementation (DRV-01, DRV-02, DRV-03 complete):** Extend the
  Phase 63 `drivers/nono-fltmgr/` skeleton with a real `IRP_MJ_CREATE` pre-create callback that
  denies a single deterministic target path at the kernel boundary, wire the kernel↔user-mode
  policy round-trip over `\NonoPolicyPort` (`FltSendMessage` + a new Rust `fltmgr_client.rs`
  user-mode client), complete the full test-signing pipeline and load the driver on a fresh
  Secure-Boot-OFF test VM, and document both build pipelines in `drivers/README.md`.
- **Track B — macOS P1 cherry-pick wave (MACOS-02):** Land the three P1 macOS security/correctness
  commits (`8f84d454`, `362ada22`, `8f1b0b74`) per the Phase 63 DIVERGENCE-LEDGER C14 disposition,
  with verbatim D-19 `Upstream-commit:` trailers and Seatbelt rule-ordering unit tests.

**Not in this phase:** the DRV-04 go/no-go ADR + measured round-trip latency (Phase 65); MACOS-03
live macOS-host re-validation + green-CI hard release gate (Phase 65); EDR HUMAN-UAT (Phase 66);
any production EV/WHQL driver signing or MSI-bundling (DRV-PROD-01, future milestone). The existing
`nono-wfp-driver.sys` placeholder and the MSI stay untouched.

</domain>

<decisions>
## Implementation Decisions

### Track A — Driver interception & deny proof (DRV-01)
- **D-01:** Deny demonstration = **scripted test harness**. A small harness attempts to open the
  deny-target path and asserts the open is refused with Win32 `ERROR_ACCESS_DENIED` (5) /
  `STATUS_ACCESS_DENIED`. Harness output **plus** `fltmc instances` / `fltmc filters` (proving the
  driver is registered at the chosen altitude) are captured to the Phase 64 SC1 evidence artifact.
  Repeatable, unambiguous — chosen over a manual interactive attempt.
- **D-02:** Deny target = **a single dedicated deterministic throwaway path** provisioned on the VM
  (e.g. `C:\nono-deny-test\secret.txt`). POC depth per DRV-01: one deterministic deny target is
  sufficient; the user-mode policy client hard-codes the deny rule for this one path. Production
  policy breadth is explicitly out of scope.

### Track A — Kernel↔user IPC & Rust client (DRV-02)
- **D-03:** `fltmgr_client.rs` lives in a **new standalone spike crate that IS a Cargo workspace
  member**, with ALL code `#[cfg(windows)]` (compiles to nothing on Linux/macOS CI). Uses
  `windows-sys` with the `Win32_Storage_InstallableFileSystems` feature. This isolates throwaway
  spike code from `nono-cli`'s production path while still getting `cargo test` / clippy coverage
  for the static-layout assertion.
- **D-04:** IPC message = `#[repr(C)] NonoIpcRequest` with **minimal POC fields** — a fixed-size
  WCHAR path buffer + originating PID + desired-access/operation — plus a
  `static_assert(sizeof(NonoIpcRequest) == N)` on the C side and a matching Rust compile-time layout
  assertion. No version/request-id field for the spike (ABI-insurance fields deferred to the
  production ADR).
- **D-05:** The driver-side pre-create callback uses the **carried-forward** ring-buffer +
  worker-thread + finite-~500 ms `FltSendMessage` + **fail-open-on-`STATUS_TIMEOUT`** pattern from
  `DESIGN.md` (T-63-02). This is NOT re-decided here; the implementation MUST reference
  `drivers/nono-fltmgr/DESIGN.md` as the hard BSOD-avoidance pre-code gate.

### Track A — VM, test-signing pipeline & docs (DRV-01 / DRV-03)
- **D-06:** VM = **reprovision a fresh Azure VM via Phase 63's scripts** (same Win11 WDK-paired
  image, **Standard** security type, Secure-Boot OFF / HVCI off). Take a **pre-load snapshot**
  before `pnputil /add-driver` — the `SERVICE_DEMAND_START` + snapshot pairing is the BSOD rollback
  safeguard.
- **D-07:** Phase 64 **completes DRV-03** = the FULL test-signing pipeline on the VM:
  `makecert → inf2cat → signtool → certmgr` (install test cert) `→ bcdedit /set testsigning on →
  pnputil /add-driver`, `SERVICE_DEMAND_START`. Capture `fltmc instances`/`fltmc filters` (driver
  registered at the chosen altitude) + the D-01 deny proof. (Phase 63 was compile-proof only, with
  `SignMode=Off`.)
- **D-08:** Altitude = **enumerate `fltmc filters` on the fresh VM and pick a non-colliding number**
  in the FSFilter Activity Monitor band (360000–389999), avoiding the AV range 320000–329998.
  Replace the `370020` placeholder in the INF + `DESIGN.md`. The official Microsoft altitude
  assignment remains pending and is NOT a blocker for the test-signed spike.
- **D-09:** `drivers/README.md` (SC4) documents **both pipelines end-to-end** — the C driver
  build+test-sign+load command sequence AND the Rust `fltmgr_client` build/run — with exact commands
  + VM prerequisites. The `nono-wfp-driver.sys` placeholder + MSI stay untouched.

### Track B — macOS P1 cherry-pick wave (MACOS-02)
- **D-10:** Cherry-pick the three P1 commits (`8f84d454`, `362ada22`, `8f1b0b74`) per the Phase 63
  DIVERGENCE-LEDGER C14 disposition, with **verbatim D-19 `Upstream-commit:` trailers**. When a
  commit does NOT apply cleanly because the fork's profile-emission call-site differs from upstream,
  **manually port** the fix at the fork's correct call-site, keep the verbatim `Upstream-commit:`
  trailer, and note the site divergence in the commit body. Diff-inspect each call-site
  (`generate_profile` / `sandbox_prepare` / `add_platform_rule`) before applying
  (`feedback_cluster_isolation_invalid` lesson).
- **D-11:** Unit tests assert Seatbelt rule **ordering** (last-match-wins: deny rules emitted AFTER
  the allow rules they override), not mere rule presence. Cover BOTH the symlink path AND the
  canonical `/private/etc` path for every affected deny group.
- **D-12:** macOS **cross-target verification runs in Phase 64** — `cargo clippy`/`build`
  `--target x86_64-apple-darwin` AND `aarch64-apple-darwin` from the dev host. If the darwin
  cross-toolchain is not installed, **mark the cross-target REQ PARTIAL and defer to live CI** per
  the CLAUDE.md cross-target MUST rule + `.planning/templates/cross-target-verify-checklist.md`.
  (The live macOS-host re-validation + green-CI hard gate is Phase 65 / MACOS-03, NOT this phase.)

### Claude's Discretion
- Exact deny-target path string and harness language (PowerShell vs a tiny Rust/C exe) — D-01/D-02
  fix the method + assertion; researcher/planner picks the concrete path + language.
- Exact `NonoIpcRequest` field widths (path-buffer length, access-mask type) and the static-assert
  value `N` — D-04 fixes the field set; researcher sizes them.
- The chosen altitude number within the Activity-Monitor band — executor picks after `fltmc filters`
  enumeration on the VM (D-08).
- The spike crate's name (e.g. `nono-fltmgr-client`) and its exact directory.
- Whether the C-side static assertion uses C11 `_Static_assert` or a WDK-compatible compile-time
  check.

</decisions>

<carried_forward>
## Carried Forward from Phase 63 (locked — do NOT re-decide)

From `63-CONTEXT.md` and `drivers/nono-fltmgr/DESIGN.md`:
- IPC = ring-buffer + worker-thread; `FltSendMessage` finite ~500 ms timeout; **fail-open on
  `STATUS_TIMEOUT`** (spike-only, T-63-02); `\NonoPolicyPort` dedicated FilterCommunicationPort
  (NOT the WFP named pipe); `NonPagedPoolNx` for all callback-reachable allocations; `NT_ASSERT`
  IRQL guards; **no driver-originated file I/O** (no `ZwCreateFile`/`NtCreateFile`).
- Altitude = FSFilter Activity Monitor band (360000–389999), avoid AV range 320000–329998;
  `370020` placeholder; official Microsoft assignment still `pending` (request sent 2026-06-07).
- The three P1 commits are dispositioned `will-sync` in the Phase 63 ledger; Phase 64 executes it.
- Spike `.sys` is VM-local throwaway (NOT committed, NOT MSI-bundled); `nono-wfp-driver.sys`
  placeholder untouched.
- VM = Azure, Standard security type, Secure-Boot OFF / HVCI off; `SERVICE_DEMAND_START` boot-loop
  safeguard + pre-load snapshot.

</carried_forward>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/REQUIREMENTS.md` — DRV-01..04, MACOS-01..03, EDR-01..02 with full acceptance language;
  out-of-scope table; traceability (DRV-01 / DRV-02 / MACOS-02 → Phase 64; DRV-03 complete here).
- `.planning/ROADMAP.md` § Phase 64 — goal, depends-on (Phase 63), success criteria 1–4.

### Driver design — MUST read before any driver code
- `drivers/nono-fltmgr/DESIGN.md` — the hard pre-code BSOD-avoidance gate (D-10): STRIDE register
  T-63-01..05; ring-buffer + worker-thread IPC; finite 500 ms `FltSendMessage`; fail-open;
  `NonPagedPoolNx`; IRQL asserts; altitude band; `\NonoPolicyPort`; `#[repr(C)]` static-assert note.
- `drivers/nono-fltmgr/nono-fltmgr.c` / `.inf` / `.vcxproj` / `.vcxproj.filters` — Phase 63 skeleton
  to extend (empty callbacks array, `StartType=3`, altitude placeholder `370020`).
- `docs/architecture/minifilter-spike-design-pointer.md` — ADR pointer stub.

### Research (HIGH confidence)
- `.planning/research/PITFALLS.md` — kernel BSOD triad (IRQL, own-I/O recursion, blocking
  `FltSendMessage`) + altitude/EDR collision + the macOS cross-target-drift release blocker.
- `.planning/research/SUMMARY.md` / `STACK.md` / `ARCHITECTURE.md` / `FEATURES.md` — WDK versions,
  test-signing pipeline, suggested build order, integration points.

### macOS cherry-pick source-of-truth
- `.planning/phases/63-minifilter-spike-groundwork-macos-divergence-ledger-audit/63-DIVERGENCE-LEDGER.md`
  — C14 cluster: the three P1 commits' `will-sync` dispositions, per-commit diff-inspect notes, and
  the Phase 54 C14 supersession (D-13).
- `crates/nono/src/sandbox/macos.rs` — Seatbelt backend (primary cherry-pick target).
- `crates/nono-cli/src/policy.rs` + profile-emission / capability code — shared deps in scope
  (Phase 63 D-11).
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md`,
  `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md`,
  `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md` — prior ledger
  shape/column templates.

### VM provisioning + test-signing (reuse)
- `.planning/phases/63-.../63-SC1-vm-state.md` — VM-state evidence shape (TESTSIGNING / Secure Boot
  / HVCI / build result).
- `.planning/phases/63-.../63-preflight-azure.ps1`,
  `63-vm-runcmd-enable-testsigning.ps1`, `63-vm-runcmd-ewdk-download.ps1`,
  `63-vm-runcmd-ewdk-build.ps1`, `63-vm-runcmd-sc1-check.ps1`, `63-vm-runcmd-diag.ps1` — reusable
  Azure provisioning + EWDK build + test-signing scripts.
- `.planning/phases/63-.../63-altitude-request.md` — altitude request status (pending).

### Carried-forward context
- `.planning/phases/63-.../63-CONTEXT.md` — Phase 63 locked decisions D-01..D-13.

### Existing Windows placeholder (MUST stay untouched)
- `crates/nono-cli/data/windows/nono-wfp-driver.sys` — out-of-scope placeholder; the spike
  `nono-fltmgr.sys` is a separate driver and is NOT MSI-bundled.

### Project memory / lessons (load-bearing)
- Memory `feedback_cluster_isolation_invalid` — diff-inspect re-export surfaces, not `--name-only`
  (drives D-10 cherry-pick site verification).
- Memory `feedback_clippy_cross_target` — macOS cross-target drift is a proven release blocker
  (drives D-12).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Phase 63 WDK skeleton** (`drivers/nono-fltmgr/`) — `DriverEntry` / `FltRegisterFilter` /
  `FltStartFiltering` / `NonoFltUnload` + empty callbacks array; extend with the pre-create callback
  body + `FltCreateCommunicationPort` + `FltSendMessage` + ring buffer.
- **`windows-sys = "0.59"`** already in `crates/nono-cli/Cargo.toml` — add the
  `Win32_Storage_InstallableFileSystems` feature in the new spike crate (D-03).
- **Phase 63 Azure provisioning + test-signing PowerShell scripts** — reusable for the fresh VM.
- **Prior DIVERGENCE-LEDGER files** (42/47/54/63) — cherry-pick disposition source + column shape.

### Established Patterns
- **Out-of-workspace C/C++ WDK MSBuild driver project** (`drivers/nono-fltmgr/`) — NOT a Cargo
  member; `windows-drivers-rs` ruled out. The Rust client (D-03) is the inverse: a `#[cfg(windows)]`
  Cargo workspace member.
- **Cross-platform cfg discipline** — cross-target clippy is a MUST for cfg-gated Unix code
  (CLAUDE.md); macOS cherry-picks touch `macos.rs` + shared policy code (drives D-12).
- **D-19 `Upstream-commit:` trailer** convention for fork cherry-picks.
- **Seatbelt last-match-wins rule ordering** (drives the D-11 ordering tests).

### Integration Points
- First real kernel↔user-mode integration: `\NonoPolicyPort` (`FltCreateCommunicationPort` +
  `FltSendMessage`) ↔ the `fltmgr_client.rs` spike crate.
- macOS cherry-picks integrate into `crates/nono/src/sandbox/macos.rs` +
  `crates/nono-cli/src/policy.rs` profile emission.

</code_context>

<specifics>
## Specific Ideas

- "A single deterministic deny target is sufficient" (DRV-01 POC depth) — one hard-coded deny path,
  not a policy engine.
- The scripted deny harness must assert the **exact** Win32 error (`ERROR_ACCESS_DENIED` = 5), not
  just "the open failed".
- Cross-target macOS verification is a **hard rule, not advisory** — v2.9 shipped two cfg-gated
  compile errors to release tags because the Windows host never compiles macOS code
  (`feedback_clippy_cross_target`).

</specifics>

<deferred>
## Deferred Ideas

- **DRV-04** go/no-go ADR + measured `FLT_PREOP_PENDING` round-trip latency → Phase 65.
- **MACOS-03** live macOS-host re-validation + green macOS CI leg as a HARD release gate → Phase 65.
- **EDR-01 / EDR-02** HUMAN-UAT under a real EDR (Sysmon / Defender for Endpoint) → Phase 66.
- **DRV-PROD-01** production EV/WHQL driver signing + MSI-bundling + kernel-version-maintenance
  hardening → future milestone, gated on the DRV-04 go/no-go.
- **`729697c2` `--trust-proxy-ca` (P2)** + the non-macOS UPST8 slice → deferred (cherry-pick
  priority below the three P1s).
- **`NonoIpcRequest` version / request-id ABI-insurance fields** → production-ADR consideration, not
  the spike (D-04).
- **Official Microsoft altitude assignment** before any non-disposable deployment → tracked, pending
  Microsoft reply (~30 business days from 2026-06-07).

</deferred>

---

*Phase: 64-Minifilter Spike Implementation + macOS P1 Cherry-pick Wave*
*Context gathered: 2026-06-08*
