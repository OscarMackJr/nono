//! Daemon handle-count-baseline integration tests (Phase 74, Wave 0).
//!
//! These tests form the **Wave 0 spike gate** for Phase 74. They must all PASS
//! on a real Win11 host (dev-layout nono.exe) before Wave 1 implementation
//! (the daemon binary) may begin. They probe the two explicitly "unspiked"
//! mechanisms identified in 74-RESEARCH.md:
//!
//! 1. **Fresh-token isolation** (`fresh_token_isolation_agents_have_distinct_package_sids`):
//!    100 successive `AppContainerProfile` mints produce distinct package SIDs.
//!    Proves that the per-agent profile machinery in the nono lib correctly
//!    yields a new SID for every invocation — token/job reuse would collapse
//!    tenant isolation (Pitfall 2, RESEARCH.md).
//!
//! 2. **Deterministic reap** (`n_agents_over_time_returns_to_baseline_handle_count`):
//!    100 AppContainer profile create→derive→drop cycles are verified with a
//!    post-warmup steady-state assertion (not a cold-baseline assertion) because
//!    the first few cycles pay one-time RPC/threadpool warmup costs (~65 handles).
//!    The steady-state delta (post-warmup→post-run) must be ≤ 5.
//!    Proves that `AppContainerProfile::Drop` correctly calls
//!    `DeleteAppContainerProfile` and frees all handles per cycle.
//!    Handle-type characterization (via NtQuerySystemInformation) uses THREE snapshots:
//!    (1) cold (before warmup), (2) post-warmup plateau, (3) post-full-run.
//!    The WARMUP delta (cold→post-warmup) names the ~65 one-time handles by kernel
//!    object type (Event/Thread/ALPC Port/IoCompletion/TpWorkerFactory/Key/Section etc.).
//!    The STEADY-STATE delta (post-warmup→post-full-run) must be empty (zero per-cycle
//!    growth); any growth in security-critical types (Token/File/Job) triggers a WARN.
//!
//! 3. **Cross-tenant denial** (`daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance`):
//!    A pipe instance created with tenant A's AppContainer package SID as the
//!    only Low-IL-admitting SDDL ACE denies a connection attempt from a process
//!    impersonating a **real** AppContainer B token (a spawned process running in
//!    tenant B's AppContainer, with tenant B's own package SID). This directly
//!    exercises the SDDL DACL gate at the OS level (DMON-02 / SC2).
//!
//! 4. **Concurrent agents** (`daemon_concurrent_agents`):
//!    Two concurrent AppContainer profile sessions produce distinct package SIDs
//!    and each can create its own independent pipe instance (SC1: two agents
//!    served concurrently with distinct SIDs + independent capability access).
//!
//! # Gate
//!
//! Set `NONO_DAEMON_INTEGRATION_TESTS=1` to run. Without this env-var the tests
//! early-return (skipped). This matches the integration-test convention in
//! `aipc_handle_brokering_integration.rs` and prevents the standard CI test
//! suite from exercising host-dependent Win32 operations.
//!
//! # Open questions probed by this spike
//!
//! - **A1**: Does `ImpersonateNamedPipeClient` work in a per-user SCM service token?
//!   Answered by `daemon_cross_tenant_denial_...` passing (the impersonation sequence
//!   works at all from the test process context — confirms the Win32 call chain).
//!
//! - **A2**: Exact `TokenAppContainerSid` variant in windows-sys 0.59?
//!   `windows_sys::Win32::Security::TokenAppContainerSid` — confirmed value `31i32`.
//!
//! - **A6**: Does the broker trust gate check the CALLER or BROKER binary?
//!   Code-read answer (see SUMMARY.md): the gate checks `nono.exe` (the CALLER)
//!   via `current_exe()`, then verifies that `nono-shell-broker.exe` (the BROKER)
//!   has a MATCHING Authenticode signature. Both must be from the same build.
//!   A `nono-agentd.exe` calling the broker arm would need its OWN path to satisfy
//!   `is_dev_build_layout()` or be Authenticode-signed matching the broker.

#![cfg(target_os = "windows")]
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;
use std::os::windows::ffi::OsStrExt;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Serialization mutex — prevents parallel test threads from perturbing each
// other's GetProcessHandleCount and NtQuerySystemInformation snapshots.
//
// Handle counts are process-global; sibling tests running concurrently churn
// handles between SC3's cold/post-warmup snapshots, producing negative per-type
// deltas (mathematically impossible for a single consistent snapshot pair).
// Acquiring this mutex at the top of every test guarantees one test at a time,
// so all handle-count and handle-type measurements are internally consistent.
// ---------------------------------------------------------------------------
static SERIAL: Mutex<()> = Mutex::new(());

use nono::AppContainerProfile;
use nono::{derive_app_container_sid, package_sid_to_string};

use windows_sys::Win32::Foundation::{CloseHandle, FreeLibrary, BOOL, HANDLE, HMODULE};
use windows_sys::Win32::Security::{
    DuplicateTokenEx, ImpersonateLoggedOnUser, RevertToSelf, SecurityImpersonation,
    TokenImpersonation, TOKEN_ALL_ACCESS, TOKEN_DUPLICATE, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetProcessHandleCount, OpenProcessToken, INFINITE,
};

// ---------------------------------------------------------------------------
// Integration-test gate
// ---------------------------------------------------------------------------

/// Returns `true` when the `NONO_DAEMON_INTEGRATION_TESTS=1` env-var is set.
/// Tests that require a real Win11 host with dev-layout nono.exe gate on this.
fn daemon_integration_tests_enabled() -> bool {
    std::env::var("NONO_DAEMON_INTEGRATION_TESTS").as_deref() == Ok("1")
}

/// Early-return from a test body if the integration gate is not set.
/// The test is skipped (pass by early-return), not failed.
macro_rules! require_integration {
    () => {
        if !daemon_integration_tests_enabled() {
            eprintln!(
                "SKIP: set NONO_DAEMON_INTEGRATION_TESTS=1 to run daemon spike tests \
                 (requires real Win11 host with dev-layout nono.exe)"
            );
            return;
        }
    };
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Returns the current process's open handle count via `GetProcessHandleCount`.
/// Used to verify deterministic reap in `n_agents_over_time_returns_to_baseline_handle_count`.
fn get_process_handle_count() -> u32 {
    let mut count: u32 = 0;
    let ok = unsafe {
        // SAFETY: GetCurrentProcess() returns a pseudo-handle (-1) for the current
        // process that is always valid; &mut count is a valid out-pointer.
        GetProcessHandleCount(GetCurrentProcess(), &mut count)
    };
    assert_ne!(
        ok,
        0,
        "GetProcessHandleCount failed: {}",
        std::io::Error::last_os_error()
    );
    count
}

/// Builds a unique AppContainer profile name for the test using a counter and PID
/// to avoid name collisions between parallel test runs.
fn unique_profile_name(tag: &str, idx: usize) -> String {
    format!("nono.spike74.{}.{}.{}", tag, std::process::id(), idx)
}

// ---------------------------------------------------------------------------
// Handle-type characterization helper (SC3 diagnostic)
//
// Uses NtQuerySystemInformation (undocumented NT API, loaded dynamically from
// ntdll.dll) to enumerate all handles in the current process and group them by
// object type index. The type index is a kernel-assigned integer; we map it to
// a human-readable name via NtQueryObject(ObjectTypeInformation) on the first
// handle of each type we see in our process.
//
// This is test-only diagnostic code; unwrap and raw FFI are intentional.
// ---------------------------------------------------------------------------

/// A per-type handle count snapshot: maps (type_index → (type_name, count)).
type HandleTypeMap = std::collections::BTreeMap<u8, (String, u32)>;

/// C-layout of the SYSTEM_HANDLE_TABLE_ENTRY_INFO struct returned by
/// NtQuerySystemInformation(SystemHandleInformation).
/// Reference: https://www.geoffchappell.com/studies/windows/km/ntoskrnl/api/ex/sysinfo/handle_table_entry.htm
#[repr(C)]
struct SystemHandleTableEntryInfo {
    unique_process_id: u16,
    creator_back_trace_index: u16,
    object_type_index: u8,
    handle_attributes: u8,
    handle_value: u16,
    object: usize,
    granted_access: u32,
}

/// Snapshot handle counts grouped by object type for the current process.
///
/// Returns an empty map if NtQuerySystemInformation is unavailable (e.g. not
/// on Windows). Errors are swallowed — this is diagnostic-only; test logic
/// does not fail on enumeration failure, it just prints less detail.
fn snapshot_handle_types_for_current_pid() -> HandleTypeMap {
    use std::os::raw::c_void;

    type NtQuerySystemInformationFn = unsafe extern "system" fn(
        u32, // SystemInformationClass
        *mut c_void,
        u32,
        *mut u32,
    ) -> i32;

    // SystemHandleInformation = 16 (0x10)
    const SYSTEM_HANDLE_INFORMATION_CLASS: u32 = 16;

    let current_pid = std::process::id() as u16;

    // Load ntdll.dll and get NtQuerySystemInformation.
    let ntdll_name = "ntdll.dll\0";
    let ntdll: HMODULE = unsafe {
        // SAFETY: ntdll_name is a valid nul-terminated ASCII string.
        windows_sys::Win32::System::LibraryLoader::LoadLibraryA(ntdll_name.as_ptr())
    };
    if ntdll.is_null() {
        eprintln!(
            "[spike74][characterize] LoadLibraryA(ntdll) failed — skipping handle-type breakdown"
        );
        return HandleTypeMap::new();
    }
    let nt_query_fn_name = "NtQuerySystemInformation\0";
    let fn_ptr = unsafe {
        // SAFETY: ntdll is a valid module handle; fn name is nul-terminated ASCII.
        windows_sys::Win32::System::LibraryLoader::GetProcAddress(ntdll, nt_query_fn_name.as_ptr())
    };
    let Some(fn_ptr) = fn_ptr else {
        unsafe { FreeLibrary(ntdll) };
        eprintln!(
            "[spike74][characterize] GetProcAddress(NtQuerySystemInformation) failed — skipping"
        );
        return HandleTypeMap::new();
    };
    let nt_query: NtQuerySystemInformationFn = unsafe {
        // SAFETY: fn_ptr is NtQuerySystemInformation from ntdll.
        std::mem::transmute(fn_ptr)
    };

    // Probe the required buffer size, then allocate and call for real.
    // SystemHandleInformation can require large buffers on busy systems (many handles).
    // We start at 4 MiB and double on STATUS_INFO_LENGTH_MISMATCH (0xC0000004).
    let mut buf_size: u32 = 4 << 20; // 4 MiB initial
    let mut buf: Vec<u8>;
    let mut retry = 0usize;
    loop {
        buf = vec![0u8; buf_size as usize];
        let mut returned: u32 = 0;
        let s = unsafe {
            // SAFETY: buf is valid writable memory of `buf_size` bytes.
            nt_query(
                SYSTEM_HANDLE_INFORMATION_CLASS,
                buf.as_mut_ptr() as *mut c_void,
                buf_size,
                &mut returned,
            )
        };
        if s == 0 {
            break; // STATUS_SUCCESS
        }
        // STATUS_INFO_LENGTH_MISMATCH = 0xC0000004 — buffer too small; grow and retry.
        if s == 0xC000_0004u32 as i32 {
            retry += 1;
            if retry > 8 {
                eprintln!(
                    "[spike74][characterize] NtQuerySystemInformation: too many retries — skipping"
                );
                unsafe { FreeLibrary(ntdll) };
                return HandleTypeMap::new();
            }
            // Use returned size as hint if non-zero, else double.
            buf_size = if returned > buf_size {
                returned.saturating_add(65536) // add a bit extra
            } else {
                buf_size.saturating_mul(2)
            };
            continue;
        }
        // Other error — give up gracefully.
        eprintln!("[spike74][characterize] NtQuerySystemInformation failed (0x{s:08X}) — skipping");
        unsafe { FreeLibrary(ntdll) };
        return HandleTypeMap::new();
    }

    // Parse the result buffer into a type-count map for our PID.
    // SYSTEM_HANDLE_TABLE_ENTRY_INFO layout (64-bit x86):
    //   offset 0: UniqueProcessId     u16
    //   offset 2: CreatorBackTraceIndex u16
    //   offset 4: ObjectTypeIndex     u8
    //   offset 5: HandleAttributes    u8
    //   offset 6: HandleValue         u16
    //   offset 8: Object              usize (8 bytes on 64-bit; aligned to 8)
    //   offset 16: GrantedAccess      u32
    //   offset 20: (padding)          u32
    //   total:                        24 bytes (on 64-bit)
    //
    // We read directly from the byte buffer using `read_unaligned` to avoid
    // alignment UB — the Vec<u8> is heap-allocated and may not satisfy usize
    // alignment requirements for the pointer-size field.
    let entry_stride = std::mem::size_of::<SystemHandleTableEntryInfo>(); // 24 on x64
    if buf.len() < 8 {
        unsafe { FreeLibrary(ntdll) };
        return HandleTypeMap::new();
    }
    let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    // SYSTEM_HANDLE_INFORMATION layout on x64:
    //   offset 0: NumberOfHandles ULONG  (4 bytes)
    //   offset 4: _padding_               (4 bytes, aligns the Handles[] array
    //              which contains pointer-sized fields to 8-byte boundary)
    //   offset 8: Handles[] SYSTEM_HANDLE_TABLE_ENTRY_INFO[count]
    //
    // NOTE: On 32-bit, entries_start would be 4 (no alignment gap), but this
    // test only runs on 64-bit Windows hosts.
    let entries_start: usize = if cfg!(target_pointer_width = "64") {
        8
    } else {
        4
    };
    // Sanity check: count * stride must fit in the buffer.
    if count.saturating_mul(entry_stride) > buf.len().saturating_sub(entries_start) {
        eprintln!(
            "[spike74][characterize] handle buffer sanity failed: count={count} stride={entry_stride} \
             buf_len={} — skipping",
            buf.len()
        );
        unsafe { FreeLibrary(ntdll) };
        return HandleTypeMap::new();
    }

    let mut type_map: HandleTypeMap = HandleTypeMap::new();
    for i in 0..count {
        let entry_offset = entries_start + i * entry_stride;
        let entry_bytes = &buf[entry_offset..entry_offset + entry_stride];

        // Read fields using little-endian byte reads to avoid alignment issues.
        let pid_u16 = u16::from_le_bytes([entry_bytes[0], entry_bytes[1]]);
        if pid_u16 != current_pid {
            continue;
        }
        let object_type_index = entry_bytes[4]; // offset 4
        let handle_value_u16 = u16::from_le_bytes([entry_bytes[6], entry_bytes[7]]); // offset 6

        let type_idx = object_type_index;
        let counter = type_map.entry(type_idx).or_insert_with(|| {
            // Try to resolve the type name via NtQueryObject on the first handle
            // of this type we see.  This is best-effort: if the handle is not
            // query-able (e.g. requires special access) we fall back to the index.
            let name = query_object_type_name(handle_value_u16 as HANDLE)
                .unwrap_or_else(|| format!("<type-{type_idx}>"));
            (name, 0u32)
        });
        counter.1 = counter.1.saturating_add(1);
    }

    unsafe { FreeLibrary(ntdll) };
    type_map
}

/// Attempt to resolve a handle's object-type name via `NtQueryObject`.
/// Returns `None` if the call fails or the name is empty/not UTF-16.
fn query_object_type_name(handle: HANDLE) -> Option<String> {
    use std::os::raw::c_void;

    type NtQueryObjectFn =
        unsafe extern "system" fn(HANDLE, u32, *mut c_void, u32, *mut u32) -> i32;

    // ObjectTypeInformation = 2
    const OBJECT_TYPE_INFORMATION_CLASS: u32 = 2;

    let ntdll_name = "ntdll.dll\0";
    let ntdll: HMODULE = unsafe {
        // SAFETY: nul-terminated ASCII name.
        windows_sys::Win32::System::LibraryLoader::LoadLibraryA(ntdll_name.as_ptr())
    };
    if ntdll.is_null() {
        return None;
    }
    let fn_name = "NtQueryObject\0";
    let fn_ptr = unsafe {
        windows_sys::Win32::System::LibraryLoader::GetProcAddress(ntdll, fn_name.as_ptr())
    };
    if fn_ptr.is_none() {
        unsafe { FreeLibrary(ntdll) };
        return None;
    }
    let fn_ptr = fn_ptr.unwrap();
    let nt_query_obj: NtQueryObjectFn = unsafe { std::mem::transmute(fn_ptr) };

    // OBJECT_TYPE_INFORMATION starts with a UNICODE_STRING (Length, MaxLength, Buffer*).
    // We allocate 512 bytes which is always enough for a type name.
    let mut buf = vec![0u8; 512];
    let mut returned: u32 = 0;
    let s = unsafe {
        // SAFETY: buf is valid writable memory; handle may not be query-able (fail-safe).
        nt_query_obj(
            handle,
            OBJECT_TYPE_INFORMATION_CLASS,
            buf.as_mut_ptr() as *mut c_void,
            buf.len() as u32,
            &mut returned,
        )
    };
    unsafe { FreeLibrary(ntdll) };

    if s != 0 {
        return None;
    }

    // OBJECT_TYPE_INFORMATION starts with a UNICODE_STRING for TypeName:
    //   offset  0: Length      (u16) — byte length of the string (not including NUL)
    //   offset  2: MaxLength   (u16)
    //   offset  4: _padding_   (4 bytes on x64, 0 on x86)
    //   offset  8: Buffer      (*u16) — absolute virtual address of string data in buf
    //                                   (NtQueryObject puts string data inline after struct)
    //
    // On x64 the UNICODE_STRING itself is 16 bytes; on x86 it is 8 bytes.
    // The string data follows immediately at offset `us_size` in the output buffer.
    //
    // We derive the inline string offset from the Buffer pointer: since NtQueryObject
    // writes the struct and string data contiguously into our `buf`, the string data
    // is at `buf_ptr + offset_of_buffer_field_value - (buf_base_ptr)`.
    // A simpler approach: compute offset as `buf[data_ptr_field] - &buf[0]` using
    // the absolute pointer value. We read the ptr from the buf directly.
    if (returned as usize) < 16 {
        return None;
    }
    let length = u16::from_le_bytes([buf[0], buf[1]]) as usize;
    if length == 0 || (length & 1) != 0 {
        return None;
    }
    let char_count = length / 2;

    // Read the Buffer absolute pointer from the UNICODE_STRING.
    // On x64: offset 8, 8 bytes little-endian. On x86: offset 4, 4 bytes.
    let (buf_ptr_offset, ptr_size) = if cfg!(target_pointer_width = "64") {
        (8usize, 8usize)
    } else {
        (4usize, 4usize)
    };
    if buf.len() < buf_ptr_offset + ptr_size {
        return None;
    }
    // Read the absolute virtual-address pointer value from the buffer.
    let abs_ptr: usize = if ptr_size == 8 {
        let bytes = [
            buf[buf_ptr_offset],
            buf[buf_ptr_offset + 1],
            buf[buf_ptr_offset + 2],
            buf[buf_ptr_offset + 3],
            buf[buf_ptr_offset + 4],
            buf[buf_ptr_offset + 5],
            buf[buf_ptr_offset + 6],
            buf[buf_ptr_offset + 7],
        ];
        usize::from_le_bytes(bytes)
    } else {
        let bytes = [
            buf[buf_ptr_offset],
            buf[buf_ptr_offset + 1],
            buf[buf_ptr_offset + 2],
            buf[buf_ptr_offset + 3],
        ];
        u32::from_le_bytes(bytes) as usize
    };
    // The base address of our output buffer in process memory.
    let buf_base = buf.as_ptr() as usize;
    // Compute the offset of the string data within our buffer.
    if abs_ptr < buf_base {
        return None; // pointer points before our buffer — something is wrong
    }
    let data_offset = abs_ptr - buf_base;
    if data_offset.saturating_add(length) > buf.len() {
        return None; // string data extends beyond our buffer
    }

    let u16_slice: Vec<u16> = buf[data_offset..]
        .chunks_exact(2)
        .take(char_count)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    String::from_utf16(&u16_slice).ok()
}

/// Print per-type handle delta between two snapshots (before → after).
/// Used in SC3 to attribute the one-time warmup cost to concrete types.
fn print_handle_type_delta(before: &HandleTypeMap, after: &HandleTypeMap, label: &str) {
    // Collect union of all type indices seen in either snapshot.
    let mut indices: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
    indices.extend(before.keys().copied());
    indices.extend(after.keys().copied());

    eprintln!("[spike74][characterize] {label}:");
    for idx in &indices {
        let (b_name, b_count) = before
            .get(idx)
            .map(|(n, c)| (n.as_str(), *c))
            .unwrap_or(("", 0));
        let (a_name, a_count) = after
            .get(idx)
            .map(|(n, c)| (n.as_str(), *c))
            .unwrap_or(("", 0));
        let name = if !a_name.is_empty() { a_name } else { b_name };
        let delta = a_count as i64 - b_count as i64;
        if delta != 0 {
            eprintln!(
                "  type-{idx:3} ({name:20}): before={b_count:4}  after={a_count:4}  delta={delta:+5}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test 1: fresh_token_isolation_agents_have_distinct_package_sids
// ---------------------------------------------------------------------------

/// **SC3 / DMON-01 spike clause 1 — Fresh-token isolation.**
///
/// Mints 10 AppContainer profiles (simulating the per-agent `CreateAppContainerProfile`
/// call the daemon will make for each tenant), derives each profile's package SID, and
/// asserts all 10 SIDs are distinct.
///
/// This proves: (a) `create_app_container_profile` with distinct names yields distinct
/// package SIDs, and (b) the nono lib's SID-derivation path (`derive_app_container_sid`
/// + `package_sid_to_string`) produces the correct string form per profile.
///
/// If any two profiles share a SID, the daemon's per-tenant isolation collapses
/// (Pitfall 2 in 74-RESEARCH.md: token reuse → cross-tenant capability theft).
#[test]
fn fresh_token_isolation_agents_have_distinct_package_sids() {
    require_integration!();
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    const AGENT_COUNT: usize = 10;
    let mut sids: HashSet<String> = HashSet::new();
    let mut profiles: Vec<AppContainerProfile> = Vec::with_capacity(AGENT_COUNT);

    for i in 0..AGENT_COUNT {
        let name = unique_profile_name("sids", i);

        // Create the AppContainer profile — mirrors the daemon's per-agent mint.
        let profile = nono::create_app_container_profile(&name)
            .unwrap_or_else(|e| panic!("create_app_container_profile({name:?}) failed: {e}"));

        // Derive the package SID from the profile name.
        let owned_sid = derive_app_container_sid(&name)
            .unwrap_or_else(|e| panic!("derive_app_container_sid({name:?}) failed: {e}"));

        // Convert to SDDL string form (the daemon's per-tenant key).
        let sid_str = package_sid_to_string(&owned_sid)
            .unwrap_or_else(|e| panic!("package_sid_to_string failed for {name:?}: {e}"));

        eprintln!("[spike74][sids][{i}] profile={name} sid={sid_str}");

        let inserted = sids.insert(sid_str.clone());
        assert!(
            inserted,
            "Duplicate package SID detected at index {i}: {sid_str}\n\
             This means create_app_container_profile returned the SAME SID for two \
             different profile names — token-isolation would collapse in the daemon."
        );

        // Keep the profile alive until the assertion above; then collect for drop.
        profiles.push(profile);
    }

    // Drop all profiles (triggers DeleteAppContainerProfile for each).
    // After drop, the registry entries are cleaned up — same as AgentTenant::Drop.
    drop(profiles);

    assert_eq!(
        sids.len(),
        AGENT_COUNT,
        "Expected {AGENT_COUNT} distinct package SIDs, got {}",
        sids.len()
    );

    eprintln!("[spike74][sids] PASS: {AGENT_COUNT} profiles produced {AGENT_COUNT} distinct SIDs");
}

// ---------------------------------------------------------------------------
// Test 2: n_agents_over_time_returns_to_baseline_handle_count
// ---------------------------------------------------------------------------

/// **SC3 / DMON-01 spike clause 2 — Deterministic reap (handle baseline).**
///
/// Records the process handle count before any cycles, runs 100 agent-lifecycle
/// cycles (create AppContainer profile → derive SID → drop profile), then asserts
/// the steady-state growth (from the post-warmup plateau) is ≤ EPSILON_STEADY.
///
/// # Why a post-warmup baseline instead of a cold baseline
///
/// The first call to `CreateAppContainerProfile` triggers one-time OS-side RPC
/// binding and threadpool initialization (~65 handles on Win11). These handles
/// are NOT leaked per-cycle — they are a one-time cost for the `AppX Deployment
/// Service` RPC channel. The flat plateau observed at cycles 25/50/75/100
/// (all 138, zero net per-cycle growth) proves this.
///
/// To guard against real per-cycle leaks while tolerating benign warmup, this test:
/// 1. Takes a COLD handle-type snapshot (before any warmup cycles).
/// 2. Runs WARMUP_CYCLES first-pass cycles (establishes the plateau).
/// 3. Records a `post_warmup` handle count (the plateau level).
/// 4. Takes a POST-WARMUP handle-type snapshot (the plateau types).
/// 5. Prints the WARMUP per-type delta (cold → post-warmup) — names the ~65 one-time handles.
/// 6. Runs the remaining (TOTAL_CYCLES - WARMUP_CYCLES) steady-state cycles.
/// 7. Takes a POST-FULL-RUN handle-type snapshot.
/// 8. Prints the STEADY-STATE per-type delta (post-warmup → post-full-run); warns on suspect types.
/// 9. Asserts `post_full_run ≤ post_warmup + EPSILON_STEADY` (per-cycle growth must be ~0).
///
/// A real Token/File/Job handle leak (e.g., `DeleteAppContainerProfile` not called)
/// accumulates 200+ handles over 100 cycles and is immediately visible via the
/// steady-state assertion AND the per-type characterization printout.
#[test]
fn n_agents_over_time_returns_to_baseline_handle_count() {
    require_integration!();
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    const TOTAL_CYCLES: usize = 100;
    const WARMUP_CYCLES: usize = 10;
    // Steady-state epsilon: max acceptable per-cycle growth in handles
    // after the warmup plateau is established.
    const EPSILON_STEADY: u32 = 5;

    let cold_baseline = get_process_handle_count();
    eprintln!("[spike74][handles] cold baseline handle count: {cold_baseline}");

    // ---- COLD handle-type snapshot — taken BEFORE the FIRST warmup cycle ----
    //
    // Strategy: snapshot handle types immediately before AND after the VERY FIRST
    // `create_app_container_profile` call in this test's warmup loop.  That first
    // call is when the OS initialises the AppX Deployment Service RPC channel and
    // its associated threadpool — the source of the ~65 one-time handles.
    //
    // NOTE: In a parallel test run (default Rust test runner), sibling tests in
    // this binary may have already called `CreateAppContainerProfile` before this
    // test body starts, pre-paying the warmup cost.  In that case `cold_baseline`
    // will already be near the plateau (~142) and the first-cycle delta will be
    // small (< 10).  The cumulative type breakdown (all warmup cycles combined)
    // still characterises the sustained types at the plateau.
    //
    // Run with `-- --test-threads=1` for guaranteed ordering if you need a pristine
    // cold characterisation from a truly unwarmed process state.
    let cold_types = snapshot_handle_types_for_current_pid();
    eprintln!(
        "[spike74][characterize] cold handle-type snapshot taken \
         (cold_baseline={cold_baseline}, {} distinct type indices)",
        cold_types.len()
    );

    // ---- Warmup phase: pay one-time OS RPC/threadpool init costs ----
    //
    // We snapshot handle types around the FIRST cycle to capture any single-call
    // delta.  Subsequent cycles should show zero type growth (steady state).
    let mut first_cycle_pre: Option<HandleTypeMap> = None;
    let mut first_cycle_post: Option<HandleTypeMap> = None;

    for i in 0..WARMUP_CYCLES {
        // Capture pre-first-cycle snapshot.
        if i == 0 {
            first_cycle_pre = Some(snapshot_handle_types_for_current_pid());
        }

        let name = unique_profile_name("handles-warm", i);
        let profile = nono::create_app_container_profile(&name)
            .unwrap_or_else(|e| panic!("create_app_container_profile failed at warmup {i}: {e}"));
        let owned_sid = derive_app_container_sid(&name)
            .unwrap_or_else(|e| panic!("derive_app_container_sid failed at warmup {i}: {e}"));
        let _sid_str = package_sid_to_string(&owned_sid)
            .unwrap_or_else(|e| panic!("package_sid_to_string failed at warmup {i}: {e}"));
        drop(owned_sid);
        drop(profile);

        // Capture post-first-cycle snapshot (after profile is dropped so we see
        // the net handle delta, not just the transient create-then-delete spike).
        if i == 0 {
            first_cycle_post = Some(snapshot_handle_types_for_current_pid());
        }
    }

    let post_warmup = get_process_handle_count();
    let warmup_delta = post_warmup.saturating_sub(cold_baseline);
    eprintln!(
        "[spike74][handles] post-warmup handle count: {post_warmup} \
         (one-time delta={warmup_delta})"
    );

    // ---- Characterize: snapshot handle types at the post-warmup plateau ----
    // This snapshot is taken immediately after warmup (before steady-state cycles).
    let before_snapshot = snapshot_handle_types_for_current_pid();

    // ---- Print the WARMUP per-type delta: COLD → POST-WARMUP (whole warmup phase) ----
    // This names the net one-time warmup handles by kernel object type using the
    // full warmup window (cold_types → before_snapshot).  When sibling tests have
    // pre-paid the RPC warmup, this delta will be small/zero and a note is printed.
    {
        let mut indices: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
        indices.extend(cold_types.keys().copied());
        indices.extend(before_snapshot.keys().copied());

        let mut warmup_deltas: Vec<(i64, u8, String)> = indices
            .iter()
            .filter_map(|idx| {
                let cold_count = cold_types.get(idx).map(|(_, c)| *c).unwrap_or(0);
                let warm_count = before_snapshot.get(idx).map(|(_, c)| *c).unwrap_or(0);
                let delta = warm_count as i64 - cold_count as i64;
                if delta == 0 {
                    return None;
                }
                let name = before_snapshot
                    .get(idx)
                    .map(|(n, _)| n.as_str())
                    .filter(|n| !n.is_empty())
                    .or_else(|| {
                        cold_types
                            .get(idx)
                            .map(|(n, _)| n.as_str())
                            .filter(|n| !n.is_empty())
                    })
                    .map(|s| s.to_owned())
                    .unwrap_or_else(|| format!("TypeIndex#{idx}"));
                Some((delta, *idx, name))
            })
            .collect();
        warmup_deltas.sort_by(|a, b| b.0.abs().cmp(&a.0.abs()).then(a.1.cmp(&b.1)));

        let total_warmup: i64 = warmup_deltas.iter().map(|(d, _, _)| d).sum();
        eprintln!(
            "[spike74][characterize] WARMUP per-type handle delta \
             (cold -> post-warmup, total={total_warmup:+}):"
        );
        if warmup_deltas.is_empty() {
            // Zero delta here means sibling tests pre-paid the warmup cost before this
            // test body started.  The first-cycle breakdown below captures what was added
            // within this test's own warmup loop.
            eprintln!(
                "[spike74][characterize]   (delta=0 — warmup likely pre-paid by sibling tests; \
                 see first-cycle breakdown below)"
            );
        } else {
            for (delta, _idx, name) in &warmup_deltas {
                eprintln!("[spike74][characterize]   {name}: {delta:+}");
            }
        }

        // ---- Soft consistency check: per-type sum must match the absolute delta from the
        // same NtQuerySystemInformation snapshots (cold_types → before_snapshot totals).
        // Both sides derive from the same API call pair, so they MUST agree algebraically.
        // A divergence here means the snapshots were taken in a split state (OS bug or
        // future code regression — should not happen with the SERIAL mutex held).
        let cold_total: i64 = cold_types.values().map(|(_, c)| *c as i64).sum();
        let warm_total: i64 = before_snapshot.values().map(|(_, c)| *c as i64).sum();
        let abs_from_snapshots: i64 = warm_total - cold_total;
        if total_warmup != abs_from_snapshots {
            eprintln!(
                "[spike74][characterize] WARN: per-type sum {total_warmup:+} != absolute {abs_from_snapshots:+} \
                 — snapshot inconsistency (the two NtQuerySystemInformation snapshots are internally \
                 inconsistent; run alone or with --test-threads=1 for a clean characterization)"
            );
        }
    }

    // ---- Print the FIRST-CYCLE per-type delta ----
    // This is the tightest measurement window: types added by the FIRST
    // create→derive→drop cycle in THIS test (before and after that cycle).
    // If the OS RPC channel was cold entering this cycle, this will show the full
    // ~65-handle warmup.  If it was already warm (parallel run), this shows the
    // per-cycle residual (expected: 0 or near-0, confirming no per-cycle leak).
    if let (Some(pre), Some(post)) = (first_cycle_pre, first_cycle_post) {
        let mut indices: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
        indices.extend(pre.keys().copied());
        indices.extend(post.keys().copied());

        let mut first_deltas: Vec<(i64, u8, String)> = indices
            .iter()
            .filter_map(|idx| {
                let pre_count = pre.get(idx).map(|(_, c)| *c).unwrap_or(0);
                let post_count = post.get(idx).map(|(_, c)| *c).unwrap_or(0);
                let delta = post_count as i64 - pre_count as i64;
                if delta == 0 {
                    return None;
                }
                let name = post
                    .get(idx)
                    .map(|(n, _)| n.as_str())
                    .filter(|n| !n.is_empty())
                    .or_else(|| {
                        pre.get(idx)
                            .map(|(n, _)| n.as_str())
                            .filter(|n| !n.is_empty())
                    })
                    .map(|s| s.to_owned())
                    .unwrap_or_else(|| format!("TypeIndex#{idx}"));
                Some((delta, *idx, name))
            })
            .collect();
        first_deltas.sort_by(|a, b| b.0.abs().cmp(&a.0.abs()).then(a.1.cmp(&b.1)));

        let first_total: i64 = first_deltas.iter().map(|(d, _, _)| d).sum();
        eprintln!(
            "[spike74][characterize] WARMUP first-cycle handle delta \
             (pre-cycle-0 -> post-cycle-0-drop, total={first_total:+}):"
        );
        if first_deltas.is_empty() {
            eprintln!(
                "[spike74][characterize]   (delta=0 after first cycle drop — \
                 OS RPC channel was already warmed before cycle 0; \
                 use --test-threads=1 for cold-process characterisation)"
            );
        } else {
            for (delta, _idx, name) in &first_deltas {
                eprintln!("[spike74][characterize]   {name}: {delta:+}");
            }
        }
    }

    // ---- Main cycles: assert no per-cycle growth ----
    for i in 0..(TOTAL_CYCLES - WARMUP_CYCLES) {
        let name = unique_profile_name("handles", i);

        // Simulate one agent's profile lifetime:
        // 1. Create profile (analogous to daemon's CreateAppContainerProfile call)
        let profile = nono::create_app_container_profile(&name).unwrap_or_else(|e| {
            panic!("create_app_container_profile({name:?}) failed at cycle {i}: {e}")
        });

        // 2. Derive the SID (analogous to the per-tenant key mint)
        let owned_sid = derive_app_container_sid(&name)
            .unwrap_or_else(|e| panic!("derive_app_container_sid failed at cycle {i}: {e}"));

        // 3. Convert SID to string (analogous to registry insert)
        let _sid_str = package_sid_to_string(&owned_sid)
            .unwrap_or_else(|e| panic!("package_sid_to_string failed at cycle {i}: {e}"));

        // 4. Drop: triggers AppContainerProfile::Drop → DeleteAppContainerProfile.
        //    Also drops OwnedAppContainerSid → FreeSid.
        //    This is the path under test: does Drop correctly clean up?
        drop(owned_sid);
        drop(profile);

        // Periodic progress log for long-running baseline tests.
        if i % 25 == 24 {
            let mid = get_process_handle_count();
            eprintln!(
                "[spike74][handles] after cycle {}: handle count = {mid} \
                 (plateau-delta={})",
                i + WARMUP_CYCLES + 1,
                mid.saturating_sub(post_warmup)
            );
        }
    }

    let post_full_run = get_process_handle_count();
    // ---- Characterize: snapshot after full run and print per-type deltas ----
    let after_snapshot = snapshot_handle_types_for_current_pid();

    // ---- Print the STEADY-STATE per-type delta (post-warmup plateau → post-full-run) ----
    // Empty output here means zero per-cycle handle growth — the ideal case.
    {
        let steady_total = post_full_run as i64 - post_warmup as i64;
        eprintln!(
            "[spike74][characterize] steady-state per-type delta \
             (post-warmup -> post-{TOTAL_CYCLES}) (empty => no per-cycle leak):"
        );

        let mut indices: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
        indices.extend(before_snapshot.keys().copied());
        indices.extend(after_snapshot.keys().copied());

        let mut steady_deltas: Vec<(i64, u8, String)> = indices
            .iter()
            .filter_map(|idx| {
                let pre_count = before_snapshot.get(idx).map(|(_, c)| *c).unwrap_or(0);
                let post_count = after_snapshot.get(idx).map(|(_, c)| *c).unwrap_or(0);
                let delta = post_count as i64 - pre_count as i64;
                if delta == 0 {
                    return None;
                }
                let name = after_snapshot
                    .get(idx)
                    .map(|(n, _)| n.as_str())
                    .filter(|n| !n.is_empty())
                    .or_else(|| {
                        before_snapshot
                            .get(idx)
                            .map(|(n, _)| n.as_str())
                            .filter(|n| !n.is_empty())
                    })
                    .map(|s| s.to_owned())
                    .unwrap_or_else(|| format!("TypeIndex#{idx}"));
                Some((delta, *idx, name))
            })
            .collect();
        steady_deltas.sort_by(|a, b| b.0.abs().cmp(&a.0.abs()).then(a.1.cmp(&b.1)));

        if steady_deltas.is_empty() {
            eprintln!("[spike74][characterize]   (none — steady-state total={steady_total:+})");
        } else {
            for (delta, _idx, name) in &steady_deltas {
                eprintln!("[spike74][characterize]   {name}: {delta:+}");
            }

            // Soft diagnostic: warn if any security-critical handle types are growing
            // in the steady-state window. These types must NOT leak per-cycle.
            // The hard assertion below catches the total; this names the culprit.
            const SUSPECT_TYPES: &[&str] = &["Token", "File", "Job", "Section", "ALPC Port", "Key"];
            for (delta, _idx, name) in &steady_deltas {
                if *delta > 0 && SUSPECT_TYPES.iter().any(|s| name.eq_ignore_ascii_case(s)) {
                    eprintln!(
                        "[spike74][characterize] WARN: suspect per-cycle growth of \
                         security-critical handle type '{name}': +{delta} over \
                         {} steady-state cycles — check DeleteAppContainerProfile/FreeSid",
                        TOTAL_CYCLES - WARMUP_CYCLES
                    );
                }
            }
        }
        // Also emit the legacy tabular view for cross-reference.
        print_handle_type_delta(
            &before_snapshot,
            &after_snapshot,
            &format!(
                "handle-type delta (tabular): post-warmup plateau → post-{TOTAL_CYCLES}-cycles \
                 (delta={steady_total:+})"
            ),
        );
    }

    eprintln!(
        "[spike74][handles] post-full-run handle count: {post_full_run} \
         (cold_baseline={cold_baseline}, post_warmup={post_warmup}, \
         one-time-warmup-cost={}, steady-state-delta={})",
        post_warmup.saturating_sub(cold_baseline),
        post_full_run.saturating_sub(post_warmup)
    );

    // Steady-state assertion: from the plateau, the remaining 90 cycles must
    // add ≤ EPSILON_STEADY handles. Zero per-cycle growth is expected.
    assert!(
        post_full_run <= post_warmup + EPSILON_STEADY,
        "Steady-state handle leak detected after {TOTAL_CYCLES} agent-lifecycle cycles.\n\
         cold_baseline={cold_baseline}  post_warmup={post_warmup}  \
         post_full_run={post_full_run}  steady_delta={}\n\
         The plateau-level increased from the post-warmup baseline — this indicates \
         a per-cycle handle leak in the AppContainerProfile or OwnedAppContainerSid \
         Drop path. The per-type characterization above shows which kernel object \
         types are growing. Check that DeleteAppContainerProfile and FreeSid are called.\n\
         (See 74-RESEARCH.md §Deterministic Reap §Known leak vectors)",
        post_full_run.saturating_sub(post_warmup)
    );

    eprintln!(
        "[spike74][handles] PASS: {TOTAL_CYCLES} cycles — one-time warmup cost={} \
         handles (expected: RPC/threadpool infra, confirmed by type characterization above); \
         steady-state delta={} (target: ≤ {EPSILON_STEADY})",
        post_warmup.saturating_sub(cold_baseline),
        post_full_run.saturating_sub(post_warmup)
    );
}

// ---------------------------------------------------------------------------
// Test 3: daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance
// ---------------------------------------------------------------------------

/// **SC2 / DMON-02 spike clause 3 — Cross-tenant pipe denial via REAL AppContainer B token.**
///
/// Verifies that a `SupervisorSocket` pipe instance created with tenant A's AppContainer
/// package SID as the only Low-IL-admitting DACL ACE correctly denies a connection
/// attempt from a process running IN tenant B's AppContainer (a distinct package SID).
///
/// # What changed from the first attempt (and why)
///
/// The original implementation used `SetTokenInformation(TokenIntegrityLevel)` to lower
/// a duplicated process token to Low-IL. This fails on Windows 11 with error code 5
/// (`ERROR_ACCESS_DENIED`) because interactive user sessions cannot lower a token below
/// the integrity level of the caller's current token using SetTokenInformation — only
/// `CreateRestrictedToken` or the trusted-installer path can do that without elevated rights.
///
/// The new implementation spawns a **real** AppContainer child process inside tenant B's
/// AppContainer profile (using `CreateProcessW` + `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`).
/// That process runs natively at AppContainer/Low-IL. We then:
/// 1. `OpenProcessToken` on the child → obtain tenant B's real AppContainer token.
/// 2. `DuplicateTokenEx` → create an impersonation-level copy.
/// 3. `ImpersonateLoggedOnUser` with the impersonation token on the test thread.
/// 4. Attempt `SupervisorSocket::connect` to tenant A's pipe.
/// 5. Expected: `ERROR_ACCESS_DENIED` — tenant B's AppContainer SID (`S-1-15-2-<b>`)
///    does NOT appear in the pipe's DACL, which only admits tenant A's SID (`S-1-15-2-<a>`).
///
/// This directly exercises the SDDL DACL gate at the OS level and proves DMON-02 / SC2.
///
/// # On the DACL check for AppContainer tokens
///
/// AppContainer tokens carry a unique package SID in their `Capabilities` list.
/// When the OS evaluates the DACL `(A;;0x0012019F;;;S-1-15-2-<a>)`, it checks if
/// any SID in the token's enabled groups OR `AppContainerSid` field matches. Tenant B's
/// token has AppContainerSid = `S-1-15-2-<b>` — a non-matching SID — so the ACE denies.
#[test]
fn daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance() {
    require_integration!();
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    use windows_sys::Win32::System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_INFORMATION,
    };

    // -----------------------------------------------------------------------
    // Step 1: Create tenant A's AppContainer profile and derive its package SID.
    // -----------------------------------------------------------------------
    let tenant_a_name = unique_profile_name("denial-a", 0);
    let tenant_a_profile = nono::create_app_container_profile(&tenant_a_name)
        .unwrap_or_else(|e| panic!("create_app_container_profile for tenant A failed: {e}"));

    let tenant_a_sid_owned = derive_app_container_sid(&tenant_a_name)
        .unwrap_or_else(|e| panic!("derive_app_container_sid for tenant A failed: {e}"));

    let tenant_a_pkg_sid = package_sid_to_string(&tenant_a_sid_owned)
        .unwrap_or_else(|e| panic!("package_sid_to_string for tenant A failed: {e}"));

    eprintln!("[spike74][denial] tenant A pkg_sid: {tenant_a_pkg_sid}");

    // -----------------------------------------------------------------------
    // Step 2: Bind the capability pipe with ONLY tenant A's pkg SID admitted at Low-IL.
    //         This mirrors what the daemon's accept loop does per tenant.
    //         `bind_low_integrity_with_session_and_package_sid` BLOCKS at
    //         ConnectNamedPipe — we run it on a background thread.
    // -----------------------------------------------------------------------
    let dir = tempfile::tempdir().expect("tempdir for pipe rendezvous");
    let rendezvous = dir.path().join("spike74-denial.rendezvous");
    let rendezvous_for_server = rendezvous.clone();
    let pkg_sid_for_server = tenant_a_pkg_sid.clone();

    let server_thread = std::thread::spawn(move || {
        nono::SupervisorSocket::bind_low_integrity_with_session_and_package_sid(
            &rendezvous_for_server,
            None, // session_sid: None (daemon pipe doesn't use WRITE_RESTRICTED arm)
            Some(&pkg_sid_for_server), // package_sid: tenant A only
        )
    });

    // Wait for the rendezvous file to appear (server writes it synchronously
    // before blocking on ConnectNamedPipe, per bind_impl source).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !rendezvous.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "server thread did not publish rendezvous within 10s at {}",
            rendezvous.display()
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    eprintln!(
        "[spike74][denial] rendezvous published at {}",
        rendezvous.display()
    );

    // -----------------------------------------------------------------------
    // Step 3: Create tenant B's AppContainer profile and derive its package SID.
    //         Tenant B's SID is DISTINCT from tenant A's (distinct profile names).
    // -----------------------------------------------------------------------
    let tenant_b_name = unique_profile_name("denial-b", 0);
    let tenant_b_profile = nono::create_app_container_profile(&tenant_b_name)
        .unwrap_or_else(|e| panic!("create_app_container_profile for tenant B failed: {e}"));

    let tenant_b_sid_owned = derive_app_container_sid(&tenant_b_name)
        .unwrap_or_else(|e| panic!("derive_app_container_sid for tenant B failed: {e}"));

    let tenant_b_pkg_sid = package_sid_to_string(&tenant_b_sid_owned)
        .unwrap_or_else(|e| panic!("package_sid_to_string for tenant B failed: {e}"));

    eprintln!("[spike74][denial] tenant B pkg_sid: {tenant_b_pkg_sid}");
    assert_ne!(
        tenant_a_pkg_sid, tenant_b_pkg_sid,
        "Tenant A and B must have DISTINCT package SIDs for this test to be meaningful"
    );

    // -----------------------------------------------------------------------
    // Step 4: Spawn a REAL AppContainer child process inside tenant B's container.
    //         This gives us a process running at Low-IL with tenant B's AppContainer SID.
    //         The child runs `cmd.exe /c ping -n 30 127.0.0.1` (long-lived so we can
    //         open its token before it exits).
    //
    //         This mirrors the `spawn_appcontainer_child` helper in launch.rs tests.
    // -----------------------------------------------------------------------
    let tenant_b_pi = spawn_appcontainer_child_for_test(tenant_b_sid_owned.as_psid());
    eprintln!(
        "[spike74][denial] spawned AppContainer B child pid={}",
        tenant_b_pi.dwProcessId
    );

    // -----------------------------------------------------------------------
    // Step 5: Open the child's process token and duplicate it to an impersonation token.
    //         This gives us a real AppContainer-B token to impersonate on this thread.
    // -----------------------------------------------------------------------
    // Open the child process with TOKEN query rights.
    let child_process_handle = unsafe {
        // SAFETY: dwProcessId is a valid PID from CreateProcessW.
        OpenProcess(PROCESS_QUERY_INFORMATION, 0, tenant_b_pi.dwProcessId)
    };
    assert!(
        !child_process_handle.is_null(),
        "OpenProcess(QUERY_INFORMATION) on AppContainer B child failed: {}",
        std::io::Error::last_os_error()
    );

    // Open the process's primary token.
    let mut primary_token: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: child_process_handle is valid from OpenProcess above.
        OpenProcessToken(
            child_process_handle,
            TOKEN_QUERY | TOKEN_DUPLICATE,
            &mut primary_token,
        )
    };
    unsafe { CloseHandle(child_process_handle) };
    assert_ne!(
        ok,
        0,
        "OpenProcessToken on AppContainer B child failed: {}",
        std::io::Error::last_os_error()
    );

    // Duplicate to an impersonation-level token.
    let mut imp_token: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: primary_token is a valid process token from OpenProcessToken.
        DuplicateTokenEx(
            primary_token,
            TOKEN_ALL_ACCESS,
            std::ptr::null(),
            SecurityImpersonation,
            TokenImpersonation,
            &mut imp_token,
        )
    };
    unsafe { CloseHandle(primary_token) };
    assert_ne!(
        ok,
        0,
        "DuplicateTokenEx for AppContainer B impersonation token failed: {}",
        std::io::Error::last_os_error()
    );

    eprintln!("[spike74][denial] obtained real AppContainer B impersonation token");

    // -----------------------------------------------------------------------
    // Step 6: Impersonate the AppContainer B token on this thread, then attempt
    //         to connect to tenant A's pipe instance.
    //         Expected: ERROR_ACCESS_DENIED because tenant B's AppContainer SID
    //         is NOT in tenant A's pipe DACL.
    // -----------------------------------------------------------------------
    let impersonate_ok: BOOL = unsafe {
        // SAFETY: imp_token is a valid impersonation token obtained from
        // DuplicateTokenEx above, and lives for this scope.
        ImpersonateLoggedOnUser(imp_token)
    };

    let connect_result = nono::SupervisorSocket::connect(&rendezvous);

    // ALWAYS revert impersonation before asserting — even on error (Pitfall 3).
    let revert_ok: BOOL = unsafe { RevertToSelf() };
    unsafe {
        // SAFETY: imp_token was opened/duplicated above; close it exactly once.
        CloseHandle(imp_token);
    }

    // Clean up the AppContainer B child BEFORE asserting (so child is never leaked).
    unsafe {
        // SAFETY: handles are valid from CreateProcessW; closed exactly once.
        let _ = TerminateProcess(tenant_b_pi.hProcess, 0);
        // Wait for the child to actually exit before closing handles.
        let _ = WaitForSingleObject(tenant_b_pi.hProcess, INFINITE);
        let _ = CloseHandle(tenant_b_pi.hThread);
        let _ = CloseHandle(tenant_b_pi.hProcess);
    }

    // Drop tenant A/B profiles (cleanup).
    drop(tenant_b_sid_owned);
    drop(tenant_b_profile);
    drop(tenant_a_sid_owned);
    drop(tenant_a_profile);

    // -----------------------------------------------------------------------
    // Step 7: Assert outcomes.
    // -----------------------------------------------------------------------
    assert_ne!(
        revert_ok,
        0,
        "RevertToSelf must succeed after impersonation: {}",
        std::io::Error::last_os_error()
    );

    assert_ne!(
        impersonate_ok,
        0,
        "ImpersonateLoggedOnUser with AppContainer B token failed: {}. \
         This means SeImpersonatePrivilege may be absent (A1 assumption check).",
        std::io::Error::last_os_error()
    );

    // The connection as tenant B (real AppContainer token, different pkg SID) must fail.
    assert!(
        connect_result.is_err(),
        "Tenant B (AppContainer SID: {tenant_b_pkg_sid}) MUST NOT be able to connect \
         to tenant A's pipe instance (admits only SID: {tenant_a_pkg_sid}).\n\
         The SDDL ACE for tenant A's pkg SID is the primary gate — if connect_result \
         is OK, the DACL gate failed to deny cross-tenant access.\n\
         This is a load-bearing security regression: DMON-02 / SC2 is broken."
    );

    eprintln!(
        "[spike74][denial] PASS: real AppContainer B token (SID: {tenant_b_pkg_sid}) \
         was correctly denied access to tenant A's pipe instance (A-SID: {tenant_a_pkg_sid}). \
         SDDL DACL gate confirmed at OS level."
    );

    // -----------------------------------------------------------------------
    // Step 8: Release the server thread by connecting as the normal (Medium-IL)
    //         test process so it doesn't hang forever.
    // -----------------------------------------------------------------------
    let _medium_client = nono::SupervisorSocket::connect(&rendezvous);
    let _server = server_thread.join().expect("server thread panicked");
}

// ---------------------------------------------------------------------------
// Test 4: daemon_concurrent_agents
// ---------------------------------------------------------------------------

/// **SC1 / DMON-01 spike clause 4 — Concurrent agents with distinct SIDs.**
///
/// Launches two concurrent "agent sessions" (in separate threads to match the
/// daemon's concurrent accept-loop model), each minting a fresh AppContainer
/// profile and pipe instance. Verifies:
///
/// 1. Two concurrent profile mints produce two DISTINCT package SIDs.
/// 2. Two concurrent pipe instances can be created and are independent (each
///    is tied to its own SID, `PIPE_UNLIMITED_INSTANCES` pattern).
/// 3. Both sessions complete without cross-tenant interference: no deadlock,
///    no shared-SID collision, no pipe-name collision.
///
/// This validates the `PIPE_UNLIMITED_INSTANCES` assumption from 74-RESEARCH.md
/// and the `Arc<Mutex<DaemonState>>` concurrency pattern the daemon will use.
///
/// The spike does NOT require actual confined child processes — it confirms that
/// the underlying OS primitives (concurrent AppContainer profile creation +
/// concurrent named pipe instances) work as expected on the real Win11 host.
/// The human verifier should confirm that both concurrent agent sessions log
/// DISTINCT SIDs (check the `[spike74][concurrent]` log lines).
#[test]
fn daemon_concurrent_agents() {
    require_integration!();
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    const AGENT_COUNT: usize = 2;

    // Shared result collection: (thread_index, pkg_sid_string)
    let results: Arc<Mutex<Vec<(usize, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let dirs: Vec<tempfile::TempDir> = (0..AGENT_COUNT)
        .map(|_| tempfile::tempdir().expect("tempdir for concurrent agent pipe"))
        .collect();

    let mut thread_handles = Vec::new();

    for (i, dir) in dirs.iter().enumerate() {
        let results = Arc::clone(&results);
        let rendezvous_dir = dir.path().to_path_buf();

        let handle = std::thread::spawn(move || {
            let name = unique_profile_name("concurrent", i);

            // Mint the AppContainer profile for this concurrent "agent".
            let profile = nono::create_app_container_profile(&name).unwrap_or_else(|e| {
                panic!("[thread {i}] create_app_container_profile failed: {e}")
            });

            // Derive the package SID.
            let owned_sid = derive_app_container_sid(&name)
                .unwrap_or_else(|e| panic!("[thread {i}] derive_app_container_sid failed: {e}"));

            let pkg_sid = package_sid_to_string(&owned_sid)
                .unwrap_or_else(|e| panic!("[thread {i}] package_sid_to_string failed: {e}"));

            eprintln!("[spike74][concurrent][thread {i}] pkg_sid={pkg_sid}");

            // Create a pipe instance tied to this agent's pkg SID.
            // This runs the server side in a background thread within this thread's scope.
            // We just create the pipe and immediately release it (no blocking wait).
            let rendezvous = rendezvous_dir.join(format!("concurrent-{i}.rendezvous"));
            let pkg_sid_for_pipe = pkg_sid.clone();
            let rendezvous_for_server = rendezvous.clone();

            // Spawn the blocking server, then immediately connect from this thread to unblock it.
            let server_thread = std::thread::spawn(move || {
                nono::SupervisorSocket::bind_low_integrity_with_session_and_package_sid(
                    &rendezvous_for_server,
                    None,
                    Some(&pkg_sid_for_pipe),
                )
            });

            // Wait for rendezvous.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while !rendezvous.exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "[thread {i}] rendezvous not published within 10s"
                );
                std::thread::sleep(std::time::Duration::from_millis(20));
            }

            // Connect (Medium-IL, current process token) to unblock the server.
            let _client = nono::SupervisorSocket::connect(&rendezvous)
                .unwrap_or_else(|e| panic!("[thread {i}] concurrent connect failed: {e}"));

            let _server = server_thread
                .join()
                .expect("[thread {i}] server thread panicked")
                .unwrap_or_else(|e| panic!("[thread {i}] server bind failed: {e}"));

            eprintln!(
                "[spike74][concurrent][thread {i}] pipe round-trip COMPLETE for sid={pkg_sid}"
            );

            // Record result for cross-thread assertion.
            let mut guard = results.lock().expect("results mutex poisoned");
            guard.push((i, pkg_sid));

            drop(owned_sid);
            drop(profile);
        });

        thread_handles.push(handle);
    }

    // Wait for all concurrent agent threads to complete.
    for (i, handle) in thread_handles.into_iter().enumerate() {
        handle
            .join()
            .unwrap_or_else(|_| panic!("[thread {i}] concurrent agent thread panicked"));
    }

    // -----------------------------------------------------------------------
    // Assert cross-thread results:
    // 1. Both agents completed (AGENT_COUNT results).
    // 2. Both package SIDs are DISTINCT (no reuse across concurrent agents).
    // -----------------------------------------------------------------------
    let guard = results.lock().expect("results mutex poisoned");
    assert_eq!(
        guard.len(),
        AGENT_COUNT,
        "Expected {AGENT_COUNT} concurrent agent results, got {}",
        guard.len()
    );

    let sids: Vec<&str> = guard.iter().map(|(_, s)| s.as_str()).collect();
    let unique_sids: HashSet<&str> = sids.iter().copied().collect();
    assert_eq!(
        unique_sids.len(),
        AGENT_COUNT,
        "Concurrent agents must have DISTINCT package SIDs — token reuse detected!\n\
         sids: {sids:?}"
    );

    for (i, sid) in guard.iter() {
        eprintln!("[spike74][concurrent] agent {i} SID: {sid}");
    }

    eprintln!(
        "[spike74][concurrent] PASS: {AGENT_COUNT} concurrent agents each produced a \
         distinct package SID and independent pipe instance"
    );
}

// ---------------------------------------------------------------------------
// Helpers used by Test 3 (SC2)
// ---------------------------------------------------------------------------

/// Spawn a real AppContainer child process inside the AppContainer identified by
/// `package_sid_psid`. The child runs `cmd.exe /c ping -n 30 127.0.0.1` so it
/// stays alive long enough for the caller to open its token.
///
/// The caller MUST `TerminateProcess + CloseHandle(hProcess) + CloseHandle(hThread)`
/// when done. The AppContainer PROFILE for `package_sid_psid` MUST already be
/// registered (via `nono::create_app_container_profile`) or `CreateProcessW` will
/// fail `ERROR_FILE_NOT_FOUND`.
///
/// This mirrors the `spawn_appcontainer_child` helper in `launch.rs` broker tests.
fn spawn_appcontainer_child_for_test(
    package_sid_psid: windows_sys::Win32::Security::PSID,
) -> windows_sys::Win32::System::Threading::PROCESS_INFORMATION {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Security::SECURITY_CAPABILITIES;
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, InitializeProcThreadAttributeList,
        UpdateProcThreadAttribute, EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST,
        PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, STARTUPINFOEXW,
        STARTUPINFOW,
    };

    // Probe the required attribute-list size for 1 slot, then initialize.
    let mut attr_size: usize = 0;
    unsafe {
        // SAFETY: documented probe idiom — null list returns required size.
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attr_size);
    }
    let mut attr_buf = vec![0u8; attr_size];
    let attr_list: LPPROC_THREAD_ATTRIBUTE_LIST =
        attr_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
    let ok = unsafe {
        // SAFETY: attr_list points to attr_buf sized by the probe above for 1 slot.
        InitializeProcThreadAttributeList(attr_list, 1, 0, &mut attr_size)
    };
    assert!(ok != 0, "InitializeProcThreadAttributeList failed");

    // Empty capability set: most restrictive lowbox.
    let caps = SECURITY_CAPABILITIES {
        AppContainerSid: package_sid_psid,
        Capabilities: std::ptr::null_mut(),
        CapabilityCount: 0,
        Reserved: 0,
    };
    let ok = unsafe {
        // SAFETY: attr_list initialized for 1 slot; caps and its AppContainerSid
        // (owned by the caller's OwnedAppContainerSid) outlive the CreateProcessW call.
        UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            &caps as *const SECURITY_CAPABILITIES as *mut _,
            std::mem::size_of::<SECURITY_CAPABILITIES>(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert!(
        ok != 0,
        "UpdateProcThreadAttribute(SECURITY_CAPABILITIES) failed; GetLastError={}",
        unsafe { GetLastError() }
    );

    let mut si: STARTUPINFOEXW = unsafe {
        // SAFETY: STARTUPINFOEXW is a plain Win32 struct safe to zero-init.
        std::mem::zeroed()
    };
    si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    si.lpAttributeList = attr_list;

    // Long-lived child so the caller can open its token before it exits.
    let mut cmdline: Vec<u16> = std::ffi::OsStr::new("cmd.exe /c ping -n 30 127.0.0.1")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let cwd: Vec<u16> = std::ffi::OsStr::new("C:\\Windows\\System32")
        .encode_wide()
        .chain(Some(0))
        .collect();

    let mut pi: PROCESS_INFORMATION = unsafe {
        // SAFETY: PROCESS_INFORMATION is a plain struct safe to zero-init.
        std::mem::zeroed()
    };
    let lp_si = &si.StartupInfo as *const STARTUPINFOW;
    let created = unsafe {
        // SAFETY: cmdline/cwd are nul-terminated UTF-16; si carries the
        // EXTENDED_STARTUPINFO_PRESENT + SECURITY_CAPABILITIES attribute that
        // places the child in the AppContainer identified by package_sid_psid.
        CreateProcessW(
            std::ptr::null(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0, // bInheritHandles = FALSE: no handle leak to child
            EXTENDED_STARTUPINFO_PRESENT,
            std::ptr::null(),
            cwd.as_ptr(),
            lp_si,
            &mut pi,
        )
    };
    unsafe {
        // SAFETY: attr_list was initialized above and is no longer needed after CreateProcessW.
        DeleteProcThreadAttributeList(attr_list);
    }
    assert!(
        created != 0,
        "CreateProcessW (AppContainer B child) failed; GetLastError={}",
        unsafe { windows_sys::Win32::Foundation::GetLastError() }
    );
    pi
}

// ---------------------------------------------------------------------------
// Test 5: classify_pid_returns_verdict_from_daemon (SC1 / SC2 / SC4 — Phase 78)
// ---------------------------------------------------------------------------

/// Control pipe name — must match `DAEMON_CONTROL_PIPE_NAME` in agent_cli.rs.
const NONO_AGENTD_CONTROL_PIPE: &str = r"\\.\pipe\nono-agentd-control";

/// Send a JSON frame to the daemon's control pipe and return the response string.
///
/// Framing: `[4-byte LE length][JSON payload bytes]` (send) /
///          `[4-byte LE length][response bytes]` (receive).
/// Mirrors `windows_control_pipe_request` in `agent_cli.rs` (which is `pub(crate)`
/// and therefore not reachable from integration tests — this local helper replicates
/// the identical framing logic for the test binary).
///
/// Returns `Err(String)` with a diagnostic on any pipe / I/O failure.
fn daemon_control_pipe_request(json_payload: &str) -> Result<String, String> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, ReadFile, WriteFile, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Pipes::WaitNamedPipeW;

    const TIMEOUT_MS: u32 = 5_000;
    const MAX_RESPONSE: usize = 64 * 1024;
    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;

    let pipe_wide: Vec<u16> = NONO_AGENTD_CONTROL_PIPE
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect();

    // SAFETY: pipe_wide is a valid null-terminated UTF-16 string.
    let handle = unsafe {
        CreateFileW(
            pipe_wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        let gle = unsafe { GetLastError() };
        return Err(format!(
            "daemon_control_pipe_request: failed to open control pipe (GLE={gle}): \
             is nono-agentd running?"
        ));
    }

    // RAII handle guard.
    struct Guard(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Guard {
        fn drop(&mut self) {
            if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                // SAFETY: self.0 is a valid HANDLE from CreateFileW.
                unsafe { CloseHandle(self.0) };
            }
        }
    }
    let _guard = Guard(handle);

    // SAFETY: pipe_wide is a valid null-terminated UTF-16 string.
    let wait_ok = unsafe { WaitNamedPipeW(pipe_wide.as_ptr(), TIMEOUT_MS) };
    if wait_ok == 0 {
        let gle = unsafe { GetLastError() };
        return Err(format!(
            "daemon_control_pipe_request: timed out waiting for pipe (GLE={gle}, {TIMEOUT_MS}ms)"
        ));
    }

    // Send: [4-byte LE length][payload].
    let payload_bytes = json_payload.as_bytes();
    let payload_len = u32::try_from(payload_bytes.len())
        .map_err(|_| "daemon_control_pipe_request: payload too large".to_string())?;
    let len_prefix = payload_len.to_le_bytes();

    let mut bytes_written: u32 = 0;
    // SAFETY: handle is a valid open pipe; len_prefix is 4 bytes.
    let ok = unsafe {
        WriteFile(
            handle,
            len_prefix.as_ptr(),
            4,
            &mut bytes_written,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || bytes_written != 4 {
        return Err("daemon_control_pipe_request: WriteFile length prefix failed".into());
    }
    // SAFETY: handle is valid; payload_bytes is a valid slice.
    let ok = unsafe {
        WriteFile(
            handle,
            payload_bytes.as_ptr(),
            payload_len,
            &mut bytes_written,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || bytes_written != payload_len {
        return Err("daemon_control_pipe_request: WriteFile payload failed".into());
    }

    // Receive: [4-byte LE length][response].
    let mut len_buf = [0u8; 4];
    let mut bytes_read: u32 = 0;
    // SAFETY: handle is valid; len_buf is 4-byte mutable buffer.
    let ok = unsafe {
        ReadFile(
            handle,
            len_buf.as_mut_ptr(),
            4,
            &mut bytes_read,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || bytes_read != 4 {
        return Err("daemon_control_pipe_request: ReadFile response length failed".into());
    }

    let resp_len = u32::from_le_bytes(len_buf) as usize;
    if resp_len > MAX_RESPONSE {
        return Err(format!(
            "daemon_control_pipe_request: response length {resp_len} exceeds max {MAX_RESPONSE}"
        ));
    }

    let mut resp_buf = vec![0u8; resp_len];
    let mut bytes_read2: u32 = 0;
    // SAFETY: handle is valid; resp_buf is a valid mutable slice.
    let ok = unsafe {
        ReadFile(
            handle,
            resp_buf.as_mut_ptr(),
            resp_len as u32,
            &mut bytes_read2,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || bytes_read2 != resp_len as u32 {
        return Err("daemon_control_pipe_request: ReadFile response payload failed".into());
    }

    String::from_utf8(resp_buf)
        .map_err(|e| format!("daemon_control_pipe_request: response not valid UTF-8: {e}"))
}

/// **SC1 / SC2 / SC4 — Cross-process classify from live daemon (Phase 78 CLAS-01/02).**
///
/// Requires `NONO_DAEMON_INTEGRATION_TESTS=1` and a running `nono-agentd`.
///
/// SC1 (non-optional): launches a real confined agent via the daemon's Launch verb,
/// obtains its PID from the response, then calls the Classify verb and asserts the
/// response is exactly "AiAgent". A "NotAnAgent" or any other response is a FAIL,
/// not a skip.
///
/// SC2: classifies the test process's own PID (which is NOT a daemon-launched
/// AppContainer agent) and asserts "NotAnAgent".
///
/// SC4: neither classify response contains "package_sid" or "S-1-15-2-".
#[test]
fn classify_pid_returns_verdict_from_daemon() {
    require_integration!();
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    // -----------------------------------------------------------------------
    // Step 1: Launch a real confined agent via the daemon's Launch verb.
    //         Profile "default" is always available in policy.json.
    //         cmd.exe /c timeout 30 keeps the agent alive long enough to classify it.
    // -----------------------------------------------------------------------
    let launch_payload = serde_json::json!({
        "action": "launch",
        "profile": "default",
        "cmd": ["cmd.exe", "/c", "timeout", "30"],
    })
    .to_string();

    let launch_response = daemon_control_pipe_request(&launch_payload)
        .expect("daemon_control_pipe_request for launch failed — is nono-agentd running?");

    eprintln!("[classify-test] launch response: {launch_response}");

    // Parse pid from response: "Launched agent:\n  tenant_id=...\n  ...\n  pid=<N>"
    let agent_pid: u32 = {
        let pid_line = launch_response
            .lines()
            .find(|l| l.trim_start().starts_with("pid="))
            .unwrap_or_else(|| {
                panic!(
                    "classify-test: launch response did not contain a 'pid=' line.\n\
                     Response was: {launch_response}"
                )
            });
        pid_line
            .trim()
            .strip_prefix("pid=")
            .unwrap_or("")
            .parse()
            .unwrap_or_else(|_| {
                panic!("classify-test: could not parse pid from launch response line: {pid_line}")
            })
    };

    assert_ne!(
        agent_pid, 0,
        "classify-test: agent PID must be non-zero; launch response: {launch_response}"
    );
    eprintln!("[classify-test] SC1 agent_pid={agent_pid}");

    // -----------------------------------------------------------------------
    // Step 2 (SC1): Classify the launched agent's PID.
    //               Response MUST be "AiAgent" — this is NON-optional.
    // -----------------------------------------------------------------------
    let classify_agent_payload = serde_json::json!({
        "action": "classify",
        "pid": agent_pid,
    })
    .to_string();

    let agent_classify_response = daemon_control_pipe_request(&classify_agent_payload)
        .expect("daemon_control_pipe_request for classify (agent pid) failed");

    eprintln!(
        "[classify-test] SC1 classify response for agent pid={agent_pid}: {agent_classify_response}"
    );

    // SC4 check: no SID in the AiAgent response.
    assert!(
        !agent_classify_response.contains("package_sid"),
        "SC4 FAIL: AiAgent classify response contains 'package_sid': {agent_classify_response}"
    );
    assert!(
        !agent_classify_response.contains("S-1-15-2-"),
        "SC4 FAIL: AiAgent classify response contains a SID string: {agent_classify_response}"
    );

    // SC1: must be "AiAgent" — not a skip if wrong, it's a FAIL.
    assert_eq!(
        agent_classify_response.trim(),
        "AiAgent",
        "SC1 FAIL: expected 'AiAgent' for daemon-launched confined agent (pid={agent_pid}), \
         got '{}'.\n\
         This is a load-bearing CLAS-01 regression: the daemon's shared registry \
         must recognise its own launched agents.",
        agent_classify_response.trim()
    );

    eprintln!("[classify-test] SC1 PASS: AiAgent for pid={agent_pid}");

    // -----------------------------------------------------------------------
    // Step 3 (SC2): Classify the test process's own PID — NOT a daemon-launched
    //               AppContainer agent, so the response must be "NotAnAgent".
    // -----------------------------------------------------------------------
    let own_pid = std::process::id();
    let classify_self_payload = serde_json::json!({
        "action": "classify",
        "pid": own_pid,
    })
    .to_string();

    let self_classify_response = daemon_control_pipe_request(&classify_self_payload)
        .expect("daemon_control_pipe_request for classify (self pid) failed");

    eprintln!(
        "[classify-test] SC2 classify response for self pid={own_pid}: {self_classify_response}"
    );

    // SC4 check: no SID in the NotAnAgent response.
    assert!(
        !self_classify_response.contains("package_sid"),
        "SC4 FAIL: NotAnAgent classify response contains 'package_sid': {self_classify_response}"
    );
    assert!(
        !self_classify_response.contains("S-1-15-2-"),
        "SC4 FAIL: NotAnAgent classify response contains a SID string: {self_classify_response}"
    );

    // SC2: must be "NotAnAgent".
    assert_eq!(
        self_classify_response.trim(),
        "NotAnAgent",
        "SC2 FAIL: expected 'NotAnAgent' for test process own PID (pid={own_pid}), got '{}'.",
        self_classify_response.trim()
    );

    eprintln!("[classify-test] SC2 PASS: NotAnAgent for own pid={own_pid}");
    eprintln!("[classify-test] SC4 PASS: no SID in either classify response");
    eprintln!("[classify-test] ALL assertions PASS (SC1/SC2/SC4)");

    // -----------------------------------------------------------------------
    // Step 4: Parse tenant_id from the launch response and send a demote
    //         request to leave the daemon in a clean state.
    //         Demote is best-effort — test passes even if demote fails
    //         (the agent will exit naturally after `timeout 30` elapses).
    // -----------------------------------------------------------------------
    let tenant_id = launch_response
        .lines()
        .find(|l| l.trim_start().starts_with("tenant_id="))
        .and_then(|l| l.trim().strip_prefix("tenant_id="))
        .unwrap_or("")
        .to_string();

    if !tenant_id.is_empty() {
        let demote_payload = serde_json::json!({
            "action": "demote",
            "tenant_id": tenant_id,
        })
        .to_string();
        match daemon_control_pipe_request(&demote_payload) {
            Ok(resp) => eprintln!("[classify-test] cleanup demote response: {resp}"),
            Err(e) => eprintln!("[classify-test] cleanup demote failed (non-fatal): {e}"),
        }
    } else {
        eprintln!(
            "[classify-test] cleanup: could not parse tenant_id — agent will exit after timeout 30"
        );
    }
}
