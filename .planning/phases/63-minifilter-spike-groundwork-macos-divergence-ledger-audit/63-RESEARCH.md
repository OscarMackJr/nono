# Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit - Research

**Researched:** 2026-06-06
**Domain:** Windows kernel minifilter (FltMgr/WDK) build-environment groundwork + macOS Seatbelt upstream-divergence audit
**Confidence:** HIGH (codebase-grounded for Track B; WDK/Azure/Microsoft-Learn-cited for Track A; all four P1/P2 commits verified present in local object store; fork target sites diff-confirmed)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Track A — Test-signing VM environment**
- **D-01:** Spike test box is a **cloud VM, not local Hyper-V.** ROADMAP's "Hyper-V Secure-Boot-OFF VM" language is satisfied by any disposable Windows box meeting the real requirements: Secure Boot OFF, HVCI off, free to reboot/brick, snapshot-able. Keeps bricking risk off the Win11 26200 dev host.
- **D-02:** **Provider = Azure.** Use a **Standard security type** VM (NOT Trusted Launch) — Gen1, or Gen2 with Secure Boot disabled — so `bcdedit /set testsigning on` + reboot works. (AWS noted as viable fallback; nested Hyper-V there needs a `.metal` instance.)
- **D-03:** **Provision the VM in-phase.** Phase 63 stands up the Azure VM, captures SC1 `msinfo32` (HVCI/Secure Boot) and `bcdedit /enum all` (TESTSIGNING) reproducibility artifacts **on that VM**, and performs the SC2 compile-to-`.sys` proof there. Self-contained — no deferred host-availability gate.
- **D-04:** **Debugging = lean (snapshot + minidumps).** Single Azure VM; test-sign and load directly on it; rely on pre-load snapshots + crash minidumps (`!analyze -v`). No nested-virt / live WinDbg required for Phase 63.
- **D-05:** **VM image = latest WDK-recommended Win11 pairing** (the build the current WDK 28000.1761/VS2026 or 26100.6584/VS2022 is validated against) — prioritizes smoothest toolchain install over exact parity with the 26200 host.
- **D-06:** **`SERVICE_DEMAND_START`** is the boot-loop safeguard: a bad driver won't auto-load at boot, so BSOD + pre-load snapshot = instant rollback rather than a Startup-Repair brick.

**Track A — Microsoft altitude request**
- **D-07:** **Send the request in-phase.** Phase 63 physically emails `fsfcomm@microsoft.com` to start the ~30 business-day clock as early as possible; records request date + `pending` status as the SC3 artifact. *(Human action gate: planner/executor drafts the email; user sends it and reports the date.)*
- **D-08:** **Provisional altitude = FSFilter Activity Monitor band** (360000–389999) — matches the spike's observe/intercept role. **MUST avoid the AV range 320000–329998.** Exact unused number is the researcher's call at Phase 64 plan-time; Phase 63 records the band + the AV-range constraint.

**Track A — Pre-code design doc**
- **D-09:** **Canonical location = `drivers/nono-fltmgr/DESIGN.md`** (co-located with the code it gates) **+ a one-line pointer stub in `.planning/adr/`.** (Planner may flip which side is canonical if it falls out cleaner.)
- **D-10:** Design doc is a **hard pre-code gate** — must specify: ring-buffer + worker-thread IPC pattern; **no driver-originated file I/O** (no `ZwCreateFile`/`NtCreateFile`; if internal I/O is unavoidable use `FltCreateFile` on the minifilter's own instance); **finite `FltSendMessage` timeout** (e.g. 500 ms → `STATUS_TIMEOUT`, never infinite); `NonPagedPoolNx` for callback-reachable allocations; IRQL assertions; the chosen altitude.

**Track B — macOS DIVERGENCE-LEDGER**
- **D-11:** **Scope = Seatbelt backend + shared dependencies, with re-export diff-inspect.** Inventory `sandbox/macos.rs` AND the shared profile-emission / capability / policy code the Seatbelt backend depends on. Disposition by **diff-inspecting re-export surfaces, NOT just `git log --name-only`** — per the `feedback_cluster_isolation_invalid` lesson.
- **D-12:** **Full dispositions in Phase 63.** Every macOS-relevant commit in `v0.57.0..v0.61.2` gets a `will-sync` / `fork-preserve` / `won't-sync` / `split` disposition + a diff-inspect note, plus a `macos-only` column (mirroring Phase 42/47/54 `windows-touch` shape).
- **D-13:** The three P1 commits — **`8f84d454`** (platform rules after user write-allows), **`362ada22`** + **`8f1b0b74`** (symlink / `$PWD` CWD capture) — are firmly dispositioned `will-sync`. `729697c2` (`--trust-proxy-ca`) is a known P2 in-range candidate.

### Claude's Discretion
- **Design-doc ↔ ADR relationship:** standalone-seeds-the-ADR-later vs. early draft of the DRV-04 ADR is the planner's call. Either way it remains a hard pre-code gate (D-10).
- **Exact provisional altitude number** within the FSFilter Activity Monitor band (deferred to Phase 64 plan-time).
- **Whether canonical design-doc location flips** to `.planning/adr/` if it falls out cleaner (D-09).

### Deferred Ideas (OUT OF SCOPE)
- **Nested-virt inner-VM live WinDbg** (named-pipe COM kernel debugging) — escalation path only; not built in Phase 63.
- **Production EV/WHQL driver signing, MSI-bundling the driver, kernel-version-maintenance hardening** — gated on DRV-04 go/no-go (DRV-PROD-01, future milestone).
- **`729697c2 --trust-proxy-ca` (P2) and other non-macOS upstream clusters** — `--trust-proxy-ca` is *in-range for the ledger inventory* but is P2 (below the three P1s in cherry-pick priority); the non-macOS UPST8 slice stays deferred (UPST8-NONMAC-01).
- **Exact altitude number selection** — deferred to the Phase 64 researcher.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DRV-03 (partial) | Reproducible driver build + test-signing pipeline exists and is documented: out-of-workspace `drivers/nono-fltmgr/` WDK MSBuild project, built and test-signed. Phase 63 = build pipeline + groundwork documented (full DRV-03 lands in Phase 64). | Track A §§1–4: concrete `.vcxproj`/`.inf`/`.c` scaffold (Standard Stack + Code Examples), `msbuild.exe` invocation, Azure Standard-security-type VM provisioning, test-signing pipeline (`makecert → inf2cat → signtool → certmgr → bcdedit → pnputil`), `SERVICE_DEMAND_START` INF config. |
| MACOS-01 | A `DIVERGENCE-LEDGER.md` audits upstream `v0.57.0..v0.61.2` scoped to the macOS surface (a `macos-only` column mirroring the Phase 42/47 `windows-touch` shape), inventorying every macOS-relevant commit — including UPST7-deferred items — with per-commit dispositions and a diff-inspect note per `feedback_cluster_isolation_invalid`. | Track B §5: exact git inventory commands, the `git fetch upstream --tags` precondition (v0.61.2 NOT in local store), diff-inspect-re-export method operationalized, prior-ledger column shape, the three P1 + one P2 commits verified in range with fork-target-site mapping. |

**Note on DRV-01/DRV-02/DRV-04:** explicitly NOT in Phase 63 (Phase 64/65). Research below references them only to keep the Phase 63 scaffold compatible with what they need.
</phase_requirements>

## Summary

Phase 63 is **pure groundwork** across two parallel-safe, independent tracks. Track A stands up a disposable Azure test-signing Windows VM, scaffolds an out-of-workspace `drivers/nono-fltmgr/` WDK MSBuild project that compiles to a `.sys` (no real driver logic beyond a skeleton `DriverEntry`/`FltRegisterFilter`/unload), writes the BSOD-avoidance design doc that gates all future driver code, and kicks off the ~30-business-day Microsoft altitude request. Track B produces a complete macOS-scoped `DIVERGENCE-LEDGER.md` for `v0.57.0..v0.61.2` with every macOS-relevant commit dispositioned. Neither track produces runtime integration; both are throwaway/scaffolding deliverables verified by artifact existence + a compile proof + an explicit disposition table.

The single highest-risk Track A landmine is **silent driver-load failure from the TESTSIGNING/HVCI/Secure-Boot triad** (PITFALLS Pitfall 4) — but D-02's Standard-security-type Azure VM (Secure Boot OFF, no HVCI default) plus D-03's `msinfo32`/`bcdedit /enum all` capture *is* the mitigation, and Phase 63 only needs the VM to **compile** a `.sys` (load/install is Phase 64). The single highest-value Track B finding is that **all three P1 commits and `c6730e43` map cleanly to fork-carried files** — `8f84d454` and `c6730e43` touch `crates/nono/src/sandbox/macos.rs`; `362ada22`+`8f1b0b74` touch `crates/nono-cli/src/sandbox_prepare.rs`; the fork carries both files at the same call sites, so the diff-inspect is tractable. The audit's defining subtlety: Phase 54's ledger already dispositioned the three P1 commits as **`won't-sync`** (its C14 cluster) because v2.7 was Windows-focused; Phase 63 (D-13) **overrides that to `will-sync`** because v2.10 changed scope to macOS parity. The new ledger must explicitly record this supersession.

**Primary recommendation:** Run the two tracks as independent waves. Track A: (1) provision Azure Standard-security-type Gen2-SB-off Win11 VM via `az vm create`, (2) install WDK 28000.1761 + VS 2026 (or EWDK ISO), (3) scaffold + `msbuild` the `.vcxproj` to `.sys`, (4) capture `msinfo32` + `bcdedit /enum all`, (5) write `drivers/nono-fltmgr/DESIGN.md` from PITFALLS 1–3, (6) draft + send the altitude email. Track B: `git fetch upstream --tags` (MANDATORY — v0.61.2 absent locally), run the pinned drift tool for the full range, diff-inspect re-export/call-site surfaces, write the ledger mirroring Phase 54's column shape with a `macos-only` column, explicitly superseding Phase 54's C14 `won't-sync` verdict.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Pre-create file-open interception (future) | Kernel (FltMgr `.sys`, C) | — | Only the kernel filter manager sees `IRP_MJ_CREATE`; no user-mode equivalent exists on Windows (this is exactly Gap 6b) |
| Kernel↔user policy round-trip (future) | Kernel `.sys` ↔ User-mode Rust client | — | `FltSendMessage` (kernel) ↔ `FilterGetMessage` (user) over a named comm port; Phase 64, not 63 |
| Driver build + test-sign pipeline | Build tooling (WDK MSBuild, outside Cargo) | Azure VM (host env) | `.sys` is C/WDK MSBuild; never a Cargo workspace member; VM provides the test-signing-capable host |
| Design-doc BSOD-avoidance gate | Documentation (`drivers/nono-fltmgr/DESIGN.md`) | `.planning/adr/` pointer stub | Pre-code contract; co-located with code it gates (D-09) |
| Altitude assignment | External (Microsoft `fsfcomm@`) | Documentation (request-status artifact) | Long-lead human-gated request; Phase 63 records band + sends email |
| macOS Seatbelt rule emission (audit target) | Library (`crates/nono/src/sandbox/macos.rs`) | CLI policy (`crates/nono-cli/src/policy.rs`, `sandbox_prepare.rs`) | Profile string assembled in macos.rs `generate_profile`; rules *built* in policy.rs/sandbox_prepare.rs — the shared-dep surface D-11 requires inspecting |
| Divergence audit | Read-only git analysis | Documentation (`DIVERGENCE-LEDGER.md`) | No code change; inventory + disposition only |

## Standard Stack

### Track A — Driver build toolchain (C/C++, outside Cargo)

> **All versions per `.planning/research/STACK.md` (HIGH confidence, MSDN-verified 2026-06-06). Do not re-research.** The table below is the actionable subset for Phase 63's *compile-only* proof.

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| WDK | 28000.1761 (latest, 2026-05) `[CITED: STACK.md / learn.microsoft.com/windows-hardware/drivers/download-the-wdk]` | Kernel headers, `FltMgr.lib`, driver project templates, `inf2cat`/`signtool`/`certmgr` | Current Microsoft-recommended; SDK and WDK build numbers MUST match |
| Visual Studio | 2026 Community/Professional `[CITED: STACK.md]` | C/C++ compiler, WDK VSIX, driver project templates | WDK 28000.1761 VSIX targets VS 2026; VS 2022 pairs with WDK 26100.6584 (D-05 alternative) |
| Windows SDK | 10.0.28000.1 (matching WDK) `[CITED: STACK.md]` | Headers + `signtool.exe`/`makecert.exe`/`certmgr.exe` | SDK build number must exactly match WDK |
| EWDK ISO | VS 2026 Build Tools 18.3.0 + SDK + WDK in one mountable ISO `[CITED: STACK.md]` | Self-contained alternative if installing full VS on the VM is impractical | Faster, deterministic VM setup; mountable, no VS installer dance |
| `FltMgr.lib` | Ships with WDK | Import library for the minifilter API | Link via `#pragma comment(lib, "FltMgr.lib")` |

### Track A — Driver API surface (kernel-mode C, `<fltKernel.h>`) — skeleton only in Phase 63

| API | Purpose in the Phase 63 SKELETON | Notes |
|-----|----------------------------------|-------|
| `DriverEntry` | Entry point; calls `FltRegisterFilter` + `FltStartFiltering` | Required for a buildable `.sys` |
| `FltRegisterFilter` | Register minifilter with FltMgr | Skeleton can register with an empty/no-op operation-callbacks array |
| `FltStartFiltering` | Activate after registration | Skeleton-valid |
| `FltUnregisterFilter` | Clean unload in the `FilterUnload` callback | D-10 + Pitfall 3 require a registered unload callback |
| `FLT_REGISTRATION` struct | Static registration table passed to `FltRegisterFilter` | Minimal `FLT_REGISTRATION` with `Unload` set; pre-op callback can be a stub for the *compile* proof |

> **Phase 63 scope guard:** the skeleton needs only enough to **compile and link to a `.sys`**. The real pre-create callback, comm port, `FltSendMessage` round-trip, ring buffer, and worker thread are **Phase 64** (DRV-01/DRV-02). Adding them in Phase 63 is Pitfall 6 scope creep.

### Track B — No new dependencies

**Track B is read-only git analysis.** No Cargo change, no new tooling. The drift tool (`scripts/check-upstream-drift.sh` / `.ps1`, pinned sha `0834aa664fbaf4c5e41af5debece292992211559`) already exists `[VERIFIED: codebase grep]`.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Azure Standard-security-type VM (D-02) | AWS `.metal` instance w/ nested Hyper-V | Viable fallback; AWS lacks native single-VM Secure-Boot-off Standard image, needs bare-metal for nested virt — more cost/complexity |
| WDK 28000.1761 + VS 2026 | WDK 26100.6584 + VS 2022 (D-05 alternative) | If VS 2026 unavailable on the chosen Azure image; SDK/WDK numbers must still match |
| Full VS install on VM | EWDK ISO | EWDK is faster + deterministic but uses `msbuild` from a command prompt, not the IDE — fine for a compile-only proof |
| C/C++ WDK driver | `windows-drivers-rs` (Rust kernel) | RULED OUT per REQUIREMENTS § Out of Scope: early-stage, KMDF-v1.33-only, not production-recommended |

**Installation (Track A driver toolchain — runs ON the Azure VM, not the dev host):**
```powershell
# Option A: full VS 2026 + WDK 28000.1761 (IDE iteration)
#   Install VS 2026 with "Desktop development with C++", the six Spectre-mitigated
#   library individual components, and the "Windows Driver Kit" individual component.
#   Then install the matching Windows SDK 10.0.28000.1 and WDK 28000.1761.
# Option B: EWDK ISO (command-line, self-contained) — mount and run LaunchBuildEnv.cmd
```

**Version verification (run on the VM before relying on the toolchain):**
```powershell
# Confirm the WDK/SDK build numbers match and msbuild can target the Driver platform
where msbuild
msbuild -version
# Confirm signtool / makecert / certmgr present (test-signing pipeline, used Phase 64)
where signtool ; where makecert ; where certmgr ; where inf2cat
```

## Package Legitimacy Audit

**Not applicable.** Phase 63 installs **no external software packages** into the nono codebase. Track A installs Microsoft-first-party tooling (WDK/VS/SDK/EWDK) onto a disposable Azure VM via official Microsoft download channels — there is no npm/PyPI/crates registry surface to slopcheck. Track B adds nothing. The *only* anticipated Cargo change across the whole spike (`windows-sys` feature `Win32_Storage_InstallableFileSystems`) is a **Phase 64** addition, not Phase 63 (per CONTEXT § Reusable Assets). slopcheck/registry verification is therefore vacuously satisfied for Phase 63.

## Architecture Patterns

### System Architecture Diagram (Phase 63 deliverable surface)

```
TRACK A (Windows kernel groundwork) ─ parallel-safe ─ TRACK B (macOS audit)

  ┌─────────────────────────────────────────┐    ┌──────────────────────────────────────┐
  │ Azure Standard-sec VM (Gen2, SB OFF)     │    │ Read-only git over upstream tags      │
  │  ├ msinfo32  ──────► SC1 artifact         │    │  git fetch upstream --tags  (REQUIRED │
  │  │   (HVCI / Secure Boot state)           │    │     — v0.61.2 absent locally)         │
  │  ├ bcdedit /enum all ─► SC1 artifact      │    │           │                           │
  │  │   (TESTSIGNING state)                  │    │           ▼                           │
  │  ├ WDK 28000.1761 + VS2026 / EWDK         │    │  drift tool (pinned sha) ──► commit   │
  │  │           │                            │    │   inventory v0.57.0..v0.61.2          │
  │  │           ▼                            │    │           │                           │
  │  │  drivers/nono-fltmgr/ scaffold         │    │           ▼                           │
  │  │   .vcxproj + .inf + skeleton .c        │    │  per-commit diff-inspect              │
  │  │           │ msbuild /p:Configuration   │    │   (re-export + call-site surfaces,    │
  │  │           ▼                            │    │    NOT just --name-only)              │
  │  │   nono-fltmgr.sys  ──► SC2 (compiles)  │    │           │                           │
  │  └─ DESIGN.md (PITFALLS 1-3) ─► SC3 gate  │    │           ▼                           │
  │     + .planning/adr/ pointer stub         │    │  DIVERGENCE-LEDGER.md ──► SC4         │
  └────────────┬────────────────────────────┘    │   (macos-only column; will-sync/      │
               │ email (human-sent)               │    fork-preserve/won't-sync/split;     │
               ▼                                   │    P1 x3 = will-sync, supersedes       │
   fsfcomm@microsoft.com ─► SC3 altitude           │    Phase 54 C14 won't-sync)            │
     request (~30 biz-day clock)                   └──────────────────────────────────────┘
```

### Recommended Project Structure (Track A scaffold — entirely new, outside the Cargo workspace)
```
drivers/                              # NEW top-level dir; NOT a Cargo workspace member
└── nono-fltmgr/
    ├── DESIGN.md                     # D-09 canonical pre-code gate (SC3)
    ├── nono-fltmgr.vcxproj           # WDK MSBuild project (ConfigurationType=Driver)
    ├── nono-fltmgr.vcxproj.filters   # optional VS solution-explorer grouping
    ├── nono-fltmgr.inf               # FltMgr service install, SERVICE_DEMAND_START, altitude placeholder
    ├── nono-fltmgr.c                 # skeleton DriverEntry + FltRegisterFilter + Unload
    └── README.md                     # (Phase 64) build + test-sign pipeline doc
```
> Keep `drivers/` out of `Cargo.toml [workspace]` members so `make build`/`make ci` (Rust) are untouched (CONTEXT § Established Patterns). Do NOT touch `crates/nono-cli/data/windows/nono-wfp-driver.sys` (out-of-scope placeholder, CONTEXT canonical refs).

### Pattern 1: Minimal buildable FltMgr minifilter scaffold
**What:** The smallest `.vcxproj` + `.inf` + `.c` set that MSBuild compiles to a `.sys`.
**When to use:** Phase 63 SC2 (compile proof). Mirror Microsoft's `nullFilter` sample, which is the canonical "does nothing but registers" minifilter.
**Source:** `[CITED: github.com/microsoft/Windows-driver-samples/tree/main/filesys/miniFilter/nullFilter]` — altitude 370020, `LoadOrderGroup "FSFilter Activity Monitor"`, `StartType SERVICE_DEMAND_START`.

See **Code Examples** below for the concrete skeleton.

### Pattern 2: Diff-inspect re-export/call-site audit (Track B, operationalized for D-11)
**What:** For each macOS-relevant commit, do NOT trust `git log --name-only` file overlap. Run `git show <sha>` and inspect (a) added/changed `pub use` / `pub mod` / `extern crate` / `pub(crate)` symbols, AND (b) **function-call dependencies** — does the commit *call* a symbol introduced by a different cluster?
**When to use:** Every will-sync candidate in the ledger.
**Why it exists:** Phase 43 proved `8b888a1c` re-exported `public_key_id_hex`/`sign_statement_bundle` that were NOT defined by the commit itself and NOT in the fork — a `--name-only`-isolated commit had a cross-cluster prereq (`feedback_cluster_isolation_invalid`). Phase 54 then surfaced a *function-call* cross-cluster dep (C5 → C3 `partition_allow_domain`) that the pub-use scan alone missed.
**Operational meaning for macos.rs's shared deps:** The Seatbelt profile string is *assembled* in `crates/nono/src/sandbox/macos.rs::generate_profile`, but the deny/allow rules are *built* in `crates/nono-cli/src/policy.rs` (`add_platform_rule` call sites) and the CWD capability is built in `crates/nono-cli/src/sandbox_prepare.rs`. A commit that "fixes ordering in macos.rs" may depend on a rule that policy.rs emits at a *different* call site than upstream. For each P1 commit, the ledger's diff-inspect note must answer: *does the fork's call site that produces the affected rule match the upstream site the commit patches?*

### Anti-Patterns to Avoid
- **(Track A) Adding the pre-create callback / comm port / FltSendMessage in Phase 63:** scope creep (PITFALLS Pitfall 6). The skeleton compiles; the logic is Phase 64.
- **(Track A) Using `ZwCreateFile` anywhere in the skeleton:** even a "log to file" stub seeds the Pitfall 2 recursion-BSOD pattern the DESIGN.md forbids. The skeleton must not do file I/O at all.
- **(Track B) Dispositioning by `git log --name-only` overlap alone:** the exact `feedback_cluster_isolation_invalid` trap (D-11).
- **(Track B) Re-litigating Phase 54's C14 verdict silently:** the new ledger overrides C14 `won't-sync`→`will-sync` because v2.10 scope changed; this must be stated explicitly, not glossed.
- **(Track A) Provisioning a Trusted Launch VM:** Trusted Launch enforces Secure Boot, which blocks `bcdedit /set testsigning on` (D-02; Pitfall 4 scenario 1).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Minifilter `.vcxproj`/`.inf`/`.c` from scratch | A bespoke driver project from MSDN prose | Copy the structure of Microsoft's `nullFilter` / `minispy` / `scanner` WDK samples | The samples encode the correct `ConfigurationType=Driver`, `DriverType`, Spectre-mitigation flags, and INF section layout that a hand-rolled project silently gets wrong (silent load failure, Pitfall 4) |
| Test-signing pipeline | A custom signing script | The documented `makecert → inf2cat → signtool → certmgr → bcdedit /set testsigning on → pnputil /add-driver` chain (STACK.md §A4) | Order matters; embed-signing the `.sys` is required for 64-bit even on demand-start; a hand-rolled chain skips steps and fails silently (Phase 64) |
| Upstream commit inventory | `git log` one-offs with ad-hoc filters | The pinned `make check-upstream-drift` tool | Reproducibility: the ledger frontmatter records the tool sha + invocation so the audit is regenerable (Phase 42/47/54 precedent) |
| Altitude number selection | Picking a round number from a tutorial | Request an official altitude from `fsfcomm@microsoft.com`; provisionally use the Activity-Monitor band, NEVER the AV range | A tutorial altitude collides with installed EDR (Pitfall 5) — fails registration or blinds the EDR |

**Key insight:** Both tracks have a "looks trivial, fails silently" core. A minifilter that "compiles" but was scaffolded wrong won't *load* (Phase 64 surprise); a ledger built on `--name-only` overlap *looks* complete but misses cross-cluster prereqs (Phase 43's proven failure mode). Lean on the canonical Microsoft samples and the pinned drift tool rather than reconstructing either by hand.

## Runtime State Inventory

> Phase 63 is **groundwork/scaffolding + read-only audit** — it renames nothing and migrates no data. This section is included only to record the explicit "nothing found" verdict for the categories that *could* plausibly apply.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — Phase 63 stores no records keyed on any string. | None |
| Live service config | None — the spike's `nono-fltmgr` is NOT registered/installed in Phase 63 (install is Phase 64). The existing `nono-wfp-service` is untouched. | None |
| OS-registered state | None in Phase 63 — `SERVICE_DEMAND_START` registration happens at Phase 64 install time. The Azure VM itself is disposable/snapshot-able (D-04). | None |
| Secrets/env vars | None — no new secret keys or env var names introduced. The test cert (Phase 64) lives only on the disposable VM. | None |
| Build artifacts | A `.sys` is produced ON the Azure VM (SC2). It is a throwaway compile artifact on a disposable host, NOT committed to the repo (Pitfall 6: spike `.sys` must not land in main). | Confirm `.sys` is VM-local; commit only the `drivers/nono-fltmgr/` source scaffold, not the binary |

**The canonical question (rename phases):** N/A — Phase 63 performs no rename. The macOS audit only *inventories* upstream commits; the *cherry-picks that rewrite fork code* are Phase 64.

## Common Pitfalls

> Full catalogue in `.planning/research/PITFALLS.md` (Pitfalls 1–11). Below are the ones that bind on **Phase 63 specifically** (most kernel-runtime pitfalls bite in Phase 64 when the driver actually loads, but several gate Phase 63 deliverables).

### Pitfall A: Standard vs Trusted Launch Azure VM (binds D-02; PITFALLS Pitfall 4 scenario 1)
**What goes wrong:** A Trusted-Launch Azure VM enforces Secure Boot; `bcdedit /set testsigning on` is rejected ("The value is protected by Secure Boot policy"). The VM is then useless for Phase 64 driver loading and the SC1 TESTSIGNING artifact can't be captured.
**Why it happens:** Azure's default and most-prominent VM security type in the portal is Trusted Launch; it's easy to accept the default.
**How to avoid:** Provision with **Standard** security type (D-02). For Gen2, also disable Secure Boot. Verify with `bcdedit /enum all` showing TESTSIGNING-capable and `msinfo32` showing Secure Boot Off / HVCI off — these captures ARE the SC1 artifact (D-03).
**Warning signs:** `bcdedit /set testsigning on` returns the Secure-Boot-policy error; `msinfo32` shows Secure Boot State = On.

### Pitfall B: HVCI silently rejects test-signed drivers (PITFALLS Pitfall 4 scenario 2)
**What goes wrong:** Even with TESTSIGNING on, HVCI (Memory Integrity) silently rejects a test-signed driver at load.
**Phase 63 relevance:** Phase 63 only **compiles** the `.sys` (SC2) — it does NOT load it, so HVCI doesn't block the Phase 63 deliverable. BUT the SC1 `msinfo32` capture must record HVCI state so Phase 64 doesn't hit a silent-load wall. Document HVCI = Off (Standard-security-type VMs default to HVCI off).
**How to avoid:** Capture HVCI state in the SC1 `msinfo32` artifact now; if On, disable it before Phase 64.

### Pitfall C: Spike scope creep into production (PITFALLS Pitfall 6) — binds the whole phase
**What goes wrong:** The "while we're here" instinct adds the pre-create callback, comm port, or IPC protocol to the Phase 63 skeleton — converting groundwork into a half-built production driver.
**How to avoid:** The Phase 63 plan MUST carry an explicit out-of-scope list (CONTEXT § Phase Boundary "Not in this phase"). The `.sys` skeleton compiles and does nothing else. Warning sign: the scaffold's `.c` grows a `FLT_OPERATION_REGISTRATION` array with a real `IRP_MJ_CREATE` pre-op body, or a `FltCreateCommunicationPort` call.

### Pitfall D: Altitude in the AV range (D-08; PITFALLS Pitfall 5)
**What goes wrong:** Recording (or later using) an altitude in `320000–329998` collides with installed EDR — fails registration or blinds the EDR.
**Phase 63 relevance:** Phase 63 only *records the band + constraint* (the exact number is Phase 64). The DESIGN.md and altitude email must state the FSFilter Activity Monitor band (360000–389999) and the AV-range avoidance.
**How to avoid:** Per D-08, record band 360000–389999, AV range 320000–329998 to avoid. Note STACK.md mentions 370020 (nullFilter default) and the 360000–389998/400000–409998 ranges — Phase 64 enumerates `fltmc filters` on the actual VM to pick a non-colliding number.

### Pitfall E: macOS audit on a stale local tag set (binds SC4)
**What goes wrong:** Running the audit on `v0.57.0..v0.61.2` without first fetching `v0.61.2` — which is **NOT in the local object store** (`3e605f27`, confirmed absent 2026-06-06 `[VERIFIED: git ls-remote / git cat-file]`). The range silently truncates to the last reachable local tag (v0.61.1), missing any v0.61.2 commit.
**How to avoid:** First plan step of Track B is `git fetch upstream --tags`. Record `upstream_head_at_audit` + `refetch_date` in the ledger frontmatter (Phase 54 precedent). Verify `git cat-file -t 3e605f27` returns `commit` before running the drift tool.

### Pitfall F: Re-using Phase 54's C14 `won't-sync` verdict (binds D-13)
**What goes wrong:** Phase 54's ledger (`54-DIVERGENCE-LEDGER.md` cluster C14) dispositioned `8f84d45`/`362ada2`/`8f1b0b7` as **`won't-sync`** ("unix/macOS-only N/A per REQUIREMENTS § Out of Scope") because v2.7's milestone was Windows-only. Phase 63 (D-13) overrides this to **`will-sync`** because v2.10's scope is explicitly macOS parity. A copy-paste of Phase 54's disposition would silently re-defer the exact commits this milestone exists to absorb.
**How to avoid:** The new ledger must (a) disposition the three P1 commits `will-sync`, and (b) include an explicit note that this **supersedes Phase 54 C14** with the scope-change rationale.

## Code Examples

> These are the **skeleton** scaffold targets for SC2 (compile-to-`.sys`). They follow Microsoft's `nullFilter` sample structure. Phase 63 needs only enough to compile; the operation-callback body stays empty/stub (Phase 64 fills it).

### Skeleton `DriverEntry` + registration + unload (`nono-fltmgr.c`)
```c
// Source: structure mirrors github.com/microsoft/Windows-driver-samples
//         filesys/miniFilter/nullFilter [CITED]
#include <fltKernel.h>

PFLT_FILTER gFilterHandle = NULL;

NTSTATUS
NonoFltUnload(_In_ FLT_FILTER_UNLOAD_FLAGS Flags)
{
    UNREFERENCED_PARAMETER(Flags);
    // PITFALLS Pitfall 3: a registered unload callback lets the driver
    // un-register cleanly so queued messages never block on user-mode exit.
    if (gFilterHandle != NULL) {
        FltUnregisterFilter(gFilterHandle);
        gFilterHandle = NULL;
    }
    return STATUS_SUCCESS;
}

// Phase 63: NO operation callbacks (empty array) — the pre-create IRP_MJ_CREATE
// pre-op, ring buffer, and FltSendMessage round-trip are Phase 64 (DRV-01/02).
CONST FLT_OPERATION_REGISTRATION Callbacks[] = {
    { IRP_MJ_OPERATION_END }
};

CONST FLT_REGISTRATION FilterRegistration = {
    sizeof(FLT_REGISTRATION),       // Size
    FLT_REGISTRATION_VERSION,       // Version
    0,                              // Flags
    NULL,                           // ContextRegistration
    Callbacks,                      // OperationRegistration (empty for skeleton)
    NonoFltUnload,                  // FilterUnloadCallback (Pitfall 3)
    NULL,                           // InstanceSetupCallback
    NULL,                           // InstanceQueryTeardownCallback
    NULL,                           // InstanceTeardownStartCallback
    NULL,                           // InstanceTeardownCompleteCallback
    NULL, NULL, NULL, NULL, NULL
};

NTSTATUS
DriverEntry(_In_ PDRIVER_OBJECT DriverObject, _In_ PUNICODE_STRING RegistryPath)
{
    UNREFERENCED_PARAMETER(RegistryPath);
    NTSTATUS status = FltRegisterFilter(DriverObject, &FilterRegistration, &gFilterHandle);
    if (NT_SUCCESS(status)) {
        status = FltStartFiltering(gFilterHandle);
        if (!NT_SUCCESS(status)) {
            FltUnregisterFilter(gFilterHandle);
        }
    }
    return status;
}
```

### Skeleton INF (`nono-fltmgr.inf`) — the load-order/altitude/start-type config
```ini
; Source: nullFilter.inf structure [CITED: microsoft/Windows-driver-samples]
[Version]
Signature   = "$Windows NT$"
Class       = "ActivityMonitor"                       ; FSFilter Activity Monitor class
ClassGuid   = {b86dff51-a31e-4bac-b3cf-e8cfe75c9fc2}  ; standard ActivityMonitor ClassGuid
Provider    = %ManufacturerName%
DriverVer   =
CatalogFile = nono-fltmgr.cat

[DefaultInstall.NTamd64]
OptionDesc  = %ServiceDescription%
CopyFiles   = MiniFilter.DriverFiles

[DefaultInstall.NTamd64.Services]
AddService  = %ServiceName%,,MiniFilter.Service

[MiniFilter.Service]
DisplayName    = %ServiceName%
Description    = %ServiceDescription%
ServiceBinary  = %12%\nono-fltmgr.sys           ; %12% = \Windows\System32\drivers
Dependencies   = "FltMgr"
ServiceType    = 2                               ; SERVICE_FILE_SYSTEM_DRIVER
StartType      = 3                               ; SERVICE_DEMAND_START (D-06 boot-loop safeguard)
ErrorControl   = 1                               ; SERVICE_ERROR_NORMAL
LoadOrderGroup = "FSFilter Activity Monitor"
AddReg         = MiniFilter.AddRegistry

[MiniFilter.AddRegistry]
HKR,"Instances","DefaultInstance",0x00000000,%DefaultInstance%
HKR,"Instances\"%Instance1.Name%,"Altitude",0x00000000,%Instance1.Altitude%
HKR,"Instances\"%Instance1.Name%,"Flags",0x00010001,%Instance1.Flags%

[Strings]
ManufacturerName    = "nono"
ServiceName         = "nono-fltmgr"
ServiceDescription  = "nono Gap 6b minifilter feasibility spike (TEST-SIGNED POC)"
DefaultInstance     = "nono-fltmgr Instance"
Instance1.Name      = "nono-fltmgr Instance"
Instance1.Altitude  = "370020"     ; PLACEHOLDER in FSFilter Activity Monitor band (D-08);
                                   ; final number = Phase 64 after `fltmc filters` enum. MUST NOT be 320000-329998 (AV range).
Instance1.Flags     = 0x0
```
> The `Altitude` is a **placeholder** (D-08): Phase 63 records the band + AV-range constraint; Phase 64 picks the exact non-colliding number after enumerating `fltmc filters` on the VM and (ideally) after the official Microsoft assignment lands.

### MSBuild invocation that compiles the project to a `.sys` (SC2 proof)
```powershell
# Source: WDK driver projects build with msbuild against the Driver platform toolset.
# Run from a Developer/EWDK command prompt on the VM.
msbuild nono-fltmgr.vcxproj /p:Configuration=Release /p:Platform=x64
# Output: x64\Release\nono-fltmgr.sys  ← SC2 success = this file exists with no build errors.
# (Inside an EWDK environment, LaunchBuildEnv.cmd sets the toolchain; msbuild is then on PATH.)
```
> The `.vcxproj` must set `ConfigurationType = Driver` and `DriverType = WDM` (a minifilter is a WDM-class file-system filter, NOT KMDF) and import the WDK `.props`/`.targets`. Copy these from the `nullFilter.vcxproj` sample rather than hand-authoring `[CITED: microsoft/Windows-driver-samples]`.

### Track B — exact git inventory commands (verified to work locally 2026-06-06)
```bash
# 0. MANDATORY precondition — v0.61.2 (3e605f27) is NOT in the local store.
git fetch upstream --tags
git cat-file -t 3e605f27   # must print "commit" before proceeding (Pitfall E)

# 1. Authoritative reproducible inventory via the pinned drift tool (Phase 42/47/54 precedent)
make check-upstream-drift ARGS="--from v0.57.0 --to v0.61.2 --format json"
#   Windows-host fallback: bash scripts/check-upstream-drift.sh --from v0.57.0 --to v0.61.2 --format json
#   Note the drift tool EXCLUDES *_windows.rs + exec_strategy_windows/ (D-11 filter) — fine for a macOS audit,
#   but means the macos-only column is derived from the tool's category data + per-commit diff-inspect, not the filter.

# 2. macOS-surface narrowing (the audit's primary signal)
git log --no-merges --format='%h %s' v0.57.0..v0.61.2 -- crates/nono/src/sandbox/macos.rs
git log --no-merges --format='%h %s' v0.57.0..v0.61.2 -- crates/nono-cli/src/sandbox_prepare.rs
git log --no-merges --format='%h %s' v0.57.0..v0.61.2 --grep='macos\|seatbelt\|symlink\|Keychain\|sandbox' -i

# 3. Per-commit diff-inspect (D-11 / Pattern 2) — run for every will-sync candidate
git show <sha> --stat
git show <sha>                  # inspect pub use / pub(crate) re-exports AND function-call cross-cluster deps

# 4. Confirm a candidate is macos-relevant even when subject doesn't say "macos"
git show c6730e43 --stat        # java_runtime: ALSO touches crates/nono/src/sandbox/macos.rs (2 lines) — macos-only=yes
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Phase 54 C14: macOS Seatbelt commits `won't-sync` (Windows-only milestone) | Phase 63 D-13: P1 macOS commits `will-sync` (macOS-parity milestone) | v2.10 scope decision (2026-06-06) | The new ledger supersedes Phase 54 C14; the three P1 commits become Phase 64 cherry-pick targets |
| `sandbox-exec` for macOS profiles | `sandbox_init()` FFI (private but stable) | Long-standing in fork | `sandbox-exec` migration is explicitly OUT OF SCOPE (REQUIREMENTS) — no public replacement API |
| `windows-drivers-rs` Rust kernel driver | C/C++ WDK minifilter | This milestone | Rust kernel framework ruled out (early-stage, KMDF-v1.33-only) — the `.sys` is C |

**Deprecated/outdated:**
- Phase 54 C14 `won't-sync` disposition for `8f84d45`/`362ada2`/`8f1b0b7`: **superseded** by D-13. The new ledger must say so explicitly.
- The `fe233db4` reference in STACK.md C1: this is a **merge commit** (PR #680) — the actual non-merge fix commits are `362ada22` + `8f1b0b74`. The drift tool excludes merges; the ledger should disposition the two non-merge SHAs, not the merge.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Azure Standard-security-type Gen2 Win11 VM with Secure Boot disabled supports `bcdedit /set testsigning on` after reboot | Standard Stack / Pitfall A | If wrong, SC1 TESTSIGNING capture fails; mitigation = AWS `.metal` fallback (D-02). Low risk — Standard security type is explicitly the non-SB-enforcing type. |
| A2 | WDK 28000.1761 + VS 2026 (or 26100.6584 + VS 2022) installs cleanly on the current WDK-recommended Win11 Azure image (D-05) | Standard Stack | If the exact image pairing differs, use EWDK ISO (self-contained) — lower-risk fallback already in the stack. |
| A3 | `c6730e43` (java_runtime) is macОS-relevant because it touches `sandbox/macos.rs` (2 lines) — should appear in the ledger with `macos-only=yes` | Code Examples / Track B | Verified via `git show c6730e43 --stat` `[VERIFIED]`. The *disposition* (will-sync vs won't-sync — java group is cross-platform policy) is the auditor's Phase 63 call. |
| A4 | The exact count of macOS-relevant commits in `v0.57.0..v0.61.2` is not yet known (v0.61.2 not fetched) | Track B | Full local range `v0.57.0..v0.61.1` = 89 non-merge commits `[VERIFIED]`; v0.61.2 adds a small delta. The ledger's authoritative count comes from running the drift tool AFTER `git fetch upstream --tags`. |
| A5 | The Microsoft altitude request email goes to `fsfcomm@microsoft.com` and expects a ~30-business-day turnaround | Track A altitude | Per ROADMAP SC3 + CONTEXT D-07/D-08. The email content (company/contact/driver-purpose/requested band) is drafted by the executor; the user sends it. If the address/process changed, the WebFetch of Microsoft's current altitude-request page (Open Questions Q1) resolves it at plan-time. |
| A6 | A minifilter is a WDM-class FS filter (`DriverType=WDM`), not KMDF | Code Examples | Standard for FltMgr minifilters; the `nullFilter` sample uses this. Low risk. |

**Confirmation path:** A1/A2/A5 are the user-facing assumptions worth confirming at discuss/plan time. A3/A4/A6 are codebase/spec-verified or low-risk.

## Open Questions

1. **Exact Microsoft altitude-request submission mechanism + current address**
   - What we know: Historically `fsfcomm@microsoft.com`, ~30 business days, free (ROADMAP SC3, CONTEXT D-07, PITFALLS Pitfall 5). The request states company, contact, driver purpose, and requested altitude band.
   - What's unclear: Whether Microsoft has moved altitude requests to a web form (Hardware Dev Center / Partner Center) since training cutoff.
   - Recommendation: At plan-time, WebFetch `learn.microsoft.com/windows-hardware/drivers/ifs/allocated-altitudes` and the "minifilter altitude request" page to confirm the current channel before the executor drafts the email. Fall back to `fsfcomm@microsoft.com` if the page still lists it.

2. **Which exact Azure VM size + image string is the "current WDK-recommended Win11 pairing" (D-05)**
   - What we know: Standard security type, Gen2-with-SB-off or Gen1, Win11 (D-02/D-05).
   - What's unclear: The precise Azure Marketplace image URN and a VM size with enough resources for VS 2026 + WDK.
   - Recommendation: The plan should pick a general-purpose size (e.g., a D-series v5 with ≥4 vCPU / ≥16 GB) and a Win11 Pro Gen2 image, then disable Secure Boot at create time (`--security-type Standard`). Capture the exact URN used in the SC1 artifact for reproducibility. Confirm against current `az vm image list` at plan-time.

3. **Does v0.61.2 contain any macOS-relevant commit beyond v0.61.1?**
   - What we know: v0.61.2 (`3e605f27`) is a patch release on the upstream remote; not yet fetched locally.
   - What's unclear: Its commit delta vs v0.61.1.
   - Recommendation: `git fetch upstream --tags` then `git log --no-merges v0.61.1..v0.61.2` as the first Track B audit step; fold any macOS-relevant commit into the ledger.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `git` + upstream remote | Track B audit | ✓ | upstream = github.com/always-further/nono | — |
| upstream tag `v0.61.2` (`3e605f27`) | Track B range | ✗ (not in local store) | remote-only | `git fetch upstream --tags` (MANDATORY first step) |
| drift tool (`check-upstream-drift.{sh,ps1}`) | Track B reproducible inventory | ✓ | pinned sha `0834aa664f…` | — |
| P1/P2 commits `8f84d454`/`362ada22`/`8f1b0b74`/`729697c2` | Track B disposition | ✓ (all in local object store `[VERIFIED]`) | — | — |
| Azure subscription + `az` CLI | Track A VM provisioning | ✗ (user-side, off-host) | — | AWS `.metal` (D-02 fallback); user has both per CONTEXT |
| WDK 28000.1761 + VS 2026 / EWDK | Track A `.sys` compile (on VM) | ✗ (installed on the VM, not the dev host) | 28000.1761 / 18.3.0 | WDK 26100.6584 + VS 2022 (D-05) |
| macOS host | NOT required in Phase 63 | n/a | — | macOS live re-validation is Phase 65, not 63 |

**Missing dependencies with no fallback:** None that block Phase 63. The Azure subscription is user-provided (D-02 confirms the user has Azure access); the WDK toolchain installs on the disposable VM.
**Missing dependencies with fallback:** `v0.61.2` (→ `git fetch`); Azure (→ AWS `.metal`); WDK 28000/VS2026 (→ 26100/VS2022 or EWDK ISO).

## Validation Architecture

> `workflow.nyquist_validation` is not explicitly false in this project's config history, so this section is included. Phase 63's deliverables are **artifacts** (a compiling `.sys`, captured VM state, a design doc, a ledger) and **one human-gated action** (the altitude email) — not Rust code with unit tests. Validation is therefore artifact-existence + content-assertion + a single compile gate, not `cargo test`.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None new. Track A = MSBuild compile gate (on VM) + artifact capture; Track B = ledger-completeness assertion. Existing Rust `cargo test` is untouched (no Rust code changes in Phase 63). |
| Config file | none — see Wave 0 (no new test harness needed) |
| Quick run command (Track A) | `msbuild nono-fltmgr.vcxproj /p:Configuration=Release /p:Platform=x64` (exit 0 + `.sys` produced = SC2) |
| Quick run command (Track B) | `make check-upstream-drift ARGS="--from v0.57.0 --to v0.61.2 --format json"` (reproduces the inventory) |
| Full suite command | `make ci` on the dev host — must stay green (Phase 63 adds NO Rust code, so this is a no-regression check, not a new gate) |

### Phase Requirements → Test Map
| Req / SC | Behavior | Test Type | Verification Command / Method | Artifact Exists? |
|----------|----------|-----------|-------------------------------|------------------|
| SC1 | HVCI/Secure Boot state documented; TESTSIGNING state recorded | manual capture | `msinfo32` export + `bcdedit /enum all` captured on the VM | ❌ Wave 0 (produced in-phase) |
| SC2 / DRV-03(partial) | `drivers/nono-fltmgr/` compiles to `.sys` with no errors | compile gate | `msbuild …/p:Configuration=Release /p:Platform=x64` exits 0; `x64\Release\nono-fltmgr.sys` exists | ❌ Wave 0 |
| SC3 | Design doc specifies ring-buffer+worker-thread IPC, forbids `ZwCreateFile`, mandates finite `FltSendMessage` timeout, records altitude band + Microsoft request status | content assertion | Grep `drivers/nono-fltmgr/DESIGN.md` for each mandated element (ring buffer, NonPagedPoolNx, IRQL assert, no-ZwCreateFile, FltSendMessage timeout, FSFilter band, AV-range avoidance, request date) | ❌ Wave 0 |
| SC3 | Altitude request sent; date + `pending` recorded | human-gated artifact | Email sent by user; request-status artifact records date + pending | ❌ Wave 0 (human action) |
| SC4 / MACOS-01 | Ledger covers `v0.57.0..v0.61.2`, every macOS commit dispositioned, `macos-only` column, diff-inspect notes, P1×3 = will-sync | completeness assertion | Ledger frontmatter (range, tool sha, upstream_head, refetch_date) + every drift-tool commit dispositioned + the three P1 SHAs present and `will-sync` + Phase-54-C14 supersession note | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** Track A — re-run `msbuild` after each scaffold file change (compile must stay green). Track B — re-run the drift tool after the fetch to confirm range/count stable.
- **Per wave merge:** `make ci` on the dev host (no-regression; Phase 63 touches no Rust).
- **Phase gate:** All four SC artifacts exist and pass their content assertions before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `drivers/nono-fltmgr/nono-fltmgr.vcxproj` + `.inf` + `.c` — the scaffold itself (SC2)
- [ ] `drivers/nono-fltmgr/DESIGN.md` — the pre-code gate (SC3); `.planning/adr/` pointer stub (D-09)
- [ ] SC1 capture artifacts (`msinfo32` export, `bcdedit /enum all`) stored as reproducibility evidence
- [ ] Altitude-request status artifact (date + pending) — human-gated
- [ ] `<phase>-DIVERGENCE-LEDGER.md` — Track B deliverable (SC4)
- [ ] Framework install: **none** — no new test framework; Phase 63 adds no Rust code

## Security Domain

> `security_enforcement` is treated as enabled (absent = enabled). nono is a security-critical sandbox codebase (CLAUDE.md § Security Considerations is NON-NEGOTIABLE). Phase 63 introduces no runtime code path, but the design doc IS a security artifact (BSOD-avoidance = availability/integrity of the host).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture/Design | yes | The DESIGN.md is the threat-model artifact for the future driver: fail-open-on-timeout for the spike (availability), explicit production-revisit note (PITFALLS Pitfall 3) |
| V5 Input Validation | partial (future) | The future kernel↔user IPC `#[repr(C)]` struct (Phase 64) needs a static layout assertion — flagged in DESIGN.md, built in Phase 64 |
| V6 Cryptography | no | Phase 63 adds no crypto. Test-signing cert (Phase 64) is VM-local POC material, never production. |
| V10 Malicious Code | yes | Pitfall 6 (scope creep) + the "spike `.sys` must not land in main" rule are V10 hygiene; the skeleton must do no file I/O (Pitfall 2) |
| V14 Configuration | yes | `SERVICE_DEMAND_START` (D-06) is the fail-secure boot-loop safeguard; TESTSIGNING is VM-only, never the dev host |

### Known Threat Patterns for {kernel-driver groundwork + Seatbelt audit}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Driver-originated recursive file I/O → stack-overflow BSOD | Denial of Service | DESIGN.md forbids `ZwCreateFile`/`NtCreateFile`; `FltCreateFile`-on-own-instance only (PITFALLS Pitfall 2) |
| Infinite `FltSendMessage` wait → host hang | Denial of Service | Finite timeout (≈500 ms) → `STATUS_TIMEOUT`; fail-open for the spike (PITFALLS Pitfall 3) |
| IRQL violation in callback → BSOD | Denial of Service | `NonPagedPoolNx`, no locks across `FltSendMessage`, `NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL)` (PITFALLS Pitfall 1) |
| Altitude collides with EDR → EDR blinded / driver fails | Tampering / Elevation | FSFilter Activity Monitor band; NEVER AV range 320000–329998; official Microsoft assignment (PITFALLS Pitfall 5) |
| Seatbelt deny silently overridden by later allow (last-match-wins) | Information Disclosure | The `8f84d454` P1 fix moves platform denies AFTER write allows; Phase 64 asserts ordering by unit test (PITFALLS Pitfall 10) — Phase 63 *dispositions* it `will-sync` |
| macOS `/etc` vs `/private/etc` symlink deny bypass | Information Disclosure | `362ada22`/`8f1b0b74` preserve symlink + canonical paths; the fork's `path_filters_for_cap` already dual-emits — Phase 64 verifies (PITFALLS Pitfall 11) |

## Sources

### Primary (HIGH confidence — codebase + git, verified this session)
- `crates/nono/src/sandbox/macos.rs` — Seatbelt `generate_profile`; platform-rules ordering at the read/write boundary (the `8f84d454` target site); dual-path symlink emission in `path_filters_for_cap`/`collect_parent_dirs` `[VERIFIED: Read]`
- `crates/nono-cli/src/policy.rs` — `add_platform_rule` call sites (L448-449 unlink, L464-472 symlink_pairs, L692-704 deny groups); `unsafe_macos_seatbelt_rules` — the shared profile-emission deps D-11 requires inspecting `[VERIFIED: Read/grep]`
- `crates/nono-cli/src/sandbox_prepare.rs` — `resolved_workdir` (L438+) + `cwd_canonical`/`pending_cwd_access_request` (L456-474) — the `362ada22`/`8f1b0b74` target site; fork currently LACKS the `is_relative()` absolute-path fix `[VERIFIED: grep + commit diff]`
- `git show 8f84d454 / 362ada22 / 8f1b0b74 / c6730e43` — P1 diffs + java_runtime macos.rs touch `[VERIFIED]`
- `git ls-remote --tags upstream` + `git cat-file -t 3e605f27` — v0.61.2 present on remote, ABSENT in local store `[VERIFIED]`
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` — column shape, disposition vocabulary, C14 prior `won't-sync` verdict to supersede `[VERIFIED: Read]`
- `.planning/phases/42-upst5-audit/DIVERGENCE-LEDGER.md` — the `windows-touch`-column template MACOS-01 mirrors `[VERIFIED: Read]`

### Primary (HIGH confidence — milestone research, MSDN-cited)
- `.planning/research/STACK.md` — WDK/VS/SDK versions, FltMgr API surface, test-signing pipeline, EWDK fallback, `nullFilter` altitude/load-order `[CITED]`
- `.planning/research/PITFALLS.md` — Pitfalls 1–11 (kernel BSOD triad, scope creep, altitude/EDR, Seatbelt ordering, symlink drift) `[CITED]`
- `.planning/REQUIREMENTS.md` / `.planning/ROADMAP.md` § Phase 63 — DRV-03/MACOS-01 acceptance, SC1–4 `[VERIFIED: Read]`

### Secondary (MEDIUM — cited Microsoft docs, confirm currency at plan-time)
- `[CITED: github.com/microsoft/Windows-driver-samples/tree/main/filesys/miniFilter/nullFilter]` — minimal minifilter `.vcxproj`/`.inf`/`.c` structure
- `[CITED: learn.microsoft.com/windows-hardware/drivers/ifs/load-order-groups-and-altitudes-for-minifilter-drivers]` — altitude bands
- `[CITED: learn.microsoft.com/windows-hardware/drivers/install/test-signing]` — test-sign pipeline ordering

### Tertiary (LOW — verify at plan-time)
- Microsoft altitude-request submission channel (`fsfcomm@microsoft.com` vs web form) — Open Question 1
- Exact Azure Win11 image URN + VM size for the WDK-recommended pairing — Open Question 2

## Metadata

**Confidence breakdown:**
- Standard stack (Track A toolchain): HIGH — versions from STACK.md (MSDN-verified); scaffold structure from the canonical `nullFilter` sample
- Architecture / scaffold: HIGH — skeleton mirrors a known-good Microsoft sample; compile-only scope keeps it minimal
- Track B audit method: HIGH — git commands verified to run this session; all four P1/P2 commits confirmed in local store; fork target sites diff-confirmed
- Pitfalls: HIGH — sourced from the milestone PITFALLS.md (codebase + MSDN cross-referenced)
- Altitude-request mechanics + exact Azure image: MEDIUM/LOW — flagged as Open Questions for plan-time WebFetch

**Research date:** 2026-06-06
**Valid until:** 2026-07-06 (30 days) — EXCEPT the Azure image URN and Microsoft altitude-request channel (verify at plan-time; these drift faster) and the v0.61.2 commit delta (fetch at audit start).
