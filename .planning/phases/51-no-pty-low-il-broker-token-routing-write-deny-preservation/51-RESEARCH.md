# Phase 51: No-PTY Low-IL broker + token routing + write-deny preservation — Research

**Researched:** 2026-05-26
**Domain:** Windows process token architecture, integrity levels, anonymous-pipe stdio, profile-field extension
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** The Low-IL broker route is a **profile-gated opt-in** via a new profile field (a boolean, Windows-only-meaningful, a no-op on Linux/macOS — cf. the existing `unsafe_macos_seatbelt_rules` precedent). Chosen over a blanket "all non-PTY run → broker" change and over a binary-shape heuristic.
- **D-02:** **Profile-only for v2.7** — no per-invocation CLI override flag (`--low-il-broker`/etc.). The field threads into `select_windows_token_arm` as a new input (e.g. `prefers_low_il_broker: bool`); when false, the existing `WriteRestricted` branch is taken unchanged.
- **D-03:** **Only the `claude-code` built-in profile** sets the field in v2.7.
- **D-04:** The no-PTY broker uses **anonymous-pipe stdio, supervisor-relayed** — nono.exe creates the pipes, passes the ends to the broker via `--inherit-handle`, and the supervisor relays child stdout/stderr. Reuses Phase 17 attach machinery. Chosen over inherited-console.
- **D-05:** Console-presence for the heavy-runtime child is handled by the broker's existing `AllocConsole` probe, independent of std-handle wiring.
- **D-06:** Add a **distinct `WindowsTokenArm::BrokerLaunchNoPty` variant**. Token construction identical to `BrokerLaunch` (null `h_token`; broker self-degrades). The variant only signals downstream spawn wiring (anonymous pipes, no ConPTY). PTY-path tests keep asserting `BrokerLaunch`.
- **D-07:** **Real-spawn integration test** (REQ-WSRH-03): spawn an actual Low-IL child via the no-PTY broker; child attempts to write a Medium-IL-labeled temp file; assert the write fails with access-denied (kernel MIC pre-DACL check).
- **D-08:** **Hard-fail, no silent skip** — test FAILS loudly if it can't set up the labeled fixture or spawn the Low-IL child. Use a `%USERPROFILE%`/`%TEMP%` fixture path, NOT a drive-root path (WRITE_OWNER drive-root label-apply limitation).

### Claude's Discretion

- Exact profile-field name (`windows_low_il_broker` is a suggestion, not locked) and its placement in the profile/policy schema.
- The exact name/signature of the new `select_windows_token_arm` input and how the profile field is resolved into it.
- stdin wiring on the no-PTY path (inherit nono's stdin vs a fourth pipe) — `claude --version` needs no stdin; keep it simple.
- Whether the broker needs an explicit `--no-pty` CLI signal or can infer the mode from the handle set / absence of a ConPTY attribute.

### Deferred Ideas (OUT OF SCOPE)

- CLI override flag (`--low-il-broker` / `--no-low-il-broker`) for ad-hoc per-invocation routing — deferred with REQ-WSRH-AUDIT-01 (v2 / follow-on).
- Profile-wide heavy-runtime audit — which other built-in / heavy-runtime (Electron/Node/CLR) profiles hit the same gate (REQ-WSRH-AUDIT-01, explicitly deferred).
- Windows-host field validation of `claude --version` + `windows-poc-handoff.mdx` doc update — REQ-WSRH-04/06, Phase 52.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-WSRH-01 | `nono-shell-broker` must launch a child process **without** a ConPTY/PTY, inheriting stdio via anonymous pipes. | Broker is already stdio-agnostic (inherits whatever `--inherit-handle` passes); the no-PTY mode is a caller-side change (new `BrokerLaunchNoPty` arm + anonymous-pipe handle construction) with possibly a mode signal to the broker. |
| REQ-WSRH-02 | Non-PTY supervised path routes through broker's Low-IL primary token (no restricting SID) for the affected case; `WriteRestricted` arm still reachable (no blanket removal). | New arm in `select_windows_token_arm` gated on `prefers_low_il_broker && !is_detached && !has_pty && has_session_sid` inserts before the `has_session_sid → WriteRestricted` fall-through. |
| REQ-WSRH-03 | Low-IL child retains mandatory-label `NO_WRITE_UP` write-deny at OS level; regression test asserts write to Medium-IL-labeled path is denied. | Real-spawn integration test using `try_set_mandatory_label` on a `%USERPROFILE%`/`%TEMP%` fixture path and the broker's Low-IL child process. |
| REQ-WSRH-05 | No regression: plain `cmd/echo` still passes; `nono shell` PTY path and detached path unchanged; Windows CI green; cross-target Linux/macOS clippy clean. | D-06 `BrokerLaunchNoPty` variant keeps existing `BrokerLaunch` tests unchanged; cascade ordering preserves all prior arms. |

</phase_requirements>

---

## Summary

Phase 51 is a Windows-only security-hardening phase that extends the Phase 31 broker mechanism (which solved `STATUS_DLL_INIT_FAILED` for the PTY `nono shell` path) to the non-PTY `nono run` supervised path. The confirmed root cause (documented in `.planning/debug/claude-exe-dll-init-failed.md`) is that `nono run` currently routes through `WindowsTokenArm::WriteRestricted`, whose synthetic restricting SID `S-1-5-117-*` double-gates all WRITE-type access checks, killing heavy DllMain/bootstrap activity in the self-contained 234 MB `claude.exe`. The fix is architecturally straightforward because all pieces already exist: the broker binary (`nono-shell-broker`) already does Low-IL primary token construction, the Phase 17 anonymous-pipe attach machinery already exists in `launch.rs`, and `select_windows_token_arm` is already a pure-function cascade.

The three implementation axes are: (1) a new profile field `windows_low_il_broker` (name is discretionary) threaded through `ExecConfig` into `select_windows_token_arm` as a new boolean parameter; (2) a new `WindowsTokenArm::BrokerLaunchNoPty` variant dispatching anonymous-pipe spawn wiring instead of ConPTY; and (3) a real-spawn integration test asserting that the Low-IL child's write to a Medium-IL-labeled `%USERPROFILE%`/`%TEMP%` fixture file is kernel-denied. All three are independently parallelizable within a single wave.

Cross-target clippy is the most disciplined verification gate: because the touched files are purely under `exec_strategy_windows/` and `nono-shell-broker/`, the scope exception in `cross-target-verify-checklist.md` ("Does NOT apply to pure Windows-only files") likely applies to the implementation files, but any files also imported/re-exported via Unix-side modules must still pass the Linux/macOS clippy check. The `profile/mod.rs` and `policy.rs` changes (for the new field) WILL touch cross-platform Rust and must pass cross-target clippy.

**Primary recommendation:** Implement in three small parallel tasks — (A) profile field + `select_windows_token_arm` extension, (B) broker no-PTY spawn wiring, (C) real-spawn write-deny integration test — then a single verification wave covering both new unit tests and the cross-target clippy gate.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Profile-field parsing + inheritance | nono-cli (`profile/mod.rs`) | `data/nono-profile.schema.json` + `policy.json` | Profile is owned entirely by nono-cli (library is policy-free). |
| Token-arm selection | nono-cli (`exec_strategy_windows/launch.rs`) | — | `select_windows_token_arm` pure function; planner inputs come from ExecConfig. |
| ExecConfig new field (`prefers_low_il_broker`) | nono-cli (`exec_strategy_windows/mod.rs`) | Callers in `sandbox_prepare.rs` + `launch_runtime.rs` | Profile field resolves at exec-config construction time. |
| Broker no-PTY spawn wiring | nono-cli (`exec_strategy_windows/launch.rs`) | nono-shell-broker (may need `--no-pty` signal or can infer) | `spawn_windows_child`'s `BrokerLaunchNoPty` branch constructs anonymous pipes; broker already `AllocConsole`-probes independently. |
| Anonymous-pipe relay to supervisor stdout | nono-cli (`exec_strategy_windows/mod.rs`, `execute_supervised`) | Phase 17 `DetachedStdioPipes` + relay threads | Supervisor must read child stdout/stderr from pipes and forward to nono's stdout. |
| Low-IL primary token construction | nono library (`nono::create_low_integrity_primary_token`) | Called by broker (and indirectly by nono-cli via BrokerLaunchNoPty → broker) | Already lifted to lib in Phase 31 D-06; no changes needed. |
| Mandatory-label NO_WRITE_UP enforcement | OS kernel (MIC pre-DACL) + nono library (`try_set_mandatory_label`) | `AppliedLabelsGuard` (labels_guard.rs) in execution path | Enforcement is kernel-side; the test verifies it using the existing label primitives. |
| Write-deny integration test | nono-cli test module (`broker_dispatch_tests` or new module in `launch.rs`) | `nono::try_set_mandatory_label`, broker artifact | Real-spawn test following Phase 31 `broker_launch_assigns_child_to_job_object` pattern. |

---

## Standard Stack

No new external dependencies are introduced by this phase. All building blocks are already in the codebase. [VERIFIED: codebase grep]

### Existing primitives this phase reuses

| Symbol | Location | Purpose |
|--------|----------|---------|
| `WindowsTokenArm` enum | `launch.rs:1073` | Add new `BrokerLaunchNoPty` variant here |
| `select_windows_token_arm` | `launch.rs:1106` | Add new parameter `prefers_low_il_broker: bool` and new branch |
| `DetachedStdioPipes` | `launch.rs:56-171` | Anonymous-pipe struct already exists; reused for no-PTY broker path |
| `spawn_windows_child` / `BrokerLaunch` arm | `launch.rs:1261-1578` | Reference: new `BrokerLaunchNoPty` arm mirrors this, substituting pipes for ConPTY |
| `nono::create_low_integrity_primary_token` | `crates/nono/src/lib.rs:85` | Re-exported; broker calls it internally. No changes needed. |
| `nono::try_set_mandatory_label` | `crates/nono/src/sandbox/windows.rs:677` | Used in D-07 test to label the fixture file |
| `AppliedLabelsGuard` | `exec_strategy_windows/labels_guard.rs` | RAII label apply/revert; test may use `try_set_mandatory_label` directly |
| `Profile` struct | `crates/nono-cli/src/profile/mod.rs:2072` | Add new `windows_low_il_broker: bool` field with `#[serde(default)]` |
| `ExecConfig` struct | `exec_strategy_windows/mod.rs:129` | Add new `prefers_low_il_broker: bool` field |
| `broker_dispatch_tests` module | `launch.rs:2346` | Phase 31 precedent for runtime integration tests in this file |

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Low-IL primary token | Custom token duplication code | `nono::create_low_integrity_primary_token` | Lifted to library in Phase 31 D-06; single source of truth for both nono-cli and nono-shell-broker |
| Anonymous-pipe stdio pairs | Custom pipe creation | `DetachedStdioPipes::create()` in `launch.rs:72` | Already exists with correct inheritance flag handling (Phase 17) |
| PROC_THREAD_ATTRIBUTE_HANDLE_LIST wiring | Custom attribute list | Mirror the `BrokerLaunch` arm pattern in `launch.rs:1339-1467` | Already debugged for the broker path; reuse structure verbatim |
| Win32 command-line quoting | Custom quoting | `build_broker_command_line` in `launch.rs:1036` | Already handles spaces, embedded quotes, and UTF-16 encoding |
| Mandatory-label ACE test fixture | Custom SACL manipulation | `nono::try_set_mandatory_label` + `low_integrity_label_and_mask` | Existing tested primitives; WRITE_OWNER limitation already documented |

**Key insight:** Every Win32 primitive this phase needs (token construction, pipe creation, attribute list, command-line building, label application) already exists in the codebase in tested form. This phase is primarily a wiring task, not a new construction task.

---

## Architecture Patterns

### System Architecture Diagram

```
nono run --profile claude-code -- claude.exe
    │
    │  profile.windows_low_il_broker = true
    ▼
ExecConfig { prefers_low_il_broker: true, session_sid: Some(...), ... }
    │
    ▼
select_windows_token_arm(
    is_detached=false,
    has_pty=false,
    has_session_sid=true,
    caps_demand_low_il=...,
    prefers_low_il_broker=true   ← NEW parameter
)
    │
    └── returns BrokerLaunchNoPty   ← NEW arm (before WriteRestricted in cascade)
    │
    ▼
spawn_windows_child BrokerLaunchNoPty arm:
  1. Resolve broker sibling path (same as BrokerLaunch)
  2. Verify Authenticode (same as BrokerLaunch)
  3. Create DetachedStdioPipes (stdin/stdout/stderr pipe pairs)
  4. Build PROC_THREAD_ATTRIBUTE_HANDLE_LIST with pipe handles
  5. Build broker command line:
       broker.exe --shell <program> --shell-arg ... --inherit-handle <stdin_r> \
                  --inherit-handle <stdout_w> --inherit-handle <stderr_w> --cwd <cwd>
     (possibly --no-pty signal; see Open Questions)
  6. CreateProcessW(broker, CREATE_SUSPENDED, bInheritHandles=1)
  7. AssignProcessToJobObject, apply_resource_limits
  8. ResumeThread
  9. Return (child, Some(detached_stdio_pipes))
    │
    ▼
Supervisor relay:
  - Read child stdout from detached_stdio_pipes.stdout_read
  - Write to nono's stdout
  (Phase 17 relay machinery — already wired for detached path)
    │
    ▼
Broker (nono-shell-broker.exe) receives:
  - Inherits pipe handles as its child's stdin/stdout/stderr
  - AllocConsole probe (non-fatal if already attached)
  - create_low_integrity_primary_token()
  - CreateProcessAsUserW(low_il_token, EXTENDED_STARTUPINFO_PRESENT)
    │
    ▼
Low-IL child process (claude.exe):
  - No restricting SID
  - Mandatory-label NO_WRITE_UP enforced by OS kernel
  - DllMain/bootstrap WRITE-type accesses succeed
  - stdout/stderr flow back through broker's pipes to supervisor
```

### Recommended Project Structure (changes)

```
crates/nono-cli/src/exec_strategy_windows/
├── launch.rs          # WindowsTokenArm::BrokerLaunchNoPty variant
│                      # select_windows_token_arm: +prefers_low_il_broker param
│                      # spawn_windows_child: BrokerLaunchNoPty branch
│                      # new write_deny_low_il_child_test module
├── mod.rs             # ExecConfig: +prefers_low_il_broker field
crates/nono-cli/src/profile/
├── mod.rs             # Profile: +windows_low_il_broker: bool field
crates/nono-cli/src/
├── sandbox_prepare.rs # Resolve profile.windows_low_il_broker → ExecConfig
crates/nono-cli/data/
├── policy.json        # claude-code profile: "windows_low_il_broker": true
├── nono-profile.schema.json  # Add windows_low_il_broker property
```

### Pattern 1: Adding a new `WindowsTokenArm` variant + cascade arm

**What:** The cascade in `select_windows_token_arm` is a pure function that maps booleans to an enum variant. Adding a new arm requires: (a) new variant in the enum, (b) new parameter to the pure function, (c) new branch inserted at the right priority, (d) new unit test asserting the new branch fires for its inputs and that old tests still pass.

**When to use:** Every new Windows token strategy follows this pattern (Phase 15, 30, 31 each added an arm).

**Cascade position:** The new `BrokerLaunchNoPty` branch MUST be inserted AFTER `has_pty` (PTY path keeps `BrokerLaunch`) and BEFORE `has_session_sid → WriteRestricted` (so WriteRestricted stays reachable when `prefers_low_il_broker` is false).

```rust
// Source: crates/nono-cli/src/exec_strategy_windows/launch.rs:1106-1137
// Extended for Phase 51 — new signature:
pub(super) fn select_windows_token_arm(
    is_detached: bool,
    has_pty: bool,
    has_session_sid: bool,
    caps_demand_low_il: bool,
    prefers_low_il_broker: bool,  // NEW
) -> WindowsTokenArm {
    if is_detached {
        WindowsTokenArm::Null
    } else if has_pty {
        WindowsTokenArm::BrokerLaunch
    } else if prefers_low_il_broker && has_session_sid {
        // NEW: non-PTY supervised path with profile opt-in
        // routes through broker Low-IL arm instead of WriteRestricted.
        // WriteRestricted remains reachable when prefers_low_il_broker=false.
        WindowsTokenArm::BrokerLaunchNoPty
    } else if has_session_sid {
        WindowsTokenArm::WriteRestricted
    } else if caps_demand_low_il {
        WindowsTokenArm::LowIlPrimary
    } else {
        WindowsTokenArm::Null
    }
}
```

**New variant in enum (at line ~1100):**
```rust
/// Phase 51 D-06: non-PTY broker path. Token construction is identical to
/// `BrokerLaunch` (null h_token; broker self-degrades to Low IL). Distinct
/// variant so the downstream spawn wiring uses anonymous-pipe stdio instead
/// of ConPTY pipes, and so PTY-path tests keep asserting `BrokerLaunch`
/// (structurally proving Phase 31 PTY path is untouched).
BrokerLaunchNoPty,
```

### Pattern 2: Profile field for platform-gated opt-in (unsafe_macos_seatbelt_rules precedent)

**What:** A new `bool` field on `Profile`, parsed cross-platform (no-op on Linux/macOS), added to `ProfileDeserialize`, forwarded through `Profile::from(ProfileDeserialize)`, and propagated by `merge_profiles` (trivially: last-writer-wins for bools, or OR logic).

**Precedent location:** `crates/nono-cli/src/profile/mod.rs:2061-2072` (`unsafe_macos_seatbelt_rules`). [VERIFIED: codebase grep]

**Schema precedent:** `crates/nono-cli/data/nono-profile.schema.json:99-103`. [VERIFIED: codebase grep]

```rust
// In Profile struct (crates/nono-cli/src/profile/mod.rs):
/// Windows-only. When `true`, routes non-PTY supervised launches through the
/// Low-IL broker arm (`WindowsTokenArm::BrokerLaunchNoPty`) instead of the
/// `WRITE_RESTRICTED` arm. Preserves mandatory-label `NO_WRITE_UP` write-deny
/// while removing the restricting-SID double-gate that causes
/// `STATUS_DLL_INIT_FAILED` in heavy-runtime children (Electron, CLR, Node SEA).
/// Ignored on Linux and macOS (no-op; deserialize-only).
/// Only set in the `claude-code` built-in profile for v2.7.
#[serde(default)]
pub windows_low_il_broker: bool,
```

```json
// In nono-profile.schema.json:
"windows_low_il_broker": {
  "type": "boolean",
  "description": "Windows-only. Routes non-PTY supervised launches through the Low-IL broker arm instead of WRITE_RESTRICTED, eliminating STATUS_DLL_INIT_FAILED for heavy-runtime children. Ignored on Linux and macOS."
}
```

```json
// In policy.json claude-code profile (after "interactive": true):
"windows_low_il_broker": true
```

### Pattern 3: BrokerLaunchNoPty spawn wiring

**What:** In `spawn_windows_child`, the new `BrokerLaunchNoPty` match arm creates a `DetachedStdioPipes`, passes the three child-end handles to the broker via `--inherit-handle`, and returns the pipes alongside the child handle so the supervisor's relay loop can forward stdout/stderr.

**Key difference from BrokerLaunch:** Uses `DetachedStdioPipes::create()` and passes `stdin_read`, `stdout_write`, `stderr_write` as the three `--inherit-handle` values instead of `pty_pair.input_write` + `pty_pair.output_read`. ConPTY is absent. The `BrokerLaunch` arm's PROC_THREAD_ATTRIBUTE_HANDLE_LIST pattern is reused verbatim.

**Stdin wiring (Claude's Discretion resolved):** For `claude --version` and similar non-interactive runs, the simplest approach is to create all three pipe pairs but not actively write to stdin. The supervisor closes its `stdin_write` end immediately after spawn so the child sees EOF on stdin. This is consistent with the Phase 17 detached path pattern.

**Broker mode signal (Claude's Discretion resolved):** The broker can infer no-PTY mode from the absence of a ConPTY-related HANDLE set — all three `--inherit-handle` values are anonymous pipe ends, not ConPTY pipe handles. However, an explicit `--no-pty` flag to the broker is cleaner and makes the broker's behavior deterministic regardless of handle shapes. Recommendation: add `--no-pty` flag. When present, the broker skips the `AllocConsole` probe in favor of inheriting the passed-in pipe handles as stdio.

Actually — the broker's `AllocConsole` probe is non-fatal and independent of stdio. The distinction is: in PTY mode the broker inherits ConPTY pipe handles, and the child writes through `\Device\ConDrv` (the ConPTY). In no-PTY mode the broker inherits plain pipe handles, and the child writes to the pipe directly. The broker's `CreateProcessAsUserW` call is identical in both cases (no PSEUDOCONSOLE attribute either way). The only behavioral difference is WHAT handles are in the HANDLE_LIST. An explicit `--no-pty` flag gives the broker a clean signal to pass the inherited stdio handles to the child in `STARTF_USESTDHANDLES` mode (binding `startup_info.hStdInput/Output/Error` to the passed handles instead of inheriting the console). This is the correct approach.

**Broker changes needed:** Add `--no-pty` flag to `BrokerArgs` and `parse_args`. When `--no-pty` is set and three pipe handles are provided, set `startup_info.dwFlags = STARTF_USESTDHANDLES` with those handles. When `--no-pty` is absent (PTY path), use the existing zero-initialized `StartupInfo` (console inheritance).

```rust
// Pseudocode for the new BrokerLaunchNoPty arm in spawn_windows_child
// (after h_token selection returns null for BrokerLaunchNoPty, same as BrokerLaunch):
WindowsTokenArm::BrokerLaunchNoPty => {
    let pipes = DetachedStdioPipes::create()?;
    let inherit_handles: [HANDLE; 3] = [pipes.stdin_read, pipes.stdout_write, pipes.stderr_write];
    // Build PROC_THREAD_ATTRIBUTE_HANDLE_LIST with three handles (same pattern as BrokerLaunch)
    // Build broker command line with --no-pty flag + three --inherit-handle values
    // CreateProcessW(broker, CREATE_SUSPENDED, bInheritHandles=1, HANDLE_LIST)
    // AssignProcessToJobObject + apply_resource_limits + ResumeThread
    // pipes.close_child_ends() after CreateProcessW success
    // Return (child, Some(pipes))  ← so supervisor relay can forward stdout/stderr
}
```

### Pattern 4: Real-spawn write-deny integration test (D-07)

**What:** Spawn the broker with a Low-IL child (`cmd.exe` or a tiny helper) that attempts to write to a Medium-IL-labeled temp file. Assert `ERROR_ACCESS_DENIED` (kernel MIC pre-DACL check).

**Test precedent:** `broker_launch_assigns_child_to_job_object` in `broker_dispatch_tests` module (launch.rs:2423). Follow the same shape: CREATE_SUSPENDED → AssignProcessToJobObject → ResumeThread → wait → assert exit code.

**Test fixture setup:**
1. Create a temp file in `%USERPROFILE%` or `%TEMP%` (NOT a drive-root path — WRITE_OWNER limitation).
2. Call `nono::try_set_mandatory_label(&fixture_path, SYSTEM_MANDATORY_LABEL_MEDIUM_RID_MASK)` to set Medium-IL label (or no label = Medium IL by default; verify with `low_integrity_label_and_mask`).
3. Actually: Medium-IL is the default for user files. Low-IL is subtractive. A file in `%USERPROFILE%` has no mandatory-label ACE by default (Medium-IL default). The Low-IL child will be denied write by MIC. The test only needs to create the file without setting any label — the default Medium-IL gives the MIC check the correct enforcement.
4. Spawn broker with `--no-pty` and `--shell cmd.exe --shell-arg /c --shell-arg "echo test > <fixture_path>"` (or equivalent write attempt).
5. Wait for child exit. Assert exit code is non-zero (cmd echoes "Access is denied.") OR read the fixture file and assert it was not modified.

**cfg-gate:** `#[cfg(all(test, target_os = "windows"))]` — same as `low_integrity_primary_token_tests` and `broker_dispatch_tests`.

**Hard-fail:** No `#[ignore]`. Fail loud if broker artifact missing or label setup fails (D-08 contract). Mirror the Phase 41 D-12 BROKER-CR-04 anti-silent-skip policy.

**WRITE_OWNER limitation reminder:** The fixture MUST be in `%USERPROFILE%` or `%TEMP%`, NOT a drive-root directory like `C:\poc\*` (memory `feedback_windows_mandatory_label_write_owner`).

### Anti-Patterns to Avoid

- **Blanket WriteRestricted removal:** D-02 explicitly requires WriteRestricted to remain reachable when `prefers_low_il_broker=false`. Never remove the `WriteRestricted` branch from the cascade.
- **Broker `--no-pty` without STARTF_USESTDHANDLES:** If the broker inherits pipe handles but doesn't set `STARTF_USESTDHANDLES`, the child won't write to the pipes and the supervisor relay will never see output. The broker MUST set `hStdInput/Output/Error` to the passed handles.
- **Double-closing broker's pipe handles:** Follow the same discipline as `DetachedStdioPipes::close_child_ends()` — after `CreateProcessW` in the supervisor, close the child-end copies so the supervisor's read end sees EOF when the child exits.
- **Forgetting PROC_THREAD_ATTRIBUTE_HANDLE_LIST for the three pipe handles:** The no-PTY broker path uses `bInheritHandles=1`, which would inherit ALL inheritable handles from nono.exe unless HANDLE_LIST gates it. Gate it to exactly the three pipe handles (same security rationale as BrokerLaunch gating on the ConPTY pipe handles).
- **Test fixture on drive root:** `C:\poc\tempfile.txt` will fail `try_set_mandatory_label` with ERROR_ACCESS_DENIED due to the WRITE_OWNER limitation. Use `std::env::var("USERPROFILE")` or `std::env::temp_dir()`.
- **Profile field in ProfileDeserialize but not in Profile:** The `ProfileDeserialize → Profile::from()` conversion is explicitly exhaustive (line 2144-2173); rustc's struct-literal completeness check catches missing fields. Add the field in both structs.
- **Forgetting `#[serde(deny_unknown_fields)]` on ProfileDeserialize:** The struct at line 2091 has this attribute — any new field MUST be added to both `Profile` and `ProfileDeserialize`, otherwise deserialization will reject policy.json with an "unknown field" error.

---

## Common Pitfalls

### Pitfall 1: `select_windows_token_arm` signature change breaks existing call sites

**What goes wrong:** Adding a new parameter to `select_windows_token_arm` (currently at line 1106) breaks the call site at `spawn_windows_child` line 1180 and all unit tests in `pty_token_gate_tests` (lines 1886-1977).

**Why it happens:** The function has 4 parameters; adding a 5th changes the call signature in 8 places (1 call site + 7 unit tests).

**How to avoid:** Add the parameter with default `false` where appropriate in test calls. The 7 existing `pty_token_gate_tests` tests must all pass `false` as the new last argument to preserve existing behavior assertions.

**Warning signs:** `error[E0061]: this function takes 5 arguments but 4 were supplied` across 7 tests.

### Pitfall 2: Cascade ordering violation

**What goes wrong:** Inserting the new `BrokerLaunchNoPty` branch AFTER `has_session_sid → WriteRestricted` instead of BEFORE it. WriteRestricted would then shadow the new branch and `prefers_low_il_broker=true` would silently be ignored.

**Why it happens:** The cascade comment at lines 1062-1071 describes ordering as load-bearing; a naive insertion at the wrong position.

**How to avoid:** Insert the new branch as `else if prefers_low_il_broker && has_session_sid` between the `has_pty` branch and the `has_session_sid` (WriteRestricted) branch. The existing `pty_none_with_session_sid_selects_write_restricted` test (line 1934) must still pass (with `prefers_low_il_broker=false`). A new test must assert `pty_none_session_sid_with_broker_opt_in_selects_broker_launch_no_pty`.

### Pitfall 3: Broker pipe-handle inheritance without HANDLE_LIST

**What goes wrong:** Passing `bInheritHandles=1` to `CreateProcessW` (broker spawn) without setting `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` causes ALL inheritable handles from nono.exe to be inherited by the broker — including capability-pipe handles, potentially leaking sandbox mediation channel to the broker.

**Why it happens:** Phase 17's `DetachedStdioPipes` creates pipe pairs with all ends inheritable initially, then explicitly flips parent ends non-inheritable. But nono.exe may have other inheritable handles.

**How to avoid:** Mirror the `BrokerLaunch` arm's HANDLE_LIST construction (lines 1340-1397) exactly, substituting the three pipe handles for the two ConPTY handles. CRITICAL: flip the child-end handles to inheritable BEFORE the HANDLE_LIST call (they start non-inheritable per `DetachedStdioPipes::create()`).

**Warning signs:** Capability pipe server unexpectedly receiving connections from broker instead of sandboxed child; or broker hanging waiting for a handle it shouldn't have.

### Pitfall 4: Forgetting to flip pipe child-ends to inheritable before HANDLE_LIST

**What goes wrong:** `DetachedStdioPipes::create()` (line 72) creates pipe pairs with all ends inheritable (`bInheritHandle: 1` in SECURITY_ATTRIBUTES), then immediately flips the PARENT ends to non-inheritable via `SetHandleInformation(HANDLE_FLAG_INHERIT, 0)`. The child ends START inheritable. However, nono.exe may have separately flipped them OR the create function may change. Always explicitly set the three child-end handles to inheritable before building the HANDLE_LIST.

**How to avoid:** Before constructing the attribute list, call `SetHandleInformation(stdin_read, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT)` etc. for all three child ends. Mirror the `BrokerLaunch` arm's `SetHandleInformation` flip pattern (lines 1312-1337) which does the same for ConPTY handles.

### Pitfall 5: Profile `deny_unknown_fields` rejection

**What goes wrong:** Adding `"windows_low_il_broker": true` to `policy.json`'s `claude-code` profile before adding the field to `ProfileDeserialize`. At startup, the profile deserializer (which has `#[serde(deny_unknown_fields)]` at line 2091) rejects the entire profile with an "unknown field" error.

**Why it happens:** The strict deny_unknown_fields attribute causes any unrecognized field to fail deserialization.

**How to avoid:** Add the field to `ProfileDeserialize` struct (line ~2132) and `Profile` struct (line ~2072) AND add it to `nono-profile.schema.json` in the same commit/task as the `policy.json` change.

### Pitfall 6: Cross-target clippy failure in profile/mod.rs

**What goes wrong:** `profile/mod.rs` is compiled on ALL platforms. If the new `windows_low_il_broker: bool` field causes any new lint (e.g., unused field warning on Linux/macOS targets where nothing reads it), the cross-target clippy gate fails.

**Why it happens:** The field is only consumed by Windows-side code in `sandbox_prepare.rs` under `#[cfg(windows)]` (implicitly). On non-Windows targets, nothing reads `windows_low_il_broker`.

**How to avoid:** The precedent field `unsafe_macos_seatbelt_rules` (a Vec<String>) is in profile/mod.rs and compiles cleanly cross-platform — because it's read in `sandbox_prepare.rs:408` under a non-cfg-gated code path. The new bool field should follow the same pattern: it's fine for it to be defined in the struct and never used on non-Windows targets (no dead_code lint for struct fields by default in Rust). Verify with `cargo clippy --target x86_64-unknown-linux-gnu` before closing REQ-WSRH-05.

### Pitfall 7: Broker receives pipes but writes to wrong fd

**What goes wrong:** The Low-IL child spawned by the broker writes to the inherited console (if `AllocConsole` succeeded) instead of the pipe handles, so the supervisor relay never sees output.

**Why it happens:** If the broker sets `STARTF_USESTDHANDLES` but doesn't assign the pipe handles to the right `hStd*` fields in the child's startup info, the child falls back to the console.

**How to avoid:** In the broker's `run()` function, when `--no-pty` is present, set `startup_info.dwFlags = STARTF_USESTDHANDLES` and assign `hStdInput = inherit_handles[0]`, `hStdOutput = inherit_handles[1]`, `hStdError = inherit_handles[2]`. Pass the SAME three handles in `HANDLE_LIST`. Don't set `STARTF_USESTDHANDLES` on the PTY path (existing behavior).

---

## Code Examples

### Verified existing patterns reused by this phase

#### Anonymous-pipe create (Phase 17 pattern)
```rust
// Source: crates/nono-cli/src/exec_strategy_windows/launch.rs:71-121
// DetachedStdioPipes::create() — creates 3 inheritable pipe pairs, then
// flips parent ends non-inheritable. Returns all 6 handles.
let pipes = DetachedStdioPipes::create()?;
// After spawn:
unsafe { pipes.close_child_ends(); }
// pipes.stdout_read / stderr_read are the supervisor's relay read ends.
```

#### PROC_THREAD_ATTRIBUTE_HANDLE_LIST construction pattern
```rust
// Source: crates/nono-cli/src/exec_strategy_windows/launch.rs:1340-1397
// Pattern: probe size, allocate, Initialize, Update, CreateProcess, Delete.
// CRITICAL: handles_array must outlive the Update + CreateProcess calls.
let mut attr_size: usize = 0;
InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attr_size);
let mut attr_buf = vec![0u8; attr_size];
let attr_list = attr_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
InitializeProcThreadAttributeList(attr_list, 1, 0, &mut attr_size);
UpdateProcThreadAttribute(attr_list, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    handles_array.as_ptr() as *mut _, size_of_val(&handles_array[..]), ...);
// ... CreateProcessW ...
DeleteProcThreadAttributeList(attr_list);
```

#### Broker command-line construction
```rust
// Source: crates/nono-cli/src/exec_strategy_windows/launch.rs:1400-1418
// build_broker_command_line handles quoting; extend with --no-pty for no-PTY path:
let mut broker_args: Vec<OsString> = Vec::new();
broker_args.push("--shell".into());
broker_args.push(launch_program.as_os_str().to_owned());
broker_args.push("--no-pty".into());                         // NEW for BrokerLaunchNoPty
broker_args.push("--inherit-handle".into());
broker_args.push(format!("0x{:016x}", stdin_read as usize).into());
broker_args.push("--inherit-handle".into());
broker_args.push(format!("0x{:016x}", stdout_write as usize).into());
broker_args.push("--inherit-handle".into());
broker_args.push(format!("0x{:016x}", stderr_write as usize).into());
broker_args.push("--cwd".into());
broker_args.push(current_dir.as_os_str().to_owned());
```

#### Broker STARTF_USESTDHANDLES binding (no-PTY mode)
```rust
// In nono-shell-broker/src/main.rs run() function, new no-pty branch:
// When args.no_pty is true and inherit_handles has exactly 3 entries:
startup_info_ex.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
startup_info_ex.StartupInfo.hStdInput  = args.inherit_handles[0];
startup_info_ex.StartupInfo.hStdOutput = args.inherit_handles[1];
startup_info_ex.StartupInfo.hStdError  = args.inherit_handles[2];
```

#### select_windows_token_arm call site (new signature)
```rust
// Source: crates/nono-cli/src/exec_strategy_windows/launch.rs:1180-1185
// Updated call in spawn_windows_child:
let arm = select_windows_token_arm(
    is_windows_detached_launch,
    pty.is_some(),
    config.session_sid.is_some(),
    should_use_low_integrity_windows_launch(config.caps),
    config.prefers_low_il_broker,    // NEW
);
```

#### D-07 write-deny test structure (referencing broker_dispatch_tests pattern)
```rust
// Source: crates/nono-cli/src/exec_strategy_windows/launch.rs:2346-2608
// (broker_launch_assigns_child_to_job_object shape — follow this pattern)
// New test: spawn broker with --no-pty, child writes to a %USERPROFILE% file,
// assert child exit code is non-zero (write denied).
#[cfg(all(test, target_os = "windows"))]
mod write_deny_low_il_broker_no_pty_tests {
    // Steps:
    // 1. Create temp file in USERPROFILE (no label set = Medium IL by default)
    // 2. Resolve broker artifact path (same two-candidate logic as broker_dispatch_tests)
    // 3. Build broker command line with --no-pty + 3 pipe handles + cmd /c "echo x > <tmpfile>"
    // 4. CreateProcessW(broker, CREATE_SUSPENDED)
    // 5. AssignProcessToJobObject (same as Phase 31 test — containment correctness)
    // 6. ResumeThread
    // 7. WaitForSingleObject(INFINITE)
    // 8. GetExitCodeProcess → assert non-zero (cmd.exe exits 1 on "Access is denied.")
    //    OR read tmpfile → assert not modified (robust cross-encoding assertion)
    // 9. Cleanup: temp file
}
```

---

## Runtime State Inventory

This is NOT a rename/refactor/migration phase — it is a new feature phase. No runtime state inventory required.

---

## Open Questions

1. **Exact broker `--no-pty` parsing: is the broker's reject-empty-handle-list (BROKER-CR-03) compatible with the new 3-handle no-PTY shape?**
   - What we know: `parse_args()` in `main.rs:132-137` rejects empty `inherit_handles`. In no-PTY mode we pass 3 handles — non-empty. BROKER-CR-03 is satisfied.
   - What's unclear: The null/INVALID_HANDLE_VALUE rejection (BROKER-CR-02, lines 103-107) will apply to each of the 3 pipe handles. If `DetachedStdioPipes::create()` can return a null handle on failure, the broker would reject it. But `create_one_pipe` returns `Err(...)` on failure, so the caller never gets a null handle.
   - Recommendation: No change needed to BROKER-CR-02/03; they're already compatible with the no-PTY shape.

2. **Whether `profile/mod.rs`'s `merge_profiles` function needs updating for the new bool field**
   - What we know: `merge_profiles` at line ~3028 explicitly lists every field; the bool should use last-writer-wins (OR logic: either base or child sets it). The simplest merge is `base.windows_low_il_broker || child.windows_low_il_broker`.
   - Recommendation: Use `base || child` for the merge; add to the exhaustive `merge_profiles` field list to trigger a rustc struct-literal completeness warning if a field is ever forgotten.

3. **Whether the supervisor relay loop handles `Some(detached_stdio_pipes)` on the supervised (non-detached) path**
   - What we know: `execute_supervised` at line 800 receives `(child, detached_stdio)` from `spawn_windows_child`. The current code handles `detached_stdio.is_some()` only for the detached path (`is_windows_detached_launch`). For the new `BrokerLaunchNoPty` path, `detached_stdio` will be `Some(...)` on the NON-detached supervised path.
   - What's unclear: Does `execute_supervised`'s relay logic correctly handle `Some(pipes)` on the supervised path, or is it gated on `is_windows_detached_launch`?
   - Recommendation: Planner should read `execute_supervised` lines 800+ to verify the relay machinery applies to the pipes regardless of detach state. If gated on `is_windows_detached_launch`, the gate must be relaxed to include `BrokerLaunchNoPty`.

4. **Authenticode check for no-PTY path**
   - What we know: The `BrokerLaunch` arm verifies broker Authenticode (`verify_broker_authenticode`, lines 1289-1297) on production builds. The `BrokerLaunchNoPty` arm must do the same (same security invariant).
   - Recommendation: Copy the same broker-path resolution + `is_dev_build_layout` + `verify_broker_authenticode` block into the `BrokerLaunchNoPty` arm. Or extract a helper `resolve_and_verify_broker()` shared by both arms.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust Windows target (`x86_64-pc-windows-msvc`) | Build + tests | Implied (current platform is win32) | 1.77+ | — |
| `nono-shell-broker.exe` artifact | D-07 write-deny test | Must be pre-built | — | Test panics loudly (D-08 no-silent-skip) |
| Cross-target Linux toolchain (`x86_64-unknown-linux-gnu`) | REQ-WSRH-05 clippy | Unknown — check `rustup target list --installed` | — | Mark REQ PARTIAL, defer to live CI per cross-target-verify-checklist.md |
| Cross-target macOS toolchain (`x86_64-apple-darwin`) | REQ-WSRH-05 clippy | Unknown | — | Same as Linux fallback |

**Missing dependencies with no fallback:**
- `nono-shell-broker.exe` must be pre-built before running the D-07 write-deny integration test. Wave 0 must include a build step.

**Missing dependencies with fallback:**
- Cross-toolchains: if unavailable, mark REQ-WSRH-05 as PARTIAL per `cross-target-verify-checklist.md`, with live CI as the decisive signal.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) |
| Config file | None (Cargo.toml `[dev-dependencies]`) |
| Quick run command | `cargo test -p nono-cli --target x86_64-pc-windows-msvc pty_token_gate_tests` |
| Full suite command | `cargo test -p nono-cli --target x86_64-pc-windows-msvc && cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-WSRH-01 | Broker accepts `--no-pty` and launches child with pipe stdio | Integration (real-spawn) | `cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc` (new `parse_args_no_pty` + `run_no_pty_launch` tests in broker) | No — Wave 0 gap |
| REQ-WSRH-02 | `select_windows_token_arm(prefers_low_il_broker=true)` returns `BrokerLaunchNoPty` | Unit | `cargo test -p nono-cli pty_token_gate_tests::pty_none_session_sid_with_broker_opt_in_selects_broker_launch_no_pty` | No — Wave 0 gap |
| REQ-WSRH-02 | `select_windows_token_arm(prefers_low_il_broker=false)` still returns `WriteRestricted` | Unit (existing, must still pass) | `cargo test -p nono-cli pty_token_gate_tests::pty_none_with_session_sid_selects_write_restricted` | Yes (launch.rs:1934) |
| REQ-WSRH-03 | Low-IL child write to Medium-IL-labeled path is kernel-denied | Integration (real-spawn) | `cargo test -p nono-cli --target x86_64-pc-windows-msvc write_deny_low_il_broker_no_pty_tests` | No — Wave 0 gap |
| REQ-WSRH-05 | No regression: PTY path still selects `BrokerLaunch` | Unit (existing, must still pass) | `cargo test -p nono-cli pty_token_gate_tests::pty_some_no_detach_selects_broker_launch` | Yes (launch.rs:1896) |
| REQ-WSRH-05 | All existing broker_dispatch_tests pass | Integration (existing) | `cargo test -p nono-cli broker_dispatch_tests` | Yes (launch.rs:2346) |
| REQ-WSRH-05 | Cross-target Linux clippy clean | Clippy | `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | N/A (toolchain check) |
| REQ-WSRH-05 | Cross-target macOS clippy clean | Clippy | `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | N/A (toolchain check) |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli pty_token_gate_tests` (fast pure-logic check)
- **Per wave merge:** `cargo test -p nono-cli --target x86_64-pc-windows-msvc && cargo test -p nono-shell-broker --target x86_64-pc-windows-msvc`
- **Phase gate:** Full suite green + cross-target clippy (or PARTIAL with CI deferral) before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] New unit test `pty_none_session_sid_with_broker_opt_in_selects_broker_launch_no_pty` in `pty_token_gate_tests` module (launch.rs) — covers REQ-WSRH-02
- [ ] New integration test module `write_deny_low_il_broker_no_pty_tests` in launch.rs — covers REQ-WSRH-03
- [ ] New unit tests `parse_args_no_pty_flag_accepted` and `run_no_pty_pipes_bound` in nono-shell-broker/src/main.rs — covers REQ-WSRH-01 broker side
- [ ] Pre-build step for `nono-shell-broker.exe` artifact required before D-07 test can run

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | — |
| V3 Session Management | No | — |
| V4 Access Control | Yes | OS MIC kernel check (pre-DACL); `NO_WRITE_UP` mandatory label; Low-IL primary token via `nono::create_low_integrity_primary_token` |
| V5 Input Validation | Yes | Broker argv: `--no-pty` flag + 3 hex handles validated by `parse_args` (null/INVALID_HANDLE_VALUE rejection BROKER-CR-02; empty-list rejection BROKER-CR-03) |
| V6 Cryptography | No | — |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Restricting-SID write-gate blocking DllMain | Tampering (process integrity failure) | Replace `WriteRestricted` with broker Low-IL token for heavy-runtime children |
| Handle inheritance leaking supervisor pipe to broker | Elevation of Privilege | `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` whitelisting exactly the 3 pipe handles |
| Broker receiving null handle via `--inherit-handle` | Tampering | BROKER-CR-02: null and INVALID_HANDLE_VALUE rejected in `parse_args` |
| Drive-root label-apply failure (WRITE_OWNER) | Tampering (false-PASS test) | Test fixture MUST be in `%USERPROFILE%`/`%TEMP%` per D-08 |
| Silent downgrade from BrokerLaunchNoPty to Null on error | Elevation of Privilege | Fail-closed: any broker/token/label failure on no-PTY path produces `NonoError` + diagnostic, never falls back to Null or WriteRestricted |
| Broker binary substitution attack | Tampering | `verify_broker_authenticode` (same check as `BrokerLaunch` arm) must apply to `BrokerLaunchNoPty` arm too |

---

## Project Constraints (from CLAUDE.md)

The following CLAUDE.md directives are directly relevant to Phase 51:

| Directive | Phase 51 Impact |
|-----------|-----------------|
| No `.unwrap()` / `.expect()` in non-test code; `clippy::unwrap_used` enforced | All new code in `launch.rs`, `mod.rs`, `profile/mod.rs`, and `main.rs` must use `?` or explicit match. Tests may use `#[allow(clippy::unwrap_used)]`. |
| Unsafe code must have `// SAFETY:` doc comments | Every `unsafe {}` block in the new broker no-PTY wiring (CreateProcessW, PROC_THREAD_ATTRIBUTE_*, SetHandleInformation) must have a SAFETY comment. |
| Cross-target clippy MUST/NEVER: Windows-host `cargo check` is NOT a substitute | `profile/mod.rs` changes MUST pass `cargo clippy --target x86_64-unknown-linux-gnu` and `--target x86_64-apple-darwin`, or be marked PARTIAL per `cross-target-verify-checklist.md`. |
| DCO sign-off on all commits | Every commit must include `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>`. |
| Library is policy-free | New profile field (`windows_low_il_broker`) belongs in `nono-cli`, NOT in the `nono` library. The library's `create_low_integrity_primary_token` is already there and needs no changes. |
| No `#[allow(dead_code)]` | The new `windows_low_il_broker` field on `Profile` is used in `sandbox_prepare.rs` (wired to `ExecConfig.prefers_low_il_broker`) — not dead. All new code paths must be exercised by tests. |
| Tests that modify env vars must save/restore | The D-07 write-deny test uses `std::env::var("USERPROFILE")` / `std::env::temp_dir()` — read-only, no save/restore needed. But any test that temporarily sets env vars must use `EnvVarGuard`. |
| Fail-secure | On any broker/token failure on the no-PTY path: return `Err(NonoError::...)`, never silently degrade to WriteRestricted or Null. |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The supervisor's relay machinery in `execute_supervised` handles `Some(detached_stdio_pipes)` on the non-detached supervised path (i.e., the relay is not gated on `is_windows_detached_launch`) | Open Questions #3, Arch Diagram | If wrong, the supervisor relay won't forward child output; the planner must add a task to unlatch the gate condition |
| A2 | Files in `%USERPROFILE%` / `%TEMP%` have no pre-existing mandatory-label ACE (Medium-IL is the OS default for user-owned files), so a Low-IL child's write is denied by MIC without needing to call `try_set_mandatory_label` on the fixture | Code Examples (D-07 test) | If wrong (if OS applies explicit Low-IL label to TEMP), the write would succeed and the test would give a false PASS — verify with `low_integrity_label_and_mask` in test setup |
| A3 | The broker's `AllocConsole` probe + the child's `STARTF_USESTDHANDLES`-bound pipes are compatible without the broker needing special handling to suppress `AllocConsole` in no-PTY mode | Architecture Patterns, Pattern 3 | If wrong (AllocConsole interferes with pipe inheritance for the child), may need to suppress `AllocConsole` in no-PTY mode |

---

## Sources

### Primary (HIGH confidence)

- `crates/nono-shell-broker/src/main.rs` — full broker source, all decision points [VERIFIED: codebase read]
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` lines 1057-1977 — `WindowsTokenArm`, `select_windows_token_arm`, `spawn_windows_child`, all tests [VERIFIED: codebase read]
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` lines 1-171 — `DetachedStdioPipes` (Phase 17 anonymous-pipe pattern) [VERIFIED: codebase read]
- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` — `AppliedLabelsGuard` pattern [VERIFIED: codebase read]
- `crates/nono-cli/src/profile/mod.rs` lines 2060-2072, 2091-2174 — `unsafe_macos_seatbelt_rules` precedent for platform-gated profile field [VERIFIED: codebase read]
- `crates/nono-cli/data/nono-profile.schema.json` lines 95-103 — schema precedent [VERIFIED: codebase read]
- `crates/nono-cli/data/policy.json` lines 656-729 — `claude-code` profile current shape [VERIFIED: codebase read]
- `.planning/phases/31-broker-process-architecture-shell-01/31-05-SUMMARY.md` — Phase 31 broker production validation: Low-IL primary token + NO_WRITE_UP enforced [VERIFIED: doc read]
- `.planning/debug/claude-exe-dll-init-failed.md` — confirmed root cause + fix rationale [VERIFIED: doc read]
- `.planning/REQUIREMENTS.md` — REQ-WSRH-01/02/03/05 acceptance criteria [VERIFIED: doc read]
- `.planning/templates/cross-target-verify-checklist.md` — cross-target clippy scope rules [VERIFIED: doc read]

### Metadata

**Confidence breakdown:**
- Standard stack (no new deps): HIGH — all primitives confirmed in codebase
- Architecture (cascade extension, broker wiring): HIGH — exact code locations verified
- Pitfalls: HIGH — drawn from documented Phase 15/30/31 regressions + memory entries
- Test shape: HIGH — follows Phase 31 `broker_dispatch_tests` precedent exactly

**Research date:** 2026-05-26
**Valid until:** 2026-06-26 (stable codebase; 30-day window)
