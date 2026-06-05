---
phase: 55-upst7-cherry-pick-wave
verified: 2026-06-05T02:21:49Z
status: human_needed
score: 9/9 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Live CI confirms cross-target clippy passes on Linux and macOS runners"
    expected: "cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used AND --target x86_64-apple-darwin both exit 0; files in scope: exec_strategy.rs, pty_proxy.rs, session_commands.rs, diagnostic.rs, pack_update_hint.rs"
    why_human: "Windows host cannot run cross-compilation clippy (x86_64-linux-gnu-gcc and macOS cc absent); deferred per CLAUDE.md MUST/NEVER and .planning/templates/cross-target-verify-checklist.md (skipped_gates_environmental). All 7 plan summaries document PARTIAL disposition. No merge to main until CI passes."
---

# Phase 55: UPST7 Cherry-pick Wave Verification Report

**Phase Goal:** The cross-platform straight-port clusters from the UPST7 audit are absorbed into the fork with correct D-19 trailers and the fork's invariants intact
**Verified:** 2026-06-05T02:21:49Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | JSONC profile parsing is present and fail-closed | VERIFIED | `raw_profile_to_jsonc_value` helper in `crates/nono-cli/src/profile/mod.rs:62-71` uses `jsonc_parser::ParseOptions{allow_comments:true, allow_trailing_commas:true}`; both `raw_profile_has_both_bypass_and_override_keys` (line 111) and `raw_profile_has_legacy_override_deny_key` (line 73) route through the same helper — CR-01 security blocker fixed in commit `61504656` |
| 2 | `target_binary` profile field is present and guard-restricted | VERIFIED | `Profile.binary: Option<String>` at `profile/mod.rs:2116`; `resolve_profile_binary` in `command_runtime.rs:24-42` enforces the `is_user_override || is_file_path_ref` guard before honoring the field |
| 3 | `opencode` is removed from built-in `policy.json` | VERIFIED | `grep '"opencode"' crates/nono-cli/data/policy.json` returns no built-in profile entry; the `opencode_linux` group remains (WR-04 advisory, not a blocker) |
| 4 | `timeouts.rs` module centralizes timeout constants with overflow clamping | VERIFIED | `crates/nono-cli/src/timeouts.rs` exists; exports `NONO_DETACH_STARTUP_TIMEOUT`, `NONO_PTY_DRAIN_TIMEOUT`, `NONO_PTY_ATTACH_TIMEOUT`; `MAX_TIMEOUT = Duration::from_secs(3600)` clamp in both `env_duration_secs` and `env_duration_millis` |
| 5 | `exec_strategy.rs` / `pty_proxy.rs` / `session_commands.rs` use `timeouts::` constants | VERIFIED | `use crate::timeouts;` at `exec_strategy.rs:52`; `timeouts::CHILD_POLL_INTERVAL` at line 1974; `pty_proxy.rs` and `session_commands.rs` duration literals replaced per 55-05-SUMMARY |
| 6 | Proxy 502 hardening (C4) — `write_upstream_failure` + CRLF sanitization | VERIFIED | `write_upstream_failure` function at `connect.rs:202`; CRLF sanitization `reason.replace(['\r', '\n'], " ")` at line 187; `let _ = write_upstream_failure(...)` pattern preserves upstream error at lines 76 and 95 |
| 7 | Pack-update-hint uses detached `std::process::Command` and atomic state writes | VERIFIED | `use std::process::{Command, Stdio};` at `pack_update_hint.rs:16`; pid-scoped atomic write at line 286 (`.{name}.{pid}.tmp` + rename); `NONO_NO_PACK_UPDATE_HINTS` at line 20 |
| 8 | ENV_LOCK acquired in `test_all_groups_no_deny_within_allow_overlap` | VERIFIED | `policy.rs:2241` acquires `crate::test_env::ENV_LOCK.lock()`; test-only change confirmed; `session_hooks` production hunk correctly deferred to Phase 58 (not present in policy.rs) |
| 9 | sigstore 0.8.0 bump and scrub.rs Cow deref port; D-32-15 invariant intact | VERIFIED | `crates/nono/Cargo.toml`: `sigstore-verify = { version = "0.8.0" }`; `crates/nono-cli/Cargo.toml`: `sigstore-sign = "0.8.0"`, `sigstore-trust-root = "0.8.0"`; `scrub.rs:188` uses `&*query_scrubbed == s`; `scrub.rs:278` uses `&*scrubbed != header_value.trim_start()`; `grep "TUF|tuf|sigstore|fetch" crates/nono/src/scrub.rs` returns zero hits |

**Score:** 9/9 truths verified

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|---------|
| 1 | `session_hooks` production field in `policy.rs:ProfileDef::to_raw_profile()` (partial port of upstream 1a764d05) | Phase 58 | Phase 58 goal: "Session Lifecycle Hooks — session_hooks profile field"; ROADMAP SC Phase 58 #1 explicitly covers `session_hooks` schema; `ProfileDef::SessionHooks` type does not exist in fork until Phase 58 |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-proxy/src/connect.rs` | C4 502 hardening + CRLF sanitization | VERIFIED | `write_upstream_failure`, `send_response` with `reason.replace(['\r', '\n'], " ")`, 4+ new tests |
| `crates/nono-cli/src/profile/mod.rs` | JSONC parsing + target_binary + CR-01 fix | VERIFIED | `raw_profile_to_jsonc_value` JSONC helper, `Profile.binary`, dual-key guard via same helper |
| `crates/nono-cli/data/policy.json` | `opencode` built-in profile removed | VERIFIED | No `"opencode"` profile entry; `opencode_linux` group orphaned (WR-04 advisory) |
| `crates/nono-cli/src/timeouts.rs` | New module with named constants | VERIFIED | File exists; `NONO_DETACH_STARTUP_TIMEOUT`, `NONO_PTY_DRAIN_TIMEOUT`, `NONO_PTY_ATTACH_TIMEOUT`, `MAX_TIMEOUT` all present |
| `crates/nono-cli/src/pack_update_hint.rs` | Detached process + atomic writes | VERIFIED | `std::process::Command` spawn; pid-scoped `.{name}.{pid}.tmp` atomic write |
| `crates/nono/src/diagnostic.rs` | `[save skipped]` annotations + canonical path pre-compute | VERIFIED | `canonical_denial_paths: Vec<PathBuf>`, `with_canonical_denial_paths`, `is_denial_suppressed`, `save skipped` label at line 1477 |
| `crates/nono-cli/src/output.rs` | `rfind` for access-mode splitting | VERIFIED | `content.rfind(" (")` at line 619 |
| `crates/nono-cli/src/policy.rs` | ENV_LOCK in test (C12) + target_binary (C7) | VERIFIED | `crate::test_env::ENV_LOCK.lock()` at line 2241; `binary: None` at line 198 |
| `crates/nono/src/scrub.rs` | Cow deref port + D-32-15 intact | VERIFIED | `&*query_scrubbed == s` at line 188; `&*scrubbed != ...` at line 278; zero sigstore/TUF imports |
| `crates/nono/Cargo.toml` | sigstore-verify 0.8.0 | VERIFIED | `sigstore-verify = { version = "0.8.0", ... }` |
| `crates/nono-cli/Cargo.toml` | sigstore-sign 0.8.0 + sigstore-trust-root 0.8.0 + jsonc-parser 0.32 | VERIFIED | All three deps present at specified versions |
| `.planning/phases/55-upst7-cherry-pick-wave/55-02-SC3-SCHEMA-COLLISION-CHECK.md` | SC3 schema-collision check (all CLEAR) | VERIFIED | File exists; `grep -c "CLEAR\|COLLISION\|PARTIAL"` = 12; 5 items checked, all CLEAR |
| `.planning/phases/55-upst7-cherry-pick-wave/55-07-C13-DISPOSITION-RESOLUTION.md` | C13 diff-inspection artifact (CLEAR verdict + D-32-15 statement) | VERIFIED | File exists; 4 CLEAR/COLLISION occurrences; CLEAR verdict; D-32-15 NOT REGRESSED stated |
| `.planning/REQUIREMENTS.md` | REQ-UPST7-02 prose updated (java-dev dropped, C9/C12/C13 added) | VERIFIED | Prose at line 27 enumerates all clusters; "java-dev / java_runtime has 0 commits" note present; no java-dev as will-sync item |
| `.planning/ROADMAP.md` | Phase 55 SC1 updated (java-dev dropped, C9/C12/C13 added); phase marked [x] completed | VERIFIED | SC1 at phases section lists all clusters; ROADMAP Milestones shows `[x] Phase 55` with "(completed 2026-06-05)" |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `profile/mod.rs` raw-key guards | `jsonc_parser` | `raw_profile_to_jsonc_value` helper | WIRED | Both `raw_profile_has_both_bypass_and_override_keys` and `raw_profile_has_legacy_override_deny_key` call `raw_profile_to_jsonc_value`; CR-01 BLOCKER closed |
| `command_runtime.rs` | `profile/mod.rs` | `resolve_profile_binary` + `is_file_path_ref` guard | WIRED | `resolve_program_from_profile_or_cli` calls `resolve_profile_binary`; guard at line 32 |
| `exec_strategy.rs` | `diagnostic.rs` | `with_canonical_denial_paths` + `ignored_denial_paths` field in `ExecConfig` | WIRED | `ExecConfig.ignored_denial_paths` at line 340; canonical path pre-computation and forwarding at lines 1621-1636 |
| `exec_strategy.rs` | `timeouts.rs` | `use crate::timeouts;` | WIRED | Import at line 52; `timeouts::CHILD_POLL_INTERVAL` used at line 1974 |
| `startup_runtime.rs` | `timeouts.rs` | `crate::timeouts::detach_startup_timeout()` | WIRED | `detach_timeout` at lines 75-78; `allows_pre_exec_update_check` includes `PackUpdateHintHelper` at line 18 |
| `pack_update_hint.rs` | `main.rs` | `PackUpdateHintHelper` subcommand dispatch | WIRED | `Commands::PackUpdateHintHelper(args) => pack_update_hint::run_refresh_helper(args)` at `app_runtime.rs:145`; `PackUpdateHintHelper(PackUpdateHintHelperArgs)` in `cli.rs:1101`; recursion guard in `startup_runtime.rs:18` |
| `policy.rs` test | `ENV_LOCK` mutex | `crate::test_env::ENV_LOCK.lock()` | WIRED | At line 2241; C7's `binary: None` at line 198 preserved |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `connect.rs write_upstream_failure` | `reason` (upstream error string) | `connect_to_resolved` error + DNS error paths | Yes — from real network errors | FLOWING |
| `pack_update_hint.rs refresh_in_background_process` | `PackUpdateHintState` | `load_state` reads JSON file; `save_state` writes with PID-scoped temp+rename | Yes — real file I/O | FLOWING |
| `diagnostic.rs is_denial_suppressed` | `canonical_denial_paths` | Pre-computed in `exec_strategy.rs` from `loaded_profile.filesystem.suppress_save_prompt` | Yes — from real profile config | FLOWING |
| `scrub.rs scrub_value_with_policy` | `query_scrubbed` / `scrubbed` | `Cow<'_, str>` from upstream scrub pipeline | Yes — mechanical Cow deref normalization | FLOWING |

### Behavioral Spot-Checks

Step 7b SKIPPED for pure code-absorption cherry-pick phase. No new runnable entry points were created; all changes are internal hardening/refactoring of existing commands. Windows build `cargo build --workspace` exits 0 per all 7 summaries.

### Probe Execution

No probes declared or discovered for this phase. Step 7c: N/A.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| REQ-UPST7-02 | Plans 55-01 through 55-07 | Cross-platform straight ports absorbed with D-19 trailers, schema-collision checks, D-43-E1 Windows invariant respected | SATISFIED | All 9 observable truths verified; 18 `Upstream-commit:` trailers in git log; 0 windows-specific files touched; SC3 and C13 disposition artifacts present and CLEAR |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/nono-cli/data/policy.json` | 380 | `opencode_linux` group definition with no profile referencing it | Warning (WR-04) | Dead config; may confuse future group surface audits. Not a security issue; `opencode` was extracted to OfficialPack registry — the `opencode_linux` group was a supporting group for the removed profile. Advisory follow-up; no audit-of-record reference required |
| `crates/nono-cli/src/cli.rs` | 2240-2241 | `requires = "detached"` combined with `env = "NONO_DETACH_STARTUP_TIMEOUT"` on `--detach-timeout` | Warning (WR-01) | A globally-set `NONO_DETACH_STARTUP_TIMEOUT` env var activates clap's `requires` check, breaking `nono run` for non-`--detached` invocations. Upstream bug absorbed as-is; fix requires dropping the `env =` binding or the `requires =` constraint |
| `crates/nono-cli/src/output.rs` | 617-626 | `rfind(" (")` split assumes trailing ` (access)` group; edge case: path contains ` (` with no trailing access group | Info (WR-03) | Misrenders bold span for denials with parens in path and no trailing access type. Advisory |
| `crates/nono/src/diagnostic.rs` | 1551-1561 | `canonical_for_denial` positional lookup via `iter().position(...)` — ambiguous if duplicate paths exist | Info (IN-01) | Harmless in practice (duplicate paths produce identical canonical values); fragile parallel-vec contract | 
| `crates/nono-cli/src/pack_update_hint.rs` | 194-206 | `let _ = child.spawn()` silently discards spawn errors | Info (IN-02) | Best-effort refresh; intentional. No debug logging on spawn failure |

No `TBD`, `FIXME`, or `XXX` markers found in any phase-modified file. No placeholder returns or hardcoded empty values introduced.

### Human Verification Required

#### 1. Cross-Target Clippy (Linux + macOS)

**Test:** Run `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` on a Linux or macOS CI runner.

**Expected:** Both commands exit 0 with no warnings or errors. Files most at risk: `exec_strategy.rs` (cfg-gated Unix code modified in C10 + C11: canonical denial pre-computation + timeout constants), `pty_proxy.rs` (C11: duration literal replacements), `session_commands.rs` (C11: duration literal replacements), `diagnostic.rs` (C10: pre-computation additions).

**Why human:** Windows host cannot install `x86_64-linux-gnu-gcc` or Apple macOS cross-compiler. Both Rust targets are installed but the C cross-linker is absent (`aws-lc-sys` build script fails before clippy runs). This is `skipped_gates_environmental` per `.planning/templates/cross-target-verify-checklist.md` — categorized as PARTIAL across plans 55-02, 55-03, 55-04, 55-05. CLAUDE.md MUST/NEVER requires live CI confirmation before main merge. The merge gate (D-55-03: v0.58.0 tag required) enforces this.

---

### Gaps Summary

No gaps blocking phase goal achievement. All 9 observable truths are verified in the codebase. The CR-01 security BLOCKER from code review was found and fixed (commit `61504656`) before this verification. The 4 advisory findings (WR-01, WR-03, WR-04, IN-01) are carry-forwards or non-blocking follow-ups from the code review; none prevent the phase goal.

The `human_needed` status is due exclusively to the cross-target clippy gate being deferred to live CI — a documented, accepted environmental limitation for this Windows-host project, not an implementation gap.

---

_Verified: 2026-06-05T02:21:49Z_
_Verifier: Claude (gsd-verifier)_
