---
phase: 79-wfp-egress-isolation-nono-ts-ergonomics
verified: 2026-06-17T23:59:00Z
status: human_needed
score: 4/5 must-haves verified
overrides_applied: 0
overrides:
  - must_have: "Two concurrent confined agents with distinct AppContainer SIDs run concurrently against a loopback mock server; Agent A exits 0 / Agent B exits non-zero"
    reason: "The loopback/egress-probe design was empirically falsified on the live Win11 host (confined AppContainers have SECURITY_CAPABILITIES CapabilityCount=0 — zero network capability; no egress probe can distinguish allowed vs blocked, and direct nono run installs no WFP filter). With operator approval across 3 AskUserQuestion decisions, the gate was redesigned as a daemon-path structural proof: two agents are launched via 'nono agent launch', their distinct package SIDs are parsed, and live kernel WFP state (netsh wfp show filters) is inspected to assert blocked-agent-has-per-SID-filter + allowed-agent-has-none. The actual ROADMAP SC1 (WFP-01) reads 'one confined agent's allowed egress succeeds while a second agent is denied — both verdicts machine-verifiable in one unattended run' — the structural proof satisfies the machine-verifiable and isolation intent. The original plan must_have wording is superseded."
    accepted_by: "operator (3 AskUserQuestion approvals documented in 79-01-SUMMARY.md)"
    accepted_at: "2026-06-17T23:08:03Z"
human_verification:
  - test: "Confirm that the ROADMAP and REQUIREMENTS.md stale-status entries (Phase 79 marked 'Planning / 0/2', WFP-01 / TSRG-01 marked 'Pending') are updated to reflect completion."
    expected: "ROADMAP progress table: '2/2 | Complete | 2026-06-17'; REQUIREMENTS traceability: both WFP-01 and TSRG-01 show Complete. Phase 79 checkbox flipped to [x]."
    why_human: "SDK phase.complete and checklist-flip are human-driven operations per feedback_sdk_roadmap_checklist_not_flipped. The code is complete but the planning metadata is stale. Cannot auto-apply."
---

# Phase 79: WFP Egress Isolation + nono-ts Ergonomics — Verification Report

**Phase Goal:** Prove per-agent WFP egress isolation empirically via an automated two-agent test (allowed vs. denied egress on the same host), and ship `confinedRun` ergonomics in nono-ts so callers get a working confined run with no manual profile or coverage flags.
**Verified:** 2026-06-17T23:59:00Z
**Status:** human_needed (code complete; one stale-metadata admin item requires human action)
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Automated gate (`scripts/verify-dark.ps1 --gate wfp-egress-isolation`) proves per-SID WFP egress isolation: blocked agent has a per-SID kernel WFP filter; allowed agent has none; both verified machine-readable in one unattended run | VERIFIED | `.nono-runtime/verdicts/wfp-egress-isolation.json` exists with `"verdict":"PASS"`, `"blockedHasFilter":true`, `"openHasFilter":false`, distinct AppContainer SIDs (`S-1-15-2-1270…` / `S-1-15-2-3965…`). Timestamp `2026-06-17T23:08:03.430Z`. |
| 2 | Gate exports `Test-Precondition` and `Invoke-Gate` (dark-factory contract), has no bare `exit` outside comments, and asserts `blockedHasFilter + !openHasFilter` | VERIFIED | `grep -v '^#' … \| grep -c 'exit '` → 0. `grep -n "^function Test-Precondition\|^function Invoke-Gate"` → lines 106 + 147. Verdict logic at lines 250/260/270 confirmed. Commit `0624256d`. |
| 3 | Three policy.json profiles (`nono-ts-wfp-test-open`, `nono-ts-wfp-test-blocked`, `nono-ts-default`) are present; `nono-ts-wfp-test-blocked` is the only `block:true` profile | VERIFIED | `grep -c '"block": true' policy.json` → 1. All three keys appear at lines 938, 956, 974. `nono-ts-wfp-test-blocked` has `"network": { "block": true }`, others have `false`. `nono-ts-default` has `"windows_low_il_broker": true`. Commit `507ff683`. |
| 4 | D-03 default-profile injection (`nono-ts-default`) is the first statement of `confined_run` in `src/windows_confined_run.rs`, before the validation guard | VERIFIED | `confined_run` body line 160: `let profile = profile.or_else(\|\| Some("nono-ts-default".to_string()));` — appears before the `if profile.is_none() && allow…` guard at line 184. Commit `e84e4d0`. |
| 5 | nono-ts `package.json` "test" script points to `tests/test_confined_run_default.js`; that test exists, has platform-skip, calls `confinedRun` with `undefined`/`undefined` allow/profile; SUMMARY documents `npm test` PASS | VERIFIED | `package.json` line 53: `"test": "node tests/test_confined_run_default.js"`. Test file exists, platform-skip at line 18, `confinedRun('node.exe', ['-e', 'process.exit(0)'], undefined, undefined, ws, 30)` at lines 78–85. `ws` is `path.join(os.homedir(), '…')` (not a drive root). Commit `aa90938`. SUMMARY documents `npm test` exit 0 with broker-arm PASS. |

**Score:** 5/5 truths verified (Truth 2 in original plan frontmatter re loopback-probe superseded by operator-approved override per deviation documented in 79-01-SUMMARY.md)

### Deferred Items

None — all ROADMAP success criteria are addressed in Phase 79 and verified above.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `scripts/gates/wfp-egress-isolation.ps1` | WFP-01 dark-factory gate | VERIFIED | 279 lines; exports `Test-Precondition` (line 106) + `Invoke-Gate` (line 147); no bare `exit`; all 4 verdict branches (PASS/FAIL-A/FAIL-B/SKIP variants) present |
| `crates/nono-cli/data/policy.json` | 3 new profiles | VERIFIED | Lines 938–991: `nono-ts-wfp-test-open` (`block:false`), `nono-ts-wfp-test-blocked` (`block:true`, only such profile), `nono-ts-default` (`block:false`, `windows_low_il_broker:true`) |
| `C:\Users\OMack\nono-ts\src\windows_confined_run.rs` | D-03 + D-04 wiring | VERIFIED | Line 160 = D-03 first-statement injection; lines 166–177 = D-04 `resolve_exe_dir` + allow shadow; `resolve_exe_dir` helper at lines 83–106 (best-effort `Ok(None)`, never `Err`); 3 new unit tests + 1 retargeted test |
| `C:\Users\OMack\nono-ts\tests\test_confined_run_default.js` | SC4 integration test | VERIFIED | 126 lines; platform-skip; calls `confinedRun` with `undefined`/`undefined`; asserts `exitCode === 0`; `ws` under `os.homedir()` (R-B3 label-grant constraint honoured) |
| `C:\Users\OMack\nono-ts\package.json` | npm test wiring | VERIFIED | `"test": "node tests/test_confined_run_default.js"` at line 53 |
| `.nono-runtime/verdicts/wfp-egress-isolation.json` | Live gate verdict | VERIFIED | `"verdict":"PASS"` with full detail block (`blockedHasFilter:true`, `openHasFilter:false`, distinct SIDs, timestamp 2026-06-17T23:08:03.430Z) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `wfp-egress-isolation.ps1` Invoke-Gate | `nono agent launch` (daemon path) | lines 169 + 208: `& $nonoExe agent launch --profile …` | WIRED | Gate invokes daemon path; parses SID from response; inspects WFP state |
| `wfp-egress-isolation.ps1` | `nono-wfp-service` pipe | `Test-Precondition` lines 120–128: `NamedPipeClientStream` connect to `nono-wfp-control` | WIRED | Probes service before running; returns SKIP if absent |
| `confined_run` | `nono-ts-default` profile (via `nono.exe --profile nono-ts-default`) | line 160: `profile.or_else(|| Some("nono-ts-default"…))` → `build_nono_run_args` at line 196 | WIRED | D-03 injects profile before validation guard; `build_nono_run_args` emits `--profile nono-ts-default` |
| `test_confined_run_default.js` | `confined_run` napi export | `require('../index.js').confinedRun` (line 23) | WIRED | Requires index.js which loads the `.node` napi binary; calls `confinedRun` directly |
| `package.json` "test" | `tests/test_confined_run_default.js` | `"test": "node tests/test_confined_run_default.js"` (line 53) | WIRED | `npm test` runs the integration test directly |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `wfp-egress-isolation.ps1` `Invoke-Gate` | `$blockedHasFilter`, `$allowedHasFilter` | `Get-NonoBlockSids` → `netsh wfp show filters` → XML parse → SID match against real WFP kernel state | Yes — live kernel WFP dump, baseline delta | FLOWING |
| `test_confined_run_default.js` | `result.exitCode` | `confinedRun` → `nono.exe run --profile nono-ts-default` → AppContainer Low-IL child running `node.exe -e process.exit(0)` | Yes — live confined child exit code | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Gate verdict file has `"verdict":"PASS"` | Read `.nono-runtime/verdicts/wfp-egress-isolation.json` | `verdict:PASS, blockedHasFilter:true, openHasFilter:false` — distinct SIDs confirmed | PASS |
| Gate has no bare `exit` outside comments | `grep -v '^#' gate.ps1 \| grep -c 'exit '` | 0 | PASS |
| Both gate functions exported at top level | `grep -n "^function Test-Precondition\|^function Invoke-Gate"` | Lines 106 + 147 | PASS |
| D-03 is first statement of `confined_run` | Read `windows_confined_run.rs` line 160 | `let profile = profile.or_else(|| Some("nono-ts-default"…));` precedes validation guard | PASS |
| Only one `block:true` profile in policy.json | `grep -c '"block": true' policy.json` | 1 | PASS |
| nono-ts commits exist in sibling repo | `git show --stat e84e4d0 aa90938` in nono-ts | Both commits verified, correct file changes | PASS |

### Probe Execution

No conventional `scripts/*/tests/probe-*.sh` probes declared for this phase. The dark-factory gate (`scripts/verify-dark.ps1 --gate wfp-egress-isolation`) was executed on the live Win11 host during the phase checkpoint; its machine-readable verdict is persisted at `.nono-runtime/verdicts/wfp-egress-isolation.json` (PASS). Re-execution requires a live Win11 host with nono-agentd + nono-wfp-service running and elevation; not re-run in this static verification (the stored verdict is the authoritative gate record per the dark-factory mandate).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| WFP-01 | 79-01-PLAN.md | Per-agent WFP egress isolation empirically proven | SATISFIED | Gate PASS verdict with `blockedHasFilter:true`, `openHasFilter:false`, distinct package SIDs — structural proof that daemon-path installs per-SID WFP filter for blocked agent only |
| TSRG-01 | 79-02-PLAN.md | `confinedRun` defaults to Low-IL broker arm, auto-covers exe dir | SATISFIED | D-03 first-statement injection of `nono-ts-default` (has `windows_low_il_broker:true`); D-04 `resolve_exe_dir` auto-cover; `npm test` PASS documented in SUMMARY (node.exe ran inside Low-IL AppContainer, exitCode=0) |

Note: REQUIREMENTS.md traceability rows for WFP-01 and TSRG-01 still show `Pending` and Phase 79 ROADMAP progress table shows `0/2 | Planning`. These are stale metadata — see Human Verification Required below.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `windows_confined_run.rs` | 300 | `status.code().unwrap_or(1)` inside `confine()` (not `confined_run`) | Info | This is in the `confine` (Shape B) function which was pre-existing; not introduced in Phase 79. The unwrap_or here is a safe default for a process exit code, not a security-critical path. Not a Phase 79 regression. |
| `windows_confined_run.rs` | 374, 388 | `stdout_thread.join().unwrap_or_default()` | Info | Pre-existing in `do_spawn_and_wait`; `.unwrap_or_default()` on a thread join is a standard safe fallback. Not introduced by Phase 79 changes. |

No TBD, FIXME, or XXX markers found in Phase 79 modified files. No placeholder implementations. No stub patterns.

### Human Verification Required

#### 1. Update stale ROADMAP and REQUIREMENTS.md planning metadata

**Test:** In `.planning/ROADMAP.md`, flip Phase 79 checkbox from `[ ]` to `[x]` and update the progress table row from `0/2 | Planning | -` to `2/2 | Complete | 2026-06-17`. In `.planning/REQUIREMENTS.md`, update WFP-01 and TSRG-01 traceability rows from `Pending` to `Complete`.

**Expected:** Planning artifacts reflect that Phase 79 is complete and both requirements are satisfied.

**Why human:** The SDK `phase.complete` and checklist flip are human-driven per project feedback (`feedback_sdk_roadmap_checklist_not_flipped.md`). The code, gate, and live verdict are all complete — only the planning metadata is stale.

### Gaps Summary

No gaps blocking goal achievement. All five must-haves are verified. The one human-verification item is a planning-metadata administrative update (stale checkbox + progress table), not a code deficiency.

The plan's original OQ-1 design (loopback egress probe) was superseded by operator-approved daemon-path structural proof — this is accepted via the override entry in the frontmatter above. The actual WFP-01 ROADMAP success criterion (machine-verifiable per-SID isolation in one unattended run) is satisfied by the stored PASS verdict with distinct SIDs and `blockedHasFilter:true`/`openHasFilter:false`.

---

_Verified: 2026-06-17T23:59:00Z_
_Verifier: Claude (gsd-verifier)_
