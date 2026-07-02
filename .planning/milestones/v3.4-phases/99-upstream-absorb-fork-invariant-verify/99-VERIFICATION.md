---
phase: 99-upstream-absorb-fork-invariant-verify
verified: 2026-06-30T00:00:00Z
status: passed
score: 12/13 must-haves verified
overrides_applied: 0
human_resolution:
  resolved: 2026-06-30
  decision: "Human approved Phase 99 complete via /gsd-execute-phase checkpoint. Resolutions: (WR-01) accepted the dual-version sigstore-trust-root 0.9.0 pin as an intentional deferred-cascade marker (RESEARCH A1); the misleading Cargo.toml comment was CORRECTED inline (commit on 2026-06-30) to state the effective root is 0.8.0 and the 0.9.0 pin is unreferenced — dep-removal deferred to Phase 100. (WR-03) deferred to Phase 100 as a product decision (block:true+allow_domain → error vs strict-mode); fail-closed today, no security impact. (WR-02) deferred to Phase 100 release-notes gate (wire UpstreamPool or add help-text note). (SC3) RESOLVED by orchestrator: both D-08 deviation tests pass; all 11 nono-cli failures are in files Phase 99 never touched (config env-lock flakiness + audit_session/profile_cmd/protected_paths) — not regressions. All deferred items tracked in 99-HUMAN-UAT.md."
human_verification:
  - test: "WR-01 decision: Is the unused sigstore-trust-root 0.9.0 direct dep acceptable per absorb-fidelity scope, or must it be removed before Phase 100?"
    expected: "Either: (a) Remove the direct dep and annotate in comments that the cascade was evaluated + deferred, satisfying CLAUDE.md dead-code rule; OR (b) Explicit sign-off that the dual-version pin is intentional interim tracking and the misleading-comment risk is accepted."
    why_human: "CLAUDE.md 'avoid dead code' is violated. The pin is unused at runtime (Cargo.lock shows dual-version: 0.9.0 as direct dep, 0.8.0 as transitive via sigstore-verify). The Cargo.toml comment says 'tracks the upstream-mandated coherent version of the TUF root' — which implies an active security upgrade that is NOT in effect. The verifier cannot decide whether this is an acceptable interim dev-artifact or a CLAUDE.md blocker."
  - test: "WR-03 decision: Does validate_block_net_conflicts preempting strict_filter constitute a phase gap, or is it forward work?"
    expected: "Either: (a) Fix the validator to scope --block-net contradiction checks to CLI args.block_net only (not profile_network_block), making the intended strict-mode path reachable; OR (b) Explicit decision that the current behavior is intentional (profile block + proxy config = error, not strict mode), with the strict_filter path and its tests updated/removed to match."
    why_human: "validate_block_net_conflicts at launch_runtime.rs:361 runs BEFORE prepare_proxy_launch_options at launch_runtime.rs:406. A profile with block:true + allow_domain/credentials will hit the validator error before strict_filter can activate. The proxy_runtime.rs strict_filter path (lines 150-174) is partially dead for profile-driven block. The test profile_network_block_with_credential_from_profile_errors locks in the error behavior. Whether this is correct or a bug introduced by the absorb is a semantic decision the verifier cannot make."
  - test: "WR-02 tracking: --allow-http2 / enable_h2 is a runtime no-op. Is this an acceptable Phase 99 split consequence or must it be surfaced before Phase 100 release notes?"
    expected: "At minimum, the --allow-http2 help text or a runtime warning should state the flag is not yet wired. Phase 100 release notes must not advertise H2 multiplexing as operational."
    why_human: "UpstreamPool::send() is never called from handle_reverse_proxy (confirmed: tls_connector path at reverse.rs:352; UpstreamPool stored on ProxyState but never invoked for forwarding). Users who set --allow-http2 silently get HTTP/1.1. The split absorb (D-03) allowed this — but whether a warning or doc guard is required before shipping Phase 100 is a product decision."
  - test: "SC3 verification: confirm 11 test-cli failures are pre-existing, not Phase 99 regressions"
    expected: "Running cargo test -p nono-cli on a clean checkout of the branch base (before any Phase 99 commits) should yield the same 11 failures. Alternatively, confirm via git log that all 11 failing test names existed before commit 19935363 (first Phase 99 commit)."
    why_human: "The 99-07 SUMMARY claims '11 failures confirmed pre-existing at branch base (confirmed via git stash + re-run)' and the 99-06 SUMMARY says '11 failures confirmed pre-existing at branch base.' The verifier cannot re-run git stash + cargo test. MEMORY.md documents 4 pre-existing failures (earlier snapshot); the current 11 may include tests added by Phase 99 D-08 deviation tests that happen to fail on Windows. Confirmation is needed that the D-08 tests (test_wsl2_proxy_policy_deviation_preserved, test_compiled_endpoint_policy_compat_deviation_preserved) are NOT among the 11 failing tests."
---

# Phase 99: Upstream Absorb + Fork-Invariant Verify — Verification Report

**Phase Goal:** All will-sync clusters from the Phase 98 ledger are absorbed into the fork in dependency order and the Windows security model, policy-free-library boundary, and cross-target clippy gates are provably unregressed.
**Verified:** 2026-06-30
**Status:** passed (human-approved 2026-06-30; 3 quality/product items deferred to Phase 100, tracked in 99-HUMAN-UAT.md)
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | All 9 upstream replay commits in git log with DCO + upstream SHA trailers | VERIFIED | All 9 SHAs confirmed: 5423f51f/72bcfd66, 92bd42fe/d457ecc3, 345891fa/5b8e94da, c4b03fb2/c808f000, 91b3bdc3/2e64798d, 18719f11/a4d68189, 6fc575f7/cdeeb5b9, c387e421/46bcfbb9, 8438faaa/08ca19a8. All have `(cherry picked from commit ...)` trailers and `Signed-off-by: Oscar Mack Jr`. |
| 2 | D-08 deviation tests exist and are substantive | VERIFIED | `profile/mod.rs:8621` — `test_wsl2_proxy_policy_deviation_preserved` (20+ lines, asserts field presence, round-trip). `proxy_runtime.rs:681` — `test_compiled_endpoint_policy_compat_deviation_preserved` (full compile→evaluate chain test). Neither is a stub. |
| 3 | tls_intercept/ module absent from crates/nono-proxy/src/ | VERIFIED | `ls crates/nono-proxy/src/` confirms no tls_intercept directory. D-03 carve-out preserved. |
| 4 | NetworkIntent absent from crates/nono/src/ (ADR-86 boundary) | VERIFIED | `grep -rn "NetworkIntent" crates/nono/src/` returns 0 matches. NetworkIntent is `pub(crate)` in `crates/nono-cli/src/launch_runtime.rs` only. NetworkMode::ProxyOnly in capability.rs untouched (0-line diff). |
| 5 | exec_strategy_windows/ untouched by Phase 99 | VERIFIED | `git diff HEAD~10..HEAD -- crates/nono-cli/src/exec_strategy_windows/` returns 0 lines. None of Plans 01-07 files_modified lists include exec_strategy_windows/. |
| 6 | 9P warning (Cluster D) in linux.rs | VERIFIED | `unsupported_filesystem_dev` function at linux.rs:423, V9FS_MAGIC constant at linux.rs:396, call site at linux.rs:936 with warning text. Tests `test_fs_type_unsupported_v9fs_magic`, `test_unsupported_filesystem_dev_native_paths`, `test_unsupported_filesystem_dev_wsl2_mount` present. |
| 7 | Cluster G: X-Nono-Token stale claim corrected (a4d68189) | VERIFIED | Commit 18719f11 present with upstream SHA trailer a4d681893d88. token.rs now uses `Proxy-Authorization: Bearer` consistently. |
| 8 | Cluster F: sigstore-trust-root 0.9.0 pin + Cargo.lock regenerated | VERIFIED | `crates/nono/Cargo.toml:54` — `sigstore-trust-root = "=0.9.0"`. `Cargo.lock` contains both `sigstore-trust-root 0.9.0` (line 2388) and `sigstore-trust-root 0.8.0` (dual-version resolution). Cascade evaluated: sigstore-verify 0.8.0 retains transitive 0.8.0; deferred acknowledged in comment. |
| 9 | Cross-target linux-gnu gate GREEN | VERIFIED (SUMMARY) | 99-07-SUMMARY.md documents: first run FAILED (let_chains edition-2021 in linux.rs, caught only by this gate); fix applied (nested if-let rewrite); second run GREEN. Human approved. The let_chains catch is itself proof the gate ran and was effective. |
| 10 | Cross-target apple-darwin gate GREEN | VERIFIED (SUMMARY) | 99-07-SUMMARY.md: both first and confirmation runs GREEN (~57s, ~21s). Human approved. |
| 11 | Phase 89 proxy guard tests pass (D-09) | VERIFIED | `crates/nono-proxy/src/reverse.rs:1350` — `denied_endpoint_returns_403_and_audit` present. `crates/nono-proxy/src/route.rs:681` — `allow_domain_endpoint_route_does_not_shadow_credential_route` present. 99-07-SUMMARY: nono-proxy 192 tests, 0 failures. |
| 12 | D-10 fork-invariant checklist complete with three "Verified unregressed" verdicts | VERIFIED | 99-07-SUMMARY.md contains all three: (1) AppContainer/WFP/broker — diff empty; (2) ADR-86 boundary — NetworkIntent absent from library; (3) exec_strategy_windows/ carve-out — diff empty. Human signed off. |
| 13 | make ci (clippy + fmt + tests) clean on dev host | UNCERTAIN | Clippy: GREEN (exit 0). fmt-check: GREEN (after auto-format of 6 files). test-lib: GREEN (803 tests). test-ffi: GREEN (49 tests). nono-proxy: GREEN (192 tests). test-cli: **11 failures (exit 101)**. make binary unavailable (ran components individually). 99-06 SUMMARY documents "11 failures confirmed pre-existing at branch base (confirmed via git stash + re-run)." Human approved. Not confirmable by verifier without re-running. |

**Score:** 12/13 truths verified (1 UNCERTAIN: SC3 make ci test-cli failures)

### Code Review Warnings Assessment (99-REVIEW.md)

| Warning | Finding | Phase Requirement Impact | Assessment |
|---------|---------|-------------------------|------------|
| WR-01 | sigstore-trust-root 0.9.0 is a direct dep but no code imports it; runtime verification uses transitive 0.8.0; Cargo.toml comment implies security upgrade that is not in effect | UPST11-03: Cluster F absorb ("sigstore-trust-root bump + cascade check") | ACCEPTABLE DEVIATION — absorb action was "add pin, cascade check, regen Cargo.lock" — all performed. Dual-version documented in comment. BUT: CLAUDE.md "avoid dead code" is violated; misleading comment is a quality risk. **Requires human decision.** |
| WR-02 | UpstreamPool::send() never called from handle_reverse_proxy; --allow-http2/enable_h2 flag is a runtime no-op | UPST11-02: Cluster C split cdeeb5b9 (#983 HTTP/2 pool) | ACCEPTABLE DEVIATION — D-03 split strategy explicitly allowed dropping h2_forward/h2_probe hunks. Pool struct is the applicable hunk absorbed. But users will see --allow-http2 in --help and be misled. Phase 100 release notes risk. |
| WR-03 | validate_block_net_conflicts folds profile_network_block into block_net check, running before prepare_proxy_launch_options; profile block:true + allow_domain = validator error instead of strict-filter mode | UPST11-02: Cluster A d457ecc3 absorbed (guard implemented) | ACCEPTABLE DEVIATION for absorb req — d457ecc3 absorbed; fails closed (error not bypass); not a Windows security regression; Phase 89 guards pass. BUT: strict_filter path (proxy_runtime.rs:174) partially dead; correctness inconsistency. **Requires human decision.** |
| WR-04 | 9P warning only for paths under /mnt; paths on 9P outside /mnt get no warning | UPST11-02: Cluster D diagnostic absorb | ACCEPTABLE — diagnostic-only (no enforcement decision); /mnt is the dominant WSL2 path. WR-04 is a scope limitation, not a regression. No phase requirement affected. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/launch_runtime.rs` | NetworkIntent enum (pub(crate)) | VERIFIED | Enum at line 147 with Unrestricted/BlockAll/ProxyFiltered variants |
| `crates/nono-cli/src/proxy_runtime.rs` | validate_block_net_conflicts imported + D-08 test | VERIFIED | Import at line 2, test at line 681 |
| `crates/nono-cli/src/profile/mod.rs` | D-08 WSL2ProxyFallback test | VERIFIED | d08_deviation_tests module at line 8607, test at 8621 |
| `crates/nono-proxy/src/pool.rs` | UpstreamPool struct (Cluster C split) | VERIFIED | Pool struct exists; send() at line 173; wired to ProxyState but never invoked from forwarding path (WR-02) |
| `crates/nono/src/sandbox/linux.rs` | 9P warning (Cluster D) | VERIFIED | unsupported_filesystem_dev at line 423; let_chains edition-2021 bug was caught by linux-gnu gate and fixed |
| `crates/nono/Cargo.toml` | sigstore-trust-root =0.9.0 pin | VERIFIED (with WR-01) | Line 54; direct dep present but unused at runtime |
| `crates/nono-cli/src/exec_strategy_windows/` | Untouched by Phase 99 | VERIFIED | 0-line git diff HEAD~10..HEAD |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| NetworkIntent (launch_runtime.rs) | proxy_runtime.rs | import at line 2 | VERIFIED | `use crate::launch_runtime::{NetworkIntent, ProxyLaunchOptions}` |
| validate_block_net_conflicts | launch_runtime.rs call site | import + call at line 361 | VERIFIED | Call at line 361, before prepare_proxy_launch_options at line 406 |
| validate_block_net_conflicts | command_runtime.rs dry-run path | import + call | VERIFIED | Per 99-03-SUMMARY Task 1 |
| D-08 test modules | WSL2ProxyFallback/CompiledEndpointPolicy types | direct struct access + evaluate() chain | VERIFIED | Tests instantiate the types and exercise the actual API |
| exec_strategy_windows/ | Cluster A NetworkIntent | no linkage required | VERIFIED | WindowsNetworkPolicyMode::ProxyOnly reads enforcement-time type; NetworkIntent is CLI-side only |
| UpstreamPool (pool.rs) | handle_reverse_proxy forwarding | NOT wired | WR-02 | tls_connector path used; pool.send() never called from forwarding |

### Data-Flow Trace (Level 4)

Not applicable — phase is a code absorb, not a feature rendering dynamic data. The relevant data-flow traces are the proxy forwarding path (WR-02 covers UpstreamPool inert) and the trust-root verification path (WR-01 covers sigstore 0.9.0 unused).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| NetworkIntent absent from library | `grep -rn "NetworkIntent" crates/nono/src/` | 0 matches | PASS |
| tls_intercept absent | `ls crates/nono-proxy/src/` | No tls_intercept directory | PASS |
| exec_strategy_windows/ unchanged | `git diff HEAD~10..HEAD -- crates/nono-cli/src/exec_strategy_windows/` | 0 lines | PASS |
| All 9 SHA trailers present | `git log --format="%b" <sha>` for each | All 9 have cherry-pick + Signed-off-by | PASS |
| sigstore-trust-root 0.9.0 in Cargo.lock | `grep "sigstore-trust-root" Cargo.lock` | Both 0.9.0 (direct) and 0.8.0 (transitive) | PASS (dual-version documented) |
| UpstreamPool.send() not called in forwarding | `grep "upstream_pool\|pool\.send" crates/nono-proxy/src/reverse.rs` | upstream_pool stored on ctx struct, send() not invoked from handle_reverse_proxy | WR-02 confirmed |
| NetworkIntent pub(crate) (not pub) | `grep "pub.crate. enum NetworkIntent" crates/nono-cli/src/launch_runtime.rs` | pub(crate) at line 147 | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| UPST11-02 | 99-02, 99-03, 99-04, 99-06 | All will-sync clusters absorbed without regressing Windows model or policy-free boundary | VERIFIED | All 9 commits in git; D-08 tests pass; D-10 checklist complete |
| UPST11-03 | 99-04, 99-05 | Dep/CI/docs clusters absorbed; Cargo.lock regenerated; build clean | VERIFIED (with WR-01 caveat) | Clusters E/F/G commits present; Cargo.lock contains 0.9.0 pin; noise PRs annotated in REQUIREMENTS.md |
| UPST11-04 | 99-07 | Cross-target clippy GREEN; make ci clean; fork-invariant checklist | PARTIAL | Cross-target GREEN (SUMMARY + human); make ci UNCERTAIN (11 pre-existing test-cli failures; human approved); D-10 complete |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/nono/Cargo.toml` | 54 | `sigstore-trust-root = "=0.9.0"` — direct dep with no Rust code importing it | Warning | Unused dep violates CLAUDE.md "avoid dead code"; comment implies active security upgrade that is not in effect (WR-01). Tracked as human decision item. |
| `crates/nono-proxy/src/pool.rs` | 173 | `pub async fn send(...)` — UpstreamPool::send never called from forwarding path | Warning | Dead infrastructure per WR-04 (IN-04). Consequence of D-03 split. --allow-http2 flag is a capability-claims gap (WR-02). |
| `crates/nono-cli/src/execution_runtime.rs` | 235–243 | Guard `if matches!(caps.network_mode(), ProxyOnly { .. })...` is dead code | Info | ProxyOnly placeholder removed; guard unreachable. Stale comment cites wrong line numbers (IN-02). Low risk — fail-secure dead guard. |
| `crates/nono-cli/src/proxy_runtime.rs` | 293–311 | Comment claims endpoint_restrictions "runs before domain-endpoint routes are merged" but routes are merged first | Info | Comment wrong; over-matching adds more restrictive rules (fail-secure direction) (IN-01). |
| `crates/nono/src/sandbox/linux.rs` | `unsupported_filesystem_dev` | Doc says "returns st_dev" but returns f_fsid; SAFETY note incomplete for transmute | Info | Documentation error, not behavioral bug (IN-03). Caught by code review. |

Debt marker scan: No TBD/FIXME/XXX markers found in Phase 99 modified files. No debt-marker blockers.

### Human Verification Required

### 1. WR-01: Unused sigstore-trust-root 0.9.0 dep — CLAUDE.md dead code decision

**Test:** Decide whether the `sigstore-trust-root = "=0.9.0"` direct dep in `crates/nono/Cargo.toml:54` should be (a) removed, with a comment stating the cascade was evaluated and upgrade deferred to a future sync phase, or (b) kept with explicit acknowledgment that it's an interim tracking artifact and the Cargo.toml comment wording is accepted.

**Expected:** If removed: `cargo update -p sigstore-trust-root` and `Cargo.lock` regen confirm the 0.9.0 entry disappears; comment remains to document the evaluation. If kept: an explicit override entry in VERIFICATION.md frontmatter.

**Why human:** CLAUDE.md says "avoid dead code — if code is unused, either remove it or write tests that use it." The pin has no Rust import anywhere in `crates/nono/src/`. The in-tree comment "tracks the upstream-mandated coherent version of the TUF root" is misleading — the runtime trust-root in effect is still 0.8.0. The verifier cannot decide whether this is a CLAUDE.md blocker (requiring a fix before Phase 100) or an accepted interim artifact.

### 2. WR-03: validate_block_net_conflicts vs strict_filter semantic decision

**Test:** Confirm the intended behavior for profile `{ "network": { "block": true, "allow_domain": ["api.github.com"] } }`. Does this profile mean (a) "strict mode: proxy-filter allowed domains, block everything else" (what strict_filter was designed for), or (b) "contradictory configuration: error"?

**Expected:** If (a): Narrow the validator's `block_net` check at `sandbox_prepare.rs:204` to `args.block_net` only (not `|| prepared.profile_network_block`), making the strict-filter path reachable. Update or remove `profile_network_block_with_credential_from_profile_errors` test. If (b): Remove/adjust the `strict_filter = prepared.profile_network_block` path in `proxy_runtime.rs:174` and its comments so the dead path is not advertised as supported.

**Why human:** The current codebase has two contradictory statements: `proxy_runtime.rs:151` computes `block_wins = args.block_net || (prepared.profile_network_block && !would_activate)` to handle the strict-filter case, and `proxy_runtime.rs:174` sets `strict_filter = prepared.profile_network_block` — both indicating profile block + active proxy = strict mode. But the validator at `sandbox_prepare.rs:204` sets `block_net = args.block_net || prepared.profile_network_block` and rejects the combination. The code has two conflicting designs. Which is correct is a product decision.

### 3. WR-02 tracking — Phase 100 release notes gate

**Test:** Before any Phase 100 release notes are drafted or published, confirm that `--allow-http2` / `network.allow_http2` behavior is explicitly described as "accepted but not yet wired" or that the pool is wired to `handle_reverse_proxy` before advertising H2 multiplexing.

**Expected:** Either `--allow-http2` help text updated with a "(not yet wired — HTTP/1.1 will be used)" note, or `UpstreamPool::send()` connected to `handle_reverse_proxy` before Phase 100 release notes mention H2.

**Why human:** Users who set the flag silently get HTTP/1.1. The Phase 99 D-03 split was correct — but the post-split user experience requires explicit guidance or implementation before a release that includes this CLI flag.

### 4. SC3 verification — confirm 11 test-cli failures are pre-existing baseline

**Test:** Run `cargo test -p nono-cli` on the commit immediately before `19935363` (first Phase 99 commit) and confirm the same 11 tests fail. Alternatively, confirm the 11 failing test names do NOT include `test_wsl2_proxy_policy_deviation_preserved` or `test_compiled_endpoint_policy_compat_deviation_preserved` (D-08 deviation tests that Phase 99 added).

**Expected:** All 11 failures are env-lock or Windows-path failures that pre-date Phase 99. The two D-08 tests pass. The 11 are: `audit_session::tests::discover_sessions_*`, `config::tests::nono_home_dir_*` (6 tests), `profile_cmd::tests::test_init_allowed_*`, `protected_paths::tests::*` (3 tests).

**Why human:** 99-06-SUMMARY claims "11 failures confirmed pre-existing at branch base via git stash + re-run." MEMORY.md documents a prior snapshot of 4 pre-existing failures (different count). The verifier cannot re-run git stash without side effects. Confirming D-08 tests pass on Windows (they should — they're pure type/struct tests) eliminates the risk that Phase 99 introduced new failures.

### Gaps Summary

No BLOCKER gaps identified. The core phase goal is achieved: all 9 upstream commits absorbed with DCO + SHA trailers, D-10 checklist complete with explicit verdicts, cross-target clippy GREEN on both mandatory gates (catching a real edition-2021 incompatibility that Windows-host clippy cannot see), and all three fork invariants (AppContainer/WFP/broker, ADR-86 boundary, exec_strategy_windows/ carve-out) verified unregressed.

Four human decision items surface from the 99-REVIEW.md code review, which was completed alongside the phase terminal gate:

1. WR-01 is the highest-priority item: the unused 0.9.0 pin with a misleading security comment violates CLAUDE.md dead-code guidance and could mislead operators reading the changelog. Recommend removing the direct dep and updating the comment to accurately state the effective trust-root is 0.8.0.

2. WR-03 needs a semantic decision to eliminate the contradictory design (validator + strict_filter disagree on what profile block + proxy config means).

3. WR-02 and SC3 are lower-priority tracking items — the split absorb was correctly scoped, and the pre-existing test failures are project-documented — but need human confirmation before Phase 100.

The ROADMAP top-level Phase 99 checkbox is unchecked ("- [ ] Phase 99 — 0/7 plans") but all 7 plan detail items are [x]. This is the documented SDK phase.complete issue; flip the top-level checkbox manually.

---

_Verified: 2026-06-30T00:00:00Z_
_Verifier: Claude (gsd-verifier)_
