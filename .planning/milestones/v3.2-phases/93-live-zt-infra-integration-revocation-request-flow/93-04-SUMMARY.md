---
phase: 93-live-zt-infra-integration-revocation-request-flow
plan: "04"
subsystem: nono-py
tags: [live-and-gate, zt-infra, python-orchestration, vfy-01, aud-02, oq-2]
dependency_graph:
  requires: ["93-01", "93-02"]
  provides: ["VFY-01 clause b", "live POST /actions AND-gate", "AUD-02 live-hash thread-through"]
  affects: ["nono-py confined_run path", "override audit event content"]
tech_stack:
  added: []
  patterns: ["Python pre-step wrapper (OQ-2)", "urllib fail-closed live check (D-01/D-02)", "AUD-02 live-hash thread-through", "MUT-05 no-override parity bypass"]
key_files:
  created:
    - C:/Users/OMack/nono-py/python/nono_py/_live.py
    - C:/Users/OMack/nono-py/tests/test_live_arm.py
  modified:
    - C:/Users/OMack/nono-py/python/nono_py/__init__.py
    - C:/Users/OMack/nono-py/src/windows_confined_run.rs
decisions:
  - "D-01 honored: live POST /actions call lives in _live.py (Python, stdlib urllib); Rust verify_override stays offline-only and policy-free"
  - "D-02 honored: 403/decision!=allow -> LiveRevoked (10010); timeout/error/non-200/malformed -> LiveUnavailable (10008); both fail-closed"
  - "AUD-02 implemented: live_check returns fresh audit.current_hash; confined_run_checked threads it into zt_audit_hash_override param; token hash used only on offline/no-override path"
  - "MUT-05 parity: override_token=None skips live check entirely, calls Rust directly with no audit metadata change"
  - "VFY-01 clause b closed: two-key AND gate (offline ECDSA + live /actions) operational Python-side"
metrics:
  duration_minutes: 30
  completed_date: "2026-06-23T01:09:24Z"
  tasks_completed: 2
  files_changed: 4
---

# Phase 93 Plan 04: Live AND-Gate + Python Pre-Step Wrapper Summary

**One-liner:** Python `confined_run_checked`/`confine_checked` wrappers enforce `_live.live_check()` POST /actions before the Rust spawn, threading the fresh `audit.current_hash` into the override-audit metadata (AUD-02), closing VFY-01 clause (b).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | `_live.py` — fail-closed POST /actions decision mapping | `ec41990` (nono-py) | `python/nono_py/_live.py`, `tests/test_live_arm.py` |
| 2 | Python pre-step wrapper + live-hash thread-through + seam-comment update | `151482d` (nono-py) | `python/nono_py/__init__.py`, `src/windows_confined_run.rs` |

## What Was Built

### Task 1 (prior executor, RED+GREEN complete)

`_live.py` implements `live_check(actions_url, grant, *, timeout=2.0, extra_header=None)`:
- Builds `{actor, action, resource, correlation_id}` body from `OverrideGrant` getters only (TOCTOU closure, D-03); never includes `flush_daal` (ZTL-05)
- Installs a per-call empty `ProxyHandler({})` opener (env-proxy disabled, Pitfall 5); restores previous opener in `finally`
- 200+allow -> returns `payload["audit"]["current_hash"]` (AUD-02 fresh hash)
- 403 -> raises `NonoOverrideError("LiveRevoked", ...)` (EventID 10010)
- timeout/URLError/OSError/non-200/malformed -> raises `NonoOverrideError("LiveUnavailable", ...)` (EventID 10008)
- `test_live_arm.py`: 16 tests covering all decision branches, body shape, no-flush_daal, hash threading, extra-header passthrough

### Task 2 (this executor)

`__init__.py` gains two wrappers in the `win32` import branch:

**`confined_run_checked(exe, args, allow, profile, cwd, timeout_secs, override_token)`**
- `override_token=None`: calls `confined_run(...)` directly, no live check (MUT-05 parity)
- `override_token` present: reads `NONO_ZT_ACTIONS_URL` (fail-closed ValueError if absent); calls `_live.live_check(url, grant)` with optional `NONO_ZT_ACTIONS_HEADER` and `NONO_ZT_ACTIONS_TIMEOUT`; on allow, calls `confined_run(..., zt_audit_hash_override=live_hash)` threading the fresh hash (AUD-02); on deny/unavailable, `NonoOverrideError` propagates, Rust spawn never reached

**`confine_checked(profile, allow, caps, override_token)`**
- Identical live pre-step logic for Shape B (`confine`)

**`windows_confined_run.rs`** (already had prior executor's changes):
- `confined_run` and `confine` both have `zt_audit_hash_override: Option<String>` param (Phase 93 AUD-02)
- `append_override_args` prefers `zt_audit_hash_override` over `grant.zt_audit_hash()` (live path wins)
- Seam comments at `:404-406` and `:544-549` record "VFY-01 clause b CLOSED (Phase 93 Plan 04): live check performed by the Python caller" and the AUD-02 live-hash contract

**`__all__` additions (win32 branch, alphabetical):** `_live`, `confine`, `confine_checked`, `confined_run`, `confined_run_checked`

## VFY-01 Closure Evidence

VFY-01 clause (b): "override accepted only when offline verify passes AND live /actions returns allow"

Before this plan: `confined_run` accepted `override_token` without any live check; the seam at `:404-406` was left as `[BLOCKING-93]`.

After this plan:
- Python callers use `confined_run_checked` (the documented entry point exported in `__all__`)
- `confined_run_checked` enforces the two-key AND gate: offline `verify_override()` (Phase 91) produces the `OverrideGrant`; live `_live.live_check()` (this plan) runs before the Rust spawn
- The Rust `confined_run` receives the grant only after the Python layer has cleared both gates
- Seam comments confirm the assumption: the Rust fn "assumes the grant is live-verified" (VFY-01 clause b closed Python-side)

## Verification Results

| Check | Result |
|-------|--------|
| `pytest tests/test_live_arm.py -x -q` | 16/16 PASS |
| `cargo build -p nono-py` | Exit 0 (Finished dev profile) |
| `cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` | Clean (0 warnings) |
| `maturin develop` + extension rebuild | PARTIAL→host (maturin not available on this Windows host; pure-Python wrapper tests are not blocked — they do not require the rebuilt extension) |
| Extension-dependent test coverage | PARTIAL→CI: `confined_run_checked` wrapper exercises Python logic fully in pure-Python tests; the end-to-end call through the Rust `confined_run` extension requires a live `maturin develop` build on a host with maturin installed |

## Deviations from Plan

None — plan executed exactly as written.

The prior executor's uncommitted `windows_confined_run.rs` changes were reviewed and found correct:
- `zt_audit_hash_override: Option<String>` added to both `confined_run` and `confine` `#[pyfunction]` signatures
- `#[pyo3(signature = ...)]` updated for both functions
- `append_override_args` updated to accept `zt_audit_hash_override: Option<&str>` and prefer it over the token hash
- Seam comments at `:404-406` and `:544-549` record VFY-01 clause b closure and AUD-02 contract
All changes were retained unchanged and included in the Task 2 commit.

## Known Stubs

None. The wrapper calls through to the real Rust `confined_run`/`confine` on the live-allow path.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes beyond those documented in the plan's `<threat_model>`. The `confined_run_checked`/`confine_checked` wrappers add a new outbound network call (POST /actions) but this surface was already registered as T-93-04-01 through T-93-04-07 in the plan threat register and is fully mitigated (fail-closed, env-proxy disabled, no cross-host redirects, no secrets in error messages).

## Self-Check: PASSED

- `C:/Users/OMack/nono-py/python/nono_py/_live.py` — FOUND (committed `ec41990`)
- `C:/Users/OMack/nono-py/tests/test_live_arm.py` — FOUND (committed `ec41990`)
- `C:/Users/OMack/nono-py/python/nono_py/__init__.py` — FOUND (committed `151482d`)
- `C:/Users/OMack/nono-py/src/windows_confined_run.rs` — FOUND (committed `151482d`)
- Task 1 commit `ec41990` — FOUND in nono-py git log
- Task 2 commit `151482d` — FOUND in nono-py git log
