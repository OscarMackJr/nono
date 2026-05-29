---
phase: 60-sandbox-the-tools-confined-coding-loop-v2-9
verified: 2026-05-29T00:00:00Z
status: gaps_found
score: 2/4 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Confinement invariants preserved — per-call Write grant must not re-introduce a write path to ~/.claude hook state (SC 3)"
    status: failed
    reason: "path_covers() fails open when the child path does not exist on disk. On Windows, canonical_existing_path() fails for any non-existent path. The fallback `unwrap_or_else(|_| child.to_path_buf())` returns the raw non-canonical child (e.g., C:\\Users\\X\\.claude.json), but the parent is the \\?\\-canonical CWD. Path::starts_with() with a \\?\\-prefix parent will never match a raw C:\\ child — so the guard returns false (no risk) when .claude.json or .claude.json.lock are absent, which is the common case for .claude.json.lock. This is CR-01 from 60-REVIEW.md — a documented BLOCKER that directly undermines SC 3."
    artifacts:
      - path: "crates/nono-cli/src/claude_code_hook.rs"
        issue: "path_covers() at line 334-337: unwrap_or_else fallback returns raw child path; Path::starts_with() comparison against \\?\\-canonical parent fails silently (returns false, guard fails open)"
      - path: "crates/nono-cli/src/claude_code_hook.rs"
        issue: "windows_cwd_guard_denies_home_claude_ancestor test (line 662-674) creates .claude.json on disk before testing — never exercises the fail-open path where the file is absent"
    missing:
      - "Fix path_covers() to canonicalize the deepest existing ancestor of the child and re-append missing tail components, OR treat canonicalization failure of a security-relevant target as covered (deny) instead of falling back to the raw path"
      - "Add regression test: CWD is an ancestor of ~/.claude.json when that file does NOT exist on disk; guard must still fire"
human_verification:
  - test: "Confined file edit lands on Win11 host"
    expected: "A POC user has Claude edit a file inside the project CWD; the edit lands via the confined Low-IL path (deny+additionalContext Bash retry succeeds at the OS level)"
    why_human: "Requires a real Win11 host running the Claude Code TUI at Medium IL with the experimental profile; OS mandatory-label enforcement cannot be exercised in CI"
  - test: "Out-of-scope write denied at OS boundary"
    expected: "Claude attempts a write outside the granted CWD; the Low-IL mandatory label denies it at the OS boundary, not merely at the hook layer"
    why_human: "Requires live Windows with MIC enforcement; the hook layer cannot exercise the OS boundary"
  - test: "deny+additionalContext steers Claude to Bash/PS retry (A1 behavioral assumption)"
    expected: "Claude reliably re-issues the blocked Write/Edit as a Bash tool call using the PS command from additionalContext, resulting in the file being written. If this is unreliable, the fallback mechanism must be documented."
    why_human: "Depends on Claude model's runtime behavior when receiving a deny with additionalContext; not unit-testable"
  - test: "Usable shell story — PowerShell steering without manual prompting (SC 2)"
    expected: "Typical run-command requests from a POC user succeed without manually instructing Claude to use PowerShell syntax; Claude uses Get-ChildItem, not ls, unprompted"
    why_human: "Requires live agent run to confirm model reads and applies the CLAUDE.md steering note"
  - test: "End-to-end POC UAT — read, edit, run task on Win11 (SC 4)"
    expected: "A POC user completes a small read -> edit -> run task on a Win11 host with the experimental profile; edits are confined, an out-of-scope write is denied at the OS boundary"
    why_human: "Full E2E requires Win11 host + Claude Code TUI + experimental profile loaded"
---

# Phase 60: Sandbox-the-Tools — Confined Coding Loop (v2.9) Verification Report

**Phase Goal:** A Windows POC user runs the Claude Code TUI at Medium IL and the agent completes a full coding loop — read, run commands, and edit files — with every side-effecting operation confined to a Low-IL nono jail. File edits work (confined) instead of being denied.
**Verified:** 2026-05-29
**Status:** gaps_found
**Re-verification:** No — initial verification

---

## Step 0: Previous Verification

No previous VERIFICATION.md found. Initial mode.

---

## Step 1 & 2: Phase Goal and Must-Haves

**Source:** ROADMAP.md Phase 60 Success Criteria (authoritative). REQ-STW-01 and REQ-STW-02 are v2.9-track requirements defined in ROADMAP.md (Coverage table, rows 176-177) but not in REQUIREMENTS.md (which covers v2.8 scope only). This is by design — the v2.9 track runs in parallel.

**Note on implementation gap vs SC wording:** SC 1 says file edits "execute as Low-IL nono-confined file operations." The plans deliberately chose a deny+additionalContext retry mechanism rather than direct execution from the hook. Per the verification context provided, this is an intentional design decision that the plans explicitly committed to — the goal-backward check should assess whether the chosen mechanism is substantively implemented and whether it can deliver the SC's outcome, not whether it matches the literal wording of "execute as." This is assessed under SC 1 below.

---

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC 1: Confined file edits — Write/Edit/MultiEdit execute as Low-IL nono-confined file operations; a write outside granted scope is denied at OS boundary (HUMAN-UAT clause) | PARTIAL | Mechanism exists and is substantive: deny+additionalContext with bare inner PS expression is present in claude_code_hook.rs lines 60-203. Three Windows-gated builder functions produce real PS expressions. The mechanism satisfies SC 1's code-verifiable portion. HUMAN-UAT (file lands; OS boundary deny) not yet run — classified as human_needed items. See note below on goal-vs-implementation gap. |
| 2 | SC 2: Usable shell story — typical run-command requests succeed without manually prompting "use PowerShell syntax" | PARTIAL | CLAUDE.md at packages/claude-code/CLAUDE.md lines 29-62 has the ## Windows Shell Syntax section with PS syntax examples and the retry instruction. grep confirms: "Windows Shell Syntax" (1 match), "PowerShell" (6 matches), "re-issue" (1 match). Code-verifiable portion VERIFIED. Live behavioral confirmation is HUMAN-UAT. |
| 3 | SC 3: Confinement invariants preserved — per-call Write grant must not re-introduce a write path to ~/.claude hook state; self-disable guard and deny-by-default for unwrappable surfaces remain intact | FAILED | path_covers() fails open when the child path (e.g., .claude.json, .claude.json.lock) does not exist on disk. Line 335: `unwrap_or_else(|_| child.to_path_buf())` returns a raw C:\\... path, but the parent is the \\?\\-canonical CWD. Path::starts_with() fails the comparison and the guard returns false (no risk) — a fail-open in a fail-secure codebase. Confirmed by CR-01 in 60-REVIEW.md. The existing test at line 666 pre-creates .claude.json, masking this path. |
| 4 | SC 4: End-to-end POC UAT — a POC user completes read -> edit -> run on Win11 host with experimental profile; edits are confined, out-of-scope write is denied | HUMAN-UAT | Cannot be verified from codebase alone. Requires live Win11 + Claude Code TUI. |

**Score:** 2/4 truths verified (SC 1 code portion + SC 2 code portion pass; SC 3 FAILED; SC 4 HUMAN-UAT)

**On SC 1 goal-vs-implementation gap:** The ROADMAP says "execute as Low-IL nono-confined file operations" but the plans deliberately chose deny+additionalContext (the agent is instructed to retry via the Bash tool, which IS wrapped as a confined Low-IL subprocess). This is a two-step confined execution rather than a single-step one; the SC outcome (file write lands in Low-IL jail) is achievable via the mechanism — but only if the A1 behavioral assumption holds (Claude reliably retries). The A1 assumption is flagged as a human verification item, not a BLOCKER, consistent with the verification context.

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/claude_code_hook.rs` | Write/Edit/MultiEdit confined arms + NotebookEdit informative deny + unit tests | WIRED | 894-line file with match dispatch (lines 59-217), three Windows-gated builder functions (lines 413-489), 6 new tests + updated deny test (lines 709-893). All present and substantive. |
| `packages/claude-code/CLAUDE.md` | PowerShell steering note + tool-support matrix for POC users | WIRED | 62-line file with ## Windows Shell Syntax (line 29), ## File edits (line 41), ## Tool support matrix (line 50) appended to existing content. |
| `packages/claude-code/claude-code-tools-windows-runner.profile.json` | Runner profile: workdir.access==readwrite, network.block==true | VERIFIED | File confirmed: "workdir": { "access": "readwrite" } (line 27), "network": { "block": true } (line 26). No changes needed. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| Write/Edit/MultiEdit match arms | cwd_self_disable_risk_reason() | First call in each Windows-gated arm body | WIRED | Lines 62, 104, 158: #[cfg(target_os = "windows")] if let Some(reason) = cwd_self_disable_risk_reason()? confirmed at each arm entry point. |
| build_confined_write_cmd / build_confined_edit_cmd / build_confined_multiedit_cmd | bare PS inner expression (no wrapped_bash_command) | Returns raw PS string; Bash arm wraps exactly once | WIRED | Builder functions at lines 414-489 return Ok(ps_inner) without calling wrapped_bash_command. The additionalContext embeds ps_inner, not the encoded outer wrapper. |
| confined arm response | hookSpecificOutput.additionalContext | json!({}) inline construction | WIRED | Lines 85-92, 139-146, 187-194: use json!({}) with additionalContext field, not deny_response() helper. |
| CLAUDE.md ## Windows Shell Syntax section | Claude agent session context | Loaded as project instructions at session start | WIRED (code side) | grep "Windows Shell Syntax" packages/claude-code/CLAUDE.md returns 1 match. HUMAN-UAT required for runtime model-loading confirmation. |
| CLAUDE.md retry instruction for Write/Edit denials | deny+additionalContext mechanism from Plan 60-01 | Claude reads denial context and re-issues as Bash tool call | PARTIAL | "re-issue" text present in CLAUDE.md. Behavioral outcome (Claude actually retries) is the A1 assumption — requires HUMAN-UAT. |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| claude_code_hook.rs Write arm | ps_inner (PS expression) | build_confined_write_cmd(file_path, content) | Yes — real base64 encoding + PS expression construction | FLOWING |
| claude_code_hook.rs Edit arm | ps_inner | build_confined_edit_cmd(file_path, old_string, new_string) | Yes — real literal replace PS expression | FLOWING |
| claude_code_hook.rs MultiEdit arm | ps_inner | build_confined_multiedit_cmd(file_path, &edits) | Yes — real in-memory multi-replace PS expression | FLOWING |
| path_covers() guard | bool (covers/not) | canonical_existing_path(child) OR raw fallback | Produces incorrect result when child is non-existent (fail-open) | HOLLOW — correct on happy path, fails open on missing-file path |

---

### Behavioral Spot-Checks

The phase modifies Rust source; the only runnable check without a server is the test suite. Per the SUMMARY, the executor ran `cargo test --bin nono claude_code_hook` and reported 14 passed, 0 failed. This verifier cannot independently re-run tests on this Windows host via these tools, but can verify the test functions exist and their assertions are structurally correct from the source.

| Behavior | Evidence | Status |
|----------|----------|--------|
| pre_tool_use_write_returns_deny_with_ps_cmd exists and asserts deny + non-empty additionalContext (Windows) | Line 710-738: test function present, correct assertions | VERIFIED (static) |
| pre_tool_use_edit_returns_deny_with_ps_cmd exists | Line 742-770: present | VERIFIED (static) |
| pre_tool_use_multiedit_returns_deny_with_ps_cmd exists | Line 774-810: present | VERIFIED (static) |
| pre_tool_use_notebookedit_deny_no_ps_cmd exists — asserts deny with absent additionalContext | Line 813-838: present | VERIFIED (static) |
| pre_tool_use_write_content_with_special_chars exists (Windows-gated) | Line 843-870: present | VERIFIED (static) |
| windows_write_arm_cwd_guard_fires_before_ps_cmd exists (Windows-gated) | Line 874-892: present — calls cwd_self_disable_risk_reason_for directly | VERIFIED (static) |
| CR-01 fail-open NOT covered by test suite | windows_cwd_guard_denies_home_claude_ancestor (line 662-674) creates .claude.json before test; never tests missing-file path | FAIL — gap in test coverage |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REQ-STW-01 (v2.9 track) | 60-01-PLAN.md | Confined file ops: Write/Edit/MultiEdit execute as Low-IL nono-confined file operations scoped to project CWD; out-of-scope writes denied at OS boundary | BLOCKED | Mechanism is present but SC 3 (confinement invariants) fails due to CR-01 fail-open bug in path_covers(); HUMAN-UAT items also pending |
| REQ-STW-02 (v2.9 track) | 60-02-PLAN.md | Usable shell story: PowerShell-backed Bash runner + Claude steered to emit PS syntax automatically | NEEDS HUMAN | CLAUDE.md section present and correct. Behavioral confirmation (model uses PS syntax unprompted) requires live agent run. |

**Orphaned requirements check:** REQ-STW-01 and REQ-STW-02 do not appear in REQUIREMENTS.md (which covers v2.8 scope). They are tracked in ROADMAP.md Coverage table (rows 176-177) as v2.9-track requirements. This is by design — not orphaned.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/nono-cli/src/claude_code_hook.rs` | 335 | `unwrap_or_else(\|_\| child.to_path_buf())` on security-critical path | BLOCKER | Guard fails open when child path (e.g., .claude.json.lock) does not exist; \\?\\-canonical parent never matches raw C:\\ child in Path::starts_with() |
| `crates/nono-cli/src/claude_code_hook.rs` | 80 | `.unwrap_or("")` on content field | WARNING | Not a BLOCKER — content default of empty string is safe (empty write is a valid, if useless, operation). Not a security regression. |
| `crates/nono-cli/src/claude_code_hook.rs` | 283-285 | command_contains_nono_wrapper: substring match `" nono run "` / `"nono run "` | WARNING | WR-04 from 60-REVIEW.md: the Windows wrapper emits `& 'nono.exe' run`, not `" nono run "`, so guard may not recognize its own output; user commands mentioning `nono run` pass through unwrapped. Correctness/security risk on Windows. |
| `crates/nono-cli/src/claude_code_hook.rs` | 65-68, 107-111, 161-167 | Missing tool_input propagates Err (not deny) | WARNING | WR-03 from 60-REVIEW.md: malformed PreToolUse payload for write-class tools may cause hook exit with non-zero code + no JSON, whose downstream handling by Claude Code is unspecified (may default to allow). |

**Debt-marker check:** No TBD, FIXME, or XXX markers found in the modified files (claude_code_hook.rs and packages/claude-code/CLAUDE.md).

---

### Human Verification Required

### 1. Confined file edit lands on Win11 host (SC 1 HUMAN-UAT)

**Test:** Start Claude Code with the experimental profile from a project directory (not ~/.claude). Ask Claude to write "hello world" to test.txt. Confirm the Write tool call is denied; Claude automatically retries as a Bash tool call with a PowerShell command from additionalContext; test.txt is created with content "hello world".
**Expected:** test.txt exists in project CWD with correct content.
**Why human:** Requires real Win11 host + Claude Code TUI + OS mandatory-label enforcement.

### 2. Out-of-scope write denied at OS boundary (SC 1 HUMAN-UAT)

**Test:** Ask Claude to write to a path outside the CWD (e.g., ../outside.txt or C:\Windows\outside.txt). Confirm it is denied at the OS boundary by the Low-IL mandatory label, not merely at the hook layer.
**Expected:** OS-level denial with a clear denial message; no file created.
**Why human:** OS MIC enforcement requires live Windows; cannot be unit-tested.

### 3. A1 behavioral assumption: deny+additionalContext causes Claude to retry as Bash

**Test:** Observe whether Claude reliably re-issues a blocked Write/Edit as a Bash tool call using the PS command from additionalContext. Per RESEARCH.md Open Question A1: if this is unreliable, the fallback approach (direct nono subprocess from hook handler) must be documented and a follow-on todo filed before phase close.
**Expected:** Claude retries without user intervention; file lands. If not reliable, fallback is documented.
**Why human:** Depends on Claude model's runtime retry behavior when receiving a deny with additionalContext.

### 4. Usable shell story — PowerShell steering without manual prompting (SC 2)

**Test:** Ask Claude to "list the files in the current directory" without mentioning PowerShell. Ask Claude to run a build command.
**Expected:** Claude uses Get-ChildItem (not ls), and typical shell commands succeed without manual PS-syntax prompting.
**Why human:** Requires live agent run to confirm model reads and applies the CLAUDE.md Windows Shell Syntax section.

### 5. Self-disable guard preserved under live CWD grant (SC 3 partial)

**Test:** Launch Claude from ~/.claude (or a dir with a .claude/ child). Attempt a file edit.
**Expected:** All file tools and Bash are denied with the self-disable reason; no PS command is offered.
**Why human:** The unit tests confirm the guard logic path, but the interaction with the live ~/ home dir and the real Windows \\?\ path representation needs live host validation, especially given CR-01.

### 6. End-to-end POC UAT — read, edit, run task on Win11 (SC 4)

**Test:** A POC user completes a small read → edit → run task on a Win11 host with the experimental profile loaded.
**Expected:** Edits are confined (land via Low-IL path), an out-of-scope write is denied, shell commands work with PS syntax.
**Why human:** Full E2E requires Win11 host + Claude Code TUI + experimental profile loaded; cannot be simulated.

---

## Gaps Summary

### BLOCKER — CR-01: path_covers() fails open on non-existent child paths (SC 3)

**Root cause:** `path_covers()` at line 334-337 uses `unwrap_or_else(|_| child.to_path_buf())` as a fallback when `canonical_existing_path(child)` fails. On Windows, `Path::canonicalize()` fails for any path that does not exist. The CWD parent is canonicalized to `\\?\C:\Users\X` form, but the non-existent child falls back to `C:\Users\X\.claude.json.lock` (raw form). `Path::starts_with()` performs component-level comparison: `\\?\C:\Users\X` is NOT a prefix of `C:\Users\X\.claude.json.lock` because the `\\?\` prefix makes them different path roots. The guard returns `false` ("no risk") when the answer should be `true` ("risk — deny").

**Security impact:** The `.claude.json.lock` file is absent while no session holds the lock (the common state). The `.claude.json` file may also be absent on a fresh install. When these files are absent, the `cwd_covers_home_claude_state()` check only fires if `.claude/` directory itself exists (and can be canonicalized). A CWD that IS the user home directory, on a machine where `.claude/` does not yet exist but will be created by the agent, passes the guard — the `--allow-cwd` grant then covers the home directory, re-introducing the write-to-`~/.claude` path that the guard is meant to prevent.

**Fix required:** Either (a) canonicalize the deepest existing ancestor of the child and re-append missing tail components so both sides use the same extended-length representation, or (b) treat canonicalization failure of a security-relevant target path as "covered" (return deny). The test `windows_cwd_guard_denies_home_claude_ancestor` must be updated to test the missing-file path.

**Relationship to SC 3:** SC 3 says "the per-call Write grant must not re-introduce a write path to `~/.claude` hook state." CR-01 is a specific, demonstrable path by which it can — when `.claude.json.lock` is absent. SC 3 is FAILED.

### Human verification items (5 items — not blockers for gap fix, but required before phase close)

Items 1-6 above. All are real behavioral questions that require a Win11 host. The A1 assumption (item 3) is particularly critical: if `deny+additionalContext` does not reliably cause Claude to retry as Bash, the entire SC 1 mechanism is non-functional regardless of code correctness.

---

_Verified: 2026-05-29_
_Verifier: Claude (gsd-verifier)_
