# Phase 72: nono-py Binding + In-Process-Exec Proof - Context

**Gathered:** 2026-06-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Prove the engine abstraction **in code**: confine a real Python/LangChain agent through the **`nono-py` binding** with NO Claude hook, exercising both confinement shapes —
- **Shape A `confined_run(exe, args, allow, profile)`** — spawn a confined child (identical to spike 003), and
- **Shape B `confine(caps)`** — the agent confines itself at startup so its **in-process `exec()`** tools are bounded.

Bump the stale internal `nono` pin (0.57.0 → 0.62.x; pyo3 0.28 kept) and write the **E1–E5 engine-abstraction contract** as a first-class, stable boundary other engines (and the future zt-infra.org layer) implement against.

**Requirements:** ABI-01, ABI-02.

**⚠ Major decision this discussion: the proof platform is WINDOWS, not Linux.** The proof must be **real Windows OS enforcement** — no Linux/Landlock fallback, no advisory/preview "proof." See D-01..D-04 for the consequences (Windows has no Landlock/Seatbelt analog, so Shape B is realized from Windows-only components and is **spike-gated inside this phase**, and SC2/SC3 get reworded away from Landlock language).

**In scope:** the `nono-py` `confined_run` + `confine` API (Windows wrapper over `nono.exe`); the in-phase spike that proves the born-confined self-re-exec is sound; the real-LangChain proof example + test; the nono pin bump; the `proj/DESIGN-engine-abstraction.md` contract doc.

**Out of scope (own phases):** the persistent multi-tenant daemon (74); the AI_AGENT marker / unforgeable token SID (73); per-agent WFP egress, demote, Copilot, nono-ts parity (75); the **full zt-infra.org control-plane integration** (future phase — only the E5 forward-compat mapping is documented here, not wired).

</domain>

<decisions>
## Implementation Decisions

### Proof platform — WINDOWS, real enforcement (overrides roadmap's Landlock framing)
- **D-01:** **The Phase 72 proof runs on Windows with real OS enforcement.** No Linux/macOS fallback as the proof; no fake/advisory proof. Rationale: the milestone is Windows Parity, and the eventual stack (incl. the future **zt-infra.org** layer, which lists `nono` as its execution sandbox) is the target. The user explicitly accepted: **"We cannot deliver a fake proof. Windows native will never have Sandbox/Landlock capability — we can only achieve equivalent Windows functionality with several Windows-only components."**
- **D-02:** **Windows equivalence is assembled from Windows-only components** — the broker, a Low-IL/restricted token, Job Objects, AppContainer package SID, and WFP — NOT a single Landlock-style in-process call. (On Windows `Sandbox::apply` is preview/advisory-only — `crates/nono/src/sandbox/windows.rs` validates the cap set and returns; it does NOT OS-enforce the running process.)
- **D-03:** **Shape B is realized as a born-confined self-re-exec via the broker** — `confine()` bootstraps a confined re-exec of the agent through `nono.exe` as the first thing `main()` does, BEFORE any privileged handle is opened. This is the **sound** Windows equivalent. The leaky mid-life self-IL-drop (the "post-hoc demote" path the research banked as unsound, spike-002) is **rejected**.
- **D-04:** **Shape B is spike-gated INSIDE Phase 72, then built.** A short in-phase spike first proves on a **real Win11 host** that the confined self-re-exec is sound — i.e., **nothing privileged escapes before confinement applies**. ONLY after the spike passes do we build `confined_run` + `confine`. (NOTE for planner: this **overrides** the roadmap's "skip research-phase for 72" line — Phase 72 now carries a spike plan as its first gate.)

### Success-criteria rewording (planner/roadmap action)
- **D-05:** **SC2 and SC3 must be reworded** from Landlock language (`Sandbox::apply` on the *current* process) to the **Windows-equivalent**: *"`confine()` makes the agent **born confined** at its own entrypoint (broker re-exec, before any privileged handle); an in-process LangChain `PythonREPLTool` `exec()` write outside the granted workspace is **DENIED** (Low-IL / Job / AppContainer enforced), a write inside is **ALLOWED**."* The **substance** of SC2 (in-process exec denial) is preserved and genuinely achievable on Windows — only the mechanism wording changes. The planner should land this reword in `.planning/ROADMAP.md` §"Phase 72" and confirm `REQUIREMENTS.md` ABI-01 still reads true (it does — "self-confining at interpreter startup" matches born-at-startup confinement).

### Binding API surface (ABI-01)
- **D-06:** **`confined_run` / `confine` on Windows are a thin wrapper over `nono.exe`** (the installed CLI broker path). They invoke `nono run --profile … -- exe args` (Shape A) and a confined self-re-exec (Shape B). This reuses the audited/signed broker + the R-B4 broker-trust gate; **no broker logic is duplicated into the `nono` lib.** Matches the architecture's "the binding is just a convenience wrapper over `nono run`." The existing Unix `Sandbox::apply` / `sandboxed_exec` path **stays as-is** in the binding (platform fork), but Windows is the Phase 72 proof target.
- **D-07:** **`confined_run(exe, args, allow, profile)` accepts EITHER an engine profile name OR an explicit `CapabilitySet`** (both). The engine-profile path (e.g. `langchain-python` from Phase 71's `policy.json`) keeps the "engine = profile" ergonomics; the raw-caps path gives full control. Two code paths to test/document.

### LangChain proof realism (ABI-01 / SC1+SC2)
- **D-08:** Use the **real `langchain` / `langchain_experimental` `PythonREPLTool`** as an **optional extra**, and ship a **runnable example `examples/15_langchain_confined.py`** plus a **driving test**. Faithful to the roadmap's "a real Python/LangChain agent" wording and to zt-infra's LangGraph/LangChain target. The proof shape:
  ```
  confine(profile='langchain-python', allow=[ws])  # born-confined via broker re-exec
  agent w/ PythonREPLTool
  tool.run("open(r'C:/outside','w')") -> PermissionError / Access denied
  tool.run("open(ws+'/ok','w')")      -> OK
  ```

### Engine-abstraction contract doc (ABI-02 / SC5)
- **D-09:** Author the **E1–E5 contract as `proj/DESIGN-engine-abstraction.md`** in the **main nono repo** (canonical, alongside `DESIGN-library.md` etc.), and **link to it from `../nono-py/docs`**. Engine/binding-neutral home (nono-ts in Phase 75 needs it too). Sourced from `.planning/research/ARCHITECTURE.md` "The Abstraction Boundary Contract."
- **D-10:** The **E5 section explicitly maps to zt-infra.org's `POST /actions`** fail-closed allow/deny + audit contract (forward-compat; the integration itself is a later phase). Document: E5 = optional pre-exec interception point; consumers = Claude PreToolUse (built) **and** zt-infra `POST /actions` (FUTURE: agent → control plane → allow/deny + crypto audit; deny ⇒ skip exec, fail-closed); **nono enforces OS confinement underneath** the policy decision. This is the "don't paint into a corner" check for the future integration.

### nono pin bump (SC4)
- **D-11:** Bump the internal `nono` pin **0.57.0 → 0.62.x** in `../nono-py/Cargo.toml` (and the `nono-proxy` pin, also `0.57.0`, to match). **pyo3 0.28 kept** — no pyo3 major migration. Binding must build + tests green. Mechanical (Claude's discretion on exact patch version — match the published fork crate version).

### Claude's Discretion
- Exact spike pass/fail instrumentation (how "no privileged handle escapes before confinement" is observed on the live host) — left to the spike/planner.
- `network.block` on/off for the file-confinement proof (broker arm needs `nono-wfp-service` for `block:true`; a `block:false` variant is fine for the file-only proof) — Claude's discretion per spike-findings guidance.
- Exact patch version of the 0.62.x pin (match the published fork crate version).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone / requirements / success criteria
- `.planning/ROADMAP.md` §"Phase 72" — SC1–SC5 (SC2/SC3 to be reworded per D-05); also §"Pitfall → Phase Ownership" (in-process-exec post-hoc-confinement pitfall → owned by 72).
- `.planning/REQUIREMENTS.md` — ABI-01, ABI-02 + traceability.

### Shape A/B architecture + the E1–E5 contract (the doc to promote)
- `.planning/research/ARCHITECTURE.md` §"The Abstraction Boundary Contract" (E1–E5 table + invariants) and §"In-process-exec() case (LangChain — no child-process boundary)" (Shape A vs Shape B, the spike-002 "sound only if before any privileged handle" caveat). **This is the source for `proj/DESIGN-engine-abstraction.md`.**
- `.planning/research/SUMMARY.md`, `.planning/research/PITFALLS.md`, `.planning/research/STACK.md` — HIGH-confidence research; deliberate non-bumps (windows-sys 0.59, pyo3 0.28, napi 2).

### Spike findings (the validated launch path being wrapped)
- `.claude/skills/spike-findings-nono/SKILL.md` + `references/engine-agnostic-confinement.md` — SEED-004 / spike-003 (VALIDATED): exe/interpreter-coverage gate, absolute grants, the PowerShell→C:\ trap, R-B3 user-owned workspace, the abstraction boundary; python.exe proven confined identically.
- `.claude/skills/spike-findings-nono/references/windows-confinement-model.md` — spawn-time is the sound mode; **post-hoc IL-drop is demote-only/leaky** (the reason D-03 rejects mid-life self-confine); CLM-safe payload constraint for confined edits.
- `.planning/spikes/003-daemon-as-launcher/` — original spike source.

### Prior phase context (the productionized launch this phase consumes)
- `.planning/phases/71-engine-agnostic-launch-productionization/71-CONTEXT.md` — engine = profile + interpreter coverage; `windows_low_il_broker:true` arm; R-B3; absolute grants; coverage-gate fail-secure. `langchain-python` is an existing Phase 71 built-in profile.

### Existing code (productionization / wrap targets)
- `../nono-py/src/lib.rs` — existing `apply()` (in-process `Sandbox::apply`), `CapabilitySet`, `load_policy` PyO3 surface.
- `../nono-py/src/sandboxed_exec.rs` — existing Unix fork+exec path (`sandboxed_exec`); **Unix-only — does not run on Windows** (informs the D-06 Windows wrapper fork).
- `../nono-py/Cargo.toml` — the `nono = "0.57.0"` / `nono-proxy = "0.57.0"` pins to bump (D-11); `pyo3 = "0.28"` (keep).
- `../nono-py/examples/` + `../nono-py/tests/` — where the LangChain example/test land (D-08).
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — the broker-arm launch `nono.exe` exposes (Shape A/B route via `nono run`).
- `crates/nono-cli/data/policy.json` §`"profiles"` — `langchain-python` engine profile (Phase 71) used by `confined_run`/`confine`.
- `crates/nono/src/sandbox/windows.rs` — confirms Windows `apply()` is preview/advisory-only (the constraint behind D-01/D-02).

### External (future-integration forward-compat)
- https://www.zt-infra.org/ — fail-closed agent security control plane (`POST /actions` allow/deny + crypto audit); composes with `nono` as execution sandbox; LangGraph/LangChain/MCP over HTTP; Linux-container/cloud runtime. Drives the E5 mapping (D-10). **Integration is a FUTURE phase — only documented here.**

### Cross-target discipline
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for any cfg-gated Unix code touched (the binding's platform fork is cfg-gated).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`nono.exe` broker path** (`exec_strategy_windows/launch.rs`, `nono run --profile … -- …`) — Shape A/B wrap this; no broker logic duplicated into the lib (D-06).
- **`langchain-python` built-in profile** (`policy.json`, Phase 71) — the engine profile `confined_run`/`confine` resolve by name (D-07).
- **nono-py PyO3 scaffolding** (`lib.rs`: `CapabilitySet`, `apply`, `load_policy`) — `confine`/`confined_run` extend this surface; the raw-caps path of D-07 reuses `CapabilitySet`.

### Established Patterns
- **Born-confined / spawn-time is the sound mode** — post-hoc IL-drop is demote-only/leaky (windows-confinement-model.md). D-03's self-re-exec keeps confinement at birth.
- **Fail-secure exe/interpreter coverage gate** + **absolute grants** + **R-B3 user-owned workspace** (Phase 71 / spike-003) — all still apply to the binding's launches.
- **Platform fork via cfg gates** — Windows wrapper-over-`nono.exe` vs Unix `Sandbox::apply`; run cross-target clippy (two cfg-gated compile errors have reached release tags in this fork before — hard gate).

### Integration Points
- nono-py `confined_run`/`confine` → spawn/`re-exec` `nono.exe` → broker arm → Low-IL/Job/AppContainer/WFP-confined python process.
- E5 (documented) → future zt-infra `POST /actions` control plane sits ABOVE nono's OS enforcement (D-10).

</code_context>

<specifics>
## Specific Ideas

- The Shape B proof shape (from the LangChain example, D-08):
  ```
  confine(profile='langchain-python', allow=[ws])   # born-confined broker re-exec
  PythonREPLTool.run("open(r'C:/outside','w')")  -> PermissionError / Access denied
  PythonREPLTool.run("open(ws+'/ok','w')")       -> writes OK
  ```
- The E5 → zt-infra mapping (D-10):
  ```
  E5 pre-exec interception point:
    - Claude PreToolUse (built)
    - zt-infra POST /actions (FUTURE phase):
        agent -> control plane -> allow/deny + crypto audit
        deny => skip exec (fail-closed)
    nono enforces OS confinement underneath the decision
  ```
- Re-assert at implementation time (banked Windows gotchas): AppContainer per-agent SID needs `CreateAppContainerProfile` (derive-only → `CreateProcessW ERROR_FILE_NOT_FOUND`); preserve `SystemRoot`/`windir`/`SystemDrive` env baseline (else CLR `0xFFFF0000`); broker arm needs dev-layout or signed `nono.exe` (R-B4 trust gate); workspace must be user-owned (R-B3).

</specifics>

<deferred>
## Deferred Ideas

- **Full zt-infra.org integration** — wiring the binding/engine path to the `POST /actions` control plane is a FUTURE phase (user confirmed: "integration is in scope for future phase"). Phase 72 only documents the E5 forward-compat mapping (D-10), it does not build the HTTP client/adapter.
- **Linux/macOS Shape-B proof** — the Unix `Sandbox::apply` path remains in the binding and genuinely enforces, but Windows is the Phase 72 proof target (D-01). A Unix proof, if ever wanted, would be a separate validation, not Phase 72.
- **Promoting broker into the `nono` lib** — considered for D-06 (so the binding could call the broker without shelling to `nono.exe`); rejected this phase to avoid relocating audited broker code / widening the lib surface. May re-surface if the daemon (74) wants an in-lib launch entrypoint.
- **nono-ts parity** (`confinedRun`/`confine`) — Phase 75; this phase's `proj/DESIGN-engine-abstraction.md` is authored binding-neutral so 75 reuses it.

</deferred>

---

*Phase: 72-nono-py-binding-in-process-exec-proof*
*Context gathered: 2026-06-14*
