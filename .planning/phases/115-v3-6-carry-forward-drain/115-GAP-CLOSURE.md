---
phase: 115-v3-6-carry-forward-drain
type: gap-closure
closes: "115-VERIFICATION.md gap — SC5 (second half) / DRAIN-06 / finding V-01 (BLOCKER)"
requirements: [DRAIN-06]
files_modified:
  - crates/nono-proxy/src/reverse.rs
  - crates/nono-proxy/tests/spiffe_integration.rs
completed: 2026-08-09
---

# Phase 115 Gap Closure: DRAIN-06 at the class level (both SPIFFE dispatch paths)

**`handle_spiffe_assertion_credential`'s `deny_domain` host-check is hoisted above its RFC 7523
token exchange, and the D-17 regression test is rebuilt to discover its own subjects from source
so it guards the ordering class rather than one named function.**

`115-VERIFICATION.md` recorded `status: gaps_found` (4/5) because Plan 115-04 applied the D-17
hoist to only one of the two SPIFFE-backed auth paths `reverse.rs:210-216` documents. This
document records the closure of that gap.

---

## 1. The hoist

### 1a. Data-dependency check (done FIRST, as required)

The hoist is only legal if `check_host`'s inputs are derivable at the earlier position.
`check_host` needs `upstream_host`/`upstream_port`, which come from
`parse_upstream_url(&upstream_url)`, which needs `upstream_url`:

```rust
let upstream_url = format!(
    "{}{}",
    spiffe_assertion.upstream.trim_end_matches('/'),
    upstream_path
);
```

Both operands are **function parameters**, resolved before the function body does any work:

| Operand | Origin | Depends on the exchanged token? |
|---|---|---|
| `spiffe_assertion.upstream` | field of the `spiffe_assertion: &SpiffeAssertionRoute` parameter | No |
| `upstream_path` | `upstream_path: &str` parameter | No |
| `method` (used only by the `debug!` line moved with it) | `parse_request_line(first_line)?`, the function's first statement | No |

`access_token` (the value `get_or_refresh()` returns) is referenced **nowhere** in the moved
region. Unlike `handle_reverse_proxy`'s static-credential path — where `transform_path_for_mode`
consumes `cred.raw_credential` to build the path before the URL exists — this path applies **no
path transformation at all** (its own comment: "the exchanged access token is always
header-injected"). The reorder is therefore purely mechanical, exactly as it was for
`handle_spiffe_route`. **No forcing was required; nothing had to be restructured.**

The other direction was also checked: `check`'s later consumer (`check.resolved_addrs`, passed to
`connect_upstream_tls`) is unaffected — the binding is in scope for the whole function either way.

### 1b. What moved

Nothing was rewritten. The `access_token = match spiffe_assertion.cache.get_or_refresh().await`
block (with its `ManagedCredentialUnavailable` / 503 fail-closed arm) moved **down**, below the
already-existing `parse_upstream_url` + `check_host` + `HostDenied` block. The deny block itself
was **not edited at all** — only a comment was added above it.

Resulting order in `handle_spiffe_assertion_credential`, now identical in shape to
`handle_spiffe_route`:

1. `parse_request_line`
2. session-token auth gate (`validate_proxy_auth` → 407 + `AuthenticationFailed`)
3. `upstream_url` → `parse_upstream_url`
4. **`ctx.filter.check_host(...)` → `HostDenied` deny + 403 + `return`**
5. `spiffe_assertion.cache.get_or_refresh()` → `ManagedCredentialUnavailable` + 503 on failure
6. header filtering, body read, upstream TLS connect, forward

### 1c. Why it mattered

`SpiffeAssertionTokenCache::get_or_refresh` (`oauth2.rs:551`) → `exchange_jwt_assertion`
(`oauth2.rs:604`) → `jwt_source.fetch_token(audience)` (`oauth2.rs:614`) mints a JWT-SVID from the
SPIRE Workload API **and POSTs it to the IdP token endpoint**. Before this change, a
`deny_domain`-blocked upstream on an `auth.client_assertion = SpiffeJwt` route (a shape Plan
115-03's D-14 deliberately keeps valid) did both of those before the deny fired.

Consistent with the verification report: **this was never a leak** — the token never reached the
denied host — it was an ordering inversion and a needless external call. Note that
`get_or_refresh` short-circuits on a live cached token (`oauth2.rs:552-563`), so the mint only
occurred on cache miss/expiry; that bounded the frequency, not the defect.

---

## 2. The widened test

`d17_spiffe_host_check_precedes_managed_auth_acquire_structurally` was **replaced** by
`d17_spiffe_dispatch_host_check_precedes_mint_structurally` in
`crates/nono-proxy/tests/spiffe_integration.rs`.

The locked D-17 test mechanism (115-VALIDATION.md option (c) — structural, no test double, no
live-SPIRE test) is **unchanged**. Only the test's *scope* changed.

### What was blind before

The old test hardcoded `"async fn handle_spiffe_route("` and scanned that one body. It was
structurally incapable of observing the sibling path — which is precisely why the gap shipped.

### The new form is self-enumerating by BEHAVIOUR, not by name

It does not name the functions it checks. It:

1. truncates `reverse.rs` at the first `\n#[cfg(test)]` so only production code is scanned;
2. enumerates **every top-level `async fn`** (`async fn` / `pub async fn` / `pub(crate) async fn`
   at column 0), bounding each body at the next unindented `\n}\n`;
3. strips whole-line `//` and `///` comments from each body;
4. classifies a function as *minting* if its body contains any `MINT_MARKERS` call syntax —
   `managed_auth.acquire(`, `.get_or_refresh(`, `.fetch_token(`, `exchange_jwt_assertion(`;
5. for **each** minting function, asserts `ctx.filter.check_host(` and
   `NetworkAuditDenialCategory::HostDenied` both textually precede the earliest mint marker.

A future third SPIFFE dispatch path in this file is therefore covered **with no edit to the test**,
whatever it is named — the name-prefix residual the task warned about does not exist.

Three deliberate must-acknowledge failure modes, all of which fail loudly rather than skipping:

- a minting function with **no** `check_host` at all → panic ("a minting dispatch path with no
  host-check cannot be 'correctly ordered' — it is unguarded"). "No check to order" must never
  read as "ordered".
- a minting function with a check but no `HostDenied` emission → panic.
- `KNOWN_MINTING_DISPATCH_FNS` (the two current paths) is asserted to be a **subset** of what
  discovery classified as minting. This is the coverage-loss detector: if a mint primitive is
  renamed or replaced so a known path stops matching `MINT_MARKERS`, discovery would silently drop
  it and the loop would vacuously pass — this assertion converts that into a failure. It is *not*
  what drives the ordering assertions.

Comment-stripping (step 3) also removes a real prior footgun: Plan 115-04 had to **reword** its own
in-source comment because the naive search matched the comment prose instead of the call site.
Ordering comments may now name the real call syntax freely — which is what let item 3 below be
written truthfully.

### Residuals, stated plainly

- **Scope is `reverse.rs` only.** A SPIFFE dispatch added to a different proxy path file would not
  be enumerated. Bounded by D-03/ADR-113: the other proxy paths (CONNECT tunnel, forward HTTP,
  external-proxy chain) fail closed on a SPIFFE-declared route rather than implementing dispatch,
  itself covered by `d03_connect_and_forward_http_deny_spiffe_declared_route_upstream_end_to_end`.
- **Call-syntax matching, not a call graph.** A mint reached only via a new helper function that
  this file calls would not be seen. `MINT_MARKERS` includes the lower-level primitives
  (`.fetch_token(`, `exchange_jwt_assertion(`) to narrow this, but it is not eliminated.
- **Source-position, not execution-order.** This is the locked option (c) and is inherent to it:
  it proves textual precedence in a straight-line function body, not a control-flow-graph
  dominance property. Both functions are straight-line between the two points, so precedence is
  order here — but a future `goto`-like restructuring (early `if` that jumps past the check) would
  need a different mechanism.

---

## 3. Load-bearing verification — BOTH functions, independently

Performed by `Edit` on `reverse.rs` (never `git stash`, never a destructive git command), with a
byte-identical copy held in the session scratchpad for restore.

### 3a. Revert the path-2 hoist (`handle_spiffe_assertion_credential`) → test FAILS

```
running 1 test
test d17_spiffe_dispatch_host_check_precedes_mint_structurally ... FAILED

thread 'd17_spiffe_dispatch_host_check_precedes_mint_structurally' (69232) panicked at
crates\nono-proxy\tests\spiffe_integration.rs:264:9:
D-17/DRAIN-06 regression in `handle_spiffe_assertion_credential`: ctx.filter.check_host(...)
(byte 2628) must textually precede the credential mint `.get_or_refresh(` (byte 1657) — a
deny_domain-blocked SPIFFE route must never trigger a SPIRE Workload API fetch (or an IdP
token-endpoint POST of a minted SVID) before being denied.

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out
```

Restored → `test result: ok. 1 passed; 0 failed`.

### 3b. Revert the path-1 hoist (`handle_spiffe_route`) → test FAILS

```
running 1 test
test d17_spiffe_dispatch_host_check_precedes_mint_structurally ... FAILED

thread 'd17_spiffe_dispatch_host_check_precedes_mint_structurally' (96004) panicked at
crates\nono-proxy\tests\spiffe_integration.rs:264:9:
D-17/DRAIN-06 regression in `handle_spiffe_route`: ctx.filter.check_host(...) (byte 3255) must
textually precede the credential mint `managed_auth.acquire(` (byte 2358) — a deny_domain-blocked
SPIFFE route must never trigger a SPIRE Workload API fetch (or an IdP token-endpoint POST of a
minted SVID) before being denied.

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out
```

Restored → full suite `test result: ok. 6 passed; 0 failed`.

Each revert names **the function that was reverted** and **that function's own mint primitive** —
so the test is not merely failing, it is failing for the right reason, per function. That is the
property the old single-function test could not have.

### 3c. Zero residue

After both restores, `grep -c "TEMPORARY REVERT" crates/nono-proxy/src/reverse.rs` → `0`, and the
file was restored from a byte-identical pre-revert copy. The pure-move proof in §4 was computed on
the restored file.

---

## 4. No deny surface was narrowed — evidence

### 4a. Mechanical proof: the change is a pure move, zero code delta

Comparing the multiset of **non-comment** lines removed vs. added across the whole `reverse.rs`
diff (whitespace-normalised):

```
$ comm -23 rem.txt add.txt   # code removed but never re-added
(empty)
$ comm -13 rem.txt add.txt   # code added that was not previously present
(empty)
$ wc -l rem.txt add.txt
  28 rem.txt
  28 add.txt
```

Every one of the 28 code lines removed reappears verbatim. **No production statement was added,
deleted, or altered** — only relocated, plus comments. The deny block itself (`check_host` call,
`if !check.result.is_allowed()`, `reason`, `warn!`, `send_error(403)`, `log_denied(... HostDenied
...)`, `return Ok(())`) shows **zero `-`/`+` lines** in the diff for the assertion path; the only
change adjacent to it is a comment inserted above it.

### 4b. The predicate is unchanged

`ctx.filter.check_host(&upstream_host, upstream_port)` — same receiver, same two arguments, same
`upstream_host`/`upstream_port` values, produced by the same `parse_upstream_url(&upstream_url)`
call on the same `upstream_url` expression. No host:port narrowing gate was introduced (the Phase
114 CR-03 lesson — *verifying a guard's placement is not verifying its predicate width*). The
`HostDenied` category, the 403 status, and the audit `EventContext` are byte-identical.

### 4c. The deny surface strictly WIDENED, and here is why

Previously, the exchange ran first, and its failure arm `return`ed 503 before the host check was
ever reached. So a request whose upstream was `deny_domain`-blocked **and** whose token exchange
failed (SPIRE agent down, IdP unreachable, workload identity gone) was answered 503
`ManagedCredentialUnavailable` and produced **no `HostDenied` audit record at all**.

Now the host check runs unconditionally after the auth gate, so:

- every request that reached the check before still reaches it, and
- requests that previously exited at the 503 arm now reach it too.

Net: strictly more requests are host-checked; strictly more `HostDenied` denials are emitted; zero
previously-denied requests are now allowed. The only behavioural difference on a denied host is
that the response for that overlap case changes 503 → 403 and gains a `HostDenied` audit record —
a more accurate denial, and the categorisation the denial/audit spine (Plan 115-02's required
`category` parameter) is meant to carry into Phase 118's receipts.

### 4d. Regression evidence

`cargo test -p nono-sandbox-proxy` — **307 lib passed, 6 integration passed, 0 failed**. This
includes `reverse.rs`'s own `capture_rewrites_via_spiffe_assertion_site` and
`spiffe_route_denies_missing_session_token_before_credential_acquisition`, which exercise both
dispatch functions' surrounding behaviour.

---

## 5. The two false comments — before / after

The project record is explicit that a false in-source comment is itself a defect. Both were made
**true**, not softened.

### 5a. `handle_spiffe_assertion_credential`'s doc comment

**Before** (false: after Plan 115-04's one-sided hoist, it did not mirror `handle_spiffe_route`'s
structure, and "exactly" was an overclaim regardless):

> `credential_store.spiffe_assertion_routes`, Plan 113-03). **Mirrors `handle_spiffe_route`'s
> structure exactly**, substituting credential acquisition with
> `spiffe_assertion.cache.get_or_refresh()` …

**After** (separates the invariant that *is* shared from the differences that are deliberate, and
names the test that pins it):

> Shares `handle_spiffe_route`'s security-relevant control-flow **ORDER** — session-token auth
> gate, then the filter host-check and its `HostDenied` deny branch, and only then any credential
> acquisition (D-17/DRAIN-06; the two paths were brought back into agreement on this after the
> Phase 115 hoist was initially applied to `handle_spiffe_route` alone, and the ordering is now
> pinned for BOTH by
> `spiffe_integration.rs::d17_spiffe_dispatch_host_check_precedes_mint_structurally`).
>
> It deliberately **DIFFERS** from `handle_spiffe_route` elsewhere: credential acquisition is
> `spiffe_assertion.cache.get_or_refresh()` …; the SPIFFE audit context is built inline …; it takes
> no `route: &LoadedRoute` (dispatching off `SpiffeAssertionRoute`, so `capture` is looked up via
> `ctx.route_store`); and it supports no per-route TLS connector.

The four enumerated differences were each verified against the code, not assumed.

### 5b. The D-17 comment in `handle_spiffe_route`

**Before** (false at the time it was written — the assertion path did *not* use a check-first
structure):

> … so a deny_domain-blocked upstream is now rejected before any live SPIRE Workload API fetch is
> attempted — **matching the check-first structure every other `HostDenied` site in this file
> already uses** (re-grep …).

**After** (true now, and honest about the fact that it was not true then — the record of the near
miss is kept in-source rather than erased):

> … so a deny_domain-blocked upstream is now rejected before any live SPIRE Workload API fetch is
> attempted.
>
> This check-first shape is shared by every `HostDenied` site in this file: the static-credential
> path, and — since the Phase 115 gap closure that followed this hoist —
> `handle_spiffe_assertion_credential`'s RFC 7523 path too. (**It was NOT true of the assertion
> path when this comment was first written**: the hoist landed on this function only, leaving the
> sibling minting before its own check. Re-grep `check_host` / `HostDenied` for the current sibling
> locations; do not trust line numbers.) Both SPIFFE dispatch paths' ordering is now pinned
> structurally by `spiffe_integration.rs::d17_spiffe_dispatch_host_check_precedes_mint_structurally`,
> which enumerates the SPIFFE dispatch functions from this source rather than naming one.

The "every `HostDenied` site" claim was re-verified against all three sites in the file
(`reverse.rs` `check_host` occurrences: the static-credential path, `handle_spiffe_route`,
`handle_spiffe_assertion_credential`) rather than asserted.

---

## 6. Gates

| Gate | Result |
|---|---|
| `cargo build --workspace --all-targets` | exit 0 |
| `cargo fmt --all -- --check` | clean, exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | clean, exit 0 |
| `cargo test -p nono-sandbox-proxy` | 307 + 6 + 0 passed, 0 failed |

`make` is not installed on this host; cargo was invoked directly. Cross-target clippy gates are
exempt for this phase per `115-VALIDATION.md` (and per V-03, the exemption's *conclusion* holds
even though its stated premise was imprecise — no changed line here sits in a Unix-cfg branch;
`reverse.rs` and `spiffe_integration.rs` contain no `#[cfg(target_os = ...)]` blocks at all).

Known pre-existing failures, not chased and not touched by this change: `nono-sandbox --lib`
`helper_stamps_session_token_from_env` (parallel-run only) and the ~11 `nono-sandbox-cli --bin
nono` Windows-host baseline failures.

## 7. Scope discipline

- `.planning/STATE.md`, `.planning/ROADMAP.md` — **not modified**. No `gsd-sdk query state.*` /
  `roadmap.*` / `phase.complete` verb was run.
- `../nono-py` — **not touched**; this gap was entirely in `nono`.
- No `git stash`, `git clean`, `git reset --hard`, or any other destructive git operation.

---

## Verdict

`115-VERIFICATION.md`'s SC5 second half — *"a `deny_domain`-blocked SPIFFE route is denied before
any JWT-SVID is minted"* — now holds for **every** SPIFFE-backed auth path in `reverse.rs`, and the
regression net discovers its own subjects rather than naming one, so DRAIN-06 is closed at the
class level the phase goal requires.

---
*Phase: 115-v3-6-carry-forward-drain*
*Closed: 2026-08-09*
