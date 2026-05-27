---
slug: claude-exe-dll-init-failed
status: resolved
trigger: |
  "  nono run --profile claude-code -- claude --version"  command errors:  Claude.exe Application error.  The application was not able to start correctly
created: 2026-05-26
updated: 2026-05-27
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
- next_action: RESOLVED 2026-05-27. Root cause confirmed (WRITE_RESTRICTED restricting-SID double-gate fails the heavy-runtime claude.exe DllMain). Fix shipped as v2.7 milestone "Windows supervised-run hardening": Phase 51 implemented WindowsTokenArm::BrokerLaunchNoPty (no-PTY Low-IL broker token, no restricting SID, write-deny preserved via mandatory-label NO_WRITE_UP); Phase 52 field-validated repro A + repro B both PASS on Win11 build-26200 with the 234 MB self-contained claude.exe — 0xC0000142 confirmed gone. v2.7 / v0.57.2 shipped + pushed + tagged 2026-05-26. Closure criterion (Resolution.verification) satisfied; session moved to resolved/.
- reasoning_checkpoint:
    hypothesis: "`nono run --profile claude-code -- claude --version` selects WindowsTokenArm::WriteRestricted (non-detached, non-PTY, session_sid=Some). The synthetic restricting SID S-1-5-117-* double-gates every WRITE-type access check. claude.exe (a 234 MB self-contained native binary, replaced 2026-05-24) performs WRITE-type accesses during DllMain/bootstrap (NtCreateSection SECTION_MAP_WRITE on \\BaseNamedObjects, named-object create, temp DLL extraction). Those are denied against the restricting SID → DllMain returns FALSE → loader exits with STATUS_DLL_INIT_FAILED (0xC0000142)."
    confirming_evidence:
      - "select_windows_token_arm cascade + launch_runtime.rs:359 (interactive_pty=false for run) deterministically routes claude --version to WriteRestricted."
      - "Resolved session nono-shell-status-dll-init-failed.md lines 113-114 + 480 document this EXACT mechanism: WRITE_RESTRICTED restricting-SID denies CLR/heavy-runtime DllMain WRITE-type accesses → 0xC0000142; cmd.exe (no heavy runtime) survives (Phase 15 Row C PASS)."
      - "`where.exe claude` → C:\\Users\\OMack\\.local\\bin\\claude.exe, 234 MB PE32+ console exe, mtime 2026-05-24 — a self-contained heavy-runtime binary that replaced the prior lighter launcher inside the 'recently broke' window."
      - "nono Windows token/label code did NOT change in 0.53.1..0.57.0 (git log confirms) — the version-bump correlation is coincidental; the external claude.exe shape change is the trigger."
    falsification_test: "If `nono run --profile claude-code -- cmd /c \"echo hi\"` ALSO fails 0xC0000142 on this host, the WriteRestricted token is not the differentiator and the hypothesis is wrong. (Phase 15 Row C predicts cmd PASSES.) Conversely, running claude.exe under a null or Low-IL primary token (no restricting SID) should NOT produce 0xC0000142."
    fix_rationale: "The root cause is the WRITE_RESTRICTED restricting-SID double-gate, not Low-IL labels. The Phase 31 broker (Low-IL primary token + inherited console, NO restricting SID) was production-validated to run heavy-runtime children (PowerShell/CLR) cleanly. Routing the non-PTY `nono run` path through an equivalent Low-IL/broker mechanism removes the restricting-SID write-gate while preserving write-deny via mandatory-label NO_WRITE_UP — addressing the cause, not the symptom. A null-token fallback would address the symptom but regress write protection (rejected in history)."
    blind_spots: "(1) The broker currently requires --inherit-handle (ConPTY pipes) and a PTY; a non-PTY broker mode (inherited console or pipe stdio) is not yet wired and needs design. (2) Have not field-confirmed claude.exe runs clean under a Low-IL primary token specifically (only PowerShell/CLR was validated). (3) Cannot reproduce the 0xC0000142 from this agent without spawning under nono on Windows; relying on documented matrix + deterministic code path. (4) Whether claude.exe needs WRITE access to a granted path (e.g. its temp-extraction dir) that the Low-IL label would also deny — separate from the token gate. [RESOLVED: Phase 51 designed + wired the no-PTY broker mode (BrokerLaunchNoPty); Phase 52 field-confirmed claude.exe runs clean under it — all four blind spots retired by the live repro B PASS.]"

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

- timestamp: 2026-05-27
  checked: VERIFICATION CLOSURE. Phase 52 HUMAN-UAT field-validation artifact (.planning/phases/52-field-validation-closure-heavy-runtime-human-uat-doc-update/52-HUMAN-UAT.md) + 52-01-SUMMARY.md, plus corroboration from the sibling resolved session unsigned-broker-trust-fail.
  found: Phase 52 ran the documented repro matrix on the operator's live Windows 11 host (build 26200) with the Phase 51 `nono 0.57.0` BrokerLaunchNoPty binary and the 234 MB self-contained claude.exe (234,248,864 bytes, 2026-05-24, PE32+). Repro A (`nono run --profile claude-code -- cmd /c "echo hi"`) → printed `hi`, exit 0 = PASS. Repro B (`nono run --profile claude-code -- claude --version`) → printed `2.1.150 (Claude Code)`, exit 0, NO 0xC0000142 / STATUS_DLL_INIT_FAILED, no WER dialog = PASS. Both operator-attested at the execute-phase checkpoint; ROADMAP SC-4 positive-spawn deferral CLOSED; REQ-WSRH-04 satisfied. Independently corroborated 2026-05-27 by the sibling debug session unsigned-broker-trust-fail, which ran the same command from a dev-layout build and observed the broker spawn PAST the old failure point ("broker: Low-IL primary token constructed") with no 0xC0000142.
  implication: The documented closure criterion (post-fix repro B prints the version and exits 0; repro A still passes) is SATISFIED on a real PowerShell console. The 0xC0000142 regression this session tracked is eliminated by the Phase 51 BrokerLaunchNoPty Low-IL primary token. NOTE — out of scope for this session: a separate NEW error (`CreateProcessAsUserW failed (GetLastError=87)` / ERROR_INVALID_PARAMETER, preceded by `alloc_console_rc=0`) surfaced only when running under the Claude Code git-bash/MSYS Bash tool (no real Win32 console; MSYS pipe stdio incompatible with Low-IL CreateProcessAsUserW std-handle inheritance). That is an environment artifact, NOT the 0xC0000142 bug, and does NOT contradict the Phase 52 PASS (which ran from a real PowerShell console). If pursued, it is a SEPARATE /gsd:debug and only if it also reproduces from a native PowerShell console.

## Eliminated

- The 0.53.1→0.57.0 version bump (commit 6cf7e85c): pure version-string change across 5 Cargo.toml + Cargo.lock, no behavior change. The version-bump correlation was coincidental.
- Possibility (B) — interactive-detection / PTY-allocation routing change in the 0.53.1..0.57.0 window: ELIMINATED. `nono run` hardcodes interactive_pty=false (launch_runtime.rs:359); the only in-window commits touching the Windows supervisor/mod files (b6a88fea Linux af_unix, 4a60f675 ApprovalDecision rename) do not alter PTY/token routing.
- nono Windows token/label/policy code regression: ELIMINATED. Nothing in restricted_token.rs / labels_guard.rs / policy.json's Windows handling changed in the suspected window. The trigger was the EXTERNAL claude.exe binary changing shape (lightweight launcher → 234 MB self-contained heavy runtime) on 2026-05-24.

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
    APPLIED (Option 1 — security-preserving) and SHIPPED as milestone v2.7 "Windows supervised-run hardening"
    (v2.7 / v0.57.2, pushed + tagged 2026-05-26). Routes the non-PTY `nono run` supervised path through a new
    WindowsTokenArm::BrokerLaunchNoPty — a no-PTY Low-IL broker token instead of WRITE_RESTRICTED. The Low-IL
    primary token carries NO restricting SID, so the heavy-runtime claude.exe DllMain WRITE-type accesses succeed;
    write-deny is preserved via mandatory-label NO_WRITE_UP rather than the restricting-SID double-gate. This
    addresses the cause (the WRITE_RESTRICTED restricting-SID write-gate, structurally incompatible with
    heavy-runtime children) without regressing the write-protection model.

    Delivered in two phases:
      - Phase 51 (implementation): added the BrokerLaunchNoPty arm + select_windows_token_arm routing + the
        broker `--no-pty` mode + a write-deny regression test; included the CR-01 stderr-deadlock fix and the
        windows_low_il_broker field wiring.
      - Phase 52 (field validation): HUMAN-UAT repro A + repro B matrix on Win11 build-26200 with the 234 MB
        self-contained claude.exe — both PASS, 0xC0000142 confirmed gone.

- verification: |
    SATISFIED. Phase 52 HUMAN-UAT repro B PASS on Win11 build 26200 with the Phase 51 `nono 0.57.0`
    BrokerLaunchNoPty binary and the 234 MB self-contained claude.exe (234,248,864 B, 2026-05-24, PE32+):
      A. `nono run --profile claude-code -- cmd /c "echo hi"`  → printed `hi`, exit 0 = PASS (no regression).
      B. `nono run --profile claude-code -- claude --version`  → printed `2.1.150 (Claude Code)`, exit 0,
         NO 0xC0000142 / STATUS_DLL_INIT_FAILED, no WER dialog = PASS (bug eliminated).
    Both operator-attested at the execute-phase checkpoint (evidence in
    .planning/phases/52-field-validation-closure-heavy-runtime-human-uat-doc-update/52-HUMAN-UAT.md;
    ROADMAP SC-4 positive-spawn deferral closed; REQ-WSRH-04 satisfied). Independently corroborated 2026-05-27
    by the sibling resolved session unsigned-broker-trust-fail, which observed the broker spawn past the old
    failure point ("broker: Low-IL primary token constructed") with no 0xC0000142.

    Out of scope for this session: a separate NEW `CreateProcessAsUserW failed (GetLastError=87)` error appears
    only under the Claude Code git-bash/MSYS Bash tool (no real Win32 console). It is an environment artifact,
    NOT this 0xC0000142 bug, and does not contradict the Phase 52 PASS (real PowerShell console). Any pursuit is
    a SEPARATE /gsd:debug, and only if it also reproduces from a native PowerShell console.

- files_changed:
    - crates/nono-cli/src/exec_strategy_windows/launch.rs   # Phase 51: WindowsTokenArm::BrokerLaunchNoPty arm + select_windows_token_arm routing for the non-PTY `nono run` supervised path
    - crates/nono-cli/src/exec_strategy_windows/mod.rs       # Phase 51: broker token-arm wiring / windows_low_il_broker field plumbing
    - crates/nono-shell-broker/                              # Phase 51: broker `--no-pty` mode (no-PTY Low-IL primary token + inherited/pipe stdio) + CR-01 stderr-deadlock fix
