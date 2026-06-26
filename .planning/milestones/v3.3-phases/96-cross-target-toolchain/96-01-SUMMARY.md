---
phase: 96-cross-target-toolchain
plan: 01
subsystem: infra
tags: [cross, docker, clippy, cross-target, seccomp, cgroup, landlock, linux-gnu, fork-invariant]

requires:
  - phase: 95-upstream-absorb-fork-invariant-verify
    provides: post-sync tree (HEAD be42a5af) that deferred linux-gnu clippy to PARTIAL→CI
provides:
  - linux-gnu cross clippy gate proven runnable + GREEN (exit 0) locally on the Windows dev host
  - pinned cross image tag recorded (ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5 @ sha256:9e5b39c0...)
  - restored SEC-01 AF_UNIX no-grant static-EPERM seccomp filter (dropped by Phase 95 absorb)
  - restored Linux cgroup v2 resource-enforcement module (REQ-RESL-NIX; dropped by Phase 95 absorb)
  - converged Linux SupportInfo/NetworkAuditEvent/approval call sites to upstream-absorbed API shapes
affects: [96-02 apple-darwin, 96-03 checklist/CLAUDE.md rewrite, 97-release, verify-work]

tech-stack:
  added: []
  patterns:
    - "linux-gnu gate = cross clippy inside pinned Docker image (D-01); discharges the bare cargo-clippy contract"
    - "Recover dropped fork invariants verbatim from the last-good pre-absorb commit (ae77d198^), not by re-derivation"

key-files:
  created:
    - .planning/phases/96-cross-target-toolchain/96-01-XTGT-LINUX-GNU-RECORD.md
  modified:
    - crates/nono/src/sandbox/linux.rs
    - crates/nono-cli/src/exec_strategy/supervisor_linux.rs

key-decisions:
  - "First local linux-gnu run surfaced compile errors (not lints): dropped fork code + stale call sites, invisible on Windows"
  - "Restore dropped fork invariants verbatim from ae77d198^; align stale sites to the converged API — all structural, no silencing allows"
  - "Daemon-down would be an operator precondition (start it), never a PARTIAL; it was confirmed UP (Server 29.5.3)"

patterns-established:
  - "Pin + record the exact cross image digest for reproducibility (SC#1)"
  - "cfg(target_os=linux) drift is only catchable by the containerized gate — Windows clippy is structurally blind to it"

requirements-completed: [XTGT-01, XTGT-02]

duration: 26min
completed: 2026-06-26
---

# Phase 96 Plan 01: linux-gnu Cross-Target Toolchain Gate Summary

**Stood up `cross clippy` for `x86_64-unknown-linux-gnu` as a provably-green local gate, and fixed the cfg-gated Unix drift it exposed — including two silently-dropped fork security/resource invariants (AF_UNIX no-grant EPERM filter, cgroup v2 enforcement) — until it exits 0.**

## Performance

- **Duration:** ~26 min
- **Started:** 2026-06-26T15:33:00Z
- **Completed:** 2026-06-26T15:58:55Z
- **Tasks:** 2
- **Files modified:** 2 source + 1 record created

## Accomplishments
- The linux-gnu `cross clippy` gate runs and **exits 0** under `-D warnings -D clippy::unwrap_used` on this Windows host — retiring the Phase 95 linux-gnu PARTIAL→CI deferral (XTGT-01 + XTGT-02).
- Caught + restored a **security-critical regression**: the SEC-01 AF_UNIX no-grant static-EPERM seccomp filter (AF_UNIX datagram bypass #1096 deny path) had been silently dropped from `linux.rs` by the Phase 95 absorb while its re-export and caller remained — invisible to Windows clippy.
- Caught + restored the **Linux cgroup v2 resource-enforcement module** (shipped REQ-RESL-NIX-01/02) dropped from `supervisor_linux.rs` by the same absorb, while its callers + macOS sibling were intact.
- Recorded the exact pinned cross image digest (SC#1) for reproducibility.

## Task Commits

1. **Task 2: Structural drift fixes until the gate exits 0** - `1a804977` (fix)
2. **Task 1: Confirm Docker, run gate, record image tag + triage** - `03d0fb08` (docs)

_(Tasks executed interleaved as a triage→fix→re-run loop, per the plan's anticipated unknown-size backlog; committed as the two logical artifacts.)_

## Files Created/Modified
- `.planning/phases/96-cross-target-toolchain/96-01-XTGT-LINUX-GNU-RECORD.md` - Gate evidence: Docker precondition, verbatim command, pinned image tag, drift triage table, structural fix summary, exit-0 proof, native no-regression check.
- `crates/nono/src/sandbox/linux.rs` - Restored `build_/install_seccomp_af_unix_nogrant_filter` + test; added `status` field to both `SupportInfo` initializers.
- `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` - Restored `pub(super) mod cgroup` (CgroupSession + tests); slimmed the `NetworkAuditEvent` construction to the converged 15-field struct; migrated the seccomp-notify approval path to `CapabilityRequest` + `request_capability()` + `is_approved()`.

## Decisions Made
- **First-run drift is compile-level, not lint-level.** The very first local linux-gnu run failed at compilation (E0432/E0063/E0433/E0560/E0599) inside `#[cfg(target_os="linux")]` code — proof the Windows host had never type-checked these branches. Root cause: Phase 95 absorbing upstream `ae77d198` (#1210) took the whole-file upstream shape and converged `nono`-crate types/APIs, dropping fork-specific Linux code and leaving stale sites.
- **Restore-verbatim over re-derive.** Dropped fork invariants were recovered byte-for-byte from the last-good pre-absorb commit `ae77d198^` (nogrant BPF filter, cgroup module) rather than re-written, minimizing risk.
- **Docker daemon was UP (Server 29.5.3)** — the Phase 95 trap (daemon-down → false PARTIAL) was explicitly avoided.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug / Rule 2 - Missing Critical] Restored dropped SEC-01 AF_UNIX no-grant EPERM seccomp filter**
- **Found during:** Task 1/2 (first gate run, `nono` lib compile)
- **Issue:** `install_seccomp_af_unix_nogrant_filter` re-exported (`mod.rs:40`) + called (`exec_strategy.rs:1499`) but its definition + private `build_seccomp_af_unix_nogrant_filter` were dropped from `linux.rs` by the Phase 95 absorb of `ae77d198`. The no-grant deny path for AF_UNIX datagram bypass #1096 was silently absent on Linux.
- **Fix:** Recovered both fns + the unit test verbatim from `ae77d198^`.
- **Files modified:** crates/nono/src/sandbox/linux.rs
- **Verification:** linux-gnu cross clippy compiles + lints clean; `test_build_seccomp_af_unix_nogrant_filter_denies_send_family` restored.
- **Committed in:** 1a804977

**2. [Rule 1 - Bug] Restored dropped Linux cgroup v2 resource-enforcement module**
- **Found during:** Task 1/2 (second gate run, `nono-cli` bin compile)
- **Issue:** `supervisor_linux::cgroup::CgroupSession` referenced by `UnixResourceLimitGuard::Linux(...)` + `apply_resource_limits_unix` (5 sites) but `pub(super) mod cgroup` was dropped from `supervisor_linux.rs` by the same absorb. The macOS sibling + all callers were intact. Shipped REQ-RESL-NIX-01/02 feature dead on Linux.
- **Fix:** Recovered the full `pub(super) mod cgroup { ... }` (incl. its `#[cfg(all(test, target_os="linux"))]` tests) verbatim from `ae77d198^`.
- **Files modified:** crates/nono-cli/src/exec_strategy/supervisor_linux.rs
- **Verification:** linux-gnu cross clippy exit 0.
- **Committed in:** 1a804977

**3. [Rule 3 - Blocking] Converged stale Linux call sites to absorbed API/struct shapes**
- **Found during:** Task 1/2 (compile runs)
- **Issue:** `SupportInfo` missing `status`; `NetworkAuditEvent` constructed with 17 dropped fields; approval used pre-convergence `ApprovalRequest::Capability` + `request_approval()` + `is_granted()` — none updated for the absorbed upstream shapes the Windows-visible sites already used.
- **Fix:** Added `status` to both `support_info()` initializers; removed the 17 `None`-valued `NetworkAuditEvent` fields (zero behavior change); migrated to `CapabilityRequest` (Phase-11 file shape) + `request_capability()` + `is_approved()`.
- **Files modified:** crates/nono/src/sandbox/linux.rs, crates/nono-cli/src/exec_strategy/supervisor_linux.rs
- **Verification:** linux-gnu cross clippy exit 0; native clippy + fmt-check exit 0.
- **Committed in:** 1a804977

---

**Total deviations:** 3 auto-fixed (2 dropped fork invariants restored [1 security-critical], 1 API convergence). **No `#[allow]` added to silence cross-target lints** (one narrow `#[allow(deprecated)]` for the API-sanctioned deprecated-but-required `CapabilityRequest.path` field; two `#[allow(clippy::unwrap_used)]` are recovered-verbatim test-module allows, CLAUDE.md-permitted).
**Impact on plan:** All fixes essential to reach SC#2 (exit 0). Two are restorations of shipped fork security/resource features the absorb regressed — high-value catches, exactly the gate's purpose. No scope creep.

## Issues Encountered
- No `make` binary on the Windows host; ran the Makefile's underlying cargo commands directly (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used`) — both exit 0.
- Pre-existing Windows baseline test failures (11 nono-cli + 1 nono) are unchanged and unrelated; the restored cgroup/nogrant tests are `cfg(target_os="linux")` and do not run on the Windows host.

## User Setup Required
None - no external service configuration required. (Operator must have Docker Desktop's Linux engine running to re-run the gate; it was confirmed up this session.)

## Next Phase Readiness
- linux-gnu gate is now local-runnable + green → Plan 96-03 can rewrite the checklist/CLAUDE.md to retire the linux-gnu PARTIAL→CI default (D-07), citing this record.
- Plan 96-02 (apple-darwin bounded zig attempt) is independent and unblocked.
- The restored fork invariants mean the post-sync Linux build now compiles + lints clean for the Phase 97 release tree.

## Self-Check: PASSED

- Files verified on disk: 96-01-SUMMARY.md, 96-01-XTGT-LINUX-GNU-RECORD.md, linux.rs, supervisor_linux.rs.
- Commits verified in git: `1a804977` (fix, DCO-signed), `03d0fb08` (docs, DCO-signed).
- Gate verified: `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` exit 0; native clippy + fmt-check exit 0.

---
*Phase: 96-cross-target-toolchain*
*Completed: 2026-06-26*
