---
phase: 87-security-sync
plan: "03"
status: human_needed
verified_by: executor (Plan 87-03)
verified_date: 2026-06-20
---

# Phase 87: Security Sync — Verification Report

## Summary

Phase 87 delivers three security hardening commits:

| Commit | Fix | Requirement |
|--------|-----|-------------|
| `6cf2645c` | SEC-01: AF_UNIX datagram bypass — trap sendto/sendmsg/sendmmsg in seccomp BPF | SEC-01 |
| `abeb2493` | SEC-02: Guard deduplicate() against inheriting procfs-remap originals (cherry-pick 6b3eb013) | SEC-02 |
| `4a936f31` | CR-02: records_verified now false for empty logs — fork hardening | CR-02 |

All three code commits carry DCO `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`.
SEC-01 and SEC-02 carry upstream provenance lines (`cherry picked from commit ...`).

**Overall status: `human_needed`** — SEC-01 and SEC-02 have PARTIAL→CI legs that require
GH Actions Linux lane confirmation. CR-02 is fully VERIFIED locally (platform-agnostic fix,
no cfg-gating, regression test passes on Windows).

---

## Gate Results

### Gate 0 — Git commit provenance

```
git log --oneline -6 (Phase 87 commits)
```

| Commit | Description | Cherry-pick line | DCO |
|--------|-------------|-----------------|-----|
| `6cf2645c` | fix(linux): trap sendto/sendmsg/sendmmsg (SEC-01) | `(cherry picked from commit e2086877)` | PRESENT |
| `abeb2493` | fix: guard deduplicate() against inheriting procfs-remap originals (SEC-02) | `(cherry picked from commit 6b3eb0130031f6769e21d3a2f9d7d3534b400249)` | PRESENT (upstream + fork) |
| `4a936f31` | fix(audit): records_verified now false for empty logs — CR-02 fork hardening | N/A (fork-hardening) | PRESENT |

All DCO trailers verified.

### Gate 1 — cargo fmt check

```
cargo fmt --all -- --check
```

**Outcome:** INITIAL FAIL → CORRECTED → PASS (exit 0)

The SEC-01 cherry-pick (`6cf2645c`) contained formatting that did not match `rustfmt` output
for the Windows host's toolchain version. Specifically, several `assert_eq!` macro calls in
`linux.rs` test code and `use` import orderings in `supervisor_linux.rs` required reformatting.
`cargo fmt --all` was applied during Task 1 verification. The fmt fix was committed as part
of the Task 1 deliverable commit.

Files reformatted:
- `crates/nono/src/sandbox/linux.rs` (assert_eq! formatting in test module)
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` (use import ordering)

**Final status: PASS** — `cargo fmt --all -- --check` exits 0 after reformatting.

### Gate 2 — Windows-host cargo clippy

```
cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used
```

**Exit code: 0**

Output (final line): `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 21.50s`

**Status: PASS** (Windows-host target only — see Gate 4 for cross-target caveat)

Note: This exercises only Windows cfg branches. It is NOT a substitute for cross-target
clippy per CLAUDE.md §Coding Standards and `.planning/templates/cross-target-verify-checklist.md`.

### Gate 3 — Unit tests (Windows-host)

```
cargo test --workspace --all-targets
```

**Result: 778 passed; 1 failed** (exit code: 101 — non-zero due to known baseline failure)

The 1 failure is a documented pre-existing baseline:
```
sandbox::windows::tests::try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails
```

This test was failing BEFORE Phase 87 began. It is UNRELATED to Phase 87 (Phase 87 never
touched `crates/nono/src/sandbox/windows.rs`). It is an environment-specific failure requiring
`WRITE_OWNER` rights on a user-owned path at a drive root (`C:\poc\*`) that is not present
in this dev environment. This is a documented pre-existing baseline failure, NOT a Phase 87
regression.

**Flaky parallel failures (observed once, not in second run):**
During one run, two additional tests failed intermittently:
- `machine_policy::tests::windows_configured_key_is_not_unconfigured`
- `supervisor::aipc_sdk::tests::windows_loopback_tests::helper_stamps_session_token_from_env`

Both pass in isolation (`cargo test -p nono -- <test_name>`) and pass in the second full run.
These are pre-existing parallel-execution flakiness issues (registry key teardown order,
env-var isolation gaps) — NOT Phase 87 regressions.

**Seccomp/Landlock tests:** All `#[cfg(target_os = "linux")]`-gated BPF filter tests
(`test_build_seccomp_af_unix_filter_*`, `test_build_seccomp_af_unix_nogrant_filter_*`) and
procfs-remap tests (`remap_preserves_dev_null_when_deduped_with_dev_stdin`) report 0 tests
run on the Windows host. This is **expected** — these gates require Linux execution
(PARTIAL→CI per D-07).

#### Targeted regression tests

**SEC-02 regression test:**
```
cargo test -p nono -- remap_preserves_dev_null_when_deduped_with_dev_stdin
```
Result: `running 0 tests` — test is `#[cfg(target_os = "linux")]`-gated.
Status: **PARTIAL→CI** (Linux-execution-gated per D-07, deferred to GH Actions Linux lane)

**CR-02 regression test:**
```
cargo test -p nono -- verify_empty_log_with_no_stored_metadata_is_not_valid
```
Result: `running 1 test ... ok` — exit 0
Status: **PASS** (platform-agnostic test, no cfg-gating)

### Gate 4 — Cross-target clippy (expected PARTIAL)

#### Linux target (x86_64-unknown-linux-gnu)

```
rustup target add x86_64-unknown-linux-gnu   # already installed, up to date
cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
```

**Exit code: 1**

Actual error (leading cause):
```
error: failed to run custom build command for `aws-lc-sys v0.41.0`

Caused by: process didn't exit successfully (exit code: 1)
  warning: aws-lc-sys@0.41.0: Compiler family detection failed due to error: ToolNotFound:
    failed to find tool "x86_64-linux-gnu-gcc": program not found
```

> Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain
> (x86_64-unknown-linux-gnu). The live GH Actions Linux Clippy lane on the head SHA is the
> decisive signal per .planning/templates/cross-target-verify-checklist.md. SEC-01/SEC-02
> REQs marked PARTIAL pending CI confirmation.

**Status: PARTIAL→CI** — Windows-host `cargo check` is NOT accepted as a substitute per
CLAUDE.md §Coding Standards and cross-target-verify-checklist.md.

#### macOS target (x86_64-apple-darwin)

```
rustup target add x86_64-apple-darwin   # already installed, up to date
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

**Exit code: 1**

Actual error:
```
error: failed to run custom build command for `ring v0.17.14`

Caused by: ToolNotFound: failed to find tool "cc": program not found
```

> Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain
> (x86_64-apple-darwin). The live GH Actions macOS Clippy lane on the head SHA is the
> decisive signal per .planning/templates/cross-target-verify-checklist.md. SEC-01/SEC-02
> REQs marked PARTIAL pending CI confirmation.

**Status: PARTIAL→CI**

### Gate 5 — make ci equivalent (Windows host)

`make` is not available in the bash environment on this Windows host. The CI target is
equivalent to: `clippy + fmt-check + test + audit`. Each component was run individually:

| Component | Command | Exit code | Status |
|-----------|---------|-----------|--------|
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | 0 | PASS |
| fmt-check | `cargo fmt --all -- --check` | 0 | PASS |
| test | `cargo test --workspace --all-targets` | 101 | KNOWN BASELINE FAIL (1 pre-existing) |
| audit | `cargo audit` | 0 | PASS (4 allowed warnings, 0 errors) |

The non-zero exit from `cargo test` is exclusively from the documented pre-existing baseline
failure (`try_set_mandatory_label...`) that predates Phase 87. All Phase 87-relevant tests
either pass (CR-02) or are cfg-gated to Linux (SEC-01/SEC-02 BPF + procfs tests).

### Gate 6 — Divergence ledger and ADR verification

```
ls proj/ADR-87-cr02-audit-bypass.md   → EXISTS (3899 bytes)
grep -c "CR-02" .planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md → 2 matches
```

T-87-10 (deliberate-divergence documentation gap) is CLOSED: ADR-87 exists and the Phase 87
CR-02 addendum is present in the divergence ledger.

---

## Per-Requirement Disposition

### SEC-01 — AF_UNIX datagram bypass (sendto/sendmsg/sendmmsg)

**Disposition: PARTIAL**

| Gate | Status | Notes |
|------|--------|-------|
| Windows-host cargo check | PASS | Native target compiles clean |
| Windows-host clippy | PASS | `-D warnings -D clippy::unwrap_used` clean |
| Cross-target clippy (linux) | PARTIAL→CI | `x86_64-linux-gnu-gcc` not found on dev host |
| Cross-target clippy (macOS) | PARTIAL→CI | `cc` not found on dev host |
| BPF filter unit tests | PARTIAL→CI | `#[cfg(target_os = "linux")]` — not executed on Windows |
| Seccomp runtime tests | PARTIAL→CI | Linux-execution-gated (D-07) |
| Fork-specific tests | PARTIAL→CI | `af_unix_pathname_sendto_is_allowed_by_grant`, etc. — linux-gated |

Decisive gate: GH Actions Linux Clippy + Test lanes on commit `6cf2645c` (or HEAD if formatting
commit landed after).

**Windows-host `cargo check` is NOT accepted as a substitute** per CLAUDE.md §Coding Standards
and cross-target-verify-checklist.md §NEVER.

### SEC-02 — procfs-remap dedup guard

**Disposition: PARTIAL**

| Gate | Status | Notes |
|------|--------|-------|
| Windows-host cargo check | PASS | Native target compiles clean |
| Windows-host clippy | PASS | `-D warnings -D clippy::unwrap_used` clean |
| Cross-target clippy (linux) | PARTIAL→CI | `x86_64-linux-gnu-gcc` not found on dev host |
| Cross-target clippy (macOS) | PARTIAL→CI | `cc` not found on dev host |
| Regression test (Windows) | PARTIAL→CI | `remap_preserves_dev_null_when_deduped_with_dev_stdin` is `#[cfg(target_os = "linux")]`-gated; 0 tests run on Windows — Linux-execution-gated per D-07 |

Decisive gate: GH Actions Linux Clippy + Test lanes on commit `abeb2493`.

**Windows-host `cargo check` is NOT accepted as a substitute** per CLAUDE.md §Coding Standards
and cross-target-verify-checklist.md §NEVER.

### CR-02 — Audit-integrity bypass fix

**Disposition: VERIFIED (locally)**

| Gate | Status | Notes |
|------|--------|-------|
| Windows-host cargo check | PASS | Native target compiles clean |
| Windows-host clippy | PASS | `-D warnings -D clippy::unwrap_used` clean |
| Cross-target clippy | N/A | `audit.rs` has no cfg-gating; fix is platform-agnostic |
| Regression test | PASS | `verify_empty_log_with_no_stored_metadata_is_not_valid` — 1 passed, exit 0 |
| ADR-87 file | PASS | `proj/ADR-87-cr02-audit-bypass.md` exists |
| Divergence ledger addendum | PASS | 2 CR-02 references in `85-DIVERGENCE-LEDGER.md` |

The `records_verified: event_count > 0` fix is in `crates/nono/src/audit.rs` which has
no `#[cfg(target_os)]` guards — it runs on all platforms. Cross-target clippy is not
required by the checklist for platform-agnostic files. VERIFIED locally.

---

## PARTIAL→CI Deferral Record

### Deferral 1: Cross-target clippy — x86_64-unknown-linux-gnu

**Applies to:** SEC-01, SEC-02

**Reason:** `aws-lc-sys` requires `x86_64-linux-gnu-gcc` (C cross-compiler) which is not
installed on this Windows 11 dev host (Win32 GCC cross-toolchain unavailable via rustup alone).

**Actual error:**
```
error: failed to run custom build command for `aws-lc-sys v0.41.0`
warning: aws-lc-sys@0.41.0: ToolNotFound: failed to find tool "x86_64-linux-gnu-gcc":
  program not found
```

**Per `.planning/templates/cross-target-verify-checklist.md` §PARTIAL Disposition:**

> Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain
> (x86_64-unknown-linux-gnu). The live GH Actions Linux Clippy lane on the head SHA is the
> decisive signal per .planning/templates/cross-target-verify-checklist.md. REQ marked
> PARTIAL pending CI confirmation.

### Deferral 2: Cross-target clippy — x86_64-apple-darwin

**Applies to:** SEC-01, SEC-02

**Reason:** macOS cross-toolchain (`cc` / Apple clang) unavailable on Windows dev host.

**Actual error:**
```
error: failed to run custom build command for `ring v0.17.14`
ToolNotFound: failed to find tool "cc": program not found
```

**Per `.planning/templates/cross-target-verify-checklist.md` §PARTIAL Disposition:**

> Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain
> (x86_64-apple-darwin). The live GH Actions macOS Clippy lane on the head SHA is the
> decisive signal per .planning/templates/cross-target-verify-checklist.md. REQ marked
> PARTIAL pending CI confirmation.

### Deferral 3: Linux-execution gates (D-07)

**Applies to:** SEC-01 (BPF filter tests), SEC-02 (procfs-remap regression test)

**Per D-07 in `87-CONTEXT.md`:**
> The Linux-execution leg is PARTIAL→CI — this is a Windows dev-host; seccomp tests
> can't run locally. Live GH Actions Linux lane is the decisive gate.

The following tests are `#[cfg(target_os = "linux")]`-gated and run 0 tests on Windows:
- `test_build_seccomp_af_unix_filter_count` (SEC-01 — BPF instruction count)
- `test_build_seccomp_af_unix_nogrant_filter_denies_send_family` (SEC-01 — no-grant filter)
- `test_build_seccomp_proxy_filter_count` (SEC-01 — proxy filter update)
- `remap_preserves_dev_null_when_deduped_with_dev_stdin` (SEC-02 — regression)
- `af_unix_pathname_sendto_is_allowed_by_grant` (SEC-01 — integration)
- `af_unix_abstract_sendto_is_denied` (SEC-01 — integration)
- `af_unix_only_mode_allows_non_af_unix_sendto` (SEC-01 — integration)

These must execute on GH Actions Linux lane for SEC-01/SEC-02 to move from PARTIAL to VERIFIED.

---

## Residual Risk

| Risk | Severity | Notes |
|------|----------|-------|
| SEC-01 BPF jt offsets may have Windows-invisible arithmetic error | HIGH | Cannot validate without Linux execution — GH Actions decisive |
| SEC-01 sendmsg/sendmmsg NULL fast-paths may have edge cases on real kernel | MEDIUM | Integration tests cfg-gated to Linux; GH Actions decisive |
| SEC-02 dedup guard may miss an additional call site | LOW | cherry-pick applied clean; single guard site in deduplicate() per upstream |
| Pre-existing `try_set_mandatory_label` test failure | LOW | Documented baseline, unrelated to Phase 87 |
| `rustls-pemfile` advisory (RUSTSEC-2025-0134) | LOW | Pre-existing allowed warning, not a Phase 87 regression |

---

## Human Verification Truths

Before marking SEC-01 and SEC-02 as VERIFIED:

1. GH Actions Linux Clippy lane reports green on HEAD SHA (`milestone/v2.13-carryforward-closeout`)
   for `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
2. GH Actions Linux Test lane reports green for all `#[cfg(target_os = "linux")]` BPF filter tests
3. GH Actions Linux Test lane reports `remap_preserves_dev_null_when_deduped_with_dev_stdin` PASS
4. GH Actions macOS Clippy lane reports green on HEAD SHA for `--target x86_64-apple-darwin`
