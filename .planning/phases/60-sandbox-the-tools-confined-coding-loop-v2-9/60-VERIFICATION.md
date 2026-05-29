---
phase: 60-sandbox-the-tools-confined-coding-loop-v2-9
verified: 2026-05-29T12:00:00Z
status: human_needed
score: 3/4 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 2/4
  gaps_closed:
    - "SC 3 — Confinement invariants preserved: path_covers() fails open when child path does not exist on disk (CR-01 BLOCKER — now CLOSED)"
  gaps_remaining: []
  regressions: []
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
**Status:** human_needed
**Re-verification:** Yes — after gap closure (Plan 60-03 closed the CR-01 BLOCKER)

---

## Step 0: Previous Verification

Previous VERIFICATION.md found with `status: gaps_found`, `score: 2/4`. Single BLOCKER: CR-01 (path_covers() fails open when child path does not exist on disk). Plan 60-03 was executed to close CR-01. This is a re-verification.

RE-VERIFICATION MODE: Full 3-level verification applied to the previously failed item (SC 3 / CR-01). Quick regression check applied to previously passing items (SC 1 code portion, SC 2 code portion).

---

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC 1: Confined file edits — Write/Edit/MultiEdit execute as Low-IL nono-confined file operations; a write outside granted scope is denied at OS boundary (HUMAN-UAT clause) | PARTIAL (code VERIFIED, HUMAN-UAT pending) | deny+additionalContext mechanism present: match arms at lines 60-201, three Windows-gated builder functions (lines 438-510). Regression check: structure unchanged from Plan 60-01. HUMAN-UAT items remain. |
| 2 | SC 2: Usable shell story — typical run-command requests succeed without manually prompting "use PowerShell syntax" | PARTIAL (code VERIFIED, HUMAN-UAT pending) | CLAUDE.md at packages/claude-code/CLAUDE.md contains "## Windows Shell Syntax" with PS syntax examples and retry instruction. Regression check: unchanged from Plan 60-02. HUMAN-UAT required for runtime behavioral confirmation. |
| 3 | SC 3: Confinement invariants preserved — per-call Write grant must not re-introduce a write path to ~/.claude hook state; self-disable guard and deny-by-default for unwrappable surfaces remain intact | VERIFIED | CR-01 CLOSED. See full verification below. path_covers() now uses canonicalize_with_existing_prefix(); cwd_covers_home_claude_state() has symmetric cwd.starts_with() check (WR-01); cwd_self_disable_risk_reason_for() fails closed when home_dir() returns None (WR-02). Two regression tests added and Windows-gated. |
| 4 | SC 4: End-to-end POC UAT — a POC user completes read -> edit -> run on Win11 host with experimental profile; edits are confined, out-of-scope write is denied | HUMAN-UAT | Cannot be verified from codebase alone. Requires live Win11 + Claude Code TUI. Unchanged from initial verification. |

**Score:** 3/4 truths verified (SC 1 code + SC 2 code + SC 3 pass; SC 4 HUMAN-UAT only)

---

## SC 3 / CR-01 Verification (Full 3-Level + Data-Flow)

### Level 1 — Artifact Exists and Is Substantive

`crates/nono-cli/src/claude_code_hook.rs` exists at 959 lines. The four guard functions are all present:

- `path_covers()` at lines 330-333
- `canonicalize_with_existing_prefix()` at lines 336-362
- `cwd_covers_home_claude_state()` at lines 301-314
- `cwd_self_disable_risk_reason_for()` at lines 279-298
- `self_disable_reason_no_home()` at lines 325-327

All carry `#[cfg(target_os = "windows")]`. No stubs or placeholder returns.

### Level 2 — Substantive Implementation (CR-01 Root Cause Fixed)

**CR-01: path_covers() fail-open on non-existent paths — FIXED**

Prior buggy body (documented in 60-VERIFICATION.md initial):
```
let child = canonical_existing_path(child).unwrap_or_else(|_| child.to_path_buf());
child.starts_with(parent)
```

Current body (lines 331-332):
```rust
let child = canonicalize_with_existing_prefix(child);
child.starts_with(parent)
```

Independent confirmation: `grep -n "unwrap_or_else"` returns **0 matches** in the file. The old fallback is gone.

**canonicalize_with_existing_prefix() — algorithm verified correct:**

Lines 336-362: iterates `path.ancestors()` from path down to root. For each ancestor, if `ancestor.exists()`, calls `ancestor.canonicalize()`. On success (`if let Ok(canonical_base)`), counts the stripped tail components (`i` = iteration depth = tail count), collects them from `path.components()` and folds them back via `Path::join()` (component operations, not string concatenation). Returns `path.to_path_buf()` only in the degenerate case. No `.unwrap()`, no `.expect()`, uses `if let Ok(...)` per the clippy lint fix documented in 60-03-SUMMARY.md. Correct.

**WR-01: symmetric inside-~/.claude check — FIXED**

`cwd_covers_home_claude_state()` lines 301-314: after the `path_covers(cwd, &home_claude)` check at line 303, line 306 adds `if cwd.starts_with(&home_claude) { return true; }`. This catches a CWD nested inside `~/.claude` (e.g., `~/.claude/projects/foo`). Independent confirmation: `grep -n "cwd.starts_with"` returns **1 match** at line 306.

**WR-02: home_dir() None fails closed — FIXED**

`cwd_self_disable_risk_reason_for()` lines 279-298: replaced `if let Some(home) = home_dir().and_then(...)` with an explicit match at lines 282-287. When `home_dir()` returns `None`, line 284 returns `Ok(Some(self_disable_reason_no_home()))` — an explicit deny. `canonical_existing_path(&path)?` at line 286 propagates Err upward on canonicalization failure (fail closed). No `.ok()` swallowing. Independent confirmation: `grep -n "canonical_existing_path.*\.ok()"` returns **0 matches**.

`self_disable_reason_no_home()` at lines 325-327 returns the denial message: "refusing to wrap Bash: home directory cannot be resolved; cannot verify CWD does not cover Claude Code hook state". Independent confirmation: `grep -n "self_disable_reason_no_home"` returns **2 matches** (definition at line 325, call at line 284). Correct.

### Level 3 — Wired

`path_covers()` is called from `cwd_covers_home_claude_state()` at lines 303 and 312. `cwd_covers_home_claude_state()` is called from `cwd_self_disable_risk_reason_for()` at line 288. `cwd_self_disable_risk_reason_for()` is called from `cwd_self_disable_risk_reason()` at line 275. `cwd_self_disable_risk_reason()` is called at lines 62, 102, 150, and 204 (all three file-op arms plus the Bash arm entry). The full chain is wired end-to-end.

### Level 4 — Regression Tests Present and Correct

**Test 1: `windows_cwd_guard_denies_home_claude_ancestor_absent_file` (lines 920-935)**

- `#[cfg(target_os = "windows")]` gated: YES (line 920)
- Creates ONLY the home directory — no `.claude.json`, no `.claude/`, no children (line 927)
- Asserts `cwd_covers_home_claude_state(&cwd, &home.canonicalize()?)` returns `true` with message "guard must fire when CWD is home dir and .claude.json does not yet exist on disk" (line 931)
- This is the EXACT CR-01 scenario: the file is absent from disk. The prior test `windows_cwd_guard_denies_home_claude_ancestor` at line 683 called `std::fs::write(home.join(".claude.json"), "{}")` before testing — this new test does NOT, specifically exercising the previously-fail-open path.
- No `set_current_dir`, no env var modification. Uses `?` propagation with `Box<dyn Error>` return type.
- STATUS: VERIFIED

**Test 2: `windows_cwd_guard_denies_inside_home_claude` (lines 940-957)**

- `#[cfg(target_os = "windows")]` gated: YES (line 939)
- Creates `home/.claude/projects/myrepo` via `create_dir_all` (line 944)
- Asserts `cwd_covers_home_claude_state(&cwd, &canonical_home)` returns `true` with message "guard must fire when CWD is nested inside ~/.claude subtree" (line 952)
- Exercises the WR-01 symmetric `cwd.starts_with(&home_claude)` check.
- No `set_current_dir`, no env var modification.
- STATUS: VERIFIED

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono-cli/src/claude_code_hook.rs` | path_covers() fixed + canonicalize_with_existing_prefix() + symmetric guard check + home-dir fail-closed + 2 regression tests | VERIFIED | 959-line file. All 4 production function changes confirmed. Both regression tests confirmed Windows-gated and structurally correct. 0 unwrap/expect in production code paths. |
| `packages/claude-code/CLAUDE.md` | PowerShell steering note + tool-support matrix | VERIFIED (no regression) | Unchanged from Plan 60-02. Content confirmed in initial verification. |
| `packages/claude-code/claude-code-tools-windows-runner.profile.json` | Runner profile: workdir.access==readwrite, network.block==true | VERIFIED (no regression) | Unchanged. Confirmed in initial verification. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `path_covers()` | `canonicalize_with_existing_prefix()` | replaces `unwrap_or_else(|_| child.to_path_buf())` | WIRED | Line 331: `let child = canonicalize_with_existing_prefix(child);` — confirmed. |
| `cwd_covers_home_claude_state()` | `cwd.starts_with(&home_claude)` | symmetric check at line 306 | WIRED | Line 306 confirmed present. |
| `cwd_self_disable_risk_reason_for()` | `Ok(Some(self_disable_reason_no_home()))` | None branch at line 284 | WIRED | Lines 282-287 confirmed. |
| Write/Edit/MultiEdit arms | `cwd_self_disable_risk_reason()` | first call in each Windows-gated arm | WIRED | Lines 62, 102, 150, 204 confirmed unchanged. |

---

### Data-Flow Trace (Level 4)

| Artifact | Guard Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `path_covers()` | `child` (PathBuf) | `canonicalize_with_existing_prefix(child)` | Yes — walks real filesystem ancestors, canonicalizes, re-appends tail | FLOWING |
| `cwd_covers_home_claude_state()` | `home_claude` | `home.join(".claude")` where home is canonical | Yes — real Path component join | FLOWING |
| `cwd_self_disable_risk_reason_for()` | `home` | `canonical_existing_path(&path)?` on real home dir | Yes — real canonicalize call; None path now explicit deny | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Evidence | Status |
|----------|----------|--------|
| `cargo test --bin nono claude_code_hook` exits 0 with 16 tests (orchestrator-reported) | 60-03-SUMMARY.md: "16 tests PASS (14 prior + 2 new)" | VERIFIED (trust supported by source: both test functions structurally complete, correct assertions, no missing await/error path) |
| `canonicalize_with_existing_prefix` called in `path_covers()` | Line 331 confirmed | VERIFIED |
| `unwrap_or_else` removed from `path_covers()` | grep returns 0 matches in entire file | VERIFIED |
| `cwd.starts_with` present in `cwd_covers_home_claude_state()` | Line 306 confirmed | VERIFIED |
| `self_disable_reason_no_home` definition + call site | Lines 325 and 284 confirmed | VERIFIED |
| Both regression tests are `#[cfg(target_os = "windows")]` gated | Lines 920, 939 confirmed | VERIFIED |
| No `.unwrap()` or `.expect()` in production code | grep returns 0 matches | VERIFIED |
| `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used` PASS (orchestrator-reported) | 60-03-SUMMARY.md: "PASS (caught and fixed if let Some / .ok() pattern)" | VERIFIED (fix visible at line 340: `if let Ok(canonical_base) = ancestor.canonicalize()`) |
| Cross-target clippy (x86_64-unknown-linux-gnu) | PARTIAL — Linux cross-toolchain not installed on Windows dev host; deferred to CI. All new functions are `#[cfg(target_os = "windows")]` gated so Linux/macOS never compile them. | PARTIAL (expected, within policy) |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REQ-STW-01 (v2.9 track) | 60-01-PLAN.md, 60-03-PLAN.md | Confined file ops: Write/Edit/MultiEdit execute as Low-IL nono-confined file operations scoped to project CWD; out-of-scope writes denied at OS boundary | NEEDS HUMAN | Mechanism is present and SC 3 confinement invariant is now closed. HUMAN-UAT items (in-CWD edit lands, OS boundary deny) remain. |
| REQ-STW-02 (v2.9 track) | 60-02-PLAN.md | Usable shell story: PowerShell-backed Bash runner + Claude steered to emit PS syntax automatically | NEEDS HUMAN | CLAUDE.md section present and correct. Behavioral confirmation (model uses PS syntax unprompted) requires live agent run. |

---

### Anti-Patterns Found

#### In Plan 60-03 scope (claude_code_hook.rs modified functions)

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None | — | No TBD/FIXME/XXX/unwrap/expect in new or modified production code | — | CLEAR |

#### Pre-existing (NOT caused by Phase 60 — do not attribute to this phase)

The following are pre-existing repo conditions present at parent commit `1418c12c`, unchanged by Phase 60:

| Issue | Scope | Attribution |
|-------|-------|-------------|
| `cargo clippy --workspace --all-targets` fails on dead-code warnings (`profile/mod.rs`: `oauth2_cred_builder` etc.) | nono-cli | Pre-existing; present before Plan 60-03 |
| `cargo clippy --workspace --all-targets` fails on `unwrap` in test `offline_verify_extended_trust_bundle` | nono (test) | Pre-existing; test modules are exempt from `clippy::unwrap_used` policy, but workspace-wide `--all-targets` still surfaces it |
| `cargo fmt --check` fails on `capability.rs`, `error.rs`, `sandbox/state.rs`, `supervisor/*.rs` | nono | Pre-existing fmt drift; unrelated to `claude_code_hook.rs` which is rustfmt-clean |

These pre-existing failures do NOT block the Phase 60 verdict. `claude_code_hook.rs` is clean under both `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used` and `cargo fmt`.

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

### 5. End-to-end POC UAT — read, edit, run task on Win11 (SC 4)

**Test:** A POC user completes a small read → edit → run task on a Win11 host with the experimental profile loaded.
**Expected:** Edits are confined (land via Low-IL path), an out-of-scope write is denied, shell commands work with PS syntax.
**Why human:** Full E2E requires Win11 host + Claude Code TUI + experimental profile loaded; cannot be simulated.

---

## Gap Closure Confirmation

### CR-01 — CLOSED

The prior BLOCKER `path_covers() fails open when child path does not exist on disk` is confirmed closed by independent code inspection:

1. `unwrap_or_else(|_| child.to_path_buf())` is GONE — grep returns 0 matches
2. `canonicalize_with_existing_prefix()` is PRESENT — definition at line 336, call site at line 331
3. The algorithm correctly walks ancestors, canonicalizes the deepest existing one, and re-appends the tail using `Path` component operations — no string manipulation, no panic paths
4. `cwd.starts_with(&home_claude)` symmetric check is PRESENT at line 306 (WR-01)
5. `self_disable_reason_no_home()` is PRESENT at lines 325-327, called in the `None` branch at line 284 (WR-02)
6. `windows_cwd_guard_denies_home_claude_ancestor_absent_file` test exists at lines 920-935, is Windows-gated, does NOT pre-create `.claude.json`, and asserts the guard fires — the exact CR-01 scenario
7. `windows_cwd_guard_denies_inside_home_claude` test exists at lines 940-957, is Windows-gated, and asserts the guard fires when CWD is inside `~/.claude` — the WR-01 scenario
8. 16 tests pass per orchestrator (14 prior + 2 new); this is consistent with the complete test function structures observed in the source

The CR-01 BLOCKER is CLOSED. SC 3 is now VERIFIED.

---

## Summary

Phase 60 has no remaining code-verifiable gaps. The single BLOCKER from the initial verification (CR-01: path_covers() fail-open on non-existent child paths) is confirmed closed by Plan 60-03. All three code-verifiable must-haves pass:

- SC 1 (code portion): deny+additionalContext mechanism for Write/Edit/MultiEdit is substantive and wired
- SC 2 (code portion): CLAUDE.md PowerShell steering note is present and correct
- SC 3: CWD self-disable guard is now fail-secure — no unwrap_or_else fallback, symmetric inside-~/.claude check, home-dir None fails closed — with regression tests covering both the absent-file and nested-inside-~/.claude paths

The five remaining human verification items (SC 1 OS boundary, SC 2 behavioral, A1 assumption, SC 4 E2E UAT) are unchanged from the initial verification and are not code defects — they require a live Win11 host with the Claude Code TUI and experimental profile.

Pre-existing `make ci` failures (workspace clippy dead-code, test unwrap, fmt drift in unrelated files) are out of scope: they are identical before and after Plan 60-03 and do not touch `claude_code_hook.rs`.

**Status: human_needed** — all code gates pass; awaiting live Win11 UAT.

---

_Verified: 2026-05-29_
_Verifier: Claude (gsd-verifier)_
_Re-verification: Yes — initial was gaps_found (CR-01 BLOCKER); re-verification confirms BLOCKER closed_
