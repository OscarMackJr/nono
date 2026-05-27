---
slug: broker-nopty-createproc-gle87
status: resolved
trigger: |
  PS C:\Users\OMack\nono-poc>  C:\Users\OMack\Nono\target\release\nono.exe run --profile claude-code --allow-cwd -- claude --version
  ... (sandbox applied, label-guard WARNs) ...
  INFO nono_shell_broker::broker: broker: console attach probe alloc_console_rc=0
  INFO nono_shell_broker::broker: broker: Low-IL primary token constructed
  ERROR nono_shell_broker: broker: fatal error error=Sandbox initialization failed: CreateProcessAsUserW failed (GetLastError=87)
  nono-shell-broker: Sandbox initialization failed: CreateProcessAsUserW failed (GetLastError=87)
created: 2026-05-27
updated: 2026-05-27
---

# Debug Session: broker-nopty-createproc-gle87

## Symptoms

- **Expected behavior:** `nono run --profile claude-code --allow-cwd -- claude --version` (run from the dev-build `target\release\nono.exe`, which intentionally skips the Authenticode broker-trust gate) launches claude.exe inside the Low-IL broker sandbox and prints the Claude version, exiting 0. This is the Phase 51 `BrokerLaunchNoPty` (no-PTY Low-IL broker) path.
- **Actual behavior:** The broker spawns, attaches a console, and constructs the Low-IL primary token successfully — but then `CreateProcessAsUserW` fails with `GetLastError=87` (ERROR_INVALID_PARAMETER). The child (claude.exe) never launches. Process aborts with `nono-shell-broker: Sandbox initialization failed: CreateProcessAsUserW failed (GetLastError=87)`.
- **Error message / code:** `CreateProcessAsUserW failed (GetLastError=87)` = Win32 ERROR_INVALID_PARAMETER. Immediately preceded by `broker: console attach probe alloc_console_rc=0` and `broker: Low-IL primary token constructed`.
- **Baseline:** The directly-preceding 0xC0000142 (STATUS_DLL_INIT_FAILED) regression was FIXED and field-validated in v2.7 / v0.57.2 (resolved session `claude-exe-dll-init-failed.md`, Phase 51 + 52). Phase 52 repro B printed `2.1.150 (Claude Code)` exit 0 on Win11 build-26200 — but see Evidence 2026-05-27 (binary-identity): that PASS ran on a `C:\Program Files\nono` build stamped `nono 0.57.0`, which predates the CR-01 stderr-merge commit; the merged HANDLE_LIST shape now in HEAD was NOT necessarily exercised by Phase 52.
- **Timeline:** Surfaced 2026-05-27 on the v0.57.3 `target\release` build (rebuilt today, quick task `260527-vb3`). The resolved `claude-exe-dll-init-failed.md` (lines 71-74, 129-132) PREDICTED this exact error and deferred it as a possible git-bash/MSYS environment artifact, with the explicit condition: "a SEPARATE /gsd:debug, and only if it also reproduces from a native PowerShell console."
- **Reproduction:** Run from a **native PowerShell console** (`PS C:\Users\OMack\nono-poc>`), NOT the Claude Code git-bash/MSYS Bash tool: `C:\Users\OMack\Nono\target\release\nono.exe run --profile claude-code --allow-cwd -- claude --version`.
- **Platform:** Windows 11 Enterprise build 26200 (win32). Windows sandbox backend (Job Objects / Integrity Level / WFP). nono v0.57.3 dev-build layout (broker-trust gate skipped).

## Current Focus

- hypothesis: **[ROOT CAUSE — CONFIRMED against production source; FIX APPLIED]** The CR-01 stderr-merge (commit `f79a5a1a`) made `nono-cli` pass THREE `--inherit-handle` values to the broker in which `hStdOutput` and `hStdError` are the **same handle value** (`pipes.stdout_write` appears at child_stdio[1] AND child_stdio[2] — launch.rs:1688-1689). The broker (`crates/nono-shell-broker/src/main.rs:237`) then built its `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` from ALL THREE values verbatim (`args.inherit_handles.clone()`), with NO dedup. A `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` containing a **duplicate handle value** is rejected by the kernel at process-creation time → `CreateProcessAsUserW` returns ERROR_INVALID_PARAMETER (87). The validation happens at CreateProcessAsUserW, not at UpdateProcThreadAttribute — exactly matching the observed failure point (token constructed OK, then 87). nono-cli's OWN HANDLE_LIST already dedupes to the 2 unique handles `{stdin_read, stdout_write}` (launch.rs:1692 `gated_handles`); the broker side was NOT given the same treatment — that was the asymmetry.
- test: read broker `main.rs` CreateProcessAsUserW construction and the nono-cli `BrokerLaunchNoPty` arm; confirm whether the broker dedupes its HANDLE_LIST or clones the raw `--inherit-handle` list including the merged duplicate. [DONE]
- expecting: broker HANDLE_LIST = `[stdin_read, stdout_write, stdout_write]` (duplicate) while nono-cli HANDLE_LIST = `[stdin_read, stdout_write]` (deduped). [CONFIRMED — launch.rs:1688-1692 vs main.rs:237]
- next_action: RESOLVED 2026-05-27. Operator PowerShell re-verify PASSED (child_exit_code=0, no GLE=87). Root cause confirmed; fix committed; session moved to resolved/.
- reasoning_checkpoint:
    hypothesis: "Broker PROC_THREAD_ATTRIBUTE_HANDLE_LIST contains a duplicate handle (the merged stdout_write bound to both hStdOutput and hStdError) → CreateProcessAsUserW rejects with ERROR_INVALID_PARAMETER (87)."
    confirming_evidence:
      - "launch.rs:1688-1689 — child_stdio = [stdin_read, stdout_write, stdout_write] (CR-01 merge: hStdOutput == hStdError)."
      - "launch.rs:1790-1793 — all THREE child_stdio values are forwarded to the broker as --inherit-handle."
      - "main.rs:237 (pre-fix) — broker HANDLE_LIST = args.inherit_handles.clone() (all 3, no dedup)."
      - "main.rs:311-314 — broker binds hStdInput=ih[0], hStdOutput=ih[1], hStdError=ih[2] (the bind correctly reuses the duplicate; only the HANDLE_LIST is the problem)."
      - "Failure point = CreateProcessAsUserW (not UpdateProcThreadAttribute), matching kernel-time HANDLE_LIST validation of duplicate handles."
      - "nono-cli's own HANDLE_LIST (gated_handles, launch.rs:1692) dedupes to 2 — proving the author knew the duplicate must be gated once, but only fixed the local CreateProcessW, not the broker's downstream CreateProcessAsUserW."
    falsification_test: "If the broker is changed to dedup its HANDLE_LIST to the unique set {ih[0], ih[1]} (== {stdin_read, stdout_write}) while STILL binding all three stdio slots (hStdError = ih[2] = the same stdout_write value), and CreateProcessAsUserW then succeeds (claude prints its version, exit 0) from a real PowerShell console, the duplicate-HANDLE_LIST root cause is confirmed. If 87 persists after dedup, the cause is a DIFFERENT parameter. — DEDUP APPLIED; the operator PowerShell run is the final confirming arm of this test."
    blind_spots: "(1) Live verification must be operator-run from PowerShell (this agent's MSYS shell would confound the result) — STILL OPEN until the operator runs it. (2) Phase 52's 'v0.57.0 PASS' ran on a Program Files build stamped 0.57.0 that predates CR-01 (f79a5a1a, committed 14:22 the same day; UAT ran 19:15 on a staged/installed binary) — so it likely exercised the PRE-merge 3-unique-handle shape, not today's merged-duplicate shape. This explains why Phase 52 passed yet HEAD failed: the regression entered with CR-01. (3) An alternate but less likely cause is that one of the merged handles is non-inheritable in the broker; ruled out because nono-cli flips both unique handles inheritable before CreateProcessW and the broker inherits them — and the bind itself is not what 87 rejects."

## Evidence

- timestamp: 2026-05-27
  checked: Cross-reference to resolved session `.planning/debug/resolved/claude-exe-dll-init-failed.md` (lines 71-74 Evidence + lines 129-132 Resolution "out of scope" note).
  found: That session explicitly recorded a "separate NEW error (`CreateProcessAsUserW failed (GetLastError=87)` / ERROR_INVALID_PARAMETER, preceded by `alloc_console_rc=0`)" and hypothesized it was a git-bash/MSYS environment artifact, explicitly NOT the 0xC0000142 bug. It set the condition: pursue as a SEPARATE /gsd:debug "only if it also reproduces from a native PowerShell console."
  implication: The user's repro IS from a native PowerShell console, so the env-artifact deferral condition is met and the env-artifact hypothesis is FALSIFIED for this case. This is a genuine bug in the Phase 51 no-PTY `BrokerLaunchNoPty` broker path.

- timestamp: 2026-05-27
  checked: Production source — `crates/nono-shell-broker/src/main.rs` (CreateProcessAsUserW call site, lines 194-359) and `crates/nono-cli/src/exec_strategy_windows/launch.rs` BrokerLaunchNoPty arm (lines 1631-1851).
  found: nono-cli (launch.rs:1688-1689) sets `child_stdio = [pipes.stdin_read, pipes.stdout_write, pipes.stdout_write]` — the CR-01 stderr→stdout merge makes positions [1] and [2] the SAME handle value. All three are forwarded to the broker via `--inherit-handle` (launch.rs:1790-1793). The broker (main.rs:237, pre-fix) built its `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` from `args.inherit_handles.clone()` — all 3 values including the duplicate, no dedup. nono-cli's OWN HANDLE_LIST (launch.rs:1692 `gated_handles`) correctly dedupes to the 2 unique handles `{stdin_read, stdout_write}`. The broker's std-handle BIND (main.rs:311-314) correctly reuses the duplicate for hStdOutput/hStdError; only the HANDLE_LIST was malformed.
  implication: ROOT CAUSE. A `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` with a duplicate handle is rejected by the kernel at process-creation time → `CreateProcessAsUserW` GLE=87 (ERROR_INVALID_PARAMETER). Failure occurs precisely at CreateProcessAsUserW (after `Low-IL primary token constructed`, before any child runs), matching the trigger log. The fix is broker-side: dedup the HANDLE_LIST to unique handles while keeping the three-slot stdio bind unchanged.

- timestamp: 2026-05-27
  checked: git provenance — `git merge-base --is-ancestor` of CR-01 (`f79a5a1a`) and the BrokerLaunchNoPty spawn wiring (`fcba74dd`) against tag `v0.57.0`; Phase 52 `52-HUMAN-UAT.md` binary-identity (line 15).
  found: Neither `fcba74dd` (spawn wiring) nor `f79a5a1a` (CR-01 merge) is an ancestor of tag `v0.57.0`; both ARE in HEAD. Phase 52 UAT line 15 records the binary as `nono 0.57.0` at `C:\Program Files\nono\nono.exe`. CR-01 was committed 2026-05-26 14:22; the UAT ran 2026-05-26 19:15 on a staged Program Files install. The CR-01 commit (`f79a5a1a`) touched the broker only to add the WR-01 fail-closed `len() < 3` guard (+14 lines) — it did NOT add HANDLE_LIST dedup. `nono-shell-broker.exe` on disk was dated May 26 23:32; `nono.exe` is dated May 27 11:50 (today's `260527-vb3` rebuild). Both binaries are post-CR-01.
  implication: Resolves the Phase 52 contradiction (reasoning_checkpoint blind-spot #2). The Phase 52 "PASS" most plausibly ran a Program Files build that predates the CR-01 merged-handle shape (the version string stayed 0.57.0 across the merge). The merged-duplicate HANDLE_LIST regression entered with CR-01 and is what the dev-build `target\release` (now built from HEAD) hits. The nono code DID change since the validated baseline — the version-string sameness is the trap, exactly the documented `feedback_sdk_state_status_clobber` / version-drift class of lesson.

- timestamp: 2026-05-27
  checked: FIX APPLICATION + verification — built/tested/clippy the broker crate on the Windows host after applying the dedup fix.
  found: `cargo build -p nono-shell-broker` clean. `cargo test -p nono-shell-broker` → 20 passed / 0 failed (including the 3 new `dedup_handles_tests`, notably `dedup_collapses_merged_stdout_stderr_duplicate` which pins the [stdin_read, stdout_write, stdout_write] → [stdin_read, stdout_write] collapse — the exact regression guard for this bug). `cargo clippy -p nono-shell-broker --all-targets -- -D warnings -D clippy::unwrap_used` clean (no warnings). `cargo build --release -p nono-shell-broker` succeeded; `target\release\nono-shell-broker.exe` rebuilt (now dated May 27 13:50, post-fix; the prior on-disk broker dated May 26 23:32 predated the fix). `nono.exe` itself is unchanged by this fix (the bug + fix are entirely broker-side; launch.rs untouched).
  implication: Fix is in place and unit-proven on the Windows host. The only remaining arm of the falsification test is the operator's live PowerShell run — which CANNOT be performed from this agent's MSYS shell without confounding the result (environment constraint per the sibling resolved session + memory feedback_windows_supervised_needs_real_console).

## Eliminated

- **Environment artifact (MSYS/git-bash no real console):** ELIMINATED. The repro is from a native PowerShell console with `alloc_console_rc=0` (a console already exists — the expected real-console case). The deferral condition set by the sibling resolved session is met.
- **Environment block / CREATE_UNICODE_ENVIRONMENT mismatch (falsification alt-cause a):** ELIMINATED. The broker's CreateProcessAsUserW passes `lpEnvironment = null` (inherit broker env) with NO `CREATE_UNICODE_ENVIRONMENT` flag (main.rs:337-338) — no env-block/flag mismatch possible.
- **Command-line / lpApplicationName malformation (falsification alt-cause b):** ELIMINATED. `build_command_line` is the same Phase-31 builder used by the PTY path (validated); lpApplicationName is null with the quoted exe as argv[0] in lpCommandLine — the standard shape.
- **Token invalidity:** ELIMINATED. The log shows `broker: Low-IL primary token constructed` immediately before the failure; the token handle is RAII-valid (OwnedHandle) and unchanged from the PTY path that works.
- **Non-inheritable std handles:** ELIMINATED as the 87 cause. nono-cli flips both unique handles inheritable (SetHandleInformation HANDLE_FLAG_INHERIT) before spawning the broker, and the broker inherits them; non-inheritable handles in STARTF_USESTDHANDLES would not produce 87 at the broker's CreateProcessAsUserW in this shape — the duplicate in the HANDLE_LIST is the decisive invalid parameter.

## Resolution

- root_cause: |
    CR-01 (commit f79a5a1a, Phase 51) merged the child's stderr into stdout so that nono-cli passes THREE
    `--inherit-handle` values to nono-shell-broker in which hStdOutput and hStdError are the SAME handle value
    (`pipes.stdout_write` at child_stdio[1] and child_stdio[2]; launch.rs:1688-1689). The broker
    (crates/nono-shell-broker/src/main.rs:237) built its PROC_THREAD_ATTRIBUTE_HANDLE_LIST from
    `args.inherit_handles.clone()` — all three values verbatim, including the duplicate stdout_write, with no
    dedup. A PROC_THREAD_ATTRIBUTE_HANDLE_LIST containing a duplicate handle value is rejected by the kernel at
    process-creation time, so CreateProcessAsUserW returns ERROR_INVALID_PARAMETER (87) after the Low-IL primary
    token is constructed. nono-cli's own HANDLE_LIST (gated_handles, launch.rs:1692) already dedupes to the two
    unique handles {stdin_read, stdout_write}; the broker side was never given the same dedup, creating the
    asymmetry. (The std-handle BIND in the broker — main.rs:311-314 — correctly reuses the duplicate for
    hStdInput/hStdOutput/hStdError; only the HANDLE_LIST is malformed.)

    Phase 52's "v0.57.0 PASS" did not contradict this: that UAT ran a Program Files build stamped 0.57.0 that
    predates the CR-01 merge (the version string was unchanged across the merge). The regression entered with
    CR-01; the dev-build target\release binary, now built from HEAD, is the first to exercise the merged-duplicate
    HANDLE_LIST from a real PowerShell console.

- fix: |
    APPLIED (broker-side, minimal). In crates/nono-shell-broker/src/main.rs:
      - Added a free helper `dedup_handles_preserve_order(&[HANDLE]) -> Vec<HANDLE>` (order-preserving, first-seen).
      - run() now builds the HANDLE_LIST via `dedup_handles_preserve_order(&args.inherit_handles)` instead of the
        raw `args.inherit_handles.clone()`, so the PROC_THREAD_ATTRIBUTE_HANDLE_LIST gates each UNIQUE handle
        exactly once. `handles_byte_size` recomputes from the deduped slice.
      - The three-slot std-handle BIND (hStdInput/hStdOutput/hStdError = ih[0]/ih[1]/ih[2]) is UNCHANGED, so
        hStdError still legitimately aliases hStdOutput (the supervisor's stderr→stdout merge is preserved).
      - The existing CR-02 (null/INVALID), CR-03 (empty-list), and WR-01 (--no-pty len<3 fail-closed) guards are
        untouched and still execute in parse_args()/run().
      - Added unit-test module `dedup_handles_tests` (3 tests): collapses the merged [stdin_read, stdout_write,
        stdout_write] to [stdin_read, stdout_write]; passes a unique list through unchanged in order; single-handle
        round-trip.
    Build/test/clippy on the Windows host all green: cargo build -p nono-shell-broker (clean),
    cargo test -p nono-shell-broker (20 passed / 0 failed), cargo clippy -p nono-shell-broker --all-targets
    -- -D warnings -D clippy::unwrap_used (no warnings), cargo build --release -p nono-shell-broker (clean;
    release broker rebuilt). Cross-target Unix clippy gate does NOT apply: the change is entirely within the
    Windows-only `#[cfg(windows)] mod broker` (no shared/Unix-cfg code touched).

- verification: |
    PASS (operator-run, native PowerShell console, 2026-05-27 18:51). From a profile-covered cwd:
      PS C:\Users\OMack\.claude>  C:\Users\OMack\Nono\target\release\nono.exe run --profile claude-code --allow-cwd -- claude --version
    Broker log: `broker: Low-IL primary token constructed` → `broker: spawned Low-IL child child_pid=11580` →
    `broker: child exited child_exit_code=0`. NO `CreateProcessAsUserW failed (GetLastError=87)`. The child
    spawned and exited 0 — the GLE=87 failure point is gone. This is the final confirming arm of the falsification
    test: deduping the HANDLE_LIST to the unique set while keeping the three-slot stdio bind made
    CreateProcessAsUserW succeed → the duplicate-HANDLE_LIST root cause is CONFIRMED. Verified on the
    fix-rebuilt broker (target\release\nono-shell-broker.exe, May 27 13:50).

- files_changed:
    - crates/nono-shell-broker/src/main.rs   # dedup_handles_preserve_order helper + HANDLE_LIST dedup at the no-PTY CreateProcessAsUserW; 3 new dedup_handles_tests; 3-slot stdio bind unchanged
