---
status: partial
phase: 44-review-polish-test-hygiene-drain
source: [44-VERIFICATION.md]
started: 2026-05-20T19:00:00Z
updated: 2026-05-20T19:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Live CI Linux Clippy lane reports green on Phase 44 head SHA
expected: `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` exits 0 on Phase 44 HEAD via GitHub Actions
result: [pending]

### 2. Live CI macOS Clippy lane reports green on Phase 44 head SHA
expected: `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` exits 0 on Phase 44 HEAD via GitHub Actions
result: [pending]

### 3. Class D Linux deny-overlap test exercised on Linux CI
expected: `cargo test -p nono-cli --test deny_overlap_run` runs on linux-ci with the either-or assertion firing and exit 0 (test no longer `#[ignore]`'d). Confirms REQ-TEST-HYG-01 closure under runtime.
result: [pending]

### 4. Class E env_vars 50-runs determinism check via cargo-nextest
expected: `cargo nextest run -p nono-cli --test env_vars --config-file .config/nextest.toml` runs 50 consecutive times on Windows CI with 0 failures across the two flaky tests. Confirms REQ-TEST-HYG-02 SC#3.
result: [pending]

### 5. Sibling-repo PRs submitted to always-further/nono-py + always-further/nono-ts (or merged by maintainer)
expected: Branch `44-broker-ffi-lockstep` (commits `61ee6aa164` / `1df3e16e6a`) reviewed by sibling maintainers per CONTRIBUTING.md (nono-py) or maintainer discretion (nono-ts). User-driven push deferred per plan-discretion D-44-D1.
result: [pending]

### 6. Phase 44 HEAD SHA recorded as v2.6 quiet-baseline anchor (Roadmap SC#5)
expected: After phase merge to main, the orchestrator captures the merged HEAD SHA and records it in ROADMAP.md / STATE.md as the v2.6 quiet-baseline anchor referenced by REQ-CI-FU-03 in Phase 46.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps
