# Phase 53: Release & Drain - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-28
**Phase:** 53-release-drain
**Areas discussed:** Signing identity, Artifact source + CI verify, Version & tag scheme, Hard-todo disposition

---

## Area selection

| Option | Description | Selected |
|--------|-------------|----------|
| Signing identity | What "signed MSIs" means for v2.8 | ✓ |
| Artifact source + CI verify | CI vs local artifacts; how release.yml is live-verified | ✓ |
| Version & tag scheme | semver/milestone tags; what `nono --version` reports | ✓ |
| Hard-todo disposition | Linux validator + TOCTOU todos — resolve vs defer | ✓ |

**User's choice:** All four areas.

---

## Signing identity

| Option | Description | Selected |
|--------|-------------|----------|
| POC self-signed (local) | sign-poc-local.ps1 as-is; internal-only POC trust | |
| Commercial CA / Azure Trusted Signing | Real OV/EV cert; public trust; needs cert + secrets | |
| POC now, CA-ready | Ship POC-signed; wire/document the CA path for a secrets-only future swap | ✓ |

**User's choice:** POC now, CA-ready.
**Notes:** v2.8 artifacts are POC self-signed; the CA / Azure Trusted Signing path is
wired/documented so a future production identity is a secrets-only change (D-53-01).

---

## Artifact source + CI verify

| Option | Description | Selected |
|--------|-------------|----------|
| POC cert into CI secrets | Base64 POC PFX into WINDOWS_SIGNING_CERT; CI produces POC-signed artifacts | |
| Local artifacts, CI smoke-test | Ship local MSIs; tag push only confirms release.yml starts/reaches signing | |
| Both: CI is source | POC cert in CI secrets AND CI-built MSIs become the official v2.8 artifacts | ✓ |

**User's choice:** Both: CI is source.
**Notes:** CI is the canonical artifact producer (D-53-03/04); sign-poc-local.ps1 demoted
to offline fallback. Follow-up — cert source: **Fresh CI cert + redistribute** chosen
(generate a new self-signed cert for CI; re-import the new public .cer on trusted machines;
old installs' trust does not carry over) (D-53-02).

---

## Version & tag scheme

| Option | Description | Selected |
|--------|-------------|----------|
| Bump to 0.57.4 | Patch bump across 5 crates; tag v0.57.4; distinct from stale local v0.57.3 MSIs | ✓ |
| Keep 0.57.3 | No bump; collides with stale local v0.57.3 MSIs | |
| Minor bump 0.58.0 | Distinct but collides conceptually with upstream v0.58/v0.59 UPST7 absorbs | |

**User's choice:** Bump to 0.57.4.
**Notes:** Follow-up — dual-tag interaction: **Semver drives, v2.8 is marker** chosen.
Push v0.57.4 to drive the build; refine release.yml trigger to semver-only (e.g. `v*.*.*`)
so the v2.8 milestone tag is a non-firing marker (D-53-06).

---

## Hard-todo disposition

| Option | Description | Selected |
|--------|-------------|----------|
| Defer both, written rationale | Re-disposition both via REQ-DRN-02's clause; keep Phase 53 a clean release phase | |
| Do Todo 2, defer Todo 3 | Run the Linux validator investigation now if a host is reachable; defer Todo 3 | |
| Backlog promotion | Promote both into REQUIREMENTS v2 "Deferred"/backlog with rationale, then close the todos | ✓ |

**User's choice:** Backlog promotion.
**Notes:** Todo 2 (Linux validator-preflight) and Todo 3 (TOCTOU fd-relative) promoted to
the v2 Deferred/backlog as tracked items with explicit rationale, then the pending todo
files are closed (D-53-08). Todo 1 (WFP elevated UAT) is the REQ-DRN-01 work (D-53-07).

---

## Final gate

| Option | Description | Selected |
|--------|-------------|----------|
| Ready for context | Write CONTEXT.md and hand off to planning | ✓ |
| Explore more gray areas | Keep discussing | |

**User's choice:** Ready for context.

## Claude's Discretion

- Exact semver glob for the release.yml trigger refinement.
- Fresh-CI-cert generation mechanics + public `.cer` export for redistribution.
- Backlog file format/location for the promoted todos.

## Deferred Ideas

- Commercial CA / Azure Trusted Signing production identity (CA-ready this phase; swap later).
- Todo 2 — Linux validator-preflight investigation (backlog).
- Todo 3 — validate_restore_target fd-relative TOCTOU hardening (dedicated future security phase).
- REQ-WSRH-AUDIT-01 broader heavy-runtime audit (already v2-deferred).
