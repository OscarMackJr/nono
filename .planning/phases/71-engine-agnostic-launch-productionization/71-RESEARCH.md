# Phase 71: Engine-Agnostic Launch Productionization - Research

**Researched:** 2026-06-13
**Domain:** Windows OS-enforced process confinement (Low-IL broker arm + Job Objects + AppContainer), profile/policy composition, fail-secure launch gating
**Confidence:** HIGH (this is a productionization phase — the path is spike-003 VALIDATED and every mechanism it needs already exists in `crates/nono-cli/src/exec_strategy_windows/launch.rs` and `crates/nono/src/sandbox/windows.rs`; verification was by direct codebase read, not assumption)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Engines are declared as **built-in profiles in `crates/nono-cli/data/policy.json`** (embedded at build time), reusing the existing profile resolver, `windows_low_il_broker` flag, allow-groups, and network identity. An "engine" *is* a profile + an executable/interpreter coverage list. No net-new config surface, no second resolver.
- **D-02:** Each engine profile carries **explicit executable + interpreter path coverage** so the fail-secure coverage gate has both. For Aider: the `aider.exe` console-script wrapper **and** the `python.exe` it spawns must be covered. (OPEN sub-decision: explicit-declared interpreter paths vs nono auto-resolving — resolved below in §"Open Question 1".)
- **D-03:** Ship **two** built-in engine profiles: `aider` (SC1 end-to-end proof) **and** a generic **LangChain-Python** profile (python.exe interpreter coverage).
- **D-04:** **Reuse `nono run --profile <engine> -- <engine.exe> <args>`** — the engine is the profile. No new verb/subcommand. Pure composition over `run`.
- **D-05:** The launcher **sets the child engine process's working directory to the profile's declared absolute workspace** so the engine's relative writes resolve INSIDE the granted (Low-relabeled) dir. Removes the spike-003 PowerShell→`C:\` trap.
- **D-06:** The writable workspace is supplied via an **explicit absolute-path flag, defaulting to the current directory canonicalized to an absolute path** when omitted. Default: child CWD == launcher CWD (canonicalized) == writable grant. Grant remains expressed absolutely regardless.
- **D-07:** **Coverage gate:** when an engine's executable/interpreter path is not covered, **fail-secure refuse** naming the exact uncovered binary AND the concrete fix (`--allow <dir>` or profile coverage). Never launch partially confined. **No auto-mutation of policy.**
- **D-08:** **R-B3 ownership:** detect non-session-user workspace ownership **BEFORE launch** and **fail-secure refuse with a named ownership diagnostic**. **No auto-`takeown`.**

### Claude's Discretion
- SC5 nested-job-collision hardening mechanics (spawn suspended → assign to job BEFORE any code runs → fail-secure terminate on assign failure → no UI limits on the job) are locked as a success criterion; implementation specifics are Claude's discretion. The job here is kill-group/descendant-capture only; the **named, ACL'd, breakaway-denied** job is Phase 73's concern.
- The interpreter-coverage open sub-decision noted in D-02.

### Deferred Ideas (OUT OF SCOPE)
- **Non-JSON config / wire format** — v2.12 LOCKS framed-JSON. Confirmed as a constraint, not pursued.
- **First-class `nono agent`/`nono launch` verb** — rejected for Phase 71. May re-surface in Phase 74+.
- **Auto-remediation** (auto-add coverage on denial; auto-`takeown` on R-B3) — rejected as fail-secure footguns; diagnose-only this phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ENG-01 | A user can run a non-Claude agent engine (Aider) confined end-to-end on a real Win11 host — writes inside the granted workspace land; outside denied (`NO_WRITE_UP`) — regardless of engine. | The `BrokerLaunchNoPty` arm in `launch.rs` already confines arbitrary executables transitively (spike-003 VALIDATED on cmd/powershell/python). Phase work = add an engine profile + set child CWD to the absolute workspace (D-05). §"Standard Stack", §"Architecture Patterns". |
| ENG-02 | The launcher fails secure with an actionable message when an engine's exe/interpreter path is not covered, or when the workspace is not owned by the session user (R-B3). | Coverage gate exists (`validate_launch_paths`, `windows.rs:2091`) — extend to the interpreter (§"Open Question 1"). R-B3 helpers exist (`path_is_owned_by_current_user`, `path_has_write_owner`, `windows.rs:1205/1050`) — wire a *pre-launch* gate (§"Open Question 3"). |
| ENG-03 | A user can declare a per-engine launch profile (exe + interpreter path(s), absolute workspace, network identity) and launch any profiled engine through one path. | Profile struct already carries `binary`, `command_args`, `windows_low_il_broker`, `network`, allow-groups (`profile/mod.rs:2138`). Engine profiles are new `policy.json` entries (§"Open Question 6"). |
</phase_requirements>

## Summary

Phase 71 is a **productionization / de-risking** phase, not a greenfield build. The entire confinement mechanism it needs is already present and battle-tested: `nono run --profile <p> -- <exe>` routes (on Windows, with `windows_low_il_broker:true`) through `WindowsTokenArm::BrokerLaunchNoPty` in `crates/nono-cli/src/exec_strategy_windows/launch.rs`, which spawns a Medium-IL broker that self-degrades to a Low-IL primary token and spawns the engine. Spike-003 proved this confines cmd / powershell / **python** identically (granted write lands, outside write denied via `NO_WRITE_UP`). The Claude-specificity was only ever in the PreToolUse hook, never in the launch primitive.

Therefore the phase's real work is four small, well-scoped extensions on top of existing code: (1) add two engine profiles (`aider`, `langchain-python`) to `policy.json`; (2) extend the **already-existing** executable-coverage gate (`validate_launch_paths`) to also cover the **interpreter** the wrapper spawns; (3) wire the **already-existing** R-B3 ownership helpers into a fail-secure *pre-launch* check; (4) set the child engine's CWD to the declared absolute workspace (D-05) — which the broker arm already does via `current_dir_u16`, so this is mostly making the workspace flag flow into `config.current_dir`. SC5 (nested-job hardening) is **already substantially implemented**: `spawn_windows_child` spawns `CREATE_SUSPENDED`, calls `apply_process_handle_to_containment` (AssignProcessToJobObject) BEFORE `ResumeThread`, and fail-secure terminates on assign failure. The remaining SC5 work is hardening against the *nested-job* case (child already in a job nono didn't create) on Win10 (no nested jobs) vs Win11 (nested jobs supported).

**Primary recommendation:** Treat this as four extension tasks over existing code, NOT a rewrite. For the D-02 interpreter-coverage open sub-decision, use **explicit declared interpreter coverage as a profile field, with an auto-resolve assist that reads the embedded shebang from the console-script `.exe` and surfaces the resolved interpreter path in the fail-secure message** — explicit keeps the gate fail-secure and per-machine-robust; the shebang read makes the "concrete fix" message name the exact `python.exe` path without the user having to hunt for it. Nothing in this phase requires a spike. No RESEARCH BLOCKED items.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Engine declaration (exe + interpreter + workspace + network identity) | Config / `policy.json` profiles | CLI profile resolver (`policy.rs`, `profile/mod.rs`) | D-01: an engine *is* a profile; no new surface. Declarative data, embedded at build time. |
| Engine-neutral launch (parent-and-confine) | CLI / `nono-cli` (`exec_strategy_windows/launch.rs`) | `nono` lib (`create_low_integrity_primary_token`, broker) | Launch policy + token construction is CLI's job; the lib supplies the Low-IL primary token primitive. |
| Exe/interpreter coverage gate | `nono` lib (`sandbox/windows.rs::validate_launch_paths`) | CLI (surfaces the error + fix text) | The coverage check is a library enforcement-boundary decision (`policy.covers_path`); CLI formats the actionable message. |
| R-B3 workspace-ownership pre-check | `nono` lib (`path_is_owned_by_current_user` / `path_has_write_owner`) | CLI (pre-launch gate + diagnostic) | Ownership/WRITE_OWNER is an OS security-descriptor query owned by the lib; CLI decides when to gate and what to say. |
| Child CWD = absolute workspace | CLI (`launch.rs` `current_dir_u16` / `config.current_dir`) | — | The launcher owns the child's `lpCurrentDirectory`; D-05 routes the workspace into it. |
| Job-object kill-group / nested-job hardening (SC5) | CLI (`launch.rs::create_process_containment` + `apply_process_handle_to_containment`) | — | Kill-group/descendant-capture is the launcher's containment concern. Authorization-bearing named job is Phase 73, NOT here. |
| Network identity (AppContainer package SID for WFP) | CLI (`execution_runtime` sets `session_sid` + `app_container_name`) | `nono-wfp-service` (egress, only if `network.block:true`) | For Phase 71 file-confinement proof, use `network.block:false` (no WFP service needed); the package SID still scopes the lowbox. |

## Standard Stack

This phase adds **no new crates**. It composes existing in-tree subsystems. The "stack" is the set of existing functions/types the plan must reuse verbatim.

### Core (existing, reuse verbatim)
| Component | Location | Purpose | Why Standard |
|-----------|----------|---------|--------------|
| `WindowsTokenArm::BrokerLaunchNoPty` | `exec_strategy_windows/launch.rs:1147,1665` | The spike-proven non-PTY Low-IL broker spawn that confines arbitrary engines | This IS the validated engine-agnostic launch path; do not re-implement [VERIFIED: codebase read] |
| `spawn_windows_child` | `launch.rs:1216` | Spawns CREATE_SUSPENDED, assigns to job, applies limits, resumes | Already implements the SC5 suspend→assign→resume sequence [VERIFIED: codebase read launch.rs:1981-2015] |
| `create_process_containment` / `apply_process_handle_to_containment` | `launch.rs:189,247` | Job Object creation (KILL_ON_JOB_CLOSE + DIE_ON_UNHANDLED_EXCEPTION) + `AssignProcessToJobObject` | The kill-group job for SC5; no UI limits set [VERIFIED: codebase read] |
| `validate_launch_paths` / `WindowsFilesystemPolicy::covers_path` | `sandbox/windows.rs:2091,2112` | Fail-secure exe-coverage gate (the spike-003 python refusal) | The coverage-gate invariant already exists; extend to interpreter (D-07) [VERIFIED: codebase read; spike-003 README confirms it fired on python] |
| `path_is_owned_by_current_user` | `sandbox/windows.rs:1205` | Returns `Ok(bool)` — is the path's NTFS owner SID == current token user SID | The R-B3 ownership primitive [VERIFIED: codebase read] |
| `path_has_write_owner` | `sandbox/windows.rs:1050` | Returns `Ok(bool)` — does current user have WRITE_OWNER (the *stronger* R-B3 signal: admin-owned drive-root subdirs lack it even when owner) | Already wired into `try_set_mandatory_label` with a named directive error [VERIFIED: quick-260522-v14 SUMMARY + codebase read] |
| `Profile { binary, command_args, windows_low_il_broker, network, ... }` | `profile/mod.rs:2138-2237` | The profile struct the engine entries deserialize into | All fields the engine abstraction needs already exist [VERIFIED: codebase read] |
| `normalize_windows_launch_path` | `launch.rs:1032` | Strips the `\\?\` verbatim prefix so `cmd.exe`/child CWD is usable | The fix for the `\\?\`-cwd gotcha; already applied to `config.current_dir` at `launch.rs:1318` [VERIFIED: codebase read] |

### Supporting (existing)
| Component | Location | Purpose | When to Use |
|-----------|----------|---------|-------------|
| `recommended_builtin_profile` | `execution_runtime.rs:98` | Maps a program file_name → builtin profile name | Extend the match to map `aider` (and optionally `python`) → engine profile, for the "you probably want `--profile aider`" hint |
| `resolve_program` (`which::which`) | `exec_strategy.rs:150` | PATH-resolves the engine exe to an absolute path | This is where `aider.exe`'s absolute path (and thus its embedded shebang) becomes available for interpreter auto-resolve |
| `append_windows_runtime_env` (env baseline) | `launch.rs:681` | Preserves `SystemRoot`/`windir`/`SystemDrive` + curated PATH | Already preserves the CLR-critical env baseline (else `0xFFFF0000`); engine launches inherit this for free |
| `app_container_name` / `session_sid` plumbing | `execution_runtime.rs`, consumed `launch.rs:1840` | Per-run AppContainer package SID = WFP/network identity | Network identity (E4) for the engine; for file-only Phase 71 proof use `network.block:false` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Extending `validate_launch_paths` to take an interpreter list | A new separate gate function | Rejected — the coverage gate is the single fail-secure choke point; adding a second gate risks one path being checked and the other not. Extend the existing one. |
| Explicit declared interpreter paths in the profile | nono fully auto-resolving the interpreter and silently allowing its dir | Rejected — silent auto-allow violates fail-secure (D-07) and "no auto-mutation of policy". Auto-resolve is a *diagnostic assist*, not a grant. |
| `nono agent` verb | `nono run --profile` | Locked by D-04 — reuse `run`. |

**Installation:** No package installs. This phase edits `policy.json`, `launch.rs`, `sandbox/windows.rs`, and the profile/runtime glue. Build/test via the existing `make build` / `make test` / `make ci`. The Aider end-to-end SC1 gate needs Aider installed on the Win11 UAT host (`pip install aider-install && aider-install`, or `pipx install aider-chat`) — that is a UAT-host prerequisite, not a nono dependency.

## Package Legitimacy Audit

> No external packages are added to the nono workspace this phase. The only third-party software involved is **Aider**, installed on the UAT host (not vendored, not a Cargo/npm/PyPI dependency of nono). slopcheck/registry verification is therefore N/A for the build; the Aider UAT-host install is gated behind human UAT on a real machine.

| Package | Registry | Disposition |
|---------|----------|-------------|
| (none added to workspace) | — | N/A |
| `aider-chat` (UAT-host only, not a nono dependency) | PyPI | UAT-host prerequisite; install + run is human-gated on the Win11 box. Verify on PyPI at UAT time. `[ASSUMED]` (engine-under-test, not a nono dependency) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
  user: nono run --profile aider --workspace C:\Users\me\proj -- aider.exe <args>
        │
        ▼
  ┌──────────────────────────────────────────────────────────────────────┐
  │ nono-cli (Medium IL, supervisor — the PARENT)                          │
  │                                                                        │
  │  1. load profile "aider" (policy.json)  ── exe-cover + interp-cover +  │
  │       windows_low_il_broker:true + network identity                    │
  │  2. resolve aider.exe abs path (which::which)                          │
  │  3. resolve workspace: --workspace OR cwd, canonicalize → ABSOLUTE     │
  │       (D-06); strip \\?\ (normalize_windows_launch_path)               │
  │  4. ── GATE A: R-B3 ownership pre-check ──────────────┐                │
  │       path_is_owned_by_current_user(workspace)        │ fail-secure    │
  │       + path_has_write_owner(workspace)               │ NAMED diag,    │
  │                                                       │ no takeown     │
  │  5. ── GATE B: coverage gate (validate_launch_paths) ─┤                │
  │       covers_path(aider.exe)  AND  covers_path(interp)│ fail-secure    │
  │       interp = declared profile interp  ∪  shebang(aider.exe)         │
  │                                                       │ name exact bin │
  │                                                       │ + --allow fix  │
  │  6. spawn (BrokerLaunchNoPty arm):                    ▼                │
  │       CreateProcessW(nono-shell-broker, CREATE_SUSPENDED,              │
  │                      lpCurrentDirectory = WORKSPACE  ← D-05)           │
  │       AssignProcessToJobObject(kill-group job)  ← SC5, BEFORE resume   │
  │         └─ fail → TerminateProcess (fail-secure)                       │
  │       ResumeThread                                                     │
  └───────────────────────────────┬──────────────────────────────────────┘
                                  ▼
  ┌──────────────────────────────────────────────────────────────────────┐
  │ nono-shell-broker.exe (Medium IL) ─ self-degrades to Low-IL primary    │
  │   token  →  CreateProcessW(aider.exe, AppContainer lowbox)             │
  └───────────────────────────────┬──────────────────────────────────────┘
                                  ▼
  ┌──────────────────────────────────────────────────────────────────────┐
  │ aider.exe (Low IL, AppContainer)  →  spawns python.exe (Low IL)        │
  │   CWD = WORKSPACE (relabeled Low-writable)                             │
  │   write inside  WORKSPACE  → LANDS                                     │
  │   write outside WORKSPACE  → DENIED (NO_WRITE_UP, MIC pre-DACL)        │
  │   ALL descendants inherit Low IL transitively (no per-tool hook)       │
  └──────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure (files this phase touches)
```
crates/nono-cli/data/policy.json          # + "aider", "langchain-python" engine profiles (D-01/D-03)
crates/nono-cli/data/nono-profile.schema.json # + interpreter-coverage field (if declared as a profile field)
crates/nono-cli/src/profile/mod.rs         # + interpreter-coverage field on Profile (if new field)
crates/nono/src/sandbox/windows.rs         # validate_launch_paths: also cover interpreter(s); shebang reader
crates/nono-cli/src/exec_strategy_windows/launch.rs # ensure workspace → config.current_dir (D-05); SC5 nested-job hardening
crates/nono-cli/src/execution_runtime.rs   # + --workspace flag flow; recommended_builtin_profile aider mapping; R-B3 pre-gate call site
crates/nono-cli/src/cli.rs                 # + --workspace <abs-path> flag (D-06)
crates/nono-cli/src/sandbox_prepare.rs     # R-B3 pre-launch gate wiring (before spawn)
```

### Pattern 1: Engine = profile + coverage list (D-01)
**What:** An engine is NOT a new abstraction — it is a `policy.json` profile entry that additionally declares the interpreter path(s) its launch exe will spawn.
**When to use:** Always — this is the locked D-01 shape.
**Example:** (a new `aider` entry, modeled on `claude-code`/`codex`/`python-dev`):
```jsonc
// crates/nono-cli/data/policy.json  — "profiles" object
"aider": {
  "extends": "default",
  "meta": { "name": "aider", "version": "1.0.0",
            "description": "Aider AI pair-programming engine (Python entry point)",
            "author": "nono-project" },
  "security": { "groups": ["python_runtime", "node_runtime", "git_config", "unlink_protection"],
                "signal_mode": "isolated" },
  "filesystem": {},                       // workspace comes from --workspace / cwd, expressed absolutely
  "network": { "block": false },          // file-confinement proof needs no WFP service
  "workdir": { "access": "readwrite" },
  "windows_low_il_broker": true,          // REQUIRED — routes to BrokerLaunchNoPty (the sound arm)
  // NEW field (this phase) — the interpreter(s) aider.exe will spawn. See Open Question 1.
  "windows_interpreters": ["python.exe"]
}
```
> Note: `python-dev` already exists with `"groups": ["python_runtime"]` but is NOT a Windows engine profile (no `windows_low_il_broker`, no interpreter coverage). The generic LangChain-Python engine profile (D-03) should be a **new** entry (e.g. `langchain-python`) that DOES set `windows_low_il_broker:true` and declares `python.exe` coverage — do not retrofit `python-dev`, whose `python_runtime` group has different (dev-tool) semantics.

### Pattern 2: Workspace as the single source of truth (D-05/D-06)
**What:** One canonicalized absolute path is simultaneously (a) the child's `lpCurrentDirectory` and (b) the writable grant. No relative-path ambiguity.
**When to use:** Every engine launch.
**Example (conceptual flow):**
```rust
// execution_runtime / sandbox_prepare (pseudocode grounded in existing helpers)
let workspace = match args.workspace {
    Some(p) => p,                                  // explicit --workspace (D-06)
    None    => std::env::current_dir()?,           // default = launcher CWD
};
let workspace = workspace.canonicalize()?;         // ABSOLUTE (D-06)
// grant it read+write (becomes the Low-relabeled writable dir)
// AND route it into config.current_dir so launch.rs sets it as child CWD:
//   launch.rs:1318  let current_dir = normalize_windows_launch_path(config.current_dir);
//   launch.rs:1321  let current_dir_u16 = to_u16_null_terminated(&current_dir...);
//   passed as lpCurrentDirectory to CreateProcessW (broker arm, launch.rs:1533/1881)
```
The `\\?\` strip is already handled by `normalize_windows_launch_path` at `launch.rs:1318` — confirming Open Question 4: the broker path already neutralizes the verbatim-prefix gotcha for the child CWD.

### Pattern 3: Two-gate fail-secure pre-launch (D-07 + D-08)
**What:** Before spawn, run GATE A (R-B3 ownership) then GATE B (coverage incl. interpreter). Both fail-secure with named diagnostics; neither mutates policy.
**When to use:** Every Windows engine launch.
**Example:** see §"Code Examples".

### Anti-Patterns to Avoid
- **Auto-`--allow`-ing the interpreter dir on a coverage miss.** Violates D-07 (no auto-mutation) and fail-secure. Diagnose only.
- **Auto-`takeown` on an R-B3 miss.** Violates D-08. Name the problem; let the user fix it.
- **Relying on the engine inheriting launcher CWD.** Spike-003 proved PowerShell did not. Always set `lpCurrentDirectory` explicitly AND express the grant absolutely.
- **Retrofitting `python-dev` as the LangChain engine.** Different group semantics; create a new `langchain-python` entry.
- **Adding a second coverage-gate function.** Extend `validate_launch_paths`; one choke point.
- **Setting UI limits on the SC5 job** (e.g. `JOB_OBJECT_UILIMIT_*`). CONTEXT locks "no UI limits on the job" — the job is kill-group only.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Low-IL confining spawn | A new CreateProcess+token path | `BrokerLaunchNoPty` arm in `launch.rs` | Spike-003/Phase-51/60 proven; CLR-safe (env baseline), Authenticode-gated broker, HANDLE_LIST-scoped stdio |
| Exe-coverage refusal | A new `which`+contains check | `validate_launch_paths` / `policy.covers_path` | Already the fail-secure choke point; spike-003 fired on python through it |
| Path ownership check | Raw `GetNamedSecurityInfoW` | `path_is_owned_by_current_user` + `path_has_write_owner` | Both exist, fail-closed, with RAII guards and tests; `path_has_write_owner` is the *correct* R-B3 signal (owner≠WRITE_OWNER on drive-root subdirs) |
| Job assignment ordering | A bespoke suspend/assign/resume | `spawn_windows_child` (suspend→assign→limits→resume already there) | The SC5 sequence is already implemented and fail-secure-terminates on assign failure |
| `\\?\` cwd stripping | A custom prefix strip | `normalize_windows_launch_path` | Already applied to `config.current_dir`; matches `query_ext::strip_verbatim_prefix` |
| Interpreter path discovery | Hardcoded per-machine python paths | Read the console-script `.exe` embedded shebang (assist) + explicit profile declaration | The shebang is `sys.executable` baked at install time — authoritative and per-machine-correct |

**Key insight:** Almost every primitive this phase needs is already in-tree and proven on a real Win11 host. The risk is NOT "can we confine an engine" (answered: yes) — it is "do we cover the *interpreter* and set the *workspace CWD* so the proof is clean and fail-secure." Both are small extensions to existing choke points.

## Runtime State Inventory

> This phase adds new behavior (engine profiles + gates) but does NOT rename/migrate stored state. The relevant "state" is policy.json (embedded at build) and the per-run AppContainer profile.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no datastore keys/collections change. Engine profiles are embedded JSON, regenerated at build from `data/policy.json`. | None |
| Live service config | `nono-wfp-service` is NOT exercised this phase (use `network.block:false`). Per-run AppContainer profile is created/derived at launch (`CreateAppContainerProfile`) and is transient. | None for Phase 71; re-assert AppContainer derive-vs-create rule (memory) only when `network.block:true` is later enabled |
| OS-registered state | The SC5 Job Object is created per-run and torn down on `ProcessContainment::drop` (`KILL_ON_JOB_CLOSE`). No persistent OS registration. | None |
| Secrets/env vars | New `--workspace` flag is a path, not a secret. Engine network identity reuses existing `session_sid`/`app_container_name` plumbing. CLR env baseline (`SystemRoot`/`windir`/`SystemDrive`) already preserved. | None |
| Build artifacts | `policy.json` is embedded via `build.rs`; editing it requires a rebuild for the new profiles to appear (`make build`). If a `windows_interpreters` field is added to the schema, `data/nono-profile.schema.json` + any typify-generated types must regenerate. | Rebuild after policy.json/schema edits; verify schema-generated types stay in sync |

**Nothing found in stored-data / secrets categories** — verified by reading the phase scope (engine declaration + gates + CWD) against the existing config/launch code; no rename or data migration is involved.

## Common Pitfalls

### Pitfall 1: Covering the wrapper exe but NOT the interpreter it spawns
**What goes wrong:** `aider.exe` is covered, launch succeeds, but `aider.exe` immediately spawns `python.exe` from an *uncovered* dir. Depending on enforcement, either the interpreter spawn is refused (good, but only if the gate covers it) or — worse — the proof is muddy.
**Why it happens:** `validate_launch_paths` today only checks the top-level `program` (`windows.rs:2112`), not the interpreter. Aider is a Python entry point: `aider.exe` is a thin distlib launcher that execs `sys.executable` (the embedded-shebang `python.exe`).
**How to avoid:** Extend `validate_launch_paths` to also assert `covers_path(interpreter, Read)` for each declared/auto-resolved interpreter (Open Question 1). Fail-secure naming the exact `python.exe`.
**Warning signs:** Launch succeeds but the engine's own subprocess writes behave inconsistently; or the python.exe path in the error is a `%LOCALAPPDATA%\Programs\Python...` / venv `Scripts\python.exe` path the profile didn't anticipate.

### Pitfall 2: Workspace owned by Administrators (R-B3) → opaque confined-write failure
**What goes wrong:** A dir created from an elevated console is `BUILTIN\Administrators`-owned; the current user lacks WRITE_OWNER, so `try_set_mandatory_label`'s `SetNamedSecurityInfoW(LABEL...)` can't relabel it → confined writes fail with a generic deny that *looks* like a confinement bug.
**Why it happens:** Owner status grants implicit WRITE_DAC + READ_CONTROL but NOT implicit WRITE_OWNER (memory: `feedback_windows_mandatory_label_write_owner`). Drive-root subdirs (`C:\poc\*`) inherit a DACL lacking WRITE_OWNER.
**How to avoid:** Pre-launch GATE A using `path_has_write_owner(workspace)` (the *correct* signal — stronger than owner-SID equality) and surface the existing named directive (`%USERPROFILE%\nono-poc` / `%TEMP%\nono-poc` / `icacls /grant`). Do this BEFORE spawn so the failure is named, not opaque (D-08).
**Warning signs:** Banner shows `r+w <path>` but the write is denied; path is under a drive root (`C:\...`) rather than `%USERPROFILE%`/`%TEMP%`.

### Pitfall 3: The PowerShell→`C:\` relative-write trap (D-05)
**What goes wrong:** The engine does a *relative* write; because it did not inherit launcher CWD, the write resolves to `C:\` and is (correctly) denied — looking like a confinement failure when it is a CWD bug.
**Why it happens:** Engines don't uniformly inherit launcher CWD (spike-003 locked finding).
**How to avoid:** Set the child's `lpCurrentDirectory` to the absolute workspace (D-05; already wired through `config.current_dir`→`current_dir_u16` in the broker arm) AND express the grant absolutely (D-06).
**Warning signs:** "Access is denied" / `UnauthorizedAccessException` to a `C:\<file>` path outside the grant on a *relative* write.

### Pitfall 4: Dev-layout / broker-trust gate (R-B4) blocks SC1 from a Program Files install
**What goes wrong:** SC1 Aider end-to-end fails at the broker Authenticode self-trust gate from an unsigned `C:\Program Files\nono\nono.exe`.
**Why it happens:** `BrokerLaunchNoPty` calls `verify_broker_authenticode` unless `is_dev_build_layout` (`launch.rs:1375,1698`). Unsigned Program-Files installs are correctly refused.
**How to avoid:** Run the SC1 UAT from a **dev-layout `target\release\nono.exe`** (or a signed install). This is a UAT-host instruction, NOT a code change. (Memory: `feedback_windows_supervised_needs_real_console` — also run from a real PowerShell console, not git-bash/MSYS, else `CreateProcessAsUserW` GLE=87.)
**Warning signs:** `TrustVerification` error naming the broker; or GLE=87 from a non-console shell.

### Pitfall 5: Win10 has no nested Job Objects (SC5)
**What goes wrong:** On Win10, if the engine (or its launcher) is *already* in a job that lacks `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`/`BREAKAWAY_OK`, `AssignProcessToJobObject` returns `ERROR_ACCESS_DENIED` (a process can be in only one job pre-nested-jobs).
**Why it happens:** Nested jobs are Win8+ for the *kernel* but the interaction with an existing job's breakaway flags still governs assignment. nono's own job is created fresh per-run, but the child could inherit membership in an ambient job (e.g. a CI runner's job, conhost, or a parent terminal's job).
**How to avoid:** See SC5 hardening in §"Open Question 2": spawn `CREATE_SUSPENDED` (already done), `AssignProcessToJobObject` BEFORE any code runs (already done), and **fail-secure terminate** on assign failure (already done) — the new work is to give a *named* diagnostic distinguishing "already in a job nono didn't create" (`ERROR_ACCESS_DENIED` on assign) from a generic assign failure, so the operator knows the cause.
**Warning signs:** `AssignProcessToJobObject failed` with GLE 5 (`ERROR_ACCESS_DENIED`) specifically.

## Code Examples

Verified against existing in-tree patterns.

### Extending the coverage gate to the interpreter (D-07)
```rust
// crates/nono/src/sandbox/windows.rs — inside validate_launch_paths, after the
// existing program-coverage check (currently windows.rs:2112). Source: codebase read.
//
// `interpreters` is the resolved set: declared profile interpreters (absolute or
// PATH-resolved) UNION any shebang auto-resolved from the program exe.
for interp in interpreters {
    let interp = interp.canonicalize().unwrap_or_else(|_| interp.to_path_buf());
    let interp = normalize_windows_path(&interp);
    if !policy.covers_path(&interp, crate::AccessMode::Read) {
        return Err(NonoError::UnsupportedPlatform(format!(
            "Windows filesystem policy does not cover the interpreter path the engine will \
             spawn: {}. The engine `{}` is a script entry point that launches this \
             interpreter. Fix: add `--allow {}` (or extend the `{}` profile's interpreter \
             coverage). nono will not launch a partially-confined engine.",
            interp.display(),
            program.display(),
            interp.parent().unwrap_or(&interp).display(),
            profile_name,
        )));
    }
}
```

### R-B3 pre-launch gate (D-08), reusing existing helpers
```rust
// In the pre-launch path (sandbox_prepare / execution_runtime), BEFORE spawn.
// Source: helpers at crates/nono/src/sandbox/windows.rs:1205 (owner) and :1050 (WRITE_OWNER).
#[cfg(target_os = "windows")]
{
    // WRITE_OWNER is the load-bearing R-B3 signal: an admin-owned drive-root subdir
    // can be "owned" by you yet still lack WRITE_OWNER, so the mandatory-label relabel
    // would fail opaquely. Gate on WRITE_OWNER (it implies the relabel can succeed).
    if !nono::sandbox::windows::path_has_write_owner(&workspace)? {
        return Err(NonoError::SandboxInit(format!(
            "Refusing to launch: the granted workspace `{}` is not relabelable by the \
             current user (lacks WRITE_OWNER — typically because it was created from an \
             elevated console and is owned by BUILTIN\\Administrators). Confined writes \
             would fail opaquely. Fix: use a workspace under %USERPROFILE% or %TEMP% \
             (e.g. %USERPROFILE%\\nono-work), or run `takeown /F \"{}\"` / \
             `icacls \"{}\" /grant %USERNAME%:(OI)(CI)F` from a non-elevated console. \
             nono will not take ownership of your directory automatically.",
            workspace.display(), workspace.display(), workspace.display(),
        )));
    }
}
```
> Note: `path_is_owned_by_current_user` is the *weaker* check (owner-SID equality). `path_has_write_owner` is what actually predicts whether the relabel can succeed and is therefore the correct R-B3 gate. Prefer it; optionally also report the owner SID for a richer message.

### SC5 nested-job-aware diagnostic (extends existing assign path)
```rust
// crates/nono-cli/src/exec_strategy_windows/launch.rs — apply_process_handle_to_containment
// already exists (launch.rs:247). The suspend→assign→resume sequence already exists at
// launch.rs:2004-2015. The hardening is a *named* error on the nested-job case:
let ok = unsafe { AssignProcessToJobObject(containment.job, process) };
if ok == 0 {
    let gle = unsafe { GetLastError() };
    let detail = if gle == 5 /* ERROR_ACCESS_DENIED */ {
        "the child is already a member of a Job Object nono did not create (and that job \
         disallows breakaway). nono cannot guarantee descendant capture/kill-group for this \
         launch and refuses to continue (fail-secure)."
    } else {
        "AssignProcessToJobObject failed."
    };
    // caller already does: terminate_suspended_process(process, ...) then propagate.
    return Err(NonoError::SandboxInit(format!(
        "Job Object assignment failed (GLE={gle}): {detail}"
    )));
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-tool PreToolUse hook (Claude-specific) confines individual operations | Parent-and-confine: `nono run -- <exe>` confines the engine + all descendants transitively | Spike-001/002/003 (2026-06) | The engine is a variable; no per-engine hook needed for launch-time confinement |
| Direct Low-IL primary token + ConPTY / WRITE_RESTRICTED | Medium-IL broker self-degrades to Low-IL (BrokerLaunch/NoPty) | Phase 31/51 | Avoids `STATUS_DLL_INIT_FAILED 0xC0000142`; the only sound heavy-runtime arm |
| Coverage gate checks only the top-level program | (THIS PHASE) coverage gate also covers the interpreter the wrapper spawns | Phase 71 | Closes the Python-entry-point gap spike-003 surfaced |
| R-B3 "owner SID equality" intuition | WRITE_OWNER is the actual relabel predicate (`path_has_write_owner`) | quick-260522-v14 | Drive-root subdir false-positives avoided |

**Deprecated/outdated:**
- **Post-hoc IL-drop as a primary boundary** — demote-only/leaky (spike-002); explicitly NOT this phase (Phase 75 owns demote).
- **TUI-in-AppContainer** — OS-blocked (`0xC0000142`); not relevant to launch-and-confine.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `aider.exe` is a distlib console-script wrapper embedding an absolute shebang to `sys.executable` (python.exe), so the interpreter path is discoverable from the exe. | Open Question 1, Pitfall 1 | LOW — verified via Python packaging docs/pip behavior [CITED: packaging.python.org/specifications/entry-points]. If a given Aider install uses a different launcher (e.g. `pipx` shim, `uv tool`), the shebang read may point at a shim; explicit declared coverage is the fallback, so fail-secure is preserved. |
| A2 | The generic LangChain-Python engine's interpreter is `python.exe` (the user runs `python my_agent.py`), so the engine exe *is* the interpreter (program == interpreter). | Open Question 1, Pattern 1 | LOW — for a raw `python.exe` launch there is no wrapper; coverage of the program already covers the interpreter. The `langchain-python` profile's interpreter declaration is then trivially satisfied. |
| A3 | Win11 26200 supports nested Job Objects; the SC5 concern is the *already-in-a-foreign-job* case, not nested-job creation failure. | Open Question 2, Pitfall 5 | LOW — nested jobs are supported Win8+. The genuine failure mode is a foreign ambient job with breakaway disallowed (`ERROR_ACCESS_DENIED` on assign), which the fail-secure terminate already handles; we only add a named diagnostic. |
| A4 | `network.block:false` is acceptable for the SC1 file-confinement proof (no WFP service needed), matching spike-003's runner profile. | §"Standard Stack", RSI | LOW — spike-003 ran exactly this way (VALIDATED). Per-agent WFP egress is explicitly Phase 75. |

**If A1 is wrong for a specific Aider install:** the explicit declared `windows_interpreters` coverage still fires the gate (fail-secure); the shebang read is only a *diagnostic assist* that names the exact path. So even a wrong A1 degrades to "user gets a correct fail-secure refusal naming the declared interpreter," never to silent partial confinement.

## Open Questions

### Open Question 1 (THE D-02 sub-decision): explicit-declared vs auto-resolved interpreter coverage — RESOLVED

**Recommendation: Explicit declared interpreter coverage in the profile (a `windows_interpreters` list of bare exe names like `python.exe`), made concrete at launch by PATH/shebang resolution, with an auto-resolve *assist* that reads the console-script `.exe` embedded shebang to NAME the exact interpreter in the fail-secure message — but never to auto-grant it.**

Rationale, grounded in the code + Python packaging facts:
- The coverage gate (`validate_launch_paths`/`covers_path`) operates on **absolute, canonicalized paths** (`windows.rs:2107-2112`). A bare `python.exe` in the profile must be resolved to an absolute path before it can be checked. Two resolution sources, used in this order:
  1. **The engine exe's embedded shebang.** `aider.exe` is a distlib console-script wrapper; its embedded shebang is `sys.executable` — the *exact* `python.exe` that will run, including venv/pyenv-win/`%LOCALAPPDATA%\Programs\Python` variation [CITED: packaging.python.org/specifications/entry-points; pip uses `sys.executable` for console scripts]. Reading the shebang from `aider.exe` (its abs path is already known via `which::which`, `exec_strategy.rs:150`) gives the per-machine-correct interpreter path with zero hardcoding.
  2. **PATH resolution of the declared bare name** (`which::which("python")`) as a fallback when no shebang is present (e.g. the generic LangChain case where the program *is* `python.exe`).
- **Why explicit-in-profile (not pure auto-allow):** D-07 forbids auto-mutation of policy and mandates fail-secure. The profile declaring "this engine spawns `python.exe`" is the *contract*; resolution + coverage-check is enforcement. If the resolved interpreter dir is not in the grant, nono refuses and names it — it does NOT silently widen. This keeps the gate fail-secure AND avoids brittle per-machine hardcoding (the abs path is resolved at launch, not baked into the profile).
- **Why the shebang assist matters:** without it, the fail-secure message can only say "python.exe not covered" generically; with it, the message names `C:\Users\me\proj\.venv\Scripts\python.exe` (the exact path) and the precise `--allow` fix — dramatically better UX while staying diagnose-only.

Concrete shape: add `windows_interpreters: Vec<String>` to `Profile` (bare exe names). At launch, resolve each to absolute via (shebang-of-program ∪ PATH), pass the resolved set into `validate_launch_paths`, which coverage-checks each. The `aider` profile declares `["python.exe"]`; `langchain-python` declares `["python.exe"]` (and since the program is python.exe, it self-satisfies once the workspace/python dir is granted).

### Open Question 2: SC5 nested-job-collision hardening mechanics — RESOLVED

**The locked sequence is already implemented; the residual work is a named diagnostic for the foreign-job case.**

What already exists (verified `launch.rs:1981-2015`, `:189-262`):
- Job created fresh per run with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION` — **no UI limits** (matches CONTEXT "no UI limits on the job").
- Child spawned `CREATE_SUSPENDED` (all arms: broker `launch.rs:1531,1878`, direct `:1955,1971`).
- `apply_process_handle_to_containment` (`AssignProcessToJobObject`) called **after CreateProcess, before `ResumeThread`** — i.e. before any child code runs.
- On assign failure: `terminate_suspended_process` (fail-secure terminate) then propagate. Same for `apply_resource_limits` failure.

What to ADD this phase:
1. **Named foreign-job diagnostic:** distinguish `GetLastError()==ERROR_ACCESS_DENIED (5)` on `AssignProcessToJobObject` — the signature of "child is already in a job that disallows breakaway" — from generic failure (see §"Code Examples"). This is the P6 negative-test hook (SC5).
2. **Decision on breakaway:** CONTEXT scopes the named/ACL'd/breakaway-denied job to Phase 73. For Phase 71's kill-group job, do NOT set `JOB_OBJECT_LIMIT_BREAKAWAY_OK` or `SILENT_BREAKAWAY_OK` — leave breakaway at default-deny so descendants are captured (the kill-group property SC5 wants). The hardening is: detect the collision, fail-secure, name it. Do not attempt to break the child out of the foreign job (that would be an authorization concern Phase 73 owns).
3. **Negative test (P6):** a unit/integration test that simulates assign failure (or asserts the GLE-5 branch produces the named error and terminates the suspended child) — proves "fail-secure on assign failure" structurally.

No spike needed. Win11 26200 supports nested jobs; the only real collision is a foreign ambient job with breakaway disallowed, which is handled by fail-secure terminate + named diagnostic.

### Open Question 3: R-B3 pre-launch ownership detection — RESOLVED

**Use `path_has_write_owner(workspace)` as the pre-launch GATE (not just `path_is_owned_by_current_user`).** Both helpers exist (`windows.rs:1050` and `:1205`). `path_has_write_owner` is the correct predictor of whether the mandatory-label relabel (`SetNamedSecurityInfoW(LABEL...)`) can succeed — it already gates `try_set_mandatory_label` internally (quick-260522-v14), but that gate fires *during* sandbox apply, producing a mid-launch error. Phase 71 wants a *pre-launch* refusal (D-08) with a workspace-specific message. Wire the check in the pre-launch path (sandbox_prepare/execution_runtime) BEFORE spawn; reuse the existing named-directive wording. Optionally enrich with the owner SID (resolve via the same `GetNamedSecurityInfoW(OWNER...)` already in `path_is_owned_by_current_user`) so the message can say "owned by BUILTIN\\Administrators". No auto-`takeown` (D-08).

### Open Question 4: child-CWD-as-absolute-workspace + `\\?\` gotcha — RESOLVED

**The broker arm already sets child CWD correctly and already strips `\\?\`.** Verified: `launch.rs:1318` `let current_dir = normalize_windows_launch_path(config.current_dir);` then `:1321` builds `current_dir_u16`, passed as `lpCurrentDirectory` to `CreateProcessW` in both broker arms (`:1533`, `:1881`) and the direct arm (`:1957`, `:1974`). The broker also forwards `--cwd <current_dir>` to `nono-shell-broker` (`:1849`). So D-05 reduces to: **make `--workspace` (canonicalized absolute, D-06) flow into `config.current_dir`.** The `\\?\` verbatim-prefix gotcha (memory: cwd `\\?\` broke some child launches) is already neutralized by `normalize_windows_launch_path`. The plan should add a test asserting the child's effective CWD equals the declared absolute workspace (and that the strip happened).

### Open Question 5: coverage-gate mechanics + naming the exact binary — RESOLVED

**The gate lives in `nono::sandbox::windows::validate_launch_paths` (`windows.rs:2091`), called on the resolved program.** It already names the uncovered binary (`"...does not cover the executable path required for launch: {program}"`). Extend it (a) to accept and check the resolved interpreter set (Open Q1), and (b) to enrich BOTH messages with the concrete `--allow <dir>` fix + which profile to extend (D-07). Confirm the call site receives the interpreter list — thread it from the profile through the existing `Sandbox::windows_filesystem_policy(config.caps)` / launch-prepare path. No auto-mutation: the error is terminal.

### Open Question 6: two engine profiles' contents — RESOLVED

Model both on `claude-code`/`codex` (which set `windows_low_il_broker:true`, allow-groups, network identity). See Pattern 1 for the `aider` entry. The `langchain-python` entry is the same shape with `"groups": ["python_runtime", "git_config", "unlink_protection"]`, `"windows_interpreters": ["python.exe"]`, `windows_low_il_broker:true`, `network.block:false`, `workdir.readwrite`. Both extend `default` (the test `test_embedded_profiles_extend_default` enforces this — `builtin.rs:340`). Add both to `list_builtin` expectations and the per-profile signal-mode test (`builtin.rs:606`) will exercise them automatically.

**No RESEARCH BLOCKED items.** Every open question resolves against existing code + verified Python-packaging facts. No spike is required for Phase 71 (the daemon's unspiked token/job-reuse risk is Phase 74, explicitly out of scope here).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Dev-layout `target\release\nono.exe` (or signed install) | SC1 broker spawn (R-B4 trust gate) | host-gated (UAT) | — | None — broker refuses from unsigned Program Files. Run from dev layout. |
| Real PowerShell console (not git-bash/MSYS) | `CreateProcessAsUserW`/broker spawn (GLE=87 otherwise) | host-gated (UAT) | — | None — run UAT from PowerShell. |
| Aider installed on Win11 UAT host | SC1 end-to-end Aider proof | host-gated (UAT) | latest | None for the *Aider* proof; the generic `langchain-python` + `python.exe` proof needs only python (also engine-variable evidence) |
| `python.exe` on UAT host | LangChain-Python engine proof | host-gated (UAT) | 3.x | None |
| Win11 (nested Job Objects, AppContainer) | BrokerLaunchNoPty + SC5 | host-gated (UAT) | Win11 26200 (spike box) | Win10 works for file-confinement but watch the foreign-job assign case (Pitfall 5) |

**Missing dependencies with no fallback (UAT-host instructions, not code):**
- Dev-layout nono.exe + real PowerShell console + Aider/python on the Win11 host — all UAT-host prerequisites, surfaced in the HUMAN-UAT doc, not nono build dependencies.

**Missing dependencies with fallback:**
- If Aider install is problematic at UAT time, the generic `langchain-python` + raw `python.exe` proof independently satisfies "engine is a variable" (spike-003 used exactly python as the strongest proof).

## Validation Architecture

> `nyquist_validation` not explicitly disabled — section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`); `proptest` available in nono-cli; integration via `tests/run_integration_tests.sh` + `tests/windows-test-harness.ps1` |
| Config file | none (Cargo) |
| Quick run command | `cargo test -p nono-cli --bins <name>` / `cargo test -p nono <name>` |
| Full suite command | `make test` (build-lib/cli/doc); `make ci` (clippy + fmt + tests) |

### Phase Requirements → Test Map
| Req / SC | Behavior | Test Type | Automated Command | File Exists? |
|----------|----------|-----------|-------------------|-------------|
| ENG-03 / SC2 | `aider` + `langchain-python` profiles load, extend `default`, set `windows_low_il_broker:true`, declare interpreter coverage | unit | `cargo test -p nono-cli --bins get_builtin` / `test_embedded_profiles_extend_default` | ✅ extend existing `profile/builtin.rs` tests |
| ENG-02 / SC3 | Coverage gate names the uncovered **interpreter** + `--allow` fix; fail-secure | unit | `cargo test -p nono validate_launch_paths` | ❌ Wave 0 — add `validate_launch_paths_refuses_uncovered_interpreter` (windows.rs `#[cfg(windows)]`) |
| ENG-02 / SC4 | R-B3 pre-launch gate refuses non-WRITE_OWNER workspace with named diagnostic | unit | `cargo test -p nono path_has_write_owner` + new pre-gate test | ⚠️ helper test exists (`path_has_write_owner_returns_true_for_userprofile_tempdir`); add the pre-launch-gate wiring test |
| SC5 / P6 | Assign-failure → fail-secure terminate + named foreign-job (GLE 5) diagnostic | unit | `cargo test -p nono-cli --bins <assign_failure_test>` | ❌ Wave 0 — add the negative test (the suspend→assign→resume path) |
| ENG-01 / SC1 | Aider confined end-to-end on real Win11: inside-workspace write lands, outside denied, descendants confined | manual (HUMAN-UAT) | dev-layout `nono run --profile aider --workspace %USERPROFILE%\nono-work -- aider.exe ...` from PowerShell | ❌ Wave 0 — author `71-HUMAN-UAT.md` (real-host gate; not automatable) |
| SC2 (CWD) | Child effective CWD == declared absolute workspace; `\\?\` stripped | unit | `cargo test -p nono-cli --bins <cwd_test>` | ❌ Wave 0 — add a `normalize_windows_launch_path` + current_dir assertion test |

### Sampling Rate
- **Per task commit:** `cargo test -p nono <touched>` / `cargo test -p nono-cli --bins <touched>` (< 30s scoped).
- **Per wave merge:** `make test`.
- **Phase gate:** `make ci` green (Windows host) + cross-target clippy IF any cfg-gated Unix code is touched (this phase is Windows-centric; `windows.rs` is `#[cfg(target_os="windows")]`-only, so the cross-target gate is typically N/A — but verify per `.planning/templates/cross-target-verify-checklist.md` if any shared/non-cfg file changes). SC1/SC5-manual gated via `71-HUMAN-UAT.md` on a real Win11 dev-layout host.

### Wave 0 Gaps
- [ ] `validate_launch_paths_refuses_uncovered_interpreter` — covers ENG-02/SC3 (interpreter coverage).
- [ ] R-B3 pre-launch-gate test — covers ENG-02/SC4 (named ownership refusal before spawn).
- [ ] SC5 assign-failure negative test — covers SC5/P6 (fail-secure terminate + GLE-5 named diagnostic).
- [ ] child-CWD == absolute-workspace test — covers SC2 (PowerShell→C:\ trap removed).
- [ ] `71-HUMAN-UAT.md` — the real-Win11 Aider end-to-end gate (SC1) + dev-layout/console/R-B4 preconditions.
- [ ] Profile-load assertions for `aider` + `langchain-python` in `profile/builtin.rs` tests.

## Security Domain

> `security_enforcement` enabled (absent = enabled). This is a security-critical codebase (CLAUDE.md: SECURITY IS NON-NEGOTIABLE, fail-secure, no `.unwrap()`/`.expect()`, path component comparison, canonicalize at the enforcement boundary).

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture / Trust Boundaries | yes | The launch is the trust boundary; nono (parent, Medium IL) confines the engine (child, Low IL/AppContainer). Coverage + R-B3 gates are pre-boundary checks. |
| V4 Access Control | yes | OS-enforced: mandatory integrity label `NO_WRITE_UP`, AppContainer, Job Object. Fail-secure refusal on coverage/ownership miss. |
| V5 Input Validation | yes | `--workspace` and engine exe/interpreter paths MUST be canonicalized and compared by path components (`Path::starts_with` / `covers_path`), NEVER string `starts_with` (CLAUDE.md footgun #1). |
| V6 Cryptography | partial | Broker Authenticode self-trust-anchor (`verify_broker_authenticode`) — do NOT hand-roll; reuse existing chain-walker. |
| V10 Malicious Code / Supply Chain | yes | No new packages. Engine exe coverage prevents launching an uncovered (possibly attacker-planted) binary by bare name; PATH grant dirs are read-only only (`launch.rs:707-717`). |
| V12 File / Resources | yes | Path canonicalization at the enforcement boundary; TOCTOU awareness on the workspace between R-B3 check and relabel (the relabel re-checks via `try_set_mandatory_label`). |

### Known Threat Patterns for {Windows Low-IL launch + profile composition}
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Uncovered interpreter spawned by a covered wrapper (partial confinement) | Elevation of Privilege / Tampering | Extend coverage gate to the interpreter; fail-secure (Open Q1) |
| Workspace owned by Administrators → relabel fails opaquely → user disables confinement to "make it work" | Tampering / EoP | Pre-launch R-B3 WRITE_OWNER gate with named, non-auto-remediating diagnostic (D-08) |
| String `starts_with` path comparison (`/home` matches `/homeevil`) | Tampering | `covers_path` / `Path::starts_with` component comparison (CLAUDE.md) |
| Auto-widening confinement from a denial prompt | EoP | No auto-mutation of policy (D-07); diagnose-only |
| Engine breaks out of the kill-group job (foreign-job collision or breakaway) | EoP | Default-deny breakaway on the SC5 job; fail-secure terminate + named GLE-5 diagnostic; the authorization-bearing job is Phase 73 |
| Symlink/junction in the workspace path changes between check and relabel (TOCTOU) | Tampering | Canonicalize at the boundary; `try_set_mandatory_label` re-verifies WRITE_OWNER at apply time |
| Unsigned broker spawn (trust-anchor bypass) | Spoofing | `verify_broker_authenticode` (already enforced unless dev-layout, R-B4) |

## Sources

### Primary (HIGH confidence — codebase read)
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — broker arms, `spawn_windows_child` suspend→assign→resume, `create_process_containment`, `normalize_windows_launch_path`, env baseline, Authenticode gate, dev-layout detection.
- `crates/nono/src/sandbox/windows.rs` — `validate_launch_paths` (`:2091`), `covers_path` (`:2112`), `path_is_owned_by_current_user` (`:1205`), `path_has_write_owner` (`:1050`).
- `crates/nono-cli/src/profile/mod.rs` (`Profile` struct `:2138`) + `crates/nono-cli/src/profile/builtin.rs` (resolver, tests).
- `crates/nono-cli/data/policy.json` (`claude-code`/`codex`/`python-dev` profiles) + `data/nono-profile.schema.json`.
- `crates/nono-cli/src/execution_runtime.rs` (`recommended_builtin_profile`), `exec_strategy.rs` (`resolve_program`/`which`), `command_runtime.rs` (profile-binary resolution).
- `.planning/spikes/003-daemon-as-launcher/README.md` + `src/main.rs` — VALIDATED engine-neutral launch; the exact coverage-gate refusal string; the PowerShell→C:\ trap.
- `.claude/skills/spike-findings-nono/references/{engine-agnostic-confinement,windows-confinement-model}.md` — SEED-004 abstraction boundary; spawn-time is the sound mode.
- `.planning/quick/260522-v14-.../260522-v14-SUMMARY.md` — `path_has_write_owner` pre-flight + the R-B3 directive wording.
- Project memory: `feedback_windows_mandatory_label_write_owner`, `feedback_windows_supervised_needs_real_console`, `project_sandbox_the_tools` (F-60-UAT-05 broker arm), `windows_appcontainer_wfp_validated` (CreateAppContainerProfile).

### Secondary (MEDIUM — official docs)
- [Entry points specification — Python Packaging User Guide](https://packaging.python.org/specifications/entry-points/) — console-script wrappers embed `sys.executable` (interpreter discoverable from the `.exe`).
- [pypa/pip Issue #6652](https://github.com/pypa/pip/issues/6652) — Windows generates `.exe` script wrappers (distlib launcher with embedded shebang).

### Tertiary (LOW — verify at UAT)
- Aider install path / `aider.exe` location on the specific UAT host (pipx vs pip vs uv) — verify the embedded shebang at UAT time; explicit declared coverage is the fail-secure fallback regardless.

## Metadata

**Confidence breakdown:**
- Standard stack (reuse-existing): HIGH — every component verified by direct codebase read with line numbers.
- Architecture (parent-and-confine + two gates + workspace CWD): HIGH — spike-003 VALIDATED; broker arm in production since Phase 51/60.
- Pitfalls: HIGH — drawn from resolved debug docs, spike-003 findings, and project memory.
- Interpreter-coverage sub-decision (D-02): HIGH — resolved against Python packaging facts + the existing coverage gate.
- SC5 nested-job hardening: HIGH — the locked sequence already exists; only a named diagnostic + negative test remain.

**Research date:** 2026-06-13
**Valid until:** ~2026-07-13 (30 days; stable in-tree subsystems). Re-verify the Aider launcher shape (A1) at UAT time only.
