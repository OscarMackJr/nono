---
created: 2026-05-29
updated: 2026-05-29
title: Runner profile must deny ~/.claude (and project .claude/) regardless of --allow-cwd
area: tooling
files:
  - packages/claude-code/claude-code-tools-windows-runner.profile.json
  - crates/nono-cli/src/claude_code_hook.rs
relates:
  - PR #4 (Add experimental Windows Claude tool wrapping profile)
  - .planning/quick/260528-sch-spec-the-sandbox-the-tools-windows-tool-/260528-sch-SPEC.md
---

## Problem (residual #1 from PR #4 re-review)

PR #4's `claude-code-tools-windows-runner` profile correctly drops the `~/.claude`
write grant (it `extends: default`, not `claude-code`), closing the self-disable
vector where a confined `Bash` tool call rewrites `~/.claude/settings.json` to
remove the PreToolUse hook.

BUT the Bash rewrite invokes `nono run --profile claude-code-tools-windows-runner
--allow-cwd -- ...`. `--allow-cwd` grants r+w to the launch CWD. If `claude` is ever
launched with its CWD **at or above `~/.claude`** (e.g. `cd %USERPROFILE%\.claude;
claude` — which is exactly the profile-covered launch dir used elsewhere in this
project, see [[project_v27_opened]] / D-52-01), then `--allow-cwd` re-grants r+w to
`~/.claude`, and a single Bash tool call can rewrite `settings.json` and disable the
hook for all subsequent tool calls. Finding 2 reopens in that narrow case.

The same applies to a **project-local `.claude/settings.json`** under the CWD: Claude
Code merges project settings, so a confined Bash call could add/alter hook config
there if `--allow-cwd` covers the project `.claude/`.

## UPDATE 2026-05-29 — profile `add_deny_access` is a NO-OP on Windows for this case

PR #4 attempted to fix this by adding `policy.add_deny_access` for `$HOME/.claude`
and `$WORKDIR/.claude` to the runner profile (commit `7903fcf3`). **Verified in code
that this does NOT work on Windows for the `--allow-cwd` overlap case:**

- `add_deny_access_rules` emits enforcement only under `cfg!(target_os = "macos")`
  (`crates/nono-cli/src/policy.rs:674`); on Windows it adds nothing to the
  `CapabilitySet`.
- There is no `Deny` `AccessMode` (`crates/nono/src/capability.rs:50-57`), so no deny
  capability reaches the Windows label backend (`Sandbox::apply` / `windows.rs` only
  label *granted* caps).
- `remove_exact_file_caps_for_paths` retains all **directory** caps
  (`crates/nono/src/capability.rs:1311-1316`); `validate_deny_overlaps` is a Windows
  no-op (`policy.rs:1045`).
- ⇒ a `--allow-cwd` ReadWrite **directory** grant for `~/.claude` survives, and the
  Low-IL child can still write `~/.claude/settings.json`.

The PR's "Access is denied" probe most likely reflects a residual mandatory label on
the NonoDebug `~/.claude` (label-guard "skipping apply" on a pre-existing ACE), not
the new deny. Also note the **inaccurate doc comment** at `policy.rs:1038-1041` which
claims Windows deny-within-allow is "structurally enforceable" — it is not, for
profile `add_deny_access`. That comment should be corrected.

## Solution (revised — backend-independent)

The robust fix does NOT depend on label-backend deny semantics:

- In `crates/nono-cli/src/claude_code_hook.rs`, when the resolved CWD is at or under
  `~/.claude` (or otherwise covers the active `settings.json`), **refuse to wrap
  `Bash`** and return `permissionDecision: deny` with a clear reason — do not emit a
  `--allow-cwd` runner command that would grant `.claude` write.
- Keep the profile `add_deny_access` entries (still valuable on macOS and for
  standalone, non-overlapping deny paths) but do not rely on them on Windows for the
  CWD-overlap case.
- Fix the misleading `policy.rs:1038-1041` doc comment.
- Add a test asserting the hook denies `Bash` (or the runner cannot write
  `~/.claude/settings.json`) when CWD is `~/.claude`.

## Acceptance

- With `claude` launched from `%USERPROFILE%\.claude`, a wrapped Bash tool call that
  attempts to write `~/.claude/settings.json` is denied at the OS boundary (or the
  hook refuses to wrap and fails closed).
- The existing project-dir UAT (Bash works, outside-CWD write denied) still passes.

## Priority

Medium — closes the last residual enforceability edge in the PR #4 defense-in-depth
slice. Not blocking the experimental merge if documented as a known limitation, but
required before the runner profile is presented as a reliable boundary.
