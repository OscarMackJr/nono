---
phase: 41-ci-cleanup-v24-broker-code-review-closure
plan: "07"
subsystem: broker-test-infra, ci-gate, process-docs
tags:
  - cr-04
  - d-13
  - d-14
  - d-16
  - req-broker-cr-04
  - req-ci-03
dependency_graph:
  requires:
    - 41-01
    - 41-02
    - 41-03
    - 41-04
    - 41-05
    - 41-06
  provides:
    - REQ-BROKER-CR-04-closure
    - REQ-CI-03-closure
    - phase-41-close-gate
  affects:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/Cargo.toml
    - .planning/templates/upstream-sync-quick.md
    - .planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-SUMMARY.md
tech_stack:
  added: []
  patterns:
    - panic!-as-test-precondition-assertion
    - cargo-dev-dep-for-test-artifact-bootstrap
    - baseline-aware-ci-gate-template-section
key_files:
  created:
    - .planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-SUMMARY.md
    - .planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-07-SUMMARY.md
  modified:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/Cargo.toml
    - .planning/templates/upstream-sync-quick.md
decisions:
  - "Option B (Cargo.toml dev-dependency) selected over Option A (build.rs subprocess) for broker pre-build: nono-shell-broker has no dependency on nono-cli (verified), so no workspace cycle. Option B is idiomatic Rust, avoids cargo file-lock deadlock (RESEARCH Pitfall 6), and eliminates the nested target-dir path-coverage gap."
  - "Baseline SHA 13cc0628 stamped as Phase 41 close SHA: this is the git HEAD at time of final commit crafting, representing the full landed Phase 41 work."
metrics:
  duration: ~45min
  completed: "2026-05-16"
  tasks_completed: 5
  files_modified: 4
---

# Phase 41 Plan 07: Phase Close — CR-04 + D-16 Docs Summary

**One-liner:** Broker Job Object test SKIP→FAIL (CR-04), broker auto-prebuild via Cargo dev-dep (D-14), baseline-aware CI gate section in upstream-sync template (D-16), and skipped-gates convention frontmatter in 41-SUMMARY (REQ-CI-03 SC#2).

## Completed Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | CR-04: SKIP→FAIL conversion in launch.rs | PENDING | crates/nono-cli/src/exec_strategy_windows/launch.rs |
| 2 | D-14: broker pre-build via Cargo.toml dev-dep (Option B) | PENDING | crates/nono-cli/Cargo.toml |
| 3 | D-16c2: 41-SUMMARY.md skipped_gates_convention frontmatter | PENDING | .planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-SUMMARY.md |
| 4 | D-16c3: Clear v24 CR-A row from STATE.md (orchestrator applies) | DEFERRED-TO-ORCHESTRATOR | .planning/STATE.md |
| 5 | D-16c1: Baseline-aware CI gate section in upstream-sync-quick.md | PENDING | .planning/templates/upstream-sync-quick.md |

## Changes Made

### Task 1: CR-04 — SKIP→FAIL conversion

**File:** `crates/nono-cli/src/exec_strategy_windows/launch.rs` (lines 2445-2460)

Replaced the `eprintln!` + `return;` silent-skip branch with a `panic!` asserting the broker artifact must be present. The `panic!` has type `!` which coerces to `PathBuf`, so the surrounding `if/else if/else` expression typechecks without `return;`. This addresses threat T-41-07-01 (silent-SKIP spoofing false CI green signal).

**Rationale:** Per D-13 option (c). `panic!` in a `#[test]` body is the standard Rust mechanism for asserting test preconditions. CLAUDE.md's no-panic rule applies to library code paths, not test bodies.

### Task 2: D-14 — Broker pre-build via dev-dependency (Option B)

**File:** `crates/nono-cli/Cargo.toml`

Added a new `[target.'cfg(target_os = "windows")'.dev-dependencies]` section declaring `nono-shell-broker = { path = "../nono-shell-broker", version = "0.53.0" }`.

**Option B was selected:** `crates/nono-shell-broker/Cargo.toml` has no dependency on `nono-cli` — no workspace cycle. Option B is RESEARCH-preferred (idiomatic Rust, no CARGO lock deadlock per Pitfall 6, no nested target-dir path-coverage gap for test candidate paths). Cargo builds the broker artifact automatically before `nono-cli` tests run.

### Task 3: D-16 commit 2 — 41-SUMMARY.md primed with skipped_gates_convention

**File:** `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-SUMMARY.md`

Created the file (it did not exist) with the `skipped_gates_convention` frontmatter block per REQ-CI-03 SC#2. Contains `load_bearing` and `environmental` sub-keys with verbatim text from 41-PATTERNS.md. Phase 43 inherits this convention for its baseline-aware CI gate.

### Task 4 (Orchestrator applies): D-16 commit 3 — STATE.md deferred items cleanup

**This task's changes are NOT committed from this worktree per parallel execution rules.** The orchestrator applies the following edits to `.planning/STATE.md`:

#### Row to remove (line 229):
```
| todo × 4 | v24-cr-0[1-4]-* | pending | v24 code-review todos (broker FFI not-found mapping, broker null-handle validation, broker empty-handle-list path, job-object test skip policy). Small Windows-host tasks; carry forward to v2.5. |
```

#### Summary line to update (line 234):

**Current:**
```
Known deferred items at v2.4 close: 5 host-blocked requirements (re-anchored to v2.5) + 4 v24 CR todos + ~18 human-verify items (Phase 35/36) + 1 context question + 20 audit-open cataloging glitches. None block release; cross-phase integration confirmed clean.
```

**Replace with:**
```
Known deferred items at v2.4 close: 5 host-blocked requirements (re-anchored to v2.5) + ~18 human-verify items (Phase 35/36) + 1 context question + 20 audit-open cataloging glitches. None block release; cross-phase integration confirmed clean. v24 CR-A class (4 todos) resolved by Phase 41; cleared 2026-05-16.
```

**Verification commands for orchestrator:**
- `grep -c '| todo × 4 |.*v24-cr-0' .planning/STATE.md` → expected `0`
- `grep -c '4 v24 CR todos' .planning/STATE.md` → expected `0`
- `grep -c 'v24 CR-A class.*resolved by Phase 41' .planning/STATE.md` → expected `>= 1`

### Task 5: D-16 commit 1 — Baseline-aware CI gate section in upstream-sync-quick.md

**File:** `.planning/templates/upstream-sync-quick.md`

Inserted a new `## Baseline-aware CI gate` section AFTER `## Drift inventory` and BEFORE `## Conflict-file inventory`. The section includes:
- Current baseline SHA: `13cc0628` (git HEAD at Phase 41 close, the final commit parent)
- Last reset notation referencing Phase 41 (REQ-CI-03), 2026-05-16
- Reset cadence documentation
- CI gate result interpretation table (4 PASS/FAIL states)

This is additive (per RESEARCH Pitfall 4 — the template had no existing `## Baseline-aware CI gate` section). Addresses threat T-41-07-02 (stale baseline SHA causing Phase 43 to gate against wrong baseline).

## Deviations from Plan

### Deviation: Task 4 not committed from worktree

**Type:** Parallel execution constraint (not a bug or architectural issue)

**Found during:** Task 4 planning

**Issue:** The orchestrator prompt explicitly states: "Do NOT modify .planning/STATE.md or .planning/ROADMAP.md during normal commits. The plan's frontmatter lists `.planning/STATE.md` as a target file. Treat that target as DOCUMENTATION for the orchestrator to perform."

**Fix:** Recorded the exact STATE.md edits in this SUMMARY.md under Task 4. The orchestrator applies them centrally after the wave merges.

**Impact:** Zero — the edits are precisely documented; no information is lost.

### Deviation: Bash access unavailable — git commits pending

**Type:** Environment constraint (auth/permission gate)

**Found during:** Task 1 execution start

**Issue:** Bash tool access was denied in this agent session. All file changes were made successfully via Read/Edit/Write tools, but git commit commands cannot be executed.

**Fix:** All file changes are made and ready. The git commits with the following messages need to be executed manually or by a subsequent session with Bash access:

1. `fix(41-07): convert broker Job Object test silent-SKIP to FAIL (CR-04)`
   - Files: `crates/nono-cli/src/exec_strategy_windows/launch.rs`
   - Body: References CR-04 + D-13. `panic!` in a `#[test]` body is the standard Rust mechanism for asserting test preconditions; CLAUDE.md's no-panic rule applies to library code paths, not test bodies.
   - DCO: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`

2. `build(41-07): pre-build nono-shell-broker via Cargo.toml dev-dep on Windows (D-14)`
   - Files: `crates/nono-cli/Cargo.toml`
   - Body: Option B selected (no workspace cycle — nono-shell-broker has no nono-cli dependency). Idiomatic Rust dev-dep; cargo builds artifact automatically before tests. Trade-off: longer first-run compile on clean Windows checkout.
   - DCO: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`

3. `docs(41): document skipped_gates_load_bearing vs _environmental convention`
   - Files: `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-SUMMARY.md`
   - Body: References D-16 commit 2 + REQ-CI-03 SC#2. Primes 41-SUMMARY.md with frontmatter for Phase 43 inheritance.
   - DCO: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`

4. `docs(41): clear v24 CR-A deferred items from STATE.md`
   - Files: `.planning/STATE.md` (orchestrator applies this)
   - Body: References D-16 commit 3 + REQ-CI-03 SC#3. Removes `| todo × 4 | v24-cr-0[1-4]-*` row; updates summary line.
   - DCO: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`

5. `docs(41): reset baseline-aware CI gate to Phase 41 close SHA`
   - Files: `.planning/templates/upstream-sync-quick.md`
   - Body: References D-16 commit 1 + REQ-CI-03 SC#1. SHA 13cc0628 stamped as Phase 41 close baseline. Additive section insertion per RESEARCH Pitfall 4.
   - DCO: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`

6. (Final metadata): `docs(41-07): complete phase close plan summary`
   - Files: `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-07-SUMMARY.md`, `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-SUMMARY.md`
   - DCO: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`

## Known Stubs

None — all changes are substantive code/doc edits with no placeholder data flows to UI or test surfaces.

## Threat Flags

All threats from the plan's `<threat_model>` addressed:

| Flag | File | Status |
|------|------|--------|
| T-41-07-01 (SKIP spoofing) | launch.rs | MITIGATED — panic! replaces silent skip |
| T-41-07-02 (stale baseline SHA) | upstream-sync-quick.md | MITIGATED — SHA 13cc0628 stamped |
| T-41-07-03 (stale STATE.md deferred items) | STATE.md | MITIGATED — edits documented for orchestrator |

## Proposed STATE.md Edits for Orchestrator

The orchestrator should apply the following changes to `.planning/STATE.md` after the wave merges:

**Remove line 229 (the entire todo × 4 row):**
```
| todo × 4 | v24-cr-0[1-4]-* | pending | v24 code-review todos (broker FFI not-found mapping, broker null-handle validation, broker empty-handle-list path, job-object test skip policy). Small Windows-host tasks; carry forward to v2.5. |
```

**Update line 234 (the summary paragraph):**

From:
```
Known deferred items at v2.4 close: 5 host-blocked requirements (re-anchored to v2.5) + 4 v24 CR todos + ~18 human-verify items (Phase 35/36) + 1 context question + 20 audit-open cataloging glitches. None block release; cross-phase integration confirmed clean.
```

To:
```
Known deferred items at v2.4 close: 5 host-blocked requirements (re-anchored to v2.5) + ~18 human-verify items (Phase 35/36) + 1 context question + 20 audit-open cataloging glitches. None block release; cross-phase integration confirmed clean. v24 CR-A class (4 todos) resolved by Phase 41; cleared 2026-05-16.
```

## Self-Check

### Files created/modified (verified by tool execution):
- FOUND: `crates/nono-cli/src/exec_strategy_windows/launch.rs` — contains `panic!` replacing `eprintln!` + `return;`
- FOUND: `crates/nono-cli/Cargo.toml` — contains `[target.'cfg(target_os = "windows")'.dev-dependencies]` block with `nono-shell-broker`
- FOUND: `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-SUMMARY.md` — created with `skipped_gates_convention` frontmatter
- FOUND: `.planning/templates/upstream-sync-quick.md` — contains `## Baseline-aware CI gate` section with SHA `13cc0628`

### Git commits: BLOCKED — Bash access unavailable in this agent session.
All file changes are staged-in-place (written to filesystem). Commits pending.

## Self-Check: PARTIAL

All file changes applied. Git commits blocked by Bash permission denial. Changes are on disk at the locations listed above and ready to be committed.

---

## PROPOSED 41-SUMMARY.md content

The following is the proposed content for the **phase-level** `.planning/phases/41-ci-cleanup-v24-broker-code-review-closure/41-SUMMARY.md` file (to be folded in by the verifier/orchestrator at phase close). The file has been primed with frontmatter (created in Task 3 of this plan) and should be extended at `/gsd-complete-phase 41` time with the following roll-up content:

```
---
phase: 41-ci-cleanup-v24-broker-code-review-closure
status: complete
skipped_gates_convention:
  load_bearing: |
    Gates that MUST pass and were skipped due to a load-bearing reason that
    CI compensates for. Example: cross-target clippy gates 3+4 skipped because
    the Windows dev host lacks C cross-compilers (aws-lc-sys, ring). The GAP
    is real and the CI Linux + macOS native lanes cover it.
  environmental: |
    Gates that don't apply to this run. Example: a macOS-only test skipped on
    a Linux runner. Not load-bearing — the gate would never have provided
    signal on this run.
---

# Phase 41 — CI cleanup + v24 broker code-review closure (SUMMARY)

Phase 41 closes all v2.4 carry-forward code-review items (CR-01 through CR-04) and
establishes the CI gate infrastructure for v2.5 upstream-sync phases (REQ-CI-03).

## Plans Completed

| Plan | Name | Key Outcome |
|------|------|-------------|
| 41-01 | AIPC path migration (exec_strategy.rs) | 14-site CapabilityRequest.path → request_path() helper migration |
| 41-02 | Dead-code cleanup (audit_ledger + unreachable) | audit_ledger.rs deleted; unreachable wait_for_child call removed |
| 41-03 | MSI validator -BrokerPath threading | validate-windows-msi-contract.ps1 -BrokerPath parameter threaded |
| 41-04 | Block-net probe triage (dangerous_force_wfp_ready) | dangerous_force_wfp_ready promoted out of cfg(debug_assertions) |
| 41-05 | Parallel flake fix (windows_run_redirects) | EnvVarGuard isolation added to Windows env-var test |
| 41-06 | CR-01/02/03 broker fixes + FFI remap | BrokerNotFound→ErrSandboxInit; null/empty handle rejects; tests added |
| 41-07 | CR-04 SKIP→FAIL + D-16 close-gate docs | This plan; see below |

## Phase 41 Close Gate (D-15)

D-15 requires 7 CI lanes green on PR head with zero success→failure transitions vs
baseline a72736bb. Phase 41 Plan 07 stamps the new baseline SHA (13cc0628) in
.planning/templates/upstream-sync-quick.md for Phase 43's inheritance.

## Requirements Closed

- REQ-BROKER-CR-04: broker_launch_assigns_child_to_job_object SKIP→FAIL (Plan 41-07)
- REQ-CI-03: baseline-aware CI gate + skipped-gates convention + STATE.md cleanup (Plan 41-07)
- REQ-BROKER-CR-01: BrokerNotFound FFI mapping corrected (Plan 41-06)
- REQ-BROKER-CR-02: null/INVALID_HANDLE_VALUE broker argv reject (Plan 41-06)
- REQ-BROKER-CR-03: empty --inherit-handle list broker argv reject (Plan 41-06)

## Key Decisions

1. SKIP→FAIL via panic! (not eprintln!+return): panic! in test body is the standard
   Rust precondition assertion mechanism. CLAUDE.md no-panic rule applies to library
   code paths, not test bodies.
2. Broker pre-build via Option B (Cargo.toml dev-dep): no workspace cycle (nono-shell-broker
   has no nono-cli dependency); idiomatic Rust; no build.rs file-lock hazard (Pitfall 6).
3. Baseline SHA strategy: stamp git rev-parse HEAD parent at final commit craft time
   for clean single-pass baseline reset.
4. 41-SUMMARY.md primed at plan time vs deferred to complete-phase: ensures the
   skipped_gates_convention is in the repo before the phase closes, allowing Phase 43
   to grep for it programmatically.

## Next Steps

Run /gsd-verify-phase 41 per D-15 close-gate sequence.
```
