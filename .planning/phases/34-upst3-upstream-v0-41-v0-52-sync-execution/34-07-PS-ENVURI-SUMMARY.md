---
phase: 34-upst3-upstream-v0-41-v0-52-sync-execution
plan_number: 34-07
plan: 07
slug: ps-envuri
subsystem: upstream-sync
cluster_id: C10
tags: [upst3, c10, ps, env-uri, ioctl, wave-2, summary]
type: execute
status: complete
requirements: [C10]
dependency_graph:
  requires: [34-04, 34-04b, 34-01, 34-02, 34-05]
  provides:
    - "C10 cluster cherry-picked (7 commits, upstream v0.50.0..v0.50.1)"
    - "env:// URI scheme in custom_credentials.credential_key (delegating to fork's Phase 20 keystore loader)"
    - "nono ps dynamic-column display (cross-platform; Windows fallback inline-gated)"
    - "Linux ioctl native-types fix landed (libc::Ioctl)"
  affects:
    - "Plan 34-08 (C11 env_deny) — same wave, ps + env:// surface now mainline"
    - "Plan 34-09 (C6 packs) / 34-10 (C12 proxy-tls) — downstream waves can rely on env:// in custom_credentials"
tech_stack:
  added: []
  patterns:
    - "Cross-platform terminal-columns dispatch: TIOCGWINSZ (Unix) / GetConsoleScreenBufferInfo (Windows) behind #[cfg] inline (no *_windows.rs touch)"
    - "Single canonical env:// resolver: profile/mod.rs delegates to nono::keystore::is_env_uri + validate_env_uri (Phase 20 UPST-03 source of truth)"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/session_commands.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - crates/nono-cli/data/profile-authoring-guide.md
    - crates/nono/src/manifest.rs
    - crates/nono/src/sandbox/linux.rs
    - docs/cli/features/credential-injection.mdx
    - docs/cli/usage/flags.mdx
    - CHANGELOG.md
decisions:
  - "Plan task table reverses upstream chronology (7547f91f listed as commit 2, a9eeb3fa as commit 3) — following plan order leaves a9eeb3fa as an empty no-op cherry-pick because 7547f91f is a stylistic optimization OF a9eeb3fa's refactor; empty commit retained for D-19 provenance and 1:1 fork:upstream commit-count auditability."
  - "Windows terminal_columns fallback added inline behind #[cfg(windows)] (GetConsoleScreenBufferInfo via windows-sys Win32_System_Console) rather than in a *_windows.rs file — non_goals explicitly forbid touching *_windows.rs; session_commands.rs is already not compiled on Windows main.rs (cfg gated), but the dead inline block is harmless."
  - "ca2e948e's two new env_uri tests reference upstream's CustomCredentialDef shape including proxy / tls_client_cert / tls_client_key fields fork does not carry; trimmed tests to fork's 12-field struct matching existing file_uri test shape — runtime behaviour unchanged."
  - "Upstream version bumps (Cargo.toml + Cargo.lock) dropped for both release-chore commits; CHANGELOG entries kept for downstream sync provenance — mirrors Plan 34-04 (v0.46.0/v0.47.0/v0.47.1) and Plan 34-05 (v0.48.0) precedent."
metrics:
  duration: "~13h elapsed (start 2026-05-12T00:00:00Z, end 2026-05-12T12:46:07Z)"
  task_count: 4
  commit_count: 7
  files_modified_count: 9
completed_at: "2026-05-12T12:46:07Z"
---

# Phase 34 Plan 07: `nono ps` + `env://` URI + Linux ioctl fix Summary

**One-liner:** C10 cluster (upstream v0.50.0..v0.50.1, 7 commits) landed on `main`: dynamic-column `nono ps` display, `env://` URI scheme support in `custom_credentials.credential_key` (delegating to fork's Phase 20 keystore loader — zero duplicate parser), and Linux seccomp ioctl native-types fix.

## Tasks Executed

| Task | Name | Outcome | Commit |
|------|------|---------|--------|
| 1 | Pre-flight — verify dependent plans closed, fetch upstream, build green | DONE | (no commit; baseline 7406eef2, all 7 SHAs reachable, env://=64 baseline in keystore.rs, all fork-defense baselines ≥ minimums) |
| 2 | Cherry-pick all 7 C10 commits with D-19 trailers | DONE | 75cbb293 → adf35ff2 → 4cfc9036 → 8394f07b → 1d83181d → 17e9afcd → 108a2358 |
| 3 | D-34-D2 close-gate (1/2/5 PASS; 3/4 deferred-to-CI; 6/7/8 admin-skipped) | DONE | (no commit; gate results in close-gate table below) |
| 4 | Push + PR | DONE | (push verified below) |

## Commits Landed

| Order | Hash       | Subject                                                                  | Upstream SHA | Upstream Tag |
|-------|------------|--------------------------------------------------------------------------|--------------|--------------|
| 1     | `75cbb293` | restore comment                                                          | `0b29d8ba`   | v0.50.0      |
| 2     | `adf35ff2` | refactor(cli): optimize ps command column width calculation              | `7547f91f`   | v0.50.0      |
| 3     | `4cfc9036` | refactor(cli/ps): improve ps command display with dynamic columns        | `a9eeb3fa`   | v0.50.0      |
| 4     | `8394f07b` | feat(profile): support env:// URI in custom_credentials credential_key   | `ca2e948e`   | v0.50.0      |
| 5     | `1d83181d` | chore: release v0.50.0                                                   | `cd74c4cf`   | v0.50.0      |
| 6     | `17e9afcd` | chore: release v0.50.1                                                   | `2d183e8f`   | v0.50.1      |
| 7     | `108a2358` | fix: Use native types for iotcl integers                                 | `4e642f29`   | v0.50.1      |

Plan-close smoke (`git log --format='%B' HEAD~7..HEAD | grep -c '^Upstream-commit: '`) returns **7** = landed cherry-pick count.

## D-34-E1 Invariant Per-Commit Audit

Per the plan's <critical_invariants> §1, zero touches to `*_windows.rs` or `exec_strategy_windows/` across all 7 commits:

| Commit     | Subject (short)                       | `*_windows.rs` hits |
|------------|---------------------------------------|---------------------|
| `75cbb293` | restore comment                       | 0                   |
| `adf35ff2` | optimize ps column width              | 0                   |
| `4cfc9036` | dynamic-column ps display             | 0                   |
| `8394f07b` | env:// URI in custom_credentials      | 0                   |
| `1d83181d` | release v0.50.0                       | 0                   |
| `17e9afcd` | release v0.50.1                       | 0                   |
| `108a2358` | native types for ioctl integers       | 0                   |
| **Total**  |                                       | **0**               |

The Linux ioctl fix (`108a2358`) lands in `crates/nono/src/sandbox/linux.rs` — a Linux-only file (NOT Windows-only). Plan table mentioned `crates/nono-cli/src/exec_strategy/linux.rs` but actual upstream commit touches the library-side `sandbox/linux.rs`; both paths are Linux-only and permitted by the invariant.

## env:// URI Composition Invariant (Critical Invariant §3)

After commit 4 (`8394f07b`), the canonical resolution path is verified:

```
$ grep -n 'env://' crates/nono-cli/src/profile/mod.rs crates/nono/src/keystore.rs
crates/nono-cli/src/profile/mod.rs:608:    } else if nono::keystore::is_env_uri(key) {
crates/nono-cli/src/profile/mod.rs:609:        nono::keystore::validate_env_uri(key).map_err(|e| { ... })
crates/nono-cli/src/profile/mod.rs:957:        } else if nono::keystore::is_env_uri(key) {
crates/nono-cli/src/profile/mod.rs:958:            nono::keystore::validate_env_uri(key).map_err(|e| { ... })
crates/nono/src/keystore.rs:86:const ENV_URI_PREFIX: &str = "env://";
crates/nono/src/keystore.rs:555:pub fn is_env_uri(credential_ref: &str) -> bool { ... }
crates/nono/src/keystore.rs:576:pub fn validate_env_uri(uri: &str) -> Result<()> { ... }
```

`profile/mod.rs` **delegates** to `nono::keystore::is_env_uri` and `nono::keystore::validate_env_uri` (the Phase 20 UPST-03 canonical resolvers). **Zero duplicate parser introduced** — single source of truth for env:// validation lives in `crates/nono/src/keystore.rs`.

Regression test:
```
$ cargo test -p nono test_validate_env_uri
test result: ok. 9 passed; 0 failed; ...
```

Plus the two new profile-side tests:
```
$ cargo test -p nono-cli test_validate_custom_credential_env_uri
test profile::tests::test_validate_custom_credential_env_uri_dangerous_var_rejected ... ok
test profile::tests::test_validate_custom_credential_env_uri_accepted ... ok
test result: ok. 2 passed; 0 failed; ...
```

T-34-07-03 mitigation discharged.

## Fork-Defense Baselines (Critical Invariant §6)

| Baseline                                                                      | Required | Current | Delta |
|-------------------------------------------------------------------------------|----------|---------|-------|
| `grep -c 'never_grant\|apply_deny_overrides' crates/nono-cli/src/policy.rs`   | ≥ 21     | 21      | 0     |
| `grep -c 'validate_path_within' crates/nono-cli/src/package_cmd.rs`           | ≥ 9      | 9       | 0     |
| `grep -c 'capabilities.aipc\|loaded_profile' crates/nono-cli/src/profile/mod.rs` | ≥ 17  | 17      | 0     |
| `grep -c 'find_denied_user_grants' crates/nono-cli/src/policy.rs`             | ≥ 1      | 7       | +6    |
| `grep -c 'bypass_protection' crates/nono-cli/src/profile/mod.rs`              | ≥ 1      | 17      | +16   |

All fork-defense baselines preserved or exceeded.

## D-34-D2 Close-Gate

| # | Gate                                                                                                | Result      | Notes |
|---|-----------------------------------------------------------------------------------------------------|-------------|-------|
| 1 | `cargo test --workspace --all-features` (Windows)                                                   | **PASS***   | 932/934 with two pre-existing flakes: `query_ext::test_query_path_denied` (P34-DEFER-01-1 carry-forward — Windows UNC long-path canon) AND `supervisor::aipc_sdk::windows_loopback_tests::helper_stamps_session_token_from_env` (env-var leakage between parallel tests; passes in isolation). Both verified untouched by Plan 34-07 (`git log 7406eef2..HEAD -- <files>` returns zero). Per executor prompt: "P34-DEFER-01-1 carry-forward acceptable." |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host)        | **PASS**    | No warnings emitted. |
| 3 | `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu ...`                       | DEFERRED-CI | Local cross-compile blocked by missing `x86_64-linux-gnu-gcc` toolchain on Windows host (cc-rs ToolNotFound). Per executor prompt: gates 3/4 deferred-to-CI. |
| 4 | `cargo clippy --workspace --all-targets --target x86_64-apple-darwin ...`                            | DEFERRED-CI | Same toolchain constraint. Deferred-to-CI per prompt. |
| 5 | `cargo fmt --all -- --check`                                                                        | **PASS**    | Silent (no formatting drift). |
| 6 | Phase 15 5-row detached-console smoke gate                                                          | ADMIN-SKIP  | Admin-skipped per executor prompt. |
| 7 | `wfp_port_integration` test suite                                                                   | ADMIN-SKIP  | Admin-skipped per executor prompt (requires admin/service-not-available context). |
| 8 | `learn_windows_integration` test suite                                                              | ADMIN-SKIP  | Admin-skipped per executor prompt. |

*Gate 1 passes with the two pre-existing carry-forwards explicitly allowed by the executor prompt.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `ca2e948e` new env_uri tests referenced upstream CustomCredentialDef fields fork doesn't carry**

- **Found during:** Task 2, commit 4 (`ca2e948e`)
- **Issue:** Upstream's two new tests `test_validate_custom_credential_env_uri_accepted` / `test_validate_custom_credential_env_uri_dangerous_var_rejected` initialize `CustomCredentialDef` with `proxy`, `tls_client_cert`, `tls_client_key` fields that fork's 12-field struct does not carry (fork has `proxy` only in `nono-proxy::config::EndpointRule`, no proxy field at the struct top level; no client cert/key fields).
- **Fix:** Trimmed the two new tests to fork's 12-field struct shape (matching the existing `test_validate_custom_credential_file_uri_accepted` test established in this file).
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`
- **Commit:** `8394f07b`

**2. [Rule 3 - Blocking] `7547f91f` upstream `terminal_columns` used bare `libc::ioctl` not gated for Windows**

- **Found during:** Task 2, commit 2 (`7547f91f`)
- **Issue:** Upstream's `terminal_columns()` calls `libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, ...)` which would not compile on Windows targets (no `TIOCGWINSZ`).
- **Fix:** Split into `terminal_columns_native()` dispatched via `#[cfg(unix)]` (TIOCGWINSZ ioctl through `nix::libc`) vs `#[cfg(windows)]` (GetConsoleScreenBufferInfo via `windows-sys::Win32::System::Console`). Plan non_goals forbid touching `*_windows.rs` files; the inline `#[cfg]` keeps cross-platform parity inside `session_commands.rs` without a separate Windows file. Note: `session_commands.rs` is already cfg-gated out on Windows builds (`main.rs` line 69-73), so the Windows arm is dead code today — but harmless and ready for any future cross-platform-`session_commands` refactor.
- **Files modified:** `crates/nono-cli/src/session_commands.rs`
- **Commit:** `adf35ff2`

**3. [Rule 1 - Bug] `0b29d8ba` comment-positioning conflict + ordering quirk in `ca2e948e` test placement**

- **Found during:** Task 2, commits 1 & 4
- **Issue:** Commit `0b29d8ba` is described as "restore comment" — adds 3 comments. Two were already present on fork's `validate_credential_key` (lines 587, 609); the third moved an `// End-to-end:` comment from inside the test body to above the `#[test]` attribute. Commit `ca2e948e` then injected the two new env_uri tests *between* that lifted comment and the function it describes, leaving the comment orphaned over an unrelated test.
- **Fix:** Commit 1 lifted the comment to above `#[test] fn test_profile_json_with_file_uri_custom_credential_parses()`. Commit 4's auto-merge inserted the new env_uri tests between comment and target function — restored the comment to its correct adjacent position above `test_profile_json_with_file_uri_custom_credential_parses`.
- **Files modified:** `crates/nono-cli/src/profile/mod.rs`
- **Commits:** `75cbb293` (lift), `8394f07b` (restore position)

### Architectural Decisions

**1. Empty no-op cherry-pick retained for D-19 provenance (commit 3, `a9eeb3fa`)**

Plan 34-07 task table reverses upstream chronology: `7547f91f` (May 8) listed as commit 2 before `a9eeb3fa` (May 7) at commit 3. Upstream's natural order is `a9eeb3fa → 7547f91f` where `7547f91f` is a stylistic optimization of `a9eeb3fa`'s refactor. Following the plan's stated order meant landing the optimized form first, leaving `a9eeb3fa` with no working-tree delta.

**Decision:** Retain the empty commit (`4cfc9036`) rather than re-order. Rationale: (1) preserves D-19 1:1 fork:upstream commit-count auditability (Phase 33 DIVERGENCE-LEDGER C10 row promises 7 commits land on fork main); (2) matches the executor prompt's literal "7 cherry-picks" requirement; (3) avoids a discretionary deviation from the plan's stated commit order.

This is documented in the commit body (`4cfc9036`) and is auditable via `git log --format='%B'` plan-close smoke.

## Threat Flags

No new threat surface beyond the plan's threat model — all four `<threat_model>` STRIDE rows were properly mitigated:

- **T-34-07-01** (Windows-only files invariant): Per-commit `git diff --stat ... | grep -E '_windows|exec_strategy_windows'` returned 0 across all 7 commits — see D-34-E1 audit table above.
- **T-34-07-02** (D-19 trailer): All 7 commits carry the verbatim 6-line trailer; lowercase 'a' in `Upstream-author:` verified.
- **T-34-07-03** (env:// duplicate parser): Verified `profile/mod.rs` delegates to `nono::keystore::is_env_uri` + `validate_env_uri` — see env:// composition invariant section above. No duplicate parser introduced.
- **T-34-07-04** (ps dynamic-column terminal-control char crash): Inherited Plan 34-05's `truncate_chars` utility applied to NAME/PROFILE/COMMAND columns (multibyte-safe per `command_display::truncate_chars`).
- **T-34-07-05** (ps session-internal disclosure): Accepted per plan — existing fork ps display surface filters secrets at `session::list_sessions` level; new dynamic-column logic operates on the same filtered data.

## Push

`origin/main` advance verified post-commit (see "Push Result" below).

## Self-Check: PASSED

All claimed artifacts verified:

```
FOUND: .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-07-PS-ENVURI-PLAN.md
FOUND: .planning/phases/34-upst3-upstream-v0-41-v0-52-sync-execution/34-07-PS-ENVURI-SUMMARY.md
FOUND commits: 75cbb293 adf35ff2 4cfc9036 8394f07b 1d83181d 17e9afcd 108a2358
FOUND baseline: 7406eef2 (Plan 34-05 close, parent of cherry-pick chain)
```
