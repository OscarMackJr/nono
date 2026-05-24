---
plan_id: 48-01
plan_name: LANDLOCK-V6-AF-UNIX
phase: 48
phase_name: upst6-sync-execution
wave: 0
depends_on: []
files_modified:
  - .planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md
  - crates/nono/src/sandbox/linux.rs
  - crates/nono/src/sandbox/mod.rs
  - crates/nono/src/lib.rs
  - crates/nono/src/capability.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/query_ext.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/setup.rs
  - crates/nono-cli/src/supervised_runtime.rs
  - crates/nono-cli/src/why_runtime.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
autonomous: false
requirements: [REQ-UPST6-02]
cluster: C4
cluster_disposition: will-sync
upstream_sha_range: c2c6f2ca..863bbfd3
upstream_commit_count: 9
baseline_sha: 3f638dc6
tags: [upstream-sync, cherry-pick, landlock, af-unix, linux, foundation, wave-0]

must_haves:
  truths:
    - "Phase 47 ledger Cluster C4 (9 commits: c2c6f2ca, b8a32006, 858ad009, bbc652a0, 1e9385a7, 98f8cb18, d146001b, a0222be2, 863bbfd3) cherry-picked into fork in upstream-chronological order"
    - "Every cherry-picked commit carries the verbatim D-19 6-line trailer block per D-48-E2 + Convention Pattern A AND a `Co-Authored-By: <upstream author>` line per upstream-sync-quick.md template (per checker WARNING reconciliation)"
    - "PR umbrella opens to upstream/main after Plan 48-01 close per D-48-A4 (title: nono: upstream v0.55.0..v0.57.0 sync (Phase 48))"
    - "Pre-flight diff-inspection artifact 48-01-PRE-CHERRY-PICK-AUDIT.md exists with conflict-prediction table + chosen resolution strategy per commit per D-48-B2 + Convention Pattern D"
    - "Windows-only-files invariant honored per D-48-E1 (zero windows-touch in C4 per Phase 47 ledger row 'C4 | ... | no')"
    - "Baseline-aware CI gate vs SHA 3f638dc6 produces zero green→red lane transitions per D-48-E3 + Convention Pattern H"
    - "Cross-target clippy gates 3+4 PASS (or PARTIAL with _environmental categorization) per D-48-E4 + Convention Pattern J (C4 is #[cfg(target_os = linux)]-gated)"
    - "REQ-UPST6-02 acceptance criterion #1 (will-sync cluster cherry-picked with verbatim D-19 trailers) satisfied for C4"
    - "REQ-UPST6-02 acceptance criterion #4 (baseline-aware CI gate zero success→failure) satisfied for Plan 48-01 head commit"
  artifacts:
    - path: ".planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md"
      provides: "Per-commit conflict prediction + per-file diff inspection + chosen resolution strategy per D-48-B2"
      contains: "## Per-commit conflict-prediction table"
    - path: ".planning/phases/48-upst6-sync-execution/48-01-PR-SECTION.md"
      provides: "Per-plan umbrella PR contribution section per D-48-E6 + Convention Pattern I"
    - path: ".planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md"
      provides: "8-check close-gate matrix per D-48-E9 + Convention Pattern G"
    - path: ".planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md"
      provides: "Plan close summary including lane transitions, skipped-gate categorization, escalation outcome (if any), C4 cluster final disposition"
  key_links:
    - from: "git log HEAD~9..HEAD"
      to: "Phase 47 DIVERGENCE-LEDGER.md Cluster C4 row"
      via: "9 cherry-pick commits with Upstream-commit: trailers matching 9 upstream shas"
      pattern: "^Upstream-commit: (c2c6f2ca|b8a32006|858ad009|bbc652a0|1e9385a7|98f8cb18|d146001b|a0222be2|863bbfd3)"
    - from: "crates/nono/src/lib.rs"
      to: "crates/nono/src/sandbox/mod.rs"
      via: "pub use sandbox::{DetectedAbi, LandlockScopePolicy, detect_abi, is_wsl2, landlock_scope_policy};"
      pattern: "pub use sandbox::\\{[^}]*LandlockScopePolicy"
    - from: "crates/nono/src/sandbox/mod.rs"
      to: "crates/nono/src/sandbox/linux.rs"
      via: "pub use linux::{DetectedAbi, LandlockScopePolicy, detect_abi, landlock_scope_policy};"
      pattern: "pub use linux::\\{[^}]*LandlockScopePolicy"
---

<objective>
Cherry-pick Phase 47 ledger Cluster C4 (Linux Landlock v6 signal/socket scoping + af_unix pathname mediation; 9 commits in `v0.55.0`) into the fork in upstream-chronological order, with verbatim D-19 6-line trailer blocks per Convention Pattern A. This is the Phase 48 **foundation gate** (Wave 0): Wave 1 plans 48-02 (C1) + 48-03 (C2) share `cli.rs` and `profile/mod.rs` with this plan and depend on its close. After this plan closes, the upstream PR umbrella opens per D-48-A4.

Purpose: Absorb the largest cluster of the cycle while it sits on a clean baseline; surface any conflict pressure via the mandatory pre-flight diff-inspection artifact (D-48-B2) so subsequent waves run on a verified foundation.

Output:
- 9 fork-side cherry-pick commits with verbatim D-19 trailers + Co-Authored-By upstream attribution (one per upstream sha)
- `.planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md` (pre-flight artifact)
- Per-plan close-gate matrix `48-01-CLOSE-GATE.md`
- Per-plan PR contribution section `48-01-PR-SECTION.md`
- Plan summary `48-01-SUMMARY.md`
- New upstream PR umbrella (D-48-A4)

Escalation: if pre-flight surfaces irreconcilable conflicts on specific commits, planner splits into `48-01a-...-PLAN.md` (cleanly-resolvable commits) + `48-01b-...-PLAN.md` (deferred per D-48-B3 + Convention Pattern F).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/48-upst6-sync-execution/48-CONTEXT.md
@.planning/phases/48-upst6-sync-execution/48-PATTERNS.md
@.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md
@.planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/47-01-SUMMARY.md
@.planning/phases/43-upst5-sync-execution/43-02-PRE-CHERRY-PICK-AUDIT.md
@.planning/phases/43-upst5-sync-execution/43-01-EDITION-2024-FOUNDATION-SUMMARY.md
@.planning/phases/43-upst5-sync-execution/43-01b-EDITION-WORKSPACE-ONLY-SUMMARY.md
@.planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md
@.planning/phases/36-upst3-deep-closure/36-01c-OVERRIDE-DENY-RENAME-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
@.planning/templates/cross-target-verify-checklist.md
@CLAUDE.md

<interfaces>
<!-- Fork-side re-export surface for sandbox types (will be EXTENDED by c2c6f2ca cherry-pick).
     Executor must verify intra-cluster origin per Phase 47 D-47-D2 discipline. -->

From crates/nono/src/sandbox/mod.rs (pre-cherry-pick HEAD):
```rust
// Current: facade re-exports for is_supported() + support_info()
// After c2c6f2ca cherry-pick: + pub use linux::{DetectedAbi, LandlockScopePolicy, detect_abi, landlock_scope_policy};
```

From crates/nono/src/lib.rs (pre-cherry-pick HEAD):
```rust
// Current: flat public API re-exports
// After c2c6f2ca cherry-pick: + pub use sandbox::{DetectedAbi, LandlockScopePolicy, detect_abi, is_wsl2, landlock_scope_policy};
```

From crates/nono-cli/src/profile/mod.rs (line ~2068):
```rust
// CRITICAL: Phase 36-01b exhaustive `impl From<ProfileDeserialize> for Profile`
// includes CommandsConfig + FilesystemConfig.deny/bypass_protection + LegacyPolicyPatch
// + DeprecationCounter canonical sections.
// Upstream a0222be2 (af_unix mediation profile config) MAY add a profile field that
// requires extending the exhaustive match. If so, the extension MUST happen in the
// same cherry-pick commit body (NOT a separate commit — preserves D-19 trailer fidelity).
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 0: Wave-merge CWD hygiene + branch baseline</name>
  <files>(no file changes; pre-flight discipline only)</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Shared Pre-flight Discipline" item 1 (`feedback_windows_worktree_cwd` recurrence prevention)
    - MEMORY: feedback_windows_worktree_cwd (Windows worktree CWD divergence)
  </read_first>
  <action>
    Verify pre-flight state before any cherry-pick begins:
    1. `cd /c/Users/OMack/Nono` (mandatory per feedback_windows_worktree_cwd)
    2. `pwd` MUST print `/c/Users/OMack/Nono`
    3. `git rev-parse --abbrev-ref HEAD` — record current branch
    4. Create plan-feature branch `phase-48-01-landlock-v6-af-unix` off the v2.6 baseline (Phase 46 close baseline SHA `3f638dc6`): `git checkout -b phase-48-01-landlock-v6-af-unix 3f638dc6`
    5. `git fetch upstream --tags` (refresh upstream tags; D-47-A1 range `v0.54.0..v0.57.0` must be locally resolvable)
    6. `git rev-parse upstream/main` — record upstream HEAD (informational; cherry-picks target the 9 specific shas, not HEAD)
    7. Verify each of the 9 C4 shas exists locally: `for sha in c2c6f2ca b8a32006 858ad009 bbc652a0 1e9385a7 98f8cb18 d146001b a0222be2 863bbfd3; do git cat-file -e $sha^{commit} && echo "$sha OK" || echo "$sha MISSING"; done`
  </action>
  <verify>
    <automated>pwd | grep -q "^/c/Users/OMack/Nono$" && git rev-parse phase-48-01-landlock-v6-af-unix &gt;/dev/null 2>&1 && for sha in c2c6f2ca b8a32006 858ad009 bbc652a0 1e9385a7 98f8cb18 d146001b a0222be2 863bbfd3; do git cat-file -e $sha^{commit} || exit 1; done; echo "Task 0 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - `pwd` prints `/c/Users/OMack/Nono` exactly
    - Branch `phase-48-01-landlock-v6-af-unix` exists and is checked out
    - `git rev-parse phase-48-01-landlock-v6-af-unix` matches `git rev-parse 3f638dc6` (clean baseline)
    - All 9 C4 commit shas exist locally (`git cat-file -e <sha>^{commit}` exits 0 for each)
    - Recorded branch + upstream HEAD in shell scrollback for SUMMARY frontmatter
  </acceptance_criteria>
  <done>Working tree clean, on plan branch, all 9 C4 shas resolvable. Ready for pre-flight diff-inspection.</done>
</task>

<task type="auto">
  <name>Task 1: Pre-flight diff-inspection artifact (D-48-B2)</name>
  <files>.planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md</files>
  <read_first>
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md § "Cluster C4" (rows for 9 shas)
    - .planning/phases/47-upst6-audit-v0-41-v0-43-drift-ingestion/DIVERGENCE-LEDGER.md § "Empirical cross-check" File #4 (profile/mod.rs hot-spot finding) + File #5 (cli.rs)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md rows #1, #2, #3, #4, #5, #6, #7 (every C4 integration-point file)
    - .planning/phases/43-upst5-sync-execution/43-02-PRE-CHERRY-PICK-AUDIT.md (TEMPLATE — copy 7-section body shape verbatim: Wave 0a closure → Upstream commit shape verification → Upstream diff shape → Fork-side divergence audit → Audit verdict → Acceptance summary)
    - .planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md (fork's exhaustive `From<ProfileDeserialize>` match arm details)
    - .planning/phases/36-upst3-deep-closure/36-01c-OVERRIDE-DENY-RENAME-SUMMARY.md (canonical `bypass_protection` name)
    - .planning/templates/upstream-sync-quick.md (D-19 trailer block reference)
  </read_first>
  <action>
    Produce `.planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md` mirroring Phase 43 `43-02-PRE-CHERRY-PICK-AUDIT.md` shape. Required sections:

    **§ 1. Pre-flight closure** — confirm Task 0 done; record branch sha + upstream/main sha at fetch time.

    **§ 2. Upstream-chronological cherry-pick order verification (D-48-B1 + Claude's Discretion bullet)** — Run:
    ```bash
    git log --pretty=format:'%H %ci %s' v0.54.0..v0.57.0 -- crates/nono/src/sandbox/linux.rs crates/nono/src/sandbox/mod.rs crates/nono-cli/src/cli.rs
    ```
    Reconcile with the 9 C4 shas. Record the canonical chronological order. If it differs from the Phase 47 ledger row-order (which is grouped semantically), USE chronological order for the cherry-pick sequence.

    **§ 3. Per-commit diff inspection table** — For each of 9 shas, run `git show <sha> --stat` + `git show <sha> -- <fork-shared-file>` for every touched fork-shared file (per PATTERNS.md row mapping). For each commit produce a row:
    | sha | upstream subject | files touched (fork-shared) | predicted conflict (yes/no/unknown) | resolution strategy |

    **§ 4. Re-export scan on C4 lead commit `c2c6f2ca` (Phase 47 D-47-D2 re-confirmation)** — Run:
    ```bash
    git show c2c6f2ca -- crates/nono/src/sandbox/linux.rs | grep '^+pub'
    git show c2c6f2ca -- crates/nono/src/sandbox/mod.rs | grep '^+pub use'
    git show c2c6f2ca -- crates/nono/src/lib.rs | grep '^+pub use'
    ```
    Verify intra-cluster origin of `LandlockScopePolicy` / `DetectedAbi` / `landlock_scope_policy` / `detect_abi` / `is_wsl2` (Phase 47 audit conclusion — re-verify per `feedback_cluster_isolation_invalid` preventive discipline).

    **§ 5. Profile/mod.rs hot-spot inspection (Phase 47 Empirical cross-check File #4)** — For commit `a0222be2` (the one C4 commit touching profile/mod.rs per PATTERNS.md row #6), run:
    ```bash
    git show a0222be2 -- crates/nono-cli/src/profile/mod.rs | grep -E '^[+-](enum|struct|impl From|pub)'
    grep -nE 'CommandsConfig|FilesystemConfig|LegacyPolicyPatch|DeprecationCounter|bypass_protection|impl From<ProfileDeserialize>' crates/nono-cli/src/profile/mod.rs
    ```
    Predict whether the exhaustive match arm needs extension. Document the extension strategy (in-commit edit, NOT a separate commit, to preserve D-19 trailer fidelity).

    **§ 6. Windows-arm intersection check on supervisor.rs / lib.rs / sandbox/linux.rs (D-48-B2 rationale)** — For each cross-platform file in the C4 file walk:
    ```bash
    git grep -nE 'cfg\\(target_os\\s*=\\s*"windows"\\)|cfg\\(windows\\)' crates/nono/src/lib.rs crates/nono/src/sandbox/linux.rs crates/nono-cli/src/exec_strategy.rs
    ```
    Confirm no upstream hunks land inside a Windows-cfg block (D-48-E1 invariant).

    **§ 7. Audit verdict + cherry-pick strategy** — One of:
    - **GREEN** (all 9 commits predicted to cherry-pick cleanly with documented resolution per § 3) → proceed to Task 2.
    - **YELLOW** (some commits predicted to need in-commit fork-side extension; documented per § 5) → proceed to Task 2 with documented per-commit conflict-resolution steps.
    - **RED** (one or more commits irreconcilable) → STOP. Escalate to D-48-B3 split. Planner re-issues plans `48-01a-...-PLAN.md` + `48-01b-...-PLAN.md`. Document the RED finding here; do not begin Task 2.

    **§ 8. Acceptance summary** — Final per-commit table mirroring § 3 with explicit "approved for cherry-pick" or "escalated to D-48-B3" disposition per commit.
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md && grep -q "Audit verdict" .planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md && grep -cE "^\\| (c2c6f2ca|b8a32006|858ad009|bbc652a0|1e9385a7|98f8cb18|d146001b|a0222be2|863bbfd3)" .planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md | awk '{exit ($1>=9)?0:1}' && echo "Task 1 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - File `.planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md` exists
    - All 8 sections present (grep for `## 1.`, `## 2.`, ..., `## 8.` or `### 1.`/`### 2.`/.../`### 8.`)
    - All 9 C4 shas appear as table rows in § 3 + § 8 (grep `^\| (c2c6f2ca|b8a32006|858ad009|bbc652a0|1e9385a7|98f8cb18|d146001b|a0222be2|863bbfd3)` returns ≥9 lines)
    - Re-export scan output in § 4 confirms intra-cluster origin (text contains "intra-cluster" or equivalent)
    - § 7 audit verdict is explicit GREEN / YELLOW / RED (exact string in document)
    - If RED: this task ends with explicit STOP + escalation to planner; Task 2 does NOT begin until split plans are re-issued
  </acceptance_criteria>
  <done>Pre-flight artifact complete; verdict documented; cherry-pick strategy per commit explicit. If GREEN/YELLOW → proceed to Task 2. If RED → escalate to D-48-B3 split.</done>
</task>

<task type="auto">
  <name>Task 2: Cherry-pick the 9 C4 commits in upstream-chronological order (D-48-B1)</name>
  <files>(all files per frontmatter `files_modified` — actual touch set per commit)</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md § 3 + § 7 (per-commit resolution strategy)
    - .planning/templates/upstream-sync-quick.md § "D-19 cherry-pick trailer block" (6-line shape, lowercase `Upstream-author:`, plus `Co-Authored-By:` 7th line per template)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern A. D-19 6-line cherry-pick trailer block"
    - CLAUDE.md § Commits (DCO sign-off `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`)
    - CLAUDE.md § Coding Standards (no `.unwrap()`; cherry-picks introducing one MUST be in-commit-rewritten to `?` propagation)
  </read_first>
  <action>
    Cherry-pick the 9 C4 commits in the chronological order recorded in `48-01-PRE-CHERRY-PICK-AUDIT.md` § 2. For each sha (use the canonical 40-char form; values below are the abbreviations from the Phase 47 ledger):

    **C4-01: `c2c6f2ca`** — `feat(landlock): add landlock v6 signal and abstract unix socket scoping` (v0.55.0; 11 files; intra-cluster re-export lead)
    **C4-02: `b8a32006`** — `docs(capability): clarify linux signal mode behavior with landlock` (v0.55.0; 2 files; docs-only)
    **C4-03: `858ad009`** — `feat(cli): add recursive unix socket directory grants` (v0.55.0; 8 files)
    **C4-04: `bbc652a0`** — `feat(unix-socket): record explicit scope for grants` (v0.55.0; 4 files)
    **C4-05: `1e9385a7`** — `feat(sandbox): add explicit allowlist for pathname af_unix sockets` (v0.55.0; 4 files)
    **C4-06: `98f8cb18`** — `test(supervisor-linux): add unix listener for connect capability test` (v0.55.0; 1 file)
    **C4-07: `d146001b`** — `fix(sandbox): correctly resolve af_unix socket paths for seccomp` (v0.55.0; 2 files)
    **C4-08: `a0222be2`** — `feat(linux): implement af_unix pathname mediation` (v0.55.0; 18 files; touches profile/mod.rs — pre-flight § 5 strategy applies)
    **C4-09: `863bbfd3`** — `refactor(supervisor): refine ipc denial reporting and audit timestamps` (v0.55.0; 2 files)

    **NOTE:** the chronological order may differ from this ledger order (verified in pre-flight § 2). Use the chronological order from § 2.

    For EACH cherry-pick, the procedure is:

    1. Resolve the 40-char canonical sha: `FULL_SHA=$(git rev-parse <abbrev>^{commit})`
    2. Extract upstream metadata for the trailer block:
       ```bash
       git log -1 --format='Upstream-commit: %H%nUpstream-author: %an <%ae>%nUpstream-date: %aI%nUpstream-subject: %s' $FULL_SHA
       ```
    3. Append `Upstream-tag: v0.55.0` (every C4 commit is in v0.55.0 per ledger) and `Upstream-categories: <drift-tool categories>` (per ledger row — e.g. `other`, `other,profile`, `other,policy,profile` for a0222be2).
    4. Append the 7th `Co-Authored-By: <name> <email>` line per `.planning/templates/upstream-sync-quick.md` template + checker WARNING reconciliation (same upstream author name+email used for BOTH `Upstream-author:` and `Co-Authored-By:` lines per template convention).
    5. Run `git cherry-pick --no-commit $FULL_SHA` (defer commit so the body can be amended with the trailer block; use `-n` if your git uses different flag).
    6. If conflict surfaces:
       - Apply the resolution strategy documented in `48-01-PRE-CHERRY-PICK-AUDIT.md` § 3 for this sha.
       - For `a0222be2` profile/mod.rs hunks: extend the `impl From<ProfileDeserialize> for Profile` exhaustive match in the SAME commit body per pre-flight § 5 (do NOT create a separate commit — preserves D-19 trailer fidelity).
       - For any other in-commit fork-side extension: keep it in the same commit body.
       - If the conflict matches the irreconcilable shape predicted in pre-flight as YELLOW-resolvable, resolve per the documented strategy.
       - If the conflict is NOT predicted and not trivially resolvable: STOP; do NOT improvise. Document the new conflict in `48-01-PRE-CHERRY-PICK-AUDIT.md` § 7 as a verdict-flip to RED; escalate to D-48-B3 split.
    7. After conflict resolution (if any): `git add <resolved files>` then `git commit -F <trailer-file>` using a temp file containing:
       ```
       <upstream subject>

       <upstream body, if any>

       Upstream-commit: <40-char sha>
       Upstream-author: <name> <email>
       Upstream-date: <iso-8601>
       Upstream-subject: <verbatim upstream subject>
       Upstream-tag: v0.55.0
       Upstream-categories: <ledger row categories>
       Co-Authored-By: <name> <email>

       Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
       ```
    8. Verify the commit body has the 7-line trailer block + DCO sign-off:
       ```bash
       git log -1 --format=%B HEAD | grep -E '^Upstream-commit: [0-9a-f]{40}$'
       git log -1 --format=%B HEAD | grep -E '^Co-Authored-By: '
       git log -1 --format=%B HEAD | grep -E '^Signed-off-by: '
       ```
    9. Per-commit smoke: `cargo build --workspace` exits 0 (catch broken builds early; full close-gate at plan close).
    10. Per-commit Windows-only-files invariant check (D-48-E1):
       ```bash
       git diff --name-only HEAD~1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l
       ```
       MUST equal 0 for every C4 cherry-pick.
    11. If `cargo build` fails or invariant check fails: STOP; do NOT proceed to the next commit; investigate per pre-flight strategy.

    Repeat for all 9 commits in chronological order. Record per-commit notes (resolution applied, in-commit fork-side extensions, smoke result) in plan SUMMARY.
  </action>
  <verify>
    <automated>COUNT=$(git log 3f638dc6..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'); test "$COUNT" = "9" && COAUTH=$(git log 3f638dc6..HEAD --format=%B | grep -cE '^Co-Authored-By: '); test "$COAUTH" = "9" && WIN=$(for sha in $(git log 3f638dc6..HEAD --format=%H); do git diff --name-only $sha~1..$sha -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs'; done | wc -l); test "$WIN" = "0" && cargo build --workspace 2>&1 | tail -1 | grep -q "^Finished" && echo "Task 2 PASS (9 cherry-picks, 9 Co-Authored-By lines, 0 Windows-invariant violations, build OK)"</automated>
  </verify>
  <acceptance_criteria>
    - `git log 3f638dc6..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'` equals exactly `9`
    - `git log 3f638dc6..HEAD --format=%B | grep -cE '^Co-Authored-By: '` equals exactly `9` (one per cherry-pick per checker WARNING reconciliation)
    - For each of 9 commits: `git log <commit> -1 --format=%B | grep -cE '^Signed-off-by: Oscar Mack Jr <oscar\\.mack\\.jr@gmail\\.com>$'` equals `1`
    - For each of 9 commits: `git log <commit> -1 --format=%B | grep -cE '^Upstream-tag: v0\\.55\\.0$'` equals `1`
    - For each of 9 commits: `git log <commit> -1 --format=%H` differs from the upstream sha (NEW sha, not equal to upstream)
    - `git diff --name-only 3f638dc6..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l` equals `0` (D-48-E1 invariant)
    - `cargo build --workspace` exits 0 at plan-head commit
    - For `a0222be2` cherry-pick: profile/mod.rs `impl From<ProfileDeserialize>` match remains exhaustive (compile-time enforcement; `cargo build -p nono-cli` is the falsifier)
    - For `c2c6f2ca` cherry-pick: `grep -E 'pub use sandbox::\\{[^}]*LandlockScopePolicy' crates/nono/src/lib.rs` and `grep -E 'pub use linux::\\{[^}]*LandlockScopePolicy' crates/nono/src/sandbox/mod.rs` both return 1 line each (re-exports landed intra-cluster)
  </acceptance_criteria>
  <done>9 cherry-pick commits land with verbatim D-19 trailers + Co-Authored-By + DCO sign-off; Windows invariant honored; per-commit build smoke passes.</done>
</task>

<task type="auto">
  <name>Task 3: Plan 48-01 close-gate (D-48-E9 + Convention Pattern G)</name>
  <files>.planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern G. Per-plan close-gate (Phase 34 D-34-D2 8-check format)"
    - .planning/templates/cross-target-verify-checklist.md (Convention Pattern J reference for gates 3+4)
    - .planning/phases/43-upst5-sync-execution/43-02-CLOSE-GATE.md (Phase 43 close-gate worked example)
    - CLAUDE.md § Coding Standards (cross-target clippy MUST/NEVER bullet)
  </read_first>
  <action>
    Produce `.planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md` with explicit verdicts for all 8 D-34-D2 standard checks + a 9th check for the baseline-aware CI gate. For each check: record the exact command, the exit code, and the verdict (PASS / FAIL / PARTIAL with `skipped_gates_load_bearing` or `_environmental` categorization per Phase 40 anti-pattern #3).

    Run each in turn from `/c/Users/OMack/Nono`:

    1. `cargo test --workspace` — 2200+ tests pass (record count + duration)
    2. `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` (Windows host)
    3. `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` (MANDATORY per CLAUDE.md MUST/NEVER for cfg-gated Linux code in C4; if cross-toolchain unavailable on Windows dev host: mark PARTIAL with `_environmental` skipped-gate categorization + defer to live CI per `.planning/templates/cross-target-verify-checklist.md`)
    4. `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (same categorization rule as gate 3)
    5. `cargo fmt --all -- --check`
    6. Phase 15 smoke test (cite the exact harness from prior Phase 43 close-gate; e.g., `cargo test -p nono-cli --test integration_smoke` or equivalent)
    7. `wfp_port_integration` test (Windows lane) — `cargo test -p nono --test wfp_port_integration` or equivalent
    8. `learn_windows_integration` test (Windows lane) — `cargo test -p nono-cli --test learn_windows_integration` or equivalent
    9. Baseline-aware CI gate (Task 4 below — runs separately; recorded here as cross-reference)

    Then call out the C4-specific adds:
    - C4 is `#[cfg(target_os = "linux")]`-gated; gates 3+4 are LOAD-BEARING per Convention Pattern J + D-48-E4
    - Linux deny-overlap regression test still passes (PATTERNS.md row #7 invariant — Phase 41 Class D test)
    - `From<ProfileDeserialize>` exhaustive match still compiles (a0222be2 in-commit extension worked)
    - Re-exports of `LandlockScopePolicy` + `DetectedAbi` + `landlock_scope_policy` + `is_wsl2` present in lib.rs + sandbox/mod.rs

    Document any PARTIAL gates with explicit categorization in CLOSE-GATE.md and propagate to plan SUMMARY frontmatter as `skipped_gates_load_bearing: [...]` / `skipped_gates_environmental: [...]`.
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md && grep -cE '^### Gate [1-9]' .planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md | awk '{exit ($1>=8)?0:1}' && echo "CLOSE-GATE rubric present with at least 8 gates"</automated>
  </verify>
  <acceptance_criteria>
    - File `.planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md` exists with at least 8 explicit `### Gate N` sections
    - Each gate has: command, exit code, verdict (PASS/FAIL/PARTIAL)
    - Gates 1, 2, 5 verdicts are PASS (or explicitly justified otherwise)
    - Gates 3 + 4 are PASS or PARTIAL with `_environmental` justification (cross-toolchain unavailable on Windows host)
    - Gates 6, 7, 8 are PASS or PARTIAL with explicit categorization
    - No `success → failure` regression introduced on any gate vs Phase 46 baseline (any FAIL must be a pre-existing red→red carry-forward per Convention Pattern H)
  </acceptance_criteria>
  <done>Close-gate matrix complete with explicit verdicts + skipped-gate categorization; ready for baseline-aware CI gate task.</done>
</task>

<task type="auto">
  <name>Task 4: Baseline-aware CI gate vs SHA 3f638dc6 (D-48-E3 + Convention Pattern H)</name>
  <files>(no fork-side file changes; CI lane verification only)</files>
  <read_first>
    - .planning/templates/upstream-sync-quick.md (lines 96-112; gate result interpretation rules)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern H. Baseline-aware CI gate vs SHA 3f638dc6"
    - .planning/phases/46-windows-squash-merge-post-merge-ci-verifications-uat-backlog/46-VERIFICATION.md (baseline `3f638dc6` provenance)
  </read_first>
  <action>
    Push plan-head commit to fork's `pre-merge` branch and run the baseline-aware CI gate:

    1. `git push -f origin phase-48-01-landlock-v6-af-unix:pre-merge` (or per fork's CI convention — verify the exact branch name from `.github/workflows/`)
    2. Wait for GitHub Actions CI on all 8 lanes:
       - Linux Clippy
       - macOS Clippy
       - Windows Build
       - Windows Integration
       - Windows Regression
       - Windows Security
       - Windows Packaging
       - Cross-target Clippy (if separate lane in this fork's CI; otherwise covered by Linux/macOS clippy lanes)
    3. For each lane, categorize transition vs baseline `3f638dc6`:
       - `green→green` = PASS
       - `green→red` = FAIL (real regression; STOP wave; investigate before next plan)
       - `red→red` = PASS (carry-forward; document explicitly)
       - `red→green` = PASS + IMPROVEMENT
    4. Record verdict per lane in `48-01-CLOSE-GATE.md` § 9 (add a new "## Gate 9 — Baseline-aware CI" section if not already present).
    5. ZERO `green → red` transitions allowed; any such transition halts the wave.
    6. Verify via:
       ```bash
       gh run list --branch pre-merge --limit 1 --json conclusion,workflowName,databaseId
       gh run view <run-id> --json jobs --jq '.jobs[] | {name, conclusion}'
       ```
    7. Capture lane transitions in plan SUMMARY frontmatter as a `lane_transitions:` YAML block + propagate to PR-SECTION.md.
  </action>
  <verify>
    <automated>gh run list --branch pre-merge --limit 1 --json conclusion --jq '.[0].conclusion' | grep -qE '^(success|failure)$' && grep -q '^### Gate 9' .planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md && echo "Baseline-aware CI gate recorded"</automated>
  </verify>
  <acceptance_criteria>
    - `gh run list --branch pre-merge --limit 1 --json conclusion --jq '.[0].conclusion'` returns either `success` (all green) or `failure` ONLY if all FAIL lanes are categorized as red→red carry-forward in `48-01-CLOSE-GATE.md` § 9
    - ZERO lanes show `green → red` (Phase 46 baseline lane was `success` AND PR lane is `failure`) — any such transition halts the wave
    - `.planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md` § 9 contains per-lane verdict table with all 8 (or appropriate count for this fork's CI) lanes
    - Plan SUMMARY frontmatter (Task 6) carries `baseline_sha: 3f638dc6` and `lane_transitions:` block
  </acceptance_criteria>
  <done>Baseline-aware CI gate complete; zero green→red transitions; lane verdicts recorded.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 5: Open upstream PR umbrella (D-48-A4 + D-48-E6 + Convention Pattern I)</name>
  <what-built>
    Per-plan PR contribution section `.planning/phases/48-upst6-sync-execution/48-01-PR-SECTION.md` written; ready to open the umbrella PR to upstream `always-further/nono` per fork's one-PR-per-branch-pair pattern.

    Section template (per Phase 43 D-43-E6 + Convention Pattern I):
    - **Subject:** `Cluster C4: Linux Landlock v6 signal/socket scoping + af_unix pathname mediation (9 commits)`
    - **Sha range:** 9 cherry-pick shas (NEW shas, not upstream) — append `git log 3f638dc6..HEAD --format='%h %s' | head -9`
    - **Cluster disposition:** will-sync (Phase 47 ledger row C4)
    - **Key decisions:** D-48-B1 single-plan 9-commit chronological order; D-48-B2 pre-flight artifact; (if applicable) D-48-B3 split escalation outcome
    - **Lane transitions vs `3f638dc6`:** per-lane verdict table from Task 4
  </what-built>
  <how-to-verify>
    Open the upstream umbrella PR. Title shape per Phase 43 precedent:
    `nono: upstream v0.55.0..v0.57.0 sync (Phase 48)`

    Body — Plan 48-01's PR section is the substantive first contribution (Plans 48-02..48-09 append their own sections at their respective closes).

    Steps for the human:
    1. Visit https://github.com/always-further/nono and confirm fork-feature branch `phase-48-01-landlock-v6-af-unix` is pushed (or per fork's umbrella branch convention — verify the exact target branch from prior Phase 43 umbrella).
    2. Open umbrella PR via `gh pr create --repo always-further/nono --base main --head <fork-org>:phase-48-01-landlock-v6-af-unix --title "nono: upstream v0.55.0..v0.57.0 sync (Phase 48)" --body-file .planning/phases/48-upst6-sync-execution/48-01-PR-SECTION.md` (substitute fork org as needed; verify branch naming convention from Phase 43 umbrella `43-UMBRELLA-PR.txt`).
    3. Confirm PR URL appears in `gh pr list --repo always-further/nono --author '@me'`.
    4. Paste PR URL into `48-01-SUMMARY.md` for hand-off to subsequent plans.

    Expected outcome: New PR opened; URL recorded; Plans 48-02..48-09 will append their `48-NN-PR-SECTION.md` content at each plan close.
  </how-to-verify>
  <resume-signal>Type "approved" + paste PR URL, or describe issues</resume-signal>
</task>

<task type="auto">
  <name>Task 6: Plan 48-01 SUMMARY + PR section + STATE update</name>
  <files>
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md
    - .planning/phases/48-upst6-sync-execution/48-01-PR-SECTION.md
    - .planning/STATE.md (update Current Position + last_activity)
  </files>
  <read_first>
    - .planning/phases/43-upst5-sync-execution/43-01-EDITION-2024-FOUNDATION-SUMMARY.md (SUMMARY shape template)
    - .planning/phases/43-upst5-sync-execution/43-02-PR-SECTION.md (PR-SECTION shape template if available; otherwise derive from `43-UMBRELLA-PR.txt`)
    - .planning/STATE.md current `## Current Position` block
  </read_first>
  <action>
    1. Author `48-01-SUMMARY.md` with sections:
       - Frontmatter: `plan_id`, `phase`, `cluster: C4`, `cluster_disposition: will-sync`, `upstream_sha_range: c2c6f2ca..863bbfd3`, `upstream_commit_count: 9`, `baseline_sha: 3f638dc6`, `lane_transitions:` (block from Task 4), `skipped_gates_load_bearing:` (from CLOSE-GATE), `skipped_gates_environmental:` (from CLOSE-GATE), `pr_url:` (from Task 5)
       - § Outcome (GREEN/YELLOW/RED verdict from pre-flight; whether D-48-B3 split was triggered)
       - § Per-commit notes (resolution applied per sha; in-commit fork-side extensions if any — especially the a0222be2 profile/mod.rs exhaustive-match extension)
       - § Cross-target clippy results (Convention Pattern J PARTIAL categorization if applicable)
       - § Re-export scan re-confirmation (intra-cluster origin holds for c2c6f2ca per pre-flight § 4)
       - § Baseline-aware CI gate verdict (per-lane table from Task 4)
       - § Wave 1 hand-off (Plan 48-02 + Plan 48-03 inherit this baseline; surface-disjoint per Phase 47 § Empirical cross-check)
    2. Author `48-01-PR-SECTION.md` per Convention Pattern I (section template). Will be appended to umbrella PR body after subsequent plans close.
    3. Update `.planning/STATE.md`:
       - `## Current Position` → "Phase: 48 / Plan: 1 of 9 (Wave 0 closed; foundation gate landed)"
       - `last_activity` → today's date + brief verb-led summary
       - `## Progress` table for Phase 48 → "1/9 — Wave 0 closed"
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md && test -f .planning/phases/48-upst6-sync-execution/48-01-PR-SECTION.md && grep -q "baseline_sha: 3f638dc6" .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md && grep -q "lane_transitions:" .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md && grep -q "Phase: 48" .planning/STATE.md && echo "SUMMARY + PR-SECTION + STATE updated"</automated>
  </verify>
  <acceptance_criteria>
    - `48-01-SUMMARY.md` exists with frontmatter containing `baseline_sha: 3f638dc6`, `cluster: C4`, `lane_transitions:` block, `pr_url:` field
    - `48-01-PR-SECTION.md` exists with subject + sha range + cluster disposition + key decisions per Convention Pattern I
    - STATE.md `## Current Position` reflects Phase 48 / Plan 1 of 9 closed; `last_activity` updated
  </acceptance_criteria>
  <done>Plan close artifacts complete; STATE.md reflects Wave 0 closure; Wave 1 ready to begin.</done>
</task>

<task type="auto">
  <name>Task 7: DCO sign-off + commit close artifacts</name>
  <files>
    - .planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md
    - .planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md
    - .planning/phases/48-upst6-sync-execution/48-01-PR-SECTION.md
    - .planning/STATE.md
  </files>
  <read_first>
    - CLAUDE.md § Commits (DCO sign-off requirement on every commit)
  </read_first>
  <action>
    Stage the planning artifacts (NOT the cherry-pick commits — those already shipped per Task 2) and create ONE doc commit:
    ```bash
    git add .planning/phases/48-upst6-sync-execution/48-01-PRE-CHERRY-PICK-AUDIT.md \
            .planning/phases/48-upst6-sync-execution/48-01-CLOSE-GATE.md \
            .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md \
            .planning/phases/48-upst6-sync-execution/48-01-PR-SECTION.md \
            .planning/STATE.md
    git commit -s -m "docs(48-01): close Wave 0 cluster C4 (Landlock v6 + af_unix mediation)" \
               -m "9 upstream cherry-picks landed with verbatim D-19 trailers + Co-Authored-By upstream attribution; pre-flight verdict + close-gate + SUMMARY artifacts shipped; STATE.md advanced to Plan 1 of 9 closed; PR umbrella opened per D-48-A4."
    ```
    Verify DCO sign-off: `git log -1 --format=%B HEAD | grep -E '^Signed-off-by: Oscar Mack Jr <oscar\\.mack\\.jr@gmail\\.com>$'`
  </action>
  <verify>
    <automated>git log -1 --format=%B HEAD | grep -qE '^Signed-off-by: Oscar Mack Jr &lt;oscar\\.mack\\.jr@gmail\\.com&gt;$' && git log -1 --format=%s HEAD | grep -q '^docs(48-01):' && echo "Close-doc commit landed with DCO"</automated>
  </verify>
  <acceptance_criteria>
    - HEAD commit subject starts `docs(48-01):`
    - HEAD commit body has `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` line
    - `git status` is clean (no untracked planning artifacts)
  </acceptance_criteria>
  <done>Plan 48-01 closed; Wave 0 complete; Wave 1 (Plans 48-02 + 48-03 parallel) cleared to begin.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

C4 cherry-picks introduce new trust-boundary surface in the Linux sandbox driver. The boundaries:

| Boundary | Description |
|----------|-------------|
| Sandboxed-process → host kernel via Landlock v6 | Landlock signal scoping + abstract unix socket scoping; new kernel-enforced restriction surface (c2c6f2ca + bbc652a0 + 1e9385a7 + 1e9385a7 + d146001b) |
| Sandboxed-process → host fs via af_unix pathname mediation | a0222be2 implements af_unix pathname mediation; pathname socket paths must be canonicalized + validated; new code path in policy.rs (per Phase 47 ledger C4 row) |
| Profile → policy resolver via new af_unix profile config (a0222be2 profile/mod.rs hunk) | Profile field additions must NOT shadow or reorder fork's Phase 36 canonical sections; exhaustive match arm extension required per pre-flight § 5 |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-48-01-01 | Tampering | af_unix pathname mediation pathname canonicalization (a0222be2 policy.rs hunk) | mitigate | Phase 41 Class D Linux deny-overlap regression test (PATTERNS.md row #7) must stay green post-cherry-pick (per gate 1 + spot-check in Close-Gate); CLAUDE.md § Path Handling — path component comparison, never string `starts_with()`, canonicalize at grant time; cherry-pick MUST NOT introduce a string-comparison footgun (visual diff inspection in pre-flight § 3 + § 6) |
| T-48-01-02 | Elevation of Privilege | Landlock v6 signal scoping (c2c6f2ca) — broken scope could let scoped process signal out of sandbox | mitigate | Cherry-pick lands upstream's tested implementation as-is; fork's `LandlockScopePolicy` re-export is intra-cluster (verified in pre-flight § 4 + Phase 47 ledger conclusion); fork-side test `98f8cb18 test(supervisor-linux): add unix listener for connect capability test` lands in the same cluster as part of cherry-pick — provides regression coverage |
| T-48-01-03 | Information Disclosure | IPC denial reporting + audit timestamps (863bbfd3) — refactor could leak unintended detail to logs | mitigate | Refactor is upstream-tested; fork's `audit-attestation` regression coverage (Phase 38 + Phase 45 REQ-RESL-NIX-04) continues to apply; per-cherry-pick smoke (`cargo test --workspace`) catches log-format regressions |
| T-48-01-04 | Denial of Service | af_unix mediation false-positive denials | accept | Upstream-tested; fork's Phase 41 Class D regression test catches deny-overlap regressions; if false-positive emerges in live use, surfaces via fork-side issue and gets patched in a follow-up plan; cluster disposition `will-sync` matches Phase 47 ledger row's `will-sync` |
| T-48-01-05 | Spoofing | Profile field shadowing (a0222be2 profile/mod.rs hunk) — fork's Phase 36-01b exhaustive match arm could break and silently swallow new af_unix profile config | mitigate | Pre-flight § 5 + Convention Pattern J Linux cross-target clippy gates catch missing arm at compile time (`non_exhaustive` warning fires); in-commit extension in the same cherry-pick body preserves D-19 trailer fidelity; PATTERNS.md row #6 explicit "test fixtures at lines 289-311 assert filesystem.deny and commands.allow survive the round-trip" — re-run after a0222be2 cherry-pick |

**Fork-side defense-in-depth preserved (per PATTERNS.md rows #1–#7):**
- Strictly allow-list per CLAUDE.md § Platform-Specific Notes (Landlock cannot express deny-within-allow) — cherry-pick MUST NOT introduce deny-style code path on `sandbox/linux.rs`
- `#[cfg(target_os = "linux")]` gate preserved on every new pub item (cross-platform compile-fence)
- Phase 41 Class D Linux deny-overlap regression test must stay green (REQ-TEST-HYG-01 closed via Phase 44 Plan 44-02)
- Fork's exhaustive `impl From<ProfileDeserialize> for Profile` at line 2068 extended in-commit if a0222be2 adds profile fields (compile-time enforcement)
- Re-export integrity: `LandlockScopePolicy` + `DetectedAbi` + `landlock_scope_policy` + `is_wsl2` re-exported from lib.rs and sandbox/mod.rs (intra-cluster — verified Phase 47 + pre-flight § 4 re-confirmation)
</threat_model>

<verification>
- Cherry-pick count: `git log 3f638dc6..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'` equals exactly `9` (or fewer if D-48-B3 split triggered — then split plans 48-01a + 48-01b cover the gap)
- Co-Authored-By count: `git log 3f638dc6..HEAD --format=%B | grep -cE '^Co-Authored-By: '` equals exactly `9` (per checker WARNING reconciliation — one per cherry-pick)
- DCO sign-off: every commit between baseline and plan-head has `Signed-off-by:` line
- Windows-only-files invariant: `git diff --name-only 3f638dc6..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l` equals `0`
- Re-export integrity: lib.rs + sandbox/mod.rs both have `pub use ... LandlockScopePolicy ...` lines (intra-cluster from c2c6f2ca)
- Profile exhaustive-match preserved: `cargo build -p nono-cli` exits 0
- Pre-flight artifact `48-01-PRE-CHERRY-PICK-AUDIT.md` exists with all 8 sections + verdict
- Close-gate matrix `48-01-CLOSE-GATE.md` exists with ≥8 explicit `### Gate N` sections + verdicts
- Baseline-aware CI gate: zero `green → red` lane transitions vs `3f638dc6`
- PR umbrella URL recorded in `48-01-SUMMARY.md` frontmatter `pr_url:` field
- STATE.md reflects Phase 48 / Plan 1 of 9 closed
- Plan close-doc HEAD commit subject starts `docs(48-01):`
</verification>

<success_criteria>
- 9 cherry-pick commits land between `3f638dc6` and plan-head with verbatim D-19 trailers + Co-Authored-By (or D-48-B3 split executed)
- Pre-flight artifact + close-gate matrix + SUMMARY + PR-SECTION + STATE all updated
- PR umbrella opened to upstream `always-further/nono` with Plan 48-01 as substantive first contribution
- Wave 1 (Plans 48-02 + 48-03 parallel) cleared to start: foundation gate (cli.rs + profile/mod.rs shared surfaces) holds; downstream plans inherit clean baseline
- REQ-UPST6-02 acceptance criteria #1 (will-sync cluster cherry-picked with verbatim D-19 trailers) satisfied for C4
- REQ-UPST6-02 acceptance criteria #4 (baseline-aware CI gate zero green→red) satisfied for Plan 48-01 head commit
- Phase 47 ledger row C4 honored in full (9 commits, will-sync, no windows-touch, intra-cluster re-export confirmed)
</success_criteria>

<output>
After completion, the following files exist under `.planning/phases/48-upst6-sync-execution/`:
- `48-01-PRE-CHERRY-PICK-AUDIT.md` (Task 1)
- `48-01-CLOSE-GATE.md` (Task 3)
- `48-01-SUMMARY.md` (Task 6)
- `48-01-PR-SECTION.md` (Task 6 + opened via Task 5)

STATE.md reflects Phase 48 / Plan 1 of 9 closed. Wave 0 closed; Wave 1 cleared to start.
