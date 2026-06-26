---
status: resolved
phase: 97-release-engineering-leapfrog-pipeline-runbook
source: [97-VERIFICATION.md, 97-REVIEW.md]
started: 2026-06-26T13:55:00Z
updated: 2026-06-26T14:10:00Z
---

## Current Test

[resolved — operator chose the 3-crate set]

## Tests

### 1. Publish-set divergence (WR-02) — which crate set is canonical?
expected: One source of truth for the crates.io publish set across release.yml, scripts/release-dry-run.ps1, and RELEASE-RUNBOOK.md before the operator tag push.
detail: |
  Three artifacts currently disagree:
  - .github/workflows/release.yml publish-crates job publishes 3 crates: nono → nono-proxy → nono-cli
  - scripts/release-dry-run.ps1 dry-runs 4 crates (adds nono-shell-broker)
  - RELEASE-RUNBOOK.md Step 4 manual fallback documents 4 crates (adds nono-shell-broker)

  nono-shell-broker has NO `publish = false`, so it is genuinely publishable — but it is a
  bin-only crate (no lib target; `cargo build` emits "ignoring invalid dependency
  nono-shell-broker which is missing a lib target"). A bin-only crate CAN be published to
  crates.io but nothing depends on it as a library.

  Resolution is a one-line change either direction:
  (A) Add a nono-shell-broker publish step to release.yml → 4-crate set everywhere.
  (B) Mark nono-shell-broker `publish = false` and drop it from the dry-run + runbook → 3-crate set everywhere.
result: RESOLVED — operator chose (B), the 3-crate set. nono-shell-broker marked `publish = false`
  (crates/nono-shell-broker/Cargo.toml); removed from scripts/release-dry-run.ps1 ($PublishableCrates)
  and from RELEASE-RUNBOOK.md publish order. Now consistent with release.yml's existing 3-crate
  publish-crates job (nono → nono-proxy → nono-cli). Build clean; release-readiness gate PASS. Commit on branch.

## Summary

total: 1
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
