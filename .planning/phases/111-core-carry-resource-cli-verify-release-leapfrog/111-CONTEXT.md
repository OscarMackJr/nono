# Phase 111: Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog - Context

**Gathered:** 2026-08-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 111 closes the v3.6 UPST12 sync. It delivers four things:

1. **CORE-01** — the macOS/core carry lands as-is for cross-target parity: the `~/.cache` grant
   (#1378) and `MAX_CRYPTO_THREADS`=12 libdispatch tuning (#1424).
2. **CORE-02** — the upstream resource-limit CLI surface (#1269/#1403) is reconciled with the
   fork's existing kernel-enforced implementation. **No new enforcement**, no regression to
   `--cpu-percent`/`--timeout`.
3. **VERIFY-01** — both cross-target clippy gates and `make ci` are GREEN locally, and a
   fork-invariant pass confirms the Windows security model + ADR-86 boundary are unregressed.
4. **RLS-14** — all 6 workspace crates + path-dep pins + both binding repos leapfrog to `0.70.0`
   (collision-free above upstream 0.69.0), with the prepare-only release gate GREEN.

**Explicitly NOT in this phase:**
- The `macos.rs` port-range emitter (#1398). Removed from CORE-01 by the 2026-07-30 amendment —
  Phase 110 absorbed `d5803b99` whole, including both Unix emitters. **Do not re-absorb.**
- Any new resource enforcement on any platform (D-02 below).
- Any actual registry publish or tag push (D-07 below).
- The security/residual cluster (Phase 112) and SPIFFE (Phase 113).

</domain>

<decisions>
## Implementation Decisions

### CORE-02 — where `ResourceLimits` lives

- **D-01: ADAPT, do not adopt.** Upstream `e6d26871` (#1269) adds `pub mod resource;` +
  `pub use resource::ResourceLimits;` to the policy-free core `crates/nono/src/lib.rs`. The fork
  **rejects the core-module absorb** and keeps all resource code CLI-side. Align only flag
  names, help text, and semantics.

  *Rationale:* the fork's implementation is materially **more complete than upstream's** — all
  four limits are kernel-enforced on three platforms with genuinely different mechanics
  (Job Object on Windows, cgroup v2 on Linux, `RLIMIT_AS`/`RLIMIT_NPROC` on macOS). Upstream's
  core module carries no enforcement; adopting it relocates a data type and forces restructuring
  of working, tested, security-critical code for cosmetic convergence. ADR-86's audit/diagnostics
  carve-out was justified because those are pure observability primitives with no enforcement
  semantics; resource limits are enforcement, sitting closer to the policy side of the boundary.
  CORE-02 explicitly forbids new enforcement and regressions — a restructure buys nothing
  functional and risks exactly that.

- **D-02: Record as a standing divergence in the ledger.** The real cost of D-01 is that every
  future UPST sync re-encounters this commit. Give it the same treatment the tool-sandbox
  subsystem got when it was routed to v3.7: an explicit standing-divergence entry so the next
  audit finds a decision rather than a surprise.

- **D-03: Write `proj/ADR-111-resource-limits-boundary.md`.** The Phase 108 ledger already
  flagged `e6d26871` as an ADR-86 boundary crossing. Without a durable artifact the next audit
  re-litigates it. One page, matching `proj/ADR-108-deny-domain-posture.md`'s shape: disposition,
  the ADR-86 reasoning, and the standing-divergence note from D-02.

- **D-04: The fork's flag surface is FROZEN — back-compat wins.** `--memory` (parses
  `512M`/`1G`/`256K`/raw bytes), `--max-processes` (u32, 1..=65535), `--cpu-percent`
  (macOS-rejected at clap parse time), `--timeout` (`30s`/`5m`/`1h`/`1d`/raw seconds) are shipped
  public CLI surface. If upstream #1403 spells any of them differently, add an **alias** — never
  rename, never change units or ranges. Precedent: Phase 110-01's handling of
  `windows_low_il_broker`/`windows_interpreters`, where an override may tighten but never
  silently change meaning.

### CORE-02 — stale Unix help text

- **D-05: Correct the help text; it is IN scope.** `crates/nono-cli/src/cli.rs` (~L2781-2815)
  currently tells users that `--memory`, `--timeout`, and `--max-processes` are "accepted with a
  warning pending cross-platform follow-up" on Linux/macOS. **This is false.**
  `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` enforces `RLIMIT_AS` +
  `RLIMIT_NPROC`, and `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` carries the
  cgroup v2 enforcement module. A user reading the current help could reasonably conclude their
  limits are not applied and reach for another mechanism.

  Correcting a doc comment is **not** new enforcement, so this clears CORE-02's fence. It is also
  the same defect class Phase 110's checkpoint hit twice (a predicate or doc that stopped
  tracking the feature it describes — see `110-06-PROF-03e-VERDICT.md` defects 1 and 2).

- **D-06: Verify per-platform truth before rewriting — do not guess.** The rewrite must be
  driven by reading the three enforcement backends, not by assumption. Known trap already
  documented in the source: macOS `--memory` maps to `RLIMIT_AS` (**address space, not RSS**) —
  `supervisor_macos.rs` says so in its own module docs, and a process can exceed the limit in
  physical memory if its mappings are sparse or shared. The corrected help must not overstate
  what each platform actually guarantees.

### RLS-14 — release posture

- **D-07: PREPARE-ONLY. No tag push, no registry publish, in this phase.** Consistent with v3.3
  and v3.4. Specific additional reason: v3.5's `v0.66.1` tag push is **still unresolved** and
  blocked on the Azure Trusted Signing 403 / lapsed identity validation. Cutting or pushing
  `0.70.0` while that is outstanding entangles two release decisions that must stay separate.

- **D-08: Bump both sibling binding repos in-phase.** `../nono-py` and `../nono-ts` are in scope
  for the `0.70.0` bump, each with its own DCO-signed commit in its own repo. v3.4's durable
  lesson: binding-repo drift is caught **only** by `maturin build` / `napi build`, never by
  `cargo build --workspace` — the sibling repos are outside the Cargo workspace and path-dep in
  by relative path. Phase 110-08 confirmed the rebuild is fast when nothing structural changed.

### VERIFY-01 — coverage boundary

- **D-09: The fork-invariant pass covers the combined 108-111 surface, including Phase 110's.**
  Phase 110's `110-08` phase-gate certification predates four library-behaviour changes landed
  2026-08-04 (`7c7a189c`, `ea26b5b2`, `6d7ef719`, `4aec1944`), so that artifact describes a tree
  that no longer exists. More pointedly, `has_port_rules()` shipped **broken** straight through
  that certification — the gate demonstrably failed to catch a real defect inside its own scope.
  Re-running both cross-target gates plus both binding builds over the combined surface is the
  honest scope and costs ~10 minutes of gate time.

- **D-10: Both cross-target clippy gates are MANDATORY and must run locally.** `cross clippy
  --workspace --target x86_64-unknown-linux-gnu` and the direct-binary `cargo-zigbuild clippy
  --workspace --target x86_64-apple-darwin` (with `SDKROOT` unset). PARTIAL→CI is the fallback
  only on a *documented* runner failure; a stopped Docker daemon or an absent-but-installable
  tool does NOT qualify. Both were confirmed live-runnable and GREEN four times on 2026-08-04.

### Claude's Discretion

- Plan/wave decomposition and task ordering.
- Whether CORE-01's two carries (`~/.cache` #1378, `MAX_CRYPTO_THREADS` #1424) land in one plan
  or two — they are small and independent.
- The exact ADR-111 section structure, provided it settles the disposition and records the
  standing divergence.
- Which specific fork-invariant assertions to encode for D-09 beyond the two clippy gates and the
  two binding builds.
- Whether `cargo test --workspace` non-greenness is re-litigated: this host has a **documented
  pre-existing baseline of 11 `--bin nono` failures** (re-confirmed at a stashed baseline on
  2026-08-04), plus 13 more surfaced by `--no-fail-fast` in Plan 110-08. Do not chase these as
  regressions; do not fabricate GREEN.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Library boundary (drives D-01/D-03)
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library boundary this phase must
  not regress, and the precedent carve-out for audit/diagnostics that D-01 argues does NOT extend
  to resource limits.
- `CLAUDE.md` § "Library vs CLI Boundary" — the in-library/in-CLI table.

### Upstream absorb work-list
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — the 100-commit
  accounting; contains the `e6d26871` (#1269) boundary-crossing flag and the Requirement Coverage
  Gap section. D-02's standing-divergence entry lands here.
- `.planning/ROADMAP.md` § Phase 111 — goal, the 4 success criteria, and the CORE-01 amendment
  that removes #1398 from scope.
- `.planning/REQUIREMENTS.md` § CORE-01, CORE-02, VERIFY-01, RLS-14.

### Resource-limit implementation (drives D-04/D-05/D-06)
- `crates/nono-cli/src/cli.rs` ~L2740-2815 — the four shipped flags, their parsers
  (`parse_byte_size`, `parse_cpu_percent`, `parse_duration`), ranges, and the **stale** Unix help
  text D-05 corrects.
- `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` — `RLIMIT_AS`/`RLIMIT_NPROC` enforcement
  and the module-level `RLIMIT_AS vs RSS` caveat that D-06 must respect.
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` § `mod cgroup` — cgroup v2 enforcement.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — Job Object limits (`JobMemoryLimit`,
  `ActiveProcessLimit`, `TerminateJobObject`).

### Verification protocol (drives D-09/D-10)
- `.planning/templates/cross-target-verify-checklist.md` — **single source of truth** for both
  cross-target gates, the pinned `cross` image tag, and the per-gate decision tree. Do not
  duplicate the runbook elsewhere.
- `.planning/phases/110-profile-policy-absorb-platform-overrides/110-08-SUMMARY.md` — the prior
  phase-gate certification D-09 declares stale, plus the binding-rebuild procedure.
- `.planning/phases/110-profile-policy-absorb-platform-overrides/110-06-PROF-03e-VERDICT.md` — the
  4 defects found on 2026-08-04 and their commits; the surface D-09 re-certifies.
- `.planning/phases/110-profile-policy-absorb-platform-overrides/deferred-items.md` — the
  documented pre-existing test-failure baseline.

### Release engineering (drives D-07/D-08)
- `.planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md`
  (archived — verified path) and `scripts/release-dry-run.ps1` — the prepare-only release gate and
  the 3-crate publish set (`nono-shell-broker` is `publish = false`).
- `scripts/gates/` + `scripts/verify-dark.ps1` — the unattended gate harness.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **All four resource flags already exist and are enforced.** `--cpu-percent`, `--memory`,
  `--timeout`, `--max-processes` are shipped with per-platform kernel enforcement. CORE-02 is a
  *reconciliation*, not an implementation — this is the single most important framing correction
  for the researcher and planner.
- **`crates/nono/src/resource.rs` does not exist** and `lib.rs` has no `pub mod resource;`. Under
  D-01 it stays that way; a plan that creates it has misread the decision.
- **Both cross-target gates are installed and proven** on this host (Docker + `cross`; zig +
  `cargo-zigbuild`). No setup work needed.
- **Version-bump surface is known from v3.3/v3.4**: 6 workspace `Cargo.toml` files + internal
  path-dep version pins + `../nono-py` (Cargo.toml + pyproject.toml) + `../nono-ts` (Cargo.toml +
  6 JSON manifests).

### Established Patterns
- **Fork-owned package names (Phase 102):** the crates publish as `nono-sandbox`,
  `nono-sandbox-proxy`, `nono-sandbox-cli` via Cargo's `package =` key; bin/lib/repo names are
  unchanged. Cargo selectors are `-p nono-sandbox-cli`, **not** `-p nono-cli`.
- **`make` is NOT on this host's PATH.** Substitute the constituent `cargo` invocations; this is
  a documented, recurring substitution (Phases 102-02, 102-05).
- **Exhaustive-struct-literal safety net:** adding a field to a shared struct forces every
  construction site to update (E0063). Phase 110 hit this four separate times. Expect it if
  CORE-02 touches any shared type.
- **Contentious absorbs get a standalone ADR** (ADR-108 deny_domain, ADR-113 SPIFFE proposed).
  D-03 follows this.

### Integration Points
- CORE-01's `~/.cache` grant touches the macOS Seatbelt profile path set; `MAX_CRYPTO_THREADS`
  touches libdispatch tuning. Both are cfg-gated Unix surfaces → **cross-target gates mandatory**
  per `CLAUDE.md`.
- RLS-14's version bump touches `Cargo.lock`; expect drift limited to the bumped packages, as in
  Phase 102-01.

</code_context>

<specifics>
## Specific Ideas

- The corrected help text (D-05) should state what each platform actually guarantees, including
  the macOS address-space-not-RSS caveat, rather than a single blanket sentence per flag.
- ADR-111 should explicitly answer why the audit/diagnostics carve-out in ADR-86 does **not**
  extend to resource limits, so the argument is not re-derived from scratch next sync.

</specifics>

<deferred>
## Deferred Ideas

- **Adopting upstream's core `resource` module.** Rejected for this milestone by D-01, recorded as
  a standing divergence by D-02. If a future milestone wants library-level resource types for
  binding consumers, that is its own ADR-gated phase — not a drive-by during a sync.
- **Unix enforcement parity work beyond what exists.** Out of bounds: CORE-02 forbids new
  enforcement. Any gap discovered while writing D-05's corrected help text should be recorded in
  `deferred-items.md`, not fixed here.

### Reviewed Todos (not folded)
- **`20260611-msi-vcredist-prereq`** — MSI VC++ redistributable prerequisite. Deferred: this is
  clean-host install territory (v3.5 Phase 106 UAT). Phase 111 never touches the MSI or a clean
  host. Match was keyword-only (score 0.6 on "2026"/"phase").
- **`20260611-poc-cert-broker-clean-host`** — POC cert / broker spawn on a clean host. Deferred:
  same reason; belongs with v3.5's clean-host UAT, which is blocked on the Azure 403.

</deferred>

---

*Phase: 111-core-carry-resource-cli-verify-release-leapfrog*
*Context gathered: 2026-08-04*
