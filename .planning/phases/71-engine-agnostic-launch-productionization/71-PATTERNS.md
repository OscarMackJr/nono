# Phase 71: Engine-Agnostic Launch Productionization - Pattern Map

**Mapped:** 2026-06-13
**Files analyzed:** 9 (8 modified + 1 new test/UAT doc surface)
**Analogs found:** 9 / 9 (all in-tree; this is an extension phase, not greenfield)

> **RESEARCH.md line-number corrections (load-bearing — verified by direct read):**
> - `path_has_write_owner` **does NOT exist** as a named function anywhere. RESEARCH.md cites `windows.rs:1050` for it; line 1050 is inside `try_set_mandatory_label`. The WRITE_OWNER logic is **inline** in `try_set_mandatory_label` (windows.rs:1133-1169) and only calls `path_is_owned_by_current_user` (the weaker owner-SID-equality check). **The D-08 R-B3 pre-gate must either (a) extract a new `path_has_write_owner` helper, or (b) reuse `path_is_owned_by_current_user` (windows.rs:1205) as the gate.** Planner must decide; do NOT assume `path_has_write_owner` exists.
> - `covers_path` / `covers_execution_dir` / `has_rules` / `has_user_intent_directory_rules` / `is_fully_supported` are **methods on `WindowsFilesystemPolicy` defined in `crates/nono/src/sandbox/mod.rs:492-569`**, NOT free functions in `windows.rs`. RESEARCH.md cites `windows.rs:2112` for `covers_path` — that is a *call site*, not the definition.
> - `normalize_windows_launch_path` is in **launch.rs:1032** (definition); the `config.current_dir` strip call site is **launch.rs:1318**. Both RESEARCH.md citations are correct (just two different lines).
> - `validate_launch_paths` at windows.rs:2091 is correct, but the CLI calls it via the wrapper `Sandbox::validate_windows_launch_paths` from `exec_strategy_windows/mod.rs:331` inside `prepare_live_windows_launch` (mod.rs:326) — that is the real pre-launch chokepoint, NOT `sandbox_prepare.rs`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/data/policy.json` (+`aider`, `+langchain-python`) | config | transform (build-embedded JSON) | existing `claude-code` / `codex` entries (same file) | exact |
| `crates/nono-cli/data/nono-profile.schema.json` (+`windows_interpreters`) | config (schema) | transform | existing optional-field schema entries (same file) | exact |
| `crates/nono-cli/src/profile/mod.rs` (+`windows_interpreters` field) | model | transform (deserialize) | `windows_low_il_broker: bool` field, mod.rs:2237 | exact |
| `crates/nono/src/sandbox/windows.rs` (`validate_launch_paths` interp extension) | service | request-response (gate) | `validate_candidate_path` helper, windows.rs:2160 | exact |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` (SC5 named diag) | service | event-driven (spawn) | `apply_process_handle_to_containment`, launch.rs:247 | exact |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` (R-B3 GATE A wiring) | service | request-response (pre-gate) | `prepare_live_windows_launch`, mod.rs:326-358 | exact |
| `crates/nono-cli/src/cli.rs` (+`--workspace` flag) | config (CLI args) | request-response | `SandboxArgs.allow: Vec<PathBuf>`, cli.rs:1528-1537 | role-match |
| `crates/nono-cli/src/execution_runtime.rs` (workspace→cwd + aider mapping) | controller | request-response | `recommended_builtin_profile`, execution_runtime.rs:98 | exact |
| `crates/nono-cli/src/profile/builtin.rs` (test assertions) | test | request-response | `test_get_builtin_codex` (builtin.rs:163) + `test_embedded_profiles_extend_default` (340) | exact |

## Pattern Assignments

### `crates/nono-cli/data/policy.json` — `aider` + `langchain-python` engine profiles (D-01/D-03/ENG-03)

**Analog:** the `"claude-code"` entry (policy.json:656-730), which is the only built-in that sets `windows_low_il_broker: true`. `"codex"` (796-840) is the leaner shape. `"python-dev"` (877-894) is the **anti-pattern** — do NOT retrofit it (no broker flag, dev-tool `python_runtime` semantics).

The `"profiles"` object starts at **policy.json:598**. Add the two new entries alongside the existing ones (the resolver loads all keys; no registration list to update).

**Exact `windows_low_il_broker` placement** (policy.json:728-729) — this is the field that routes to `BrokerLaunchNoPty`:
```json
      "interactive": true,
      "windows_low_il_broker": true
    },
```

**`codex` minimal shape to model `aider`/`langchain-python` on** (policy.json:796-840):
```json
    "codex": {
      "extends": "default",
      "meta": { "name": "codex", "version": "1.0.0",
                "description": "OpenAI Codex CLI agent", "author": "nono-project" },
      "security": {
        "groups": ["codex_macos", "node_runtime", "rust_runtime", "python_runtime",
                   "linux_sysfs_read", "nix_runtime", "git_config", "unlink_protection"],
        "signal_mode": "isolated", "capability_elevation": false
      },
      "filesystem": { "allow": ["$HOME/.codex"] },
      "network": { "block": false },
      "workdir": { "access": "readwrite" },
      "interactive": true
    },
```

**New `aider` entry to write** (mirrors codex shape + broker flag + NEW interpreter field):
```json
    "aider": {
      "extends": "default",
      "meta": { "name": "aider", "version": "1.0.0",
                "description": "Aider AI pair-programming engine (Python entry point)",
                "author": "nono-project" },
      "security": {
        "groups": ["python_runtime", "git_config", "unlink_protection"],
        "signal_mode": "isolated"
      },
      "filesystem": {},
      "network": { "block": false },
      "workdir": { "access": "readwrite" },
      "windows_low_il_broker": true,
      "windows_interpreters": ["python.exe"]
    },
```
`langchain-python` is the same shape (groups `["python_runtime", "git_config", "unlink_protection"]`, `windows_interpreters: ["python.exe"]`, `windows_low_il_broker: true`, `network.block:false`, `workdir.readwrite`).

> **MUST `"extends": "default"`** — enforced by `test_embedded_profiles_extend_default` (builtin.rs:340-353), which iterates every embedded profile and asserts `def.extends.as_deref() == Some("default")`.

---

### `crates/nono-cli/src/profile/mod.rs` — `windows_interpreters: Vec<String>` field (D-02/Open-Q1)

**Analog:** `windows_low_il_broker: bool` on the `Profile` struct (mod.rs:2229-2237). It is the canonical pattern for a Windows-only, deserialize-on-all-platforms, `#[serde(default)]` field with a doc-comment explaining the Windows-only runtime semantics.

**Exact pattern to mirror** (mod.rs:2229-2237):
```rust
    /// Windows-only. When true, routes non-PTY supervised launches through
    /// `WindowsTokenArm::BrokerLaunchNoPty` instead of `WriteRestricted`.
    /// ... Ignored on Linux and macOS (no-op; deserialize-only).
    #[serde(default)]
    pub windows_low_il_broker: bool,
```

New field (place adjacent to `windows_low_il_broker`):
```rust
    /// Windows-only. Bare exe names of the interpreter(s) this engine's launch
    /// program will spawn (e.g. `python.exe` for a console-script entry point).
    /// Resolved to absolute paths at launch (shebang-of-program ∪ PATH) and
    /// coverage-checked by `validate_launch_paths`. Ignored on Linux/macOS
    /// (deserialize-only). Empty = no interpreter coverage required.
    #[serde(default)]
    pub windows_interpreters: Vec<String>,
```
`Vec<String>` with `#[serde(default)]` is the same pattern as `command_args` (mod.rs:2214), `skipdirs` (2192), `packs` (2203). The struct is `#[derive(Debug, Clone, Default, Serialize)]` (mod.rs:2137) so a `Vec` default is free.

**Schema mirror:** `crates/nono-cli/data/nono-profile.schema.json` must also gain `windows_interpreters` (array of string). Per RESEARCH §"Runtime State Inventory", if typify-generated types regenerate from the schema, keep them in sync after the edit.

---

### `crates/nono/src/sandbox/windows.rs` — extend `validate_launch_paths` to the interpreter (D-07/ENG-02/SC3)

**Analog:** `validate_candidate_path` (windows.rs:2160-2174) — the existing helper that canonicalizes, normalizes, runs `policy.covers_path(...)`, and returns a `NonoError::UnsupportedPlatform` naming the uncovered path. The new interpreter loop should reuse this exact construction style.

**`validate_launch_paths` signature + existing program-coverage check to extend** (windows.rs:2091-2117):
```rust
pub fn validate_launch_paths(
    policy: &WindowsFilesystemPolicy,
    program: &Path,
    current_dir: &Path,
) -> Result<()> {
    ...
    let program = program.canonicalize().unwrap_or_else(|_| program.to_path_buf());
    let program = normalize_windows_path(&program);

    if !policy.covers_path(&program, crate::AccessMode::Read) {
        return Err(NonoError::UnsupportedPlatform(format!(
            "Windows filesystem policy does not cover the executable path required for launch: {}",
            program.display()
        )));
    }
    ...
```

**`validate_candidate_path` pattern to copy for the interpreter loop** (windows.rs:2160-2174):
```rust
fn validate_candidate_path(
    policy: &WindowsFilesystemPolicy,
    candidate: &Path,
    required: crate::AccessMode,
    description: &str,
) -> Result<()> {
    let normalized = normalize_candidate_path(candidate);   // canon + normalize_windows_path, line 2153
    if !policy.covers_path(&normalized, required) {
        return Err(NonoError::UnsupportedPlatform(format!(
            "Windows filesystem policy does not cover the {description} required for launch: {}",
            normalized.display()
        )));
    }
    Ok(())
}
```

**Implementation note for the planner:** the signature must grow an `interpreters: &[PathBuf]` (or similar) parameter, OR a sibling `validate_launch_interpreters` called right after. RESEARCH §"Alternatives Considered" mandates **extending the single gate** (one chokepoint), not adding a parallel gate. The D-07 message must additionally name the concrete fix (`--allow <dir>` / which profile to extend) and `program.display()` (the wrapper that will spawn the uncovered interpreter) — richer than the current bare program message. Use `normalize_candidate_path` (windows.rs:2153) for each interpreter and `policy.covers_path(.., AccessMode::Read)`.

**Call-site thread-through** (`exec_strategy_windows/mod.rs:331`):
```rust
    Sandbox::validate_windows_launch_paths(
        &fs_policy,
        config.resolved_program,
        config.current_dir,
    )?;
```
The resolved interpreter set (declared `windows_interpreters` ∪ shebang(program)) must reach this call — thread it from the loaded profile through `ExecConfig`.

---

### `crates/nono-cli/src/exec_strategy_windows/mod.rs` — R-B3 GATE A pre-launch wiring (D-08/ENG-02/SC4)

**Analog:** `prepare_live_windows_launch` (mod.rs:326-358) — the SOLE CLI pre-launch chokepoint. It already runs `validate_windows_launch_paths` (331) and `validate_windows_command_args` (336) BEFORE the label/DACL apply guard (`AppliedLabelsGuard::snapshot_and_apply`, 358). The R-B3 GATE A belongs here, **after the coverage gates, before the label apply** — so the named refusal fires before any relabel can fail opaquely.

**Exact insertion-point context** (mod.rs:330-358):
```rust
    let fs_policy = Sandbox::windows_filesystem_policy(config.caps);
    Sandbox::validate_windows_launch_paths(&fs_policy, config.resolved_program, config.current_dir)?;
    Sandbox::validate_windows_command_args(&fs_policy, config.resolved_program,
        &config.command[1..], config.current_dir)?;
    // ── R-B3 GATE A (D-08) wires in HERE: refuse non-relabelable workspace ──
    // BEFORE AppliedLabelsGuard::snapshot_and_apply (so failure is named, not opaque)
    let applied_labels = labels_guard::AppliedLabelsGuard::snapshot_and_apply(&fs_policy)?;
```

**Ownership helper to reuse** — `path_is_owned_by_current_user` (windows.rs:1205, returns `Result<bool>`):
```rust
pub fn path_is_owned_by_current_user(path: &Path) -> Result<bool> {
```
> **⚠️ `path_has_write_owner` does NOT exist** (see correction at top). The *stronger* WRITE_OWNER signal RESEARCH.md wants is currently only available **inline** in `try_set_mandatory_label` (windows.rs:1133-1169), which calls `path_is_owned_by_current_user` and emits this directive on the `ERROR_ACCESS_DENIED` branch (windows.rs:1153-1163):
> ```rust
> "The current user lacks WRITE_OWNER (0x00080000) on this path. \
>  Mandatory integrity labels require WRITE_OWNER, which is NOT implicit for path owners. \
>  ... Recommended: run nono from a working directory under your user profile \
>  (e.g. %USERPROFILE%\\nono-poc or %TEMP%\\nono-poc). \
>  Local override: ... `icacls {} /grant <user>:(OI)(CI)F` ..."
> ```
> **Planner decision required:** either (a) extract a `pub fn path_has_write_owner(path) -> Result<bool>` from the inline logic and gate on it (RESEARCH's preference — the correct relabel predictor), or (b) gate on `path_is_owned_by_current_user` (weaker but exists) and lean on the inline directive at apply time. Reuse the existing directive wording (`%USERPROFILE%\nono-poc` / `%TEMP%` / `icacls`) for the D-08 message either way. **No auto-`takeown`** (D-08).

---

### `crates/nono-cli/src/exec_strategy_windows/launch.rs` — SC5 nested-job named diagnostic (Claude's discretion / SC5 / P6)

**Analog:** `apply_process_handle_to_containment` (launch.rs:247-263) and the suspend→assign→resume sequence at launch.rs:2004-2015. The SC5 suspend/assign/terminate machinery is **already implemented**; the only new work is a *named GLE-5 (ERROR_ACCESS_DENIED) diagnostic* distinguishing the foreign-job case.

**Existing assign helper to extend** (launch.rs:247-263) — note it currently does NOT capture `GetLastError()`:
```rust
pub(super) fn apply_process_handle_to_containment(
    containment: &ProcessContainment,
    process: HANDLE,
) -> Result<()> {
    let ok = unsafe { AssignProcessToJobObject(containment.job, process) };
    if ok == 0 {
        return Err(NonoError::SandboxInit(
            "Failed to assign Windows child process to process containment job object".to_string(),
        ));
    }
    Ok(())
}
```
Extend the `ok == 0` branch to read `GetLastError()` and special-case `5` (ERROR_ACCESS_DENIED) with a "child already in a foreign Job Object that disallows breakaway" message (see RESEARCH §"Code Examples", SC5 block).

**Existing fail-secure terminate caller — already correct, do not change** (launch.rs:2004-2015):
```rust
    if let Err(err) = apply_process_handle_to_containment(containment, process.raw()) {
        terminate_suspended_process(process.raw(), "AssignProcessToJobObject failed");
        return Err(err);
    }
    if let Err(err) = apply_resource_limits(containment, limits) {
        terminate_suspended_process(process.raw(), "apply_resource_limits failed");
        return Err(err);
    }
    resume_contained_process(process.raw(), thread.raw())?;
```

**Job creation — already SC5-correct, do NOT add UI limits** (launch.rs:221-222):
```rust
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
```
CONTEXT locks "no UI limits"; do NOT set `JOB_OBJECT_UILIMIT_*` or any breakaway flag (kill-group only; the named/ACL'd/breakaway-denied job is Phase 73).

**CWD = absolute workspace (D-05) — already wired** (launch.rs:1318/1321):
```rust
    let current_dir = normalize_windows_launch_path(config.current_dir);   // strips \\?\
    ...
    let current_dir_u16 = to_u16_null_terminated(&current_dir.to_string_lossy());
```
Passed as `lpCurrentDirectory` to `CreateProcessW` in both broker arms (launch.rs:1533, :1880) and direct arm (:1957, :1973). D-05 reduces to making `--workspace` flow into `config.current_dir`. The `\\?\` strip is already handled by `normalize_windows_launch_path` (launch.rs:1032).

---

### `crates/nono-cli/src/cli.rs` — `--workspace <abs-path>` flag (D-06)

**Analog:** `SandboxArgs.allow: Vec<PathBuf>` (cli.rs:1528-1537) — the established single-path-flag clap pattern with `value_name`, `env`, and `help_heading`.

**Exact pattern to mirror** (cli.rs:1528-1537):
```rust
    /// Allow read+write access to a directory (recursive)
    #[arg(
        long,
        short = 'a',
        value_name = "DIR",
        env = "NONO_ALLOW",
        value_delimiter = ',',
        help_heading = "FILESYSTEM"
    )]
    pub allow: Vec<PathBuf>,
```

New flag (single `Option<PathBuf>`, no delimiter — one workspace):
```rust
    /// Absolute path to the writable workspace. Defaults to the current
    /// directory (canonicalized) when omitted. Set as the child engine's CWD
    /// AND the writable grant (single source of truth).
    #[arg(long, value_name = "DIR", help_heading = "FILESYSTEM")]
    pub workspace: Option<PathBuf>,
```
Add it to `SandboxArgs` (cli.rs:1526) so it flows through `run`/`wrap` via the existing `From`/builder plumbing (cli.rs:2174 maps `allow: args.allow` — add `workspace` analogously). The `--profile` reference is the existing `ProfileRef`/`profile: Option<String>` (cli.rs:1816, :2130) — no change needed (D-04: reuse `run --profile`).

---

### `crates/nono-cli/src/execution_runtime.rs` — workspace→cwd resolution + aider profile hint

**Analog:** `recommended_builtin_profile` (execution_runtime.rs:98-108) — the file_name→builtin-profile match to extend with `aider`.

**Exact match to extend** (execution_runtime.rs:98-108):
```rust
fn recommended_builtin_profile(program: &Path) -> Option<&'static str> {
    let name = program.file_name()?.to_str()?;
    match name {
        "claude" => Some("claude-code"),
        "codex" => Some("codex"),
        "opencode" => Some("opencode"),
        "openclaw" => Some("openclaw"),
        "swival" => Some("swival"),
        _ => None,
    }
}
```
Add `"aider" => Some("aider"),` (and optionally `"aider.exe"` for the Windows file_name). Update the test `recommended_builtin_profile_matches_known_agent_commands` (execution_runtime.rs:780).

**Workspace resolution (D-06)** — `--workspace` OR `std::env::current_dir()?`, then `.canonicalize()?` to absolute, then route into `config.current_dir` (which `launch.rs:1318` consumes as child CWD). The grant must also be expressed for that absolute path (read+write). See RESEARCH §"Pattern 2" for the conceptual flow.

---

### `crates/nono-cli/src/profile/builtin.rs` — profile-load test assertions (ENG-03/SC2)

**Analog:** `test_get_builtin_codex` (builtin.rs:163-172) for the per-profile field assertions, and `test_embedded_profiles_extend_default` (builtin.rs:340-353) which auto-covers the new profiles' `extends: "default"`.

**Exact assertion pattern to copy** (builtin.rs:163-172):
```rust
    fn test_get_builtin_codex() {
        let profile = get_builtin("codex").expect("Profile not found");
        assert_eq!(profile.meta.name, "codex");
        ...
        assert!(profile.security.groups.contains(&"codex_macos".to_string()));
    }
```
Add `test_get_builtin_aider` / `test_get_builtin_langchain_python` asserting `meta.name`, the `python_runtime` group, `windows_low_il_broker == true`, and `windows_interpreters == ["python.exe"]`. The signal-mode sweep `test_all_profiles_signal_mode_resolves` (builtin.rs:606) exercises every profile automatically once added. `list_builtin()` (builtin.rs:14) returns all keys — no registration list to update.

## Shared Patterns

### Fail-secure refusal (NonoError construction)
**Source:** `validate_candidate_path` (windows.rs:2160) + `validate_launch_paths` (windows.rs:2112)
**Apply to:** the interpreter coverage gate AND the R-B3 GATE A.
Construction style for coverage misses is `NonoError::UnsupportedPlatform(format!(...))` naming the uncovered path. For the R-B3 ownership/relabel refusal, the existing precedent is `NonoError::LabelApplyFailed { path, hresult, hint }` (windows.rs:1074, :1177) — but for a *pre-launch* gate RESEARCH §"Code Examples" uses `NonoError::SandboxInit(format!(...))` (the same variant `create_process_containment` uses, launch.rs:208). Planner: pick `SandboxInit` for the pre-gate (no hresult yet) to match the launch-time error family.

### Path comparison (CLAUDE.md footgun #1 — NON-NEGOTIABLE)
**Source:** `WindowsFilesystemPolicy::covers_path` (sandbox/mod.rs:504-530)
**Apply to:** every workspace/exe/interpreter path check.
`covers_path` already does component-wise `windows_paths_start_with_case_insensitive` (mod.rs:522) — NEVER use string `starts_with`. Reuse `covers_path`; do not hand-roll.

### `\\?\` verbatim-prefix strip
**Source:** `normalize_windows_launch_path` (launch.rs:1032), `normalize_windows_path` (windows.rs:380), `normalize_candidate_path` (windows.rs:2153)
**Apply to:** the child CWD (already done at launch.rs:1318) and any newly-resolved interpreter path before `covers_path`. Use `normalize_candidate_path` for interpreter resolution.

### Windows-only `#[serde(default)]` profile field
**Source:** `windows_low_il_broker: bool` (mod.rs:2237)
**Apply to:** the new `windows_interpreters: Vec<String>` field — same `#[serde(default)]`, same Windows-only-runtime doc-comment convention, same deserialize-on-all-platforms guarantee.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `.planning/phases/71-.../71-HUMAN-UAT.md` | doc (UAT script) | n/a | New artifact; not source code. Model on prior phase HUMAN-UAT docs (e.g. Phase 58/60). SC1 real-Win11 Aider end-to-end gate — not automatable. |
| shebang-reader for console-script `.exe` (Open-Q1 assist) | utility | file-I/O | No existing in-tree shebang/PE-script reader. NEW small utility (read the distlib launcher's embedded shebang). Pure diagnostic assist — must NOT auto-grant (D-07). Fallback is the explicit `windows_interpreters` declaration, so a wrong/absent shebang degrades to a correct fail-secure refusal, never silent partial confinement. |

## Metadata

**Analog search scope:** `crates/nono/src/sandbox/` (windows.rs, mod.rs), `crates/nono-cli/src/` (exec_strategy_windows/, profile/, cli.rs, execution_runtime.rs), `crates/nono-cli/data/` (policy.json, schema).
**Files scanned:** ~10 (targeted reads, no full-file re-reads).
**Pattern extraction date:** 2026-06-13
**Key invariant:** This is a productionization phase — every primitive exists and is spike-003/Phase-51/60 proven. New code is confined to: 2 JSON profile entries, 1 schema + 1 struct field, 1 gate extension, 1 pre-gate wiring, 1 named diagnostic, 1 CLI flag, 1 profile-hint mapping, test assertions. No rewrites.
