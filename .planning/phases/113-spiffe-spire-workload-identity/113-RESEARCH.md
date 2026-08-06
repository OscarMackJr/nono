# Phase 113: SPIFFE/SPIRE Workload Identity - Research

**Researched:** 2026-08-06
**Domain:** Rust proxy security (SPIFFE/SPIRE workload identity, OAuth2 jwt-bearer assertion, upstream-fork absorb)
**Confidence:** MEDIUM-HIGH (every fork-side claim below is `[VERIFIED]` via direct `git show`/`grep`/`sed` against the live tree; upstream crate-ecosystem claims are `[VERIFIED: cargo info]`/`[VERIFIED: slopcheck]`; a small number of protocol/threat-model claims are `[ASSUMED]` and listed in the Assumptions Log)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**The absent `tls_intercept/` module**

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

  > **RESEARCH NOTE (see `## ⚠ Decision Conflicts` below):** the "~90% that lands on real fork
  > files" is now known to be substantially larger/riskier than a clean extension — the machinery
  > `handle_spiffe_route`/`handle_spiffe_assertion_credential` depend on (`CredentialStore`
  > OAuth2-route wiring, `LoadedRoute` managed-credential fields, `forward.rs`'s upstream-connect
  > abstraction) is ALSO absent from the fork, tracing to an unrelated, unabsorbed, pre-`v0.66.0`
  > upstream commit (`b1ecbc02`), not to `tls_intercept`. This does **not** reverse D-01's verdict.

- **D-03: A request for a SPIFFE-declared route that arrives on a proxy path with no SPIFFE
  implementation (CONNECT tunnel, forward HTTP, external-proxy chain) MUST fail closed at request
  time.** A route declared SPIFFE-authenticated must never be served unauthenticated regardless of
  arrival path. This is the direct residual risk created by D-01, and it is the exact shape of
  WR-13 in `112-REVIEW.md` (`handle_forward_http` was the one path of six that ignored
  `config.require_auth`). Startup-time validation was considered and NOT chosen — request-time is
  the locked enforcement point.

**`RouteStore::load` and the Phase 109 divergence**

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

  > **RESEARCH NOTE:** the blast radius is smaller than feared — exactly ONE production call site
  > (`server.rs:557`, already inside an `async fn`) plus 7 test-only call sites, all within
  > `nono-proxy`. See `## D-04 Blast-Radius Analysis` below.

**Dependency posture**

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

**Test and CI infrastructure**

- **D-06: Absorb the fail-closed tests AND stand up the SPIRE CI lane** (`.github/workflows/spire.yml`,
  `scripts/spire-test.sh`, `testdata/spire/{agent,server}.conf`), so the live SPIFFE path is genuinely
  executed somewhere. Rationale: D-03's fail-closed enforcement and D-04's eager async connect
  otherwise ship with compile-time proof only. The fork already has Linux CI lanes.
  *Upstream's test split is already favourable:* `spiffe_integration.rs` states *"Fail-closed tests run
  everywhere (no SPIRE needed)"* with live tests gated on the `SPIRE_AGENT_SOCKET` env var;
  `spiffe_run.rs` is end-to-end against the real `nono` binary and skips entirely without that var.
  Its mock HTTP server is a Rust `TcpListener`, **not** `spiffe-mock-server.py` — so the local
  `cross` container's missing python3 does **not** gate the test bodies.

  > **RESEARCH NOTE:** CONFIRMED true — `spiffe_run.rs` uses `std::net::TcpListener`
  > (`MockHttpServer::start()`), zero Python. Additionally confirmed: BOTH of `spiffe_run.rs`'s 2
  > tests are 100% `SPIRE_AGENT_SOCKET`-gated (no fail-closed-without-SPIRE assertion at the CLI-e2e
  > level); `spiffe_integration.rs` has exactly 1 of 4 tests that runs unconditionally.

- **D-07: Loud skip reporting is a HARD ACCEPTANCE CRITERION.** Any SPIFFE test that skips must be
  named, counted, and surfaced in the phase's verification artifact, so "tests passed" can never be
  mistaken for "the live path works". This targets a failure that already occurred this milestone:
  `crates/nono-cli/tests/socket_access_run.rs` reported `ok` in ~0.01s across all its tests while
  exercising nothing (no python3 in the `cross` container), and that nearly became evidence in
  Phase 112's verification. The requirement is written to generalise beyond SPIFFE.

**Carried from Phase 109 — standing rules, NOT to be re-litigated**

- **D-08 (109): SPIFFE's core-library touches were inspected and ADR-86 appears INTACT.**
  `crates/nono/src/undo/types.rs` (+45) adds only auth-method enum variants (`SpiffeJwtBearer`,
  `SpiffeOAuthAssertion`, `SpiffeJwt`) and serde structs `SpiffeAuditContext` /
  `SpiffeDelegationContext` recording *what happened*; `crates/nono/src/audit.rs` (+2) is two
  `spiffe_context: None,` initializers. No enforcement, no policy evaluation — consistent with
  CLAUDE.md's boundary table placing audit in the core library as observability, not policy.
  **Caveat that ADR-113 MUST honour:** it does introduce SPIFFE *vocabulary* into the policy-free
  library — a mild concept-leak. **ADR-113 must confirm this reading against the full diff rather than
  inherit it.** (This is ROADMAP SC3.)

  > **RESEARCH NOTE:** CONFIRMED TRUE at symbol level via direct diff read — zero enforcement/policy
  > logic in either hunk.

- **D-09 (109): Rebuild BOTH bindings after any `nono-proxy` struct change** — `maturin build` in
  `../nono-py`, `napi build --platform --release` in `../nono-ts`. Static inspection misses struct
  drift; only building catches it. (This is ROADMAP SC4.)

  > **RESEARCH NOTE:** CONFIRMED exact break site — `../nono-py/src/proxy.rs:206-218`'s exhaustive
  > `RustRouteConfig { .. }` literal will fail `E0063` once `spiffe` field lands. `../nono-ts`
  > confirmed structurally immune (zero `nono_proxy` references).

- **D-11 (109): Verify feature presence by BEHAVIOR, never by identifier name.**
- **D-12 (109): Both cross-target clippy gates mandatory** (`cross` linux-gnu + `cargo-zigbuild`
  apple-darwin, both GREEN locally, **no PARTIAL→CI**) — `c831dade` touches
  `crates/nono-cli/src/exec_strategy/supervisor_linux.rs`, a cfg-gated Unix surface. Run `make ci`
  (clippy + fmt + tests), not clippy alone.

  > **RESEARCH NOTE:** Docker (Server 29.6.2), `zig`, and `cargo-zigbuild` all confirmed present and
  > runnable on this host — both gates are locally runnable, no PARTIAL→CI fallback expected to be
  > needed.

### Claude's Discretion

- **D-02: The dropped `tls_intercept` hunks get a carry-forward note in
  `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`**, following the SEC-09
  Carry-Forward Note precedent set in Phase 112 — so whichever future phase introduces a
  `tls_intercept/` module must consciously re-decide whether to bring the SPIFFE interception path
  with it, rather than silently inheriting the gap. Filed in the ledger (the canonical work-list a
  future planner reads), not only in ADR-113.
- Whether to opportunistically trim the dependency tree beyond D-05's proof obligation.
- Plan/wave decomposition, and whether ADR-113 lands as its own plan or as Wave 1 of a larger one.

### Deferred Ideas (OUT OF SCOPE)

- **Fixing `nono-py`'s exhaustive struct literal permanently** (switching to `..Default::default()`).
  Raised but explicitly not taken: it would end a four-time-recurring build break, but trades a loud
  compile error for silent field adoption on a security-relevant struct — a design decision, not a
  cleanup, and arguably the worse failure mode. Belongs in its own scoped change, not this absorb.
- **The profile/schema surface for SPIFFE** (`profile/mod.rs` +200, `nono-profile.schema.json`) — SC2
  requires profile-configurability, but the shape was not discussed. Left to research + planning.
  **(This research pass provides the concrete shape — see `## SC2: Profile/Schema Configuration
  Surface` below.)**
- **Re-verifying D-08's ADR-86 reading in depth** — carried as a proof obligation on ADR-113 (above)
  rather than pre-decided here. **(This research pass performs that re-verification — see D-08's
  disposition-table row.)**
- **Opportunistically trimming `rustls-platform-verifier`/JNI out of the tree** — noted under D-05 as
  discretionary, not a gate.
- **The 7 open Info findings in `112-REVIEW.md`** — several live in files this phase edits. Not folded
  into Phase 113 scope; they remain open by operator decision.
- Out of scope entirely (per the phase boundary): anything requiring a `tls_intercept/` module; OAuth
  capture (Phase 114 / SEC-02); the tool-sandbox subsystem (v3.7).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NET-02 | "SPIFFE/SPIRE workload-identity auth for upstream routes (#1272) is absorbed." Per `REQUIREMENTS.md`: *"→ MOVED to Phase 113 (2026-07-29): `c831dade` measured 4354 ins / 545 del / 33 files (57% of the original Phase 109), crosses the ADR-86 boundary, rewrites fork-divergent `tls_intercept`/`reverse.rs`, and expands the dependency surface by ~633 lockfile lines — it gets its own ADR-gated phase."* | Full symbol-level disposition table (this document) covers every file in `c831dade`'s 33-file diffstat; `## SC2: Profile/Schema Configuration Surface` gives the concrete profile/schema shape; `## D-04 Blast-Radius Analysis` and `## D-09` sections size the two binding-rebuild-relevant blast radii; `## Validation Architecture` maps every ROADMAP SC (SC1-SC4) to a concrete test or verification command. |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

Directives from `./CLAUDE.md` that bind this phase's plan and implementation, extracted verbatim/paraphrased:

- **Library vs CLI boundary (table, "Coding Standards" + "Architecture" sections):** the core `nono` crate applies ONLY what's in `CapabilitySet`; policy groups, deny rules, profile loading, and all CLI-owned constructs stay in `nono-cli`. `audit`/`diagnostic` in the core library are observability primitives, not policy — this is the exact test D-08/SC3 apply to `undo/types.rs`/`audit.rs`'s SPIFFE additions (CONFIRMED compliant, see disposition table).
- **Error handling:** `NonoError`/`ProxyError` for all errors; propagate via `?` only. All new SPIFFE code paths in the diff already follow this (`Result<Self>`, `ProxyError::Config`/`Credential` variants) — preserve it.
- **Unwrap policy:** strictly forbid `.unwrap()`/`.expect()` outside test modules; enforced by `clippy::unwrap_used` under `-D warnings`. `#[allow(clippy::unwrap_used)]` is permitted only in `#[cfg(test)]` modules (the diff's own test files already use this pattern, e.g. `#![allow(clippy::unwrap_used)]` at the top of `spiffe_run.rs`/`spiffe_integration.rs`).
- **Unsafe code:** restrict to FFI; not implicated by this phase (no `unsafe` blocks appear anywhere in the diff).
- **Path security:** validate/canonicalize paths before applying capabilities; use `Path::starts_with()` never string `starts_with()`. `enforce_spiffe_socket_isolation()`'s `try_canonicalize()` + `Path::starts_with` comparison (in `proxy_runtime.rs`) already follows this correctly — verified via direct diff read.
- **Arithmetic:** `checked_`/`saturating_`/`overflowing_` for security-critical math. The diff's `depth = depth.saturating_add(1)` (delegation-chain depth counter, `spiffe.rs`) already follows this.
- **Memory:** `zeroize` crate for sensitive data. The diff already wraps every token/secret in `Zeroizing<String>` (`SpiffeJwtSource::fetch_token`, `UpstreamAuthMaterial::BearerToken.token`, `SpiffeAssertionTokenCache`'s cached token) — preserve this pattern in any rewritten code.
- **Testing:** unit tests for all new capability types and sandbox logic — the diff ships tests for every new type; the disposition table above flags which need rewriting (async conversion) vs. which are net-new.
- **Environment variables in tests:** save/restore pattern for `HOME`/`TMPDIR`/etc. — not directly implicated (SPIFFE tests use `SPIRE_AGENT_SOCKET`/`SPIRE_TRUST_DOMAIN`/`SPIRE_WORKLOAD_SPIFFE_ID`, read-only env vars, no mutation of shared state).
- **Cross-target clippy verification (MUST):** any commit touching cfg-gated Unix code — `supervisor_linux.rs` is directly touched by this diff — MUST be verified via `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` AND `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`, both locally runnable and GREEN on this host (confirmed this session). PARTIAL→CI is fallback-only, not default.
- **Commits:** DCO sign-off (`Signed-off-by: Name <email>`) on every commit — Oscar Mack Jr per the standing memory entry.
- **Security is non-negotiable / fail-secure:** every SPIFFE-specific control this research surfaces (D-03's request-time fail-closed guard, `enforce_spiffe_socket_isolation`'s hard-error-on-conflict, `SpiffeJwtSource::connect`'s fail-closed-on-unreachable-socket) already matches this posture — preserve, do not weaken, during adaptation.

**Toolchain fact correction for the planner:** `make` is confirmed NOT installed on this host (per the standing project fact); all `make ci`/`make test-spiffe` references in CONTEXT.md/CLAUDE.md should be translated to their constituent `cargo`/`bash scripts/spire-test.sh` invocations, as done throughout `## Validation Architecture` below.

## Summary

`c831dade` is NOT a clean "drop two `tls_intercept` files, keep the rest" absorb, even though D-01's verdict (ADAPT-DOWN) is correct and should not be re-opened. Symbol-level verification against the live fork tree found that the machinery `handle_spiffe_route`/`handle_spiffe_assertion_credential` depend on — `CredentialStore.oauth2_routes`/`get_oauth2()`, `LoadedRoute.requires_managed_credential`/`managed_auth_mechanism`/`managed_injection_mode`/`missing_managed_credential()`, and `reverse.rs`'s upstream-forwarding via `crate::forward::{UpstreamSpec, UpstreamStrategy, AuditCtx, forward_request}` — **does not exist in this fork at all**. It originates not from the absent `tls_intercept/` module but from an entirely separate, much older upstream commit, `b1ecbc02` ("feat(profile): support OAuth2 auth config in custom_credentials", landed shortly after upstream `v0.38.0`, dated months before this milestone's `v0.66.0..v0.69.0` sync window and never referenced by the Phase 108 ledger). The fork's own code contains an explicit comment at `route.rs:617-639` attributing this gap to "the upstream tls_intercept module not present in this fork" — that attribution is itself incorrect; the real ancestor is `b1ecbc02`, and the fork's `reverse.rs` forwards upstream requests via its own hand-rolled `parse_upstream_url()` (3-tuple, no scheme) + `connect_upstream_tls()` (always-TLS), not upstream's `forward.rs` module (which is also entirely absent — confirmed independently by `112-OAUTH-CAPTURE-DISPOSITION.md` for a different reason).

The practical consequence: absorbing SPIFFE's `handle_spiffe_route` requires **rewriting it against the fork's own upstream-connect primitives**, not lifting it verbatim, and absorbing the OAuth2-client-assertion half (`handle_spiffe_assertion_credential`) requires **building a SPIFFE-only slice of the missing OAuth2-route-wiring layer from scratch** (new `CredentialStore` fields/methods, new `LoadedRoute` fields, a new dispatch branch in `reverse.rs`) rather than extending pre-existing wiring, while deliberately declining to backport `b1ecbc02`'s general (non-SPIFFE) OAuth2 client_credentials proxy routing — that is out of NET-02's scope and would silently expand this absorb into an unrelated, out-of-window feature. `oauth2.rs`'s `TokenCache`/`OAuth2ExchangeConfig`/raw-HTTP-exchange helpers (`read_http_response`, `parse_token_response`, chunked decoding) DO already exist and are directly reusable for `SpiffeAssertionTokenCache`/`exchange_jwt_assertion` — that half of the absorb is materially closer to a clean port. Good news partially offsets this: `server.rs` (the `no_proxy`/`managed_loopback_upstream` layer) was already pre-adapted for this absorb in Phase 109 — `ProxyHandle.managed_loopback_upstream` and its comment ("a future absorb can wire detection without another `env_vars()` rewrite") exist today specifically anticipating this phase — and `RouteStore::load`'s async signature change (D-04) has exactly **one** production call site (`server.rs:557`, already inside an `async fn`), not the "signature churn across all callers" D-04's own text worried about.

`config.rs` (schema types), `undo/types.rs`/`nono/audit.rs` (D-08's audit vocabulary), `profile/mod.rs` (SC2's `CustomCredentialDef.spiffe` field + validation), and the test-file split (D-06) all absorb cleanly and are described below with concrete symbol-level evidence. Two genuine new hazards were found and are not covered by any locked decision: (1) two `&& let` let-chains in the diff (`spiffe.rs::check_nbf`, `proxy_runtime.rs`'s host-collection loop) are invalid under this workspace's edition-2021 `cargo fmt` gate — the identical failure mode Plan 112-02 hit and fixed by rewriting to nested `if let`; (2) `.gitignore:16` contains a bare `proj/` pattern that silently drops **new** files under `proj/` from `git add`/`git status` (already-tracked ADRs are unaffected) — `proj/ADR-113-spiffe-disposition.md`, required by SC1, MUST be added with `git add -f` or it will silently vanish from the commit.

**Primary recommendation:** Confirm D-01/D-03/D-04/D-08/D-09 exactly as locked (all independently re-verified true at symbol level); treat the `handle_reverse_proxy`/`CredentialStore`/`route.rs` absorb as a from-scratch build guided by upstream's diff as a design reference rather than a literal port; scope the OAuth2-assertion half to SPIFFE only (no general `client_credentials` route wiring); force-add the new ADR file; rewrite both let-chains to nested `if let` before the `cargo fmt --all --check` gate.

## Architectural Responsibility Map

This tool is not a web app; the standard Browser/SSR/API/CDN/DB tiers do not map cleanly. Adapted tiers for `nono`'s actual architecture:

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| JWT-SVID fetch from SPIRE Workload API | `nono-proxy` (proxy mediator) | — | The proxy, not the sandboxed child, holds the workload identity — this is the entire point of the feature (SEC threat: prevent the child from self-issuing SVIDs). |
| Bearer-token injection into upstream requests | `nono-proxy` (`reverse.rs`) | — | Existing credential-injection pattern (`ReverseProxyCtx`); SPIFFE is a new `ManagedUpstreamAuth` variant alongside the pattern already used for static/OAuth2 credentials. |
| OAuth2 jwt-bearer token exchange (RFC 7523) | `nono-proxy` (`oauth2.rs`) | — | Extends the existing `TokenCache` raw-HTTP-exchange machinery; no new subsystem. |
| SPIFFE route config schema + validation | `nono-cli` (`profile/mod.rs`, `nono-profile.schema.json`) | — | Per CLAUDE.md's boundary table: profile loading and validation is CLI-owned; the library stays policy-free. |
| SPIRE-socket sandbox isolation (deny child direct access) | `nono-cli` (`proxy_runtime.rs::enforce_spiffe_socket_isolation`) | `nono` core (`CapabilitySet`/Seatbelt platform rules) | CLI computes the isolation policy; the core library's existing `add_platform_rule`/`unix_socket_capabilities` primitives (already present, unmodified) apply it — matches the existing library-is-policy-free / CLI-owns-policy split. |
| SPIFFE audit vocabulary (`SpiffeAuditContext`, enum variants) | `nono` core (`undo/types.rs`, `audit.rs`) | — | Confirmed by direct diff read: pure data types + `None` initializers, no enforcement — matches ADR-86's audit-is-observability carve-out exactly (D-08 CONFIRMED). |
| Fail-closed request-time enforcement on non-SPIFFE-implementing proxy paths | `nono-proxy` (`server.rs::handle_forward_http`, `connect.rs`, `external.rs`) | — | D-03's locked enforcement point; mirrors the WR-13 (`112-REVIEW.md`) fixed shape exactly — a per-request-path `if` guard, not a startup check. |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|---------------|
| `spiffe` (aliased `spiffe-workload` in `Cargo.toml`) | 0.16.0 `[VERIFIED: cargo info spiffe]` | SPIFFE Workload API client (`jwt-source` feature = `workload-api-jwt`) | Official `spiffe/rust-spiffe` project (`github.com/maxlambrecht/rust-spiffe`), the canonical Rust SPIFFE client. `rust-version: 1.88` per its own manifest — **not a blocker**: this workspace's `Cargo.toml` declares `rust-version = "1.95"` (installed `rustc 1.95.0`), well above 1.88. **Note:** CLAUDE.md's prose ("Minimum Rust version: 1.82") is stale relative to the live `Cargo.toml` field — a pre-existing doc-drift, not something this phase must fix, but worth a one-line correction if the planner touches that file for any other reason. |

**Package name provenance:** `spiffe`'s package name and version were discovered by reading `git show c831dade -- crates/nono-proxy/Cargo.toml` (an authoritative diff of the actual dependency line upstream added), not from training data or web search — this satisfies the `[VERIFIED]` bar under the package-name provenance rule. `cargo info spiffe` independently confirms the registry entry, its `github.com/maxlambrecht/rust-spiffe` source repo, and its exact feature graph (matches the diff's `features = ["jwt-source"]` exactly: `jwt-source = [workload-api-jwt]`).

### Supporting (transitive, arriving via `spiffe 0.16` `jwt-source`)

| Crate | Purpose | Arrives because |
|---|---|---|
| `tonic`, `tonic-prost`, `prost`, `prost-derive`, `prost-types` | gRPC/protobuf stack | SPIFFE Workload API is a gRPC service (`workload-api-jwt` → `workload-api-core` → `transport-grpc` → `dep:tonic`, confirmed via `cargo info spiffe`'s feature graph) |
| `jni`, `jni-macros`, `simd_cesu8` | JNI (Java Native Interface) bindings | `rustls-platform-verifier 0.7.0`'s `jni` feature (`jni = [dep:jni]`, confirmed via `cargo info rustls-platform-verifier`) — this feature is Android-support-only; must be proven cfg-gated-out per D-05(c), see Package Legitimacy Audit below |
| `rustls-platform-verifier`, `futures`, `tokio-stream`, `pin-project`(+`-internal`), `hyper-timeout`, `itertools`, `rand_pcg`, `jsonschema-regex`, `simdutf8` | Various (TLS platform-native cert verification, async combinators, JSON-schema tooling) | Transitive dependents of `spiffe`'s `workload-api-jwt` feature chain |

Full crate count matches CONTEXT.md's measured 19 (18 named above + `spiffe` itself); this research did not re-derive the count from `cargo tree` (that requires adding the dependency, which is out of this research-only pass's scope — no source files were modified) but independently confirmed every named crate exists on crates.io and passes `slopcheck` (see Package Legitimacy Audit).

### Alternatives Considered

| Instead of | Could use | Tradeoff |
|---|---|---|
| `spiffe` crate's gRPC Workload API client | Hand-rolled Unix-socket + gRPC client | Rejected by upstream and by this research: reimplementing gRPC/protobuf framing for a security-critical identity-fetch path is exactly the kind of "don't hand-roll" case CLAUDE.md's crypto/keystore guidance already generalizes from. |
| `rustls-platform-verifier`'s JNI feature | Trim via `default-features = false` on the alias | D-05 marks this **discretionary, not a gate** — opportunistic, not required for this phase. |

**Installation:** No `npm install`/`pip install` — this is a `Cargo.toml` dependency addition. The single line to add (already verified against `c831dade`'s diff):
```toml
# crates/nono-proxy/Cargo.toml, [dependencies] section
spiffe-workload = { package = "spiffe", version = "0.16", features = ["jwt-source"] }
```

**Version verification:** `cargo info spiffe` run live 2026-08-06 confirms version `0.16.0` is current and matches upstream's diff exactly; `cargo info rustls-platform-verifier` confirms `0.7.0` is current with the `jni` feature gated behind `dep:jni`, not enabled by default.

## Package Legitimacy Audit

`slopcheck` 0.x was installed successfully this session (`pip install slopcheck --break-system-packages`) and run in `scan --pkg crates.io <name>` mode (a check-only, non-installing invocation — no packages were actually installed) against every direct/near-direct new crate the diff introduces.

| Package | Registry | slopcheck | Flags | Disposition |
|---|---|---|---|---|
| `spiffe` | crates.io | OK | none | Approved |
| `tonic-prost` | crates.io | OK | none | Approved |
| `prost` | crates.io | OK | none | Approved |
| `jni` | crates.io | OK | none | Approved |
| `jni-macros` | crates.io | OK | none | Approved |
| `simd_cesu8` | crates.io | OK | none | Approved |
| `rustls-platform-verifier` | crates.io | OK | none | Approved |
| `hyper-timeout` | crates.io | OK | none | Approved |
| `tokio-stream` | crates.io | OK | none | Approved |
| `pin-project` | crates.io | OK | none | Approved |
| `rand_pcg` | crates.io | OK | none | Approved |
| `simdutf8` | crates.io | OK | none | Approved |
| `jsonschema-regex` | crates.io | OK | `RECENTLY_CREATED` (info: "Created 34 days ago") | Approved — info-severity only, below the `[SUS]` threshold; note for the planner but no `checkpoint:human-verify` required |

**Packages removed due to slopcheck `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none. `jsonschema-regex`'s info-level recency flag does not rise to `[SUS]`; the planner may still choose to note it as a lower-confidence transitive dependency given its youth, but slopcheck itself does not gate on it.

**`cargo audit` availability:** `cargo-audit 0.22.1` is installed on this host (`cargo audit --version` confirmed live) — contrary to the research-brief's assumption that it might be absent. D-05(b)'s "`cargo audit` clean" proof obligation should be run by the planner/executor **after** the dependency is actually added to `Cargo.toml` (this research pass made no source changes, so there is nothing new for `cargo audit` to scan yet).

**D-05(c) JNI cfg-gating proof — exact commands for the executor to run once `spiffe` is added:**
```bash
# Confirms jni links (or does not) per target triple:
cargo tree --target x86_64-pc-windows-msvc -e features -p nono-sandbox-proxy | grep -i jni
cargo tree --target x86_64-unknown-linux-gnu -e features -p nono-sandbox-proxy | grep -i jni
cargo tree --target x86_64-apple-darwin -e features -p nono-sandbox-proxy | grep -i jni
# Expected: zero output on all three (jni is Android-only via rustls-platform-verifier's
# own target-cfg dependency declaration, confirmed via `cargo info rustls-platform-verifier`
# showing `jni` as an opt-in feature, not a default). A non-empty result on any of the three
# shipped targets is a hard finding that must block the absorb until resolved.
cargo tree -i jni   # after the dep lands, shows every path that pulls jni in — use to confirm
                     # the pull is scoped to an Android-only feature/target, not unconditional.
```

## Symbol-Level Disposition Table

Every row below was verified by `git show c831dade -- <path>` (upstream's hunk) AND a direct `grep`/`sed` read of the live fork file at the SAME path (not inferred from file presence). Confidence is rated only from symbol evidence, per this phase's mandate.

| File | Upstream hunk (lines) | Symbols the hunk needs | Present in fork? | At what path/signature | Confidence | Disposition |
|---|---|---|---|---|---|---|
| `crates/nono-proxy/src/spiffe.rs` | +192 (new file) | `SpiffeJwtSource`, `spiffe_workload::{JwtSource, SpiffeId}` (external crate) | Absent (new file) | n/a | HIGH | ADOPT verbatim — clean new file, no fork symbol collisions. **Caveat:** `check_nbf`'s `if let Some(nbf) = ... && now_ts < nbf` is a let-chain — invalid under this workspace's edition-2021 `cargo fmt --all --check` gate (confirmed empirically by Plan 112-02's identical failure). Must rewrite to nested `if let` before landing. |
| `crates/nono-proxy/src/auth.rs` | +137 (new file) | `nono::undo::SpiffeAuditContext`, `crate::spiffe::delegation_from_jwt`, `crate::config::resolved_credential_format` | `SpiffeAuditContext` — absent until `undo/types.rs` lands this same commit (self-consistent); `resolved_credential_format` — **VERIFIED present**, `grep -n "fn resolved_credential_format" crates/nono-proxy/src/config.rs` returns a hit | Fresh file, one real dependency (`resolved_credential_format`) confirmed to already exist | HIGH | ADOPT verbatim — clean new file. |
| `crates/nono/src/undo/types.rs` | +45 | `NetworkAuditAuthMechanism` enum, `NetworkAuditInjectionMode` enum, `NetworkAuditEvent` struct | All three present, confirmed via direct read | Exact | HIGH | ADOPT verbatim — pure additive enum variants + 2 new pure-data structs (`SpiffeAuditContext`, `SpiffeDelegationContext`) + one new `Option<SpiffeAuditContext>` field. **D-08 CONFIRMED TRUE at symbol level**: zero enforcement/policy logic in this hunk. |
| `crates/nono/src/audit.rs` | +2 | test-fixture literals only | n/a | n/a | HIGH | ADOPT verbatim — two `spiffe_context: None,` test-fixture additions. Confirms D-08. |
| `crates/nono-cli/src/audit_integrity.rs`, `audit_ledger.rs`, `exec_strategy/supervisor_linux.rs` | +1 each | test-fixture literals only | n/a | n/a | HIGH | ADOPT verbatim — trivial `spiffe_context: None,` additions. `supervisor_linux.rs` is in-scope for the D-12 cross-target clippy gate purely by directory location (`exec_strategy/`), independent of any cfg-attribute inspection. |
| `crates/nono-proxy/src/config.rs` | +104 | `RouteConfig` struct, `OAuth2Config` struct | Both present; `OAuth2Config` fields (`token_url`/`client_id`/`client_secret`/`scope`) match upstream's PRE-hunk shape exactly (built in this fork's own Phase 22, `b1ecbc02`-independent lineage) | `config.rs:1051` (`OAuth2Config`), `config.rs:528` (`RouteConfig`) | HIGH | ADOPT — clean, additive: `RouteConfig.spiffe: Option<SpiffeAuthConfig>`, new `SpiffeAuthConfig`/`ClientAssertionConfig` enums, `OAuth2Config.client_assertion`/`extra_params` fields. No symbol conflicts. |
| `crates/nono-proxy/src/oauth2.rs` | +290 | `TokenCache`, `OAuth2ExchangeConfig`, `read_http_response`, `parse_token_response`, `CachedToken`, `EXPIRY_BUFFER_SECS` | **All present** (built in Phase 22-04, commit `6653ea54`) | `oauth2.rs:69` (`TokenCache`), `:49` (`OAuth2ExchangeConfig`) | HIGH | ADOPT — `SpiffeAssertionTokenCache`/`exchange_jwt_assertion` are additive and reuse existing raw-HTTP-exchange helpers cleanly. This is the least risky file in the absorb. |
| `crates/nono-proxy/src/credential.rs` | +243 | `CredentialStore.oauth2_routes: HashMap<String, OAuth2Route>`, `CredentialStore.get_oauth2()`, `OAuth2Route` struct, `use crate::capture::CredentialCaptureMaterial` | **ALL ABSENT.** `grep -n "oauth2_routes\|get_oauth2\|struct OAuth2Route" crates/nono-proxy/src/credential.rs` → 0 hits. `crate::capture` module does not exist anywhere in the fork (`ls crates/nono-proxy/src/` has no `capture.rs`). The fork's `CredentialStore` has only `credentials: HashMap<String, LoadedCredential>` and `aws_routes: HashMap<String, ()>` (a placeholder). `TokenCache::new` (oauth2.rs) is never called from `credential.rs` — confirmed via `grep -rn "TokenCache" crates/nono-proxy/src/*.rs` returning hits ONLY inside `oauth2.rs` itself. | n/a — genuinely absent | **LOW→re-verify at implementation time; DISPOSITION AMENDMENT (see below)** | **ADAPT, substantially larger than "gains `get_spiffe_assertion`."** Must build `CredentialStore.spiffe_assertion_routes: HashMap<String, SpiffeAssertionRoute>` + `get_spiffe_assertion()` from scratch, calling `SpiffeAssertionTokenCache::new` directly — WITHOUT building the parallel general-OAuth2 `oauth2_routes`/`OAuth2Route`/`get_oauth2()` machinery upstream's diff assumes as pre-existing context (that's `b1ecbc02`, out of NET-02's scope). Drop the `use crate::capture::...` import line (context-only, no fork equivalent needed for this absorb). |
| `crates/nono-proxy/src/route.rs` | +348 | `LoadedRoute.requires_managed_credential`, `.managed_auth_mechanism`, `.managed_injection_mode`, `.missing_managed_credential()`, `RouteStore::spiffe_loaded_prefixes()`, `RouteStore::has_managed_loopback_upstream()`, `lookup_all_by_upstream()`, `has_intercept_route()` | **requires_managed_credential/managed_auth_mechanism/managed_injection_mode/missing_managed_credential ALL ABSENT** — confirmed via direct read of `LoadedRoute` (lines 25-64) and an EXPLICIT fork comment at `route.rs:614-639`: *"Fork divergence: tests for requires_intercept, requires_managed_credential, managed_auth_mechanism, managed_injection_mode, lookup_by_upstream, lookup_all_by_upstream, has_intercept_route, missing_managed_credential ... are not ported. These fields and methods belong to the upstream tls_intercept module not present in this fork."* This attribution is **factually wrong** — `git log --all -S"handle_oauth2_credential" -- crates/nono-proxy/src/reverse.rs` on the `upstream` remote traces this machinery to `b1ecbc02` (`git describe` = `v0.38.0-3-gb1ecbc02`), a completely different, much older commit with **zero** relationship to `tls_intercept`. `RouteStore::load` IS present, but synchronous (`pub fn load`, matches D-04's premise exactly). | n/a for the missing fields; `route.rs:89-95` for `load()`'s real (sync) signature | **LOW→re-verify; DISPOSITION AMENDMENT** | **ADAPT, build from scratch.** Add `managed_auth: Option<Arc<crate::auth::ManagedUpstreamAuth>>` and `has_spiffe_source()` to `LoadedRoute` (self-contained, low risk). DECLINE porting `requires_managed_credential`/`managed_auth_mechanism`/`managed_injection_mode`/`missing_managed_credential`/`lookup_all_by_upstream`/`has_intercept_route` wholesale (that's `b1ecbc02`'s scope, not this phase's) — instead write the minimum SPIFFE-specific equivalent needed for `handle_reverse_proxy`'s dispatch (a bare `route.has_spiffe_source()` boolean check is sufficient; the diff's `missing_managed_credential(has_static, has_oauth2, has_aws, has_spiffe)` 4-bool-arg signature exists only because upstream ALSO tracks 3 other managed-credential kinds this fork doesn't route yet — do not port that signature verbatim). `RouteStore::load` → `pub async fn load` is a correct, low-risk D-04 change (see D-04 blast-radius analysis below). Correct the misleading fork comment at `route.rs:617-618`/`633-639` while touching this file (it will otherwise continue misdirecting future absorbs). |
| `crates/nono-proxy/src/reverse.rs` | +545 (net, after -tls_intercept-adjacent hunks removed) | `crate::forward::{AuditCtx, UpstreamScheme, UpstreamSpec, UpstreamStrategy}`, `forward::forward_request()`, `route.managed_auth`, `credential_store.get_spiffe_assertion()` | **`crate::forward` module and everything in it is ABSENT.** `grep -n "UpstreamScheme" crates/nono-proxy/src/*.rs` → 0 hits anywhere in the fork. `grep -n "^use crate::forward" crates/nono-proxy/src/reverse.rs` → 0 hits (the fork's `reverse.rs` never imports from a `forward` module at all — confirmed by reading its actual import block, lines 17-30). The fork's real upstream-connect path is `fn parse_upstream_url(url_str: &str) -> Result<(String, u16, String)>` (3-tuple, **no scheme**) at `reverse.rs:599`, and `async fn connect_upstream_tls(...)` at `reverse.rs:644` (always negotiates TLS — there is no HTTP-vs-HTTPS branch in the fork's upstream connector at all). `handle_oauth2_credential`/`get_oauth2`/`OAuth2Route` — 0 hits (matches `credential.rs`'s absence). | `reverse.rs:599` (`parse_upstream_url`), `:644` (`connect_upstream_tls`) | **LOW→re-verify; DISPOSITION AMENDMENT** | **ADAPT, rewrite not port.** `handle_spiffe_route` and `handle_spiffe_assertion_credential`/`handle_oauth2_like` as literally diffed reference `forward::forward_request`/`UpstreamSpec`/`UpstreamStrategy::Direct` — none of which exist. Rewrite both against the fork's own `parse_upstream_url()` (3-tuple) + `connect_upstream_tls()` idiom, following the pattern the fork's EXISTING credential-injection code in the same file already uses (the un-diffed portion of `handle_reverse_proxy` that handles `static_cred`). `validate_reverse_local_auth` (D-01's cited "local session-token validation") is genuinely self-contained and DOES port cleanly — it only calls `token::validate_proxy_auth`/`validate_phantom_token_for_mode`, both confirmed present. Drop the top-of-file `use crate::forward::{...}` import entirely (no fork equivalent needed). |
| `crates/nono-proxy/src/server.rs` | +204 | `ProxyHandle.managed_loopback_upstream`, `RouteStore::spiffe_loaded_prefixes()`, `RouteStore::load(...).await`, `bound_port` field, `CredentialStore::load_with_diagnostics` | `managed_loopback_upstream` field **ALREADY PRESENT**, with a fork comment explicitly anticipating this absorb: *"This fork does not yet detect that condition ... the field exists so `env_vars()` carries the same semantics upstream does and so a future absorb can wire detection without another `env_vars()` rewrite."* `bound_port` — absent, needs adding (small, self-contained: one struct field + one `handle_forward_http` self-request check). `CredentialStore::load_with_diagnostics` — **absent**, fork calls `CredentialStore::load(&config.routes)?` directly (confirmed `server.rs:566`); the fork's own comment at `server.rs:73-74` says so explicitly ("no `load_with_diagnostics` integration yet"). | `server.rs:60-68` (`managed_loopback_upstream` field + comment), `:557`/`:566` (real call sites), `:235` (the stale D-04 divergence note) | HIGH | ADAPT — genuinely the lowest-risk file of the five real merge targets; Phase 109 pre-adapted the `no_proxy` groundwork for exactly this absorb. Use `CredentialStore::load(&config.routes)?` (not `load_with_diagnostics`) as the real call site to extend. Rewrite the stale `server.rs:235` comment (D-04 obligation) — it currently reads *"...adapted to this fork's simpler `ProxyHandle`/`RouteStore` shape (no TLS intercept, no SPIFFE, no async `RouteStore::load`)"*, which is now 2 of 3 clauses false. |
| `crates/nono-proxy/src/audit.rs` | +51 | `EventContext.spiffe_context` field, `log_reverse_proxy()` deletion | `EventContext` — present, `#[derive(Debug, Clone, Default)]` (confirmed `audit.rs:39-40`) — adding `Option<SpiffeAuditContext>` is safe under `Default`. **The diff DELETES `log_reverse_proxy` (deprecated since upstream 0.46.0) — but the fork's `reverse.rs:460` STILL CALLS `audit::log_reverse_proxy` today**, confirmed via `grep -rn "log_reverse_proxy" crates/nono-proxy/src/*.rs`. | `audit.rs:39` (`EventContext`), `reverse.rs:460` (live call site) | HIGH | ADAPT with one hard exclusion: add `spiffe_context: Option<SpiffeAuditContext>` to `EventContext` and thread it through `log_allowed`/`log_denied`/`log_l7_request`/`log_l7_policy_decision`/`log_credential_capture` (all confirmed present at matching names). **Do NOT delete `log_reverse_proxy`** — the fork has a live call site the deprecating upstream commit's own tree does not. |
| `crates/nono-cli/src/network_policy.rs` | +19 | `resolve_credentials()`, `partition_allow_domain()`, `RouteConfig` struct literals | Both functions present at matching names/roles | Confirmed via direct read | HIGH | ADOPT — 1-line `spiffe: cred.spiffe.clone()` / `spiffe: None` additions to existing `RouteConfig { .. }` literals. Mechanical. |
| `crates/nono-cli/src/profile/mod.rs` | +200 | `CustomCredentialDef` struct, `validate_custom_credential()`, `validate_oauth2_auth()`, `validate_header_mode()` | All present at matching names, existing mutual-exclusion pattern (`credential_key`/`auth`/`aws_auth`) directly extensible | `profile/mod.rs:369` (`CustomCredentialDef`), `:605` region (`validate_custom_credential`) | HIGH | ADOPT — this is the **cleanest file in the whole absorb** and is the concrete answer to SC2 (see below). |
| `crates/nono-cli/src/proxy_runtime.rs` | +197 | `build_proxy_config_from_flags()`, `start_proxy_runtime()`, `CapabilitySet::add_platform_rule`/`unix_socket_capabilities`, `crate::policy::escape_seatbelt_path` | All present, confirmed via targeted `grep`: `capability.rs:1389` (`add_platform_rule`), `:1422` (`unix_socket_capabilities`), `policy.rs:408` (`escape_seatbelt_path`) | Exact | HIGH | ADOPT `enforce_spiffe_socket_isolation()` — a genuine, valuable defense-in-depth control (denies the sandboxed child direct access to the SPIRE Workload API socket; hard-errors on a conflicting `unix_socket` grant). **Contains a let-chain** (`if let Ok(parsed) = url::Url::parse(...) && let Some(host) = parsed.host_str()`) in the unrelated host-collection loop just above — same edition-2021 hazard as `spiffe.rs`, must rewrite. **Minor cosmetic issue to fix while porting:** the `#[cfg(not(target_os = "macos"))]` debug-log branch hardcodes `"(Linux, no deny needed)"` in its log message even though that `cfg` also matches Windows — not a security bug (no active deny is needed on either platform since the child never receives the socket grant), but misleading log text; correct to name both platforms or drop the platform name from the string. |
| `crates/nono-proxy/src/tls_intercept/h2_forward.rs` (+192), `handle.rs` (+286) | 478 lines total | `tls_intercept` module | **Module absent entirely** (`ls crates/nono-proxy/src/` confirms) | n/a | HIGH | **DROP — D-01, LOCKED.** Confirmed exact line count (192+286=478) matches CONTEXT.md's figure precisely. |
| `crates/nono-cli/tests/spiffe_run.rs` (+285, new) | n/a | `Command::new(env!("CARGO_BIN_EXE_nono"))`, standard integration-test scaffolding | Standard pattern used elsewhere in `nono-cli/tests/` | n/a | HIGH | ADOPT verbatim. **Both** of this file's 2 tests `return` immediately without `SPIRE_AGENT_SOCKET` set — CONFIRMED: unlike `spiffe_integration.rs`, this file has ZERO fail-closed-without-SPIRE assertions; it is 100% live-agent-gated. Uses a plain `std::net::TcpListener` mock server (`MockHttpServer::start()`), NOT Python — CONFIRMED, resolving CONTEXT.md's flagged-for-re-verification claim. |
| `crates/nono-proxy/tests/spiffe_integration.rs` (+188, new) | n/a | `nono_proxy::server::start`, `nono_proxy::spiffe::SpiffeJwtSource` | `server::start` present (`server.rs:519`, `pub async fn start`) | Exact | HIGH | ADOPT verbatim. 1 of 4 tests (`test_spiffe_jwt_fails_closed_on_missing_socket`) runs unconditionally (no SPIRE needed) and asserts a `SpiffeError::Config` string; the other 3 are `SPIRE_AGENT_SOCKET`-gated. |
| `.github/workflows/spire.yml` (+112, new) | n/a | GitHub Actions, `dtolnay/rust-toolchain`, SPIRE 1.9.6 binary release | The fork has other `.github/workflows/*.yml` lanes to compare shape against | n/a | MEDIUM | ADOPT with an action-pin refresh check — the diff pins `actions/checkout@9c091bb2...` (v7.0.0) and `actions/cache@27d5ce7f...` (v5); the planner should confirm these SHAs are still current at implementation time rather than trusting a commit from 2026-07-16 verbatim (pins drift). Otherwise self-contained: downloads SPIRE 1.9.6 as a release tarball, no fork-specific coupling. |
| `Makefile` (+3), `.gitignore` (+/-2) | n/a | `test-spiffe` target, `.PHONY` list | n/a | n/a | HIGH | ADOPT the `Makefile` `test-spiffe` target (mechanical). **DO NOT adopt the `.gitignore` hunk** — upstream's diff removes a bare `proj/` ignore-pattern and adds `demo/` for reasons specific to upstream's own repo layout (upstream's `proj/` and `demo/` do not correspond to this fork's `proj/ADR-*.md` directory, which this very phase needs unblocked — see Common Pitfalls). |

### ⚠ Decision Conflicts

None of D-01 through D-12's LOCKED VERDICTS are contradicted — every re-verified claim (D-01's drop-tls_intercept call, D-03's fail-closed-at-request-time framing, D-04's adopt-async call, D-06's TcpListener-not-Python claim, D-08's ADR-86-intact reading, D-09's nono-py struct-drift prediction) held up exactly as locked. What is contradicted is the **evidentiary premise embedded in D-01's own supporting text and CONTEXT.md's `<code_context>` "Reusable Assets" inventory**, which materially understates the size and risk of the absorb:

1. **CONTEXT.md's `<code_context>` states:** *"`crates/nono-proxy/src/oauth2.rs` (+290 upstream) — present; SPIFFE assertion flow extends it"* and *"`crates/nono-proxy/src/credential.rs` (+243 upstream) — present; gains `get_spiffe_assertion`."* Both are technically true (the files exist, the diff does add those symbols) but omit that the entire OAuth2-route-wiring layer those hunks build ON TOP OF (`CredentialStore.oauth2_routes`, `get_oauth2()`, `OAuth2Route`, `handle_oauth2_credential`) is **itself absent from the fork**, tracing to unabsorbed upstream commit `b1ecbc02` (pre-`v0.66.0`, outside this milestone's sync window, never in any DIVERGENCE-LEDGER). The planner should read `credential.rs`'s and `route.rs`'s disposition rows above as "build a SPIFFE-only slice from scratch," not "extend existing wiring."
2. **D-01's own evidence text states:** *"`reverse.rs` (which the fork HAS) carries the primary SPIFFE path — `handle_spiffe_route`, `handle_spiffe_assertion_credential` ... credential injection ..."* — implying `reverse.rs` is close to a drop-in target. In fact `handle_spiffe_route` as diffed references `crate::forward::{UpstreamSpec, UpstreamStrategy, AuditCtx, forward_request}`, a module that is **completely absent from the fork** (independently confirmed by `112-OAUTH-CAPTURE-DISPOSITION.md` for a different reason — `forward.rs`'s creating commit `149abde0` was also never absorbed). This function must be rewritten against the fork's own `parse_upstream_url()`/`connect_upstream_tls()` idiom, not ported.
3. **A previously-undocumented `git log -S` finding, new to this research pass:** the fork's OWN code comment at `route.rs:617-618`/`633-639` misattributes the missing `requires_managed_credential`/`managed_auth_mechanism`/etc. machinery to *"the upstream tls_intercept module not present in this fork"* — this is factually incorrect (verified via `git log --all -S"handle_oauth2_credential" -- crates/nono-proxy/src/reverse.rs` on the `upstream` remote, which traces the machinery to `b1ecbc02`, `git describe` = `v0.38.0-3-gb1ecbc02`, unrelated to `tls_intercept`). This stale comment should be corrected while this phase touches `route.rs`, so a future absorb doesn't inherit the same wrong mental model.

None of this reverses D-01 (ADAPT-DOWN is still correct — the tls_intercept-adjacent hunks genuinely have nowhere to land) or D-04 (adopt-async is still correct and, per the blast-radius analysis below, cheaper than feared). It changes the *shape* of the "adapt" work from "extend existing scaffolding" to "build a narrow, SPIFFE-scoped slice of missing scaffolding, using upstream's diff as a design reference." ADR-113 should record this explicitly so a future reader does not re-derive it.

## D-04 Blast-Radius Analysis: `RouteStore::load` → async

Every call site of `RouteStore::load` across the entire fork (all 5 crates, `bindings/c/`, and both sibling binding repos) was enumerated by `grep -rn "RouteStore::load\b"`:

| Call site | Context | Action needed |
|---|---|---|
| `crates/nono-proxy/src/server.rs:557` | **The one production call site.** Inside `pub async fn start(config: ProxyConfig) -> Result<ProxyHandle>` (`server.rs:519`) — already `async fn`. | Add `.await`. One-line change. |
| `crates/nono-proxy/src/reverse.rs:1406` | Test-only (`#[test] fn` in `mod tests`) | Convert to `#[tokio::test] async fn`, add `.await` — mirrors upstream's own test-file diff exactly. |
| `crates/nono-proxy/src/route.rs` (6 sites, all `mod tests`) | Test-only | Same conversion, mirrors upstream's diff line-for-line (already captured in the `route.rs` diff read during this research pass). |
| `crates/nono-cli/src/*`, `bindings/c/src/*` | **Zero hits.** `RouteStore` is never referenced outside `nono-proxy` — confirmed by `grep -rn "RouteStore::load\b" crates/nono-proxy/src/ crates/nono-proxy/tests/ crates/nono-cli/src/ crates/nono-cli/tests/ bindings/c/src/` returning matches only within `nono-proxy`. | No action. |
| `../nono-py`, `../nono-ts` | Neither sibling repo constructs or calls `RouteStore` directly (they construct `RouteConfig`/`ProxyConfig` structs, which are separate types) — confirmed no `RouteStore` symbol appears in either repo. | No action for D-04 specifically (see D-09 section for the SEPARATE `RouteConfig.spiffe` struct-literal issue, which is real). |

**Conclusion: D-04's "signature churn across all callers" concern is much smaller in practice than its own text anticipated** — 1 production site + 7 test sites, all within `nono-proxy`, zero cross-crate or cross-repo impact. This lowers, not raises, confidence that D-04 (adopt) was the right call.

## D-09: `nono-py` Struct-Drift — Confirmed Exact Break Site

`../nono-py/src/proxy.rs:206-218` constructs `RustRouteConfig` (the PyO3 wrapper's inner type) as an **exhaustive struct literal**:
```rust
inner: RustRouteConfig {
    // ... (other fields) ...
    oauth2: None,
    aws_auth: None,
}
```
confirmed via direct grep (`grep -n "oauth2\|aws_auth" ../nono-py/src/proxy.rs`). Once `RouteConfig` (aliased as `RustRouteConfig` on the `nono-py` side) gains `spiffe: Option<SpiffeAuthConfig>`, this literal will fail to compile with `E0063` (missing field `spiffe`) — the exact 4th-consecutive-occurrence pattern CONTEXT.md predicts. **The concrete fix:** add `spiffe: None,` at `../nono-py/src/proxy.rs:218` (immediately after the existing `aws_auth: None,` line). `../nono-ts` is confirmed structurally immune — it has zero references to `nono_proxy`/`nono-proxy` anywhere in its source tree (checked via `grep -rln "nono_proxy\|nono-proxy" ../nono-ts/`, no results), consuming only `nono::query::*`.

## SC2: Profile/Schema Configuration Surface

**Concrete field shape** (derived from the diff's `profile/mod.rs` hunk, ADOPT-verbatim per the disposition table above):

`crates/nono-cli/src/profile/mod.rs`, on `CustomCredentialDef` (`:369`):
```rust
/// SPIFFE/SPIRE Workload API auth. Mutually exclusive with `credential_key`, `auth`, and `aws_auth`.
#[serde(default)]
pub spiffe: Option<nono_proxy::config::SpiffeAuthConfig>,
```
Validation additions to `validate_custom_credential()`: (a) mutual-exclusion check — `spiffe.is_some() && (credential_key.is_some() || auth.is_some() || aws_auth.is_some())` → `NonoError::ProfileParse`; (b) the "at least one auth mechanism" check gains `&& cred.spiffe.is_none()`; (c) a new `validate_header_name()` helper (extracted, reusable) validates `inject_header` for SPIFFE routes the same way `validate_header_mode()` already does for `credential_key` routes. All four symbols this needs (`NonoError::ProfileParse`, `CustomCredentialDef`, `validate_custom_credential`, `is_http_token_char`) are confirmed present at matching names.

**Schema additions required** (`crates/nono-cli/data/nono-profile.schema.json`) — the fork-only strict-schema companion file upstream never touches, per the Phase 110 lesson CONTEXT.md cites:
1. `CustomCredentialDef.properties.spiffe` — new property, pattern to copy is the existing `aws_auth` property (`schema.json:638-644`, `oneOf: [{"$ref": "#/$defs/AwsAuthConfig"}, {"type": "null"}]`).
2. New `$defs/SpiffeAuthConfig` — `type: object`, `discriminator`-style `type: "jwt"` tag (mirror the existing `$defs/AwsAuthConfig` or `$defs/InjectMode` enum-object shape), properties `workload_api_socket` (string, required), `audience` (array of string, required), `inject_header` (string, default `"Authorization"`), `credential_format` (nullable string), `svid_hint` (nullable string).
3. `$defs/OAuth2Config` (`schema.json:707-731`) needs THREE changes, all currently missing: (a) add `client_assertion` property (`oneOf: [{"$ref": "#/$defs/ClientAssertionConfig"}, {"type": "null"}]`), (b) add `extra_params` property (`type: object, additionalProperties: {"type": "string"}`), (c) **remove `client_id`/`client_secret` from `required`** — the schema currently has `"required": ["token_url", "client_id", "client_secret"]` with `"additionalProperties": false`, which will hard-reject any profile using `client_assertion` instead of `client_id`/`client_secret` (the Rust struct already makes both `#[serde(default)]` for exactly this reason — the schema must follow).
4. New `$defs/ClientAssertionConfig` — same discriminated-union shape as `SpiffeAuthConfig`, `type: "spiffe_jwt"`, properties `workload_api_socket`, `audience`, `svid_hint`.

All four are additive to a schema with `"additionalProperties": false` at both `CustomCredentialDef` and (presumably) `OAuth2Config` — omitting any of them causes `validate_against_schema()` to reject an otherwise-valid profile at parse time, exactly the Phase 110 failure mode.

## Architecture Patterns

### System Architecture Diagram

```
Sandboxed child process (untrusted agent)
      │  plain HTTP request to http://{PREFIX}_BASE_URL/... (SDK-style env var)
      │  OR CONNECT/absolute-form via HTTP_PROXY (no credentials attached)
      ▼
nono-proxy (server.rs::handle_connection dispatcher)
      │
      ├─ absolute-form self-request to proxy's own bound_port? ──► delegate to handle_reverse_proxy
      ├─ CONNECT tunnel?           ──► connect.rs::handle_connect (D-03: must fail-closed check spiffe route)
      ├─ plain-HTTP forward-proxy? ──► server.rs::handle_forward_http (D-03: must fail-closed check spiffe route)
      ├─ external-proxy chain?     ──► external.rs::handle_external_proxy (D-03: must fail-closed check spiffe route)
      └─ path-prefix reverse-proxy ──► reverse.rs::handle_reverse_proxy
                                             │
                                             ├─ route.spiffe route? ──► handle_spiffe_route()
                                             │      │
                                             │      ├─ validate_reverse_local_auth() [session token / phantom]
                                             │      │        (fail → 407, audit denied)
                                             │      ├─ route.managed_auth.acquire()
                                             │      │      └─ ManagedUpstreamAuth::SpiffeJwt
                                             │      │            └─ SpiffeJwtSource::fetch_token()
                                             │      │                  └─ SPIRE Workload API (Unix socket, gRPC)
                                             │      │                       (fail → 503, audit denied — FAIL CLOSED)
                                             │      ├─ inject Bearer/custom header
                                             │      └─ forward via fork's own parse_upstream_url()+connect_upstream_tls()
                                             │           (NOT upstream's forward.rs — that module is absent)
                                             │
                                             └─ oauth2.client_assertion (SPIFFE-jwt-bearer) route?
                                                    └─ handle_spiffe_assertion_credential()
                                                          └─ SpiffeAssertionTokenCache::get_or_refresh()
                                                                └─ exchange_jwt_assertion() [oauth2.rs, raw HTTP/TLS]
                                                                      └─ SpiffeJwtSource::fetch_token() (RFC 7523 assertion)
                                                                            └─ IdP token endpoint (HTTPS)
                                                                                  (fail → 503, audit denied)

Startup path (RouteStore::load, now async):
  profile.custom_credentials["x"].spiffe  →  nono-cli/profile/mod.rs validate_custom_credential()
       →  network_policy.rs resolve_credentials()  →  RouteConfig.spiffe
       →  RouteStore::load().await  →  SpiffeJwtSource::connect() [fail-closed: proxy startup aborts
                                          if socket unreachable, ONLY for profiles that declare spiffe]
       →  proxy_runtime.rs enforce_spiffe_socket_isolation()  →  CapabilitySet deny/no-grant of
                                          the SPIRE socket path for the sandboxed child
```

### Recommended Project Structure

No new directories — all new files land at the existing flat `crates/nono-proxy/src/*.rs` level (`spiffe.rs`, `auth.rs`), matching the fork's existing convention (no `tls_intercept/`-style subdirectory needed since D-01 drops that half).

### Pattern 1: Fail-closed managed-credential dispatch (D-03's enforcement point)

**What:** Every request-entry path in `nono-proxy` that CAN reach a SPIFFE-declared route must explicitly check whether the resolved route requires SPIFFE and, if so, either invoke the SPIFFE handler or hard-deny — never silently pass through unauthenticated.
**When to use:** Any of `connect.rs::handle_connect`, `server.rs::handle_forward_http`, `external.rs::handle_external_proxy` — the three paths D-01 confirms have no SPIFFE implementation of their own.
**Example (the exact fixed shape to mirror, from WR-13's resolution, `112-REVIEW.md:419-436`):**
```rust
// Source: crates/nono-proxy/src/server.rs, WR-13 fix pattern (112-REVIEW.md)
// Mirror this shape for D-03: a route-level boolean gates the check.
if state.config.require_auth {
    if let Err(e) = token::validate_proxy_auth(header_bytes, &state.session_token) {
        // ... 407, audit denied ...
    }
}
```
For D-03 specifically, the analogous per-path guard is: after route resolution but before forwarding, `if let Some(route) = resolved_route { if route.has_spiffe_source() { /* deny or delegate — this path has no SPIFFE impl */ } }`. This **composes with** (does not duplicate) Phase 112's `require_auth`/`strict_connect_auth` — those gate the SESSION-TOKEN check; D-03's new guard gates the SEPARATE question of "does this specific route need a credential this code path cannot supply."

### Anti-Patterns to Avoid

- **Porting `handle_spiffe_route` verbatim and hoping it compiles:** it references `crate::forward` which does not exist. Always rewrite against `parse_upstream_url()`/`connect_upstream_tls()`.
- **Building the general (non-SPIFFE) OAuth2 `client_credentials` route-wiring layer "since we're already in there":** that's `b1ecbc02`, out of NET-02's scope, and would silently expand this absorb's blast radius (more code = more attack surface for a security tool, with no requirement backing it).
- **Adopting the diff's `.gitignore` hunk:** removes this fork's own `proj/` history-preservation semantics for an upstream-specific reason unrelated to this repo.
- **Trusting `git apply` for any of these files:** every non-trivial file in this absorb (`credential.rs`, `route.rs`, `reverse.rs`) has context lines that reference symbols absent from the fork — `git apply --check` will fail immediately, exactly as it did for Phase 112's SEC-07 absorb (`112-07-SUMMARY.md`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| SPIFFE Workload API client (gRPC, JWT-SVID parsing/validation) | Hand-rolled Unix-socket gRPC client | `spiffe` crate (`spiffe_workload::JwtSource`) | Official ecosystem client; hand-rolling gRPC framing + JWT-SVID trust-domain validation for a security-critical identity path is precisely the "crypto/keystore, don't hand-roll" class of risk CLAUDE.md already generalizes. |
| JWT `nbf`/`exp` validation | Custom JWT parsing throughout | The diff's `check_nbf` (base64url payload decode + `serde_json::Value` claim inspection) — SPIRE has already verified the signature; this is JSON parsing only, not cryptographic verification | Verified-signature JWTs only need claim-freshness checks, which the diff already implements minimally and correctly (once the let-chain is rewritten) — do not add a full JWT library dependency for this. |
| OAuth2 RFC 7523 jwt-bearer token exchange | New HTTP client dependency | `oauth2.rs`'s existing raw-HTTP-over-`tokio_rustls::TlsConnector` machinery (`exchange_jwt_assertion`, `read_http_response`, `parse_token_response`, chunked-decoding) | Already exists, already tested, already used for the sibling `client_credentials` flow — extending it keeps the dependency surface flat. |

**Key insight:** The two things worth NOT hand-rolling here (SPIFFE protocol, JWT claim parsing) are both already solved by the diff itself using upstream's design — the phase's actual hand-rolling risk is the opposite direction: don't hand-roll a REPLACEMENT for `forward.rs`'s upstream-connect abstraction when the fork's simpler, pre-existing `parse_upstream_url`/`connect_upstream_tls` already does the job for this fork's narrower feature set (no HTTP-vs-HTTPS scheme selection needed, since the fork's proxy always speaks TLS to upstream today).

## Common Pitfalls

### Pitfall 1: Let-chains fail `cargo fmt --all --check`, not `cargo build`

**What goes wrong:** `cargo build`/`cargo check` silently accept `if let ... && let ...` syntax because `rustc` 1.95 supports let-chains at the language level; `cargo fmt --all --check` (part of `make ci`) rejects it as invalid for this workspace's `edition = "2021"`.
**Why it happens:** Let-chain stabilization is edition-gated; this workspace has not migrated to the edition where it's valid. Confirmed empirically by Plan 112-02's identical discovery ("a ported let-chain, invalid under this workspace's Rust 2021 edition ... `cargo build` silently accepted it but `cargo fmt --all --check` correctly rejected it").
**How to avoid:** Two confirmed instances in this diff — `spiffe.rs::check_nbf`'s `if let Some(nbf) = claims.get("nbf")... && now_ts < nbf` and `proxy_runtime.rs`'s host-collection loop `if let Ok(parsed) = url::Url::parse(...) && let Some(host) = parsed.host_str()`. Rewrite both to nested `if let` (the codebase's own `hook_runtime.rs` precedent, per 112-02's fix).
**Warning signs:** `cargo build --workspace --all-targets` exits 0 but `cargo fmt --all --check` (or `make ci`) fails on these two files specifically.

### Pitfall 2: `.gitignore:16`'s bare `proj/` pattern silently swallows the new ADR

**What goes wrong:** `proj/ADR-113-spiffe-disposition.md` (required by SC1) is a brand-new file under `proj/`. `.gitignore:16` contains a bare `proj/` pattern. Confirmed empirically this session: `touch proj/test-ignore-check.md && git status --porcelain proj/test-ignore-check.md` produces NO output, and `git check-ignore -v proj/test-ignore-check.md` exits 0 (ignored) with match `.gitignore:16:proj/`.
**Why it happens:** Already-tracked files under `proj/` (e.g. `proj/ADR-111-*.md`, confirmed present via `git ls-files proj/`) are unaffected — git does not un-track a file merely because a later `.gitignore` rule would now match it. But a genuinely NEW file is invisible to plain `git add`/`git status`.
**How to avoid:** `git add -f proj/ADR-113-spiffe-disposition.md` explicitly when committing. Analogous to the existing memory entry for `docs/cli/development/` (also gitignored-but-tracked).
**Warning signs:** `git status` after writing the ADR shows no untracked file; a subsequent `git commit` silently omits it.

### Pitfall 3: The fork's own code comments about "why this is missing" can themselves be wrong

**What goes wrong:** `route.rs:617-618`/`633-639` attributes the absence of `requires_managed_credential`/`managed_auth_mechanism`/etc. to "the upstream tls_intercept module not present in this fork" — this research traced the real ancestor to a completely unrelated commit (`b1ecbc02`, OAuth2 profile support). Trusting an in-repo comment's causal claim without re-deriving it via `git log -S` would have propagated a wrong mental model into ADR-113.
**Why it happens:** A prior phase (likely Phase 109, writing the `no_proxy` overhaul) needed to explain a gap it wasn't scoped to fix and picked the most locally-plausible explanation (tls_intercept was the OTHER known gap at the time) without checking `git log -S`.
**How to avoid:** For any "this is missing because of X" comment encountered during an absorb, verify X with `git log --all -S"<distinctive symbol>" -- <path>` on the `upstream` remote before repeating the claim in a new document.
**Warning signs:** A causal claim in a code comment that isn't backed by a cited commit SHA.

### Pitfall 4: `git apply`/literal-port instinct fails on every non-trivial file here

**What goes wrong:** Attempting `git apply --check` (or eyeballing the diff as "just a patch") on `credential.rs`, `route.rs`, or `reverse.rs` will fail or, worse, if forced with `--3way`, produce code that references non-existent symbols and fails to compile with confusing errors far from the actual cause.
**Why it happens:** Every one of these three files has upstream base-tree context lines (unrelated to SPIFFE) that don't match the fork's tree — this diff was generated against upstream's much larger, more feature-complete `nono-proxy`.
**How to avoid:** Treat the diff purely as a **design reference** for these three files; write the adaptation by hand, function by function, against the fork's actual current symbols (as enumerated in the disposition table above). This mirrors the established precedent from `112-07-SUMMARY.md` ("`git apply --check` failed outright for 112-07's commit").
**Warning signs:** `patch`/`git apply` reports "hunk failed to apply" at context lines nowhere near the actual new SPIFFE code.

## Code Examples

### Existing pattern: `ManagedUpstreamAuth`-style enum for future extensibility (from the diff, ADOPT verbatim)

```rust
// Source: c831dade, crates/nono-proxy/src/auth.rs (new file, ADOPT verbatim)
pub enum ManagedUpstreamAuth {
    /// JWT-SVID fetched from the SPIRE Workload API and injected as a bearer token.
    SpiffeJwt(Arc<crate::spiffe::SpiffeJwtSource>),
}
```
Comment convention at the top of the file: *"Adding a new auth mechanism: add a variant here and a corresponding handler branch in reverse.rs"* — a deliberate extension point; the planner should preserve this doc comment as-is since it correctly describes the SPIFFE-only scope this phase should keep.

### Fork's existing upstream-connect idiom to rewrite `handle_spiffe_route` against (do NOT use `forward::forward_request`)

```rust
// Source: crates/nono-proxy/src/reverse.rs (fork, current), lines 599 and 644 (fn signatures)
fn parse_upstream_url(url_str: &str) -> Result<(String, u16, String)> { /* host, port, path — no scheme */ }
async fn connect_upstream_tls(/* ... */) -> Result<TlsStream<TcpStream>> { /* always TLS */ }
```
The `handle_spiffe_route` adaptation should call these two functions (matching the pattern the un-diffed portion of `handle_reverse_proxy` already uses for `static_cred`-based routes) instead of constructing an `UpstreamSpec`/`UpstreamStrategy::Direct` and calling `forward::forward_request`.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| Static keystore credential per route (pre-existing fork feature) | SPIFFE/SPIRE workload identity, machine-verified at request time, no long-lived secret stored anywhere | This absorb (upstream `c831dade`, 2026-07-16) | Zero-trust-style credential rotation with no keystore entry to leak; the tradeoff is a live runtime dependency on a SPIRE agent being reachable, which is why D-03's fail-closed design matters. |

**Deprecated/outdated:** upstream's `log_reverse_proxy` (deprecated since upstream 0.46.0) is dead in upstream's tree but LIVE in the fork's — do not delete it as part of this absorb (see Pitfall/disposition-table note above).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | The 19-crate transitive dependency count and the specific 18 named crates (beyond `spiffe` itself) are accurate. This research independently confirmed each NAMED crate exists on crates.io and passes `slopcheck`, but did NOT run `cargo tree` against the actual dependency graph (would require adding `spiffe` to `Cargo.toml`, which this research-only pass does not do). | Standard Stack, Package Legitimacy Audit | Low — if the real transitive count differs slightly, the audit table simply needs re-running post-dependency-add; no crate on the list failed legitimacy checks. |
| A2 | `.github/workflows/spire.yml`'s pinned action SHAs (`actions/checkout@9c091bb2...`, `actions/cache@27d5ce7f...`) are still current/valid at plan-execution time. | Symbol-level disposition table (CI row) | Low-medium — a stale/moved SHA would fail the workflow at first CI run, caught immediately, not a security issue. |
| A3 | The `jni` feature pulled in via `rustls-platform-verifier` is genuinely Android-cfg-gated at the Cargo dependency-resolution level (confirmed the feature is opt-in via `cargo info`, but the actual per-target `cargo tree` proof requires the dependency to be present in `Cargo.lock`, which it is not yet). | D-05(c), Package Legitimacy Audit | Medium — this is exactly why D-05(c) is a proof obligation, not a footnote; if wrong, JNI actually links on a shipped target and must be fixed via feature trimming before release. |
| A4 | No other upstream commit between `b1ecbc02` (the OAuth2-route-wiring ancestor) and `c831dade` further modified `CredentialStore`/`route.rs`'s managed-credential machinery in a way this research's single `git log -S` query missed. | Symbol-level disposition table (credential.rs/route.rs rows), Decision Conflicts | Low-medium — a missed intermediate commit could mean a slightly different exact shape is needed for the SPIFFE-only slice; the core finding (the layer is absent, ancestor predates the sync window) is independently confirmed by TWO separate grep/read passes against the live fork tree regardless of which upstream commit(s) are the true full ancestry. |

**If this table is empty:** N/A — see rows above.

## Open Questions (RESOLVED)

1. **Should `route.rs:617-639`'s misattributed comment be corrected as part of THIS phase, or is that scope creep?**
   - What we know: the comment is factually wrong (attributes the gap to `tls_intercept`, actual ancestor is `b1ecbc02`) and this phase must touch `route.rs` anyway to add `managed_auth`/`has_spiffe_source()`.
   - What's unclear: whether "correct a misleading comment while editing the same file" counts as in-scope maintenance or an unrelated change needing its own justification.
   - Recommendation: correct it — the cost is one comment edit, and leaving it wrong actively misleads the next absorb (per Pitfall 3 above), which is a documented failure mode this milestone has hit repeatedly (Phase 108's D-06 substring-glob miss, 109's three "already present" misses).
   - **RESOLVED:** see OD-2 — Plan 113-04's Task 2 corrects the comment in place, citing the real ancestor commit (`b1ecbc02`) and this phase's own ADR (`proj/ADR-113-spiffe-disposition.md`).

2. **Is the SPIFFE-only OAuth2-assertion slice (declining `b1ecbc02`'s general `client_credentials` wiring) the right scope call, or should ADR-113 recommend absorbing `b1ecbc02` first as a prerequisite?**
   - What we know: `b1ecbc02` is 974 lines across 8 files, entirely outside this milestone's sync window and never audited by Phase 108. NET-02's requirement text only asks for SPIFFE.
   - What's unclear: whether declining it creates a permanently-asymmetric `CredentialStore` (SPIFFE assertion routes wired, plain OAuth2 client_credentials routes still not) that a future upstream sync will have to reconcile awkwardly.
   - Recommendation: ADR-113 should record this as a deliberate, named scope boundary (mirroring ADR-111's "reject the core-module absorb" shape) rather than silently building only what SPIFFE needs — a future planner absorbing `b1ecbc02` proper should find this ADR and understand the SPIFFE-only slice it's reconciling against.
   - **RESOLVED:** see OD-1 — the phase implements the SPIFFE-only slice (Plans 113-03/113-04/113-06 build only `spiffe_assertion_routes`/`get_spiffe_assertion()`, declining `b1ecbc02`'s general `oauth2_routes`/`OAuth2Route`/`get_oauth2()` layer), and Plan 113-08's ADR-113 records this as a named, permanent scope boundary plus a carry-forward note in `108-DIVERGENCE-LEDGER.md`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| Docker Linux engine | D-12 linux-gnu cross-target clippy gate | ✓ | Server 29.6.2 | — |
| `zig` / `cargo-zigbuild` | D-12 apple-darwin cross-target clippy gate | ✓ | both on PATH | — |
| `cargo-audit` | D-05(b) dependency-audit proof | ✓ | 0.22.1 | — |
| SPIRE server/agent binaries | D-06 local dev-time live testing | ✗ (Windows dev host) | — | Not needed locally — live tests are `SPIRE_AGENT_SOCKET`-gated and only exercised in the new `spire.yml` Linux CI lane; the fail-closed tests run everywhere without SPIRE. |
| `python3` in the local `cross` container | N/A for this phase | ✗ (known, pre-existing gap) | — | Both new test files (`spiffe_run.rs`, `spiffe_integration.rs`) use pure-Rust mocking (`TcpListener`), CONFIRMED — this phase's tests are NOT affected by the container's missing python3, unlike `socket_access_run.rs`'s tests in prior phases. |

**Missing dependencies with no fallback:** none — SPIRE binaries are CI-only by design (D-06), not a local dev-time requirement.
**Missing dependencies with fallback:** SPIRE server/agent (fallback: fail-closed tests + CI-gated live tests, per above).

## Validation Architecture

### Test Framework

| Property | Value |
|---|---|
| Framework | Rust built-in test runner (`cargo test`), `#[tokio::test]` for async |
| Config file | none — standard `Cargo.toml` test targets; new CI lane at `.github/workflows/spire.yml` |
| Quick run command | `cargo test -p nono-sandbox-proxy --test spiffe_integration -- --nocapture` (fail-closed test runs everywhere, <1s) |
| Full suite command | `bash scripts/spire-test.sh` (adopted `make test-spiffe` target; downloads/runs live SPIRE — Linux CI only) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| NET-02 (SC1) | ADR-113 exists and settles disposition | manual-only (doc review) | `test -f proj/ADR-113-spiffe-disposition.md` | ❌ this phase creates it |
| NET-02 (SC2) | SPIFFE JWT config round-trips through profile schema | unit | `cargo test -p nono-sandbox-cli --lib -- profile::tests::test_validate_custom_credential_spiffe` | ❌ Wave 0 — port from diff |
| NET-02 (SC2) | `RouteConfig.spiffe` serde round-trip | unit | `cargo test -p nono-sandbox-proxy --lib -- config::tests::test_spiffe_jwt_config_roundtrip` | ❌ Wave 0 — port from diff |
| NET-02 (SC2/D-03) | SPIFFE route without live agent fails closed at proxy startup (SPIFFE routes only) | integration | `cargo test -p nono-sandbox-proxy --test spiffe_integration -- test_spiffe_jwt_fails_closed_on_missing_socket --nocapture` | ❌ Wave 0 — port from diff, runs everywhere without SPIRE |
| NET-02 (D-03) | A SPIFFE-declared route hit via CONNECT/forward-HTTP/external-proxy fails closed, not silently unauthenticated | unit/integration | new test, no upstream equivalent — must be authored | ❌ Wave 0 — NEW, not in upstream's diff (D-03 is a fork-original requirement) |
| NET-02 (SC3) | ADR-86 boundary non-regressed (audit-only additions) | unit | `cargo test -p nono-sandbox --lib -- audit::tests` (existing suite, extended with `spiffe_context: None,` fixtures) | ✓ existing, extend |
| NET-02 (SC4) | Cross-target clippy GREEN both gates | build/lint | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` + `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✓ existing gate infra |
| NET-02 (SC4) | `maturin build` / `napi build` green after `nono-proxy` struct change | build | `maturin build` (in `../nono-py`, after the `spiffe: None,` fix); `napi build --platform --release` (in `../nono-ts`, expected no-op) | ✓ existing gate infra |
| NET-02 (D-07) | Loud skip reporting: any SPIRE-gated test that skips is named/counted/surfaced | new mechanism | see Wave 0 Gaps below | ❌ this phase must design it |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-sandbox-proxy --lib --test spiffe_integration -- --nocapture` (fast, no SPIRE needed for the fail-closed subset).
- **Per wave merge:** both cross-target clippy gates + `cargo fmt --all --check` + full `cargo test --workspace --no-fail-fast` (diffed against the documented pre-existing failing-test baseline, per prior-phase precedent).
- **Phase gate:** `spire.yml`'s new Linux CI lane is the only place the live-agent tests genuinely execute end-to-end — local/dev verification cannot fully close D-06's loop without either a locally-installed SPIRE agent (not attempted this session; out of scope for research) or trusting the new CI lane's first real run.

### D-07 Loud-Skip-Reporting Mechanism — Design Proposal

**Survey of existing fork precedent:** no generalized skip-reporting harness exists today. The closest precedent is `socket_access_run.rs`'s ad-hoc pattern — individual tests `eprintln!("skipping: python3 not available")` and return early, with no aggregation, counting, or surfacing into any verification artifact; this exact gap is what let a 0.01s all-`ok` run nearly pass as evidence in Phase 112.

**Minimum generalizable mechanism proposed** (for the planner to size into a task):
1. Every skip-capable test in `spiffe_run.rs`/`spiffe_integration.rs` (ported from upstream, using `return;`/early-`None`-match idioms) should be changed to print a **structurally greppable** marker on skip, e.g. `eprintln!("SKIP[{}]: SPIRE_AGENT_SOCKET not set", module_path!())` (or a small shared `test_skip!(reason)` macro placed somewhere reusable, e.g. `crates/nono-proxy/tests/common/mod.rs` — no such shared test-helper module currently exists in `nono-proxy/tests/`, confirmed by directory listing, so this would be a new but minimal addition).
2. The phase's verification pass runs `cargo test ... -- --nocapture 2>&1 | grep -c "^SKIP\["` and includes that count explicitly in the phase's `human_verification_truths`/verification report — e.g. *"4 SPIFFE tests SKIPPED (no local SPIRE agent): spiffe_jwt_credential_injected_end_to_end, spiffe_jwt_proxy_starts_with_live_agent, test_spiffe_jwt_live_fetch, test_spiffe_jwt_live_delegation_none_on_plain_svid, test_spiffe_jwt_live_proxy_startup — compensated by the new spire.yml CI lane, not locally exercised."*
3. This generalizes beyond SPIFFE by being a plain `stderr` string convention, not a new test framework — any future skip-capable test (e.g. a future GPU-gated or python3-gated test) can adopt the same `SKIP[...]` prefix and be counted the same way, closing the exact gap that let `socket_access_run.rs`'s silent 0.01s pass through undetected.

### Wave 0 Gaps

- [ ] `crates/nono-proxy/tests/spiffe_integration.rs` — port from diff, covers SC2/D-03(partial)/D-06.
- [ ] `crates/nono-cli/tests/spiffe_run.rs` — port from diff, covers SC2 end-to-end (SPIRE-gated only).
- [ ] A NEW test (no upstream equivalent) asserting D-03's fail-closed behavior on the CONNECT/forward-HTTP/external-proxy paths specifically — upstream's diff does not need this test because upstream HAS a `tls_intercept` implementation on those paths; this fork does not, so this is a fork-original test obligation.
- [ ] The D-07 `SKIP[...]` marker convention and its grep-count verification step — no existing infra to extend.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V2 Authentication | yes | JWT-SVID validated by the SPIRE agent (signature) + this proxy's own `exp`/`nbf` freshness checks (`check_nbf`); session-token/phantom-token validation (`validate_reverse_local_auth`) gates the client→proxy leg separately from the proxy→SPIRE leg. |
| V3 Session Management | yes | Existing fork session-token mechanism (`token::validate_proxy_auth`), unmodified by this absorb — SPIFFE routes reuse it for the client-facing leg. |
| V4 Access Control | yes | `enforce_spiffe_socket_isolation()` — denies the sandboxed child direct Workload-API-socket access; hard-errors (not warns) on a conflicting `unix_socket` grant. |
| V5 Input Validation | yes | `SpiffeAuthConfig`/`ClientAssertionConfig` deserialize via `serde` with `#[serde(tag = "type", ...)]` discriminated unions (structurally rejects malformed shapes); profile-schema `additionalProperties: false` (strict). |
| V6 Cryptography | yes (delegated, not hand-rolled) | JWT-SVID signing/verification is performed entirely by the SPIRE agent/`spiffe` crate — this absorb must NOT hand-roll any cryptographic verification; the fork-side code only parses already-verified claims (`check_nbf`). |

### Known Threat Patterns for SPIFFE/SPIRE proxy auth

| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| Sandboxed child obtains its own SVID by reaching the SPIRE socket directly | Spoofing / Elevation of Privilege | `enforce_spiffe_socket_isolation()` — deny-by-omission on Linux (no `unix_socket` grant ever issued), explicit Seatbelt `deny network-outbound` on macOS, hard error if conflicting grant detected. **Gap to flag:** the diff's Windows behavior for this control is unclear/undocumented (the `#[cfg(not(target_os = "macos"))]` branch is a log-only no-op, relying on AppContainer's default-deny for unlisted sockets — consistent with the fork's existing Windows sandbox model but not independently verified this session; the planner should confirm AppContainer denies unlisted Unix-domain-socket paths by default, matching Landlock's allow-list-only posture). |
| Fail-open on SPIRE agent unavailability at request time | Denial of Service (safe failure) vs. Tampering (unsafe failure) | D-03/upstream's own design: 503, never forward unauthenticated. This is the single most security-critical property of the whole feature. |
| A SPIFFE-declared route reached via a proxy path with no SPIFFE implementation | Tampering / Elevation of Privilege (silent unauthenticated forward) | D-03's locked, fork-original fail-closed guard — the direct residual risk this phase's own absorb creates by dropping `tls_intercept`. |
| Trust-domain confusion (a JWT-SVID from the wrong trust domain accepted) | Spoofing | Delegated entirely to the `spiffe` crate's `audience` parameter and the SPIRE agent's own trust-domain-scoped Workload API socket — the proxy never independently re-derives trust-domain matching; this is intentional (don't hand-roll), but the planner's threat model should note this as a delegated-trust boundary, not an unaddressed gap. |
| SVID/access-token leakage via logs or audit records | Information Disclosure | `SpiffeJwtSource::fetch_token` returns `Zeroizing<String>`; `SpiffeAuditContext`/`SpiffeDelegationContext` (the audit-record types) carry only `workload_spiffe_id`/`trust_domain`/`svid_type`/delegation metadata — never the raw token, confirmed by direct read of `undo/types.rs`'s new struct fields. |

## Sources

### Primary (HIGH confidence — direct repository inspection)
- `git show c831dade422f2bdf37d7429af0423cafa0a60c06` (full commit + per-file diffs) — the authoritative source for every upstream-hunk claim in this document.
- Live fork tree reads (`Read`/`Grep`/`sed`) against `crates/nono-proxy/src/{reverse,route,credential,oauth2,server,audit,config,spiffe(absent),auth(absent),lib}.rs`, `crates/nono-cli/src/{profile/mod,network_policy,proxy_runtime}.rs`, `crates/nono/src/{undo/types,audit}.rs`, `../nono-py/src/proxy.rs`, `crates/nono-cli/data/nono-profile.schema.json`, `.gitignore`, `Cargo.toml` — all claims about fork state are `[VERIFIED]` this way.
- `git log --all -S"handle_oauth2_credential" -- crates/nono-proxy/src/reverse.rs` (upstream remote) → `b1ecbc02`; `git describe --tags b1ecbc02` → `v0.38.0-3-gb1ecbc02` — the ancestry finding underlying the Decision Conflicts section.
- `cargo info spiffe`, `cargo info rustls-platform-verifier` — live registry queries, 2026-08-06.
- `slopcheck scan --pkg crates.io <name> --json` — live legitimacy checks for 13 named crates, 2026-08-06.
- `cargo audit --version` (0.22.1, installed), `docker info` (Server 29.6.2, running), `zig`/`cargo-zigbuild` on PATH — environment availability, live-probed 2026-08-06.

### Secondary (MEDIUM confidence)
- `.github/workflows/spire.yml`'s pinned Action SHAs (`actions/checkout@9c091bb2...`, `actions/cache@27d5ce7f...`) — read directly from the diff but their continued validity at plan-execution time was not independently re-verified (flagged as Assumption A2).

### Tertiary (LOW confidence)
- None — every claim in this document traces to a primary source above; no web-search-only claims were used for fork-specific or upstream-diff-specific content.

## Metadata

**Confidence breakdown:**
- Standard stack (crate identity/version): HIGH — `[VERIFIED: cargo info]` live registry query.
- Symbol-level disposition table: HIGH for most rows; explicitly LOW→re-verify flagged for `credential.rs`/`route.rs`/`reverse.rs` rows where the absorb shape is genuinely larger/different than CONTEXT.md characterized — this LOW rating is itself the deliverable (a disposition amendment, not an unresolved gap).
- Architecture/pattern guidance: HIGH — grounded in direct reads of the fork's own existing idioms (`connect_upstream_tls`, `ReverseProxyCtx`, WR-13's fixed shape).
- Pitfalls: HIGH — all four are either empirically reproduced this session (`.gitignore` behavior) or directly cite a prior-phase's identical, already-fixed occurrence (let-chains, `git apply` failure mode).
- Validation architecture / D-07 mechanism: MEDIUM — the design is a genuinely new proposal (no existing fork precedent to lift verbatim), sized conservatively to generalize.
- Security domain: MEDIUM — ASVS mapping is straightforward for a proxy-auth feature, but the Windows AppContainer default-deny-for-unlisted-sockets claim (in the threat table) was not independently verified this session and is flagged as such inline.

**Research date:** 2026-08-06
**Valid until:** ~14 days (fast-moving fork under active weekly-cadence upstream sync; `spiffe`/`rustls-platform-verifier` crate versions and the pinned CI Action SHAs are the most likely to drift first).
