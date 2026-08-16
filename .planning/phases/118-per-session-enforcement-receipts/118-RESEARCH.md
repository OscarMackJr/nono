# Phase 118: Per-Session Enforcement Receipts - Research

**Researched:** 2026-08-16
**Domain:** Windows security attestation / tamper-evident audit records, in an existing Rust
capability-sandboxing codebase
**Confidence:** HIGH for what exists today (all claims below are symbol-verified against the
current tree); MEDIUM-LOW for what the new receipt machinery must look like, because — as this
research repeatedly found — the phase's central assumption ("read what 117 already computes and
serialize it") is not fully true. See Finding 1.

## Summary

Phase 117 shipped three *independent* gate implementations that each decide "proceed or abort,"
but **none of the three retains a complete 13-row census** — every one of them is structured to
stop looking the moment it has enough information to decide. `nono-cli`'s `attest_and_decide`
computes a per-row verdict inside a loop but only ever returns an *aggregate* decision (a single
`Abort{layer}`, or a `Vec<LayerId>` of downgraded rows) — 11 of 13 rows' `Confirmed`/
`NotApplicable` verdicts are discarded the moment the loop passes over them. `nono-agentd`'s
`daemon_attest_and_decide` is a 5-layer early-return chain: if the first check fails, the other
four are never even probed. `nono-shell-broker`'s `broker_resume_gate` checks only the (at most
two) layers named in its wire contract and knows nothing about the other eleven `LayerId` rows at
all — it has no access to `layer_registry.rs`. RCPT-01's "full census of every registry row" is
therefore new construction in all three binaries, not a serialization pass over existing data.
This is the single most load-bearing finding in this research and the planner should size Wave 0
around it.

The second load-bearing finding concerns D-11/RCPT-02's "same terms as the existing HMAC-chained
`SecurityEventLayer`." That chain's HMAC key is generated fresh in-process at
`SecurityEventLayer::new` and explicitly zeroized on `Drop` — it is never persisted anywhere. That
construction is tamper-evident *within a live process* but structurally **cannot** be verified
after the process exits, because nobody — including an operator — retains the key. D-09 says the
governance consumer is "the HMAC key holder," and D-10's own precedent
(`nono audit verify`, `AuditVerifyArgs`) actually recomputes and compares against
`crates/nono/src/audit.rs`'s *separate*, *keyless* SHA-256 hash chain (`CHAIN_DOMAIN_ALPHA`), not
the HMAC one. Mirroring "the existing HMAC-chained `SecurityEventLayer`'s construction" gets you an
in-process-only tamper-evidence property; achieving RCPT-02's actual requirement (a downstream
consumer can verify *after the fact*) needs a **persistent** key or a keyless construction — neither
of which the phase's canonical refs currently name as a decision. This needs to be raised at
planning/discuss time, not silently resolved by the executor.

Everything else in this phase is comparatively well-precedented: the D-03 write point
(`apply_startup_attestation_gate`, `launch.rs:1552`) is real and sits exactly where CONTEXT.md says,
immediately before `resume_contained_process` (`launch.rs:2586`); the mandatory-label half of D-08's
sink guard reuses an existing, well-documented Win32 wrapper
(`crates/nono/src/sandbox/windows.rs::try_set_mandatory_label`); the DACL-grant half has grant/read
wrappers but **no existing deny-ACE wrapper** — that part is genuinely new, though a low-risk
extension of an existing `SetEntriesInAclW` pattern; and D-14's source-scan idiom has three solid,
directly-copyable precedents already in `crates/nono-cli/tests/`.

**Primary recommendation:** Treat "build the full 13-row census, on all three arms, without any
early-return shortcuts" as its own Wave-0-adjacent task before the chain/sink/command-family work —
everything downstream (D-01, D-13, D-14's completeness scan) depends on that data existing at all.
Separately, raise the ephemeral-key-vs-persistent-verification tension as an explicit open question
for the planner/operator rather than assuming D-11 resolves it.

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** The receipt is a full census of every registry row (13), on every confined session —
  not filtered, not downgrade-only.
- **D-02:** One receipt type, carrying a terminal session-outcome field (`ran` / `refused`).
- **D-03:** The receipt is written ONCE, at the D-21 attestation gate, before `ResumeThread`.
- **D-04:** Emitter failure degrades visibly by default; `HKLM\SOFTWARE\Policies\nono` can make it
  fail-closed. Mirrors the existing abort-vs-degrade split in `machine_policy.rs`.
- **D-05:** Strict identity — opaque session id + pid only. No path-derived value, not even salted.
- **D-06:** A dedicated operator-ACL'd receipt sink is primary; Windows Event Log is demoted to a
  coarse pointer.
- **D-07:** The Event Log readability assumption is NOT cleared (live `wevtutil` measurement in
  `<specifics>` below); D-06 makes the phase independent of the answer.
- **D-08:** The sink is guarded by a DENY ACE on the per-session synthetic SID AND package SID,
  PLUS a `NO_READ_UP` mandatory label. Both, not either — supervisor and confined child run as the
  same user, and `WRITE_RESTRICTED` tokens check restricting SIDs on writes only (reads bypass).
- **D-09:** Governance consumer is same-trust-domain (the HMAC key holder). Third-party-verifiable
  receipts deferred to Phase 119.
- **D-10:** A new `nono receipt list | show | verify` command family, reusing `AuditCommands`'
  shape verbatim (`cli.rs:3498`).
- **D-11:** Receipts get their OWN chain domain, same construction/discipline as
  `TELEMETRY_CHAIN_DOMAIN`/core `CHAIN_DOMAIN`, but a separate chain instance/sink (interleaving on
  one chain across two sinks would make independent verification structurally impossible).
- **D-12:** The receipt type lives in `crates/nono` (core); `LayerId` is promoted to core alongside
  `LayerAttestationStatus` as policy-free vocabulary. Policy (`ArmExpectancy`, enforcing call sites)
  stays in `layer_registry.rs`.
- **D-13:** All four `LayerAttestationStatus` states carried verbatim — RCPT-03's three are a FLOOR.
- **D-14:** SC2 enforced by a type-allowlist source scan AND a sentinel round-trip test, each with
  its own perturbation proof.
- **D-15:** Whoever attests, writes — one uniform rule, per-writer chain segments correlated by
  session id, no cross-process lock. A broker-arm session yields TWO receipts.
- **D-16:** The daemon records faithfully; `Unconfirmed`-on-`Proceed` findings get triaged under
  117's D-13 severity rule, not smoothed over.
- **D-17:** The per-tool-call hook path gets a receipt per invocation, with a MEASURED latency
  budget (D-24).
- **D-18 (carried):** Windows-only.
- **D-19 (carried):** The supervisor attests; the confined process never does.
- **D-20 (carried):** Startup-only; no mid-session claim.
- **D-21 (carried):** ADR-65 stands — `MinifilterAbsence` is `NotApplicable`, never `Unconfirmed`.
- **D-22 (carried):** `/gsd:code-review` runs on this phase.
- **D-23 (carried):** Cross-target clippy MUST for any cfg-gated Unix edit — both local gates, no
  PARTIAL→CI. `LayerId` promotion into core + `nono receipt` in `cli.rs` both touch files with Unix
  `cfg` branches.
- **D-24 (carried):** SDK STATE/ROADMAP writers stay banned; append-only; DCO-signed commits.

### Claude's Discretion

- Sink location and layout (`%PROGRAMDATA%\nono\receipts` vs `%LOCALAPPDATA%`), directory
  structure, file naming.
- Retention/rotation policy (provided rotation never silently truncates a chain segment).
- On-disk serialization format and schema versioning (JSONL is the obvious fit; open).
- Whether the receipt records token arm and entry path as fields (strongly implied by D-15).
- Whether the coarse Event Log pointer (D-06) earns its place at all.
- How the census stays in sync with the registry as layers are added (discovery-based meta-test in
  the shape of `layer_registry_meta_test.rs` is the obvious answer). NOT discretionary: adding a
  `LayerId` variant without a receipt census row must fail the build.
- Plan/wave breakdown; whether core type + chain land before or alongside sink + command family.

### Deferred Ideas (OUT OF SCOPE)

- Third-party-verifiable (asymmetric) receipts — handed to Phase 119 as a named recorded decision.
- Empirical proof of Event Log readability from a real confined child (D-07 narrowed, did not
  settle) — stays a tracked open item, not blocking.
- Continuous/periodic re-attestation (D-20 locks startup-only).
- Security-model boundary statement + SOTA decision log → Phase 119 (BOUND-01/02/03).
- Tool-sandbox verdict execution → Phase 120.
- Sink retention/rotation policy, on-disk schema versioning, whether the Event Log pointer earns
  its place, and whether token-arm/entry-path are first-class receipt fields — all gray areas
  explicitly available to planning, not decided.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RCPT-01 | Every confined session emits a receipt naming which layers were confirmed active; content-free (no paths/args/payload) | Finding 1 (census must be newly built on all 3 arms); D-05 identity shape verified against `LayerId`/`LayerAttestationStatus` in `crates/nono/src/attestation.rs`; D-14 test precedents in `crates/nono-cli/tests/` |
| RCPT-02 | Tamper-evident on the same terms as the existing HMAC-chained audit events | Finding 2 (ephemeral-key tension); exact HMAC construction verified in `crates/nono-cli/src/telemetry/mod.rs`; core keyless chain verified in `crates/nono/src/audit.rs`; `AuditCommands`/`AuditVerifyArgs` verified in `cli.rs:3498-3590` |
| RCPT-03 | Four-state vocabulary distinguishes confirmed / not-expected / expected-but-unconfirmed, no out-of-band knowledge needed | `LayerAttestationStatus` (4 variants) verified in `crates/nono/src/attestation.rs:74-113`; D-12's core-promotion argument grounded against ADR-86 |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Receipt data type + chain construction (`EnforcementReceipt`, `RECEIPT_CHAIN_DOMAIN`) | Core library (`crates/nono`) | — | D-12: policy-free vocabulary + primitive, reachable uniformly from all 3 binaries (only 2 of which link `nono-cli` code at all) |
| Per-row census computation (walking all 13 `LayerId`s without early return) | CLI / binary-specific (`nono-cli`, `nono-agentd`, `nono-shell-broker`) | — | Each binary owns a different subset of live probes and a different (entry_path) expectancy; the *policy* of what's expected on which arm is CLI-side (`layer_registry.rs`), reachable only from `nono.exe` |
| Receipt sink (file write, ACL/label application) | CLI / binary-specific | Core (Win32 ACL primitives) | The DENY-ACE + mandatory-label mechanics are OS primitives that belong in `crates/nono/src/sandbox/windows.rs` (where `try_set_mandatory_label`/`grant_sid_*_on_path` already live); the *decision of when/where to write* is CLI/daemon/broker-specific |
| `nono receipt list\|show\|verify` command family | CLI (`nono-cli/src/cli.rs`) | — | Mirrors `AuditCommands` exactly (D-10); command routing/UX is explicitly CLI territory per CLAUDE.md's library-vs-CLI table |
| Machine-policy "require receipts" gate | Core library (`crates/nono/src/machine_policy.rs`) | — | Extends the existing `RequiredLayersPolicy`/degrade-not-abort pattern in the same file |

## Finding 1 (load-bearing): No binary computes a full 13-row census today

**Verified against the live tree, not assumed.**

1. **`nono-cli`'s `attest_and_decide`** (`crates/nono-cli/src/exec_strategy_windows/attestation.rs:659-687`)
   calls `decide_from_entries` (`:503-642`), which loops `layer_registry::all_entries()` (13 rows)
   and computes `classify_row(entry, input)` — a genuine per-row `RowVerdict{status, partially_established}`
   — for every row. **But the loop only ever accumulates into two things**: a `Vec<LayerId>` named
   `downgraded` (pushed only on specific non-abort outcomes), and an early `return
   AttestationDecision::Abort{layer, status}` the moment any row fails its contract. The per-row
   verdict for every OTHER row (the 11 or 12 that were fine) is computed, inspected, and then
   thrown away — `decide_from_entries` has no data structure that survives the loop except the
   3-variant `AttestationDecision` enum (`Proceed` / `ProceedDowngraded{downgraded}` /
   `Abort{layer, status}`, `:125-148`). **To build a receipt this function (or a sibling that
   shares `classify_row`) must be changed to return `Vec<(LayerId, LayerAttestationStatus,
   bool /* partially_established */)>` for all 13 rows, in addition to (or instead of) the
   existing aggregate decision** — and every one of `attest_and_decide`'s ~15 unit tests
   (`attestation.rs:757-1219+`) that assert on the aggregate `AttestationDecision` will need a
   parallel assertion against the new census output, or the tests will pass while the census is
   silently wrong (this is exactly the class of gap Phase 117's D-13/D-14/D-16 severity rule
   exists to catch).

2. **`nono-agentd`'s `daemon_attest_and_decide`** (`crates/nono-cli/src/agent_daemon/launch.rs:1417-1500`)
   is a hand-written **early-return chain checking only 5 of the 13 `LayerId`s**
   (`AppContainerProfile`, `JobObjectContainment`, `WfpEgressFilters`, `DaclPackageSidGrant`,
   `DaclAncestorTraverse`) — see the four `return DaemonAttestationDecision::Abort{...}` sites at
   lines 1435, 1446, 1455, 1470. If `AppContainerProfile` fails, `JobObjectContainment` (and every
   layer after it) is **never even probed** — there is no code path that would classify it as
   `Unconfirmed` vs. "not evaluated" because the function returns before reaching it. The other 8
   `LayerId`s (`RestrictedToken`, `MandatoryIntegrityLabel`, `DaclSessionSidGrant`,
   `DaclAncestorReadAttrs`, `FirewallRulesEgress`, `MinifilterAbsence`,
   `BrokerAuthenticodeTrustGate`, `InterpreterCoverageGate`) are not modelled by this function at
   all — the daemon binary has no access to `layer_registry.rs`'s `(EntryPath::Daemon, None)`
   expectancy cells (confirmed: `nono-agentd.rs`'s `#[path]` includes are exactly
   `agent_daemon/mod.rs`, `telemetry/mod.rs`, `agent_daemon/telemetry_init.rs` — no
   `exec_strategy_windows`). A daemon receipt with a genuine 13-row census therefore needs either
   (a) a small, independently-maintained daemon-side expectancy table (kept in sync with
   `layer_registry.rs` via a discovery-based cross-check test — there is already a strong precedent
   for this exact pattern: `daemon_decision_enum_variants`/the variant-name cross-check test at
   `agent_daemon/launch.rs:2450-2570` that keeps `DaemonAttestationDecision` in sync with the CLI's
   `AttestationDecision` by source-scanning both files), or (b) restructuring
   `daemon_attest_and_decide` itself to keep probing every modelled layer even after one fails
   (changing its fail direction from "stop at first failure" to "collect all, then decide" — a
   behavior change beyond a pure serialization pass).

3. **`nono-shell-broker`'s `broker_resume_gate`** (`crates/nono-shell-broker/src/main.rs:369-453`)
   is the narrowest of the three: `BROKER_ATTESTABLE_LAYERS` (`:321`) is a hardcoded 2-element list
   — `["AppContainerProfile", "MandatoryIntegrityLabel"]` — and the function only checks whichever
   of those two names appear in the `NONO_BROKER_REQUIRED_LAYERS` wire-contract string it was
   handed. It has zero knowledge of the other 11 `LayerId`s (it cannot import `layer_registry.rs` —
   confirmed: `nono-shell-broker/Cargo.toml` depends only on `nono` core, `thiserror`, `tracing`,
   `windows-sys`). For the broker's receipt to carry a full 13-row census, nono-cli must either (a)
   extend `NONO_BROKER_REQUIRED_LAYERS`-style wire-contract data to also communicate the
   *NotApplicable* rows for `(EntryPath::Broker, ...)` so the broker can populate the other 11 rows
   without guessing, or (b) the broker hardcodes "everything except AppContainerProfile and
   MandatoryIntegrityLabel is NotApplicable on this arm" — which duplicates registry knowledge a
   third time and needs its own drift guard.

**Consequence for planning:** D-01 ("full census... on every confined session") is not a
serialization task layered on top of Phase 117's work. It requires: (a) a non-short-circuiting
census-computation change in `nono-cli` (straightforward — `classify_row` already exists and is
pure); (b) a harder decision in `nono-agentd` about whether to restructure the early-return chain
or accept a "not evaluated because an earlier required layer already aborted" sub-case (which is
not covered by any of D-13's four states — flag as an open question, see below); and (c) new
wire-contract or hardcoded-and-guarded knowledge in `nono-shell-broker` to classify the 11 rows it
doesn't check. This is a bigger unit of work than "add a struct and serialize it," and the planner
should size Wave 0 (or an early wave) around it explicitly, with its own tests, before the
chain/sink/command-family plumbing.

**Open sub-question this raises, not resolved by CONTEXT.md:** what does the daemon's receipt say
for a `LayerId` whose probe was never reached because an *earlier* required layer already
`Abort`ed? None of D-13's four states (`Confirmed`, `EstablishedNotIndependentlyObservable`,
`Unconfirmed`, `NotApplicable`) precisely means "expected on this arm, but the launch was refused
before this layer's own probe ran." Folding it into `Unconfirmed` is defensible (it was, in fact,
not confirmed) but loses the "we never even looked" distinction Phase 117's whole honesty argument
is built on. The planner should either (a) explicitly rule this maps to `Unconfirmed` with a written
rationale, or (b) restructure `daemon_attest_and_decide` to probe every modelled layer regardless of
earlier failures (recommended — it is a bounded, mechanical change and avoids inventing a 5th
vocabulary value mid-phase, which D-13 permits but which none of the canonical refs anticipated).

## Finding 2 (load-bearing): The "existing HMAC-chained" precedent cannot be verified post-process

**Two structurally different tamper-evidence constructions already exist in this codebase; D-11/RCPT-02
point at the wrong one for D-09's stated consumer.**

- **`crates/nono-cli/src/telemetry/mod.rs`'s `SecurityEventLayer` chain** (the one RCPT-02's literal
  text and D-11 both name): `advance_chain` (`:126-153`) computes
  `HMAC-SHA256(chain.key, TELEMETRY_CHAIN_DOMAIN || prev_head || TELEMETRY_EVENT_DOMAIN ||
  event_bytes)`. `ChainState.key` (`:87-94`) is documented as "an ephemeral 32-byte random key
  generated at `SecurityEventLayer` construction time" — confirmed at `SecurityEventLayer::new`
  (`:319`, uses `OsRng`-filled `Zeroizing<[u8;32]>`) — and `Drop for ChainState` (`:96-103`)
  explicitly zeroizes it. **The key is never written to disk, never exported, never
  recoverable after the process exits.** This construction proves tamper-evidence to *anyone
  observing the live process*, but is cryptographically unverifiable by anyone — including the
  operator — once the process that built it has terminated, because there is no persisted secret
  to recompute the HMAC with. `hmac = "0.13"` (`crates/nono-cli/Cargo.toml:62`).
- **`crates/nono/src/audit.rs`'s ledger chain** (the one D-10's actual command precedent verifies):
  `hash_chain` (`:658-669`) computes `SHA256(CHAIN_DOMAIN_ALPHA || prev || leaf_hash)` — **no key
  at all**, a plain content-addressed hash chain (`sha2 = "0.11"` at the workspace root
  `Cargo.toml:37`). `AuditVerifyArgs`'s doc comment (`cli.rs:3503-3510`) confirms `nono audit
  verify` "re-reads `audit-events.ndjson`, recomputes the per-event leaf hash, the hash-chain head,
  and the Merkle root... then fail-closes if any commitment... does not match" — this recompute-
  and-compare needs no secret. Its integrity guarantee is "nobody edited this file without leaving
  a hash mismatch," not "only the key holder could have produced this," and its optional
  strengthening path is sigstore sign/verify (mentioned in CONTEXT.md's deferred asymmetric-receipt
  item), not an HMAC.

**The tension:** D-09 says the governance consumer is "the HMAC key holder (operator / fleet
admin)" — implying a *persistent, shared* key an operator retains and later uses to verify. D-11
says receipts should use "the identical construction... `Hmac<Sha256>(domain || prev_head ||
event_domain || bytes)`... same advance-under-mutex discipline" as the telemetry chain — but that
chain's key is *deliberately ephemeral and non-recoverable by design* (it exists specifically so a
compromised confined child cannot forge chain continuation, not so an operator can verify offline).
Literally copying the telemetry chain's key lifecycle into the receipt chain would produce receipts
an operator can never actually verify after the session ends — which contradicts D-09's premise and,
more importantly, contradicts the actual use case ("a downstream governance consumer can detect an
edited receipt," RCPT-02) for anything except live tailing.

**This was not surfaced or resolved in CONTEXT.md's `<decisions>` block** — D-11's "decisive
technical reason" section addresses *why receipts need their own chain domain* (correctly), but does
not address *where the receipt chain's key comes from or how it survives process exit*. This is a
genuine open question the planner must resolve, not an implementation detail the executor can
default their way through — the two obvious defaults (reuse the ephemeral in-process key, vs. adopt
the keyless core-audit construction) produce meaningfully different security properties, and CLAUDE.md's
"fail secure... never silently degrade" principle argues against picking silently. Candidate
resolutions to present to the planner/operator: (a) a persistent per-machine or per-fleet HMAC key
sourced from the Windows keystore (the `keyring` crate is already a workspace dependency —
`crates/nono/src/keystore.rs` — and used for credential loading, though not yet for this purpose);
(b) drop the "HMAC" framing for receipts and adopt the core audit module's keyless hash-chain +
optional sigstore-signature construction instead, satisfying RCPT-02's literal words ("tamper-evident,
detects an edited receipt") without D-11's more specific HMAC claim; (c) accept that receipts are
only verifiable *within the emitting process's own lifetime* for now (contradicts D-09's "downstream
governance consumer" framing, so unlikely to be acceptable, but should be named and explicitly
rejected rather than left unconsidered).

## Standard Stack

### Core (already workspace dependencies — no new external packages required)

| Library | Version (verified in-tree) | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `hmac` | 0.13 (`crates/nono-cli/Cargo.toml:62`) | HMAC-SHA256 for the telemetry chain construction (D-11 mirrors this) | Already the exact primitive the phase's own canonical construction uses |
| `sha2` | 0.11 (workspace `Cargo.toml:37`) | SHA-256 for the core audit keyless hash chain | Already used by `crates/nono/src/audit.rs` |
| `serde` / `serde_json` | workspace-pinned (`serde_json = "1.0.149"`) | Receipt struct (de)serialization | Existing house convention for the ndjson audit ledger |
| `zeroize` | workspace dependency (used in `telemetry/mod.rs`, `keystore.rs`) | If the receipt chain adopts a persistent key, it must be held `Zeroizing` per CLAUDE.md's memory-handling rule | Already the house pattern for `ChainState.key` |
| `clap` (v4) | workspace-pinned | `nono receipt list\|show\|verify` subcommand family | Mirrors `AuditCommands` exactly |
| `windows-sys` | 0.59 | `SetNamedSecurityInfoW`/`SetEntriesInAclW` for the D-08 sink guard | Already wrapped in `crates/nono/src/sandbox/windows.rs`; extend, don't re-import raw FFI |

**No new external packages are needed for this phase.** Every primitive the locked decisions call
for (HMAC/SHA-256 chaining, JSON/ndjson serialization, clap subcommands, Win32 ACL/label calls) is
already a workspace dependency with an in-tree usage precedent to copy. **Package Legitimacy Audit
is therefore N/A — skip; this section exists only because the template requires stating that
explicitly rather than omitting it silently.**

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| A dedicated receipt chain domain constant + ephemeral key (literal D-11 reading) | Reuse the core audit module's keyless hash-chain + optional sigstore signature | Resolves Finding 2's post-process-verifiability gap; but is not literally "HMAC-chained" per RCPT-02's wording, so needs an explicit written justification if chosen (see Finding 2) |
| Hand-rolled deny-ACE construction | Extend `crates/nono/src/sandbox/windows.rs`'s existing `SetEntriesInAclW`-based grant functions (`grant_sid_write_on_path` etc.) with a `DENY_ACCESS` variant | Same underlying Win32 call (`SetEntriesInAclW`), same file, same test patterns already exist — do not introduce a second ACL-manipulation code path |

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────────────────┐
                    │  Three independent CREATE_SUSPENDED gate sites          │
                    │  (D-15: whoever attests, writes)                        │
                    └─────────────────────────────────────────────────────────┘

  nono.exe (DirectCli)          nono-shell-broker.exe (Broker)      nono-agentd.exe (Daemon)
  launch.rs:2569                main.rs::run (~L735-960)            agent_daemon/launch.rs (~L900-970)
  apply_startup_attestation_    broker_resume_gate (L369-453)       daemon_attest_and_decide (L1417-1500)
  gate()                              │                                    │
        │                             │  KNOWS ONLY 2 OF 13 LAYERID ROWS   │  EARLY-RETURNS AFTER FIRST
        │  Discards per-row           │  (BROKER_ATTESTABLE_LAYERS)       │  FAILURE — 5 OF 13 ROWS MODELED,
        │  census today               │                                    │  REST NEVER PROBED
        ▼                             ▼                                    ▼
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  NEW: per-row census builder (13 LayerId rows, all 3 binaries)              │
  │  — must NOT early-return; must classify NotApplicable rows even where      │
  │    the binary has no local probe (broker/daemon need registry knowledge    │
  │    they don't currently have access to — Finding 1)                        │
  └─────────────────────────────────────────────────────────────────────────────┘
        │
        ▼
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  NEW: EnforcementReceipt (crates/nono core, D-12)                           │
  │  — session id (opaque) + pid + outcome(ran/refused) + [LayerId × Status]    │
  │  — NO paths, NO args, NO payload (D-05) — enforced by D-14's scan + sentinel│
  └─────────────────────────────────────────────────────────────────────────────┘
        │
        ▼ (before ResumeThread — D-03; same write point as the existing
        │  ProceedDowngraded banner/audit-event emission in
        │  apply_startup_attestation_gate)
        ▼
  ┌──────────────────────────────┐        ┌───────────────────────────────────┐
  │ NEW: receipt chain domain     │        │ NEW: dedicated receipt sink        │
  │ (D-11) — key-lifecycle        │───────▶│ (D-06) — %PROGRAMDATA%\nono\       │
  │ decision is OPEN (Finding 2)  │        │ receipts\ or %LOCALAPPDATA%\...    │
  └──────────────────────────────┘        │ guarded by DENY ACE (session SID + │
                                           │ package SID) + NO_READ_UP label    │
                                           │ (D-08) — reuses                    │
                                           │ try_set_mandatory_label() +        │
                                           │ a NEW deny-ACE variant of the      │
                                           │ existing grant_sid_*_on_path()     │
                                           │ pattern (windows.rs)               │
                                           └───────────────────────────────────┘
                                                          │
                                                          ▼
                                           ┌───────────────────────────────────┐
                                           │ NEW: `nono receipt list|show|      │
                                           │ verify` (cli.rs, mirrors           │
                                           │ AuditCommands D-10) — verify does  │
                                           │ fail-closed recompute-and-compare  │
                                           └───────────────────────────────────┘
```

### Recommended Project Structure

```
crates/nono/src/
├── attestation.rs          # EXISTING: LayerAttestationStatus (unchanged), probes
├── receipt.rs               # NEW (D-12): EnforcementReceipt type + LayerId promoted here
                              #   from layer_registry.rs (policy-free half only)
├── receipt_chain.rs          # NEW: RECEIPT_CHAIN_DOMAIN, chain advance fn mirroring
                              #   telemetry/mod.rs's advance_chain — key-lifecycle TBD (Finding 2)

crates/nono-cli/src/
├── exec_strategy_windows/
│   ├── layer_registry.rs     # UNCHANGED except LayerId moves to core (re-export for callers)
│   ├── attestation.rs         # MODIFIED: decide_from_entries must also emit a full-row census
│   ├── launch.rs               # MODIFIED: apply_startup_attestation_gate writes the receipt
├── receipt_sink.rs            # NEW: sink path resolution, DENY-ACE + label application
├── cli.rs                     # MODIFIED: add ReceiptCommands (mirrors AuditCommands)
├── receipt_commands.rs         # NEW: list/show/verify implementations

crates/nono-cli/src/agent_daemon/
├── launch.rs                  # MODIFIED: daemon_attest_and_decide restructured to not
                                 #   early-return before all modeled layers are probed,
                                 #   PLUS a small daemon-local expectancy table for the 8
                                 #   unmodeled rows, cross-checked against layer_registry.rs
                                 #   by a discovery-based test (mirrors the existing
                                 #   DaemonAttestationDecision/AttestationDecision variant sync test)

crates/nono-shell-broker/src/
├── main.rs                    # MODIFIED: broker_resume_gate extended (or a sibling function)
                                 #   to build a full 13-row census from wire-contract data
                                 #   nono-cli must now also send (extending
                                 #   NONO_BROKER_REQUIRED_LAYERS or a new env var)
```

### Pattern 1: Discovery-based source-scan self-check (D-14)

**What:** A `#[test]` in `crates/nono-cli/tests/` reads its OWN target source file fresh via
`env!("CARGO_MANIFEST_DIR")` + `std::fs::read_to_string` — never `include_str!` (that would compile
the scanned text into the test binary and defeat "the test discovers, never assumes"), never the
`regex` crate (house style avoids it entirely across all three existing test files) — and asserts a
structural property by parsing plain-text patterns (word-boundary-exact `fn` name matching, PascalCase
↔ snake_case conversion, etc.).

**When to use:** Any completeness/coverage guarantee that must survive future edits without a human
remembering to update a list. D-14's type-allowlist scan is exactly this shape.

**Example (verified idiom, from `crates/nono-cli/tests/layer_registry_selfcheck.rs:35-71`):**
```rust
// Source: crates/nono-cli/tests/layer_registry_selfcheck.rs (in-tree, house pattern)
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR {} has no parent", manifest_dir().display()))
        .parent()
        .unwrap_or_else(|| panic!(/* ... */))
        .to_path_buf()
}
fn read_layer_registry() -> String {
    let path = workspace_root()
        .join("crates/nono-cli/src/exec_strategy_windows/layer_registry.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}
```
Word-boundary-exact symbol matching helper (avoids false positives from prefix-preserving renames
and doc-comment mentions — both are pinned by dedicated negative tests in the same file):
```rust
// Source: crates/nono-cli/tests/layer_registry_selfcheck.rs:311
fn content_defines_symbol(content: &str, symbol: &str) -> bool { /* ... */ }
```

**For D-14's type-allowlist scan specifically:** the receipt struct's field TYPES (not values) need
checking. There is no existing "parse a struct's field types without regex" precedent in this repo
— every existing scan matches `fn` definitions or enum variant names, not struct field type
annotations. The planner should budget this as new parsing logic (still regex-free per house style —
likely line-by-line matching on `    pub field_name: Type,` shape within the struct's brace-delimited
body, located the same way `call_sites_regions`/`module_doc_lines` locate other structural regions in
`layer_registry_selfcheck.rs`), not an assumed drop-in reuse of an existing helper.

### Pattern 2: The daemon/CLI decision-shape cross-check (reusable for D-16's daemon triage AND for keeping a daemon-local expectancy table honest)

**What:** `crates/nono-cli/src/agent_daemon/launch.rs` already contains a discovery-based test that
source-scans BOTH `agent_daemon/launch.rs` and `exec_strategy_windows/attestation.rs`, extracts each
file's decision-enum variant names via `parse_enum_variant_names`, and asserts the daemon's variant
set is a subset of the CLI's (with one documented, tested exception) — see
`agent_daemon/launch.rs:2450-2570` (`daemon_decision_enum_variants` / the surrounding cross-check
test). **This is the exact mechanism to reuse** for keeping a new daemon-local
"which LayerIds does this binary model" table honest against `layer_registry.rs`'s
`(EntryPath::Daemon, None)` expectancy cells, without giving `nono-agentd` a real dependency on
`exec_strategy_windows`.

### Anti-Patterns to Avoid

- **Trusting `attest_and_decide`'s return value as "the census."** It is an aggregate decision, not
  a census (Finding 1). Do not build the receipt by wrapping the existing `AttestationDecision` —
  build it from a new function that walks all 13 rows without early return.
- **Copying `SecurityEventLayer`'s ephemeral-key HMAC construction into the receipt chain without
  first resolving Finding 2.** This produces a receipt that looks tamper-evident but cannot actually
  be verified by the D-09 governance consumer after the session ends.
- **Adding a `#[allow(dead_code)]` to solve the double-compilation `dead_code` hazard** for any
  shared receipt-emission code reachable from only one of `nono.exe`/`nono-agentd.exe`. CLAUDE.md
  forbids it, and `attestation_downgrade_event.rs`'s module header documents that this was tried and
  empirically fails — the correct fix is placing the code where its real (singular, per-binary) call
  graph lives, exactly as that file's own history shows.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Mandatory (`NO_READ_UP`) label on the receipt sink directory | New `SetNamedSecurityInfoW` FFI wrapper | `crates/nono/src/sandbox/windows.rs::try_set_mandatory_label(path, mask)` (`:1045`) | Already exists, already handles the WRITE_OWNER footgun (see next row), already unit-tested (`plant_mandatory_label_with_flags` test helpers, `:3554`+) |
| DENY ACE for session SID / package SID on the sink | Ad hoc `SetEntriesInAclW` call scattered in new code | Extend the existing `grant_sid_write_on_path`/`grant_sid_traverse_on_path`/`grant_sid_read_on_path` family (`windows.rs:1800-1962`) with a `DENY_ACCESS`-mode sibling | Same file, same `SetEntriesInAclW` primitive, same test conventions — avoids a second, divergent ACL code path. **This is genuinely new code (no deny-ACE wrapper exists today)**, but it should be a small addition beside the existing grant functions, not a new module |
| HMAC chain domain separation | A bespoke chaining scheme | Mirror `TELEMETRY_CHAIN_DOMAIN`'s exact `Hmac<Sha256>(domain \|\| prev_head \|\| event_domain \|\| bytes)` shape (`telemetry/mod.rs:107-153`), pending Finding 2's key-lifecycle resolution | This construction is already reviewed and hardened (WR-21 mutex discipline); reinventing it risks reintroducing bugs that construction already fixed |
| Fail-closed "recompute and compare" verify command | A bespoke JSON diff | Mirror `AuditVerifyArgs`'s doc-documented pattern (`cli.rs:3503-3510`) — re-read the on-disk record, recompute the chain from genesis, compare to the stored/claimed head | Existing, reviewed, exact shape D-10 asks for |
| Type-allowlist / completeness self-check | A hand-maintained checklist in a comment | Discovery-based source scan per `layer_registry_selfcheck.rs`/`layer_registry_meta_test.rs`'s idiom | House style, D-14 explicitly requires this shape, and Phase 115's V-01 lesson (a test that names its targets is blind by construction) is a hard-won rule in this codebase |

**Key insight:** almost everything this phase needs at the Win32/crypto primitive level already
exists in-tree with test coverage. The actual net-new engineering is (a) the census-computation
logic across three binaries with three different amounts of registry visibility (Finding 1), and
(b) resolving the receipt chain's key lifecycle (Finding 2) — not new FFI or new crypto.

## Common Pitfalls

### Pitfall 1: Assuming the daemon/broker gates already "know" all 13 layers
**What goes wrong:** A plan that says "wire the receipt writer into the existing gate" for the
daemon or broker silently ships a receipt with 8-11 rows hardcoded `NotApplicable` that are
actually just "this binary never checked."
**Why it happens:** CONTEXT.md's `<code_context>` section describes `LayerId` + the 13-row registry
as "already drift-guarded," which is true of the registry *definition*, but not of what each
gate-implementation *consults*. The drift guard (`ALL`/`assert_all_layer_ids_covered`) protects the
enum-to-registry-row mapping, not the gate-to-row mapping.
**How to avoid:** Treat Finding 1 as a task, not an assumption. Write the daemon/broker census tests
FIRST, expecting them to fail against today's code, per the D-14 discipline this codebase already
practices.
**Warning signs:** A receipt test that only exercises the `nono.exe` DirectCli arm and never
constructs a Daemon- or Broker-arm receipt from a live (or force-unavailable-seeded) session.

### Pitfall 2: Building the receipt chain's key exactly like the telemetry chain's
**What goes wrong:** Ships a receipt chain that is internally self-consistent but that nobody can
verify once the emitting process exits — RCPT-02 technically "passes" (an edit does break the
chain, detectable while the process runs / in the same address space) but D-09's actual use case
(operator verifies a receipt file days later) silently fails.
**Why it happens:** D-11's wording ("identical construction and discipline") reads as "reuse the
exact code," and the exact code includes the ephemeral-key lifecycle, which was designed for a
different threat model (protecting a live audit stream against a confined child, not enabling
offline verification).
**How to avoid:** Resolve Finding 2 explicitly at planning time — pick a key-persistence strategy
(or the keyless-hash-chain alternative) and write down why, before implementation starts.
**Warning signs:** A `nono receipt verify` command that can only be run in the same process session
that emitted the receipt, or that silently accepts any key because none was ever persisted to check
against.

### Pitfall 3: `#[allow(dead_code)]` or a Cargo feature to solve cross-binary compilation
**What goes wrong:** New shared receipt-emission code compiled into both `nono.exe` and
`nono-agentd.exe` (via `#[path]` inclusion) trips `-D warnings`-fatal `dead_code` on whichever
binary doesn't call it.
**Why it happens:** `nono-cli` has no `[lib]` target (confirmed: `Cargo.toml` declares only
`[[bin]] name = "nono"` and `[[bin]] name = "nono-agentd"`), so shared `.rs` files are compiled
twice as independent translation units with different real call graphs.
**How to avoid:** Follow `attestation_downgrade_event.rs`'s documented fix exactly: place
binary-specific code in a module tree the OTHER binary's `#[path]` set never includes (e.g.
`exec_strategy_windows/` for anything `nono.exe`-only). Code genuinely shared by both belongs in
`crates/nono` (core), which both binaries link normally (not via `#[path]`).
**Warning signs:** A clippy warning that only reproduces on one of the two binary targets, or a fix
that "works" by adding an `#[allow]`/`#[expect]` attribute instead of moving code.

### Pitfall 4: Early-return refactors that silently change fail direction
**What goes wrong:** Restructuring `daemon_attest_and_decide` (or `attest_and_decide`) to "keep
going and collect a census" instead of "return on first Abort" can accidentally change WHICH error
is reported first, or worse, cause a later probe's side effect (if any probe has one — verify none
do) to run when it previously wouldn't have.
**Why it happens:** The existing functions were written and reviewed (Phase 117, multiple CR/WR
findings) specifically as early-return decision functions; converting them to census-builders is a
structural change to code that has already been hardened once.
**How to avoid:** Keep the DECISION logic (`decide_from_entries`, `daemon_attest_and_decide`)
returning exactly what it returns today for the `Result<()>`/`AttestationDecision` control-flow
consumers; add a SEPARATE, PURE census-building pass that reuses `classify_row` (already pure, no
OS side effects beyond the read-only probes) without touching the existing decision function's
control flow. `classify_row` in `attestation.rs` is already side-effect-free per-row, so this is
achievable without duplicating probe logic — call it once per row in a loop that never returns
early, in addition to (not instead of) the existing `decide_from_entries` loop, or refactor
`decide_from_entries` to build both outputs from one pass.
**Warning signs:** A test that passes for the decision but a receipt that reports `Confirmed` for a
layer whose probe was never actually re-run after the refactor (i.e., census data reused from a
stale/cached value rather than a fresh probe).

## Code Examples

### The exact D-03 write point (verified, `launch.rs:2565-2586`)
```rust
// Source: crates/nono-cli/src/exec_strategy_windows/launch.rs:2565-2586 (in-tree)
let wfp_preconfirmed = derive_wfp_preconfirmed(network_enforcement);
if let Err(err) = apply_startup_attestation_gate(
    process.raw(),
    containment.job,
    layer_registry::EntryPath::DirectCli,
    Some(arm),
    wfp_preconfirmed,
    applied_layers,
    config.session_sid.as_deref(),
    session_id,
) {
    terminate_suspended_process(
        process.raw(),
        &format!("startup self-attestation failed: {err}"),
    );
    return Err(err);
}

resume_contained_process(process.raw(), thread.raw())?;
```
The receipt write (both `ran` and `refused` outcomes, D-02) belongs inside
`apply_startup_attestation_gate` itself (`launch.rs:1552-1675`), which already branches on
`Proceed`/`Abort`/`ProceedDowngraded` and already performs its own emission work (the D-27 banner,
the `LayerAttestationDowngraded` audit event) on exactly this branch — the receipt write is a fourth
thing to do at each of those three branches, not a new call site.

### The HMAC chain construction to mirror (or explicitly deviate from — Finding 2)
```rust
// Source: crates/nono-cli/src/telemetry/mod.rs:126-153 (in-tree)
pub(crate) fn advance_chain(chain: &mut ChainState, event_bytes: &[u8]) {
    use hmac::KeyInit as _;
    let mut mac = match HmacSha256::new_from_slice(chain.key.as_ref()) {
        Ok(m) => m,
        Err(e) => { /* degrade to zeroed key, D-14 pattern */ }
    };
    mac.update(TELEMETRY_CHAIN_DOMAIN);
    mac.update(&chain.head);
    mac.update(TELEMETRY_EVENT_DOMAIN);
    mac.update(event_bytes);
    let result = mac.finalize().into_bytes();
    chain.head.copy_from_slice(&result);
    chain.sequence = chain.sequence.saturating_add(1);
}
```

### The keyless alternative (core `crates/nono/src/audit.rs:658-669`, in-tree)
```rust
pub fn hash_chain(previous: Option<&ContentHash>, leaf_hash: &ContentHash) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(CHAIN_DOMAIN_ALPHA);
    if let Some(prev) = previous {
        hasher.update(prev.as_bytes());
    } else {
        hasher.update([0u8; 32]);
    }
    hasher.update(leaf_hash.as_bytes());
    ContentHash::from_bytes(hasher.finalize().into())
}
```

### The `AuditVerifyArgs` shape D-10 mirrors verbatim (`cli.rs:3497-3590`)
```rust
// Source: crates/nono-cli/src/cli.rs:3497-3590 (in-tree)
#[derive(Subcommand, Debug)]
pub enum AuditCommands {
    List(AuditListArgs),
    Show(AuditShowArgs),
    Verify(AuditVerifyArgs),
    Cleanup(AuditCleanupArgs),
}

#[derive(Parser, Debug)]
pub struct AuditVerifyArgs {
    pub session_id: String,
    #[arg(long, value_name = "PATH")]
    pub public_key_file: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
    // ...
}
```
Note: `AuditCommands` and its subtypes carry **no `#[cfg(target_os = ...)]` gate** in `cli.rs` —
they compile on every platform (Linux/macOS included). `nono receipt` should follow the identical
pattern (cross-platform command surface, with the underlying receipt data being empty/absent on
non-Windows per D-18), which is exactly why D-23 flags `cli.rs` edits as being in the cross-target
clippy blast radius.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| CINT-02's decision (`AttestationDecision`) is the only thing produced at the D-21 gate | A parallel, non-discarded per-row census must also be produced at the same gate | This phase (118) | `decide_from_entries`/`daemon_attest_and_decide`/`broker_resume_gate` all need a second data path alongside their existing decision logic |
| Windows Event Log is the primary downgrade-detail channel (D-27/D-28, Phase 117) | Demoted to a coarse pointer; a dedicated file sink is primary (D-06) | This phase (118) | New sink code, new ACL/label guard code; Event Log code paths (`attestation_downgrade_event.rs`, `telemetry/windows.rs`) are NOT being removed, just no longer the primary receipt channel |

**Deprecated/outdated:** None — this phase adds new machinery beside Phase 117's, it does not
replace any existing Phase 117 code path.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The receipt chain should resolve Finding 2 by adopting a persistent key sourced from the existing `keyring`/`keystore.rs` machinery, OR by adopting the core audit module's keyless construction — no third option was investigated (e.g., a machine-policy-distributed shared secret) | Finding 2, Standard Stack | If a better-fitting mechanism exists (e.g., DPAPI-protected local key, cert-based), the planner should evaluate it against A1's two candidates rather than treating them as exhaustive |
| A2 | The daemon-local expectancy table (for the 8 `LayerId`s `daemon_attest_and_decide` doesn't model) should be built via the same discovery-based cross-check pattern as `daemon_decision_enum_variants`, rather than by widening the daemon's `#[path]` includes to reach `layer_registry.rs` directly | Finding 1, Pattern 2 | Widening `#[path]` includes was not evaluated for feasibility/cost — it might be simpler than a parallel table + cross-check test, but would also pull the CLI's Windows-only `layer_registry.rs` into the daemon's compilation unit, which the current architecture deliberately avoids |
| A3 | `classify_row` (`attestation.rs:405-497`) has no side effects beyond read-only OS probes, so calling it once per row in a non-short-circuiting census pass is safe to add alongside the existing early-return decision loop without behavior change | Pitfall 4 | If any live probe (`probe_restricted_sids`, `probe_in_job`, `probe_app_container_sid`) has a side effect not visible from this research pass (unlikely for read-only `Get*`/`Is*` Win32 calls, but not exhaustively verified here), doubling the probe calls could have an unexpected cost or interaction |

**If this table is empty:** N/A — see rows above.

## Open Questions

1. **Receipt chain key lifecycle (Finding 2)**
   - What we know: the telemetry chain's key is ephemeral/zeroized; the core audit chain is keyless;
     D-09 implies a persistent shared secret; D-11 implies literal reuse of the ephemeral-key
     construction. These are in tension.
   - What's unclear: whether the phase intends "same cryptographic primitive, new key-persistence
     strategy" or "accept in-process-only verification for now, defer true offline verification to
     Phase 119 alongside the deferred asymmetric-receipt item."
   - Recommendation: raise explicitly at plan-discuss time; do not let an executor default this.

2. **Daemon "not-yet-probed" sub-case (Finding 1)**
   - What we know: `daemon_attest_and_decide`'s early-return structure means a receipt built naively
     from it would have some rows silently `Unconfirmed` when they were actually just unreached.
   - What's unclear: whether D-13's four states are meant to absorb this (folding "not reached" into
     `Unconfirmed`) or whether the daemon function should be restructured to probe everything
     regardless of earlier failures.
   - Recommendation: restructure `daemon_attest_and_decide` to keep probing (Pitfall 4's guidance);
     this is more work but avoids a semantically muddy `Unconfirmed`.

3. **Broker wire-contract extension for the 11 unmodeled rows (Finding 1)**
   - What we know: the broker cannot see `layer_registry.rs`; `NONO_BROKER_REQUIRED_LAYERS` today
     only names the (at most 2) layers that must be Confirmed-or-terminate.
   - What's unclear: whether to extend that env var's shape (e.g., a second var naming which rows are
     `NotApplicable` on this arm) or hardcode the 11-row NotApplicable set directly in the broker with
     a cross-check test against the registry (mirroring `broker_expected_rows_are_abort_only`'s
     existing discovery-based pattern in `layer_registry.rs`).
   - Recommendation: prefer the hardcode + discovery-based cross-check — it is the same shape as the
     precedent that already exists (`broker_expected_rows_are_abort_only`), and avoids widening a
     cross-binary wire contract that Phase 117 documented as "a lockstep two-binary change... a mixed-
     version pair fails closed."

## Environment Availability

Skip — this phase has no external tool/service dependencies beyond the existing Rust toolchain and
Windows APIs already in use throughout the codebase. `slopcheck`/registry verification is N/A (no new
external packages — see Standard Stack).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`), `#[test]` in `crates/nono-cli/tests/*.rs` and `#[cfg(test)] mod tests` inline modules |
| Config file | none — no `pytest.ini`/`jest.config` equivalent; workspace `Cargo.toml` + per-crate `Cargo.toml` |
| Quick run command | `cargo test -p nono-cli --lib exec_strategy_windows::attestation -- --nocapture` (per-module); `cargo test -p nono --lib attestation` for core |
| Full suite command | `make test` (workspace) / `make ci` (clippy + fmt + tests, includes cross-target clippy per D-23) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RCPT-01 (census completeness) | A receipt from a real (or force-unavailable-seeded) DirectCli/Broker/Daemon session names all 13 `LayerId` rows, none silently dropped | unit + discovery-based meta-test | `cargo test -p nono-cli --test layer_registry_meta_test` (extend) + new `cargo test -p nono-cli --test receipt_census_test` | ❌ Wave 0 (new test file) |
| RCPT-01 (content-free) | No path/arg/payload bytes anywhere in a serialized receipt | source-scan (type allowlist) + sentinel round-trip, each with a perturbation proof | new `cargo test -p nono-cli --test receipt_content_free_scan` | ❌ Wave 0 (new test file, mirrors `layer_registry_selfcheck.rs` idiom) |
| RCPT-02 (tamper-evidence) | An edited receipt is detectable via `nono receipt verify` | unit (chain construction) + integration (verify command, fail-closed recompute-and-compare) | `cargo test -p nono --lib receipt_chain` + `cargo test -p nono-cli receipt_verify` | ❌ Wave 0/1 (new) |
| RCPT-03 (four-state vocabulary, no out-of-band knowledge) | `NotApplicable`/`EstablishedNotIndependentlyObservable`/`Unconfirmed`/`Confirmed` render distinguishably; an unattested layer never renders as attested | unit + discovery-based (mirrors `every_reported_layer_is_actually_consulted` idiom) | new `cargo test -p nono-cli receipt_vocabulary_rendering` | ❌ Wave 0/1 (new) |

### Sampling Rate
- **Per task commit:** the module-scoped `cargo test -p <crate> --lib <module>` for whatever was
  touched (existing house convention — Phase 117's plans used this pattern throughout).
- **Per wave merge:** `make test` (workspace-wide).
- **Phase gate:** `make ci` (clippy `-D warnings -D clippy::unwrap_used` + `cargo fmt --check` +
  full test suite) green, PLUS both cross-target clippy gates (D-23: `cross clippy --workspace
  --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` and `cargo-zigbuild
  clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`) BEFORE
  `/gsd:verify-work`, since `LayerId`'s promotion into `crates/nono` and the `nono receipt` addition
  to `cli.rs` both touch files with Unix `cfg` branches.

### Perturbation Proofs (mandatory per finding/test — this codebase's hard house rule)

For every discovery/self-enforcing test this phase adds, state the specific mutation that MUST make
it fail:

1. **D-14's type-allowlist scan.** Perturbation: add a field of type `PathBuf`, `String` populated
   from a path, or any non-allowlisted type to the receipt struct — the scan must fail the build.
   Converse perturbation (proves it isn't vacuously failing everything): a field of an allowlisted
   type (e.g. `u32`, `LayerId`, `LayerAttestationStatus`) must NOT trip it.
2. **D-14's sentinel round-trip.** Perturbation: construct a receipt from a session seeded with a
   sentinel path/argument/env value (mirrors `layer_force_unavailable.rs`'s seam for producing
   non-`Confirmed` rows without mocking the OS) and assert those exact bytes do NOT appear in the
   serialized output; the negative-control perturbation is deliberately LEAVING a path field in and
   confirming the test catches it (temporarily, in review, not committed).
3. **Census-completeness meta-test (Finding 1's fix).** Perturbation: add a 14th `LayerId` variant to
   the enum without adding a corresponding census row/branch — must fail to compile (exhaustive match,
   mirroring `assert_all_layer_ids_covered`'s existing no-wildcard-arm pattern) or fail the meta-test
   if the census builder isn't itself exhaustively matched.
4. **Chain verify's recompute-and-compare.** Perturbation: hand-edit one byte of a stored receipt
   record on disk and assert `nono receipt verify` returns a mismatch, mirroring the existing
   `AuditVerifyArgs` fail-closed pattern's own test coverage in `crates/nono/src/audit.rs`.
5. **Daemon/broker expectancy cross-check (Pattern 2 reuse).** Perturbation: add an expectancy cell
   to `layer_registry.rs` for `(EntryPath::Daemon, ...)` or `(EntryPath::Broker, ...)` without updating
   the daemon/broker's local mirror table — the discovery-based cross-check test (source-scanning both
   files, per the existing `daemon_decision_enum_variants` idiom) must fail.

### Three audit questions for every guard added in this phase

- **Placement:** does the check run at the actual D-03 write point (`apply_startup_attestation_gate`,
  before `ResumeThread`/`terminate_suspended_process`), not somewhere upstream/downstream that could
  be bypassed?
- **Predicate width:** does the check cover the FULL 13-row census on every arm, not just the rows
  each binary happens to already probe (Finding 1's core lesson)?
- **Class coverage:** does the perturbation proof exercise the general CLASS of violation (e.g. "any
  new non-allowlisted field type"), not just the one field a developer happened to think of? This is
  the exact discipline Phase 117 learned the hard way across three consecutive gap-closure rounds.

## Sources

### Primary (HIGH confidence — direct file reads, symbol-verified, current tree)
- `crates/nono-cli/src/exec_strategy_windows/attestation.rs` (full read, lines 1-1219 of 1777) —
  `attest_and_decide`, `AttestationDecision`, `AttestationInput`, `classify_row`,
  `decide_from_entries`, `BROKER_REQUIRED_LAYERS_ENV_VAR`, `required_layers_for_broker`
- `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` (full read, lines 1-1036 of 1633 +
  offset 1037-1260) — `LayerId` (13 variants), `EntryPath`, `ArmExpectancy`, `ProbeKind`,
  `LayerApplication`, `AppliedLayers`, `ContractOutcome`, `ALL`, `all_entries`, the 13-row
  `REGISTRY_ENTRIES` table, `every_reported_layer_is_actually_consulted` test
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` (offsets 1500-1720, 2500-2620) —
  `apply_startup_attestation_gate` (exact D-03 write point, line 1552), `spawn_windows_child`'s
  call site (line 2569) and its position relative to `resume_contained_process` (line 2586),
  `derive_wfp_preconfirmed`
- `crates/nono-cli/src/agent_daemon/launch.rs` (offset 1275-1500) — `DaemonAttestationDecision`
  (2-state), `daemon_attest_and_decide` (5-layer early-return chain, lines 1417-1500)
- `crates/nono-shell-broker/src/main.rs` (offset 330-510) — `broker_resume_gate` (lines 369-453),
  `BROKER_ATTESTABLE_LAYERS` (line 321, 2-element hardcoded list)
- `crates/nono-shell-broker/Cargo.toml` (full read) — confirms `nono` core dependency
  (`package = "nono-sandbox"`), no `nono-cli` dependency
- `crates/nono-cli/src/telemetry/mod.rs` (offset 60-220) — `TELEMETRY_CHAIN_DOMAIN`,
  `ChainState` (ephemeral key + `Drop`/zeroize, lines 87-103), `advance_chain` (lines 126-153),
  `SecurityEventLayer::new` (ephemeral key generation, line 319)
- `crates/nono/src/audit.rs` (offset 635-670 + grep) — `CHAIN_DOMAIN_ALPHA`, `hash_chain` (keyless
  SHA-256, lines 658-669), confirms core chain is NOT HMAC-based
- `crates/nono/src/attestation.rs` (offset 1-110 + probe fn grep) — `LayerAttestationStatus` (4
  variants, lines 74-113), `ProcessHandle`/`JobHandle` platform-neutral aliases, `probe_integrity_level`
  / `probe_in_job` / `probe_app_container_sid` / `probe_restricted_sids` signatures
- `crates/nono-cli/src/cli.rs` (offset 3490-3590 + cfg grep) — `AuditCommands` (line 3498),
  `AuditListArgs`/`AuditShowArgs`/`AuditVerifyArgs` (lines 3521-3590+), confirms no
  `#[cfg(target_os)]` gate around `AuditCommands` (cross-platform command surface)
- `crates/nono/src/machine_policy.rs` (offset 1-60 + grep) — `RequiredLayersPolicy`
  degrade-not-abort pattern (lines 96, 683-757), the D-04 precedent to mirror
- `crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs` (offset 1-60) — the
  `nono-cli` has-no-`[lib]`-target constraint, the `dead_code` double-compilation hazard, and its
  documented fix (move code to the module tree the other binary never `#[path]`-includes)
- `crates/nono/src/sandbox/windows.rs` (grep for label/DACL functions) —
  `try_set_mandatory_label` (line 1045, uses `SetNamedSecurityInfoW` +
  `LABEL_SECURITY_INFORMATION`), `path_is_owned_by_current_user` (WRITE_OWNER proxy check, line
  1226), `grant_sid_write_on_path`/`grant_sid_traverse_on_path`/`grant_sid_read_on_path`/
  `grant_sid_read_attributes_on_path` (lines 1800-1962, all GRANT/`SET_ACCESS` — confirms NO
  existing DENY-ACE wrapper)
- `crates/nono-cli/tests/layer_registry_selfcheck.rs`,
  `crates/nono-cli/tests/layer_registry_meta_test.rs`,
  `crates/nono-cli/tests/layer_force_unavailable.rs` (function-signature greps) — confirmed the
  `env!("CARGO_MANIFEST_DIR")` + `fs::read_to_string`, no-regex, no-`include_str!` house scan
  idiom; `layer_force_unavailable.rs`'s `#![cfg(all(target_os = "windows", feature =
  "layer-fault-injection"))]` gate (D-30's compiled-out-of-release requirement)
- `crates/nono-cli/data/hooks/nono-tool-hook.ps1` (offset 1-40) — confirms the per-tool-call hook
  path is a two-process-spawn shape (PowerShell → `nono claude-code-hook` decision, then a separate
  `nono run`), relevant to D-17's latency measurement scope
- `crates/nono-cli/Cargo.toml`, workspace `Cargo.toml` (grep) — confirmed no `[lib]` target in
  nono-cli; `hmac = "0.13"`, `sha2 = "0.11"` versions
- `proj/ADR-86-library-boundary-convergence.md` (offset 1-60) — the precedent pattern D-12's
  core-promotion argument should cite (Cluster A: audit logic relocated core-ward; Cluster B:
  diagnostic UX stayed CLI-side)
- `.planning/phases/118-per-session-enforcement-receipts/118-CONTEXT.md` (full read) — all 24
  locked decisions, discretion areas, deferred ideas
- `.planning/REQUIREMENTS.md` (offset 100-160) — RCPT-01/02/03 exact wording, v3.7 architecture
  invariants
- `.planning/ROADMAP.md` (offset 355-395) — Phase 118 goal, SC1-SC4, dependency on Phase 117/115

### Secondary (MEDIUM confidence)
- None — this phase's research was entirely groundable against the live tree; no web research was
  needed given the in-scope Win32 primitives (`SetNamedSecurityInfoW`, `WRITE_RESTRICTED` semantics)
  are already documented and tested in-tree with the exact behavior CONTEXT.md's D-08 describes.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every dependency verified against actual `Cargo.toml` files in-tree; no new
  packages needed.
- Architecture / data availability (Finding 1): HIGH — verified by reading the actual gate-decision
  functions in all three binaries, not inferred from CONTEXT.md's summary.
- Chain/crypto construction (Finding 2): HIGH on what exists; MEDIUM on what the phase should do
  about it, since this is a genuine open design question this research surfaced rather than resolved.
- Pitfalls: HIGH — grounded in this codebase's own documented history (`attestation_downgrade_event.rs`'s
  module header explicitly records the `dead_code`/`#[allow]`/feature-flag failures that were tried
  and rejected).
- Sink ACL guidance (D-08): HIGH for the mandatory-label half (existing, tested code); MEDIUM for the
  deny-ACE half (no existing wrapper, but a clear, low-risk extension pattern).

**Research date:** 2026-08-16
**Valid until:** This phase's own implementation should invalidate large portions of this research
(the census-completeness gap and the chain-key question are exactly what this phase exists to close)
— treat as valid only through the planning and Wave-0 stage of Phase 118 itself, not as a general
reference afterward.
