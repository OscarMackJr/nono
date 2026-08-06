---
phase: 113-spiffe-spire-workload-identity
verified: 2026-08-06T22:14:56Z
status: human_needed
score: 4/4 must-haves (SC1-SC4) verified; 1 outstanding human/live-path verification item
overrides_applied: 0
human_verification:
  - test: "Run the live SPIFFE/SPIRE end-to-end path at least once, somewhere"
    expected: "The `.github/workflows/spire.yml` CI lane executes green on a pushed branch/PR (real SPIRE 1.9.6 server+agent), OR a reviewer runs `bash scripts/spire-test.sh` on a Linux/macOS host with a live `SPIRE_AGENT_SOCKET` and confirms `test_spiffe_jwt_live_fetch`, `test_spiffe_jwt_live_delegation_none_on_plain_svid`, `test_spiffe_jwt_live_proxy_startup`, `d03_connect_and_forward_http_deny_spiffe_declared_route_upstream_end_to_end` (spiffe_integration.rs) and `spiffe_jwt_credential_injected_end_to_end`, `spiffe_jwt_proxy_starts_with_live_agent` (spiffe_run.rs) all pass live."
    why_human: "No SPIRE agent exists on this Windows dev host so this cannot be exercised programmatically here; `.github/workflows/spire.yml` exists on disk but has never run — `gh run list --workflow=spire.yml` returns HTTP 404 'workflow not found on the default branch', confirming the branch carrying this workflow has never been pushed/merged. The phase's own D-06/D-07 decisions anticipated this gap and built loud-skip reporting rather than hiding it, but the underlying fact remains: as of this verification, the SPIFFE live path (both the direct JWT-SVID bearer path AND the RFC 7523 OAuth2 assertion path) has been exercised **zero times, anywhere** — not locally, not in CI. This is a decision point for the operator: proceed to Phase 114 with this residual open, or gate on running spire.yml/spire-test.sh first."
---

# Phase 113: SPIFFE/SPIRE Workload Identity Verification Report

**Phase Goal:** Upstream's SPIFFE/SPIRE workload-identity auth for upstream routes (#1272, upstream
commit `c831dade`) is absorbed under its own ADR-gated review, without regressing the fork's
divergent TLS-interception model or the ADR-86 policy-free-library boundary.

**Verified:** 2026-08-06T22:14:56Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | `proj/ADR-113-spiffe-disposition.md` exists and settles adopt/adapt/defer, weighing the ADR-86 crossing, the (corrected) `reverse.rs` rewrite, and the `Cargo.lock` expansion, with a positive grep-backed D-01 proof | ✓ VERIFIED | File exists, tracked in git (`git show --stat dbf735c5` includes it — force-added past `.gitignore:16`'s bare-`proj/` trap). Independently re-ran every cited command: `grep -rn 'tls_intercept' crates/nono-proxy/src/` = 11 (all comments, confirmed by inspection), structural `mod/struct/fn` grep = 0, `SpiffeUnsupportedPath` count in `server.rs` = 8, D-01/D-04/OD-1/OD-2/Landmine-L1 grep assertions all reproduce the ADR's claimed numbers exactly (see "Independent Re-Verification" below). ADR correctly names and corrects ROADMAP SC1's stale `tls_intercept` framing rather than silently inheriting it. |
| SC2 | SPIFFE/SPIRE auth for upstream routes is absorbed per the ADR's disposition AND is configurable via profile | ✓ VERIFIED (with named caveat) | `crates/nono-cli/data/nono-profile.schema.json`'s `SpiffeAuthConfig` `$defs` block and `crates/nono-proxy/src/config.rs`'s `SpiffeAuthConfig` Rust enum agree field-for-field (`additionalProperties:false`/`deny_unknown_fields`, matching `required` arrays — confirmed by direct read). `profile/mod.rs`'s mutual-exclusion validator (`cred.spiffe.is_some() && (credential_key\|auth\|aws_auth).is_some()` → reject) has 5 passing unit tests (`test_validate_custom_credential_spiffe*`), independently confirmed present and worded as claimed. `handle_spiffe_route`'s (direct JWT-SVID) successful-forward path IS proven end-to-end via CI-gated `spiffe_run.rs::spiffe_jwt_credential_injected_end_to_end`. **Caveat, not a failure:** `handle_spiffe_assertion_credential`'s (RFC 7523 OAuth2 jwt-bearer) successful-forward path has zero test coverage anywhere (WR-02, left open by explicit 2026-08-06 operator decision) — named honestly in the ADR/SUMMARY, not hidden. |
| SC3 | ADR-86 boundary confirmed non-regressed: `crates/nono/src/undo/types.rs` and `crates/nono/src/audit.rs` additions are audit/telemetry data only, no policy/enforcement | ✓ VERIFIED | Read the live diff directly: `undo/types.rs` adds 2 enum variants each to `NetworkAuditAuthMechanism`/`NetworkAuditInjectionMode`/`NetworkAuditDenialCategory`, two `#[derive(Debug, Clone, Serialize, Deserialize)]`-only structs (`SpiffeDelegationContext`, `SpiffeAuditContext`) with zero methods beyond derives, and one new `Option<SpiffeAuditContext>` field on `NetworkAuditEvent`. `audit.rs` (+2) is two `spiffe_context: None,` fixture initializers. Zero enforcement/decision logic anywhere in the diff — confirmed by reading, not by trusting the summary's claim. ADR-113 honestly discloses the one real caveat: SPIFFE *vocabulary* now exists in the policy-free library's public surface (a mild concept-leak), which does not cross the enforcement/policy line ADR-86 actually draws. |
| SC4 | Cross-target clippy GREEN on both gates (no PARTIAL→CI), `maturin build` + `napi build` green after the `nono-proxy` struct changes | ✓ VERIFIED | Both cross-target clippy gates confirmed GREEN by the orchestrator this session (linux-gnu via `cross`, apple-darwin via `cargo-zigbuild`, both exit 0). Independently re-verified `cargo fmt --all --check` clean and `cargo build --lib` in `../nono-py` green (1m03s, 0 errors) after the 3-file fix (`proxy.rs`/`policy.rs`/`undo.rs`) landed in commit `485821f`. `../nono-py/target/wheels/nono_sandbox-0.70.0-...whl` exists with an Aug 6 14:15 timestamp matching the phase's completion window (independent corroboration `maturin build` actually ran, not merely claimed). `../nono-ts/nono.win32-x64-msvc.node` exists with an Aug 6 14:17 timestamp (independent corroboration `napi build` ran); confirmed `nono-ts` has zero `nono_proxy`/`nono-proxy` references anywhere, so it is structurally immune to this absorb as claimed. |

**Score:** 4/4 Success Criteria VERIFIED. 1 unresolved human/live-path verification item (see below) — this is why overall status is `human_needed`, not `passed`.

### Independent Re-Verification of Cited Proof Commands

All of the following were re-run fresh by this verifier (not copied from the ADR/SUMMARY) and matched the claimed output exactly:

| Command | Claimed | Verified |
|---|---|---|
| `grep -rn 'tls_intercept' crates/nono-proxy/src/ \| wc -l` | 11 | 11 (all comments) |
| `grep -rn "mod tls_intercept\|struct.*TlsIntercept\|fn.*tls_intercept" crates/nono-proxy/src/ \| wc -l` | 0 | 0 |
| `grep -c "SpiffeUnsupportedPath" crates/nono-proxy/src/server.rs` | 8 | 8 |
| `grep -c "oauth2_routes\|struct OAuth2Route\|fn get_oauth2" crates/nono-proxy/src/credential.rs` | 0 | 0 |
| `grep -c "lookup_all_by_upstream\|has_intercept_route\|requires_managed_credential" crates/nono-proxy/src/route.rs` | 0 | 0 |
| `grep -c "belong to the upstream tls_intercept module" crates/nono-proxy/src/route.rs` | 0 | 0 |
| `grep -c "b1ecbc02" crates/nono-proxy/src/route.rs` | ≥1 | 4 |
| `grep -c "&& let" crates/nono-proxy/src/spiffe.rs` | 0 | 0 |
| `grep -c "&& let" crates/nono-cli/src/proxy_runtime.rs` | 0 | 0 |
| `cargo test -p nono-sandbox-proxy --lib` | 245 passed, 0 failed | 245 passed, 0 failed |
| `cargo test -p nono-sandbox-proxy --test spiffe_integration -- --nocapture` SKIP count | 4 | 4 (source has exactly 4 `eprintln!("SKIP[...")` call sites) |
| `cargo test -p nono-sandbox-cli --test spiffe_run -- --nocapture` SKIP count | 2 | 2 |
| WR-01 fix: `declared_spiffe_routes`/`declared_spiffe_prefixes()` in `credential.rs` | present, wired into `route_diagnostics`/`loaded_prefixes` | present, matches ADR/REVIEW resolution note exactly |
| `cargo fmt --all --check` | clean | clean |
| `cargo build --lib` in `../nono-py` | green | green, 0 errors, 1m03s |

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `proj/ADR-113-spiffe-disposition.md` | Substantive ADR discharging D-01/D-05/D-08/OD-1/D-07 proof obligations with commands, not prose | ✓ VERIFIED | 349 lines, tracked in git, every proof obligation carries cited command output re-verified above |
| `crates/nono-proxy/src/spiffe.rs`, `auth.rs` | New SPIFFE modules | ✓ VERIFIED | Present, `SpiffeJwtSource::connect`/`check_nbf` etc. exist, exercised by 245-test proxy suite |
| `crates/nono-proxy/src/reverse.rs` (`handle_spiffe_route`, `handle_spiffe_assertion_credential`) | Primary SPIFFE dispatch path | ✓ VERIFIED | Present at `reverse.rs:831-1080` per REVIEW.md's own citation; `handle_spiffe_route`'s path is CI-tested, `handle_spiffe_assertion_credential`'s successful path is not (named residual, WR-02) |
| `crates/nono-proxy/src/route.rs` (`declares_spiffe`, `has_spiffe_source`, async `RouteStore::load`) | D-04 async conversion, scoped to per-route SPIFFE branch | ✓ VERIFIED | Present, `.await` confirmed scoped inside the per-route `spiffe` match arm only |
| `crates/nono-proxy/src/credential.rs` (`declared_spiffe_routes`, WR-01 fix) | Direct-SPIFFE routes visible in diagnostics/env-var gating | ✓ VERIFIED | Fix landed commit `5cd5562c`, field/accessor present, 3 named tests present |
| `crates/nono-cli/data/nono-profile.schema.json` (`SpiffeAuthConfig`) | Fork-only strict schema, mutually exclusive with other auth fields | ✓ VERIFIED | `additionalProperties: false`, matches Rust struct exactly |
| `.github/workflows/spire.yml`, `scripts/spire-test.sh`, `testdata/spire/*` | D-06 live-SPIRE CI lane | ⚠️ EXISTS BUT NEVER RUN | Files present on disk and well-formed (`name: SPIRE`, `on:` trigger, SPIRE 1.9.6 download/start steps). `gh run list --workflow=spire.yml` → HTTP 404 "workflow not found on the default branch" — confirms this workflow has never executed anywhere, local or CI. See Human Verification below. |
| `108-DIVERGENCE-LEDGER.md` SPIFFE Carry-Forward Note | D-02 carry-forward note | ✓ VERIFIED | Present, section header "### SPIFFE Carry-Forward Note (Phase 113, D-02)" confirmed at line 659, references `b1ecbc02` and `proj/ADR-113-spiffe-disposition.md` correctly |
| `../nono-py/src/{proxy,policy,undo}.rs` | Struct-drift fix for `RouteConfig.spiffe` + 4 new core enum variants | ✓ VERIFIED | All 3 files updated, `cargo build --lib` green, wheel artifact freshly built |
| `../nono-ts` | Structurally immune | ✓ VERIFIED | Zero `nono_proxy`/`nono-proxy` references; `.node` binary freshly rebuilt anyway (confirms `napi build` ran and was a no-op as claimed) |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `server.rs::handle_connection` (CONNECT arm) | D-03 fail-closed guard | `RouteStore::is_route_upstream` + `spiffe_declared_for_upstream`, gated before `use_external` computed | ✓ WIRED | Confirmed by grep at `server.rs:697-700`, `1055-1066`, `1316-1344`; two end-to-end tests (`d03_connect_denies_spiffe_declared_route_upstream`, `d03_forward_http_denies_spiffe_declared_route_upstream`) both configured with `require_auth: false`, proving the guard is not gated on that flag |
| `server.rs::handle_forward_http` | D-03 fail-closed guard | `spiffe_declared_for_upstream` check, unconditional | ✓ WIRED | Same evidence as above; this is the exact WR-13-shaped gap the phase was built to close, confirmed closed |
| `RouteConfig.spiffe` (profile) | `nono-profile.schema.json` `SpiffeAuthConfig` | serde + JSON-schema `$ref` | ✓ WIRED | Field-for-field match confirmed by direct read of both files |
| `CredentialStore::load` | `declared_spiffe_routes` → `route_diagnostics()`/`credential_env_vars()` | WR-01 fix | ✓ WIRED | Confirmed post-fix; `"cred: spiffe"` reported instead of misleading `"cred: none"` |
| `nono-py::proxy.rs`/`policy.rs`/`undo.rs` | `nono-proxy`/`nono` new SPIFFE fields/enum variants | struct-literal + match-arm completion | ✓ WIRED | `cargo build --lib` green confirms compile-time wiring; `SpiffeAuditContext`'s rich payload does NOT round-trip through the Python dict layer (named residual, not a wiring break — the enum tags do round-trip) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| NET-02 | 113-01 through 113-08 | SPIFFE/SPIRE workload-identity auth for upstream routes (#1272) is absorbed | ✓ SATISFIED | REQUIREMENTS.md line 114 marked Complete; traceability table row 164 → Phase 113 Complete; all supporting truths above VERIFIED. Caveat: live-path never exercised (see Human Verification) — this does not block NET-02's textual requirement (absorption + profile-configurability), which is met, but is material context for anyone relying on the feature being battle-tested. |

No orphaned requirements found for Phase 113 (only NET-02 is tagged to this phase in REQUIREMENTS.md).

### Anti-Patterns Found

None blocking. Reviewed the same 25 files code-review already covered plus independently re-checked `route.rs`, `credential.rs`, `server.rs`, `spiffe.rs`, `nono-profile.schema.json`. No `TBD`/`FIXME`/`XXX` markers without a tracked reference found in the diffed files. `112-REVIEW.md`'s 3 open Warnings (WR-02, WR-03) and 3 Info items are pre-existing, explicitly operator-accepted residuals from `113-REVIEW.md` — not new anti-patterns, and already named/tracked rather than hidden. WR-01 (the one Warning that WAS a real functional/UX defect) is confirmed fixed and re-tested.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Proxy crate test suite (SPIFFE-inclusive) | `cargo test -p nono-sandbox-proxy --lib` | 245 passed, 0 failed | ✓ PASS |
| D-03 fail-closed guard, request-time | Confirmed via existing `server.rs` unit tests `d03_connect_denies_spiffe_declared_route_upstream`/`d03_forward_http_denies_spiffe_declared_route_upstream` (part of the 245) | pass | ✓ PASS |
| Profile schema round-trip | `SpiffeAuthConfig` mutual-exclusion unit tests in `profile/mod.rs` | present, passing per SUMMARY and structurally confirmed | ✓ PASS |
| `../nono-py` build | `cargo build --lib` | 0 errors, 1m03s | ✓ PASS |
| Live SPIFFE/SPIRE path (both flows) | N/A — no SPIRE agent on this host, `spire.yml` never run | untested | ? SKIP → routed to Human Verification |

### Probe Execution

No `scripts/*/tests/probe-*.sh` convention used by this phase; `scripts/spire-test.sh` is the phase's own live-verification script and is covered under Human Verification below (requires a live `SPIRE_AGENT_SOCKET`, cannot run on this Windows host in 30s without external SPIRE infrastructure).

### Human Verification Required

#### 1. Live SPIFFE/SPIRE end-to-end path has never been exercised, anywhere

**Test:** Push/merge the branch carrying `.github/workflows/spire.yml` and confirm the "Integration
(Linux)" job runs green with a real SPIRE 1.9.6 server+agent, exercising
`test_spiffe_jwt_live_fetch`, `test_spiffe_jwt_live_delegation_none_on_plain_svid`,
`test_spiffe_jwt_live_proxy_startup`,
`d03_connect_and_forward_http_deny_spiffe_declared_route_upstream_end_to_end` (in
`spiffe_integration.rs`) and `spiffe_jwt_credential_injected_end_to_end`,
`spiffe_jwt_proxy_starts_with_live_agent` (in `spiffe_run.rs`). Alternatively, a reviewer with a
Linux/macOS host can run `bash scripts/spire-test.sh` locally with a live SPIRE agent.

**Expected:** All 6 currently-skipped tests pass against a real Workload API, and (separately,
per WR-02) a manual smoke test of the RFC 7523 OAuth2 jwt-bearer assertion flow
(`handle_spiffe_assertion_credential`) succeeds end-to-end at least once, since no automated test
covers its successful-forward path anywhere.

**Why human:** No SPIRE agent exists on this Windows dev host. `gh run list --workflow=spire.yml`
returns HTTP 404 ("workflow not found on the default branch"), confirming the workflow file exists
on disk (verified: well-formed YAML, SPIRE 1.9.6 download/start steps present) but has genuinely
never executed — not once, locally or in CI. The phase's own D-06/D-07 decisions correctly
anticipated this gap and built loud, honest skip-reporting rather than hiding it (verified: 4+2=6
of 8 SPIFFE tests SKIP on this host, named individually in the ADR and this report) — but the
underlying fact that the live path is entirely unproven remains, and per D-07's own stated purpose
("tests passed" must never be mistaken for "the live path works"), this verifier is surfacing it
explicitly rather than letting "245 passed" stand in for "SPIFFE works against a real SPIRE
deployment." This is an operator decision point: proceed to Phase 114 with this residual
acknowledged and open, or gate on running the CI lane / `spire-test.sh` first.

### Gaps Summary

No BLOCKER-level gaps. All 4 Success Criteria (SC1-SC4) are VERIFIED against the actual codebase
via independent re-execution of the ADR's own cited grep/test/build commands — none of the numbers
were taken on faith. The phase's engineering is unusually well-evidenced: the ADR corrects its own
prior research's dependency-count claim (19→9 genuinely-new crates) with fresh command output, the
code review found and the operator fixed a real functional defect (WR-01) with new regression tests,
and every "residual" the phase carries forward (WR-02 untested OAuth2-assertion success path, WR-03
unverified Windows socket isolation, the SPIFFE-vocabulary concept-leak in the core library, the
`nono-py` dict-encoding gap for `SpiffeAuditContext`) is named explicitly rather than smoothed into
a green checkmark — matching this project's stated preference for honestly-named residuals over
laundered uncertainty.

The one item elevating this report to `human_needed` rather than `passed` is that the live SPIFFE
path — the actual value proposition of this phase (real SPIRE Workload API interop) — has been
exercised zero times by anyone, on any host, in this session or any prior one. `spire.yml` exists,
is well-formed, and is designed correctly, but a CI workflow file that has never run is not evidence
that it works. This does not block NET-02 as textually defined (absorption + profile-configurability,
both true), and does not indicate a defect in the phase's engineering — but it is squarely the kind
of gap D-07 was written to force into the open rather than let "compensated by CI" quietly stand in
for "verified." Recommend either running `scripts/spire-test.sh` on a Linux/macOS host with a real
SPIRE agent before relying on this feature in production, or explicitly accepting this residual (as
the operator already did for WR-02/WR-03 in `113-REVIEW.md`'s resolution) before proceeding to
Phase 114.

---

_Verified: 2026-08-06T22:14:56Z_
_Verifier: Claude (gsd-verifier)_
