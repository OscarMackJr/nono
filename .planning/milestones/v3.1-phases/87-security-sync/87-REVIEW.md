---
phase: 87-security-sync
reviewed: 2026-06-20T03:46:01Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - crates/nono/src/sandbox/linux.rs
  - crates/nono/src/capability.rs
  - crates/nono/src/audit.rs
  - crates/nono-cli/src/exec_strategy/supervisor_linux.rs
  - crates/nono-cli/src/exec_strategy.rs
  - crates/nono/src/sandbox/mod.rs
findings:
  critical: 1
  warning: 4
  info: 2
  total: 7
status: resolved
post_review_fix:
  commit: "718fe59d"
  date: "2026-06-20"
  cr_01: resolved
  wr_01: accepted
  wr_02: accepted
  wr_03: accepted
  wr_04: resolved
---

# Phase 87: Code Review Report

**Reviewed:** 2026-06-20T03:46:01Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** resolved (post-review fix `718fe59d` 2026-06-20)

## Summary

Phase 87 ports upstream commit `e2086877` (trap `sendto`/`sendmsg`/`sendmmsg` to
close the AF_UNIX SOCK_DGRAM bypass, issue #1089) plus three fork-specific items:
SEC-01 (multi-sockaddr supervisor dispatch + a new no-grant static-EPERM seccomp
filter, D-01), SEC-02 (guard `deduplicate()` against inheriting procfs-remap
originals), and CR-02 (`records_verified = false` for empty audit logs).

The BPF filter work is carefully done: every jump offset in
`build_seccomp_proxy_filter` (19 → 23 insns), `build_seccomp_af_unix_filter`
(5 → 8 insns), and the new `build_seccomp_af_unix_nogrant_filter` (6 insns) was
recounted and is internally consistent, and each is covered by an offset-asserting
unit test. The SEC-02 dedup guard (`is_procfs_remap_original`) is correctly placed
in both `deduplicate()` branches and correctly delegates to the rewriter so the two
stay in sync. The CR-02 audit fix is correct and well-tested. The arithmetic in
`read_mmsghdr_dests` uses `checked_mul`/`checked_add` and a `MAX_MMSGHDRS` cap as
required.

The blocking issue is the **install condition** for the new no-grant EPERM filter
in `exec_strategy.rs`: it diverges from the plan and over-blocks all datagram sends
(including AF_INET UDP/DNS) for supervised runs in the *default* `af_unix_mediation
= Off` configuration. Secondary findings concern the CONTINUE-after-pointer-read
TOCTOU window the send paths inherit, a best-effort failure mode that silently
weakens the very bypass this phase closes, and an inaccurate SUMMARY claim.

This review could only be performed by inspection: the changed code is cfg-gated
Linux and cannot compile on the Windows dev host.

## Critical Issues

### CR-01: No-grant EPERM filter over-blocks all datagram sends in default config

**Status: RESOLVED** — commit `718fe59d` (2026-06-20)

**Fix:** Extracted `af_unix_send_filter_action(proxy_fallback, mediation, has_unix_grants)
-> AfUnixSendFilterAction` as a pure helper function shared by both child and parent fork
arms. The gate now correctly installs:
  - `NoFilter` in `Off` mode (default, no mediation — CR-01 regression guard)
  - `StaticEperm` in `Pathname` mode with no unix-socket grants (D-01 no-grant path)
  - `UserNotify` in `Pathname` mode with grants, or when proxy_fallback is true

Both child notify-fd condition (`install_network_notify`) and parent recv-fd condition
(`parent_send_action == AfUnixSendFilterAction::UserNotify`) now evaluate the identical
helper, preventing child/parent deadlock. Exhaustive unit tests added for all 8
(proxy, mediation, has_grants) combinations in a `#[cfg(target_os = "linux")]` test
module — run on CI.

**File:** `crates/nono-cli/src/exec_strategy.rs:1294`

**Issue:** The new D-01 no-grant filter is installed under:

```rust
if !config.seccomp_proxy_fallback && !config.af_unix_mediation.is_pathname() {
    // installs install_seccomp_af_unix_nogrant_filter()
}
```

`LinuxAfUnixMediation` is a binary enum — only `Off` (the `#[default]`) and
`Pathname` (`crates/nono-cli/src/profile/mod.rs:1927`). Therefore
`!is_pathname()` is true precisely when mediation is `Off`. Combined with
`!seccomp_proxy_fallback`, this condition fires for **every supervised Linux run
that has neither the proxy fallback nor pathname mediation enabled — i.e. the
default supervised configuration.**

The installed filter (`build_seccomp_af_unix_nogrant_filter`,
`crates/nono/src/sandbox/linux.rs:446`) matches purely on syscall number and
returns `SECCOMP_RET_ERRNO(EPERM)` for `sendto`/`sendmsg`/`sendmmsg` with **no
address-family discrimination**. Consequences for default supervised runs
(rollback / capability-expansion / non-proxy supervised paths):

- All UDP `sendto`/`sendmsg` fail with EPERM even when `NetworkMode::AllowAll` —
  this breaks DNS resolvers that use `sendmsg`/`sendmmsg`, QUIC/HTTP3, NTP, and
  any UDP client.
- `sendmsg` is also used over connected stream and Unix sockets by many libc
  paths; baking a static EPERM denies legitimate connected-socket sends that the
  USER_NOTIF path is specifically designed to fast-path-allow.

This contradicts the library's fail-secure-but-least-surprise contract: a user
who never opted into AF_UNIX mediation gets datagram I/O silently broken.

The plan (`87-01-PLAN.md:219`) specified a different gate:

```rust
if config.af_unix_mediation.is_active() && !config.af_unix_mediation.is_pathname() {
```

i.e. install **only** when mediation is explicitly active *but* no pathname grants
exist. With a binary `Off`/`Pathname` enum, `is_active()` does not exist and
"active" is synonymous with `is_pathname()`, so the plan's literal predicate is
unsatisfiable. Rather than reconcile this (e.g. by adding a third state, or by
gating on "mediation requested AND grant list empty"), the implementation
substituted a predicate that fires in the opposite, far-too-broad case. The
SUMMARY's claim that the "plan executed exactly as written" is incorrect here
(see IN-01).

**Fix:** Gate the no-grant filter on "AF_UNIX mediation was requested but no
pathname grants are present", not on the default `Off` mode. The intended D-01
shape is roughly:

```rust
// Only install when the policy asked for AF_UNIX pathname control but no
// pathname grants resolved (grant-present path uses the USER_NOTIF filter).
if config.af_unix_mediation.is_pathname() && config.unix_socket_grants.is_empty() {
    if let Err(_e) = nono::sandbox::install_seccomp_af_unix_nogrant_filter() {
        // best-effort warn ...
    }
}
```

If the design genuinely intends a distinct "mediation on, no grants" state,
introduce a third `LinuxAfUnixMediation` variant (or pass the resolved grant
count) so the gate is expressible. Either way, the filter must NOT install in the
default `Off` configuration, and — because it cannot distinguish socket families —
it must only be reachable on a code path where the policy has already committed to
blocking unmediated AF_UNIX sends. If non-AF_UNIX datagram traffic must remain
permitted in that mode, the filter needs a family check (route to USER_NOTIF and
inspect `sockaddr` family) instead of a blanket EPERM.

## Warnings

### WR-01: CONTINUE-after-pointer-read reopens a TOCTOU on send destinations

**Status: ACCEPTED** (commit `718fe59d`) — Concise limitation comment added at the
`continue_notif` call site in `supervisor_linux.rs` acknowledging the TOCTOU window,
why CONTINUE is used instead of emulation, and that a single-threaded child makes
exploitation extremely unlikely. The alternative (emulating without CONTINUE) is not
available at this ABI level. See Phase 87 VERIFICATION §Accepted Limitations.

**File:** `crates/nono-cli/src/exec_strategy/supervisor_linux.rs:837-1003`

**Issue:** `handle_network_notification` reads pointer-derived destination
addresses from the child's `/proc/PID/mem` (`sendto` `args[4]`, `sendmsg`
`msghdr.msg_name`, each `sendmmsg` `mmsghdr.msg_name`), decides ALLOW, then calls
`continue_notif()` — which sets `SECCOMP_USER_NOTIF_FLAG_CONTINUE`
(`crates/nono/src/sandbox/linux.rs:1811`). On CONTINUE the kernel re-executes the
syscall and **re-reads the destination address from child memory**. A second
thread sharing the child's address space can swap a permitted `sockaddr_un` path
for a denied one inside that window, defeating the path allowlist for the actual
send.

The library's own documentation explicitly warns against this:
`crates/nono/src/sandbox/linux.rs:2515` — "Callers must not use
`SECCOMP_USER_NOTIF_FLAG_CONTINUE` after authorizing pointer-derived data unless
that TOCTOU window is explicitly acceptable." The `connect`/`bind` paths already
relied on CONTINUE, so this is a pre-existing weakness, but Phase 87 extends the
CONTINUE-allow-after-pointer-read pattern to per-message datagram destinations —
the exact surface the ported upstream fix exists to harden. The single
`notif_id_valid` TOCTOU check (line ~864) only confirms the notification is still
pending; it does not pin the memory the kernel will re-read.

**Fix:** For ALLOW decisions on pointer-derived AF_UNIX destinations, prefer a
non-CONTINUE response that does not let the kernel re-read child memory (e.g.
emulate/validate without CONTINUE, or document and accept the window with a
single-threaded-child precondition). At minimum, add a code comment at each
`continue_notif` call in the send paths acknowledging the inherited TOCTOU and
referencing the `read_notif_sockaddr` doc warning, and confirm the threat model
treats a multi-threaded sandboxed child swapping `msg_name` as out of scope.

### WR-02: No-grant filter failure is best-effort, silently leaving the bypass open

**Status: ACCEPTED** (commit `718fe59d`) — Added a detailed comment at the failure
path explaining: (a) Landlock V4 does NOT backstop AF_UNIX sends, (b) failing closed
(`_exit(126)`) was considered but rejected because kernels with Landlock V4 pathname
support but without seccomp-BPF are exceedingly rare, (c) the stderr warning now
includes "(WR-02: bypass residual risk on this kernel)" so operators aware of the
limitation can act. Documented in VERIFICATION §Accepted Limitations.

**File:** `crates/nono-cli/src/exec_strategy.rs:1301-1313`

**Issue:** If `install_seccomp_af_unix_nogrant_filter()` fails, the child writes a
one-line stderr warning and **continues to exec without the filter**. This is the
fail-secure path for the SOCK_DGRAM bypass that SEC-01 is supposed to close: on
failure the child runs with `sendto`/`sendmsg`/`sendmmsg` unrestricted, i.e. the
bypass remains open while the user believes mediation is active. Contrast with the
proxy/pathname USER_NOTIF installs a few lines above (1271-1284), which `_exit(126)`
on failure. The comment rationalizes this as "defense-in-depth here; Landlock still
applies" — but Landlock V4 has no AF_UNIX send filtering, which is the whole reason
this seccomp filter exists, so Landlock does not in fact backstop it.

(Note: this finding presumes CR-01 is fixed so the filter only runs where AF_UNIX
mediation was actually requested. In that corrected context, failing open defeats
an explicitly requested security control and should fail closed.)

**Fix:** On the *requested-mediation* path, treat install failure as fatal
(`libc::_exit(126)` after the stderr write), matching the proxy/pathname filters.
Reserve best-effort behavior only for paths where the filter is genuinely advisory.

### WR-03: `sendmmsg` mediation cannot enforce partial-success semantics

**Status: ACCEPTED** (commit `718fe59d`) — Added comment at the `continue_notif` call
in `handle_network_notification` documenting the all-or-nothing semantics: USER_NOTIF
CONTINUE cannot be told "send messages 0..k only". The fail-secure behavior (deny on
ANY single denied address) is correct; the accepted limitation is the ALLOW path
sends all messages. Documented in VERIFICATION §Accepted Limitations.

**File:** `crates/nono-cli/src/exec_strategy/supervisor_linux.rs:889-985`

**Issue:** `sendmmsg(2)` sends an array of messages and reports how many were
sent. The handler reads every destination, then applies a deny-on-ANY policy:
if any one message targets a denied address, the whole syscall is rejected with
EACCES (line ~990 region). That is the correct fail-secure choice for *denial*,
but the ALLOW path uses `continue_notif` (CONTINUE), after which the kernel sends
**all** messages. There is no mechanism to allow a prefix and deny the tail, and —
combined with WR-01 — the per-message addresses the supervisor validated are not
the addresses the kernel necessarily re-reads at send time. The net effect is that
`sendmmsg` mediation is all-or-nothing and TOCTOU-exposed, which should be stated
as a known limitation so callers do not assume per-message enforcement.

**Fix:** Document the all-or-nothing semantics on `read_mmsghdr_dests` and in the
handler. If true per-message enforcement is required, the design must avoid
CONTINUE for `sendmmsg` (the kernel cannot be told "send messages 0..k only" via
USER_NOTIF), e.g. by denying multi-destination `sendmmsg` outright when any entry
is a mediated AF_UNIX pathname target.

### WR-04: `read_msghdr_dest` assumes x86_64 `struct msghdr` layout

**Status: RESOLVED** (commit `718fe59d`) — Field offsets and pointer size are now
derived via `core::mem::offset_of!(libc::msghdr, msg_name)`,
`core::mem::offset_of!(libc::msghdr, msg_namelen)`, and
`core::mem::size_of::<usize>()` rather than the hard-coded literal `12`. Two
compile-time `const _: () = assert!(...)` guards verify the field ordering invariants.
Byte extraction copies pointer bytes into a zero-padded `[u8; 8]` buffer, correct on
both LP64 and ILP32 targets. The `#[must_use]` message was also updated to describe the
return value without policy semantics (IN-02 fix).

**File:** `crates/nono/src/sandbox/linux.rs:2660-2708`

**Issue:** `read_msghdr_dest` hard-codes the offsets `msg_name` at byte 0 (8 bytes)
and `msg_namelen` at byte 8 (4 bytes), reading a fixed `MSGHDR_MIN_READ = 12`. The
doc comment says "x86_64 Linux". The crate also builds for `aarch64`
(`SYS_OPENAT`/`SYS_OPENAT2` are defined for `#[cfg(target_arch = "aarch64")]` at
`linux.rs:1110`). On 64-bit Linux `struct msghdr` does begin with
`void *msg_name; socklen_t msg_namelen;` on both x86_64 and aarch64, so the read
is correct there, but there is no `#[cfg(target_arch = ...)]` guard or
`size_of`-based assertion to prevent silent breakage on a 32-bit or padding-
divergent target (and `read_mmsghdr_dests` derives its stride from
`size_of::<libc::mmsghdr>()` while `read_msghdr_dest` uses a literal `12`, so the
two are not derived from the same source of truth).

**Fix:** Derive `msg_namelen`'s offset from the struct layout
(`memoffset::offset_of!` or a `const` computed from field types) rather than a
literal `12`, or add an explicit compile-time guard restricting these helpers to
the LP64 targets whose layout matches, with a clear `compile_error!` otherwise.

## Info

### IN-01: SUMMARY claims "plan executed exactly as written" — it was not

**Status: ACKNOWLEDGED** — The deviation (wrong gate predicate) is now recorded and
corrected by commit `718fe59d`. The SUMMARY.md for plan 87-01 was not retroactively
updated (out of scope for this post-review fix); this REVIEW.md and VERIFICATION.md
serve as the authoritative deviation record.

**File:** `.planning/phases/87-security-sync/87-01-SUMMARY.md:99` (Deviations)

**Issue:** The SUMMARY states "plan executed exactly as written" and lists no code
deviations. The install condition for the no-grant filter was materially changed
from the plan's `is_active() && !is_pathname()` (`87-01-PLAN.md:219`) to
`!seccomp_proxy_fallback && !is_pathname()` (`exec_strategy.rs:1294`) — a
different predicate with opposite triggering behavior (see CR-01). Whether or not
the plan's predicate was satisfiable, this is a code deviation that should have
been recorded and security-reviewed.

**Fix:** Record the deviation in the SUMMARY and reconcile the gate against the
intended D-01 semantics.

### IN-02: Inconsistent `#[must_use]` message wording on send-helper Results

**Status: RESOLVED** (commit `718fe59d`) — `read_msghdr_dest` now uses
`"Result must be checked — None means msg_name was NULL (connected socket)"` and
`read_mmsghdr_dests` uses `"Result must be checked — empty Vec or all-None means all
msg_name fields were NULL"`. Policy semantics ("fast-path allow") removed.

**File:** `crates/nono/src/sandbox/linux.rs:2657` and `:2709`

**Issue:** `read_msghdr_dest` carries
`#[must_use = "Result must be checked — None means connected socket (fast-path allow)"]`
and `read_mmsghdr_dests` carries a similar message. The wording embeds policy
semantics ("fast-path allow") in a library primitive that, per the
library/CLI boundary in CLAUDE.md, should be policy-free. It is harmless but
mildly misleading to future readers of the core crate. Minor.

**Fix:** Reword to describe the return value only (e.g. "None means msg_name was
NULL"), leaving the allow/deny interpretation to the CLI caller.

---

_Reviewed: 2026-06-20T03:46:01Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_Post-review fix: 2026-06-20, commit `718fe59d`_
_CR-01: RESOLVED | WR-01: ACCEPTED | WR-02: ACCEPTED | WR-03: ACCEPTED | WR-04: RESOLVED_
_IN-01: ACKNOWLEDGED | IN-02: RESOLVED_
