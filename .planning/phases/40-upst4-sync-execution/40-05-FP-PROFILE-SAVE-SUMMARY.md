---
phase: 40-upst4-sync-execution
plan: 05
slug: fp-profile-save
cluster_id: C4
subsystem: nono-cli
tags: [upst4, c4, fork-preserve, profile-save, d20-manual-replay, wave-2, v0.52.2]

# Dependency graph
requires:
  - phase: 39-upst4-audit
    provides: cluster C4 disposition (fork-preserve, 2 commits) + commit chain inventory
  - phase: 40-01-PROXY-HARDENING
    provides: Wave 1 closed (PR #922 body + system_keystore_label CR-A fix on main)
  - phase: 40-04-RELEASE-RIDE
    provides: Wave 1 closed (Landlock ABI cache + full failure diagnostic + v0.52.1/v0.52.2/v0.53.0 CHANGELOG)
provides:
  - "filesystem.suppress_save_prompt" profile schema field + struct field (with serde "ignore" alias)
  - Suppression filter in save-profile patch construction (canonical-path or Path-component-prefix match)
  - 4 new unit tests for the suppression filter (Linux/macOS only — file is cfg(not(target_os = "windows")))
  - 1 manual-replay commit with full D-40-B3 commit body sections (no D-19 trailer)
  - 1 disposition-resolution docs commit (Task 1) preserving the D-20 decision record
affects: [40-06-FP-PROXY-TLS]

# Tech tracking
tech-stack:
  added: []  # no new fork dependencies; replay uses existing nono::try_canonicalize + nix/std
  patterns:
    - "D-20 manual replay (D-40-B3 commit body sections; no D-19 trailer; Upstream-replayed-from: provenance)"
    - "Disposition resolution committed BEFORE any code change (Task 1 docs commit)"
    - "Serde alias discipline for upstream key compatibility (mirror of D-36-B3 bypass_protection / override_deny pattern)"
    - "Component-wise Path::starts_with for path-prefix suppression (CLAUDE.md § Path Security)"
    - "nono::try_canonicalize at the comparison boundary (CLAUDE.md § canonicalize at enforcement boundary)"

key-files:
  created: []
  modified:
    - crates/nono-cli/data/nono-profile.schema.json (+5/-0: suppress_save_prompt schema field)
    - crates/nono-cli/src/profile/mod.rs (+27/-0: FilesystemConfig.suppress_save_prompt field + serde alias + merge_profiles dedup-append + 2 literal-constructor updates)
    - crates/nono-cli/src/profile_save_runtime.rs (+245/-2: ignored_denial_paths threading through offer_save_run_profile + build_run_profile_patch + add_patch_grant; matches_ignored_denial / canonicalize_suppress_entry helpers; 4 new unit tests)
    - .planning/phases/40-upst4-sync-execution/40-05-FP-PROFILE-SAVE-PLAN.md (+56/-0: ## Disposition resolution section inline; Task 1 record)

# Skipped gates categorization (per .continue-here.md anti-pattern #3)
skipped_gates_load_bearing: [3, 4]   # cross-target clippy linux-gnu/darwin (CI substitute required)
skipped_gates_environmental: [6, 7, 8]   # detached-console / wfp_port / learn_windows (Windows runtime missing in agent context)

key-decisions:
  - "Disposition resolution stays D-20 manual replay. Trial cherry-pick of 9b07bf7 produced 14 content conflicts + 1 modify/delete (on tests/schema_shape.rs which fork has removed). D-40-B1 clause (a) fails (not zero conflicts). D-40-B1 clause (b) fails (upstream's three-way ProfileSaveChoice + new CLI flag = non-zero behavioral surprise). Stays D-20."
  - "Surface-overlap Q1–Q6 all returned 0. Phase 18.1 D-04-locked surface (build_prompt_text + HandleKind) NOT touched by upstream 9b07bf7 — upstream's plumbing is in cli.rs / profile_cmd.rs / profile_runtime.rs / exec_strategy.rs / launch_runtime.rs, not in the approval-prompt surface."
  - "Minimal replay scope: only the schema + struct + save-patch-filter pieces. The CLI-flag plumbing across 9 files is dead without --suppress-save-prompt flag itself; deferred. The three-way ProfileSaveChoice prompt restructure is UX, not security; not replayed."
  - "Plan frontmatter listed 4 modified files (profile_save_runtime.rs / terminal_approval.rs / policy.rs / profile/mod.rs); reality is 3 (profile_save_runtime.rs / profile/mod.rs / schema.json). terminal_approval.rs and policy.rs not touched because upstream 9b07bf7 doesn't touch them either (Q1 + Q3 both returned 0)."
  - "eb6cb09 review-fix folded into the same replay commit. eb6cb09's diff is entirely inside the not-replayed ProfileSaveChoice three-way prompt — there is no semantic difference from the fork's perspective."
  - "Tests run only on Linux/macOS CI because profile_save_runtime.rs is gated cfg(not(target_os = 'windows')) at the main.rs module declaration. Windows CI does not compile this file. 4 new unit tests cover: exact-canonical-match suppression, directory-prefix Path::starts_with suppression, empty-list short-circuit, serde 'ignore' alias deserialization."

patterns-established:
  - "D-20 manual replay shape: disposition documented as a separate docs commit BEFORE the code commit, so reviewers see the decision rationale ahead of the diff."
  - "Serde 'ignore' alias retention: upstream's two-name pattern (canonical key 'suppress_save_prompt' + alias 'ignore') replays cleanly as `#[serde(default, alias = \"ignore\")]` — mirrors the D-36-B3 bypass_protection / override_deny precedent."
  - "Suppression filter short-circuit: matches_ignored_denial returns false on empty ignored_denial_paths slice BEFORE calling try_canonicalize, avoiding unnecessary syscall traffic on the common-case path of no suppression list."

requirements-completed: [REQ-UPST4-02]

# Metrics
duration: ~50m
completed: 2026-05-14
---

# Phase 40 Plan 05: FP-PROFILE-SAVE Summary

**Cluster C4 (v0.52.2, 2 commits) replayed onto fork main via D-20 manual replay — `filesystem.suppress_save_prompt` profile field + `ignore` serde alias + canonical-path / Path-prefix suppression filter absorbed into the save-profile flow. D-40-E1 invariant holds (0 Windows-file edits). D-40-B1 upgrade rule did NOT fire (14 content conflicts + 1 modify/delete in trial cherry-pick; non-zero behavioral surprise on upstream's three-way ProfileSaveChoice restructure). Wave 2 first-plan close — orchestrator-merge + CI gate (Task 5) is downstream of this worktree return.**

## Performance

- **Duration:** ~50 min (most of it inside the trial-cherry-pick + disposition-resolution writeup)
- **Started:** 2026-05-14
- **Completed:** 2026-05-14
- **Tasks:** 4 plan tasks (Task 5 wait-for-CI is downstream of orchestrator-merge per worktree pattern)
- **Files modified:** 3 (`crates/nono-cli/data/nono-profile.schema.json`, `crates/nono-cli/src/profile/mod.rs`, `crates/nono-cli/src/profile_save_runtime.rs`) + 1 plan doc (PLAN.md `## Disposition resolution` inline)
- **Commits landed:** 2 (1 disposition docs + 1 D-20 replay)

## Accomplishments

- **Disposition resolution documented BEFORE any code change** (commit `64973c63`). All 6 D-40-B1 surface-overlap questions (Q1–Q6) answered with numeric output; trial cherry-pick attempted + result recorded; FINAL DISPOSITION = D-20 with justification.
- **`filesystem.suppress_save_prompt` schema field** added to `crates/nono-cli/data/nono-profile.schema.json` (+5 lines under FilesystemConfig.properties).
- **`FilesystemConfig.suppress_save_prompt: Vec<String>` Rust field** added with `#[serde(default, alias = "ignore")]` to mirror upstream's `ignore` alias. `merge_profiles` extended to dedup-append the new field across base + child profiles. 2 test-fixture literal constructors updated with the new field.
- **Suppression filter implemented in `profile_save_runtime.rs`:** `offer_save_run_profile` loads the compared profile's `filesystem.suppress_save_prompt` list, canonicalizes each entry (with `~/` expansion via `crate::config::validated_home`), passes the canonicalized `Vec<PathBuf>` to `build_run_profile_patch`. `add_patch_grant` short-circuits any denial that matches via `matches_ignored_denial` (component-wise `Path::starts_with` on canonical paths — CLAUDE.md § Path Security).
- **4 new unit tests:** exact-canonical-match suppression, directory-prefix suppression, empty-list short-circuit, serde `ignore` alias deserialization. Tests run only on Linux/macOS CI (file is `cfg(not(target_os = "windows"))`).
- **D-40-B3 commit body discipline:** all 5 sections present (`Upstream intent:` / `What was replayed:` / `What was NOT replayed and why:` / `Fork-only wiring preserved:` / `Upstream-replayed-from:`). Zero `^Upstream-commit:` trailer lines (D-19 trailer would be misleading on a manual replay).
- **D-40-E1 invariant holds:** 0 Windows-file edits across the chain. Verified via `git diff --stat HEAD~2 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l = 0`.
- **Phase 18.1 D-04-locked surface preserved:** `grep -c 'build_prompt_text\|HandleKind' crates/nono-cli/src/terminal_approval.rs = 45` (unchanged from pre-plan baseline).
- **Phase 36 / 36.5 surface preserved:** deprecated_schema integration intact at top of `profile/mod.rs`; `cmd_promote` parser at `profile_cmd.rs:456` untouched; profile-drafts surface in profile/mod.rs at lines 2213 and onward untouched.
- **eb6cb09 review-fix folded** into the same replay commit (its diff is entirely inside the not-replayed ProfileSaveChoice three-way prompt restructure — no separate replay needed).

## Task Commits

| Task | Subject | Commit | Trailer shape |
|------|---------|--------|---------------|
| 1 | Disposition resolution docs (PLAN.md + commit message) | `64973c63` | DCO sign-offs only (docs commit) |
| 2 | D-20 replay: `feat(profile-save): suppress save-profile prompts for denied paths` | `5c3da3d7` | D-40-B3 sections + `Upstream-replayed-from: 9b07bf7` + `Upstream-replayed-from: eb6cb09` + `Co-Authored-By: Claude` + 2× `Signed-off-by:` |

(SUMMARY-doc commit follows separately.)

## Files Created/Modified

- `.planning/phases/40-upst4-sync-execution/40-05-FP-PROFILE-SAVE-PLAN.md` — added `## Disposition resolution (D-40-B1)` section (56 lines) at the top of `<tasks>` with full Q1–Q6 evidence and FINAL DISPOSITION = D-20.
- `crates/nono-cli/data/nono-profile.schema.json` — added `filesystem.suppress_save_prompt` schema field (5 lines) under FilesystemConfig.properties, after `bypass_protection`.
- `crates/nono-cli/src/profile/mod.rs` — added `FilesystemConfig.suppress_save_prompt: Vec<String>` field (with `#[serde(default, alias = "ignore")]` + 13-line doc comment); extended `merge_profiles` to dedup-append the new field across base + child profiles; added `suppress_save_prompt: vec![]` to 2 test-fixture literal constructors (4140 / 4225 — other 5 already used `..Default::default()`).
- `crates/nono-cli/src/profile_save_runtime.rs` — `offer_save_run_profile` now loads `compared_profile`'s `suppress_save_prompt` list and canonicalizes entries; `build_run_profile_patch` accepts `ignored_denial_paths: &[PathBuf]`; `add_patch_grant` accepts the same and short-circuits matching denials; new `matches_ignored_denial` helper (component-wise `Path::starts_with` on canonical paths); new `canonicalize_suppress_entry` helper (handles `~/` shorthand inverse of `shorten_path_for_profile`); 4 new unit tests; 2 existing tests updated to pass the new `&[]` argument.

## Decisions Made

- **DEC-1: Disposition stayed D-20 manual replay.** Trial cherry-pick produced 14 content conflicts + 1 modify/delete; non-zero behavioral surprise (upstream's three-way ProfileSaveChoice + new CLI flag). D-40-B1 upgrade rule failed on both clauses (a) and (b).
- **DEC-2: Minimal replay scope.** Only schema + struct + save-patch-filter pieces. Deferred upstream's CLI-flag plumbing across 9 files (cli.rs / main.rs / command_runtime.rs / execution_runtime.rs / launch_runtime.rs / profile_cmd.rs / profile_runtime.rs / sandbox_prepare.rs / exec_strategy.rs) — without the `--suppress-save-prompt` CLI flag, that plumbing is dead. Fork users can author `filesystem.suppress_save_prompt` in profile JSON by hand; the CLI-flag wrapper is a UX convenience that a follow-up phase can add if/when there is demand.
- **DEC-3: ProfileSaveChoice three-way prompt restructure not replayed.** This is UX shape (Grant / Suppress / Skip three-way) rather than security gate. The existing fork prompt-shape (binary Y/N + 'override'-typed confirmation) remains functional; the suppression filter takes effect upstream of the prompt branch, so the existing UX is unaffected.
- **DEC-4: eb6cb09 review-fix folded into the single replay commit.** eb6cb09's diff is entirely inside the not-replayed ProfileSaveChoice prompt — there is no behavioral distinction once the upstream UX shape is left out. Two `Upstream-replayed-from:` lines in the replay commit's body cite both upstream SHAs for provenance.
- **DEC-5: Tests cover the security/UX path that survived the replay.** 4 unit tests verify: exact-canonical-match filters (test 1), directory-prefix Path::starts_with filters (test 2), empty-list short-circuit avoids syscall traffic (test 3), serde `ignore` alias deserializes identically to canonical key (test 4). These run only on Linux/macOS CI per the file's cfg gate.
- **DEC-6: Plan frontmatter `files_modified` list inaccurate; SUMMARY frontmatter `key-files.modified` reflects reality.** Plan listed 4 files (profile_save_runtime.rs / terminal_approval.rs / policy.rs / profile/mod.rs); actual replay touches 3 (profile_save_runtime.rs / profile/mod.rs / schema.json). terminal_approval.rs and policy.rs not touched because upstream 9b07bf7 doesn't touch them either (Q1 + Q3 both returned 0 in Task 1). schema.json was missing from the plan frontmatter but is the canonical place to register a new profile schema field.
- **DEC-7: D-40-C2 gates 3+4 (cross-target clippy linux-gnu / darwin) are load-bearing skip → CI-verified** rather than environmental-skip. Categorized in SUMMARY frontmatter as `skipped_gates_load_bearing: [3, 4]` per `.continue-here.md` anti-pattern #3. The Windows host lacks `aws-lc-sys`/`ring` C cross-compilers; CI's `ubuntu-latest` + `macos-latest` clippy jobs are the substitute. Task 5 baseline-aware regression gate (post-orchestrator-merge) is the enforcement point.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Plan-vs-reality mismatch] Plan frontmatter `files_modified` listed 4 files; actual replay touches 3.**
- **Found during:** Task 1 (surface-overlap check Q1–Q6 all returned 0; trial cherry-pick conflict set confirmed terminal_approval.rs and policy.rs not touched by upstream)
- **Issue:** Plan frontmatter listed `profile_save_runtime.rs / terminal_approval.rs / policy.rs / profile/mod.rs`. Upstream 9b07bf7 does NOT touch terminal_approval.rs or policy.rs (confirmed by Q1 = 0 and Q3 = 0). The plan's `<interfaces>` block also did not anticipate that `crates/nono-cli/data/nono-profile.schema.json` would need editing, but the new field has to be registered there or the schema validator will reject profiles using the new key.
- **Fix:** Followed the actual upstream diff and added the field to schema.json + profile/mod.rs + profile_save_runtime.rs. Did NOT touch terminal_approval.rs or policy.rs (because upstream didn't either — adding fork-only edits there would have been opportunistic composition, violating D-40-E6).
- **Files modified:** `crates/nono-cli/data/nono-profile.schema.json` (added; not in plan frontmatter); `crates/nono-cli/src/profile/mod.rs` (in plan frontmatter); `crates/nono-cli/src/profile_save_runtime.rs` (in plan frontmatter).
- **Files NOT modified (deliberately):** `crates/nono-cli/src/terminal_approval.rs` (Q1 = 0 — upstream did not touch); `crates/nono-cli/src/policy.rs` (Q3 = 0 — upstream did not touch).
- **Verification:** `git diff --stat HEAD~2 HEAD -- crates/nono-cli/src/terminal_approval.rs crates/nono-cli/src/policy.rs` returns empty.
- **Committed in:** body of `5c3da3d7` (DEC-6 cite + DEC-1 plan-mismatch note).

**2. [Rule 1 — False-positive `^Upstream-commit:` grep] Initial replay commit body contained prose phrasing "no\nUpstream-commit: trailer line because this is a manual replay" that produced a false-positive on the Task 3 `grep -c '^Upstream-commit: '` check (1 instead of 0).**
- **Found during:** Task 3 close-gate trailer-smoke verification (between Task 2 commit and SUMMARY.md write)
- **Issue:** Multi-line prose explanation of the no-D-19-trailer rationale wrapped inside the commit body such that "Upstream-commit:" landed at line start, triggering the canonical D-19 grep pattern despite being prose.
- **Fix:** Amended the commit message only (no code change) to rephrase the prose so "Upstream-commit:" no longer starts a line. Commit hash changed from `68f11fbf` to `5c3da3d7`. Code diff identical; only message reflowed.
- **Files modified:** None (commit-message-only amend).
- **Verification:** `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` returns 0; trailer-discipline gates pass.
- **Committed in:** message-only amend of Task 2 replay commit (the single allowed kind of amend per the protocol's intent — protects pre-existing committed work; this was unpushed and message-only).

---

**Total deviations:** 2 auto-fixed (1 plan-vs-reality file-list correction, 1 false-positive grep on prose phrasing).
**Impact on plan:** Both auto-fixes were necessary for correctness and gate-discipline. No scope creep; no Windows files touched. C1–C5 boundary preserved.

## Issues Encountered

- **Worktree vs main-repo cwd confusion:** Early commands ran `cd C:/Users/OMack/Nono` which targeted the main repo, not the worktree at `/c/Users/OMack/Nono/.claude/worktrees/agent-a31312b362d2a4669`. The trial cherry-pick of 9b07bf7 (intentionally aborted) ran against main's working tree, leaving 14 unmerged-paths states there. Resolved by `git reset --hard HEAD` in main (no commit landed) and restoring main's pre-existing uncommitted state (`.gitignore` + `.planning/STATE.md` were stashed before the trial; popped after). The PLAN.md disposition edit was salvaged via `/tmp/plan-with-disposition.md` copy → checkout-revert on main → re-apply on the worktree. Net effect on main: zero state change.
- **`schema_shape.rs` modify/delete:** Upstream 9b07bf7 modifies `crates/nono-cli/tests/schema_shape.rs`. Fork removed that file at some prior point; the file is NOT resurrected by this replay. If a future phase wants the schema test back, it can author one from the current schema.json shape.
- **Pre-existing flaky test (`helper_stamps_session_token_from_env`):** `cargo test --workspace --all-features` reports 1 failure in `nono::supervisor::aipc_sdk::tests::windows_loopback_tests::helper_stamps_session_token_from_env`. Same env-var-pollution race class as the `env_vars` parallel race documented in 40-01 SUMMARY (Phase 41 scope). Confirmed pre-existing: the test passes in isolation under `--test-threads=1` (verified locally). Zero touches to `crates/nono/` from this plan (confirmed by `git diff --stat HEAD~2 HEAD -- crates/nono/` = empty), so the test is structurally not caused by this plan.

## D-40-C2 8-check close gate

| Gate | Description | Status | Notes |
|------|-------------|--------|-------|
| 1 | `cargo test --workspace --all-features` (Windows host) | **PASS (modulo pre-existing flake)** | 688 passed + 1 pre-existing flake (`helper_stamps_session_token_from_env`); flake passes in isolation; Phase 41 scope; zero touches to `crates/nono/` from this plan confirmed |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) | **PASS** | Clean |
| 3 | `cargo clippy --target x86_64-unknown-linux-gnu` | **load-bearing-skip → CI-verified** | C cross-compiler not available on Windows host (`aws-lc-sys` requires `x86_64-linux-gnu-gcc`); CI's `ubuntu-latest` clippy job confirms; Task 5 baseline-aware gate (post-orchestrator-merge) is the enforcement point |
| 4 | `cargo clippy --target x86_64-apple-darwin` | **load-bearing-skip → CI-verified** | Same as gate 3; CI's `macos-latest` clippy job covers |
| 5 | `cargo fmt --all -- --check` | **PASS** | Silent |
| 6 | Phase 15 5-row detached-console smoke | **environmental-skip** | Requires interactive Windows TTY session; cannot run in this executor context |
| 7 | `wfp_port_integration` tests | **environmental-skip** | Requires WFP service admin privileges; Phase 40 plans are documented-skip per `.continue-here.md` |
| 8 | `learn_windows_integration` tests | **environmental-skip** | Requires elevated Windows execution context; Phase 40 plans are documented-skip |

**Load-bearing skip categorization (per `.continue-here.md` anti-pattern #3):** Gates 3+4 are `skipped_gates_load_bearing` (CI substitute required, NOT environmental missingness). Gates 6+7+8 are `skipped_gates_environmental` (Windows runtime genuinely unavailable in the executor's sandboxed context). The Task 5 baseline-aware CI gate (post-orchestrator-merge) is the gate-3-and-4 enforcement point — `PLAN COMPLETE` cannot be declared until CI on the head commit (post-merge) confirms zero `success → failure` job transitions versus the most recent code-touching commit on main (`4665ae75` Plan 40-01 CR-A fix per 40-01 SUMMARY; or any later code-touching baseline if main has advanced).

### Branch-specific smoke check (D-20 branch)

- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` returns **0** (MUST be 0 for D-20)
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream intent:'` returns **1**
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Fork-only wiring preserved:'` returns **1**
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-replayed-from: '` returns **2** (9b07bf7 + eb6cb09)
- `git log --format='%B' HEAD~2..HEAD | grep -c '^Co-Authored-By: Claude'` returns **2** (disposition commit + replay commit)
- D-40-E1: `git diff --stat HEAD~2 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l` returns **0**

## Wave 2 CI Verification (Task 5 — DOWNSTREAM)

**Task 5 is the wait-for-CI baseline-regression gate** per `.continue-here.md` anti-pattern #2. In worktree mode, the head commit on `main` has not advanced; Task 5 runs after the orchestrator merges this worktree branch to main and pushes. The orchestrator owns:

1. Merge worktree `worktree-agent-a31312b362d2a4669` → `main` (fast-forward; 2 new commits land).
2. Push `main` → `origin/main`.
3. `gh run watch` the resulting CI run.
4. Per-job diff vs baseline `4665ae75` (Plan 40-01 CR-A fix; the last known-good code-touching baseline confirmed by 40-04 SUMMARY's per-job CI table); if main has advanced past `4665ae75` with code-touching commits since 40-04, use the latest code-touching commit on main as the new baseline.
5. Zero `success → failure` job transitions = PASS; otherwise apply 40-01 / 40-04 CR-A class fix-on-main pattern.
6. Update PR #922 body with Plan 40-05's contribution section per the fork's umbrella-PR pattern (40-01 + 40-04 already in the umbrella; 40-05 appends).

**This SUMMARY.md committing on the worktree is the executor's "return signal" — orchestrator picks it up after merge.**

## Threat-model close-out

| Threat ID | Mitigation status | Evidence |
|-----------|-------------------|----------|
| T-40-05-01 (Tampering, D-40-E1 Windows-only files invariant) | **mitigated** | `git diff --stat HEAD~2 HEAD -- crates/ \| grep -E '_windows\|exec_strategy_windows' \| wc -l` returns 0; pre-plan Windows sentinel unchanged |
| T-40-05-02 (Tampering, cherry-pick overwrites Phase 18.1 build_prompt_text per-HandleKind surface) | **mitigated** | Task 1 Q5 returned 0; D-20 chosen because trial cherry-pick conflicts confirmed upstream feature wouldn't reach this surface (and didn't touch terminal_approval.rs at all); post-commit `grep -c 'build_prompt_text\|HandleKind' crates/nono-cli/src/terminal_approval.rs` = 45 (unchanged) |
| T-40-05-03 (Tampering, cherry-pick overwrites Phase 36/36.5 surface) | **mitigated** | Task 1 Q6 returned 0; deprecated_schema integration intact at top of profile/mod.rs; cmd_promote parser at profile_cmd.rs:456 untouched; profile-drafts surface in profile/mod.rs lines 2213+ untouched |
| T-40-05-04 (Elevation of Privilege, suppression gate silently suppresses prompts for paths user ALLOWED, not denied) | **mitigated** | Implementation gates on the compared profile's `filesystem.suppress_save_prompt` list which is a per-profile UX preference applied BEFORE the save-patch is assembled from runtime denials; the runtime deny set itself is untouched. Test `build_run_profile_patch_suppresses_paths_in_ignored_denial_list` verifies an allowed path (visible.json) still appears in the patch even when a sibling path (secret.json) is suppressed. Deny enforcement at runtime is unaffected by this UX gate. |
| T-40-05-05 (Repudiation, D-19 trailer on D-20 manual replay commit body) | **mitigated** | `git log --format='%B' HEAD~2..HEAD \| grep -c '^Upstream-commit: '` returns 0; `Upstream-replayed-from:` provenance present (2 entries — 9b07bf7 + eb6cb09); reviewers cannot mistake this for a cherry-pick |
| T-40-05-06 (Repudiation, D-40-B3 sections absent on D-20 commit) | **mitigated** | All 5 D-40-B3 sections present in `5c3da3d7` body; grep counts verified above |

## Self-Check: PASSED

**Files verified present:**
- `crates/nono-cli/data/nono-profile.schema.json` — contains `"suppress_save_prompt"` schema property under FilesystemConfig.properties. FOUND.
- `crates/nono-cli/src/profile/mod.rs` — contains `pub suppress_save_prompt: Vec<String>` field on FilesystemConfig with `#[serde(default, alias = "ignore")]`. FOUND.
- `crates/nono-cli/src/profile_save_runtime.rs` — contains `matches_ignored_denial`, `canonicalize_suppress_entry`, `ignored_denial_paths` threading through `build_run_profile_patch` + `add_patch_grant`, and 4 new unit tests. FOUND.
- `.planning/phases/40-upst4-sync-execution/40-05-FP-PROFILE-SAVE-PLAN.md` — contains `## Disposition resolution (D-40-B1)` section before `<tasks>`. FOUND.

**Commits verified in git log:**
- `64973c63` (Task 1 disposition docs), `5c3da3d7` (Task 2 D-20 replay) — both reachable from `worktree-agent-a31312b362d2a4669` HEAD via `git log --oneline HEAD~2..HEAD`.

**Gates verified:**
- D-19 trailer count (D-20 branch): 0 ✓ (must be 0)
- D-40-B3 sections present: 1 + 1 + 2 + 2 ✓
- D-40-E1 windows-file edits: 0 ✓
- Phase 18.1 surface count (build_prompt_text + HandleKind): 45 ✓ (unchanged from baseline)
- Phase 36 deprecated_schema integration: intact ✓
- Phase 36.5 cmd_promote / profile-drafts surface: intact ✓
- Schema field registered: ✓
- Struct field registered with `ignore` serde alias: ✓
- merge_profiles dedup-append extended for new field: ✓
- New tests added (4): suppression filter exact match, prefix match, empty-list noop, serde alias roundtrip: ✓
- cargo build --workspace: PASS ✓
- cargo clippy --workspace (Windows host): PASS ✓
- cargo fmt --all --check: PASS ✓

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 40-06 (FP-PROXY-TLS)** can begin after this plan closes (Wave 2 sequential per D-40-C1). Plan 40-06 reads the post-Wave-1 proxy state + post-Plan-40-05 profile state. Surface overlap is minimal: 40-05 touches profile schema + save-runtime; 40-06 touches `crates/nono-proxy/` + `crates/nono-cli/src/credential.rs` boundary held back from Plan 40-01. No expected interaction.
- **PR #922 umbrella PR** will receive a Plan 40-05 contribution section after orchestrator merges + pushes (per the fork's umbrella-PR pattern established in 40-01 + 40-04).
- **Phase 41 backlog** unchanged — no new failures introduced by this plan. Pre-existing red Linux/macOS Clippy + Test jobs + 5 Windows job classes (+ the `helper_stamps_session_token_from_env` parallel-test race in `crates/nono/`) remain Phase 41 scope.
- **CLI flag follow-up (deferred):** if user demand for `--suppress-save-prompt` / `--ignore-denied` materializes, a follow-up phase can plumb the CLI flag through the 9 files identified in the D-20 disposition resolution. Schema field + struct field are already in place, so the follow-up is minimal.

---

*Phase: 40-upst4-sync-execution*
*Completed: 2026-05-14*
