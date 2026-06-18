# Phase 77: Copilot CLI End-to-End Confinement - Pattern Map

**Mapped:** 2026-06-17
**Files analyzed:** 8 (5 modified, 1 new Rust, 1 new script, 1 docs)
**Analogs found:** 8 / 8 (every touched file has an exact in-repo analog)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono/src/sandbox/windows.rs` (+ `grant_sid_read_attributes_on_path` + mask const) | library / DACL primitive | transform (mask→ACE) | `grant_sid_read_on_path` / `grant_sid_traverse_on_path` (same file, lines 1816/1857) | exact (same file, sibling fn) |
| `crates/nono/src/lib.rs` (+ re-export) | library / barrel | re-export | existing grant-fn re-export cluster (line 86-88) | exact |
| `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` (+ `AppliedAncestorReadAttributesGuard`) | middleware / RAII guard | event-driven (apply/Drop-revert) | `AppliedAncestorTraverseGuard` (same file, lines 222-307) | exact (same file, sibling guard) |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` (wire new guard) | service / launch orchestrator | request-response (prepare launch) | `_applied_ancestor_traverse` wiring (lines 318, 437-443, 455) | exact |
| `crates/nono-cli/data/policy.json` (`copilot-cli`: + `windows_interpreters`) | config / policy data | transform (profile shape) | `langchain-python` / `aider` profile (lines 884-900, 919-935) | exact |
| `crates/nono-cli/src/profile/builtin.rs` (invert native-PE tests) | test | transform assertions | `test_get_builtin_langchain_python` (lines 227-247) | exact |
| `crates/nono-cli/src/cli.rs` (`SetupArgs`: + `--grant-ancestors` + `--profile`) | route / clap arg | request-response | existing `SetupArgs` flags (lines 2570-2638) | exact |
| `crates/nono-cli/src/setup.rs` (handle grant-ancestors path) | service / setup runtime | request-response | `register_windows_wfp_service` (lines 172-189) + dispatch (lines 55-61) | role-match (admin-gated setup action) |
| `scripts/gates/copilot-e2e.ps1` (NEW) | test / harness gate | request-response (probe→run→verdict) | `scripts/gates/harness-self-check.ps1` (full file) | exact (reference contract) |

## Pattern Assignments

### `crates/nono/src/sandbox/windows.rs` — `grant_sid_read_attributes_on_path` (CPLT-01, library primitive)

**Analog:** `grant_sid_read_on_path` (lines 1857-1864) and `grant_sid_traverse_on_path` (lines 1816-1827), both `#[cfg(target_os = "windows")]`, both one-liners over the shared `edit_dacl_for_sid` core.

**Mask-constant pattern** (the const-fold idiom used at lines 1483-1486 / 1513-1516):
```rust
// New const beside PACKAGE_SID_READ_MASK (line 1483) / PACKAGE_SID_TRAVERSE_MASK (line 1513).
// MINIMAL grant per D-09: attribute-read only, NOT FILE_GENERIC_READ.
const PACKAGE_SID_READ_ATTRS_MASK: u32 = {
    use windows_sys::Win32::Storage::FileSystem::FILE_READ_ATTRIBUTES; // 0x80
    FILE_READ_ATTRIBUTES
};
```

**Public wrapper pattern** (copy `grant_sid_read_on_path` verbatim, lines 1857-1864, swap mask):
```rust
#[cfg(target_os = "windows")]
pub fn grant_sid_read_attributes_on_path(path: &Path, sid: &str) -> Result<()> {
    use windows_sys::Win32::Security::Authorization::SET_ACCESS;
    use windows_sys::Win32::Security::NO_INHERITANCE;
    edit_dacl_for_sid(path, sid, PACKAGE_SID_READ_ATTRS_MASK, SET_ACCESS, NO_INHERITANCE)
}
```

**Shared core (DO NOT reimplement)** — `edit_dacl_for_sid` (lines 1629-1733): `parse_sid` → `GetNamedSecurityInfoW(DACL)` → `SetEntriesInAclW(1, &entry, old_dacl, ..)` MERGE → `SetNamedSecurityInfoW(DACL, no PROTECTED flag)`. Fail-closed at every step via `NonoError::DaclApplyFailed { path, hresult, hint }`. The new fn just supplies a new mask. **Revert path is already covered** by `revoke_sid_on_path` (lines 1880-1887) — trustee-match removal ignores the mask, so no new revoke fn is needed.

**Doc-comment style:** mirror lines 1829-1856 — state the exact mask (`FILE_READ_ATTRIBUTES = 0x80`), why it is minimal (D-09: attribute-read, not content-read), and the per-ancestor `realpathSync`/`lstat` rationale; include an `# Errors` section naming `NonoError::DaclApplyFailed`.

---

### `crates/nono/src/lib.rs` — re-export (CPLT-01)

**Analog:** lines 86-88, the existing alphabetized grant-fn re-export cluster:
```rust
derive_app_container_sid, grant_sid_read_on_path, grant_sid_traverse_on_path,
grant_sid_write_on_path, label_mask_for_access_mode, low_integrity_label_and_mask,
package_sid_to_string, path_has_write_owner, path_is_owned_by_current_user, revoke_sid_on_path,
```
Insert `grant_sid_read_attributes_on_path` in alphabetical position (between `grant_sid_read_on_path` and `grant_sid_traverse_on_path`).

---

### `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` — `AppliedAncestorReadAttributesGuard` (CPLT-01, RAII guard)

**Analog:** `AppliedAncestorTraverseGuard` (lines 222-307) — clone the struct, the `snapshot_and_apply`, `revert_all`, and `Drop`, swapping ONE call.

**Struct + import pattern** (lines 222-227; add the new grant fn to the `nono::{…}` import at lines 44-47):
```rust
#[derive(Debug)]
pub(crate) struct AppliedAncestorReadAttributesGuard {
    applied: Vec<PathBuf>,
    package_sid: String,
}
```

**Ownership-gated ancestor-walk (the load-bearing pattern)** — copy lines 246-282, swap the grant call:
```rust
for ancestor in walk_target.ancestors().skip(1) {
    match path_is_owned_by_current_user(ancestor) {
        Ok(true) => {
            if let Err(err) = grant_sid_read_attributes_on_path(ancestor, package_sid) {
                tracing::warn!(/* … */ "ancestor-RA guard: grant failed; reverting");
                guard.revert_all();
                return Err(err);
            }
            guard.applied.push(ancestor.to_path_buf());
        }
        Ok(false) => break,    // C:\Users, C:\ — admin grant (CPLT-02) covers these. D-04 split.
        Err(err)  => { guard.revert_all(); return Err(err); }  // NEVER swallow ownership errors
    }
}
```
**D-04 enforcement is structural:** the `Ok(false) => break` makes the runtime guard provably never touch `C:\Users`/`C:\` — the exact split CPLT-02 (admin) covers. Keep this verbatim.

**Drop + revert_all:** copy lines 287-307 verbatim (LIFO `revoke_sid_on_path`, best-effort, never panic).

**Walk-target divergence from the analog (the one design decision):** the traverse guard walks `current_dir` (cwd). CPLT-01 must walk the **confined target binary's resolution chain** (and/or the package-SID grant-set paths) per RESEARCH Pattern 2 / OQ-3. Parameterize `snapshot_and_apply(walk_target: &Path, package_sid: &str)` and feed it the resolved program path; if multiple roots need RA, loop over them reusing the single-path walk. Document the difference from the traverse guard in the doc comment (RESEARCH "Alternatives Considered" says keep them separate).

**Tests:** clone the two `#[cfg(test)] #[cfg(target_os = "windows")]` tests at lines 532-587:
- `ancestor_read_attributes_grants_owned_ancestors_and_reverts_on_drop` (mirror line 533) — use the shared `dacl_contains_sid` helper (lines 323-397) and `TEST_PACKAGE_SID` (line 526).
- `ancestor_read_attributes_stops_at_non_owned_ancestor` (mirror line 570) — assert the drive root never appears in `applied`.

---

### `crates/nono-cli/src/exec_strategy_windows/mod.rs` — wire the guard (CPLT-01)

**Analog:** the `_applied_ancestor_traverse` field + apply site. Three edits mirror it exactly:

1. **Struct field** (after line 318, in `PreparedWindowsLaunch`): add `_applied_ancestor_read_attrs: Option<dacl_guard::AppliedAncestorReadAttributesGuard>,` with the same drop-order comment block (declared AFTER `_applied_ancestor_traverse`, BEFORE `_network_enforcement`).

2. **Apply site** (mirror lines 437-443) inside `prepare_live_windows_launch`:
```rust
let applied_ancestor_read_attrs = config
    .package_sid
    .as_deref()
    .map(|sid| {
        dacl_guard::AppliedAncestorReadAttributesGuard::snapshot_and_apply(
            config.resolved_program, sid)   // walk the target-binary chain, not cwd
    })
    .transpose()?;   // fail-closed
```

3. **Struct init** (mirror line 455): add `_applied_ancestor_read_attrs: applied_ancestor_read_attrs,` to the returned `PreparedWindowsLaunch`.

Note the gate `config.package_sid.as_deref().map(...)` — the RA grant is only active on the AppContainer arm (package SID present), identical to the traverse guard.

---

### `crates/nono-cli/data/policy.json` — `copilot-cli` profile (CPLT-01 / D-02)

**Analog:** `langchain-python` (lines 919-935) and `aider` (lines 884-900) — both end with `"windows_low_il_broker": true` then `"windows_interpreters": ["python.exe"]`.

**Current `copilot-cli`** (lines 902-918) ends at `"windows_low_il_broker": true` with NO `windows_interpreters` and a stale `meta.description` (line 907). Edits:
```jsonc
"copilot-cli": {
  "meta": {
    // line 907: REPLACE the "native PE … no windows_interpreters needed" string with
    // the standalone-Node-CLI description (D-01/D-02/State-of-the-Art table).
  },
  // …
  "windows_low_il_broker": true,
  "windows_interpreters": ["node.exe"]   // ← ADD (mirror python.exe sibling)
}
```
**Rebuild required** after this edit — `policy.json` is embedded at build time via `build.rs` (`make build`). The coverage gate `validate_windows_launch_paths` (mod.rs:345) already consumes `config.interpreters`, so no new launch-code path. OQ-3 (shim/`%APPDATA%\npm` coverage) is settled empirically on-host by the gate.

---

### `crates/nono-cli/src/profile/builtin.rs` — invert native-PE tests (CPLT-01)

**Analog (the CORRECT target shape):** `test_get_builtin_langchain_python` (lines 227-247), specifically lines 242-246:
```rust
assert_eq!(
    profile.windows_interpreters,
    vec!["python.exe".to_string()],
    "… profile must declare python.exe interpreter coverage"
);
```

**Edits (two stale tests encode the superseded D-06 native-PE assumption):**
- `copilot_cli_profile_is_native_pe` (lines 290-299) asserts `profile.windows_interpreters.is_empty()` — **INVERT** to assert `== vec!["node.exe".to_string()]` (rename to e.g. `copilot_cli_profile_declares_node_interpreter`). Mirror the langchain assertion.
- `copilot_cli_profile_present` (lines 256-280) — line 264's comment `(native PE engine)` and the broader native-PE framing must be updated; the structural asserts (name, `windows_low_il_broker`, ReadWrite, isolated, `!network.block`) stay valid.

---

### `crates/nono-cli/src/cli.rs` — `SetupArgs` extension (CPLT-02 / D-06)

**Analog:** existing `SetupArgs` boolean+`PathBuf` flags (lines 2570-2638), e.g. `register_wfp_service` (line 2576-2577) and `from_file: Option<PathBuf>` (line 2615-2621).

**Add (generic, NOT Copilot-specific — D-06):**
```rust
/// Grant the well-known ALL APPLICATION PACKAGES SID FILE_READ_ATTRIBUTES on the
/// system ancestors (C:\, C:\Users) a confined engine cannot ACL at runtime.
/// One-time, requires admin, idempotent, non-destructive (D-09). Windows only.
#[arg(long, help_heading = "OPTIONS")]
pub grant_ancestors: bool,

/// Profile whose ancestor coverage to grant (used with --grant-ancestors).
#[arg(long, value_name = "PROFILE", help_heading = "OPTIONS", requires = "grant_ancestors")]
pub profile: Option<String>,
```
Use the same `#[arg(long, help_heading = "OPTIONS")]` derive idiom and the `requires`/`conflicts_with` relations seen at lines 2615-2620. Add a clap parse/dispatch unit test (per the test map, CPLT-02).

---

### `crates/nono-cli/src/setup.rs` — grant-ancestors runtime (CPLT-02 / D-05 resolution / D-09)

**Analogs:**
- **Admin gate + dispatch shape:** `register_windows_wfp_service` (lines 172-189) — the `if !crate::exec_strategy::is_admin_process() { return Err(NonoError::Setup("… requires an elevated administrator session …")) }` guard is the exact pattern to copy.
- **Short-circuit dispatch:** the `uninstall_wfp` early-return in `run()` (lines 55-61) — add a parallel `if !self.check_only && self.grant_ancestors { return self.grant_ancestors_for_profile(); }`.
- **Field plumbing:** `SetupRunner` struct fields + `new()` (lines 12-53) — add `#[cfg(target_os = "windows")] grant_ancestors: bool` and `profile: Option<String>` mirroring `register_wfp_service`/`from_file`.

**Grantee SID (D-05 RESOLUTION — locked):** the durable admin grant targets the **well-known `ALL APPLICATION PACKAGES` SID `"S-1-15-2-1"`** (a hardcoded constant string passed to `grant_sid_read_attributes_on_path`), NOT a per-run package SID and NOT `derive_app_container_sid`. The per-run derivation (`derive_app_container_sid`, windows.rs:747 + `package_sid_to_string`, windows.rs:789) is the analog the CPLT-01 *runtime* grant uses — **do NOT reuse it here.** This is the single divergence the operator locked: per-run SID for runtime, well-known SID for the durable admin grant.

**Runtime sketch** (per RESEARCH "Idempotent admin grant"):
```rust
const ALL_APPLICATION_PACKAGES_SID: &str = "S-1-15-2-1";
for ancestor in ["C:\\", "C:\\Users"] {   // system ancestors only — runtime guard covers the rest
    // idempotency: query DACL + EqualSid match using the dacl_contains_sid / GetAce
    //   technique cloned from dacl_guard.rs:323-397; if present → no-op (don't stack ACEs).
    // if absent: nono::grant_sid_read_attributes_on_path(Path::new(ancestor), ALL_APPLICATION_PACKAGES_SID)?;
    // never remove/modify an existing ACE (D-09 non-destructive).
}
```
Fail-closed: surface a clear "requires elevation" `NonoError::Setup` if `SetNamedSecurityInfoW` returns access-denied (the existing `edit_dacl_for_sid` hint already names WRITE_DAC).

**Tests** (Windows-gated, per the test map): `grant_ancestors_idempotent` (run twice → exactly one ACE, using the `GetAce` count technique) and `grant_ancestors_non_destructive` (pre-existing ACEs unchanged).

---

### `scripts/gates/copilot-e2e.ps1` — NEW gate (CPLT-03)

**Analog:** `scripts/gates/harness-self-check.ps1` (full file) — the **reference contract for phases 77-80** (its header line 16 says so). Copy the structure exactly.

**Two-function contract (copy verbatim shape, lines 60-126):**
```powershell
function Test-Precondition {
    # D-07: return $null → run Invoke-Gate; return "reason" → SKIP_HOST_UNAVAILABLE.
    if (-not (Get-Command copilot -ErrorAction SilentlyContinue)) { return 'copilot CLI not installed' }
    # probe GitHub auth + network; if absent → return a reason string (SKIP, not FAIL — D-07/D-08)
    return $null
}
function Invoke-Gate {
    # Run ONE-SHOT confined copilot under nono; assert no STATUS_ACCESS_DENIED + no Node
    # module-resolution crash (Cannot find module / ERR_MODULE_NOT_FOUND). D-08.
    # NEVER call exit (the runner owns exit mapping). NEVER return PASS on a broken framework.
    return [ordered]@{ gate='copilot-e2e'; verdict=$verdict; reason=$reason; detail=$detail; timestamp=(Get-Date -Format 'yyyy-MM-ddTHH:mm:ss.fffZ') }
}
```

**Verdict-object key order** must be `gate, verdict, reason, detail, timestamp` — matches `Build-Verdict` in `verify-dark.ps1` (lines 37-43) and the self-check candidate (harness-self-check.ps1 lines 73-79). The runner owns emit + persist + exit mapping (PASS=0 / FAIL=2 / SKIP_HOST_UNAVAILABLE=3 / harness-error=4) and persists to `.nono-runtime/verdicts/copilot-e2e.json` BEFORE the stdout line (WR-04, verify-dark.ps1 lines 46-60). **Do NOT** duplicate persist/exit logic in the gate.

**Assertion helpers:** mirror `Assert-Equal` / `Assert-True` (harness-self-check.ps1 lines 25-54) — throw on failure (a throw inside `Invoke-Gate` = harness-internal error, exit 4, never a silent PASS).

**Determinism (Pitfall 4):** use Copilot's non-interactive one-shot mode + a timeout (REPL would hang the unattended harness). Confirm the exact flag on-host (OQ-3).

## Shared Patterns

### Fail-closed DACL editing
**Source:** `crates/nono/src/sandbox/windows.rs` `edit_dacl_for_sid` (lines 1629-1733)
**Apply to:** CPLT-01 library fn, CPLT-02 admin grant
Every grant routes through this single mask-parameterized core (`GetNamedSecurityInfoW` → `SetEntriesInAclW` MERGE → `SetNamedSecurityInfoW` no-PROTECTED). It is fail-closed at every step via `NonoError::DaclApplyFailed { path, hresult, hint }`. Never hand-roll `SetEntriesInAclW`.

### Ownership-gated, fail-closed, Drop-reverting RAII guard
**Source:** `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` `AppliedAncestorTraverseGuard` (lines 222-307)
**Apply to:** CPLT-01 runtime guard
`path_is_owned_by_current_user` gate before any DACL edit; `Ok(false) => break` (the D-04 split); `Err => revert_all + propagate` (never swallow ownership errors); LIFO best-effort revert on `Drop`. The guard is wired into `PreparedWindowsLaunch` with reverse-drop-order discipline (mod.rs:300-321).

### Path-ownership check (never string-compare paths)
**Source:** `nono::path_is_owned_by_current_user` (used at dacl_guard.rs:113, 247)
**Apply to:** CPLT-01 guard
CLAUDE.md footgun #1: never `starts_with` on path strings. Use the component-correct ownership probe.

### Admin-gated setup action
**Source:** `crates/nono-cli/src/setup.rs` `register_windows_wfp_service` (lines 172-189) + `run()` short-circuit (lines 55-61)
**Apply to:** CPLT-02 setup command
`is_admin_process()` guard returning `NonoError::Setup("… requires an elevated administrator session …")`; field plumbed through `SetupRunner::new` cfg-gated for Windows.

### Dark-factory gate two-function contract
**Source:** `scripts/gates/harness-self-check.ps1` (lines 60-126) + runner `scripts/verify-dark.ps1` (lines 29-60)
**Apply to:** CPLT-03 gate
`Test-Precondition` (→ `$null` | reason) + `Invoke-Gate` (→ ordered verdict dict); never `exit`; throw = harness-internal error; runner owns persist-before-emit + exit mapping.

### Cross-target clippy discipline (cfg-gated Windows code)
**Source:** CLAUDE.md § Coding Standards; the `#[cfg(target_os = "windows")]` + `#[cfg(test)] #[cfg(target_os = "windows")]` gating throughout `windows.rs` / `dacl_guard.rs`
**Apply to:** the new library fn + guard + their tests
Gate every new Windows-only `pub fn`, guard, and test; verify via `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`, or mark PARTIAL + defer to CI per the cross-target-verify checklist. The dev host cannot compile non-Windows targets (RESEARCH Pitfall 3 / MEMORY PR #9).

## No Analog Found

None. Every file touched by Phase 77 has an exact or near-exact in-repo analog. The only genuinely new design element is the **grantee-SID choice** for CPLT-02 (well-known `ALL APPLICATION PACKAGES` `S-1-15-2-1` rather than the per-run package SID) — that is a locked user decision (D-05 resolution), not a missing pattern; the grant *mechanism* reuses the existing primitive.

## Metadata

**Analog search scope:** `crates/nono/src/sandbox/windows.rs`, `crates/nono/src/lib.rs`, `crates/nono-cli/src/exec_strategy_windows/{dacl_guard.rs,mod.rs,restricted_token.rs}`, `crates/nono-cli/src/{profile/builtin.rs,cli.rs,setup.rs,app_runtime.rs}`, `crates/nono-cli/data/policy.json`, `scripts/gates/harness-self-check.ps1`, `scripts/verify-dark.ps1`
**Files scanned:** 12
**Pattern extraction date:** 2026-06-17
