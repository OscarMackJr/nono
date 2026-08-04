# Phase 111: Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog - Research

**Researched:** 2026-08-04
**Domain:** Upstream cherry-pick reconciliation (macOS Seatbelt tuning), CLI help-text correctness against existing kernel-enforced resource limits, local cross-target clippy/CI verification, and multi-repo SemVer leapfrog (Cargo + PyO3 + napi-rs).
**Confidence:** HIGH — every claim below is grounded in a file path + line range or a git SHA read directly from this repo; no speculative library research was needed (no new external dependencies are introduced by this phase).

## Summary

Phase 111 is a reconciliation and verification phase, not a greenfield-feature phase. All four
requirements touch code or process that already exists and already works. **CORE-01** is a
2-commit macOS carry where the real payload turned out to be much smaller than the upstream diff
stat suggests: one is a single JSON line (`~/.cache` added to the fork's existing
`user_caches_macos` policy group), the other is a single `usize` constant bump
(`MAX_CRYPTO_THREADS` 7→12) plus its doc comment, in a constant this fork already has (it did not
need to be introduced). **CORE-02** is purely a documentation-accuracy fix: `crates/nono/src/lib.rs`
has no `pub mod resource;` (confirming D-01 holds today) and the fork's four resource flags
(`--cpu-percent`/`--memory`/`--timeout`/`--max-processes`) are already fully wired to real,
tested, per-platform kernel enforcement — the CLI help text is simply wrong about what happens on
Linux/macOS. The corrected text has a ready-made source of truth already living in this repo:
`launch_runtime.rs`'s `ResourceLimits` doc comment (lines 187-203) states the true per-platform
picture accurately today. **VERIFY-01** is procedural: both cross-target gates and their exact
invocations already exist in `.planning/templates/cross-target-verify-checklist.md`; `make` is
absent from PATH so `make ci`'s three constituent commands must be run directly; and a documented,
named, 24-failure `cargo test --workspace` baseline already exists from Phase 110-08 that this
phase's gate must reproduce (not exceed) rather than chase to green. **RLS-14** is a mechanical
6-crate + 2-sibling-repo version bump, for which the exact file inventory, current version
(`0.66.1` everywhere), and 2 hardcoded-version-string touchpoints (`release-dry-run.ps1`,
`release-readiness.ps1`) are all enumerated below.

**Primary recommendation:** Treat this phase as four independent, low-risk, well-evidenced tasks
— absorb 2 tiny macOS diffs verbatim, rewrite CLI + docs help text to match the already-accurate
`launch_runtime.rs` doc comment, run the two mandatory local cross-target gates plus a
combined-108-111 fork-invariant sweep, then do a mechanical version-string bump across 6
`Cargo.toml`s + 2 sibling repos + 2 gate scripts. The highest real risk is CORE-01's naive
cherry-pick: 5 of the 6 files upstream's `#1378` commit touches do not exist under those names in
this fork (or don't exist at all), so a literal `git cherry-pick`/patch-apply will fail or corrupt
unrelated files — the plan must apply the ONE relevant JSON line by hand, not attempt to port the
commit wholesale.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| macOS Seatbelt cache-path grant (`~/.cache`) | CLI (policy) | — | `crates/nono-cli/data/policy.json` is CLI-owned built-in policy data (per CLAUDE.md's Library vs CLI table: "Policy groups... system paths" are CLI-side); the library's Seatbelt profile generator (`crates/nono/src/sandbox/macos.rs`) only compiles whatever paths the CLI feeds it. |
| macOS fork-safety thread-count budget (`MAX_CRYPTO_THREADS`) | CLI (exec strategy) | — | `crates/nono-cli/src/exec_strategy.rs` is CLI-side supervised-fork machinery, not library code; the constant gates a pre-fork safety check, not a sandbox capability. |
| Resource-limit CLI surface + help text | CLI (policy/UX) | — | `cli.rs` argument definitions and their `///` help text are CLI-owned UX (CLAUDE.md: "All output and UX" is CLI-side). No library change is in scope (D-01). |
| Resource-limit enforcement (Job Object / cgroup v2 / `setrlimit`) | CLI (exec strategy) | OS kernel | All three backends live under `crates/nono-cli/src/exec_strategy{,_windows}/` — CLI-side per ADR-86's boundary table; the actual enforcement mechanism is the OS kernel (Job Object, cgroup v2, POSIX rlimits), which the CLI configures but does not implement. |
| Cross-target clippy / `make ci` / binding rebuild verification | Dev tooling (out-of-band) | — | Verification tooling (`cross`, `cargo-zigbuild`, `maturin`, `napi`) is host-side CI/dev infrastructure, not a runtime tier of the product itself. |
| Version bump (Cargo.toml × 6, sibling repos × 2) | Build/packaging | — | SemVer identity is a packaging concern (`[package] version`), touching no runtime code path. |

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01 (ADAPT, do not adopt):** Upstream `e6d26871` (#1269) adds `pub mod resource;` +
  `pub use resource::ResourceLimits;` to the policy-free core `crates/nono/src/lib.rs`. The fork
  **rejects the core-module absorb** and keeps all resource code CLI-side. Align only flag names,
  help text, and semantics. *(Verified live 2026-08-04: `crates/nono/src/lib.rs` has 18 `pub mod`
  lines, none named `resource`; `find crates/nono/src -iname "resource*"` returns nothing. D-01
  holds today.)*
- **D-02 (standing divergence):** Record the same treatment the tool-sandbox subsystem got when
  routed to v3.7 — an explicit standing-divergence entry in the ledger so the next audit finds a
  decision, not a surprise.
- **D-03:** Write `proj/ADR-111-resource-limits-boundary.md`, one page, matching
  `proj/ADR-108-deny-domain-posture.md`'s shape: disposition, the ADR-86 reasoning, and the
  standing-divergence note from D-02. *(Confirmed: `proj/ADR-111*.md` does not yet exist — this
  phase creates it.)*
- **D-04 (flag surface FROZEN):** `--memory`, `--max-processes`, `--cpu-percent`, `--timeout` are
  shipped public CLI surface. If upstream #1403 spells any of them differently, add an alias —
  never rename, never change units or ranges.
- **D-05 (correct the help text; IN scope):** `crates/nono-cli/src/cli.rs` (~L2781-2815)
  currently tells users `--memory`/`--timeout`/`--max-processes` are "accepted with a warning
  pending cross-platform follow-up" on Linux/macOS — **false**. Correcting a doc comment is not
  new enforcement.
- **D-06 (verify per-platform truth before rewriting):** Rewrite must be driven by reading the
  three enforcement backends, not assumption. Known trap: macOS `--memory` maps to `RLIMIT_AS`
  (address space, not RSS).
- **D-07 (PREPARE-ONLY):** No tag push, no registry publish, in this phase. v3.5's `v0.66.1` tag
  push is still unresolved (Azure 403) — cutting/pushing `0.70.0` while that is outstanding
  entangles two release decisions.
- **D-08 (bump both sibling binding repos in-phase):** `../nono-py` and `../nono-ts` are in scope,
  each with its own DCO-signed commit in its own repo. Binding-repo drift is caught only by
  `maturin build` / `napi build`.
- **D-09 (combined 108-111 surface):** Phase 110's `110-08` phase-gate certification predates
  four library-behaviour changes landed 2026-08-04 (`7c7a189c`, `ea26b5b2`, `6d7ef719`,
  `4aec1944`); re-run both cross-target gates plus both binding builds over the combined surface.
- **D-10 (both cross-target clippy gates MANDATORY, local):** `cross clippy --workspace --target
  x86_64-unknown-linux-gnu` and `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin`
  (SDKROOT unset). PARTIAL→CI only on a documented runner failure.

### Claude's Discretion

- Plan/wave decomposition and task ordering.
- Whether CORE-01's two carries (`~/.cache` #1378, `MAX_CRYPTO_THREADS` #1424) land in one plan or
  two — they are small and independent.
- The exact ADR-111 section structure, provided it settles the disposition and records the
  standing divergence.
- Which specific fork-invariant assertions to encode for D-09 beyond the two clippy gates and the
  two binding builds.
- Whether `cargo test --workspace` non-greenness is re-litigated: this host has a documented
  pre-existing baseline of 11 `--bin nono` failures plus 13 more surfaced by `--no-fail-fast` in
  Plan 110-08 (24 total). Do not chase these as regressions; do not fabricate GREEN.

### Deferred Ideas (OUT OF SCOPE)

- **Adopting upstream's core `resource` module.** Rejected for this milestone by D-01, recorded as
  a standing divergence by D-02. A future milestone wanting library-level resource types is its
  own ADR-gated phase.
- **Unix enforcement parity work beyond what exists.** CORE-02 forbids new enforcement. Any gap
  discovered while writing D-05's corrected help text goes to `deferred-items.md`, not fixed here.
- `#1398`'s `macos.rs` port-range emitter — Phase 110 absorbed `d5803b99` whole. **Do not
  re-absorb.**
- Any actual tag push / registry publish (D-07).
- MSI/VC++-redist and POC-cert-broker clean-host todos — belong to v3.5, blocked on Azure 403.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CORE-01 | macOS/core carry lands as-is: `~/.cache` (#1378) + `MAX_CRYPTO_THREADS`=12 (#1424) | § CORE-01 below identifies the exact 1-line JSON diff and the exact 1-constant + doc-comment diff, verified against the current tree, plus a hard warning about the other 5 files in upstream's `#1378` commit that do NOT map onto this fork. |
| CORE-02 | `--memory`/`--max-processes` CLI surface aligned onto existing Job Object impl, no new enforcement, no regression to `--cpu-percent`/`--timeout` | § CORE-02 gives the verbatim current (stale) help text, the verbatim corrected source of truth already in `launch_runtime.rs`, the exact per-platform enforcement truth table (incl. the macOS best-effort/silent-failure caveat, stronger than the RSS-vs-AS caveat CONTEXT.md names), and confirms no test asserts on help-text strings. |
| VERIFY-01 | Both cross-target clippy gates + `make ci` GREEN locally; fork-invariant pass confirms Windows security model + ADR-86 boundary unregressed | § VERIFY-01 gives the exact `make ci` constituent commands (since `make` is absent), the exact 24-failure pre-existing baseline (11 + 13, by name) to reproduce not exceed, and concrete grep-based fork-invariant assertions modeled on Phase 110-08's method. |
| RLS-14 | All 6 workspace crates + path-dep pins + both binding repos leapfrog to `0.70.0`, Cargo.lock clean, prepare-only release gate GREEN | § RLS-14 enumerates the exact 6-crate + N-file inventory (including the 2 non-obvious hardcoded-`0.66.1`-string gate scripts), confirms current version and that `tools/sign-fixture` is NOT part of the bump, and gives the exact prepare-only gate invocation and what GREEN looks like. |
</phase_requirements>

## Standard Stack

Not applicable in the conventional sense — this phase introduces **zero new external
dependencies**. All tooling used (`cross`, `cargo-zigbuild`, `maturin`, `napi`, `cargo-audit`) is
already installed and proven on this host per Phase 96/102/110. No `Standard Stack` /
`Package Legitimacy Audit` section is populated because there is nothing to audit.

**Package Legitimacy Audit: N/A — no new external packages are introduced by CORE-01, CORE-02,
VERIFY-01, or RLS-14.** RLS-14 only bumps the version field of 6 crates the fork already owns and
2 sibling repos it already owns; it adds no new dependency to `Cargo.lock`.

## CORE-01 — macOS/Core Carry

### #1378 — `~/.cache` on macOS

**Upstream commit:** `ca888108fe5983be866e8d1a6eccf96edc2a8dd5` — "fix: missing ~/.cache on macOS
(e.g., uv, Corepack)" (Xiaokui Shu, 2026-07-10). `git show --stat` on this commit (run against this
repo's fetched upstream history) reports 6 files changed, 12+/12-.

**The actual feature is exactly one line.** The real diff:

```diff
--- a/crates/nono-cli/data/policy.json
+++ b/crates/nono-cli/data/policy.json
@@ -325,7 +325,8 @@
       "allow": {
         "readwrite": [
           "~/Library/Caches",
-          "~/Library/Logs"
+          "~/Library/Logs",
+          "~/.cache"
         ],
```

This is a one-line addition to the `readwrite` array of the fork's existing `user_caches_macos`
policy group `[VERIFIED: crates/nono-cli/data/policy.json:324-334, read live]`. Some
macOS-ecosystem tools (`uv`, Corepack, and others in the Node/Python toolchain space) write to
`~/.cache` even on macOS despite the platform convention being `~/Library/Caches` — the fork
already grants the convention path but not this common non-conventional one.

**Critical finding — the other 5 files in the upstream commit do NOT apply to this fork:**

| Upstream path (from the commit) | Fork reality | Verdict |
|---|---|---|
| `crates/nono-cli/src/learn.rs` | **Exists**, and still has the exact pre-fix shape (`if let Some(comma_idx) = after_paren.find(',') { ... } else { return None; }` at `extract_path_from_syscall`, line 1385) | Applicable in principle, but it is a pure `clippy::question_mark`-style cosmetic simplification unrelated to the `~/.cache` fix — bundled into the same upstream PR by coincidence of authorship, not because it belongs to #1378's feature. |
| `crates/nono-cli/src/macos_trust.rs` | **Does not exist under this name.** Fork's cert-trust code lives in `cert_trust.rs`, `trust_intercept.rs`, `trust_keystore.rs`, etc. — a different module decomposition. `git ls-files \| grep -i macos_trust` returns nothing. | Not portable as a path-based cherry-pick. |
| `crates/nono-cli/src/proxy_command.rs` | **Does not exist under this name** — fork's equivalent is `proxy_runtime.rs`/`pty_proxy.rs`. `grep -rln "key_pem" crates/` finds nothing matching upstream's diff shape. | Not portable. |
| `crates/nono-proxy/src/server.rs` diff (`&*self.token` → `*self.token`) | Fork's `server.rs` exists but the specific format!-string line upstream touches was not found via `grep -rn 'format!("{}{}"' ... \| grep key_pem\|cert_pem` (zero matches across `nono-cli/src` and `nono-proxy/src`). | Fork has diverged past this line; nothing to port. |
| `crates/nono-proxy/src/tls_intercept/ca.rs` | **The `tls_intercept/` directory does not exist in this fork's `nono-proxy`** (`ls crates/nono-proxy/src/tls_intercept/` → not found). | Not portable — the module does not exist. |

**Conclusion:** all 5 non-`policy.json` files in `ca888108` are upstream-side clippy-lint cleanups
(`&*x` → `*x`, an `if/else` → `?`-operator simplification) on code that either doesn't exist under
that name in this fork or has diverged too far to apply. **The plan should absorb ONLY the
one-line `policy.json` change** — attempting a literal cherry-pick or patch-apply of the full
commit will fail on missing files or touch unrelated fork code for zero benefit. This is exactly
the kind of upstream-diff-stat-overstates-the-real-change trap CONTEXT.md's "lands as-is" framing
did not anticipate; flag it explicitly in the plan so the executor does not attempt a `git
cherry-pick ca888108` (which will conflict/fail) and instead hand-edits the one JSON array.

### #1424 — `MAX_CRYPTO_THREADS` raised to 12

**Upstream commit:** `099237da94d33752ca5617db3f4902a216f9f84c` — "fix(exec): raise
MAX_CRYPTO_THREADS to 12 for macOS libdispatch workqueue threads" (James Carnegie, 2026-07-16).
Single file, single constant, `git show --stat`: `crates/nono-cli/src/exec_strategy.rs | 9 ++++---`.

**The constant already exists in the fork** (this is a value change, not a new-constant
introduction) `[VERIFIED: crates/nono-cli/src/exec_strategy.rs:168, read live]`:

```rust
// CURRENT (fork, pre-absorb):
/// Maximum threads allowed when crypto library thread pool is active.
/// Main thread (1) + tokio proxy workers (2) + aws-lc-rs ECDSA pool (4).
/// When --network-profile is used with trust scanning, both the proxy runtime
/// and crypto verification threads may be active simultaneously.
const MAX_CRYPTO_THREADS: usize = 7;
```

**Upstream's exact target text** (from `git show 099237da`):

```rust
/// Maximum threads allowed when crypto library thread pool is active.
/// Main thread (1) + tokio proxy workers (2) + aws-lc-rs ECDSA pool (4), plus
/// headroom for the OS-managed libdispatch workqueue threads that
/// Security.framework spawns for `SecTrustSettings*` XPC during proxy CA setup
/// on macOS (2-5 observed, scales with load). Those workqueue threads are
/// unnamed, parked, and fork-safe, but a tighter budget intermittently tripped
/// on them (issue: fork thread-count flake).
const MAX_CRYPTO_THREADS: usize = 12;
```

This is a clean, isolated, no-conflict absorb — a single scalar + doc comment. The constant is
consumed at `exec_strategy.rs:915` (`ThreadingContext::CryptoExpected` arm) and `:939` (error
message), both of which read `MAX_CRYPTO_THREADS` by name — no other call site needs updating.

**Cross-target relevance:** `exec_strategy.rs` is not itself cfg-gated Unix-only, but it contains
`#[cfg(target_os = "macos")]` blocks elsewhere in the same file (the `RLIMIT_AS` pre_exec hook at
~line 1127 — see CORE-02 below). Per
`.planning/templates/cross-target-verify-checklist.md` § Scope, editing this file requires both
mandatory cross-target clippy gates to run, even though the specific lines changed for #1424 are
not themselves inside a `cfg` block.

## CORE-02 — Resource-Limit CLI Surface Reconciliation

### The stale help text (verbatim, to be corrected)

`crates/nono-cli/src/cli.rs` lines 2767-2815 `[VERIFIED: read live 2026-08-04]`:

```rust
    // ── Resource Limits ──────────────────────────────────────────────
    /// Cap the sandboxed agent tree's CPU to this percentage of one logical core.
    /// Kernel-enforced on Windows via `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE` with hard-cap.
    /// On Linux: enforced via cgroup v2 `cpu.max` (requires systemd delegation).
    /// On macOS: rejected at parse time — no per-process CPU-quota equivalent exists.
    /// See REQUIREMENTS.md § REQ-RESL-NIX-03 for the macOS constraint.
    #[arg(long, value_name = "PERCENT", value_parser = parse_cpu_percent, help_heading = "RESOURCE LIMITS")]
    pub cpu_percent: Option<u16>,

    /// Cap the sandboxed agent tree's total memory. Accepts `512M`, `1G`, `256K`,
    /// or raw bytes. Kernel-enforced job-wide on Windows via `JobMemoryLimit`.
    /// On Linux/macOS: accepted with a warning pending cross-platform follow-up.
    /// See REQUIREMENTS.md § RESL-02.
    #[arg(long, value_name = "SIZE", value_parser = parse_byte_size, help_heading = "RESOURCE LIMITS")]
    pub memory: Option<u64>,

    /// Kill the sandboxed agent tree after this wall-clock duration.
    /// Accepts `30s`, `5m`, `1h`, `1d`, or raw seconds. Enforced by supervisor-side
    /// timer + `TerminateJobObject` on Windows (see REQUIREMENTS.md § RESL-03).
    /// On Linux/macOS: accepted with a warning pending cross-platform follow-up.
    #[arg(long, value_name = "DURATION", value_parser = parse_duration, help_heading = "RESOURCE LIMITS")]
    pub timeout: Option<std::time::Duration>,

    /// Cap the number of active processes in the sandboxed agent tree.
    /// Kernel-enforced on Windows via `ActiveProcessLimit`. Range: 1..=65535.
    /// On Linux/macOS: accepted with a warning pending cross-platform follow-up.
    /// See REQUIREMENTS.md § RESL-04.
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..=65535), help_heading = "RESOURCE LIMITS")]
    pub max_processes: Option<u32>,
```

Only `--cpu-percent`'s doc comment is already accurate (Linux cgroup v2 + macOS rejection are both
correctly described). The other three ("accepted with a warning pending cross-platform
follow-up") are false for both Linux and macOS.

**The identical false claim also exists in `docs/cli/usage/flags.mdx` lines 1234, 1238, 1247,
1259** `[VERIFIED: read live]` — the "Resource Limits" section intro and all three flag entries
repeat "accepted with a warning pending cross-platform follow-up." This file is not named in
CONTEXT.md's canonical refs, but it is the same defect class in the same words — the planner
should decide whether to fold this docs-site fix into the same task (low risk, same fix shape) or
flag it as a closely-related but separately-scoped follow-up. Not fixing it would leave the
project's public docs site contradicting the corrected `--help` output.

### The already-correct source of truth (use this as the rewrite's basis)

`crates/nono-cli/src/launch_runtime.rs` lines 187-203 `[VERIFIED: read live]` — this doc comment
on the CLI-internal `ResourceLimits` struct is accurate today and should be the primary source the
corrected `cli.rs`/docs text is condensed from:

```rust
/// Optional resource caps applied to the sandboxed agent tree.
/// All fields are `None` by default; `Some(_)` opts into enforcement on that dimension.
///
/// - **Windows:** kernel-enforced via Job Object (CPU, memory, process-count via
///   `SetInformationJobObject`; timeout via supervisor-side `TerminateJobObject`).
///   See `exec_strategy_windows::launch::apply_resource_limits`.
/// - **Linux:** kernel-enforced via cgroup v2 delegated hierarchy (see
///   `exec_strategy::apply_resource_limits_unix` → `supervisor_linux::cgroup::CgroupSession`).
///   Requires cgroup v2 + systemd delegation; fails fast on cgroup v1 hosts with
///   `NonoError::UnsupportedKernelFeature { feature: "cgroup_v2", hint }`
///   (Phase 37 D-05; hint points the user at the `cgroup_no_v1=all` boot flag).
/// - **macOS:** kernel-enforced for memory + max_processes via
///   `setrlimit(RLIMIT_AS, RLIMIT_NPROC)` in a `pre_exec` hook.
///   `--cpu-percent` is rejected at clap parse time (no per-process CPU-quota
///   equivalent on macOS; see `cli.rs::parse_cpu_percent`).
///   `--timeout` enforced via supervisor-side `Instant` deadline +
///   `kill(pgrp, SIGKILL)` watchdog.
```

### Per-platform enforcement truth table (D-06's mandatory verification, done)

| Flag | Windows | Linux | macOS |
|---|---|---|---|
| `--cpu-percent` | **Kernel-enforced, fail-closed.** `JobObjectCpuRateControlInformation`, `ENABLE\|HARD_CAP`. Any `SetInformationJobObject` failure returns `NonoError::SandboxInit` (`exec_strategy_windows/launch.rs:471-474` doc: "Fail-closed"). | **Kernel-enforced, fail-closed.** cgroup v2 `cpu.max = "<quota> <period>"` write; fails the whole launch if delegation is absent (see cgroup precondition below). | **Rejected at clap parse time** (`parse_cpu_percent`, `cli.rs:99-117`) — no per-process CPU-quota primitive exists on macOS; this is intentional, not a gap. |
| `--memory` | **Kernel-enforced, fail-closed.** `JobMemoryLimit` via `JobObjectExtendedLimitInformation`; any Set/Query failure → `NonoError::SandboxInit`. | **Kernel-enforced, fail-closed.** cgroup v2 `memory.max` write; write failure → `NonoError::SandboxInit`. | **Best-effort, silently degrades — the real trap, stronger than "RSS not AS."** `setrlimit(RLIMIT_AS, ...)` is attempted in the `pre_exec` hook; on Apple Silicon, dyld pre-maps hundreds of MiB of VAS before `main()`, so `setrlimit` below current usage returns `EINVAL`. The code's own comment (`supervisor_macos.rs:272-276`, `exec_strategy.rs:1134-1153`) explicitly says: *"D2 (Phase 68-02): macOS arm64 RLIMIT_AS is not reliably enforced... Best-effort: ignore the error and continue to exec."* The exact stderr message written on this path (async-signal-safe `libc::write`, `exec_strategy.rs:1142-1143`): `"nono: setrlimit(RLIMIT_AS) not enforced on macOS (best-effort); continuing\n"`. **Even when `setrlimit` succeeds, `RLIMIT_AS` bounds virtual address space, not RSS** (module doc `supervisor_macos.rs:12-19`) — a process can exceed the intended physical-memory cap if its mappings are sparse/shared. |
| `--timeout` | Supervisor-side timer + `TerminateJobObject`. | Supervisor-side `Instant` deadline + `kill(-pgrp, SIGKILL)` (process-group kill; `supervisor_macos.rs:327-339` documents the same pattern is shared cross-Unix). | Same as Linux: supervisor `Instant` + `kill(pgrp, SIGKILL)`. Requires the child to `setpgid(0,0)` post-fork (best-effort; non-fatal if it fails, tolerated per D-06 note in `exec_strategy.rs:1100-1122`). |
| `--max-processes` | **Kernel-enforced, fail-closed.** `ActiveProcessLimit` via the same extended-limit struct as `--memory`. | **Kernel-enforced, fail-closed.** cgroup v2 `pids.max` write; failure → `NonoError::SandboxInit`. | **Kernel-enforced, but UID-wide not descendant-tree-scoped** (unlike Linux `pids.max`). Uses `RLIMIT_NPROC = baseline_uid_count + N` (`supervisor_macos.rs` D-01/D-03), where `baseline_uid_count` is read via `proc_listpids(PROC_UID_ONLY)` in the parent before `fork()`. This bounds *all* processes owned by the UID, not just the sandboxed tree's descendants — other processes owned by the same UID can consume the budget. This enforcement path IS fail-closed on `setrlimit` failure (returns `std::io::Error` from the `pre_exec` closure, aborting `exec`), unlike the memory path. |

**Linux cgroup v2 precondition (verify-before-rewrite detail):** `supervisor_linux.rs`'s
`CgroupSession::detect()` (lines 2217-2263) requires **pure cgroup v2 with systemd delegation** —
it reads `/proc/self/cgroup` and requires exactly one line starting with `0::`. Any v1/hybrid
system, or a v2 system without delegation, produces
`NonoError::UnsupportedKernelFeature { feature: "cgroup_v2", hint: CGROUP_V2_HINT }` **before any
child is spawned** — i.e., the whole `nono run` invocation fails closed rather than silently
skipping the limit. `CGROUP_V2_HINT` is a library-level constant (`crates/nono/src/error.rs:20`,
re-exported at `lib.rs:78`) pointing the user at the `cgroup_no_v1=all` boot flag. This is a
stronger, more precise fail-closed guarantee than the corrected help text needs to spell out in
full, but the plan's rewrite should not claim Linux enforcement is unconditional — it is
conditional on cgroup v2 delegation, and fails the whole launch (not silently) when absent.

**No test asserts on the current (stale) help-text strings** — `[VERIFIED]` via repo-wide search
for `"warning pending"`/`"accepted with a warning"`/`"cross-platform follow-up"`: matches only in
`cli.rs` itself (the doc comments), `docs/cli/usage/flags.mdx`, and planning docs. `nono-cli`'s
`Cargo.toml` has no `insta`/`trycmd`/snapshot-test dependency at all, and no test file in
`crates/nono-cli/tests/` references these strings. **Correcting the help text requires zero test
changes** — this answers research question 7 definitively.

## VERIFY-01 — Fork-Invariant Verify

### Cross-target gate commands (cite, don't duplicate)

Per `.planning/templates/cross-target-verify-checklist.md` (single source of truth):

```bash
# linux-gnu (MUST run locally):
cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
# Pinned image: ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5@sha256:9e5b39c0...

# apple-darwin (MUST run locally, direct-binary form, SDKROOT unset):
cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

Both were confirmed GREEN as recently as Phase 110-08 (2026-07-30/31) against a superset of the
tree Phase 111 will touch, and again per STATE.md "four times on 2026-08-04" during the PROF-03e
checkpoint's defect-fix cycle. PARTIAL→CI is fallback-only on a *documented* runner failure
(stopped Docker daemon or absent-but-installable tool do NOT qualify — start/install it).

### `make ci` substitution (`make` confirmed absent from PATH)

`[VERIFIED: command -v make → exit 1 on this host, and read live from Makefile]`. `make ci`
(`Makefile:143`) expands to `check test audit`, whose constituent commands are:

```bash
# check: clippy fmt-check
cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used
cargo fmt --all -- --check

# test: test-lib test-cli test-ffi
cargo test -p nono-sandbox
cargo test -p nono-sandbox-cli
cargo test -p nono-ffi

# audit
cargo audit
```

Note the package selectors are the **fork-owned** names (`nono-sandbox`, `nono-sandbox-cli`), per
Phase 102 — not `nono`/`nono-cli`.

`cargo audit` is confirmed **exit 0 today** `[VERIFIED: ran live, exit 0]` — 567 crate
dependencies scanned, 6 allowed warnings (unsound/informational advisories on transitive deps:
`event-listener`/`async-std`/etc.), zero denied vulnerabilities. RUSTSEC-2026-0204
(`crossbeam-epoch`) was already closed by the out-of-band quick task `260729-nh4` on 2026-07-29
(`cargo update -p crossbeam-epoch --precise 0.9.20`, Cargo.lock-only change, byte-identical to
upstream `373a67ae`/#1369) — VERIFY-01's `cargo audit` leg should already be clean going in; no
action needed for Phase 111 itself (Phase 112 SC3 references this same fix, already done).

### The exact pre-existing `cargo test --workspace` baseline to reproduce (not exceed)

`[VERIFIED: .planning/phases/110-profile-policy-absorb-platform-overrides/110-08-VERIFICATION-NOTES.md,
read live]` — 24 total failures across 4 test binaries, all confirmed pre-existing and unrelated
to any 110-0X (or earlier) change via `git log --oneline --all -- <file>`:

**(a) 11 documented baseline failures** — `cargo test -p nono-sandbox-cli --bin nono`:
```
audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty
config::tests::nono_home_dir_falls_through_when_unset
config::tests::nono_home_dir_rejects_non_absolute_override
config::tests::nono_home_dir_returns_override_when_set
config::tests::test_validated_home_falls_back_to_userprofile
config::tests::test_validated_home_ignores_non_absolute_home_when_userprofile_exists
config::tests::user_state_dir_uses_localappdata_on_windows
profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name
protected_paths::tests::blocks_child_directory_capability
protected_paths::tests::blocks_parent_directory_capability
protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root
```

**(b) 13 more failures**, visible only with `--no-fail-fast` (default fail-fast always stopped at
(a) in every prior phase gate), across 3 files:
- `crates/nono-cli/tests/audit_attestation.rs` (2): hardcoded `"/bin/pwd"` literal, no Windows
  fallback.
- `crates/nono-cli/tests/env_vars.rs` (10): live `windows_run_*` integration tests
  (`windows_run_allow_all_network_probe_connects`, `windows_run_blocks_live_block_net_without_enforcement`,
  `windows_run_executes_basic_command`, `windows_run_filters_dangerous_env_vars_and_keeps_safe_ones`,
  `windows_run_filters_host_toolchain_home_vars_without_runtime_dir`,
  `windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified`,
  `windows_run_live_default_profile_executes_command`, `windows_run_propagates_child_exit_code`,
  `windows_run_smoke_validates_stdout_stderr_and_exit_code`,
  `windows_run_supervised_blocks_runtime_capability_elevation_with_actionable_diagnostic`) —
  correlated with host-state mandatory-label ACE contamination on real paths, not a code
  regression.
- `crates/nono-cli/tests/resl_nix_async_signal_safety.rs` (1): stale text-signature-match test
  expecting `std::io::Result<()>` where the real signature now spells the crate's own `Result`
  alias.

**Recommended validation approach for VERIFY-01:** run `cargo test --workspace --no-fail-fast`,
assert the failing-test-name set is a **subset of or identical to** these 24 names (plus whatever
the CORE-01/CORE-02 changes' own new/modified tests add and pass), and treat any *new* failure
name not in this list as a real regression requiring investigation — not as "pre-existing, ignore."

### Concrete fork-invariant assertions for D-09 (beyond the two clippy gates + two binding builds)

Modeled directly on Phase 110-08's own method (grep/diff, not a bespoke script):

1. **D-01 still holds:** `grep -n "^pub mod resource" crates/nono/src/lib.rs` → zero matches;
   `find crates/nono/src -iname "resource*"` → empty.
2. **ADR-86 Windows carve-out unregressed:** `git diff <phase-111-base>..HEAD --
   crates/nono-cli/src/exec_strategy_windows/` should show no unexpected structural change beyond
   what CORE-02's help-text task explicitly plans (none, since CORE-02 is CLI-help-text +
   `cli.rs` only — `exec_strategy_windows/launch.rs` itself is not modified by CORE-02, only read
   for verification).
3. **SC3-style back-compat for the 4 resource flags:** the existing flag-parsing regression-guard
   tests already in `cli.rs` (`cpu_percent_range_enforced_by_clap`, `max_processes_range_enforced_by_clap`,
   `memory_zero_rejected_by_parser`, and the two "Regression guard: Phase 16's --cpu-percent /
   --memory / --timeout / --max-processes remain parseable" tests at lines ~4133/~4249) must still
   pass unmodified after the help-text rewrite (a `///` doc-comment change cannot break `#[arg(...)]`
   parsing, but this is the cheap, concrete check to run and cite as evidence).
4. **Version-family crate count sanity for RLS-14:** `cargo metadata --format-version 1 --no-deps`
   parsed and asserted that exactly the 6 named version-family crates (`nono-sandbox`,
   `nono-sandbox-cli`, `nono-sandbox-proxy`, `nono-shell-broker`, `nono-fltmgr-client`,
   `nono-ffi`) report `0.70.0`, and that `sign-fixture` does NOT (it keeps its own `0.1.0`
   versioning) — this is exactly `scripts/gates/release-readiness.ps1`'s existing Assertion (a)
   mechanism, just re-pointed at the new target version.

## RLS-14 — `0.70.0` Leapfrog

### Complete version-bump file inventory (all currently `0.66.1`, verified live)

**This repo — 6 workspace-member crates that share the version-family bump** (the 7th/8th
workspace members, `nono-fltmgr-client` and `tools/sign-fixture`, need separate treatment — see
below):

| File | `[package] name` | Current version | In RLS-14's 6-crate set? |
|---|---|---|---|
| `Cargo.toml` (workspace root) | — (no root `[package]`) | — | n/a |
| `crates/nono/Cargo.toml` | `nono-sandbox` (`[lib] name = "nono"`) | `0.66.1` | Yes |
| `crates/nono-cli/Cargo.toml` | `nono-sandbox-cli` (`[[bin]] name = "nono"`) | `0.66.1` | Yes |
| `crates/nono-proxy/Cargo.toml` | `nono-sandbox-proxy` | `0.66.1` | Yes |
| `crates/nono-shell-broker/Cargo.toml` | `nono-shell-broker` (`publish = false`) | `0.66.1` | Yes |
| `bindings/c/Cargo.toml` | `nono-ffi` (`publish = false`) | `0.66.1` | Yes |
| `crates/nono-fltmgr-client/Cargo.toml` | `nono-fltmgr-client` (`publish = false`) | `0.66.1` | **Yes — this is the 6th crate.** `[VERIFIED live]`: this is a Windows-only minifilter-client spike crate, part of the workspace `members` list, versioned `0.66.1` today alongside the other 5. |
| `tools/sign-fixture/Cargo.toml` | `sign-fixture` (`publish = false`) | `0.1.0` | **No.** Independent versioning scheme (a CI signing-fixture tool, unrelated to the product's release train). Confirmed a workspace member via root `Cargo.toml`'s `members` array, but its version has never tracked the other 6 and should not be touched by RLS-14. |

**Internal path-dependency version pins** (each of these declares `version = "0.66.1"` alongside
its `path =`/`package =`, and must be bumped in lockstep) `[VERIFIED live via grep]`:
- `crates/nono-cli/Cargo.toml:47` — `nono = { version = "0.66.1", path = "../nono", package = "nono-sandbox", ... }`
- `crates/nono-cli/Cargo.toml:48` — `nono-proxy = { version = "0.66.1", path = "../nono-proxy", package = "nono-sandbox-proxy", ... }`
- `crates/nono-cli/Cargo.toml:180` — `nono-shell-broker = { path = "../nono-shell-broker", version = "0.66.1" }`
- `crates/nono-proxy/Cargo.toml:21` — `nono = { version = "0.66.1", path = "../nono", package = "nono-sandbox", ... }`
- `crates/nono-shell-broker/Cargo.toml:20` — `nono = { version = "0.66.1", path = "../nono", package = "nono-sandbox" }`
- `bindings/c/Cargo.toml:18` — `nono = { version = "0.66.1", path = "../../crates/nono", package = "nono-sandbox", ... }`

**`../nono-py` (sibling repo, own git history)** `[VERIFIED live]`:
- `Cargo.toml` — `name = "nono-py"`, `version = "0.66.1"` (line 3); `[lib] name = "_nono_py"`.
  Also has its own `nono`/`nono-proxy` path-dep pins (not directly read here, but per Phase 102-03
  precedent these are patched alongside).
- `pyproject.toml` — `name = "nono-sandbox"`, `version = "0.66.1"` (lines 6-7).
- `Cargo.lock` is gitignored-but-tracked (per Phase 102-03's `git add -f` precedent) — regenerate
  and force-add.

**`../nono-ts` (sibling repo, own git history)** `[VERIFIED live]` — **6 manifests total** (1
Cargo.toml + 5 JSON files, matching the "6 JSON manifests" figure from Phase 102-04's memory when
Cargo.toml is counted loosely alongside them):
- `Cargo.toml` — `name = "nono-node"`, `version = "0.66.1"` (line 3).
- `package.json` — `"name": "@oscarmackjr/nono-ts"`, `"version": "0.66.1"` (lines 2-3), **plus** an
  `optionalDependencies` block (lines 77-83) pinning all 5 platform packages at `"0.66.1"` each —
  this block must be bumped too, not just the top-level `version` field.
- `npm/darwin-arm64/package.json`, `npm/darwin-x64/package.json`,
  `npm/linux-arm64-gnu/package.json`, `npm/linux-x64-gnu/package.json` — each `"version": "0.66.1"`.
- **`npm/win32-x64-msvc/package.json` does NOT exist yet** in this repo — it is referenced in
  `optionalDependencies` (line 82: `"@oscarmackjr/nono-ts-win32-x64-msvc": "0.66.1"`) but the
  directory is absent. This is v3.5 Phase 105-03's still-unexecuted "close the win32-x64-msvc
  platform-package gap" work (0/5 plans done). **RLS-14 should bump the `optionalDependencies`
  version string for this entry anyway** (it's a string pin, harmless even though the package
  doesn't exist locally), but should NOT attempt to create the missing directory — that is
  explicitly out of this phase's scope (a v3.5 concern, not v3.6).

### Two hardcoded-version-string gate scripts (non-obvious touchpoints)

`[VERIFIED live via grep]` — these are NOT Cargo manifests but DO hardcode `0.66.1` and will
silently pass a stale check against the wrong version unless updated:

- `scripts/release-dry-run.ps1` — lines 3, 20, 85, 91 reference `0.66.1` in doc comments and a
  human-readable status message (`"nono ^0.66.1 not yet on crates.io; re-run after publishing
  nono"`). These are cosmetic (the actual regex match at line 88,
  `'failed to select a version for the requirement'`, is version-string-agnostic) but should be
  updated for operator-facing accuracy.
- `scripts/gates/release-readiness.ps1` — line 76: `$targetVersion = '0.66.1'` and line 77:
  `$upstreamHighest = '0.66.0'` are **live assertion inputs**, not cosmetic. This script's
  Assertion (a) (all 6 version-family crates match `$targetVersion` via `cargo metadata`) and
  Assertion (c) (leapfrog: `$targetVersion` > `$upstreamHighest`) will assert the WRONG thing
  (`0.66.1`/`0.66.0`) if not updated to `0.70.0`/`0.69.0`. **This file must be edited as part of
  RLS-14's own task, not left for a future phase** — it is the prepare-only release gate's actual
  version assertion.
- Also note `scripts/gates/release-readiness.ps1:81-88`'s `$versionFamilyCrates` array is already
  correctly enumerated (`nono-sandbox`, `nono-sandbox-cli`, `nono-sandbox-proxy`,
  `nono-shell-broker`, `nono-fltmgr-client`, `nono-ffi`) — this independently corroborates the
  "6 crates" inventory above and requires no change itself, only the two version-string variables
  above it.

### Prepare-only release gate — exact invocation and what GREEN looks like

`scripts/release-dry-run.ps1` `[VERIFIED: read live in full]`. Invoke via:

```powershell
pwsh -File scripts\release-dry-run.ps1
```

**Never** `pwsh -Command "<bare path>"` (documented repo-wide gotcha: swallows non-zero exit
codes). What it checks, per its own 3 "legs":
1. `cargo publish --dry-run -p <crate>` for `nono-sandbox` → `nono-sandbox-proxy` →
   `nono-sandbox-cli` (dependency order). **Expected `PRE_PUBLISH_REGISTRY_BLOCKED`** for the
   downstream two (they depend on `nono = ^0.70.0`, which does not exist on crates.io pre-publish)
   — this is documented as a non-failure status, not a hard FAIL.
2. `maturin build` + `twine check` in `../nono-py` (SKIP if `maturin`/`twine` absent from PATH —
   both are present on this host per Phase 102-03/110-08 precedent).
3. `npm publish --dry-run` in `../nono-ts`, asserting the tarball manifest contains `index.js` +
   `index.d.ts`.

**GREEN = exit 0**, which the script defines as: zero `HardFailures` (PRE_PUBLISH_REGISTRY_BLOCKED
and toolchain-absent SKIPs do not count as failures). **What must NOT be run per D-07:** no
`cargo publish` without `--dry-run`, no `twine upload`, no `npm publish` without `--dry-run` — the
script as written already enforces this (no live-upload command exists in the file at all; this
was independently confirmed by reading the full script, not just its `.SYNOPSIS`).

### `maturin build` / `napi build` — exact commands + prerequisites

- **`../nono-py`:** `maturin build` (bare, no flags needed for a dev-profile smoke build — Phase
  110-08 used exactly this and it built clean in 1m16s). Prerequisite: CPython 3.12 findable
  (`maturin` auto-detects; Phase 110-08 found `C:\Users\OMack\AppData\Local\Programs\Python\Python312\python.exe`).
- **`../nono-ts`:** `napi build --platform --release`, invoked via `npx napi build --platform
  --release` since there is no global `napi` binary on PATH (Phase 110-08 precedent: `@napi-rs/cli`
  3.6.0 resolves via `npx` from the local devDependency). Release-profile build took ~1m56s in
  Phase 110-08.
- Both are proven-green, no-fix-required builds as of Phase 110-08 against a superset of this
  phase's Rust-side changes; expect a similarly clean rebuild for Phase 111's changes since none
  of CORE-01/CORE-02 touch any `pub` struct field either binding constructs (CORE-01 is
  policy.json + a private constant; CORE-02 is CLI-only help text).

## Common Pitfalls

### Pitfall 1: Treating CORE-01's `#1378` as a whole-commit cherry-pick candidate

**What goes wrong:** A plan or executor runs `git cherry-pick ca888108` or manually copies all 6
files' diffs, either failing outright (files don't exist under those upstream paths) or
introducing unrelated clippy-lint churn into fork files that have already diverged from upstream's
shape.
**Why it happens:** The divergence ledger's per-commit table (correctly) lists all 6 files
upstream touched, without flagging that 5 of them are incidental cleanup on code this fork has
restructured or removed.
**How to avoid:** Apply only the one-line `policy.json` addition by hand. Treat the other 5 as
N/A, explicitly, in the plan/summary.
**Warning signs:** A patch-apply or cherry-pick step reports "file not found" or "no such path"
for `macos_trust.rs`, `proxy_command.rs`, or `tls_intercept/ca.rs`.

### Pitfall 2: Overstating macOS `--memory` enforcement in the corrected help text

**What goes wrong:** D-06 names the `RLIMIT_AS`-is-address-space-not-RSS caveat, but the actual
code reveals a stronger trap: on Apple Silicon the `setrlimit` call frequently fails outright
(`EINVAL`) and the failure is **silently swallowed** — execution continues with NO memory limit
applied at all, only a stderr warning. A corrected help text that says "macOS enforces `--memory`
via `RLIMIT_AS` (address space, not RSS)" without mentioning the "and it may not apply at all" case
would still overstate the guarantee.
**Why it happens:** The RSS-vs-address-space nuance is the one CONTEXT.md names from memory of the
module's own docstring; the deeper "and it's advisory/best-effort, not guaranteed to apply"
behavior is one level down, in the `pre_exec` closure's error-handling branch, not the module-level
doc comment.
**How to avoid:** Base the corrected text on the literal stderr message the code already emits
("not enforced on macOS (best-effort); continuing") rather than re-deriving softer language.
**Warning signs:** Corrected help text that says macOS "enforces" `--memory` without a
best-effort/non-guaranteed qualifier.

### Pitfall 3: Chasing `cargo test --workspace` to a fabricated green

**What goes wrong:** An executor sees 24 failing tests and "fixes" some of them to make the number
smaller, silently expanding scope into unrelated files (`audit_attestation.rs`, `env_vars.rs`,
`resl_nix_async_signal_safety.rs`) that Phase 110-08 explicitly logged as pre-existing and
out-of-scope.
**Why it happens:** Pressure to present a clean verification report.
**How to avoid:** Reproduce the exact 24-name list; any name not in the list is a real signal, any
name in the list is expected and should be cited (not "fixed").
**Warning signs:** A verification report claiming `cargo test --workspace` exits 0, or fixing test
code in files this phase's `files_modified` list does not name.

### Pitfall 4: Forgetting `release-readiness.ps1`'s hardcoded `$targetVersion`/`$upstreamHighest`

**What goes wrong:** RLS-14 bumps all Cargo.toml/JSON manifests but the prepare-only gate script
still asserts against `0.66.1`/`0.66.0`, so the "release gate GREEN" success criterion is
technically satisfied against the wrong target — the gate would report the crates ARE at `0.66.1`
(false, they're now `0.70.0`) or worse, silently pass because nobody re-ran it after the bump and
it was never wired into CI to catch drift automatically.
**Why it happens:** This script is not a Cargo manifest, so a naive "grep all Cargo.toml for
0.66.1" sweep would miss it entirely.
**How to avoid:** Explicitly include `scripts/gates/release-readiness.ps1` lines 76-77 in RLS-14's
task file list.
**Warning signs:** `release-readiness.ps1` run after the bump reports `version_family` mismatches
or a `leapfrog` FAIL — that's actually a good sign the check is alive, but if the plan doesn't
budget a step to update the script first, it will always spuriously fail before it can ever pass.

## Contradictions Found

None. Every claim in this research is either directly consistent with, or a deeper-verified
elaboration of, CONTEXT.md's locked decisions. Two findings *sharpen* rather than contradict
CONTEXT.md's framing:

1. CONTEXT.md's D-06 names the macOS `RLIMIT_AS`-is-not-RSS caveat; live code inspection found a
   stronger, additional caveat (silent best-effort failure on `EINVAL`) that the corrected help
   text should also reflect. This does not contradict D-06 — D-06 explicitly says "verify... do
   not guess," and this is exactly the kind of detail that verification surfaces.
2. CONTEXT.md's "lands as-is" framing for CORE-01 (§ Claude's Discretion: "whether the two carries
   land in one plan or two — they are small and independent") is consistent with the finding that
   #1378's real payload is a single JSON line; the finding only adds the caution that a literal
   whole-commit cherry-pick is the wrong mechanism, not that the carry itself is larger or riskier
   than framed.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `docs/cli/usage/flags.mdx`'s identical stale text is worth correcting in the same phase (recommendation, not a locked requirement) | CORE-02 | Low — if deferred, only the public docs site (not `--help` output or any test) remains momentarily inconsistent with the corrected CLI text; easy follow-up. |
| A2 | The `npm/win32-x64-msvc/package.json` version-string bump (a file that doesn't exist locally) should still be applied to the `optionalDependencies` pin in the main `package.json` | RLS-14 | Low — worst case, a string bump to a still-nonexistent reference is inert; omitting it would leave a stale `0.66.1` reference sitting next to the other 4 correctly-bumped platform pins, a minor inconsistency Phase 105 will need to reconcile anyway. |

**All other claims in this research were verified live** (file reads, `git show`, `grep`, or
direct command execution on this host) — no other assumption requires user confirmation before
becoming a locked planning decision.

## Open Questions

1. **Does CORE-01's `~/.cache` addition need a corresponding `read`-only vs `readwrite` review?**
   - What we know: upstream added `~/.cache` to the `readwrite` array (same array as
     `~/Library/Caches`/`~/Library/Logs`), matching the existing convention for that policy group.
   - What's unclear: nothing substantive — the addition is a straight parallel to the existing
     entries in the same array.
   - Recommendation: apply as-is; no further design decision needed.

2. **Should the `learn.rs` clippy-style simplification (`if/else` → `?`) bundled in `#1378` be
   picked up opportunistically since the file does exist in the fork?**
   - What we know: it is a pure, behavior-preserving simplification (`if let Some(x) = y.find(',')
     { x+2 } else { return None }` → `let x = y.find(',')?; x+2`), unrelated to the `~/.cache`
     feature.
   - What's unclear: whether bundling unrelated cosmetic cleanup into a CORE-01 task violates the
     "lands as-is" framing's spirit (scope creep) or is harmless housekeeping.
   - Recommendation: leave it out. It is not part of CORE-01's actual feature, and pulling in
     incidental upstream diffs from a commit whose only fork-relevant line is elsewhere sets a bad
     precedent for future syncs (silently expanding "adopt as-is" into "adopt plus whatever else
     was in the same PR").

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cross` + Docker | VERIFY-01 linux-gnu gate | ✓ | `cross` 0.2.5, Docker Desktop 29.6.2 (per 110-08 evidence) | PARTIAL→CI only on documented failure |
| `zig` + `cargo-zigbuild` | VERIFY-01 apple-darwin gate | ✓ | zig 0.16.0, cargo-zigbuild 0.23.0 | PARTIAL→CI only on documented failure |
| `make` | Referenced by CLAUDE.md's `make ci` | ✗ | — | Run the 5 constituent `cargo`/`cargo fmt`/`cargo audit` commands directly (documented, recurring substitution since Phase 102) |
| `maturin` | RLS-14 `../nono-py` rebuild | ✓ (per 110-08 evidence, same host) | — | — |
| `napi`/`npx` | RLS-14 `../nono-ts` rebuild | ✓ via `npx` (no global `napi` binary) | `@napi-rs/cli` 3.6.0 | `npx napi build ...` |
| `cargo-audit` | `make ci`'s audit leg | ✓ | 0.22.1 (confirmed via live run) | — |
| PowerShell (`pwsh`) | `release-dry-run.ps1`, `release-readiness.ps1` | ✓ (primary shell on this host) | — | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** `make` (fallback: direct `cargo`/`cargo fmt`/`cargo audit`
invocations, already the established pattern on this host since Phase 102-02/102-05).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust's built-in `cargo test` (workspace-native); no snapshot/insta/trycmd framework present |
| Config file | none dedicated — driven by `Cargo.toml` per-crate `[[test]]`/integration-test-dir convention |
| Quick run command | `cargo test -p nono-sandbox-cli --bin nono -- <filter>` (targeted, per Phase 110-08 precedent) |
| Full suite command | `cargo test --workspace --no-fail-fast` (must be `--no-fail-fast` to see the full 24-failure baseline, not just the first binary) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CORE-01 | `~/.cache` present in the macOS Seatbelt-generated profile for `user_caches_macos`-consuming profiles | unit (schema/policy load) | `cargo test -p nono-sandbox-cli -- policy` (existing policy-load tests; a new assertion that `user_caches_macos.allow.readwrite` contains `~/.cache` should be added) | ❌ — new assertion needed, Wave 0 gap |
| CORE-01 | `MAX_CRYPTO_THREADS == 12` and the threading-guard arms still compile/pass | unit | `cargo test -p nono-sandbox-cli --bin nono -- exec_strategy` (existing threading-guard tests exercise the constant indirectly; a direct `assert_eq!(MAX_CRYPTO_THREADS, 12)` is trivial to add) | ❌ — trivial assertion, Wave 0 gap |
| CORE-02 | Corrected help text matches the true per-platform enforcement (no test currently asserts on help strings) | manual / doc-review | `cargo run -p nono-sandbox-cli -- run --help` (visual diff against the corrected text) | ✅ — command exists, no automated snapshot test to add (no insta/trycmd in this crate) |
| CORE-02 | Existing flag-parsing regression guards still pass unmodified | unit | `cargo test -p nono-sandbox-cli --bin nono -- cpu_percent_range_enforced_by_clap max_processes_range_enforced_by_clap memory_zero_rejected_by_parser` | ✅ |
| VERIFY-01 | Both cross-target clippy gates GREEN | integration/gate | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` + `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✅ |
| VERIFY-01 | `make ci` constituents GREEN (clippy/fmt/test/audit) | integration/gate | see § VERIFY-01 `make ci` substitution above | ✅ |
| VERIFY-01 | `cargo test --workspace` failure set == documented 24-name baseline (no new regressions) | integration/gate | `cargo test --workspace --no-fail-fast 2>&1 \| grep FAILED` diffed against the 24-name list above | ✅ |
| RLS-14 | All 6 version-family crates report `0.70.0`; `sign-fixture` unchanged | gate | `scripts/gates/release-readiness.ps1` (after updating its `$targetVersion`/`$upstreamHighest`) | ✅ (after the required script edit) |
| RLS-14 | Prepare-only release gate GREEN, no live upload | gate | `pwsh -File scripts/release-dry-run.ps1` | ✅ |
| RLS-14 | Both sibling bindings rebuild clean at `0.70.0` | integration | `maturin build` in `../nono-py`; `npx napi build --platform --release` in `../nono-ts` | ✅ |

### Sampling Rate

- **Per task commit:** targeted `cargo test -p nono-sandbox-cli --bin nono -- <filter>` for the
  specific area touched (policy/exec_strategy/cli.rs).
- **Per wave merge:** both cross-target clippy gates (mandatory, per D-10, for any wave touching
  `policy.json`/`exec_strategy.rs`/`cli.rs`).
- **Phase gate:** full `make ci` substitution set + `cargo test --workspace --no-fail-fast`
  (baseline-diffed) + both binding rebuilds + `release-readiness.ps1` + `release-dry-run.ps1`,
  before `/gsd:verify-work`.

### Wave 0 Gaps

- [ ] A direct assertion that `crates/nono-cli/data/policy.json`'s `user_caches_macos` group's
      `readwrite` array contains `~/.cache` (currently no test checks this array's contents by
      value, only that the group loads/parses).
- [ ] A direct `assert_eq!(MAX_CRYPTO_THREADS, 12)` (or equivalent) — currently only exercised
      indirectly through the threading-guard match arms, which would pass with any value ≥ actual
      thread count and would not catch a regression back to 7 unless a test specifically drives
      thread count into the 8-12 range.
- [ ] No test exists asserting the corrected help-text content itself (acceptable — no
      snapshot-test framework is present in this crate; manual `--help` review is the established
      pattern here, consistent with the rest of `cli.rs`'s ~150 other flags having no snapshot
      coverage either).

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Not touched by this phase. |
| V3 Session Management | No | Not touched by this phase. |
| V4 Access Control | Yes | Resource-limit enforcement (Job Object / cgroup v2 / `setrlimit`) is an access-control-adjacent (resource-exhaustion-prevention) mechanism; CORE-02 explicitly forbids weakening it — verification, not modification, of the existing kernel-enforced controls. |
| V5 Input Validation | Yes | `parse_byte_size`/`parse_cpu_percent`/`parse_duration` clap value-parsers (unchanged by this phase; verified still covered by existing unit tests: `parse_byte_size_rejects_invalid`, `parse_byte_size_rejects_overflow`, `parse_duration_rejects_invalid`). |
| V6 Cryptography | No | Not touched. |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Resource-exhaustion / fork-bomb via unbounded sandboxed-process trees | Denial of Service | Kernel-enforced Job Object / cgroup v2 `pids.max` / macOS `RLIMIT_NPROC` — already implemented; this phase only corrects documentation about it, per CORE-02's explicit no-new-enforcement constraint. |
| Help-text/documentation drift understating actual security posture (a user reads "not enforced" and reaches for a weaker external mitigation, or conversely over-trusts a limit that is actually best-effort) | Information Disclosure (of incorrect security posture, not data) | D-05/D-06's corrected help text — this IS the mitigation this phase delivers for CORE-02; the research's Pitfall 2 above is the concrete failure mode to avoid (don't swap "understated" for "overstated"). |
| Version/package-identity confusion during a multi-repo, multi-registry leapfrog (RLS-14) | Spoofing (package-identity) | Already mitigated structurally by Phase 102's fork-owned `nono-sandbox`/`@oscarmackjr/nono-ts` naming; RLS-14 only bumps version strings within already-owned identities — no new identity-confusion surface. |

## Sources

### Primary (HIGH confidence — read live from this repo)
- `crates/nono-cli/src/cli.rs` lines 15-121, 2740-2824, 3880-3995, 4120-4260 — resource-flag
  definitions, parsers, existing regression-guard tests.
- `crates/nono-cli/src/launch_runtime.rs` lines 180-234 — the accurate `ResourceLimits` doc
  comment used as the corrected-text source of truth.
- `crates/nono-cli/src/exec_strategy/supervisor_macos.rs` (full file) — macOS `RLIMIT_AS`/
  `RLIMIT_NPROC` enforcement, module-level doc table, `uid_process_count`, `install_pre_exec`.
- `crates/nono-cli/src/exec_strategy.rs` lines 160-172, 895-943, 1100-1160 — `MAX_CRYPTO_THREADS`
  constant + consumers, macOS `pre_exec` inline resource-limit application, `MSG_RLIMIT_AS_WARN`.
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` lines 2099-2400+ — cgroup v2 session
  detection, controller enablement, `memory.max`/`cpu.max`/`pids.max` writes.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` lines 454-568 — Job Object
  `apply_resource_limits`, fail-closed contract.
- `crates/nono-cli/data/policy.json` lines 320-350 — `user_caches_macos`/`user_caches_linux` groups.
- `crates/nono/src/lib.rs` lines 48-123 — confirms no `pub mod resource;` exists.
- `crates/nono/src/error.rs` line 20 — `CGROUP_V2_HINT`.
- `docs/cli/usage/flags.mdx` lines 1232-1265 — the docs-site copy of the same stale claim.
- `Makefile` lines 1-150 — `make ci`'s exact constituent commands.
- `scripts/release-dry-run.ps1` (full file) — prepare-only release gate, safety invariants.
- `scripts/gates/release-readiness.ps1` lines 60-176 — hardcoded `$targetVersion`/`$upstreamHighest`.
- Root `Cargo.toml` `members` array + all 8 workspace-member `Cargo.toml` files — version/name
  inventory.
- `../nono-py/Cargo.toml`, `../nono-py/pyproject.toml` — sibling repo version fields.
- `../nono-ts/Cargo.toml`, `../nono-ts/package.json`, `../nono-ts/npm/*/package.json` — sibling
  repo version fields + `optionalDependencies`.
- `.planning/templates/cross-target-verify-checklist.md` (full file) — cross-target gate protocol.
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` lines 760-810 — CORE
  cluster per-commit table, Windows-relevance finding, threat flag.
- `.planning/phases/110-profile-policy-absorb-platform-overrides/110-08-VERIFICATION-NOTES.md`
  (full file) — the 24-failure baseline, both gate outputs, binding-rebuild evidence.
- `.planning/phases/110-profile-policy-absorb-platform-overrides/deferred-items.md` (full file) —
  per-failure root-cause detail.
- `proj/ADR-86-library-boundary-convergence.md`, `proj/ADR-108-deny-domain-posture.md` (full
  files) — boundary precedent + ADR shape to mirror for ADR-111.
- `git show ca888108fe5983be866e8d1a6eccf96edc2a8dd5` (full commit, all 6 file diffs) — verified
  live against this repo's fetched upstream history.
- `git show 099237da94d33752ca5617db3f4902a216f9f84c` (full commit) — verified live.
- Live command execution on this host: `cargo audit` (exit 0), `command -v make` (exit 1),
  `cargo audit` output review (RUSTSEC-2026-0204 absent, 6 unrelated allowed warnings present).
- `.planning/quick/260729-nh4-crossbeam-epoch-rustsec-2026-0204/SUMMARY.md` — confirms the
  crossbeam-epoch RUSTSEC fix already landed pre-phase.
- `.planning/config.json` — confirms `nyquist_validation: true`, `security_enforcement` absent
  (treated enabled).

### Secondary (MEDIUM confidence)
- None used — every claim above was verifiable directly against this repo's live tree or git
  history; no external web research was needed for this phase's domain.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- CORE-01: HIGH — both upstream commits read in full via `git show`; both diffs verified against
  the current fork tree line-by-line.
- CORE-02: HIGH — all three enforcement backends read in full; the stale and corrected text both
  quoted verbatim from live files; confirmed by direct grep that no test depends on the stale text.
- VERIFY-01: HIGH — gate commands cited from the existing single-source-of-truth checklist; the
  24-failure baseline and `make ci` substitution both read from live, dated verification records
  and confirmed live (`cargo audit`, `command -v make`).
- RLS-14: HIGH — every version-bearing file enumerated and read live across this repo and both
  sibling repos; the two non-obvious hardcoded-version-string gate scripts found by targeted grep,
  not by chance.

**Research date:** 2026-08-04
**Valid until:** Short-lived — 7 days, since this research depends on exact line numbers and a
git-history snapshot (`git show <sha>`) that will shift if any file it cites is edited before
planning executes. If planning is delayed materially past 2026-08-11, re-verify the exact line
ranges in `cli.rs`/`launch_runtime.rs`/`exec_strategy.rs` before trusting the quoted verbatim text.
