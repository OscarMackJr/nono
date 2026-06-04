---
phase: 55-upst7-cherry-pick-wave
plan: 06
type: execute
wave: 4
depends_on: [55-02]
files_modified:
  - crates/nono-cli/src/policy.rs
autonomous: true
requirements: [REQ-UPST7-02]
must_haves:
  truths:
    - "test_all_groups_no_deny_within_allow_overlap acquires ENV_LOCK before calling expand_path()"
    - "The test no longer races against sibling tests that mutate HOME (notably the new hook_runtime tests)"
    - "The commit carries a verbatim 6-line D-19 trailer + DCO Signed-off-by"
    - "policy.rs test module is the only change — no production code is modified"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
    - "C7's policy.rs production-code change (1-line target_binary addition from 9398a13) is preserved intact"
  artifacts:
    - path: "crates/nono-cli/src/policy.rs"
      provides: "ENV_LOCK acquisition in test_all_groups_no_deny_within_allow_overlap (test-only)"
      min_lines: 0
  key_links:
    - from: "policy.rs test"
      to: "ENV_LOCK mutex"
      via: "ENV_LOCK.lock() before expand_path()"
      pattern: "ENV_LOCK"
---

<objective>
Cherry-pick cluster C12 — policy ENV_LOCK test serialization (1 upstream commit from v0.57.0..v0.59.0).

C12 is parallel with C11 (plan 55-05) in Wave 4 — no file overlap (C11 touches cli.rs/exec_strategy.rs/timeouts.rs etc.; C12 touches only policy.rs). Both depend on C7 (55-02, Wave 2) which also touched policy.rs.

This is the simplest cluster: one test-only commit adding ENV_LOCK acquisition to prevent race conditions with HOME mutation tests. No production code change. No new dependencies. Cross-target clippy not required (test-only code in policy.rs has no cfg-gated Unix blocks — verify).

Purpose: Absorb the ENV_LOCK test serialization hardening.
Output: 1 new commit on the held feature branch with D-19 trailer.
</objective>

<execution_context>
@C:\Users\OMack\.claude\get-shit-done\workflows\execute-plan.md
@C:\Users\OMack\.claude\get-shit-done\templates\summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/STATE.md
@.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md
@.planning/templates/upstream-sync-quick.md
@.planning/phases/55-upst7-cherry-pick-wave/55-CONTEXT.md

<interfaces>
<!-- C12 commits -->
C12 commits:
  1. 1a764d05 — test: lock ENV_LOCK in test_all_groups_no_deny_within_allow_overlap  (v0.58.0)
     Files: crates/nono-cli/src/policy.rs (test-only — #[cfg(test)] module only)

Upstream-tag: v0.58.0. windows-touch: no. Cross-cluster re-export check: clean (test-only).

Context from ledger: "test-only hardening of policy.rs (ENV_LOCK serialization, aligns with the fork's EnvVarGuard discipline) → Phase 55"

Fork's ENV_LOCK discipline: CLAUDE.md § "Environment variables in tests" — tests modifying HOME/TMPDIR/XDG_CONFIG_HOME MUST save+restore and keep the modified window short. This commit aligns with that policy.

C7 already touched policy.rs (1-line production-code addition in 9398a13). C12 must land on top of C7's production-code change. The C12 change is in the test module only — no conflict with C7's production-code line is expected.

REQ-DENY-PREFLIGHT-01 context (from REQUIREMENTS.md v2 deferred): this ENV_LOCK fix targets the same test (`test_all_groups_no_deny_within_allow_overlap`) that the deferred `validate_deny_overlaps` investigation mentions. The C12 cherry-pick does NOT resolve REQ-DENY-PREFLIGHT-01 (that is a deferred Linux-host investigation); it only hardens the test against HOME-mutation races.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: C12 cherry-pick (1 commit)</name>
  <files>crates/nono-cli/src/policy.rs</files>
  <read_first>
    - crates/nono-cli/src/policy.rs (full post-C7-cherry-pick state — read the test module at the bottom to understand current ENV_LOCK usage and the test that is being hardened)
    - .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md § Cluster C12
    - .planning/templates/upstream-sync-quick.md (D-19 trailer)
    - git show 1a764d05 (full diff — verify it is test-only and adds ENV_LOCK.lock())
    - CLAUDE.md § "Environment variables in tests" (env-var save/restore discipline)
  </read_first>
  <action>
Cherry-pick the 1 C12 commit:
  1a764d05 — test: lock ENV_LOCK in test_all_groups_no_deny_within_allow_overlap

a. `git cherry-pick 1a764d05` from the upstream remote.
b. Resolve conflicts (if any):
   - policy.rs: C7 added exactly 1 production-code line (target_binary field registration). C12 modifies the test module. These are in different sections of policy.rs — conflicts should not occur. If they do, the resolution is: preserve C7's production-code addition AND apply C12's test modification independently.
   - Verify: `git diff --stat HEAD~1 HEAD` must show ONLY policy.rs with test-only additions.
c. Amend the commit to append the verbatim D-19 6-line trailer block (D-55-E2). Upstream-tag is v0.58.0.

D-55-E1: verify `git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines.

D-55-E3 cross-target clippy: check if policy.rs contains cfg-gated Unix code blocks (`grep -n "cfg(target_os" crates/nono-cli/src/policy.rs`). If no cfg-gated blocks: cross-target clippy is NOT required for this cluster — document as N/A. If cfg-gated blocks are found: run cross-target clippy per the checklist.

D-55-E4: run `cargo test -p nono-cli -- policy::tests` to confirm the specific test passes. Record lane-transition category.

Post-cherry-pick note: record in SUMMARY that this fix does NOT resolve REQ-DENY-PREFLIGHT-01 (the deferred Linux-host validate_deny_overlaps investigation in REQUIREMENTS.md § v2 Deferred) — it only hardens the test against parallel race.
  </action>
  <verify>
    <automated>
git log --format="%B" HEAD~1..HEAD | grep -c "^Upstream-commit:"
    </automated>
    Must equal 1. Also verify:
      git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      git diff HEAD~1 HEAD -- crates/nono-cli/src/policy.rs | grep -v "^#\|^@@\|^---\|^+++" | grep -c "ENV_LOCK" (must be >= 1)
      git diff HEAD~1 HEAD -- crates/nono-cli/src/policy.rs | grep -v "^#\|^@@\|^---\|^+++" | grep -c "fn test_all_groups_no_deny_within_allow_overlap\|pub fn\|^+pub\|^+fn [a-z]" (production-code additions must be 0 — only test code changed)
      cargo test -p nono-cli -- policy::tests (exit 0 or documented carry-forward)
  </verify>
  <acceptance_criteria>
    - Exactly 1 cherry-pick commit (1a764d05) with D-19 trailer; `git log --format="%B" HEAD~1..HEAD | grep -c "^Upstream-commit:"` equals 1
    - `git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines (D-55-E1 PASS)
    - policy.rs diff is test-only: `git diff HEAD~1 HEAD -- crates/nono-cli/src/policy.rs` contains changes ONLY in the `#[cfg(test)]` module
    - `git diff HEAD~1 HEAD -- crates/nono-cli/src/policy.rs` contains ENV_LOCK
    - C7's 1-line production-code addition (target_binary from 9398a13) is present and unchanged in policy.rs
    - `cargo test -p nono-cli -- policy::tests` exits 0
    - Cross-target clippy: N/A (no cfg-gated Unix blocks in policy.rs) or PARTIAL-documented if blocks found
    - Baseline-aware CI gate: lane-transition category recorded (D-55-E4)
    - SUMMARY notes: this C12 fix does NOT resolve REQ-DENY-PREFLIGHT-01
    - Feature branch NOT merged to main (D-55-03)
  </acceptance_criteria>
  <done>C12 ENV_LOCK test serialization cherry-picked; policy test is race-hardened; production code unchanged.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| parallel test runner → ENV_LOCK | test threads competing for HOME env var; ENV_LOCK serializes access |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-55-06-01 | Tampering | test-only change sneaking production-code edits | mitigate | Acceptance criterion explicitly verifies the diff is confined to the #[cfg(test)] module; no production-code line additions allowed |
| T-55-06-02 | Information Disclosure | ENV_LOCK deadlock risk | accept | ENV_LOCK is a standard test mutex (MutexGuard RAII, dropped at test end); no deadlock risk; test-only, no production exposure |
| T-55-06-SC | Tampering | No new cargo deps | accept | C12 adds no new dependencies; no package legitimacy gate needed |
</threat_model>

<verification>
1. `git log --format="%B" HEAD~1..HEAD | grep -c "^Upstream-commit:"` equals 1
2. `git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines
3. `git diff HEAD~1 HEAD -- crates/nono-cli/src/policy.rs` is in the #[cfg(test)] module only
4. `cargo test -p nono-cli -- policy::tests` exits 0
5. Feature branch NOT merged to main
</verification>

<success_criteria>
- 1 C12 cherry-pick commit on the held feature branch with D-19 trailer
- policy.rs test is ENV_LOCK serialized; production code unchanged
- C7's 1-line target_binary addition preserved in policy.rs
- D-55-E1 PASS
</success_criteria>

<output>
Create `.planning/phases/55-upst7-cherry-pick-wave/55-06-SUMMARY.md` when done.
Include: C12 cherry-pick log (1 commit, SHA, trailer verification); confirmation that diff is test-only; REQ-DENY-PREFLIGHT-01 non-resolution note; cross-target clippy status (N/A or PARTIAL); D-55-E1 PASS; baseline-aware CI gate result; held-branch status.
</output>
