---
phase: 111-core-carry-resource-cli-verify-release-leapfrog
reviewed: 2026-08-05T02:55:40Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - crates/nono-cli/data/policy.json
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono-cli/src/policy.rs
  - docs/cli/usage/flags.mdx
  - proj/ADR-111-resource-limits-boundary.md
  - scripts/gates/release-readiness.ps1
  - scripts/release-dry-run.ps1
  - crates/nono/Cargo.toml
  - crates/nono-cli/Cargo.toml
  - crates/nono-proxy/Cargo.toml
  - crates/nono-shell-broker/Cargo.toml
  - crates/nono-fltmgr-client/Cargo.toml
  - bindings/c/Cargo.toml
findings:
  critical: 1
  warning: 2
  info: 3
  total: 6
status: issues_found
---

# Phase 111: Code Review Report

**Reviewed:** 2026-08-05T02:55:40Z
**Depth:** standard
**Files Reviewed:** 13 (+ CLAUDE.md as review baseline)
**Status:** issues_found

## Summary

Reviewed the actual `git diff c5c8c5fc..HEAD` for every file listed under CORE-01 (policy.json
cache grant + fork-safety threshold), CORE-02 (`--memory`/`--timeout`/`--max-processes` help-text
corrections + ADR-111), and RLS-14 (six `Cargo.toml` version bumps 0.66.1 → 0.70.0 + the
`release-dry-run.ps1` / `release-readiness.ps1` release-gate scripts).

Six of the eight changed items are narrow, well-justified, and match the code they describe:
the six version bumps are internally consistent (workspace member `version =` and every
`path`-dependency's paired `version = "0.70.0"` constraint move together — no crate is left on a
stale pin), the corrected `--memory`/`--cpu-percent`/`--max-processes` help text in `cli.rs` and
`flags.mdx` accurately reflects the enforcement code in `exec_strategy.rs` (including correctly
disclosing that macOS `--memory` is best-effort and can silently continue past an `EINVAL`
`setrlimit` failure — this does **not** overstate the guarantee), the `~/.cache` policy.json
addition doesn't overlap any existing `deny.access` rule, and ADR-111 is a self-consistent
documentation-only artifact.

One finding is classified **Critical**: the widened `PRE_PUBLISH_REGISTRY_BLOCKED` detection
regex in `scripts/release-dry-run.ps1` is unscoped to the specific expected missing package name
and will reclassify *any* `cargo publish --dry-run` "no matching package named" failure —
including one caused by an unrelated dependency typo or a genuinely broken/yanked crate — as a
non-blocking, expected pre-publish state. Because this script gates an irreversible crates.io
publish, a false negative here is a real risk, not a cosmetic one.

Two **Warning**-level findings concern quality/auditability regressions: the `MAX_CRYPTO_THREADS`
fork-safety threshold was raised from a precisely-justified 7 to an approximately-justified 12,
applied uniformly to both Linux and macOS even though the stated rationale (macOS
Security.framework libdispatch workqueue threads) is macOS-specific; and the corrected
`--timeout` help text in `cli.rs` conflates the Linux (`cgroup.kill`) and macOS
(`kill(-pgrp, SIGKILL)` + `setpgid(0,0)`) enforcement mechanisms into a single inaccurate
cross-platform description.

## Critical Issues

### CR-01: Unscoped "no matching package named" regex silently masks genuine packaging errors in the release gate

**File:** `scripts/release-dry-run.ps1:97-101`
**Issue:**

The `PRE_PUBLISH_REGISTRY_BLOCKED` detection was widened this phase to also match the literal
substring `'no matching package named'` anywhere in `cargo publish --dry-run`'s combined
stdout/stderr, in addition to the pre-existing `'failed to select a version for the requirement'`
match:

```powershell
if ($outStr -match 'failed to select a version for the requirement' -or `
    $outStr -match 'no matching package named') {
    Write-Host "  PRE_PUBLISH_REGISTRY_BLOCKED" -ForegroundColor Yellow
    Add-Result "crates.$crate" "PRE_PUBLISH_REGISTRY_BLOCKED" `
        "nono-sandbox ^0.70.0 not yet on crates.io; re-run after publishing nono-sandbox"
}
```

Neither pattern is anchored to the specific missing package name (`nono-sandbox`). Cargo emits
the exact same `"no matching package named `<name>` found"` phrasing for **any** unresolved
dependency — a typo'd crate name in `Cargo.toml`, a crate that was yanked/removed from
crates.io, or an accidental version bump referencing a package that doesn't exist. All of these
are genuine packaging defects that this script exists to catch before an irreversible
`cargo publish` (crates.io publishes cannot be un-published, only yanked).

With this change, any such genuine error is silently downgraded from `FAIL` (red, increments
`$HardFailures`) to `PRE_PUBLISH_REGISTRY_BLOCKED` (yellow, does not increment
`$HardFailures`). Per the script's own final-verdict logic (`release-dry-run.ps1:246-260`), the
overall script then reports `PASS` and exits `0` — the release-readiness gate goes green while
masking a real defect. This directly conflicts with CLAUDE.md's "Fail Secure" principle ("On any
error, deny access. Never silently degrade to a less secure state.") applied to the release
pipeline: an ambiguous/overly-broad match here degrades a hard gate into a soft one.

**Fix:** Scope the match to the specific expected missing package name instead of matching the
error class generically. For example:

```powershell
$missingPackagePattern = 'no matching package named `nono-sandbox(-proxy|-cli)?`'
if ($outStr -match 'failed to select a version for the requirement.*nono-sandbox' -or `
    $outStr -match $missingPackagePattern) {
    ...
}
```

or, more robustly, parse the package name out of the matched error text and assert it is exactly
one of the crates this workspace publishes (`$PublishableCrates`) before treating it as expected.
Any other missing/unresolvable package name must fall through to the existing `FAIL` branch.

## Warnings

### WR-01: MAX_CRYPTO_THREADS raised from an exact to an approximate bound, and applied to both platforms despite a platform-specific rationale

**File:** `crates/nono-cli/src/exec_strategy.rs:164-171`
**Issue:**

```rust
/// Maximum threads allowed when crypto library thread pool is active.
/// Main thread (1) + tokio proxy workers (2) + aws-lc-rs ECDSA pool (4), plus
/// headroom for the OS-managed libdispatch workqueue threads that
/// Security.framework spawns for `SecTrustSettings*` XPC during proxy CA
/// setup on macOS (2-5 observed, scales with load). Those workqueue threads
/// are unnamed, parked, and fork-safe, but a tighter budget intermittently
/// tripped on them (issue: fork thread-count flake).
const MAX_CRYPTO_THREADS: usize = 12;
```

This constant gates the pre-`fork()` thread-count safety check in `execute_supervised`
(`exec_strategy.rs:918-923`, `crates/nono-cli/src/exec_strategy.rs` is compiled for
`not(target_os = "windows")`, i.e. **both** Linux and macOS — see `main.rs:35-36`). The previous
value (7) was derived from an exact accounting: `1 (main) + 2 (tokio) + 4 (aws-lc-rs)`. The new
value (12) is derived from an *observed range* ("2-5 observed, scales with load") of a
macOS-only thread source (Security.framework libdispatch workqueue), rounded up with headroom.
Two concerns:

1. The new bound is not a hard accounting — "scales with load" means the actual worst case is
   unenumerated. If load characteristics change (e.g., a future dependency bump spawns more
   XPC-driven threads), 12 may again prove insufficient, and the fix will likely be another ad
   hoc bump rather than a structural one.
2. The constant is not platform-gated, so the widened tolerance also applies on Linux, where the
   stated justification (macOS Security.framework/XPC dispatch workers) does not exist. This
   silently loosens the Linux fork-safety tripwire — a thread count of 8-12 on Linux would now be
   accepted as `CryptoExpected` where it previously would have failed closed with
   `NonoError::SandboxInit`, even though nothing in the Linux threading model justifies the wider
   band.

Fork-safety validation exists specifically to prevent forking with threads that might hold an
allocator lock (the child immediately calls `Sandbox::apply()`, which allocates). Loosening this
check with an approximate, cross-platform-shared bound weakens a defense-in-depth control without
tightening the platform scoping that would keep the change contained to its actual justification.

**Fix:** Split the constant per-platform (e.g. `MAX_CRYPTO_THREADS_LINUX = 7`,
`MAX_CRYPTO_THREADS_MACOS = 12`, selected via `#[cfg(target_os = ...)]`) so the Linux tripwire
stays at its precisely-accounted value while only macOS gets the wider, load-scaling tolerance.

### WR-02: `--timeout` help text conflates the Linux and macOS enforcement mechanisms

**File:** `crates/nono-cli/src/cli.rs:2804-2810`
**Issue:**

```rust
/// On Linux and macOS: enforced via a supervisor-side `Instant` deadline plus
/// `kill(-pgrp, SIGKILL)` (process-group kill), requiring a best-effort,
/// non-fatal `setpgid(0,0)` from the child post-fork.
```

The actual implementation (`exec_strategy.rs:1793-1828`, own inline comment: *"Linux: writes
`"1\n"` to `cgroup.kill` at deadline (atomically kills all descendants). macOS: sends SIGKILL to
child process group at deadline."*) uses two structurally different kill mechanisms:

- **Linux** (`spawn_linux_timeout_watchdog`, `exec_strategy.rs:126-143`): writes `"1"` to the
  cgroup v2 `cgroup.kill` control file — an atomic, cgroup-scoped multi-process kill. It does
  **not** call `kill(-pgrp, ...)` and does **not** depend on `setpgid`.
- **macOS** (`supervisor_macos::spawn_macos_timeout_watchdog`): sends `SIGKILL` to the child's
  process group, which is why `setpgid(0,0)` (child-side, `exec_strategy.rs:1105-1126`) and the
  parent-side double-`setpgid` race-closer (`exec_strategy.rs:1773-1791`) exist — both are gated
  `#[cfg(target_os = "macos")]` only.

The corrected help text presents the macOS-specific `kill(-pgrp, SIGKILL)` + `setpgid(0,0)`
mechanism as if it applies to Linux as well. This doesn't overstate the *security guarantee*
(both mechanisms are kernel-enforced and effective), but it is a factual inaccuracy in
user-facing, security-relevant documentation — exactly the kind of implicit/inexact description
CLAUDE.md's "Explicit Over Implicit" principle calls out ("Security-relevant behavior must be
explicit and auditable").

**Fix:**

```rust
/// On Linux: enforced via a supervisor-side `Instant` deadline that writes to the
/// cgroup v2 `cgroup.kill` control file, atomically killing every process in the
/// cgroup tree. On macOS: enforced via a supervisor-side `Instant` deadline plus
/// `kill(-pgrp, SIGKILL)` (process-group kill), requiring a best-effort,
/// non-fatal `setpgid(0,0)` from the child post-fork.
```

## Info

### IN-01: `~/.cache` readwrite grant widens the macOS `user_caches_macos` group without subpath scoping

**File:** `crates/nono-cli/data/policy.json:328-332`
**Issue:** The `user_caches_macos` group's `allow.readwrite` list gained `~/.cache` (in addition
to the pre-existing `~/Library/Caches` and `~/Library/Logs`). This group is reachable by the
`claude-code`, `claude-no-kc`, and `swival` built-in profiles. Verified: no existing
`deny.access` rule in policy.json targets any path under `~/.cache` on macOS, so this does not
punch a hole through a deny rule. The new regression test
(`crates/nono-cli/src/policy.rs:1742-1762`) only re-asserts the JSON content, not the absence of
deny overlap.

This is a legitimate least-privilege consideration even though it mirrors an upstream fix
(#1378, cited in the new test's comment: "some macOS tools (uv, Corepack) write to `~/.cache`
despite the platform convention being `~/Library/Caches`") — it grants readwrite to the entire
`~/.cache` subtree for three profiles rather than the specific subdirectories those tools
actually use (e.g. `~/.cache/uv`, `~/.cache/corepack`). Not a blocker since it brings macOS to
parity with the existing Linux `user_caches_linux` group (which already grants the whole
`~/.cache`), but worth tracking if a future audit wants tighter per-tool scoping on macOS.

**Fix:** No action required to ship; consider narrowing to specific subdirectories
(`~/.cache/uv`, `~/.cache/corepack`, etc.) in a follow-up if the broader grant proves too coarse
in practice.

### IN-02: New `max_crypto_threads_raised_to_12` test is tautological

**File:** `crates/nono-cli/src/exec_strategy.rs:4383-4386`
**Issue:**

```rust
#[test]
fn max_crypto_threads_raised_to_12() {
    assert_eq!(MAX_CRYPTO_THREADS, 12);
}
```

This test asserts that a constant equals the literal value it was just declared with — it will
only ever fail if someone edits the constant, at which point the test's own assertion also needs
editing. It provides no coverage of the actual fork-safety behavior the constant gates (e.g. that
`execute_supervised` correctly accepts 12 threads under `CryptoExpected` and rejects 13). It
documents intent but does not regression-protect behavior.

**Fix:** Either drop the test (the doc comment above the constant already records the intent), or
replace it with a behavioral test that exercises the `ThreadingContext::CryptoExpected` match arm
at the boundary (11 accepted, 12 accepted, 13 rejected) if the threading count can be
faked/injected in tests.

### IN-03: `docs/cli/usage/flags.mdx` "Resource Limits" section still has no `--timeout` subsection

**File:** `docs/cli/usage/flags.mdx:1232-1264`
**Issue:** This phase corrected the `--cpu-percent`, `--memory`, and `--max-processes`
subsections under "Resource Limits" (and the shared intro paragraph), and separately corrected
`--timeout`'s doc comment in `cli.rs`. However, `flags.mdx` has never had a `--timeout`
subsection under "Resource Limits" (confirmed: no `#### \`--timeout\`` heading anywhere in the
file; the only `--timeout` references are for the unrelated `nono learn --timeout` flag). This
is a pre-existing gap, not introduced by this phase, but the phase touched the immediately
surrounding text three times without closing it, and the `cli.rs` doc comment for `--timeout` was
updated in the same commit — the natural place to add the missing user-facing doc entry.

**Fix:** Add a `#### \`--timeout\`` subsection to `flags.mdx` alongside the other three
resource-limit flags, mirroring the corrected `cli.rs` doc comment (once WR-02 above is also
fixed, to avoid porting the same inaccuracy into the docs site).

---

_Reviewed: 2026-08-05T02:55:40Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
