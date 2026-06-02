# Phase 62: Add WFP kernel network enforcement for Windows supervised runs - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-02
**Phase:** 62-add-wfp-kernel-network-enforcement-for-windows-supervised-ru
**Areas discussed:** Phase scope, Service start posture, Behavior when unavailable, Kernel driver scope, Milestone/track/REQ-ID

---

## Phase scope (foundational — ROADMAP goal was "[To be planned]")

A codebase scout established that WFP enforcement is *already* wired and kernel-level for
supervised runs; the F-60-UAT-03 pain is operational (service not running → fail closed).

| Option | Description | Selected |
|--------|-------------|----------|
| Make network.block work out-of-box | Close operational gap: MSI install + ensure service running; no new kernel code; closes F-60-UAT-03 | ✓ |
| Per-process / AppID filter scoping | Extend WFP filters from port/session-SID to per-executable | |
| Full kernel minifilter driver | Build/wire nono-wfp-driver.sys (NOTE: PROJECT.md defers to v3.0) | |

**User's choice:** Make network.block work out-of-box.
**Notes:** Pragmatic, in-scope; kernel minifilter (Gap 6b) stays v3.0-deferred.

---

## Service start posture

| Option | Description | Selected |
|--------|-------------|----------|
| MSI sets start=auto | Machine MSI registers start=auto; SCM boot-starts as SYSTEM; no per-run elevation | ✓ |
| On-demand auto-start from nono run | sc start at run time — needs admin, usually unavailable, brittle | |
| Setup-time start only | Keep start=demand + one-time elevated setup; resets on reboot | |

**User's choice:** MSI sets start=auto.
**Notes:** `nono setup --start-wfp-service` retained as manual/dev path.

---

## Behavior when service unavailable at enforcement time

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-start if able, else fail-closed | Try start; if it can't, abort with actionable remediation; never pass through | ✓ |
| Hard fail-closed only | Keep current UnsupportedPlatform error, improve message | |
| Allow netsh FirewallRules fallback | Fall back to firewall rules backend (weaker, divergent) | |

**User's choice:** Auto-start if able, else fail-closed.
**Notes:** Fail-secure remains non-negotiable; auto-start only *adds* helpfulness, no silent pass-through. No netsh fallback for this path.

---

## Kernel driver (.sys) scope

| Option | Description | Selected |
|--------|-------------|----------|
| Service-only, no .sys | User-mode service drives kernel WFP (FwpmEngine) = kernel enforcement; driver out of scope | ✓ |
| Include driver install/start | Also auto-install/start nono-wfp-driver.sys (signing burden, overlaps Gap 6b) | |

**User's choice:** Service-only, no .sys.
**Notes:** Real minifilter (Gap 6b) stays v3.0-deferred.

---

## Milestone / track / REQ-ID

| Option | Description | Selected |
|--------|-------------|----------|
| v2.9 POC track, REQ-WFP-01 | Fold into v2.9 Sandbox-the-Tools (Phases 60/61); closes F-60-UAT-03; add REQ-WFP-01 | ✓ |
| New dedicated track/milestone | WFP networking hardening as its own milestone | |
| Keep loosely in v2.8 | Leave under active v2.8 (poor fit) | |

**User's choice:** v2.9 POC track, REQ-WFP-01.
**Notes:** Phase goal + success criteria + REQ-WFP-01 Coverage mapping to be written into ROADMAP during planning.

---

## Claude's Discretion

- Internal API shape for the "ensure service running + start attempt" check relative to `select_network_backend` / probe in `network.rs`, and elevation detection.
- Exact WiX/`.wxs` change to set `start=auto` (and any `ServiceConfig` recovery policy).
- Fail-closed remediation message wording (must name the elevated remediation command).
- Test unit/integration split beyond the mandatory human-UAT.

## Deferred Ideas

- Per-process / AppID WFP filter scoping (own phase).
- `nono-wfp-driver.sys` real kernel minifilter (Gap 6b, v3.0).
- Fine-grained `allow_domain` path/method filtering (Phase 56 / REQ-NET-01, proxy layer).
- Reviewed-not-folded todo: `2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md` — filesystem deny concern, orthogonal to WFP network.
