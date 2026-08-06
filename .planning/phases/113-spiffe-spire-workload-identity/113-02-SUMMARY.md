---
phase: 113-spiffe-spire-workload-identity
plan: 02
subsystem: proxy-network-security
tags: [spiffe, spire, workload-identity, oauth2, jwt-bearer, jsonschema, rust, nono-proxy, nono-cli]

# Dependency graph
requires:
  - phase: 113-spiffe-spire-workload-identity
    provides: "Plan 113-01's foundational SpiffeAuditContext vocabulary (core nono crate) and nono-proxy/src/spiffe.rs (SpiffeJwtSource) + auth.rs (ManagedUpstreamAuth) — this plan does not import them yet, but the phase's dead_code RED state on those symbols is inherited unchanged, not caused, here"
provides:
  - "nono-proxy/src/config.rs: RouteConfig.spiffe: Option<SpiffeAuthConfig>, SpiffeAuthConfig::Jwt, ClientAssertionConfig::SpiffeJwt, OAuth2Config.client_id/client_secret now #[serde(default)], OAuth2Config.client_assertion/extra_params, test_spiffe_jwt_config_roundtrip, test_spiffe_absent_by_default"
  - "nono-cli/src/profile/mod.rs: CustomCredentialDef.spiffe, spiffe mutual-exclusion + at-least-one validation, validate_header_name() helper (extracted, reused by spiffe), validate_oauth2_auth()'s client_assertion branch"
  - "nono-cli/data/nono-profile.schema.json: spiffe property + $defs/SpiffeAuthConfig + $defs/ClientAssertionConfig + OAuth2Config's required array narrowed to [token_url] + client_assertion/extra_params properties (fork-only strict-schema companion file, hand-edited per the Phase 110 lesson)"
  - "nono-cli/src/proxy_runtime.rs: enforce_spiffe_socket_isolation() — denies the sandboxed child direct access to every configured SPIRE Workload API socket (RouteConfig.spiffe AND oauth2.client_assertion), wired into start_proxy_runtime() before the proxy starts"
  - "Every pre-existing exhaustive RouteConfig{..}/CustomCredentialDef{..}/OAuth2Config{..} literal in the workspace (nono-proxy: config.rs/credential.rs/reverse.rs/route.rs/server.rs; nono-cli: network_policy.rs/profile/mod.rs/proxy_runtime.rs — ~44 sites total) updated so cargo build --workspace --all-targets stays green at this wave boundary"
affects: [113-03, 113-04, 113-05, 113-06, 113-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "RouteConfig.spiffe placed after aws_auth, before endpoint_policy, matching upstream's relative field placement and the fork's existing mutual-exclusion field cluster"
    - "validate_header_name() extracted as a reusable helper so credential_key-based and spiffe-based header validation share one implementation instead of diverging"
    - "enforce_spiffe_socket_isolation() follows policy.rs::add_deny_access_rules's existing Seatbelt Unix-socket-deny idiom (deny network-outbound, not a file rule, since connect(2) on AF_UNIX is enforced as network-outbound by Seatbelt) rather than inventing a new pattern"
    - "Compiler-driven build-fix-build loop (not a static line-number list) for the workspace-wide struct-literal sweep — matches 113-01's own established approach and stayed reliable across ~44 sites despite line-number drift between grep passes"

key-files:
  created: []
  modified:
    - crates/nono-proxy/src/config.rs
    - crates/nono-proxy/src/credential.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/route.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono-cli/src/network_policy.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - crates/nono-cli/src/proxy_runtime.rs

key-decisions:
  - "Disposition amendment: the let-chain landmine RESEARCH.md/PATTERNS.md flagged for proxy_runtime.rs's 'host-collection loop' does not exist in the current fork tree — grep -c '&& let' already returns 0 before this plan touched the file. parse_allow_domain_arg already uses nested if-let. No rewrite was needed; verified, not assumed."
  - "Disposition amendment: the plan's read_first claim that proxy_runtime.rs already has add_platform_rule/unix_socket_capabilities call sites is inaccurate — grep confirms zero such call sites in this file before this plan. Both are CapabilitySet methods called directly in the new enforce_spiffe_socket_isolation(), following policy.rs::add_deny_access_rules's macOS deny-rule idiom as the design reference instead."
  - "enforce_spiffe_socket_isolation() collects workload_api_socket paths from BOTH RouteConfig.spiffe and oauth2.client_assertion (SpiffeJwt) — broader than the plan's literal 'extracts each SpiffeAuthConfig::Jwt { workload_api_socket, .. } path' text, which named only the direct-JWT shape. Both configurations cause the proxy to reach the same SPIRE Workload API socket on the sandboxed child's behalf, so isolating only one would leave a real, reachable gap (Rule 2 — missing critical functionality on a security control named in this plan's own threat model, T-113-04)."
  - "validate_oauth2_auth()'s client_assertion branch returns early (Ok(())) after validating the assertion's own subfields, rather than the plan's if/else prose shape — functionally identical, avoids duplicating the empty-check logic under a nested else."

patterns-established:
  - "SPIFFE/client_assertion schema types added to the fork-only nono-profile.schema.json using the existing aws_auth/$defs/AwsAuthConfig copy-pattern verbatim (additionalProperties: false, oneOf-with-null optionals, required arrays for the two mandatory fields) — no new schema idiom introduced"

requirements-completed: [NET-02]

# Metrics
duration: ~35min
completed: 2026-08-06
---

# Phase 113 Plan 02: SPIFFE Schema Types + SC2 Profile Validation + Socket Isolation Summary

**RouteConfig/CustomCredentialDef gain a `spiffe` field and OAuth2Config gains RFC 7523 `client_assertion`, both schema-validated (fork-only `nono-profile.schema.json` hand-edited per the Phase 110 lesson) and mutually-exclusion-checked at profile-load time, plus a new `enforce_spiffe_socket_isolation()` that denies the sandboxed child direct access to the SPIRE Workload API socket — the config-only half of the SPIFFE absorb that every later plan (`credential.rs`, `route.rs`, `reverse.rs`, `server.rs`) reads at runtime.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-08-06T11:40:00Z (approx)
- **Completed:** 2026-08-06T12:20:00Z
- **Tasks:** 3 completed
- **Files modified:** 9

## Accomplishments

- `RouteConfig.spiffe: Option<SpiffeAuthConfig>` and `SpiffeAuthConfig::Jwt`/`ClientAssertionConfig::SpiffeJwt` landed in `nono-proxy/src/config.rs`; `OAuth2Config.client_id`/`client_secret` gained `#[serde(default)]` and the struct gained `client_assertion`/`extra_params` fields. `test_spiffe_jwt_config_roundtrip` and `test_spiffe_absent_by_default` ported verbatim into `config.rs`'s `mod tests` — both pass, proving the serde round-trip, not just build success.
- Every pre-existing exhaustive `RouteConfig`/`OAuth2Config` struct literal in `nono-proxy` (config.rs's own tests, credential.rs x3, reverse.rs x1, route.rs x8, server.rs x12) fixed via a compiler-driven build-fix-build loop. `cargo test -p nono-sandbox-proxy --lib`: 229 passed (227 baseline + 2 new), zero new failures.
- `CustomCredentialDef.spiffe` landed in `nono-cli/src/profile/mod.rs`, matching `RouteConfig`'s shape. The two real production `RouteConfig` construction sites in `network_policy.rs::resolve_credentials` propagate correctly — the custom-credential branch sets `spiffe: cred.spiffe.clone()` (a profile-declared spiffe route actually reaches the proxy, not a stub), the built-in-credential branch and `partition_allow_domain`'s route both set `spiffe: None`. Every other construction site (test fixtures across `network_policy.rs`, `profile/mod.rs`, `proxy_runtime.rs`) fixed the same compiler-driven way (~24 sites). `cargo test -p nono-sandbox-cli --bin nono -- network_policy:: profile::`: 342 passed, zero new failures.
- SC2's full profile-configurability surface landed: `validate_custom_credential()` gained spiffe mutual-exclusion + at-least-one-auth-mechanism + header-name validation; `validate_oauth2_auth()` gained a `client_assertion` branch that validates the SPIFFE-JWT assertion's own subfields instead of requiring `client_id`/`client_secret`, while preserving the original empty-check rejection when `client_assertion` is absent (T-113-06 — no silent no-auth OAuth2 route). 9 new profile-validation unit tests + 4 new schema-level round-trip tests, all passing.
- `nono-profile.schema.json` (fork-only strict-schema companion) gained the `spiffe` property, `$defs/SpiffeAuthConfig`, `$defs/ClientAssertionConfig`, and `$defs/OAuth2Config`'s `required` array narrowed from `[token_url, client_id, client_secret]` to `[token_url]` plus `client_assertion`/`extra_params` properties — all four SC2-mandatory edits landed, verified via `python -c "import json; json.load(...)"` (syntactically valid) and a new `test_schema_self_is_valid_json` unit test.
- `enforce_spiffe_socket_isolation()` landed in `proxy_runtime.rs`: deny-by-omission on Linux (no `unix_socket` grant ever issued — structural under Landlock's allow-list-only model), an explicit Seatbelt `(deny network-outbound (path ...))` rule on macOS (mirroring `policy.rs::add_deny_access_rules`'s existing pattern — `connect(2)` on AF_UNIX is Seatbelt network-outbound, not a file operation), a logged residual-risk no-op on Windows (T-113-05, not silently assumed protected), and a hard `NonoError` (never `warn!`) on a conflicting explicit `unix_socket` grant. Wired into `start_proxy_runtime()` before the tokio runtime starts the proxy. 3 new unit tests, all passing.

## Task Commits

Each task was committed atomically, DCO-signed:

1. **Task 1: config.rs schema types + serde round-trip tests + workspace-wide RouteConfig literal fixup (nono-proxy crate)** - `184209b6` (feat)
2. **Task 2: Workspace-wide CustomCredentialDef literal fixup (nono-cli crate)** - `611a8f4b` (feat)
3. **Task 3: SC2 validation + schema + proxy_runtime socket isolation** - `37ae069d` (feat)

**Plan metadata:** this SUMMARY + its own commit is that step (see below).

## Files Created/Modified

- `crates/nono-proxy/src/config.rs` - `SpiffeAuthConfig`, `ClientAssertionConfig`, `RouteConfig.spiffe`, `OAuth2Config.client_assertion`/`extra_params`, `#[serde(default)]` on `client_id`/`client_secret`, `Debug` impl updated, 2 new tests, 1 existing test-fixture literal fixed
- `crates/nono-proxy/src/credential.rs` - 3 pre-existing `RouteConfig{..}` test-fixture literals fixed (`spiffe: None`)
- `crates/nono-proxy/src/reverse.rs` - 1 pre-existing `RouteConfig{..}` test-fixture literal fixed
- `crates/nono-proxy/src/route.rs` - 8 pre-existing `RouteConfig{..}` test-fixture literals fixed
- `crates/nono-proxy/src/server.rs` - 12 pre-existing `RouteConfig{..}` test-fixture literals fixed
- `crates/nono-cli/src/network_policy.rs` - `CustomCredentialDef.spiffe` thread-through at the real `resolve_credentials`/`partition_allow_domain` production sites + ~12 test-fixture literals fixed
- `crates/nono-cli/src/profile/mod.rs` - `CustomCredentialDef.spiffe`, `validate_header_name()` extraction, spiffe mutual-exclusion + client_assertion validation, ~27 test-fixture literals fixed, 13 new tests (9 validation + 4 schema)
- `crates/nono-cli/data/nono-profile.schema.json` - `spiffe` property, `$defs/SpiffeAuthConfig`, `$defs/ClientAssertionConfig`, `$defs/OAuth2Config` required-array narrowing + new properties
- `crates/nono-cli/src/proxy_runtime.rs` - `enforce_spiffe_socket_isolation()`, `try_canonicalize()`, `collect_spiffe_socket_paths()`, wired into `start_proxy_runtime()`, 3 new tests, 2 test-fixture literals fixed

## Decisions Made

- Confirmed by direct grep, not inherited: `proxy_runtime.rs` had zero pre-existing let-chains and zero pre-existing `add_platform_rule`/`unix_socket_capabilities` call sites before this plan — both claims in the plan's `<action>`/`<read_first>` text were inaccurate for the current tree state. Recorded as disposition amendments above rather than silently papered over.
- Broadened `enforce_spiffe_socket_isolation()`'s socket-collection scope to cover `oauth2.client_assertion` in addition to `RouteConfig.spiffe` — both configurations reach the same SPIRE socket on the child's behalf; isolating only the direct-JWT shape would have left the OAuth2-assertion shape's socket path ungoverned by this control.
- `validate_oauth2_auth()`'s client_assertion branch uses an early return rather than the plan's if/else prose shape — functionally identical, avoids duplicating logic under a nested else block.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `cargo fmt --all --check` failure on a new test's line width**
- **Found during:** Task 3, after adding `test_enforce_spiffe_socket_isolation_hard_errors_on_conflicting_grant`
- **Issue:** `nono::UnixSocketCapability::new_file(...)` call exceeded the configured line-wrap width in the new test
- **Fix:** Ran `cargo fmt --all`, which reformatted the call across two lines; re-ran `cargo fmt --all --check` to confirm clean
- **Files modified:** `crates/nono-cli/src/proxy_runtime.rs`
- **Verification:** `cargo fmt --all --check` exits 0
- **Committed in:** `37ae069d` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 bug/fmt)
**Impact on plan:** Trivial and mechanical. No scope creep. The two disposition amendments above (missing let-chain, missing call sites) are documentation corrections to the plan's stated assumptions, not code deviations — they changed nothing about what code needed to be written, only confirmed the plan's `<read_first>` text was imprecise for this file.

## Issues Encountered

None blocking. Both cross-target clippy gates were run because `proxy_runtime.rs` (touched by Task 3) contains pre-existing `#[cfg(target_os = "linux")]` blocks (unrelated to this plan's own changes), triggering CLAUDE.md's cross-target clippy MUST rule:

- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — **RED**, 10 `dead_code` errors, all on `spiffe.rs`/`auth.rs` symbols (`SpiffeJwtSource`, `check_nbf`, `delegation_from_jwt`, `ManagedUpstreamAuth`, `UpstreamAuthMaterial`, `extract_trust_domain`, `JWT_REFRESH_SECS`) — **identical to Plan 113-01's documented inherited RED state**, zero new findings.
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) — **RED**, identical 10 `dead_code` errors, zero new findings.

**Why this is expected, not a regression:** per 113-01-SUMMARY.md's own "Issues Encountered" section, this RED state was explicitly flagged as inherited until a later plan (113-03 or later) wires `ManagedUpstreamAuth`/`SpiffeJwtSource` into `route.rs`/`reverse.rs`/`credential.rs` as real consumers. This plan's own `<verification>` block does not call for a clippy run (only `cargo build`/`cargo test`/`cargo fmt --all --check`) — consistent with that. No `#[allow(dead_code)]` was added (CLAUDE.md's explicit guidance against masking). This RED state remains a known, load-bearing fact for whichever later plan runs the phase-level cross-target clippy gate — only *new* findings beyond these exact 10 would indicate a regression, and there are none.

## User Setup Required

None — no external service configuration required. SPIRE agent socket connectivity is exercised only by later plans' integration tests (D-06, gated on `SPIRE_AGENT_SOCKET`), not by this plan's config/validation-only surface.

## Verification Results

- `cargo build --workspace --all-targets` — **GREEN**, zero errors.
- `cargo test -p nono-sandbox-proxy --lib -- config::tests::test_spiffe_jwt_config_roundtrip` — **GREEN**.
- `cargo test -p nono-sandbox-proxy --lib -- config::tests::test_spiffe_absent_by_default` — **GREEN**.
- `cargo test -p nono-sandbox-proxy --lib` — **GREEN**, 229 passed (227 baseline + 2 new), 0 failed.
- `cargo test -p nono-sandbox-cli --bin nono -- profile:: network_policy:: proxy_runtime::` — **GREEN**, 384 passed, 0 failed. (Per 113-01's documented disposition amendment: `nono-cli` is bin-only, no `--lib` target — `--bin nono` used throughout, matching the inherited fact.)
- `cargo fmt --all --check` — **GREEN**.
- `grep -c "&& let" crates/nono-cli/src/proxy_runtime.rs` — returns `0`.
- `grep -A3 "cred.credential_key.clone()" crates/nono-cli/src/network_policy.rs | grep -c "spiffe: cred.spiffe.clone()"` — the plan's literal pattern returns 0 due to line-distance, but `grep -n "spiffe: cred.spiffe.clone()"` confirms the propagation exists at `network_policy.rs:251`, correctly placed in the custom-credential branch (see excerpt in Task 2 commit).
- `python -c "import json; json.load(open('crates/nono-cli/data/nono-profile.schema.json'))"` — **VALID JSON** (this host's `python3` alias resolves to `python`, not a missing binary — the CLAUDE.md note about the `cross` container's missing python3 does not apply to this host-level check).
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — **RED** (10 `dead_code` errors, all inherited from Plan 113-01, zero new — see Issues Encountered).
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` — **RED** (identical inherited state, zero new).

## Next Phase Readiness

- `RouteConfig.spiffe`, `CustomCredentialDef.spiffe`, `OAuth2Config.client_assertion`, the full schema surface, and `enforce_spiffe_socket_isolation()` are all landed and stable for Plan 113-03+ to build the actual request-handling logic (`handle_spiffe_route`, `handle_spiffe_assertion_credential`, `CredentialStore.spiffe_assertion_routes`) against.
- **Watch-item carried from 113-01, unchanged by this plan:** the dead_code RED state on `spiffe.rs`/`auth.rs` (10 errors, both cross-target gates) must resolve once a later plan wires real consumers. This plan did not touch `spiffe.rs`/`auth.rs` and did not change that count — confirmed identical before/after via direct grep of the clippy output.
- **Known follow-up, out of this plan's file scope:** `../nono-py/src/proxy.rs:206-218`'s exhaustive `RustRouteConfig{..}` literal will break with `E0063: missing field 'spiffe'` the next time `maturin build` (or any Rust build in that sibling repo) runs, per 113-RESEARCH.md's D-09 finding. This plan's `files_modified` scope does not include `../nono-py`; the fix (`spiffe: None,` after the existing `aws_auth: None,` line) is a one-line change for whichever later plan rebuilds the bindings (113-08 per the phase's ROADMAP SC4 test map).
- No blockers for Plan 113-03.

---
*Phase: 113-spiffe-spire-workload-identity*
*Completed: 2026-08-06*

## Self-Check: PASSED

All 9 claimed modified files confirmed present on disk (`config.rs`, `credential.rs`, `reverse.rs`,
`route.rs`, `server.rs`, `network_policy.rs`, `profile/mod.rs`, `nono-profile.schema.json`,
`proxy_runtime.rs`). All 3 claimed task commit hashes (`184209b6`, `611a8f4b`, `37ae069d`)
confirmed present in `git log --oneline --all`.
