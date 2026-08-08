---
phase: 115-v3-6-carry-forward-drain
reviewed: 2026-08-08T22:30:46Z
depth: deep
files_reviewed: 17
files_reviewed_list:
  - crates/nono/src/undo/types.rs
  - crates/nono-proxy/src/audit.rs
  - crates/nono-proxy/src/connect.rs
  - crates/nono-proxy/src/external.rs
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/src/server.rs
  - crates/nono-proxy/tests/spiffe_integration.rs
  - crates/nono-cli/src/profile/mod.rs
  - crates/nono-cli/src/network_policy.rs
  - crates/nono-cli/src/profile_cmd.rs
  - crates/nono-cli/src/proxy_runtime.rs
  - crates/nono-cli/data/nono-profile.schema.json
  - ../nono-py/src/proxy.rs
  - ../nono-py/src/undo.rs
  - ../nono-py/src/lib.rs
  - ../nono-py/python/nono_py/__init__.py
  - ../nono-py/python/nono_py/_nono_py.pyi
findings:
  critical: 1
  warning: 6
  info: 8
  total: 15
status: issues_found
---

# Phase 115: Code Review Report

**Reviewed:** 2026-08-08T22:30:46Z
**Depth:** deep
**Files Reviewed:** 17 (2 repos: `nono` @ `10a9b0e2..HEAD`, `../nono-py` @ `21ab6c2..HEAD`)
**Status:** issues_found

## Summary

The phase's five core changes were traced end-to-end. **The four highest-risk items
in the review brief all came back clean** — see "Verified Clean" below; the defects
found are elsewhere.

The one BLOCKER is on the surface the phase newly *opened*, not the surfaces it
fixed: `../nono-py`'s eight new PyO3 wrapper types perform **no** validation, while the
Rust-native construction path (`nono-cli`'s `validate_custom_credential`) validates the
identical fields. `SpiffeAuthConfig(inject_header=...)` reaches
`reverse.rs`'s `format!("{}: {}\r\n", header, …)` verbatim, so a CRLF-bearing header name
smuggles arbitrary headers into an upstream request that also carries a live JWT-SVID.
This is precisely the class the brief asked about: *"do any of them accept a value the
Rust side treats as more permissive than the Rust-native construction path would?"*

The second-most consequential finding is that the D-11 drift guard **does not guard what
its own doc comment claims it guards**. `assert_all_variants_covered` compile-forces a
match arm per variant; it does **not** force `ALL` to contain that variant, and the
companion test hardcodes `10`. Adding an 11th variant + its match arm compiles clean and
passes the test with `ALL` at 10 — silently under-covering the very nono-py round-trip
test D-12 built on top of `ALL`. The guard is one-directional, and the direction it misses
is the dangerous one.

Also: the D-13/D-14 rejections left ~55 lines of now-unreachable validation code behind
(including a tautological `if auth.client_assertion.is_none()` guard on a path where it is
always true), and the phase's "no denial site can be uncategorised" invariant is only true
of sites that call `log_denied` — five denial branches emit no audit event at all.

### Verified Clean (negative results — these looked suspicious and are not defects)

- **SPIFFE reorder (`reverse.rs:625-664`) does not narrow the deny surface.** Diffed
  against `10a9b0e2:crates/nono-proxy/src/reverse.rs`: the moved block is byte-identical in
  predicate (`ctx.filter.check_host(&upstream_host, upstream_port)`), the inputs
  (`route.upstream` + `upstream_path`) are not mutated between the old and new positions,
  and nothing between them can change `upstream_url`. `parse_upstream_url`'s `?` now fires
  earlier, which is also fail-closed. The structural regression test is genuinely
  load-bearing: `body.find("managed_auth.acquire(")` takes the *first* occurrence, so
  re-hoisting the mint above the check flips it.
- **All 27 production `log_denied` call sites carry a semantically correct category.**
  Each was read in situ, not just checked for presence. The three sites DRAIN-03 named
  (`connect.rs:81` → `HostDenied`, `external.rs:133` → `HostDenied`,
  `external.rs:198` → `ExternalProxyRejected`) match their enforcement points exactly.
  `server.rs:1427`'s `denial_category` variable is the Phase-114 CR-03 guard, unchanged
  and still correctly OR-ing three pre-evaluated predicates.
- **The aws_auth / plain-OAuth2 rejections are on every profile-load path.**
  `validate_profile_custom_credentials` runs in `parse_profile_file` (:3349),
  `parse_profile_bytes` (:3290), **and** `finalize_profile` (:3174, after
  `apply_platform_overrides`). Built-in profiles route through
  `policy::get_policy_profile` → `resolve_and_finalize_profile` → `finalize_profile`.
  `load_raw_profile_from_path` calls `parse_profile_file`. No bypass route exists.
  (The two duplicate `validate_against_schema` helpers the brief warned about are both
  inside `#[cfg(test)]` modules — see IN-06.)
- **`inject_mode`/`inject_header` consumer coverage is complete.** Grepped every read of
  those fields workspace-wide. Exactly one production `RouteConfig` construction consumes
  them (`network_policy.rs:234-238`) plus one export path (`profile_cmd.rs:3451-3460`) and
  one validation path (`profile/mod.rs:1295`, `:1486`); all three resolve identically
  (`unwrap_or_default()` / `unwrap_or_else(default_inject_header)`). `network_policy.rs:286`
  reads `CredentialDef` (built-in policy), a different type still holding `String` — not a
  missed site. There is **no** path where `None` reaches request time as `header`/
  `Authorization` for a route the author wrote as `url_path`/`query_param`.
- **Schema ↔ validator: no fail-open divergence.** `inject_mode`/`inject_header` accept
  `null` in both; `InjectMode`'s `$def` retains its own `"default": "header"`. Everywhere
  the schema is looser than the validator (`aws_auth` accepted structurally,
  `inject_header` unconstrained by pattern), the validator rejects — fail-closed direction.
  Nothing the schema rejects is accepted by the validator.
- **Codec is symmetric, and the `ALL`-driven test does cover the encoder's output.**
  `denial_category_to_string` (serde `to_value`) and `denial_category_from_string`
  (serde `from_value`) share one vocabulary by construction; the round-trip test drives
  encode→decode over `ALL`, and both `audit_event_to_py_dict` and `SessionMetadata::from_dict`
  call those exact functions. (The gap is in `ALL` itself — see WR-01.)
- **`InterceptHandshakeFailed` removal is safe.** `git grep` at `10a9b0e2` finds zero
  production constructors and zero occurrences of the wire string outside planning docs.
  `../nono-ts` has no denial-category codec at all (grepped) — no second drifted binding.
- **CRLF hazard in the new structural test is not real.** `.gitattributes` pins `*.rs` to
  `eol=lf`, and `reverse.rs` is confirmed LF-only, so `include_str!` + `"\n}\n"` cannot
  mis-bound the function. Verified the marker lands on line 868 (`handle_spiffe_route` ends
  there) and that all three searched tokens fall inside the body.
- **`EndpointPolicy` exposure is fail-closed.** `RustEndpointPolicyDefault::default()` is
  `Deny` (`config.rs:780-783`), so `nono_py.EndpointPolicy()` with no arguments denies
  everything; `Approve` also fails closed (no backend wired, `reverse.rs:184-207`).
- **Build/test state.** `cargo check --workspace --all-targets` clean;
  `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` clean;
  `-p nono-sandbox-proxy` 301 + 6 tests pass (including the new `d17_…` structural test);
  `-p nono-sandbox --lib undo::types` 14 pass; `../nono-py` `cargo test` 71 pass (including
  both new tests); `-p nono-sandbox-cli --bin nono` shows 1537 pass / 11 fail — all 11 are
  the known pre-existing Windows-host baseline (config/protected_paths/audit_session), none
  profile- or credential-related.

---

## Critical Issues

### CR-01: nono-py `SpiffeAuthConfig` accepts unvalidated `inject_header`/`credential_format` — upstream HTTP header injection

**File:** `C:\Users\OMack\nono-py\src\proxy.rs:112-136` (enforcement point:
`C:\Users\OMack\nono\crates\nono-proxy\src\reverse.rs:769-773`)

**Issue:** The new `SpiffeAuthConfig::new` PyO3 constructor writes `inject_header` and
`credential_format` straight into `RustSpiffeAuthConfig::Jwt` with no validation. The value
flows unchanged through `route.rs:210-222` → `SpiffeJwtSource::connect` (`spiffe.rs:55-60`)
→ `ManagedUpstreamAuth::SpiffeJwt` → `UpstreamAuthMaterial::BearerToken { header }` and is
finally emitted as raw wire bytes:

```rust
// crates/nono-proxy/src/reverse.rs:769-773
request.push_str(&format!(
    "{}: {}\r\n",
    header,
    credential_format.replace("{}", svid_token.as_str())
));
```

`nono-cli` validates this exact field — `profile/mod.rs:1244-1249` calls
`validate_header_name`, which rejects any non-RFC-7230-token character (including CR/LF)
— and `validate_header_mode` (`:1494-1505`) additionally rejects CRLF in the effective
credential format. **Neither `nono-proxy` nor `nono-py` performs either check**, so the
PyO3 path this phase newly opened is strictly more permissive than the Rust-native path.

Concrete failing input:

```python
import nono_py
route = nono_py.RouteConfig(
    prefix="svc",
    upstream="https://api.example.com",
    spiffe=nono_py.SpiffeAuthConfig(
        workload_api_socket="/run/spire/sockets/agent.sock",
        audience=["api.example.com"],
        inject_header="X-Svc\r\nX-Admin: true",
    ),
)
nono_py.start_proxy(nono_py.ProxyConfig(routes=[route], ...))
```

Every request on `/svc/...` reaches `https://api.example.com` as:

```
GET /… HTTP/1.1
Host: api.example.com
X-Svc
X-Admin: true: eyJhbGciOi…   <- attacker-chosen header, on a request bearing a live JWT-SVID
```

`credential_format="Bearer {}\r\nX-Admin: true"` produces the same result via the value
side. Impact scales with the upstream's trust in request headers (tenant/role/impersonation
headers are the common case), and the injected header rides an *authenticated* request,
so it is a privilege-escalation primitive against the upstream, not merely a malformed
request. Header-splitting also enables request smuggling against intermediaries.

Trust-model caveat, stated honestly: the value is embedder-supplied, not
sandboxed-agent-supplied. That makes it a misconfiguration hazard in the narrow case where
the embedder hardcodes the route. It becomes directly attacker-reachable in the realistic
SDK pattern where a Python embedder builds `RouteConfig`s from a tenant-, file-, or
API-supplied service descriptor. Given CLAUDE.md's "Escape and validate all data" and
"Fail Secure" mandates, and given that the sibling construction path already rejects this,
the asymmetry should not ship.

**Fix:** Validate in the PyO3 constructor, mirroring `validate_header_name` /
`validate_header_mode`. Add to `../nono-py/src/proxy.rs`:

```rust
fn is_http_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c)
}

#[pymethods]
impl SpiffeAuthConfig {
    #[new]
    #[pyo3(signature = (workload_api_socket, audience, inject_header = None,
                        credential_format = None, svid_hint = None))]
    fn new(
        workload_api_socket: String,
        audience: Vec<String>,
        inject_header: Option<String>,
        credential_format: Option<String>,
        svid_hint: Option<String>,
    ) -> PyResult<Self> {
        let inject_header = inject_header.unwrap_or_else(|| "Authorization".to_string());
        if inject_header.is_empty() || !inject_header.chars().all(is_http_token_char) {
            return Err(PyValueError::new_err(format!(
                "inject_header '{inject_header}' is not a valid HTTP token \
                 (alphanumeric and !#$%&'*+-.^_`|~)"
            )));
        }
        if let Some(fmt) = credential_format.as_deref() {
            if fmt.contains('\r') || fmt.contains('\n') {
                return Err(PyValueError::new_err(
                    "credential_format must not contain CR or LF (header injection)",
                ));
            }
        }
        if workload_api_socket.is_empty() || audience.is_empty() {
            return Err(PyValueError::new_err(
                "workload_api_socket and audience must be non-empty",
            ));
        }
        Ok(Self { inner: RustSpiffeAuthConfig::Jwt { /* … */ } })
    }
}
```

Stronger, and the durable fix: move the check into `nono-proxy` itself (a
`RouteConfig::validate()` invoked from `RouteStore::load`), so *every* embedder — Python,
TypeScript, or Rust — inherits it and the CLI's copy becomes defense-in-depth rather than
the only gate. `../nono-py/python/nono_py/_nono_py.pyi` must then document that
`SpiffeAuthConfig.__init__` raises `ValueError`.

---

## Warnings

### WR-01: `NetworkAuditDenialCategory::ALL` is not compile-guarded against drift, contrary to its own doc comment

**File:** `crates/nono/src/undo/types.rs:333-344` (const), `:347-377` (guard), `:649-662` (test)

**Issue:** The doc comment at `:352-355` states:

> *"This is `ALL`'s own compile-time drift guard: forgetting to keep this match (and `ALL`,
> alongside it) in sync with the enum is caught at build time, not left for a runtime test
> to discover."*

The parenthetical is false. `assert_all_variants_covered` is an exhaustive `match` over
`&NetworkAuditDenialCategory`; adding a variant forces a **match arm** (E0004). Nothing
ties `ALL`'s contents or length to the enum's variant count. The companion test asserts
`ALL.len() == 10` against a **hardcoded literal**.

Concrete drift sequence — all steps compile and all tests pass:

1. Add `NetworkAuditDenialCategory::ApprovalBackendUnavailable` to the enum (`:259-289`).
2. `cargo build` fails with E0004 → developer adds
   `| NetworkAuditDenialCategory::ApprovalBackendUnavailable` to the guard arm at `:375`.
3. Build is green. `ALL` still lists 10 entries; `assert_eq!(ALL.len(), 10)` still passes.
4. `../nono-py`'s `denial_category_round_trips_every_core_variant` iterates `ALL` and
   therefore **never exercises the new variant** — the exact under-coverage DRAIN-02/NEW-06
   exist to close, silently reintroduced.

The guard only catches the *safe* direction (variant added to `ALL` but the literal not
bumped → test fails). The dangerous direction is unguarded. Note that D-11 explicitly
rejected `strum::EnumIter` on the grounds that the fallback "achieves the identical
'fails to compile on a missing variant' guarantee" — it does not.

**Fix:** Derive `ALL` from the same token list that declares the enum, so drift is
structurally impossible with zero new dependencies:

```rust
macro_rules! denial_categories {
    ($( $(#[$m:meta])* $variant:ident ),+ $(,)?) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum NetworkAuditDenialCategory { $( $(#[$m])* $variant ),+ }

        impl NetworkAuditDenialCategory {
            /// Every variant, in declaration order. Generated from the same
            /// list that declares the enum — cannot drift.
            pub const ALL: &'static [NetworkAuditDenialCategory] =
                &[ $( NetworkAuditDenialCategory::$variant ),+ ];
        }
    };
}

denial_categories! {
    AuthenticationFailed,
    EndpointPolicy,
    // …
    CaptureBufferOrRewriteFailed,
}
```

`assert_all_variants_covered` and the hardcoded-`10` assertion can then both be deleted.
If the macro is unwanted, at minimum replace `assert_eq!(ALL.len(), 10)` with a check that
does not depend on a hand-copied literal (e.g. assert every entry is distinct **and** that
`ALL.len()` equals a `const VARIANT_COUNT` declared immediately adjacent to the enum with a
comment binding the two) — and correct the doc comment either way.

### WR-02: D-13/D-14 left ~55 lines of unreachable validation code and a tautological guard

**File:** `crates/nono-cli/src/profile/mod.rs:1238-1240`, `:1373-1409`, `:1420-1458`

**Issue:** Two distinct dead regions, both created by this phase:

1. **`validate_aws_auth` is unreachable.** Its only call site is `:1239`
   (`if let Some(ref aws) = cred.aws_auth { validate_aws_auth(name, aws)?; }`), but
   `:1174` now returns `Err` unconditionally when `cred.aws_auth.is_some()`. The 39-line
   function at `:1420-1458` can never execute. `grep` confirms exactly two occurrences of
   the symbol workspace-wide (definition + that call), so no test covers it either.
   This is precisely what CLAUDE.md's "Lazy use of dead code — if code is unused, either
   remove it or write tests that use it" forbids.

2. **`validate_oauth2_auth`'s tail is unreachable, and its guard is tautological.** At
   `:1335-1359`, the `if let Some(ref assertion) = auth.client_assertion { … return Ok(()); }`
   block returns on every `Some`. Control therefore reaches `:1373` **only** when
   `client_assertion` is `None` — so `if auth.client_assertion.is_none()` is always true
   and the function always returns `Err` there. Everything below is dead:
   `client_id.is_empty()` (`:1383-1388`), `client_secret.is_empty()` (`:1390-1395`), and the
   CL-03-M literal-secret warning (`:1403-1409`). Confirmed by the phase's own test edits:
   `oauth2_empty_client_id_rejected` and `oauth2_empty_client_secret_rejected` were rewritten
   to assert the D-14 message, so those three blocks now have **zero** coverage.

   The tautological `if` is the more dangerous half: it reads like a conditional gate, so a
   future maintainer relaxing D-14 (e.g. once plain `client_credentials` is wired) will see
   "the emptiness checks are still there" and will not notice they have been untested and
   unexecuted for an entire release cycle.

**Fix:**

```rust
// :1238-1240 — delete; :1420-1458 — delete `validate_aws_auth` entirely.
// A future UPST absorb of real SigV4 re-adds it alongside un-rejecting :1174.
// Record the deletion in 108-DIVERGENCE-LEDGER.md next to the D-13 entry.

// :1373 — replace the tautological `if` with an unconditional return that
// states the reached-only-when-None precondition:
    // Control reaches here ONLY when `client_assertion` is None: the
    // `if let Some(..)` block above returns on every Some. D-14 (Phase 115,
    // DRAIN-04): plain OAuth2 client_credentials has no route-wiring in this
    // fork (credential.rs:289-380 registers it in none of the three maps) and
    // would forward with ZERO token injection.
    Err(NonoError::ProfileParse(format!(
        "auth for custom credential '{name}' declares plain OAuth2 \
         client_credentials (no client_assertion), which has no route-wiring \
         implementation in this fork and would silently forward requests with \
         no token injection; configure client_assertion (SPIFFE JWT) or remove \
         this credential"
    )))
// …and delete :1383-1411 (client_id / client_secret / CL-03-M warn).
```

Also update the now-stale doc comment on `CustomCredentialDef.auth` (`:968-971`, "client_id
and client_secret must not be empty") — see IN-02.

### WR-03: nono-py `CaptureConfig` bypasses every gate `validate_capture_config` enforces

**File:** `C:\Users\OMack\nono-py\src\proxy.rs:282-300`
(gate bypassed: `crates/nono-cli/src/profile/credential_provider.rs:40-99`)

**Issue:** The new `CaptureConfig::new` accepts `response_fields`, `request_nonce_fields`,
and `max_response_bytes` with no validation. `nono-cli` enforces four rules on the same
struct that the Python path now skips entirely:

| Rule | nono-cli | nono-py |
|---|---|---|
| `response_fields` non-empty | rejected (`:43-49`) | **default is `vec![]`** |
| `max_response_bytes != 0` | rejected (`:60-66`) | accepted |
| `max_response_bytes <= 1 MiB` (`CAPTURE_MAX_RESPONSE_BYTES_CEILING`, D-05) | rejected (`:67-73`) | accepted |
| field paths non-empty / no leading-trailing `.` / no `..` / no NUL | rejected (`:85-98`) | accepted |

The ceiling bypass is the material one. `config.rs:681-685` documents the 1 MiB ceiling as
a hard limit "enforced at profile-validation time", and `reverse.rs:1913-1915` consumes the
value with **no clamp**:

```rust
let cap = capture.max_response_bytes
    .unwrap_or(crate::config::DEFAULT_CAPTURE_MAX_RESPONSE_BYTES);
```

Concrete failing input:

```python
nono_py.CaptureConfig(
    response_fields=[nono_py.CaptureResponseField("access_token")],
    max_response_bytes=2**32,   # 4 GiB — nono-cli rejects anything > 1 MiB
)
```

`read_capped_response` will now buffer up to 4 GiB of a hostile upstream's response into
process memory before any rewrite is attempted — an explicit, documented security control
(D-05) defeated through a sibling API. `max_response_bytes=0` and empty `response_fields`
are less severe (both end up failing closed: a 0 cap denies everything, and empty
`response_fields` still trips `reject_unrewritten_token_fields` at `reverse.rs:2064` for
token-shaped fields — verified, no leak), but both produce silently non-functional routes
that `nono-cli` refuses to accept.

**Fix:** Validate in `CaptureConfig::new`, mirroring `validate_capture_config`:

```rust
#[new]
fn new(
    response_fields: Vec<CaptureResponseField>,
    request_nonce_fields: Vec<String>,
    max_response_bytes: Option<usize>,
) -> PyResult<Self> {
    if response_fields.is_empty() {
        return Err(PyValueError::new_err(
            "capture.response_fields must be non-empty — a capture config \
             declaring nothing to rewrite is not valid",
        ));
    }
    for f in &response_fields { validate_capture_path(&f.inner.path)?; }
    for p in &request_nonce_fields { validate_capture_path(p)?; }
    if let Some(n) = max_response_bytes {
        if n == 0 {
            return Err(PyValueError::new_err("capture.max_response_bytes must not be 0"));
        }
        if n > nono_proxy::config::CAPTURE_MAX_RESPONSE_BYTES_CEILING {
            return Err(PyValueError::new_err(format!(
                "capture.max_response_bytes ({n}) exceeds the hard ceiling of {} bytes",
                nono_proxy::config::CAPTURE_MAX_RESPONSE_BYTES_CEILING
            )));
        }
    }
    Ok(Self { inner: RustCaptureConfig { /* … */ } })
}
```

As with CR-01, the durable fix is to clamp/reject inside `nono-proxy` (either in
`RouteStore::load` or by clamping `cap` at `reverse.rs:1913` with
`.min(CAPTURE_MAX_RESPONSE_BYTES_CEILING)`), so no binding can outflank the ceiling.

### WR-04: `_nono_py.pyi` declares two `RouteConfig` parameters that do not exist, putting the three new parameters at the wrong positional indices

**File:** `C:\Users\OMack\nono-py\python\nono_py\_nono_py.pyi:558-577`, `:602-611`

**Issue:** The stub's `RouteConfig.__init__` lists `tls_client_cert` and `tls_client_key`
(positions 13 and 14) and exposes them as properties (`:603`, `:605`). Neither exists in
the Rust `#[pyo3(signature = …)]` (`../nono-py/src/proxy.rs:682-698`), and neither exists on
the fork's `RustRouteConfig` — `crates/nono-cli/src/network_policy.rs:487-489` documents them
as deliberately ABSENT ("Phase 34 fork-preserve decision"). `grep` confirms the only four
occurrences of those names anywhere in `../nono-py` are in the stub.

The mismatch pre-dates this phase, but the phase **appended** `spiffe`/`capture`/
`endpoint_policy` after them, so the stub now claims positions 15/16/17 while the Rust
binding has them at 13/14/15. `nono-py`'s own CLAUDE.md calls this file "the source of truth
for IDE autocompletion and mypy," so mypy will type-check code the runtime rejects.

Concrete failing input (mypy-clean, `TypeError` at runtime):

```python
RouteConfig("svc", "https://api.example.com", None, InjectMode.HEADER, "Authorization",
            None, None, None, None, None, [], "/ca.pem", None, None, my_spiffe_cfg)
# stub says arg 15 is `spiffe`; Rust binds arg 15 to `endpoint_policy`
# -> TypeError: argument 'endpoint_policy': 'SpiffeAuthConfig' cannot be converted to 'EndpointPolicy'
```

and `route.tls_client_cert` type-checks but raises `AttributeError` at runtime.

**Fix:** Delete lines 572-573 and 602-605 from the stub so the parameter order matches
`RouteConfig::new` exactly:

```python
        endpoint_rules: list[tuple[str, str]] = ...,
        tls_ca: str | None = None,
        spiffe: SpiffeAuthConfig | None = None,
        capture: CaptureConfig | None = None,
        endpoint_policy: EndpointPolicy | None = None,
    ) -> None: ...
```

Consider adding a stub-drift test (build the module, `inspect.signature` each `__init__`,
compare against the stub via `ast`) so this class of divergence fails CI rather than
compounding — this is the second phase in a row to append to a stub block that was already
wrong.

### WR-05: `nono profile diff` now reports a false "changed" for semantically identical credentials, and labels the effective default `<inherited>`

**File:** `crates/nono-cli/src/profile_cmd.rs:2398`, `:2445-2470`, `:2827-2845`

**Issue:** `CustomCredentialDef` derives `PartialEq`, and `cmd_diff` uses it directly
(`:2398`) to decide whether a credential changed. Before this phase, `inject_header` was
`#[serde(default = "default_inject_header")] String`, so a profile that **omitted** the key
and one that set `"Authorization"` deserialized to the same value and compared **equal**.
They are now `None` vs `Some("Authorization")` and compare **unequal**, even though both
place the credential in the identical header at request time.

Concrete failing input — `a.json` contains `"inject_header": "Authorization"`, `b.json`
omits the key; everything else identical. `nono profile diff a.json b.json` prints:

```
  Custom credentials:
    ~ svc (changed)
      - inject_header: Authorization
      + inject_header: <inherited>
```

Two defects in one output. (1) A false positive: nothing changed on the wire, but the
operator is told a credential-placement field did. (2) `<inherited>` is the wrong label —
at this point the profile is a concrete document, not an override block; `None` here means
"use the built-in default", and the effective value **is** `Authorization`. An operator
reading this cannot tell whether the header changed, and the natural reading ("it inherits
from a base, so it's unchanged") is wrong in the mirror case where the *old* side is
`Some("X-Api-Key")` and the new side is `None` — there the effective header really does
change to `Authorization`, and the diff hides it behind the same `<inherited>` token.

The `--json` path (`:2844`) has the same problem and additionally emits `null` where it
previously emitted a string, changing the machine-readable output shape.

**Fix:** Compare and render *effective* values, not raw `Option`s:

```rust
let old_header = old.inject_header.as_deref().unwrap_or("Authorization");
let new_header = new.inject_header.as_deref().unwrap_or("Authorization");
if old_header != new_header {
    // …render old_header / new_header…
}
```

Apply the same `unwrap_or_default()` normalisation to `inject_mode` at `:2445` and
`:2827`, and to the credential-level equality filter at `:2398` (compare a normalised
projection, or add a `fn effective(&self) -> …` helper on `CustomCredentialDef` reused by
`network_policy.rs`, `profile_cmd.rs`, and the validator so all four sites cannot drift).

### WR-06: five denial branches emit no audit event at all — DRAIN-03's "no uncategorised denial" invariant covers only sites that call `log_denied`

**File:** `crates/nono-proxy/src/reverse.rs:325`, `:391`, `:711`, `:995`;
`crates/nono-proxy/src/external.rs:125`

**Issue:** Making `category` a required `log_denied` parameter guarantees that any denial
which *calls* `log_denied` is categorised. It says nothing about denials that never call it.
Five branches deny a request and produce **zero** audit record:

- `reverse.rs:324-327` — `if aws_route.is_some() { send_error(stream, 501, …); return Ok(()); }`
- `reverse.rs:389-392`, `:709-712`, `:993-996` — `413 Payload Too Large` on the static-cred,
  SPIFFE-bearer, and SPIFFE-assertion paths
- `external.rs:125` — `validate_proxy_auth(remaining_header, ctx.session_token)?;` propagates
  the error with **no** audit entry *and* no `407` response to the client (the CONNECT and
  reverse paths both audit + respond in the same situation)

Concrete failing scenario: a sandboxed agent POSTs a body larger than `MAX_REQUEST_BODY` to
a credential route. The proxy denies with `413`. `nono audit` / `SessionMetadata.network_events`
contain nothing — the operator has no record that the agent's request was refused, no
`route_id`, and no denial category. The same agent can repeat this indefinitely with no
audit trail.

These sites are outside DRAIN-03's literally-named three, so this is scope commentary as
much as a defect — but the phase's summary language ("no denial site can be uncategorised")
overstates what the change achieves, and this is the review that should say so.

**Fix:** Add `log_denied` calls at each site with an appropriate category. The 413s want a
new `RequestBodyTooLarge` variant (or `EndpointPolicy` if a new variant is unwanted); the
`external.rs` auth failure wants `AuthenticationFailed` plus a `407` response mirroring
`connect.rs:56-70`. Example for `reverse.rs:389-392`:

```rust
if len > MAX_REQUEST_BODY {
    audit::log_denied(
        ctx.audit_log,
        audit::ProxyMode::Reverse,
        nono::undo::NetworkAuditDenialCategory::RequestBodyTooLarge,
        &audit::EventContext { route_id: Some(&service), ..Default::default() },
        &service,
        0,
        "request body exceeds MAX_REQUEST_BODY",
    );
    send_error(stream, 413, "Payload Too Large").await?;
    return Ok(());
}
```

The `reverse.rs:325` AWS 501 is now unreachable from the CLI (D-13 rejects `aws_auth` at
validation) and from `nono-py` (`aws_auth` hardcoded `None`); the cleanest resolution there
is to delete the branch and the `CredentialStore::aws_routes` placeholder map alongside
`validate_aws_auth` (WR-02), rather than audit-instrumenting dead code.

---

## Info

### IN-01: `denial_category` is absent from both tracing streams in `log_denied`

**File:** `crates/nono-proxy/src/audit.rs:207-232`
**Issue:** The category is written only into the in-memory `NetworkAuditEvent` (`:245`).
Neither the `target: "nono_proxy::audit"` `info!` nor the `target: "nono_security::network_deny"`
`warn!` (which feeds Windows ETW / Application-log dual-emit) carries it, so an operator
tailing logs — or an EDR consuming the ETW stream — still cannot distinguish a `HostDenied`
from a `ConnectBypassesL7` from a `CaptureUnsupportedPath`.
**Fix:** Add `denial_category = %format_args!("{category:?}")` (or a `&'static str`
accessor on the enum) to both macro invocations. The category name is drawn from a fixed
vocabulary and carries no request-derived bytes, so it does not violate the secret-hygiene
constraint documented at `:220-225`.

### IN-02: Rust doc comments on `CustomCredentialDef.aws_auth` and `.auth` are stale relative to D-13/D-14

**File:** `crates/nono-cli/src/profile/mod.rs:1043-1049`, `:960-973`
**Issue:** The JSON schema was carefully updated to say `aws_auth` is "REJECTED AT
VALIDATION TIME" and that plain `client_credentials` is rejected, but the Rust doc comments
on the same two fields still read "When present, the proxy will sign outbound requests with
AWS SigV4 credentials" and "`client_id` and `client_secret` must not be empty." A reader
working from `cargo doc` or the source gets the pre-115 contract for both.
**Fix:** Mirror the schema wording into both doc comments, citing D-13 / D-14.

### IN-03: `../nono-py/src/undo.rs` test claims to be "the crate's first Rust-level test"

**File:** `C:\Users\OMack\nono-py\src\undo.rs:952-953`
**Issue:** `cargo test` in `../nono-py` runs 71 tests; 69 pre-date this phase
(`override_mod::token`, `override_mod::verify`, `windows_confined_run::tests`, …). The
claim is factually wrong and, if believed, would mislead a future maintainer into thinking
the `[dev-dependencies] pyo3 = { features = ["auto-initialize"] }` block was added here.
**Fix:** Delete the sentence, or replace with "the first Rust-level test in this module."

### IN-04: `profile diff` renders `Option<InjectMode>` via `{:?}`

**File:** `crates/nono-cli/src/profile_cmd.rs:2447-2455`
**Issue:** Output changed from `Header` to `Some(Header)` / `None`. Cosmetic, but it leaks
a Rust type into operator-facing output. Subsumed by WR-05's fix if the effective value is
rendered instead.

### IN-05: an explicit `"inject_mode": null` in a `platform_overrides` block now means "inherit", with no way to reset to the default

**File:** `crates/nono-cli/src/profile/mod.rs:3668-3669`;
`crates/nono-cli/data/nono-profile.schema.json` (`inject_mode`/`inject_header` now accept `null`)
**Issue:** `.or(base)` cannot distinguish "key absent" from "key present with value null",
and the schema now explicitly permits the latter. An author who writes
`"inject_header": null` in an override intending "go back to the default `Authorization`"
gets the base's `X-Api-Key` instead. This is the correct trade-off given D-05's
"silence means inherit" rule, but it is an expressibility gap the schema now advertises.
**Fix:** Document it in the two schema `description` strings ("`null` is treated identically
to omission — it inherits; to force the default, state the value explicitly"), or reject
explicit `null` at the schema level with `{"$ref": …}` only (no `null` branch) plus
`#[serde(default)]` on the Rust side.

### IN-06: duplicate `validate_against_schema` test helpers

**File:** `crates/nono-cli/src/profile/mod.rs:7584`, `:9958`
**Issue:** Two independent definitions of the same helper in two `#[cfg(test)]` modules
within one file. Test-only (so not a production fail-open, which is what the review brief
flagged this class for), but they can drift — one could be updated for a schema change and
the other not, silently weakening half the schema test suite.
**Fix:** Hoist to a single `#[cfg(test)] mod schema_test_util` and `use super::…` from both.

### IN-07: the new structural regression test asserts textual position, not reachability

**File:** `crates/nono-proxy/tests/spiffe_integration.rs:130-176`
**Issue:** The test byte-compares source offsets. Wrapping the host-check block in
`if false { … }`, or making it `continue`-free but unreachable, would still pass. This is
the explicit consequence of the locked option-(c) decision (recorded in the test's own
comment) and it is a reasonable trade given the SPIRE-agent dependency of the alternatives
— noted only so the limitation is on record, not as a defect.
**Fix:** None required. If a stronger guarantee is wanted later, extract the host-check
into a small `async fn check_upstream_allowed(...) -> Result<Option<DenyReason>>` and unit-test
that the SPIFFE path calls it before constructing any `ManagedUpstreamAuth` material.

### IN-08: the D-16 allowlist test's rationale cites the wrong enforcement layer

**File:** `C:\Users\OMack\nono-py\src\proxy.rs:686-692`, `:697`
**Issue:** The comment justifies excluding `oauth2`/`aws_auth` from the Python constructor
because they are "rejected at nono-cli validation time (D-13/D-14, Plan 115-03)". `nono-cli`
validation does not run on the `nono-py` path at all — a Python-constructed `RouteConfig`
goes straight to `nono_proxy::RouteStore::load`. The *conclusion* (do not expose an
unimplemented mechanism) is right; the stated reason is not, and it implies a protection
that does not exist on this path. Worth correcting because the same muddle is what would
lead a future maintainer to expose `oauth2` here believing the CLI validator has their back.
**Fix:** Reword to the actual reason: "`aws_auth` 501s unconditionally at request time
(`reverse.rs:324-327`) and plain `oauth2` is registered in none of `CredentialStore`'s three
maps (`credential.rs:289-380`), so it would forward with zero token injection. Neither is
gated on this path by any validator — exposing them would hand embedders config for a
mechanism that cannot work."

---

_Reviewed: 2026-08-08T22:30:46Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
