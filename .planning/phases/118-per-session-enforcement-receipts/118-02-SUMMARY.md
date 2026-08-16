---
phase: 118-per-session-enforcement-receipts
plan: 02
subsystem: windows-sandbox-primitives
tags: [windows, dacl, acl, machine-policy, registry, d-08, d-04]
dependency-graph:
  requires: []
  provides:
    - crates/nono/src/sandbox/windows.rs::deny_sid_on_path
    - crates/nono/src/machine_policy.rs::MachineEgressPolicy.require_receipts
  affects:
    - crates/nono-cli (Plan 118-05 sink-infrastructure, Plan 118-06 broker sink)
tech-stack:
  added: []
  patterns:
    - "DENY_ACCESS ACL wrapper as a thin sibling of the existing SET_ACCESS grant_sid_*_on_path family"
    - "EGRESS abort-on-unreadable machine-policy field lifecycle (vs the required_layers/telemetry degrade template)"
key-files:
  created: []
  modified:
    - crates/nono/src/sandbox/windows.rs
    - crates/nono/src/machine_policy.rs
decisions:
  - "revoke_sid_on_path rewritten to manually walk+DeleteAce instead of SetEntriesInAclW(REVOKE_ACCESS), which was empirically found to skip DENY-type ACEs for a trustee"
  - "require_receipts follows the egress (abort) lifecycle, not required_layers/telemetry's degrade-to-default template, per D-04's explicit fail-closed wording"
metrics:
  duration: "~2.5h"
  completed: 2026-08-16
---

# Phase 118 Plan 02: Windows Win32/registry primitives (D-08 deny ACE + D-04 require-receipts policy) Summary

Added the two Wave-1 Win32/registry primitives Phase 118 needs that have no dependency on the
receipt type itself: a DENY-ACE DACL wrapper (`deny_sid_on_path`, D-08's second sink-guard
mechanism) and a fleet-wide `require_receipts` machine-policy field (D-04's fail-closed knob),
following the codebase's existing EGRESS abort-on-unreadable read lifecycle rather than the
degrade-to-default template `required_layers`/`telemetry` use.

## What Was Built

**Task 1 — `deny_sid_on_path` (D-08 DACL half), `crates/nono/src/sandbox/windows.rs`:**
- `pub fn deny_sid_on_path(path: &Path, sid: &str, access_mask: u32) -> Result<()>` — a thin
  `DENY_ACCESS` sibling of the existing `grant_sid_*_on_path` (`SET_ACCESS`) family, sharing the
  same `edit_dacl_for_sid` primitive. First `DENY_ACCESS` caller in this file.
- Doc comment states D-08's both-not-either rationale explicitly: `WRITE_RESTRICTED` tokens
  double-check restricting SIDs on writes only (reads bypass), so the DENY ACE closes the
  read-side gap the mandatory label's `NO_READ_UP` half does not reach on that arm; a Medium-IL
  broker child has no per-session SID for a DENY ACE to name, so the mandatory label is what
  covers that arm instead.
- New tests: `deny_then_revoke_sid_round_trips_on_tempdir` (round-trip + an ACE-type perturbation
  proof: the deny ACE must never be readable via the ALLOW-only test reader) and
  `deny_invalid_sid_fails_closed`.

**Task 2 — `require_receipts` (D-04), `crates/nono/src/machine_policy.rs`:**
- `MachineEgressPolicy.require_receipts: bool`, `#[serde(default)]`, read from
  `HKLM\SOFTWARE\Policies\nono\RequireReceipts` (`REG_DWORD`) in the same single registry pass
  that already reads `telemetry`/`required_layers`.
- New private `read_require_receipts` reader modeled on the EGRESS (abort) precedent: absent →
  `Ok(false)`; present valid `REG_DWORD` → `Ok(value != 0)`; present but wrong REG type →
  `Err(reason)`, propagated through `parse_policy`'s existing `?` chain straight to
  `read_machine_egress_policy_impl`'s `NonoError::PolicyLoadFailed` abort — the identical path
  every egress list field already uses. Explicitly NOT modeled on `read_required_layers`'s
  degrade-to-default template.
- Excluded from `is_unconfigured()` (documented invariant, mirroring the existing
  `telemetry`/`required_layers` notes) so a fleet admin who sets only this knob does not
  accidentally flip the daemon to strict deny-all egress.
- 6 new tests covering all three registry states (absent, present-valid 1, present-valid 0
  converse, present-malformed at both the `parse_policy` layer and end-to-end via the
  `NonoError` wrap) plus a new `is_unconfigured_ignores_require_receipts_field` exclusion test.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `revoke_sid_on_path` never actually removed DENY-type ACEs**

- **Found during:** Task 1, writing the deny-ACE round-trip test.
- **Issue:** `revoke_sid_on_path`'s own doc comment claimed `SetEntriesInAclW(REVOKE_ACCESS)`
  "removes ALL allow/deny ACEs for the trustee." This was never exercised for a DENY ACE because
  no `DENY_ACCESS` caller existed in this file before `deny_sid_on_path`. Empirically verified
  (via a merged-ACE-count probe inserted temporarily into `edit_dacl_for_sid`, then removed): the
  `REVOKE_ACCESS` merge step left a matching `ACCESS_DENIED_ACE` completely untouched (ACE count
  unchanged across the merge), while correctly removing `ACCESS_ALLOWED_ACE` entries for the same
  trustee. Not a security hole (a stale deny ACE is fail-secure — it can only keep denying) but a
  correctness/cleanup-contract break: every per-session synthetic SID's deny ACE would accumulate
  on the sink directory's DACL forever, one per session, never revoked.
- **Fix:** Rewrote `revoke_sid_on_path` to manually walk the DACL (`GetAce` in reverse index
  order) and call `DeleteAce` on every entry whose `ACE_HEADER::AceType` is
  `ACCESS_ALLOWED_ACE_TYPE` or `ACCESS_DENIED_ACE_TYPE` and whose trustee SID matches, then
  writes the mutated-in-place DACL back via `SetNamedSecurityInfoW` — the same three-step
  read/mutate/write shape as `edit_dacl_for_sid`, but without delegating to
  `SetEntriesInAclW(REVOKE_ACCESS)`.
- **Files modified:** `crates/nono/src/sandbox/windows.rs`
- **Commit:** d0aa6a55

**2. [Rule 1 - Bug] `dacl_contains_sid` test helper silently matched DENY ACEs too**

- **Found during:** Task 1, same round-trip test — the deny-ACE perturbation-proof assertion
  ("a deny ACE must not satisfy the ALLOW-only reader") initially failed because the helper never
  checked `AceType` before reading `SidStart` (both `ACCESS_ALLOWED_ACE` and
  `ACCESS_DENIED_ACE` share that offset). Its own doc comment claimed "we only ever add
  allow-ACEs" — true only because no test in this module had ever added a deny ACE before.
- **Fix:** Added an explicit `AceType == ACCESS_ALLOWED_ACE_TYPE` check before the `SidStart`
  cast, making the helper genuinely allow-only (matching what every existing grant-test assertion
  already implicitly assumed).
- **Files modified:** `crates/nono/src/sandbox/windows.rs`
- **Commit:** d0aa6a55

**3. [Rule 1 - Bug fix, downstream compile] Three pre-existing exhaustive `MachineEgressPolicy`
struct literals**

- **Found during:** Task 2, after adding the `require_receipts` field — `cargo build` failed on
  three test-module struct literals that listed every field explicitly (no `..Default::default()`).
- **Fix:** Added `require_receipts: false`/`true` to
  `is_unconfigured_ignores_telemetry_field`, `is_unconfigured_ignores_required_layers_field`, and
  `policy_serde_round_trip_with_telemetry`, so they keep compiling and exercise the new field
  rather than being silently skipped.
- **Files modified:** `crates/nono/src/machine_policy.rs`
- **Commit:** b0de6661

No other deviations. Both `must_haves.artifacts` and `key_links` in the plan frontmatter are
satisfied verbatim (verified via grep below).

## Verification Evidence

```
$ grep -n "pub fn deny_sid_on_path" crates/nono/src/sandbox/windows.rs
1959:pub fn deny_sid_on_path(path: &Path, sid: &str, access_mask: u32) -> Result<()> {
(exactly 1 match)

$ grep -c "DENY_ACCESS" crates/nono/src/sandbox/windows.rs
4   (was 0 before this task)

$ cargo test -p nono-sandbox --lib sandbox::windows -- deny_sid
test result: ok. (deny_then_revoke_sid_round_trips_on_tempdir, deny_invalid_sid_fails_closed
both pass; non-zero test count)

$ grep -c "require_receipts\|RequireReceipts" crates/nono/src/machine_policy.rs
62  (field, doc, read function, 7 test functions — well over the required 3)

$ cargo test -p nono-sandbox --lib machine_policy -- require_receipts
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 818 filtered out
(includes windows_require_receipts_malformed_type_aborts_with_policy_load_failed, which
asserts matches!(err, NonoError::PolicyLoadFailed { .. }) — mirrors the pattern at
machine_policy.rs's pre-existing egress-field tests, NOT the RequiredLayersPolicy::default()
degrade pattern)

$ cargo test -p nono-sandbox --lib
test result: ok. 862 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
(was 856 before this plan; +6 net after the 2 deny tests + revoke-doc-fix consolidation + 6
require_receipts tests = 862; no regressions anywhere in the crate)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 06s   (clean, no errors)

$ cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used
Finished `dev` profile [unoptimized + debuginfo] target(s) in 53.56s   (clean)

$ cargo fmt --check -p nono-sandbox
(no output — clean)
```

## Cross-Target Clippy Gate Scope (Plan 118-10)

Both files touched by this plan (`crates/nono/src/sandbox/windows.rs`,
`crates/nono/src/machine_policy.rs`) carry `#[cfg(target_os = "windows")]` blocks (and
`windows.rs` additionally sits under `crates/nono/src/sandbox/`, a directory whose sibling
`linux.rs`/`macos.rs` files are the Unix cfg branches the CLAUDE.md rule is protecting). Per
CLAUDE.md's cross-target clippy rule, **both files are in scope** for Plan 118-10's mandatory
`cross clippy --target x86_64-unknown-linux-gnu` and `cargo-zigbuild clippy --target
x86_64-apple-darwin` gates. This plan did not run those gates itself (deferred to 118-10 per the
phase's stated aggregation point) but ran the Windows-host equivalents
(`cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings
-D clippy::unwrap_used`, `cargo fmt --check`) as a fast local pre-check, all clean. Neither file's
new code is itself behind a `#[cfg(target_os = "linux")]`/`#[cfg(target_os = "macos")]` branch —
`deny_sid_on_path` is `#[cfg(target_os = "windows")]`-gated (compiles out entirely on
Linux/macOS) and `require_receipts` is a plain `bool` field on an already-platform-neutral struct
(compiles on every target; only the *reader* is Windows-gated) — so the Unix-target gates are
expected to be a structural no-op for this plan's diff, but Plan 118-10 must still run them
per the MUST/NEVER rule (no self-certification via Windows-host tooling).

## Self-Check: PASSED

```
$ [ -f crates/nono/src/sandbox/windows.rs ] && echo FOUND || echo MISSING
FOUND: crates/nono/src/sandbox/windows.rs

$ [ -f crates/nono/src/machine_policy.rs ] && echo FOUND || echo MISSING
FOUND: crates/nono/src/machine_policy.rs

$ git log --oneline --all | grep -q "d0aa6a55" && echo FOUND || echo MISSING
FOUND: d0aa6a55

$ git log --oneline --all | grep -q "b0de6661" && echo FOUND || echo MISSING
FOUND: b0de6661
```

## Commits

- `d0aa6a55` — feat(118-02): add deny_sid_on_path DACL-deny primitive (D-08)
- `b0de6661` — feat(118-02): add require_receipts fleet-wide machine policy field (D-04)
