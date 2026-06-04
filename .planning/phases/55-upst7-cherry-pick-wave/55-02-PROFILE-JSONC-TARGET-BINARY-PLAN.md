---
phase: 55-upst7-cherry-pick-wave
plan: 02
type: execute
wave: 2
depends_on: [55-01]
files_modified:
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile/builtin.rs
  - crates/nono-cli/src/migration.rs
  - crates/nono-cli/src/profile_cmd.rs
  - crates/nono-cli/src/profile_save_runtime.rs
  - crates/nono-cli/data/policy.json
  - Cargo.lock
  - crates/nono-cli/Cargo.toml
autonomous: true
requirements: [REQ-UPST7-02]
must_haves:
  truths:
    - "Profile files with JSONC syntax (// comments, /* */ comments, trailing commas) are parsed correctly"
    - "A profile can declare a target binary via the `binary` field; the field is honoured only for user profiles"
    - "The opencode profile is extracted from built-in policy.json and routes to an OfficialPack registry entry"
    - "Chained if-let refactoring in resolved_workdir and prepare_sandbox compiles cleanly"
    - "Review-fix corrections (command_runtime.rs + profile/mod.rs) are applied"
    - "Schema-collision check confirms no conflict between C7 profile schema changes and the fork's nono-profile.schema.json / policy.json canonical sections (SC3)"
    - "Each of the 5 commits carries a verbatim 6-line D-19 trailer + DCO Signed-off-by"
    - "No *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files are touched"
    - "policy.rs is amended correctly (1-line addition for target_binary from 9398a13)"
  artifacts:
    - path: "crates/nono-cli/src/profile/mod.rs"
      provides: "JSONC parsing via jsonc-parser + resolve_user_profile_path + opencode extraction + review fixes"
    - path: "crates/nono-cli/src/command_runtime.rs"
      provides: "target_binary field handling + profile binary precedence over CLI command"
    - path: "crates/nono-cli/data/policy.json"
      provides: "opencode profile removed from built-ins (moved to OfficialPack registry entry)"
    - path: "55-02-SC3-SCHEMA-COLLISION-CHECK.md"
      provides: "SC3 schema-collision check artifact documenting diff-inspect result vs nono-profile.schema.json canonical sections"
  key_links:
    - from: "profile/mod.rs"
      to: "nono-profile.schema.json"
      via: "SC3 schema-collision check"
      pattern: "target_binary|jsonc"
---

<objective>
Cherry-pick cluster C7 — profile system: JSONC parsing + target_binary field + opencode pack relocation + chained-if-let refactor + review fixes (5 upstream commits from v0.57.0..v0.59.0).

This is the largest cluster in Phase 55. It touches the profile module, policy.json, command_runtime, cli, migration, profile_cmd, and profile_save_runtime. Must run in Wave 2 (after 55-01) because:
- It touches `policy.rs` (which C12 in Wave 4 also touches — C7 must precede C12)
- It touches Cargo.lock + nono-cli Cargo.toml (which C13 in Wave 5 also touches — C7 must precede C13)
- It touches `cli.rs`, `startup_runtime.rs` (which C9 + C11 also touch — C7 must precede those waves)

This plan also produces the mandatory SC3 schema-collision check artifact (`55-02-SC3-SCHEMA-COLLISION-CHECK.md`) before any commits land, ensuring the fork's canonical profile sections are preserved.

Purpose: Absorb the upstream JSONC/target_binary/opencode C7 cluster straight ports.
Output: 5 new commits on the held feature branch with D-19 trailers + the SC3 artifact.
</objective>

<execution_context>
@C:\Users\OMack\.claude\get-shit-done\workflows\execute-plan.md
@C:\Users\OMack\.claude\get-shit-done\templates\summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md
@.planning/templates/upstream-sync-quick.md
@.planning/phases/55-upst7-cherry-pick-wave/55-CONTEXT.md

<interfaces>
<!-- C7 commits (chronological — oldest-first, from git log v0.57.0..v0.59.0) -->
C7 commit order:
  1. 53a0c521 — feat(profile): add JSONC support for profile files                     (v0.58.0)
     Files: Cargo.lock, crates/nono-cli/Cargo.toml, profile/mod.rs (adds jsonc-parser dep)
  2. 9398a139 — feat(profile): allow profiles to specify a target binary               (v0.58.0)
     Files: cli.rs, command_runtime.rs, policy.rs (+1 line), profile/mod.rs
  3. e15aa53c — fix: review fixes                                                      (v0.58.0)
     Files: command_runtime.rs, profile/mod.rs
  4. cfa24f3d — refactor: use chained if let for conditional statements                (v0.58.0)
     Files: profile/mod.rs (+ test_list_profiles isolation with ENV_LOCK + TempDir XDG_CONFIG_HOME)
  5. 2bd9b4d5 — refactor(profile): extract opencode profile from built-ins            (v0.59.0)
     Files: data/policy.json, migration.rs, profile/builtin.rs, profile/mod.rs, profile_cmd.rs,
            profile_save_runtime.rs

Upstream-tag: v0.58.0 for commits 1-4; v0.59.0 for commit 5.

SC3 canonical-section sources:
  - crates/nono-cli/data/nono-profile.schema.json (the fork's profile JSON schema)
  - crates/nono-cli/data/policy.json (the fork's canonical group + profile definitions)
  - .planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md
  - .planning/phases/36-upst3-deep-closure/36-01c-OVERRIDE-DENY-RENAME-SUMMARY.md

Phase-49 override_deny → bypass_protection rename (from 36-01c): honor if any profile field is renamed.

D-55-E2 trailer field order: Upstream-commit → Upstream-tag → Upstream-author → Co-Authored-By → Signed-off-by (full) → Signed-off-by (handle)
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: SC3 schema-collision check (produce 55-02-SC3-SCHEMA-COLLISION-CHECK.md before cherry-picks)</name>
  <files>.planning/phases/55-upst7-cherry-pick-wave/55-02-SC3-SCHEMA-COLLISION-CHECK.md</files>
  <read_first>
    - crates/nono-cli/data/nono-profile.schema.json (full file)
    - crates/nono-cli/data/policy.json (full file — note the opencode entry that C7 removes)
    - .planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md (fork's From&lt;ProfileDeserialize&gt; for Profile match — full read)
    - .planning/phases/36-upst3-deep-closure/36-01c-OVERRIDE-DENY-RENAME-SUMMARY.md (override_deny → bypass_protection rename)
    - git show 9398a13 (upstream target_binary commit — inspect what schema fields it adds to nono-profile.schema.json)
    - git show 53a0c52 (upstream JSONC commit — inspect what profile/mod.rs + Cargo.toml changes it makes)
    - git show 2bd9b4d5 (upstream opencode commit — inspect what it removes from data/policy.json)
    - .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md § Cluster C7 + § Cross-cluster re-export deps
  </read_first>
  <action>
Before any cherry-pick, produce the SC3 schema-collision check artifact. The artifact must answer:

1. Does the `target_binary` field (9398a13) conflict with any existing field in the fork's `nono-profile.schema.json`? Diff-inspect the upstream commit's schema additions against the fork's current schema file. Confirm: (a) the field does not already exist under a different name, (b) it does not alias a fork-renamed field (check the override_deny → bypass_protection rename from 36-01c), (c) it does not collide with any Windows-specific fork additions.

2. Does the JSONC parsing change (53a0c52) conflict with the fork's `From<ProfileDeserialize> for Profile` canonical match (36-01b)? The JSONC parser replaces `serde_json::from_str` with `jsonc-parser` — does the fork have any custom deserialization code in profile/mod.rs that would conflict?

3. Does the opencode extraction (2bd9b4d5) leave any dangling reference to the `opencode` profile in the fork's policy.json canonical sections? The fork's `data/policy.json` currently embeds an opencode profile definition — removing it (as the upstream commit does) means nono-profile.schema.json must NOT still require it.

4. Does the fork's `phase 36 canonical-sections` enumeration still hold after the C7 changes? No canonical section should be renamed or removed without a corresponding update to the 36-01b/36-01c artifacts.

Record findings in `55-02-SC3-SCHEMA-COLLISION-CHECK.md` with a verdict per item: CLEAR (no collision), COLLISION (action required before cherry-pick), or PARTIAL (note the fork-specific handling needed during cherry-pick conflict resolution).

If any COLLISION is found, document the resolution action and block the cherry-pick task until resolved. If all CLEAR, proceed.
  </action>
  <verify>
    <automated>
test -f ".planning/phases/55-upst7-cherry-pick-wave/55-02-SC3-SCHEMA-COLLISION-CHECK.md" && grep -c "CLEAR\|COLLISION\|PARTIAL" ".planning/phases/55-upst7-cherry-pick-wave/55-02-SC3-SCHEMA-COLLISION-CHECK.md"
    </automated>
    Must return >= 3 (at least 3 items checked).
  </verify>
  <acceptance_criteria>
    - File 55-02-SC3-SCHEMA-COLLISION-CHECK.md exists in the phase directory
    - Contains at least 3 checked items (target_binary schema, JSONC deserialization, opencode removal)
    - Each item has a verdict: CLEAR, COLLISION, or PARTIAL
    - If any COLLISION: action plan documented and cherry-pick is blocked until resolved
    - References the specific 36-01b/36-01c canonical-section files as authority
    - Checks the override_deny → bypass_protection rename surface (from 36-01c)
    - Commit carrying this artifact has: Signed-off-by: Oscar Mack Jr &lt;oscar.mack.jr@gmail.com&gt;
    - git diff for this commit touches ONLY the planning phases/ directory (no crates/ edits)
  </acceptance_criteria>
  <done>SC3 schema-collision check is documented and committed before any C7 code cherry-picks land.</done>
</task>

<task type="auto">
  <name>Task 2: C7 cherry-picks (5 commits, chronological)</name>
  <files>crates/nono-cli/src/cli.rs, crates/nono-cli/src/command_runtime.rs, crates/nono-cli/src/policy.rs, crates/nono-cli/src/profile/mod.rs, crates/nono-cli/src/profile/builtin.rs, crates/nono-cli/src/migration.rs, crates/nono-cli/src/profile_cmd.rs, crates/nono-cli/src/profile_save_runtime.rs, crates/nono-cli/data/policy.json, Cargo.lock, crates/nono-cli/Cargo.toml</files>
  <read_first>
    - .planning/phases/55-upst7-cherry-pick-wave/55-02-SC3-SCHEMA-COLLISION-CHECK.md (MUST read before cherry-picking — follow any COLLISION resolutions)
    - crates/nono-cli/src/profile/mod.rs (full current fork state)
    - crates/nono-cli/src/command_runtime.rs (full current fork state)
    - crates/nono-cli/data/policy.json (full current fork state)
    - .planning/templates/upstream-sync-quick.md (D-19 trailer + fork-divergence catalog — especially profile/mod.rs conflict-file entry)
    - .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md § Cluster C7
    - .planning/phases/36-upst3-deep-closure/36-01b-CANONICAL-PROFILE-SECTIONS-SUMMARY.md (fork's deserialization canonical shape)
  </read_first>
  <action>
Cherry-pick the 5 C7 commits in chronological order. For each commit:
  a. `git cherry-pick <upstream-sha>` from the upstream remote.
  b. Resolve conflicts using these rules:
     - profile/mod.rs: The fork may have Windows-specific deserialization arms or cfg-gated sections — keep them. Apply the upstream JSONC / target_binary / opencode changes around the fork's cfg-gated regions (per the upstream-sync-quick.md conflict-file entry for profile/mod.rs). Do NOT remove any `#[cfg(target_os = "windows")]` arms.
     - policy.rs: commit 9398a139 adds exactly 1 line (the `target_binary` field registration). Apply it verbatim; no other changes to policy.rs.
     - data/policy.json: commit 2bd9b4d5 removes the embedded opencode profile definition. Apply the removal. Verify no other canonical group or profile is accidentally touched (SC3 verdict: CLEAR expected for this).
     - command_runtime.rs: the target_binary precedence logic (profile binary wins over CLI trailing command, with a warning). Apply the upstream logic; the fork's Windows-specific launch path (exec_strategy_windows/) is NOT in command_runtime.rs — no conflict expected.
     - Cargo.lock: 53a0c521 adds jsonc-parser dependency. Apply verbatim.
  c. After each cherry-pick (or after resolving conflicts), amend the commit to append the verbatim D-19 6-line trailer block (D-55-E2). Use the 8-char SHA abbreviation in Upstream-commit field. Both Signed-off-by lines required.
  d. For commit cfa24f3d (chained if-let + test isolation): the commit also adds ENV_LOCK + TempDir XDG_CONFIG_HOME isolation to test_list_profiles — apply verbatim; this aligns with the fork's env-var save/restore discipline (CLAUDE.md).

D-55-E1: after all 5 cherry-picks, verify no *_windows.rs / exec_strategy_windows/ / nono-shell-broker/ files were touched.

D-55-E3 cross-target clippy: profile/mod.rs and command_runtime.rs may contain cfg-gated Unix code (check for `#[cfg(target_os = "linux")]` or `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks). If found:
  - Run `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `--target x86_64-apple-darwin`.
  - If cross-toolchain unavailable: mark PARTIAL, document per .planning/templates/cross-target-verify-checklist.md, defer to live CI.

D-55-E4: run `cargo test -p nono-cli` and categorise transitions vs Phase 54 baseline SHA.
  </action>
  <verify>
    <automated>
git log --format="%B" HEAD~5..HEAD | grep -c "^Upstream-commit:"
    </automated>
    Must equal 5 (one per C7 commit). Also verify:
      git diff --name-only HEAD~5 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker" (must return 0 lines)
      cargo build -p nono-cli (exit 0)
      cargo test -p nono-cli (exit 0 or pre-existing failures documented)
  </verify>
  <acceptance_criteria>
    - Exactly 5 cherry-pick commits, in chronological order: 53a0c521 → 9398a13 → e15aa53c → cfa24f3d → 2bd9b4d5
    - Each commit carries the verbatim D-19 6-line trailer; `git log --format="%B" HEAD~5..HEAD | grep -c "^Upstream-commit:"` equals 5
    - `git diff --name-only HEAD~5 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines (D-55-E1 PASS)
    - `cargo build -p nono-cli` exits 0
    - policy.rs diff shows exactly 1 line added (target_binary field registration from 9398a13); no other policy.rs changes
    - data/policy.json diff shows only the opencode removal (from 2bd9b4d5); no other canonical groups or profiles removed
    - JSONC: .jsonc profile extension is now supported (parsable by the codebase)
    - target_binary: user profiles can declare a `binary` field (verified via grep in profile/mod.rs)
    - Cross-target clippy: PASS (clean) or PARTIAL with cross-target-verify-checklist.md documentation
    - Baseline-aware CI gate: lane-transition categories recorded (D-55-E4)
    - SC3 check artifact exists and all items are CLEAR (or COLLISION items were resolved before cherry-picking)
    - Feature branch NOT merged to main (D-55-03)
  </acceptance_criteria>
  <done>C7 cluster is fully cherry-picked with D-19 trailers; SC3 schema-collision check documented and clean; profile system changes compile and test clean.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| user profile file → JSONC parser | profile/mod.rs now parses untrusted user-authored .jsonc files |
| profile binary field → exec path | target_binary field declares a binary to execute — untrusted user input |
| upstream cherry-pick → fork policy.json | opencode removal alters the embedded policy; canonical sections must be preserved |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-55-02-01 | Tampering | JSONC parser (jsonc-parser crate) input handling | mitigate | SC3 check verifies the jsonc-parser dependency is from the upstream-audited range; profile parsing remains fail-closed (parse error → profile load failure per existing error handling); validate jsonc-parser crate legitimacy in SC3 artifact (check npm/crates.io provenance of jsonc-parser crate version) |
| T-55-02-02 | Elevation of Privilege | target_binary field — user-authored profile declares binary to exec | mitigate | Upstream commit 9398a13 explicitly restricts target_binary to user profiles only (not pack or built-in profiles; a warning is emitted if a built-in tries to use it); verify this restriction is preserved during cherry-pick conflict resolution |
| T-55-02-03 | Tampering | policy.json opencode removal — canonical sections must not be damaged | mitigate | SC3 check verifies the fork's canonical groups/profiles are intact after 2bd9b4d5 removal; diff policy.json to confirm only the opencode definition is removed |
| T-55-02-04 | Spoofing | D-19 trailer integrity on 5-commit chain | mitigate | `git log --format="%B" HEAD~5..HEAD | grep -c "^Upstream-commit:"` gate verifies all 5 trailers present |
| T-55-02-SC | Tampering | jsonc-parser cargo dep (new dependency in 53a0c52) | mitigate | Verify jsonc-parser crate on crates.io before applying Cargo.toml change; check for typosquatting / suspicious version; block if [SUS] or [SLOP] |
</threat_model>

<verification>
After both tasks:

1. `git log --format="%B" HEAD~5..HEAD | grep -c "^Upstream-commit:"` equals 5
2. `git diff --name-only HEAD~5 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines
3. `git diff HEAD~5..HEAD -- crates/nono-cli/src/policy.rs | grep "^+" | wc -l` is 1 (only 1 line added to policy.rs by 9398a13)
4. `test -f ".planning/phases/55-upst7-cherry-pick-wave/55-02-SC3-SCHEMA-COLLISION-CHECK.md"` exits 0
5. `cargo build -p nono-cli` exits 0
6. `cargo test -p nono-cli` exits 0 or pre-existing failures documented
7. Feature branch NOT merged to main
</verification>

<success_criteria>
- SC3 schema-collision check artifact exists and documents CLEAR verdict for all 3 items (target_binary, JSONC, opencode removal)
- 5 C7 cherry-pick commits on the held feature branch in chronological order with D-19 trailers
- JSONC profile parsing, target_binary field, and opencode OfficialPack relocation are live in the fork
- policy.rs touched by exactly 1 line addition (no other changes)
- data/policy.json opencode removed cleanly
- D-55-E1, D-55-E3, D-55-E4 gates satisfied or PARTIAL-documented
</success_criteria>

<output>
Create `.planning/phases/55-upst7-cherry-pick-wave/55-02-SUMMARY.md` when done.
Include: SC3 check verdict (per-item table); C7 cherry-pick log (5 commits, SHAs, trailer verification); conflict-file inventory (what conflicted in profile/mod.rs / command_runtime.rs if anything); baseline-aware CI gate result; cross-target clippy status; D-55-E1 windows-invariant status (PASS); held-branch status.
</output>
