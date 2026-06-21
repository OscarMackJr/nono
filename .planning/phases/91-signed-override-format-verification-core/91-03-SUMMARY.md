---
phase: 91-signed-override-format-verification-core
plan: "03"
subsystem: nono-py/override
tags: [pyo3, pyclass, custom-exception, frozen, verify-override, vfy-07, sc4, sc5, tdd]
dependency_graph:
  requires:
    - nono-py/src/override.rs (verify_override_impl, OverrideGrant, OverrideErrorKind, override_err_to_py) — from Plans 01/02
  provides:
    - nono-py/src/override.rs (#[pyclass(frozen)] OverrideGrant + #[pymethods], NonoOverrideError create_exception!, override_err_to_py, #[pyfunction] verify_override PyO3 wrapper)
    - nono-py/src/lib.rs (three new registrations: add_class OverrideGrant, add_function verify_override, m.add NonoOverrideError)
  affects:
    - nono-py/Cargo.toml ([dev-dependencies] pyo3 auto-initialize for cargo test bootstrap)
tech_stack:
  added:
    - pyo3::create_exception! (repo's FIRST custom exception — zero prior uses confirmed by grep)
    - pyo3 auto-initialize feature ([dev-dependencies] only — production cdylib unchanged)
  patterns:
    - "#[pyclass(frozen, skip_from_py_object)] — skip_from_py_object required by PyO3 0.28 to silence deprecated FromPyObject impl warning (matches FsCapability/ExecResult pattern)"
    - "PyO3 0.28 module registration: m.py().get_type::<NonoOverrideError>() — confirmed against pyo3-0.28.3/src/exceptions.rs example"
    - "PyO3 0.28 test pattern: Python::attach (not Python::with_gil — renamed in 0.28)"
    - "A2 outcome: PyO3 0.28 create_exception! macro signature is create_exception!(module, Name, Base, optional_doc) — identical to RESEARCH assumption; m.py().get_type::<T>() registration unchanged"
    - "override_err_to_py: kind.as_str() as args[0] (machine-readable D-04 code); msg is redaction-safe (no raw key/sig bytes)"
    - "verify_override_impl (pub(crate)) vs verify_override #[pyfunction] (pub) — separate to avoid Rust #[pyfunction] + #[must_use] interaction"
key_files:
  modified:
    - nono-py/src/override.rs (472 insertions: pyclass/pymethods OverrideGrant, create_exception!, override_err_to_py, #[pyfunction] verify_override, pyo3_boundary #[cfg(test)] module with 11 SC4 assertions)
    - nono-py/src/lib.rs (6 insertions: 3 registration lines + comment block)
    - nono-py/Cargo.toml (6 insertions: [dev-dependencies] pyo3 auto-initialize)
decisions:
  - "A2 verified: PyO3 0.28 create_exception! macro and m.py().get_type::<T>() registration idiom confirmed against pyo3-0.28.3 source — identical to RESEARCH/PATTERNS assumptions"
  - "skip_from_py_object on OverrideGrant: required by PyO3 0.28 to avoid deprecated FromPyObject for Clone pyclasses; matches existing FsCapability/ExecResult pattern in the repo"
  - "Python::attach replaces Python::with_gil in PyO3 0.28; allow_threads removed — updated all test helpers to the new API"
  - "pyo3 auto-initialize in [dev-dependencies] only: production cdylib uses extension-module (interpreter provided by Python); test binary needs auto-initialize to bootstrap"
  - "verify_override_impl rename: #[pyfunction] verify_override is the public Python API; the Rust-internal verify_override_impl retains #[must_use] for SC5 Rust callers"
  - "#[expect(dead_code)] on OutOfScope: forward-declared for Phase 92 scope enforcement; Phase 92 will remove it when adding the first non-test construction site"
metrics:
  duration_minutes: 45
  completed_date: "2026-06-21T00:00:00Z"
  tasks_completed: 2
  files_created: 0
  files_modified: 3
---

# Phase 91 Plan 03: PyO3 Boundary — Frozen OverrideGrant, NonoOverrideError, verify_override Summary

Frozen `OverrideGrant` pyclass, repo's first custom exception `NonoOverrideError` via `create_exception!`, and `#[pyfunction] verify_override` wrapper — every `Err` raises `NonoOverrideError` with the stable machine-readable kind as `args[0]`. All three surfaces registered on `_nono_py`. 50 tests pass; clippy clean.

## Tasks Completed

| Task | Name | Commit (nono-py) | Files |
|------|------|------------------|-------|
| 1 | Frozen OverrideGrant pyclass + NonoOverrideError + override_err_to_py + verify_override #[pyfunction] | e138fc2 | nono-py/src/override.rs, src/lib.rs, Cargo.toml |
| 2 | Register on _nono_py + SC4 pyo3_boundary assertions + SC5 must-use check | e138fc2 | (same commit — both tasks compiled and tested together) |

Note: Tasks 1 and 2 were committed together because the PyO3 registration in lib.rs (Task 2) is required for Task 1's `#[pyfunction]` to compile (the module name `_nono_py` in `create_exception!` must match), and the SC4 pyo3_boundary tests (Task 2) depend on the #[pyfunction] wrapper from Task 1. Both are pure additive changes with no shared mutable state.

## Outcome

- SC4 MET: `cargo test -p nono-py override_mod::pyo3 2>&1` → `11 passed; 0 failed`. Every `OverrideErrorKind` variant raises `NonoOverrideError` (not RuntimeError/ValueError/None); `args[0]` is the stable kind string.
- SC5 MET: `#[must_use = "verify_override Result must be checked — dropping it silently may grant access"]` on `verify_override_impl`. Negative-control documented in `pyo3_boundary` module comment: ignoring the result in Rust produces `unused_must_use` warning; verified via manual `cargo build 2>&1 | grep must_use` during development.
- `cargo build -p nono-py` compiles clean (0 warnings; `OutOfScope` handled via `#[expect(dead_code)]`).
- `cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` clean.
- 50 total tests pass (39 prior + 11 new pyo3_boundary SC4 assertions).
- `grep -n 'pyclass(frozen' nono-py/src/override.rs` matches line 663 on OverrideGrant.
- `grep -n 'create_exception' nono-py/src/override.rs` matches line 39 (repo's first).
- `grep -nE 'OverrideGrant|verify_override|NonoOverrideError' nono-py/src/lib.rs` shows all three in `_nono_py` at lines 768-773.

## PyO3 0.28 Idiom (A2 Outcome)

**create_exception! registration (verified against pyo3-0.28.3/src/exceptions.rs):**
```rust
pyo3::create_exception!(
    _nono_py,
    NonoOverrideError,
    pyo3::exceptions::PyException,
    "docstring"
);
// In #[pymodule]:
m.add("NonoOverrideError", m.py().get_type::<override_mod::NonoOverrideError>())?;
```
This matches the RESEARCH A2 assumption exactly — no adjustment needed.

**API change in PyO3 0.28:** `Python::with_gil` → `Python::attach`; `Python::allow_threads` removed. Updated all pyo3_boundary test helpers to use `Python::attach`.

## SC5 Must-Use Mechanism

`verify_override_impl` carries `#[must_use = "verify_override Result must be checked — dropping it silently may grant access"]` (inherited from Plan 02).

The `#[pyfunction] verify_override` wrapper returns `PyResult<OverrideGrant>` — PyO3 forces the Python caller to handle the exception (cannot return None/falsy). The Rust-level `#[must_use]` on `verify_override_impl` prevents RUST callers from silently dropping the result.

Negative-control assertion (documented in `pyo3_boundary` module comment, NOT added to production code):
```
// SC5: let _ = verify_override_impl("", b"", &[], None);  // triggers unused_must_use warning
// Verify: cargo build 2>&1 | grep must_use
```

## Python-Side pytest

No Python-side pytest was required. All SC4 assertions are implemented as Rust `#[cfg(test)]` tests using `Python::attach` + PyErr downcast. The `cargo test -p nono-py override_mod::pyo3` command is sufficient — no `maturin develop` step needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] PyO3 0.28 changed Python::with_gil to Python::attach**
- **Found during:** Task 2 — pyo3_boundary tests failed to compile with "no function or associated item named `with_gil` found"
- **Issue:** PyO3 0.28.x renamed `Python::with_gil` to `Python::attach` and removed `Python::allow_threads`
- **Fix:** Updated all test helpers to use `Python::attach(|py| { ... })` pattern
- **Files modified:** nono-py/src/override.rs (all `Python::with_gil` and `Python::allow_threads` calls in pyo3_boundary)
- **Commit:** e138fc2

**2. [Rule 3 - Blocking] Python interpreter not initialized in cargo test context**
- **Found during:** Task 2 — all pyo3_boundary tests panicked with "The Python interpreter is not initialized and the `auto-initialize` feature is not enabled"
- **Issue:** Production cdylib uses `extension-module` feature (Python provides the interpreter); `cargo test` builds a native binary where no Python is running yet
- **Fix:** Added `pyo3 = { version = "0.28", features = ["extension-module", "auto-initialize"] }` to `[dev-dependencies]` (affects test binary only; production cdylib unchanged)
- **Files modified:** nono-py/Cargo.toml
- **Commit:** e138fc2

**3. [Rule 2 - Missing Critical] skip_from_py_object required on frozen OverrideGrant**
- **Found during:** Task 1 build — deprecated warning about `FromPyObject` impl for Clone `#[pyclass]`
- **Issue:** PyO3 0.28 changed how Clone pyclasses expose `FromPyObject`; `skip_from_py_object` is now mandatory for frozen classes (matches existing `FsCapability`/`ExecResult` pattern in repo)
- **Fix:** Added `#[pyclass(frozen, skip_from_py_object)]` instead of `#[pyclass(frozen)]`
- **Files modified:** nono-py/src/override.rs (line 663)
- **Commit:** e138fc2

**4. [Rule 1 - Bug] verify_override #[pyfunction] name collision with existing core function**
- **Found during:** Task 1 design — the Plan 02 `pub fn verify_override(...)` and Plan 03's `#[pyfunction] verify_override(...)` cannot both be `pub` with the same name
- **Fix:** Renamed Plan 02 core to `pub(crate) fn verify_override_impl`; Plan 03 `#[pyfunction] fn verify_override` is the public Python-facing wrapper; all tests updated to call `verify_override_impl`
- **Files modified:** nono-py/src/override.rs
- **Commit:** e138fc2

**5. [Rule 2 - CLAUDE.md] OutOfScope variant constructed only in #[cfg(test)] — clippy -D dead-code**
- **Found during:** Task 2 clippy run with `-D warnings`
- **Issue:** `OverrideErrorKind::OutOfScope` is forward-declared for Phase 92; production code doesn't construct it; `#[cfg(test)]` construction in pyo3_boundary tests is invisible to clippy's lib check
- **Fix:** Added `#[expect(dead_code, reason = "forward-declared for Phase 92 scope enforcement; remove when Phase 92 constructs it")]` to the variant. Used `#[expect]` (not `#[allow]`) per CLAUDE.md guidance — the attribute will error if Phase 92 adds a non-test construction site and the attribute is not removed
- **Files modified:** nono-py/src/override.rs
- **Commit:** e138fc2

## Requirement Coverage

| Requirement | Status |
|-------------|--------|
| VFY-07 (OverrideGrant immutable, fail-closed PyO3 boundary) | DONE — #[pyclass(frozen)] + NonoOverrideError raised for every Err (SC4 + SC5) |

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes introduced.

- T-91-03-FALSY: `#[pyfunction] verify_override` returns `PyResult<OverrideGrant>` — every `Err` raises `NonoOverrideError`; no path returns `None`/falsy. SC4 pyo3_boundary tests confirm. MET.
- T-91-03-WRONGEXC: `override_err_to_py` exclusively uses `NonoOverrideError::new_err` — never a built-in. All 11 pyo3_boundary tests downcast to `NonoOverrideError`. MET.
- T-91-03-MUTGRANT: `#[pyclass(frozen, skip_from_py_object)]` — Python cannot mutate `OverrideGrant` fields after verification. MET.
- T-91-03-LEAK: `override_err_to_py` message = `format!("override verification denied: {}", kind.as_str())` — no raw signature, no key DER bytes, no path info. `__repr__` shows jti + expiry count only. MET.
- T-91-03-MUSTUSE: `#[must_use]` on `verify_override_impl` (Plan 02) + SC5 negative-control documented. MET.

## Known Stubs

All stubs inherited from Plans 01/02 remain unchanged:
- **VFY-03 clause (a):** `pubkey_der` param is test-injected. Production pubkey sourcing is `[BLOCKING-93]` Phase 93.
- **jti cross-process:** `CONSUMED_JTIS` is in-process only. Phase 93 durable enforcement.
- **Scope enforcement:** `partition_scope` validates format but does NOT enforce against sandbox policy. Phase 92.
- **Operator TTL config:** `TTL_CAP_SECONDS = 8h` hard-coded. Phase 93.

## Self-Check: PASSED

**nono-py commits (branch 44-broker-ffi-lockstep):**
- `14aafc1` feat(91-01): scaffold override.rs module...
- `a6128fb` feat(91-01): implement canonical_bytes/sha256...
- `61d9811` feat(91-02): implement ECDSA verify pipeline + OverrideGrant + jti replay
- `e138fc2` feat(91-03): expose OverrideGrant pyclass + NonoOverrideError + verify_override #[pyfunction]

**Files confirmed present:**
- nono-py/src/override.rs — `pyclass(frozen` at line 663, `create_exception!` at line 39, `override_err_to_py` function, `#[pyfunction] verify_override` at line 827, `pyo3_boundary` test module with 11 tests
- nono-py/src/lib.rs — `OverrideGrant`, `verify_override`, `NonoOverrideError` registrations at lines 768-773
- nono-py/Cargo.toml — [dev-dependencies] pyo3 auto-initialize

**Test results:** `cargo test -p nono-py` → `50 passed; 0 failed` (39 prior + 11 pyo3_boundary)
**Clippy:** `cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` → clean
