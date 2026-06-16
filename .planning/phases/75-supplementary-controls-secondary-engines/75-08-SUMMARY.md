---
phase: 75-supplementary-controls-secondary-engines
plan: "08"
subsystem: confinement
tags: [windows, appcontainer, node-esm, copilot, spike, gap-closure, re-scoped]

# Dependency graph
requires:
  - phase: 75-supplementary-controls-secondary-engines
    plan: "05"
    provides: SC3 finding (Copilot Node-ESM lstat('C:\\') under AppContainer)

provides:
  - Full root-cause characterization of Node-ESM-under-AppContainer failure
  - Disposition decision (operator-approved 2026-06-16): re-scope — Copilot confine-only; claude-code is the end-to-end Engine-2
  - Documented FUTURE fix recipe (admin drive-ACL prereq + nono ancestor READ_ATTRIBUTES change) for any Node-ESM engine

affects: [Phase 75 close, future Node-ESM-engine work]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Node ESM realpathSync lstat's EVERY ancestor of the module path up to the drive root; each needs FILE_READ_ATTRIBUTES for the AppContainer package SID"
    - "FILE_TRAVERSE != FILE_READ_ATTRIBUTES: a traverse-only ancestor grant does NOT satisfy lstat; realpath-walking runtimes need RA on every ancestor"
    - "System-owned ancestors (C:\\, C:\\Users) need a one-time admin RA grant for ALL APPLICATION PACKAGES (S-1-15-2-1) — a non-elevated user has no WRITE_DAC to set them at runtime"

key-files:
  created: []
  modified: []

requirements: [SUPP-03]
gap_closure: true
status: resolved-by-rescope
---

# Phase 75 Plan 08: GAP-75-C Spike — Node-ESM under AppContainer (SC3) Summary

## Outcome: SPIKE COMPLETE → operator re-scoped (no code change shipped)

The Task-1 spike fully characterized why a daemon/`nono run`-launched **Copilot CLI** (a Node ESM
app) confines correctly but does not run to completion under AppContainer. The operator's
disposition (2026-06-16): **re-scope** — Copilot stays *confine-only*; **`claude-code` (native PE,
proven confined end-to-end in the 75-07 UAT) is the end-to-end Engine-2**. The full fix recipe is
documented below as a FUTURE option for Node-ESM engines.

## Root cause (live-characterized on Win11 26200)

Node's ESM resolver (`finalizeResolution` → `realpathSync`) calls `lstat` on **every ancestor** of
the module path, walking to the drive root. Under AppContainer (deny-by-default for the lowbox),
each ancestor needs `FILE_READ_ATTRIBUTES` for the per-run package SID. Stepwise proof, granting
`ALL APPLICATION PACKAGES (*S-1-15-2-1) :(RA)` on each ancestor in turn:

| Grant applied (one-time admin `icacls … :(RA)`) | Next failure |
|--------------------------------------------------|--------------|
| (none) | `EPERM lstat 'C:\'` |
| `C:\` | `EPERM lstat 'C:\Users'` |
| `C:\` + `C:\Users` | `EPERM lstat 'C:\Users\OMack'` |

`C:\Users\OMack` is **user-owned**; nono grants it `FILE_TRAVERSE` today (ancestor-traverse guard),
but `lstat` needs `FILE_READ_ATTRIBUTES` — traverse does not include it — so it still fails there.

### Options evaluated
- **Option B (Node resolver flag) — RULED OUT empirically.** `NODE_OPTIONS=--preserve-symlinks
  --preserve-symlinks-main` did NOT avoid the `realpathSync`/`lstat`. Confirmed the broker does NOT
  strip env (a control probe `MYPROBE` reached the confined child), so the flag *was* delivered and
  simply doesn't prevent the ancestor walk for this package's loader.
- **Option A (drive-root attribute grant) — feasible but admin-gated + needs nono code.** Works
  (each grant advanced the failure one ancestor up), but: (1) system-owned ancestors `C:\` and
  `C:\Users` require a **one-time admin** ACL change (a non-elevated user has no `WRITE_DAC`), and
  (2) user-owned ancestors need nono to grant `FILE_TRAVERSE | FILE_READ_ATTRIBUTES` (not just
  traverse) on the per-run package SID.

## FUTURE fix recipe (documented; NOT implemented per re-scope)

To make any Node-ESM engine (Copilot, etc.) complete confined:
1. **Admin host-prereq (one-time):** `icacls C:\ /grant "*S-1-15-2-1:(RA)"` and
   `icacls C:\Users /grant "*S-1-15-2-1:(RA)"` (read-attributes only — no data read; minor,
   acceptable info-disclosure on normally-traversable system roots).
2. **nono code:** upgrade the per-run package-SID **ancestor** grants from `FILE_TRAVERSE` to
   `FILE_TRAVERSE | FILE_READ_ATTRIBUTES` on user-owned ancestors, in BOTH the daemon
   `DaemonDaclGuard` ancestor walk (launch.rs) and the `nono run` `AppliedAncestorTraverseGuard`
   (exec_strategy_windows/dacl_guard.rs).
3. **copilot-cli profile:** add `windows_interpreters: ["node.exe"]` (A4=YES) + follow the
   engine-exe symlink (WinGet Links → Packages) for coverage.

## SC3 disposition (final)

- **Copilot confinement: PASS** — write-outside-workspace denied; fail-secure launch-coverage gate
  works; AppContainer spawn confirmed.
- **Copilot end-to-end completion: DEFERRED** — requires the admin-prereq + nono ancestor-RA change
  above. Documented as a future Node-ESM-engine enablement; not shipped (avoids baking an admin
  drive-ACL prereq into the default product story for one engine).
- **Engine-2 (end-to-end) = `claude-code`** — native PE, confined cleanly in the 75-07 UAT
  (launched under AppContainer, no DLL-death, write-outside denied, clean reap), zero host prereq.

## SUPP-03 satisfaction

- Engines (≥2, end-to-end confined): **Aider** (Ph71) + **claude-code** (75-07). ✅
- Bindings (≥2, confined on Win11): **nono-py** (Ph72) + **nono-ts** (75-05 SC5). ✅
- Copilot: confine-only (documented Node-ESM/AppContainer limitation + future fix recipe).

## Verification

This plan was a spike + disposition; no production code changed (per re-scope). All findings are
live-characterized on Win11 26200 and recorded here + in 75-HUMAN-UAT.md. The icacls test grants on
`C:\` and `C:\Users` were reverted after the spike (operator).
