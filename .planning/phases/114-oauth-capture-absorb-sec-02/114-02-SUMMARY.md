---
phase: 114-oauth-capture-absorb-sec-02
plan: 02
subsystem: network-proxy
tags: [oauth-capture, sec-02, route-config, config-schema, capture-buffer]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-01's CaptureAuditContext / NetworkAuditDenialCategory::CaptureUnsupportedPath / NetworkAuditEvent.capture_context audit vocabulary"
provides:
  - "CaptureConfig, CaptureResponseField, CaptureResponseFieldKind declarative types in crates/nono-proxy/src/config.rs"
  - "RouteConfig.capture: Option<CaptureConfig> field, response-direction, composes with (not mutually exclusive with) credential_key/oauth2/aws_auth/spiffe"
  - "DEFAULT_CAPTURE_MAX_RESPONSE_BYTES (256 KiB) and CAPTURE_MAX_RESPONSE_BYTES_CEILING (1 MiB) constants (D-05)"
  - "Every exhaustive RouteConfig struct literal in the workspace compiles with an explicit capture: None stub"
affects: [114-05, 114-06, 114-07, 114-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Declarative wire-format field added to RouteConfig following the exact serde idiom of the existing spiffe field (#[serde(default, skip_serializing_if = \"Option::is_none\")])"
    - "Response-direction field documented in its own doc comment as deliberately NOT joining the request-direction mutual-exclusion group, to prevent a future reader from 'fixing' it by reflexive analogy"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/config.rs
    - crates/nono-cli/src/network_policy.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-proxy/src/credential.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/route.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono-proxy/tests/spiffe_integration.rs

key-decisions:
  - "capture: None is a deliberate stub at every construction site, including network_policy.rs's custom-credential route (CustomCredentialDef has no capture field until Plan 114-08's real conversion)"
  - "RouteConfig.capture field placed immediately after spiffe (mirrors the plan's interface spec), with a doc comment explicitly stating it is response-direction and therefore composes with, rather than joins, the request-direction mutual-exclusion group"

patterns-established:
  - "Fifth consecutive phase confirming: never trust a plan's files_modified list as the complete RouteConfig/NetworkAuditEvent break surface — always close with cargo build --workspace --all-targets and fix every compiler-reported site, per-file counts verified against endpoint_policy: None literal counts before batch sed edits"

requirements-completed: [SEC-02]

# Metrics
duration: ~20min
completed: 2026-08-06
---

# Phase 114 Plan 02: RouteConfig.capture Declarative Wire Format Summary

**Added `CaptureConfig`/`CaptureResponseField`/`CaptureResponseFieldKind` declarative types and `RouteConfig.capture: Option<CaptureConfig>` to `nono-proxy`'s config.rs, then closed 38 resulting `RouteConfig { ... }` exhaustive-literal compile breaks across 7 files workspace-wide (2 more than the plan's own predicted list) to bring `cargo build --workspace --all-targets` back to green.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2 planned, both completed with no deviation rules invoked beyond the plan's own explicit "compiler is ground truth" instruction
- **Files modified:** 8

## Accomplishments
- `CaptureConfig`, `CaptureResponseField`, `CaptureResponseFieldKind` exist in `crates/nono-proxy/src/config.rs`, matching the plan's exact field shape (D-07 declarative wire format for SEC-02 OAuth capture).
- `DEFAULT_CAPTURE_MAX_RESPONSE_BYTES` (256 KiB) and `CAPTURE_MAX_RESPONSE_BYTES_CEILING` (1 MiB) constants added per D-05, ready for Plan 114-08's profile-validation enforcement and Plan 114-05's buffer-read-time enforcement.
- `RouteConfig.capture: Option<CaptureConfig>` field added immediately after `spiffe`, with a doc comment recording the deliberate design decision that it is response-direction and composes with (does not join the mutual-exclusion group of) `credential_key`/`oauth2`/`aws_auth`/`spiffe`.
- All 38 `RouteConfig { ... }` exhaustive struct literals across the workspace now compile with an explicit `capture: None,` stub — `cargo build --workspace --all-targets` exits 0.
- `cargo fmt --all -- --check` is clean.
- `cargo test -p nono-sandbox-proxy --lib` (245 tests) and `cargo test -p nono-sandbox-cli --bin nono network_policy::` (47 tests) both pass with zero regressions.

## Task Commits

Each task was committed atomically:

1. **Task 1: Declarative CaptureConfig types + RouteConfig.capture field** - `e2754cf1` (feat)
2. **Task 2: Close every RouteConfig exhaustive-literal break; green workspace build** - `c8007112` (fix)

## Files Created/Modified
- `crates/nono-proxy/src/config.rs` - `CaptureConfig`/`CaptureResponseField`/`CaptureResponseFieldKind` types, `RouteConfig.capture` field, `DEFAULT_CAPTURE_MAX_RESPONSE_BYTES`/`CAPTURE_MAX_RESPONSE_BYTES_CEILING` constants
- `crates/nono-cli/src/network_policy.rs` - `capture: None,` added to 3 `RouteConfig` literals: the custom-credential route, the built-in-credential route, and `partition_allow_domain`'s endpoint-route literal (the third site the plan's own interface spec did not name — found via compiler, not prediction)
- `crates/nono-cli/src/proxy_runtime.rs` - `capture: None,` added to the `spiffe_route()` test helper literal
- `crates/nono-proxy/src/credential.rs` - `capture: None,` added to 7 test `RouteConfig` literals
- `crates/nono-proxy/src/reverse.rs` - `capture: None,` added to 1 test `RouteConfig` literal
- `crates/nono-proxy/src/route.rs` - `capture: None,` added to 11 test `RouteConfig` literals
- `crates/nono-proxy/src/server.rs` - `capture: None,` added to 14 test `RouteConfig` literals
- `crates/nono-proxy/tests/spiffe_integration.rs` - `capture: None,` added to the `make_jwt_route()` integration-test helper literal

## Decisions Made
- **`capture: None` everywhere, no real conversion yet:** per the plan's explicit scope, `CustomCredentialDef` has no `capture` field until Plan 114-08 — `network_policy.rs`'s custom-credential route sets `capture: None,` as a deliberate stub, documented inline with a comment referencing Plan 114-08.
- **Doc-comment placement of the mutual-exclusion caveat:** recorded directly on the `capture` field in `RouteConfig` (not just in this SUMMARY) so a future reader editing `config.rs` sees the rationale in context, per the plan's explicit instruction ("record it in the doc comment so a future reader does not 'fix' it into the mutual-exclusion block by reflexive analogy").
- **Batch-fix verification method:** for each file, counted `grep -c "endpoint_policy: None,"` (the field immediately preceding `capture`'s intended insertion point in most literals, or `aws_auth:` for literals with `spiffe` first) and cross-checked that count against the compiler's per-file `missing field \`capture\`` error count before applying a batch `sed` insertion — this caught that `network_policy.rs` had 3 real sites, not the 2 the plan's `<interfaces>` block enumerated.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `network_policy.rs`'s third RouteConfig literal (`partition_allow_domain`) needed `capture: None,` too — not named in the plan's interfaces block**
- **Found during:** Task 2, `cargo build --workspace --all-targets` compiler error at `crates\nono-cli\src\network_policy.rs:454:42`
- **Issue:** The plan's `<interfaces>` block named exactly two production `RouteConfig` literals in `network_policy.rs` (the custom-credential and built-in-credential construction sites in `resolve_credentials`/similar). A third exhaustive literal exists in `partition_allow_domain` (the allow-domain-with-endpoints route builder), which the plan's own text anticipated as a possibility ("do not trust this plan's file list as exhaustive, the compiler is the source of truth") but did not specifically call out for this file.
- **Fix:** Added `capture: None,` immediately after the literal's existing `spiffe: None, // allow-domain-derived routes never carry credential auth` line.
- **Files modified:** `crates/nono-cli/src/network_policy.rs`
- **Verification:** `cargo build --workspace --all-targets` exits 0; `cargo test -p nono-sandbox-cli --bin nono network_policy::` (47 tests) passes.
- **Committed in:** `c8007112` (Task 2 commit)

**2. [Rule 3 - Blocking] 33 additional test-target `RouteConfig` literals in nono-proxy's own test modules needed `capture: None,`**
- **Found during:** Task 2, `cargo build --workspace --all-targets` (the plan's own `<interfaces>` block explicitly flagged this as likely: "There may be additional exhaustive RouteConfig { ... } literals inside nono-proxy's own test modules")
- **Issue:** `credential.rs` (7 sites), `reverse.rs` (1 site), `route.rs` (11 sites), `server.rs` (14 sites), and `tests/spiffe_integration.rs` (1 site) all construct `RouteConfig` exhaustively in `#[cfg(test)]` code and integration tests, none of which the plan's `files_modified` frontmatter listed (only `config.rs`, `network_policy.rs`, `proxy_runtime.rs` were listed there).
- **Fix:** Added `capture: None,` at every compiler-reported site, verified per-file against `grep -c "endpoint_policy: None,"` counts before applying batch edits.
- **Files modified:** `crates/nono-proxy/src/credential.rs`, `crates/nono-proxy/src/reverse.rs`, `crates/nono-proxy/src/route.rs`, `crates/nono-proxy/src/server.rs`, `crates/nono-proxy/tests/spiffe_integration.rs`
- **Verification:** `cargo build --workspace --all-targets` exits 0; `cargo test -p nono-sandbox-proxy --lib` (245 tests) passes with zero regressions.
- **Committed in:** `c8007112` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 - blocking, both explicitly anticipated by the plan's own `<interfaces>` block instruction to treat the compiler as ground truth rather than the plan's predicted file list)
**Impact on plan:** No scope creep — every fix is the identical `capture: None,` stub pattern the plan's Task 2 `<action>` prescribed, applied to sites the compiler found rather than sites the plan predicted. This is the fifth consecutive phase in this project where a new `RouteConfig`/`NetworkAuditEvent` field broke more sites than static prediction found (see MEMORY.md `feedback_disposition_confidence_needs_symbol_level.md` and D-14 in `114-CONTEXT.md`), reinforcing that only a full-workspace build closes this class of break.

## Issues Encountered
None beyond the deviations documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
`CaptureConfig`/`CaptureResponseField`/`CaptureResponseFieldKind` and `RouteConfig.capture` are in place, compiling cleanly workspace-wide, for Plan 114-08 (real `CustomCredentialDef` conversion) and Plans 114-05/114-06 (buffer-and-rewrite enforcement logic consuming `CaptureConfig.response_fields`/`request_nonce_fields`/`max_response_bytes`) to build against without a second `RouteConfig` field edit. `DEFAULT_CAPTURE_MAX_RESPONSE_BYTES`/`CAPTURE_MAX_RESPONSE_BYTES_CEILING` are ready for D-05's fail-closed enforcement in later plans. No blockers. Note for `114-10` (nono-py/nono-ts binding fix, D-14): this plan's `RouteConfig.capture` field is the trigger for that sibling-repo break — `../nono-py/src/proxy.rs` was NOT touched in this plan per the executor's explicit instruction.

---
*Phase: 114-oauth-capture-absorb-sec-02*
*Completed: 2026-08-06*

## Self-Check: PASSED

All modified files verified present on disk (`crates/nono-proxy/src/config.rs`,
`crates/nono-cli/src/network_policy.rs`, this SUMMARY.md); all 3 commit hashes
(`e2754cf1`, `c8007112`, `01ee45f`) verified present in git log.
