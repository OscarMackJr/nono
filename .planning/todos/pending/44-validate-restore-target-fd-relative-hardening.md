---
id: 44-validate-restore-target-fd-relative-hardening
opened: 2026-05-20
opened_by: Phase 44 Plan 44-01 (REQ-REVIEW-FU-01 docs WR-01 P43, D-44-B4)
priority: medium
category: security-hardening
tags: [snapshot, restore, toctou, fd-relative, cross-platform]
affects:
  - crates/nono/src/undo/snapshot.rs
resolves_phase: 53
---

# validate_restore_target fd-relative TOCTOU hardening

## Context

Phase 43 introduced `validate_restore_target` (snapshot.rs:595-687) as a
per-file pre-write gate that rejects symlinked parent components before
`create_dir_all` / `retrieve_to` / `set_permissions`. The function uses
`fs::symlink_metadata` for a component-wise check.

Phase 43 code review (43-REVIEW.md WR-01) noted a residual TOCTOU race
window between the lexical validation and the non-atomic write sequence:
a local attacker with write access inside the tracked tree can swap a
directory for a symlink between validation returning `Ok(())` and the
write. Phase 44 D-44-B4 chose doc-only fix + this follow-up todo.

## Scope

Full closure requires `O_NOFOLLOW` + fd-relative ops (`openat`,
`mkdirat`, `renameat`, `fchmodat`). This is a substantial cross-platform
refactor:

- **Linux**: nix crate exposes `openat`, `mkdirat`, `renameat`,
  `fchmodat`, `fchownat`. `O_NOFOLLOW` is a standard open flag. The
  library already depends on `nix` for other syscalls, so the
  dependency is in place.
- **macOS**: same nix surface (Darwin supports all the `*at` syscalls
  + `O_NOFOLLOW`). Spot-check `fchmodat` behavior under
  `AT_SYMLINK_NOFOLLOW` flag — the macOS fchmodat semantics historically
  differ from Linux's on this flag.
- **Windows**: NO direct equivalent. Closure options:
  1. `NtCreateFile` with `OBJ_DONT_REPARSE` + `FILE_FLAG_OPEN_REPARSE_POINT`.
  2. Rejection of any symlinked target component at validation time +
     double-check via a second `symlink_metadata` at write time
     (best-effort defense-in-depth).
  3. Require the restore target tree to be on a no-symlink filesystem
     (NTFS without ReparsePoint privilege grants) and document the
     constraint.

  Closing the race on Windows may require a different architectural
  approach than Linux/macOS.

## Acceptance Criteria

1. `validate_restore_target` + the subsequent `create_dir_all` /
   `retrieve_to` / `set_permissions` sequence is refactored to use
   fd-relative ops on Linux + macOS such that the write happens
   through the SAME fd the validation gated. No TOCTOU window.
2. On Windows, the residual race is either closed via NtCreateFile-
   based path, OR a documented defense-in-depth pattern
   (double-validation + best-effort `symlink_metadata` at write-time)
   is applied AND the residual risk is documented in the function
   doc comment.
3. Cross-platform tests prove the gate holds under concurrent
   symlink-swap attempts (e.g. spawn an attacker thread that
   busy-loops swapping a path; validate that the restore either
   succeeds atomically or fails closed — never writes through a
   symlink that was swapped in mid-flight).
4. The "Residual race window" paragraph in the function doc comment
   is removed (or replaced with "Closed by fd-relative op refactor
   in Phase NN").

## Estimated Cost

Substantial: ~2-3 weeks of focused work spread across Linux, macOS,
Windows + new race-detection test infrastructure. A dedicated
security-scoped phase is warranted. Target window: post-v2.6
(after the windows-squash merge in Phase 46 lands the baseline;
revisit at v2.7 milestone planning).

## References

- Phase 43 43-REVIEW.md WR-01 (the original finding + suggested doc).
- Phase 44 44-CONTEXT.md D-44-B4 (the doc-only disposition + this todo).
- CLAUDE.md § Path Handling (TOCTOU with symlinks + canonicalization).
- The doc note shipped in Phase 44 Plan 44-01 Task 7 commit (see
  `git log --oneline --all | grep 'validate_restore_target'`).
