
## Plan 62-12 — pre-existing out-of-scope test failures (NOT introduced by 62-12)

Confirmed pre-existing on the pristine baseline (e290d6bf code, before any 62-12
change). Environment-dependent (real user AppData/HOME state + parallel env-var
contamination per CLAUDE.md note). Untouched by 62-12 (no AppContainer / WFP /
ExecConfig / DACL / broker code in these paths):

- `protected_paths::tests::blocks_parent_directory_capability` — `blocked: ()` panic (HOME/path-coverage).
- `protected_paths::tests::blocks_child_directory_capability`
- `protected_paths::tests::requested_path_blocks_nonexistent_child_under_protected_root`
- `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name` — fails because a REAL `my-agent.json` profile already exists in the operator's actual `%APPDATA%\nono\profiles\` (test-env pollution).

Disposition: out of scope for 62-12 (SCOPE BOUNDARY). Defer to a test-hygiene pass.

### 62-13 re-confirmation (2026-06-03)

The same 4 failures persist at the pristine `e290d6bf` baseline (verified via a
detached worktree on the parent of all 62-13 commits, run single-threaded — so
NOT a parallelism flake either). 62-13 touched only the AppContainer profile
guard (windows.rs), the broker spawn (main.rs), and the cwd-ancestor DACL guard
(dacl_guard.rs / mod.rs) — none of which are on the `protected_paths` /
`profile_cmd` code paths. Re-confirmed out of scope; still deferred to a
test-hygiene pass.
