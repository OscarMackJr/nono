# Phase 112: Security + Residual Sync - Pattern Map

**Mapped:** 2026-08-05
**Files analyzed:** 12 (1 genuinely-new code file, 3 documentation deliverables, 5 modified-existing
library/CLI files, 4 Wave-0 test gaps layered onto 3 of those 5 files)
**Analogs found:** 12 / 12 (every target has a concrete analog; no "No Analog Found" section needed)

**Framing note (read first):** This is an upstream-absorb phase, not greenfield. "Analog" below
means one of two things depending on the file:
- For the one genuinely new file (`proxy_command.rs`), the analog is a sibling command module the
  planner should structurally imitate.
- For every modified-existing file, the "analog" IS the file itself — the pattern to extract is the
  file's own current convention (a fail-closed guard, a stacked-ruleset boundary, a cfg-split) that
  the absorb must not silently overwrite. This distinction is called out per-row.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/nono-cli/src/proxy_command.rs` (NEW, SEC-07) | controller (CLI command module) | request-response (subcommand dispatch) | `crates/nono-cli/src/rollback_commands.rs` (+ `audit_commands.rs`) | exact (structural) |
| `crates/nono-cli/src/cli.rs` — `Commands::Proxy` variant (SEC-07) | route (clap enum arm) | request-response | `Commands::Rollback(RollbackArgs)` (cli.rs:869) | exact |
| `crates/nono-cli/src/app_runtime.rs` — dispatch arm (SEC-07) | route (match dispatch) | request-response | `Commands::Rollback(args) => ...` (app_runtime.rs:98-100) | exact |
| `crates/nono-cli/src/cli_bootstrap.rs` — verbosity match arm (SEC-07) | route (exhaustive match) | request-response | `Commands::Rollback(_) \| Commands::Trust(_) \| ...` (cli_bootstrap.rs:280-299) | exact |
| `crates/nono/src/sandbox/linux.rs` — `restrict_execute()` (SEC-05) | service (library primitive) | CRUD (Landlock ruleset construction) | itself — the pre-patch `restrict_execute()` at `:1105-1157` is the analog for the stacked-ruleset pattern to extend | exact (self) |
| `crates/nono/src/sandbox/linux.rs` — NVIDIA procfs (SEC-03) | service (library primitive) | transform (predicate/path-collection) | `is_nvidia_compute_device` + test at `:5057-5078` ("D-13 upstream parity port") — extend, don't recreate | exact (self) |
| `crates/nono-cli/src/exec_strategy.rs` — orphan-reap (SEC-06) | service (process supervision) | event-driven (waitpid/reap loop) | `linux_child_requires_dumpable()` (`:445-447`) + `run_supervisor_loop` `#[cfg(target_os = "linux")]` variant (`:3111`) | role-match (predicate exists; reap loop is new) |
| `crates/nono/src/trust/types.rs` — predicate discriminator (SEC-04) | model (data/format validation) | transform | `TrustPolicy::validate_version()` + `TRUST_POLICY_VERSION` const (`:12`, `:71-77`) | exact (self) |
| `crates/nono-cli/src/profile_runtime.rs` — `allow_vars` (SEC-08) | service (policy resolution) | transform | `allowed_env_vars` field-build closure (`:840-872`) — this IS the fail-closed guard to preserve | exact (self, guard-preservation case) |
| `crates/nono-cli/src/pty_proxy.rs` — CPR-reply drain (RES-02) | utility (terminal I/O teardown) | event-driven (drain-on-teardown) | `drain_terminal_output()` (`:1495-1513`) + `restore_terminal()` call site (`:688-697`) | role-match |
| `crates/nono-cli/tests/socket_access_run.rs` (SEC-06 orphan-reap regression test) | test | integration (subprocess CLI run) | itself — `af_unix_mediation_pathname_allows_connect_to_listed_socket` (`:128`) is the shape to imitate for a new orphan-reap integration test | role-match |
| `crates/nono-cli/tests/` (SEC-07 subcommand dispatch test, NEW) | test | integration (subprocess CLI run) | `crates/nono-cli/tests/socket_access_run.rs` (`nono_bin()`/`run_nono()` harness, `:18-48`) | role-match |

## Documentation Deliverables (no code role/data-flow — separate table)

| Deliverable | Target Path | Analog | Gitignored-but-tracked? |
|---|---|---|---|
| D-05 ledger addendum (all 18 SHA dispositions) | `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` | "## Phase 111 Standing Divergence Addendum" (`:1648-1682`) | **YES** — `.planning/` itself is tracked normally (not gitignored); only `proj/` and `docs/cli/development/` need `git add -f`. Confirm before assuming this file needs `-f`. |
| SEC-01 won't-sync finding document (new file) | `.planning/phases/112-security-residual-sync/` | `.planning/phases/109-proxy-network-absorb/109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` (full file read, see excerpt below) | No (`.planning/phases/` is tracked normally) |
| SEC-09 carry-forward note (v3.7 tool-sandbox work-list) | wherever the v3.7 milestone work-list lives (deferred-idea record / future ROADMAP amendment) | Same `won't-sync` + explicit-obligation shape as the SEC-01 finding doc's "Conclusion" section; also mirrors ADR-111's "Distinguished from... DEFERRED->v3.7" framing (`108-DIVERGENCE-LEDGER.md:1672-1681`) | No |
| Contingent ADR-112 (SEC-08, only if the planner confirms the guard-relaxation judgment needs escalation) | `proj/ADR-112-<slug>.md` | `proj/ADR-111-resource-limits-boundary.md` (header + Context + Decision shape, see excerpt below); also `proj/ADR-108-deny-domain-posture.md` | **YES** — `proj/` is gitignored but tracked. New file needs `git add -f proj/ADR-112-*.md`; plain `git add` exits 1 and will break `&& git commit` chains. |

---

## Pattern Assignments

### `crates/nono-cli/src/proxy_command.rs` (NEW — SEC-07, highest-value output)

**Analog:** `crates/nono-cli/src/rollback_commands.rs` (structure) + `crates/nono-cli/src/audit_commands.rs` (dispatch-fn shape) + `crates/nono-cli/src/proxy_runtime.rs` (import/domain-type conventions — this file already exists and will be *extended*, not created, per RESEARCH.md's diffstat: `+110 lines` to `proxy_runtime.rs`).

**1. `cli.rs` — declare the `Commands::Proxy` enum variant** (pattern from `Commands::Rollback`, `cli.rs:855-869`):
```rust
    // Doc comment above enum arm becomes the top-level `nono --help` one-liner.
    /// Manage a standalone network-filtering proxy (no sandboxed child)
    #[command(subcommand_help_heading = "COMMANDS", disable_help_subcommand = true)]
    #[command(help_template = "\
{about}

\x1b[1mUSAGE\x1b[0m
  nono proxy <command>

{all-args}
{after-help}")]
    #[command(after_help = "\x1b[1mEXAMPLES\x1b[0m
  nono proxy start --allow-domain api.openai.com   # Start standalone proxy
  nono proxy stop                                  # Stop the running proxy
")]
    Proxy(ProxyArgs),
```
Then, mirroring `RollbackArgs`/`RollbackCommands` (`cli.rs:3149-3172`), declare `ProxyArgs { #[command(subcommand)] pub command: ProxyCommands, help: Option<bool> }` and a `ProxyCommands` enum with one variant per upstream subcommand (upstream `2663e990` diffstat: `cli.rs` — check `git show 2663e990 -- crates/nono-cli/src/cli.rs` for exact upstream subcommand names before inventing new ones).

**2. `app_runtime.rs` — wire the dispatch arm** (pattern from `Commands::Rollback`, `app_runtime.rs:98-100`):
```rust
        Commands::Proxy(args) => run_command_with_update(update_handle, silent, || {
            proxy_command::run_proxy(args)
        }),
```

**3. `cli_bootstrap.rs` — the verbosity match is EXHAUSTIVE; the compiler will force this addition** (`cli_bootstrap.rs:280-299`):
```rust
        Commands::Why(_)
        | Commands::Classify(_)
        | Commands::Rollback(_)
        | Commands::Trust(_)
        | Commands::Audit(_)
        // ... ADD: | Commands::Proxy(_)
```
This is not optional polish — omitting it is a compile error, not a silent gap, so it is low-risk to
forget-and-catch, but flag it in the plan's task list explicitly so the executor doesn't burn a cycle
rediscovering it.

**4. `main.rs` — module registration** (pattern from `mod rollback_commands;` at `main.rs:92`):
```rust
mod proxy_command;
```
Insert alphabetically among the existing `mod` declarations (main.rs:5-100 is alphabetized; `proxy_command` sits between `protected_paths` and `proxy_runtime`).

**5. `proxy_command.rs` — dispatch-fn body** (pattern from `audit_commands.rs:28-36` / `rollback_commands.rs:133-142`):
```rust
//! Proxy subcommand implementations
//!
//! Handles `nono proxy start|stop|...` for standalone (non-sandboxed-child)
//! proxy operation.

use crate::cli::{ProxyArgs, ProxyCommands /* , per-subcommand Args types */};
use nono::{NonoError, Result};

/// Dispatch to the appropriate proxy subcommand.
pub fn run_proxy(args: ProxyArgs) -> Result<()> {
    match args.command {
        ProxyCommands::Start(args) => cmd_start(args),
        ProxyCommands::Stop(args) => cmd_stop(args),
        // ... one arm per subcommand, upstream 2663e990's actual set
    }
}
```
Route into `crates/nono-proxy/src/{server,external,reverse,token}.rs` via `proxy_runtime.rs`'s
existing helpers (`resolve_effective_proxy_settings`, `EffectiveProxySettings` — see
`proxy_runtime.rs:1-31` for the existing type/import shape to extend, not replace).

**Sequencing constraint (RESEARCH.md, load-bearing):** SEC-07 (`2663e990`) creates `proxy_command.rs`
*before* SEC-02a (`9b692e07`) makes a 6-line edit to it. If SEC-02a lands in any form this phase,
SEC-07 must be absorbed first. Per D-04/RESEARCH.md's Summary, SEC-02a is very likely NOT a clean
absorb this phase (adapt-down-vs-defer judgment call) — so in practice SEC-07 stands alone.

**Error handling pattern to copy** (from `audit_commands.rs:52-57`, `NonoError::ConfigParse` for a
missing-required-arg guard):
```rust
    if args.keep.is_none() && args.older_than.is_none() && !args.all {
        return Err(NonoError::ConfigParse(
            "audit cleanup: specify one of --keep <N>, --older-than <DAYS>, or --all".to_string(),
        ));
    }
```
Use the same `NonoError` variant + `Result<()>` return shape for `proxy_command.rs`'s own arg
validation — no `.unwrap()`/`.expect()` (CLAUDE.md `clippy::unwrap_used` is `-D`).

---

### `crates/nono/src/sandbox/linux.rs` — `restrict_execute()` (SEC-05, Landlock `Refer` in execute-restriction layer)

**Analog: itself.** This is a "modify-in-place, preserve the stacking boundary" case, not a
cross-file-copy case. Confirmed byte-for-byte pre-patch state at `linux.rs:1105-1157`:

```rust
// crates/nono/src/sandbox/linux.rs:1105-1146 (live fork, pre-Phase-112)
pub fn restrict_execute(paths: &[impl AsRef<Path>]) -> Result<()> {
    let abi = detect_abi()?;
    if !abi.has_execute() {
        return Err(NonoError::SandboxInit(format!(
            "Tool Sandbox  execute restriction requires Landlock ABI V3+; detected {}",
            abi.version_string()
        )));
    }

    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::Execute)   // <- upstream d84b4818 ALSO adds .handle_access(AccessFs::Refer) here
        .map_err(|e| { /* ... */ })?
        .set_compatibility(CompatLevel::BestEffort)
        .create()
        .map_err(|e| { /* ... */ })?;

    for path in paths {
        // ... ruleset.add_rule(PathBeneath::new(fd, AccessFs::Execute)) ...
        // <- upstream ALSO adds: if abi.has_refer() { add a bare-Refer rule on "/" here }
    }

    let status = ruleset.restrict_self().map_err(|e| { /* ... */ })?;
    ensure_execute_restriction_fully_enforced(status.ruleset)?;
    Ok(())
}
```

**Key fact (resolves the "is this a duplicate grant?" question with certainty):** this is a
**separate, stacked** Landlock ruleset from the main `apply_landlock`/`access_to_landlock` ruleset,
which already grants `Refer` under `AccessMode::Write` at `linux.rs:365` (confirmed via
`access_to_landlock()`, `:349-365`). Landlock requires each stacked layer to independently grant
`Refer` or cross-directory renames break under this layer specifically — the general grant at
`:365` does NOT propagate here. This satisfies CLAUDE.md's "Landlock is strictly allow-list, cannot
express deny-within-allow" constraint because the fix is an *additional* grant in a *stacking*
layer, not a deny-within-allow expression.

**Error-handling convention to preserve:** every fallible Landlock call in this function wraps its
error in `NonoError::SandboxInit(format!("Tool Sandbox  execute restriction: <step>: {e}"))` — note
the literal double-space in `"Tool Sandbox  execute restriction"` (typo, present 5+ times in this
function; preserve it verbatim in any new error strings added by the port, don't "fix" it as a
drive-by — an inconsistent subset of matching strings would break substring-based test assertions).

**Wave-0 regression test to port** (RESEARCH.md Validation Architecture): upstream `d84b4818` itself
adds `test_restrict_execute_does_not_break_rename_into_new_subdir`. Port it verbatim; run via
`cross test --target x86_64-unknown-linux-gnu -p nono-sandbox`. Existing test-module gating
convention in this file uses `#[cfg(target_os = "linux")]` on individual `#[test]` fns inside an
un-gated `mod tests` (see SEC-03's analog below) — follow that, not a file-level `#![cfg(...)]`.

---

### `crates/nono/src/sandbox/linux.rs` — NVIDIA procfs mediation (SEC-03)

**Analog: itself**, extending a prior partial port. Existing test at `linux.rs:5057-5078`:

```rust
// crates/nono/src/sandbox/linux.rs:5057-5078 (live fork)
// --allow-gpu Linux Landlock path list (D-13 upstream parity port)
//
// Tests the pure predicate `is_nvidia_compute_device` and asserts
// that `collect_linux_gpu_paths` produces a list shape consistent
// with the upstream contract. File-level checks (path existence) are
// not asserted — those depend on the host having NVIDIA/AMD hardware.

#[cfg(target_os = "linux")]
#[test]
fn test_is_nvidia_compute_device_accepts_upstream_list() {
    // Upstream upstream parity list: nvidiactl + nvidia-uvm +
    // nvidia-uvm-tools + nvidia[0-9]+.
    for name in [
        "nvidiactl", "nvidia-uvm", "nvidia-uvm-tools",
        "nvidia0", "nvidia1", "nvidia9", "nvidia42",
    ] {
        assert!(is_nvidia_compute_device(name), /* ... */);
    }
}
```
The fork already carries a prior partial port ("D-13 upstream parity port" comment) — SEC-03 EXTENDS
this predicate/test pair, it does not create a new one. Confirm via `git show a3243907 --
crates/nono/src/sandbox/linux.rs` exactly which lines are new vs. already covered by D-13 before
writing the absorb task. CLI-side wiring analog: `crates/nono-cli/src/capability_ext.rs:1468`/`:1512`
(`test_from_args_allow_gpu_sets_capability_on_unix`,
`test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu`) — the `--allow-gpu` flag
parse-to-capability wiring already exists and is CLI-owned policy per the architectural responsibility
map (library = mechanism, CLI = when-to-apply decision).

---

### `crates/nono-cli/src/exec_strategy.rs` — orphan-reap (SEC-06)

**Analog:** the existing predicate function + its test, both in this same file — the pattern to
imitate for the NEW orphan-reap mechanism upstream `ac5ccd70` adds.

**Existing predicate (pure function, unit-testable in isolation)** — `exec_strategy.rs:444-447`:
```rust
#[cfg(target_os = "linux")]
const fn linux_child_requires_dumpable(capability_elevation: bool, network_notify: bool) -> bool {
    capability_elevation || network_notify
}
```
**Existing test for it** — `exec_strategy.rs:4450-4457`:
```rust
#[cfg(target_os = "linux")]
#[test]
fn test_linux_child_requires_dumpable_only_for_seccomp_driven_features() {
    assert!(!linux_child_requires_dumpable(false, false));
    assert!(linux_child_requires_dumpable(true, false));
    assert!(linux_child_requires_dumpable(false, true));
    assert!(linux_child_requires_dumpable(true, true));
}
```
This is the exact shape a new `orphan_reap_required(...)`-style pure predicate + matching
`#[cfg(target_os = "linux")] #[test]` should follow: **keep the gate-decision logic in a pure,
`const fn`-if-possible function separate from the syscall side effect**, so it's testable without a
live process tree.

**Adapt requirement (not a clean port):** upstream's diff guards on
`config.tool_sandbox_runtime.is_some() || config.seccomp_policy.child_requires_dumpable()`. The fork
has **neither** field/method (confirmed: `tool_sandbox_runtime` absent per D-01/SEC-09; no
`seccomp_policy.child_requires_dumpable()` method exists). The fork's real equivalent predicate is
already `linux_child_requires_dumpable` above, called at `exec_strategy.rs:1524`'s
`PR_SET_DUMPABLE(0)` site (see excerpt below). Absorb = re-wire the new `PR_SET_CHILD_SUBREAPER` +
`reap_reparented_orphans()` call to gate on
`linux_child_requires_dumpable(config.capability_elevation, config.seccomp_proxy_fallback ||
config.af_unix_mediation.is_pathname())` — **dropping the `tool_sandbox_runtime` disjunct entirely**
(nothing to gate on; carries no meaning in this fork).

**`PR_SET_DUMPABLE` call-site convention to match** (`exec_strategy.rs:1641-1660`):
```rust
            // PARENT: Apply ptrace hardening immediately. This is CRITICAL
            // because the parent is unsandboxed in Supervised mode.
            // Failure to harden is fatal - we kill the child and abort.
            #[cfg(target_os = "linux")]
            {
                use nix::sys::prctl;
                if let Err(e) = prctl::set_dumpable(false) {
                    let _ = signal::kill(child, Signal::SIGKILL);
                    let _ = waitpid(child, None);
                    return Err(NonoError::SandboxInit(format!(
                        "Failed to verify PR_SET_DUMPABLE(0) on supervised parent: {}. \
                         Aborting: unsandboxed parent must not be ptrace-attachable.",
                        e
                    )));
                }
            }
```
Fail-fast-and-kill-child-on-hardening-failure is the established convention (`nix::sys::prctl`, not
raw `libc::prctl`) — follow it for `PR_SET_CHILD_SUBREAPER`'s own failure path too.

**Call-site target (Assumption A3 from RESEARCH.md, needs confirming during the plan's own reality
check, not assumed here):** two `run_supervisor_loop` definitions exist, `#[cfg(not(target_os =
"linux"))]` at `exec_strategy.rs:2897` and `#[cfg(target_os = "linux")]` at `exec_strategy.rs:3111`.
Since `PR_SET_CHILD_SUBREAPER` and seccomp-notify orphan-reaping are both Linux-only concerns, the
new `reap_reparented_orphans()` call site belongs in the **`:3111` Linux variant only** — do not
duplicate into the non-Linux variant.

**Wave-0 test target (RESEARCH.md):** the `#[cfg(target_os = "linux")]`-gated predicate test above
extends cleanly; the actual orphan-reap mechanism itself has no existing test scaffolding to extend
— author a new one following the same pure-predicate-first pattern, run via `cross test`.

---

### `crates/nono/src/trust/types.rs` — predicate-field discriminator (SEC-04)

**Note:** the phase-guidance prompt named `crates/nono/src/trust/bundle.rs`; live-grep confirms the
actual target is `crates/nono/src/trust/types.rs` (RESEARCH.md's own citation, independently
re-confirmed here via `grep -rn "TRUST_POLICY_VERSION\|TRUST_POLICY_PREDICATE" crates/nono/src/trust/`).
Flag this correction for the planner.

**Analog: itself.** Current version-check pattern (`types.rs:11-12`, `:71-77`):
```rust
/// Current supported trust policy format version.
pub const TRUST_POLICY_VERSION: u32 = 1;

// ...
pub fn validate_version(&self) -> Result<()> {
    if self.version != TRUST_POLICY_VERSION {
        return Err(NonoError::TrustPolicy(format!(
            "unsupported trust policy version {} (expected {})",
            self.version, TRUST_POLICY_VERSION
        )));
    }
    // ... MAX_BLOCKLIST_ENTRIES / MAX_INCLUDES / MAX_FILES / MAX_PUBLISHERS bounds checks follow
    Ok(())
}
```
Upstream `f943fb5a` adds `#[deprecated(since = "0.66.0")]` on `TRUST_POLICY_VERSION` plus a new
`TRUST_POLICY_PREDICATE` URI-based const, additive to (not replacing) this struct/validation. Per
RESEARCH.md's State of the Art table, carry the deprecation marker forward — this is pure
data/format-detection, no policy embedding, and stays inside `crates/nono/src/trust/` (library
primitive, no ADR-86 boundary question). Preserve the existing `Result<()>`/`NonoError::TrustPolicy`
error convention and the `MAX_*` bounds-checking style for any new field validation.

---

### `crates/nono-cli/src/profile_runtime.rs` — `allow_vars` (SEC-08, guard-preservation case)

**Analog: itself — this IS the fail-closed guard that must NOT be overwritten.**
`profile_runtime.rs:840-872`:
```rust
        // Plan 34-08a Task 3 (D-20 manual replay of upstream `1b412a7`):
        // surface `profile.environment.allow_vars` as a runtime allow-list.
        // Plan 34-08a Task 5 (D-20 replay of v0.52.0 `780965d7`): preserve
        // fail-closed semantics for empty allow_vars. An empty `allow_vars`
        // list returns `Some([])` (strip all inherited vars) rather than
        // `None` (no filtering). Profiles that set env_credentials but omit
        // allow_vars would otherwise silently inherit every parent env var.
        allowed_env_vars: loaded_profile.as_ref().and_then(|profile| {
            profile.environment.as_ref().map(|env_config| {
                if let Some(err) = crate::exec_strategy::validate_env_var_patterns(
                    &env_config.allow_vars,
                    "allow_vars",
                ) {
                    eprintln!("Warning: {}", err);
                }
                env_config.allow_vars.clone()
            })
        }),
```
Whenever `profile.environment` is `Some(_)` at all, this resolves to `Some(env_config.allow_vars.clone())`
— never collapsed to `None` — so an `environment` block with `deny_vars` set but `allow_vars` omitted
already strips all inherited vars (the filter activates at `exec_strategy.rs:581`/`:796`,
`if let Some(ref allowed) = config.allowed_env_vars`).

**DO NOT adopt upstream `a5a441c2` verbatim.** It changes `EnvironmentConfig.allow_vars` from
`Vec<String>` to `Option<Vec<String>>` so that *omitting* `allow_vars` means "allow everything"
(`None`) — the **opposite default** from the fork's own deliberate fail-closed choice. Adopting it
as-is reopens the exact leak Plan 34-08a closed. This is D-05's named contingent-ADR-112 trigger
("any adopt that loosens an existing fork guard").

**Regression guard to run BEFORE and AFTER any SEC-08 change** (RESEARCH.md Validation Architecture):
```
cargo test -p nono-sandbox-cli --lib -- empty_allow_vars_fails_closed
```
located at `profile_runtime.rs:989` (test fn name confirmed via grep; full body not re-read here per
no-duplicate-read discipline — the planner should read `:975-1010` directly when writing the task).

**Recommended disposition for the plan to record:** won't-sync, or adapt with an explicit
fail-closed-preserving redesign (e.g., only adopt upstream's *other* 4 files' non-semantic changes —
docs, `.mdx` — while keeping `allow_vars: Vec<String>` unchanged) — flagged for ADR-112 escalation.

---

### `crates/nono-cli/src/pty_proxy.rs` — CPR-reply teardown drain (RES-02, `503045801a`)

**Analog:** the file's own existing "drain-on-teardown" family — `drain_terminal_output()`
(`pty_proxy.rs:1495-1513`) is the closest sibling to imitate for the new
`discard_late_terminal_input()`/`CprReplyParse` upstream adds:
```rust
// crates/nono-cli/src/pty_proxy.rs:1495-1513 (live fork)
fn drain_terminal_output(fd: RawFd) {
    // SAFETY: `isatty` only inspects the borrowed fd and does not take ownership.
    if unsafe { libc::isatty(fd) } != 1 {
        return;
    }

    loop {
        // SAFETY: `tcdrain` waits for queued terminal output on the borrowed fd.
        let ret = unsafe { libc::tcdrain(fd) };
        if ret == 0 {
            break;
        }
        let err = std::io::Error::last_os_error();
        if err.kind() != std::io::ErrorKind::Interrupted {
            debug!("PTY proxy: terminal output drain failed: {}", err);
            break;
        }
    }
}
```
Pattern to imitate: isatty-guard first, EINTR-retry loop, `debug!`-log-and-break on any other error
(never panic, never `.unwrap()`). The other two siblings in the same "drain" family —
`drain_socket_replay` (`:1329`) and `drain_attach_resize_pipe` (`:1756`) — confirm this is an
established multi-instance convention, not a one-off.

**Call-site integration point:** `restore_terminal()` (`pty_proxy.rs:688-697`) is the final-teardown
path where the new drain call belongs — confirmed additive-only by RESEARCH.md (new fn + one call
site, explicitly NOT touching `restore_terminal()`'s non-draining behavior used by suspend/resume):
```rust
    pub(crate) fn restore_terminal(&mut self) {
        if let Some(ref termios) = self.saved_termios {
            let _ = nix::sys::termios::tcsetattr(
                std::io::stdin(),
                nix::sys::termios::SetArg::TCSANOW,
                termios,
            );
            self.saved_termios = None;
        }
    }
```

**Correction to the phase-guidance prompt:** the prompt named
`crates/nono-cli/src/exec_strategy_windows/{launch,mod}.rs` as RES-02's target. Live-grep/RESEARCH.md
both confirm the real target is `pty_proxy.rs` + `timeouts.rs`, both reached only via the
`#[cfg(not(target_os = "windows"))] mod pty_proxy;` arm in `main.rs:85-86` (the Windows arm loads a
wholly different file, `pty_proxy_windows.rs`, at `main.rs:87-89`, which upstream's diff does not
touch). `pty_proxy_windows.rs` was grepped for `drain|tcdrain|discard|teardown` — zero hits, confirming
RES-02 has no Windows counterpart to port. Flag this correction; do not plan Windows-side work for
RES-02.

**Cross-target gate scope note (RESEARCH.md Pitfall 3):** `pty_proxy.rs` is `#[cfg(unix)]`-gated via
its `main.rs` mod-selection, not `target_os`-specific internally — it falls outside the
cross-target-verify-checklist's literal named-file list but is in-scope in spirit (any file reached
only under Unix-conditional compilation). Run both cross-target clippy gates on it anyway.

**Wave-0 test (RESEARCH.md):** no existing scaffolding to extend; author a new unit test around
`discard_late_terminal_input()`/`CprReplyParse`, following `drain_terminal_output`'s
isatty-guard/EINTR-retry-loop shape as the implementation pattern and a plain `#[test]` (this file is
`#[cfg(not(target_os = "windows"))]` at the module level via `main.rs`, so no additional per-test cfg
gate is needed inside `pty_proxy.rs` itself).

---

### Test file: `crates/nono-cli/tests/socket_access_run.rs` (SEC-06 regression test analog + SEC-07 dispatch test harness)

**Header + `#![cfg(...)]` gating convention** (`socket_access_run.rs:1-16`):
```rust
//! Runtime enforcement tests for AF_UNIX socket access control.
//! ...
//! These tests are Unix-only — Windows has no AF_UNIX socket mediation support.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::fs;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
```

**Subprocess-harness pattern** (`socket_access_run.rs:18-58`) — reusable for a new SEC-07
`nono proxy` dispatch integration test:
```rust
fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

fn run_nono(args: &[&str], home: &Path, cwd: &Path) -> Output {
    nono_bin()
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env_remove("NONO_DETACHED_LAUNCH")
        .current_dir(cwd)
        .output()
        .expect("failed to run nono")
}

// Socket paths must stay under the 104-byte SUN_LEN limit; use /tmp directly
// rather than std::env::temp_dir() which on macOS expands to a long path
// under /var/folders/... that can exceed the limit.
fn short_tempdir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("nono-sock-")
        .tempdir_in(std::path::Path::new("/tmp"))
        .expect("tempdir in /tmp")
}
```
`RES-02`'s `4cc0af2c` (test-only `/tmp` switch) directly parallels this file's own existing
`short_tempdir()` rationale — an independent confirmation that `/tmp` (not `std::env::temp_dir()`)
is this codebase's established convention for Unix-socket-path-length-safe test tempdirs. This
somewhat de-risks RESEARCH.md's Assumption A2 (macOS Seatbelt safety of `/tmp`) for the *socket-path*
use case specifically, though `4cc0af2c`'s actual target (`nono/supervisor/socket.rs`) is a
production, not test-only-in-this-file, code path — the caution in RESEARCH.md's per-SHA table
still applies to that specific file's own doc-commented Seatbelt-avoidance rationale.

**No existing `nono proxy` test exists** (confirmed: `Glob crates/nono-cli/tests/*.rs` lists 34
files, none proxy-command-specific) — SEC-07's dispatch test is a pure Wave-0 addition; use this
file's harness shape (`nono_bin()`/`run_nono()`) as the starting scaffold, run as
`cargo test -p nono-sandbox-cli --bin nono -- proxy` per RESEARCH.md's Phase Requirements → Test Map.

---

## Shared Patterns

### CLI subcommand wiring (4-site checklist, applies to SEC-07 only)
**Source:** `Commands::Rollback` end-to-end — `cli.rs:869` (enum variant) + `cli.rs:3149-3172`
(Args/Commands structs) + `app_runtime.rs:98-100` (dispatch) + `cli_bootstrap.rs:280-299`
(exhaustive verbosity match) + `main.rs:92` (`mod rollback_commands;`).
**Apply to:** `proxy_command.rs`'s full 4-site wiring (cli.rs, app_runtime.rs, cli_bootstrap.rs,
main.rs). Missing any of the 4 sites is either a compile error (cli_bootstrap.rs's exhaustive match,
main.rs's `mod` declaration) or a silent dead-command bug (app_runtime.rs's dispatch arm, if the
enum variant exists but no match arm routes it — though Rust's own exhaustiveness check on
`Commands` match blocks in `app_runtime.rs` and `cli.rs`'s own `cli_verbosity`/help-render matches
will also catch this at compile time).

### Fail-closed policy resolution (governs SEC-08, cited pattern to preserve, not to copy elsewhere)
**Source:** `crates/nono-cli/src/profile_runtime.rs:840-872`.
**Apply to:** any SEC/RES item touching security-adjacent defaults (RESEARCH.md's Common Pitfall 2:
"for every SEC/RES commit touching security-adjacent defaults ... read the fork's *current*
implementation and its own doc comments/tests before accepting the upstream commit's
self-description"). The doc-comment convention of citing a `Plan NN-NNx Task N (D-NN ...)` provenance
trailer is itself a project convention — any new guard-preserving decision in SEC-08's absorb task
should cite Phase 112 the same way.

### Stacked Landlock ruleset error-message convention (governs SEC-05)
**Source:** `crates/nono/src/sandbox/linux.rs:1105-1157`, `NonoError::SandboxInit(format!("Tool
Sandbox  execute restriction: <step>: {e}"))` (note the verbatim double-space).
**Apply to:** any new error path added inside `restrict_execute()` by the SEC-05 absorb — match the
existing string prefix exactly, including the double-space, for `grep`/test-assertion stability.

### `#[cfg(target_os = "linux")]`-gated unit test convention (governs SEC-03/05/06's Wave-0 tests)
**Source:** `crates/nono/src/sandbox/linux.rs:5064-5065` and
`crates/nono-cli/src/exec_strategy.rs:4450-4451` — both gate individual `#[test]` fns with
`#[cfg(target_os = "linux")]` inside an otherwise-ungated `mod tests`, rather than gating the whole
test module or file. Both are verified live via `cross test --target x86_64-unknown-linux-gnu` per
D-06/Phase 111-01 precedent (native Windows `cargo test` silently reports "0 tests" for these — a
silent-pass trap, not a genuine pass).
**Apply to:** all 4 Wave-0 test gaps this phase adds (SEC-05 ported regression test, SEC-06
orphan-reap test, SEC-07 subcommand dispatch test [Unix+Windows, no cfg gate needed — CLI dispatch
is cross-platform], RES-02 CPR-reply drain test [gated only by `pty_proxy.rs`'s own
`#[cfg(not(target_os = "windows"))]` module selection, no additional per-test cfg needed]).

## Documentation Deliverable Excerpts

### D-05 ledger addendum — exact shape to extend (`108-DIVERGENCE-LEDGER.md:1642-1682`)
The addendum sits AFTER the "Ledger closed" declaration, which explicitly names its own single
locked exception:
```
**Ledger closed.** All 5 plans in Phase 108 (108-01 through 108-05) have now contributed to this
document; no further plan is expected to append to `108-DIVERGENCE-LEDGER.md` — **except the one
deliberate, locked exception recorded immediately below** (Phase 111 CONTEXT.md D-02).

---

## Phase 111 Standing Divergence Addendum
[... commits, disposition, rationale, "Distinguished from ... DEFERRED->v3.7" closing section ...]
```
**Do not edit the "Ledger closed" sentence.** Phase 112's own addendum must follow the identical
shape: a new `---`-delimited `## Phase 112 Security + Residual Sync Addendum` section appended after
the Phase 111 addendum, itself declaring (in its own closing line) that it is the *next* locked
exception, so a hypothetical Phase 113+ auditor knows where to look. The addendum's per-SHA table
should reuse RESEARCH.md's Per-SHA Disposition Table columns (SHA, Req, disposition, cited evidence)
— RESEARCH.md's own table (lines 55-76) is largely append-ready as-is per Claude's Discretion in
CONTEXT.md ("format of the reality-check disposition table... provided every one of the 18 SHAs
appears with an explicit disposition and cited evidence").

### SEC-01 won't-sync finding document — exact shape (full file read: `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`)
Section skeleton to imitate: `# Phase NNN: <slug> Disposition Finding` header with **Written**/
**Status** metadata lines → `## Summary` → `## Re-verification evidence` (with `ls`/`grep` command
transcripts as inline evidence, e.g. `$ ls crates/nono-proxy/src/aws` / `ls: cannot access ...`) →
per-item subsections with code excerpts proving the placeholder/absent state → `## Conclusion`
explicitly stating what WAS absorbed vs. what is N/A and why implementing the missing subsystem is
out of scope ("Either would be a net-new feature... not a bug-fix absorb. If either is ever
prioritized, it needs its own future phase"). SEC-01's finding should cite this exact precedent
document by path, since it is literally the same absent `aws/`/`tls_intercept/` subsystem this
commit originates.

### Contingent ADR-112 — exact shape (`proj/ADR-111-resource-limits-boundary.md:1-60`)
Header block:
```
# ADR-111: Resource-Limit CLI Surface — Reject the Core-Module Absorb

**Status:** Accepted
**Phase:** 111 — Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog
**Date:** 2026-08-04
**Authors:** Phase 111 execution

---

## Context
[upstream commit description, Phase 108 audit's prior classification + threat flag, fork's
current/superior implementation described in detail]

---

## Decision

**ADAPT, not adopt.**
[explicit statement of what will NOT be added, what stays unchanged, and why]
```
If SEC-08's guard-relaxation judgment is escalated (D-05's contingent trigger), title it
`# ADR-112: allow_vars Fail-Closed Semantics — Reject the Option<Vec<String>> Reinterpretation` (or
similar), reusing this exact Status/Phase/Date/Authors header block and Context/Decision section
shape. **Remember `git add -f proj/ADR-112-*.md`** — `proj/` is gitignored but tracked.

## No Analog Found

None. Every file in scope has a concrete analog (self-referential for modified-existing files,
sibling-command-module for the one new file, precedent-document for the three doc deliverables).

## Metadata

**Analog search scope:** `crates/nono-cli/src/` (command/runtime modules, `cli.rs`, `app_runtime.rs`,
`cli_bootstrap.rs`, `main.rs`, `exec_strategy.rs`, `pty_proxy.rs`, `profile_runtime.rs`, `timeouts.rs`),
`crates/nono/src/sandbox/linux.rs`, `crates/nono/src/trust/types.rs`, `crates/nono-cli/tests/`,
`.planning/phases/108-upst12-divergence-audit/`, `.planning/phases/109-proxy-network-absorb/`,
`proj/ADR-108-*.md`, `proj/ADR-111-*.md`.
**Files scanned:** ~20 direct reads/greps against the live tree (see inline citations above); no
speculative/unverified excerpts — every code block in this document was read live from the cited
file:line range on 2026-08-05, in the same session as this map.
**Pattern extraction date:** 2026-08-05
**Corrections flagged to the planner (do not silently apply — surface them):**
1. SEC-04's real target file is `crates/nono/src/trust/types.rs`, not `crates/nono/src/trust/bundle.rs`
   as the phase-guidance prompt named.
2. RES-02's real target files are `crates/nono-cli/src/pty_proxy.rs` + `crates/nono-cli/src/timeouts.rs`
   (both Unix-only via `main.rs` mod-cfg), not `crates/nono-cli/src/exec_strategy_windows/{launch,mod}.rs`
   as the phase-guidance prompt named. `pty_proxy_windows.rs` has zero drain/discard/teardown hits —
   RES-02 has no Windows-side work.
