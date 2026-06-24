---
slug: ci-linux-cfg-compile-errors
status: investigating
trigger: "PR #12 (v3.1/v3.2 -> main) CI red: pre-existing Linux cfg-gated compile errors never caught on Windows host"
created: 2026-06-23
updated: 2026-06-23
---

# Debug Session: CI Linux cfg-gated compile errors (PR #12)

## Symptoms

**Expected behavior:** PR #12 CI green — at minimum `Test` and `Clippy` on ubuntu-latest + macos-latest. The v3.1/v3.2 milestone code should compile on Linux.

**Actual behavior:** `Test (ubuntu-latest)` fails to COMPILE nono-cli. 17 of 20 CI checks fail (only Classify Changes, Conventional Commit Title, Verify FFI Header pass). Local Windows `cargo check --workspace` passes — it never compiles the Unix `cfg` branches.

**Error messages (from `Test (ubuntu-latest)`):**
```
error[E0432]: unresolved import `nono::SupervisorListener`
    crates/nono-cli/src/exec_strategy.rs:33
error[E0609]: no field `unix_socket_allowlist` on type `&ExecConfig<'_>`   (x2)
    crates/nono-cli/src/exec_strategy/supervisor_linux.rs:724 (+ one more)
error[E0425]: cannot find function `drain_terminal_output` in this scope   (x2)
    crates/nono-cli/src/pty_proxy.rs:391, 405
```
(Compilation stopped after these — more errors may be masked.)

**Timeline:** First-ever CI run of v3.1/v3.2. Tags were never pushed, so CI never exercised this code. All milestone verification was Windows `cargo check` + native clippy. The cross-target verification was deferred as PARTIAL→CI throughout v3.1/v3.2.

**Reproduction:** Push to PR #12 branch → `Test (ubuntu-latest)` recompiles and fails. Authoritative verification loop:
`gh pr checks 12 --repo OscarMackJr/nono` then `gh run view --repo OscarMackJr/nono --job <id> --log-failed`.

## Constraints

- **Windows host.** Unix code is `#[cfg(target_os="linux")]` / `#[cfg(unix)]` gated; local `cargo check --workspace` does NOT exercise it.
- **Cross-target check BLOCKED locally:** `cargo check --target x86_64-unknown-linux-gnu` dies in `aws-lc-sys` build.rs (needs `x86_64-linux-gnu-gcc`, no cross C compiler on host). Both rust stds ARE installed, but the C dep blocks it. **CI is the only authoritative verifier.**
- **SECURITY-CRITICAL:** `supervisor_linux.rs` and `exec_strategy.rs` are sandbox enforcement. Resolve carefully; when in doubt choose the more restrictive option (CLAUDE.md).
- All commits need DCO sign-off: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
- Branch: `milestone/v2.13-carryforward-closeout`. The `sandbox/linux.rs` merge conflict is ALREADY resolved (bind filter[20]) — do NOT revert it.
- These errors PRE-EXIST on the branch (merge only touched command_runtime.rs, execution_runtime.rs, sandbox/linux.rs — none of the failing files).

## Initial hypotheses to test

1. **Phase 86 library-boundary-convergence** moved types into the core `nono` crate but did not re-export `SupervisorListener` (and possibly `SupervisorSocket`, `UnixSocketCapability`) from `nono`'s public API → E0432.
2. **`unix_socket_allowlist`**: field IS defined at exec_strategy.rs:413 on `ExecConfig`, yet supervisor_linux.rs:724 says no such field. Likely a cfg-gated field, a second `ExecConfig` definition, or a v3.1↔v3.2 name drift between the field decl and its Unix-only use site.
3. **`drain_terminal_output`**: called at pty_proxy.rs:391,405 but no `fn drain_terminal_output` exists anywhere in the tree → lost definition, wrong cfg gate, or rename mismatch (PR #1135 ctrl-z PTY work).

## Out of scope (separate triage, note but do not chase first)
- Windows CI checks all red while local Windows `cargo check --workspace` is green → likely chronic red baseline (memory: nono_cli/nono Windows baseline test failures) + full-build/test differences.
- Clippy/Integration/macos failures — triage after the ubuntu compile errors clear (they may be downstream of the same compile break).

## Current Focus

reasoning_checkpoint:
  hypothesis: "All three E-codes are botched-cherry-pick / wrong-symbol artifacts that Windows cargo check never exercised, each with a single correct intended symbol resolvable from the existing source — none require weakening allow-list/deny logic."
  confirming_evidence:
    - "E0432: `SupervisorListener` type does NOT exist in the nono crate at all; the import line is its only occurrence in nono-cli (unused)."
    - "E0609: field exists on SupervisorConfig; CR-01 commit 718fe59d added reads of it on ExecConfig (wrong struct) without adding the field; the data is reachable via the ExecConfig.caps CapabilitySet (caps.unix_socket_capabilities()), the same source supervised_runtime.rs:367 uses to populate SupervisorConfig."
    - "E0425: cherry-pick 1f4fd335 added the calls but dropped the fn; upstream/main has the authoritative def at pty_proxy.rs:1785."
  falsification_test: "Push the 3 fixes to PR #12; if Test(ubuntu) still reports any of E0432/E0609/E0425 on these lines, a fix is wrong. If new/masked errors surface, scope was larger than 3."
  fix_rationale: "Each fix restores the symbol the code INTENDED: drop a dead import; read the same allowlist from the same CapabilitySet already on ExecConfig; restore the missing fn verbatim from upstream. The E0609 fix reads caps.unix_socket_capabilities() (identical bytes to the SupervisorConfig field) so the child(1338)/parent(1642) send-action predicate stays in lockstep — no allow-list narrowed or widened."
  blind_spots: "Compilation stopped at these 3; more errors may be masked behind them (CI is the only way to know). The fork lacks upstream's 2 extra drain_terminal_output call sites (1753/1758) — not restoring those is intentional but could mean a residual ctrl-z drain gap (functional, not a compile blocker)."
- next_action: CHECKPOINT to operator with the 3-fix batch (security-critical files). On confirm: apply edits, commit w/ DCO on milestone/v2.13-carryforward-closeout, push, watch `gh pr checks 12`.

## Resolution

root_cause: |
  Three independent Unix-cfg compile errors, all invisible to Windows `cargo check`:
  1. E0432 exec_strategy.rs:33 — dead import of nonexistent `nono::SupervisorListener`.
  2. E0609 exec_strategy.rs:1338,1642 — `config.unix_socket_allowlist` read on `ExecConfig` (field lives on `SupervisorConfig`); introduced by CR-01 (718fe59d) which never added the field to ExecConfig.
  3. E0425 pty_proxy.rs:391,405 — `drain_terminal_output` calls cherry-picked (1f4fd335 / #1135) without the function definition (upstream/main pty_proxy.rs:1785).
fix: |
  1. Remove `SupervisorListener,` from the nono import at exec_strategy.rs:33.
  2. Replace `config.unix_socket_allowlist.is_empty()` with `config.caps.unix_socket_capabilities().is_empty()` at exec_strategy.rs:1338 and 1642 (same grant data via ExecConfig.caps; preserves child/parent predicate lockstep).
  3. Restore `fn drain_terminal_output(fd: RawFd)` verbatim from upstream/main above `write_all_fd` in pty_proxy.rs.
verification: "PENDING — CI on PR #12 (Test ubuntu-latest + macos-latest) is sole authoritative verifier."
files_changed:
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/pty_proxy.rs

## Layer 2 (after fix push f0c5e8a4)

The 3 resolution fixes (E0432/E0609/E0425) WORKED — those errors are gone. They had masked **17 dead_code/unused lints** promoted to errors by CI's `-Dwarnings`. **Linux and macOS show the IDENTICAL 17** → all are items used only on Windows (or genuinely dead). Fix rule:
- Symbol used elsewhere ONLY under Windows cfg / in a Windows path → `#[cfg_attr(not(target_os = "windows"), allow(dead_code))]` (matches existing codebase pattern, e.g. ExecConfig.audit_recorder).
- Symbol referenced NOWHERE (cherry-pick leftover etc.) → remove per CLAUDE.md "avoid allow(dead_code); remove if unused".
- Unused import → remove the specific item (cfg-gate the import if used only on Windows).

The 17 (same on linux + macos):
1. unused import `RejectStage` — audit_integrity.rs:1:63
2. unused imports `UnixSocketCapability`,`UnixSocketMode` — exec_strategy.rs:33:48
3. unused import `nono::SandboxViolation` — profile_save_runtime.rs:6:5
4. fn `classify_daemon_request` never used — agent_cli.rs:748 (Windows named-pipe?)
5. fn `is_pipe_not_found` never used — agent_cli.rs:1058 (Windows named-pipe?)
6. fields `sandbox_violations`,`ignored_denial_paths` never read — exec_strategy.rs:181
7. methods `in_alt_screen`,`shutdown_attach_listener` never used — pty_proxy.rs:378 (#1135 cherry-pick leftovers)
8. fn `rollback_root` never used — rollback_session.rs:31
9-11. fns `import_machine_root`/`import_current_user_root`/`is_cert_present_current_user` — cert_trust.rs:207/214/222 (Windows cert store)
12. variants `Ok`,`Broken` never constructed — health.rs:62
13. const `AUDIT_LEDGER_FILENAME` never used — state_paths.rs:15
14. fn `maybe_migrate_legacy_audit_ledger` never used — state_paths.rs:267
15-17. const `EVENT_LOG_SOURCE`/enum `EventLogLevel`/fn `build_event_log_message` — telemetry/windows.rs:35/45/79 (Windows Event Log)

CONSTRAINT: still can't compile linux/macos locally (aws-lc-sys cross C compiler). Windows `cargo check --workspace` is a guard that fixes don't break Windows. CI (Test ubuntu+macos) is the verifier.

## Evidence

- timestamp: 2026-06-23 — `git diff 5370d7ae 0eb9d6a0 --name-only` shows merge touched only command_runtime.rs, execution_runtime.rs, sandbox/linux.rs. Failing files untouched → errors pre-exist on branch.
- timestamp: 2026-06-23 — `grep drain_terminal_output` finds 2 call sites (pty_proxy.rs:391,405), zero definitions.
- timestamp: 2026-06-23 — `grep SupervisorListener` finds 1 use (exec_strategy.rs:33 importing from `nono::`); need to check nono crate exports.
- timestamp: 2026-06-23 — `grep unix_socket_allowlist` shows field decl at exec_strategy.rs:413 (`pub unix_socket_allowlist: &'a [nono::UnixSocketCapability]`) and uses in supervisor_linux.rs:724/1510/1528/1595/1597 + exec_strategy.rs:1338/1417/1642.
- timestamp: 2026-06-23 — cross-target `cargo check --target x86_64-unknown-linux-gnu` blocked at aws-lc-sys (no x86_64-linux-gnu-gcc). CI is sole verifier.

- timestamp: 2026-06-23 — E0432 ROOT CAUSE: `nono` crate exports NO `SupervisorListener` type anywhere (lib.rs:119-122 exports `SupervisorSocket` only; no `SupervisorListener` def in supervisor/mod.rs or the tree). exec_strategy.rs:33 imports it but NEVER USES it (grep: 1 hit, the import line itself). It is a phantom/stale import — `SupervisorSocket`, `UnixSocketCapability`, `UnixSocketMode` on the same line ARE used (10 hits). FIX: drop `SupervisorListener` from the import list. No security logic touched (unused import).
- timestamp: 2026-06-23 — E0609 ROOT CAUSE: `unix_socket_allowlist` is a field on `SupervisorConfig` (exec_strategy.rs:413, inside struct at 378), NOT on `ExecConfig` (struct 290-360). Use at supervisor_linux.rs:724 is CORRECT (`config: &SupervisorConfig`). The ERRORING sites are exec_strategy.rs:1338 and 1642, inside `execute_supervised(config: &ExecConfig, supervisor: Option<&SupervisorConfig>)` where `config` is an `ExecConfig` — alongside `config.seccomp_proxy_fallback`/`config.af_unix_mediation` which DO exist on ExecConfig. git: commit 718fe59d (CR-01) added `config.unix_socket_allowlist` reads at these two sites but never added the field to ExecConfig (the field was added to SupervisorConfig in ffac4e89). Classic cross-target blind-spot — never compiled on Windows. The data originates from `caps.unix_socket_capabilities()` (supervised_runtime.rs:367 populates SupervisorConfig from it) and `ExecConfig` already holds `caps: &CapabilitySet` (line 298). FIX: replace `config.unix_socket_allowlist.is_empty()` -> `config.caps.unix_socket_capabilities().is_empty()` at 1338 + 1642. SAME data, same source, identical child/parent predicate (preserves the 1334-1337 invariant). No weakening.
- timestamp: 2026-06-23 — E0425 ROOT CAUSE: `fn drain_terminal_output` is missing entirely. Calls at pty_proxy.rs:391,405 were introduced by cherry-pick 1f4fd335 (#1135 ctrl-z PTY fix) which added the CALLS but dropped the DEFINITION. `git show upstream/main:crates/nono-cli/src/pty_proxy.rs` HAS the authoritative def at line 1785: `fn drain_terminal_output(fd: RawFd)` — isatty-guards, then loops `libc::tcdrain(fd)` retrying on EINTR, debug-logs other errors. RawFd (import line 24) and debug! (line 29) are in scope. FIX: restore the upstream definition verbatim, placed just above `write_all_fd` (line 1509). NOTE: upstream also has 2 EXTRA call sites (1753,1758) the fork's cherry-pick lacks; out of scope for the compile fix — fork's drain_socket_replay path differs.

## Eliminated

- hypothesis: SupervisorListener was moved into core nono crate by Phase 86 but not re-exported.
  evidence: No `SupervisorListener` type exists anywhere in the nono crate (not just unexported — it does not exist). The import is dead/never-used. So it is a stale import, not a missing re-export.
  timestamp: 2026-06-23
- hypothesis: `unix_socket_allowlist` is missing due to a cfg-gate or a v3.1<->v3.2 name drift on a single ExecConfig.
  evidence: There are TWO distinct structs (ExecConfig + SupervisorConfig). The field exists on SupervisorConfig and is used correctly there. The error is a wrong-struct access added by CR-01 (718fe59d), not a cfg gate or rename.
  timestamp: 2026-06-23
