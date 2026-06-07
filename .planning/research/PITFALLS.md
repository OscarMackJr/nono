# Pitfalls Research

**Domain:** Security-critical Rust sandbox — v2.10 new feature surfaces (kernel minifilter spike, EDR HUMAN-UAT, macOS Seatbelt upstream parity)
**Researched:** 2026-06-06
**Confidence:** HIGH (codebase-grounded; cross-referenced against project retrospectives v2.7–v2.9 and WDK/MSDN authoritative docs)

This file is scoped to v2.10 only. It does not repeat the v2.0–v2.9 pitfalls (ConPTY, WFP weight ordering, ETW admin check, etc.) that remain in the archived v2.0 PITFALLS.md. Generic Rust advice is omitted.

---

## Critical Pitfalls

### Pitfall 1: IRQL Violation in Pre-Create Callback — BSOD

**What goes wrong:**
A pre-create callback acquires a lock, calls a memory allocator tagged `PagedPool`, or calls `FltSendMessage` while the IRQL is above `APC_LEVEL`. The kernel BSODs immediately with `IRQL_NOT_LESS_OR_EQUAL` or `KERNEL_AUTO_BOOST_LOCK_ACQUISITION_WITH_RAISED_IRQL`. On a spike host this means a hard reboot, 3–10 minutes of lost iteration time, and a potential Windows Startup Repair loop if the driver registered as a boot-start filter.

**Why it happens:**
Pre-create callbacks are called at `IRQL = PASSIVE_LEVEL` for most creates, but the minifilter stack can call them at `APC_LEVEL` in certain fast I/O or paging paths. Spike code written at a desk (always `PASSIVE_LEVEL`) silently accumulates assumptions that break on the first real paging operation. The FltMgr docs note that `FltSendMessage` itself blocks until a user-mode reader calls `FilterGetMessage`; if no reader is running or the reader is on the same thread that triggered the I/O, the kernel thread deadlocks at `APC_LEVEL` and the watchdog fires a BSOD.

**How to avoid:**
- Use `NonPagedPoolNx` for any allocations reachable from callback context; never `PagedPool`.
- Never hold a resource (`ERESOURCE`, `FastMutex`) across a `FltSendMessage` call.
- In the spike, set `FltSendMessage` with a finite `Timeout` (e.g., 500 ms) so a missing user-mode reader returns `STATUS_TIMEOUT`, not an infinite wait.
- Before calling any Ke/Ex/IO routine in the callback, assert the IRQL: `NT_ASSERT(KeGetCurrentIrql() <= APC_LEVEL)`.
- For the spike's pre-create callback, do the minimum work at callback time (record filename + PID into a pre-allocated ring buffer) and defer all user-mode communication to a worker thread running at `PASSIVE_LEVEL`.

**Warning signs:**
BSOD minidump `!analyze -v` reports `IRQL_NOT_LESS_OR_EQUAL` with a stack trace terminating inside the minifilter's callback. First occurrence is typically on a file open from the paging subsystem (`\pagefile.sys` or during a DLL map).

**Phase to address:**
Phase 63 (kernel minifilter spike). The ring-buffer design and worker-thread IPC pattern must be defined in the spike's plan before a single line of driver code is written.

---

### Pitfall 2: Filtering Your Own I/O — Infinite Recursion BSOD

**What goes wrong:**
The minifilter's pre-create callback fires on a `FltCreateFile` call the minifilter itself issued (e.g., to open a log file, read a trust database, or communicate with a user-mode component via a file-backed channel). The callback fires again, issues the same `FltCreateFile`, and the stack overflows — BSOD `DRIVER_OVERRAN_STACK_BUFFER` or `KERNEL_STACK_OVERFLOW`.

**Why it happens:**
Unlike legacy filter drivers, FltMgr's generated I/O model is non-recursive by default **only** when the minifilter uses its own instance via `FltCreateFile` with an instance below itself. If the minifilter issues I/O at the same altitude level, or uses the native `ZwCreateFile`, the request re-enters the filter stack from the top and the callback fires again. The spike is particularly vulnerable because the typical "log to a file" pattern uses `ZwCreateFile`.

**How to avoid:**
- All driver-originated I/O must use `FltCreateFile` with the minifilter's own instance, NOT `ZwCreateFile` or `NtCreateFile`.
- In the pre-create callback, check `FltGetRequestorProcess` / `IoGetRequestorProcess` and skip processing if the requestor is the driver's own work-item context. Store a "driver PID" marker at `DriverEntry` time.
- For the spike, avoid all driver-originated file I/O. Use `FltSendMessage` to push data to user mode; let user mode write logs. This eliminates the entire recursion surface.

**Warning signs:**
BSOD with a stack trace showing the same callback symbol repeated 100+ times. The first occurrence is always on a path the driver opens internally.

**Phase to address:**
Phase 63 design doc must call out "no driver-originated file I/O; all logging/communication via FltSendMessage to user-mode service."

---

### Pitfall 3: Blocking the User Thread Indefinitely in Pre-Create — System Hang

**What goes wrong:**
`FltSendMessage` is called inside the pre-create callback with `Timeout = NULL` (wait forever), and the user-mode component (nono supervisor) is not running, is slow, or deadlocks. The calling thread blocks at kernel level, holding the original `IRP_MJ_CREATE` request open. Any other thread that transitively opens the same file (or any file on the same volume) may deadlock behind it. The system visually appears frozen; explorer.exe hangs.

**Why it happens:**
`FltSendMessage` with `NULL` timeout is the "simplest" code path shown in driver tutorials. For a test driver run in a controlled lab it "works." In real use (EDR UAT, macOS parity test environment, any CI runner) the supervisor process may not be alive at the time a file open occurs during boot.

**How to avoid:**
- Always pass a finite `Timeout` to `FltSendMessage`. For the spike, 200 ms is sufficient; treat `STATUS_TIMEOUT` as "permit the operation" (fail-open on communication loss is acceptable for a spike; document this clearly in the ADR).
- The production ADR must decide the fail-direction: fail-open (permit) vs. fail-closed (block). For a security spike the ADR should default to fail-open-on-timeout so the system remains bootable, with an explicit note that production must revisit.
- Register a `FilterUnloadCallback` and a `InstanceTeardownStartCallback` so the driver un-registers cleanly when the user-mode component exits, preventing queued messages from blocking.

**Warning signs:**
Mouse/keyboard become unresponsive 5–30 seconds after loading the driver. `!irp` in WinDbg shows a `IRP_MJ_CREATE` stuck in the minifilter's callback waiting on a `KEVENT`.

**Phase to address:**
Phase 63 spike plan. The timeout value and fail-direction must be explicit ADR decisions, not left as implementation details.

---

### Pitfall 4: TESTSIGNING + HVCI/Secure Boot Interaction — Driver Refuses to Load, No Error Message

**What goes wrong:**
`bcdedit /set testsigning on` is run, a self-signed driver is deployed, and `sc start` returns success — but the driver never actually loads and no BSOD or error appears. Alternatively, `bcdedit` itself refuses with "The value is protected by Secure Boot policy and cannot be modified or deleted."

**Why it happens:**
Three independent blockers can each produce silent-failure:
1. **Secure Boot is on:** `bcdedit /set testsigning on` is rejected. The developer disables Secure Boot in UEFI, but then BitLocker (if enabled) triggers a recovery key prompt on next boot — if the key is unavailable, the machine is unbootable.
2. **HVCI (Memory Integrity) is on:** On Windows 11 build 26200 (the project's target host), HVCI is enabled by default on most OEM machines. A test-signed driver built without HVCI-compatible practices (no executable kernel memory, no `MmMapIoSpaceEx` with `PAGE_EXECUTE`) is silently rejected by the hypervisor at load time even with `TESTSIGNING ON`.
3. **The test cert is not in the Trusted Root / Trusted Publishers store of the TARGET machine** (common when the cert was generated on a different machine).

**How to avoid:**
- Before the spike starts, document the host machine's configuration: `msinfo32.exe` → Device Security section. Check Secure Boot state AND HVCI/Memory Integrity state.
- If HVCI is on: the test cert must be added to the Trusted Root store on the target host, AND the driver must be built with HVCI-compatible constraints. The WDK has a "HVCI Compatibility" test; run it before attempting to load.
- Use a VM (Hyper-V with Secure Boot OFF) for the spike rather than the bare-metal dev host. This avoids the BitLocker/Secure Boot/HVCI triad entirely and enables kernel debugging over a virtual serial/pipe without needing a second physical machine.
- Document the exact `bcdedit` state (`/enum all`) as a reproducibility artifact in the spike's PLAN.md.

**Warning signs:**
`sc start nono-minifilter` returns `ERROR_SERVICE_ALREADY_RUNNING` or `ERROR_SUCCESS` but `fltmc instances` shows no instance. The Windows Event Log System channel will have an event from source `Microsoft-Windows-FilterManager` with Event ID 3 or 6 indicating the driver failed integrity checks.

**Phase to address:**
Phase 63 pre-spike environment setup task (plan step 1, before any driver code).

---

### Pitfall 5: Altitude Conflict With Installed EDR — Driver Fails to Register or EDR Goes Blind

**What goes wrong:**
Two scenarios from the same root cause — the spike uses an altitude value that collides with an installed EDR driver:
- **Scenario A:** The spike driver fails to start (`FltRegisterFilter` returns `STATUS_FLT_INSTANCE_ALTITUDE_COLLISION`) because an EDR driver already claimed that altitude.
- **Scenario B:** The spike uses an altitude in the `320000–329998` Anti-Virus range (the most common EDR range). The EDR's callback stack is disrupted; the EDR vendor flags the machine as compromised and quarantines files or blocks execution.

**Why it happens:**
Microsoft allocates altitude ranges per functional category. The Anti-Virus range (`320000–329998`) is where CrowdStrike, Defender, SentinelOne, etc. operate. Developers pick round numbers (e.g., `320000`) or borrow from a tutorial without checking what is already registered. `fltmc` shows all registered altitudes; developers skip this check.

**How to avoid:**
- Request an official altitude from Microsoft before deploying any driver, even for spikes. The request form is fast (~1 week) and free. Use the Activity Monitor range (`360000–389998`) or the FSFilter range (`400000–409998`) for a trust/monitor spike; these are below the AV range and minimize EDR conflict risk.
- For the spike's test environment, enumerate existing minifilters with `fltmc filters` and `fltmc instances` and pick an altitude that does not conflict.
- In the spike's ADR, document the chosen altitude and the rationale (category, why not AV range).
- The EDR UAT (Phase 64) MUST run AFTER the spike's altitude is finalized. Running UAT with a placeholder altitude risks triggering EDR false positives that contaminate the UAT results.

**Warning signs:**
`FltRegisterFilter` returns a non-`STATUS_SUCCESS` NTSTATUS at DriverEntry time. `fltmc filters` shows another driver at or adjacent to the chosen altitude. EDR console shows a "suspicious driver loaded" alert.

**Phase to address:**
Phase 63 (altitude selection in design doc). Phase 64 (EDR UAT must run with spike's final altitude, not a placeholder).

---

### Pitfall 6: Spike Scope Creep Into Production — "While We're Here" Additions

**What goes wrong:**
The spike starts as a test-signed POC to validate the pre-create trust interception design. Partway through, a contributor adds:
- Driver signing pipeline integration (CI-triggered `.cat` generation)
- A full user-mode ↔ kernel IPC protocol with versioning
- Installation via INF/MSI

Each addition is "small," but collectively they convert the spike into a production driver — without WHQL certification, without cross-version testing, without the security review a production driver requires.

**Why it happens:**
Kernel driver development is genuinely difficult; contributors naturally want to make progress on the production deliverable while the kernel environment is set up. The spike's success criteria ("prove pre-create trust interception is feasible") are fuzzy, making it easy to justify additions as "still proving feasibility."

**How to avoid:**
- Write the spike's success criteria as a binary go/no-go gate before any code: "Does a test-signed minifilter on Windows 11 build 26200 intercept `IRP_MJ_CREATE` for a specific executable path and send the path + PID to a user-mode process via `FltSendMessage` before the create completes? YES or NO."
- The spike's plan must include an explicit "out of scope" list: no CI signing pipeline, no INF/MSI, no versioned IPC protocol, no cross-version testing, no HVCI certification.
- The ADR produced by the spike must explicitly scope what "production" would require vs. what the spike proved.
- Treat the spike as throwaway code. If the spike's driver code gets committed to the main repo, it signals scope creep has already occurred.

**Warning signs:**
The spike's PR includes a `Cargo.toml` change, a new `scripts/build-driver.ps1`, or any CI workflow change. If any of these appear, the spike has crossed into production scope.

**Phase to address:**
Phase 63. The spike phase plan must have an explicit success-criteria section and an explicit out-of-scope list before the first task executes.

---

### Pitfall 7: EDR UAT Proving the Wrong Thing — Contaminated Results

**What goes wrong:**
The WR-02 EDR HUMAN-UAT executes on a machine where:
- The EDR was installed the same day (hasn't learned the baseline)
- nono's broker binary is freshly compiled and unsigned (unsigned = EDR-suspicious by definition)
- The test runs from a dev-layout path (`target\release\nono.exe`) not the production MSI install path

The UAT "passes" with no EDR alerts, but only because the EDR was not monitoring at the time, or "fails" with alerts that would not occur in normal deployment. Either result is not actionable.

**Why it happens:**
WR-02 has been deferred since v2.1. The instinct when finally running it is to "get it done quickly" — run nono, see if the EDR quarantines anything, report pass/fail. But UAT validity depends on the environment being representative.

**How to avoid:**
- The EDR UAT plan must specify: (a) which EDR product (at minimum Defender + one third-party), (b) that the EDR has been running on the machine for ≥24 hours before the test (baseline established), (c) that nono is installed via the production signed MSI (not dev-layout), (d) that the test exercises the token-manipulation paths (Low-IL broker, `CreateProcessAsUser`, mandatory label) specifically, because those are the paths most likely to generate alerts.
- Record the EDR version, policy mode (audit vs. block), and alert log state before and after each test case.
- "EDR did not quarantine" and "EDR did not alert" are different outcomes; the UAT must distinguish them (check the EDR console, not just whether the test binary ran).
- If the EDR generates a false-positive alert, document the exact alert + rule triggering it in the UAT artifact. This is valuable signal for the production ADR, not a test failure.

**Warning signs:**
UAT plan has a single pass/fail criterion ("nono run succeeded"). No EDR product or version specified. Test binary is from `target\release\`.

**Phase to address:**
Phase 64. The phase plan must define the environment preconditions and per-test-case criteria before the UAT is attempted.

---

### Pitfall 8: EDR Flagging Broker Token Manipulation as Malicious — Quarantine Loop

**What goes wrong:**
The EDR's behavioral engine flags `CreateProcessAsUserW` called by `nono-shell-broker.exe` (a non-system process creating processes under a different token) as `T1134.002 — Create Process with Token`. The EDR quarantines `nono-shell-broker.exe`. On next `nono run`, the broker is missing, nono falls back or errors, and the user's machine is partially broken.

**Why it happens:**
nono's broker legitimately uses token manipulation to enforce Low-IL mandatory labels. This is indistinguishable from a privilege-escalation attack at the API level: the sequence `OpenProcessToken` → `DuplicateTokenEx` → `SetTokenInformation(TokenIntegrityLevel)` → `CreateProcessAsUserW` is exactly MITRE ATT&CK T1134. Modern EDRs flag this pattern out of the box.

**How to avoid:**
- The production MSI installer must add `nono-shell-broker.exe` and `nono.exe` to the EDR's exclusion list via documented exclusion APIs (CrowdStrike, Defender, SentinelOne all have management APIs or Group Policy exclusion paths). Include this in the MSI's custom action or installation guide.
- The WR-02 UAT must specifically test the "EDR quarantines broker" failure mode: run the UAT once WITHOUT exclusions to characterize which EDR rules fire, then run again WITH exclusions to confirm the exclusions are sufficient.
- The spike ADR must include a "EDR exclusion requirements" section so production packaging knows what to exclude.
- Never test with an unsigned broker binary in the EDR environment — unsigned + token manipulation = highest-confidence malicious verdict.

**Warning signs:**
After `nono run`, `nono-shell-broker.exe` disappears from disk. The EDR console shows a "process isolation" or "quarantine" event for `nono-shell-broker.exe` within the first 30 seconds.

**Phase to address:**
Phase 64. Phase 63's spike ADR should note the exclusion requirement even though the spike itself runs in a test environment without an EDR.

---

### Pitfall 9: macOS Seatbelt Changes Silently Compile on Windows — Ship Broken Unix Code

**What goes wrong:**
A macOS-relevant upstream commit is cherry-picked (e.g., `$PWD` symlink-CWD capture, or platform-rules-after-user-write-allows ordering fix). The change touches `#[cfg(target_os = "macos")]` or `#[cfg(unix)]` blocks. Windows-host `cargo build` and `cargo clippy --workspace` pass because those blocks are not compiled. The change is committed, tagged, and pushed. The `release.yml` macOS build leg fails with a compile error. This is exactly what happened twice in v2.9 (`E0716` in `claude_code_hook.rs`, edition-2024 let-chain in `hook_runtime.rs`).

**Why it happens:**
The project's CLAUDE.md encodes this as a MUST rule, but the local cross-target build is blocked by the `ring`/`aws-lc-sys` C-toolchain — cross-compilation from Windows to macOS requires macOS SDK headers that cannot be legally redistributed. The only cross-compile signal is CI. When a session runs fast and the developer doesn't wait for CI before merging, the broken code reaches a release tag.

**How to avoid:**
- Any commit touching a file that contains `#[cfg(target_os = "macos")]`, `#[cfg(unix)]`, `#[cfg(not(windows))]`, or any file under `crates/nono/src/sandbox/macos.rs` or `crates/nono-cli/src/exec_strategy/` MUST NOT be tagged until the CI macOS build leg is green.
- The macOS parity phase plan must include a CI-green gate as a REQUIRED close condition (not PARTIAL, not deferred) because the entire value of the phase is macOS code being correct.
- Use `cargo check --target x86_64-apple-darwin` via the WSL cross-toolchain as a fast pre-commit signal if available. If not available (blocked by ring/aws-lc-sys), mark the gate as CI-deferred in the plan's CLOSE-GATE.md and DO NOT push a release tag until CI completes.
- After cherry-picking any upstream macOS commit, scan the commit for `let ... && let ...` chains (edition-2024 syntax) and `E0716` lifetime patterns (temporary dropped while borrowed). These are the two compile error classes that hit v2.9.

**Warning signs:**
A macOS-touching commit is merged to `main` and a `v*.*.*` release tag is pushed within the same session without a CI macOS green check. The `release.yml` run shows a red macOS build leg within ~10 minutes of the tag push.

**Phase to address:**
Phase 65 (macOS Seatbelt parity sync). The phase plan's CLOSE-GATE.md must list "macOS `release.yml` build leg green" as a required gate, not advisory.

---

### Pitfall 10: Seatbelt Rule Ordering — Deny After Allow Is Silently Ignored on macOS

**What goes wrong:**
An upstream commit reorders rules so that platform deny rules (e.g., `deny file-read* ~/.ssh`) appear AFTER broad user-written allow rules (e.g., `allow file-read* (subpath "/Users/alice")`). On macOS Seatbelt, the last matching rule wins. The allow rule fires, the deny is never evaluated, and `~/.ssh` is readable by the sandboxed process.

**Why it happens:**
The upstream commit being cherry-picked might fix the ordering in the upstream's code, but the fork's rule emission order (inside `policy.rs`'s `CapabilitySetExt` call chain) is different. A blind cherry-pick changes the upstream's ordering logic without verifying that the fork's rule emission produces the same final Seatbelt profile string order. The fork emits rules at a different call site than upstream.

**How to avoid:**
- After cherry-picking any commit that touches rule ordering, emit the actual Seatbelt profile string for a representative profile (e.g., `claude-code`) with `nono run --dry-run --profile claude-code` on a macOS host and manually verify that deny rules appear after the allow rules they are intended to override.
- Unit tests for Seatbelt profile generation must assert rule ordering, not just rule presence. The test fixture should assert the exact string position of the deny rule relative to the allow rule for at least one sensitive group (`ssh_config`, `git_credentials`).
- The CLAUDE.md "Strictly allow-list: cannot express deny-within-allow" note applies to Linux Landlock; macOS Seatbelt CAN deny within an allow, but only if the deny appears after the allow in the profile string. This asymmetry is a recurring source of confusion when porting upstream commits across platforms.

**Warning signs:**
`nono run --profile claude-code -- cat ~/.ssh/id_rsa` exits 0 and prints key content on a macOS host after the cherry-pick.

**Phase to address:**
Phase 65. The phase plan must include a "rule order validation" task that runs the dry-run profile emission and manually inspects sensitive group ordering on a macOS host.

---

### Pitfall 11: macOS `/private/etc` Symlink Path Drift in Upstream Cherry-Picks

**What goes wrong:**
An upstream commit adds a new deny group (e.g., `hosts_file`) that emits `(deny file-read* (literal "/etc/hosts"))`. On macOS, `/etc` is a symlink to `/private/etc`. The sandboxed process opens `/private/etc/hosts` (the canonical path), which is not matched by the literal `/etc/hosts` rule. The deny is silently skipped.

**Why it happens:**
The upstream codebase runs integration tests on Linux (where `/etc` is canonical) and may not test the symlink resolution on macOS. When the fork cherry-picks the commit, it inherits the same gap. The fork's existing code in `macos.rs` has partial symlink handling, but new deny paths from upstream cherry-picks bypass it if the deny emitter is in `policy.rs` (library side) rather than `sandbox/macos.rs`.

**How to avoid:**
- Any new path literal emitted in a deny rule for macOS must be validated against `std::fs::canonicalize()` on macOS. If the canonical path differs from the literal, both must be emitted: `(deny file-read* (literal "/etc/hosts") (literal "/private/etc/hosts"))`.
- Add a macOS-specific test in `crates/nono/src/sandbox/macos.rs` that asserts both the symlink path and the canonical path are covered for every new sensitive deny group.
- This applies specifically to: `/etc`, `/tmp` (→ `/private/tmp`), and any vendor-specific macOS paths added by upstream.

**Warning signs:**
`nono run --profile claude-code -- cat /etc/hosts` is blocked, but `nono run --profile claude-code -- cat /private/etc/hosts` succeeds on macOS.

**Phase to address:**
Phase 65. Add a symlink-coverage check to the phase's verification checklist.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Spike driver committed to main repo | Avoids managing a throwaway branch | Spike code becomes "production" without WHQL/HVCI audit; entropy accumulates | Never — keep spike in a scratch branch or separate repo |
| `FltSendMessage` with `NULL` timeout | Simplest code | System hang on supervisor exit; boot-blocking on crash | Never in any non-lab driver |
| Using `ZwCreateFile` instead of `FltCreateFile` in driver callbacks | Familiar Win32-like API | Recursion BSOD on first self-generated I/O | Never in minifilter callbacks |
| Cherry-picking macOS upstream commits without waiting for CI green | Faster development | Broken macOS build leg reaches release tags (happened twice in v2.9) | Never for release-tagged commits |
| Running EDR UAT with unsigned dev-layout binary | Faster setup | UAT results not representative; false negatives on real deployment | Never — UAT must use production-signed MSI binary |
| Seatbelt deny rules without ordering tests | Less test scaffolding | Silent allow-over-deny on macOS (`~/.ssh` readable) — security regression | Never for sensitive deny groups |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| FltMgr + user-mode supervisor | `FltSendMessage` inside pre-create callback with no timeout | Pass a finite `Timeout`; handle `STATUS_TIMEOUT` as permit-and-log |
| Driver + HVCI host | Assume `TESTSIGNING ON` is sufficient | Check HVCI state with `msinfo32`; use a VM or disable Memory Integrity for the spike |
| EDR + broker binary | Run unsigned binary during UAT | Install production-signed MSI, add EDR exclusions, then run UAT |
| Seatbelt + upstream cherry-pick | Trust that upstream tested macOS paths | Manually verify profile emission and rule order on a macOS host post-cherry-pick |
| macOS `/etc` symlinks | Emit only the literal path in deny rules | Emit both symlink and canonical (`/etc` + `/private/etc`) for every macOS deny path |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Spike driver with `fail-closed-on-timeout` (blocks all opens when supervisor not running) | System unbootable if supervisor crashes at boot | Spike must fail-open on timeout; production ADR decides fail-direction explicitly |
| EDR exclusion applied to entire nono install directory | Excludes future malicious binaries dropped into that directory | Exclude specific executables by path + hash or Authenticode signature, not directory |
| Seatbelt deny rule absent canonical macOS path | `~/.ssh`, `~/Library/Keychains` accessible via canonical path | Always emit both symlink and canonical form for macOS deny paths |
| macOS-only code change tagged before CI green | Broken macOS sandbox ships to users | Never push release tag without CI macOS build leg green |
| Spike altitude in AV range (320000–329998) | EDR disruption, false-positive quarantine of system files | Choose Activity Monitor range; request official altitude from Microsoft |

---

## "Looks Done But Isn't" Checklist

- [ ] **Driver loads on first try:** Verify with `fltmc instances` — `sc start` success does NOT mean the driver registered with FltMgr. Check Event ID 3/6 from `Microsoft-Windows-FilterManager`.
- [ ] **Pre-create callback fires:** Insert a `DbgPrint` in the callback and attach WinDbg or use DebugView — no output means the instance setup callback rejected the volume or the altitude was not registered correctly.
- [ ] **EDR UAT used production binary:** Confirm via file timestamp + authenticode signature on the binary under test — dev-layout binaries are never signed.
- [ ] **macOS Seatbelt deny rules cover canonical paths:** `cat /private/etc/hosts` must be blocked, not just `cat /etc/hosts`.
- [ ] **Seatbelt rule ordering validated:** `nono run --dry-run --profile claude-code` emits a profile where deny rules appear AFTER the allow rules they override.
- [ ] **CI macOS build leg green before release tag:** `gh run list --workflow release.yml` shows green for the macOS build leg, not just the Windows leg.
- [ ] **Spike ADR documents go/no-go verdict explicitly:** ADR must conclude with "proceed to production driver" or "defer — feasibility not proven"; "interesting results" is not a verdict.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| BSOD from IRQL violation | MEDIUM | Hard reboot; if boot-start filter, use WinRE → bcdedit to disable the driver service before boot; remove the allocation and replace with `NonPagedPoolNx` |
| Recursion BSOD from own I/O | MEDIUM | Same as above; remove `ZwCreateFile` call from callback; replace with FltCreateFile-based non-recursive path |
| TESTSIGNING blocked by Secure Boot | LOW | Disable Secure Boot in UEFI (have BitLocker recovery key ready); or switch to a VM |
| Driver refuses to load (HVCI conflict) | LOW–MEDIUM | Disable Memory Integrity in Windows Security settings; rebuild driver with HVCI-compatible settings and re-test |
| Altitude conflict with EDR | LOW | Change the altitude in the driver's registry key (`HKLM\SYSTEM\CurrentControlSet\Services\<driver>\Instances\<instance>\Altitude`) to a non-conflicting value and restart |
| EDR quarantines broker binary | MEDIUM | Restore from quarantine in EDR console; add binary-level exclusion; re-run UAT |
| macOS broken build from cfg-gated error | LOW | Fix the compile error, push a new tag (leapfrog version per fork release rule), wait for CI |
| Seatbelt deny-order regression | HIGH | Add canonical path variant and ordering assertion; validate on macOS host; security regression must be confirmed fixed by live test before release |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| IRQL violation BSOD (Pitfall 1) | Phase 63 — ring-buffer design in plan before code | `!analyze -v` minidump shows no IRQL_NOT_LESS_OR_EQUAL from nono code |
| Own-I/O recursion BSOD (Pitfall 2) | Phase 63 — no driver-originated file I/O rule in design doc | No BSOD during 10-minute soak with 100 concurrent file opens |
| Blocking user thread indefinitely (Pitfall 3) | Phase 63 — finite `Timeout` in plan spec | Supervisor exit while driver loaded: file opens complete within 250 ms |
| TESTSIGNING + HVCI/Secure Boot (Pitfall 4) | Phase 63 — environment setup task before driver code | `fltmc instances` shows driver registered; `!analyze` shows no load error |
| Altitude conflict with EDR (Pitfall 5) | Phase 63 design + Phase 64 UAT ordering | `fltmc filters` shows no altitude collision on the UAT machine |
| Spike scope creep (Pitfall 6) | Phase 63 plan — explicit success criteria and out-of-scope list | Spike PR contains no `Cargo.toml` changes and no CI workflow changes |
| EDR UAT proving wrong thing (Pitfall 7) | Phase 64 plan — environment preconditions section | UAT artifact includes EDR product + version + policy mode + alert log |
| EDR quarantining broker (Pitfall 8) | Phase 64 — "no exclusion" run first, then "with exclusion" run | Broker binary present on disk after both UAT runs; EDR alert log documented |
| macOS cfg-gated compile errors ship (Pitfall 9) | Phase 65 — CI macOS green required in CLOSE-GATE.md | `gh release list` shows a published release with macOS build assets |
| Seatbelt deny-after-allow ordering (Pitfall 10) | Phase 65 — rule order assertion in unit tests | `nono run --profile claude-code -- cat ~/.ssh/id_rsa` is blocked on macOS |
| macOS `/private/etc` symlink drift (Pitfall 11) | Phase 65 — canonical path coverage in cherry-pick checklist | Both `/etc/hosts` and `/private/etc/hosts` blocked on macOS host |

---

## Sources

- Microsoft WDK: [Writing Pre-operation Callback Routines](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/writing-preoperation-callback-routines)
- Microsoft WDK: [FltSendMessage function](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/fltkernel/nf-fltkernel-fltsendmessage)
- Microsoft WDK: [I/O Requests Generated by the Minifilter Driver](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/i-o-requests-generated-by-the-minifilter-driver)
- Microsoft WDK: [Load Order Groups and Altitudes for Minifilter Drivers](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/load-order-groups-and-altitudes-for-minifilter-drivers)
- Microsoft WDK: [Loading Test Signed Code](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/the-testsigning-boot-configuration-option)
- Microsoft WDK: [Driver Compatibility with HVCI](https://learn.microsoft.com/en-us/windows-hardware/test/hlk/testref/driver-compatibility-with-device-guard)
- MITRE ATT&CK: [T1134.002 Create Process with Token](https://attack.mitre.org/techniques/T1134/002/)
- Tier Zero Security: [Abusing MiniFilter Altitude to blind EDR](https://tierzerosecurity.co.nz/2024/03/27/blind-edr.html)
- Project Zero: [Hunting for Bugs in Windows Mini-Filter Drivers](https://projectzero.google/2021/01/hunting-for-bugs-in-windows-mini-filter.html)
- OSR Community: [Minifilter BSOD KERNEL_AUTO_BOOST_LOCK_ACQUISITION_WITH_RAISED_IRQL](https://community.osr.com/discussion/291805/minifilter-bsod-kernel-auto-boost-lock-acquisition-with-raised-irql)
- nono RETROSPECTIVE.md — v2.8/v2.9 entry, cross-target drift lessons
- nono PROJECT.md — v2.10 milestone scope and Key Decisions §4
- nono CLAUDE.md — § Coding Standards, cross-target clippy rule; § Platform-Specific Notes macOS `/etc` symlink note
- memory/feedback_clippy_cross_target — two cfg-gated compile errors reached release tags in v2.9

---
*Pitfalls research for: nono v2.10 — kernel minifilter spike, EDR HUMAN-UAT, macOS Seatbelt upstream parity*
*Researched: 2026-06-06*
