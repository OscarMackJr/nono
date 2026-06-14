---
phase: 70-upst8-cherry-pick-sync
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - .planning/REQUIREMENTS.md
  - .planning/ROADMAP.md
  - crates/nono-cli/data/nono-profile.schema.json
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile_runtime.rs
  - crates/nono-cli/src/profile_save_runtime.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/sandbox_state.rs
  - crates/nono/src/diagnostic.rs
autonomous: true
requirements: [UPST8-02]
must_haves:
  truths:
    - "REQUIREMENTS.md UPST8-01 acceptance language reflects v0.62.0 upper bound (not v0.61.2)"
    - "ROADMAP.md Phase 70 SC reflects v0.62.0 (D-70-01 amendment applied)"
    - "69-DIVERGENCE-LEDGER.md is byte-identical to pre-task state (immutable)"
    - "Profiles accept a diagnostics.suppress_system_services array that filters matching system service denials from the diagnostic footer"
    - "Profile extends field preserves registry references (e.g. always-further/claude) when saving"
    - "Each cherry-picked commit carries a verbatim 6-line D-19 trailer and DCO Signed-off-by"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
  artifacts:
    - path: ".planning/REQUIREMENTS.md"
      provides: "Updated UPST8-01 acceptance language citing v0.62.0 upper bound"
      contains: "v0.62.0"
    - path: "crates/nono/src/diagnostic.rs"
      provides: "suppressed_system_service_operations filtering logic"
      contains: "suppress"
    - path: "crates/nono-cli/src/profile/mod.rs"
      provides: "diagnostics.suppress_system_services profile field deserialization"
      contains: "suppress_system_service"
    - path: "crates/nono-cli/src/sandbox_prepare.rs"
      provides: "suppressed_system_service_operations field on PreparedSandbox (prerequisite for Plan 70-03 C2)"
      contains: "suppressed_system_service_operations"
    - path: "crates/nono-cli/src/profile_save_runtime.rs"
      provides: "Registry ref preservation in profile extends"
      contains: "registry"
  key_links:
    - from: "crates/nono-cli/src/sandbox_prepare.rs"
      to: "crates/nono-cli/src/exec_strategy.rs"
      via: "PreparedSandbox struct field suppressed_system_service_operations"
      pattern: "suppressed_system_service_operations"
    - from: "crates/nono-cli/src/profile/mod.rs"
      to: "crates/nono/src/diagnostic.rs"
      via: "DiagnosticFormatter suppress list"
      pattern: "suppress"
---

<objective>
Plan 70-01 does two things in sequence:

1. D-70-01 REQ/SC amendment (planning docs only): Amend .planning/REQUIREMENTS.md UPST8-01 and .planning/ROADMAP.md Phase 70 SC to reflect the v0.62.0 upper bound (the Phase 69 D-01 range correction adds +3 tail commits beyond the SC-locked v0.61.2). The 69-DIVERGENCE-LEDGER.md stays byte-identical. This is a planning-artifact commit only — zero source edits.

2. Cluster C3 cherry-picks — profile/diagnostic feature additions (cc21229f + 20cc5df9): Absorb the two cross-platform profile/diagnostic feature commits from upstream v0.60.0..v0.62.0. These commits are prerequisites for Plan 70-03 (C2 network-policy hardening): bd4c469a references the suppressed_system_service_operations field on PreparedSandbox that cc21229f introduces. C3 MUST land before C2.

Purpose: Deliver the diagnostics.suppress_system_services profile option and the registry-ref-in-extends feature; establish the PreparedSandbox field dependency that unblocks Plan 70-03 (C2).

Output: Updated REQUIREMENTS.md + ROADMAP.md (D-70-01 amendment); two C3 cherry-pick commits on main with D-19 trailers and DCO sign-off.
</objective>

<execution_context>
@C:\Users\OMack\.claude\get-shit-done\workflows\execute-plan.md
@C:\Users\OMack\.claude\get-shit-done\templates\summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/STATE.md
@.planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md
@.planning/phases/69-upst8-audit/69-01-SUMMARY.md
@.planning/templates/upstream-sync-quick.md
@.planning/templates/cross-target-verify-checklist.md

<interfaces>
<!-- Key upstream commits for C3 (from 69-DIVERGENCE-LEDGER.md § Cluster C3) -->
<!-- Cherry-pick source: upstream remote (https://github.com/always-further/nono.git) -->
<!-- SHA COLLISION LANDMINE: local fork tag v0.62.0 = 3c5e9025 (divergent history); -->
<!-- upstream v0.62.0 = 52809dda. NEVER use tag in git range; always use literal SHAs. -->

C3 commits (chronological, oldest-first per git log):
  cc21229f — feat(diagnostic): add profile option to suppress system service diagnostics (#1059)
             Files: nono-profile.schema.json, command_runtime.rs, exec_strategy.rs (UNIX, not _windows/),
                    execution_runtime.rs, launch_runtime.rs, main.rs, policy.rs, profile/mod.rs,
                    profile_runtime.rs, sandbox_prepare.rs, diagnostic.rs
             Author: Luke Hinds <lukehinds@gmail.com>
             Upstream-tag: v0.61.0
             windows-touch: no
             Upstream-date: Tue Jun 2 06:10:32 2026 +0100

  20cc5df9 — feat(profile): allow registry refs in profile extends (#1061)
             Files: profile_save_runtime.rs, sandbox_state.rs
             Author: Luke Hinds <lukehinds@gmail.com>
             Upstream-tag: v0.61.0
             windows-touch: no
             Upstream-date: Tue Jun 2 10:31:20 2026 +0100

Critical field introduced by C3 (required by C2 / Plan 70-03):
  PreparedSandbox.suppressed_system_service_operations: Vec<String>
  (introduced in sandbox_prepare.rs by cc21229f)

D-19 trailer format (verbatim from .planning/templates/upstream-sync-quick.md):
  Field order (FIXED): Upstream-commit -> Upstream-tag -> Upstream-author -> Co-Authored-By -> Signed-off-by (full name) -> Signed-off-by (handle)
  Upstream-commit: <8-char sha>
  Upstream-tag: v0.61.0
  Upstream-author: Luke Hinds <lukehinds@gmail.com>
  Co-Authored-By: Luke Hinds <lukehinds@gmail.com>
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>

D-70-02: Land cherry-picks on main directly (no held branch; no signed-release-ride hazard this cycle).
Plan 70 base SHA (Phase 69 close): 6667177e
Upstream remote alias: upstream
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: D-70-01 — Amend REQUIREMENTS.md and ROADMAP.md to reflect v0.62.0 upper bound</name>
  <files>.planning/REQUIREMENTS.md, .planning/ROADMAP.md</files>
  <read_first>
    - .planning/REQUIREMENTS.md (full file — read before editing; find UPST8-01 acceptance block)
    - .planning/ROADMAP.md (full file — read before editing; find Phase 70 SC block)
    - .planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md § Headline and § range_note (D-01 range correction authority)
    - .planning/phases/69-upst8-audit/69-01-SUMMARY.md § Decisions Made (D-01 range extension confirmed)
  </read_first>
  <action>
Edit REQUIREMENTS.md § UPST8-01 acceptance criteria:
  - Change the range reference from "v0.60.0..v0.61.2" to "v0.60.0..v0.62.0" in the UPST8-01 requirement body.
  - Add a parenthetical citing the 69-DIVERGENCE-LEDGER.md D-01 as authority: "(range extended to v0.62.0 per D-01 in 69-DIVERGENCE-LEDGER.md; +3 tail commits absorbed in Phase 70)".
  - Do NOT change any other requirement text; do NOT touch the Traceability table status fields.

Edit ROADMAP.md § Phase 70 SC (Success Criteria):
  - Update any SC text that references "v0.60.0..v0.61.2" to read "v0.60.0..v0.62.0".
  - If Phase 69 SC also references v0.61.2, add a note "(D-01 extended to v0.62.0; see 69-DIVERGENCE-LEDGER.md)".
  - Do NOT change any other phase entries or sections.

Do NOT edit .planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md — it is the immutable audit-of-record.

Commit message subject: "docs(70): D-70-01 amend UPST8-01 + ROADMAP Phase 70 SC to v0.62.0 upper bound"
Body must explain the D-01 authority (Phase 69 audit extended range from SC-locked v0.61.2 to v0.62.0 adding +3 tail commits).
DCO sign-off required: Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
No Upstream-commit trailer (this is a fork-internal planning amendment, not a cherry-pick).
  </action>
  <verify>
    <automated>git diff HEAD~1 HEAD -- .planning/REQUIREMENTS.md | grep "v0.62.0" | wc -l</automated>
    Must be greater than 0 (the amendment appears in the diff). Also verify:
      grep "v0.62.0" .planning/REQUIREMENTS.md (must find the updated UPST8-01 text)
      git diff HEAD~1 HEAD -- .planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md (must be empty — immutable)
      git diff HEAD~1 HEAD -- crates/ bindings/ scripts/ Makefile (must be empty — zero source edits)
  </verify>
  <acceptance_criteria>
    - REQUIREMENTS.md UPST8-01 references v0.62.0 (not v0.61.2) as the upper bound
    - ROADMAP.md Phase 70 SC reflects v0.62.0 as the actual range
    - 69-DIVERGENCE-LEDGER.md git diff HEAD~1 HEAD is empty (byte-identical)
    - Commit message body cites 69-DIVERGENCE-LEDGER.md D-01 as authority
    - Commit carries Signed-off-by: Oscar Mack Jr &lt;oscar.mack.jr@gmail.com&gt;
    - git diff HEAD~1 HEAD -- crates/ bindings/ scripts/ Makefile is empty (zero source edits)
  </acceptance_criteria>
  <done>Planning artifacts reflect the v0.62.0 upper bound; ledger is untouched.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: C3 cherry-picks — profile/diagnostic suppress + registry-ref-in-extends (cc21229f + 20cc5df9)</name>
  <files>
    crates/nono-cli/data/nono-profile.schema.json,
    crates/nono-cli/src/command_runtime.rs,
    crates/nono-cli/src/exec_strategy.rs,
    crates/nono-cli/src/execution_runtime.rs,
    crates/nono-cli/src/launch_runtime.rs,
    crates/nono-cli/src/main.rs,
    crates/nono-cli/src/policy.rs,
    crates/nono-cli/src/profile/mod.rs,
    crates/nono-cli/src/profile_runtime.rs,
    crates/nono-cli/src/profile_save_runtime.rs,
    crates/nono-cli/src/sandbox_prepare.rs,
    crates/nono-cli/src/sandbox_state.rs,
    crates/nono/src/diagnostic.rs
  </files>
  <behavior>
    - After cc21229f: profile/mod.rs deserializes diagnostics.suppress_system_services: Vec[String]; PreparedSandbox.suppressed_system_service_operations field exists; DiagnosticFormatter filters violations matching suppressed names from the footer; test_suppressed_system_service_violations_do_not_offer_profile_save passes
    - After 20cc5df9: profile_save_runtime.rs prepare_profile_save_from_patch preserves registry references (e.g. "always-further/claude") in the extends field; sandbox_state tests use tempdir_in("/tmp") for isolation
  </behavior>
  <read_first>
    - crates/nono-cli/src/exec_strategy.rs (read before cherry-picking — identify cfg-gated Unix blocks that must not be disturbed)
    - crates/nono-cli/src/sandbox_prepare.rs (read before cherry-picking — understand current PreparedSandbox struct; C2 plan will add strict_filter here)
    - crates/nono/src/diagnostic.rs (read before cherry-picking — understand current DiagnosticFormatter shape)
    - crates/nono-cli/src/profile/mod.rs (read before cherry-picking — understand current Profile struct and deserialization)
    - .planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md § Cluster C3 (commit rows, cross-cluster re-export check, windows-touch: no)
    - .planning/templates/upstream-sync-quick.md (D-19 trailer field order, fork-divergence catalog)
    - .planning/templates/cross-target-verify-checklist.md (MANDATORY — exec_strategy.rs and sandbox_prepare.rs are cfg-gated Unix surfaces)
  </read_first>
  <action>
Cherry-pick C3 commits from the upstream remote in chronological order (oldest-first):

  1. cc21229f — feat(diagnostic): add profile option to suppress system service diagnostics (#1059)
     Cherry-pick: git cherry-pick cc21229f
     If conflicts arise in exec_strategy.rs: apply upstream changes around the cfg-gated Unix regions; do not touch any Windows arms or exec_strategy_windows/ files. The upstream commit touches only the UNIX exec_strategy.rs (not exec_strategy_windows/).
     If conflicts arise in sandbox_prepare.rs: apply upstream's suppressed_system_service_operations Vec<String> field addition cleanly — this field is the C2 prerequisite.
     After successful cherry-pick (or conflict resolution), amend the commit to append the verbatim D-19 trailer:
       Upstream-commit: cc21229f
       Upstream-tag: v0.61.0
       Upstream-author: Luke Hinds <lukehinds@gmail.com>
       Co-Authored-By: Luke Hinds <lukehinds@gmail.com>
       Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
       Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
     Trailer block separated from body by exactly ONE blank line.

  2. 20cc5df9 — feat(profile): allow registry refs in profile extends (#1061)
     Cherry-pick: git cherry-pick 20cc5df9
     If conflicts arise in profile_save_runtime.rs or sandbox_state.rs: apply upstream intent while preserving any fork-specific divergences.
     Amend the commit with the verbatim D-19 trailer:
       Upstream-commit: 20cc5df9
       Upstream-tag: v0.61.0
       Upstream-author: Luke Hinds <lukehinds@gmail.com>
       Co-Authored-By: Luke Hinds <lukehinds@gmail.com>
       Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
       Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>

D-70-E1 Windows-only-files invariant: After both cherry-picks, run:
  git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"
  Must return 0 lines.

D-70-E3 Cross-target clippy (MANDATORY per CLAUDE.md and cross-target-verify-checklist.md):
  cc21229f touches exec_strategy.rs (cfg-gated Unix code) and sandbox_prepare.rs — cross-target verification is REQUIRED.
  Run: cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
  Run: cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
  If the cross-toolchain is unavailable on the Windows dev host (ring/aws-lc-sys C-toolchain missing):
    Mark cross-target gate as PARTIAL per cross-target-verify-checklist.md § PARTIAL Disposition.
    Add human_verification_truths entry: "GH Actions Linux Clippy + macOS Clippy lanes on HEAD must report green."
    NEVER flip to VERIFIED on Windows-host cargo check alone.

D-70-E4 Baseline-aware CI gate (plan base SHA: 6667177e):
  Run: cargo test -p nono-cli and cargo test -p nono (or cargo test --workspace on Windows)
  Categorize each test transition vs baseline 6667177e:
    green->green: PASS | green->red: FAIL (regression) | red->red: carry-forward | red->green: improvement
  Known pre-existing Windows test failures (do NOT chase as regressions):
    4 nono-cli failures (profile_cmd init + 3 protected_paths) + 1 nono lib failure (try_set_mandatory_label)
    These are red->red carry-forward per memory nono_cli_windows_baseline_test_failures.

D-70-02: Land directly on main (no held branch). Before any push:
  Verify no build_notes/ or .gsd/ staged: git status --short | grep -E "build_notes|\.gsd" must return 0 lines.

IMPORTANT: Do not use .unwrap() or .expect() in any conflict resolution code you write. Use ? operator and NonoError propagation per CLAUDE.md coding standards.
  </action>
  <verify>
    <automated>git log --format="%B" HEAD~2..HEAD | grep -v "^#" | grep -c "^Upstream-commit:"</automated>
    Must equal 2 (one per cherry-picked commit). Also verify:
      git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      grep -n "suppressed_system_service_operations" crates/nono-cli/src/sandbox_prepare.rs (must find the field)
      cargo build -p nono (exit 0)
      cargo build -p nono-cli (exit 0)
      cargo test -p nono-cli (exit 0 or pre-existing failures documented as carry-forward)
  </verify>
  <acceptance_criteria>
    - Exactly 2 cherry-pick commits on main, one per C3 SHA (cc21229f, 20cc5df9)
    - Each commit message contains the verbatim 6-line D-19 trailer with correct field order: Upstream-commit (8-char) -> Upstream-tag -> Upstream-author -> Co-Authored-By -> Signed-off-by (full name) -> Signed-off-by (handle)
    - git log --format="%B" HEAD~2..HEAD | grep -v "^#" | grep -c "^Upstream-commit:" equals 2
    - git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" returns 0 lines (D-70-E1 PASS)
    - grep "suppressed_system_service_operations" crates/nono-cli/src/sandbox_prepare.rs finds the field (C2 prerequisite present)
    - cargo build -p nono exits 0
    - cargo build -p nono-cli exits 0
    - cargo test -p nono-cli exits 0 OR only pre-existing red->red failures documented (no new green->red)
    - Cross-target clippy: VERIFIED (both targets clean) OR PARTIAL (toolchain unavailable, CI deferred per cross-target-verify-checklist.md § PARTIAL Disposition) — status explicitly recorded in SUMMARY
    - No push without verifying git status --short shows no build_notes/ or .gsd/ staged (D-70-02 repo stays PUBLIC)
    - No .unwrap()/.expect() introduced in any conflict-resolution code
  </acceptance_criteria>
  <done>C3 cherry-picks are on main with correct D-19 trailers; suppressed_system_service_operations field exists on PreparedSandbox (unblocking Plan 70-03); cross-target clippy status recorded.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| upstream commit -> fork main | cc21229f + 20cc5df9 absorbed via git cherry-pick from the always-further/nono upstream remote |
| profile -> diagnostic filter | diagnostics.suppress_system_services accepts user-provided strings that filter violation names from the footer |
| profile extends field -> registry resolver | extends can now hold registry references (e.g. always-further/claude) that survive prepare_profile_save_from_patch |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-70-01-01 | Information Disclosure | diagnostics.suppress_system_services | mitigate | Suppression ONLY affects the diagnostic footer (reporting); the sandbox still enforces the underlying denials. cc21229f commit message confirms: "This suppression only affects the diagnostic reporting; the sandbox still enforces the underlying denials." Verify in code that the filter does not short-circuit enforcement. |
| T-70-01-02 | Tampering | 69-DIVERGENCE-LEDGER.md (immutable audit-of-record) | mitigate | Task 1 acceptance criteria requires git diff HEAD~1 HEAD -- .planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md to be empty; enforced before Task 2 cherry-picks. |
| T-70-01-03 | Elevation of Privilege | profile extends registry ref path traversal | mitigate | Registry reference strings (e.g. "always-further/claude") go through the existing profile resolver validation path. Executor must confirm 20cc5df9 does NOT relax path-component validation for the extends value (no raw string starts_with; must use structured comparison per CLAUDE.md § Path Handling). |
| T-70-01-04 | Spoofing | D-19 trailer integrity | mitigate | Each cherry-picked commit carries the upstream SHA in the trailer; git log grep-c gate verifies presence; immutable after push. |
| T-70-01-SC | Tampering | cargo installs during cherry-pick | accept | C3 introduces no new Cargo.toml dependencies (pure logic additions to existing crates); no package legitimacy audit entry needed. Verify: git diff HEAD~2 HEAD -- Cargo.toml Cargo.lock returns no new dependency lines. |
</threat_model>

<verification>
After both tasks complete, verify the plan as a whole:

1. git diff HEAD~3 HEAD~2 -- .planning/REQUIREMENTS.md | grep "v0.62.0" returns at least 1 match (D-70-01 amendment present)
2. git diff HEAD~3 HEAD~2 -- .planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md returns empty (ledger immutable)
3. git log --format="%B" HEAD~2..HEAD | grep -v "^#" | grep -c "^Upstream-commit:" equals 2 (D-19 trailers present)
4. git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" returns 0 lines (D-70-E1 PASS)
5. grep "suppressed_system_service_operations" crates/nono-cli/src/sandbox_prepare.rs finds the field (C2/Plan 70-03 prerequisite PRESENT)
6. cargo build -p nono exits 0
7. cargo build -p nono-cli exits 0
8. Cross-target clippy status recorded (VERIFIED or PARTIAL with CI deferral)
9. git status --short | grep -E "build_notes|\.gsd" returns 0 lines before any push (repo stays PUBLIC)
</verification>

<success_criteria>
- REQUIREMENTS.md and ROADMAP.md reflect v0.62.0 upper bound (D-70-01 done)
- 69-DIVERGENCE-LEDGER.md is byte-identical to its pre-plan state (audit-of-record immutability preserved)
- Two C3 cherry-pick commits on main with correct D-19 trailers (cc21229f + 20cc5df9)
- PreparedSandbox.suppressed_system_service_operations field exists (Plan 70-03 prerequisite)
- No windows-only files touched (D-70-E1 PASS)
- Cross-target clippy: VERIFIED or PARTIAL with explicit CI deferral (never silently skipped)
- cargo build --workspace exits 0
- No new green->red test transitions vs plan base SHA 6667177e
</success_criteria>

<output>
Create .planning/phases/70-upst8-cherry-pick-sync/70-01-SUMMARY.md when done.
Include: D-70-01 amendment summary (what changed in REQUIREMENTS.md + ROADMAP.md); C3 cherry-pick log (2 commits, SHAs, conflict inventory, trailer verification result); PreparedSandbox.suppressed_system_service_operations field confirmation (Plan 70-03 prerequisite status); baseline-aware CI gate result; cross-target clippy status (VERIFIED or PARTIAL with CI deferral note); D-70-E1 windows-invariant status (PASS); D-70-02 repo-public check result.
</output>
