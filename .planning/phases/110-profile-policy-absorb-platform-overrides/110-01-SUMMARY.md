---
phase: 110-profile-policy-absorb-platform-overrides
plan: 01
subsystem: profile
tags: [profile, platform_overrides, windows_low_il_broker, windows_interpreters, extends, merge_profiles, schema, upstream-absorb]

# Dependency graph
requires:
  - phase: 51
    provides: "windows_low_il_broker fail-secure OR semantics (T-51A-02) merge_profiles field"
  - phase: 71
    provides: "windows_interpreters dedup-append merge_profiles field"
provides:
  - "PlatformOverrides/PlatformOverride types (per-OS profile-patch container) on Profile"
  - "apply_platform_overrides, wired as the first step of finalize_profile"
  - "merge_platform_overrides/merge_platform_override_slot deep-merge (D-10 extends-preservation)"
  - "platform_overrides schema property in nono-profile.schema.json"
  - "D-08a fail-secure OR/union precedent for windows_low_il_broker/windows_interpreters under platform_overrides.windows"
affects: [110-04, 110-05, 110-06, 110-07, 110-08, v3.7-windows-tool-sandbox-parity]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-OS profile patch merged in via the existing merge_profiles child-wins/dedup-append/deny-union rules (no new precedence code)"
    - "Exhaustive struct-literal safety net: adding a Profile field forces every construction site to be updated (compile error, not silent drop)"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/data/nono-profile.schema.json

key-decisions:
  - "D-08a preserved verbatim: windows_low_il_broker/windows_interpreters keep Phase 51/71 fail-secure OR/union merge_profiles semantics under platform_overrides.windows; no special-case code added"
  - "4th Profile-shape site discovered beyond the plan's named 3: ProfileDef::to_raw_profile in policy.rs (policy.json-defined built-in profiles) also needed platform_overrides: None"

patterns-established:
  - "Per-OS platform_overrides merged as an ordinary merge_profiles child (reuses existing field-level merge rules unchanged)"

requirements-completed: [PROF-01]

# Metrics
duration: 20min
completed: 2026-07-30
---

# Phase 110 Plan 01: platform_overrides + extends preservation Summary

**Per-OS `platform_overrides` profile-patch field (upstream `ae1c513e`/`719975cf`) ported verbatim, applied once in `finalize_profile` after `extends` resolution, deep-merged through `merge_profiles` so it survives `extends` intact — with `windows_low_il_broker`/`windows_interpreters` deliberately kept on their Phase 51/71 fail-secure OR/union semantics per D-08a.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-30
- **Tasks:** 3/3 completed
- **Files modified:** 3

## Accomplishments
- `PlatformOverrides`/`PlatformOverride` types added to `profile/mod.rs`, with a custom `Deserialize` on `PlatformOverride` that rejects nested `extends`/`platform_overrides` at parse time (T-110-02, 2 regression tests).
- `platform_overrides: Option<PlatformOverrides>` declared at all 4 required Profile-shape sites (the plan's named 3 — `Profile` struct, `ProfileDeserialize`, the `From<ProfileDeserialize>` impl — plus a 4th discovered during Task 1's build: `ProfileDef::to_raw_profile()` in `policy.rs`, the policy.json→Profile conversion for built-in profiles) plus both exhaustive test-fixture literals (`base_profile()`/`child_profile()`).
- `apply_platform_overrides` wired as the FIRST statement in `finalize_profile`, before `merge_implicit_default_groups`; immediately followed by post-merge re-validation of `validate_profile_custom_credentials`, `validate_env_credential_keys`, and `validate_set_vars` (D-10/`719975cf`'s fix — closes the gap where an override-introduced invalid entry would bypass validation entirely, T-110-03).
- `merge_profiles`'s naive `platform_overrides: None,` (Task 1 placeholder) replaced with `merge_platform_overrides(base.platform_overrides, child.platform_overrides)` — the D-10 deep-merge that lets a base profile's `platform_overrides` survive `extends` resolution into a child that doesn't declare its own override, and merges same-OS-key override blocks rather than one clobbering the other.
- `nono-profile.schema.json` (fork-only, strict `additionalProperties: false`) gained a `platform_overrides` top-level property so profiles using the field pass `validate_against_schema()`.
- 14 new tests in a new `platform_overrides_tests` module, all host-portable (keyed off `crate::platform::current_os_name()`, not hardcoded to one OS) — see Task Commits below for the full list.
- Confirmed Phase 51's `merge_profiles_or_semantics_base_true_child_false` (T-51A-02) is unmodified and still passing; confirmed via `git diff HEAD` that no hunk touches its body.

## Task Commits

Each task was committed atomically (all DCO-signed):

1. **Task 1: PlatformOverrides/PlatformOverride types + 3-site field declaration + schema property** - `d44b5647` (feat)
2. **Task 2: apply_platform_overrides + finalize_profile wiring + extends-preserving deep merge + post-merge re-validation** - `9872afa6` (feat)
3. **Task 3: Port upstream test suite + D-08/D-08a back-compat + OR-semantics proof tests + schema-resolution test** - `e8ce3a63` (test)

_No plan-metadata-only commit — this response's final `docs` commit (STATE.md/ROADMAP.md/REQUIREMENTS.md/SUMMARY.md) follows separately per the execution protocol._

## Files Created/Modified
- `crates/nono-cli/src/profile/mod.rs` - `PlatformOverrides`/`PlatformOverride` types + custom Deserialize; `platform_overrides` field on `Profile`/`ProfileDeserialize`/`From` impl + 2 test fixtures; `apply_platform_overrides`, `merge_platform_overrides`, `merge_platform_override_slot`; `finalize_profile` wiring + post-merge re-validation; 14 new tests in `platform_overrides_tests`
- `crates/nono-cli/src/policy.rs` - `platform_overrides: None,` added to `ProfileDef::to_raw_profile()` (4th exhaustive-literal site, discovered via build error — built-in policy.json profiles don't declare `platform_overrides` today)
- `crates/nono-cli/data/nono-profile.schema.json` - new `platform_overrides` top-level schema property (object, `additionalProperties: false`, per-OS slots `additionalProperties: true` since no reusable `$defs/Profile` exists in this schema)

## Decisions Made
- **D-08a preserved verbatim, no new precedence code.** `apply_platform_overrides` calls the same `merge_profiles` every other inheritance step uses; `windows_low_il_broker`'s `base || child` OR-line and `windows_interpreters`'s `dedup_append` union-line (both untouched, verified via `git diff`) apply unchanged when the override is merged in as the "child" — an override can tighten (add) but never silently loosen (disable/remove) either flag. Pinned by a new distinguishing test (`platform_overrides_windows_low_il_broker_or_semantics_top_level_true_override_false_stays_true`): top-level `true` + override `false` → `true`, the only input shape that tells fail-secure OR apart from new-form-wins.
- **4th Profile-shape site.** Research/patterns docs named 3 sites (`Profile`, `ProfileDeserialize`, `From` impl); the build immediately surfaced a 4th, structurally analogous site: `policy.rs::ProfileDef::to_raw_profile()`, which builds a raw `Profile` from a policy.json-defined built-in profile via its own exhaustive struct literal. Fixed the same way `environment: None` was handled there previously (built-in profiles don't declare the field yet).
- **Schema per-OS slots use `additionalProperties: true`, not full nested `Profile` schema fidelity.** `nono-profile.schema.json` has no reusable `$defs/Profile` reference (profile shape is described inline), so replicating full nested validation would mean hand-duplicating the entire schema. Goal (per plan interfaces) was "a profile using `platform_overrides` does not fail `validate_against_schema()`", which the looser per-slot shape satisfies while the real structural validation continues to happen via the `Profile`/`PlatformOverride` serde deserializers at parse time.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 4th exhaustive-literal site (`policy.rs::ProfileDef::to_raw_profile`) not named in the plan**
- **Found during:** Task 1 (`cargo build -p nono-sandbox-cli --bin nono` after adding the `platform_overrides` field)
- **Issue:** The plan's `<interfaces>` named exactly 3 `Profile`-shape sites (struct, `ProfileDeserialize`, `From` impl) plus 2 test fixtures. The compiler additionally flagged `policy.rs:174`'s `profile::Profile { ... }` literal inside `ProfileDef::to_raw_profile()` — a 4th exhaustive struct literal converting policy.json built-in profile definitions into a raw `Profile`.
- **Fix:** Added `platform_overrides: None,` with a doc comment explaining built-in policy.json profiles don't declare the field yet (mirrors the existing `environment: None` precedent in the same literal).
- **Files modified:** `crates/nono-cli/src/policy.rs`
- **Verification:** `cargo build -p nono-sandbox-cli --bin nono` exits 0 with zero warnings.
- **Committed in:** `d44b5647` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking — missing struct-literal field the compiler caught).
**Impact on plan:** Necessary for compilation; no scope creep. Confirms the plan's own "exhaustive struct literal as compile-forced safety net" pattern worked exactly as designed — it caught an omission the plan itself didn't anticipate.

## Issues Encountered

**Package rename fallout (pre-existing, not this plan's doing):** `cargo build -p nono-cli --lib`/`cargo test -p nono-cli --lib` (the plan's literal verify commands) fail because Phase 102 renamed the crate to `nono-sandbox-cli` and it has no `[lib]` target (bin-only crate). Substituted `cargo build -p nono-sandbox-cli --bin nono` / `cargo test -p nono-sandbox-cli --bin nono` throughout, consistent with the same environment-substitution already recorded in STATE.md for Phase 102 P05.

**Pre-existing dev-host test baseline (not this plan's doing):** Full `cargo test -p nono-sandbox-cli --bin nono` shows 1425 passed / 11 failed. All 11 failures are in files this plan never touched (`audit_session.rs`, `config/mod.rs`, `profile_cmd.rs`, `protected_paths.rs`) and match the documented dev-host env-state baseline (STATE.md Phase 100 P01 decision: stale `my-agent.json`, env-lock `PoisonError` cascade under parallel test execution, session-dir count drift). Confirmed out of scope per the SCOPE BOUNDARY rule (pre-existing failures in unrelated files).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `platform_overrides` is fully wired: parses, applies only the current-OS block, survives `extends` resolution intact, deep-merges base+child overrides, and both `windows_low_il_broker`/`windows_interpreters` resolve correctly under either the legacy top-level form or the new `platform_overrides.windows` form — fail-secure OR/union semantics preserved per D-08a.
- Wave 1 sibling plans (110-02, 110-03) can proceed independently — no shared-file conflicts expected with `profile/mod.rs`'s `NetworkConfig`/token-expansion work in Wave 2 (110-04/05/06), since this plan touched only the `Profile`-level `platform_overrides` field and its 4 sites, not `NetworkConfig`.
- No blockers. Cross-target clippy (linux-gnu via `cross`, apple-darwin via `cargo-zigbuild`) both clean, required because `policy.rs` contains `#[cfg(target_os = "linux"/"macos")]` blocks elsewhere in the file (CLAUDE.md file-level trigger), even though this plan's own edit to that file was a single non-cfg-gated struct-literal line.

---
*Phase: 110-profile-policy-absorb-platform-overrides*
*Completed: 2026-07-30*

## Self-Check: PASSED

All created/modified files verified present on disk; all 3 task commit hashes (`d44b5647`, `9872afa6`, `e8ce3a63`) verified present in `git log --oneline --all`.
