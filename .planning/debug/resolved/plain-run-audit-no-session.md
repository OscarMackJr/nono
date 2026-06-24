---
slug: plain-run-audit-no-session
status: resolved
trigger: "Plain `nono run` (audit default-on) creates no audit session at the XDG state root; rollback/--audit-integrity runs DO. Surfaced in PR #12 Integration Tests (macOS) Audit Trail suite."
created: 2026-06-24
updated: 2026-06-24
---

# Debug Session: plain `nono run` creates no audit session (XDG state root)

## Symptoms

**Expected:** A plain `nono run -- <cmd>` (no `--no-audit`) creates an audit session at the audit root. Audit is default-ON; `launch_runtime.rs:294,405` gate it OFF only via `--no-audit` (`audit_disabled: no_audit`).

**Actual (PR #12 CI, `Integration Tests` macOS job, "Audit Trail" suite, `tests/integration/test_audit.sh`):**
```
Audit root: /Users/runner/.local/state/nono/audit
--- Direct Execution (No Audit Session) ---
FAIL: plain run creates audit session        Expected session for PID 26957: not found
FAIL: read-only run creates audit session    Expected session for PID 26964: not found
FAIL: non-zero exit creates audit session    Expected session for PID 26971: not found
... (rollback audit tests PASS — sessions ARE created for --rollback runs)
FAIL: audit list shows sessions   (cascade: list shows the 3 rollback/flagged sessions, not the 3 plain-run ones)
Suite FAILED: Audit Trail  (Tests run 11, Passed 7, Failed 4)
```
So: **plain runs create NO audit session; runs WITH `--rollback`/`--audit-integrity` DO** (those pass and appear in `audit list`).

**Likely v3.1 regression (Phase 88 XDG migration):** v3.x changed `test_audit.sh` ONLY to migrate the audit root path `$HOME/.nono/audit` → `${XDG_STATE_HOME:-$HOME/.local/state}/nono/audit` (correct, matches Phase 88 FEAT-02). Hypothesis: the production plain-run audit path did NOT fully migrate to the XDG state dir (or plain-run audit-session creation regressed), while the rollback path did — so plain-run sessions land nowhere (or at the old `$HOME/.nono/audit`) and the test (now looking at the XDG root) can't find them.

## Classification
- Integration Tests JOB is a pre-existing-red baseline (its `Pack Resolution` suite fails because `tests/integration/test_pack_resolution.sh` is MISSING on disk AND on origin/main — the job is red on main regardless). Shell suite = headless-CI PTY `setsid()` env issue. Profiles = `opencode` profile/test mismatch (also on main).
- BUT the **Audit Trail** failure is the concern: v3.x touched `test_audit.sh` + Phase 88 moved state to XDG. This sub-suite may be a REAL regression, not baseline. **That is what this session must determine.**

## Key code locations / leads
- Default-audit gate: `crates/nono-cli/src/launch_runtime.rs:294` (`no_audit = run_args.no_audit`), `:405` (`audit_disabled: no_audit`). Audit is default-ON.
- Audit session creation: `crates/nono-cli/src/audit_session.rs` (`Failed to create audit session directory` at :50; SessionMetadata builders ~396-545).
- Audit ROOT resolution (XDG state dir) — find where the audit root path is computed for the PLAIN-run path vs the ROLLBACK path. The bug is likely a divergence: rollback path uses the XDG state dir; plain path uses a different/legacy root (or skips session creation). Compare to Phase 88 FEAT-02 XDG state migration (`config/` or `state_paths.rs`).
- Test: `tests/integration/test_audit.sh` — `AUDIT_ROOT="${XDG_STATE_HOME:-$HOME/.local/state}/nono/audit"`; a `find_session_for_pid` helper scans AUDIT_ROOT.

## Decisive questions
1. Does a plain `nono run` (no flags) actually create an audit session in current code, and WHERE (which root)? Compare the audit-root path used on the plain-run path vs the rollback path.
2. Is the divergence a Phase 88 (v3.1) regression (plain path left on legacy `$HOME/.nono/audit` while rollback/test moved to XDG), or did plain-run audit-session creation get gated/removed?
3. Is this reproducible at the unit level (a Rust test that a plain confined run writes session.json under the XDG state audit root), so the fix is locally verifiable rather than CI-only?

## Constraints / verification
- Host is WINDOWS; the `test_audit.sh` integration suite is a Unix shell harness (won't run locally). BUT this may be reproducible via a Rust unit/integration test or by reading the path-resolution code. Prefer a locally-verifiable fix (unit test) over CI-only.
- CI verifier: `Integration Tests` (macOS) on PR #12 — but note the job stays red regardless until the pre-existing `test_pack_resolution.sh`/Shell/Profiles issues are also addressed; the Audit Trail suite passing is the target signal for THIS session (grep the job log for "Suite PASSED: Audit Trail").
- SECURITY-CRITICAL: audit trail integrity. Do NOT disable/weaken default audit to make the test pass. If plain-run audit is genuinely supposed to be created, fix the path/creation; if the test expectation is wrong (audit became opt-in by design), prove it from code/decisions and update the test.
- DCO: `Oscar Mack Jr <oscar.mack.jr@gmail.com>`. Branch `milestone/v2.13-carryforward-closeout` (PR #12). Merge is ON HOLD pending this.

## Current Focus

reasoning_checkpoint:
  hypothesis: "Plain `nono run` creates NO audit session because `create_audit_state` (crates/nono-cli/src/rollback_runtime.rs:184-186) early-returns `Ok(None)` whenever `!rollback_active && !audit_integrity_requested`. A plain run has rollback_requested=false, audit_integrity_requested=false, audit_disabled=false → falls into that gate → no session dir, no session.json. This is NOT a path divergence: both plain and rollback paths resolve the audit root via state_paths::audit_root() = $XDG_STATE_HOME/nono/audit. It is a GATING bug — a fork minimal-scope divergence (commit 87108a37 `feat(22-05a): minimal AuditRecorder lifecycle integration`, 'Plan 22-05a Decision 5 minimal scope') that never ungated default audit for plain supervised runs."
  confirming_evidence:
    - "rollback_runtime.rs:174-186 — `if audit_disabled { return Ok(None) }` then `if !rollback_active && !audit_integrity_requested { return Ok(None) }`. Plain run hits the second return."
    - "audit_session.rs:37-39 + state_paths.rs:79-81 — audit_root() ALWAYS resolves to user_state_dir()/audit = $XDG_STATE_HOME/nono/audit (Win: %LOCALAPPDATA%/nono/audit). No legacy/plain divergence in the WRITE path. The Phase 88 XDG migration (commit 0a09ff41) is complete on the production side; the divergence is purely the create-gate."
    - "rollback_runtime.rs:673-708 (finalize, audit-only path) ALREADY writes session.json via write_session_metadata to audit_state.session_dir with snapshot_count:0 / empty merkle_roots / audit_event_count:0 when audit_state is Some and no recorder. So if create_audit_state returned Some for a plain run, a valid session.json (session_id, command, started, ended, exit_code) WOULD be written — no other gate blocks it."
    - "tests/integration/test_audit.sh:4-6 header + Test 1/2/3 (lines 66-103) EXPECT plain runs to CREATE an audit session ('Audit is on by default for all supervised sessions (#269). Plain `nono run` creates an audit session; --no-audit opts out'). Test 4 (line 114) expects --no-audit to NOT create one. The XDG commit 0a09ff41 changed ONLY the AUDIT_ROOT path (6 lines); the plain-run create-assertions predate it → the test has long expected default audit, the fork production never matched."
    - "git: gating block introduced by 87108a37 whose own message says it was a SCOPE DEVIATION (Decision 5/Option A) deferring full upstream 4f9552ec restructuring; create_audit_state was minimally wired to audit-integrity/rollback only."
  falsification_test: "If, after removing the `!rollback_active && !audit_integrity_requested` gate (keeping only the `audit_disabled` gate), a plain `nono run` did NOT produce $XDG_STATE_HOME/nono/audit/<id>/session.json, the hypothesis is wrong (some other gate exists). A Rust unit test on create_audit_state asserting Some for the plain-flags case + finalize writing session.json under audit_root() proves/disproves this locally on Windows."
  fix_rationale: "Remove the audit-integrity/rollback gate from create_audit_state so the function creates an audit session for ANY run that is not --no-audit (the only legitimate opt-out, matching the test + #269 default-on design + the deny gate already at line 174). The rollback_dir + attestation bundle remain conditional (rollback_active / signer). This ungates plain-run session creation WITHOUT weakening any security control — it ADDS audit coverage that was missing. Downstream finalize already writes the session.json correctly for the audit_state-Some/no-recorder case."
  blind_spots: "1) `nono wrap` (Direct strategy, test 9 line 220) must STILL NOT create a session — verify create_audit_state is only reached on the supervised path, not the Direct/wrap path. 2) Whether any caller relies on create_audit_state returning None for plain runs (e.g. perf: skip snapshot baseline). 3) Windows AppContainer/broker supervised path — confirm it routes through supervised_runtime::create_audit_state too. 4) Need to confirm the audit-only finalize branch is actually reached for a plain run (i.e. finalize_supervised_exit runs even with no recorder + no rollback)."

confirmed_root_cause: "create_audit_state gates audit-session creation behind --rollback OR --audit-integrity; plain `nono run` (default-on audit, no --no-audit) gets Ok(None) and writes no session.json. Fork minimal-scope divergence (87108a37), not a Phase 88 path-migration regression. Production fix = ungate (audit unless --no-audit)."
next_action: "EMIT CHECKPOINT (fix_approval) — do NOT edit production code until acknowledged. Proposed fix: delete the `if !rollback_active && !audit_integrity_requested { return Ok(None); }` block in rollback_runtime.rs (~184-186); keep `if audit_disabled { return Ok(None); }`. Add a Windows-runnable Rust unit test on create_audit_state (plain flags → Some) + confirm wrap/Direct path is untouched."

## ORIGINAL Hypothesis (revised — was path-divergence; actual is gating)

- hypothesis (ELIMINATED): Phase 88 XDG migration left the plain-run audit path on legacy `~/.nono/audit`. FALSE — audit_root() is single-sourced via state_paths and identical for all write paths. The divergence is a CREATE-TIME GATE, not a path.

## Evidence

- timestamp: 2026-06-24 — PR #12 `Integration Tests` (macOS) job 83229302044: 4 suites failed (Shell, Profiles, Pack Resolution, Audit Trail). Pack Resolution = missing `test_pack_resolution.sh` (also absent on origin/main) → job is pre-existing-red. Audit Trail = plain-run audit sessions not found at XDG root; rollback audit sessions found.
- timestamp: 2026-06-24 — `git diff origin/main 5370d7ae -- tests/integration/test_audit.sh`: the ONLY v3.x change was AUDIT_ROOT/ROLLBACK_ROOT `$HOME/.nono/...` → `${XDG_STATE_HOME:-$HOME/.local/state}/nono/...` (Phase 88 XDG migration). Plain-run "creates audit session" assertions themselves were NOT changed by v3.x.
- timestamp: 2026-06-24 — code: audit default-ON, gated off only by `--no-audit` (launch_runtime.rs:294/405). So plain runs are expected to audit.

## Eliminated

(none yet)
