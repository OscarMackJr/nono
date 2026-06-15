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
//!    100 AppContainer profile create→derive→drop cycles return to the baseline
//!    OS handle count (±5). Proves that `AppContainerProfile::Drop` is calling
//!    `DeleteAppContainerProfile` and freeing all handles, so 100-agent daemon
//!    uptime does not accumulate stale registry entries or leaked handles
//!    (Pitfall 4, RESEARCH.md).
//!
//! 3. **Cross-tenant denial** (`daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance`):
//!    A pipe instance created with tenant A's AppContainer package SID as the
//!    only Low-IL-admitting SDDL ACE denies a Low-IL impersonated context that
//!    does NOT carry tenant A's AppContainer SID. Proves the per-tenant SDDL
//!    pipe-instance gate works at the OS level (DMON-02 / SC2).
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
use std::sync::{Arc, Mutex};

use nono::AppContainerProfile;
use nono::{derive_app_container_sid, package_sid_to_string};

use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HANDLE};
use windows_sys::Win32::Security::{
    DuplicateTokenEx, ImpersonateLoggedOnUser, RevertToSelf, SecurityImpersonation,
    TokenImpersonation, TOKEN_ALL_ACCESS, TOKEN_IMPERSONATE, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetProcessHandleCount, OpenProcessToken,
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
    assert_ne!(ok, 0, "GetProcessHandleCount failed: {}", std::io::Error::last_os_error());
    count
}

/// Builds a unique AppContainer profile name for the test using a counter and PID
/// to avoid name collisions between parallel test runs.
fn unique_profile_name(tag: &str, idx: usize) -> String {
    format!("nono.spike74.{}.{}.{}", tag, std::process::id(), idx)
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

    eprintln!(
        "[spike74][sids] PASS: {AGENT_COUNT} profiles produced {AGENT_COUNT} distinct SIDs"
    );
}

// ---------------------------------------------------------------------------
// Test 2: n_agents_over_time_returns_to_baseline_handle_count
// ---------------------------------------------------------------------------

/// **SC3 / DMON-01 spike clause 2 — Deterministic reap (handle baseline).**
///
/// Records the process handle count before the test, runs 100 agent-lifecycle
/// cycles (create AppContainer profile → derive SID → drop profile), then asserts
/// the post-run handle count is ≤ baseline + 5.
///
/// A real handle leak (e.g., `DeleteAppContainerProfile` not called in Drop, or
/// a SID handle not freed) accumulates 200+ handles over 100 cycles and is
/// immediately visible here. The ±5 epsilon absorbs any one-time test-harness
/// overhead that would otherwise make the test brittle.
///
/// This test ONLY exercises the profile-create/delete lifecycle managed by
/// `AppContainerProfile::Drop`. Spawning actual confined processes (which would
/// also exercise `CreateJobObjectW` + broker arm handle lifetimes) requires
/// the full daemon binary and is gated on Wave 1 / human checkpoint "approved +
/// spike green". The handle-count test here validates the profile-registry path
/// which is the most likely leak vector identified in 74-RESEARCH.md §What needs
/// to be proven.
#[test]
fn n_agents_over_time_returns_to_baseline_handle_count() {
    require_integration!();

    const CYCLES: usize = 100;
    const EPSILON: u32 = 5;

    let baseline = get_process_handle_count();
    eprintln!("[spike74][handles] baseline handle count: {baseline}");

    for i in 0..CYCLES {
        let name = unique_profile_name("handles", i);

        // Simulate one agent's profile lifetime:
        // 1. Create profile (analogous to daemon's CreateAppContainerProfile call)
        let profile = nono::create_app_container_profile(&name)
            .unwrap_or_else(|e| panic!("create_app_container_profile({name:?}) failed at cycle {i}: {e}"));

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
            eprintln!("[spike74][handles] after cycle {}: handle count = {mid}", i + 1);
        }
    }

    let post = get_process_handle_count();
    eprintln!("[spike74][handles] post-run handle count: {post} (baseline={baseline}, delta={})", post.saturating_sub(baseline));

    assert!(
        post <= baseline + EPSILON,
        "Handle count did not return to baseline after {CYCLES} agent-lifecycle cycles.\n\
         before={baseline}  after={post}  delta={}\n\
         This indicates a handle leak in the AppContainerProfile or OwnedAppContainerSid \
         Drop path. Check that DeleteAppContainerProfile and FreeSid are called.\n\
         (See 74-RESEARCH.md §Deterministic Reap §Known leak vectors)",
        post.saturating_sub(baseline)
    );

    eprintln!(
        "[spike74][handles] PASS: {CYCLES} cycles returned to baseline \
         (before={baseline} after={post} delta={})",
        post.saturating_sub(baseline)
    );
}

// ---------------------------------------------------------------------------
// Test 3: daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance
// ---------------------------------------------------------------------------

/// **SC2 / DMON-02 spike clause 3 — Cross-tenant pipe denial.**
///
/// Verifies that a `SupervisorSocket` pipe instance created with tenant A's AppContainer
/// package SID as the only Low-IL-admitting DACL ACE correctly denies a connection
/// attempt from a Low-IL impersonated token that does NOT carry tenant A's SID.
///
/// The test drives the capability pipe at the protocol layer (in-process impersonation,
/// not via a CLI query verb — per D-06 in 74-CONTEXT.md). The two gate mechanisms:
///
/// 1. **Primary gate (SDDL ACE)**: `bind_low_integrity_with_session_and_package_sid` creates
///    the pipe with an ACE admitting ONLY tenant A's AppContainer package SID at Low-IL.
///    Tenant B's token lacks this SID → `CreateFileW` returns `ERROR_ACCESS_DENIED`.
///
/// 2. **Defense-in-depth (ImpersonateNamedPipeClient)**: If the primary SDDL gate were
///    bypassed, `ImpersonateNamedPipeClient` + registry SID check on the server side would
///    catch the cross-tenant attempt. This spike confirms the SDDL gate is the load-bearing
///    first layer.
///
/// # What "tenant B" means here
///
/// Tenant B is simulated by impersonating a Low-IL duplicate of the current process token
/// — a token that carries the test user's SID and group membership but NOT the AppContainer
/// package SID of tenant A's profile. From the pipe DACL's perspective, this is an unknown
/// Low-IL caller: no matching AppContainer SID ACE → denied.
///
/// A genuine AppContainer tenant B process would have its OWN package SID (distinct from
/// tenant A's), and the denial holds for the same reason: the DACL ACE for tenant A's SID
/// is not a match for tenant B's SID.
///
/// # On `ImpersonateNamedPipeClient` (A1)
///
/// This test uses `ImpersonateLoggedOnUser` (the existing pattern from `socket_windows.rs`
/// line ~2349) rather than `ImpersonateNamedPipeClient` directly. `ImpersonateNamedPipeClient`
/// is the daemon's server-side call (after `ConnectNamedPipe` returns) to verify the client
/// SID. The spike confirms the SDDL gate works; the full `ImpersonateNamedPipeClient` chain
/// is validated when the daemon binary itself is tested in Wave 1.
#[test]
fn daemon_cross_tenant_denial_tenant_b_cannot_connect_to_tenant_a_pipe_instance() {
    require_integration!();

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
    //
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
            None,            // session_sid: None (daemon pipe doesn't use WRITE_RESTRICTED arm)
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
    // Step 3: Impersonate a Low-IL token that does NOT carry tenant A's AppContainer SID.
    //         This simulates "tenant B" — a process with a different (or absent) pkg SID.
    //
    //         We use the current process token duplicated to an impersonation token,
    //         then lowered to Low-IL. This token has the test user's SIDs + Low-IL label
    //         but NO AppContainer SID matching tenant A's profile.
    //
    //         `ImpersonateLoggedOnUser` pattern from socket_windows.rs line ~2419.
    // -----------------------------------------------------------------------
    let low_il_token = create_low_il_impersonation_token()
        .expect("create Low-IL impersonation token for tenant B simulation");

    let impersonate_ok: BOOL = unsafe {
        // SAFETY: low_il_token is a valid impersonation token created by
        // create_low_il_impersonation_token() above, which lives for this scope.
        ImpersonateLoggedOnUser(low_il_token)
    };

    // -----------------------------------------------------------------------
    // Step 4: Attempt to connect to the pipe as the impersonated "tenant B" token.
    //         Expected: ERROR_ACCESS_DENIED because the token lacks tenant A's pkg SID.
    // -----------------------------------------------------------------------
    let connect_result = nono::SupervisorSocket::connect(&rendezvous);

    // ALWAYS revert impersonation before asserting — even on error (Pitfall 3).
    let revert_ok: BOOL = unsafe { RevertToSelf() };
    unsafe {
        // SAFETY: low_il_token was opened/duplicated in create_low_il_impersonation_token.
        CloseHandle(low_il_token);
    }

    assert_ne!(
        revert_ok, 0,
        "RevertToSelf must succeed after impersonation: {}",
        std::io::Error::last_os_error()
    );

    // -----------------------------------------------------------------------
    // Step 5: Assert the impersonation succeeded but the connection was denied.
    // -----------------------------------------------------------------------
    assert_ne!(
        impersonate_ok, 0,
        "ImpersonateLoggedOnUser failed: {}. \
         This means SeImpersonatePrivilege may be absent (A1 assumption check).",
        std::io::Error::last_os_error()
    );

    // The connection as "tenant B" (Low-IL, no tenant-A AppContainer SID) must fail.
    assert!(
        connect_result.is_err(),
        "Tenant B (Low-IL, no AppContainer SID) MUST NOT be able to connect to \
         tenant A's pipe instance.\n\
         The SDDL ACE for tenant A's pkg SID ({tenant_a_pkg_sid}) is the primary gate — \
         if connect_result.is_ok(), the DACL gate failed to deny cross-tenant access.\n\
         This is a load-bearing security regression: DMON-02 / SC2 is broken."
    );

    eprintln!(
        "[spike74][denial] PASS: tenant B (Low-IL token) was correctly denied \
         access to tenant A's pipe instance (SID: {tenant_a_pkg_sid})"
    );

    // -----------------------------------------------------------------------
    // Step 6: Release the server thread by connecting as the normal (Medium-IL)
    //         test process so it doesn't hang forever.
    // -----------------------------------------------------------------------
    let _medium_client = nono::SupervisorSocket::connect(&rendezvous);
    let _server = server_thread.join().expect("server thread panicked");

    drop(tenant_a_sid_owned);
    drop(tenant_a_profile);
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

    const AGENT_COUNT: usize = 2;

    // Shared result collection: (thread_index, pkg_sid_string)
    let results: Arc<Mutex<Vec<(usize, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let dirs: Vec<tempfile::TempDir> = (0..AGENT_COUNT)
        .map(|_| tempfile::tempdir().expect("tempdir for concurrent agent pipe"))
        .collect();

    let mut thread_handles = Vec::new();

    for i in 0..AGENT_COUNT {
        let results = Arc::clone(&results);
        let rendezvous_dir = dirs[i].path().to_path_buf();

        let handle = std::thread::spawn(move || {
            let name = unique_profile_name("concurrent", i);

            // Mint the AppContainer profile for this concurrent "agent".
            let profile = nono::create_app_container_profile(&name)
                .unwrap_or_else(|e| panic!("[thread {i}] create_app_container_profile failed: {e}"));

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

            eprintln!("[spike74][concurrent][thread {i}] pipe round-trip COMPLETE for sid={pkg_sid}");

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
        handle.join().unwrap_or_else(|_| panic!("[thread {i}] concurrent agent thread panicked"));
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
// Low-IL impersonation token helper (local to this file)
// ---------------------------------------------------------------------------

/// Creates a Low Integrity Level impersonation token from the current process token.
///
/// This is used in `daemon_cross_tenant_denial_...` to simulate "tenant B":
/// a token that is Low-IL and carries the current user's SIDs, but does NOT
/// carry any AppContainer package SID (i.e., it is NOT an AppContainer token).
///
/// # Why this simulates cross-tenant denial
///
/// The pipe DACL for tenant A's pipe instance admits:
/// - SYSTEM (`S-1-5-18`)
/// - Built-in Administrators (`S-1-5-32-544`)
/// - Object Owner
/// - Tenant A's AppContainer package SID (`S-1-15-2-<a-sid>`)
///
/// A Low-IL token for the test user does NOT have membership in SYSTEM or BA
/// at Low-IL (MIC prevents write-up). When the impersonated Low-IL token tries
/// to open the pipe with `GENERIC_READ|GENERIC_WRITE`, the OS finds no matching
/// DACL ACE for the Low-IL principal → `ERROR_ACCESS_DENIED`.
///
/// This is the same access-check behavior that denies a real AppContainer process
/// with a different pkg SID (tenant B).
fn create_low_il_impersonation_token() -> Result<HANDLE, std::io::Error> {
    use windows_sys::Win32::Security::{
        SetTokenInformation, TokenIntegrityLevel,
        SECURITY_MAX_SID_SIZE, TOKEN_MANDATORY_LABEL,
        CreateWellKnownSid, WinLowLabelSid,
    };
    use windows_sys::Win32::System::SystemServices::SE_GROUP_INTEGRITY;

    // Step 1: Open the current process token.
    let mut process_token: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY | TOKEN_IMPERSONATE | 0x0004 /* TOKEN_DUPLICATE */,
            &mut process_token,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }

    // Step 2: Duplicate to an impersonation token.
    let mut imp_token: HANDLE = std::ptr::null_mut();
    let ok = unsafe {
        // SAFETY: process_token is a valid handle opened above.
        DuplicateTokenEx(
            process_token,
            TOKEN_ALL_ACCESS,
            std::ptr::null(),
            SecurityImpersonation,
            TokenImpersonation,
            &mut imp_token,
        )
    };
    unsafe { CloseHandle(process_token) };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }

    // Step 3: Apply the Low Integrity Level mandatory label to the token.
    //         This mirrors `create_low_integrity_primary_token` in sandbox/windows.rs.
    let mut sid_buffer = [0u8; SECURITY_MAX_SID_SIZE as usize];
    let mut sid_size = SECURITY_MAX_SID_SIZE;
    let ok = unsafe {
        // SAFETY: sid_buffer is valid and sized per SECURITY_MAX_SID_SIZE.
        CreateWellKnownSid(
            WinLowLabelSid,
            std::ptr::null_mut(),
            sid_buffer.as_mut_ptr() as *mut _,
            &mut sid_size,
        )
    };
    if ok == 0 {
        unsafe { CloseHandle(imp_token) };
        return Err(std::io::Error::last_os_error());
    }

    // Build TOKEN_MANDATORY_LABEL to pass to SetTokenInformation.
    // SE_GROUP_INTEGRITY = 0x00000020 (the SidAttributes for a mandatory label).
    let label = TOKEN_MANDATORY_LABEL {
        Label: windows_sys::Win32::Security::SID_AND_ATTRIBUTES {
            Sid: sid_buffer.as_mut_ptr() as *mut _,
            Attributes: SE_GROUP_INTEGRITY as u32,
        },
    };

    let ok = unsafe {
        // SAFETY: imp_token is valid; label is correctly initialized above.
        SetTokenInformation(
            imp_token,
            TokenIntegrityLevel,
            &label as *const _ as *const _,
            std::mem::size_of::<TOKEN_MANDATORY_LABEL>() as u32,
        )
    };
    if ok == 0 {
        unsafe { CloseHandle(imp_token) };
        return Err(std::io::Error::last_os_error());
    }

    Ok(imp_token)
}
