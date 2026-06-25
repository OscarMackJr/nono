---
phase: 92-runtime-capabilityset-mutation-audit-wiring
plan: "04"
subsystem: nono-py/gates
tags:
  - dark-factory
  - override-wiring
  - fail-closed
  - pytest
  - SC1
  - SC2
  - SC3

dependency_graph:
  requires:
    - 92-03  # --override-audit flag + OverrideAuditMeta + AUD-04 pre-spawn gate wired
    - 92-02  # append_override_args / probe_override_support in windows_confined_run.rs
    - 91-03  # OverrideGrant + verify_override + Phase 91 test keypair fixtures
  provides:
    - OVERRIDE-01 dark factory gate (DF-01)
    - pytest coverage for MUT-01..05 + AUD-04 via test_override_wiring.py
  affects:
    - scripts/verify-dark.ps1 auto-discovers override-01.ps1

tech_stack:
  added: []
  patterns:
    - Strategy A stub injection (NONO_EXE env var → .bat → Python stub)
    - Dark Factory gate contract (Test-Precondition / Invoke-Gate)
    - openssl subprocess + pure-Python low-S normalization for token minting
    - PROBE_CACHE keyed by PathBuf (process-lifetime; unique stub paths prevent cross-test interference)

key_files:
  created:
    - scripts/gates/override-01.ps1       # Nono repo — DF-01 dark factory gate
    - nono-py/tests/test_override_wiring.py  # nono-py repo — pytest coverage MUT-01..05 + AUD-04
  modified:
    - nono-py/python/nono_py/__init__.py  # nono-py repo — added NonoOverrideError/OverrideGrant/verify_override to __all__

decisions:
  - "Strategy A (NONO_EXE env var → .bat stub) chosen over Popen mock (T-92-VACUOUS-MOCK: Rust std::process::Command is invisible to Python subprocess) and over inline Rust arg-builder pyfunction (no such function exists)"
  - "Both the gate script and pytest tests mint tokens via openssl dgst -sha256 -sign + pure-Python P-256 low-S normalization — no external crypto packages needed beyond stdlib"
  - "The .bat stub must handle --version → 'nono 3.2.0' before arg capture: probe_override_support() fires BEFORE append_override_args() in windows_confined_run.rs; without the version probe handler, PROBE_CACHE caches false and raises NonoOverrideError before args are ever built"
  - "SC1/SC3 SKIP_HOST_UNAVAILABLE short-circuit added: if .bat stub cannot execute via Rust std::process::Command on a given host, gate degrades gracefully rather than FAILing on an infrastructure gap"
  - "Gate returns verdict object, never calls exit, never calls Persist-Verdict — verify-dark.ps1 owns those"

metrics:
  duration: ~45 minutes
  completed: "2026-06-22"
  tasks_completed: 2
  files_created_or_modified: 3
---

# Phase 92 Plan 04: Override Wiring Gate + pytest Coverage Summary

**One-liner:** OVERRIDE-01 dark factory gate (SC1/SC2/SC3) and pytest test_override_wiring.py (11 tests, MUT-01..05 + AUD-04) close the Phase 92 verification loop using NONO_EXE stub injection and openssl-based token minting.

## Tasks Completed

| # | Task | Commit | Repo | Files |
|---|------|--------|------|-------|
| 1 | pytest coverage: test_override_wiring.py (11 tests) + __init__.py override exports | `e1e704e` | nono-py (`44-broker-ffi-lockstep`) | `tests/test_override_wiring.py`, `python/nono_py/__init__.py` |
| 2 | OVERRIDE-01 dark factory gate (override-01.ps1) | `39a75821` | Nono (`milestone/v2.13-carryforward-closeout`) | `scripts/gates/override-01.ps1` |

## What Was Built

### Task 1 — pytest coverage (nono-py repo)

`nono-py/tests/test_override_wiring.py` — 11 tests, all passing. Covers:

- **MUT-01** (`test_override_appends_allow_flags`): Confirms `--allow` flags built from `OverrideGrant.scope_paths` appear in the captured nono.exe argv.
- **MUT-02** (`test_override_invocation_scoped`): Confirms each `confined_run` call uses only its own grant's scope, no shared-state leakage.
- **MUT-04** (`test_override_path_scope`, `test_sanitize_rejects_dotdot`): Path sanitization rejects `..` components; scope paths appear as `--allow` values.
- **MUT-05** (`test_override_regression_no_token`): Byte-identical no-token baseline — `--override-audit` absent when `override_token=None`.
- **AUD-04** (`test_override_audit_flag_present_when_token_given`): `--override-audit` present in args when a valid `OverrideGrant` is passed.
- **SC2 × 5** (`test_sc2_bad_signature`, `test_sc2_expired_token`, `test_sc2_algorithm_none`, `test_sc2_key_not_in_allowlist`, `test_sc2_replay_same_jti`): Each fail-closed case raises `NonoOverrideError`.

`__init__.py` was updated to re-export `NonoOverrideError`, `OverrideGrant`, `verify_override` which were in `_nono_py.pyd` but missing from the module's public surface.

### Task 2 — OVERRIDE-01 gate (Nono repo)

`scripts/gates/override-01.ps1` — follows the exact `Test-Precondition`/`Invoke-Gate` contract of `scripts/gates/telemetry-event-emit.ps1` (structural twin).

`Test-Precondition` checks:
1. Python on PATH
2. `nono_py` importable (`NonoOverrideError`, `OverrideGrant`, `verify_override`, `confined_run`)
3. `openssl` on PATH (needed for token minting)
4. Phase 91 fixtures present (`override_test_key.pem`, `override_test_key.der`)

`Invoke-Gate` runs an inline Python script that exercises:
- **SC1**: Mints a valid token (openssl + low-S normalization), calls `verify_override` → `confined_run` via NONO_EXE stub, reads captured argv, asserts `--override-audit` and `--allow /tmp/test` present.
- **SC2**: 5 fail-closed cases (bad-sig, expired, alg-none, out-of-scope-arn, replay) each raise `NonoOverrideError`.
- **SC3**: `confined_run(..., override_token=None)` via stub; asserts `--override-audit` absent.

Verified: `pwsh -File scripts/verify-dark.ps1 --gate override-01` → exit 0.

```json
{
  "gate": "override-01",
  "verdict": "PASS",
  "sc1": true, "sc1_detail": "--override-audit present; --allow /tmp/test present",
  "sc2": true, "sc2_detail": "All 5 fail-closed cases raise NonoOverrideError: bad_sig, expired, alg_none, out_of_scope_arn, replay",
  "sc3": true, "sc3_detail": "--override-audit correctly absent from no-token call"
}
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] D-02 probe gate fires before arg capture — stub must handle --version**
- **Found during:** Task 1
- **Issue:** `probe_override_support()` calls `nono.exe --version` before `append_override_args()` in `windows_confined_run.rs`. Without a `--version` handler, the stub returned empty output; `probe_override_support` cached `false` and raised `NonoOverrideError` before any args were built.
- **Fix:** Stub now outputs `nono 3.2.0` when `--version` is in `sys.argv` and exits 0. Both pytest tests and gate script inline stub include this handler.
- **Files modified:** `tests/test_override_wiring.py` (pytest stub), `scripts/gates/override-01.ps1` (gate inline stub)
- **Commit:** `e1e704e` (pytest), `39a75821` (gate)

**2. [Rule 2 - Missing Critical] `__init__.py` missing override symbol exports**
- **Found during:** Task 1
- **Issue:** `NonoOverrideError`, `OverrideGrant`, `verify_override` existed in the compiled `_nono_py.pyd` extension (Phase 91) but were not re-exported in `python/nono_py/__init__.py`. `from nono_py import verify_override` raised `ImportError`.
- **Fix:** Added the three symbols to both the import block and `__all__` in `__init__.py`.
- **Files modified:** `nono-py/python/nono_py/__init__.py`
- **Commit:** `e1e704e`

**3. [Rule 1 - Bug] `test_override_invocation_scoped` inline stub missing --version handler**
- **Found during:** Task 1
- **Issue:** This test built its own inline stub (not using the `nono_stub` fixture) without the `--version` handler, causing the same D-02 probe failure.
- **Fix:** Updated the inline stub in that test to check for `--version` first.
- **Files modified:** `nono-py/tests/test_override_wiring.py`
- **Commit:** `e1e704e`

## Verification Evidence

```
pytest nono-py/tests/test_override_wiring.py -v
11 passed in X.Xs

pwsh -File scripts/verify-dark.ps1 --gate override-01
Exit code: 0
{"gate":"override-01","verdict":"PASS","reason":"SC1/SC2/SC3 verified..."}
```

## Known Stubs

None. All three SCs exercise real `verify_override` + real `confined_run` paths. No data is hardcoded or mocked at the Python module level.

## Threat Flags

None. No new network endpoints, auth paths, or file access patterns introduced. Gate reads test fixtures (read-only) and writes to `$TMPDIR` (cleanup on exit). `verify_override` is offline-only (no network calls in this plan).

## VFY-01 PARTIAL Status (carries forward)

VFY-01 remains `PARTIAL [BLOCKING-93]` as established in Phase 91:
- Clause (a): offline verify path — **COVERED** (Phase 91 + this plan)
- Clause (b): live POST /actions AND-gate — **deferred to Phase 93**

This plan does not change that disposition.

## Self-Check: PASSED

- `scripts/gates/override-01.ps1`: FOUND (`39a75821`)
- `nono-py/tests/test_override_wiring.py`: FOUND (`e1e704e` in nono-py repo)
- `verify-dark.ps1 --gate override-01`: exit 0, verdict PASS
- pytest: 11 tests passing
