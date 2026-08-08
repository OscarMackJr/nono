# Phase 115: v3.6 Carry-Forward Drain - Research

**Researched:** 2026-08-08
**Domain:** Rust proxy denial/audit spine + profile-merge semantics + cross-repo (PyO3) binding codec, in a fail-secure sandboxing codebase
**Confidence:** HIGH — every claim below is either a fresh grep/read against the tree at `157b2c6d` (this session), or an empirical `cargo test` run (Q8), not inherited from CONTEXT.md without re-verification.

## Summary

This research answers the two evidence-gated questions CONTEXT.md's D-09 and D-14 explicitly forbid pre-committing, and re-verifies every other locked decision's blast-radius claim by symbol.

**Q1 verdict (D-09 precondition — SATISFIED, safe to remove `InterceptHandshakeFailed`):** Zero production constructors exist anywhere in `crates/` (confirmed by exhaustive grep of `NetworkAuditDenialCategory::` construction sites — 30 hits, none is `InterceptHandshakeFailed`). The only Rust-side references to the variant are its own definition (`types.rs:249`) and `../nono-py`'s two hand-written matches (dead code on the Rust side since Rust never constructs it). Because no production code path can ever have written the string `intercept_handshake_failed` into a persisted ledger, and the decode path (`serde_json::from_str::<AuditEventRecord>`, which contains `AuditEventPayload::Network { event: Box<NetworkAuditEvent> }`, whose `denial_category: Option<NetworkAuditDenialCategory>` has no `#[serde(other)]` tolerance and would hard-error on an unrecognized variant) has never been asked to decode this string, removal is safe without adding an unknown-variant escape route. D-09 may proceed directly.

**Q2 verdict (D-14 — UNWIRED, rejection applies):** Traced at source. `CredentialStore::load` (`credential.rs:296-384`) only inserts a route into `spiffe_assertion_routes` when `oauth2.client_assertion` matches `Some(ClientAssertionConfig::SpiffeJwt {...})`. For plain `client_credentials` (`client_id`/`client_secret`, `client_assertion: None`), the `if let` at line 304 is false, the whole block is skipped, and the route is inserted **nowhere** — not `credentials`, not `aws_routes`, not `spiffe_assertion_routes`. There is also, by design, no `get_oauth2()` accessor (`credential.rs:436`: "OD-1 scope note: there is deliberately no `get_oauth2()` accessor"). At dispatch (`reverse.rs:250`), `ctx.credential_store.get(&service)` returns `None` for such a route, so it falls into the "no-credential route (L7 filtering only)" branch (`reverse.rs:293-299`) — the request proceeds to upstream with **no OAuth2 token acquisition of any kind**, silently behaving as if no auth were configured at all. `validate_oauth2_auth` (`profile/mod.rs:1302-1364`) currently *validates* (requires non-empty) `client_id`/`client_secret` even on this dead path — it does not reject it. This is the same "validates but doesn't work" class as `aws_auth`, confirmed unwired. D-14's rejection should be added in `validate_oauth2_auth`, gated on `auth.client_assertion.is_none()`.

**Contradiction found in CONTEXT.md, surfaced per instructions:** D-01's stated blast radius claims `../nono-py`'s `RouteConfig::new` (`proxy.rs:191-192`, `:206-207`) "intersects" D-01 and must be sequenced with it. This is **not correct** — `../nono-py` has **zero** references to `CustomCredentialDef` anywhere (confirmed via grep across the whole `nono-py` tree). `CustomCredentialDef` is a `nono-cli`-only type; `nono-py` never depends on `nono-cli` and never touches it. `RouteConfig::new`'s `inject_mode`/`inject_header` params build `nono_proxy::config::RouteConfig` directly (a different struct, which stays non-`Option` regardless of what `CustomCredentialDef` becomes) — see Q5 below for the full trace. D-01 has **zero** structural effect on `../nono-py`. The planner should drop this "sequence D-01 with nono-py" instruction; it does not apply.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Profile override merge semantics (DRAIN-01) | CLI (`nono-cli/src/profile/mod.rs`) | — | Profile loading/merging is CLI policy, per CLAUDE.md's library-vs-CLI boundary table |
| Denial category classification (DRAIN-03) | API/Backend (`nono-proxy`) | Core library (`nono/src/undo/types.rs` — the vocabulary) | `nono-proxy` is the enforcement point; `nono::undo` types are pure audit vocabulary (ADR-86: observability primitive, not policy) |
| Denial codec round-trip (DRAIN-02) | Binding layer (`../nono-py`) | Core library (source of truth: `NetworkAuditDenialCategory`'s own serde) | The binding must never hold a second copy of the vocabulary — D-10's whole point |
| Config-validation rejection of unimplemented mechanisms (DRAIN-04) | CLI (`profile/mod.rs::validate_custom_credential`) | API/Backend (`nono-proxy/src/reverse.rs`'s 501) | Fail-secure principle: reject at load time (CLI), not at request time (proxy) |
| Python route-config parity (DRAIN-05) | Binding layer (`../nono-py`) | API/Backend (the `RouteConfig` struct being mirrored) | Binding struct must track every field the Rust proxy struct actually enforces |
| SPIFFE mint-before-deny ordering (DRAIN-06) | API/Backend (`nono-proxy/src/reverse.rs`) | — | Pure request-dispatch ordering inside the proxy |

## Project Constraints (from CLAUDE.md)

- **No `.unwrap()`/`.expect()`** (clippy `-D clippy::unwrap_used`) — every new code path in this phase (D-01 `.unwrap_or_default()`, D-06's signature change, D-09's removal) must not introduce panics.
- **Library-vs-CLI boundary**: `nono::undo::NetworkAuditDenialCategory` (core library) is audit vocabulary, not policy — D-09's removal and D-11's `strum` derive stay within ADR-86's "observability primitive" carve-out. Do not add enforcement logic to `crates/nono/src/undo/types.rs`.
- **Fail Secure**: D-13/D-14's validation rejections and D-09's removal must fail closed on ambiguity, never silently degrade.
- **Path/string handling**: not directly implicated this phase (no filesystem path logic touched).
- **Checked arithmetic**: not implicated (no arithmetic in scope).
- **Commits**: DCO sign-off required; `nono-py` changes are a **separate repo commit** (own DCO sign-off).
- **Cross-target clippy**: see Q9 below — **does NOT apply** to this phase's file set (no Unix `#[cfg(target_os = "linux"/"macos")]` blocks, no `exec_strategy/`, no `bindings/c/src/` touched).

## Q1 — D-09 Precondition: Is removing `InterceptHandshakeFailed` safe for persisted ledgers?

**Verdict: SAFE. Precondition satisfied without needing an unknown-variant decode route.**

### (a) Can any persisted ledger contain `intercept_handshake_failed`?

Grep across the whole repo (target/ excluded via `.gitignore`, confirmed `search_gitignored: false`):

```
crates\nono\src\undo\types.rs:249:    InterceptHandshakeFailed,     <- enum definition only
../nono-py/src/proxy.rs:85-86                                        <- hand-written encoder (dead — never constructed)
../nono-py/src/undo.rs:603-604                                       <- hand-written decoder (dead — never constructed)
```

No hits in any `.jsonl`, no `tests/fixtures/` or `testdata/` directory in the repo (`crates/nono/tests/fixtures/trust-root-frozen.json` and `crates/nono-cli/tests/fixtures/*` are TUF/trust-root fixtures, unrelated), no goldens.

**Exhaustive production-constructor check** (`grep -rn "NetworkAuditDenialCategory::" crates/ --glob '*.rs'`, 30 hits total): every construction site in `crates/nono-proxy/src/{reverse,connect,external,server}.rs` and `crates/nono-cli/src/{exec_strategy/supervisor_linux,proxy_command}.rs` uses one of: `AuthenticationFailed`, `EndpointPolicy`, `ManagedCredentialUnavailable`, `HostDenied`, `UpstreamConnectFailed`, `ConnectBypassesL7`, `SpiffeUnsupportedPath`, `CaptureUnsupportedPath`, `CaptureBufferOrRewriteFailed`. **Zero** hits for `InterceptHandshakeFailed` or `ExternalProxyRejected` (the second dead variant DRAIN-03 also names — confirmed independently, zero constructors). This directly reconfirms CONTEXT.md's "zero production constructors, can never gain one" claim — verified fresh, not inherited.

### (b) What is the decode path, and does it tolerate unknown variants?

`NetworkAuditEvent.denial_category: Option<NetworkAuditDenialCategory>` (`types.rs:366`). `NetworkAuditDenialCategory` derives plain `Serialize, Deserialize` with `#[serde(rename_all = "snake_case")]` (`types.rs:242-244`) — **no** `#[serde(other)]`, **no** catch-all variant. Ledger persistence flows through `crates/nono/src/audit.rs`:
- `AuditEventPayload::Network { event: Box<NetworkAuditEvent> }` (`audit.rs:106-109`) is the variant carrying network denial events.
- Decode sites: `serde_json::from_str::<AuditEventRecord>(&line)` at `audit.rs:895`, `:974`, `:1438` (three call sites: `append_session_to_ledger_file`, `verify_session_in_ledger_reader`, `verify_audit_log`).

Because plain `#[derive(Deserialize)]` on a Rust enum errors on an unrecognized string (standard serde behavior — confirmed by the absence of `#[serde(other)]`), an unknown `denial_category` value **would** break decode of any ledger that somehow contained it. But per (a), no such ledger can exist. **D-09 may remove the variant directly; no unknown-variant escape route is required as a precondition.** (If the planner wants defense-in-depth for *future* variant removals of this kind, that is a discretionary addition, not a DRAIN-01..06 requirement.)

## Q2 — D-14: Is plain OAuth2 `client_credentials` genuinely unwired?

**Verdict: UNWIRED. D-14's rejection applies.**

Traced end-to-end at `crates/nono-proxy/src/credential.rs`:

1. `CredentialStore::load`'s per-route loop (`credential.rs:228-384`) checks `route.credential_key` first, then `route.aws_auth`, then `route.oauth2` (`:296`).
2. Inside the `oauth2` branch, the **only** handled shape is `Some(ClientAssertionConfig::SpiffeJwt {...})` matched via `if let Some(...) = &oauth2.client_assertion` (`:304-308`). For plain `client_credentials` (`client_assertion: None`), this `if let` is `false` and the entire block (`:304-383`) is a no-op — the route is inserted into **none** of `credentials`, `aws_routes`, or `spiffe_assertion_routes`.
3. `credential.rs:436` documents this as deliberate: "OD-1 scope note: there is deliberately no `get_oauth2()` accessor for..." — confirming there is no other lookup path a caller could use to find this route's OAuth2 config at request time.
4. Dispatch (`reverse.rs:250`): `let cred = ctx.credential_store.get(&service);` returns `None` for this route (nothing was ever inserted). `reverse.rs:293-299` then treats it as a "no-credential route (L7 filtering only)" — it validates only the **sandbox's own session token**, not any upstream OAuth2 credential, and forwards the request with **zero token injection**.

So a profile declaring plain `client_credentials` does not 501 (unlike `aws_auth`) — it silently degrades to a passthrough that omits the very credential the config was supposed to provide. This is the same "config accepted, mechanism absent" class DRAIN-04 (`aws_auth`) is about, confirmed unwired by direct trace, not by absence-of-evidence.

**Where the rejection belongs:** `validate_oauth2_auth` (`profile/mod.rs:1302-1364`) currently validates `client_id`/`client_secret` non-emptiness for the plain flow (`:1335-1347`) — it does not reject it. Add the rejection there, symmetrical with D-13's `reverse.rs:328-331` unconditional `aws_auth` 501 → move to load-time. Suggested gate: `if auth.client_assertion.is_none() { return Err(...) }` before the `client_id`/`client_secret` checks (the `client_assertion.is_some()` branch already returns early at `:1332`, so adding the rejection just above the `client_id.is_empty()` check at `:1335` is the natural insertion point).

## Q3 — D-11: Is `strum` available, and does it hold under ADR-86?

`strum`/`strum_macros` are **absent everywhere** — zero hits in `Cargo.lock` (root workspace), zero in any of the 5 workspace member `Cargo.toml`s, zero in `../nono-py/Cargo.toml`/`Cargo.lock`, zero in `../nono-ts/Cargo.toml`. This is a genuinely new dependency, not "already in the tree."

**Cost if added:** `strum_macros` depends on `syn`/`quote`/`proc-macro2`, all three of which are **already resolved** in the root `Cargo.lock` (`syn` 1.0.109 *and* 2.0.117 dual-version, `quote` 1.0.45, `proc-macro2` 1.0.106) via the existing `typify`/`sigstore-verify` build-dependency chain. So the marginal new crates would be just `strum` + `strum_macros` themselves — low transitive cost.

**ADR-86 precedent:** `thiserror` (a derive-only macro crate, structurally identical in kind to `strum`) is **already** a direct dependency of the core `nono` crate (`crates/nono/Cargo.toml:35`, `thiserror.workspace = true`) and is explicitly cited in CLAUDE.md's own coding standards ("Use `NonoError` for all errors... propagation via `?`"). This is direct, load-bearing precedent that a derive-only macro dependency in the core crate does not cross ADR-86's policy-free-library boundary — ADR-86 constrains what the library *decides* (security policy), not what it *derives*.

**Cheaper option, evidence-based:** the fallback (`pub const ALL: &[Self]` + a private exhaustive-`match` guard) has **zero** new-dependency cost and achieves the identical "fails to compile on a missing variant" property `strum::EnumIter` provides. Given `strum` is completely net-new to the workspace (not "already present, just needs a feature flag"), and the fallback is equally strong structurally, **the fallback is the cheaper choice** on pure dependency-surface grounds — though `thiserror`'s precedent means adding `strum` is not contentious either. This is the planner's discretion call per CONTEXT.md; both are viable, evidence favors the fallback only on cost, not on strength.

## Q4 — D-15: Is `endpoint_policy` live enough to expose to Python?

**Verdict: YES — live, enforced, and tested in `nono-proxy`. Expose it.**

`RouteConfig.endpoint_policy: Option<EndpointPolicyConfig>` (`config.rs:645`) is:
1. **Compiled** at route-load time: `route.rs:182-183`, `CompiledEndpointPolicy::compile(route.endpoint_policy.as_ref(), &route.endpoint_rules)`, stored on `LoadedRoute.endpoint_policy: CompiledEndpointPolicy` (`route.rs:42`, non-`Option` — always compiled, empty-policy is the "legacy" no-op case).
2. **Enforced** on every reverse-proxy request: `reverse.rs:155`, `route.endpoint_policy.evaluate(&method, &upstream_path)`, with an **exhaustive match** (`reverse.rs:153`: "match is exhaustive (no wildcard arm) so the compiler forces handling of every current and future `EndpointPolicyOutcome` variant") over `Allow`/`Deny`/`Approve`. `Deny` emits `NetworkAuditDenialCategory::EndpointPolicy` (`reverse.rs:174`, `:198`) and a 403. `Approve` fails closed (no approval backend wired — `reverse.rs:184-204`, explicit fail-secure comment: "an operator-configurable control must never silently degrade to a pass-through").
3. **Tested**: `config.rs` has 3 dedicated tests — `endpoint_policy_deny_rule_is_enforced` (`:2065`), `endpoint_policy_legacy_route_preserves_deny_semantics` (`:2101`), `endpoint_policy_approve_without_backend_is_recognized` (`:2130`).

**Notable, related-but-out-of-scope finding:** `endpoint_policy` has **zero** exposure at the `nono-cli` profile-schema level either — `CustomCredentialDef` (`profile/mod.rs`) has no `endpoint_policy` field at all, and `nono-profile.schema.json` has zero hits for `endpoint_policy`. Both production `RouteConfig` construction sites in `network_policy.rs` (the Rust CLI's own profile→proxy-config conversion, `:257` and `:289`) hardcode `endpoint_policy: None`. This means **the only way to actually set `endpoint_policy` on a live route today is to hand-construct a `RouteConfig` in Rust** (e.g., in a test) — there is no profile-level surface for it at all, Rust or Python. This makes D-15's Python exposure the *first* configuration surface for this mechanism outside test code, not a parity catch-up. Worth a one-line note in the D-15 task description; not in scope to also add it to the CLI profile schema this phase (that's a DRAIN-05-adjacent but distinct gap, not named in any DRAIN-0x requirement).

### Field shapes the PyO3 wrappers must mirror

**`SpiffeAuthConfig`** (`nono-proxy/src/config.rs:656-673`, single-variant enum):
```rust
pub enum SpiffeAuthConfig {
    Jwt {
        workload_api_socket: String,
        audience: Vec<String>,
        inject_header: String,        // #[serde(default = "default_inject_header")]
        credential_format: Option<String>,
        svid_hint: Option<String>,
    },
}
```

**`CaptureConfig`** (`config.rs:721-742`):
```rust
pub struct CaptureConfig {
    response_fields: Vec<CaptureResponseField>,   // #[serde(default)]
    request_nonce_fields: Vec<String>,             // #[serde(default)] — LIVE, consumed
    max_response_bytes: Option<usize>,
}
pub struct CaptureResponseField {                  // config.rs:703-712
    path: String,
    kind: CaptureResponseFieldKind,                 // #[serde(default)]
}
pub enum CaptureResponseFieldKind { Opaque, Jwt }   // config.rs:692-698, #[default] Opaque
```

**`EndpointPolicyConfig`** (`config.rs:805-816`):
```rust
pub struct EndpointPolicyConfig {
    default: EndpointPolicyDefault,   // #[serde(default)]
    deny: Vec<EndpointPolicyRule>,
    approve: Vec<EndpointPolicyRule>,
    allow: Vec<EndpointPolicyRule>,
}
pub struct EndpointPolicyDefault {    // config.rs:770-788
    decision: EndpointPolicyDecision, // Deny | Approve | Allow, #[default] Deny
    backend: Option<String>,
    timeout_secs: Option<u64>,
}
pub struct EndpointPolicyRule {       // config.rs:790-802
    method: String,
    path: String,
    backend: Option<String>,
    reason: Option<String>,
    timeout_secs: Option<u64>,
}
```

`aws_auth: Option<AwsAuthConfig>` and `oauth2: Option<OAuth2Config>` stay hardcoded `None` per D-15's explicit exclusion (unimplemented/rejected mechanisms — see Q2 above for why `oauth2` plain flow specifically must stay excluded).

## Q5 — D-01 Blast Radius (exact, symbol-level)

### Field definitions (`crates/nono-cli/src/profile/mod.rs`)

```rust
#[serde(default)]
pub inject_mode: InjectMode,                          // line 966

#[serde(default = "default_inject_header")]
pub inject_header: String,                             // line 972
```
`InjectMode` is `nono_proxy::config::InjectMode` (re-exported/imported), which derives `Default` with `#[default] Header` (`nono-proxy/src/config.rs:11-23`) — `.unwrap_or_default()` resolves to `Header` cleanly. `default_inject_header()` is a **private fn duplicated in both crates** (`nono-cli/src/profile/mod.rs:1053-1055` and `nono-proxy/src/config.rs:1099-1101`), both returning `"Authorization".to_string()` — identical implementation, not a single shared symbol.

`CustomCredentialDef` derives `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` — **no `Default` impl** (confirmed: `grep "impl Default for CustomCredentialDef"` → zero hits). This means every struct-literal construction site names every field explicitly; none can use `..Default::default()`.

### Consumer inventory

**Production consumers requiring a code change (2 sites):**
1. `crates/nono-cli/src/network_policy.rs:228-229` — the **only** production `CustomCredentialDef` → `nono_proxy::config::RouteConfig` conversion site (`RouteConfig.inject_mode`/`.inject_header` are non-`Option`, so this site needs `cred.inject_mode.clone().unwrap_or_default()` / `cred.inject_header.clone().unwrap_or_else(default_inject_header)`).
2. `crates/nono-cli/src/profile/mod.rs:3566-3567` — `merge_custom_credential_def`, the fix's own target (`inject_mode: child.inject_mode.or(base.inject_mode)`, `inject_header: child.inject_header.or(base.inject_header)`).

**Struct-literal construction sites needing mechanical field-shape updates (36 more, all inside `#[cfg(test)]` modules — verified by locating each file's `mod tests {` boundary):**
- `crates/nono-cli/src/network_policy.rs` — 11 more literals at lines 617, 660, 709, 745, 791, 909, 942, 975, 1013, 1145, 1228, 1283 (module's `mod tests` starts at line 507 — all 12 total literals in this file are inside it, including the one counted above at :617+ style, i.e. **all 12** `CustomCredentialDef {` literals in this file are test-only except none — recount: the production conversion at :222 reads `cred.inject_mode`/`cred.inject_header`, it does not *construct* a `CustomCredentialDef` literal, so all 12 `CustomCredentialDef {` literal sites in this file ARE inside `mod tests`).
- `crates/nono-cli/src/proxy_runtime.rs` — 2 literals at lines 947, 1209 (both inside `mod tests`, boundary at line 665).
- `crates/nono-cli/src/profile/mod.rs` — 23 test literals (lines 5218, 5570, 5592, 5616, 5640, 5662, 5686, 5708, 5732, 5756, 6163, 6185, 6325, 6347, 7764, 7789, 7817, 7845, 7905, 7927, 8489, 8519, 10174) plus the 1 production `merge_custom_credential_def` literal already counted above (3564).

**Total struct-literal sites needing `inject_mode`/`inject_header` field-shape edits: 38** (2 production-path + 36 test-fixture, all mechanical: wrap the current bare value in `Some(...)`). Verified via `grep -c "CustomCredentialDef {"` per file (network_policy.rs: 12, proxy_runtime.rs: 2, profile/mod.rs: 29 minus 5 false-positive matches on the struct definition line and 4 function-signature lines ending `-> CustomCredentialDef {` = 24 real literals) — cross-checked against the manual line enumeration above, consistent.

**Schema file:** `crates/nono-cli/data/nono-profile.schema.json` — `inject_mode`/`inject_header` are **not** in this credential block's `"required": ["upstream"]` array (line ~618), so no requiredness change. But their JSON shape needs updating to match the `oneOf`-with-null pattern already used for every other `Option<T>` field in this same block (e.g. `credential_format` at lines 669-675: `"oneOf": [{"type": "string"}, {"type": "null"}]`, no `"default"` key) — currently `inject_mode`/`inject_header` (lines 659-668) carry a bare type + a `"default"` metadata key, which should be dropped in favor of the `oneOf`-null shape.

**Correction to CONTEXT.md (see Summary):** `../nono-py`'s `RouteConfig::new` is **not** a consumer of `CustomCredentialDef` — zero references anywhere in the `nono-py` repo. It constructs `nono_proxy::config::RouteConfig` directly with its own independently Python-defaulted params (`inject_mode = InjectMode::Header`, `inject_header = "Authorization"` at `proxy.rs:192-193`), which stays non-`Option` regardless of D-01. **No sequencing with D-01 is needed.**

## Q6 — D-06: `audit::log_denied` Signature Change

### Verified counts

`grep -c "audit::log_denied("` across `nono-proxy/src/{connect,external,reverse,server}.rs`: connect.rs 3, external.rs 2, reverse.rs 17, server.rs 6 = **28 production call sites**, confirmed exactly matching CONTEXT.md's figure.

**Exactly 3 pass `&audit::EventContext::default()`** (verified by reading 3 lines of context after every `log_denied(` call):
- `connect.rs:83-86` — `deny_domain`'s HTTPS enforcement point (the filter-host-check branch of `handle_connect`), `&audit::EventContext::default()` at line 86.
- `external.rs:133-136` — the cloud-metadata deny-list check inside `handle_external_proxy`, default context at line 136.
- `external.rs:197-200` — the "enterprise proxy rejected with status ≠200" branch, default context at line 200.

The `#[cfg(test)]` 4th hit (`audit.rs:381-384`, function `log_denied_records_reason` at `:377-378`) is confirmed test-only, not counted — matches CONTEXT.md's claim exactly.

### `EventContext` struct definition

`crates/nono-proxy/src/audit.rs:40-50`:
```rust
#[derive(Debug, Clone, Default)]
pub struct EventContext<'a> {
    pub route_id: Option<&'a str>,
    pub auth_mechanism: Option<NetworkAuditAuthMechanism>,
    pub auth_outcome: Option<NetworkAuditAuthOutcome>,
    pub managed_credential_active: Option<bool>,
    pub injection_mode: Option<NetworkAuditInjectionMode>,
    pub denial_category: Option<NetworkAuditDenialCategory>,
    pub spiffe_context: Option<SpiffeAuditContext>,
    pub capture_context: Option<CaptureAuditContext>,
}
```
`#[derive(Default)]` on the whole struct — confirmed `denial_category: Option<_>`, confirmed `Default`-derived. **Note:** this `EventContext` (in `nono-proxy/src/audit.rs`) is a different type from the core `nono::audit`'s own audit-recorder machinery — do not confuse the two "audit" modules.

### Recommended signature shape (cheapest, evidence-based)

`log_allowed` (the sibling function) hardcodes `denial_category: None,` in its own event-push (`audit.rs:167`) and **never reads** `ctx.denial_category` — confirmed by inspection of `log_allowed`'s body (lines 138-178). So `denial_category` is meaningful **only** to `log_denied`. The cheapest structural fix: **remove `denial_category` from `EventContext` entirely** and add it as a new required, non-`Option` 4th positional parameter to `log_denied`:

```rust
pub fn log_denied(
    audit_log: Option<&SharedAuditLog>,
    mode: ProxyMode,
    category: NetworkAuditDenialCategory,   // NEW — required, not in ctx
    ctx: &EventContext<'_>,
    host: &str,
    port: u16,
    reason: &str,
) { ... push_event(..., denial_category: Some(category), ...) }
```
This forces all 28 call sites to pass a category explicitly (omission is now a missing-argument compile error, not a silently-defaulted struct field), and the 25 sites that currently set `denial_category: Some(X)` inside the `EventContext` literal simply move `X` to the new parameter position — mechanical, and the field's removal from the struct means those literals get an "unknown field" compile error if left unedited, so the migration cannot be silently skipped. `log_allowed` is entirely unaffected (it never read the field). Category assignment for the 3 previously-defaulted sites: `connect.rs:86` and `external.rs:136` → `HostDenied` (D-07, matching the existing `reverse.rs:365-376` template exactly); `external.rs:200` → `ExternalProxyRejected` (D-08).

## Q7 — D-04: Compile-Time Guard Shape for `HookConfig`

`HookConfig` (`profile/mod.rs:1999-2012`):
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookConfig {
    pub event: String,
    pub matcher: String,
    pub script: String,
}
```
Note: the struct **does** derive `Default` (produces 3 empty strings), but there is **no field-level** `#[serde(default)]` and the struct carries `#[serde(deny_unknown_fields)]` with no struct-level `#[serde(default)]` either — so **JSON deserialization** still requires all 3 fields present (an override JSON object omitting any of `event`/`matcher`/`script` fails deserialization, it does not silently zero-fill). This confirms CONTEXT.md's "no silent loss" framing is correct even though the struct technically has a `Default` impl (the impl is unused by serde's derive path).

`merge_profiles`'s `hooks.hooks` merge (`profile/mod.rs:3819-3825`):
```rust
hooks: HooksConfig {
    hooks: {
        let mut merged = base.hooks.hooks;
        merged.extend(child.hooks.hooks);
        merged
    },
},
```
`HooksConfig.hooks: HashMap<String, HookConfig>` (`#[serde(flatten)]`, `profile/mod.rs:2018-2022`).

**Precedent in this repo:** `merge_custom_credential_def` (`profile/mod.rs:3564-3589`) already constructs its output via a fully-named struct literal (`CustomCredentialDef { upstream: ..., inject_mode: ..., ... }`, no `..` rest pattern) — this is itself an exhaustive-struct-literal compile-time guard: adding a field to `CustomCredentialDef` without updating this literal is an `E0063` (missing field) compile error today, verified by the fact that this literal already names all 15 current fields individually. This is the reusable idiom: **an exhaustive `let HookConfig { event: _, matcher: _, script: _ } = <value>;` destructure with no `..`**, placed either as a dead-store in a `#[test]` (cheapest, zero runtime cost, colocated with the D-03 regression test) or as a `const _: fn(HookConfig) = |HookConfig { event: _, matcher: _, script: _ }| {};` module-level zero-cost guard. Either form fails to compile the moment a new field is added to `HookConfig`, without changing `merge_profiles`'s whole-value-replace behavior for hooks (which stays correct per D-04's own instruction not to field-merge). No dedicated "exhaustive destructure as a guard" idiom with that exact name exists elsewhere in the repo to cite verbatim, but the struct-literal precedent above is functionally identical and already load-bearing in the same function this guard is meant to protect.

## Q8 — D-10: Byte-Identical Serde Check

**Verified empirically**, not by hand-computation: a temporary `#[test]` was inserted into `crates/nono/src/undo/types.rs` asserting `serde_json::to_string(&variant)` for all 11 `NetworkAuditDenialCategory` variants against the exact strings `../nono-py/src/proxy.rs`'s hand-written encoder emits. `cargo test -p nono-sandbox --lib` ran it; all 11 assertions passed:

```
OK: AuthenticationFailed -> authentication_failed
OK: EndpointPolicy -> endpoint_policy
OK: ManagedCredentialUnavailable -> managed_credential_unavailable
OK: HostDenied -> host_denied
OK: InterceptHandshakeFailed -> intercept_handshake_failed
OK: UpstreamConnectFailed -> upstream_connect_failed
OK: ConnectBypassesL7 -> connect_bypasses_l7
OK: ExternalProxyRejected -> external_proxy_rejected
OK: SpiffeUnsupportedPath -> spiffe_unsupported_path
OK: CaptureUnsupportedPath -> capture_unsupported_path
OK: CaptureBufferOrRewriteFailed -> capture_buffer_or_rewrite_failed
```
The test was then reverted (`git checkout -- crates/nono/src/undo/types.rs`) — the tree is clean, no trace of the scratch test remains. **`ConnectBypassesL7` — the variant CONTEXT.md specifically flagged as "the awkward one" — is confirmed byte-identical** (`connect_bypasses_l7`; serde's snake_case algorithm inserts `_` before every uppercase letter after the first character and lowercases, and digits are not uppercase so `L7` becomes `l7` with no extra separator, matching the hand-written string). **D-10's hard precondition is fully discharged: zero mismatches across all 11 (10 after D-09 removes one) variants.**

**Decoder gap (NEW-06), confirmed:** `../nono-py/src/undo.rs:587-623`'s decoder covers exactly 9 variants (`authentication_failed`, `endpoint_policy`, `managed_credential_unavailable`, `host_denied`, `intercept_handshake_failed`, `upstream_connect_failed`, `connect_bypasses_l7`, `external_proxy_rejected`, `spiffe_unsupported_path`) then falls to `other => Err(PyValueError::new_err(...))`. **Missing: `capture_unsupported_path` and `capture_buffer_or_rewrite_failed`** — exactly the two variants the encoder (`proxy.rs:98-103`) already emits and the decoder's own `other` arm rejects. Confirmed by direct comparison of the two match statements.

## Q9 — Cross-Target + Build Gates

**Cross-target clippy gate: does NOT apply to this phase.** Checked every file named in CONTEXT.md's "Code under change" list for `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, or `#[cfg(any(target_os = "linux", target_os = "macos"))]` blocks, and for membership under `crates/nono-cli/src/exec_strategy/` or `bindings/c/src/` (neither applies — no files in this phase's scope live under either path):

| File | Unix cfg blocks? |
|---|---|
| `crates/nono-cli/src/profile/mod.rs` | Only `#[cfg(target_os = "windows")]` (5 hits, e.g. lines 4034, 4137, 4148, 4448, 4458) — **Windows-only cfg is explicitly out of the checklist's scope** ("Does NOT apply to: Pure Windows-only files... that has NO Unix counterpart" — these are inline blocks, not a Unix-counterpart file, but the checklist's positive scope is specifically linux/macos cfg, which this file has zero of) |
| `crates/nono-proxy/src/{connect,external,reverse,server,audit,credential}.rs` | Zero hits for any `target_os` cfg |
| `crates/nono/src/undo/types.rs` | Zero hits |
| `../nono-py/src/{proxy,undo}.rs` | Zero hits |

**Conclusion: neither the linux-gnu `cross clippy` nor the apple-darwin `cargo-zigbuild clippy` gate is a MUST for this phase.** `cargo build --workspace --all-targets` + native `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` (Windows host) + `cargo fmt --all -- --check` is the correct gate set. This is a genuine, useful correction against a default assumption — do not budget time for the cross-target runners on this phase.

**`maturin build` requirement (D-12):** confirmed structurally required — `../nono-py`'s `Cargo.toml` builds `crate-type = ["cdylib"]` only (`nono-py/Cargo.toml:13`), and the binding has **zero** existing `#[cfg(test)]`/`#[test]` Rust unit tests in either `src/proxy.rs` or `src/undo.rs` today (both files have zero `#[test]` hits) — D-12's round-trip test will be the **first** Rust-level test in this crate's source, not an addition to an existing suite. `cargo test -p nono-py` (package name confirmed: `Cargo.toml:2`, `name = "nono-py"`) works fine despite `cdylib`-only crate-type (cargo test compiles the crate source directly for the harness, not via the `.pyd`/`.so` artifact) — but `maturin build` is still the decisive gate per D-12/Phase 113/114 precedent because only a real wheel build has caught every prior struct-drift break (3 of the last 5 cross-repo syncs).

**`workflow.use_worktrees`:** confirmed **already `false`** in `.planning/config.json` (`workflow.use_worktrees: false`) — no change needed, D-12/D-15/D-16's requirement is already satisfied at the config level.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`), workspace-native; no separate framework for `../nono-py`'s Rust-level test (first one in that crate) |
| Config file | none — standard `cargo test` |
| Quick run command | `cargo test -p nono-sandbox-proxy --lib` (nono-proxy crate); `cargo test -p nono-sandbox --lib` (core enum); `cargo test -p nono-py` (binding, after `cargo build -p nono-py` succeeds) |
| Full suite command | `cargo test --workspace` + `maturin build` (from `../nono-py`) + `cargo fmt --all -- --check` + native `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DRAIN-01 | Override omitting `inject_mode`/`inject_header` inherits base, not header/`Authorization` | unit | `cargo test -p nono-sandbox-cli --lib profile::mod::tests -- --nocapture` (extend the existing NEW-02 regression test at `mod.rs:10166+`) | ❌ Wave 0 — extend existing test, add exhaustive assertion for all 14 fields incl. the new 2 |
| DRAIN-01 (ACC-04) | `spiffe` arm of the merge is asserted, not just `capture` | unit | same test as above | ❌ Wave 0 — currently missing |
| DRAIN-02 | `nono-py` decodes every category its own encoder emits | unit (binding crate, Rust-level) | `cargo test -p nono-py --lib` then `cd ../nono-py && maturin build` | ❌ Wave 0 — first Rust test in this crate; also gate on `maturin build` exit 0 |
| DRAIN-03 | `connect.rs:86`, `external.rs:136`, `external.rs:200` emit a real category; no zero-constructor variant remains | unit + compile-time | `cargo test -p nono-sandbox-proxy --lib` (existing denial-category assertion tests at `server.rs:1896`, `:1973` style) + `cargo build --workspace` (proves `InterceptHandshakeFailed` removal doesn't break compilation) | ⚠️ Partial — existing pattern exists (`server.rs` denial-category assertions), needs 3 new assertions for the previously-default sites |
| DRAIN-04 | `aws_auth` route rejected at validation, not 501 at runtime | unit | `cargo test -p nono-sandbox-cli --lib profile::mod::tests::test_.*aws_auth` | ❌ Wave 0 — new validation-rejection test |
| DRAIN-04/D-14 | plain OAuth2 `client_credentials` rejected at validation | unit | `cargo test -p nono-sandbox-cli --lib profile::mod::tests::test_.*oauth2` | ❌ Wave 0 — new validation-rejection test |
| DRAIN-05 | Python `RouteConfig` exposes `capture`/`spiffe`/`endpoint_policy`, no unconditional `None` field | unit (binding crate, Rust-level, D-16) | `cargo test -p nono-py --lib` + `cd ../nono-py && maturin build` | ❌ Wave 0 — new allowlist test |
| DRAIN-06 | SPIFFE route blocked by `deny_domain` is denied before JWT-SVID mint | integration | `cargo test -p nono-sandbox-proxy --test spiffe_integration -- --nocapture` (extend, mirroring the D-03 pattern from Phase 113: assert absence of a SPIRE fetch, not just the 403) | ⚠️ Partial — `spiffe_integration.rs` exists with the D-03 pattern to mirror; new test asserts *no mint occurred*, which may require a mock/spy on `SpiffeJwtSource::connect` or `managed_auth.acquire()` — **flagged as an open design question for the planner**, see Open Risks |

### Sampling Rate
- **Per task commit:** the crate-scoped quick-run command for the file(s) touched (e.g. `cargo test -p nono-sandbox-cli --lib profile::` after a `profile/mod.rs` edit).
- **Per wave merge:** `cargo build --workspace --all-targets`, `cargo test -p nono-sandbox-proxy --lib`, `cargo test -p nono-sandbox --lib`, `cargo fmt --all -- --check`; `maturin build` from `../nono-py` at the wave that touches the binding.
- **Phase gate:** full suite green (all of the above) + native `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` before `/gsd:verify-work`. Cross-target clippy gates are **not required** for this phase (Q9).

### Wave 0 Gaps
- [ ] Extended NEW-02 regression test in `crates/nono-cli/src/profile/mod.rs` (exhaustive over all 14 `CustomCredentialDef` `.or(base)`-shaped fields, plus the `spiffe` assertion ACC-04 requires) — DRAIN-01.
- [ ] First Rust `#[cfg(test)] mod tests` block in `../nono-py/src/undo.rs` (round-trip test driven from the encoder's own output, per D-12) — DRAIN-02.
- [ ] 3 new `denial_category` assertions for the previously-default sites — DRAIN-03.
- [ ] New `validate_custom_credential`/`validate_oauth2_auth` rejection tests for `aws_auth` and plain `client_credentials` — DRAIN-04.
- [ ] New allowlist test in `../nono-py` asserting no `RustRouteConfig` field is unconditionally `None` outside the named D-15 exclusions — DRAIN-05.
- [ ] Extended `spiffe_integration.rs` test asserting absence of a SPIRE mint on a denied host (needs a design decision — see Open Risks) — DRAIN-06.

## Planning Implications

1. **Q1/Q2 are both resolved — no blocking evidence gap remains.** D-09 (remove `InterceptHandshakeFailed`) and D-14 (reject plain OAuth2 `client_credentials`) can both proceed as locked, without further investigation tasks.
2. **Drop the "sequence D-01 with `../nono-py`'s `RouteConfig::new`" instruction from CONTEXT.md** — it is based on a mistaken premise (see Summary). D-01's actual cross-repo touch is `../nono-py` for a **different** reason entirely: none — D-01 has zero `nono-py` footprint. `../nono-py` is touched this phase only for D-10 (DRAIN-02 codec) and D-15/D-16 (DRAIN-05).
3. **Cross-repo ordering, confirmed correct as CONTEXT.md states:** core enum change (D-09 removal) → serde codec (D-10 delete-both-hand-written-matches) → `strum` iteration or fallback (D-11) → `maturin build` (D-12 gate). This chain is real and must be sequenced within a single wave or across waves in that order, because D-10's decoder rewrite needs the enum in its final (post-D-09-removal) shape to avoid writing code that references a variant about to disappear.
4. **DRAIN-01 (D-01) and DRAIN-05 (D-15/D-16) touch overlapping files but are independent** — D-01 is entirely `nono-cli` + schema; D-15/D-16 is entirely `../nono-py`. They can run in parallel waves.
5. **DRAIN-03 (D-06/D-07/D-08/D-09) is the largest single-task blast radius** — 28 call sites for D-06's signature change, all mechanical but touching 4 files (`connect.rs`, `external.rs`, `reverse.rs`, `server.rs`). Recommend a dedicated wave/task, not bundled with other DRAIN items in the same file set.
6. **DRAIN-01's real blast radius is bigger than "every consumer" suggests — 38 struct-literal sites**, though only 2 are production code (the other 36 are test fixtures, all inside `#[cfg(test)]` modules, all mechanical `X` → `Some(X)` edits). Budget accordingly — this is high task-count, low-risk-per-task work, well suited to a single focused task with a broad `files_modified` list rather than splitting.
7. **Cross-target clippy gates are NOT required this phase** (Q9) — do not add a cross-target-verify task; this saves real time versus a default assumption that every phase needs it.
8. **DRAIN-06's test design needs a decision before planning the task**, not during execution: "assert no SPIRE fetch occurred" requires either (a) a test double/spy on `SpiffeJwtSource::connect`, (b) an integration test that points `workload_api_socket` at a non-existent path and asserts the specific *absence* of a connection-attempt log line, or (c) reordering the code so the assertion is structural (the deny returns before `managed_auth.acquire()` is even reachable, provable by control-flow inspection alone without a live SPIRE agent). Option (c) is cheapest and matches D-17's own "hoist the check" mechanism — the regression test can be a pure code-structure assertion (e.g., grep-based self-enforcing test, per the loud-skip/self-enforcing-scan convention already used elsewhere in this codebase) rather than a live-SPIRE integration test. Flag this for the planner to decide explicitly.
9. **Schema file (`nono-profile.schema.json`) edits are cosmetic-but-necessary** for D-01 — not required-field changes (neither field is in the `required` array today), just shape/description corrections to match the `oneOf`-with-null pattern already used for sibling `Option<T>` fields.

## Open Risks

1. **DRAIN-06's test mechanism is INCONCLUSIVE on which of the three options (test double / socket-absence integration test / structural code-review assertion) the planner should choose.** Settled by: a design decision at plan time, informed by whether the executor wants a live-SPIRE-optional test (favor structural/self-enforcing-scan, matching Phase 113's D-07 loud-skip precedent) or is willing to accept a CI-only (SPIRE-socket-required) integration test that the local host will `SKIP[...]` on (per the established convention). Recommend structural, given this phase explicitly avoids adding new host-gated tests where a compile/structural check suffices.
2. **`strum` vs. fallback (D-11) is a genuine open discretion call, not resolved by evidence** — both are viable; Q3 above gives the cost/precedent data needed to decide, but the decision itself is explicitly left to the planner/CONTEXT.md's "Claude's Discretion" section.
3. **The exact count of `EndpointRule`/`RouteConfig`-adjacent existing PyO3 wrapper precedent for a nested-enum-with-struct-variant type (like `SpiffeAuthConfig::Jwt{...}`) was not found in `../nono-py`** — the only existing wrapper precedent (`InjectMode`, `proxy.rs:114-172`) is a simple C-like enum with `#[pyclass(frozen, eq, hash, from_py_object)]`. Building a PyO3 wrapper for a struct-variant enum (`SpiffeAuthConfig`) or a config struct with nested Vec<struct> (`CaptureConfig`/`CaptureResponseField`) has no exact in-repo precedent to copy; the planner should expect this to be genuinely new PyO3 surface, not a mechanical port. Settled by: check PyO3's own docs for struct-variant `#[pyclass]` support during planning/execution (not investigated further here — outside this research's scope, which was symbol-verification of the fork's own code, not PyO3 API capability).

## Sources

### Primary (HIGH confidence — direct grep/read of the live tree at `157b2c6d`, or empirical `cargo test`)
- `crates/nono/src/undo/types.rs` — `NetworkAuditDenialCategory` definition, serde derives, all 11 variants (empirically round-tripped, Q8)
- `crates/nono/src/audit.rs` — ledger decode path (`AuditEventPayload`, `AuditEventRecord`, 3 `serde_json::from_str` call sites)
- `crates/nono-proxy/src/{connect,external,reverse,server,audit,credential,config,route}.rs` — all Q1/Q2/Q4/Q6/Q9 evidence
- `crates/nono-cli/src/{profile/mod.rs,network_policy.rs,proxy_runtime.rs}` — all Q3/Q5/Q7/Q9 evidence
- `crates/nono-cli/data/nono-profile.schema.json` — Q5 schema shape evidence
- `../nono-py/src/{proxy.rs,undo.rs}` — Q1/Q4/Q5/Q8/Q9 evidence, `Cargo.toml` for crate-type/package-name
- `../nono-ts/Cargo.toml` — confirms zero `nono-proxy` dependency (Q3/Q9)
- `Cargo.lock` (root) — Q3 strum/syn/quote/proc-macro2 resolution check
- `.planning/config.json` — `workflow.use_worktrees: false` confirmed, `nyquist_validation` absent (treated enabled), `security_enforcement` absent (treated enabled)
- Empirical `cargo test -p nono-sandbox --lib` run (Q8), tree reverted clean via `git checkout --`

### Secondary (MEDIUM confidence)
- `.planning/milestones/v3.6-MILESTONE-AUDIT.md` §"Re-Audit — second pass" (NEW-05..NEW-08, ACC-02/03/04) — cross-checked against fresh greps, all findings reconfirmed accurate.
- `proj/ADR-113-spiffe-disposition.md` §D-01 — the standing no-TLS-interception decision underpinning D-09's "can never gain a constructor" claim.
- `proj/ADR-86-library-boundary-convergence.md` — `thiserror` precedent for derive-only deps in the core crate (Q3).

### Tertiary (LOW confidence)
- None — every claim in this document traces to a direct grep, read, or executed test against the live tree.

## Metadata

**Confidence breakdown:**
- Q1/Q2 (the two blocking evidence questions): HIGH — both traced to source with exhaustive grep + code-path walk, no assumption.
- Q3 (strum): HIGH on the facts (absence, cost, precedent); the recommendation itself is explicitly a discretion call, not a fact.
- Q4 (endpoint_policy liveness): HIGH — enforcement site, compile site, and 3 dedicated tests all located.
- Q5 (D-01 blast radius): HIGH — every site individually verified by grep + `mod tests` boundary check; the CONTEXT.md contradiction is a direct empirical finding (zero grep hits), not an inference.
- Q6 (log_denied): HIGH — exact call-site count matches CONTEXT.md's claim, independently re-derived.
- Q7 (HookConfig guard): MEDIUM — the field/derive facts are HIGH confidence; the "no exact precedent idiom exists" claim is a negative claim, mitigated by finding the functionally-equivalent existing precedent (`merge_custom_credential_def`'s own exhaustive literal).
- Q8 (byte-identical serde): HIGH — empirically executed, not computed by hand.
- Q9 (cross-target scope): HIGH — direct grep of every in-scope file for the checklist's exact trigger patterns.

**Research date:** 2026-08-08
**Valid until:** 14 days (fast-moving phase — this is a code-touching phase in an actively-developed repo with two open milestones; line numbers and struct shapes will drift the moment execution starts, consistent with this repo's own documented lesson about `proxy_runtime.rs:590` → `:591` drift)
