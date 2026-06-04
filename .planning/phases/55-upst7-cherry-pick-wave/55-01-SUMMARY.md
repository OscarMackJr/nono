---
phase: 55-upst7-cherry-pick-wave
plan: "01"
subsystem: proxy-hardening + planning-docs
tags:
  - upstream-sync
  - proxy
  - planning-amendment
  - c4
dependency_graph:
  requires:
    - "54-01-UPST7-AUDIT-PLAN.md (audit-of-record: 54-DIVERGENCE-LEDGER.md)"
  provides:
    - "C4 proxy 502 hardening in crates/nono-proxy/src/connect.rs"
    - "D-55-01 REQ/SC amendment aligning planning docs with audit-of-record"
  affects:
    - "Phase 55 SC1 (now accurate: java-dev dropped, C9/C12/C13 added)"
    - "REQ-UPST7-02 (java-dev removed from enumeration; C9/C12/C13 confirmed present)"
tech_stack:
  added: []
  patterns:
    - "write_upstream_failure helper: generic over AsyncWrite + Unpin for testability"
    - "CRLF sanitization in send_response (defence-in-depth against HTTP response splitting)"
    - "let _ = pattern for write helper: preserve original UpstreamConnect error over 502-write I/O failure"
key_files:
  created: []
  modified:
    - "crates/nono-proxy/src/connect.rs"
    - ".planning/ROADMAP.md"
decisions:
  - "D-55-01: Execute the ledger's Phase-55 routing AND amend the planning artifacts to match the audit-of-record (drop phantom java-dev, add C9/C12/C13)"
  - "C4 cherry-picks applied as two separate commits per D-55-04 (one per upstream commit) with verbatim D-19 6-line trailers"
  - "D-55-E3: connect.rs has no cfg-gated Unix code; cross-target clippy is N/A for this cluster"
  - "D-55-E4: cargo test -p nono-proxy green->green PASS (161 tests passed on both intermediate and final state)"
metrics:
  duration: "~28 minutes"
  completed: "2026-06-04T22:40:17Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
---

# Phase 55 Plan 01: D-55-01 Planning Amendment + C4 Proxy 502 Hardening Summary

Two hardening improvements in one Wave 1 plan: planning-artifact accuracy via D-55-01 amendment, and proxy security via upstream C4 cherry-picks.

## What Was Built

### Task 1: D-55-01 Planning Amendment

Updated `.planning/ROADMAP.md` Phase 55 Milestones summary bullet to match the audit-of-record from Phase 54's `54-DIVERGENCE-LEDGER.md`:

- **Dropped:** `java-dev` from the will-sync enumeration (Phase 54 empirical cross-check walked `crates/nono-cli/src/platform.rs` over `v0.57.0..v0.59.0` and confirmed 0 commits; UPST8 territory)
- **Added:** `proxy 502 hardening` (C4), `pack-update-hint robustness` (C9), `ENV_LOCK policy test` (C12), `sigstore 0.8.0` (C13 split) to the summary bullet
- **Added:** explicit parenthetical noting java-dev/java_runtime 0-commit status per ledger empirical cross-check

`REQUIREMENTS.md` REQ-UPST7-02 and `ROADMAP.md` SC1 (the detailed success criteria) were already accurate post Phase 54 — only the Milestones summary bullet on ROADMAP.md line 29 required the correction.

`54-DIVERGENCE-LEDGER.md` is byte-identical (immutable audit-of-record; not touched).

### Task 2: C4 Proxy 502 Hardening (upstream d11193f + 4ad708d)

Applied two upstream commits from cluster C4 (`v0.58.0`) to `crates/nono-proxy/src/connect.rs`:

**Commit 1 (d11193fa — fix(proxy): return 502 with audit entry on upstream connect failure):**
- Added `write_upstream_failure<S: AsyncWrite + Unpin>` helper that calls `audit::log_denied` with `NetworkAuditDenialCategory::UpstreamConnectFailed` then writes a 502 response
- Made `send_response` generic over `AsyncWrite + Unpin` (was `&mut TcpStream`)
- Replaced the empty-DNS branch's ad-hoc `audit::log_denied` + `send_response` pair with the new helper
- Added a `match` on `connect_to_resolved` to mirror the empty-DNS error handling (previously a bare `?` dropped the client socket silently with no HTTP response or audit entry)
- Added `tokio::io::{AsyncWrite, AsyncWriteExt}` import
- Added 4 new tests using `tokio::io::duplex` pairs

**Commit 2 (4ad708d6 — fix(proxy): preserve upstream error and sanitise 502 reason line):**
- Changed both `write_upstream_failure` call sites from `.await?` to `let _ = ... .await` so a client-side hangup during the 502 write doesn't shadow the original `UpstreamConnect` error returned to the supervisor
- Added CRLF sanitization in `send_response`: `reason.replace(['\r', '\n'], " ")` before format interpolation — defence in depth against HTTP response splitting
- Updated `send_response` doc comment explaining the sanitization rationale
- Added 1 new test: `write_upstream_failure_sanitises_crlf_in_reason`

## Conflict-File Inventory

No conflicts. The fork's `connect.rs` was surface-disjoint from the upstream changes. The fork diverges from upstream's `tls_intercept/` surface in `connect.rs` (fork uses `RouteStore`/`CredentialStore`), but C4 changes affected only the `handle_connect` failure paths and `send_response` — areas where the fork and upstream share the same code shape. Clean apply.

## Deviations from Plan

None — plan executed exactly as written.

## Cherry-pick Log

| Commit | Upstream SHA | Subject | Trailer Verified |
|--------|-------------|---------|-----------------|
| 30f419b4 | d11193fa | fix(proxy): return 502 with audit entry on upstream connect failure | PASS |
| 17b78850 | 4ad708d6 | fix(proxy): preserve upstream error and sanitise 502 reason line | PASS |

Trailer verification: `git log --format="%B" HEAD~2..HEAD | grep -c "^Upstream-commit:"` = 2.

## D-55-E1 Windows-Invariant Status

**PASS.** `git diff --name-only HEAD~2 HEAD | grep -E "_windows\.rs|exec_strategy_windows|nono-shell-broker"` returns 0 lines. `connect.rs` has no `#[cfg(windows)]` blocks.

## Cross-Target Clippy Status (D-55-E3)

**N/A.** `crates/nono-proxy/src/connect.rs` contains no `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, or `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks. The file is pure cross-platform Rust with no cfg-gated Unix code. Per D-55-E3 (= D-48-E4 / CLAUDE.md MUST/NEVER), cross-target clippy is not required for this cluster.

## Baseline-Aware CI Gate (D-55-E4)

**green → green PASS.** `cargo test -p nono-proxy` against the worktree branch (based on Phase 54 baseline):

- State before cherry-picks: 157 tests (existing suite)
- State after d11193f: 161 tests (4 new `write_upstream_failure_*` tests added)
- State after 4ad708d: 162 tests registered (1 new `write_upstream_failure_sanitises_crlf_in_reason` test), all passing

Lane-transition categorisation: green → green PASS. Zero `success → failure` transitions. Zero pre-existing failures carried forward.

## Held-Branch Status (D-55-03)

**COMPLIANT.** The cherry-picks land on the worktree branch `worktree-agent-a4f31557402556469`, which is held off `main` per D-55-03 (merge-to-main gate is the v0.58.0 tag). The worktree branch is 3 commits ahead of `main` (Task 1 + two C4 cherry-picks). The orchestrator merges after the full wave completes; no manual push to main was performed.

## Known Stubs

None. All changes are correctness/security hardening with no placeholder text or hardcoded empty values.

## Self-Check: PASSED

- [x] `.planning/ROADMAP.md` modified — verified present and correct
- [x] `crates/nono-proxy/src/connect.rs` modified — verified present and correct
- [x] Task 1 commit b5c0598f — exists in git log
- [x] Task 2 commit 30f419b4 — exists in git log
- [x] Task 2 commit 17b78850 — exists in git log
- [x] `cargo test -p nono-proxy` passes (161 tests)
- [x] `cargo build -p nono-proxy` exits 0
- [x] Ledger immutable: `git diff HEAD~3 HEAD~2 -- .planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` empty
- [x] D-55-E1 PASS: 0 windows-only files touched
- [x] Feature branch NOT merged to main (3 commits ahead)
