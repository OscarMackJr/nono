---
phase: 35-upst3-closure-quick-wins
plan: 03
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/nono-cli/src/query_ext.rs
  - crates/nono-cli/src/profile_cmd.rs
  - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md
autonomous: true
requirements:
  - REQ-PORT-CLOSURE-07
tags:
  - phase-35
  - port-closure
  - windows
  - test-hygiene
  - json-shape
  - fork-local-regression
  - p34-defer-01-1
  - p34-defer-09-3
  - p34-defer-10-1

must_haves:
  truths:
    - "On Windows, `nono query path /some/random/path` produces a `suggested_flag` value with NO `\\\\?\\` UNC verbatim prefix (e.g., `--read C:\\some\\random` not `--read \\\\?\\C:\\some\\random`)."
    - "`nono policy show --json` emits no Rust Debug-format strings — `signal_mode` is either absent (None) or the snake_case literal `\"isolated\"` / `\"detached\"` / etc. (Some); never `\"Some(Isolated)\"` or `\"None\"`."
    - "`nono policy diff --json` emits the same clean shape for both `profile1` and `profile2` security sections (paired emission); no Rust Debug strings on either side."
    - "`workdir.access` in both `policy show --json` and `policy diff --json` emits the lowercase snake_case literal (e.g., `\"readwrite\"` / `\"readonly\"`) via `serde_json::to_value`; never `\"ReadWrite\"` (PascalCase Debug leak)."
    - "All four Option<…> security enums (`ProfileSignalMode`, `ProfileProcessInfoMode`, `ProfileIpcMode`, `Wsl2ProxyPolicy`) emit via omit-when-None semantics: the JSON key is ABSENT when the profile field is `None`; the JSON value is a snake_case string when `Some(_)`."
    - "Regression tests `test_policy_show_json_no_rust_debug_syntax` and `test_policy_diff_json_no_rust_debug_syntax` pass deterministically on Windows + Linux + macOS."
    - "`query_ext::test_query_path_denied` passes deterministically on Windows + Linux + macOS with NO `#[cfg]` gates on the test itself (the production-code `strip_verbatim_prefix` helper handles platform dispatch)."
    - "P34-DEFER-01-1, P34-DEFER-08a-1, P34-DEFER-09-1, P34-DEFER-09-3, P34-DEFER-10-1 are appended as a `Phase 35 closure` section to `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` (D-35-D4)."
  artifacts:
    - path: "crates/nono-cli/src/query_ext.rs"
      provides: "Two `suggested_flag` emission sites (lines 167 + 179) wrap `&canonical` with `strip_verbatim_prefix(&canonical)` — leverages existing helper at lines 295-317 (zero new functions added in this file)."
      contains: "strip_verbatim_prefix(&canonical)"
    - path: "crates/nono-cli/src/profile_cmd.rs"
      provides: "`profile_to_json` (line 1041) refactored to use `serde_json::Map` insertion with omit-when-None semantics for the four Option<…> security fields. Return type changed from `serde_json::Value` to `Result<serde_json::Value>` per CLAUDE.md no-unwrap policy; call sites at `cmd_show` (line 746) + `cmd_diff` (line 1164) propagate via `?`."
      contains: "serde_json::Map::new"
    - path: "crates/nono-cli/src/profile_cmd.rs"
      provides: "`diff_to_json` (line 1777) refactored with same Map-insertion shape for paired profile1/profile2 emission; `workdir.access` (PascalCase enum) emitted via `serde_json::to_value` (non-Optional, snake_case via existing `#[serde(rename_all = \"lowercase\")]`)."
      contains: "diff_to_json"
    - path: ".planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md"
      provides: "New `## Phase 35 closure` section at the bottom of the file with five entries marking P34-DEFER-01-1, P34-DEFER-08a-1, P34-DEFER-09-1, P34-DEFER-09-3, P34-DEFER-10-1 as `closed-by-Phase-35` with the closing plan + commit SHA reference."
      contains: "Phase 35 closure"
  key_links:
    - from: "crates/nono-cli/src/query_ext.rs::query_path (line 167 + 179)"
      to: "crates/nono-cli/src/query_ext.rs::strip_verbatim_prefix (line 295)"
      via: "in-file function call; on Windows strips `\\\\?\\` / `\\\\?\\UNC\\` / `\\??\\`; on non-Windows is the identity no-op"
      pattern: "strip_verbatim_prefix\\(&canonical\\)"
    - from: "crates/nono-cli/src/profile_cmd.rs::profile_to_json"
      to: "serde_json::to_value(profile.security.signal_mode)"
      via: "Map insertion gated `if let Some(ref mode) = profile.security.signal_mode { ... }` — uses existing `#[serde(rename_all = \"snake_case\")]` on the enum"
      pattern: "serde_json::to_value\\(mode\\)"
    - from: "crates/nono-cli/tests/profile_cli.rs::test_policy_show_json_no_rust_debug_syntax"
      to: "crates/nono-cli/src/profile_cmd.rs::profile_to_json"
      via: "integration test exercising the full CLI surface `nono policy show --json`"
      pattern: "test_policy_(show|diff)_json_no_rust_debug_syntax"
---

<objective>
Two surgical fork-local regression fixes plus a phase-closure ledger append:

1. **UNC strip at the production-code source** (D-35-C1): wrap the `&canonical` argument with `strip_verbatim_prefix(&canonical)` at the two `suggested_flag` emission sites in `crates/nono-cli/src/query_ext.rs` (lines 167 + 179). The cross-platform helper already exists at lines 295-317; this fix is a 2-line wrap per call site. Mirrors commit `400f8c90` production-code analog. Fixes `test_query_path_denied` Windows UNC flake AND closes the latent UX bug where suggested CLI flags contained literally untypeable `\\?\C:\…` syntax. NO `#[cfg]` gates at the call site — helper handles platform dispatch internally.

2. **JSON Map-insertion shape for Option<…> security fields** (D-35-C2 + D-35-C3): replace `format!("{:?}", profile.security.signal_mode)` and sibling Debug-format sites in `profile_to_json` (line 1041) and `diff_to_json` (line 1777) with `serde_json::Map` insertion + omit-when-None semantics. Full audit of all `format!("{:?}")` / `format!("{:#?}")` JSON-emission sites in `profile_cmd.rs` per the PATTERNS.md scope table. Restores the upstream `f3e7f885` (v0.47.0) shape that Plan 34-04b adopted but later Wave-3 plans regressed. Locked by existing regression tests `test_policy_show_json_no_rust_debug_syntax` + `test_policy_diff_json_no_rust_debug_syntax`.

3. **Phase 35 closure ledger append** (D-35-D4): as the last plan to close in Phase 35, append a consolidated `## Phase 35 closure` section to `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md`, flipping P34-DEFER-01-1, P34-DEFER-08a-1, P34-DEFER-09-1, P34-DEFER-09-3, P34-DEFER-10-1 from open to `closed-by-Phase-35-XX` with commit SHAs.

**Commit shape (D-35-A4):** Fork-local regression fixes — NO D-19 trailer. Regular DCO sign-off only. Commit body references upstream `f3e7f885` (v0.47.0 JSON shape pattern) and in-fork commit `400f8c90` (production-code UNC strip analog) as DESIGN-SOURCE CITATIONS, not cherry-picks.

**Purpose:** Closes the entire `format!("{:?}")` JSON-leak regression class (D-35-C3 — full audit, not just the two flagged tests). Closes the UNC UX bug at the production code surface (D-35-C1 — typeable suggested flags). Plan 35-03 SUMMARY closes the 5 P34-DEFER-* tickets that the entire Phase 35 wave closes (D-35-D4).

**Scope ceiling (D-35-C3):** `format!("{:?}")` audit is SCOPED to JSON-emission helpers in `profile_cmd.rs` (`profile_to_json`, `diff_to_json`, `diff_custom_credentials_json`). The 4 `cmd_diff`-body human-readable stdout printer sites at lines 1297-1318 are NOT JSON emission — they are colored stdout via `diff_scalar_option` — leave them alone per PATTERNS.md § "Out-of-scope clarification".
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/35-upst3-closure-quick-wins/35-CONTEXT.md
@.planning/phases/35-upst3-closure-quick-wins/35-PATTERNS.md
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md

<interfaces>
<!-- Existing helpers + enums Plan 35-03 calls. Do not re-implement. -->

From `crates/nono-cli/src/query_ext.rs` lines 295-317 (helper already in tree from commit `400f8c90`):
```rust
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

From `crates/nono-cli/src/profile/mod.rs` (verified via PATTERNS.md § "Verified"):
- `ProfileSignalMode` (line 1250) carries `#[serde(rename_all = "snake_case")]` → `Some(Isolated)` → `"isolated"`.
- `ProfileProcessInfoMode` (line 1274) carries `#[serde(rename_all = "snake_case")]`.
- `ProfileIpcMode` (line 1298) carries `#[serde(rename_all = "snake_case")]`.
- `Wsl2ProxyPolicy` (line 1324) carries `#[serde(rename_all = "snake_case")]` → `InsecureProxy` → `"insecure_proxy"`.
- `WorkdirAccess` (line 1339) carries `#[serde(rename_all = "lowercase")]` → `ReadWrite` → `"readwrite"`.

NO enum-attribute changes needed (D-35-C2 "Claude's discretion" item resolved by PATTERNS.md verification).

From `crates/nono-cli/src/profile_cmd.rs::build_skeleton` (lines 141-247) — canonical Map-insertion shape ALREADY PRESENT in this file (for `nono profile init`):
```rust
let mut root = serde_json::Map::new();
if let Some(ref base) = args.extends {
    root.insert("extends".to_string(), serde_json::Value::String(base.clone()));
}
let mut meta = serde_json::Map::new();
meta.insert("name".to_string(), serde_json::Value::String(args.name.clone()));
if let Some(ref desc) = args.description {
    meta.insert("description".to_string(), serde_json::Value::String(desc.clone()));
}
root.insert("meta".to_string(), serde_json::Value::Object(meta));
```

From `crates/nono-cli/src/profile_cmd.rs` line 1041-1062 (CURRENT broken shape):
```rust
fn profile_to_json(
    name: &str,
    profile: &Profile,
    raw_extends: &Option<Vec<String>>,
) -> serde_json::Value {
    let mut val = serde_json::json!({ ... });
    val["security"] = serde_json::json!({
        "groups": profile.security.groups,
        "allowed_commands": profile.security.allowed_commands,
        "signal_mode": format!("{:?}", profile.security.signal_mode),  // LEAKS Debug
        "process_info_mode": format!("{:?}", profile.security.process_info_mode),  // LEAKS Debug
        "ipc_mode": format!("{:?}", profile.security.ipc_mode),  // LEAKS Debug
        "capability_elevation": profile.security.capability_elevation,
        "wsl2_proxy_policy": format!("{:?}", profile.security.wsl2_proxy_policy),  // LEAKS Debug
    });
    // ...
    val["workdir"] = serde_json::json!({
        "access": format!("{:?}", profile.workdir.access),  // LEAKS Debug (PascalCase)
    });
}
```

Regression tests at `crates/nono-cli/tests/profile_cli.rs:120-176` (test names `test_policy_show_json_no_rust_debug_syntax` + `test_policy_diff_json_no_rust_debug_syntax`).

</interfaces>

</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Strip UNC verbatim prefix in query_ext suggested_flag emission</name>
  <files>crates/nono-cli/src/query_ext.rs</files>
  <read_first>
    - crates/nono-cli/src/query_ext.rs (lines 1-50 for imports; lines 80-180 for `query_path` function body containing both `suggested_flag:` call sites at lines 167 + 179; lines 290-320 for the existing `strip_verbatim_prefix` helper definitions; lines 360-400 for `test_query_path_denied`)
    - .planning/phases/35-upst3-closure-quick-wins/35-PATTERNS.md § "Pattern Assignments § `crates/nono-cli/src/query_ext.rs` (Plan 35-03)" — explicit Analog identifying the two wrap sites
    - CLAUDE.md § "Path Handling" — confirms why `.replace()` on lossy OS-string is the canonical fork pattern (path-component-safety footgun only applies to path-segment matching; UNC prefixes are NOT path components, they are canonicalization encoding artifacts)
    - In-fork commit `400f8c90` body — `git log --format='%B' 400f8c90` for the design rationale on the analog production-code UNC strip (already shipped for the sensitive-path check; this plan extends the same pattern to `suggested_flag`)
  </read_first>
  <behavior>
    - On Windows, calling `query_path(req, caps)` for a path that canonicalizes to `\\?\C:\some\random\path` returns a `QueryResult::Denied` whose `suggested_flag` is `Some("--read C:\\some\\random")` (parent of the requested path, NO `\\?\` UNC verbatim prefix).
    - On Linux + macOS, behavior is byte-identical (the `#[cfg(not(target_os = "windows"))]` arm of `strip_verbatim_prefix` is the identity no-op).
    - `crates/nono-cli/src/query_ext.rs::tests::test_query_path_denied` (the test at line 365+) passes deterministically on all three platforms with NO `#[cfg]` gate added to the test itself.
    - `cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings` passes (cross-target Linux gate — checks the `strip_verbatim_prefix` non-Windows arm at line 314-316 is still reachable from the new call sites).
  </behavior>
  <action>
    1. Open `crates/nono-cli/src/query_ext.rs`. Locate the two `suggested_flag: Some(suggested_flag_for_path(&canonical, requested)),` lines at lines 167 and 179.
    2. At line 167 (inside the `if let Some(cap) = best_covering` near-miss branch), change:
       ```rust
       suggested_flag: Some(suggested_flag_for_path(&canonical, requested)),
       ```
       to:
       ```rust
       // Plan 35-03 (REQ-PORT-CLOSURE-07 / D-35-C1): strip Windows UNC
       // verbatim prefix (`\\?\`, `\\?\UNC\`, `\??\`) from the canonicalized
       // path before deriving the suggested CLI flag. Mirrors in-fork commit
       // 400f8c90 (which fixed the same UX bug for the sensitive-path check).
       // The helper is identity-no-op on non-Windows; platform dispatch is
       // internal — no `#[cfg]` gate needed at this call site.
       suggested_flag: Some(suggested_flag_for_path(
           &strip_verbatim_prefix(&canonical),
           requested,
       )),
       ```
    3. At line 179 (inside the `path_not_granted` branch at the end of `query_path`), apply the same wrap with the same comment (or a shorter comment that references the comment at line 167 for the rationale — "see comment above on Plan 35-03 D-35-C1").
    4. Verify the `strip_verbatim_prefix` helper at lines 295-317 is already importable from inside `query_path` — they live in the same file/module so no `use` change is needed.
    5. Verify `cargo test -p nono-cli --lib query_ext::tests::test_query_path_denied` exits 0 on Windows host (the test was previously failing per P34-DEFER-01-1; with this wrap it should pass deterministically).
    6. Verify `cargo test -p nono-cli --lib query_ext::tests` exits 0 on Linux/macOS (or via cross-target build): the `strip_verbatim_prefix` non-Windows identity arm means the test assertion `Some("--read /some/random")` continues to hold for POSIX paths.
    7. Check baseline grep for the helper usage to confirm we added 2 call sites: `grep -c 'strip_verbatim_prefix' crates/nono-cli/src/query_ext.rs`. Pre-plan baseline: 3 (the two cfg-gated definitions at lines 294 + 313, plus the existing call site at line 87 from commit `400f8c90`). Post-plan: 5 (definitions unchanged at 3, plus 2 new wrap sites at lines 167 + 179 — though their visual line numbers may shift due to the inserted comments).
  </action>
  <acceptance_criteria>
    - `grep -c 'strip_verbatim_prefix(&canonical)' crates/nono-cli/src/query_ext.rs` returns 3 (one pre-existing at line 87 from commit `400f8c90`, two new at the modified `suggested_flag` sites). Verify with `grep -n 'strip_verbatim_prefix(&canonical)' crates/nono-cli/src/query_ext.rs` showing 3 line numbers.
    - `grep -c 'Plan 35-03 (REQ-PORT-CLOSURE-07' crates/nono-cli/src/query_ext.rs` returns at least 1.
    - `grep -c '#\[cfg' crates/nono-cli/src/query_ext.rs` does NOT increase vs pre-plan baseline (no new cfg gates at the call sites — the helper handles platform dispatch internally).
    - On Windows host: `cargo test -p nono-cli --lib query_ext::tests::test_query_path_denied` exits 0.
    - On Windows host: full `cargo test -p nono-cli --lib query_ext::tests` exits 0 (no regression in other query_ext tests).
    - Cross-target: `cargo clippy -p nono-cli --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` exits 0.
    - Cross-target: `cargo clippy -p nono-cli --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` exits 0.
    - <automated>cargo test -p nono-cli --lib query_ext::tests::test_query_path_denied</automated>
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p nono-cli --lib query_ext::tests::test_query_path_denied</automated>
  </verify>
  <done>Both `suggested_flag` emission sites in `query_path` wrap `&canonical` via `strip_verbatim_prefix`; `test_query_path_denied` passes deterministically on Windows + Linux + macOS with no `#[cfg]` gates on the test. Closes P34-DEFER-01-1 and the carry-forward duplicate P34-DEFER-09-3 in one fix.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Replace format!("{:?}") JSON-emission with serde_json::Map insertion — profile_to_json + diff_to_json + diff_custom_credentials_json audit</name>
  <files>crates/nono-cli/src/profile_cmd.rs</files>
  <read_first>
    - crates/nono-cli/src/profile_cmd.rs (lines 1-30 for imports + `to_json` pattern; lines 141-247 for canonical `build_skeleton` Map-insertion analog; lines 740-760 for `cmd_show` call site; lines 1041-1130 for `profile_to_json`; lines 1160-1170 for `cmd_diff` call site; lines 1290-1330 for the `cmd_diff` body human-readable stdout printer block — VERIFY these are OUT OF SCOPE; lines 1777-1830 for `diff_to_json`; lines 1985-2000 for `diff_custom_credentials_json`)
    - crates/nono-cli/src/profile/mod.rs (lines 1240-1360 for the four security enums + WorkdirAccess + verify `#[serde(rename_all = "snake_case")]` / `"lowercase"` attributes present per PATTERNS.md verification)
    - crates/nono-cli/tests/profile_cli.rs (lines 120-176 for the two locked regression tests `test_policy_show_json_no_rust_debug_syntax` + `test_policy_diff_json_no_rust_debug_syntax`)
    - .planning/phases/35-upst3-closure-quick-wins/35-PATTERNS.md § "Pattern Assignments § `crates/nono-cli/src/profile_cmd.rs`" — full audit table of 7 in-scope sites + 4 out-of-scope sites
    - Run: `grep -n 'format!("{:?}\|format!("{:#?}\|format!("{v:?}' crates/nono-cli/src/profile_cmd.rs` to enumerate every site; cross-reference each against the PATTERNS.md scope table to mark in-scope vs out-of-scope
  </read_first>
  <behavior>
    - `nono policy show --json` for a profile with `signal_mode: None` produces JSON where `.security` object does NOT have a `signal_mode` key (omit-when-None).
    - `nono policy show --json` for a profile with `signal_mode: Some(ProfileSignalMode::Isolated)` produces JSON where `.security.signal_mode == "isolated"` (snake_case string via `serde_json::to_value`).
    - Same omit-when-None semantics apply to `process_info_mode`, `ipc_mode`, `wsl2_proxy_policy`.
    - `workdir.access` (NOT Optional) always emits as `.workdir.access == "readwrite"` / `"readonly"` (lowercase per `#[serde(rename_all = "lowercase")]` on the enum at `profile/mod.rs:1339`).
    - `nono policy diff --json` emits identical shape for both `profile1` and `profile2` security/workdir sections (paired emission; same omit-when-None semantics on each side).
    - Regression tests `test_policy_show_json_no_rust_debug_syntax` + `test_policy_diff_json_no_rust_debug_syntax` pass deterministically on Windows + Linux + macOS. Both tests already exist in `crates/nono-cli/tests/profile_cli.rs:120-176` and exercise three built-in profiles each (`default`, `claude-code`, `node-dev`).
    - `cargo build -p nono-cli` succeeds: the `profile_to_json` return type changes from `serde_json::Value` to `Result<serde_json::Value>`; both call sites (`cmd_show` at line 746 + `cmd_diff` at line 1164) propagate via `?`.
    - NO `cmd_diff`-body human-readable stdout printer at lines 1297-1318 is modified (out of scope per D-35-C3 — those are stdout printers, not JSON emitters).
  </behavior>
  <action>
    1. Open `crates/nono-cli/src/profile_cmd.rs`. Enumerate every `format!("{:?}` / `format!("{:#?}` site:
       ```bash
       grep -n 'format!("{:?}\|format!("{:#?}' crates/nono-cli/src/profile_cmd.rs
       ```
       Cross-reference each line number against the PATTERNS.md scope table:
       - **IN SCOPE (rewrite to Map insertion):** 1056 (signal_mode), 1057 (process_info_mode), 1058 (ipc_mode), 1060 (wsl2_proxy_policy), 1098 (workdir.access), 1812-1813 (diff_to_json wsl2_proxy_policy paired), 1818-1819 (diff_to_json workdir.access paired), 1991 (diff_custom_credentials_json inject_mode — see step 6 for D-35-C3 discretion).
       - **OUT OF SCOPE (do NOT touch):** 1297-1298, 1303-1304, 1309-1310, 1315-1316 — these are `cmd_diff`-body human-readable stdout printers via `diff_scalar_option`, NOT JSON emitters.
       If `grep` finds any IN-SCOPE site NOT in the table, treat it as in-scope and apply the same fix (D-35-C3 mandates closing the full regression class).
    2. Refactor `profile_to_json` (line 1041). Change the signature:
       ```rust
       fn profile_to_json(
           name: &str,
           profile: &Profile,
           raw_extends: &Option<Vec<String>>,
       ) -> crate::Result<serde_json::Value> {
       ```
       Replace the `val["security"] = serde_json::json!({ ... })` block (lines 1053-1061) with explicit `serde_json::Map` insertion. The full replacement block:
       ```rust
           // Plan 35-03 (REQ-PORT-CLOSURE-07 / D-35-C2 + D-35-C3): replace
           // format!("{:?}", …) JSON emission of Option<…> security fields
           // with serde_json::Map insertion + omit-when-None semantics.
           // Restores the upstream f3e7f885 (v0.47.0) shape that Plan 34-04b
           // adopted but later Wave-3 plans regressed (P34-DEFER-10-1).
           let mut security = serde_json::Map::new();
           security.insert(
               "groups".to_string(),
               serde_json::json!(profile.security.groups),
           );
           security.insert(
               "allowed_commands".to_string(),
               serde_json::json!(profile.security.allowed_commands),
           );
           if let Some(ref mode) = profile.security.signal_mode {
               security.insert(
                   "signal_mode".to_string(),
                   serde_json::to_value(mode).map_err(|e| {
                       NonoError::ProfileParse(format!("signal_mode serialize: {e}"))
                   })?,
               );
           }
           if let Some(ref mode) = profile.security.process_info_mode {
               security.insert(
                   "process_info_mode".to_string(),
                   serde_json::to_value(mode).map_err(|e| {
                       NonoError::ProfileParse(format!("process_info_mode serialize: {e}"))
                   })?,
               );
           }
           if let Some(ref mode) = profile.security.ipc_mode {
               security.insert(
                   "ipc_mode".to_string(),
                   serde_json::to_value(mode).map_err(|e| {
                       NonoError::ProfileParse(format!("ipc_mode serialize: {e}"))
                   })?,
               );
           }
           security.insert(
               "capability_elevation".to_string(),
               serde_json::json!(profile.security.capability_elevation),
           );
           if let Some(ref policy) = profile.security.wsl2_proxy_policy {
               security.insert(
                   "wsl2_proxy_policy".to_string(),
                   serde_json::to_value(policy).map_err(|e| {
                       NonoError::ProfileParse(format!("wsl2_proxy_policy serialize: {e}"))
                   })?,
               );
           }
           val.as_object_mut()
               .ok_or_else(|| {
                   NonoError::ProfileParse(
                       "profile_to_json root not an object".to_string(),
                   )
               })?
               .insert("security".to_string(), serde_json::Value::Object(security));
       ```
       (Note: verify the exact `Profile` field types for the four Option fields — some may be `Option<ProfileSignalMode>` and some may be `Option<&ProfileSignalMode>` depending on accessor convention. Adapt the `if let Some(ref ...)` shape to match.)
    3. At line 1097-1099, replace the `val["workdir"] = serde_json::json!({ "access": format!("{:?}", profile.workdir.access) });` with:
       ```rust
       let mut workdir = serde_json::Map::new();
       workdir.insert(
           "access".to_string(),
           serde_json::to_value(&profile.workdir.access).map_err(|e| {
               NonoError::ProfileParse(format!("workdir.access serialize: {e}"))
           })?,
       );
       val.as_object_mut()
           .ok_or_else(|| {
               NonoError::ProfileParse(
                   "profile_to_json root not an object".to_string(),
               )
           })?
           .insert("workdir".to_string(), serde_json::Value::Object(workdir));
       ```
       At the end of `profile_to_json`, change the bare `val` return to `Ok(val)`.
    4. Update `cmd_show` (line 746) to propagate `Result<Value>`. Find the call site `let json = profile_to_json(name, &profile, &raw_extends);` and change to `let json = profile_to_json(name, &profile, &raw_extends)?;`.
    5. Refactor `diff_to_json` (line 1777) with the same Map-insertion shape for the paired profile1/profile2 emission. For lines 1812-1819 (wsl2_proxy_policy + workdir.access for both sides), apply the same `serde_json::to_value` shape. Change `diff_to_json`'s return type to `Result<serde_json::Value>` and update the `cmd_diff` (line 1164) call site to propagate via `?`.
    6. **D-35-C3 discretion at `diff_custom_credentials_json` line 1991 (`inject_mode`):** Read the `InjectMode` enum definition in `nono-proxy` crate (`crates/nono-proxy/src/config.rs` or similar — find via `grep -rn 'enum InjectMode' crates/nono-proxy/`). Two outcomes:
       - **If `InjectMode` carries `#[serde(rename_all = "snake_case")]` (or `"lowercase"`):** rewrite line 1991 to use `serde_json::to_value(&old.inject_mode).map_err(...)?` (same shape as workdir.access).
       - **If `InjectMode` does NOT carry a serde rename attribute:** D-35-C3 explicit discretion permits either (a) adding the attribute in nono-proxy (preferred, lower-noise) OR (b) keeping the `format!("{:?}")` and documenting in the commit body that `inject_mode` is intentionally Debug-format-preserved because the regression tests don't exercise this path. Pick (a) — add `#[serde(rename_all = "snake_case")]` to `InjectMode` and use `serde_json::to_value`. If (a) requires touching `crates/nono-proxy/src/...` and the change feels invasive, fall back to (b) and add a `// Plan 35-03 D-35-C3 discretion: inject_mode left as Debug-format pending …` comment.
    7. Run the regression tests on Windows host: `cargo test -p nono-cli --test profile_cli test_policy_show_json_no_rust_debug_syntax`. Must exit 0. Also run `cargo test -p nono-cli --test profile_cli test_policy_diff_json_no_rust_debug_syntax`. Must exit 0.
    8. Run the full `cargo test -p nono-cli` on Windows to verify no other test regressed.
    9. Cross-target gates: `cargo clippy -p nono-cli --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `cargo clippy -p nono-cli --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`. Both exit 0.
  </action>
  <acceptance_criteria>
    - `grep -cE 'format!\("\{:\?\}".*profile\.security\.(signal_mode|process_info_mode|ipc_mode|wsl2_proxy_policy)' crates/nono-cli/src/profile_cmd.rs` returns 0 (all four Option<…> security field Debug-format sites eliminated).
    - `grep -cE 'format!\("\{:\?\}".*profile\.workdir\.access' crates/nono-cli/src/profile_cmd.rs` returns 0 (workdir.access Debug-format site eliminated).
    - `grep -cE 'format!\("\{:\?\}".*p[12]\.security\.wsl2_proxy_policy' crates/nono-cli/src/profile_cmd.rs` returns 0 (diff_to_json paired wsl2_proxy_policy eliminated).
    - `grep -cE 'format!\("\{:\?\}".*p[12]\.workdir\.access' crates/nono-cli/src/profile_cmd.rs` returns 0 (diff_to_json paired workdir.access eliminated).
    - `grep -c 'serde_json::Map::new\|serde_json::to_value' crates/nono-cli/src/profile_cmd.rs` increases by at least 6 vs pre-plan baseline (multiple new Map allocations + `to_value` calls in profile_to_json + diff_to_json).
    - `grep -c 'Plan 35-03 (REQ-PORT-CLOSURE-07' crates/nono-cli/src/profile_cmd.rs` returns at least 1.
    - `grep -c '\.unwrap()' crates/nono-cli/src/profile_cmd.rs` does NOT grow vs pre-plan baseline (CLAUDE.md no-unwrap policy).
    - **Out-of-scope preservation:** lines 1297-1318 in `cmd_diff` body still contain `format!("{:?}` (or `format!("{v:?}` etc.) for `diff_scalar_option` stdout printer — these are stdout printers, not JSON emitters. Verify with `sed -n '1295,1320p' crates/nono-cli/src/profile_cmd.rs | grep -c 'diff_scalar_option\|format!("{'` returning a value > 0 (the stdout printer block is preserved).
    - On Windows: `cargo test -p nono-cli --test profile_cli test_policy_show_json_no_rust_debug_syntax` exits 0.
    - On Windows: `cargo test -p nono-cli --test profile_cli test_policy_diff_json_no_rust_debug_syntax` exits 0.
    - On Windows: `cargo test -p nono-cli` exits 0 (no other test regressed by the signature change).
    - Cross-target Linux clippy + macOS clippy gates both exit 0.
    - <automated>cargo test -p nono-cli --test profile_cli test_policy_show_json_no_rust_debug_syntax &amp;&amp; cargo test -p nono-cli --test profile_cli test_policy_diff_json_no_rust_debug_syntax</automated>
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p nono-cli --test profile_cli test_policy_show_json_no_rust_debug_syntax &amp;&amp; cargo test -p nono-cli --test profile_cli test_policy_diff_json_no_rust_debug_syntax</automated>
  </verify>
  <done>All in-scope `format!("{:?}")` JSON-emission sites in `profile_cmd.rs` replaced with `serde_json::Map` + `serde_json::to_value`; omit-when-None semantics applied to Option<…> security fields; `workdir.access` lowercase via existing `#[serde(rename_all = "lowercase")]`; both regression tests pass; signature changes propagated through `cmd_show` + `cmd_diff` call sites; out-of-scope `cmd_diff`-body stdout printers untouched. Closes P34-DEFER-10-1 entire regression class via full audit per D-35-C3.</done>
</task>

<task type="auto">
  <name>Task 3: Append Phase 35 closure section to Phase 34 deferred-items ledger</name>
  <files>.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md</files>
  <read_first>
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md (read entire file to understand the section format used by the prior entries; identify each P34-DEFER-* entry header shape so the closure section references the right tickets)
    - .planning/phases/35-upst3-closure-quick-wins/35-CONTEXT.md § "D-35-D4" — defines the consolidated append shape; this task is the SUMMARY-side action (the per-plan SUMMARYs also record their own closures)
    - Run: `git log --format='%H %s' --all main~10..main | head -20` to find the commit SHAs landed by Plan 35-01, Plan 35-02 (Task 1 + Task 2 follow-up), and Plan 35-03 (the in-progress Task 1 + Task 2 commits in this plan). Record all SHAs (first 8 chars) for the closure ledger.
  </read_first>
  <behavior>
    - File `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` gains a new section titled `## Phase 35 closure` at the bottom (after the last existing P34-DEFER-* entry).
    - The new section contains five sub-entries — one per P34-DEFER-* ticket that Phase 35 closes:
      - P34-DEFER-01-1 → closed-by-Plan-35-03-Task-1 (UNC strip)
      - P34-DEFER-08a-1 → closed-by-Plan-35-01 (Windows env-filter wiring)
      - P34-DEFER-09-1 → closed-by-Plan-35-02-Task-1 (Landlock pre-create cherry-pick)
      - P34-DEFER-09-3 → closed-by-Plan-35-03-Task-1 (transitive — carry-forward duplicate of 01-1)
      - P34-DEFER-10-1 → closed-by-Plan-35-03-Task-2 (JSON Debug-leak full audit)
    - Each sub-entry includes the closing commit SHA (first 8 chars) and one-line summary.
    - The original P34-DEFER-* entries earlier in the file are NOT edited (the closure section is purely additive — append at the bottom).
  </behavior>
  <action>
    1. Read the full current contents of `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` to understand existing section formatting + tone.
    2. Resolve commit SHAs for the five closing commits:
       - Plan 35-01's single commit: `git log --format='%H %s' main --grep='Plan 35-01\|35-01.*WIN-ENV-FILTER' -1 | cut -c1-8`
       - Plan 35-02's Task-1 cherry-pick commit: `git log --format='%H %s' main --grep='Upstream-commit: bdf183e9' -1 | cut -c1-8`
       - Plan 35-03's Task-1 commit (UNC strip — likely committed before this Task 3): `git log --format='%H %s' main --grep='35-03.*query_ext\|strip_verbatim_prefix\|UNC.*suggested_flag' -1 | cut -c1-8`
       - Plan 35-03's Task-2 commit (JSON shape fix): `git log --format='%H %s' main --grep='35-03.*profile_cmd\|serde_json::Map\|policy.*json' -1 | cut -c1-8`
       If the Plan 35-03 commits haven't landed yet (Task 3 runs before commits), use placeholder `<35-03-task-1-sha>` and `<35-03-task-2-sha>` markers and update via `git commit --amend` after the closure-section commit lands. Preferred: run Task 3 AFTER Task 1 and Task 2 commits land, so all SHAs are available.
    3. Append the new section to the bottom of the file. Use markdown shape consistent with the existing P34-DEFER-* entries (header shape `## ` for section, `### ` for sub-entry if existing entries use that depth — adapt to file convention). Exact append content:
       ```markdown
       ---

       ## Phase 35 closure

       Phase 35 (UPST3-closure quick wins, completed YYYY-MM-DD) closed the
       following Phase 34 deferrals via three wave-parallel plans
       (35-01-WIN-ENV-FILTER, 35-02-LINUX-LANDLOCK-PROFILES,
       35-03-WIN-TEST-HYGIENE). Per D-35-D4, Plan 35-03 (last to close in
       Phase 35) owns this consolidated append.

       ### P34-DEFER-01-1 — closed-by-Plan-35-03

       **Closing commit:** `<35-03-task-1-sha>` (Plan 35-03 Task 1 —
       production-code UNC verbatim-prefix strip in
       `query_ext::query_path::suggested_flag` emission).

       **Closure shape:** Wrapped both `suggested_flag_for_path(&canonical, ...)`
       call sites at `crates/nono-cli/src/query_ext.rs:167` + `:179` with the
       existing `strip_verbatim_prefix` helper (lines 295-317; originally
       introduced by in-fork commit `400f8c90` for the sensitive-path check).
       Test `test_query_path_denied` now passes deterministically on Windows
       + Linux + macOS with no `#[cfg]` gate on the test itself; cross-platform
       helper handles platform dispatch internally.

       ### P34-DEFER-08a-1 — closed-by-Plan-35-01

       **Closing commit:** `<35-01-sha>` (Plan 35-01 — Windows execution-path
       env-filter wiring; D-20 manual replay shape per D-35-A4).

       **Closure shape:** Added `allowed_env_vars` / `denied_env_vars` to
       Windows `ExecConfig` in `exec_strategy_windows/mod.rs`; wired into
       `build_child_env` (`launch.rs`) with deny-before-allow precedence
       mirroring the Unix call-site at `exec_strategy.rs:435-457`; removed
       the two `#[allow(dead_code)]` attributes on `is_env_var_allowed` /
       `is_env_var_denied` in `env_sanitization.rs`. Locked by Windows-gated
       regression test `test_windows_empty_allow_denies_all_env_vars`
       (fail-closed invariant from upstream `780965d7`) plus three sibling
       tests covering deny precedence, allow filtering, and nono-injected
       credential bypass.

       ### P34-DEFER-09-1 — closed-by-Plan-35-02

       **Closing commit:** `<35-02-task-1-sha>` (Plan 35-02 Task 1 — D-19
       cherry-pick of upstream `bdf183e9` v0.44.0).

       **Closure shape:** Cherry-picked the 15-line `profile_runtime.rs`
       Landlock pre-create hunk only; upstream's `wiring.rs` work (188/239
       LOC) is Phase 36 REQ-PORT-CLOSURE-04 territory. Commit carries the
       verbatim D-19 6-line trailer block (`Upstream-commit: bdf183e9`,
       lowercase `'a'` in `Upstream-author:`, two `Signed-off-by:` lines
       per template § D-19 field rules 4-6). Linux integration test
       `test_pre_create_landlock_profiles_dir_idempotent` ships in a
       follow-up commit; CI Linux lane is the functional verification
       surface per D-35-D3.

       ### P34-DEFER-09-3 — closed-by-Plan-35-03 (transitive)

       **Closing commit:** `<35-03-task-1-sha>` (same as P34-DEFER-01-1).

       **Closure shape:** Carry-forward duplicate of P34-DEFER-01-1 (same
       test, same failure shape). The Plan 35-03 Task 1 UNC strip closes
       both tickets in one fix. Recorded explicitly here for ledger
       traceability per D-35-C4.

       ### P34-DEFER-10-1 — closed-by-Plan-35-03

       **Closing commit:** `<35-03-task-2-sha>` (Plan 35-03 Task 2 — full
       `format!("{:?}")` audit + replacement with `serde_json::Map` insertion
       in `profile_cmd.rs` JSON-emission helpers).

       **Closure shape:** Replaced every in-scope Debug-format JSON-emission
       site in `profile_to_json` (line 1041), `diff_to_json` (line 1777),
       and `diff_custom_credentials_json` (line 1991) with
       `serde_json::Map::new` + `serde_json::to_value` calls. Applied
       omit-when-None semantics for the four Option<…> security fields
       (`signal_mode`, `process_info_mode`, `ipc_mode`, `wsl2_proxy_policy`)
       — JSON key absent when None, snake_case string when Some. The four
       PascalCase enum sites (workdir.access, plus paired profile1/profile2
       in diff_to_json) use `serde_json::to_value` against existing
       `#[serde(rename_all = ...)]` attributes — no enum-attribute changes
       needed (verified by Plan 35 PATTERNS.md). Out-of-scope sites in
       `cmd_diff` body (lines 1297-1318 — `diff_scalar_option` stdout
       printer, NOT JSON emission) preserved per D-35-C3. Function
       signature changes propagated through `cmd_show` (line 746) +
       `cmd_diff` (line 1164) call sites via `?`. Both regression tests
       (`test_policy_show_json_no_rust_debug_syntax` +
       `test_policy_diff_json_no_rust_debug_syntax`) pass deterministically
       on Windows + Linux + macOS. Restores the upstream `f3e7f885` (v0.47.0)
       shape that Plan 34-04b adopted but later Wave-3 plans regressed —
       closes the entire `format!("{:?}")` JSON-leak regression class via
       full audit per D-35-C3.

       ---
       ```
       Replace each `<...-sha>` marker with the actual 8-char commit SHA resolved in step 2. Replace `YYYY-MM-DD` with the actual Phase 35 close date.
    4. Save the file. Verify:
       - The pre-existing P34-DEFER-* entries (P34-DEFER-04b-1, P34-DEFER-04b-2, P34-DEFER-01-1, P34-DEFER-06-1, P34-DEFER-06-2, P34-DEFER-08a-1, P34-DEFER-08b-1, P34-DEFER-08b-2, P34-DEFER-09-1, P34-DEFER-09-2, P34-DEFER-09-3, P34-DEFER-10-1, P34-DEFER-10-2) are STILL PRESENT in their original locations — the closure section is purely additive.
       - The five closure sub-entries reference the correct ticket IDs in their headers.
    5. Commit:
       - **Subject:** `docs(35-03): append Phase 35 closure to Phase 34 deferred-items ledger`
       - **Body:** Cites D-35-D4 (consolidated append owned by last-to-close Plan 35-03) + lists the five closed tickets with one-line summaries.
       - **Trailer:** Regular DCO sign-off only (no D-19 trailer — this is fork-local documentation).
  </action>
  <acceptance_criteria>
    - `grep -c '^## Phase 35 closure' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` returns 1.
    - `grep -c '^### P34-DEFER-01-1 — closed-by-Plan-35-03' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` returns 1.
    - `grep -c '^### P34-DEFER-08a-1 — closed-by-Plan-35-01' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` returns 1.
    - `grep -c '^### P34-DEFER-09-1 — closed-by-Plan-35-02' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` returns 1.
    - `grep -c '^### P34-DEFER-09-3 — closed-by-Plan-35-03' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` returns 1.
    - `grep -c '^### P34-DEFER-10-1 — closed-by-Plan-35-03' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` returns 1.
    - The pre-existing P34-DEFER-* headers (e.g., `## P34-DEFER-04b-1`, `## P34-DEFER-08a-1`, etc.) are STILL PRESENT — verify via `grep -c '^## P34-DEFER' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` returning the same count as pre-plan baseline (at least 13).
    - Each of the five closing-commit SHA placeholders has been replaced with a real 8-char SHA (no `<...-sha>` literal remains): `grep -c '<.*-sha>' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` returns 0.
    - <automated>grep -c '^## Phase 35 closure' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md returns 1</automated>
  </acceptance_criteria>
  <verify>
    <automated>grep -c '^## Phase 35 closure' .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md</automated>
  </verify>
  <done>Phase 35 closure section appended to Phase 34 deferred-items ledger with five sub-entries (one per closed ticket), each citing the closing plan + commit SHA. Pre-existing P34-DEFER-* entries unchanged. Commit lands on `main` with regular DCO sign-off (no D-19 trailer — fork-local documentation only).</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| operator CLI → `nono query path …` output | Operator reads the `suggested_flag` value from `query_path` and copy-pastes it into a shell. The flag must be syntactically valid and re-executable. |
| `nono policy show --json` / `nono policy diff --json` → downstream JSON consumer | Operator or external tooling parses the JSON. The shape must be canonical (snake_case via serde, no Debug-format leakage). |
| Profile struct → JSON serializer | Internal struct shape (Rust enum variant identifiers like `Isolated`, `ReadWrite`) must NOT leak into operator-visible JSON. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-35-03-01 | Information disclosure | `format!("{:?}")` JSON emission | mitigate | Replace ALL in-scope `format!("{:?}")` sites in `profile_to_json` + `diff_to_json` + `diff_custom_credentials_json` with `serde_json::to_value`. Internal Rust struct shape (`Some(Isolated)`, `ReadWrite`) no longer leaks; consumers see canonical snake_case strings or omitted keys. Regression tests `test_policy_show_json_no_rust_debug_syntax` + `test_policy_diff_json_no_rust_debug_syntax` lock the invariant. |
| T-35-03-02 | Tampering (operator UX) | `suggested_flag` UNC verbatim prefix | mitigate | Wrap `&canonical` with `strip_verbatim_prefix(&canonical)` at both emission sites in `query_path`. Operators copy-pasting the suggested flag now receive a syntactically valid path (e.g., `C:\Users\...`), not the untypeable `\\?\C:\Users\...` form. Cross-platform helper makes Linux/macOS behavior identical. |
| T-35-03-03 | Information disclosure | Canonicalized path leaked via `suggested_flag` | accept | The `strip_verbatim_prefix` helper only strips well-known Windows NT verbatim prefixes (`\\?\`, `\\?\UNC\`, `\??\`); the underlying canonical path is still emitted (e.g., the actual Windows drive-letter path). This is the intended behavior — the operator needs to know which path was probed. No new exposure introduced. |
| T-35-03-04 | Denial of service | `serde_json::to_value` allocation pressure | accept | Each `to_value` call allocates a small Value enum; the total allocation overhead is negligible for the small set of Option<…> security fields. No DoS surface. |
| T-35-03-05 | Tampering | Closure ledger append racing with concurrent Phase 35 plan SUMMARYs | mitigate | Task 3 is the last task in Plan 35-03; Plan 35-03 is the LAST plan to close in Phase 35 per D-35-D4 (Plan 35-01 closes 08a-1; Plan 35-02 closes 09-1; Plan 35-03 closes 01-1 + 09-3 + 10-1). The consolidated append is owned exclusively by Plan 35-03; per-plan SUMMARYs only record their own closures (not the consolidated ledger). No race. |

</threat_model>

<verification_criteria>

## Phase 34 Close-Gate (D-35-D2 inherited verbatim — all 8 steps)

1. `cargo test --workspace --all-features` (Windows host) exits 0.
2. `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) exits 0.
3. `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` exits 0 (cross-target Linux gate — `strip_verbatim_prefix` non-Windows arm must remain reachable; the JSON shape changes are cross-platform).
4. `cargo clippy --workspace --all-targets --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` exits 0 (symmetric coverage; macOS uses the same JSON shape).
5. `cargo fmt --all -- --check` exits 0.
6. Phase 15 5-row detached-console smoke gate — NOT applicable to Plan 35-03 (no Windows execution-path edits; only `query_ext.rs` + `profile_cmd.rs` cross-platform JSON/CLI changes). Document as `N/A — no Windows execution-path edits` in SUMMARY.
7. `wfp_port_integration` test suite — passes or documented-skipped (no WFP surface touched; expect skip-due-to-no-change).
8. `learn_windows_integration` test suite — passes or documented-skipped (no Windows learn surface touched; expect skip-due-to-no-change).

## Plan-Specific Verification

- **PSV-1:** `git log --format='%B' main~3..main | grep -c '^Upstream-commit: '` returns 0 for Plan 35-03 commits (D-35-A4 — fork-local regression fixes; no D-19 trailer).
- **PSV-2:** `cargo test -p nono-cli --test profile_cli test_policy_show_json_no_rust_debug_syntax` and `cargo test -p nono-cli --test profile_cli test_policy_diff_json_no_rust_debug_syntax` both exit 0 on Windows host (and on Linux/macOS via cross-platform clippy or CI lanes).
- **PSV-3:** `cargo test -p nono-cli --lib query_ext::tests::test_query_path_denied` exits 0 on Windows host (previously failing per P34-DEFER-01-1; now passes deterministically without `#[cfg]` gate on the test).
- **PSV-4:** Out-of-scope preservation: `cmd_diff`-body stdout printer at lines 1297-1318 still contains `format!("{:?}` / `format!("{v:?}` strings — `sed -n '1290,1320p' crates/nono-cli/src/profile_cmd.rs | grep -c 'diff_scalar_option'` returns at least 1 (the human-readable stdout printer block is preserved per D-35-C3 scope).
- **PSV-5:** Phase 35 closure section appended to `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` with five sub-entries (D-35-D4 — consolidated ledger append owned by last-to-close Plan 35-03).
- **PSV-6:** No `*_windows.rs` edits — `git diff --stat HEAD~3 HEAD -- crates/nono-cli/src/ | grep -cE '_windows\.rs|exec_strategy_windows/'` returns 0 (Plan 35-03 stays in cross-platform files only).

## Acceptance Criteria Mapping (REQ-PORT-CLOSURE-07)

1. ✓ `cargo test query_ext::test_query_path_denied` passes deterministically on Windows — Task 1 production-code UNC strip; verified via PSV-3.
2. ✓ `nono profile show --json` and `nono profile diff --json` emit clean JSON (no `Debug` derive leak); regression tests pass — Task 2 Map-insertion refactor; verified via PSV-2.

</verification_criteria>

<success_criteria>

- Plan 35-03 closes P34-DEFER-01-1, P34-DEFER-09-3 (transitively), and P34-DEFER-10-1 directly. Plan 35-03 SUMMARY records all three closures.
- Three commits land on `main` for Plan 35-03 (Task 1 UNC strip + Task 2 JSON shape audit + Task 3 closure ledger append). All three have regular DCO sign-off; NONE carry the D-19 trailer block.
- Phase 35 closure section appended to `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/deferred-items.md` with five sub-entries (one per ticket closed by the entire Phase 35 wave) and real 8-char SHA references (no `<...-sha>` placeholders remain).
- Workspace compiles, tests pass, clippy clean on Windows + cross-target Linux + cross-target macOS (D-35-D2 close gate steps 1-5 green; steps 6/7/8 documented as `N/A`).
- No `*_windows.rs` edits; Plan 35-03 stays in cross-platform files only.
- No `.unwrap()` / `.expect()` introduced in production code; signature changes in `profile_to_json` + `diff_to_json` propagate cleanly via `?`.
- Both locked regression tests (`test_policy_show_json_no_rust_debug_syntax` + `test_policy_diff_json_no_rust_debug_syntax`) pass deterministically; UX bug closed (`suggested_flag` no longer contains `\\?\` UNC verbatim prefix).
- Out-of-scope `cmd_diff`-body stdout printers at lines 1297-1318 preserved per D-35-C3 (only JSON-emission helpers fixed; stdout printers unchanged).

</success_criteria>

<output>
After completion, create `.planning/phases/35-upst3-closure-quick-wins/35-03-WIN-TEST-HYGIENE-SUMMARY.md` with:
- Frontmatter recording the three commit SHAs (Task 1 UNC strip, Task 2 JSON shape audit, Task 3 closure ledger append), test pass counts (`profile_cli` regression tests = 2 passed; `query_ext::tests::test_query_path_denied` = 1 passed; full `cargo test -p nono-cli` summary line).
- Body documenting: D-35-C1 (production-code UNC strip — not test gating), D-35-C2 (Map-insertion shape + omit-when-None semantics), D-35-C3 (full audit scope — what's in vs out), D-35-C4 (P34-DEFER-09-3 transitive closure), D-35-D4 (consolidated ledger append owned by last-to-close), close-gate disposition for each of the 8 D-35-D2 steps.
- Closure-section ledger entries: marks P34-DEFER-01-1, P34-DEFER-09-3, P34-DEFER-10-1 as `closed-by-Phase-35-03` (each with the relevant Task SHA from the three commits).
- Pointer to Phase 34 `deferred-items.md` § Phase 35 closure (the consolidated cross-plan ledger).
</output>
