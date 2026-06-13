# Phase 71: Engine-Agnostic Launch Productionization - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-13
**Phase:** 71-engine-agnostic-launch-productionization
**Areas discussed:** Engine profile surface, Launch verb / UX, Workspace & CWD semantics, Fail-secure diagnostics

---

## Engine profile surface

| Option | Description | Selected |
|--------|-------------|----------|
| Extend policy.json profiles | Add engine entries as built-in profiles in existing policy.json; reuse resolver/broker flag/allow-groups/network identity. | ✓ |
| Dedicated engine manifest | Separate engine-manifest concept above run profiles; net-new config surface + second resolver. | |
| Pure CLI flags only | No declarative profile; user passes exe + --allow + --workspace each run (raw spike shape). | |

**User's choice:** Extend policy.json profiles
**Notes:** An engine IS a profile + exe/interpreter coverage list. Matches the locked "no new framework / composition" constraint.

| Option | Description | Selected |
|--------|-------------|----------|
| Aider + generic Python | Ship `aider` profile (SC1 proof) + generic LangChain-Python profile. | ✓ |
| Aider only | Ship just `aider`; defer Python to Phase 72. | |

**User's choice:** Aider + generic Python
**Notes:** Two shapes prove "engine is a variable" and seed Phase 72. Each profile must carry explicit executable + interpreter coverage (Aider = aider.exe wrapper AND python.exe).

---

## Launch verb / UX

| Option | Description | Selected |
|--------|-------------|----------|
| Reuse nono run --profile | Engine IS the profile: `nono run --profile aider -- aider <args>`; broker/allow/network from profile. No new verb. | ✓ |
| First-class engine verb | Dedicated `nono agent aider -- <args>` wrapping run+profile; net-new subcommand. | |
| Both (verb aliases run) | Ship run path now + thin `nono agent` alias desugaring to it; two surfaces. | |

**User's choice:** Reuse nono run --profile
**Notes:** Minimal new surface; verb may re-surface for daemon/marker phases later.

---

## Workspace & CWD semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Set child CWD to workspace | Launcher sets child CWD to declared absolute workspace so relative writes resolve inside the grant. | ✓ |
| Keep launcher CWD, require absolute | Don't touch CWD; document absolute paths required (raw spike shape; trap stays live). | |
| Set CWD + still grant absolute | Belt-and-suspenders. | (folded) |

**User's choice:** Set child CWD to workspace
**Notes:** Removes the spike-003 PowerShell→C:\ trap. Grant remains expressed absolutely regardless.

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit flag, default to CWD | Explicit absolute workspace flag; default = canonicalized current dir. | ✓ |
| Always explicit, no default | Require explicit workspace every run. | |
| Profile-declared default | Engine profile carries a default workspace, overridable. | |

**User's choice:** Explicit flag, default to CWD
**Notes:** By default child CWD == launcher CWD (canonicalized) == writable grant — one source of truth.

---

## Fail-secure diagnostics

| Option | Description | Selected |
|--------|-------------|----------|
| Name path + suggest fix | Refuse; name exact uncovered binary + concrete fix (--allow <dir> / profile coverage). No policy auto-mutation. | ✓ |
| Refuse, generic message | Generic "not covered" error; weaker than ENG-02's "actionable". | |
| Offer to auto-add coverage | Interactively widen policy + re-launch; fail-secure footgun. | |

**User's choice:** Name path + suggest fix (coverage gate)
**Notes:** Never launch partially confined; no auto-mutation of policy from a denial.

| Option | Description | Selected |
|--------|-------------|----------|
| Refuse + named R-B3 diagnostic | Detect non-session-user ownership pre-launch; refuse naming the ownership problem + suggest takeown/create-non-elevated. | ✓ |
| Refuse + offer takeown | Same detection + offer to re-ACL the dir; privileged side effect. | |
| Warn + proceed | Surface warning but launch anyway; rejected by fail-secure model. | |

**User's choice:** Refuse + named R-B3 diagnostic
**Notes:** No auto-takeown — directory-ownership mutation left explicit.

---

## Claude's Discretion

- SC5 nested-job-collision hardening mechanics (spawn suspended → assign before code runs → fail-secure terminate on assign failure → no UI limits). Named/ACL'd/breakaway-denied job is Phase 73's concern.
- Interpreter-coverage open sub-decision: explicit-declared interpreter paths vs auto-resolve (per-machine/venv variance) — flagged for research/planning.

## Deferred Ideas

- Non-JSON config / wire format — no SEED exists; v2.12 locks framed-JSON. Confirmed as constraint, not pursued.
- First-class `nono agent`/`nono launch` verb — rejected this phase; may return for daemon phases.
- Auto-remediation (auto-add coverage; auto-takeown) — rejected as fail-secure footguns.
