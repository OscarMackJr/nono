# ADR-116: Tool-Sandbox Divergence — Adopt vs. Formalize Fork-Native

**Status: Accepted**
**Phase:** 116 — Tool-Sandbox Divergence Audit + Disposition ADR
**Date:** 2026-08-09
**Authors:** Phase 116 execution

<!-- Status: Accepted, set by Plan 116-06 per D-11, derived from the D-09 scoring table and the
D-07 feasibility tally above. -->

---

## Context

Upstream `nolabs-ai/nono` shipped a `tool-sandbox/` subsystem at v0.65.0 (PR #1105) and has grown
it continuously through v0.71.0 without ever being absorbed into this fork. `116-DIVERGENCE-LEDGER.md`
covers the subsystem's full upstream history (`v0.64.1..v0.71.0`, D-01) and closes with the
following finalized figures, cited here from its `## Headline` section (Plan 116-04's output,
which has closed by the time this plan runs):

- **38** total dispositioned commits across the subsystem's 3-path union
  (`crates/nono-cli/src/tool-sandbox/`, `crates/nono-cli/src/command_policy.rs`,
  `crates/nono-cli/src/lineage_cgroup.rs`) — split as 7 pre-fence (`v0.64.1..v0.66.0`) + 20 fenced
  (`v0.66.0..v0.69.0`, cross-referenced from `108-DIVERGENCE-LEDGER.md`, D-13) + 11 post-fence
  (`v0.69.0..v0.71.0`). The D-01 hypothesis and the live re-measurement agree exactly on every one
  of these figures — "confirmed, no delta", as the ledger's own Headline states.
- **windows-touch: 0 of 38.** Every dispositioned commit in the subsystem's history is
  `windows-touch: no` — consistent with the measured fact that upstream ships no
  `platform/windows.rs` for this subsystem at all.
- At `v0.71.0`, the subsystem is **12 files, 18,433 LOC total** (Section 4,
  `116-RESEARCH.md`). Of that, `platform/linux.rs` (6,586 LOC) + `platform/macos.rs` (7,041 LOC) =
  **13,627 LOC — 74% of the subsystem** — is Unix platform-driver code, and **no
  `platform/windows.rs` file exists upstream.**

**The single most decision-relevant framing fact, stated neutrally:** 74% of the subsystem
(`platform/linux.rs` + `platform/macos.rs`) is Unix platform drivers, and upstream ships no
`platform/windows.rs` at all — so "adopt" on the platform this milestone cares about means "take
upstream's model and write its largest component from scratch," not "take upstream's code as-is."
This framing applies equally to evaluating both poles below; it is not evidence for either.

---

## Pole A: Adopt + Build platform/windows.rs

Pole A adopts upstream's `tool-sandbox/` subsystem model and builds a new `platform/windows.rs`
driver from scratch, on the fork's existing AppContainer / Low-IL / WFP / broker primitives
(`crates/nono-cli/src/exec_strategy_windows/`, `crates/nono-shell-broker/`). It takes upstream's
12-file, 18,433-LOC `tool-sandbox/` inventory (`audit_context.rs`, `credentials.rs`,
`dynamic_providers.rs`, `env.rs`, `launch.rs`, `mod.rs`, `platform/linux.rs`, `platform/macos.rs`,
`policy.rs`, `protocol.rs`, `token_broker.rs`, `url_shim.rs`, `116-RESEARCH.md` §4) as its module
shape, absorbing the Unix drivers largely as-is and authoring the Windows driver net-new.
`mod.rs` already cfg-gates `mod linux;` / `mod macos;` under
`#[cfg(any(target_os = "linux", target_os = "macos"))]` and defines a no-op stub
(`PreparedToolSandboxRuntime`, `maybe_run_internal_tool_sandbox_entrypoint() -> false`) for any
other platform — meaning upstream's own crate already compiles cleanly on Windows today, with the
subsystem inert; building `platform/windows.rs` is additive to upstream's existing cfg-gate shape,
not a fork against it (`116-RESEARCH.md` §4, Platform driver entry points).

## Pole B: Formalize Fork-Native

Pole B formalizes the fork-native path that already exists: PR #4's PreToolUse hook →
`nono run` + Low-IL primary-token broker becomes a permanent, named scope boundary, recorded in
the ADR-111/ADR-113 shape (a standalone ADR naming a considered scope and its boundary, rather
than absorbing an upstream subsystem). It builds on the design captured in
`.planning/quick/260528-sch-spec-the-sandbox-the-tools-windows-tool-/260528-sch-SPEC.md`
("sandbox the tools, not the TUI": `claude` runs at Medium IL with a real terminal; each
tool-invocation hook wraps the operation with `nono run --profile <p> -- <the tool cmd>`, which
spawns a Low-IL, kernel-confined child) and on the shipped hook wrapper
`crates/nono-cli/data/hooks/nono-tool-hook.ps1` (57 lines, fail-closed deny JSON on any non-zero
exit).

---

## D-06: Considered and Rejected Intermediate Shapes

### (i) Adopt Unix-Only

Absorb upstream's `tool-sandbox/` subsystem for Linux and macOS, where its `platform/linux.rs` and
`platform/macos.rs` drivers land largely as-is, and leave Windows on the fork-native PreToolUse
hook path unchanged. **Rejection reason:** this leaves two tool-confinement architectures
permanently split by OS — one subsystem (upstream's `tool-sandbox/`) governing Linux/macOS, and an
entirely different one (the PreToolUse hook + `nono run` + Low-IL broker) governing Windows,
with no shared code path or shared policy surface between them going forward.

### (ii) Adopt the Policy Model Only

Absorb only `command_policy.rs`'s schema and the `command_policies` profile surface — which the
fork already documents in `crates/nono-cli/data/profile-authoring-guide.md` — enforced through the
fork's existing Windows path, without adopting the rest of `tool-sandbox/`. Before writing this
subsection, the `command_policies` claim was re-verified live (2026-08-09, not inherited from
Phase 108 D-06, per D-16):

```bash
$ grep -rn "command_policies" crates/nono-cli/src/ crates/nono-cli/data/
crates/nono-cli/data/profile-authoring-guide.md:83:### command_policies
crates/nono-cli/data/profile-authoring-guide.md:85:tool-sandbox policies live under `command_policies`. ...
crates/nono-cli/data/profile-authoring-guide.md:91:  "command_policies": {
crates/nono-cli/data/profile-authoring-guide.md:122:Look under `command_policies.commands.<command>.from.<caller>.invocation_policy` ...
crates/nono-cli/data/profile-authoring-guide.md:128:  "command_policies": {
crates/nono-cli/data/profile-authoring-guide.md:179:  "command_policies": {
```

**Zero hits inside `crates/nono-cli/src/`; six hits, all inside
`crates/nono-cli/data/profile-authoring-guide.md`.** The claim holds live: `command_policies` is a
documented profile surface with no `.rs` wiring anywhere in the fork today. **Rejection reason:** a
docs-only surface enforced through the fork's existing Windows path is a narrower absorb than
either full pole — it is not a genuine third option, it is a subset of Pole A's policy-model
component with no independent enforcement mechanism of its own. ADR-113's OD-1 section (the
permanently-considered-and-rejected general OAuth2 `client_credentials` route-wiring layer) is the
precedent shape for recording an option as named-and-rejected rather than silently dropped, so a
future absorb proposal does not re-propose "just adopt the policy model" as though it had never
been examined.

---

## D-09: Symmetric Scoring

| Criterion | Pole A | Pole B |
|---|---|---|
| **Per-command granularity** | Upstream's drivers grant per-command, per-invocation filesystem access and exec shims — `platform/linux.rs:3721-3734` (`expand_dynamic_tokens(&policy.fs_read, ...)` etc.) resolves fs grants per command, and `platform/linux.rs:124,2934` (`shims_by_command`, `materialize_shim_source()`) installs a per-command trampoline (`116-FEASIBILITY-MATRIX.md` rows 1-2). | The fork grants per-*invocation* via `nono run --profile` dispatched from the PreToolUse hook (`claude_code_hook.rs::run()`) — each tool call spawns one `nono run` with the profile's grant set, giving per-tool-call rather than per-declared-command granularity; the granularity unit is "one hook-mediated tool invocation," not a pre-declared command table. |
| **Engine-agnosticism** | Upstream's `tool-sandbox/` subsystem is engine-neutral by design — its entrypoint is the sandboxed process launch itself (`platform/linux.rs`/`platform/macos.rs` drivers), not any particular calling agent or IDE integration (`116-FEASIBILITY-MATRIX.md`, Engine-Agnosticism Grounding; `116-RESEARCH.md` §4). | This is the fork's named weak spot, stated plainly per the "primitive vs. entry point" distinction (`.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md`): the fork's underlying confinement primitive, `nono run --profile ... -- <engine.exe>`, is itself engine-neutral (Spike 003, VALIDATED — proven against `cmd.exe`, `powershell.exe`, and `python.exe` identically on Win11 26200.8390) — but its current ENTRY POINT is Claude Code's PreToolUse contract, wired through `claude_code_hook.rs::run()` and the shipped hook wrapper `nono-tool-hook.ps1`. Decoupling confinement from that one engine's hook contract is real, undone integration work, not a primitive-level limitation. |
| **Enforcement depth** | Upstream's Unix drivers make no Windows kernel-enforcement claim at all today, because `platform/windows.rs` does not exist — a hypothetical Pole-A Windows driver's enforcement depth is unbuilt and unproven, an honest gap rather than a disqualifying one. | This is the fork's strongest claim, stated without softening: kernel-enforced confinement on Windows via AppContainer + WFP + Job Object, built on `exec_strategy_windows/` primitives — `RestrictedToken`/`create_restricted_token_with_sid()` (`restricted_token.rs`), `AppliedLabelsGuard::snapshot_and_apply()` (`labels_guard.rs`), `AppliedDaclGrantsGuard::snapshot_and_apply()` (`dacl_guard.rs`), and the WFP service/driver lifecycle in `network.rs` (`install_windows_wfp_service()`, `probe_windows_wfp_readiness()`) plus `JobObjectHandle::create()`/`create_process_containment()` (`launch.rs`) — all cited in `116-FEASIBILITY-MATRIX.md`'s Fork Primitive Inventory. |
| **Fail-direction under layer failure** | Not yet built — no driver exists to test. A hypothetical Windows driver's fail-direction under partial-layer failure (e.g. WFP unavailable, AppContainer registration failure) is unproven either way. | Visible today in `execution_runtime.rs::execute_sandboxed()` and `exec_strategy_windows/launch.rs`: the `WindowsTokenArm::{WriteRestricted, BrokerLaunch, BrokerLaunchNoPty}` cascade (`launch.rs:1237-1278`, selected by `select_windows_token_arm()`) exists specifically to route around a kernel-loader OS behavior (`STATUS_DLL_INIT_FAILED`/`0xC0000142` under `WRITE_RESTRICTED` + `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`/`DETACHED_PROCESS`) by selecting a different token-construction mechanism rather than degrading permissions (`116-FEASIBILITY-MATRIX.md`, WRITE_RESTRICTED Worked Example); `WindowsSupervisorDenyAllApprovalBackend` (`exec_strategy_windows/mod.rs:259`) is a named fail-closed approval backend. |
| **Platform coverage** | Linux and macOS proven upstream since v0.65.0 (PR #1105); Windows is net-new work — the ledger confirms `platform/windows.rs` does not exist upstream at `v0.71.0`, and `windows-touch: 0 of 38` across every dispositioned commit in the subsystem's history (`116-DIVERGENCE-LEDGER.md` Headline). | Windows is proven via PR #4 (shipped, `nono-tool-hook.ps1` + `claude_code_hook.rs`); Linux and macOS are untested for this specific hook-based model — the fork has no host module for `tool-sandbox/`, `command_policy.rs`, or `lineage_cgroup.rs` on any platform (`116-RESEARCH.md` §3c, confirmed absent from the fork). |
| **ADR-86 library-vs-CLI boundary impact** | This is a wash between the two poles: both would live entirely in `nono-cli` (a new `crates/nono-cli/src/tool-sandbox/` or equivalent module tree), with the core `nono` library untouched either way, per `proj/ADR-86-library-boundary-convergence.md`'s Architectural Responsibility Map (policy, profiles, hooks, and UX are CLI-owned; the library applies only what clients place in `CapabilitySet`). | Also a wash on this criterion: the existing PreToolUse hook, `claude_code_hook.rs`, `hooks.rs`, and `exec_strategy_windows/` all already live in `nono-cli`; formalizing this path adds no new library-tier surface, per the same `proj/ADR-86-library-boundary-convergence.md` boundary. |
| **Ongoing divergence + maintenance cost** | `116-DIVERGENCE-LEDGER.md`'s Headline records **38** dispositioned commits across the subsystem's `v0.64.1..v0.71.0` history — the rate a Pole-A-absorbed subsystem would need continued syncing against on every future upstream sync, on top of authoring and then maintaining the net-new `platform/windows.rs` driver alongside the absorbed Unix drivers. | Formalizing removes `tool-sandbox/` from future upstream-sync scope entirely — the fork would no longer need to disposition or absorb any of upstream's `tool-sandbox/`-surface commits going forward — but this leaves the fork's own Windows-specific hook + broker + `exec_strategy_windows/` code as its own, separately-maintained surface, independent of upstream's release cadence. |

## Neutrality Self-Check

**7 criteria x 2 poles = 14 populated cells.** Re-reading the table above row by row confirms
every one of the 14 cells (7 rows, Pole A and Pole B columns each) is non-empty and phrased as a
capability fact or a citation, not a comparison. Re-scanning the whole document (Context, both Pole
sections, D-06, and the D-09 table) for comparative or recommending phrasing of any kind (a
sentence declaring one pole superior, preferred, victorious, or the other's inverse) finds none —
the engine-agnosticism row states the fork's weak spot plainly (per the "primitive vs.
entry point" distinction) without softening, and the enforcement-depth row states the fork's
kernel-enforced claim as its strongest claim without softening, while Pole A's corresponding gap is
phrased as "unbuilt"/"unproven"/"no driver exists to test" rather than "impossible", "cannot", or
"fails". No `## Decision` section, bolded verdict sentence, reversal triggers, Consequences
section, or Phase 120 sizing appears anywhere in this document — those are Plan 116-06's job.

---

## References

- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md` —
  the finalized `## Headline` figures this Context section cites (38 dispositioned commits,
  windows-touch 0/38, the 12-file/18,433-LOC/74%-Unix breakdown).
- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-FEASIBILITY-MATRIX.md` —
  the D-07 Fork Primitive Inventory and 11-row feasibility matrix this D-09 table's Pole A/Pole B
  cells cite by symbol.
- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-RESEARCH.md` §4 — the
  12-file/18,433-LOC upstream surface inventory and the `platform/` cfg-gate no-op-stub finding.
- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-CONTEXT.md` — D-05
  (two-pole framing), D-06 (rejected intermediates), D-09 (criteria set), D-10 (open-verdict
  posture), D-11 (Accepted-status gate, Plan 116-06's job).
- `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` — the "primitive
  vs. entry point" distinction and Spike 003's engine-neutrality validation.
- `.planning/quick/260528-sch-spec-the-sandbox-the-tools-windows-tool-/260528-sch-SPEC.md` — Pole
  B's design source.
- `proj/ADR-86-library-boundary-convergence.md` — the library-vs-CLI boundary D-09's sixth
  criterion cites.
- `proj/ADR-113-spiffe-disposition.md` — header/OD-1 house-style precedent for D-06's
  considered-and-rejected shape.

---

## Decision

**Formalize fork-native** (Pole B).

This verdict falls out of the D-09 scoring table above, read in full, weighed against the D-07
feasibility matrix's row-by-row tally computed in the section below (11 rows: 5 rated
`implementable-on-fork-primitives`, 4 rated `implementable-but-new-work`, 2 rated
`structurally-blocked` — 9 of 11 capabilities are buildable in principle, which rules out treating
Pole A as infeasible, but does not by itself decide the verdict). Three of the D-09 table's seven
criteria state a concrete, unsoftened advantage for Pole B rather than a wash. **Enforcement
depth** states the fork's Windows kernel-enforced confinement (AppContainer + WFP + Job Object) as
"the fork's strongest claim... stated without softening," against Pole A's Windows arm as "unbuilt
and unproven, an honest gap rather than a disqualifying one" — no driver exists yet to score.
**Fail-direction under layer failure** states the same asymmetry concretely: Pole B's
`WindowsTokenArm` cascade is "visible today," "exists specifically to route around" a
kernel-loader failure mode "rather than degrading permissions," while Pole A's hypothetical
driver's fail-direction is, again, "not yet built — no driver exists to test." **Ongoing
divergence + maintenance cost** states that formalizing "removes `tool-sandbox/` from future
upstream-sync scope entirely," while adopting inherits the 38-commit-and-growing sync burden
`116-DIVERGENCE-LEDGER.md` measured, on top of authoring and then maintaining a net-new driver
alongside it. The one criterion that states an equally concrete disadvantage for Pole B is
**Engine-agnosticism** — named plainly as "the fork's named weak spot," an "undone integration
problem" in the PreToolUse hook contract rather than in the confinement primitive itself — and
that gap is real and stays open regardless of this verdict (see Consequences below). The remaining
three criteria (per-command granularity, platform coverage, ADR-86 boundary impact) are stated
symmetrically in the table, with no comparative language in either cell, and do not move the
needle either way. D-08's `structurally-blocked` rating additionally caps what Pole A could ever
deliver beyond Pole B's existing model on the one capability class upstream's Landlock/Seatbelt
drivers reach that DACL/mandatory-label enforcement cannot (dynamic, per-open file-policy
decisions) — that capability is blocked by ADR-65's standing verdict for either pole, not unlocked
by choosing Pole A, so it is not counted here as a Pole-A-favoring fact.

---

## D-07/D-08: Feasibility Matrix Summary

Full detail lives in
`.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-FEASIBILITY-MATRIX.md`'s
`## D-07 Feasibility Matrix` (11 rows; not duplicated here). Its top-line tally: **5 of 11**
capabilities rated `implementable-on-fork-primitives` (exec shims via the shell-broker's
mediating-trampoline pattern, env scrubbing, the sealed-shim-runtime-dir analog via DACL guards,
per-command open-port mediation via native WFP port ranges, and credential resolution via
`keystore.rs`); **4 of 11** rated `implementable-but-new-work` (per-command filesystem-grant
dispatch, `url_shim`, `token_broker`-style per-command credential scoping, and wiring
`dynamic_providers`-equivalent expansion into the Windows capability-construction path); **2 of
11** rated `structurally-blocked`.

Both `structurally-blocked` rows apply to either pole identically, not to Pole A alone. The first
cites ADR-65 directly: fine-grained per-file read-policy decided dynamically at file-open time is
rated `structurally-blocked` because the only Windows mechanism capable of that class of dynamic,
per-open interception is a minifilter driver, and
`.planning/architecture/adr-65-minifilter-go-no-go.md` §6's standing verdict is **"lean No-go /
Conditional-go"** for a production minifilter — a standing precondition this document does not
revisit or presuppose changing.

The second is the matrix's worked example, restated here for this document's own
self-containment: a process holding a `WRITE_RESTRICTED` token combined with
`PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` or `DETACHED_PROCESS` fails its loader initialization at
`STATUS_DLL_INIT_FAILED` (`0xC0000142`) before the child's own code runs — reproducing for
console-hosted .NET/PowerShell CLR runtimes. This is why the fork's
`execution_runtime.rs::execute_sandboxed()` selects between the `BrokerLaunchNoPty` and
`WriteRestricted` token arms rather than constructing a single arm for every launch (the two are
mutually exclusive, `execution_runtime.rs:548-552`), and it is the reason `claude_code_hook.rs`'s
PreToolUse dispatch path (`run()`) can rely on that split already existing underneath it rather
than assuming one token strategy suffices for every tool invocation. No amount of engineering
effort inside the `WriteRestricted` arm itself would avoid this — the fork routes around it via a
different mechanism entirely, which is the structural distinction `structurally-blocked` means in
this matrix, not a higher-cost variant of the same mechanism.

---

## D-12: Reversal Triggers + Re-Test Point

This verdict is not permanent by assumption — it is reversible on named, individually checkable
conditions. If any of the following becomes true, this ADR's Decision should be re-opened rather
than silently assumed to still hold:

- **Upstream ships `platform/windows.rs`.** If upstream ever authors and ships a Windows driver
  for the `tool-sandbox/` subsystem itself, the D-09 "platform coverage" symmetry this Decision
  relies on (Windows proven via PR #4, Linux/macOS proven upstream, each pole with a mirror-image
  gap) breaks — Pole A would no longer require building the largest, riskiest component from
  scratch, materially changing the D-09 table's weighing.
- **A second agent engine needs confinement.** This Decision's Consequences section names
  engine-agnosticism as the fork's one honestly-stated weak spot. A second engine surfacing that
  gap in practice (not hypothetically) is direct evidence the gap is costing something concrete,
  strengthening the case to revisit.
- **ADR-65's minifilter verdict is revisited.** Both `structurally-blocked` rows in the D-07
  feasibility matrix — including the one capability class upstream's Landlock/Seatbelt drivers
  reach that this fork's current model cannot — are gated on ADR-65's standing No-go/Conditional-go
  verdict. If that verdict changes, the feasibility matrix's tally this Decision cites changes with
  it.
- **The Unix drivers stop growing.** `116-DIVERGENCE-LEDGER.md`'s Headline records 38 dispositioned
  commits across `v0.64.1..v0.71.0`, with 11 in the most recent `v0.69.0..v0.71.0` window alone —
  live, active growth. If that growth stops (upstream stabilizes or deprecates the subsystem), the
  "ongoing divergence + maintenance cost" criterion this Decision cites as Pole-B-favoring no longer
  applies in the same way.
- **The D-09 table's engine-agnosticism gap is closed independently.** If a future phase decouples
  `nono run`'s confinement primitive from Claude Code's PreToolUse hook contract (the
  engine-abstraction/`nono-agentd` line named in `116-CONTEXT.md`'s Deferred Ideas), Pole B's one
  clearly-stated weak spot disappears — removing the single criterion that currently favors Pole A
  and making a future re-evaluation of this Decision unnecessary rather than more urgent, the
  inverse-direction trigger worth naming explicitly alongside the other four.

Re-tested at the next UPST sync (UPST13/FUT-08).

A trigger list nobody is obliged to look at is how this subsystem became a standing unexamined
divergence in the first place — naming the next UPST sync as the re-test checkpoint keeps this
obligation live, not implicit.

---

## Phase 120 Scope (D-14/D-15)

**Because this ADR's verdict is Pole B (formalize fork-native), Phase 120's scope is the smaller
"close the divergence by naming the boundary" shape, not a row-by-row build-out of the feasibility
matrix.** The banding rule is stated verbatim below, per D-15, because it still governs the small
set of deliverables that remain — but the feasibility matrix's 9 non-`structurally-blocked` rows
are explicitly **not** sized as deliverables here: each of those rows describes a capability
upstream's `platform/{linux,macos}.rs` drivers provide that a from-scratch `platform/windows.rs`
(Pole A) would need to build. Under Pole B, none of that build-out happens, so none of those rows
convert into Phase 120 work. This mirrors ADR-111's and ADR-113's Consequences shape (record a
permanent boundary, not a build-out).

**Banding rule (verbatim, per D-15):**
- S = wraps/extends one existing fork primitive with no new OS surface
- M = a new fork primitive is needed but no new kernel/OS mechanism
- L = a new OS-mechanism integration is required
- Commit counts are explicitly not the sizing basis — one commit in the audited window is a
  6,586-line Landlock-equivalent driver, another is a one-line fix; LOC arithmetic off either would
  be false precision.

**Phase 120 deliverables (closing-boundary shape, substituted for a row-by-row build-out):**

1. Formalize PR #4's PreToolUse-hook + Low-IL-broker path as a permanent, named scope boundary in
   a standalone operator-facing artifact (this ADR already begins that work; Phase 120 finishes
   recording it in the ADR-111/ADR-113 shape — a `116-DIVERGENCE-LEDGER.md` carry-forward note plus
   the `ROADMAP.md`/`REQUIREMENTS.md` amendment this section proposes) — **(S)**, wraps the
   existing hook + `nono run` + broker primitives, no new OS surface.
2. Close every one of `116-DIVERGENCE-LEDGER.md`'s 38 dispositioned tool-sandbox rows against this
   verdict (each row's disposition becomes "will not absorb, see ADR-116" rather than an open
   question) — **(S)**, a recording task over already-dispositioned rows, no new code.
3. Apply this ADR's proposed `ROADMAP.md`/`REQUIREMENTS.md` amendment (Phase 120's own
   goal/success-criteria text, TSBX-03's completion condition) once the operator ratifies —
   **(S)**, documentation only.
4. Name the engine-agnosticism gap (D-09's one Pole-A-favoring criterion, and D-12's fifth trigger
   above) as its own tracked item: decouple `nono run`'s confinement primitive from the
   PreToolUse hook's Claude-Code-specific contract — the one deliverable in this list that is a
   genuine build item, not a naming task — **(M)**, a new integration layer is needed (an
   engine-neutral entry point) but no new kernel/OS mechanism, since the underlying primitive
   (`nono run --profile ... -- <engine.exe>`) is already proven engine-neutral (Spike 003).

This closing-boundary list carries 4 tagged deliverables (three **(S)**, one **(M)**) —
substituted here for a row-by-row build-out of the feasibility matrix's 9 non-blocked rows, since
Pole B's verdict means none of those upstream-driver capabilities get built from scratch.

**In-v3.7 vs. FUT-09:** Items 1-3 are recording/ratification work sized to land inside Phase 120's
own v3.7 slot. Item 4 (the engine-agnosticism decoupling) is the one item genuinely open-ended
enough to risk overrunning Phase 120's slot; per `REQUIREMENTS.md`'s FUT-09 wording ("conditional:
only exists as future work if TSBX-02 returns *adopt* and TSBX-03 cannot fully land inside v3.7"),
FUT-09 as currently worded is conditioned on an *adopt* verdict and therefore does not apply
verbatim to this Pole-B outcome — if item 4 cannot land inside Phase 120's v3.7 slot, that overflow
needs its own future-work naming distinct from FUT-09's literal adopt-conditioned text, a
correction this document flags rather than silently reusing FUT-09's wording for a case it was not
written to cover.

This section is a proposal for ROADMAP.md's Phase 120 entry — it is not applied by this phase;
ratification and `.planning/ROADMAP.md` edits happen only after operator sign-off (D-11 gate).

---

## Consequences

1. **Ledger closure confirmed, not reopened.** `116-DIVERGENCE-LEDGER.md`'s Headline states
   "**Ledger closed except for named exceptions**" — the two named exceptions (the 12-commit
   Uncovered-Window Finding and the 3-item Post-Fence Residue Finding) remain open, operator-gated
   proposals, unaffected by this ADR's verdict; this ADR's Decision does not reopen or alter that
   closure.

2. **Permanent vs. deferred under this verdict.** PR #4's PreToolUse-hook + `nono run` +
   Low-IL-broker path becomes a **permanent, named scope boundary** (ADR-111/ADR-113 shape) — not a
   placeholder awaiting a later absorb. Upstream's `tool-sandbox/` subsystem is **not** adopted; a
   future sync that touches it must re-affirm this decision by citing this ADR, the same obligation
   ADR-111 places on future `resource`-module syncs, rather than treat non-adoption as an
   unabsorbed gap to close. **Deferred, not decided by this verdict:** the engine-agnosticism gap
   named in D-09 (Phase 120 Scope item 4) stays open work, independent of this Decision.

3. **What future syncs must re-affirm.** The D-12 trigger list above — not a reasoning paragraph —
   is what a future UPST sync (starting with UPST13/FUT-08) must check against before treating this
   verdict as stale.

4. **Fork invariants unaffected by either pole (D-18).** The Windows security model — AppContainer,
   Low-IL, WFP, Job Object — and `proj/ADR-86-library-boundary-convergence.md`'s policy-free
   library boundary are unchanged by this Decision; nothing in either pole's evaluation touches
   `crates/nono/src/`, and this ADR's own D-09 table's ADR-86 criterion recorded that impact as a
   wash for both poles.

5. **Phase 120 planning is operator-gated.** Per D-11, Phase 120 planning does not begin until the
   operator ratifies this ADR (`Status: Accepted` here records the executor's derivation of the
   verdict from evidence, not operator sign-off — those are two distinct gates, per the
   Manual-Only Verifications table in `116-VALIDATION.md`).

6. **Phase 117 CINT-01 fail-direction cross-check: a real wording difference, named rather than
   reconciled.** `ROADMAP.md`'s Phase 117 SC2 requires: "Starting a confined session with a layer
   forced unavailable produces either an **abort** or a visibly **downgraded** claim; there is no
   path on which nono presents a confinement guarantee it did not confirm." This document's own
   D-09 fail-direction row describes a third outcome Phase 117's two-option framing does not name:
   the fork's `WindowsTokenArm` cascade (`WriteRestricted`/`BrokerLaunch`/`BrokerLaunchNoPty`)
   responds to a layer failure (the `WRITE_RESTRICTED`-plus-PTY loader crash) by **switching to a
   different token-construction mechanism that preserves the same confinement class**, rather than
   aborting or downgrading. Whether that substitution counts as a legitimate third outcome under
   Phase 117's eventual contract, or whether Phase 117's contract should be read as implicitly
   permitting "route to an equivalent-strength alternate mechanism" as a form of
   neither-abort-nor-downgrade, is not resolved here — Phase 117 owns that contract and has not yet
   run. This is named explicitly rather than silently reconciled, per this plan's own instruction,
   and left for Phase 117 to settle on its own terms.

---

## References

- Pinned upstream SHAs: `0551eba27ea53b0eda20f4f796f757dd168adb92` (`v0.64.1`),
  `0055bf3c686de6fa665decd9831dc5686b282cb0` (`v0.71.0`) — the window this ADR's Context and both
  ledgers audit.
- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-DIVERGENCE-LEDGER.md` —
  the closed per-commit ledger this Decision and Consequences sections cite.
- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-FEASIBILITY-MATRIX.md` —
  the D-07 feasibility matrix this Decision's numeric tally and the Phase 120 Scope section's
  "not sized as deliverables" reasoning are drawn from.
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library boundary this Decision's
  Consequences item 4 confirms is unaffected.
- `proj/ADR-111-resource-limits-boundary.md` — the "permanent scope boundary, future syncs must
  re-affirm" Consequences shape this Decision mirrors.
- `proj/ADR-113-spiffe-disposition.md` — the second precedent for the same Consequences shape
  (OD-1).
- `.planning/architecture/adr-65-minifilter-go-no-go.md` — the standing minifilter
  No-go/Conditional-go verdict D-08 and this Decision's D-07/D-08 summary cite.
- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-CONTEXT.md` —
  D-10/D-11/D-12/D-14/D-15, the locked decisions this Decision and Phase 120 Scope section satisfy.
- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-RESEARCH.md` — the
  underlying research this ADR's Context section draws its measured figures from.
- `.planning/ROADMAP.md` — Phase 117's CINT-01 entry, cross-checked against this Decision's
  fail-direction row in Consequences item 6.
