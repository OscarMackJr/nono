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

## Solution

Make the runner profile deny `~/.claude` (and the project-local `.claude/`) write
**unconditionally**, so the guarantee holds regardless of where `claude` is launched
from:

- Add an explicit deny of `$HOME/.claude` (+ `$HOME/.claude.json` family) to
  `claude-code-tools-windows-runner.profile.json`, evaluated AFTER `--allow-cwd` so
  the deny wins even when CWD == `~/.claude`. (On Windows the per-path
  mandatory-label model: confirm a deny can override a CWD grant for a nested path;
  if the backend can't express deny-within-allow on Windows, instead refuse to wrap
  Bash when the resolved CWD is at/above `~/.claude` and emit a fail-closed deny from
  `claude_code_hook.rs`.)
- Optionally also deny project-local `./.claude/` writes from the runner.
- Add a unit/integration test asserting the runner cannot write `~/.claude/settings.json`
  even with `--allow-cwd` pointed at `~/.claude`.

## Acceptance

- With `claude` launched from `%USERPROFILE%\.claude`, a wrapped Bash tool call that
  attempts to write `~/.claude/settings.json` is denied at the OS boundary (or the
  hook refuses to wrap and fails closed).
- The existing project-dir UAT (Bash works, outside-CWD write denied) still passes.

## Priority

Medium — closes the last residual enforceability edge in the PR #4 defense-in-depth
slice. Not blocking the experimental merge if documented as a known limitation, but
required before the runner profile is presented as a reliable boundary.
