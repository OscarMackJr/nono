---
phase: 114-oauth-capture-absorb-sec-02
plan: 04
subsystem: network-proxy
tags: [oauth-capture, sec-02, capture-store, phantom-token, zeroize, fail-closed]

# Dependency graph
requires:
  - phase: 114-oauth-capture-absorb-sec-02
    provides: "Plan 114-02's CaptureConfig / CaptureResponseField / CaptureResponseFieldKind declarative types in crates/nono-proxy/src/config.rs"
provides:
  - "CapturePhantomStore (mint/resolve) — in-memory, session-scoped, zeroized, per-consumer admission-scoped phantom-token store, crates/nono-proxy/src/capture.rs"
  - "rewrite_response_fields() — configured-field response rewrite, one phantom per field"
  - "reject_unrewritten_token_fields() — fail-closed whole-tree scan backstop against provider-config drift"
  - "jwt_shaped_phantom() — alg:none JWT-shaped phantom construction"
  - "resolve_request_nonce_fields() — request-side mirror of rewrite_response_fields(), store-level half of the mint-to-resolve loop"
affects: [114-05, 114-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Not-yet-wired-in module functions made `pub` rather than `pub(crate)`/private, specifically to avoid the dead_code trap `cargo clippy --lib` (no test cfg) hits when a function's only call sites are inside #[cfg(test)] — mirrors Plan 114-03's `capture_declared_for_upstream` precedent for the identical not-yet-wired-in reason"
    - "Mutex lock poisoning handled via `.lock().unwrap_or_else(std::sync::PoisonError::into_inner)`, the established codebase-wide idiom (keystore.rs, control_loop.rs, capability_ext.rs, etc.) — chosen over a Result-returning lock accessor so mint()/resolve() could keep the plan's literal `String`/`Option<...>` return signatures"

key-files:
  created:
    - crates/nono-proxy/src/capture.rs
  modified:
    - crates/nono-proxy/src/lib.rs

key-decisions:
  - "mint() returns Result<String> (deviates from the plan's literal `-> String`) so an RNG failure fails closed via ProxyError::Config rather than being silently absorbed into a fallback/weaker phantom — matches token.rs's own generate_session_token() -> Result<Zeroizing<String>> pattern for the identical 'generate a random identifier' operation, and CLAUDE.md's Fail Secure principle. resolve() keeps the plan's literal Option<...> (no failure mode beyond lock poisoning, which is handled by non-panicking recovery, not propagated)."
  - "rewrite_response_fields/reject_unrewritten_token_fields/jwt_shaped_phantom/resolve_request_nonce_fields are `pub`, not the plan's specified `pub(crate)`. With no production caller landing until Plan 114-06 Task 3, a pub(crate) function whose only call sites are #[cfg(test)] is flagged dead_code by `cargo clippy --lib` (which does not compile the test cfg) — and #[allow(dead_code)] is disallowed by CLAUDE.md policy and by this plan's own project_gotchas #8 ('do NOT add #[allow(dead_code)] to work around it'). Widening visibility to pub is the same resolution Plan 114-03 already applied to capture_declared_for_upstream for the identical reason, and is a pure visibility change with zero behavior difference (all four functions are still crate-internal in practice until 114-06 wires them in)."
  - "CapturePhantomStore has a manual, redacted Debug impl (never derived) that reports only an entry count — never real tokens or admitted-consumer names. Proven by a dedicated test (capture_store_debug_output_never_contains_real_token), not just documented."
  - "capture_store_holds_only_in_memory (the D-08 in-memory-only structural proof) is implemented as an automated #[test] reading this module's own source via include-at-runtime (std::fs::read_to_string of its own known crate-relative path) and scanning only the portion BEFORE the #[cfg(test)] marker for banned disk-write patterns — not by shelling out to an external `grep` binary (fragile on a Windows host where `grep` may not be on PATH for a `cargo test`-spawned process) and not by include_str!(self) (which would self-match the banned-pattern string literals inside the test's own source)."

patterns-established:
  - "The `#[must_use]` + Result-returning-function combo triggers clippy::double_must_use ('this function has a #[must_use] attribute with no message, but returns a type already marked as #[must_use]') under -D warnings — caught immediately by the -D warnings gate on mint(), which was written with a redundant #[must_use] alongside `-> Result<String>`. Fixed by removing the attribute (Result is already must_use). resolve()'s `#[must_use]` on `Option<...>` did NOT trigger the same lint (Option-returning #[must_use] functions already exist unflagged elsewhere in this crate, e.g. credential.rs::get, route.rs::get, pool.rs::get_pinned)."

requirements-completed: [SEC-02]

# Metrics
duration: ~25min
completed: 2026-08-06
---

# Phase 114 Plan 04: CapturePhantomStore + Portable Rewrite Logic Summary

**Built `crates/nono-proxy/src/capture.rs`: an in-memory, session-scoped, zeroized phantom-token store (`CapturePhantomStore::mint`/`resolve`, per-consumer admission-scoped, `None`-for-both existence/admission failure to avoid an enumeration oracle) plus the D-07 portable rewrite algorithms (`rewrite_response_fields`, `reject_unrewritten_token_fields` fail-closed backstop, `jwt_shaped_phantom`, `resolve_request_nonce_fields`) adapted from upstream's cited `oauth_capture/{jwt,rewrite}.rs` design — 17 tests, zero disk I/O, zero real-token leakage into any Debug output, all proven by dedicated tests rather than assertion alone.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 2 planned (both TDD), both completed
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments
- `CapturePhantomStore` exists with `new()`, `mint(real, admitted_consumers) -> Result<String>`, and `resolve(phantom, consumer) -> Option<Zeroizing<Vec<u8>>>`. Real token bytes live in `Zeroizing<Vec<u8>>` inside a private `StoredCapture`, never persisted to disk (D-08). A manual, redacted `Debug` impl reports only an entry count.
- `mint()` generates a 256-bit random phantom via `getrandom::fill` + URL-safe base64, proven unique across 1000 iterations in one test; fails closed (`ProxyError::Config`) on RNG failure rather than falling back to weaker randomness.
- `resolve()` returns `None` uniformly for both "phantom not found" and "phantom found but consumer not admitted" — proven by two separate tests — closing the existence-oracle threat (T-114-07).
- `value_at_path_mut()` (private) does dot-path traversal into a `serde_json::Value`, returning `None` (never panicking) on any missing/non-object intermediate segment.
- `rewrite_response_fields()` mints one phantom per configured `CaptureResponseField`, writes it in place (opaque string or `jwt_shaped_phantom()` output per `CaptureResponseFieldKind`), returns the dot-paths actually rewritten; missing paths and non-string values are silently skipped, not an error.
- `reject_unrewritten_token_fields()` walks the whole JSON tree recursively and errors (`ProxyError::HttpParse`) if any `access_token`/`refresh_token`/`id_token` field holds non-empty string content at a path NOT in the configured set — the fail-closed backstop against provider-config drift (D-05/SC2). Proven to both trip on an unconfigured field and pass on a configured or absent one.
- `jwt_shaped_phantom()` builds a 3-part `header.payload.signature` string, `alg: none`, `signature` literally equal to the input phantom — proven by a dedicated part-count/verbatim-signature test.
- `resolve_request_nonce_fields()` is the request-side mirror, reusing `value_at_path_mut()` (no duplicated traversal logic): resolves an admitted phantom to its real value in place and returns the count resolved; leaves unresolved/unadmitted/unknown values unchanged, never errors — closing the mint-to-resolve loop's store-level half ahead of Plan 114-06 Task 3's live dispatch-path wiring.
- 17 new tests, all passing: `cargo test -p nono-sandbox-proxy --lib` = 268/268 (251 baseline + 17 new).
- `cargo build --workspace --all-targets` exits 0; `cargo fmt --all -- --check` clean; `cargo clippy -p nono-sandbox-proxy --lib -- -D warnings -D clippy::unwrap_used` and `--all-targets` variant both clean.
- No `target_os` cfg gates touched in this plan's files, so CLAUDE.md's cross-target clippy gate (linux-gnu/apple-darwin) does not apply.

## Task Commits

Each task was committed atomically:

1. **Task 1: CapturePhantomStore (mint/resolve) — zeroized, session-scoped, in-memory only** - `1dedd342` (feat)
2. **Task 2: Portable rewrite logic — dot-path field rewrite, fail-closed unrewritten-field scan, JWT-shaped phantom, request-side nonce resolution** - `bcede427` (feat)

## Files Created/Modified
- `crates/nono-proxy/src/capture.rs` (created) - `CapturePhantomStore`, `StoredCapture` (private), `mint`/`resolve`, `generate_phantom_id` (private), `value_at_path_mut` (private), `is_sensitive_token_field` (private), `rewrite_response_fields`, `reject_unrewritten_token_fields`/`reject_unrewritten_token_fields_inner` (private), `jwt_shaped_phantom`, `resolve_request_nonce_fields`, 17 tests
- `crates/nono-proxy/src/lib.rs` - `pub mod capture;` added alphabetically between `mod auth;` and `pub mod config;`

## Decisions Made
- **`mint()` returns `Result<String>`, not the plan's literal `-> String`.** The plan's `<behavior>` bullet stated a bare `String` return, but its own `<action>` text says to "map a lock-poisoning error to `ProxyError::Config`, never `.unwrap()`" — internally inconsistent with a non-`Result` signature. I resolved this in favor of fail-secure behavior on the one genuine failure mode `mint()` has (RNG failure via `getrandom::fill`), matching this crate's own `token.rs::generate_session_token() -> Result<Zeroizing<String>>` for the identical operation, and CLAUDE.md's "libraries should almost never panic... use Result instead" guidance. Lock poisoning itself is handled via the codebase-wide `.lock().unwrap_or_else(PoisonError::into_inner)` recovery idiom (not propagated as an error), so `resolve()` keeps its plan-literal `Option<...>` signature unchanged.
- **Widened `rewrite_response_fields`/`reject_unrewritten_token_fields`/`jwt_shaped_phantom`/`resolve_request_nonce_fields` from the plan's specified `pub(crate)` to `pub`.** `cargo clippy -p nono-sandbox-proxy --lib -- -D warnings -D clippy::unwrap_used` (the plan's own acceptance-criteria command, and a subset of CLAUDE.md's authoritative gate) does not compile the `#[cfg(test)]` module — it is not a `--tests` invocation. With no production caller landing until Plan 114-06 Task 3, every `pub(crate)`-or-private function whose only call sites are inside `#[cfg(test)]` was flagged `dead_code` under `-D warnings`, a hard compile error. This plan's own `project_gotchas #8` explicitly named this exact class of problem for `resolve()`/`resolve_request_nonce_fields()` and explicitly forbade `#[allow(dead_code)]` as the fix. The correct fix — already established by Plan 114-03's identical `capture_declared_for_upstream` situation — is visibility widening to `pub`, which the compiler exempts from `dead_code` analysis as part of the crate's external API surface. Zero behavior change; all four are still crate-internal in practice until 114-06 wires them into `reverse.rs`.
- **`CapturePhantomStore` derives `Default`** (rather than a hand-written empty constructor) so `new()` can delegate to it, avoiding `clippy::new_without_default` without adding an unnecessary manual impl — `Mutex<HashMap<...>>` already implements `Default`.
- **`resolve_request_nonce_fields`'s real-value write-back uses `String::from_utf8(real.to_vec())`**, tolerating (by skipping, not erroring) a non-UTF-8 resolved value — best-effort per the plan's own behavior spec ("this function never errors").
- **`capture_store_holds_only_in_memory` (D-08's automated in-memory-only proof) reads its own source via `std::fs::read_to_string` at test time and scans only the slice before the `#[cfg(test)]` marker**, rather than shelling out to an external `grep` binary (the plan's suggested alternative) or using `include_str!` on itself (which would self-match the banned-pattern string literals embedded in the test's own source, producing a false failure). This is the "structural code-review assertion... automated, not prose" alternative the plan's `<behavior>` block explicitly permitted.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `#[must_use]` on `mint()` (returning `Result<String>`) triggered `clippy::double_must_use` under `-D warnings`**
- **Found during:** Task 1, first `cargo clippy -p nono-sandbox-proxy --lib -- -D warnings -D clippy::unwrap_used` run
- **Issue:** `Result<T, E>` is already `#[must_use]` in std; adding a bare `#[must_use]` (no custom message) to a function returning it is itself a clippy-flagged anti-pattern, promoted to a hard error by `-D warnings`.
- **Fix:** Removed `#[must_use]` from `mint()`. `resolve()`'s `#[must_use]` on `Option<...>` was left in place (does not trigger the lint; matches multiple existing `Option`-returning `#[must_use]` functions elsewhere in this crate).
- **Files modified:** `crates/nono-proxy/src/capture.rs`
- **Verification:** `cargo clippy -p nono-sandbox-proxy --lib -- -D warnings -D clippy::unwrap_used` exits 0.
- **Committed in:** `1dedd342` (Task 1 commit)

**2. [Rule 3 - Blocking] `pub(crate)` rewrite/reject/jwt/resolve functions flagged `dead_code` under `cargo clippy --lib` (no test cfg, no production caller yet)**
- **Found during:** Task 2, first `cargo clippy -p nono-sandbox-proxy --lib -- -D warnings -D clippy::unwrap_used` run after adding all four Task 2 functions at the plan's specified `pub(crate)` visibility
- **Issue:** `cargo clippy --lib` (without `--tests`) does not enable the `test` cfg, so `#[cfg(test)] mod tests { ... }` — the only call sites these functions had prior to Plan 114-06 — is entirely elided from that compilation unit. `-D warnings` promotes the resulting `dead_code` lint to a hard error for all of: `value_at_path_mut`, `is_sensitive_token_field`, `rewrite_response_fields`, `reject_unrewritten_token_fields`/`_inner`, `jwt_shaped_phantom`, `resolve_request_nonce_fields`.
- **Fix:** Widened `rewrite_response_fields`, `reject_unrewritten_token_fields`, `jwt_shaped_phantom`, and `resolve_request_nonce_fields` from `pub(crate)` to `pub` (the private helpers `value_at_path_mut`/`is_sensitive_token_field`/`reject_unrewritten_token_fields_inner` then resolve transitively, since rustc's reachability analysis treats anything called from a `pub` function's body as reachable). Confirmed `--tests` variant of the same clippy invocation does NOT independently fix this (the plain `--lib` target is still checked separately and still fails) — visibility widening was the only compliant fix given the "no `#[allow(dead_code)]`" constraint.
- **Files modified:** `crates/nono-proxy/src/capture.rs` (also updated `resolve_request_nonce_fields`'s doc comment, which had stated the now-inaccurate `pub(crate)` rationale)
- **Verification:** `cargo clippy -p nono-sandbox-proxy --lib -- -D warnings -D clippy::unwrap_used` and `--all-targets` variant both exit 0; `cargo build --workspace --all-targets` exits 0.
- **Committed in:** `bcede427` (Task 2 commit)

**3. [Rule 1 - Bug] `assert_eq!` type mismatch in `capture_phantom_resolve_returns_real_for_admitted_consumer`**
- **Found during:** Task 1, first `cargo test -p nono-sandbox-proxy --lib capture::` run
- **Issue:** `Option<Zeroizing<Vec<u8>>>::as_deref()` derefs to `&Vec<u8>`, not `&[u8]` — comparing against `Some(b"real-secret".as_slice())` was an E0308 type mismatch, not a real assertion failure.
- **Fix:** Changed the comparison to `resolved.map(|z| z.to_vec())` against `Some(b"real-secret".to_vec())`.
- **Files modified:** `crates/nono-proxy/src/capture.rs`
- **Verification:** Test compiles and passes.
- **Committed in:** `1dedd342` (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (2 Rule 3 - blocking compile errors under the mandatory `-D warnings`/`-D clippy::unwrap_used` gates, 1 Rule 1 - test-code type-mismatch bug). No scope creep: every fix is either a lint-driven mechanical correction (attribute removal, visibility widening exactly mirroring Plan 114-03's established precedent) or a test-only type fix. All behavior described in the plan's `<behavior>` blocks is implemented and tested; no functionality was dropped or narrowed.
**Impact on plan:** None on the delivered capability. The `pub`-vs-`pub(crate)` visibility deviation is cosmetic from a security standpoint (both are equally reachable from `crate::reverse`, the only intended caller) and is explicitly the same resolution the phase already applied once before (114-03) for the identical not-yet-wired-in dead-code situation — Plan 114-06 Task 3 can narrow these back to `pub(crate)` once it adds the real call site, if desired, though there is no correctness reason to.

## Issues Encountered
None beyond the deviations documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
`CapturePhantomStore`, `rewrite_response_fields`, `reject_unrewritten_token_fields`, `jwt_shaped_phantom`, and `resolve_request_nonce_fields` are all implemented, tested (17 tests), and exported (`pub`) from `crates/nono-proxy/src/capture.rs` for Plan 114-05 (the D-05 response-buffer-cap enforcement point) and Plan 114-06 (wiring the buffer-and-rewrite hook into `reverse.rs`'s three response-relay sites, plus `resolve_request_nonce_fields`'s live outbound-dispatch call site per this plan's own `<behavior>` note). No blockers. Note for Plan 114-06: `rewrite_response_fields` takes an `admitted_consumers: HashSet<String>` parameter that the caller must construct from route/consumer context — this plan does not define what "consumer" identity means at the `reverse.rs` call site (route prefix? session ID?); that decision belongs to 114-06, which has the actual dispatch context.

---
*Phase: 114-oauth-capture-absorb-sec-02*
*Completed: 2026-08-06*

## Self-Check: PASSED

`crates/nono-proxy/src/capture.rs` verified present on disk (336+229 lines
added across 2 commits). `crates/nono-proxy/src/lib.rs`'s `pub mod capture;`
line verified present. Both commit hashes (`1dedd342`, `bcede427`) verified
present in `git log --oneline -5`. `cargo test -p nono-sandbox-proxy --lib`
re-verified at 268 passed, 0 failed (251 baseline + 17 new) immediately
before writing this section. `cargo build --workspace --all-targets`,
`cargo fmt --all -- --check`, and both `cargo clippy` gates
(`--lib` and `--all-targets`, both `-D warnings -D clippy::unwrap_used`)
re-verified clean immediately before writing this section.
