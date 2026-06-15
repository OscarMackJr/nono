---
slug: zt-infra-e5-poc-runbook
status: complete
created: 2026-06-15
completed: 2026-06-15
---

# Summary: zt-infra E5 POC runbook + SEED-005 pointer

## What was done
- Investigated zt-infra readiness/relationship: `ZeroTrust2\ZERO_TRUST_V2` is the ZT-Infra v2 AWS
  control plane (Apache-2.0); `POST /actions` allow/deny + hash-chained KMS-signed audit; adapters
  `zt_langgraph`/`zt_openai`/`zt_mcp`/`zt_a2a`; install docs live in its own README (full AWS
  Quickstart + a no-AWS "Local Provisioner" path) + `docs/ONBOARDING.md`/`ARCHITECTURE.md`.
- **Verdict: wait.** Phase 73 (AI_AGENT marker) and Phase 75 (supplementary controls + secondary
  engines) are pure nono work; neither consumes zt-infra. The nono↔zt-infra integration is the
  dormant P3 `SEED-005` (external-ledger dependency, "its own later milestone") + the paper E5
  mapping in `DESIGN-engine-abstraction.md`. No nono-side install doc exists (nor is one needed yet).
- Wrote `proj/POC-zt-infra-e5-local-provisioner.md` — AWS-free runbook: start the local provisioner
  (`cd provisioner && npm i && npm start`), then ~30-line throwaway Python glue that authorizes via
  `POST /actions` and only runs the tool confined (`nono_py.confined_run(exe, args, allow, profile)`)
  on `allow`; `deny` ⇒ skip exec fail-closed. Grounded in the real `confined_run` signature + the
  `/actions` response shape (`decision`/`reason`/`audit.current_hash`). Clearly marked POC-only (no
  HTTP adapter in nono core).
- Added a concrete one-line pointer in `SEED-005` (live repo path + local-provisioner quickstart +
  POC runbook link + "not on the 73/75 path" note).

## Files
- created: `proj/POC-zt-infra-e5-local-provisioner.md`
- modified: `.planning/seeds/SEED-005-zt-infra-policy-override-attestation.md`

## Notes
- Docs/POC only — zero changes to nono or nono-py source; no build/test impact.
- Next: proceed to Phase 73 (AI_AGENT Marker), the only remaining incomplete v2.12 phase.
