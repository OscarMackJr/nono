---
status: partial
phase: 45-source-migration-aipc-g-04-resl-native-re-validation
source: [45-VERIFICATION.md]
started: 2026-05-23T00:00:00Z
updated: 2026-05-23T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. REQ-RESL-NIX-04 live workflow run (Phase 46 orchestrator action)
expected: |
  From a host with `gh` CLI access to the repository:
    gh workflow run phase-45-resl-native-host.yml -f gh_runner_os=both
    gh run watch <run-id>

  Both `resl-nix` (ubuntu-24.04) and `resl-darwin` (macos-latest) jobs must produce:
    running 2 tests
    test audit_verify_reports_signed_attestation_with_pinned_public_key ... ok
    test rollback_signed_session_verifies_from_audit_dir_bundle ... ok
    test result: ok. 2 passed; 0 failed; 0 ignored

  Apply SC#3 decision tree per `45-03-NATIVE-RESL-PROTOCOL.md`:
    - Branch (a): both jobs pass → flip REQ-RESL-NIX-04 to VERIFIED
    - Branch (b): gap surfaced → file follow-up todo at `.planning/todos/pending/45-resl-nix-04-host-native-gap-<id>.md`; close REQ as PARTIAL
result: [pending]

### 2. Cross-target Linux/macOS clippy verdict (REQ-PORT-CLOSURE-08 + REQ-AIPC-G04-01)
expected: |
  Verify the GitHub Actions CI runs triggered by the Phase 45 head SHA complete green on Linux Clippy and macOS Clippy lanes:
    cargo clippy --workspace -- -D warnings -D clippy::unwrap_used

  Both lanes (ubuntu + macos) must exit 0. PARTIAL disposition is documented in `45-01-CLIPPY-CROSS-TARGET.md` and `45-02-CLIPPY-CROSS-TARGET.md` per the established 4-precedent host-toolchain limitation pattern (Phase 41 + 43-01b + 44 + 45). The GH Actions Linux Clippy + macOS Clippy lanes are the decisive close signal.

  Phase 46 orchestrator records the CI verdict for both REQs. If both lanes pass → flip cross-target status to VERIFIED. If either fails → surface the clippy error and create a follow-up gap task.
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
