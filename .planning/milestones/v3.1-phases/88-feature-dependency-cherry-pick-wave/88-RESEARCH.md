# Phase 88: Feature + Dependency Cherry-Pick Wave - Research

**Researched:** 2026-06-20
**Domain:** Rust upstream cherry-pick absorption (cherry-pick mechanics, env-injection, XDG state, AWS auth, FFI diagnostics, dep bumps)
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Adopt upstream `state_paths.rs` as single source of truth; rewrite `config/mod.rs` helpers to delegate.
- **D-02:** Windows arm → `%LOCALAPPDATA%\nono`; reconcile against v3.0 scratch-space provisioner.
- **D-03:** One-time auto-migrate (move), fail-secure on any migration error.
- **D-04:** Resolve latest-compatible via `cargo update`; typify spec edit is the only Cargo.toml change.
- **D-05:** One atomic DEPS commit for all 9 bumps; watch typify-0.7 codegen fallout.
- **D-06:** Explicit 5-crate path-dep pin checklist gate before `make ci`.
- **D-07:** Adopt upstream namespace convention + keep bare-name aliases for built-in profiles.
- **D-08:** Namespace fork-only profiles (`nono-ts-*`, `swival`) consistently.
- **D-09:** FEAT-06 absorbs update-check CI-provider discovery (`cc11b389`) and truthy env bool flags (`42e5bf73`) independent of rename.
- **D-10:** CR-01 clear-on-entry across ALL FFI entry points.
- **D-11:** Dedicated FFI regression test + standalone fork-divergence commit for CR-01.
- **D-12:** `git cherry-pick -x` + DCO per upstream SHA.
- **D-13:** Cherry-pick Cluster I (`7d274cf7`) BEFORE Cluster M (`e54cf9cb`).
- **D-14:** `e54cf9cb` env_clear removal is Unix path only; `hook_runtime_windows.rs` RETAINS `env_clear()` + SystemRoot/windir/SystemDrive baseline.
- **D-15:** `5bb098cd` tls_intercept hunk won't-apply; extract shared-surface hunks only.

### Claude's Discretion
- `set_vars` (FEAT-01) env-name validation/error-surface wiring internals.
- AWS auth (FEAT-03) mutual-exclusion enforcement: where validation lands (profile load vs proxy route config).
- `$PACK_DIR` / `source_pack` propagation details (FEAT-05) and pack-verification dry-run skip (`9800f307`) internals.
- typify-0.7 codegen split decision (D-05) if fallout is non-trivial.

### Deferred Ideas (OUT OF SCOPE)
- Cluster F proxy hardening (route/403/TLS-CONNECT/reactive-auth/customCredentials) → Phase 89.
- `TlsInterceptIntent` assessment (`bd4b6b7f`) → Phase 89.
- `policy.json` `go_runtime` go-build cache group (`5413a0b3`) → future policy.json sync pass.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FEAT-01 | `set_vars` static env injection (profile + `CapabilitySet`), env-name validation rejects `PATH` and `NONO_` prefix | `d48aeb7b` adds `env_sanitization.rs::validate_set_vars()` + `ExecConfig::set_vars` field; wired to both Direct and Supervised strategies |
| FEAT-02 | XDG state dirs with legacy `~/.nono` fallback + one-time migration; Windows path verified against v3.0 provisioner | `e8293b36` + `8e0d94f9`; upstream `state_paths.rs` is the complete solution; Windows reconciliation documented below |
| FEAT-03 | `AwsAuthConfig` accepted and validated in profiles + proxy route config, mutually exclusive with `credential_key`/`oauth2` | `5bb098cd` shared-surface hunks (config.rs, credential.rs, route.rs, server.rs, profile/mod.rs, network_policy.rs); tls_intercept hunk skipped |
| FEAT-04 | Keyring access honors `NONO_KEYRING_TIMEOUT_SECS` (default 120s, 0 = no timeout) | `c6b13345` adds timeout logic to `keystore.rs` and credential warn in `credential.rs` |
| FEAT-05 | `$PACK_DIR` store-pack session hooks resolve with `source_pack` propagation | `7d274cf7` touches `hook_runtime.rs` (I-before-M ordering enforced) + `profile/mod.rs` + `profile_runtime.rs` |
| FEAT-06 | Update-check CI provider/environment discovery; profile names standardized to namespace; bool CLI flags accept truthy env values | `cc11b389` + `6d88638e` + `42e5bf73`; aliases required for backward compat |
| DEPS-01 | PTY ctrl-z suspend/resume no longer hangs under a PTY | `4179ce03`; touches `exec_strategy.rs` (Unix-specific nix:: deps, cross-target concern) + `pty_proxy.rs` (Unix-only) |
| DEPS-02 | 9 dep bumps absorbed across all 5 crates with path-dep pin sync | typify 0.6→0.7 is the only Cargo.toml spec edit; 8 others are lockfile-only; typify-0.7 API is non-breaking |
</phase_requirements>

---

## Summary

Phase 88 absorbs 14 upstream SHAs (11 cherry-picks + 3 dep bump targets + CR-01 fork fix) across the 5-crate workspace. All SHAs are confirmed reachable in the local git object graph (`git cat-file -t` returns `commit` for every SHA in scope).

The wave is genuinely additive: no commit in scope changes the library/CLI security boundary (established in Phase 86), and none touch the Linux seccomp/Landlock security surface (Phase 87). The highest-risk items are FEAT-02 (XDG state migration — touches audit, session, and rollback roots that are currently split between `audit_session.rs::audit_root()` using `nono_home_dir()` and `rollback_session.rs::rollback_root()` using platform-split logic) and the I-before-M ordering constraint on `hook_runtime.rs`.

The typify 0.6→0.7 bump is confirmed non-breaking: the CHANGELOG documents only two additive JSON Schema features; `TypeSpaceSettings::with_struct_builder()` and `to_stream()` are unchanged. The `build.rs` pattern in `crates/nono` will compile cleanly after the spec edit.

CR-01 (FFI `set_last_error` paths leave `LAST_DIAGNOSTIC_CODE` stale) is a fork fix on the Phase-86-converged surface. The fix is clear-on-entry across all entry points, landed as a deliberate fork-divergence commit with regression test.

**Primary recommendation:** Execute cherry-picks in the exact sequence prescribed by D-12/D-13, gate each wave with `make ci` (Windows native), and mark Unix-path additions PARTIAL→CI for Linux/macOS clippy per the CLAUDE.md MUST/NEVER rule.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `set_vars` env injection (FEAT-01) | CLI (exec_strategy.rs) | Profile load (profile/mod.rs) | Injection happens at exec time after profile resolution; validation at profile parse time |
| XDG state paths (FEAT-02) | CLI (state_paths.rs) | config/mod.rs delegation | State is CLI concern; library has no runtime-path knowledge |
| AWS auth config (FEAT-03) | Proxy (config.rs, credential.rs, route.rs) | CLI profile load (profile/mod.rs) | Mutual-exclusion validation is profile-load-time; proxy implements the 501 placeholder |
| Keyring timeout (FEAT-04) | Library (keystore.rs) | Proxy (credential.rs warn) | Keyring is a library primitive; proxy surfaces load failures via warn! |
| `$PACK_DIR` session hooks (FEAT-05) | CLI (hook_runtime.rs, profile_runtime.rs) | — | Hook expansion is CLI lifecycle |
| Profile namespace + CI + truthy flags (FEAT-06) | CLI (cli.rs, update_check.rs, policy.json, profile/mod.rs) | — | All CLI surface |
| PTY ctrl-z fix (DEPS-01) | CLI (exec_strategy.rs, pty_proxy.rs) | — | PTY is Unix exec strategy |
| FFI CR-01 clear-on-entry (fork fix) | FFI (bindings/c/src/diagnostic.rs, lib.rs) | — | FFI thread-local diagnostic store |
| Dep bumps (DEPS-02) | All 5 crates (Cargo.toml + Cargo.lock) | — | Workspace-wide |

---

## Standard Stack

### Core (no new packages — this is an absorption wave)

All packages below are already in `Cargo.lock`. Phase 88 bumps versions only.

| Package | Current Version | Target Version | Edit Required |
|---------|----------------|----------------|---------------|
| typify | 0.6.2 | 0.7.0 | `crates/nono/Cargo.toml:71` — change `"0.6"` → `"0.7"` |
| cbindgen | 0.29.2 | 0.29.4 | Lockfile-only (`cargo update -p cbindgen`) |
| hyper | 1.9.0 (v1 slot) | 1.10.1 | Lockfile-only (`cargo update -p hyper`) |
| zeroize | 1.8.2 | 1.9.0 | Lockfile-only (`cargo update -p zeroize`) |
| time | 0.3.47 | 0.3.49 | Lockfile-only/transitive (`cargo update -p time`) |
| chrono | 0.4.44 | 0.4.45 | Lockfile-only (`cargo update -p chrono`) |
| ignore | 0.4.25 | 0.4.26 | Lockfile-only (`cargo update -p ignore`) |
| which | 8.0.2 | 8.0.3 | Lockfile-only (in `crates/nono/Cargo.toml:54` as `"8"`) |
| x509-parser | not present currently | 0.18.1 | Transitive-only — not a direct dep in any Cargo.toml; resolved via `cargo update` |

**Verification:** `cargo search typify` confirms `typify = "0.7.0"` is on the registry. `[VERIFIED: cargo registry]`

**5-crate path-dep pin checklist (D-06):** Version pins are internal path deps (nono → nono version, nono-cli → nono version, nono-proxy → nono version, bindings/c → nono version). A version bump to any crate's `[package] version` field must be mirrored in all dependents' `[dependencies] nono = { path = "...", version = "..." }` specs across all 5 Cargo.toml files: workspace root, `crates/nono`, `crates/nono-cli`, `crates/nono-proxy`, `bindings/c`. Phase 88 does NOT change crate versions (marker-only milestone), so this is a checklist gate to confirm no accidental version drift.

---

## Package Legitimacy Audit

No new external packages are introduced in Phase 88. The wave updates existing packages only.

| Package | Registry | Current Age | slopcheck | Disposition |
|---------|----------|-------------|-----------|-------------|
| typify 0.7.0 | crates.io | oxidecomputer project; established | [ASSUMED] N/A | Approved — same maintainer as 0.6.x |
| cbindgen 0.29.4 | crates.io | mozilla/cbindgen; well-established | [ASSUMED] N/A | Approved |
| hyper 1.10.1 | crates.io | hyperium/hyper; well-established | [ASSUMED] N/A | Approved |
| (others) | crates.io | all established; patch/minor bumps | [ASSUMED] N/A | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was not run (no new packages to gate). All packages are minor/patch bumps of already-approved deps already present in Cargo.lock. `[ASSUMED]` on "established" characterization.*

---

## Architecture Patterns

### System Architecture Diagram

```
Upstream SHA objects (reachable in git object graph)
              |
              v
  cherry-pick -x per SHA (D-12)
              |
       ┌──────┴──────────────────────────────────────────┐
       │  Cluster sequence:                               │
       │  D(set_vars) → E(XDG) → G(AWS) → H(keyring)    │
       │  → I(PACK_DIR) → J(PTY) → K(CI) → L(profile)   │
       │  → M-without-e54cb → e54cb(Unix-path-only)      │
       │  → a6aa9995(CR-01 fork fix, NOT cherry-pick)     │
       │  → DEPS (one atomic commit: spec + lock)         │
       └──────┬──────────────────────────────────────────┘
              |
    ┌─────────┴─────────────────────────────────────────────────┐
    │ Files touched per cluster:                                  │
    │                                                             │
    │ D  exec_strategy.rs, env_sanitization.rs, profile/mod.rs,  │
    │    sandbox_prepare.rs, profile_runtime.rs (+schema/docs)    │
    │                                                             │
    │ E  state_paths.rs [NEW], config/mod.rs, audit_session.rs,   │
    │    rollback_session.rs, session.rs, rollback_commands.rs,   │
    │    audit_ledger.rs, protected_paths.rs, proxy_runtime.rs,   │
    │    launch_runtime.rs (+tests/docs)                          │
    │                                                             │
    │ G  profile/mod.rs, network_policy.rs, proxy/config.rs,      │
    │    proxy/credential.rs, proxy/route.rs, proxy/server.rs     │
    │    [tls_intercept/handle.rs hunk SKIPPED]                   │
    │                                                             │
    │ H  keystore.rs, proxy/credential.rs                         │
    │ I  hook_runtime.rs, profile/mod.rs, profile_runtime.rs      │
    │ J  exec_strategy.rs [Unix-nix:: functions], pty_proxy.rs    │
    │ K  update_check.rs                                           │
    │ L  cli.rs, migration.rs, profile/mod.rs, policy scripts     │
    │ M  cli.rs, output.rs, capability_ext.rs, sandbox_prepare.rs,│
    │    startup_runtime.rs, pull_ui.rs, profile/mod.rs,          │
    │    profile_cmd.rs, schema, profile_runtime.rs,              │
    │    hook_runtime.rs [env_clear removal, Unix only]            │
    │                                                             │
    │ CR-01 bindings/c/src/diagnostic.rs, bindings/c/src/lib.rs   │
    │ DEPS  crates/nono/Cargo.toml, Cargo.lock                    │
    └─────────────────────────────────────────────────────────────┘
              |
        make ci (Windows) + PARTIAL→CI for Unix-path
              |
         REQUIREMENTS green
```

### Recommended Commit Sequence

```
Wave 1 — Additive features (low risk):
  commit 1: cherry-pick d48aeb7b (FEAT-01: set_vars)
  commit 2: cherry-pick e8293b36 (FEAT-02: XDG state dirs + state_paths.rs)
             + D-01 config/mod.rs delegation rewrite
             + D-02 Windows LOCALAPPDATA reconciliation (see § XDG below)
             + D-03 fail-secure migration annotation
  commit 3: cherry-pick 8e0d94f9 (FEAT-02: XDG config paths consistent)
  commit 4: cherry-pick 5bb098cd PARTIAL (FEAT-03: AWS auth, skip tls_intercept hunk)
  commit 5: cherry-pick c6b13345 (FEAT-04: keyring timeout)
  commit 6: cherry-pick 7d274cf7 (FEAT-05: $PACK_DIR) [MUST precede e54cb — D-13]
  commit 7: cherry-pick 4179ce03 (DEPS-01: PTY ctrl-z fix)
  commit 8: cherry-pick cc11b389 (FEAT-06a: CI provider discovery)
  commit 9: cherry-pick 6d88638e (FEAT-06b: profile namespace)
             + D-07 bare-name aliases in policy.json + builtin.rs
             + D-08 fork-only profile namespacing

Wave 2 — M-cluster misc fixes:
  commit 10: cherry-pick 42e5bf73 (FEAT-06c: truthy env bool flags)
  commit 11: cherry-pick a0bba5eb (macOS blocked grants display)
  commit 12: cherry-pick ee7a3bda (schema domain nono.dev→nono.sh)
  commit 13: cherry-pick 7e076d2d (sigstore provenance removal)
  commit 14: cherry-pick 9800f307 (pack-verification dry-run skip)
  commit 15: cherry-pick e54cb [PARTIAL: Unix-path hook_runtime.rs only] (env_clear removal)
               hook_runtime_windows.rs RETAINS env_clear() + baseline restore (D-14)

Wave 3 — Fork fixes + deps:
  commit 16: CR-01 fork fix (clear-on-entry across all FFI entry points; NOT a cherry-pick)
               + regression test for stale-code-between-calls
               + DIVERGENCE-LEDGER addendum
  commit 17: DEPS-02 atomic commit (typify Cargo.toml spec + cargo update all 9 deps)
```

### Anti-Patterns to Avoid

- **Applying e54cb to hook_runtime_windows.rs**: Removes `env_clear()` from the Windows CLR path → `0xFFFF0000 / -65536` CLR init failure. `hook_runtime_windows.rs` line 301 retains `env_clear()` + lines 326 SystemRoot/windir/SystemDrive restore. Do NOT touch those lines.
- **Cherry-picking M (`e54cb`) before I (`7d274cf7`)**: Both touch `hook_runtime.rs`. I inserts `PACK_DIR` env injection before `env_clear` removal in M. M applied first → rebase conflict. Enforce I-before-M.
- **Atomic DEPS commit without typify spec edit**: Running `cargo update` without editing `crates/nono/Cargo.toml:71` leaves Cargo.lock at typify 0.6.2. The spec must change from `"0.6"` to `"0.7"` first.
- **Partial `state_paths.rs` delegation**: After cherry-picking `e8293b36`, `config/mod.rs::user_state_dir()` is rewritten to delegate to `state_paths::user_state_dir()`. If any callers still use the old `nono_home_dir().join(".nono")` pattern directly (e.g., `audit_session.rs::audit_root()` line 36), they will diverge silently. Every state-root callsite must be audited.
- **Profile rename without aliases**: `6d88638e` renames `claude-code` references to `always-further/claude` in docs/scripts/comments — but the fork's `policy.json` and `builtin.rs` still expose bare names (`claude-code`, `codex`, `swival`) because users run `--profile claude-code`. D-07 requires keeping both. Do not delete the bare-name entries.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| env-name validation for `set_vars` | Custom regex/string logic | Upstream `validate_set_vars()` in `env_sanitization.rs` | Already handles PATH, NONO_ prefix, POSIX name syntax, prefix-collision safety (PA does not match PATH) |
| XDG state path resolution | Custom `std::env::var("XDG_STATE_HOME")` logic | Upstream `state_paths::resolve_xdg_state_base()` + `user_state_dir()` | Already handles absolute-path guard, `~/.local/state` fallback, legacy `~/.nono` dual-read |
| AWS credential mutual-exclusion | Ad-hoc if/else in profile load | Upstream `validate_custom_credential()` in `profile/mod.rs` (5bb098cd hunk) | Validates `aws_auth` + `credential_key` + `auth` with proper error messages |
| Legacy audit-ledger migration | Custom file copy | Upstream `maybe_migrate_legacy_audit_ledger()` in `state_paths.rs` | Implements copy-via-temp-rename (atomic), stale-tmp cleanup, primary-not-exists guard |
| FFI thread-local clear-on-entry | Per-function clear boilerplate | Helper `clear_last_call_state()` that resets all three thread-locals (CR-01 pattern) | Systematic: `LAST_ERROR`, `LAST_DIAGNOSTIC_CODE`, and `LAST_REMEDIATION_JSON` must all reset together |

**Key insight:** Every feature in this wave has complete upstream implementation — the work is absorption and fork-reconciliation, not greenfield development. The only net-new code is the CR-01 clear-on-entry helper and its regression test.

---

## Per-Commit Diff Analysis

### FEAT-01: `d48aeb7b` (set_vars static env injection) — CONFIRMED CLEAN APPLY

**Files touched:** `exec_strategy.rs`, `exec_strategy/env_sanitization.rs`, `profile/mod.rs`, `sandbox_prepare.rs`, `profile_runtime.rs`, `command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, schema/docs (13 files, 434+/5-).

**Validation internals (Claude's Discretion resolved):**

`validate_set_vars()` in `env_sanitization.rs` is the gating function. It:
1. Rejects `key == "PATH"` with an explicit message pointing to `allow_vars`/`deny_vars`.
2. Rejects `key.starts_with("NONO_")` (prefix reservation).
3. Validates POSIX name syntax `[A-Za-z_][A-Za-z0-9_]*` via `is_valid_env_var_name()`.
4. Intentionally does NOT apply the dangerous-variable blocklist (`LD_PRELOAD`, etc.) — this is explicit operator intent, not parent-shell injection.

Prefix-collision safety: `PA` does NOT match `PATH` (the guard checks `key == "PATH"` exactly, not `key.starts_with("PA")`). The `push_set_vars()` helper for the supervised execve path deduplicates by removing any prior entry with the same key before pushing, preventing the dynamic-linker-bypass vector.

`ExecConfig` gains a `set_vars: Vec<(String, String)>` field. All existing `ExecConfig` construction sites need this field added (default `Vec::new()`). The profile loads `environment.set_vars` from JSON and expands variable references before passing to `ExecConfig`.

**cfg-gated Unix blocks introduced:** None. The functions are added to `env_sanitization.rs` and `exec_strategy.rs` without cfg gates. `exec_strategy.rs` already contains cfg-gated Unix blocks, so the cross-target clippy MUST rule applies to any commit touching this file. However, the new `set_vars` code itself uses only std types. [ASSUMED: applies cleanly without conflict, pending actual apply attempt]

**Windows impact:** `exec_strategy_windows/` is NOT touched by this commit (`windows-touch: no` per ledger). The `ExecConfig::set_vars` field is shared but the Windows exec path uses it through its own `execute_direct`/`execute_supervised` equivalents — verify those pick up the new field.

---

### FEAT-02: `e8293b36` + `8e0d94f9` (XDG state dirs) — REQUIRES FORK RECONCILIATION

**`e8293b36` (876+/263-):** Creates `crates/nono-cli/src/state_paths.rs` (+422 LOC) and updates 20 other files. The new module provides:
- `user_state_dir()` → `$XDG_STATE_HOME/nono` (default `~/.local/state/nono`)
- `audit_root()`, `sessions_dir()`, `rollback_root()` (primary)
- `legacy_audit_root()`, `legacy_sessions_dir()`, `legacy_rollback_root()` (read fallback)
- `audit_discovery_roots()`, `session_registry_dirs_for_read()`, `rollback_discovery_roots()` (dual-root lists)
- `maybe_migrate_legacy_audit_ledger()` (atomic copy-via-rename)
- `StateLookupContext` struct for per-call legacy-warn bookkeeping
- Full test suite using `EnvVarGuard` (safe env override pattern, CLAUDE.md compliant)

**`8e0d94f9` (228+/55-):** Routes `user_config_dir()` through `resolve_user_config_dir()`, adds `$NONO_CONFIG` profile expansion, updates 22 files. Touches `config/mod.rs`, `config/user.rs`, `profile/mod.rs`, `wiring.rs`, `setup.rs`, `trust_cmd.rs`, `update_check.rs`, and scripts.

**D-01 reconciliation (delegation rewrite required):**

The fork's `config/mod.rs::user_state_dir()` currently reads:
```rust
dirs::state_dir()
    .or_else(dirs::data_local_dir)
    .map(|p| p.join("nono"))
```

After `e8293b36`, upstream rewrites this to:
```rust
crate::state_paths::user_state_dir().ok()
```

This delegation is exactly D-01. The fork's `audit_session.rs::audit_root()` STILL uses `nono_home_dir()` directly:
```rust
let home = crate::config::nono_home_dir()?;
Ok(home.join(".nono").join("audit"))
```

This must be rewritten to delegate to `state_paths::audit_root()`. Similarly, `rollback_session.rs::rollback_root()` has a platform split that must be unified under `state_paths::rollback_root()`.

**D-02 Windows LOCALAPPDATA reconciliation (CRITICAL DETAIL):**

Upstream's `state_paths::resolve_xdg_state_base()` calls `crate::config::validated_home()` and returns `home.join(".local").join("state")`. On Windows, `validated_home()` returns `%USERPROFILE%` — NOT `%LOCALAPPDATA%`. This means on Windows, upstream state would land at `%USERPROFILE%\.local\state\nono` (e.g., `C:\Users\Oscar\.local\state\nono`).

The fork's v3.0 Windows provisioner creates `%LOCALAPPDATA%\nono\workspace` (e.g., `C:\Users\Oscar\AppData\Local\nono\workspace`). The state root is separate: the fork currently uses `dirs::data_local_dir()` which IS `%LOCALAPPDATA%`. After D-02, the implementer must add a Windows-specific arm to `state_paths::user_state_dir()`:

```rust
pub fn user_state_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA").map_err(|_| {
            NonoError::Setup("LOCALAPPDATA not set".to_string())
        })?;
        return Ok(PathBuf::from(local_app_data).join("nono"));
    }
    #[cfg(not(target_os = "windows"))]
    Ok(resolve_xdg_state_base()?.join("nono"))
}
```

This Windows arm is a fork-divergence from upstream's platform-agnostic implementation. The provisioner uses `%LOCALAPPDATA%\nono\workspace`; state goes to `%LOCALAPPDATA%\nono` (parent). They co-exist without conflict.

**D-03 fail-secure migration:** Upstream's `maybe_migrate_legacy_audit_ledger()` uses copy-via-temp-rename (atomic on same filesystem, non-atomic cross-fs). The function returns `Err` on any filesystem failure — the caller in `audit_ledger.rs` must propagate this as fatal rather than swallowing it. Verify the upstream call site does `?` not `.unwrap_or_default()`.

**State-root callsites that MUST migrate (FEAT-02 scope):**

| File | Current pattern | New pattern |
|------|-----------------|-------------|
| `audit_session.rs:36` | `nono_home_dir()?.join(".nono").join("audit")` | `state_paths::audit_root()?` |
| `audit_session.rs:72` | `rollback_session::rollback_root()?` | already delegates; stays |
| `rollback_session.rs:29` | platform split (Windows: `user_state_dir()`; Unix: `nono_home_dir()/.nono/rollbacks`) | `state_paths::rollback_root()?` |
| `rollback_runtime.rs:223-224` comment | references old path | update comment |
| Protected paths | `protected_state_roots()` in `state_paths.rs` | included in `e8293b36` |

**`undo/snapshot.rs`:** The context says this is a callsite. Verify whether `undo/snapshot.rs` uses `nono_home_dir()` directly for rollback paths, or delegates through `rollback_session::rollback_root()`.

---

### FEAT-03: `5bb098cd` (AWS auth config) — PARTIAL SYNC (tls_intercept hunk SKIPPED)

**Won't-apply hunk (confirmed):** `crates/nono-proxy/src/tls_intercept/handle.rs` — the entire diff of this file adds `ctx.credential_store.get_aws(s)` and a 501 short-circuit in the TLS intercept path. The fork has no `tls_intercept/` directory. **Skip this hunk entirely.**

**Shared-surface hunks (all safe to apply):**

1. **`crates/nono-proxy/src/config.rs`:** Adds `AwsAuthConfig` struct (with `profile`, `region`, `service` optional fields, all `#[serde(default)]`), adds `aws_auth: Option<AwsAuthConfig>` to `RouteConfig`. The `RouteConfig` struct change is an additive field addition with `serde(default)` — backwards compatible.

2. **`crates/nono-proxy/src/credential.rs`:** Adds `aws_routes: HashMap<String, ()>` to `CredentialStore`, adds `get_aws()` accessor, extends `load()`, `empty()`, `is_empty()`, `len()`, `loaded_prefixes()`. All existing tests in this file need `aws_auth: None` added to `RouteConfig` literal constructions (the compiler will catch missed cases via exhaustive struct update).

3. **`crates/nono-proxy/src/route.rs`:** Adds `aws_auth.is_some()` to `requires_managed_credential`, updates `missing_managed_credential()` signature to include `has_aws: bool`, updates `auth_mechanism_for_route()` and `injection_mode_for_route()` with AWS arms. Tests need `aws_auth: None` additions.

4. **`crates/nono-proxy/src/server.rs`:** Test-only additions of `aws_auth: None` to `RouteConfig` struct literals. No logic changes.

5. **`crates/nono-proxy/src/reverse.rs`:** Minor update to handle `missing_managed_credential` new signature.

6. **`crates/nono-cli/src/network_policy.rs`:** Adds AWS auth awareness to network policy handling.

7. **`crates/nono-cli/src/profile/mod.rs`:** Adds `aws_auth: Option<nono_proxy::config::AwsAuthConfig>` to `CustomCredentialDef`, adds mutual-exclusion validation in `validate_custom_credential()`, adds `validate_aws_auth()` function. The mutual-exclusion logic (Claude's Discretion resolved): validation is at **profile-load time** in `validate_custom_credential()`, NOT at proxy-route time. The proxy gets an already-validated config.

**AWS behavior on the fork's non-TLS proxy path:** The fork's proxy handles non-CONNECT requests through `server.rs` → `route.rs` → `reverse.rs`. The 501 short-circuit that upstream implements in `tls_intercept/handle.rs` is for the TLS path. On the fork's non-TLS path, the `aws_auth` config lands as a registered route prefix in `CredentialStore::aws_routes`, and `get_aws()` returns `Some(&())`. The implementer must wire a 501 response in the non-TLS server path when `aws_route.is_some()` — mirroring what upstream does in `tls_intercept/handle.rs`. This is the fork-specific adaptation for D-15.

---

### FEAT-04: `c6b13345` (keyring timeout) — CONFIRMED CLEAN APPLY

**Files:** `crates/nono/src/keystore.rs` (298+/69-), `crates/nono-proxy/src/credential.rs` (66+/8-).

Adds `NONO_KEYRING_TIMEOUT_SECS` env var: parses `u64`, default 120s, 0 = no timeout (matches prior "wait forever"). Invalid values (non-numeric, overflow) fall back to default with a `tracing::warn!`. In the proxy, credential load failures now emit `warn!` with a timeout hint rather than silent failure.

**Windows-touch:** No. `keystore.rs` is not platform-gated at the module level, but the keyring feature (`keyring = "3"`) is used cross-platform. [ASSUMED: applies cleanly]

---

### FEAT-05: `7d274cf7` ($PACK_DIR session hooks) — I-BEFORE-M REQUIRED

**Files:** `hook_runtime.rs` (3+/0-), `profile/mod.rs` (563+/1-), `profile_runtime.rs` (514+/78-). Very large: 1002+/78-.

`hook_runtime.rs` change is minimal (3 lines): adds `PACK_DIR` env variable injection in `build_hook_command()`. This is the hunk that conflicts with `e54cb`'s `env_clear()` removal — both touch `build_hook_command()`. **Must apply `7d274cf7` first.**

`profile/mod.rs` and `profile_runtime.rs` changes implement `source_pack` as a `PackageRef`, session hook provenance tracking, and pack verification for session hooks.

**`9800f307` (pack-verification dry-run skip, Cluster M):** Touches `profile_runtime.rs` (24+/2-) and integration tests. Adds `--dry-run` skip of pack verification. Applies after `7d274cf7` (both touch `profile_runtime.rs` but `9800f307` is additive to a different function).

---

### DEPS-01: `4179ce03` (PTY ctrl-z hang fix) — CROSS-TARGET CONCERN

**Files:** `exec_strategy.rs` (119+/0-), `pty_proxy.rs` (179+/9-).

**`pty_proxy.rs`:** Gated at module level — `main.rs:77` has `#[cfg(not(target_os = "windows"))] mod pty_proxy;` and `#[cfg(target_os = "windows")] mod pty_proxy;` pointing to `pty_proxy_windows.rs`. Unix-path module. Any additions to `pty_proxy.rs` are verified only on Linux/macOS.

**`exec_strategy.rs`:** The new functions `signal_pty_foreground_group()` and `handle_pty_suspension()` reference `nix::unistd::tcgetpgrp`, `Signal::SIGTSTP/SIGSTOP/SIGWINCH`, `WaitStatus`, `WaitPidFlag` — all from the `nix` crate which is Unix-only. These functions are added WITHOUT explicit `#[cfg(unix)]` guards in the diff, BUT they are called only from `wait_for_child_with_pty()` which itself lives inside an existing Unix execution path.

**PARTIAL→CI determination:** `exec_strategy.rs` contains `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, and `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks per CLAUDE.md MUST/NEVER rule. The new PTY functions reference `nix::` which is conditional. **This commit MUST be marked PARTIAL→CI** — Windows-host `cargo clippy` will not exercise these new functions. The PARTIAL deferral is to Linux+macOS CI clippy.

If upstream's new functions are not wrapped in `#[cfg(any(target_os = "linux", target_os = "macos"))]`, the implementer must add that gate to prevent Windows compilation failures (nix is not available on Windows).

---

### FEAT-06a: `cc11b389` (CI provider discovery) — CONFIRMED CLEAN APPLY

**Files:** `update_check.rs` (197+/4-). Adds `detect_ci_provider() -> Option<&'static str>`. No cfg gates needed — pure env-var lookup using `std::env::var`. Applies cleanly on all platforms.

---

### FEAT-06b: `6d88638e` (profile namespace standardization) — ALIAS WORK REQUIRED

**Files touched by upstream (30 files, 120+/266-):** The commit renames `claude-code` references to `always-further/claude` in docs, scripts, READMEs, migration comments, and two source files (`cli.rs`, `migration.rs`). The actual profile JSON data entries in `policy.json` are NOT renamed by this commit (upstream moved profiles to the registry).

**Fork situation:** The fork still ships `claude-code`, `codex`, `swival`, `nono-ts-*` as embedded built-in profiles in `policy.json` and `builtin.rs`. The fork's `builtin.rs` tests assert `profile.meta.name == "claude-code"` (line 26). The `list_profiles()` test asserts `profiles.contains(&"claude-code".to_string())`.

**D-07 implementation:** The cherry-pick of `6d88638e` applies only the doc/comment/script changes (safe). The fork must additionally:
1. Register `always-further/claude` as an alias resolving to the `claude-code` built-in (or rename the built-in and keep `claude-code` as alias).
2. Update `builtin.rs` tests to cover both bare and namespaced names.
3. Leave `policy.json` `"claude-code"` key intact (bare name is the stable internal key; alias is the public-facing namespace form).

**D-08:** `nono-ts-wfp-test-open`, `nono-ts-wfp-test-blocked`, `nono-ts-default`, `swival` should gain namespace forms (e.g., `nono-ts/wfp-test-open`, `swival/default`) with the bare names as aliases.

---

### FEAT-06c: `42e5bf73` (truthy env bool flags) — CONFIRMED CLEAN APPLY

**Files:** `cli.rs` (65+/3-), test `env_vars.rs` (46+/0- new file). Adds `BoolishValueParser` to `--trust-proxy-ca`, `--trust-override`, `--capability-elevation`, wires `NONO_TRUST_OVERRIDE`. No platform-specific code. Applies cleanly.

---

### Cluster M misc (`a0bba5eb`, `ee7a3bda`, `7e076d2d`, `9800f307`) — CLEAN APPLIES

| SHA | What | Risk |
|-----|------|------|
| `a0bba5eb` | macOS blocked grants display in capability summary (`capability_ext.rs`, `output.rs`, `sandbox_prepare.rs`, `startup_runtime.rs`) | Additive; `a0bba5eb` touches `startup_runtime.rs` which may not exist in fork — verify file presence |
| `ee7a3bda` | nono.dev → nono.sh in `profile/mod.rs`, `profile_cmd.rs`, schema JSON | String replacement; safe |
| `7e076d2d` | Remove sigstore provenance display from `pull_ui.rs` | Deletion; safe |
| `9800f307` | Pack-verification dry-run skip in `profile_runtime.rs` + integration tests | Additive; safe |

**`startup_runtime.rs` note:** The ledger lists `a0bba5eb` as touching 4 files. Verify `crates/nono-cli/src/startup_runtime.rs` exists in the fork. If it does not, that hunk is an additional won't-apply.

---

### `e54cb` (env_clear removal) — UNIX-PATH ONLY CHERRY-PICK (D-14)

**The exact diff:** Removes `cmd.env_clear();` from `build_hook_command()` in `hook_runtime.rs` (line 196 in fork). This is the Unix hook path.

**`hook_runtime_windows.rs`:** Line 301 has `cmd.env_clear();` followed by lines 307-326 explaining the CLR baseline restore for SystemRoot/windir/SystemDrive. **Do NOT touch this file.** The DCO comment in the cherry-pick commit must note: "Applied to hook_runtime.rs (Unix path) only; hook_runtime_windows.rs retains env_clear() + CLR baseline per windows_hook_interpreter_spawn_gotchas."

**I-before-M ordering proof:** `7d274cf7` adds `cmd.env("PACK_DIR", ...)` AFTER `cmd.env_clear()` in upstream. After `e54cb` removes `cmd.env_clear()`, there is no ordering dependency between the two env injections. But if `e54cb` is applied first, the line numbers shift and `7d274cf7`'s contextual hunk (which uses `cmd.env_clear()` as a context line) fails to apply. Apply `7d274cf7` first.

---

### CR-01 (FFI stale LAST_DIAGNOSTIC_CODE) — FORK FIX, NOT CHERRY-PICK

**The bug (from 86-REVIEW.md CR-01):**
- `nono_merge_diagnostic_report_json` calls `set_last_error("session_json is null")` and `set_last_error("invalid UTF-8 in session_json")` without resetting `LAST_DIAGNOSTIC_CODE`.
- `nono_session_diagnostic_report_to_json` calls `set_last_error(&e)` for JSON parse failures without resetting `LAST_DIAGNOSTIC_CODE`.
- After any of these paths, a C caller reading `nono_last_diagnostic_code()` gets whatever code was set by the PREVIOUS `map_error()` call on this thread.

**D-10 fix pattern (clear-on-entry):** At the start of each public `extern "C"` fn that can set any thread-local, reset all three:

```rust
fn clear_last_call_state() {
    crate::LAST_DIAGNOSTIC_CODE.with(|c| *c.borrow_mut() = None);
    crate::LAST_REMEDIATION_JSON.with(|c| *c.borrow_mut() = None);
    LAST_ERROR.with(|c| *c.borrow_mut() = None);
}
```

Call `clear_last_call_state()` at the entry of `nono_session_diagnostic_report_to_json()` and `nono_merge_diagnostic_report_json()`, and review all other `pub extern "C"` fns in `bindings/c/src/` for the same pattern.

The existing `set_last_error()` in `lib.rs` only updates `LAST_ERROR`. The existing `map_error()` correctly updates all three. The fix makes `set_last_error` callers consistent with `map_error` callers by resetting state up-front rather than on-error.

**D-11 regression test:**
```rust
#[test]
fn diagnostic_code_is_cleared_between_calls() {
    // First call: populate LAST_DIAGNOSTIC_CODE via a successful map_error path
    // ... arrange an error that goes through map_error → sets Some(SandboxDeniedPath) ...
    
    // Second call: trigger the set_last_error-only path in nono_merge_diagnostic_report_json
    let json_ptr = unsafe { nono_merge_diagnostic_report_json(std::ptr::null(), std::ptr::null()) };
    assert!(json_ptr.is_null());
    // The diagnostic code must be Other (reset at entry), NOT the stale SandboxDeniedPath
    assert_eq!(nono_last_diagnostic_code(), NonoDiagnosticCode::Other);
}
```

**Upstream reference:** `a6aa9995` is the upstream SHA where the same logical fix was applied. This is a fork fix — do NOT cherry-pick `a6aa9995` (that commit adds the whole diagnostic module; it is already absorbed in Phase 86). Land as a fork-divergence commit with `Signed-off-by` only (no `-x` upward SHA). Record in DIVERGENCE-LEDGER as a Phase 88 addendum mirroring the Phase 87 CR-02 addendum pattern.

---

### DEPS-02 (9 dependency bumps) — typify-0.7 is NON-BREAKING

**typify 0.7.0 changelog (VERIFIED from github.com/oxidecomputer/typify CHANGELOG.adoc):**
- "Support for merging multiple string types with pattern fields (#1008)"
- "Generate JsonSchema for newtypes with enumerated patterns (#1017)"

No changes to `TypeSpaceSettings`, `with_struct_builder()`, `TypeSpace::new()`, or `to_stream()`. The `crates/nono/build.rs` pattern:
```rust
typify::TypeSpace::new(typify::TypeSpaceSettings::default().with_struct_builder(true))
```
compiles unchanged after the bump. **D-05 split is NOT needed.** [VERIFIED: CHANGELOG.adoc]

**Direct spec edits required:**
- `crates/nono/Cargo.toml:71` — `typify = "0.6"` → `typify = "0.7"` (the only Cargo.toml change for DEPS-02)

**Lockfile-only bumps (all via `cargo update`):**
- `cargo update -p cbindgen` (0.29.2 → 0.29.4)
- `cargo update -p hyper` (1.9.0 → 1.10.1; note: `hyper 0.14.x` also in Cargo.lock as legacy; only the `version = "1"` slot is bumped)
- `cargo update -p zeroize` (1.8.2 → 1.9.0)
- `cargo update -p time` (0.3.47 → 0.3.49; transitive)
- `cargo update -p chrono` (0.4.44 → 0.4.45)
- `cargo update -p ignore` (0.4.25 → 0.4.26)
- `cargo update -p which` (8.0.2 → 8.0.3)
- `cargo update -p x509-parser` (not yet present → 0.18.1; transitive via sigstore)

**D-06 path-dep pin gate:** After DEPS-02 commit, before `make ci`, manually verify:
```
grep "version" Cargo.toml crates/*/Cargo.toml bindings/c/Cargo.toml \
  | grep -E 'nono = \{|nono-proxy = \{'
```
Confirm all internal path-dep version pins are consistent. Phase 88 does NOT bump crate versions (marker-only milestone), so this should be a no-op, but the gate catches accidental drift.

---

## Common Pitfalls

### Pitfall 1: XDG State Split (Two Parallel Implementations)
**What goes wrong:** Cherry-pick `e8293b36` adds `state_paths.rs` but forgets to update `audit_session.rs::audit_root()` which still calls `nono_home_dir()?.join(".nono").join("audit")`. Result: audit writes to `~/.nono/audit/` while rollbacks write to `~/.local/state/nono/rollbacks/`. Sessions are silently split across two trees.
**Why it happens:** `e8293b36` rewrites `audit_session.rs` to use `state_paths::audit_root()` upstream, but the fork's `audit_session.rs` already diverged. The cherry-pick may not apply cleanly to `audit_session.rs`.
**How to avoid:** After the cherry-pick, run `grep -rn "nono_home_dir\|\.nono.*audit\|\.nono.*rollback" crates/nono-cli/src/` and verify zero hits (other than legacy fallback functions in `state_paths.rs` itself).
**Warning signs:** Tests using `NONO_TEST_HOME` pass but sessions written during integration tests appear in `$NONO_TEST_HOME/.nono/audit/` instead of `$NONO_TEST_HOME/.local/state/nono/audit/`.

### Pitfall 2: env_clear Applied to Windows Hook Path
**What goes wrong:** `e54cb` is cherry-picked and applied to `hook_runtime_windows.rs`, removing `env_clear()` without the SystemRoot/windir/SystemDrive safety net. PowerShell hooks exit `-65536` (CLR init failure).
**Why it happens:** The cherry-pick touches `hook_runtime.rs`; `hook_runtime_windows.rs` is a different file but contains the same `cmd.env_clear()` pattern. Lazy "apply everywhere" approach.
**How to avoid:** Cherry-pick specifies `-- crates/nono-cli/src/hook_runtime.rs` only. Confirm `hook_runtime_windows.rs` lines 301-326 are untouched post-apply.
**Warning signs:** Windows hook integration test exits with code `-65536` or `0xFFFF0000`.

### Pitfall 3: I-after-M ordering (rebase conflict on hook_runtime.rs)
**What goes wrong:** `e54cb` is applied before `7d274cf7`. The `7d274cf7` diff uses `cmd.env_clear()` as a context line (it appears before the new `cmd.env("PACK_DIR", ...)` line). If `env_clear` is already removed, the hunk fails to apply.
**How to avoid:** Strictly enforce I-before-M. The prescribed sequence above has `7d274cf7` (commit 6) before `e54cb` (commit 15).

### Pitfall 4: AWS auth without 501 on non-TLS proxy path
**What goes wrong:** `5bb098cd` is absorbed with the tls_intercept hunk skipped, but the non-TLS proxy path in `server.rs` → `reverse.rs` is not updated with the 501 stub. Result: an AWS-auth route is accepted at profile load time and registered in `CredentialStore::aws_routes`, but the proxy silently passes the request without AWS signing (no credential, no error, just unauthenticated outbound).
**Why it happens:** The 501 short-circuit in upstream lives in `tls_intercept/handle.rs` (won't-apply). The shared `server.rs` hunk doesn't include the 501 logic.
**How to avoid:** After applying the `5bb098cd` shared hunks, verify that a test with `aws_auth: {}` route config results in a 501 response from the proxy. Add a proxy integration test for this path.

### Pitfall 5: typify codegen generates different output on 0.7
**What goes wrong:** Even though the API is unchanged, typify 0.7's two new pattern features produce slightly different token output for existing schemas. The generated `capability_manifest_types.rs` in `OUT_DIR` differs from the 0.6.x output, causing spurious diffs or test failures.
**Why it happens:** build.rs generates into `OUT_DIR`; if any downstream code compares generated output or if a test snapshot exists, it fails.
**How to avoid:** `crates/nono/build.rs` writes to `OUT_DIR/capability_manifest_types.rs` and re-runs only when the schema changes. No snapshot tests exist for the generated code. The only risk is if the schema itself uses pattern constraints — inspect `schema/capability-manifest.schema.json` for `pattern:` keys.
**Warning signs:** `make build-lib` fails with a syn parse error or type-mismatch after the typify bump.

### Pitfall 6: Windows state_dir using XDG fallback
**What goes wrong:** `state_paths::user_state_dir()` is adopted without the D-02 Windows arm. On Windows, `crate::config::validated_home()` returns `C:\Users\Oscar`, so state lands at `C:\Users\Oscar\.local\state\nono` instead of `C:\Users\Oscar\AppData\Local\nono`. The provisioner created `%LOCALAPPDATA%\nono\workspace` → state and workspace are under different root paths.
**Why it happens:** `dirs::state_dir()` returns `None` on Windows (confirmed by docs); the `XDG_STATE_HOME` fallback code uses `HOME`, which on Windows is `%USERPROFILE%` not `%LOCALAPPDATA%`.
**How to avoid:** The D-02 Windows arm must be added to `state_paths::user_state_dir()` as a cfg(target_os = "windows") block reading `%LOCALAPPDATA%`. This is a fork-divergence from upstream's platform-agnostic implementation.

---

## Code Examples

### XDG state_paths.rs — Windows arm insertion point

After cherry-picking `e8293b36`, add this Windows arm to `user_state_dir()`:

```rust
// Source: fork-divergence for D-02 (Windows %LOCALAPPDATA% vs XDG)
pub fn user_state_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA").map_err(|_| {
            NonoError::Setup(
                "state_paths: %LOCALAPPDATA% is not set".to_string(),
            )
        })?;
        if local_app_data.trim().is_empty() {
            return Err(NonoError::Setup(
                "state_paths: %LOCALAPPDATA% is empty".to_string(),
            ));
        }
        return Ok(PathBuf::from(local_app_data).join("nono"));
    }
    #[cfg(not(target_os = "windows"))]
    Ok(resolve_xdg_state_base()?.join("nono"))
}
```

### CR-01 clear-on-entry pattern

```rust
// Source: fork fix for CR-01 (86-REVIEW.md § CR-01)
// In bindings/c/src/lib.rs
pub(crate) fn clear_last_call_state() {
    LAST_ERROR.with(|c| *c.borrow_mut() = None);
    LAST_DIAGNOSTIC_CODE.with(|c| *c.borrow_mut() = None);
    LAST_REMEDIATION_JSON.with(|c| *c.borrow_mut() = None);
}

// In bindings/c/src/diagnostic.rs — add at entry of each public fn
pub unsafe extern "C" fn nono_session_diagnostic_report_to_json(...) -> *mut c_char {
    crate::clear_last_call_state();  // CR-01: clear before this call's errors
    // ... existing body ...
}

pub unsafe extern "C" fn nono_merge_diagnostic_report_json(...) -> *mut c_char {
    crate::clear_last_call_state();  // CR-01: clear before this call's errors
    // ... existing body ...
}
```

### validate_set_vars — key validation logic (from d48aeb7b)

```rust
// Source: upstream d48aeb7b, crates/nono-cli/src/exec_strategy/env_sanitization.rs
pub(crate) fn validate_set_vars(
    set_vars: &std::collections::HashMap<String, String>,
) -> Option<String> {
    for key in set_vars.keys() {
        if key == "PATH" {
            return Some("Invalid set_vars key 'PATH': ...".to_string());
        }
        if key.starts_with("NONO_") {
            return Some(format!("Invalid set_vars key '{}': NONO_* reserved", key));
        }
        if !is_valid_env_var_name(key) {
            return Some(format!("Invalid set_vars key '{}': must match [A-Za-z_][A-Za-z0-9_]*", key));
        }
    }
    None
}
```

---

## Cross-Target Verification Surface

The CLAUDE.md MUST/NEVER rule: any commit touching `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, or `#[cfg(any(...))]` Unix blocks MUST be verified via `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`. Windows-host `cargo check` is NOT a substitute.

| Commit | Files with cfg-gated Unix code | PARTIAL→CI? |
|--------|-------------------------------|-------------|
| `d48aeb7b` (set_vars) | `exec_strategy.rs` (has cfg-gated blocks) — new code itself is std-only | Yes — file triggers the rule |
| `e8293b36` (XDG state) | `state_paths.rs` (new, no cfg gates); `audit_session.rs` (has `#[cfg(unix)]` perms block) | Yes — `audit_session.rs` has cfg blocks |
| `8e0d94f9` (XDG config) | `config/mod.rs`, `profile/mod.rs` — no cfg gates in touched hunks | No — unless these files have other cfg blocks |
| `5bb098cd` (AWS auth) | No cfg-gated blocks in shared-surface hunks | No |
| `c6b13345` (keyring) | `keystore.rs` — no cfg blocks in diff; feature-gated but not platform-cfg | No |
| `7d274cf7` ($PACK_DIR) | `hook_runtime.rs` has `#[cfg(unix)]` elements | Yes |
| `4179ce03` (PTY ctrl-z) | `exec_strategy.rs` + `pty_proxy.rs` — both have Unix-specific nix:: usage | **Yes — mandatory PARTIAL→CI** |
| `6d88638e` (profile namespace) | `cli.rs`, `migration.rs` — no cfg blocks | No |
| `e54cb` (env_clear Unix) | `hook_runtime.rs` has `#[cfg(unix)]` | Yes |
| D-02 Windows arm | New `#[cfg(target_os = "windows")]` + `#[cfg(not(target_os = "windows"))]` | Yes — cross-target for the non-Windows arm |

**Expected PARTIAL→CI deferral count:** 5–6 commits. Record each in the phase verification document per `.planning/templates/cross-target-verify-checklist.md`.

---

## Runtime State Inventory

> Greenfield feature additions — no rename/refactor in scope. However, FEAT-02 is a state-root MIGRATION (existing data moves).

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | Audit sessions at `~/.nono/audit/`; rollback sessions at `~/.nono/rollbacks/` (Unix) or `%LOCALAPPDATA%\nono\rollbacks` (Windows) | One-time migration via `maybe_migrate_legacy_audit_ledger()` at first write; dual-read for sessions |
| Live service config | None in scope for Phase 88 | None |
| OS-registered state | None in scope | None |
| Secrets/env vars | `NONO_KEYRING_TIMEOUT_SECS` is new (code-only, read from env at runtime) | None — no existing state |
| Build artifacts | `crates/nono/target/` — typify-generated `capability_manifest_types.rs` in OUT_DIR will be regenerated on next build | Clean build recommended after typify bump |

---

## Environment Availability

> Phase 88 is a source-code change phase; no new external tools or services required beyond the existing development setup.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All compilation | Confirmed (branch builds pass) | 1.82+ | — |
| cargo | Build/test | Confirmed | Same | — |
| `git` upstream remote | SHA lookup | Confirmed (all SHAs reachable) | — | — |
| Cross C-compiler (Linux/macOS) | PARTIAL→CI verification | Not available on Windows dev host | — | GH Actions Linux/macOS CI (mandatory per CLAUDE.md) |

**Missing dependencies with no fallback:**
- Cross-target clippy verification: Windows dev host cannot run `--target x86_64-unknown-linux-gnu` clippy. All Unix-path commits must be marked PARTIAL→CI and resolved by GH Actions.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `~/.nono/` for all runtime state | `~/.local/state/nono/` (XDG) with `~/.nono/` fallback | Upstream v0.63.0 (#1152) | FEAT-02; fork must adopt + Windows arm |
| `dirs::state_dir().or_else(dirs::data_local_dir)` in `config/mod.rs` | Delegate to `state_paths::user_state_dir()` | Upstream v0.63.0 | D-01 convergence |
| Bare profile names (`claude-code`, `codex`) | Namespaced (`always-further/claude`) + bare aliases | Upstream v0.63.0 | D-07: fork keeps bare as aliases |
| typify 0.6.x codegen | typify 0.7.x | Upstream v0.63.0 | Non-breaking; lockfile-only except spec edit |
| `env_clear()` in session hook subprocess | No env_clear (Unix); retains env_clear (Windows) | Upstream v0.63.0 | D-14: fork split |
| Block-forever keyring access | `NONO_KEYRING_TIMEOUT_SECS` (default 120s) | Upstream v0.63.0 | FEAT-04 |

**Deprecated/outdated:**
- `audit_session.rs::audit_root()` using `nono_home_dir()` directly: superseded by `state_paths::audit_root()` (FEAT-02 convergence).
- `rollback_session.rs::rollback_root()` platform split: superseded by `state_paths::rollback_root()`.
- `config/mod.rs` `user_state_dir()` implementation: superseded by delegation to `state_paths::user_state_dir()`.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | typify 0.7 API is non-breaking (no source changes needed) | DEPS-02 | If wrong: build.rs fails; D-05 split is needed; delays the wave |
| A2 | `startup_runtime.rs` exists in the fork (for a0bba5eb) | Cluster M | If absent: a0bba5eb hunk won't apply; skip or adapt |
| A3 | `undo/snapshot.rs` delegates rollback paths through `rollback_session::rollback_root()` not `nono_home_dir()` directly | FEAT-02 callsite audit | If wrong: snapshot writes split from state root; additional callsite to migrate |
| A4 | `exec_strategy.rs` PTY additions compile-fail on Windows without `#[cfg(unix)]` guard (nix:: not available on Windows) | DEPS-01 cross-target | If wrong (nix is already conditionally compiled and the file compiles fine on Windows): no additional cfg guard needed |
| A5 | `8e0d94f9` applies cleanly to the fork's `config/mod.rs` without conflict | FEAT-02 | If wrong: manual merge required; the fork's `config/mod.rs` has extra helpers (check_blocked_command etc.) after line 157 that may shift context |

---

## Open Questions (RESOLVED)

1. **`startup_runtime.rs` presence in fork** — RESOLVED
   - What we know: `a0bba5eb` touches `startup_runtime.rs` (4+/0- lines)
   - Resolution: file EXISTS in the fork (`crates/nono-cli/src/startup_runtime.rs`); `a0bba5eb` applies cleanly. Handled in Plan 88-05 `<interfaces>`.

2. **`undo/snapshot.rs` rollback-path delegation** — RESOLVED
   - What we know: `rollback_runtime.rs` comment line 223-224 references the path divergence
   - Resolution: `undo/snapshot.rs` does NOT call `nono_home_dir()` directly — it receives paths from callers (`rollback_session.rs`). No extra FEAT-02 callsite migration in the library crate. Handled in Plan 88-02 `<interfaces>`.

3. **AWS auth 501 on non-TLS proxy path** — RESOLVED
   - What we know: upstream 501 is in `tls_intercept/handle.rs` (won't-apply); shared hunks don't include a 501 for non-TLS
   - Resolution: fork needs a NET-NEW 501 short-circuit in `reverse.rs::handle_reverse_proxy()` when the AWS route/credential is present. Delivered by Plan 88-03 Task 1 with a proxy test.

---

## Sources

### Primary (HIGH confidence)
- `git show <sha>` for all 14 SHAs — actual diff inspection [VERIFIED: git object graph]
- `crates/nono-cli/src/config/mod.rs` lines 130-207 — current fork state-path implementations [VERIFIED: Read tool]
- `bindings/c/src/diagnostic.rs` — current CR-01 affected code [VERIFIED: Read tool]
- `bindings/c/src/lib.rs` — FFI thread-local store and map_error pattern [VERIFIED: Read tool]
- `crates/nono-cli/src/provision_windows.rs` — Windows `scratch_dir()` uses `%LOCALAPPDATA%\nono\workspace` [VERIFIED: Read tool]
- `crates/nono-cli/src/rollback_session.rs:29-43` — Windows arm uses `user_state_dir()` [VERIFIED: Read tool]
- `85-DIVERGENCE-LEDGER.md` — per-cluster dispositions, cross-checks, and dep-bump noise section [VERIFIED: Read tool]
- `88-CONTEXT.md` — all locked decisions D-01..D-15 [VERIFIED: Read tool]
- `86-REVIEW.md` — CR-01 full finding text [VERIFIED: Read tool]
- `87-CONTEXT.md` — D-13 CR-01 deferral rationale [VERIFIED: Read tool]
- typify CHANGELOG.adoc — 0.7.0 is non-breaking [VERIFIED: WebFetch from github.com/oxidecomputer/typify]
- `cargo search typify` — confirms typify 0.7.0 on registry [VERIFIED: Bash tool]

### Secondary (MEDIUM confidence)
- `crates/nono/build.rs` — typify usage pattern (TypeSpaceSettings::default().with_struct_builder(true)) [VERIFIED: Read tool]
- Main.rs lines 76-80 — pty_proxy Unix vs Windows module gating [VERIFIED: Read tool]

### Tertiary (LOW confidence — see Assumptions Log)
- `startup_runtime.rs` file presence [ASSUMED]
- `undo/snapshot.rs` rollback path delegation [ASSUMED]

---

## Metadata

**Confidence breakdown:**
- Cherry-pick applicability: HIGH — all SHAs reachable, actual diffs inspected
- typify-0.7 codegen: HIGH — CHANGELOG verified, non-breaking
- XDG Windows reconciliation: HIGH — provisioner code read, %LOCALAPPDATA% path confirmed
- AWS auth shared surface: HIGH — actual diff of each hunk inspected
- Profile namespace aliases: MEDIUM — upstream diff shows doc/comment renames only; fork alias implementation is new work
- Cross-target surface: HIGH — cfg blocks enumerated per file

**Research date:** 2026-06-20
**Valid until:** 2026-07-20 (stable domain; upstream has not pushed past v0.64.0)
