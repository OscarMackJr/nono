# Phase 113: SPIFFE/SPIRE Workload Identity - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-06
**Phase:** 113-spiffe-spire-workload-identity
**Areas discussed:** The tls_intercept hole, Overturning the 109 RouteStore decision, Dependency posture, Test + CI infrastructure

---

## Pre-discussion finding

Before questions were framed, the phase was re-measured live rather than trusting the 2026-07-29
split-time figures. Two things surfaced:

1. **ROADMAP SC1 is factually stale.** It directs ADR-113 to weigh *"the 545-deletion rewrite of the
   fork's divergent `tls_intercept`/`reverse.rs`"* — but the fork has no `tls_intercept` module.
   Confirmed by `ls crates/nono-proxy/src/`, and independently recorded twice before
   (`109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`, `112-AWS-SIGV4-PROXY-AUTH-FINDING.md`).
2. **`crates/nono-proxy/src/server.rs:235` already carries a deliberate fork note** — *"adapted to
   this fork's simpler `ProxyHandle`/`RouteStore` shape (no TLS intercept, no SPIFFE, no async
   `RouteStore::load`)"* — making the RouteStore question an overturn of a recorded decision rather
   than a fresh choice.

---

## The tls_intercept hole

Grounded first by reading what the hunks *do*, not their filenames: `reverse.rs` (fork has it) carries
the primary SPIFFE path; `tls_intercept/handle.rs` (fork lacks it) carries
`handle_spiffe_intercept_request`, a **parallel** implementation for intercepted TLS streams — sibling
paths, not a base plus its only enforcement hook. This is the inverse of SEC-02, where the missing
`forward.rs` `ResponseRewrite` hook WAS the sole enforcement point.

| Option | Description | Selected |
|--------|-------------|----------|
| Adapt-down: drop them, prove unreachable | Absorb the ~90% landing on real files; ADR carries a positive grep-backed unreachability argument. Mirrors 112-07. | ✓ |
| Adapt-down + fail-closed tripwire | Same, plus a guard hard-erroring if TLS interception is ever reintroduced alongside SPIFFE routes. | |
| Treat the gap as disqualifying — defer | Don't absorb partially; record as deferred with reality-check evidence, mirroring SEC-02 → Phase 114. | |
| You decide — let the ADR settle it | No pre-locked direction. | |

**User's choice:** Adapt-down: drop them, prove unreachable
**Notes:** Became D-01. The proof obligation is explicitly *positive* — ADR-113 must show no fork route
type is left silently unauthenticated, not merely note the files are absent.

---

### Follow-up: the residual risk D-01 creates

The fork's proxy has several request paths (reverse, CONNECT tunnel, forward HTTP, external chain);
SPIFFE lands only on `reverse.rs`. Directly the shape of WR-13 in `112-REVIEW.md`, where
`handle_forward_http` was the one path of six ignoring `config.require_auth`.

| Option | Description | Selected |
|--------|-------------|----------|
| Fail closed at request time | Reject the request; a SPIFFE-declared route is never served unauthenticated regardless of arrival path. | ✓ |
| Fail closed at startup/profile-validation | Refuse to start if a SPIFFE route is reachable via a non-SPIFFE path. Earlier, but may not be statically computable for all route shapes. | |
| Both — startup guard plus runtime backstop | Defence in depth, at the cost of two enforcement points to keep in sync. | |

**User's choice:** Fail closed at request time
**Notes:** Became D-03.

---

## Overturning the 109 RouteStore decision

Grounded by checking whether `async` was required or incidental: `RouteStore::load` awaits
`SpiffeJwtSource::connect(workload_api_socket, …)` and returns `ProxyError::Config` on failure —
structurally required by an eager connect-at-load design. Mitigating nuance surfaced during the check:
that connect sits inside the per-route SPIFFE branch, so non-SPIFFE profiles never acquire a
live-SPIRE startup dependency.

| Option | Description | Selected |
|--------|-------------|----------|
| Adopt async — overturn and update the 109 note | Take upstream's shape; minimise merge distance on a file that is already a repeated conflict point. Requires rewriting the now-false `server.rs:235` note. | ✓ |
| Keep sync — adapt SPIFFE to lazy connect | Preserve 109 exactly; connect on first use. Raises the merge tax on every future proxy sync. | |
| Adopt async, but gate it behind SPIFFE being configured | Async signature plus explicit opt-in construction. More machinery for a property option 1 mostly already has. | |
| You decide — let ADR-113 settle it | No pre-locked direction. | |

**User's choice:** Adopt async — overturn and update the 109 note
**Notes:** Became D-04. ADR-113 must record that it overturns a prior phase's decision, and the stale
`server.rs:235` note must be rewritten rather than left asserting something untrue.

---

## Dependency posture

Grounded by enumerating what actually enters the lockfile: one direct dep (`spiffe 0.16`,
`jwt-source`) pulling **19 crates** — a gRPC/protobuf stack (`tonic`, `tonic-prost`, `prost`,
`prost-derive`, `prost-types`) and JNI bindings (`jni`, `jni-macros`, `simd_cesu8`) arriving via
`rustls-platform-verifier`'s Android support. A more pointed framing than the roadmap's
"~633-line Cargo.lock expansion".

| Option | Description | Selected |
|--------|-------------|----------|
| Accept, gated on an explicit dependency review | Enumerate the 19, `cargo audit` clean, and prove the JNI/Android path is cfg-gated out on all three shipped targets. | ✓ |
| Accept, and try to trim first | Same review, plus attempt to shrink the tree (e.g. avoid `rustls-platform-verifier`) before accepting. | |
| Treat the surface expansion as disqualifying | Too much new attack surface for one auth mechanism; decline or defer with the dependency evidence as the reason. | |
| You decide — let ADR-113 settle it | No pre-locked direction. | |

**User's choice:** Accept, gated on an explicit dependency review
**Notes:** Became D-05. Item (c) — proving the JNI path is cfg-gated out per target — is a proof
obligation, not a footnote. Trimming remains available to the planner as an opportunistic improvement.

---

## Test + CI infrastructure

Grounded by checking what the tests actually require. Upstream's split is already favourable:
`spiffe_integration.rs` states *"Fail-closed tests run everywhere (no SPIRE needed)"* with live tests
gated on `SPIRE_AGENT_SOCKET`; `spiffe_run.rs`'s mock HTTP server is a Rust `TcpListener`, not
`spiffe-mock-server.py` — so the local `cross` container's missing python3 does not gate the test
bodies.

| Option | Description | Selected |
|--------|-------------|----------|
| Fail-closed tests + stand up the SPIRE CI lane | Absorb the tests plus `spire.yml`, `spire-test.sh`, `testdata/spire/*` so the live path is genuinely executed. | ✓ |
| Fail-closed tests only — no SPIRE lane | Cheapest; but the live SPIFFE path would ship never having been executed. | |
| Absorb everything, live tests CI-only | Same coverage as option 1; differs mainly in documented local developer experience. | |

**User's choice:** Fail-closed tests + stand up the SPIRE CI lane
**Notes:** Became D-06. Rationale: D-03's fail-closed enforcement and D-04's eager async connect would
otherwise ship with compile-time proof only.

---

### Follow-up: skip reporting

| Option | Description | Selected |
|--------|-------------|----------|
| Loud skip reporting is a hard acceptance criterion | Skipped tests named, counted, surfaced in the verification artifact; written to generalise beyond SPIFFE. | ✓ |
| Report skips, but scoped to this phase only | Narrower; leaves existing silently-skipping suites untouched. | |
| No special requirement | Rely on standard cargo output and reviewer attention. | |

**User's choice:** Loud skip reporting is a hard acceptance criterion
**Notes:** Became D-07. Motivated by a failure that already occurred this milestone —
`socket_access_run.rs` reported `ok` in ~0.01s across all tests while exercising nothing, and nearly
became evidence in Phase 112's verification.

---

## Claude's Discretion

- **D-02** — recording the dropped `tls_intercept` hunks as a carry-forward note in
  `108-DIVERGENCE-LEDGER.md` (SEC-09 precedent). Taken as discretion rather than spending a question,
  since the precedent was set earlier in this same milestone; the user was told and offered the
  alternative of confining it to ADR-113.
- Whether to opportunistically trim the dependency tree beyond D-05's proof obligation.
- Plan/wave decomposition, and whether ADR-113 lands as its own plan or as Wave 1 of a larger one.

## Deferred Ideas

- Fixing `nono-py`'s exhaustive struct literal permanently (`..Default::default()`) — would end a
  four-time-recurring build break but trades a loud compile error for silent field adoption on a
  security-relevant struct. Its own scoped change, not this absorb.
- The profile/schema surface for SPIFFE (`profile/mod.rs` +200, `nono-profile.schema.json`) — SC2
  requires profile-configurability; shape left to research and planning.
- Re-verifying D-08's ADR-86 reading in depth — carried as a proof obligation on ADR-113.
- The 7 open Info findings in `112-REVIEW.md`, several in files this phase edits.

### Todos reviewed, not folded

- `20260611-msi-vcredist-prereq.md` (score 0.6) — matched only on generic keywords ("2026", "phase",
  "code"); a v3.5 host-gated MSI/VC++ item unrelated to SPIFFE.
- `20260611-poc-cert-broker-clean-host.md` (score 0.6) — same; v3.5 clean-host POC-cert broker item.
