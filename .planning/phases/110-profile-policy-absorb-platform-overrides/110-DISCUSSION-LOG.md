# Phase 110: Profile/Policy Absorb + platform_overrides - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-30
**Phase:** 110-profile-policy-absorb-platform-overrides
**Areas discussed:** PROF-02 deferred-subsystem dependency, `d5803b99` cross-phase split, WFP-native emitter scope, PROF-01 back-compat contract

Every question resolved to the recommended option. No free-text overrides; no scope creep raised.

---

## Scouting findings that shaped the questions

Three things measured before any question was asked:

**PROF-02's dependency is `cfg`-gated.** `d4927f95` guards its `tool_sandbox` import behind
`#[cfg(any(target_os = "linux", target_os = "macos"))]` and ships a local fallback `fn` for
everything else. So Windows absorbs cleanly; only the Unix arm is a problem — and it is a real one,
since the fork ships Linux/macOS binaries and cross-target clippy would fail on a dangling
`crate::tool_sandbox` path.

**PROF-03's core-library touch is mechanism, not policy.** `capability.rs +155` adds
`MACOS_PORT_RANGE_LIMIT` (2¹⁴, guarding a real `sandbox_init()` SIGILL above ~17,770 Seatbelt
rules), a pure `merge_port_ranges()`, and a `localhost_port_ranges` capability field. ADR-86 intact.

**`d5803b99` is far bigger than the roadmap implies**, and two phases claimed pieces of it:

| File | Lines | Note |
|---|---|---|
| `crates/nono/src/capability.rs` | +155 | core library |
| `crates/nono/src/sandbox/macos.rs` | +150 | cfg-gated Unix — *Phase 111's CORE-01 claimed this* |
| `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` | +81 | cfg-gated Unix |
| `crates/nono/src/sandbox/linux.rs` | +28 | cfg-gated Unix |
| `crates/nono/src/manifest_convert.rs` | +40 | core library |

Also established: SC3's WFP-native Windows emitter is **fork-original work** — upstream implemented
only macOS and Linux.

---

## PROF-02 — deferred-subsystem dependency

| Option | Description | Selected |
|--------|-------------|----------|
| Port `expand_dynamic_tokens` into a fork-owned module | Both cfg arms resolve without pulling tool-sandbox forward; `@git:*` works on all platforms; cross-target clippy passes | ✓ |
| Use the non-Unix stub on ALL platforms | Zero v3.7 leakage, but `@git:*` silently doesn't expand on Linux/macOS | |
| Defer PROF-02 to v3.7 entirely | Cleanest boundary; needs a REQUIREMENTS amendment | |
| Absorb `$VAR` only, defer `@git:*` | Splits the requirement; only `@git:*` needs `dynamic_providers` | |

**User's choice:** port the function into a fork-owned module.
**Notes:** one function of v3.7 code arrives early. CONTEXT D-02 makes a v3.7 reconciliation note mandatory — v3.7 must adopt-or-replace, never silently duplicate.

---

## `d5803b99` cross-phase split

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 110 takes the whole commit; 111's CORE-01 drops #1398 | One commit absorbed once; capability.rs contract and all consumers land together | ✓ |
| Split by file as the roadmap implies | Honors current wording, but half-absorbs a commit across phases | |
| Phase 111 takes the whole commit | Keeps 110 purely profile/policy; delays the WFP emitter to the last phase | |

**User's choice:** Phase 110 takes it whole.
**Notes:** operator elected to apply the resulting ROADMAP/REQUIREMENTS amendment immediately rather than defer, so Phase 110 plans against unambiguous ownership.

---

## WFP-native port-range emitter

| Option | Description | Selected |
|--------|-------------|----------|
| Native ranges, no unrolling, own threat model | WFP expresses ranges natively; no `MACOS_PORT_RANGE_LIMIT` equivalent; first new kernel-facing surface in v3.6 | ✓ |
| Mirror the Unix unrolling approach | Cross-platform code symmetry; discards WFP's native capability and inherits a limit Windows doesn't have | |
| Defer the Windows emitter | Smallest change; leaves Windows accepting port-range config it doesn't enforce — a silent no-op | |

**User's choice:** native ranges with its own threat model.

---

## PROF-01 back-compat contract

| Option | Description | Selected |
|--------|-------------|----------|
| Both forms accepted, new form wins, no deprecation warning yet | Existing profiles load unchanged (SC1); least churn | ✓ |
| Both accepted, new wins, WARN on legacy | Drives migration faster; warning fires on every existing profile | |
| Both accepted, ERROR if both set | Removes precedence ambiguity; new failure mode for partial migrations | |

**User's choice:** both accepted, new form wins, warning deferred.
**Notes:** CONTEXT D-09 additionally requires the migration cover BOTH declaration sites in `profile/mod.rs` (:2391/:2398 and ~:2468–2475) — migrating one leaves a silently-diverging second path.

---

## Roadmap amendment applied during discussion

Phase 111's SC1 and REQUIREMENTS CORE-01 had the `#1398` `macos.rs` port-range emitter claim
removed, with an explicit "Phase 111 must not re-absorb it" note. CORE-01 retains `#1378` and `#1424`.

## Claude's Discretion

- Module name/placement for the ported `expand_dynamic_tokens`, provided it is fork-owned and not under a `tool_sandbox` path.
- Plan/wave decomposition, provided `d5803b99` isn't bundled with the trivial PROF-04 preset changes.
- Whether `bun` and `mise` share a plan.

## Deferred Ideas

- The 3 unmapped PROF-cluster commits (`0374e454`, `9ef59181`, `f58c7c22`) → Phase 112.
- v3.7 reconciliation of the ported `expand_dynamic_tokens`.
- Not discussed, available if planning needs them: whether `platform_overrides` should absorb other platform-divergent fork settings beyond the two named flags; whether `bun`/`mise` need Windows interpreter entries; deprecation-warning timing for the legacy `windows_*` flags.
- Reviewed todos, not folded: `20260611-msi-vcredist-prereq.md`, `20260611-poc-cert-broker-clean-host.md` (v3.5 Phase 106).
