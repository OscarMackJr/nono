---
phase: 114-oauth-capture-absorb-sec-02
plan: 09
subsystem: profile-cli
tags: [oauth-capture, sec-02, json-schema, profile-validation, d-13]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-02's CaptureConfig/CaptureResponseField/CaptureResponseFieldKind types in crates/nono-proxy/src/config.rs, and Plan 114-08's CustomCredentialDef.capture field + validate_capture_config()/validate_capture_field_path() in crates/nono-cli/src/profile/{mod.rs,credential_provider.rs}"
provides:
  - "$defs.CaptureConfig and $defs.CaptureResponseField in crates/nono-cli/data/nono-profile.schema.json, both strict (additionalProperties: false)"
  - "capture property on $defs.CustomCredentialDef, oneOf $defs/CaptureConfig or null, documenting what was and was not ported (D-13)"
  - "test_schema_validates_capture_custom_credential — round-trip proof a realistic capture-provider profile deserializes and passes validate_against_schema()"
  - "test_schema_rejects_capture_empty_response_fields and test_schema_rejects_capture_invalid_response_field_kind — negative-case schema coverage"
affects: [114-10]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Schema-level minItems: 1 as defense-in-depth alongside an existing Rust-level empty-array check (validate_capture_config), rather than marking the #[serde(default)] field 'required' — avoids the required-on-optional-field trap named in project_gotchas item 5 while still rejecting the present-but-empty-array shape at the schema layer"

key-files:
  created: []
  modified:
    - crates/nono-cli/data/nono-profile.schema.json
    - crates/nono-cli/src/profile/mod.rs

key-decisions:
  - "response_fields/request_nonce_fields/max_response_bytes are NOT in $defs.CaptureConfig's required list — all three carry #[serde(default)] in the live Rust struct (crates/nono-proxy/src/config.rs), so marking any of them required would hard-reject valid omitted-property JSON, the exact defect class Phase 113 shipped and this plan's project_gotchas item 5 calls out by name"
  - "Chose minItems: 1 (schema-level, present-but-empty) over marking response_fields required (schema-level, omitted-entirely) as the schema's enforcement layer for D-13's non-empty-response_fields behavior spec — the omitted-entirely case is left to the existing Rust-level validate_capture_config() check (already covered by 114-08's test suite), keeping the two enforcement layers complementary rather than duplicating the same check twice"
  - "capture's schema description explicitly names what was NOT ported (disk persistence, credential-store detection, login-flow helpers) and cites ADR-114/D-13, mirroring the plan's instruction to make the scope limit legible to a future schema reader, not just to this SUMMARY"

patterns-established: []

requirements-completed: [SEC-02]

# Metrics
duration: ~25min
completed: 2026-08-07
---

# Phase 114 Plan 09: Declarative OAuth-Capture Profile Schema Summary

**Hand-ported `$defs.CaptureConfig`/`$defs.CaptureResponseField` into the fork-only, strict `nono-profile.schema.json` — deliberately excluding upstream's unported persistence/interception schema surface (D-13) — with a round-trip test plus two negative cases.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 1 planned, completed as written (no deviations)
- **Files modified:** 2

## Accomplishments
- `$defs.CaptureConfig` added to `crates/nono-cli/data/nono-profile.schema.json`: object, `additionalProperties: false`, no `required` list (all three fields carry `#[serde(default)]` in the live Rust struct), properties `response_fields` (array of `$defs/CaptureResponseField`, `minItems: 1`, default `[]`), `request_nonce_fields` (array of strings, default `[]`), `max_response_bytes` (integer >= 1, or null).
- `$defs.CaptureResponseField` added: object, `additionalProperties: false`, `required: ["path"]` (matches the Rust struct's non-`#[serde(default)]` `path: String` field), properties `path` (string) and `kind` (string enum `["opaque", "jwt"]`, default `"opaque"`).
- `capture` property added to `$defs.CustomCredentialDef` directly after `spiffe`, `oneOf` referencing `$defs/CaptureConfig` or `null`, with a description naming D-13/ADR-114 and explicitly listing what was NOT ported (disk persistence, credential-store detection, login-flow helpers).
- `test_schema_validates_capture_custom_credential` added to the same `mod tests` block housing `test_schema_validates_spiffe_custom_credential` (the block whose local `validate_against_schema` helper is at line ~7362, confirmed by symbol-level grep, not by any prior plan's cited line numbers) — proves a `custom_credentials.<name>.capture` block with one `response_fields` entry (`path: "access_token"`, `kind: "opaque"`) passes `validate_against_schema()`.
- Two negative-case tests added directly after it: `test_schema_rejects_capture_empty_response_fields` (explicit `"response_fields": []` fails schema via `minItems: 1`) and `test_schema_rejects_capture_invalid_response_field_kind` (an unrecognized `kind` value fails schema via enum enforcement).
- `cargo build --workspace --all-targets`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` all exit 0.
- `cargo test -p nono-sandbox-cli --bin nono profile::` is 324/324 (zero regression to the existing SPIFFE schema test or any other profile:: test); all three new tests individually verified green.

## Task Commits

Each task was committed atomically:

1. **Task 1: CaptureConfig $defs entry + capture property + round-trip test** - `4f4d4428` (feat)

## Files Created/Modified
- `crates/nono-cli/data/nono-profile.schema.json` - `$defs.CaptureConfig`, `$defs.CaptureResponseField`, and `CustomCredentialDef.capture` property added; both new `$defs` entries are `additionalProperties: false`
- `crates/nono-cli/src/profile/mod.rs` - `test_schema_validates_capture_custom_credential` plus two negative-case tests added to the `mod tests` block co-located with `test_schema_validates_spiffe_custom_credential`

## Decisions Made
See `key-decisions` in frontmatter — summarized: neither `required` on the `#[serde(default)]` `CaptureConfig` fields nor duplicate enforcement of the omitted-vs-empty `response_fields` case; `minItems: 1` for the present-but-empty case at the schema layer, existing Rust-level `validate_capture_config()` (114-08) for the omitted-entirely case.

## Deviations from Plan

None - plan executed exactly as written. Read the live `crates/nono-proxy/src/config.rs` `CaptureConfig`/`CaptureResponseField`/`CaptureResponseFieldKind` definitions and the live `crates/nono-cli/src/profile/mod.rs` `CustomCredentialDef.capture` field (both per this plan's `<read_first>` instruction) before writing any schema JSON; both matched the plan's paraphrase in this instance (unlike the Phase 110/113 mismatches this plan's gotchas warned about), so no corrective schema edits were needed beyond the planned scope.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
The fork-only profile schema now describes exactly what Plans 114-02/114-08 implemented for declarative OAuth capture — no more, no less, per D-13. An operator-authored profile declaring `custom_credentials.<name>.capture` is now validated at the schema layer (structure, `additionalProperties: false`, `kind` enum, `minItems: 1` on `response_fields`) in addition to the existing Rust-level `validate_capture_config()` checks (field-path shape, max-bytes ceiling, non-empty `response_fields` when omitted).

114-10 (bindings, D-14) remains the next carry-forward: `../nono-py/src/proxy.rs` constructs `RouteConfig` with exhaustive struct literals and has not yet been updated for the `capture` field Plan 114-02 added to `RouteConfig` (this plan touched only the CLI-side schema and `CustomCredentialDef`, not `RouteConfig` itself, and made no `nono-py`/`nono-ts` changes — consistent with this plan's declared `files_modified` scope).

No blockers.

---
*Phase: 114-oauth-capture-absorb-sec-02*
*Completed: 2026-08-07*

## Self-Check: PASSED

All modified files verified present on disk:
- `crates/nono-cli/data/nono-profile.schema.json` — FOUND
- `crates/nono-cli/src/profile/mod.rs` — FOUND
- `.planning/phases/114-oauth-capture-absorb-sec-02/114-09-SUMMARY.md` — FOUND

Both commit hashes verified present in `git log`:
- `4f4d4428` — FOUND
- `3e5bb535` — FOUND
