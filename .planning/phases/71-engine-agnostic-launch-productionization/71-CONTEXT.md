# Phase 71: Engine-Agnostic Launch Productionization - Context

**Gathered:** 2026-06-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Promote the validated spike-003 "daemon-as-launcher" path into a **first-class, de-spiked, engine-neutral `nono run` launch path** that can parent-and-confine *any* covered AI agent engine — starting with **Aider** and a **generic LangChain-Python** profile — through one engine-neutral path. The launch is OS-confined transitively (nono is the *parent*, not a per-tool hook): the engine's in-process and subprocess writes inside the granted absolute workspace land; writes outside are denied (`NO_WRITE_UP`).

This is the **foundation** of v2.12 — the daemon (Phase 74), nono-py binding (Phase 72), and AI_AGENT marker (Phase 73) all sit on top of this single-launch code path. Requirements: **ENG-01, ENG-02, ENG-03**.

**In scope:** the engine-neutral launch path; engine profile declaration; fail-secure exe/interpreter coverage gate; R-B3 workspace-ownership diagnostic; SC5 nested-job-collision hardening.

**Out of scope (own phases):** the persistent multi-tenant daemon (74); the AI_AGENT marker / unforgeable token SID (73); the nono-py binding + in-process `confine()` (72); per-agent WFP egress, demote, Copilot/nono-ts (75).

</domain>

<decisions>
## Implementation Decisions

### Engine profile surface (ENG-03)
- **D-01:** Engines are declared as **built-in profiles in `crates/nono-cli/data/policy.json`** (embedded at build time), reusing the existing profile resolver, `windows_low_il_broker` flag, allow-groups, and network identity. An "engine" *is* a profile + an executable/interpreter coverage list. No net-new config surface, no second resolver — matches the locked "composition over existing subsystems / no new framework" constraint.
- **D-02:** Each engine profile carries **explicit executable + interpreter path coverage** so the fail-secure coverage gate has both. For Aider this means the `aider.exe` console-script wrapper **and** the `python.exe` it spawns must be covered (Aider is a Python entry point, not a self-contained binary). NOTE for research/planning: decide explicit-declared interpreter paths vs nono auto-resolving the interpreter — explicit is safer for fail-secure, but interpreter paths vary per machine/venv. This is the one open sub-decision under D-01/D-02.
- **D-03:** Ship **two** built-in engine profiles this phase: `aider` (the SC1 end-to-end proof) **and** a generic **LangChain-Python** profile (python.exe interpreter coverage). Two shapes prove "engine is a variable" and seed Phase 72's nono-py work, per the roadmap goal.

### Launch verb / UX (ENG-01)
- **D-04:** **Reuse `nono run --profile <engine> -- <engine.exe> <args>`** — the engine is the profile. The broker flag, allow-groups (incl. interpreter dirs), and network identity all come from the profile so the user never hand-wires `--allow`. No new verb / subcommand. Pure composition over the existing `run` path. (`nono agent`/`nono launch` verb explicitly rejected for this phase — keep surface minimal; the daemon may introduce verbs later.)

### Workspace & CWD semantics (ENG-01 — the PowerShell→C:\ trap)
- **D-05:** The launcher **sets the child engine process's working directory to the profile's declared absolute workspace**, so the engine's own *relative* writes resolve INSIDE the granted (Low-relabeled) dir. This removes the spike-003 trap where PowerShell resolved a relative write to `C:\` (correctly denied, but opaque).
- **D-06:** The writable workspace is supplied via an **explicit absolute-path flag, defaulting to the current directory canonicalized to an absolute path** when omitted. Consequence: by default child CWD == launcher CWD (canonicalized), and that same dir is the writable grant — one source of truth, no relative-path ambiguity. The grant remains expressed absolutely regardless (engines do not uniformly inherit launcher CWD — locked spike-003 finding).

### Fail-secure diagnostics (ENG-02)
- **D-07:** **Coverage gate:** when an engine's executable/interpreter path is not covered by the launch policy, **fail-secure refuse** and the message **names the exact uncovered binary** (e.g. the `python.exe` Aider would spawn) **and the concrete fix** (`--allow <that dir>` or which profile coverage to extend). Never launch partially confined. **No auto-mutation of policy** from a denial (auto-widening confinement from a prompt is a footgun — rejected).
- **D-08:** **R-B3 ownership:** detect non-session-user workspace ownership **BEFORE launch** and **fail-secure refuse with a named ownership diagnostic** — the message names the ownership problem specifically (e.g. "workspace `<path>` is owned by BUILTIN\Administrators; nono can't apply its grant — run `takeown /F <path>` or create it non-elevated"), not a generic deny. **No auto-`takeown`** (nono mutating directory ownership is a privileged side effect best left explicit — rejected).

### Claude's Discretion
- SC5 nested-job-collision hardening mechanics (spawn suspended → assign to job BEFORE any code runs → fail-secure terminate on assign failure → no UI limits on the job) are locked as a success criterion but the implementation specifics are Claude's discretion. The job here is for kill-group/descendant-capture; the **named, ACL'd, breakaway-denied** job is Phase 73's concern (the marker), not 71's.
- The interpreter-coverage open sub-decision noted in D-02.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Spike findings (the validated path being productionized)
- `.claude/skills/spike-findings-nono/SKILL.md` — spike-findings index; auto-loaded for daemon/multi-engine/token-labeling work.
- `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` — **SEED-004 / spike-003 (VALIDATED)**: daemon-as-launcher is the sound primary model; exe-coverage + absolute-grant contracts; the PowerShell→C:\ trap; R-B3 user-owned-workspace rule; the abstraction boundary (exe/interpreter path, writable workspace, network identity).
- `.claude/skills/spike-findings-nono/references/windows-confinement-model.md` — sandbox-the-tools (not the-TUI); spawn-time is the sound mode; post-hoc IL-drop is demote-only/leaky; CLM-safe payload constraint for confined edits.
- `.planning/spikes/003-daemon-as-launcher/` — original spike source (and the untracked `daemon_grant/` working dir).
- `.planning/seeds/SEED-004-multi-engine-agent-pluggability.md` — the originating seed for v2.12.

### Milestone / requirements
- `.planning/ROADMAP.md` §"Phase 71" + §"Pitfall → Phase Ownership" — SC1-SC5; P6 nested-job-collision owned by 71; R-B3 carry-forward.
- `.planning/REQUIREMENTS.md` — ENG-01, ENG-02, ENG-03 + traceability.
- `.planning/research/SUMMARY.md`, `.planning/research/PITFALLS.md`, `.planning/research/ARCHITECTURE.md`, `.planning/research/STACK.md` — HIGH-confidence research; Shape A/B data flow; load-bearing security properties; deliberate non-bumps (windows-sys 0.59, pyo3 0.28, napi 2).

### Existing code (productionization targets)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — the broker-arm launch path (`windows_low_il_broker:true`).
- `crates/nono-cli/data/policy.json` §`"profiles"` — existing built-in profiles (`claude-code`, `codex`, `python-dev`, etc.) the engine profiles extend.
- `crates/nono-cli/src/cli.rs` — `nono run --profile <name> -- <command>` surface.
- `crates/nono-cli/src/profile/` (`mod.rs`, `builtin.rs`) + `policy.rs` — profile loading + group resolver.

### Cross-target discipline
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for any cfg-gated Unix code touched.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **policy.json profile system** — already carries `windows_low_il_broker`, allow-groups, and network identity per profile (`claude-code`, `codex`, `python-dev`). Engine profiles are new entries here; the resolver, broker flag, and network plumbing are reused wholesale.
- **`nono run --profile <name> -- <cmd>`** — the engine-neutral launch path already exists; this phase formalizes engine profiles on top of it rather than adding a verb.
- **`exec_strategy_windows/launch.rs`** broker arm — the spike-proven `windows_low_il_broker:true` launch; de-spiked here into the first-class path every later phase consumes.

### Established Patterns
- **Fail-secure coverage gate** — core nono invariant: refuse to launch any binary the policy doesn't cover (spike: python under `%LOCALAPPDATA%\Programs\Python` was refused until `--allow`'d). Extend, don't soften.
- **Absolute grants** — engines do not uniformly inherit launcher CWD; express all grant paths absolutely.
- **R-B3** — elevated-created dirs are `BUILTIN\Administrators`-owned → no `WRITE_DAC` → confined writes fail-secure opaquely; workspace must be user-owned.

### Integration Points
- New engine profile entries in `policy.json` → consumed by the profile resolver → fed into the broker-arm launch in `launch.rs`.
- The coverage-gate refusal + R-B3 diagnostic surface as launch-time errors before spawn.

</code_context>

<specifics>
## Specific Ideas

- SC1 acceptance is a **real Win11 host** Aider end-to-end gate: a write inside the granted absolute workspace lands; a write outside is denied (`NO_WRITE_UP`); the engine's in-process and subprocess ops are confined transitively. The broker arm only works from a real host (or dev-layout/signed `nono.exe` per the R-B4 broker-trust gate).
- Re-assert at implementation time: AppContainer per-agent SID needs `CreateAppContainerProfile` (derive-only → `CreateProcessW ERROR_FILE_NOT_FOUND`); preserve `SystemRoot`/`windir`/`SystemDrive` env baseline (else CLR `0xFFFF0000`).
- python.exe was the strongest spike proof that "engine is a variable" (a real non-shell engine confined identically to cmd/powershell).

</specifics>

<deferred>
## Deferred Ideas

- **Non-JSON config / wire format** — user asked whether a SEED exists for getting rid of JSON; none does, and v2.12 explicitly LOCKS framed-JSON (`policy.json` profiles + framed `SupervisorMessage` on the cap pipe, Phase 74 "no net-new wire protocol"). Confirmed as a constraint, not pursued. Any future move off JSON would be a new SEED/backlog item, out of scope for v2.12.
- **First-class `nono agent`/`nono launch` verb** — rejected for Phase 71 (keep surface minimal); may re-surface as a natural home for daemon/marker verbs in Phase 74+.
- **Auto-remediation** (auto-add coverage on denial; auto-`takeown` on R-B3) — rejected as fail-secure footguns; diagnose-only this phase.

</deferred>

---

*Phase: 71-engine-agnostic-launch-productionization*
*Context gathered: 2026-06-13*
