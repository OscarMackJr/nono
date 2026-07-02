# Phase 100: Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-01
**Phase:** 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker
**Areas discussed:** Cross-repo binding scope, Version-bump membership, CI reconcile disposition, PyPI fix depth, Folded-todo scope, Follow-up tracking

---

## Cross-repo binding scope

| Option | Description | Selected |
|--------|-------------|----------|
| nono-py in-phase, nono-ts bump-only | Full endpoint_policy fix + bump in nono-py; version bump only in nono-ts | |
| Both bindings fully in-phase | 0.66.1 bump in both repos + nono-py endpoint_policy fix, separate DCO commits per repo | ✓ |
| Workspace-only + handoff | Only the nono workspace this phase; bindings as coordinated follow-up | |

**User's choice:** Both bindings fully in-phase
**Notes:** Publish remains operator-gated (D-02). "Fully in-phase" = where the work lands; depth of the endpoint_policy fix is governed separately (see PyPI fix depth).

---

## Version-bump membership

| Option | Description | Selected |
|--------|-------------|----------|
| All members lockstep | Bump every member carrying 0.66.0 + keep all path-dep pins consistent | ✓ |
| Publishable set + deps only | Bump only nono/nono-proxy/nono-cli + required deps; accept version skew | |

**User's choice:** All members lockstep
**Notes:** Surfaced that RLS-10 lists 5 crates but the workspace has 7 members (adds nono-fltmgr-client @0.66.0 and tools/sign-fixture). No `version.workspace` inheritance — every Cargo.toml edited individually.

---

## CI reconcile disposition

| Option | Description | Selected |
|--------|-------------|----------|
| Adapt + short ADR | Cherry-pick applicable #1245/#1251 hunks; preserve prepare-only/signed-MSI/verify-dark; record disposition in a brief ADR | ✓ |
| Adapt, no ADR | Adapt inline; rationale in commit trailers + runbook | |
| Adopt wholesale | Take upstream release-PR CI jobs as-is | |

**User's choice:** Adapt + short ADR
**Notes:** Mirrors the #1225 / ADR-98 adopt-vs-adapt precedent. The verify-dark gate + signed-MSI order are invariants.

---

## PyPI fix depth

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal None stub + follow-up | `endpoint_policy: None` at both initializers; wheel builds + twine passes; file a follow-up to thread the field | ✓ |
| Fully thread endpoint_policy | Expose the field to Python end-to-end this phase | |

**User's choice:** Minimal None stub + follow-up
**Notes:** Unblocks PyPI now; Python users can't set endpoint policy yet (documented). Full threading reserved as a named future phase.

---

## Folded-todo scope

| Option | Description | Selected |
|--------|-------------|----------|
| Keep deferred | Leave both clean-host UAT todos deferred to the trusted-signing/distribution thread | |
| Fold into Phase 100 | Bring both into Phase 100 as the release phase | ✓ |

**User's choice:** Fold into Phase 100
**Notes:** Both are narrowed to host-gated UAT only (code fixes DONE). poc-cert one is additionally externally blocked on the Azure Trusted Signing verify-gate fix → plan as gated/deferred verification, not a hard exit criterion.

---

## Follow-up tracking (endpoint_policy threading)

| Option | Description | Selected |
|--------|-------------|----------|
| Backlog todo | File as a pending todo / CONTEXT deferred idea | |
| Named future phase | Reserve as an explicit future phase | ✓ |

**User's choice:** Named future phase
**Notes:** Recorded in CONTEXT Deferred Ideas as "nono-py endpoint_policy binding completeness."

---

## Claude's Discretion

- Wave/plan breakdown, ADR filename/location, and whether RELEASE-RUNBOOK.md is updated in place vs brought forward.
- Whether correcting the workspace `repository = always-further/nono` field is in RLS-10 scope or a follow-up.

## Deferred Ideas

- Named future phase: fully thread `endpoint_policy` through nono-py `RouteConfig`.
- Actual tag push + registry publish (crates.io/PyPI/npm) — the single operator-gated step after Phase 100.
- Correcting `[workspace.package] repository` to the fork identity (if out of RLS-10 scope).
