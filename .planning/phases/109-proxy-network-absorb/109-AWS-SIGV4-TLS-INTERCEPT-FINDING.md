# Phase 109: #1430 / #1437 N/A Disposition Finding

**Written:** 2026-07-29 (Plan 109-05, Task 1)
**Status:** Verified N/A — target subsystem absent from the fork

## Summary

`6fb7ecbf` (#1430, SigV4 encoded-URI generation fix) and `23d93fc9` (#1437, sibling-route
cross-deny fix) — both dispositioned `adopt` in the Phase 108 divergence ledger and both named in
ROADMAP Phase 109 SC3 and REQUIREMENTS.md NET-03 as items this phase "absorbs" — **cannot be
absorbed as code changes**. Each patches a file that does not exist anywhere in this fork. This is
independently re-verified below against the tree as it stands after Plans 109-01 through 109-04
(i.e. after `deny_domain`, both halves of `no_proxy`, and `HTTP_PROXY` forward-proxying landed),
not merely re-stated from 109-CONTEXT.md's planning-time finding.

This is the fourth instance of the D-11 failure mode this milestone (109-CONTEXT.md's
`<specifics>` names three prior instances: the Phase 108 `tool-sandbox` substring-glob miss, and
two "already present" misses on `#1415`/`#1335` from identifier-name matches). All four instances
share the same shape: something in the upstream ledger's description or file list matched a fork
identifier or file path, and that match was mistaken for feature presence, without confirming the
matched code actually implements the named behavior. Here the miss ran the other direction —
Phase 108's ledger dispositioned both commits `adopt` from their upstream file paths alone,
without checking whether the fork's tree has those paths at all.

## Re-verification evidence (execution-time, 2026-07-29)

### 1. Target files confirmed absent

```
$ ls crates/nono-proxy/src/
audit.rs config.rs connect.rs credential.rs diagnostic.rs error.rs external.rs
filter.rs lib.rs oauth2.rs pool.rs reverse.rs route.rs server.rs token.rs

$ ls crates/nono-proxy/src/aws
ls: cannot access 'crates/nono-proxy/src/aws': No such file or directory

$ ls crates/nono-proxy/src/tls_intercept
ls: cannot access 'crates/nono-proxy/src/tls_intercept': No such file or directory
```

`Glob crates/nono-proxy/src/aws/*.rs` and `Glob crates/nono-proxy/src/tls_intercept/*.rs` both
return no files (glob tooling timed out on this host during this run; the `ls` directory listing
above is the equivalent affirmative evidence — the full `src/` listing enumerates every file that
exists in the crate, and neither `aws/` nor `tls_intercept/` is a subdirectory of it).

`6fb7ecbf`'s sole target is `crates/nono-proxy/src/aws/sign.rs` — absent.
`23d93fc9`'s sole target is `crates/nono-proxy/src/tls_intercept/handle.rs` — absent.

### 2. #1430 (SigV4 encoded-URI fix) — fork has no signing implementation to patch

`crates/nono-proxy/src/credential.rs` (`CredentialStore`), lines 131-134:

```rust
/// Map from route prefix to AWS SigV4 route (placeholder until full
/// SigV4 signing is implemented; value is () because no runtime state
/// is needed yet).
aws_routes: HashMap<String, ()>,
```

`get_aws()` (line 267): `pub fn get_aws(&self, prefix: &str) -> Option<&()>` — returns a
placeholder unit type, not a signing route. Unchanged by Plans 109-01 through 109-04 (none of
those plans' `key-files: modified` lists include `credential.rs`).

`crates/nono-proxy/src/reverse.rs`, lines 260-268, still present verbatim:

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

This is a pre-existing, already-decided fork posture (D-15) from a prior phase — not new
information introduced by this plan, and not touched by 109-01..04.

Signing-algorithm grep, re-run against the full post-absorb crate:

```
$ grep -rniE "hmac|sha256|canonical_request|StringToSign" crates/nono-proxy/src
No matches found. (0 hits across 0 files)
```

Zero hits confirms there is no SigV4 signing algorithm implementation anywhere in the crate for
`6fb7ecbf`'s 20-line `PercentEncodingMode`/`UriPathNormalizationMode` fix to apply to.

**Disposition: N/A — target subsystem (`aws/sign.rs`, full SigV4 signing) absent, verified.**

### 3. #1437 (sibling-route cross-deny fix) — fork has no multi-route TLS-intercept dispatch

`crates/nono-proxy/src/credential.rs`, lines 104-107, doc comment on `CredentialStore`:

```rust
/// 2. **2-match-deny** — structurally impossible in this store. The
///    upstream "ambiguous selection" case only arises when multiple
///    routes share an upstream host AND the inner-request path can
///    match more than one route's `endpoint_rules`. Both preconditions
```

`crates/nono-proxy/src/server.rs`, lines 560-564:

```rust
// Combines webpki roots with the OS trust store via the shared
// `route::build_base_root_store()` helper. This replays the security
// intent of upstream 8ddb143 (TLS trust on corporate networks with
// MITM inspection) WITHOUT pulling in the tls_intercept module
// (per D-40-B2 fork-preserve lock; fork has no tls_intercept).
```

Both comments independently confirm the same architectural fact and are unchanged by Plans
109-01 through 109-04 (`credential.rs` and this section of `server.rs` are outside every plan's
`key-files: modified` list; Plan 109-02/109-04 touch other parts of `server.rs` — the `no_proxy`
pipeline and forward-proxy dispatch respectively — not this TLS-trust section).

The fork's actual route dispatch, re-confirmed:

```rust
// crates/nono-proxy/src/route.rs:85-86
pub struct RouteStore {
    routes: HashMap<String, LoadedRoute>,
}
// crates/nono-proxy/src/route.rs:161
pub fn get(&self, prefix: &str) -> Option<&LoadedRoute> {
```

Single-key `HashMap` lookup by path prefix — no "first route by prefix order among several
matches" concept exists, so the sibling-route ambiguity `#1437` fixes (`select_intercept_route`'s
`EndpointPolicyOutcome::Deny` arm treating non-explicit default-deny as "route does not apply"
rather than terminal 403) has no fork equivalent to be ambiguous in.

**Disposition: N/A — target subsystem (`tls_intercept/handle.rs`, multi-route TLS-intercept
dispatch) absent, verified.**

## ROADMAP Phase 109 SC3 satisfaction statement

SC3 names `#1430` and `#1437` as items this phase "absorbs." This finding record satisfies that
criterion via **confirmed non-applicable, with evidence** rather than a code change — the same
discipline the Phase 108 ledger uses when it records a `none` requirement-mapping disposition with
recorded reasoning instead of silent omission (see e.g. `c344efb0`/`4192bfa5` rows, dispositioned
`adopt` with mapping `none` and an explicit one-line reason each). SC3 is not satisfied by *not
mentioning* these two items; it is satisfied by this document existing and being reviewable.

Implementing either missing subsystem (full AWS SigV4 signing in a new `aws/sign.rs`, or
TLS-intercept multi-route dispatch in a new `tls_intercept/handle.rs`) is explicitly **out of this
phase's scope**. Either would be a net-new feature on the order of NET-02's SPIFFE/SPIRE absorb
(already split to Phase 113 for the identical reason: it is a refactor/introduction of a subsystem
shape the fork does not have), not a 16-line or 87-line bug-fix absorb. If either is ever
prioritized, it needs its own future phase with its own scope, design review, and threat model —
this document is a disposition record, not a feature-implementation deferral or a commitment to
build either subsystem.

## Second correction proposal: Phase 108 ledger disposition

The Phase 108 divergence ledger (`108-DIVERGENCE-LEDGER.md`) dispositions both `6fb7ecbf` (line
686) and `23d93fc9` (line 687) as `adopt`. Per this finding, both are wrong and should be corrected
to **`won't-sync (target subsystem absent)`** — the identical disposition the ledger already uses
elsewhere for the tool-sandbox refinements (per 109-CONTEXT.md's own note: "the same disposition
the tool-sandbox refinements got, and for the same reason"). This plan does not edit
`108-DIVERGENCE-LEDGER.md` (out of this plan's `files_modified` scope and outside the
`.planning/phases/109-proxy-network-absorb/` directory this plan owns) — the correction is
proposed here for the operator/a future ledger-maintenance pass to apply.

**Proposed ledger row corrections** (`108-DIVERGENCE-LEDGER.md`, "NET Cluster — Per-Commit Table"):

| sha | current disposition | proposed disposition |
|-----|---------------------|----------------------|
| `6fb7ecbf36e5d18760d8084b7f4900e93582004c` | `adopt` | `won't-sync (target subsystem absent)` |
| `23d93fc96abf795d672c712e9c1834a8f97aa0aa` | `adopt` | `won't-sync (target subsystem absent)` |

## REQUIREMENTS.md NET-03 correction proposal

**This plan does NOT edit `.planning/REQUIREMENTS.md`.** The correction below is proposed only,
for operator approval, mirroring Phase 108's own precedent of flagging a ROADMAP SC4 amendment for
a future phase (Phase 112) rather than silently leaving an unsatisfiable success criterion in
place, and rather than editing ROADMAP.md/REQUIREMENTS.md itself from within the phase that found
the problem.

**Current NET-03 wording** (`.planning/REQUIREMENTS.md` line 115, verbatim):

> The SigV4 encoded-URI generation fix (#1430) and the sibling-route cross-deny fix (#1437) are
> absorbed; HTTP/2 injection, `HTTP_PROXY` forward-proxy, and `no_proxy` bypass are verified
> non-regressed; `maturin` + `napi` binding builds are green.

After this phase's execution, that wording is contradicted by the verified-N/A finding above:
`#1430` and `#1437` are not, and cannot be, "absorbed" as code changes.

**Proposed corrected wording:**

> The SigV4 encoded-URI generation fix (#1430) and the sibling-route cross-deny fix (#1437) are
> confirmed non-applicable — both target upstream subsystems (`aws/sign.rs` full SigV4 signing,
> `tls_intercept/handle.rs` multi-route TLS-intercept dispatch) absent from the fork's
> architecture, per `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`; `no_proxy` bypass and `HTTP_PROXY`
> forward-proxy are verified non-regressed; `maturin` + `napi` binding builds are green.

(Note: this plan's Task 2 found no HTTP/2-injection-specific code touched by this phase's actual
changes — the original NET-03 wording's "HTTP/2 injection" clause traces to `726ac1f1`'s upstream
commit message context, not a fork-verified behavior distinct from the forward-proxy path Plan
109-04 already covers. The proposed wording above drops that clause as redundant with "HTTP_PROXY
forward-proxy... verified non-regressed" rather than leaving an unverifiable claim in place; the
operator may restore it verbatim if a distinct HTTP/2 injection check is intended.)

This correction requires operator approval before being applied to `REQUIREMENTS.md` — it is not
applied by this plan or this finding document.

## Conclusion

Phase 109 absorbs 3 of the 5 NET-cluster commits assigned to it as real code changes
(`3b207eeb` deny_domain / NET-01, `1619275c` no_proxy / NET-03, `726ac1f1` HTTP_PROXY
forward-proxy / NET-03). The remaining 2 (`6fb7ecbf` #1430, `23d93fc9` #1437) are verified N/A —
target subsystems absent from the fork — and recorded here as a reviewable finding rather than
silently dropped or falsely claimed as absorbed. Two corrections are proposed for operator
approval: the Phase 108 ledger's `adopt` -> `won't-sync (target subsystem absent)` disposition for
both commits, and REQUIREMENTS.md's NET-03 wording.
