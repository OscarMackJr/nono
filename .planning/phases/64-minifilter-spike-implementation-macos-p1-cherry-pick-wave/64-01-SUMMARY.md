---
phase: 64
plan: "01"
subsystem: fltmgr-client + macos-sandbox
tags: [windows, fltmgr, spike, macos, seatbelt, tdd-red, cargo-workspace]
dependency_graph:
  requires: []
  provides:
    - nono-fltmgr-client Cargo workspace member (DRV-02 spike crate scaffold)
    - Wave 0 RED test stubs in macos.rs (D-11 ordering tests for cherry-pick 8f84d454)
  affects:
    - Cargo.toml (workspace members)
    - crates/nono-fltmgr-client/ (new crate)
    - crates/nono/src/sandbox/macos.rs (3 tests added/updated)
tech_stack:
  added:
    - windows-sys 0.59 feature Win32_Storage_InstallableFileSystems (FilterConnectCommunicationPort, FilterGetMessage, FilterReplyMessage)
    - windows-sys 0.59 feature Win32_Security (FilterConnectCommunicationPort gate)
    - windows-sys 0.59 feature Win32_System_IO (FilterGetMessage gate)
  patterns:
    - repr(C, packed(1)) struct with compile-time layout assertion (const _: () = assert!)
    - cfg(windows)/cfg(not(windows)) crate-level empty-stub discipline
    - SAFETY: comment convention on every unsafe block
    - Wave 0 RED state TDD: test stubs that MUST fail until cherry-pick lands
key_files:
  created:
    - crates/nono-fltmgr-client/Cargo.toml
    - crates/nono-fltmgr-client/src/lib.rs
    - crates/nono-fltmgr-client/src/main.rs
  modified:
    - Cargo.toml (added crates/nono-fltmgr-client to workspace members)
    - Cargo.lock (updated by cargo build)
    - crates/nono/src/sandbox/macos.rs (3 tests: 1 updated, 2 added)
decisions:
  - "NonoIpcRequest uses repr(C, packed(1)) matching C-side pragma pack(push,1); payload size 532 bytes verified by compile-time assert"
  - "windows-sys 0.59 FilterConnectCommunicationPort signature uses out-parameter HANDLE (not return value); HRESULT return; requires Win32_Security + Win32_System_IO feature gates"
  - "packed struct field access via ptr::read_unaligned/addr_of! to avoid UB from misaligned references in Rust"
  - "x86_64-apple-darwin cross-target clippy PARTIAL: Rust target installed but C toolchain (ring/aws-lc-sys cc) unavailable on Windows host; deferred to live CI per CLAUDE.md rule"
metrics:
  duration: "~45 minutes"
  completed: "2026-06-08"
  tasks: 2
  files: 6
---

# Phase 64 Plan 01: Wave 0 Scaffolding (nono-fltmgr-client + macOS RED tests) Summary

One-liner: New nono-fltmgr-client Cargo workspace member with NonoIpcRequest compile-time layout assertion and Wave 0 RED macOS ordering test stubs that gate cherry-pick 8f84d454.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create nono-fltmgr-client Cargo workspace member | 55372a46 | Cargo.toml, Cargo.lock, crates/nono-fltmgr-client/Cargo.toml, src/lib.rs, src/main.rs |
| 2 | Add Wave 0 macOS ordering test stubs (RED state) to macos.rs | 1f08c925 | crates/nono/src/sandbox/macos.rs |

## What Was Built

### Task 1 — nono-fltmgr-client (DRV-02 spike crate)

The `crates/nono-fltmgr-client/` Cargo workspace member is the user-mode policy client stub for the Phase 64 DRV-02 minifilter IPC round-trip proof. It provides:

- **`NonoIpcRequest`** (`repr(C, packed(1))`): `FILTER_MESSAGE_HEADER` + `[u16; 260]` path buffer + `u32` PID + `u32` desired_access + `u32` reserved = 532-byte payload. The compile-time assertion `const _: () = assert!(size_of::<NonoIpcRequest>() - size_of::<FILTER_MESSAGE_HEADER>() == 532)` links the Rust user-mode client to the C-side `_Static_assert(sizeof(NONO_IPC_REQUEST) == 532)`.

- **`run_policy_client(deny_path: &str)`**: Connects to `\NonoPolicyPort` via `FilterConnectCommunicationPort`, runs a blocking `FilterGetMessage` loop, sends allow/deny replies via `FilterReplyMessage`. Plan 03 fills in the full IPC client logic; this task provides the scaffold and binary entry point.

- **`src/main.rs`**: `#[cfg(windows)]` guarded CLI binary accepting a deny-path argument; `#[cfg(not(windows))]` stub for Linux/macOS CI. The `nono_fltmgr_client.exe` binary produced by `cargo build` is the runnable artifact for the DRV-02 VM round-trip test (Plan 04 Step 5).

- **cfg-guard discipline**: All Windows-specific code lives inside `#[cfg(windows)] mod client { ... }`. The `#[cfg(not(windows))]` empty stub ensures the crate compiles to nothing on Linux/macOS CI (T-64-SC-02).

- **No `.unwrap()` / `.expect()`** in production code — CLAUDE.md Unwrap Policy enforced. `[lints] workspace = true` inherits `clippy::unwrap_used = "deny"`.

### Task 2 — macOS Wave 0 RED Test Stubs (D-11)

Three test functions in `crates/nono/src/sandbox/macos.rs`:

1. **`test_generate_profile_platform_rules_after_writes`** (renamed + updated): Was `test_generate_profile_platform_rules_between_reads_and_writes` (asserted the pre-fix wrong ordering `read < deny < write`). Now asserts the post-fix correct ordering `read < write < deny` — MUST FAIL until cherry-pick `8f84d454` moves the `platform_rules()` loop to after write-allows in `generate_profile`. This is the Wave 0 RED state for Track B.

2. **`test_platform_rules_after_write_allows`** (new): D-11 ordering test. Adds a ReadWrite FsCapability + a platform deny rule, asserts `write_pos < deny_pos`. MUST FAIL with pre-cherry-pick code (deny currently between reads and writes, so `deny_pos < write_pos`). Turns GREEN in Plan 64-03.

3. **`test_platform_deny_symlink_and_canonical_path`** (new): D-11 symlink+canonical coverage. Adds both `/etc/passwd` and `/private/etc/passwd` as platform rules, asserts both appear in the profile. PASSES immediately — validates the dual-path discipline (Pitfall 11: macOS `/private/etc` symlink drift).

## Verification

| Gate | Result |
|------|--------|
| `cargo build -p nono-fltmgr-client` (Windows host) | PASS — exits 0, no warnings |
| `cargo clippy -p nono-fltmgr-client -- -D warnings -D clippy::unwrap_used` (Windows host) | PASS |
| `nono_fltmgr_client.exe` binary produced | PASS — `target/debug/nono_fltmgr_client.exe` exists |
| Workspace `Cargo.toml` contains `crates/nono-fltmgr-client` | PASS |
| `publish = false` in crate Cargo.toml | PASS |
| `[lints] workspace = true` in crate Cargo.toml | PASS |
| `NonoIpcRequest` static assert (532-byte payload) compiles | PASS |
| `src/main.rs` exists with `#[cfg(windows)]` guarded main | PASS |
| No `.unwrap()` / `.expect()` in production code | PASS |
| Three test functions exist in macos.rs | PASS |
| `cargo clippy -p nono-fltmgr-client --target x86_64-apple-darwin` | PASS (empty stub — no Windows imports on macOS) |
| `cargo clippy -p nono --target x86_64-apple-darwin` | PARTIAL — C toolchain (ring/aws-lc-sys cc) unavailable on Windows host; deferred to live CI per CLAUDE.md cross-target rule |
| Tests 1 + 2 FAIL (RED — pre-cherry-pick) | EXPECTED — confirmed by Wave 0 design; macos.rs tests are cfg(target_os = "macos") gated and cannot run on Windows host; will verify on macOS CI |
| Test 3 PASSES | EXPECTED — cannot verify on Windows host (macOS-gated); will verify on macOS CI |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] windows-sys 0.59 FilterConnectCommunicationPort API signature mismatch**
- **Found during:** Task 1 verification (cargo build)
- **Issue:** The RESEARCH.md code example used an incorrect API signature for `FilterConnectCommunicationPort`. In windows-sys 0.59, this function: (a) returns `HRESULT` (not a `HANDLE`), (b) writes the port handle to an out-parameter `*mut HANDLE`, and (c) requires additional feature gates `Win32_Security` (for `FilterConnectCommunicationPort`) and `Win32_System_IO` (for `FilterGetMessage`) beyond the base `Win32_Storage_InstallableFileSystems` feature.
- **Fix:** Corrected the function call signature, added `Win32_Security` and `Win32_System_IO` feature flags to `Cargo.toml`, checked HRESULT return.
- **Files modified:** `crates/nono-fltmgr-client/Cargo.toml`, `crates/nono-fltmgr-client/src/lib.rs`
- **Commit:** 55372a46

**2. [Rule 1 - Bug] Misaligned reference to packed struct field**
- **Found during:** Task 1 verification (cargo build)
- **Issue:** `&buf.path_buffer` is a reference to a field inside `repr(C, packed(1))` struct, which Rust rejects (E0793: reference to field of packed struct is unaligned — UB to create a misaligned reference).
- **Fix:** Used `ptr::read_unaligned(ptr::addr_of!(buf.path_buffer))` to copy path_buffer into a local aligned `[u16; 260]` array before calling `from_utf16_lossy`. Same fix applied to `buf.header.MessageId` field access.
- **Files modified:** `crates/nono-fltmgr-client/src/lib.rs`
- **Commit:** 55372a46

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| `run_policy_client` function body (minimal — only scaffold for Plan 03) | `crates/nono-fltmgr-client/src/lib.rs` | Plan 03 Task 1 implements the full IPC client logic; this task provides the type definitions, layout assertion, and binary entry point only |

## Threat Surface Scan

No new network endpoints, auth paths, or schema changes introduced. The only new surface is the `\NonoPolicyPort` named port client code, which is already in the STRIDE register for this plan as T-64-SC-01 (mitigated by the compile-time layout assertion). No unmodeled surface found.

## Self-Check: PASSED

- `crates/nono-fltmgr-client/Cargo.toml`: FOUND
- `crates/nono-fltmgr-client/src/lib.rs`: FOUND
- `crates/nono-fltmgr-client/src/main.rs`: FOUND
- Commit `55372a46`: FOUND
- Commit `1f08c925`: FOUND
- Three test functions in macos.rs: FOUND (`test_generate_profile_platform_rules_after_writes`, `test_platform_rules_after_write_allows`, `test_platform_deny_symlink_and_canonical_path`)
- Old test name `test_generate_profile_platform_rules_between_reads_and_writes`: ABSENT (renamed)
- `nono_fltmgr_client.exe`: FOUND in `target/debug/`
