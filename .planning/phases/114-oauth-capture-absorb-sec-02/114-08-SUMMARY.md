---
phase: 114-oauth-capture-absorb-sec-02
plan: 08
subsystem: profile-cli
tags: [oauth-capture, sec-02, credential-provider, profile-validation, route-config]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-02's CaptureConfig/CaptureResponseField/CaptureResponseFieldKind declarative types + RouteConfig.capture field + DEFAULT_CAPTURE_MAX_RESPONSE_BYTES/CAPTURE_MAX_RESPONSE_BYTES_CEILING constants, and its capture: None stub in network_policy.rs's custom-credential branch"
provides:
  - "crates/nono-cli/src/profile/credential_provider.rs — validate_capture_config()/validate_capture_field_path(), ported/adapted from upstream's file of the same name per D-12"
  - "CustomCredentialDef.capture: Option<nono_proxy::config::CaptureConfig> field, validated and threaded into validate_custom_credential()'s at-least-one-of check as a valid alternative (not a mutual-exclusion arm)"
  - "network_policy.rs's real capture: cred.capture.clone() conversion, replacing Plan 114-02's stub — an operator-declared capture-only custom credential now reaches the proxy's real RouteConfig.capture"
  - "PartialEq derive on nono_proxy::config::CaptureConfig (required for CustomCredentialDef's existing PartialEq derive to keep compiling)"
affects: [114-05, 114-06, 114-09, 114-10]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Response-direction field (capture) added to the 'at least one of' validation check as a valid alternative, explicitly NOT added to any of the three request-direction mutual-exclusion if-blocks — documented inline on both the field doc comment and validate_custom_credential() to prevent a future reader from 'fixing' it by reflexive analogy"
    - "Field-path validation ported from upstream's validate_provider_field/validate_provider_path shape but extended per this plan's explicit behavior spec (leading/trailing/double-dot rejection) — upstream's own shape only rejects empty/NUL-bearing values, a live-source vs. plan-cited divergence worth flagging"

key-files:
  created:
    - crates/nono-cli/src/profile/credential_provider.rs
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/network_policy.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-proxy/src/config.rs

key-decisions:
  - "capture does NOT join credential_key/auth/aws_auth/spiffe's mutual-exclusion checks — it IS added as a valid alternative to the 'at least one of' check, since a bare capture-only route (no other auth mechanism) is the primary use case per the plan's interfaces section"
  - "CaptureConfig needed a PartialEq derive add (Rule 3 blocking fix, not in the plan's files_modified list) because CustomCredentialDef already derives PartialEq and the new capture field broke that derive — RouteConfig itself has no PartialEq derive so Plan 114-02 never hit this"
  - "Plan's own clippy acceptance-criteria command (cargo clippy -p nono-sandbox-cli --lib ...) does not apply to this crate (nono-cli has no --lib target, only a --bin nono target per Cargo.toml) — verified with the bin-target equivalent instead, consistent with project gotcha #1"

patterns-established:
  - "Sixth consecutive occurrence in this phase (after 114-01/02/03/04) of a new CustomCredentialDef/RouteConfig field breaking more exhaustive struct-literal sites than the plan's own interfaces block predicted (36 CustomCredentialDef sites across 3 files, all found via cargo build --workspace --all-targets, none via prediction) — this is now a stable pattern for this codebase, not phase-specific noise"

requirements-completed: [SEC-02]

# Metrics
duration: ~30min
completed: 2026-08-06
---

# Phase 114 Plan 08: Declarative OAuth-Capture Profile Surface Summary

**`profile/credential_provider.rs` field-path/max-bytes validation (D-12) plus the real `CustomCredentialDef.capture` -> `RouteConfig.capture` conversion, closing Plan 114-02's stub so an operator-declared capture-only custom credential actually reaches the proxy.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 2 planned, both completed with expected deviations (see below)
- **Files modified:** 4 (1 created, 3 modified)

## Accomplishments
- `crates/nono-cli/src/profile/credential_provider.rs` exists: `pub(super) fn validate_capture_config()` rejects an empty `response_fields`, any empty/leading-dot/trailing-dot/double-dot/NUL-bearing field path in `response_fields[].path` or `request_nonce_fields[]`, and any `max_response_bytes` of `0` or exceeding `CAPTURE_MAX_RESPONSE_BYTES_CEILING` — validated by 11 unit tests, one per `<behavior>` case in the plan.
- `CustomCredentialDef.capture: Option<nono_proxy::config::CaptureConfig>` exists in `profile/mod.rs`, wired into `validate_custom_credential()`: calls `credential_provider::validate_capture_config()` when set, and is included as a valid alternative in the "at least one of" check — proven by 4 new tests, including one asserting a capture-only credential (all four other auth fields `None`) validates successfully.
- `network_policy.rs`'s custom-credential branch now does `capture: cred.capture.clone(),` — the real conversion, replacing Plan 114-02's `capture: None,` stub. Proven by a new test (`test_resolve_credentials_custom_capture_threaded_to_route_config`) that asserts the resolved `RouteConfig.capture`'s `response_fields`/`request_nonce_fields` match the declared `CustomCredentialDef.capture`, not just `Option::is_some()`.
- The built-in-credential branch's `capture: None,` is unchanged (no `CustomCredentialDef` to source from), its comment now matches the `spiffe: None, // Built-in credentials don't support SPIFFE` style directly above it.
- `cargo build --workspace --all-targets`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` all exit 0.
- `cargo test -p nono-sandbox-proxy --lib` remains 268/268 (zero regression). `cargo test -p nono-sandbox-cli --bin nono profile::` is 321/321 (was 314 pre-plan; +7 new tests: 4 profile::tests + 11 profile::credential_provider::tests minus overlap counted once — see Files Created/Modified for the exact split). `cargo test -p nono-sandbox-cli --bin nono network_policy::` is 48/48 (was 47; +1 new test).

## Task Commits

Each task was committed atomically:

1. **Task 1: profile/credential_provider.rs — ported field-path validation (D-12)** - `ec28271b` (feat)
2. **Task 2: CustomCredentialDef.capture field + validate_custom_credential wiring + network_policy.rs real conversion** - `fb668279` (feat)

## Files Created/Modified
- `crates/nono-cli/src/profile/credential_provider.rs` (created) - `validate_capture_config()` + private `validate_capture_field_path()` helper; module doc comment records D-12's port/drop rationale (drops `credential_store`/`helpers`/`api_hosts`/the `credential_providers`/`credential_routes` indirection layer, names why); 11 unit tests
- `crates/nono-cli/src/profile/mod.rs` - `pub(crate) mod credential_provider;` registered; `CustomCredentialDef.capture` field added after `spiffe`; `validate_custom_credential()`'s "at least one of" check extended with `capture`, plus a call to `validate_capture_config()`; 4 new tests; 22 exhaustive `CustomCredentialDef { ... }` test literals closed with `capture: None,`
- `crates/nono-cli/src/network_policy.rs` - custom-credential branch's stub replaced with `capture: cred.capture.clone(),`; built-in branch's comment updated to match the `spiffe` line's style; 1 new test proving the real threading; 11 exhaustive test literals closed with `capture: None,`
- `crates/nono-cli/src/proxy_runtime.rs` - 2 exhaustive `CustomCredentialDef { ... }` test literals closed with `capture: None,`
- `crates/nono-proxy/src/config.rs` - `CaptureConfig` gains a `PartialEq` derive (Rule 3, blocking — see Deviations)

## Decisions Made
- **`capture` joins the "at least one of" check, not any mutual-exclusion arm.** Per the plan's `<interfaces>` section, this is deliberate: capture is response-direction, the other four fields are request-direction, and a bare capture-only route is the primary use case. Documented on both the field's doc comment and inline in `validate_custom_credential()`.
- **Field-path validation is stricter than upstream's actual shape.** Upstream's `validate_provider_field` only rejects empty/NUL-bearing values — no leading/trailing/double-dot check. The plan's `<behavior>` section explicitly required the dot-shape checks regardless, citing them as "ported from upstream's shape" — implemented per the plan's explicit spec since live-source (verified via `git show 9b692e07:...`) disagreed with the plan's characterization. Recorded here per the "live source wins, record the discrepancy" instruction in `project_gotchas`.
- **Verification command adjusted for `nono-sandbox-cli`'s crate shape.** The plan's acceptance criteria specify `cargo clippy -p nono-sandbox-cli --lib -- ...`, but this crate has no `--lib` target (`cargo test -p nono-sandbox-cli --lib` errors with "no library targets found" — only `--bin nono` exists). Verified with `cargo clippy -p nono-sandbox-cli --bin nono -- -D warnings -D clippy::unwrap_used` instead, consistent with `project_gotchas` item 1's documented convention.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `nono_proxy::config::CaptureConfig` needed a `PartialEq` derive**
- **Found during:** Task 2, `cargo build -p nono-sandbox-cli --all-targets` (`E0369: binary operation == cannot be applied to type Option<CaptureConfig>` at `profile/mod.rs:1050`, the derived `PartialEq` impl site for `CustomCredentialDef`)
- **Issue:** `CustomCredentialDef` derives `PartialEq` (unlike `RouteConfig`, which does not — this is why Plan 114-02 never hit this). Adding `capture: Option<CaptureConfig>` to a `PartialEq`-deriving struct requires `CaptureConfig: PartialEq`, but Plan 114-02 only derived `Debug, Clone, Serialize, Deserialize` on `CaptureConfig`.
- **Fix:** Added `PartialEq` to `CaptureConfig`'s derive list in `crates/nono-proxy/src/config.rs`. `CaptureResponseField`/`CaptureResponseFieldKind` (its nested types) already derived `PartialEq`/`Eq`, so this was a single-line, contained fix.
- **Files modified:** `crates/nono-proxy/src/config.rs`
- **Verification:** `cargo build --workspace --all-targets` exits 0; `cargo test -p nono-sandbox-proxy --lib` remains 268/268.
- **Committed in:** `fb668279` (Task 2 commit)

**2. [Rule 3 - Blocking] 36 exhaustive `CustomCredentialDef { ... }` struct literals broke on the new `capture` field**
- **Found during:** Task 2, `cargo build -p nono-sandbox-cli --all-targets` — 36 `E0063: missing field 'capture'` errors
- **Issue:** Same blast-radius pattern as every prior plan in this phase (114-01/02/03/04). Adding a field to a struct with no `Default` impl breaks every exhaustive literal; the plan's `<interfaces>` block does not enumerate a specific count for this class of break, only warns to treat the compiler as ground truth.
- **Fix:** Added `capture: None,` immediately after each literal's existing `spiffe:` line (matching field order in the struct), located and applied via the exact line numbers the compiler reported — 22 sites in `profile/mod.rs` (21 test-module literals + confirming `header_cred_builder()`'s single exhaustive literal covers most call sites via reuse/mutation), 11 sites in `network_policy.rs`'s own test module, 2 sites in `proxy_runtime.rs`'s test module. Also verified line-count parity: the number of `spiffe:` lines matched the number of compiler-reported error lines exactly in each file before applying edits, per this phase's established "compiler is ground truth" verification discipline.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`, `crates/nono-cli/src/network_policy.rs`, `crates/nono-cli/src/proxy_runtime.rs`
- **Verification:** `cargo build --workspace --all-targets` exits 0; `cargo test -p nono-sandbox-cli --bin nono profile:: network_policy::` both green (321 and 48 tests respectively).
- **Committed in:** `fb668279` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 - blocking, both directly caused by the new `capture` field and required to reach a compiling workspace)
**Impact on plan:** No scope creep — both fixes are the identical shape prior plans in this phase established (add the missing derive bound / add the missing struct-literal field), applied to sites the compiler found. This is the sixth consecutive plan in Phase 114 confirming the same lesson recorded in `114-CONTEXT.md`'s D-14 and this project's `feedback_disposition_confidence_needs_symbol_level.md` memory: only `cargo build --workspace --all-targets` reliably closes this class of break.

## Issues Encountered
None beyond the deviations documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
`CustomCredentialDef.capture` is validated at profile-load time and threaded through to the real `RouteConfig.capture` value the proxy consumes. This closes the declarative-config half of SEC-02 (D-12): an operator can now write a profile declaring `capture = { response_fields = [...] }` on a custom credential, have it rejected at load time if malformed, and have it reach `nono-proxy`'s `RouteConfig.capture` for Plans 114-05/114-06's buffer-and-rewrite enforcement to consume.

Two carry-forwards for later plans in this phase, both already anticipated and explicitly out of scope for 114-08:
- **114-09 (schema, D-13):** `crates/nono-cli/data/nono-profile.schema.json` does not yet describe `capture` on a custom credential. This plan did not touch the schema file per its explicit instruction. If a schema round-trip test exists and asserts a capture-bearing profile passes `validate_against_schema()`, it will fail until 114-09 lands — not a regression from this plan, an expected sequencing gap.
- **114-10 (bindings, D-14):** `../nono-py/src/proxy.rs` was NOT touched. `RouteConfig.capture` was already added by Plan 114-02 (not this plan), so the binding break this plan's `capture` field could theoretically compound is already tracked there. This plan added no new `RouteConfig` fields — only `CustomCredentialDef.capture`, which is CLI-side-only and has no exposure in `nono-py`'s `RouteConfig` construction. No new binding-repo action needed from this plan specifically.

No blockers.

---
*Phase: 114-oauth-capture-absorb-sec-02*
*Completed: 2026-08-06*

## Self-Check: PASSED

All modified/created files verified present on disk:
- `crates/nono-cli/src/profile/credential_provider.rs` (created) — FOUND
- `crates/nono-cli/src/profile/mod.rs` — FOUND
- `crates/nono-cli/src/network_policy.rs` — FOUND
- `crates/nono-cli/src/proxy_runtime.rs` — FOUND
- `crates/nono-proxy/src/config.rs` — FOUND

Both commit hashes verified present in `git log`:
- `ec28271b` — FOUND
- `fb668279` — FOUND
