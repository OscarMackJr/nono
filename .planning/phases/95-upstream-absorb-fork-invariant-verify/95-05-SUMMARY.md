---
phase: 95-upstream-absorb-fork-invariant-verify
plan: "05"
subsystem: nono-core-sandbox-linux
tags:
  - gap-closure
  - fork-invariant
  - landlock
  - security
dependency_graph:
  requires:
    - 95-04 (fork-invariant verify checklist that identified WR-02/WR-03 gaps)
  provides:
    - WR-02 closed: arch-portable msghdr offset derivation in read_msghdr_dest
    - WR-03 closed: GPU Landlock enforcement (is_nvidia_compute_device, collect_linux_gpu_paths, caps.gpu() branch)
  affects:
    - crates/nono/src/sandbox/linux.rs
tech_stack:
  added: []
  patterns:
    - core::mem::offset_of! for compile-time struct layout derivation (WR-04 pattern)
    - #[cfg(target_os = "linux")] gating for platform-specific code on Windows host
key_files:
  created: []
  modified:
    - crates/nono/src/sandbox/linux.rs
decisions:
  - WR-04 offset derivation replaces hardcoded MSGHDR_MIN_READ = 12: compile-time portability, no change to function signatures or call sites
  - GPU tests gated with #[cfg(target_os = "linux")] so they compile on Windows but only run on Linux CI
  - caps.gpu() branch also gated with #[cfg(target_os = "linux")] inside apply_with_abi (the fn itself is already Linux-only via cfg, but the explicit gate is belt-and-suspenders)
metrics:
  duration: "8 minutes"
  completed: "2026-06-26T12:49:29Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 1
---

# Phase 95 Plan 05: WR-02/WR-03 Fork-Invariant Restore Summary

**One-liner:** Restored arch-portable msghdr offset_of! derivation (WR-04/WR-02) and Linux GPU Landlock enforcement path (WR-03) from phase-base ed6cdde1, closing two silent security regressions introduced by Cluster A cherry-pick conflict resolution.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Restore WR-02 — arch-portable msghdr offset derivation | 8bca078b | crates/nono/src/sandbox/linux.rs |
| 2 | Restore WR-03 — GPU enforcement (helpers + caps.gpu() branch + 4 tests) | 5d1e9077 | crates/nono/src/sandbox/linux.rs |

## What Was Built

### Task 1 — WR-02 (arch-portable msghdr offset derivation)

Replaced the hardcoded `const MSGHDR_MIN_READ: usize = 12` (LP64-only) in `read_msghdr_dest` with a compile-time arch-portable derivation:

- `const MSG_NAME_OFFSET: usize = core::mem::offset_of!(libc::msghdr, msg_name);`
- `const MSG_NAME_LEN_OFFSET: usize = core::mem::offset_of!(libc::msghdr, msg_namelen);`
- `const PTR_SIZE: usize = core::mem::size_of::<usize>();`
- `const MSGHDR_MIN_READ: usize = MSG_NAME_LEN_OFFSET + 4;`
- Two compile-time `assert!()` guards (ordering + non-overlap)
- Arch-portable pointer extraction using zero-padded `[0u8; 8]` buffer
- Arch-portable socklen_t extraction via `try_into()` returning `NonoError` on failure
- WR-01 TOCTOU comment at the `msg_name == 0` branch
- `#[must_use]` on `read_msghdr_dest` and `read_mmsghdr_dests`

### Task 2 — WR-03 (GPU Landlock enforcement)

Restored three deleted GPU enforcement regions from ed6cdde1:

**`is_nvidia_compute_device`** (`#[cfg(target_os = "linux")]`, `#[must_use]`):
- Matches: `nvidiactl`, `nvidia-uvm`, `nvidia-uvm-tools`, `nvidia[0-9]+`
- Rejects: `nvidia-modeset` (display, not compute), bare `nvidia`, non-compute variants

**`collect_linux_gpu_paths`** (`#[cfg(target_os = "linux")]`):
- Enumerates `/dev/dri/renderD*`, `/dev/nvidia*` (via predicate), `/dev/nvidia-caps/*`, `/dev/kfd`, `/dev/dxg`, `/usr/lib/wsl/lib`
- NVIDIA procfs grants (`/proc/driver/nvidia`, `/proc/driver/nvidia-uvm`, `/proc/self`, `/proc/self/task`) gated on `nvidia_present`
- Vulkan ICD manifests and GPU sysfs (`/usr/share/vulkan`, `/etc/vulkan`, `/sys/class/drm`)
- Absent paths silently skipped — works across headless CI, NVIDIA-only, AMD-only, WSL2

**`caps.gpu()` Landlock branch** in `apply_with_abi` (before `restrict_self()`):
- `#[cfg(target_os = "linux")]` gate
- IoctlDev augmentation for device nodes on ABI v5+
- Saturating `nvidia_device_count` accumulator
- `info!` log with total path count, NVIDIA count, `nvidia_present`

**4 GPU unit tests** (all `#[cfg(target_os = "linux")]`):
- `test_is_nvidia_compute_device_accepts_upstream_list`
- `test_is_nvidia_compute_device_rejects_non_compute`
- `test_collect_linux_gpu_paths_is_callable_without_panic`
- `test_collect_linux_gpu_paths_nvidia_procfs_gated_on_nvidia_presence`

## Verification

All acceptance criteria satisfied:

**WR-02:**
- `grep "const MSGHDR_MIN_READ: usize = 12"` returns NO MATCH
- `core::mem::offset_of!(libc::msghdr, msg_name)` present
- `core::mem::offset_of!(libc::msghdr, msg_namelen)` present
- `const PTR_SIZE: usize = core::mem::size_of::<usize>()` present
- 2+ `const _: () = assert!` guards present
- `#[must_use]` on both `read_msghdr_dest` and `read_mmsghdr_dests`
- TOCTOU comment at `msg_name == 0` branch
- `namelen_bytes.*try_into` arch-portable extraction present

**WR-03:**
- `fn is_nvidia_compute_device` present (18 references total)
- `fn collect_linux_gpu_paths` present
- `caps.gpu()` Landlock branch wired in apply path
- All 4 GPU tests present in `#[cfg(test)]` module

**CI (Windows host):**
- `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`: CLEAN
- `cargo fmt --all -- --check`: CLEAN
- `cargo test -p nono`: 802 passed, 1 failed (pre-existing `try_set_mandatory_label` only)
- `cargo test -p nono-cli`: 1369 passed, 11 failed (all pre-existing baseline)
- `cargo test -p nono-ffi`: 49 passed, 0 failed
- `cargo audit`: 0 errors, 4 allowed warnings (pre-existing)
- `git diff HEAD -- crates/nono-cli/src/exec_strategy_windows/`: EMPTY

## Deviations from Plan

None — plan executed exactly as written. Recovery source `ed6cdde1` used verbatim. Cluster A AF_UNIX deadlock fix from `ae77d198` unregressed (IPC ordering intact, `read_msghdr_dest` call sites unchanged).

## Known Stubs

None.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes. Both restored items are security hardening (correctness fixes for LP64-only offset assumption and silent fail-open GPU enforcement).

## Self-Check: PASSED

- `crates/nono/src/sandbox/linux.rs` modified and staged
- Commit `8bca078b` exists (Task 1 WR-02)
- Commit `5d1e9077` exists (Task 2 WR-03)
- No unexpected file deletions
- WR-02 grep checks: all pass
- WR-03 grep checks: all pass
- make ci equivalent (Windows): CLEAN
