---
phase: 118-per-session-enforcement-receipts
plan: 10
subsystem: windows-receipt-phase-gate
tags: [windows, receipts, cross-target-clippy, human-verify, d-08, sink-guard, scope-correction]

# Dependency graph
requires:
  - phase: 118-per-session-enforcement-receipts (Plan 05)
    provides: "receipt_sink.rs: ensure_sink_guarded (the D-08 guard under test), ReceiptWriter, sink at %PROGRAMDATA%\\nono\\receipts"
  - phase: 118-per-session-enforcement-receipts (Plans 07/08)
    provides: "production call sites writing receipts from nono.exe and nono-agentd.exe"
  - phase: 118-per-session-enforcement-receipts (Plan 09)
    provides: "nono receipt list|show|verify — the read side used to confirm on-disk receipt shape"
provides:
  - "Both mandatory cross-target clippy gates run clean at --workspace scope for Phase 118 (Task 1)"
  - "D-17 per-invocation receipt latency, MEASURED not estimated (Task 1)"
  - "receipt_sink.rs module-doc record of every Phase 118 discretionary decision (Task 2)"
  - "D-08 SCOPE CORRECTION: the sink guard is a WRITE-INTEGRITY guard only; read confidentiality is explicitly NOT claimed (Task 3 + operator decision)"
  - "Named mirror asymmetry between the CLI and daemon ensure_sink_guarded call sites, recorded at both sites so a future widening cannot fix one and miss the other"
affects: ["Phase 118 close-out", "any future phase claiming confidentiality for the receipt sink", "Phase 119 security-model boundary statement"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Correction-first doc ordering: the falsifying correction is placed ABOVE the historical account it invalidates, and the specific false sentence is marked inline (`^^^ FALSIFIED`) rather than left for a reader to reconcile against a note 30 lines away (P117 anaphoric-claim lesson applied to prose)"
    - "Discriminating verification form: grant the capability layer permissive access to the object under test so the ONLY remaining thing that can refuse is the mechanism being tested — an ungranted denial is ambiguous between 'policy never granted it' and 'the guard refused it'"

key-files:
  created:
    - .planning/phases/118-per-session-enforcement-receipts/118-10-SUMMARY.md
  modified:
    - crates/nono-cli/src/receipt_sink.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-cli/src/agent_daemon/launch.rs
    - crates/nono/src/sandbox/windows.rs
---

# Plan 118-10 Summary — Phase Gate

## Accomplishments

- **Task 1 (commit `f19752e8`)** — both mandatory cross-target clippy gates green at `--workspace`
  scope: `cross clippy --target x86_64-unknown-linux-gnu` (4m59s, clean) and `cargo-zigbuild clippy
  --target x86_64-apple-darwin` (57.69s, clean, `SDKROOT` unset). No findings needed fixing. D-17
  latency MEASURED (N=30, fresh `session_id` per iteration): avg 5.7ms, p50 5.6ms, min 2.6ms, max
  13.5ms, recorded durably at `crates/nono-cli/src/exec_strategy_windows/launch.rs:1568`.
- **Task 2 (commit `f81a009d`)** — all four open "Claude's Discretion" items from `118-CONTEXT.md`
  recorded in `receipt_sink.rs`'s module doc: sink location/layout, retention policy (explicitly
  NONE, with the must-never-truncate-mid-chain constraint stated for any future implementation),
  the Event Log pointer (explicitly NOT implemented, with justification), and confirmation that
  `entry_path`/`token_arm` are first-class fields.
- **Task 3 (commit `050d0710`)** — the blocking human-verify checkpoint was RUN on a real host.
  **It FAILED**, in the specific sense that two of its four denial checks did not hold. The
  operator's disposition was to narrow D-08's claim rather than widen the guard. See
  "Deviations from Plan" — this is the substantive content of this plan.

## Task Commits

| Task | Commit | Content |
|---|---|---|
| 1 | `f19752e8` | Cross-target clippy gates + D-17 latency measurement |
| 2 | `f81a009d` | Discretionary decisions in `receipt_sink.rs` module doc |
| 3 | `050d0710` | D-08 scope correction across 4 sites (docs only, no logic change) |

## Deviations from Plan

### Task 3's acceptance criteria were NOT met, and were superseded by an operator decision

The plan required: *"Read access to the sink from the confined child returns ACCESS_DENIED (or
equivalent denial) on the restricted-token arm and on the broker Medium-IL arm; write access is
likewise denied on both arms; the supervisor's own process can still read and write the sink
directory without modification."*

**Measured result, 2026-09-05, non-elevated session, dev-layout `target/debug/nono.exe`, real
PowerShell console:**

| # | Check | Arm | Result |
|---|-------|-----|--------|
| 1 | Confined child READ | WriteRestricted | **FAIL — read succeeded** |
| 2 | Confined child WRITE | WriteRestricted | PASS — denied, no file created |
| 3 | Confined child READ | Broker / AppContainer | **FAIL — read succeeded** |
| 4 | No-session-SID arm | — | N/A — arm structurally unreachable |
| 5 | Supervisor read + write | supervisor | PASS |

### Operator response (VERBATIM, per the plan's resume-signal contract)

The operator ran the checks and supplied the raw console transcripts, which are the primary
evidence for every row above. Their disposition, quoted exactly as typed across two messages:

> narrow D-08's claim to write-integrity and amend the d

> admend the document

**Attribution note, recorded so a later reader is not misled:** the operator performed the checks
and made the disposition. The check-by-check narrative, the root-cause analysis, and the wording of
the doc amendments in `050d0710` are the orchestrator's, derived from the operator's transcripts
plus source inspection. The operator did not type a check-by-check verdict, and none is attributed
to them here.

## Verification Evidence

### The two failing reads

```
& $nono run --read $sink -- cmd /c type $victim
  → printed receipt fd4be8f26f567495 in full
  → that session's own receipt: token_arm "WriteRestricted", RestrictedToken "Confirmed",
    JobObjectContainment "Confirmed", outcome "Ran"  (child was genuinely confined)

& $nono run -p claude-code --allow-cwd --allow $sink -- cmd /c type $victim
  → printed receipt 52f75abfdb514ac7 in full
  → broker: AppContainer profile registered; app_container=true; child_exit_code=0
```

### Why check 3's FIRST result was discarded as non-attributable

An earlier broker run with a READ-ONLY grant (`--read $sink`) *was* denied. That denial was traced
to `dacl_guard.rs:215` (`if !rule.access.contains(AccessMode::Write)`) — the package-SID DACL grant
is applied only to WRITABLE rules, so an AppContainer child hits its own default-deny on a path
that was never granted to its package SID. A control run confirmed the AppContainer child reads
granted files normally (`--allow-cwd` on a r+w cwd, exit 0). Re-running check 3 with `--allow
$sink` supplied the package-SID grant, removed AppContainer default-deny from the picture, and the
read succeeded.

**Recording this explicitly because the first result would have been logged as a PASS.** It was
denied, on the right arm, by the right command shape — and it proved nothing about D-08. A control
that isolates the mechanism under test is what separated the two.

### Write-denial (the property that DOES hold)

```
& $nono run -- cmd /c "echo pwned> $sink\intruder.txt"                → "Access is denied.", exit 1
& $nono run --allow $sink -- cmd /c "echo pwned> $sink\intruder.txt"  → "Access is denied.", exit 1
Test-Path "$sink\intruder.txt"                                        → False
```

The second form is the load-bearing one: the capability layer granted read+write and the write was
*still* refused, so the guard — not the sandbox's path policy — is what stopped it.

### Supervisor access (T-118-31)

`Get-Content $victim` succeeded; `Out-File "$sink\supervisor-probe.txt"` succeeded; read back and
removed cleanly. The guard does not lock out the legitimate writer.

### On-disk ACL evidence

```
icacls C:\ProgramData\nono\receipts
  S-1-5-117-*:(DENY)(M)   ×7      ← session SIDs only; NO (OI)(CI) inheritance flags
  Mandatory Label\Low Mandatory Level:(NR,NX)   ← directory only, Low RID
  (no S-1-15-2-* AppContainer package SID DENY entry, after two AppContainer runs)

icacls C:\ProgramData\nono\receipts\52f75abfdb514ac7.jsonl
  NT AUTHORITY\SYSTEM:(I)(F) / BUILTIN\Administrators:(I)(F)
  TWGGLOBAL\OMack:(I)(F)          ← the same user the confined child runs as
  BUILTIN\Users:(I)(RX)
  (no DENY ACE, no Mandatory Label line at all)
```

### Root cause

Both guards are scoped to the sink **directory object** and neither reaches the files inside it:

1. `nono::deny_sid_on_path` applies its ACE with `NO_INHERITANCE`
   (`crates/nono/src/sandbox/windows.rs:2046`), and the mandatory label carries no `(OI)(CI)`.
   Reading an existing file consults the FILE's DACL, which inherits allow-only ACEs from
   `%PROGRAMDATA%`. Creating a file is an operation on the DIRECTORY, where the DENY does apply —
   which is exactly why write-denial holds and read-denial does not.
2. `nono::try_set_mandatory_label` hardcodes the `LW` SDDL alias
   (`crates/nono/src/sandbox/windows.rs:1058`), pinning the object to a LOW RID. Windows denies
   only subjects strictly BELOW the object's level, so a Low-IL child is EQUAL, not below.

This falsified the justification previously recorded at `exec_strategy_windows/launch.rs` for
omitting the package-SID deny — *"an AppContainer-arm child is Low-IL by construction, so the label
alone still covers it"* — which was wrong on both counts above.

### Post-amendment gates

| Gate | Result |
|---|---|
| `cargo doc -p nono-sandbox-cli --no-deps` | 0 errors; 33 warnings for bin `nono`, 2 for `nono-agentd` — **unchanged pre-existing baseline**; ZERO warnings cite any edited file |
| `cargo doc -p nono-sandbox --no-deps` | 0 errors; no warning cites `sandbox/windows.rs` |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy -p nono-sandbox --all-targets --all-features -D warnings -D clippy::unwrap_used` | clean |
| `cargo clippy --workspace ...` | **RED — pre-existing, see Known Open Items** |

## Decisions Made

- **D-08 narrowed to WRITE INTEGRITY only** (operator decision at the Task 3 checkpoint). The sink
  guard prevents a confined child from creating, overwriting, or deleting sink entries — so it
  cannot forge or destroy the enforcement record. It provides **no read confidentiality**. Widening
  the guard (an `(OI)(CI)` deny at the call site, threading the package SID through both producers,
  or per-file guards) remains available and deliberately unimplemented.
- **Amend rather than widen.** No enforcement behavior changed in `050d0710`; it is docs-only.
- **Historical claims retained, marked, not deleted.** The falsified text stays in place with the
  correction ABOVE it and an inline `^^^ FALSIFIED` marker on the specific false sentence, because
  the correction is unintelligible without the account it corrects.
- **The Plan 05 probe is not called wrong.** Its reported Low-IL denial does not reproduce against
  the sink, and it never recorded the object/inheritance shape it measured, so the discrepancy
  cannot be reconstructed. The doc says exactly that rather than asserting the probe was mistaken.

## Threat Flags

- **T-118-30 (guard failing silently on one arm while appearing to work on another) — MATERIALISED,
  and in the exact shape the threat register anticipated.** The broker arm's first result looked
  like a pass. Only a control run that isolated AppContainer default-deny revealed it was
  non-attributable. The register's own mitigation ("test all named arms separately rather than
  accepting a single passing case") is what caught it; it needed one further step — testing each
  arm under a form where only the guard could refuse.
- **T-118-31 (guard locking out the legitimate writer) — NOT materialised.** Supervisor retained
  read and write throughout.
- **NEW — receipt confidentiality is not a property this system has.** A confined child can read
  every session's receipt: layer census, `session_id`, `pid`, `entry_path`, `token_arm`, outcome.
  Content-free by construction (D-05/D-14), so no user data, paths, or arguments are exposed. But
  "which confinement layers are inert on this host" is legible to an untrusted agent, which is
  reconnaissance value ahead of an escape attempt. Named here so Phase 119's security-model
  boundary statement can carry it explicitly rather than inheriting a claim that was never true.

## Known Open Items (carried to phase close-out, NOT resolved by this plan)

1. ~~**`cargo clippy --workspace` is RED**~~ — **RESOLVED at close-out, commit `816b683a`.** The two
   `dead_code` errors (`ReceiptWriter::session_id` field + `session_id()`/`file_path()` accessors)
   were removed, not suppressed. 118-07 had named a hypothesis for a future consumer — Plan
   118-09's read side — and 118-09 built it using `session_file_path()` instead, because
   `receipt_commands.rs` never holds a `ReceiptWriter`. The predicted caller was unfulfillable, so
   the surface was deleted. `cargo clippy --workspace --all-targets --all-features -- -D warnings
   -D clippy::unwrap_used` now **exits 0**. The replacement test assertion (on the derived
   `<session_id>.jsonl` filename) is perturbation-proved.

   **Why the phase's own gates missed it for five plans:** the clippy scope alternated between
   `--tests` (118-06, 118-09), where the accessors ARE used, and `--bin` (118-07, 118-08), where
   they are not. 118-05's original remedy — add a test assertion — cannot clear a bin-target
   `dead_code`, because the bin build excludes `#[cfg(test)]`. Task 1's cross-target gates were
   clean and equally blind: `receipt_sink` is `#[cfg(target_os = "windows")]`, so linux-gnu and
   apple-darwin never compile it.
2. ~~**Three unadjudicated integration-test failures**~~ — **ADJUDICATED at close-out: all three are
   PRE-EXISTING, none is a Phase 118 regression.** Full reasoning in `deferred-items.md`. In brief:
   they were absent from the documented 12-name baseline because that baseline is scoped to
   `--bin nono` and all three live in separate integration-test binaries no prior sweep reached
   (`cargo test` is fail-fast across targets). The two `audit_attestation` tests invoke `/bin/pwd`
   with **zero platform gating** and so cannot ever have passed on Windows; that file has been
   untouched since 2026-06-24. The `env_vars` test grants a temp dir containing its own
   `fake-localappdata\nono`, so nono correctly fail-closes on a protected-state-root overlap —
   product right, fixture wrong; neither the test nor any of the three files able to emit that
   refusal changed during the phase window. **Not fixed here** — the repair is test-fixture work
   outside a receipts phase, with options recorded in `deferred-items.md`.
3. **DENY ACEs accumulate unbounded on the sink** — one per session, never revoked (observed 5 → 7
   across two runs). At fleet scale this walks toward the 64KB ACL limit. Not a correctness bug
   today; no cleanup path exists.
4. **`/gsd:code-review 118` (D-22) has not been run** and is required before the phase is done.

## Self-Check: PASSED (with Task 3's acceptance criteria NOT met — superseded by operator decision)

All three tasks executed. Tasks 1 and 2 met their acceptance criteria. **Task 3 did not** — two of
its four denial checks failed — and this summary records that failure rather than reporting the
plan green. The plan's own contract (a blocking human checkpoint whose typed response is the
pass/fail signal) was honored: the check was run by the operator on a real host, was not
self-approved, and its failure drove a scope correction rather than being absorbed silently.

This is the outcome the checkpoint existed to produce. An automated gate would have reported green:
check 3 alone looked like a clean pass, and only a control run designed to isolate the mechanism
exposed it.

## Next Phase Readiness

Phase 118's code work is complete and its receipts are being written, retrieved, and verified in
production paths. The close-out sequence remains: adjudicate items 1–3 above → `/gsd:code-review
118` → regression gate → `gsd-verifier` (RCPT-01/02/03) → hand-edit STATE.md / ROADMAP.md /
REQUIREMENTS.md (no SDK state writers, per D-24).

**RCPT-01/02/03 are unaffected by the Task 3 finding.** None of them asserts read confidentiality:
RCPT-01 is emission, RCPT-02 is tamper-evidence (which rests on write-integrity, verified intact),
RCPT-03 is three-state distinguishability. The narrowed D-08 claim does not reduce any requirement
this phase committed to.
