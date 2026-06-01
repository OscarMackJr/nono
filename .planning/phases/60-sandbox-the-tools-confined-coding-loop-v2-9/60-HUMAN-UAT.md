---
phase: 60-sandbox-the-tools-confined-coding-loop-v2-9
doc: live-human-uat-results
host: Win11 (operator), Claude Code v2.1.159, nono 0.57.5 (unreleased Phase 60 build)
started: 2026-06-01
status: blocked-setup-conflict
---

# Phase 60 — Live Human UAT Results

Tracks the 5 human-verification items from `60-VERIFICATION.md` (status `human_needed`).
Build under test: local `main` rebuild, `nono 0.57.5` (contains unreleased Phase 60 confined
Write/Edit/MultiEdit arms + CR-01 `path_covers` fix). Runbook: `C:\temp\nono-phase60-uat-cookbook.md`.

## Results matrix

| # | UAT item | SC | Result | Notes |
|---|----------|----|--------|-------|
| 1 | Confined edit lands | SC 1 | ⛔ BLOCKED (setup conflict) | self-disable guard fired — see finding F-60-UAT-01 |
| 2 | Out-of-scope write denied at OS boundary | SC 1 | ⏸ not reached | blocked by item 1 |
| 3 | deny+additionalContext → Bash retry (A1) | — | ⏸ not reached | blocked by item 1 |
| 4 | PowerShell steering unprompted | SC 2 | ⏸ not reached | blocked by item 1 |
| 5 | E2E read→edit→run | SC 4 | ⏸ not reached | blocked by item 1 |

## F-60-UAT-01 — Self-disable guard fires on the documented working dir (setup conflict)

**Observed (2026-06-01).** From `C:\Users\omack\nono-poc` (the cookbook's "supported" dir), the
first `Write test.txt` was denied, and the Bash fallback was denied with the **same** reason:

```
refusing to wrap Bash: CWD '\\?\C:\Users\omack\nono-poc' covers Claude Code hook settings
or agent state; would allow the tool jail to disable its own hooks
```

Claude correctly recognized this as a deliberate safety guard (not a path-permission denial),
stopped after the deny without retry-spamming, and asked the operator how to proceed.

**Root cause — `crates/nono-cli/src/claude_code_hook.rs` lines 292–295:**

```rust
let project_claude = cwd.join(".claude");
if project_claude.exists() {
    return Ok(Some(self_disable_reason(&cwd)));
}
```

The guard refuses to wrap any tool call when the CWD contains a `.claude` subdirectory. The
UAT setup (cookbook §0 / bundle README step 3) wires the hook via
`Copy-Item ... "$proj\.claude\settings.json"` — i.e. it creates `nono-poc\.claude` **inside the
CWD that the jail would be granted write to**. A confined write to the CWD could therefore
overwrite the hook config and disable the very confinement under test, so the guard fires. The
guard is **working as designed**; the *documented setup is self-defeating*.

This is NOT the home `~/.claude` branch (`nono-poc` is a sibling of `~/.claude`, so
`cwd_covers_home_claude_state()` returns false) — it is specifically the project-local `.claude`
branch at line 293.

**Why it matters (design finding, exceeds /gsd-fast).** The guard is *CWD-coarse*: it disables
confinement for the entire CWD whenever **any** `.claude` dir exists directly under it. But
project-scoped hook registration (`<project>/.claude/settings.json`) is the **normal** way to
install a Claude Code hook. As written, sandbox-the-tools confinement can never engage in a real
project that registers the hook project-locally. The blanket CWD-level refusal could plausibly be
narrowed to: refuse only if the *specific write target* is within `.claude`, or only if the
granted scope provably includes `.claude`. That is a security-sensitive decision (don't weaken the
self-disable invariant) and needs a proper plan/debug pass — captured here as a follow-on, not
fixed inline.

**Operator workarounds — tried (2026-06-01):**

- **(B) Launch from a subdirectory — FAILED.** Kept `nono-poc\.claude\settings.json`, launched
  `claude` from `nono-poc\work`. `/hooks` showed **no** nono PreToolUse `*` entry. Confirmed via
  claude-code-guide: **Claude Code does NOT walk up the directory tree for project settings** — it
  reads `.claude/settings.json` relative to the launch CWD / git root only. So the only place the
  hook reliably loads (`CWD\.claude`) is exactly the place the guard refuses. Project scope is a
  hard dead end with the current guard.
- **(A) User-scope in real `~\.claude\settings.json` — REJECTED (unsafe here).** That file is the
  operator's live GSD harness (applies to every Claude Code session for the user, incl. the
  assistant session helping run this UAT). A `matcher:"*"` deny-by-default nono hook there would
  break normal Claude Code usage. Not viable on this box.
- **(C) `CLAUDE_CONFIG_DIR` relocation — WORKING PATH.** Point `CLAUDE_CONFIG_DIR` at an isolated
  dir (e.g. `C:\temp\nono-uat-cfg`) holding a minimal `settings.json` with ONLY the nono PreToolUse
  hook; copy `~\.claude\.credentials.json` into it to skip re-login; remove `nono-poc\.claude`; and
  launch `claude` from `nono-poc`. The self-disable guard checks the *real* `~\.claude` (resolved
  via `home_dir()`, NOT `CLAUDE_CONFIG_DIR`), which is a sibling of `nono-poc`, so it passes; the
  hook loads from the relocated config; and the operator's real `~\.claude` is never modified, so
  the assistant session is unaffected. NOTE: `.claude/settings.local.json` does NOT work — it still
  creates `CWD\.claude`, re-tripping the guard.

The CLAUDE_CONFIG_DIR requirement reinforces F-60-UAT-01: with the guard as written, the only way
to exercise confinement is a relocated/isolated user config, never project-scoped hooks.

## Next
- Operator re-runs UAT 1–5 using workaround (A) or (B).
- Decide disposition of the CWD-coarse self-disable guard (narrow vs. document project-scope as
  unsupported) — needs `/gsd:debug` or a scoped plan, not a fast edit.
