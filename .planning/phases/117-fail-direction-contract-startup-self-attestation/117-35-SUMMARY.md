---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 35
subsystem: docs
tags: [testing, baseline, gap-closure, round-4, no-fail-fast, cint-03]

# Dependency graph
requires: []
provides:
  - "117-BASELINE-ROUND4.md — the dated, untruncated `cargo test --workspace --no-fail-fast`
    capture on HEAD 9ef519cf, before any round-4 code fix landed"
  - "A per-target classification of all 5 failing targets / 19 failing tests, each with evidence
    and an explicit disposition (pre-existing / round-4 regression / UNCLASSIFIED)"
  - "The round's standing verification rule: every round-4 plan asserting suite-level state uses
    --no-fail-fast"
affects: [117-36, 117-37, 117-39, 117-40, 117-41, 117-42, 117-43, 117-44, 117-45]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A test baseline is captured to a file in full, never through a truncating filter — cargo's
      terminal `error: N targets failed:` block is the authoritative verdict and is exactly what
      a `tail`/`head` pipe destroys"
    - "An unrecognized failure is classified explicitly or marked UNCLASSIFIED with its evidence;
      it is never folded into 'pre-existing' because it resembles one"

key-files:
  created:
    - .planning/phases/117-fail-direction-contract-startup-self-attestation/117-BASELINE-ROUND4.md
  modified: []

key-decisions:
  - "The plan's own <objective> predicted `error: 6 targets failed`; this run reports 5. The
    earlier capture was not retained, so the sixth target cannot be identified and the delta is
    recorded as UNRECONCILED rather than explained away. No round-4 code had landed, so this is
    explicitly NOT treated as evidence a target was fixed."
  - "Two env_vars failures are marked UNCLASSIFIED — needs investigation rather than pre-existing.
    env_vars.rs was modified by Phase 117 commit 76432e05 (CR-08), so a 117 relationship is
    plausible and must be ruled in or out by reading, not assumed. Flagged as operator-facing."
  - "The capture script's `=== cargo exit code: 0 ===` line records the exit of a preceding echo,
    not cargo's. Recorded as a correction inside the artifact rather than silently deleted — a
    stray `exit code: 0` in a baseline file is the false-green class this plan exists to close."
  - "nono-wfp-service.exe was found running from target\\release\\; left running. The test run
    builds target\\debug\\, so there was no lock conflict, and stopping a live WFP enforcement
    service is a system-affecting action the plan did not require."
---

# Plan 117-35: Round-4 Test Baseline — Execution Summary

**Executed:** 2026-08-12 · **HEAD:** `9ef519cf` · **Wave:** 15 (runs alone, before any round-4 fix)

## Task 1 — Full `--no-fail-fast` capture

`cargo test --workspace --no-fail-fast` run to completion, stdout+stderr redirected in full to a
scratch file with no truncating filter, then copied verbatim into
`117-BASELINE-ROUND4.md`. Run took **39m29s** (03:37:09Z → 04:16:38Z); the long pole is `env_vars`
at 2061s alone, which spawns real supervised children, several individually over 60s.

Pre-flight per the plan's warning: checked for processes holding `target\debug\nono.exe`. Found
`nono-wfp-service.exe` (PID 94760) — but `Get-CimInstance Win32_Service` shows it runs from
`target\release\nono-wfp-service.exe`, and the test run builds debug artifacts. No conflict, no
build failure, service left running.

**Acceptance criteria met:**
- Artifact contains cargo's `error: 5 targets failed:` block — capture reached the true end of the run.
- Contains `layer_registry_selfcheck` in a FAILED context (7 occurrences).
- Contains `resl_nix_async_signal_safety` in a FAILED context (8 occurrences).
- 4056 lines total; 3 balanced fenced blocks.

## Task 2 — Enumeration and classification

**`error: 5 targets failed` — 19 failing tests.** Every other workspace target passed, including
all `nono` library tests, `nono-proxy`, `nono-ffi`, and all doc-tests.

| Target | Failures | Disposition |
|---|---|---|
| `--bin nono` | 12 | pre-existing (count matches documented 11 + WR-20 exactly) |
| `--test audit_attestation` | 2 | pre-existing — host-environmental (`/bin/pwd` on Windows) |
| `--test env_vars` | 1 (`windows_run_allow_all_network_probe_connects`) | pre-existing — host-environmental (rollback budget: 2,153,433,890 B tracked > 2 GiB limit) |
| `--test env_vars` | 2 (`windows_run_blocks_live_block_net_without_enforcement`, `windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified`) | **UNCLASSIFIED — needs investigation** |
| `--test layer_registry_selfcheck` | 1 | **round-4 regression (CR-04)** — fixed by 117-36 |
| `--test resl_nix_async_signal_safety` | 1 | pre-existing (red since ~Phase 25) |

Row count matches the `error: 5 targets failed:` line; no failing target omitted.

The 12 `--bin nono` names were listed in full and counted rather than accepted: 11 HOME/env-race
failures + WR-20's host-blocked `non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`
= 12. **No new name appeared.**

**Round 4 is responsible for exactly one target:** `layer_registry_selfcheck` (CR-04), fixed by
117-36.

## Verification rule stated for the round

Recorded in the artifact as a standing instruction: every round-4 plan whose `<verify>` asserts
suite-level state MUST use `--no-fail-fast`, because `cargo test` is fail-fast **across targets** —
`--bin nono` fails first and cargo never builds a single `tests/*.rs` binary. That is precisely how
CR-04 shipped through 117-32's plan, 117-34's dependent plan, and 117-34's own full-suite verify.

Audited at capture time: **all 20 `cargo test` commands across 117-36 … 117-45 already carry
`--no-fail-fast`.** No retrofit needed.

## Items for the operator

1. **Two UNCLASSIFIED `env_vars` failures.** Not round-4's responsibility to fix, but not
   launderable into "pre-existing" either. `windows_run_blocks_live_block_net_without_enforcement`
   asserts a preview-era "should block live network restriction requests" refusal while the run
   shows nono proceeding normally — reads as a stale expectation of the same class as the CI
   ubuntu hang's stale seccomp test, but that is not establishable from the captured output alone.
   `windows_run_ignores_unverified_localappdata_override_when_runtime_root_is_verified` fails on a
   protected-path overlap because its fixture nests fake LOCALAPPDATA inside the granted temp dir;
   whether fixture or policy is wrong is undetermined. **Both need a decision before phase 117 closes.**
2. **The 6-vs-5 target discrepancy is unreconciled**, and the earlier capture no longer exists.
3. **`make test` / `make ci` carry the same fail-fast blind spot** — they wrap `cargo test` without
   `--no-fail-fast`. CLAUDE.md treats them as canonical. Fixing the Makefile was out of this plan's
   scope; recorded as a follow-up candidate.
4. **`windows_run_allow_all_network_probe_connects` will keep failing and get worse** — it grants
   `target\debug` r+w and that directory is now 2.15 GB against a 2 GiB rollback budget. A
   `cargo clean` would mask it temporarily; an exclusion pattern would fix it properly.

## Deviations

None affecting the plan's outcome. The capture script recorded a misleading `cargo exit code: 0`
(it read `$?` after an intervening `echo`); corrected in-place inside the artifact with an
explanation rather than deleted.
