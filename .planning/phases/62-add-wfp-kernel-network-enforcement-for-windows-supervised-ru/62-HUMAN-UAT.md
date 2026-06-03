# Phase 62 — HUMAN-UAT (62-04)

**Requirement gated:** REQ-WFP-01 — out-of-box WFP kernel network enforcement for Windows supervised runs.
**Phase gate:** All 5 success criteria (SC1–SC5) must show **PASS** in this file before `/gsd:verify-work`.

## Build under test

| Field | Value |
|-------|-------|
| Version | **v0.57.12** (62-13 AppContainer profile-registration spawn fix) |
| Machine MSI | `dist/windows/nono-v0.57.12-x86_64-pc-windows-msvc-machine.msi` (ProductVersion 0.57.12, MajorUpgrade) |
| Signing | Authenticode Valid; POC cert `319E507E…` (imported on host) |
| `nono.exe` SHA256 | `C9F900CFCB6AAC5A288233BF84DB23D4CBAC3AC8D6BCD330070C227E8A59A318` |
| `nono-wfp-service.exe` SHA256 | `17A24FB19EB942F1EF55732DC0EBA2FA6CC05BBD590D0C1B502AC2AE37CAF25B` |
| Host | Windows 11 build 26200 (live, non-elevated for SC1/SC5; elevated for setup/SC3/SC4) |
| Date | 2026-06-03 |

---

## Results summary

| SC | Scenario | Status |
|----|----------|--------|
| SC1 | Out-of-box enforced block (non-elevated, no prior `--start-wfp-service`) | ✅ **PASS** |
| SC2 | Boot-start survives reboot (service auto-running; SC1 repeatable) | ⏳ PENDING |
| SC3 | Fail-closed with remediation when service stopped (dev layout) | ⏳ PENDING |
| SC4 | Clean uninstall leaves nothing behind | ✅ **PASS** |
| SC5 | Confined child cannot reach WFP control pipe (only nono.exe does) | ⏳ PENDING (expected PASS — see SC1) |

> Phase NOT complete until SC2, SC3, SC5 are PASS. If any SC is FAIL, open a gap-closure plan instead of marking the phase complete.

---

## SC1 — Out-of-box enforced block ✅ PASS

**Scenario:** After installing the machine MSI and WITHOUT a prior `nono setup --start-wfp-service`, a non-elevated supervised `nono run` with `network.block:true` denies the confined child's outbound network.

**Command (cwd `%USERPROFILE%\.claude`):**
```
nono run --profile claude-code --block-net --allow-cwd -- curl.exe -sS -m 5 https://api.ipify.org
```

**Observed (PASS):**
```
broker: AppContainer profile registered ...
broker: spawned child app_container=true
curl: (6) Could not resolve host: api.ipify.org
child_exit 6
```

**Verdict:** The confined child **STARTED** (no `0xC0000142` / `ERROR_FILE_NOT_FOUND`) and its outbound connection was **WFP-kernel-BLOCKED** — with no prior manual service-start step. The block manifests as DNS/connect failure (exit 6), confirming kernel filtering of the per-run AppContainer's package SID via the existing `ALE_USER_ID` SD path. Resolved the `WRITE_RESTRICTED → AppContainer` chain (debug `wfp-write-restricted-0142` → resolved).

**Bypass-traverse note:** The profile-deep cwd worked, so the lowbox retains `SeChangeNotifyPrivilege`; only the leaf `.claude` traverse grant (`c3d7644f`) was needed. The 62-13 Task-3 ancestor grant was unnecessary (harmless; removable in cleanup).

---

## SC2 — Boot-start survives reboot ⏳ PENDING

**Scenario:** After rebooting, `nono-wfp-service` is already running (boot-started by SCM), and the SC1 scenario succeeds without any manual step.

**Steps:**
```powershell
# Step 1 — full reboot
shutdown /r /t 0

# Step 2 (after reboot, NON-ELEVATED)
sc query nono-wfp-service          # Expected STATE: 4 RUNNING
sc qc nono-wfp-service             # Expected START_TYPE: 2 AUTO_START

# Step 3 (NON-ELEVATED, cwd %USERPROFILE%\.claude) — repeat SC1, no manual start
nono run --profile claude-code --block-net --allow-cwd -- curl.exe -sS -m 5 https://api.ipify.org
```

**Pass criteria:** service `RUNNING` before any manual start, START_TYPE `2 AUTO_START`, and block enforced (curl exit 6 / no external IP).

- [ ] `sc query` STATE post-reboot: __________
- [ ] `sc qc` START_TYPE: __________
- [ ] SC1 probe result post-reboot: __________

**Verdict:** PENDING

---

## SC3 — Fail-closed with remediation when service stopped ⏳ PENDING

**Scenario:** When `network.block:true` finds the service stopped, nono either auto-starts it (elevated) or fails closed naming the exact remediation command — NEVER proceeds unenforced.

**Steps:**
```powershell
# Step 1 (ELEVATED) — stop service to simulate dev-layout / stopped state
sc stop nono-wfp-service
sc query nono-wfp-service           # Confirm STATE: 1 STOPPED

# Step 2 (NON-ELEVATED) — must fail closed, must NOT print "hello"
nono run --profile claude-code --block-net --allow-cwd -- cmd /c echo hello

# Step 3 (ELEVATED) — confirm auto-start works when elevated
sc start nono-wfp-service
sc query nono-wfp-service           # STATE: 4 RUNNING
# then re-run non-elevated → WFP enforcement active
```

**Pass criteria:** Step 2 exits **non-zero** with an error containing BOTH `nono-wfp-service` and `nono setup --start-wfp-service`, indicates elevation required, and does **NOT** print `hello`.

- [ ] Exit code (Step 2): __________
- [ ] `hello` printed? (must be NO): __________
- [ ] Step 3 re-run after elevated start enforced block? __________

**Exact fail-closed error text (Step 2 stderr) — paste verbatim:**
```
<paste here>
```

**Verdict:** PENDING

---

## SC4 — Clean uninstall leaves nothing behind ✅ PASS

**Scenario:** `sc stop nono-wfp-service` + `msiexec /x` of the machine MSI leaves no service registration, no install dir, no orphaned WFP filters.

**Observed (PASS):** After stop + `msiexec /x` of the v0.57.12 machine MSI:
- `sc query nono-wfp-service` → **FAILED 1060** (service does not exist).
- Install directory removed (no `C:\Program Files\nono` residue).
- No orphaned `nono` WFP filters / `NONO_SUBLAYER_GUID` sublayer remain.
- No MsiInstaller FAILED/ERROR entries for the nono install.

**Verdict:** Leave-nothing invariant holds; `start=auto` (62-02) did NOT regress the Phase 53 REQ-DRN-01 clean-uninstall guard. Note: the runtime self-cleans its per-run WFP filters per run, in addition to MSI-time service removal.

---

## SC5 — Confined child cannot reach WFP control pipe ⏳ PENDING (expected PASS)

**Scenario:** The confined child cannot connect to the WFP control pipe directly; only `nono.exe` (Medium IL) connects. The block is a WFP kernel filter, not a pipe access denial.

**Verification:** SC5 is verified as part of the SC1 run (not a separate test). Confirm against the SC1 output:

- [ ] SC1 block was a kernel filter (curl exit 6 / TCP timeout/reset), NOT `Access is denied` on the pipe.
- [ ] `nono.exe` console showed enforcement active, NOT an `Access is denied` pipe error.

Since SC1 PASSED with a clean kernel-level block and no pipe error in the nono.exe output, SC5 is expected PASS — confirm the absence of any `Access is denied` line in the SC1 run and mark PASS.

**Verdict:** PENDING (confirm)

---

## Sign-off

- [ ] All 5 SCs PASS
- [ ] SC3 fail-closed error text captured verbatim above
- [ ] Operator: __________  Date: __________

When all five show PASS, mark phase 62 complete and proceed to `/gsd:verify-work`, then Phase 61 (ship/release v2.9).
