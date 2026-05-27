---
quick_id: 260527-r2i
slug: intern-agents-intro-guide
date: 2026-05-27
status: complete
---

# Summary: Intern intro guide — running AI agents with nono

## What changed

- **New `docs/cli/development/nono-agents-intern-guide.mdx`** — a jovial, college-intern-facing
  intro that assumes Desktop Support already installed nono. Sections:
  - **What nono does** — sandbox = playground with guardrails; OS-enforced (Job Objects / IL /
    WFP), least-privilege via profiles, transparent capability banner.
  - **30-second quickstart** — `nono run --profile claude-code -- claude --version`, then
    `--allow-cwd -- claude`; Windows cwd quirk + TUI-via-`nono shell` caveat (linked).
  - **How the default `claude-code` setup works** — granted dirs, net-on, banner-is-the-contract,
    the WARN lines are normal; `nono why` for spot checks.
  - **Extending the sandbox** — short recap of the two methods, links to the canonical
    "Adding allowed directories" section (no duplication).
  - **Experiment! (safely)** — concrete things to try; "denied = the feature working".
  - **Feedback** — what to capture (`nono audit list --recent 5`, banner, exact cmd) + where:
    internal Zt-Infra intake (placeholder) first, then upstream Discord/GitHub with the README's
    "ask in Discord first / don't auto-file LLM security issues" caveat honored.
  - **Cheat sheet** table + where-to-next links.

## Key decisions

- **Leveraged existing docs** (per user instruction): the guide links out to
  `windows-poc-handoff.mdx` (#working-directory-choice-windows, #adding-allowed-directories,
  the TUI-limitation note) and `nono run --help` rather than re-explaining mechanics. Kept thin.
- **Placement** in `docs/cli/development/` alongside the POC doc cluster so relative links resolve
  cleanly; staged with `git add -f` (gitignored-but-tracked, [[feedback_docs_cli_dev_gitignored]]).
- **Feedback channels** are real (README): Discord `discord.gg/pPcjYzGvbS`, GitHub
  `always-further/nono`. The INTERNAL Zt-Infra intake is a clearly-marked HTML-comment placeholder
  for the team to fill — flagged to the user.

## Verification

- Markdown well-formed: frontmatter, headings, fenced code blocks, one table, blockquotes all
  balanced; relative links use anchors confirmed present in windows-poc-handoff.mdx
  (Step 4 / Working directory choice / Known limitation TUI / Adding allowed directories).
- Facts match this session's verified content (claude-code profile defaults, the two
  config-extension methods, README feedback channels + alpha caveat).
- Fixed one typo (throwaiay→throwaway).

## Notes / follow-ups

- The internal Zt-Infra feedback intake line is a placeholder — the team must fill it in.
- Doc-only; cross-target clippy N/A. Not pushed (local `main`).
