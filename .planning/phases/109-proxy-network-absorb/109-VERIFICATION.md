---
phase: 109-proxy-network-absorb
verified: 2026-07-30T03:05:11Z
status: passed
score: 9/9 must-haves verified
overrides_applied: 0
---

# Phase 109: Proxy/Network Absorb Verification Report

**Phase Goal:** The v0.67–v0.69 proxy/network features are absorbed into the fork's proxy without
regressing its fork-divergent TLS-interception + allowlist model, with the bindings rebuilt.

**Verified:** 2026-07-30T03:05:11Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `deny_domain` (#1374) is wired into the proxy filter + profile schema and composes with `allow_domain` without weakening default-deny (ROADMAP SC1, NET-01) | ✓ VERIFIED | `crates/nono/src/net_filter.rs:224-278` — deny checked at steps 1/1b, strictly before the allowlist (steps 3-6); ADR-108 (b) defense-in-depth confirmed via `strict_filter` selection in `network_policy.rs:324` and test `test_build_proxy_config_deny_only_sets_strict_filter` (`network_policy.rs:785`), which PASSES live. |
| 2 | D-04: the fail-closed guard is called from BOTH `nono run` entry points, not just one | ✓ VERIFIED | `grep` confirms `validate_deny_domain_requires_allow_domain(...)` called at `command_runtime.rs:155` (dry-run branch) AND `launch_runtime.rs:379` (real launch path); function body at `sandbox_prepare.rs:294`. |
| 3 | D-05: a deny-only profile (`deny_domain` with no `allow_domain`) is a hard parse-time error, not a silent fallback | ✓ VERIFIED | `sandbox_prepare.rs:301-313` returns `NonoError::ConfigParse` naming the fork's upstream divergence; live test run: `deny_domain_without_allow_domain_errors`, `deny_domain_from_profile_without_allow_domain_errors` both PASS (`cargo test -p nono-sandbox-cli --bin nono deny_domain` → 12/12 passed, re-run independently). |
| 4 | The `--config` asymmetry is closed: `--config` + `--deny-domain` is rejected, not silently ignored | ✓ VERIFIED | `cli.rs:2208` — `deny_proxy` present in `--config`'s `conflicts_with_all` array (same array as `allow_proxy`); `has_proxy_flags()` (`cli.rs:2244`) and `has_proxy_intent()` (`sandbox_prepare.rs:184`) both mirror `deny_domain`; live test `config_and_deny_domain_are_rejected_together` PASSES. |
| 5 | D-06: all six `no_proxy` overlap/grammar validators exist and are reachable from real call paths (direct profile, `extends`-inherited, group-expanded), not just defined | ✓ VERIFIED | Proxy-crate: `validate_no_proxy_entry` (`config.rs:142`), `no_proxy_entry_overlaps_host_pattern` (`config.rs:240`), `bare_single_label_suffix_overlaps_host` (`config.rs:281`). CLI-crate: `validate_profile_no_proxy` called at 3 sites — `parse_profile_bytes` (`profile/mod.rs:2996`), `parse_profile_file` (`:3063`), `finalize_profile` (`:2891`, the extends-merged case); `validate_proxy_launch_no_proxy_conflicts`/`validate_expanded_proxy_no_proxy_conflicts` both called in `build_proxy_config_from_flags` (`proxy_runtime.rs:348,397`). Named regression tests `test_network_no_proxy_rejects_allow_domain_overlap`, `test_finalize_profile_rejects_inherited_no_proxy_allow_domain_overlap`, `test_build_proxy_config_rejects_group_expanded_no_proxy_overlap` all PASS live (`cargo test ... no_proxy` → 9/9 passed). |
| 6 | D-07: `localhost`/`127.0.0.1` still reach the child's `no_proxy` env var after the `push_no_proxy_entry` pipeline replacement — proven by a named test, not assumed | ✓ VERIFIED | `no_proxy_pipeline_preserves_localhost_and_loopback_bypass` (`server.rs:2295`) asserts both entries present in `NO_PROXY`/`no_proxy`; PASSES live as part of the 218/0 `nono-sandbox-proxy` suite run. |
| 7 | #1335 forward-proxy: `handle_forward_http` enforces the SAME `check_host` decision as the CONNECT path, and does not inject managed credentials | ✓ VERIFIED | Both `connect::handle_connect` (`connect.rs:51`) and `handle_forward_http` (`server.rs:949`) call `.check_host()` on the identical `&state.filter`/`filter: &ProxyFilter` instance passed from `ProxyState`. `classify_request_target` dispatch (`server.rs:1263,1270`) runs BEFORE the `!state.route_store.is_empty()` reverse-proxy branch (`:1279`), closing #1334. `forward_http_does_not_inject_managed_credential` test (`server.rs:1496`) configures an unresolvable-credential route and proves the client's own `Authorization` header survives unmodified with `managed_credential_active=false`; test file confirms all 9 named upstream security-contract tests exist. |
| 8 | ADR-108 fidelity: `deny_domain` is genuinely an additional deny layer, never an allowlist substitute, and never auto-activates the proxy on its own | ✓ VERIFIED | `net_filter.rs` deny-before-allow ordering (Truth 1); `proxy_runtime.rs`'s `would_activate` OR-chain has a `DELIBERATELY`-documented exclusion of `deny_domain` per the 109-01 SUMMARY, confirmed present via grep of `DELIBERATELY` near `would_activate`. |
| 9 | SC4: `maturin build`/`cargo check` (`../nono-py`) and `napi build`/grep (`../nono-ts`) are green after the `nono-proxy` struct changes, with the fix actually committed | ✓ VERIFIED | `../nono-py/src/proxy.rs` contains `denied_hosts: Vec::new()` and `no_proxy: Vec::new()` (lines 394/399); commit `e24c1ff` present in `../nono-py`'s history, DCO-signed, `git status --porcelain` clean. Independently re-ran `cargo check --release` in `../nono-py` in this verification session — **exits 0**, compiles `nono-sandbox-proxy v0.66.1` and `nono-py` cleanly against the live phase-109 tree. `../nono-ts` confirmed to have zero `ProxyConfig`/`nono-proxy` surface (grep, 0 hits) — untracked scratch files present (`decode-test.js` etc.) are dated 2026-06-16, pre-dating this phase, unrelated to proxy structs. |

**Score:** 9/9 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/src/net_filter.rs` | `deny_suffixes` field, `with_denied_hosts()`, deny-before-allow `check_host()` | ✓ VERIFIED | Present, wired, tested (30/30 `net_filter` tests pass live) |
| `crates/nono-proxy/src/{config,filter,server}.rs` | `denied_hosts`/`no_proxy` fields, D-06 validators, `push_no_proxy_entry` pipeline | ✓ VERIFIED | All present, wired into `ProxyHandle`/`start()`; 218/0 live `nono-sandbox-proxy` suite |
| `crates/nono-cli/src/{cli,sandbox_prepare,network_policy,profile/mod,proxy_runtime,launch_runtime,command_runtime}.rs` | `--deny-domain` flag, `validate_deny_domain_requires_allow_domain` at both entry points, `no_proxy` profile plumbing, launch/group-expansion validators | ✓ VERIFIED | All confirmed by direct grep + live test runs (12/12 deny_domain, 9/9 no_proxy) |
| `crates/nono-proxy/src/server.rs` (`handle_forward_http`, `classify_request_target`) | Forward-proxy serving path, dispatched before reverse-proxy route | ✓ VERIFIED | Confirmed ordering; 9 named tests present and passing as part of 218/0 |
| `crates/nono-cli/data/nono-profile.schema.json` | `deny_domain`, `no_proxy` schema keys | ✓ VERIFIED | Both keys present (lines 485, 490) |
| `.planning/phases/109-proxy-network-absorb/109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` | Reviewable N/A disposition for #1430/#1437 | ✓ VERIFIED | Present, re-verifies target-file absence with fresh `ls`/grep evidence, proposes (does not apply) 2 corrections |
| `../nono-py/src/proxy.rs` | `denied_hosts`/`no_proxy` no-op defaults, committed | ✓ VERIFIED | Present; commit `e24c1ff`; `cargo check --release` re-run green in this session |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `command_runtime.rs` dry-run branch | `validate_deny_domain_requires_allow_domain` | direct call, line 155 | ✓ WIRED | Confirmed by grep + code read |
| `launch_runtime.rs` real launch path | `validate_deny_domain_requires_allow_domain` | direct call, line 379 | ✓ WIRED | Confirmed by grep + code read |
| `--config` flag | `--deny-domain` (`deny_proxy`) | `conflicts_with_all` array | ✓ WIRED | `cli.rs:2208` |
| `parse_profile_bytes`/`parse_profile_file`/`finalize_profile` | `validate_profile_no_proxy` | direct call, 3 sites | ✓ WIRED | Confirmed at `profile/mod.rs:2891,2996,3063` |
| `build_proxy_config_from_flags` | `validate_proxy_launch_no_proxy_conflicts` + `validate_expanded_proxy_no_proxy_conflicts` | direct calls | ✓ WIRED | `proxy_runtime.rs:348,397` |
| `build_proxy_config_from_flags` | `ProxyConfig.no_proxy` | `proxy_config.no_proxy = proxy.no_proxy.clone()` | ✓ WIRED | Deviation-fixed in 109-03; without it the feature would validate-and-discard. Confirmed present, and `build_proxy_config_propagates_non_overlapping_no_proxy` proves it live. |
| CONNECT path (`connect::handle_connect`) | `ProxyFilter::check_host` | `&state.filter` | ✓ WIRED | Same filter instance as forward path — no separate/weaker trust boundary |
| Forward-HTTP path (`handle_forward_http`) | `ProxyFilter::check_host` | `state.filter.check_host` | ✓ WIRED | Identical trust boundary, confirmed by code read |
| `classify_request_target` dispatch | `handle_forward_http` / reverse-proxy branch | ordering in `handle_connection` | ✓ WIRED | Absolute-form dispatch (lines 1263/1270) precedes `!state.route_store.is_empty()` (line 1279) — closes #1334 |
| `../nono-py`'s `ProxyConfig::new()` | `nono_sandbox_proxy::ProxyConfig` struct | `RustProxyConfig { denied_hosts: Vec::new(), no_proxy: Vec::new(), ... }` | ✓ WIRED | Re-verified independently: `cargo check --release` in `../nono-py` exits 0 against live tree |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `deny_domain` fail-closed guard (D-04/D-05) actually rejects/accepts as designed | `cargo test -p nono-sandbox-cli --bin nono deny_domain` | 12 passed, 0 failed | ✓ PASS |
| `no_proxy` D-06/D-07 validators + pipeline actually reject/accept/preserve as designed | `cargo test -p nono-sandbox-cli --bin nono no_proxy` | 9 passed, 0 failed | ✓ PASS |
| Full `nono-sandbox-proxy` crate (deny_domain + no_proxy + forward-proxy) | `cargo test -p nono-sandbox-proxy` | 218 passed, 0 failed | ✓ PASS |
| `net_filter` deny-before-allow mechanism | `cargo test -p nono-sandbox net_filter` | 30 passed, 0 failed | ✓ PASS |
| Full `nono-sandbox-cli` regression (baseline check) | `cargo test -p nono-sandbox-cli --bin nono` | 1411 passed, 11 failed (all pre-existing/environmental, see below), 2 ignored | ✓ PASS (baseline matches documented exemption exactly) |
| `../nono-py` compiles against live post-109 `nono-proxy` struct | `cargo check --release` (in `../nono-py`) | exit 0 | ✓ PASS |
| `cargo fmt --all -- --check` | (this repo) | clean | ✓ PASS |
| `git status --porcelain` (`../nono-py`) | clean | ✓ PASS |
| ROADMAP.md / REQUIREMENTS.md / STATE.md unedited by phase-109 commits (except STATE.md's planning record) | `git log --name-only 2f6e768d..HEAD -- .planning/{STATE,ROADMAP,REQUIREMENTS}.md` | only `STATE.md` touched (1d1148f8, planning record) | ✓ PASS |

The 11 pre-existing `nono-sandbox-cli --bin nono` failures (`audit_session.rs`, `config/mod.rs`,
`profile_cmd.rs`, `protected_paths.rs`) were independently re-run in this verification session and
match the exact file set and failure signatures (`env lock: PoisonError`, pre-existing
`protected_paths`/`profile_cmd` baseline) documented in `deferred-items.md` and the project's
durable memory note `nono_cli_windows_baseline_test_failures.md`. None are in files this phase's
`key-files` lists touch. Confirmed pre-existing, not a regression, per the verification context's
explicit exemption.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|--------------|------------|--------------|--------|----------|
| NET-01 | 109-01 | `deny_domain` composes with `allow_domain` without weakening default-deny | ✓ SATISFIED | Truths 1, 2, 3, 4, 8 |
| NET-03 | 109-02, 109-03, 109-04, 109-05 | `no_proxy` bypass surface validated; `HTTP_PROXY` forward-proxy verified non-regressed; SigV4/sibling-route items confirmed N/A with evidence; bindings green | ✓ SATISFIED | Truths 5, 6, 7, 9; `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` records the correctly-scoped N/A disposition for the two absent-subsystem items |

No orphaned requirements: ROADMAP names only NET-01/NET-03 for Phase 109 (NET-02/SPIFFE correctly
moved to Phase 113 per D-01, struck through in ROADMAP SC2).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | No `.unwrap()`/`.expect()` found in production (non-test) code across all 16 files this phase's plans list as modified (`net_filter.rs`, `config.rs`, `route.rs`, `filter.rs`, `server.rs`, `reverse.rs`, `audit.rs`, `cli.rs`, `profile/mod.rs`, `network_policy.rs`, `sandbox_prepare.rs`, `profile_runtime.rs`, `proxy_runtime.rs`, `launch_runtime.rs`, `command_runtime.rs`, `profile_cmd.rs`) — independently verified in this session via per-file cfg(test)-boundary-aware grep. | — | — |
| `crates/nono-proxy/src/server.rs:68,149,648,685` | `managed_loopback_upstream` field always `false` in `start()` | ℹ️ INFO | Documented, non-blocking scaffolding: field IS read (line 149's `if self.managed_loopback_upstream`) and IS tested with both `true` and `false` values (`no_proxy_pipeline_clears_loopback_when_managed_loopback_upstream` exercises the `true` branch directly against `env_vars()`). This is legitimate forward-compatible scaffolding for a future absorb, not hidden dead code — judged acceptable per the verification task's own framing. |
| — | — | No `TBD`/`FIXME`/`XXX` debt markers found in any phase-109-touched file. | — | — |

### Deviations From Plan (auto-fixed, reviewed)

All deviations documented in the five SUMMARYs were spot-checked against live code and found
honest — no deviation silently narrowed the security contract:

- 109-01: `strict_filter` ordering bug + OR-not-overwrite fix — confirmed live via
  `test_build_proxy_config_deny_only_sets_strict_filter` and
  `test_build_proxy_config_strict_filter_off_when_no_block`, both passing.
- 109-02: `managed_loopback_upstream` inert scaffolding — reviewed above, acceptable.
- 109-03: `proxy_config.no_proxy = proxy.no_proxy.clone()` addition (Rule 2, not in plan's stated
  artifacts) — without it the feature would validate-and-discard; confirmed present and tested.
- 109-04: `handle_forward_http` implemented against the fork's own `reverse.rs` primitives instead
  of upstream's absent `forward.rs` abstraction; external-proxy chaining explicitly out of scope —
  confirmed none of the 9 required security-contract tests exercise that path, so the required
  contract (host-filter trust boundary, credential non-injection) is unaffected.
- 109-05: N/A disposition for `#1430`/`#1437` — independently re-verified in this session
  (`ls crates/nono-proxy/src/` — no `aws/` or `tls_intercept/` subdirectory) — genuinely correct,
  not a scope-reduction dressed up as a finding.

### Human Verification Required

None. This phase is entirely backend/library/CLI logic with no UI, no visual rendering, and no
external-service dependency requiring live/manual testing. All required behaviors are covered by
automated tests that were independently re-run (not merely trusted from SUMMARY.md) in this
verification session.

### Gaps Summary

None. All 9 must-have truths (roadmap SC1, SC3, SC4 plus D-04/D-05/D-06/D-07/#1335/ADR-108
sub-truths) verified against live code and live test runs, not SUMMARY.md claims. SC2 (SPIFFE) is
correctly struck through and absent — that absence is the deliberate D-01 scope split to Phase 113,
not a gap. The two proposed-but-unapplied corrections (Phase 108 ledger disposition,
REQUIREMENTS.md NET-03 wording) are explicitly operator-gated per the phase's own design (mirrors
Phase 108 precedent) and do not block phase completion — they are advisory follow-ups, not defects
in Phase 109's delivered code.

---

_Verified: 2026-07-30T03:05:11Z_
_Verifier: Claude (gsd-verifier)_
