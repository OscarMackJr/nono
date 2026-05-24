---
plan_id: 48-04
plan_name: LINUX-POLICY-POLISH
phase: 48
phase_name: upst6-sync-execution
wave: 2
depends_on: [48-02, 48-03]
files_modified:
  - crates/nono-cli/src/policy.rs
  - crates/nono/src/sandbox/linux.rs
autonomous: true
requirements: [REQ-UPST6-02]
cluster: C5
cluster_disposition: will-sync
upstream_sha_range: 4fa9f6a6..1122c315
upstream_commit_count: 3
baseline_sha: 3f638dc6
tags: [upstream-sync, cherry-pick, linux, policy, wave-2]

must_haves:
  truths:
    - "Phase 47 ledger Cluster C5 (3 commits: 4fa9f6a6, e6215f8b, 1122c315) cherry-picked in upstream-chronological order"
    - "Every cherry-picked commit carries the verbatim D-19 6-line trailer block per D-48-E2 + Convention Pattern A AND a `Co-Authored-By: <upstream author>` line per upstream-sync-quick.md template (per checker WARNING reconciliation)"
    - "Fork's Phase 41 Class D Linux deny-overlap regression test (REQ-TEST-HYG-01 closed in Phase 44 Plan 44-02) stays GREEN through every cherry-pick (PATTERNS.md row #7 invariant)"
    - "Windows-only-files invariant honored per D-48-E1"
    - "Cross-target Linux + macOS clippy gates PASS per CLAUDE.md MUST/NEVER + Convention Pattern J (C5 is Linux-cfg-gated)"
    - "Baseline-aware CI gate vs SHA 3f638dc6 produces zero green→red lane transitions"
    - "REQ-UPST6-02 acceptance criterion #1 satisfied for C5"
  artifacts:
    - path: ".planning/phases/48-upst6-sync-execution/48-04-PR-SECTION.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-04-CLOSE-GATE.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-04-SUMMARY.md"
  key_links:
    - from: "git log Wave0-head..HEAD"
      to: "Phase 47 DIVERGENCE-LEDGER.md Cluster C5 row"
      via: "3 cherry-pick commits"
      pattern: "^Upstream-commit: (4fa9f6a6|e6215f8b|1122c315)"
    - from: "crates/nono-cli/src/policy.rs"
      to: "Phase 41 Class D Linux deny-overlap regression test"
      via: "REQ-TEST-HYG-01 invariant (Phase 44 Plan 44-02)"
      pattern: "(deny-overlap|deny_overlap)"
---

<objective>
Cherry-pick Phase 47 ledger Cluster C5 (Linux Landlock deny-overlap diagnostic quieting + code-review polish; 3 commits in v0.55.0). Wave 2 polish; surface-disjoint with Plans 48-05, 48-06, 48-07, 48-08 (parallel 5-way wave).

**Wave gating note (per checker BLOCKER reconciliation):** `depends_on: [48-02, 48-03]` makes the D-48-A2 4-wave SEQUENTIAL model explicit — Wave 2 cannot start until BOTH Wave 1 plans (48-02 and 48-03) close. Transitively this still includes 48-01 since both 48-02 and 48-03 depend on 48-01.

Purpose: Quiet Linux Landlock deny-overlap diagnostics without regressing the underlying deny-overlap protection. Composes additively with fork's Phase 41 Class D regression test.

Output: 3 cherry-picks, close-gate, SUMMARY, PR-SECTION (appended to umbrella).
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
@.planning/phases/48-upst6-sync-execution/48-02-SUMMARY.md
@.planning/phases/48-upst6-sync-execution/48-03-SUMMARY.md
@.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md
@.planning/phases/40-upst4-sync-execution/40-01-PROXY-HARDENING-SUMMARY.md
@.planning/phases/44-review-polish-test-hygiene-drain/44-02-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
@.planning/templates/cross-target-verify-checklist.md
@CLAUDE.md
</context>

<tasks>

<task type="auto">
  <name>Task 0: Wave-merge CWD hygiene + Wave 1 closure confirmation + branch creation</name>
  <files>(no file changes)</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Shared Pre-flight Discipline" item 1
    - .planning/phases/48-upst6-sync-execution/48-02-SUMMARY.md (Wave 1 plan-head sha — 48-02)
    - .planning/phases/48-upst6-sync-execution/48-03-SUMMARY.md (Wave 1 plan-head sha — 48-03)
    - MEMORY: feedback_windows_worktree_cwd
  </read_first>
  <action>
    1. `cd /c/Users/OMack/Nono`; `pwd` MUST be `/c/Users/OMack/Nono`
    2. Read Wave 1 plan-head shas from `48-02-SUMMARY.md` AND `48-03-SUMMARY.md`; identify the merged Wave 1 head sha (the integration point downstream of both Wave 1 plans). Call it `WAVE_1_HEAD`. Per D-48-A2 wave-sequential model, Wave 2 plans branch off the Wave 1 head (not Wave 0).
    3. `git checkout -b phase-48-04-linux-policy-polish $WAVE_1_HEAD` (Wave 2 plans all branch off Wave 1 head per D-48-A2 + checker BLOCKER reconciliation)
    4. Verify 3 C5 shas: `for sha in 4fa9f6a6 e6215f8b 1122c315; do git cat-file -e $sha^{commit} && echo "$sha OK" || echo "$sha MISSING"; done`
    5. Record chronological order: `git log --pretty=format:'%H %ci %s' v0.54.0..v0.57.0 -- crates/nono-cli/src/policy.rs crates/nono/src/sandbox/linux.rs`
    6. Pre-flight regression-test baseline: `cargo test -p nono-cli --test linux_deny_overlap 2>/dev/null || echo "test target name needs verification — check tests/ dir"`. Record exit status; this MUST stay green after every cherry-pick (PATTERNS.md row #7).
  </action>
  <verify>
    <automated>pwd | grep -q "^/c/Users/OMack/Nono$" && git rev-parse phase-48-04-linux-policy-polish &gt;/dev/null 2&gt;&amp;1 && for sha in 4fa9f6a6 e6215f8b 1122c315; do git cat-file -e $sha^{commit} || exit 1; done; echo "Task 0 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - `pwd` is `/c/Users/OMack/Nono`
    - Both Wave 1 plans (48-02 and 48-03) confirmed closed via SUMMARY status
    - Branch off Wave 1 head exists
    - 3 C5 shas resolvable
    - Pre-flight regression-test baseline status recorded
  </acceptance_criteria>
  <done>Plan branch ready.</done>
</task>

<task type="auto">
  <name>Task 1: Cherry-pick the 3 C5 commits in upstream-chronological order</name>
  <files>
    - crates/nono-cli/src/policy.rs
    - crates/nono/src/sandbox/linux.rs
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #7 (policy.rs invariants + Phase 41 Class D test)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #1 (sandbox/linux.rs invariants)
    - .planning/templates/upstream-sync-quick.md § D-19 trailer block (5-line trailer + Co-Authored-By line per template)
    - CLAUDE.md § Commits (DCO)
  </read_first>
  <action>
    Cherry-pick the 3 C5 commits in chronological order. All v0.55.0 per Phase 47 ledger.

    **C5-01: `4fa9f6a6`** — `cli: quiet Landlock deny-overlap diagnostics on Linux` (1 file: policy.rs; categories: `policy`)
    **C5-02: `e6215f8b`** — `review fix` (1 file: policy.rs; categories: `policy`)
    **C5-03: `1122c315`** — `fix: code review` (1 file: sandbox/linux.rs; categories: `other`)

    For each (full procedure per Plan 48-01 Task 2):
    1. `FULL_SHA=$(git rev-parse <abbrev>^{commit})`
    2. Extract upstream metadata (name + email + iso-date + subject)
    3. Compose the augmented 7-line trailer block per D-48-E2 + checker WARNING reconciliation (D-48-E2 6-line block + `Co-Authored-By:` 7th line per `.planning/templates/upstream-sync-quick.md`):
       ```
       Upstream-commit: <full 40-char sha>
       Upstream-author: <name> <email>
       Upstream-date: <iso-8601>
       Upstream-subject: <verbatim upstream subject>
       Upstream-tag: v0.55.0
       Upstream-categories: <per ledger row>
       Co-Authored-By: <name> <email>
       ```
       Same upstream author name+email used for BOTH `Upstream-author:` and `Co-Authored-By:` lines per template convention.
    4. `git cherry-pick --no-commit $FULL_SHA`
    5. **For `4fa9f6a6` specifically (PATTERNS.md row #7 invariant):** confirm the change is "pure diagnostic-quieting" — verify upstream commit has no logic-path removal: `git show 4fa9f6a6 -- crates/nono-cli/src/policy.rs | grep -E '^[+-]' | grep -vE '^[+-]\\s*//' | head -30`. Spot-check for removal of validation steps.
    6. Resolve conflicts per PATTERNS.md row #7 strategy (Phase 41 Class D regression test must stay green)
    7. `git commit -F <trailer-file>` (with DCO `Signed-off-by:` AFTER the 7-line trailer block)
    8. Per-commit verify: 7-line trailer + DCO; `cargo build --workspace`
    9. Per-commit regression-test run: `cargo test -p nono-cli --test linux_deny_overlap` (or equivalent) — MUST stay green
    10. Windows invariant: `git diff --name-only HEAD~1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l` equals `0`
  </action>
  <verify>
    <automated>WAVE1=$(git merge-base HEAD phase-48-02-profile-shadowing); COUNT=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'); test "$COUNT" = "3" && COAUTH=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Co-Authored-By: '); test "$COAUTH" = "3" && WIN=$(git diff --name-only $WAVE1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l); test "$WIN" = "0" && cargo build --workspace 2&gt;&amp;1 | tail -1 | grep -q "^Finished" && echo "Task 1 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - 3 cherry-picks with `^Upstream-commit:`, `^Co-Authored-By: `, `^Signed-off-by:`, `^Upstream-tag: v0\\.55\\.0$` (per upstream-sync-quick.md template reconciliation; one Co-Authored-By line per cherry-pick = 3 total across plan)
    - Windows invariant 0 violations
    - `cargo build --workspace` exits 0
    - Phase 41 Class D regression test stays green (record exit status)
  </acceptance_criteria>
  <done>3 cherry-picks landed; deny-overlap protection preserved.</done>
</task>

<task type="auto">
  <name>Task 2: Plan 48-04 close-gate (Convention Pattern G) — cross-target clippy MANDATORY</name>
  <files>.planning/phases/48-upst6-sync-execution/48-04-CLOSE-GATE.md</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern G" + § "Convention Pattern J"
    - .planning/templates/cross-target-verify-checklist.md
    - CLAUDE.md § Coding Standards (MUST/NEVER cross-target clippy bullet)
  </read_first>
  <action>
    Produce `48-04-CLOSE-GATE.md` with 8 standard gates AND leave placeholder header `### Gate 9 — Baseline-aware CI` for the baseline-CI task (Task 3) to fill (per checker WARNING #2 reconciliation — explicit gate-sequence ordering). C5 specifics:
    - C5 touches Linux-cfg-gated code (`sandbox/linux.rs`) → Gates 3+4 (cross-target clippy Linux + macOS) MANDATORY per CLAUDE.md MUST/NEVER + Convention Pattern J
    - Gates 7+8 (`wfp_port_integration` + `learn_windows_integration`) — Windows lane; should remain green (no Windows surface touched)
    - Additional spot-check: cite Phase 41 Class D regression test exit status from Task 1
    - **Produce 8 standard gates AND leave placeholder header `### Gate 9 — Baseline-aware CI` for the baseline-CI task to fill** so the close-gate file has a fixed `### Gate 9` anchor that downstream verify-greps (Task 3 + plan SUMMARY) can rely on without ordering ambiguity.
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-04-CLOSE-GATE.md && grep -cE '^### Gate [1-9]' .planning/phases/48-upst6-sync-execution/48-04-CLOSE-GATE.md | awk '{exit ($1&gt;=8)?0:1}' &amp;&amp; grep -qE '^### Gate 9' .planning/phases/48-upst6-sync-execution/48-04-CLOSE-GATE.md && echo "CLOSE-GATE present with Gate 9 placeholder"</automated>
  </verify>
  <acceptance_criteria>
    - File exists with ≥8 gate sections AND a `### Gate 9 — Baseline-aware CI` placeholder header
    - Gates 3+4 PASS (or PARTIAL `_environmental` if cross-toolchain unavailable on Windows host — document explicitly)
    - Phase 41 Class D regression test status cited
  </acceptance_criteria>
  <done>Close-gate complete with Gate 9 placeholder ready for Task 3.</done>
</task>

<task type="auto">
  <name>Task 3: Baseline-aware CI gate vs SHA 3f638dc6</name>
  <files>(no file changes)</files>
  <read_first>
    - .planning/templates/upstream-sync-quick.md (lines 96-112)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern H"
  </read_first>
  <action>
    Push `phase-48-04-linux-policy-polish` to fork's `pre-merge` branch; wait for GH Actions; categorize lanes vs `3f638dc6`. Fill the `### Gate 9 — Baseline-aware CI` placeholder header in `48-04-CLOSE-GATE.md` (placeholder authored in Task 2) with the per-lane verdict table. ZERO green→red allowed.
  </action>
  <verify>
    <automated>gh run list --branch pre-merge --limit 1 --json conclusion --jq '.[0].conclusion' | grep -qE '^(success|failure)$' && grep -q '^### Gate 9' .planning/phases/48-upst6-sync-execution/48-04-CLOSE-GATE.md && echo "Baseline-aware CI recorded"</automated>
  </verify>
  <acceptance_criteria>
    - Zero green→red transitions
    - § Gate 9 placeholder filled with per-lane verdict
  </acceptance_criteria>
  <done>Baseline-aware CI complete.</done>
</task>

<task type="auto">
  <name>Task 4: SUMMARY + PR section + STATE update + close-doc commit</name>
  <files>
    - .planning/phases/48-upst6-sync-execution/48-04-SUMMARY.md
    - .planning/phases/48-upst6-sync-execution/48-04-PR-SECTION.md
    - .planning/STATE.md
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md (template)
  </read_first>
  <action>
    1. Author `48-04-SUMMARY.md` (frontmatter: `cluster: C5`, `cluster_disposition: will-sync`, `upstream_sha_range: 4fa9f6a6..1122c315`, `upstream_commit_count: 3`, `baseline_sha: 3f638dc6`, `lane_transitions:`, `skipped_gates_*:`, `pr_section:`, `phase_41_class_d_test_status: <green|red>`).
    2. Author `48-04-PR-SECTION.md` per Convention Pattern I.
    3. Append to umbrella PR body.
    4. Update STATE.md (Plan 4 of 9).
    5. Commit:
       ```bash
       git add .planning/phases/48-upst6-sync-execution/48-04-*.md .planning/STATE.md
       git commit -s -m "docs(48-04): close cluster C5 (Linux policy + deny-overlap diagnostic polish)" \
                  -m "3 upstream cherry-picks landed with verbatim D-19 trailers + Co-Authored-By upstream attribution; Phase 41 Class D regression test stayed green; cross-target clippy verdicts recorded; STATE.md advanced; umbrella PR body appended."
       ```
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-04-SUMMARY.md && test -f .planning/phases/48-upst6-sync-execution/48-04-PR-SECTION.md && grep -q "cluster: C5" .planning/phases/48-upst6-sync-execution/48-04-SUMMARY.md && git log -1 --format=%s HEAD | grep -q "^docs(48-04):" && echo "Plan 48-04 closed"</automated>
  </verify>
  <acceptance_criteria>
    - SUMMARY + PR-SECTION exist
    - STATE.md reflects Plan 4 of 9
    - Close-doc commit subject `docs(48-04):` + DCO
  </acceptance_criteria>
  <done>Plan 48-04 closed.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Linux Landlock policy resolver → sandbox apply | C5 quiets deny-overlap diagnostics but MUST NOT remove the underlying deny-overlap protection per PATTERNS.md row #7 (Phase 41 Class D regression test) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-48-04-01 | Tampering | Diagnostic-quieting (4fa9f6a6) could mask a real deny-overlap regression | mitigate | Phase 41 Class D regression test (REQ-TEST-HYG-01) re-run after each cherry-pick (Task 1 step 9); PATTERNS.md row #7 invariant codified; close-gate Gate 1 (cargo test --workspace) catches regressions |
| T-48-04-02 | Tampering | Code-review polish (1122c315) on sandbox/linux.rs could regress strictly-allow-list invariant per CLAUDE.md § Platform-Specific Notes | mitigate | PATTERNS.md row #1 invariant: cherry-pick MUST NOT introduce deny-style code path on sandbox/linux.rs; cross-target Linux clippy gate catches semantic regressions; spot-check `git show 1122c315` for deny-style additions |
</threat_model>

<verification>
- 3 cherry-picks with verbatim D-19 trailers + Co-Authored-By + DCO
- Windows invariant 0 violations
- Phase 41 Class D regression test stays green
- Cross-target clippy Linux + macOS PASS (or PARTIAL `_environmental` with categorization)
- Close-gate + Gate 9 baseline-aware CI complete
- Zero green→red lane transitions
</verification>

<success_criteria>
- REQ-UPST6-02 acceptance criteria #1 satisfied for C5
- Phase 41 Class D Linux deny-overlap regression coverage preserved
- Wave 2 partial complete (depends on 48-05, 48-06, 48-07, 48-08 siblings)
</success_criteria>

<output>
After completion:
- `48-04-CLOSE-GATE.md`
- `48-04-SUMMARY.md`
- `48-04-PR-SECTION.md`

STATE.md reflects Plan 4 of 9.
