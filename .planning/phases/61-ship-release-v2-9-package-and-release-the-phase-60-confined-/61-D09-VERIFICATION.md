# D-09 Verification Record — deny-`~/.claude` Hook-Layer Guard

**Plan:** 61-02
**Date:** 2026-06-03
**Build:** v0.58.0 (current `main`)
**Status:** CLOSED — guard present, tests pass, scope documented

---

## 1. What Was Verified

The D-09 pre-ship security blocker concerns the runner profile's ability to protect Claude Code
hook settings (`~/.claude/`, `~/.claude.json`, project `.claude/`) from a confined tool call
that could use `--allow-cwd` to re-grant write access and thereby disable its own hook.

### Enforcement Location

The guard is implemented in `crates/nono-cli/src/claude_code_hook.rs`:

| Symbol | Line | Role |
|--------|------|------|
| `pre_tool_use_response` / Bash arm | :204 | Calls `cwd_self_disable_risk_reason()?` before emitting any `--allow-cwd` runner command; returns `deny` immediately if the check fires |
| `cwd_self_disable_risk_reason()` | :272 | Resolves `current_dir()` and delegates to `cwd_self_disable_risk_reason_for()` |
| `cwd_self_disable_risk_reason_for(cwd)` | :279 | Canonicalizes CWD and home; checks `cwd_covers_home_claude_state` + project `.claude/` child |
| `cwd_covers_home_claude_state(cwd, home)` | :301 | Calls `path_covers(cwd, home/.claude)`, checks `cwd.starts_with(home_claude)`, and checks `home/.claude.json[.lock]` |
| `path_covers(parent, child)` | :330 | Uses `canonicalize_with_existing_prefix` then `child.starts_with(parent)` — **path-component comparison, not string `starts_with`** — closing the CR-01 fail-open |

The guard was implemented in **Phase 60-03** (commits `ddb711dc`, `fe832dfc`, `309c94a4`).

---

## 2. Test Results on the 0.58.0 Build

Command run (via Bash tool on Windows host):

```
cargo test -p nono-cli --bin nono -- claude_code_hook
```

Result:

```
running 16 tests
test claude_code_hook::tests::non_pre_tool_use_event_is_silent ... ok
test claude_code_hook::tests::pre_tool_use_edit_returns_deny_with_ps_cmd ... ok
test claude_code_hook::tests::pre_tool_use_multiedit_returns_deny_with_ps_cmd ... ok
test claude_code_hook::tests::pre_tool_use_notebookedit_deny_no_ps_cmd ... ok
test claude_code_hook::tests::pre_tool_use_read_only_tools_allow ... ok
test claude_code_hook::tests::pre_tool_use_write_content_with_special_chars ... ok
test claude_code_hook::tests::pre_tool_use_file_tools_deny ... ok
test claude_code_hook::tests::pre_tool_use_bash_rewrites_command ... ok
test claude_code_hook::tests::pre_tool_use_write_returns_deny_with_ps_cmd ... ok
test claude_code_hook::tests::windows_wrapper_uses_native_powershell_child ... ok
test claude_code_hook::tests::windows_cwd_guard_denies_home_claude_ancestor_absent_file ... ok
test claude_code_hook::tests::windows_cwd_guard_denies_project_claude_child ... ok
test claude_code_hook::tests::windows_write_arm_cwd_guard_fires_before_ps_cmd ... ok
test claude_code_hook::tests::windows_cwd_guard_denies_inside_home_claude ... ok
test claude_code_hook::tests::windows_cwd_guard_denies_home_claude_ancestor ... ok
test claude_code_hook::tests::windows_cwd_guard_uses_path_components ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 1124 filtered out; finished in 0.04s
```

**All 16 hook tests pass.** Key coverage:

- `windows_cwd_guard_denies_home_claude_ancestor` — home CWD that is an ancestor of `~/.claude` → denied
- `windows_cwd_guard_uses_path_components` — `.claudefoo` does NOT falsely match (path-component boundary)
- `windows_cwd_guard_denies_project_claude_child` — repo CWD with a project-local `.claude/` child → denied
- `windows_cwd_guard_denies_inside_home_claude` — CWD inside `~/.claude` → denied
- `windows_cwd_guard_denies_home_claude_ancestor_absent_file` — absent `~/.claude` path (not yet created) → denied
- `pre_tool_use_file_tools_deny` — non-Bash file tools deny
- `windows_write_arm_cwd_guard_fires_before_ps_cmd` — the guard fires in the Write arm too, before any PowerShell steering command is emitted

D-09 is **proven closed** on the release build.

---

## 3. Honest Scope: Hook-Layer Enforcement

The deny is a **hook-layer fail-closed guard**, NOT an OS-level deny-within-allow.

### Why the OS Label Backend Cannot Enforce This on Windows

The Windows sandbox backend has no deny-within-allow primitive:

- `add_deny_access_rules` in `policy.rs` emits enforcement only under `cfg!(target_os = "macos")`
  (line 674); on Windows it adds nothing to the `CapabilitySet`.
- There is no `Deny` variant in `AccessMode` (`capability.rs:50–57`); the Windows label backend
  (`sandbox/windows.rs`) only ever applies granted caps via mandatory-label / DACL grants.
- `remove_exact_file_caps_for_paths` retains all **directory** caps (capability.rs:1311–1316).
- `validate_deny_overlaps` is a Windows no-op (`policy.rs:1045`).

Result: a `--allow-cwd ReadWrite` directory grant for `~/.claude` survives on Windows regardless
of any `add_deny_access` entries in the runner profile. The profile entries remain valuable on
macOS (where `validate_deny_overlaps` enforces them) but are inert on Windows for the
allow-overlap case.

### The Actual v2.9 Enforcement Boundary

The load-bearing protection is the **hook-level CWD guard** at `claude_code_hook.rs:204`.
This guard fires when Claude Code is the launcher and calls the PreToolUse hook before emitting
any `nono run --allow-cwd` invocation. The sequence is:

1. Claude Code calls the hook → `nono hook` runs.
2. Hook resolves CWD and checks `cwd_covers_home_claude_state` using `path_covers`
   (component-safe comparison, not string `starts_with`).
3. If CWD covers `~/.claude` (or is inside it, or a project `.claude/` exists) → hook returns
   `permissionDecision: deny` with an explanatory reason string.
4. The `nono run --allow-cwd` runner command is **never emitted** — the Low-IL child is never
   spawned from a dangerous CWD.

This is fail-closed: if home cannot be resolved, the guard denies (see
`self_disable_reason_no_home()`).

---

## 4. Documented Limitation — Bare-CLI Gap

**The hooked-loop boundary is THE documented v2.9 boundary. The bare-CLI path is a documented
limitation, not a Phase 61 code task.**

A user or script invoking:

```
nono run --profile claude-code-tools-windows-runner --allow-cwd -- <cmd>
```

directly (outside the Claude Code hooked loop) bypasses the hook entirely. The hook-level guard
does not fire; no deny-within-allow OS primitive is available on Windows; the runner child
receives a ReadWrite grant to whatever `--allow-cwd` resolves.

This gap is:

- **Consistent** with the Phase 60 "defense-in-depth, not full isolation" verdict.
- **Accepted** as T-61-04 in the 61-02 threat model (`disposition: accept`).
- **Documented here** and in the release notes as a scope limitation.
- **Not a Phase 61 code task** — addressing it would require either (a) a Windows deny-within-allow
  OS primitive (unavailable) or (b) a kernel-level minifilter driver (a v3.0 deferral, Gap 6b).

The Phase 62 WFP kernel enforcement adds network-level blocking for AppContainer-spawned children
but does not change the filesystem-write surface for the bare-CLI path.

---

## 5. Release-Notes Paragraph (ready to paste into 61-04)

> **Security: hook-layer `~/.claude` self-disable guard (Windows)**
>
> v2.9 closes a residual attack surface in the Windows confined tool-mediation loop (Phase 60):
> the PreToolUse hook now refuses to wrap `Bash` whenever the launch CWD covers `~/.claude`,
> `~/.claude.json[.lock]`, or a project-local `.claude/` directory. The guard fires before any
> `nono run --allow-cwd` invocation is emitted, using path-component comparison (not string
> prefix matching) to prevent false-negatives on names like `.claudefoo`. This closes the
> self-disable vector where a confined `Bash` tool call could rewrite `~/.claude/settings.json`
> to remove the PreToolUse hook. **Scope note:** this protection is a hook-layer boundary that
> fires when Claude Code is the launcher. A direct `nono run --allow-cwd ~/.claude` invocation
> outside the hooked Claude loop is outside this boundary — the Windows OS label backend has no
> deny-within-allow primitive for the overlap case, and the bare-CLI gap is a documented
> limitation consistent with the Phase 60 "defense-in-depth, not full isolation" verdict.

---

## 6. Key Commits (Phase 60-03)

| Commit | Description |
|--------|-------------|
| `ddb711dc` | hook: add `cwd_self_disable_risk_reason` CWD guard (initial implementation) |
| `fe832dfc` | hook: fix `path_covers` CR-01 fail-open via `canonicalize_with_existing_prefix` |
| `309c94a4` | hook: add unit tests for `cwd_covers_home_claude_state` and component-boundary edge cases |

---

*Created by plan 61-02 execution on 2026-06-03.*
*See also: `.planning/todos/done/2026-05-29-claude-tools-runner-deny-dotclaude-regardless-of-cwd.md`*
