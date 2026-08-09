# ADR-116: Tool-Sandbox Divergence — Adopt vs. Formalize Fork-Native

**Status: Proposed**
**Phase:** 116 — Tool-Sandbox Divergence Audit + Disposition ADR
**Date:** 2026-08-09
**Authors:** Phase 116 execution

<!-- Status flips to Accepted only in Plan 116-06's Decision task, per D-11. This document is not
final until then. -->

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
