# Phase 93 — Gap-Closure Summary

**Date:** 2026-06-22
**Trigger:** `93-VERIFICATION.md` verdict `GAPS_FOUND` (6/8 must-haves) — 1 BLOCKER + 2 WARNINGs.
**Outcome:** All 3 gaps closed and re-verified. Phase goal achieved (PASS-WITH-PARTIALS — host-gated + cross-target items deferred as designed).

This closure spans both repos:
- **nono-py** (`C:\Users\OMack\nono-py`, branch `44-broker-ffi-lockstep`): `358d9c6 fix(93-gap): expose+wire verify_override_production (VFY-03a) + single live check`
- **Nono** (`C:\Users\OMack\Nono`, branch `milestone/v2.13-carryforward-closeout`): `38121c69 fix(93-gap): seed HKLM trust root + elevation SKIP in override-02 gate` (+ this summary)

---

## Gap 1 — BLOCKER: VFY-03a `[BLOCKING-93]` not actually closed (CLOSED)

**Finding:** `verify_override_production()` (HKLM trust reader + per-`key_id` `VK_CACHE`, built in Plan 01) was `pub(crate)` + `#[allow(dead_code)]` — never exposed to Python, never called. Every Python production path used the test-injection `verify_override(token, pubkey_der, allowed_arns=[...])`, bypassing the HKLM trust root. So the production trust integration that this final phase exists to deliver was inert.

**Fix:**
- `nono-py/src/lib.rs:773` — `verify_override_production` registered as a `#[pyfunction]` (no `pubkey_der`/`allowed_arns` params; sources the KMS pubkey DER + ARN allowlist from `HKLM\SOFTWARE\Policies\nono\Override` per D-05/D-06). `#[allow(dead_code)]` removed.
- `nono-py/python/nono_py/_cli_apply.py:140` — the `nono-override-apply` production console entry now calls `verify_override_production(token_str)`; the test-injection `verify_override(...)` remains available for tests/gate token-minting only.
- Fail-closed preserved: missing HKLM trust material → `KeyNotAllowlisted` deny. **No HKCU/env trust fallback** was added (would let a non-admin/tampered env install a rogue pubkey — a D-05 security regression).

**Verification:** `verify_override_production` importable + wired in `_cli_apply.py`; 31/31 Python override+live tests pass; 66/66 Rust override tests pass; `cargo build`/`clippy` clean.

## Gap 2 — WARNING: gate did not exercise the HKLM production trust path (CLOSED)

**Finding:** `scripts/gates/override-02.ps1` seeded the TEST pubkey into `HKCU\...\Override`, but `override_trust.rs` opens `HKEY_LOCAL_MACHINE` exclusively. The gate's allow path therefore never touched the production reader (it used test-injection).

**Fix (`scripts/gates/override-02.ps1`):**
- `$script:RegistryBase = 'HKLM:\SOFTWARE\Policies\nono\Override'` — seeds into HKLM, the production trust store the HKLM-only reader opens, genuinely exercising `verify_override_production` (Gap 1).
- `Test-Precondition` gains an elevation check (`WindowsPrincipal … IsInRole(Administrator)`): HKLM writes require admin, so a non-admin run yields `SKIP_HOST_UNAVAILABLE` (exit 3), never FAIL.
- Seeded HKLM values torn down in the `finally` block (T-93-06-06).
- Reader unchanged (HKLM-only, no HKCU fallback). Stale "HKCU" doc comments corrected to HKLM.

**Verification:** `pwsh -File scripts/verify-dark.ps1 --gate override-02` → exit 3 `SKIP_HOST_UNAVAILABLE` (clean SKIP; this host lacks openssl/provisioner/elevation — host-gated, as designed). Script parses and runs; precondition chain behaves correctly.

## Gap 3 — WARNING: double live check in `_cli_apply.py` (CLOSED)

**Finding:** `_cli_apply.py` called `_live.live_check(...)` explicitly **and** `confined_run_checked(override_token=grant)` (which calls `_live.live_check` again) → two `POST /actions` per `nono-override-apply` invocation.

**Fix:** `_cli_apply.py` now calls `_live.live_check()` **exactly once**, then the plain Rust `confined_run()` (not the `_checked` wrapper), threading the single live response's fresh `audit.current_hash` into `zt_audit_hash_override` (AUD-02). Ordering preserved: offline `verify_override_production` → single live check → confined exec. Documented inline at `_cli_apply.py:22-24`.

**Verification:** test_override_apply.py asserts a single live call; 31/31 pass.

---

## Residual / deferred (unchanged disposition)

- **Host-gated:** OVERRIDE-02 live allow/revoke proof requires the local ZT-Infra provisioner + openssl + an elevated session — `SKIP_HOST_UNAVAILABLE` on this host, consistent with the Dark Factory mandate.
- **Cross-target clippy (PARTIAL→CI):** Unix-cfg-gated `AWS_*` strip (`exec_strategy/env_sanitization.rs`) verified native-Windows; linux-gnu/apple-darwin clippy deferred to CI (host lacks cross C compiler) per CLAUDE.md.
- **Pre-existing (not a regression):** `tests/test_smoke.py` & 3 siblings fail to collect on Windows (`sandboxed_exec` is the Unix-only entry; Unix override parity is explicitly out of scope). Established Windows baseline.

## Self-Check: PASSED
