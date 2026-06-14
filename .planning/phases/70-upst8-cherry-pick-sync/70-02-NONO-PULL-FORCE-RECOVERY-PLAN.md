---
phase: 70-upst8-cherry-pick-sync
plan: 02
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/nono-cli/src/package_cmd.rs
  - crates/nono-cli/src/profile_runtime.rs
  - crates/nono-cli/src/wiring.rs
autonomous: true
requirements: [UPST8-02]
must_haves:
  truths:
    - "nono pull --force allows recovery when local package metadata (lockfile entry, .nono-trust.bundle) is missing or corrupted"
    - "nono pull --force with matching content allows write_file directives to adopt existing unmanaged files"
    - "The cherry-pick commit carries a verbatim 6-line D-19 trailer and DCO Signed-off-by"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
    - "No new Cargo.toml dependencies introduced"
  artifacts:
    - path: "crates/nono-cli/src/package_cmd.rs"
      provides: "--force flag on nono pull subcommand"
      contains: "force"
    - path: "crates/nono-cli/src/profile_runtime.rs"
      provides: "lockfile-entry and .nono-trust.bundle existence checks with re-pull prompt"
      contains: "nono-trust.bundle"
    - path: "crates/nono-cli/src/wiring.rs"
      provides: "write_file adopt-existing-unmanaged-file logic when --force is set"
      contains: "force"
  key_links:
    - from: "crates/nono-cli/src/package_cmd.rs"
      to: "crates/nono-cli/src/profile_runtime.rs"
      via: "--force flag threaded through pull execution"
      pattern: "force"
    - from: "crates/nono-cli/src/profile_runtime.rs"
      to: "crates/nono-cli/src/wiring.rs"
      via: "force-mode wiring execution allowing adopt of existing files"
      pattern: "force"
---

<objective>
Plan 70-02 absorbs Cluster C4 — the single merge commit db073750 that adds --force to nono pull for metadata recovery.

This cluster is surface-disjoint from C3 (profile/diagnostic, Plan 70-01) and C2 (network-policy, Plan 70-03): C4 touches only package_cmd.rs, profile_runtime.rs, and wiring.rs — none of which are touched by C3 or C2. C4 can therefore run in Wave 1 in parallel with Plan 70-01.

C4 is a tail-range commit (v0.61.2..v0.62.0) — Phase 63 never saw it. The commit is a "Merge commit from fork" in upstream history; it is cherry-picked with a D-20 Upstream-replayed-from trailer instead of D-19 Upstream-commit, since the upstream subject is "Merge commit from fork" (not an authored feature commit). The executor must choose the correct trailer format based on the actual upstream commit message.

Purpose: Deliver --force recovery for nono pull when lockfile entries or trust bundles are missing or corrupted after partial metadata loss.

Output: One C4 cherry-pick commit on main with the correct D-19 or D-20 trailer and DCO sign-off.
</objective>

<execution_context>
@C:\Users\OMack\.claude\get-shit-done\workflows\execute-plan.md
@C:\Users\OMack\.claude\get-shit-done\templates\summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md
@.planning/templates/upstream-sync-quick.md
@.planning/templates/cross-target-verify-checklist.md

<interfaces>
<!-- C4 commit (from 69-DIVERGENCE-LEDGER.md § Cluster C4) -->
<!-- Cherry-pick source: upstream remote (https://github.com/always-further/nono.git) -->

C4 commit:
  db073750 — Merge commit from fork (nono pull --force recovery)
             Files: package_cmd.rs, profile_runtime.rs, wiring.rs
             Author: Luke Hinds <lukehinds@gmail.com>
             Upstream-tag: v0.62.0
             windows-touch: no
             +294/-67 lines
             Upstream-date: Sun Jun 7 08:21:19 2026 +0100

D-19 trailer format (standard — use if clean cherry-pick):
  Upstream-commit: db073750
  Upstream-tag: v0.62.0
  Upstream-author: Luke Hinds <lukehinds@gmail.com>
  Co-Authored-By: Luke Hinds <lukehinds@gmail.com>
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>

D-20 trailer format (manual replay — use if direct cherry-pick conflicts dominate):
  Upstream-replayed-from: db073750
  Upstream-tag: v0.62.0
  Upstream-author: Luke Hinds <lukehinds@gmail.com>
  Co-Authored-By: Luke Hinds <lukehinds@gmail.com>
  Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>
  Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>

Note: db073750 is a "Merge commit from fork" in upstream — git cherry-pick may or may not apply
cleanly depending on how the fork's diverged package_cmd/profile_runtime/wiring surfaces. If
conflicts dominate, use D-20 manual replay of the upstream intent.

D-70-02: Land on main directly (no held branch).
Plan 70 base SHA: 6667177e
Upstream remote alias: upstream
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: C4 cherry-pick — nono pull --force recovery (db073750)</name>
  <files>
    crates/nono-cli/src/package_cmd.rs,
    crates/nono-cli/src/profile_runtime.rs,
    crates/nono-cli/src/wiring.rs
  </files>
  <behavior>
    - After db073750: nono pull accepts a --force flag; package verification checks for lockfile entry and .nono-trust.bundle existence and prompts re-pull if either is missing; wiring execution in force mode allows write_file to adopt an existing unmanaged file when content matches exactly
    - No new regressions vs plan base SHA 6667177e
  </behavior>
  <read_first>
    - crates/nono-cli/src/package_cmd.rs (full file — understand current nono pull subcommand structure before cherry-picking)
    - crates/nono-cli/src/profile_runtime.rs (full file — understand current profile_runtime pull path; this file has 240+ line additions in the commit)
    - crates/nono-cli/src/wiring.rs (full file — understand current wiring execution logic)
    - .planning/phases/69-upst8-audit/69-DIVERGENCE-LEDGER.md § Cluster C4 (commit row, disposition, cross-cluster re-export check: clean — no cross-cluster deps)
    - .planning/templates/upstream-sync-quick.md (D-19 trailer field order; D-20 Upstream-replayed-from format; fork-divergence catalog)
    - .planning/templates/cross-target-verify-checklist.md (cross-target scope decision tree)
  </read_first>
  <action>
Attempt cherry-pick: git cherry-pick db073750

Since db073750 is a "Merge commit from fork," the cherry-pick may or may not apply cleanly:

CASE A — Clean cherry-pick (no conflicts):
  Amend the commit to append the D-19 trailer (Upstream-commit: db073750).
  Trailer block separated from body by exactly ONE blank line.

CASE B — Conflicts dominate (the fork's package_cmd/profile_runtime/wiring surfaces have diverged significantly):
  Abort the cherry-pick: git cherry-pick --abort
  Manually implement the upstream intent by reading git show db073750 and applying the semantic changes:
    - Add --force flag to the nono pull subcommand in package_cmd.rs
    - Add lockfile-entry and .nono-trust.bundle existence checks in profile_runtime.rs with re-pull prompting
    - Add write_file adopt-existing-unmanaged logic in wiring.rs when force mode is set
  Commit the manual replay with D-20 trailer (Upstream-replayed-from: db073750 instead of Upstream-commit:).

For BOTH cases, the commit message body must describe the upstream intent (the --force recovery feature) and note which trailer format was used (D-19 direct or D-20 replay) and why.

D-70-E1 Windows-only-files invariant check:
  git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"
  Must return 0 lines.

D-70-E3 Cross-target clippy scope check:
  package_cmd.rs, profile_runtime.rs, wiring.rs — inspect for #[cfg(target_os = "linux")], #[cfg(target_os = "macos")], or #[cfg(any(...))] blocks.
  If NO cfg-gated Unix blocks are present in the changed files -> cross-target clippy is N/A for C4; document as N/A in SUMMARY.
  If cfg-gated blocks ARE present -> follow cross-target-verify-checklist.md (run both targets or mark PARTIAL).

D-70-E4 Baseline-aware CI gate (plan base SHA: 6667177e):
  Run: cargo test -p nono-cli
  Categorize transitions vs baseline. Known pre-existing failures (red->red carry-forward, not regressions):
    4 nono-cli failures (profile_cmd init + 3 protected_paths) + 1 nono lib failure (try_set_mandatory_label)

D-70-05 Cargo lockfile check:
  C4 introduces no new Cargo.toml dependencies per the ledger.
  Verify: git diff HEAD~1 HEAD -- Cargo.toml Cargo.lock | grep "^+" | grep -v "^+++" | grep -v "^+#" returns no new dependency entries.
  If the cherry-pick (or manual replay) inadvertently changes Cargo files: investigate and revert the unintended change.

D-70-E5 5-crate workspace: No dep bumps expected for C4 — confirm lockfile unchanged.

IMPORTANT: No .unwrap()/.expect() in any conflict-resolution code. Use ? and NonoError. No #[allow(dead_code)] for new code. If a field or function is added, write or reference existing tests that use it.

Before push: git status --short | grep -E "build_notes|\.gsd" must return 0 lines (repo stays PUBLIC per D-70-02).
  </action>
  <verify>
    <automated>git log --format="%B" HEAD~1..HEAD | grep -v "^#" | grep -c -E "^Upstream-(commit|replayed-from):"</automated>
    Must equal 1 (D-19 or D-20 trailer present on the single C4 commit). Also verify:
      git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      git diff --name-only HEAD~1 HEAD | grep -v -E "package_cmd|profile_runtime|wiring" (must return 0 lines — only the 3 C4 files touched)
      cargo build -p nono-cli (exit 0)
      cargo test -p nono-cli (exit 0 or only pre-existing failures)
  </verify>
  <acceptance_criteria>
    - Exactly 1 commit on main for C4 (db073750 direct pick or D-20 manual replay)
    - Commit message contains either "Upstream-commit: db073750" (D-19) or "Upstream-replayed-from: db073750" (D-20) — one of these two, not both
    - Commit message also contains "Upstream-tag: v0.62.0" and both Signed-off-by lines
    - git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" returns 0 lines (D-70-E1 PASS)
    - git diff --name-only HEAD~1 HEAD shows only package_cmd.rs, profile_runtime.rs, wiring.rs (no other source files)
    - cargo build -p nono-cli exits 0
    - cargo test -p nono-cli exits 0 OR only pre-existing red->red carry-forward failures documented
    - Cross-target clippy status: N/A (if no cfg-gated Unix blocks in changed files) or VERIFIED/PARTIAL (documented in SUMMARY)
    - Cargo.toml and Cargo.lock unchanged (no new dependencies introduced)
    - No .unwrap()/.expect() introduced in any conflict-resolution code
    - git status --short | grep -E "build_notes|\.gsd" returns 0 lines before push
  </acceptance_criteria>
  <done>C4 cherry-pick (or D-20 replay) is on main with correct D-19/D-20 trailer; --force recovery for nono pull is committed; no windows-only files touched.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| upstream commit -> fork main | db073750 absorbed via git cherry-pick (or D-20 replay) from always-further/nono upstream remote |
| nono pull --force -> filesystem | --force recovery allows adopt of existing unmanaged files when content matches — an overwrite-safety boundary |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-70-02-01 | Tampering | --force adopt-existing-file logic in wiring.rs | mitigate | The upstream commit description says write_file in force mode "allows write_file directives to adopt existing unmanaged files if their content exactly matches the pack's source." Executor must verify the content-equality check is a cryptographic or byte-exact comparison, NOT a length or mtime check. If it is a weaker comparison, escalate before landing. |
| T-70-02-02 | Elevation of Privilege | nono pull --force re-pull prompt | mitigate | --force only triggers re-pull prompt; it should NOT bypass trust-bundle verification on the re-pulled artifact. Executor must confirm that re-pulled packages still go through the full trust-bundle verification path (not skip it because "force" was requested). |
| T-70-02-03 | Spoofing | D-19/D-20 trailer integrity | mitigate | The commit carries either Upstream-commit: db073750 (D-19) or Upstream-replayed-from: db073750 (D-20); grep gate verifies presence before push. |
| T-70-02-SC | Tampering | cargo installs during cherry-pick | accept | C4 introduces no new Cargo.toml dependencies per the ledger (pure logic additions); Cargo.toml + Cargo.lock must be unchanged — verified in acceptance criteria. |
</threat_model>

<verification>
After task completes, verify the plan as a whole:

1. git log --format="%B" HEAD~1..HEAD | grep -v "^#" | grep -c -E "^Upstream-(commit|replayed-from):" equals 1 (D-19 or D-20 trailer present)
2. git diff --name-only HEAD~1 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" returns 0 lines (D-70-E1 PASS)
3. git diff --name-only HEAD~1 HEAD shows only the 3 C4 source files
4. git diff HEAD~1 HEAD -- Cargo.toml Cargo.lock shows no new dependency lines (5-crate lockfile stable)
5. cargo build -p nono-cli exits 0
6. cargo test -p nono-cli exits 0 or pre-existing failures documented as carry-forward
7. Cross-target clippy status documented in SUMMARY (N/A or VERIFIED/PARTIAL)
</verification>

<success_criteria>
- One C4 commit on main with correct D-19 or D-20 trailer (db073750)
- --force flag on nono pull is delivered and builds cleanly
- No windows-only files touched (D-70-E1 PASS)
- Cargo.toml and Cargo.lock unchanged (D-70-E5 PASS)
- No new green->red test transitions vs plan base SHA 6667177e
- Cross-target clippy status explicitly recorded
</success_criteria>

<output>
Create .planning/phases/70-upst8-cherry-pick-sync/70-02-SUMMARY.md when done.
Include: C4 cherry-pick log (1 commit, SHA, D-19 or D-20 trailer choice + rationale, conflict-or-clean outcome); baseline-aware CI gate result; cross-target clippy status (N/A or VERIFIED/PARTIAL); D-70-E1 windows-invariant status (PASS); Cargo lockfile status (unchanged); D-70-02 repo-public check result.
</output>
