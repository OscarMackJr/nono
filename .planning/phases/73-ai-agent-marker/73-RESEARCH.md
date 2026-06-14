# Phase 73: AI_AGENT Marker - Research

**Researched:** 2026-06-14
**Domain:** Windows Win32 AppContainer token identity, Job Object security descriptors, in-memory registry, Rust cfg-gating
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Marker = per-run AppContainer package SID (`S-1-15-2-*`) already on every `BrokerLaunchNoPty` token. No new token crafting. One SID serves confinement + network identity + authorization.
- **D-02:** Authorization = minting authority's PRIVATE in-memory registry of SIDs it actually minted. 122-bit random suffix makes the SID unguessable; private registry means a self-made AppContainer is rejected even if named `nono.session.<guess>`.
- **D-03:** Job hardening — (1) explicit SD on `CreateJobObjectW` denying the agent's package SID / Low-IL any job access; (2) negative tests that `JOB_OBJECT_LIMIT_BREAKAWAY_OK` is never set; (3) classification via `IsProcessInJob` / `QueryInformationJobObject` for enumeration only, never authz.
- **D-04:** `nono` crate gets marker-extraction + `AgentRegistry`. Wire mint→registry into live launch path. Ship best-effort `nono classify <pid>` CLI verb documented as NON-authoritative.

### Claude's Discretion

- `AgentRegistry` internal shape (map type, key = package SID bytes/string).
- Error-path wording.
- Precise `nono classify` output format (fail-secure: unknown PID = "not an agent").
- Exact SDDL / security-descriptor construction for the job ACL.
- Exact wording and placement of SC5 adopted-agent documentation.

### Deferred Ideas (OUT OF SCOPE)

- Persistent / multi-tenant / cross-process registry — Phase 74.
- Token-handle pinning — Phase 74 only if SID-value collision becomes a concern.
- First-class `nono agent` / daemon verb namespace — Phase 74.
- Marking `WriteRestricted` / `LowIlPrimary` arms.
- MSI VC++ prereq, POC-cert broker on clean host (unrelated todos).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MARK-01 | Each confined agent carries an unforgeable `AI_AGENT` identity bound to its daemon-minted token SID; a non-agent process cannot claim the identity and a confined agent cannot shed it. Named job objects are for kill-group/enumeration/resource-caps only — never authorization. | Win32 call chain confirmed (Q1); existing `derive_app_container_sid` + `package_sid_to_string` are exact reuse points (Q2); `ConvertStringSecurityDescriptorToSecurityDescriptorW` SDDL pattern already used for SACL construction in this codebase (Q3); `IsProcessInJob` already imported in broker_dispatch_tests (Q4); `AgentRegistry` design is straightforward `HashMap<String, ()>` + `Mutex` (Q5); cfg-gating pattern is identical to `create_low_integrity_primary_token` non-Windows stubs (Q6). |
</phase_requirements>

---

## Summary

Phase 73 is a pure-Rust Windows-API phase with no new library dependencies. All required Win32 APIs exist in windows-sys 0.59 (the pinned version in both `nono` and `nono-cli` crates). The codebase already has every building block: `derive_app_container_sid`, `package_sid_to_string`, `OwnedAppContainerSid`, `ConvertStringSecurityDescriptorToSecurityDescriptorW`, `IsProcessInJob`, `GetTokenInformation`, and the exact SDDL-based security-descriptor construction pattern (used in `try_set_mandatory_label`). The single new Win32 call family not currently called at the classification site is `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `OpenProcessToken(TOKEN_QUERY)` + `GetTokenInformation(TokenAppContainerSid)`, which is a straightforward read-only call chain present in windows-sys 0.59.

The wiring point for D-04 (mint→registry insert) is `execution_runtime.rs` lines 483-488, immediately after `generate_app_container_name()` + `derive_app_container_sid()` + `package_sid_to_string()` — the package SID string `windows_package_sid` is already in scope. The `AgentRegistry` lives in the `nono` crate (library-vs-CLI boundary), exposed as a singleton via `std::sync::OnceLock<Mutex<AgentRegistry>>` or passed as `Arc<Mutex<AgentRegistry>>` (research recommends the `Arc` shape as it is testable without global state). The `nono classify <pid>` CLI verb adds to the `Commands` enum in `nono-cli/src/cli.rs`, with implementation in a new `classify_runtime.rs` (mirrors existing `*_runtime.rs` files).

**Primary recommendation:** Implement in three logical chunks: (1) `AgentRegistry` + marker-extraction in `nono/src/sandbox/windows.rs` + lib.rs re-exports + non-Windows stubs; (2) job-hardening SDDL wiring in `launch.rs` + negative tests; (3) `nono classify` CLI verb + SC4 in-process integration test. No new crate dependencies required.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| AppContainer package SID extraction (cross-process, read a PID's token) | `nono` crate (library) | — | Policy-free primitive; Phase 74 daemon will call the same function. Library-vs-CLI boundary mandates mechanism here. |
| `AgentRegistry` (insert-on-mint, classify) | `nono` crate (library) | `nono-cli` (wiring) | Registry is the authorization predicate, not policy. The daemon (Phase 74) will consume it directly. CLI wires the insert at spawn time. |
| `nono classify <pid>` verb and UX | `nono-cli` | `nono` (underlying primitives) | UX and verb namespace belong in CLI. |
| Job object SDDL / explicit ACL | `nono-cli` (`launch.rs`, `create_process_containment`) | — | Job creation is CLI-layer code; the security descriptor is policy. |
| Negative tests (no BREAKAWAY_OK, agent cannot open job) | `nono-cli` (test module in `launch.rs`) | — | Tests live next to the code they exercise. |
| SC4 in-process integration test | `nono-cli` test | `nono` (AgentRegistry::classify) | Needs a real confined child — requires the broker arm available only from the CLI build. |
| Non-Windows stubs for new nono-crate APIs | `nono` crate (`#[cfg(not(target_os = "windows"))]`) | — | Cross-target clippy mandates stubs (CLAUDE.md MUST). |

---

## Standard Stack

### Core (all already in Cargo.toml — zero new dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `windows-sys` | 0.59 (pinned) | All Win32 FFI: `OpenProcess`, `OpenProcessToken`, `GetTokenInformation(TokenAppContainerSid)`, `EqualSid`, `ConvertStringSecurityDescriptorToSecurityDescriptorW`, `IsProcessInJob` | Already pinned in both `nono` and `nono-cli` Cargo.toml. All required APIs confirmed present at `PROCESS_QUERY_LIMITED_INFORMATION`, `TokenAppContainerSid = 31i32`, `TOKEN_APPCONTAINER_INFORMATION.TokenAppContainer: PSID`, `EqualSid`, `IsProcessInJob`. [VERIFIED: windows-sys registry cache `/c/Users/OMack/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/windows-sys-0.59.0`] |
| `std::sync` | stdlib | `Mutex<AgentRegistry>`, `Arc` for testable registry sharing | Standard Rust — no external dep needed |

### No New Dependencies Required

**Installation:** None — all building blocks are already in-tree.

---

## Package Legitimacy Audit

> No external packages are added in Phase 73. All Win32 APIs are accessed via the already-pinned `windows-sys 0.59` crate. This section is not applicable.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| (none) | — | — | — | — | — | — |

---

## Architecture Patterns

### System Architecture Diagram

```
execution_runtime.rs (nono-cli)
  generate_app_container_name()          ← already exists
  derive_app_container_sid(name)         ← already exists in nono crate
  package_sid_to_string(&psid)           ← already exists in nono crate
      │
      │  [NEW: mint→registry insert]
      ▼
  AgentRegistry::insert(package_sid_str) ← NEW in nono crate
      │
      └── stored in Arc<Mutex<AgentRegistry>>
               passed through ExecConfig or thread-local
                      │
  ┌────────────────────────────────────────────────────────┐
  │  spawn_windows_child / BrokerLaunchNoPty arm           │
  │    CreateProcessW (broker) → confined child            │
  │    apply_process_handle_to_containment (Job assign)    │
  └────────────────────────────────────────────────────────┘
              │
              │  [also NEW: job explicit ACL at CreateJobObjectW]
              ▼
  create_process_containment(session_id)
    CreateJobObjectW(SECURITY_ATTRIBUTES with SDDL)  ← D-03


Classification path:
  nono classify <pid>  (nono-cli)
      │
      ▼
  AgentRegistry::classify(pid)  ← NEW in nono crate
      ├── read_process_appcontainer_sid(pid)
      │     OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, pid)
      │     OpenProcessToken(hProcess, TOKEN_QUERY)
      │     GetTokenInformation(hToken, TokenAppContainerSid, ...)
      │         → TOKEN_APPCONTAINER_INFORMATION.TokenAppContainer (PSID)
      │         → non-AppContainer process: TokenAppContainer is NULL
      │     ConvertSidToStringSidW(psid) → package_sid_str
      │
      ├── pre-filter: does package_sid_str start with "S-1-15-2-"?
      │   + IsProcessInJob(hProcess, NULL, &in_job) — enumeration only
      │
      └── AUTHZ: is package_sid_str in registry.minted_sids?
              YES → AI_AGENT
              NO  → not an agent (fail-secure default)
```

### Recommended Project Structure (new files only)

```
crates/nono/src/
└── agent.rs                  # AgentRegistry, read_process_appcontainer_sid,
                              # classify(), AgentClassification enum
                              # + non-Windows stubs via #[cfg]

crates/nono-cli/src/
└── classify_runtime.rs       # nono classify <pid> implementation
                              # (mirrors existing *_runtime.rs pattern)
```

Existing files modified:
- `crates/nono/src/lib.rs` — re-export `AgentRegistry`, `AgentClassification`
- `crates/nono/src/sandbox/windows.rs` — no changes needed (all SID helpers already here); OR co-locate agent.rs with sandbox/
- `crates/nono-cli/src/cli.rs` — add `Classify(ClassifyArgs)` variant to `Commands` enum
- `crates/nono-cli/src/main.rs` — route `Commands::Classify` to `classify_runtime`
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `create_process_containment`: replace null SA with SDDL-built SD
- `crates/nono-cli/src/execution_runtime.rs` — insert `registry.insert(windows_package_sid.clone())` after line 487

### Pattern 1: AppContainer SID from Foreign PID

**What:** Cross-process read of the AppContainer package SID from an arbitrary PID's token.
**When to use:** In `AgentRegistry::classify(pid)` and `read_process_appcontainer_sid`.

```rust
// Source: windows-sys 0.59 confirmed APIs; mirrors pattern in
// crates/nono/src/sandbox/windows.rs (OpenProcessToken usage at line 539)
// VERIFIED against registry cache: TokenAppContainerSid = 31i32,
// TOKEN_APPCONTAINER_INFORMATION.TokenAppContainer: PSID

#[cfg(target_os = "windows")]
fn read_process_appcontainer_sid(pid: u32) -> Result<Option<String>> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenAppContainerSid,
        TOKEN_APPCONTAINER_INFORMATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    // Open the target process with the minimal required right.
    let h_process = unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid)
    };
    if h_process.is_null() {
        let gle = unsafe { GetLastError() };
        return Err(NonoError::SandboxInit(format!(
            "OpenProcess(pid={pid}) failed (GLE={gle})"
        )));
    }
    let h_process = OwnedHandle(h_process);

    let mut h_token = std::ptr::null_mut();
    let ok = unsafe {
        OpenProcessToken(h_process.raw(), TOKEN_QUERY, &mut h_token)
    };
    if ok == 0 {
        let gle = unsafe { GetLastError() };
        return Err(NonoError::SandboxInit(format!(
            "OpenProcessToken(pid={pid}) failed (GLE={gle})"
        )));
    }
    let h_token = OwnedHandle(h_token);

    // First call: query the required buffer size.
    let mut needed: u32 = 0;
    unsafe {
        GetTokenInformation(
            h_token.raw(),
            TokenAppContainerSid,  // = 31i32
            std::ptr::null_mut(),
            0,
            &mut needed,
        )
    };
    // needed = 0 means the token has no AppContainer SID (non-AppContainer process).
    // This is the correct "not an agent" fast path — not an error.
    if needed == 0 {
        return Ok(None);
    }

    let mut buf = vec![0u8; needed as usize];
    let ok = unsafe {
        GetTokenInformation(
            h_token.raw(),
            TokenAppContainerSid,
            buf.as_mut_ptr() as *mut _,
            needed,
            &mut needed,
        )
    };
    if ok == 0 {
        let gle = unsafe { GetLastError() };
        return Err(NonoError::SandboxInit(format!(
            "GetTokenInformation(TokenAppContainerSid, pid={pid}) failed (GLE={gle})"
        )));
    }

    let info = unsafe { &*(buf.as_ptr() as *const TOKEN_APPCONTAINER_INFORMATION) };
    // TokenAppContainer is NULL for non-AppContainer tokens even when the struct
    // is returned (some Windows versions return the struct with a null SID pointer
    // rather than returning needed=0). Treat null as "not an agent".
    if info.TokenAppContainer.is_null() {
        return Ok(None);
    }

    // Convert the PSID to its SDDL string ("S-1-15-2-*").
    // Reuse the existing package_sid_to_string pattern from windows.rs.
    let sid_str = {
        let owned = OwnedAppContainerSid(info.TokenAppContainer);
        let s = package_sid_to_string_raw(info.TokenAppContainer)?;
        std::mem::forget(owned); // PSID is owned by the token buffer, not by FreeSid
        s
    };
    Ok(Some(sid_str))
}
```

**Critical safety note:** The `PSID` inside `TOKEN_APPCONTAINER_INFORMATION` is owned by the buffer returned by `GetTokenInformation`, NOT by `FreeSid`. Do NOT call `FreeSid` on it. The buffer lives until the `Vec<u8>` is dropped. Extract the string form before dropping the buffer.

### Pattern 2: AgentRegistry Shape

**What:** In-memory set of minted package SID strings (the authorization predicate).
**When to use:** Insert at mint time; classify at any time.

```rust
// Source: standard Rust; no Win32 involved in the data structure itself

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Authorization predicate: the set of package SID strings actually minted by
/// this launcher instance. Only SIDs in this set classify as AI_AGENT.
/// 
/// Key choice: String (SDDL form "S-1-15-2-...") — canonical, printable,
/// already produced by `package_sid_to_string`, and directly comparable to
/// what `read_process_appcontainer_sid` returns. No conversion at classify time.
pub struct AgentRegistry {
    minted_sids: HashSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AgentClassification {
    /// The PID is a launcher-spawned confined agent (authoritative).
    AiAgent { package_sid: String },
    /// The PID was not spawned by this launcher, or the token has no AppContainer SID.
    /// Fail-secure default.
    NotAnAgent,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self { minted_sids: HashSet::new() }
    }

    /// Register a package SID at mint time. Called from the launch path
    /// immediately after `package_sid_to_string` succeeds.
    pub fn insert(&mut self, package_sid_str: String) {
        self.minted_sids.insert(package_sid_str);
    }

    /// Classify a PID. Returns AI_AGENT only if the PID's AppContainer package SID
    /// is in the registry. Fail-secure: any error or missing SID → NotAnAgent.
    #[cfg(target_os = "windows")]
    pub fn classify(&self, pid: u32) -> AgentClassification {
        match read_process_appcontainer_sid(pid) {
            Ok(Some(sid_str)) if self.minted_sids.contains(&sid_str) => {
                AgentClassification::AiAgent { package_sid: sid_str }
            }
            _ => AgentClassification::NotAnAgent,
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn classify(&self, _pid: u32) -> AgentClassification {
        AgentClassification::NotAnAgent
    }
}
```

**Thread-safety:** The launch path (which inserts) and the classify path (which reads) run in the same process. `Arc<Mutex<AgentRegistry>>` is the correct wrapper. For Phase 73's single-launcher use case, `OnceLock<Mutex<AgentRegistry>>` (a process-global singleton) also works. Recommendation: use `Arc<Mutex<AgentRegistry>>` created at launch startup and passed through the launch path — avoids global state and makes the SC4 integration test cleaner (test creates its own registry instance).

### Pattern 3: Job Object Explicit ACL (D-03)

**What:** Replace the null SA in `create_process_containment` with an SDDL-built security descriptor.
**When to use:** At `CreateJobObjectW` call site in `launch.rs` line 199.

The SDDL pattern is ALREADY established in this codebase — `try_set_mandatory_label` (windows.rs line 1040+) uses `ConvertStringSecurityDescriptorToSecurityDescriptorW` with exactly this SDDL→SD→`LocalFree` pattern. The job ACL simply uses a DACL instead of a SACL.

```rust
// Source: established pattern from crates/nono/src/sandbox/windows.rs:1040-1079
// SDDL intent (D-03): grant only owner; deny Low-IL label; deny agent package SID.
// Note: the package SID is NOT known at create_process_containment time (it is
// derived in execution_runtime.rs). See "Design Gap" below.

// Minimal owner-only SDDL for Phase 73:
// D:P   — protected DACL (no inheritance)
// (A;;MAJOBS;;;OW) — ALLOW job_object_all_access to the object owner
// (D;;MAJOBS;;;LW) — DENY Low Integrity label any job access
//
// "MAJOBS" is the SDDL mnemonic for JOB_OBJECT_ALL_ACCESS on Windows.
// If SDDL mnemonic for JOB_OBJECT_ALL_ACCESS is not available in older Windows
// versions, use the hex form 0x1F001F.
//
// The per-agent-package-SID deny ACE requires the SID at create time.
// See "Design Gap: Package SID at Job Create Time" below.

fn build_job_security_attributes(
    // package_sid_str: Option<&str>,  // pass if known at this call site
) -> Result<(Vec<u8>, SECURITY_ATTRIBUTES)> {
    let sddl = "D:P(A;;0x1F001F;;;OW)(D;;0x1F001F;;;LW)";
    // If package SID is known: append "(D;;0x1F001F;;;<package_sid>)"
    let wide_sddl: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();

    let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide_sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(NonoError::SandboxInit(format!(
            "ConvertStringSecurityDescriptorToSecurityDescriptorW for job ACL failed (GLE={})",
            unsafe { GetLastError() }
        )));
    }

    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd,
        bInheritHandle: 0,
    };
    // Caller must LocalFree(sd) after CreateJobObjectW.
    // Return the SD as a raw pointer in the SA; caller holds the buffer.
    Ok((vec![], sa))  // real impl: return OwnedSecurityDescriptor
}
```

### Design Gap: Package SID Not Available at Job Create Time

The call chain is:
1. `create_process_containment(session_id)` in `launch.rs` (creates job, line 189-244) — **no package SID available here**
2. `derive_app_container_sid` / `package_sid_to_string` in `execution_runtime.rs` (lines 485-488) — **package SID produced here**

The package SID is derived BEFORE `spawn_windows_child` is called, but AFTER `create_process_containment`. Two approaches:

**Option A (recommended): Pass package_sid_str into `create_process_containment`.** Refactor signature from `create_process_containment(session_id: Option<&str>)` to `create_process_containment(session_id: Option<&str>, package_sid: Option<&str>)`. The call sites in `launch.rs` (lines 818, 892) are reached from `spawn_windows_child`, which already receives `ExecConfig` that carries `package_sid: Some(windows_package_sid)`. Thread it through.

**Option B:** Create the job with owner-only SDDL (no per-SID deny ACE on creation), then call `SetSecurityInfo` after spawn to add the per-agent-SID deny ACE. More complex, race-window between create and tighten. Not recommended.

**Option C:** Accept that the per-package-SID deny ACE in D-03 requires the refactor. The owner-only + Low-IL-deny SDDL already provides the core hardening; the package-SID deny ACE is belt-and-suspenders (MIC Low→Medium already blocks upward writes, and the job ACL primarily prevents the agent from opening/modifying its own job object).

Recommendation: Option A. The planner should add a refactor task that threads `package_sid: Option<&str>` through `create_process_containment`.

### Pattern 4: IsProcessInJob for Enumeration-Only Classification

```rust
// Source: already used in nono-cli broker_dispatch_tests (launch.rs line 3072)
// Already imported in mod.rs line 51 (in test cfg only).
// For production use in AgentRegistry::classify, import in the classify fn.

use windows_sys::Win32::System::JobObjects::IsProcessInJob;

fn is_pid_in_any_nono_job(h_process: HANDLE) -> bool {
    // Pass NULL as the job handle — Windows returns TRUE if the process is
    // in ANY job object (not just a specific one). This is the enumeration
    // pre-filter only — NOT the authz check.
    let mut in_job: i32 = 0;
    let ok = unsafe { IsProcessInJob(h_process, std::ptr::null_mut(), &mut in_job) };
    ok != 0 && in_job != 0
}
```

**Key behavior:** `IsProcessInJob(hProcess, NULL, &result)` returns TRUE in `result` if the process is in ANY job object. This is a cheap pre-filter for the `nono classify` display; the authz check is always `registry.minted_sids.contains(sid)`.

### Pattern 5: Non-Windows Stubs (cfg-gating)

Every new public function added to `crates/nono/src/agent.rs` that uses Win32 APIs MUST have a non-Windows stub. Pattern from existing `create_low_integrity_primary_token`:

```rust
// In crates/nono/src/agent.rs

#[cfg(target_os = "windows")]
pub fn read_process_appcontainer_sid(pid: u32) -> Result<Option<String>> {
    // ... real Win32 implementation ...
}

#[cfg(not(target_os = "windows"))]
pub fn read_process_appcontainer_sid(_pid: u32) -> Result<Option<String>> {
    Err(NonoError::UnsupportedPlatform(
        "AppContainer SID classification is Windows-only".into(),
    ))
}
```

The `AgentRegistry::classify` non-Windows stub is already shown above (returns `NotAnAgent`).

### Anti-Patterns to Avoid

- **Using namespace pattern match for authz:** `package_sid_str.starts_with("S-1-15-2-") && name.starts_with("nono.session.")` is forgeable — fails SC2. The registry check is mandatory.
- **Calling `FreeSid` on the PSID from `GetTokenInformation`:** That PSID is owned by the buffer, not heap-allocated via `DeriveAppContainerSidFromAppContainerName`. Only call `FreeSid` on SIDs returned by `CreateWellKnownSid`, `DeriveAppContainerSidFromAppContainerName`, or similar alloc-returning APIs.
- **`unwrap()` / `expect()` anywhere:** Enforced by `clippy::unwrap_used` (`-D clippy::unwrap_used`). Every fallible Win32 call must propagate via `?` or return `NotAnAgent` (fail-secure).
- **Checking `needed == 0` after the first `GetTokenInformation` call as an error:** `needed = 0` is the correct return when the token has no AppContainer SID (non-AppContainer process). It is not an error; it is the "not an agent" fast path.
- **Testing the classify path with `#[allow(clippy::unwrap_used)]` in the test but not guarding with `#[cfg(target_os = "windows")]`:** The SC4 in-process integration test spawns a real confined child and requires the broker arm. It MUST be `#[cfg(all(test, target_os = "windows"))]` and `#[ignore]` (requires dev-layout / signed broker). Mark with `// requires: real Win11 host, dev-layout nono.exe` comment.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SDDL → security descriptor conversion | Manual ACL building (`InitializeAcl`, `AddAccessAllowedAce`, etc.) | `ConvertStringSecurityDescriptorToSecurityDescriptorW` with SDDL string | Already the codebase pattern (try_set_mandatory_label line 1062); SDDL is auditable, reduces error surface. |
| SID string conversion | Manual sub-authority iteration | `ConvertSidToStringSidW` → already wrapped as `package_sid_to_string` | Already in the codebase. |
| SID equality | Byte comparison of raw PSID | String equality on SDDL forms (the `S-1-15-2-*` string) | Simpler and already produced by both the mint path and the classify path. `EqualSid` would also work but adds FFI when string equality suffices. |

---

## Runtime State Inventory

Phase 73 is a net-new code feature (AgentRegistry, classify verb, job ACL hardening) with no rename/refactor component. No runtime state migration is required.

- **Stored data:** None — AgentRegistry is per-run, in-memory.
- **Live service config:** None.
- **OS-registered state:** None (no new service, no new scheduled task).
- **Secrets/env vars:** None.
- **Build artifacts:** None beyond normal Cargo build outputs.

---

## Common Pitfalls

### Pitfall 1: PSID Ownership After GetTokenInformation

**What goes wrong:** Calling `FreeSid` on the `TokenAppContainer` PSID field from `TOKEN_APPCONTAINER_INFORMATION`. This double-frees memory owned by the buffer.

**Why it happens:** `OwnedAppContainerSid` is documented to call `FreeSid` on Drop. If you wrap the extracted PSID in an `OwnedAppContainerSid`, it will `FreeSid` on drop — but that SID was not allocated by `DeriveAppContainerSidFromAppContainerName`, it points into the `Vec<u8>` buffer.

**How to avoid:** Extract the string form (`ConvertSidToStringSidW`) while the buffer is alive, then drop the buffer. Never wrap the buffer-internal PSID in `OwnedAppContainerSid`. Use `std::mem::forget` if you accidentally create one, or write a private helper that does NOT use `OwnedAppContainerSid`.

**Warning signs:** Heap corruption / `STATUS_ACCESS_VIOLATION` in tests.

### Pitfall 2: GetTokenInformation Returns needed=0 for Non-AppContainer

**What goes wrong:** Treating `needed = 0` after the first (size-query) `GetTokenInformation(TokenAppContainerSid, null, 0, &mut needed)` as an error.

**Why it happens:** For non-AppContainer processes, `TokenAppContainerSid` returns `needed = 0` (the struct has nothing to return). The return value of the API call itself is 0 (failure), but `GetLastError()` returns `ERROR_INSUFFICIENT_BUFFER` would NOT be set — the behavior is that the API simply doesn't write anything and needed stays 0.

**How to avoid:** Check `if needed == 0 { return Ok(None); }` before the buffer-allocating second call. This is the "not an agent" fast path.

**Warning signs:** `Ok(None)` is never returned for non-AppContainer processes; every PID classifies as an error instead.

### Pitfall 3: SDDL LW Deny ACE May Need hex Rights

**What goes wrong:** SDDL mnemonic `MAJOBS` for job object rights may not be recognized on all Windows 10 builds.

**Why it happens:** SDDL mnemonics for non-standard object types (job objects) are not always documented or available. `JOB_OBJECT_ALL_ACCESS` = `0x1F001F`.

**How to avoid:** Use `0x1F001F` in hex form in the SDDL rather than an undocumented mnemonic: `(A;;0x1F001F;;;OW)`.

### Pitfall 4: Non-Windows Stubs Missing → Cross-Target Clippy Failure

**What goes wrong:** Adding new cfg-gated functions to `nono/src/agent.rs` (or `sandbox/windows.rs`) without non-Windows stubs causes `cargo clippy --target x86_64-unknown-linux-gnu` to fail (unused import, unresolved symbol on the re-export path).

**Why it happens:** lib.rs re-exports gate everything `#[cfg(target_os = "windows")]` but the module-level re-export needs to exist for Linux/macOS. If `crates/nono/src/agent.rs` is not gated at the module level in lib.rs, the stub functions still need to compile.

**How to avoid:** Follow the CLAUDE.md MUST rule: add non-Windows stubs for every new public function. Pattern: `#[cfg(not(target_os = "windows"))]` block returns `Err(NonoError::UnsupportedPlatform(...))` or `NotAnAgent`.

### Pitfall 5: SC4 Integration Test Needs Real Confined Agent (Not a Mock)

**What goes wrong:** Unit-testing `classify(pid)` with the current process's PID (which is Medium IL, not AppContainer).

**Why it happens:** The real test proves three outcomes: (1) a real confined agent → `AI_AGENT`; (2) the current process (unrelated, no AppContainer) → `NotAnAgent`; (3) a self-made AppContainer → `NotAnAgent`. Only the test can create (1) and (3) as real Windows processes.

**How to avoid:** The SC4 test must use the full `BrokerLaunchNoPty` path to spawn a real confined child, then call `AgentRegistry::classify(child_pid)`. Mark `#[ignore]` (requires dev-layout). For the spoof test (3): create a real AppContainer profile + process via `CreateAppContainerProfile` + `CreateProcess` with `SECURITY_CAPABILITIES` — but do NOT insert it into the `AgentRegistry`. `classify(spoof_pid)` must return `NotAnAgent`.

### Pitfall 6: create_process_containment Parameter Threading

**What goes wrong:** D-03's per-agent-package-SID deny ACE on the job requires the package SID at job-creation time, but the current `create_process_containment` signature does not accept a package SID.

**Why it happens:** Architectural sequence — job is created before the AppContainer profile is registered and the package SID is available.

**How to avoid:** Thread `package_sid: Option<&str>` through `create_process_containment`. The caller (`run_windows_supervised` at launch.rs line ~818) passes it from `config.package_sid`. This is a one-line signature change + two call-site updates + a Cargo build to verify.

---

## Code Examples

### Example 1: Non-AppContainer Process Check (fail-secure path)

```rust
// Source: Behavior confirmed by windows-sys 0.59 TOKEN_APPCONTAINER_INFORMATION docs
// For a non-AppContainer process (e.g., cmd.exe at Medium IL):
// GetTokenInformation(hToken, TokenAppContainerSid, null, 0, &needed) sets needed=0
// → the fast-path returns Ok(None) → classify returns NotAnAgent

let result = registry.classify(std::process::id()); // current process is Medium IL
assert_eq!(result, AgentClassification::NotAnAgent);
```

### Example 2: Wiring Insert into execution_runtime.rs

```rust
// File: crates/nono-cli/src/execution_runtime.rs, after line 487
// Existing code (lines 483-488):
//   let windows_app_container_name = exec_strategy::generate_app_container_name();
//   let windows_package_sid = {
//       let psid = nono::derive_app_container_sid(&windows_app_container_name)?;
//       nono::package_sid_to_string(&psid)?
//   };
//
// NEW: register the minted SID before spawning
registry.lock()
    .map_err(|_| NonoError::SandboxInit("AgentRegistry mutex poisoned".into()))?
    .insert(windows_package_sid.clone());
// Note: `.map_err` on PoisonError satisfies clippy::unwrap_used
```

### Example 3: nono classify Output Format

```
$ nono classify 1234
PID 1234: AI_AGENT
  Package SID: S-1-15-2-1234567890-...
  AppContainer: yes
  In job: yes

$ nono classify 5678
PID 5678: not an agent
  AppContainer: no
  (NOTE: This check is structural only — not an authorization decision)

$ nono classify 9999
Error: OpenProcess(pid=9999) failed (GLE=87) — process may not exist
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Null security attributes on `CreateJobObjectW` | Explicit SDDL-built SD with owner-only DACL + Low-IL deny | Phase 73 (this phase) | Confined agent's token cannot open/modify its own job object |
| No identity predicate (job name was the only signal) | AgentRegistry with private minted-SID set | Phase 73 | Authz is now sound: spoofed AppContainer rejected |
| Classification non-existent | `nono classify <pid>` best-effort structural check | Phase 73 | Operator/demo surface; Phase 74 daemon consumes the registry directly |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `GetTokenInformation(TokenAppContainerSid, null, 0, &mut needed)` returns `needed=0` (not an error) for non-AppContainer processes on Windows 11 | Pattern 1 / Pitfall 2 | If it returns an error code instead, the fast-path must check `GetLastError() == ERROR_INSUFFICIENT_BUFFER` and treat needed=0 differently. Low risk — MSDN behavior for null-buffer token info queries is well-established. | [ASSUMED — training knowledge, no official MSDN source fetched in this session] |
| A2 | SDDL hex rights `0x1F001F` for `JOB_OBJECT_ALL_ACCESS` in a job-object security descriptor are accepted by `ConvertStringSecurityDescriptorToSecurityDescriptorW` on Windows 10/11 | Pattern 3 | Hex rights in SDDL are universally accepted by this API. Risk: low. | [ASSUMED] |
| A3 | `PROCESS_QUERY_LIMITED_INFORMATION` is sufficient to call `OpenProcessToken` on a process at Lower or Equal IL | Pattern 1 | If MIC blocks `OpenProcessToken` on a Low-IL AppContainer process from Medium-IL caller, classification fails. Research: MIC applies to object ACLs, not to `OpenProcess` with this access right — the right is specifically designed for monitoring tools. Risk: low. | [ASSUMED] |

**If this table is empty:** — it is not empty, see above.

---

## Open Questions

1. **GetTokenInformation(TokenAppContainerSid) behavior for non-AppContainer on older Win10**
   - What we know: On Win11 26200 (the dev/test host), `needed = 0` is the expected return for non-AppContainer tokens.
   - What's unclear: Whether Win10 1809/2004 behaves identically or returns `ERROR_INVALID_PARAMETER`.
   - Recommendation: Add a defensive check: if `needed < size_of::<TOKEN_APPCONTAINER_INFORMATION>()`, return `Ok(None)`. This handles both the `needed=0` and the unlikely alternative-error cases safely.

2. **OwnedHandle re-use in nono crate**
   - What we know: `OwnedHandle` is defined in `crates/nono/src/sandbox/windows.rs` and re-exported from `crates/nono/src/lib.rs`.
   - What's unclear: Whether `agent.rs` should be a new file or its code should live in `sandbox/windows.rs`.
   - Recommendation: New file `crates/nono/src/agent.rs` with `use super::sandbox::windows::OwnedHandle` (or the re-exported `nono::OwnedHandle`). Keeps the sandbox module focused on sandbox policy; the agent module on identity.

3. **Arc<Mutex<AgentRegistry>> threading path**
   - What we know: The launch path in `execution_runtime.rs` runs on the same thread that calls `spawn_windows_child`.
   - What's unclear: Whether passing `Arc<Mutex<AgentRegistry>>` through `ExecConfig` or a separate parameter is cleaner.
   - Recommendation: Pass as a separate parameter to `run_windows_supervised` and `run_windows_direct` (similar to how `cap_file` is threaded). Avoids cluttering `ExecConfig` with authz concerns.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Windows 10/11 (`PROCESS_QUERY_LIMITED_INFORMATION`) | marker-extraction | ✓ (Win11 26200 confirmed) | Win11 26200.8390 | Win10 1809+ (both have this right) |
| `nono-shell-broker.exe` (dev-layout or signed) | SC4 integration test | ✓ (dev-layout from `cargo build`) | current build | Test marked `#[ignore]` if not built |
| `cargo clippy --target x86_64-unknown-linux-gnu` | cross-target lint | Needs verification on dev host | — | Mark PARTIAL per cross-target checklist |
| `cargo clippy --target x86_64-apple-darwin` | cross-target lint | Needs verification on dev host | — | Mark PARTIAL per cross-target checklist |

**Missing dependencies with no fallback:** None blocking implementation.

**Missing dependencies with fallback:** Cross-target clippy toolchains — may need `rustup target add x86_64-unknown-linux-gnu x86_64-apple-darwin` on dev host per CLAUDE.md cross-target mandate.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (cargo test) |
| Config file | none (workspace-level `[lints]` in Cargo.toml) |
| Quick run command | `cargo test -p nono --target x86_64-pc-windows-msvc` |
| Full suite command | `make test` (runs lib + cli + doc tests) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| MARK-01 SC1 | Launched agent is marked with its package SID | integration (real child, Windows) | `cargo test -p nono-cli --target x86_64-pc-windows-msvc sc4_classify_real_agent -- --ignored` | ❌ Wave 0 |
| MARK-01 SC2 | Non-agent PID returns NotAnAgent (fail-secure) | unit | `cargo test -p nono --target x86_64-pc-windows-msvc agent::tests::classify_current_process_not_agent` | ❌ Wave 0 |
| MARK-01 SC2 | Spoof AppContainer (not in registry) returns NotAnAgent | integration (real AppContainer child) | `cargo test -p nono-cli --target x86_64-pc-windows-msvc sc4_classify_spoof_not_agent -- --ignored` | ❌ Wave 0 |
| MARK-01 SC3 | BREAKAWAY_OK NOT set on job | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc launch::tests::job_never_has_breakaway_ok` | ❌ Wave 0 |
| MARK-01 SC3 | Job has explicit SD (owner-only, Low-IL denied) | unit | `cargo test -p nono-cli --target x86_64-pc-windows-msvc launch::tests::job_security_descriptor_denies_low_il` | ❌ Wave 0 |
| MARK-01 SC4 | classify(unrelated_pid) = NotAnAgent | unit | `cargo test -p nono --target x86_64-pc-windows-msvc agent::tests::classify_nonexistent_pid_not_agent` | ❌ Wave 0 |
| MARK-01 SC5 | `nono classify <pid>` outputs "not an agent" for unrelated PID | smoke (manual UAT step) | manual only (CLI output) | — |

### Sampling Rate

- **Per task commit:** `cargo test -p nono --target x86_64-pc-windows-msvc 2>&1` (unit tests only; fast)
- **Per wave merge:** `cargo test -p nono -p nono-cli --target x86_64-pc-windows-msvc 2>&1` + cross-target clippy
- **Phase gate:** Full `make test` green + real Win11 UAT (`nono classify <confined-pid>` outputs `AI_AGENT`) before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `crates/nono/src/agent.rs` — new module: `AgentRegistry`, `AgentClassification`, `read_process_appcontainer_sid` (Windows + stub)
- [ ] `crates/nono/src/agent.rs#[cfg(all(test, target_os = "windows"))]` test module — covers classify SC2 + SC4 unit cases
- [ ] `crates/nono-cli/src/classify_runtime.rs` — new file: `nono classify <pid>` implementation
- [ ] `crates/nono-cli/src/exec_strategy_windows/launch.rs` test module addition — `job_never_has_breakaway_ok` + `job_security_descriptor_denies_low_il`
- [ ] SC4 integration test in `crates/nono-cli/src/exec_strategy_windows/launch.rs` broker_dispatch_tests — `sc4_classify_real_agent` + `sc4_classify_spoof_not_agent` (both `#[ignore]`)

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (process identity) | AppContainer package SID + private registry (D-01, D-02) |
| V3 Session Management | no | — |
| V4 Access Control | yes (classify → authz predicate) | Registry check is the sound predicate; classification is NON-authoritative display only |
| V5 Input Validation | yes (PID input to classify, package_sid_str in registry key) | PIDs are u32 (no injection); SID strings validated by `ConvertSidToStringSidW` output format |
| V6 Cryptography | no | — |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SID namespace spoofing (`nono.session.<guess>`) | Spoofing | 122-bit random UUID — unguessable; private registry rejects even correct-namespace names not in registry |
| Job object renaming / namespace hijacking | Spoofing | Job name is NOT the auth signal; token SID is. Job name = kill-group handle only |
| Opening the named job by name to claim membership | Elevation of Privilege | Opening the job by name does not grant job membership or identity; membership is set by `AssignProcessToJobObject` at spawn time |
| Agent calling `TerminateJobObject` on its own job | Tampering | Explicit ACL denies Low-IL / package-SID any job access (D-03) |
| Breakaway from job to shed confinement | Tampering | `JOB_OBJECT_LIMIT_BREAKAWAY_OK` NOT set (verified by negative test, already true today per codebase inspection) |
| Cross-PID classification forgery (attacker passes wrong PID to classify) | Spoofing | `read_process_appcontainer_sid(pid)` opens the real token of `pid` — cannot be lied to about PID ownership |

---

## Project Constraints (from CLAUDE.md)

| Directive | Impact on Phase 73 |
|-----------|-------------------|
| No `.unwrap()` / `.expect()` (`clippy::unwrap_used` enforced) | Every Win32 return value checked; `PoisonError` on Mutex via `.map_err`; GetLastError path on every FFI call |
| Library-vs-CLI boundary: mechanism in `nono`, verb/UX in `nono-cli` | `AgentRegistry`, `read_process_appcontainer_sid`, `classify` → `nono` crate; `nono classify` verb → `nono-cli` |
| Cross-target clippy: any cfg-gated Unix code touched needs Linux+macOS clippy | `agent.rs` is new Windows-cfg-gated code in `nono` crate → must have non-Windows stubs; re-exports in `lib.rs` must be cfg-gated |
| `#[must_use]` on functions returning critical Results | `AgentRegistry::classify` should be `#[must_use]`; `read_process_appcontainer_sid` similarly |
| DCO sign-off on all commits | Every commit: `Signed-off-by: Oscar Mack Jr <oscar.mack.jr@gmail.com>` |
| Fail secure | Unknown PID → `NotAnAgent` (never a false positive); `Err` paths in classify → `NotAnAgent` (not propagated to caller as an error that could be ignored) |
| GSD workflow enforcement | All changes via `/gsd:execute-phase 73` |
| No `#[allow(dead_code)]` | All new code must have tests that exercise it |

---

## Sources

### Primary (HIGH confidence)

- `crates/nono/src/sandbox/windows.rs` (read this session) — `derive_app_container_sid`, `package_sid_to_string`, `OwnedAppContainerSid`, `create_low_integrity_primary_token`, `ConvertStringSecurityDescriptorToSecurityDescriptorW` pattern at line 1062, `try_set_mandatory_label`
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` (read this session) — `generate_app_container_name()`, `generate_session_sid()`
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` (read this session) — `create_process_containment` at line 189-244 (null SA at line 199), `select_windows_token_arm` at line 1193, `BrokerLaunchNoPty` arm at line 1867, `IsProcessInJob` usage at line 3072
- `crates/nono-cli/src/execution_runtime.rs` lines 483-530 (read this session) — exact wiring point for mint→registry insert
- `crates/nono/src/lib.rs` (read this session) — re-export surface for Windows-only APIs
- windows-sys 0.59 registry cache (verified in this session):
  - `TokenAppContainerSid = 31i32` [VERIFIED: `/c/Users/OMack/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/windows-sys-0.59.0/src/Windows/Win32/Security/mod.rs`]
  - `TOKEN_APPCONTAINER_INFORMATION { pub TokenAppContainer: PSID }` [VERIFIED: same file]
  - `PROCESS_QUERY_LIMITED_INFORMATION: PROCESS_ACCESS_RIGHTS = 4096u32` [VERIFIED: Threading/mod.rs]
  - `OpenProcess(dwdesiredaccess, binherithandle, dwprocessid) -> HANDLE` [VERIFIED: Threading/mod.rs]
  - `EqualSid(psid1, psid2) -> BOOL` [VERIFIED: Security/mod.rs]
  - `IsProcessInJob(processhandle, jobhandle, result: *mut BOOL) -> BOOL` [VERIFIED: JobObjects/mod.rs]
- CONTEXT.md (read this session) — D-01..D-04, SC1..SC5 locked decisions
- REQUIREMENTS.md (read this session) — MARK-01 + DMON traceability
- ROADMAP.md (read this session) — Phase 73/74 detail + pitfall table

### Secondary (MEDIUM confidence)

- `.claude/skills/spike-findings-nono/references/windows-confinement-model.md` (read this session) — validated confinement model; AppContainer + Job Object + Low-IL mechanics
- `.claude/skills/spike-findings-nono/references/engine-agnostic-confinement.md` (read this session) — spike 003 VALIDATED; BrokerLaunchNoPty is the marked arm

### Tertiary (LOW confidence)

- A1-A3 in Assumptions Log — training-data knowledge of `GetTokenInformation(TokenAppContainerSid)` behavior for non-AppContainer tokens; needs Win11 host verification during SC4 test authoring

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all Win32 APIs verified in windows-sys 0.59 registry cache; all codebase integration points read and confirmed
- Architecture: HIGH — wiring point identified to the exact line (execution_runtime.rs 483-488); design gap (package SID threading) identified with recommended solution
- Pitfalls: HIGH — PSID ownership trap and GetTokenInformation null-buffer behavior confirmed from codebase patterns; cfg-gating requirements confirmed from CLAUDE.md and existing code

**Research date:** 2026-06-14
**Valid until:** 2026-07-14 (stable Win32 APIs; windows-sys 0.59 pinned in Cargo.toml — no drift risk within 30 days)
