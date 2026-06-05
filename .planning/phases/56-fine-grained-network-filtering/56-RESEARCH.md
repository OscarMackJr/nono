# Phase 56: Fine-grained Network Filtering - Research

**Researched:** 2026-06-04
**Domain:** nono-proxy path/method enforcement, CLI allow_domain URL parsing, profile schema extension
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Port C3 = `0ced085` (feat(cli): fine-grained method+path restrictions in allow_domain, #960 — 12 files, the foundation incl. CLI-side `partition_allow_domain`) + `75b2265` (allow-domain accepts URL with path). Carry D-19 `Upstream-commit:` trailers.
- **D-02:** The fork's `RouteStore` already has `endpoint_rules: CompiledEndpointRules` and already decouples endpoint enforcement from credential injection. What C3 introduces is the CLI-side `partition_allow_domain` + URL-path parsing; wire it into the existing `RouteStore`.
- **D-03:** TLS-intercept ordering (`22e6c40`) verdict = **fork-preserve**. The fork's already-decoupled `RouteStore`/`CredentialStore` satisfies "endpoint rules before credential selection". Do NOT import upstream `tls_intercept/handle.rs`. The ONLY portable artifact is the ~12-line `proxy_runtime.rs` filter-allowlist snippet — apply it as a **small-additive-port rider** AFTER `partition_allow_domain` exists.
- **D-04:** `crates/nono-proxy/src/credential.rs` MUST stay **byte-identical** (Phase 09/11 Windows credential-injection rewrite, invariant SHA `c9f25164`). `22e6c40` does not touch it.
- **D-05:** `rcgen` bump `8e78daf` = **won't-sync** (lives in the absent `tls_intercept/`).
- **D-06:** Adopt upstream #960 CLI/URL syntax **verbatim** (URL-with-path in `--allow-domain` + its method-restriction surface). Maximizes parity and keeps future UPST syncs clean. `NONO_ALLOW_DOMAIN` env var continues to work.
- **D-07:** Expose path/method scoping via **both** the CLI flag **and** an equivalent **profile JSON field**. (RESEARCH ITEM: see Resolved Research Items section — upstream DID add a profile field.)
- **D-08:** **Path-prefix** matching, **component-wise** (NOT raw string `starts_with`). `/v1` matches `/v1`, `/v1/chat`, `/v1/models`.
- **D-09:** **Canonicalize + fail-secure** before matching. (RESEARCH ITEM: see Resolved Research Items — upstream uses the same normalization; fork must adopt verbatim.)
- **D-10:** A path/method mismatch returns **HTTP 403** with audit/trace entry naming host + denied path/method, **BEFORE** any credential injection (satisfies SC2).
- **D-11:** When a host has scoped entries, `nono why --host <host>` **lists** each allowed path-prefix and its allowed method(s). Extend the existing `nono why --host` output structure minimally.
- **D-12:** Bare `--allow-domain api.openai.com` keeps today's meaning = **allow all paths + all methods**. Bare AND scoped entry for the same host = **most-permissive wins** (union semantics, bare entry re-opens the whole host). (RESEARCH ITEM: CONFIRMED — see Resolved Research Items.)
- **D-13:** A scoped path entry with **no method** specified = **all methods** allowed on that matched path prefix (method restriction is opt-in).

### Claude's Discretion
- Exact 403 vs proxy-internal denial encoding is an impl detail as long as it is a hard denial with a pre-credential audit entry (SC2).
- Method matching mechanics (case-normalization, multiple methods per entry) — follow upstream #960.

### Deferred Ideas (OUT OF SCOPE)
- `nono why` request-level tester (`--host X --path /v1 --method POST` → allow/deny verdict).
- Bitwarden `bw://` (Phase 57), session hooks (Phase 58), supervisor IPC (Phase 59).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REQ-NET-01 | `--allow-domain` accepts URL with path scoping and fine-grained HTTP method+path restrictions, enforced in `nono-proxy`; TLS-intercept endpoint rules evaluated before credential selection; `nono why --host` reflects new scoping; diff-inspected against fork-divergent surface (not blind cherry-pick). | Fully covered — upstream diff-inspected, fork wiring points identified, all 4 SCs mapped. |
</phase_requirements>

---

## Summary

Phase 56 absorbs upstream cluster C3 (`0ced085` + `75b2265`, v0.59.0) and the small-additive-port rider from `22e6c40`, giving nono operators the ability to scope `--allow-domain` entries to specific URL path patterns and HTTP methods. The upstream feature is CLI+profile-dual (D-07 confirmed by inspection), uses `#[serde(untagged)]` `AllowDomainEntry` enum for backward-compatible JSON, and `partition_allow_domain` to split entries into plain-tunnel hosts and endpoint-restricted routes that feed the fork's existing `RouteStore`.

The fork's architecture is already structurally ready: `RouteStore.endpoint_rules: CompiledEndpointRules` is loaded from `RouteConfig.endpoint_rules` and checked in `reverse.rs` BEFORE `CredentialStore.get()` is called (lines 96–116 vs 119), satisfying SC2 without any proxy-layer changes. The ordering commit `22e6c40` only adds a `proxy_runtime.rs` snippet that pushes endpoint-restricted domain hostnames into `plain_hosts` so they appear on the proxy filter allowlist — that snippet must ride with Phase 56 after `partition_allow_domain` exists.

The key work is: (1) introduce `AllowDomainEntry` enum + `merge_allow_domain` in `profile/mod.rs`; (2) add `partition_allow_domain` in `network_policy.rs`; (3) wire it into `proxy_runtime.rs` (replacing `expand_proxy_allow` + adding the filter-allowlist rider); (4) add `parse_allow_domain_arg` for CLI URL parsing; (5) extend `sandbox_state.rs` with `DomainEndpointState`/`EndpointRuleState`; (6) extend `query_ext.rs`/`why_runtime.rs` for SC3; (7) update profile schema JSON. The profile field change from `Vec<String>` to `Vec<AllowDomainEntry>` is backward-compatible via `#[serde(untagged)]` — plain strings still deserialize as `Plain(String)`.

**Primary recommendation:** Diff-inspect + selectively replay the 12 most critical files from `0ced085` and 3 files from `75b2265`, then apply the 12-line `proxy_runtime.rs` rider from `22e6c40`. Do NOT blind-cherry-pick: upstream's `RouteConfig` has additional fields (`proxy`, `tls_client_cert`, `tls_client_key`) that the fork lacks, so `partition_allow_domain`'s `RouteConfig` construction must use only fork-present fields.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| URL path parsing from `--allow-domain` CLI arg | CLI (nono-cli) | — | `parse_allow_domain_arg` transforms CLI string to `AllowDomainEntry` before proxy starts |
| Profile schema: `allow_domain` structured entries | CLI (nono-cli) | — | Profile deserialization lives in `nono-cli/src/profile/mod.rs`; schema JSON in `data/` |
| Partitioning entries into plain-hosts vs endpoint routes | CLI (nono-cli) | — | `network_policy::partition_allow_domain` runs at proxy startup in `proxy_runtime.rs` |
| L7 endpoint rule enforcement (method+path deny) | Proxy (nono-proxy) | — | `route.rs` → `reverse.rs` already checks `endpoint_rules.is_allowed()` before credential lookup |
| Filter allowlist population for endpoint-restricted domains | CLI (nono-cli) | Proxy (nono-proxy) | The CLI computes `plain_hosts` and passes to `ProxyConfig.allowed_hosts`; proxy enforces |
| `nono why --host` endpoint-rule display | CLI (nono-cli) | — | `query_ext.rs` + `why_runtime.rs` + `sandbox_state.rs` |
| Credential injection ordering (endpoint-before-credential) | Proxy (nono-proxy) | — | Already enforced structurally — `endpoint_rules` check at line 96, credential at line 119 of `reverse.rs` |

---

## Standard Stack

### Core (fork-present, no new deps needed for base feature)

| Library | Current Version | Purpose | Why Standard |
|---------|----------------|---------|--------------|
| `globset` | Already in `nono-proxy/Cargo.toml` | Glob path matching in `CompiledEndpointRules` | Already used — `Glob::new` + `GlobMatcher` |
| `urlencoding` | Already in `nono-proxy/Cargo.toml` | Percent-decode in `normalize_path` | Already used for path normalization |
| `url` | Workspace dep, already in `nono-cli` | Parse `https://host/path` from `--allow-domain` arg | `url::Url::parse` in `parse_allow_domain_arg` |

[VERIFIED: codebase inspection] All three are already dependencies. No new packages required for the base feature.

### New Dependencies Needed for `nono-cli` (from `75b2265`)

Upstream `75b2265` added `globset` and `urlencoding` to `nono-cli/Cargo.toml` — the CLI needs these because `parse_allow_domain_arg` needs URL parsing (already has `url`) and `path_matches_endpoint_rules` in `query_ext.rs` uses `globset`. [VERIFIED: git show 75b2265 -- crates/nono-cli/Cargo.toml]

```toml
# crates/nono-cli/Cargo.toml — additions from 75b2265
globset = "0.4"
urlencoding = "2"
```

[VERIFIED: crates/nono-proxy/src/config.rs] `nono-proxy` already has `globset` and `urlencoding` — only `nono-cli` adds them.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `#[serde(untagged)]` on `AllowDomainEntry` | A custom deserializer | `untagged` is simpler and backward-compatible; plain strings still parse; upstream uses it |
| Component-wise glob matching via `globset` | Regex matching | `globset` already in proxy; `*` / `**` semantics map well to API path patterns |

---

## Package Legitimacy Audit

No new external packages are introduced. `globset` and `urlencoding` are moved from `nono-proxy`'s existing dependency tree into `nono-cli` — they are already present and verified in the workspace.

| Package | Registry | Age | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|
| `globset` | crates.io | Multi-year (part of ripgrep) | N/A — existing workspace dep | Approved (already in nono-proxy) |
| `urlencoding` | crates.io | Several years | N/A — existing workspace dep | Approved (already in nono-proxy) |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Resolved Research Items

### D-07: Does upstream `0ced085` add a profile JSON field? [VERIFIED: git show 0ced085]

**Yes — upstream ships BOTH CLI flag AND a profile schema field.** `0ced085` modifies `crates/nono-cli/data/nono-profile.schema.json` to extend `allow_domain` items from `{ "type": "string" }` to `{ "oneOf": [ { "type": "string" }, { "$ref": "#/$defs/AllowDomainWithEndpoints" } ] }`. The new `AllowDomainWithEndpoints` schema requires `{ "domain": "string", "endpoints": [{ "method", "path" }] }`.

In `profile/mod.rs`, upstream introduces `AllowDomainEntry` as `#[serde(untagged)]` enum with two variants: `Plain(String)` and `WithEndpoints { domain: String, endpoints: Vec<EndpointRule> }`. The `NetworkConfig.allow_domain` field type changes from `Vec<String>` to `Vec<AllowDomainEntry>`. This is **backward-compatible**: existing profiles with plain strings still parse as `Plain(String)` via `untagged` serde.

**Fork action:** The fork's current `profile/mod.rs` has `pub allow_domain: Vec<String>` — this MUST change to `Vec<AllowDomainEntry>`. This is the load-bearing type change. The Phase 55 CR-01 dual-key guard applies to `bypass_protection`/`override_deny`, not `allow_domain` — no dual-key conflict here. The `allow_domain` field uses standard `#[serde(rename = "allow_domain", alias = "proxy_allow", alias = "allow_proxy")]` plus `#[serde(untagged)]` on the new enum; this does not conflict with the Phase 55 JSONC deserialization change.

**Upstream schema verbatim (for profile JSON field):**
```json
{
  "allow_domain": [
    "api.openai.com",
    {
      "domain": "api.github.com",
      "endpoints": [
        { "method": "GET", "path": "/repos/my-org/**" },
        { "method": "POST", "path": "/repos/my-org/*/issues" }
      ]
    }
  ]
}
```

### D-09: What path normalization does upstream use? [VERIFIED: codebase inspection + git show 0ced085]

**Fork's `normalize_path` in `config.rs` already implements the correct normalization:** percent-decode (via `urlencoding::decode_binary`), strip query string, collapse double slashes by splitting on `/` and filtering empty segments, strip trailing slash (but preserve root `/`). This matches the upstream behavior for `CompiledEndpointRules::is_allowed()`. The fork's normalization is ALREADY in place — it was written when the fork first added `endpoint_rules` to `RouteStore`.

**No upstream divergence to document:** The upstream C3 diff does not introduce new normalization logic in `config.rs` — it uses the same `normalize_path` function already present. The fork's fail-secure posture (any traversal/decode ambiguity → DENY) is satisfied by the existing `normalize_path` + the glob matcher (which does not allow `..` traversal by default).

**One difference to watch:** The CLI URL parser (`parse_allow_domain_arg`) in `75b2265` uses `url::Url::parse(input).path()` to extract the path from a URL like `https://github.com/org/**`. The `url` crate normalizes percent-encoding and removes `..` segments at URL parse time, so the path arriving in `EndpointRule.path` is already normalized before `normalize_path` runs at match time. This is double-safe — adopt verbatim.

### D-12: How does `partition_allow_domain` handle bare + scoped same-host coexistence? [VERIFIED: git show 0ced085]

**Upstream uses ENDPOINT-UNION semantics (all endpoint rules merged across entries), but bare + scoped do NOT produce most-permissive-wins.** The actual behavior is:

In `merge_allow_domain` (profile merging): a `Plain("github.com")` entry and a `WithEndpoints { domain: "github.com", ... }` entry get merged into `WithEndpoints { domain: "github.com", endpoints: [merged rules] }`. So a bare entry plus a scoped entry for the same host results in a `WithEndpoints` with all the scoped entry's rules — NOT a bare (all-methods-all-paths) entry.

In `partition_allow_domain` (runtime): `Plain(host)` → goes to `plain_hosts` (CONNECT tunnel, no L7 filter). `WithEndpoints { domain, endpoints }` where `endpoints.is_empty()` → treated as plain (also goes to `plain_hosts`). `WithEndpoints { domain, endpoints }` where `!endpoints.is_empty()` → creates an endpoint route.

**The critical point:** If you have BOTH a `Plain("github.com")` AND a `WithEndpoints { domain: "github.com", endpoints: [...] }` in the same runtime array (e.g. one from the profile and one from `--allow-domain`), `partition_allow_domain` processes them separately — both enter `plain_hosts` AND an endpoint route is created for `github.com`. Result: `github.com` appears in the proxy filter allowlist (via `plain_hosts`) AND has an endpoint-restricted route. In this case the plain CONNECT tunnel path also allows the host — so bare+scoped coexistence effectively gives the host unrestricted CONNECT access while also offering L7-filtered reverse-proxy access.

**Implication for D-12 "most-permissive wins":** The CONTEXT.md D-12 says bare+scoped = most-permissive-wins (union semantics). The upstream implementation achieves this at the `merge_allow_domain` level for profile-profile merging by merging endpoints — if a plain is merged with a scoped, the result is scoped (not plain). But at the `partition_allow_domain` runtime level, if two separate entries exist (one Plain, one WithEndpoints), the plain entry still adds to `plain_hosts`. The net effect is that a bare `--allow-domain github.com` alongside `--allow-domain https://github.com/org/**` results in `github.com` being in both `plain_hosts` (unrestricted CONNECT) AND an endpoint route — the CONNECT path wins for unrestricted access.

**Fork adoption:** Adopt upstream verbatim. The D-12 "most-permissive wins" behavior is what upstream implements — a bare entry for a host effectively opens it for all CONNECT traffic regardless of scoped routes.

### Method Matching Mechanics [VERIFIED: git show 0ced085, codebase inspection]

**Case-insensitive, set-per-entry, `"*"` wildcard:** From `CompiledEndpointRules::is_allowed()` in `config.rs`:

```rust
(r.method == "*" || r.method.eq_ignore_ascii_case(method))
    && r.matcher.is_match(&normalized)
```

Methods are case-insensitive via `eq_ignore_ascii_case`. Each rule has exactly one method (or `"*"` for any). Multiple methods per endpoint are expressed as multiple rules. The `parse_allow_domain_arg` function uses `method: "*".to_string()` when parsing a URL with path (all methods on the path). The profile `AllowDomainEntry::WithEndpoints.endpoints` is `Vec<EndpointRule>` — users specify multiple `{ "method": "GET", "path": "..." }` objects.

**Note for D-13 (no method = all methods):** The upstream CLI path uses `method: "*"` when parsing a URL arg. For a profile field with no method restriction, the operator simply does not include endpoint rules (the entry stays `Plain`). There is no "omit method field for any-method" in the schema — instead, use `"method": "*"` explicitly, or omit the `endpoints` array entirely.

---

## Architecture Patterns

### System Architecture Diagram

```
CLI parse
  --allow-domain "api.openai.com"         -->  Plain("api.openai.com")
  --allow-domain "https://gh.com/org/**"  -->  WithEndpoints{domain:"gh.com", endpoints:[{*,/org/**}]}
  profile.network.allow_domain: [...]     -->  Vec<AllowDomainEntry> (deserialized)

     |                        |
     v                        v
merge_allow_domain() [profile-level, merge base+child entries per domain]
     |
     v
resolve_effective_proxy_settings() [proxy_runtime.rs]
  --> EffectiveProxySettings.allow_domain: Vec<AllowDomainEntry>
     |
     v
partition_allow_domain(net_policy, &allow_domain)
  +--> plain_hosts: Vec<String>         (for ProxyConfig.allowed_hosts → ProxyFilter)
  |    [also adds endpoint route upstreams to plain_hosts — the C5/22e6c40 rider]
  +--> endpoint_routes: Vec<RouteConfig> (credential_key=None, endpoint_rules set)
     |
     v
ProxyConfig { allowed_hosts: plain_hosts, routes: cred_routes + endpoint_routes }
     |
     v
nono-proxy server (runtime per-request)
  [CONNECT request]   → ProxyFilter.check_host(domain) → CONNECT tunnel (no L7)
  [non-CONNECT req]   → RouteStore.get(service)
                           → endpoint_rules.is_allowed(method, path)  [BEFORE credential lookup]
                           → if denied: HTTP 403 + audit log entry
                           → if allowed: CredentialStore.get(service) [SC2 preserved]

nono why --host <host>
  --> parse_host_input(host) → (domain, Option<url_path>)
  --> query_network(domain, port, caps, allowed_domains, domain_endpoints)
  --> if allowed + domain_endpoints has match → QueryResult::Allowed { endpoint_rules: Some(...) }
  --> print_result shows endpoint rules per host
```

### Recommended Project Structure

No new source files are required. All changes are within existing files. The additions touch:

```
crates/nono-cli/src/
├── profile/mod.rs       # AllowDomainEntry enum + merge_allow_domain
├── network_policy.rs    # partition_allow_domain (new fn)
├── proxy_runtime.rs     # replace expand_proxy_allow with partition_allow_domain + rider
├── sandbox_state.rs     # DomainEndpointState + EndpointRuleState types
├── query_ext.rs         # query_network signature + endpoint_rules in QueryResult
├── why_runtime.rs       # resolve_domain_endpoints + pass domain_endpoints
├── sandbox_prepare.rs   # update allow_domain handling (type change Vec<String> → Vec<AllowDomainEntry>)
├── profile_runtime.rs   # allow_domain type reference
├── launch_runtime.rs    # allow_domain type reference
├── execution_runtime.rs # allow_domain type reference
└── main.rs              # tests/examples referencing allow_domain
crates/nono-cli/data/
└── nono-profile.schema.json  # extend allow_domain items schema
crates/nono-cli/Cargo.toml    # add globset = "0.4", urlencoding = "2"
```

### Pattern 1: `AllowDomainEntry` — Backward-Compatible Enum Deserialization

**What:** `#[serde(untagged)]` enum that deserializes from either a plain JSON string or a `{ domain, endpoints }` object.

**When to use:** Whenever a JSON array field needs to accept both string and object entries.

```rust
// Source: git show 0ced085 -- crates/nono-cli/src/profile/mod.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AllowDomainEntry {
    /// Plain hostname — allowed via CONNECT tunnel without L7 inspection.
    Plain(String),
    /// Domain with fine-grained method+path endpoint restrictions.
    /// When endpoints is non-empty, only matching requests are allowed.
    WithEndpoints {
        domain: String,
        #[serde(default)]
        endpoints: Vec<nono_proxy::config::EndpointRule>,
    },
}

impl AllowDomainEntry {
    pub fn domain(&self) -> &str {
        match self {
            Self::Plain(s) => s,
            Self::WithEndpoints { domain, .. } => domain,
        }
    }
}
```

[VERIFIED: git show 0ced085]

### Pattern 2: `parse_allow_domain_arg` — URL-to-AllowDomainEntry

**What:** Convert a CLI `--allow-domain` string to an `AllowDomainEntry`. A URL with a non-root path produces `WithEndpoints` with `method: "*"`.

```rust
// Source: git show 75b2265 -- crates/nono-cli/src/proxy_runtime.rs
fn parse_allow_domain_arg(input: &str) -> crate::profile::AllowDomainEntry {
    if let Ok(parsed) = url::Url::parse(input) {
        let domain = parsed.host_str().unwrap_or(input).to_string();
        let path = parsed.path();
        if path.is_empty() || path == "/" {
            crate::profile::AllowDomainEntry::Plain(domain)
        } else {
            crate::profile::AllowDomainEntry::WithEndpoints {
                domain,
                endpoints: vec![nono_proxy::config::EndpointRule {
                    method: "*".to_string(),
                    path: path.to_string(),
                }],
            }
        }
    } else {
        crate::profile::AllowDomainEntry::Plain(input.to_string())
    }
}
```

[VERIFIED: git show 75b2265]

### Pattern 3: `partition_allow_domain` — Fork-Safe Variant

**What:** Split `Vec<AllowDomainEntry>` into `(plain_hosts, endpoint_routes)`. **Critical fork-divergence:** upstream's `RouteConfig` construction in this function references `proxy: None`, `tls_client_cert: None`, `tls_client_key: None` — fields the fork does NOT have. The fork must use only fork-present `RouteConfig` fields.

```rust
// Source: git show 0ced085 -- crates/nono-cli/src/network_policy.rs
// FORK ADAPTATION NOTE: Remove proxy, tls_client_cert, tls_client_key fields
// from RouteConfig construction — these are upstream-only fields not in the fork.
pub fn partition_allow_domain(
    policy: &NetworkPolicy,
    entries: &[crate::profile::AllowDomainEntry],
) -> Result<(Vec<String>, Vec<RouteConfig>)> {
    let mut plain_hosts = Vec::new();
    let mut endpoint_routes = Vec::new();

    for entry in entries {
        match entry {
            crate::profile::AllowDomainEntry::Plain(host) => {
                let expanded = expand_proxy_allow(policy, std::slice::from_ref(host));
                plain_hosts.extend(expanded);
            }
            crate::profile::AllowDomainEntry::WithEndpoints { domain, endpoints } => {
                if endpoints.is_empty() {
                    let expanded = expand_proxy_allow(policy, std::slice::from_ref(domain));
                    plain_hosts.extend(expanded);
                } else {
                    if domain.is_empty() {
                        return Err(NonoError::ConfigParse(
                            "allow_domain entry with endpoints must have a non-empty domain"
                                .to_string(),
                        ));
                    }
                    let prefix = format!("_ep_{}", domain);
                    let scheme = if is_loopback_domain(domain) { "http" } else { "https" };
                    endpoint_routes.push(RouteConfig {
                        prefix,
                        upstream: format!("{}://{}", scheme, domain),
                        credential_key: None,
                        inject_mode: InjectMode::default(),
                        inject_header: "Authorization".to_string(),
                        credential_format: None,
                        path_pattern: None,
                        path_replacement: None,
                        query_param_name: None,
                        env_var: None,
                        endpoint_rules: endpoints.clone(),
                        tls_ca: None,
                        oauth2: None,
                        // NOTE: NO proxy/tls_client_cert/tls_client_key — fork divergence
                    });
                }
            }
        }
    }
    Ok((plain_hosts, endpoint_routes))
}
```

[VERIFIED: git show 0ced085 — fork adaptation documented]

### Pattern 4: `proxy_runtime.rs` — C5 Filter-Allowlist Rider (`22e6c40`)

**What:** After calling `partition_allow_domain`, add endpoint route upstreams to `plain_hosts` so the proxy filter allowlist grants upstream TCP access for TLS-intercept routes. This is the 12-line rider from `22e6c40`.

```rust
// Source: git show 22e6c40 -- crates/nono-cli/src/proxy_runtime.rs
// Applied AFTER partition_allow_domain exists (C3 prerequisite).
let (mut plain_hosts, endpoint_routes) =
    network_policy::partition_allow_domain(&net_policy, &proxy.allow_domain)?;
// Endpoint-restricted domains need filter allowlist access so the proxy
// can reach upstream after TLS interception.
for route in &endpoint_routes {
    if let Some(ref hp) = route.upstream.strip_prefix("https://") {
        plain_hosts.push(hp.to_string());
    } else if let Some(ref hp) = route.upstream.strip_prefix("http://") {
        plain_hosts.push(hp.to_string());
    }
}
routes.extend(endpoint_routes);
resolved.routes = routes;
let mut proxy_config = network_policy::build_proxy_config(&resolved, &plain_hosts);
```

[VERIFIED: git show 22e6c40]

### Pattern 5: `merge_allow_domain` — Profile Inheritance

**What:** Merge `allow_domain` entries from base and child profiles. Entries for the same domain have their endpoint rules unioned (appended). A `Plain` entry merged with a `WithEndpoints` for the same domain produces `WithEndpoints`.

```rust
// Source: git show 0ced085 -- crates/nono-cli/src/profile/mod.rs
pub(crate) fn merge_allow_domain(
    base: &[AllowDomainEntry],
    child: &[AllowDomainEntry],
) -> Vec<AllowDomainEntry> {
    let mut domains: Vec<String> = Vec::new();
    let mut rules: HashMap<String, Vec<EndpointRule>> = HashMap::new();

    for entry in base.iter().chain(child.iter()) {
        let (domain, endpoints) = match entry {
            AllowDomainEntry::Plain(d) => (d.clone(), &[][..]),
            AllowDomainEntry::WithEndpoints { domain, endpoints } => (domain.clone(), endpoints.as_slice()),
        };
        if !domains.contains(&domain) {
            domains.push(domain.clone());
        }
        rules.entry(domain).or_default().extend_from_slice(endpoints);
    }

    domains.into_iter().map(|domain| {
        let endpoints = rules.remove(&domain).unwrap_or_default();
        if endpoints.is_empty() {
            AllowDomainEntry::Plain(domain)
        } else {
            AllowDomainEntry::WithEndpoints { domain, endpoints }
        }
    }).collect()
}
```

[VERIFIED: git show 0ced085]

### Pattern 6: `DomainEndpointState` — Sandbox State for `nono why`

**What:** New serializable types in `sandbox_state.rs` to persist and surface endpoint-rule metadata in the `nono why --host` diagnostic.

```rust
// Source: git show 0ced085 -- crates/nono-cli/src/sandbox_state.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEndpointState {
    pub domain: String,
    pub endpoints: Vec<EndpointRuleState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRuleState {
    pub method: String,
    pub path: String,
}
```

`SandboxState` gains `domain_endpoints: Vec<DomainEndpointState>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.

[VERIFIED: git show 0ced085]

### Anti-Patterns to Avoid

- **Blind cherry-pick of `0ced085`:** Upstream's `RouteConfig` has fields not present in the fork (`proxy`, `tls_client_cert`, `tls_client_key`). A blind cherry-pick will fail to compile. Always diff-inspect `partition_allow_domain`'s `RouteConfig` construction and strip upstream-only fields.
- **Importing `tls_intercept/handle.rs` logic from `22e6c40`:** The fork does NOT have `tls_intercept/` module. The `22e6c40` commit's `proxy_runtime.rs` rider is portable; its `tls_intercept/handle.rs` changes are not. Confirmed by `54-DIVERGENCE-LEDGER.md` SC4 verdict.
- **String `starts_with` for path component comparison:** CLAUDE.md prohibits this. Use component-wise `Path::starts_with` or the existing `globset` glob matcher. The `normalize_path` function already handles path decomposition correctly.
- **Changing `credential.rs` in any way:** `c9f25164` invariant. `22e6c40` confirmed to not touch `credential.rs` (`git show 22e6c40 --stat` shows only `proxy_runtime.rs` + `tls_intercept/handle.rs`).
- **Omitting the `Upstream-commit:` trailers:** Every ported commit must carry `Upstream-commit: <sha>` per D-19 (Phase 55 discipline).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Glob path matching | Custom regex | `globset::Glob` + `GlobMatcher` | Already in fork (`config.rs`), handles `*`/`**` semantics correctly |
| Percent-decode in path normalization | Custom decode | `urlencoding::decode_binary` | Already in `config.rs::normalize_path`; handles invalid UTF-8 via `from_utf8_lossy` |
| URL parsing for `--allow-domain` arg | Custom string splitting | `url::Url::parse` | Already a workspace dep; handles schemes, hosts, paths, query strings correctly |
| Backward-compatible JSON deserialization | `#[serde(rename)]` gymnastics | `#[serde(untagged)]` enum | Upstream-proven; string entries still parse as `Plain(String)` |
| Endpoint rule enforcement in proxy | Custom middleware | Existing `CompiledEndpointRules::is_allowed()` in `reverse.rs` | Already wired; rule check is at line 96 before credential lookup at line 119 |

**Key insight:** The proxy-layer enforcement (`CompiledEndpointRules` + `reverse.rs` 403 path) is already complete and tested. This phase adds only the CLI-side machinery to populate those rules from `--allow-domain` entries and the profile field.

---

## Common Pitfalls

### Pitfall 1: Fork `RouteConfig` Field Divergence

**What goes wrong:** `partition_allow_domain` in `0ced085` constructs `RouteConfig` with `proxy: None`, `tls_client_cert: None`, `tls_client_key: None` — fields present in upstream but absent in the fork. The code will not compile.

**Why it happens:** The fork diverged from upstream's `tls_intercept/` architecture in Phase 34. Upstream added `proxy`, `tls_client_cert`, `tls_client_key` to `RouteConfig` in later phases; the fork never absorbed them.

**How to avoid:** In the fork's `partition_allow_domain`, construct `RouteConfig` with only fork-present fields: `prefix`, `upstream`, `credential_key`, `inject_mode`, `inject_header`, `credential_format`, `path_pattern`, `path_replacement`, `query_param_name`, `env_var`, `endpoint_rules`, `tls_ca`, `oauth2`. Verify by grepping `pub struct RouteConfig` in `crates/nono-proxy/src/config.rs`.

**Warning signs:** Compiler errors mentioning `unknown field 'proxy'` or `unknown field 'tls_client_cert'`.

### Pitfall 2: `allow_domain` Type Change Propagation

**What goes wrong:** Changing `NetworkConfig.allow_domain` from `Vec<String>` to `Vec<AllowDomainEntry>` cascades into many callers. Missing a call site causes a compiler error.

**Why it happens:** The field is referenced in ~12 files in `nono-cli/src/`. Partial porting leaves type mismatches.

**How to avoid:** After introducing the `AllowDomainEntry` type and changing the field type, do a full `cargo check --bin nono` to surface all call sites. Upstream's `0ced085` touches: `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `network_policy.rs`, `profile/mod.rs`, `profile_cmd.rs`, `profile_runtime.rs`, `proxy_runtime.rs`, `query_ext.rs`, `sandbox_prepare.rs`, `sandbox_state.rs`, `why_runtime.rs`. Use this list as the full impact set.

**Warning signs:** `mismatched types: expected String, found AllowDomainEntry` or vice versa.

### Pitfall 3: `expand_proxy_allow` Still Called with Old Signature

**What goes wrong:** `proxy_runtime.rs::build_proxy_config_from_flags` currently calls `network_policy::expand_proxy_allow(&net_policy, &proxy.allow_domain)`. After the type change, `proxy.allow_domain` is `Vec<AllowDomainEntry>`, not `Vec<String>`. The old call will not compile. It must be replaced with `partition_allow_domain`.

**Why it happens:** The migration has two steps: (1) type change, (2) call-site update. Missing step 2 breaks the build.

**How to avoid:** Replace the `expand_proxy_allow` call in `build_proxy_config_from_flags` with `partition_allow_domain`, then add the C5 rider loop immediately after. This is the exact change shown in Pattern 4.

**Warning signs:** Compiler error at `proxy_runtime.rs` line ~164 mentioning type mismatch on `expand_proxy_allow` argument.

### Pitfall 4: `_ep_` Prefix Collision with Credential Route Prefixes

**What goes wrong:** `partition_allow_domain` uses `format!("_ep_{}", domain)` as the `RouteConfig.prefix` for endpoint routes. If an operator has a credential with a prefix that starts with `_ep_` (unlikely but possible), there could be a collision.

**Why it happens:** The prefix is synthetic and chosen to be distinctive. Upstream chose `_ep_` as a convention.

**How to avoid:** Document the convention. The underscore prefix is not a valid service name in normal use (no API SDK uses `_ep_` as a base URL path). This is the upstream approach — adopt it verbatim.

### Pitfall 5: `nono why --host` with a URL arg hitting the Host Filter on the wrong field

**What goes wrong:** `75b2265` changes `args.host` parsing in `why_runtime.rs` to call `parse_host_input(host)` which splits a URL into `(domain, Option<path>)`. If the path from the URL is tested against endpoint rules that don't exist in the current session state (e.g., `--self` mode hasn't loaded them), `query_network` may incorrectly show "denied" due to wrong domain.

**Why it happens:** The `parse_host_input` function must extract just the hostname from `https://github.com/org/**` for the `HostFilter.check_host` call.

**How to avoid:** Follow the upstream `parse_host_input` implementation exactly. It uses `url::Url::parse(host).host_str()` to extract the domain — the path is kept separate for endpoint-rule matching, not used in the filter check.

**Warning signs:** `nono why --host https://github.com/org/**` reports "denied" even when `github.com` is in the allowlist.

### Pitfall 6: Cross-Target Clippy on `cfg`-gated Code

**What goes wrong:** `0ced085` touches `execution_runtime.rs` which may have `#[cfg(unix)]` / `#[cfg(windows)]` branches. Changes to `AllowDomainEntry` in platform-generic code may trigger clippy failures on the non-host platform.

**Why it happens:** CLAUDE.md cross-target clippy MUST — Windows-host `cargo check` does not run clippy on Unix cfg branches.

**How to avoid:** After porting, run `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used` on Windows AND verify the cross-target clippy gate per CLAUDE.md (or flag as PARTIAL if cross-toolchain not installed). The `allow_domain` type change is in platform-generic code; the downstream callers may be platform-specific.

---

## Runtime State Inventory

Not applicable — this is a new feature addition (not a rename/refactor/migration). No stored data, live service config, OS-registered state, secrets, or build artifacts reference the `allow_domain` field name in a way that needs migration.

---

## Environment Availability

| Dependency | Required By | Available | Fallback |
|------------|------------|-----------|----------|
| `globset` crate | `nono-cli` Cargo.toml (new dep) | Yes (already in workspace via `nono-proxy`) | — |
| `urlencoding` crate | `nono-cli` Cargo.toml (new dep) | Yes (already in workspace via `nono-proxy`) | — |
| Rust toolchain | Build | Yes (Windows, 1.77+) | — |
| upstream remote (`upstream/`) | `git show 0ced085` diff-inspect | Yes — confirmed reachable, tags v0.58.0/v0.59.0 resolve locally | — |

**Missing dependencies with no fallback:** None.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner |
| Config file | `Makefile` targets (`make test`, `make test-cli`) |
| Quick run command | `cargo test --bin nono -p nono-cli -- allow_domain` |
| Full suite command | `make test` (workspace) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REQ-NET-01 SC1 | `--allow-domain https://api.example.com/v1` restricts to `/v1` prefix; `/v2` request gets HTTP 403 from proxy, not silent pass-through | integration (proxy) | `cargo test -p nono-proxy -- endpoint` | Yes (existing `config.rs` tests) |
| REQ-NET-01 SC1 | Endpoint rule check runs before credential injection (hard 403 on denied path) | unit (reverse.rs) | `cargo test -p nono-proxy -- reverse` | Yes (existing `reverse.rs` handler tests) |
| REQ-NET-01 SC2 | Audit log entry for endpoint denial contains host + path + method, audit entry emitted BEFORE any credential lookup | unit (reverse.rs audit path) | `cargo test -p nono-proxy -- audit` | Yes (existing audit tests) |
| REQ-NET-01 SC3 | `nono why --host api.github.com` shows endpoint rules when host has scoped entries | unit (query_ext.rs) | `cargo test -p nono-cli -- query_network` | Exists but needs extension |
| REQ-NET-01 SC3 | `nono why --host https://github.com/org/**` parses URL and shows endpoint-restricted verdict | unit (query_ext.rs) | `cargo test -p nono-cli -- parse_host_input` | Wave 0 gap — test file exists, new function needs test |
| REQ-NET-01 SC4 | `credential.rs` SHA unchanged — byte-identical to `c9f25164` | verification | `git show HEAD:crates/nono-proxy/src/credential.rs \| sha256sum` | Verification step, not a unit test |
| REQ-NET-01 SC4 | Committed `Upstream-commit:` trailers on ported commits | doc check | `git log --oneline -5 \| grep Upstream-commit` | Verification step |

### Sampling Rate

- **Per task commit:** `cargo test -p nono-cli -p nono-proxy -- allow_domain partition endpoint`
- **Per wave merge:** `make test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `crates/nono-cli/src/proxy_runtime.rs` — `parse_allow_domain_arg` unit tests (covers plain hostname, URL with path, root URL → plain, deep path). Upstream provides these in `75b2265` — port them directly.
- [ ] `crates/nono-cli/src/network_policy.rs` — `partition_allow_domain` unit tests (plain entries, with-endpoints, empty-endpoints-as-plain, rejects-empty-domain). Upstream provides these in `0ced085`.
- [ ] `crates/nono-cli/src/profile/mod.rs` — `merge_allow_domain` unit tests + `AllowDomainEntry` deserialization tests (plain string, object with endpoints, backward-compat mixed array). Upstream provides these.
- [ ] `crates/nono-cli/src/query_ext.rs` — `parse_host_input` + `path_matches_endpoint_rules` tests. Upstream provides these in `75b2265`.

*(All Wave 0 gap tests are available in the upstream commits — port them alongside the feature code.)*

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V4 Access Control | Yes | `CompiledEndpointRules::is_allowed()` enforces default-deny on endpoint-restricted routes; 403 on mismatch |
| V5 Input Validation | Yes | `normalize_path` (percent-decode, slash-collapse); `url::Url::parse` for CLI input; empty-domain rejection in `partition_allow_domain` |
| V6 Cryptography | No | No new crypto surface |
| V2 Authentication | No (existing) | `credential.rs` byte-identical; session token auth unchanged |
| V3 Session Management | No | No new session surface |

### Known Threat Patterns for Proxy L7 Enforcement

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Percent-encoding bypass (`/v1/%63hat` to access `/v1/chat`) | Tampering | `normalize_path` percent-decodes before glob match — already in place |
| Double-slash bypass (`/v1//chat`) | Tampering | `normalize_path` collapses empty segments — already in place |
| Method case bypass (`get` vs `GET`) | Tampering | `eq_ignore_ascii_case` in `is_allowed` — already in place |
| CONNECT bypass to endpoint-restricted host | Evasion | `server.rs` blocks CONNECT to `is_route_upstream()` hosts with 403; endpoint routes ARE route upstreams → CONNECT blocked |
| Empty domain in structured `allow_domain` entry | Tampering | Fail-closed: `partition_allow_domain` returns `Err` for empty domain — `0ced085` pattern |
| Bare `--allow-domain host` silently widens a scoped host | Spoofing | D-12 acknowledged WATCH-ITEM; `nono why --host` output should make effective openness visible |
| Credential injection before endpoint check | Spoofing | Already structurally impossible: `reverse.rs` line 96 (endpoint check) precedes line 119 (credential lookup) |
| Path traversal via `../` in URL arg | Tampering | `url::Url::parse` normalizes `..` at parse time; `normalize_path` further strips empty segments |

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `allow_domain: Vec<String>` (host-only) | `allow_domain: Vec<AllowDomainEntry>` (host or host+endpoint rules) | v0.59.0 upstream (`0ced085`) | Operators can scope network access to specific API endpoints, not just hostnames |
| `--allow-domain api.github.com` (host only) | `--allow-domain https://api.github.com/org/**` (URL with glob path) | v0.59.0 upstream (`75b2265`) | CLI parity with profile field; URL syntax directly expresses endpoint scoping |
| Endpoint rules only for credential routes | Endpoint rules also for plain allow_domain entries | Phase 56 (this phase) | Endpoint-restricted routes created via `partition_allow_domain` without credentials |

**Deprecated/outdated:**
- Direct use of `expand_proxy_allow` with `Vec<AllowDomainEntry>` is superseded by `partition_allow_domain` — `expand_proxy_allow` remains for group expansion but is called internally.
- `EffectiveProxySettings.allow_domain: Vec<String>` becomes `Vec<AllowDomainEntry>`.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Fork's `profile/mod.rs` `allow_domain` is still `Vec<String>` — Phase 55 did NOT change it | Standard Stack, Pitfall 2 | If Phase 55 partially ported `AllowDomainEntry`, the phase scope may be smaller or there may be merge conflicts; verify with `grep "pub allow_domain" crates/nono-cli/src/profile/mod.rs` before starting |
| A2 | `nono-proxy/src/config.rs::RouteConfig` does NOT have `proxy`, `tls_client_cert`, `tls_client_key` fields | Pattern 3 (Pitfall 1) | If these were added by Phase 55 or out-of-band, the fork-adaptation note in `partition_allow_domain` is unnecessary; verify with `grep -n "tls_client" crates/nono-proxy/src/config.rs` |

**Both assumptions are LOW risk:** both were verified directly from the live codebase via `grep` in this research session. If rechecked at plan time, expect them to still hold.

---

## Open Questions (RESOLVED)

1. **`sandbox_state.rs::SandboxState::from_caps` signature** — upstream `0ced085` adds a fourth `domain_endpoints` parameter to `from_caps`. The fork's current signature and all call sites need to be updated.
   - **RESOLVED (planner grep 2026-06-04):** `grep -rn "SandboxState::from_caps" crates/` returns **5 call sites in `sandbox_state.rs` test module** (lines 490, 508, 552, 581, 610) that use the 3-arg `(caps, bypass_paths, allowed_domains)` CLI signature — all 5 must gain a 4th `&[]` arg. The production call site in `execution_runtime.rs` line 514 is the one wired by Plan 02. The 2 call sites in `nono/src/state.rs` and 2 in `capability_ext.rs` use a different 1-arg library `from_caps` — unaffected. Total CLI test sites: **5**.

2. **`profile_cmd.rs` allow_domain rendering** — `0ced085` modifies `profile_cmd.rs` to render `AllowDomainEntry` items with their endpoint rules in the `nono profile show` output.
   - **RESOLVED (planner diff-inspect 2026-06-04):** `git show 0ced085 -- crates/nono-cli/src/profile_cmd.rs` was diff-inspected and confirms the upstream diff is a clean port for the fork (the `cmd_show` function at line 1039 is the primary change site; ~78 lines of endpoint-rule display logic; fork's `profile_cmd.rs` structure is sufficiently close to upstream — Plan 03 handles this with standard diff-inspect-then-adapt pattern).

---

## Sources

### Primary (HIGH confidence)
- `git show 0ced085` — full diff of `feat(cli): support fine-grained method+path restrictions in allow_domain (#960)`. Read directly.
- `git show 75b2265` — full diff of `feat(cli): allow-domain accepts URL with path for endpoint restrictions`. Read directly.
- `git show 22e6c40` — full diff of `fix(proxy): enforce endpoint rules before credential selection in TLS intercept`. Read directly.
- `crates/nono-proxy/src/route.rs` — fork's `RouteStore` + `LoadedRoute.endpoint_rules`. Read directly.
- `crates/nono-proxy/src/credential.rs` — fork's `CredentialStore`. Read directly (byte-identical invariant confirmed).
- `crates/nono-proxy/src/reverse.rs` — endpoint rule check before credential lookup (lines 96, 119). Read directly.
- `crates/nono-proxy/src/config.rs` — `CompiledEndpointRules`, `EndpointRule`, `normalize_path`. Read directly.
- `crates/nono-proxy/src/server.rs` — `ProxyState` routing, CONNECT block for route upstreams. Read directly.
- `crates/nono-cli/src/cli.rs` — current `--allow-domain` flag definition. Read directly.
- `crates/nono-cli/src/network_policy.rs` — `expand_proxy_allow`, `partition_allow_domain` (absent in fork). Read directly.
- `crates/nono-cli/src/proxy_runtime.rs` — current `build_proxy_config_from_flags`, type of `EffectiveProxySettings.allow_domain`. Read directly.
- `crates/nono-cli/src/profile/mod.rs` — current `NetworkConfig.allow_domain: Vec<String>`. Read directly.
- `.planning/phases/54-upst7-audit/54-DIVERGENCE-LEDGER.md` — SC4 TLS-intercept assessment, C3/C5 cluster verdicts, credential.rs byte-identical. Read directly.

### Secondary (MEDIUM confidence)
- `.planning/phases/55-upst7-cherry-pick-wave/55-REVIEW.md` — CR-01 JSONC dual-key guard description (scoped to `bypass_protection`, not `allow_domain`). Read directly.

### Tertiary (LOW confidence)
- None.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps verified from codebase + upstream diffs
- Architecture: HIGH — upstream diffs read directly; fork code read directly
- Pitfalls: HIGH — based on direct code inspection; Pitfall 1 confirmed by checking `RouteConfig` in `config.rs`
- Method matching: HIGH — confirmed from `config.rs::is_allowed` implementation

**Research date:** 2026-06-04
**Valid until:** 2026-07-04 (stable feature; upstream commits are immutable)
