---
phase: 33-windows-parity-upstream-0-52-divergence
plan: 00
wave: 0
status: complete
completed: 2026-05-11
---

# Phase 33 Plan 00 — Wave 0 Prep SUMMARY

## Provenance captured (D-33-A2 inputs for Wave 1 ledger header)

- **drift-tool-sh-sha:** `0834aa664fbaf4c5e41af5debece292992211559` (`scripts/check-upstream-drift.sh`)
- **drift-tool-ps1-sha:** `0834aa664fbaf4c5e41af5debece292992211559` (`scripts/check-upstream-drift.ps1` — same commit moved both files)
- **upstream-head-sha:** `54f7c32a315dabe56cf0530e8ea6bdc44985122d` (`upstream/main` at audit time)
- **audit-date:** `2026-05-11` (UTC)
- **drift-tool-invocation:** `make check-upstream-drift ARGS="--from v0.40.1 --to v0.52.0 --format json"` (D-33-A1 LOCKED verbatim)
- **smoke-test total_unique_commits:** `97` (sets Wave 1 ledger sizing expectation; row count MUST be ≥ 97 per REQ-1 acceptance)

**Host note:** On the Windows audit host, `make` is not on PATH; the smoke test was executed via the dispatched POSIX script `bash scripts/check-upstream-drift.sh --from v0.40.1 --to v0.52.0 --format json` (same shell command the Makefile target dispatches to). The locked invocation above is recorded verbatim because D-33-A1 lock points at the make wrapper.

## Wave 0 prerequisites verified (per 33-VALIDATION.md)

- [x] `git fetch upstream --tags` — v0.43.1..v0.52.0 fetched locally (12 new tags including v0.52.0)
- [x] `git remote -v | grep upstream` — points at `https://github.com/always-further/nono.git`
- [x] `make check-upstream-drift --help` — succeeds (via `bash scripts/check-upstream-drift.sh --help` on Windows host)
- [x] Drift-tool dry-run with locked invocation — exits 0, well-formed JSON with `total_unique_commits` and `range` keys

## Smoke-test sizing breakdown

- range: `v0.40.1..v0.52.0`
- by_category: `profile=15, policy=5, package=5, proxy=6, audit=4, other=91`
- total_unique_commits: `97`

## RESEARCH Open Questions resolved (operator decision 2026-05-11)

- **project-md-target:** `3-column Key Decisions table at PROJECT.md L158-183 (append one row)` — Wave 3 Task (REQ-3) writes the row at this target.
- **adr-commit-pattern:** `Accepted-first-commit (single commit for the ADR in Wave 2)` — Wave 2 Task (REQ-2) commits the ADR with `**Status:** Accepted` from the first commit.

## Handoffs to downstream waves

- **Wave 1 (Plan 33-01):** reads `drift-tool-sh-sha`, `upstream-head-sha`, `audit-date`, `drift-tool-invocation` into the DIVERGENCE-LEDGER.md header (D-33-A2). Reads `smoke-test total_unique_commits` (97) to size the ledger walk — total cluster row count MUST be ≥ 97.
- **Wave 2 (Plan 33-02):** reads `adr-commit-pattern: Accepted-first-commit` to ship the ADR in a single commit with `**Status:** Accepted`.
- **Wave 3 (Plan 33-03):** reads `project-md-target: 3-column Key Decisions table at L158-183` to lock REQ-3 row destination.
