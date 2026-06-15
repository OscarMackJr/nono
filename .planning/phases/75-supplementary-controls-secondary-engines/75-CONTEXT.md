# Phase 75: Supplementary Controls + Secondary Engines - Context

**Gathered:** 2026-06-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Round out v2.12 with the **supplementary (never-the-boundary) controls** and the **second-engine / second-binding parity** that proves the engine abstraction generalizes — all low-cost adds layered on the proven launch-time default (Phases 71–74). Requirements: **SUPP-01, SUPP-02, SUPP-03**.

**Demote must FOLLOW a proven launch-time default** — it is an incident-response lever, NOT a confinement model (never the "detect-and-confine-as-primary" anti-feature).

**In scope:**
- **SUPP-01 — Operator demote:** `nono agent demote <tenant>` daemon verb that applies a post-hoc token IL-drop (further IL-drop + kill-switch lever) to an already-born-confined Phase 74 agent, targeted by its daemon `tenant_id` (from `nono agent list`). Documents the spike-002 leak/soundness limits (leaked handles before drop, already-started children, network not auto-covered) — explicitly NOT a standalone boundary.
- **SUPP-02 — Per-agent WFP egress:** the least-privilege USER daemon coordinates with the **elevated `nono-wfp-service`** (over the service's existing control pipe) to install a per-agent, identity-keyed (E4 package SID per tenant) WFP filter **automatically at agent launch**, removed on reap. Couples the two services WITHOUT collapsing the D-04 privilege split (USER daemon never becomes elevated; it requests, the service enforces).
- **SUPP-03a — Copilot CLI engine:** GitHub Copilot CLI ships as a second non-Claude `node.exe` engine profile, confined through the same engine-neutral launch path proven in Phase 71.
- **SUPP-03b — nono-ts parity:** the `nono-ts` (Node) binding reaches parity with `nono-py` — both `confinedRun` (spawn-confined, Shape A) and `confine` (self-confine / born-confined broker re-exec, Shape B) exist; internal `nono` pin bumped `0.33.0` → `0.62.x` (**napi 2 kept — no napi 3 migration**).
- **SC5 proof:** the abstraction is demonstrably proven across **≥2 engines (Aider + Copilot CLI)** and **≥2 bindings (`nono-py` + `nono-ts`)**.

**Out of scope (own phases / deferred):**
- **Generic post-hoc confine of arbitrary same-user PIDs** (the original spike-002 framing) — demote stays a **daemon "demote tenant" verb** keyed on daemon-launched tenants only. Sound adoption of externally-spawned agents remains a v2 deferred requirement (blocked by the post-hoc-IL-drop leak).
- **Cross-platform nono-ts confined_run/confine** (Unix Landlock/Seatbelt shapes) — parity this phase is the **Windows shapes only**, mirroring nono-py's cfg-gated Windows `windows_confined_run.rs`.
- **napi 3 / pyo3 / windows-sys major migrations** — deliberate non-bumps (STACK.md).
- **Cursor native-Windows confinement** — Linux/macOS/WSL-only today (v2 deferred anti-feature).

</domain>

<decisions>
## Implementation Decisions

### SUPP-01 — Demote semantics & targeting
- **D-01:** `demote` is a **further IL-drop + kill-switch lever** on an already-born-confined Phase 74 agent (NOT the original spike-002 "confine an unconfined process" framing). It lowers the running agent's token integrity level / severs capabilities as an **incident-response lever**, layered on the spawn-time default. The spike-002 leak limits (handles leaked before drop, children already started, network not covered) are documented inline as the soundness boundary. It is **demote-only — never a standalone confinement boundary**.
- **D-02:** The operator targets a **daemon `tenant_id`** (obtained from `nono agent list`) — it is a daemon "demote tenant" verb, scoped to daemon-launched agents only. Demoting an arbitrary same-user PID is OUT of scope (would widen to the rejected spike-002 generic shape).
- **D-03:** `demote` is a **distinct verb that does NOT reap/kill** the agent (separate from `stop`/kill, which terminates + reaps). **Whether `demote` ALSO deletes the agent's per-agent WFP filter (cut egress) is a PLANNING decision** — lock that it does not reap; let planning decide if it composes the SUPP-02 network-cut based on how cleanly the wiring lands.

### SUPP-02 — Per-agent WFP coupling & fail mode
- **D-04:** Coupling is **daemon → WFP control pipe, automatic at launch.** The least-priv USER daemon sends the agent's per-agent identity (E4 AppContainer **package SID** per tenant) to the elevated `nono-wfp-service` over the service's existing elevated control pipe; the service adds the SID-keyed filter when the agent launches and **removes it on reap** (filter lifetime tied to the agent's owning struct / reap path). The USER daemon never becomes elevated — it requests, the elevated service enforces — preserving the D-04 (Phase 74) privilege split.
- **D-05:** **Fail-secure when the WFP service is absent.** If an agent's profile declares network scoping but the elevated `nono-wfp-service` is NOT installed/reachable at launch, the daemon **refuses to launch and names the missing service** in an actionable error — never silently launches an agent whose egress cannot be enforced. (Matches nono's fail-secure coverage-gate invariant.)

### SUPP-03 — Second engine & second binding
- **D-06:** **Copilot CLI variant chosen during research/planning.** Lock only that it is a **`node.exe` engine** profiled like aider/langchain-python (exe + `windows_interpreters` coverage). Research/planning picks the concrete distribution (standalone `@github/copilot` npm CLI vs `gh copilot` extension) after confirming which Copilot CLI is current/installable on the Win11 test host.
- **D-07:** **nono-ts parity = Windows shapes only.** Mirror nono-py's `confined_run`/`confine` (Shape A + Shape B born-confined broker re-exec) as `confinedRun`/`confine`; pin bump `0.33.0` → `0.62.x`, **napi 2 kept**. Do NOT extend parity to the Unix Landlock/Seatbelt path this phase (keeps cross-target clippy surface bounded; nono-py's confine path is itself Windows-cfg-gated).

### SC5 — Acceptance bar
- **D-08:** **Live Win11 UAT for engines + Windows-only ts parity proof.** Copilot is confined end-to-end on a **real Win11 host** (like Aider's Phase 71 SC1 gate). nono-ts `confinedRun`/`confine` build + test green **plus a Win11 confined-run proof** (mirrors how Phases 71/72 gated — not build-green-only).

### Claude's Discretion
- **WFP keying field** — keying the per-agent filter on the AppContainer **package SID** (E4 identity) vs user SID: the service already accepts a `session_sid` + builds a SID security descriptor (`nono-wfp-service.rs:1557`); the exact condition field (`ALE_PACKAGE_ID` vs `ALE_USER_ID` + SID-scoped SD) is research/implementation discretion, provided it is **per-agent** (one agent's allowed domain never leaks to another).
- **Demote↔WFP-cut composition** (D-03) — planning's call whether `demote` deletes the WFP filter.
- **Control-pipe message shape** for the daemon→wfp-service per-agent add/remove — reuse/extend the service's existing request shape; no net-new wire protocol unless the existing `session_sid` request proves insufficient.
- **nono-ts examples/tests mirror** — whether to port a TS analog of nono-py's `examples/15_langchain_confined.py` + `tests/test_confined_run.py`; default to mirroring for the SC5 proof, exact shape is discretion.
- **Event Log IDs, verb output formats, error wording** — discretion within fail-secure throughout (any coverage/auth/service-reachability error → deny).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone / requirements
- `.planning/ROADMAP.md` §"Phase 75: Supplementary Controls + Secondary Engines" — SC1–SC5; §"Pitfall → Phase Ownership" ("Detect-and-confine as primary model" anti-feature owned here / SC1).
- `.planning/REQUIREMENTS.md` — **SUPP-01, SUPP-02, SUPP-03** + traceability; v2 deferred (sound adoption, Cursor native-Windows, signed-policy).
- `.planning/PROJECT.md` — v2.12 milestone scope; "user-mode only, no kernel driver"; isolation ≥ per-invocation `nono run`; deliberate non-bumps (napi 2, windows-sys 0.59).

### Research (HIGH-confidence)
- `.planning/research/SUMMARY.md` — milestone research synthesis (Shape A/B data flow; load-bearing security properties; deliberate non-bumps).
- `.planning/research/PITFALLS.md` — the "detect-and-confine as primary model" anti-feature; demote-as-IR-lever framing.
- `.planning/research/STACK.md` — napi 2 (no napi 3), windows-sys 0.59 non-bumps; net-new Win32 surfaces.
- `.planning/research/ARCHITECTURE.md` — the 4 new components over 3 existing subsystems (broker-arm launch, cap pipe, WFP service).

### Spike findings (the demote-only model & engine-agnostic contract)
- `.claude/skills/spike-findings-nono/SKILL.md` — spike-findings index (auto-loaded for multi-engine / token-labeling work).
- `.claude/skills/spike-findings-nono/references/windows-confinement-model.md` — **spike-002 PARTIAL**: post-hoc IL-drop is demote-only / leaky (handles leaked before drop, children already started, legitimate handles also broken, network not covered). The load-bearing basis for SUPP-01's documented leak limits and "never the boundary."
- `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` — SEED-004 / spike-003 engine-agnostic launch contract (the path Copilot's profile consumes).

### Engine-abstraction contract & process model
- `proj/DESIGN-engine-abstraction.md` — **E1–E5 contract** (E1 exe/interpreter path, E2 ownable launch command, E3 absolute workspace, **E4 network identity = AppContainer package SID** — the per-agent WFP key, E5 pre-exec interception). Copilot's profile and nono-ts parity both implement against this.
- `proj/DESIGN-supervisor.md` — process model, execution strategies, supervisor IPC.

### Carried-forward phase context
- `.planning/phases/74-persistent-multi-tenant-daemon/74-CONTEXT.md` — **D-04 there:** the USER daemon is deliberately SPLIT from the elevated WFP service (Phase 74 kept DMON-03 clean); SUPP-02 now couples them without collapsing that split. Daemon verb surface (`nono agent launch|list`, `nono daemon …`); per-agent owning-struct + `Drop` reap (the filter-removal hook point); fresh per-agent AppContainer package SID = the tenant key / E4 identity.
- `.planning/phases/72-nono-py-binding-in-process-exec-proof/72-CONTEXT.md` — the nono-py `confined_run`/`confine` (Shape A/B) shapes nono-ts mirrors; the Phase 72 pin-bump pattern (`0.57.0`→`0.62.x`) nono-ts repeats (`0.33.0`→`0.62.x`); E1–E5 contract.
- `.planning/phases/71-engine-agnostic-launch-productionization/71-CONTEXT.md` — engine profiles in `policy.json` (`windows_interpreters`), absolute-grant + exe/interpreter fail-secure coverage gate, R-B3 user-owned workspace, AppContainer/CLR re-assertions — all consumed by the Copilot profile + its SC3 UAT.

### Existing code (implementation targets)
- `crates/nono-cli/src/bin/nono-wfp-service.rs` — the elevated WFP service. Already accepts a per-request `session_sid` and keys filters via `FWPM_CONDITION_ALE_USER_ID` + a SID-scoped security descriptor (`sid_to_security_descriptor`, ~line 1328) / `FWPM_CONDITION_ALE_APP_ID` (~line 1556–1558). SUPP-02 adds the daemon→service per-agent (package-SID) add/remove path over its control pipe.
- `crates/nono-cli/src/agent_daemon/control_loop.rs` — the daemon `ControlRequest` enum (`Launch`/`List`); SUPP-01 adds a `Demote` verb; SUPP-02 hooks the per-agent WFP add at launch.
- `crates/nono-cli/src/agent_daemon/{launch.rs,reap.rs,mod.rs,accept_loop.rs}` — launch orchestration + per-agent owning-struct/`Drop` reap (the WFP-filter add-at-launch / remove-at-reap hook points).
- `crates/nono-cli/src/agent_cli.rs` + `crates/nono-cli/src/cli.rs` — `nono agent …` verb surface (add `demote`).
- `crates/nono-cli/data/policy.json` — engine profiles (`aider`, `langchain-python` with `windows_interpreters`; `node-dev`, `node_runtime` group ~line 365) — add the Copilot `node.exe` profile here.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — the fresh-token + fresh-job Broker-arm spawn the daemon orchestrates (per-agent package SID minted here = the WFP key).

### nono-ts (sibling repo — parity target)
- `../nono-ts/Cargo.toml` — `nono = { version = "0.33.0" }` (bump → `0.62.x`); `name = "nono-node"`, napi.
- `../nono-ts/src/lib.rs` — current napi exports (`JsCapabilitySet` with `allow_path`/`block_network`/etc.); add `confinedRun` + `confine` mirroring nono-py.
- `../nono-py/src/windows_confined_run.rs` — the **nono-py `confined_run`/`confine` reference implementation** to mirror in nono-ts (Shape A spawn-confined + Shape B born-confined broker re-exec).
- `../nono-py/examples/15_langchain_confined.py` + `../nono-py/tests/test_confined_run.py` — the Phase 72 example/test shape to port as the nono-ts SC5 proof.

### Cross-target discipline
- `.planning/templates/cross-target-verify-checklist.md` — mandatory Linux+macOS clippy protocol for any cfg-gated Unix code touched (the daemon/WFP additions and nono-ts both need non-Windows stubs).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **WFP service SID-keyed filter path** — `nono-wfp-service.rs` already accepts `session_sid` and builds a SID-scoped security descriptor + `ALE_USER_ID`/`ALE_APP_ID` conditions. SUPP-02 reuses this; the delta is a daemon→service per-agent (package-SID) add/remove request over the existing control pipe, not a new filter engine.
- **Daemon control loop + reap** — `control_loop.rs` `ControlRequest` enum (`Launch`/`List`) extends with `Demote`; the per-agent owning-struct `Drop` (Phase 74) is the natural WFP-filter-removal hook.
- **Engine profile pattern** — `policy.json` `aider`/`langchain-python` profiles (exe + `windows_interpreters`) are the template for the Copilot `node.exe` profile; `node_runtime`/`node-dev` groups already model node coverage.
- **nono-py `windows_confined_run.rs`** — a complete, shipped `confined_run`/`confine` reference (Shape A + Shape B born-confined re-exec) for nono-ts to mirror; Phase 72's pin-bump + examples/tests are a step-by-step parity recipe.

### Established Patterns
- **Library-vs-CLI boundary** — WFP/demote *mechanism* in the `nono`/service layer; `nono agent demote` verb + UX in `nono-cli`.
- **Fail-secure default** — any coverage/auth/service-reachability error → deny (D-05 WFP-service-absent → refuse launch; exe/interpreter coverage gate for Copilot).
- **Deterministic reap via `Drop`** — per-agent WFP filter lifetime tied to the agent's owning struct (add at launch, remove at reap) — same discipline as Phase 74's token/job handles.
- **Pin-bump + cfg-gated parity** — nono-ts bump `0.33.0`→`0.62.x` (napi 2), Windows-cfg-gated `confine`/`confinedRun` + non-Windows stub + cross-target clippy (mirrors nono-py Phase 72).

### Integration Points
- Daemon launch path → (auto) daemon→`nono-wfp-service` control pipe → per-agent package-SID filter add; agent reap → filter remove.
- `nono agent demote <tenant>` → daemon `Demote` request → post-hoc token IL-drop on the tenant's process (optionally WFP-filter delete, D-03 planning call).
- Copilot profile (`policy.json`) → Phase 71 engine-neutral launch path → confined `node.exe` child (SC3 Win11 UAT).
- nono-ts `confinedRun`/`confine` → nono `0.62.x` Broker-arm launch (Shape A/B), proven on Win11 (SC5).

</code_context>

<specifics>
## Specific Ideas

- **SC3/SC5 are real-Win11-host gates** — Copilot confined end-to-end like Aider's Phase 71 SC1; nono-ts confined-run proven on Win11 (not build-green-only).
- Re-assert at implementation (carried from 71/72/74): AppContainer per-agent SID needs `CreateAppContainerProfile` (derive-only → `CreateProcessW ERROR_FILE_NOT_FOUND`); preserve `SystemRoot`/`windir`/`SystemDrive` env baseline (else CLR `0xFFFF0000`); Broker arm needs a real host or dev-layout/signed `nono.exe` (R-B4 trust gate).
- Demote leak limits MUST be documented at the verb (spike-002): handles leaked before the drop, child processes already started, legitimate handles also severed, network not auto-covered.
- nono-ts pin bump must touch all internal path-dep `version` pins consistently (sibling-repo analog of the 5-crate workspace bump discipline).

</specifics>

<deferred>
## Deferred Ideas

- **Generic post-hoc confine of arbitrary same-user PIDs** (original spike-002 framing) — rejected this phase; demote stays daemon-tenant-scoped (D-02). Sound adoption of externally-spawned agents is a v2 deferred requirement (blocked by the post-hoc-IL-drop leak — needs a different mechanism).
- **Cross-platform nono-ts `confinedRun`/`confine`** (Unix Landlock/Seatbelt) — out of scope (D-07, Windows shapes only). Revisit if nono-ts Unix users need the in-binding confine path.
- **napi 3 migration** — deliberate non-bump; nono-ts stays napi 2.
- **Operator `net-scope`/explicit per-agent WFP verb** — rejected in favor of auto-at-launch (D-04). Revisit only if an opt-in/auditable per-agent egress toggle is later wanted.
- **Cursor native-Windows confinement** — Linux/macOS/WSL-only (v2 deferred anti-feature).

### Reviewed Todos (not folded)
None — no pending todos matched Phase 75 scope.

</deferred>

---

*Phase: 75-supplementary-controls-secondary-engines*
*Context gathered: 2026-06-15*
