# Phase 110: Profile/Policy Absorb + platform_overrides - Context

**Gathered:** 2026-07-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb upstream's per-OS profile-patch model (`platform_overrides`) and the v0.67–v0.68
profile/policy features, retire the fork's top-level `windows_*` flag sprawl into
`platform_overrides.windows`, and land the port-range profile schema with a **WFP-native**
Windows emitter.

**Requirements: PROF-01, PROF-02, PROF-03, PROF-04.**

| Commit | Req | What |
|---|---|---|
| `ae1c513e` (#1371) | PROF-01 | `platform_overrides` field for per-OS profile patches |
| `719975cf` (#1380) | PROF-01 | preserve `platform_overrides` through `extends` resolution |
| `2cbaa9a0` (#1296) | PROF-02 | `$VAR` process-env token expansion in profile paths |
| `d4927f95` (#1298) | PROF-02 | `@git:*` dynamic token expansion — **has a deferred-subsystem dependency, see D-02** |
| `d5803b99` (#1398) | PROF-03 | port-range schema + emitters — **absorbed WHOLE here, see D-05** |
| `b620ed8e` (#1305) | PROF-04 | `bun` runtime preset (`policy.json`) |
| `f016b2d5` (#1387) | PROF-04 | `mise` runtime preset (`policy.json`) |

**Out of scope:** the 3 unmapped PROF-cluster commits (`0374e454`, `9ef59181`, `f58c7c22`)
belong to **Phase 112**. NET work is done (Phase 109). SPIFFE is Phase 113.

</domain>

<decisions>
## Implementation Decisions

### PROF-02 — the deferred-subsystem dependency

- **D-01 (finding): the dependency is `cfg`-gated, and upstream already ships a non-Unix fallback.**
  `d4927f95` opens `capability_ext.rs` with:
  ```
  #[cfg(any(target_os = "linux", target_os = "macos"))]
  use crate::tool_sandbox::dynamic_providers::expand_dynamic_tokens;

  #[cfg(not(any(target_os = "linux", target_os = "macos")))]
  fn expand_dynamic_tokens(entries: &[String], _workdir: Option<&Path>) -> nono::Result<Vec<String>>
  ```
  So on **Windows** this absorbs with no `tool_sandbox` dependency at all. The problem is only the
  **Unix arm** — and it is a real problem, not theoretical: the fork ships Linux and macOS binaries
  and **cross-target clippy is mandatory**, so a dangling `crate::tool_sandbox` path fails the gate.

- **D-02: PORT `expand_dynamic_tokens` into a fork-owned module.** Lift just that function out of
  upstream's `tool-sandbox/dynamic_providers.rs` into a fork module (e.g. alongside
  `capability_ext.rs`, or a small dedicated token-expansion module — planner's choice), so **both**
  `cfg` arms resolve without pulling the tool-sandbox subsystem forward. `@git:*` then works on all
  three platforms and cross-target clippy passes.
  - **Do NOT import `crate::tool_sandbox::*`** — that module does not exist in the fork and must not
    be created as a side effect of this phase.
  - **MANDATORY v3.7 reconciliation note:** record in the SUMMARY and the finding trail that this
    one function arrived early. v3.7 must **reconcile** it (adopt the fork's copy, or replace it and
    delete the fork module) rather than blindly introducing a second copy. A silent duplicate is the
    likely failure mode.

- **D-03: `$VAR` (#1296, `2cbaa9a0`) has no such dependency** and absorbs normally. PROF-02 covers
  both features; only `@git:*` needed the decision above.

### PROF-03 — port ranges

- **D-04: ADR-86 is INTACT — `capability.rs +155` is mechanism, not policy.** Inspected during
  discussion. It adds `MACOS_PORT_RANGE_LIMIT` (2¹⁴ = 16,384, because `sandbox_init()` SIGILLs above
  ~17,770 Seatbelt rules), a pure `merge_port_ranges(&[(u16,u16)]) -> Vec<(u16,u16)>` helper, and a
  `localhost_port_ranges` field with a range-allow method on the capability type. That is exactly
  what `CapabilitySet` exists to express — same conclusion shape as Phase 109's finding on SPIFFE's
  audit types. No policy or enforcement decision enters the library.

- **D-05: Phase 110 absorbs `d5803b99` WHOLE — schema + `capability.rs` + BOTH Unix emitters +
  the Windows emitter.** One commit, absorbed once, by one phase, so the port-range feature is
  coherent and testable in a single step rather than landing its shared `capability.rs` contract in
  one phase and a consumer in another.
  **Roadmap amendment applied 2026-07-30:** Phase 111's CORE-01 and its SC1 have had the `#1398`
  `macos.rs` port-range emitter claim REMOVED. **Phase 111 must not re-absorb it.** CORE-01 retains
  `#1378` (`~/.cache`) and `#1424` (`MAX_CRYPTO_THREADS`).

- **D-06: the Windows emitter uses WFP-native ranges — no unrolling, no macOS-style limit.**
  WFP expresses remote-port ranges natively. Do **not** mimic Seatbelt's per-port unroll, and do
  **not** propagate `MACOS_PORT_RANGE_LIMIT` to Windows — that cap exists solely because macOS turns
  each port into an individual Seatbelt rule. Per the upstream doc comments: macOS unrolls and is
  capped at 16,384; Linux accepts the full 16-bit space (1–65535); Windows/WFP needs neither.

- **D-07: the Windows emitter is FORK-ORIGINAL enforcement code and gets its own threat model.**
  Upstream implemented only macOS and Linux. This is the first genuinely new kernel-facing surface
  in v3.6, so it is written, not ported — treat it accordingly. Discrete-`Vec<u16>` back-compat is
  required (SC3): existing profiles using discrete port lists must keep working.

### PROF-01 — `platform_overrides` and the flag migration

- **D-08: back-compat contract — BOTH forms accepted, `platform_overrides.windows` WINS, no
  deprecation warning this milestone.** Existing profiles keep loading unchanged (SC1's explicit
  requirement); when a setting appears both as a top-level `windows_*` flag and under
  `platform_overrides.windows`, the new form takes precedence. Deprecation signalling is deliberately
  deferred until the new form is proven — adding a warning now would fire on every existing profile.

- **D-09: the migration must cover BOTH declaration sites.** `windows_low_il_broker` and
  `windows_interpreters` are declared at `crates/nono-cli/src/profile/mod.rs:2391/2398` **and again
  at ~2468–2475** (a second struct with its own serde handling — note the existing doc comment about
  deserializing "any profile JSON containing `windows_interpreters`"). Migrating only one site leaves
  a silently-diverging second path.

- **D-10: `#1380` (`719975cf`) is the `extends`-preservation fix and is required, not optional.**
  The fork already has profile `extends` (#1320 is present per the parity map). Without #1380,
  `platform_overrides` is silently dropped during `extends` resolution — a config that appears
  applied and isn't. That is the same silent-no-op failure class this milestone keeps surfacing.

### PROF-04

- **D-11: `bun` (#1305) and `mise` (#1387) are `policy.json` + roundtrip-test changes only.**
  Low risk. Verify the presets are actually *resolvable* (SC4), not merely present in the JSON —
  presence without wiring is the failure mode.

### Standing Rules (carried, not re-litigated)

- **D-12: cross-target clippy is MANDATORY for this phase.** `d5803b99` alone touches
  `crates/nono/src/sandbox/macos.rs` (+150), `crates/nono/src/sandbox/linux.rs` (+28), and
  `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (+81). Both gates (`cross` linux-gnu,
  `cargo-zigbuild` apple-darwin) GREEN locally — **no PARTIAL→CI**. `make ci`, not clippy alone.
- **D-13: rebuild BOTH bindings** if any `nono-proxy`/`nono` struct changes — `maturin build`
  (`../nono-py`), `napi build --platform --release` (`../nono-ts`). Phase 109 proved this is
  load-bearing: `../nono-py` failed to compile with `E0063` on the new `ProxyConfig` fields.
  `capability.rs` gains a field here, so expect drift.
- **D-14: verify feature presence by BEHAVIOR, never by name, and confirm the fork actually HAS a
  file before dispositioning a commit `adopt`.** See `<specifics>` — this has now failed seven times
  in this milestone.

### Claude's Discretion

- Where the ported `expand_dynamic_tokens` lives (module name/placement), provided it is fork-owned
  and not under a `tool_sandbox` path.
- Plan/wave decomposition, provided `d5803b99` (the largest, cross-cutting commit) is not bundled
  into one plan with the trivial PROF-04 preset changes.
- Whether `bun` and `mise` share a plan (they almost certainly should).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Work-list authority
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` §"PROF Cluster — Per-Commit Table" — per-commit dispositions, windows-touch, re-export scans. **Authoritative over the `260727-jkn` parity map.**
- `.planning/phases/108-upst12-divergence-audit/108-CONTEXT.md` — D-05/D-06/D-07 (tool-sandbox fencing, residue accounting), and the CORRECTED measurement blocks.

### Precedent for this phase's decisions
- `.planning/phases/109-proxy-network-absorb/109-CONTEXT.md` — the D-11 "verify by behavior, not name" rule and its CORRECTED block; the same ADR-86 inspection pattern applied to SPIFFE.
- `proj/ADR-108-deny-domain-posture.md` — how a fork-divergence posture decision is written up.

### Fork invariants
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free library boundary (D-04 reasons against it).
- `CLAUDE.md` — Library-vs-CLI boundary table; **cross-target clippy MUST/NEVER**; Landlock's allow-list-only constraint; path-security rules; DCO sign-off.
- `.planning/templates/cross-target-verify-checklist.md` — single source of truth for both cross-target gates.

### Code surfaces this phase edits (verified present in the fork)
- `crates/nono-cli/src/profile/mod.rs` — `windows_low_il_broker`/`windows_interpreters` at **:2391/:2398 and ~:2468–2475** (D-09 requires both).
- `crates/nono-cli/src/capability_ext.rs` — where `$VAR`/`@git:*` expansion lands.
- `crates/nono/src/capability.rs` — `CapabilitySet`; gains `localhost_port_ranges` + `merge_port_ranges`.
- `crates/nono/src/sandbox/{macos.rs,linux.rs}`, `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — cfg-gated Unix; **trigger D-12**.
- `crates/nono-cli/data/policy.json` — `bun`/`mise` presets.
- `crates/nono/schema/capability-manifest.schema.json`, `crates/nono-cli/data/nono-profile.schema.json`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Profile `extends` already exists** in the fork (#1320 present) — `#1380` is a preservation fix on top, not a new mechanism.
- **`merge_port_ranges`** is a pure function usable by all three platform emitters, though only macOS/Linux need the unroll it feeds.
- **The fork's existing `windows_*` flags already carry serde handling and doc comments** describing legacy-JSON acceptance — the back-compat shape D-08 wants is partly there already.
- **Phase 109's absorb pattern** (validators ported, wired at real call sites, omission-detectable acceptance criteria) is the template that worked.

### Established Patterns
- **CapabilitySet holds mechanism; the CLI holds policy** (ADR-86). D-04 confirms `#1398` respects it.
- **Platform emitters diverge deliberately** — Seatbelt unrolls, Landlock is allow-list-only, WFP does ranges natively. Do not force uniformity.
- **Silent no-ops are the recurring bug class here** — see D-10, D-11, and Phase 109's `proxy_config.no_proxy` never-assigned catch.

### Integration Points
- `platform_overrides` must survive `extends` resolution (D-10) — the seam where it can silently vanish.
- `capability.rs`'s new field flows to all three sandbox backends and to the C FFI/bindings (D-13).
- The ported `expand_dynamic_tokens` is a v3.7 reconciliation point (D-02).

</code_context>

<specifics>
## Specific Ideas

- **Seven times in this milestone, an upstream assumption about the fork turned out false.** D-06's
  `*tool-sandbox*` glob matching a docs file; `#1415`'s `no_proxy_hosts` being a different mechanism;
  `#1335`'s `HTTP_PROXY` pointing the opposite direction; `#1430`/`#1437` targeting directories the
  fork lacks; `managed_loopback_upstream`, `parse_non_connect_target`, and upstream's
  `forward.rs`/`UpstreamSpec` abstraction all described as existing when absent.
  **Every instance was a name or path match mistaken for fork presence.** Before writing a task that
  edits file `X` or calls function `Y`, confirm `X`/`Y` exists in the fork — and expect the plan's
  own `<interfaces>` block to be wrong about this, because it has been repeatedly.
- The macOS 16,384-port cap has a real crash behind it (`sandbox_init()` SIGILL above ~17,770 rules).
  Keep that rationale in the code comment when carrying it — a bare magic number invites removal.
- D-11's "resolvable, not merely present" applies more widely than PROF-04: a schema key that
  deserializes but reaches no enforcement path is this milestone's most-caught defect.

</specifics>

<deferred>
## Deferred Ideas

- **The 3 unmapped PROF-cluster commits → Phase 112**: `0374e454` (#1400/#1402 omit inheritable
  `Option` fields when `None` on save), `9ef59181` (credential-provider doc comment), `f58c7c22`
  (#1320 CLI profile `extends` — already present in the fork).
- **v3.7 reconciliation of the ported `expand_dynamic_tokens`** (D-02) — must adopt-or-replace, never
  silently duplicate.
- **Gray areas raised but not discussed** (available if planning needs them): whether
  `platform_overrides` should also absorb the fork's other platform-divergent settings beyond the two
  named flags; whether `bun`/`mise` presets need Windows interpreter entries to be meaningful on this
  fork; and deprecation-warning timing for the legacy `windows_*` flags (D-08 defers it — revisit
  when the new form is proven).

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md`, `20260611-poc-cert-broker-clean-host.md` — host-gated v3.5 distribution items owned by v3.5 Phase 106; unrelated to a profile/policy absorb.

</deferred>

---

*Phase: 110-profile-policy-absorb-platform-overrides*
*Context gathered: 2026-07-30*
