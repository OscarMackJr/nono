---
plan_id: 48-06
plan_name: PTY-MUSL-PORTABILITY
phase: 48
phase_name: upst6-sync-execution
wave: 2
depends_on: [48-02, 48-03]
files_modified:
  - crates/nono-cli/src/pty_proxy.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono/src/sandbox/linux.rs
autonomous: true
requirements: [REQ-UPST6-02]
cluster: C7
cluster_disposition: will-sync
upstream_sha_range: 1f552106..3cd22aa5
upstream_commit_count: 4
baseline_sha: 3f638dc6
tags: [upstream-sync, cherry-pick, pty, musl, unix-portability, wave-2]

must_haves:
  truths:
    - "Phase 47 ledger Cluster C7 (4 commits: 1f552106, 279af554, 3d0ff87f, 3cd22aa5) cherry-picked in upstream-chronological order"
    - "Every cherry-picked commit carries the verbatim D-19 6-line trailer block per D-48-E2 + Convention Pattern A AND a `Co-Authored-By: <upstream author>` line per upstream-sync-quick.md template (per checker WARNING reconciliation)"
    - "Windows-only-files invariant honored per D-48-E1 (zero windows-touch in C7)"
    - "Cross-target Linux + macOS clippy gates PASS per CLAUDE.md MUST/NEVER + Convention Pattern J"
    - "Close-gate adds D-48-D4 musl-target verification (`cargo check --target x86_64-unknown-linux-musl`) — PARTIAL `_environmental` if cross-toolchain unavailable on Windows dev host"
    - "Baseline-aware CI gate vs SHA 3f638dc6 produces zero green→red lane transitions"
    - "REQ-UPST6-02 acceptance criterion #1 satisfied for C7"
    - "exec_strategy.rs collision risk with Plan 48-03 D-48-D3 cleanup resolved by wave-sequential gating (depends_on: [48-02, 48-03] guarantees Plan 48-03's cleanup commit is upstream of this plan's cherry-picks per PATTERNS.md row #12)"
  artifacts:
    - path: ".planning/phases/48-upst6-sync-execution/48-06-PR-SECTION.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-06-CLOSE-GATE.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-06-SUMMARY.md"
  key_links:
    - from: "git log Wave0-head..HEAD"
      to: "Phase 47 DIVERGENCE-LEDGER.md Cluster C7 row"
      via: "4 cherry-pick commits"
      pattern: "^Upstream-commit: (1f552106|279af554|3d0ff87f|3cd22aa5)"
---

<objective>
Cherry-pick Phase 47 ledger Cluster C7 (PTY proxy fixes + musl libc Ioctl portability; 4 commits in v0.55.0). Wave 2 polish; surface-disjoint with Plans 48-04, 48-05, 48-07, 48-08.

**Wave gating note (per checker BLOCKER reconciliation):** `depends_on: [48-02, 48-03]` makes the D-48-A2 4-wave SEQUENTIAL model explicit. Critically, this resolves the exec_strategy.rs collision risk flagged in PATTERNS.md row #12 — Plan 48-06 cherry-picks `3cd22aa5` which touches `exec_strategy.rs`, the same file Plan 48-03's D-48-D3 cleanup commit rewrites. The dep edge guarantees Plan 48-03's cleanup has landed before any Plan 48-06 cherry-pick begins, so `3cd22aa5` lands cleanly on the post-cleanup tree (instead of relying on executor prose-level judgment).

Per D-48-D4: close-gate adds `cargo check --target x86_64-unknown-linux-musl` invocation. PARTIAL with `_environmental` skipped-gate categorization acceptable if musl-cross-toolchain unavailable on Windows dev host (defer to live CI per `.planning/templates/cross-target-verify-checklist.md`).

Output: 4 cherry-picks, close-gate (with D-48-D4 musl add-on), SUMMARY, PR-SECTION.
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
    - .planning/phases/48-upst6-sync-execution/48-03-SUMMARY.md (Wave 1 closure confirmation — startup_prompt cleanup commit is upstream of this plan)
    - MEMORY: feedback_windows_worktree_cwd
  </read_first>
  <action>
    1. `cd /c/Users/OMack/Nono`; `pwd` MUST be `/c/Users/OMack/Nono`
    2. Confirm BOTH Wave 1 plans (48-02 and 48-03) closed via SUMMARY status. CRITICAL: verify Plan 48-03's D-48-D3 cleanup commit `cleanup(48-03): remove dead startup_prompt references ahead of upstream 4e0e127a absorption` is present in the merged history — this is the prerequisite for `3cd22aa5` to land cleanly on exec_strategy.rs per PATTERNS.md row #12 collision-risk resolution.
    3. Identify the merged Wave 1 head sha (`WAVE_1_HEAD`); `git checkout -b phase-48-06-pty-musl-portability $WAVE_1_HEAD`
    4. Verify 4 C7 shas: `for sha in 1f552106 279af554 3d0ff87f 3cd22aa5; do git cat-file -e $sha^{commit} && echo "$sha OK" || echo "$sha MISSING"; done`
    5. Record chronological order: `git log --pretty=format:'%H %ci %s' v0.54.0..v0.57.0 -- crates/nono-cli/src/pty_proxy.rs crates/nono-cli/src/exec_strategy.rs crates/nono/src/sandbox/linux.rs`
    6. Check musl-cross-toolchain availability: `rustup target list --installed | grep -q x86_64-unknown-linux-musl && echo "musl target installed" || echo "musl target NOT installed — D-48-D4 will be PARTIAL _environmental"`
  </action>
  <verify>
    <automated>pwd | grep -q "^/c/Users/OMack/Nono$" && git rev-parse phase-48-06-pty-musl-portability &gt;/dev/null 2&gt;&amp;1 && for sha in 1f552106 279af554 3d0ff87f 3cd22aa5; do git cat-file -e $sha^{commit} || exit 1; done; echo "Task 0 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - `pwd` is `/c/Users/OMack/Nono`
    - Both Wave 1 plans confirmed closed; Plan 48-03 cleanup commit confirmed in merged history (prerequisite for 3cd22aa5)
    - Branch off Wave 1 head exists
    - 4 C7 shas resolvable
    - musl target availability recorded (informs D-48-D4 verdict in Task 2)
  </acceptance_criteria>
  <done>Plan branch ready.</done>
</task>

<task type="auto">
  <name>Task 1: Cherry-pick the 4 C7 commits in upstream-chronological order</name>
  <files>
    - crates/nono-cli/src/pty_proxy.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono/src/sandbox/linux.rs
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #13 (pty_proxy.rs invariants)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #12 (exec_strategy.rs — Plan 48-03 cleanup is upstream of this plan per wave-sequential gating; collision-risk resolved at dep-graph level)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #1 (sandbox/linux.rs invariants)
    - .planning/templates/upstream-sync-quick.md § D-19 trailer block (5-line trailer + Co-Authored-By line per template)
    - CLAUDE.md § Commits (DCO)
  </read_first>
  <action>
    Cherry-pick the 4 C7 commits in chronological order. All v0.55.0 per Phase 47 ledger.

    **C7-01: `1f552106`** — `fix: preserve child output without trailing newline (#881)` (1 file: pty_proxy.rs; categories: `other`)
    **C7-02: `279af554`** — `fix(pty): forward bare ESC immediately in filter_client_input` (1 file: pty_proxy.rs; categories: `other`)
    **C7-03: `3d0ff87f`** — `fix(musl): use as _ for TIOCSCTTY ioctl cast to support all platforms` (1 file: pty_proxy.rs; categories: `other`)
    **C7-04: `3cd22aa5`** — `fix(musl): fix libc::Ioctl type mismatches for x86_64-unknown-linux-musl target` (3 files: pty_proxy.rs, exec_strategy.rs, sandbox/linux.rs; categories: `other`)

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
       Upstream-categories: other
       Co-Authored-By: <name> <email>
       ```
       Same upstream author name+email used for BOTH `Upstream-author:` and `Co-Authored-By:` lines per template convention.
    4. `git cherry-pick --no-commit $FULL_SHA`
    5. **For `3cd22aa5` specifically:** Plan 48-03's D-48-D3 cleanup commit is guaranteed upstream of this plan per Task 0 dep-graph confirmation, so the exec_strategy.rs hunks land cleanly on the post-cleanup tree (no `startup_prompt` references to conflict with). Run `git show 3cd22aa5 -- crates/nono-cli/src/exec_strategy.rs` before cherry-pick as a sanity check.
    6. **For sandbox/linux.rs hunks in `3cd22aa5`:** PATTERNS.md row #1 invariant — strictly-allow-list preserved; `#[cfg(target_os = "linux")]` gate preserved on new pub items.
    7. `git commit -F <trailer-file>` (with DCO `Signed-off-by:` AFTER the 7-line trailer block)
    8. Per-commit verify: 7-line trailer + DCO; `cargo build --workspace`
    9. Windows invariant: 0 violations
  </action>
  <verify>
    <automated>WAVE1=$(git merge-base HEAD phase-48-02-profile-shadowing); COUNT=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'); test "$COUNT" = "4" && COAUTH=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Co-Authored-By: '); test "$COAUTH" = "4" && WIN=$(git diff --name-only $WAVE1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l); test "$WIN" = "0" && cargo build --workspace 2&gt;&amp;1 | tail -1 | grep -q "^Finished" && echo "Task 1 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - 4 cherry-picks with trailer + Co-Authored-By + DCO + `Upstream-tag: v0.55.0` (4 Co-Authored-By lines total across plan)
    - Windows invariant 0 violations
    - `cargo build --workspace` exits 0
  </acceptance_criteria>
  <done>4 cherry-picks landed.</done>
</task>

<task type="auto">
  <name>Task 2: Plan 48-06 close-gate (Convention Pattern G) + D-48-D4 musl verification add</name>
  <files>.planning/phases/48-upst6-sync-execution/48-06-CLOSE-GATE.md</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern G" + § "Convention Pattern J"
    - .planning/templates/cross-target-verify-checklist.md
  </read_first>
  <action>
    Produce `48-06-CLOSE-GATE.md` with 8 standard gates PLUS Gate 9 (D-48-D4 musl verification — note this is a different Gate 9 from Convention Pattern H's baseline-aware CI gate, which becomes Gate 10 for this plan):
    - Gates 1-8 standard per Convention Pattern G
    - Gate 3+4: cross-target Linux + macOS clippy MANDATORY (C7 touches Linux-cfg-gated code)
    - **Gate 9 (D-48-D4):** `cargo check --target x86_64-unknown-linux-musl` — PASS if cross-toolchain installed; PARTIAL `_environmental` if Windows dev host lacks musl-cross (likely — per Task 0 musl availability check). Defer to live CI per `.planning/templates/cross-target-verify-checklist.md` if PARTIAL.
    - **Gate 10:** baseline-aware CI gate (Convention Pattern H — moved from Gate 9 due to D-48-D4 inserting a new gate)
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-06-CLOSE-GATE.md && grep -cE '^### Gate [1-9]' .planning/phases/48-upst6-sync-execution/48-06-CLOSE-GATE.md | awk '{exit ($1&gt;=9)?0:1}' && grep -q 'x86_64-unknown-linux-musl' .planning/phases/48-upst6-sync-execution/48-06-CLOSE-GATE.md && echo "CLOSE-GATE present with D-48-D4 musl gate"</automated>
  </verify>
  <acceptance_criteria>
    - File exists with ≥9 gate sections (gates 1-8 standard + Gate 9 musl + Gate 10 baseline-aware CI OR Gate 9 musl PLUS § for baseline-aware CI — exact numbering at planner discretion)
    - Gate 9 includes `cargo check --target x86_64-unknown-linux-musl` command + verdict (PASS or PARTIAL `_environmental`)
    - Skipped-gate categorization explicit
  </acceptance_criteria>
  <done>Close-gate complete with D-48-D4 add.</done>
</task>

<task type="auto">
  <name>Task 3: Baseline-aware CI gate vs SHA 3f638dc6 (Gate 10 or final gate)</name>
  <files>(no file changes)</files>
  <read_first>
    - .planning/templates/upstream-sync-quick.md (lines 96-112)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern H"
  </read_first>
  <action>
    Push `phase-48-06-pty-musl-portability` to fork's `pre-merge` branch; wait for GH Actions; categorize lanes vs `3f638dc6`. Record in `48-06-CLOSE-GATE.md` final gate. ZERO green→red.

    Note: live CI may execute the musl-target gate that Task 2 marked PARTIAL — record the live-CI musl verdict here if visible.
  </action>
  <verify>
    <automated>gh run list --branch pre-merge --limit 1 --json conclusion --jq '.[0].conclusion' | grep -qE '^(success|failure)$' && grep -qE '^### Gate (9|10)' .planning/phases/48-upst6-sync-execution/48-06-CLOSE-GATE.md && echo "Baseline-aware CI recorded"</automated>
  </verify>
  <acceptance_criteria>
    - Zero green→red lane transitions
    - Final gate section has per-lane verdict
    - Live-CI musl verdict cross-referenced if PARTIAL in Task 2
  </acceptance_criteria>
  <done>Baseline-aware CI + musl live-CI verdict captured.</done>
</task>

<task type="auto">
  <name>Task 4: SUMMARY + PR section + STATE update + close-doc commit</name>
  <files>
    - .planning/phases/48-upst6-sync-execution/48-06-SUMMARY.md
    - .planning/phases/48-upst6-sync-execution/48-06-PR-SECTION.md
    - .planning/STATE.md
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md (template)
  </read_first>
  <action>
    1. Author `48-06-SUMMARY.md` (frontmatter: `cluster: C7`, `cluster_disposition: will-sync`, `upstream_sha_range: 1f552106..3cd22aa5`, `upstream_commit_count: 4`, `baseline_sha: 3f638dc6`, `lane_transitions:`, `skipped_gates_*:`, `musl_target_verdict: <PASS|PARTIAL_environmental>`, `pr_section:`).
    2. Author `48-06-PR-SECTION.md` per Convention Pattern I — note D-48-D4 musl-target gate verdict in key decisions.
    3. Append to umbrella PR body.
    4. Update STATE.md (Plan 6 of 9).
    5. Commit:
       ```bash
       git add .planning/phases/48-upst6-sync-execution/48-06-*.md .planning/STATE.md
       git commit -s -m "docs(48-06): close cluster C7 (PTY proxy + musl portability)" \
                  -m "4 upstream cherry-picks landed with verbatim D-19 trailers + Co-Authored-By upstream attribution; D-48-D4 musl-target verification verdict recorded; cross-target clippy verdicts captured; STATE.md advanced; umbrella PR body appended."
       ```
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-06-SUMMARY.md && test -f .planning/phases/48-upst6-sync-execution/48-06-PR-SECTION.md && grep -q "cluster: C7" .planning/phases/48-upst6-sync-execution/48-06-SUMMARY.md && grep -q "musl_target_verdict:" .planning/phases/48-upst6-sync-execution/48-06-SUMMARY.md && git log -1 --format=%s HEAD | grep -q "^docs(48-06):" && echo "Plan 48-06 closed"</automated>
  </verify>
  <acceptance_criteria>
    - SUMMARY + PR-SECTION exist
    - SUMMARY frontmatter has `musl_target_verdict:` field (explicit D-48-D4 verdict)
    - STATE.md reflects Plan 6 of 9
    - Close-doc commit subject `docs(48-06):` + DCO
  </acceptance_criteria>
  <done>Plan 48-06 closed.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| PTY proxy → child output stream | Cross-platform PTY child-output forwarding + interactive terminal handling; 1f552106 + 279af554 affect output preservation + ESC forwarding behavior |
| musl libc Ioctl type interface | Cross-platform Unix-side ioctl type system intersection; 3d0ff87f + 3cd22aa5 fix type-mismatches for x86_64-unknown-linux-musl target |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-48-06-01 | Information Disclosure | PTY trailing newline preservation (1f552106) — change in output format could leak unintended whitespace patterns | accept | Upstream PR #881 tested; user-visible output formatting change only; no security-relevant data leak path |
| T-48-06-02 | Tampering | Bare ESC forwarding (279af554) — terminal escape sequences from child could affect parent terminal rendering | accept | Upstream behavior change intended; fork's PTY proxy work is upstream-equivalent (Phase 27); no escape-sequence sanitization regression |
| T-48-06-03 | Denial of Service | musl Ioctl type fix (3cd22aa5) — could break Linux build if Ioctl type definitions diverge | mitigate | D-48-D4 musl-target gate explicitly verifies `cargo check --target x86_64-unknown-linux-musl` (PARTIAL `_environmental` deferred to live CI if cross-toolchain unavailable); cross-target Linux gnu clippy gate also runs |
| T-48-06-04 | Tampering | sandbox/linux.rs hunks in 3cd22aa5 could introduce non-`#[cfg(target_os = "linux")]`-gated additions | mitigate | PATTERNS.md row #1 invariant: `#[cfg(target_os = "linux")]` gate preserved on every new pub item; cross-target macOS clippy catches gating regressions |
</threat_model>

<verification>
- 4 cherry-picks with verbatim D-19 trailers + Co-Authored-By + DCO
- Windows invariant 0 violations
- `cargo build --workspace` exits 0
- D-48-D4 musl-target verdict recorded (PASS or PARTIAL `_environmental`)
- Cross-target Linux + macOS clippy PASS (or PARTIAL `_environmental`)
- Close-gate (≥9 gates including Gate 9 musl) + baseline-aware CI complete
- Zero green→red transitions
</verification>

<success_criteria>
- REQ-UPST6-02 acceptance criteria #1 satisfied for C7
- D-48-D4 musl-target verification per CONTEXT.md decision
- Wave 2 partial complete
</success_criteria>

<output>
After completion:
- `48-06-CLOSE-GATE.md`
- `48-06-SUMMARY.md`
- `48-06-PR-SECTION.md`

STATE.md reflects Plan 6 of 9.
