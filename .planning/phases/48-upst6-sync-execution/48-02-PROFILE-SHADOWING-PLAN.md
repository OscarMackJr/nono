---
plan_id: 48-02
plan_name: PROFILE-SHADOWING
phase: 48
phase_name: upst6-sync-execution
wave: 1
depends_on: [48-01]
files_modified:
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile_cmd.rs
  - crates/nono-cli/src/profile_save_runtime.rs
  - crates/nono-cli/src/profile_runtime.rs
  - crates/nono-cli/src/learn_runtime.rs
autonomous: true
requirements: [REQ-UPST6-02]
cluster: C1
cluster_disposition: will-sync
upstream_sha_range: 0b05508f..750f4653
upstream_commit_count: 9
baseline_sha: 3f638dc6
tags: [upstream-sync, cherry-pick, profile, pack-verification, wave-1]

must_haves:
  truths:
    - "Phase 47 ledger Cluster C1 (9 commits: 750f4653, 316c6a2c, 3d3d239a, 0a4db57e, bd76c6b5, c897c8cc, b3556139, 0015f348, 0b05508f) cherry-picked in upstream-chronological order"
    - "Every cherry-picked commit carries the verbatim D-19 6-line trailer block per D-48-E2 + Convention Pattern A AND a `Co-Authored-By: <upstream author>` line per upstream-sync-quick.md template (per checker WARNING reconciliation)"
    - "Fork's Phase 36-01b exhaustive `From<ProfileDeserialize> for Profile` match preserved through every cherry-pick (compile-time enforcement)"
    - "Fork's Phase 36-01c `bypass_protection` canonical name honored if cherry-picks touch profile fields (no `override_deny` regression)"
    - "Windows-only-files invariant honored per D-48-E1 (zero windows-touch in C1 per Phase 47 ledger)"
    - "Baseline-aware CI gate vs SHA 3f638dc6 produces zero green→red lane transitions per D-48-E3 + Convention Pattern H"
    - "REQ-UPST6-02 acceptance criterion #1 satisfied for C1"
    - "PR umbrella body extended with `48-02-PR-SECTION.md` contribution per D-48-E6 + Convention Pattern I"
  artifacts:
    - path: ".planning/phases/48-upst6-sync-execution/48-02-PR-SECTION.md"
      provides: "Per-plan umbrella PR contribution section"
    - path: ".planning/phases/48-upst6-sync-execution/48-02-CLOSE-GATE.md"
      provides: "8-check close-gate matrix per Convention Pattern G"
    - path: ".planning/phases/48-upst6-sync-execution/48-02-SUMMARY.md"
      provides: "Plan close summary"
  key_links:
    - from: "git log 48-01-head..HEAD"
      to: "Phase 47 DIVERGENCE-LEDGER.md Cluster C1 row"
      via: "9 cherry-pick commits with Upstream-commit: trailers matching 9 upstream shas"
      pattern: "^Upstream-commit: (750f4653|316c6a2c|3d3d239a|0a4db57e|bd76c6b5|c897c8cc|b3556139|0015f348|0b05508f)"
    - from: "crates/nono-cli/src/profile/mod.rs"
      to: "Phase 36-01b canonical sections"
      via: "impl From<ProfileDeserialize> for Profile exhaustive match"
      pattern: "impl From<ProfileDeserialize> for Profile"
---

<objective>
Cherry-pick Phase 47 ledger Cluster C1 (profile shadowing checks + pack signer verification + name-resolution polish; 9 commits) in upstream-chronological order. Wave 1; surface-disjoint with Plan 48-03 (C2) — runs in parallel.

Purpose: Absorb upstream's profile shadowing hardening + pack-signer verification surface. This composes additively with fork's Phase 36-01b canonical sections (`CommandsConfig`, `FilesystemConfig.deny/bypass_protection`, `LegacyPolicyPatch`, `DeprecationCounter`) per Phase 47 § Empirical cross-check File #4 — but the executor MUST diff-inspect each profile/mod.rs hunk and extend the exhaustive match in-commit if any cherry-pick adds a new profile field.

Output:
- 9 fork-side cherry-pick commits with verbatim D-19 trailers + Co-Authored-By upstream attribution
- `48-02-CLOSE-GATE.md`, `48-02-SUMMARY.md`, `48-02-PR-SECTION.md`
- PR umbrella body appended with `48-02-PR-SECTION.md` contribution
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
@.planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md
@.planning/phases/36-upst3-deep-closure/36-01c-OVERRIDE-DENY-RENAME-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
@.planning/templates/cross-target-verify-checklist.md
@CLAUDE.md

<interfaces>
<!-- Fork's exhaustive From<ProfileDeserialize> match (Phase 36-01b LOAD-BEARING). -->

From crates/nono-cli/src/profile/mod.rs (~line 2068):
```rust
impl From<ProfileDeserialize> for Profile {
    fn from(d: ProfileDeserialize) -> Self {
        // exhaustive match arms over:
        //   CommandsConfig, FilesystemConfig { deny, bypass_protection, ... },
        //   LegacyPolicyPatch, DeprecationCounter, ... (Phase 36-01b)
        // Any new profile field upstream adds MUST get an arm here in the SAME cherry-pick commit.
    }
}
```

From test fixtures (~lines 289-311):
```rust
// Asserts filesystem.deny + commands.allow survive round-trip through ProfileDeserialize→Profile→serialize.
// Re-run after every C1 cherry-pick (cargo test -p nono-cli).
```

Canonical name (Phase 36-01c rename):
```rust
// USE: bypass_protection
// DO NOT regress to: override_deny
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 0: Wave-merge CWD hygiene + Wave 0 baseline confirmation</name>
  <files>(no file changes; pre-flight discipline only)</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Shared Pre-flight Discipline" item 1
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md (Wave 0 close — confirm baseline sha for Wave 1)
    - MEMORY: feedback_windows_worktree_cwd
  </read_first>
  <action>
    1. `cd /c/Users/OMack/Nono`; `pwd` MUST print `/c/Users/OMack/Nono`
    2. Confirm Wave 0 closed: read `48-01-SUMMARY.md`; record the plan-head sha of Plan 48-01 (call it `WAVE_0_HEAD`)
    3. Create Wave 1 branch: `git checkout -b phase-48-02-profile-shadowing $WAVE_0_HEAD`
    4. Verify each of the 9 C1 shas exists locally:
       ```bash
       for sha in 750f4653 316c6a2c 3d3d239a 0a4db57e bd76c6b5 c897c8cc b3556139 0015f348 0b05508f; do
         git cat-file -e $sha^{commit} && echo "$sha OK" || echo "$sha MISSING"
       done
       ```
    5. Record chronological order: `git log --pretty=format:'%H %ci %s' v0.54.0..v0.57.0 -- crates/nono-cli/src/profile/mod.rs crates/nono-cli/src/profile_cmd.rs crates/nono-cli/src/profile_save_runtime.rs crates/nono-cli/src/profile_runtime.rs crates/nono-cli/src/learn_runtime.rs`
  </action>
  <verify>
    <automated>pwd | grep -q "^/c/Users/OMack/Nono$" && git rev-parse phase-48-02-profile-shadowing &gt;/dev/null 2&gt;&amp;1 && for sha in 750f4653 316c6a2c 3d3d239a 0a4db57e bd76c6b5 c897c8cc b3556139 0015f348 0b05508f; do git cat-file -e $sha^{commit} || exit 1; done; echo "Task 0 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - `pwd` is `/c/Users/OMack/Nono` exactly
    - Branch `phase-48-02-profile-shadowing` exists off Wave 0 plan-head sha
    - All 9 C1 shas resolvable locally
    - Chronological order recorded in shell scrollback
  </acceptance_criteria>
  <done>Wave 0 baseline confirmed; plan branch created; 9 C1 shas verified.</done>
</task>

<task type="auto">
  <name>Task 1: Pre-cherry-pick profile/mod.rs hot-spot inspection</name>
  <files>(read-only inspection; no file changes)</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #6 (profile/mod.rs LOAD-BEARING invariant + Phase 47 hot-spot finding)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md § "Empirical cross-check" File #4
    - .planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md (fork's exhaustive match details)
    - .planning/phases/36-upst3-deep-closure/36-01c-OVERRIDE-DENY-RENAME-SUMMARY.md (`bypass_protection` canonical name)
  </read_first>
  <action>
    For each C1 commit that touches profile/mod.rs (per ledger: 750f4653, 316c6a2c, 3d3d239a, c897c8cc, b3556139, 0015f348 — verify exact set via `git show <sha> --stat | grep profile/mod.rs`), do the surface-prediction step:

    1. `git show <sha> -- crates/nono-cli/src/profile/mod.rs | grep -E '^[+-](enum|struct|impl From|pub fn|fn)'` — surface every new variant / struct / arm / function added or removed.
    2. `git show <sha> -- crates/nono-cli/src/profile/mod.rs | grep -E 'override_deny|bypass_protection'` — confirm no `override_deny` regression (Phase 36-01c canonical name = `bypass_protection`).
    3. `grep -nE 'CommandsConfig|FilesystemConfig|LegacyPolicyPatch|DeprecationCounter|impl From<ProfileDeserialize>' crates/nono-cli/src/profile/mod.rs` — record fork's exhaustive arm list for cross-reference.

    Record findings inline in plan SUMMARY § "Profile/mod.rs hot-spot inspection". If a commit adds a new profile field that requires arm extension, document the strategy: extension MUST land in the same cherry-pick commit body (NOT separate commit — preserves D-19 trailer fidelity).
  </action>
  <verify>
    <automated>for sha in 750f4653 316c6a2c 3d3d239a c897c8cc b3556139 0015f348; do git show $sha --stat 2&gt;/dev/null | grep -q "profile/mod.rs" &amp;&amp; echo "$sha touches profile/mod.rs" || echo "$sha does not touch profile/mod.rs"; done; echo "Inspection complete"</automated>
  </verify>
  <acceptance_criteria>
    - For each C1 sha touching profile/mod.rs: surface prediction recorded in shell scrollback / SUMMARY notes
    - Zero `override_deny` references introduced (grep ledger) — only `bypass_protection`
    - Fork's exhaustive arm list captured for cross-reference during Task 2
  </acceptance_criteria>
  <done>Pre-cherry-pick profile inspection complete; per-commit extension strategy documented.</done>
</task>

<task type="auto">
  <name>Task 2: Cherry-pick the 9 C1 commits in upstream-chronological order</name>
  <files>(per `files_modified` frontmatter — actual touch set per commit)</files>
  <read_first>
    - Task 1 inspection notes
    - .planning/templates/upstream-sync-quick.md § D-19 trailer block (5-line trailer + Co-Authored-By line per template)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern A. D-19 6-line cherry-pick trailer block"
    - CLAUDE.md § Commits (DCO sign-off)
    - CLAUDE.md § Coding Standards (no `.unwrap()`)
  </read_first>
  <action>
    Cherry-pick the 9 C1 commits in chronological order recorded in Task 0. For each sha:

    **C1-01: `0b05508f`** — `fix(profile-verification): strengthen profile and pack verification checks` (v0.55.0; 1 file; categories: `other`)
    **C1-02: `0015f348`** — `feat(profile): ensure source pack is included for verification` (v0.55.0; 3 files; categories: `other,profile`)
    **C1-03: `b3556139`** — `feat(profiles): verify pack signer identities` (v0.55.0; 2 files; categories: `other,profile`)
    **C1-04: `c897c8cc`** — `feat(profiles): expand shadowing checks to include pack profiles` (v0.57.0; 2 files; categories: `other`)
    **C1-05: `bd76c6b5`** — `fix(profiles): address review points on shadow-check PR` (v0.57.0; 1 file; categories: `other`)
    **C1-06: `0a4db57e`** — `fix(profiles): block profile init when name shadows builtin or pack profile` (v0.57.0; 1 file; categories: `other`)
    **C1-07: `3d3d239a`** — `feat(profile): refine profile name resolution and init validation` (v0.57.0; 3 files; categories: `other,profile`)
    **C1-08: `316c6a2c`** — `fix(profile): handle versioned package refs in fast path` (v0.57.0; 1 file; categories: `profile`)
    **C1-09: `750f4653`** — `fix(profile): fix fmt and test assertion after shadow-check refactor` (v0.57.0; 3 files; categories: `other,profile`)

    **NOTE:** chronological order from Task 0 may differ. Use chronological.

    For each:
    1. `FULL_SHA=$(git rev-parse <abbrev>^{commit})`
    2. Extract upstream metadata for trailer block (per Convention Pattern A; full procedure in Plan 48-01 Task 2)
    3. Compose the augmented 7-line trailer block per D-48-E2 + checker WARNING reconciliation (6-line D-48-E2 block + `Co-Authored-By:` 7th line per `.planning/templates/upstream-sync-quick.md`):
       ```
       Upstream-commit: <full 40-char sha>
       Upstream-author: <name> <email>
       Upstream-date: <iso-8601>
       Upstream-subject: <verbatim upstream subject>
       Upstream-tag: <v0.55.0 or v0.57.0 per ledger>
       Upstream-categories: <ledger row categories>
       Co-Authored-By: <name> <email>
       ```
       Same upstream author name+email used for BOTH `Upstream-author:` and `Co-Authored-By:` lines per template convention.
    4. `git cherry-pick --no-commit $FULL_SHA`
    5. If profile/mod.rs hunk surfaces exhaustive-match break: extend `impl From<ProfileDeserialize> for Profile` in-commit per Task 1 inspection
    6. `git commit -F <trailer-file>` (with 7-line trailer + DCO `Signed-off-by:` AFTER the trailer block)
    7. Per-commit verify: `git log -1 --format=%B HEAD | grep -E '^Upstream-commit: [0-9a-f]{40}$'` AND `grep -E '^Co-Authored-By: '`
    8. Per-commit smoke: `cargo build -p nono-cli` (catches exhaustive-match regressions specifically) + `cargo build --workspace`
    9. Windows-only-files invariant check: `git diff --name-only HEAD~1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l` MUST equal `0`
  </action>
  <verify>
    <automated>WAVE0=$(git merge-base HEAD phase-48-01-landlock-v6-af-unix); COUNT=$(git log $WAVE0..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'); test "$COUNT" = "9" && COAUTH=$(git log $WAVE0..HEAD --format=%B | grep -cE '^Co-Authored-By: '); test "$COAUTH" = "9" && WIN=$(git diff --name-only $WAVE0..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l); test "$WIN" = "0" && cargo build --workspace 2&gt;&amp;1 | tail -1 | grep -q "^Finished" && echo "Task 2 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - 9 cherry-picks between Wave 0 head and plan-head; each carries `^Upstream-commit: [0-9a-f]{40}$` + `^Co-Authored-By: ` + `^Signed-off-by: ` + `^Upstream-tag: v0\\.5[57]\\.0$` (9 Co-Authored-By lines total across plan per WARNING reconciliation)
    - `cargo build -p nono-cli` exits 0 (exhaustive-match enforcement)
    - `cargo build --workspace` exits 0
    - Windows invariant violations equal 0
    - `grep -c override_deny crates/nono-cli/src/profile/mod.rs` returns 0 (Phase 36-01c invariant preserved)
  </acceptance_criteria>
  <done>9 C1 cherry-pick commits land with verbatim D-19 trailers + Co-Authored-By + DCO; exhaustive match preserved; Windows invariant honored.</done>
</task>

<task type="auto">
  <name>Task 3: Plan 48-02 close-gate (Convention Pattern G)</name>
  <files>.planning/phases/48-upst6-sync-execution/48-02-CLOSE-GATE.md</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern G"
    - .planning/templates/cross-target-verify-checklist.md
    - .planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md (Wave 0 reference)
  </read_first>
  <action>
    Produce `48-02-CLOSE-GATE.md` with 8 standard gates per Convention Pattern G. Specifics for C1:
    - C1 touches only cross-platform profile/profile_cmd/profile_save_runtime/profile_runtime/learn_runtime files — NO cfg-gated Linux or macOS code
    - Gates 3+4 (cross-target Linux + macOS clippy) are still MANDATORY per CLAUDE.md MUST/NEVER (workspace-wide clippy must pass on all targets)
    - Gates 7+8 (`wfp_port_integration` + `learn_windows_integration`) — Windows-lane tests; PASS or PARTIAL with `_environmental` if Windows lane unavailable; should remain green since C1 does not touch Windows surface

    Run all 8 gates; record verdicts; document any skipped-gate categorization.
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-02-CLOSE-GATE.md && grep -cE '^### Gate [1-9]' .planning/phases/48-upst6-sync-execution/48-02-CLOSE-GATE.md | awk '{exit ($1&gt;=8)?0:1}' && echo "CLOSE-GATE present"</automated>
  </verify>
  <acceptance_criteria>
    - File exists with ≥8 explicit gate sections
    - Each gate has command + exit code + verdict
    - Skipped-gate categorization explicit (load_bearing vs environmental)
  </acceptance_criteria>
  <done>Close-gate matrix complete.</done>
</task>

<task type="auto">
  <name>Task 4: Baseline-aware CI gate vs SHA 3f638dc6 (Convention Pattern H)</name>
  <files>(no fork-side file changes)</files>
  <read_first>
    - .planning/templates/upstream-sync-quick.md (lines 96-112)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern H"
  </read_first>
  <action>
    Push `phase-48-02-profile-shadowing` to fork's `pre-merge` branch; wait for GH Actions; categorize each lane vs baseline `3f638dc6` (green→green PASS, green→red FAIL, red→red PASS, red→green PASS+IMPROVEMENT).

    Record per-lane verdict in `48-02-CLOSE-GATE.md` § Gate 9 + plan SUMMARY `lane_transitions:` block. ZERO green→red allowed.

    Use `gh run list --branch pre-merge --limit 1 --json conclusion,databaseId` + `gh run view <id> --json jobs --jq '.jobs[] | {name, conclusion}'`.
  </action>
  <verify>
    <automated>gh run list --branch pre-merge --limit 1 --json conclusion --jq '.[0].conclusion' | grep -qE '^(success|failure)$' && grep -q '^### Gate 9' .planning/phases/48-upst6-sync-execution/48-02-CLOSE-GATE.md && echo "Baseline-aware CI recorded"</automated>
  </verify>
  <acceptance_criteria>
    - Latest pre-merge run exists; `conclusion` is `success` or `failure` (failure only if all FAIL lanes are red→red carry-forward)
    - Zero green→red transitions
    - SUMMARY frontmatter (Task 5) has `baseline_sha: 3f638dc6` + `lane_transitions:` block
  </acceptance_criteria>
  <done>Baseline-aware CI gate complete.</done>
</task>

<task type="auto">
  <name>Task 5: Plan 48-02 SUMMARY + PR section + STATE update + close-doc commit</name>
  <files>
    - .planning/phases/48-upst6-sync-execution/48-02-SUMMARY.md
    - .planning/phases/48-upst6-sync-execution/48-02-PR-SECTION.md
    - .planning/STATE.md
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md (SUMMARY shape reference)
    - .planning/phases/48-upst6-sync-execution/48-01-PR-SECTION.md (PR-SECTION shape reference)
    - CLAUDE.md § Commits (DCO sign-off)
  </read_first>
  <action>
    1. Author `48-02-SUMMARY.md` with frontmatter (`plan_id`, `phase`, `cluster: C1`, `cluster_disposition: will-sync`, `upstream_sha_range: 0b05508f..750f4653`, `upstream_commit_count: 9`, `baseline_sha: 3f638dc6`, `lane_transitions:`, `skipped_gates_*:`, `pr_section: 48-02-PR-SECTION.md`) and sections:
       - § Profile/mod.rs hot-spot inspection findings (Task 1)
       - § Per-commit notes (resolution applied; in-commit exhaustive-match extensions if any)
       - § Cross-target clippy results
       - § Baseline-aware CI gate verdict
       - § Wave 1 sibling status (Plan 48-03 status reference, if known)
    2. Author `48-02-PR-SECTION.md` per Convention Pattern I.
    3. Append `48-02-PR-SECTION.md` content to umbrella PR body (`gh pr edit <pr-number> --body-file <concatenated-body>` or per Phase 43 `43-UMBRELLA-PR.txt` convention).
    4. Update STATE.md `## Current Position` + `last_activity` + `## Progress` Phase 48 row.
    5. Stage planning artifacts + commit:
       ```bash
       git add .planning/phases/48-upst6-sync-execution/48-02-*.md .planning/STATE.md
       git commit -s -m "docs(48-02): close cluster C1 (profile shadowing + pack verification)" \
                  -m "9 upstream cherry-picks landed with verbatim D-19 trailers + Co-Authored-By upstream attribution; close-gate + SUMMARY + PR-SECTION shipped; STATE.md advanced; umbrella PR body appended."
       ```
    6. Verify DCO sign-off on close-doc HEAD: `git log -1 --format=%B HEAD | grep -E '^Signed-off-by: '`
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-02-SUMMARY.md && test -f .planning/phases/48-upst6-sync-execution/48-02-PR-SECTION.md && grep -q "cluster: C1" .planning/phases/48-upst6-sync-execution/48-02-SUMMARY.md && git log -1 --format=%s HEAD | grep -q "^docs(48-02):" && echo "Plan 48-02 closed"</automated>
  </verify>
  <acceptance_criteria>
    - SUMMARY + PR-SECTION exist with required frontmatter + sections
    - Umbrella PR body extended with `48-02-PR-SECTION.md` content (manual verify via `gh pr view <pr-number>`)
    - STATE.md reflects Phase 48 / Plan 2 of 9
    - Close-doc commit subject `docs(48-02):` + DCO sign-off
  </acceptance_criteria>
  <done>Plan 48-02 closed; Wave 1 sibling Plan 48-03 may run in parallel or has already closed; Wave 2 cleared after both Wave 1 plans close.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| User-saved profile → built-in/pack profile shadowing prevention | C1's shadowing checks (c897c8cc + 0a4db57e + bd76c6b5) prevent user profiles from inadvertently overriding pack/builtin profiles — security-relevant trust-boundary integrity |
| Pack source → trust-bundle signer verification | C1's pack signer verification (b3556139 + 0015f348) + hard-block on trust-bundle-without-lockfile (0b05508f) protect against silent acceptance of unpinned signers when a trust bundle is present |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-48-02-01 | Spoofing | Profile shadowing — user-saved profile silently overrides pack/builtin profile | mitigate | Upstream's hardening cherry-picked as-is (c897c8cc + 0a4db57e + bd76c6b5); fork's Phase 36-01b canonical sections preserved through in-commit exhaustive-match extension (Task 2) |
| T-48-02-02 | Tampering | Pack signer verification — unpinned signer silently accepted when trust bundle present | mitigate | C1 commit 0b05508f hard-blocks trust-bundle-without-lockfile provenance; b3556139 + 0015f348 verify pack signer identities + ensure source pack inclusion |
| T-48-02-03 | Spoofing | Profile field shadowing breaking fork's exhaustive From<ProfileDeserialize> match | mitigate | Pre-flight inspection (Task 1) + compile-time enforcement (cargo build -p nono-cli in Task 2) + Phase 36-01b regression coverage at lines 289-311 |
| T-48-02-04 | Tampering | `override_deny` → `bypass_protection` rename regression | mitigate | Task 1 explicit grep for `override_deny`; Phase 36-01c invariant; close-gate spot-check `grep -c override_deny crates/nono-cli/src/profile/mod.rs` returns 0 |
</threat_model>

<verification>
- Cherry-pick count: `git log <Wave0-head>..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'` equals `9`
- Co-Authored-By count: `git log <Wave0-head>..HEAD --format=%B | grep -cE '^Co-Authored-By: '` equals `9` (per checker WARNING reconciliation)
- DCO sign-off on every commit
- Windows invariant: `git diff --name-only <Wave0-head>..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l` equals `0`
- Exhaustive match preserved: `cargo build -p nono-cli` exits 0
- Canonical name preserved: `grep -c override_deny crates/nono-cli/src/profile/mod.rs` returns 0
- Close-gate matrix `48-02-CLOSE-GATE.md` complete with ≥8 gates + Gate 9 (baseline-aware CI)
- Plan SUMMARY + PR-SECTION exist; umbrella PR body extended
- STATE.md reflects Plan 2 of 9
</verification>

<success_criteria>
- 9 C1 cherry-picks land with verbatim D-19 trailers + Co-Authored-By
- Fork's Phase 36-01b exhaustive match holds; Phase 36-01c canonical name preserved
- Wave 1 sibling Plan 48-03 unaffected (surface-disjoint per Phase 47 § Empirical cross-check)
- REQ-UPST6-02 acceptance criteria #1 satisfied for C1
- Baseline-aware CI gate zero green→red
</success_criteria>

<output>
After completion under `.planning/phases/48-upst6-sync-execution/`:
- `48-02-CLOSE-GATE.md`
- `48-02-SUMMARY.md`
- `48-02-PR-SECTION.md` (appended to umbrella PR body)

STATE.md reflects Phase 48 / Plan 2 of 9 closed.
