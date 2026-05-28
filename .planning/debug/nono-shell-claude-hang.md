---
slug: nono-shell-claude-hang
status: fix_applied_field_verification_pending
trigger: |
  DATA_START
  "this command hung - it should have launched claude:
  PS C:\Users\OMack\nono-poc> nono.exe shell --profile claude-code --allow-cwd

    nono v0.57.3
  WARN Profile policy path '$HOME/Library/Keychains' does not exist, skipping
    Capabilities: r+w .claude, .cache\claude, AppData\Roaming\nono\profiles,
    .claude\claude.json, nono-poc; +6 system/group paths; net outbound allowed
    Applying sandbox...
  (10x WARN label guard: pre-existing mandatory-label ACE; skipping apply+revert; prior_rid=0x1000)
  WARN label guard: path not owned by current user; skipping mandatory label apply path=C:\Windows access=Read
  INFO nono_shell_broker::broker: broker: console attach probe alloc_console_rc=0
  INFO nono_shell_broker::broker: broker: Low-IL primary token constructed
  INFO nono_shell_broker::broker: broker: spawned Low-IL child child_pid=38460
  PS Microsoft.PowerShell.Core\FileSystem::\\?\C:\Users\OMack\nono-poc> claude"
  DATA_END
created: 2026-05-28T15:40:00Z
updated: 2026-05-28T18:30:00Z
host: windows-11-build-26200 (OMack dev box)
binary: nono.exe v0.57.3 (run from C:\Users\OMack\nono-poc; installed MSI or copy, NOT dev-layout target/release with untagged post-v2.7 fixes)
specialist_hint: rust-windows
related_resolved:
  - .planning/debug/resolved/nono-shell-status-dll-init-failed.md (Phase 30 — shell LAUNCH failure 0xC0000142; CSRSS ALPC denial at Low-IL; broker pattern was the fix)
  - .planning/debug/resolved/windows-supervised-exec-cascade.md (Phase 15 — detached path 0xC0000142)
  - .planning/debug/resolved/nopty-broker-stdout-swallowed.md (post-v2.7 — no-PTY relay swallowed child stdout)
  - .planning/debug/resolved/broker-nopty-createproc-gle87.md (post-v2.7 — broker HANDLE_LIST dedup)
  - .planning/debug/resolved/claude-exe-dll-init-failed.md
related_phases: [30, 31, 51, 52]
---

# Debug: `claude` hangs (zero output) when launched inside a working `nono shell` Low-IL sandbox

## Symptoms

**Expected:** Inside the Low-IL sandboxed PowerShell opened by `nono shell --profile claude-code --allow-cwd`, typing `claude` (no args, the interactive Claude Code TUI) should launch claude and render its UI.

**Actual:** The shell itself launches correctly (broker spawned Low-IL child PID 38460; PowerShell prompt `PS ...\nono-poc>` appears). Typing `claude` then **hangs with ZERO output** — frozen cursor, no banner, no spinner, no error. Never returns to a prompt; the user must Ctrl-C / close the window.

**Error messages:** None. No exit code observed (it never exits — it hangs). The 10 label-guard warnings + the C:\Windows ownership-skip warning are informational (D-09 leaked-Low-IL-label noise), not failures.

**Timeline:** First time the user has tried running `claude` inside `nono shell`. Binary is **v0.57.3** (the older installed build run from `C:\Users\OMack\nono-poc`), NOT the dev-layout `target/release/nono.exe` carrying the untagged post-v2.7 fixes that Phase 53 is about to release as v0.57.4.

**Reproduction:**
1. `cd C:\Users\OMack\nono-poc`
2. `nono.exe shell --profile claude-code --allow-cwd`  → lands in Low-IL PowerShell (works)
3. At the sandboxed prompt, type `claude` <Enter>  → hangs, zero output

**Baseline (control):** `claude` / `claude --version` works fine OUTSIDE nono in a plain (Medium-IL) PowerShell on the same machine. So claude.exe itself is healthy; the sandbox context is the differentiator.

## Critical framing — this is NOT the resolved Phase 30 launch bug

The resolved session `nono-shell-status-dll-init-failed.md` was about the **shell never launching** (silent exit, `0xC0000142 STATUS_DLL_INIT_FAILED`, CSRSS console-subsystem ALPC connect denied to a Low-IL client). Phase 31's **broker-process pattern** fixed that: a Medium-IL broker attaches the console, then spawns the Low-IL child which **inherits the already-attached console** and thereby skips the CSRSS ALPC connect (RESEARCH Assumption A1, PoC- and production-validated 2026-05-08/09).

Here, the broker pattern is WORKING — the log proves it (`broker: spawned Low-IL child child_pid=38460`, prompt rendered). The failure is one level deeper: **`claude.exe` is a grandchild** spawned by the Low-IL PowerShell (via PowerShell's own normal CreateProcess, NOT via the broker). The open question is whether that grandchild — and any further descendants claude spawns (node, ripgrep, helper procs) — re-encounter the CSRSS/Low-IL barrier that the broker only bypassed for the *direct* child it spawned with the inherited console.

## Hypotheses (initial, ranked)

### H1 (now DOWNGRADED — see Eliminated/reasoning_checkpoint) — claude's startup spawns a Low-IL descendant that hits the CSRSS ALPC denial the broker only bypassed for its own direct child
PowerShell inherited the broker's console (A1). But when PowerShell spawns `claude.exe`, those descendants go through ordinary CreateProcess. Theory: a descendant tries to establish its own CSRSS console connection at Low-IL and fails 0xC0000142, blocking claude. **Static analysis downgrades this:** claude.exe inherits the SAME real console PowerShell did (see Investigation Summary), so the CSRSS attach is skipped for claude exactly as for PowerShell — no 0xC0000142 expected. Field probe P4 (is claude alive or dead while hung?) is the arbiter.

### H5-UNIVERSAL (HIGH — refined root-cause candidate after P1-P4) — every grandchild is a NEW Low-IL console client that hangs registering with the Medium-IL conhost; the direct-child A1 console-inheritance skip does not extend to grandchildren
PowerShell inherits nono.exe's real console at creation (A1 → its own `ConClntInitialize` is a no-op; PowerShell's REPL works). But each grandchild PowerShell spawns (`claude`, `cmd /c echo`, `node`) is a fresh Low-IL console-subsystem process that must register with the conhost serving nono.exe's REAL console — a conhost at Medium IL. A1 (PoC- + production-validated) only covers the DIRECT inherited-console child, not grandchildren. The grandchild's cross-IL console-client registration (Low-IL grandchild ↔ Medium-IL conhost ALPC connect, and/or conhost's server-side open-back of the Low-IL grandchild — Project Zero "In-Console-Able" server-side mode) BLOCKS at the IL boundary and neither completes nor fails → silent indefinite HANG, zero output, grandchild ALIVE (P4). UNIVERSAL across grandchild types (P2 `cmd /c echo`, no raw mode, hangs identically). This SUPERSEDES H2-REFINED's raw-mode-contention sub-explanation. NOT the Phase 30 client-side STATUS_ACCESS_DENIED→0xC0000142 crash (H1, eliminated by P4 — alive, not dead). Residual G1/G2 ambiguity (block upstream of first write vs at first console/pipe write) is resolved by the `cmd /c "echo HI > out.txt"` discriminator before any fix.

### H2-REFINED (SUPERSEDED by H5-UNIVERSAL; valid sub-findings retained) — the ConPTY is never attached; PowerShell+grandchildren share nono.exe's real console
The live `WindowsTokenArm::BrokerLaunch` path does NOT attach `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` and does NOT set `STARTF_USESTDHANDLES`. So the ConPTY `hpcon` created by nono.exe is never attached to any process; PowerShell (and its grandchildren) inherit nono.exe's REAL console. This static finding REMAINS VALID and is load-bearing. The original "supervisor input thread contends for the console input queue → raw-mode deadlock" CAUSAL claim is ELIMINATED by P2 (`cmd /c echo`, no raw-mode input, also hangs) — it cannot be the universal blocker. The supervisor stdin thread may be a secondary irritant but is not the root cause.

### H3 (MEDIUM) — claude hangs on a Low-IL-denied resource (network/auth/named-pipe/temp), not console
claude at startup may open a named pipe / lockfile / config write / network auth. Under Low-IL (mandatory-label NO_WRITE_UP) a write to a Medium-IL location, or a blocked outbound connection, could hang claude indefinitely with no output if it retries silently. Kept as a fallback if the field probes contradict H2-REFINED.

### H4 (LOW) — v0.57.3-specific; fixed or changed in the dev-layout post-v2.7 binary
The post-v2.7 fixes (d8b7ce00 broker HANDLE_LIST dedup; 005b4c9e no-PTY stdout echo) target the `nono run` no-PTY path, not `nono shell` ConPTY/console. The static analysis shows the `nono shell` BrokerLaunch (PTY) arm is untouched by those fixes, so the dev-layout binary is expected to behave identically here — useful only as a control.

## Current Focus

**hypothesis:** H5-UNIVERSAL (HIGH, refined after P1-P4) — **the live `nono shell` BrokerLaunch (PTY) path never attaches the ConPTY to any process and never binds pipe stdio (no `STARTF_USESTDHANDLES`); PowerShell inherits nono.exe's REAL console at creation, so PowerShell's OWN console-client registration is the A1-validated inheritance no-op (it works). But every grandchild PowerShell spawns (`claude`, `cmd /c echo`, `node`) is a NEW Low-IL console-subsystem client that must register fresh with the conhost serving nono.exe's REAL console — a conhost running at Medium IL. RESEARCH A1 only proves the DIRECT broker child skips `ConClntInitialize`; it does NOT extend to grandchildren. The grandchild's cross-IL console-client registration (Low-IL grandchild ↔ Medium-IL conhost ALPC, and/or conhost's server-side open-back of the Low-IL grandchild — Project Zero "In-Console-Able" server-side mode) BLOCKS at the IL boundary and never completes AND never fails → silent indefinite HANG, zero output, grandchild ALIVE (P4). This is UNIVERSAL (P2 `cmd /c echo`, no raw mode, also hangs) and is NOT the Phase 30 client-side 0xC0000142 crash (H1, eliminated by P4).**
**test:** RESOLVED. The DISCRIMINATOR `cmd /c "echo HI > out.txt"` was run by the operator on a REAL console
  inside a working sandbox (Evidence 2026-05-28T17:40:00Z): STILL HUNG, no out.txt, no prompt return.
**expecting:** RESOLVED → **G1 confirmed, G2 refuted.** out.txt was NOT created and no prompt returned →
  the grandchild blocks UPSTREAM of its first write, in cross-IL console-client registration / handle setup,
  exactly as the Win32 finding (a) predicted. There is NO residual G1/G2 ambiguity; the root cause is now
  singular: a fresh Low-IL grandchild cannot complete console-client registration with the Medium-IL conhost
  serving nono.exe's real console.
**next_action:** HANG FIX VERIFIED (Option D′, commit `40c11831`). Live Win11 build-26200 field run with the
  dev-layout v0.57.4 binary confirms the G1 hang is GONE: `cmd /c echo` returns output AND control returns to
  the prompt (previously every grandchild hung with zero output). The ORIGINAL bug (hang) is RESOLVED.
  TWO PRE-EXISTING issues were UNMASKED by the fix (the hang previously hid them); both are DISTINCT from the
  hang and are NOT regressions from the D′ change (the old hanging path passed the same cwd/env):
    - **Issue A (cwd `\\?\` verbatim prefix):** the sandboxed cwd is `\\?\C:\Users\OMack\.claude`; `cmd.exe`
      rejects `\\?\` cwd ("UNC paths are not supported. Defaulting to Windows directory") and falls back to
      C:\Windows, where relative writes (`echo HI > out.txt`) are denied. The "Access is denied" CONFIRMS
      Low-IL NO_WRITE_UP still enforces (not a security regression — a usability bug). ROOT CAUSE: cwd comes
      from `workdir.canonicalize()` (execution_runtime.rs:72) → Windows verbatim `\\?\` path; the strip in
      `normalize_windows_launch_path` (launch.rs:998-1009) is OFF BY ONE BACKSLASH — it strips `r"\?\UNC"` /
      `r"\?"` (single leading backslash) but a canonicalized verbatim path is `\\?\UNC\` / `\\?\` (double), so
      the prefix is NEVER stripped. Fix: correct the strip patterns to `\\?\UNC\` / `\\?\` (small, contained,
      same security-critical launch path → checkpoint discipline still applies).
    - **Issue B (`claude` not found):** `CommandNotFoundException` inside the sandbox though `claude` works
      outside in plain PowerShell. PATH / command-resolution issue — env not propagated into the Low-IL shell,
      OR claude lives in a dir not on the sandbox PATH (note `.local\bin` IS granted RX per label-guard mask
      0x5), OR `claude` is a `$PROFILE` function and the sandboxed PowerShell launched without loading the
      profile. NEEDS a diagnostic (`Get-Command claude` / `where.exe claude` OUTSIDE the sandbox + PATH dump
      inside) before root-causing.
  OPERATOR CHOSE "Fix A now, here." **Issue A FIXED (code-complete):** corrected the off-by-one-backslash
  strip patterns in `normalize_windows_launch_path` (launch.rs:998-1019) to `\\?\UNC\` / `\\?\`, mirroring the
  three existing correct siblings (`rollback_commands::normalize_path_for_compare`,
  `query_ext::strip_verbatim_prefix`, `protected_paths`). Added a Windows-gated regression unit test
  (`normalize_windows_launch_path_strips_verbatim_prefix`). Windows-host: test PASS (+ 6/6 broker_dispatch_tests),
  `cargo clippy -p nono-cli --bins -D warnings -D clippy::unwrap_used` CLEAN. Cross-target Linux/macOS clippy
  PARTIAL/deferred (this code is `exec_strategy_windows`-only — not compiled on Unix — and the host lacks the C
  cross-linker; per cross-target checklist). **Field re-verify of A PENDING:** debug `nono.exe` rebuild was
  BLOCKED by a file lock — nono.exe PID 41900 + broker PID 4324 from the operator's still-open field-test
  sandbox hold `target\debug\nono.exe`. Operator must close that sandbox window, rebuild
  (`cargo build -p nono-cli --bin nono`), then re-run `cmd /c "echo HI > out.txt"` inside a fresh sandbox —
  expect out.txt created in the (now-plain) cwd + prompt returns, no "UNC paths are not supported".
  **Issue A FIELD-VERIFIED RESOLVED** (2026-05-28T19:10): plain cwd, `echo > out.txt` + `type` work.
  **Issue B ROOT-CAUSED (curated PATH):** nono's `append_windows_runtime_env` hardcodes a minimal sandbox
  PATH (System32 + Windows + Wbem + WindowsPowerShell\v1.0); `build_child_env` drops the inherited user PATH.
  `claude.exe` is at `C:\Users\OMack\.local\bin\` (granted RX) but `.local\bin` isn't on the curated PATH →
  bare-name `claude` unresolvable. DESIGN decision, not a typo. Fix DIRECTION pending operator choice:
    - Option P1 (principled): add capability-granted executable dirs (the profile's read/exec-granted paths,
      e.g. `.local\bin`, `.cargo\bin`) to the curated sandbox PATH so explicitly-granted tools are invokable.
      Security note: prefer read/execute-only granted dirs; be cautious adding writable (r+w) grants to PATH.
    - Option P2 (profile-scoped): have the claude-code profile contribute a PATH augmentation (only when that
      profile is active) rather than a global env-builder change.
    - Option P3 (workaround/interim): invoke claude by full path; document `.local\bin`-on-PATH as a known gap.
  OPERATOR CHOSE P1. **Issue B FIXED (code-complete):** `append_windows_runtime_env` (launch.rs:688+) now
  appends capability-granted READ-ONLY directories (`access == AccessMode::Read && !is_file`) to the curated
  PATH, after the System32 baseline, with the `\\?\` verbatim prefix stripped via `normalize_windows_launch_path`
  and case-insensitive dedup. Read-only-only is the security boundary: a non-writable PATH dir cannot be used
  by the sandboxed agent to plant-and-execute an attacker-controlled binary; writable (r+w) and single-file
  grants are excluded; appending after System32 prevents shadowing of system commands. `.local\bin` is granted
  `read` via the cross-platform `user_tools` policy group → now on PATH → bare-name `claude` resolves.
  Added Windows-gated regression test `test_windows_read_only_granted_dir_appended_to_path` (asserts RO dir
  appended + after System32 + no `\\?\` + RW dir excluded). Windows-host: 5/5 env_filter_tests PASS, clippy
  (-D warnings -D unwrap_used) CLEAN, `target\debug\nono.exe` rebuilt. Cross-target linux/macos clippy
  PARTIAL/deferred (exec_strategy_windows is Windows-only; host lacks C cross-linker; per checklist).
  **FIELD VERIFY of B PENDING:** in a fresh `nono shell --profile claude-code --allow-cwd`, run `$env:PATH`
  (expect `.local\bin` now present) then `claude` (expect it resolves + launches; degraded/line-mode TUI per
  Option D′ is acceptable). Also still want the `& "C:\...\.local\bin\claude.exe"` full-path probe result to
  confirm claude RUNS under Low-IL (isolates PATH-resolution from any deeper claude-under-sandbox issue).
  Specialist dispatch: rust-windows.
**reasoning_checkpoint:**
  hypothesis: "Every grandchild PowerShell spawns inside the Low-IL `nono shell` sandbox is a NEW Low-IL
    console client that hangs registering with the Medium-IL conhost serving nono.exe's real console; the
    direct-child A1 console-inheritance skip does not extend to grandchildren."
  confirming_evidence:
    - "P2 `cmd /c echo` (no raw mode, no CLR, no Node) HANGS with zero output → universal grandchild block, not raw-mode-specific."
    - "P4 claude.exe ALIVE SI=1 → blocked/pending, not crashed → consistent with a never-completing ALPC connect, not STATUS_ACCESS_DENIED."
    - "Static: broker PTY-path spawn sets neither STARTF_USESTDHANDLES nor PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE; HANDLE_LIST does not govern console handles (Win32 docs) → grandchildren attach to the REAL console served by a Medium-IL conhost."
    - "RESEARCH A1 is scoped to the DIRECT inherited-console child only (PoC never spawned a grandchild); Project Zero documents a server-side conhost-open-back failure for Low-IL console clients."
  falsification_test: "`cmd /c \"echo HI > out.txt\"` writes out.txt AND returns to the prompt → grandchild ran fully → refutes the upstream-registration-hang form (G1) and points at a stdout/console-write topology block (G2)."
  fix_rationale: "Deferred until G1/G2 resolved. If G1: the fix must change how the broker establishes the console/stdio for the whole Low-IL tree (real ConPTY through broker, or pipe stdio) so a fresh grandchild never registers cross-IL with a Medium-IL conhost. If G2: the fix is stdio relay/wiring. Either way the prior Option A (supervisor stops contending for console input) is INSUFFICIENT — `cmd /c echo` does no console input and still hangs."
  blind_spots: "Cannot reproduce on this MSYS/no-console host. Cannot from static analysis alone distinguish a pending-ALPC-connect hang (G1) from a write-to-unserviced-handle hang (G2); the single discriminator probe resolves it. Have not confirmed which conhost instance the grandchildren target (assume nono.exe's real-console conhost) or whether AppContainer-vs-Low-IL changes the conhost open-back result."

## Evidence

- timestamp: 2026-05-28T19:10:00Z
  checked: FIELD RE-VERIFY of Issue A (cwd `\\?\` strip fix, commit `2fb0fad2`) + Issue B root-cause
    (`claude` not found). Live Win11 build-26200, rebuilt dev-layout `target\debug\nono.exe`, fresh
    `nono shell --profile claude-code --allow-cwd` from `%USERPROFILE%\.claude`.
  found:
    Issue A → FIXED + VERIFIED. Prompt now renders PLAIN `PS C:\Users\OMack\.claude>` (no `\\?\` provider
      prefix). `cmd /c "echo HI > out.txt"` returns cleanly (NO "UNC paths are not supported" message);
      `type out.txt` prints `HI`. The verbatim-prefix strip works end-to-end on the live box.
    Issue B → ROOT-CAUSED. `Get-Command claude` (outside nono) → `claude.exe`, CommandType `Application`,
      Source `C:\Users\OMack\.local\bin\claude.exe` (a real on-PATH exe, NOT a `$PROFILE` function/alias —
      profile-function theory refuted). Inside the sandbox `claude` → `CommandNotFoundException`. CODE
      ROOT CAUSE: nono builds a CURATED child environment — `build_child_env` (launch.rs:551-663) puts
      `PATH` in the skip-list (the inherited user PATH is dropped), and `append_windows_runtime_env`
      (launch.rs:688-694) rebuilds `PATH` to a hardcoded minimal baseline:
      `{SystemRoot}\System32;{SystemRoot};{SystemRoot}\System32\Wbem;{SystemRoot}\System32\WindowsPowerShell\v1.0`.
      `.local\bin` (where claude lives, and which IS granted RX per the label-guard mask 0x5) is NOT on that
      PATH → `claude` (and `node`, `ripgrep`, any user-dir tool) is unresolvable by bare name; `cmd` /
      `powershell` resolve only because they live in System32. This is a DELIBERATE security/determinism
      design (don't inherit arbitrary, possibly-writable user PATH dirs), NOT a typo like Issue A.
  implication: Issue A is DONE (verified). Issue B is a DESIGN decision, not a one-line fix: to make
    granted tools invokable, nono must add capability-granted executable directories to the curated sandbox
    PATH (e.g. append the profile's read/execute-granted dirs like `.local\bin`). Options + the full-path
    discriminator probe (`& "C:\Users\OMack\.local\bin\claude.exe"` — confirms claude RUNS under the sandbox,
    isolating PATH-resolution from a deeper claude-under-Low-IL problem) are in Current Focus → next_action.
    Touches the security-critical env-construction path → operator picks the direction before any edit.
  source: operator field run 2026-05-28T19:10 (verbatim, treated as data); code:
    crates/nono-cli/src/exec_strategy_windows/launch.rs:551-663 (build_child_env skip-list) + 681-694
    (append_windows_runtime_env curated PATH); crates/nono-cli/src/exec_strategy/env_sanitization.rs:64-76
    (should_skip_env_var)

- timestamp: 2026-05-28T18:30:00Z
  checked: FIELD VERIFICATION of the Option D′ hang fix — live Win11 build-26200, dev-layout v0.57.4
    `C:\Users\OMack\Nono\target\debug\nono.exe shell --profile claude-code --allow-cwd`, profile-covered
    cwd (`%USERPROFILE%\.claude`). Ran grandchildren that previously hung.
  found:
    HANG FIXED → `cmd /c echo HELLO and node -e "console.log(123)"` PRINTED `HELLO and node -e console.log(123)`
      and control RETURNED to the prompt. (Parsed by PowerShell as ONE `cmd /c echo <literal>`; node did not
      run as a separate process, so node is not independently re-verified — but a cmd grandchild executing +
      returning is the decisive proof the universal grandchild hang is GONE.)
    Issue A (cwd) → `cmd /c "echo HI > out.txt"` did NOT hang; cmd printed: `'\\?\C:\Users\OMack\.claude'
      CMD.EXE was started with the above path as the current directory. UNC paths are not supported.
      Defaulting to Windows directory. Access is denied.` — cmd RAN (no hang), rejected the `\\?\` cwd, fell
      back to C:\Windows, and the relative write was denied there. Repeated identically from `..\nono-poc`.
    Issue B (PATH) → `claude` → `CommandNotFoundException: The term 'claude' is not recognized as the name of a
      cmdlet, function, script file, or operable program.` claude works fine outside nono on the same host.
  implication: (1) The G1 hang is RESOLVED — grandchildren now execute and return. Option D′ verified on the
    live box. (2) "Access is denied" on the C:\Windows fallback write CONFIRMS Low-IL NO_WRITE_UP is still
    enforcing → no security regression from D′. (3) Two pre-existing, distinct issues are now visible: A (cwd
    `\\?\` verbatim prefix not stripped → cmd.exe unusable cwd) and B (`claude` not on the sandbox PATH /
    not resolved). Neither is a regression from D′ (the old hanging path carried the same cwd + env; the hang
    masked them). See Current Focus → next_action for root causes and the decision checkpoint.
  source: operator field run 2026-05-28 (verbatim transcript, treated as data); code: execution_runtime.rs:70-72,
    crates/nono-cli/src/exec_strategy_windows/launch.rs:998-1009 + 1284

- timestamp: 2026-05-28T15:33:38Z
  observation: Broker successfully spawned Low-IL child PID 38460; PowerShell prompt rendered. nono shell LAUNCH works on v0.57.3 (Phase 31 broker pattern intact). Distinct from resolved Phase 30 launch failure.
  source: user-supplied transcript (trigger)

- timestamp: 2026-05-28T15:33:38Z
  observation: "broker: console attach probe alloc_console_rc=0" — broker AllocConsole returned 0 (probe). "Low-IL primary token constructed". These are the Phase 31 broker mechanism logs. NOTE (static analysis): AllocConsole rc=0 means it FAILED because a console was ALREADY attached — i.e. the broker inherited nono.exe's real console. This is consistent with the H2-REFINED finding that the broker (and thence PowerShell) runs on nono.exe's real console, NOT on the ConPTY.
  source: user-supplied transcript (trigger) + crates/nono-shell-broker/src/main.rs:227-231

- timestamp: 2026-05-28
  observation: claude / claude --version works fine OUTSIDE nono in plain PowerShell (Medium-IL) on the same host. First-time attempt of claude-inside-nono-shell. Frozen with zero output (not an error, not a fast exit).
  source: user symptom-gathering answers

- timestamp: 2026-05-28T16:20:00Z
  checked: Static call-graph + console/stdio-wiring read of the live `nono shell` PTY path.
  found: The live `WindowsTokenArm::BrokerLaunch` arm (crates/nono-cli/src/exec_strategy_windows/launch.rs:1313-1522) (a) flips ONLY `pty_pair.input_write` and `pty_pair.output_read` — the PARENT-END ConPTY pipes — inheritable and whitelists them via PROC_THREAD_ATTRIBUTE_HANDLE_LIST (1361-1466); (b) spawns the broker with CreateProcessW using `CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT` ONLY (1491-1503) — no CREATE_NEW_CONSOLE, no PSEUDOCONSOLE, no STARTF_USESTDHANDLES; (c) the broker (crates/nono-shell-broker/src/main.rs:223-385) with no_pty=false spawns PowerShell via CreateProcessAsUserW(low_il_token) with EXTENDED_STARTUPINFO_PRESENT only and HANDLE_LIST=the two pipes — NO PSEUDOCONSOLE, NO STARTF_USESTDHANDLES.
  implication: The ConPTY `hpcon` created by `open_pty()` (pty_proxy_windows.rs:41-87) is NEVER attached to the broker or PowerShell. PSEUDOCONSOLE attachment exists ONLY in the legacy/structurally-unreachable else-branch (launch.rs:1523-1630, gated on `arm != BrokerLaunch`). Consequence: PowerShell inherits nono.exe's REAL console (not the ConPTY); the supervisor's ConPTY relay reads `pty.output_read`, a pipe nothing ever writes to. This is why the prompt renders (PowerShell ↔ real console directly) and why the ConPTY relay is a dead path.
  source: crates/nono-cli/src/exec_strategy_windows/launch.rs:1313-1630; crates/nono-shell-broker/src/main.rs:223-385; crates/nono-cli/src/pty_proxy_windows.rs:41-87

- timestamp: 2026-05-28T16:20:00Z
  checked: Supervisor interactive relay threads for `nono shell`.
  found: `start_streaming` (supervisor.rs:433-441) routes interactive_shell=true to `start_interactive_terminal_io` (865-995), which spawns (1) an OUTPUT thread reading `pty.output_read` → `std::io::stdout()` (903-922) and (2) an INPUT thread reading `std::io::stdin()` → `pty.input_write` (926-945), plus a resize-poll thread. Because the ConPTY child end is never attached (prior evidence), the output thread blocks on a pipe nothing writes, and the input thread blocks in `std::io::stdin().read()` on nono.exe's REAL console — the SAME console PowerShell and (later) claude.exe inherit and use directly.
  implication: ROOT-CAUSE MECHANISM (pending field confirmation): nono.exe's supervisor input thread and the grandchild claude.exe are both bound to the single Win32 console input queue. PowerShell at a line-input prompt tolerates this (interleaved line reads). But claude (Node/Ink TUI) switches the console to raw mode and expects exclusive ownership of console input events; with the supervisor's blocking ReadFile contending for the same input queue, claude's raw-mode reads/first-frame render stall → zero output, indefinite hang. No loader failure, so no 0xC0000142 (distinguishes this from Phase 30).
  source: crates/nono-cli/src/exec_strategy_windows/supervisor.rs:433-441, 865-995

- timestamp: 2026-05-28T16:20:00Z
  checked: Whether the untagged post-v2.7 fixes (d8b7ce00 broker HANDLE_LIST dedup; 005b4c9e no-PTY stdout echo) touch this path.
  found: Both fixes target the `BrokerLaunchNoPty` / no-PTY relay (`start_logging`, broker `--no-pty` STARTF_USESTDHANDLES bind). The `nono shell` interactive path is `BrokerLaunch` (PTY) → `start_interactive_terminal_io` — a different arm/relay, untouched by those commits.
  implication: H4 confirmed LOW. The dev-layout v0.57.4-candidate binary is expected to hang identically on `nono shell`. v0.57.4's release scope (the `nono run` no-PTY fixes) is INDEPENDENT of this `nono shell` bug; this bug neither blocks nor is fixed by that release.
  source: crates/nono-cli/src/exec_strategy_windows/launch.rs:1631+ (BrokerLaunchNoPty arm); resolved/nopty-broker-stdout-swallowed.md; resolved/broker-nopty-createproc-gle87.md

- timestamp: 2026-05-28T17:00:00Z
  checked: Operator field probes P1-P4 on a REAL PowerShell console, inside a working `nono shell --profile claude-code --allow-cwd` Low-IL sandbox (verbatim, treated as data).
  found:
    P1 `claude --version`            → HANGS (zero output).
    P2 `cmd /c echo HELLO_GRANDCHILD` → HANGS (zero output).
    P3 `node -e "console.log(123)"`  → HANGS (zero output).
    P4 (second normal window, while bare `claude` hung) Get-Process claude,node | ft Id,Name,SI →
       claude (PID 19648) ALIVE, SI=1; NO node row.
  implication: DECISIVE REFINEMENT. (1) The hang is UNIVERSAL to every grandchild PowerShell spawns,
    not specific to claude's raw-mode/Ink TUI — P2 `cmd /c echo` is a flat console app with NO raw mode,
    NO TTY takeover, NO Node, NO CLR, and it STILL hangs with zero output. This REFUTES the prior
    H2-REFINED "raw-mode console takeover contention" sub-explanation as too narrow. (2) claude.exe is
    ALIVE (SI=1), not crashed — so this is NOT the Phase 30 0xC0000142 loader death (H1). The grandchild
    is BLOCKED/HUNG, not dying. (3) The grandchild never returns control to the PowerShell prompt either,
    so the block is in the grandchild's own startup/IO path. PowerShell's REPL works (it drives the REAL
    console directly for line input), but every child it spawns hangs. The differentiator is what changes
    between PowerShell driving its own prompt vs a freshly-spawned Low-IL grandchild registering with the
    console subsystem.
  source: operator checkpoint response 2026-05-28 (field-verification)

- timestamp: 2026-05-28T17:05:00Z
  checked: Win32 semantics — what std handles a child gets under CreateProcess with bInheritHandles=TRUE +
    PROC_THREAD_ATTRIBUTE_HANDLE_LIST but WITHOUT STARTF_USESTDHANDLES (the broker's PTY-path spawn shape),
    and how console-subsystem registration works for a NEW Low-IL grandchild of a Low-IL PowerShell whose
    conhost was created at Medium IL. Cross-checked against Microsoft "Inheritance (Processes and Threads)",
    rprichard/win32-console-docs, and the Phase 31 RESEARCH §2d Project Zero "In-Console-Able" two-failure-mode
    analysis.
  found: (a) PROC_THREAD_ATTRIBUTE_HANDLE_LIST does NOT govern traditional console handles — when
    STARTF_USESTDHANDLES is unset and the child attaches to the parent's console, the child's std handles
    are the REAL console handles, NOT the pipes in the HANDLE_LIST. So PowerShell (and its grandchildren)
    get nono.exe's real-console std handles, NOT the ConPTY parent-end pipes. (b) nono-cli NEVER calls
    SetStdHandle / FreeConsole / AttachConsole anywhere (grep), so nono.exe keeps its real console and the
    broker inherits it; the broker's PTY-path spawn (no_pty=false) leaves dwFlags without
    STARTF_USESTDHANDLES (only the no-PTY branch at main.rs:341-357 sets it). (c) Project Zero documents TWO
    CSRSS/Low-IL console failure modes: client-side (NtAlpcConnectPort from ConClntInitialize in the child's
    DllMain → STATUS_ACCESS_DENIED, the Phase 30 0xC0000142 crash) and server-side (conhost cannot open the
    Low-IL process to complete attachment). RESEARCH A1 (PoC- + production-validated) only proves the DIRECT
    broker child (PowerShell) skips ConClntInitialize because it INHERITS the already-attached console — A1
    says NOTHING about a fresh grandchild that PowerShell spawns and that must register with conhost as a NEW
    Low-IL console client.
  implication: ROOT-CAUSE MECHANISM (refined, universal-fit): every grandchild PowerShell spawns is a NEW
    Low-IL console-subsystem client that must connect to the conhost serving nono.exe's REAL console — a
    conhost running at Medium IL. The direct-child A1 inheritance skip does NOT extend to grandchildren.
    The grandchild's console-client registration (ConClntInitialize → NtAlpcConnectPort to conhost, and/or
    conhost's server-side open-back of the Low-IL grandchild) blocks at the IL boundary and never completes
    AND never fails — producing a silent indefinite HANG with zero output and the grandchild ALIVE (P4),
    distinct from the Phase 30 client-side STATUS_ACCESS_DENIED that crashes with 0xC0000142. PowerShell
    itself is exempt only because it inherited the console at creation (A1). This refutes the prior
    "supervisor stdin-thread contends for the console input queue" framing as the primary cause: P2
    `cmd /c echo` does no raw-mode input and still hangs, and the block is in the grandchild's startup, not
    in input arbitration.
  blind_spot: ONE residual ambiguity that changes the fix and that static analysis cannot resolve from a
    no-console MSYS host: is the grandchild blocked (G1) UPSTREAM of its first stdout write (console-client
    registration / handle setup), or (G2) AT its first stdout WriteFile to an inherited pipe whose read end
    is unserviced? The Win32 finding (a) argues strongly for G1 (grandchildren get the real console, not a
    pipe) and against G2, but a single field probe (`cmd /c "echo HI > out.txt"`) cleanly separates them and
    must gate the fix on this security-critical launch path. See Proposed verification below.
  source: WebSearch (MS "Inheritance (Processes and Threads)" + rprichard/win32-console-docs); grep of
    crates/nono-cli/src for SetStdHandle/FreeConsole/AttachConsole (zero hits); .planning/quick/260508-lqh
    .../RESEARCH.md §1b A1 + §2d Project Zero; crates/nono-shell-broker/src/main.rs:302-385;
    crates/nono-cli/src/exec_strategy_windows/launch.rs:1313-1522; crates/nono-cli/src/pty_proxy_windows.rs:41-87

- timestamp: 2026-05-28T17:40:00Z
  checked: DISCRIMINATOR field probe (operator, REAL PowerShell console, inside a working
    `nono shell --profile claude-code --allow-cwd` Low-IL sandbox, profile-covered cwd):
    `cmd /c "echo HI > out.txt"`.
  found: STILL HUNG — control never returned to the prompt AND no `out.txt` file was created.
  implication: DECISIVE — **G1 confirmed, G2 refuted.** The grandchild (`cmd /c`) blocks UPSTREAM of its
    first stdout/file write: it never reaches the `WriteFile` that would create out.txt, and never returns.
    This eliminates the G2 sub-mechanism (block AT a console-bound stdout write to an unserviced handle —
    which would have let the redirected file write succeed and the process exit). The block is in the
    grandchild's cross-IL console-client REGISTRATION / handle setup (ConClntInitialize → NtAlpcConnectPort
    to the Medium-IL conhost serving nono.exe's real console, and/or conhost's server-side open-back of the
    Low-IL grandchild — Project Zero "In-Console-Able" server-side mode), which neither completes nor fails.
    Note the redirect `> out.txt` only rebinds stdOUT; the grandchild still attaches to the inherited real
    console for the subsystem registration that happens before any user code runs, so the redirect cannot
    bypass the registration hang. ROOT CAUSE IS NOW SINGULAR (no residual G1/G2 ambiguity). Fix locus: how
    the broker establishes the console/stdio for the WHOLE Low-IL tree so a fresh grandchild never registers
    cross-IL with a Medium-IL conhost → Option D′ (pipe stdio, proven on `nono run`) as the low-risk unblock,
    or Option B′ (real ConPTY through broker) as the TUI-preserving fix that needs a Phase-30-0xC0000142
    re-trip PoC first.
  source: operator checkpoint response 2026-05-28T17:40 (field-verification); prior Win32 finding
    Evidence 2026-05-28T17:05:00Z finding (a)

## Investigation Summary (reasoning checkpoint — static analysis, pending field confirmation)

**Verified call graph (`nono shell --profile claude-code --allow-cwd`):**

```
nono.exe (Medium IL, runs in user's REAL PowerShell console)
  → open_pty(): creates a ConPTY (hpcon) over two pipe pairs; keeps PARENT ends
       input_write (parent writes child stdin), output_read (parent reads child stdout)
       — child ends were handed to CreatePseudoConsole and closed locally
  → spawn_windows_child, arm = BrokerLaunch (PTY + claude-code profile)
       h_token = null (broker runs at nono.exe's Medium-IL identity)
       HANDLE_LIST whitelists [input_write, output_read]  (the PARENT-end pipes)
       NO PSEUDOCONSOLE attribute, NO STARTF_USESTDHANDLES
       CreateProcessW(broker, EXTENDED_STARTUPINFO_PRESENT only)
         → broker inherits nono.exe's REAL console  (AllocConsole rc=0 = already attached)
  → broker (Medium IL): builds Low-IL primary token; no_pty=false
       CreateProcessAsUserW(low_il_token, powershell.exe, EXTENDED_STARTUPINFO_PRESENT only,
                            HANDLE_LIST=[input_write, output_read])
         → PowerShell (Low IL) inherits the BROKER's console = nono.exe's REAL console
            (KernelBase skips CSRSS attach because a console is already inherited — A1)
  → PowerShell renders its prompt to the REAL console; user types into the REAL console  (WORKS)
  ── meanwhile, in nono.exe ──
  → start_interactive_terminal_io():
       OUTPUT thread: read pty.output_read → stdout   (BLOCKS: nothing ever writes the ConPTY)
       INPUT thread:  read std::io::stdin() → pty.input_write   (BLOCKS on the REAL console)
  ── user types any subprocess (claude, cmd /c echo, node) ──
  → PowerShell CreateProcessW(grandchild.exe)  → grandchild is a NEW Low-IL console-subsystem process
  → grandchild ConClntInitialize → must register as a NEW client with the conhost serving the REAL console
       (that conhost runs at MEDIUM IL — created for nono.exe). A1 does NOT cover this fresh registration;
       A1 only covered PowerShell's INHERITED console (no-op registration).
  → cross-IL client registration (Low-IL grandchild ↔ Medium-IL conhost ALPC connect, and/or conhost's
       server-side open-back of the Low-IL grandchild — Project Zero "In-Console-Able" server-side mode)
       BLOCKS at the IL boundary; neither completes nor fails  → ZERO OUTPUT, INDEFINITE HANG, grandchild
       ALIVE (P4). UNIVERSAL across grandchild types (P2 cmd /c echo confirms — no raw mode needed).
```

**Why the ConPTY is dead in this path:** A pseudoconsole is bound to a process ONLY via `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE = hpcon` at CreateProcess time. The live BrokerLaunch arm never sets it (it lives only in the legacy `else` branch at launch.rs:1523-1630, which is gated `arm != BrokerLaunch` and described in-source as "structurally unreachable today"). Whitelisting the parent-end pipes for inheritance does not attach the ConPTY — those are the wrong ends, and nothing binds them as the child's stdio either (no_pty=false ⇒ no STARTF_USESTDHANDLES). Critically, per Microsoft "Inheritance (Processes and Threads)" + rprichard/win32-console-docs, `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` does NOT govern console handles — so PowerShell (and grandchildren) attach to nono.exe's REAL console, not to the listed pipes. This is also why the shell is usable at all.

**Why this is a different bug from Phase 30 (and why H1 is eliminated, not just downgraded):** Phase 30 was an immediate `0xC0000142 STATUS_DLL_INIT_FAILED` (a LOADER failure during the DIRECT child's DllMain, before main(), from the client-side ConClntInitialize STATUS_ACCESS_DENIED). This bug is a silent indefinite HANG with zero output and no exit, and P4 confirms the grandchild is ALIVE (SI=1) — initialized, just unable to complete console-client registration. The broker pattern fixed Phase 30 for the DIRECT child (A1 inheritance skip). The novel finding here: the A1 skip does NOT extend to GRANDCHILDREN, which register fresh cross-IL with the Medium-IL conhost and HANG (pending connect) rather than crash. The signature difference (universal hang + alive vs fast 0xC0000142 exit) is the tell, and P2 (`cmd /c echo`, no raw mode) proves it is universal, not TUI-specific.

## Proposed verification (USER-CHECKPOINT — DO NOT FIX YET)

P1–P4 are DONE (see Evidence 2026-05-28T17:00:00Z). ONE remaining discriminating probe separates the two
viable sub-mechanisms (G1 = block UPSTREAM of first write, in console-client registration; G2 = block AT
first console/stdout write to an unserviced handle). They imply DIFFERENT fixes (Option B′/D′ vs targeted
stdio-relay), so this must gate the edit. Run on the REAL PowerShell console inside a working
`nono shell --profile claude-code --allow-cwd`, from a profile-covered cwd (e.g. `%USERPROFILE%\.claude`,
NOT bare `%USERPROFILE%` — D-52-01 cwd-coverage gate):

- **DISCRIMINATOR (primary):** `cmd /c "echo HI > out.txt"`  then (in the same sandboxed prompt, if it
  returns) `type out.txt`. Report: (1) does control return to the prompt? (2) does `out.txt` exist with
  `HI`? (3) does `type out.txt` print `HI` (or also hang)?
- **BACKUP (only if the primary is ambiguous):** `cmd /c "echo HI 1> out.txt 2>&1"` (force both stdout and
  stderr to the file) — distinguishes a console-write block on stdout vs stderr from a pure
  registration-upstream block.

Interpretation:
- **out.txt written + prompt returns** → grandchild RAN fully; block is only on console-bound stdout
  (G2) → targeted stdio-relay/wiring fix (Option D′-adjacent).
- **out.txt NOT written + still hangs** → grandchild blocks UPSTREAM of first write, in cross-IL
  console-client registration (G1) → Option D′ (pipe stdio, proven on `nono run`) as the unblock, or
  Option B′ (real ConPTY through broker — needs a Phase-30-0xC0000142-re-trip PoC) for TUI preservation.
- **P4 (second window, while bare `claude` hangs):** `Get-Process claude,node -ErrorAction SilentlyContinue | Format-Table Id,Name,SI`  (expected: claude.exe ALIVE under H2-REFINED; ABSENT would reopen H1)

Interpretation is in Current Focus → **expecting**.

## Ranked fix options (RE-EVALUATED against the universal-grandchild signature; do NOT apply until G1/G2 confirmed)

All options are Windows-only (`exec_strategy_windows/` + `nono-shell-broker/`), so CLAUDE.md's cross-target clippy MUST/NEVER rule applies before any commit: verify with `cargo clippy --workspace --target x86_64-unknown-linux-gnu` AND `--target x86_64-apple-darwin`, or mark PARTIAL and defer per the cross-target checklist (Windows-host `cargo check` is NOT a clippy substitute and does NOT exercise these cfg branches). NonoError + `?` only, no unwrap/expect, fail-secure (never weaken Low-IL NO_WRITE_UP to fix I/O wiring), DCO sign-off `Oscar Mack Jr <oscar.mack.jr@gmail.com>`.

**Prior Option A is now INSUFFICIENT.** "Make the supervisor stop contending for console input" cannot fix a universal grandchild hang: P2 `cmd /c echo` does NO console input and still hangs. The block is in the grandchild's console-client REGISTRATION (or its first console/stdout write), not in input-queue arbitration. Removing the supervisor stdin thread might be necessary hygiene but will NOT unblock grandchildren. Re-ranked options below address the actual locus — the stdio/console the broker hands the Low-IL tree.

1. **Option B′ (PREFERRED if G1 confirmed — "give the Low-IL tree a console it can use without cross-IL registration"): attach the ConPTY through the broker so the WHOLE Low-IL subtree talks to nono.exe's ConPTY, not to a Medium-IL conhost.** Pass `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`(hpcon) through to the broker→PowerShell spawn so PowerShell (and its inherited grandchildren) run on the ConPTY whose server end is nono.exe's supervisor relay (at nono.exe's IL). A grandchild then registers with the ConPTY pseudoconsole owned by the (Medium-IL but cooperating) supervisor relay rather than connecting cross-IL to the real-console conhost. This also makes `start_interactive_terminal_io`'s relay LIVE (it currently reads a dead pipe). Pros: single well-defined I/O path; resize/attach become real; addresses the universal grandchild block at its source. Cons/RISK: this is precisely the PSEUDOCONSOLE-through-a-Low-IL-spawn shape Phase 30 found brittle (0xC0000142) for the DIRECT path — the broker pattern was adopted SPECIFICALLY to avoid it. Must PoC that broker-mediated ConPTY attach does not re-trip CSRSS at Low-IL for the direct child AND that grandchildren register against the ConPTY cleanly. Architectural; candidate for a planned phase, NOT a hotfix. (Open Question #1 in RESEARCH §8 is exactly this and was never resolved.)

2. **Option D′ (PREFERRED if G2 confirmed, or as the lower-risk unblock — "pipe stdio for the whole tree, no ConPTY"): run the broker→PowerShell spawn with anonymous-pipe stdio (`STARTF_USESTDHANDLES`) the way the WORKING `nono run` no-PTY path already does** (`BrokerLaunchNoPty` arm + broker `--no-pty` bind at main.rs:341-357, plus the supervisor's serviced relay — see resolved `nopty-broker-stdout-swallowed` + `broker-nopty-createproc-gle87`). PowerShell and its grandchildren get inherited PIPE std handles (no console-subsystem registration at all → no cross-IL conhost barrier). The supervisor relay services those pipes (the `nono run` path proves this works end-to-end after the two post-v2.7 fixes). Pros: reuses a PROVEN-WORKING wiring; no PSEUDOCONSOLE/CSRSS fragility; preserves Low-IL NO_WRITE_UP exactly (token shape unchanged; only stdio binding changes — same security analysis as Phase 51 T-51B-02). Cons: NO TTY raw mode / alternate screen / resize → claude's full Ink TUI will render degraded or fall back to non-TUI; loses the "TUI-in-sandbox" promise. This is RESEARCH §7b, previously rejected for Phase 30's TUI acceptance criterion — but it is the honest, low-risk unblock for "grandchildren run at all".

3. **Option C′ (scope/expectation fix, interim): document `nono shell` grandchild execution on Windows as a known limitation; recommend `nono run -- <cmd>` (the working no-PTY path, v0.57.4) for now.** Pros: zero code risk; honest; `nono run` already works. Cons: `nono shell` is effectively unusable for spawning ANY subprocess on Windows until B′ or D′ lands. Acceptable only as an interim banner alongside scheduling B′/D′.

**Recommendation (pending G1/G2 discriminator):**
  - If the discriminator shows **G1** (grandchild blocks UPSTREAM of first write — no out.txt, no prompt): the block is console-client registration. **Option D′** (pipe stdio, proven) is the pragmatic unblock; **Option B′** (real ConPTY through broker) is the proper TUI-preserving fix but carries the Phase 30 0xC0000142 re-trip risk and needs a PoC first → planned phase.
  - If the discriminator shows **G2** (out.txt written, prompt returns — grandchild ran, only console stdout blocked): the block is stdout/console-write topology → a targeted stdio-relay wiring fix (closest to **Option D′**'s relay, possibly without abandoning ConPTY).
  - Either way, confirm with the single field probe before editing this security-critical launch path.

## Relationship to the v0.57.4 release (Phase 53)

INDEPENDENT (assessment re-confirmed after P1-P4). The v0.57.4 release is about the `nono run` no-PTY fixes (broker HANDLE_LIST dedup `d8b7ce00` + no-PTY stdout echo `005b4c9e`), which live on the `BrokerLaunchNoPty` arm + broker `--no-pty` STARTF_USESTDHANDLES bind + `start_logging` relay. This `nono shell` hang is on the `BrokerLaunch` (PTY) arm + `start_interactive_terminal_io` relay — untouched by those commits. The refined root cause (cross-IL grandchild console-client registration hang) is squarely on the PTY path. This bug does NOT block the v0.57.4 release and is NOT fixed by it. NOTE: if the fix lands as **Option D′** (pipe stdio for the `nono shell` tree), it would REUSE the very `BrokerLaunchNoPty`/`--no-pty` wiring v0.57.4 ships and de-risk that fix path — another reason to ship v0.57.4 on its own merits first, then build the `nono shell` fix on top of the proven no-PTY relay.


## Fix Applied — Option D′ (pipe stdio for the whole Low-IL tree)

**Operator decision (2026-05-28):** Option D′ — give the whole Low-IL `nono shell`
tree pipe stdio (`STARTF_USESTDHANDLES`), reusing the PROVEN-WORKING `nono run`
no-PTY wiring (`WindowsTokenArm::BrokerLaunchNoPty` arm + broker `--no-pty` bind +
the supervisor's serviced relay with the post-v2.7 `nopty-broker-stdout-swallowed`
+ `broker-nopty-createproc-gle87` fixes). Grandchildren inherit PIPE std handles →
no console-subsystem registration → no cross-IL Medium-IL-conhost barrier. Operator
ACCEPTED the known cost: loss of raw-mode TUI (claude renders degraded / non-TUI
under `nono shell` on Windows). Option B′ (real ConPTY through broker, TUI-preserving)
was deliberately deferred.

### Mechanism of the fix

The fix routes the interactive `nono shell` path through the EXISTING, proven
`BrokerLaunchNoPty` pipe-stdio wiring instead of the dead `BrokerLaunch` (ConPTY)
arm. Three coordinated, Windows-only changes; the Low-IL NO_WRITE_UP token shape is
UNCHANGED (only the stdio binding differs — same security analysis as Phase 51
T-51B-02; FAIL-SECURE preserved):

1. **`crates/nono-cli/src/supervised_runtime.rs` — `should_allocate_pty` gate.**
   Added a `prefers_low_il_broker` parameter (sourced from
   `config.prefers_low_il_broker`, i.e. `profile.windows_low_il_broker`). On
   Windows, an interactive shell with the Low-IL broker opt-in now SKIPS ConPTY
   allocation (`session.interactive_pty && !prefers_low_il_broker`). With
   `pty = None`, `select_windows_token_arm(has_pty=false, prefers_low_il_broker=true,
   has_session_sid=true)` resolves to `BrokerLaunchNoPty` — the proven anonymous-pipe
   stdio path. The `prefers_low_il_broker` read is `cfg(target_os = "windows")`-gated
   (the field exists only on the Windows `ExecConfig`); non-Windows passes `false`.

2. **`crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — `start_streaming`.**
   The interactive branch now splits on `self.pty.is_some()`:
   - PTY present → `start_interactive_terminal_io()` (classic ConPTY relay, Unix-parity
     TUI path — UNCHANGED; non-opted-in Windows profiles still use it).
   - PTY absent → new `start_interactive_pipe_io()` (Option D′).
   `interactive_shell` STAYS true for any foreground `nono shell` (no new
   `SupervisorConfig` field needed — the PTY-presence check carries the split).

3. **`crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — new
   `start_interactive_pipe_io()`.** Reuses the proven no-PTY relay plus a foreground
   stdin pump the one-shot `nono run` path never needs:
   - `start_logging()` — drains `detached_stdio.stdout_read` (child stdout, stderr
     merged at spawn) → session log + foreground console (post-v2.7
     `nopty-broker-stdout-swallowed` echo) + `nono attach` pipe. UNCHANGED.
   - `start_data_pipe_server()` — `nono attach` data pipe → child stdin. UNCHANGED.
   - foreground stdin pump (NEW) — `std::io::stdin()` → `detached_stdio.stdin_write`,
     mirroring the ConPTY relay's stdin thread but writing to the pipe, so the
     sandboxed REPL is interactive. `NonoError` + `?`, no `unwrap`/`expect`,
     best-effort writes (Pitfall 1) — a broken pipe ends the pump without panicking.

The broker (`crates/nono-shell-broker/src/main.rs`) required NO change — its
`--no-pty` `STARTF_USESTDHANDLES` bind + HANDLE_LIST dedup were already production
code from Phase 51 + the post-v2.7 fixes.

### Files changed
- `crates/nono-cli/src/supervised_runtime.rs` (PTY gate + threading; 5 unit tests).
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` (`start_streaming` split +
  `start_interactive_pipe_io`).

### Verification status

- **Build (Windows host):** PASS — `cargo build -p nono-cli -p nono-shell-broker`
  finished clean.
- **Unit tests (Windows host):** PASS —
  - `supervised_runtime::tests` 5/5 PASS (incl. new
    `windows_low_il_broker_interactive_skips_pty` and regression guard
    `windows_non_low_il_interactive_still_allocates_pty`).
  - `nono-shell-broker` 20/20 PASS (no-PTY arg-parse + HANDLE_LIST dedup intact).
- **Clippy (Windows host):** PASS — `cargo clippy -p nono-cli -p nono-shell-broker
  -- -D warnings -D clippy::unwrap_used` clean.
- **Cross-target clippy (Linux + macOS):** **PARTIAL / SKIPPED.**
  `supervised_runtime.rs` is in-scope per `.planning/templates/cross-target-verify-checklist.md`
  (contains `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` blocks).
  Cross-target clippy gate SKIPPED on Windows dev host due to missing toolchain
  (x86_64-unknown-linux-gnu and x86_64-apple-darwin both fail at the C cross-compiler
  stage: `x86_64-linux-gnu-gcc` / `cc` not found, required by `ring` / `aws-lc-sys`).
  The live GH Actions Linux Clippy + macOS Clippy lanes on the head SHA are the
  decisive signal. Marked PARTIAL pending CI confirmation. (My `cfg` gating is
  symmetric — non-Windows passes `prefers_low_il_broker = false` and the
  `should_allocate_pty` `cfg!` branch never reads it — so no Unix-cfg behavior change
  is expected, but the gate is NOT discharged until CI confirms.)
- **Field verification (live Win11 build-26200):** **REQUIRED — NOT YET DONE.** This
  is the decisive signal for "the hang is gone". The dev-layout
  `target\debug
ono.exe` (or a fresh `targetelease` build) must be run from a
  profile-covered cwd (e.g. `%USERPROFILE%\.claude`, NOT bare `%USERPROFILE%` —
  D-52-01 cwd-coverage gate), then inside the sandboxed shell run the original repros:
    1. `nono.exe shell --profile claude-code --allow-cwd`
    2. `cmd /c "echo HI > out.txt"`  → EXPECT: out.txt created with `HI`, prompt returns
       (G1 unblocked).
    3. `cmd /c echo HELLO_GRANDCHILD` → EXPECT: prints `HELLO_GRANDCHILD`, prompt returns.
    4. `node -e "console.log(123)"` → EXPECT: prints `123`, prompt returns.
    5. `claude` (interactive) → EXPECT: launches and is usable (degraded/non-TUI line
       mode is the ACCEPTED Option D′ cost — NOT a failure). `claude --version` →
       EXPECT: prints the version and exits.
  If any grandchild still hangs, the cross-IL registration is not the only block and
  the session must reopen.

### Relationship to v0.57.4 / Phase 53
INDEPENDENT (confirmed). This `nono shell` fix lives on the interactive relay split +
the PTY gate; it REUSES the `BrokerLaunchNoPty` wiring v0.57.4 ships but does not
block or depend on that release. nono.exe changed → any pre-built v0.57.x MSIs are
stale and must be rebuilt before distribution (dev-layout `target` binaries are current).

## Eliminated

- **H1 (CSRSS 0xC0000142 grandchild DEATH):** CONFIRMED-ELIMINATED by field probe P4 (2026-05-28).
  evidence: While bare `claude` was hung, a second normal PowerShell window showed `Get-Process claude` →
    claude.exe PID 19648 ALIVE, SI=1. The grandchild is NOT dying with STATUS_DLL_INIT_FAILED (0xC0000142,
    the Phase 30 client-side ConClntInitialize STATUS_ACCESS_DENIED crash). It is alive and BLOCKED/HUNG.
    NOTE: the CSRSS/conhost integrity barrier is STILL the suspected root-cause locus — but it manifests as
    a HANG (pending/never-completing console-client registration for a NEW Low-IL grandchild), not as the
    immediate loader-death crash that defines H1. H1 specifically (grandchild dies 0xC0000142) is eliminated;
    the refined hypothesis (grandchild HANGS in console-client registration) supersedes it.
  timestamp: 2026-05-28T17:00:00Z

- **H2-REFINED sub-explanation "raw-mode console-input contention is the primary cause":** ELIMINATED by P2.
  evidence: P2 `cmd /c echo HELLO_GRANDCHILD` — a flat console app with NO raw mode, NO TTY takeover, NO
    Node/CLR — HANGS with zero output. The hang is therefore UNIVERSAL to every grandchild, not specific to
    claude's raw-mode/Ink TUI. The supervisor's blocking stdin reader contending for the console input queue
    cannot be the primary cause, because `cmd /c echo` never reads console input in raw mode and still hangs.
    The supervisor stdin thread MAY still be a contributing irritant, but it is NOT the universal blocker.
    (The broader H2-REFINED finding — ConPTY never attached, PowerShell/grandchildren on the real console —
    REMAINS VALID and is load-bearing for the refined root cause.)
  timestamp: 2026-05-28T17:00:00Z
