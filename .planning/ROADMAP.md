---
milestone: v2.11
milestone_name: Clean-Host Distribution Cleanup + UPST8
status: active
created: 2026-06-11
last_updated: 2026-06-11
granularity: standard
---

# Roadmap — nono

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 01-12 (shipped 2026-03-31) — see [`milestones/v1.0-*`](milestones/)
- ✅ **v2.0 Windows Gap Closure** — Phases 13-18 — see [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md)
- ✅ **v2.1 Resource Limits / Extended IPC / Attach-Streaming** — see [`milestones/v2.1-ROADMAP.md`](milestones/v2.1-ROADMAP.md)
- ✅ **v2.2 Windows/macOS Parity Sweep** — see [`milestones/v2.2-ROADMAP.md`](milestones/v2.2-ROADMAP.md)
- ✅ **v2.3 Linux POC Unblock + Deferreds Closure** — see [`milestones/v2.3-ROADMAP.md`](milestones/v2.3-ROADMAP.md)
- ✅ **v2.4 Complete the Partial Ports + UPST4** — Phases 35, 36, 36.5, 39, 40 (shipped 2026-05-15) — see [`milestones/v2.4-ROADMAP.md`](milestones/v2.4-ROADMAP.md)
- ✅ **v2.5 Backlog Drain + UPST5** — Phases 37, 41, 42, 43 (shipped 2026-05-20) — see [`milestones/v2.5-ROADMAP.md`](milestones/v2.5-ROADMAP.md)
- ✅ **v2.6 UPST6 + v2.5 Drain** — Phases 44, 44.1, 45, 46, 47, 48, 49, 50 (shipped 2026-05-25) — see [`milestones/v2.6-ROADMAP.md`](milestones/v2.6-ROADMAP.md)
- ✅ **v2.7 Windows supervised-run hardening** — Phases 51, 52 (shipped 2026-05-26) — see [`milestones/v2.7-ROADMAP.md`](milestones/v2.7-ROADMAP.md)
- ✅ **v2.8 UPST7 + v2.7 Drain & Release** — Phases 53-59 (shipped 2026-06-06; tags `v2.8`+`v0.57.5`) — see [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md)
- ✅ **v2.9 Windows Sandbox-the-Tools — Confined Coding Loop** — Phases 60, 61, 62 (published as `v0.62.2` 2026-06-06) — see [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md)
- ✅ **v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity** — Phases 63-66 (shipped 2026-06-11; tag `v2.10`) — see [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md)
- 🚧 **v2.11 Clean-Host Distribution Cleanup + UPST8** — Phases 67-70 (active; started 2026-06-11)

## Phases

- [ ] **Phase 67: Clean-Host Windows Install** — The machine MSI installs to completion on a fresh Win11 host (VC++ runtime handled, service-start non-fatal) and the supervised/broker path works there via an interim, auditable cert-trust helper + docs.
- [ ] **Phase 68: macOS Resource-Limit Enforcement Fix** — `nono run --timeout` and `--max-processes` actually fire on a real macOS host (supervisor watchdog + `RLIMIT_NPROC`), re-validated with `NONO_RESL_HOST_VALIDATED=1`.
- [ ] **Phase 69: UPST8 Audit** — A DIVERGENCE-LEDGER audits the non-macOS slice of upstream `v0.60.0..v0.61.2` with per-commit dispositions, `windows-touch` column, and ADR-cadence review.
- [ ] **Phase 70: UPST8 Cherry-pick Sync** — The will-sync UPST8 commits are absorbed with D-19 trailers, Windows-only-files invariant preserved, cross-target clippy + full suite green.

## Phase Details

### Phase 67: Clean-Host Windows Install
**Goal**: An operator can install the public machine MSI to completion on a fresh Windows 11 host with no manual steps, and run the supervised/broker path there without manual `certmgr` work — the cert-independent half of "make the public release work out-of-the-box."
**Depends on**: Nothing (independent of Phases 68-70; can run in parallel)
**Requirements**: DIST-01, DIST-02, TRUST-01, TRUST-02
**Success Criteria** (what must be TRUE):
  1. A clean-host machine-MSI install completes (does NOT fail `1603` / `0xC0000135` STATUS_DLL_NOT_FOUND) on a fresh Win11 host with no VC++ x64 runtime pre-installed — the CRT dependency is satisfied structurally (bundled redist / `+crt-static` / declared-and-checked prereq, chosen at plan time).
  2. A `nono-wfp-service` start failure during install does NOT roll back the product — the install completes and leaves a usable `nono.exe`; the clean-uninstall invariant (no orphaned WFP filters / service registration) is preserved.
  3. `nono setup --trust-broker` (or equivalent) imports the shipped code-signing cert into LocalMachine `Root` + `TrustedPublisher` so `nono run --profile claude-code` spawns the broker on a clean host with no manual `Import-Certificate`; the helper states what it trusts and why and never silently weakens the D-32-12 gate for an untrusted binary.
  4. The clean-host trust limitation and the supported interim path are documented (e.g. `docs/cli/development/windows-signing-guide.mdx`), the cert + import step ship with the release, and the doc plainly states public releases use a self-signed POC cert for the supervised path until publicly-trusted signing lands (pointing at the `--trust-broker` helper as the supported path).
**Host gate**: Clean Windows 11 host (no VC++ runtime, no pre-trusted cert) for the install + broker-spawn UAT. Must use the production-signed MSI, not a dev-layout binary (the broker trust gate only fires from a signed Program-Files install).
**Plans**: TBD
**UI hint**: no

### Phase 68: macOS Resource-Limit Enforcement Fix
**Goal**: `nono run --timeout` and `--max-processes` deliver real enforcement on a real macOS host — fixing the nono supervisor-watchdog / `setrlimit` bug surfaced as the Phase 65 gate-65-A "A5" finding (REQ-RESL-NIX-03 defect), not merely re-gating the tests.
**Depends on**: Nothing (independent of Phases 67/69/70; can run in parallel)
**Requirements**: RESL-MAC-01, RESL-MAC-02
**Success Criteria** (what must be TRUE):
  1. `nono run --timeout <D>` SIGKILLs the child at the deadline on a real macOS host — the supervisor wall-clock watchdog fires (it is nono's own cross-platform code; non-firing is a nono bug, fixed here).
  2. `nono run --max-processes <N>` makes the child's `fork()` fail (EAGAIN) past the cap on a real macOS host, via `setrlimit(RLIMIT_NPROC)` applied before `exec` (accounting for macOS `RLIMIT_NPROC` counting all per-UID processes — may need a different bounding strategy than Linux `pids.max`).
  3. `macos_timeout_kills_at_deadline` and `macos_max_processes_blocks_on_rlimit_nproc` both PASS with `NONO_RESL_HOST_VALIDATED=1` on a real macOS host.
  4. The fix touches cfg-gated Unix code, so cross-target clippy (Linux + macOS) is verified per `.planning/templates/cross-target-verify-checklist.md` and the macOS CI build leg stays green.
**Host gate**: Real macOS host for the `NONO_RESL_HOST_VALIDATED=1` enforcement re-validation (CI runners can't validate — they hang; the two tests stay env-gated off the runner).
**Plans**: 1 plan
Plans:
- [ ] 68-01-PLAN.md � macOS resl enforcement fix (setpgid + RLIMIT_NPROC): both Direct and Supervised paths, uid_process_count helper, host UAT + cross-target CI deferred
**UI hint**: no

### Phase 69: UPST8 Audit
**Goal**: A DIVERGENCE-LEDGER inventories the non-macOS slice of upstream `always-further/nono` `v0.60.0..v0.61.2` so the will-sync set is known before any cherry-pick — mirroring the Phase 54 audit shape.
**Depends on**: Phase 55 (UPST7 cherry-pick wave closed — the cadence rule preserves linear ordering; independent of Phases 67/68)
**Requirements**: UPST8-01
**Success Criteria** (what must be TRUE):
  1. `DIVERGENCE-LEDGER.md` audits `v0.60.0..v0.61.2` scoped to the non-macOS surface (the macOS slice was absorbed in v2.10), inventorying every relevant commit with a per-commit disposition (will-sync / fork-preserve / won't-sync / split).
  2. The ledger includes a `windows-touch` column and an ADR-cadence review per the Phase 33 Option A `continue` rule (does not silently supersede the Phase 33 ADR).
  3. A diff-inspect note records the re-export / cross-cluster cross-check per the `feedback_cluster_isolation_invalid` lesson (don't trust `--name-only` isolation).
  4. Upstream is re-fetched at audit-open and the head SHA + refetch date are recorded.
**Plans**: TBD
**UI hint**: no

### Phase 70: UPST8 Cherry-pick Sync
**Goal**: The will-sync UPST8 commits land on fork `main` with the fork's invariants preserved and the workspace green — mirroring the Phase 55 cherry-pick-wave shape.
**Depends on**: Phase 69 (audit dispositions drive the cherry-pick set)
**Requirements**: UPST8-02
**Success Criteria** (what must be TRUE):
  1. The will-sync upstream commits are cherry-picked with verbatim D-19 `Upstream-commit:` trailer blocks; D-20 manual replays are used where direct cherry-pick conflicts dominate.
  2. The Windows-only-files invariant holds (no `*_windows.rs` / `exec_strategy_windows/` drift from upstream) and the fork-divergence catalog is preserved.
  3. Cross-target clippy (Linux + macOS) is verified per `.planning/templates/cross-target-verify-checklist.md`.
  4. The full workspace test suite passes post-sync.
**Plans**: TBD
**UI hint**: no

<details>
<summary>✅ v2.8 UPST7 + v2.7 Drain & Release (Phases 53-59) — SHIPPED 2026-06-06</summary>

- [x] Phase 53: Release & Drain (3/4) — completed 2026-05-29 (shipped v0.57.5)
- [x] Phase 54: UPST7 Audit (1/1) — completed 2026-06-04
- [x] Phase 55: UPST7 Cherry-pick Wave (7/7) — completed 2026-06-05
- [x] Phase 56: Fine-grained Network Filtering (4/4) — completed 2026-06-05
- [x] Phase 57: Bitwarden Credential Source (1/1) — completed 2026-06-05
- [x] Phase 58: Session Lifecycle Hooks (3/3) — completed 2026-06-06
- [x] Phase 59: Supervisor IPC Robustness (3/3) — completed 2026-06-06

Audit: `tech_debt`, 10/10 reqs satisfied, 0 blockers. Full detail: [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md).

</details>

<details>
<summary>✅ v2.9 Windows Sandbox-the-Tools — Confined Coding Loop (Phases 60-62) — PUBLISHED as v0.62.2 2026-06-06</summary>

- [x] Phase 60: Confined Coding Loop (3/3) — completed 2026-05-29
- [x] Phase 61: Ship/Release v2.9 (4/4) — completed 2026-06-06 (published v0.62.2)
- [x] Phase 62: WFP kernel network enforcement — Windows supervised (13/13) — completed 2026-06-03

Separate initiative from UPST7 (builds on merged PR #4). The v0.62.0/v0.62.1 release attempts failed on two latent cfg-gated cross-target compile errors (E0716 + edition-2024 let-chain), fixed in `4de294e8`+`7bb7c7e3` → v0.62.2 published. Full detail: [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md).

</details>

<details>
<summary>✅ v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity (Phases 63-66) — SHIPPED 2026-06-11</summary>

- [x] Phase 63: Minifilter Spike Groundwork + macOS DIVERGENCE-LEDGER Audit (3/3) — completed 2026-06-08
- [x] Phase 64: Minifilter Spike Implementation + macOS P1 Cherry-pick Wave (5/5) — completed 2026-06-09
- [x] Phase 65: Minifilter ADR + macOS Live Re-validation (4/4) — completed 2026-06-11 (D-11c CI green; gate-65-A Seatbelt PASS; go/no-go ADR **Accepted** — No-go/Conditional-go)
- [x] Phase 66: WR-02 EDR HUMAN-UAT (1/1) — completed 2026-06-11 (**WR-02 CLOSED** under Sysmon+Defender EDR-proxy)

9/9 reqs satisfied (DRV-01..04, EDR-01..02, MACOS-01..03). DRV-PROD-01 (production driver) deferred to v2.11/v3.0 per ADR-65. Full detail: [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md).

</details>

## Phase Details (archived)

<details>
<summary>✅ v2.8 UPST7 + v2.7 Drain & Release — Phase Details (archived)</summary>

See [`milestones/v2.8-ROADMAP.md`](milestones/v2.8-ROADMAP.md) for full phase detail blocks.

</details>

<details>
<summary>✅ v2.9 Windows Sandbox-the-Tools — Phase Details (archived)</summary>

See [`milestones/v2.9-ROADMAP.md`](milestones/v2.9-ROADMAP.md) for full phase detail blocks.

</details>

<details>
<summary>✅ v2.10 Kernel-Driver Spike + EDR UAT + macOS Upstream Parity — Phase Details (archived)</summary>

See [`milestones/v2.10-ROADMAP.md`](milestones/v2.10-ROADMAP.md) for full phase detail blocks (Phases 63-66).

</details>

## Progress

v2.11 active (Phases 67-70). Phases 67 and 68 are independent and host-gated (clean Win11; real macOS) — runnable in parallel. Phase 69→70 is the UPST8 audit-then-sync pair (linear). Prior milestones (63-66) archived above.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 67. Clean-Host Windows Install | 0/TBD | Not started | - |
| 68. macOS Resource-Limit Enforcement Fix | 0/TBD | Not started | - |
| 69. UPST8 Audit | 0/TBD | Not started | - |
| 70. UPST8 Cherry-pick Sync | 0/TBD | Not started | - |

## Future Cycles

### Carried v2-deferred requirements (from v2.8)

These were defined but deferred during v2.8 (not yet milestone-scoped):

- **REQ-WSRH-AUDIT-01** — profile-wide audit of which heavy-runtime binaries hit the `WriteRestricted` gate.
- **REQ-RLS-ATTEST-01** — evaluate `actions/attest-build-provenance` vs the existing sigstore/TUF + Authenticode pipeline.
- **REQ-UPST-RESID-01** — residual v0.44–v0.57 macОS-learn-diagnostics refactors (`b5f0a3ab`, `bbdf7b85`, `wiring.rs`).
- **REQ-DENY-PREFLIGHT-01** — Linux-host-gated `validate_deny_overlaps` preflight investigation (security equivalence already proven).
- **REQ-UNDO-TOCTOU-01** — full fd-relative TOCTOU hardening of `validate_restore_target` (standalone security phase, ~2-3 wk).

### Deferred to the Enterprise Distribution Milestone (next milestone)

The big distribution effort is scoped after v2.11, gated on the incoming publicly-trusted cert:

- **DIST-SIGN-01** *(BLOCKED on incoming cert)* — publicly-trusted Authenticode signing (Azure Trusted Signing) so the broker gate passes with no manual cert trust — the real fix that supersedes the v2.11 `TRUST-01` interim helper.
- **DIST-SILENT-01 (SEED-001, P0)** — silent/headless/unattended install + GPO/SCCM/Intune packaging + machine-wide provisioning + auto-provisioned secure scratch space.
- **ENT-EGRESS-01 (SEED-002, P1)** — enterprise-policy-managed egress allowlists reconciling `nono-proxy` + `nono-wfp-service`.
- **ENT-SIEM-01 (SEED-003, P2)** — structured security-event telemetry to Event Log / Syslog for SIEM/EDR.
- **ENT-MULTI-01 (SEED-004, P3)** — multi-engine pluggability (confine any `AI_AGENT`-labeled token).
- **ENT-ATTEST-01 (SEED-005, P3)** — signed/attested policy overrides via the external ZT-Infra ledger.
- **DRV-PROD-01** *(gated No-go/Conditional-go per ADR-65)* — production EV/WHQL-signed Gap 6b minifilter.

## Next

**v2.11 is active.** Start with the independent, parallel-safe phases as hosts become available:
- `/gsd:plan-phase 67` — needs a clean Windows 11 host for the install + broker UAT (production-signed MSI, not dev-layout).
- `/gsd:plan-phase 68` — needs a real macOS host for the `NONO_RESL_HOST_VALIDATED=1` re-validation.
- `/gsd:plan-phase 69` then `70` — UPST8 audit-then-sync (host-agnostic; cross-target clippy via CI).

**Repo MUST stay PUBLIC** until Microsoft approves the minifilter altitude (verify no `build_notes/`/`.gsd/` staged before any push). Real publicly-trusted signing is cert-gated and OUT OF SCOPE this milestone.

## References

- `.planning/PROJECT.md` — project context + current state.
- `.planning/REQUIREMENTS.md` — v2.11 requirements (DIST-01/02, TRUST-01/02, RESL-MAC-01/02, UPST8-01/02) + traceability.
- `.planning/MILESTONES.md` — shipped milestone history (v1.0 → v2.10).
- `.planning/milestones/v2.10-REQUIREMENTS.md` — archived v2.10 requirements (DRV-01..04, EDR-01..02, MACOS-01..03).
- `.planning/research/SUMMARY.md` — HIGH-confidence research; build-order recommendations.
- `.planning/research/PITFALLS.md` — pitfall→phase ownership; cross-target drift guards.
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for cfg-gated Unix code (Phases 68, 70).
- `.planning/milestones/v2.8-ROADMAP.md` / `v2.9-ROADMAP.md` — archived v2.8/v2.9 (Phase 54/55 UPST audit-then-sync precedent for Phases 69/70).
