---
phase: 56-fine-grained-network-filtering
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/src/sandbox_state.rs
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/src/profile_runtime.rs
  - crates/nono-cli/src/launch_runtime.rs
  - crates/nono-cli/src/execution_runtime.rs
  - crates/nono-cli/src/sandbox_prepare.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/profile_cmd.rs
  - crates/nono-cli/src/query_ext.rs
  - crates/nono-cli/src/why_runtime.rs
  - crates/nono-cli/data/nono-profile.schema.json
findings:
  critical: 2
  warning: 3
  info: 3
  total: 8
status: issues_found
resolved:
  - CR-01  # fixed in 05cd7580 — PreparedSandbox.allow_domain threaded as Vec<AllowDomainEntry> end-to-end
  - CR-02  # fixed in 05cd7580 — parse_allow_domain_arg gated on explicit http(s):// scheme
  - WR-01  # fixed in c4931750 — is_loopback_domain uses parsed IpAddr semantics
open:
  - WR-02  # nono why endpoint display vs proxy enforcement divergence (advisory)
  - WR-03  # profile+CLI same-host entries not merged at runtime (advisory)
  - IN-01  # merge_allow_domain does not dedup duplicate endpoint rules
  - IN-02  # schema lacks additionalProperties:false
  - IN-03  # redundant/behavior-changing domain lowercasing before check_host
---

> **Resolution (2026-06-05):** Both blockers (CR-01 fail-open, CR-02 regression)
> and WR-01 (TLS-downgrade footgun) were fixed during execute-phase before
> verification — commits `05cd7580` (CR-01 + CR-02) and `c4931750` (WR-01), with
> regression tests added. `cargo test -p nono-cli` → 1190 passed (only the 4
> known pre-existing baseline failures remain). The remaining advisory warnings
> (WR-02, WR-03) and info items (IN-01..03) are left open as non-blocking
> follow-ups.

# Phase 56: Code Review Report

**Reviewed:** 2026-06-05
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Phase 56 adds an `AllowDomainEntry` enum (`#[serde(untagged)]` for backward
compat), wires per-endpoint method+path restrictions through the proxy, profile
display/schema/manifest, and `nono why --host`. The `#[serde(untagged)]`
backward-compat boundary, the proxy partition logic, and the `nono why` display
approximation are all implemented correctly and well-tested at their own
boundaries.

However, the data-flow wiring between the **profile layer** and the
**proxy/enforcement layer** contains a fail-open security defect: profile-defined
endpoint restrictions are silently dropped before they reach the proxy, because
`PreparedSandbox.allow_domain` was left as `Vec<String>` and the structured
endpoints are flattened away and then lossily re-parsed. A second, related bug
mangles `host:port` allow-domain entries into bogus endpoint routes. Both are
correctness/security blockers because they change network enforcement silently.

## Critical Issues

### CR-01: Profile-defined `allow_domain` endpoint rules are silently dropped (fail-open)

**File:** `crates/nono-cli/src/sandbox_prepare.rs:508` (with `crates/nono-cli/src/proxy_runtime.rs:144-149`)

**Issue:**
The profile loads `allow_domain` as `Vec<AllowDomainEntry>` (including
`WithEndpoints { domain, endpoints }`), but `PreparedSandbox.allow_domain`
(`sandbox_prepare.rs:72`) is still typed `Vec<String>`. At line 508 the
structured entries are flattened with `.map(|e| e.domain().to_string())`,
**discarding all endpoint rules**. Downstream in
`resolve_effective_proxy_settings` (`proxy_runtime.rs:144-149`) those bare
domain strings are re-parsed via `parse_allow_domain_arg`, which can only ever
reconstruct a single wildcard-method rule from a URL path — it can never recover
the original multi-rule endpoint set, and for a bare domain string it produces
`Plain(domain)`.

Net effect: a profile entry such as
```json
{"domain":"api.github.com","endpoints":[
  {"method":"GET","path":"/repos/**"},
  {"method":"POST","path":"/issues/**"}]}
```
is reduced to `Plain("api.github.com")` before it reaches
`build_proxy_config_from_flags` / `partition_allow_domain`. The domain is then
placed in `plain_hosts` and granted an **unrestricted CONNECT tunnel** — the
exact opposite of the intended restriction. This violates the project's
"Fail Secure" principle: a misconfiguration in the wiring widens access instead
of denying it. The same flattening also means the persisted `domain_endpoints`
state file (derived from `flags.proxy.allow_domain` in
`execution_runtime.rs:163-191`) is empty for profile-driven restrictions, so
`nono why --host` reports the domain as fully open as well.

This path is not covered by the phase's validation matrix (56-VALIDATION.md only
exercises CLI `--allow-domain` and direct proxy `RouteConfig` tests), so the
regression is invisible to the existing test suite.

**Fix:** Carry the structured entries end-to-end instead of flattening. Change
`PreparedSandbox.allow_domain` to `Vec<AllowDomainEntry>`:
```rust
// sandbox_prepare.rs
pub(crate) allow_domain: Vec<crate::profile::AllowDomainEntry>,
...
// line 508 — stop flattening:
allow_domain: profile_allow_domain,
```
Then in `resolve_effective_proxy_settings` (`proxy_runtime.rs:144`) clone the
prepared entries directly rather than re-parsing strings:
```rust
let mut allow_domain: Vec<AllowDomainEntry> = prepared.allow_domain.clone();
allow_domain.extend(args.allow_proxy.iter().map(|s| parse_allow_domain_arg(s)));
```
Keep the existing `.domain().to_string()` projections only where a flat string
list is genuinely needed (e.g. `print_allow_domain_port_warnings`,
`PreparedProfile`/`why` display). Add an integration test that loads a profile
with multi-rule endpoints and asserts the resulting `ProxyConfig.routes`
contains the endpoint route (and that `domain_endpoints` in the state file is
populated).

### CR-02: `host:port` allow-domain entries are mangled into bogus endpoint routes

**File:** `crates/nono-cli/src/proxy_runtime.rs:31-49`

**Issue:**
`parse_allow_domain_arg` calls `url::Url::parse(input)` on every entry. A bare
`host:port` string — an explicitly supported input shape (see
`collect_allow_domain_port_warnings`, which warns on but accepts `:port`
suffixes) — parses as a non-special URL where the host segment is treated as the
**scheme**. For `"api.openai.com:8080"`, `host_str()` returns `None` (so
`domain = unwrap_or(input) = "api.openai.com:8080"`) and `path()` returns
`"8080"`. Since that path is non-empty and `!= "/"`, the function emits:
```rust
WithEndpoints { domain: "api.openai.com:8080",
                endpoints: [EndpointRule { method: "*", path: "8080" }] }
```
This is doubly wrong: (1) the domain now carries a `:8080` suffix that will not
match real CONNECT host filtering, and (2) a nonsense endpoint rule with path
`8080` is created, turning a previously-working plain host filter into a broken
endpoint route. Before Phase 56 the same entry flowed through
`expand_proxy_allow` and filtered correctly on the hostname. This is a silent
behavioral regression that can either over-block (entry no longer matches) or
mis-scope traffic.

**Fix:** Only treat the input as a URL when it has an explicit, recognized
scheme; otherwise treat it as a bare host (optionally stripping a numeric
`:port` to mirror `collect_allow_domain_port_warnings`). For example:
```rust
fn parse_allow_domain_arg(input: &str) -> AllowDomainEntry {
    let looks_like_url = input.starts_with("http://") || input.starts_with("https://");
    if looks_like_url {
        if let Ok(parsed) = url::Url::parse(input) {
            if let Some(host) = parsed.host_str() {
                let path = parsed.path();
                return if path.is_empty() || path == "/" {
                    AllowDomainEntry::Plain(host.to_string())
                } else {
                    AllowDomainEntry::WithEndpoints {
                        domain: host.to_string(),
                        endpoints: vec![nono_proxy::config::EndpointRule {
                            method: "*".to_string(),
                            path: path.to_string(),
                        }],
                    }
                };
            }
        }
    }
    AllowDomainEntry::Plain(input.to_string())
}
```
Add a regression test: `parse_allow_domain_arg("api.openai.com:8080")` must
yield `Plain` (with the host, or host:port preserved as today's plain filter
expects), never `WithEndpoints`.

## Warnings

### WR-01: `is_loopback_domain` uses a string prefix that misclassifies real domains

**File:** `crates/nono-cli/src/network_policy.rs:339-345`

**Issue:**
`domain.starts_with("127.")` will classify a public hostname such as
`127.example.com` (or any domain whose first label begins with `127.`) as
loopback, causing `partition_allow_domain` to emit an `http://` upstream instead
of `https://` for it. This is a string-prefix comparison on host data, which the
project's CLAUDE.md explicitly flags as a footgun. While the user must list the
domain explicitly, a silent TLS downgrade (https→http) for a non-loopback host
weakens transport security. Note also that `0.0.0.0` is the unspecified/all-
interfaces address, not strictly loopback.

**Fix:** Match loopback via parsed IP semantics and exact host equality rather
than a textual prefix:
```rust
fn is_loopback_domain(domain: &str) -> bool {
    if domain == "localhost" { return true; }
    if let Ok(ip) = domain.parse::<std::net::IpAddr>() {
        return ip.is_loopback() || ip.is_unspecified();
    }
    false
}
```
`Ipv4Addr::is_loopback()` covers the entire `127.0.0.0/8` block correctly and
will not match `127.example.com`.

### WR-02: `nono why` endpoint display can diverge from proxy enforcement (false reassurance)

**File:** `crates/nono-cli/src/query_ext.rs:336-373`

**Issue:**
`path_matches_endpoint_rules`/`normalize_path` re-implement the proxy's matching
ad hoc (per-call `Glob::new(...).compile_matcher()`), and the doc comment
admits it "diverges from `CompiledEndpointRules::is_allowed` in edge cases ...
diagnostic only, not access control." However the display also ignores the
rule's `method` entirely — it matches on path alone, whereas the proxy requires
`(method == "*" || method.eq_ignore_ascii_case(req_method))` AND path match. A
user running `nono why --host https://api.github.com/repos/x` will be told the
request is ALLOWED even when every matching rule is method-scoped to a method
the request would not use. Because `nono why` is the primary operator tool for
reasoning about what the sandbox permits, a display that over-reports "allowed"
relative to actual enforcement is a security-usability hazard (operators may
grant trust they do not actually have).

**Fix:** Either (a) reuse `nono_proxy::config::CompiledEndpointRules` /
`endpoint_allowed` so the diagnostic cannot drift from enforcement, or (b)
clearly surface in the output that the match is path-only and method
restrictions still apply (and accept/parse a `--method` for `why` to evaluate
the method dimension). Option (a) is strongly preferred to eliminate drift.

### WR-03: profile + CLI `allow_domain` for the same host is not merged at runtime

**File:** `crates/nono-cli/src/proxy_runtime.rs:144-149` and `crates/nono-cli/src/network_policy.rs:358-405`

**Issue:**
`merge_allow_domain` correctly unions endpoint rules during profile inheritance,
but `resolve_effective_proxy_settings` simply concatenates the (lossy) profile
list and the CLI list without merging. `partition_allow_domain` then processes a
`Plain("github.com")` and a `WithEndpoints{github.com,...}` independently: the
`Plain` pushes `github.com` into `plain_hosts` while the `WithEndpoints` creates
an endpoint route. On the fork, `server.rs` blocks CONNECT to any route upstream
(403), so HTTPS traffic is in practice forced through L7 — but the resulting
state is contradictory (host is simultaneously in the plain allowlist and an
endpoint route), depends on a non-obvious cross-crate invariant for its
fail-secure behavior, and the `plain_hosts` membership is what makes the
intent ambiguous. The 56-RESEARCH.md even reasons (incorrectly for the fork)
that "the CONNECT path wins for unrestricted access," showing how easy it is to
mis-reason about this state.

**Fix:** Run the concatenated runtime list through `merge_allow_domain` (or an
equivalent domain-keyed dedup) before `partition_allow_domain`, so a given host
resolves to exactly one disposition (plain XOR endpoint-scoped). This makes the
fail-secure outcome explicit instead of relying on the downstream CONNECT block.

## Info

### IN-01: `merge_allow_domain` does not deduplicate identical endpoint rules

**File:** `crates/nono-cli/src/profile/mod.rs:3197-3227`

**Issue:** `rules.entry(domain).or_default().extend_from_slice(endpoints)` unions
without deduping, so the same `{method, path}` appearing in both base and child
profiles is stored twice. Harmless to matching, but bloats the rule list and the
`nono why` "N endpoint rules" count.

**Fix:** Dedup after collecting, e.g. retain only the first occurrence of each
`(method, path)` pair (`EndpointRule` is `PartialEq`).

### IN-02: schema permits endpoint objects with no `endpoints` and no `additionalProperties` guard

**File:** `crates/nono-cli/data/nono-profile.schema.json:672-688`

**Issue:** `AllowDomainWithEndpoints` requires only `domain`; `endpoints` is
optional and the endpoint object does not set `additionalProperties: false`.
Typos in property names (e.g. `"paths"` instead of `"path"`) would validate, and
a `WithEndpoints` with omitted/empty `endpoints` silently degrades to a plain
allow. This is acceptable for backward compat but worth tightening to catch
profile authoring mistakes.

**Fix:** Add `"additionalProperties": false` to the endpoint item object and
consider requiring a non-empty `endpoints` array if the structured form is used
intentionally for scoping.

### IN-03: lowercasing the domain before `filter.check_host` is redundant or subtly behavior-changing

**File:** `crates/nono-cli/src/query_ext.rs:251` and `285`

**Issue:** `parse_host_input` lowercases the domain, and that lowercased value is
passed to `filter.check_host(&domain, &[])`. If `HostFilter` already normalizes
case this is dead normalization; if it does not, this changes pre-existing
`nono why --host UPPER.example.com` matching semantics relative to the prior
code path that passed `host` verbatim. Confirm `HostFilter` case-handling and
either rely on it or document the intentional normalization.

**Fix:** Verify `HostFilter::check_host` case behavior; if it lowercases
internally, drop the redundant lowercasing for clarity; otherwise add a test
pinning the intended `nono why` case-insensitivity.

---

_Reviewed: 2026-06-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
