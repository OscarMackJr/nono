---
phase: 112
slug: security-residual-sync
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-05
---

# Phase 112 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `112-RESEARCH.md` § Validation Architecture. D-06 (LOCKED) governs the
> Linux-only enforcement items.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) + `cross test` for Linux-only-compiled modules |
| **Config file** | none — no dedicated test-framework config; workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p <crate> --lib -- <filter>` (native, cross-platform code) · `cross test --target x86_64-unknown-linux-gnu -p <crate> -- <filter>` (for `#[cfg(target_os = "linux")]`-gated code) |
| **Full suite command** | `cargo test --workspace --no-fail-fast` — **diff against the inherited baseline below, do NOT expect exit 0** |
| **Estimated runtime** | ~60–90s targeted filter · full workspace several minutes · `cross test` adds Docker image startup |

**Cross-target gates (mandatory, per CLAUDE.md):**
- linux-gnu: `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
- apple-darwin: `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset)

Exact invocation syntax, pinned cross image tag, and the per-gate decision tree live in
`.planning/templates/cross-target-verify-checklist.md` — **single source of truth, do not
duplicate the runbook into plans.** PARTIAL→CI is the fallback only on a *documented runner
failure*; a stopped Docker daemon or absent tool does NOT qualify.

---

## Sampling Rate

- **After every task commit:** targeted `cargo test` / `cross test` filter for the touched module (see Per-Task Verification Map)
- **After every plan wave:** `make ci` (clippy + fmt + tests), **plus both cross-target clippy gates** for any wave touching `crates/nono/src/sandbox/linux.rs`, `crates/nono-cli/src/exec_strategy.rs`, or `crates/nono-cli/src/exec_strategy/`
- **Before `/gsd:verify-work`:** full `cargo test --workspace --no-fail-fast` diffed against the inherited baseline; both cross-target gates GREEN (ROADMAP SC4)
- **Max feedback latency:** ~90s for targeted filters

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists | Status |
|-----|----------|-----------|-------------------|-------------|--------|
| SEC-03 | NVIDIA procfs mediation predicate | unit | `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox -- test_is_nvidia_compute_device_accepts_upstream_list` | ✅ `crates/nono/src/sandbox/linux.rs:5059` (extend, don't create) | ⬜ pending |
| SEC-03 | `--allow-gpu` capability wiring | unit | `cargo test -p nono-sandbox-cli --lib -- test_from_args_allow_gpu_sets_capability_on_unix test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu` | ✅ `crates/nono-cli/src/capability_ext.rs:1468`/`:1512` | ⬜ pending |
| SEC-04 | Trust-policy predicate-field discriminator (no full-parse crash on foreign `trust-policy.json`) | unit | `cargo test -p nono-sandbox --lib -- trust::bundle` (filter TBD by plan) | ✅ `crates/nono/src/trust/bundle.rs` | ⬜ pending |
| SEC-05 | Landlock `Refer` present in the **execute-restriction** layer | unit (regression) | `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox -- test_restrict_execute_does_not_break_rename_into_new_subdir` | ❌ **Wave 0** — this test is what `d84b4818` itself adds; port it verbatim with the absorb | ⬜ pending |
| SEC-06 | Orphan-reaping under seccomp-notify mediation | unit | `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox-cli -- test_linux_child_requires_dumpable_only_for_seccomp_driven_features` + new orphan-reap test | ✅ predicate at `crates/nono-cli/src/exec_strategy.rs:4452`; orphan-reap test is **Wave 0** | ⬜ pending |
| SEC-07 | `nono proxy` subcommand dispatch | integration | `cargo test -p nono-sandbox-cli --bin nono -- proxy` (new tests, names TBD by plan) | ❌ **Wave 0** — no `Proxy` variant exists yet | ⬜ pending |
| SEC-08 | `allow_vars` **fail-closed** semantics preserved (anti-regression guard) | unit | `cargo test -p nono-sandbox-cli --lib -- empty_allow_vars_fails_closed` | ✅ `crates/nono-cli/src/profile_runtime.rs:989` — **run BEFORE and AFTER any SEC-08 change** | ⬜ pending |
| SEC-09 | Carry-forward note into the v3.7 tool-sandbox work-list | manual-only | N/A — documentation deliverable | N/A | ⬜ pending |
| SEC-01 | Won't-sync finding document (mirrors `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`) | manual-only | N/A — finding document deliverable | N/A | ⬜ pending |
| SEC-02a/b/c | Disposition go/no-go (Open Question 1) then adapt-or-defer | unit (scope TBD) | Deferred until the reality-check task resolves adapt-vs-defer | N/A pending disposition | ⬜ pending |
| RES-01 (×4) | Skip-with-recorded-reasoning (skip-biased, D-03) | manual-only | N/A unless a task diff-proves applicability | N/A | ⬜ pending |
| RES-02 `503045801a` | Late CPR-reply teardown drain | unit (new) | New unit test around `discard_late_terminal_input()` / `CprReplyParse` | ❌ **Wave 0** | ⬜ pending |
| RES-02 `9840a16f` | Denial-marker assertion tightening | integration | `cargo test -p nono-sandbox-cli --test socket_access_run -- af_unix_mediation_pathname_allows_connect_to_listed_socket` | ✅ | ⬜ pending |
| RES-02 `4cc0af2c` | Test-infra `/tmp` path change (adapt-with-caution — see A2) | integration | Covered by the `socket_access_run` suite above | ✅ | ⬜ pending |
| D-07 / SC3 | `crossbeam-epoch` 0.9.20 confirmed, `cargo audit` clean | recorded confirmation | `cargo audit` — expect 0 vulnerabilities, 6 pre-existing allowed advisory warnings | ✅ `Cargo.lock` | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono/src/sandbox/linux.rs` — port `test_restrict_execute_does_not_break_rename_into_new_subdir` verbatim from `d84b4818` (SEC-05)
- [ ] `crates/nono-cli/src/exec_strategy.rs` — new orphan-reap unit test (SEC-06); A3 open: confirm whether the fork's **two** `run_supervisor_loop` definitions are cfg-split such that the reap call site needs adding to one or both
- [ ] `crates/nono-cli/` — new `nono proxy` subcommand dispatch tests (SEC-07)
- [ ] `crates/nono-cli/src/exec_strategy_windows/` — new unit test around `discard_late_terminal_input()` / `CprReplyParse` (RES-02 `503045801a`)

No framework install needed — Rust's built-in test runner and the `cross` / `cargo-zigbuild`
toolchains are already local-runnable and were exercised GREEN in Phase 111.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Carry-forward note lands in the v3.7 tool-sandbox work-list | SEC-09 (D-01 LOCKED) | Documentation deliverable — the note IS the deliverable; "won't-sync" ≠ "no work" | Confirm the note exists at the target file/insertion point RESEARCH.md identifies, and that it states the fork must *consciously decide* whether to bring upstream's relaxed guard when tool-sandbox is absorbed |
| Won't-sync finding document for SEC-01 | SEC-01 | Absent subsystems (`nono-proxy/src/aws/*`, `tls_intercept/*`) — nothing to test | Mirror the shape of `.planning/phases/109-proxy-network-absorb/109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` |
| Standing-divergence addendum records all 18 SHAs with explicit dispositions + cited evidence | D-05 (LOCKED) | Ledger-document deliverable | Append to `108-DIVERGENCE-LEDGER.md` following the Phase 111 Standing Divergence Addendum shape (~line 1648). **Do NOT edit the "Ledger closed" declaration away** (~line 1642) |
| RES-01 skip reasoning recorded per-SHA | RES-01 (D-03, ROADMAP SC2) | Skip is a first-class outcome, but "silently dropped" is forbidden | Every one of the 4 RES-01 SHAs carries written reasoning in the ledger addendum |
| Live-kernel enforcement behavior | SEC-05, SEC-06 | **Explicitly rejected for this phase (D-06).** `cross test` + both clippy gates substitute; phase stays autonomous end-to-end | Out of scope — deferred to any future phase that stands up live-Linux verification |

---

## Inherited Baseline — do NOT mis-read as new regressions

Per `111-04-VERIFICATION-NOTES.md`, `cargo test --workspace --no-fail-fast` exits **101, not 0**,
on this host for pre-existing, phase-unrelated reasons.

- **24-name Phase 111 baseline** (11 `nono` `--bin` names + 13 across `audit_attestation.rs` / `env_vars.rs` / `resl_nix_async_signal_safety.rs`) — host-state/flake-driven (mandatory-label ACE contamination, hardcoded Unix path literals in Windows-run tests, a stale signature-match panic message). Confirmed not a code regression.
- **+3-name orchestrator addendum** — `profile_cmd.rs`: `test_init_creates_valid_profile`, `test_init_rejects_existing_file_without_force`, `test_schema_output_to_file`. Shared-fixture flake (fixed `%TEMP%\nono-test-profile-init` racing between parallel runner processes rather than `tempfile::TempDir`); passes 10/10 in isolation.

**Phase 112's baseline is the 27-name total.** Any NEW failing name outside that set is a real
regression signal. Any name already in the set reappearing is expected host noise.

*Fixing the `profile_cmd.rs` shared-fixture flake is explicitly out of scope (deferred in CONTEXT.md).*

---

## Validation Sign-Off

- [ ] All tasks have an automated verify command or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all ❌ MISSING references above
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s for targeted filters
- [ ] Both cross-target clippy gates GREEN (ROADMAP SC4)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
