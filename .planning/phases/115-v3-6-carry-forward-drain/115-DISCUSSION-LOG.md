# Phase 115: v3.6 Carry-Forward Drain - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-08
**Phase:** 115-v3-6-carry-forward-drain
**Areas discussed:** Inject-field inheritance mechanism, Dead denial variants, nono-py drift-proofing, Class breadth for DRAIN-04/DRAIN-05

---

## Gray-area selection

| Option | Description | Selected |
|--------|-------------|----------|
| Inject-field inheritance mechanism | DRAIN-01: serde-defaulted concrete types make "omitted" indistinguishable from "explicitly default" at merge time | ✓ |
| Dead denial variants: remove vs reserve | DRAIN-03: two zero-constructor variants; `InterceptHandshakeFailed` structurally unreachable | ✓ |
| nono-py drift-proofing mechanism | DRAIN-02: hand-written encoder + decoder duplicating serde | ✓ |
| Class breadth for DRAIN-04 / DRAIN-05 | Fix the named symptom or the whole class, in two places | ✓ |

**User's choice:** all four.
**Notes:** Presented alongside five scouting findings that changed the shape of the work — most
consequentially that DRAIN-01's stated `.or(base)` fix is not directly expressible without a
struct-shape change, and that DRAIN-02 has a stronger available fix than the requirement asks for.

---

## Inject-field inheritance mechanism (DRAIN-01)

### Q1 — How to make `.or(base)` expressible for `inject_mode`/`inject_header`

| Option | Description | Selected |
|--------|-------------|----------|
| `Option<T>` + resolve at use site | Drop serde defaults from the struct; `.unwrap_or_default()` / `.unwrap_or_else(default_inject_header)` where consumed. Homogeneous merge. Cost: every consumer + schema + nono-py constructor | ✓ |
| Track key-presence at deserialize | Custom `Deserialize` or `#[serde(skip)]` sidecar recording literally-present keys. No consumer churn; generalizes. Cost: hand-written Deserialize on a security-relevant struct is its own drift surface | |
| Reject the ambiguity at validation | Make it an error for an override to redefine a credential without restating both fields. Fail-secure, cheapest. Cost: behaviour break; diverges from the other 12 fields | |

**User's choice:** `Option<T>` + resolve at use site.

### Q2 — Does the phase cover `hooks.hooks`?

| Option | Description | Selected |
|--------|-------------|----------|
| Compile-time guard, don't merge | Keep whole-value replace; add a guard that fails to compile if `HookConfig` gains an optional/defaulted field | ✓ |
| Field-merge it too | Uniform treatment for every struct-valued map. Cost: changes hook override semantics with no defect driving it | |
| Out of scope — note it only | Record as a future-reopen point. Cost: the "patch the arm, reproduce the class" pattern the milestone warns against | |

**User's choice:** compile-time guard.

### Q3 — How far does the NEW-02 / ACC-04 regression test go?

| Option | Description | Selected |
|--------|-------------|----------|
| Exhaustive over every merged field | Distinct non-default on all fields of the base, minimal child, assert every field inherited. Fails when a 16th field is added without a merge arm | ✓ |
| Add the spiffe assertion only | Literal ACC-04 close; minimal diff. Leaves the other 11 arms unasserted — which is how ACC-04 happened | |
| You decide | Let the planner size it | |

**User's choice:** exhaustive.

### Q4 — Should `upstream` become inheritable?

| Option | Description | Selected |
|--------|-------------|----------|
| Keep upstream required on the child | It identifies what the credential talks to; an omission is more likely a typo than intent. Asymmetry asserted deliberately in the test | ✓ |
| Make upstream inheritable too | Fully uniform, no exceptions to explain. Cost: a malformed override silently points at the base's host | |
| You decide | Let the planner check post-merge validation behaviour first | |

**User's choice:** keep required on the child.

---

## Dead denial variants (DRAIN-03)

### Q1 — `InterceptHandshakeFailed`: remove or reserve?

| Option | Description | Selected |
|--------|-------------|----------|
| Remove it — with a decode fallback | Delete the variant; it advertises an enforcement point the fork structurally lacks. Requires establishing that persisted HMAC-chained ledgers still verify | ✓ |
| Keep it, documented as unreachable | Doc comment citing ADR-113 D-01. Zero ledger/codec risk. Cost: satisfies DRAIN-03's criterion only by rewriting it | |
| You decide | Planner decides after checking persisted-ledger exposure | |

**User's choice:** remove, with the ledger check as a precondition.

### Q2 — Categories for the two host-check sites

| Option | Description | Selected |
|--------|-------------|----------|
| Both `HostDenied`, matching the other four paths | One policy decision, one category across all six `deny_domain` dispatch paths | ✓ |
| `HostDenied` + a path/mode distinction | Extra `EventContext` discriminator. Likely redundant — `ProxyMode` is already a separate argument | |
| You decide | Confirm the denial semantics really are identical first | |

**User's choice:** both `HostDenied`. (`external.rs:200` → `ExternalProxyRejected` was presented as
already settled, wiring the second dead variant.)

### Q3 — What stops the 29th call site repeating this?

| Option | Description | Selected |
|--------|-------------|----------|
| Make the category a required argument | Signature change so omitting it fails to compile. Class becomes unrepresentable. Cost: 28 mechanical call-site edits | ✓ |
| Self-enforcing source scan test | Grep-based test, reuses the in-repo convention. Cost: a string matcher misses helper-built contexts | |
| Both | Belt and braces | |

**User's choice:** required argument. (Planner may still add a scan as a cheap secondary.)

---

## nono-py drift-proofing (DRAIN-02)

### Q1 — How to close the codec drift

| Option | Description | Selected |
|--------|-------------|----------|
| Delete both matches, use serde | Enum already derives `Serialize/Deserialize` + `rename_all = "snake_case"`; nono-py stops holding its own vocabulary. Precondition: prove byte-identical output | ✓ |
| Keep the matches, add the exhaustiveness test | DRAIN-02's literal text. No wire-format risk; explicit match stays greppable. Cost: hand-maintained list survives with a tripwire | |
| You decide | Verify snake_case output against the 11 strings first, then pick | |

**User's choice:** delete both matches.

### Q2 — How to enumerate every variant for the test

| Option | Description | Selected |
|--------|-------------|----------|
| `strum::EnumIter` on the core enum | Iteration generated from the definition; zero maintenance. Derive-only dep, consistent with ADR-86 | ✓ |
| `ALL` const + compile-time exhaustive guard | No new dependency, greppable list. Cost: keeping the guard and `ALL` in lockstep is still a two-step | |
| You decide | Check whether `strum` is already in the tree | |

**User's choice:** `strum::EnumIter`. (`ALL` + guard retained in CONTEXT.md as the fallback if
adding the dep to the core crate proves contentious.)

### Q3 — Where the round-trip gate lives

| Option | Description | Selected |
|--------|-------------|----------|
| Rust test in nono-py + real `maturin build` | Test lives with the codec, runs against the path-dep core enum. Only building catches binding drift (5th consecutive occurrence). Forces `use_worktrees=false` | ✓ |
| Python-level pytest as well | Tests the surface the embedder actually touches. Cost: needs the wheel installed in a test env | |
| Both | Structural guarantee plus embedder-facing behaviour | |

**User's choice:** Rust test + maturin build.

---

## Class breadth for DRAIN-04 / DRAIN-05

### Q1 — How wide is the `aws_auth` validation rejection?

| Option | Description | Selected |
|--------|-------------|----------|
| Reject any `aws_auth` route at validation | SigV4 501s on every path, so accepting one at load is the same "never silently degrade" violation, wider than written. Closes the class | ✓ |
| Reject only `aws_auth` + `capture` | DRAIN-04's literal scope. Cost: an `aws_auth`-alone route still validates and still 501s | |
| You decide | Check for shipped profiles/fixtures first | |

**User's choice:** reject any `aws_auth` route.
**Notes:** Verified live during discussion — `aws_auth` appears only in `nono-profile.schema.json`
descriptions and in `network_policy.rs` test constructors (all `aws_auth: None`). No shipped
profile is affected, which removed the main risk in this option.

### Q2 — Is plain OAuth2 `client_credentials` in scope?

| Option | Description | Selected |
|--------|-------------|----------|
| Investigate, then reject if confirmed unwired | One consistent rule for unimplemented mechanisms rather than two. Explicitly evidence-gated | ✓ |
| Reject `aws_auth` only; record oauth2 as a finding | Bounded diff, one behaviour break instead of two. Cost: ships an inconsistent rule | |
| Out of scope entirely | Don't investigate, don't file | |

**User's choice:** investigate, then reject if confirmed.
**Notes:** Surfaced from `credential.rs:296-300` — plain `client_credentials` is "intentionally left
untouched"; only the SPIFFE jwt-bearer (RFC 7523) arm is implemented.

### Q3 — How wide does the Python constructor go?

| Option | Description | Selected |
|--------|-------------|----------|
| `spiffe` + `capture` + `endpoint_policy` | Expose only what's backed by an implementation; excluding `aws_auth`/`oauth2` avoids manufacturing the DRAIN-04 defect on the Python side | ✓ |
| All five — full struct parity | Complete mirror, no second list to drift. Cost: hands embedders an unimplemented mechanism | |
| Exactly the two DRAIN-05 names | Literal requirement, minimal PyO3 surface | |

**User's choice:** `spiffe` + `capture` + `endpoint_policy`, exclusions recorded as intentional.

### Q4 — What stops the next hardcoded `None`?

| Option | Description | Selected |
|--------|-------------|----------|
| Test asserting no field is unconditionally `None` | Named allowlist for deliberate exclusions; a new hardcoded field fails until someone adds it — a visible decision | ✓ |
| Doc comment naming the invariant | Zero machinery. Cost: a comment is exactly what failed here (Phase 113 precedent) | |
| You decide | Decide once the parameter list is settled | |

**User's choice:** the allowlist test.

---

## Not discussed

**DRAIN-06** (SPIFFE JWT-SVID minted before the deny check) was accepted as mechanical without a
question round: hoist the filter host check above `managed_auth.acquire()` in `reverse.rs`, with a
test asserting no SPIRE fetch occurs on a denied host. Feasibility was verified during scouting —
the check's inputs do not depend on the acquired material.

## Claude's Discretion

- Plan/wave decomposition across four crates plus the sibling repo; cross-repo DCO commit sequencing.
- Whether D-09's ledger-compatibility check is its own task or folded into the removal.
- `NonoError` variant and message wording for the D-13/D-14 rejections.
- Whether `endpoint_policy` is live enough to expose under D-15.
- The concrete shape of D-04's compile-time guard and D-16's allowlist mechanism.
- Whether a secondary source-scan test is worth adding alongside D-06's signature change.

## Deferred Ideas

- `spiffe_context` has no dict encoding in `audit_event_to_py_dict` (`../nono-py/src/undo.rs`) —
  surfaced while scouting DRAIN-02; a separate binding-parity gap.
- Implementing AWS SigV4 signing (D-13 rejects its config precisely because it is unimplemented).
- Implementing plain OAuth2 `client_credentials` route-wiring.
- Switching `../nono-py` to `..Default::default()` — raised and rejected in Phase 113; D-16 closes
  the silent-`None` half without reopening it.
- The risk-accepted v3.6 residuals: WR-02, WR-03, WR-05, NEW-04.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` (0.6) — generic keyword match; a v3.5 host-gated MSI/VC++ item.
- `20260611-poc-cert-broker-clean-host.md` (0.6) — same; v3.5 clean-host POC-cert item, blocked on
  Phase 106's Azure VM.
