---
phase: 51
plan: 02
subsystem: nono-shell-broker
tags: [windows, broker, no-pty, stdio, STARTF_USESTDHANDLES, tdd]
dependency_graph:
  requires: []
  provides: ["nono-shell-broker --no-pty flag", "BrokerArgs.no_pty field", "STARTF_USESTDHANDLES stdio binding"]
  affects: ["crates/nono-shell-broker/src/main.rs"]
tech_stack:
  added: []
  patterns: ["STARTF_USESTDHANDLES stdio handle binding", "boolean argv flag with no value"]
key_files:
  created: []
  modified:
    - crates/nono-shell-broker/src/main.rs
decisions:
  - "no_pty field placed after cwd in BrokerArgs; boolean flag with no value argument (matches plan spec)"
  - "STARTF_USESTDHANDLES branch added between lpAttributeList init and CreateProcessAsUserW; guarded by args.no_pty && len>=3"
  - "build_command_line_tests args() helper updated to include no_pty: false (struct completeness, not behavior change)"
metrics:
  duration: "~8 minutes"
  completed: "2026-05-26"
  tasks_completed: 1
  tasks_total: 1
  files_modified: 1
---

# Phase 51 Plan 02: nono-shell-broker --no-pty Flag + STARTF_USESTDHANDLES Summary

**One-liner:** `--no-pty` flag parsed into `BrokerArgs.no_pty`; `run()` binds three pipe handles as child stdio via `STARTF_USESTDHANDLES` when flag present; 2 new TDD unit tests pass; existing PTY path byte-behaviorally unchanged.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| RED  | Failing tests for --no-pty flag | `65c8aee6` | crates/nono-shell-broker/src/main.rs |
| GREEN | Implement --no-pty + STARTF_USESTDHANDLES | `7553a927` | crates/nono-shell-broker/src/main.rs |

## What Was Built

Extended `nono-shell-broker` to support a `--no-pty` CLI flag enabling the supervisor relay path for non-PTY sessions:

1. **`BrokerArgs.no_pty: bool`** — new field with Phase 51 doc comment explaining the purpose and PTY-path independence.

2. **`parse_args` `--no-pty` arm** — boolean flag (no value argument); sets `no_pty = true`. Initialized to `false` alongside other `let mut` declarations. Added before the `other =>` catch-all so it is explicitly recognized and does not hit the `"unknown broker arg"` hard-fail.

3. **`STARTF_USESTDHANDLES` import** — added to the existing `windows_sys::Win32::System::Threading` use block (no new use statement).

4. **`run()` branch** — after `startup_info_ex.lpAttributeList = attr_list;` and before `CreateProcessAsUserW`, when `args.no_pty && args.inherit_handles.len() >= 3`: sets `dwFlags = STARTF_USESTDHANDLES` and binds `hStdInput/hStdOutput/hStdError` to `args.inherit_handles[0..2]`. Includes `// SAFETY:` comment per CLAUDE.md.

5. **`build_command_line_tests::args()` helper** — updated to include `no_pty: false` (struct completeness required by Rust; no behavior change to existing tests).

6. **Two new unit tests** in `parse_args_tests` module:
   - `parse_args_no_pty_flag_accepted`: argv with `--no-pty` + 3 handles; asserts `no_pty=true` and `inherit_handles.len()==3`
   - `parse_args_no_pty_absent_defaults_false`: argv without `--no-pty`; asserts `no_pty=false`

## Verification Results

```
running 17 tests
test broker::build_command_line_tests::... (5 tests) ... ok
test broker::parse_args_tests::... (12 tests, includes 2 new) ... ok
test result: ok. 17 passed; 0 failed; 0 ignored
```

- `cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc` exits 0 (17/17)
- `cargo build -p nono-shell-broker --target x86_64-pc-windows-msvc` exits 0
- `cargo clippy -p nono-shell-broker --target x86_64-pc-windows-msvc -- -D warnings -D clippy::unwrap_used` exits 0
- `grep "STARTF_USESTDHANDLES" main.rs`: 8 matches (use decl + doc comments + assignment + test assertion)
- `grep "no_pty" main.rs`: 18 matches (field def, init, match arm, struct literal, run branch, tests)

## TDD Gate Compliance

| Gate | Commit | Status |
|------|--------|--------|
| RED (test) | `65c8aee6` | PASS — `E0609: no field no_pty` confirmed failure |
| GREEN (feat) | `7553a927` | PASS — all 17 tests pass |
| REFACTOR | N/A | Not needed — code is clean on first pass |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing struct completeness] `build_command_line_tests::args()` helper needed `no_pty: false`**
- **Found during:** GREEN phase compilation
- **Issue:** The `args()` helper in `build_command_line_tests` constructs `BrokerArgs` via struct literal. Adding `no_pty: bool` to the struct required adding `no_pty: false` to this helper or it would not compile.
- **Fix:** Added `no_pty: false` to the `args()` helper struct literal.
- **Files modified:** crates/nono-shell-broker/src/main.rs
- **Commit:** `7553a927`

## Security Analysis (Threat Model Coverage)

| Threat ID | Status |
|-----------|--------|
| T-51B-01 | MITIGATED — `STARTF_USESTDHANDLES` branch binds 3 pipe handles; child writes to relay pipes, not broker console |
| T-51B-02 | ACCEPTED — mandatory-label NO_WRITE_UP is kernel-level, independent of stdio handle binding |
| T-51B-03 | ACCEPTED — BROKER-CR-02 and BROKER-CR-03 execute in `parse_args()` before the `STARTF_USESTDHANDLES` branch |
| T-51B-04 | MITIGATED — `if args.no_pty && ...` guard; existing PTY/console path unchanged when `--no-pty` absent |
| T-51B-SC | ACCEPTED — no new dependencies; `STARTF_USESTDHANDLES` available via existing `windows-sys` |

## Known Stubs

None — implementation is complete. The `no_pty` field is fully wired: parsed from argv, stored in `BrokerArgs`, and consumed in `run()`.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced. The `STARTF_USESTDHANDLES` branch operates on already-validated handle values within existing trust boundary (nono-cli → broker argv).

## Self-Check: PASSED

- [x] `crates/nono-shell-broker/src/main.rs` modified and exists
- [x] Commit `65c8aee6` (RED) exists in git log
- [x] Commit `7553a927` (GREEN) exists in git log
- [x] 17/17 tests pass
- [x] Clippy clean (0 warnings, 0 errors)
- [x] `grep "STARTF_USESTDHANDLES"` returns 8 matches (>= 2 required)
- [x] `grep "no_pty"` returns 18 matches (>= 4 required)
