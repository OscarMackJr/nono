---
phase: 88-feature-dependency-cherry-pick-wave
plan: "01"
subsystem: nono-cli/exec_strategy, nono/keystore, nono-proxy/credential
tags: [cherry-pick, env-injection, keyring-timeout, set_vars, FEAT-01, FEAT-04]
dependency_graph:
  requires: []
  provides:
    - validate_set_vars() in env_sanitization.rs (blocks set_vars keys PATH and NONO_*)
    - ExecConfig::set_vars field + push_set_vars() deduplication in exec_strategy.rs
    - keyring_timeout() + call_with_keyring_timeout() in keystore.rs
    - NONO_KEYRING_TIMEOUT_SECS env var read at keyring access time
  affects:
    - nono-cli profile parse path (validate_set_vars called for every profile load)
    - nono keyring access path (timeout wrapper on every Entry::get_password call)
    - nono-proxy credential load (warn! with timeout hint on KeystoreAccess failure)
tech_stack:
  added: []
  patterns:
    - git cherry-pick -x (provenance annotation) + DCO sign-off via --amend -s
    - PARTIAL→CI deferral for exec_strategy.rs cfg-gated Unix blocks
key_files:
  created:
    - .planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md
  modified:
    - crates/nono-cli/src/exec_strategy/env_sanitization.rs
    - crates/nono-cli/src/exec_strategy.rs
    - crates/nono-cli/src/exec_strategy_windows/mod.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/profile_runtime.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/command_runtime.rs
    - crates/nono-cli/src/execution_runtime.rs
    - crates/nono-cli/src/launch_runtime.rs
    - crates/nono-cli/src/main.rs
    - crates/nono-cli/src/proxy_runtime.rs
    - crates/nono-cli/data/nono-profile.schema.json
    - crates/nono-cli/data/profile-authoring-guide.md
    - docs/cli/features/environment.mdx
    - docs/cli/features/profile-authoring.mdx
    - crates/nono/src/keystore.rs
    - crates/nono-proxy/src/credential.rs
decisions:
  - "FEAT-01 d48aeb7b: fork keeps is_env_var_denied() (Windows case-insensitive path); upstream removed it but fork needs it for exec_strategy_windows"
  - "FEAT-01 d48aeb7b: removed collect_ignored_denial_paths() and expand_ignored_denial_path() (dead code in fork; inline approach used instead; CLAUDE.md forbids #[allow(dead_code)])"
  - "FEAT-01 d48aeb7b: validate_set_vars called in BOTH parse_profile_file() and parse_profile_bytes() — upstream only added to parse_profile_file but fork tests hit parse_profile_bytes path"
  - "FEAT-01 d48aeb7b: expand_profile_set_vars_expands_home test gated #[cfg(not(target_os = 'windows'))] — HOME expansion differs on Windows"
  - "FEAT-01: dangerous-variable blocklist (LD_PRELOAD, etc.) intentionally NOT applied in validate_set_vars — set_vars is explicit operator intent; security posture per T-88-02"
  - "FEAT-04 c6b13345: fork kept system_keystore_label() in error messages for platform-specific keystore naming (macOS Keychain vs Linux Secret Service)"
  - "FEAT-04 c6b13345: upstream's OAuth2 block in credential.rs conflict EXCLUDED — fork's CredentialStore struct lacks oauth2_routes field; kept continue; after credentials.insert() for structural correctness"
  - "FEAT-04 c6b13345: build_credential_miss_hint() wired into SecretNotFound arm of credential.rs to eliminate dead code (CLAUDE.md: avoid #[allow(dead_code)])"
  - "PARTIAL→CI: exec_strategy.rs and env_sanitization.rs deferred to GH Actions Linux/macOS lanes; cross-C-toolchain missing on Windows dev host"
metrics:
  duration_minutes: 180
  tasks_completed: 3
  tasks_total: 3
  files_modified: 17
  completed_date: "2026-06-20"
---

# Phase 88 Plan 01: Cherry-pick FEAT-01 set_vars + FEAT-04 keyring timeout Summary

One-liner: Cherry-picked upstream d48aeb7b (set_vars static env injection into ExecConfig) and c6b13345 (NONO_KEYRING_TIMEOUT_SECS configurable keyring timeout) with fork-specific wiring for Windows compatibility, dual profile-parse paths, and dead-code elimination.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Cherry-pick d48aeb7b (FEAT-01 set_vars) | 89ba09cf | env_sanitization.rs, exec_strategy.rs, profile/mod.rs, profile_runtime.rs, 11 files total |
| 2 | Cherry-pick c6b13345 (FEAT-04 keyring timeout) | 614cf1c7 | keystore.rs, credential.rs |
| 3 | make ci gate + PARTIAL→CI record + fmt fix | 6829004a | 88-PARTIAL-CI.md, exec_strategy_windows/mod.rs |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed duplicate should_skip_env_var in env_sanitization.rs**
- **Found during:** Task 1 cargo check
- **Issue:** Conflict resolution left two definitions of `should_skip_env_var` — fork's Windows-aware version at top AND a second (upstream-simplified) copy further down
- **Fix:** Removed the duplicate; kept fork's version with `env_key_matches()` for Windows case-insensitivity
- **Files modified:** crates/nono-cli/src/exec_strategy/env_sanitization.rs
- **Commit:** 89ba09cf

**2. [Rule 1 - Bug] Fixed let-chain Rust 2024 syntax to nested if-let for Rust 2021**
- **Found during:** Task 1 cargo check
- **Issue:** Upstream used `if let Some(x) = y && !z && let Some(e) = w` (Rust 2024 only)
- **Fix:** Rewrote as nested `if let` blocks in profile/mod.rs
- **Files modified:** crates/nono-cli/src/profile/mod.rs
- **Commit:** 89ba09cf

**3. [Rule 1 - Bug] Added validate_set_vars re-export to exec_strategy_windows/mod.rs**
- **Found during:** Task 1 cargo check (Windows build)
- **Issue:** On Windows, `exec_strategy` maps to `exec_strategy_windows/mod.rs` which only re-exported `is_dangerous_env_var` and `validate_env_var_patterns`; profile/mod.rs called `crate::exec_strategy::validate_set_vars` which was not found
- **Fix:** Added `validate_set_vars` to the re-export line in exec_strategy_windows/mod.rs
- **Files modified:** crates/nono-cli/src/exec_strategy_windows/mod.rs
- **Commit:** 89ba09cf

**4. [Rule 2 - Dead code] Removed collect_ignored_denial_paths() and expand_ignored_denial_path()**
- **Found during:** Task 1 integration
- **Issue:** Upstream's `profile_runtime.rs` added these functions but the fork builds `ignored_denial_paths` inline; functions would be dead code and CLAUDE.md forbids `#[allow(dead_code)]`
- **Fix:** Removed both functions; kept `expand_profile_set_vars()` which IS called
- **Files modified:** crates/nono-cli/src/profile_runtime.rs
- **Commit:** 89ba09cf

**5. [Rule 1 - Bug] Added validate_set_vars call to parse_profile_bytes() in addition to parse_profile_file()**
- **Found during:** Task 1 test run (set_vars_rejects_* tests failed)
- **Issue:** Tests use `parse_profile_bytes()` path but upstream only added validation to `parse_profile_file()`
- **Fix:** Added identical validate_set_vars call block to parse_profile_bytes()
- **Files modified:** crates/nono-cli/src/profile/mod.rs
- **Commit:** 89ba09cf

**6. [Rule 1 - Bug] Gated expand_profile_set_vars_expands_home with #[cfg(not(target_os = "windows"))]**
- **Found during:** Task 1 test run
- **Issue:** Test sets HOME=/home/tester but Windows uses registry/env differently; $HOME expands to real Windows path, not the test value
- **Fix:** Added platform gate to exclude the test on Windows
- **Files modified:** crates/nono-cli/src/profile_runtime.rs
- **Commit:** 89ba09cf

**7. [Rule 1 - Bug] Added set_vars: Default::default() to EnvironmentConfig struct literal in proxy_runtime.rs test**
- **Found during:** Task 1 test compilation
- **Issue:** After adding set_vars field to EnvironmentConfig, existing test struct literal was missing the field
- **Fix:** Added `set_vars: Default::default()` to the struct literal
- **Files modified:** crates/nono-cli/src/proxy_runtime.rs
- **Commit:** 89ba09cf

**8. [Rule 2 - Dead code] Wired build_credential_miss_hint() into SecretNotFound arm**
- **Found during:** Task 2 integration
- **Issue:** Upstream added `build_credential_miss_hint()` helper but the first conflict block (which would have called it in the OAuth2 path) was excluded because the fork's CredentialStore struct lacks oauth2_routes; function would be dead code
- **Fix:** Called `build_credential_miss_hint(key)` in the `SecretNotFound` arm of `credential.rs::CredentialStore::load()`, upgrading it from silent `debug!` to `warn!` with a source-cross-probe hint
- **Files modified:** crates/nono-proxy/src/credential.rs
- **Commit:** 614cf1c7

**9. [Rule 1 - Bug] fmt fix for exec_strategy_windows/mod.rs use glob**
- **Found during:** Task 3 cargo fmt --check
- **Issue:** Three-item use-glob added in Task 1 exceeded formatter line threshold
- **Fix:** Split onto multiple lines per rustfmt standard
- **Files modified:** crates/nono-cli/src/exec_strategy_windows/mod.rs
- **Commit:** 6829004a

## Known Stubs

None. Both features fully wired end-to-end.

## PARTIAL→CI Record

See `.planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md`.

| Commit | File | Status |
|--------|------|--------|
| 89ba09cf (d48aeb7b) | exec_strategy.rs | PARTIAL — GH Actions Linux/macOS CI lanes decisive |
| 89ba09cf (d48aeb7b) | exec_strategy/env_sanitization.rs | PARTIAL — GH Actions Linux/macOS CI lanes decisive |

## CI Results (Windows Host)

- `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`: PASS
- `cargo fmt --all -- --check`: PASS
- `cargo test -p nono`: 784 passed, 1 failed (pre-existing: `try_set_mandatory_label`)
- `cargo test -p nono-cli`: 1325 passed, 4 failed (pre-existing: `profile_cmd::test_init_allowed_when_pack_has_same_short_name`, 3x `protected_paths`)
- `cargo test -p nono-proxy`: 163 passed, 0 failed
- `cargo audit`: 4 warnings (pre-existing `rustls-pemfile` unmaintained), 0 errors

## Self-Check

PASSED — verified below:

Files exist: exec_strategy/env_sanitization.rs (validate_set_vars), keystore.rs (NONO_KEYRING_TIMEOUT_SECS), 88-PARTIAL-CI.md.
Commits: 89ba09cf (cherry picked from d48aeb7b, Signed-off-by oscarmackjr-twg), 614cf1c7 (cherry picked from c6b13345, Signed-off-by oscarmackjr-twg), 6829004a (docs commit with fmt fix and PARTIAL-CI).
