---
plan_id: 48-08
plan_name: PACKAGE-MANIFEST
phase: 48
phase_name: upst6-sync-execution
wave: 2
depends_on: [48-02, 48-03]
files_modified:
  - .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md
  - crates/nono-cli/src/package_cmd.rs
  - crates/nono/src/trust/policy.rs
  - crates/nono/src/manifest.rs
  - tests/integration/offline_verify_extended_trust_bundle.rs
autonomous: false
requirements: [REQ-UPST6-02]
cluster: C9
cluster_disposition: fork-preserve
upstream_sha_range: 5f1c9c73..8d774753
upstream_commit_count: 2
baseline_sha: 3f638dc6
tags: [upstream-sync, fork-preserve, upgrade-authority, package, manifest, trust-bundle, wave-2]

must_haves:
  truths:
    - "Phase 47 ledger Cluster C9 (2 commits: 5f1c9c73, 8d774753; disposition fork-preserve with upgrade authority per D-48-C1)"
    - "Diff-inspection artifact 48-08-DISPOSITION-RESOLUTION.md produced BEFORE any code change per D-48-C2 + Convention Pattern E"
    - "Upgrade-or-no-upgrade decision recorded explicitly in 48-08-DISPOSITION-RESOLUTION.md with rationale per D-48-C1"
    - "If upgrade → D-47-D2 re-export scan performed on 5f1c9c73 + 8d774753 BEFORE cherry-pick (per Claude's Discretion bullet in CONTEXT.md)"
    - "If upgrade → cherry-picks carry verbatim D-19 trailers + Co-Authored-By; if no upgrade → D-20 manual-replay commits carry `Upstream-replayed-from:` trailer + Co-Authored-By per Convention Pattern B + checker WARNING reconciliation"
    - "MANDATORY D-48-C3 fork-side regression test for D-32-15 offline-verify-with-extended-schema lands regardless of upgrade-or-not decision (NO D-19 trailer + NO Co-Authored-By since fork-authored)"
    - "Phase 47 DIVERGENCE-LEDGER.md stays as-shipped per D-48-C4 (audit-of-record immutability); resolution recorded in 48-08-DISPOSITION-RESOLUTION.md + 48-SUMMARY.md hand-off"
    - "Windows-only-files invariant honored per D-48-E1"
    - "Cross-target Linux + macOS clippy gates PASS per CLAUDE.md MUST/NEVER"
    - "Baseline-aware CI gate vs SHA 3f638dc6 produces zero green→red lane transitions"
    - "REQ-UPST6-02 acceptance criteria #1 (cherry-pick) OR #3 (D-20 manual-replay) satisfied for C9"
  artifacts:
    - path: ".planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md"
      provides: "Diff-inspection methodology + per-file comparison findings + D-32-15 invariant check + upgrade-or-not verdict per D-48-C2"
    - path: "tests/integration/offline_verify_extended_trust_bundle.rs (or fork's equivalent location)"
      provides: "MANDATORY D-48-C3 regression test exercising D-32-15 offline-verify with .nono-trust.bundle carrying new `installed_path` + `sha256_digest` fields"
    - path: ".planning/phases/48-upst6-sync-execution/48-08-PR-SECTION.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-08-CLOSE-GATE.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-08-SUMMARY.md"
  key_links:
    - from: "git log Wave0-head..HEAD"
      to: "Phase 47 DIVERGENCE-LEDGER.md Cluster C9 row + 48-08-DISPOSITION-RESOLUTION.md verdict"
      via: "Either 2 cherry-pick commits with `^Upstream-commit:` trailers (upgrade path) OR 2 D-20 manual-replay commits with `^Upstream-replayed-from:` trailers (no-upgrade path)"
      pattern: "^(Upstream-commit|Upstream-replayed-from): (5f1c9c73|8d774753)"
    - from: "tests/integration/offline_verify_extended_trust_bundle.rs (or equivalent)"
      to: "D-32-15 offline-verify invariant + .nono-trust.bundle schema"
      via: "Test asserts offline-verify holds when bundle carries installed_path + sha256_digest fields"
      pattern: "installed_path|sha256_digest"
---

<objective>
Handle Phase 47 ledger Cluster C9 (package install path conflict prevention + manifest-based installs; 2 commits in v0.55.0) with the **fork-preserve-with-upgrade-authority** disposition per D-48-C1. This is the only fork-preserve cluster in Phase 48; structurally the most complex per-plan due to:

1. **MANDATORY pre-cherry-pick disposition resolution** (D-48-C2) via separate artifact `48-08-DISPOSITION-RESOLUTION.md` — diff-inspect upstream vs fork's Phase 35 + 45 trust-bundle work; verify D-32-15 offline-verify invariant preserved; verdict: UPGRADE to will-sync OR stay D-20 manual-replay
2. **Conditional cherry-pick path** (if upgrade) OR **D-20 manual-replay path** (if no upgrade) — both with explicit trailer convention per Pattern A vs Pattern B
3. **MANDATORY D-48-C3 regression test** for D-32-15 offline-verify-with-extended-schema regardless of verdict
4. **D-48-C4 ledger immutability** — Phase 47 DIVERGENCE-LEDGER.md stays as-shipped; resolution lives in plan artifacts + 48-SUMMARY hand-off

**Wave gating note (per checker BLOCKER reconciliation):** `depends_on: [48-02, 48-03]` makes the D-48-A2 4-wave SEQUENTIAL model explicit — Wave 2 cannot start until BOTH Wave 1 plans (48-02 and 48-03) close.

Output: 1 disposition-resolution artifact, 2 cherry-picks OR 2 D-20 manual-replays, 1 mandatory regression test, close-gate (with D-48-C3 gate), SUMMARY, PR-SECTION.

**This plan has `autonomous: false`** — the D-48-C1 upgrade-or-no-upgrade decision is a structural call surfaced for explicit verdict in `48-08-DISPOSITION-RESOLUTION.md`. The diff inspection (Task 1) is automatable; the verdict-acknowledgment checkpoint (Task 2) requires human-readable confirmation before Task 3 (cherry-pick or manual-replay) commits.
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
@.planning/phases/43-upst5-sync-execution/43-05-DISPOSITION-RESOLUTION.md
@.planning/phases/43-upst5-sync-execution/43-06-DISPOSITION-RESOLUTION.md
@.planning/phases/43-upst5-sync-execution/43-05-PLATFORM-DETECTION-FOUNDATION-SUMMARY.md
@.planning/phases/43-upst5-sync-execution/43-06-PLATFORM-DETECTION-WINDOWS-SUMMARY.md
@.planning/phases/40-upst4-sync-execution/40-05-FP-PROFILE-SAVE-SUMMARY.md
@.planning/phases/40-upst4-sync-execution/40-06-FP-PROXY-TLS-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
@.planning/templates/cross-target-verify-checklist.md
@CLAUDE.md

<interfaces>
<!-- Fork's Phase 35 / 45 trust-bundle schema (diff-inspection target). -->

From crates/nono/src/trust/policy.rs (HEAD):
```rust
// Fork's trust-bundle schema state at this point includes Phase 35 / 45 additions.
// Upstream 5f1c9c73 adds: installed_path + sha256_digest per artifact in .nono-trust.bundle
// Upstream introduces: validate_bundle_relative_path (defense-in-depth — same intent as fork's existing path-canonicalization helpers)
// Diff-inspection MUST verify no schema collision before upgrade decision.
```

From crates/nono/src/manifest.rs (HEAD):
```rust
// Fork's manifest module — Phase 35 / 45 anchor for manifest-driven install pipeline.
// Upstream 5f1c9c73 introduces: installed_artifact_relative_path helper (centralizes manifest→install-path mapping)
// Removes: infer_artifact_type (superseded by manifest-driven approach)
// Diff-inspection MUST verify no function-removal collision with fork-side callers.
```

From crates/nono-cli/src/package_cmd.rs (HEAD):
```rust
// Fork's package command surface — entry point for `nono package install`
// Diff-inspection target.
```

D-32-15 verify-is-offline invariant (CRITICAL — must not break):
```
Cached trusted_root.json is read via plain JSON deserialization, NOT TUF re-verification.
New .nono-trust.bundle schema fields (installed_path + sha256_digest) MUST NOT break this offline-verify path.
```
</interfaces>
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
    2. Confirm BOTH Wave 1 plans (48-02 and 48-03) closed via SUMMARY status; identify the merged Wave 1 head sha (`WAVE_1_HEAD`).
    3. `git checkout -b phase-48-08-package-manifest $WAVE_1_HEAD`
    4. Verify 2 C9 shas: `for sha in 5f1c9c73 8d774753; do git cat-file -e $sha^{commit} && echo "$sha OK" || echo "$sha MISSING"; done`
    5. Record chronological order: `git log --pretty=format:'%H %ci %s' v0.54.0..v0.57.0 -- crates/nono-cli/src/package_cmd.rs crates/nono/src/trust/policy.rs crates/nono/src/manifest.rs`
  </action>
  <verify>
    <automated>pwd | grep -q "^/c/Users/OMack/Nono$" && git rev-parse phase-48-08-package-manifest &gt;/dev/null 2&gt;&amp;1 && for sha in 5f1c9c73 8d774753; do git cat-file -e $sha^{commit} || exit 1; done; echo "Task 0 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - `pwd` is `/c/Users/OMack/Nono`
    - Both Wave 1 plans confirmed closed
    - Branch off Wave 1 head exists
    - 2 C9 shas resolvable
  </acceptance_criteria>
  <done>Plan branch ready.</done>
</task>

<task type="auto">
  <name>Task 1: D-48-C2 mandatory disposition-resolution artifact</name>
  <files>.planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md rows #8, #9, #10 (package_cmd.rs / trust/policy.rs / manifest.rs LOAD-BEARING invariants)
    - .planning/phases/48-upst6-sync-execution/48-CONTEXT.md § D-48-C1 + D-48-C2 + D-48-C3 + D-48-C4
    - .planning/phases/43-upst5-sync-execution/43-05-DISPOSITION-RESOLUTION.md (TEMPLATE — 9-section body shape to mirror)
    - .planning/phases/43-upst5-sync-execution/43-06-DISPOSITION-RESOLUTION.md (second template — fork-preserve manual-replay precedent)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md § "Cluster C9"
  </read_first>
  <action>
    Produce `.planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md` mirroring Phase 43 `43-05-DISPOSITION-RESOLUTION.md` 9-section shape. Required sections:

    **§ 1. Pre-flight prerequisites** — Plan 48-01 (Wave 0) closed; Plan 48-02 + Plan 48-03 (Wave 1) closed; baseline = `3f638dc6`; branch `phase-48-08-package-manifest` off Wave 1 head.

    **§ 2. D-48-C1 surface-overlap analysis (per file)** — for each of `crates/nono-cli/src/package_cmd.rs`, `crates/nono/src/trust/policy.rs`, `crates/nono/src/manifest.rs`:
    ```bash
    git show 5f1c9c73 -- <file> > /tmp/upstream-${file//\//_}.diff
    git show 8d774753 -- <file> >> /tmp/upstream-${file//\//_}.diff
    ```
    Read the upstream diffs hunk-by-hunk. For each hunk, document:
    - What upstream is adding/changing/removing
    - The corresponding fork-side state (`git show HEAD -- <file>` or `cat <file>`)
    - Whether the upstream change composes additively with fork-side OR collides

    **§ 3. Schema collision check** — focus on `.nono-trust.bundle` schema field set:
    - Upstream adds: `installed_path`, `sha256_digest` (per artifact)
    - Upstream adds: `validate_bundle_relative_path` defense-in-depth function
    - Fork-side state: Phase 35 + 45 trust-bundle schema (grep `crates/nono/src/trust/policy.rs` + `crates/nono/src/manifest.rs` for existing field declarations)
    - Verdict: NO COLLISION (additive compose) OR COLLISION (specific fields conflict — document which)

    **§ 4. D-32-15 verify-is-offline invariant check** — confirm new schema fields MUST NOT break the offline-verify path:
    - Read fork's offline-verify code path (likely `crates/nono-cli/src/setup.rs::trust_refresh` area + `crates/nono/src/trust/` JSON deserialization sites)
    - Verify the new fields parse via Serde's `#[serde(default)]` or `Option<>` semantics (not strict required-field semantics)
    - Verdict: D-32-15 invariant PRESERVED OR BROKEN (specific code path documented)

    **§ 5. Trial cherry-pick evidence (if Verdict so far is UPGRADE)** — speculative cherry-pick to a throwaway branch:
    ```bash
    git stash
    git checkout -b tmp/c9-trial $WAVE_1_HEAD
    git cherry-pick --no-commit 5f1c9c73 2>&1 | tee /tmp/c9-trial-cherry-pick.log
    # If conflicts: document them
    # If clean: cargo build --workspace; cargo test --workspace (record results)
    git checkout phase-48-08-package-manifest
    git branch -D tmp/c9-trial
    git stash pop || true
    ```

    **§ 6. Surface-semantics divergence evidence** — for each upstream change that DOES compose:
    - Does upstream's `validate_bundle_relative_path` have a fork-side analog? If so, is the fork's analog stricter or equivalent? (CLAUDE.md § Security Considerations — choose more restrictive option)
    - Does upstream's `installed_artifact_relative_path` helper subsume any fork-side equivalent function? Document the relationship.

    **§ 7. D-47-D2 re-export scan (CONDITIONAL — only if Verdict = UPGRADE)** — per CONTEXT.md `Claude's Discretion` bullet:
    ```bash
    git show 5f1c9c73 -- crates/nono-cli/src/package_cmd.rs crates/nono/src/trust/policy.rs crates/nono/src/manifest.rs | grep -E '^\+pub use|^\+pub mod|^\+extern crate'
    git show 8d774753 -- crates/nono-cli/src/package_cmd.rs crates/nono/src/trust/policy.rs crates/nono/src/manifest.rs | grep -E '^\+pub use|^\+pub mod|^\+extern crate'
    ```
    Verify no cross-cluster re-export deps (Phase 47 deferred this scan for C9 because it was fork-preserve at audit time).

    **§ 8. Verdict** — explicit ONE OF:
    - **UPGRADE to will-sync** — cherry-pick both commits with D-19 trailers + Co-Authored-By; D-47-D2 re-export scan PASSED
    - **STAY D-20 manual-replay** — replay upstream intent fork-side; preserve fork's defense-in-depth; trailers carry `Upstream-replayed-from:` + Co-Authored-By per template

    Verdict rationale: 2-3 sentences citing § 3 + § 4 verdicts.

    **§ 9. Implications for Task 3** — explicit per-commit disposition:
    - 5f1c9c73 → cherry-pick OR D-20 manual-replay (specific strategy)
    - 8d774753 → cherry-pick OR D-20 manual-replay (specific strategy)

    Then add filename suffix per Claude's Discretion: rename file to `48-08-DISPOSITION-RESOLUTION-UPGRADED.md` (if upgrade) or `48-08-DISPOSITION-RESOLUTION-DEFERRED.md` (if no upgrade) at plan close. For now, keep bare name during inspection.
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md && grep -cE '^### [1-9]\. ' .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md | awk '{exit ($1&gt;=8)?0:1}' && grep -qE 'Verdict.*(UPGRADE|D-20 manual-replay)' .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md && echo "DISPOSITION-RESOLUTION present with verdict"</automated>
  </verify>
  <acceptance_criteria>
    - File `.planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md` exists with ≥8 numbered sections
    - § 3 schema-collision check has explicit verdict
    - § 4 D-32-15 invariant check has explicit verdict
    - § 7 re-export scan executed if verdict is UPGRADE
    - § 8 verdict is explicit ONE OF: `UPGRADE to will-sync` OR `STAY D-20 manual-replay`
    - § 9 per-commit disposition explicit for both 5f1c9c73 and 8d774753
  </acceptance_criteria>
  <done>Disposition-resolution artifact complete with explicit verdict. Task 2 acknowledgment checkpoint follows.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 2: Human verdict acknowledgment (D-48-C1 upgrade-or-no-upgrade decision)</name>
  <what-built>
    `48-08-DISPOSITION-RESOLUTION.md` artifact complete (Task 1) with explicit verdict in § 8. The verdict drives Task 3's commit path (cherry-pick vs D-20 manual-replay), so the human reviews the diff-inspection findings + verdict rationale before Task 3 commits.
  </what-built>
  <how-to-verify>
    1. Read `.planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md` § 3 (schema collision), § 4 (D-32-15 invariant), § 8 (verdict), § 9 (per-commit disposition).
    2. Confirm the verdict matches your reading of upstream's intent + fork's existing schema state. Specifically:
       - § 3 verdict: does fork's `.nono-trust.bundle` schema (Phase 35 / 45 state) compose with upstream's `installed_path` + `sha256_digest` additions cleanly, or are there fork-divergent field declarations that collide?
       - § 4 verdict: does the offline-verify code path (D-32-15 invariant) tolerate the new schema fields without breaking? Are the fields parsed with `Option<>` or `#[serde(default)]` semantics, or strict required-field semantics?
    3. If you agree with the verdict, type "approved-upgrade" or "approved-defer" (matching § 8 verdict).
    4. If you disagree or want more diff-inspection: type "more-inspection-needed" and describe what's missing.

    Expected outcome: explicit human approval of the verdict, OR redirect to additional Task 1 sub-inspection.
  </how-to-verify>
  <resume-signal>Type "approved-upgrade" OR "approved-defer" OR "more-inspection-needed"</resume-signal>
</task>

<task type="auto">
  <name>Task 3: Apply the verdict — cherry-pick (UPGRADE) OR D-20 manual-replay (DEFER)</name>
  <files>
    - crates/nono-cli/src/package_cmd.rs
    - crates/nono/src/trust/policy.rs
    - crates/nono/src/manifest.rs
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md § 8 + § 9 (verdict + per-commit disposition)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern A" (D-19 trailer for upgrade path)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern B" (D-20 manual-replay trailer + 5-section body for defer path)
    - .planning/phases/43-upst5-sync-execution/43-05-PLATFORM-DETECTION-FOUNDATION-PLAN.md lines 228 + 356 (D-20 manual-replay commit body template — 5-section body: Upstream intent / What was replayed / What was NOT replayed and why / Fork-only wiring preserved / Upstream-replayed-from)
    - .planning/templates/upstream-sync-quick.md § D-19 trailer block (5-line trailer + Co-Authored-By line per template)
    - CLAUDE.md § Commits (DCO)
  </read_first>
  <action>
    Based on Task 2 verdict, execute ONE of two paths:

    **PATH A — UPGRADE to will-sync** (if verdict = `approved-upgrade`):

    Cherry-pick the 2 C9 commits in chronological order. Both v0.55.0 per Phase 47 ledger.

    **C9-01: `5f1c9c73`** — `refactor(package): base installs on package manifest` (2 files; categories: `other,package`)
    **C9-02: `8d774753`** — `feat(package): prevent artifact install path conflicts` (1 file; categories: `package`)

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
       Upstream-categories: <package or other,package per ledger row>
       Co-Authored-By: <name> <email>
       ```
       Same upstream author name+email used for BOTH `Upstream-author:` and `Co-Authored-By:` lines per template convention.
    4. `git cherry-pick --no-commit $FULL_SHA`
    5. Resolve any predicted conflicts per `48-08-DISPOSITION-RESOLUTION.md` § 9
    6. `git commit -F <trailer-file>` (with DCO `Signed-off-by:` AFTER the 7-line trailer block)
    7. Per-commit verify: 7-line trailer + DCO; `cargo build --workspace`
    8. Windows invariant: 0 violations

    **PATH B — STAY D-20 manual-replay** (if verdict = `approved-defer`):

    Author 2 fork-side manual-replay commits implementing upstream's intent while preserving fork's defense-in-depth. For each commit:

    1. Read upstream `git show <sha>` to understand intent + extract author name+email for Co-Authored-By attribution
    2. Implement the same end-behavior using fork's existing helpers (e.g., fork's `validate_bundle_relative_path` analog) where they're stricter; substitute upstream's helper otherwise
    3. Author commit body per Convention Pattern B 5-section shape + Co-Authored-By per checker WARNING reconciliation (D-20 manual-replay path still attributes to original upstream author per template convention):
       ```
       <fork-side commit subject mirroring upstream intent, e.g. "refactor(48-08): manifest-driven install pipeline (replay of 5f1c9c73)">

       <fork-side commit body>

       Upstream intent: <what upstream's commit accomplishes>
       What was replayed: <fork-side hunks that implement same intent>
       What was NOT replayed and why: <upstream pieces deferred + rationale>
       Fork-only wiring preserved: <fork-side defense-in-depth callouts>
       Upstream-replayed-from: <40-char sha>
       Co-Authored-By: <upstream author name> <upstream author email>

       Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
       ```
    4. `git commit -F <body-file>`
    5. Per-commit verify: `git log -1 --format=%B HEAD | grep -E '^Upstream-replayed-from: [0-9a-f]{40}$'` AND `git log -1 --format=%B HEAD | grep -cE '^Upstream-commit: '` equals `0` (per Convention Pattern B falsifiability) AND `git log -1 --format=%B HEAD | grep -E '^Co-Authored-By: '` returns one line
    6. `cargo build --workspace`; Windows invariant 0 violations

    Whichever path executed, record per-commit notes in plan SUMMARY.
  </action>
  <verify>
    <automated>WAVE1=$(git merge-base HEAD phase-48-02-profile-shadowing); UPGRADE_COUNT=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'); DEFER_COUNT=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Upstream-replayed-from: [0-9a-f]{40}$'); TOTAL=$((UPGRADE_COUNT + DEFER_COUNT)); test "$TOTAL" = "2" && COAUTH=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Co-Authored-By: '); test "$COAUTH" = "2" && WIN=$(git diff --name-only $WAVE1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l); test "$WIN" = "0" && cargo build --workspace 2&gt;&amp;1 | tail -1 | grep -q "^Finished" && echo "Task 3 PASS (upgrade=$UPGRADE_COUNT, defer=$DEFER_COUNT, total=$TOTAL, coauth=$COAUTH)"</automated>
  </verify>
  <acceptance_criteria>
    - Exactly 2 new commits between Wave 1 head and plan-head (NOT counting D-48-C3 regression test from Task 4)
    - Either ALL 2 commits have `^Upstream-commit:` trailers (UPGRADE path) OR ALL 2 commits have `^Upstream-replayed-from:` trailers (DEFER path) — not mixed without explicit rationale in SUMMARY § Per-commit notes
    - Both commits have `^Co-Authored-By: ` line attributing original upstream author (per checker WARNING reconciliation; applies to BOTH upgrade + defer paths since both replicate upstream intent)
    - Every commit has `^Signed-off-by:` DCO line
    - Windows invariant 0 violations
    - `cargo build --workspace` exits 0
  </acceptance_criteria>
  <done>2 C9 commits landed per verdict (with Co-Authored-By attribution on both upgrade + defer paths).</done>
</task>

<task type="auto" tdd="true">
  <name>Task 4: D-48-C3 mandatory regression test for D-32-15 offline-verify-with-extended-schema</name>
  <files>
    - tests/integration/offline_verify_extended_trust_bundle.rs (OR fork's equivalent location — verify via `ls tests/integration/ crates/nono/tests/ crates/nono-cli/tests/`)
  </files>
  <behavior>
    - Test 1: When `.nono-trust.bundle` carries the new `installed_path` + `sha256_digest` fields per artifact, fork's D-32-15 offline-verify path (cached `trusted_root.json` plain JSON deserialization, NOT TUF re-verification) succeeds — i.e., the extra schema fields do NOT cause a parse failure or short-circuit verification.
    - Test 2: When `.nono-trust.bundle` carries ONLY the legacy fork-side fields (NO `installed_path`/`sha256_digest`), fork's D-32-15 offline-verify still succeeds (backwards compatibility — for the case where fork still produces pre-upgrade bundles).
    - Test 3 (upgrade path only): if a bundle has an invalid `installed_path` value (e.g., a `..` traversal or absolute path), fork's `validate_bundle_relative_path` defense-in-depth REJECTS the bundle with a clear error message (verifies that the security-critical validator from upstream lands intact OR fork's equivalent helper remains the gating call site).
  </behavior>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-CONTEXT.md § D-48-C3 (mandatory regardless of upgrade-or-not decision)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #8 + row #9 (package_cmd.rs + trust/policy.rs invariants)
    - .planning/phases/32-* (or wherever D-32-15 verify-is-offline invariant was established — search via `grep -rn "D-32-15" .planning/`)
    - Existing fork-side test patterns: `ls tests/integration/ crates/nono/tests/ crates/nono-cli/tests/ 2>/dev/null`
    - CLAUDE.md § Coding Standards (no `.unwrap()`, env-var save/restore in tests)
  </read_first>
  <action>
    Author `tests/integration/offline_verify_extended_trust_bundle.rs` (or fork's equivalent location — verify the right path via `ls tests/integration/` first):

    ```rust
    // Test 1: extended bundle (installed_path + sha256_digest) passes offline-verify
    // Test 2: legacy bundle (no installed_path/sha256_digest) still passes offline-verify (backwards compat)
    // Test 3 (only if upgrade path): invalid installed_path values get rejected by validate_bundle_relative_path
    ```

    Use existing fork-side trust-bundle fixture patterns (search via `grep -rn 'trust-root-frozen.json\\|\\.nono-trust\\.bundle' crates/`); construct extended-schema test fixtures inline (string literals containing JSON) so the test is hermetic.

    Verify D-32-15 invariant by exercising the offline-verify code path explicitly (not the TUF re-verification path).

    Commit the regression test as a fork-side test commit (NO D-19 trailer per D-48-C3 — this is fork-authored regression coverage, not an upstream cherry-pick; NO `Co-Authored-By:` line either since there is no upstream author to attribute):
    ```bash
    git add tests/integration/offline_verify_extended_trust_bundle.rs
    cargo test --workspace --test offline_verify_extended_trust_bundle # confirm new tests pass
    git commit -s -m "test(48-08): D-48-C3 regression coverage for offline-verify with extended trust-bundle schema" \
               -m "Mandatory D-48-C3 regression test exercising D-32-15 verify-is-offline invariant when .nono-trust.bundle carries new installed_path + sha256_digest fields. Coverage applies regardless of Task 3 upgrade-or-defer verdict per D-48-C3 (codifies the invariant against future drift). NO D-19 trailer + NO Co-Authored-By (fork-authored regression test, not an upstream cherry-pick)."
    ```
  </action>
  <verify>
    <automated>git log -1 --format=%s HEAD | grep -q "^test(48-08):" &amp;&amp; ! git log -1 --format=%B HEAD | grep -qE '^Upstream-commit: ' &amp;&amp; git log -1 --format=%B HEAD | grep -qE '^Signed-off-by: ' &amp;&amp; cargo test --workspace 2&gt;&amp;1 | tail -3 | grep -q "test result: ok" && echo "D-48-C3 regression test landed"</automated>
  </verify>
  <acceptance_criteria>
    - HEAD commit subject starts `test(48-08):`
    - HEAD commit body has DCO sign-off
    - HEAD commit body does NOT have `Upstream-commit:` trailer (per D-48-C3 — fork-authored)
    - HEAD commit body does NOT have `Co-Authored-By:` line (per D-48-C3 — fork-authored, no upstream author to attribute)
    - `cargo test --workspace` exits 0 with new tests included (record per-test pass status: at least 2, ideally 3 per Task 4 behavior block)
    - Test exercises D-32-15 offline-verify code path explicitly (verify by inspection of test code)
  </acceptance_criteria>
  <done>D-48-C3 mandatory regression test landed; D-32-15 invariant codified against future drift.</done>
</task>

<task type="auto">
  <name>Task 5: Plan 48-08 close-gate (Convention Pattern G) + D-48-C3 regression test gate</name>
  <files>.planning/phases/48-upst6-sync-execution/48-08-CLOSE-GATE.md</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern G"
    - .planning/templates/cross-target-verify-checklist.md
  </read_first>
  <action>
    Produce `48-08-CLOSE-GATE.md` with 8 standard gates per Convention Pattern G PLUS Gate 9 (D-48-C3 regression test):
    - Gates 1-8 standard
    - Gate 3+4: cross-target Linux + macOS clippy MANDATORY
    - **Gate 9 (D-48-C3 MANDATORY):** `cargo test --workspace --test offline_verify_extended_trust_bundle` (or per fork's test convention) — must PASS regardless of upgrade-or-defer verdict
    - Gate 10: baseline-aware CI gate (Convention Pattern H — bumped to Gate 10 due to D-48-C3 inserting new gate)
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-08-CLOSE-GATE.md && grep -cE '^### Gate [1-9]' .planning/phases/48-upst6-sync-execution/48-08-CLOSE-GATE.md | awk '{exit ($1&gt;=9)?0:1}' && grep -qE 'offline_verify_extended_trust_bundle|D-48-C3' .planning/phases/48-upst6-sync-execution/48-08-CLOSE-GATE.md && echo "CLOSE-GATE present with D-48-C3 gate"</automated>
  </verify>
  <acceptance_criteria>
    - File exists with ≥9 gate sections
    - Gate 9 explicitly references D-48-C3 regression test
    - Skipped-gate categorization explicit
  </acceptance_criteria>
  <done>Close-gate complete with D-48-C3 add.</done>
</task>

<task type="auto">
  <name>Task 6: Baseline-aware CI gate vs SHA 3f638dc6</name>
  <files>(no file changes)</files>
  <read_first>
    - .planning/templates/upstream-sync-quick.md (lines 96-112)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern H"
  </read_first>
  <action>
    Push `phase-48-08-package-manifest` to fork's `pre-merge`; wait for GH Actions; categorize lanes vs `3f638dc6`. Record in `48-08-CLOSE-GATE.md` final gate. ZERO green→red.
  </action>
  <verify>
    <automated>gh run list --branch pre-merge --limit 1 --json conclusion --jq '.[0].conclusion' | grep -qE '^(success|failure)$' && grep -qE '^### Gate (9|10)' .planning/phases/48-upst6-sync-execution/48-08-CLOSE-GATE.md && echo "Baseline-aware CI recorded"</automated>
  </verify>
  <acceptance_criteria>
    - Zero green→red lane transitions
    - Final gate has per-lane verdict
  </acceptance_criteria>
  <done>Baseline-aware CI complete.</done>
</task>

<task type="auto">
  <name>Task 7: SUMMARY + PR section + STATE update + close-doc commit + filename suffix on disposition artifact</name>
  <files>
    - .planning/phases/48-upst6-sync-execution/48-08-SUMMARY.md
    - .planning/phases/48-upst6-sync-execution/48-08-PR-SECTION.md
    - .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION{-UPGRADED,-DEFERRED}.md (rename per Claude's Discretion)
    - .planning/STATE.md
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md (template)
    - .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md § 8 (verdict — determines filename suffix)
  </read_first>
  <action>
    1. Per Claude's Discretion bullet in CONTEXT.md: rename `48-08-DISPOSITION-RESOLUTION.md` to one of:
       - `48-08-DISPOSITION-RESOLUTION-UPGRADED.md` (if Task 2 verdict was upgrade)
       - `48-08-DISPOSITION-RESOLUTION-DEFERRED.md` (if Task 2 verdict was defer)
       Use `git mv` so git tracks the rename: `git mv .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION.md .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION-<suffix>.md`
    2. Author `48-08-SUMMARY.md` (frontmatter: `cluster: C9`, `cluster_disposition: <fork-preserve-upgraded|fork-preserve-deferred>` per verdict, `upstream_sha_range: 5f1c9c73..8d774753`, `upstream_commit_count: 2`, `baseline_sha: 3f638dc6`, `lane_transitions:`, `skipped_gates_*:`, `d_48_c1_verdict: <upgrade|defer>`, `d_48_c3_regression_test_status: <pass|partial>`, `disposition_resolution_artifact: 48-08-DISPOSITION-RESOLUTION-<suffix>.md`, `pr_section:`) and sections:
       - § D-48-C1 verdict + rationale (cite `48-08-DISPOSITION-RESOLUTION-<suffix>.md` § 8)
       - § Per-commit notes (cherry-pick OR manual-replay per Task 3; document Co-Authored-By upstream attribution)
       - § D-48-C3 regression test (Task 4 — explicit mention that it landed regardless of verdict)
       - § Baseline-aware CI verdict
       - § Phase 47 ledger immutability note (per D-48-C4 — Phase 47 ledger stays as-shipped; resolution lives here + hand-off to Phase 48 SUMMARY)
    3. Author `48-08-PR-SECTION.md` per Convention Pattern I — key decisions include D-48-C1 verdict + D-48-C3 regression test landing.
    4. Append to umbrella PR body.
    5. Update STATE.md (Plan 8 of 9).
    6. Commit:
       ```bash
       git add .planning/phases/48-upst6-sync-execution/48-08-*.md .planning/STATE.md
       git commit -s -m "docs(48-08): close cluster C9 (package manifest + trust-bundle schema; verdict: <upgrade|defer>)" \
                  -m "C9 fork-preserve cluster resolved per D-48-C1 with verdict <upgrade|defer>; both Task 3 commits carry Co-Authored-By upstream attribution per template; D-48-C3 mandatory regression test for D-32-15 offline-verify-with-extended-schema landed (no Co-Authored-By since fork-authored); disposition-resolution artifact renamed per Claude's Discretion; STATE.md advanced; umbrella PR body appended."
       ```
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-08-SUMMARY.md && test -f .planning/phases/48-upst6-sync-execution/48-08-PR-SECTION.md && grep -q "cluster: C9" .planning/phases/48-upst6-sync-execution/48-08-SUMMARY.md && grep -q "d_48_c1_verdict:" .planning/phases/48-upst6-sync-execution/48-08-SUMMARY.md && grep -q "d_48_c3_regression_test_status:" .planning/phases/48-upst6-sync-execution/48-08-SUMMARY.md && (test -f .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION-UPGRADED.md || test -f .planning/phases/48-upst6-sync-execution/48-08-DISPOSITION-RESOLUTION-DEFERRED.md) && git log -1 --format=%s HEAD | grep -q "^docs(48-08):" && echo "Plan 48-08 closed"</automated>
  </verify>
  <acceptance_criteria>
    - SUMMARY + PR-SECTION exist
    - SUMMARY frontmatter has `d_48_c1_verdict:` (upgrade/defer) + `d_48_c3_regression_test_status:` (pass/partial)
    - Disposition-resolution artifact renamed with suffix matching verdict
    - STATE.md reflects Plan 8 of 9
    - Close-doc commit subject `docs(48-08):` + DCO
  </acceptance_criteria>
  <done>Plan 48-08 closed; Phase 47 ledger immutability preserved per D-48-C4; resolution captured in artifacts + hand-off to Phase 48 SUMMARY.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

C9 is the most security-critical cluster of Phase 48 — it touches trust-bundle schema (`.nono-trust.bundle`) which gates `nono trust verify` operations. The boundaries:

| Boundary | Description |
|----------|-------------|
| Trust-bundle producer → trust-bundle consumer | Upstream 5f1c9c73 extends bundle schema with `installed_path` + `sha256_digest`; fork-side consumers (offline verify path per D-32-15) MUST tolerate the extension without breaking |
| User-controlled installed_path field → filesystem | Without `validate_bundle_relative_path` defense-in-depth, an attacker-controlled `installed_path` could traverse outside the package install root (e.g., `../../etc/passwd`); upstream's helper + fork's equivalent gate this |
| Cached `trusted_root.json` → offline verify | D-32-15 invariant: cached TUF root is read via plain JSON deserialization, NOT TUF re-verification; new bundle fields MUST NOT break this path (D-48-C3 regression test codifies) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-48-08-01 | Tampering | Attacker-controlled `installed_path` field traversing outside package install root | mitigate | `validate_bundle_relative_path` (upstream OR fork analog per Task 1 § 6) gates the value; D-48-C3 regression test (Task 4) Test 3 exercises invalid path rejection (only if upgrade path; if defer, fork's existing analog gates) |
| T-48-08-02 | Tampering | Schema extension breaks fork's D-32-15 offline-verify (cached `trusted_root.json` plain JSON deserialization) | mitigate | Task 1 § 4 explicit invariant check; D-48-C3 regression test (MANDATORY regardless of verdict) Test 1 + Test 2 codify backwards compat + extension tolerance |
| T-48-08-03 | Information Disclosure | `sha256_digest` field could leak artifact identity to bundle consumers | accept | Digest is intentional integrity metadata (security feature, not leak); upstream-tested |
| T-48-08-04 | Spoofing | If upgrade path drops fork's existing `validate_bundle_relative_path` analog in favor of upstream's, fork's stricter posture (per CLAUDE.md § Security Considerations) could regress | mitigate | Task 1 § 6 surface-semantics divergence evidence MUST verify fork's analog (if any) is preserved or replaced by an upstream equivalent of equal-or-stricter strength; D-20 manual-replay path (DEFER) explicitly preserves fork-side analog |
| T-48-08-05 | Elevation of Privilege | Manifest-driven install pipeline (`installed_artifact_relative_path` helper) could be misused if fork-side callers pass user-controlled values | mitigate | Helper centralizes the mapping (less surface for callers to misuse); fork-side callers verified in Task 1 § 6 surface-semantics analysis |

**Fork-side defense-in-depth preserved (per PATTERNS.md rows #8, #9, #10):**
- D-32-15 verify-is-offline invariant codified via D-48-C3 regression test (REGARDLESS of upgrade-or-defer verdict)
- CLAUDE.md § Path Handling — path component comparison, never string `starts_with()`, canonicalize before validation
- CLAUDE.md § Security Considerations — choose more restrictive option when in doubt (Task 1 § 6 codifies for fork analog comparison)
- Phase 35 + 45 trust-bundle work composes additively (Task 1 § 3 verifies)
</threat_model>

<verification>
- `48-08-DISPOSITION-RESOLUTION.md` (or renamed `-UPGRADED.md`/`-DEFERRED.md`) exists with ≥8 sections + explicit § 8 verdict + § 9 per-commit disposition
- Exactly 2 commits between Wave 1 head and plan-head (NOT counting D-48-C3 regression test or close-doc commit): either both have `^Upstream-commit:` (UPGRADE) OR both have `^Upstream-replayed-from:` (DEFER); BOTH paths carry `^Co-Authored-By: ` upstream attribution per checker WARNING reconciliation
- D-48-C3 regression test commit subject `test(48-08):` + DCO + NO `Upstream-commit:` trailer + NO `Co-Authored-By:` line (fork-authored); `cargo test --workspace --test offline_verify_extended_trust_bundle` exits 0
- Windows invariant 0 violations across all commits
- Cross-target Linux + macOS clippy PASS (or PARTIAL `_environmental`)
- Close-gate ≥9 gates (Gate 9 = D-48-C3) + Gate 10 (baseline-aware CI)
- Zero green→red lane transitions
- Phase 47 ledger UNCHANGED (`git diff <ledger-baseline> .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md` returns no output — per D-48-C4 audit-of-record immutability)
- Plan SUMMARY frontmatter has explicit `d_48_c1_verdict:` + `d_48_c3_regression_test_status:` fields
</verification>

<success_criteria>
- REQ-UPST6-02 acceptance criteria #1 (cherry-pick path) OR #3 (D-20 manual-replay path) satisfied for C9
- D-48-C3 mandatory regression test lands regardless of verdict (codifies D-32-15 invariant against future drift)
- D-48-C4 Phase 47 ledger immutability preserved
- Wave 2 partial complete (all 5 Wave 2 plans close)
- Phase 48 SUMMARY hand-off (Plan 48-09 close artifact) will record C9 final disposition + rationale per D-48-C4
</success_criteria>

<output>
After completion:
- `48-08-DISPOSITION-RESOLUTION-{UPGRADED,DEFERRED}.md` (renamed at plan close)
- `tests/integration/offline_verify_extended_trust_bundle.rs` (or fork's equivalent location)
- `48-08-CLOSE-GATE.md`
- `48-08-SUMMARY.md`
- `48-08-PR-SECTION.md`

STATE.md reflects Plan 8 of 9.
