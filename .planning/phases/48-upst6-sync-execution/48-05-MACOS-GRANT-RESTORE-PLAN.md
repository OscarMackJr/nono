---
plan_id: 48-05
plan_name: MACOS-GRANT-RESTORE
phase: 48
phase_name: upst6-sync-execution
wave: 2
depends_on: [48-02, 48-03]
files_modified:
  - crates/nono-cli/src/capability_ext.rs
  - crates/nono-cli/src/sandbox_state.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono/src/capability.rs
  - crates/nono/src/sandbox/linux.rs
  - crates/nono/src/sandbox/macos.rs
autonomous: true
requirements: [REQ-UPST6-02]
cluster: C6
cluster_disposition: will-sync
upstream_sha_range: 2c3742ab..abca959a
upstream_commit_count: 3
baseline_sha: 3f638dc6
tags: [upstream-sync, cherry-pick, macos, seatbelt, wave-2]

must_haves:
  truths:
    - "Phase 47 ledger Cluster C6 (3 commits: 2c3742ab, 74b0be71, abca959a) cherry-picked in upstream-chronological order"
    - "Every cherry-picked commit carries the verbatim D-19 6-line trailer block per D-48-E2 + Convention Pattern A AND a `Co-Authored-By: <upstream author>` line per upstream-sync-quick.md template (per checker WARNING reconciliation)"
    - "Windows-only-files invariant honored per D-48-E1 (zero windows-touch in C6)"
    - "Cross-target macOS clippy gate PASS per CLAUDE.md MUST/NEVER + Convention Pattern J (C6 is macOS-cfg-gated)"
    - "Baseline-aware CI gate vs SHA 3f638dc6 produces zero green→red lane transitions"
    - "REQ-UPST6-02 acceptance criterion #1 satisfied for C6"
    - "Fork's exhaustive `From<ProfileDeserialize>` match preserved when abca959a touches profile/mod.rs"
  artifacts:
    - path: ".planning/phases/48-upst6-sync-execution/48-05-PR-SECTION.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-05-CLOSE-GATE.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-05-SUMMARY.md"
  key_links:
    - from: "git log Wave0-head..HEAD"
      to: "Phase 47 DIVERGENCE-LEDGER.md Cluster C6 row"
      via: "3 cherry-pick commits"
      pattern: "^Upstream-commit: (2c3742ab|74b0be71|abca959a)"
---

<objective>
Cherry-pick Phase 47 ledger Cluster C6 (macOS exact-path / future-file grant restore + localhost outbound; 3 commits in v0.55.0). Wave 2 polish; surface-disjoint with Plans 48-04, 48-06, 48-07, 48-08.

**Wave gating note (per checker BLOCKER reconciliation):** `depends_on: [48-02, 48-03]` makes the D-48-A2 4-wave SEQUENTIAL model explicit — Wave 2 cannot start until BOTH Wave 1 plans (48-02 and 48-03) close.

Per Claude's Discretion bullet in CONTEXT.md: this plan MAY mark `wfp_port_integration` and `learn_windows_integration` gates as `_environmental` skipped — they exercise Windows-only surfaces irrelevant to macOS-only changes.

Output: 3 cherry-picks, close-gate, SUMMARY, PR-SECTION.
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
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-01-CLI-CONSOLIDATION-SUMMARY.md
@.planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
@.planning/templates/cross-target-verify-checklist.md
@CLAUDE.md
</context>

<tasks>

<task type="auto">
  <name>Task 0: Wave-merge CWD hygiene + Wave 1 closure confirmation + branch + sha verify</name>
  <files>(no file changes)</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-02-SUMMARY.md (Wave 1 closure confirmation)
    - .planning/phases/48-upst6-sync-execution/48-03-SUMMARY.md (Wave 1 closure confirmation)
    - MEMORY: feedback_windows_worktree_cwd
  </read_first>
  <action>
    1. `cd /c/Users/OMack/Nono`; `pwd` MUST be `/c/Users/OMack/Nono`
    2. Confirm BOTH Wave 1 plans (48-02 and 48-03) closed via their SUMMARY status; identify the merged Wave 1 head sha (`WAVE_1_HEAD`). Per D-48-A2 + checker BLOCKER reconciliation, Wave 2 plans branch off the Wave 1 head (not Wave 0).
    3. `git checkout -b phase-48-05-macos-grant-restore $WAVE_1_HEAD`
    4. Verify 3 C6 shas: `for sha in 2c3742ab 74b0be71 abca959a; do git cat-file -e $sha^{commit} && echo "$sha OK" || echo "$sha MISSING"; done`
    5. Record chronological order: `git log --pretty=format:'%H %ci %s' v0.54.0..v0.57.0 -- crates/nono-cli/src/capability_ext.rs crates/nono-cli/src/sandbox_state.rs crates/nono-cli/src/profile/mod.rs crates/nono/src/sandbox/macos.rs`
    6. For abca959a (touches profile/mod.rs per PATTERNS.md row #6): `git show abca959a -- crates/nono-cli/src/profile/mod.rs | grep -E '^[+-](enum|struct|impl From|pub)'` — predict if exhaustive-match extension required
  </action>
  <verify>
    <automated>pwd | grep -q "^/c/Users/OMack/Nono$" && git rev-parse phase-48-05-macos-grant-restore &gt;/dev/null 2&gt;&amp;1 && for sha in 2c3742ab 74b0be71 abca959a; do git cat-file -e $sha^{commit} || exit 1; done; echo "Task 0 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - `pwd` is `/c/Users/OMack/Nono`
    - Both Wave 1 plans confirmed closed
    - Branch off Wave 1 head exists
    - 3 C6 shas resolvable
    - profile/mod.rs surface prediction recorded
  </acceptance_criteria>
  <done>Plan branch ready.</done>
</task>

<task type="auto">
  <name>Task 1: Cherry-pick the 3 C6 commits in upstream-chronological order</name>
  <files>
    - crates/nono-cli/src/capability_ext.rs
    - crates/nono-cli/src/sandbox_state.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono/src/capability.rs
    - crates/nono/src/sandbox/linux.rs
    - crates/nono/src/sandbox/macos.rs
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #4 (capability.rs invariants)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #6 (profile/mod.rs invariants — LOAD-BEARING for abca959a)
    - .planning/templates/upstream-sync-quick.md § D-19 trailer block (5-line trailer + Co-Authored-By line per template)
    - CLAUDE.md § Commits (DCO)
  </read_first>
  <action>
    Cherry-pick the 3 C6 commits in chronological order. All v0.55.0 per Phase 47 ledger.

    **C6-01: `2c3742ab`** — `fix(cli): preserve macOS future-file grants in why --self` (2 files: capability_ext.rs + sandbox_state.rs; categories: `other`)
    **C6-02: `74b0be71`** — `fix(cli): unify macOS exact-path grant restore` (2 files: capability_ext.rs + sandbox_state.rs; categories: `other`)
    **C6-03: `abca959a`** — `feat(macos): treat open_port 0 as localhost:* outbound` (4 files: profile/mod.rs, capability.rs, sandbox/linux.rs, sandbox/macos.rs; categories: `other,profile`)

    For each (full procedure per Plan 48-01 Task 2):
    1. `FULL_SHA=$(git rev-parse <abbrev>^{commit})`
    2. Extract upstream metadata (name + email + iso-date + subject)
    3. Compose the augmented 7-line trailer block per D-48-E2 + checker WARNING reconciliation (6-line D-48-E2 block + `Co-Authored-By:` 7th line per `.planning/templates/upstream-sync-quick.md`):
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
    5. **For `abca959a` specifically (touches profile/mod.rs per PATTERNS.md row #6):** if exhaustive-match extension required per Task 0 prediction, extend `impl From<ProfileDeserialize> for Profile` in-commit (NOT separate commit — preserves D-19 trailer fidelity).
    6. **For sandbox/linux.rs hunks in `abca959a`:** verify additions are `#[cfg(target_os = "macos")]`-gated OR cross-platform open_port semantics — `git show abca959a -- crates/nono/src/sandbox/linux.rs` (PATTERNS.md row #1 invariant: cherry-pick MUST NOT introduce deny-style code path on Linux side).
    7. `git commit -F <trailer-file>` (with DCO `Signed-off-by:` AFTER the 7-line trailer block)
    8. Per-commit verify: 7-line trailer + DCO; `cargo build --workspace`; `cargo build -p nono-cli` (exhaustive-match enforcement for abca959a)
    9. Windows invariant: 0 violations
  </action>
  <verify>
    <automated>WAVE1=$(git merge-base HEAD phase-48-02-profile-shadowing); COUNT=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'); test "$COUNT" = "3" && COAUTH=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Co-Authored-By: '); test "$COAUTH" = "3" && WIN=$(git diff --name-only $WAVE1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l); test "$WIN" = "0" && cargo build --workspace 2&gt;&amp;1 | tail -1 | grep -q "^Finished" && echo "Task 1 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - 3 cherry-picks with trailer + Co-Authored-By + DCO + `Upstream-tag: v0.55.0` (3 Co-Authored-By lines total across plan)
    - Windows invariant 0 violations
    - `cargo build -p nono-cli` exits 0 (exhaustive-match check)
    - For abca959a: profile/mod.rs hunks land cleanly; if extension required, in-commit
  </acceptance_criteria>
  <done>3 cherry-picks landed.</done>
</task>

<task type="auto">
  <name>Task 2: Plan 48-05 close-gate (Convention Pattern G) — Windows-lane gates may be `_environmental`</name>
  <files>.planning/phases/48-upst6-sync-execution/48-05-CLOSE-GATE.md</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern G" + § "Convention Pattern J"
    - .planning/templates/cross-target-verify-checklist.md
    - .planning/phases/48-upst6-sync-execution/48-CONTEXT.md § "Claude's Discretion" bullet on Cluster C5/C6/C7 close-gate adjustments
  </read_first>
  <action>
    Produce `48-05-CLOSE-GATE.md` with 8 standard gates AND leave placeholder header `### Gate 9 — Baseline-aware CI` for the baseline-CI task (Task 3) to fill (per checker WARNING #2 reconciliation — explicit gate-sequence ordering). Per Claude's Discretion bullet in CONTEXT.md:
    - C6 touches only macOS-side surfaces + cross-platform open_port semantics (no Windows surface)
    - Gates 3+4 (cross-target Linux + macOS clippy) MANDATORY per CLAUDE.md MUST/NEVER + Convention Pattern J — gate 4 (macOS) particularly load-bearing for C6
    - Gates 7 (`wfp_port_integration`) + 8 (`learn_windows_integration`) MAY be marked `_environmental` skipped (Windows-only tests irrelevant to macOS-only changes) — document explicitly in CLOSE-GATE.md + SUMMARY frontmatter `skipped_gates_environmental:` field
    - **Produce 8 standard gates AND leave placeholder header `### Gate 9 — Baseline-aware CI` for the baseline-CI task to fill** so the close-gate file has a fixed `### Gate 9` anchor.
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-05-CLOSE-GATE.md && grep -cE '^### Gate [1-9]' .planning/phases/48-upst6-sync-execution/48-05-CLOSE-GATE.md | awk '{exit ($1&gt;=8)?0:1}' &amp;&amp; grep -qE '^### Gate 9' .planning/phases/48-upst6-sync-execution/48-05-CLOSE-GATE.md && echo "CLOSE-GATE present with Gate 9 placeholder"</automated>
  </verify>
  <acceptance_criteria>
    - File exists with ≥8 gate sections AND a `### Gate 9 — Baseline-aware CI` placeholder header
    - Gate 4 (macOS cross-target clippy) PASS (or PARTIAL `_environmental` if cross-toolchain unavailable)
    - Gates 7+8 EITHER PASS OR explicitly `_environmental` per Claude's Discretion
    - Skipped-gate categorization explicit
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
    Push `phase-48-05-macos-grant-restore` to fork's `pre-merge` branch; wait for GH Actions; categorize lanes vs `3f638dc6`. Fill the `### Gate 9 — Baseline-aware CI` placeholder header in `48-05-CLOSE-GATE.md` (placeholder authored in Task 2) with the per-lane verdict table. ZERO green→red.
  </action>
  <verify>
    <automated>gh run list --branch pre-merge --limit 1 --json conclusion --jq '.[0].conclusion' | grep -qE '^(success|failure)$' && grep -q '^### Gate 9' .planning/phases/48-upst6-sync-execution/48-05-CLOSE-GATE.md && echo "Baseline-aware CI recorded"</automated>
  </verify>
  <acceptance_criteria>
    - Zero green→red lane transitions
    - § Gate 9 placeholder filled with per-lane verdict
  </acceptance_criteria>
  <done>Baseline-aware CI complete.</done>
</task>

<task type="auto">
  <name>Task 4: SUMMARY + PR section + STATE update + close-doc commit</name>
  <files>
    - .planning/phases/48-upst6-sync-execution/48-05-SUMMARY.md
    - .planning/phases/48-upst6-sync-execution/48-05-PR-SECTION.md
    - .planning/STATE.md
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md (template)
  </read_first>
  <action>
    1. Author `48-05-SUMMARY.md` (frontmatter: `cluster: C6`, `cluster_disposition: will-sync`, `upstream_sha_range: 2c3742ab..abca959a`, `upstream_commit_count: 3`, `baseline_sha: 3f638dc6`, `lane_transitions:`, `skipped_gates_environmental: [wfp_port_integration, learn_windows_integration]` — explicit per Claude's Discretion, `pr_section:`).
    2. Author `48-05-PR-SECTION.md` per Convention Pattern I.
    3. Append to umbrella PR body.
    4. Update STATE.md (Plan 5 of 9).
    5. Commit:
       ```bash
       git add .planning/phases/48-upst6-sync-execution/48-05-*.md .planning/STATE.md
       git commit -s -m "docs(48-05): close cluster C6 (macOS grant restore + localhost outbound)" \
                  -m "3 upstream cherry-picks landed with verbatim D-19 trailers + Co-Authored-By upstream attribution; Windows-lane gates marked _environmental per Claude's Discretion; STATE.md advanced; umbrella PR body appended."
       ```
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-05-SUMMARY.md && test -f .planning/phases/48-upst6-sync-execution/48-05-PR-SECTION.md && grep -q "cluster: C6" .planning/phases/48-upst6-sync-execution/48-05-SUMMARY.md && git log -1 --format=%s HEAD | grep -q "^docs(48-05):" && echo "Plan 48-05 closed"</automated>
  </verify>
  <acceptance_criteria>
    - SUMMARY + PR-SECTION exist
    - SUMMARY frontmatter explicitly documents `_environmental` skipped gates
    - STATE.md reflects Plan 5 of 9
    - Close-doc commit subject `docs(48-05):` + DCO
  </acceptance_criteria>
  <done>Plan 48-05 closed.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

No new trust boundary introduced by C6 — macOS Seatbelt grant restore + localhost outbound semantics are operating on the existing capability-resolution + sandbox-apply trust path. Rationale documented per Phase 47 ledger row C6 ("macOS-only seatbelt grant work; composes additively with fork's existing macOS sandbox layer").

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-48-05-01 | Elevation of Privilege | macOS exact-path grant restore (74b0be71) unification could unintentionally grant broader access via path-resolution change | mitigate | PATTERNS.md row #4 capability.rs invariant: paths canonicalized at grant time; cross-target macOS clippy gate catches type-system regressions; upstream-tested |
| T-48-05-02 | Spoofing | abca959a `open_port 0` → `localhost:*` outbound on macOS — could be misinterpreted on Linux side | mitigate | Task 1 step 6 verification: sandbox/linux.rs hunks must be `#[cfg(target_os = "macos")]`-gated OR cross-platform semantics; PATTERNS.md row #1 invariant (Linux strictly-allow-list preserved) |
</threat_model>

<verification>
- 3 cherry-picks with verbatim D-19 trailers + Co-Authored-By + DCO
- Windows invariant 0 violations
- `cargo build -p nono-cli` exits 0 (exhaustive-match preserved for abca959a)
- Cross-target macOS clippy PASS (or PARTIAL `_environmental`)
- Close-gate + Gate 9 baseline-aware CI complete
- Skipped Windows-lane gates categorized `_environmental` per Claude's Discretion
- Zero green→red transitions
</verification>

<success_criteria>
- REQ-UPST6-02 acceptance criteria #1 satisfied for C6
- Fork's exhaustive `From<ProfileDeserialize>` match preserved
- Wave 2 partial complete
</success_criteria>

<output>
After completion:
- `48-05-CLOSE-GATE.md`
- `48-05-SUMMARY.md`
- `48-05-PR-SECTION.md`

STATE.md reflects Plan 5 of 9.
