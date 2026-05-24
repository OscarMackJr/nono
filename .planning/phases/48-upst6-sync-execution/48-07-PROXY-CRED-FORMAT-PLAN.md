---
plan_id: 48-07
plan_name: PROXY-CRED-FORMAT
phase: 48
phase_name: upst6-sync-execution
wave: 2
depends_on: [48-02, 48-03]
files_modified:
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile_cmd.rs
  - crates/nono-proxy/src/config.rs
  - crates/nono-proxy/src/credential.rs
  - crates/nono-proxy/src/route.rs
  - crates/nono-proxy/src/server.rs
autonomous: true
requirements: [REQ-UPST6-02]
cluster: C8
cluster_disposition: will-sync
upstream_sha_range: 57005737..530306ee
upstream_commit_count: 2
baseline_sha: 3f638dc6
tags: [upstream-sync, cherry-pick, proxy, credential, schema, wave-2]

must_haves:
  truths:
    - "Phase 47 ledger Cluster C8 (2 commits: 57005737, 530306ee) cherry-picked in upstream-chronological order"
    - "Every cherry-picked commit carries the verbatim D-19 6-line trailer block per D-48-E2 + Convention Pattern A AND a `Co-Authored-By: <upstream author>` line per upstream-sync-quick.md template (per checker WARNING reconciliation)"
    - "Windows-only-files invariant honored per D-48-E1 (zero windows-touch in C8)"
    - "D-48-D2 schema-validator coverage check executed BEFORE cherry-pick; focused regression test added ONLY if gap detected"
    - "Cross-target Linux + macOS clippy gates PASS per CLAUDE.md MUST/NEVER"
    - "Baseline-aware CI gate vs SHA 3f638dc6 produces zero green→red lane transitions"
    - "REQ-UPST6-02 acceptance criterion #1 satisfied for C8"
    - "Fork's exhaustive `From<ProfileDeserialize>` match preserved (57005737 touches profile/mod.rs)"
  artifacts:
    - path: ".planning/phases/48-upst6-sync-execution/48-07-PR-SECTION.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-07-CLOSE-GATE.md"
    - path: ".planning/phases/48-upst6-sync-execution/48-07-SUMMARY.md"
    - path: "(conditional, only if D-48-D2 detects gap) crates/nono-cli/tests/credential_format_regression.rs OR similar"
  key_links:
    - from: "git log Wave0-head..HEAD"
      to: "Phase 47 DIVERGENCE-LEDGER.md Cluster C8 row"
      via: "2 cherry-pick commits + (conditional) 1 fork-side regression test commit per D-48-D2"
      pattern: "^Upstream-commit: (57005737|530306ee)"
---

<objective>
Cherry-pick Phase 47 ledger Cluster C8 (proxy credential format on custom inject headers; 2 commits in v0.55.0). Wave 2 polish; surface-disjoint with Plans 48-04, 48-05, 48-06, 48-08.

**Wave gating note (per checker BLOCKER reconciliation):** `depends_on: [48-02, 48-03]` makes the D-48-A2 4-wave SEQUENTIAL model explicit — Wave 2 cannot start until BOTH Wave 1 plans (48-02 and 48-03) close.

Per D-48-D2: Plan 48-07 includes a pre-flight task that greps fork-side tests for jsonschema validation against `crates/nono-cli/data/nono-profile.schema.json`. If existing coverage exercises the new `credential_format` field shape across all 3 cases (omitted → default-resolution, explicit `'Bearer {}'`, explicit bare token) → no new test. If gap → add a focused fork-side regression test (cleanup commit, NOT a D-19 trailer commit) BEFORE cherry-pick.

Output: 2 cherry-picks, optionally 1 fork-side regression-test commit, close-gate, SUMMARY, PR-SECTION.
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
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-SUMMARY.md
@.planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
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
    2. Confirm BOTH Wave 1 plans (48-02 and 48-03) closed via SUMMARY status; identify the merged Wave 1 head sha (`WAVE_1_HEAD`).
    3. `git checkout -b phase-48-07-proxy-cred-format $WAVE_1_HEAD`
    4. Verify 2 C8 shas: `for sha in 57005737 530306ee; do git cat-file -e $sha^{commit} && echo "$sha OK" || echo "$sha MISSING"; done`
    5. Record chronological order: `git log --pretty=format:'%H %ci %s' v0.54.0..v0.57.0 -- crates/nono-cli/data/nono-profile.schema.json crates/nono-proxy/src/`
  </action>
  <verify>
    <automated>pwd | grep -q "^/c/Users/OMack/Nono$" && git rev-parse phase-48-07-proxy-cred-format &gt;/dev/null 2&gt;&amp;1 && for sha in 57005737 530306ee; do git cat-file -e $sha^{commit} || exit 1; done; echo "Task 0 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - `pwd` is `/c/Users/OMack/Nono`
    - Both Wave 1 plans confirmed closed
    - Branch off Wave 1 head exists
    - 2 C8 shas resolvable
  </acceptance_criteria>
  <done>Plan branch ready.</done>
</task>

<task type="auto">
  <name>Task 1: D-48-D2 schema-validator coverage check (mandatory pre-cherry-pick)</name>
  <files>(read-only inspection; output captured in plan SUMMARY)</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #11 (nono-profile.schema.json invariants + D-48-D2 task)
    - .planning/phases/48-upst6-sync-execution/48-CONTEXT.md § "D-48-D2" decision
  </read_first>
  <action>
    Per D-48-D2 pre-flight:
    ```bash
    grep -rnE 'credential_format|nono-profile.schema.json' crates/nono-cli/tests/ tests/integration/ 2>/dev/null
    ```

    Inspect upstream's intent for `credential_format`:
    ```bash
    git show 57005737 -- crates/nono-cli/data/nono-profile.schema.json | head -50
    git show 57005737 -- crates/nono-cli/src/network_policy.rs | grep -E 'credential_format|^[+-]' | head -40
    ```

    Determine: does existing fork-side coverage exercise the new `credential_format` field shape across all 3 cases?
    1. **Case A: omitted** → default resolution (Authorization → `Bearer {}`, other headers → bare `{}`)
    2. **Case B: explicit `'Bearer {}'`**
    3. **Case C: explicit bare token (e.g., `{}`)**

    Verdict:
    - **No gap:** all 3 cases exercised → skip Task 2 (cherry-pick straight to Task 3); record `d_48_d2_verdict: no_gap_coverage_present` in SUMMARY frontmatter
    - **Gap:** at least one case not exercised → proceed to Task 2 (focused regression test) BEFORE cherry-pick; record `d_48_d2_verdict: gap_detected_added_regression_test` in SUMMARY frontmatter

    Record full inspection findings + verdict in shell scrollback / SUMMARY notes.
  </action>
  <verify>
    <automated>grep -rnE 'credential_format' crates/nono-cli/tests/ tests/integration/ 2&gt;/dev/null | wc -l | awk '{print "credential_format coverage line count: "$1}' &amp;&amp; echo "D-48-D2 verdict pending Task 2 conditional"</automated>
  </verify>
  <acceptance_criteria>
    - Full grep output recorded
    - Verdict explicitly determined: `no_gap_coverage_present` OR `gap_detected_added_regression_test`
    - SUMMARY notes capture per-case (A/B/C) coverage status
  </acceptance_criteria>
  <done>D-48-D2 verdict reached; Task 2 conditionally proceeds.</done>
</task>

<task type="auto">
  <name>Task 2 (CONDITIONAL — only if Task 1 verdict = gap_detected): Add focused fork-side regression test</name>
  <files>(conditional — likely under crates/nono-cli/tests/ or tests/integration/)</files>
  <read_first>
    - Task 1 inspection notes
    - Existing fork-side test patterns: `ls crates/nono-cli/tests/ tests/integration/ 2>/dev/null`
    - CLAUDE.md § Coding Standards (no `.unwrap()`, env-var save/restore in tests if env vars are used)
  </read_first>
  <action>
    **SKIP THIS TASK if Task 1 verdict = `no_gap_coverage_present`.**

    Otherwise, author a focused fork-side regression test exercising the 3 `credential_format` cases. Suggested filename: `crates/nono-cli/tests/credential_format_regression.rs` OR add to an existing tests file if that's the fork's convention (verify via `ls crates/nono-cli/tests/`).

    Test shape (per CLAUDE.md § Coding Standards — Rust idioms):
    ```rust
    // Test 1: credential_format omitted → Authorization header gets "Bearer {}"; other headers get bare "{}"
    // Test 2: credential_format explicit "Bearer {}" → applied as-is
    // Test 3: credential_format explicit "{}" → bare token, no Bearer prefix
    ```

    Use jsonschema validation against `crates/nono-cli/data/nono-profile.schema.json` to assert the cases.

    Commit as a fork-side regression test (NO D-19 trailer per D-48-D2 — this is fork-authored, not an upstream cherry-pick; NO `Co-Authored-By:` trailer either since there is no upstream author to attribute):
    ```bash
    git add <test-file>
    cargo test --workspace # confirm tests pass
    git commit -s -m "test(48-07): add credential_format regression coverage per D-48-D2 gap" \
               -m "Fork-side regression test exercising 3 credential_format cases (omitted/explicit Bearer/explicit bare) against nono-profile.schema.json. Closes the D-48-D2 coverage gap detected in Task 1 pre-flight. NO D-19 trailer + NO Co-Authored-By (fork-authored regression test, not an upstream cherry-pick)."
    ```
  </action>
  <verify>
    <automated>VERDICT=$(grep 'd_48_d2_verdict' .planning/phases/48-upst6-sync-execution/48-07-SUMMARY.md 2&gt;/dev/null || echo "pending"); if echo "$VERDICT" | grep -q "gap_detected"; then git log -1 --format=%s HEAD | grep -q "^test(48-07):" &amp;&amp; ! git log -1 --format=%B HEAD | grep -qE '^Upstream-commit: ' &amp;&amp; echo "Conditional regression test commit OK"; else echo "Task 2 skipped (no_gap_coverage_present)"; fi</automated>
  </verify>
  <acceptance_criteria>
    - **If gap detected:** HEAD commit subject starts `test(48-07):`; body has DCO sign-off; body does NOT have `Upstream-commit:` trailer; body does NOT have `Co-Authored-By:` line (fork-authored); `cargo test --workspace` exits 0 with the new test included
    - **If no gap:** Task is skipped; no new commit
  </acceptance_criteria>
  <done>D-48-D2 coverage gap (if any) closed.</done>
</task>

<task type="auto">
  <name>Task 3: Cherry-pick the 2 C8 commits in upstream-chronological order</name>
  <files>
    - crates/nono-cli/data/nono-profile.schema.json
    - crates/nono-cli/src/network_policy.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/profile_cmd.rs
    - crates/nono-proxy/src/config.rs
    - crates/nono-proxy/src/credential.rs
    - crates/nono-proxy/src/route.rs
    - crates/nono-proxy/src/server.rs
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #11 (schema.json invariants)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md row #6 (profile/mod.rs LOAD-BEARING — 57005737 touches it)
    - .planning/templates/upstream-sync-quick.md § D-19 trailer block (5-line trailer + Co-Authored-By line per template)
    - CLAUDE.md § Commits (DCO)
  </read_first>
  <action>
    Cherry-pick the 2 C8 commits in chronological order. Both v0.55.0 per Phase 47 ledger.

    **C8-01: `57005737`** — `fix(proxy): honor explicit credential_format on custom inject headers` (7 files; categories: `other,profile,proxy`)
    **C8-02: `530306ee`** — `review fix` (3 files; categories: `other,profile,proxy`)

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
       Upstream-categories: other,profile,proxy
       Co-Authored-By: <name> <email>
       ```
       Same upstream author name+email used for BOTH `Upstream-author:` and `Co-Authored-By:` lines per template convention.
    4. `git cherry-pick --no-commit $FULL_SHA`
    5. **For `57005737` specifically (touches profile/mod.rs per PATTERNS.md row #6):** if exhaustive-match extension required, extend in-commit
    6. **For schema.json hunks:** PATTERNS.md row #11 invariant — extension MUST be additive; existing fork-side validators (now including Task 2's regression if added) must continue to validate
    7. `git commit -F <trailer-file>` (with DCO `Signed-off-by:` AFTER the 7-line trailer block)
    8. Per-commit verify: 7-line trailer + DCO; `cargo build --workspace`; `cargo build -p nono-cli` (exhaustive-match enforcement)
    9. Windows invariant: 0 violations
    10. If Task 2 added regression test: re-run it to confirm it now exercises the cherry-picked behavior (`cargo test -p nono-cli --test credential_format_regression` or equivalent)
  </action>
  <verify>
    <automated>WAVE1=$(git merge-base HEAD phase-48-02-profile-shadowing); COUNT=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Upstream-commit: [0-9a-f]{40}$'); test "$COUNT" = "2" && COAUTH=$(git log $WAVE1..HEAD --format=%B | grep -cE '^Co-Authored-By: '); test "$COAUTH" = "2" && WIN=$(git diff --name-only $WAVE1..HEAD -- crates/nono-cli/src/exec_strategy_windows/ crates/nono-shell-broker/ '*_windows.rs' | wc -l); test "$WIN" = "0" && cargo build --workspace 2&gt;&amp;1 | tail -1 | grep -q "^Finished" && echo "Task 3 PASS"</automated>
  </verify>
  <acceptance_criteria>
    - 2 cherry-picks with trailer + Co-Authored-By + DCO + `Upstream-tag: v0.55.0` (2 Co-Authored-By lines from cherry-picks; Task 2 regression test commit does NOT carry Co-Authored-By since fork-authored)
    - Windows invariant 0 violations
    - `cargo build -p nono-cli` exits 0 (exhaustive-match enforcement)
    - Schema additions verify against existing fork-side validators
    - If Task 2 added regression test: it passes post-cherry-pick
  </acceptance_criteria>
  <done>2 cherry-picks landed; schema additions composed with existing validator coverage.</done>
</task>

<task type="auto">
  <name>Task 4: Plan 48-07 close-gate (Convention Pattern G)</name>
  <files>.planning/phases/48-upst6-sync-execution/48-07-CLOSE-GATE.md</files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern G"
    - .planning/templates/cross-target-verify-checklist.md
  </read_first>
  <action>
    Produce `48-07-CLOSE-GATE.md` with 8 standard gates per Convention Pattern G AND leave placeholder header `### Gate 9 — Baseline-aware CI` for the baseline-CI task (Task 5) to fill (per checker WARNING #2 reconciliation — explicit gate-sequence ordering). C8 specifics:
    - Cross-platform `nono-proxy` + profile schema work; gates 3+4 (cross-target Linux + macOS clippy) MANDATORY per CLAUDE.md
    - If Task 2 added regression test, include it as an explicit check in close-gate (sub-bullet in Gate 1 `cargo test --workspace`)
    - Gates 7+8 — Windows lane; should remain green (no Windows surface touched)
    - **Produce 8 standard gates AND leave placeholder header `### Gate 9 — Baseline-aware CI` for the baseline-CI task to fill** so the close-gate file has a fixed `### Gate 9` anchor.
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-07-CLOSE-GATE.md && grep -cE '^### Gate [1-9]' .planning/phases/48-upst6-sync-execution/48-07-CLOSE-GATE.md | awk '{exit ($1&gt;=8)?0:1}' &amp;&amp; grep -qE '^### Gate 9' .planning/phases/48-upst6-sync-execution/48-07-CLOSE-GATE.md && echo "CLOSE-GATE present with Gate 9 placeholder"</automated>
  </verify>
  <acceptance_criteria>
    - File exists with ≥8 gate sections AND a `### Gate 9 — Baseline-aware CI` placeholder header
    - If Task 2 added regression test: explicit reference in close-gate
    - Skipped-gate categorization explicit
  </acceptance_criteria>
  <done>Close-gate complete with Gate 9 placeholder ready for Task 5.</done>
</task>

<task type="auto">
  <name>Task 5: Baseline-aware CI gate vs SHA 3f638dc6</name>
  <files>(no file changes)</files>
  <read_first>
    - .planning/templates/upstream-sync-quick.md (lines 96-112)
    - .planning/phases/48-upst6-sync-execution/48-PATTERNS.md § "Convention Pattern H"
  </read_first>
  <action>
    Push `phase-48-07-proxy-cred-format` to fork's `pre-merge`; wait for GH Actions; categorize lanes vs `3f638dc6`. Fill the `### Gate 9 — Baseline-aware CI` placeholder header in `48-07-CLOSE-GATE.md` (placeholder authored in Task 4) with the per-lane verdict table. ZERO green→red.
  </action>
  <verify>
    <automated>gh run list --branch pre-merge --limit 1 --json conclusion --jq '.[0].conclusion' | grep -qE '^(success|failure)$' && grep -q '^### Gate 9' .planning/phases/48-upst6-sync-execution/48-07-CLOSE-GATE.md && echo "Baseline-aware CI recorded"</automated>
  </verify>
  <acceptance_criteria>
    - Zero green→red lane transitions
    - § Gate 9 placeholder filled with per-lane verdict
  </acceptance_criteria>
  <done>Baseline-aware CI complete.</done>
</task>

<task type="auto">
  <name>Task 6: SUMMARY + PR section + STATE update + close-doc commit</name>
  <files>
    - .planning/phases/48-upst6-sync-execution/48-07-SUMMARY.md
    - .planning/phases/48-upst6-sync-execution/48-07-PR-SECTION.md
    - .planning/STATE.md
  </files>
  <read_first>
    - .planning/phases/48-upst6-sync-execution/48-01-SUMMARY.md (template)
  </read_first>
  <action>
    1. Author `48-07-SUMMARY.md` (frontmatter: `cluster: C8`, `cluster_disposition: will-sync`, `upstream_sha_range: 57005737..530306ee`, `upstream_commit_count: 2`, `baseline_sha: 3f638dc6`, `lane_transitions:`, `skipped_gates_*:`, `d_48_d2_verdict: <no_gap_coverage_present|gap_detected_added_regression_test>`, `fork_side_regression_commits: <0|1>`, `pr_section:`).
    2. Author `48-07-PR-SECTION.md` per Convention Pattern I — note D-48-D2 verdict + any regression test added in key decisions.
    3. Append to umbrella PR body.
    4. Update STATE.md (Plan 7 of 9).
    5. Commit:
       ```bash
       git add .planning/phases/48-upst6-sync-execution/48-07-*.md .planning/STATE.md
       git commit -s -m "docs(48-07): close cluster C8 (proxy credential_format schema extension)" \
                  -m "2 upstream cherry-picks landed with verbatim D-19 trailers + Co-Authored-By upstream attribution; D-48-D2 schema-validator coverage verdict recorded; (if applicable) fork-side regression test for credential_format 3-case shape (no Co-Authored-By on fork-authored regression commit); STATE.md advanced; umbrella PR body appended."
       ```
  </action>
  <verify>
    <automated>test -f .planning/phases/48-upst6-sync-execution/48-07-SUMMARY.md && test -f .planning/phases/48-upst6-sync-execution/48-07-PR-SECTION.md && grep -q "cluster: C8" .planning/phases/48-upst6-sync-execution/48-07-SUMMARY.md && grep -q "d_48_d2_verdict:" .planning/phases/48-upst6-sync-execution/48-07-SUMMARY.md && git log -1 --format=%s HEAD | grep -q "^docs(48-07):" && echo "Plan 48-07 closed"</automated>
  </verify>
  <acceptance_criteria>
    - SUMMARY + PR-SECTION exist
    - SUMMARY frontmatter has `d_48_d2_verdict:` and `fork_side_regression_commits:` fields
    - STATE.md reflects Plan 7 of 9
    - Close-doc commit subject `docs(48-07):` + DCO
  </acceptance_criteria>
  <done>Plan 48-07 closed.</done>
</task>

</tasks>

<threat_model>

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Proxy credential injection → outbound HTTP header | `credential_format` field governs HOW the credential is rendered into the Authorization header (or other custom inject header); incorrect format renders the credential unusable OR exposes it as bare bytes when wrapping was expected |
| Profile schema → proxy config | Upstream extension treats `credential_format` as `Option<String>`; omitted vs explicit `'Bearer {}'` are now distinct (previously may have collapsed to same default) — security-relevant wire-protocol behavior change |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-48-07-01 | Information Disclosure | `credential_format` misinterpreted — bare token rendered when Bearer was expected (or vice versa) — could leak credential format to peer | mitigate | D-48-D2 regression test (if Task 2 added) explicitly exercises the 3 cases; upstream's default resolution (Authorization → `Bearer {}`, other → bare `{}`) is unchanged in semantics; cherry-pick + schema update lands as a unit per Task 3 |
| T-48-07-02 | Tampering | Schema extension breaks existing fork-side validator | mitigate | Pre-flight (Task 1) verifies coverage; Phase 25 fork-side credential injection layer composes additively per Phase 47 ledger C8 row |
| T-48-07-03 | Spoofing | Profile field shadowing in `From<ProfileDeserialize>` exhaustive match breaks when 57005737 adds `credential_format: Option<String>` field | mitigate | Compile-time enforcement via `cargo build -p nono-cli`; PATTERNS.md row #6 invariant; in-commit extension preserves D-19 trailer fidelity |
</threat_model>

<verification>
- 2 cherry-picks with verbatim D-19 trailers + Co-Authored-By + DCO
- 0 or 1 fork-side regression-test commits per D-48-D2 verdict (NO D-19 trailer + NO Co-Authored-By on regression-test commit)
- Windows invariant 0 violations
- `cargo build -p nono-cli` exits 0 (exhaustive-match preserved)
- Cross-target Linux + macOS clippy PASS (or PARTIAL `_environmental`)
- Close-gate + baseline-aware CI complete; zero green→red
- D-48-D2 verdict recorded explicitly in SUMMARY
</verification>

<success_criteria>
- REQ-UPST6-02 acceptance criteria #1 satisfied for C8
- D-48-D2 schema-validator coverage decision honored
- Wave 2 partial complete
</success_criteria>

<output>
After completion:
- `48-07-CLOSE-GATE.md`
- `48-07-SUMMARY.md`
- `48-07-PR-SECTION.md`
- (conditional) regression-test file under `crates/nono-cli/tests/`

STATE.md reflects Plan 7 of 9.
