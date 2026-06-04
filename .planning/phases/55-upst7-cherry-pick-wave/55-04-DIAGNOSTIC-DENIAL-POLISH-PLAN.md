---
phase: 55-upst7-cherry-pick-wave
plan: 04
type: execute
wave: 3
depends_on: [55-02]
files_modified:
  - crates/nono/src/diagnostic.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/output.rs
autonomous: true
requirements: [REQ-UPST7-02]
must_haves:
  truths:
    - "Denied paths that are in suppress_save_prompt are annotated [save skipped] in the diagnostic footer"
    - "Canonical denial paths are pre-computed once to avoid repeated fs I/O in the diagnostic path"
    - "rfind is used for access mode splitting (not find); the test for this is present"
    - "Only the path is bold in the diagnostic footer — not the access type or labels"
    - "Each of the 4 commits carries a verbatim 6-line D-19 trailer + DCO Signed-off-by"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
    - "exec_strategy.rs from this plan does NOT conflict with C11's exec_strategy.rs changes (wave 4 plan 55-05)"
  artifacts:
    - path: "crates/nono/src/diagnostic.rs"
      provides: "pre-computed canonical denial paths + [save skipped] annotations + bold-path-only footer"
    - path: "crates/nono-cli/src/exec_strategy.rs"
      provides: "pre-computed canonical denial paths wiring (7-line change from a606b5b5)"
    - path: "crates/nono-cli/src/output.rs"
      provides: "rfind access-mode split + bold-only-path styling (2 commits: 668e341 + 8fd8da0)"
  key_links:
    - from: "exec_strategy.rs"
      to: "diagnostic.rs"
      via: "canonical denial path pre-computation"
      pattern: "precompute|canonical_denial"
---

<objective>
Cherry-pick cluster C10 — diagnostic/output/denial polish (4 upstream commits from v0.57.0..v0.59.0).

C10 is parallel with C9 (plan 55-03) in Wave 3 because their file sets are completely disjoint:
- C10 touches: exec_strategy.rs, diagnostic.rs, output.rs
- C9 touches: app_runtime.rs, cli.rs, cli_bootstrap.rs, main.rs, pack_update_hint.rs, startup_runtime.rs
No overlap → safe parallel execution.

C10 must run BEFORE C11 (55-05, Wave 4) because C11 also touches exec_strategy.rs. Applying C10 first avoids a conflict where C11's timeout-constant refactor lands on top of C10's denial-path additions.

C10 also touches diagnostic.rs (library crate `crates/nono/src/`) and exec_strategy.rs — both may contain cfg-gated Unix code. Cross-target clippy required per D-55-E3.

Purpose: Absorb the diagnostic/output polish commits.
Output: 4 new commits on the held feature branch with D-19 trailers.
</objective>

<execution_context>
@C:\Users\OMack\.claude\get-shit-done\workflows\execute-plan.md
@C:\Users\OMack\.claude\get-shit-done\templates\summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/STATE.md
@.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md
@.planning/templates/upstream-sync-quick.md
@.planning/templates/cross-target-verify-checklist.md
@.planning/phases/55-upst7-cherry-pick-wave/55-CONTEXT.md

<interfaces>
<!-- C10 commits (chronological — oldest-first) -->
C10 commit order (chronological per git log v0.57.0..v0.59.0):
  1. 8fd8da0c — Bold only path in diagnostic footer, not access type or labels  (v0.59.0)
     Files: crates/nono-cli/src/output.rs
  2. 7cb315c0 — fix: annotate suppressed denials and style save prompt paths (#984)  (v0.59.0)
     Files: crates/nono-cli/src/output.rs + crates/nono/src/diagnostic.rs (and possibly output.rs for styling)
  3. a606b5b5 — diagnostic: pre-compute canonical denial paths to avoid repeated fs I/O  (v0.59.0)
     Files: crates/nono-cli/src/exec_strategy.rs (+7 lines), crates/nono/src/diagnostic.rs (+62 lines, -11)
  4. 668e3410 — fix: use rfind for access mode splitting; add test  (v0.59.0)
     Files: crates/nono-cli/src/output.rs (+18 lines)

All: windows-touch no. Upstream-tag: v0.59.0 for all. Cross-cluster re-export check: clean (per ledger).

cfg-gated Unix code risk:
  - diagnostic.rs (crates/nono/src/): check for cfg(target_os = "linux"/"macos") blocks
  - exec_strategy.rs (crates/nono-cli/src/): the upstream-sync-quick.md conflict-file inventory
    explicitly calls out this file as having "144+ lines of Windows-specific exec wiring" — it IS
    a known cfg-gated file. Cross-target clippy REQUIRED per D-55-E3 + CLAUDE.md MUST.
  - output.rs: likely pure cross-platform string formatting; check for cfg blocks.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: C10 cherry-picks (4 commits, chronological) + cross-target clippy</name>
  <files>crates/nono/src/diagnostic.rs, crates/nono-cli/src/exec_strategy.rs, crates/nono-cli/src/output.rs</files>
  <read_first>
    - crates/nono/src/diagnostic.rs (full current fork state)
    - crates/nono-cli/src/exec_strategy.rs (full current fork state — note all existing cfg-gated Windows regions; C7 may have touched this)
    - crates/nono-cli/src/output.rs (full current fork state)
    - .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md § Cluster C10
    - .planning/templates/upstream-sync-quick.md (fork-divergence catalog entry for exec_strategy.rs: "Fork has 144+ lines of Windows-specific exec wiring")
    - .planning/templates/cross-target-verify-checklist.md (MANDATORY — exec_strategy.rs is in-scope)
    - git show 8fd8da0c (bold-path-only commit)
    - git show 7cb315c0 (suppressed-denial annotation commit)
    - git show a606b5b5 (pre-compute canonical denial paths)
    - git show 668e3410 (rfind access-mode split + test)
  </read_first>
  <action>
Cherry-pick the 4 C10 commits in chronological order:
  1. 8fd8da0c — Bold only path in diagnostic footer
  2. 7cb315c0 — fix: annotate suppressed denials and style save prompt paths (#984)
  3. a606b5b5 — diagnostic: pre-compute canonical denial paths to avoid repeated fs I/O
  4. 668e3410 — fix: use rfind for access mode splitting; add test

For each cherry-pick:
  a. `git cherry-pick <upstream-sha>` from the upstream remote.
  b. Resolve conflicts using these rules:
     - exec_strategy.rs (commit a606b5b5): This file has extensive Windows-specific exec wiring behind cfg-gates. The upstream change adds ~7 lines for pre-computing canonical denial paths. Apply ONLY the cross-platform diagnostic pre-computation additions. DO NOT modify any `#[cfg(target_os = "windows")]` or `exec_strategy_windows/` references. If the upstream diff touches any Windows-specific arm, skip those lines.
     - diagnostic.rs: Apply verbatim (crates/nono library; no Windows-specific sections expected).
     - output.rs: Apply verbatim (pure cross-platform formatting).
  c. Amend each commit to append the verbatim D-19 6-line trailer block (D-55-E2). Upstream-tag is v0.59.0 for all 4.

D-55-E1: verify `git diff --name-only HEAD~4 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines.

D-55-E3 cross-target clippy (MANDATORY for exec_strategy.rs):
  - Run `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
  - Run `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`
  - If cross-toolchain unavailable on Windows dev host: mark PARTIAL per .planning/templates/cross-target-verify-checklist.md § PARTIAL Disposition. Document the exact "Cross-target clippy gate SKIPPED" prose. Set plan as human_needed for live CI confirmation.

D-55-E4: run `cargo test --workspace` and categorise vs Phase 54 baseline SHA. The rfind test (668e3410) adds a new test to output.rs — it must pass.
  </action>
  <verify>
    <automated>
git log --format="%B" HEAD~4..HEAD | grep -c "^Upstream-commit:"
    </automated>
    Must equal 4. Also verify:
      git diff --name-only HEAD~4 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      grep -c "rfind" crates/nono-cli/src/output.rs (must be >= 1)
      grep -c "save skipped\|save_skipped\|suppress_save_prompt" crates/nono/src/diagnostic.rs (must be >= 1)
      cargo build --workspace (exit 0)
      cargo test --workspace (exit 0 or documented carry-forwards)
  </verify>
  <acceptance_criteria>
    - Exactly 4 cherry-pick commits (8fd8da0c → 7cb315c0 → a606b5b5 → 668e3410) with D-19 trailers
    - `git log --format="%B" HEAD~4..HEAD | grep -c "^Upstream-commit:"` equals 4
    - `git diff --name-only HEAD~4 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines (D-55-E1 PASS)
    - exec_strategy.rs diff shows ONLY the cross-platform diagnostic pre-computation additions (a606b5b5); no Windows-cfg-arm changes
    - output.rs contains rfind for access-mode split (668e3410 applied)
    - diagnostic.rs contains [save skipped] annotation logic (7cb315c0 applied)
    - diagnostic.rs contains canonical denial path pre-computation (a606b5b5 applied)
    - `cargo build --workspace` exits 0
    - Cross-target clippy: PASS (clean) or PARTIAL with cross-target-verify-checklist.md prose (exec_strategy.rs is in-scope → MANDATORY)
    - `cargo test --workspace` exits 0; the new rfind test passes (or pre-existing failures documented as carry-forward)
    - Baseline-aware CI gate: lane-transition categories recorded (D-55-E4)
    - Feature branch NOT merged to main (D-55-03)
  </acceptance_criteria>
  <done>C10 cluster cherry-picked; diagnostic polish and denial annotations are in the fork; cross-target clippy verified or PARTIAL-documented.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| exec_strategy.rs → diagnostic.rs | canonical denial path pre-computation changes the diagnostic call shape |
| cfg-gated Unix code (exec_strategy.rs, diagnostic.rs) → cross-target clippy | upstream changes to cfg-gated files must not break Linux/macOS clippy lanes |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-55-04-01 | Information Disclosure | [save skipped] annotations in diagnostic output | accept | Annotations expose that a path is in suppress_save_prompt — this is diagnostic output shown to the user who already knows their profile config; no new untrusted-input-to-output path |
| T-55-04-02 | Tampering | exec_strategy.rs Windows-cfg-arm preservation | mitigate | Conflict resolution rules explicitly require skipping any upstream diff lines that touch cfg(windows) arms; D-55-E1 acceptance criterion verifies no Windows-specific files touched |
| T-55-04-03 | Denial of Service | pre-compute canonical denial paths fs I/O | accept | The change REDUCES fs I/O (pre-compute once vs repeated calls); no new attack surface; upstream-validated |
| T-55-04-SC | Tampering | No new cargo deps in C10 | accept | C10 adds no new Cargo.toml dependencies; no package legitimacy gate needed |
</threat_model>

<verification>
1. `git log --format="%B" HEAD~4..HEAD | grep -c "^Upstream-commit:"` equals 4
2. `git diff --name-only HEAD~4 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines
3. `cargo build --workspace` exits 0
4. Cross-target clippy gate: PASS or PARTIAL documented (exec_strategy.rs is in-scope — MANDATORY)
5. `cargo test --workspace` exits 0 or carry-forwards documented
6. Feature branch NOT merged to main
</verification>

<success_criteria>
- 4 C10 cherry-pick commits on the held feature branch with D-19 trailers
- Diagnostic polish (suppressed-denial annotations, canonical-path pre-compute, bold-path, rfind) live in the fork
- exec_strategy.rs Windows-cfg-arms preserved intact
- Cross-target clippy PASS or PARTIAL-documented per checklist
- D-55-E1, D-55-E4 gates satisfied
</success_criteria>

<output>
Create `.planning/phases/55-upst7-cherry-pick-wave/55-04-SUMMARY.md` when done.
Include: C10 cherry-pick log (4 commits, SHAs, trailer verification); conflict-file inventory (exec_strategy.rs conflict resolution detail); cross-target clippy status (PASS or PARTIAL with exact prose); D-55-E1 PASS; baseline-aware CI gate result; held-branch status.
</output>
