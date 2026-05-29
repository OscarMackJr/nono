---
phase: 60
slug: sandbox-the-tools-confined-coding-loop-v2-9
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-29
---

# Phase 60 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) |
| **Config file** | none — workspace `Cargo.toml`; tests live inline in `crates/nono-cli/src/claude_code_hook.rs` (`#[cfg(test)] mod tests`) |
| **Quick run command** | `cargo test --bin nono claude_code_hook` |
| **Full suite command** | `make test` |
| **Estimated runtime** | ~30–90 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --bin nono claude_code_hook`
- **After every plan wave:** Run `make test`
- **Before `/gsd:verify-work`:** Full suite must be green + `make ci` (clippy + fmt) clean
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

> Filled by the planner — one row per task. The HUMAN-UAT rows (confined edit lands; out-of-scope write denied at OS boundary; deny+retry steers Claude to the Bash/PS path) are Manual-Only by nature (require a real Win11 host + the Claude Code TUI).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | REQ-STW-01 / REQ-STW-02 | TBD | TBD | unit | `cargo test --bin nono claude_code_hook` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.* The 8 existing `claude_code_hook` unit tests pass; new hook arms add tests to the same inline `#[cfg(test)] mod tests` block. No new test framework or file scaffolding needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Confined file edit lands (in-project) | REQ-STW-01 | Requires a real Win11 host running the Claude Code TUI at Medium IL with the experimental profile; OS mandatory-label enforcement cannot be exercised in CI | Have Claude edit a file inside the project CWD; confirm the edit lands via the confined Low-IL path |
| Out-of-scope write denied at OS boundary | REQ-STW-01 | OS-enforced (Low-IL mandatory label); needs live Windows | Have Claude attempt a write outside the granted CWD; confirm it is denied at the OS boundary |
| `deny`+`additionalContext` steers Claude to Bash/PS retry | REQ-STW-01 | Behavioral assumption A1 — depends on Claude's runtime retry behavior, not unit-testable | Ask Claude to write a file; confirm success via the Bash+PowerShell retry path (not a user-visible failure) |
| Usable shell story (PowerShell steering) | REQ-STW-02 | Requires live agent run to confirm typical run-command requests succeed without manual "use PowerShell syntax" prompting | Run a few typical run-command tasks; confirm no manual PowerShell-syntax steering is needed |
| Self-disable guard preserved under CWD grant | REQ-STW-01 (D-05) | OS + path-coverage interaction; partially unit-tested (guard tests) but the live `~/.claude` write-deny needs a real host | Launch from `~/.claude`; confirm the file-op confinement refuses to wrap (guard fires) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies (HUMAN-UAT rows documented as Manual-Only)
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
