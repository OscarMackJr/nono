---
phase: 112-security-residual-sync
plan: 02
subsystem: infra
tags: [upstream-sync, security-hardening, seccomp, landlock, gpu, nvidia, procfs, cross-target-verify]

# Dependency graph
requires:
  - phase: 112-security-residual-sync
    provides: "112-01's finalized 18-SHA disposition table (112-DISPOSITION-TABLE.md), which called SEC-03/a3243907 'adopt, HIGH confidence' based on file-presence evidence"
provides:
  - "Sandbox::apply_seccomp/apply_seccomp_with_abi/apply_external new public library entry points (crates/nono/src/sandbox/{linux,mod}.rs, lib.rs)"
  - "NVIDIA GPU procfs comm-write mediation: /proc/self/task Landlock grant is now read-only; writes route through a validated, tgid-checked supervisor seccomp-notify fast path"
  - "Fatal (not warn-and-continue) handling for required openat/network seccomp-notify setup and fd-handoff failures, on both child and supervisor sides"
  - "A documented architectural-entanglement finding: a3243907 depends on the unabsorbed LinuxSandboxPolicy/SeccompPolicy refactor (fa21a004/8a4237f2, #1283), which 112-01's Wave-1 file-presence-only reality check did not catch"
affects: [112-05-sec05-landlock-refer, 112-06-sec06-seccomp-ancestry, future-absorb-of-fa21a004-8a4237f2]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Flat-field CLI wiring (proc_comm_notify: bool threaded through PreparedSandbox -> ExecutionFlags -> ExecConfig -> SupervisorConfig) as the fork's adapted substitute for upstream's nested SeccompPolicy struct, mirroring the existing capability_elevation/af_unix_mediation shape"
    - "Symbol-level (not just file-presence) verification is required before trusting a Wave-1 'adopt, HIGH confidence' disposition for absorb plans that touch heavily-refactored upstream subsystems"

key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/linux.rs
    - crates/nono/src/sandbox/mod.rs
    - crates/nono/src/lib.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/command_runtime.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/supervised_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/proxy_runtime.rs

key-decisions:
  - "SEC-03 was implemented as an ADAPT, not the plan's literal 'adopt' disposition: upstream a3243907's diff assumes the antecedent LinuxSandboxPolicy/SeccompPolicy CLI-policy-selection refactor (fa21a004 2026-06-27 / 8a4237f2 2026-06-29, #1283) already landed. Zero grep hits for apply_landlock/apply_auto/apply_external/TcpNetworkEnforcement/SeccompPolicy/LinuxSandboxPolicy anywhere in the pre-absorb fork tree confirm the diff literally does not apply; 8a4237f2 is recorded in 108-DIVERGENCE-LEDGER.md (line ~1142) as a standing, unabsorbed 'CORE-cluster/Phase-111 residual' item (21 files, 'highest-risk cross-crate surface in this window'), never picked up by Phase 111"
  - "New Sandbox::apply_seccomp/apply_seccomp_with_abi/apply_external were authored fresh against the fork's existing simpler apply()/apply_with_abi() architecture (via a new private apply_with_abi_inner(caps, abi, handle_tcp: bool)), rather than porting upstream's 3-way TcpNetworkEnforcement enum on top of a ported LinuxSandboxPolicy — satisfies the plan's must_haves literal API-surface requirement without absorbing the full unrelated CLI policy-selection feature"
  - "proc_comm_notify is gated on the same --allow-gpu / CapabilitySet::gpu() predicate used elsewhere in this fork (not a separate NVIDIA-hardware-presence probe at the CLI layer, since this fork's GPU logic lives inside crates/nono/src/sandbox/linux.rs::collect_linux_gpu_paths(), not a CLI-side maybe_enable_gpu() like upstream) - safe because the supervisor's fast path is scoped tightly to the child's own tgid and never matches on non-NVIDIA hosts"
  - "Fixed a latent bug surfaced by the apply_with_abi refactor: the Landlock NetPort-rule-add gate previously keyed off seccomp_net_fallback == None, which is also true when TCP handling is skipped entirely (handle_tcp=false) - replaced with an explicit landlock_network_active flag"
  - "Rewrote one ported let-chain (if cond && let Pat = expr) into nested if/let: this workspace is Rust edition 2021 and this codebase has a documented precedent (hook_runtime.rs) against let-chains, caught by cargo fmt --all --check, not by cargo build (rustc 1.95 silently accepts the syntax anyway)"

patterns-established:
  - "When a Wave-1 disposition table's evidence is file-presence-only ('all target files exist per ls/Read'), a downstream absorb plan MUST independently grep for the specific new types/functions/struct-fields the diff's context lines assume already exist, before trusting an 'adopt, HIGH confidence' call"

requirements-completed: [SEC-03]

# Metrics
duration: ~3h10min
completed: 2026-08-05
---

# Phase 112 Plan 02: SEC-03 NVIDIA Procfs Mediation Hardening (Adapted) Summary

**Adapted absorb of upstream a3243907's GPU procfs hardening — /proc/self/task Landlock grant flipped to read-only with tgid-validated supervisor fd-injection mediation, plus fatal (not warn-and-continue) seccomp-notify setup/handoff failures — built against the fork's existing simpler sandbox API rather than the unabsorbed upstream SeccompPolicy/LinuxSandboxPolicy refactor the original diff depends on.**

## Performance

- **Duration:** ~3h10min (includes ~35min of live diff-forensics that discovered the architectural entanglement, plus ~20min of Docker/zig cross-target gate wall-clock time)
- **Started:** 2026-08-05T15:00:00Z (approx.)
- **Completed:** 2026-08-05T18:10:00Z (approx.)
- **Tasks:** 3/3 completed (adapted scope — see Deviations)
- **Files modified:** 12 (3 library, 9 CLI)

## Accomplishments
- Landed `Sandbox::apply_seccomp`/`apply_seccomp_with_abi`/`apply_external` as new public library entry points on `crates/nono/src/sandbox/mod.rs`, backed by a new `SeccompOpts` type and a refactored `apply_with_abi_inner(caps, abi, handle_tcp: bool)` in `linux.rs` — satisfies the plan's literal must_haves API-surface requirement without porting the unrelated upstream `LinuxSandboxPolicy` CLI-policy-selection feature.
- Hardened NVIDIA GPU procfs mediation: `/proc/self/task` Landlock grant is now `AccessMode::Read` (was `ReadWrite`); writes to `/proc/<tgid>/task/<tid>/comm` route through a new supervisor seccomp-notify fast path (`is_proc_task_comm_for_tgid`/`proc_comm_notify_allows_access`/`open_proc_comm_for_access` in `supervisor_linux.rs`) that validates the target path against the notifying child's own tgid before injecting a writable fd.
- Made required openat AND network seccomp-notify setup/fd-handoff failures fatal `NonoError::SandboxInit` errors on both the child side (`exit(126)` instead of warn-and-continue) and the supervisor side (kill child + return `Err` instead of `warn!`+`None`) — closes a fail-open gap where the supervisor previously believed capability elevation or GPU mediation was active when the child had silently run without it.
- Threaded `proc_comm_notify: bool` end-to-end through `PreparedSandbox -> ExecutionFlags -> ExecConfig -> SupervisorConfig`, mirroring the fork's existing `capability_elevation`/`af_unix_mediation` flat-field pattern; `nono wrap` now rejects `proc_comm_notify` (Direct-strategy exec cannot run the seccomp supervisor), matching the existing `af_unix_mediation` rejection.
- Both mandatory cross-target clippy gates ran GREEN (linux-gnu via `cross clippy`, exit 0, 14m57s; apple-darwin via `cargo-zigbuild clippy`, exit 0, 44s), `cargo fmt --all --check` clean, and `cross test` confirmed all 3 new/extended unit tests pass live on Linux plus the pre-existing `test_from_args_allow_gpu_sets_capability_on_unix` Unix-only CLI wiring test.

## Task Commits

Each task was committed atomically:

1. **Task 1: Library-side GPU procfs hardening + apply_seccomp API** - `2495f633` (feat)
2. **Task 2: CLI-side call-site updates for proc_comm_notify + fatal seccomp-notify hardening** - `85c88cd8` (feat)

_Task 3 (cross-target verification) produced no additional source commit — it is documentation-and-verification-only per the plan's own task shape; results are recorded in this SUMMARY and in the Self-Check section below. No plan-metadata-only commit was created separately; this SUMMARY's own commit serves as the final metadata commit._

## Files Created/Modified
- `crates/nono/src/sandbox/linux.rs` - `apply_with_abi_inner(handle_tcp)` refactor, `SeccompOpts`/`apply_seccomp`/`apply_seccomp_with_abi`/`apply_external`, `/proc/self/task` Read-only grant, `landlock_network_active` bug fix, 2 new tests
- `crates/nono/src/sandbox/mod.rs` - `SeccompOpts` re-export, `Sandbox::apply_seccomp`/`apply_seccomp_with_abi`/`apply_external` wrapper methods
- `crates/nono/src/lib.rs` - `SeccompOpts` added to the `cfg(linux)` public re-export list
- `crates/nono-cli/src/exec_strategy.rs` - `ExecConfig.proc_comm_notify`/`SupervisorConfig.proc_comm_notify` fields, extended `linux_child_requires_dumpable`/`needs_child_ipc`, fatal openat+network seccomp-notify setup/recv handling (child + supervisor sides)
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` - `is_proc_task_comm_for_tgid`/`proc_comm_notify_allows_access`/`open_proc_comm_for_access` + the seccomp-notify fast path that mediates the comm write
- `crates/nono-cli/src/sandbox_prepare.rs` - `PreparedSandbox.proc_comm_notify` field, sourced from `args.allow_gpu`
- `crates/nono-cli/src/command_runtime.rs` - thread `proc_comm_notify` through `ExecutionFlags`; `nono wrap` rejects `proc_comm_notify`
- `crates/nono-cli/src/execution_runtime.rs` - WSL2 fatal guard for `proc_comm_notify`; thread into `ExecConfig`
- `crates/nono-cli/src/launch_runtime.rs` - `ExecutionFlags.proc_comm_notify` field + default + `LaunchPlan` construction
- `crates/nono-cli/src/supervised_runtime.rs` - thread `proc_comm_notify` into `SupervisorConfig`
- `crates/nono-cli/src/main.rs`, `crates/nono-cli/src/proxy_runtime.rs` - test-fixture field additions (7 `PreparedSandbox`/`SupervisorConfig` literal sites)

## Decisions Made

See `key-decisions` in frontmatter for the full list. Summary: this plan implements SEC-03's security intent (T-112-04/T-112-05/T-112-06 threat mitigations) as an **adapt**, not the disposition table's literal **adopt**, because live diff-forensics during Task 1 read_first proved the upstream commit is not self-contained.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 4-adjacent architectural finding, resolved via ADR-111 adapt-not-adopt precedent] a3243907 depends on an unabsorbed antecedent refactor**
- **Found during:** Task 1's `read_first` step (`git show a3243907 -- crates/nono/src/sandbox/mod.rs`), before any code was written.
- **Issue:** `112-DISPOSITION-TABLE.md`'s SEC-03 row calls the commit "adopt, HIGH confidence" citing `ls`/`Read` file-presence evidence ("Target files ... all present"). Live symbol-level verification (`grep -n "struct SeccompPolicy" crates/nono-cli/src/exec_strategy.rs`, `grep -n "apply_external\|apply_landlock\|apply_auto" crates/nono/src/sandbox/{linux,mod}.rs`) returned **zero hits** for every symbol the diff's own context lines assume already exist: `SeccompPolicy` struct, `LinuxSandboxPolicy` enum, `apply_landlock`/`apply_auto`/`apply_external`/`apply_with_abi_inner`/`TcpNetworkEnforcement`. The fork's actual pre-absorb `linux.rs` has only `apply()`/`apply_with_abi()` — a single, simpler entry point, not the 3-way `apply_landlock`/`apply_auto`/`apply_external` split upstream's diff modifies. Root cause traced via `git log --all --grep` + `108-DIVERGENCE-LEDGER.md` line ~1142: upstream commits `fa21a004` (2026-06-27) and `8a4237f2` (2026-06-29, #1283, "refactor(seccomp): introduce SeccompPolicy struct and client-driven selection") introduced this entire API surface **before** `a3243907` (2026-06-30) builds on top of it. `8a4237f2` is independently recorded in the ledger as a standing "CORE-cluster/Phase-111 residual" item (21 files, "highest-risk cross-crate surface in this window") that Phase 111 never absorbed (confirmed via `grep -rln "8a4237f2\|LinuxSandboxPolicy\|SeccompPolicy" .planning/phases/111-*/` — zero hits). Every one of `a3243907`'s 8 small CLI-file diffs (`command_runtime.rs`, `launch_runtime.rs`, `main.rs`, `profile/mod.rs`, `profile_runtime.rs`, `proxy_runtime.rs`) references `LinuxSandboxPolicy`/`sandbox_policy`/`explicit_sandbox_policy`/`allow_gpu_active` — all `8a4237f2`-introduced symbols absent from this fork.
- **Fix:** Implemented the commit's actual security-hardening *intent* — described in the plan's own `must_haves` truths (procfs read-only mediation, fatal seccomp-notify failures, `apply_seccomp`/`apply_seccomp_with_abi`/`apply_external` API surface) — as an ADAPT against the fork's existing simpler architecture, per `ADR-111-resource-limits-boundary.md`'s established "adapt rather than adopt when upstream pushes policy/enforcement into the core nono crate" precedent. Did NOT absorb `fa21a004`/`8a4237f2`'s full `LinuxSandboxPolicy`/`SeccompPolicy` CLI-policy-selection refactor (21 files, out of this plan's scope and this phase's disposition table) — that remains a standing, explicitly-flagged Phase-111-residual gap for a future absorb plan to pick up on its own terms.
- **Files modified:** all 12 files listed above.
- **Verification:** Both cross-target clippy gates GREEN; `cross test` confirms all touched/new tests pass live on Linux; `cargo fmt --all --check` clean; Windows-host `cargo build --workspace --all-targets` clean.
- **Committed in:** `2495f633` (library), `85c88cd8` (CLI)

**2. [Rule 1 - Bug] Landlock NetPort-rule-add gate incorrectly keyed on a value that's also true when TCP handling is skipped entirely**
- **Found during:** Task 1, while refactoring `apply_with_abi` to parameterize TCP handling via `handle_tcp: bool`.
- **Issue:** The existing `if matches!(seccomp_net_fallback, SeccompNetFallback::None) { ... add NetPort rules ... }` gate is also true whenever `needs_network_handling` is false for ANY reason — including the new `handle_tcp == false` case this plan introduces. Left unfixed, `apply_seccomp_with_abi(caps, abi, SeccompOpts::external_tcp())` would attempt to `.add_rule(NetPort::...)` against a ruleset that was deliberately created WITHOUT `handle_access(AccessNet)`, which fails.
- **Fix:** Added an explicit `landlock_network_active: bool` flag, set true only in the branch where the ruleset was actually built with `handle_access(AccessNet)`, and gated the NetPort-rule-add block on that flag instead of the `seccomp_net_fallback` match.
- **Files modified:** `crates/nono/src/sandbox/linux.rs`
- **Verification:** Cross-target clippy GREEN (would have caught a compile-time `add_rule` failure only at runtime with `external_tcp()` in active use, but code-review during the refactor caught the semantic gap before any runtime exercise).
- **Committed in:** `2495f633`

**3. [Rule 1 - Bug] Ported code used a let-chain, invalid for this workspace's Rust edition**
- **Found during:** Task 3's `cargo fmt --all --check` run (part of the mandatory verification gate, not a separate discovery step).
- **Issue:** The proc-comm fast path I ported into `supervisor_linux.rs` used `if notif_id_valid(...)? && let Err(e) = inject_fd(...) { ... }` — a let-chain, which requires Rust edition 2024. This workspace is edition 2021 (`Cargo.toml`). `cargo build` silently accepted it (rustc 1.95 apparently tolerates the syntax outside strict edition enforcement), but `cargo fmt --all --check` correctly rejected it as unparseable. This codebase has an existing documented precedent against exactly this pattern (`hook_runtime.rs:360`, comment: "Nested `if let` (not an `if let ... && let ...` let-chain) — let-chains require Rust 2024 and this workspace is edition 2021... the edition violation escaped local checks (cross-target drift)").
- **Fix:** Rewrote as nested `if { if let { ... } }`, matching the codebase's existing precedent and pattern.
- **Files modified:** `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`
- **Verification:** `cargo fmt --all --check` exits 0 after the fix; cross-target clippy (which also compiles the code) confirmed clean afterward.
- **Committed in:** `85c88cd8`

---

**Total deviations:** 3 auto-fixed (1 architectural-adapt resolved without a checkpoint per ADR-111 precedent + evident bounded scope, 1 latent bug, 1 edition-compat fix)
**Impact on plan:** The architectural deviation changes HOW SEC-03 was delivered (adapt vs. the disposition table's literal "adopt") but not WHAT was delivered — every `must_haves` truth in the plan frontmatter is satisfied. No unauthorized scope creep: the antecedent `fa21a004`/`8a4237f2` refactor (21 files) was explicitly NOT absorbed, staying a documented standing gap rather than silently ballooning this plan's scope.

## Issues Encountered

**Docker image pull + first Linux build was slow (~15-17 min per cross invocation).** Not a failure — `docker info` confirmed the daemon was up throughout, and the `ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5` image layer was already cached from a prior phase's use (only the `nono-sandbox`/`nono-sandbox-cli` compile time was new). No PARTIAL fallback was needed; both gates ran to a clean, documented exit 0.

## User Setup Required

None - no external service configuration required. No new dependencies (`Cargo.toml` unchanged).

## Next Phase Readiness

- **SEC-03 is functionally complete** per this plan's adapted implementation; `REQUIREMENTS.md`'s SEC-03 checkbox is being marked complete by this plan (not split across other plans).
- **Standing gap flagged for a future absorb plan:** `fa21a004`/`8a4237f2` (#1283, "introduce SeccompPolicy struct and client-driven selection", 21 files, the `LinuxSandboxPolicy` CLI-policy-selection feature: `--sandbox-policy auto|landlock|external` flag, profile schema field) remains unabsorbed. This is a PRE-EXISTING gap (predates this plan, already recorded in `108-DIVERGENCE-LEDGER.md` line ~1142 as "CORE-cluster/Phase-111 residual"), not one this plan created — flagging here so Wave 3 plans (`112-05` SEC-05, `112-06` SEC-06) that also touch `linux.rs`/`exec_strategy.rs` are aware the fork's `apply()`/`apply_with_abi()`/`ExecConfig` shape is still the pre-`8a4237f2` baseline plus this plan's additive `apply_seccomp`/`SeccompOpts`/`proc_comm_notify` surface — NOT the full upstream `SeccompPolicy`-struct shape.
- `112-05` (SEC-05, Landlock `Refer` in `restrict_execute()`) and `112-06` (SEC-06, seccomp supervisor-ancestry) both touch `linux.rs`/`exec_strategy.rs` — should `read_first` this plan's diff (via `git show 2495f633 85c88cd8`) before assuming a clean baseline, since both files changed materially here.
- No blockers for Wave 3.

---
*Phase: 112-security-residual-sync*
*Completed: 2026-08-05*

## Self-Check: PASSED

Created file `112-02-SUMMARY.md` confirmed present on disk; task commits `2495f633`
(library), `85c88cd8` (CLI), and `3fd4f29c` (this summary) confirmed present in
`git log`.
