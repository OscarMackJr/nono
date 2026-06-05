---
phase: 58
slug: session-lifecycle-hooks
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-05
---

# Phase 58 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p nono-cli hook` |
| **Full suite command** | `make test` |
| **Estimated runtime** | ~120 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p nono-cli hook`
- **After every plan wave:** Run `make test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 58-01-01 | 01 | 1 | REQ-HOOK-01 | — | SessionHooks parsed + threaded through to_raw_profile without data loss | unit | `cargo test -p nono-cli session_hooks` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono-cli/tests/schema_shape.rs` — schema-shape assertions for `session_hooks` (port upstream pattern)
- [ ] Unit tests for `is_dangerous_env_var()` Windows danger-var set
- [ ] Fail-closed behavior tests (before-hook non-zero → session does not start)

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Windows Low-IL broker-spawned hook execution + env-file ACL enforcement | REQ-HOOK-01 | Requires a real Win11 console + broker trust gate (dev-layout binary); CI cannot exercise the Low-IL spawn | Run a profile with `session_hooks.before`/`after` under `target\release\nono.exe` from a profile-covered cwd; confirm hook runs Low-IL, env-file is `CREATE_NEW` + restrictive ACL, and danger vars are filtered |

*If none: "All phase behaviors have automated verification."*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
