---
phase: 118-per-session-enforcement-receipts
verified: 2026-09-06T14:39:54Z
status: human_needed
score: 4/4 success criteria verified (1 with a named guard-coverage gap)
overrides_applied: 0
re_verification:
  previous_status: none
  previous_score: n/a
  gaps_closed: []
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Run TWO broker-arm sessions that register an AppContainer (`nono run -p claude-code --allow-cwd -- cmd /c echo hi` from a %USERPROFILE% subdir; confirm `app_container=true` in broker output). Before and after, run `icacls C:\\ProgramData\\nono\\receipts` and count `S-1-15-2-*:(DENY)` ACEs."
    expected: "The S-1-15-2-* DENY ACE count is IDENTICAL before and after. Code reading says it will GROW by one per AppContainer broker session and never shrink — nothing in crates/nono-shell-broker ever calls revoke_sid_on_path."
    why_human: "Requires a real host, a registrable AppContainer profile, and an out-of-band icacls read of a machine-wide directory. The 16 → 16 measurement already on record is consistent with a BrokerLaunch/app_container=false session, which adds no package-SID ACE at all, so it does not cover this arm."
  - test: "Confirm the operator accepts that RCPT-02's requirement TEXT in REQUIREMENTS.md line 126 (and ROADMAP SC-3) still says 'HMAC-chained SecurityEventLayer' while D-25 shipped a KEYLESS SHA-256 chain — then amend the requirement text or record the divergence deliberately."
    expected: "A decision: either REQUIREMENTS.md/ROADMAP are amended to name the keyless chain, or the divergence is explicitly accepted in writing."
    why_human: "The verifier is forbidden from editing REQUIREMENTS.md or ROADMAP.md (D-24). This is a requirement/implementation divergence, not a code defect — only the operator can decide which side is wrong."
---

# Phase 118: Per-Session Enforcement Receipts — Verification Report

**Phase Goal:** The conjunction "restricted token AND low-integrity label AND AppContainer profile AND WFP coverage" stops being asserted at launch and becomes attested per session — turning "we configured enforcement" into "we can show enforcement held."

**Verified:** 2026-09-06T14:39:54Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Method note

Every claim below was checked against source, not against SUMMARY.md. Two things in particular
were re-derived from scratch rather than accepted:

1. **Terminal-exit enumeration.** I did not accept "all branches instrumented." I read each
   producer's spawn function, located the `CreateProcess*` call, and enumerated every `return Err`
   / `?` downstream of it myself. Results in the RCPT-01 table.
2. **D-23 cross-target clippy.** The phase's own record (Task 1, commit `f19752e8`) predates HEAD
   by seven code-changing commits, five of which touch files carrying `cfg(not(target_os =
   "windows"))` blocks that a Windows-host clippy is structurally blind to. I re-ran **both** gates
   at HEAD. See Behavioral Spot-Checks.

---

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Every confined session emits a receipt naming which layers were confirmed active | ✓ VERIFIED | All post-spawn terminal exits in all three producers enumerated and instrumented — see the RCPT-01 table below. One latent, currently-unreachable exit named (WR-07). |
| 2 | Receipt is content-free — no paths, args, or payload — verified by a self-enforcing source scan | ✓ VERIFIED **with a named guard-coverage gap** | The shipped `EnforcementReceipt` shape is content-free by construction; the scan is real and perturbation-proved. But the scan covers only `EnforcementReceipt`, **not** `LayerReceiptRow`, which is serialized inside every receipt (WR-06). |
| 3 | Receipt integrity is verifiable so an edited receipt is detectable | ✓ VERIFIED **with a requirement-text divergence** | Keyless SHA-256 chain, own domain, fail-closed recompute-and-compare from genesis, 8 tamper shapes perturbation-proved, CR-02 chain-restart defect fixed and pinned. Requirement text still says "HMAC-chained" — see the RCPT-02 section. |
| 4 | A reader distinguishes confirmed / not-expected / expected-but-unconfirmed without out-of-band knowledge; unattested never renders as attested | ✓ VERIFIED | Exhaustive no-wildcard match, lexically distinct labels, discovery scan + non-vacuity guard + behavioural distinctness test, all passing. WR-02 fixed at `06393e57`. |

**Score:** 4/4 truths verified.

---

## RCPT-01 — Terminal-exit enumeration (independently derived)

The phase shipped four separate instances of the unenumerated-terminal-exit class (118-06's
3-of-5 refuse branches, CR-01's three silent post-spawn exits, CR-06's misdirected broker sink,
CR-03's empty session id). I enumerated the exits myself in all three producers.

### `nono.exe` — `spawn_windows_child` (`exec_strategy_windows/launch.rs:2212-3216`)

Boundary: a "confined session" begins when `CreateProcess{AsUser}W` returns a real suspended
child. Every `return Err` downstream of the last `CreateProcess*` in the cascade:

| Line | Exit | Emits? | Evidence |
|------|------|--------|----------|
| 3133 | `apply_process_handle_to_containment` fails | ✓ `Refused` | `emit_pre_gate_refusal_receipt` at 3122, before `terminate_suspended_process` |
| 3155 | `apply_resource_limits` fails | ✓ `Refused` | `emit_pre_gate_refusal_receipt` at 3144 |
| 3175 | attestation gate returns `Err` | ✓ `Refused` | gate's own `Abort` arm emits at 1800 |
| 3201 | `resume_contained_process` fails after gate wrote `Ran` | ✓ corrective `Refused` | `emit_pre_gate_refusal_receipt` at 3191; the `?` was converted to `if let Err` as CR-01 required |
| — | Every earlier `return Err` (2679, 2704, 3086, etc.) | n/a — justified | All precede a successful `CreateProcess*`; no confined child exists |

`wfp_preconfirmed` is now hoisted to 3113 so the two pre-gate exits can build a real census
(previously computed too late for them to use). `census_for_pre_gate_refusal` re-probes at each
site rather than cloning a stale census — correct, since each exit observes the child at a
different point in its containment lifecycle.

**Residual (WR-07, open):** `attest_and_decide(input)?` at `launch.rs:1773` is a fifth
receipt-less exit. Unreachable today — `required_layers_override` and `machine_required_layers`
are both hardcoded `&[]` (the `RF-13 OPEN` block at 1742-1753). It becomes live the moment RF-13
threads machine policy into that call site. The census is already in scope one line above.

### `nono-agentd.exe` — `launch_agent` (`agent_daemon/launch.rs:634-1240`)

Every terminal exit between step 5 (`spawn_appcontainer_process_suspended`) and step 8
(`ResumeThread`) emits:

| Step | Exit | Emits? |
|------|------|--------|
| 6 | `AssignProcessToJobObject` fails (762) | ✓ `write_daemon_refuse_receipt_best_effort` at 748 |
| 6.5 | WFP gate — two branches (831, 854) | ✓ at 817 and 839 |
| 6.6 | Package-SID DACL grants fail (904) | ✓ at 890 |
| 7a | AgentRegistry insert fails (957) | ✓ at 940 |
| 7b | tenants-lock poisoned (1014) | ✓ at 1002 (inside a `map_err` closure — IN-02, cosmetic) |
| 6.7 | gate `Proceed` / `Abort` | ✓ `Ran` at 1109 (fail-closed: terminate + `return Err` on emission failure under `require_receipts`) / `Refused` at 1150 |
| 8 | `ResumeThread` fails | ✓ corrective `Refused` at 1209 |
| — | `create_agent_job` / spawn itself failing | n/a — no confined child yet |

### `nono-shell-broker.exe` — `run` (`main.rs:1037-1770`)

Every terminal exit downstream of the `CreateProcess{W,AsUserW}` at 1325/1350:

| Line | Exit | Emits? |
|------|------|--------|
| 1477 | `OpenProcessToken` on AppContainer child fails | ✓ `Refused` at 1464 |
| 1516 | `apply_low_il_label_to_token` fails | ✓ `Refused` at 1504 |
| 1612 | `NONO_BROKER_REQUIRED_LAYERS` absent/unreadable (fail-closed) | ✓ `Refused` at 1599 |
| 1672 | resume gate refuses | ✓ `Refused` at 1659 |
| 1722 | `ResumeThread` fails after `Ran` written at 1683 | ✓ corrective `Refused` at 1710 |
| 1743, 1758 | `WaitForSingleObject` / `GetExitCodeProcess` fail | n/a — justified: the child has already **run**, so the `Ran` receipt at 1683 is truthful |
| 1391 | `CreateProcess*` itself fails | n/a — no child |

The wire-contract reads and `session_id`/`pid` are deliberately hoisted to 1408-1413, *before* the
two label-application fail-closed branches, so the earliest refuse paths can build a census.

**Verdict: RCPT-01 SATISFIED.** All 16 post-spawn terminal exits across three producers emit or are
justified. One latent exit (WR-07) named and gated behind unwired RF-13.

---

## RCPT-02 — Tamper-evidence, and a stale requirement

**Mechanism as shipped:** keyless, domain-separated SHA-256 (`crates/nono/src/receipt_chain.rs`),
mirroring `crates/nono/src/audit.rs::hash_chain`, with `RECEIPT_EVENT_DOMAIN` /
`RECEIPT_CHAIN_DOMAIN` distinct from both the audit and telemetry domains (pinned by tests).
`verify_records` (`receipt_commands.rs:676`) walks from genesis checking non-emptiness, sequence
continuity, `prev_head` parentage, recomputed leaf hash, and recomputed chain head.

**CR-02 fix verified in code, not on assertion.** Both writers now resume:
`ReceiptWriter::new` (`receipt_sink.rs:674-717`) and `BrokerReceiptWriter::new`
(`nono-shell-broker/src/main.rs:832-892`) read the segment's last non-blank line, resume from
`chain_head` / `sequence + 1`, and **fail closed** on an unparseable tail. Pinned by
`second_writer_over_existing_segment_resumes_the_chain` and
`new_writer_rejects_a_segment_with_an_unparseable_tail_record`, both passing. The broker needed a
new owned `BrokerReceiptRecordTail` type because `BrokerReceiptRecord` borrows its receipt and
cannot derive `Deserialize` — a real fix, not a doc change.

**D-25 discipline holds.** No `key` field exists on any chain struct in either writer. Every
occurrence of "key holder" / "HMAC" in the receipt modules is a *disclaimer*, never a claim
(`receipt_chain.rs:11-18`, `receipt_sink.rs:145`, `receipt_commands.rs:47-56`). Both the human and
`--json` verify outputs are worded to the narrower claim: *"tamper-evident: an edit is detectable
(keyless hash chain, D-25) — this does not prove who wrote it."*

### FINDING — the requirement TEXT is stale relative to the shipped mechanism

REQUIREMENTS.md line 126 and ROADMAP SC-3 both say the receipt must be tamper-evident *"on the
same terms as the existing audit chain (**HMAC-chained `SecurityEventLayer`**)"*. Measured:

- `crates/nono/src/audit.rs` — "the existing audit chain" — is **keyless SHA-256**
  (`hash_chain` at 659, no `Hmac` import in the file).
- `SecurityEventLayer` (`crates/nono-cli/src/telemetry/mod.rs`) is a **different** chain, `Hmac<Sha256>`
  with an ephemeral `OsRng` key zeroized on `Drop` (`ChainState.key` at 89, `zeroize()` at 100).

So the two halves of the requirement sentence name two different mechanisms, and the parenthetical
was already inaccurate about the pre-existing codebase. **D-25's choice is the correct one** — an
HMAC chain keyed by a per-process, never-persisted key produces receipts nobody can verify after
the emitting process exits, defeating RCPT-02's whole purpose. The shipped implementation satisfies
the *main clause* ("the same terms as the existing audit chain") exactly.

REQUIREMENTS.md and ROADMAP have never been amended. Recorded as a divergence for operator
decision (see `human_verification`); the verifier may not edit those files (D-24).

### Known, accepted limitation — but only recorded in a planning doc, not in code

A clean deletion of a segment's **tail** record leaves an internally self-consistent shorter chain
that `verify_records` accepts. This is inherent to a keyless chain with no external commitment
(contrast `nono audit verify`, which compares against `SessionMetadata.audit_integrity`).

Two problems with how it is recorded:

1. `118-09-SUMMARY.md` (line 95) asserts *"D-25's own module doc already states the keyless chain's
   limit precisely"* and quotes the sentence **"anyone who can write the sink can rebuild a
   self-consistent chain — the sink's ACL is what bounds that, not the hash."** A repo-wide grep for
   `self-consistent chain` across `crates/` returns **zero hits**. That sentence does not exist.
   The nearest real text (`receipt_commands.rs:53-55`) states the *authorship* limit only.
2. Consequently, `nono receipt verify`'s own output tells a reader "an edit is detectable" and
   discloses the authorship limit — but never discloses the truncation limit. A tail deletion is an
   edit that is *not* detectable.

Not scored as a phase failure (the limitation itself is accepted), but the code should carry the
sentence its own SUMMARY claims it already carries.

**Verdict: RCPT-02 SATISFIED on substance.** Requirement text stale; one accepted residual
under-documented in shipped code.

---

## RCPT-03 — Three-state distinguishability

`layer_status_label` (`receipt_commands.rs:543-550`):

| `LayerAttestationStatus` | Label | RCPT-03 case |
|---|---|---|
| `Confirmed` | `confirmed` | layer confirmed active |
| `EstablishedNotIndependentlyObservable` | `applied (unobservable)` | (fourth state, D-13 floor) |
| `Unconfirmed` | `absent-or-failed` | expected but unconfirmed |
| `NotApplicable` | `not-applicable` | not expected in this configuration |

- **No wildcard arm** — a fifth variant fails the build rather than inheriting a label. Enforced by
  `layer_attestation_status_rendering_is_exhaustive_with_no_wildcard_arm`, a brace-balanced source
  scan of that function's actual body.
- **Non-vacuity guard present** — `layer_status_label_marker_is_present_exactly_once_as_a_real_definition`
  asserts the scanned marker exists exactly once as an unquoted `fn` definition line. This is the
  Phase 115 V-01 lesson correctly applied; the scan cannot silently pass on zero matches.
- **Lexical distinctness proven per value** — `confirmed_label_is_lexically_distinct_from_the_other_three_states`
  renders all four through the *actual* function `cmd_show` calls and asserts `"confirmed"` is not a
  substring of any other label, plus pairwise distinctness. (A naive `"unconfirmed"` label would have
  failed this — the vocabulary was chosen against the test, not the other way round.)
- Rendering is applied in both the human path (`print_receipt_human:494`) and the JSON path
  (`receipt_summary_json:510`), so `--json` consumers get the same label.

**WR-02 fix verified** (`06393e57`, `broker_census` at `main.rs:620-631`): `required` is now checked
**before** `not_applicable`, so a name on both unvalidated env channels yields its *probed* status
instead of `NotApplicable`. An overlap also emits a `tracing::warn!` rather than being silently
normalised. This closes the exact hole where a must-be-`Confirmed`-or-terminate layer could read as
"not expected in this configuration."

**Census construction checked in all three producers:**
- `nono.exe` / `nono-agentd.exe` — registry-driven `census_from_entries`, pinned by
  `census_from_entries_has_no_early_return_and_is_registry_driven` and `coverage_split_accounts_for_every_layer_id`.
- Broker — env-channel-driven `broker_census`, iterating `LayerId::ALL` (13 rows, never a subset);
  the producer-side partition (`broker_role`, `attestation.rs:827`) is arm-aware and proven complete
  with zero overlap or gap.
- Arm correctness spot-checked: `RESTRICTED_TOKEN_EXPECTANCY` is scoped to `(DirectCli,
  WriteRestricted)` only, and `APP_CONTAINER_PROFILE_EXPECTANCY` to `(Broker, BrokerLaunchNoPty)` and
  `(Daemon, None)`. So `nono.exe`'s DirectCli receipt on a broker arm correctly reports those
  `not-applicable` for the Medium-IL broker it actually spawned, and the *grandchild*'s own layers are
  attested by the broker's paired `.broker.jsonl` segment. The D-15 pair is what makes the broker arm
  fully attested — and it now works (CR-06, live-verified on both arms).

**Verdict: RCPT-03 SATISFIED.**

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/nono/src/receipt.rs` | Policy-free receipt vocabulary, 13-row `LayerId` | ✓ VERIFIED | 411 lines; 13 variants; `ALL` const; 5 tests pass. Content-free field set confirmed by reading, not by trusting the scan. |
| `crates/nono/src/receipt_chain.rs` | Keyless domain-separated chain primitive | ✓ VERIFIED | 170 lines; no `key`, no `Hmac`; 5 tests pass incl. domain distinctness. |
| `crates/nono-cli/src/receipt_sink.rs` | Guarded sink + chain writer | ✓ VERIFIED | 1253 lines; CR-02/CR-04/CR-05 fixes present and tested; `SinkSidGuard` RAII with own-only + no-op-safe tests. |
| `crates/nono-cli/src/receipt_commands.rs` | `list`/`show`/`verify` + four-state rendering | ✓ VERIFIED | 1315 lines; wired into `cli.rs` + `app_runtime.rs`; `run_receipt_dispatches_list_to_the_real_command` exercises real dispatch. |
| `crates/nono-shell-broker/src/main.rs` | Own census + sink writer + argv sink base | ✓ VERIFIED | `broker_census`, `BrokerReceiptWriter` (CR-02-resuming), `broker_receipt_sink_dir` fail-closed on argv (CR-06). |
| `crates/nono/src/machine_policy.rs` | `require_receipts` fleet control | ✓ VERIFIED | 4 tests pass incl. malformed-value-aborts. |
| `crates/nono-cli/tests/receipt_content_free_scan.rs` | Self-enforcing content-free gate | ⚠️ PARTIAL | Real and perturbation-proved, but scans only `EnforcementReceipt` (WR-06). |

## Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `spawn_windows_child` | receipt sink | `apply_startup_attestation_gate` + `emit_pre_gate_refusal_receipt` | ✓ WIRED | 4/4 post-spawn exits |
| `launch_agent` | receipt sink | `daemon_emit_enforcement_receipt_at_resolved_sink` + `write_daemon_refuse_receipt_best_effort` | ✓ WIRED | 9 call sites |
| broker `run` | receipt sink | `record_broker_receipt` | ✓ WIRED | 6 call sites |
| `nono.exe` | broker sink base | argv `--receipt-sink-base` | ✓ WIRED | `push_broker_receipt_sink_base_arg` at 2603 **and** 2954 — both arms. Pinned by `broker_receipt_sink_base_travels_on_argv_not_through_the_redirected_child_env`. |
| `nono.exe` | broker session id | env `NONO_SESSION_ID` | ✓ WIRED | `broker_env_pairs.push` at 2348 (CR-03) |
| `receipt_sink` write side | `receipt_commands` read side | shared `session_file_path` | ✓ WIRED | One naming convention, one place |
| `SinkSidGuard` | session lifetime | `WindowsSupervisedChild` / `AgentTenant` | ✓ WIRED | `supervisor.rs:92`, `reap.rs:109` |
| **broker sink DENY ACE** | **revoke** | — | ✗ **NOT WIRED** | See CR-05 residual below |

## Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data? | Status |
|---|---|---|---|---|
| `EnforcementReceipt.layers` | 13-row census | live OS probes via `census_from_entries` / `broker_census` | ✓ | FLOWING — probes are real `GetTokenInformation`/`IsProcessInJob` calls on the supervisor's own handle (D-19), never child-supplied |
| `EnforcementReceipt.pid` | child pid | `GetProcessId(process)` on supervisor handle | ✓ | FLOWING |
| `EnforcementReceipt.entry_path` / `token_arm` | control flow | supervisor's own branch, hardcoded per site | ✓ | FLOWING (D-19-correct: not child-derived) |
| `nono receipt show` output | on-disk records | `read_segment` → real JSONL | ✓ | FLOWING — live-verified: both segments chain-intact on both broker arms |
| `resolve_sink_dir()` | sink path | validated `%PROGRAMDATA%`, fail-closed | ✓ | FLOWING — no silent fallback to an untrusted value |

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Core receipt vocabulary + chain + policy | `cargo test -p nono-sandbox --lib receipt` | 16 passed, 0 failed | ✓ PASS |
| CLI receipt surface (sink, commands, gate, attestation) | `cargo test -p nono-sandbox-cli --bin nono receipt` | 39 passed, 0 failed | ✓ PASS |
| Broker crate | `cargo test -p nono-shell-broker` | 51 + 2 passed, 0 failed | ✓ PASS |
| Content-free scan | `cargo test -p nono-sandbox-cli --test receipt_content_free_scan` | 4 passed | ✓ PASS |
| Registry/census meta-test | `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` | 12 passed | ✓ PASS |
| Sentinel round-trip | `cargo test -p nono-sandbox-cli --test receipt_sentinel_roundtrip` | 1 passed | ✓ PASS |
| **D-23 apple-darwin gate at HEAD** | `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (SDKROOT unset) | Finished in 36.00s, clean | ✓ PASS |
| **D-23 linux-gnu gate at HEAD** | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | Finished in 8m 13s, clean | ✓ PASS |

All selectors use the real crate names and every run reports a non-zero test count — the
`-p nono` / `-p nono-cli` silent-zero trap was avoided.

**D-23 note:** the phase's own cross-target record was taken at `f19752e8`, seven code-changing
commits before HEAD. Five of the files changed since then carry `cfg(not(target_os = "windows"))`
or Unix-cfg blocks (`agent_daemon/accept_loop.rs`, `control_loop.rs`, `launch.rs`,
`receipt_commands.rs`, `nono/src/lib.rs`) — invisible to a Windows-host clippy. Both gates re-run
at HEAD and green. **D-23 is closed at HEAD, not merely at Task 1.**

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `exec_strategy_windows/attestation.rs` | 2152 | `TBD` | ℹ️ Info | Inside a doc comment describing the SPEC's *former* `TBD` that D-17's measurement replaced — a historical reference, not a live debt marker. Not gating. |
| `receipt_commands.rs` | 367 | `last.sequence + 1` — unchecked `u64` add on untrusted on-disk data | ⚠️ Warning | WR-03. Debug panic / release wrap-to-`0 record(s)`. Direct CLAUDE.md violation ("use `checked_`/`saturating_`/`overflowing_` for security-critical math") in phase-new code. Also inconsistent with `list_entry_json:408`, which uses `records.len()` for the same concept. |
| `receipt_sink.rs` | 260-275 | Two const doc comments still assert read protection | ⚠️ Warning | WR-04 — see the D-08 consistency section. |
| `nono-shell-broker/src/main.rs` | 840-848 | Unsafe `session_id` degrades to `"unknown-session"` | ℹ️ Info | IN-01. Diverges from `ReceiptWriter::new`, which fails closed on the same input class. Much less likely to fire now CR-03 is fixed, but two producers sharing one sink still disagree on whether the value class is fatal. |

No `FIXME`/`XXX` markers, and no live `TODO`/`TBD` in any phase-modified `.rs` file.

---

## FINDING — D-08's narrowed claim is NOT consistently reflected (WR-04 stands)

The task asks specifically whether the write-integrity-only narrowing is reflected everywhere the
guard is described, and whether anything still asserts read protection. **Two live assertions
remain**, both in `crates/nono-cli/src/receipt_sink.rs`:

```rust
// line 260-267
/// Deny-ACE access mask applied to the sink directory for each named SID
/// (D-08's DACL half). Comprehensive — read (reconnaissance, T-118-14),
/// write/delete (tampering, T-118-15), and execute ...
const RECEIPT_SINK_DENY_MASK: u32 = ...;

// line 269-275
/// Mandatory-label mask applied to the sink directory unconditionally
/// (D-08's label half): `NO_READ_UP` (the guard's actual job — see module
/// doc) ...
const RECEIPT_SINK_LABEL_MASK: u32 = ...;
```

Task 3 measured that the label does nothing for the receipt files (pinned to a LOW RID, so a Low-IL
subject is *equal* not below; and no `(OI)(CI)`, so it never attaches to files) and that the read
half of the DENY mask does not hold. `NO_READ_UP` is not "the guard's actual job" — it has no job
here at all.

**Correctly amended:** the module doc's "D-08 SCOPE CORRECTION" section (56-96), the emit-site
comment (`exec_strategy_windows/launch.rs:1953-1984`), the daemon mirror, and
`receipt_commands.rs:53-55`'s "the sink's ACL, not the hash, is what bounds who could have written
it" (which remains true under the narrowed claim).

**Not amended:** the two constants above, and — per WR-04 — `deny_sid_on_path`'s bullet list in
`crates/nono/src/sandbox/windows.rs`. A reader hovering the constant or reading the function's
rustdoc sees only the falsified claim. This is the same "overstated coverage claim" class Task 3
itself found, left live in the same file the correction lives in.

Recorded as a WARNING, not a blocker: no requirement (RCPT-01/02/03) rests on read confidentiality.

---

## FINDING — CR-05's fix has a class-coverage hole on the third producer

**This contradicts the stated verified state that "Six Criticals … are fixed and verified."**
CR-05 is fixed on **two of three** producers.

- `nono.exe` — `SinkSidGuard` constructed at `exec_strategy_windows/launch.rs:2262`, moved into
  `WindowsSupervisedChild` (`supervisor.rs:92`). ✓
- `nono-agentd.exe` — constructed at `agent_daemon/launch.rs:708`, held by `AgentTenant`
  (`reap.rs:109`). ✓
- `nono-shell-broker.exe` — `record_broker_receipt:1006` calls
  `ensure_broker_receipt_sink_guarded(&sink_dir, package_sid_for_guard)`, which at `main.rs:757`
  does `deny_sid_on_path(dir, sid, DENY_MASK)` for the **per-run AppContainer package SID** on the
  **shared, permanent machine-wide sink**. **The string `revoke` does not appear anywhere in
  `crates/nono-shell-broker/`.** ✗

On the broker arm where an AppContainer profile is registered (`app_container=true`, observed live
during Task 3), `expected_sid` is `Some(...)`, so this is reachable, not theoretical. Every such
session adds one permanent `S-1-15-2-*` DENY ACE (~60 bytes) to a DACL capped at 65,535 bytes,
with no cleanup path — exactly the failure mode CR-05 described.

**Why the 16 → 16 measurement does not cover this.** That figure is consistent with a
`BrokerLaunch` / `app_container=false` session (the shape the ROADMAP's carried-defect note
records for `nono shell` on this host): `app_container_sid` is `None`, so `expected_sid` is `None`,
so the broker adds no package-SID ACE at all — while `nono.exe`'s own session-SID ACE is added and
correctly revoked by `SinkSidGuard`, netting zero. It is a valid measurement of the CLI half.

**Also note:** CR-05's suggested class assertion — *"add a test that calls `ensure_sink_guarded` N
times with N distinct SIDs against one directory and asserts the ACE count does not grow
linearly"* — was **not** added. The three shipped tests
(`sink_sid_guard_revokes_its_own_deny_ace_on_drop`,
`sink_sid_guard_drop_never_touches_a_different_sids_deny_ace`,
`sink_sid_guard_drop_is_a_safe_no_op_when_nothing_was_ever_applied`) each verify one guard's
behaviour. None asserts the *accumulation* property across sessions, and none touches the broker.
This is the "guard covers the instance someone noticed, not the class" pattern the project's own
durable lessons flag.

Not a blocker: no correctness defect today (a stale session SID is never reused, so the extra ACEs
are inert), and no requirement fails. Routed to human verification for measurement.

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| RCPT-01 | 118-01/03/04/05/06/07/08 | Every confined session emits a content-free receipt naming confirmed layers | ✓ SATISFIED | 16 post-spawn terminal exits enumerated across 3 producers, all instrumented or justified; census is 13 rows on every arm; field set content-free by construction |
| RCPT-02 | 118-01/05/06/09 | Tamper-evident on the audit chain's terms | ✓ SATISFIED (text divergence) | Keyless SHA-256 mirroring `audit.rs::hash_chain`; 8 tamper shapes perturbation-proved; CR-02 fixed. Requirement TEXT still says "HMAC-chained" — routed to operator |
| RCPT-03 | 118-01/03/06/09 | Three-state distinguishability; unattested never reads as attested | ✓ SATISFIED | Exhaustive no-wildcard rendering + non-vacuity guard + per-value distinctness test; WR-02 closed the one permissive-resolution hole |

No orphaned requirements: REQUIREMENTS.md maps exactly RCPT-01/02/03 to Phase 118, and all three
are claimed by plans in this phase.

---

## Open Items — recorded, not scored as phase failures

Per the phase's own records these are known and deliberately open. Listed so nothing lapses.

1. **WR-06** (highest priority of the open warnings — it is the only one that touches a Success
   Criterion's own mechanism): the D-14 content-free scan enumerates `EnforcementReceipt`'s fields
   only. `LayerReceiptRow` is a `pub struct` with `Serialize`, embedded via
   `layers: Vec<LayerReceiptRow>` in every receipt, and its fields are never scanned. A future
   `pub detail: String` there would serialize into every receipt on disk and the gate would stay
   green. Secondary: `field_types_in_struct` uses `source.find` (first occurrence) with no
   single-definition guard — its sibling scan in `receipt_commands.rs` has exactly such a guard.
2. **WR-01**: `validate_session_id_for_filename` (`receipt_sink.rs:607`) has no length cap and does
   not reject Windows reserved device names, while its sibling `session::validate_session_id`
   applies the same character class *plus* a 64-byte cap.
3. **WR-03**: unchecked `sequence + 1` (see Anti-Patterns).
4. **WR-04**: two const docs still assert read protection (see the D-08 finding).
5. **WR-05**: the "MIRROR ASYMMETRY" comment's claim holds only on the `WriteRestricted` arm.
6. **WR-07**: the latent fifth receipt-less exit (see RCPT-01).
7. **IN-01…IN-04**: the four Info items, unchanged.
8. Three integration-test failures adjudicated pre-existing (two hardcode `/bin/pwd` ungated; one
   fixture overlaps the protected state root).
9. Sink receipts are not read-confidential (D-08 narrowing, operator decision).
10. `nono shell` interactive rendering broken on this host — carried onto Phase 119.
11. Four pre-fix broker receipts stranded under a `.nono-runtime` path; no migration.

---

## Gaps Summary

**No blockers.** All four ROADMAP Success Criteria are met in the codebase, verified against source
and against tests I ran myself with correct crate selectors. RCPT-01's terminal-exit enumeration —
the class this phase tripped on four separate times — was re-derived from scratch across all three
producers and is complete for every reachable exit. RCPT-02's chain-restart defect (CR-02), the
misdirected broker sink (CR-06), the missing correlation id (CR-03), the env-trusted sink location
(CR-04), and the permissive wire-channel resolution (WR-02) are all genuinely fixed in code with
tests, not merely narrated in a SUMMARY.

Three things a reader of this phase should not walk away without:

1. **CR-05 is fixed on two of three producers.** The broker adds an unrevoked per-run package-SID
   DENY ACE to the shared machine-wide sink. The "16 → 16 held steady" measurement does not cover
   that arm. Routed to human measurement.
2. **The D-08 narrowing is not consistently reflected.** Two constant doc comments in the same file
   as the correction still assert the read protection Task 3 falsified.
3. **RCPT-02's requirement text is stale.** "HMAC-chained `SecurityEventLayer`" names a mechanism
   D-25 deliberately and correctly did not use — and one that was never what "the existing audit
   chain" meant anyway. The implementation is right; the requirement text needs amending or an
   explicit accepted-divergence record. Only the operator can make that edit (D-24).

Status is `human_needed` rather than `passed` solely because of items (1) and (3) above — both need
an operator action that a verifier cannot take.

---

_Verified: 2026-09-06T14:39:54Z_
_Verifier: Claude (gsd-verifier)_
