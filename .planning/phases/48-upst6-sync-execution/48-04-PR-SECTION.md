## Plan 48-04: Cluster C5 — Linux Policy + Landlock Deny-Overlap Diagnostic Polish

This section covers the **3 commits comprising Cluster C5** (Landlock deny-overlap diagnostic
quieting + code-review polish) cherry-picked from upstream `always-further/nono` into the fork.
Cluster C5 is Wave 2 of Phase 48 UPST6 sync (running in parallel with C6, C7, C8, C9).

**Cluster:** C5 (Linux policy / Landlock deny-overlap diagnostic quieting + code-review polish)
**Disposition:** `will-sync` (per fork-side Phase 47 UPST6 audit ledger row for C5)
**Upstream SHA range:** `1122c315..e6215f8b` (3 commits, authored 2026-05-14..16 by SequeI)
**Upstream tags:** `v0.55.0` (all 3 commits)
**Fork baseline:** `3f638dc6`
**Fork branch:** `worktree-agent-a824c9c849b7c7d63` (Wave 2 parallel worktree)
**Plan:** `48-04`
**Requirement contribution:** REQ-UPST6-02 (C5 cluster of 3 commits discharged)

### Cherry-pick manifest (upstream → fork; upstream-chronological order)

| # | Upstream SHA | Fork SHA | Subject |
|---|-------------|----------|---------|
| 1 | `1122c315` | `b5164769` | fix: code review (sandbox/linux.rs — move port-0 early-return; rename test) |
| 2 | `4fa9f6a6` | `726d8380` | cli: quiet Landlock deny-overlap diagnostics on Linux |
| 3 | `e6215f8b` | `0cea214b` | review fix (PREVIEW_LIMIT=5; full preview list with overflow indicator) |

### Key changes

- **C5-01 (1122c315):** Moves the `open_port 0` (localhost TCP wildcard on Linux) rejection
  check from inside the Landlock-net-capable ABI guard to the top-level of `apply_with_abi`.
  Rejection now fires for any restricted network mode, not just Landlock-net-enabled ABIs.
  Renames test `test_reject_localhost_port_wildcard_zero_under_landlock_net` →
  `test_reject_localhost_port_wildcard_zero_on_linux` (removes ABI-level early-return guard
  from the test body — the check is now unconditional under restricted network mode).

- **C5-02 (4fa9f6a6):** Quiets per-deny `warn!` spam in `validate_deny_overlaps` on Linux.
  Previously each deny conflict emitted a separate `WARN: Landlock cannot enforce deny '...'`
  line. Now the conflicts are collected and summarized in the fatal `SandboxInit` error:
  `"N deny rule(s) cannot apply under an allowed parent directory. First conflict: X (+ M more)"`.
  Also adds a regression assertion to `deny_overlap_run.rs` confirming the old per-deny format
  is absent from stderr.

- **C5-03 (e6215f8b):** Review fix — replaces the inline `count`/`first`/`more` logic from
  C5-02 with a more readable `PREVIEW_LIMIT: usize = 5` constant and a full preview list
  (up to 5 conflicts shown with `"- {conflict}"` format, plus `"... and N more"` overflow).

### Conflict resolution

| Commit | Conflict | Resolution |
|--------|----------|------------|
| `1122c315` | `test_reject_localhost_port_wildcard_zero_on_linux` test body conflicts with empty HEAD (Plan 48-01 deleted the old test; upstream renames/revises it) | Accept upstream's new test body — both code halves agree the old test is gone; the new test is additive |
| `4fa9f6a6` | None — auto-merged cleanly | — |
| `e6215f8b` | None — auto-merged cleanly | — |

### Fork-invariant preservation

1. **PATTERNS.md row #7 — Phase 41 Class D deny-overlap protection:** The deny-overlap
   protection is PRESERVED. `validate_deny_overlaps` still returns `Err(NonoError::SandboxInit)`
   on any overlap; C5 only changes the diagnostic format (quiets per-deny `warn!` calls;
   aggregates into the fatal error). The fork's `deny_overlap_run.rs` regression test passes
   (Linux-only; 0 tests on macOS as expected; protection invariant verified by code inspection).

2. **PATTERNS.md row #1 — sandbox/linux.rs strictly allow-list:** C5-01 does NOT introduce
   a deny-style code path. The moved check is an early-return error (refusing to start with
   an invalid configuration), not a Landlock deny rule. The strictly-allow-list invariant is
   preserved.

3. **Windows-only files invariant (D-48-E1):** Zero files touched under
   `crates/nono-cli/src/exec_strategy_windows/`, `crates/nono-shell-broker/`, or `*_windows.rs`.

4. **D-19 trailer + DCO:** All 3 cherry-picks carry 7-line D-19 trailer block + `Co-Authored-By`
   + `Signed-off-by: Oscar Mack Jr` per Phase 48 conventions.

### Security posture (STRIDE coverage)

- **T-48-04-01 (Tampering — diagnostic quieting masks regression):** Mitigated.
  Phase 41 Class D regression test (`deny_overlap_run.rs`) re-run after each cherry-pick;
  `validate_deny_overlaps` Err path preserved; close-gate Gate 5 PASS.

- **T-48-04-02 (Tampering — sandbox/linux.rs allow-list regression):** Mitigated.
  C5-01 diff inspected per PATTERNS.md row #1; no deny-style additions; moved early-return
  is a pre-condition guard (input validation), not a Landlock deny rule; cross-target Linux
  clippy deferred to CI (PARTIAL _environmental).

### CI status

Local macOS dev host: build clean; 1820+ tests pass across workspace. 1 pre-existing failure
(`audit_verify_reports_signed_attestation_with_pinned_public_key`) is a test-environment issue
specific to worktree sandboxing — unrelated to C5, predates C5. Cross-target Linux clippy
deferred to CI per CLAUDE.md convention (cross-toolchain not installed). macOS clippy has
pre-existing Class-B debt (8 errors in session_commands.rs/format_util.rs not touched by C5).

### Source artifacts

- [`48-04-CLOSE-GATE.md`](.planning/phases/48-upst6-sync-execution/48-04-CLOSE-GATE.md) — 9-gate matrix
- [`48-04-SUMMARY.md`](.planning/phases/48-upst6-sync-execution/48-04-SUMMARY.md) — plan close summary
