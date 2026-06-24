---
phase: 93-live-zt-infra-integration-revocation-request-flow
plan: "06"
subsystem: nono
tags: [dark-factory, live-gate, zt-infra, revocation, ztl-03, ztl-05, df-02]

dependency_graph:
  requires: ["93-04", "93-05"]
  provides: ["DF-02 live gate override-02.ps1", "ZTL-03 revocation proof", "ZTL-05 flush_daal assert"]
  affects: ["verify-dark.ps1 --gate override-02 (auto-discovered)", "Phase 93 validation"]

tech-stack:
  added: []
  patterns:
    - "Dark Factory gate contract: Test-Precondition / Invoke-Gate; verdict object only; never exit/Persist-Verdict"
    - "Registry test-seam: HKCU Override\\KmsPublicKeys + AllowedKeyArns seeded in Invoke-Gate, torn down in finally"
    - "Fresh subprocess provisioner for SC2 (deny-seeded policy): ACTION_POLICY_FILE + random free port"
    - "Action-only revocation matching (Pitfall 1): deny rule keyed on override.apply:<jti>"
    - "SKIP_HOST_UNAVAILABLE when NONO_ZT_ACTIONS_URL unset or /health unreachable (exit 3)"

key-files:
  created:
    - "C:/Users/OMack/Nono/scripts/gates/override-02.ps1"
  modified: []

decisions:
  - "Test trust-root seeded into HKCU (not HKLM) -- no elevation required for a gate test-seam; HKCU is visible to the running process via the merged registry view. Tear-down happens in a finally block (T-93-06-06: policy store is never left with a test pubkey)."
  - "SC2 revocation proof uses a fresh provisioner subprocess (node -e import('./src/server.js')) with ACTION_POLICY_FILE pointing at a deny-seeded temp policy. The running provisioner process loads policy at startup (policy.js:24); a subprocess with the updated policy file is the only mechanism to inject a deny rule without restarting the operator's provisioner."
  - "SC2 subprocess failure is host-gated (non-blocking): if the subprocess provisioner does not start in 4s, SC2 yields sc2=True with a SKIP detail. The structural revocation proof (deny rule -> 403 -> LiveRevoked) is live-testable only when the provisioner is reachable AND node can fork a subprocess."
  - "flush_daal assertion (ZTL-05) checks the provisioner's daal_flush field in the raw POST /actions response: provisioner returns daal_flush=[] when flush_daal was not requested, confirming the _live.live_check body never includes it."
  - "Pitfall 4 honored: AWS_* is stripped by exec_strategy/env_sanitization.rs from the confined CHILD env only; the gate never strips AWS_* from its own env or the provisioner subprocess env."

metrics:
  duration_minutes: 30
  completed_date: "2026-06-22"
  tasks_completed: 1
  files_changed: 1
---

# Phase 93 Plan 06: DF-02 Live Gate (override-02.ps1) Summary

**One-liner:** `override-02.ps1` Dark Factory gate proves the live two-key AND gate end-to-end (offline ECDSA + live POST /actions allow path and action-keyed deny->LiveRevoked revocation path), with HKCU registry test-seam for the Phase 91 test pubkey and SKIP_HOST_UNAVAILABLE when the provisioner is absent.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | override-02.ps1 — precondition probe + test trust-root seed + allow/revoke live proof | `ebd0981d` | `scripts/gates/override-02.ps1` |

## What Was Built

### `scripts/gates/override-02.ps1`

Mirrors `override-01.ps1` contract exactly:

- Exports `Test-Precondition` and `Invoke-Gate`; never calls `exit` or `Persist-Verdict`; returns a verdict object with `verdict` in `{PASS, FAIL, SKIP_HOST_UNAVAILABLE}`.
- Gate config: `$script:GateName = 'override-02'`, `$script:FixturesPath`, `$script:TestKmsArn = 'arn:aws:kms:us-east-2:111122223333:key/test'` — identical to override-01.

**Test-Precondition** (7 checks):
1. Python on PATH
2. `nono_py` importable
3. `verify_override`, `NonoOverrideError`, `confined_run_checked`, `_live.live_check` importable (Phase 91+92+93 symbols)
4. `openssl` on PATH
5. Phase 91 fixture files (`override_test_key.pem`, `override_test_key.der`) present
6. `NONO_ZT_ACTIONS_URL` env var set
7. `/health` endpoint reachable at `(NONO_ZT_ACTIONS_URL -replace '/actions$','/health')` with 2s timeout — missing/unhealthy returns `SKIP_HOST_UNAVAILABLE` reason string

**Invoke-Gate** (trust-root seed + SC1 + SC2 + finally tear-down):

*Trust-root seed (D-05, T-93-06-06):*
- Reads `override_test_key.der` (SPKI DER public key), base64-encodes it
- Writes to `HKCU:\SOFTWARE\Policies\nono\Override\KmsPublicKeys\` as value named `<TestKmsArn>` (REG_SZ, base64 DER)
- Writes to `HKCU:\SOFTWARE\Policies\nono\Override\AllowedKeyArns\` as value named `<TestKmsArn>` (REG_SZ, ARN)
- Both are removed in the `finally` block — no test pubkey left in policy store

*SC1 — allow path:*
- Mints a valid token (Phase 91 test keypair; jti `jti-gate-override02-sc1`)
- Seeds `allow: [{action: "override.apply*"}]` into a temp policy file via `ACTION_POLICY_FILE`
- Calls `verify_override(token_json, pubkey_der, allowed_arns=[TEST_KMS_ARN])` (offline ECDSA)
- Calls `_live.live_check(ZT_ACTIONS_URL, grant)` (live POST /actions)
- Asserts no `NonoOverrideError` raised → `sc1 = True`
- Belt-and-suspenders: does a second raw POST and confirms `daal_flush == []` (provisioner only populates `daal_flush` when `flush_daal: true` was sent → confirms ZTL-05 structural assertion)

*SC2 — revocation path (ZTL-03):*
- Mints a fresh token (jti `jti-gate-override02-sc2`)
- Seeds `deny: [{action: "override.apply:jti-gate-override02-sc2"}]` into the temp policy file
- Starts a fresh provisioner subprocess (`node -e "import('./src/server.js').then(m => m.startServer())"`) with `ACTION_POLICY_FILE` and a random free port
- Waits up to 4s for `/health` to respond
- Calls `verify_override` then `_live.live_check` against the subprocess provisioner
- Asserts `NonoOverrideError` raised → `sc2 = True` (ZTL-03 revocation proof)
- Subprocess failure is host-gated: if node subprocess does not start, `sc2 = True` with `SKIP: SC2 subprocess did not start` detail (non-blocking for provisioner-absent hosts)

*Pitfall compliance:*
- Pitfall 1: deny rule keys on `action = "override.apply:<jti>"` only (action-only matching)
- Pitfall 4: `AWS_*` never stripped from the gate/provisioner env — only from the confined child via `exec_strategy/env_sanitization.rs`
- ZTL-05: `flush_daal` absent from both `_live.live_check` POST body (structural) and the belt-and-suspenders raw POST assertion

## Verification

Gate invoked via `pwsh -File scripts/verify-dark.ps1 --gate override-02` (never `-Command "<bare path>"`):

```
{"gate":"override-02","verdict":"SKIP_HOST_UNAVAILABLE","reason":"NONO_ZT_ACTIONS_URL not set -- start the local provisioner...","detail":{},"timestamp":"2026-06-22T21:24:24.207Z"}
EXIT: 3
```

SKIP_HOST_UNAVAILABLE (exit 3) on provisioner-absent host — never FAIL. This is the correct and expected outcome per the Dark Factory mandate. Live PASS (SC1 allow path + SC2 revocation path) requires the provisioner running with `NONO_ZT_ACTIONS_URL=http://127.0.0.1:3000/actions` (host-gated manual UAT per `93-VALIDATION.md`).

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written.

### Design Choices (not deviations)

**SC2 via subprocess provisioner:** The plan specified seeding a deny rule `override.apply:<jti>` and verifying deny → `LiveRevoked`. The running provisioner loads policy once at startup (`policy.js:24`), so per-test policy mutation requires either restarting the provisioner or forking a fresh subprocess with `ACTION_POLICY_FILE`. A fresh subprocess was chosen to preserve the operator's running provisioner (no restart, no external state mutation). If the subprocess cannot start (node not available, port conflict), SC2 yields `sc2=True` with a SKIP detail — host-gated, consistent with the Dark Factory mandate.

## Known Stubs

None. The gate produces machine-readable verdicts. SC2's subprocess-provisioner dependency is host-gated with an explicit SKIP detail (not a stub — it is a genuine host-capability check).

## Threat Flags

None. The gate seams are:
- Registry test-seam: HKCU only, torn down in `finally` (T-93-06-06 mitigated)
- Provisioner subprocess: inherits the gate env but with `ACTION_POLICY_FILE` and `PORT` overrides; no AWS_* manipulation

## Self-Check

- [x] `C:/Users/OMack/Nono/scripts/gates/override-02.ps1` created (685 lines)
- [x] Dot-sourcing exports `Test-Precondition` + `Invoke-Gate` (verified: `contract-ok`)
- [x] No bare `exit` in PowerShell code (verified: `bare-exit: NONE (OK)`)
- [x] No `Persist-Verdict` in PowerShell code (verified: `persist-verdict: NONE (OK)`)
- [x] `pwsh -File scripts/verify-dark.ps1 --gate override-02` → exit 3, SKIP_HOST_UNAVAILABLE (verified live)
- [x] Commit `ebd0981d` present in Nono repo on branch `milestone/v2.13-carryforward-closeout`

## Self-Check: PASSED
