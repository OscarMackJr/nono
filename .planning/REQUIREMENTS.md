# Requirements: nono — v2.7 Windows supervised-run hardening

**Defined:** 2026-05-26
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms; every nono command that works on Linux/macOS should work on Windows with equivalent security guarantees, or be explicitly documented as intentionally unsupported with a clear rationale.

**Trigger:** Root cause confirmed 2026-05-26 in `.planning/debug/claude-exe-dll-init-failed.md`. `nono run --profile claude-code -- claude --version` fails with `STATUS_DLL_INIT_FAILED (0xC0000142)` because the non-PTY supervised path routes the new 234 MB self-contained `claude.exe` through `WindowsTokenArm::WriteRestricted`, whose synthetic restricting SID `S-1-5-117-*` double-gates the heavy WRITE-type DllMain/bootstrap activity (`NtCreateSection SECTION_MAP_WRITE` on `\BaseNamedObjects`, named-object create, temp DLL extraction). nono's Windows token/label code did NOT regress in 0.53.1..0.57.0 — the external `claude.exe` changed shape (rebuilt 2026-05-24) and exposed the long-known `WRITE_RESTRICTED` brittleness on the non-PTY `nono run` path. Plain console apps (`cmd`/`echo`) still pass under the same token; the differentiator is heavy DllMain WRITE-type activity. User selected **Option 1** (security-preserving Low-IL broker extension) over Option 2 (null-token, regresses write-deny) and Option 3 (document-only).

## v1 Requirements (v2.7 Scope)

### Windows Supervised-Run Hardening (WSRH)

- [ ] **REQ-WSRH-01**: The `nono-shell-broker` can launch a child process **without** a ConPTY/PTY, inheriting stdio via the parent console or anonymous pipes. Today the broker requires `--inherit-handle` ConPTY pipes + a PTY (Phase 31 `BrokerLaunch` arm); a no-PTY broker mode must be wired so non-PTY callers can route through it.
- [ ] **REQ-WSRH-02**: The non-PTY `nono run` supervised path routes through the broker's **Low-IL primary token (no synthetic restricting SID)** for the affected case, instead of `WindowsTokenArm::WriteRestricted`. The `select_windows_token_arm` cascade (`exec_strategy_windows/launch.rs`) is extended so a non-detached, non-PTY, session-SID launch of a heavy-runtime child dispatches to the broker/Low-IL arm rather than the restricting-SID arm.
- [ ] **REQ-WSRH-03**: The Low-IL child retains mandatory-label `NO_WRITE_UP` write-deny enforced at the OS level (kernel MIC pre-DACL check), parity with the Phase 31 broker's production-validated PowerShell/CLR guarantee. A regression test asserts a write attempt by the Low-IL child to a Medium-IL-labeled path is denied. No regression to the documented `nono run` security model.
- [ ] **REQ-WSRH-04**: `nono run --profile claude-code -- claude --version` launches the self-contained `claude.exe` with **no `0xC0000142`** (DllMain/bootstrap succeeds), prints the Claude version, and exits 0 on a Windows 11 host.
- [ ] **REQ-WSRH-05**: No regression to existing paths — plain console apps (`nono run --profile claude-code -- cmd /c "echo hi"`) still pass; the existing `nono shell` broker path and the detached path are unchanged; Windows CI lanes remain green; cross-target Linux/macOS clippy is clean per the CLAUDE.md § Coding Standards MUST/NEVER enforcement bullet + `.planning/templates/cross-target-verify-checklist.md`.
- [ ] **REQ-WSRH-06**: Windows runtime field validation (HUMAN-UAT) — the reproduction matrix is executed on a real Windows host with recorded verdicts: **A** (`cmd /c "echo hi"`) passes; **B** (`claude --version`) prints the version and exits 0 post-fix; the `windows-poc-handoff.mdx` doc is updated to reflect the supported `nono run` behavior for heavy-runtime children.

## v2 Requirements (Deferred)

Items acknowledged but not in v2.7 roadmap.

### Broader heavy-runtime audit

- **REQ-WSRH-AUDIT-01** *(deferred)*: Systematic audit of which other built-in profiles / heavy-runtime binaries (Electron/Node/CLR-class) hit the same `WriteRestricted` gate under `nono run`. v2.7 fixes the path for the confirmed `claude.exe` case; a profile-wide audit is a follow-on.

## Out of Scope (Explicit Exclusions)

- **Option 2 null-token fallback** — explicitly rejected. Would let `claude.exe` launch at Medium IL but regress BOTH `WRITE_RESTRICTED` and mandatory-label write-deny on the `nono run` path. Narrower exposure on a one-shot `claude --version`, but still weakens the documented security model. Rejected in history on the interactive path.
- **WR-02 EDR HUMAN-UAT** — v3.0-deferred pending EDR-instrumented runner (re-affirmed every milestone since v2.1).
- **Gap 6b (runtime trust interception via kernel minifilter)** — requires a signed kernel driver; deferred to v3.0.
- **ConPTY resize on the no-PTY path** — the no-PTY broker mode is structurally exclusive of ConPTY (same constraint as the Phase 17 anonymous-pipe attach path, D-07); resize is not in scope.

## Traceability

| REQ-ID | Phase | Status |
|--------|-------|--------|
| REQ-WSRH-01 | TBD (roadmap) | Pending |
| REQ-WSRH-02 | TBD (roadmap) | Pending |
| REQ-WSRH-03 | TBD (roadmap) | Pending |
| REQ-WSRH-04 | TBD (roadmap) | Pending |
| REQ-WSRH-05 | TBD (roadmap) | Pending |
| REQ-WSRH-06 | TBD (roadmap) | Pending |

*(Phase column filled by the roadmapper.)*
