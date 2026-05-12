# Phase 35: UPST3-closure quick wins - Pattern Map

**Mapped:** 2026-05-12
**Files analyzed:** 5 production files + 1 test file across 3 plans
**Analogs found:** 5 / 5 (all in-tree; high-quality matches)

## File Classification

| New/Modified File | Plan | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | 35-01 | production / Windows-gated | request-response (child process env-block construction) | `crates/nono-cli/src/exec_strategy.rs` (Unix sibling: `execute_direct` lines 435-456 + `execute_supervised` lines 598-625) | exact (same task, opposite platform) |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | 35-01 | production / Windows-gated | config-struct surface | `crates/nono-cli/src/exec_strategy.rs` lines 314-325 (`ExecConfig.allowed_env_vars` + `denied_env_vars` fields with their D-20 doc comments) | exact |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` (tests) | 35-01 | test / Windows-gated `#[cfg(target_os = "windows")]` | unit test (filter call site invariant) | `crates/nono-cli/src/exec_strategy/env_sanitization.rs` tests + `crates/nono-cli/src/exec_strategy_windows/launch.rs` `pty_token_gate_tests` module (lines 1869-1961) | role-match (Windows-gated unit tests in same file) |
| `crates/nono-cli/src/profile_runtime.rs` | 35-02 | production / Linux-gated | file-I/O pre-create | upstream `bdf183e9` Landlock-hardening hunk (no in-tree analog — first pre-create-before-Landlock in fork) + `crates/nono-cli/src/config/user.rs::user_profiles_dir()` (line 286) | role-match (in-tree path helper; pattern net-new) |
| `crates/nono-cli/src/query_ext.rs` | 35-03 | production / cross-platform with `#[cfg(target_os = "windows")]` strip | transform (UNC-prefix strip in suggested_flag emission) | `crates/nono-cli/src/query_ext.rs::strip_verbatim_prefix` (lines 295-317) + commit `400f8c90` `canonical_for_sensitive` usage (line 87) | exact (same file, same helper, different call site) |
| `crates/nono-cli/src/profile_cmd.rs` | 35-03 | production / cross-platform | transform (JSON emission of Option<…> enum security fields) | `crates/nono-cli/src/profile_cmd.rs::build_skeleton` (lines 141-247: `serde_json::Map` insertion with omit-when-None semantics) + upstream `f3e7f885` (out-of-tree) | role-match (same file uses the canonical Map shape elsewhere; only the show/diff helpers regressed) |

## Pattern Assignments

### `crates/nono-cli/src/exec_strategy_windows/launch.rs` and `mod.rs` (Plan 35-01)

**Plan goal:** Wire `allowed_env_vars` / `denied_env_vars` into the Windows execution path, mirroring the Unix call site Plan 34-08a landed.

**Analog 1 — Unix ExecConfig field shape:** `crates/nono-cli/src/exec_strategy.rs` lines 314-325

```rust
/// Plan 34-08a Task 3 (D-20 manual replay of upstream `1b412a7`):
/// allow-list of environment variable names. When `Some`, only
/// variables matching an exact name or prefix pattern (e.g. `"AWS_*"`)
/// are passed to the child. `None` means inherit-all (default).
/// Nono-injected credentials (`config.env_vars`) always bypass this list.
pub allowed_env_vars: Option<Vec<String>>,
/// Plan 34-08a Task 4 (D-20 manual-replay-by-escalation of upstream
/// v0.52.0 `3657c935`): operator-controlled deny-list of environment
/// variable names. Variables matching an exact name or prefix pattern
/// (e.g. `"GITHUB_*"`) are stripped even if they also appear in
/// `allowed_env_vars`. Nono-injected credentials bypass this list.
pub denied_env_vars: Option<Vec<String>>,
```

**Apply to Windows `ExecConfig`** at `exec_strategy_windows/mod.rs:130-145`. Currently the Windows `ExecConfig` (lines 130-145) has no `allowed_env_vars` / `denied_env_vars` fields at all. Add the two `pub` fields at the bottom of the struct with the same doc-comment shape, dropping any `#[cfg_attr(target_os = "windows", allow(dead_code))]` gate. (Per D-35-A1, this is the only Windows-only-file edit Phase 35 permits — explicitly inverted from D-34-E1.)

**Analog 2 — Unix filter call-site shape (the load-bearing pattern):** `crates/nono-cli/src/exec_strategy.rs` lines 435-457 (`execute_direct`)

```rust
for (key, value) in std::env::vars() {
    if should_skip_env_var(&key, &config.env_vars, &["NONO_CAP_FILE"]) {
        continue;
    }
    // Plan 34-08a Task 4 (D-20 replay of v0.52.0 `3657c935`): deny-list
    // checked BEFORE the allow-list. Precedence: dangerous-var blocklist
    // > deny_vars > allow_vars. Nono-injected credentials bypass both
    // (unconditionally added below).
    if let Some(ref denied) = config.denied_env_vars {
        if env_sanitization::is_env_var_denied(&key, denied) {
            continue;
        }
    }
    // Plan 34-08a Task 3 (D-20 replay of `1b412a7`): when an allow-list
    // is configured, only matching variables pass through. Profile-
    // injected env_vars are unconditionally added below (after this
    // loop) — they always bypass the allow-list.
    if let Some(ref allowed) = config.allowed_env_vars {
        if !env_sanitization::is_env_var_allowed(&key, allowed) {
            continue;
        }
    }
    cmd.env(&key, &value);
}
```

**Apply to Windows `build_child_env`** at `exec_strategy_windows/launch.rs:551-647`. Current Windows shape (lines 551-647) only invokes `should_skip_env_var`; the `for (key, value) in std::env::vars()` body must add the same two `denied` / `allowed` filter arms immediately after the existing `should_skip_env_var` check (which has already been inverted via `if !should_skip_env_var(...)`). The deny-check precedes the allow-check; both bypassed for `config.env_vars` (nono-injected credentials) which are added in a later loop at lines 656-658.

**Important divergence to preserve:** Windows passes a much longer `blocked_extra` list (lines 558-643: `PATH`, `PATHEXT`, `COMSPEC`, `SystemRoot`, `APPDATA`, etc. — 60+ entries) because Windows env-block runtime entries are rebuilt in `append_windows_runtime_env` (line 665+) rather than inherited. The new `denied` / `allowed` filters must compose with — not replace — that existing list. Add the two new arms inside the existing `if !should_skip_env_var(...) { ... }` block, before `env_pairs.push((key, value));`.

**Analog 3 — Re-exports:** `crates/nono-cli/src/exec_strategy_windows/mod.rs` lines 20-21, 77-78

```rust
#[path = "../exec_strategy/env_sanitization.rs"]
mod env_sanitization;
// ...
pub(crate) use env_sanitization::is_dangerous_env_var;
use env_sanitization::should_skip_env_var;
```

**Apply:** Extend the existing `use env_sanitization::...` line at mod.rs:78 (or add a sibling `use` in launch.rs) to also bring in `is_env_var_allowed` and `is_env_var_denied`. They are already declared `pub(crate)` in `env_sanitization.rs` (lines 114, 154) so the path-rewired Windows copy of the module already exposes them. The current `#[allow(dead_code)]` attribute on those two functions at lines 113 + 153 of `env_sanitization.rs` should be REMOVED once Windows wires them, since they will no longer be dead on Windows.

**Analog 4 — Windows-gated unit-test shape:** `crates/nono-cli/src/exec_strategy_windows/launch.rs` lines 1869-1961 (`pty_token_gate_tests` module)

```rust
#[cfg(test)]
mod pty_token_gate_tests {
    use super::{select_windows_token_arm, WindowsTokenArm};

    /// Phase 31 D-15 / D-01 NEW path: PTY allocation triggers the broker spawn,
    /// even when session_sid is also Some (which it always is on Windows supervised).
    /// This test pins the branch-ordering rule documented in 31-CONTEXT D-01.
    #[test]
    fn pty_some_no_detach_selects_broker_launch() {
        let arm = select_windows_token_arm(
            /* is_detached */ false, /* has_pty */ true,
            /* has_session_sid */ true,
            /* caps_demand_low_il */ false,
        );
        assert_eq!(arm, WindowsTokenArm::BrokerLaunch);
    }
}
```

**Apply:** Add a new `#[cfg(test)]` module at the bottom of `exec_strategy_windows/launch.rs` (similar to `pty_token_gate_tests` shape). Name it `env_filter_tests` or similar. The mandated D-35-B3 test is `test_windows_empty_allow_denies_all_env_vars`. It must call `build_child_env` (or whichever helper takes an `ExecConfig` and returns the env-pair list) with an `ExecConfig` carrying `allowed_env_vars: Some(vec![])` and `denied_env_vars: None`, then assert the result contains **only** the runtime-block entries from `append_windows_runtime_env` plus any `config.env_vars` — zero inherited user env vars.

**Important:** Per CLAUDE.md § Testing § Environment variables in tests, if the test calls into a helper that reads `std::env::vars()`, the test must save/restore at least `HOME` / `USERPROFILE` / any test-fixture env var it seeds. Look at the `low_integrity_primary_token_tests` module (lines 1963+) for the existing `#[cfg(all(test, target_os = "windows"))]` shape and re-use that gate.

---

### `crates/nono-cli/src/profile_runtime.rs` (Plan 35-02)

**Plan goal:** Cherry-pick upstream `bdf183e9` 15-line Landlock pre-create hunk behind `#[cfg(target_os = "linux")]`. Pre-create `~/.config/nono/profiles/` BEFORE Landlock ruleset apply (Landlock requires the parent directory to exist for `mkdir` operations even when the child path is granted write).

**Analog 1 — Path helper to invoke:** `crates/nono-cli/src/config/user.rs` lines 286-292

```rust
/// Get the path to user profiles directory
pub fn user_profiles_dir() -> Result<PathBuf> {
    let config_dir = super::user_config_dir().ok_or_else(|| {
        NonoError::ConfigParse("Could not determine user config directory".to_string())
    })?;

    Ok(config_dir.join("profiles"))
}
```

**Apply:** Call `crate::config::user_profiles_dir()` to resolve `~/.config/nono/profiles/`, then `std::fs::create_dir_all()` it before returning from `prepare_profile` (or wherever the caller subsequently constructs the Landlock ruleset — see Analog 3). Do NOT use `dirs::home_dir()` directly (STATE.md Windows blocker per CONTEXT § Deferred). The existing `user_profiles_dir()` helper is XDG-aware and the canonical fork-wide path resolution.

**Analog 2 — Existing `create_dir_all` shape in fork:** `crates/nono-cli/src/profile/builtin.rs` lines 447-451

```rust
profiles_dir: &std::path::Path,
// ...
std::fs::create_dir_all(profiles_dir)?;
```

**Apply:** Mirror this shape — `std::fs::create_dir_all(...)?` propagates `io::Error` via the `From<io::Error> for NonoError` impl. Idempotent: a no-op if the directory already exists, which is the normal case after first invocation. No `.unwrap()` (CLAUDE.md § Coding Standards inherited verbatim per D-35-D2).

**Analog 3 — Linux-gated insertion site:** `crates/nono-cli/src/profile_runtime.rs` lines 9-10 + 141-145 (existing Linux gates)

```rust
#[cfg(target_os = "linux")]
pub(crate) wsl2_proxy_policy: profile::Wsl2ProxyPolicy,
// ...
#[cfg(target_os = "linux")]
wsl2_proxy_policy: loaded_profile
    .as_ref()
    .and_then(|profile| profile.security.wsl2_proxy_policy)
    .unwrap_or_default(),
```

**Apply:** Add a `#[cfg(target_os = "linux")]` helper function (e.g. `pre_create_landlock_profiles_dir`) early in `prepare_profile` (line 123+). It runs BEFORE the function returns the `PreparedProfile`. The caller (`sandbox_prepare.rs:298`) then proceeds to build the `CapabilitySet` and call `Sandbox::apply` (which on Linux invokes `restrict_self` via `crates/nono/src/sandbox/linux.rs:860`).

**D-19 commit shape (per D-35-A4):** Plan 35-02 is the only Phase 35 commit with the full 6-line trailer block. From `.planning/templates/upstream-sync-quick.md`:

```
Upstream-commit: bdf183e9
Upstream-author: Luke Hinds <luke@example.com>
Upstream-tag: v0.44.0
[remaining 3 trailer lines per template]
```

Lowercase `'a'` in `Upstream-author:`. Smoke check at plan close: `git log --format='%B' main~1..main | grep -c '^Upstream-commit: '` equals 1 for Plan 35-02.

---

### `crates/nono-cli/src/query_ext.rs` (Plan 35-03)

**Plan goal:** Strip the `\\?\` UNC prefix in the `suggested_flag` value emitted by `query_path`, mirroring the shape of in-fork commit `400f8c90`. Fixes the `test_query_path_denied` flake at the production-code source (not via test gating).

**Analog — in-tree precedent (commit `400f8c90`):** `crates/nono-cli/src/query_ext.rs` lines 87 + 295-317

```rust
// At line 87 — usage site for sensitive-path check:
let canonical_for_sensitive = strip_verbatim_prefix(&canonical);

// At lines 295-317 — helper definition (already exists in tree):
/// Strip the Windows NT verbatim / device prefixes (`\\?\`, `\\?\UNC\`, `\??\`)
/// from a canonicalized path so it can be compared against non-canonical
/// policy paths that were produced by env-var expansion.
///
/// Kept deliberately narrow: on non-Windows the returned path is identical,
/// and on Windows only the well-known prefixes are stripped. Same pattern as
/// `protected_paths::normalize_for_compare`, localized here so query_path's
/// sensitive-path check does not need to pull in that module's wider
/// normalization rules.
#[cfg(target_os = "windows")]
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let raw = path.as_os_str().to_string_lossy();
    let stripped = raw
        .replace("\\\\?\\UNC\\", r"\\")
        .replace("\\\\?\\", "")
        .replace("\\??\\", "");
    PathBuf::from(stripped)
}

#[cfg(not(target_os = "windows"))]
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    path.to_path_buf()
}
```

**Apply:** The helper already exists. The fix is to wrap `&canonical` with `strip_verbatim_prefix(&canonical)` at the two `suggested_flag` emission sites:

- `crates/nono-cli/src/query_ext.rs:167` (near-miss case inside `if let Some(cap) = best_covering`):
  ```rust
  suggested_flag: Some(suggested_flag_for_path(&canonical, requested)),
  ```
  becomes:
  ```rust
  suggested_flag: Some(suggested_flag_for_path(&strip_verbatim_prefix(&canonical), requested)),
  ```

- `crates/nono-cli/src/query_ext.rs:179` (path-not-granted case at end of `query_path`): same wrap.

The non-Windows no-op `strip_verbatim_prefix` (line 315) ensures Linux/macOS behaviour is byte-identical (no `#[cfg]` gates at the call site — per D-35-C1 "no `#[cfg]` gates", the cross-platform helper handles platform dispatch internally).

**Path-component-safety note:** Per CLAUDE.md § Path Handling § Common Footguns #1 ("string `starts_with()` on paths is a vulnerability"), the existing helper uses `.replace()` on the OS-string lossy representation rather than `Path::starts_with`. This is the canonical fork pattern for prefix stripping and matches `protected_paths::normalize_for_compare`. The replace operations are on well-known verbatim prefix tokens that are NOT path components themselves (they are encoding artifacts of `Path::canonicalize` on Windows), so the path-component-safety footgun does not apply here.

**Test that will then pass deterministically:** `crates/nono-cli/src/query_ext.rs:365-383` (existing `test_query_path_denied`). The assertion `assert_eq!(suggested_flag.as_deref(), Some("--read /some/random"))` at line 378 will pass on Windows because the canonical path `\\?\C:\some\random\path` will be stripped to `C:\some\random\path` BEFORE `suggested_flag_for_path` derives the parent. (Note: the test asserts a POSIX-shape path string; verify on a Windows host whether the underlying `tempfile`/`PathBuf` shape requires test-data adjustment — the CONTEXT indicates this is the expected fix.)

---

### `crates/nono-cli/src/profile_cmd.rs` (Plan 35-03)

**Plan goal:** Replace `format!("{:?}", …)` JSON-emission of `Option<…>` security fields with serde-driven Map insertion + omit-when-None semantics. Mirrors upstream `f3e7f885` shape adopted by Plan 34-04b. Full audit of ALL `format!("{:?}")` / `format!("{:#?}")` JSON-emission sites in `profile_cmd.rs` (D-35-C3).

**Sites in scope** (all confirmed by grep `format!("\{:?\}"|format!("\{:\#?\}"|format!("\{v:\?\}"` in profile_cmd.rs):

| Line | Function | Field | Current shape | Fix |
|------|----------|-------|---------------|-----|
| 1056 | `profile_to_json` | `signal_mode: Option<ProfileSignalMode>` | `format!("{:?}", profile.security.signal_mode)` produces `"Some(Isolated)"` or `"None"` | Map insertion: omit when None, snake_case string when Some |
| 1057 | `profile_to_json` | `process_info_mode: Option<ProfileProcessInfoMode>` | same shape | same fix |
| 1058 | `profile_to_json` | `ipc_mode: Option<ProfileIpcMode>` | same shape | same fix |
| 1060 | `profile_to_json` | `wsl2_proxy_policy: Option<Wsl2ProxyPolicy>` | same shape | same fix |
| 1098 | `profile_to_json` | `workdir.access: WorkdirAccess` (NOT Optional — bare enum) | `format!("{:?}", profile.workdir.access)` produces `"ReadWrite"` (PascalCase leak) | Direct `serde_json::to_value(&profile.workdir.access)?` — emits snake_case via `#[serde(rename_all = "lowercase")]` on enum (line 1340 of profile/mod.rs) |
| 1297-1298 | `cmd_diff` body | `wsl2_proxy_policy` for human-readable diff (NOT JSON — colored stdout) | string-level diff helper | **OUT OF SCOPE** per D-35-C3 (only JSON-emission helpers in this file; this is `diff_scalar_option` printing to stdout) |
| 1303-1304 | `cmd_diff` body | `signal_mode` for human-readable diff | same — stdout printer | **OUT OF SCOPE** |
| 1309-1310 | `cmd_diff` body | `process_info_mode` for human-readable diff | same — stdout printer | **OUT OF SCOPE** |
| 1315-1316 | `cmd_diff` body | `ipc_mode` for human-readable diff | same — stdout printer | **OUT OF SCOPE** |
| 1812-1813 | `diff_to_json` | `wsl2_proxy_policy` (paired profile1/profile2) | `format!("{:?}", p1.security.wsl2_proxy_policy)` | Map insertion: omit when None, snake_case when Some, in each of profile1/profile2 |
| 1818-1819 | `diff_to_json` | `workdir.access` (paired profile1/profile2; NOT Optional) | `format!("{:?}", p1.workdir.access)` | Direct `serde_json::to_value(...)?` for each side |
| 1991 | `diff_custom_credentials_json` | `inject_mode: InjectMode` (re-exported from `nono_proxy::config::InjectMode`, NOT Optional) | `format!("{:?}", old.inject_mode)` | Verify `InjectMode` carries `#[serde(rename_all = "snake_case")]`; if so, use `serde_json::to_value(&old.inject_mode)?`; if not, this is D-35-C3 discretion: either add the serde attr in nono-proxy OR keep `format!` and add a regression-test marker for `inject_mode` values |

**Out-of-scope clarification:** Lines 1289-1318 in `cmd_diff` are colored human-readable stdout via `diff_scalar_option`, NOT JSON emission. They DON'T feed `policy show --json` or `policy diff --json`. D-35-C3 audit scope is "format!("{:?}") sites in JSON-emission helpers in `profile_cmd.rs`" — `cmd_diff` body's stdout printing isn't a JSON helper. Leave those alone; the regression tests don't exercise them.

**Analog — canonical Map-insertion shape already present in this file:** `crates/nono-cli/src/profile_cmd.rs:141-247` (`build_skeleton`)

```rust
fn build_skeleton(args: &ProfileInitArgs) -> serde_json::Value {
    let mut root = serde_json::Map::new();

    if let Some(ref base) = args.extends {
        root.insert(
            "extends".to_string(),
            serde_json::Value::String(base.clone()),
        );
    }
    // ...
    let mut meta = serde_json::Map::new();
    meta.insert(
        "name".to_string(),
        serde_json::Value::String(args.name.clone()),
    );
    if let Some(ref desc) = args.description {
        meta.insert(
            "description".to_string(),
            serde_json::Value::String(desc.clone()),
        );
    }
    root.insert("meta".to_string(), serde_json::Value::Object(meta));
```

**Apply** to `profile_to_json` (line 1041) and `diff_to_json` (line 1777): replace the `serde_json::json!({ ... })` macro for the `security` section (currently lines 1053-1061) with explicit `serde_json::Map` insertion. For each Option<…> security field:

```rust
let mut security = serde_json::Map::new();
security.insert("groups".to_string(), serde_json::json!(profile.security.groups));
security.insert("allowed_commands".to_string(), serde_json::json!(profile.security.allowed_commands));
// Plan 35-03 (D-35-C2): Option<...> security fields — omit when None,
// emit serde-driven snake_case string when Some. Mirrors upstream f3e7f885
// (v0.47.0) per Plan 34-04b SUMMARY's "Map-insertion for Option<...> Security
// fields, omitted-when-None semantics" expectation. Replaces the regressed
// format!("{:?}", …) shape that leaked Rust Debug syntax ("Some(Isolated)").
if let Some(ref mode) = profile.security.signal_mode {
    security.insert(
        "signal_mode".to_string(),
        serde_json::to_value(mode).map_err(|e| NonoError::ProfileParse(format!("signal_mode serialize: {e}")))?,
    );
}
if let Some(ref mode) = profile.security.process_info_mode {
    security.insert("process_info_mode".to_string(), serde_json::to_value(mode).map_err(|e| /* ... */)?);
}
// (same for ipc_mode, wsl2_proxy_policy)
security.insert("capability_elevation".to_string(), serde_json::json!(profile.security.capability_elevation));
val.as_object_mut()
    .ok_or_else(|| NonoError::ProfileParse("profile_to_json root not an object".to_string()))?
    .insert("security".to_string(), serde_json::Value::Object(security));
```

**Verified:** All four enums already carry `#[serde(rename_all = "snake_case")]`:
- `ProfileSignalMode` at profile/mod.rs:1250-1259
- `ProfileProcessInfoMode` at profile/mod.rs:1274-1283
- `ProfileIpcMode` at profile/mod.rs:1298-1305
- `Wsl2ProxyPolicy` at profile/mod.rs:1324-1337

And `WorkdirAccess` (profile/mod.rs:1339-1351) carries `#[serde(rename_all = "lowercase")]`. So `serde_json::to_value(&Wsl2ProxyPolicy::InsecureProxy)` produces `"insecure_proxy"` and `serde_json::to_value(&WorkdirAccess::ReadWrite)` produces `"readwrite"`. (D-35-C2 "Claude's discretion" verification: no enum attribute changes required.)

**Signature change implication:** `profile_to_json` (line 1041) currently returns `serde_json::Value` directly. The new `serde_json::to_value` calls return `Result<Value, serde_json::Error>`. Two options:
1. Change the return type to `Result<serde_json::Value>` (preferred — matches the `to_json` pattern at line 22).
2. Use `serde_json::to_value(...).unwrap_or(serde_json::Value::Null)` — REJECTED per CLAUDE.md § Coding Standards (no `.unwrap()` / `.unwrap_or` on a security-critical path).

The planner should pick option 1 and update the two call sites (`cmd_show` line 746 + `cmd_diff` line 1164) to propagate the Result via `?`.

**Regression-test invariant (locked):** `crates/nono-cli/tests/profile_cli.rs:120-176` — `test_policy_show_json_no_rust_debug_syntax` + `test_policy_diff_json_no_rust_debug_syntax` exercise the full output of `nono policy show --json` and `nono policy diff --json` for three built-in profiles (`default`, `claude-code`, `node-dev`). They assert no string in the resulting JSON contains `"Some("`, `"None)"`, `"Isolated"`, `"AllowSameSandbox"`, `"AllowAll"`, `"ReadWrite"`, or `"InsecureProxy"` (the seven Debug-format markers). Both tests must pass deterministically post-fix on Windows + Linux + macOS.

---

## Shared Patterns

### Path-stripping helper boundary
**Source:** `crates/nono-cli/src/query_ext.rs::strip_verbatim_prefix` (lines 295-317)
**Apply to:** Any future Phase 35 surface that emits a canonicalized path into user-visible output (CLI flag suggestions, JSON paths, error messages).
**Shape:** `#[cfg(target_os = "windows")]` strips `\\?\`, `\\?\UNC\`, `\??\`; `#[cfg(not(target_os = "windows"))]` is the identity no-op. Use string `.replace()` on the lossy OS-string representation — NOT `Path::starts_with()` (footgun per CLAUDE.md). The replace tokens are verbatim-prefix encoding artifacts, not path components.

### Linux-gated production hunk
**Source:** `crates/nono-cli/src/profile_runtime.rs` lines 9-10, 141-145 (existing `#[cfg(target_os = "linux")]` gates on `wsl2_proxy_policy` field + initializer)
**Apply to:** Plan 35-02 Landlock pre-create hunk.
**Shape:** `#[cfg(target_os = "linux")]` attribute on either a helper function or an inline block early in `prepare_profile`. Must compile cleanly on Windows + macOS (no symbols referenced). Cross-target Linux clippy gate (D-35-D2 step 3) is load-bearing because Windows-host `cargo clippy --workspace` cannot lint inside Linux-gated blocks (memory file `feedback_clippy_cross_target.md` — Phase 25 CR-A lesson).

### Cross-platform `serde_json::to_value` Map insertion
**Source:** `crates/nono-cli/src/profile_cmd.rs::build_skeleton` (lines 141-247) — already in this file for the `nono profile init` skeleton
**Apply to:** Plan 35-03 `profile_to_json` (line 1041) and `diff_to_json` (line 1777) replacement.
**Shape:**
```rust
let mut section = serde_json::Map::new();
section.insert("non_optional_field".to_string(), serde_json::json!(profile.field));
if let Some(ref value) = profile.optional_field {
    section.insert(
        "optional_field".to_string(),
        serde_json::to_value(value).map_err(|e| NonoError::ProfileParse(...))?,
    );
}
// (omit-when-None: just don't insert)
```
For non-Optional enum fields, drop the `if let` and call `serde_json::to_value` unconditionally.

### Test save/restore for env-var manipulation
**Source:** CLAUDE.md § Testing § Environment variables in tests
**Apply to:** Plan 35-01 `test_windows_empty_allow_denies_all_env_vars` if it seeds env vars to verify the empty-allow strip-all invariant.
**Shape:** Save the prior value via `std::env::var_os("KEY").ok()` BEFORE setting, then restore in the test's scope-exit (or via a guard struct that implements `Drop`). Rust runs unit tests in parallel within the same process — an unrestored env var causes flaky failures in unrelated tests (per CLAUDE.md, the example given is `config::check_sensitive_path` failing when another test temporarily sets `HOME`).

### D-19 cherry-pick trailer (Plan 35-02 only)
**Source:** `.planning/templates/upstream-sync-quick.md` § D-19 cherry-pick trailer block; D-35-A4 in CONTEXT.md
**Apply to:** Plan 35-02 cherry-pick commit only. NOT Plan 35-01 (D-20 manual replay), NOT Plan 35-03 (fork-local regression fixes).
**Shape:** Verbatim 6-line block. Lowercase `'a'` in `Upstream-author:`. Smoke check at plan close: `git log --format='%B' main~1..main | grep -c '^Upstream-commit: '` equals 1 for Plan 35-02.

## No Analog Found

| File / Pattern | Plan | Reason |
|----------------|------|--------|
| Pre-create-before-Landlock invocation pattern | 35-02 | First instance in fork — upstream `bdf183e9` introduces the shape; fork's `crates/nono-cli/src/profile/builtin.rs:447-451` uses `create_dir_all` in a different context (`seed_user_profile` test helper, not production runtime pre-sandbox-apply). Plan 35-02 establishes the canonical fork pattern. |

## Metadata

**Analog search scope:**
- `crates/nono-cli/src/exec_strategy.rs` (Unix execution path)
- `crates/nono-cli/src/exec_strategy/env_sanitization.rs` (filter helpers)
- `crates/nono-cli/src/exec_strategy_windows/` (Windows execution path: mod.rs, launch.rs)
- `crates/nono-cli/src/profile_runtime.rs` (profile preparation, Plan 35-02 target)
- `crates/nono-cli/src/query_ext.rs` (Plan 35-03 target — UNC strip)
- `crates/nono-cli/src/profile_cmd.rs` (Plan 35-03 target — JSON emission)
- `crates/nono-cli/src/profile/mod.rs` (enum serde attributes verification)
- `crates/nono-cli/src/config/user.rs` (path helper for Landlock pre-create)
- `crates/nono-cli/tests/profile_cli.rs` (regression-test invariants)
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-08a-ENV-SURFACE-PORT-SUMMARY.md` (Plan 34-08a reference shape)
- `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` (P34-DEFER ledger for closure ordering)

**Files scanned:** 11
**Pattern extraction date:** 2026-05-12

## Quick Reference for Planner

### Plan 35-01 — WIN-ENV-FILTER (REQ-PORT-CLOSURE-01)
- **Modify:** `crates/nono-cli/src/exec_strategy_windows/mod.rs` (ExecConfig fields) + `crates/nono-cli/src/exec_strategy_windows/launch.rs` (build_child_env filter + new tests module)
- **Modify:** `crates/nono-cli/src/exec_strategy/env_sanitization.rs` (remove two `#[allow(dead_code)]` attributes at lines 113 + 153)
- **Wire from:** `crates/nono-cli/src/exec_strategy.rs:435-457` (Unix call-site shape) + `crates/nono-cli/src/exec_strategy.rs:314-325` (Unix ExecConfig fields with D-20 doc comments)
- **Acceptance test:** `test_windows_empty_allow_denies_all_env_vars` (Windows-gated unit test in launch.rs, modeled on `pty_token_gate_tests` at lines 1869-1961)
- **Commit shape:** D-20 manual-replay; commit body cites Plan 34-08a + upstream `1b412a7` (v0.37.0) + `780965d7` (fail-closed)

### Plan 35-02 — LINUX-LANDLOCK-PROFILES (REQ-PORT-CLOSURE-06)
- **Modify:** `crates/nono-cli/src/profile_runtime.rs` only (single ~15-line hunk behind `#[cfg(target_os = "linux")]`)
- **Use helper:** `crate::config::user_profiles_dir()` (resolves `~/.config/nono/profiles/`)
- **Pattern:** `std::fs::create_dir_all(&path)?` (idempotent; propagates via `From<io::Error> for NonoError`)
- **Linux integration test:** `#[cfg(target_os = "linux")]` + `#[ignore]` on Windows host (per D-35-D3 CI Linux lane verification)
- **Commit shape:** Full D-19 6-line trailer block; smoke check `grep -c '^Upstream-commit: ' == 1`

### Plan 35-03 — WIN-TEST-HYGIENE (REQ-PORT-CLOSURE-07)
- **Modify:** `crates/nono-cli/src/query_ext.rs` lines 167 + 179 — wrap `&canonical` with `strip_verbatim_prefix(&canonical)` (helper already exists at lines 295-317)
- **Modify:** `crates/nono-cli/src/profile_cmd.rs::profile_to_json` (lines 1052-1061) — replace `format!("{:?}", …)` security fields with `serde_json::Map` insertion + omit-when-None
- **Modify:** `crates/nono-cli/src/profile_cmd.rs::profile_to_json` (line 1098) — replace `format!("{:?}", profile.workdir.access)` with `serde_json::to_value`
- **Modify:** `crates/nono-cli/src/profile_cmd.rs::diff_to_json` (lines 1811-1820) — same pattern for paired profile1/profile2 emission
- **Modify:** `crates/nono-cli/src/profile_cmd.rs::diff_custom_credentials_json` (line 1991) — `inject_mode` review per D-35-C3 audit
- **Modify:** `cmd_show` (line 746) + `cmd_diff` (line 1164) call sites — propagate new `Result<Value>` return type via `?`
- **Append:** Phase 35 closure section to `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` (D-35-D4)
- **Regression tests** (already exist; must pass post-fix): `crates/nono-cli/tests/profile_cli.rs:127` + `:155`
- **Commit shape:** Regular DCO sign-off; commit body cites `f3e7f885` + `400f8c90` as design-source citations (NOT cherry-picks)
