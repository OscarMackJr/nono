---
phase: 58
slug: session-lifecycle-hooks
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-05
validated: 2026-06-06
---

# Phase 58 — Validation Strategy

> Per-phase validation contract for REQ-HOOK-01 (session lifecycle hooks). State-A audit:
> the original contract was a draft stub (1 mapped task); the phase actually shipped full
> automated coverage across all 3 plans. This contract reflects the as-built test inventory.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `cargo test --bin nono -- hook_runtime_windows session_hooks env_sanitization` |
| **Full suite command** | `cargo test -p nono-cli` (Unix `hook_runtime.rs` tests run on CI Linux/macOS) |
| **Estimated runtime** | ~1s (targeted) / ~120s (full suite) |

---

## Sampling Rate

- **After every task commit:** Run the targeted quick command (sub-second feedback)
- **After every plan wave:** Run `cargo test -p nono-cli`
- **Before `/gsd:verify-work`:** Full suite green on the Windows host + CI Linux/macOS lanes green for the `#[cfg(unix)]` `hook_runtime.rs` body
- **Max feedback latency:** ~1s (targeted), 120s (full)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement (SC) | Secure Behavior | Test Type | Automated Command | Tests | Status |
|---------|------|------|------------------|-----------------|-----------|-------------------|-------|--------|
| 58-01-01 | 01 | 1 | REQ-HOOK-01 (SC1) | `SessionHooks` parsed + threaded through the 4-lockstep + `to_raw_profile` without data loss; `deny_unknown_fields` rejects typos | unit | `cargo test --bin nono -- session_hooks` | `test_session_hooks_basic_deserialize`, `test_session_hooks_rejects_unknown_field`, `test_session_hooks_rejects_unknown_top_level_field`, `test_merge_profiles_session_hooks_child_overrides_per_field`, `test_merge_profiles_session_hooks_child_inherits_when_absent` | ✅ green (5/5) |
| 58-01-02 | 01 | 1 | REQ-HOOK-01 (SC1) | Schema exposes `session_hooks` property + `$defs`; built-in `ProfileDef` forwards the field | unit | `cargo test --bin nono -- policy::tests::test_schema_has_session_hooks` | `test_schema_has_session_hooks_property`, `test_schema_has_session_hooks_defs`, `test_to_raw_profile_includes_session_hooks` | ✅ green (3/3) |
| 58-02-01 | 02 | 2 | REQ-HOOK-01 (SC2/SC4) | Unix `hook_runtime` fail-closed divergence: before/after non-zero exit + timeout → `Err` (not upstream warn+Ok); `read_env_file` KEY=VALUE parse; `EnvFileGuard` O_EXCL+0o600 RAII | unit (cfg(unix)) | `cargo test -p nono-cli --bin nono -- hook_runtime::` (CI Linux/macOS) | `test_execute_before_hook_fail_closed`, `test_execute_before_hook_timeout_fail_closed`, `test_execute_after_hook_fail_closed`, `test_execute_before_hook_basic`, `test_read_env_file_*` (4), `test_env_file_guard_removes_file_on_drop` | 🟡 CI-deferred (Unix-gated; not compiled on Windows host) |
| 58-02-02 | 02 | 2 | REQ-HOOK-01 (SC4) | `execute_sandboxed` before-hook `Err` aborts the session (`?` propagation) | unit (cfg(unix)) | `cargo test -p nono-cli --bin nono -- test_execute_sandboxed_before_hook_err_aborts_session` (CI Linux/macOS) | `test_execute_sandboxed_before_hook_err_aborts_session` (`execution_runtime.rs:735`) | 🟡 CI-deferred (Unix-gated) |
| 58-03-01 | 03 | 2 | REQ-HOOK-01 (SC3) | Windows hook validation: rejects relative/non-file paths + world-writable parent (D-10 unconditional DACL+EqualSid); env-file `CREATE_NEW` anti-clobber (D-08) | unit (cfg(windows)) | `cargo test --bin nono -- hook_runtime_windows` | `test_validate_hook_script_windows_rejects_relative`, `test_validate_hook_script_windows_rejects_non_file`, `test_validate_rejects_world_writable_parent`, `test_env_file_create_new_prevents_clobber` | ✅ green (4/4) |
| 58-03-02 | 03 | 2 | REQ-HOOK-01 (SC3) | Windows danger-var filter (D-09, 10 vars, `eq_ignore_ascii_case`); danger vars stripped from the hook env-file read path | unit | `cargo test --bin nono -- env_sanitization::tests::test_windows_dangerous_vars hook_runtime_windows::tests::test_windows_dangerous_vars_filtered_from_env_file` | `test_windows_dangerous_vars_blocked`, `test_should_skip_env_var_matches_windows_keys_case_insensitively`, `test_windows_dangerous_vars_filtered_from_env_file` | ✅ green (3/3) |
| 58-03-03 | 03 | 2 | REQ-HOOK-01 (SC3) | PowerShell hook spawn regressions: `SystemRoot` baseline re-inject (F-58-UAT-01, no -65536 CLR fail) + `\\?\` verbatim-prefix strip for `-File` (F-58-UAT-02, no false unsigned-block); CR-02 job-assignment timeout fail-closed | unit/integration (cfg(windows)) | `cargo test --bin nono -- hook_runtime_windows` | `test_execute_before_hook_powershell_does_not_clr_fail`, `test_strip_verbatim_prefix_deterministic`, `test_cr02_timeout_hook_exits_cleanly` | ✅ green (3/3) |

*Status: ⬜ pending · ✅ green · 🟡 CI-deferred (Unix cfg branch, per CLAUDE.md cross-target rule) · ❌ red · ⚠️ flaky*

**Targeted suite live run (Windows host, 2026-06-06):** `cargo test --bin nono -- hook_runtime_windows session_hooks env_sanitization` → **46 passed; 0 failed**.

---

## Wave 0 Requirements

The draft contract's three Wave-0 items are all COVERED by as-built tests — no net-new scaffolding required:

- [x] Schema-shape assertions for `session_hooks` → `test_schema_has_session_hooks_property` + `test_schema_has_session_hooks_defs` (policy.rs tests)
- [x] `is_dangerous_env_var()` Windows danger-var set → `test_windows_dangerous_vars_blocked` (10 vars, `eq_ignore_ascii_case`)
- [x] Fail-closed behavior (before-hook non-zero → session does not start) → `test_execute_before_hook_fail_closed` (Unix) + `test_execute_sandboxed_before_hook_err_aborts_session` (Unix) + Live Windows UAT fail-closed PASS

*Existing infrastructure covers all phase requirements. No Wave-0 stubs outstanding.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Status |
|----------|-------------|------------|--------|
| End-to-end hook execution on real Win11 (before-hook env export into sandboxed child + fail-closed on non-zero) | REQ-HOOK-01 (SC1/SC4) | Requires a real Win11 console + release `nono.exe` + profile-covered cwd; the unit tests don't spawn the full supervised pipeline | ✅ **PASS** — operator UAT 2026-06-05, real Win11 build 26200, release v0.62.0: `GOT=hello_from_hook` exported; hook `exit 1` → `Before-hook exited with code 1 (fail-closed)`, nono exit 1, child never ran (see 58-03-SUMMARY § Live Windows UAT) |
| Windows hooks spawn as a **Low-IL** primary-token process (vs the current Medium-IL) | REQ-HOOK-01 (SC3) | `std::process::Command` has no custom-token API; requires raw `CreateProcessAsUserW` plumbing. Token is created (`create_low_integrity_primary_token`) but not yet plumbed into the spawn | 🟡 **DEFERRED** — ADR-tracked (`adr-58-windows-hook-executor.md`, Research Open Question 1 / CR-01). A real defense-in-depth confinement gap (hooks currently run Medium-IL with Job-Object containment), explicitly recorded — not a silent omission. Close in a follow-up phase. |

---

## Validation Sign-Off

- [x] All tasks have an `<automated>` verify command or are CI-deferred (Unix cfg branch, documented)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covered (all three draft items map to existing green tests)
- [x] No watch-mode flags
- [x] Feedback latency < 120s (~1s targeted)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-06-06

---

## Validation Audit 2026-06-06

State-A audit. The original `58-VALIDATION.md` was a draft stub (`status: draft`,
`nyquist_compliant: false`, 1 mapped task, 3 unchecked Wave-0 items) authored before execution.
The phase actually shipped comprehensive automated coverage across all 3 plans (per
58-01/02/03-SUMMARY + 58-VERIFICATION). This audit reconstructed the per-task map from the
as-built test inventory and ran the targeted suite live — **no gaps requiring test generation
were found**, so no `gsd-nyquist-auditor` spawn was needed.

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 (all behaviors already COVERED) |
| Escalated | 0 |

**Live run (Windows host):** `cargo test --bin nono -- hook_runtime_windows session_hooks env_sanitization` → **46 passed, 0 failed**.

**Coverage by success criterion:**
- **SC1** (hooks run at start/stop, output in logs) — profile-pipeline tests (8) + operator Live UAT PASS (`GOT=hello_from_hook`)
- **SC2** (Unix upstream `hook_runtime` preserved, fail-closed divergence) — `hook_runtime.rs` tests (CI-deferred, Unix-gated): before/after fail-closed + timeout + `read_env_file` + `EnvFileGuard`
- **SC3** (Windows broker-spawned design + ADR) — `hook_runtime_windows` tests (10) + `env_sanitization` Windows danger-var tests + `adr-58-windows-hook-executor.md` (8 sections). The Low-IL spawn leg is the documented D-05 deferral (Manual-Only above)
- **SC4** (fail-closed, never silently skipped) — `test_execute_before_hook_fail_closed` + `test_execute_sandboxed_before_hook_err_aborts_session` + Live UAT fail-closed PASS

**Carry-forward (NOT nyquist gaps; tracked in 58-REVIEW.md):** the D-05 Medium-IL→Low-IL hook
confinement gap (defense-in-depth, ADR-tracked), WR-02 stdio-handle leak on timeout, WR-03 env-file
`len as usize` guard, WR-04 Unix `LD_*`/`DYLD_*` case-sensitivity. These are accepted code-review
follow-ups, not validation-coverage gaps.
