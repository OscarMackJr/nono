---
phase: 36-upst3-deep-closure
plan: 01c
subsystem: profile
tags: [rename, atomic, serde, clap, override_deny, bypass_protection, d-36-b4]

# Dependency graph
requires:
  - phase: 36-upst3-deep-closure
    plan: 01b
    provides: "canonical FilesystemConfig.bypass_protection + CommandsConfig fields; PolicyPatchConfig.override_deny still present"
provides:
  - "Rust identifier override_deny retired from all 17 CLI source files; canonical bypass_protection dominant"
  - "CLI flag direction flipped: --bypass-protection is canonical long-name, --override-deny is visible_alias"
  - "SandboxState.bypass_protection_paths (serialized; serde alias preserves NONO_CAP_FILE backward compat)"
  - "Legacy JSON override_deny key accepted indefinitely via serde alias on PolicyPatchConfig + FilesystemConfig"
affects:
  - "36-upst3-deep-closure/36-01d (data migration: policy.json, schema.json, docs)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Atomic identifier rename discipline (D-36-B4): single commit, cargo build/clippy/test gate pre-commit"
    - "Serde alias canonical-first orientation: field is bypass_protection, alias = override_deny"
    - "Clap visible_alias for indefinite backwards-compat: long=bypass-protection, visible_alias=override-deny"

key-files:
  created: []
  modified:
    - "crates/nono-cli/src/profile/mod.rs — PolicyPatchConfig::bypass_protection (was override_deny); serde alias flipped"
    - "crates/nono-cli/src/cli.rs — SandboxArgs::bypass_protection; #[arg(long=bypass-protection, visible_alias=override-deny)]"
    - "crates/nono-cli/src/capability_ext.rs — profile.policy.bypass_protection; finalize_caps param renamed"
    - "crates/nono-cli/src/sandbox_state.rs — SandboxState::bypass_protection_paths + bypass_protection_as_paths()"
    - "crates/nono-cli/src/profile_runtime.rs — PreparedSandbox::bypass_protection_paths; collect_bypass_protection_paths()"
    - "crates/nono-cli/src/profile_save_runtime.rs — all 23 callsites renamed"
    - "crates/nono-cli/src/profile_cmd.rs — all 17 callsites + bypass_protection_expanded local var"
    - "crates/nono-cli/src/policy.rs + profile/builtin.rs + learn.rs + query_ext.rs + sandbox_prepare.rs — field accesses"
    - "crates/nono-cli/src/command_runtime.rs + execution_runtime.rs + launch_runtime.rs + main.rs — override_deny_paths → bypass_protection_paths"
    - "crates/nono-cli/src/why_runtime.rs — bypass_protection_as_paths() call"

key-decisions:
  - "Plan 36-01c D-36-B4 atomic-rename: single commit across 17 files; cargo build/clippy/test gate pre-commit"
  - "deprecated_schema.rs UNTOUCHED: it is the legacy-key detection module; all override_deny refs intentional"
  - "apply_deny_overrides helper NOT renamed per PATTERNS.md: semantics broader than schema-level field"
  - "SandboxState.bypass_protection_paths gets serde alias override_deny_paths for NONO_CAP_FILE backward compat"
  - "test_schema_validates_full_profile reverted to use legacy override_deny key: nono-profile.schema.json update is Plan 36-01d scope"
  - "Cross-target Linux/macOS clippy skipped — missing cross-compilers on Windows host (same as 36-01a + 36-01b)"
  - "Pre-existing nono flaky test helper_stamps_session_token_from_env: env-var race in parallel tests; passes in isolation; unrelated to rename"

patterns-established:
  - "Serde alias direction after rename: new canonical field name is the Rust identifier; old name lives in alias string"
  - "Clap visible_alias direction after CLI flag rename: new canonical is long=, old name is visible_alias="

requirements-completed: [REQ-PORT-CLOSURE-02]

# Metrics
duration: 90min
completed: 2026-05-13
---

# Phase 36 Plan 01c: Override-Deny to Bypass-Protection Atomic Rename Summary

**Atomic mechanical rename of `override_deny` → `bypass_protection` across 17 fork-side source files — CLI flag direction flipped, legacy JSON + CLI acceptance preserved via serde alias + clap visible_alias per D-36-B3 indefinite backwards-compat.**

## Performance

- **Duration:** ~90 min
- **Started:** 2026-05-13T~03:00Z
- **Completed:** 2026-05-13T~04:30Z
- **Tasks:** 2 (Task 1: atomic rename + commit; Task 2: close-gate verification)
- **Files modified:** 17

## Accomplishments

- Atomically renamed `override_deny` Rust struct field identifier to `bypass_protection` across all 17 CLI source files in a single commit (D-36-B4 invariant)
- Flipped CLI flag canonical direction: `--bypass-protection` is now the primary long-name, `--override-deny` remains accepted via clap `visible_alias`
- Flipped serde alias direction on both `PolicyPatchConfig` and `FilesystemConfig`: canonical field is `bypass_protection`, legacy JSON key `override_deny` still deserializes via `#[serde(alias = "override_deny")]`
- Renamed runtime identifiers consistently: `override_deny_paths` → `bypass_protection_paths` across `PreparedSandbox`, `LaunchPlan`, `ProfilePrepared`; `SandboxState::bypass_protection_paths` gains serde alias for in-flight session backward compat
- All 969 nono-cli tests pass; clippy clean; fmt clean; build green at commit boundary

## Task Commits

1. **Task 1: Pre-flight + atomic mechanical rename (17 files)** - `e168dd6b` (refactor)
2. **Task 2: Close-gate verification** — no additional commit (verification only; Task 1 commit is the plan commit)

## Files Created/Modified

The 17 files modified in the single atomic commit:

- `crates/nono-cli/src/profile/mod.rs` — PolicyPatchConfig::bypass_protection (serde alias flip); FilesystemConfig::bypass_protection already existed from 36-01b; legacy detection functions preserved
- `crates/nono-cli/src/cli.rs` — SandboxArgs::bypass_protection; flag direction flipped; test names kept as descriptive
- `crates/nono-cli/src/capability_ext.rs` — profile.policy.bypass_protection; finalize_caps param → profile_bypass_protection; apply_deny_overrides preserved
- `crates/nono-cli/src/sandbox_state.rs` — SandboxState::bypass_protection_paths; serde alias override_deny_paths for compat
- `crates/nono-cli/src/profile_runtime.rs` — PreparedSandbox + collect_bypass_protection_paths + expand_bypass_protection_path
- `crates/nono-cli/src/profile_save_runtime.rs` — 23 callsites; PatchGrant::bypass_protection bool
- `crates/nono-cli/src/profile_cmd.rs` — 17 callsites + bypass_protection_expanded local var
- `crates/nono-cli/src/command_runtime.rs` — override_deny_paths → bypass_protection_paths
- `crates/nono-cli/src/execution_runtime.rs` — bypass_protection_paths parameter
- `crates/nono-cli/src/launch_runtime.rs` — LaunchPlan::bypass_protection_paths
- `crates/nono-cli/src/main.rs` — bypass_protection_paths initializer
- `crates/nono-cli/src/sandbox_prepare.rs` — PreparedSandbox::bypass_protection_paths
- `crates/nono-cli/src/profile/builtin.rs` — 6 callsites
- `crates/nono-cli/src/policy.rs` — 6 callsites
- `crates/nono-cli/src/learn.rs` — learned_bypass_protection_paths function renamed
- `crates/nono-cli/src/query_ext.rs` — 4 callsites
- `crates/nono-cli/src/why_runtime.rs` — bypass_protection_as_paths() call

## Decisions Made

- **deprecated_schema.rs is intentionally untouched.** It is the legacy-key detection module. All its `override_deny` references are intentional JSON key string literals and struct field names needed for detecting/rewriting legacy profile JSON.
- **apply_deny_overrides NOT renamed per PATTERNS.md.** Its semantics encompass any deny-override operation, not just the schema-level field.
- **SandboxState backward compat.** `bypass_protection_paths` serde alias `override_deny_paths` preserves ability to read NONO_CAP_FILE JSON written by previous versions during in-flight sessions.
- **test_schema_validates_full_profile uses legacy key.** The JSON schema file (`nono-profile.schema.json`) update is Plan 36-01d scope; using `override_deny` here keeps the test passing until the schema migration lands.
- **Cross-target Linux/macOS clippy skipped.** Missing `x86_64-linux-gnu-gcc` and macOS cross-compilers on this Windows host — same documented skip as 36-01a + 36-01b. CI matrix covers these platforms.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] deprecated_schema.rs excluded from rename pass**
- **Found during:** Task 1 (baseline inventory)
- **Issue:** Plan lists 17 files but grep found 18 files with `override_deny`. The 18th is `deprecated_schema.rs` which was created in Plan 36-01a specifically for legacy-key detection. All its `override_deny` refs are intentional string literals for the legacy detection mechanism, not schema-level field identifiers.
- **Fix:** Explicitly excluded `deprecated_schema.rs` from the rename sed pass.
- **Files modified:** None (exclusion, not a fix)
- **Verification:** Build + tests pass; legacy detection tests still work

**2. [Rule 1 - Bug] Serde alias direction fix required post-sed**
- **Found during:** Task 1 (manual inspection after sed pass)
- **Issue:** The mechanical sed rename `override_deny` → `bypass_protection` also renamed the string inside `#[serde(default, alias = "bypass_protection")]` to `alias = "bypass_protection"` — which is self-referential and wrong. The correct post-rename alias is `alias = "override_deny"` (so legacy JSON still deserializes).
- **Fix:** Manually edited both `FilesystemConfig` and `PolicyPatchConfig` serde annotations to `alias = "override_deny"`. Also fixed the detection function `json_value_has_key(&value, "override_deny")` and `emit_once("override_deny", "bypass_protection")` which the sed had incorrectly renamed.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`
- **Verification:** Tests `legacy_override_deny_key_detected`, `legacy_override_deny_key_still_deserializes`, `filesystem_legacy_override_deny_alias_to_bypass_protection` all pass

**3. [Rule 1 - Bug] Test fixture corrections in profile/mod.rs**
- **Found during:** Task 1 (test fixture review)
- **Issue:** Several test JSON strings that intentionally use the legacy `override_deny` key (to test detection/deserialization) were incorrectly renamed to `bypass_protection` by the sed pass.
- **Fix:** Reverted legacy-coverage test fixtures to use `"override_deny"` JSON key; kept canonical-behavior tests using `"bypass_protection"`.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`
- **Verification:** All 969 nono-cli tests pass

**4. [Rule 1 - Bug] test_schema_validates_full_profile policy key**
- **Found during:** Task 1 (test run showing 1 failure)
- **Issue:** `test_schema_validates_full_profile` test data was renamed from `"override_deny"` to `"bypass_protection"` but the JSON schema file still only knows `override_deny`. Error: "Additional properties are not allowed ('bypass_protection' was unexpected) at /policy".
- **Fix:** Reverted test policy key to `"override_deny"` with a comment explaining the schema is updated in Plan 36-01d.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`
- **Verification:** Test passes; comment documents handoff to 36-01d

**5. [Rule 2 - Missing Critical] Additional compound identifier renames**
- **Found during:** Task 1 (post-rename inventory showing 5 files with 0 bypass_protection hits)
- **Issue:** `override_deny_paths` (compound identifier for resolved runtime paths) in `PreparedSandbox`, `LaunchPlan`, `SandboxState`, etc. were not renamed by the word-boundary sed (correct behavior: `\boverride_deny\b` doesn't match within `override_deny_paths`). For consistency and to bring all 17 files into the canonical naming, these needed manual renames.
- **Fix:** Renamed `override_deny_paths` → `bypass_protection_paths` across all 17 files; renamed helper functions `collect_override_deny_paths`, `expand_override_deny_path`, `learned_override_deny_paths` → canonical names; added `SandboxState` serde alias for backward compat.
- **Files modified:** `command_runtime.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `sandbox_prepare.rs`, `profile_runtime.rs`, `sandbox_state.rs`, `why_runtime.rs`, `learn.rs`
- **Verification:** Build + tests pass

---

**Total deviations:** 5 auto-fixed (3 Rule 1 bugs, 1 Rule 2 missing, 1 Rule 1 exclusion)
**Impact on plan:** All auto-fixes were necessary to complete the rename correctly and consistently. No scope creep.

## Issues Encountered

**Pre-existing flaky test in unmodified nono crate:**
`supervisor::aipc_sdk::tests::windows_loopback_tests::helper_stamps_session_token_from_env` fails intermittently during parallel test execution due to env-var contamination from other tests. This is a pre-existing issue documented in prior phases (Phase 19 CLEAN-02 notes). Passes reliably in isolation. The `nono` crate was not modified in this plan.

**Known stub / deferral:**
`data/policy.json` line 695 still uses `override_deny` key in JSON data. This is intentional — data migration is Plan 36-01d's scope per T-36-01-DATA-JSON-MISS.

## Close-Gate Verification (D-36-A5)

| Step | Status | Notes |
|------|--------|-------|
| 1. cargo test --workspace --lib | PASS | 969 nono-cli + 678 nono tests pass |
| 2. cargo clippy (Windows, release) | PASS | 0 warnings/errors |
| 3. cargo clippy (Linux cross-target) | SKIP | Missing x86_64-linux-gnu-gcc; same documented skip as 36-01a + 36-01b |
| 4. cargo clippy (macOS cross-target) | SKIP | Missing macOS cross-compiler |
| 5. cargo fmt --all -- --check | PASS | Formatting clean |
| 6. Detached-console surface check | SKIP | No *_windows.rs files modified |
| 7. WFP filter surface check | SKIP | No WFP files modified |
| 8. Learn integration check | SKIP | Only learned_bypass_protection_paths rename; no behavioral change |

## Commit Verification (D-20 + D-36-B4)

- **Single atomic commit:** `e168dd6b` — 17 files changed, 187 insertions, 181 deletions
- **No Upstream-commit trailer:** verified (grep returns 0)
- **f0abd413 design source cited:** verified
- **36-01c + D-36-B4 + 17 files in commit body:** verified (3 hits)
- **DCO sign-off:** 2x Signed-off-by present
- **Commit touches ≥17 RS files:** 17 (verified via `git diff --name-only HEAD~1..HEAD -- '*.rs'`)

## Test Fixture Dispositions

| File | Key | Disposition |
|------|-----|-------------|
| `profile/mod.rs` detection tests | `"override_deny"` in test JSON | KEPT — intentional legacy-coverage tests for the serde detection machinery |
| `profile/mod.rs::test_schema_validates_full_profile` | `"override_deny"` in policy block | KEPT using legacy key; comment documents Plan 36-01d schema migration |
| `profile/mod.rs` canonical tests | `"bypass_protection"` in test JSON | RENAMED — correct canonical key for behavior tests |
| `deprecated_schema.rs` all tests | `"override_deny"` in test strings | UNTOUCHED — module's purpose is detecting this legacy key |
| `data/policy.json` line 695 | `"override_deny"` in profile data | DEFERRED to Plan 36-01d data migration |

## Acceptance Criteria Verification

| Criterion | Result |
|-----------|--------|
| Schema-level override_deny eliminated (non-comment, non-alias, non-visible-alias) | MET — only intentional string literals remain |
| bypass_protection in ≥17 source files | MET — 18 files |
| CLI long = "bypass-protection" | MET — 2 instances |
| CLI visible_alias = "override-deny" | MET — 2 instances |
| alias = "override_deny" in profile/mod.rs | MET — 4 instances |
| apply_deny_overrides preserved in capability_ext.rs | MET — 3 instances |
| Build green at commit boundary | MET — release mode |
| Clippy clean at commit boundary | MET — 0 warnings |
| Test green at commit boundary | MET — 969 passing |
| Single atomic commit shape | MET — 1 commit, 17 RS files |
| Commit body cites 36-01c + D-36-B4 + 17 files | MET — 3 grep hits |
| No Upstream-commit: trailer | MET — 0 hits |
| f0abd413 design source cited | MET — 1 hit |
| DCO trailer | MET — 2 Signed-off-by |

## Hand-off to Plan 36-01d

Plan 36-01c closes REQ-PORT-CLOSURE-02 acceptance criterion #1 (canonical Rust identifier rename). Plan 36-01d must complete:

1. `data/policy.json` line 695: rename `override_deny` key in built-in profile data to `bypass_protection`
2. `data/nono-profile.schema.json`: add `bypass_protection` property in policy section (alongside or replacing `override_deny`)
3. `data/profile-authoring-guide.md`: update all `override_deny` references to `bypass_protection`
4. `docs/cli/features/profiles-groups.mdx` + `docs/cli/usage/flags.mdx`: update documentation
5. Scripts: `scripts/test-list-aliases.sh` + `scripts/lint-docs.sh` (new tooling per PATTERNS.md § Plan 36-01d)

## Known Stubs

None — the rename is mechanical; no data placeholders were introduced.

## Threat Flags

None — this is an identifier rename; no new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries were introduced.

---
*Phase: 36-upst3-deep-closure*
*Completed: 2026-05-13*
