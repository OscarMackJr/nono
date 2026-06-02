---
quick_id: 260601-wha
slug: sync-up-binaries-and-source-code-rebuild
date: 2026-06-02
status: complete
---

# Quick Task 260601-wha — Summary

Synced binaries and the Windows MSI source to current source at version **0.57.5**.

## What was done

### T1 — Rebuilt release binaries (x86_64-pc-windows-msvc triple)
`cargo build --release --target x86_64-pc-windows-msvc` (workspace, 6m28s, exit 0).
Produced fresh:
- `target/x86_64-pc-windows-msvc/release/nono.exe` (15,614,976 bytes)
- `target/x86_64-pc-windows-msvc/release/nono-shell-broker.exe`
- `target/x86_64-pc-windows-msvc/release/nono-wfp-service.exe` (a `nono-cli` bin)

### T2 — Re-derived wxs + built both MSIs
Discarded the stale uncommitted working-copy wxs first (`git restore`), then ran
`scripts/build-windows-msi.ps1` — which regenerates `dist/windows/nono-{machine,user}.wxs`
and builds the MSI in one pass:
- **machine** scope: `-ServiceBinaryPath`/`-DriverBinaryPath` supplied so the MSI ships
  the full WFP backend; driver sourced from the pre-signed checked-in
  `crates/nono-cli/data/windows/nono-wfp-driver.sys`.
- **user** scope: WFP omitted (kernel driver / LocalSystem service are machine-only).

Both MSIs built (exit 0):
- `dist/windows/nono-v0.57.5-x86_64-pc-windows-msvc-machine.msi` (5,570,560 bytes)
- `dist/windows/nono-v0.57.5-x86_64-pc-windows-msvc-user.msi` (5,439,488 bytes)

### T3 — Verify + commit
Verified the regenerated wxs against the three locked decisions:
- `Version="0.57.5"` in both ✓ (was `0.53.1`)
- `Source` paths use `target\x86_64-pc-windows-msvc\release` throughout ✓ (no `target\release`)
- machine wxs retains `cmpWfpServiceExe` + `cmpWfpDriverSys` + `cmpEventLogSource`;
  user wxs has zero WFP components ✓

Committed the tracked changes (MSIs/wixpdb are gitignored → local artifacts only) plus
the stray `.planning/.continue-here.md` deletion.

## Decisions (locked via AskUserQuestion)
1. Scope = **Full** (rebuild binaries → build both MSIs → commit; no push).
2. wxs = **discard + re-derive cleanly** (don't ship the degraded `target\release` working copy).
3. Path convention = **`x86_64-pc-windows-msvc\release`** (explicit triple).

## Notes
- The `dist/windows/*.wxs` are script-generated reference snapshots; the build script
  rewrites them on every run. The earlier "degraded" working copy was the same script
  output but generated with the wrong (bare `target\release`) binary paths.
- Diff drops the old hand-written explanatory comments (the generator doesn't emit them)
  and fixes the mojibake `â€"` → `—`. Functionally identical components otherwise.
- MSIs are **local/UNSIGNED** (CI signing not run here, per scope).
- Not pushed — `main` remains ahead of `origin`.
