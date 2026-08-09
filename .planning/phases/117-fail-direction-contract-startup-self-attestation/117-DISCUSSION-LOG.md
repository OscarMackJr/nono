# Phase 117: Fail-Direction Contract + Startup Self-Attestation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-09
**Phase:** 117-fail-direction-contract-startup-self-attestation
**Areas discussed:** Contract source of truth, Outcome vocabulary, Platform scope, Discrepancy policy (SC4), Attestation depth, Where the gate runs, Downgrade surface, Forced-unavailable seam

All eight offered gray areas were selected. Default (interactive) mode; no flags.

---

## Contract source of truth

### Authoritative source of the fail-direction contract

| Option | Description | Selected |
|--------|-------------|----------|
| Code registry is truth | Rust layer registry is the single source of truth; markdown contract generated from it or drift-checked against it by a test | ✓ |
| Doc is truth, test greps | Hand-authored markdown; self-enforcing test asserts cited call sites exist and every row has a test; fail-direction values stay unverifiable prose | |
| Doc only, review-gated | Reviewed document, drift caught by human review at next sync | |

**User's choice:** Code registry is truth
**Notes:** Aligns with the milestone's "structural fixes over spot fixes" invariant — DRAIN-02/03 both arose from hand-maintained lists drifting.

### Crate ownership under ADR-86

| Option | Description | Selected |
|--------|-------------|----------|
| nono-cli owns it | Windows layers all enforced CLI-side; ADR-86 already carves out Windows denial rendering | ✓ |
| Library type, CLI populates | Enum + status type in crates/nono next to diagnostic/ and the audit spine | |
| You decide | Planner settles against ADR-86 and 118's needs | |

**User's choice:** nono-cli owns it
**Notes:** Corroborated mid-discussion — the HMAC-chained `SecurityEventLayer` that 118's receipts hang off is already CLI-side (`telemetry/mod.rs:218`), distinct from the library's `audit_integrity.rs` chain.

### Contract document home

| Option | Description | Selected |
|--------|-------------|----------|
| proj/ as a SPEC doc | Durable non-ADR spec; signals "living description of shipped behaviour"; citable by 118 and 119 | ✓ |
| proj/ADR-117-... | Standalone phase-numbered ADR convention (ADR-98/108/111/113/114/116) | |
| Phase dir artifact | 117-FAIL-DIRECTION-CONTRACT.md with the phase's other artifacts | |

**User's choice:** proj/ as a SPEC doc

### Shared layer type for Phase 118

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, design for 118 now | One enumeration consumed by both the contract and the receipts | ✓ |
| No, keep 117 self-contained | 118 adopts or adapts later; avoids speculative generality | |
| You decide | | |

**User's choice:** Yes, design for 118 now

---

## Outcome vocabulary

### Per-layer outcome vocabulary

| Option | Description | Selected |
|--------|-------------|----------|
| Four values incl. substitute | abort / degrade-with-visible-claim / substitute-equivalent-mechanism / fail-open-defect | ✓ |
| Three values, CINT-01 wording | fail-closed / fail-open / continue-in-reduced-mode | |
| Two values, SC2 wording | abort / visibly-downgraded-claim | |

**User's choice:** Four values incl. substitute
**Notes:** Settles the question ADR-116 Consequence 6 explicitly deferred to this phase — the `WindowsTokenArm` cascade substitutes an equivalent-strength mechanism, which is neither abort nor downgrade.

### Is "fail open" admissible?

| Option | Description | Selected |
|--------|-------------|----------|
| Admissible but must be justified | Written reason + explicit acceptance per instance | ✓ |
| Defect by definition | Every fail-open row is a bug fixed in-phase | |
| Admissible, no justification bar | Document describes, does not judge | |

**User's choice:** Admissible but must be justified

### Keeping "substitute" from becoming a loophole

| Option | Description | Selected |
|--------|-------------|----------|
| Named substitute + attested | Row names the alternate mechanism; attestation confirms it actually took effect; unconfirmed falls through to abort/downgrade | ✓ |
| Named substitute, doc-level equivalence | Equivalence argued in the document; attestation confirms only that some accepted arm was reached | |
| You decide | | |

**User's choice:** Named substitute + attested

### Expectancy per configuration

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, expectancy is per-config | Registry becomes a matrix (layer × arm → expectancy + outcome); pre-builds RCPT-03's three-state | ✓ |
| No, one expectancy per layer | Simpler table and attestation | |

**User's choice:** Yes, expectancy is per-config
**Notes:** Load-bearing for this fork — WFP is daemon-path only, AppContainer absent on the broker arm, session-SID DACLs only operative on the WriteRestricted arm.

---

## Platform scope

### Platforms covered

| Option | Description | Selected |
|--------|-------------|----------|
| Windows-only | CINT-01/02/03 all say "the Windows backend" | ✓ |
| Windows now, Unix rows stubbed | Placeholder rows recording why Unix is out | |
| All three fully | Landlock + Seatbelt with equal rigor and tests | |

**User's choice:** Windows-only

### Layer set closure

| Option | Description | Selected |
|--------|-------------|----------|
| Derived, six are the floor | Follow the code; six named plus whatever else is load-bearing; every addition and exclusion reasoned on the record | ✓ |
| Exactly the six named | Bounded to the requirement text | |
| You decide | | |

**User's choice:** Derived, six are the floor

### cfg shape

| Option | Description | Selected |
|--------|-------------|----------|
| Platform-neutral enum, Windows values | Types compile everywhere; only population and probes are cfg(windows) | ✓ |
| Fully cfg(windows) | Registry and attestation entirely Windows-gated | |
| You decide | | |

**User's choice:** Platform-neutral enum, Windows values

### Recording Unix non-coverage

| Option | Description | Selected |
|--------|-------------|----------|
| Named boundary + 119 handoff | Stated in writing with reason; Phase 119 picks it up as a recorded decision | ✓ |
| Named boundary + FUT item | As above, plus an operator-gated proposed FUT entry | |
| Silence — scope is scope | | |

**User's choice:** Named boundary + 119 handoff

---

## Discrepancy policy (SC4)

### Default resolution direction

| Option | Description | Selected |
|--------|-------------|----------|
| Severity triage | Fix in-phase when the gap lets nono present an unconfirmed claim or silently widens the boundary; otherwise record + justify + name a successor | ✓ |
| Fix everything found | No deferred debt; phase size set by the enumeration | |
| Document everything, fix nothing | Reads as declining SC4 | |

**User's choice:** Severity triage

### Overflow rule when a fix is too large

| Option | Description | Selected |
|--------|-------------|----------|
| Contract shows actual, claim downgrades | Row states the real behaviour AND attestation treats the layer as unconfirmed so the runtime claim downgrades; plus a named successor | ✓ |
| Contract shows actual + successor | Honest on paper; runtime claim unchanged | |
| You decide | | |

**User's choice:** Contract shows actual, claim downgrades

### Where discrepancies are recorded

| Option | Description | Selected |
|--------|-------------|----------|
| Section in the contract SPEC | Standing "Contract vs. code discrepancies" section with resolution per row | ✓ |
| Separate 117-DISCREPANCIES.md | Phase artifact in the 116-ledger shape | |
| Both — detail in phase, summary in SPEC | | |

**User's choice:** Section in the contract SPEC

### Evidence standard

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — command, hits, date | 116 D-16 shape; greps must discover targets, not confirm pre-named paths | ✓ |
| Call site citation is enough | file:line only | |

**User's choice:** Yes — command, hits, date

---

## Attestation depth

### Probe depth

| Option | Description | Selected |
|--------|-------------|----------|
| Observed where possible | Live kernel state where a probe exists; configured-only fallback recorded per row | ✓ |
| Observed, unobservable = unconfirmed | Only live-observed counts; likely downgrades every real session | |
| Configured only | Attest that the establishing call returned success | |

**User's choice:** Observed where possible

### State for unobservable layers

| Option | Description | Selected |
|--------|-------------|----------|
| Distinct state, downgrades claim | Fourth state "established, not independently observable"; still downgrades | ✓ |
| Distinct state, does not downgrade | Named for honesty, counted as confirmed | |
| Collapses to unconfirmed | Most conservative; makes downgraded the normal case | |

**User's choice:** Distinct state, downgrades claim
**Notes:** Claude flagged the consequence explicitly — WFP is unreadable from a non-elevated supervisor, so downgraded becomes the normal daemon-path outcome. Accepted knowingly; the per-row default (Downgrade surface area) is what keeps it meaningful.

### Who attests

| Option | Description | Selected |
|--------|-------------|----------|
| Supervisor only, never the child | Confined process is never the source of a claim about its own containment | ✓ |
| Supervisor, child may supplement | Richer data; introduces untrusted input into a security claim | |

**User's choice:** Supervisor only, never the child

### Timing

| Option | Description | Selected |
|--------|-------------|----------|
| Startup only | As CINT-02 literally specifies | ✓ |
| Startup + exit re-check | Detects mid-session layer removal; fits 118's per-session receipt | |
| Startup + periodic | Continuous; adds a polling loop and latency | |

**User's choice:** Startup only
**Notes:** Consequence recorded — the contract must state plainly that mid-session layer removal is outside the claim.

---

## Where the gate runs

### Placement in the launch sequence

| Option | Description | Selected |
|--------|-------------|----------|
| Suspended child, then resume | Attest the real spawned process while CREATE_SUSPENDED, then ResumeThread or TerminateProcess | ✓ |
| Pre-spawn only | Attest what was built; no child ever exists on the failure path | |
| Pre-spawn + suspended | Both; two code paths to keep consistent | |

**User's choice:** Suspended child, then resume

### Abort mechanics

| Option | Description | Selected |
|--------|-------------|----------|
| Terminate + unwind guards + typed error | TerminateProcess, existing Drop-order unwind, typed NonoError with a diagnostic code naming the layer | ✓ |
| Terminate + generic failure | Generic sandbox-init error | |
| You decide | | |

**User's choice:** Terminate + unwind guards + typed error

### Gated execution paths

| Option | Description | Selected |
|--------|-------------|----------|
| All confined-child paths | Direct run, daemon launch, broker arm, per-tool-call hook path | ✓ |
| Supervised paths only | Leaves the broker arm unattested | |
| You decide | | |

**User's choice:** All confined-child paths

### Latency posture

| Option | Description | Selected |
|--------|-------------|----------|
| Measure and record a budget | Measure the pass, record cost per path in the contract; expensive probes become recorded trade-offs | ✓ |
| Cheap probes only | Coverage ceiling baked in to protect the hook path | |
| Correctness first | Latency revisited only if it measurably hurts | |

**User's choice:** Measure and record a budget

---

## Downgrade surface

### Default posture

| Option | Description | Selected |
|--------|-------------|----------|
| Per-row contracted default | Each row's contracted outcome is its default; reviewable row by row | ✓ |
| Downgrade by default, strict opt-in | Never abort unless a knob is set | |
| Abort by default, downgrade opt-in | Would abort ordinary daemon sessions on a probe limitation | |

**User's choice:** Per-row contracted default

### Strictness knob

| Option | Description | Selected |
|--------|-------------|----------|
| Machine policy + CLI flag | HKLM policy spine can require layers fleet-wide; CLI flag can tighten but never loosen | ✓ |
| CLI flag only | No fleet-wide enforcement | |
| No knob | Per-row contract is the whole policy | |

**User's choice:** Machine policy + CLI flag

### Surfaces carrying the claim

| Option | Description | Selected |
|--------|-------------|----------|
| Banner + diagnostic code + event | Human banner, typed NonoDiagnosticCode, HMAC-chained security event (the channel 118 reuses) | ✓ |
| Banner + structured event | Skips diagnostic-code plumbing | |
| Structured event only | Hard to square with SC2's "visibly downgraded" | |

**User's choice:** Banner + diagnostic code + event

### Recon-hardening

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — specifics off child-readable channels | Layer names go to the operator channel and the audit event; child-readable output stays coarse | ✓ |
| No — full detail everywhere | Simpler, one message path | |
| You decide | | |

**User's choice:** Yes — specifics off child-readable channels

---

## Forced-unavailable seam

### Seam mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| Generalize the existing toggle | Extend the shipped set_windows_wfp_test_force_ready pattern into a per-layer seam driven off the registry | ✓ |
| cfg(test) injection | Zero production surface; tests around the real spawn path | |
| Real host deprivation | Highest fidelity; needs a privileged host | |

**User's choice:** Generalize the existing toggle

### Production safety

| Option | Description | Selected |
|--------|-------------|----------|
| Compiled out of release | Not merely refused at runtime — a runtime env-var switch that disables a confinement layer is live attack surface | ✓ |
| Runtime-gated, as today | Consistent with the shipped NONO_TEST_HARNESS precedent | |
| Compiled out + audit event if reached | Belt and braces | |

**User's choice:** Compiled out of release
**Notes:** Claude flagged the resulting inconsistency — the existing WFP toggle is runtime-gated and now violates this rule; it must either migrate or be recorded as an SC4 discrepancy row. Captured as D-30.

### Untestable rows

| Option | Description | Selected |
|--------|-------------|----------|
| Loud gap, never a silent skip | Explicit unsatisfied entry in the contract plus a named manual-verification item | ✓ |
| Host-gated test, ignored by default | Reads as green-by-absence | |
| You decide | | |

**User's choice:** Loud gap, never a silent skip
**Notes:** Grounded in Phase 115's V-01 — a check that read green while being blind by construction.

### Coverage enforcement

| Option | Description | Selected |
|--------|-------------|----------|
| Meta-test discovers rows | Enumerates the registry; fails when a layer is added without a forced-unavailable test | ✓ |
| Hand-kept checklist in the SPEC | A hand-maintained list that can drift | |

**User's choice:** Meta-test discovers rows

---

## Claude's Discretion

- Exact SPEC filename under `proj/` and the document's internal section ordering (the `proj/` home
  and non-ADR SPEC form are locked).
- Whether the SPEC is generated from the registry or drift-checked against a hand-written file.
- Registry table layout, column ordering, and enum/struct shape.
- Plan and wave breakdown; whether registry + contract land before or alongside the attestation.
- Which specific candidate layers from the D-10 derivation land in or out — the call is
  discretionary, the reasoning on the record is not.

## Deferred Ideas

- Linux Landlock ABI-downgrade and macOS Seatbelt fail-direction enumeration → named boundary,
  handed to Phase 119.
- Continuous / periodic re-attestation → out of scope; startup-only is locked and the limit is
  stated in the contract.
- Per-session enforcement receipts → Phase 118 (RCPT-01/02/03).
- Security-model boundary statement + state-of-the-art decision log → Phase 119 (BOUND-01/02/03).
- Engine-agnosticism / decoupling from Claude Code's PreToolUse contract → carried from Phase 116;
  belongs with the engine-abstraction / `nono-agentd` line.
- Gray areas offered at the close but not taken: how the minifilter *absence* row is expressed and
  tested; whether call-site citations are drift-checked by the same meta-test; how SPEC
  generation/drift-check is wired into `make ci`.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` — generic keyword match; v3.5 Phase 106 MSI work. Also declined
  by Phase 116.
- `20260611-poc-cert-broker-clean-host.md` — "broker" is the POC certificate broker, not the Low-IL
  shell broker; v3.5 Phase 106. Declined by Phases 108 and 116 for the same reason.
