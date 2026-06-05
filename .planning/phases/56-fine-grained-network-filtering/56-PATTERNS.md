# Phase 56: Fine-grained Network Filtering - Pattern Map

**Mapped:** 2026-06-04
**Files analyzed:** 12 modified files + 1 new function file
**Analogs found:** 12 / 13

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/nono-cli/src/profile/mod.rs` | model | transform | self (existing `NetworkConfig`, `dedup_append` merge pattern) | exact |
| `crates/nono-cli/src/network_policy.rs` | service | transform | self (existing `expand_proxy_allow`, `resolve_credentials`) | exact |
| `crates/nono-cli/src/proxy_runtime.rs` | service | request-response | self (existing `build_proxy_config_from_flags`) | exact |
| `crates/nono-cli/src/sandbox_state.rs` | model | CRUD | self (existing `FsCapState`, `SandboxState`) | exact |
| `crates/nono-cli/src/query_ext.rs` | service | request-response | self (existing `query_network`, `QueryResult`) | exact |
| `crates/nono-cli/src/why_runtime.rs` | controller | request-response | self (existing `resolve_allowed_domains`) | exact |
| `crates/nono-cli/src/sandbox_prepare.rs` | service | transform | self (existing `print_allow_domain_port_warnings`) | exact |
| `crates/nono-cli/src/profile_runtime.rs` | service | transform | self (existing `allow_domain: Vec<String>` field) | exact |
| `crates/nono-cli/src/launch_runtime.rs` | service | transform | self (existing `allow_domain: Vec<String>` field) | exact |
| `crates/nono-cli/src/execution_runtime.rs` | controller | request-response | self (existing `&flags.proxy.allow_domain`) | exact |
| `crates/nono-cli/src/profile_cmd.rs` | controller | request-response | self (existing `net.allow_domain.join(", ")` display) | exact |
| `crates/nono-cli/src/main.rs` | test | — | self (existing `allow_domain: vec![...]` test fixtures) | exact |
| `crates/nono-cli/data/nono-profile.schema.json` | config | — | self (existing `allow_domain` string-array schema) | exact |
| `crates/nono-cli/Cargo.toml` | config | — | `crates/nono-proxy/Cargo.toml` (globset + urlencoding already there) | role-match |

---

## Fork Shape Invariants (Critical Pre-Check)

Before any plan task executes, verify these two assumptions hold:

```bash
# A1: allow_domain is still Vec<String>
grep "pub allow_domain" crates/nono-cli/src/profile/mod.rs
# Expected: pub allow_domain: Vec<String>,   (line 1580)

# A2: RouteConfig has no proxy/tls_client_cert/tls_client_key
grep "tls_client" crates/nono-proxy/src/config.rs
# Expected: no output

# credential.rs invariant
git show HEAD:crates/nono-proxy/src/credential.rs | sha256sum
# Expected SHA: c9f25164 (verify prefix matches)
```

---

## Pattern Assignments

### `crates/nono-cli/src/profile/mod.rs` (model, transform)

**Change type:** Introduce `AllowDomainEntry` enum; change `NetworkConfig.allow_domain` field type; replace `dedup_append` call in merge function with `merge_allow_domain`.

**Analog:** Self — the existing `NetworkConfig` struct and merge function.

**Existing field to change** (`profile/mod.rs` lines 1574–1580):
```rust
#[serde(
    default,
    rename = "allow_domain",
    alias = "proxy_allow",
    alias = "allow_proxy"
)]
pub allow_domain: Vec<String>,
```
Becomes `Vec<AllowDomainEntry>` — keep all three serde attributes unchanged.

**Existing merge call to replace** (`profile/mod.rs` line 3015):
```rust
allow_domain: dedup_append(&base.network.allow_domain, &child.network.allow_domain),
```
Becomes a call to `merge_allow_domain(&base.network.allow_domain, &child.network.allow_domain)`.

**Import block pattern** (`profile/mod.rs` lines 9–13):
```rust
use nono::{NonoError, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
```
Add `use nono_proxy::config::EndpointRule;` after the `nono` import for the `AllowDomainEntry::WithEndpoints` variant.

**New `AllowDomainEntry` enum** (port from `0ced085`, place after existing imports):
```rust
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
**Critical serde note:** `#[serde(untagged)]` is NOT subject to the Phase 55 dual-key guard. The CR-01 guard covers `bypass_protection`/`override_deny` only. The `allow_domain` field uses standard `#[serde(rename)]` — no conflict.

**New `merge_allow_domain` function** (port from `0ced085`, place alongside existing `dedup_append`):
```rust
pub(crate) fn merge_allow_domain(
    base: &[AllowDomainEntry],
    child: &[AllowDomainEntry],
) -> Vec<AllowDomainEntry> {
    let mut domains: Vec<String> = Vec::new();
    let mut rules: HashMap<String, Vec<EndpointRule>> = HashMap::new();

    for entry in base.iter().chain(child.iter()) {
        let (domain, endpoints) = match entry {
            AllowDomainEntry::Plain(d) => (d.clone(), &[][..]),
            AllowDomainEntry::WithEndpoints { domain, endpoints } => {
                (domain.clone(), endpoints.as_slice())
            }
        };
        if !domains.contains(&domain) {
            domains.push(domain.clone());
        }
        rules.entry(domain).or_default().extend_from_slice(endpoints);
    }

    domains
        .into_iter()
        .map(|domain| {
            let endpoints = rules.remove(&domain).unwrap_or_default();
            if endpoints.is_empty() {
                AllowDomainEntry::Plain(domain)
            } else {
                AllowDomainEntry::WithEndpoints { domain, endpoints }
            }
        })
        .collect()
}
```

**`NetworkConfig.has_proxy_flags` update** (`profile/mod.rs` lines 1639–1644) — the `!self.allow_domain.is_empty()` check is unchanged in logic; no edit needed there.

---

### `crates/nono-cli/src/network_policy.rs` (service, transform)

**Change type:** Add `partition_allow_domain` function. Update `collect_allow_domain_port_warnings` to accept `&[AllowDomainEntry]` or add a parallel helper.

**Analog:** `expand_proxy_allow` (lines 311–335) — same fan-out-and-classify structure.

**Existing `expand_proxy_allow` signature** (lines 311–335) — stays unchanged; called internally by `partition_allow_domain`:
```rust
pub fn expand_proxy_allow(policy: &NetworkPolicy, entries: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for entry in entries {
        if let Some(group) = policy.groups.get(entry.as_str()) {
            result.extend(group.hosts.clone());
            for suffix in &group.suffixes {
                // ... wildcard expansion
            }
        } else {
            let host = entry
                .rsplit_once(':')
                .and_then(|(h, p)| p.parse::<u16>().ok().map(|_| h))
                .unwrap_or(entry.as_str());
            result.push(host.to_string());
        }
    }
    result
}
```

**New `partition_allow_domain` function** (port from `0ced085`, FORK ADAPTATION — strip `proxy`, `tls_client_cert`, `tls_client_key` from `RouteConfig` struct literal):

The fork's `RouteConfig` fields (confirmed from `crates/nono-proxy/src/config.rs` lines 81–164):
- `prefix`, `upstream`, `credential_key`, `inject_mode`, `inject_header`, `credential_format`, `path_pattern`, `path_replacement`, `query_param_name`, `env_var`, `endpoint_rules`, `tls_ca`, `oauth2`
- ABSENT (upstream-only): `proxy`, `tls_client_cert`, `tls_client_key`

```rust
use crate::profile::AllowDomainEntry;

pub fn partition_allow_domain(
    policy: &NetworkPolicy,
    entries: &[AllowDomainEntry],
) -> Result<(Vec<String>, Vec<RouteConfig>)> {
    let mut plain_hosts = Vec::new();
    let mut endpoint_routes = Vec::new();

    for entry in entries {
        match entry {
            AllowDomainEntry::Plain(host) => {
                let expanded = expand_proxy_allow(policy, std::slice::from_ref(host));
                plain_hosts.extend(expanded);
            }
            AllowDomainEntry::WithEndpoints { domain, endpoints } => {
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
                        // NOTE: upstream also sets proxy/tls_client_cert/tls_client_key
                        // — these fields are ABSENT from the fork's RouteConfig (Phase 34
                        // fork-preserve decision, confirmed c9f25164 invariant).
                    });
                }
            }
        }
    }
    Ok((plain_hosts, endpoint_routes))
}
```

`is_loopback_domain` helper — check if upstream includes it in `0ced085`; if not, implement as:
```rust
fn is_loopback_domain(domain: &str) -> bool {
    domain == "localhost"
        || domain.starts_with("127.")
        || domain == "::1"
        || domain == "0.0.0.0"
}
```

**Imports to add** at top of `network_policy.rs` (line 8 currently):
```rust
// Add to existing use nono_proxy::config::{...} line:
use nono_proxy::config::{EndpointRule, InjectMode, ProxyConfig, RouteConfig};
// Already present: use crate::profile::CustomCredentialDef;
// Add:
use crate::profile::AllowDomainEntry;
```

**`collect_allow_domain_port_warnings` update** — the existing function takes `&[String]`; after the type change to `Vec<AllowDomainEntry>`, either:
1. Change the signature to `&[AllowDomainEntry]` and extract `.domain()` for the port check, or
2. Add a parallel `collect_allow_domain_port_warnings_typed` helper.
Follow upstream `0ced085` to decide; confirm at diff-inspect time.

---

### `crates/nono-cli/src/proxy_runtime.rs` (service, request-response)

**Change type:** (1) Change `EffectiveProxySettings.allow_domain` type from `Vec<String>` to `Vec<AllowDomainEntry>`. (2) Add `parse_allow_domain_arg` function. (3) Replace `expand_proxy_allow` call with `partition_allow_domain` + C5 rider.

**Analog:** Self — the existing `build_proxy_config_from_flags` function (lines 132–180).

**Field type change** (`proxy_runtime.rs` line 17):
```rust
// Before:
pub(crate) allow_domain: Vec<String>,
// After:
pub(crate) allow_domain: Vec<AllowDomainEntry>,
```

**New `parse_allow_domain_arg` function** (port from `75b2265`):
```rust
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

**`resolve_effective_proxy_settings` update** (lines 112–113 currently):
```rust
// Before:
let mut allow_domain = prepared.allow_domain.clone();
allow_domain.extend(args.allow_proxy.clone());
// After:
let mut allow_domain = prepared.allow_domain.clone();
allow_domain.extend(args.allow_proxy.iter().map(|s| parse_allow_domain_arg(s)));
```

**`build_proxy_config_from_flags` update** — replace lines 163–165:
```rust
// Before:
let expanded_allow_domain =
    network_policy::expand_proxy_allow(&net_policy, &proxy.allow_domain);
let mut proxy_config = network_policy::build_proxy_config(&resolved, &expanded_allow_domain);

// After (Pattern 3 + Pattern 4 combined):
let (mut plain_hosts, endpoint_routes) =
    network_policy::partition_allow_domain(&net_policy, &proxy.allow_domain)?;
// C5 rider (22e6c40): push endpoint route upstreams into plain_hosts so
// the proxy filter allowlist allows upstream TCP for TLS-intercept routes.
for route in &endpoint_routes {
    if let Some(hp) = route.upstream.strip_prefix("https://") {
        plain_hosts.push(hp.to_string());
    } else if let Some(hp) = route.upstream.strip_prefix("http://") {
        plain_hosts.push(hp.to_string());
    }
}
resolved.routes.extend(endpoint_routes);
let mut proxy_config = network_policy::build_proxy_config(&resolved, &plain_hosts);
```

---

### `crates/nono-cli/src/sandbox_state.rs` (model, CRUD)

**Change type:** Add `domain_endpoints: Vec<DomainEndpointState>` field to `SandboxState`; add `DomainEndpointState` and `EndpointRuleState` types; update `from_caps` signature to accept `&[DomainEndpointState]`.

**Analog:** Self — existing `FsCapState` + `SandboxState` pattern (lines 41–88).

**Existing serializable sub-type pattern** (lines 41–54 — copy this structure):
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct FsCapState {
    pub original: String,
    pub path: String,
    pub access: String,
    pub is_file: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}
```

**New types to add** (port from `0ced085`):
```rust
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

**`SandboxState` field addition** — add after `allowed_domains` (line 37):
```rust
/// Endpoint-scoped rules per domain at sandbox creation time
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub domain_endpoints: Vec<DomainEndpointState>,
```

**`from_caps` signature change** (lines 58–62):
```rust
// Before:
pub fn from_caps(
    caps: &CapabilitySet,
    bypass_protection_paths: &[PathBuf],
    allowed_domains: &[String],
) -> Self {
// After:
pub fn from_caps(
    caps: &CapabilitySet,
    bypass_protection_paths: &[PathBuf],
    allowed_domains: &[String],
    domain_endpoints: &[DomainEndpointState],
) -> Self {
```
All existing `SandboxState::from_caps` call sites gain `&[]` as the fourth argument (check with `grep -rn "SandboxState::from_caps" crates/` at plan time to find all call sites).

---

### `crates/nono-cli/src/query_ext.rs` (service, request-response)

**Change type:** (1) Add `endpoint_rules` field to `QueryResult::Allowed` variant. (2) Update `query_network` signature to accept `domain_endpoints: &[DomainEndpointState]`. (3) Add `parse_host_input` and `path_matches_endpoint_rules` functions.

**Analog:** Self — existing `query_network` function (lines 235–295) and `QueryResult` enum (lines 44–86).

**Existing `QueryResult::Allowed` variant** (lines 47–56):
```rust
#[serde(rename = "allowed")]
Allowed {
    reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    granted_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
},
```
Add a new optional field for endpoint rules display (D-11):
```rust
#[serde(skip_serializing_if = "Option::is_none")]
endpoint_rules: Option<Vec<EndpointRuleDisplay>>,
```
where `EndpointRuleDisplay` is a display-only struct with `method: String, path: String`.

**Existing `query_network` signature** (line 235–239):
```rust
pub fn query_network(
    host: &str,
    port: u16,
    caps: &CapabilitySet,
    allowed_domains: &[String],
) -> QueryResult {
```
After this phase:
```rust
pub fn query_network(
    host: &str,
    port: u16,
    caps: &CapabilitySet,
    allowed_domains: &[String],
    domain_endpoints: &[crate::sandbox_state::DomainEndpointState],
) -> QueryResult {
```

**New `parse_host_input` function** (port from `75b2265`):
```rust
/// Parse a `--host` argument that may be a bare hostname or a URL.
/// Returns `(domain, Option<path>)`.
pub fn parse_host_input(host: &str) -> (String, Option<String>) {
    if let Ok(parsed) = url::Url::parse(host) {
        let domain = parsed.host_str().unwrap_or(host).to_string();
        let path = parsed.path();
        let path_opt = if path.is_empty() || path == "/" {
            None
        } else {
            Some(path.to_string())
        };
        (domain, path_opt)
    } else {
        (host.to_string(), None)
    }
}
```

**Existing `print_result` extension** (lines 383–455) — add a branch in the `Allowed` arm to print endpoint rules when present:
```rust
if let Some(ref rules) = endpoint_rules {
    if !rules.is_empty() {
        println!("  Endpoint rules:");
        for r in rules {
            println!("    {} {}", r.method, r.path);
        }
    }
}
```

---

### `crates/nono-cli/src/why_runtime.rs` (controller, request-response)

**Change type:** (1) Update `resolve_allowed_domains` to also return `domain_endpoints`. (2) Pass `domain_endpoints` to `query_network`.

**Analog:** Self — existing `resolve_allowed_domains` + `run_why` functions (lines 14–176).

**Existing `resolve_allowed_domains` structure** (lines 14–44):
```rust
fn resolve_allowed_domains(profile: &profile::Profile) -> Vec<String> {
    // ... loads net_policy, resolves profile hosts, calls expand_proxy_allow
    domains
}
```
After: rename to `resolve_proxy_context` returning a struct `(Vec<String>, Vec<DomainEndpointState>)`, OR keep `resolve_allowed_domains` and add `resolve_domain_endpoints`. Follow upstream `0ced085` for the exact shape; if upstream adds a separate function, use that.

**Existing `query_network` call** (line 159):
```rust
query_network(host, args.port, &ctx.caps, &ctx.allowed_domains)
```
After:
```rust
query_network(host, args.port, &ctx.caps, &ctx.allowed_domains, &ctx.domain_endpoints)
```

`WhyContext` struct gains:
```rust
domain_endpoints: Vec<crate::sandbox_state::DomainEndpointState>,
```

---

### `crates/nono-cli/src/profile_runtime.rs` (service, transform)

**Change type:** Field type change `allow_domain: Vec<String>` → `Vec<AllowDomainEntry>`.

**Analog:** Self — `PreparedProfile.allow_domain` field (line 18) + population at lines 543–546.

**Field declaration** (line 18):
```rust
// Before:
pub(crate) allow_domain: Vec<String>,
// After:
pub(crate) allow_domain: Vec<crate::profile::AllowDomainEntry>,
```

**Field population** (lines 543–546):
```rust
allow_domain: loaded_profile
    .as_ref()
    .map(|profile| profile.network.allow_domain.clone())
    .unwrap_or_default(),
```
No change needed — `profile.network.allow_domain` is now `Vec<AllowDomainEntry>`, so `.clone()` and `.unwrap_or_default()` still work.

---

### `crates/nono-cli/src/launch_runtime.rs` (service, transform)

**Change type:** Field type change `allow_domain: Vec<String>` → `Vec<AllowDomainEntry>` in `ProxyLaunchOptions`.

**Analog:** Self — `ProxyLaunchOptions.allow_domain` field (line 100).

```rust
// Before:
pub(crate) allow_domain: Vec<String>,
// After:
pub(crate) allow_domain: Vec<crate::profile::AllowDomainEntry>,
```

---

### `crates/nono-cli/src/execution_runtime.rs` (controller, request-response)

**Change type:** `SandboxState::from_caps` call gains a fourth `domain_endpoints` argument.

**Analog:** Self — existing call site at line 164 (`&flags.proxy.allow_domain`).

The call at line 164 currently passes `&flags.proxy.allow_domain` as `allowed_domains`. After the type changes, this is `Vec<AllowDomainEntry>`. The fourth arg is `domain_endpoints` — derive it from `flags.proxy.allow_domain` by extracting `WithEndpoints` entries:
```rust
SandboxState::from_caps(
    &caps,
    &bypass_protection_paths,
    &plain_allowed_domains,   // Vec<String>, extracted from allow_domain
    &domain_endpoints,        // Vec<DomainEndpointState>, from WithEndpoints entries
)
```
Follow upstream `0ced085` for the exact construction pattern.

---

### `crates/nono-cli/src/sandbox_prepare.rs` (service, transform)

**Change type:** `print_allow_domain_port_warnings` call site — the function currently takes `&[String]`; after the type change it takes `&[AllowDomainEntry]` or uses `.domain()` to extract strings.

**Analog:** Self — existing helper (line 18):
```rust
fn print_allow_domain_port_warnings(entries: &[String], context: &str, silent: bool) {
    for warning in network_policy::collect_allow_domain_port_warnings(entries, context) {
        // ...
    }
}
```
Update signature to `&[AllowDomainEntry]` and extract domains inline, or adapt `collect_allow_domain_port_warnings` to take `&[AllowDomainEntry]`. Follow upstream for the exact change.

---

### `crates/nono-cli/src/profile_cmd.rs` (controller, request-response)

**Change type:** Display rendering of `allow_domain` entries must handle both `Plain(String)` and `WithEndpoints { domain, endpoints }`.

**Analog:** Self — existing display at lines 1575–1579:
```rust
if !net.allow_domain.is_empty() {
    println!(
        "  {}: {}",
        theme::fg("allow_domain", t.subtext),
        net.allow_domain.join(", ")
    );
}
```
After the type change, `allow_domain` is `Vec<AllowDomainEntry>`. The `join(", ")` call must be replaced. Follow upstream `0ced085`'s `profile_cmd.rs` diff for the exact rendering (check `git show 0ced085 -- crates/nono-cli/src/profile_cmd.rs` at plan time).

For the diff/compare functions (lines 2068–2070, 2599), use `.domain()` accessor or upstream's comparison helpers.

---

### `crates/nono-cli/src/main.rs` (test)

**Change type:** Test fixtures that construct `EffectiveProxySettings` / `ProxyLaunchOptions` with `allow_domain: vec![...]` must use `AllowDomainEntry::Plain(...)` instead of bare strings.

**Analog:** Self — existing test fixtures (lines 266, 317, 348):
```rust
allow_domain: vec!["docs.python.org".to_string()],
```
After:
```rust
allow_domain: vec![crate::profile::AllowDomainEntry::Plain("docs.python.org".to_string())],
```

---

### `crates/nono-cli/data/nono-profile.schema.json` (config)

**Change type:** Extend `allow_domain` items from `{ "type": "string" }` to `{ "oneOf": [ { "type": "string" }, { "$ref": "#/$defs/AllowDomainWithEndpoints" } ] }`. Add `AllowDomainWithEndpoints` definition.

**Analog:** Self — existing `allow_domain` array item schema.

**Upstream schema verbatim** (from `0ced085` diff):
```json
{
  "$defs": {
    "AllowDomainWithEndpoints": {
      "type": "object",
      "required": ["domain"],
      "properties": {
        "domain": { "type": "string" },
        "endpoints": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["method", "path"],
            "properties": {
              "method": { "type": "string" },
              "path": { "type": "string" }
            }
          }
        }
      }
    }
  }
}
```
The `allow_domain` array items change from:
```json
{ "type": "string" }
```
to:
```json
{ "oneOf": [ { "type": "string" }, { "$ref": "#/$defs/AllowDomainWithEndpoints" } ] }
```

---

### `crates/nono-cli/Cargo.toml` (config)

**Change type:** Add `globset = "0.4"` and `urlencoding = "2"` dependencies.

**Analog:** `crates/nono-proxy/Cargo.toml` — both packages already present there.

```toml
# Add to [dependencies] section in crates/nono-cli/Cargo.toml
globset = "0.4"
urlencoding = "2"
```

---

## Shared Patterns

### Serde Untagged Enum (AllowDomainEntry)
**Source:** RESEARCH.md Pattern 1 (verified from `git show 0ced085`)
**Apply to:** `profile/mod.rs` `AllowDomainEntry` definition
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AllowDomainEntry {
    Plain(String),
    WithEndpoints { domain: String, #[serde(default)] endpoints: Vec<EndpointRule> },
}
```
Backward-compatible: existing profile JSON with plain strings still deserializes as `Plain(String)`.

### RouteConfig Construction (Fork-Safe)
**Source:** `crates/nono-proxy/src/config.rs` lines 81–164 (confirmed field set)
**Apply to:** `network_policy.rs::partition_allow_domain`
Fork-present fields only: `prefix`, `upstream`, `credential_key`, `inject_mode`, `inject_header`, `credential_format`, `path_pattern`, `path_replacement`, `query_param_name`, `env_var`, `endpoint_rules`, `tls_ca`, `oauth2`.
**ABSENT** (upstream-only, do NOT include): `proxy`, `tls_client_cert`, `tls_client_key`.

### Error Handling
**Source:** `crates/nono-cli/src/network_policy.rs` lines 195–200
**Apply to:** `partition_allow_domain` empty-domain check
```rust
return Err(NonoError::ConfigParse(
    "allow_domain entry with endpoints must have a non-empty domain".to_string(),
));
```

### Upstream-commit Trailer Discipline
**Source:** Phase 55 D-19 pattern
**Apply to:** All three ported commits (`0ced085`, `75b2265`, `22e6c40`)
Each commit message must end with:
```
Upstream-commit: <sha>
```

### `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
**Source:** `crates/nono-cli/src/sandbox_state.rs` lines 33–37 (`bypass_protection_paths` field)
**Apply to:** `SandboxState.domain_endpoints` field
Keeps backward compatibility — old `NONO_CAP_FILE` JSON without the field deserializes as empty `Vec`.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `parse_host_input` function (in `query_ext.rs`) | utility | transform | New URL-parsing helper; no existing URL-to-hostname splitter in `query_ext.rs`. Pattern comes from `75b2265` upstream — use `url::Url::parse(host).host_str()` pattern. |
| `path_matches_endpoint_rules` function (in `query_ext.rs`) | utility | transform | New glob-path test for `nono why` SC3 display; closest analog is `CompiledEndpointRules::is_allowed` in `nono-proxy/src/config.rs` lines 214–224, but that is a compiled runtime path. Use `globset` directly for the `query_ext` diagnostic helper, as in `75b2265`. |

---

## Critical Anti-Pattern Reminders (for Planner)

1. **`RouteConfig` construction in `partition_allow_domain`:** Do NOT include `proxy`, `tls_client_cert`, or `tls_client_key` fields. They are upstream-only. Confirmed absent from `crates/nono-proxy/src/config.rs`.

2. **`credential.rs` is untouchable:** `crates/nono-proxy/src/credential.rs` MUST remain byte-identical (SHA `c9f25164`). Phase 56 has no reason to touch it — `22e6c40` confirmed it does not touch `credential.rs`.

3. **No import of `tls_intercept/`:** The fork has no `tls_intercept/` module. Take only the 12-line `proxy_runtime.rs` rider from `22e6c40`; ignore `tls_intercept/handle.rs` changes entirely.

4. **Type change cascade:** `allow_domain: Vec<String>` → `Vec<AllowDomainEntry>` cascades into ~12 files. Use `cargo check --bin nono` after introducing the type to surface all call sites. Full impact list: `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `network_policy.rs`, `profile/mod.rs`, `profile_cmd.rs`, `profile_runtime.rs`, `proxy_runtime.rs`, `query_ext.rs`, `sandbox_prepare.rs`, `sandbox_state.rs`, `why_runtime.rs`.

5. **`SandboxState::from_caps` call site count:** Run `grep -rn "SandboxState::from_caps" crates/` before writing the plan task to get the exact count of call sites needing a `&[]` fourth argument.

6. **Cross-target clippy MUST:** After porting, run `cargo clippy --bin nono -- -D warnings -D clippy::unwrap_used`. If the cross-toolchain is not installed, mark the REQ PARTIAL per CLAUDE.md.

---

## Metadata

**Analog search scope:** `crates/nono-cli/src/`, `crates/nono-proxy/src/`, `crates/nono-cli/data/`
**Files scanned:** 14 source files read directly; 4 grep searches
**Pattern extraction date:** 2026-06-04
