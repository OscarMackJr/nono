---
phase: 72-nono-py-binding-in-process-exec-proof
plan: 04
subsystem: nono-py
tags: [windows, nono-py, langchain, pyo3, confined_run, confine, shape-b, appcontainer, low-il, pytest]

# Dependency graph
requires:
  - phase: 72-02
    provides: "confined_run (Shape A) + confine (Shape B) Windows PyO3 binding; ExecResult pyclass"

provides:
  - "examples/15_langchain_confined.py: Shape B born-confined PythonREPLTool proof with graceful langchain-absent fallback"
  - "tests/test_confined_run.py: 4 win32-only pytest tests with nono_exe_path fixture; importorskip guard; proven invocation contract"
  - "DEFERRED: live UAT (build + run) to orchestrator + human operator per Task 2 checkpoint"

affects:
  - 72-UAT
  - langchain-python-profile-users

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "importorskip guard: nono_py = pytest.importorskip('nono_py') at module level so --collect-only is clean before maturin build"
    - "nono_exe_path fixture: NONO_EXE env -> sibling repo path -> shutil.which; session-scoped; pytest.skip if absent"
    - "confine()-guard test: monkeypatch.setenv('NONO_ALREADY_CONFINED', '1') exercises T-72-02-01 guard without spawning nono.exe"
    - "Allow list: always pass both workspace AND interpreter_dir (Path(sys.executable).parent) to satisfy langchain-python interpreter-coverage gate"
    - "Shape B example degradation: try/except ImportError around langchain -> HAS_LANGCHAIN flag -> fallback to confined_run proof"

key-files:
  created:
    - ../nono-py/examples/15_langchain_confined.py
    - ../nono-py/tests/test_confined_run.py
    - .planning/phases/72-nono-py-binding-in-process-exec-proof/72-04-SUMMARY.md
  modified: []

key-decisions:
  - "Test uses allow=[str(tmp_path), interpreter_dir] (not just tmp_path) to match proven 72-01 invocation contract; interpreter_dir required by langchain-python engine-coverage gate"
  - "Outside path is tmp_path.parent / 'nono_outside_probe.txt' not a hardcoded C:\\ path (T-72-04-04 mitigation)"
  - "test_confine_already_confined_guard does NOT request nono_exe_path fixture — guard fires before find_nono_exe(); test works without nono.exe present"
  - "Example uses confine() for Shape B (langchain path) and confined_run() for fallback (no-langchain path) — both code paths documented and validated"
  - "Live pytest run deferred to orchestrator: maturin develop + pytest -m integration requires user's env (cannot pip install or run maturin here per AUTHORING-PHASE objective)"

patterns-established:
  - "Win32-only test file pattern: pytestmark at module level + importorskip + conditional import inside if sys.platform == 'win32'"
  - "nono_exe_path fixture resolution order: NONO_EXE env -> ../Nono/target/release/nono.exe sibling -> PATH; documented in fixture docstring"

requirements-completed: [ABI-01]

# Metrics
duration: ~45 min (authoring only; UAT deferred)
completed: 2026-06-14
---

# Phase 72 Plan 04: LangChain Confined Example + Pytest Suite Summary

**Shape B born-confined LangChain PythonREPLTool proof (examples/15_langchain_confined.py) + 4-test win32-only pytest suite (tests/test_confined_run.py) authored with proven 72-01 invocation contract; py_compile PASS; --collect-only skips cleanly; live UAT deferred to orchestrator.**

## Performance

- **Duration:** ~45 min (authoring phase only; per objective, no build/live-run)
- **Started:** 2026-06-14
- **Completed:** 2026-06-14
- **Tasks:** 1 of 2 (Task 1: write code + commit; Task 2: human-verify checkpoint — awaiting orchestrator)
- **Files modified:** 2 (nono-py repo: examples + tests)

## Accomplishments

- `examples/15_langchain_confined.py` written: Shape B born-confined proof via `confine(profile='langchain-python', allow=[ws, interpreter_dir])`; `PythonREPLTool` deny + allow assertions with `[PASS]`/`[FAIL]` output; degrades to `confined_run` fallback when langchain not installed (graceful degradation per robustness requirement)
- `tests/test_confined_run.py` written: 4 win32-only integration tests; module-level `pytestmark` + `pytest.importorskip("nono_py")` so `--collect-only` is clean before build; `nono_exe_path` session fixture with 3-step resolution; API matched exactly to `windows_confined_run.rs` `#[pyo3(signature)]` declarations
- py_compile PASS on both files (Python 3.12.10)
- pytest `--collect-only` with `--noconftest` shows "1 skipped" (clean — importorskip fires when nono_py not built); no collection errors

## Task Commits

**Task 1 commit (nono-py repo):**
- `989ae24` — `feat(72-04): add LangChain confined example + pytest test suite`

**Plan metadata (nono repo, this commit):**
- see final commit below

**Task 2: NOT YET EXECUTED** — Human-verify checkpoint awaiting orchestrator (`maturin develop` + `pytest -m integration -v` + example run on Win11)

## Files Created/Modified

**nono-py repo (`C:\Users\OMack\nono-py`, branch `44-broker-ffi-lockstep`):**
- `examples/15_langchain_confined.py` — NEW; Shape B born-confined PythonREPLTool proof; fallback to confined_run; exits 0 without langchain; `[PASS]`/`[FAIL]` verdict output
- `tests/test_confined_run.py` — NEW; 4 win32-only integration tests; session `nono_exe_path` fixture; importorskip guard; `test_write_outside_workspace_denied` asserts `exit_code != 0 AND not outside.exists()`

**nono repo (`C:\Users\OMack\Nono`, main tree):**
- `.planning/phases/72-nono-py-binding-in-process-exec-proof/72-04-SUMMARY.md` — This file

## Decisions Made

**Invocation contract reproduction:** Tests use `allow=[str(tmp_path), interpreter_dir]` where `interpreter_dir = str(Path(sys.executable).parent)`. This exactly reproduces the proven 72-01 spike contract: the `langchain-python` profile's `windows_interpreters` engine-coverage gate requires the directly confined command (`python.exe`) to reside in an `--allow`-covered directory. Without `interpreter_dir`, nono would deny the spawn at the interpreter-path coverage check (fail-secure gate).

**Outside path choice:** `tmp_path.parent / "nono_outside_probe.txt"` rather than `C:\\outside_test.txt`. The plan's threat model (T-72-04-04) notes that hardcoded `C:\\` paths may behave differently across environments. `tmp_path.parent` is reliably one level above the granted workspace — always outside `allow=[str(tmp_path)]`.

**Guard test without nono.exe:** `test_confine_already_confined_guard` deliberately does NOT request the `nono_exe_path` fixture. The `NONO_ALREADY_CONFINED` guard fires inside `confine()` before `find_nono_exe()` is called (per `windows_confined_run.rs` line 263 — first thing in the function body). So the test works even when `NONO_EXE` is not set and `nono.exe` is absent — a targeted unit-like test for the infinite-loop guard (T-72-02-01, T-72-04-03).

**Example degradation strategy:** The example uses `HAS_LANGCHAIN` flag + `HAS_NONO_PY` flag to produce clear human-readable messages rather than crashing. The Shape B born-confined path (`confine()`) is only taken when both flags are true; the fallback (`confined_run`) is taken when langchain is absent but nono_py is built. If nono_py itself is not built, the example exits 1 with a clear build instruction.

## Deviations from Plan

None — plan executed exactly as written. Both files produced per specification. All robustness requirements met (importorskip guard, nono_exe_path fixture with 3-step resolution, 4 tests including guard test, proven invocation contract, graceful langchain-absent fallback).

## Issues Encountered

**pytest not installed in current Python env:** The bash environment's Python 3.12 and 3.14 do not have pytest installed, and no `.venv` exists in the nono-py repo (uv not on PATH in this shell). Pytest was located in a sibling project's venv (`cursor_loan_engine/backend/venv`). Used `sys.path.insert()` to load pytest 8.3.3 for the `--collect-only` validation.

**Broken sibling venvs:** The sibling venvs were built against `C:\Python312` which no longer exists (Python 3.12 is now at `C:\Users\OMack\AppData\Local\Programs\Python\Python312`). Only `--noconftest` mode was usable (conftest.py imports nono_py which is not built). This is expected behavior for an authoring-only phase with no maturin build.

**`--collect-only` output interpretation:** Exit code 5 ("no tests collected") + "0 items / 1 skipped" is the CORRECT behavior: on win32 platform Python, `importorskip("nono_py")` fires first (nono_py extension not built) and skips the entire file as a module. This is the intended clean-collection gate. The live `--collect-only` with a built nono_py will show 4 items.

## User Setup Required

**Prerequisite (operator action before Task 2 UAT):**
1. Ensure `nono.exe` is built: `cargo build --release -p nono-cli` from `C:\Users\OMack\Nono`
2. Set `NONO_EXE=C:\Users\OMack\Nono\target\release\nono.exe` (or ensure it is on PATH)
3. Build the nono-py extension: `maturin develop` or `maturin develop --release` from `C:\Users\OMack\nono-py`
4. Optional (for full LangChain proof): `pip install langchain langchain_experimental`

**UAT commands (from PowerShell in `C:\Users\OMack\nono-py`):**
```powershell
# Step 1: pytest integration suite
python -m pytest tests/test_confined_run.py -v

# Step 2: LangChain example (Shape B proof or fallback)
python examples/15_langchain_confined.py

# Step 3: Verify no nono-wfp-service dependency (example must work without WFP service)
#         If example fails with "WFP service not running" — report as gap
```

## Pytest --collect-only Output (authoring phase, before build)

```
============================= test session starts =============================
collecting ... collected 0 items / 1 skipped

========================= no tests collected in 0.02s =========================
```

Result: `importorskip("nono_py")` fires because extension not built — correct clean-skip behavior. No collection errors.

## API Match Verification

Signatures from `C:\Users\OMack\nono-py\src\windows_confined_run.rs`:

**`confined_run`** (line 175-184):
```rust
#[pyfunction]
#[pyo3(signature = (exe, args, allow=None, profile=None, cwd=None, timeout_secs=None))]
pub fn confined_run(
    py: Python<'_>,
    exe: String,
    args: Vec<String>,
    allow: Option<Vec<String>>,
    profile: Option<String>,
    cwd: Option<String>,
    timeout_secs: Option<f64>,
) -> PyResult<ExecResult>
```

**`confine`** (line 254-259):
```rust
#[pyfunction]
#[pyo3(signature = (profile=None, allow=None, caps=None))]
pub fn confine(
    profile: Option<String>,
    allow: Option<Vec<String>>,
    caps: Option<&CapabilitySet>,
) -> PyResult<()>
```

**`ExecResult` fields** (lines 33-40):
```rust
#[pyclass(frozen)]
pub struct ExecResult {
    #[pyo3(get)]
    pub stdout: Vec<u8>,
    #[pyo3(get)]
    pub stderr: Vec<u8>,
    #[pyo3(get)]
    pub exit_code: i32,
}
```

Tests use: `result.exit_code`, `result.stdout`, `result.stderr` — all match the `#[pyo3(get)]` fields. Function calls use exact kwarg names from `#[pyo3(signature)]`.

## Live UAT Status: DEFERRED

Per the plan objective ("AUTHORING PHASE ONLY"), the live build + enforcement test is deferred to the orchestrator. The Task 2 checkpoint (`checkpoint:human-verify`) is the gate:

- **Resume signal:** "uat-passed" with pytest output, or "uat-failed: reason"
- **SC1:** pytest `test_write_outside_workspace_denied` exits 0 with `exit_code != 0` AND file not created
- **SC2:** `examples/15_langchain_confined.py` prints `[PASS]` for both deny and allow assertions (with or without langchain)
- **SC3:** `test_confine_already_confined_guard` PASS confirms `NONO_ALREADY_CONFINED` guard works

## Known Stubs

None. `confine()` and `confined_run()` are fully wired in `windows_confined_run.rs`. The example and tests reference real API functions — no placeholder logic.

## Threat Flags

No new network endpoints, auth paths, or schema changes beyond what the plan's `<threat_model>` covers. Both new files are test/example artifacts; they call the existing `confined_run`/`confine` binding functions documented in the threat register.

---
*Phase: 72-nono-py-binding-in-process-exec-proof*
*Completed: 2026-06-14 (authoring; UAT deferred)*
