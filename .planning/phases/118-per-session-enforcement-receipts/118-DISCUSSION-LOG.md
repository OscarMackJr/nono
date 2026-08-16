# Phase 118: Per-Session Enforcement Receipts - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-16
**Phase:** 118-per-session-enforcement-receipts
**Areas discussed:** Emission completeness & abort semantics, Delivery channel + how a consumer verifies, Which chain and the receipt's shape, Cross-binary coverage (broker + daemon)

**Blocking anti-patterns acknowledged before discussion** (from `.planning/.continue-here.md`,
per the `check_blocking_antipatterns` gate): multi-command `run:` on a Windows CI job, and scoping
a fix to one file when the contract spans two. Both were answered on all three required questions
and both were judged directly applicable to this phase rather than ceremonial — see CONTEXT.md
`<domain>` and `<specifics>`.

---

## Emission completeness & abort semantics

### Census scope — stated as a forced reading, not put to a vote

Presented as already determined by the requirements rather than open: RCPT-01 says "every confined
session", and RCPT-03 requires a reader to identify "not expected in this configuration", which is
only possible if `NotApplicable` rows are physically present. Therefore a full census of all 13
registry rows, on every session. The user was invited to dispute this reading and did not.

### Abort semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — same receipt, terminal outcome | One receipt type with a session-outcome field (ran / refused). An abort names the failed layer AND the state of the other 12, and makes "nono refused" distinguishable from "nono never ran" / "nono crashed". | ✓ |
| Yes — but a distinct record type | A separate "session refused" record, so "receipt" always means a session that ran. Cleaner semantics, two shapes to keep in sync. | |
| No — aborts are already covered | Aborts surface as a typed `NonoError` + `NonoDiagnosticCode` (D-22) plus the operator channel. Cost: absence of a receipt carries meaning — the green-by-absence shape. | |

**User's choice:** Same receipt, terminal outcome.
**Notes:** Became D-02. The decisive framing was that without an abort receipt, "nono refused this
session" and "nono never ran" are indistinguishable to a governance consumer.

### Emitter fail-direction

| Option | Description | Selected |
|--------|-------------|----------|
| Degrade by default, policy can require | Proceed but record the failure (`TelemetryDegraded` + banner) and downgrade the claim — never silently. An `HKLM\SOFTWARE\Policies\nono` field can make receipts mandatory fleet-wide. Mirrors D-26 and the shipped egress-aborts/telemetry-degrades split. | ✓ |
| Always fail-closed — no receipt, no session | Strongest reading of "every confined session emits". Cost: diverges from shipped telemetry posture and turns an observability failure into an availability failure. | |
| Degrade only — no policy knob | Least new surface. Cost: an enterprise operator cannot make receipts a hard requirement. | |

**User's choice:** Degrade by default, policy can require.
**Notes:** Became D-04. Grounded in the existing `machine_policy.rs` precedent rather than a new
posture.

### Write point

| Option | Description | Selected |
|--------|-------------|----------|
| Once, at the gate before `ResumeThread` | Complete and durable before the child executes an instruction; matches D-20's startup-only claim; survives a supervisor kill; "refused" falls out as the same write point with a different terminal value. | ✓ |
| Open at the gate, close at session end | Richer (duration, exit status). Cost: dangling unclosed receipts on a supervisor kill, and it implies a mid-session claim D-20 does not make. | |
| Once, at session end | Complete picture. Cost: nothing exists if the supervisor is killed — the case where evidence matters most. | |

**User's choice:** Once, at the gate before `ResumeThread`.
**Notes:** Became D-03.

### Identity — does "content-free" forbid path-derived values?

| Option | Description | Selected |
|--------|-------------|----------|
| Strict: opaque session id + pid only | No path-derived value, not even salted. Makes SC2 provable via a TYPE ALLOWLIST scan; admitting hashes turns the scan into a judgment call about which hashes are OK. | ✓ |
| Allow the existing salted `path_hash` + `PathCategory` | Reuses the shipped telemetry shape for workspace grouping; in-tree precedent. Cost: "no paths, no arguments, no payload content" becomes a claim with an asterisk. | |
| Opaque id + a separate non-reversible workspace tag | Correlation without path derivation. Cost: a new identifier concept and its lifecycle. | |

**User's choice:** Strict.
**Notes:** Became D-05, and directly enabled D-14's type-allowlist mechanism.

---

## Delivery channel + how a consumer verifies

### Live measurement taken mid-discussion (not a question)

117 left an open manual-verification item on whether a Low-IL/AppContainer child can read the
Application Event Log. Its prescribed remediation was a read-only command, so it was **run** rather
than discussed hypothetically — consistent with the prior session's theme that every stale gate
surfaced only by actually running it.

`wevtutil gl Application` returned:

```
channelAccess: O:BAG:SYD:(A;;0x2;;;S-1-15-2-1)
  (A;;0x2;;;S-1-15-3-1024-3153509613-960666767-3724611135-2725662640-12138253-543910227-1950414635-4190290187)
  (A;;0xf0007;;;SY)(A;;0x7;;;BA)(A;;0x7;;;SO)(A;;0x3;;;IU)(A;;0x3;;;SU)
  (A;;0x3;;;S-1-5-3)(A;;0x3;;;S-1-5-33)(A;;0x1;;;S-1-5-32-573)
```

Result: `S-1-1-0` (Everyone) absent; `ALL APPLICATION PACKAGES` write-only — so 117's stated worry
measured **clean**. But `INTERACTIVE` (`S-1-5-4`) and `WRITE RESTRICTED` (`S-1-5-33`) both carry
`0x3`, which includes read, and no `S:` mandatory-label section is present. **The assumption is not
cleared — the plausible read path moved from the package SID to `INTERACTIVE`.** Not proven either
way: whether the child token retains `IU` post-construction, and whether the Event Log service
honors these ACEs as a raw object ACL check would.

### Where per-layer detail lands

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated operator-ACL'd receipt sink | Own retrievable store; Event Log demoted to a coarse pointer. Sidesteps the D-28 reconnaissance question instead of betting on an unproven ACL reading, and gives RCPT-02's consumer something retrievable. | ✓ |
| Prove it empirically first, then decide | Spawn a real confined child and have it attempt the read. Cost: the answer gates the design, and a negative result forces the redesign anyway. | |
| Event Log primary, coarse wording only | Apply 117's stated remediation as written. Cheapest and fail-safe. Cost: a receipt that cannot name layers would gut RCPT-03. | |

**User's choice:** Dedicated operator-ACL'd receipt sink.
**Notes:** Became D-06/D-07. The empirical question was preserved as a deferred item rather than
being closed by the routing decision.

### What keeps the child out of the sink

| Option | Description | Selected |
|--------|-------------|----------|
| DENY ACE on session+package SID, plus `NO_READ_UP` label | Defense in depth across all three token arms, reusing machinery nono already owns. Neither alone covers every arm. | ✓ |
| Mandatory `NO_READ_UP` label only | One mechanism, simplest to reason about. Cost: fails open for any arm where the child is not actually below the sink's IL — the broker grandchild is exactly the unproven case. | |
| Route writes through an elevated service | Strongest tamper-evidence. Cost: the existing elevated service is daemon-path only, so this would fail RCPT-01's "every confined session" without a new always-on service. | |

**User's choice:** DENY ACE + `NO_READ_UP`.
**Notes:** Became D-08. The framing that drove it: supervisor and child share a user identity, so a
plain DACL cannot separate them; and `WRITE_RESTRICTED` tokens check restricting SIDs on **writes
only**, so the restricted-token arm is not self-protecting for reads.

### Who is the "downstream governance consumer"

| Option | Description | Selected |
|--------|-------------|----------|
| Same trust domain — HMAC key holder | RCPT-02's literal wording; keeps the phase bounded. Third-party verification named as a deferred item rather than silently absorbed. | ✓ |
| Third-party verifiable — signed chain head | Uses the core audit module's existing sigstore sign/verify. Stronger governance story. Cost: key management and real scope growth. | |
| Both — HMAC continuity + signed checkpoint | Best of both; merkle/inclusion-proof code already exists. Largest scope. | |

**User's choice:** Same trust domain.
**Notes:** Became D-09; the alternative was preserved as a deferred item handed to Phase 119.

### Command surface

| Option | Description | Selected |
|--------|-------------|----------|
| New `nono receipt list\|show\|verify` family | Keeps the chain domains visibly separate so a consumer knows which integrity claim they verified. Reuses the shipped `AuditCommands` shape verbatim. | ✓ |
| Extend `nono audit` to cover receipts | Zero new surface; operators know the verbs. Cost: implies receipts live in the audit ledger, contradicting RCPT-02 and merging two deliberately independent chains. | |
| On-disk format only, no new commands | Smallest footprint. Cost: "a consumer can verify" becomes "a consumer could write a verifier" — the untested-gate shape. | |

**User's choice:** New `nono receipt` family.
**Notes:** Became D-10.

---

## Which chain, and the receipt's shape

### Chain choice

Framed with a technical constraint stated up front: an HMAC chain commits each entry to
`prev_head`, so receipts and telemetry events interleaved on one chain but landing in two sinks
would make the receipt sink unverifiable on its own. "Same terms as" therefore most likely means
same construction and discipline, not the same chain instance — the Phase 115 lesson (a
requirement's prescribed mechanism can be un-implementable as literally written) in a new place.

| Option | Description | Selected |
|--------|-------------|----------|
| Own chain domain, same construction | Third domain constant alongside `TELEMETRY_CHAIN_DOMAIN` and the core audit `CHAIN_DOMAIN`, identical `Hmac<Sha256>` construction and advance-under-mutex discipline. Sink is self-contained and independently verifiable. | ✓ |
| Ride the existing telemetry chain | Literal reading of RCPT-02; zero new crypto surface. Cost: receipt-only verification impossible, and receipts inherit the telemetry enable/disable switch. | |
| Reuse the core audit ledger chain | Shipped, tested verify path with chain head + Merkle root. Cost: contradicts RCPT-02's naming and merges two independent chains. | |

**User's choice:** Own chain domain.
**Notes:** Became D-11.

### Where the receipt type lives

| Option | Description | Selected |
|--------|-------------|----------|
| Core owns receipt + `LayerId` promoted to core | `LayerId` joins `LayerAttestationStatus` as policy-free vocabulary; policy stays in the CLI registry. Only an exhaustive enum makes "every layer is covered" a compile-time property. | ✓ |
| Core owns receipt, identity crosses as `&'static str` | Strictest policy-free reading; house precedent exists (`ArmExpectancy::token_arm`). Cost: loses exhaustiveness exactly where it matters most. | |
| nono-cli owns the whole receipt | No boundary argument needed, everything in one place. Cost: puts chain/verify primitives CLI-side against the core-audit precedent. | |

**User's choice:** Core owns receipt, `LayerId` promoted.
**Notes:** Became D-12. Verified during discussion that `nono-shell-broker` already depends on
`nono` (core) — which is what makes this decision unblock all three binaries uniformly.

### The fourth state

| Option | Description | Selected |
|--------|-------------|----------|
| All four verbatim — RCPT-03's three are a floor | RCPT-03 forbids only one thing: an unattested layer rendering as attested. Four states satisfy that strictly. Collapsing would make WFP read as a gap on essentially every ordinary session. | ✓ |
| Store four, render three | Full fidelity in the record, literal trichotomy in `receipt show`. Cost: the human reading the output is the reader RCPT-03 is about. | |
| Collapse to three at record level | Strictest literal compliance, errs toward under-claiming. Cost: discards a state 117 built on purpose and makes the common outcome look like a failure. | |

**User's choice:** All four verbatim.
**Notes:** Became D-13. D-37's "a permanently-on warning is functionally identical to no warning"
argument was reused directly.

### SC2 enforcement

| Option | Description | Selected |
|--------|-------------|----------|
| Type-allowlist scan + sentinel round-trip | The scan parses the struct fresh (discovery-based) and fails on non-allowlisted types; the behavioral test proves no sentinel path/arg/env bytes reach the serialized output — catching an allowlisted type populated from content, which a type scan structurally cannot see. | ✓ |
| Type-allowlist source scan only | Exactly SC2's literal wording, least surface. Cost: leaves the allowlisted-type-populated-from-a-path hole open. | |
| Denylist of content-capable types | Trivial to write. Cost: names its targets, which is precisely the V-01 blindness — a type alias or newtype walks past it. | |

**User's choice:** Scan + sentinel round-trip.
**Notes:** Became D-14, with the requirement that each carries its own perturbation proof.

---

## Cross-binary coverage (broker + daemon)

Two structural facts established before the question: the daemon has no parent `nono.exe`
(`nono-agentd` *is* the supervisor), so multi-writer is unavoidable; and a broker-arm session has
two genuinely independent `CREATE_SUSPENDED` windows, which is why 117 made `Broker` its own
`EntryPath` variant.

### Writers and chain sharing

| Option | Description | Selected |
|--------|-------------|----------|
| Per-writer chain segments, correlated by session id | Whoever attests, writes — one uniform rule. No cross-process lock on the hot hook path; each segment verifies independently. A broker-arm session honestly yields two receipts, mirroring the two real suspended windows. | ✓ |
| One shared chain, advisory lock | One continuous sequence, simplest consumer story. Cost: cross-process locking on a per-tool-call path, plus stale-lock failure modes in a security tool's launch path. | |
| Broker reports back, two rules | Single receipt per broker session. Cost: two rules for who writes, and the broker's results get re-asserted by a process that did not observe them. | |

**User's choice:** Per-writer chain segments.
**Notes:** Became D-15.

### Daemon `Unconfirmed`-on-`Proceed`

| Option | Description | Selected |
|--------|-------------|----------|
| Record faithfully, triage what it reveals | The receipt is a record, not a decision. Any `Unconfirmed`-on-`Proceed` becomes a finding triaged under D-13 severity rules. Pre-judging is the same error as assuming a gate works. | ✓ |
| Make it structurally impossible first | Assert every daemon-arm expected row can reach `Confirmed`/`Established`. Stronger, but commits to a fix before the receipt has shown one is needed. | |
| Daemon out of scope, named boundary | Keeps 118 tight. Cost: fails RCPT-01 on the fork's multi-tenant path, where governance receipts arguably matter most. | |

**User's choice:** Record faithfully.
**Notes:** Became D-16. D-37 point 2 preserved — no `ProceedDowngraded` reintroduction in the
daemon's decision shape.

### Per-tool-call hook path

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — one per confined process, with a measured budget | Every tool call is a confined process. Coalescing would mean a degraded tool call leaves no record — green-by-absence on the fork's production path. Cost measured per D-24; a bad number becomes a recorded trade-off, not a silent compromise. | ✓ |
| Coalesce identical consecutive receipts | Cuts volume hard while still catching changed pictures. Cost: "every confined session" becomes "every distinct configuration". | |
| One receipt per hook session | Lowest volume, closest to a human's idea of "session". Cost: a degraded individual tool call is invisible. | |

**User's choice:** One per confined process, with a measured budget.
**Notes:** Became D-17.

---

## Claude's Discretion

Deferred to Claude with constraints stated in CONTEXT.md:

- Sink location (`%PROGRAMDATA%` vs `%LOCALAPPDATA%`), directory structure, file naming
- Retention and rotation policy, provided rotation never silently truncates a chain segment
- On-disk serialization format and schema versioning
- Whether the receipt records token arm and entry path as first-class fields
- Whether the coarse Event Log pointer earns its place given its three conditional dependencies
- How the census stays in sync with the registry (a discovery-based meta-test is the obvious
  answer; the wiring is open — but adding a `LayerId` without a census row must fail the build)
- Plan and wave breakdown

## Deferred Ideas

- Third-party-verifiable (asymmetric) receipts via a signed chain head — handed to Phase 119's
  boundary statement as a named recorded decision
- Empirical proof that a real confined child cannot read the Application Event Log — narrowed by
  the live measurement, not settled
- Continuous / periodic re-attestation — D-20 locks startup-only; receipts inherit the limit
- Security-model boundary statement → Phase 119; tool-sandbox verdict execution → Phase 120
- Undiscussed gray areas available if planning needs them: sink retention/rotation, schema
  versioning and migration, the Event Log pointer's value, token arm / entry path as fields

## Todos Reviewed (none folded)

All three matched on bare keywords at ≤ 0.4 and none are topical:

- `20260816-ci-gates-outlived-their-scope.md` — matched on "phase"; dispositions already settled
  per the pause handoff ("do not re-survey"). One finding echoed in D-23, but the todo is tooling
  work.
- `20260611-msi-vcredist-prereq.md` — matched on "phase"; v3.5 Phase 106 work. Declined by 116/117.
- `20260611-poc-cert-broker-clean-host.md` — matched on "out"/"phase"; the POC *certificate*
  broker, not the Low-IL shell broker. Declined by 108/116/117.
