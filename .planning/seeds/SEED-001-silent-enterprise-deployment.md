---
id: SEED-001
status: dormant
planted: 2026-06-08
planted_during: v2.10 (Phase 63)
trigger_when: milestone scope includes enterprise deployment, MSI/MSIX, GPO/SCCM/Intune, headless install, or fleet provisioning
scope: large
priority: P0
---

# SEED-001: Headless Enterprise Deployment & Silent Installation Strategy

## Why This Matters

A corporate IT desk cannot manually configure local working directories or manage unsigned binaries across 500 engineer workstations. The current Windows setup is manual: copy JSON profile → check CWD permissions → run local PS1 bypasses. To position nono for enterprise adoption, deployment must become: **silent MSI → GPO/SCCM/Intune push → machine-wide service → invariant environment variables.**

This is the **P0 ("Deployment")** priority of the enterprise horizon — it unblocks every other enterprise feature because nothing ships to a fleet without it.

## When to Surface

**Trigger:** when a milestone targets enterprise packaging, silent/headless install, GPO/SCCM/Intune distribution, or fleet-scale provisioning.

This seed will surface during `/gsd:new-milestone` when the milestone scope matches.

## Scope Estimate

**Large** — but a *layer on top of existing work*, not greenfield. nono **already ships signed machine + user MSIs via WiX** (Phases 53, 61; published v0.62.2). Net-new work:
- Silent/unattended install flags + GPO/SCCM/Intune packaging (MSIX consideration).
- Register nono as a machine-wide service / standard PATH environment variable (invariant env vars, not per-user JSON copies).
- Auto-provision a secure scratch space (e.g. `%LOCALAPPDATA%\nono\workspaces\`) with `WRITE_OWNER` inheritance pre-configured — eliminates the manual profile-owned-CWD requirement and the Drive-Root ACL-inheritance failures.
- Silent root-certificate install via GPO / Intune.

## Breadcrumbs

- `scripts/build-windows-msi.ps1` — existing WiX MSI build (the `.wxs` is generated from here-strings; see `windows_msi_wxs_is_generated`).
- Existing signed-MSI release pipeline: `release.yml` (Phases 53/61), `scripts/validate-windows-msi-contract.ps1`.
- The profile-owned-CWD / Drive-Root ACL constraint this replaces: see `feedback_windows_mandatory_label_write_owner` and the Phase 60 `WRITE_OWNER` scratch-space handling.
- Related: `feedback_windows_supervised_needs_real_console` (unsigned `Program Files` install can't spawn the broker — the enterprise installer must land *signed* binaries).

## Notes

Captured 2026-06-08 from an enterprise-positioning task list (CISO/CTO horizon). Sibling seeds: [[SEED-002]] network egress, [[SEED-003]] SIEM/EDR logging, [[SEED-004]] multi-engine pluggability, [[SEED-005]] ZT-Infra policy overrides.
