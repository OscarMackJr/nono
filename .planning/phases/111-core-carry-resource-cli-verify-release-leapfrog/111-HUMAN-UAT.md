---
status: partial
phase: 111-core-carry-resource-cli-verify-release-leapfrog
source: [111-VERIFICATION.md, 111-REVIEW.md]
started: 2026-08-05
updated: 2026-08-05
---

## Current Test

[awaiting operator accept-or-fix decision on unresolved code-review findings]

## Tests

### 1. CR-01 (Critical) — release-dry-run.ps1 pre-publish regex is unscoped

expected: `scripts/release-dry-run.ps1`'s `PRE_PUBLISH_REGISTRY_BLOCKED` detection should treat
ONLY the expected missing fork-owned package (`nono-sandbox`) as an acceptable pre-publish block.
As committed in `be3d0ecc`, it matches the unanchored substring `no matching package named`, so any
cargo dependency-resolution failure (typo'd dep, yanked crate, wrong version bump) would be
reclassified as an expected non-failure and the gate would still exit 0 — immediately ahead of an
irreversible crates.io publish.

Suggested fix shape: scope the match to the expected package name, e.g.
`no matching package named \`nono-sandbox`` rather than the bare phrase.

Note: the gate is genuinely GREEN today for the correct reason; this is a latent false-negative
risk on a future run, not a current false pass.

result: RESOLVED 2026-08-05 (commit 5cf5e121). Operator elected "fix now". Match scoped to the
fork-owned `nono-sandbox` name family; anything else falls through to the FAIL branch. Gate re-run
confirms the verdict is unchanged: `PASS: Hard failures: 0. Blocked: 2 (pre-publish). Skipped: 1.`
exit 0.

Residual note (pre-existing, NOT introduced by phase 111): the sibling first pattern
`failed to select a version for the requirement` remains unscoped and carries the same class of
false-negative risk. It predates this phase and was left unchanged; scope it the same way if ever
revisited.

### 2. WR-01 (Warning) — MAX_CRYPTO_THREADS widened on Linux for a macOS-only rationale

expected: `crates/nono-cli/src/exec_strategy.rs:171` is compiled for `not(target_os = "windows")`,
so raising `MAX_CRYPTO_THREADS` 7 -> 12 loosens the fork-safety tripwire on Linux as well as macOS,
while upstream #1424's justification (libdispatch workqueue thread flake) is macOS-specific.
Decide whether to accept the uniform widening (matches upstream exactly) or gate the higher
threshold behind `cfg(target_os = "macos")`.

result: [pending]

### 3. WR-02 (Warning) — corrected --timeout help text conflates Linux and macOS mechanisms

expected: `crates/nono-cli/src/cli.rs:2804-2810` describes the macOS `setpgid(0,0)` +
`kill(-pgrp, SIGKILL)` mechanism as applying to Linux too. Linux actually uses `cgroup.kill` and
never calls `setpgid`. The text is not a security overstatement, but it is inaccurate in the same
category of defect CORE-02 existed to fix.

result: [pending]

## Summary

total: 3
passed: 1
issues: 0
pending: 2
skipped: 0
blocked: 0

CR-01 (Critical) resolved in commit 5cf5e121. WR-01 and WR-02 remain open by operator decision —
both are Warning-severity accuracy/tuning items on `exec_strategy.rs` and `cli.rs`, and addressing
them requires re-running both cross-target clippy gates.

## Gaps
