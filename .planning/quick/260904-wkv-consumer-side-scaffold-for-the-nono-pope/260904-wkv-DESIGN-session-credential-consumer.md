# Design Note: Session Credential Contract v0.1 — nono Consumer Side

> **Gate banner.** The Nono↔Popeye Session Credential Contract v0.1
> (`fiskroad/contracts/NONO_POPEYE_SESSION_CREDENTIAL_CONTRACT_v0.1.md`, external, canonical,
> never copied into this repo) is **Status: Draft**, requiring joint review by the nono
> maintainers, the popeye maintainers, and the program lead before either side implements
> anything against it. Separately, the `NONO_AGENT_RUNTIME_CHARTER_v0.1.md` phase this work
> would eventually serve states its own §7 entry criteria are **UNMET** — item 1 requires
> `endpoint-detection/0.1` detection emission to already exist, and a repo-wide search
> confirms `endpoint-detection/0.1` appears **nowhere under `crates/`** in this repository;
> the string exists only in two prose mentions inside
> `.planning/NONO_AGENT_RUNTIME_CHARTER_v0.1.md` itself (lines 38 and 75). This document
> **builds nothing, wires nothing, and decides none of D-1 through D-5.** It exists so the
> joint review has something concrete to react to, and so a future, charter-entry-criteria-met
> implementation has a compiling, tested contract shape to build against. The contract is
> referenced below by its `fiskroad/contracts/...` path throughout; it is never quoted
> wholesale or copied into this repo.

## 1. Launch-Sequence Placement (contract §3, REQ-CRED-05)

The contract's §3 states the credential is injected by the proxy, never written to the agent
workspace, and that issuance failure must cause governed launch to **fail closed** with a
distinct error and no fallback credential. That places session-credential issuance as a
pre-resume gate: a check that must succeed *before* the confined child is allowed to run,
structurally analogous in class — not in mechanism — to nono's own D-21 attestation gate
described in `crates/nono/src/receipt.rs`'s module doc (`receipt.rs:20-26`, D-20): "the receipt
is written once, at the D-21 attestation gate, before `ResumeThread` (D-03)". Both gates share
the shape "verify a precondition before the suspended child is resumed, and abort the launch
rather than resume into a degraded state if the precondition fails" — but they are not the same
mechanism. D-21's attestation gate probes *already-applied OS state* on the confined child
itself (token labels, AppContainer SID, job membership); a session-credential issuance gate
would call an *external* service (popeye) and block on a network round-trip. The former is a
local OS probe; the latter is a remote dependency with its own availability and latency
characteristics that the launch-sequence design would need to account for (timeout, retry
policy — none of which this scaffold specifies, since D-1/D-2 are undecided and no HTTP client
exists here). The analogy is offered only to establish that "issuance sits before containment
finalize, and fails closed" is architecturally consistent with an existing, accepted pattern in
this codebase — not that the two gates should share code, a call site, or a receipt field.

## 2. The Injection Boundary (REQ-CRED-04)

REQ-CRED-04 requires the bearer key be unreadable from the contained workspace for the whole
session. This repository already has two concrete mechanisms a real implementation would build
on — named here for reference only; this scaffold imports from neither and calls into neither:

- **`crates/nono-proxy/src/credential.rs`** — `LoadedCredential` (`credential.rs:44-65`) holds a
  raw secret in a `Zeroizing<String>` field and a hand-written, redacting `Debug` impl
  (`credential.rs:69-81`) that never derives `Debug` mechanically. `CredentialStore::load`
  (`credential.rs:202-393`, per the plan's citation range) is the per-route injection wiring
  that loads a credential once at proxy startup and injects it into outbound requests — headers,
  URL paths, query parameters, or Basic Auth — so the sandboxed agent process never sees the
  real credential value. A session-credential implementation would plausibly add a new
  `InjectMode` variant or a parallel loader that calls popeye's issuance endpoint instead of a
  static keystore lookup, and reuse this same "hold it in the proxy process, inject only at the
  HTTP layer" boundary.
- **`crates/nono/src/keystore.rs`** — the `env://` URI-scheme secret loader (`ENV_URI_PREFIX`,
  `keystore.rs:187`) and its dispatch table (`keystore.rs:284-296` documents the five-backend
  dispatch order: `file://`, `env://`, `op://`, `apple-password://`, keyring) is the existing
  "credential reference resolves to a credential value, never inline in config" pattern. A
  session-credential value is fundamentally different in kind from everything `keystore.rs`
  currently loads — every existing backend resolves a **durable** reference (an environment
  variable, a file, a password-manager entry) to a **durable** secret, whereas a session
  credential is **minted per session** by a network call, never durably stored anywhere. This
  scaffold does not attempt to fit session-credential issuance into the `keystore.rs` dispatch
  table; it is named here only as the closest existing "secret resolution" vocabulary in this
  crate, for the joint review's awareness, not as a proposed reuse target.

This scaffold's own secret-adjacent type, `SessionCredentialKey` (in
`crates/nono/src/session_credential.rs`), mirrors `LoadedCredential`'s hand-written `Debug`
redaction shape but is otherwise independent of both files above — it builds no injection path
and calls no proxy code.

## 3. D-4 Renewal Sketch (contract §4.2 — both branches, undecided)

Contract §4.2 leaves D-4 (renewal policy) as an open joint-review decision: "Allow in-session
renewal (§4.2) or force session end at expiry." A state-machine sketch, both branches drawn
explicitly since neither is chosen here:

```
                 issue
                   |
                   v
              +---------+
              | active  |
              +---------+
                   |
          (approaching expires_at)
                   v
              +----------+
              | expiring |
              +----------+
               /            \
   [D-4: renewal allowed]   [D-4: renewal disallowed]
              |                          |
              v                          v
        +-----------+              +---------+
        | renewed   |              |  ended  |
        | (new key, |              | (session|
        |  same     |              |  ends at|
        |  session_ |              |  expiry,|
        |  id, new  |              |  no new |
        |  key_ref) |              |  key)   |
        +-----------+              +---------+
              |
       (loops back to "active"
        under the same session_id,
        per contract §4.2:
        "a fresh §2 issuance —
        same validation, no
        shortcut path")
```

- **Branch A — renewal allowed.** On expiry (or gateway refusal of an expired key, contract
  §4.2's REQ-AGT-04 reference), nono re-issues within the same `session_id`: a full, fresh §2
  issuance request (no shortcut validation path), producing a new `key_ref` under the unchanged
  `session_id`. The session continues. This is the shape `req_cred_06_renewal_produces_new_key_ref_under_same_session_id`
  in `session_credential.rs`'s test module exercises structurally (same `session_id`, distinct
  `key_ref` values across two issuance calls) — without deciding that renewal *should* happen,
  only that the type shape supports representing it if D-4 picks this branch.
- **Branch B — renewal disallowed.** On expiry, the session simply ends. One session, one key,
  for its entire life — "simpler audit story" per the contract's own tradeoff framing in §7 D-4.
  No renewal call is ever made; `IssueError::RefusedOrExpired` (this scaffold's error variant for
  REQ-CRED-06) is the terminal condition for the session, not an input to a retry loop.

Both branches are representable by this scaffold's types without modification — `SessionCredential`
carries `session_id` and `key_ref` as independent fields, so a caller can either mint a second one
under the same `session_id` (Branch A) or simply stop calling the issuer (Branch B). This scaffold
takes no position on which branch is correct; it is a design note observation, not a decision.

## 4. Conflicts With Existing nono Design

Four conflicts, each **UNRESOLVED**, each requiring a joint-review decision before any real
implementation proceeds:

### Conflict 1 — D-20's "no mid-session field" claim vs. contract §5's session-window / §4.2 key_ref chain

`crates/nono/src/receipt.rs:18-26` (D-20) states, verbatim in its module doc: "No mid-session
field exists on `EnforcementReceipt` or will be added. The claim is startup-only... it never
claims anything about what happens to the child after that point. Continuous or periodic
re-attestation is explicitly out of scope for this phase." The contract's §5 "the join" requires
a receipt to carry `receipt.session window [start, end]` that must satisfy `⊇ ledger row
timestamps for those keys`, and §4.2 requires representing a renewal chain — "new key, same
session, new `key_ref`" — as a growing, mid-session fact (a session can renew multiple times
while running). A session-window `end` field is definitionally not known at launch time (the
D-20/D-21 write point, before `ResumeThread`) for a still-running session, and a renewal chain
is definitionally a fact that accretes *during* the session, not at its start. `EnforcementReceipt`
as currently designed is a single, immutable, startup-only artifact; the contract's §5 join
requires something that can represent session-window close and multi-entry key-ref chains,
which is a different write-lifetime shape than D-20 permits for `EnforcementReceipt`. **This is
not resolved here.** Whether a session-credential receipt is a startup-only artifact that only
gets appended-to under some new mutation discipline, or a wholly separate one-per-session-event
record type, is a joint-review decision, not a scaffold decision — see the D-5 proposal
(`260904-wkv-D5-receipt-schema-proposal.md`) for the "separate schema family" position this
scaffold takes on that question, without deciding the mutation-discipline question.

### Conflict 2 — receipt_content_free_scan.rs's `ALLOWED_TYPES` allowlist vs. adding key_refs/agent_id/on_behalf_of to EnforcementReceipt

`crates/nono-cli/tests/receipt_content_free_scan.rs:144-151` defines `ALLOWED_TYPES` as exactly
`["u16", "u32", "EntryPath", "Option<TokenArm>", "SessionOutcome", "Vec<LayerReceiptRow>"]`, with
one named exception at `receipt_content_free_scan.rs:153-158`
(`SESSION_ID_EXCEPTION_FIELD`/`SESSION_ID_EXCEPTION_TYPE`) permitting exactly one `String`-typed
field, named exactly `session_id`. This is a mechanically-enforced allowlist scan, not a
convention — any new field on `EnforcementReceipt` typed `String` and named anything other than
`session_id` (for example `agent_id`, `on_behalf_of`, or a `Vec<String>` for `key_refs`) fails
this test today, by design (its own module doc calls this the D-05 content-free discipline). If
the joint review's D-5 answer is "extend `EnforcementReceipt` directly" rather than "add a
sibling schema," landing `key_refs: Vec<String>`, `agent_id: String`, or `on_behalf_of: String`
on `EnforcementReceipt` requires either widening this allowlist (a deliberate, reviewed loosening
of the content-free guarantee `receipt_content_free_scan.rs` exists to enforce) or wrapping those
values in a new content-free-by-construction type the way `LayerId`/`EntryPath`/`TokenArm` already
are. **This is not resolved here** — this scaffold does not touch `receipt.rs` or
`receipt_content_free_scan.rs`, and the D-5 proposal explicitly recommends a *sibling* schema
rather than an extension, partly because of this conflict.

### Conflict 3 — the contract's "extend endpoint-detection/0.1" D-5 option is locally inexpressible

Contract §7 D-5 offers, as one option, that the receipt fields it requires could live "same file
as `endpoint-detection/0.1`". A repo-wide search of this repository (`Grep` for
`endpoint-detection` under `crates/`, executed as part of writing this note) returns **zero
matches** under `crates/`. The string `endpoint-detection/0.1` exists only in
`.planning/NONO_AGENT_RUNTIME_CHARTER_v0.1.md` at lines 38 and 75, both prose references to a
schema family the charter's own §7 entry criteria list as **not yet landed** ("`NONO_v0.4`
outstanding work landed: `twg-gateway-only` profile and detection emission in
`endpoint-detection/0.1`" — an entry criterion, meaning it is not yet satisfied). There is,
therefore, no `endpoint-detection/0.1` schema, module, or type anywhere in this codebase today
to extend. This is a factual, repo-scoped finding about *this* repository — it says nothing about
whether `endpoint-detection/0.1` exists in popeye's repository or in the shared `fiskroad`
vocabulary spec. **This is not resolved here**; it is recorded as evidence for the D-5 proposal's
preference for a sibling schema (Option B), since Option A cannot be locally exercised at all
until the charter's own entry criteria are met.

### Conflict 4 — durable-by-design credential paths vs. REQ-CRED-08's "no shared, static, or cross-session agent credential"

`crates/nono/src/keystore.rs`'s `env://` scheme (`ENV_URI_PREFIX`, `keystore.rs:187`) and
`crates/nono-proxy/src/credential.rs`'s `CredentialStore::load` (`credential.rs:202-393`) are
both existing, shipped, **durable-by-design** credential paths: they load a secret once (at
proxy startup, or at env-var resolution time) and hold it for the process's entire lifetime,
across however many requests or sessions that process serves. That is precisely their intended
behavior — an API key for a long-running proxy process is not supposed to rotate every session.
REQ-CRED-08 states flatly: "No shared, static, or cross-session agent credential exists on any
endpoint," verified by an "Endpoint config audit; grep for provisioned model-path secrets."
A session-credential implementation that reused `CredentialStore`'s existing loader shape
unmodified — pointing an existing `InjectMode` at a popeye-issued key the same way it points at
a static `env://` secret today — would produce exactly the shared/static/cross-session pattern
REQ-CRED-08 forbids for *agent* credentials, unless a new, session-scoped loading and eviction
discipline is added specifically for agent-principal keys (mint once per session, discard at
session end or renewal, never cache across sessions). **This is not resolved here** — this
scaffold builds no loader of any kind, and Task 2/3's `SessionCredentialIssuer` trait and
`issue_session_credential_or_fail_closed` helper explicitly do not import from or wire into
`credential.rs`.

## 5. What This Note Does Not Do

Restating the hard gates verbatim: this note performs no containment engineering, no
driver/WFP/AppContainer changes, no live gateway calls, and implements no real HTTP client. It
does not register or wire the D-5 JSON Schema proposal into any emission path. It does not
decide D-1 (issuer identity), D-2 (endpoint mapping), D-3 (revocation), D-4 (renewal policy), or
D-5 (receipt schema residency) — all five remain open human decisions for the joint review named
in the contract's own header. It does not modify `crates/nono/src/receipt.rs`,
`receipt_chain.rs`, `crates/nono-cli/src/receipt_sink.rs`, `receipt_commands.rs`, anything under
`exec_strategy_windows/`, `agent_daemon/`, or `nono-proxy/`, and it does not touch
`.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md`, or `.planning/STATE.md`. It does not touch
any Phase 118 artifact or code surface — Phase 118 is paused at a human-verify gate and is
out of scope for this quick task entirely.
