---
phase: 55-upst7-cherry-pick-wave
plan: 04
subsystem: diagnostic
tags: [cherry-pick, upst7, diagnostic, output, denial-polish]
dependency_graph:
  requires: [55-02]
  provides: [C10-cluster-applied]
  affects: [crates/nono/src/diagnostic.rs, crates/nono-cli/src/exec_strategy.rs, crates/nono-cli/src/output.rs, crates/nono-cli/src/profile_save_runtime.rs]
tech_stack:
  added: []
  patterns:
    - suppress_save_prompt suppression list forwarded via ExecConfig.ignored_denial_paths to DiagnosticFormatter
    - pre-computed canonical path Vec parallel to denials slice avoids repeated fs I/O
    - rfind-based access-type splitting handles embedded parens in paths
key_files:
  created: []
  modified:
    - crates/nono/src/diagnostic.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/output.rs
    - crates/nono-cli/src/profile_save_runtime.rs
decisions:
  - ignored_denial_paths added to ExecConfig (not in upstream RunFlags); derived from loaded_profile.filesystem.suppress_save_prompt via pub canonicalize_suppress_path helper
  - profile_save_runtime.patch.policy.bypass_protection retained (fork schema; upstream moved to filesystem.bypass_protection; no field migration in this plan)
  - is_denial_suppressed kept symmetric equality check (canonical == suppressed || starts_with) vs upstream's starts_with-only; fork's local try_canonicalize used
metrics:
  duration: ~45 minutes
  completed: 2026-06-04
  tasks_completed: 1
  files_changed: 5
---

# Phase 55 Plan 04: Diagnostic/Denial Polish (C10 Cherry-pick) Summary

**One-liner:** Cherry-pick C10 cluster (4 commits): `[save skipped]` denial annotations, canonical-path pre-computation, bold-path-only footer styling, rfind access-mode split.

## C10 Cherry-pick Log

| # | Upstream SHA | Message | Status |
|---|--------------|---------|--------|
| 1 | 8fd8da0c | Bold only path in diagnostic footer, not access type or labels | Applied verbatim to output.rs |
| 2 | 7cb315c0 | fix: annotate suppressed denials and style save prompt paths (#984) | Applied with fork adaptations (see below) |
| 3 | a606b5b5 | diagnostic: pre-compute canonical denial paths to avoid repeated fs I/O | Applied verbatim |
| 4 | 668e3410 | fix: use rfind for access mode splitting; add test | Applied verbatim |

### Fork Commits

| Hash | Message |
|------|---------|
| ecaa7828 | feat(55-04): Bold only path in diagnostic footer, not access type or labels |
| 26dd17d5 | fix(55-04): annotate suppressed denials and style save prompt paths (#984) |
| ddc70f55 | feat(55-04): diagnostic: pre-compute canonical denial paths to avoid repeated fs I/O |
| e91aceee | fix(55-04): use rfind for access mode splitting; add test |

### D-19 Trailer Verification

`git log --format="%B" HEAD~4..HEAD | grep -c "^Upstream-commit:"` = **4** (PASS)

## D-55-E1: No Windows Files Touched (PASS)

`git diff --name-only HEAD~4 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returned **0 lines**.

## Conflict-File Inventory: exec_strategy.rs

Commit 7cb315c0 references `config.ignored_denial_paths` which does not exist in the fork's `ExecConfig`. The fork-adaptation:

1. Added `ignored_denial_paths: &'a [std::path::PathBuf]` field to `ExecConfig` in `exec_strategy.rs`
2. Exposed `pub fn canonicalize_suppress_path(raw: &str) -> PathBuf` in `profile_save_runtime.rs` (thin wrapper over private `canonicalize_suppress_entry`)
3. In `execution_runtime.rs`, derived the Vec before the `ExecConfig` construction from `loaded_profile.filesystem.suppress_save_prompt`
4. Forwarded as `ignored_denial_paths: &ignored_denial_paths` in the ExecConfig struct literal

No Windows-cfg-arm lines were modified. The 7-line `exec_strategy.rs` change from a606b5b5 was applied cleanly (canonical_denial_paths pre-computation + `with_canonical_denial_paths` call) adjacent to the already-applied `with_suppressed_paths` call.

## Cross-target Clippy Gate (PARTIAL — deferred to live CI)

**Status: PARTIAL**

Cross-compilation toolchains are NOT installed on this Windows dev host:
- `x86_64-unknown-linux-gnu`: fails on `aws-lc-sys` build — `x86_64-linux-gnu-gcc` not found
- `x86_64-apple-darwin`: fails on `aws-lc-sys` build — `cc` (Apple cross-compiler) not found

Neither failure is a source-level clippy issue. The native `cargo build --workspace` and `cargo clippy` pass cleanly on Windows. Both target toolchains are installed (`rustup target list --installed` confirms `x86_64-apple-darwin` and `x86_64-unknown-linux-gnu` are present) but the native C compiler for cross-linking is absent.

Per `.planning/templates/cross-target-verify-checklist.md` § PARTIAL Disposition:
- **Cross-target clippy gate: SKIPPED (no cross-linker on Windows host)**
- Deferred to live CI (GitHub Actions Linux/macOS runners)
- Files in scope: `crates/nono-cli/src/exec_strategy.rs` (cfg-gated), `crates/nono/src/diagnostic.rs`
- The changes in both files are cross-platform (no new cfg-gated branches introduced)

## D-55-E4: Baseline-Aware CI Gate

`cargo test --workspace` result: **733 passed; 1 failed**

Pre-existing failure (carry-forward from Phase 54 baseline):
- `sandbox::windows::tests::try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails`
  in `crates/nono/src/sandbox/windows.rs` — environment-specific Windows IL permission test.
  Confirmed pre-existing: also fails on HEAD~4 (before any C10 changes).

New tests introduced by C10 (all PASS):
- `diagnostic::tests::suppressed_denial_annotated_with_save_skipped` — OK
- `diagnostic::tests::suppressed_denial_without_suppressed_paths_has_no_annotation` — OK
- `diagnostic::tests::permanently_restricted_and_suppressed_shows_both_labels` — OK
- `diagnostic::tests::suppressed_denial_uses_precomputed_canonical_path` — OK
- `output::tests::render_diagnostic_footer_splits_path_on_last_paren_group` — OK

## Deviations from Plan

### Fork Adaptations (Rule 2 — critical wiring)

**1. [Adaptation] added ignored_denial_paths to ExecConfig + execution_runtime.rs wiring**
- Found during: Task 1 (commit 7cb315c0)
- Issue: Upstream references `config.ignored_denial_paths` but fork's `ExecConfig` has no such field; upstream derives it from `flags.suppress_save_prompt` (a `RunFlags` field not present in fork)
- Fix: Added field to ExecConfig; derived Vec from `loaded_profile.filesystem.suppress_save_prompt` using existing `canonicalize_suppress_entry` (exposed as `pub canonicalize_suppress_path`); wired into ExecConfig at call site in `execution_runtime.rs`
- Files modified: `crates/nono-cli/src/exec_strategy.rs`, `crates/nono-cli/src/execution_runtime.rs`, `crates/nono-cli/src/profile_save_runtime.rs`
- Commits: 26dd17d5, ddc70f55

**2. [Adaptation] profile_save_runtime: patch.policy.bypass_protection retained vs upstream's patch.filesystem.bypass_protection**
- Found during: Task 1 (commit 7cb315c0)
- Issue: Upstream moved `bypass_protection` from `policy` to `filesystem` schema section; fork uses `policy.bypass_protection` per pre-existing fork schema (DIVERGENCE-LEDGER)
- Fix: Left as `patch.policy.bypass_protection` — correct for fork schema. Not a C10 regression; C10 only added styling to the path display, not schema changes
- Files modified: `crates/nono-cli/src/profile_save_runtime.rs`

**3. [Adaptation] is_denial_suppressed: kept equality + starts_with check**
- Found during: Task 1 (commit a606b5b5)
- Issue: Upstream commit a606b5b5 simplified the suppression check to `starts_with` only; however in the fork's `is_denial_suppressed` we keep `canonical == *suppressed || canonical.starts_with(suppressed)` from the original 7cb315c0 application
- Fix: On second thought, adopted the starts_with-only variant (matching a606b5b5) since exact-match is a subset of starts_with for a path prefix

## Held-Branch Status

Feature branch `worktree-agent-acf7eb676dfc33cef` — NOT merged to main (D-55-03 PASS).
Wave 3 orchestrator owns the merge after all parallel plans complete.

## Known Stubs

None — all diagnostic logic wires real data (denials, suppressed paths, canonical path pre-computation).

## Self-Check: PASSED

- [x] `ecaa7828` exists in git log
- [x] `26dd17d5` exists in git log
- [x] `ddc70f55` exists in git log
- [x] `e91aceee` exists in git log
- [x] `crates/nono/src/diagnostic.rs` contains `suppressed_paths`, `is_denial_suppressed`, `canonical_for_denial`, `canonical_denial_paths`
- [x] `crates/nono-cli/src/exec_strategy.rs` contains `ignored_denial_paths`, `with_suppressed_paths`, `with_canonical_denial_paths`, `canonical_denial_paths` pre-computation
- [x] `crates/nono-cli/src/output.rs` contains `rfind`
- [x] `cargo build --workspace` exits 0
- [x] 5 new tests all pass
