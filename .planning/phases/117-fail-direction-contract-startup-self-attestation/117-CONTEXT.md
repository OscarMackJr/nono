# Phase 117: Fail-Direction Contract + Startup Self-Attestation - Context

**Gathered:** 2026-08-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Three coupled deliverables that make the Windows composite's fail-direction one system-level
answer instead of a per-layer accident, and stop nono from reporting "enforcing" while a layer is
silently inert:

1. **A code-resident layer registry** (`crates/nono-cli`) enumerating every layer the Windows
   backend composes, with each layer's expectancy per execution arm and its contracted behaviour
   when it cannot be established — plus **`proj/SPEC-windows-fail-direction-contract.md`**,
   generated from or drift-checked against that registry. (**CINT-01**)
2. **A supervisor-side startup self-attestation pass** that probes each registry row against the
   real spawned child and either aborts or visibly downgrades the session's claim — never
   presenting a confinement guarantee it did not confirm. (**CINT-02**)
3. **A per-layer forced-unavailable test** for every registry row, with a meta-test that
   discovers rows from the registry and fails on any row lacking one. (**CINT-03**)

Plus **SC4**: where the contract and the code disagree, one of them changes *in this phase*, and
the discrepancy is recorded with re-runnable evidence rather than quietly reconciled.

**Windows-only.** Linux Landlock and macOS Seatbelt fail-direction are a named, reasoned boundary
handed to Phase 119 — not an omission and not in scope here.

**Not this phase:** per-session receipts (Phase 118, RCPT-*), the security-model boundary statement
and state-of-the-art decision log (Phase 119, BOUND-*), any new confinement layer, any minifilter
work (ADR-65 stands).

</domain>

<decisions>
## Implementation Decisions

### Contract Source of Truth

- **D-01: A code-resident layer registry is the single source of truth; the markdown contract is
  derived from it.** A Rust registry (layer-identity enum + per-layer metadata: name, expectancy
  per arm, contracted outcome, enforcing call site) is authoritative; the `proj/` SPEC document is
  generated from it, or drift-checked against it by a test that fails when they disagree. Rationale:
  the milestone's own architecture invariant is *"structural fixes over spot fixes"* — DRAIN-02 and
  DRAIN-03 both existed **because** a hand-maintained list drifted from its source of truth, and a
  hand-authored contract table would reproduce that class in a new place.

- **D-02: `crates/nono-cli` owns the registry.** Every Windows layer is enforced CLI-side
  (`exec_strategy_windows/`), ADR-86 already carves out Windows denial rendering as a deliberate
  CLI-side boundary, and — verified during discussion — the HMAC-chained `SecurityEventLayer` that
  Phase 118's receipts will hang off is *also* CLI-side (`crates/nono-cli/src/telemetry/mod.rs:218`,
  `TELEMETRY_CHAIN_DOMAIN`), distinct from the library's `audit_integrity.rs` chain. `crates/nono`
  stays policy-free; no ADR-86 argument is required.

- **D-03: The contract document is `proj/SPEC-windows-fail-direction-contract.md`** (exact filename
  is Claude's discretion; the `proj/` home and non-ADR SPEC form are not). Rationale: CINT-01 asks
  for *"a description of behaviour rather than an assertion about it"* — that is a living spec, not
  a decision record. `proj/` keeps it citable by Phases 118 and 119 and survives milestone archival,
  which a phase-directory artifact would not. The genuine *decisions* in this phase (the SC4
  discrepancy resolutions) live in a section inside it (D-14).

- **D-04: The registry type is the shared layer identity Phase 118's receipts consume.** 118's
  receipt attests "which layers were confirmed active for this process" over the same enumeration,
  with the same three-way distinction RCPT-03 demands. One type, not two lists that can diverge.
  117 designs it with that consumer in mind.

### Contract Shape & Vocabulary

- **D-05: Four-value outcome vocabulary** — `abort` / `degrade-with-visible-claim` /
  `substitute-equivalent-mechanism` / `fail-open-defect`. This settles the question
  `proj/ADR-116-tool-sandbox-disposition.md` **Consequence 6** explicitly deferred to this phase:
  the `WindowsTokenArm` cascade (`WriteRestricted` / `BrokerLaunch` / `BrokerLaunchNoPty`) responds
  to a layer failure by switching to a different token-construction mechanism that preserves the
  same confinement class — which is neither an abort nor a downgrade. Naming it as a first-class
  outcome keeps the contract honest; folding it into "fail-closed" would hide a materially different
  mechanism, which is exactly what ADR-116 warned against.

- **D-06: `fail-open` is an admissible contract value, but only with a written justification and an
  explicit acceptance.** A row may read fail-open where the layer is hardening-only and its absence
  does not widen the confinement claim — stated in writing, per instance. This keeps the document a
  true description of shipped behaviour (CINT-01's stated intent) while making every instance
  visible and arguable rather than either hidden or auto-classified as a bug.

- **D-07: A `substitute-equivalent-mechanism` row must name its specific alternate mechanism in the
  registry, and the attestation must confirm the substitute actually took effect.** The equivalence
  claim is verified per session, not asserted once in a document. An unconfirmed substitute falls
  through to `abort` or `degrade` per that row's contract. Without this, "substitute" is a loophole
  that launders any failure into a pass.

- **D-08: Expectancy is per-configuration — the registry is a matrix, not a flat list.** Each row
  carries the execution arms in which the layer is expected. This is load-bearing for this fork:
  per-SID WFP is daemon-path only (`nono agent launch`), not direct `nono run`; AppContainer is
  absent on the broker arm; the session-SID DACL grants are only operative on the `WriteRestricted`
  arm (`exec_strategy_windows/mod.rs`, `_applied_dacls` comment). Without per-arm expectancy most
  rows read "unavailable" on an ordinary run and the attestation becomes noise. This also pre-builds
  RCPT-03's "confirmed active / not expected here / expected but unconfirmed" three-state.

### Scope

- **D-09: Windows-only.** CINT-01/02/03 all say "the Windows backend", and the milestone's core
  value is the deny-by-composition model that only Windows has.

- **D-10: The layer set is derived from the code; the six named in CINT-01 are the floor, not the
  ceiling.** Follow the composition out of `prepare_live_windows_launch` /
  `execute_sandboxed` and enumerate what actually participates — the six named (restricted token,
  mandatory integrity label, AppContainer profile + package SID, DACL grants, WFP egress filters,
  minifilter *absence*) plus anything else load-bearing that the derivation finds (candidates to
  return an explicit in-or-out call on: Job Object containment, env sanitization, the interpreter
  coverage gate that already fail-secure-refuses, the broker Authenticode trust gate, proxy egress).
  **Every addition and every exclusion is reasoned on the record** — an exclusion implied by
  omission is a ledger defect. Matches CINT-01's "derived from the code".

- **D-11: Platform-neutral types, `cfg(windows)` population.** The layer-identity enum and
  attestation-status types compile on every host so tests, docs and 118's receipt type work
  anywhere; only the Windows layer population and the probes are `cfg(target_os = "windows")`.
  Avoids a Windows-only island. **Note for planning:** any shared file this touches
  (`execution_runtime.rs`, `cli.rs`, `output.rs`) carries Unix `cfg` branches, so the
  cross-target clippy MUST almost certainly applies — both local gates, no PARTIAL→CI.

- **D-12: Unix non-coverage is a named boundary handed to Phase 119.** The contract states in
  writing that Linux/macOS fail-direction is deliberately not enumerated here and why (Landlock's
  best-effort ABI downgrade at `crates/nono/src/sandbox/linux.rs:308`, `:3544` is the nearest
  analog), and Phase 119's boundary statement picks it up as one of its recorded decisions. Same
  posture Phase 116 used for its post-`v0.71.0` tail: a named boundary, not silence.

### Discrepancy Handling (SC4)

- **D-13: Severity triage decides fix-now vs. record-and-defer.** Fix in-phase when the gap lets
  nono present a confinement claim it did not confirm (the exact failure mode CINT-02 closes) or
  when a layer's absence silently widens the enforced boundary. Otherwise record the actual
  behaviour, justify it under D-06, and name a successor. Bounds the phase by security impact
  rather than by discrepancy count — which matters because this phase also has to ship the
  registry, the attestation and the per-layer tests.

- **D-14: A deferred fix must still downgrade the runtime claim.** When a required fix is too large
  for this phase: the contract row states the **real (worse)** behaviour, never the intended one,
  **and** the attestation treats that layer as unconfirmed so the session's claim downgrades
  accordingly, **and** a successor is named. The honesty property must hold even with a known
  unfixed defect — the defect becomes visible at runtime, not only in a document.

- **D-15: Discrepancies live in a standing section of the `proj/` SPEC** ("Contract vs. code
  discrepancies"), one row each with its resolution (fixed here / deferred to X). They travel with
  the document 118 and 119 read, rather than being archived with the phase directory.

- **D-16: Every discrepancy carries re-runnable evidence in the Phase 116 D-16 shape** — the
  literal command run, its hit count, and the date measured. **The greps must discover their
  targets, not confirm pre-named ones**: search the symbol across the tree rather than grepping a
  path someone already decided was the answer (Phase 115 V-01 — a verification test that scanned one
  function *by name* was blind by construction). File presence is never evidence.

### Self-Attestation

- **D-17: Observed where a probe exists; the fallback is recorded per row.** Query live kernel state
  per layer where an API exists — `GetTokenInformation` for integrity label and restricted SIDs
  (already imported in `exec_strategy_windows/mod.rs`), `IsProcessInJob` for containment, package
  SID on the token for AppContainer — and fall back to configured-only where no probe exists, with
  that fallback stated on the row. The honesty gap this phase closes was precisely a
  configured-but-not-observed claim.

- **D-18: A distinct fourth attestation state — "established, not independently observable" — which
  still downgrades the claim.** Separate from both `confirmed` and `unconfirmed`, so a reader can
  tell "we proved it" from "the call succeeded and we cannot check" from "it failed" without
  out-of-band knowledge. **Accepted consequence, stated up front:** WFP filter state is not readable
  from a non-elevated supervisor on the daemon path, so this state — and therefore a downgraded
  claim — will be the *normal* outcome for the WFP row on ordinary sessions. D-21's per-row default
  is what keeps that from turning into either mass aborts or a meaningless signal.

- **D-19: The supervisor attests; the confined process never does.** The supervisor probes the
  child's token and process state from outside. The confined process must never be the source of
  any claim about its own containment — a compromised child would assert whatever it wants. Stated
  as an invariant so no later plan introduces a child-side reporting path.

- **D-20: Startup only.** One pass, as CINT-02 literally specifies. **Consequence the contract must
  state plainly:** mid-session layer removal (e.g. WFP filters torn down by another actor after
  launch) is outside the claim. Do not let that limit go unwritten.

- **D-21: The attestation gate runs against the real child while it is suspended.** The child is
  already created `CREATE_SUSPENDED` in `launch.rs`; attest its actual token and job membership
  there, then `ResumeThread` or `TerminateProcess`. This is the only placement that observes what
  the OS actually gave the process rather than what the supervisor intended, and it guarantees no
  child instruction executes under an unconfirmed claim.

- **D-22: Abort mechanics — terminate, unwind guards, typed error naming the layer.**
  `TerminateProcess` the suspended child, let the existing `PreparedWindowsLaunch` Drop-order
  discipline unwind (labels → DACL grants → ancestor grants → network), and surface a typed
  `NonoError` with a `NonoDiagnosticCode` naming the failed layer. The operator learns which layer
  failed, not that "something" failed.

- **D-23: Every confined-child path is gated** — direct `nono run`, daemon `nono agent launch`, the
  broker arm (`nono-shell-broker` spawning Low-IL children), and the per-tool-call hook path via
  `claude_code_hook.rs`. An ungated path is a place the old failure mode survives, and the hook path
  is the fork's actual production shape today (sandbox-the-tools).

- **D-24: Measure the attestation cost and record a budget per path in the contract.** The token and
  job probes are cheap local calls — state the measured number rather than asserting "negligible".
  If a probe turns out expensive, that row's depth becomes a recorded trade-off, not a silent
  regression on the per-tool-call hook path. The project constraint is explicit: zero startup
  latency for the Windows backend.

### Downgrade Surface & Policy

- **D-25: The default posture is per-row, not global.** Each registry row's contracted outcome *is*
  its default — some layers abort, some downgrade, each with a written reason. This is what handles
  D-18's WFP reality without either aborting ordinary daemon sessions on a probe limitation or
  blanket-downgrading layers whose absence is genuinely fatal. It also makes the posture reviewable
  row by row instead of hinging on one switch.

- **D-26: The strictness knob is machine policy + a CLI flag, with machine policy winning.** An
  `HKLM\SOFTWARE\Policies\nono` field (the v3.0 enterprise spine, `crates/nono/src/machine_policy.rs`)
  can require layers fleet-wide; a CLI flag exists for local use and **can only tighten, never
  loosen**, what machine policy sets. Consistent with the existing fail-closed policy-load
  discipline (present-but-unreadable already aborts — `machine_policy.rs:19`).

- **D-27: A downgraded claim surfaces on three channels** — human-visible banner text, a typed
  `NonoDiagnosticCode` (so the existing `DiagnosticFormatter` and remediation surface pick it up),
  and a structured security event through `SecurityEventLayer`'s HMAC chain. The event channel is
  the one Phase 118's receipts will use, so that integration work is not thrown away.

- **D-28: Layer-specific downgrade detail stays off channels the confined process can read.**
  Naming which layer is not enforcing is reconnaissance for the process being confined, and in
  supervised mode the child shares the console. Specific layer names go to the operator's channel
  and the audit event; anything the child can read stays coarse. Recorded as a contract rule so no
  later plan regresses it.

### Forced-Unavailable Tests

- **D-29: Generalize the shipped test toggle into one per-layer force-unavailable seam driven off
  the registry.** The precedent already exists and has been reviewed and shipped:
  `set_windows_wfp_test_force_ready`, exposed as a CLI flag (`cli.rs:2211`,
  `command_runtime.rs:92`). Extending it exercises the real launch path end-to-end rather than
  testing around it — which matters because these layers are OS calls, not pure logic.

- **D-30: The seam is compiled out of release builds, not merely refused at runtime.** A
  runtime-gated "turn off a confinement layer" switch is live attack surface in a security tool — an
  env var an attacker can set. **This deliberately diverges from the existing toggle's
  `NONO_TEST_HARNESS` runtime gate.** *Flagged for planning:* that existing toggle is now
  inconsistent with this rule — it either migrates to the compiled-out form in this phase, or the
  inconsistency is recorded as an SC4 discrepancy row under D-13/D-15. Do not leave it unaddressed.

- **D-31: A row that cannot be tested on an ordinary host is a loud gap, never a silent skip.**
  Record it as an explicit unsatisfied entry in the contract and report it as a named
  manual-verification item. CINT-03 already says a row without a test is not satisfied; Phase 115's
  V-01 was exactly a check that read green while being blind by construction. A host-gated test that
  is simply ignored by default reads as green-by-absence.

- **D-32: A meta-test discovers registry rows and asserts each has a forced-unavailable test.**
  Discovery-based, so adding a layer without a test fails the build. Consistent with D-01's
  registry-as-source-of-truth and with the V-01 lesson (prefer tests that discover their targets
  over ones that name them).

### Carried Forward (not re-litigated)

- **D-33: ADR-65 stands.** No minifilter, no driver lifecycle. The minifilter row in the contract is
  an **absence** row, and per-file read policy inside one directory is explicitly not claimed. Phase
  116's D-08 `structurally-blocked` rows citing ADR-65 are the ready citation.
- **D-34: `/gsd:code-review` runs on this phase.** It touches code, and the milestone invariant is
  explicit: executor self-check is not security evidence — Phase 112's review gate caught 4 Critical
  fail-open defects that all 8 executor self-checks passed over.
- **D-35: Cross-target clippy MUST** for any cfg-gated Unix edit — `cross` linux-gnu +
  `cargo-zigbuild` apple-darwin, both local, **no PARTIAL→CI**. See D-11.
- **D-36: SDK STATE/ROADMAP writers stay banned** (five consecutive phases now); two milestones are
  open, so `REQUIREMENTS.md` and `ROADMAP.md` are appended to, never overwritten, and `phases.clear`
  must not run. All commits DCO-signed.

### Claude's Discretion

- Exact SPEC filename under `proj/` (the `proj/` home and non-ADR SPEC form are locked, the name is
  not), and the SPEC's internal section ordering — provided the D-15 discrepancies section, the D-12
  Unix boundary note, the D-20 mid-session limit, and the D-24 latency budget are all present.
- Whether the SPEC is *generated* from the registry or *drift-checked* against a hand-written file
  by a test — D-01 requires only that the registry wins and that drift fails a test.
- Registry table layout, column ordering, and enum/struct shape, provided expectancy-per-arm (D-08),
  the four-value outcome (D-05), the named substitute (D-07), and the enforcing call site are all
  representable.
- Plan and wave breakdown; whether the registry + contract land before or alongside the attestation.
- Which specific candidate layers from D-10 land in or out — the *call* is discretionary, the
  *reasoning on the record* is not.

### Reviewed Todos (not folded)

See `<deferred>`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone definition
- `.planning/REQUIREMENTS.md` — v3.7 section: **CINT-01 / CINT-02 / CINT-03** (this phase),
  RCPT-01/02/03 (Phase 118 — the consumer of D-04's shared type), BOUND-01/02/03 (Phase 119 — the
  consumer of D-12's boundary note), and the **architecture invariants** block (ADR-65 stands;
  composite fails at its seams; the honesty-gap failure mode; structural fixes over spot fixes;
  executor self-check is not security evidence; two milestones open).
- `.planning/ROADMAP.md` — Phase 117 goal + SC1–SC4; Phase 118/119 entries and their stated
  dependency on this phase.

### The question this phase inherits
- `proj/ADR-116-tool-sandbox-disposition.md` — **Consequence 6** (the `WindowsTokenArm`
  third-outcome question, named and explicitly left for Phase 117 to settle — see D-05), and the
  **D-09 fail-direction row** (line ~123) which already describes the fork's fail-direction posture
  in evidence form and must not be contradicted.
- `.planning/phases/116-tool-sandbox-divergence-audit-disposition-adr/116-CONTEXT.md` — D-16
  (evidence discipline reused as D-16 here), D-08 (ADR-65 structurally-blocked citation).

### Fork invariants
- `proj/ADR-86-library-boundary-convergence.md` — policy-free library boundary and the Windows
  denial-rendering CLI-side carve-out (grounds D-02).
- `.planning/architecture/adr-65-minifilter-go-no-go.md` — the standing No-go/Conditional-go verdict
  behind the minifilter *absence* row (D-33).
- `CLAUDE.md` — Library-vs-CLI boundary table; cross-target clippy MUST/NEVER; unwrap policy;
  fail-secure and path-security rules.
- `.planning/templates/cross-target-verify-checklist.md` — the two cross-target clippy gates
  (relevant per D-11/D-35).

### The Windows layers being enumerated
- `crates/nono-cli/src/exec_strategy_windows/mod.rs` — `prepare_live_windows_launch`,
  `PreparedWindowsLaunch` (the guard set and its **Drop-order discipline**, ~lines 293–333),
  `NetworkEnforcementGuard` (`FirewallRules` vs `WfpServiceManaged`, ~line 280),
  `WindowsSupervisorDenyAllApprovalBackend` (~line 259, a named fail-closed backend),
  `set_windows_wfp_test_force_ready` (the D-29 seam precedent).
- `crates/nono-cli/src/exec_strategy_windows/restricted_token.rs` — restricted-token layer.
- `crates/nono-cli/src/exec_strategy_windows/labels_guard.rs` — mandatory integrity label ACEs.
- `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs` — session-SID grants, ancestor traverse,
  ancestor read-attributes (all arm-dependent — grounds D-08).
- `crates/nono-cli/src/exec_strategy_windows/network.rs` — WFP egress filters and readiness probing.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — `CREATE_SUSPENDED` spawn (the D-21 gate
  point), `select_windows_token_arm()` and the `WindowsTokenArm` cascade (~1237–1278, the D-05
  substitution outcome).
- `crates/nono-cli/src/exec_strategy_windows/supervisor.rs` — supervised-run lifecycle.
- `crates/nono-cli/src/execution_runtime.rs` — `execute_sandboxed()`; the `BrokerLaunchNoPty` **XOR**
  `WriteRestricted` token-arm split and the `.NET`/PowerShell-CLR-under-`WRITE_RESTRICTED` finding.
- `crates/nono/src/sandbox/windows.rs` — `Sandbox::windows_filesystem_policy`,
  `validate_windows_launch_paths` (the interpreter coverage gate that already fail-secure-refuses —
  a D-10 candidate), and the `UnsupportedPlatform` returns that are the original honesty-gap site.
- `crates/nono-shell-broker/src/main.rs` — the Medium-IL broker arm (a D-23 gated path).
- `crates/nono-cli/src/claude_code_hook.rs`, `crates/nono-cli/data/hooks/nono-tool-hook.ps1` — the
  per-tool-call hook path (a D-23 gated path; the D-24 latency-sensitive one).

### Surfaces the attestation plugs into
- `crates/nono-cli/src/telemetry/mod.rs` — `SecurityEventLayer` (line 218), `advance_chain`,
  `TELEMETRY_CHAIN_DOMAIN` (line 75): the HMAC-chained event channel D-27 uses and Phase 118's
  receipts will extend. Distinct from `audit_integrity.rs`'s `CHAIN_DOMAIN` — the two chains must
  stay independent (D-06 of that module).
- `crates/nono/src/diagnostic/codes.rs`, `report.rs`, and `crates/nono/src/error.rs` — the
  `NonoDiagnosticCode` surface and `NonoError::diagnostic_code()`/`remediation()` methods D-22/D-27
  emit through.
- `crates/nono/src/machine_policy.rs` — the `HKLM\SOFTWARE\Policies\nono` spine for D-26, including
  its established abort-vs-degrade discipline (D-07 egress aborts, D-14 telemetry degrades).
- `crates/nono-cli/src/output.rs` — `print_banner` (line 32), the human-visible claim surface.
- `crates/nono-cli/src/cli.rs:2211`, `crates/nono-cli/src/command_runtime.rs:89–92` — the existing
  `NONO_TEST_HARNESS`-gated toggle: D-29's model and D-30's flagged inconsistency.

### Platform behaviour
- `Skill("spike-findings-nono")` — engine-agnostic confinement patterns, landmines, and verified
  Windows OS-behaviour facts (SEED-004). Load before writing the layer derivation or the probe set.
- `crates/nono/src/sandbox/linux.rs:308`, `:1233`, `:3544` — Landlock's best-effort ABI downgrade,
  cited by D-12 as the nearest Unix analog being deliberately left out of scope.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`CREATE_SUSPENDED` spawn already in `launch.rs`** — D-21's attestation window exists; the gate
  slots into a control point the code already has rather than restructuring the launch path.
- **`PreparedWindowsLaunch`'s reverse-of-declaration Drop-order discipline** — labels → session-SID
  DACLs → ancestor traverse → ancestor read-attrs → network. D-22's abort path unwinds through
  machinery that is already correct and already commented for exactly this ordering concern.
- **`set_windows_wfp_test_force_ready` + its `NONO_TEST_HARNESS` CLI flag** — a shipped,
  reviewed fault-injection precedent to generalize (D-29), and the one whose runtime gate D-30
  supersedes.
- **`GetTokenInformation` / `TokenIntegrityLevel` / `TokenElevation` / `IsProcessInJob`** — already
  imported in `exec_strategy_windows/mod.rs`; D-17's probes are largely already-available calls, not
  new FFI surface.
- **`SecurityEventLayer`'s HMAC chain** — D-27's structured channel is built and chained; the
  attestation event is a new record type on an existing spine, and Phase 118 extends the same one.
- **`WindowsSupervisorDenyAllApprovalBackend`** — a named fail-closed backend already in the tree; a
  worked example of the posture the contract is formalizing.

### Established Patterns
- **No layer enum exists today.** `NetworkEnforcementGuard` (two private variants) is the only
  per-layer type in the tree. D-01's registry is genuinely new construction, not a refactor — plan
  for it as such.
- **Arm-dependent layer applicability is already encoded in comments, not types** — the `_applied_dacls`
  block explains that the session-SID grant is "OPERATIVE on the WriteRestricted arm; inert but
  harmless elsewhere". D-08's matrix promotes that prose into the registry.
- **Machine policy already distinguishes abort from degrade** (`machine_policy.rs`: egress errors
  abort, malformed telemetry degrades) — D-25/D-26 extend an existing posture rather than inventing one.
- **Self-enforcing source-scan tests exist** (`crates/nono-cli/tests/*.rs` using `CARGO_MANIFEST_DIR`
  /`include_str!`) — D-32's meta-test has house precedent to follow.

### Integration Points
- **D-04 → Phase 118**: the registry type is the receipt's layer vocabulary, and D-18's four-state
  attestation status is RCPT-03's three-state plus the unobservable case. Design once.
- **D-12 → Phase 119**: the Unix non-coverage note and the ADR-65 minifilter *absence* row are two of
  BOUND-02/BOUND-03's inputs, handed over as written decisions rather than re-derived.
- **D-05 → ADR-116**: settling the third-outcome question closes an explicitly open item in an
  Accepted ADR. Whatever this phase decides should be reflected back so ADR-116's Consequence 6 does
  not read as perpetually unresolved.
- **D-11/D-35 → cross-target gates**: touching `execution_runtime.rs` / `cli.rs` / `output.rs` puts
  Unix `cfg` branches in the blast radius. Assume both local clippy gates are required.

</code_context>

<specifics>
## Specific Ideas

- **The tension to design around, stated plainly:** D-18 ("unobservable still downgrades") combined
  with the fact that WFP filter state is unreadable from a non-elevated supervisor means the WFP row
  will normally sit in the unobservable state and normally downgrade the claim. That is accepted
  knowingly. D-25's per-row default is the mechanism that keeps it from becoming either mass aborts
  or a signal nobody reads. Do not "solve" this by quietly promoting unobservable to confirmed.
- **The failure mode has a real precedent in this repo** — the honesty gap where `Sandbox::apply()`
  returned `UnsupportedPlatform` while the CLI enforced via WFP. The contract should cite it as the
  worked example of why configured ≠ observed, not treat it as hypothetical.
- **`fail-open-defect` as a vocabulary value is deliberate.** It is not a synonym for "fail open" —
  it is the label for a row where the code fails open and *shouldn't*, which under D-13/D-14 forces
  either a fix or a downgraded runtime claim plus a named successor. A row justified under D-06 is
  `fail-open`, not `fail-open-defect`; keep the two distinct.
- **A contract row without a test is not satisfied** is CINT-03's own wording — D-31 and D-32 exist
  so that this is enforced by the build rather than by a reviewer noticing.

</specifics>

<deferred>
## Deferred Ideas

- **Linux Landlock ABI-downgrade and macOS Seatbelt fail-direction enumeration** — named boundary
  per D-12, handed to Phase 119's boundary statement as a recorded decision. Not a FUT item unless
  Phase 119 decides otherwise.
- **Continuous / periodic re-attestation** — D-20 locks startup-only. Mid-session layer removal is
  outside the claim and the contract says so; making it in-scope is its own work item.
- **Per-session enforcement receipts** → Phase 118 (RCPT-01/02/03). 117 lands the shared layer type
  (D-04), not the receipt.
- **Security-model boundary statement + state-of-the-art decision log** → Phase 119
  (BOUND-01/02/03).
- **Engine-agnosticism / decoupling confinement from Claude Code's PreToolUse contract** — carried
  forward from Phase 116's deferred list; belongs with the engine-abstraction / `nono-agentd` line,
  not here.
- **Gray areas raised but not discussed** (available if planning needs them): how the minifilter
  *absence* row is expressed and tested when a structurally-nonexistent layer has no probe and no
  failure mode; whether the contract's call-site citations are drift-checked by the same meta-test
  that enforces D-32; and how the SPEC generation/drift check is wired into `make ci`.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` — matched at 0.6 on generic keywords ("clean", "confirm",
  "phase", "code"). MSI prerequisite work owned by v3.5 Phase 106; unrelated to fail-direction or
  attestation. Phase 116 reviewed and declined the same todo.
- `20260611-poc-cert-broker-clean-host.md` — matched at 0.6 on "clean"/"phase". The "broker" here is
  the POC **certificate** broker for clean-host install, not the Low-IL shell broker. Host-gated
  v3.5 distribution item owned by v3.5 Phase 106. Declined by Phases 108 and 116 for the same reason.

</deferred>

---

*Phase: 117-fail-direction-contract-startup-self-attestation*
*Context gathered: 2026-08-09*
