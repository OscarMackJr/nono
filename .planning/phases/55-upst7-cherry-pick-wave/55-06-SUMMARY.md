---
phase: 55-upst7-cherry-pick-wave
plan: "06"
subsystem: nono-cli/policy
tags: [upstream-sync, cherry-pick, test-hardening, env-lock, c12]
dependency_graph:
  requires: [55-02]
  provides: [C12-env-lock-test-serialization]
  affects: [crates/nono-cli/src/policy.rs]
tech_stack:
  added: []
  patterns: [ENV_LOCK mutex acquisition for env-mutating tests]
key_files:
  modified: [crates/nono-cli/src/policy.rs]
decisions:
  - "Partial port of upstream 1a764d05: test-only ENV_LOCK hunk applied; session_hooks production hunk deferred to Phase 58"
metrics:
  duration: ~10m
  completed: 2026-06-05T01:22:22Z
---

# Phase 55 Plan 06: POLICY-ENV-LOCK-TEST Summary

**One-liner:** ENV_LOCK acquisition added to `test_all_groups_no_deny_within_allow_overlap` to serialize parallel env-var races; session_hooks production hunk deferred to Phase 58.

## Tasks Completed

| # | Name | Commit | Files |
|---|------|--------|-------|
| 1 | C12 cherry-pick (test-only ENV_LOCK) | f4b396b1 | crates/nono-cli/src/policy.rs |

## C12 Cherry-pick Log

**Upstream commit:** 1a764d05 (`test: lock ENV_LOCK in test_all_groups_no_deny_within_allow_overlap`)
**Upstream tag:** v0.58.0
**Upstream author:** Caio Silva <caio@cdcs.dev>
**Fork commit:** f4b396b1

### Trailer Verification

```
git log --format="%B" HEAD~1..HEAD | grep -c "^Upstream-commit:"
```
Result: **1** (PASS)

### Diff Summary (test-only)

The diff adds 7 lines to the `#[cfg(test)]` module only:

```rust
        //
        // We hold ENV_LOCK because expand_path() reads HOME/TMPDIR from the
        // process env, and other tests in the suite mutate those vars.
        let _guard = match crate::test_env::ENV_LOCK.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
```

Inserted at the beginning of `test_all_groups_no_deny_within_allow_overlap`, immediately before `let policy = load_embedded_policy()`.

## Verification Results

### D-55-E1 (Windows files check): PASS

```
git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"
```
Result: 0 lines (no Windows files touched).

### D-55-E3 (Cross-target clippy): N/A

`policy.rs` contains `#[cfg(target_os = ...)]` attributes in the test module only (all in the 2200+ line range). The change is confined to the `#[cfg(test)]` module and introduces no new cfg-gated Unix-only code blocks. Cross-target clippy is not required per CLAUDE.md rules (the change is test-only and platform-agnostic).

### D-55-E4 (Baseline-aware CI gate): PASS

```
cargo test -p nono-cli -- policy::tests
```
Result: **80 passed; 0 failed** (exit 0)

Lane transition: carry-forward (no new failures introduced; test_all_groups_no_deny_within_allow_overlap passes with ENV_LOCK).

### C7 Production-Code Preservation: PASS

`binary: None` (from upstream 9398a13, C7 cherry-pick in 55-02) is present and unchanged at policy.rs line 198.

## Deviations from Plan

### Partial Port — D-55-06-PARTIAL

**Upstream commit 1a764d05 contains two changes mixed together:**

1. **Test-only** (ported): `ENV_LOCK` acquisition in `test_all_groups_no_deny_within_allow_overlap`
2. **Production code** (deferred): `session_hooks: profile::SessionHooks::default()` added to `ProfileDef::to_raw_profile()`

**Why:** `profile::SessionHooks` does not exist in the fork. It is introduced by the Phase 58 session-hooks plan (upstream v0.58.0 session lifecycle hooks). Applying the `session_hooks` field would cause a compilation error since the type is undefined.

**Action taken:** Applied only the test-only hunk manually (git cherry-pick --no-commit, then git reset/checkout, then Edit tool for test-only hunk). The commit message documents the deferral clearly.

**Impact:** The test serialization (the plan's stated goal) is fully achieved. The `session_hooks` production field will be absorbed as part of Phase 58 work.

## REQ-DENY-PREFLIGHT-01 Non-Resolution Note

This C12 cherry-pick does **NOT** resolve REQ-DENY-PREFLIGHT-01 (the deferred Linux-host `validate_deny_overlaps` investigation in REQUIREMENTS.md § v2 Deferred). The `test_all_groups_no_deny_within_allow_overlap` test now acquires ENV_LOCK to prevent parallel HOME-mutation races — this only hardens the test against flakiness, it does not address the substantive REQ-DENY-PREFLIGHT-01 question about whether the deny-overlap check itself has correctness gaps.

## Self-Check: PASSED

| Item | Status |
|------|--------|
| crates/nono-cli/src/policy.rs exists | FOUND |
| 55-06-SUMMARY.md exists | FOUND |
| Commit f4b396b1 exists | FOUND |
| policy tests 80/80 pass | PASS |
| D-55-E1 (no Windows files) | PASS |
| D-55-E3 (cross-target clippy) | N/A |
| C7 binary: None preserved | PASS |
| Upstream-commit trailer count = 1 | PASS |
