---
phase: 34-upst3-upstream-v0-41-v0-52-sync-execution
plan_number: 34-04b
plan: 04b
slug: fp-canonical-schema
cluster_id: C7-residual
parent_plan: 34-04
type: execute
wave: 0.5
depends_on: ["34-04"]
blocks: ["34-01", "34-02", "34-03", "34-05", "34-06", "34-07", "34-08", "34-09", "34-10"]
files_modified:
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/profile/builtin.rs
  - crates/nono-cli/src/profile_cmd.rs
  - crates/nono-cli/src/profile_runtime.rs
  - crates/nono-cli/src/policy.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/query_ext.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/why_runtime.rs
  - crates/nono-cli/data/policy.json
  - crates/nono-cli/Cargo.toml
  - crates/nono/Cargo.toml
  - crates/nono-proxy/Cargo.toml
  - Cargo.lock
  - crates/nono-cli/tests/manifest_roundtrip.rs
  - crates/nono-cli/tests/deny_overlap_run.rs
  - crates/nono-cli/src/registry_client.rs
  - crates/nono/src/error.rs
  - docs/cli/features/profiles-groups.mdx
  - docs/cli/usage/flags.mdx
  - tests/integration/test_bypass_protection.sh
upstream_tag_range: v0.47.0..v0.47.1
upstream_commit_count: 6
disposition: fork-preserve-manual-replay-split
autonomous: false
requirements: [C7-residual]
tags: [upst3, c7-residual, canonical-schema, fork-preserve, manual-replay, d-20, wave-0.5, split-from-34-04]

must_haves:
  truths:
    - "All 6 upstream commits (f0abd413, f3e7f885, 0cba04a5, 7329ef73, 829c341a, ab74f5cd) READ in full and EXPLICITLY dispositioned in Task 2: each gets either (A) straight cherry-pick (after f0abd413's canonical-schema base lands), (B) manual replay with `Manual-replay:` body, or (C) skip with documented rationale. Disposition decision recorded in /tmp/34-04b-disposition.txt."
    - "Upstream commit `f0abd413` (canonical JSON schema restructure: override_deny → bypass_protection, deprecated_schema module, SecurityConfig/PolicyPatchConfig canonical sections groups/commands.{allow,deny}/filesystem.{deny,bypass_protection}) replayed manually onto fork's 5584-line crates/nono-cli/src/profile/mod.rs. Replay commit body documents what was ported + what fork-only paths were preserved + why straight cherry-pick was infeasible. NO `Upstream-commit:` D-19 trailer on this commit; body explicitly cites `Manual-replay: f0abd413` per D-20 convention (mirror 34-09/34-10 commit-body precedent)."
    - "Plan 18.1-03 Windows widening fork-only path (`capabilities.aipc` / `loaded_profile`) BYTE-PRESERVED through f0abd413 replay. Verified: `grep -c 'capabilities.aipc\\|capabilities_aipc\\|loaded_profile' crates/nono-cli/src/profile/mod.rs` returns >= 17 post-replay (pre-plan baseline confirmed 17 hits at plan-write time)."
    - "Phase 26 PKGS-02 `ArtifactType::Plugin` round-trip PRESERVED through f0abd413 replay. Verified: `grep -c 'ArtifactType::Plugin\\|    Plugin,' crates/nono-cli/src/package.rs` returns >= 4 post-replay (pre-plan baseline 4 hits); `cargo test -p nono-cli package::tests::artifact_type_plugin_round_trips` exits 0 if the test exists, otherwise `cargo test -p nono-cli package` exits 0 with the Plugin variant participating in serde round-trips."
    - "Phase 22-01 `ProfileDeserialize` companion-struct pattern PRESERVED post-replay (fork retains its companion-struct deserialization path even if upstream's canonical-schema restructure removes the analog upstream-side). Verified: `grep -c 'ProfileDeserialize\\|struct ProfileDeserialize' crates/nono-cli/src/profile/mod.rs` >= 1 post-replay."
    - "Phase 22-03 PKG-04 `validate_path_within` defense-in-depth at 9 callsites in `crates/nono-cli/src/package_cmd.rs` PRESERVED. Verified: `grep -c 'validate_path_within' crates/nono-cli/src/package_cmd.rs` returns >= 9 post-plan (pre-plan baseline 9)."
    - "Phase 19 v2.1 `never_grant` / `apply_deny_overrides` (21 callsites in `crates/nono-cli/src/policy.rs`) PRESERVED through `override_deny → bypass_protection` rename. Verified: `grep -c 'never_grant\\|apply_deny_overrides' crates/nono-cli/src/policy.rs` returns >= 21 post-rename (pre-plan baseline 21)."
    - "`override_deny → bypass_protection` rename strategy DECIDED in Task 2 checkpoint (three options: A=hard rename + v2.4 milestone breaking change; B=backwards-compat: accept both `override_deny` and `bypass_protection` as aliases in deserialize; C=deprecated_schema module reads legacy `override_deny` and rewrites to canonical `bypass_protection`). Decision recorded in `/tmp/34-04b-disposition.txt`. The chosen strategy is documented in the f0abd413 replay commit body."
    - "Upstream's `deprecated_schema` module ported (or its functional equivalent provided in the fork's profile/mod.rs) so legacy v2.3 user profiles continue to parse. Verified: `grep -cE 'deprecated_schema|LegacyPolicyPatch|override_deny' crates/nono-cli/src/profile/mod.rs` >= 1 post-replay if Option B or C; if Option A, an `override_deny → bypass_protection` migration note is documented in the replay commit body + CHANGELOG."
    - "Cherry-picks for the 5 dependent commits (`f3e7f885`, `0cba04a5`, `7329ef73`, `829c341a`, `ab74f5cd`) carry the verbatim D-19 6-line trailer block (lowercase 'a' in `Upstream-author:`). For each commit where Task 2 dispositions a manual replay instead, the body carries `Manual-replay: <8-char-sha>` + the 2 DCO Signed-off-by lines."
    - "Plan-close smoke check: `git log --format='%B' main~N..main | grep -v '^#' | grep -c '^Upstream-commit: '` returns exactly the count of commits Task 2 dispositioned as straight cherry-pick (default expected = 5: f3e7f885 + 0cba04a5 + 7329ef73 + 829c341a + ab74f5cd; the f0abd413 manual-replay commit is EXEMPT per D-20 convention). N = total commit count from plan-start HEAD `f6f4fb14` to HEAD."
    - "D-34-E1 Windows-only file invariant: per-commit `git diff --stat <prev>..<this> -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l` returns 0 for EVERY commit in the 34-04b chain."
    - "D-34-D2 close gates: Gates 1 (`cargo test --workspace --all-features` Windows host - lib subset minimum), 2 (Windows clippy `-D warnings -D clippy::unwrap_used`), 5 (`cargo fmt --all -- --check`) PASS on dev host. Gates 3, 4 (Linux + macOS cross-target clippy) DOCUMENTED-SKIPPED with rationale 'deferred to CI per dev-host limitation; user accepted same posture at 34-04 close on 2026-05-11'. Gates 6 (Phase 15 5-row detached-console smoke), 7 (`wfp_port_integration --ignored`), 8 (`learn_windows_integration`) DOCUMENTED-SKIPPED per 'incomplete plan / admin / service' rationale from 34-04 SUMMARY."
    - "`cargo build --workspace` exits 0 post-Task 6 (jsonschema 0.46.4 bump). The bump must not break fork's existing schema validation surface in profile/mod.rs."
    - "Plan 34-04b commits pushed to origin/main at plan close; per-plan PR opened per D-34-D1. `git log origin/main..main --oneline | wc -l` returns 0 post-push."
  artifacts:
    - path: "crates/nono-cli/src/profile/mod.rs"
      provides: "Canonical JSON schema sections (groups, commands.{allow,deny}, filesystem.{deny,bypass_protection}) ported from upstream f0abd413; Plan 18.1-03 `loaded_profile` + `capabilities.aipc` PRESERVED; Phase 22-01 `ProfileDeserialize` companion-struct pattern PRESERVED; Phase 22-04 OAuth2 `validate_upstream_url` loopback gate PRESERVED; deprecated_schema legacy-key handling (per rename strategy chosen in Task 2)"
      grep_pattern: "capabilities.aipc|loaded_profile|ProfileDeserialize|bypass_protection|validate_upstream_url"
      grep_negative: "// removed capabilities.aipc|// loaded_profile dropped|// upstream-only ProfileDeserialize gone"
      min_call_sites: 17
    - path: "crates/nono-cli/src/policy.rs"
      provides: "`override_deny → bypass_protection` rename applied at consumers (per Task 2 strategy); Phase 19 v2.1 `never_grant` / `apply_deny_overrides` defense-in-depth UNCHANGED; Phase 22-03 PKG-04 `validate_path_within` PRESERVED; the `find_denied_user_grants` helper (added in 34-04 commit ac9f0a59) PRESERVED"
      grep_pattern: "never_grant|apply_deny_overrides|find_denied_user_grants|bypass_protection"
      grep_negative: "// removed never_grant|// dropped apply_deny_overrides"
    - path: "crates/nono-cli/src/sandbox_prepare.rs"
      provides: "Rename `override_deny → bypass_protection` propagated through SandboxArgs / PreparedSandbox callers per Task 2 strategy"
      grep_pattern: "bypass_protection|override_deny"
    - path: "crates/nono-cli/src/cli.rs"
      provides: "Clap flag rename `--override-deny → --bypass-protection` (per Task 2 strategy); fork retains alias for backwards-compat if Option B chosen"
      grep_pattern: "bypass.protection|override.deny"
    - path: "crates/nono-cli/data/policy.json"
      provides: "Canonical sections (groups, commands, filesystem) per upstream f0abd413 layout; claude-code + codex builtin entries Phase 18.1-03 wiring PRESERVED"
      grep_pattern: "groups|commands|filesystem|bypass_protection|claude-code|codex"
    - path: "crates/nono-cli/Cargo.toml"
      provides: "jsonschema 0.45.1 → 0.46.4 (upstream 7329ef73)"
      grep_pattern: "jsonschema.*=.*\"0\\.46"
    - path: "crates/nono-cli/src/registry_client.rs"
      provides: "Draft commands support (upstream 829c341a); composes with fork's existing registry-client surface"
      grep_pattern: "draft"
  key_links:
    - from: "Plan 34-04 STOP at commit 18 (upstream f0abd413) on 2026-05-11"
      to: "Plan 34-04b D-20 manual-replay continuation"
      via: "Phase 22-05a/22-05b split precedent: mid-plan STOP triggered by exceeding the D-02 fallback gate (60-file restructure with deep fork divergence); split-plan continuation honours D-34-A2 Wave 0 sequential-gate posture as Wave 0.5"
      pattern: "Manual-replay: f0abd413|Upstream-commit: (f3e7f88|0cba04a|7329ef7|829c341|ab74f5c)"
    - from: "Upstream f0abd413 (60 files, ~5K-line delta, override_deny → bypass_protection rename, deprecated_schema module, canonical sections groups/commands/filesystem)"
      to: "Fork's 5584-line crates/nono-cli/src/profile/mod.rs (Plan 18.1-03 capabilities.aipc + Phase 22-01 ProfileDeserialize + Phase 22-04 validate_upstream_url loopback gate + Phase 26 PKGS-02 ArtifactType::Plugin)"
      via: "D-20 manual replay: read upstream's diff in full; identify the structural intent (canonical-schema sections + rename + deprecated_schema); apply the intent without deleting fork-only paths"
      pattern: "capabilities.aipc|loaded_profile|ProfileDeserialize|bypass_protection"
    - from: "Phase 22-03 PKG-04 `validate_path_within` defense-in-depth (9 callsites in package_cmd.rs)"
      to: "Plan 34-04b post-rename state"
      via: "rename does not touch package_cmd.rs; baseline preserved by construction; verified at close"
      pattern: "validate_path_within"
    - from: "Phase 19 v2.1 `never_grant` / `apply_deny_overrides` (21 callsites in policy.rs)"
      to: "Plan 34-04b post-rename state"
      via: "the rename touches the `override_deny → bypass_protection` field name only; never_grant/apply_deny_overrides defense-in-depth logic is orthogonal; verified at close"
      pattern: "never_grant|apply_deny_overrides"
    - from: "Plan 34-04 close (Wave 0 partial - 17/23 commits landed, HEAD at f6f4fb14)"
      to: "Plan 34-04b close (Wave 0.5 - canonical-schema base + 5 dependent commits land)"
      via: "Wave 0.5 sequential gate; Wave 1+ plans (34-01, 34-03, 34-06) BLOCKED until 34-04b closes; 34-04b inherits 34-04's D-34-A2 Wave 0 status"
      pattern: "depends_on.*34-04"
---

<objective>
Continue the cluster-C7 sync that Plan 34-04 paused on 2026-05-11 at upstream commit `f0abd413` (canonical JSON schema restructure). 6 upstream commits remain unlanded: `f0abd413`, `f3e7f885`, `0cba04a5`, `7329ef73`, `829c341a`, `ab74f5cd`. Plan 34-04 already landed 17 cherry-picks cleanly; local HEAD is `f6f4fb14` with full D-19 + D-34-E1 + fork-defense invariants intact (per 34-04 SUMMARY § Self-Check).

**Why this plan exists as 34-04b (not a re-run of 34-04):** Plan 34-04 explicitly hit its D-02 fallback gate on commit 18 (`f0abd413`). 10 conflicted files, 1 modify/delete conflict, ~5K-line upstream diff across 60 files, semantic renames (`override_deny → bypass_protection`) rippling through fork-divergent code, and a 5584-line `profile/mod.rs` with multi-thousand-line delta vs upstream. The plan's STOP-trigger fired; cherry-pick aborted; local main rolled back to commit 17 state; SUMMARY produced. This is the Phase 22-05a/22-05b mid-plan split precedent — a continuation plan with its own scope, not a retry.

**Disposition split (Task 2 confirms; defaults below):**

| # | Upstream SHA | Tag | Files | Default disposition | Rationale |
|---|--------------|-----|-------|---------------------|-----------|
| 1 | `f0abd413` | v0.47.0 | 60 | **D-20 manual replay** | Canonical JSON schema restructure; 5K-line delta; deep fork divergence in profile/mod.rs (5584 lines); semantic rename; deprecated_schema module |
| 2 | `f3e7f885` | v0.47.0 | 2 | **cherry-pick** (after #1) | profile_cmd.rs render-serde-values; rebases on canonical-schema base |
| 3 | `0cba04a5` | v0.47.1 | 6 | **cherry-pick (partial)** | Release commit; Cargo.toml/Cargo.lock version-bumps will be dropped (mirror Plan 34-04 commits 3 + 12 partial-cherry-pick shape); CHANGELOG entry merged |
| 4 | `7329ef73` | v0.47.1 | 3 | **cherry-pick** | jsonschema 0.45.1 → 0.46.4; Cargo.lock change |
| 5 | `829c341a` | v0.47.1 | 10 | **cherry-pick** (likely) | Draft commands; 796 insertions/21 deletions; may conflict with fork's registry_client.rs + profile/mod.rs - Task 2 confirms after read |
| 6 | `ab74f5cd` | v0.47.1 | 8 | **cherry-pick** | Docs-only; deprecation wording + built-in vs pack distinction |

`autonomous: false` — Task 2 is a per-commit `checkpoint:decision` gate where the user approves the disposition table (and specifically the `override_deny → bypass_protection` rename strategy) before per-commit execute tasks begin.

**override_deny → bypass_protection rename strategy:** Task 2 must decide between:
- **Option A** — hard rename to `bypass_protection`; v2.3 user profiles using `override_deny` fail to parse; v2.4 milestone breaking change; CHANGELOG entry required.
- **Option B** — accept both `override_deny` and `bypass_protection` as deserialize aliases (via `#[serde(alias = "...")]`); both fields write to the canonical `bypass_protection` internal name; no breaking change.
- **Option C** — port upstream's `deprecated_schema` module verbatim: deserialize legacy `override_deny` into a LegacyPolicyPatch struct that gets rewritten to canonical `bypass_protection` post-parse; emit a deprecation warning to stderr (matches upstream behavior); migration path for users.

Default recommendation (planner's bias): **Option C** — closest to upstream-fidelity, preserves backwards compat, gives a deprecation runway. Task 2 confirms.

Purpose: After 34-04b closes, downstream plans (34-01 CLI consolidation, 34-03 keyring, 34-06 trust scan, et al.) can rebase against the canonical-JSON-schema state without re-discovering schema conflicts. Wave 1+ unblocks.

Output: 6-7 commits on `main` (1 manual-replay for `f0abd413` + 5 cherry-picks for f3e7f885/0cba04a5/7329ef73/829c341a/ab74f5cd + optional fork-only cleanup commits if needed) bringing fork to upstream v0.47.1 schema parity. Plan 34-04's full 23-commit goal becomes 34-04 (17) + 34-04b (6) = 23 total at HEAD; downstream waves unblocked.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-CONTEXT.md
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-SUMMARY.md
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-PLAN.md
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-09-FP-PACKS-PLAN.md
@.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-10-FP-PROXY-TLS-PLAN.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md
@.planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05b-AUD-RENAME-PLAN.md
@.planning/phases/33-windows-parity-upstream-0-52-divergence/DIVERGENCE-LEDGER.md
@.planning/templates/upstream-sync-quick.md
@crates/nono-cli/src/profile/mod.rs
@crates/nono-cli/src/policy.rs
@crates/nono-cli/src/sandbox_prepare.rs
@crates/nono-cli/data/policy.json

<interfaces>
**Pre-plan HEAD (Plan 34-04 close state):** `f6f4fb14e82ef4ec04d95851e78ead7d9bd8dc4b` (docs(34-04): SUMMARY ...)

**6 upstream commits in scope (verified via `git show -s` at plan-write time, 2026-05-11):**

| # | SHA (8) | Tag | Author | Files | Insertions/Deletions | Subject |
|---|---------|-----|--------|-------|----------------------|---------|
| 1 | `f0abd413` | v0.47.0 | Leo Lapworth <leo@cuckoo.org> | 60 | +3939 / -1007 | feat(profile): #594 phase 2 — canonical JSON schema restructure (#594) |
| 2 | `f3e7f885` | v0.47.0 | Matt Palcic <matt.palcic@naturalforms.com> | 2 | +191 / -16 | fix(profile): emit serde-rendered values in show/diff JSON output |
| 3 | `0cba04a5` | v0.47.1 | Luke Hinds <lukehinds@gmail.com> | 6 | +23 / -10 | chore: release v0.47.1 |
| 4 | `7329ef73` | v0.47.1 | dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com> | 3 | +17 / -9 | chore(deps): bump jsonschema from 0.45.1 to 0.46.4 |
| 5 | `829c341a` | v0.47.1 | Luke Hinds <lukehinds@gmail.com> | 10 | +796 / -21 | add commands to manage profile drafts and check package status |
| 6 | `ab74f5cd` | v0.47.1 | SequeI <asiek@redhat.com> | 8 | +32 / -31 | docs: fix stale references, deprecation wording, and built-in vs pack distinction |

**Conflict surface for `f0abd413` (per Plan 34-04 SUMMARY § Deviations):**

10 files conflicted on the cherry-pick that triggered the D-02 STOP:
- `crates/nono-cli/src/profile/builtin.rs`
- `crates/nono-cli/src/profile/mod.rs` (5584 lines; fork-side delta multi-thousand lines)
- `crates/nono-cli/src/profile_cmd.rs`
- `crates/nono-cli/src/profile_runtime.rs`
- `crates/nono-cli/src/sandbox_prepare.rs`
- `crates/nono-cli/src/query_ext.rs`
- `crates/nono-cli/tests/deny_overlap_run.rs`
- `crates/nono-cli/tests/manifest_roundtrip.rs`
- `crates/nono/src/diagnostic.rs`
- `docs/cli/features/profiles-groups.mdx`

Plus 1 modify/delete: `crates/nono-cli/src/profile_save_runtime.rs` (fork-deleted, upstream-modified).

Plus the renamed file: `tests/integration/test_override_deny.sh → test_bypass_protection.sh` (161 lines moved).

**Fork-divergence surface that MUST survive the f0abd413 manual replay (verified baselines captured at plan-write time, 2026-05-11):**

| Surface | File | Baseline (grep count) | Notes |
|---------|------|------------------------|-------|
| Plan 18.1-03 `capabilities.aipc` widening | crates/nono-cli/src/profile/mod.rs | **17** | Phase 18.1-03 G-06 profile widening end-to-end → AipcResolvedAllowlist via Windows SupervisorConfig field |
| `loaded_profile` struct | crates/nono-cli/src/profile/mod.rs | (part of 17 above) | Fork-only struct; carries `capabilities.aipc` |
| Phase 22-01 `ProfileDeserialize` companion-struct pattern | crates/nono-cli/src/profile/mod.rs | (>= 1 occurrence) | Fork-only deserialize pattern (PROF-01..03 retained) |
| Phase 22-04 `validate_upstream_url` loopback gate | crates/nono-cli/src/profile/mod.rs | (>= 1 occurrence) | OAuth2 composition |
| Phase 26 PKGS-02 `ArtifactType::Plugin` | crates/nono-cli/src/package.rs | **4** | 7th ArtifactType variant; serde `rename_all = "snake_case"` round-trip |
| Phase 19 v2.1 `never_grant` / `apply_deny_overrides` | crates/nono-cli/src/policy.rs | **21** | Defense-in-depth gate |
| Phase 22-03 PKG-04 `validate_path_within` | crates/nono-cli/src/package_cmd.rs | **9** | 9 callsites; defense-in-depth |
| 34-04 commit ac9f0a59 helper `find_denied_user_grants` | crates/nono-cli/src/policy.rs | (>= 1 occurrence) | Added during 34-04 run |

**Plan 34-04 line file totals (current state at f6f4fb14):**
- `crates/nono-cli/src/profile/mod.rs` = **5584 lines**
- `crates/nono-cli/src/policy.rs` = **3131 lines**
- `crates/nono-cli/src/package_cmd.rs` = **1379 lines**

**D-19 cherry-pick trailer block (verbatim — applies to the 5 cherry-pick commits f3e7f885/0cba04a5/7329ef73/829c341a/ab74f5cd):**

```
Upstream-commit: {8-char-sha}
Upstream-tag: {v0.47.0 or v0.47.1}
Upstream-author: {upstream_author_name} <{upstream_author_email}>
Co-Authored-By: {upstream_author_name} <{upstream_author_email}>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
```

Field rules per `.planning/templates/upstream-sync-quick.md` § D-19 cherry-pick trailer block:
- Lowercase 'a' in `Upstream-author:` (NOT `Upstream-Author:`).
- 8-character SHA abbrev in `Upstream-commit:`.
- `Upstream-author:` + `Co-Authored-By:` carry the SAME `name <email>`.
- Two `Signed-off-by:` lines (DCO full name + GitHub handle).
- Trailer block separated from body by EXACTLY ONE blank line.

**Manual-replay trailer block (verbatim — applies ONLY to the f0abd413 replay commit):**

```
{free-form prose body documenting (1) what upstream's f0abd413 did,
 (2) which fork-only paths a straight cherry-pick would have deleted,
 (3) the override_deny → bypass_protection rename strategy chosen (Option A/B/C),
 (4) what intent the fork now carries from the replay,
 (5) what was NOT replayed (and why),
 (6) reference to Plan 34-04 SUMMARY § Deviations for the D-02 STOP context}

Manual-replay: f0abd413
Upstream-tag: v0.47.0
Upstream-author: Leo Lapworth <leo@cuckoo.org>
Co-Authored-By: Leo Lapworth <leo@cuckoo.org>
Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
```

The `Manual-replay:` field SUBSTITUTES for `Upstream-commit:` per D-20 (mirror Plan 34-09 + Plan 34-10's read-and-document commit shape; mirror Phase 26-01 PKGS-02 commit-body precedent). Semantic: "this commit replays the INTENT of upstream's commit but the form differs because of fork divergence."

NOTE on DCO sign-off: do NOT use `git commit -s` — produces only ONE Signed-off-by line. Use explicit HEREDOC body to write BOTH lines (DCO full name + GitHub handle attribution) per Phase 22 D-19.

**Plan-close smoke check (verbatim):**

```bash
# Expected: exactly the count of straight cherry-picks Task 2 dispositioned
# (default = 5: f3e7f885 + 0cba04a5 + 7329ef73 + 829c341a + ab74f5cd)
git log --format='%B' f6f4fb14..HEAD | grep -v '^#' | grep -c '^Upstream-commit: '

# Expected: exactly 1 (the f0abd413 manual replay)
git log --format='%B' f6f4fb14..HEAD | grep -v '^#' | grep -c '^Manual-replay: '

# Expected: 0 (case-sensitivity invariant)
git log --format='%B' f6f4fb14..HEAD | grep -v '^#' | grep -c '^Upstream-Author:'

# Expected: 2N where N = total commits in the plan (DCO + GitHub handle per commit)
git log --format='%B' f6f4fb14..HEAD | grep -v '^#' | grep -c '^Signed-off-by: '
```

Note the `grep -v '^#'` filter per `<task_breakdown>` "Grep gate hygiene" rule (header prose / comments would trigger self-invalidating grep gates otherwise; per feedback memory and Plan 22-05a/22-05b precedent).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Read source artifacts + capture pre-34-04b baselines</name>
  <files>(read-only — no files modified; produces /tmp/34-04b-* state files)</files>
  <read_first>
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-SUMMARY.md (FULL READ — Disposition table, D-34-04-FORK invariants list, § Deviations § D-02 STOP trigger, § Recommended next steps for 34-04b)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-PLAN.md (must_haves + acceptance criteria carry forward to 34-04b for the residual 6 commits)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-CONTEXT.md (D-34-A2 Wave 0 status; D-34-B1 manual-replay shape; D-34-D2 8 close-gates; D-34-E1..E5 invariants; D-34-E3 verbatim "Files in scope where fork drift is high (profile/mod.rs... policy.rs...) are read-upstream-and-replay candidates")
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05a-AUD-CORE-SUMMARY.md (mid-plan STOP trigger precedent + per-commit disposition table shape)
    - .planning/phases/22-upst2-upstream-v038-v040-parity-sync/22-05b-AUD-RENAME-PLAN.md (split-continuation shape — informs 34-04b's task structure as a split-from-34-04 continuation)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-09-FP-PACKS-PLAN.md (D-20 manual-replay task structure; per-commit disposition + per-commit execute + close-gate + push/PR shape)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-10-FP-PROXY-TLS-PLAN.md (D-20 manual-replay SPLIT shape: 1 clean replay + N read-and-document — mirror the per-commit `Manual-replay:` / D-19 trailer disposition pattern)
    - .planning/templates/upstream-sync-quick.md § Fork-divergence catalog (validate_path_within retention; hooks ownership; D-21 Windows-only file globs) + § D-19 cherry-pick trailer block (verbatim 6-line shape)
    - crates/nono-cli/src/profile/mod.rs (full file — 5584 lines; you need to know which regions carry the fork-only paths Plan 18.1-03 + Phase 22-01 + Phase 22-04 + Phase 26 PKGS-02)
    - crates/nono-cli/src/policy.rs (full file — 3131 lines; contains never_grant + apply_deny_overrides + find_denied_user_grants)
    - crates/nono-cli/data/policy.json (canonical schema layout; claude-code + codex builtin entries)
  </read_first>
  <action>
    Read the artifacts listed in <read_first>. Then:

    1. **Verify upstream remote + 6 commits reachable:**
       ```bash
       git fetch upstream --tags
       for sha in f0abd413 f3e7f885 0cba04a5 7329ef73 829c341a ab74f5cd; do
         git cat-file -e ${sha}^{commit} && echo "OK: $sha" || echo "MISSING: $sha"
       done
       # Expected: 6 OK lines, 0 MISSING. If MISSING: STOP + return PLAN BLOCKED.
       ```

    2. **Capture upstream metadata for trailer blocks (feeds Tasks 3-8):**
       ```bash
       for sha in f0abd413 f3e7f885 0cba04a5 7329ef73 829c341a ab74f5cd; do
         echo "==== $sha ===="
         git log -1 $sha --format='full_sha=%H subject=%s author=%an email=%ae tag=%D'
         echo "---- stat ----"
         git show --stat $sha | tail -5
         echo "---- body ----"
         git log -1 $sha --format='%b' | head -30
       done > /tmp/34-04b-upstream-meta.txt
       wc -l /tmp/34-04b-upstream-meta.txt
       ```

    3. **Verify Plan 34-04 close state — current HEAD must be f6f4fb14:**
       ```bash
       PRE_HEAD=$(git rev-parse HEAD)
       test "$PRE_HEAD" = "f6f4fb14e82ef4ec04d95851e78ead7d9bd8dc4b" || { echo "FAIL: expected f6f4fb14 but at $PRE_HEAD"; exit 1; }
       echo "PRE_HEAD=$PRE_HEAD" > /tmp/34-04b-baseline.txt

       # Verify 34-04 SUMMARY exists
       test -f .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-SUMMARY.md
       ```

    4. **Capture pre-34-04b fork-divergence baselines (record all to /tmp/34-04b-baseline.txt):**
       ```bash
       # Plan 18.1-03 capabilities.aipc + loaded_profile (expected 17)
       grep -c 'capabilities.aipc\|capabilities_aipc\|loaded_profile' crates/nono-cli/src/profile/mod.rs >> /tmp/34-04b-baseline.txt

       # Phase 22-01 ProfileDeserialize (expected >= 1)
       grep -c 'ProfileDeserialize\|struct ProfileDeserialize' crates/nono-cli/src/profile/mod.rs >> /tmp/34-04b-baseline.txt

       # Phase 22-04 validate_upstream_url (expected >= 1)
       grep -c 'validate_upstream_url' crates/nono-cli/src/profile/mod.rs >> /tmp/34-04b-baseline.txt

       # Phase 26 PKGS-02 ArtifactType::Plugin (expected 4)
       grep -c 'ArtifactType::Plugin\|    Plugin,' crates/nono-cli/src/package.rs >> /tmp/34-04b-baseline.txt

       # Phase 19 v2.1 never_grant + apply_deny_overrides (expected 21)
       grep -c 'never_grant\|apply_deny_overrides' crates/nono-cli/src/policy.rs >> /tmp/34-04b-baseline.txt

       # Phase 22-03 PKG-04 validate_path_within (expected 9)
       grep -c 'validate_path_within' crates/nono-cli/src/package_cmd.rs >> /tmp/34-04b-baseline.txt

       # 34-04 commit ac9f0a59 find_denied_user_grants helper (expected >= 1)
       grep -c 'find_denied_user_grants' crates/nono-cli/src/policy.rs >> /tmp/34-04b-baseline.txt

       # File line counts (5584 / 3131 / 1379 expected at plan-write time)
       wc -l crates/nono-cli/src/profile/mod.rs crates/nono-cli/src/policy.rs crates/nono-cli/src/package_cmd.rs >> /tmp/34-04b-baseline.txt

       cat /tmp/34-04b-baseline.txt
       ```

    5. **Workspace must be clean before starting:**
       ```bash
       git status --porcelain | wc -l   # Expected: 0
       ```

    6. **Baseline build green:**
       ```bash
       cargo build --workspace
       ```
  </action>
  <verify>
    <automated>git fetch upstream --tags &amp;&amp; for sha in f0abd413 f3e7f885 0cba04a5 7329ef73 829c341a ab74f5cd; do git cat-file -e ${sha}^{commit} || exit 1; done &amp;&amp; test "$(git rev-parse HEAD)" = "f6f4fb14e82ef4ec04d95851e78ead7d9bd8dc4b" &amp;&amp; test -f /tmp/34-04b-upstream-meta.txt &amp;&amp; test -f /tmp/34-04b-baseline.txt &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - All 6 upstream shas reachable from upstream remote.
    - `/tmp/34-04b-upstream-meta.txt` exists and records full_sha + subject + author + email for all 6 commits (feeds D-19 + Manual-replay trailer blocks in Tasks 3-8).
    - `/tmp/34-04b-baseline.txt` records pre-plan baselines: capabilities.aipc count (>= 17), ProfileDeserialize count (>= 1), validate_upstream_url count (>= 1), ArtifactType::Plugin count (>= 4), never_grant+apply_deny_overrides count (>= 21), validate_path_within count (>= 9), find_denied_user_grants count (>= 1), and file line counts.
    - Workspace clean (`git status --porcelain` empty).
    - HEAD is `f6f4fb14...` (Plan 34-04 SUMMARY close state).
    - Baseline `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    Pre-state captured; 6 upstream commits reachable; fork-defense baselines recorded; ready for disposition checkpoint.
  </done>
</task>

<task type="checkpoint:decision" gate="blocking">
  <name>Task 2: Disposition checkpoint — per-commit decisions + override_deny rename strategy</name>
  <files>/tmp/34-04b-disposition.txt</files>
  <read_first>
    - /tmp/34-04b-upstream-meta.txt (per-commit author + subject + body captured in Task 1)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-SUMMARY.md § Outcome (commits 19-23 rebase on f0abd413's canonical-schema base)
  </read_first>
  <action>
    **BLOCKING CHECKPOINT — pauses for user input.** Confirm two decisions before per-commit execute tasks begin:
    1. **Per-commit disposition** for each of 6 upstream commits (manual-replay vs cherry-pick vs skip).
    2. **`override_deny → bypass_protection` rename strategy** (Option A / B / C).

    **Recommended disposition table** (Task 1 read confirms; planner may adjust if upstream diff reveals surprises):

    | # | SHA | Default disposition | Rationale |
    |---|-----|---------------------|-----------|
    | 1 | `f0abd413` | **D-20 manual replay** | 60 files, ~5K-line delta, deep fork divergence in profile/mod.rs |
    | 2 | `f3e7f885` | **straight cherry-pick** | 2 files; profile_cmd.rs + profile_cli.rs test; rebases on canonical-schema base |
    | 3 | `0cba04a5` | **cherry-pick partial** | Release commit; drop Cargo.toml/Cargo.lock version-bumps; merge CHANGELOG entry (mirror Plan 34-04 commits 3 + 12 partial shape) |
    | 4 | `7329ef73` | **straight cherry-pick** | jsonschema 0.45.1 → 0.46.4; 3 files; clean Cargo.toml/Cargo.lock bump |
    | 5 | `829c341a` | **cherry-pick** (likely) | 10 files; +796/-21; draft commands; may conflict with fork's registry_client.rs + profile/mod.rs — confirm during read |
    | 6 | `ab74f5cd` | **straight cherry-pick** | 8 files; docs-only; +32/-31 |

    **override_deny → bypass_protection rename strategy (load-bearing for v2.3 backwards compat):**
    - **Option A** — hard rename to `bypass_protection`; v2.3 user profiles using `override_deny` fail to parse; v2.4 milestone breaking change; CHANGELOG entry required.
    - **Option B** — `#[serde(alias = "override_deny")]` on the canonical `bypass_protection` field; both keys deserialize to the same internal name; NO breaking change; no deprecation warning.
    - **Option C** (recommended) — port upstream's `deprecated_schema` module verbatim: deserialize legacy `override_deny` into a `LegacyPolicyPatch` struct that gets rewritten to canonical `bypass_protection` post-parse; emit a deprecation warning to stderr (matches upstream behavior); migration path for users.

    **Context:** Plan 34-04 hit its D-02 fallback gate on f0abd413 because the cherry-pick produced 10 conflicted files + 1 modify/delete + ~5K-line delta over fork-divergent code. The fork's profile/mod.rs is 5584 lines (vs upstream's much smaller analog) with deep Plan 18.1-03 / Phase 22-01 / Phase 22-04 / Phase 26 PKGS-02 divergence. A straight cherry-pick of f0abd413 would silently delete fork-only paths. Commits 19-23 (f3e7f885 onward) "all rebase on the canonical-schema state from f0abd413 and cannot be cherry-picked independently" per Plan 34-04 SUMMARY § Outcome.

    **Resume options (reply with the bracketed token):**
    - `[proceed-default]` — approve recommended disposition + Option C rename (matches upstream-fidelity; preserves v2.3 backwards compat with deprecation runway).
    - `[proceed-option-b]` — approve recommended disposition + Option B rename (serde alias; invisible to users; no deprecation runway).
    - `[proceed-option-a]` — approve recommended disposition + Option A rename (hard breaking change; v2.3 profiles fail; v2.4 milestone CHANGELOG entry required).
    - `[adjust-dispositions: &lt;describe-changes&gt;]` — modify the per-commit disposition table before proceeding.

    After user reply, write the resolved dispositions to `/tmp/34-04b-disposition.txt`:
    ```
    f0abd413 → manual-replay
    f3e7f885 → cherry-pick
    0cba04a5 → cherry-pick-partial (drop Cargo.toml + Cargo.lock version bumps)
    7329ef73 → cherry-pick
    829c341a → {cherry-pick | manual-replay - to be confirmed after read}
    ab74f5cd → cherry-pick
    rename-strategy → {Option A | Option B | Option C}
    ```
  </action>
  <verify>
    <automated>test -f /tmp/34-04b-disposition.txt &amp;&amp; test "$(grep -c '^rename-strategy' /tmp/34-04b-disposition.txt)" = "1" &amp;&amp; test "$(grep -cE '^f0abd413|^f3e7f885|^0cba04a5|^7329ef73|^829c341a|^ab74f5cd' /tmp/34-04b-disposition.txt)" = "6"</automated>
  </verify>
  <done>
    Per-commit dispositions recorded for all 6 upstream shas; rename strategy (Option A/B/C) recorded; user has approved or amended the default table.
  </done>
</task>

<task type="auto">
  <name>Task 3: D-20 manual replay — `f0abd413` canonical JSON schema restructure</name>
  <files>
    crates/nono-cli/src/profile/mod.rs
    crates/nono-cli/src/profile/builtin.rs
    crates/nono-cli/src/profile_cmd.rs
    crates/nono-cli/src/profile_runtime.rs
    crates/nono-cli/src/policy.rs
    crates/nono-cli/src/sandbox_prepare.rs
    crates/nono-cli/src/query_ext.rs
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/why_runtime.rs
    crates/nono-cli/data/policy.json
    crates/nono-cli/tests/manifest_roundtrip.rs
    crates/nono-cli/tests/deny_overlap_run.rs
    docs/cli/features/profiles-groups.mdx
    tests/integration/test_bypass_protection.sh (renamed from test_override_deny.sh — see action below)
  </files>
  <read_first>
    - `git show f0abd413` (full upstream diff — 60 files)
    - `git show f0abd413 -- crates/nono-cli/src/` (production-code diff)
    - `git show f0abd413 -- crates/nono-cli/data/` (canonical policy.json layout)
    - crates/nono-cli/src/profile/mod.rs § fork-only regions: `capabilities.aipc` deserialization (Plan 18.1-03); `ProfileDeserialize` companion struct (Phase 22-01); `validate_upstream_url` loopback gate (Phase 22-04)
    - .planning/templates/upstream-sync-quick.md § Fork-divergence catalog (all entries; load-bearing for this replay)
    - /tmp/34-04b-upstream-meta.txt (Leo Lapworth's metadata for the Manual-replay trailer)
    - /tmp/34-04b-disposition.txt (rename strategy Option A/B/C from Task 2)
    - /tmp/34-04b-baseline.txt (capabilities.aipc=17, ProfileDeserialize>=1, validate_upstream_url>=1, ArtifactType::Plugin=4, never_grant+apply_deny_overrides=21, validate_path_within=9, find_denied_user_grants>=1)
  </read_first>
  <action>
    This is the **D-20 manual replay** for upstream commit `f0abd413` per D-34-E3. NO `git cherry-pick` — read upstream's diff in full, identify the INTENT (canonical-schema sections, override_deny → bypass_protection rename, deprecated_schema module), and apply the intent BY HAND against the fork's current state WITHOUT deleting fork-only paths.

    **Step 1: Read upstream's diff structure.**
    ```bash
    git show f0abd413 --stat > /tmp/34-04b-f0abd413-stat.txt
    wc -l /tmp/34-04b-f0abd413-stat.txt
    git show f0abd413 -- crates/nono-cli/src/profile/mod.rs > /tmp/34-04b-f0abd413-profile-diff.txt
    git show f0abd413 -- crates/nono-cli/data/policy.json > /tmp/34-04b-f0abd413-policy-diff.txt
    git show f0abd413 -- crates/nono-cli/src/sandbox_prepare.rs > /tmp/34-04b-f0abd413-sandbox-diff.txt
    git show f0abd413 -- crates/nono-cli/src/cli.rs > /tmp/34-04b-f0abd413-cli-diff.txt
    ```
    Identify the structural changes:
    - Canonical JSON sections: `groups`, `commands.{allow,deny}`, `filesystem.{deny,bypass_protection}`
    - Rename: `override_deny`/`override_deny_paths` → `bypass_protection`/`bypass_protection_paths`
    - CLI flag rename: `--override-deny` → `--bypass-protection`
    - `deprecated_schema` module: handles legacy `override_deny` shape; emits deprecation warnings; rewrites to canonical
    - `SecurityConfig` narrowing: extracts groups + filesystem into separate canonical sections
    - `PolicyPatchConfig` removal from canonical `Profile` (lives in deprecated_schema only)
    - File rename: `tests/integration/test_override_deny.sh → test_bypass_protection.sh` (161 lines)

    **Step 2: Apply the intent by hand against fork's current state.**

    Sub-step 2a: Canonical sections in `policy.json` + Profile struct (`profile/mod.rs`):
    - Add `groups`, `commands`, `filesystem` canonical sections to the `Profile` / `LoadedProfile` structs.
    - **PRESERVE** the fork-only `capabilities.aipc` / `loaded_profile` paths (Plan 18.1-03). Add them as a sibling on the new canonical shape — do NOT delete.
    - **PRESERVE** the `ProfileDeserialize` companion-struct pattern (Phase 22-01). Compose it with the new canonical sections.
    - **PRESERVE** the `validate_upstream_url` loopback gate (Phase 22-04). Move to the appropriate canonical section if needed but DO NOT delete.

    Sub-step 2b: `override_deny → bypass_protection` rename per Task 2 decision:
    - **Option A** (hard rename): Replace all `override_deny`/`override_deny_paths` with `bypass_protection`/`bypass_protection_paths` in struct fields, function names, CLI flags. v2.3 user profiles fail to parse — document in CHANGELOG.
    - **Option B** (serde alias): Add `#[serde(alias = "override_deny")]` / `#[serde(alias = "override_deny_paths")]` on the canonical fields. Both keys deserialize cleanly.
    - **Option C** (deprecated_schema module): Port upstream's `deprecated_schema` module verbatim (read `git show f0abd413 -- crates/nono-cli/src/profile/deprecated_schema.rs` or wherever upstream placed it — should be visible in the diff). Deserialize legacy `override_deny` into a `LegacyPolicyPatch` struct; post-parse rewrite to canonical `bypass_protection`; emit a deprecation warning to stderr at `Profile::load` time.

    Sub-step 2c: Propagate the rename through callers:
    - `crates/nono-cli/src/sandbox_prepare.rs` — `PreparedSandbox` field rename if affected; `SandboxArgs` field rename.
    - `crates/nono-cli/src/cli.rs` — clap flag rename `--override-deny → --bypass-protection`; if Option B/C, KEEP `--override-deny` as a hidden alias (`#[arg(alias = "override-deny", hide = true)]`) for backwards compat.
    - `crates/nono-cli/src/why_runtime.rs` — display rename if applicable.
    - `crates/nono-cli/src/policy.rs` — consumer rename; **PRESERVE** never_grant/apply_deny_overrides (21 callsites) and the find_denied_user_grants helper.

    Sub-step 2d: Rename the integration test file:
    ```bash
    # Use git mv to preserve history (NOT cp + delete)
    git mv tests/integration/test_override_deny.sh tests/integration/test_bypass_protection.sh
    # Then update the file's internal references (--override-deny → --bypass-protection)
    sed -i 's/--override-deny/--bypass-protection/g' tests/integration/test_bypass_protection.sh
    sed -i 's/override_deny/bypass_protection/g' tests/integration/test_bypass_protection.sh
    ```

    Sub-step 2e: Update docs:
    ```bash
    # docs/cli/features/profiles-groups.mdx + docs/cli/usage/flags.mdx
    # Update prose references to the canonical schema sections and new flag names.
    # Mirror upstream's documentation changes from `git show f0abd413 -- docs/`.
    ```

    Sub-step 2f: Handle `profile_save_runtime.rs` modify/delete conflict:
    - Plan 34-04 SUMMARY § Deviations notes upstream-modified + fork-deleted. Confirm the file IS fork-deleted at HEAD f6f4fb14:
      ```bash
      test ! -f crates/nono-cli/src/profile_save_runtime.rs || echo "WARN: file exists; check"
      ```
    - The fork's deletion stands. Upstream's modifications to this file are NOT ported (fork deleted it deliberately).
    - Document this in the replay commit body.

    Sub-step 2g: Build + test:
    ```bash
    cargo build --workspace
    # If build fails, hand-resolve until build is green.

    cargo test -p nono-cli --lib profile  # Targeted profile-module tests
    # Tests should pass; if they fail with rename-related errors, that's expected before all callers are updated.
    ```

    **Step 3: Fork-divergence sentinels (run AFTER build green, BEFORE commit):**
    ```bash
    grep -c 'capabilities.aipc\|capabilities_aipc\|loaded_profile' crates/nono-cli/src/profile/mod.rs   # Expected: >= 17 (baseline preserved)
    grep -c 'ProfileDeserialize\|struct ProfileDeserialize' crates/nono-cli/src/profile/mod.rs         # Expected: >= 1
    grep -c 'validate_upstream_url' crates/nono-cli/src/profile/mod.rs                                  # Expected: >= 1
    grep -c 'ArtifactType::Plugin\|    Plugin,' crates/nono-cli/src/package.rs                          # Expected: >= 4 (unchanged; this replay should not touch package.rs)
    grep -c 'never_grant\|apply_deny_overrides' crates/nono-cli/src/policy.rs                           # Expected: >= 21
    grep -c 'validate_path_within' crates/nono-cli/src/package_cmd.rs                                   # Expected: >= 9 (unchanged; this replay should not touch package_cmd.rs)
    grep -c 'find_denied_user_grants' crates/nono-cli/src/policy.rs                                     # Expected: >= 1
    ```

    ANY sentinel that returns LESS than baseline is a STOP trigger — revert the replay edits and re-investigate.

    **Step 4: Stage + commit with Manual-replay trailer.**
    ```bash
    UPSTREAM_AUTHOR='Leo Lapworth <leo@cuckoo.org>'
    RENAME_STRATEGY=$(grep '^rename-strategy' /tmp/34-04b-disposition.txt | sed -E 's/.* → //')

    git add -A
    git commit -m "$(cat <<EOF
    replay(34-04b): canonical JSON schema restructure from upstream f0abd413

    Upstream's f0abd413 (feat(profile): #594 phase 2 — canonical JSON schema restructure)
    introduces canonical sections (groups, commands.{allow,deny}, filesystem.{deny,bypass_protection}),
    renames override_deny → bypass_protection across the profile schema + CLI flag surface,
    and adds a deprecated_schema module for legacy v2.3 profile parsing.

    Replayed by hand (D-20 per D-34-E3) rather than via straight cherry-pick because:
      - The upstream diff is 60 files, ~5K-line delta over fork-divergent code paths.
      - Fork's crates/nono-cli/src/profile/mod.rs is 5584 lines (vs upstream's smaller analog)
        with deep divergence from Plan 18.1-03 (capabilities.aipc widening), Phase 22-01
        (ProfileDeserialize companion-struct pattern), Phase 22-04 (validate_upstream_url
        OAuth2 loopback gate), and Phase 26 PKGS-02 (ArtifactType::Plugin round-trip).
      - Straight cherry-pick triggered the D-02 fallback gate during Plan 34-04 execution
        on 2026-05-11 (see .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/
        34-04-PATH-CANON-SCHEMA-SUMMARY.md § Deviations § D-02 STOP trigger for the
        10-file + 1 modify/delete conflict surface).

    Fork-only paths PRESERVED through this replay (baseline counts from /tmp/34-04b-baseline.txt):
      - capabilities.aipc / loaded_profile (Plan 18.1-03 G-06 widening) - 17 callsites
      - ProfileDeserialize companion-struct pattern (Phase 22-01 PROF-01..03) - >=1 callsite
      - validate_upstream_url loopback gate (Phase 22-04 OAuth2) - >=1 callsite
      - ArtifactType::Plugin round-trip (Phase 26 PKGS-02) - 4 callsites (in package.rs, not touched)
      - never_grant / apply_deny_overrides (Phase 19 v2.1) - 21 callsites in policy.rs
      - validate_path_within (Phase 22-03 PKG-04) - 9 callsites in package_cmd.rs (not touched)
      - find_denied_user_grants (added in Plan 34-04 commit ac9f0a59) - >=1 callsite

    override_deny → bypass_protection rename strategy: ${RENAME_STRATEGY}.
      [Document the strategy choice rationale here:
       - Option A: hard rename; v2.3 user profiles fail to parse; CHANGELOG entry added.
       - Option B: #[serde(alias = "override_deny")] on canonical bypass_protection field;
         both keys deserialize; no breaking change; no deprecation runway.
       - Option C: deprecated_schema module ported verbatim; legacy override_deny rewritten
         to canonical bypass_protection post-parse; deprecation warning emitted to stderr.]

    Not replayed:
      - crates/nono-cli/src/profile_save_runtime.rs (modify/delete conflict): fork deleted
        this file deliberately at an earlier phase; upstream's modifications to it are
        NOT ported. The fork's deletion stands.

    Per D-20 (manual port for heavily-diverged files; Phase 22 D-19 lineage; Phase 26-01
    PKGS-02 precedent for the same fork-preserve disposition class).

    Future re-evaluation trigger: if upstream's profile/mod.rs becomes structurally
    closer to the fork's shape, re-audit this manual-replay choice in a subsequent UPST
    phase.

    Manual-replay: f0abd413
    Upstream-tag: v0.47.0
    Upstream-author: ${UPSTREAM_AUTHOR}
    Co-Authored-By: ${UPSTREAM_AUTHOR}
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    **Step 5: Per-commit verification (mandatory — STOP on failure):**
    ```bash
    # D-34-E1: Windows-only files NOT touched
    test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" || exit 1

    # Manual-replay trailer present, NOT Upstream-commit:
    test "$(git log -1 --format='%B' | grep -v '^#' | grep -c '^Manual-replay: f0abd413')" = "1" || exit 1
    test "$(git log -1 --format='%B' | grep -v '^#' | grep -c '^Upstream-commit: ')" = "0" || exit 1

    # 2 DCO Signed-off-by lines
    test "$(git log -1 --format='%B' | grep -v '^#' | grep -c '^Signed-off-by: ')" = "2" || exit 1

    # Case-sensitivity invariant
    test "$(git log -1 --format='%B' | grep -v '^#' | grep -c '^Upstream-Author:')" = "0" || exit 1

    # Fork-divergence sentinels (re-run after commit; must match Step 3 results)
    test "$(grep -c 'capabilities.aipc\|capabilities_aipc\|loaded_profile' crates/nono-cli/src/profile/mod.rs)" -ge "17" || exit 1
    test "$(grep -c 'never_grant\|apply_deny_overrides' crates/nono-cli/src/policy.rs)" -ge "21" || exit 1
    test "$(grep -c 'validate_path_within' crates/nono-cli/src/package_cmd.rs)" -ge "9" || exit 1
    test "$(grep -c 'ArtifactType::Plugin\|    Plugin,' crates/nono-cli/src/package.rs)" -ge "4" || exit 1
    ```
  </action>
  <verify>
    <automated>test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" &amp;&amp; test "$(git log -1 --format='%B' | grep -v '^#' | grep -c '^Manual-replay: f0abd413')" = "1" &amp;&amp; test "$(git log -1 --format='%B' | grep -v '^#' | grep -c '^Upstream-commit: ')" = "0" &amp;&amp; test "$(grep -c 'capabilities.aipc\|capabilities_aipc\|loaded_profile' crates/nono-cli/src/profile/mod.rs)" -ge "17" &amp;&amp; test "$(grep -c 'never_grant\|apply_deny_overrides' crates/nono-cli/src/policy.rs)" -ge "21" &amp;&amp; test "$(grep -c 'validate_path_within' crates/nono-cli/src/package_cmd.rs)" -ge "9" &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - HEAD commit body carries `Manual-replay: f0abd413` (NOT `Upstream-commit:`); 2 DCO Signed-off-by lines; lowercase 'a' in `Upstream-author:` line.
    - D-34-E1 invariant: zero hits on `*_windows.rs` or `exec_strategy_windows/`.
    - Fork-defense baselines preserved: capabilities.aipc/loaded_profile >= 17, ProfileDeserialize >= 1, validate_upstream_url >= 1, never_grant+apply_deny_overrides >= 21, validate_path_within >= 9, ArtifactType::Plugin >= 4, find_denied_user_grants >= 1.
    - `cargo build --workspace` exits 0.
    - Replay commit body documents: (1) upstream's intent, (2) why straight cherry-pick was infeasible, (3) rename strategy chosen (Option A/B/C from Task 2), (4) fork-only paths preserved, (5) what was NOT replayed.
  </acceptance_criteria>
  <done>
    f0abd413 canonical-schema restructure manually replayed; fork-defense invariants preserved; build green.
  </done>
</task>

<task type="auto">
  <name>Task 4: Cherry-pick `f3e7f885` — render serde values in show/diff JSON output</name>
  <files>
    crates/nono-cli/src/profile_cmd.rs
    crates/nono-cli/tests/profile_cli.rs
  </files>
  <read_first>
    - `git show f3e7f885 --stat` (2 files; +191/-16)
    - `git show f3e7f885` (full diff — focus on profile_cmd.rs show/diff JSON output rendering)
    - /tmp/34-04b-upstream-meta.txt (Matt Palcic's metadata)
    - /tmp/34-04b-disposition.txt (confirm cherry-pick disposition from Task 2)
  </read_first>
  <action>
    Should be a clean cherry-pick after Task 3's canonical-schema base lands. Standard D-19 trailer block.

    **Step 1: Attempt cherry-pick.**
    ```bash
    git cherry-pick f3e7f885
    ```

    **Step 2a: If clean (no conflicts):** proceed to Step 3.

    **Step 2b: If conflicts (D-02 trigger threshold per Plan 34-04 SUMMARY = conflicts > 50 lines OR > 2 files):**
    - Abort: `git cherry-pick --abort`.
    - Investigate: are conflicts on fork-divergent surface (likely profile_cmd.rs render logic)?
    - If conflicts are small (< 50 lines, <= 2 files): hand-resolve + continue with `git cherry-pick --continue`.
    - If conflicts exceed threshold: STOP, escalate to user, propose D-20 manual replay for this commit (would amend Task 2 disposition).

    **Step 3: Amend with D-19 trailer.**
    ```bash
    UPSTREAM_AUTHOR='Matt Palcic <matt.palcic@naturalforms.com>'
    UPSTREAM_SUBJECT=$(git show -s --format='%s' f3e7f885)
    UPSTREAM_BODY=$(git show -s --format='%b' f3e7f885)

    git commit --amend -m "$(cat <<EOF
    ${UPSTREAM_SUBJECT}

    ${UPSTREAM_BODY}

    Upstream-commit: f3e7f885
    Upstream-tag: v0.47.0
    Upstream-author: ${UPSTREAM_AUTHOR}
    Co-Authored-By: ${UPSTREAM_AUTHOR}
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    **Step 4: Per-commit verification.**
    ```bash
    test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: f3e7f885')" = "1" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-tag: v0.47.0')" = "1" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-author: ')" = "1" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-Author:')" = "0" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Signed-off-by: ')" = "2" || exit 1
    cargo build --workspace
    ```
  </action>
  <verify>
    <automated>test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: f3e7f885')" = "1" &amp;&amp; test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - HEAD commit carries D-19 trailer with `Upstream-commit: f3e7f885` (lowercase 'a' in `Upstream-author:`); 2 Signed-off-by lines.
    - D-34-E1 invariant: zero hits on `*_windows.rs` or `exec_strategy_windows/`.
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    f3e7f885 (serde-rendered show/diff JSON output) cherry-picked onto canonical-schema base.
  </done>
</task>

<task type="auto">
  <name>Task 5: Cherry-pick partial `0cba04a5` — release v0.47.1 (drop Cargo version bumps)</name>
  <files>
    CHANGELOG.md
  </files>
  <read_first>
    - `git show 0cba04a5 --stat` (6 files: Cargo.toml bumps + CHANGELOG)
    - `git show 0cba04a5 -- CHANGELOG.md` (the entry to merge)
    - `git show 0cba04a5 -- '*.toml' Cargo.lock` (the version bumps to DROP)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-SUMMARY.md § Commits table § commits 3 + 12 (partial-cherry-pick shape precedent)
    - /tmp/34-04b-upstream-meta.txt (Luke Hinds's metadata)
  </read_first>
  <action>
    Mirror Plan 34-04 commits 3 (`d49585b8` v0.46.0 release) and 12 (`7a01e32a` v0.47.0 release) partial-cherry-pick shape: drop Cargo.toml/Cargo.lock version-bumps; merge CHANGELOG entry only.

    **Step 1: Cherry-pick with --no-commit.**
    ```bash
    git cherry-pick --no-commit 0cba04a5
    ```

    **Step 2: Reset Cargo.toml + Cargo.lock to fork's version (drop the bump).**
    ```bash
    # Identify which Cargo files were touched
    git status --porcelain | grep -E 'Cargo.toml|Cargo.lock'

    # Reset each to its pre-cherry-pick state
    for f in $(git diff --cached --name-only -- '*.toml' Cargo.lock); do
      git checkout HEAD -- "$f"
    done

    # Verify only CHANGELOG.md remains staged
    git diff --cached --name-only
    # Expected: CHANGELOG.md only (maybe some doc files too — verify against upstream's diff)
    ```

    **Step 3: Commit with D-19 trailer.**
    ```bash
    UPSTREAM_AUTHOR='Luke Hinds <lukehinds@gmail.com>'

    git commit -m "$(cat <<EOF
    chore: release v0.47.1

    Upstream version bumps in Cargo.toml + Cargo.lock NOT applied (fork tracks its own
    v2.3+ versioning scheme per .planning/STATE.md). CHANGELOG entry for v0.47.1
    merged for downstream sync provenance only.

    Mirrors Plan 34-04 commits 3 (d49585b8 v0.46.0 release) and 12 (7a01e32a v0.47.0
    release) partial-cherry-pick shape: drop upstream version bumps; merge CHANGELOG
    entry for traceability.

    Upstream-commit: 0cba04a5
    Upstream-tag: v0.47.1
    Upstream-author: ${UPSTREAM_AUTHOR}
    Co-Authored-By: ${UPSTREAM_AUTHOR}
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    **Step 4: Per-commit verification.**
    ```bash
    test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: 0cba04a5')" = "1" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-tag: v0.47.1')" = "1" || exit 1

    # Verify Cargo.toml/Cargo.lock NOT in the commit
    test "$(git diff --stat HEAD~1 HEAD -- 'Cargo.toml' '**/Cargo.toml' 'Cargo.lock' | wc -l)" = "0" || exit 1

    cargo build --workspace
    ```
  </action>
  <verify>
    <automated>test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: 0cba04a5')" = "1" &amp;&amp; test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" &amp;&amp; test "$(git diff --stat HEAD~1 HEAD -- 'Cargo.toml' '**/Cargo.toml' 'Cargo.lock' | wc -l)" = "0" &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - HEAD commit carries D-19 trailer with `Upstream-commit: 0cba04a5`; 2 Signed-off-by lines.
    - Cargo.toml + Cargo.lock NOT modified by this commit (version bumps dropped per Plan 34-04 partial-cherry-pick precedent).
    - CHANGELOG entry for v0.47.1 present.
    - D-34-E1 invariant: zero hits.
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    0cba04a5 release-bump landed (CHANGELOG only; Cargo bumps dropped).
  </done>
</task>

<task type="auto">
  <name>Task 6: Cherry-pick `7329ef73` — bump jsonschema 0.45.1 → 0.46.4</name>
  <files>
    crates/nono-cli/Cargo.toml
    crates/nono/Cargo.toml
    Cargo.lock
  </files>
  <read_first>
    - `git show 7329ef73 --stat` (3 files: Cargo.toml × 2 + Cargo.lock)
    - `git show 7329ef73` (full diff — the version bump)
    - /tmp/34-04b-upstream-meta.txt (dependabot's metadata)
  </read_first>
  <action>
    Clean cherry-pick of the jsonschema dependency bump. UNLIKE Task 5, this commit's Cargo.toml + Cargo.lock changes ARE applied (this is a dependency bump, not a version-tag release commit).

    **Step 1: Cherry-pick.**
    ```bash
    git cherry-pick 7329ef73
    ```

    **Step 2: If conflicts on Cargo.lock (likely, due to fork's diverged lockfile):**
    ```bash
    # If Cargo.lock conflicts: regenerate from Cargo.toml
    git checkout --theirs crates/nono-cli/Cargo.toml crates/nono/Cargo.toml
    git rm Cargo.lock 2>/dev/null || true
    cargo build --workspace   # regenerates Cargo.lock
    git add crates/nono-cli/Cargo.toml crates/nono/Cargo.toml Cargo.lock
    git cherry-pick --continue
    ```

    **Step 3: Amend with D-19 trailer.**
    ```bash
    UPSTREAM_AUTHOR='dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>'
    UPSTREAM_SUBJECT=$(git show -s --format='%s' 7329ef73)
    UPSTREAM_BODY=$(git show -s --format='%b' 7329ef73)

    git commit --amend -m "$(cat <<EOF
    ${UPSTREAM_SUBJECT}

    ${UPSTREAM_BODY}

    Upstream-commit: 7329ef73
    Upstream-tag: v0.47.1
    Upstream-author: ${UPSTREAM_AUTHOR}
    Co-Authored-By: ${UPSTREAM_AUTHOR}
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    **Step 4: Build + targeted test verifies the bump did not break fork's schema validation surface.**
    ```bash
    cargo build --workspace
    cargo test -p nono-cli --lib profile  # profile module owns jsonschema usage
    ```

    **Step 5: Per-commit verification.**
    ```bash
    test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: 7329ef73')" = "1" || exit 1
    test "$(grep -c '0\\.46' crates/nono-cli/Cargo.toml)" -ge "1" || exit 1   # jsonschema 0.46.x present
    ```
  </action>
  <verify>
    <automated>test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: 7329ef73')" = "1" &amp;&amp; test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" &amp;&amp; cargo build --workspace &amp;&amp; cargo test -p nono-cli --lib profile</automated>
  </verify>
  <acceptance_criteria>
    - HEAD commit carries D-19 trailer with `Upstream-commit: 7329ef73`; 2 Signed-off-by lines.
    - `jsonschema = "0.46.x"` present in Cargo.toml (verify via `grep '0\.46' crates/nono-cli/Cargo.toml`).
    - D-34-E1 invariant: zero hits.
    - `cargo build --workspace` exits 0.
    - `cargo test -p nono-cli --lib profile` exits 0 (jsonschema 0.46 bump did not break fork's schema validation surface).
  </acceptance_criteria>
  <done>
    7329ef73 jsonschema bump landed; build + profile tests green.
  </done>
</task>

<task type="auto">
  <name>Task 7: Cherry-pick `829c341a` — draft commands + package status (or manual replay if conflicts)</name>
  <files>
    crates/nono-cli/src/registry_client.rs
    crates/nono/src/error.rs
    crates/nono-cli/src/cli.rs
    crates/nono-cli/src/profile/mod.rs
    crates/nono-cli/src/profile_cmd.rs
    (+ up to 5 more per upstream's 10-file diff)
  </files>
  <read_first>
    - `git show 829c341a --stat` (10 files; +796/-21)
    - `git show 829c341a` (full diff — focus on draft commands surface)
    - `git show 829c341a -- crates/nono-cli/src/registry_client.rs` (likely conflict surface)
    - `git show 829c341a -- crates/nono-cli/src/profile/mod.rs` (may conflict with Task 3's canonical-schema replay)
    - /tmp/34-04b-upstream-meta.txt (Luke Hinds's metadata)
    - /tmp/34-04b-disposition.txt (Task 2 disposition for 829c341a; default cherry-pick)
  </read_first>
  <action>
    Largest cherry-pick in this plan (10 files, +796 insertions). Likely to conflict on profile/mod.rs (post-Task-3 canonical schema state) and registry_client.rs (fork-divergent surface). If conflicts exceed threshold, escalate to D-20 manual replay.

    **Step 1: Attempt cherry-pick.**
    ```bash
    git cherry-pick 829c341a
    ```

    **Step 2a: If clean:** proceed to Step 3.

    **Step 2b: If small conflicts (< 50 lines OR <= 2 files):**
    - Hand-resolve, preserving fork-only paths in profile/mod.rs (capabilities.aipc, ProfileDeserialize, validate_upstream_url, find_denied_user_grants).
    - `git cherry-pick --continue`.
    - Proceed to Step 3.

    **Step 2c: If large conflicts (> 50 lines OR > 2 files) — D-02 trigger:**
    - Abort: `git cherry-pick --abort`.
    - STOP this task: report conflict surface to user; propose D-20 manual replay for 829c341a; amend Task 2 disposition (`/tmp/34-04b-disposition.txt`).
    - User confirms manual-replay path before continuing. If manual-replay path: follow Task 3's shape (`Manual-replay: 829c341a` trailer; document fork-only preservation).

    **Step 3: Amend with D-19 trailer (or `Manual-replay:` if 2c manual-replay path).**
    ```bash
    UPSTREAM_AUTHOR='Luke Hinds <lukehinds@gmail.com>'
    UPSTREAM_SUBJECT=$(git show -s --format='%s' 829c341a)
    UPSTREAM_BODY=$(git show -s --format='%b' 829c341a)

    git commit --amend -m "$(cat <<EOF
    ${UPSTREAM_SUBJECT}

    ${UPSTREAM_BODY}

    Upstream-commit: 829c341a
    Upstream-tag: v0.47.1
    Upstream-author: ${UPSTREAM_AUTHOR}
    Co-Authored-By: ${UPSTREAM_AUTHOR}
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    **Step 4: Verify draft-commands acceptance:**
    ```bash
    # Smoke test — does `nono profile draft --help` work?
    cargo build --workspace
    cargo run --bin nono -- profile --help 2>&1 | grep -i 'draft' || echo "WARN: profile draft subcommand not visible — investigate"
    ```

    **Step 5: Per-commit verification.**
    ```bash
    test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: 829c341a\|^Manual-replay: 829c341a')" = "1" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-Author:')" = "0" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Signed-off-by: ')" = "2" || exit 1

    # Fork-defense sentinels still preserved
    test "$(grep -c 'capabilities.aipc\|capabilities_aipc\|loaded_profile' crates/nono-cli/src/profile/mod.rs)" -ge "17" || exit 1
    test "$(grep -c 'never_grant\|apply_deny_overrides' crates/nono-cli/src/policy.rs)" -ge "21" || exit 1
    test "$(grep -c 'validate_path_within' crates/nono-cli/src/package_cmd.rs)" -ge "9" || exit 1
    ```
  </action>
  <verify>
    <automated>test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: 829c341a\|^Manual-replay: 829c341a')" = "1" &amp;&amp; test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" &amp;&amp; test "$(grep -c 'capabilities.aipc\|capabilities_aipc\|loaded_profile' crates/nono-cli/src/profile/mod.rs)" -ge "17" &amp;&amp; test "$(grep -c 'never_grant\|apply_deny_overrides' crates/nono-cli/src/policy.rs)" -ge "21" &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - HEAD commit carries either D-19 `Upstream-commit: 829c341a` trailer OR `Manual-replay: 829c341a` (if Step 2c escalation occurred); 2 Signed-off-by lines.
    - D-34-E1 invariant: zero hits.
    - Fork-defense sentinels preserved: capabilities.aipc >= 17; never_grant >= 21; validate_path_within >= 9.
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    829c341a draft-commands landed (cherry-pick or manual-replay per Step 2 path).
  </done>
</task>

<task type="auto">
  <name>Task 8: Cherry-pick `ab74f5cd` — docs-only (deprecation wording, built-in vs pack)</name>
  <files>
    docs/cli/features/profiles-groups.mdx
    docs/cli/usage/flags.mdx
    (+ up to 6 more per upstream's 8-file diff)
  </files>
  <read_first>
    - `git show ab74f5cd --stat` (8 files; +32/-31 — docs only)
    - `git show ab74f5cd` (full diff)
    - /tmp/34-04b-upstream-meta.txt (SequeI's metadata)
  </read_first>
  <action>
    Should be a clean cherry-pick — docs-only changes; +32/-31 delta. Standard D-19 trailer.

    **Step 1: Cherry-pick.**
    ```bash
    git cherry-pick ab74f5cd
    ```

    **Step 2: Hand-resolve any small conflicts** (likely tiny if Task 3 already updated some docs/cli paths during canonical-schema replay).

    **Step 3: Amend with D-19 trailer.**
    ```bash
    UPSTREAM_AUTHOR='SequeI <asiek@redhat.com>'
    UPSTREAM_SUBJECT=$(git show -s --format='%s' ab74f5cd)
    UPSTREAM_BODY=$(git show -s --format='%b' ab74f5cd)

    git commit --amend -m "$(cat <<EOF
    ${UPSTREAM_SUBJECT}

    ${UPSTREAM_BODY}

    Upstream-commit: ab74f5cd
    Upstream-tag: v0.47.1
    Upstream-author: ${UPSTREAM_AUTHOR}
    Co-Authored-By: ${UPSTREAM_AUTHOR}
    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    **Step 4: Per-commit verification.**
    ```bash
    test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: ab74f5cd')" = "1" || exit 1
    test "$(git log -1 --format='%B' | grep -c '^Signed-off-by: ')" = "2" || exit 1
    ```
  </action>
  <verify>
    <automated>test "$(git log -1 --format='%B' | grep -c '^Upstream-commit: ab74f5cd')" = "1" &amp;&amp; test "$(git diff --stat HEAD~1 HEAD -- crates/ | grep -cE '_windows|exec_strategy_windows')" = "0" &amp;&amp; cargo build --workspace</automated>
  </verify>
  <acceptance_criteria>
    - HEAD commit carries D-19 trailer with `Upstream-commit: ab74f5cd`; 2 Signed-off-by lines.
    - D-34-E1 invariant: zero hits.
    - `cargo build --workspace` exits 0.
  </acceptance_criteria>
  <done>
    ab74f5cd docs-only update landed.
  </done>
</task>

<task type="auto">
  <name>Task 9: D-34-D2 8-gate close + plan-close smoke checks</name>
  <files>(read-only — verification only; produces /tmp/34-04b-close-gates.txt for SUMMARY)</files>
  <read_first>
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-CONTEXT.md § D-34-D2 (8 close-gates)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-SUMMARY.md § Verification table (Phase 25 CR-A CI-enforcement posture user accepted at 34-04 close for Gates 3 + 4)
    - /tmp/34-04b-baseline.txt (pre-plan baselines for sentinel comparison)
  </read_first>
  <action>
    Run D-34-D2 8-gate close + plan-close smoke checks. Mirror Plan 34-04 SUMMARY § Verification table for skip-rationale framing.

    **Gate 1: Workspace tests (Windows host).**
    ```bash
    cargo test --workspace --lib 2>&1 | tail -20
    # Expected: all pass. If failures: investigate; revert offending commit if needed.
    ```

    **Gate 2: Windows clippy (`-D warnings -D clippy::unwrap_used`).**
    ```bash
    cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used
    # Expected: zero warnings, zero unwrap_used.
    ```

    **Gate 3: Linux cross-target clippy (DOCUMENTED-SKIPPED per 34-04 close).**
    ```bash
    echo "Gate 3 (Linux cross-target clippy): SKIPPED - deferred to CI per dev-host limitation (x86_64-linux-gnu-gcc linker not installed; user accepted same posture at 34-04 close on 2026-05-11)." > /tmp/34-04b-close-gates.txt
    ```

    **Gate 4: macOS cross-target clippy (DOCUMENTED-SKIPPED).**
    ```bash
    echo "Gate 4 (macOS cross-target clippy): SKIPPED - deferred to CI per dev-host limitation (x86_64-apple-darwin cc toolchain not installed; user accepted same posture at 34-04 close on 2026-05-11)." >> /tmp/34-04b-close-gates.txt
    ```

    **Gate 5: `cargo fmt --all -- --check`.**
    ```bash
    cargo fmt --all -- --check
    # If fmt drift: run `cargo fmt --all`, stage, create a fork-only fmt-drift commit
    # (mirror Plan 34-04 commit 6d8a7e18 shape: NO Upstream-commit: trailer; just 2 DCO Signed-off-by lines).
    ```

    **Gate 6: Phase 15 5-row detached-console smoke (DOCUMENTED-SKIPPED).**
    ```bash
    echo "Gate 6 (Phase 15 5-row detached-console smoke): SKIPPED - requires admin-elevated session; not exercised on dev host (same posture as 34-04 SUMMARY)." >> /tmp/34-04b-close-gates.txt
    ```

    **Gate 7: `wfp_port_integration --ignored` (DOCUMENTED-SKIPPED).**
    ```bash
    echo "Gate 7 (wfp_port_integration --ignored): SKIPPED - requires admin + nono-wfp-service installed; not exercised on dev host (same posture as 34-04 SUMMARY)." >> /tmp/34-04b-close-gates.txt
    ```

    **Gate 8: `learn_windows_integration` (DOCUMENTED-SKIPPED).**
    ```bash
    echo "Gate 8 (learn_windows_integration): SKIPPED - requires elevated session + ETW provider; not exercised on dev host (same posture as 34-04 SUMMARY)." >> /tmp/34-04b-close-gates.txt
    ```

    **Plan-close smoke check: D-19 trailer count.**
    ```bash
    PRE_HEAD=f6f4fb14e82ef4ec04d95851e78ead7d9bd8dc4b
    TOTAL_COMMITS=$(git log --format='%H' $PRE_HEAD..HEAD | wc -l)
    UPSTREAM_COMMIT_TRAILERS=$(git log --format='%B' $PRE_HEAD..HEAD | grep -v '^#' | grep -c '^Upstream-commit: ')
    MANUAL_REPLAY_TRAILERS=$(git log --format='%B' $PRE_HEAD..HEAD | grep -v '^#' | grep -c '^Manual-replay: ')
    SIGNED_OFF_LINES=$(git log --format='%B' $PRE_HEAD..HEAD | grep -v '^#' | grep -c '^Signed-off-by: ')

    echo "Total commits in 34-04b: $TOTAL_COMMITS" >> /tmp/34-04b-close-gates.txt
    echo "Upstream-commit: trailers: $UPSTREAM_COMMIT_TRAILERS (expected: 5 cherry-picks if default dispositions held; may be 4 if Task 7 escalated to manual-replay)" >> /tmp/34-04b-close-gates.txt
    echo "Manual-replay: trailers: $MANUAL_REPLAY_TRAILERS (expected: 1 for f0abd413, possibly 2 if Task 7 escalated)" >> /tmp/34-04b-close-gates.txt
    echo "Signed-off-by lines: $SIGNED_OFF_LINES (expected: 2 × $TOTAL_COMMITS = $((2 * TOTAL_COMMITS)))" >> /tmp/34-04b-close-gates.txt

    # Case-sensitivity invariant
    test "$(git log --format='%B' $PRE_HEAD..HEAD | grep -v '^#' | grep -c '^Upstream-Author:')" = "0" || { echo "FAIL: case-sensitivity invariant"; exit 1; }

    # 2N Signed-off-by check
    test "$SIGNED_OFF_LINES" = "$((2 * TOTAL_COMMITS))" || { echo "FAIL: expected $((2 * TOTAL_COMMITS)) Signed-off-by lines, got $SIGNED_OFF_LINES"; exit 1; }
    ```

    **D-34-E1 per-commit invariant (re-check across entire chain).**
    ```bash
    for sha in $(git log --format='%H' $PRE_HEAD..HEAD); do
      count=$(git diff --stat $sha^..$sha -- crates/ | grep -cE '_windows|exec_strategy_windows')
      if [ "$count" != "0" ]; then
        echo "FAIL: $sha touches Windows files: $count"
        exit 1
      fi
    done
    echo "D-34-E1 per-commit invariant: PASS (0 hits across all $TOTAL_COMMITS commits)" >> /tmp/34-04b-close-gates.txt
    ```

    **Fork-defense final sentinels.**
    ```bash
    grep -c 'capabilities.aipc\|capabilities_aipc\|loaded_profile' crates/nono-cli/src/profile/mod.rs >> /tmp/34-04b-close-gates.txt
    grep -c 'ProfileDeserialize\|struct ProfileDeserialize' crates/nono-cli/src/profile/mod.rs >> /tmp/34-04b-close-gates.txt
    grep -c 'validate_upstream_url' crates/nono-cli/src/profile/mod.rs >> /tmp/34-04b-close-gates.txt
    grep -c 'ArtifactType::Plugin\|    Plugin,' crates/nono-cli/src/package.rs >> /tmp/34-04b-close-gates.txt
    grep -c 'never_grant\|apply_deny_overrides' crates/nono-cli/src/policy.rs >> /tmp/34-04b-close-gates.txt
    grep -c 'validate_path_within' crates/nono-cli/src/package_cmd.rs >> /tmp/34-04b-close-gates.txt
    grep -c 'find_denied_user_grants' crates/nono-cli/src/policy.rs >> /tmp/34-04b-close-gates.txt

    cat /tmp/34-04b-close-gates.txt
    ```

    All sentinels MUST be >= /tmp/34-04b-baseline.txt values. If any are below: STOP, investigate, revert offending commit.
  </action>
  <verify>
    <automated>cargo test --workspace --lib &amp;&amp; cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used &amp;&amp; cargo fmt --all -- --check &amp;&amp; test -f /tmp/34-04b-close-gates.txt &amp;&amp; test "$(git log --format='%B' f6f4fb14e82ef4ec04d95851e78ead7d9bd8dc4b..HEAD | grep -v '^#' | grep -c '^Upstream-Author:')" = "0" &amp;&amp; for sha in $(git log --format='%H' f6f4fb14e82ef4ec04d95851e78ead7d9bd8dc4b..HEAD); do count=$(git diff --stat $sha^..$sha -- crates/ | grep -cE '_windows|exec_strategy_windows'); [ "$count" = "0" ] || exit 1; done</automated>
  </verify>
  <acceptance_criteria>
    - Gates 1, 2, 5 PASS on Windows host.
    - Gates 3, 4 documented-skipped with rationale "deferred to CI per dev-host limitation; user accepted same posture at 34-04 close on 2026-05-11".
    - Gates 6, 7, 8 documented-skipped per "admin/service-not-available" rationale.
    - Plan-close smoke: `git log --format='%B' f6f4fb14..HEAD | grep -v '^#' | grep -c '^Upstream-commit: '` returns the expected cherry-pick count (default 5 if all dispositions held).
    - Plan-close smoke: `git log --format='%B' f6f4fb14..HEAD | grep -v '^#' | grep -c '^Manual-replay: '` returns 1 (f0abd413).
    - Case-sensitivity invariant: zero `^Upstream-Author:` hits.
    - 2N Signed-off-by check: equals 2 × total commit count.
    - D-34-E1 per-commit invariant: 0 Windows-file hits across all commits.
    - Fork-defense sentinels all >= pre-plan baselines.
    - /tmp/34-04b-close-gates.txt produced for SUMMARY.
  </acceptance_criteria>
  <done>
    8 close-gates evaluated; D-34-E1 + fork-defense invariants preserved across the chain; plan ready for push.
  </done>
</task>

<task type="auto">
  <name>Task 10: Push to origin/main + open PR per D-34-D1 + write SUMMARY</name>
  <files>
    .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04b-FP-CANONICAL-SCHEMA-SUMMARY.md
  </files>
  <read_first>
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04-PATH-CANON-SCHEMA-SUMMARY.md (SUMMARY shape precedent)
    - .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-CONTEXT.md § D-34-D1 (direct-on-main; one PR per plan)
    - /tmp/34-04b-disposition.txt + /tmp/34-04b-baseline.txt + /tmp/34-04b-close-gates.txt (SUMMARY inputs)
  </read_first>
  <action>
    **Step 1: Write SUMMARY at `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04b-FP-CANONICAL-SCHEMA-SUMMARY.md`.**

    Mirror the 34-04 SUMMARY shape:
    - Frontmatter: phase, plan_number=04b, slug, cluster_id=C7-residual, status, outcome, subsystem, tags, requirements=[C7-residual], metrics (duration, completed_date, commits_landed, upstream_trailers, manual_replay_trailers, windows_file_touches, all sentinel values), dependency_graph, tech_stack, key_files, decisions (D-34-04b-RENAME-01 documenting Option A/B/C choice; D-34-04b-FORK-01 documenting fork-defense preservation; etc.).
    - Outcome paragraph: 6 commits landed (1 manual-replay + 5 cherry-picks default; adjust if Task 7 escalated). Wave 0.5 closes; Wave 1+ unblocks.
    - Pre-Plan-34-04b HEAD: `f6f4fb14...`.
    - Plan-34-04b HEAD: `git rev-parse HEAD` at this step.
    - Commits table (mirror 34-04 SUMMARY § Commits table shape).
    - Verification table (mirror 34-04 SUMMARY § Verification table; reference /tmp/34-04b-close-gates.txt).
    - D-34-E1 Windows-only file invariant section.
    - Fork-defense invariants section.
    - Deviations (Task 7 escalation if applicable; any fork-only fmt-drift commits per Task 9 Gate 5).
    - Self-Check section.

    **Step 2: Commit the SUMMARY.**
    ```bash
    git add .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04b-FP-CANONICAL-SCHEMA-SUMMARY.md
    git commit -m "$(cat <<EOF
    docs(34-04b): SUMMARY for canonical-schema residual replay (Plan 34-04b close)

    Plan 34-04b closes the canonical-JSON-schema residual cluster (C7-residual) that
    Plan 34-04 paused on 2026-05-11. 1 D-20 manual-replay (f0abd413) + 5 D-19
    cherry-picks (f3e7f885 + 0cba04a5 + 7329ef73 + 829c341a + ab74f5cd) landed on main.

    Phase 22-05a/22-05b mid-plan-split precedent followed for the split-from-34-04
    continuation; D-34-A2 Wave 0 sequential-gate inherited as Wave 0.5.

    Wave 1+ plans (34-01, 34-03, 34-06, 34-02, 34-05, 34-07, 34-08, 34-09, 34-10)
    UNBLOCKED.

    Signed-off-by: Oscar Mack <oscar.mack.jr@gmail.com>
    Signed-off-by: oscarmackjr-twg <oscar.mack.jr@gmail.com>
    EOF
    )"
    ```

    **Step 3: Push to origin/main per D-34-D1.**
    ```bash
    git push origin main
    test "$(git log origin/main..main --oneline | wc -l)" = "0" || { echo "FAIL: local main not in sync with origin/main"; exit 1; }
    ```

    **Step 4: Open PR per D-34-D1 (one PR per plan).**
    ```bash
    PR_BODY=$(cat <<EOF
    ## Summary
    - Plan 34-04b closes the cluster-C7 residual that Plan 34-04 paused on 2026-05-11 at upstream commit f0abd413 (canonical JSON schema restructure)
    - 1 D-20 manual-replay (f0abd413: canonical schema + override_deny → bypass_protection rename) + 5 D-19 cherry-picks (f3e7f885 + 0cba04a5 + 7329ef73 + 829c341a + ab74f5cd) landed
    - Phase 22-05a/22-05b mid-plan-split precedent; D-34-A2 Wave 0 inherited as Wave 0.5; Wave 1+ plans UNBLOCKED

    ## Test plan
    - [x] Gate 1: cargo test --workspace --lib (Windows host) PASS
    - [x] Gate 2: Windows clippy -D warnings -D clippy::unwrap_used PASS
    - [ ] Gate 3: Linux cross-target clippy — deferred to CI (user accepted same posture at 34-04 close)
    - [ ] Gate 4: macOS cross-target clippy — deferred to CI (user accepted same posture at 34-04 close)
    - [x] Gate 5: cargo fmt --all --check PASS
    - [ ] Gate 6: Phase 15 5-row detached-console smoke — skipped (admin required)
    - [ ] Gate 7: wfp_port_integration --ignored — skipped (admin + nono-wfp-service)
    - [ ] Gate 8: learn_windows_integration — skipped (admin + ETW provider)
    - [x] D-34-E1 Windows-only file invariant: 0 hits across all commits
    - [x] Fork-defense sentinels: capabilities.aipc >= 17, never_grant+apply_deny_overrides >= 21, validate_path_within >= 9, ArtifactType::Plugin >= 4 — all preserved through canonical-schema replay

    🤖 Generated with [Claude Code](https://claude.com/claude-code)
    EOF
    )

    gh pr create --title "Phase 34 Plan 04b: C7-residual canonical-schema manual replay + 5 cherry-picks" --body "$PR_BODY"
    ```

    Capture PR URL; record in /tmp/34-04b-pr-url.txt for SUMMARY backfill.
  </action>
  <verify>
    <automated>test -f .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04b-FP-CANONICAL-SCHEMA-SUMMARY.md &amp;&amp; test "$(git log origin/main..main --oneline | wc -l)" = "0"</automated>
  </verify>
  <acceptance_criteria>
    - SUMMARY exists at the expected path with frontmatter + outcome + commits table + verification table + D-34-E1 section + fork-defense section + Self-Check.
    - SUMMARY commit landed with 2 DCO Signed-off-by lines (NO Upstream-commit: trailer).
    - `git push origin main` succeeds.
    - `git log origin/main..main --oneline | wc -l` returns 0 post-push.
    - PR opened via `gh pr create`; URL captured for SUMMARY backfill.
  </acceptance_criteria>
  <done>
    Plan 34-04b closed; cluster C7-residual complete; Wave 1+ plans unblocked.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| user profile (JSON) → Profile::load deserializer | v2.3 user profiles with legacy `override_deny` key may load into post-rename canonical schema |
| upstream commit → fork's profile/mod.rs replay | Plan 18.1-03 / Phase 22-01 / Phase 22-04 / Phase 26 PKGS-02 fork-only paths cross this boundary during manual replay |
| jsonschema 0.45.1 → 0.46.4 | dep version bump may interact with fork's schema-validation surface (custom keywords, error message shapes) |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-34-04b-01 | T (Tampering) | crates/nono-cli/src/profile/mod.rs Plan 18.1-03 `capabilities.aipc` / `loaded_profile` paths | mitigate | Pre/post grep on capabilities.aipc + loaded_profile; baseline 17 captured at plan-write time. Task 3 + Task 7 + Task 9 sentinels assert >= 17. Replay commit body documents preservation. STOP trigger if sentinel falls below baseline. |
| T-34-04b-02 | T (Tampering) | crates/nono-cli/src/package.rs Phase 26 PKGS-02 `ArtifactType::Plugin` round-trip | mitigate | Pre/post grep on `ArtifactType::Plugin` (baseline 4). Task 3 replay should NOT touch package.rs at all (the canonical-schema restructure is profile-side); sentinel verifies preservation. `cargo test -p nono-cli package` exits 0 post-replay. |
| T-34-04b-03 | E (Elevation of Privilege) | crates/nono-cli/src/cli.rs + crates/nono-cli/src/profile/mod.rs `override_deny → bypass_protection` rename | mitigate | Task 2 explicit rename-strategy decision (Option A/B/C). Option A: hard rename + v2.3 user-profile breaking change + CHANGELOG entry. Option B: `#[serde(alias = ...)]` for backwards compat. Option C: deprecated_schema module ported verbatim with deprecation warning. Rename strategy documented in f0abd413 replay commit body for auditability. |
| T-34-04b-04 | T (Tampering) | D-34-E1 Windows-only file invariant | mitigate | Per-commit `git diff --stat <prev>..<this> -- crates/ \| grep -E '_windows\|exec_strategy_windows' \| wc -l` must return 0. Asserted in Tasks 3-8 individually AND across the full chain in Task 9. STOP trigger if any commit touches a Windows-gated file. |
| T-34-04b-05 | R (Repudiation) | D-19 trailer compliance for the 5 cherry-pick commits + Manual-replay: convention for f0abd413 | mitigate | Plan-close smoke check `git log --format='%B' f6f4fb14..HEAD \| grep -v '^#' \| grep -c '^Upstream-commit: '` returns exactly the cherry-pick count (default 5); `grep -c '^Manual-replay: '` returns 1 (f0abd413 only; possibly 2 if Task 7 escalated); 2N Signed-off-by lines; case-sensitivity invariant zero `^Upstream-Author:`. Per-commit verification in Tasks 4-8. |
| T-34-04b-06 | D (Denial of Service) | jsonschema 0.45.1 → 0.46.4 bump in Task 6 may break fork's existing schema validation surface | mitigate | Task 6 Step 4: `cargo test -p nono-cli --lib profile` exits 0 post-bump. If new jsonschema version introduces breaking error-shape changes or custom-keyword regressions, Task 6 STOP + revert + escalate. Acceptance criterion explicit. |
</threat_model>

<verification>
- All 6 upstream commits dispositioned (Task 2): default disposition table approved or amended.
- `f0abd413` D-20 manually replayed (Task 3): canonical-schema sections added; override_deny → bypass_protection rename applied per Task 2 strategy; fork-only paths preserved verbatim.
- `f3e7f885` cherry-picked (Task 4) with D-19 trailer.
- `0cba04a5` partial cherry-picked (Task 5) with D-19 trailer; Cargo bumps dropped.
- `7329ef73` cherry-picked (Task 6) with D-19 trailer; jsonschema 0.46.4 bumped; profile tests green.
- `829c341a` cherry-picked OR manually replayed (Task 7) per Task 2 disposition or Step 2c escalation.
- `ab74f5cd` cherry-picked (Task 8) with D-19 trailer.
- D-34-D2 8-gate close (Task 9): Gates 1, 2, 5 PASS; Gates 3, 4, 6, 7, 8 documented-skipped per 34-04 SUMMARY precedent.
- Plan-close smoke (Task 9): `grep -c '^Upstream-commit: '` = 5 cherry-picks (or 4 if Task 7 escalated); `grep -c '^Manual-replay: '` = 1 (f0abd413; or 2 if Task 7 escalated); `grep -c '^Upstream-Author:'` = 0; `grep -c '^Signed-off-by: '` = 2 × total commit count.
- D-34-E1 per-commit invariant (Task 9): 0 Windows-file hits across the entire chain.
- Fork-defense sentinels (Task 9): all >= baseline (capabilities.aipc/loaded_profile >= 17; ProfileDeserialize >= 1; validate_upstream_url >= 1; ArtifactType::Plugin >= 4; never_grant+apply_deny_overrides >= 21; validate_path_within >= 9; find_denied_user_grants >= 1).
- SUMMARY written (Task 10) + pushed to origin/main + PR opened.
</verification>

<success_criteria>
- 6-7 commits on `main` (1 manual-replay + 5 cherry-picks + 1 SUMMARY commit; +/- 1 if Task 7 escalated to manual-replay; +/- 1 fork-only fmt-drift commit per Task 9 Gate 5 if needed).
- All cherry-pick commits carry verbatim D-19 trailer block (lowercase 'a' in `Upstream-author:`; 2 Signed-off-by lines per commit).
- f0abd413 replay commit carries `Manual-replay: f0abd413` trailer (NOT `Upstream-commit:`); body documents rename strategy + fork-only preservation.
- Zero edits to `*_windows.rs` / `exec_strategy_windows/` files across the entire chain (D-34-E1 verified per-commit AND across-chain in Task 9).
- All fork-defense sentinels preserved at or above baseline (capabilities.aipc >= 17; never_grant+apply_deny_overrides >= 21; validate_path_within >= 9; ArtifactType::Plugin >= 4).
- D-34-D2 8-gate close: Gates 1, 2, 5 PASS; Gates 3, 4, 6, 7, 8 documented-skipped per 34-04 SUMMARY precedent (user accepted same posture on 2026-05-11).
- `cargo build --workspace` exits 0 at HEAD.
- `cargo test -p nono-cli --lib profile` exits 0 (jsonschema 0.46.4 bump did not regress fork's schema validation surface).
- Plan 34-04b SUMMARY exists at `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04b-FP-CANONICAL-SCHEMA-SUMMARY.md`.
- `git log origin/main..main --oneline | wc -l` returns 0 post-push.
- Wave 1+ plans (34-01, 34-03, 34-06, then Wave 2: 34-02, 34-05, 34-07, 34-08, then Wave 3: 34-09, 34-10) UNBLOCKED.
</success_criteria>

<output>
After completion, create `.planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-04b-FP-CANONICAL-SCHEMA-SUMMARY.md` per Task 10 instructions. Frontmatter mirrors 34-04 SUMMARY shape with cluster_id: C7-residual; status: complete (or split-checkpoint if Task 7 escalated); outcome paragraph documents the canonical-schema restructure replay outcome; commits table lists all 6 upstream shas + their dispositions + landed fork shas + landed Upstream-commit: OR Manual-replay: trailer; verification table documents D-34-D2 8-gate results; D-34-04b decisions section documents the rename strategy chosen (Option A/B/C) + the fork-defense preservation; Self-Check section verifies all baselines preserved.
</output>
