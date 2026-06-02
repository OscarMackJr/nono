---
quick_id: 260601-wha
slug: sync-up-binaries-and-source-code-rebuild
date: 2026-06-02
status: in-progress
description: >
  Sync binaries and source: rebuild release binaries for the
  x86_64-pc-windows-msvc triple, re-derive dist/windows/*.wxs cleanly and build
  both MSIs (machine + user) at version 0.57.5, then commit the reconciled wxs
  reference snapshots + stray working-tree changes.
---

# Quick Task 260601-wha — Sync up binaries and source code

## Context (decisions locked via AskUserQuestion)

1. **Scope = Full**: rebuild release binaries → build both MSIs → commit. No `git push`.
2. **wxs handling = discard + re-derive cleanly**: drop the degraded uncommitted
   working-copy (which switched to `target\release` paths, stripped the machine-MSI
   WFP comments, and mangled component tags). Regenerate the canonical wxs via the
   build script so the machine MSI keeps full WFP service+driver components and the
   user MSI stays minimal.
3. **Source path convention = `target\x86_64-pc-windows-msvc\release`** (explicit
   triple — the CI/release convention, not dev-default `target\release`).

### Key facts established during investigation
- `scripts/build-windows-msi.ps1` **generates** `dist/windows/nono-{machine,user}.wxs`
  at build time (writes to `$OutputDir/nono-{scope}.wxs`, line 209-210/404). The
  checked-in wxs are therefore script-derived reference snapshots — running the
  script with correct args *is* the clean re-derivation.
- Workspace version is **0.57.5** (committed wxs said `0.53.1` — genuinely stale).
- `.wxs` are git-TRACKED; `.msi`/`.wixpdb` are git-IGNORED (MSIs are local artifacts,
  not committed).
- WiX 7.0.0, cargo, and the `x86_64-pc-windows-msvc` target are all present.
- Worktree isolation is unsafe here (empty `target/` → full rebuild; gitignored MSIs
  would be written into and destroyed with the worktree). Run sequentially on `main`.
- Machine scope requires BOTH `-ServiceBinaryPath` and `-DriverBinaryPath` (or
  neither); driver MUST be the pre-signed `crates/nono-cli/data/windows/nono-wfp-driver.sys`.
  User scope MUST omit both (script throws otherwise).

## Tasks

### T1 — Rebuild release binaries (x86_64-pc-windows-msvc)
- **files:** `target/x86_64-pc-windows-msvc/release/{nono,nono-shell-broker,nono-wfp-service}.exe`
- **action:** `cargo build --release --target x86_64-pc-windows-msvc` (workspace).
  Produces all three required bins (`nono-wfp-service` is a bin in `nono-cli`).
- **verify:** all three `.exe` exist with a fresh `LastWriteTime`.
- **done:** binaries rebuilt against current source under the triple path.

### T2 — Re-derive wxs + build both MSIs
- **files:** `dist/windows/nono-machine.wxs`, `dist/windows/nono-user.wxs`,
  `dist/windows/nono-v0.57.5-x86_64-pc-windows-msvc-{machine,user}.msi`
- **action:** discard the stale working copy first
  (`git restore dist/windows/*.wxs`), then run `build-windows-msi.ps1` twice:
  - machine: `-VersionTag v0.57.5 -BinaryPath <triple>/nono.exe -BrokerPath <triple>/nono-shell-broker.exe -ServiceBinaryPath <triple>/nono-wfp-service.exe -DriverBinaryPath crates/nono-cli/data/windows/nono-wfp-driver.sys -Scope machine`
  - user: same `-BinaryPath`/`-BrokerPath`, `-Scope user` (no service/driver).
- **verify:** both wxs now show `Version="0.57.5"` + `target\x86_64-pc-windows-msvc\release`
  Source paths; machine wxs retains WFP service+driver+eventlog components; both MSIs built.
- **done:** canonical wxs regenerated and MSIs produced on disk.

### T3 — Verify + commit
- **files:** `dist/windows/*.wxs`, `.planning/.continue-here.md` (stray deletion),
  quick-task artifacts, `.planning/STATE.md`
- **action:** confirm wxs diff is the clean version bump + correct paths (no dropped
  components); stage tracked changes (MSIs are gitignored); commit atomically.
- **verify:** `git status` clean except intended; wxs committed; MSIs present but untracked.
- **done:** working tree reconciled; STATE.md records the quick task.

## must_haves
- truths:
  - Both checked-in wxs reference snapshots are at version 0.57.5 and reference the
    `x86_64-pc-windows-msvc\release` Source paths.
  - The machine wxs ships the full WFP backend (service + driver + eventlog registry).
  - Fresh MSIs for both scopes exist on disk at v0.57.5.
- artifacts:
  - `dist/windows/nono-machine.wxs`, `dist/windows/nono-user.wxs`
  - `dist/windows/nono-v0.57.5-x86_64-pc-windows-msvc-{machine,user}.msi`
- key_links:
  - `scripts/build-windows-msi.ps1`
  - `crates/nono-cli/data/windows/nono-wfp-driver.sys`
