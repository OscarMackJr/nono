# Phase 75: Supplementary Controls + Secondary Engines - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-15
**Phase:** 75-supplementary-controls-secondary-engines
**Areas discussed:** Demote semantics & targeting, Per-agent WFP coupling & fail mode, Copilot CLI variant, SC5 proof bar & nono-ts platform scope

---

## Demote semantics & targeting (SUPP-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Further IL-drop + kill-switch lever | demote = drop the running agent's token IL even lower / sever capabilities as an IR lever on an already-confined agent (targets a daemon tenant_id from `nono agent list`); documents spike-002 leak limits | ✓ |
| Network-cut + restrict, not kill | demote = tighten the live agent (revoke network + lower IL) but keep it running for forensics | |
| Generic post-hoc confine of any same-user proc | demote works on ANY same-user PID (original spike-002 framing; widens scope) | |

**User's choice:** Further IL-drop + kill-switch lever.
**Notes:** Demote stays a daemon "demote tenant" verb (tenant_id-targeted), not a generic same-user-PID tool. Spike-002 leak limits documented as the soundness boundary; demote is never the confinement boundary.

---

## Demote scope follow-up (verb relationship + network cut)

| Option | Description | Selected |
|--------|-------------|----------|
| New `demote` verb: IL-drop + cut network, keep running | distinct verb, lowers IL AND deletes the per-agent WFP filter, does not reap | |
| New `demote` verb: IL-drop only (network untouched) | demote only lowers token IL; SUPP-01 and SUPP-02 stay separate | |
| Decide during planning | lock that demote is a distinct non-reaping daemon verb; let planning decide if it also deletes the WFP filter | ✓ |

**User's choice:** Decide during planning.
**Notes:** Locked: demote is a distinct verb that does NOT reap/kill. The WFP-filter-delete composition is left to planning based on how cleanly the SUPP-02 wiring lands.

---

## Per-agent WFP coupling (SUPP-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Daemon → WFP control pipe, auto at launch | USER daemon sends agent package SID over the wfp-service's elevated control pipe; service adds SID-keyed filter at launch, removes on reap | ✓ |
| Daemon → WFP pipe, explicit operator verb | coupling triggered by an explicit operator action, not automatically | |
| Profile-declared, daemon relays at launch | profile is source-of-truth, daemon relays at launch | |

**User's choice:** Daemon → WFP control pipe, auto at launch.
**Notes:** Preserves the Phase 74 D-04 privilege split — USER daemon requests, elevated service enforces; daemon never becomes elevated. Filter lifetime tied to agent reap.

---

## WFP fail mode (service absent)

| Option | Description | Selected |
|--------|-------------|----------|
| Fail-secure: deny launch with actionable error | if profile declares network scoping but WFP service is unreachable, refuse launch + name the missing service | ✓ |
| Degrade to Phase 74 profile-only posture, warn | launch proceeds with profile network.block posture, loud warning | |
| Profile flag decides per-engine | profile declares strict (fail-secure) vs best-effort (degrade) | |

**User's choice:** Fail-secure: deny launch with actionable error.
**Notes:** Matches nono's fail-secure coverage-gate invariant — never silently launch an agent whose egress can't be enforced.

---

## Copilot CLI variant (SUPP-03a)

| Option | Description | Selected |
|--------|-------------|----------|
| Standalone @github/copilot npm CLI | dedicated `copilot` CLI, a node.exe engine — cleanest aider/langchain mirror | |
| gh copilot extension (gh.exe + node) | the `gh copilot` extension — covers gh.exe plus its node subprocess | |
| Decide during research | research/planning picks the variant after checking which is current/installable on the Win11 host; lock only node.exe engine | ✓ |

**User's choice:** Decide during research.
**Notes:** Locked only that Copilot is a `node.exe` engine profiled like aider/langchain-python; concrete distribution chosen during research/planning.

---

## SC5 proof bar + nono-ts platform scope (SUPP-03b / SC5)

| Option | Description | Selected |
|--------|-------------|----------|
| Live Win11 UAT (engines) + Win-only ts parity | Copilot confined end-to-end on real Win11 (like Aider SC1); nono-ts mirrors nono-py Windows-only shapes, build+test green + Win11 confined-run proof | ✓ |
| Live UAT engines + cross-platform ts parity | same engine UAT, but nono-ts also exposes the Unix path — broader parity, more cross-target surface | |
| Build-green parity, no live engine UAT | Copilot + nono-ts proven by automated build+test only | |

**User's choice:** Live Win11 UAT (engines) + Windows-only ts parity.
**Notes:** Matches how Phases 71/72 gated. nono-ts parity is Windows shapes only (mirror nono-py's cfg-gated `windows_confined_run.rs`); pin bump `0.33.0`→`0.62.x`, napi 2 kept.

---

## Claude's Discretion

- WFP keying field (`ALE_PACKAGE_ID` vs `ALE_USER_ID` + SID-scoped SD) — research/implementation, provided it is per-agent.
- Whether `demote` deletes the WFP filter (network cut) — planning's call.
- The daemon→wfp-service control-pipe message shape for per-agent add/remove (reuse/extend existing `session_sid` request; no net-new wire protocol unless insufficient).
- Whether to port a TS analog of nono-py's example 15 + test_confined_run for the SC5 proof.
- Event Log IDs, verb output formats, error wording — fail-secure throughout.

## Deferred Ideas

- Generic post-hoc confine of arbitrary same-user PIDs (original spike-002 framing) — rejected; sound adoption of externally-spawned agents is a v2 deferred requirement.
- Cross-platform nono-ts `confinedRun`/`confine` (Unix Landlock/Seatbelt) — out of scope; Windows shapes only.
- napi 3 migration — deliberate non-bump.
- Operator `net-scope` / explicit per-agent WFP verb — rejected in favor of auto-at-launch.
- Cursor native-Windows confinement — Linux/macOS/WSL-only (v2 deferred anti-feature).
