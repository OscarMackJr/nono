---
quick_id: "260529-mb6"
slug: "rebuild-all-5-apps-rebuild-msi-installer"
date: "2026-05-29"
status: complete
---

# Quick Task 260529-mb6 — Rebuild apps + MSIs, create test profile + UAT cookbook

## What was done

All four deliverables completed inline by the orchestrator (build-heavy operational task; a
worktree would have forced a from-scratch recompile and risked Windows worktree fragility).

### 1. Rebuilt all 5 workspace apps (release, x86_64-pc-windows-msvc)
`cargo build --release --target x86_64-pc-windows-msvc` — exit 0, 3m40s. Compiled:
`nono` 0.57.5 (core lib), `nono-cli` 0.57.5 → **nono.exe** (relinked 16:10), `nono-proxy` 0.57.5,
`nono-shell-broker` 0.57.5 → **nono-shell-broker.exe**, `nono-ffi` 0.57.5 (+ the `sign-fixture`
dev tool). `nono.exe --version` → `nono 0.57.5`.

These binaries contain the **unreleased Phase 60** work: confined `Write`/`Edit`/`MultiEdit`
arms + the CR-01 `path_covers` fail-open fix (commits `ddb711dc`..`c4714a0c`). The Cargo version
is still 0.57.5, so the version string matches the tagged release even though the code is ahead.

### 2. Rebuilt MSI installers (per-user + machine, no WFP)
`scripts/build-windows-msi.ps1 -VersionTag v0.57.5-poc.1 ... -Scope {user|machine}` — both exit 0:
- `dist\windows\nono-v0.57.5-poc.1-x86_64-pc-windows-msvc-user.msi`
- `dist\windows\nono-v0.57.5-poc.1-x86_64-pc-windows-msvc-machine.msi`

**No WFP backend:** `nono-wfp-service` does not exist as a crate in this fork, so `-ServiceBinaryPath`
/ `-DriverBinaryPath` were omitted (the script's scope-coherence guard permits this when neither
is passed). The machine MSI installs nono.exe + broker machine-wide without network filtering;
`--block-net` / `--network-profile` / `--allow-domain` will fail closed on these builds.
**Both MSIs are UNSIGNED local dev builds** (`-poc.1` tag) — not for external distribution.

### 3. Test Claude Code profile reference bundle → C:\temp\nono-uat\
- `claude-code-tools-windows.profile.json` — the experimental nono profile (copy of `packages/claude-code/`)
- `nono-tool-hook.ps1` — the PreToolUse hook script (copy of `crates/nono-cli/data/hooks/`)
- `claude-code-settings.json` — reference Claude Code `settings.json` wiring the PreToolUse hook → script
- `CLAUDE.md` — the sandbox steering note (copy of `packages/claude-code/CLAUDE.md`)
- `README.md` — how the pieces fit + one-time setup (install profile to `%APPDATA%\nono\profiles\`, wire hook into project `.claude\`)

### 4. UAT cookbook → C:\temp\nono-phase60-uat-cookbook.md
Covers all 5 Phase 60 human-verification items from `60-VERIFICATION.md`, each with concrete
PowerShell steps, expected tool/hook behavior, pass/fail criteria, and a fill-in results matrix:
1. Confined file edit lands (SC 1)
2. Out-of-scope write denied at OS boundary (SC 1)
3. deny+additionalContext → Bash retry (A1) — with the documented fallback contingency
4. PowerShell steering unprompted (SC 2)
5. E2E read→edit→run (SC 4)
Plus the two critical setup gotchas: use a **supported working dir** (WRITE_OWNER — not a
drive-root subdir) and **do not run from inside/above `~/.claude`** (the Phase 60 CR-01 guard
fires there, by design), and a triage payload.

## Verification
- `nono.exe --version` → 0.57.5 ✓
- Both `nono-v0.57.5-poc.1-*.msi` exist under `dist\windows\` (5.44 MB each) ✓
- `C:\temp\nono-uat\` has 5 files (profile, hook, settings, CLAUDE.md, README) ✓
- `C:\temp\nono-phase60-uat-cookbook.md` written (all 5 UAT items + matrix) ✓

## Notes / caveats
- Binaries report 0.57.5 but are **ahead of** the tagged v0.57.5 release (unreleased Phase 60).
  The cookbook flags this so the UAT operator knows they're testing the right build.
- MSIs are unsigned; the dev-layout `target\...\release\nono.exe` skips the broker trust gate
  (fine for UAT). An installed unsigned MSI cannot spawn the broker — run from the dev layout,
  or sign locally via `scripts/sign-poc-local.ps1` for an installed-MSI test.
- No repo source changed — only this PLAN/SUMMARY pair is committed. Build artifacts (binaries,
  MSIs) are gitignored; the profile bundle + cookbook live in C:\temp (outside the repo).

## Self-Check: PASSED
