---
phase: "60"
plan: "02"
subsystem: "claude-code-hook / package-docs"
tags: ["powershell-steering", "tool-support-matrix", "windows-poc", "claude.md"]
dependency_graph:
  requires: ["60-01"]
  provides: ["REQ-STW-02"]
  affects: ["packages/claude-code/CLAUDE.md"]
tech_stack:
  added: []
  patterns: ["append-only CLAUDE.md section", "tool-support-matrix"]
key_files:
  created: []
  modified:
    - path: "packages/claude-code/CLAUDE.md"
      description: "Added ## Windows Shell Syntax section, ## File edits retry instruction, and ## Tool support matrix"
decisions:
  - "D-02 honored: PowerShell tool runner; CLAUDE.md is the only model-visible steering surface (profile meta.description is display-only)"
  - "D-03 honored: Medium-IL agent reads (Read/Glob/Grep) remain unconfined; documented in tool-support matrix"
  - "D-04 honored: Fail-closed deny with clear reason for unwrappable tools; tool-support matrix shipped for POC users"
  - "Runner profile verified: no changes needed (workdir.access==readwrite, network.block==true already correct)"
metrics:
  duration: "8 minutes"
  completed: "2026-05-29T18:42:18Z"
  tasks_completed: 2
  tasks_total: 2
  files_created: 0
  files_modified: 1
---

# Phase 60 Plan 02: PowerShell Steering + Tool-Support Matrix Summary

**One-liner:** Wired PowerShell-syntax steering via `packages/claude-code/CLAUDE.md` — added Windows Shell Syntax section with PS-syntax examples, deny+retry instruction for Write/Edit denials, and a tool-support matrix for POC users (D-02, D-04, REQ-STW-02).

## Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add Windows Shell Syntax section and tool-support matrix to CLAUDE.md | `46402bcc` | `packages/claude-code/CLAUDE.md` |
| 2 | Verify runner profile (no change needed) | n/a (no change) | `packages/claude-code/claude-code-tools-windows-runner.profile.json` |

## What Was Built

### Task 1: Windows Shell Syntax Section (CLAUDE.md)

Three new sections were appended to `packages/claude-code/CLAUDE.md` after the existing `## Working directory` section:

**`## Windows Shell Syntax`** — Shell syntax steering (D-02). Instructs Claude that the Bash tool is backed by native PowerShell on Windows. Provides PS-syntax equivalents for common shell operations:
- `Get-ChildItem` not `ls`
- `Get-Content` not `cat`
- `Test-Path` not `[ -f ... ]`
- `Copy-Item`, `Move-Item`, `Remove-Item`
- `Set-Content` for file writes

**`## File edits`** — Deny+retry routing instruction (RESEARCH.md Pattern 1 + deny+additionalContext mechanism from Plan 60-01). Instructs Claude to re-issue blocked Write/Edit calls as Bash tool calls using the PowerShell command from the denial context. Explicitly prohibits retrying Write/Edit directly after a denial.

**`## Tool support matrix`** — D-04 matrix with all 7 rows:
- Read, Glob, Grep: Allowed (unconfined reads; D-03 accepted posture)
- Bash: Confined Low-IL (PowerShell-backed; use PS syntax)
- Write, Edit, MultiEdit: Confined Low-IL via Bash retry (Hook converts to Bash+PS; files inside CWD only)
- NotebookEdit: Denied (not yet supported in Windows POC)
- WebFetch, WebSearch: Denied (network blocked)
- Task: Denied (subagent spawn is unconfined)
- mcp__*: Denied (MCP runs at Medium-IL outside confinement boundary)

All existing content in CLAUDE.md was preserved unchanged (append-only).

### Task 2: Runner Profile Verification (no change needed)

`packages/claude-code/claude-code-tools-windows-runner.profile.json` already contains all required fields:
- `"workdir": { "access": "readwrite" }` — CWD r+w grant (D-01 scope, required for Write/Edit confinement)
- `"network": { "block": true }` — Network blocked (T-60-09 threat mitigation verified)
- `"extends": "default"` — Security groups inherited

No changes were needed. Profile is correct for the file-op confinement path.

## Phase Gate Verification

| Check | Command | Result |
|-------|---------|--------|
| Windows Shell Syntax present | `grep -c "Windows Shell Syntax" packages/claude-code/CLAUDE.md` | 1 (PASS) |
| PowerShell mentions >= 3 | `grep -c "PowerShell" packages/claude-code/CLAUDE.md` | 6 (PASS) |
| Retry instruction present | `grep "re-issue" packages/claude-code/CLAUDE.md` | Found (PASS) |
| workdir.access==readwrite | `grep "readwrite" packages/claude-code/claude-code-tools-windows-runner.profile.json` | Found (PASS) |
| network.block==true | `grep '"block": true' packages/claude-code/claude-code-tools-windows-runner.profile.json` | Found (PASS) |
| Windows-host clippy | `cargo clippy --package nono-cli -- -D warnings -D clippy::unwrap_used` | Exit 0 (PASS) |
| claude_code_hook tests | `cargo test --package nono-cli --bin nono claude_code_hook` | 8/8 PASS |

## Cross-Target Clippy: PARTIAL

**Status: PARTIAL — deferred to CI (linux runner)**

`claude_code_hook.rs` contains `#[cfg(target_os = "windows")]` gated code added in Plan 60-01.
Per CLAUDE.md §Coding Standards, cross-target clippy is required:
```
cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```

Status: PARTIAL — Windows host cannot run these commands. Deferred to CI (linux runner).

The non-Windows path in `claude_code_hook.rs` returns `deny_response` without any PS command
construction, so Unix branches are simple and unlikely to have lint issues, but CI is the
authoritative verification.

**This plan itself (Plan 60-02) only modifies a markdown file and does not add any Rust code.**
The cross-target PARTIAL status is inherited from Plan 60-01 (the hook code changes).

## Pre-existing Test Failures (out of scope)

The full test suite `cargo test --package nono-cli --bin nono` has 6 pre-existing failures
that are unrelated to this plan's changes (CLAUDE.md is a markdown file with no Rust code):

| Test | Failure Reason | Pre-existing? |
|------|---------------|---------------|
| `broker_launch_assigns_child_to_job_object` | `nono-shell-broker.exe` not pre-built in worktree environment | Yes — requires `cargo build -p nono-shell-broker --release` |
| `write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file` | Same: broker binary missing | Yes |
| `test_init_allowed_when_pack_has_same_short_name` | Profile file already exists from prior test run | Yes — env isolation issue |
| `blocks_child_directory_capability` | `protected_paths` test failure | Yes — unrelated |
| `blocks_parent_directory_capability` | `protected_paths` test failure | Yes — unrelated |
| `requested_path_blocks_nonexistent_child_under_protected_root` | `protected_paths` test failure | Yes — unrelated |

All 8 claude_code_hook tests pass (`test result: ok. 8 passed; 0 failed`). These pre-existing
failures are logged to `deferred-items.md` scope per deviation rule boundary.

## Deviations from Plan

None — plan executed exactly as written.

- Runner profile: no changes needed (confirmed correct, documented as such)
- CLAUDE.md: append-only section added matching existing heading/body style
- Clippy: exit 0 on Windows host
- All phase gate checks pass

## HUMAN-UAT (Pending — Requires Real Win11 Host)

Per VALIDATION.md, the following manual verification steps are required after this plan:

a. Start Claude Code with the experimental profile from a project directory (not `~/.claude`)
b. Ask Claude to "list the files in the current directory" WITHOUT mentioning PowerShell
   Expected: Claude uses `Get-ChildItem` or `dir`, not `ls`
c. Ask Claude to write "hello world" to `test.txt`
   Expected: Claude's Write tool call is denied; Claude automatically retries as a Bash tool
   call with a PowerShell `Set-Content` command; `test.txt` is created with content "hello world"
d. Verify `test.txt` is inside the project CWD (not e.g. `~/.claude/`)
e. Ask Claude to write to a path outside the CWD (e.g. `../outside.txt`)
   Expected: denied at the OS boundary (Low-IL mandatory label); clear denial message
f. Self-disable guard: start Claude from `~/.claude` (or a dir with `.claude/` child)
   Expected: all file tools and Bash are denied with the self-disable reason

Note per RESEARCH.md Open Question A1: if deny+additionalContext does NOT cause Claude to
retry as Bash in step (c), document the fallback approach (direct nono subprocess from hook
handler) and file a follow-on todo before closing the phase.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries.
CLAUDE.md is a model-visible instruction file (documentation). T-60-08 (prompt injection
suppressing PS steering) accepted per STRIDE register — the CLAUDE.md note is a UX aid, not
a structural security control.

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| `packages/claude-code/CLAUDE.md` exists | FOUND |
| `packages/claude-code/claude-code-tools-windows-runner.profile.json` exists | FOUND |
| `60-02-SUMMARY.md` exists | FOUND |
| Commit `46402bcc` exists in git log | FOUND |
