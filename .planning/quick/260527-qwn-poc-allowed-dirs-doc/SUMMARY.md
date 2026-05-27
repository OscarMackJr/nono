---
quick_id: 260527-qwn
slug: poc-allowed-dirs-doc
date: 2026-05-27
status: complete
---

# Summary: "Adding allowed directories" POC handoff section

## What changed

- **`docs/cli/development/windows-poc-handoff.mdx`** — added a `## Adding allowed directories`
  section between the "Windows `nono run` — heavy-runtime children" section and
  "## Step 6 — POC user handoff content". Documents two operator methods:
  - **Method 1 — per-run flags:** a table of `-a/--allow` (rw), `-r/--read` (ro), `-w/--write`
    (wo), `--allow-file/--read-file/--write-file`, `--allow-cwd` + example + "paths must exist" note.
  - **Method 2 — persistent custom profile:** `%APPDATA%\nono\profiles\<name>.json` that
    `extends` claude-code and adds `filesystem.allow`/`read`, with env-var expansion and the
    "just a config file, no rebuild/re-sign" note.
  - Blockquote footnotes: deny rules still win → `--allow-deny-override`; verify with `nono why`.

## Verification

- Facts sourced from this session's reads: flags `crates/nono-cli/src/cli.rs:1509-1538` (+ 1576
  for `--allow-deny-override`); profile schema `crates/nono-cli/data/policy.json` claude-code
  `filesystem.allow`/`allow_file` block; user-profile dir confirmed by the live capabilities
  banner (`AppData\Roaming\nono\profiles`).
- Markdown well-formed: one `##` heading, two `###` subheads, table + json/powershell fences
  balanced, blockquote footnotes. Inserted cleanly before the existing `## Step 6` heading.

## Notes

- `docs/cli/development/` is gitignored-but-tracked → staged with `git add -f`
  ([[feedback_docs_cli_dev_gitignored]]).
- Doc-only; no Rust/cfg code → cross-target clippy gate N/A. Not pushed (local `main`).
