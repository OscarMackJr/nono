# Phase 114: OAuth Capture Absorb (SEC-02) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-06
**Phase:** 114-oauth-capture-absorb-sec-02
**Areas discussed:** The disposition verdict, What satisfies SC2's bar, Decline permanence + re-entry, The freestanding 391-line piece

> **Note on structure:** the verdict was decided, then **retracted and re-decided** mid-session at
> the operator's instruction. Both passes are recorded below in order. The retracted branch is kept
> because the evidence that overturned it is the most consequential finding of the discussion.

---

## The disposition verdict — FIRST PASS (retracted)

| Option | Description | Selected |
|--------|-------------|----------|
| Formally DECLINE | Record adopt-vs-adapt-vs-decline as DECLINE with cited evidence, ADR-113 OD-1 / ADR-112 shape. Zero code under `crates/`. | ✓ (later retracted) |
| ADAPT — build a fork-native enforcement point | Decline TLS interception, design a fork-side response buffer+rewrite on the reverse-proxy path. | |
| ADOPT — take 149abde0 too | Reverse the no-MITM decision, absorb ~974-line prerequisite + 4,425 lines + 7 dependencies. | |
| Defer again — needs its own milestone | Push SEC-02 to v3.7; contradicts ROADMAP SC1. | |

**User's choice:** Formally DECLINE — subsequently retracted.
**Notes:** Orchestrator surfaced that `149abde0` (which creates `forward.rs`, home of SC2's
`ResponseRewrite` hook) is the TLS-interception commit: proxy terminates TLS, generates an ephemeral
session CA, injects a trust bundle into the sandboxed child (`SSL_CERT_FILE`/`REQUESTS_CA_BUNDLE`),
adds `CapabilitySet::allow_file_mut`, and pulls 7 new dependencies. The fork declines TLS
interception by standing decision.

---

## What satisfies SC2's bar — FIRST PASS (applied to the decline branch)

| Option | Description | Selected |
|--------|-------------|----------|
| Positive no-partial-absorb proof | Prove no oauth_capture symbols / credential_provider / schema fields leaked in — a schema that accepts config while nothing enforces it is the fail-open shape. | ✓ |
| Re-confirm the hook claim at symbol level | Re-run Phase 112's evidence against today's tree and the full diff. | |
| Both | Carry both obligations. | |

**User's choice:** Positive no-partial-absorb proof.

### Follow-up: how to discharge it

| Option | Description | Selected |
|--------|-------------|----------|
| Assert it with a live test | Feed a profile with upstream's OAuth-capture config block, assert REJECTED. Standing regression guard. | ✓ |
| Grep-verified assertion only | Literal grep commands asserted in the ADR (112-07 / ADR-113 D-01 precedent). Point-in-time. | |
| Grep now, test only if a gap is found | Escalate conditionally. | |

**User's choice:** Assert it with a live test.

### Follow-up: SC3/SC4 treatment on a test-only diff

| Option | Description | Selected |
|--------|-------------|----------|
| SC3 vacuous-and-say-so; run SC4 anyway | Record SC3 as trivially satisfied *because* nothing entered the core crate, with a grep proving zero `crates/nono/` diff; run both gates regardless since `--all-targets` covers test files. | ✓ |
| SC3 vacuous; SC4 via zero-crates-diff evidence chain | Reuse 112-08's precedent. | |
| Amend the ROADMAP SCs to match a decline | Operator-level change. | |

**User's choice:** SC3 vacuous-and-say-so; run SC4 anyway.
**Notes:** All three of these answers were superseded when the verdict was retracted — under ADAPT,
SC3 is live and the proof obligations change shape. Retained here for the audit trail.

---

## Decline permanence + re-entry

| Option | Description | Selected |
|--------|-------------|----------|
| Conditional — gated on TLS interception | Decline stands UNLESS the fork ever adopts TLS interception; names the single unblocking condition. | ✓ |
| Permanent — ADR-113 OD-1 shape | Blanket, permanent out-of-scope declaration. | |
| Conditional, with the enforcement bar restated | Conditional + pre-commit the safety bar for any future attempt. | |

**User's choice:** Conditional — gated on TLS interception.

### Follow-up: where the re-entry obligation gets filed (multi-select)

| Option | Description | Selected |
|--------|-------------|----------|
| 108-DIVERGENCE-LEDGER carry-forward note | Canonical work-list a future UPST planner actually reads. | ✓ |
| ADR-114 itself | Document of record, SC1-mandated regardless. | |
| REQUIREMENTS.md SEC-02 disposition | Resolve the checkbox rather than leaving it ambiguous. | |
| A code comment at the would-be integration point | Discoverable in place — but Phase 113 proved comments drift and mislead. | |

**User's choice:** 108-DIVERGENCE-LEDGER carry-forward note (only).
**Notes:** Orchestrator flagged a tension — Phase 112 left SEC-02's checkbox unchecked *specifically*
pending this phase, so declining without resolving it would leave SEC-02 open with no owner and
block a clean v3.6 close.

### Follow-up: SEC-02 requirement status

| Option | Description | Selected |
|--------|-------------|----------|
| Resolve as won't-sync citing ADR-114 | A declined requirement with recorded reasoning IS resolved — how SEC-01/SEC-09 closed in Phase 112. | ✓ |
| Leave unchecked, note it points to ADR-114 | Preserves visibility that the capability is absent. | |
| Ledger only — don't touch REQUIREMENTS.md | Smallest footprint; SEC-02 reads as still-pending after 114 ruled on it. | |

**User's choice:** Resolve as won't-sync citing ADR-114 — later superseded by D-11
(satisfied-with-scope-limit) once the verdict flipped to ADAPT.

---

## THE RETRACTION

At the area-transition checkpoint the user replied, in free text:

> **"Phase 114 needs to resolve sec-02. let's revisit"**

Orchestrator did not presume the intent and asked which reading applied:

| Option | Description | Selected |
|--------|-------------|----------|
| Reopen the verdict — decline isn't enough | The CAPABILITY must be delivered, not just adjudicated. | ✓ |
| Verdict stands — tighten how the decline resolves it | Keep DECLINE, sharpen closure obligations. | |
| Something else — let me explain | Free-text. | |

**User's choice:** Reopen the verdict.

**Investigation triggered by the retraction — the pivotal finding:** the orchestrator inspected the
fork's response relay path and found the reverse proxy **already has plaintext visibility into
upstream responses** — the sandboxed client speaks plain HTTP to the proxy, and the proxy holds its
own `tls_stream` to upstream. MITM was never required for token endpoints reachable as configured
routes. The real blocker was **buffering**, not visibility: the fork streams responses in 8 KiB
chunks across three relay sites, deliberately, for SSE / MCP Streamable HTTP / A2A JSON-RPC.

This invalidated the premise underlying the first-pass decline — and underlying
`112-OAUTH-CAPTURE-DISPOSITION.md`'s security conclusion, which reads as though `forward.rs` were
the only possible enforcement point.

---

## The disposition verdict — SECOND PASS (binding)

| Option | Description | Selected |
|--------|-------------|----------|
| ADAPT — fork-native rewrite, reverse-proxy scope | Bounded opt-in response buffer+rewrite; satisfies SC2 by construction; no MITM; must cover all three relay sites and fail closed on paths it cannot reach. | ✓ |
| ADOPT — take 149abde0 and TLS interception | Widest capability; installs a proxy-controlled CA in the sandboxed child's trust store. | |
| ADAPT now, TLS interception as its own future phase | Same rewrite + a named future phase for the wider question. | |

**User's choice:** ADAPT — fork-native rewrite, reverse-proxy scope. → **D-01r / D-02r**

### Follow-up: buffer bound and failure mode

| Option | Description | Selected |
|--------|-------------|----------|
| Tight cap, fail CLOSED on exceed | ~256 KiB–1 MiB; deny oversized responses rather than release unrewritten. | ✓ |
| Tight cap, pass through unrewritten on exceed | Explicitly fail-open. | |
| Reuse the existing 16 MiB limit | Consistency; buys nothing security-wise, widens DoS surface. | |

**User's choice:** Tight cap, fail CLOSED. → **D-05**

### Follow-up: scoping and uncovered paths

| Option | Description | Selected |
|--------|-------------|----------|
| Opt-in per route + D-03-style fail-closed | Buffer only declared token-endpoint routes; deny capture-declared routes arriving via CONNECT / forward-HTTP / external-proxy chain. | ✓ |
| Opt-in per route only | No cross-path guard — leaves the WR-13 hole. | |
| Opt-in, plus content-type heuristic | Heuristics on a security boundary. | |

**User's choice:** Opt-in per route + D-03-style fail-closed. → **D-06**

### Follow-up: how much of the 26-file surface

| Option | Description | Selected |
|--------|-------------|----------|
| Port the config/rewrite logic, rebuild the plumbing | Upstream's config/JWT/mapping logic as design reference; enforcement rebuilt against `reverse.rs`. Phase 113's proven pattern. | ✓ |
| Minimal slice — in-memory only, no provider config | Smallest thing satisfying SC2. | |
| Full adapt — all 17 present-file hunks plus rebuilt core | Closest parity, largest review surface. | |

**User's choice:** Port the config/rewrite logic, rebuild the plumbing. → **D-07**

### Follow-up: persistence of real tokens

| Option | Description | Selected |
|--------|-------------|----------|
| In-memory, session-scoped only | Zeroized on drop; no secret at rest; re-login after restart. | ✓ |
| Persist with bounded retention, owner-only perms | Better UX for long-running agents; adds secret-at-rest surface. | |
| In-memory now, persistence as a named follow-on | Split the hard call. | |

**User's choice:** In-memory, session-scoped only. → **D-08**

---

## The freestanding 391-line piece

| Option | Description | Selected |
|--------|-------------|----------|
| Port adapted, minus absent-subsystem coupling | Bring the declarative provider surface adapted to the fork's profile shape; drop `oauth_capture/`/`forward.rs` reach-through. | ✓ |
| Port only the fields the rewrite consumes | Minimum config; plainer hand-written configuration story. | |
| Defer it — hardcode config in the route | Smallest phase; under-delivers "declarative" in SEC-02's own title. | |

**User's choice:** Port adapted, minus absent-subsystem coupling. → **D-12**

### Follow-up: the fork-only strict schema

| Option | Description | Selected |
|--------|-------------|----------|
| First-class task with a round-trip test | Hand-port `$defs`/properties + assert a realistic profile both deserializes AND passes `validate_against_schema()`. | ✓ |
| Hand-port the schema, rely on existing profile tests | Less new test code; mismatch may go unnoticed. | |
| Port upstream's 265 schema lines wholesale | Advertises config nothing implements — fail-open shape. | |

**User's choice:** First-class task with a round-trip test. → **D-13**

### Follow-up: the `nono-py` binding break

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit task + rebuild both bindings | Named task; `maturin build` + `napi build` with real results reported. Forces `use_worktrees=false`. | ✓ |
| Fold it into the config task | Tends to be discovered at phase end — how it has slipped four times. | |
| Switch nono-py to `..Default::default()` | Root fix; rejected in Phase 113 as trading a loud error for silent field adoption. | |

**User's choice:** Explicit task + rebuild both bindings. → **D-14**

---

## Claude's Discretion

- Exact buffer cap value within D-05's range, and whether it is configurable.
- The phantom-token minting/resolution mechanism and how it composes with the fork's existing
  outbound-direction phantom machinery.
- Audit-record contents — hard constraint: never a raw token.
- Whether `nono-cli` needs a helper command to drive a login flow.
- Plan/wave decomposition; whether ADR-114 is its own plan or a wave.

## Deferred Ideas

- Disk persistence of real OAuth tokens (upstream `oauth_capture/persist.rs`) — needs its own
  security review; secret-at-rest is a separate decision from the enforcement-point decision.
- TLS interception / `149abde0` — the named condition under which SEC-02's scope limit could lift.
- Switching `nono-py` to `..Default::default()` — rejected in Phase 113, still its own scoped change.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` (0.6) — generic keyword match; v3.5 host-gated MSI item.
- `20260611-poc-cert-broker-clean-host.md` (0.6) — generic keyword match; v3.5 clean-host POC-cert item.
