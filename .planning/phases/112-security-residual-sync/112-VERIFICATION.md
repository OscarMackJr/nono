---
phase: 112-security-residual-sync
verified: 2026-08-05T00:00:00Z
status: human_needed
score: 8/8 must-haves verified (SC4 partially — see below)
overrides_applied: 0
human_verification:
  - test: "Run the full `cargo test -p nono-sandbox-cli --tests` integration sweep (all 36 integration test binaries) to completion on a host that does not stall/OOM, or equivalently confirm on the live GH Actions Linux CI lane."
    expected: "All integration binaries complete without a >25min stall; no new failures beyond the documented 11-name Windows-host baseline / 27-name workspace baseline."
    why_human: "Locally, this session's own Windows-host run of `cargo test -p nono-sandbox-cli --bin nono` completed (1484 passed / 11 failed, exactly matching the pre-existing baseline) but the separate `--tests` invocation that exercises all 36 integration-test binaries was reported by the phase's own fix-pass work as stalling after ~25min and being killed, with only ~11 relevant binaries actually exercised. No artifact in the phase directory documents a completed run of the full integration-binary sweep. This is a coverage gap on SC4's 'make ci GREEN' claim, not a known failure — needs a host/CI lane that can complete the full sweep."
  - test: "Run `tests/socket_access_run.rs`'s 3 python3-dependent tests (2 pre-existing + SEC-06's new `af_unix_mediation_pathname_allows_orphaned_child_tcp_connect`) on a Linux host/CI lane with python3 present."
    expected: "The new orphan-reap regression test actually exercises the double-forked-orphan + loopback-TCP-connect scenario and passes for the right reason (not the python3-unavailable skip path)."
    why_human: "112-06-SUMMARY.md documents that the local `cross`-rs container used for cross-target verification lacks python3, so all 3 tests short-circuit on `python3_available()`'s guard and report `ok` trivially in ~0.01s — SEC-06's behavioral assertions (orphan TCP connect succeeds, no 'Failed to read sockaddr' ancestry-loss fingerprint) were never actually runtime-exercised in this phase's local verification. The executor itself flagged this as needing a live python3-equipped CI lane before treating SEC-06's regression coverage as fully proven."
  - test: "Confirm `af_unix_mediation_pathname_allows_connect_to_listed_socket`'s RES-02 denial-marker tightening (112-04) on a python3-equipped host."
    expected: "The tightened assertion (bare socket-path marker) actually distinguishes a denial from a non-denial at runtime."
    why_human: "Same python3-unavailable local-container limitation as above, self-flagged in 112-04-SUMMARY.md's 'Known limitation' section — the test's compile/type correctness was confirmed but not its runtime behavior."
---

# Phase 112: Security + Residual Sync Verification Report

**Phase Goal:** The security-relevant and residual commits from the `v0.66.0..v0.69.0` window that no other v3.6 phase covers are absorbed under a fork-invariant review kept separate from any release-cut phase — mirrors the v3.1 Phase 87 precedent.
**Verified:** 2026-08-05
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths / Roadmap Success Criteria

| # | Truth (Roadmap SC) | Status | Evidence |
|---|---|---|---|
| SC1 | 8/9 security-relevant requirements (SEC-01, SEC-03..SEC-09) absorbed under a fork-invariant review distinct from any release-cut phase; SEC-02 carved out to Phase 114 with reality-check evidence recorded in the D-05 ledger addendum | ✓ VERIFIED | `REQUIREMENTS.md:130-138` — SEC-01/03-09 all `[x]` with cited evidence; SEC-02 (`:131`) explicitly `[ ]` with "→ CARVED OUT to Phase 114" note. `108-DIVERGENCE-LEDGER.md`'s "Phase 112 Security + Residual Sync Addendum" (line ~1718) records all 18 SHAs including SEC-02a/b/c as "deferred → Phase 114 — CONFIRMED". `ROADMAP.md:37/39/284-323` — Phase 112 checked, Phase 114 section present with full carve-out rationale and dependency ("Phase 112 (SEC-07 creates proxy_command.rs)... 112 lands first"). Live code: `crates/nono-proxy/src/` has no `aws/`/`tls_intercept/`/`oauth_capture/` dirs (SEC-01/SEC-02 absent-subsystem confirmed by `ls`); `execution_runtime.rs` has 0 grep hits for `command_policies`/`tool_sandbox_runtime`/`tool_sandbox_initial_shim` (SEC-09 confirmed). |
| SC2 | RES-01 and RES-02 each individually reviewed and absorbed/skipped with recorded reasoning, never silently dropped | ✓ VERIFIED | `REQUIREMENTS.md:139-140` both `[x]` with per-SHA disposition citations. `112-OAUTH-CAPTURE-DISPOSITION.md` Part 2 gives all 4 RES-01 SHAs individual skip-with-reasoning (HKLM machine-policy-spine collision, D-03). `112-04-SUMMARY.md` gives all 3 RES-02 SHAs individual dispositions: `503045801a` adopted verbatim (38/38 `pty_proxy` tests, live-verified this session on Windows via a related integration test), `4cc0af2c52` SKIP with corrected symbol-level reasoning (target `socket_test_dir()` confirmed absent from the fork), `9840a16f35` adopted-adapted (marker rebuilt around the fork's actual diagnostic format). Ledger addendum table cites all 7 SHAs with "as shipped" dispositions. |
| SC3 | `373a67ae` (#1369, crossbeam-epoch 0.9.18→0.9.20) confirmed already closed out-of-band; `cargo audit` clean, not re-absorbed | ✓ VERIFIED | Live-run this session: `grep -A1 'name = "crossbeam-epoch"' Cargo.lock` → `version = "0.9.20"`. `cargo audit` → exit 0, "6 allowed warnings found" (unrelated advisories), 0 vulnerabilities. Matches `112-DISPOSITION-TABLE.md`'s D-07 confirmation section verbatim. |
| SC4 | Both cross-target clippy gates (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) and `make ci` GREEN locally after the absorb | ⚠ PARTIALLY VERIFIED | Cross-target clippy: every code-touching plan (112-02, 112-04, 112-05, 112-06, 112-07) documents both gates GREEN with `--all-targets`, and the 4 code-review fix commits (CR-01..CR-04) are all `#[cfg(target_os = "linux")]`-relevant or CLI-only changes cited against the same gates in `112-REVIEW.md`'s frontmatter (`linux_clippy: green`, `darwin_clippy: green`, `fmt_check: clean`). Live-verified this session (Windows host, supplementary not a substitute for the mandatory gate): `cargo build --workspace --all-targets` clean; `cargo fmt --all -- --check` exit 0; `cargo test -p nono-sandbox-cli --bin nono` → 1484 passed / 11 failed, and the 11 failing names are byte-for-byte the documented pre-existing Windows-host baseline (`config::`/`protected_paths::`/`profile_cmd::`/`audit_session::` — matches `112-REVIEW.md`'s claimed "1484 pass with the same 11 pre-existing... failures" exactly); `cargo test -p nono-sandbox-proxy` → 218/218 pass; `cargo test -p nono-sandbox --lib` → 820/820 pass (Windows subset; Linux-only tests not compiled here). **Gap:** no artifact in the phase directory documents a completed run of `cargo test -p nono-sandbox-cli --tests` (the full 36-integration-binary sweep) — per the phase's own fix-pass work this stalled after ~25min and was killed, with only ~11 relevant binaries actually run. `112-08-SUMMARY.md`'s "combined verification" instead ran `cargo test --workspace --no-fail-fast` (17-name failing set, matching `111-04`'s baseline) — a different, narrower command than the full per-crate `--tests` integration sweep. Routed to human verification below, not scored as a blocker (no evidence of an actual regression, only unproven coverage). |

**Score:** 4/4 roadmap SCs have supporting evidence; SC4 is PARTIALLY verified (clippy/fmt/build/unit-tests confirmed GREEN, full integration-binary sweep unconfirmed).

### Execution-History Items Explicitly Verified

| # | Item | Status | Evidence |
|---|---|---|---|
| 1a | SEC-03 shipped ADAPT, not the disposition table's "adopt, HIGH confidence" (depends on unabsorbed `fa21a004`/`8a4237f2` antecedent) | ✓ CONFIRMED | `112-02-SUMMARY.md` key-decisions; ledger addendum "Divergences" §1; live: `grep -rn "SeccompPolicy\|LinuxSandboxPolicy" crates/nono-cli/src crates/nono/src` finds no such types — the fork's `apply()`/`apply_with_abi()`/`apply_seccomp` shape is confirmed the adapted (not ported) architecture described. |
| 1b | `4cc0af2c52` (RES-02) shipped SKIP, not "adapt with caution" — `socket_test_dir()` absent | ✓ CONFIRMED | `112-04-SUMMARY.md`; live: `grep -rn socket_test_dir crates/` → 0 hits. |
| 1c | `9840a16f35` (RES-02) shipped adapted, not verbatim | ✓ CONFIRMED | `112-04-SUMMARY.md`; live: `crates/nono-cli/tests/socket_access_run.rs` uses the bare socket path as the marker, not upstream's `"send {path}"` string (confirmed by reading the file). |
| 1d | `112-08` ledger addendum records what SHIPPED, not what was planned | ✓ CONFIRMED | `108-DIVERGENCE-LEDGER.md`'s "18-SHA Final Disposition (as executed)" table has a dedicated "Table's planned disposition" vs. "Disposition as shipped" column pair and a "Divergences" section explicitly naming all 3 corrections plus a methodological lesson. |
| 2 | 4 Critical fail-open defects found by code review, all fixed in `916d3bd7`/`892d2e01`/`585f2743`/`20f9a1cd` | ✓ CONFIRMED | All 4 commits exist with real diffs (`git show` inspected). Live code confirms each fix present: `proxy_command.rs` merges `net.deny_domain`/`net.no_proxy`/`net.block` (CR-01, plus 5 passing regression tests run live this session); `trust_scan.rs::load_nono_policy` resolves absent-predicate by content (legacy accept vs. foreign skip) with hard errors for wrong/non-string predicates (CR-02, 5 tests pass live); `exec_strategy.rs`'s `reap_reparented_orphans` early-return now drains `drain_pending_network_notifications` before returning (CR-03, code read directly); `linux.rs::validate_external_tcp_delegation` rejects `external_tcp()` whenever a network policy is declared (CR-04, code read directly). 12 Warnings + 5 Info remain OPEN by documented operator scoping decision (`112-REVIEW.md` frontmatter `fix_pass.scope: critical_only`) — recorded as known open items below, not scored as gaps. |
| 3 | `cargo test -p nono-sandbox-cli --tests` (36 integration binaries) stalled ~25min, killed; only ~11 binaries run; full sweep NOT run end-to-end | ✓ ACCOUNTED FOR | No phase artifact documents a completed full-sweep run; this matches the described limitation. Reflected in SC4 above and routed to human verification. |
| 4 | Windows-host `nono-sandbox-cli --bin nono` has ~11 pre-existing, env-specific test failures — NOT regressions | ✓ CONFIRMED, NOT REGRESSIONS | Live-run this session reproduces the exact 11 names: `audit_session::tests::discover_sessions_does_not_warn_when_legacy_audit_root_is_empty`, 6× `config::tests::*`, `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`, 3× `protected_paths::tests::*`. Matches `112-REVIEW.md`'s cited baseline exactly (stash-and-rerun confirmed by the fixer; this session's independent live run reproduces the identical count and names). |

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `112-DISPOSITION-TABLE.md` | 18-SHA finalized disposition table | ✓ VERIFIED | Present, 18 rows, arithmetic closes (10+1+4+3=18). |
| `112-AWS-SIGV4-PROXY-AUTH-FINDING.md` | SEC-01 won't-sync finding | ✓ VERIFIED | Present, cites live `ls`/`git show` evidence. |
| `112-OAUTH-CAPTURE-DISPOSITION.md` | SEC-02a/b/c evidence + RES-01 skip docs | ✓ VERIFIED | Present, both parts complete, security reasoning explicit. |
| `112-REVIEW.md` | Code review with resolved Criticals | ✓ VERIFIED | Present, 4/4 Criticals resolved with per-finding notes; 12 Warnings/5 Info open, documented as scoped-out. |
| `proj/ADR-112-allow-vars-fail-closed-preserved.md` | SEC-08 PRESERVE decision | ✓ VERIFIED | Present; `git diff -- crates/nono-cli/src/profile_runtime.rs` for this phase's commits shows the file untouched (confirms "zero source-code change" claim). |
| `108-DIVERGENCE-LEDGER.md` Phase 112 addendum | Ledger addendum recording shipped reality | ✓ VERIFIED | Present at line ~1718, 18-row table + divergences + new-information + tally sections all present and arithmetic-closed. |
| `crates/nono-cli/src/proxy_command.rs` | Standalone `nono proxy` command (SEC-07) + CR-01 fix | ✓ VERIFIED, WIRED | Exists, dispatches via `cli.rs`/`main.rs`/`app_runtime.rs`; 5 CR-01 regression tests + 2 Wave-0 integration tests all pass live (this session). |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `REQUIREMENTS.md` SEC-02 entry | `ROADMAP.md` Phase 114 section | cross-reference | ✓ WIRED | Both cite the carve-out with matching rationale (26 files, ~9 absent-subsystem, `forward.rs` ResponseRewrite hook). |
| `112-DISPOSITION-TABLE.md` | `108-DIVERGENCE-LEDGER.md` addendum | supersession | ✓ WIRED | Addendum explicitly names and corrects all 3 divergent rows rather than silently overwriting. |
| `112-REVIEW.md` Critical findings | Fix commits | resolution | ✓ WIRED | All 4 findings have `**RESOLVED**` sections citing exact commit SHAs; commits verified to exist and contain the described fix. |
| `nono proxy --profile X` | Profile's `deny_domain`/`no_proxy`/`network.block` | `build_launch_options` | ✓ WIRED | Live code + 5 passing tests confirm the merge (post CR-01 fix). |
| `trust_scan::load_nono_policy` | User-level trust policy | `trust_cmd::load_trust_policy` | ✓ WIRED | Live code confirms project/user policies loaded independently, no early-return short-circuit (post CR-02 fix). |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| SEC-01 | 112-01 | AWS SigV4 auth for MiTM proxy | ✓ SATISFIED (won't-sync) | Absent subsystem confirmed live; `112-AWS-SIGV4-PROXY-AUTH-FINDING.md` |
| SEC-02 | — | Sandboxed OAuth capture | ⏭ DEFERRED (not this phase's to satisfy) | `REQUIREMENTS.md:131` unchecked, points to Phase 114; not silently dropped |
| SEC-03 | 112-02 | NVIDIA procfs mediation hardening | ✓ SATISFIED (adapted) | Live code (`apply_seccomp`, `/proc/self/task` Read-only) + tests |
| SEC-04 | 112-03 | Trust-policy `predicate` discriminator | ✓ SATISFIED (adopt) | Live code + 67 passing trust tests (this session) |
| SEC-05 | 112-05 | Landlock execute-restriction `Refer` grant | ✓ SATISFIED (adopt) | `112-05-SUMMARY.md`, symbol-level re-verified |
| SEC-06 | 112-06 | Seccomp supervisor-ancestry orphan reaping | ✓ SATISFIED (adapted), coverage gap noted | Live code (`reap_reparented_orphans`, `PR_SET_CHILD_SUBREAPER`); behavioral test not yet exercised (python3 absent locally) — see human verification |
| SEC-07 | 112-07 | Standalone `nono proxy` command | ✓ SATISFIED (adapted) | Live code + 218/218 proxy tests + 5/5 proxy_command tests (this session) |
| SEC-08 | 112-03 | `allow_vars` empty-list fix | ✓ SATISFIED (won't-sync/PRESERVE) | ADR-112, zero source diff confirmed |
| SEC-09 | 112-08 | Credential-broker guard relaxation | ✓ SATISFIED (won't-sync + carry-forward) | Live grep confirms absent target; carry-forward note filed in ledger |
| RES-01 | 112-01 | Registry/update-check header residual | ✓ SATISFIED (skip x4) | `112-OAUTH-CAPTURE-DISPOSITION.md` Part 2 |
| RES-02 | 112-04 | PTY-teardown + test-infra residual | ✓ SATISFIED (mixed dispositions) | `112-04-SUMMARY.md`, live tests for the adopted commit |

No orphaned requirements — all 10 Phase-112-owned IDs plus SEC-02 accounted for.

### Anti-Patterns Found

Scanned all phase-touched core files (`proxy_command.rs`, `trust_cmd.rs`, `trust_scan.rs`, `exec_strategy.rs`, `sandbox/linux.rs`, `sandbox/mod.rs`, `pty_proxy.rs`, `supervisor/socket.rs`) for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER`: zero hits. No debt markers, no blockers.

### Known Open Items (recorded, not scored as gaps per task instructions)

- 12 Warning-severity + 5 Info-severity code review findings remain OPEN by explicit operator scoping decision (`112-REVIEW.md` frontmatter, `fix_pass.scope: critical_only`). None contradicts a must-have; several (WR-04, WR-05, WR-12) describe residual proxy-auth hardening opportunities that are pre-existing-risk-class, not regressions introduced by this phase's absorb.
- `fa21a004`/`8a4237f2` (#1283, 21-file `LinuxSandboxPolicy`/`SeccompPolicy` refactor) remains a standing unabsorbed antecedent that SEC-03's adapt depends on but does not close — flagged in both `112-02-SUMMARY.md` and the ledger addendum for a future absorb plan.

### Human Verification Required

See frontmatter `human_verification` — 3 items, all concerning completing test coverage on a Linux/CI host that this Windows-host verification session cannot provide (python3-dependent integration tests, and the full 36-binary integration sweep). None represent a known failure; all represent unproven coverage the phase's own executors flagged honestly.

### Gaps Summary

No FAILED must-haves. All 4 roadmap Success Criteria have direct, live-verified supporting evidence; SEC-01 through SEC-09 (minus the explicitly-carved-out SEC-02) and RES-01/RES-02 are all individually dispositioned with cited evidence, none silently dropped. The 4 Critical code-review findings are confirmed fixed in live code with passing regression tests. The one substantive shortfall is SC4's test-coverage completeness: the full `cargo test -p nono-sandbox-cli --tests` integration-binary sweep and the python3-dependent behavioral assertions for SEC-06/RES-02 have not been exercised end-to-end in this phase (a limitation the phase's own executors disclosed, not one this verification discovered). This is a coverage gap, not a demonstrated regression — routed to human/CI verification rather than blocking the phase.

---

_Verified: 2026-08-05_
_Verifier: Claude (gsd-verifier)_
