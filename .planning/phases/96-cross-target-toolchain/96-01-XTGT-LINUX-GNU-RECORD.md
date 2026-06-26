# 96-01 — linux-gnu cross clippy gate: RECORD

**Phase:** 96-cross-target-toolchain · **Plan:** 01 · **Date:** 2026-06-26
**Requirements:** XTGT-01 (toolchain installed + documented), XTGT-02 (linux-gnu clippy passes; drift fixed in-milestone)
**Result:** ✅ GREEN — gate exits 0 on this Windows dev host; all surfaced cfg-gated Unix drift fixed structurally.

---

## 1. Docker Linux engine precondition (D-01 / Research Pattern 2)

Asserted BEFORE running the gate (Phase 95 trap avoidance — a stopped daemon is an
operator precondition, NOT a documented Docker/cross failure → never deferred to PARTIAL):

```
$ docker info 2>&1 | grep "Server Version"
 Server Version: 29.5.3
```

`docker info` exits 0 with a Server section → precondition satisfied. The engine was
confirmed running for every gate run below.

## 2. Canonical gate command (D-01, verbatim)

```bash
cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
```

`cross` version: **0.2.5**. Subcommand `clippy` runs `cargo clippy` inside the pinned
Linux container (real `x86_64-linux-gnu-gcc`, so `aws-lc-sys`/`ring` C deps link cleanly),
so `-D warnings -D clippy::unwrap_used` behaves identically to a native run.

## 3. Pinned cross image tag (SC#1 — reproducibility)

The literal image string `cross` pulled (authoritative, copied from the build log — NOT
assumed). The Cross.toml stanza does not override `image`, so the default tag is pinned to
the `cross` binary version (0.2.5), and Docker resolved it to a content digest:

```
ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5
  digest: sha256:9e5b39c09874bc1816c675ed11afca2c2ed6cee0c4ed2b3c1d5763c346c9ae3f
```

Full pinned reference (use this to reproduce the exact image):

```
ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5@sha256:9e5b39c09874bc1816c675ed11afca2c2ed6cee0c4ed2b3c1d5763c346c9ae3f
```

Cross.toml `[target.x86_64-unknown-linux-gnu]` pre-build (`libdbus-1-dev` + `pkg-config`)
ran as the `RUN` layer on top of that base image (cached after the first run).

## 4. First-run drift triage (the cfg-gated Unix backlog)

This was the **first-ever local linux-gnu clippy run** against the post-Phase-95 tree. It did
NOT surface a list of clippy lints — it surfaced **hard compile errors** in
`#[cfg(target_os = "linux")]` code that the Windows host never compiles. Root cause: the
Phase 95 absorb of upstream `ae77d198` (PR #1210, "exempt IPC fd from sendmsg trapping")
took the whole-file upstream shape of `supervisor_linux.rs` and the converged `nono`-crate
type/API shapes, **silently dropping fork-specific Linux code and leaving stale call sites**.
Invisible on Windows; caught on the first containerized run — exactly the gate's purpose.

Distinct error classes from the first run(s) (file · class):

| # | Error | Location | Class |
|---|-------|----------|-------|
| 1 | E0432 unresolved import `install_seccomp_af_unix_nogrant_filter` | `crates/nono/src/sandbox/mod.rs:40` | **dropped fork invariant** (SEC-01 AF_UNIX no-grant filter) |
| 2 | E0063 `SupportInfo` missing field `status` (×2) | `crates/nono/src/sandbox/linux.rs:304,314` | converged struct gained `status`; Linux backend not updated |
| 3 | E0433 cannot find `ApprovalRequest` in `supervisor` | `crates/nono-cli/src/exec_strategy/supervisor_linux.rs:450` | converged approval API |
| 4 | E0433 cannot find `cgroup` in `supervisor_linux` (×5) | `crates/nono-cli/src/exec_strategy.rs:69,102,971,974,1061` | **dropped fork invariant** (REQ-RESL-NIX cgroup v2 resource enforcement) |
| 5 | E0599 no method `request_approval` on `&dyn ApprovalBackend` | `supervisor_linux.rs:459` | converged approval API |
| 6 | E0560 `NetworkAuditEvent` missing 17 fields | `supervisor_linux.rs:1145–1162` | converged (slimmed) audit struct; stale construction site |
| 7 | E0599 no method `is_granted` on `ApprovalDecision` | `supervisor_linux.rs:505` | converged approval API |

## 5. Structural fixes applied (D-05 — no silencing of cross-target lints)

All fixes are structural. Dropped fork invariants were recovered **verbatim** from the
last-good pre-absorb commit `ae77d198^`; stale call sites were aligned to the converged API.

1. **SEC-01 AF_UNIX no-grant static-EPERM filter (restored).** Re-added private
   `build_seccomp_af_unix_nogrant_filter()` (6-insn BPF: ld + 3 JEQ + ALLOW + EPERM) + pub
   `install_seccomp_af_unix_nogrant_filter() -> Result<()>` + its unit test to `linux.rs`,
   recovered verbatim from `ae77d198^`. This is the fork-specific no-grant deny path
   (AF_UNIX datagram bypass #1096) that the re-export (`mod.rs:40`) and caller
   (`exec_strategy.rs:1499`) still depend on. **Security-critical regression** — the deny
   filter had been silently dropped on Linux.
2. **`SupportInfo.status` (converged).** Added `status:` to both Linux `support_info()`
   initializers — `SupportStatus::Supported` (Landlock available) / `SupportStatus::NotImplemented`
   (unavailable), mirroring `macos.rs`.
3. **cgroup v2 resource-enforcement module (restored).** Re-appended `pub(super) mod cgroup`
   (`CgroupSession` RAII + `apply_limits`/`install_pre_exec`/`place_self_in_cgroup_raw` +
   tests) to `supervisor_linux.rs`, recovered verbatim from `ae77d198^`. Its callers
   (`UnixResourceLimitGuard::Linux(...)` + `apply_resource_limits_unix`) and its macOS sibling
   (`MacosResourceLimits`) were all intact — only the Linux module had been dropped. Shipped
   REQ-RESL-NIX-01/02 feature.
4. **`NetworkAuditEvent` construction (converged).** Removed 17 dropped, `None`-valued fields
   (`endpoint_policy_*`, `approval_backend`, `credential_capture_*`, `upstream`) from the lone
   Linux-cfg construction site so it matches the slimmed 15-field struct used by every other
   (Windows-visible) site. Zero behavior change — the fields were all `None` and referenced
   nowhere else.
5. **Approval API (converged).** Migrated the seccomp-notify approval path from the
   pre-convergence `ApprovalRequest::Capability { … }` + `request_approval()` + `is_granted()`
   to `nono::CapabilityRequest { … }` (Phase-11 file-path shape) + `request_capability()` +
   `is_approved()` — the same API `exec_strategy.rs` already uses.

**`#[allow]` audit (D-05 compliance):**
- **No** `#[allow]` added to silence a cross-target lint in production code. All fixes are structural.
- One narrow `#[allow(deprecated)]` on the `CapabilityRequest` let-binding: the struct's `path`
  field is `#[deprecated]`-but-required for backward compatibility; this is the API-author-sanctioned
  construction pattern, identical to the existing `terminal_approval.rs` usage. It fires on all
  targets equally (not a cross-target silencer) and is outside the plan's no-new-allow grep set.
- Two `#[allow(clippy::unwrap_used)]` on `#[cfg(all(test, target_os = "linux"))]` modules appear in
  the diff — these are **recovered verbatim** as part of the cgroup module (present in `ae77d198^`),
  in test code, which CLAUDE.md explicitly permits. Not introduced to pass the gate.

## 6. Final green exit (SC#2)

```
$ cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 31.92s
$ echo $?
0
```

**Exit 0. Zero errors, zero warnings under `-D warnings -D clippy::unwrap_used`.**

## 7. Native-target regression check (no native-target regression)

All drift fixes live inside `#[cfg(target_os = "linux")]` modules, so native Windows clippy
cannot compile them — native build is structurally unaffected. Confirmed green anyway:

| Native check | Command | Result |
|--------------|---------|--------|
| rustfmt | `cargo fmt --all -- --check` | exit 0 |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` | exit 0 (`Finished … in 18.69s`) |

Native tests: the 11 nono-cli + 1 nono pre-existing Windows baseline failures
(memory `nono_cli_windows_baseline_test_failures`) are unchanged and unrelated — the restored
cgroup/nogrant tests are `cfg(target_os = "linux")` and do not run on the Windows host. Not a
regression from this plan.

---

*Consumed by Plan 96-03 (checklist/CLAUDE.md rewrite) and by `/gsd:verify-work`.*
