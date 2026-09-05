---
phase: 260904-wkv-consumer-side-scaffold-for-the-nono-pope
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-DESIGN-session-credential-consumer.md
  - .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-D5-receipt-schema-proposal.md
  - .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-endpoint-receipt-0.1.schema.json
  - crates/nono/src/session_credential.rs
  - crates/nono/src/lib.rs
autonomous: true
requirements: [REQ-CRED-03, REQ-CRED-04, REQ-CRED-05, REQ-CRED-06]
must_haves:
  truths:
    - "A reviewer can read one design note that explains launch-sequence placement, the injection boundary, the D-4 renewal sketch (both branches, undecided), and all 4 named conflicts with file:line evidence — without opening the external contract file."
    - "A reviewer can see a D-5 proposal labeled 'PROPOSAL FOR JOINT REVIEW — NOT DECIDED' presenting both options with a preferred pick and a one-paragraph rationale, plus a schema file for the preferred option that no code references."
    - "cargo build -p nono-sandbox succeeds with the new session_credential module compiling, containing zero .unwrap()/.expect() outside #[cfg(test)]."
    - "cargo test -p nono-sandbox session_credential runs 6 or more tests, all passing, covering REQ-CRED-04, REQ-CRED-05, REQ-CRED-06, and a dedicated redaction perturbation check."
    - "The secret credential value is structurally unreachable via Debug output of SessionCredentialKey or via Serialize output of anything derived from SessionCredential (WorkspaceVisibleSessionCredential has no key field at all)."
    - "The D-1 outbound-auth placeholder exists as an unreachable todo!() behind a trait default method and does not break cargo build or cargo test."
  artifacts:
    - path: ".planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-DESIGN-session-credential-consumer.md"
      provides: "Design note: gate banner, launch-sequence placement, injection boundary, D-4 renewal sketch, 4 recorded conflicts, explicit non-scope statement"
      min_lines: 80
    - path: ".planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-D5-receipt-schema-proposal.md"
      provides: "D-5 proposal, both options, preferred option + rationale, non-adoption statement"
      min_lines: 30
    - path: ".planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-endpoint-receipt-0.1.schema.json"
      provides: "Draft 2020-12 JSON Schema for the preferred D-5 option, unregistered anywhere in crates/"
      contains: "\"$schema\": \"https://json-schema.org/draft/2020-12/schema\""
    - path: "crates/nono/src/session_credential.rs"
      provides: "SessionCredentialRequest, SessionCredential, WorkspaceVisibleSessionCredential, SessionCredentialKey, IssueError, SessionCredentialIssuer trait, fail-closed launch-step helper, cfg(test) mock + REQ-named tests"
      exports: ["SessionCredentialRequest", "SessionCredential", "WorkspaceVisibleSessionCredential", "SessionCredentialKey", "IssueError", "SessionCredentialIssuer", "issue_session_credential_or_fail_closed"]
    - path: "crates/nono/src/lib.rs"
      provides: "pub mod session_credential; plus a re-export line for the module's public types"
      contains: "session_credential"
  key_links:
    - from: "crates/nono/src/lib.rs"
      to: "crates/nono/src/session_credential.rs"
      via: "pub mod session_credential; + pub use session_credential::{...}"
      pattern: "session_credential"
    - from: "crates/nono/src/session_credential.rs (tests)"
      to: "SessionCredentialIssuer trait"
      via: "impl SessionCredentialIssuer for MockSessionCredentialIssuer"
      pattern: "impl SessionCredentialIssuer for"
    - from: "crates/nono/src/session_credential.rs (issue_session_credential_or_fail_closed)"
      to: "IssueError::FailClosed"
      via: "no fallback branch — Result is returned unchanged from the issuer call"
      pattern: "IssueError::FailClosed"
---

<objective>
Build the consumer-side, spec-and-scaffold-only artifacts for the Nono↔Popeye Session Credential Contract
v0.1 (canonical, external, at `fiskroad/contracts/NONO_POPEYE_SESSION_CREDENTIAL_CONTRACT_v0.1.md` — reference
by path, never copy into this repo): a design note, a labeled-not-adopted D-5 schema proposal, and a
policy-free Rust interface stub (types + trait + mock + REQ-named tests) in `crates/nono`'s core library.

Purpose: give the joint review (nono + popeye maintainers + program lead) something concrete to react to for
D-1..D-5, and give a future, charter-entry-criteria-met implementation a compiling, tested contract shape to
build against — without deciding anything, wiring anything, or touching any containment/network/receipt-emission
code path.

Output: 3 new `.planning/` documents (design note, D-5 proposal, JSON Schema) + 1 new Rust module
(`crates/nono/src/session_credential.rs`) + 1 one-line-plus-re-export edit to `crates/nono/src/lib.rs`.

**Hard gates (from the requester, non-negotiable):**
- NO containment engineering, NO driver/WFP/AppContainer changes, NO live gateway calls, NO real HTTP client.
- NO schema registration or emission wiring — the D-5 schema is a proposal file only.
- D-1..D-5 are OPEN HUMAN DECISIONS. Do not decide them. D-5 is proposed (labelled, not adopted).
- The client's own auth (D-1) MUST be `todo!()` behind a trait default method citing contract §7 D-1, and
  MUST be unreachable from any test or any binary call graph.
- Do NOT modify: `crates/nono/src/receipt.rs`, `receipt_chain.rs`, `receipt_sink.rs`, `receipt_commands.rs`,
  anything under `exec_strategy_windows/`, `agent_daemon/`, `nono-proxy/`, or `.planning/ROADMAP.md` /
  `.planning/REQUIREMENTS.md` / `.planning/STATE.md`.
- Do NOT touch Phase 118 artifacts or code surface (it is PAUSED awaiting a human-verify gate).
- Cross-target clippy is NOT required for this plan: no code added is `#[cfg(target_os = ...)]`-gated. If a
  later change to this file ever adds platform-gated code, that requirement flips back on — it does not apply
  to the work described here.
- No new Cargo dependency is needed: `thiserror`, `serde` (with the `derive` feature, enabled at the workspace
  level), and `zeroize` are already present in `crates/nono/Cargo.toml`. Do not edit `Cargo.toml`.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@.planning/NONO_AGENT_RUNTIME_CHARTER_v0.1.md

The canonical contract (read by path, never copy into this repo):
C:\Users\OMack\fiskroad\contracts\NONO_POPEYE_SESSION_CREDENTIAL_CONTRACT_v0.1.md

<interfaces>
<!-- Existing patterns the executor should follow, extracted so no further codebase exploration is needed. -->

From `crates/nono/src/receipt.rs` (existing EnforcementReceipt module — read fully before writing the design
note's Conflict 1/2 sections and before writing session_credential.rs's module doc, to match its documentation
style and to avoid accidentally reproducing its D-20/content-free constraints in the new module):
- Module doc pattern: numbered `# D-NN: <title>` subsections explaining WHY a constraint exists, not just what
  it is (receipt.rs:1-67).
- `EnforcementReceipt` struct (receipt.rs:270-300) is the type whose D-20 doc comment (receipt.rs:18-26)
  states "No mid-session field exists... or will be added" — this is Conflict 1's primary citation.
- `SessionOutcome`'s doc comment (receipt.rs:221-228) is the precedent for "never duplicate a fact that
  already has a canonical home" reasoning — reuse this exact reasoning style for the D-5 proposal's
  `layer_attestations_ref` (a reference, never inlined attestation rows).

From `crates/nono-cli/tests/receipt_content_free_scan.rs`:
- `ALLOWED_TYPES` (receipt_content_free_scan.rs:144-151) and the `SESSION_ID_EXCEPTION_*` constants
  (receipt_content_free_scan.rs:153-158) are Conflict 2's primary citation — do not modify this file; only
  cite it in the design note.

From `crates/nono-proxy/src/credential.rs` (DO NOT MODIFY — naming reference only):
- `LoadedCredential`'s hand-written redacting `Debug` impl (credential.rs:69-81) is the established house
  pattern for a secret-adjacent struct's `Debug` — mirror its shape (a `f.debug_struct`/`debug_tuple` call with
  a literal `"[REDACTED]"` field, never a derived `Debug`) in `SessionCredentialKey`.
- `CredentialStore::load`'s per-route injection wiring (credential.rs:202-393) and its use of
  `nono::keystore::load_secret_by_ref` (credential.rs:234) are the design note's §3 "naming only" citations —
  do not import from or call into this file from `session_credential.rs`.

From `crates/nono/src/error.rs`:
- `NonoError` (error.rs:24-394) is a single, large `thiserror`-derived enum with an exhaustive
  `diagnostic_code()`/`remediation()` match (error.rs:399-570) that every existing variant must satisfy. This
  scaffold's `IssueError` is a SEPARATE, dedicated `thiserror`-derived enum — NOT a new `NonoError` variant —
  specifically so this speculative, unratified-contract error shape never has to be threaded through
  `NonoError`'s exhaustive match arms. Follow `NonoError`'s style (`#[derive(Error, Debug)]`, `#[error("...")]`
  messages with named fields) for `IssueError`, but do not add anything to `error.rs` itself.

From `crates/nono/src/lib.rs` (re-export pattern to replicate for the new module):
- Existing `pub mod receipt;` (lib.rs:61) paired with `pub use receipt::{EnforcementReceipt, EntryPath,
  LayerId, LayerReceiptRow, SessionOutcome, TokenArm};` (lib.rs:94-96) is the exact pattern to replicate for
  `session_credential`.

From `crates/nono/Cargo.toml`:
- `thiserror.workspace = true`, `serde.workspace = true`, `zeroize.workspace = true` (Cargo.toml:35-39) are
  already present — no edit needed. Root `Cargo.toml:26` confirms `serde` has the `derive` feature enabled at
  the workspace level.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Write the design note, the D-5 proposal, and its JSON Schema</name>
  <files>
    .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-DESIGN-session-credential-consumer.md
    .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-D5-receipt-schema-proposal.md
    .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-endpoint-receipt-0.1.schema.json
  </files>
  <action>
    Write the design note first. Open with a gate banner (a blockquote at the very top) stating: the contract
    is status Draft requiring joint review before either side implements; the charter's entry criteria are
    UNMET (cite that `endpoint-detection/0.1` appears nowhere under `crates/` in this repo — verified by a
    repo-wide search, only two prose mentions in `.planning/NONO_AGENT_RUNTIME_CHARTER_v0.1.md`); this document
    builds nothing, wires nothing, and decides none of D-1..D-5; the contract is referenced by its
    `fiskroad/contracts/...` path throughout, never quoted wholesale or copied into this repo. Then cover, as
    separate headed sections: (1) launch-sequence placement — issuance sits before containment finalize and
    must fail closed on failure (contract §3, REQ-CRED-05), drawing the "same class of pre-resume gate" analogy
    to the D-21 attestation gate described in `receipt.rs`'s module doc without claiming they are the same
    mechanism; (2) the injection boundary (REQ-CRED-04) — name `crates/nono-proxy/src/credential.rs`'s
    `LoadedCredential`/`CredentialStore::load` injection pattern and `crates/nono/src/keystore.rs`'s URI-scheme
    secret loading as the existing concrete mechanisms a real implementation would build on, naming only,
    building nothing; (3) a renewal state-machine sketch for contract §4.2 with states issue -> active ->
    expiring -> renewed | ended, explicitly describing BOTH the "renewal allowed" and "renewal disallowed"
    branches since D-4 leaves the choice undecided; (4) a "Conflicts With Existing nono Design" section with
    exactly four numbered conflicts, each UNRESOLVED, each with file:line evidence: Conflict 1 cites
    `receipt.rs:18-26`'s D-20 doc ("No mid-session field exists... or will be added") against the contract's
    §5 session-window and §4.2 key_ref chain requirements; Conflict 2 cites
    `receipt_content_free_scan.rs:144-158`'s `ALLOWED_TYPES`/`SESSION_ID_EXCEPTION_*` constants against adding
    `key_refs`/`agent_id`/`on_behalf_of` fields to `EnforcementReceipt`; Conflict 3 cites the repo-wide search
    result showing `endpoint-detection/0.1` exists nowhere under `crates/`, only in the charter's prose, making
    the contract's "extend endpoint-detection/0.1" D-5 option locally inexpressible; Conflict 4 cites
    `keystore.rs:186-296`'s `env://` scheme and `credential.rs:202-393`'s `CredentialStore::load` as
    durable-by-design credential paths in tension with REQ-CRED-08's "no shared, static, or cross-session agent
    credential" rule. Close with a "What This Note Does Not Do" section restating the hard gates. Target at
    least 80 lines of substantive markdown, not padding.

    Write the D-5 proposal second, in its own file, opening with the literal heading text
    "PROPOSAL FOR JOINT REVIEW — NOT DECIDED" as a top-level banner (this exact phrase must appear verbatim —
    it is mechanically checked). State the contract's own D-5 wording (paraphrase, cite section §7, do not
    verbatim-copy more than the one short defining sentence). Present "Option A: Extend endpoint-detection/0.1"
    — note explicitly that a repo-wide search found no such schema under `crates/` in this repo, so this option
    is not locally exercisable today (this is a factual, repo-scoped finding, not a claim about any other
    repository). Present "Option B (preferred): Sibling endpoint-receipt/0.1" with a single paragraph of
    rationale: detections are instantaneous events, credential-join receipts are session-window claims that can
    grow via renewal, and coupling the two risks the same "startup-only claim quietly stretched" drift D-20
    already warns against for `EnforcementReceipt` — plus the concrete local-inexpressibility of Option A.
    State plainly that the schema for the preferred option covers `session_id`, `key_refs[]`, a session window
    (start/end), and a reference (not inlined content) to the layer-attestation census. Close with an explicit
    non-adoption statement: the schema file is a `.planning/` artifact, nothing in `crates/` reads or validates
    against it, and the interface stub built in Task 2/3 does not import or depend on it.

    Write the JSON Schema third, as a separate `.json` file (not embedded in the markdown), using
    `"$schema": "https://json-schema.org/draft/2020-12/schema"`, a descriptive `"title"` that includes the text
    "PROPOSAL — NOT ADOPTED", a top-level `"description"` restating that no code in this repository loads or
    validates against this file, `"type": "object"`, `"additionalProperties": false`, and a `"required"` array
    containing at minimum `session_id`, `key_refs`, and a session-window property (name it `session_window`,
    an object with `start`/`end` RFC-3339 date-time strings, `end` nullable for an in-progress session) plus a
    `layer_attestations_ref` string property whose description states it is a reference/correlation key, never
    the inlined attestation rows. `key_refs` is an array of strings with `"minItems": 1`. Every property needs
    a `"description"` explaining which contract section it satisfies.
  </action>
  <verify>
    <automated>python -c "import json; d=json.load(open('.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-endpoint-receipt-0.1.schema.json')); assert d['$schema']=='https://json-schema.org/draft/2020-12/schema'; req=set(d['required']); assert {'session_id','key_refs','session_window','layer_attestations_ref'} <= req, req; print('schema OK')"</automated>
  </verify>
  <done>
    All 3 files exist. The D-5 proposal file contains the literal string "PROPOSAL FOR JOINT REVIEW — NOT
    DECIDED". The design note contains four distinct conflict citations with file:line references and a gate
    banner. The JSON Schema is valid draft-2020-12 JSON with the required fields above and is referenced by no
    file under `crates/`.
  </done>
</task>

<task type="auto">
  <name>Task 2: Write the core interface stub (types, secret newtype, trait, fail-closed helper, D-1 placeholder)</name>
  <files>
    crates/nono/src/session_credential.rs
    crates/nono/src/lib.rs
  </files>
  <action>
    Create `crates/nono/src/session_credential.rs`. Open with a module doc (`//!`) stating: this module is
    policy-free vocabulary for the Nono↔Popeye Session Credential Contract v0.1 (reference by path:
    `fiskroad/contracts/NONO_POPEYE_SESSION_CREDENTIAL_CONTRACT_v0.1.md`, never quoted or copied in full here);
    it contains no network implementation and calls no real issuance endpoint; it must never import from
    `nono-cli` or `nono-proxy`; and it explicitly documents why `SessionCredentialKey` never derives or
    implements `Serialize` (serde's derive would serialize the wrapped secret string verbatim, exactly the leak
    REQ-CRED-04 forbids — omitting the impl entirely makes the mistake a compile error on `SessionCredential`
    rather than a runtime leak, since `SessionCredential` itself also does not derive `Serialize` for the same
    reason). Cross-reference the companion design note by relative path for the recorded, unresolved conflicts
    without re-explaining them here.

    Define `pub struct SessionCredentialRequest` mirroring contract §2.1 exactly, field names included:
    `principal_type: String`, `agent_id: String`, `on_behalf_of: String`, `purpose: String`,
    `tenant_id: String`, `requested_ttl_seconds: u64`, `session_id: String`, `device_ref: String`. Derive
    `Debug, Clone, PartialEq, Eq, Serialize, Deserialize` (this struct carries no secret). Add a
    `pub const AGENT_PRINCIPAL_TYPE: &str = "agent";` constant documented as the contract's fixed
    `principal_type` value.

    Define `pub struct SessionCredentialKey` as a newtype wrapping `zeroize::Zeroizing<String>`. Derive `Clone`
    only (never `Debug`, never anything serde-related). Hand-write `impl std::fmt::Debug for
    SessionCredentialKey` that formats as a debug tuple named `SessionCredentialKey` with a single field
    literal `"[REDACTED]"` — mirror `crates/nono-proxy/src/credential.rs`'s `LoadedCredential` Debug impl shape.
    Do not implement `std::fmt::Display`. Add `pub fn new(secret: impl Into<String>) -> Self` and
    `pub fn expose_secret(&self) -> &str` with a doc comment stating this accessor exists only for a future
    real proxy-injection call site and must never be called from workspace-visible code.

    Define `pub struct SessionCredential` mirroring contract §2.3 exactly, field names included:
    `key: SessionCredentialKey`, `key_ref: String`, `expires_at: String`, `session_id: String`,
    `budget_scope: String`. Derive `Debug, Clone` only — do NOT derive or implement `Serialize` on this struct
    (it contains the secret-bearing `key` field; document in a doc comment why no `Serialize` impl exists,
    referencing the module doc's explanation). Add `pub fn workspace_view(&self) -> WorkspaceVisibleSessionCredential`
    that clones `key_ref`, `expires_at`, `session_id`, `budget_scope` only — never touches `key`.

    Define `pub struct WorkspaceVisibleSessionCredential` with exactly four fields: `key_ref: String`,
    `expires_at: String`, `session_id: String`, `budget_scope: String` — no `key` field, ever. Derive
    `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`. Doc comment: this is the subset a contained
    workspace or config surface may see; its shape structurally cannot carry the secret because the field does
    not exist on the type.

    Define `pub enum IssueError` deriving `thiserror::Error, Debug`, with exactly two variants:
    `FailClosed { reason: String }` with `#[error("session credential issuance failed; governed launch fails
    closed: {reason}")]`, documented as REQ-CRED-05's variant (issuance failure, no fallback credential); and
    `RefusedOrExpired { reason: String }` with `#[error("session credential refused or expired: {reason}")]`,
    documented as REQ-CRED-06's variant (gateway refusal of an expired key, or issuer-side refusal, surfaced as
    a distinct condition from a fail-closed launch abort).

    Define `pub trait SessionCredentialIssuer` with one required method,
    `fn request_session_credential(&self, req: &SessionCredentialRequest) -> Result<SessionCredential,
    IssueError>;`, and one default method, `fn outbound_auth_placeholder(&self) -> !`, whose body is exactly
    `todo!("contract §7 D-1 — issuer identity, undecided")`, with a doc comment citing "contract §7 D-1 —
    issuer identity, undecided" verbatim and stating this default is never overridden or called by any code in
    this crate; a real HTTP-backed implementor fills it in only after the joint review ratifies D-1.

    Define `pub fn issue_session_credential_or_fail_closed(issuer: &dyn SessionCredentialIssuer, req: &
    SessionCredentialRequest) -> Result<SessionCredential, IssueError>` whose body calls
    `issuer.request_session_credential(req)` and returns the result unchanged — no retry, no default
    credential, no fallback branch of any kind. Doc comment: this is the single call site REQ-CRED-05 needs; it
    is not wired into `exec_strategy_windows/`, `agent_daemon/`, or any real launch path by this scaffold.

    Edit `crates/nono/src/lib.rs`: add `pub mod session_credential;` alongside the existing `pub mod receipt;`
    line (keep the existing module list's alphabetical-ish ordering intact — insert near `scrub`/`state` per
    the existing list's position), and add a re-export line
    `pub use session_credential::{IssueError, SessionCredential, SessionCredentialIssuer,
    SessionCredentialKey, SessionCredentialRequest, WorkspaceVisibleSessionCredential,
    issue_session_credential_or_fail_closed};` following the same style as the existing `pub use receipt::{...};`
    line.
  </action>
  <verify>
    <automated>cargo build -p nono-sandbox 2>&1 | tail -60</automated>
  </verify>
  <done>
    `cargo build -p nono-sandbox` exits 0. `grep -n "unwrap()\|expect(" crates/nono/src/session_credential.rs`
    returns no matches. `crates/nono/src/lib.rs` contains `pub mod session_credential;` and a `pub use
    session_credential::{...}` re-export line. `SessionCredentialKey` has a hand-written `Debug` impl and no
    `Serialize`/`Display` impl. `WorkspaceVisibleSessionCredential` has no field named `key`.
    `outbound_auth_placeholder` exists as a trait default method whose body is `todo!("contract §7 D-1 —
    issuer identity, undecided")`.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Add the cfg(test) mock and the REQ-named tests, then run the full verification gate</name>
  <files>crates/nono/src/session_credential.rs</files>
  <behavior>
    - `req_cred_04_debug_output_redacts_secret_and_contains_marker`: build a `SessionCredentialKey::new("sk-scaffold-secret-001")`, format it with `{:?}`, assert the output contains `"[REDACTED]"` and does NOT contain the literal string `"sk-scaffold-secret-001"`.
    - `req_cred_04_workspace_view_serializes_key_ref_without_secret`: build a `SessionCredential` with a distinct secret literal (e.g. `"sk-scaffold-secret-002"`) and a known `key_ref` (e.g. `"kr-002"`), call `.workspace_view()`, `serde_json::to_string` it, assert the JSON contains `"kr-002"` and does NOT contain `"sk-scaffold-secret-002"`.
    - `redaction_perturbation_secret_absent_from_debug_and_marker_present`: using a THIRD, distinct secret literal (e.g. `"sk-scaffold-secret-003-perturbed"`) not reused from the two tests above, format the `SessionCredentialKey`'s `Debug` output and assert both that the literal secret is absent AND that `"[REDACTED]"` is present — this is the dedicated anti-vacuous-pass check, distinct from the two req_cred_04 tests above because it uses its own literal rather than sharing one.
    - `req_cred_05_issuance_failure_returns_fail_closed_error_with_no_credential`: a `MockSessionCredentialIssuer` configured to return `Err(IssueError::FailClosed { reason: "issuer unreachable".into() })` from `request_session_credential`; call `issue_session_credential_or_fail_closed(&mock, &req)`; assert the result is `Err` and specifically `matches!(result, Err(IssueError::FailClosed { .. }))` (not merely `is_err()`); assert it is NOT `IssueError::RefusedOrExpired`.
    - `req_cred_06_expiry_or_refusal_is_a_distinct_condition`: a mock configured to return `Err(IssueError::RefusedOrExpired { reason: "key expired".into() })`; call the trait method (directly or via the helper); assert `matches!(result, Err(IssueError::RefusedOrExpired { .. }))` and assert it is NOT `IssueError::FailClosed` — proving the two error conditions are distinguishable, not collapsed.
    - `req_cred_06_renewal_produces_new_key_ref_under_same_session_id`: two separate mock instances, each configured to return `Ok(SessionCredential { .. })` with the SAME `session_id` (e.g. `"sess-renewal-001"`) but DIFFERENT `key_ref`s (e.g. `"kr-a"` then `"kr-b"`); call both; assert `first.session_id == second.session_id` and `first.key_ref != second.key_ref` — the contract §4.2/§5 renewal-chain shape.
  </behavior>
  <action>
    Below the production code in `crates/nono/src/session_credential.rs`, add a `#[cfg(test)]` section
    containing: (1) a small `MockSessionCredentialIssuer` struct wrapping a fixed `Result<SessionCredential,
    IssueError>` (construct it via a `Result::clone()`-able value, e.g. store
    `std::cell::RefCell<Option<Result<SessionCredential, IssueError>>>` or simpler — a struct holding the
    already-built `Result` directly and returning `.clone()` from `request_session_credential`, since
    `SessionCredential` derives `Clone` and `IssueError` can derive `Clone` too if needed, or simply construct
    a fresh `Result` per test rather than reusing one mock across calls); implement `SessionCredentialIssuer`
    for it, returning the stored/constructed result and never overriding `outbound_auth_placeholder`. (2) a
    `mod tests` block with the six test functions specified in `<behavior>` above, each named exactly as
    listed there (the `req_cred_04_*`/`req_cred_05_*`/`req_cred_06_*` prefixes are load-bearing — they are how
    this scaffold demonstrates per-REQ coverage). Use plain `assert!`/`assert_eq!`/`matches!` — no
    `.unwrap()`/`.expect()` outside this `#[cfg(test)]` block (which is permitted here per CLAUDE.md's testing
    exception).

    After the tests compile, run the full verification gate in this exact order and confirm each step's exit
    code and reported test count before moving to the next: `cargo build -p nono-sandbox` (must exit 0);
    `cargo test -p nono-sandbox session_credential -- --nocapture` (must exit 0 AND report running 6 or more
    tests, all `ok` — a 0-tests-run result is a FAILURE of this gate, not a pass, per the house selector-hazard
    lesson); `cargo build --workspace` (confirms the additive `lib.rs` re-export does not break `nono-cli`,
    `nono-proxy`, `nono-shell-broker`, or `nono-ffi`, all of which depend on `nono`); `cargo fmt --all --
    --check` (must exit 0 — reformat with `cargo fmt --all` and re-check if it fails, do not hand-format);
    `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` (must exit 0
    — this is the Makefile's own `make clippy` invocation; do not scope it down to just this crate, since it
    must also confirm the `lib.rs` edit introduces no cross-crate warning). Cross-target clippy
    (`cross clippy ... --target x86_64-unknown-linux-gnu`, `cargo-zigbuild clippy ... --target
    x86_64-apple-darwin`) is NOT required for this task: no file touched by this plan contains
    `#[cfg(target_os = ...)]`.
  </action>
  <verify>
    <automated>cargo test -p nono-sandbox session_credential -- --nocapture 2>&1 | tail -30</automated>
  </verify>
  <done>
    All six named tests exist and pass; `cargo test -p nono-sandbox session_credential` reports 6 or more
    tests run, 0 failed. `cargo build --workspace`, `cargo fmt --all -- --check`, and
    `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` all exit 0.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| `SessionCredentialKey` internal state <-> any `Debug`/log/panic-message surface | The secret bearer key must never cross into human-readable diagnostic output. |
| `SessionCredential` <-> any config/workspace-visible serialized surface | The secret must never cross into a serialized artifact a contained workspace or config file could expose. |
| `issue_session_credential_or_fail_closed` caller <-> issuance outcome | A future real launch sequence must receive a typed, distinguishable error on failure, never a silent default/fallback credential. |
| This scaffold <-> a future real HTTP-backed `SessionCredentialIssuer` | The D-1 outbound-auth gap is a deliberate, documented, unreachable `todo!()` — not a stubbed-out insecure default. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-260904wkv-01 | Information Disclosure | `SessionCredentialKey` | mitigate | Hand-written `Debug` redacts to `"[REDACTED]"`; no `Serialize`/`Display` impl exists on the type at all (compile-time absence, not runtime filtering); verified by `req_cred_04_debug_output_redacts_secret_and_contains_marker` and the independent-literal `redaction_perturbation_secret_absent_from_debug_and_marker_present` test |
| T-260904wkv-02 | Information Disclosure | `WorkspaceVisibleSessionCredential` | mitigate | Struct has no `key` field at all — structurally cannot serialize the secret regardless of derive macros used; verified by `req_cred_04_workspace_view_serializes_key_ref_without_secret` |
| T-260904wkv-03 | Denial of Service / fail-open | `issue_session_credential_or_fail_closed` | mitigate | Helper returns the issuer's `Result` unchanged — no retry, no default credential, no fallback branch; the distinct `IssueError::FailClosed` variant (never collapsed with `RefusedOrExpired`) is asserted by `req_cred_05_issuance_failure_returns_fail_closed_error_with_no_credential` |
| T-260904wkv-04 | Spoofing | `SessionCredentialIssuer::outbound_auth_placeholder` (D-1) | accept | D-1 (the client's own issuer-facing auth) is an explicitly open joint-review decision per contract §7; the placeholder is an unreachable `todo!()` behind a trait default method never invoked by any test or call graph in this crate — accepted as scaffold-scope, not a shipped or reachable auth mechanism |
| T-260904wkv-05 | Tampering (supply chain) | Cargo dependencies | accept | Zero new dependencies added — `thiserror`, `serde` (derive), `zeroize` are already present in `crates/nono/Cargo.toml`; no package-legitimacy gate applies |
</threat_model>

<verification>
1. `python -c "import json; json.load(open('.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-endpoint-receipt-0.1.schema.json'))"` — schema is valid JSON.
2. `grep -c "PROPOSAL FOR JOINT REVIEW — NOT DECIDED" .planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-D5-receipt-schema-proposal.md` reports >= 1.
3. `cargo build -p nono-sandbox` exits 0.
4. `cargo test -p nono-sandbox session_credential -- --nocapture` exits 0 and reports 6 or more tests run, 0 failed.
5. `cargo build --workspace`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::unwrap_used` all exit 0.
6. `git diff --name-only` shows no changes to `crates/nono/src/receipt.rs`, `receipt_chain.rs`, `receipt_sink.rs`, `receipt_commands.rs`, anything under `exec_strategy_windows/`, `agent_daemon/`, `nono-proxy/`, `Cargo.toml` (any), `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md`, or `.planning/STATE.md`.
</verification>

<success_criteria>
- The design note, D-5 proposal, and JSON Schema exist under `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/` and satisfy the content checks in `<verification>`.
- `crates/nono/src/session_credential.rs` compiles as part of the `nono-sandbox` library and is re-exported from `crates/nono/src/lib.rs`.
- All 6 REQ-named tests pass; the full `make ci`-equivalent gate (build workspace, test, fmt --check, clippy -D warnings -D clippy::unwrap_used) is green.
- No file outside this plan's `files_modified` list changed.
- D-1 through D-5 remain undecided in code and in prose; D-5's proposal is clearly labeled as not adopted.
</success_criteria>

<output>
Create `.planning/quick/260904-wkv-consumer-side-scaffold-for-the-nono-pope/260904-wkv-SUMMARY.md` when done.
</output>
