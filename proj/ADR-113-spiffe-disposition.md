# ADR-113: SPIFFE/SPIRE Workload-Identity Auth — Absorb Disposition

**Status:** Accepted
**Phase:** 113 — SPIFFE/SPIRE Workload Identity
**Date:** 2026-08-06
**Authors:** Phase 113 execution

---

## Context

Upstream `nolabs-ai/nono` commit `c831dade422f2bdf37d7429af0423cafa0a60c06` (#1272, "feat(proxy):
add SPIFFE/SPIRE workload identity auth for upstream routes") adds JWT-SVID bearer injection and
an RFC 7523 jwt-bearer OAuth2 client-assertion flow to the proxy's reverse-proxy credential
model. Re-measured live against the tree at absorb time: 33 files, +4354/−545, one new direct
dependency (`spiffe 0.16`, `jwt-source` feature), 19 crates in the dependency's own resolved
subtree, 633 `Cargo.lock` lines.

**ROADMAP SC1 is partly stale and this ADR corrects it on the record.** SC1 asks this document to
weigh "the 545-deletion rewrite of the fork's divergent `tls_intercept`/`reverse.rs`" as a single
unit. The fork has **no `tls_intercept` module at all** — `ls crates/nono-proxy/src/` confirms,
and `108-upst12-divergence-audit`/`112-security-residual-sync` independently re-confirmed the same
absence for unrelated reasons (see References). SC1's premise is only half-true: the `reverse.rs`
half of the rewrite is live and real (the fork has this file and it carries the primary SPIFFE
dispatch path); the `tls_intercept` half is superseded entirely by D-01 below — there is no fork
code for that hunk to regress. This ADR records the `reverse.rs`-only reading as the correction to
carry forward.

`113-RESEARCH.md`'s own "⚠ Decision Conflicts" section found the absorb genuinely larger than
`113-CONTEXT.md`'s "~90% lands cleanly" framing suggested: the machinery
`handle_spiffe_route`/`handle_spiffe_assertion_credential` depend on — `CredentialStore`'s
OAuth2-route wiring, `LoadedRoute`'s managed-credential fields, and `reverse.rs`'s
`crate::forward` upstream-connect abstraction — is **also** absent from the fork, tracing not to
the absent `tls_intercept` module but to a separate, older, never-absorbed upstream commit,
`b1ecbc02` (`git describe` = `v0.38.0-3-gb1ecbc02`, pre-`v0.66.0`, outside this milestone's sync
window). This ADR's OD-1 section records that finding as a permanent, named scope boundary.

---

## Decision

**ADAPT-DOWN — D-01 confirmed exactly as locked.** Absorb the ~90% that lands on real fork files;
drop the two `tls_intercept/{h2_forward,handle}.rs` hunks (478 lines total: `h2_forward.rs` +192,
`handle.rs` +286) because the fork has no host module for them to land on.

### D-01's positive proof — no fork route type is left silently unauthenticated

This is not an assertion that the absent files are absent. It is an enumeration of every request
path this fork's proxy actually has, with each path's confirmed SPIFFE disposition, run fresh this
session:

```
$ grep -rn 'tls_intercept' crates/nono-proxy/src/ | wc -l
11
$ grep -rn "mod tls_intercept|struct.*TlsIntercept|fn.*tls_intercept" crates/nono-proxy/src/ | wc -l
0
```

The first count (11) is documentation — every hit is a doc comment or code comment recording that
this fork has no `tls_intercept` module (`route.rs`'s OD-2-corrected divergence note,
`credential.rs`'s D-20 doc, `pool.rs`'s module doc, `server.rs`'s D-40-B2 comment, `reverse.rs`'s
pre-existing AWS SigV4 501 comment) — not code. The second, structural count (0) is the actual
proof: no `tls_intercept` module, struct, or function exists anywhere in the crate.

Every request-entry path this fork's proxy dispatches on, and its SPIFFE disposition:

| Path | Dispatcher | SPIFFE disposition |
|---|---|---|
| Path-prefix reverse-proxy | `reverse.rs::handle_reverse_proxy` | **Implements SPIFFE directly.** `route.has_spiffe_source()` dispatches to `handle_spiffe_route` (direct JWT-SVID bearer injection, Plan 113-06); `credential_store.get_spiffe_assertion()` dispatches to `handle_spiffe_assertion_credential` (RFC 7523 jwt-bearer exchange, Plan 113-06). Both auth-gate via the same `token::validate_proxy_auth` call the pre-existing `static_cred`/no-credential branches use, then fail closed (503, `ManagedCredentialUnavailable`) on any credential-acquisition failure. |
| CONNECT tunnel (bypass-route, non-bypass, and the external-proxy-chain arm) | `server.rs::handle_connection`'s CONNECT dispatch | **Covered by D-03's fail-closed guard.** A pre-existing, unconditional "Block CONNECT tunnels to route upstreams" check (`RouteStore::is_route_upstream`) already runs before `use_external` is computed and before any of the 3 CONNECT arms are selected — proven (Plan 113-05, symbol-level) to be a strict superset of `spiffe_declared_for_upstream` (same `host_port_matches` normalisation, plus an additional `has_spiffe_source()` AND condition). Rather than add three duplicate, provably-unreachable checks inside each arm, the existing check's audit `denial_category` was refined to `SpiffeUnsupportedPath` specifically when the matched route declares SPIFFE. |
| Plain-HTTP forward-proxy | `server.rs::handle_forward_http` | **New D-03 guard, genuinely missing before this phase.** This is the exact WR-13 defect shape from `112-REVIEW.md` (one path of six missing a check the others already had) — but for the SPIFFE-unsupported-path condition, not `require_auth`. Unconditional; not gated on `state.config.require_auth`. |

```
$ grep -c "SpiffeUnsupportedPath" crates/nono-proxy/src/server.rs
8
```

D-03's guard composes with (does not gate on) `require_auth`/`strict_connect_auth` — proven by two
end-to-end tests (`d03_connect_denies_spiffe_declared_route_upstream`,
`d03_forward_http_denies_spiffe_declared_route_upstream`, Plan 113-05) both run with
`require_auth: false`, and the CONNECT-path test additionally configures `external_proxy` to prove
the guard preempts that arm too.

**Contrast with `112-OAUTH-CAPTURE-DISPOSITION.md` (SEC-02), which this ADR is required to
distinguish itself from:** SEC-02's `forward.rs`'s `ResponseRewrite` hook was the SOLE mechanism
keeping real OAuth tokens out of the sandboxed client — dropping it (because `forward.rs` is
absent) would have shipped a half-feature that silently leaks real tokens. This absorb is the
inverse: `tls_intercept/handle.rs`'s `handle_spiffe_intercept_request` was a **sibling**
implementation of the identical enforcement `reverse.rs`'s SPIFFE handlers already provide for the
one request path this fork can reach via TLS interception in the first place — a code path the
fork **cannot reach at all** (no TLS-interception subsystem exists), not a guard removed from a
path it can reach. Dropping it removes an unreachable branch, not an enforcement point.

---

## D-05: Dependency Review

### (a) Full crate enumeration and correction of the "19 new crates" figure

`113-RESEARCH.md`'s D-05 characterized the absorb as pulling "19 crates transitively" via
`spiffe 0.16`'s `jwt-source` feature, based on a static `cargo info spiffe` feature-graph read
performed **before** the dependency was live in `Cargo.lock`. Now that it is live, the actual
`Cargo.lock` diff (`git show 8e78f487 -- Cargo.lock`, the commit that added the dependency) was
re-read directly, and the true picture is more precise and, on the genuinely-new axis, smaller:

**Genuinely new to `Cargo.lock` as a direct result of this absorb (9 packages):** `arc-swap`,
`hyper-timeout`, `prost`, `prost-derive`, `prost-types`, `spiffe`, `tokio-stream`, `tonic`,
`tonic-prost`.

**Already present in `Cargo.lock` before this phase, merely shared/re-used by `spiffe`'s
dependency graph (confirmed via `git log --oneline --all -S "rustls-platform-verifier" --
Cargo.lock`, which traces `rustls-platform-verifier`/`jni` back to this fork's pre-existing
sigstore/Trusted-Signing dependency stack, not to this absorb):** `futures`, `rustls-platform-
verifier`, `jni`, `jni-macros`, `simd_cesu8`, `pin-project`(+`-internal`), `itertools`,
`rand_pcg`, `jsonschema-regex`, `simdutf8`.

This is a genuine correction to `113-RESEARCH.md`'s framing, discovered by direct `git show`/
`git log -S` evidence during this plan's execution, not inherited: `jni 0.21.1` (via
`rustls-platform-verifier 0.6.2` → `ureq` → `nono-sandbox-cli`'s update-check client) predates
this milestone entirely; `jni 0.22.4` (via `rustls-platform-verifier 0.7.0` → `reqwest 0.13.3` →
this fork's sigstore/`ambient-id`/`jsonschema` dev-dependency stack) is unrelated to `spiffe` —
confirmed directly: `cargo tree -p spiffe -e features --target all | grep -i
"rustls-platform-verifier\|jni"` returns **zero output**. `spiffe` reaches the Workload API purely
via `tonic`/`hyper` gRPC; it does not depend on `reqwest` or `rustls-platform-verifier` at all.

The full resolved subtree `spiffe` participates in (`cargo tree -p spiffe --target all`) spans 153
unique crate/version pairs — this is the correct number for "what is in `spiffe`'s supply chain,"
most of which (`tokio`, `hyper`, `serde`, `tower`, etc.) were already load-bearing dependencies of
this workspace before the absorb and are shared, not duplicated.

### (b) `cargo audit` — clean

```
$ cargo audit
...
warning: 6 allowed warnings found
$ echo $?
0
```

6 pre-existing advisory warnings (`async-std` RUSTSEC-2025-0052 unmaintained, `fxhash`
RUSTSEC-2025-0057 unmaintained, `paste` RUSTSEC-2024-0436 unmaintained, `rustls-pemfile`
RUSTSEC-2025-0134 unmaintained, `anyhow` RUSTSEC-2026-0190 unsound, `event-listener`
RUSTSEC-2026-0221 unsound), all pre-dating this absorb (confirmed in `111-04-VERIFICATION-NOTES.md`
before this phase started) and none tracing through the `spiffe`/`tonic`/`prost` dependency tree.
Zero vulnerabilities. Exit code 0.

### (c) JNI/Android cfg-gate — proven closed on all three shipped targets

```
$ cargo tree --target x86_64-pc-windows-msvc -e features -p nono-sandbox-proxy | grep -i jni
(zero output)
$ cargo tree --target x86_64-unknown-linux-gnu -e features -p nono-sandbox-proxy | grep -i jni
(zero output)
$ cargo tree --target x86_64-apple-darwin -e features -p nono-sandbox-proxy | grep -i jni
(zero output)
```

Zero output on all three shipped targets, confirmed fresh this session — the JNI/Android path
never links when building `nono-sandbox-proxy` for Windows, Linux, or macOS.

```
$ cargo tree -i jni@0.22.4 --target all
jni v0.22.4
└── rustls-platform-verifier v0.7.0
    └── reqwest v0.13.3
        └── ... (sigstore/jsonschema-dev-dependency stack, NOT spiffe)
```

The `--target all` reverse-dependency view (which shows every platform's hypothetical resolution,
not what actually links for a given target) confirms the pull path is `rustls-platform-verifier`'s
own `jni = [dep:jni]` opt-in feature — Android-only per `cargo info rustls-platform-verifier` — and
that this feature reaches the tree via `reqwest`/sigstore, not via `spiffe`. `cargo tree -p spiffe
--target all | grep -i "rustls-platform-verifier\|jni"` independently returns zero output,
confirming `spiffe` itself never touches this path. The per-target checks above are the
authoritative proof regardless of `Cargo.lock`'s flat, platform-unfiltered dependency listing.

**D-05 fully discharged: (a) genuine 9-crate new-addition count with the 19-crate figure corrected
and explained, (b) `cargo audit` clean, (c) JNI/Android path proven not linked on all three shipped
targets, with the pull path traced to a pre-existing, `spiffe`-unrelated dependency.**

---

## D-08/SC3: Re-confirming the ADR-86 Boundary Against the Full Diff

`crates/nono/src/undo/types.rs` (+45, Plan 113-01) adds exactly:

- Two enum variants on `NetworkAuditAuthMechanism`: `SpiffeJwtBearer`, `SpiffeOAuthAssertion`.
- One enum variant each on `NetworkAuditInjectionMode` (`SpiffeJwt`) and
  `NetworkAuditDenialCategory` (`SpiffeUnsupportedPath`).
- Two new serde structs, `SpiffeDelegationContext` (`authorized_by`, `on_behalf_of`,
  `chain_depth`) and `SpiffeAuditContext` (`workload_spiffe_id`, `trust_domain`, `svid_type`,
  `source`, `upstream_spiffe_id`, `delegation`) — both `#[derive(Debug, Clone, Serialize,
  Deserialize)]`, no methods beyond derives, no logic.
- One new field, `NetworkAuditEvent.spiffe_context: Option<SpiffeAuditContext>`.

`crates/nono/src/audit.rs` (+2) is two `spiffe_context: None,` test-fixture initializers.

Read directly from the live file this session (`crates/nono/src/undo/types.rs:203-333`): every
addition is a pure-data enum variant or a `#[derive]`-only struct recording *what happened*
(which workload presented a credential, which trust domain, whether a delegation chain was
observed) — zero enforcement, zero policy evaluation, zero decision logic. This matches ADR-86's
own invariant precisely: "Audit and diagnostic modules are observability primitives — they observe
and report on sandbox operations; they do not define security policy, grant permissions, or apply
sandbox restrictions."

**The one honest caveat, carried forward rather than hidden:** this absorb does introduce SPIFFE
*vocabulary* into the policy-free library — `SpiffeJwtBearer`, `SpiffeAuditContext`, and friends
are now identifiers the core `nono` crate's public API surface carries, where before this phase it
carried none. This is a mild concept-leak: a client linking only against `nono` (not `nono-proxy`)
now sees SPIFFE-shaped types in its audit vocabulary even if it never touches the proxy. It does
not cross the enforcement/policy line ADR-86 draws — nothing in these types decides anything — but
it is a real, if small, widening of what the "policy-free" library's public surface *names*, and
this ADR records that honestly rather than asserting the boundary is untouched. **D-08/SC3
confirmed: the boundary holds, with this one named, acknowledged, non-blocking caveat.**

---

## OD-1: The SPIFFE-Only Scope Boundary Against `b1ecbc02` — A Named, Permanent Decision

Mirroring ADR-111's "reject the core-module absorb" shape: this phase deliberately declined to
build the general (non-SPIFFE) OAuth2 `client_credentials` route-wiring layer that upstream's
`c831dade` diff assumes as pre-existing context. That layer — `CredentialStore.oauth2_routes`,
`CredentialStore::get_oauth2()`, `struct OAuth2Route`, `LoadedRoute.requires_managed_credential`/
`.managed_auth_mechanism`/`.managed_injection_mode`/`.missing_managed_credential()`, and
`crate::forward` (the `UpstreamSpec`/`UpstreamStrategy`/`forward_request` upstream-connect
abstraction upstream's diffed `handle_spiffe_route` calls) — does not exist anywhere in this fork.
It was never absorbed from its true origin, upstream commit `b1ecbc02` ("feat(profile): support
OAuth2 auth config in custom_credentials", `git describe` = `v0.38.0-3-gb1ecbc02`), a commit that
predates this milestone's `v0.66.0..v0.69.0` sync window entirely and has never appeared in any
`108-DIVERGENCE-LEDGER.md` cluster.

The fork's own pre-Phase-113 code carried a comment at `route.rs` misattributing this gap to "the
upstream tls_intercept module not present in this fork" — traced via `git log --all
-S"handle_oauth2_credential" -- crates/nono-proxy/src/reverse.rs` on the `upstream` remote to be
factually wrong (that machinery has zero relationship to `tls_intercept`). Plan 113-04 (OD-2)
corrected this comment in place:

```
$ grep -c "b1ecbc02" crates/nono-proxy/src/route.rs
4
$ grep -c "lookup_all_by_upstream|has_intercept_route|requires_managed_credential" crates/nono-proxy/src/route.rs
0
$ grep -c "oauth2_routes|struct OAuth2Route|fn get_oauth2" crates/nono-proxy/src/credential.rs
0
```

**Decision, recorded permanently:** this phase built a SPIFFE-only slice —
`CredentialStore.spiffe_assertion_routes`/`get_spiffe_assertion()`,
`LoadedRoute.managed_auth`/`has_spiffe_source()` — using upstream's diff only as a design-shape
reference, deliberately declining the general `client_credentials` route-wiring layer. This is
**not** a placeholder awaiting completion in a later Phase 113 plan; it is a scope boundary this
ADR records as intentional, mirroring ADR-111's treatment of upstream's `resource` module. A
future planner who eventually absorbs `b1ecbc02` proper (bringing general OAuth2
`client_credentials` route wiring for non-SPIFFE routes) must find this ADR and understand that
Phase 113's SPIFFE-scoped slice was built independently of, and does not need to be reconciled
against, that general layer's eventual shape — the two are additive, not conflicting, provided the
future absorb does not silently rename or restructure the SPIFFE-only symbols this phase created.
The corresponding ledger carry-forward note is filed in `108-DIVERGENCE-LEDGER.md` (see
Consequences).

---

## D-07: Loud-Skip Reality — Named Residuals, Not Smoothed Over

Re-run fresh this session, independently of Plan 113-07's own report:

```
$ cargo test -p nono-sandbox-proxy --test spiffe_integration -- --nocapture | grep -c '^SKIP\['
4
$ cargo test -p nono-sandbox-cli --test spiffe_run -- --nocapture | grep -c '^SKIP\['
2
```

**6 of 8 SPIFFE tests SKIP on this host** (no local SPIRE agent; `SPIRE_AGENT_SOCKET` unset):
`spiffe_integration.rs`'s `test_spiffe_jwt_live_fetch`,
`test_spiffe_jwt_live_delegation_none_on_plain_svid`, `test_spiffe_jwt_live_proxy_startup`, and
the fork-original `d03_connect_and_forward_http_deny_spiffe_declared_route_upstream_end_to_end`;
`spiffe_run.rs`'s `spiffe_jwt_credential_injected_end_to_end` and
`spiffe_jwt_proxy_starts_with_live_agent`. Only `test_spiffe_jwt_fails_closed_on_missing_socket`
(unconditional, fail-closed) genuinely exercises SPIFFE behavior locally on this host every time.
These 6 are compensated by the new `.github/workflows/spire.yml` CI lane (a real SPIRE 1.9.6
server+agent), not locally exercised — "tests passed" on this host must never be read as "the live
SPIFFE path works," per D-07's own stated purpose.

**A second, distinct residual, carried forward explicitly rather than smoothed over:**
`handle_spiffe_assertion_credential`'s (the OAuth2 jwt-bearer exchange path's) successful-forward
path has **no test coverage anywhere in this fork, local or CI.** `SpiffeAssertionTokenCache::new`
performs a live initial token exchange at construction with no `#[cfg(test)]` bypass constructor —
proving this path would require both a live SPIRE agent AND a live OAuth2 token endpoint
simultaneously, which Plan 113-07 explicitly declined to force via a production-code change that
would weaken the fail-secure posture (per its own stated instruction). By contrast,
`handle_spiffe_route`'s (the direct JWT-SVID bearer-injection path's) successful-forward path IS
proven end-to-end, CI-only, via `spiffe_run.rs`'s `spiffe_jwt_credential_injected_end_to_end`.

**This asymmetry is a real, open gap in this phase's evidence, named here rather than
inherited silently by whichever plan next touches `oauth2.rs`.**

---

## Consequences

1. **`108-DIVERGENCE-LEDGER.md` receives a "SPIFFE Carry-Forward Note (Phase 113, D-02)"** covering
   both the dropped `tls_intercept` hunks and the newly-identified `b1ecbc02` divergence — see that
   document for the full note; cross-referenced here, not duplicated.
2. **OD-1 is a permanent scope boundary**, not a deferred TODO — see the OD-1 section above. Future
   absorbs of `b1ecbc02` must consult this ADR.
3. **The ADR-86 boundary is confirmed intact** with one small, honestly-recorded concept-leak
   caveat (SPIFFE vocabulary now named in the core library's public surface) — no enforcement or
   policy logic crossed the boundary.
4. **D-07's residuals are named, not hidden:** 6 of 8 SPIFFE tests skip locally (compensated by
   `spire.yml`); `handle_spiffe_assertion_credential`'s successful-forward path is untested
   anywhere. Both are carried forward as open items for whichever future plan next touches this
   surface, not treated as closed by this phase's overall GREEN test/clippy state.
5. **`../nono-py`'s binding required more than the single anticipated `spiffe: None,` fix** — see
   this plan's own SUMMARY for the full break-site inventory (two exhaustive `RustRouteConfig`
   literals, three non-exhaustive-match compile errors on the new core-library enum variants, and
   one `NetworkAuditEvent` struct-literal field). All fixed, `maturin build` and `napi build` both
   confirmed green this session. This is recorded here because it directly evidences the D-11
   ("verify by behavior, never by identifier name alone") discipline this phase's own summaries
   repeatedly demonstrate — the plan's own `<interfaces>` block, itself derived from
   `113-RESEARCH.md`'s D-09 finding, underestimated the sibling-repo blast radius, and only running
   the actual build caught the gap.

---

## References

- Upstream commit: `c831dade422f2bdf37d7429af0423cafa0a60c06` — `feat(proxy): add SPIFFE/SPIRE
  workload identity auth for upstream routes (#1272)`
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library boundary D-08/SC3 test
  against, and whose concept-leak caveat this ADR records honestly
- `proj/ADR-111-resource-limits-boundary.md` — the "reject the core-module absorb" shape OD-1
  mirrors
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — the D-02 carry-forward
  note this ADR's Consequences section cross-references, including the newly-identified `b1ecbc02`
  divergence
- `.planning/phases/109-proxy-network-absorb/109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` — first
  recorded finding that the fork has no `tls_intercept`/`aws` subsystems
- `.planning/phases/112-security-residual-sync/112-AWS-SIGV4-PROXY-AUTH-FINDING.md` — re-confirms
  the same absence independently
- `.planning/phases/112-security-residual-sync/112-OAUTH-CAPTURE-DISPOSITION.md` — the SEC-02
  contrast case this ADR distinguishes itself from in the D-01 proof section
- `.planning/phases/113-spiffe-spire-workload-identity/113-CONTEXT.md` — D-01 through D-12, the
  locked decisions this ADR records durably
- `.planning/phases/113-spiffe-spire-workload-identity/113-RESEARCH.md` — the symbol-level
  disposition table and "⚠ Decision Conflicts" section this ADR's Context section builds on
- `.planning/phases/113-spiffe-spire-workload-identity/113-01-SUMMARY.md` through `113-07-SUMMARY.md`
  — the per-plan execution record this ADR's evidence is drawn from
