# Phase 88 PARTIAL→CI Deferral Record

Per CLAUDE.md §"Cross-target clippy verification" MUST/NEVER rule and
`.planning/templates/cross-target-verify-checklist.md`.

Windows-host `cargo clippy` cannot exercise `#[cfg(unix)]` / `nix::` branches.
Cross-target Linux/macOS clippy is SKIPPED on this dev host due to missing
C cross-toolchain (`x86_64-linux-gnu-gcc`, `x86_64-apple-darwin`).

The live GH Actions Linux/macOS Clippy lanes on the head SHA are the decisive
signals per the PARTIAL disposition protocol.

## Plan 88-01 Deferrals

| Commit | File | Reason | CI Gate |
|--------|------|--------|---------|
| 89ba09cf (d48aeb7b) | crates/nono-cli/src/exec_strategy.rs | File contains `#[cfg(target_os = "linux")]` and `#[cfg(target_os = "macos")]` blocks; new `set_vars` code in this file is std-only but the file triggers the CLAUDE.md MUST/NEVER cross-target rule | GH Actions Linux/macOS CI clippy lanes |
| 89ba09cf (d48aeb7b) | crates/nono-cli/src/exec_strategy/env_sanitization.rs | File is under `crates/nono-cli/src/exec_strategy/` (Unix supervisor code directory); `validate_set_vars` and `push_set_vars` are std-only but the directory triggers the MUST/NEVER rule | GH Actions Linux/macOS CI clippy lanes |

## Plan 88-02 Deferrals

| Commit | File | Reason | CI Gate |
|--------|------|--------|---------|
| 0a09ff41 (e8293b36) | crates/nono-cli/src/state_paths.rs | Contains `#[cfg(target_os = "windows")]` and `#[cfg(not(target_os = "windows"))]` blocks (D-02 Windows arm + XDG arm); `resolve_xdg_state_base`, `AUDIT_LEDGER_FILENAME`, and `maybe_migrate_legacy_audit_ledger` are only reachable on Linux/macOS — suppressed with `#[cfg_attr(target_os = "windows", allow(dead_code))]` on Windows host | GH Actions Linux/macOS CI clippy lanes |
| 0a09ff41 (e8293b36) | crates/nono-cli/src/audit_session.rs | File has `#[cfg(unix)]` permission block; new callsite delegation (`state_paths::audit_root()`) is pure Rust but the file triggers the CLAUDE.md MUST/NEVER cross-target rule | GH Actions Linux/macOS CI clippy lanes |
| 0a09ff41 (e8293b36) | crates/nono-cli/src/protected_paths.rs | File has platform-specific `#[cfg]` blocks; `resolve_path` normalization applied to XDG state roots in `from_defaults()` triggers MUST/NEVER rule | GH Actions Linux/macOS CI clippy lanes |
| 74c5ac23 (8e0d94f9) | crates/nono-cli/src/profile/mod.rs | `test_expand_vars_xdg_config_home` and `test_expand_vars_nono_config` gated as `#[cfg(unix)]` (tests use Unix-absolute paths `/home/user`, `/custom/config`); XDG config expansion only verified on Linux/macOS CI | GH Actions Linux/macOS CI clippy lanes |
| de553185 | crates/nono-cli/src/session.rs | `sessions_dir_uses_xdg_state_home` test gated as `#[cfg(not(target_os = "windows"))]`; XDG session dir behavior only verified on Linux/macOS CI | GH Actions Linux/macOS CI clippy lanes |
| de553185 | crates/nono-cli/src/config/mod.rs | `test_user_config_dir_uses_xdg_fallback` gated as `#[cfg(not(target_os = "windows"))]`; XDG config dir fallback behavior only verified on Linux/macOS CI | GH Actions Linux/macOS CI clippy lanes |

Forward-compat note: `wiring.rs` `$NONO_CONFIG`/`$NONO_PACKAGES` variable expansion exists in upstream 8e0d94f9 but references `WiringContext`/`expand_vars` types not yet present in this fork. The upstream tests for this expansion are not included in this cherry-pick. They will be absorbed when the wiring refactor is synced in a future phase.

## Plan 88-03 Deferrals

| Commit | File | Reason | CI Gate |
|--------|------|--------|---------|
| 5eab6d46 | crates/nono-cli/src/profile/mod.rs | `validate_aws_auth()` uses `#[cfg_attr]`-compatible code but profile/mod.rs has `#[cfg(unix)]` test blocks; aws_auth field test coverage verified on Windows host, Unix-only tests deferred | GH Actions Linux/macOS CI clippy lanes |
| c0ea3af7 | crates/nono-cli/src/hook_runtime.rs | File contains `#[cfg(unix)]` blocks (execute_before_hook, execute_after_hook); source_pack: None additions are test-only but the file triggers MUST/NEVER cross-target rule | GH Actions Linux/macOS CI clippy lanes |
| c0ea3af7 | crates/nono-cli/src/profile/mod.rs | resolve_store_pack_session_hooks() and call sites are conditional on pack-store subsystem (Unix-primary); function body is std-only Rust but the file's existing cfg-gated tests trigger MUST/NEVER rule | GH Actions Linux/macOS CI clippy lanes |
| c0ea3af7 | crates/nono-cli/src/profile_runtime.rs | verify_profile_packs() session-hook containment check uses strip_prefix on paths; path handling is std-only but verify_profile_packs test coverage is incomplete on Windows (path separator differences) | GH Actions Linux/macOS CI clippy lanes |

## Plan 88-04 Deferrals

| Commit | File | Reason | CI Gate |
|--------|------|--------|---------|
| 1f4fd335 (4179ce03) | crates/nono-cli/src/exec_strategy.rs | New `signal_pty_foreground_group()` and `handle_pty_suspension()` functions reference `nix::unistd::tcgetpgrp`, `Signal::SIGTSTP/SIGSTOP/SIGWINCH`, `WaitStatus`, `WaitPidFlag` — all `nix::` symbols. Module is gated `#[cfg(not(target_os = "windows"))]` at module level in main.rs so functions are only compiled on Unix; file has many `#[cfg(target_os = "linux")]`/`#[cfg(target_os = "macos")]` blocks triggering CLAUDE.md MUST/NEVER cross-target rule | GH Actions Linux/macOS CI clippy lanes |
| 1f4fd335 (4179ce03) | crates/nono-cli/src/pty_proxy.rs | Unix-only module (`#[cfg(not(target_os = "windows"))]` gate in main.rs); additions (`in_alt_screen()`, `leave_screen_for_suspension()`, `reenter_screen_for_resume()`, `take_suspension_request()`, `shutdown_attach_listener()`) are Unix-path by module-level gating; `nix::sys::termios` references only verified on Linux/macOS CI | GH Actions Linux/macOS CI clippy lanes |

## Plan 88-05 Deferrals

| Commit | File | Reason | CI Gate |
|--------|------|--------|---------|
| 76e1e40 (e54cf9cb) | crates/nono-cli/src/hook_runtime.rs | File contains `#[cfg(unix)]` pre_exec block (execute_before_hook, execute_after_hook); env_clear removal is within `build_hook_command()` which is called from the Unix exec path; Windows-host clippy cannot verify `#[cfg(unix)]` arms or validate the path the env is now inherited (not cleared). hook_runtime_windows.rs retains env_clear() + CLR baseline per D-14. | GH Actions Linux/macOS CI clippy lanes |

## Status

PARTIAL — pending GH Actions confirmation on the head SHA.

Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain
(x86\_64-unknown-linux-gnu, x86\_64-apple-darwin). The live GH Actions Linux
Clippy and macOS Clippy lanes on the head SHA are the decisive signals per
.planning/templates/cross-target-verify-checklist.md. REQs FEAT-01, FEAT-02, FEAT-04,
FEAT-03, FEAT-05, FEAT-06c, and DEPS-01 marked PARTIAL pending CI confirmation.
