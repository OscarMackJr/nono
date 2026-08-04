# ADR-111: Resource-Limit CLI Surface — Reject the Core-Module Absorb

**Status:** Accepted
**Phase:** 111 — Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog
**Date:** 2026-08-04
**Authors:** Phase 111 execution

---

## Context

Upstream `nolabs-ai/nono` commit `e6d26871f0498e7dc7a867e67af5c6136b84f91c` (#1269, "feat:
resource limiting") adds `pub mod resource;` and `pub use resource::ResourceLimits;` to the
policy-free core library's `crates/nono/src/lib.rs`, alongside a new 23-file diff spanning
`cli.rs`, `command_runtime.rs`, `exec_strategy.rs`, `sandbox_state.rs`, a new
`crates/nono-cli/src/resource_cgroup.rs`, and a new `crates/nono/src/resource/mod.rs`. Its
sibling `34c2c975d649844923cf1515be94de624c689c6c` (#1403, "feat(resources): cap sandbox process
count with --max-processes") extends the same module with a 15-file diff, adding no further
`pub mod`/`pub use` items beyond what `e6d26871` already introduced.

Both commits were classified `adapt` (not `adopt`) by the Phase 108 divergence audit's CORE
cluster table (`108-DIVERGENCE-LEDGER.md`, "CORE Cluster — Per-Commit Table"), which also raised
an explicit Threat Flag: `threat_flag: cross-crate-reexport` against `crates/nono/src/lib.rs`,
noting this is "the first CORE-cluster commit in this window to cross the library/CLI boundary
rather than stay `pub(crate)`/CLI-internal" and directing Phase 111 to settle the disposition
before any absorb.

The Phase 108 audit also hand-verified, by grepping every line of both diffs for the string
`windows`, that **neither commit mentions Windows anywhere** — not in code, not in a doc comment.
`e6d26871` and `34c2c975` implement their enforcement entirely through
`crates/nono-cli/src/resource_cgroup.rs` (Linux cgroup v2), with zero Windows Job Object
counterpart authored upstream. This means CORE-02 ("reconciled with the fork's existing
kernel-enforced Job Object implementation") was never an adopt-then-verify task — there is no
upstream Windows code to adapt in the first place.

By contrast, this fork already ships all four resource limits —
`--cpu-percent`/`--memory`/`--timeout`/`--max-processes` — with genuine kernel enforcement on
**three** platforms and three structurally different mechanisms:

- **Windows:** Job Object limits (`JobMemoryLimit`, `ActiveProcessLimit`,
  `TerminateJobObject`) in `crates/nono-cli/src/exec_strategy_windows/launch.rs`.
- **Linux:** cgroup v2 enforcement in `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`
  (`mod cgroup`).
- **macOS:** `RLIMIT_AS` + `RLIMIT_NPROC` enforcement in
  `crates/nono-cli/src/exec_strategy/supervisor_macos.rs`.

The fork's implementation is materially more complete than upstream's un-enforced core module —
upstream's `ResourceLimits` type carries the *shape* of a resource limit, not its enforcement.

---

## Decision

**ADAPT, not adopt.**

`crates/nono/src/lib.rs` will **NOT** gain `pub mod resource;` or `pub use
resource::ResourceLimits;`. `crates/nono/src/resource.rs` (or a `resource/` module directory)
will not be created. All resource-limit code — flag parsing, per-platform dispatch, and
enforcement — stays `nono-cli`-side, exactly as it is today.

Only flag names, help text, and semantics are aligned with upstream where upstream's spelling
happens to match the fork's. Per D-04 (Phase 111 locked decision), no rename was needed this
phase: both upstream commits spell all four flags (`--cpu-percent`, `--memory`, `--timeout`,
`--max-processes`) identically to the fork's existing, shipped, public CLI surface. The fork's
flag surface is frozen — back-compat wins. If a future upstream sync introduces a differently
spelled or differently ranged flag, the correct response is an **alias**, never a rename or a
silent change of units/ranges (precedent: Phase 110-01's handling of
`windows_low_il_broker`/`windows_interpreters`).

---

## Why the ADR-86 audit/diagnostics carve-out does NOT extend here

ADR-86 (`proj/ADR-86-library-boundary-convergence.md`) relocated the audit stack and structured
diagnostics **into** the core library, on the grounds that "audit and diagnostic modules are
observability primitives — they observe and report on sandbox operations; they do not define
security policy, grant permissions, or apply sandbox restrictions." That carve-out is the closest
prior-art precedent for "should this move into the library," so ADR-111 must explain, not merely
assert, why the same reasoning produces the opposite answer for resource limits.

The distinction is what each surface *does*, not what it's named:

- **Audit and diagnostics are read-only observers.** `AuditRecorder`, ledger append/verify, and
  `DiagnosticFormatter`'s data layer (`crates/nono/src/diagnostic/`) record and describe facts
  about a sandbox operation that has already happened, or format a human-readable explanation of
  a decision made elsewhere. Moving them into the library changes *where the code that watches
  and reports lives*; it does not change *what the sandbox is allowed to do*. ADR-86's own
  Consequences section is explicit on this: "the library still applies ONLY what clients
  explicitly add to `CapabilitySet`."

- **Resource limits are the enforcement mechanism itself.** A `ResourceLimits` value is not a
  record of something that happened — it is a live cap the OS kernel is instructed to impose
  (Job Object memory ceiling, cgroup v2 `memory.max`/`cpu.max`/`pids.max`, `RLIMIT_AS`/
  `RLIMIT_NPROC`). Relocating the type — even as "just a data type" — pulls the seam of *which
  limits apply and how they are dispatched per platform* toward the library boundary, on the
  enforcement side of the line ADR-86 drew, not the observability side.

- **ADR-86's own boundary invariant confirms this reading.** ADR-86 states the library owns
  "observability primitives, not security policy," while `nono-cli` owns "all diagnostic
  rendering... and audit command UX." Resource-limit enforcement is neither UX nor
  observability — it is exactly the kind of security-policy dispatch logic ADR-86 leaves with
  the CLI (compare: policy groups, deny rules, and `ExecStrategy` selection are all listed as
  CLI-owned in `CLAUDE.md`'s "Library vs CLI Boundary" table; per-platform resource enforcement
  is architecturally the same class of thing).

- **Relocating working, tested, security-critical code for cosmetic convergence buys nothing
  functional.** Unlike the audit/diagnostics move — which reduced FFI friction for `nono-py`/
  `nono-ts` by exposing structured facts at the library layer — moving `ResourceLimits` into the
  library would not change what any binding can *do*; the fork's platform dispatch and
  enforcement already work end-to-end from `nono-cli`. It would only restructure proven code
  to mirror upstream's shape, which CORE-02 (per Phase 111's own scope: "No new enforcement, no
  regression") explicitly forbids risking.

In short: ADR-86 asks "does this code decide or enforce anything?" Audit/diagnostics answered
no. Resource limits answer yes — so the carve-out's conclusion does not transfer.

---

## Consequences

1. **Standing divergence recorded.** `108-DIVERGENCE-LEDGER.md` receives a "Phase 111 Standing
   Divergence Addendum" (this plan's Task 2) recording `e6d26871`/`34c2c975` as a **permanent**
   divergence — not a deferral to a named future phase, unlike the tool-sandbox subsystem's
   `DEFERRED->v3.7` treatment.

2. **Future upstream syncs must re-affirm, not re-litigate.** Every future sync that touches
   upstream's `resource` module in `nolabs-ai/nono`'s core `nono` crate must re-affirm this
   decision — citing this ADR — rather than treat the fork's non-adoption of `pub mod resource;`
   as an unabsorbed gap to close.

3. **User-visible change is the corrected help text only.** The only user-facing consequence of
   Phase 111's CORE-02 work is Plan 111-02's correction of `crates/nono-cli/src/cli.rs`'s stale
   help text, which previously and incorrectly told users that `--memory`/`--timeout`/
   `--max-processes` are "accepted with a warning pending cross-platform follow-up" on
   Linux/macOS. No enforcement behavior changes; no flag is renamed, re-ranged, or re-typed.

4. **The library's public API surface is unchanged by this decision.** `crates/nono/src/lib.rs`
   gains no new `pub mod` or `pub use` from this ADR. The ADR-86 policy-free-library boundary is
   preserved exactly as it stood before Phase 111.

---

## References

- Upstream commit: `e6d26871f0498e7dc7a867e67af5c6136b84f91c` — `feat: resource limiting (#1269)`
- Upstream commit: `34c2c975d649844923cf1515be94de624c689c6c` — `feat(resources): cap sandbox
  process count with --max-processes (#1403)`
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library boundary this ADR
  confirms is preserved, and the audit/diagnostics carve-out this ADR explains does not extend
  to resource limits
- `proj/ADR-108-deny-domain-posture.md` — the structural precedent this document mirrors in
  shape and naming
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — CORE Cluster
  per-commit table (the adapt disposition and windows-relevance finding this ADR builds on) and
  the Threat Flags table (`threat_flag: cross-crate-reexport`) this ADR resolves
- `.planning/phases/111-core-carry-resource-cli-verify-release-leapfrog/111-CONTEXT.md` —
  D-01 through D-04, the locked decisions this ADR records durably
