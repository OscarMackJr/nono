---
phase: 93-live-zt-infra-integration-revocation-request-flow
plan: "05"
subsystem: nono-py
tags: [cli-02, console-script, verify-then-run, d-07, d-08, oq-5, vfy-01]
dependency_graph:
  requires: ["93-04"]
  provides: ["CLI-02 console-script nono-override-apply", "D-08 one-shot verify-then-run", "OQ-5 project.scripts entry"]
  affects: ["nono-py apply path", "operator CLI surface"]
tech_stack:
  added: []
  patterns: ["TDD RED/GREEN", "module-level imports for patchability", "fail-closed argv split guard", "lazy-extension-not-available guard"]
key_files:
  created:
    - C:/Users/OMack/nono-py/python/nono_py/_cli_apply.py
    - C:/Users/OMack/nono-py/tests/test_override_apply.py
  modified:
    - C:/Users/OMack/nono-py/pyproject.toml
decisions:
  - "D-07 honored: nono-override-apply is a standalone nono-py console-script; no nono.exe->nono-py shell-back"
  - "D-08 honored: offline verify_override() then _live.live_check() before any confined_run_checked() call; any NonoOverrideError blocks exec"
  - "OQ-5 honored: [project.scripts] nono-override-apply = nono_py._cli_apply:main registered in pyproject.toml"
  - "Module-level imports adopted (not lazy-inside-function) so mock.patch targets module attributes correctly in tests"
  - "T-93-05-03 honored: scope path handling delegated to confined_run_checked/sanitize_override_path; no string starts_with on paths in _cli_apply.py"
metrics:
  duration_minutes: 25
  completed_date: "2026-06-23T02:00:00Z"
  tasks_completed: 2
  files_changed: 3
---

# Phase 93 Plan 05: nono-override-apply Console-Script (CLI-02) Summary

**One-liner:** `nono-override-apply` console-script delivers the CLI-02 affordance as a one-shot verify-then-run entry (offline ECDSA + live /actions AND-gate before any confined exec), registered in `pyproject.toml` `[project.scripts]` with no nono.exe shell-back (D-07/D-08/OQ-5).

## Tasks Completed

| Task | Name | Commit (nono-py) | Files |
|------|------|------------------|-------|
| RED  | Failing tests for _cli_apply.py (TDD RED) | `f89543e` | `tests/test_override_apply.py` |
| 1 | `_cli_apply.py` — verify-then-run console-script (CLI-02 / D-08) | `45e2d0e` | `python/nono_py/_cli_apply.py` |
| 2 | Register `[project.scripts]` console-entry (OQ-5) | `2c8734c` | `pyproject.toml` |

## What Was Built

### Task 1 — `_cli_apply.py` (TDD RED + GREEN)

`_cli_apply.py` implements `main(argv=None)`:

- Parses `<token-path> -- <command...>` splitting on the `--` separator; exits code 2 (usage error) if `--` is absent, token-path is missing, or command is empty.
- Reads the token file; exits code 3 on `OSError`.
- Calls `verify_override(token_str)` (offline ECDSA via the Rust extension); any exception → exit code 1, `confined_run_checked` never called.
- Reads `NONO_ZT_ACTIONS_URL` from env; absent/empty → exit code 1, fail-closed (D-04).
- Calls `_live.live_check(actions_url, grant, timeout, extra_header)` from Plan 04; `NonoOverrideError(LiveRevoked)` or `(LiveUnavailable)` → exit code 1, `confined_run_checked` never called.
- On complete pass, calls `confined_run_checked(exe=..., args=..., override_token=grant)` — scope paths delegated component-wise to `sanitize_override_path` in `windows_confined_run.rs` (T-93-05-03; never string `starts_with` on paths).
- Module-level imports (`verify_override`, `_live`, `confined_run_checked`) wrapped in `try/except ImportError` so the module is importable on non-Windows CI; failures surfaced with user-friendly messages at call time.

`test_override_apply.py` (12 tests):
- `TestFullPassRunsCommand`: full offline+live pass → `confined_run_checked` called; live hash threaded
- `TestOfflineFailBlocks`: `verify_override` raises → `confined_run_checked` NOT called (2 tests)
- `TestLiveFailBlocks`: `live_check` raises `LiveRevoked`/`LiveUnavailable` → NOT called (2 tests)
- `TestArgvSplit`: `--` split correctness, no-`--` guard, empty-command guard (3 tests)
- `TestMissingActionsUrl`: absent/empty URL → fail-closed (2 tests)
- `TestTokenFileRead`: missing file → non-zero exit without confined_run (1 test)

### Task 2 — `pyproject.toml` `[project.scripts]`

Added a new `[project.scripts]` table immediately before `[project.urls]`:

```toml
[project.scripts]
nono-override-apply = "nono_py._cli_apply:main"
```

With a comment noting that `pip install -e .` (or `maturin develop`) must be re-run after adding `[project.scripts]` for the console-script to appear in the venv `Scripts/`. `[tool.maturin]` (`python-source = "python"`, `module-name = "nono_py._nono_py"`) is unchanged.

## Verification Results

| Check | Result |
|-------|--------|
| `pytest tests/test_override_apply.py -x -q` | 12/12 PASS |
| `pytest tests/test_live_arm.py -x -q` | 16/16 PASS (Plan 04 regression; no regressions) |
| `python -c "import tomllib; d=tomllib.load(...); print(d['project']['scripts']['nono-override-apply'])"` | `nono_py._cli_apply:main` |
| `[tool.maturin]` unchanged | `python-source=python`, `module-name=nono_py._nono_py` ✓ |
| `cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` | Clean (0 warnings, no Rust changes in this plan) |
| `maturin develop` / extension rebuild | PARTIAL→host (maturin not available on this Windows host; pure-Python `_cli_apply.py` tests do not require the rebuilt extension — they run with mocks) |

## Deviations from Plan

**1. [Rule 2 - Missing] Module-level imports instead of lazy-inside-function**

- **Found during:** Task 1 (TDD GREEN)
- **Issue:** The plan's action used lazy `from nono_py import X` inside `main()` bodies. `unittest.mock.patch("nono_py._cli_apply.verify_override", ...)` requires `verify_override` to be an attribute on the `_cli_apply` module object at the time the patch is applied — lazy import means the attribute does not exist until the function runs, so `patch` raises `AttributeError`.
- **Fix:** Moved `verify_override`, `_live`, and `confined_run_checked` to module-level imports wrapped in `try/except ImportError = None`. This is the standard pattern for mockable modules and is consistent with how `_live.py` imports `NonoOverrideError` lazily for non-Windows tests. Pure-Python tests pass without the compiled extension because the `None` guards surface the error at runtime, not import time.
- **Files modified:** `python/nono_py/_cli_apply.py`
- **Commit:** `45e2d0e`

## Known Stubs

None. `_cli_apply.py` calls through to the real `verify_override` (Rust extension), real `_live.live_check` (Plan 04), and real `confined_run_checked` (Plan 04) on the pass path.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes beyond those registered in the plan's `<threat_model>`. The `_cli_apply.py` module:
- Reads one token file (new file-I/O surface; bounded to the path supplied by the caller)
- Makes one live call via `_live.live_check` (already registered as T-93-05-01/T-93-05-02)
- Delegates to `confined_run_checked` (Windows-only, already registered as T-93-05-03/T-93-05-04)

No new trust boundaries introduced beyond those in the plan threat register.

## Self-Check: PASSED

- `C:/Users/OMack/nono-py/python/nono_py/_cli_apply.py` — FOUND (committed `45e2d0e`)
- `C:/Users/OMack/nono-py/tests/test_override_apply.py` — FOUND (committed `f89543e` RED + `45e2d0e` GREEN)
- `C:/Users/OMack/nono-py/pyproject.toml` `[project.scripts]` — FOUND (committed `2c8734c`)
- nono-py commit `f89543e` (TDD RED) — FOUND in nono-py git log
- nono-py commit `45e2d0e` (GREEN implementation) — FOUND in nono-py git log
- nono-py commit `2c8734c` (pyproject.toml) — FOUND in nono-py git log
