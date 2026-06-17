# 77-03 SUMMARY — CPLT-03 unattended scripted gate + host proof

**Plan:** 77-03 (`autonomous: false`, blocking human-verify checkpoint)
**Requirement:** CPLT-03
**Status:** Deliverables (Task 1, Task 2) COMPLETE. Host-proof checkpoint (Task 3) ran on a real Win11 host and **FAILed**, surfacing a genuine CPLT-01 coverage gap → **routed to gap closure** (operator decision 2026-06-17).

## Self-Check: PARTIAL

Tasks 1 & 2 fully delivered, committed, and their automated checks pass. Task 3 (host-gated end-to-end proof) was executed live and correctly returned a **FAIL** verdict — a real confinement gap, not a false-PASS. The phase cannot close until the gap is fixed and the gate re-run returns PASS.

## What was delivered

### Task 1 — `scripts/gates/copilot-e2e.ps1` (CPLT-03 dark-factory gate) ✅
Two-function gate (`Test-Precondition` + `Invoke-Gate`) dot-sourced by `scripts/verify-dark.ps1 --gate copilot-e2e`. Auto-discovered by the runner; never calls `exit`; verdict dict uses the locked key order `gate, verdict, reason, detail, timestamp`. Runner integration proven end-to-end (auto-discovery, persist-before-emit/WR-04, exit mapping PASS=0/FAIL=2/SKIP=3/error=4).

The gate was **hardened on-host** during the Task 3 proof (commit `78f1101f`):
- **Critical false-PASS fix (T-77-03):** the gate originally PASSed on a nono launch error (`"Profile not found"`) because the output was non-empty with no denial marker. Added a nono-launch-error FAIL rule + a non-zero-exit FAIL rule, **ordered after** the confinement-FAIL checks so a real `STATUS_ACCESS_DENIED` / module-resolution crash is never masked.
- Broadened the module-resolution-crash detection to catch the signature ancestor-RA denial (`realpathSync` EPERM `lstat` / `Failed to load package index`).
- OQ-3 executable/interpreter coverage settled empirically: resolves the real `copilot` command **and follows the WinGet `…\WinGet\Links` symlink** to the `…\WinGet\Packages\GitHub.Copilot_*` dir; covers the Node interpreter dir (`C:\Program Files\nodejs`) — nono refuses to launch a partially-confined engine whose interpreter is uncovered.
- cwd/workspace + R-B3 ownership: runs from a dedicated `--workspace` under `%USERPROFILE%`; sets the current user as owner (by SID) + grants Full Control (WRITE_OWNER) so the mandatory-label apply succeeds even from an elevated session.
- Swapped `Start-Process` → `ProcessStartInfo.ArgumentList` (correct quoting of paths with spaces) + async stream drain (no pipe-buffer deadlock).

### Task 2 — `proj/DESIGN-engine-abstraction.md` ✅
Documented the permanent, non-destructive ancestor-RA grant (D-09): the two-part chain (runtime per-run-SID grant on user-owned ancestors + one-time-admin `ALL APPLICATION PACKAGES` `S-1-15-2-1` RA on `C:\`/`C:\Users`), attribute-read only, idempotent, persists after uninstall, no `--revoke`. Grep verify passes (`S-1-15-2-1` + `non-destructive` present).

### Task 3 — Host-gated end-to-end proof ⚠️ FAIL (gap found)
Executed live on Win11 (WinGet-installed `@github/copilot` 1.0.62, `gh` authenticated, fresh `nono 0.62.2`).

**Proven working:**
- CPLT-02 admin grant works: `nono setup --grant-ancestors --profile copilot-cli` granted `S-1-15-2-1` RA on `C:\` and `C:\Users`; **idempotency confirmed** (2nd run clean, exit 0).
- The confined launch runs fully: AppContainer applied → WinGet `copilot.exe` → system `node.exe` → package self-extracts → ESM resolution begins.

**The FAIL (recorded verdict):** `realpathSync` `lstat` EPERM. As each ancestor was covered, the denial climbed exactly one level: `C:\` → `C:\Users` (both fixed by the admin grant) → **stuck at `C:\Users\OMack`**.

**Root cause (gap-closure target):** the WinGet copilot **self-extracts its Node package under the `--workspace`** (`…\nono-copilot-e2e-gate\.nono-runtime\…\AC\copilot\pkg\…`), so `realpathSync` walks the **workspace's** ancestor chain. But 77-01's `AppliedAncestorReadAttributesGuard` walks only the **target binary's** chain (copilot.exe); the workspace's ancestors get *traverse* (0x21), not *read-attributes* (0x80). So `C:\Users\<user>` — parent of the workspace — never gets RA.

**Fix (for the gap-closure plan):** extend the CPLT-01 runtime guard to also walk the **workspace's** user-owned ancestor chain and grant RA (its chain has no non-owned mid-ancestors, so it cleanly covers `C:\Users\<user>` and stops at `C:\Users`). Re-run the gate after rebuild for a real PASS; watch for any further uncovered ancestor (e.g. `C:\Program Files` for the node interpreter chain).

## Recorded verdict artifact

`.nono-runtime/verdicts/copilot-e2e.json`:
```json
{"gate":"copilot-e2e","verdict":"FAIL","reason":"Node module-resolution crash under confinement (realpathSync/lstat ancestor walk denied — run the CPLT-02 one-time-admin grant: nono setup --grant-ancestors --profile copilot-cli)", ... "detail":{"...":"EPERM ... lstat 'C:\\Users\\OMack' ... realpathSync"}}
```
(`SKIP_HOST_UNAVAILABLE` is also a clean recorded outcome when network/auth/Copilot is absent — proven earlier in the session; the gate's D-07/D-08 split is verified.)

## Commits
- `1720b1d0` feat(77-03): add copilot-e2e dark-factory gate (CPLT-03)
- `d9a17a76` docs(77-03): document permanent non-destructive ancestor-RA grant (D-09)
- `78f1101f` fix(77-03): harden copilot-e2e gate — kill false-PASS + settle OQ-3 on-host

## Key files
- created: `scripts/gates/copilot-e2e.ps1`
- modified: `proj/DESIGN-engine-abstraction.md`

## Next step
Gap-closure plan (operator-selected route): fix the CPLT-01 runtime guard to cover the workspace's user-owned ancestors, rebuild, and re-run `verify-dark.ps1 --gate copilot-e2e` for a real PASS to fully close CPLT-03.
