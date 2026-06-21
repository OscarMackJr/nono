# Phase 89: Proxy Hardening Sync - Context

**Gathered:** 2026-06-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb upstream's 7-commit proxy-hardening cluster (Cluster F, `v0.62.0..v0.64.0`)
into the fork, reconciled against the fork's **structurally divergent** proxy surface
without regressing fork TLS-intercept behavior. The fork's proxy differs from upstream
in three load-bearing ways:

1. **No `tls_intercept/` module** — Phase 34 C11 `fork-preserve`. Upstream's hardening
   touches `tls_intercept/handle.rs` + `tls_intercept/mod.rs`; those hunks are won't-apply.
   The fork enforces a **CONNECT-block model** instead (server.rs returns 403 to route
   upstreams, forcing L7 reverse-proxy filtering — a raw TLS pipe would bypass injection).
2. **`external_proxy` (not upstream's `upstream_proxy`)** — the fork already maps the CLI
   `--upstream-proxy` flag to `ProxyConfig.external_proxy` and honors it in the CONNECT path.
3. **Prefix-keyed `RouteStore`** — not upstream's `RouteSelection`/`select_route` abstraction.

**Cluster F commits (per Phase 85 ledger, `split` disposition):**
- `b0b2c743` (#1132) — allow_domain route shadowing credential catch-all; adds RouteSelection/select_route
- `a5d623fd` (#1077) — 403 + audit for denied non-CONNECT requests
- `b5f8db5c` (#1048/#1091) — respect upstream_proxy in TLS CONNECT intercept path
- `7c9abd3b` (#1151) — keep connection open for reactive proxy auth on CONNECT
- `76b7b695` (#1192) — refactor `forward_inner_request` (entirely inside tls_intercept/ — won't-apply)
- `bd4b6b7f` (#1199) — separate proxy intent from activation (5 CLI files; adds TlsInterceptIntent)
- `724bb207` (#1197) — proxy should activate when customCredentials is set

**In scope:** PROXY-01 (#1132, #1077) and PROXY-02 (#1048/#1091, #1151, #1197) — the
*behavioral intents* of the cluster, reconciled against the fork's actual proxy code. The
`split` disposition's reconciliation findings (equivalence notes + won't-sync records) are
recorded as a Phase-85 ledger addendum.

**Out of scope:** the `tls_intercept/` module itself (fork-preserve); real AWS SigV4 signing
(stays a 501 stub — see D-08); bd4b6b7f's structural intent refactor (deferred, D-04); the
v3.0 host-gated UAT drain (Phase 90).

**Disposition source:** Cluster F's `split` disposition and per-commit sync-safe/won't-apply
findings are LOCKED in the Phase 85 ledger (Cluster F section, lines 306-374). This phase
executes the reconciliation those findings call for — it does not re-litigate the disposition.

</domain>

<decisions>
## Implementation Decisions

### CONNECT-path fixes — #1048/#1091, #1151 (PROXY-02)
- **D-01: Test-driven gap proof is the bar for every CONNECT-path commit.** Default to
  no-op. For each fix, require (a) a written behavioral-equivalence note AND (b) a fork test
  demonstrating the fork's path already delivers the fix's intent. Only port the behavioral
  intent into the fork's CONNECT path (`server.rs` / `connect.rs`) if a test exposes a real
  gap. Rationale: the fork already honors `external_proxy` in the CONNECT path
  (`server.rs:534-559`) and runs lenient undici-compatible auth in `connect.rs` — upstream's
  fixes target tls_intercept behavior the fork achieves differently. Don't add fork code that
  duplicates existing behavior; prove the gap first.
- **D-02: #1151 (reactive-auth keep-open) — port only if the test fails.** Write a
  keep-connection-open behavioral test. The fork's `connect.rs` lenient auth (accepts missing
  `Proxy-Authorization`) may make the 407-retry handshake moot. If the test proves the fork
  *drops* the connection where upstream keeps it open, implement keep-open in `connect.rs`;
  otherwise record equivalence and skip. Same bar as D-01 — no special-casing.

### Intent refactor — bd4b6b7f / #1199
- **D-03 (recorded as D-04 below):** see D-04. *(numbering note: intent refactor decision is D-04.)*
- **D-04: Defer bd4b6b7f's structural refactor; keep the behavior.** The
  intent/activation-separation refactor (303+/80-, 5 CLI files) is organizational, not
  behavioral, conflicts with the fork's `EffectiveProxySettings` model
  (`proxy_runtime.rs` — `resolve_effective_proxy_settings` / `build_proxy_config_from_flags`),
  and carries a `TlsInterceptIntent` struct the fork cannot back. Implement #1197's behavioral
  fix directly against the fork's existing `proxy_runtime.rs` path (see D-07). Record
  `bd4b6b7f` as **won't-sync (architecture divergence)** in the ledger addendum. Do NOT
  introduce a stub `TlsInterceptIntent` (no-dead-code standard).

### customCredentials activation + AWS stub — #1197 (PROXY-02)
- **D-05: `76b7b695` is fully won't-sync.** The `forward_inner_request` refactor lives
  entirely inside the absent `tls_intercept/` module. Record as won't-apply in the ledger
  addendum; no fork change.
- **D-06 (recorded as D-07 below):** see D-07. *(numbering note: activation-gate fix is D-07.)*
- **D-07: Land the customCredentials activation-gate fix.** The #1197 bug is precise: the
  activation gate in `proxy_runtime.rs:90-116` checks `credentials`, `network_profile`,
  `allow_domain`, and `upstream_proxy` — but NOT `custom_credentials`. So the proxy does not
  start when only `customCredentials` is set. Fix: add `!prepared.custom_credentials.is_empty()`
  (and the `--block-net` override branch's mirror) to the activation predicate. The fork
  already plumbs `custom_credentials` through `resolve_credentials` (`proxy_runtime.rs:202`) —
  this is the only missing link. Add a regression test asserting activation with
  customCredentials-only config.
- **D-08: AWS auth stays a 501 stub — activation-only this phase.** #1197 is about *activation*,
  not *signing*. Real AWS SigV4 request signing is a separate feature beyond Cluster F; keep the
  honest 501 at `credential.rs:219` and `reverse.rs:189` (deliberately deferred in Phase 88,
  D-15). Defer real SigV4 to a dedicated future phase. (Note: the 501 stays a bare status line
  this phase — the diagnostic-upgrade option was considered and not taken.)

### 403 + audit and allow_domain — #1077, #1132 (PROXY-01)
- **D-09: #1077 (403 + audit on denied non-CONNECT) — verify-present.** The fork already
  returns `403 Forbidden` + `audit::log_denied` on endpoint-rule default-deny
  (`reverse.rs:96-114`). Write an equivalence note + confirming test; if present, record
  equivalence and skip the cherry-pick. Same test-driven bar as D-01.
- **D-10: #1132 (allow_domain shadowing) — verify the shadow on the fork's model, fix
  directly if it reproduces.** Upstream adds a `RouteSelection`/`select_route` abstraction; the
  fork uses prefix-keyed `RouteStore` lookup (`route.rs`). Write a test reproducing the
  allow_domain-endpoint-route-shadows-credential-catch-all scenario against the fork's
  `RouteStore`. If the fork's prefix-keyed lookup already avoids the shadow → record
  equivalence + skip the RouteSelection refactor. If it reproduces → fix the fork's route
  selection **directly** (do not import upstream's `RouteSelection` enum). Don't restructure
  working route code to match an abstraction the fork doesn't use.

### Recording mechanism (cross-cutting)
- **D-11: All equivalence-notes and won't-sync findings land as a Phase-85 ledger addendum**
  (mirroring the Phase 87 CR-02 / Phase 88 CR-01 addendum pattern), so future syncs expect the
  Cluster F divergences and don't re-attempt blind cherry-picks. Any fix that *does* land on
  fork-divergent lines is recorded as a deliberate fork-divergence with its regression test.

### Claude's Discretion
- Exact wording/structure of each behavioral-equivalence test (researcher/planner territory).
- Whether the `--block-net` override branch (`proxy_runtime.rs:91-95`) needs the same
  `custom_credentials` predicate addition as the active branch (D-07) — likely yes for
  consistency, but the planner confirms against the warn-and-ignore semantics.
- ProxyDiagnostic surface usage for any newly-added denial (Phase 86 surface) — only if a fix
  actually adds a denial path; not mandated.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 89 scope & dispositions (PRIMARY — read first)
- `.planning/phases/85-upst9-divergence-audit/85-DIVERGENCE-LEDGER.md` §"Cluster F: Proxy
  Hardening" (lines 306-374) — `split` disposition, per-commit sync-safe / won't-apply
  findings, the cross-cluster re-export check, and the per-commit SHA table. THE source of
  truth for what syncs vs. what's won't-apply. Also Check 4 (route.rs, ~643-650) and the
  Cluster F empirical cross-check (~694).
- `.planning/REQUIREMENTS.md` — PROXY-01 (line 52), PROXY-02 (line 53).
- `.planning/ROADMAP.md` §"Phase 89: Proxy Hardening Sync" (lines 111-119) — goal + 3 success
  criteria.

### Prior-phase context (dependency surfaces)
- `.planning/phases/88-feature-dependency-cherry-pick-wave/88-CONTEXT.md` D-15 + `<deferred>` —
  AWS-auth 501-stub rationale; customCredentials/AWS shared credential path; the
  `TlsInterceptIntent` assessment deferral to Phase 89.
- `.planning/phases/86-library-boundary-convergence/86-CONTEXT.md` — ProxyDiagnostic surface
  (Phase 86) available for newly-added denials, if any.

### Fork code touchpoints (this is where reconciliation happens)
- `crates/nono-proxy/src/server.rs:445-615` — `handle_connection`: CONNECT-block model,
  403+audit for blocked CONNECT-to-route-upstream, `external_proxy` honoring (D-01, D-02).
- `crates/nono-proxy/src/connect.rs` — `handle_connect` + lenient `validate_proxy_auth`
  (undici compat); the reactive-auth equivalence surface (D-02).
- `crates/nono-proxy/src/reverse.rs:85-189` — non-CONNECT reverse proxy: endpoint-rule
  default-deny 403+audit (D-09); AWS 501 stub at line 189 (D-08).
- `crates/nono-proxy/src/route.rs` — prefix-keyed `RouteStore`, `is_route_upstream`,
  `endpoint_rules` (D-10 shadowing test target).
- `crates/nono-proxy/src/credential.rs:219` — AWS 501 stub (D-08); `resolve_credentials`
  custom-credentials path.
- `crates/nono-cli/src/proxy_runtime.rs:90-116` — the activation gate; missing
  `custom_credentials` predicate is the #1197 bug (D-07). `build_proxy_config_from_flags`
  (175-234) maps `upstream_proxy`→`external_proxy` + threads `custom_credentials`.
- `crates/nono-cli/src/sandbox_prepare.rs` — `custom_credentials` plumbing into
  PreparedSandbox (D-07 source).

### Process rules
- `CLAUDE.md` §"Coding Standards" — cross-target clippy MUST/NEVER. Cluster F is **no
  Windows-touch** (ledger line 313) and the proxy is not cfg-gated Unix code, but verify any
  edit per the checklist; PARTIAL→CI for anything unverifiable on the Windows dev-host.
- `.planning/templates/cross-target-verify-checklist.md` — PARTIAL→CI deferral form.
- Memory `feedback_cluster_isolation_invalid` — diff-inspect re-export surfaces (already done
  for Cluster F in the ledger; honor those findings).

### Upstream commits (Cluster F)
- `b0b2c743` (#1132) · `a5d623fd` (#1077) · `b5f8db5c` (#1048/#1091) · `7c9abd3b` (#1151) ·
  `76b7b695` (#1192, won't-sync D-05) · `bd4b6b7f` (#1199, won't-sync D-04) · `724bb207` (#1197).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Fork already honors `external_proxy` in the CONNECT path** (`server.rs:534-559`) and maps
  `--upstream-proxy`→`ProxyConfig.external_proxy` (`proxy_runtime.rs:222-228`) — #1048/#1091
  may be a documented no-op (D-01).
- **Fork already returns 403+audit on denied non-CONNECT** (`reverse.rs:96-114`, endpoint-rule
  default-deny) — #1077 likely already-present (D-09).
- **`custom_credentials` is already plumbed** through `sandbox_prepare.rs` →
  `proxy_runtime.rs` → `resolve_credentials` — only the activation *gate* misses it (D-07).
- **`audit::log_denied` + `NetworkAuditDenialCategory`** already exist for both CONNECT
  (`ConnectBypassesL7`) and Reverse modes — reuse for any new denial.

### Established Patterns
- **Test-driven equivalence proof** is THIS phase's defining method (D-01) — divergent surface
  means cherry-picks are reconciled by behavioral test, not blind `git cherry-pick -x`.
- **Ledger-addendum recording of won't-sync / fork-divergence** (Phase 87 CR-02, Phase 88
  CR-01) — D-04, D-05, D-11 follow it.
- **PARTIAL→CI deferral** for anything unverifiable on the Windows dev-host (proxy tests are
  cross-platform Rust; most should run locally).

### Integration Points
- Activation gate (`proxy_runtime.rs:90-116`) → proxy start decision → `customCredentials`
  end-to-end (D-07).
- CONNECT-block model (`server.rs`) ↔ reverse-proxy L7 filtering (`reverse.rs`) — the fork's
  structural replacement for tls_intercept; all CONNECT-path reconciliation respects it.
- AWS 501 stub (`credential.rs:219`, `reverse.rs:189`) stays a hard boundary (D-08).

</code_context>

<specifics>
## Specific Ideas

- The fork's proxy is NOT a thin variant of upstream's — it's a CONNECT-block + reverse-proxy
  L7 model that deliberately omits tls_intercept. Every Cluster F decision is "prove the fork
  already does this differently, else port the *intent* (not the upstream code shape)."
- #1197 is the one clearly-real, clearly-small behavioral fix in the cluster: a single missing
  predicate in the activation gate. Everything else is verify-equivalence-or-port-intent.
- Do not import upstream abstractions (`RouteSelection`, `TlsInterceptIntent`,
  intent/activation separation) into the fork — they don't map and add divergence cost with no
  behavioral payoff this milestone.

</specifics>

<deferred>
## Deferred Ideas

- **Real AWS SigV4 request signing** (replace the 501 stub) — its own future phase; out of
  Cluster F scope (D-08).
- **bd4b6b7f intent/activation refactor + `TlsInterceptIntent`** — won't-sync this phase
  (D-04); only revisit if the fork ever revives a `tls_intercept/` module.
- **Diagnostic-quality upgrade of the AWS 501** (ProxyDiagnostic + audit instead of a bare
  status line) — considered for D-08, not taken; a candidate polish item if AWS work resumes.

None of the above are scope creep raised in discussion — they are ledger-dispositioned or
explicitly-considered-and-deferred items recorded so they are not lost.

</deferred>

---

*Phase: 89-proxy-hardening-sync*
*Context gathered: 2026-06-20*
