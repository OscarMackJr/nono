# Phase 112: SEC-01 (`0ecc476b`) Won't-Sync Finding — AWS SigV4 Proxy Authentication

**Written:** 2026-08-05 (Plan 112-01, Task 2)
**Status:** Verified won't-sync — target subsystem absent from the fork
**Phase:** 112-security-residual-sync

## Summary

`0ecc476b` (#1195, "feat: implement aws authentication for the MiTM proxy") — the origin commit
that introduces AWS SigV4 signing for `nono-proxy`'s MiTM path — **cannot be absorbed as a code
change**. Of its 11 touched files, the two subsystems it creates/modifies do not exist anywhere in
this fork: `crates/nono-proxy/src/aws/*` (new, 4 files, 846 lines) and
`crates/nono-proxy/src/tls_intercept/{h2_forward,handle}.rs` (modified, 213 lines). This is
re-verified below against the live fork tree as it stands after Phase 111, not merely restated
from `112-RESEARCH.md`'s SEC-01 table row.

`0ecc476b` is the **origin commit** of the same absent `aws/`/`tls_intercept/` subsystem that
`109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` already found absent when it verified `0ecc476b`'s two
later sibling bug-fix commits (`6fb7ecbf` #1430, `23d93fc9` #1437) as `won't-sync (target
subsystem absent)` during Phase 109 (2026-07-29). This finding closes the loop on the origin
commit itself, which Phase 109 did not need to examine because its two commits under review were
downstream fixes, not the feature's introduction.

## Re-verification Evidence

### 1. Diffstat confirmed (11 files, +1,873/-412)

```
$ git show 0ecc476b --stat
commit 0ecc476bf0db3c9509bcc2bd8efae43832c764b8
Author: Anil Kulkarni <6687139+intentionally-left-nil@users.noreply.github.com>
Date:   Wed Jul 1 05:42:56 2026 -0700

    feat: implement aws authentication for the MiTM proxy  (#1195)
    ...

 Cargo.lock                                        | 1033 ++++++++++++++-------
 crates/nono-proxy/Cargo.toml                      |    6 +
 crates/nono-proxy/src/aws/endpoints.rs            |  320 +++++++
 crates/nono-proxy/src/aws/mod.rs                  |   10 +
 crates/nono-proxy/src/aws/route.rs                |  152 +++
 crates/nono-proxy/src/aws/sign.rs                 |  364 ++++++++
 crates/nono-proxy/src/credential.rs               |  131 +--
 crates/nono-proxy/src/lib.rs                      |    4 +
 crates/nono-proxy/src/test_env.rs                 |   44 +
 crates/nono-proxy/src/tls_intercept/h2_forward.rs |   34 +-
 crates/nono-proxy/src/tls_intercept/handle.rs     |  187 +++-
 11 files changed, 1873 insertions(+), 412 deletions(-)
```

### 2. Target subsystems confirmed absent (verbatim `ls` output)

```
$ ls crates/nono-proxy/src/
audit.rs config.rs connect.rs credential.rs diagnostic.rs error.rs external.rs
filter.rs lib.rs oauth2.rs pool.rs reverse.rs route.rs server.rs token.rs

$ ls crates/nono-proxy/src/aws
ls: cannot access 'crates/nono-proxy/src/aws': No such file or directory

$ ls crates/nono-proxy/src/tls_intercept
ls: cannot access 'crates/nono-proxy/src/tls_intercept': No such file or directory
```

`0ecc476b`'s two new/modified subsystems — `aws/{mod,sign,route,endpoints}.rs` (846 of the diff's
~1,185 non-lockfile insertion lines) and `tls_intercept/{h2_forward,handle}.rs` (213 modified
lines) — target directories that are not subdirectories of `crates/nono-proxy/src/` at all. The
full 15-file `src/` listing above enumerates every file the crate has; neither `aws/` nor
`tls_intercept/` is among them.

### 3. `credential.rs`'s placeholder field (unmodified by this or any prior phase)

`crates/nono-proxy/src/credential.rs`, lines 131-134:

```rust
/// Map from route prefix to AWS SigV4 route (placeholder until full
/// SigV4 signing is implemented; value is () because no runtime state
/// is needed yet).
aws_routes: HashMap<String, ()>,
```

The field's type (`HashMap<String, ()>`) is itself evidence of intentional non-implementation —
routes are tracked only well enough to know a route *exists*, with zero payload for any actual
signing state.

### 4. `reverse.rs`'s hand-authored 501 guard citing prior fork decision D-15

`crates/nono-proxy/src/reverse.rs`, lines 260-268:

```rust
// AWS SigV4 signing is not yet implemented. Return 501 so the caller
// knows the route exists but is not functional. This branch will be
// replaced with real SigV4 signing in a follow-up. (D-15 fork adaptation:
// upstream's 501 is in tls_intercept/handle.rs which the fork does not
// have; this is the equivalent guard on the non-TLS proxy path.)
if aws_route.is_some() {
    send_error(stream, 501, "Not Implemented").await?;
    return Ok(());
}
```

This guard predates Phase 112 — it is the fork's own prior, deliberate decision (D-15) to ship a
structural stub (a route that resolves and returns a clean `501`, rather than silently ignoring an
`aws_routes`-configured route or attempting partial signing) instead of building the missing
`tls_intercept/handle.rs`-hosted equivalent upstream has. Neither `credential.rs`'s placeholder nor
`reverse.rs`'s guard has been touched by any commit since (confirmed: no Phase 108-111 SHA touches
either file for this reason, per the same audit discipline `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`
applied).

## Cross-reference: `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`

`.planning/phases/109-proxy-network-absorb/109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` independently
verified (2026-07-29, post Plans 109-01 through 109-04) that `aws/` and `tls_intercept/` were
absent from the fork when it dispositioned `0ecc476b`'s two later sibling bug-fix commits —
`6fb7ecbf` (#1430, SigV4 encoded-URI generation fix) and `23d93fc9` (#1437, sibling-route
cross-deny fix) — as `won't-sync (target subsystem absent)`. Both of those commits patch files
(`aws/sign.rs`, `tls_intercept/handle.rs`) that only exist because `0ecc476b` creates/modifies them
in the first place. This finding establishes that the **origin** commit itself is equally
won't-sync, for the identical underlying reason: the fork made a deliberate, prior decision (D-15)
not to build this subsystem, and nothing in the intervening phases (108 through 111) has changed
that posture.

## Conclusion

**`0ecc476b` (SEC-01) is dispositioned won't-sync (target subsystem absent).** Implementing the
missing `aws/` SigV4-signing subsystem and the `tls_intercept/` multi-route dispatch it depends on
is explicitly out of this phase's scope. A full AWS SigV4 signing implementation is a net-new
feature on the order of NET-02's SPIFFE/SPIRE absorb (already split to its own Phase 113 for the
same underlying reason — a subsystem shape the fork doesn't have) — not a mechanical diff-apply. If
this capability is ever prioritized, it needs its own future phase with its own design review and
threat model, matching the discipline `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` already established
for the sibling commits to this same subsystem.

**This finding corrects `112-CONTEXT.md`'s `<code_context>` summary table.** That table's "Existing
Code Insights" section stated: "All SEC target subsystems exist — there are no free 'target
subsystem absent' passes except the one proven for SEC-09" and listed SEC-01's fork target as
`crates/nono-proxy/src/config.rs`, `crates/nono-cli/src/profile/mod.rs` — "present." That listing
named the wrong target files for SEC-01 (the actual commit's targets are `aws/*` and
`tls_intercept/*`, not `config.rs`/`profile/mod.rs`, which the commit does not touch) and is
superseded by this live-verified absent-subsystem finding. A future reader should treat this
finding document, not that CONTEXT.md table row, as authoritative for SEC-01's disposition.
