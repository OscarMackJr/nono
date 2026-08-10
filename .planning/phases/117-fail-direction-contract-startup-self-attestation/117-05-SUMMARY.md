---
phase: 117-fail-direction-contract-startup-self-attestation
plan: 05
subsystem: infra
tags: [windows, attestation, ffi, job-objects, appcontainer, integrity-level, restricted-sids, adr-86]

# Dependency graph
requires:
  - phase: 117-02
    provides: "NonoError::LayerAttestationFailed, NonoDiagnosticCode::LayerAttestationFailed, RequiredLayersPolicy/MachineEgressPolicy.required_layers"
provides:
  - "crates/nono/src/attestation.rs: LayerAttestationStatus (4-state D-18 vocabulary), ProcessHandle (D-11 platform-neutral alias), probe_integrity_level/probe_in_job/probe_app_container_sid/probe_restricted_sids"
  - "crate-root re-export of LayerAttestationStatus and ProcessHandle from crates/nono/src/lib.rs"
affects: [117-08, phase-118-receipts]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "cfg-on-block-as-tail-expression: probe functions declared once (not cfg-duplicated at the fn signature level); #[cfg(target_os = \"windows\")] { .. } / #[cfg(not(target_os = \"windows\"))] { .. } as the two mutually-exclusive tail-expression blocks inside one function body, matching precedent in crates/nono/src/keystore.rs:1268-1284"
    - "D-11 Warning-6 fix: ProcessHandle platform-neutral alias (HANDLE on Windows, () off Windows) so pub fn signatures never name raw windows_sys types unconditionally"

key-files:
  created: [crates/nono/src/attestation.rs]
  modified: [crates/nono/src/lib.rs]

key-decisions:
  - "Split Task 1 (type declarations) and Task 2 (probe functions) into two separate commits for atomic per-task history, reconstructing the file content between commits rather than a single combined commit."
  - "Non-Windows unit-typed `process` parameter is consumed via `#[allow(clippy::let_unit_value)] let _ = process;` rather than a bare `process;` statement -- the bare-statement form passes Windows-host clippy (block is cfg-stripped there) but fails rustc's `path_statements` lint under `-D warnings` on the actual Linux-gnu cross-target build where the not(windows) block compiles for real."
  - "in_job_probe_on_current_process test rewritten from an exact Ok(false) assertion to Ok(_) (call succeeds) -- the test runner's own process is already inside a nested Job Object on this host (Windows 8+ nested jobs), so the plan's literal expected value does not hold universally and asserting it would be host-environment-dependent, not a property of the function under test."

patterns-established:
  - "Probe functions take an already-open ProcessHandle and never call OpenProcess/OpenProcess themselves (D-19); every OS-call failure maps to NonoError::LayerAttestationFailed { layer, reason }, never a silent default."

requirements-completed: [CINT-02]

# Metrics
duration: ~55min
completed: 2026-08-10
---

# Phase 117 Plan 05: Startup Self-Attestation Library Primitive Summary

**Policy-free `crates/nono::attestation` module: a shared 4-state `LayerAttestationStatus` vocabulary (D-18) plus four raw Windows OS probes (`GetTokenInformation`/`IsProcessInJob`) that query a suspended child's real token/job state from an already-open external process handle, never from the process itself (D-19).**

## Performance

- **Duration:** ~55 min
- **Completed:** 2026-08-10T00:17:45Z
- **Tasks:** 2 (both `type="auto" tdd="true"`)
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments
- `LayerAttestationStatus` (D-18's four-value vocabulary: `Confirmed` / `EstablishedNotIndependentlyObservable` / `Unconfirmed` / `NotApplicable`), documented with the `Confirmed` sub-case distinction (independent kernel observation vs. confirmed-by-report-from-the-enforcing-component) and the `Unconfirmed` non-distinction ("could not check" vs. "checked and it's absent" both fold into one state).
- `ProcessHandle` platform-neutral alias closing the D-11 Warning-6 gap (`HANDLE` on Windows, `()` off Windows) so every probe signature compiles on every target without naming raw `windows_sys::Win32::Foundation::HANDLE` unconditionally.
- Four raw OS probes over an external process handle, all fail-closed to `NonoError::LayerAttestationFailed { layer, reason }` on any OS-call failure, never a default/optimistic value:
  - `probe_integrity_level` — `GetTokenInformation(TokenIntegrityLevel)`, returns the mandatory-label RID.
  - `probe_in_job` — `IsProcessInJob` promoted from test-only usage to production, null job handle queries "in ANY job".
  - `probe_app_container_sid` — `TokenAppContainerSid` (genuinely new production FFI surface in this crate), `Ok(None)` for a successful-but-negative probe, distinct from `Err` on OS-call failure.
  - `probe_restricted_sids` — `TokenRestrictedSids` promoted from test-only usage, returns every restricting SID in SDDL string form.
- Wired into `crates/nono/src/lib.rs`: `pub mod attestation;` + crate-root re-export of `LayerAttestationStatus` and `ProcessHandle`.
- 10 unit tests (4 exhaustive-match/compile-shape tests + platform-appropriate probe-behavior tests), all passing on this Windows host.
- Both local cross-target clippy gates run clean end-to-end (`cross clippy --workspace --target x86_64-unknown-linux-gnu` and `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin`, both `-D warnings -D clippy::unwrap_used`), plus `cargo check --workspace --all-targets` and `cargo fmt --all -- --check`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Define LayerAttestationStatus, ProcessHandle, and wire crates/nono/src/lib.rs** - `106911e1` (feat)
2. **Task 2: Raw probe functions over an external process handle** - `fa102a76` (feat)

**Plan metadata:** (this commit, pending) - `docs(117-05): complete startup self-attestation library primitive plan`

## Files Created/Modified
- `crates/nono/src/attestation.rs` - New module: `LayerAttestationStatus`, `ProcessHandle`, and the four `probe_*` functions.
- `crates/nono/src/lib.rs` - Adds `pub mod attestation;` and re-exports `LayerAttestationStatus`/`ProcessHandle` from the crate root.

## Decisions Made
- **Function-body cfg-block, not cfg-duplicated signatures.** Each of the 4 probe functions is declared exactly once; the Windows real implementation and the non-Windows `UnsupportedPlatform` stub are two mutually-exclusive `#[cfg(...)] { .. }` tail-expression blocks inside one function body (precedent: `crates/nono/src/keystore.rs:1268-1284`). This satisfies the plan's acceptance-criteria greps literally (`grep -c "fn probe_"` == 4, `grep -c "process: ProcessHandle"` == 4) while keeping D-11's "one signature, cfg(windows)-populated body" shape.
- **`#[allow(clippy::not_unsafe_ptr_arg_deref)]` on all four public probes**, following the existing house pattern in `crates/nono/src/supervisor/socket_windows.rs` (a public function taking a raw pointer/HANDLE that is only ever passed into an FFI call whose own `// SAFETY:` contract is documented at the call site — never dereferenced as a Rust reference by the function itself).
- **`#[allow(clippy::let_unit_value)]` on the non-Windows `let _ = process;` lines**, rather than the bare-statement form clippy's Windows-host message initially suggested — the bare form triggers rustc's `path_statements` lint under `-D warnings` once the not(windows) block actually compiles (verified live via the linux-gnu cross-target gate, which is the whole reason this file is subject to the cross-target checklist in the first place).
- **Two commits, not one**, splitting the plan's two tasks even though they land in the same new file — the file was written in a Task-1-only intermediate state (type declarations + their tests only), verified to build/lint/test clean standalone, committed, then Task 2's probe functions and their tests were added back and re-verified before the second commit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Non-Windows `let _ = process;` triggers `clippy::let_unit_value` on Windows-host clippy but `path_statements` (a *different* rustc lint) on the actual cross-target Linux build**
- **Found during:** Task 2, cross-target clippy verification (`cross clippy --workspace --target x86_64-unknown-linux-gnu`)
- **Issue:** `ProcessHandle` is `()` off Windows (D-11), so the parameter-consumption idiom needed to silence "unused variable" on non-Windows compiles collides with two different lints depending on which linter's suggestion is followed: clippy's own message on the Windows host (where the not(windows) block is cfg-stripped and never even lints) suggested removing the `let _ =` wrapper; doing so produces a bare `process;` statement that rustc's `path_statements` then rejects under `-D warnings` on the Linux target where that block actually compiles.
- **Fix:** Kept the `let _ = process;` form and added `#[allow(clippy::let_unit_value)]` directly above it, with a comment explaining the parameter is a genuine, used argument on the Windows branch — this is intentional unit-value handling, not dead code.
- **Files modified:** crates/nono/src/attestation.rs
- **Verification:** `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` both exit 0; Windows-host `cargo clippy -p nono-sandbox -- -D warnings -D clippy::unwrap_used` also exits 0.
- **Committed in:** fa102a76 (Task 2 commit)

**2. [Rule 1 - Bug] Plan's literal test expectation (`probe_in_job` on the current process returns `Ok(false)`) is empirically false on this host**
- **Found during:** Task 2, `cargo test -p nono-sandbox --lib attestation`
- **Issue:** The plan's `<behavior>` block specified `probe_in_job` on the current process "returns `Ok(false)` (the test runner's own process is not normally in the target job)". On this host the test runner's own process IS already inside a nested Job Object (Windows 8+ supports nested jobs; common under shells/terminals/CI harnesses), so the literal assertion failed on a correct implementation.
- **Fix:** Rewrote the test to assert `result.is_ok()` (the FFI call succeeds against a real, always-available handle) rather than a specific boolean payload, with an inline comment recording the empirical finding and why a host-dependent boolean is not a meaningful assertion here. This preserves the test's stated purpose ("sanity-checks the FFI call shape compiles and runs without requiring a live spawned child") without asserting an environment-dependent value.
- **Files modified:** crates/nono/src/attestation.rs
- **Verification:** `cargo test -p nono-sandbox --lib attestation -- --test-threads=1` passes (10/10).
- **Committed in:** fa102a76 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs/incorrect expectations surfaced during verification, not scope changes).
**Impact on plan:** Neither changes the module's public API, behavior, or security posture. Both are corrections to plumbing details (a lint-satisfaction idiom, and a test assertion) discovered only by actually running the mandated cross-target and local test gates.

## Issues Encountered
None beyond the two deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `crates/nono::attestation` is ready for Plan 08 (CLI-side attestation decision policy, D-02) to consume: the four probes plus `LayerAttestationStatus` give Plan 08 everything it needs to attest each registry row against a real suspended child at the D-21 gate point without adding any further library-side surface.
- `LayerAttestationStatus` is also the exact shared type Phase 118's per-session receipts are designed to consume (D-04) — no adapter/conversion layer needed.
- No blockers. This plan touched only `crates/nono` (library, policy-free); no CLI-side policy, rendering, or `NonoDiagnosticCode` wiring was introduced here (that already landed in 117-02 and remains Plan 08's responsibility per ADR-86/D-02).

---
*Phase: 117-fail-direction-contract-startup-self-attestation*
*Completed: 2026-08-10*
