---
phase: 71-engine-agnostic-launch-productionization
verified: 2026-06-14T00:00:00Z
status: human_needed
score: 5/6 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Run nono with the literal aider.exe binary (not langchain-python) end-to-end on Win11"
    expected: "Inside-workspace write lands; outside write denied; aider's internal python subprocess confined transitively"
    why_human: "SC1 roadmap text names Aider specifically. The langchain-python proof is operationally equivalent but the literal aider.exe was explicitly deferred (needs pip install aider-chat + LLM API key). Operator must decide if the langchain-python substitution closes SC1 for final milestone sign-off, or if the literal aider.exe run is required before Phase 72 begins."
---

# Phase 71: Engine-Agnostic Launch Productionization — Verification Report

**Phase Goal:** nono can parent-and-confine any covered AI agent engine (starting with Aider + a LangChain-Python profile) through one engine-neutral launch path — the validated spike-003 path promoted to a first-class, de-spiked code path that every later phase consumes.
**Verified:** 2026-06-14
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | ENG-03: `windows_interpreters` field exists in Profile struct; `aider` and `langchain-python` built-in engine profiles are present in policy.json with broker:true and interpreter coverage | VERIFIED | `profile/mod.rs:2244` (`pub windows_interpreters: Vec<String>`); `policy.json:884-919` (both profiles); tests `test_get_builtin_aider` + `test_get_builtin_langchain_python` in `builtin.rs:193-250` |
| 2 | ENG-02 (coverage gate): `validate_launch_paths` fails-secure with a named message when an interpreter is not covered by policy | VERIFIED | `sandbox/windows.rs:2154` (4-arg signature with interpreter loop); named error at `windows.rs:3780-3798` (test `validate_launch_paths_refuses_uncovered_interpreter`); component-wise path comparison (not string `starts_with`); live ENG-02-B observed on Win11 26200 |
| 3 | ENG-02 (R-B3 gate): `path_has_write_owner` pre-launch gate refuses admin-owned workspaces with a named diagnostic | VERIFIED | `sandbox/windows.rs:1222` (`pub fn path_has_write_owner`); R-B3 GATE A at `exec_strategy_windows/mod.rs:358-395` (before `AppliedLabelsGuard`); `SandboxInit` error with named WRITE_OWNER diagnostic; 3 gate tests pass |
| 4 | ENG-01 (workspace): `--workspace` is the single source of truth for child CWD and writable grant; the PowerShell→C:\ trap is closed | VERIFIED | `cli.rs:1621,2029` (`pub workspace: Option<PathBuf>`); `sandbox_prepare.rs:489-501` (auto-grant); `launch_runtime.rs:313,357` (`workspace.or(workdir)` → `ExecutionFlags.workdir`); `lpCurrentDirectory` receives the canonicalized absolute workspace |
| 5 | SC5: Job-assignment path hardened against nested-job collisions; `assign_failure_message` names GLE-5 foreign-job cause; no UI limits or breakaway flags | VERIFIED | `launch.rs:253-287` (`assign_failure_message` + `apply_process_handle_to_containment`); `CREATE_SUSPENDED` at `launch.rs:1558,1656,1675,1905`; no `JOB_OBJECT_UILIMIT*` or `BREAKAWAY_OK` in launch.rs (grep confirms empty); 3 negative tests in `assign_failure_tests` mod |
| 6 | SC1/ENG-01: A non-Claude agent engine is confined end-to-end on real Win11 26200 — inside-write lands, outside-write denied (NO_WRITE_UP), transitive subprocess denied | PARTIAL — operator-APPROVED on langchain-python; literal aider.exe deferred | `71-HUMAN-UAT.md` operator capture: SC1-1 PASS, SC1-2b PASS, SC1-3 PASS (T-71-14 transitive label inherited), SC2 PASS, ENG-02-B PASS. Engine: `langchain-python` (raw python.exe, AppContainer, Win11 26200 build 10.0.26200.0). Literal `aider.exe` pending (needs pip install aider-chat + LLM API key). ROADMAP SC1 names "Aider" in the parenthetical — the substitution is documented sufficient per spike-003 precedent and is operator-accepted, but the literal Aider run has not been executed |

**Score:** 5/6 truths verified (truth 6 is PARTIAL pending literal Aider run)

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/profile/mod.rs` | `windows_interpreters` field + `merge_profiles` wiring | VERIFIED | Field at line 2244; `dedup_append` merge at line 3269; exhaustive struct literals updated |
| `crates/nono-cli/data/policy.json` | `aider` + `langchain-python` profiles; `python_runtime` standard paths | VERIFIED | Profiles at lines 884-919; `python_runtime` group covers `~/AppData/Local/Programs/Python`, `C:/Program Files/Python31x`, `C:/Python31x`; follow-on commit `1b473b4a` added standard python.org paths |
| `crates/nono-cli/data/nono-profile.schema.json` | `windows_interpreters` as optional array-of-string | VERIFIED | Modified in commit `4639d302` per SUMMARY-01 |
| `crates/nono-cli/src/profile/builtin.rs` | `test_get_builtin_aider` + `test_get_builtin_langchain_python` assertions | VERIFIED | Lines 193-250; both tests assert `windows_low_il_broker == true`, `windows_interpreters == ["python.exe"]`, groups |
| `crates/nono/src/sandbox/windows.rs` | `validate_launch_paths(interpreters: &[PathBuf])` + `path_has_write_owner` | VERIFIED | `validate_launch_paths` at line 2154 (4-arg); `path_has_write_owner` at line 1222 with `#[must_use]`; `try_set_mandatory_label` delegates to `path_has_write_owner` (single source of truth) |
| `crates/nono/src/sandbox/mod.rs` | `validate_windows_launch_paths` wrapper updated for 4-arg signature | VERIFIED | Lines 840-849; `#[must_use]` attribute present |
| `crates/nono/src/lib.rs` | `path_has_write_owner` re-exported for CLI use | VERIFIED | Line 87 in `#[cfg(target_os = "windows")] pub use` block |
| `crates/nono-cli/src/cli.rs` | `--workspace` flag on `SandboxArgs` + `WrapSandboxArgs` | VERIFIED | Lines 1621 and 2029 |
| `crates/nono-cli/src/sandbox_prepare.rs` | `resolved_workdir()` prefers workspace; auto-grant workspace read+write | VERIFIED | Lines 227-235 (resolved_workdir); lines 484-502 (auto-grant block) |
| `crates/nono-cli/src/launch_runtime.rs` | `workspace.or(workdir)` wired into `ExecutionFlags.workdir` | VERIFIED | Lines 312-357 |
| `crates/nono-cli/src/execution_runtime.rs` | `resolve_interpreter_paths` call + `windows_resolved_interpreters` in `ExecConfig` | VERIFIED | Lines 495-529 |
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` | Interpreter threading into coverage gate + R-B3 GATE A | VERIFIED | `&config.interpreters` at line 349; R-B3 GATE A at lines 358-395 |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | `assign_failure_message` helper + GLE-5 branch + `assign_failure_tests` | VERIFIED | Lines 248-290 (impl); lines 3754-3817 (tests) |
| `.planning/phases/71-engine-agnostic-launch-productionization/71-HUMAN-UAT.md` | SC1 acceptance script + operator-recorded outcomes | VERIFIED | 345-line runbook; operator capture section filled (PASS on langchain-python 2026-06-13) |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `windows_interpreters` field (profile/mod.rs) | policy.json profiles | `serde(default)` deserialization | WIRED | Both aider + langchain-python profiles have the field; `ProfileDeserialize` also carries it (`deny_unknown_fields`); forwarded through `From<ProfileDeserialize>` |
| policy.json profiles | `execution_runtime.rs` | `get_builtin` + field access | WIRED | `windows_resolved_interpreters` at lines 495-529 reads `profile.windows_interpreters` |
| `windows_resolved_interpreters` | `validate_launch_paths` | `ExecConfig.interpreters` + `prepare_live_windows_launch` | WIRED | `interpreters: windows_resolved_interpreters` at line 529; `&config.interpreters` passed at line 349 |
| `path_has_write_owner` (windows.rs) | `nono::lib.rs` | `#[cfg(target_os = "windows")] pub use` | WIRED | Line 87 of lib.rs re-exports it |
| `path_has_write_owner` (nono pub API) | R-B3 GATE A (exec_strategy_windows/mod.rs) | `nono::path_has_write_owner(config.current_dir)` | WIRED | Lines 358-395; gate fires BEFORE `AppliedLabelsGuard::snapshot_and_apply` |
| `--workspace` CLI flag | child `lpCurrentDirectory` | `resolved_workdir` → `ExecutionFlags.workdir` → broker `lpCurrentDirectory` | WIRED | `launch_runtime.rs:313,357` → ExecConfig → broker spawn |
| `--workspace` CLI flag | writable grant | `sandbox_prepare.rs:489-501` auto-grant | WIRED | `FsCapability::new_dir(ws_canonical, AccessMode::ReadWrite)` added unconditionally when workspace is set |
| `assign_failure_message(gle)` | `apply_process_handle_to_containment` | called on `ok == 0` + `GetLastError()` immediately after failed Win32 call | WIRED | Lines 282-288 |

---

### Data-Flow Trace (Level 4)

Not applicable — this phase ships configuration, library primitives, and CLI plumbing, not data-rendering components. The critical data flow (profile field → resolver → interpreter resolution → coverage gate) was verified by tracing the wiring chain above and confirmed by unit tests.

---

### Behavioral Spot-Checks

| Behavior | Evidence | Status |
|----------|----------|--------|
| `validate_launch_paths` refuses uncovered interpreter, names exact path + `--allow` fix | Unit test `validate_launch_paths_refuses_uncovered_interpreter` passes (9/9 tests per SUMMARY-03); live ENG-02-B on Win11 confirmed exact python.exe path named | PASS |
| `path_has_write_owner` returns true for user-owned tempdir, false for System32 (non-elevated) | Tests `workspace_owned_by_current_user_passes_write_owner_check` + `system_dir_lacks_write_owner_for_standard_user` both pass | PASS |
| `assign_failure_message(5)` produces "did not create" + "GLE=5"; generic produces "GLE=1" | Tests `assign_failure_message_gle5_contains_did_not_create` + `assign_failure_message_generic_gle_contains_gle_value` pass (3/3) | PASS |
| `apply_process_handle_to_containment(INVALID_HANDLE_VALUE, ...)` returns `Err`, never `Ok` | Test `apply_process_handle_to_containment_invalid_job_returns_err` pass | PASS |
| `aider` + `langchain-python` profiles load with correct fields from policy.json | `test_get_builtin_aider` + `test_get_builtin_langchain_python` pass (SUMMARY-01 reports 3/3) | PASS |
| SC1 end-to-end confinement on Win11 26200 | Operator-APPROVED 2026-06-13 via langchain-python engine (see 71-HUMAN-UAT.md) | PASS (langchain-python), PENDING (literal aider.exe) |

---

### Probe Execution

No probe scripts declared for Phase 71. The SC1 acceptance is the human-gated proof (71-HUMAN-UAT.md). Step 7c skipped — the runnable proof requires a live Win11 host and is properly classified as manual-only in 71-VALIDATION.md.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| ENG-01 | Plans 71-04, 71-05 | User can run a non-Claude engine confined end-to-end on Win11; inside-write lands, outside denied | SATISFIED (with SC1 note below) | `--workspace` flag, child CWD wiring, writable grant auto-grant, live Win11 UAT PASS (langchain-python); literal Aider deferred |
| ENG-02 | Plans 71-03, 71-04 | Launcher fails secure with actionable message on uncovered interpreter OR non-user-owned workspace | SATISFIED | `validate_launch_paths` 4-arg with named interpreter diagnostic; R-B3 GATE A with named ownership diagnostic; both live-confirmed on Win11 |
| ENG-03 | Plan 71-01 | Per-engine launch profile (interpreter paths, workspace, network identity) through one engine-neutral path | SATISFIED | `windows_interpreters` field; `aider` + `langchain-python` profiles in policy.json; profile resolver unchanged (composition) |

---

### SC1 Substitution Assessment

The ROADMAP SC1 text reads: "A user runs a non-Claude engine (Aider) confined end-to-end on a real Win11 host..."

The REQUIREMENTS.md ENG-01 text reads: "A user can run a non-Claude agent engine (e.g. Aider)..."

The live proof used the `langchain-python` profile (raw python.exe) in an AppContainer on Win11 26200. This proves the identical confinement mechanism — same broker arm (`BrokerLaunchNoPty`), same Low-IL relabel, same AppContainer path, same transitive subprocess confinement. The spike-003 precedent and the operator's APPROVED verdict are the basis for treating this as sufficient.

The gap is narrow: the ROADMAP parenthetical names Aider specifically. The literal aider.exe substitution was deferred because the test host lacked `pip install aider-chat` and an LLM API key. This is surfaced as a human verification item rather than a BLOCKER because:

1. The `langchain-python` proof is mechanically equivalent — the confinement primitive is identical.
2. The operator accepted it explicitly on 2026-06-13.
3. The REQUIREMENTS.md wording uses "e.g." (not "specifically Aider").

A human decision is needed to confirm the substitution is final for milestone sign-off before Phase 72 begins.

---

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `crates/nono-cli/src/exec_strategy_windows/mod.rs` (line 191, 342-349) | `interpreters: Vec::new()` in test helper initializers (`make_minimal_exec_config`, etc.) | Info | Correctly documented as backward-compatible stubs in test helpers; production path (via `execution_runtime.rs:529`) passes the real resolved set. Not a stub in the production code path. |
| `71-VALIDATION.md` frontmatter | `status: draft`, `nyquist_compliant: false`, `wave_0_complete: false` — never updated post-execution | Info | VALIDATION.md is a planning artifact; the actual tests executed per-plan per-SUMMARY are the ground truth. No false PASS claim arises from this. |

No TBD/FIXME/XXX markers in any phase-modified source file. No unresolved debt markers. No placeholder components. No hardcoded-empty data reaching user-visible output.

**Cross-target clippy note (documented PARTIAL from Plans 02/03/04):** `exec_strategy_windows/` files are Windows-cfg-gated; the Windows host cannot cross-compile to Linux/macOS. Cross-target clippy is PARTIAL, deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`. This is an acknowledged methodology gap, not a code defect.

---

### Human Verification Required

#### 1. Literal Aider.exe End-to-End Run (SC1 completion)

**Test:** On a Win11 host with `python -m pip install aider-chat` installed and an LLM API key available: run `nono run --profile aider --workspace %USERPROFILE%\nono-work -- aider.exe --no-git --no-check-update --yes --message "Write 'hello from aider' to result.txt"`. Then run the outside-write and transitive-subprocess tests from 71-HUMAN-UAT.md Steps 2-3.

**Expected:** Inside-workspace write lands (`result.txt` under `$ws`); absolute outside-write (`C:\outside.txt`) denied; transitive python.exe grandchild denied — matching the langchain-python run results. ENG-02-B: if the aider.exe distlib shebang interpreter is not yet in the python_runtime group, nono refuses pre-spawn naming the exact python.exe path (ENG-02 working correctly; operator applies `--allow` and re-runs).

**Why human:** The broker arm requires a real Win11 host, a real Aider install, and an LLM API key that issues actual tool calls. No CI or unit test can exercise this. The langchain-python proof already confirmed the mechanism; this run is the literal-Aider exercise the ROADMAP SC1 parenthetical specifies.

---

### Gaps Summary

No gaps in the implemented code. All five plans delivered substantive, wired, tested code. The single open item is the deferred literal aider.exe live run — a proof-of-completion exercise, not a code gap.

The phase delivered:

- `windows_interpreters` field + `aider`/`langchain-python` profiles (ENG-03)
- `validate_launch_paths` interpreter coverage gate with named D-07 diagnostics (ENG-02)
- `path_has_write_owner` + R-B3 GATE A with named D-08 diagnostic (ENG-02)
- `--workspace` flag wired as child CWD + writable grant single source of truth (ENG-01)
- GLE-5 named foreign-job diagnostic + fail-secure terminate on assign failure (SC5/P6)
- Operator-APPROVED SC1 live proof on Win11 26200 via langchain-python engine

The deferred literal Aider run is a human-verification item, not a code gap.

---

_Verified: 2026-06-14_
_Verifier: Claude (gsd-verifier)_
