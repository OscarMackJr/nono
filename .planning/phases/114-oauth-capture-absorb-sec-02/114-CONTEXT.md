# Phase 114: OAuth Capture Absorb (SEC-02) - Context

**Gathered:** 2026-08-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Resolve SEC-02 — upstream's declarative sandboxed OAuth capture (`9b692e07` / `3c59c62e` /
`d033c631`) — by **delivering the capability**, not merely adjudicating it. The disposition is
**ADAPT**: build a fork-native response buffer-and-rewrite enforcement point on the reverse-proxy
path so real OAuth tokens are replaced with sandbox-visible phantoms before reaching the sandboxed
client.

**The operator instruction that governs this phase:** *"Phase 114 needs to resolve SEC-02."*
A recorded decline was considered and explicitly retracted (see D-01r). SEC-02 closes as a working
capability with a named scope limit, not as a won't-sync.

**Scope limit (load-bearing, must be recorded — see D-10):** capture works for OAuth token
endpoints reachable as **configured reverse-proxy routes**. It does **not** cover agent-initiated
flows to arbitrary hosts via CONNECT — that genuinely requires TLS interception, which this fork
declines by standing decision.

**Out of scope:** TLS interception / `149abde0`; disk persistence of real tokens
(upstream `oauth_capture/persist.rs`); the `tls_intercept/{h2_forward,handle}.rs` hunks.

</domain>

<decisions>
## Implementation Decisions

### The disposition verdict

- **D-01r: ADAPT — build a fork-native response buffer+rewrite on the reverse-proxy path.**
  **This SUPERSEDES a retracted D-01 (formally DECLINE).** The decline was chosen, then withdrawn
  by the operator mid-discussion ("Phase 114 needs to resolve SEC-02, let's revisit"). The
  retraction is recorded deliberately, not erased — the evidence that triggered it matters:
  *Discovered during discussion:* the fork's reverse proxy **already has plaintext visibility into
  upstream responses**. The sandboxed client speaks plain HTTP to the proxy; the proxy holds its
  own `tls_stream` to upstream (`reverse.rs`, verified live). So MITM was never required for token
  endpoints reachable as configured routes — the premise that `forward.rs`'s `ResponseRewrite` was
  the *only possible* enforcement point conflated **visibility** (which the fork has) with
  **buffering** (which it lacks, by deliberate design: responses stream in 8 KiB chunks across
  **three** relay sites, for SSE / MCP Streamable HTTP / A2A JSON-RPC).
  TLS interception (`149abde0`) is **NOT** absorbed; the fork's no-MITM decision stands.
- **D-02r: SC2 is satisfied BY CONSTRUCTION**, via a fork-native enforcement point, rather than by
  importing upstream's `forward.rs` hook. SC2's forbidden outcome — "a reduced-scope absorb that
  drops the rewrite hook without an equivalent enforcement point" — is avoided because an
  equivalent point is *built*, not omitted.
- **D-09: SC3 is LIVE, not vacuous.** The ADAPT ships real code, so the ADR-86 / ADR-111 boundary
  must be confirmed non-regressed — no policy or enforcement logic may land in the core `nono`
  crate. SC4's two cross-target clippy gates are mandatory (no PARTIAL→CI).

### The enforcement point

- **D-05: Tight response-buffer cap, FAIL CLOSED on exceed.** Bound sized for token responses
  (~256 KiB–1 MiB, planner may refine). If an OAuth-capture-declared route returns more than the
  cap, **DENY the response — never release it unrewritten.** Exceeding the cap means the proxy
  cannot guarantee the token was rewritten, and releasing it anyway is exactly the fail-open shape
  SC2 forbids. This also bounds the DoS surface that buffering introduces. Precedent: the fork's
  existing 16 MiB **request**-body cap (`reverse.rs`) — deliberately NOT reused here, being far
  larger than any token response needs.
- **D-06: Buffering is OPT-IN PER ROUTE, plus a D-03-style cross-path fail-closed guard.**
  Only routes explicitly declared as OAuth token endpoints are buffered; **streaming remains the
  default for everything else** — the fork streams deliberately (module doc, `reverse.rs`) and
  buffering SSE / MCP Streamable HTTP / A2A JSON-RPC would break them.
  A capture-declared route arriving via **CONNECT, forward-HTTP, or the external-proxy chain**
  must **fail closed at request time**, since those paths have no rewrite. This mirrors Phase 113's
  D-03 exactly and is a **fork-original test obligation** — upstream ships no equivalent test
  because upstream *has* interception on those paths. See also WR-13 in `112-REVIEW.md`: one
  request path of six ignoring an auth gate is the precise defect shape this guard prevents.
- **D-08: Real OAuth tokens are held IN-MEMORY, SESSION-SCOPED ONLY**, zeroized on drop
  (CLAUDE.md mandates `zeroize` for secrets). Upstream's `oauth_capture/persist.rs` disk
  persistence is **not** adopted: no secret at rest, no file-permission surface, no cross-session
  replay if the host is later compromised. **Accepted cost:** re-login after every proxy restart.

### Absorb surface and shape

- **D-07: Port upstream's config/rewrite LOGIC as a design reference; REBUILD the plumbing.**
  Take the declarative provider config, token-field/phantom-mapping logic, and JWT handling — they
  encode real problem knowledge (which fields OAuth providers actually return) that would be
  wasteful to re-derive. But rebuild the enforcement plumbing against the fork's `reverse.rs` relay
  sites rather than porting `forward.rs`-shaped code. This is Phase 113's proven pattern for
  `reverse.rs` / `route.rs` / `credential.rs`; `git apply` will not work here either.
- **D-12: `profile/credential_provider.rs` is ported ADAPTED, minus absent-subsystem coupling.**
  Bring the declarative provider surface (token endpoint, response/request token fields, API hosts,
  credential-store detection) adapted to the fork's profile shape; drop anything reaching into
  `oauth_capture/` or `forward.rs`. SEC-02 is literally titled *"declarative* sandboxed OAuth
  capture" — dropping the declarative half would under-deliver the requirement.
- **D-13: The profile schema is a FIRST-CLASS TASK with a round-trip test.** Hand-port the needed
  `$defs`/properties into `crates/nono-cli/data/nono-profile.schema.json` (fork-only companion,
  `additionalProperties: false`, upstream never touches it) **and** add a test asserting a realistic
  capture-provider profile both deserializes AND passes `validate_against_schema()`.
  Do **not** port upstream's 265 schema lines wholesale — they describe the persistence and
  interception surfaces this fork is not building, and a schema advertising configuration nothing
  implements is itself a fail-open shape. Phase 110 hit the serde/schema mismatch; Phase 113 needed
  four hand edits.
- **D-14: `nono-py` binding fix is its OWN NAMED TASK, with both bindings rebuilt.**
  `RouteConfig` gains capture config; `../nono-py` constructs it with exhaustive struct literals, so
  it will break — the **fifth** consecutive occurrence (after `endpoint_policy`/`enable_h2`,
  `denied_hosts`/`no_proxy`, `require_auth`/`strict_connect_auth`, and Phase 113's `spiffe`, which
  broke **three** files where static inspection predicted one). Run `maturin build` in `../nono-py`
  and `napi build --platform --release` in `../nono-ts`, reporting real results — only building
  catches struct drift. **This forces `workflow.use_worktrees=false`** (the plan reaches a sibling
  repo via `..`).

### Recording and closure

- **D-10: File the scope limit as a carry-forward note in
  `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`**, updating the deferred
  SEC-02 row to **ADAPTED-with-scope-limit**. Name `149abde0` as a tracked divergence, mirroring
  how Phase 113 filed `b1ecbc02`. The ledger is the canonical work-list a future UPST planner
  actually reads. Rationale for not relying on a code comment alone: Phase 113 proved a fork's own
  "why is this missing" comment can be wrong and mislead for months.
- **D-11: SEC-02's `REQUIREMENTS.md` checkbox resolves as satisfied-with-scope-limit citing
  ADR-114** — not left open. Phase 112 deliberately left it unchecked pending this verdict; leaving
  it open again would misrepresent the milestone.

### Claude's Discretion

- Exact buffer cap value within D-05's stated range, and whether it is configurable.
- The concrete phantom-token minting/resolution mechanism, and how it composes with the fork's
  existing phantom machinery (which runs the *outbound* direction — proxy injects real credentials
  into outbound requests; capture is the *inbound* direction).
- What the audit record carries — subject to the hard constraint that it must **never** contain a
  raw token (cf. Phase 113's `SpiffeAuditContext`, which carries only IDs).
- Whether `nono-cli` needs a helper command to drive a login flow.
- Plan/wave decomposition, and whether ADR-114 lands as its own plan or a wave of a larger one.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### This phase's governing documents
- `.planning/ROADMAP.md` § "Phase 114: OAuth Capture Absorb (SEC-02)" — goal, SC1–SC4.
  **Note SC2's framing assumes a binary "hook in place, or decline"; D-01r/D-02r resolve it via a
  third path — construct an equivalent enforcement point.** SC3 is LIVE under ADAPT (D-09).
- `.planning/REQUIREMENTS.md` — SEC-02 (line ~131), traceability row (line ~175).
- `proj/ADR-114-oauth-capture-disposition.md` — **does not exist yet; this phase creates it.**
  Required by SC1. Shape precedent: `proj/ADR-111-resource-limits-boundary.md`.

### The evidence base this phase builds on
- `.planning/phases/112-security-residual-sync/112-OAUTH-CAPTURE-DISPOSITION.md` — the full
  reality-check: `9b692e07`'s 26-file inventory, file-presence table, `149abde0` as `forward.rs`'s
  sole creating commit, and the `ResponseRewrite` doc comment. **Read §1c before writing ADR-114.**
  Its security conclusion is not overturned — it is *resolved differently* by D-01r.
- `.planning/phases/112-security-residual-sync/112-REVIEW.md` — WR-13, the already-fixed instance
  of the defect shape D-06's cross-path guard prevents.

### Prior-phase precedent this phase mirrors
- `proj/ADR-113-spiffe-disposition.md` — **the closest structural precedent.** Its OD-1 section is
  the shape for naming a declined prerequisite as a tracked boundary; its D-01 section is the shape
  for a positive, grep-backed proof; its D-03 work is the shape for a request-time fail-closed
  guard across every arrival path.
- `.planning/phases/113-spiffe-spire-workload-identity/113-CONTEXT.md` — D-11 (verify by behavior,
  never identifier name), D-12 (both cross-target gates mandatory).
- `.planning/phases/113-spiffe-spire-workload-identity/113-06-SUMMARY.md` — how a from-scratch
  rewrite against `parse_upstream_url()` / `connect_upstream_tls()` was actually executed.
- `proj/ADR-112-allow-vars-fail-closed-preserved.md` — precedent for a formal decision recorded
  with zero/low code change.
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — D-10's note lands here;
  see its "SEC-09 Carry-Forward Note" and the Phase 113 SPIFFE note for shape.

### Boundary and standards
- `CLAUDE.md` — authoritative. Fail-secure, no `.unwrap()`/`.expect()`, `zeroize` for secrets,
  path *component* comparison, checked arithmetic, and the library-vs-CLI boundary table.
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library boundary SC3 tests against.
- `.planning/templates/cross-target-verify-checklist.md` — single source of truth for SC4's gates.

### Upstream source
- `9b692e07` (SEC-02a, 26 files / +4,425), `3c59c62e` (SEC-02b, hardening follow-up),
  `d033c631` (SEC-02c, test fixture). Reachable via the `upstream` remote — `git show <sha> -- <path>`
  works locally, no fetch needed.
- `149abde0` — creates `forward.rs`; **not absorbed** (brings TLS interception). Read its message
  to understand what the fork is declining.
- `docs/cli/features/sandboxed-oauth-logins.mdx` (+635 in `9b692e07`) — upstream's user-facing doc,
  useful for the declarative-provider config surface.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/nono-proxy/src/reverse.rs` — **the enforcement site.** Already terminates plain HTTP from
  the client and holds its own `tls_stream` to upstream, so it sees plaintext response bodies.
  Response relay is a `[u8; 8192]` read/write loop at **three** sites (static-credential handler
  plus the two SPIFFE handlers Phase 113 added). A rewrite must cover **all three** — that
  multiplicity is exactly where WR-13/D-03-class bugs come from.
- `extract_content_length()` / the 16 MiB request-body cap in `reverse.rs` — the existing idiom for
  bounded body handling; the response buffer should mirror its shape at a much tighter bound.
- `parse_response_status()` (`reverse.rs`) — already parses the status line from the first response
  chunk; a rewrite needs the same header/body split.
- `crates/nono-proxy/src/config.rs` — `RouteConfig`, `OAuth2Config`, and the existing phantom-token
  fields (`env_var`, the `{}` placeholder pattern) — the declarative surface to extend.
- `crates/nono-cli/src/profile/mod.rs` — `CustomCredentialDef` and `validate_custom_credential()`
  with its established mutual-exclusion pattern (`credential_key` / `auth` / `aws_auth` / `spiffe`);
  capture config becomes the next variant.
- `crates/nono-proxy/src/audit.rs` — `EventContext` is `#[derive(Default)]`, so a capture context
  field is safely additive (Phase 113 did exactly this for `spiffe_context`).

### Established Patterns
- **The fork streams responses by deliberate design** (module doc, `reverse.rs`): SSE, MCP
  Streamable HTTP, A2A JSON-RPC. Buffering must be strictly opt-in or these break.
- **The fork has NO response-body rewrite hook of any kind** — verified live, grep returns nothing.
  This is genuinely new machinery, not an extension.
- **The fork's phantom-token model runs the OPPOSITE direction** — proxy injects real credentials
  into *outbound* requests so the agent never sees them. Capture rewrites *inbound* responses. Same
  vocabulary, inverse data flow; do not assume the existing machinery is reusable.
- **The fork has no TLS interception, by standing decision** — `ProxyHandle::intercept_ca_path()`
  always returns `None`; 112-07 dropped `apply_tls_intercept_config`; Phase 113's D-01 re-affirmed
  it. Do not introduce one.
- **`nono-py` uses EXHAUSTIVE struct literals** (`../nono-py/src/proxy.rs`) — every new
  `RouteConfig` field breaks its build. `../nono-ts` is structurally immune (uses only
  `nono::query::*`).
- Upstream proxy absorbs routinely require adaptation, not `git apply` — `git apply --check` failed
  outright for both 112-07's and Phase 113's commits.

### Integration Points
- The three `reverse.rs` response-relay loops — where buffering+rewrite hooks in.
- `server.rs`'s `handle_forward_http` and the CONNECT dispatch — where D-06's cross-path guard goes,
  composing with (not duplicating) `ProxyConfig.require_auth` / `strict_connect_auth` and the
  existing `is_route_upstream` / `spiffe_declared_for_upstream` checks Phase 113 added.
- `crates/nono-cli/data/nono-profile.schema.json` — the strict fork-only companion (D-13).
- `../nono-py/src/proxy.rs` — the exhaustive-literal break site (D-14).

</code_context>

<specifics>
## Specific Ideas

- **The retracted decline is part of the record.** ADR-114 should state that a formal decline was
  considered and why it was rejected — the reasoning ("the only enforcement point requires MITM")
  rested on conflating response *visibility* with response *buffering*. A future reader who finds
  only the ADAPT will otherwise re-derive the same wrong premise from `112-OAUTH-CAPTURE-DISPOSITION.md`,
  which still reads as though `forward.rs` were the sole possible hook.
- **Verify by symbol, never by line number.** Phase 113 saw plans cite signatures a same-day sibling
  plan had already changed, and its disposition table was rated LOW-confidence on exactly the three
  files this phase touches most (`reverse.rs`, `route.rs`, `credential.rs`). Grep before editing.
- D-06's guard should be proven the way 112-07 and ADR-113's D-01 were: a literal grep-verified
  acceptance criterion, not prose assurance.
- Phase 113's D-07 loud-skip convention (`eprintln!("SKIP[{}]: reason", module_path!())` +
  `grep -c '^SKIP\['`) already exists in this repo. If any capture test ends up host-gated, reuse it
  rather than inventing a second convention.

</specifics>

<deferred>
## Deferred Ideas

- **Disk persistence of real OAuth tokens** (upstream `oauth_capture/persist.rs`, bounded phantom
  retention). Deliberately not taken — secret-at-rest is a separate decision from the
  enforcement-point decision and deserves its own security review (file permissions, path
  validation, multi-user hosts). Revisit only with that review.
- **TLS interception / `149abde0`.** Would extend capture to agent-initiated flows to arbitrary
  hosts via CONNECT. The fork's no-MITM stance is a standing decision and was not reopened here.
  This is the named condition under which SEC-02's scope limit could be lifted.
- **Switching `nono-py` to `..Default::default()`** to end the recurring build break. Raised in
  Phase 113 and rejected there: it trades a loud compile error for silent field adoption on a
  security-relevant struct — arguably the worse failure mode. Still its own scoped change.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` (score 0.6) — matched only on generic keywords ("clean", "uat",
  "host", "2026", "phase"). A v3.5 host-gated MSI/VC++ redistributable item, unrelated to OAuth
  capture. Not folded. (Phase 113 reviewed and declined the same item.)
- `20260611-poc-cert-broker-clean-host.md` (score 0.6) — same; a v3.5 clean-host POC-cert broker
  item. Not folded.

</deferred>

---

*Phase: 114-oauth-capture-absorb-sec-02*
*Context gathered: 2026-08-06*
