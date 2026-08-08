# Phase 115: v3.6 Carry-Forward Drain - Context

**Gathered:** 2026-08-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Close the six findings the v3.6 milestone audit carried forward (DRAIN-01 … DRAIN-06) **at the
class level, not the symptom level**, so the denial/audit spine that Phase 118's receipt work
builds on is clean before it is built on.

Four code locations are in scope:
- `crates/nono-cli/src/profile/mod.rs` — the `platform_overrides` merge and profile validation
- `crates/nono-proxy/src/{connect,external,reverse}.rs` — the denial spine and dispatch ordering
- `crates/nono/src/undo/types.rs` — the `NetworkAuditDenialCategory` enum
- `../nono-py/src/{proxy,undo}.rs` — the sibling Python binding's codec and `RouteConfig`

**The governing framing (from the milestone invariants):** *"Structural fixes over spot fixes for
DRAIN-02/DRAIN-03. Both findings exist because a hand-maintained list drifted from its source of
truth. Patching the missing arms reproduces the class."* Every decision below was taken under that
rule — the discussion repeatedly chose the option that makes the defect **unrepresentable** over
the option that makes it **tested for**.

**Out of scope:** new confinement layers; anything from the CINT/RCPT/BOUND/TSBX workstreams;
implementing AWS SigV4 or plain OAuth2 `client_credentials` (this phase *rejects* their config,
it does not build them); the risk-accepted v3.6 residuals (WR-02, WR-03, WR-05, NEW-04).

</domain>

<decisions>
## Implementation Decisions

### DRAIN-01 — `platform_overrides` inject-field inheritance (NEW-05 + ACC-04)

**Scouting correction the planner must start from:** DRAIN-01's requirement text says these fields
are merged "with `.or(base)`". **That is not directly expressible today.** `inject_mode` is
`InjectMode` (not `Option`) with `#[serde(default)]`, and `inject_header` is `String` with
`#[serde(default = "default_inject_header")]` (`crates/nono-cli/src/profile/mod.rs:963-972`). At
merge time, "omitted" and "explicitly set to header/`Authorization`" are the *same value*. A merge
edit alone cannot fix this — a struct-shape change is required.

- **D-01: Change both to `Option<T>` and resolve at the use site.**
  `inject_mode: Option<InjectMode>`, `inject_header: Option<String>`; drop the serde defaults from
  the struct and apply `.unwrap_or_default()` / `.unwrap_or_else(default_inject_header)` where the
  values are actually consumed. This makes `merge_custom_credential_def` homogeneous — all 14
  optional fields use the identical `.or(base)` shape.
  **Known blast radius the planner must budget for:** every consumer of these two fields;
  `crates/nono-cli/data/nono-profile.schema.json`; and `../nono-py`'s `RouteConfig::new`, which
  currently takes `inject_mode`/`inject_header` as **non-Option** constructor parameters
  (`../nono-py/src/proxy.rs:191-192`, `:206-207`) — this intersects D-13 below, so sequence them.
  *Rejected:* deserialize-time key-presence tracking (a hand-written `Deserialize` on a
  security-relevant struct is its own drift surface); validation-rejection of overrides that omit
  the fields (fail-secure and cheap, but a behaviour break that diverges from how the other 12
  fields behave).

- **D-02: `upstream` stays required-on-the-child — deliberately, and the test says so.**
  It remains non-`Option` and always takes the child's value. It is the one field that identifies
  *what the credential talks to*; an override omitting it is far more likely a typo than an
  inheritance intent, and requiring it keeps every override block self-describing. This asymmetry
  must be asserted in the exhaustive test (D-03) with a comment marking it **intended, not
  overlooked** — otherwise a future reader re-files it as NEW-05's successor.

- **D-03: The regression test goes exhaustive over every merged field, replacing the ACC-04 patch.**
  Set a distinct non-default value for *all* fields on the base, give the child a minimal body that
  redefines only `upstream`, and assert every other field inherited. This closes ACC-04's missing
  `spiffe` assertion (`mod.rs:10188`, `:10238-10252`) **and** the 11 other unasserted `.or(base)`
  arms in one place, and it fails when a future field is added to `CustomCredentialDef` without a
  merge arm. Fix the test's inaccurate doc comment (`mod.rs:10166-10168`) while there.
  *Rejected:* adding the single `spiffe` assertion — that is the literal ACC-04 close and leaves
  the mechanism that produced ACC-04 intact.

- **D-04: `hooks.hooks` gets a compile-time guard, not a field-merge.**
  The audit named it "the place a future field addition would re-open NEW-02" — it is the last
  struct-valued `.extend()` map in `merge_profiles` (`mod.rs:~3820`), safe today only because
  `HookConfig` (`mod.rs:2003-2012`) has three required `String` fields and no serde defaults. Keep
  whole-value replace (correct for hooks), but add a structural guard — an exhaustive destructuring
  or match that **fails to compile** if `HookConfig` gains an optional or defaulted field.
  *Do not* field-merge it: that changes override semantics with no defect driving it.
  *Already cleared by the audit, do not re-audit:* `env_credentials.mappings` (`:3789`) and
  `environment.set_vars` (`:3804`) are `HashMap<String, String>` — scalar values, nothing to drop.

- **D-05: Correct the disputed doc comment.** `mod.rs:3557-3559` currently justifies the bug:
  `inject_mode`/`inject_header` are called "presentation fields that remove no security control."
  NEW-05 disputes this and this phase agrees — *where a credential is placed on the wire is not
  presentation*. Rewrite the comment to state the corrected rule and why, rather than deleting it.

### DRAIN-03 — the denial spine (NEW-01)

- **D-06: `log_denied` takes the denial category as a required argument.**
  DRAIN-03 exists because `EventContext` is `#[derive(Default)]` with `denial_category:
  Option<_>`, so a new call site defaults to uncategorised and nothing complains. Change
  `audit::log_denied`'s signature to take `NetworkAuditDenialCategory` as its own non-optional
  parameter (or split the denial fields out of the `Default`-derived struct) so **omitting it fails
  to compile**. This makes the class unrepresentable rather than merely tested-for.
  Touches all 28 production call sites; most are mechanical.
  *Rejected as the primary mechanism:* a self-enforcing source scan for
  `log_denied(.., &EventContext::default(), ..)`. It was seriously considered (it reuses the
  in-repo convention the milestone cites for RCPT-02) but a string matcher misses a call site that
  builds its context via a helper. The planner may add one as a cheap secondary if the signature
  change leaves any indirection unguarded.

- **D-07: Both host-check sites emit `HostDenied`, matching the four that already do.**
  `connect.rs:86` (`deny_domain`'s **HTTPS** enforcement point — the dominant path) and
  `external.rs:136` join `reverse.rs:365`, `reverse.rs:684`, `reverse.rs:977` and `server.rs:1146`.
  One policy decision, one category, across all six `deny_domain` dispatch paths — so a consumer
  filtering on `denial_category == HostDenied` finally sees all six. `ProxyMode` is already passed
  as its own argument, so no extra path/mode discriminator is needed in `EventContext`.

- **D-08: `external.rs:200` emits `ExternalProxyRejected`** — this is the site the audit identified
  as the one that *should* construct it, and wiring it retires one of the two dead variants.

- **D-09: `InterceptHandshakeFailed` is REMOVED, not reserved.**
  It has zero production constructors and **can never gain one** — the fork declines TLS
  interception by standing decision (ADR-113 D-01; `ProxyHandle::intercept_ca_path()` always
  returns `None`). A variant advertising an enforcement point the fork structurally does not have
  is the same over-claiming shape BOUND-01/BOUND-02 exist to close.
  **Precondition the planner must satisfy first:** persisted HMAC-chained audit ledgers must still
  verify. Either establish that no ledger can contain `intercept_handshake_failed` (it was never
  emitted — grep the audit path, and check any ledger fixtures/goldens in the repo), **or** give
  the decode path an explicit unknown-variant route so verification of historic data cannot break.
  Removal also ripples into `../nono-py`'s decoder — which under D-10 stops being hand-written
  anyway, so sequence D-09 with D-10.

### DRAIN-02 — the nono-py denial codec (NEW-06, blocker-class)

**Scouting finding that changes the fix:** `NetworkAuditDenialCategory` **already derives**
`Serialize, Deserialize` with `#[serde(rename_all = "snake_case")]`
(`crates/nono/src/undo/types.rs:242-244`). Both nono-py matches are re-implementing serde by hand.

- **D-10: Delete both hand-written matches; use the enum's own serde.**
  Replace the encoder (`../nono-py/src/proxy.rs:70-104`) and the decoder
  (`../nono-py/src/undo.rs:588-621`) with serde-driven conversion, so nono-py never holds its own
  copy of the vocabulary. The drift class stops existing rather than being tested for.
  **Hard precondition:** prove serde's `rename_all = "snake_case"` output is byte-identical to all
  current strings before switching, so no emitted category silently changes on the wire. Spot-check
  the awkward ones (`ConnectBypassesL7` → `connect_bypasses_l7`).
  *Rejected:* adding the two missing decoder arms plus an exhaustiveness test — that is DRAIN-02's
  literal text, but it leaves the hand-maintained list alive with only a tripwire attached.

- **D-11: Derive `strum::EnumIter` on the core enum so the round-trip test enumerates itself.**
  A hand-written `ALL` const would reintroduce the very drift class being closed. `strum` is a
  derive-only dependency with no policy content — consistent with ADR-86, which constrains what the
  library *decides*, not what it *derives*. The planner should confirm whether `strum` is already
  in the workspace tree before adding it; if adding it to the core crate proves contentious, the
  fallback is `pub const ALL: &[Self]` plus a private exhaustive-`match` guard that fails to
  compile when a variant is added.

- **D-12: The round-trip gate lives in `../nono-py`, backed by a real `maturin build`.**
  A Rust test in the binding crate (which owns the codec) running against the path-dep core enum —
  so `strum` iteration picks up a new core variant the moment nono-py rebuilds. **Only building
  catches binding struct drift** (D-14 of Phase 114; this is the fifth consecutive occurrence).
  **This forces `workflow.use_worktrees=false`** — the plan reaches a sibling repo via `..`.
  A Python-level pytest was considered and not required; the Rust test is the structural guarantee.
  `../nono-ts` needs nothing: it has no `nono-proxy` dependency (core `nono` only) and no
  `NetworkAuditDenialCategory` surface.

### DRAIN-04 / DRAIN-05 — class breadth

- **D-13: Reject *any* `aws_auth` route at config-validation time, not just `aws_auth` + `capture`.**
  `reverse.rs:328-331` returns 501 on `aws_route.is_some()` **unconditionally** — SigV4 is
  unimplemented on every path, so "make the pair work" was never available. If every `aws_auth`
  route 501s, then accepting one at load is exactly the "never silently degrade" violation DRAIN-04
  names, merely wider than it was written. Reject in `validate_custom_credential`
  (`mod.rs:~1147-1148`) with an error naming SigV4 as unimplemented.
  **Verified during discussion:** no shipped profile or fixture declares `aws_auth` — it appears
  only in `nono-profile.schema.json` descriptions (`:616`, `:629`, `:636`, `:638`, `:650`, `:657`)
  and in `network_policy.rs` test constructors (all `aws_auth: None`). **No profile fallout.**
  Existing unit tests that build `aws_auth`-bearing profiles and expect them to validate will need
  updating — that is expected, not a signal to reconsider.
  Update the schema descriptions to match, and record this in the divergence ledger: a future UPST
  absorb of real SigV4 must **un-reject** it.

- **D-14: Investigate plain OAuth2 `client_credentials`, and reject it on the same rule if
  confirmed unwired.** `credential.rs:296-300` states that plain `client_credentials`
  (`client_id`/`client_secret` with **no** `client_assertion`) is "intentionally left untouched —
  that flow's general route-wiring" is absent; only the SPIFFE jwt-bearer arm (RFC 7523) is
  implemented. That is the same validates-but-doesn't-work class as `aws_auth`. Establish at source
  whether such a route actually reaches upstream. If it does not, reject it at validation on the
  same terms as D-13 — one consistent rule ("config for an unimplemented mechanism is refused at
  load"), not two. If it does work, record that finding and leave it alone.
  **Do not guess in either direction** — this decision is explicitly conditional on evidence.

- **D-15: The Python `RouteConfig::new` exposes `spiffe` + `capture` + `endpoint_policy`, and
  deliberately excludes the unimplemented mechanisms.** Five fields are currently hardcoded `None`
  (`../nono-py/src/proxy.rs:228-232`, `:237`), not the two DRAIN-05 names. Expose those backed by a
  working implementation: `spiffe` (Phase 113), `capture` (Phase 114), and `endpoint_policy` if the
  planner confirms it is live. Keep `aws_auth` and `oauth2` hardcoded `None` — handing Python
  embedders config for a mechanism that 501s (or that D-13/D-14 is about to reject) would
  manufacture the exact DRAIN-04 defect on the Python side. **Record the exclusion as an intentional
  decision in code, not as an omission.** Both `spiffe` and `capture` need new PyO3 `#[pyclass]`
  wrapper types.

- **D-16: A test asserts no `RustRouteConfig` field is unconditionally `None`, with a named
  allowlist.** DRAIN-05 happened because a field can be added to `RustRouteConfig` and silently
  hardcoded `None` in the Python constructor — the exhaustive struct literal still compiles. Build a
  `RouteConfig` through the Python constructor with every parameter supplied, assert each `inner`
  field is non-`None`, and carry an explicit allowlist for the D-15 exclusions. A new field
  hardcoded `None` then fails the test unless someone adds it to the allowlist — a **visible
  decision** rather than silence.
  *Rejected:* a doc comment stating the invariant. A comment is precisely what failed here; Phase
  113 proved a fork's own "why is this missing" comment can be wrong and mislead for months.

### DRAIN-06 — SPIFFE mint ordering (NEW-08)

- **D-17: Hoist the filter host check above `managed_auth.acquire()`.**
  In `reverse.rs`, `managed_auth.acquire()` (`:634`) runs before the filter host check (`:684`), so
  a `deny_domain`-blocked upstream still triggers a live SPIRE Workload API fetch. No leak — the
  token never reaches the wire — but it inverts the ordering every other dispatch path uses.
  **Verified feasible during discussion:** the check's inputs (`upstream_url` from
  `route.upstream` + `upstream_path`, then `parse_upstream_url`) do **not** depend on `material`,
  so the reorder is mechanical. Pin it with a test asserting **no SPIRE fetch occurs** on a denied
  host — i.e. assert on the absence of the mint, not merely on the 403.

### Claude's Discretion

- Plan/wave decomposition across the four crates plus the sibling repo, and the commit sequencing
  between the `nono` and `nono-py` repos (both need DCO sign-off; nono-py commits separately).
- Whether D-09's ledger-compatibility check is its own task or folded into the removal.
- The exact `NonoError` variant and message wording for the D-13/D-14 validation rejections.
- Whether `endpoint_policy` is live enough to expose under D-15 (verify, then decide).
- The concrete shape of D-04's compile-time guard and D-16's allowlist mechanism.
- Whether a secondary source-scan test is worth adding alongside D-06's signature change.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### This phase's governing documents
- `.planning/ROADMAP.md` § "Phase 115: v3.6 Carry-Forward Drain" — goal and SC1–SC5.
  **Note:** SC1 describes DRAIN-01's fix as a `.or(base)` merge; D-01 records why that requires a
  struct-shape change first. SC1's outcome is unchanged — only the route to it is wider.
- `.planning/REQUIREMENTS.md` § "v3.6 Carry-Forward Drain (DRAIN)" (lines ~141-148) — DRAIN-01…06
  and the traceability rows (lines ~178-183). **Every line number cited there must be re-grepped by
  symbol before use** — the file records line numbers that have already drifted once
  (`proxy_runtime.rs:590` → `:591`).
- `.planning/REQUIREMENTS.md` § v3.7 "Architecture invariants" (lines ~103-113) — in particular
  *"Structural fixes over spot fixes for DRAIN-02/DRAIN-03"*, *"Executor self-check is not security
  evidence"*, and the two-open-milestones constraint (append, never overwrite; `phases.clear` must
  not run).

### The evidence base this phase drains
- `.planning/milestones/v3.6-MILESTONE-AUDIT.md` — **the primary source.** Read
  §"Cross-Phase Integration Findings" (NEW-01…NEW-04, ACC-01) and §"Re-Audit — second pass"
  (NEW-05…NEW-08, ACC-02/03/04, and the re-verified-at-source table at lines ~365-377).
  The re-audit section supersedes the first pass where they differ.
- `.planning/phases/114-oauth-capture-absorb-sec-02/114-VERIFICATION.md` — the risk-accepted
  residual table (WR-02, WR-03, WR-05, NEW-04). **Out of scope for this phase** — do not re-open.
- `.planning/phases/114-oauth-capture-absorb-sec-02/114-CONTEXT.md` — D-14 (binding rebuild
  discipline, `use_worktrees=false`), D-05/D-06 (capture's fail-closed shape).
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — where D-13's
  "a future SigV4 absorb must un-reject this" note lands.

### Standing decisions this phase relies on
- `proj/ADR-113-spiffe-disposition.md` § D-01 — the standing no-TLS-interception decision that
  makes `InterceptHandshakeFailed` permanently unconstructable (D-09's whole basis).
- `proj/ADR-114-oauth-capture-disposition.md` — capture's scope limit; the `aws_auth` 501 guard's
  origin (D-15 fork adaptation, `reverse.rs:323-331`).
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free-library boundary. D-11 adds a
  derive-only dependency to the core crate; confirm that reading holds.

### Boundary and standards
- `CLAUDE.md` — authoritative. Fail-secure, no `.unwrap()`/`.expect()`, path *component*
  comparison, checked arithmetic, and the library-vs-CLI boundary table.
- `.planning/templates/cross-target-verify-checklist.md` — single source of truth for the two
  cross-target clippy gates. Both remain MUST for any cfg-gated Unix edit; PARTIAL→CI is not the
  default.

### Code under change (verify by symbol, never by line number)
- `crates/nono-cli/src/profile/mod.rs` — `CustomCredentialDef` (`:963-1040`),
  `validate_custom_credential` (`:~1147-1194`), `merge_custom_credential_def` (`:3556-3588`),
  `merge_profiles` (`:3590+`), `HookConfig` (`:2003-2012`), the NEW-02 regression test
  (`:10166-10252`).
- `crates/nono-proxy/src/connect.rs:80-93`, `external.rs:130-143` and `:196-210`,
  `reverse.rs:318-331` (aws 501), `:355-378` (the categorised `HostDenied` template),
  `:625-700` (SPIFFE acquire-before-filter), `credential.rs:288-300` (the aws/oauth2 wiring gap).
- `crates/nono/src/undo/types.rs:241-262` — `NetworkAuditDenialCategory`.
- `crates/nono-cli/data/nono-profile.schema.json:616-660` — the credential `$defs` block.
- `../nono-py/src/proxy.rs:70-104` (encoder), `:186-241` (`RouteConfig::new`);
  `../nono-py/src/undo.rs:588-621` (decoder).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`reverse.rs:365-376` is the template for every denial-site fix.** It already builds the exact
  shape D-07 needs: `EventContext { route_id: Some(..), denial_category:
  Some(NetworkAuditDenialCategory::HostDenied), ..Default::default() }`. The three broken sites just
  pass `&audit::EventContext::default()` instead.
- **`NetworkAuditDenialCategory` already derives `Serialize, Deserialize` +
  `#[serde(rename_all = "snake_case")]`** (`undo/types.rs:242-244`) — this is what makes D-10
  possible without inventing a serialization format.
- **`merge_custom_credential_def`'s 12 existing `.or(base)` arms** are the shape D-01 extends to
  two more fields; the `endpoint_rules` empty-vec-inherits arm is the precedent for handling a
  non-`Option` field that still needs inheritance semantics.
- **`validate_custom_credential`'s mutual-exclusion arm** (`mod.rs:1147-1148`) is the established
  idiom D-13/D-14's rejections extend.
- **Phase 113's loud-skip convention** (`eprintln!("SKIP[{}]: reason", module_path!())` +
  `grep -c '^SKIP\['`) already exists — reuse it if any test ends up host-gated rather than
  inventing a second convention.

### Established Patterns
- **`../nono-py` builds an exhaustive `RustRouteConfig` struct literal** (`src/proxy.rs:220-241`,
  all 16 fields). A new *field* breaks its build loudly — but a field hardcoded `None` inside that
  literal is silent, which is exactly NEW-07/DRAIN-05. D-16 closes the silent half.
- **`../nono-ts` is structurally immune** — no `nono-proxy` dependency, core `nono` only. It needs
  no change in this phase; its last commit being the 111 bump is not a gap (audit ACC-02).
- **`deny_domain` is evaluated on all six dispatch paths** (`reverse.rs:360`, `:684`, `:977`,
  `server.rs:1143`, `connect.rs:80`, `external.rs:130`) — verified WIRED by the re-audit. This
  phase changes what those sites *report*, not whether they *fire*.
- **28 production `log_denied` sites exist; exactly 3 pass the default context.** The 4th
  `EventContext::default()` hit (`audit.rs:384`) is `#[cfg(test)]` — do not count it as a defect.
- **AWS SigV4 is a config surface with no implementation** — `credential.rs:289-295` registers the
  prefix with a `()` placeholder purely so `reverse.rs` can return 501. There is no partial
  implementation to preserve.

### Integration Points
- `audit::log_denied`'s signature (D-06) — the single choke point all 28 sites flow through.
- `crates/nono/src/undo/types.rs` → `../nono-py`'s codec (D-09 × D-10 × D-11) — the cross-repo edge
  where the drift lived. Sequence: enum change → serde codec → strum iteration → maturin build.
- `CustomCredentialDef`'s field types (D-01) → `nono-profile.schema.json` → `../nono-py`'s
  `RouteConfig::new` parameters (D-15). One struct change, three downstream surfaces.
- `validate_custom_credential` (D-13/D-14) → existing profile unit tests that expect `aws_auth` to
  validate. Expect breakage; it is the intended signal.

</code_context>

<specifics>
## Specific Ideas

- **The through-line of every decision in this phase is "make it unrepresentable, not tested-for."**
  D-06 (required argument over source scan), D-10 (delete the match over test the match), D-04
  (compile-time guard over merge), D-16 (allowlist test over doc comment) all resolved the same way.
  Where the planner faces an unlisted sub-choice, resolve it the same direction.
- **Two decisions are explicitly conditional on evidence and must not be pre-committed:**
  D-09's removal is gated on the persisted-ledger check, and D-14's rejection is gated on
  confirming plain `client_credentials` is genuinely unwired. Executing either without its evidence
  step is a deviation.
- **Verify by symbol, never by line number.** Phase 113 saw plans cite signatures a same-day
  sibling plan had already changed; the audit itself recorded a line drifting between passes
  (`proxy_runtime.rs:590` → `:591`). Every `file:line` in this document is a starting point for a
  grep, not an address.
- **`/gsd:code-review` is mandatory for this phase** — it is code-touching, and the milestone
  invariant is explicit: Phase 112's review gate caught 4 Critical fail-open defects that all 8
  executor self-checks passed over, and Phase 114 repeated it twice. Executor self-check is not
  security evidence.
- **`workflow.use_worktrees=false` is forced** by D-12/D-15/D-16 — the plan reaches `../nono-py`
  via a relative path, which a worktree breaks.
- **Two milestones are open.** v3.5 owns Phases 101–107 and is live. `REQUIREMENTS.md` and
  `ROADMAP.md` are appended to, never overwritten; SDK STATE writers stay banned; `phases.clear`
  must not run.

</specifics>

<deferred>
## Deferred Ideas

- **`spiffe_context` has no dict encoding in `audit_event_to_py_dict`** (noted in
  `../nono-py/src/undo.rs`, just past the decoder). Surfaced while scouting DRAIN-02. It is a
  separate binding-parity gap from the denial-category codec and was not folded in — file it if it
  is not already tracked.
- **Implementing AWS SigV4 signing.** D-13 rejects its config precisely because it is unimplemented.
  Building it is a real feature, not a drain item — and a future UPST absorb may bring upstream's
  implementation, at which point D-13's rejection must be lifted.
- **Implementing plain OAuth2 `client_credentials` route-wiring.** Same shape as SigV4: D-14 may
  reject its config; building the flow is separate work.
- **Switching `../nono-py` to `..Default::default()`** to end the recurring struct-drift build
  break. Raised in Phase 113 and rejected there (it trades a loud compile error for silent field
  adoption on a security-relevant struct); D-16 addresses the *silent-None* half of the problem
  without reopening that decision.
- **The risk-accepted v3.6 residuals** — WR-02 (case-sensitive backstop), WR-03 (trailing-dot host),
  WR-05 (route-sharing), NEW-04 (`direct_connect_ports` voids the capture guarantee). Consciously
  accepted at the v3.6 close with recorded reasoning (`114-VERIFICATION.md`). Not reopened here.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` (score 0.6) — matched only on generic keywords ("clean",
  "phase"). A v3.5 host-gated MSI/VC++ redistributable item, unrelated to this drain. Not folded.
  (Phases 113 and 114 reviewed and declined the same item.)
- `20260611-poc-cert-broker-clean-host.md` (score 0.6) — same; a v3.5 clean-host POC-cert broker
  item, blocked on Phase 106's Azure VM. Not folded.

</deferred>

---

*Phase: 115-v3-6-carry-forward-drain*
*Context gathered: 2026-08-08*
