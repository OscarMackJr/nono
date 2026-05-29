---
phase: 60-sandbox-the-tools-confined-coding-loop-v2-9
reviewed: 2026-05-29T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - crates/nono-cli/src/claude_code_hook.rs
  - packages/claude-code/CLAUDE.md
findings:
  critical: 1
  warning: 5
  info: 3
  total: 9
status: issues_found
---

# Phase 60: Code Review Report

**Reviewed:** 2026-05-29
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Reviewed the confined Write/Edit/MultiEdit arms added to the Claude Code PreToolUse
hook (`claude_code_hook.rs`) plus the PowerShell steering docs (`CLAUDE.md`).

The core PowerShell construction is sound: file-op arms emit a *bare inner* PS
expression, which the Bash arm later wraps exactly once via `-EncodedCommand`
(UTF-16LE base64), neutralizing inner metacharacters. Write content is base64-encoded
with the standard alphabet matching `[Convert]::FromBase64String`, and Edit/MultiEdit
use literal `String.Replace()` rather than `-replace` regex — both correct anti-injection
choices.

The principal concern is the CWD self-disable guard, which is the only thing standing
between the `--allow-cwd` grant and a hook-disabling write to `~/.claude/settings.json`
(the profile-level `add_deny_access` is a documented Windows no-op — see the pending
todo `2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`). That guard
**fails open** when its target paths do not yet exist on disk, which is a security
regression in a fail-secure codebase.

## Critical Issues

### CR-01: CWD self-disable guard fails open when `.claude.json` state files do not exist

**File:** `crates/nono-cli/src/claude_code_hook.rs:334-337` (used by `cwd_covers_home_claude_state`, lines 313-323)
**Issue:**
`path_covers` canonicalizes the *parent* (the CWD, via `canonical_existing_path` at
line 296) but falls back to the **non-canonical** child path when the child cannot be
canonicalized:

```rust
fn path_covers(parent: &Path, child: &Path) -> bool {
    let child = canonical_existing_path(child).unwrap_or_else(|_| child.to_path_buf());
    child.starts_with(parent)
}
```

`canonicalize()` on Windows fails for any path that does not exist. The guard checks
`~/.claude.json` and `~/.claude.json.lock` (lines 319-322) — the per-session lock file
in particular is frequently absent (it only exists while a session holds it). When the
child does not exist, `child` falls back to the raw `home.join(".claude.json")` form
(e.g. `C:\Users\X\.claude.json`), but `parent` is the canonicalized CWD, which on
Windows is an extended-length path (`\\?\C:\Users\X`). `Path::starts_with` compares
components and `\\?\C:\Users\X` will **not** be a prefix of `C:\Users\X`, so the check
returns `false` and the guard reports "no risk" — i.e. it fails open exactly when the
state file is missing.

This defeats the `.claude.json` / `.claude.json.lock` limb of the self-disable guard,
which (per the pending todo and CLAUDE.md "Fail Secure") is the backend-independent
mitigation for the `--allow-cwd` re-grant of `~/.claude` write. A CWD whose only
coverage is the not-yet-created `.claude.json` would be wrapped instead of denied.

Note the asymmetry is the bug: the `.claude` *directory* limb usually passes because
the directory exists and canonicalizes, masking the defect in normal manual testing.

**Fix:** Make the guard fail **closed** and compare on canonicalized-or-lexically-
normalized paths consistently. Either (a) when the child cannot be canonicalized,
canonicalize its existing parent and append the final component, or (b) normalize the
CWD with the same scheme used for the child (do not mix `\\?\`-canonical parent with raw
child), or (c) on canonicalization failure of a *security-relevant target*, treat it as
covered (deny). For example, compare against the canonical home directory and use
component matching on the relative tail rather than canonicalizing the (possibly
missing) leaf:

```rust
fn path_covers(parent: &Path, child: &Path) -> bool {
    // Canonicalize the deepest existing ancestor of `child`, then re-append the
    // missing tail, so both sides use the same (extended-length) representation.
    let normalized_child = canonicalize_existing_prefix(child); // never raw fallback
    normalized_child.starts_with(parent)
}
```

Add a regression test covering a CWD that is an ancestor of `~/.claude.json` when that
file does **not** exist on disk (the current `windows_cwd_guard_denies_home_claude_ancestor`
test writes `.claude.json` first at line 666, so it never exercises the fail-open path).

## Warnings

### WR-01: Self-disable guard does not cover CWDs nested *inside* `~/.claude`

**File:** `crates/nono-cli/src/claude_code_hook.rs:313-323`
**Issue:**
`cwd_covers_home_claude_state` only catches the case where the CWD is an
*ancestor-or-equal* of `~/.claude` (`path_covers` evaluates `home_claude.starts_with(cwd)`).
It does not catch a CWD nested *inside* `~/.claude`, e.g. `~/.claude/projects/foo`. That
launch dir is explicitly the "profile-covered" location used elsewhere in this project
(D-52-01). While a deep nested CWD does not grant write to the sibling
`~/.claude/settings.json` via `--allow-cwd`, it *does* grant write to other agent state
under `~/.claude` (project session files, MCP config caches), which is still inside the
sensitive directory the guard is meant to protect. The guard's intent ("CWD covers Claude
Code hook settings or agent state") is broader than what it enforces.
**Fix:** Add a symmetric check: also deny when the canonicalized CWD *is under*
`home/.claude` (i.e. `cwd.starts_with(home_claude)`), not only when it is an ancestor.

### WR-02: `home_dir()` trusts unvalidated `USERPROFILE`/`HOME` environment variables

**File:** `crates/nono-cli/src/claude_code_hook.rs:349-354`
**Issue:**
The self-disable guard derives the sensitive directory from `USERPROFILE`/`HOME` env
vars with no validation. CLAUDE.md explicitly lists this as a footgun ("Validate
environment variables before use. Never assume `HOME`, `TMPDIR`, etc. are trustworthy").
Because this value gates a *deny* decision, an attacker (or a misconfigured launcher)
who controls `USERPROFILE` can point the guard at an irrelevant directory, causing
`cwd_covers_home_claude_state` to never match the real `~/.claude` — a fail-open path.
If both `USERPROFILE` and `HOME` are unset, `home_dir()` returns `None` and the entire
home-state limb is silently skipped (line 298 `if let Some(home) = ...`), again failing
open rather than denying.
**Fix:** When the home directory cannot be resolved, fail closed (deny) rather than
skipping the check, and prefer a validated/canonicalized home resolution.

### WR-03: Hook fail-secure posture is inconsistent — missing fields propagate `Err` instead of denying

**File:** `crates/nono-cli/src/claude_code_hook.rs:65-76, 107-134, 161-182, 224-238`
**Issue:**
The file-op arms `?`-propagate a `NonoError::HookInstall` when `tool_input` or a required
field (`file_path`, `old_string`, `new_string`, `edits`, `command`) is missing or the
wrong type. A propagated `Err` from `run()` bubbles to the process exit code; depending on
how Claude Code interprets a non-zero hook exit with no JSON on stdout, this may be treated
as "hook errored, allow the tool" rather than "deny". Compare this with the top of the
handler (lines 47-51), which correctly returns an explicit `deny_response` when `tool_name`
is absent. The fail-secure contract should be uniform: a malformed PreToolUse payload for a
write-class tool should produce an explicit `deny`, not an `Err` whose downstream handling
is unspecified.
**Fix:** Replace the `?`-on-missing-field paths in the Write/Edit/MultiEdit/Bash arms with
explicit `deny_response(...)` returns (matching the `tool_name` handling), so a malformed
event always fails closed with a defined decision. At minimum, document and verify Claude
Code's behavior on a hook that exits non-zero with empty stdout.

### WR-04: `command_contains_nono_wrapper` re-wrap guard is substring-based and can be evaded or misfire

**File:** `crates/nono-cli/src/claude_code_hook.rs:283-285, 240-242`
**Issue:**
The "already wrapped, skip re-wrapping" guard matches the literal substrings `" nono run "`
or a `"nono run "` prefix. This is brittle in both directions: (1) a benign user command
that merely *mentions* `nono run` (e.g. `echo " nono run "` or a Bash command documenting
usage) will be passed through **unwrapped** (line 241 returns `Ok(None)` = no rewrite =
runs unconfined at the Bash tool's normal privilege), defeating confinement for that call;
(2) the emitted wrapper from `windows_bash_wrapper_script` is `& 'nono.exe' run --profile ...`,
which does *not* contain the literal `" nono run "` (it's `'...nono.exe' run`), so the guard
would not actually recognize the hook's own Windows output as already-wrapped if it were
re-fed — the idempotency it claims is not guaranteed on Windows.
**Fix:** Use a structural signal instead of substring matching — e.g. detect the
`-EncodedCommand` trampoline shape the hook itself emits, or carry an explicit marker
(env var / sentinel flag) that the wrapper injects and the guard checks. Do not let an
arbitrary user-supplied substring suppress confinement.

### WR-05: `additionalContext` PS command relies on the model to copy a long base64 blob verbatim

**File:** `crates/nono-cli/src/claude_code_hook.rs:90, 144, 192, 414-422`
**Issue:**
For Write, the confined command embeds the entire file content as a base64 string inside
free-text `additionalContext` ("Use the Bash tool with this PowerShell command: {ps_inner}").
The security model depends on Claude reproducing that base64 blob *byte-for-byte* into a
subsequent Bash tool call. For large files this is a correctness/reliability hazard
(truncation, whitespace normalization, model paraphrasing) that can silently corrupt
written content — a data-integrity risk, not just UX. There is no integrity check (e.g.
expected length/hash) in the emitted PS expression to detect a mangled blob.
**Fix:** Consider writing content via a temp file the hook controls, or append a guard to
the PS expression that verifies decoded byte length before `WriteAllText` and fails loudly
on mismatch. At minimum, document the size limit/known-failure mode in CLAUDE.md.

## Info

### IN-01: Doc comment claims raw-byte base64 but Write path differs from Bash path subtly

**File:** `crates/nono-cli/src/claude_code_hook.rs:409-411, 414-415`
**Issue:** The comment states content is "base64-encoded (raw bytes, not UTF-16LE)",
which is accurate for `build_confined_write_cmd` (line 415 encodes `content.as_bytes()`),
but the surrounding wrap (`powershell_encoded_command`, line 399) encodes UTF-16LE. The two
distinct encodings (raw-byte for `FromBase64String`, UTF-16LE for `-EncodedCommand`) are
easy to conflate during maintenance. A one-line note at the call site clarifying the
double-encoding boundary would reduce future error risk.
**Fix:** Add a cross-reference comment at line 84/138/186 noting the inner blob is raw-byte
b64 and the outer wrap is UTF-16LE b64.

### IN-02: Non-Windows file-op arms silently discard extracted fields

**File:** `crates/nono-cli/src/claude_code_hook.rs:96, 150, 198`
**Issue:** On non-Windows targets the arms extract and validate `file_path`/`content`/etc.,
then discard them via `let _ = (...)` and return a static deny. The validation work is dead
on those targets. Harmless, but the `?`-propagation of missing fields (WR-03) still fires on
non-Windows, so the deny is not reached for malformed input even though the platform deny is
unconditional. Consider returning the platform deny *before* field extraction on non-Windows.
**Fix:** Move the `#[cfg(not(target_os = "windows"))]` static deny ahead of field parsing.

### IN-03: CLAUDE.md tool matrix overstates Write/Edit confinement guarantees

**File:** `packages/claude-code/CLAUDE.md:58`
**Issue:** The matrix labels Write/Edit/MultiEdit as "Confined (Low-IL via Bash retry) …
files inside CWD only". Per the pending todo and CR-01/WR-01, the "files inside CWD only"
guarantee is enforced by `--allow-cwd` + the self-disable guard, and that guard currently
has fail-open gaps. The doc presents a hard guarantee where the implementation provides
defense-in-depth with known edges. Align the doc with the "experimental / defense-in-depth,
not isolation" framing used elsewhere (PR #4 Q1 verdict).
**Fix:** Soften "files inside CWD only" to note it is best-effort confinement pending the
guard hardening tracked in the pending todo.

---

_Reviewed: 2026-05-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
