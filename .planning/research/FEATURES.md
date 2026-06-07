# Feature Research: nono v2.10

**Domain:** OS-enforced sandboxing — kernel driver spike, EDR validation, macOS upstream parity
**Researched:** 2026-06-06
**Confidence:** HIGH (codebase direct inspection + upstream git diff + Windows kernel driver docs + official Microsoft Learn)

---

## Context

This document is **milestone-scoped to v2.10**. It supersedes the prior v2.0-era FEATURES.md
(Windows gap closure). The v2.9 foundation is complete: WFP user-mode service, Low-IL tokens,
AppContainer, Job Objects, broker, Seatbelt macOS backend, UPST7 (v0.57.0..v0.59.0) synced.

Three new capabilities:

1. **Gap 6b: Windows kernel minifilter — feasibility spike (POC)**
2. **WR-02 EDR HUMAN-UAT — validate nono under a real EDR runner**
3. **macOS Seatbelt upstream parity through `v0.61.2`**

---

## Feature Landscape

### Table Stakes (Users Expect These)

These are the minimum deliverables that make v2.10 coherent. Missing any of them leaves a
stated milestone goal unfulfilled.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Minifilter spike: IRP_MJ_CREATE pre-op intercept (proof)** | Gap 6b has been deferred as "needs kernel driver" since v2.0. Users running on Windows expect *some* path toward runtime file-open enforcement parity with Linux seccomp-BPF+SIGSYS. The spike proves or refutes feasibility. | HIGH | C/unsafe Rust kernel code; test-sign pipeline required; cannot run in userspace. Driver must register with FltMgr via `FltRegisterFilter`, attach altitude in INF, implement `PFLT_PRE_OPERATION_CALLBACK` for `IRP_MJ_CREATE`. |
| **Minifilter spike: allow/deny decision returned from pre-op** | To be useful as a trust-enforcement layer, the driver must be able to return `FLT_PREOP_COMPLETE` with `STATUS_ACCESS_DENIED` to block a file open. A logging-only spike (like WDK `minispy`) proves observability but not enforcement. | HIGH | Return `FLT_PREOP_COMPLETE` + set `Data->IoStatus.Status = STATUS_ACCESS_DENIED` to deny; `FLT_PREOP_SUCCESS_NO_CALLBACK` to allow passthrough. Both must be demonstrated in the spike. |
| **Minifilter spike: test-sign pipeline and bcdedit test-mode boot** | A kernel driver must be signed to load. For a spike/POC, test signing via WDK self-cert + `bcdedit /set testsigning on` on the dev host is sufficient. Production EV signing is out of scope. | MEDIUM | WDK VS2022 extension (`WDK.vsix` from `C:\Program Files (x86)\Windows Kits\10\Vsix\VS2022\...`); `makecert`/`signtool` pipeline; `fltmc load`/`fltmc unload` for manual registration during spike. Altitude must be assigned; use a placeholder from the FSFilter Activity Monitor range (320000–329999) for spikes. |
| **Minifilter spike: ADR + go/no-go recommendation** | The milestone explicitly requires a deliverable ADR. Without it, the spike is just a demo. | LOW | Must record: altitude range selection rationale, user-mode communication design (FltSendMessage port or registry-policy passthrough), blocking latency observed, BSOD risk surface, production viability verdict. |
| **WR-02 EDR UAT: execute on a real EDR-equipped host** | WR-02 has been deferred since v2.1 with "pending EDR-instrumented runner." v2.10 is the milestone that finally runs it. | MEDIUM | Requires a host with a real EDR agent (Defender for Endpoint, Crowdstrike, or SentinelOne). All success criteria must be asserted by a human operator. Cannot be automated. |
| **WR-02 EDR UAT: record pass/fail against concrete success criteria** | Deferred UATs that produce no verdicts have zero value. The UAT must result in a closed or explicitly re-scoped WR-02 item. | LOW | See EDR UAT success criteria table below. |
| **macOS $PWD symlink CWD fix** | Upstream `362ada22`. Without `--workdir`, `getcwd()` returns the canonical path; on macOS, Seatbelt uses literal path matching, so `cd /symlink && nono run` causes EPERM on the symlink path. This is a correctness regression users hit with common project layouts. | LOW | `sandbox_prepare.rs`: prefer `$PWD` over `current_dir()` when valid, validate against `canonicalize` match. Two unit tests already upstream. |
| **macOS CWD symlink path in FsCapability** | Upstream `8f1b0b74`. Companion to the $PWD fix — `FsCapability` was constructed from the already-canonical path, so `path_filters_for_cap` never emitted Seatbelt rules for the symlink path. Also: unconditional macOS-only check when workdir != its canonical form (even when canonical path already in caps). | LOW | `sandbox_prepare.rs`; `deduplicate()` handles the merge. |
| **macOS Seatbelt platform-rules-after-write-allows ordering fix** | Upstream `8f84d454`. `add_deny_access` / `unsafe_macos_seatbelt_rules` were emitted BEFORE user write allows. Under Seatbelt's last-rule-wins semantics for equal-specificity rules, user write allows silently overrode targeted denies — a security correctness bug. | LOW | `macos.rs`: move platform rule emission to AFTER write allow block. Comment in source must be updated to match new ordering rationale. |
| **macOS --trust-proxy-ca flag** | Upstream `729697c2`. Go CLI tools (`gh`, `terraform`) ignore `SSL_CERT_FILE` and verify TLS only via `com.apple.trustd`. Without this flag, `nono run` with a proxy causes `x509: certificate is not trusted` for all Go-based tooling. Blocks real-world agentic use on macOS with network filtering. | MEDIUM | New `crates/nono-cli/src/macos_trust.rs` module (299 lines); Security.framework FFI; ECDSA P-256 CA in macOS Keychain; biometric trust prompt on first run. macOS-only (`#[cfg(target_os = "macos")]`). Includes `fix(proxy)` cleanup/expiry companions `2f4e1a37` + `197008ae`. |

### Differentiators (Beyond Minimum Viable Deliverable)

These features exceed the minimum milestone scope but have clear value if capacity allows.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Minifilter spike: FLT_PREOP_PENDING + user-mode policy roundtrip** | Proves the full architectural design: kernel pends the IRP, sends file metadata to a user-mode policy daemon via `FltSendMessage`/communication port, receives allow/deny, calls `FltCompletePendedPreOperation`. This is what production nono-wfp-driver.sys would need to do. | HIGH | `FltSendMessage` with a timeout is the kernel side; user-mode `FilterGetMessage`/`FilterReplyMessage` is the policy daemon side. Latency measurement critical: even a simple pend+reply adds ~1-5ms per file open; must be quantified for go/no-go. Deadlock risk: `FLT_PREOP_PENDING` must not be returned for pagefile/boot-critical opens. |
| **Minifilter spike: pre-exec image load interception** | Closes the full scope of Gap 6b. `IRP_MJ_ACQUIRE_FOR_SECTION_SYNCHRONIZATION` with `SyncType == SyncTypeCreateSection` can block executable section mapping (the pre-exec intercept path). On Windows 8+, apps can bypass `IRP_MJ_CREATE` by opening via file mapping. | HIGH | Important caveat: can only fail with `STATUS_INSUFFICIENT_RESOURCES` (not `STATUS_ACCESS_DENIED`) per WDK constraint — this limits the deny semantics. `PsSetLoadImageNotifyRoutine` is observe-only (cannot block). The minifilter path is the only blocking option. |
| **Interactive denied-path selector** | Upstream `1cfb5363` + `f9271fd2`. When a denial occurs, instead of just showing the denied path, the CLI offers an interactive selector to add the path to the profile. Visible UX improvement on macOS/Linux. | LOW | Unix-only new feature; `denial_selector.rs`; requires terminal. Not Windows-relevant for this milestone. |
| **cap-file tmpdir mismatch fix** | Upstream `4911d6f1`. `nono why --self` failed if the cap file was under a different `/tmp` alias (e.g., `/private/tmp` vs `/tmp`). Trivial fix. | LOW | `why_runtime.rs`: accept cap file under any known temp root. Affects macOS primarily. |
| **Profile diagnostic suppression** | Upstream `cc21229f`. `suppress_system_service_diagnostics: true` in profile suppresses noisy system-service denial diagnostics. UX polish for macOS agentic profiles. | LOW | `policy.json` field + profile struct addition. Cross-platform. |
| **Registry refs in profile extends** | Upstream `20cc5df9`. Allows `extends: ["registry:some-profile"]` in profiles, not just built-in names. Cross-platform profile ergonomics. | LOW | `profile/mod.rs`. Cross-platform. |

### Anti-Features (Do Not Build in v2.10)

| Feature | Why Requested | Why Anti-Feature | Alternative |
|---------|---------------|-----------------|-------------|
| **Production EV/WHQL-signed minifilter driver** | Logical follow-on to the spike. | Out of scope — requires EV certificate purchase (~$400/yr), Microsoft Hardware Compatibility Lab submission (weeks), kernel-version-maintenance commitment across every OS update. The spike ADR gates the decision. | Do the spike first. If go, start EV procurement as a separate milestone. |
| **Rust minifilter bindings via `windows-drivers-rs`** | Microsoft's `windows-drivers-rs` repo promises safe Rust for kernel drivers. | `windows-drivers-rs` is in early stages, only supports KMDF v1.33, and is not production-recommended. Writing a minifilter in Rust today means fighting the toolchain, not the feature. The C reference implementations (WDK scanner/minispy samples) are well-documented. | Write the POC driver in C using WDK samples as baseline. Document the decision in the ADR. If `windows-drivers-rs` matures, re-evaluate. |
| **ETW-based file-open blocking** | Seems simpler than a kernel driver. | ETW providers are observe-only. You cannot block or modify an operation from an ETW event handler. This is a documented Windows API constraint. | ETW for `nono learn` only (already shipped). Blocking requires minifilter. |
| **EDR telemetry emission / EDR evasion hardening** | Logical extension of "validate under EDR." | v2.10 validates *under* EDR — it does not build EDR integrations or evasion resistance. Emitting nono-specific telemetry to EDR vendor APIs is a separate feature (different build dependencies, vendor-specific APIs). | UAT verdict informs whether this is needed. Scope it in v2.11 if UAT reveals a gap. |
| **Non-macOS UPST8 cherry-picks** | Broader upstream sync (v0.60..v0.61.2 Linux/Windows items) is overdue. | Out of scope for v2.10 — the broader Windows/Linux sync stays on its own cadence. Including it here inflates risk and scope. | Schedule as UPST8 in v2.11. |
| **macOS `sandbox-exec` migration** | Apple has deprecated `sandbox-exec`. | The private `sandbox_init()` API has been stable for over a decade and is used by Chrome, Firefox, and many production apps. Apple has not provided a public replacement API. The Hacker News thread on this (June 2026) confirms no migration path exists. Migration is not actionable — there is nothing to migrate to. | Continue using `sandbox_init()`. Monitor for replacement API; document the known deprecation in PITFALLS. |
| **Automated EDR UAT in CI** | Natural follow-on to the human UAT. | EDR agent installation on CI runners requires special runner configuration or an EDR-vendor CI integration (Defender for Endpoint GitHub connector, etc.). This is infrastructure scope, not code scope. | HUMAN-UAT only for v2.10. Automated EDR regression testing is a v2.11+ infrastructure investment. |

---

## Minifilter Spike: What to Prove vs. Defer

This table distinguishes **spike deliverables** (must prove before ADR go/no-go) from **production
items** (deferred to the production driver milestone if the go decision is made).

| Item | Spike Must Prove | Production Driver (deferred) |
|------|-----------------|------------------------------|
| FltMgr registration + altitude selection | YES — `FltRegisterFilter` + INF altitude must load without BSOD | Altitude must be officially assigned via Microsoft `fsfcomm@microsoft.com` request (30 business day lead time) |
| `IRP_MJ_CREATE` pre-op: allow passthrough | YES — `FLT_PREOP_SUCCESS_NO_CALLBACK` for allowed paths | Policy engine integration |
| `IRP_MJ_CREATE` pre-op: deny with STATUS_ACCESS_DENIED | YES — `FLT_PREOP_COMPLETE` with deny status | Full policy rule matching |
| Test-sign pipeline + test-mode boot | YES — driver must load on the dev host in test mode | EV/WHQL signing + deployment without test mode |
| Altitude placeholder in 320000–329999 range (Activity Monitor) | YES — sufficient for a spike | Official altitude allocation from Microsoft |
| `FLT_PREOP_PENDING` + user-mode roundtrip (FltSendMessage) | DIFFERENTIATOR — proves the full design; include if time allows | Production: hardened communication port, policy daemon, timeout handling, deadlock avoidance |
| `IRP_MJ_ACQUIRE_FOR_SECTION_SYNCHRONIZATION` intercept | DIFFERENTIATOR — required for pre-exec blocking | Production: full image-load trust policy |
| Per-session or per-process scoping | DEFER — not needed for proof-of-concept | Production: scope filter to nono Job Object children only |
| Unload safety / BSOD-prevention hardening | MINIMUM — spike must not BSOD the host | Production: full stability test matrix, verifier pass |
| Multi-volume / network filesystem support | DEFER | Production |
| Performance characterization of `FLT_PREOP_PENDING` latency | INCLUDE IN SPIKE REPORT — critical for go/no-go | Production: latency budget, caching policy |
| Kernel-version-maintenance strategy (API changes, compat matrix) | INCLUDE IN ADR — scope the maintenance cost | Production: CI test matrix across Windows 10/11 builds |

---

## EDR HUMAN-UAT: What to Observe and Assert

The UAT must produce concrete verdicts against observable behaviors. "EDR was running" is not
a verdict. Each row is a pass/fail assertion.

| Assertion | Observation Method | Pass Criterion | Fail Criterion |
|-----------|-------------------|----------------|----------------|
| **EDR can see nono supervisor process** | EDR console / process telemetry | EDR logs `nono.exe` process creation with full path + SHA-256 hash | EDR shows no process telemetry for `nono.exe` |
| **EDR can see Low-IL child process** | EDR console / process telemetry | EDR logs the sandboxed child process creation (e.g. `claude.exe`) with integrity level in process attributes | EDR shows no entry for the child process spawned under Low-IL token |
| **EDR can see AppContainer child process** | EDR console / process telemetry | EDR logs the AppContainer-scoped child with package SID visible in telemetry | EDR shows no entry for AppContainer child |
| **EDR does NOT treat nono's Job Object as malicious** | EDR alert feed during a `nono run` session | No alert generated for Job Object creation or `AssignProcessToJobObject` | EDR fires a false-positive alert categorizing the Job Object as suspicious |
| **EDR does NOT treat WFP filter installation as malicious** | EDR alert feed during `nono-wfp-service` start | No alert generated for WFP sublayer/filter add (`FwpmFilterAdd0`) | EDR fires a false-positive alert for WFP filter installation |
| **EDR does NOT treat Low-IL mandatory label application as malicious** | EDR alert feed during `SetNamedSecurityInfoW(LABEL_SECURITY_INFORMATION)` | No alert generated for integrity label writes | EDR flags the label-write operation as privilege escalation or tampering |
| **nono's broker Authenticode gate passes under EDR** | `nono run` exits 0 with expected child output | Broker launches successfully; child output visible | `WinVerifyTrust` returns unexpected result under EDR code-signing hooks; broker trust gate fails |
| **EDR does NOT block nono's named-pipe IPC** | Supervisor IPC keep-alive traffic | Capability requests over the named pipe complete without EDR-injected denial | EDR named-pipe monitoring blocks or delays IPC; timeout or disconnection observed |
| **EDR does NOT inject a DLL into the sandboxed Low-IL child** | Process module list at child startup | No unexpected DLL loaded into the Low-IL child (EDR DLL injection fails at Low-IL boundary per Windows MIC) | EDR DLL injection succeeds into Low-IL child — unexpected (should be blocked by MIC) |
| **WFP enforcement survives EDR network interception** | Attempt blocked outbound connection from sandboxed child | Connection denied as expected; WFP block confirmed in audit log | EDR network inspection intercepts before WFP; connection allowed when WFP block is expected |

**Notes on EDR DLL injection:** Windows Mandatory Integrity Control (MIC) prevents a Medium-IL EDR agent from injecting a DLL into a Low-IL process (`NO_WRITE_UP`). If the EDR runs as System/High-IL it may succeed. The test distinguishes EDR-at-Medium-IL (expected: DLL injection fails) from EDR-at-System-IL (may succeed). Record both the EDR elevation level and the injection verdict.

**Host requirement:** A real production EDR agent (not a sandbox/trial). Defender for Endpoint or Crowdstrike Falcon are the highest-coverage choices for this UAT. Record EDR product + version in the UAT result.

---

## macOS Seatbelt Parity: Upstream Commit Inventory

The fork's macOS high-water mark is upstream `v0.57.0`. The following upstream commits in the
`v0.57.0..v0.61.2` window are **not yet in the fork** (verified via `git cherry`; `+` = in
upstream, not in fork):

| Commit | Subject | Category | macOS Impact | Priority |
|--------|---------|----------|--------------|----------|
| `8f1b0b74` | fix(sandbox): preserve symlink path when adding CWD capability on macOS | Seatbelt correctness | **HIGH** — EPERM on symlinked CWDs | P1 |
| `362ada22` | fix(sandbox): use $PWD to capture symlink CWD without --workdir | Seatbelt correctness | **HIGH** — $PWD vs getcwd() gap | P1 |
| `8f84d454` | fix(macos): emit platform rules after user write allows | Seatbelt security | **HIGH** — targeted denies silently overridden | P1 |
| `729697c2` | feat(proxy): add --trust-proxy-ca for macOS system trust store integration | Proxy/macOS | **HIGH** — Go tools reject proxy CA without this | P1 |
| `2f4e1a37` | fix(proxy): clean up Keychain on trust failure and expand security docs | Proxy/macOS | MEDIUM — companion cleanup to trust-proxy-ca | P2 |
| `197008ae` | fix(proxy): detect user-cancelled trust prompts via OSStatus codes | Proxy/macOS | MEDIUM — companion to trust-proxy-ca | P2 |
| `4e1c7957` | feat(proxy): align leaf cert expiry with CA and add --proxy-ca-validity flag | Proxy cross-platform | LOW — cert validity UX | P3 |
| `1cfb5363` | feat(cli): introduce interactive denied path selector | CLI UX | LOW — Unix UX; macOS benefits but not macOS-only | P3 |
| `4911d6f1` | fix(cli): accept cap file under any known temp root for why --self | CLI | LOW — macOS /tmp vs /private/tmp | P2 |
| `cc21229f` | feat(diagnostic): add profile option to suppress system service diagnostics | CLI | LOW — cross-platform UX | P3 |
| `20cc5df9` | feat(profile): allow registry refs in profile extends | Profile | LOW — cross-platform | P3 |

Note: `be7681c1` (supervisor socket IPC: Unix named socket replacing fd-based IPC), `9820a2eb`, `51f56b82`, `d1851c9e`, `284ae1d3`, `4a22e94c`, `f956fb6c` (supervisor loop keep-alive / read timeout / blocking mode fixes) are cross-platform correctness items that affect both macOS and Linux. These are valuable but are UPST8 scope (broader Windows/Linux/macOS sync), not the macOS-scoped parity slice unless there is phase capacity.

**P1 items are the minimum for "macOS parity" to be a credible deliverable.** P1 items fix active
correctness bugs (EPERM on symlinked CWDs, security ordering defect). Without them, the macOS
backend is incorrectly ordered and silently broken for common symlink-CWD workflows.

---

## Feature Dependencies

```
MINIFILTER SPIKE
  └── requires: WDK + VS2022 + WDK.vsix installed on dev host
  └── requires: bcdedit /set testsigning on (test-mode boot)
  └── requires: INF file with altitude in 320000-329999 range
  └── requires: C driver skeleton (DriverEntry, FltRegisterFilter, FLT_REGISTRATION)
  └── provides: ADR with go/no-go for production driver milestone
  MINIFILTER SPIKE (differentiator: user-mode roundtrip)
    └── requires: MINIFILTER SPIKE (basic intercept) above
    └── requires: FltCreateCommunicationPort (kernel) + FilterConnectCommunicationPort (user)
    └── requires: nono-wfp-driver.sys placeholder replaced with real minifilter code
    └── provides: latency measurement for go/no-go

WR-02 EDR HUMAN-UAT
  └── requires: EDR-equipped host (not a dev VM without EDR)
  └── requires: existing nono installation (v0.62.2 or current main)
  └── requires: Authenticode-signed nono.exe (required for broker trust gate)
  └── depends on: existing Low-IL broker, AppContainer, WFP service (already shipped in v2.9)
  └── provides: closed or re-scoped WR-02

MACOS SEATBELT P1 PARITY (8f1b0b74, 362ada22, 8f84d454)
  └── requires: cherry-pick against current fork main (no conflicting patches expected — confirmed via git cherry)
  └── requires: cross-target clippy verification (--target x86_64-apple-darwin)
  └── provides: correct behavior on macOS symlink CWD + correct security ordering

MACOS --trust-proxy-ca (729697c2)
  └── requires: MACOS SEATBELT P1 PARITY (ordering correctness baseline)
  └── requires: Security.framework FFI (macOS-only, cfg-gated)
  └── requires: cross-target clippy verification (--target x86_64-apple-darwin)
  └── provides: Go tool proxy compat on macOS
```

---

## MVP Definition for v2.10

### Must Ship (Phase 63+)

- [ ] **Minifilter spike** — IRP_MJ_CREATE pre-op intercept (allow + deny), test-sign pipeline, ADR with go/no-go
- [ ] **WR-02 EDR HUMAN-UAT** — execute with real EDR, record all 10 observable assertions
- [ ] **macOS P1 parity** — cherry-pick `8f1b0b74` + `362ada22` + `8f84d454` (symlink CWD + security ordering)

### Should Ship (add if capacity)

- [ ] **macOS --trust-proxy-ca** — `729697c2` + companions `2f4e1a37` + `197008ae`
- [ ] **Minifilter spike: FLT_PREOP_PENDING user-mode roundtrip** — latency measurement + full design proof
- [ ] **cap-file tmpdir fix** — `4911d6f1` (trivial, pair with P1 cherry-picks)
- [ ] **Minifilter spike: IRP_MJ_ACQUIRE_FOR_SECTION_SYNCHRONIZATION** — pre-exec image load intercept

### Defer to v2.11

- [ ] **Non-macOS UPST8** — broader Linux/Windows sync
- [ ] **Production EV/WHQL-signed driver** — gated on spike ADR
- [ ] **Interactive denied-path selector** (`1cfb5363`) — UX polish, not parity
- [ ] **Profile diagnostic suppression** (`cc21229f`) + registry refs (`20cc5df9`) — cross-platform UX polish

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Minifilter spike: allow/deny intercept | HIGH — only path to runtime FS enforcement parity | HIGH — kernel driver, test signing | P1 |
| Minifilter spike: test-sign pipeline | HIGH — prerequisite to loading the driver | MEDIUM — WDK toolchain setup | P1 |
| Minifilter spike: ADR go/no-go | HIGH — gates the production driver investment | LOW — write-up | P1 |
| WR-02 EDR HUMAN-UAT | HIGH — closes 5-milestone-old deferral | MEDIUM — needs EDR host, operator time | P1 |
| macOS $PWD symlink CWD fix | HIGH — correctness bug, common workflow | LOW — small code change | P1 |
| macOS CWD symlink in FsCapability | HIGH — correctness companion | LOW — small code change | P1 |
| macOS platform-rules ordering fix | HIGH — security bug (targeted denies overridden) | LOW — reorder emission in macos.rs | P1 |
| macOS --trust-proxy-ca | HIGH — Go tools broken without it | MEDIUM — Security.framework FFI | P2 |
| Minifilter: FLT_PREOP_PENDING roundtrip | HIGH — proves production design | HIGH — communication port + latency | P2 |
| Minifilter: IRP_MJ_ACQUIRE_FOR_SECTION_SYNCHRONIZATION | MEDIUM — completes pre-exec story | HIGH — complex IRP type, limited deny semantics | P2 |
| cap-file tmpdir fix | LOW — edge case | LOW — trivial | P2 |
| Interactive denied-path selector | MEDIUM — UX improvement | LOW — upstream cherry-pick | P3 |
| Profile diagnostic suppression | LOW — noise reduction | LOW — upstream cherry-pick | P3 |
| Registry refs in profile extends | LOW — power user feature | LOW — upstream cherry-pick | P3 |

**Priority key:** P1 = must ship for milestone; P2 = include if capacity; P3 = UPST8 scope.

---

## Sources

- Windows kernel driver pre-operation callbacks: [Writing Pre-operation Callback Routines](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/writing-preoperation-callback-routines) (HIGH confidence — Microsoft official docs)
- FLT_PREOP_CALLBACK_STATUS: [PFLT_PRE_OPERATION_CALLBACK](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/fltkernel/nc-fltkernel-pflt_pre_operation_callback) (HIGH confidence)
- FltSendMessage user-mode communication: [FltSendMessage function](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/fltkernel/nf-fltkernel-fltsendmessage) + [Communication Between User-mode and Minifilters](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/communication-between-user-mode-and-kernel-mode) (HIGH confidence)
- IRP_MJ_ACQUIRE_FOR_SECTION_SYNCHRONIZATION: [FLT_PARAMETERS for IRP_MJ_ACQUIRE_FOR_SECTION_SYNCHRONIZATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/flt-parameters-for-irp-mj-acquire-for-section-synchronization) (HIGH confidence)
- PsSetLoadImageNotifyRoutine vs minifilter: [FortiGuard Labs research](https://www.fortinet.com/blog/threat-research/windows-pssetloadimagenotifyroutine-callbacks-the-good-the-bad) (MEDIUM confidence)
- Minifilter altitude registration: [Load Order Groups and Altitudes](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/load-order-groups-and-altitudes-for-minifilter-drivers) (HIGH confidence)
- Minifilter test signing: [Signing a Driver During Development and Testing](https://learn.microsoft.com/en-us/windows-hardware/drivers/develop/signing-a-driver-during-development-and-testing) (HIGH confidence)
- Rust for Windows drivers: [windows-drivers-rs](https://github.com/microsoft/windows-drivers-rs) (HIGH confidence — current state: early stage, KMDF v1.33 only)
- WDK minifilter scanner sample (canonical user-mode roundtrip reference): [microsoft/Windows-driver-samples/filesys/miniFilter/scanner](https://github.com/Microsoft/Windows-driver-samples/tree/main/filesys/miniFilter/scanner) (HIGH confidence)
- EDR telemetry via kernel callbacks: [Understanding Telemetry: Kernel Callbacks](https://jonny-johnson.medium.com/understanding-telemetry-kernel-callbacks-1a97cfcb8fb3) (MEDIUM confidence)
- macOS parity commits: upstream `always-further/nono` git log, verified via `git cherry` against fork HEAD (HIGH confidence — direct codebase inspection)
- macOS `sandbox-exec` deprecation discussion: [Hacker News, June 2026](https://news.ycombinator.com/item?id=44283454) (MEDIUM confidence — community signal, no Apple API replacement published)

---
*Feature research for: nono v2.10 — kernel driver spike + EDR UAT + macOS parity*
*Researched: 2026-06-06*
