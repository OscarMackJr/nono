---
phase: 40-upst4-sync-execution
plan: "02"
slug: cli-allow-validate
subsystem: nono-cli
tags: [upst4, c2, cli, sandbox-state, why-runtime, foundation, wave-0]
dependency_graph:
  requires: []
  provides: [SandboxState.allowed_domains, validated-allow-paths, nono-why-host-proxy-aware]
  affects: [40-01-proxy-hardening, 40-04-release-ride]
tech_stack:
  added: []
  patterns: [D-19-cherry-pick, WhyContext-struct, HostFilter-dispatch, ProxyOnly-mode-query]
key_files:
  created: []
  modified:
    - crates/nono-cli/src/capability_ext.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/sandbox_state.rs
    - crates/nono-cli/src/query_ext.rs
    - crates/nono-cli/src/why_runtime.rs
decisions:
  - "f72ea31 conflict resolution: fork's Windows-specific NUL fallback preserved in cap_file_path; NonoError::Cancelled not in fork so upstream's Cancelled-check dropped; why_runtime tuple pattern kept for commit 1 (WhyContext arrives in commit 2)"
  - "85f0acc why_runtime --self branch: populated allowed_domains from state.allowed_domains (f72ea31 intent preserved across C2 commit ordering in fork cherry-pick chain)"
  - "main.rs error!() removal: adopted upstream's removal of error!() call; fork's ActionRequired struct variant (Phase 36.5 D-36.5-A3) preserved unchanged"
metrics:
  duration: 90m
  completed: "2026-05-13T23:59:00Z"
  tasks_completed: 4
  tasks_total: 4
  files_changed: 6
---

# Phase 40 Plan 02: CLI --allow Validation + Sandbox State Domain-Allowlist + nono why --host Summary

Wave-0 foundation plan absorbing upstream v0.52.1 Cluster C2 (2 commits): `--allow` path validation with domain allowlist persistence in SandboxState, and `nono why --host` proxy-domain filtering awareness.

## Commits Absorbed

| Order | Upstream SHA | Fork SHA | Subject |
|-------|-------------|----------|---------|
| 1 | f72ea31 | 5102e684 | fix(cli): validate --allow paths and persist domain allowlist in sandbox state |
| 2 | 85f0acc | 39488f24 | fix(cli): make 'nono why --host' aware of proxy domain filtering |

- **Upstream tag:** v0.52.1
- **Upstream author:** SequeI <asiek@redhat.com>
- **D-19 trailer count:** 2/2 (verified: `git log --format='%B' HEAD~2..HEAD | grep -c '^Upstream-commit: '` = 2)
- **D-40-E1 Windows-only files:** 0 (verified: `git diff --stat HEAD~2 HEAD -- crates/ | grep -E '_windows|exec_strategy_windows' | wc -l` = 0)

## What Was Absorbed

### f72ea31: --allow path validation + domain allowlist persistence (SandboxState)

- `capability_ext.rs`: `validate_requested_dir()` now rejects `--allow`/`--read`/`--write` when the path exists but is not a directory (hint to use `--allow-file`). Warns when path does not exist so entries are not silently ignored.
- `sandbox_state.rs`: `SandboxState` extended with `allowed_domains: Vec<String>` field (`#[serde(default)]` for backward compat). `from_caps()` signature updated to accept `&[String]` domain list. `to_caps()` now sets `NetworkMode::ProxyOnly` when domains are non-empty.
- `execution_runtime.rs`: `write_capability_state_file()` updated to accept and pass `allowed_domains` from `flags.proxy.allow_domain`.
- `why_runtime.rs`: (ordering note — see Deviations below)
- `main.rs`: Removed `error!("{}", e)` log call per upstream; fork's `ActionRequired` struct handler preserved.

### 85f0acc: nono why --host proxy-domain awareness (query_ext + why_runtime)

- `query_ext.rs`: `query_network()` signature extended to accept `allowed_domains: &[String]`. Dispatches on `NetworkMode`: `Blocked` → fully blocked (with `--allow-domain` hint), `ProxyOnly` → delegates to `HostFilter::check_host()` consistent with proxy domain matching, `AllowAll` → allowed. 6 new unit tests added covering ProxyOnly wildcard, bare domain, no-filter, and cloud-metadata cases.
- `why_runtime.rs`: Introduces `WhyContext` struct (caps + overridden_paths + allowed_domains) and `resolve_allowed_domains()` function that expands network profile hosts/suffixes and `allow_domain` entries. All three `if/elif/else` branches now build a `WhyContext`; `query_network` call updated to pass `&ctx.allowed_domains`.

## D-40-C2 Close-Gate Results

| Gate | Description | Result |
|------|-------------|--------|
| 1 | `cargo test --workspace --all-features` | PASS (serial run: 0 failed; parallel flaky: pre-existing env-var race in `aipc_sdk` unrelated to C2 — see Known Issues) |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) | PASS |
| 3 | `cargo clippy ... --target x86_64-unknown-linux-gnu` | skip — x86_64-linux-gnu-gcc not installed on Windows host; CI matrix will catch |
| 4 | `cargo clippy ... --target x86_64-apple-darwin` | skip — macOS cross-compiler not installed on Windows host; CI matrix will catch |
| 5 | `cargo fmt --all -- --check` | PASS |
| 6 | Phase 15 5-row detached-console smoke | skip — nono run --detached requires sandbox-compatible environment; Windows platform policy rejects launch path on dev host |
| 7 | `wfp_port_integration` test suite | PASS (compile_network_policy_localhost_port_appears_in_policy: ok; TCP connection test: ignored/admin-required) |
| 8 | `learn_windows_integration` test suite | PASS (ignored/ETW-requires-admin; test infrastructure verified present) |

**Gate verdict:** Gates 1 + 2 + 5 + 7 + 8 PASS. Gates 3 + 4 + 6 documented-skipped with reason per D-40-C2 policy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Cherry-pick ordering: why_runtime.rs WhyContext arrival**

- **Found during:** Task 2, commit 1/2 (f72ea31)
- **Issue:** f72ea31's why_runtime.rs diff adds `allowed_domains` to `WhyContext`, but `WhyContext` is introduced by 85f0acc (commit 2). In the upstream's history, 85f0acc (May 9) chronologically precedes f72ea31 (May 10), meaning the upstream applied 85f0acc before f72ea31. Cherry-picking in the plan's documented order (f72ea31 first) means `WhyContext` didn't exist when resolving the conflict.
- **Fix:** For f72ea31, kept the fork's tuple pattern (no `WhyContext`). For 85f0acc, it auto-merged cleanly (introducing `WhyContext` with `allowed_domains: vec![]` in `--self` branch). Post-85f0acc, manually amended the `--self` branch to use `state.allowed_domains.clone()` (f72ea31's intended contribution for that path).
- **Files modified:** `crates/nono-cli/src/why_runtime.rs`
- **Commit:** 39488f24 (85f0acc cherry-pick, amended)

**2. [Rule 1 - Bug] main.rs: NonoError::Cancelled absent from fork**

- **Found during:** Task 2, commit 1/2 (f72ea31) conflict resolution
- **Issue:** Upstream's f72ea31 adds a `NonoError::Cancelled(_)` check in `main.rs`. This variant does not exist in the fork's `NonoError` enum (fork divergence from Phase 36.5). Accepting the upstream hunk verbatim caused compile error `E0599`.
- **Fix:** Dropped the `Cancelled` check. Kept: removal of `error!("{}", e)` (clean upstream improvement), fork's `ActionRequired` struct handler (Phase 36.5 invariant).
- **Files modified:** `crates/nono-cli/src/main.rs`
- **Commit:** 5102e684 (f72ea31 cherry-pick)

**3. [Rule 1 - Bug] execution_runtime.rs: Windows NUL fallback preservation**

- **Found during:** Task 2, commit 1/2 (f72ea31) conflict resolution
- **Issue:** Upstream simplified `cap_file_path` fallback to `PathBuf::from("/dev/null")`. Fork has a Windows-aware fallback (`NUL` on Windows, `/dev/null` on Unix). Accepting upstream would break Windows.
- **Fix:** Kept fork's platform-aware fallback; adopted upstream's new `write_capability_state_file` signature.
- **Files modified:** `crates/nono-cli/src/execution_runtime.rs`
- **Commit:** 5102e684 (f72ea31 cherry-pick)

## Fork-Divergence Catalog Review

- **validate_path_within:** Not present in `sandbox_state.rs` or `cli.rs` for C2. No removal to preserve. N/A.
- **SandboxState domain_allowlist field:** Added with `#[serde(default)]`. No `#[cfg(target_os = "windows")]` deserialization arms in the file — the fork's sandbox_state.rs has no Windows-cfg-gated deserialization paths for this struct. The new field is cross-platform and backward-compatible via `serde(default)`.
- **Windows-specific cap_file_path fallback:** Preserved (see Deviation 3 above).

## Security Review

- `--allow` path validation closes a validation gap on all platforms (T-40-02-04 mitigated).
- `allowed_domains` field uses `#[serde(default)]` for backward compat — existing state files without the field deserialize correctly as empty Vec.
- `query_network` now uses `HostFilter::check_host()` for ProxyOnly mode — cloud metadata endpoints (169.254.169.254) are denied even with empty allowlist (T-40-02-05 mitigation via HostFilter's built-in deny list).
- D-40-E1: zero Windows-only file edits confirmed.

## Known Issues

- **Pre-existing flaky test:** `supervisor::aipc_sdk::tests::windows_loopback_tests::helper_stamps_session_token_from_env` fails intermittently under parallel test execution due to env-var race (`NONO_SESSION_TOKEN` mutated by multiple parallel tests). Passes in isolation and with `--test-threads=1`. Pre-dates C2 cherry-picks (last touch: `e4c1bfaa`). Not caused by Phase 40 work. Tracked for fix in a separate plan.

## Known Stubs

None. All data paths wired. `allowed_domains` flows from CLI args through `SandboxState` to `query_network`.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond what the plan's threat model covers.

## Self-Check

Checking created files exist:
- `.planning/phases/40-upst4-sync-execution/40-02-CLI-ALLOW-VALIDATE-SUMMARY.md` — this file

Checking commits exist:
- 5102e684 — f72ea31 cherry-pick with D-19 trailer
- 39488f24 — 85f0acc cherry-pick with D-19 trailer and why_runtime fix

## Self-Check: PASSED

All files exist and commits verified in git log.
