# Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit - Context

**Gathered:** 2026-06-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Two parallel-safe tracks of **groundwork before any driver code or cherry-pick runs**:

- **Track A — Minifilter spike groundwork:** Stand up a test-signing-capable Windows environment, scaffold the out-of-workspace `drivers/nono-fltmgr/` WDK MSBuild project so it compiles to a `.sys`, write the pre-code design doc (the BSOD-avoidance gate), and kick off the long-lead Microsoft altitude request. (DRV-03 partial — build pipeline + groundwork documented; full DRV-01/02/03 land in Phase 64.)
- **Track B — macOS DIVERGENCE-LEDGER audit:** Produce a complete `DIVERGENCE-LEDGER.md` for upstream `v0.57.0..v0.61.2` scoped to the macOS surface, with every macOS-relevant commit dispositioned, ready for Phase 64's cherry-pick wave. (MACOS-01.)

**Not in this phase:** any driver C code beyond a skeleton entry point, loading/installing the driver on the VM (Phase 64), the actual macOS cherry-picks (Phase 64), the go/no-go ADR with latency data (Phase 65). New capabilities belong in their own phases.

</domain>

<decisions>
## Implementation Decisions

### Track A — Test-signing VM environment
- **D-01:** The spike's test box is a **cloud VM, not local Hyper-V.** The roadmap's "Hyper-V Secure-Boot-OFF VM" language is satisfied by any disposable Windows box meeting the real requirements: Secure Boot OFF, HVCI off, free to reboot/brick, snapshot-able. This keeps the bricking risk off the primary Win11 26200 dev host.
- **D-02:** **Provider = Azure.** Use a **Standard security type** VM (NOT Trusted Launch) — Gen1, or Gen2 with Secure Boot disabled — so `bcdedit /set testsigning on` + reboot works. Azure chosen over AWS for native Hyper-V underneath, snapshot/serial-console/boot-diagnostics recovery, and best-fit WDK/VS tooling. (AWS noted as viable fallback; nested Hyper-V there needs a `.metal` instance.)
- **D-03:** **Provision the VM in-phase.** Phase 63 stands up the Azure VM, captures the SC1 `msinfo32` (HVCI/Secure Boot state) and `bcdedit /enum all` (TESTSIGNING state) reproducibility artifacts **on that VM**, and performs the SC2 compile-to-`.sys` proof there. Self-contained — no deferred host-availability gate for the build/compile proof.
- **D-04:** **Debugging = lean (snapshot + minidumps).** Single Azure VM; test-sign and load directly on it; rely on pre-load snapshots + crash minidumps (`!analyze -v`) for BSOD diagnosis. No nested-virt / live WinDbg required for Phase 63. (Nested inner-VM live WinDbg via named-pipe COM is the documented escalation path if Phase 64 BSOD iteration gets painful — not built now.)
- **D-05:** **VM image = latest WDK-recommended Win11 pairing** (the build the current WDK 28000.1761/VS2026 or 26100.6584/VS2022 is validated against) — prioritizes smoothest toolchain install over exact parity with the 26200 host.
- **D-06:** The already-locked **`SERVICE_DEMAND_START`** disposition is the boot-loop safeguard: a bad driver won't auto-load at boot, so a BSOD plus a pre-load snapshot = instant rollback rather than a Startup-Repair brick.

### Track A — Microsoft altitude request
- **D-07:** **Send the request in-phase.** Phase 63 physically emails `fsfcomm@microsoft.com` to start the ~30 business-day clock as early as possible, and records the request date + `pending` status as the SC3 artifact. *(Human action gate: the email needs company/contact/driver-purpose details — the planner/executor drafts it; the user sends it and reports the date.)*
- **D-08:** **Provisional altitude = FSFilter Activity Monitor band** (360000–389999) — matches the spike's observe/intercept role. **MUST avoid the AV range 320000–329998** (can fail load AND disrupt the installed EDR). The exact unused number is the researcher's call at Phase 64 plan-time; Phase 63 records the band + the AV-range constraint.

### Track A — Pre-code design doc
- **D-09:** **Canonical location = `drivers/nono-fltmgr/DESIGN.md`** (co-located with the driver code it gates) **+ a one-line pointer stub in `.planning/adr/`** so neither a driver dev nor a planning reader misses it. (Planner may flip which side is canonical if it falls out cleaner.)
- **D-10:** The design doc is a **hard pre-code gate** — it must exist and specify: ring-buffer + worker-thread IPC pattern; **no driver-originated file I/O** (no `ZwCreateFile`/`NtCreateFile`; if any internal I/O is unavoidable use `FltCreateFile` on the minifilter's own instance); **finite `FltSendMessage` timeout** (e.g. 500 ms → `STATUS_TIMEOUT`, never an infinite wait); `NonPagedPoolNx` for callback-reachable allocations; IRQL assertions; the chosen altitude. (See PITFALLS.md 1–3.)

### Track B — macOS DIVERGENCE-LEDGER
- **D-11:** **Scope = Seatbelt backend + shared dependencies, with re-export diff-inspect.** Inventory `sandbox/macos.rs` AND the shared profile-emission / capability / policy code the Seatbelt backend depends on. Disposition by **diff-inspecting re-export surfaces, NOT just `git log --name-only`** — per the `feedback_cluster_isolation_invalid` lesson (Phase 43 proved a `--name-only`-isolated commit had cross-cluster re-export deps). This is the right scope because the three P1 commits themselves touch shared emission/capture code, not just macos.rs.
- **D-12:** **Full dispositions in Phase 63.** Every macOS-relevant commit in `v0.57.0..v0.61.2` gets a `will-sync` / `fork-preserve` / `won't-sync` / `split` disposition + a diff-inspect note, plus a `macos-only` column (mirroring the Phase 42/47/54 `windows-touch` audit shape). Phase 64's cherry-pick wave then just *executes* the ledger.
- **D-13:** The three P1 commits — **`8f84d454`** (platform rules after user write-allows — security ordering defect), **`362ada22`** + **`8f1b0b74`** (symlink / `$PWD` CWD capture correctness) — are firmly dispositioned `will-sync`. `729697c2` (`--trust-proxy-ca`) is a known P2 in-range candidate.

### Claude's Discretion
- **Design-doc ↔ ADR relationship:** Whether the Phase 63 design doc is standalone-seeds-the-ADR-later or written as an early draft of the DRV-04 ADR is the planner's call. Either way, the design doc remains a hard pre-code gate (D-10).
- **Exact provisional altitude number** within the FSFilter Activity Monitor band (researcher at Phase 64 plan-time).
- **Whether canonical design-doc location flips** to `.planning/adr/` if it falls out cleaner (D-09).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/REQUIREMENTS.md` — v2.10 requirements; DRV-01..04, EDR-01..02, MACOS-01..03 with full acceptance language and out-of-scope table.
- `.planning/ROADMAP.md` § Phase 63 — goal, depends-on, success criteria 1–4.

### Research (HIGH confidence — read before driver/audit work)
- `.planning/research/SUMMARY.md` — work-stream overview, stack additions, suggested build order, open questions.
- `.planning/research/PITFALLS.md` — **mandatory for the design doc**: Pitfall 1 (IRQL/BSOD), Pitfall 2 (own-I/O recursion), Pitfall 3 (blocking FltSendMessage hang); macOS cross-target-drift release-blocker.
- `.planning/research/ARCHITECTURE.md` — integration points per theme.
- `.planning/research/STACK.md` / `.planning/research/FEATURES.md` — WDK versions, test-signing pipeline, table-stakes vs anti-features.

### macOS audit — prior ledger templates (audit shape to mirror)
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` — most recent UPST ledger (shape + columns).
- `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` — the `windows-touch`-column template MACOS-01 mirrors with a `macos-only` column.
- `.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md` — backfill-style ledger reference.

### macOS source under audit
- `crates/nono/src/sandbox/macos.rs` — Seatbelt backend (primary audit target).
- `crates/nono-cli/src/policy.rs` + profile-emission / capability code — shared deps in scope per D-11.

### Existing Windows driver placeholder (MUST stay untouched)
- `crates/nono-cli/data/windows/nono-wfp-driver.sys` — out-of-scope placeholder; the spike's `nono-fltmgr.sys` is separate and NOT MSI-bundled.

### Project memory / lessons (load-bearing)
- Memory `feedback_cluster_isolation_invalid` — diff-inspect re-export surfaces, not `--name-only` (drives D-11).
- Memory `feedback_clippy_cross_target` — macOS cross-target drift is a proven release blocker (relevant to Phase 64/65 cherry-picks).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Prior DIVERGENCE-LEDGER files (Phases 42/47/54):** directly reusable column structure + disposition vocabulary; add a `macos-only` column in place of `windows-touch`.
- **`windows-sys = "0.59"` already in `crates/nono-cli/Cargo.toml`:** the *only* Cargo change anticipated across the spike is adding the `Win32_Storage_InstallableFileSystems` feature (for the Phase 64 user-mode `fltmgr_client.rs`) — Phase 63 does not need it yet.

### Established Patterns
- **Out-of-workspace driver project:** `drivers/nono-fltmgr/` is entirely new and is NOT a Cargo workspace member (C/C++ WDK MSBuild; `windows-drivers-rs` ruled out). Keeps the Rust workspace build untouched.
- **Cross-platform cfg discipline:** macOS cherry-picks must respect the cross-target-drift guard — relevant to Phase 64/65, but the ledger should flag any commit touching cfg-gated shared code.

### Integration Points
- Phase 63 produces no runtime integration — it's scaffolding (driver project, design doc, VM, ledger). The first real kernel↔user-mode integration (`\NonoPolicyPort` + `fltmgr_client.rs`) is Phase 64.

</code_context>

<specifics>
## Specific Ideas

- "Where does this VM need to live?" → resolved to a **cloud VM (Azure preferred)**, explicitly NOT required to be local Hyper-V. The user has Azure + AWS access; Azure chosen (D-02).
- The design doc must read as a BSOD-avoidance contract — the three PITFALLS.md kernel pitfalls are the checklist it has to satisfy (D-10).

</specifics>

<deferred>
## Deferred Ideas

- **Nested-virt inner-VM live WinDbg** (named-pipe COM kernel debugging) — documented as the escalation path; only stand up if Phase 64 BSOD iteration becomes painful. Not built in Phase 63.
- **Production EV/WHQL driver signing, MSI-bundling the driver, kernel-version-maintenance hardening** — gated on the DRV-04 go/no-go (DRV-PROD-01, future milestone).
- **`729697c2` `--trust-proxy-ca` (P2)** and other non-macOS upstream clusters — `--trust-proxy-ca` is in-range for the ledger inventory but is P2 (cherry-pick priority below the three P1s); the non-macOS UPST8 slice stays deferred (UPST8-NONMAC-01).
- **Exact altitude number selection** — deferred to the Phase 64 researcher within the FSFilter Activity Monitor band.

</deferred>

---

*Phase: 63-Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit*
*Context gathered: 2026-06-06*
</content>
</invoke>
