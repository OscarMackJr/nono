---
phase: 55-upst7-cherry-pick-wave
plan: 03
type: execute
wave: 3
depends_on: [55-02]
files_modified:
  - crates/nono-cli/src/app_runtime.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/pack_update_hint.rs
  - crates/nono-cli/src/startup_runtime.rs
  - docs/cli/features/managing-packs.mdx
  - docs/cli/usage/flags.mdx
autonomous: true
requirements: [REQ-UPST7-02]
must_haves:
  truths:
    - "pack-update-hint refresh runs in a detached process (not a background thread) to avoid threading issues before fork in supervised mode"
    - "NONO_NO_PACK_UPDATE_HINTS env var disables pack update hints independently of the general update check opt-out"
    - "pack-update-hint-helper subcommand dispatches the out-of-process refresh"
    - "Pack update hint state file writes are atomic (write-to-temp + rename)"
    - "Each commit carries a verbatim 6-line D-19 trailer + DCO Signed-off-by"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
    - "startup_runtime.rs changes from this plan do not conflict with C11's startup_runtime.rs changes (wave 4)"
  artifacts:
    - path: "crates/nono-cli/src/pack_update_hint.rs"
      provides: "detached-process refresh + atomic state writes + run_refresh_helper debug log"
    - path: "crates/nono-cli/src/main.rs"
      provides: "pack-update-hint-helper subcommand dispatch"
    - path: "crates/nono-cli/src/cli.rs"
      provides: "NONO_NO_PACK_UPDATE_HINTS + --pack-update-hint-helper flag"
  key_links:
    - from: "pack_update_hint.rs"
      to: "main.rs"
      via: "pack-update-hint-helper subcommand dispatch"
      pattern: "pack.update.hint.helper"
---

<objective>
Cherry-pick cluster C9 — pack-update-hint robustness (2 upstream commits from v0.57.0..v0.59.0).

C9 is parallel with C10 (plan 55-04) in Wave 3 because their file sets are disjoint:
- C9 touches: app_runtime.rs, cli.rs, cli_bootstrap.rs, main.rs, pack_update_hint.rs, startup_runtime.rs, docs/
- C10 touches: exec_strategy.rs, diagnostic.rs, output.rs
No overlap → Wave 3 parallel execution.

C9 must run AFTER C7 (55-02) because the C7 cluster also touches cli.rs, startup_runtime.rs, and main.rs — applying C9 on top of the C7 cherry-picks avoids conflicts.

C9 must run BEFORE C11 (55-05, Wave 4) because C11's `194788ee` also touches startup_runtime.rs and cli.rs.

Purpose: Absorb the detached-process pack-hint refresh and atomic state file write commits.
Output: 2 new commits on the held feature branch with D-19 trailers.
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
@.planning/phases/55-upst7-cherry-pick-wave/55-CONTEXT.md

<interfaces>
<!-- C9 commits (chronological — oldest-first) -->
C9 commit order:
  1. 74fbbf12 — refactor(pack-hints): refresh in detached process to avoid threads  (v0.58.0)
     Files: app_runtime.rs, cli.rs, cli_bootstrap.rs, main.rs, pack_update_hint.rs,
            startup_runtime.rs, docs/cli/features/managing-packs.mdx, docs/cli/usage/flags.mdx
  2. b1a650a3 — fix(pack-update-hint): make state file writes atomic                (v0.58.0)
     Files: pack_update_hint.rs

Both: windows-touch no. Cross-cluster re-export check: clean (per ledger).
Upstream-tag: v0.58.0 for both.

Key platform concern: 74fbbf12 introduces a detached child process via process::Command. The fork's Windows exec path uses CreateProcessW/broker; verify the detached-process spawn in pack_update_hint.rs uses the cross-platform `std::process::Command` (not fork/exec directly). If it does, the change is safe on Windows without cfg-gates.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: C9 cherry-picks (2 commits, chronological)</name>
  <files>crates/nono-cli/src/app_runtime.rs, crates/nono-cli/src/cli.rs, crates/nono-cli/src/cli_bootstrap.rs, crates/nono-cli/src/main.rs, crates/nono-cli/src/pack_update_hint.rs, crates/nono-cli/src/startup_runtime.rs, docs/cli/features/managing-packs.mdx, docs/cli/usage/flags.mdx</files>
  <read_first>
    - crates/nono-cli/src/pack_update_hint.rs (full current fork state)
    - crates/nono-cli/src/main.rs (full current fork state — to understand existing subcommand dispatch)
    - crates/nono-cli/src/startup_runtime.rs (full current fork state — C7 may have touched this; read post-C7-cherry-picks)
    - .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md § Cluster C9
    - .planning/templates/upstream-sync-quick.md (D-19 trailer + fork-divergence catalog)
    - git show 74fbbf12 (full diff — inspect whether detached process spawn uses std::process::Command)
    - git show b1a650a3 (full diff — atomic write pattern)
  </read_first>
  <action>
Cherry-pick the 2 C9 commits in chronological order:
  1. 74fbbf12 — refactor(pack-hints): refresh in detached process to avoid threads
  2. b1a650a3 — fix(pack-update-hint): make state file writes atomic

For each cherry-pick:
  a. `git cherry-pick <upstream-sha>` from the upstream remote.
  b. Resolve conflicts (if any):
     - cli.rs: C7 already added cli.rs changes (target_binary, JSONC flags). C9 adds NONO_NO_PACK_UPDATE_HINTS + --pack-update-hint-helper. Apply C9's additions around the existing C7 additions; no removal of C7 additions.
     - startup_runtime.rs: C7 may have modified startup_runtime.rs (opencode extraction commit 2bd9b4d5 touches it via startup_runtime.rs changes in 74fbbf12). Apply C9's pack-hint-helper guard (`disable_pre_execution_update_checks`) around the existing code.
     - main.rs: C7 adds to main.rs (opencode dispatch). C9 adds the pack-update-hint-helper subcommand dispatch. Apply both; they dispatch to different subcommands.
     - docs/: Apply verbatim (no fork divergence in docs).
  c. Platform check on 74fbbf12: after cherry-pick, verify `grep -n "process::Command\|fork()\|libc::fork" crates/nono-cli/src/pack_update_hint.rs` — the spawn MUST use `std::process::Command` (cross-platform), NOT `fork()` / `libc::fork`. If the upstream commit uses fork() or cfg-gated spawn, document the Windows compat note in the SUMMARY and add a comment in the code. Given the ledger says windows-touch:no, std::process::Command is expected.
  d. Amend each commit to append the verbatim D-19 6-line trailer (D-55-E2).

D-55-E1: verify `git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines.

D-55-E3 cross-target clippy: check if pack_update_hint.rs / startup_runtime.rs / main.rs contain cfg-gated Unix code. If yes, run cross-target clippy per .planning/templates/cross-target-verify-checklist.md (or mark PARTIAL if toolchain unavailable).

D-55-E4: run `cargo test -p nono-cli` and categorise vs Phase 54 baseline SHA.
  </action>
  <verify>
    <automated>
git log --format="%B" HEAD~2..HEAD | grep -c "^Upstream-commit:"
    </automated>
    Must equal 2. Also verify:
      git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      grep -c "pack.update.hint.helper" crates/nono-cli/src/main.rs (must be >= 1)
      grep -c "NONO_NO_PACK_UPDATE_HINTS" crates/nono-cli/src/pack_update_hint.rs (must be >= 1)
      cargo build -p nono-cli (exit 0)
  </verify>
  <acceptance_criteria>
    - Exactly 2 cherry-pick commits (74fbbf12 → b1a650a3) with D-19 trailers
    - `git log --format="%B" HEAD~2..HEAD | grep -c "^Upstream-commit:"` equals 2
    - `git diff --name-only HEAD~2 ..HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines (D-55-E1 PASS)
    - pack_update_hint.rs uses `std::process::Command` for detached spawn (not fork() or libc::fork) — confirmed via grep, documented in SUMMARY
    - NONO_NO_PACK_UPDATE_HINTS env var is present in cli.rs or pack_update_hint.rs
    - pack-update-hint-helper subcommand dispatch is present in main.rs
    - Atomic write pattern (write-to-temp + rename) present in pack_update_hint.rs (confirmed via grep)
    - `cargo build -p nono-cli` exits 0
    - Cross-target clippy: PASS or PARTIAL with cross-target-verify-checklist.md documentation
    - Baseline-aware CI gate: lane-transition categories recorded (D-55-E4)
    - Feature branch NOT merged to main (D-55-03)
  </acceptance_criteria>
  <done>C9 cluster cherry-picked with D-19 trailers; pack-hint detached-process and atomic writes are in the fork.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| pack_update_hint detached process → network | refresh spawn makes an outbound check; detaching it changes the process supervision model |
| state file writes → filesystem | atomic write via temp+rename prevents partial-write state corruption |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-55-03-01 | Elevation of Privilege | Detached process spawn in pack_update_hint.rs | mitigate | Verify spawn uses std::process::Command (cross-platform, inherits no special tokens); the detached child must not inherit the sandbox token or WFP filter handles from the supervised parent; document in SUMMARY |
| T-55-03-02 | Denial of Service | pack-update-hint-helper recursion guard | mitigate | Upstream commit 74fbbf12 adds a guard to disable pre-execution update checks for the helper command itself (prevents recursion); verify the guard is preserved during cherry-pick conflict resolution |
| T-55-03-03 | Tampering | Atomic state file write race | accept | write-to-temp + rename is the standard atomic pattern; no stronger mitigation needed for a non-security-critical hint file |
| T-55-03-SC | Tampering | No new cargo deps in C9 | accept | C9 adds no new Cargo.toml dependencies (only code refactor + atomic write); no package legitimacy gate needed |
</threat_model>

<verification>
1. `git log --format="%B" HEAD~2..HEAD | grep -c "^Upstream-commit:"` equals 2
2. `git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines
3. `grep -c "process::Command" crates/nono-cli/src/pack_update_hint.rs` >= 1 (cross-platform spawn confirmed)
4. `cargo build -p nono-cli` exits 0
5. Feature branch NOT merged to main
</verification>

<success_criteria>
- 2 C9 cherry-pick commits on the held feature branch with D-19 trailers
- Detached-process refresh + atomic state writes are live; pack_update_hint.rs uses std::process::Command
- D-55-E1, D-55-E3, D-55-E4 gates satisfied or PARTIAL-documented
- Startup_runtime.rs and cli.rs are clean (no conflict regression with C7 cherry-picks)
</success_criteria>

<output>
Create `.planning/phases/55-upst7-cherry-pick-wave/55-03-SUMMARY.md` when done.
Include: C9 cherry-pick log (2 commits, SHAs, trailer verification); platform spawn verification (std::process::Command grep result); conflict-file inventory; baseline-aware CI gate result; cross-target clippy status; D-55-E1 PASS; held-branch status.
</output>
