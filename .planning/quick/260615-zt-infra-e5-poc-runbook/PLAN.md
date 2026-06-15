---
slug: zt-infra-e5-poc-runbook
created: 2026-06-15
type: quick
---

# Quick: zt-infra E5 POC runbook + SEED-005 pointer

Advisory follow-up after the Phase 74 close: the user asked whether to configure/install zt-infra
now or wait for Phases 73/75. Verdict (from investigating both repos): **wait** — neither phase
consumes zt-infra; the integration is the dormant P3 `SEED-005`, its own future milestone.

Two concrete deliverables:
1. `proj/POC-zt-infra-e5-local-provisioner.md` — a runnable, AWS-free runbook for demonstrating the
   E5 composition (zt-infra `POST /actions` decides → `nono_py.confined_run` enforces OS confinement)
   via the zt-infra **local provisioner**. Documentation/POC only — no nono-core HTTP adapter.
2. One-line concrete pointer in `SEED-005` to the live zt-infra repo path + local-provisioner
   quickstart + the POC runbook, plus a 2026-06-15 note that the seed is NOT on the 73/75 path.

No production code; no nono/nono-py source changes.
