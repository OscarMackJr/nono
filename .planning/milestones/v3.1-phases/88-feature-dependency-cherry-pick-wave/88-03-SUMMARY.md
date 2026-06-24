---
phase: 88-feature-dependency-cherry-pick-wave
plan: "03"
subsystem: nono-proxy / nono-cli
tags: [cherry-pick, upstream-sync, aws-auth, pack-dir, session-hooks, feat]
dependency_graph:
  requires: [88-02]
  provides: [FEAT-03-aws-auth, FEAT-05-pack-dir]
  affects: [88-05-env-clear-removal]
tech_stack:
  added: []
  patterns:
    - "AwsAuthConfig struct on RouteConfig (fork-adapted partial cherry-pick with D-15 501 stub)"
    - "source_pack provenance tracking on SessionHook via resolve_store_pack_session_hooks()"
key_files:
  created: []
  modified:
    - crates/nono-proxy/src/config.rs
    - crates/nono-proxy/src/credential.rs
    - crates/nono-proxy/src/reverse.rs
    - crates/nono-proxy/src/route.rs
    - crates/nono-proxy/src/server.rs
    - crates/nono-cli/src/network_policy.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/profile_runtime.rs
    - crates/nono-cli/src/hook_runtime.rs
    - crates/nono-cli/src/hook_runtime_windows.rs
    - crates/nono-cli/src/policy.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - .planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md
decisions:
  - "D-15 applied: 501 stub in reverse.rs (non-TLS path) rather than tls_intercept/handle.rs (absent in fork) — equivalent guard on the reachable proxy path"
  - "D-13 satisfied: 7d274cf7 committed in Plan 88-03 before e54cb env_clear removal in Plan 88-05; env_clear() still present at hook_runtime.rs:196"
  - "D-14 verified: hook_runtime_windows.rs env_clear() + CLR baseline restore untouched"
  - "Manual implementation chosen over cherry-pick --no-commit for both commits due to fork structural divergence (no tls_intercept module; expanded profile/mod.rs with aws_auth already added)"
metrics:
  duration: "~3.5 hours"
  completed: "2026-06-20"
  tasks_completed: 3
  files_changed: 13
---

# Phase 88 Plan 03: FEAT-03 (AWS Auth) + FEAT-05 ($PACK_DIR Hooks) Summary

Partial cherry-pick of upstream `5bb098cd` (AWS SigV4 auth config, FEAT-03) with fork-specific 501 stub adaptation per D-15, plus cherry-pick of `7d274cf7` (PACK_DIR session hooks, FEAT-05) honoring D-13 I-before-M ordering constraint.

## What Was Built

**FEAT-03 — AWS auth config (partial cherry-pick of 5bb098cd, D-15 fork adaptation):**

1. `AwsAuthConfig` struct added to `nono-proxy/src/config.rs` with optional `profile`, `region`, `service` fields. `RouteConfig` gains `aws_auth: Option<AwsAuthConfig>` with `serde(default)`.
2. `CredentialStore` in `credential.rs` gains `aws_routes: HashMap<String, ()>` populated at `load()` time, with `get_aws()` accessor and updated `is_empty()`/`len()`/`loaded_prefixes()`.
3. Fork-specific 501 stub in `reverse.rs`: `get_aws(&service).is_some()` returns HTTP 501 before any credential forwarding (D-15: upstream's guard lives in `tls_intercept/handle.rs` which the fork does not have).
4. `CustomCredentialDef` in `profile/mod.rs` gains `aws_auth: Option<AwsAuthConfig>` with mutual-exclusion validation against `credential_key` and `auth`. New `validate_aws_auth()` validates profile/region/service format.
5. `network_policy.rs` wires `aws_auth: cred.aws_auth.clone()` through `RouteConfig` construction.
6. `nono-profile.schema.json` adds `AwsAuthConfig` `$def` and `aws_auth` field in `CustomCredentialDef`.

**FEAT-05 — $PACK_DIR session hooks (cherry-pick of 7d274cf7):**

1. `SessionHook` in `profile/mod.rs` gains `#[serde(skip)] pub(crate) source_pack: Option<PackageRef>` for pack provenance tracking.
2. New `resolve_store_pack_session_hooks()` function stamps provenance and expands `$PACK_DIR` prefix to the pack's install directory. Called from 3 load paths: `load_profile_inner`, `load_registry_profile`, `load_base_profile_raw`.
3. `verify_profile_packs()` in `profile_runtime.rs` upgraded with profile parameter, source_pack consistency guard (hook not in packs list → hard error), and session hook containment check (hook script must be a declared artifact in the lockfile).
4. All `SessionHook` struct literals updated with `source_pack: None` across `hook_runtime.rs`, `hook_runtime_windows.rs`, `profile/mod.rs`, `policy.rs`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Direct cherry-pick failed on both commits**
- **Found during:** Task 1, Task 2
- **Issue:** `git cherry-pick -x --no-commit 5bb098cd` failed with conflicts on all shared files (fork's credential.rs lacks `oauth2_routes`; profile/mod.rs already had `aws_auth` from Task 1; hook_runtime.rs has fork-specific divergence tests). `7d274cf7` similarly conflicted on all 3 files.
- **Fix:** Switched to manual implementation for both commits: read the upstream diff, identified each structural difference, applied changes file-by-file.
- **Files modified:** All plan-declared files
- **Commit:** 5eab6d46 (FEAT-03), c0ea3af7 (FEAT-05)

**2. [Rule 2 - Missing critical functionality] Missed SessionHook literals in hook_runtime_windows.rs and policy.rs**
- **Found during:** Task 2 compilation check (cargo test)
- **Issue:** struct literal errors for `source_pack` missing in `hook_runtime_windows.rs:1100`, `hook_runtime_windows.rs:1184`, and `policy.rs:3282`.
- **Fix:** Added `source_pack: None` to all 3 literals.
- **Files modified:** `crates/nono-cli/src/hook_runtime_windows.rs`, `crates/nono-cli/src/policy.rs`
- **Commit:** c0ea3af7

**3. [Rule 2 - Missing] fmt check failure**
- **Found during:** Task 3 `cargo fmt --all -- --check`
- **Issue:** `profile_runtime.rs` `ok_or_else` closure formatting didn't match rustfmt style.
- **Fix:** `cargo fmt --all` applied.
- **Commit:** 58baf02d

## D-Constraint Verification

| Constraint | Status | Evidence |
|-----------|--------|---------|
| D-13: 7d274cf7 before e54cb | PASS | env_clear() still at hook_runtime.rs:196 after Plan 88-03 |
| D-14: Windows file untouched | PASS | hook_runtime_windows.rs env_clear() + CLR baseline intact |
| D-15: tls_intercept hunk skipped | PASS | No tls_intercept/ directory in fork; 501 stub in reverse.rs instead |

## CI Gate Results (Windows Host)

- **clippy:** PASS (0 warnings, 0 errors)
- **fmt:** PASS
- **cargo test -p nono-proxy:** 170 passed, 0 failed
- **cargo test -p nono-cli:** 1350 passed, 4 failed (all 4 are pre-existing Windows baseline failures per `nono_cli_windows_baseline_test_failures` memory note)
- **PARTIAL→CI:** hook_runtime.rs has `#[cfg(unix)]` blocks; profile/mod.rs and profile_runtime.rs have cross-platform pack-store path handling — deferred to GH Actions Linux/macOS lanes (see 88-PARTIAL-CI.md Plan 88-03 rows)

## Known Stubs

The 501 stub in `reverse.rs` is intentional (documented in D-15, FEAT-03 design): AWS SigV4 signing implementation is not present in the fork. The stub ensures requests to AWS-auth routes fail explicitly (501 Not Implemented) rather than silently passing unauthenticated. FEAT-03 wires the schema and validation; the actual signing engine is an upstream-only feature deferred per D-15.

## Self-Check: PASSED

Files exist:
- `crates/nono-proxy/src/config.rs` — AwsAuthConfig struct present
- `crates/nono-proxy/src/credential.rs` — aws_routes field present
- `crates/nono-proxy/src/reverse.rs` — 501 stub present at line 189
- `crates/nono-cli/src/profile/mod.rs` — source_pack field + resolve_store_pack_session_hooks() present
- `crates/nono-cli/src/profile_runtime.rs` — verify_profile_packs profile param present
- `.planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md` — Plan 88-03 rows present

Commits exist:
- `5eab6d46` — feat(88-03): add AwsAuthConfig + 501 stub on non-TLS path (FEAT-03)
- `c0ea3af7` — feat(88-03): add PACK_DIR session hooks + source_pack provenance (FEAT-05)
- `58baf02d` — chore(88-03): cargo fmt + update PARTIAL-CI gate record
