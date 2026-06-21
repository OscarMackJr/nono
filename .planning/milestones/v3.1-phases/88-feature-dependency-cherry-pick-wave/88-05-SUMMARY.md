---
phase: 88-feature-dependency-cherry-pick-wave
plan: "05"
subsystem: nono-cli
tags: [cherry-pick, upstream-sync, feat, truthy-flags, blocked-grants, env-clear, dry-run, domain-rename]
dependency_graph:
  requires: [88-04]
  provides: [FEAT-06c-truthy-env-flags, cluster-M-misc, env-clear-unix-removal]
  affects: []
tech_stack:
  added: []
  patterns:
    - "BoolishValueParser on trust-proxy-ca, trust-override, capability-elevation flags + NONO_TRUST_OVERRIDE env source (42e5bf73)"
    - "PreparedCaps.blocked_grants Vec collects macOS deny-overlaps for folded capability summary display (a0bba5eb)"
    - "finalize_caps() returns Vec<(PathBuf, Option<String>)> instead of () -- blocked grants bubble to caller (a0bba5eb)"
    - "env_clear removal from Unix hook path only; Windows CLR baseline restore retained (e54cf9cb, D-14)"
key_files:
  created: []
  modified:
    - crates/nono-cli/src/cli.rs
    - crates/nono-cli/src/capability_ext.rs
    - crates/nono-cli/src/output.rs
    - crates/nono-cli/src/sandbox_prepare.rs
    - crates/nono-cli/src/startup_runtime.rs
    - crates/nono-cli/src/profile_runtime.rs
    - crates/nono-cli/src/profile/mod.rs
    - crates/nono-cli/src/hook_runtime.rs
    - crates/nono/schema/capability-manifest.schema.json
    - crates/nono-cli/tests/env_vars.rs
    - .planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md
decisions:
  - "7e076d2d (sigstore removal) skipped as empty: fork already deleted pull_ui.rs entirely in Plan 88-04; sigstore provenance already removed"
  - "9800f307 conflict resolved: fork uses verify_profile_packs(packs, &profile) signature (88-03 added profile param); options.hook_output_silent -> silent parameter"
  - "a0bba5eb conflicts resolved: capability_ext.rs kept allow-gpu wiring (fork addition) + used new blocked_grants return; output.rs test imports merged (normalize_terminal_line_endings kept, render_terminal_block_for_tty absent in fork)"
  - "42e5bf73 conflict resolved: fork lacked trust_proxy_ca/proxy_ca_validity fields; accepted upstream additions; added proxy_ca_validity: None and #[cfg(target_os = macos)] trust_proxy_ca: false to WrapSandboxArgs->SandboxArgs conversion"
  - "D-14 invariant confirmed: hook_runtime.rs env_clear count=0, hook_runtime_windows.rs env_clear count=7"
metrics:
  duration: "~1.5 hours"
  completed: "2026-06-20"
  tasks_completed: 4
  files_changed: 11
---

# Phase 88 Plan 05: Cluster M Misc Fixes + FEAT-06c (env_clear Removal) Summary

Completed the Cluster M cherry-pick sequence: FEAT-06c truthy env bool flags (42e5bf73), macOS blocked grants display (a0bba5eb), schema domain rename nono.dev->nono.sh (ee7a3bda), sigstore provenance removal (7e076d2d), pack-verification dry-run skip (9800f307), and env_clear removal on Unix path only (e54cf9cb, D-14).

## What Was Built

**FEAT-06c (42e5bf73) — Truthy env bool flags:**

1. `BoolishValueParser` added to `--trust-proxy-ca` (macOS), `--trust-override`, and `--capability-elevation` flags in `cli.rs`. Values like "yes", "1", "true" are now accepted.
2. `NONO_TRUST_OVERRIDE` wired as env source on `--trust-override`.
3. `trust_proxy_ca: bool` and `proxy_ca_validity: Option<u32>` fields added to `SandboxArgs` (these were absent from fork; upstream conversion required accepting upstream's additions).
4. New test file `tests/env_vars.rs` gains `env_nono_capability_elevation_accepts_truthy`, `env_nono_trust_override_accepts_truthy`, and `env_nono_trust_proxy_ca_accepts_truthy` tests.

**a0bba5eb — macOS blocked grants display:**

1. `PreparedCaps` struct gains `blocked_grants: Vec<(PathBuf, Option<String>)>` field.
2. `finalize_caps()` in `capability_ext.rs` returns `Result<Vec<(PathBuf, Option<String>)>>` instead of `Result<()>`. Collects macOS-denied-user-grants instead of emitting individual `warn!` calls.
3. `print_capabilities()` in `output.rs` gains `blocked_grants` parameter; renders them via new `print_blocked_grants()` helper (collapsed by default, expanded with `-v`).
4. `sandbox_prepare.rs` plumbs `blocked_grants` from `PreparedCaps` through to `finalize_prepared_sandbox()`.
5. `startup_runtime.rs` updated to recognize "deny ... kept blocked" capability summary lines.

**ee7a3bda — Schema domain rename:**

1. `crates/nono-cli/src/profile/mod.rs` — schema URL updated: `nono.dev` -> `nono.sh`.
2. `crates/nono-cli/src/profile_cmd.rs` — same domain rename.
3. `crates/nono/schema/capability-manifest.schema.json` — `$schema` URL updated.

**7e076d2d — Sigstore provenance removal:**

Skipped (empty apply): fork deleted `pull_ui.rs` entirely in Plan 88-04. Sigstore provenance display is already absent.

**9800f307 — Pack-verification dry-run skip:**

`profile_runtime.rs` updated: when `args.dry_run` is true, pack verification is skipped with an informational message. Non-dry-run runs still hit `verify_profile_packs()`. Integration test script `tests/integration/test_pack_resolution.sh` was already deleted in the fork — removed from cherry-pick.

**e54cf9cb — env_clear removal (Unix path only, D-14):**

`cmd.env_clear()` removed from `build_hook_command()` in `hook_runtime.rs` (Unix hook path). `hook_runtime_windows.rs` unchanged — retains `cmd.env_clear()` at line 301 and `SystemRoot`/`windir`/`SystemDrive` CLR baseline restore at line 326. This satisfies D-14 and `windows_hook_interpreter_spawn_gotchas`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 42e5bf73 conflict: fork lacked trust_proxy_ca/proxy_ca_validity fields**
- **Found during:** Task 1 cherry-pick
- **Issue:** Upstream diff modifies existing `trust_proxy_ca` field with BoolishValueParser; fork never had this field. Conflict on SandboxArgs.
- **Fix:** Accepted upstream's addition of `trust_proxy_ca: bool` (macOS-gated) and `proxy_ca_validity: Option<u32>`. Added `#[cfg(target_os = "macos")] trust_proxy_ca: false` and `proxy_ca_validity: None` to the `WrapSandboxArgs -> SandboxArgs` impl.
- **Files modified:** `crates/nono-cli/src/cli.rs`
- **Commit:** 2a86abdf

**2. [Rule 3 - Blocking] a0bba5eb conflict in capability_ext.rs: allow_gpu wiring (fork addition) plus blocked_grants return**
- **Found during:** Task 2 cherry-pick
- **Issue:** Fork has allow_gpu wiring before `finalize_caps()` call that was absent in upstream's diff context. Conflict on the call site.
- **Fix:** Kept fork's allow_gpu wiring AND applied upstream's `let blocked_grants = finalize_caps(...)` return pattern.
- **Files modified:** `crates/nono-cli/src/capability_ext.rs`
- **Commit:** 8c081fe8

**3. [Rule 3 - Blocking] a0bba5eb conflict in output.rs: test imports and unix_socket tests**
- **Found during:** Task 2 cherry-pick
- **Issue:** (a) Fork's test module had different imports from upstream. (b) Unix socket tests were previously commented out with a "not ported" note but are now active. (c) Missing `CapabilitySet` and `UnixSocketMode` imports.
- **Fix:** Merged imports; accepted upstream's unix socket tests + print_blocked_grants test; added `CapabilitySet` import (separate fix commit after clippy failure).
- **Files modified:** `crates/nono-cli/src/output.rs`
- **Commits:** 8c081fe8, 8a63334a

**4. [Rule 3 - Blocking] 7e076d2d: modify/delete conflict on pull_ui.rs**
- **Found during:** Task 2 cherry-pick
- **Issue:** Fork deleted pull_ui.rs entirely in Plan 88-04. Upstream modifies it. Cherry-pick leaves the file's upstream version in the tree.
- **Fix:** `git rm pull_ui.rs`; cherry-pick became empty (expected); used `git cherry-pick --skip`. Sigstore display already absent from fork.
- **Files modified:** none

**5. [Rule 3 - Blocking] 9800f307 conflict: verify_profile_packs signature divergence + integration test deletion**
- **Found during:** Task 2 cherry-pick
- **Issue:** Fork's `verify_profile_packs()` takes `(packs, &profile)` (added in Plan 88-03); upstream version takes `(packs)` only. Also `tests/integration/test_pack_resolution.sh` was a modify/delete conflict.
- **Fix:** Resolved conflict by keeping fork's profile parameter; replaced `options.hook_output_silent` with `silent`. Removed integration test with `git rm`.
- **Files modified:** `crates/nono-cli/src/profile_runtime.rs`
- **Commit:** af7c7e60

## D-Constraint Verification

| Constraint | Status | Evidence |
|-----------|--------|---------|
| D-12: cherry-pick -x + DCO | PASS | All applied commits have (cherry picked from ...) + Signed-off-by |
| D-13: 7d274cf7 before e54cb | PASS | 7d274cf7 committed in Plan 88-03; e54cb applied last in this plan |
| D-14: Windows file untouched | PASS | hook_runtime.rs env_clear count=0; hook_runtime_windows.rs env_clear count=7; SystemRoot/windir/SystemDrive at line 326 |

## CI Gate Results (Windows Host)

- **clippy:** PASS (0 warnings, 0 errors after CapabilitySet import fix)
- **fmt:** PASS
- **cargo test -p nono:** 785 passed, 1 failed (pre-existing: `try_set_mandatory_label_surfaces_directive_when_user_owned_apply_fails`)
- **cargo test -p nono-cli:** 1346 passed, 4 failed (all 4 are pre-existing Windows baseline failures per `nono_cli_windows_baseline_test_failures` memory note)
- **PARTIAL->CI:** hook_runtime.rs has `#[cfg(unix)]` blocks; env_clear removal on Unix path unverifiable on Windows host — deferred to GH Actions Linux/macOS lanes (see 88-PARTIAL-CI.md Plan 88-05 row)

## Task Commits

1. **Task 1: Cherry-pick 42e5bf73 (FEAT-06c truthy env bool flags)** - `2a86abdf`
2. **Task 2a: Cherry-pick a0bba5eb (macOS blocked grants display)** - `8c081fe8`
3. **Task 2b: Cherry-pick ee7a3bda (schema domain rename)** - `b7c31126`
4. **Task 2c: Cherry-pick 7e076d2d (sigstore provenance)** - SKIPPED (empty; pull_ui.rs already deleted)
5. **Task 2d: Cherry-pick 9800f307 (dry-run pack verification skip)** - `af7c7e60`
6. **Task 3: Cherry-pick e54cf9cb (env_clear Unix-path only)** - `76e1e40d`
7. **Task 3b: PARTIAL-CI.md update** - `f0ec2540`
8. **Task 4: CapabilitySet import fix (clippy)** - `8a63334a`

## Known Stubs

None.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| T-88-18 | crates/nono-cli/src/hook_runtime.rs | Unix hook subprocess now inherits parent env (env_clear removed per e54cf9cb); intentional per upstream design (hooks are trusted operator-authored scripts) |

## Self-Check: PASSED

Files exist:
- `crates/nono-cli/src/cli.rs` — BoolishValueParser on trust flags; NONO_TRUST_OVERRIDE env source present
- `crates/nono-cli/src/hook_runtime.rs` — env_clear count = 0
- `crates/nono-cli/src/hook_runtime_windows.rs` — env_clear count = 7 (retained)
- `crates/nono-cli/src/profile_runtime.rs` — dry_run check present
- `crates/nono-cli/src/profile/mod.rs` — nono.sh domain present
- `.planning/phases/88-feature-dependency-cherry-pick-wave/88-PARTIAL-CI.md` — Plan 88-05 entry present

Commits exist:
- `2a86abdf` — fix(cli): accept truthy env values for bool flags (#1136)
- `8c081fe8` — feat(output): show blocked macos grants in capability summary (#1178)
- `b7c31126` — fix: replace stale nono.dev schema domains with nono.sh
- `af7c7e60` — feat(pack-verification): skip pack verification on dry runs
- `76e1e40d` — fix(hooks): remove env_clear from Unix hook path (e54cf9cb)
- `f0ec2540` — docs(88-05): add Plan 88-05 PARTIAL->CI deferral entry
- `8a63334a` — fix(88-05): add CapabilitySet import to output.rs test module
