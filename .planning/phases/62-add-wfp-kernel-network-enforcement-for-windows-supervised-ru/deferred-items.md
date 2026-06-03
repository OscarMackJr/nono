
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
