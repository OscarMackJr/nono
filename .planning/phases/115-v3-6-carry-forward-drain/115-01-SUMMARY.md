---
phase: 115-v3-6-carry-forward-drain
plan: 01
subsystem: auth
tags: [rust, serde, profile-merge, credential-injection, compile-time-guard]

# Dependency graph
requires: []
provides:
  - "CustomCredentialDef.inject_mode/inject_header as Option<T>, merged with .or(base) (D-01)"
  - "Exhaustive NEW-02 regression test split into pipeline-routed (Test A) and validation-agnostic exhaustive (Test B) variants (D-03, closes ACC-04)"
  - "HookConfig compile-time exhaustive-destructure guard (D-04)"
affects: [115-02, 115-03, 115-04, 115-05, 115-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Option<T> + .or(base) merge for profile-override inheritance, homogeneous across all 14 CustomCredentialDef optional fields"
    - "No-Default-impl struct as an implicit compile-time completeness guard (E0063 on every un-updated literal)"
    - "const _: fn(T) = |T { field: _, ... }| {}; as a zero-cost exhaustive-destructure compile-time guard"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/network_policy.rs
    - crates/nono-cli/src/profile_cmd.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/data/nono-profile.schema.json

key-decisions:
  - "D-01: inject_mode/inject_header changed from serde-defaulted non-Option to Option<T>, dropping #[serde(default...)] — Option-absence is now the inherit signal"
  - "D-02: upstream stays required-on-the-child, deliberately, with the asymmetry documented inline"
  - "D-03: NEW-02 regression test split into two — a pipeline-routed test (validation-compatible shape) and a direct-merge exhaustive test (bypasses validate_custom_credential, immune to its mutual-exclusion rules)"
  - "D-04: HookConfig gets an exhaustive-destructure compile-time guard; hooks.hooks merge stays whole-value-replace (not field-merged)"
  - "D-05: merge_custom_credential_def's doc comment corrected — injection placement is not presentation"

patterns-established:
  - "Bite-proof verification protocol: temporarily reintroduce the defect class, capture the compiler's exact rejection, restore, re-verify clean — applied to both the no-Default struct guard and the HookConfig destructure guard"

requirements-completed: [DRAIN-01]

# Metrics
duration: ~45min
completed: 2026-08-08
---

# Phase 115 Plan 01: CustomCredentialDef Inject-Field Inheritance Summary

**`CustomCredentialDef.inject_mode`/`inject_header` changed from serde-defaulted `InjectMode`/`String` to `Option<InjectMode>`/`Option<String>`, merged with the same `.or(base)` shape as the struct's other 12 optional fields, closing NEW-05 (silent reset to header/`Authorization` on override) and ACC-04 (missing `spiffe` assertion in the regression test).**

## Performance

- **Duration:** ~45 min (commit-timestamp span; actual session included upfront context reading)
- **Started:** 2026-08-08T21:38:22+01:00 (first task commit)
- **Completed:** 2026-08-08T21:46:10+01:00 (last task commit)
- **Tasks:** 3/3 completed
- **Files modified:** 5

## Accomplishments

- `CustomCredentialDef.inject_mode`/`inject_header` are now `Option<T>`, closing the class of bug where a `platform_overrides.<os>` block that redefines a credential and omits these two fields silently resets a `url_path`/`query_param` route to header-mode `Authorization` (NEW-05).
- All 38 `CustomCredentialDef` struct-literal construction sites (2 production, 36 test fixtures) updated across `profile/mod.rs`, `network_policy.rs`, and `proxy_runtime.rs` — count confirmed live via the bite-proof compile check, not just grep.
- The NEW-02 regression test is now exhaustive over all 14 non-`upstream` fields (Test B), immune to `validate_custom_credential`'s mutual-exclusion rules by calling `merge_custom_credential_def` directly — closing ACC-04's missing `spiffe` assertion — plus a companion pipeline-routed test (Test A) proving the same inheritance through the real, validation-gated path.
- `HookConfig` has a compile-time exhaustive-destructure guard (`const _: fn(HookConfig) = |HookConfig { event: _, matcher: _, script: _ }| {};`) so a future field addition fails to compile (`E0027`) instead of silently reopening the NEW-02 class in `merge_profiles`'s whole-value-replace hooks merge.
- Both compile-time guards (D-01's no-`Default` struct shape, D-04's `HookConfig` destructure) were live bite-proof verified: temporarily reintroduced the defect, captured the compiler's rejection, reverted, confirmed clean.

## Task Commits

Each task was committed atomically:

1. **Task 1: CustomCredentialDef Option<T> shape + merge + production consumer + schema + bite-proof** - `c0ee1286` (fix)
2. **Task 2: Exhaustive NEW-02 regression test, split by validation-reachability (D-03, closes ACC-04)** - `d104cbf7` (test)
3. **Task 3: HookConfig compile-time guard with verified bite-proof (D-04)** - `99e45f33` (feat)

**Plan metadata:** SUMMARY commit follows (see final commit in this response).

## Files Created/Modified

- `crates/nono-cli/src/profile/mod.rs` - `CustomCredentialDef` field shape change, `merge_custom_credential_def` merge + doc comment (D-05), all in-file struct literals, `validate_custom_credential`/`validate_header_mode` consumer fixes, the two split regression tests, `HookConfig` compile-time guard + `merge_profiles` comment
- `crates/nono-cli/src/network_policy.rs` - the one production `CustomCredentialDef` → `RouteConfig` conversion site (`.unwrap_or_default()` / `.unwrap_or_else(default_inject_header)`), 12 test-fixture literals
- `crates/nono-cli/src/profile_cmd.rs` - manifest-export consumer (`cred.inject_mode`/`cred.inject_header` reads) and the profile-diff pretty-printer, updated for the new `Option` shape (Rule 3: blocking compile fix, not in the plan's original `files_modified` list)
- `crates/nono-cli/src/proxy_runtime.rs` - 2 test-fixture `CustomCredentialDef` literals
- `crates/nono-cli/data/nono-profile.schema.json` - `inject_mode`/`inject_header` now `oneOf`-with-`null`, matching the sibling `credential_format` shape, no `"default"` key

## Decisions Made

- D-01 (locked in CONTEXT.md): `Option<T>` type change over deserialize-time key-presence tracking or validation-rejection — makes a missed consumer site a compile failure, not a silent wire-placement change.
- D-02 (locked): `upstream` stays required-on-the-child, asymmetry documented inline in the corrected doc comment so a future reader does not re-file it as NEW-05's successor.
- D-03 (locked): the regression test goes exhaustive and is split into two tests rather than adding a single `spiffe` assertion to the existing one — Test B (direct-merge) is immune to `validate_custom_credential`'s mutual-exclusion rules by construction, so it cannot be silently narrowed by a future validation rule (Plan 115-03's D-13/D-14 are about to add exactly such rules).
- D-04 (locked): `HookConfig` gets a compile-time guard, not a field-merge — `merge_profiles`'s whole-value-replace hooks semantics are unchanged.
- D-05 (locked): the doc comment above `merge_custom_credential_def` is rewritten in place, not deleted, per CONTEXT.md's instruction.
- `pub(crate) fn default_inject_header()` (was private) — needed to make it reachable from `profile_cmd.rs`'s manifest-export consumer without duplicating the "Authorization" string a third time in this crate (the plan's "do not duplicate" instruction extends naturally to this newly-discovered consumer).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `profile_cmd.rs` consumers of `cred.inject_mode`/`cred.inject_header` needed updating for the Option<T> shape**
- **Found during:** Task 1 (`cargo build -p nono-sandbox-cli` after the field-shape change)
- **Issue:** `profile_cmd.rs` was not in the plan's `files_modified` list, but it has two independent consumers of `CustomCredentialDef.inject_mode`/`.inject_header`: the manifest-export path (`match cred.inject_mode { ... }`, `header: cred.inject_header.clone()`) and the profile-diff pretty-printer/JSON-diff functions (`old.inject_header != new.inject_header` feeding `theme::fg(&old.inject_header, ...)`, which requires `&str`). Both fail to compile against the new `Option<T>` field types.
- **Fix:** Manifest-export path resolves via `cred.inject_mode.clone().unwrap_or_default()` and a new `effective_inject_header` local resolved via `.unwrap_or_else(profile::default_inject_header)` (required promoting `default_inject_header` from private to `pub(crate)`). The plain-text diff printer resolves each side via `.as_deref().unwrap_or("<inherited>")` before formatting. The JSON-diff function needed no change — `serde_json::json!` and `serde_json::to_value` both serialize `Option<T>` correctly as-is.
- **Files modified:** `crates/nono-cli/src/profile_cmd.rs`, `crates/nono-cli/src/profile/mod.rs` (visibility change only)
- **Verification:** `cargo build --workspace --all-targets` exits 0; `cargo test -p nono-sandbox-cli --bin nono profile:: network_policy::` all pass.
- **Committed in:** `c0ee1286` (Task 1 commit)

**2. [Rule 1 - Bug] `InjectMode` does not derive `Copy`, so `.unwrap_or_default()` on a `&Option<InjectMode>` field read requires an explicit `.clone()` first**
- **Found during:** Task 1 (`cargo build` after adding `.unwrap_or_default()` calls in `validate_custom_credential` and `profile_cmd.rs`)
- **Issue:** `match cred.inject_mode.unwrap_or_default()` attempted to move out of a shared reference (`E0507`) — `InjectMode` derives `Clone` but not `Copy`.
- **Fix:** Added `.clone()` before `.unwrap_or_default()` at both call sites.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`, `crates/nono-cli/src/profile_cmd.rs`
- **Verification:** `cargo build --workspace --all-targets` exits 0.
- **Committed in:** `c0ee1286` (Task 1 commit)

**3. [Rule 1 - Bug] The Test A pipeline fixture's `credential_key` value initially contained hyphens, which `validate_custom_credential` rejects**
- **Found during:** Task 2 (first `cargo test` run of the new split tests)
- **Issue:** `"test-credential-key-base"` failed validation (`must contain only alphanumeric characters and underscores`), causing `finalize_profile(...).expect(...)` to panic — an artifact of writing the fixture value before checking the validator's character-class rule, not a defect in the D-01/D-03 fix itself.
- **Fix:** Renamed to `"test_credential_key_base"` (underscores) throughout Test A.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`
- **Verification:** Both split tests pass; `cargo fmt --all -- --check` clean.
- **Committed in:** `d104cbf7` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 Rule 3/blocking-compile, 1 Rule 1/test-fixture bug)
**Impact on plan:** All three were necessary consequences of the D-01 type change reaching further than the plan's `files_modified` list anticipated (`profile_cmd.rs`) or minor test-fixture correctness issues. No scope creep — no new features, no architectural changes.

## Issues Encountered

None beyond the deviations documented above.

## Bite-Proof Verification Transcripts

**D-01 (no-`Default` guard on `CustomCredentialDef`), Task 1:** Temporarily added `pub _bite_proof_probe: Option<bool>` to the struct. `cargo build -p nono-sandbox-cli --all-targets` failed with **38 `E0063` errors**, one per un-updated struct-literal construction site — matching RESEARCH.md's independently-derived blast-radius count exactly (2 production + 36 test fixtures). Removed the field; `cargo build -p nono-sandbox-cli --all-targets` returned to a clean `Finished` result.

**D-01 (`.or(base)` load-bearing check on `inject_mode`), Task 2:** Temporarily reverted `inject_mode: child.inject_mode.or(base.inject_mode)` to the unconditional `inject_mode: child.inject_mode`. Both `platform_overrides_custom_credential_collision_inherits_via_pipeline` and `platform_overrides_custom_credential_merge_exhaustive_over_every_field` failed with the exact expected assertion message (`left: None, right: Some(Header)`). Restored the `.or(base)` edit; both tests passed again.

**D-04 (`HookConfig` exhaustive-destructure guard), Task 3:** Temporarily added `pub timeout: Option<u64>` to `HookConfig`. `cargo build -p nono-sandbox-cli --all-targets` failed with **`E0027`** (pattern does not mention field `timeout`) at the guard itself, plus `E0063` at every un-updated `HookConfig { .. }` struct-literal site. Removed the field; build returned to a clean `Finished` result.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- DRAIN-01 (this plan's sole requirement) is structurally closed: the class of bug NEW-05 named (silent inject-field reset on override) is now unrepresentable at the type level, not merely tested for.
- `default_inject_header()` is now `pub(crate)` in `profile/mod.rs` — available to any future consumer in the `nono-cli` crate without duplicating the "Authorization" string a third time.
- Plans 115-02 through 115-06 (DRAIN-02 through DRAIN-06, per `115-CONTEXT.md`/`115-RESEARCH.md`) are unaffected by this plan's changes — RESEARCH.md's Q5 finding (`../nono-py`'s `RouteConfig::new` has zero references to `CustomCredentialDef`) is reconfirmed: no cross-repo follow-up is needed from this plan.
- Plan 115-03 (D-13/D-14 validation rejections for `aws_auth`/plain OAuth2 `client_credentials`) should note that `platform_overrides_custom_credential_merge_exhaustive_over_every_field` (Test B, this plan) deliberately populates `aws_auth` simultaneously with `credential_key`/`auth`/`spiffe` on its `base_def` — this is intentional (the test bypasses `validate_custom_credential` entirely) and must not be "fixed" by a future editor who notices the mutual-exclusion violation without reading the test's doc comment.

---
*Phase: 115-v3-6-carry-forward-drain*
*Completed: 2026-08-08*
