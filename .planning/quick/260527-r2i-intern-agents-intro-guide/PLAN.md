---
quick_id: 260527-r2i
slug: intern-agents-intro-guide
date: 2026-05-27
---

# Quick Task: Intern intro guide — running AI agents with nono

## Description

Write a jovial, college-intern-facing introduction to running AI agents inside nono on a
Windows desktop that Desktop Support has already provisioned. Cover: what nono does, how the
default config works, how to extend it, and how to give feedback to development. Lean heavily
on existing docs (link out, don't duplicate).

## Why

Zt-Infra is rolling nono out to intern users. They need a friendly on-ramp that assumes the
install is done and encourages fearless experimentation (the whole point of a sandbox), while
pointing at the existing operator/POC docs for the gritty mechanics.

## Tasks

1. New `docs/cli/development/nono-agents-intern-guide.mdx` — sections:
   - Welcome / what nono is (sandbox = playground with guardrails; jovial).
   - 30-second quickstart (`nono run --profile claude-code -- claude --version`, then `--allow-cwd -- claude`).
   - How the default `claude-code` setup works (granted dirs, net allowed, the capability banner
     is the contract, the WARN lines are normal).
   - Reading the banner + `nono why`.
   - Extending config (SHORT recap of the two methods; link to the "Adding allowed directories" section).
   - Experiment! (safely) — worst case is "access denied", not "deleted prod".
   - Feedback to development — what to capture (`nono audit`, banner, exact cmd) + where
     (internal Zt-Infra contact first [placeholder], then upstream GitHub + Discord; honor the
     README "don't dump LLM security issues / ask in Discord first" caveat).
   - Cheat sheet + where-to-next links.

2. Leverage existing docs via relative links (confirmed anchors):
   - `./windows-poc-handoff` (install/run companion)
   - `./windows-poc-handoff#adding-allowed-directories`
   - `./windows-poc-handoff#working-directory-choice-windows`
   - `./windows-poc-handoff#known-limitation-nono-run-cannot-host-tui-agents-on-windows`

## Verification

- Markdown well-formed (frontmatter, headings, fenced code, table); relative links use
  confirmed anchors; facts match this session's verified content (claude-code defaults, the two
  config-extension methods, README feedback channels).

## Notes

- `docs/cli/development/` gitignored-but-tracked → `git add -f` ([[feedback_docs_cli_dev_gitignored]]).
- Feedback section's INTERNAL Zt-Infra intake is a clearly-marked placeholder for the team to fill;
  the upstream channels (GitHub always-further/nono, Discord discord.gg/pPcjYzGvbS) are real (README).
- Doc-only; cross-target clippy N/A. Tone: jovial, pro-experimentation, intern-friendly.
