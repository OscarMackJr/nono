---
phase: 93-live-zt-infra-integration-revocation-request-flow
verified: 2026-06-22T23:00:00Z
status: gaps_found
score: 6/8 must-haves verified
overrides_applied: 0
gaps:
  - truth: "verify_override_production() closes VFY-03a [BLOCKING-93]: pubkey is policy-sourced (HKLM) and per-key_id cached on the production Python call path"
    status: failed
    reason: "verify_override_production() was built (override.rs:987) and tested in Rust, but it was never exposed to Python and never called from any production Python path. All Python callers (_cli_apply.py, confined_run_checked, the gate SC1/SC2 inline script) call the backward-compat verify_override(token, pubkey_der, allowed_arns=[...]) with test-injected keys. The four forward-declared items (VK_CACHE, lookup_pubkey_der_from_policy, require_arn_allowlist_from_policy, verify_override_production) all carry #[allow(dead_code)] with comments stating the call site was planned for Phase 93 Plan 02 _live.py — that call site was never created."
    artifacts:
      - path: "C:/Users/OMack/nono-py/src/override.rs"
        issue: "verify_override_production is pub(crate) with #[allow(dead_code)]; not registered in lib.rs PyO3 module; not reachable from Python"
      - path: "C:/Users/OMack/nono-py/python/nono_py/_cli_apply.py"
        issue: "Calls verify_override(token_str) — the test-injection path with explicit pubkey_der/allowed_arns parameters — not verify_override_production. The HKLM trust root reader is bypassed entirely on the CLI apply path."
      - path: "C:/Users/OMack/nono-py/python/nono_py/__init__.py"
        issue: "confined_run_checked uses verify_override indirectly via _live.live_check (which takes an already-verified grant); the initial offline verification call in _cli_apply.py does not use the production HKLM reader."
    missing:
      - "Either expose verify_override_production as a PyO3 #[pyfunction] in lib.rs and update _cli_apply.py to call it (no pubkey_der/allowed_arns args), OR update _cli_apply.py to call read_override_trust_py() to source the keys and pass them to verify_override(). The VK_CACHE, lookup_pubkey_der_from_policy, require_arn_allowlist_from_policy dead_code annotations must be removed when the production call site is added."

  - truth: "Gate SC1/SC2 registry test-seam (HKCU) is reachable by the production HKLM trust reader"
    status: failed
    reason: "override_trust.rs opens HKEY_LOCAL_MACHINE exclusively (line 169: RegKey::predef(HKEY_LOCAL_MACHINE)). The override-02.ps1 gate seeds the test pubkey into HKCU:\\SOFTWARE\\Policies\\nono\\Override (RegistryBase = 'HKCU:\\SOFTWARE\\Policies\\nono\\Override', lines 63-65). On Windows, HKCU and HKLM are separate hives; HKCU values are NOT visible via an HKLM open. The gate's inline Python calls verify_override() (test-injection path) so the HKLM reader is not invoked during gate execution — but the gate comment claims it seeds into the 'policy-authoritative trust store (HKCU registry test-seam)' and the plan (93-06) claims 'HKCU is visible to the running process via the merged registry view'. That claim is FALSE for explicit HKEY_LOCAL_MACHINE opens; registry hive merging applies to HKCU into HKLM for HKCU-typed keys in certain per-user policy scenarios but NOT for standard RegKey::predef(HKEY_LOCAL_MACHINE) opens. Even if verify_override_production were called, it would NOT find the HKCU-seeded test key."
    artifacts:
      - path: "C:/Users/OMack/nono-py/src/override_trust.rs"
        issue: "Reads HKEY_LOCAL_MACHINE (line 169). No HKCU fallback. No merged-view read. If called with an HKCU-seeded key, returns Ok(OverrideTrust::empty()) (fail-closed) because the HKLM sub-key is absent."
      - path: "C:/Users/OMack/Nono/scripts/gates/override-02.ps1"
        issue: "Seeds test trust root into HKCU (line 63: RegistryBase = 'HKCU:\\SOFTWARE\\Policies\\nono\\Override'). This is unreachable from the HKLM reader. The gate does not actually exercise the HKLM reader — it bypasses it by using the test-injection verify_override() path."
    missing:
      - "If the gate is intended to test the production HKLM reader path: change RegistryBase to 'HKLM:\\SOFTWARE\\Policies\\nono\\Override' and run the gate elevated (requires admin). OR document that the gate deliberately uses the test-injection path and add a separate host-gated manual UAT item for the production HKLM reader."
      - "If the gate is only testing the two-key AND gate (live_check + verify_override) and not the HKLM reader: add a comment clarifying that SC1/SC2 use test-injected trust roots and that HKLM reader is tested separately by the Rust unit tests (phase93_vk_cache module)."
---

# Phase 93: Live ZT-Infra Integration + Revocation + Request Flow Verification Report

**Phase Goal:** The complete two-key AND gate is operational — a signed token is accepted only when both the KMS signature verifies offline AND a live ZT-Infra `POST /actions` lookup returns `allow`; revoked tokens are rejected on the next live check; AWS credentials never reach the sandboxed child environment; a developer can request and apply overrides via CLI.
**Verified:** 2026-06-22T23:00:00Z
**Status:** GAPS_FOUND
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | With `NONO_ZT_ACTIONS_URL` set, a live `POST /actions` call is made and a deny/timeout blocks the invocation (ZTL-01/ZTL-02) | VERIFIED | `_live.py:55-186` implements `live_check()` with 2s timeout (`urlopen(req, timeout=2.0)`); 403 → `LiveRevoked`; timeout/error/non-200/malformed → `LiveUnavailable`; both raise `NonoOverrideError`. 16 pytest tests in `test_live_arm.py` confirm all branches. |
| 2 | An override id on the ZT-Infra deny-list is rejected on the next live check (ZTL-03) | VERIFIED | `_live.py:160-165`: 403 HTTPError → `LiveRevoked`; provisioner `deny[]` rule → 403. Gate SC2 (override-02.ps1:415-490) seeds a deny rule and asserts `NonoOverrideError(LiveRevoked)`. No new revocation infra in nono. |
| 3 | After `confined_run()` with an override, the child process environment contains no `AWS_*` variables (ZTL-04) | VERIFIED | `env_sanitization.rs:23-26`: `|| key.starts_with("AWS_")` added cfg-unconditionally. 5 regression tests: `test_blocks_aws_access_key_id`, `test_blocks_aws_secret_access_key`, `test_blocks_aws_session_token`, `test_blocks_aws_region`, `test_blocks_aws_prefix_arbitrary_suffix`. `SystemRoot`/`windir` baseline untouched (cfg(windows) block, line 73). |
| 4 | `nono override request` surfaces denial context; `nono override apply <token-path>` runs full fail-closed verify before expansion (CLI-01, CLI-02) | VERIFIED | CLI-01: `override_request.rs:52-78` emits JSON bundle with scope/repo_context/reason/nonce; 5 unit tests. `nono override request` is registered in `cli.rs`. CLI-02: `_cli_apply.py:54-194` implements offline verify (`verify_override`) + live check (`_live.live_check`) + `confined_run_checked`; pyproject.toml `[project.scripts]` registers `nono-override-apply`. 12 pytest tests confirm offline-fail and live-fail both block exec. |
| 5 | DAAL anchoring is async/non-blocking (ZTL-05) | VERIFIED | `_live.py:85-95`: `flush_daal` key is intentionally absent from the POST body (comment: "Including it would risk blocking on DAAL drain (ZTL-05)"). Gate SC1 belt-and-suspenders check confirms `daal_flush == []` in the provisioner response. |
| 6 | `verify-dark.ps1 --gate override-02` emits `SKIP_HOST_UNAVAILABLE` when provisioner absent (DF-02) | VERIFIED | `override-02.ps1` exits 3 (SKIP_HOST_UNAVAILABLE) when `NONO_ZT_ACTIONS_URL` is not set. Confirmed by gate execution: `{"gate":"override-02","verdict":"SKIP_HOST_UNAVAILABLE","reason":"NONO_ZT_ACTIONS_URL not set...","timestamp":"2026-06-22T21:24:24.207Z"}` EXIT: 3. |
| 7 | VFY-03a [BLOCKING-93]: pubkey is policy-sourced (HKLM) and per-key_id cached on the production Python call path | FAILED | `verify_override_production()` built and tested Rust-side (override.rs:987) but NOT exposed to Python. All Python callers use test-injection `verify_override(token, pubkey_der, allowed_arns=[...])`. HKLM reader is bypassed on every Python execution path. See gaps section. |
| 8 | Gate registry test-seam (HKCU seed) is reachable by the production HKLM trust reader, proving the live allow path under policy-controlled trust | FAILED | Trust reader opens `HKEY_LOCAL_MACHINE` exclusively (override_trust.rs:169). Gate seeds `HKCU:\SOFTWARE\Policies\nono\Override` (override-02.ps1:63). These are separate hives; the HKLM reader will never find the HKCU-seeded key. Gate SC1/SC2 bypass this by using test-injection `verify_override()`. |

**Score:** 6/8 truths verified

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `nono-py/src/override_trust.rs` | Fail-secure HKLM reader (D-05/D-06) | VERIFIED | 279 lines; `windows_reader` mod with `HKEY_LOCAL_MACHINE`, `KEY_READ | KEY_WOW64_64KEY`, fail-closed taxonomy. Non-Windows stub returns `Ok(OverrideTrust::empty())`. |
| `nono-py/src/override.rs` | `LiveRevoked`/`LiveUnavailable` kinds + `VK_CACHE` + `verify_override_production` | PARTIAL | Kinds: VERIFIED (lines 88-122). VK_CACHE + lookup functions: EXIST but have `#[allow(dead_code)]` — forward-declared, no production Python call site. `verify_override_production`: ORPHANED (Rust-only, not Python-reachable). |
| `nono-py/python/nono_py/_live.py` | Fail-closed POST /actions AND-gate | VERIFIED | 187 lines; all decision branches implemented and tested. |
| `nono-py/python/nono_py/__init__.py` | `confined_run_checked`/`confine_checked` wrappers | VERIFIED | Lines 77-220; both wrappers with live pre-step; `__all__` updated. |
| `nono-py/python/nono_py/_cli_apply.py` | CLI-02 console-script | VERIFIED | 209 lines; full verify-then-run flow; 12 tests passing. |
| `nono-py/pyproject.toml` | `[project.scripts]` registration | VERIFIED | `nono-override-apply = "nono_py._cli_apply:main"` present. |
| `crates/nono-cli/src/exec_strategy/env_sanitization.rs` | `AWS_*` prefix clause | VERIFIED | `|| key.starts_with("AWS_")` at line 23; 5 regression tests. |
| `crates/nono-cli/src/override_audit_emit.rs` | Live-reject HMAC emission | VERIFIED | `emit_override_audit_event` wired to `SECURITY_LAYER.get().emit_override_event()`; kind→EventID 1:1; 5 unit tests. |
| `crates/nono-cli/src/override_request.rs` | CLI-01 denial bundle | VERIFIED | `run_override_request` emits JSON bundle with nonce; 5 unit tests. |
| `scripts/gates/override-02.ps1` | DF-02 gate | VERIFIED (with WARNING) | 685 lines; SKIP_HOST_UNAVAILABLE contract confirmed live (exit 3). SC1/SC2 logic implemented. WARNING: HKCU seam unreachable from HKLM reader (see Finding #2). |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `_live.py` | ZT-Infra provisioner | `urllib.request.urlopen(req, timeout=2.0)` | VERIFIED | Fail-closed; proxy-disabled; all branches tested with mocks. |
| `__init__.py:confined_run_checked` | `_live.live_check` | `_live.live_check(actions_url, override_token, ...)` | VERIFIED | Lines 145-150; wired before Rust `confined_run` call. |
| `_cli_apply.py` | `_live.live_check` | `_live.live_check(actions_url, grant, ...)` | VERIFIED | Lines 148-156; offline verify precedes live check; any failure blocks exec. |
| `override_audit_emit.rs` | `telemetry/mod.rs` | `SECURITY_LAYER.get().emit_override_event(...)` | VERIFIED | Lines 47-67; fail-closed on absent layer or poisoned mutex. |
| `env_sanitization.rs` | child env | `is_dangerous_env_var` → `should_skip_env_var` | VERIFIED | `AWS_*` clause unconditional; called from both Unix and Windows exec strategies. |
| `verify_override_production` | `override_trust::read_override_trust()` | `lookup_pubkey_der_from_policy` | NOT_WIRED | Function exists and is internally correct, but has no Python call site. `lib.rs` only registers `verify_override` (test-injection path). |
| Gate HKCU seed | `override_trust.rs` HKLM reader | Windows registry merged view | NOT_WIRED | `HKEY_LOCAL_MACHINE` open does not see `HKCU` values. |

---

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `_live.py:live_check` | `payload["audit"]["current_hash"]` | `urlopen(POST /actions)` response | Yes (mocked in tests; live with provisioner) | FLOWING |
| `__init__.py:confined_run_checked` | `live_hash` from `_live.live_check` | Threading live response hash to `zt_audit_hash_override` | Yes — wired to `confined_run(..., zt_audit_hash_override=live_hash)` | FLOWING (AUD-02 verified) |
| `_cli_apply.py:main` | `grant` from `verify_override(token_str)` | Uses test-injection path; pubkey_der/allowed_arns from external caller | Functional for offline tests; does NOT use HKLM trust reader | STATIC on production path (no HKLM read) |
| `override_trust.rs:read_override_trust` | `OverrideTrust { allowed_arns, kms_pubkeys }` | `HKEY_LOCAL_MACHINE` winreg read | Yes (when HKLM key present) | FLOWING — but NOT CALLED from any Python path |

---

## Behavioral Spot-Checks

| Behavior | Check | Result | Status |
|----------|-------|--------|--------|
| `AWS_*` strip | `grep 'starts_with.*AWS_' env_sanitization.rs` | Found at line 26 | PASS |
| `live_check` 2s timeout | `_live.py` line 157: `urlopen(req, timeout=timeout)` with default 2.0 | Present | PASS |
| `LiveRevoked` on 403 | `_live.py` lines 161-166: `if exc.code == 403: raise _override_error("LiveRevoked", ...)` | Present | PASS |
| `LiveUnavailable` on timeout/error | `_live.py` lines 172-179: `except (URLError, TimeoutError, socket.timeout, OSError)` | Present | PASS |
| `verify_override_production` reachable from Python | `grep 'verify_override_production' nono-py/src/lib.rs` | Not found | FAIL |
| Gate HKCU readable from HKLM reader | `grep 'HKEY_LOCAL_MACHINE' override_trust.rs` | Line 169; no HKCU fallback | FAIL |
| `flush_daal` absent from POST body | `_live.py` lines 89-95: key intentionally absent with comment | Present | PASS |
| `confined_run_checked` live pre-step not skippable | `__init__.py:127-132`: `if not actions_url: raise ValueError(...)` | Present | PASS |

---

## Probe Execution

No `scripts/tests/probe-*.sh` probes declared for this phase. Gate run:

| Gate | Command | Result | Status |
|------|---------|--------|--------|
| `scripts/gates/override-02.ps1` | `pwsh -File scripts/verify-dark.ps1 --gate override-02` | Exit 3, SKIP_HOST_UNAVAILABLE (provisioner absent on this host) | SKIP_HOST_UNAVAILABLE (expected per Dark Factory mandate) |

---

## Requirements Coverage

| Requirement | Plans | Description | Status | Evidence |
|-------------|-------|-------------|--------|---------|
| ZTL-01 | 93-01, 93-04 | Configurable live endpoint | PARTIAL | `NONO_ZT_ACTIONS_URL` read from env in `_live.py:127` and `_cli_apply.py:127`. Trust root config via HKLM reader exists (override_trust.rs) but not wired to Python. |
| ZTL-02 | 93-04 | 2s timeout, fail-closed | VERIFIED | `_live.py:157`: `urlopen(req, timeout=timeout)` with 2.0s default; all fail modes raise `NonoOverrideError`. |
| ZTL-03 | 93-04, 93-06 | Deny-list revocation honored | VERIFIED | 403 → `LiveRevoked`; gate SC2 proves deny rule rejects on next check. |
| ZTL-04 | 93-02 | AWS_* stripped from child env | VERIFIED | `env_sanitization.rs:26`; 5 regression tests. |
| ZTL-05 | 93-04, 93-06 | Async non-blocking DAAL | VERIFIED | `flush_daal` absent from POST body; gate asserts `daal_flush == []`. |
| CLI-01 | 93-03 | `nono override request` surfaces denial context | VERIFIED | `override_request.rs`; JSON bundle with scope/reason/nonce; 5 tests. |
| CLI-02 | 93-05 | `nono override apply` full fail-closed verify | VERIFIED | `_cli_apply.py`; offline+live gates before exec; 12 tests. |
| DF-02 | 93-06 | `verify-dark.ps1 --gate OVERRIDE-02` with SKIP | VERIFIED | Gate emits SKIP_HOST_UNAVAILABLE (exit 3) without provisioner. SC1/SC2 logic present. |
| VFY-01 clause (b) | 93-04 | Live AND gate closes the composition seam | PARTIAL | `confined_run_checked` enforces `_live.live_check` before Rust spawn (wired). However: `_cli_apply.py` performs a redundant second live check because `confined_run_checked` re-runs `_live.live_check` on a non-None override_token (double live check — behavioral warning, not a security regression). |
| VFY-03 clause (a) | 93-01 | Production pubkey policy-sourced (HKLM) | FAILED | `verify_override_production` built Rust-side but not exposed to Python; test-injection path used everywhere. |

---

## Specific Concerns Investigation

### Finding 1: HKCU vs HKLM Trust Store Mismatch

**Severity: WARNING** (gate correctness issue, not a production security regression)

`override_trust.rs:169` opens `HKEY_LOCAL_MACHINE` exclusively. The gate (`override-02.ps1:63`) seeds the test pubkey into `HKCU:\SOFTWARE\Policies\nono\Override`. On Windows, `RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(...)` does NOT see HKCU values — the merged registry view mentioned in the gate plan comment applies only to certain per-user policy paths (e.g., HKCU\\Software\\Policies which propagates to HKLM in certain GPO contexts), not to standard application paths under `SOFTWARE\Policies\nono`.

However, this mismatch does NOT affect gate correctness in practice because the gate's inline Python calls `verify_override(token_json, pubkey_der, allowed_arns=[TEST_KMS_ARN])` — the test-injection path that accepts explicit DER/ARN parameters — and does NOT call `verify_override_production()`. The HKLM reader is never invoked during gate execution. The gate proves the live `POST /actions` AND-gate behavior correctly; it simply does not prove the production HKLM trust-root sourcing.

**Impact:** The gate cannot validate that the HKLM reader would accept or reject a token based on seeded registry values, because the reader is not called. SC1 and SC2 succeed by bypassing the HKLM path entirely. This is **not** a production security regression (the HKLM reader is fail-closed: absent key → deny), but it means the gate does not provide the end-to-end integration proof it claims to provide for D-05/D-06.

**Suggested override-02.ps1 comment fix:** Replace "HKCU is visible to the running process via the merged registry view" with "gate uses test-injection verify_override() to bypass HKLM reader; HKLM reader is exercised by Rust unit tests (phase93_vk_cache module in override.rs)."

---

### Finding 2: AUD-02 Live-Hash Thread-Through

**Status: VERIFIED**

`_live.live_check` returns `payload.get("audit", {}).get("current_hash")` (line 147). `confined_run_checked` in `__init__.py` assigns this to `live_hash` (line 145) and threads it as `zt_audit_hash_override=live_hash` into `confined_run()` (line 161). `windows_confined_run.rs:427`: `append_override_args(&mut cmd, grant, zt_audit_hash_override.as_deref())?` — `append_override_args` prefers the override hash over the token hash (verified in SUMMARY). The end-to-end chain is wired.

**Note on double live check:** `_cli_apply.py` explicitly calls `_live.live_check` (lines 148-156), then calls `confined_run_checked(override_token=grant)` (line 182). `confined_run_checked` checks `if override_token is None:` — since `grant` is not None, it will call `_live.live_check` again internally. This results in two live POST /actions calls. This is a behavioral WARNING (redundant live call, wasteful) but not a security regression: both checks are fail-closed. The `live_hash` threaded into the Rust layer will be from the `confined_run_checked`'s second live call, not from the first explicit call in `_cli_apply.py` — so AUD-02 still threads a real live hash, though potentially from a slightly later point in time.

---

### Finding 3: Fail-Closed Totality (ZTL-02)

**Status: VERIFIED**

`_live.py` maps every failure mode:
- 403 HTTPError → `LiveRevoked` (deny, blocked) ✓
- Non-200/403 HTTPError → `LiveUnavailable` (infra-down, blocked) ✓
- `urllib.error.URLError` (DNS failure, connection refused) → `LiveUnavailable` ✓
- `TimeoutError` / `socket.timeout` (2s bound) → `LiveUnavailable` ✓
- `OSError` (lower-level connection failures) → `LiveUnavailable` ✓
- `json.JSONDecodeError` (malformed response) → `LiveUnavailable` ✓
- 200 but `decision != "allow"` → `LiveRevoked` (lines 137-142) ✓
- Missing `decision` key (e.g., `payload.get("decision") != "allow"`) → caught by the `!= "allow"` check ✓

`confined_run_checked` fails closed if `NONO_ZT_ACTIONS_URL` is not set (raises `ValueError`). `_cli_apply.py` fails closed if URL is absent (returns exit code 1). Both block exec.

Missing trust material (HKLM reader returning empty) → `verify_override_production` returns `Err(KeyNotAllowlisted)` — but this path is not invoked (see Finding 1/Gap 1). On the actual Python path used today, an absent HKLM key is not detected (test-injected keys are used instead), so fail-closed totality on the HKLM reader path is Rust-verified but Python-unreachable.

---

## Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `nono-py/src/override.rs` | `#[allow(dead_code)]` on `VK_CACHE`, `lookup_pubkey_der_from_policy`, `require_arn_allowlist_from_policy`, `verify_override_production` | WARNING | Forward-declarations with planned call sites in `_live.py` that were never created. Dead code with suppressed lint = unresolvable gap without manual inspection. |
| `nono-py/python/nono_py/_cli_apply.py` | Double live check: explicit `_live.live_check` + `confined_run_checked` both call the provisioner | WARNING | Two `POST /actions` calls per `nono-override-apply` invocation. Wasteful; could trigger rate limiting; minor timing window between the two checks (not a security issue). |
| `Nono/.planning/REQUIREMENTS.md` | ZTL-01..05, CLI-01/02, DF-02 all still show status "Pending" after Phase 93 execution | INFO | Documentation drift; requirements were not updated to "Complete" after the phase ran. |

No `TBD`, `FIXME`, or `XXX` debt markers found in phase-modified files.

---

## Human Verification Required

### 1. Production HKLM Trust Root Read (ZTL-01 / VFY-03a)

**Test:** On an elevated Windows host, seed `HKLM\SOFTWARE\Policies\nono\Override\KmsPublicKeys\<test_arn>` and `\AllowedKeyArns\` with the Phase 91 test keypair DER/ARN. Run `nono-override-apply <token-path> -- echo verified` (after exposing `verify_override_production` to Python or updating the call path).
**Expected:** Override succeeds when the key is in HKLM; fails with `KeyNotAllowlisted` when absent.
**Why human:** Requires admin elevation, HKLM write, production HKLM read path currently not wired in Python.

### 2. Live provisioner end-to-end with HKLM-sourced trust

**Test:** With local provisioner running (`NONO_ZT_ACTIONS_URL=http://127.0.0.1:3000/actions`), use a real-KMS-signed token against the production `verify_override_production` path (once wired).
**Expected:** Both the offline HKLM-sourced verify and the live `/actions` call pass; `confined_run_checked` spawns the command with the live audit hash threaded.
**Why human:** Requires provisioner running locally and the HKLM trust reader call site to be wired in Python.

### 3. Double live check regression in _cli_apply.py

**Test:** Run `nono-override-apply` with `NONO_ZT_ACTIONS_URL` set and observe HTTP traffic.
**Expected:** Confirm whether one or two `POST /actions` calls are made per invocation. If two, assess whether this is intentional or a regression.
**Why human:** Requires network traffic inspection (e.g., provisioner access log).

---

## Gaps Summary

**2 gaps blocking full goal achievement:**

**Gap 1 (BLOCKER — VFY-03a [BLOCKING-93] not fully closed):** `verify_override_production()` was built in Rust (with `VK_CACHE`, HKLM trust reader, per-key_id cache) but was never exposed to Python and never called from any production Python path. Every Python caller (`_cli_apply.py`, `confined_run_checked`, gate inline script) uses the backward-compat `verify_override(token, pubkey_der, allowed_arns=[...])` with test-injected keys. The HKLM trust reader is exercised only by Rust unit tests. The original phase objective "close the test-injected-pubkey seam via the policy-authoritative HKLM trust reader" is achieved only at the Rust function level, not at the Python integration level where it matters operationally. The `#[allow(dead_code)]` annotations on four forward-declared items confirm the production call sites were planned but not implemented.

**Gap 2 (WARNING — gate test-seam misalignment):** The gate seeds the test pubkey into HKCU but the trust reader reads HKLM exclusively. The gate does not exercise the HKLM reader; it bypasses it by using the test-injection path. The claim in the plan that "HKCU is visible via the merged registry view" is incorrect for explicit HKEY_LOCAL_MACHINE opens. This is a documentation/correctness issue in the gate, not a production regression (HKLM reader is fail-closed), but it means the gate does not provide the end-to-end trust-root-to-live-check proof it claims to provide.

**Root cause:** Both gaps share a single root cause: the planned production call site for `verify_override_production` in `_live.py` (or `_cli_apply.py`) was never implemented. The forward-declaration comments in `override.rs` explicitly name "Phase 93 Plan 02 _live.py" as the construction site — but Plan 04 (which implemented `_live.py`) and Plan 05 (which implemented `_cli_apply.py`) both used the test-injection `verify_override()` path instead. The plans' `must_haves` did not explicitly require calling `verify_override_production` (they required "Rust offline verify_override"), allowing the implementation to satisfy the letter of the plan criteria without closing the HKLM seam.

---

## Deferred Items

None — all gaps are actionable and addressable within this phase scope.

---

## Cross-Target Clippy Notes (Partial→CI)

Per CLAUDE.md MUST/NEVER rule: several files modified this phase contain cfg-gated Unix/Windows code:
- `nono-py/src/override_trust.rs` — `#[cfg(target_os = "windows")]` / `#[cfg(not(target_os = "windows"))]` guards. Non-Windows stub compiled on Windows host; Linux/macOS path not exercised on Windows host.
- `crates/nono-cli/src/exec_strategy/env_sanitization.rs` — `cfg!(target_os = "windows")` block at line 73.
- `crates/nono-cli/src/override_audit_emit.rs` — no cfg gating.

**Status: PARTIAL→CI.** Native Windows clippy (`cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`) passed (per plan summaries 93-01, 93-02, 93-03). Cross-target Linux/macOS verify requires cross-toolchain not present on this host. CI (`ubuntu-latest`) provides the authoritative cross-target check.

---

_Verified: 2026-06-22T23:00:00Z_
_Verifier: Claude (gsd-verifier)_

---

## ADDENDUM — Gaps Closed (2026-06-22, post-verification)

The verifier's `gaps_found` findings above were RESOLVED in the gap-closure pass. Original findings retained verbatim for the record. See `93-GAP-CLOSURE-SUMMARY.md`.

**Revised verdict: PASS-WITH-PARTIALS** (8/8 must-haves; host-gated + cross-target items deferred as designed).

- **Gap 1 (BLOCKER, VFY-03a) — CLOSED.** `verify_override_production` exposed as a `#[pyfunction]` (`nono-py/src/lib.rs:773`, `#[allow(dead_code)]` removed) and wired into the production `_cli_apply.py:140` apply path (HKLM-sourced trust, no test-injected pubkey). No HKCU/env trust fallback (D-05 preserved). Commit `358d9c6` (nono-py).
- **Gap 2 (WARNING, gate trust path) — CLOSED.** `override-02.ps1` retargeted HKCU→`HKLM\SOFTWARE\Policies\nono\Override` + elevation precondition (non-admin → SKIP_HOST_UNAVAILABLE); reader unchanged (HKLM-only). Re-run → exit 3 clean SKIP on this host. Commit `38121c69` (Nono).
- **Gap 3 (WARNING, double live check) — CLOSED.** `_cli_apply.py` performs exactly one `POST /actions`; plain `confined_run()` with AUD-02 live-hash thread-through. Commit `358d9c6` (nono-py).

**Re-verification evidence:** 31/31 nono-py override+live pytest pass; 66/66 Rust override tests pass; `cargo build`/`clippy` clean (nono-py + nono-cli); OVERRIDE-02 gate SKIPs cleanly. The 4 `sandboxed_exec` collection errors are the pre-existing Windows baseline (Unix-only entry; out of scope), not a regression.
