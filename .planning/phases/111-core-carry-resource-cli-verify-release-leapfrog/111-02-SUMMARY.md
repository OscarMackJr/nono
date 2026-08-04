---
phase: 111-core-carry-resource-cli-verify-release-leapfrog
plan: 02
subsystem: cli
tags: [clap, docs, cli-help, resource-limits, cgroup, rlimit, job-object]

# Dependency graph
requires:
  - phase: 111-01
    provides: policy.json ~/.cache grant + MAX_CRYPTO_THREADS=12 (no overlap with this plan's files)
provides:
  - Corrected --memory/--timeout/--max-processes doc comments in cli.rs (D-05/D-06 satisfied)
  - Corrected Resource Limits section in docs/cli/usage/flags.mdx (including a docs-only
    --cpu-percent correction the plan text did not explicitly call out but acceptance
    criteria required)
affects: [111-04 (VERIFY-01 combined fork-invariant pass reads this corrected surface)]

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - crates/nono-cli/src/cli.rs
    - docs/cli/usage/flags.mdx

key-decisions:
  - "D-04 confirmed: neither upstream e6d26871 (#1269) nor 34c2c975 (#1403) renames or re-ranges any of the four flags — no alias needed this phase."
  - "docs/cli/usage/flags.mdx's --cpu-percent entry also carried the stale claim (unlike cli.rs's already-accurate --cpu-percent doc comment) — corrected it too, since the acceptance criteria required zero remaining occurrences of the stale substring anywhere in the file, not just in the three entries the plan's action text named."

patterns-established: []

requirements-completed: [CORE-02]

# Metrics
duration: 25min
completed: 2026-08-04
---

# Phase 111 Plan 02: Resource-Limit CLI Help-Text Correction Summary

**Corrected the false "accepted with a warning pending cross-platform follow-up" claim for `--memory`/`--timeout`/`--max-processes` in `cli.rs` and `docs/cli/usage/flags.mdx` to state the true, already-implemented per-platform kernel enforcement (cgroup v2 fail-closed on Linux; best-effort `setrlimit(RLIMIT_AS)` with possible silent `EINVAL` on macOS for `--memory`; UID-wide fail-closed `RLIMIT_NPROC` on macOS for `--max-processes`).**

## Performance

- **Duration:** 25 min
- **Started:** 2026-08-04T19:45:00Z (approx, following 111-01 completion)
- **Completed:** 2026-08-04
- **Tasks:** 2/2 completed
- **Files modified:** 2

## Accomplishments
- `crates/nono-cli/src/cli.rs`'s three stale doc comments (`--memory`, `--timeout`, `--max-processes`) rewritten to state true per-platform enforcement, derived by reading all three enforcement backends live (`launch_runtime.rs`, `exec_strategy.rs`'s macOS `pre_exec` hook, `supervisor_macos.rs`, `supervisor_linux.rs`, `exec_strategy_windows/launch.rs`) — not from assumption or from CONTEXT.md's from-memory summary.
- `docs/cli/usage/flags.mdx`'s identical stale claim corrected across the section intro and all three flag entries, matching the corrected `cli.rs` text in expanded prose form.
- `--cpu-percent`'s doc comment in `cli.rs` left byte-identical (already accurate); its `docs/cli/usage/flags.mdx` counterpart, which was NOT accurate (carried the same stale claim), was also corrected — going beyond the plan's literal action text but required by the plan's own acceptance criteria (zero remaining occurrences of the stale substring in the file).
- D-04 flag-freeze confirmed and preserved: zero `#[arg(...)]` attribute, value-type, or range changes.
- Both mandatory cross-target clippy gates (linux-gnu via `cross clippy`, apple-darwin via `cargo-zigbuild clippy`, `SDKROOT` unset) re-confirmed GREEN on the modified `cli.rs`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Correct cli.rs's three stale resource-limit doc comments** - `f378d68b` (docs)
2. **Task 2: Correct the identical stale claim in docs/cli/usage/flags.mdx** - `5d7761a1` (docs)

_No TDD tasks in this plan — doc-comment-only change with zero test modifications, per 111-RESEARCH.md's confirmation that no test asserts on help-text strings._

## Files Created/Modified
- `crates/nono-cli/src/cli.rs` - Corrected `--memory`/`--timeout`/`--max-processes` doc comments (lines ~2781-2815) to state true per-platform kernel enforcement; `--cpu-percent`'s already-accurate doc comment left untouched.
- `docs/cli/usage/flags.mdx` - Corrected "Resource Limits" section intro (line ~1234) and all three flag entries (`--cpu-percent`, `--memory`, `--max-processes`) to match the corrected `cli.rs` text in expanded docs-site prose.

## Decisions Made
- **D-04 explicitly confirmed:** upstream commits `e6d26871` (#1269) and `34c2c975` (#1403) were read live during Phase 111 research and neither renames nor re-ranges any of the four resource-limit flags — the fork's flag surface (`--cpu-percent`/`--memory`/`--timeout`/`--max-processes`) stays byte-for-byte unchanged, no alias required.
- **Docs-site `--cpu-percent` correction (beyond the plan's literal action text):** the plan's action paragraph described `--cpu-percent`'s docs-site entry as "already correct" and only asked to correct the section intro's blanket claim, but on read the `--cpu-percent` flag entry itself (line 1238) also carried the stale "accepted with a warning pending cross-platform follow-up" substring — distinct from `cli.rs`, where `--cpu-percent`'s doc comment genuinely was already accurate. Corrected it too, since the plan's acceptance criteria required zero remaining occurrences of the stale substring anywhere in the file (a grep-verifiable criterion), and leaving it in would have left the docs site self-contradictory (accurate intro, stale flag entry).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] docs/cli/usage/flags.mdx's --cpu-percent entry also carried the stale claim**
- **Found during:** Task 2 (correcting flags.mdx)
- **Issue:** The plan's action text characterized the docs-site `--cpu-percent` entry as "already correct" (mirroring `cli.rs`'s genuinely-accurate `--cpu-percent` doc comment), but the actual file content at line 1238 read "On Linux/macOS the value is accepted with a warning pending cross-platform follow-up" — the same false claim as the other two flags, not the true parse-time-rejection behavior.
- **Fix:** Corrected the `--cpu-percent` entry to state the true macOS parse-time-rejection and Linux cgroup v2 `cpu.max` enforcement, matching `cli.rs`'s accurate text.
- **Files modified:** `docs/cli/usage/flags.mdx`
- **Verification:** `grep -c 'accepted with a warning pending cross-platform follow-up' docs/cli/usage/flags.mdx` returns `0`.
- **Committed in:** `5d7761a1` (part of Task 2 commit)

## Verification Results

- `grep -c "accepted with a warning pending cross-platform follow-up" crates/nono-cli/src/cli.rs` → `0` (PASS).
- `test "$(grep -c 'accepted with a warning pending cross-platform follow-up' docs/cli/usage/flags.mdx)" = "0"` → PASS.
- `cargo test -p nono-sandbox-cli --bin nono -- cpu_percent_range_enforced_by_clap max_processes_range_enforced_by_clap memory_zero_rejected_by_parser` → 3 passed, 0 failed.
- `cargo test -p nono-sandbox-cli --bin nono -- env_filter_flags_do_not_collide_with_phase16_flags` (the Phase 16 regression guard at ~line 4147) → 1 passed, 0 failed.
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` → exit 0, zero findings.
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (SDKROOT unset) → exit 0, zero findings.
- `git check-ignore -v docs/cli/usage/flags.mdx` → exit 1 (not gitignored, confirming plain `git add` sufficiency, unlike `docs/cli/development/`).

## Known Stubs

None.

## Threat Flags

None — this plan corrects documentation about an already-implemented, unmodified security control (T-111-01 in the plan's own threat model); no new network endpoints, auth paths, file access patterns, or schema changes were introduced.

## Issues Encountered
None blocking. The single deviation (docs-site `--cpu-percent` entry also being stale) was auto-fixed under Rule 1 as documented above.

## User Setup Required
None — doc-comment and docs-site text changes only, no new dependencies, environment variables, or manual steps.
