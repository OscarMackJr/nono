---
phase: 109-proxy-network-absorb
plan: 01
subsystem: proxy-network
tags: [deny-domain, host-filter, proxy-config, adr-108, fail-closed, cli-flags, profile-schema]

# Dependency graph
requires:
  - phase: 108-upst12-divergence-audit
    provides: "ADR-108 (deny_domain disposition = ADAPT) and the 108-DIVERGENCE-LEDGER NET-cluster per-commit table this plan implements"
provides:
  - "HostFilter/ProxyFilter/ProxyConfig caller-supplied deny mechanism (deny_suffixes, with_denied_hosts) evaluated before the allowlist"
  - "--deny-domain CLI flag + profile deny_domain key, threaded end-to-end into ProxyConfig.denied_hosts"
  - "validate_deny_domain_requires_allow_domain fail-closed guard wired into both nono run call sites"
affects: [109-02, 109-03, 109-04, 109-05, 112, 113]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CLI-side fail-closed policy validator mirroring validate_block_net_conflicts's exact shape, called from both nono-run entry points"
    - "Defense-in-depth: two independent layers (parse-time hard error + strict_filter auto-selection) enforce the same invariant"

key-files:
  created: []
  modified:
    - crates/nono/src/net_filter.rs
    - crates/nono-proxy/src/config.rs
    - crates/nono-proxy/src/filter.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/network_policy.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/profile_runtime.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/command_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - crates/nono-cli/data/profile-authoring-guide.md
    - docs/cli/usage/flags.mdx

key-decisions:
  - "ADR-108 ADAPT posture implemented literally: deny_domain is an additional deny layer evaluated before the allowlist, never an allowlist substitute, and never activates the proxy on its own (Consequence (c))."
  - "strict_filter defense-in-depth (build_proxy_config) computed AFTER the full allowlist (resolved policy + extra_hosts) is assembled, not before — an earlier ordering bug made a legitimate allow+deny config falsely trip strict_filter; caught by the new deny+allow_domain propagation test before commit."
  - "proxy.strict_filter now ORs with (never overwrites) build_proxy_config's own strict-selection in build_proxy_config_from_flags, so the ADR-108 (b) defense-in-depth layer isn't silently discarded by the block-net/profile-network-block strict flag."

patterns-established:
  - "A guard on only one of two nono-run entry points is a bypass, not a guard (D-04) — validate_deny_domain_requires_allow_domain is wired into both command_runtime.rs's dry-run branch and launch_runtime.rs's real launch path, mirroring validate_block_net_conflicts."

requirements-completed: [NET-01]

# Metrics
duration: ~2h
completed: 2026-07-29
---

# Phase 109 Plan 01: deny_domain absorb (#1374, ADR-108 ADAPT) Summary

**Absorbed upstream `deny_domain` (#1374) as a fail-closed, deny-before-allow layer that composes with `allow_domain` and never auto-activates the proxy or falls back to allow-all — the exact security gap ADR-108 found in upstream's verbatim design.**

## Performance

- **Duration:** ~2h
- **Completed:** 2026-07-29
- **Tasks:** 3/3 completed
- **Files modified:** 17 (16 from the plan's `files_modified` list + `.planning/phases/109-proxy-network-absorb/deferred-items.md`)

## Accomplishments

- `HostFilter`/`ProxyFilter`/`ProxyConfig` gained a caller-supplied deny mechanism (`deny_suffixes`, `with_denied_hosts()`), additive to the existing cloud-metadata `deny_hosts`/`DENY_HOSTS`/`FilterResult::Deny*` family, evaluated in `check_host()` before the allowlist.
- `--deny-domain` CLI flag and profile `deny_domain` key threaded end-to-end through every layer `allow_domain` already flows through (`SandboxArgs`, `NetworkConfig`, `PreparedSandbox`, `PreparedProfile`, `EffectiveProxySettings`, `ProxyLaunchOptions`) into `ProxyConfig.denied_hosts`.
- `validate_deny_domain_requires_allow_domain` (D-04/D-05): a deny-only configuration is a hard parse-time error, wired into both `nono run` entry points (`command_runtime.rs` dry-run branch, `launch_runtime.rs` real launch path), naming the fork's deliberate divergence from upstream's auto-activating default-allow semantics.
- ADR-108 Consequence (c) made explicit: `deny_domain` is deliberately excluded from `prepare_proxy_launch_options`'s `would_activate` OR-chain, documented with a `DELIBERATELY` doc comment and two dedicated tests.
- `--config` (capability manifest mode) now rejects `--deny-domain`, closing the exact asymmetry class the plan's revision found: `deny_proxy` added to the `conflicts_with_all` array, `has_proxy_flags()`, `has_proxy_intent()`, and the `print_allow_domain_port_warnings` pairing.

## Task Commits

Each task was committed atomically:

1. **Task 1: Library + proxy-layer deny mechanism** - `09179b5f` (feat)
2. **Task 2: CLI flag, profile schema, and end-to-end plumbing** - `a468c04b` (feat)
3. **Task 3: Fail-closed guard — both call sites (D-04) + parse-time hard error (D-05)** - `f7691a15` (feat)
4. **Follow-up: cross-target clippy fix** - `f0989a09` (fix) — surfaced by the mandatory linux-gnu cross-target clippy gate, not a plan task; see Deviations.

_No plan-metadata-only commit yet; this SUMMARY + STATE/ROADMAP updates land in the orchestrator's final commit._

## Files Created/Modified

- `crates/nono/src/net_filter.rs` — `HostFilter.deny_suffixes` field, `with_denied_hosts()` builder, `check_host()` step 1b, 6 ported unit tests.
- `crates/nono-proxy/src/filter.rs` — `ProxyFilter::with_denied_hosts()`, 2 tests.
- `crates/nono-proxy/src/config.rs` — `ProxyConfig.denied_hosts` field.
- `crates/nono-proxy/src/server.rs` — chains `.with_denied_hosts(&config.denied_hosts)` onto the filter-construction site.
- `crates/nono-cli/src/cli.rs` — `--deny-domain` flag (`deny_proxy: Vec<String>`), `NONO_DENY_DOMAIN` env var, `--config`'s `conflicts_with_all` gains `deny_proxy`, `has_proxy_flags()` mirror, 2 new parse-conflict tests.
- `crates/nono-cli/src/profile/mod.rs` — `NetworkConfig.deny_domain: Vec<String>`, merged via `dedup_append`; 3 new tests.
- `crates/nono-cli/src/network_policy.rs` — `build_proxy_config()` gains `denied_hosts` param + ADR-108 (b) `strict_filter` auto-selection; new `expand_proxy_deny()`; 2 new tests.
- `crates/nono-cli/src/sandbox_prepare.rs` — `PreparedSandbox.deny_domain`, `has_proxy_intent()` mirror, port-warning pairing, new `validate_deny_domain_requires_allow_domain()` + 5 tests.
- `crates/nono-cli/src/profile_runtime.rs` — `PreparedProfile.deny_domain`.
- `crates/nono-cli/src/proxy_runtime.rs` — `EffectiveProxySettings.deny_domain`, `ProxyLaunchOptions.deny_domain` threading, `would_activate` doc comment (ADR-108 (c)), 4 new tests.
- `crates/nono-cli/src/launch_runtime.rs` — `ProxyLaunchOptions.deny_domain` field; validator call site.
- `crates/nono-cli/src/command_runtime.rs` — validator call site (dry-run branch).
- `crates/nono-cli/src/main.rs` — struct-literal completeness for new fields.
- `crates/nono-cli/data/nono-profile.schema.json` — `deny_domain` schema key.
- `crates/nono-cli/data/profile-authoring-guide.md` / `docs/cli/usage/flags.mdx` — documentation.
- `.planning/phases/109-proxy-network-absorb/deferred-items.md` — new; logs 11 pre-existing, out-of-scope test failures found during the regression run (see Deviations).

## Decisions Made

- **strict_filter ordering fix (Rule 1):** `build_proxy_config`'s ADR-108 (b) defense-in-depth check must run *after* `extra_hosts` (which carries the fork's `allow_domain`-derived plain hosts) is merged into `allowed_hosts`, not before. The initial implementation checked before the merge, which falsely forced `strict_filter: true` on any config with `allow_domain` but no `network_profile` (since `resolved.hosts` alone was empty at that point). Caught by `build_proxy_config_propagates_deny_domain_with_allow_domain` before commit.
- **strict_filter OR, not overwrite:** `build_proxy_config_from_flags` previously did `proxy_config.strict_filter = proxy.strict_filter;` unconditionally, which would have silently discarded `build_proxy_config`'s own ADR-108 (b) strict-selection. Changed to `proxy_config.strict_filter = proxy.strict_filter || proxy_config.strict_filter;`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] strict_filter computed before extra_hosts merge, falsely tripping on legitimate allow_domain configs**
- **Found during:** Task 2, while writing `build_proxy_config_propagates_deny_domain_with_allow_domain`
- **Issue:** `build_proxy_config`'s ADR-108 (b) `strict_filter` auto-selection checked `allowed_hosts.is_empty()` before `extra_hosts` (the fork's actual `allow_domain` carrier for plain hosts) was merged in, so any deny+allow_domain config with no `network_profile` would incorrectly force `strict_filter: true`.
- **Fix:** Moved the `strict_filter` computation to after `allowed_hosts.extend(extra_hosts...)`.
- **Files modified:** `crates/nono-cli/src/network_policy.rs`
- **Verification:** New test `build_proxy_config_propagates_deny_domain_with_allow_domain` (proxy_runtime.rs) and `test_build_proxy_config_deny_only_sets_strict_filter` (network_policy.rs) both pass; both cover the boundary this bug lived on.
- **Commit:** `a468c04b`

**2. [Rule 1 - Bug] proxy.strict_filter overwrite discarding ADR-108 (b) defense-in-depth**
- **Found during:** Task 2, same investigation as above
- **Issue:** `build_proxy_config_from_flags` unconditionally overwrote `proxy_config.strict_filter` with `proxy.strict_filter` (block-net / profile network.block), which would silently discard `build_proxy_config`'s own deny-only strict-selection whenever `proxy.strict_filter` was `false`.
- **Fix:** Changed to `proxy_config.strict_filter = proxy.strict_filter || proxy_config.strict_filter;` — the two independent layers now compose instead of one clobbering the other.
- **Files modified:** `crates/nono-cli/src/proxy_runtime.rs`
- **Verification:** `test_build_proxy_config_strict_filter_off_when_no_block` still passes (proves the OR doesn't force strict on by default).
- **Commit:** `a468c04b`

**3. [Rule 1 - Bug] clippy::doc_lazy_continuation on check_host doc comment**
- **Found during:** Post-Task-3 mandatory cross-target clippy verification (CLAUDE.md requires this because Task 3 touches files containing `#[cfg(target_os = "linux")]`/`"macos"` blocks — `cli.rs`, `sandbox_prepare.rs`, `profile_runtime.rs`, `proxy_runtime.rs`, `launch_runtime.rs`, `command_runtime.rs`, `main.rs`).
- **Issue:** The `# Check Order` doc comment on `HostFilter::check_host()` used a `1b.` list marker, which rustdoc doesn't recognize as an ordered-list continuation; its continuation line's indentation also didn't match item 1's. `cross clippy --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` failed with `error: doc list item without indentation` (`-D clippy::doc-lazy-continuation` implied by `-D warnings`). This did not surface on the Windows-host `cargo build`/`cargo clippy` runs because the standard lint set doesn't enable `-D warnings` the same way, and per CLAUDE.md a Windows-host run is structurally blind to this class of issue regardless.
- **Fix:** Folded the deny-suffix description into item 1's prose instead of a separate `1b.` list item. `check_host()`'s inline step comments (non-doc, unaffected by this lint) still narrate the "1b." step order.
- **Files modified:** `crates/nono/src/net_filter.rs`
- **Verification:** `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` both GREEN after the fix (both rerun in full; see Verification section).
- **Commit:** `f0989a09`

### Deferred / Out-of-Scope Findings

**11 pre-existing test failures unrelated to this plan**, found while running the full `cargo test -p nono-sandbox-cli --bin nono` regression suite. All are in files this plan's `files_modified` list does not touch (`audit_session.rs`, `config/mod.rs`, `profile_cmd.rs`, `protected_paths.rs`) and match the durably-tracked class of pre-existing Windows-host / parallel-test-execution failures (`env lock: PoisonError` races, and the specific `profile_cmd`/`protected_paths` failures already documented in the project's `nono_cli_windows_baseline_test_failures.md` memory note as "env-specific, fail at phase-base too; don't chase as regressions"). Logged in `.planning/phases/109-proxy-network-absorb/deferred-items.md` per the SCOPE BOUNDARY rule; not fixed.

## Verification

- `cargo build --workspace --all-targets` — exits 0.
- `cargo fmt --all -- --check` — clean.
- `cargo test -p nono-sandbox net_filter` — 30 passed, 0 failed (includes 6 new deny-mechanism tests).
- `cargo test -p nono-sandbox-proxy` — 194 passed, 0 failed (regression gate vs the 192/0 baseline captured before this plan; +2 new tests, 0 new failures).
- `cargo test -p nono-sandbox-cli --bin nono` (targeted modules) — `sandbox_prepare` 15/15, `network_policy` 49/49, `profile::allow_domain_tests` deny_domain tests 3/3, `proxy_runtime` 25/25, `cli::tests` deny_domain tests 2/2 — all 0 failed.
- `cargo test -p nono-sandbox-cli --bin nono` (full suite) — 1402 passed, 11 failed (all pre-existing, out-of-scope; see Deviations), 2 ignored.
- `cargo test -p nono-sandbox --lib` (full) — 808 passed, 0 failed (baseline noted 1 pre-existing `try_set_mandatory_label` failure; not reproduced this run — environment-dependent, not a regression either direction).
- Cross-target clippy (mandatory — Task 3 touches cfg-gated Unix files): `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` GREEN; `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` GREEN (`SDKROOT` unset).
- Acceptance-criteria greps: `with_denied_hosts(&config.denied_hosts)` in `server.rs` (1 hit), `deny_suffixes` in `net_filter.rs` (5 hits), `expand_proxy_deny` in `network_policy.rs` (1 hit), `deny_domain` in the JSON schema (1 hit), `deny_proxy` in `--config`'s `conflicts_with_all` array (1 hit, same array as `allow_proxy`), `validate_deny_domain_requires_allow_domain` in both `command_runtime.rs` and `launch_runtime.rs` (1 hit each, preceding their respective proxy-preparation calls), `DELIBERATELY` near `would_activate` in `proxy_runtime.rs` (1 hit) — all confirmed.
- No `.unwrap()`/`.expect()` introduced in production (non-test) code — confirmed by direct diff review of every non-test hunk.

## Known Stubs

None. `deny_domain` is fully wired end-to-end (library → proxy config → CLI/profile plumbing → fail-closed guard); no placeholder or empty-value stub was introduced.

## Threat Flags

None. All security-relevant surface introduced by this plan (`--deny-domain` flag, `deny_domain` profile key, `ProxyConfig.denied_hosts`, the `validate_deny_domain_requires_allow_domain` guard, the `--config` conflicts_with_all entry) is explicitly covered by the plan's own `<threat_model>` (T-109-01 through T-109-17, T-109-SC). No new network endpoints, auth paths, or schema changes at trust boundaries beyond what the threat model already names.

## Self-Check: PASSED

- FOUND: `.planning/phases/109-proxy-network-absorb/109-01-SUMMARY.md`
- FOUND: `.planning/phases/109-proxy-network-absorb/deferred-items.md`
- FOUND commit: `09179b5f` (Task 1)
- FOUND commit: `a468c04b` (Task 2)
- FOUND commit: `f7691a15` (Task 3)
- FOUND commit: `f0989a09` (cross-target clippy follow-up fix)
