---
phase: 49
slug: sigstore-trust-root-poc-resilience-from-file-flag-release-as
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-21
---

# Phase 49 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Failure modes and validation evidence sourced from 49-RESEARCH.md § "Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace-built-in test harness) |
| **Config file** | `Cargo.toml` (workspace) |
| **Quick run command** | `cargo test -p nono-cli --test setup_trust_root` |
| **Full suite command** | `make test` (workspace) |
| **Estimated runtime** | quick: ~30 s; full: ~5 min |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli --test setup_trust_root` (or `cargo test -p nono trust::bundle` for the freshness fn visibility-widen task)
- **After every plan wave:** Run `make ci` (clippy + fmt + tests)
- **Before `/gsd-verify-work`:** Full suite + cross-target clippy must be green
- **Max feedback latency:** 60 s for unit; 300 s for full

---

## Per-Task Verification Map

> Filled in by `gsd-planner` at plan-open. Each task in `49-01`, `49-02`, `49-03` plan files MUST map to one or more failure modes from §"Failure Modes (Nyquist Dimension 8)" below; the planner records `validates: F-XX-YY` in each task and copies the corresponding row here.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | 01 | 1 | REQ-POC-TRUST-01 | F-01-01 / F-01-02 / F-01-03 / F-01-04 / F-01-05 / F-01-06 / F-01-07 / F-01-08 | clap-mutex rejects, freshness gate / schema gate fails closed, no partial cache on IO error, stdout shape matches `--refresh-trust-root` with `Source:` breadcrumb, cross-target clippy clean, phase-index correct, freshness fn visible from nono-cli | integration (`assert_cmd` or raw `std::process::Command`) + cargo build + cargo clippy 3-target | `cargo test -p nono-cli --test setup_trust_root` + `cargo clippy --workspace --target {linux,macos,windows}` | ❌ W0 (test file extension OR new file) | ⬜ pending |
| TBD | 02 | 1 | REQ-POC-TRUST-02 | F-02-01 / F-02-02 / F-02-03 / F-02-04 / F-02-05 | release asset byte-identical to frozen fixture, listed in `softprops/action-gh-release` `files:`, hashed into `SHA256SUMS.txt`, CI step uses `set -euo pipefail`, working directories compose | shellcheck + yamllint + manual `act`-style local dry-run of the new release.yml block | `bash -n` on extracted block + `diff <(sha256sum src) <(sha256sum dst)` | ✅ (release.yml exists) | ⬜ pending |
| TBD | 03 | 1 | REQ-POC-TRUST-03 | F-03-01 / F-03-02 / F-03-03 / F-03-04 / F-03-05 | template file present, smoke scripts present and exec-bit set on `.sh`, doc has zero `(sigstore-verify 0.6.5)` / `P32-DEFER-005` / `deferred-items.md` matches, "Run once after install" block mentions `--from-file`, `.ps1` propagates exit codes | file-existence + negative-grep + positive-grep + manual `.ps1` exit-code check | `test -f` + `test -x` + `! grep ...` + `grep ...` | ❌ W0 (all three artifacts NEW) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Failure Modes (Nyquist Dimension 8)

Sourced from 49-RESEARCH.md § "Validation Architecture". Each plan's `verification_strategy` block MUST cite the IDs below.

### REQ-POC-TRUST-01

| ID | Failure Mode | Validation Gate |
|----|--------------|-----------------|
| F-01-01 | clap-mutex bypass (`--from-file <p> --refresh-trust-root` accepted) | `cargo test from_file_with_refresh_rejected` — `!status.success()` + stderr contains `cannot be used with` |
| F-01-02 | freshness gate bypass (expired tlog accepted) | `cargo test from_file_expired_fails_closed` — `!status.success()` + `!cache_path.exists()` |
| F-01-03 | schema bypass (malformed JSON accepted) | `cargo test from_file_malformed_fails_closed` (truncation + quote-flip cases) |
| F-01-04 | cache leak on copy failure (partial-write persists after IO error) | `cargo test from_file_missing_path_no_partial_cache` — `!cache_path.exists()` after a failed run |
| F-01-05 | stdout drift (D-49-B3 not honored) | `cargo test from_file_stdout_matches_refresh_shape` — output contains `[X/N] Loading...` + `Source: <path>` |
| F-01-06 | cross-target clippy regression | `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `--target x86_64-apple-darwin` (PARTIAL per `.planning/templates/cross-target-verify-checklist.md` if cross-toolchain unavailable) |
| F-01-07 | phase-index off-by-one (counter wrong at runtime) | `cargo test from_file_phase_index_matches_refresh` — parses `[X/N]` and asserts `X == self.refresh_trust_root_phase_index()` |
| F-01-08 | freshness fn still private (build breaks outside `crates/nono`) | `cargo build -p nono-cli` — exit 0 |

### REQ-POC-TRUST-02

| ID | Failure Mode | Validation Gate |
|----|--------------|-----------------|
| F-02-01 | byte-identity drift (`artifacts/trusted_root.json` ≠ fixture) | CI step `sha256sum` comparison — step exits non-zero on drift |
| F-02-02 | release-asset omission (not in `files:` glob) | `gh release view <tag> --json assets \| jq '.assets[].name'` contains `trusted_root.json` |
| F-02-03 | SHA256SUMS omission | `gh release download <tag> -p SHA256SUMS.txt && grep trusted_root.json SHA256SUMS.txt` exit 0 |
| F-02-04 | CI silent-pass (missing `set -euo pipefail`) | shellcheck on the new CI step + manual review confirms `set -euo pipefail` |
| F-02-05 | working-directory mismatch (paths don't compose) | review of `working-directory:` annotations OR `act` dry-run |

### REQ-POC-TRUST-03

| ID | Failure Mode | Validation Gate |
|----|--------------|-----------------|
| F-03-01 | cadence template absent | `test -f .planning/templates/sigstore-rotation-refresh.md` exit 0 |
| F-03-02 | smoke script absent or non-executable | `test -x scripts/verify-trust-root-cached.sh` AND `test -f scripts/verify-trust-root-cached.ps1` exit 0 |
| F-03-03 | doc stale cross-references | `! grep -E '(sigstore-verify 0\.6\.5\|P32-DEFER-005\|deferred-items\.md)' docs/cli/development/windows-poc-handoff.mdx` exit 0 (zero matches) |
| F-03-04 | "Run once after install" inconsistency | `grep -A 5 'Run once after install' docs/cli/development/windows-poc-handoff.mdx \| grep -q -- '--from-file'` exit 0 |
| F-03-05 | PowerShell script silent-failure (`$LASTEXITCODE` not checked) | Manual: run `.ps1` with corrupt `<PATH>` and verify exit code ≠ 0 |

---

## Wave 0 Requirements

- [ ] `crates/nono-cli/tests/setup_trust_root.rs` (or `setup_from_file.rs` if a new file) — integration test stubs for F-01-01 through F-01-07. Planner picks file name at plan-open after re-inventorying the existing `tests/` directory.
- [ ] `crates/nono-cli/tests/common/` — REUSE existing `test_env::{lock_env, EnvVarGuard}` primitives (Phase 44 D-44-E6); do NOT add new helpers unless a fixture-mutation helper is genuinely shared across multiple test files.
- [ ] `.planning/templates/sigstore-rotation-refresh.md` — NEW file (REQ-POC-TRUST-03).
- [ ] `scripts/verify-trust-root-cached.sh` — NEW file, `git update-index --chmod=+x` MUST be applied.
- [ ] `scripts/verify-trust-root-cached.ps1` — NEW file (no chmod-bit, but exit-code-propagating).

*No new framework dependencies. `tempfile` and `assert_cmd` (or `std::process::Command` per the existing pattern) are already in `crates/nono-cli/Cargo.toml` dev-deps.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Release asset uploaded by `softprops/action-gh-release` to a real GitHub Release | REQ-POC-TRUST-02 | Releases trigger on tag push; can't be tested in PR CI without creating a dry-run release | After Phase 49 ships, on the next tagged release (e.g., v2.6.0): `gh release view <tag> --json assets \| jq '.assets[].name'` must list `trusted_root.json`; `gh release download <tag> -p trusted_root.json && diff trusted_root.json crates/nono/tests/fixtures/trust-root-frozen.json` must exit 0 |
| Sigstore-rotation cadence template followed on next root rotation | REQ-POC-TRUST-03 | Cadence template is a maintainer-process document; correctness is asserted by the next rotation event itself, not by automated CI | Next time Sigstore announces a root rotation: maintainer follows `.planning/templates/sigstore-rotation-refresh.md` step-by-step; smoke script `scripts/verify-trust-root-cached.sh <new-fixture>` exits 0 before commit; PR opens against the fixture path; verify CI gates green |
| `.ps1` smoke script on real Windows host | REQ-POC-TRUST-03 | Windows shell semantics + `$LASTEXITCODE` propagation are easier to validate by hand than via WSL/`pwsh-in-CI` | After Plan 49-01 lands, run `scripts\verify-trust-root-cached.ps1 crates\nono\tests\fixtures\trust-root-frozen.json` on a Windows POC host; expect exit 0 + `nono trust verify` output |
| POC-handoff doc readability post-rewrite | REQ-POC-TRUST-03 | Prose quality is not grep-checkable; reviewer ensures the rewrite reads cleanly to a fresh POC user | Reviewer reads the rewritten "Known issue: Sigstore TUF root rotation" subsection + adjacent "Run once after install" block; confirms no leftover stale wording, recommendation flow is coherent |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies (only Manual-Only items above are exempt)
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all NEW files / test-file extensions (5 items listed above)
- [ ] No watch-mode flags
- [ ] Feedback latency < 60 s (quick) / < 300 s (full)
- [ ] Cross-target clippy commands cited (Plan 49-01 — Linux + macOS + Windows targets)
- [ ] `nyquist_compliant: true` set in frontmatter once planner finalizes Per-Task Verification Map

**Approval:** pending (set to `approved YYYY-MM-DD` after `gsd-plan-checker` Dimension 8 review)
