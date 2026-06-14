---
phase: 73-ai-agent-marker
plan: "01"
subsystem: nono-core
tags: [windows, appcontainer, identity, security, agent-registry]
dependency_graph:
  requires: []
  provides: [AgentRegistry, AgentClassification, read_process_appcontainer_sid]
  affects: [nono-crate-public-api]
tech_stack:
  added: []
  patterns:
    - OwnedHandle RAII from sandbox/windows.rs
    - GetTokenInformation(TokenAppContainerSid) Win32 call chain
    - cfg-gated non-Windows stub pattern from sandbox/windows.rs
    - ConvertSidToStringSidW string extraction from package_sid_to_string
key_files:
  created:
    - crates/nono/src/agent.rs
  modified:
    - crates/nono/src/lib.rs
decisions:
  - "PSID from GetTokenInformation buffer never wrapped in OwnedAppContainerSid (FreeSid double-free avoided); string extracted while Vec<u8> alive"
  - "Module agent.rs is NOT cfg-gated at module level — compiles on all platforms; Windows-specific functions gated per-function with paired cfg blocks"
  - "pub use agent::{AgentClassification, AgentRegistry} is platform-neutral (not inside #[cfg(windows)]) because classify() has non-Windows stubs returning NotAnAgent"
  - "Cross-target clippy PARTIAL — aws-lc-sys C toolchain not installed on dev host; Rust code structure verified; deferred to CI"
metrics:
  duration: "6 minutes"
  completed: "2026-06-14T22:58:55Z"
  tasks_completed: 2
  files_created: 1
  files_modified: 1
---

# Phase 73 Plan 01: AgentRegistry, AgentClassification, read_process_appcontainer_sid Summary

**One-liner:** In-memory `AgentRegistry` (HashSet<String> minted-SID set) with Win32 `OpenProcess → OpenProcessToken → GetTokenInformation(TokenAppContainerSid)` call chain for cross-process package SID extraction, plus non-Windows stubs for cross-target clippy compliance.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1+2 | Create agent.rs + wire lib.rs | 7a2e2088 | crates/nono/src/agent.rs (new), crates/nono/src/lib.rs (modified) |

## What Was Built

### crates/nono/src/agent.rs (new, 416 lines)

Public types (all platforms):
- `AgentClassification` — enum `AiAgent { package_sid: String }` / `NotAnAgent`. Derives `Debug, PartialEq, Eq`. Annotated `#[must_use]`.
- `AgentRegistry` — struct wrapping `HashSet<String>` private `minted_sids`. Methods: `new()`, `insert(String)`, `classify(u32) -> AgentClassification`. classify is `#[must_use]`.

Windows arm (`#[cfg(target_os = "windows")]`):
- `read_process_appcontainer_sid(pid: u32) -> Result<Option<String>>` — 11-step Win32 call chain:
  1. `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid)` → OwnedHandle
  2. `OpenProcessToken(hProcess, TOKEN_QUERY, &mut h_token)` → OwnedHandle
  3. First `GetTokenInformation` with null buffer → `needed`; if needed==0 or < struct size → `Ok(None)` (fast path for non-AppContainer)
  4. Allocate `Vec<u8>` of size `needed`
  5. Second `GetTokenInformation` to fill buffer
  6. Cast to `TOKEN_APPCONTAINER_INFORMATION`, check null → `Ok(None)`
  7. `ConvertSidToStringSidW` on `info.TokenAppContainer` while buffer is alive → SDDL string
  8. Return `Ok(Some(sid_str))`
- `AgentRegistry::classify` — calls `read_process_appcontainer_sid`, checks `minted_sids.contains(&sid_str)`; any error or miss → `NotAnAgent`

Non-Windows arm (`#[cfg(not(target_os = "windows"))]`):
- `read_process_appcontainer_sid` → `Err(NonoError::UnsupportedPlatform(...))`
- `AgentRegistry::classify` → `NotAnAgent` unconditionally

Tests (`#[cfg(all(test, target_os = "windows"))]`):
- `classify_current_process_not_agent` — current process (Medium IL) → NotAnAgent
- `classify_nonexistent_pid_not_agent` — PID 0xFFFF_FFFF → NotAnAgent (fail-secure)
- `insert_and_classify_requires_registry_membership` — fake SID inserted, current process not a match → NotAnAgent
- `read_sid_current_process_returns_none` — Ok(None) for current process

### crates/nono/src/lib.rs (modified)

Two surgical edits:
1. `pub mod agent;` added alphabetically before `pub mod capability;`
2. `pub use agent::{AgentClassification, AgentRegistry};` added after the `#[cfg(windows)]` sandbox re-export block, NOT inside any `#[cfg]` block (platform-neutral)

## Verification Results

| Check | Result |
|-------|--------|
| `cargo build -p nono --target x86_64-pc-windows-msvc` | PASS |
| `cargo test -p nono --target x86_64-pc-windows-msvc agent` | PASS (4/4) |
| `cargo clippy -p nono --target x86_64-pc-windows-msvc -- -D warnings -D clippy::unwrap_used` | PASS (0 warnings/errors) |
| `grep OwnedAppContainerSid agent.rs` (functional use) | 0 matches (2 in comments only — correct) |
| `.unwrap()` outside test boundaries | 0 |
| Cross-target `x86_64-unknown-linux-gnu` clippy | PARTIAL — aws-lc-sys C toolchain missing; deferred to CI |
| Cross-target `x86_64-apple-darwin` clippy | PARTIAL — cc C toolchain missing; deferred to CI |
| Pre-existing failure `try_set_mandatory_label_surfaces_directive...` | Pre-existing; not caused by this plan |

## Deviations from Plan

None — plan executed exactly as written. The PSID ownership safety note was implemented correctly by extracting the string via `ConvertSidToStringSidW` while the `Vec<u8>` buffer lives, without wrapping in `OwnedAppContainerSid`.

## Known Stubs

None. The non-Windows stubs for `read_process_appcontainer_sid` (returns `Err(UnsupportedPlatform)`) and `AgentRegistry::classify` (returns `NotAnAgent`) are intentional per-plan design (D-01: marker is Windows/AppContainer-only). No stub prevents the plan's goal from being achieved.

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes introduced. The `read_process_appcontainer_sid` function is a read-only Win32 call chain; the `AgentRegistry` is a plain in-memory `HashSet`. No new threat surface beyond what is documented in the plan's threat model.

## Self-Check

Files created/modified:
- `crates/nono/src/agent.rs` — FOUND
- `crates/nono/src/lib.rs` — FOUND (contains `pub mod agent` and `pub use agent::{AgentClassification, AgentRegistry}`)

Commits:
- `7a2e2088` — FOUND (feat(73-01): add AgentRegistry, AgentClassification, read_process_appcontainer_sid)

## Self-Check: PASSED
