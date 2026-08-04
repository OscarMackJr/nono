---
phase: 111
slug: core-carry-resource-cli-verify-release-leapfrog
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-04
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

> Task IDs are populated by `gsd-planner`. The requirement→command rows below are the
> authoritative validation contract each task must map onto.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | 1 | CORE-01 | — | `user_caches_macos.allow.readwrite` contains `~/.cache` | unit | `cargo test -p nono-sandbox-cli -- policy` | ❌ W0 — new by-value assertion needed | ⬜ pending |
| TBD | TBD | 1 | CORE-01 | — | `MAX_CRYPTO_THREADS == 12` | unit | `cargo test -p nono-sandbox-cli --bin nono -- exec_strategy` | ❌ W0 — direct assertion needed | ⬜ pending |
| TBD | TBD | 1 | CORE-02 | T-111-01 | `run --help` states the **true** per-platform enforcement, neither understating nor overstating | manual / doc-review | `cargo run -p nono-sandbox-cli -- run --help` | ✅ (no snapshot test exists) | ⬜ pending |
| TBD | TBD | 1 | CORE-02 | T-111-01 | Existing flag-parsing regression guards pass **unmodified** | unit | `cargo test -p nono-sandbox-cli --bin nono -- cpu_percent_range_enforced_by_clap max_processes_range_enforced_by_clap memory_zero_rejected_by_parser` | ✅ | ⬜ pending |
| TBD | TBD | 2 | VERIFY-01 | — | Linux cross-target clippy GREEN | gate | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | ✅ | ⬜ pending |
| TBD | TBD | 2 | VERIFY-01 | — | macOS cross-target clippy GREEN (`SDKROOT` unset) | gate | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✅ | ⬜ pending |
| TBD | TBD | 2 | VERIFY-01 | — | `make ci` constituents GREEN (clippy + fmt-check + tests + audit) | gate | see `111-RESEARCH.md` § `make ci` substitution | ✅ | ⬜ pending |
| TBD | TBD | 2 | VERIFY-01 | — | Failure set **equals** the documented 24-name baseline — no new regressions, no fabricated GREEN | gate | `cargo test --workspace --no-fail-fast` diffed against the 24-name list | ✅ | ⬜ pending |
| TBD | TBD | 3 | RLS-14 | T-111-03 | All 6 version-family crates report `0.70.0`; `tools/sign-fixture` stays at its independent `0.1.0` | gate | `scripts/gates/release-readiness.ps1` (after updating its hardcoded `$targetVersion`/`$upstreamHighest`) | ✅ after required script edit | ⬜ pending |
| TBD | TBD | 3 | RLS-14 | T-111-03 | Prepare-only release gate GREEN — **no tag push, no registry upload** | gate | `pwsh -File scripts/release-dry-run.ps1` | ✅ | ⬜ pending |
| TBD | TBD | 3 | RLS-14 | — | Both sibling binding repos rebuild clean at `0.70.0` (only `maturin`/`napi` catch struct drift) | integration | `maturin build` in `../nono-py`; `npx napi build --platform --release` in `../nono-ts` | ✅ | ⬜ pending |

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

- [ ] All tasks have an automated verify command or a declared Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without an automated verify
- [ ] Wave 0 covers both MISSING assertions (`~/.cache`, `MAX_CRYPTO_THREADS`)
- [ ] No watch-mode flags
- [ ] Baseline honesty: the 24-failure baseline is **diffed**, never re-litigated and never reported as GREEN
- [ ] Feedback latency < 40s for targeted runs
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
