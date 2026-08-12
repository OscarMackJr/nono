---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 42
subsystem: windows-attestation
tags: [windows, d-28, fail-open, gap-closure, round-4, wr-23, wr-24, cint-02, cint-03]

# Dependency graph
requires:
  - phase: 117-fail-direction-contract-startup-self-attestation
    provides: "117-35's --no-fail-fast baseline"
provides:
  - "GRANTED_READ_PATHS as Option<Vec<PathBuf>> — 'no granted-path check has run' is representable
    and fail-secure (WR-23)"
  - "An executed verdict on WR-24: mechanism true, conclusion false for production inputs"
  - "A claim-to-test table in log_target_is_private's doc comment covering all 7 documented cases"
affects: [117-44]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A documented fail-secure rule must be checked against the code that implements it; WR-16
      wrote the rule as prose for two rounds while the data model could not express one clause"
    - "Settle a review's premise by execution BEFORE adopting its prescribed fix — the mechanism
      can be right and the conclusion still wrong if the operand is not what the reviewer assumed"

key-files:
  created: []
  modified:
    - crates/nono-cli/src/cli_bootstrap.rs

key-decisions:
  - "WR-24 is a FALSE POSITIVE for production and no logic was changed. The review's Path::starts_with
    reasoning is correct, but every production `granted` value is already verbatim, so the fallback
    compares verbatim-against-verbatim and works. Applying the prescribed `return true` would have
    suppressed layer detail on genuinely private channels with no defect behind it."
  - "Task 3's enumeration surfaced TWO further documented-but-untested fail-secure cases beyond the
    two findings named; both got tests in this plan."
---

# Plan 117-42: WR-23 + WR-24 — Execution Summary

**Executed:** 2026-08-12 · **Wave:** 16 · **Requirements:** CINT-02, CINT-03

## Task 1 — WR-23 (real fail-open, fixed)

`log_target_is_private`'s doc comment has claimed since WR-16 that "no granted-path check has run"
returns `false`. The code could not implement it: `GRANTED_READ_PATHS` was
`Mutex<Vec<PathBuf>>` initialised to `Vec::new()`, with nothing distinguishing "never populated"
from "populated empty", and the final expression `!granted_paths.iter().any(..)` on an empty vec is
`!false` = **`true` ("private")**. Any path reaching the gate before `set_granted_read_paths` ran
was reported private and the D-28 gate OPENED.

Changed to `Mutex<Option<Vec<PathBuf>>>` initialised to `None`, mirroring `TRACING_LOG_TARGET_PATH`'s
shape one declaration up, with the same `let Some(..) else { return false }` idiom the function
already uses at `:102`. `set_granted_read_paths` stores `Some(paths)`;
`clear_log_target_is_private_test_state` restores `None` (**not** `Some(Vec::new())` — getting that
backwards would silently make every other test in the module exercise the wrong branch).

### Perturbation proof

Reverted to `unwrap_or_default()`:

```
test ...no_granted_paths_recorded_is_not_private ... FAILED
WR-23: with no granted-path check recorded, log_target_is_private() must fail-secure to false —
got true, log_path="C:\\Users\\OMack\\AppData\\Local\\Temp\\.tmpNEr40V\\nono.log"
```

The discriminating sibling `granted_path_check_ran_and_found_nothing_is_still_private` **passed**
under the same perturbation, confirming the pair isolates the two states rather than collapsing
them. Without it, the primary test is satisfiable by hardcoding `false`.

## Task 2 — WR-24: settled by execution, and it does not hold

The plan required establishing the premise before adopting the prescribed fix. Both halves were
checked:

**Step 1 — the mechanism (TRUE).** `verbatim_and_non_verbatim_prefixes_do_not_compare_equal`
confirms by execution that a canonicalized (verbatim, `\\?\C:\...`) child does NOT `starts_with`
its own non-verbatim parent — `Path::starts_with` is component-wise and
`Prefix(VerbatimDisk) != Prefix(Disk)`. The test asserts both preconditions explicitly and includes
a control proving the same comparison DOES succeed when both sides are canonical.

**Step 2 — production reachability (the conclusion FAILS).** Every value reaching `granted` is a
`FsCapability::resolved`:

| Site | Production route | Verbatim? |
|---|---|---|
| `capability.rs:110` | `path.canonicalize()` | yes |
| `capability.rs:144` | `path.canonicalize()` | yes |
| `capability.rs:361` / `:392-405` | `resolved_parent.join(file_name)` where the parent IS canonicalized — the not-yet-existing-file case | yes (may not exist) |
| `capability.rs:457` | `path.canonicalize()` | yes |
| `capability.rs:1455` | `cap.resolved = rewritten`, only in `remap_procfs_self_references` | Linux `/proc/self`/`/dev/fd` only — **cannot reach this `#[cfg(target_os = "windows")]` function** |

So the fallback at `:131` compares **verbatim against verbatim** and preserves the comparison,
exactly as its comment claimed.

**Step 3 — branch taken: no logic change.** WR-24's mechanism claim is correct; its conclusion
("the fallback comparison can never match", "the stated defense does not exist") does **not** hold
for production inputs. Applying the review's prescribed `return true` would have been a behaviour
regression with no defect behind it: a legitimately-granted-but-not-yet-existing target would be
treated as covering, suppressing layer detail on channels that are genuinely private.

Saying so plainly, per the plan's own instruction not to manufacture a change to look responsive.
What the branch DID lack was coverage — no test exercised it at all, which is why the claim could
be made without contradiction. `fallback_branch_still_matches_for_a_verbatim_nonexistent_grant`
now drives a verbatim non-existent grant through it, and the comment records the reasoning with its
`capability.rs` citations.

## Task 3 — claim-to-test enumeration found two MORE gaps

Enumerating every early return against the doc comment's claims (rather than only checking the two
findings) surfaced two documented cases with **no test at all**:

| Documented case | Branch | Test before | Test now |
|---|---|---|---|
| file-log arm never selected | `!TRACING_LOG_TARGET_IS_PRIVATE` | `no_log_path_recorded_is_not_private` | unchanged |
| arm selected, no path recorded | `stored_path == None` | **none** | `log_arm_selected_but_no_path_recorded_is_not_private` |
| stored path fails to canonicalize | `canonicalize(..).is_err()` | **none** | `uncanonicalizable_log_path_is_not_private` |
| no granted-path check has run | `granted_paths == None` | **none** (unimplementable) | `no_granted_paths_recorded_is_not_private` |
| check ran, found nothing | `Some(vec![])` | none | `granted_path_check_ran_and_found_nothing_is_still_private` |
| log path inside a granted path | final `any(..)` | existing | unchanged |
| log path outside every granted path | final `any(..)` | existing | unchanged |

`no_log_path_recorded_is_not_private`'s name suggested it covered the "no path recorded" case; it
actually leaves `TRACING_LOG_TARGET_IS_PRIVATE` false and exercises the FIRST clause. The second
clause was uncovered.

The table is now in the function's own doc comment, so a future edit adding a fail-secure branch
without a test is visible in review — the mechanism that would have caught WR-23 two rounds ago,
when the rule was written as prose and nothing checked it against the code.

## Verification

| Gate | Command | Result |
|---|---|---|
| Module | `cargo test -p nono-sandbox-cli --bin nono --no-fail-fast -- --test-threads=1 cli_bootstrap::` | **11 passed / 0 failed** (was 5) |
| Format | `cargo fmt --all -- --check` | clean |
| Cross-target clippy (linux-gnu) | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |
| Cross-target clippy (apple-darwin) | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | **PASS** — exit 0, 0 warnings, 0 errors |

`cli_bootstrap.rs` carries `#[cfg(target_os = "windows")]` blocks, so CLAUDE.md's cross-target MUST
applies; both gates run locally, neither deferred.
