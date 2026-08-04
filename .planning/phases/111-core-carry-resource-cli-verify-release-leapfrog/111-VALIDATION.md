---
phase: 111
slug: core-carry-resource-cli-verify-release-leapfrog
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-04
task_map_bound: 2026-08-04
---

# Phase 111 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `111-RESEARCH.md` § Validation Architecture (2026-08-04).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` (workspace-native). No snapshot framework (`insta`/`trycmd`) present in `nono-cli`. |
| **Config file** | none dedicated — per-crate `Cargo.toml` + integration-test-dir convention |
| **Quick run command** | `cargo test -p nono-sandbox-cli --bin nono -- <filter>` |
| **Full suite command** | `cargo test --workspace --no-fail-fast` |
| **Estimated runtime** | quick ~15-40s (filtered) · full suite ~6-10 min |

**Host constraints (must be honored by every plan):**
- `make` is **NOT on PATH**. Substitute the constituent `cargo` invocations (see `111-RESEARCH.md` § VERIFY-01 `make ci` substitution).
- Fork-owned package selectors: `-p nono-sandbox-cli`, **not** `-p nono-cli` (Phase 102 `package =` rename).
- `--no-fail-fast` is **mandatory** on the full suite — without it the run stops at the first failing binary and the 24-failure baseline is not observable.

---

## Sampling Rate

- **After every task commit:** targeted `cargo test -p nono-sandbox-cli --bin nono -- <filter>` scoped to the area touched (`policy` / `exec_strategy` / flag-parser guards).
- **After every wave touching cfg-gated Unix surfaces** (`policy.json`, `exec_strategy*.rs`, `cli.rs`): **both** cross-target clippy gates — mandatory per D-10, not optional, not deferrable to CI.
- **Phase gate (before `/gsd:verify-work`):** full `make ci` substitution set + `cargo test --workspace --no-fail-fast` (baseline-diffed) + both binding rebuilds + `release-readiness.ps1` + `release-dry-run.ps1`.
- **Max feedback latency:** ~40s for targeted runs; ~10 min for the full phase gate.

---

## Per-Task Verification Map

> Bound to the 6 plans created 2026-08-04 and confirmed against them by `gsd-plan-checker`.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 111-01-01 | 01 | 1 | CORE-01 | — | `user_caches_macos.allow.readwrite` contains `~/.cache` | unit | `cargo test -p nono-sandbox-cli -- policy` | ❌ W0 — by-value assertion written by this task | ⬜ pending |
| 111-01-02 | 01 | 1 | CORE-01 | — | `MAX_CRYPTO_THREADS == 12` | unit | `cargo test -p nono-sandbox-cli --bin nono -- exec_strategy` | ❌ W0 — direct assertion written by this task | ⬜ pending |
| 111-02-01 | 02 | 1 | CORE-02 | T-111-01 | `run --help` states the **true** per-platform enforcement, neither understating nor overstating | unit + manual | `cargo test -p nono-sandbox-cli --bin nono -- cpu_percent_range_enforced_by_clap max_processes_range_enforced_by_clap memory_zero_rejected_by_parser`; editorial review vs `launch_runtime.rs` L180-234 | ✅ | ⬜ pending |
| 111-02-02 | 02 | 1 | CORE-02 | T-111-01 | Docs site no longer contradicts the corrected `--help` | grep + manual | `test "$(grep -c 'accepted with a warning pending cross-platform follow-up' docs/cli/usage/flags.mdx)" = "0"` | ✅ | ⬜ pending |
| 111-03-01 | 03 | 1 | CORE-02 | — | ADR-111 settles the D-01 disposition with reasoning, not just a conclusion | grep + manual | `test -f proj/ADR-111-resource-limits-boundary.md` + both-SHA greps | ✅ | ⬜ pending |
| 111-03-02 | 03 | 1 | CORE-02 | — | Standing divergence recorded in the 108 ledger | grep + manual | `grep -q '## Phase 111 Standing Divergence Addendum' …/108-DIVERGENCE-LEDGER.md` | ✅ | ⬜ pending |
| 111-04-01 | 04 | 2 | VERIFY-01 | — | Both cross-target clippy gates GREEN (`SDKROOT` unset for darwin); D-01/ADR-86 boundary grep-diff clean | gate | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` + `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✅ | ⬜ pending |
| 111-04-02 | 04 | 2 | VERIFY-01 | — | `make ci` constituents GREEN; failure set **equals** the documented 24-name baseline — no new regressions, no fabricated GREEN | gate | `cargo test --workspace --no-fail-fast` diffed against the 24-name list | ✅ | ⬜ pending |
| 111-04-03 | 04 | 2 | VERIFY-01 | — | Both bindings rebuild clean at the pre-bump version (struct/API compat, not version numbers) | integration | `maturin build` in `../nono-py`; `npx napi build --platform --release` in `../nono-ts` | ✅ | ⬜ pending |
| 111-05-01 | 05 | 3 | RLS-14 | T-111-03 | All 6 version-family crates report `0.70.0`; `tools/sign-fixture` stays at its independent `0.1.0`; `Cargo.lock` drift limited to bumped packages | gate | `cargo metadata` / version grep + `cargo build --workspace --all-targets` | ✅ | ⬜ pending |
| 111-05-02 | 05 | 3 | RLS-14 | T-111-03 | The two hardcoded-version gate scripts assert against `0.70.0`/`0.69.0` — **not** vacuously against `0.66.1`/`0.66.0` | gate | `pwsh -File scripts/gates/release-readiness.ps1` | ✅ after the required script edit | ⬜ pending |
| 111-06-01 | 06 | 4 | RLS-14 | T-111-03 | `../nono-py` at `0.70.0` and rebuilds clean (only `maturin` catches struct drift) | integration | `maturin build` in `../nono-py` | ✅ | ⬜ pending |
| 111-06-02 | 06 | 4 | RLS-14 | T-111-03 | `../nono-ts` at `0.70.0` **including** its live `version = "0.66"` path-dep requirement → `"0.70"`; rebuilds clean | integration | `npx napi build --platform --release` in `../nono-ts` | ✅ | ⬜ pending |
| 111-06-03 | 06 | 4 | RLS-14 | T-111-03 | Prepare-only release gate GREEN — **no tag push, no registry upload** | gate | `pwsh -File scripts/release-dry-run.ps1` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] By-value assertion that `crates/nono-cli/data/policy.json`'s `user_caches_macos` group's `readwrite` array contains `~/.cache` — existing tests only prove the group *loads*, not its contents.
- [ ] Direct `assert_eq!(MAX_CRYPTO_THREADS, 12)` — the constant is currently only exercised indirectly through threading-guard match arms, which would still pass at the old value of 7.

*No test-framework install is needed — `cargo test` infrastructure already covers both gaps.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Corrected `--memory` / `--timeout` / `--max-processes` help text is accurate per platform | CORE-02 | No snapshot-test framework exists in `nono-cli`; ~150 other flags likewise have no snapshot coverage. Adding one solely for this phase is out of scope. | Run `cargo run -p nono-sandbox-cli -- run --help`; diff the three flag descriptions against the per-platform enforcement table in `111-RESEARCH.md`. Confirm the macOS `RLIMIT_AS` caveat is present **and** that it does not overstate (research found `setrlimit(RLIMIT_AS)` can fail outright with `EINVAL` on Apple Silicon, silently — the text must be best-effort, not guaranteed). |
| ADR-111 settles the disposition and records the standing divergence | CORE-02 (D-03) | Prose artifact — correctness is editorial, not executable. | Confirm `proj/ADR-111-resource-limits-boundary.md` exists, mirrors `ADR-108`'s shape, and explicitly answers why ADR-86's audit/diagnostics carve-out does **not** extend to resource limits. |
| Standing-divergence entry recorded in the 108 ledger | CORE-02 (D-02) | Prose artifact in a hand-maintained ledger. | Confirm `108-DIVERGENCE-LEDGER.md` has an explicit standing-divergence entry for `e6d26871` (#1269), matching the treatment the tool-sandbox subsystem got when routed to v3.7. |

---

## Validation Sign-Off

- [x] All tasks have an automated verify command or a declared Wave 0 dependency
- [x] Sampling continuity: no 3 consecutive tasks without an automated verify — closed 2026-08-04 by wiring greps into `111-02-02`, `111-03-01`, `111-03-02` and the real build commands into `111-04-03`
- [x] Wave 0 covers both MISSING assertions (`~/.cache`, `MAX_CRYPTO_THREADS`) — planned in `111-01-01` / `111-01-02`
- [x] No watch-mode flags
- [x] Baseline honesty: the 24-failure baseline is **diffed**, never re-litigated and never reported as GREEN — enforced by `111-04-02`
- [x] Feedback latency < 40s for targeted runs
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-04 (gsd-plan-checker: VERIFICATION PASSED, 0 blockers; the 3 non-blocking warnings it raised were applied to the plans and to this file before approval)
