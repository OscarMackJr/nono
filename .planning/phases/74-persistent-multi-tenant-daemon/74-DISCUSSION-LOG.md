# Phase 74: Persistent Multi-Tenant Daemon - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-14
**Phase:** 74-persistent-multi-tenant-daemon
**Areas discussed:** Daemon process model, Launch vs adopt, Daemon-death agent fate, Operator verb surface, Network scope, Isolation proof

---

## Daemon process model

| Option | Description | Selected |
|--------|-------------|----------|
| Per-user Windows service | SCM-registered (windows-service crate), MSI-installed, runs as the user (not LocalSystem), auto-start at login | |
| On-demand user background process | No SCM/MSI; lazily spawned user process; simpler privilege story but diverges from the nono-wfp-service shape | |
| Both — service with manual fallback | Per-user service as the supported path + a foreground/on-demand mode for dev/testing and uninstalled hosts | ✓ |

**User's choice:** Both — service with manual fallback
**Notes:** Service runs as least-priv USER (not LocalSystem) — the key divergence from nono-wfp-service. Privilege-model ADR written BEFORE the service host is coded (SC4).

---

## Launch vs adopt

| Option | Description | Selected |
|--------|-------------|----------|
| Daemon launches only (sound) | Daemon owns fresh token + job from birth; adopt out of scope (stays Phase 73 best-effort/demote-only) | ✓ |
| Launch primary + adopt secondary | Also expose best-effort 'adopt this PID'; weaker guarantees, more surface/tests | |

**User's choice:** Daemon launches only (sound)
**Notes:** Soundest path; no weaker-guarantee adopt surface in the riskiest phase.

---

## Daemon-death → agent fate

| Option | Description | Selected |
|--------|-------------|----------|
| Agents die with the daemon (fail-secure) | Daemon holds job handle; KILL_ON_JOB_CLOSE terminates all agents on daemon exit; no orphaned confined agents | ✓ |
| Agents survive; daemon re-attaches | Decouple job lifetime + re-attach on restart; better days-long UX but adds orphan-management + re-attach complexity | |

**User's choice:** Agents die with the daemon (fail-secure)
**Notes:** Accepted cost — operator loses running agents on a daemon restart/crash and re-launches. Reinforces deterministic-reap (job lifetime bound to the owning struct's Drop). Foreground fallback behaves consistently (closing the terminal kills its agents).

---

## Operator verb surface

| Option | Description | Selected |
|--------|-------------|----------|
| Daemon lifecycle verbs | `nono daemon start\|stop\|status` | ✓ |
| Agent launch/list verbs | `nono agent launch --profile <engine> -- <cmd>` + `nono agent list` | ✓ |
| Reuse `nono classify` + minimal | Lean on existing `nono classify <pid>`; add only bare daemon control | ✓ |
| Tenant-scoped query verb | A verb to query one tenant's grants over the pipe | |

**User's choice:** Daemon lifecycle verbs + Agent launch/list verbs + Reuse `nono classify` + minimal (tenant-scoped query verb NOT selected)
**Notes:** Minimal surface; reuse Phase 73's `nono classify` for inspection. No tenant-scoped CLI query verb — isolation proven at the protocol layer instead (see Isolation proof).

---

## Network scope

| Option | Description | Selected |
|--------|-------------|----------|
| Profile-only; no WFP coupling in 74 | Agents get their engine profile's network policy; USER daemon stays split from elevated nono-wfp-service; per-agent WFP egress deferred to Phase 75 | ✓ |
| Daemon brokers WFP per agent now | Pull SUPP-02 forward; couples USER daemon to elevated service (weakens DMON-03 split); not recommended | |

**User's choice:** Profile-only; no WFP coupling in 74
**Notes:** Clean DMON-03 split. SC1 'confined' = filesystem NO_WRITE_UP + the profile's existing network posture.

---

## Isolation proof

| Option | Description | Selected |
|--------|-------------|----------|
| Test drives the pipe directly | SC2 cross-tenant-denial negative test exercises the pipe programmatically (two impersonated tenants); no CLI query verb needed | ✓ |
| Add the query verb after all | Also ship a tenant-scoped CLI query verb; adds previously-deferred surface | |

**User's choice:** Test drives the pipe directly
**Notes:** Consistent with the minimal verb surface; isolation proven at the boundary that matters (protocol layer).

---

## Claude's Discretion

- In-phase spike structure (spike vs build directly; harness shape) gated on fresh-token isolation + deterministic reap + cross-tenant denial.
- Whether the wire frame needs a new tenant id field or `session_id` suffices (research-determined; extend ONLY if insufficient).
- Per-agent owning-struct/`Drop` shape; per-tenant SDDL pipe-instance vs single-pipe-with-impersonation mechanism; Event Log IDs; `nono daemon`/`nono agent` output formats. Keep fail-secure throughout.

## Deferred Ideas

- Adopting externally-spawned agents — out of scope (Phase 73 best-effort/demote-only).
- Per-agent WFP egress scoping — Phase 75 / SUPP-02.
- Post-hoc demote (spike-002) — Phase 75 / SUPP-01.
- Tenant-scoped `nono agent query` CLI verb — rejected this phase.
- Agents surviving a daemon restart (decouple job lifetime + re-attach) — rejected (chose fail-secure kill-with-daemon).
