# Phase 56: Fine-grained Network Filtering - Context

**Gathered:** 2026-06-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Operators can scope `--allow-domain` entries to specific URL **paths** and **HTTP methods**, enforced in `nono-proxy` (`route`/`filter`/`server`), with TLS-intercept endpoint rules evaluated **before** credential selection, and `nono why --host` reflecting the new scoping. Implements **REQ-NET-01**.

This is the marquee absorption of upstream cluster **C3** (`0ced085` + `75b2265`, v0.59.0), plus a small-additive-port rider from the TLS-intercept ordering commit (`22e6c40`). The upstream **sync strategy is already locked by the Phase 54 audit** (see Canonical References) — this phase clarifies the *implementation* decisions the audit did not settle.

**Out of scope (own phases / deferred):** Bitwarden `bw://` (Phase 57), session hooks (Phase 58), supervisor IPC (Phase 59), a `nono why` request-level allow/deny tester (deferred idea below).

</domain>

<decisions>
## Implementation Decisions

### Upstream sync strategy (LOCKED by Phase 54 audit — do not re-litigate)
- **D-01:** Port **C3** = `0ced085` (feat(cli): fine-grained method+path restrictions in allow_domain, #960 — 12 files, the foundation incl. CLI-side `partition_allow_domain`) + `75b2265` (allow-domain accepts URL with path). Disposition `will-sync`. Carry D-19 `Upstream-commit:` trailers per the fork's cherry-pick discipline.
- **D-02:** The fork's `RouteStore` (`crates/nono-proxy/src/route.rs`) **already** has `endpoint_rules: CompiledEndpointRules` and already decouples endpoint enforcement from credential injection — the proxy-side target exists. What C3 introduces is the **CLI-side** `partition_allow_domain` + URL-path parsing; wire it into the existing `RouteStore`.
- **D-03:** TLS-intercept ordering (`22e6c40`) verdict = **fork-preserve**. The fork's already-decoupled `RouteStore`/`CredentialStore` satisfies "endpoint rules before credential selection". **Do NOT import upstream `tls_intercept/handle.rs`** (the fork has no `tls_intercept/` module). The ONLY portable artifact is the ~12-line `proxy_runtime.rs` filter-allowlist snippet (adds endpoint-restricted domains to the proxy filter allowlist so upstream connections succeed after interception) — apply it as a **small-additive-port rider** AFTER `partition_allow_domain` exists.
- **D-04:** `crates/nono-proxy/src/credential.rs` MUST stay **byte-identical** (Phase 09/11 Windows credential-injection rewrite, invariant SHA `c9f25164`). `22e6c40` does not touch it; nothing here may regress it.
- **D-05:** `rcgen` bump `8e78daf` = **won't-sync** (lives in the absent `tls_intercept/`). Do not pull it.

### CLI + profile syntax
- **D-06:** Adopt upstream #960 CLI/URL syntax **verbatim** (URL-with-path in `--allow-domain` + its method-restriction surface). Maximizes parity and keeps future UPST syncs clean. `NONO_ALLOW_DOMAIN` env var continues to work.
- **D-07:** Expose path/method scoping via **both** the CLI flag **and** an equivalent **profile JSON field** (so profiles like `claude-code` can ship scoped rules, consistent with how nono profiles already carry policy). **RESEARCH ITEM:** confirm from the `0ced085` diff whether upstream #960 already added a profile field. If upstream shipped CLI-only, the fork still extends the profile schema — consistent with upstream's CLI semantics, NOT a divergent surface. (Note: the profile deserializer was just reworked in Phase 55 — JSONC + the CR-01 dual-key fail-closed guard; any new structured `allow_domain` field must respect that fail-secure path.)

### Denial & path-match semantics (security core)
- **D-08:** **Path-prefix** matching, **component-wise** (NOT raw string `starts_with` — per CLAUDE.md path-security footgun #1). `/v1` matches `/v1`, `/v1/chat`, `/v1/models`.
- **D-09:** **Canonicalize + fail-secure** before matching: percent-decode, collapse `.`/`..`, normalize duplicate/trailing slashes; any residual traversal or decode ambiguity → **DENY** (no silent fall-through to host-level allow). Aligns with CLAUDE.md Path Handling + Fail Secure. **RESEARCH ITEM:** if upstream's normalization is more lenient than this, prefer the fork's stricter posture and document the divergence.
- **D-10:** A path/method mismatch (host allowed, endpoint not) returns **HTTP 403** (or proxy-equivalent hard denial) **AND** emits an audit/trace entry naming host + denied path/method, **BEFORE** any credential injection (satisfies SC2). Distinct from the C4 proxy-502 upstream-connect-failure path.

### `nono why --host` output
- **D-11:** When a host has scoped entries, `nono why --host <host>` **lists** each allowed path-prefix and its allowed method(s) (e.g. `/v1/* — GET, POST`). Satisfies SC3. Extend the existing `nono why --host` output structure minimally; don't redesign it.

### Default posture for partial scope
- **D-12:** Bare `--allow-domain api.openai.com` (no path/method) keeps today's meaning = **allow all paths + all methods** on that host (backward compatible). When a bare AND a scoped entry coexist for the same host, **most-permissive wins** — the bare entry re-opens the whole host (union semantics). **RESEARCH ITEM:** confirm `partition_allow_domain`/`0ced085` uses the same combine rule; if upstream narrows-per-host instead, surface the difference before locking.
- **D-13:** A scoped path entry with **no method** specified = **all methods** allowed on that matched path prefix (method restriction is opt-in).

### Claude's Discretion
- Exact 403 vs proxy-internal denial encoding is an impl detail as long as it is a hard denial (no silent pass-through) with a pre-credential audit entry (SC2).
- Method matching mechanics (case-normalization, multiple methods per entry) — follow upstream #960; default to case-insensitive method tokens and a set per entry unless the diff says otherwise.

### WATCH-ITEM (not a blocker)
- D-12 "most-permissive wins" means a stray bare `--allow-domain host` silently widens a host you otherwise scoped. The `nono why --host` output (D-11) and user docs should make a host's **effective** openness obvious so operators can spot an accidental bare entry.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Upstream sync strategy (the diff-inspect note this phase requires)
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` — **Cluster C3** (allow_domain path+method: `0ced085`, `75b2265`) disposition `will-sync`→Phase 56; **Cluster C5** function-call dep on C3; and the dedicated **`## TLS-intercept clean-apply assessment (Phase 34 C11)`** section — the load-bearing verdict (fork-preserve + the 12-line `proxy_runtime.rs` small-additive-port rider; `22e6c40` details; `credential.rs` byte-identical; `rcgen`/`8e78daf` won't-sync). READ THIS FIRST.
- `.planning/phases/54-upst7-audit/54-01-UPST7-AUDIT-PLAN.md` §SC4 — defines the TLS-intercept assessment contract and the `c9f25164` credential.rs invariant.

### Requirements / roadmap
- `.planning/REQUIREMENTS.md` — REQ-NET-01 (path scoping + method restrictions + endpoint-before-credential ordering + `nono why --host` awareness + diff-inspect-not-blind-cherry-pick).
- `.planning/ROADMAP.md` §"Phase 56: Fine-grained Network Filtering" — Goal + 4 Success Criteria (SC1 path/method enforcement, SC2 endpoint-before-credential, SC3 `nono why` surfacing, SC4 fork-preserve + documented dispositions).

### Fork code surface (diff-inspect targets — do not blind cherry-pick)
- `crates/nono-proxy/src/route.rs` — `RouteStore` + `endpoint_rules: CompiledEndpointRules` (the proxy-side enforcement target; already credential-independent).
- `crates/nono-proxy/src/credential.rs` — Phase 09/11 rewrite, invariant `c9f25164`; MUST stay byte-identical.
- `crates/nono-proxy/src/filter.rs` + `crates/nono/src/net_filter.rs` — `ProxyFilter`/`HostFilter` host-level allow-list (`check_host`); the proxy_runtime filter-allowlist rider (D-03) feeds here.
- `crates/nono-cli/src/cli.rs` — current `--allow-domain` flag (`long="allow-domain"`, `env="NONO_ALLOW_DOMAIN"`) + `nono why` surface.

### Upstream commits to port (fetch from upstream remote)
- `0ced085` (#960, v0.59.0) — fine-grained method+path; introduces `partition_allow_domain`.
- `75b2265` (v0.59.0) — allow-domain accepts URL with path.
- `22e6c40` (v0.59.0) — endpoint-before-credential ordering; take ONLY the ~12-line `proxy_runtime.rs` filter-allowlist snippet (per D-03).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `RouteStore.endpoint_rules: CompiledEndpointRules` (`route.rs`) — the proxy already supports endpoint restrictions decoupled from credentials; this phase wires the new path/method rules into it rather than building new enforcement.
- `HostFilter`/`ProxyFilter` (`net_filter.rs`/`filter.rs`) — host-level allow-list with DNS-rebinding-safe `check_host(host, resolved_ips)`; the `proxy_runtime.rs` rider adds endpoint-restricted domains to its allowlist.
- `nono why --host` (CLI) — existing diagnostic to extend (D-11).

### Established Patterns
- D-19 `Upstream-commit:` trailer discipline for every ported commit (see Phase 55).
- CLAUDE.md path-security: component-wise `Path::starts_with`, canonicalize at the enforcement boundary, fail-secure on ambiguity — directly governs D-08/D-09.
- Profile deserializer is JSONC + fail-closed dual-key guard (Phase 55 CR-01) — a new structured `allow_domain` profile field (D-07) must route through that fail-secure path.

### Integration Points
- CLI parse (`cli.rs`) → `partition_allow_domain` (new) → `RouteStore` endpoint rules + `ProxyFilter` allowlist.
- Proxy reverse handler (`server.rs`/`route.rs`) → endpoint-rule check BEFORE `CredentialStore` selection/injection (D-03, SC2).

</code_context>

<specifics>
## Specific Ideas

- SC1 example to honor: `--allow-domain https://api.example.com/v1 --method GET` (or equivalent profile field) restricts to that path prefix + method; disallowed path/method → proxy denial, not silent pass-through.
- SC3 example: `nono why --host api.example.com` surfaces the path/method scoping rules.

</specifics>

<deferred>
## Deferred Ideas

- **`nono why` request-level tester** (`--host X --path /v1 --method POST` → allow/deny verdict) — a new diagnostic capability beyond SC3; note for a future phase, do not build in Phase 56.

None else — discussion stayed within phase scope.

</deferred>

---

*Phase: 56-fine-grained-network-filtering*
*Context gathered: 2026-06-05*
