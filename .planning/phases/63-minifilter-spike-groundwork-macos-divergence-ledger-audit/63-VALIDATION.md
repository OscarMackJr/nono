---
phase: 63
slug: minifilter-spike-groundwork-macos-divergence-ledger-audit
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-06
---

# Phase 63 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Phase 63's deliverables are **artifacts** (a compiling `.sys`, captured VM state, a design
> doc, a divergence ledger) plus **one human-gated action** (the Microsoft altitude email).
> There is NO new Rust code, so validation is artifact-existence + content-assertion + a single
> MSBuild compile gate — not `cargo test`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None new. Track A = MSBuild compile gate (on the test-signing VM) + artifact capture; Track B = ledger-completeness assertion. Existing Rust `cargo test` is untouched (no Rust code changes). |
| **Config file** | none — no new test harness needed |
| **Quick run command (Track A)** | `msbuild nono-fltmgr.vcxproj /p:Configuration=Release /p:Platform=x64` (exit 0 + `.sys` produced = SC2) |
| **Quick run command (Track B)** | reproduce the macOS commit inventory for `v0.57.0..v0.61.2` (drift tool / `git log`) after `git fetch upstream --tags` |
| **Full suite command** | `make ci` on the dev host — no-regression check only (Phase 63 adds NO Rust code) |
| **Estimated runtime** | Track A compile ~30–90s on VM; Track B inventory <30s; `make ci` per existing baseline |

---

## Sampling Rate

- **After every task commit (Track A):** Re-run `msbuild` after each scaffold file change — compile must stay green.
- **After every task commit (Track B):** Re-run the inventory after the upstream fetch — confirm range/count stable (guards against the silently-truncated-range landmine).
- **After every plan wave:** `make ci` on the dev host (no-regression; Phase 63 touches no Rust).
- **Before `/gsd:verify-work`:** All four SC artifacts exist and pass their content assertions.
- **Max feedback latency:** ~90 seconds (VM compile gate)

---

## Per-Task Verification Map

> Task IDs are assigned at plan-time. This phase verifies **artifacts**, not unit tests — each row
> is an artifact-existence or content-assertion check the executor runs after the producing task.

| SC / Req | Track | Behavior | Test Type | Verification Command / Method | Status |
|----------|-------|----------|-----------|-------------------------------|--------|
| SC1 | A | HVCI/Secure Boot state documented; TESTSIGNING state recorded | manual capture | `msinfo32` export + `bcdedit /enum all` captured on the VM, stored as reproducibility artifacts | ⬜ pending |
| SC2 / DRV-03(partial) | A | `drivers/nono-fltmgr/` compiles to `.sys` with no errors | compile gate | `msbuild …/p:Configuration=Release /p:Platform=x64` exits 0; `x64\Release\nono-fltmgr.sys` exists | ⬜ pending |
| SC3 | A | Design doc specifies ring-buffer+worker-thread IPC, forbids `ZwCreateFile`/`NtCreateFile`, mandates finite `FltSendMessage` timeout, records altitude band + Microsoft request status | content assertion | Grep `drivers/nono-fltmgr/DESIGN.md` for each mandated element (ring buffer, NonPagedPoolNx, IRQL assert, no-ZwCreateFile, FltSendMessage timeout, FSFilter band 360000–389999, AV-range 320000–329998 avoidance, request date) | ⬜ pending |
| SC3 | A | Altitude request sent; date + `pending` recorded | human-gated artifact | Email sent by user to `fsfcomm@microsoft.com`; status artifact records date + `pending` | ⬜ pending |
| SC4 / MACOS-01 | B | Ledger covers `v0.57.0..v0.61.2`, every macOS commit dispositioned, `macos-only` column, diff-inspect notes, P1×3 = will-sync | completeness assertion | Ledger frontmatter (range, upstream_head, refetch_date) + every macOS-relevant commit dispositioned + the three P1 SHAs (`8f84d454`, `362ada22`, `8f1b0b74`) present and `will-sync` + explicit Phase-54-C14 supersession note | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

All Phase 63 deliverables are produced in-phase (the artifacts do not exist yet):

- [ ] `drivers/nono-fltmgr/nono-fltmgr.vcxproj` + `.inf` + skeleton `.c` — the scaffold (SC2)
- [ ] `drivers/nono-fltmgr/DESIGN.md` — the pre-code gate (SC3) + `.planning/adr/` pointer stub (D-09)
- [ ] SC1 capture artifacts (`msinfo32` export, `bcdedit /enum all`) stored as reproducibility evidence
- [ ] Altitude-request status artifact (date + `pending`) — human-gated (D-07)
- [ ] `63-DIVERGENCE-LEDGER.md` — Track B deliverable (SC4)
- [ ] **Framework install: none** — no new test framework; Phase 63 adds no Rust code

*Mandatory first Track B step: `git fetch upstream --tags` — `v0.61.2` (`3e605f27`) is NOT in the
local object store and the range silently truncates to v0.61.1 without it (RESEARCH landmine).*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Azure VM provisioned (Standard security type, Secure Boot OFF, TESTSIGNING on) | SC1 / D-01..D-03 | Requires a live cloud VM and reboot; not reproducible in CI | Provision per RESEARCH; run `bcdedit /set testsigning on` + reboot; capture `msinfo32` + `bcdedit /enum all` |
| `.sys` compiles on the VM | SC2 | WDK toolchain lives on the VM, not the dev host | Run the `msbuild` command on the VM; confirm exit 0 + `.sys` artifact |
| Microsoft altitude email sent | SC3 / D-07 | Human action — the email needs company/contact/driver-purpose details only the user can supply | User sends the planner-drafted email to `fsfcomm@microsoft.com`; reports the send date |

---

## Validation Sign-Off

- [ ] All four success-criteria artifacts have a defined existence/content verification
- [ ] Sampling continuity: compile gate (Track A) and inventory re-run (Track B) after each producing task
- [ ] Wave 0 covers all artifacts (none pre-exist)
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s (VM compile gate)
- [ ] `nyquist_compliant: true` set in frontmatter (set by planner once tasks map to these checks)

**Approval:** pending
