---
plan_id: 48-03
plan_name: STARTUP-TIMEOUT
phase: 48
phase_name: upst6-sync-execution
wave: 1
depends_on: [48-01]
files_modified:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/pty_proxy.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/startup_prompt.rs
autonomous: true
requirements: [REQ-UPST6-02]
cluster: C2
cluster_disposition: will-sync
upstream_sha_range: 2bed3565..50272a03
upstream_commit_count: 7
baseline_sha: 3f638dc6
tags: [upstream-sync, cherry-pick, startup-timeout, cli, wave-1, pre-flight-cleanup]

must_haves:
  truths:
    - "Phase 47 ledger Cluster C2 (7 commits: 2bed3565, a8646d26, 8628fd6d, 468d3813, 4e0e127a, 1be97978, 50272a03) cherry-picked in upstream-chronological order"
    - "Pre-flight fork-side cleanup commit removes all `startup_prompt` references BEFORE cherry-picking 4e0e127a per D-48-D3 (PATTERNS.md row #12)"
    - "Cleanup commit carries NO D-19 trailer and NO Co-Authored-By line (fork-authored cleanup per D-48-D3 — no upstream author to attribute); documented in plan SUMMARY"
    - "Every cherry-picked commit (7 upstream) carries the verbatim D-19 6-line trailer block per D-48-E2 + Convention Pattern A AND a `Co-Authored-By: <upstream author>` line per upstream-sync-quick.md template (per checker WARNING reconciliation)"
    - "Windows-only-files invariant honored per D-48-E1 EXCEPT D-48-D3 carve-out (cleanup commit may touch exec_strategy_windows/ and nono-shell-broker/ to remove `startup_prompt` references if any found; documented)"
    - "Baseline-aware CI gate vs SHA 3f638dc6 produces zero green→red lane transitions"
    - "REQ-UPST6-02 acceptance criterion #1 satisfied for C2"
  artifacts:
    - path: ".planning/phases/48-upst6-sync-execution/48-03-PR-SECTION.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-03-CLOSE-GATE.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-03-SUMMARY.md"
  key_links:
    - from: "git log Wave0-head..HEAD"
      to: "Phase 47 DIVERGENCE-LEDGER.md Cluster C2 row"
      via: "7 cherry-pick commits + 1 fork-side cleanup commit"
      pattern: "^Upstream-commit: (2bed3565|a8646d26|8628fd6d|468d3813|4e0e127a|1be97978|50272a03)"
    - from: "crates/nono-cli/src/main.rs"
      to: "(removed) crates/nono-cli/src/startup_prompt.rs"
      via: "mod startup_prompt; declaration removed in cleanup commit"
      pattern: "mod startup_prompt"
---

<objective>
Cherry-pick Phase 47 ledger Cluster C2 (process startup timeout configuration; 7 commits in v0.56.0) in upstream-chronological order. Wave 1; surface-disjoint with Plan 48-02 (C1) — runs in parallel.

**Special handling (D-48-D3):** upstream commit `4e0e127a` removes 193 lines of dead `startup_prompt.rs` infrastructure. The fork still has ~13 references to `startup_prompt` in `crates/nono-cli/src/exec_strategy.rs` + `crates/nono-cli/src/main.rs` per PATTERNS.md row #12 verified-state. Cherry-picking `4e0e127a` directly would break the fork build. Mandatory pre-flight cleanup commit removes the fork-side references FIRST (cleanup commit carries NO D-19 trailer and NO Co-Authored-By line per D-48-D3 — fork-authored, no upstream author to attribute); THEN `4e0e127a` cherry-picks cleanly.

Purpose: Absorb upstream's `--startup-timeout` flag + interactive-detection refactor; pay down the dead `startup_prompt` infrastructure debt that's been carried since well before this cycle.

Output:
- 1 fork-side cleanup commit (NO D-19 trailer, NO Co-Authored-By) BEFORE 4e0e127a
- 7 fork-side cherry-pick commits with verbatim D-19 trailers + Co-Authored-By upstream attribution
- `48-03-CLOSE-GATE.md`, `48-03-SUMMARY.md`, `48-03-PR-SECTION.md`
- PR umbrella body appended with `48-03-PR-SECTION.md`
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/48-upst6-sync-execution/48-CONTEXT.md
@.planning/phases/48-upst6-sync-execution/48-PATTERNS.md
@.planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md
@.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md
@.planning/phases/40-upst4-sync-execution/40-03-SCRUB-MODULE-SUMMARY.md
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-08b-LEARN-DEPRECATION-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
@CLAUDE.md

<interfaces>
<!-- Fork-side startup_prompt references (per PATTERNS.md row #12 verified state). -->

From crates/nono-cli/src/main.rs (~line 82):
```rust
mod startup_prompt;  // REMOVE in cleanup commit
```

From crates/nono-cli/src/exec_strategy.rs (~lines 22, 1809, 1868-1869, 1909, 1915-1916, 2245, 2344-2345, 2425, 2588-2589, 3598):
```rust
// ~13 references to startup_prompt that must be removed/refactored
// per pre-flight grep results (Task 1)
```

From crates/nono-cli/src/startup_prompt.rs:
```rust
// Entire file removed in upstream 4e0e127a (193 deletions)
// Fork removes in cleanup commit BEFORE cherry-picking 4e0e127a
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 0: Wave-merge CWD hygiene + Wave 0 baseline confirmation</name>
  <files>(no file changes)</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Shared Pre-flight Discipline" item 1
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md (Wave 0 close — confirm baseline)
    - MEMORY: feedback_windows_worktree_cwd
  </read_first>
  <action>
    1. `cd /c/Users/OMack/Nono`; `pwd` MUST be `/c/Users/OMack/Nono`
    2. Confirm Wave 0 closed; read `48-01-SUMMARY.md`; record `WAVE_0_HEAD` sha
    3. `git checkout -b phase-48-03-startup-timeout $WAVE_0_HEAD`
    4. Verify 7 C2 shas locally:
       ```bash
       for sha in 2bed3565 a8646d26 8628fd6d 468d3813 4e0e127a 1be97978 50272a03; do
         git cat-file -e $sha^{commit} && echo "$sha OK" || echo "$sha MISSING"
       done
       ```
    5. Record chronological order: `git log --pretty=format:'%H %ci %s' v0.54.0..v0.57.0 -- crates/nono-cli/src/cli.rs crates/nono-cli/src/exec_strategy.rs crates/nono-cli/src/startup_prompt.rs`
  </action>
  <verify>
    <automated>pwd | grep -q "^/c/Users/OMack/Nono$" && git rev-parse phase-48-03-startup-timeout &gt;/dev/null 2&gt;&amp;1 && for sha in 2bed3565 a8646d26 8628fd6d 468d3813 4e0e127a 1be97978 50272a03; do git cat-file -e $sha^{commit} || exit 1; done; echo "Task 0 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - `pwd` is `/c/Users/OMack/Nono`
    - Branch `phase-48-03-startup-timeout` exists off Wave 0 plan-head
    - All 7 C2 shas resolvable
  </acceptance_criteria>
  <done>Wave 0 baseline confirmed; plan branch created.</done>
</task>

<task type="auto">
  <name>Task 1: Pre-flight grep for `startup_prompt` references (D-48-D3)</name>
  <files>(read-only inspection)</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #12 (exec_strategy.rs invariants + D-48-D3 mandatory cleanup)
    - .planning/phases/40-upst4-sync-execution/40-03-SCRUB-MODULE-SUMMARY.md (cleanup-then-cherry-pick precedent)
  </read_first>
  <action>
    Per D-48-D3 mandatory pre-flight:
    ```bash
    grep -rn 'startup_prompt' crates/ | grep -v target/
    ```
    Record the full reference set in plan SUMMARY § "Fork-side startup_prompt references". Expected (per PATTERNS.md row #12 verified state):
    - `crates/nono-cli/src/main.rs:82` — `mod startup_prompt;`
    - `crates/nono-cli/src/exec_strategy.rs` — ~13 references at lines 22, 1809, 1868-1869, 1909, 1915-1916, 2245, 2344-2345, 2425, 2588-2589, 3598
    - `crates/nono-cli/src/startup_prompt.rs` — entire file
    - Verify ZERO references in `crates/nono-cli/src/exec_strategy_windows/` and `crates/nono-shell-broker/` (D-48-D3 expectation per PATTERNS.md row #15-invariant carve-out — if any found here, the cleanup commit may touch fork-only Windows files under D-48-D3 carve-out; otherwise honor D-48-E1)

    Also inspect upstream `4e0e127a` to confirm scope:
    ```bash
    git show 4e0e127a --stat
    git show 4e0e127a -- crates/nono-cli/src/startup_prompt.rs | head -20
    git show 4e0e127a -- crates/nono-cli/src/exec_strategy.rs | grep -E '^[+-]' | head -50
    ```
  </action>
  <verify>
    <automated>grep -rn 'startup_prompt' crates/ | grep -v target/ | wc -l | awk '{exit ($1&gt;0)?0:1}' &amp;&amp; echo "startup_prompt references found (cleanup task 2 required)"</automated>
  </verify>
  <acceptance_criteria>
    - Full grep output recorded in shell scrollback / SUMMARY notes
    - Confirmed reference count + locations
    - Verified zero (or recorded count) of references in fork-only Windows files
    - Upstream 4e0e127a scope confirmed (193 deletions in startup_prompt.rs + related exec_strategy.rs hunks)
  </acceptance_criteria>
  <done>Reference set recorded; ready for cleanup commit.</done>
</task>

<task type="auto">
  <name>Task 2: Fork-side cleanup commit (D-48-D3) — remove `startup_prompt` references BEFORE 4e0e127a cherry-pick</name>
  <files>
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/startup_prompt.rs
    - (under D-48-D3 carve-out only if Task 1 found Windows-side refs) crates/nono-cli/src/exec_strategy_windows/, crates/nono-shell-broker/
  </files>
  <read_first>
    - Task 1 reference set
    - .planning/phases/40-upst4-sync-execution/40-03-SCRUB-MODULE-SUMMARY.md (cleanup commit shape)
    - CLAUDE.md § Commits (DCO sign-off)
    - CLAUDE.md § Coding Standards (no `.unwrap()`)
  </read_first>
  <action>
    Author a fork-side cleanup commit that removes all `startup_prompt` references identified in Task 1. The cleanup is purely "delete dead code" — there is no functional behavior change because upstream confirms (via 4e0e127a) that the infrastructure is dead.

    Steps:
    1. Read `crates/nono-cli/src/exec_strategy.rs` around each cited line range. Determine for each whether the line is:
       - A direct `startup_prompt::function_name(...)` call → remove or replace per the surrounding logic (often the call is a no-op now; if it had side effects, document the replacement strategy)
       - An import `use crate::startup_prompt::...;` → remove
       - A comment / module-level reference → remove
    2. Remove `mod startup_prompt;` from `crates/nono-cli/src/main.rs:82`
    3. Delete `crates/nono-cli/src/startup_prompt.rs` entirely (`git rm crates/nono-cli/src/startup_prompt.rs`)
    4. If Task 1 found Windows-side refs (PATTERNS.md row #12 verified expects ZERO; if any found, this is the D-48-D3 carve-out permitting fork-only Windows file touches): remove those refs as well; explicitly document in SUMMARY that the cleanup commit touches Windows-only files under D-48-D3 carve-out (NOT a D-48-E1 invariant violation)
    5. `cargo build --workspace` MUST exit 0 (no dangling references)
    6. `cargo test --workspace` MUST exit 0 (no test regressions; removed code was dead — tests should still pass)
    7. Commit (NO D-19 trailer, NO Co-Authored-By — fork-authored cleanup per D-48-D3; there is no upstream author to attribute):
       ```bash
       git add crates/nono-cli/src/main.rs crates/nono-cli/src/exec_strategy.rs crates/nono-cli/src/startup_prompt.rs <any windows-side files from carve-out>
       git commit -s -m "cleanup(48-03): remove dead startup_prompt references ahead of upstream 4e0e127a absorption" \
                  -m "Fork-side cleanup removing ~13 references in exec_strategy.rs + mod declaration in main.rs + the startup_prompt.rs source file itself. Upstream 4e0e127a (in next cherry-pick) removes the same infrastructure (193 deletions). No D-19 trailer + no Co-Authored-By per D-48-D3 (fork-authored cleanup, not an upstream cherry-pick; no upstream author to attribute). The 4e0e127a cherry-pick will land cleanly because fork-side references are already gone."
       ```

    **CRITICAL:** This commit has NO `Upstream-commit:` trailer block AND NO `Co-Authored-By:` line. It DOES have a `Signed-off-by:` DCO line.
  </action>
  <verify>
    <automated>git log -1 --format=%s HEAD | grep -q "^cleanup(48-03):" &amp;&amp; git log -1 --format=%B HEAD | grep -qE '^Signed-off-by: ' &amp;&amp; ! git log -1 --format=%B HEAD | grep -qE '^Upstream-commit: ' &amp;&amp; ! git log -1 --format=%B HEAD | grep -qE '^Co-Authored-By: ' &amp;&amp; grep -rn 'startup_prompt' crates/ | grep -v target/ | wc -l | awk '{exit ($1==0)?0:1}' &amp;&amp; cargo build --workspace 2&gt;&amp;1 | tail -1 | grep -q "^Finished" && echo "Cleanup commit OK"</automated>
  </verify>
  <acceptance_criteria>
    - HEAD commit subject starts `cleanup(48-03):`
    - HEAD commit body has `Signed-off-by:` line
    - HEAD commit body does NOT have `Upstream-commit:` trailer (per D-48-D3 — fork-authored cleanup, not upstream cherry-pick)
    - HEAD commit body does NOT have `Co-Authored-By:` line (per D-48-D3 — fork-authored cleanup, no upstream author to attribute)
    - `grep -rn 'startup_prompt' crates/ | grep -v target/ | wc -l` returns `0` (all references removed)
    - `cargo build --workspace` exits 0
    - `cargo test --workspace` exits 0 (record count for SUMMARY)
    - File `crates/nono-cli/src/startup_prompt.rs` deleted (git status confirms)
  </acceptance_criteria>
  <done>Fork-side cleanup commit landed; ready to cherry-pick 4e0e127a cleanly.</done>
</task>

<task type="auto">
  <name>Task 3: Cherry-pick the 7 C2 commits in upstream-chronological order</name>
  <files>(per `files_modified` frontmatter)</files>
  <read_first>
    - Task 0 chronological order
    - Task 2 cleanup commit (committed; `startup_prompt` already removed)
    - .planning/templates/upstream-sync-quick.md § D-19 trailer block (5-line trailer + Co-Authored-By line per template)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern A"
    - CLAUDE.md § Commits (DCO sign-off)
  </read_first>
  <action>
    Cherry-pick the 7 C2 commits in chronological order. All 7 commits are in v0.56.0 per Phase 47 ledger.

    **C2-01: `2bed3565`** — `feat(cli): add option to configure process startup timeout` (8 files; categories: `other`)
    **C2-02: `a8646d26`** — `feat(cli): expand startup timeout interactive detection` (4 files; categories: `other`)
    **C2-03: `8628fd6d`** — `refactor(cli): require alt-screen for startup timeout` (1 file; categories: `other`)
    **C2-04: `468d3813`** — `docs(cli): clarify startup timeout definition of interactive` (1 file; categories: `other`)
    **C2-05: `4e0e127a`** — `fix(startup): use SIGKILL consistently and remove dead prompt infrastructure` (5 files; categories: `other`) — Cherry-picks cleanly because Task 2 cleanup already removed fork-side refs
    **C2-06: `1be97978`** — `refactor(cli-exec-strategy): simplify startup timeout checks` (1 file; categories: `other`)
    **C2-07: `50272a03`** — `refactor(cli): simplify startup timeout check` (1 file; categories: `other`)

    For each (full procedure per Plan 48-01 Task 2):
    1. `FULL_SHA=$(git rev-parse <abbrev>^{commit})`
    2. Extract upstream metadata (`git log -1 --format='%H%n%an <%ae>%n%aI%n%s' $FULL_SHA`)
    3. Compose the augmented 7-line trailer block per D-48-E2 + checker WARNING reconciliation (6-line D-48-E2 block + `Co-Authored-By:` 7th line per `.planning/templates/upstream-sync-quick.md`):
       ```
       Upstream-commit: <full 40-char sha>
       Upstream-author: <name> <email>
       Upstream-date: <iso-8601>
       Upstream-subject: <verbatim upstream subject>
       Upstream-tag: v0.56.0
       Upstream-categories: other
       Co-Authored-By: <name> <email>
       ```
       Same upstream author name+email used for BOTH `Upstream-author:` and `Co-Authored-By:` lines per template convention.
    4. `git cherry-pick --no-commit $FULL_SHA`
    5. If conflict on `4e0e127a` specifically: VERY unexpected since Task 2 cleanup ran — investigate; do NOT improvise. The expected case is that `4e0e127a` lands cleanly because the fork-side references are gone.
    6. `git commit -F <trailer-file>` (with 7-line trailer + DCO `Signed-off-by:` AFTER the trailer block)
    7. Per-commit verify: `git log -1 --format=%B HEAD | grep -E '^Upstream-commit: [0-9a-f]{40}$'` AND `grep -E '^Co-Authored-By: '`
    8. Per-commit smoke: `cargo build --workspace`
    9. Windows invariant check (cleanup commit was carve-out; cherry-picks themselves MUST equal 0): `git diff --name-only HEAD~1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l` MUST equal `0`

    After all 7 cherry-picks: `cargo test --workspace` to confirm baseline tests pass.
  </action>
  <verify>
    <automated>WAVE0=$(git merge-base HEAD phase-48-01-landlock-v6-af-unix); COUNT=$(git log $WAVE0..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'); test "$COUNT" = "7" && COAUTH=$(git log $WAVE0..HEAD --format=%B | grep -cE '^Co-Authored-By: '); test "$COAUTH" = "7" && CLEANUP=$(git log $WAVE0..HEAD --format=%s | grep -c '^cleanup(48-03):'); test "$CLEANUP" = "1" && cargo build --workspace 2&gt;&amp;1 | tail -1 | grep -q "^Finished" && echo "Task 3 PASS (7 upstream + 1 cleanup commits; 7 Co-Authored-By from cherry-picks; 0 from cleanup)"</automated>
  </verify>
  <acceptance_criteria>
    - `git log <Wave0-head>..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'` equals exactly `7`
    - `git log <Wave0-head>..HEAD --format=%B | grep -cE '^Co-Authored-By: '` equals exactly `7` (one per cherry-pick; cleanup commit from Task 2 carries NO Co-Authored-By)
    - `git log <Wave0-head>..HEAD --format=%s | grep -c '^cleanup(48-03):'` equals exactly `1` (the Task 2 cleanup)
    - Total new commits since Wave 0 = 8 (1 cleanup + 7 cherry-picks)
    - Each of 7 cherry-picks has `^Signed-off-by:` + `^Upstream-tag: v0\\.56\\.0$`
    - Windows invariant on cherry-pick commits (NOT the cleanup): violations equal 0
    - `cargo build --workspace` + `cargo test --workspace` exit 0 at plan-head
    - Zero remaining `startup_prompt` references in fork (`grep -rn 'startup_prompt' crates/ | grep -v target/ | wc -l` equals 0)
  </acceptance_criteria>
  <done>7 cherry-picks + 1 cleanup landed; startup_prompt fully removed.</done>
</task>

<task type="auto">
  <name>Task 4: Plan 48-03 close-gate (Convention Pattern G)</name>
  <files>.planning/phases/48-upst6-sync-execution/48-03-CLOSE-GATE.md</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern G"
    - .planning/templates/cross-target-verify-checklist.md
  </read_first>
  <action>
    Produce `48-03-CLOSE-GATE.md` with 8 standard gates per Convention Pattern G. C2 specifics:
    - Cross-platform cli.rs + runtime files; gates 3+4 (cross-target Linux + macOS clippy) MANDATORY per CLAUDE.md
    - Gates 7+8 (`wfp_port_integration` + `learn_windows_integration`) — Windows lane tests; should remain green (Task 2 cleanup carve-out's Windows-side touches, if any, were behavior-neutral removals)
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-03-CLOSE-GATE.md && grep -cE '^### Gate [1-9]' .planning/phases/48-upst6-sync-execution/48-03-CLOSE-GATE.md | awk '{exit ($1&gt;=8)?0:1}' && echo "CLOSE-GATE present"</automated>
  </verify>
  <acceptance_criteria>
    - File exists with ≥8 gate sections + verdicts
    - Skipped-gate categorization explicit
  </acceptance_criteria>
  <done>Close-gate matrix complete.</done>
</task>

<task type="auto">
  <name>Task 5: Baseline-aware CI gate vs SHA 3f638dc6</name>
  <files>(no file changes)</files>
  <read_first>
    - .planning/templates/upstream-sync-quick.md (lines 96-112)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern H"
  </read_first>
  <action>
    Push `phase-48-03-startup-timeout` to fork's `pre-merge` branch; wait for GH Actions; categorize lanes vs `3f638dc6`. Record in `48-03-CLOSE-GATE.md` § Gate 9 + plan SUMMARY `lane_transitions:`. ZERO green→red allowed.
  </action>
  <verify>
    <automated>gh run list --branch pre-merge --limit 1 --json conclusion --jq '.[0].conclusion' | grep -qE '^(success|failure)$' && grep -q '^### Gate 9' .planning/phases/48-upst6-sync-execution/48-03-CLOSE-GATE.md && echo "Baseline-aware CI recorded"</automated>
  </verify>
  <acceptance_criteria>
    - Zero green→red lane transitions
    - `48-03-CLOSE-GATE.md` § Gate 9 has per-lane verdict table
  </acceptance_criteria>
  <done>Baseline-aware CI gate complete.</done>
</task>

<task type="auto">
  <name>Task 6: Plan 48-03 SUMMARY + PR section + STATE update + close-doc commit</name>
  <files>
    - .planning/phases/48-upst6-sync-execution/48-03-SUMMARY.md
    - .planning/phases/48-upst6-sync-execution/48-03-PR-SECTION.md
    - .planning/STATE.md
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md (template)
    - .planning/phases/48-upst6-sync-execution/48-01-PR-SECTION.md (template)
  </read_first>
  <action>
    1. Author `48-03-SUMMARY.md` with frontmatter (`cluster: C2`, `cluster_disposition: will-sync`, `upstream_sha_range: 2bed3565..50272a03`, `upstream_commit_count: 7`, `baseline_sha: 3f638dc6`, `lane_transitions:`, `skipped_gates_*:`, `pr_section: 48-03-PR-SECTION.md`, `fork_side_cleanup_commits: 1` — explicit field documenting the D-48-D3 cleanup) and sections:
       - § Fork-side startup_prompt reference inventory (Task 1)
       - § D-48-D3 cleanup commit shape + rationale (Task 2 — explicit note: NO D-19 trailer + NO Co-Authored-By per D-48-D3 carve-out)
       - § Per-commit cherry-pick notes (Task 3 — 7 cherry-picks each with Co-Authored-By upstream attribution)
       - § Cross-target clippy results
       - § Baseline-aware CI verdict
       - § Wave 1 sibling status (Plan 48-02)
    2. Author `48-03-PR-SECTION.md` per Convention Pattern I — note the D-48-D3 cleanup commit in the "key decisions" section.
    3. Append `48-03-PR-SECTION.md` content to umbrella PR body.
    4. Update STATE.md.
    5. Stage + commit close artifacts:
       ```bash
       git add .planning/phases/48-upst6-sync-execution/48-03-*.md .planning/STATE.md
       git commit -s -m "docs(48-03): close cluster C2 (startup-timeout + dead-infra cleanup)" \
                  -m "7 upstream cherry-picks with Co-Authored-By upstream attribution + 1 fork-side D-48-D3 cleanup commit (no Co-Authored-By since fork-authored); startup_prompt infrastructure fully removed (13 refs + mod decl + source file); STATE.md advanced; umbrella PR body appended."
       ```
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-03-SUMMARY.md && test -f .planning/phases/48-upst6-sync-execution/48-03-PR-SECTION.md && grep -q "cluster: C2" .planning/phases/48-upst6-sync-execution/48-03-SUMMARY.md && grep -q "fork_side_cleanup_commits: 1" .planning/phases/48-upst6-sync-execution/48-03-SUMMARY.md && git log -1 --format=%s HEAD | grep -q "^docs(48-03):" && echo "Plan 48-03 closed"</automated>
  </verify>
  <acceptance_criteria>
    - SUMMARY + PR-SECTION exist
    - SUMMARY frontmatter has `fork_side_cleanup_commits: 1` (explicit D-48-D3 carve-out marker)
    - STATE.md reflects Phase 48 / Plan 3 of 9
    - Close-doc commit subject `docs(48-03):` + DCO
  </acceptance_criteria>
  <done>Plan 48-03 closed; Wave 1 complete when Plan 48-02 also closes; Wave 2 cleared.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Sandboxed-process startup timeout configuration | New `--startup-timeout` flag + `NONO_STARTUP_TIMEOUT` env-var on `run`/`shell`/`wrap`; user-controllable timeout governs SIGKILL escalation per 4e0e127a |
| Interactive-detection refactor (alt-screen instead of any-output) | Refactor changes detection heuristic for whether to apply the startup timeout; security-relevant only insofar as it could let a misbehaving child evade timeout-based termination |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-48-03-01 | Denial of Service | Misconfigured `--startup-timeout` could DoS legitimate slow-starting children | accept | New flag is user-configurable; upstream design defaults preserved; fork's existing fork+wait process model (CLAUDE.md § Key Design Decisions #2) is unchanged |
| T-48-03-02 | Tampering | Cleanup commit removing `startup_prompt` references could accidentally remove a security-relevant code path | mitigate | Pre-flight grep (Task 1) records exact reference set; cleanup is "delete dead code" — verified by 4e0e127a evidence that the infrastructure is dead in upstream; `cargo test --workspace` after cleanup catches any test regressions; explicit verification in Task 2 acceptance criteria |
| T-48-03-03 | Information Disclosure | Refactor termination messages with `--profile` hints (2bed3565) could leak profile names to stderr | accept | Profile names are user-facing identifiers (not secrets); fork's audit-attestation surface unaffected (Phase 38 + Phase 45 REQ-RESL-NIX-04 coverage preserved) |
| T-48-03-04 | Elevation of Privilege | SIGKILL consistency (4e0e127a) — SIGKILL is the strongest termination signal; using it consistently does NOT escalate privileges, but stricter termination could mask bugs | accept | Upstream-tested; fork's Phase 27 PTY proxy work is upstream-equivalent |

**Fork-side defense-in-depth preserved (per PATTERNS.md row #12):**
- Execution strategies (Direct/Monitor/Supervised) per CLAUDE.md § Key Design Decisions #2 unchanged; fork+wait process model preserved
- D-48-D3 cleanup commit carve-out explicitly bounded: removes only dead `startup_prompt` references (no behavior change); no other fork-only files touched
</threat_model>

<verification>
- Cherry-pick count: 7 (between Wave 0 head and plan-head, NOT counting cleanup)
- Co-Authored-By count: 7 (one per cherry-pick per checker WARNING reconciliation; cleanup commit from Task 2 has none)
- Cleanup count: 1 (cleanup(48-03) commit subject)
- `startup_prompt` references: 0 in fork after plan close
- Windows invariant on cherry-picks: 0 violations
- Cleanup commit Windows-side touches: documented in SUMMARY if any (D-48-D3 carve-out)
- `cargo build --workspace` + `cargo test --workspace` exit 0
- Close-gate matrix complete (8+ gates + Gate 9 baseline-aware CI)
- SUMMARY + PR-SECTION exist; umbrella body extended
</verification>

<success_criteria>
- 7 C2 cherry-picks land with verbatim D-19 trailers + Co-Authored-By in v0.56.0 tag
- 1 fork-side D-48-D3 cleanup commit lands with NO D-19 trailer + NO Co-Authored-By but WITH DCO sign-off
- Zero `startup_prompt` references remain in fork
- Baseline-aware CI gate zero green→red
- REQ-UPST6-02 acceptance criteria #1 satisfied for C2
- Wave 1 complete when sibling Plan 48-02 also closes
</success_criteria>

<output>
After completion:
- `48-03-CLOSE-GATE.md`
- `48-03-SUMMARY.md`
- `48-03-PR-SECTION.md` (appended to umbrella PR body)

STATE.md reflects Phase 48 / Plan 3 of 9.
