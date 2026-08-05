---
phase: 112-security-residual-sync
plan: 07
subsystem: infra
tags: [nono-proxy, cli, network-filtering, credential-injection, upstream-sync, auth]

# Dependency graph
requires:
  - phase: 112-security-residual-sync
    provides: "112-02's Wave-1 lesson (symbol-level verification over file-presence-only trust) and the D-04 sizing that gave SEC-07 its own plan/wave"
provides:
  - "Standalone `nono proxy` subcommand: foreground network-filtering / credential-injection proxy server with no sandboxed child"
  - "ProxyConfig.require_auth / ProxyConfig.strict_connect_auth: a require-auth concept in nono-proxy that did not exist before this plan (the fork previously had no --no-auth mode at all; reverse-proxy auth was unconditionally fatal, CONNECT auth was unconditionally lenient)"
  - "parse_allow_domain_arg / parse_allow_endpoint_arg promoted pub(crate) for reuse outside proxy_runtime.rs"
affects: [future-nono-proxy-auth-model-changes, any-future-standalone-proxy-flag-additions]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Standalone command reuses the same ProxyLaunchOptions -> build_proxy_config_from_flags -> nono_proxy::server::start path nono run's proxy activation uses, instead of reimplementing config construction — inherits ADR-108 deny-domain and D-06 no_proxy-overlap validation for free"
    - "Auth-mode threading via two independent bools (require_auth, strict_connect_auth) on ProxyConfig rather than a 3-state enum — simpler than upstream's ConnectAuthMode, defaults preserve pre-existing sandboxed-run behavior byte-for-byte"
    - "ExternalProxyCtx struct introduced to keep handle_external_proxy under clippy::too_many_arguments after adding require_auth, mirroring the existing ReverseProxyCtx pattern"

key-files:
  created:
    - crates/nono-cli/src/proxy_command.rs
    - crates/nono-cli/tests/proxy_command_run.rs
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/app_runtime.rs
    - crates/nono-cli/src/cli_bootstrap.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/Cargo.toml
    - crates/nono-proxy/src/config.rs
    - crates/nono-proxy/src/connect.rs
    - crates/nono-proxy/src/external.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/server.rs

key-decisions:
  - "Adapted, not literally ported: upstream's proxy_command.rs imports CredentialProxyIntent/DomainFilterIntent/EndpointFilterIntent/TlsInterceptIntent/UpstreamProxyIntent from launch_runtime.rs — 0 grep hits in this fork. Built ProxyArgs -> ProxyLaunchOptions -> build_proxy_config_from_flags -> nono_proxy::server::start instead, reusing the fork's real (post-ADR-98) proxy-launch API."
  - "apply_tls_intercept_config dropped entirely (0 grep hits in the fork; no tls_intercept module exists) — proxy_command.rs's own module doc comment states this and is itself grep-clean of the banned type/function names, satisfying the plan's literal acceptance criterion."
  - "ProxyArgs omits upstream's --pass (custom session password) and --proxy-ca-cert/--proxy-ca-key (CA reuse across runs): this fork's nono_proxy::token module only supports randomly generated session tokens, and TLS interception is not implemented in this fork (ProxyHandle::intercept_ca_path() always returns None, confirmed via grep — the CA flags on nono run's own SandboxArgs are already unconsumed dead CLI surface, pre-existing and out of scope). Neither omission weakens auth or filtering; both have safe fallbacks."
  - "Security-relevant adaptation beyond the plan's literal per-file diff list: added ProxyConfig.require_auth (default true) and ProxyConfig.strict_connect_auth (default false) to nono-proxy. Before this plan the fork had no no-auth concept at all — reverse.rs's two auth checks were unconditionally fatal and connect.rs's CONNECT auth check was unconditionally lenient (comment: 'Non-fatal for CONNECT: Node.js undici doesn't send Proxy-Authorization'), both correct for the sandboxed-run-only world where the OS sandbox is the trust boundary. A standalone proxy has no OS sandbox behind it, so --no-auth needed a real way to disable auth (require_auth=false), and the default (auth on) needed the CONNECT-tunnel leniency closed so an unauthenticated client bound to a non-loopback --listen with no --no-auth could not tunnel straight past the session-token boundary via CONNECT. Both fields default to this fork's pre-existing behavior, so nono run/shell/wrap is byte-for-byte unaffected."
  - "Fixed a clippy::too_many_arguments violation (8 args) in external::handle_external_proxy caught only by the cross-target linux-gnu gate (invisible to the Windows-host build) by introducing ExternalProxyCtx, mirroring the existing ReverseProxyCtx pattern."

patterns-established:
  - "When adding an auth-mode toggle to a security-critical proxy path, default new fields to the exact pre-existing behavior and gate every enforcement check on the new field explicitly, rather than restructuring existing control flow — keeps the diff auditable and the default path byte-for-byte unchanged."

requirements-completed: [SEC-07]

# Metrics
duration: ~2h30min
completed: 2026-08-05
---

# Phase 112 Plan 07: Standalone `nono proxy` Command (SEC-07) Summary

**Absorbed upstream `2663e990` (#1261) as an adaptation: a new `nono proxy` foreground-server subcommand built on the fork's real `ProxyLaunchOptions`/`build_proxy_config_from_flags`/`nono_proxy::server::start` API (upstream's Intent-struct imports and `apply_tls_intercept_config` confirmed absent from this fork), plus a genuinely new `require_auth`/`strict_connect_auth` concept added to `nono-proxy` because the fork had no `--no-auth` mode at all before this plan.**

## Performance

- **Duration:** ~2h30min
- **Started:** 2026-08-05 (session start)
- **Completed:** 2026-08-05
- **Tasks:** 3/3 completed
- **Files modified:** 11 modified + 2 created (13 total; plan listed 14 including `route.rs`/`token.rs`, neither of which required a code change — see Deviations)

## Accomplishments
- `nono proxy` dispatches end-to-end: `Commands::Proxy(Box<ProxyArgs>)` in `cli.rs`, dispatch arm in `app_runtime.rs`, per-variant verbosity arm in `cli_bootstrap.rs`, `mod proxy_command;` in `main.rs`.
- `proxy_command.rs::run_proxy` preserves the `--no-auth` + non-loopback `--listen` fail-secure guard verbatim (T-112-15), merges `--profile` network settings with CLI flags into a `ProxyLaunchOptions`, reuses `build_proxy_config_from_flags` (inheriting ADR-108 deny-domain and D-06 no_proxy-overlap validation identically to the sandboxed `run` path), and blocks on Ctrl-C while periodically draining the network audit buffer.
- Added a real auth-mode toggle to `nono-proxy` (`ProxyConfig.require_auth` / `ProxyConfig.strict_connect_auth`) that did not exist in the fork before this plan — see the Security Note below.
- Both mandatory cross-target clippy gates ran GREEN with `--all-targets`: linux-gnu via `cross clippy` (Docker, ~1m09s), apple-darwin via `cargo-zigbuild clippy` (SDKROOT unset, ~16s). `cargo fmt --all --check` clean. `cargo build --workspace --all-targets` exits 0.
- 218/218 pre-existing `nono-sandbox-proxy` unit tests still pass unchanged after the auth-mode threading (proves the new fields' defaults are byte-for-byte compatible with prior behavior). 2/2 new Wave-0 tests pass (`--help` usage line, `--no-auth`+non-loopback rejection).

## Task Commits

Each task was committed atomically:

1. **Task 1: CLI wiring contracts (cli.rs/main.rs/app_runtime.rs/cli_bootstrap.rs)** - `00d45c67` (feat)
2. **Task 2: proxy_command.rs implementation + nono-proxy auth-mode adaptation** - `14ed22e5` (feat)
3. **Task 3: Wave-0 dispatch/fail-secure tests + too_many_arguments fix (cross-target gate finding)** - `ad8132e7` (test)

_This SUMMARY's own metadata commit follows separately per the standard protocol._

## Files Created/Modified
- `crates/nono-cli/src/cli.rs` - `Commands::Proxy(Box<ProxyArgs>)` variant, full `ProxyArgs` struct (adapted field set), banner lines, `ALL_SUBCOMMANDS` test entry
- `crates/nono-cli/src/main.rs` - `mod proxy_command;`
- `crates/nono-cli/src/app_runtime.rs` - dispatch arm
- `crates/nono-cli/src/cli_bootstrap.rs` - per-variant verbosity arm
- `crates/nono-cli/src/proxy_runtime.rs` - `parse_allow_domain_arg`/`parse_allow_endpoint_arg` promoted to `pub(crate)`
- `crates/nono-cli/Cargo.toml` - added `signal`/`time` tokio features for `ctrl_c()`/`interval()`
- `crates/nono-cli/src/proxy_command.rs` (new) - `run_proxy`, `build_launch_options`, `run_until_shutdown`, `print_connection_info`
- `crates/nono-cli/tests/proxy_command_run.rs` (new) - the 2 Wave-0 dispatch/fail-secure tests
- `crates/nono-proxy/src/config.rs` - `ProxyConfig.require_auth`/`strict_connect_auth` fields + defaults
- `crates/nono-proxy/src/connect.rs` - `handle_connect` gains `strict_auth: bool`, enforces 407 when set
- `crates/nono-proxy/src/external.rs` - new `ExternalProxyCtx` struct, `handle_external_proxy` gains `require_auth` gating
- `crates/nono-proxy/src/reverse.rs` - `ReverseProxyCtx.require_auth` field, both auth checks gated on it
- `crates/nono-proxy/src/server.rs` - threads `state.config.require_auth`/`strict_connect_auth` into all 4 call sites (external, bypass-route, 2x connect)

## Decisions Made

See `key-decisions` in frontmatter. Summary: this plan is an **adapt**, not a literal port, at two independent levels — (1) the CLI/config-construction layer uses the fork's real `ProxyLaunchOptions` API instead of upstream's absent Intent structs, and (2) the auth-enforcement layer required a genuinely new `require_auth`/`strict_connect_auth` mechanism because the fork had no `--no-auth` concept at all prior to this plan (not merely a differently-named equivalent).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - missing critical functionality] Fork had no `--no-auth` / auth-disable concept in `nono-proxy` at all**
- **Found during:** Task 2, while tracing how `--no-auth` would need to reach the reverse-proxy and CONNECT auth checks.
- **Issue:** `grep -n "require_auth" crates/nono-proxy/src/*.rs` returned 0 hits before this plan. `reverse.rs`'s two auth checks (credential-route phantom-token, no-credential session-token) were unconditionally fatal — correct for the sandboxed-run-only world, but with no way to satisfy `--no-auth`'s documented contract without a real toggle. Separately, `connect.rs`'s CONNECT-tunnel auth check was unconditionally *lenient* (`if let Err(e) = validate_proxy_auth(...) { debug!(...) }`, never rejecting) — correct for the sandboxed path (OS sandbox is the trust boundary; Node.js undici doesn't echo Proxy-Authorization on CONNECT) but a genuine bypass for a standalone proxy where the session token is the *only* boundary: a client bound to a non-loopback `--listen` with auth left on (no `--no-auth`) could tunnel arbitrary CONNECT traffic through unauthenticated, defeating the very token the banner advertises as the auth boundary.
- **Fix:** Added `ProxyConfig.require_auth: bool` (default `true`, gates `reverse.rs`'s two checks and `external.rs`'s enterprise-proxy-chain check) and `ProxyConfig.strict_connect_auth: bool` (default `false`, makes `connect.rs`'s CONNECT check fatal — 407 — instead of only logged). The standalone command sets both to `!args.no_auth`. Both defaults preserve the fork's pre-existing behavior exactly, so `nono run`/`shell`/`wrap` (which never touch these new fields beyond their `Default` values) are unaffected — confirmed by all 218 pre-existing `nono-sandbox-proxy` unit tests passing unchanged.
- **Files modified:** `crates/nono-proxy/src/config.rs`, `crates/nono-proxy/src/connect.rs`, `crates/nono-proxy/src/external.rs`, `crates/nono-proxy/src/reverse.rs`, `crates/nono-proxy/src/server.rs`.
- **Verification:** 218/218 `nono-sandbox-proxy` unit tests pass; both cross-target clippy gates GREEN; the 2 new Wave-0 tests confirm the fail-secure guard.
- **Committed in:** `14ed22e5`

**2. [Rule 1 - Bug, caught by mandatory cross-target gate] `external::handle_external_proxy` exceeded clippy's `too_many_arguments` threshold**
- **Found during:** Task 3's cross-target linux-gnu clippy gate (`cross clippy --all-targets`) — invisible to the Windows-host build, which has no C linker for `aws-lc-sys`/`ring` and cannot run clippy on this crate at all.
- **Issue:** Adding `require_auth: bool` to `handle_external_proxy` brought its argument count to 8, one over clippy's default `too_many_arguments` threshold of 7 (`-D warnings` promotes this to a hard error under the project's gate).
- **Fix:** Introduced `ExternalProxyCtx<'a>` bundling `filter`/`session_token`/`audit_log`/`require_auth`, mirroring the existing `ReverseProxyCtx` pattern already used by `reverse.rs`. Function signature drops to 5 args.
- **Files modified:** `crates/nono-proxy/src/external.rs`, `crates/nono-proxy/src/server.rs`.
- **Verification:** Both cross-target clippy gates re-run GREEN after the fix; `cargo build --workspace --all-targets` and all `nono-sandbox-proxy` tests still pass.
- **Committed in:** `ad8132e7`

**3. [Scope note, not a defect] `route.rs` and `token.rs` required no code changes**
- **Found during:** Task 2's `read_first` step, comparing upstream's diffstat (`route.rs` +4/-2, `token.rs` +47) against the scoped adaptation this plan implements.
- **Issue:** The plan's `files_modified` frontmatter lists both files (carried from the upstream diffstat), but upstream's `route.rs` hunk is part of the route-diagnostics-collapse QoL feature (not ported — out of scope per the plan's own Task 2 action: "skip any hunk that references tls_intercept or the absent Intent types" and the general adapt-only-what's-needed instruction) and `token.rs`'s +47 lines implement the `--pass` custom-password flag (also explicitly omitted — see `key-decisions`).
- **Fix:** No code change made to either file; documented here rather than silently diverging from the plan's file list.
- **Files modified:** none.
- **Verification:** N/A (no-op).
- **Committed in:** N/A (not applicable — nothing to commit).

---

**Total deviations:** 3 (2 auto-fixed under Rules 1/2, 1 scope-note explaining an unmodified-but-listed file pair)
**Impact on plan:** All `must_haves` truths and artifacts are satisfied. The auth-mode addition (Deviation 1) is additive security-hardening beyond the plan's literal per-file diff enumeration, not a departure from its intent — the plan's own `<security_note>` explicitly asked for exactly this kind of disclosure if the adaptation left an enforcement point weaker than upstream's design; instead, it is now *stronger* than the naive default-everything-lenient adaptation would have been.

## Issues Encountered

None blocking. The linux-gnu Docker image layer was already cached from a prior phase's use; both cross-target gates completed quickly (linux-gnu ~1m09s, apple-darwin ~16s) on the second (post-fix) run.

## Security Note (per plan's `<security_note>`)

The plan's `<security_note>` asked: if dropping `apply_tls_intercept_config` leaves any enforcement point weaker than the in-sandbox proxy path, say so plainly rather than shipping it silently.

**`apply_tls_intercept_config` itself:** no weakening. This fork has no TLS-interception module at all (`ProxyHandle::intercept_ca_path()` always returns `None`, confirmed via grep) — there is no MITM enforcement point in this fork for that function to have configured, on the standalone path or the sandboxed path. Dropping the call is a no-op relative to the fork's actual capabilities, not a regression.

**A separate, real gap this plan found and closed (not requested by the plan text, but directly in scope of "don't ship weaker than upstream's design silently"):** before this plan, the fork's CONNECT-tunnel auth check was unconditionally lenient — by design for the sandboxed-run world, but a real bypass for a standalone proxy with no OS sandbox behind it. A `nono proxy` bound to a non-loopback `--listen` with the default auth-on posture (no `--no-auth`) would have advertised a session token as the auth boundary while `CONNECT` tunnels silently bypassed it. This is now closed via `ProxyConfig.strict_connect_auth`, defaulting to `true` (i.e., enforced) for every standalone invocation except explicit `--no-auth`. See Deviation 1 above for the full trace.

## User Setup Required

None — no external service configuration, no new runtime dependencies added to `Cargo.lock` beyond the two `tokio` feature flags (`signal`, `time`), both part of the already-vendored `tokio` crate.

## Next Phase Readiness

- SEC-07 is functionally complete; `REQUIREMENTS.md`'s SEC-07 checkbox is being marked complete by this plan (sole contributing plan per the disposition table's "own plan, own wave" sizing).
- No blockers for subsequent Phase 112 plans. `proxy_runtime.rs`'s `parse_allow_domain_arg`/`parse_allow_endpoint_arg` are now `pub(crate)` (were private) — any future plan touching those functions should be aware they now have a second caller (`proxy_command.rs`).

---
*Phase: 112-security-residual-sync*
*Completed: 2026-08-05*

## Self-Check: PASSED

Created/modified files confirmed present on disk: `crates/nono-cli/src/proxy_command.rs`,
`crates/nono-cli/tests/proxy_command_run.rs`,
`.planning/phases/112-security-residual-sync/112-07-SUMMARY.md`. Task commits
`00d45c67` (Task 1), `14ed22e5` (Task 2), `ad8132e7` (Task 3), and `d83f9842`
(this SUMMARY) all confirmed present in `git log --oneline --all`.
