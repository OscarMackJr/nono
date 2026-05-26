---
milestone: v2.7
milestone_name: Windows supervised-run hardening
status: active
created: 2026-05-26
granularity: standard
---

# Roadmap — nono

## Milestones

- ✅ **v1.0 Windows Alpha** — Phases 01-12 (shipped 2026-03-31) — see [`milestones/v1.0-*`](milestones/)
- ✅ **v2.0 Windows Gap Closure** — Phases 13-18 — see [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md)
- ✅ **v2.1 Resource Limits / Extended IPC / Attach-Streaming** — see [`milestones/v2.1-ROADMAP.md`](milestones/v2.1-ROADMAP.md)
- ✅ **v2.2 Windows/macOS Parity Sweep** — see [`milestones/v2.2-ROADMAP.md`](milestones/v2.2-ROADMAP.md)
- ✅ **v2.3 Linux POC Unblock + Deferreds Closure** — see [`milestones/v2.3-ROADMAP.md`](milestones/v2.3-ROADMAP.md)
- ✅ **v2.4 Complete the Partial Ports + UPST4** — Phases 35, 36, 36.5, 39, 40 (shipped 2026-05-15) — see [`milestones/v2.4-ROADMAP.md`](milestones/v2.4-ROADMAP.md)
- ✅ **v2.5 Backlog Drain + UPST5** — Phases 37, 41, 42, 43 (shipped 2026-05-20) — see [`milestones/v2.5-ROADMAP.md`](milestones/v2.5-ROADMAP.md)
- ✅ **v2.6 UPST6 + v2.5 Drain** — Phases 44, 44.1, 45, 46, 47, 48, 49, 50 (shipped 2026-05-25) — see [`milestones/v2.6-ROADMAP.md`](milestones/v2.6-ROADMAP.md)

## Phases

### v2.7 Windows supervised-run hardening (Phases 51-52) — ACTIVE

- [ ] **Phase 51: No-PTY Low-IL broker + token routing + write-deny preservation** — Extend the Phase 31 broker with a no-PTY console-inherit/pipe-stdio mode; wire the `select_windows_token_arm` cascade to dispatch heavy-runtime non-PTY `nono run` through the broker/Low-IL arm; assert NO_WRITE_UP mandatory-label write-deny is preserved with a regression test; run the full no-regression sweep (CI + cross-target clippy).
- [ ] **Phase 52: Field validation closure — heavy-runtime HUMAN-UAT + doc update** — Execute the reproduction matrix (A: cmd/echo passes; B: `claude --version` prints version and exits 0) on a real Windows host with recorded verdicts; update `docs/cli/development/windows-poc-handoff.mdx` to reflect supported `nono run` behavior for heavy-runtime children.

## Phase Details

### Phase 51: No-PTY Low-IL broker + token routing + write-deny preservation
**Goal**: The non-PTY `nono run` supervised path launches heavy-runtime children (e.g. `claude.exe`) through a Low-IL primary token with no synthetic restricting SID, eliminating the `STATUS_DLL_INIT_FAILED (0xC0000142)` failure class while preserving mandatory-label `NO_WRITE_UP` write-deny at the OS level — security-preserving broker extension, not a null-token regression.
**Depends on**: Nothing (first phase of v2.7; Windows-only deliberate work — the Windows-only-files invariant D-34-E1 / D-40-E1 / D-43-E1 applies to upstream-sync commits; this milestone INTENTIONALLY touches `*_windows.rs`, `exec_strategy_windows/`, and `crates/nono-shell-broker/`).
**Requirements**: REQ-WSRH-01, REQ-WSRH-02, REQ-WSRH-03, REQ-WSRH-05
**Success Criteria** (what must be TRUE):
  1. `crates/nono-shell-broker/` accepts a `--no-pty` invocation mode (console-inherit or anonymous-pipe stdio) and successfully launches a child process without `--inherit-handle` ConPTY pipes; the broker still lowers a duplicate token to Low-IL and calls `CreateProcessAsUserW` with `EXTENDED_STARTUPINFO_PRESENT` on this path, consistent with the Phase 31 PTY path.
  2. `select_windows_token_arm` in `exec_strategy_windows/launch.rs` dispatches a non-detached, non-PTY, session-SID launch through `WindowsTokenArm::BrokerLaunch` (or an equivalent Low-IL arm) rather than `WindowsTokenArm::WriteRestricted`; the old `WriteRestricted` branch is still reachable for cases that do not match (regression-safe conditional, not a blanket removal).
  3. A regression test asserts that a write attempt by the Low-IL child process to a Medium-IL-labeled path is denied by the OS kernel MIC pre-DACL check; the test passes on Windows and is cfg-gated for non-Windows targets; `NO_WRITE_UP` mandatory-label semantics are not weakened.
  4. `nono run --profile claude-code -- cmd /c "echo hi"` (repro A from the debug session) still passes — the child launches, prints `hi`, and exits 0 — confirming no regression to plain console-app paths.
  5. Cross-target clippy is clean per CLAUDE.md § Coding Standards MUST/NEVER enforcement bullet: `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `--target x86_64-apple-darwin` from the dev host (or verification REQ marked PARTIAL per `.planning/templates/cross-target-verify-checklist.md` if cross-toolchain unavailable); Windows CI lanes (Build, Integration, Regression, Security, Packaging) remain green; existing `nono shell` broker path and detached path produce no new failures.
**Plans**: TBD
**UI hint**: no

### Phase 52: Field validation closure — heavy-runtime HUMAN-UAT + doc update
**Goal**: Confirm on a real Windows 11 host that the Phase 51 implementation eliminates the `0xC0000142` failure for `claude.exe`, record the reproduction matrix verdicts, and update the Windows POC handoff documentation to reflect the new `nono run` behavior for heavy-runtime children.
**Depends on**: Phase 51 (implementation must be code-complete and CI-green before field validation).
**Requirements**: REQ-WSRH-04, REQ-WSRH-06
**Success Criteria** (what must be TRUE):
  1. `nono run --profile claude-code -- claude --version` executes on a Windows 11 host post-fix with NO `0xC0000142` dialog — DllMain/bootstrap of the 234 MB self-contained `claude.exe` succeeds, the Claude version string is printed to stdout, and the process exits 0.
  2. `nono run --profile claude-code -- cmd /c "echo hi"` (repro A) continues to pass on the same host in the same run of the validation matrix — confirming the plain console-app path was not regressed by the Phase 51 changes.
  3. Both reproduction matrix verdicts (A: pass; B: pass) are recorded with timestamps in the Phase 52 HUMAN-UAT artifact; the VERIFICATION.md closes with `status: pass` (not `human_needed`).
  4. `docs/cli/development/windows-poc-handoff.mdx` is updated: the `nono run` section describes that heavy-runtime children (self-contained executables with embedded runtimes) are now supported via the Low-IL broker path; the doc does not claim `nono run` is limited to plain console apps for the claude-code profile.
**Plans**: TBD
**UI hint**: no

## Sequencing Rationale

```
Phase 51 (no-PTY broker + token routing + write-deny + CI sweep)
  └──► Phase 52 (field validation: repro matrix on Windows host + doc update)
```

Phase 51 is code-complete when: the broker supports no-PTY mode, the token cascade dispatches correctly, the mandatory-label regression test passes, and CI is green including cross-target clippy. Phase 52 cannot run until Phase 51 is deployed to the Windows test host — it is purely a live-run validation and documentation phase. Sequential dependency is load-bearing.

The folding decision: REQ-WSRH-04 (heavy-runtime launch passes) and REQ-WSRH-06 (HUMAN-UAT + doc) are co-located in Phase 52 because both require the same Windows host session with the fixed binary. Running them in separate phases would impose an unnecessary context-switch cost with no parallelism benefit.

## Requirement Coverage

6 v2.7 requirements. Every requirement mapped to exactly one phase; zero unmapped; zero double-mapped.

| REQ-ID | Phase | Category |
|--------|-------|----------|
| REQ-WSRH-01 | Phase 51 | WSRH |
| REQ-WSRH-02 | Phase 51 | WSRH |
| REQ-WSRH-03 | Phase 51 | WSRH |
| REQ-WSRH-05 | Phase 51 | WSRH |
| REQ-WSRH-04 | Phase 52 | WSRH |
| REQ-WSRH-06 | Phase 52 | WSRH |

**Coverage: 6/6 ✓**

## Cross-Phase Invariants

These invariants are inherited from prior milestones and remain in force across v2.7:

- **Intentional Windows-only-files work** — This milestone deliberately touches `*_windows.rs`, `exec_strategy_windows/`, and `crates/nono-shell-broker/`. The D-34-E1 / D-40-E1 / D-43-E1 Windows-only-files invariant governs upstream-sync commits, not deliberate Windows feature work. Reviewers should not flag Windows-file edits in Phases 51-52 as invariant violations.
- **Cross-target clippy required** — Phase 51 touches `exec_strategy_windows/launch.rs` and the broker; the cascade enum / shared types may be cross-platform. Run `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin` per CLAUDE.md § Coding Standards MUST/NEVER enforcement bullet + `.planning/templates/cross-target-verify-checklist.md`. Windows-host workspace clippy alone is insufficient.
- **Mandatory-label NO_WRITE_UP non-regression** — The Low-IL primary token (no restricting SID) must be paired with the mandatory-label write-deny mechanism; the Phase 31 broker's production-validated pattern is the reference implementation. A null-token fallback (Option 2) is explicitly rejected and must not be introduced.
- **Phase 31 shell path unchanged** — `nono shell` routes through `BrokerLaunch` via the existing PTY/ConPTY path. Phase 51 adds a no-PTY broker mode as an extension, not a replacement. The PTY broker path must remain byte-behaviorally identical after Phase 51.
- **Fail-closed on error** — Any error in the no-PTY broker mode (handle inheritance failure, token duplication failure, mandatory-label apply failure) must produce a clean nono error with a diagnostic, not a silent fallback to a less-secure token arm.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 51. No-PTY Low-IL broker + token routing + write-deny | 0/TBD | Not started | - |
| 52. Field validation closure — heavy-runtime HUMAN-UAT + doc | 0/TBD | Not started | - |

## References

- `.planning/PROJECT.md` — v2.7 milestone context, trigger, options, explicit deferrals.
- `.planning/REQUIREMENTS.md` — v2.7 requirements (REQ-WSRH-01..06) with acceptance criteria + traceability table.
- `.planning/debug/claude-exe-dll-init-failed.md` — confirmed root cause (STATUS_DLL_INIT_FAILED via WriteRestricted token) + fix decision (Option 1 selected).
- `.planning/milestones/v2.6-ROADMAP.md` — archived v2.6 phase detail (Phases 44-50).
- `.planning/phases/31-broker-process-architecture-shell-01/31-05-SUMMARY.md` — Phase 31 broker production validation (PowerShell/CLR under Low-IL primary token, NO_WRITE_UP enforced).
- `.planning/templates/cross-target-verify-checklist.md` — cross-target clippy verification protocol (mandatory for Phase 51).
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `select_windows_token_arm` cascade; `WindowsTokenArm::BrokerLaunch` arm.
- `crates/nono-shell-broker/` — Phase 31 broker; no-PTY mode extends this crate.
