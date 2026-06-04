---
phase: 55-upst7-cherry-pick-wave
plan: 05
type: execute
wave: 4
depends_on: [55-03, 55-04]
files_modified:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/learn.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/pty_proxy.rs
  - crates/nono-cli/src/session_commands.rs
  - crates/nono-cli/src/startup_runtime.rs
  - crates/nono-cli/src/timeouts.rs
  - docs/cli/usage/flags.mdx
autonomous: true
requirements: [REQ-UPST7-02]
must_haves:
  truths:
    - "A new timeouts.rs module centralizes all timeout constants (NONO_DETACH_STARTUP_TIMEOUT, NONO_PTY_DRAIN_TIMEOUT, NONO_PTY_ATTACH_TIMEOUT)"
    - "exec_strategy.rs, pty_proxy.rs, session_commands.rs, startup_runtime.rs, learn.rs reference timeouts.rs constants instead of inline Duration literals"
    - "Overflow checks in startup_runtime.rs and timeouts.rs are tightened (69af73d)"
    - "Formatting is applied (1442818)"
    - "Each of the 3 commits carries a verbatim 6-line D-19 trailer + DCO Signed-off-by"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
    - "exec_strategy.rs Windows-cfg-arms (from prior C7+C10 cherry-picks) are preserved"
    - "pty_proxy.rs and session_commands.rs are Unix-only or cross-platform — confirmed via cfg-gate check"
  artifacts:
    - path: "crates/nono-cli/src/timeouts.rs"
      provides: "new module with named timeout constants + env-var overrides"
    - path: "crates/nono-cli/src/exec_strategy.rs"
      provides: "inline Duration literals replaced with timeouts.rs constants (on top of C10 changes)"
    - path: "crates/nono-cli/src/pty_proxy.rs"
      provides: "inline Duration literals replaced with timeouts.rs constants"
    - path: "crates/nono-cli/src/session_commands.rs"
      provides: "inline Duration literals replaced with timeouts.rs constants"
  key_links:
    - from: "exec_strategy.rs"
      to: "timeouts.rs"
      via: "use crate::timeouts::{DETACH_STARTUP_TIMEOUT, PTY_DRAIN_TIMEOUT}"
      pattern: "timeouts::"
---

<objective>
Cherry-pick cluster C11 — timeout constants: centralized timeouts.rs module + configurable user-facing timeouts + overflow-check tightening + formatting (3 upstream commits from v0.57.0..v0.59.0).

C11 depends on both C9 (55-03) and C10 (55-04), which both run in Wave 3:
- C9 already modified startup_runtime.rs and cli.rs → C11 lands on top of C9's changes
- C10 already modified exec_strategy.rs → C11 lands on top of C10's canonical-denial additions

C11 is parallel with C12 (plan 55-06) in Wave 4 — no file overlap (C12 touches only policy.rs; C11 does not).

C11 touches exec_strategy.rs (cfg-gated Unix code) and pty_proxy.rs / session_commands.rs (may be cfg-gated Unix). Cross-target clippy REQUIRED per D-55-E3.

Purpose: Absorb centralized timeout constants into the fork.
Output: 3 new commits on the held feature branch with D-19 trailers.
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
<!-- C11 commits (chronological — oldest-first) -->
C11 commit order:
  1. 194788ee — feat(cli): centralize timeout constants and make user-facing timeouts configurable  (v0.59.0)
     Files: cli.rs, exec_strategy.rs, learn.rs, main.rs, pty_proxy.rs, session_commands.rs,
            startup_runtime.rs, timeouts.rs (NEW FILE), docs/cli/usage/flags.mdx
  2. 69af73d5 — fix: tighten up overflow checks  (v0.59.0)
     Files: startup_runtime.rs, timeouts.rs
  3. 14428182 — fix: formatting  (v0.59.0)
     Files: timeouts.rs

All: windows-touch no. Upstream-tag: v0.59.0 for all. Cross-cluster re-export check: clean (per ledger).

Key context for conflict resolution:
  - C9 (already applied in Wave 3) touched: cli.rs, startup_runtime.rs, main.rs
  - C10 (already applied in Wave 3) touched: exec_strategy.rs
  - C11 touches ALL of these — conflicts are expected. Resolve by applying C11's timeout-constant
    refactor ON TOP of C9's pack-hint additions and C10's canonical-denial pre-computation.
  - CRITICAL: exec_strategy.rs also has Windows-specific cfg-gated arms from prior phases.
    Apply timeout constant replacements ONLY in the cross-platform sections of exec_strategy.rs.

cfg-gated Unix code risk (HIGH for this cluster):
  - exec_strategy.rs: KNOWN cfg-gated file (see upstream-sync-quick.md conflict-file inventory)
  - pty_proxy.rs: PTY is Unix-only (no Windows PTY; the fork uses a different path for Windows PTY)
  - session_commands.rs: likely Unix-only session management
  Cross-target clippy MANDATORY per D-55-E3 for all three.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: C11 cherry-picks (3 commits, chronological) + cross-target clippy</name>
  <files>crates/nono-cli/src/cli.rs, crates/nono-cli/src/exec_strategy.rs, crates/nono-cli/src/learn.rs, crates/nono-cli/src/main.rs, crates/nono-cli/src/pty_proxy.rs, crates/nono-cli/src/session_commands.rs, crates/nono-cli/src/startup_runtime.rs, crates/nono-cli/src/timeouts.rs, docs/cli/usage/flags.mdx</files>
  <read_first>
    - crates/nono-cli/src/exec_strategy.rs (full post-C10-cherry-pick state — read the current file)
    - crates/nono-cli/src/startup_runtime.rs (full post-C9-cherry-pick state)
    - crates/nono-cli/src/cli.rs (full post-C9-cherry-pick state)
    - crates/nono-cli/src/pty_proxy.rs (full current fork state)
    - crates/nono-cli/src/session_commands.rs (full current fork state)
    - .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md § Cluster C11
    - .planning/templates/upstream-sync-quick.md (fork-divergence catalog — exec_strategy.rs and pty_proxy.rs entries)
    - .planning/templates/cross-target-verify-checklist.md
    - git show 194788ee (full diff — understand all timeout constants and new timeouts.rs structure)
    - git show 69af73d5 (overflow check diff)
  </read_first>
  <action>
Cherry-pick the 3 C11 commits in chronological order:
  1. 194788ee — feat(cli): centralize timeout constants and make user-facing timeouts configurable
  2. 69af73d5 — fix: tighten up overflow checks
  3. 14428182 — fix: formatting

For each cherry-pick:
  a. `git cherry-pick <upstream-sha>` from the upstream remote.
  b. Resolve conflicts using these rules:
     - cli.rs: C9 already added NONO_NO_PACK_UPDATE_HINTS + --pack-update-hint-helper. C11 adds --detach-timeout flag + NONO_DETACH_STARTUP_TIMEOUT. Apply both additions side by side. Do NOT remove C9's additions.
     - startup_runtime.rs: C9 already added the pack-hint-helper guard. C11 replaces inline Duration literals with timeouts.rs constants. Apply the constant replacements around C9's guard code. The overflow checks (69af73d) also land here.
     - main.rs: C9 added pack-update-hint-helper dispatch. C11 adds the --detach-timeout clap wiring. Apply both.
     - exec_strategy.rs: C10 already added canonical-denial pre-computation. C11 replaces inline Duration literals with timeouts.rs constants. Apply ONLY the cross-platform constant-replacement lines. PRESERVE all `#[cfg(target_os = "windows")]` arms and the Windows-specific exec strategy sections verbatim.
     - pty_proxy.rs: likely no conflict (fork's pty_proxy.rs may have Windows-specific pty path; apply upstream's timeout constant replacements only in the cross-platform sections; do NOT touch cfg(target_os="windows") arms).
     - session_commands.rs: apply timeout constant replacements verbatim if no fork-specific divergence; check for cfg-gated arms.
     - timeouts.rs: new file — apply verbatim from upstream.
     - docs/: apply verbatim.
  c. Amend each commit to append the verbatim D-19 6-line trailer block (D-55-E2). Upstream-tag v0.59.0 for all 3.

D-55-E1: verify `git diff --name-only HEAD~3 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines.

D-55-E3 cross-target clippy (MANDATORY for exec_strategy.rs + pty_proxy.rs + session_commands.rs):
  - Run `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
  - Run `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`
  - If cross-toolchain unavailable: mark PARTIAL per .planning/templates/cross-target-verify-checklist.md, document the "Cross-target clippy gate SKIPPED" prose verbatim from the checklist § PARTIAL Disposition. Set SUMMARY as human_needed for live CI confirmation.

D-55-E4: run `cargo test --workspace` and categorise vs Phase 54 baseline SHA.
  </action>
  <verify>
    <automated>
git log --format="%B" HEAD~3..HEAD | grep -c "^Upstream-commit:"
    </automated>
    Must equal 3. Also verify:
      git diff --name-only HEAD~3 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      test -f crates/nono-cli/src/timeouts.rs (must exist)
      grep -c "timeouts::" crates/nono-cli/src/exec_strategy.rs (must be >= 1 — constants in use)
      grep -c "NONO_DETACH_STARTUP_TIMEOUT\|NONO_PTY_DRAIN_TIMEOUT\|NONO_PTY_ATTACH_TIMEOUT" crates/nono-cli/src/timeouts.rs (must be >= 3)
      cargo build --workspace (exit 0)
  </verify>
  <acceptance_criteria>
    - Exactly 3 cherry-pick commits (194788ee → 69af73d5 → 14428182) with D-19 trailers
    - `git log --format="%B" HEAD~3..HEAD | grep -c "^Upstream-commit:"` equals 3
    - `git diff --name-only HEAD~3 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines (D-55-E1 PASS)
    - `crates/nono-cli/src/timeouts.rs` exists and exports NONO_DETACH_STARTUP_TIMEOUT, NONO_PTY_DRAIN_TIMEOUT, NONO_PTY_ATTACH_TIMEOUT constants
    - exec_strategy.rs uses `timeouts::` constants (not inline Duration literals for the covered timeouts)
    - exec_strategy.rs Windows-cfg-arms (from prior cherry-picks) are preserved intact
    - startup_runtime.rs C9 pack-hint guard is preserved alongside C11 overflow-check tightening
    - cli.rs has both C9's NONO_NO_PACK_UPDATE_HINTS and C11's NONO_DETACH_STARTUP_TIMEOUT / --detach-timeout
    - `cargo build --workspace` exits 0
    - Cross-target clippy: PASS or PARTIAL with checklist § PARTIAL Disposition prose (exec_strategy.rs + pty_proxy.rs in-scope → MANDATORY)
    - `cargo test --workspace` exits 0 or pre-existing failures documented as carry-forward
    - Baseline-aware CI gate: lane-transition categories recorded (D-55-E4)
    - Feature branch NOT merged to main (D-55-03)
  </acceptance_criteria>
  <done>C11 cluster cherry-picked; centralized timeouts.rs module is live; Windows-cfg-arms preserved; cross-target clippy verified or PARTIAL-documented.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| cfg-gated Unix code (exec_strategy.rs, pty_proxy.rs, session_commands.rs) → cross-target clippy | timeout constant refactor in cfg-gated files must not break Linux/macOS clippy |
| env-var timeout overrides → process environment | NONO_DETACH_STARTUP_TIMEOUT etc. read from env; validate input (saturating cast for large values) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-55-05-01 | Tampering | Env-var timeout overrides (NONO_DETACH_STARTUP_TIMEOUT etc.) | mitigate | Upstream commit 69af73d5 tightens overflow checks (saturating_* or checked_* arithmetic) — verify these checks are present after cherry-pick; no attacker-controllable timeout leading to u32/u64 overflow |
| T-55-05-02 | Tampering | exec_strategy.rs Windows-cfg-arm preservation | mitigate | Conflict resolution rules require preserving all cfg(target_os = "windows") arms; D-55-E1 gate verifies no windows-specific files touched; reviewer checks exec_strategy.rs diff for Windows section drift |
| T-55-05-03 | Denial of Service | configurable timeouts enabling DoS (e.g., extremely large timeout) | accept | Timeouts are operator-configured (env var on the host running nono, not from sandboxed child); the sandboxed child cannot modify the parent's env vars; low risk |
| T-55-05-SC | Tampering | No new cargo deps in C11 | accept | C11 adds no new Cargo.toml dependencies (only code refactor + new module); no package legitimacy gate needed |
</threat_model>

<verification>
1. `git log --format="%B" HEAD~3..HEAD | grep -c "^Upstream-commit:"` equals 3
2. `git diff --name-only HEAD~3 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines
3. `test -f crates/nono-cli/src/timeouts.rs` exits 0
4. `cargo build --workspace` exits 0
5. Cross-target clippy: PASS or PARTIAL documented (exec_strategy.rs + pty_proxy.rs → MANDATORY)
6. Feature branch NOT merged to main
</verification>

<success_criteria>
- 3 C11 cherry-pick commits on the held feature branch with D-19 trailers
- timeouts.rs module live; exec_strategy.rs / pty_proxy.rs / session_commands.rs use constants
- Overflow checks tightened (69af73d5 applied)
- exec_strategy.rs Windows-cfg-arms preserved intact
- Cross-target clippy PASS or PARTIAL-documented
- D-55-E1, D-55-E4 gates satisfied
</success_criteria>

<output>
Create `.planning/phases/55-upst7-cherry-pick-wave/55-05-SUMMARY.md` when done.
Include: C11 cherry-pick log (3 commits, SHAs, trailer verification); conflict-file inventory (cli.rs/startup_runtime.rs/exec_strategy.rs conflict resolution detail); cross-target clippy status (PASS or PARTIAL with exact checklist prose); D-55-E1 PASS; baseline-aware CI gate result; held-branch status.
</output>
