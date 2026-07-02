---
phase: 99-upstream-absorb-fork-invariant-verify
plan: "04"
subsystem: sandbox/linux + cli-org-refs
tags: [upstream-sync, cluster-D, cluster-E, 9p-filesystem, org-migration, will-sync]
dependency_graph:
  requires: [99-03]
  provides: [cluster-D-absorbed, cluster-E-absorbed]
  affects: [crates/nono/src/sandbox/linux.rs, crates/nono-cli/src/setup.rs, crates/nono-cli/src/test_env.rs, crates/nono-cli/src/update_check.rs]
tech_stack:
  added: []
  patterns: [manual-replay, cherry-pick-trailer, DCO-sign-off]
key_files:
  created: []
  modified:
    - crates/nono/src/sandbox/linux.rs
    - crates/nono-cli/src/setup.rs
    - crates/nono-cli/src/test_env.rs
    - crates/nono-cli/src/update_check.rs
decisions:
  - "Cluster D (5b8e94da): is_9p_path detection via statfs(2) + V9FS_MAGIC 0x01021997; warnings deduplicated per mount device ID; fork GPU tests (test_is_nvidia_compute_device_*) preserved after upstream test section"
  - "Cluster E (c808f000): always-further/nono GitHub URLs migrated to nolabs-ai/nono in 3 src/ files; always-further/claude package registry identifiers preserved (setup.rs lines 903, 1309); profile/mod.rs, proxy_runtime.rs, route.rs had no matching lines in fork — no action taken"
  - "OscarMackJr/nono fork identity URLs in .github/workflows/release.yml and scripts/build-windows-msi.ps1 untouched — correct, they are not always-further references"
metrics:
  duration: "10 min"
  completed: "2026-06-30T14:54:54Z"
  tasks: 2
  files_modified: 4
---

# Phase 99 Plan 04: Cluster D + E Manual Replay Summary

**One-liner:** Replayed 9P filesystem warning (additive `is_9p_path` detection in linux.rs) and org-ref migration (always-further→nolabs-ai) as two atomic DCO-signed commits; fork GPU tests and production identity URLs preserved.

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Manual replay 5b8e94da (Cluster D — 9P warning) | 345891fa | crates/nono/src/sandbox/linux.rs (+128) |
| 2 | Manual replay c808f000 (Cluster E — org-ref migration) | c4b03fb2 | setup.rs, test_env.rs, update_check.rs (+/-5) |

## What Was Built

### Task 1: Cluster D — 9P Filesystem Warning (5b8e94da)

Added to `crates/nono/src/sandbox/linux.rs`:

- `V9FS_MAGIC: libc::c_long = 0x0102_1997` constant
- `fs_type_unsupported(f_type)` — pure predicate against V9FS_MAGIC
- `unsupported_filesystem_dev(path)` — statfs(2) probe returning mount device ID for dedup; skips syscall for paths not under `/mnt`
- `warned_unsupported_devs: HashSet<u64>` — per-loop dedup tracker before the Landlock capability-add loop
- 9P warning emission (after IoctlDev block, before "Adding rule" debug log)
- 5 new test functions: `test_fs_type_unsupported_v9fs_magic`, `test_fs_type_unsupported_known_supported_types`, `test_unsupported_filesystem_dev_native_paths`, `test_unsupported_filesystem_dev_nonexistent_path`, `test_unsupported_filesystem_dev_wsl2_mount`

Fork's GPU tests (`test_is_nvidia_compute_device_accepts_upstream_list`, `test_is_nvidia_compute_device_rejects_non_compute`, `test_collect_linux_gpu_paths_*`) are inserted AFTER the upstream test section and are preserved intact.

### Task 2: Cluster E — Org-Ref Migration (c808f000)

Applied `always-further/nono` → `nolabs-ai/nono` GitHub URL replacements in drift-filter `src/` files:

- `setup.rs`: 3 GitHub doc/readme URLs migrated; `always-further/claude` package registry identifiers at lines 903 and 1309 preserved (per upstream commit intent)
- `test_env.rs`: Issue tracker URL in doc comment migrated
- `update_check.rs`: Test fixture JSON `release_url` migrated; production `UPDATE_SERVICE_URL = "https://update.nono.sh/v1/check"` untouched

Files with no matching `always-further/nono` references in fork — no action taken:
- `profile/mod.rs`: `/repos/always-further/nono/issues` line absent in fork (fork's test fixtures diverged)
- `proxy_runtime.rs`: `/repos/always-further/nono/issues/787` absent in fork
- `route.rs`: `/always-further/nono.git/` test fixtures absent in fork
- `command_policy.rs`, `migration.rs`: deleted in fork — confirmed absent

## Verification

```
grep -n "is_9p_path\|unsupported_filesystem_dev\|V9FS_MAGIC" crates/nono/src/sandbox/linux.rs
# → 15 matches (function definition + call sites + tests)

grep -n "test_is_nvidia_compute_device" crates/nono/src/sandbox/linux.rs
# → 2 matches (GPU tests preserved)

grep -rn "OscarMackJr" .github/workflows/release.yml
# → 2 matches (fork identity preserved in release.yml)

grep -rn "always-further/nono" crates/nono-cli/src/setup.rs
# → 0 matches (migration applied)

grep -n "nolabs-ai" crates/nono-cli/src/setup.rs
# → 3 matches (migration confirmed)

grep -n "UPDATE_SERVICE_URL" crates/nono-cli/src/update_check.rs
# → production URL "https://update.nono.sh/v1/check" unchanged

cargo check --workspace --all-targets
# → Finished dev profile (exit 0, no errors)
```

## Deviations from Plan

### Auto-resolved — No action on absent lines

**[Rule 1 — No Bug] profile/mod.rs, proxy_runtime.rs, route.rs: no matching lines**
- **Found during:** Task 2
- **Issue:** The upstream diff for c808f000 showed changes to these files, but the specific lines (`/repos/always-further/nono/issues`, `/repos/always-further/nono/issues/787`, `/always-further/nono.git/info/refs`) do not exist in the fork's current versions — the fork's test fixtures have diverged from upstream.
- **Resolution:** No action taken on these 3 files; this is expected behavior for a conflict-prone org-ref migration (c808f000 was empirically confirmed as a CONFLICT during research phase).
- **Impact:** None — these were test fixtures/comments that do not affect functionality.

### Observation — OscarMackJr only in non-drift-filter files

The plan's concern about preserving `OscarMackJr/nono` fork identity URLs was valid but moot for drift-filter files: no `OscarMackJr` references exist in `crates/` at all. They appear only in `.github/workflows/release.yml` and `scripts/build-windows-msi.ps1` which are outside the drift-filter scope. Production identity confirmed intact.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The `unsupported_filesystem_dev` function makes a `statfs(2)` syscall but only on paths under `/mnt` — this is a read-only metadata probe with no security surface.

## Known Stubs

None. All changes are complete as-is.

## Self-Check: PASSED

- `crates/nono/src/sandbox/linux.rs` modified: FOUND (V9FS_MAGIC, unsupported_filesystem_dev, warnings, tests)
- `crates/nono-cli/src/setup.rs` modified: FOUND (3 nolabs-ai URLs)
- `crates/nono-cli/src/test_env.rs` modified: FOUND (nolabs-ai URL)
- `crates/nono-cli/src/update_check.rs` modified: FOUND (test fixture URL)
- Commit 345891fa: FOUND (`fix(sandbox): warn when capability path is on a 9P filesystem`)
- Commit c4b03fb2: FOUND (`chore: migrate GitHub org references from always-further to nolabs-ai`)
- Both commits have `(cherry picked from commit ...)` trailers: VERIFIED
- Both commits have `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`: VERIFIED
- `cargo check --workspace --all-targets` exits 0: VERIFIED
