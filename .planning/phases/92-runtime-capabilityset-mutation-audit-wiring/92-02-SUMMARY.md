---
phase: 92-runtime-capabilityset-mutation-audit-wiring
plan: "02"
subsystem: nono-py override wiring
tags: [override, audit, capability-mutation, nono-py, pyo3, tdd]
dependency_graph:
  requires:
    - 92-01  # Nono core audit.rs + telemetry changes (parallel wave, independent repo)
  provides:
    - OverrideGrant.zt_audit_hash getter (D-06)
    - confined_run/confine override_token parameter (D-01)
    - append_override_args / sanitize_override_path / probe_override_support (MUT-01/MUT-04/D-02)
  affects:
    - nono-py/src/override.rs
    - nono-py/src/windows_confined_run.rs
tech_stack:
  added: []
  patterns:
    - TDD RED/GREEN/REFACTOR with #[cfg(test)] Rust unit tests
    - LazyLock<Mutex<HashMap>> for process-lifetime probe cache
    - Path::components() for ParentDir rejection (never string starts_with)
    - base64::engine::general_purpose::URL_SAFE_NO_PAD for --override-audit payload
    - serde_json::json!() for audit metadata construction
key_files:
  created: []
  modified:
    - C:/Users/OMack/nono-py/src/override.rs
    - C:/Users/OMack/nono-py/src/windows_confined_run.rs
decisions:
  - "sanitize_override_path accepts both Unix-convention '/' paths AND Windows-native absolute paths (same rationale as partition_scope in override.rs: CAF v0.1 tokens use Unix-style paths; Path::is_absolute() returns false for these on Windows)"
  - "PROBE_CACHE uses LazyLock<Mutex<HashMap>> (Rust 1.80+ stable) rather than Mutex::new(HashMap::new()) in static initializer (E0015 error)"
  - "parse_version_gte_3_2() uses string splitting on whitespace + '.', not a regex crate (no new deps; closed-universe co-deployment model)"
  - "VFY-01 PARTIAL [BLOCKING-93] seam comment inserted in both confined_run and confine per plan spec"
metrics:
  duration: "~25 minutes"
  completed: "2026-06-22T12:55:31Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
---

# Phase 92 Plan 02: nono-py Override Wiring Summary

nono-py override path wired end-to-end: OverrideGrant gains `zt_audit_hash` getter from `token.current_hash` (D-06 bi-directional audit link); `confined_run` and `confine` accept `override_token: Option<PyRef<'_, OverrideGrant>>` and invoke `probe_override_support` (D-02 bilateral capability gate, version >= 3.2) then `append_override_args` (sanitized `--allow` flags + `--override-audit <base64url-json>` with AUD-02 metadata) before the `--` separator.

## Task Results

| Task | Name | Commit (nono-py) | Status |
|------|------|-----------------|--------|
| 1 | Extend OverrideGrant with zt_audit_hash; remove OutOfScope dead_code annotation | `519fa9f` | DONE |
| 2 | Wire override into confined_run/confine; add append_override_args + sanitize_override_path | `b7be784` | DONE |

## What Was Built

### Task 1 — `nono-py/src/override.rs` (commit `519fa9f`)

**New field:** `pub zt_audit_hash: Option<String>` added to `OverrideGrant` struct, after `repo_context`.

**New getter:** `#[getter] pub fn zt_audit_hash(&self) -> Option<String>` added to `#[pymethods] impl OverrideGrant`.

**Construction site:** `verify_override_impl` `Ok(OverrideGrant { ... })` now includes `zt_audit_hash: token.current_hash.clone()`. Reads from the already-verified grant, never re-parses raw token bytes (D-06: closes TOCTOU verify→apply gap).

**Dead code removal:** `#[expect(dead_code, reason = "forward-declared for Phase 92 scope enforcement; ...")]` removed from `OverrideErrorKind::OutOfScope`. Plan 03 adds the first production construction site.

**Tests added (4):** `phase92_zt_audit_hash` module — getter returns `Some`, getter returns `None`, construction site populates from `token.current_hash`, construction site produces `None` when `current_hash` absent.

### Task 2 — `nono-py/src/windows_confined_run.rs` (commit `b7be784`)

**New static:** `PROBE_CACHE: LazyLock<Mutex<HashMap<PathBuf, bool>>>` — process-lifetime cache of probe results keyed by nono.exe path.

**New helpers:**
- `probe_override_support(nono_path: &Path) -> PyResult<()>`: runs `nono.exe --version`, requires version >= 3.2, caches true/false per path, raises `NonoOverrideError` on failure. Transient I/O errors not cached.
- `parse_version_gte_3_2(s: &str) -> bool`: parses "v<major>.<minor>" from version string output.
- `sanitize_override_path(raw: &str) -> PyResult<PathBuf>`: accepts Unix-convention `/` paths and Windows-native absolute paths; rejects relative via `!is_absolute() && !starts_with('/')` check; rejects `..` via `Path::components()` / `Component::ParentDir` (never string `starts_with`; MUT-04 / CLAUDE.md §Path Handling).
- `append_override_args(cmd, grant)`: sanitizes scope_paths → `--allow` flags; builds `serde_json::json!({zt_audit_hash, kms_key_id, jti, granted_paths, expires_at})` → base64url-no-pad → `--override-audit <meta>` (D-05/D-06).

**Signature extensions:**
- `confined_run`: `#[pyo3(signature = (exe, args, allow=None, profile=None, cwd=None, timeout_secs=None, override_token=None))]` + `override_token: Option<PyRef<'_, OverrideGrant>>` parameter.
- `confine`: `#[pyo3(signature = (profile=None, allow=None, caps=None, override_token=None))]` + `override_token: Option<PyRef<'_, OverrideGrant>>` parameter.

**Override hook (both functions):** After `build_nono_run_args` / `append_caps_allow_flags`, before `cmd.arg("--")`:
```rust
if let Some(ref grant) = override_token {
    probe_override_support(&nono_path)?;   // D-02: fail-closed version gate
    append_override_args(&mut cmd, grant)?; // D-05/D-06: sanitize + audit metadata
}
// VFY-01 PARTIAL [BLOCKING-93]: Phase 92 wires the offline verify arm.
// Phase 93 adds the live POST /actions AND-gate here before confined_run
// is called (D-03 composition seam; VFY-01 clause b).
```

**Tests added (6):** sanitize accepts absolute, rejects relative (error contains "absolute"), rejects ParentDir on Unix-style path, rejects ParentDir on Windows-absolute path; no-token baseline does not produce `--override-audit`; probe caches result via PROBE_CACHE pre-population.

## Test Results

```
test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Previous count: 55 (Phase 91). Added: 4 (Task 1) + 6 new (Task 2) = 10 new tests.

`cargo build`: 1 pre-existing warning (`OutOfScope` dead_code — BLOCKED on Plan 03 production construction site; `#[expect(dead_code)]` removed per plan as Phase 92 removes the forward-declaration; Plan 03 will add the first non-test construction site and resolve the warning).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `Mutex::new(HashMap::new())` in `static` initializer fails to compile (E0015)**
- **Found during:** Task 2 GREEN phase, first compile attempt
- **Issue:** `HashMap::new()` is not `const`, so `static PROBE_CACHE: Mutex<HashMap<PathBuf, bool>> = Mutex::new(HashMap::new())` fails with `E0015: cannot call non-const associated function in statics`.
- **Fix:** Changed to `static PROBE_CACHE: LazyLock<Mutex<HashMap<PathBuf, bool>>> = LazyLock::new(|| Mutex::new(HashMap::new()))`. `LazyLock` is stable since Rust 1.80; MSRV is 1.82 (Cargo.toml) — acceptable.
- **Files modified:** `nono-py/src/windows_confined_run.rs`
- **Commit:** `b7be784`

**2. [Rule 1 - Bug] `sanitize_override_path("/tmp/project")` failed on Windows — `Path::is_absolute()` returns false for Unix-convention paths**
- **Found during:** Task 2 GREEN phase, first test run (`test_sanitize_override_path_accepts_absolute`)
- **Issue:** On Windows, `Path::is_absolute()` requires a drive letter; `/tmp/project` returns false. The plan spec says "uses `p.is_absolute()` for the absolute check" but the test behavior spec requires `/tmp/project` to be accepted (override tokens use Unix-convention paths per CAF v0.1).
- **Fix:** Added `/`-prefix check alongside `is_absolute()`, mirroring the same decision made in `partition_scope()` (override.rs line 918–924) which has the same cross-platform constraint and documents the same rationale. Updated doc comment to explain the dual-acceptance.
- **Files modified:** `nono-py/src/windows_confined_run.rs`
- **Commit:** `b7be784`

## VFY-01 PARTIAL Status

Per D-03 (locked): Phase 92 wires the offline arm only. The live `POST /actions` AND-gate is deferred to Phase 93. Both `confined_run` and `confine` have the seam comment:
```
// VFY-01 PARTIAL [BLOCKING-93]: Phase 92 wires the offline verify arm.
// Phase 93 adds the live POST /actions AND-gate here before confined_run/confine
// is called (D-03 composition seam; VFY-01 clause b).
```

## Known Stubs

None — all fields are wired to real data from `OverrideGrant`. No placeholder text or hardcoded empty values in production code paths.

## Host-Gated Deferrals

- `maturin develop` / Python wheel build: not run (requires `maturin` CLI installed; `cargo test` verifies all Rust-level behavior; maturin build is a deployment step).
- OVERRIDE-01 gate (`scripts/gates/override-01.ps1`): not implemented in Plan 02 (DF-01 gate is Plan-level scope for Phase 92 as a whole, not Plan 02 specifically).

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries introduced beyond those documented in the plan's `<threat_model>`.

## Self-Check: PASSED

Files exist:
- `C:/Users/OMack/nono-py/src/override.rs` — FOUND (modified)
- `C:/Users/OMack/nono-py/src/windows_confined_run.rs` — FOUND (modified)

Commits exist (nono-py repo):
- `519fa9f` — FOUND (Task 1: OverrideGrant zt_audit_hash)
- `b7be784` — FOUND (Task 2: override wiring in confined_run/confine)

Test count: 61 passed, 0 failed (up from 55 in Phase 91).
