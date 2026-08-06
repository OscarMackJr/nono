# Phase 113: SPIFFE/SPIRE Workload Identity - Context

**Gathered:** 2026-08-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb upstream `c831dade` (#1272, 2026-07-16) — SPIFFE/SPIRE workload-identity auth for upstream
proxy routes — under a standalone ADR-gated review, configurable via profile, without regressing the
fork's divergent proxy model or the ADR-86 policy-free-library boundary.

**Re-measured live against today's tree (2026-08-06), not the 2026-07-29 split-time figures:**
33 files, +4354 / −545. One new *direct* dependency (`spiffe 0.16`, `jwt-source` feature); 19 new
crates transitively; 633 lockfile lines.

| Target | Fork state | Consequence |
|---|---|---|
| `nono-proxy/src/spiffe.rs` (+192), `auth.rs` (+137) | **absent** | clean new files |
| `route.rs` (+348), `reverse.rs` (545), `oauth2.rs` (+290), `credential.rs` (+243), `config.rs` (+104), `server.rs` (+204), `audit.rs` (+51) | present | real merge work |
| `tls_intercept/h2_forward.rs` (+192), `handle.rs` (+286) | **absent** | ~478 lines have nowhere to land → D-01 |
| `nono-cli/src/profile/mod.rs` (+200), `proxy_runtime.rs` (+197), `network_policy.rs` (+19) | present | CLI/profile wiring |
| `nono-cli/src/audit_ledger.rs` (+1) | **absent** | trivial, drop |
| `nono/src/undo/types.rs` (+45), `nono/src/audit.rs` (+2) | present | the ADR-86 crossing (see D-08 carried) |
| `crates/nono-cli/tests/spiffe_run.rs` (+285), `nono-proxy/tests/spiffe_integration.rs` (+188) | new | test bodies |
| `.github/workflows/spire.yml` (+112), `scripts/spire-test.sh` (+187), `scripts/spiffe-mock-server.py` (+174), `testdata/spire/*` (+55) | new | live-SPIRE infra → D-06 |

**Out of scope:** anything requiring a `tls_intercept/` module (the fork has none — see D-01);
OAuth capture (Phase 114 / SEC-02); the tool-sandbox subsystem (v3.7).

</domain>

<decisions>
## Implementation Decisions

### The absent `tls_intercept/` module

- **D-01: ADAPT-DOWN — absorb the ~90% that lands on real fork files; drop the two `tls_intercept/`
  hunks (~478 lines).** ADR-113 MUST carry a **positive, grep-backed proof** that no fork route type
  is left silently unauthenticated — not merely an observation that the files are absent.
  *Evidence gathered during discussion that motivates this:* `reverse.rs` (which the fork HAS) carries
  the primary SPIFFE path — `handle_spiffe_route`, `handle_spiffe_assertion_credential`,
  `validate_reverse_local_auth`, local session-token validation, mTLS/JWT-SVID auth, credential
  injection, `SpiffeAuditContext` emission. `tls_intercept/handle.rs` (which the fork LACKS) carries
  `handle_spiffe_intercept_request`, a **parallel** implementation doing the same job for intercepted
  TLS streams. They are sibling paths, **not** a base plus its only enforcement hook. This is the
  inverse of SEC-02, where `forward.rs`'s `ResponseRewrite` was the sole mechanism keeping real tokens
  out of the sandbox, so a reduced absorb would have leaked. Here the fork has no TLS interception at
  all, so dropping these hunks removes a code path the fork **cannot reach**, rather than removing a
  guard from a path it can. **The planner must re-verify this at symbol level rather than inherit it**
  (see D-11 carried, and the Phase 112 disposition-table failures).
- **D-03: A request for a SPIFFE-declared route that arrives on a proxy path with no SPIFFE
  implementation (CONNECT tunnel, forward HTTP, external-proxy chain) MUST fail closed at request
  time.** A route declared SPIFFE-authenticated must never be served unauthenticated regardless of
  arrival path. This is the direct residual risk created by D-01, and it is the exact shape of
  WR-13 in `112-REVIEW.md` (`handle_forward_http` was the one path of six that ignored
  `config.require_auth`). Startup-time validation was considered and NOT chosen — request-time is
  the locked enforcement point.

### `RouteStore::load` and the Phase 109 divergence

- **D-04: ADOPT upstream's async `RouteStore::load` — this OVERTURNS a recorded Phase 109 decision.**
  `crates/nono-proxy/src/server.rs:235` currently asserts the fork deliberately keeps a
  *"simpler `ProxyHandle`/`RouteStore` shape (no TLS intercept, no SPIFFE, no async
  `RouteStore::load`)"*. Phase 113 makes two-thirds of that sentence false; **the note MUST be
  rewritten, not left stale.**
  *Why async is unavoidable:* `RouteStore::load` awaits `SpiffeJwtSource::connect(workload_api_socket,
  audience, inject_header, credential_format, svid_hint)` and returns `ProxyError::Config` on failure —
  it is structurally required by an eager connect-at-load design, not incidental churn.
  *Why adopting is safe:* that connect sits **inside the per-route SPIFFE branch**, so a profile with
  no SPIFFE routes never touches the Workload API and no non-SPIFFE deployment gains a live-SPIRE
  startup dependency. The cost is signature churn across all `RouteStore::load` callers.
  *Why adopt rather than fork further:* `route.rs` is already a repeated merge-conflict point; holding
  a divergent sync signature raises the merge tax on every future proxy absorb.

### Dependency posture

- **D-05: ACCEPT the new dependency surface, GATED on an explicit dependency review recorded in
  ADR-113.** The review MUST (a) enumerate all 19 new crates, (b) show `cargo audit` clean, and
  (c) **prove** the JNI/Android path is cfg-gated out on all three shipped targets rather than
  assuming it.
  *What actually enters the tree* (measured, not estimated): one direct dep `spiffe 0.16`
  (`jwt-source`) pulling **19 crates** — a full gRPC/protobuf stack (`tonic`, `tonic-prost`, `prost`,
  `prost-derive`, `prost-types` — the SPIFFE Workload API is gRPC), **JNI bindings** (`jni`,
  `jni-macros`, `simd_cesu8`) arriving via `rustls-platform-verifier`'s Android support, plus
  `rustls-platform-verifier`, `futures`, `tokio-stream`, `pin-project`(+`-internal`), `hyper-timeout`,
  `itertools`, `rand_pcg`, `jsonschema-regex`, `simdutf8`. On Linux/macOS/Windows the JNI path should
  be Android-cfg-gated and never linked — but it remains in `cargo audit`'s scope and in the supply
  chain, which is why (c) is a proof obligation and not a footnote. The fork already has the local
  cross-target toolchain needed to check what actually links per target.
  *Trimming (e.g. avoiding `rustls-platform-verifier` via feature flags) was considered and NOT
  chosen as a gate — the planner may still note it as an opportunistic improvement.*

### Test and CI infrastructure

- **D-06: Absorb the fail-closed tests AND stand up the SPIRE CI lane** (`.github/workflows/spire.yml`,
  `scripts/spire-test.sh`, `testdata/spire/{agent,server}.conf`), so the live SPIFFE path is genuinely
  executed somewhere. Rationale: D-03's fail-closed enforcement and D-04's eager async connect
  otherwise ship with compile-time proof only. The fork already has Linux CI lanes.
  *Upstream's test split is already favourable:* `spiffe_integration.rs` states *"Fail-closed tests run
  everywhere (no SPIRE needed)"* with live tests gated on the `SPIRE_AGENT_SOCKET` env var;
  `spiffe_run.rs` is end-to-end against the real `nono` binary and skips entirely without that var.
  Its mock HTTP server is a Rust `TcpListener`, **not** `spiffe-mock-server.py` — so the local
  `cross` container's missing python3 does **not** gate the test bodies.
- **D-07: Loud skip reporting is a HARD ACCEPTANCE CRITERION.** Any SPIFFE test that skips must be
  named, counted, and surfaced in the phase's verification artifact, so "tests passed" can never be
  mistaken for "the live path works". This targets a failure that already occurred this milestone:
  `crates/nono-cli/tests/socket_access_run.rs` reported `ok` in ~0.01s across all its tests while
  exercising nothing (no python3 in the `cross` container), and that nearly became evidence in
  Phase 112's verification. The requirement is written to generalise beyond SPIFFE.

### Carried from Phase 109 — standing rules, NOT to be re-litigated

- **D-08 (109): SPIFFE's core-library touches were inspected and ADR-86 appears INTACT.**
  `crates/nono/src/undo/types.rs` (+45) adds only auth-method enum variants (`SpiffeJwtBearer`,
  `SpiffeOAuthAssertion`, `SpiffeJwt`) and serde structs `SpiffeAuditContext` /
  `SpiffeDelegationContext` recording *what happened*; `crates/nono/src/audit.rs` (+2) is two
  `spiffe_context: None,` initializers. No enforcement, no policy evaluation — consistent with
  CLAUDE.md's boundary table placing audit in the core library as observability, not policy.
  **Caveat that ADR-113 MUST honour:** it does introduce SPIFFE *vocabulary* into the policy-free
  library — a mild concept-leak. **ADR-113 must confirm this reading against the full diff rather than
  inherit it.** (This is ROADMAP SC3.)
- **D-09 (109): Rebuild BOTH bindings after any `nono-proxy` struct change** — `maturin build` in
  `../nono-py`, `napi build --platform --release` in `../nono-ts`. Static inspection misses struct
  drift; only building catches it. (This is ROADMAP SC4.)
- **D-11 (109): Verify feature presence by BEHAVIOR, never by identifier name.**
- **D-12 (109): Both cross-target clippy gates mandatory** (`cross` linux-gnu + `cargo-zigbuild`
  apple-darwin, both GREEN locally, **no PARTIAL→CI**) — `c831dade` touches
  `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`, a cfg-gated Unix surface. Run `make ci`
  (clippy + fmt + tests), not clippy alone.

### Claude's Discretion

- **D-02: The dropped `tls_intercept` hunks get a carry-forward note in
  `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`**, following the SEC-09
  Carry-Forward Note precedent set in Phase 112 — so whichever future phase introduces a
  `tls_intercept/` module must consciously re-decide whether to bring the SPIFFE interception path
  with it, rather than silently inheriting the gap. Filed in the ledger (the canonical work-list a
  future planner reads), not only in ADR-113.
- Whether to opportunistically trim the dependency tree beyond D-05's proof obligation.
- Plan/wave decomposition, and whether ADR-113 lands as its own plan or as Wave 1 of a larger one.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### This phase's governing documents
- `.planning/ROADMAP.md` § "Phase 113: SPIFFE/SPIRE Workload Identity" — goal, SC1–SC4.
  **SC1 is factually stale:** it asks the ADR to weigh *"the 545-deletion rewrite of the fork's
  divergent `tls_intercept`/`reverse.rs`"*, but the fork has **no `tls_intercept` module at all**
  (`ls crates/nono-proxy/src/` confirms; see also the two finding docs below). Written 2026-07-29
  against an assumption Phases 109 and 112 have since disproved. Treat the `reverse.rs` half as live
  and the `tls_intercept` half as superseded by D-01.
- `.planning/REQUIREMENTS.md` — NET-02 (line ~114), traceability row (line ~164).
- `proj/ADR-113-spiffe-disposition.md` — **does not exist yet; this phase creates it.** Required by
  SC1. Shape precedent: `proj/ADR-111-resource-limits-boundary.md`.

### Prior-phase decisions this phase inherits or overturns
- `.planning/phases/109-proxy-network-absorb/109-CONTEXT.md` — D-01/D-02 (the split), **D-08**
  (ADR-86 reading + its caveat), D-09/D-10/D-11/D-12 (standing rules). Carried above.
- `.planning/phases/109-proxy-network-absorb/109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` — first
  recorded finding that the fork has no `tls_intercept`/`aws` subsystems.
- `.planning/phases/112-security-residual-sync/112-AWS-SIGV4-PROXY-AUTH-FINDING.md` — re-confirms the
  same absence independently (SEC-01 won't-sync).
- `.planning/phases/112-security-residual-sync/112-OAUTH-CAPTURE-DISPOSITION.md` — the SEC-02
  contrast case: there the missing piece WAS the sole enforcement point, which is why that absorb was
  deferred and this one is not. Read before writing D-01's proof.
- `.planning/phases/112-security-residual-sync/112-07-SUMMARY.md` — 112-07 dropped
  `apply_tls_intercept_config` with a grep-verified acceptance criterion; the mechanical precedent
  for D-01.
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — canonical work-list;
  D-02's carry-forward note lands here. See its "SEC-09 Carry-Forward Note" for the shape.

### Boundary and standards
- `CLAUDE.md` — authoritative. Fail-secure, unwrap policy, `// SAFETY:` on unsafe, path security,
  Landlock allow-list-only, and the library-vs-CLI boundary table.
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library boundary D-08/SC3 test
  against.
- `.planning/templates/cross-target-verify-checklist.md` — single source of truth for D-12's gates.

### Review debt carried into this phase's files
- `.planning/phases/112-security-residual-sync/112-REVIEW.md` — **7 Info findings remain OPEN** by
  operator scoping, several in files this phase edits (`proxy_command.rs`, `cli.rs`, `server.rs`).
  WR-13's fixed shape (one request path of six ignoring an auth gate) is the direct precedent for D-03.

### Upstream source
- Commit `c831dade422f2bdf37d7429af0423cafa0a60c06` — reachable via the `upstream` remote
  (`nolabs-ai/nono`). `git show c831dade -- <path>` works locally; no fetch needed.
- `docs/cli/features/spiffe.mdx` (+188 in that commit) — upstream's user-facing doc, useful for the
  profile-configuration surface (SC2).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/nono-proxy/src/reverse.rs` — the fork HAS this; it is where the primary SPIFFE path lands.
- `crates/nono-proxy/src/oauth2.rs` (+290 upstream) — present; SPIFFE assertion flow extends it.
- `crates/nono-proxy/src/credential.rs` (+243 upstream) — present; gains `get_spiffe_assertion`.
- `crates/nono-cli/src/network_policy.rs`, `audit_integrity.rs` — both present (verified).
- `ProxyConfig.require_auth` / `strict_connect_auth` — added Phase 112 (112-07 + WR-04/WR-13 fixes).
  D-03's fail-closed check should compose with, not duplicate, this existing auth gating.
- The fork's `validate_block_net_conflicts` / D-04(109) fail-closed-guard pattern is the idiom to
  mirror for D-03's enforcement.

### Established Patterns
- **The fork has no TLS interception, by standing decision.** `ProxyHandle::intercept_ca_path()`
  always returns `None` (112-07). Do not introduce one.
- **`nono-py` constructs `RustProxyConfig`/`RouteConfig` with EXHAUSTIVE struct literals, not
  `..Default::default()`** — so *every* new field breaks its build. `RouteConfig` gains a `spiffe`
  field in this absorb, which will be the **fourth consecutive drift** (after `endpoint_policy`/
  `enable_h2` in v3.4, `denied_hosts`/`no_proxy` in 109-05, `require_auth`/`strict_connect_auth`
  in Phase 112 — fixed 2026-08-06 in nono-py `252ffe3`). Budget for it; D-09 is how it gets caught.
  `../nono-ts` does **not** use the proxy crate at all (only `nono::query::*`) and is structurally
  immune.
- Absorbs of upstream proxy commits routinely require adaptation, not `git apply` — `git apply --check`
  failed outright for 112-07's commit.

### Integration Points
- `crates/nono-proxy/src/server.rs:235` — the divergence note D-04 makes false; must be rewritten.
- `RouteStore::load` call sites — all become async under D-04.
- `crates/nono-cli/src/profile/mod.rs` + the fork-only `nono-profile.schema.json` — SC2's
  profile-configurability. Note the Phase 110 lesson: a new profile field that parses via serde but
  is absent from the strict schema fails `validate_against_schema()`; the schema is a fork-only
  companion file upstream never touches, so it must be edited by hand.

</code_context>

<specifics>
## Specific Ideas

- **Do not inherit the disposition table's confidence.** Phase 112's reality-check table was wrong in
  3 of the ~6 dispositions re-checked at implementation time, every time because confidence came from
  file-presence rather than symbol-level evidence. For every hunk in this absorb, grep the fork for
  each type/function/field/import it references before rating it. A missing symbol is a disposition
  amendment to record, not an implementation detail to paper over.
- **Re-locate by symbol, never by line number.** Line numbers in this document and in `c831dade` are
  already stale relative to the fork; Phase 112 saw a plan cite a 2-arg signature that a same-day
  sibling plan had already extended to 3 args.
- D-01's proof should read like `112-07`'s: a literal grep-verified acceptance criterion
  (e.g. "`grep -rn 'tls_intercept' crates/nono-proxy/src/ | wc -l` returns 0"), not prose assurance.

</specifics>

<deferred>
## Deferred Ideas

- **Fixing `nono-py`'s exhaustive struct literal permanently** (switching to `..Default::default()`).
  Raised but explicitly not taken: it would end a four-time-recurring build break, but trades a loud
  compile error for silent field adoption on a security-relevant struct — a design decision, not a
  cleanup, and arguably the worse failure mode. Belongs in its own scoped change, not this absorb.
- **The profile/schema surface for SPIFFE** (`profile/mod.rs` +200, `nono-profile.schema.json`) — SC2
  requires profile-configurability, but the shape was not discussed. Left to research + planning.
- **Re-verifying D-08's ADR-86 reading in depth** — carried as a proof obligation on ADR-113 (above)
  rather than pre-decided here.
- **Opportunistically trimming `rustls-platform-verifier`/JNI out of the tree** — noted under D-05 as
  discretionary, not a gate.
- **The 7 open Info findings in `112-REVIEW.md`** — several live in files this phase edits. Not folded
  into Phase 113 scope; they remain open by operator decision.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` (score 0.6) — matched only on generic keywords ("2026", "phase",
  "code"). A v3.5 host-gated MSI/VC++ redistributable item, unrelated to SPIFFE. Not folded.
- `20260611-poc-cert-broker-clean-host.md` (score 0.6) — same; v3.5 clean-host POC-cert broker item.
  Not folded.

</deferred>

---

*Phase: 113-spiffe-spire-workload-identity*
*Context gathered: 2026-08-06*
