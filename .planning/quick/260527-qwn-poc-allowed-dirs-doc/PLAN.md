---
quick_id: 260527-qwn
slug: poc-allowed-dirs-doc
date: 2026-05-27
---

# Quick Task: Document "Adding allowed directories" in the POC handoff

## Description

Add a short, operator-focused `## Adding allowed directories` section to
`docs/cli/development/windows-poc-handoff.mdx` documenting the two ways a POC operator
grants extra directories: per-run CLI flags, and a persistent custom user profile.

## Why

POC users repeatedly need to widen the allow-list beyond the `claude-code` profile defaults
(e.g. their work/project dirs). The two mechanisms exist but aren't documented in the handoff;
this captures both with verified flags/schema so operators don't have to ask.

## Tasks

1. Insert `## Adding allowed directories` into `windows-poc-handoff.mdx` between the
   "Windows `nono run` — heavy-runtime children" section (ends ~L638) and
   "## Step 6 — POC user handoff content" (L640). Content (verified this session):
   - **Method 1 — per-run flags** (cli.rs:1509-1538): `-a/--allow` (rw, recursive, `NONO_ALLOW`
     env, comma-list), `-r/--read` (ro), `-w/--write` (wo), `--allow-file/--read-file/--write-file`,
     `--allow-cwd`. Example + "paths must already exist" note.
   - **Method 2 — persistent user profile** at `%APPDATA%\nono\profiles\<name>.json` that
     `extends` claude-code and adds `filesystem.allow`/`read` (schema mirrors policy.json's
     claude-code profile). Env-var expansion ($HOME→%USERPROFILE%, $XDG_CONFIG_HOME→%APPDATA%);
     just a config file (no rebuild/re-sign).
   - Footnotes: deny rules still win → `--allow-deny-override`; verify with `nono why`.

## Verification

- Section renders: heading present, both code fences well-formed, links resolve.
- Facts match source (flags cli.rs:1509-1538/1576; profile schema policy.json claude-code).

## Notes

- `docs/cli/development/` gitignored-but-tracked → stage with `git add -f`
  ([[feedback_docs_cli_dev_gitignored]]).
- Doc-only; cross-target clippy N/A. Bash MSYS fork errors → retry once.
