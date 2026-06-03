# Phase 61: Ship/Release v2.9 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-03
**Phase:** 61-ship-release-v2-9-package-and-release-the-phase-60-confined
**Areas discussed:** Release target, Version/tag scheme, Release scope, Pre-ship readiness

---

## Release target (signing & distribution)

| Option | Description | Selected |
|--------|-------------|----------|
| CI-signed public release | release.yml on a v* tag → CI-signed machine+user MSIs as a GitHub release (ROADMAP goal; production-signed; live-verifies pipeline) | ✓ |
| Internal POC-cert distribution | Ship locally-signed MSIs (POC cert 319E507E) to the ~5 admin machines; no public release | |
| Both | CI-signed public + POC-cert local for the controlled group | |

**User's choice:** CI-signed public release via release.yml.
**Notes:** Assumes real signing cert is in CI secrets (signed v0.57.5). If unavailable at release time → surface as blocker, do NOT silently fall back to the self-signed POC cert (D-02).

---

## Version & tag scheme

| Option | Description | Selected |
|--------|-------------|----------|
| v2.9 + bump to 0.58.0 | Milestone tag v2.9 + cut v0.58.0; lockstep 5-crate workspace bump + MSI ProductVersion | ✓ |
| v2.9 + v0.57.13 | Milestone tag + patch bump from the Phase 62 v0.57.12 line | |
| v2.9 milestone tag only | No Cargo bump (stays 0.57.5); MSI version from tag | |

**User's choice:** v2.9 + bump to 0.58.0.
**Notes:** Minor bump = correct semver for a release carrying two features (Phase 60 confined loop + Phase 62 WFP). Mirrors the fork's dual-tag pattern (v2.8 + v0.57.5).

---

## Release scope / binary baseline

| Option | Description | Selected |
|--------|-------------|----------|
| Ship current main — incl. Phase 62 | Bundle Phase 60 confined loop + Phase 62 WFP enforcement + untagged v2.7 drain fixes | ✓ |
| Phase 60 confined-loop only | Ship the ROADMAP's literal 0.57.5 scope; defer Phase 62 WFP | |

**User's choice:** Ship current main (everything, incl. Phase 62).
**Notes:** ROADMAP goal's "0.57.5 binaries" wording predates Phase 62; shipped binaries are the 0.58.0 build off current main.

---

## Pre-ship readiness (multiSelect)

| Item | Selected |
|------|----------|
| release.yml live-verify on the v2.9 tag | ✓ |
| Annotate/delete superseded v0.57.4 release | ✓ |
| Confirm v2.7 drain fixes on main | ✓ |
| Fold in deny-`~/.claude` runner-profile fix | ✓ |

**User's choice:** ALL FOUR selected.
**Notes:** The deny-`~/.claude` runner-profile fix (todo 2026-05-29) is treated as a pre-ship SECURITY BLOCKER (D-09), not deferred — shipping a confined-tools POC whose runner profile can expose `~/.claude` via `--allow-cwd` is a real gap in what v2.9 distributes.

## Claude's Discretion

- Release-notes structure/wording (Windows confined tool-mediation + WFP enforcement; honest POC framing).
- Whether the 0.58.0 bump is its own plan/wave vs folded into release-prep.
- Defining the phase's requirement ID (currently TBD) — propose a `REQ-RLS`-style requirement + register in REQUIREMENTS.md.

## Deferred Ideas

- v2.8 UPST7 upstream sync (Phases 53–59) — separate milestone track.
- v3.0 deferrals — kernel minifilter (Gap 6b), EDR HUMAN-UAT (WR-02).
- claude.exe full read-grant model under AppContainer (Phase 62 deferral).
