# Phase 118: Per-Session Enforcement Receipts - Context

**Gathered:** 2026-08-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 117 shipped a supervisor-side startup attestation that produces a **decision** and speaks
only when something is wrong. Phase 118 turns that same pass into a **record** that speaks on
every session — content-free, tamper-evident, and readable by someone who was not there.

Three deliverables:

1. **A per-session enforcement receipt** naming the state of every layer the Windows backend
   composes, emitted for every confined session on every entry path. Content-free: no paths, no
   arguments, no payload content. (**RCPT-01**)
2. **Tamper-evidence** on the same construction and discipline as the existing HMAC-chained
   `SecurityEventLayer`, so a governance consumer can detect an edited receipt. (**RCPT-02**)
3. **A four-state per-layer vocabulary** in which an unattested layer can never be read as an
   attested one, and a reader can tell the cases apart without out-of-band knowledge.
   (**RCPT-03**)

**Windows-only**, carried forward from 117's D-09 without re-litigation. Linux Landlock and macOS
Seatbelt have no layer registry to attest; their non-coverage is a named boundary already handed
to Phase 119 (117 D-12), and receipts inherit that same boundary rather than inventing a second
one.

**Not this phase:** any new confinement layer; any change to the fail-direction contract's
outcomes; third-party-verifiable (asymmetric) receipts; continuous or periodic re-attestation
(D-20 locks startup-only and receipts inherit that limit); the security-model boundary statement
(Phase 119, BOUND-*); the tool-sandbox verdict execution (Phase 120).

**Critical framing carried from the pause handoff.** Both `blocking` anti-patterns in
`.planning/.continue-here.md` are directly on point and were acknowledged before this discussion:
(1) a gate that reports success while structurally incapable of reporting failure — a receipt that
claims "layer active" when the check silently did not run is that same defect wearing different
clothes; and (2) a contract that spans two files where only one was scoped — a receipt emitter,
its verifier, and its content-free source scan are three artifacts that must move together.

</domain>

<decisions>
## Implementation Decisions

### Emission Completeness

- **D-01: The receipt is a full census of every registry row, on every confined session.** Not a
  filtered subset and not downgrade-only. This is forced rather than chosen: RCPT-01 says "every
  confined session", and RCPT-03 requires a reader to identify "not expected in this
  configuration", which is only possible if `NotApplicable` rows are *physically present* in the
  record. Today's `LayerAttestationDowngraded` event fires only on downgrade — a record that
  speaks only on failure is green-by-absence, the exact anti-pattern the pause handoff logged as
  `blocking`.

- **D-02: One receipt type, carrying a terminal session-outcome field (`ran` / `refused`).** An
  aborted launch — required layer failed, suspended child terminated, never executed an
  instruction — still produces a receipt. It is the highest-value record there is: it names the
  failed layer *and* the state of the other twelve. Decisive reason: without it, "nono refused
  this session" and "nono never ran / crashed" are the same observation to a governance consumer.

- **D-03: The receipt is written ONCE, at the D-21 attestation gate, before `ResumeThread`.** Not
  an open/close pair and not at session end. Three reasons, all load-bearing: the record is
  complete and durable before the child executes a single instruction; it matches D-20 exactly
  (the claim *is* startup-only, so a record describing startup is the honest shape, and an
  open/close pair would imply a mid-session claim nono does not make); and it survives a
  supervisor kill, which is the case where evidence matters most. The `refused` outcome falls out
  naturally — same write point, different terminal value.

- **D-04: Emitter failure degrades visibly by default; machine policy can make it fail-closed.**
  If the receipt cannot be emitted (poisoned chain mutex, `RegisterEventSourceW` returns NULL,
  unwritable sink), the session proceeds — but the failure is itself recorded (`TelemetryDegraded`
  + operator banner) and the session's claim downgrades. Never silently. An
  `HKLM\SOFTWARE\Policies\nono` field can require receipts fleet-wide, turning it fail-closed.
  Rationale: this mirrors D-26's machine-policy-wins pattern and the *existing* abort-vs-degrade
  split already shipped in `machine_policy.rs` (egress errors abort, malformed telemetry
  degrades), rather than inventing a new posture. Always-fail-closed was rejected because it turns
  an observability failure into an availability failure — a broken event-log registration on one
  host would stop confined work entirely.

- **D-05: Strict identity — opaque session id + pid only. No path-derived value, not even
  salted.** This is what makes RCPT-01's content-free claim *mechanically provable*: the SC2 scan
  can assert a **type allowlist**, so any new field capable of carrying content fails the build.
  The moment a hash-of-a-path is admissible, the scan degrades into a judgment call about which
  hashes are acceptable — and this phase exists to stop honesty from being a judgment call. This
  deliberately declines to reuse the shipped `path_hash` + `PathCategory` telemetry shape
  (D-08/D-09 of that module), which would make "no paths, no arguments, no payload content" a
  claim with an asterisk.

### Delivery & Sink

- **D-06: A dedicated operator-ACL'd receipt sink is primary; the Windows Event Log is demoted to
  a coarse pointer** ("receipt N emitted, chain head X"). Two problems solved at once: it
  sidesteps the D-28 reconnaissance question instead of betting on an unproven ACL reading, and it
  gives RCPT-02's "downstream governance consumer" something it can actually *retrieve* — which
  the Event Log never really offered. See D-07 for the measurement that drove this.

- **D-07: The Event Log readability assumption is NOT cleared — and the risk moved.** 117 left
  this as an open manual-verification item; it was run live during this discussion (evidence in
  `<specifics>`). `S-1-1-0` (Everyone) is absent, and `ALL APPLICATION PACKAGES` holds `0x2`
  (write-only) — so the AppContainer route 117 was most worried about is **clean**. But
  `INTERACTIVE` (`S-1-5-4`) carries `0x3`, which includes `0x1` read, and neither Low-IL nor
  AppContainer strips group SIDs; `S-1-5-33` (WRITE RESTRICTED — the SID
  `CreateRestrictedToken`'s `WRITE_RESTRICTED` flag adds) also carries read. No `S:` mandatory-label
  section is present, so there is no `NO_READ_UP` barrier against a Low-IL read-up. **Planning must
  not treat this as settled either way** — what remains unproven is whether the child's token
  retains `IU` after AppContainer/restricted-token construction, and whether the Event Log service
  honors these ACEs the way a raw object ACL check would. D-06 makes the phase not depend on the
  answer.

- **D-08: The sink is guarded by a DENY ACE on the per-session synthetic SID and package SID, PLUS
  a `NO_READ_UP` mandatory label.** Both, not either. The supervisor and the confined child run as
  **the same user**, so an ordinary DACL cannot separate them — this is the central constraint.
  Neither mechanism alone covers every arm: the mandatory label misses a Medium-IL broker child,
  and the DENY ACE misses an arm where no session SID was minted. Note specifically that
  `WRITE_RESTRICTED` tokens check restricting SIDs on **writes only** — reads go through the
  normal token — so the restricted-token arm is *not* self-protecting here even though the
  AppContainer arm structurally is (an AppContainer process is denied anything not explicitly
  granting its package SID). Matches CLAUDE.md's defense-in-depth principle rather than betting on
  one mechanism.

- **D-09: The governance consumer is same-trust-domain — the HMAC key holder** (operator / fleet
  admin). This is RCPT-02's literal wording ("same terms as the existing HMAC-chained
  `SecurityEventLayer`") and it keeps the phase bounded. Third-party-verifiable receipts are
  **deferred**, not silently absorbed — see `<deferred>`.

- **D-10: A new `nono receipt list | show | verify` command family**, reusing the shipped
  `AuditCommands` shape verbatim (`cli.rs:3498`). Rationale: it keeps the chain domains visibly
  separate, which is the existing house discipline, so a consumer is never confused about *which*
  integrity claim they just verified. Extending `nono audit` was rejected because it would imply
  receipts live in the core audit ledger, contradicting RCPT-02 and merging two chains the
  codebase deliberately keeps apart.

### Chain & Receipt Shape

- **D-11: Receipts get their OWN chain domain, using the identical construction and discipline.** A
  third domain constant alongside `TELEMETRY_CHAIN_DOMAIN` and the core audit module's
  `CHAIN_DOMAIN`, with the same `Hmac<Sha256>(domain || prev_head || event_domain || bytes)`
  construction and the same advance-under-mutex discipline (`advance_and_emit`, hardened in 117
  WR-21 to hold the mutex across the full build+advance+emit sequence).

  **Decisive technical reason, and planning must not "simplify" past it:** an HMAC chain commits
  each entry to `prev_head`. If receipts and telemetry events interleave on one chain but land in
  two different sinks (which D-06 requires), then verifying the receipt sink *alone* is
  structurally impossible — a consumer would need the telemetry events just to recompute the
  intermediate heads, and would see a chain full of holes. RCPT-02's "same terms as" therefore
  means **same construction and discipline, not the same chain instance**. This is the Phase 115
  lesson in a new place: a requirement's prescribed mechanism can be un-implementable as literally
  written, and the mechanism is what to verify.

- **D-12: The receipt type lives in `crates/nono` (core), and `LayerId` is promoted to core
  alongside `LayerAttestationStatus` as policy-free vocabulary.** The *policy* — per-arm
  expectancy, contracted outcome, enforcing call sites — stays in nono-cli's
  `layer_registry.rs`. Rationale: `LayerAttestationStatus` already lives in core and its module
  doc already calls itself "a policy-free status vocabulary"; the identity enum is its natural
  companion, and core already owns comparable primitives (the `audit` module's ledger
  append+verify and merkle; the structured `diagnostic` codes, with rendering left CLI-side).

  **Decisive reason:** the receipt must be a full census of all 13 rows (D-01), and only an
  exhaustive enum makes "every layer is covered" a *compile-time* property. A stringly-typed seam
  (the `&'static str` approach `ArmExpectancy::token_arm` uses to dodge a circular concern) would
  make completeness unprovable in the one artifact whose entire value is completeness.

  **Consequence that unblocks D-14:** `nono-shell-broker` already depends on `nono` (core) —
  verified in `crates/nono-shell-broker/Cargo.toml`. `nono-agentd` cannot reach
  `exec_strategy_windows/` (it `#[path]`-includes only `agent_daemon/mod.rs`, `telemetry/mod.rs`,
  and `agent_daemon/telemetry_init.rs`) but *can* reach core. So core ownership is what makes
  receipts writable from all three binaries uniformly.

- **D-13: All four `LayerAttestationStatus` states are carried verbatim — RCPT-03's three are a
  FLOOR, not a cap.** RCPT-03 requires a reader to distinguish three cases and forbids exactly one
  thing: an unattested layer rendering as attested. Four states satisfy that strictly.

  **Decisive reason for keeping `EstablishedNotIndependentlyObservable` distinct:** 117 accepted
  up front that WFP filter state is unreadable from a non-elevated supervisor, so this is the
  *normal* state for the WFP row on ordinary sessions. Collapsing it into `Unconfirmed` would make
  the most common real-world outcome indistinguishable from an actual failure — and D-37 already
  ruled, on exactly this reasoning, that a permanently-on warning is functionally identical to no
  warning.

- **D-14: SC2 is enforced by a type-allowlist source scan AND a sentinel round-trip test.** The
  scan parses the receipt struct fresh on every run (discovery-based, never naming its targets —
  the Phase 115 V-01 lesson) and fails on any field whose type is not allowlisted. The behavioral
  test builds a receipt from a session with sentinel paths / arguments / env values and asserts
  none of those bytes appear anywhere in the serialized output.

  Both are needed because they have different blind spots: a source scan checks the *type*, not
  the *value*, so an allowlisted `u64` or opaque `String` populated from the cwd would pass it and
  still leak. **Each must carry its own perturbation proof** — proof that the test can fail — per
  the discipline that paid off four times in the prior session.

### Cross-Binary Coverage

- **D-15: Whoever attests, writes — one uniform rule — with per-writer chain segments correlated
  by session id.** No cross-process lock, which matters because the per-tool-call hook path is
  hot; each segment verifies independently.

  **A broker-arm session honestly yields TWO receipts**: `nono.exe`'s for `broker.exe`
  (`EntryPath::DirectCli`) and `broker.exe`'s for the real confined grandchild
  (`EntryPath::Broker`), each naming its entry path, stitched by session id. This mirrors the two
  genuinely independent `CREATE_SUSPENDED` windows — 117 made `Broker` its own `EntryPath` variant
  for precisely this reason — rather than flattening them into one claim.

  Multi-writer was unavoidable regardless: **the daemon has no parent `nono.exe`**. `nono-agentd`
  *is* the supervisor for daemon sessions, so "report back to the parent" is not available there.

- **D-16: The daemon records faithfully; whatever that reveals gets triaged, not smoothed over.**
  `nono-agentd`'s decision is deliberately two-state (Plan 17 deleted `ProceedDowngraded` as dead
  code that read as a supported state; D-37 point 2 forbids reintroducing it), while its receipt
  carries four-state per-layer detail. A daemon receipt may therefore show `Unconfirmed` rows on a
  session that returned `Proceed`. That is either fine (those rows are genuinely `NotApplicable`
  on the daemon arm) or it is exactly the honesty gap this milestone exists to close — **and
  nobody currently knows which, because nothing has ever recorded it.**

  Any `Unconfirmed`-on-`Proceed` becomes a finding triaged under 117's D-13 severity rule: fix the
  expectancy matrix in-phase, or accept with written justification and a named successor. The
  receipt is a *record*, not a decision. D-37 point 2 is preserved — no `ProceedDowngraded`
  reintroduction in the daemon's decision shape.

- **D-17: The per-tool-call hook path gets a receipt per invocation, with a MEASURED latency
  budget recorded per D-24.** Every tool call is a confined process, so under RCPT-01 it gets a
  receipt. Coalescing was rejected because a single tool call that ran with a degraded layer
  picture would leave no record — green-by-absence on the fork's actual production shape
  (sandbox-the-tools). The project constraint is explicit (zero startup latency for the Windows
  backend) and D-24 requires a measured number, not an asserted "negligible". If the measurement
  comes back bad, that becomes a recorded trade-off with a named fallback — not a silent design
  compromise.

### Carried Forward (not re-litigated)

- **D-18: Windows-only** — 117's D-09, unchanged. Unix non-coverage is already a named boundary
  handed to Phase 119 (117 D-12); receipts inherit it rather than opening a second one.
- **D-19: The supervisor attests; the confined process never does** — 117's D-19, unchanged. No
  receipt field may ever be populated from a claim made by the process being confined. Stated as
  an invariant so no plan introduces a child-side reporting path.
- **D-20: Startup-only** — 117's D-20. The receipt describes the launch, not the session's course.
  Mid-session layer teardown is outside the claim, and the receipt must say so rather than let the
  limit go unwritten.
- **D-21: ADR-65 stands** — no minifilter. `MinifilterAbsence` is a census row that reads
  `NotApplicable`, never `Unconfirmed`; per-file read policy inside one directory stays explicitly
  unclaimed.
- **D-22: `/gsd:code-review` runs on this phase.** It touches security-relevant code, and the
  milestone invariant is explicit — executor self-check is not security evidence (Phase 112's
  review gate caught 4 Critical fail-open defects that all 8 executor self-checks passed over).
- **D-23: Cross-target clippy MUST** for any cfg-gated Unix edit — `cross` linux-gnu +
  `cargo-zigbuild` apple-darwin, both local, **no PARTIAL→CI**. Promoting `LayerId` into
  `crates/nono` and adding a `nono receipt` command family touches `cli.rs` and core, which carry
  Unix `cfg` branches — assume both gates apply. Note the known gap: CLAUDE.md's rule pins a
  *target set*, not a *toolchain version* (local 1.95.0, CI stable resolves to 1.97.0), so local
  green is not proof.
- **D-24: SDK STATE/ROADMAP writers stay banned.** Two milestones are open (v3.5 paused, v3.7
  active), so `REQUIREMENTS.md` and `ROADMAP.md` are appended to, never overwritten, and
  `phases.clear` must not run. All commits DCO-signed.

### Resolved During Planning (operator call, 2026-08-16)

Phase 118 research (`118-RESEARCH.md`, Findings 1 and 2) verified two of this document's working
assumptions against the live tree and found them **false**. Three operator decisions were taken to
resolve them before planning. These supersede RESEARCH.md's Open Questions 1–3.

- **D-25: The receipt chain is a KEYLESS hash chain on the core audit module's construction —
  `SHA256(domain || prev || leaf_hash)` — NOT the telemetry chain's HMAC.** Research Finding 2:
  `SecurityEventLayer`'s chain key is generated fresh per-process from `OsRng`
  (`telemetry/mod.rs:319`) and zeroized on `Drop` (`:96-103`); it is never persisted, exported, or
  recoverable. Mirroring it literally, as D-11 prescribes, would produce receipts that **nobody —
  including the operator — can verify once the emitting process exits**, contradicting D-09's
  "governance consumer = the HMAC key holder" premise and RCPT-02's actual use case.

  **D-11 is amended, not overturned:** its load-bearing argument — receipts get their OWN chain
  domain, because an interleaved chain landing in two sinks is structurally unverifiable from the
  receipt sink alone — **stands unchanged and still governs**. What changes is only the primitive:
  keyless SHA-256 in place of `Hmac<Sha256>`, matching `crates/nono/src/audit.rs`'s `hash_chain`
  (`:658-669`), which is the construction `nono audit verify`'s fail-closed
  recompute-and-compare already consumes — the same command family D-10 mirrors verbatim. RCPT-02's
  literal requirement ("verifiable... so an edited receipt is detectable") is satisfied: the
  integrity claim is "nobody edited this without leaving a hash mismatch." The stronger claim
  ("only the key holder could have produced this") is NOT made, and no plan may imply it. The
  advance-under-mutex discipline from D-11 (WR-21: mutex held across the full build+advance+emit
  sequence; WR-09: chain fields private behind a single accessor) is retained in full — it is
  independent of the primitive. A persistent-HMAC-key-via-keystore route was considered and
  rejected: it opens key provisioning, rotation, escrow, and a key-absent fail-direction question
  inside a phase scoped to receipts.

- **D-26: `nono-agentd`'s `daemon_attest_and_decide` is restructured from stop-at-first-failure to
  collect-all-then-decide.** Research Finding 1: it is today a 5-of-13-layer early-return chain
  (`agent_daemon/launch.rs:1417-1500`, four `Abort` returns at `:1435/:1446/:1455/:1470`) — if
  `AppContainerProfile` fails, `JobObjectContainment` and every layer after it is **never probed**,
  so there is no state to record. Every modelled layer is now probed before the decision is
  computed. Rejected alternatives: mapping unreached layers to `Unconfirmed` (destroys the
  "expected but not evaluated" vs "evaluated and failed" distinction — the exact honesty gap this
  milestone exists to close), and adding a 5th `LayerAttestationStatus` state (permitted by D-13,
  but ripples into every 117 consumer for a case a restructure removes entirely).

  **This is a fail-direction change to shipped security code and must be planned as one.** The
  decision outcome must be provably unchanged for every input: same aborting layer, same
  `DaemonAttestationDecision`. That equivalence needs its own perturbation-proofed test — proof the
  test can FAIL — not an executor self-check. D-37 point 2 is preserved: no `ProceedDowngraded`
  reintroduction in the daemon's decision shape. The daemon remains two-state.

- **D-27: The broker's 11 unmodelled census rows come from an EXTENDED WIRE CONTRACT, not from
  hardcoded knowledge.** Research Finding 1: `nono-shell-broker` knows only 2 of 13 layers
  (`BROKER_ATTESTABLE_LAYERS`, `main.rs:321`) and structurally cannot import `layer_registry.rs`
  (its `Cargo.toml` depends only on `nono` core, `thiserror`, `tracing`, `windows-sys`). `nono-cli`
  therefore communicates the `(EntryPath::Broker, …)` `NotApplicable` rows to the broker alongside
  the existing `NONO_BROKER_REQUIRED_LAYERS` string, keeping `layer_registry.rs` the single source
  of truth for expectancy. Hardcoding-plus-drift-guard was rejected: it would make a **third** copy
  of registry knowledge, with a guard as the only thing keeping it honest. The widened wire
  contract needs its own compatibility guard — a broker handed an unrecognised or absent row set
  must fail toward "unconfirmed", never toward a silently short census.

### Claude's Discretion

- **Sink location and layout** — `%PROGRAMDATA%\nono\receipts` (machine-wide) vs
  `%LOCALAPPDATA%` (per-user), directory structure, and file naming. The *guard* (D-08) and the
  *independently-verifiable-segment* property (D-11/D-15) are locked; the path is not.
- **Retention and rotation policy** for the sink, provided rotation can never silently truncate a
  chain segment in a way that makes a retained receipt unverifiable.
- **On-disk serialization format and schema versioning** — JSONL is the obvious fit given the
  audit ledger's `audit-events.ndjson` precedent, but the call is open provided the format is
  parseable by the D-10 verify command and compatible with the D-14 sentinel scan.
- **Whether the receipt records the token arm and entry path as fields.** Strongly implied by
  D-15's correlation story, but the exact shape is open.
- **Whether the coarse Event Log pointer (D-06) earns its place at all**, given it is conditional
  on three independent things holding (`SECURITY_LAYER` initialised, telemetry enabled,
  `RegisterEventSourceW` succeeding). Dropping it is acceptable if justified on the record.
- **How the census stays in sync with the registry as layers are added** — a discovery-based
  meta-test in the shape of `layer_registry_meta_test.rs` is the obvious answer, but the wiring is
  open. What is *not* discretionary: adding a `LayerId` variant without a corresponding receipt
  census row must fail the build, not a review.
- **Plan and wave breakdown**; whether the core type + chain land before or alongside the sink and
  the command family.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone definition
- `.planning/REQUIREMENTS.md` — v3.7 section: **RCPT-01 / RCPT-02 / RCPT-03** (lines 125–127, this
  phase), CINT-01/02/03 (Phase 117 — the producer of the layer vocabulary this consumes),
  BOUND-01/02/03 (Phase 119 — the consumer of D-09's deferred asymmetric-verification decision),
  and the **architecture invariants** block (ADR-65 stands; composite fails at its seams; the
  honesty-gap failure mode; structural fixes over spot fixes; executor self-check is not security
  evidence; two milestones open).
- `.planning/ROADMAP.md` — Phase 118 goal + SC1–SC4 (lines 369–377); Phase 119's stated dependency
  on this phase.

### The phase this one consumes
- `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-CONTEXT.md` — the
  full D-01…D-37 decision set. Most load-bearing here: **D-04** (the registry type is 118's
  receipt vocabulary — "one type, not two lists that can diverge"), **D-18** (the four-state
  vocabulary and the accepted WFP consequence), **D-19** (supervisor attests, child never),
  **D-20** (startup-only), **D-21** (the `CREATE_SUSPENDED` gate point), **D-26** (machine policy
  wins), **D-27/D-28** (the three downgrade channels and the reconnaissance rule), **D-37** (the
  contract-exempt vs real-gap classification, and the permanently-on-warning argument reused in
  D-13 above).
- `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-VALIDATION.md` — line
  141: the Event Log readability item this discussion executed. **Read this before acting on
  D-07** — the remediation it prescribes ("move to coarse wording, track as SC4-6 discrepancy")
  was written for the package-SID case, which measured clean; the `INTERACTIVE` finding is new.
- `proj/SPEC-windows-fail-direction-contract.md` — the generated/drift-checked contract. Its
  standing "Contract-exempt skips are NOT downgrades" rule (~lines 232–238) governs how census
  rows may be classified.

### Fork invariants
- `CLAUDE.md` — Library-vs-CLI boundary table (grounds D-12); cross-target clippy MUST/NEVER
  (D-23); unwrap policy; fail-secure and path-security rules; defense-in-depth principle (D-08).
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free library boundary and the Windows
  denial-rendering CLI-side carve-out. **D-12 promotes a type INTO core, so this ADR's boundary
  argument must be satisfied explicitly, not assumed.**
- `proj/ADR-116-tool-sandbox-disposition.md` — the fork-native (Pole B) verdict and Consequence 6.
- `.planning/architecture/adr-65-minifilter-go-no-go.md` — the standing verdict behind the
  `MinifilterAbsence` census row (D-21).
- `.planning/templates/cross-target-verify-checklist.md` — the two cross-target clippy gates
  (D-23). Single source of truth for the runbook.

### The types being consumed and extended
- `crates/nono/src/attestation.rs` — **`LayerAttestationStatus` (line 74)**, the four states D-13
  carries verbatim; its module doc already names Phase 118 as the consumer. Also the probe
  functions (`probe_integrity_level`, `probe_in_job`, `probe_app_container_sid`,
  `probe_restricted_sids`) and the D-19 invariant stated in the module header.
- `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` — **`LayerId` (line 209, 13
  variants)** which D-12 promotes to core; `EntryPath` (DirectCli / Broker / Daemon), `ArmExpectancy`
  (and its `&'static str` decoupling precedent), `ProbeKind` (line 368), **`LayerApplication` (line
  407)** and `AppliedLayers` — the fail-secure `NotApplied` default and the CR-14 "these are EFFECT
  facts, not construction facts" rule both apply to what a receipt may record.
- `crates/nono-cli/src/exec_strategy_windows/attestation.rs` — `attest_and_decide` (line 659),
  `AttestationDecision` (line 125), `AttestationInput` (line 153), and
  `BROKER_REQUIRED_LAYERS_ENV_VAR` — the receipt's data source, and the broker wire contract D-15
  builds beside.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `apply_startup_attestation_gate`, the
  `CREATE_SUSPENDED` window that is D-03's write point.

### Surfaces the receipt plugs into or parallels
- `crates/nono-cli/src/telemetry/mod.rs` — `TELEMETRY_CHAIN_DOMAIN` (line 75), `advance_chain`
  (line 126), **`advance_and_emit` (line 509)** — the construction and mutex discipline D-11
  mirrors into a new domain. Note the WR-21 hardening (mutex held across build+advance+emit) and
  WR-09 (chain fields private, single accessor) — both are properties the receipt chain must also
  have.
- `crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs` — `emit_attestation_event`
  and `DowngradeDetailChannel`. **Read the module header before planning**: it documents why
  `nono-cli` has no `[lib]` target, how `#[path]`-inclusion compiles shared files twice into two
  binaries with different call graphs, and why that makes `dead_code` lints binary-specific. This
  directly constrains where D-15's writer code can live.
- `crates/nono-cli/src/telemetry/event.rs` — `SecurityEvent` (line 266) and `SecurityEventType`
  (line 70, incl. `LayerAttestationDowngraded`). The `downgraded_layers` field's doc comment
  carries the D-28 reasoning and the unverified assumption D-07 measured.
- `crates/nono/src/audit.rs` and the `audit` module — ledger append+verify, attestation
  sign/verify, merkle/inclusion-proof. The machinery D-09 deliberately does NOT use this phase,
  and the deferred item's implementation if 119 takes it up.
- `crates/nono-cli/src/cli.rs:3498` — `AuditCommands` (List / Show / Verify / Cleanup), the shape
  D-10 mirrors. `AuditVerifyArgs`' doc (lines 3503–3510) describes the fail-closed
  recompute-and-compare pattern the receipt verifier should follow.
- `crates/nono/src/machine_policy.rs` — the `HKLM\SOFTWARE\Policies\nono` spine for D-04,
  including its established abort-vs-degrade discipline and its fail-closed
  present-but-unreadable behaviour (line 19).
- `crates/nono-cli/src/claude_code_hook.rs` and `crates/nono-cli/data/hooks/nono-tool-hook.ps1` —
  the per-tool-call path D-17 measures.
- `crates/nono-shell-broker/src/main.rs` and `crates/nono-shell-broker/Cargo.toml` — the second
  `CREATE_SUSPENDED` window and the confirmed `nono` (core) dependency that makes D-12/D-15 work.

### Test patterns to follow
- `crates/nono-cli/tests/layer_registry_selfcheck.rs` — the house source-scan pattern
  (`env!("CARGO_MANIFEST_DIR")` + `fs::read_to_string`, no `regex`, no `include_str!`), and
  explicitly discovery-based: "a test that names its targets is blind by construction". D-14's
  scan follows this.
- `crates/nono-cli/tests/layer_registry_meta_test.rs` — the discovery-based meta-test shape for
  keeping the census in sync with the registry.
- `crates/nono-cli/tests/layer_force_unavailable.rs` — the per-layer forced-unavailable seam,
  useful for producing receipts with non-`Confirmed` rows in tests.

### Platform behaviour
- `Skill("spike-findings-nono")` — engine-agnostic confinement patterns, landmines, and verified
  Windows OS-behaviour facts (SEED-004). Load before writing the sink ACL / mandatory-label code.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`LayerAttestationStatus` is already public, in core, and platform-neutral** — its own doc names
  Phase 118 as the consumer. D-13 consumes it unchanged; this is not new construction.
- **`LayerId` + the 13-row registry already exist and are already drift-guarded** — the receipt's
  census is an enumeration over a list the build already keeps honest.
- **`advance_and_emit` is a hardened, reviewed chain-advancement accessor** — mutex held across
  build+advance+emit (WR-21), chain fields private with a single accessor (WR-09). D-11 mirrors a
  design that has already survived a review round, rather than inventing one.
- **`AuditCommands` (List/Show/Verify/Cleanup) with a fail-closed recompute-and-compare verifier** —
  D-10's command family is a pattern copy, not a design problem.
- **`AppliedLayers` already encodes EFFECT facts, not construction facts** (CR-14) — the receipt
  can record what a guard *achieved* without re-deriving it, and the fail-secure `NotApplied`
  default already means a forgotten layer degrades rather than silently passing.
- **`nono-shell-broker` already depends on `nono` (core)** — verified in its `Cargo.toml`. The
  broker can write its own receipts natively once D-12 puts the type in core.
- **`layer_force_unavailable.rs`'s per-layer seam** — a shipped way to produce non-`Confirmed`
  rows for tests without mocking the OS.

### Established Patterns
- **Two chain domains already coexist and are deliberately independent** (`TELEMETRY_CHAIN_DOMAIN`
  vs the core audit `CHAIN_DOMAIN`). D-11 adds a third to an existing pattern rather than
  inventing separation.
- **`nono-cli` has NO `[lib]` target** — shared files are `#[path]`-included wholesale into both
  `nono` and `nono-agentd` as two independent compilations. A symbol used by only one binary is a
  `-D warnings`-fatal `dead_code` lint on the other, and neither `#[expect]` nor a Cargo feature
  fixes it (both empirically tested in 117 Plan 17). **This is why D-12's core placement is not
  merely tidy — it is the mechanism that lets all three binaries share the receipt code.**
- **Self-enforcing discovery-based source scans are house style** and explicitly justified by the
  Phase 115 V-01 lesson. D-14 has precedent to follow, including the no-`regex`/no-`include_str!`
  convention.
- **`machine_policy.rs` already distinguishes abort from degrade** — D-04 extends an existing
  posture rather than inventing one.

### Integration Points
- **D-03 → `launch.rs`'s `apply_startup_attestation_gate`** — the receipt write slots into the
  existing D-21 gate, a control point the code already has. No launch-path restructuring.
- **D-12 → `crates/nono` + ADR-86** — promoting `LayerId` into core needs an explicit boundary
  argument in the plan, not an assumption. The split (identity/status vocabulary in core, policy
  in the CLI registry) is the argument; write it down.
- **D-15 → three binaries** — `nono.exe` (`exec_strategy_windows/`), `nono-shell-broker.exe`
  (own `CREATE_SUSPENDED` window), `nono-agentd.exe` (`agent_daemon/launch.rs`, reachable to core
  but not to `exec_strategy_windows/`).
- **D-16 → the daemon's expectancy matrix** — whatever the daemon's receipts reveal feeds back
  into `layer_registry.rs`'s `ArmExpectancy` rows for `EntryPath::Daemon`, or into a recorded
  discrepancy.
- **D-23 → cross-target gates** — touching `crates/nono` and `cli.rs` puts Unix `cfg` branches in
  the blast radius. Assume both local clippy gates are required.

</code_context>

<specifics>
## Specific Ideas

- **The live measurement taken during this discussion, recorded verbatim so planning does not
  re-derive it.** `wevtutil gl Application`, run 2026-08-16 on this host:

  ```
  channelAccess: O:BAG:SYD:(A;;0x2;;;S-1-15-2-1)
    (A;;0x2;;;S-1-15-3-1024-3153509613-960666767-3724611135-2725662640-12138253-543910227-1950414635-4190290187)
    (A;;0xf0007;;;SY)(A;;0x7;;;BA)(A;;0x7;;;SO)(A;;0x3;;;IU)(A;;0x3;;;SU)
    (A;;0x3;;;S-1-5-3)(A;;0x3;;;S-1-5-33)(A;;0x1;;;S-1-5-32-573)
  ```

  Event-log access bits: `0x1` = read, `0x2` = write, `0x4` = clear. `S-1-1-0` absent.
  `ALL APPLICATION PACKAGES` = write-only. `INTERACTIVE`, `SERVICE`, `BATCH`, and
  `WRITE RESTRICTED` all = `0x3` (read + write). No `S:` mandatory-label section.
  **What this does and does not prove is stated in D-07** — do not upgrade it to a settled verdict
  in either direction.

- **The word "receipt" is doing real work.** A receipt is a *record*, not a *decision*. Wherever a
  plan is tempted to make the receipt smooth over an awkward state (D-16's `Unconfirmed`-on-
  `Proceed`, D-13's fourth state), the receipt's job is to *show* it. The decision layer is
  separate and already shipped.

- **The failure mode has a worked precedent in this repo** — `Sandbox::apply()` returning
  `UnsupportedPlatform` while the CLI enforced via WFP. Receipts should cite it as the reason
  configured ≠ observed, the same way 117's contract does.

- **Expect fixing this gate to reveal what it was hiding.** The prior session hit this three
  separate times (a green CI job masking 22 test failures; an unrunnable integration script hiding
  5 suite failures; an MSI build requiring a gitignored file). D-16 is written on the assumption
  that receipts will surface something — treat that as the normal outcome, not a surprise, and
  budget for the triage.

- **Two `blocking` anti-patterns from `.planning/.continue-here.md` apply directly**, and are
  restated here so planning inherits them: (1) never a multi-command `run:` block on a Windows CI
  job — one command per step, and prove a new gate can FAIL before trusting that it passes; (2)
  before scoping any "make X conditional / assert X" change, grep for **every** asserter of X —
  producer and validator are one unit of work. For this phase that unit is: receipt emitter →
  verifier → content-free scan.

</specifics>

<deferred>
## Deferred Ideas

- **Third-party-verifiable receipts (asymmetric).** A signature over the chain head using the core
  audit module's existing sigstore sign/verify + merkle inclusion proofs, letting an auditor with
  no shared secret verify. The machinery already exists in-tree. Deferred per D-09 as real scope
  growth beyond RCPT-02's wording — **handed to Phase 119's boundary statement as a named recorded
  decision**, not dropped. Phase 119 should state plainly that receipts are verifiable only within
  the operator's trust domain today.
- **Empirical proof of Event Log readability from a real confined child.** D-07 narrowed it (the
  AppContainer route is write-only) but did not settle it (`INTERACTIVE` grants read). D-06 makes
  this phase independent of the answer; the question itself remains open and should stay a tracked
  item rather than lapsing now that receipts route elsewhere.
- **Continuous / periodic re-attestation.** D-20 locks startup-only and receipts inherit that
  limit. Mid-session layer teardown remains outside the claim. Its own work item if ever taken up.
- **Security-model boundary statement + state-of-the-art decision log** → Phase 119
  (BOUND-01/02/03).
- **Tool-sandbox verdict execution** → Phase 120, sized by ADR-116's fork-native (Pole B) verdict.
- **Gray areas raised but not discussed** (available if planning needs them): sink retention and
  rotation policy; on-disk schema versioning and migration; whether the coarse Event Log pointer
  earns its place given its three conditional dependencies; and whether the receipt should record
  the token arm and entry path as first-class fields.

### Reviewed Todos (not folded)
- `20260816-ci-gates-outlived-their-scope.md` — matched at 0.4 on the bare keyword "phase". Covers
  libdbus / Linux credential storage, the Phase 102 rename tail, and the floating toolchain pin.
  Dispositions are already settled per the pause handoff ("do not re-survey"). One finding from it
  *is* echoed in D-23 (CLAUDE.md pins a target set, not a toolchain version) but the todo itself is
  tooling work, not receipt work.
- `20260611-msi-vcredist-prereq.md` — matched at 0.2 on "phase". MSI prerequisite work owned by
  v3.5 Phase 106. Previously reviewed and declined by Phases 116 and 117 for the same reason.
- `20260611-poc-cert-broker-clean-host.md` — matched at 0.4 on "out"/"phase". The "broker" here is
  the POC **certificate** broker for clean-host install, not the Low-IL shell broker. Host-gated
  v3.5 distribution item owned by v3.5 Phase 106. Previously declined by Phases 108, 116 and 117.

</deferred>

---

*Phase: 118-per-session-enforcement-receipts*
*Context gathered: 2026-08-16*
