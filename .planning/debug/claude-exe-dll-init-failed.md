---
slug: claude-exe-dll-init-failed
status: awaiting_human_verify
trigger: |
  "  nono run --profile claude-code -- claude --version"  command errors:  Claude.exe Application error.  The application was not able to start correctly
created: 2026-05-26
updated: 2026-05-26
---

# Debug Session: claude-exe-dll-init-failed

## Symptoms

- **Expected behavior:** `nono run --profile claude-code -- claude --version` launches Claude.exe inside the sandbox and prints the Claude version, exiting cleanly.
- **Actual behavior:** Claude.exe shows an "Application error" dialog: "The application was not able to start correctly." Process fails to initialize.
- **Error message / code:** `0xc0000142` (STATUS_DLL_INIT_FAILED) — a dependent DLL failed to initialize during process startup.
- **Baseline:** `claude --version` works correctly OUTSIDE nono (normal shell, no sandbox). Failure occurs only when wrapped by `nono run`.
- **Timeline:** Worked before under nono; recently broke. Suspected correlation with recent workspace version bump 0.53.1 → 0.57.0 (UPST6 sync / Phase 48 in progress on Windows backend).
- **Reproduction:** `nono run --profile claude-code -- claude --version` on Windows 11.
- **Platform:** Windows 11 Enterprise (win32). Windows sandbox backend (Job Objects / Integrity Level / WFP).

## Current Focus

- hypothesis: STATUS_DLL_INIT_FAILED (0xc0000142) under nono is caused by the Windows low-IL / restricted-token backend blocking a path or registry/DLL resource that Claude.exe (Electron/Node) needs at DllMain time. The "0.53.1 -> 0.57.0 version bump" correlation is likely a red herring (that commit only touched Cargo.toml/Cargo.lock — no behavior change). The real change is the Windows IL mandatory-label / restricted-token work that landed in this window.
- test: examine Windows launch.rs / restricted_token.rs / labels_guard.rs code paths and the claude-code profile capability set; identify what resource the low-IL child cannot access.
- expecting: a path/registry/IL drop that an Electron app needs at startup but the sandbox now denies.
- next_action: ROOT CAUSE CONFIRMED. FIX DECISION MADE 2026-05-26 — user selected Option 1 (extend Phase 31 Low-IL broker to the non-PTY `nono run` path) AND scoped it as a NEW MILESTONE v2.7 (v2.6 shipped 2026-05-25). Milestone v2.7 "Windows supervised-run hardening" created 2026-05-26 (REQ-WSRH-01..06): Phase 51 (no-PTY Low-IL broker mode + select_windows_token_arm routing + NO_WRITE_UP write-deny preservation + CI/cross-target sweep) and Phase 52 (Windows-host HUMAN-UAT repro A/B matrix + windows-poc-handoff.mdx doc update). This debug session stays open (unfixed) until Phase 52 confirms reproduction B (`nono run --profile claude-code -- claude --version`) prints the version and exits 0. Next: /gsd:plan-phase 51.
- reasoning_checkpoint:
    hypothesis: "`nono run --profile claude-code -- claude --version` selects WindowsTokenArm::WriteRestricted (non-detached, non-PTY, session_sid=Some). The synthetic restricting SID S-1-5-117-* double-gates every WRITE-type access check. claude.exe (a 234 MB self-contained native binary, replaced 2026-05-24) performs WRITE-type accesses during DllMain/bootstrap (NtCreateSection SECTION_MAP_WRITE on \\BaseNamedObjects, named-object create, temp DLL extraction). Those are denied against the restricting SID → DllMain returns FALSE → loader exits with STATUS_DLL_INIT_FAILED (0xC0000142)."
    confirming_evidence:
      - "select_windows_token_arm cascade + launch_runtime.rs:359 (interactive_pty=false for run) deterministically routes claude --version to WriteRestricted."
      - "Resolved session nono-shell-status-dll-init-failed.md lines 113-114 + 480 document this EXACT mechanism: WRITE_RESTRICTED restricting-SID denies CLR/heavy-runtime DllMain WRITE-type accesses → 0xC0000142; cmd.exe (no heavy runtime) survives (Phase 15 Row C PASS)."
      - "`where.exe claude` → C:\\Users\\OMack\\.local\\bin\\claude.exe, 234 MB PE32+ console exe, mtime 2026-05-24 — a self-contained heavy-runtime binary that replaced the prior lighter launcher inside the 'recently broke' window."
      - "nono Windows token/label code did NOT change in 0.53.1..0.57.0 (git log confirms) — the version-bump correlation is coincidental; the external claude.exe shape change is the trigger."
    falsification_test: "If `nono run --profile claude-code -- cmd /c \"echo hi\"` ALSO fails 0xC0000142 on this host, the WriteRestricted token is not the differentiator and the hypothesis is wrong. (Phase 15 Row C predicts cmd PASSES.) Conversely, running claude.exe under a null or Low-IL primary token (no restricting SID) should NOT produce 0xC0000142."
    fix_rationale: "The root cause is the WRITE_RESTRICTED restricting-SID double-gate, not Low-IL labels. The Phase 31 broker (Low-IL primary token + inherited console, NO restricting SID) was production-validated to run heavy-runtime children (PowerShell/CLR) cleanly. Routing the non-PTY `nono run` path through an equivalent Low-IL/broker mechanism removes the restricting-SID write-gate while preserving write-deny via mandatory-label NO_WRITE_UP — addressing the cause, not the symptom. A null-token fallback would address the symptom but regress write protection (rejected in history)."
    blind_spots: "(1) The broker currently requires --inherit-handle (ConPTY pipes) and a PTY; a non-PTY broker mode (inherited console or pipe stdio) is not yet wired and needs design. (2) Have not field-confirmed claude.exe runs clean under a Low-IL primary token specifically (only PowerShell/CLR was validated). (3) Cannot reproduce the 0xC0000142 from this agent without spawning under nono on Windows; relying on documented matrix + deterministic code path. (4) Whether claude.exe needs WRITE access to a granted path (e.g. its temp-extraction dir) that the Low-IL label would also deny — separate from the token gate."

## Evidence

- timestamp: 2026-05-26
  checked: git show 6cf7e85c (the suspected 0.53.1->0.57.0 version bump) + git log of all commits since 3780515e (0.53.1)
  found: The version bump commit 6cf7e85c ONLY touched Cargo.toml (5 crates) + Cargo.lock — pure version string change, no code/behavior change. However, the commit window 0.53.1..0.57.0 contains substantial Windows Integrity-Level work: il-label-apply-access-denied debug session, WRITE_OWNER pre-flight fixes (260522-v14, 260522-wn0), mandatory-label apply changes, Phase 50 IL backend.
  implication: The version bump is a red herring for the regression. The behavioral change is in the Windows IL / restricted-token / mandatory-label code. Focus there, not on the bump.

- timestamp: 2026-05-26
  checked: crates/nono-cli/src/exec_strategy_windows/launch.rs — select_windows_token_arm() cascade + the WRITE_RESTRICTED arm doc comments; restricted_token.rs.
  found: 0xC0000142 (STATUS_DLL_INIT_FAILED) is a DOCUMENTED, KNOWN failure mode in this codebase. It is the exact symptom that drove Phase 30 (PTY path) and Phase 15 (detached path) to AVOID the WRITE_RESTRICTED + session-SID token. For a non-interactive, non-detached `nono run` (which `claude --version` is), the cascade selects WindowsTokenArm::WriteRestricted → CreateProcessAsUserW(restricted_token). That arm is documented to work for plain console apps (cmd/powershell) but to trigger 0xC0000142 under specific process shapes (ConPTY, DETACHED_PROCESS).
  implication: Claude on Windows is an Electron/Node app. The launcher `claude` spawns a grandchild Claude.exe (Electron). The WRITE_RESTRICTED token inherited by that Electron grandchild is the prime suspect for the DllMain init failure — same bug class Phase 15/30/31 already fought, but on the plain (non-PTY, non-detached) WriteRestricted arm that those phases left in place.

- timestamp: 2026-05-26
  checked: crates/nono-cli/data/policy.json claude-code profile (lines 656-729) + git log of policy.json, restricted_token.rs, labels_guard.rs.
  found: (1) The claude-code profile is Unix-centric — groups are claude_code_macos / claude_code_linux / vscode_macos / vscode_linux etc.; there is NO claude_code_windows group. Filesystem allow paths are $HOME/.claude, $HOME/.cache/claude (Unix layout). The Windows Claude Electron install dir + its temp DLL-extraction dir are NOT in the grant set. (2) The WRITE_RESTRICTED fix (e094994d) and AppliedLabelsGuard (3ad4f64f / da25619b) are OLD (Phase 19/21) — NOTHING in the Windows token/label/policy code changed in the 0.53.1..0.57.0 window. labels_guard.rs even carries a regression test explicitly named for "the claude-code profile on Windows".
  implication: The Windows backend CODE did not change in the suspected window. Two live possibilities for "worked before, broke now": (A) external — Claude Code auto-updated to an Electron build that loads a DLL the WRITE_RESTRICTED token now blocks at DllMain; (B) the interactive-detection / PTY-allocation logic changed in the 0.53.1..0.57.0 window (Phase 48 startup-timeout + interactive-detection work) and now routes `claude --version` differently. Need to inspect should_allocate_pty / interactive detection history next.

- timestamp: 2026-05-26
  checked: supervised_runtime.rs should_allocate_pty() + launch_runtime.rs:359 (interactive_pty for `run`) + command_runtime.rs:153 (interactive_pty for `shell`); git history of supervisor.rs/mod.rs in the 0.53.1..0.57.0 window.
  found: For `nono run` the LaunchPlan hardcodes interactive_pty=false (launch_runtime.rs:359). The profile's "interactive": true does NOT propagate to PTY allocation for `run` (only `nono shell` sets interactive_pty=true). On Windows should_allocate_pty = session.interactive_pty, so `nono run -- claude --version` allocates NO PTY. The cascade => select_windows_token_arm(is_detached=false, has_pty=false, has_session_sid=true) => WindowsTokenArm::WriteRestricted. The only commits touching the Windows supervisor/mod files in-window are b6a88fea (Linux af_unix) and 4a60f675 (ApprovalDecision enum rename) — neither alters PTY/token routing. Possibility (B) ELIMINATED.
  implication: `claude --version` definitively runs under CreateProcessAsUserW with the WRITE_RESTRICTED + session-SID restricted token, no PTY. This is the same token shape Phase 15 Row C proved works for cmd.exe but the resolved nono-shell-status-dll-init-failed.md proved FAILS (0xC0000142) for heavy-runtime children (CLR / ConPTY). Focus shifts to possibility (A): what claude resolves to and whether it changed.

- timestamp: 2026-05-26
  checked: `where.exe claude` + `ls -la` + `file` on the resolved program on THIS host.
  found: `claude` resolves to C:\Users\OMack\.local\bin\claude.exe — a 234 MB PE32+ console executable (MZ header confirmed), last modified 2026-05-24 15:53 (squarely inside the "recently broke" window). This is NOT a node/cmd shim — it is a single self-contained native binary (Node SEA / Bun-compiled / embedded-runtime class; strings scan returns no plaintext runtime markers, consistent with a packed/compressed embedded runtime). resolve_program() (exec_strategy_windows/mod.rs:105 which::which) returns this .exe and launch.rs spawns it directly under the restricted token.

- timestamp: 2026-05-26
  checked: ORCHESTRATOR FALSIFICATION TEST on the live Windows host (nono v0.57.0, target/release/nono.exe). Ran a trivial system binary under the IDENTICAL sandbox/profile the failing claude command uses.
  found: `nono run --profile claude-code --allow-cwd -- C:\Windows\System32\whoami.exe` → sandbox applied ("Applying sandbox..." + mandatory-label guard warnings emitted), whoami.exe LAUNCHED SUCCESSFULLY (its DllMain initialized; it reached main()), then printed "ERROR: Access is denied." x2 and exited 1 — the restricted token denying whoami's token/SID-lookup APIs (expected WRITE_RESTRICTED behavior). It did NOT produce 0xC0000142. (Two earlier attempts — bare `cmd /c echo hi` and the same with --allow-cwd — were stopped by pre-sandbox Windows path-policy gates: "execution directory outside supported allowlist" and "filesystem policy does not cover absolute path argument: C:\", which are unrelated argument-validation gates, not the DLL-init failure.)
  implication: CONFIRMS the falsification_test prediction. A normal-shape executable survives the WRITE_RESTRICTED + restricting-SID token (DllMain succeeds; only privileged runtime ops are denied). The 234 MB heavy-runtime claude.exe fails at DllMain (0xC0000142) under the SAME token. The differentiator is process shape (heavy DllMain write-type activity), not nono being broken for all children. Diagnosis upheld. Test B (claude.exe itself) was NOT re-run by the orchestrator because it pops a blocking WER modal and the user already reported its 0xC0000142 outcome.
  implication: ROOT CAUSE CONFIRMED. (1) WHAT CHANGED: claude.exe was replaced on 2026-05-24 with a large self-contained native binary that performs heavy DllMain/bootstrap-time initialization (embedded-runtime memory mapping, writable named sections, temp extraction). The prior `claude` (npm/node-launcher era) did lighter init and survived the WRITE_RESTRICTED token. (2) WHY IT FAILS UNDER nono: nono runs it via WindowsTokenArm::WriteRestricted. The synthetic restricting SID S-1-5-117-* double-gates every WRITE-type access check. The embedded runtime's init issues WRITE-type accesses (NtCreateSection SECTION_MAP_WRITE on \BaseNamedObjects, named-object create, temp DLL extraction) → STATUS_ACCESS_DENIED against the restricting SID → DllMain/bootstrap returns FALSE → loader exits with STATUS_DLL_INIT_FAILED (0xC0000142). This is the exact CLR-class mechanism documented at nono-shell-status-dll-init-failed.md lines 113-114, now realized for the native claude.exe. The nono code did NOT regress; an external dependency (claude.exe) changed shape and exposed the long-known WRITE_RESTRICTED brittleness on the `nono run` non-PTY supervised path.

## Eliminated

(none yet)

## Resolution

- root_cause: |
    `nono run --profile claude-code -- claude --version` runs claude.exe under WindowsTokenArm::WriteRestricted
    (CreateProcessAsUserW with a WRITE_RESTRICTED token carrying the synthetic per-session restricting SID
    S-1-5-117-*). That SID is absent from every object DACL on the system, so the token's second access check
    denies ALL write-type operations. `claude` on this host now resolves to C:\Users\OMack\.local\bin\claude.exe —
    a 234 MB self-contained native binary (rebuilt 2026-05-24, inside the "recently broke" window) whose
    DllMain/bootstrap performs WRITE-type accesses (NtCreateSection SECTION_MAP_WRITE on \BaseNamedObjects,
    named-object creation, embedded-runtime temp extraction). Those writes are denied against the restricting SID,
    DllMain returns FALSE, and the loader exits the process with STATUS_DLL_INIT_FAILED (0xC0000142).

    This is the SAME mechanism the resolved session nono-shell-status-dll-init-failed.md documented for CLR/heavy
    runtimes (lines 113-114). The nono Windows token/label code did NOT change in 0.53.1..0.57.0 — the version-bump
    correlation is coincidental. The trigger is the EXTERNAL claude.exe binary changing from a lightweight launcher
    (which survived WRITE_RESTRICTED) to a heavy self-contained runtime (which does not). Plain console apps like
    `cmd /c echo` still pass under this exact token (Phase 15 Row C), confirming the differentiator is the
    heavy-runtime DllMain write activity, not the nono path.

- fix: |
    DECIDED 2026-05-26: Option 1 selected by user. The fix is deferred to a dedicated roadmap phase
    (scoped via plan-phase) — NOT applied inline because it is ~Phase-31-class architectural work
    (a no-PTY Low-IL broker mode + Windows field validation). This debug session remains open until
    that phase lands and reproduction B (`nono run --profile claude-code -- claude --version`) prints
    the version and exits 0.

    NOT YET APPLIED — fix shape required a user decision (see CHECKPOINT). The root cause is the WRITE_RESTRICTED
    restricting-SID write-gate, which is structurally incompatible with heavy-runtime children. Ranked options:

    Option 1 (RECOMMENDED, security-preserving, larger): Route the non-PTY `nono run` supervised path through a
    Low-IL primary token instead of WRITE_RESTRICTED for the affected case — i.e. extend the Phase 31 broker
    mechanism (or a no-PTY broker mode) to the non-PTY path. Low-IL primary token has NO restricting SID, so
    heavy-runtime DllMain writes succeed; write-deny is preserved via mandatory-label NO_WRITE_UP (the broker's
    PowerShell/CLR child was production-validated, Postscript 2). Needs a no-PTY console-inherit/pipe-stdio broker
    mode (broker currently requires --inherit-handle PTY pipes) + Windows field validation. ~Phase-31-class effort.

    Option 2 (smallest, regresses write protection — matches the Phase 15 detached waiver): null-token the non-PTY
    `nono run` path. claude.exe would launch (no restricting SID, Medium IL), but the child loses WRITE_RESTRICTED
    write-deny AND mandatory-label write-deny (Medium-IL child dominates Medium-IL files). Job Object + CapabilitySet
    + AppID WFP remain. This was explicitly rejected on the interactive path in history as a real regression; on a
    one-shot `claude --version` the exposure is narrower but it still weakens the documented `nono run` security model.

    Option 3 (no nono change): document that the native claude.exe is unsupported under `nono run` on Windows until
    Option 1 lands; advise the user to use the node-launcher form of Claude Code (lighter DllMain) if available, or
    run `claude --version` outside the sandbox (version check is not a sensitive operation).

- verification: |
    Pre-fix reproduction confirmation the user can run NOW to validate the diagnosis (predicted by the matrix):
      A. `nono run --profile claude-code -- cmd /c "echo hi"`  → predicted PASS (prints hi, exit 0)
         [if this FAILS 0xC0000142, the hypothesis is wrong].
      B. `nono run --profile claude-code -- claude --version`  → reproduces 0xC0000142 (the bug).
    Post-fix (Option 1 or 2): B prints the Claude version and exits 0; A still passes.

- files_changed: []
