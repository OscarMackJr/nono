---
phase: 118-per-session-enforcement-receipts
plan: 06
subsystem: windows-broker-attestation-receipts
tags: [windows, enforcement-receipts, broker, wire-contract, tdd, source-scan, keyless-chain]

dependency-graph:
  requires:
    - "crates/nono::EnforcementReceipt / LayerId / EntryPath / TokenArm / LayerReceiptRow / SessionOutcome (118-01)"
    - "crates/nono::hash_receipt_event / hash_receipt_chain / RECEIPT_*_DOMAIN (118-01)"
    - "crates/nono::deny_sid_on_path / try_set_mandatory_label (118-02/118-05)"
    - "crates/nono-cli/src/exec_strategy_windows/attestation.rs::required_layers_for_broker + BROKER_REQUIRED_LAYERS_ENV_VAR wire contract (117-08)"
  provides:
    - "crates/nono-cli/src/exec_strategy_windows/attestation.rs::required_not_applicable_for_broker + BROKER_NOT_APPLICABLE_LAYERS_ENV_VAR (D-27 sibling wire-contract channel)"
    - "crates/nono-shell-broker/src/main.rs::broker_census (13-row census from the widened wire contract)"
    - "crates/nono-shell-broker/src/main.rs::BrokerReceiptWriter / BrokerReceiptRecord (hand-rolled keyless sink writer)"
    - "crates/nono-shell-broker/src/main.rs::record_broker_receipt (D-03 write points wired into run())"
  affects:
    - "crates/nono-shell-broker/Cargo.toml (serde/serde_json/tempfile deps, Win32_Storage_FileSystem feature)"
    - "118-09 (nono receipt verify/show/list — must read this writer's JSONL alongside receipt_sink.rs's)"
    - "118-10 (cross-target clippy aggregation, manual broker-arm checkpoint)"

tech-stack:
  added: []
  patterns:
    - "shared classifier (broker_role) called by BOTH wire-contract producer functions so exactly one place decides Required vs NotApplicableOnBroker"
    - "set-equality/partition perturbation proof (union == ALL, zero overlap) rather than an arithmetic 11+2==13 count"
    - "hand-rolled, single-writer (no-mutex) keyless chain writer, mirroring receipt_sink.rs's shape without importing it (separate crate)"
    - "census computed once, reused across the gate's Ran/Refused branches (no re-probing)"
    - "generic 'discover every ..._ENV_VAR const' cross-crate scan, avoiding hardcoding even the const names, not just their string values"

key-files:
  created:
    - crates/nono-shell-broker/tests/broker_wire_contract_names.rs
  modified:
    - crates/nono-cli/src/exec_strategy_windows/attestation.rs
    - crates/nono-cli/src/exec_strategy_windows/launch.rs
    - crates/nono-shell-broker/src/main.rs
    - crates/nono-shell-broker/Cargo.toml

decisions:
  - "required_not_applicable_for_broker(entries, spawn_arm) takes the SAME (entries, spawn_arm) signature as required_layers_for_broker, not the plan's literal 0-arg/Vec-only interfaces text — AppContainerProfile's Broker-role classification genuinely differs between the two broker spawn arms (BrokerLaunch vs BrokerLaunchNoPty per CR-03.1), so a spawn-arm-agnostic function could not classify it correctly."
  - "broker_census gained an expected_app_container_sid parameter beyond the plan's literal 4-argument signature — without it, AppContainerProfile would classify Confirmed merely because SOME AppContainer SID was present, never verifying it is the SAME per-run SID broker_resume_gate itself requires. This would let the receipt claim a layer active more strongly than the gate's own security decision verified — the exact 'claims success while structurally incapable of reporting failure' anti-pattern named in 118-CONTEXT.md's pause-handoff framing."
  - "NONO_SESSION_ID (the existing house env-var convention documented in nono::supervisor::aipc_sdk and used by hook_runtime.rs/hook_runtime_windows.rs) is threaded to the broker's spawn environment for D-15's session-id correlation between nono.exe's own DirectCli receipt for broker.exe and the broker's own Broker receipt for the real confined grandchild — read 'audit-correlation only; accept empty', never fail-closed on absence."
  - "Only 3 of the broker's 5 TerminateProcess branches are receipt-instrumented (required-layers-absent, gate-fail, resume-fail) plus the Ran write before ResumeThread. The two label-application failures (OpenProcessToken / apply_low_il_label_to_token, which occur BEFORE the wire contract is even read) are left uninstrumented — a documented, reasoned scope decision, not an oversight; see Deviations."
  - "A ResumeThread failure after a successful gate produces TWO appended receipts for the same session (Ran, then a correcting Refused) rather than one — SessionOutcome::Ran is written before ResumeThread per the plan's literal D-03 instruction, but Ran's own doc says it means 'was resumed'; if ResumeThread then fails, a second truthful Refused record is appended rather than leaving a false Ran record uncorrected. A consumer reads the LAST record per session_id as authoritative (D-15's append-only, per-writer segment design)."

metrics:
  duration: "~2h30m"
  completed: 2026-08-17
---

# Phase 118 Plan 06: Broker Wire-Contract Widening + Own Receipt Pipeline (D-15/D-27) Summary

Widened the existing `NONO_BROKER_REQUIRED_LAYERS` wire contract with a sibling
`NONO_BROKER_NOT_APPLICABLE_LAYERS` channel so `nono-cli` (the single source of truth for
`layer_registry.rs`) tells `nono-shell-broker.exe` which of the 13 `LayerId`s are NOT its
responsibility on a given spawn arm — closing Finding 1's third gap without giving the broker a
third, hardcoded copy of registry knowledge (D-27). Gave the broker its own 13-row census, a
hand-rolled keyless-chain receipt sink writer (this crate cannot import `nono-cli`'s
`receipt_sink.rs`), and wired both into `run()`'s D-03 write points — a second, independently
writable `EnforcementReceipt` per launch, correlated with `nono.exe`'s own receipt by session id
(D-15).

## What Was Built

**Task 1 (`a5c06abb`)** — `test(118-06): widen broker wire contract with NotApplicable partition (D-27)`

- `crates/nono-cli/src/exec_strategy_windows/attestation.rs`: added `BROKER_NOT_APPLICABLE_LAYERS_ENV_VAR
  = "NONO_BROKER_NOT_APPLICABLE_LAYERS"` and `required_not_applicable_for_broker(entries, spawn_arm) ->
  Vec<LayerId>`. Extracted a shared `broker_role(entry, spawn_arm) -> BrokerRole { Required |
  NotApplicableOnBroker }` classifier that BOTH `required_layers_for_broker` (refactored to call it,
  behavior byte-identical — confirmed by the pre-existing test suite passing unchanged) and the new
  function call — exactly one place decides which of the two wire channels a row belongs to.
- `crates/nono-cli/src/exec_strategy_windows/launch.rs`: wired the new env var into BOTH the
  production broker-spawn call site (`spawn_windows_child`'s `BrokerLaunch`/`BrokerLaunchNoPty` arm)
  and the `write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file` live-run test
  harness's own broker-spawn code (which had documented itself as mirroring the production call site
  — kept that claim true rather than letting the harness drift onto a stale one-variable contract).
- Two new tests in `attestation.rs`'s `broker_wire_contract_tests` module:
  `required_not_applicable_for_broker_returns_the_11_row_complement_for_the_nopty_arm` (concrete
  count check against the "broker knows 2 of 13" framing, on the `BrokerLaunchNoPty` arm where both
  Broker-expected `Abort` rows apply) and
  `broker_wire_contract_partitions_all_13_layer_ids_with_zero_overlap_or_gap` (a real
  set-equality/partition check — union of both channels equals `layer_registry::ALL` with zero
  overlap, across BOTH spawn arms — not an arithmetic `11 + 2 == 13` count, which would still pass
  if a row were double-counted while another silently fell off both channels).

**Task 2 (`70ebe6ee`)** — `feat(118-06): broker's own 13-row receipt census + hand-rolled sink writer (D-15/D-27)`

- `crates/nono-shell-broker/Cargo.toml`: added `serde`/`serde_json` (workspace-pinned), a
  `[dev-dependencies]` section with `tempfile = "3"`, and the `Win32_Storage_FileSystem` windows-sys
  feature (needed for `FILE_GENERIC_READ`/`FILE_GENERIC_WRITE`/`FILE_EXECUTE`/`DELETE`).
- `broker_role`-analog for the broker side: `broker_required_row_status` classifies the two
  wire-contract-`Required` rows (`AppContainerProfile`/`MandatoryIntegrityLabel`) from the SAME probe
  results `broker_resume_gate` already computed (no re-probing) — any OTHER name reaching this
  function (required but not one of this binary's two attestable rows) fails toward `Unconfirmed`,
  matching `broker_resume_gate`'s own unrecognized-name refusal without assuming `Confirmed`.
- `broker_census(required_layers_raw, not_applicable_raw, app_container_probe,
  expected_app_container_sid, integrity_probe) -> Vec<nono::LayerReceiptRow>`: builds all 13 rows.
  `not_applicable`-channel rows classify `NotApplicable`; `required`-channel rows classify via
  `broker_required_row_status`; a row named in NEITHER channel — a version-skewed nono-cli/broker
  pair, or a `LayerId` this broker binary predates — fails toward `Unconfirmed`, never dropped and
  never promoted.
- Hand-rolled `BrokerReceiptRecord`/`BrokerReceiptWriter`: keyless (D-25, no `key` field), NO mutex
  (documented why — this binary's write call site inside `run()` is single-threaded by construction),
  JSONL envelope field names (`sequence`/`prev_head`/`leaf_hash`/`chain_head`/`receipt`) byte-identical
  to `receipt_sink.rs`'s `ReceiptRecord`. Sink directory resolution
  (`broker_receipt_sink_dir`) and guard application (`ensure_broker_receipt_sink_guarded`, D-08's DENY
  ACE + unconditional `NO_READ_UP` label) are hand-rolled from core (`crates/nono`) primitives only,
  since this crate cannot depend on `nono-cli`.
- `record_broker_receipt(session_id, pid, outcome, census, package_sid_for_guard)`: resolves+guards
  the sink, assembles the `EnforcementReceipt` (`entry_path: Broker`, `token_arm: None`), and writes
  it. Degrades VISIBLY on any emitter failure (`tracing::warn!`, D-04) — never aborts an
  already-decided resume/terminate outcome.
- Wired into `run()`: `NONO_BROKER_NOT_APPLICABLE_LAYERS` and `NONO_SESSION_ID` (house convention,
  "audit-correlation only; accept empty") are read alongside the existing fail-closed
  `NONO_BROKER_REQUIRED_LAYERS` read — the new channels feed the RECEIPT CENSUS only, never the
  resume/terminate DECISION. `census` is built once (reusing probe results) and consumed at every
  write point: `Refused` on the required-layers-absent branch (still probes read-only for the
  most truthful census that branch can produce), `Refused` on the gate-fail branch, `Ran` immediately
  before `ResumeThread`, and a correcting `Refused` if `ResumeThread` itself then fails (see decisions
  above).
- 8 new tests in a new `broker_receipt_tests` module: unrecognized-row-classifies-Unconfirmed,
  not-applicable-channel classification, AppContainerProfile SID-match (both matching and
  mismatching), MandatoryIntegrityLabel RID threshold (both sides), the JSONL field-name-match test,
  a 2-record chain-and-recompute-from-disk test (tempdir-backed), the sink-guard label test
  (tempdir-backed), and the D-14 sentinel round-trip.

### Negative-control demonstration (Task 2, required evidence — NOT committed)

Temporarily added `pub raw_app_container_sid: String` to `crates/nono/src/receipt.rs`'s
`EnforcementReceipt`, updated the 6 other struct-literal call sites across the tree that needed to
keep compiling (`receipt.rs`'s own 3 test literals, `attestation.rs`'s `build_enforcement_receipt`,
`agent_daemon/launch.rs`'s `build_daemon_receipt`, `receipt_sink.rs`'s test `sample_receipt`) with
`String::new()`, then threaded the REAL sentinel value into the broker's own
`sentinel_seeded_app_container_sid_never_leaks_into_the_serialized_broker_receipt` test's struct
literal (`raw_app_container_sid: SENTINEL.to_string()`). Re-ran that one test — it **FAILED** as
expected, with the panic transcript showing
`"raw_app_container_sid":"S-1-15-2-1-SENTINEL-BROKER-4f1a"` verbatim inside the serialized JSON
(captured live during execution). All changes (the field, and every touched struct literal across 4
files) were then reverted in full — confirmed via `git status --short` showing only
`nono-shell-broker`'s own files modified afterward, and `cargo test -p nono-shell-broker` (all 42
unit tests) passing again unchanged post-revert.

**Task 3 (`a74ed575`)** — `test(118-06): cross-crate wire-contract name-match discovery test (D-27)`

- `crates/nono-shell-broker/tests/broker_wire_contract_names.rs` (new file): a discovery-based
  cross-crate scanner following the house idiom (`env!("CARGO_MANIFEST_DIR")` + `read_to_string`, no
  `regex`, no `include_str!`). `extract_env_var_const_values` discovers EVERY
  `pub(crate) const ..._ENV_VAR: &str = "...";` declaration in `attestation.rs` generically — it does
  not hardcode either const's Rust identifier NAME, and it never hardcodes either env var's string
  VALUE anywhere in the discovery test (Test 2's synthetic stand-in strings are the one deliberate
  exception, by the plan's own design).
  - `broker_reads_every_attestation_rs_wire_contract_env_var_by_its_current_literal_value` (Test 1):
    reads `attestation.rs` fresh, discovers 2 `..._ENV_VAR` consts (non-vacuity-guarded), then reads
    `nono-shell-broker/src/main.rs` fresh and asserts both extracted values appear as the literal
    argument to an `std::env::var(...)` call.
  - `perturbation_proof_matcher_detects_a_name_drift_between_stand_in_sources` (Test 2): synthetic
    in-test strings standing in for both files (never a real file mutation) — proves the matcher
    reports a MATCH on a genuinely-matching pair (a vacuous-failure guard on the perturbation proof
    itself), then renames one value in the "attestation.rs" stand-in without updating the "main.rs"
    stand-in and confirms the matcher now reports a mismatch.
  - Per this plan's `<interfaces>` "On Task 3's design" rationale: deliberately does NOT attempt a
    per-`LayerId` reachability proof — Task 1's partition test already proves that against the real
    compiled registry data, more reliably than a text scan could.

## Deviations from Plan

### Auto-fixed / Rule 2 (missing critical functionality)

**1. [Rule 2] `broker_census` gained an `expected_app_container_sid` parameter**
- **Found during:** Task 2, designing `broker_required_row_status`'s `AppContainerProfile` arm.
- **Issue:** The plan's literal `broker_census` signature (4 params: `required_layers_raw`,
  `not_applicable_raw`, `app_container_probe`, `integrity_probe`) has no way to verify the probed
  AppContainer SID is the SAME per-run SID `broker_resume_gate` itself requires. Without it,
  `AppContainerProfile` would classify `Confirmed` merely because SOME AppContainer SID was present
  on the child token — a receipt claiming a layer active more strongly than the gate's own security
  decision verified. `118-CONTEXT.md`'s pause-handoff framing names this exact shape ("claims success
  while structurally incapable of reporting failure") as the anti-pattern this whole phase exists to
  close.
- **Fix:** Added `expected_app_container_sid: Option<&str>` as a 5th parameter, threaded from the
  SAME `expected_sid` value `broker_resume_gate` already receives at every call site.
- **Files modified:** `crates/nono-shell-broker/src/main.rs`
- **Commit:** `70ebe6ee`

**2. [Rule 2] `required_not_applicable_for_broker` takes `(entries, spawn_arm)`, not 0 args**
- **Found during:** Task 1, verifying `AppContainerProfile`'s Broker expectancy cell.
- **Issue:** The plan's literal interfaces text described `required_not_applicable_for_broker() ->
  Vec<LayerId>` with no parameters. `AppContainerProfile`'s `(EntryPath::Broker, ...)` expectancy is
  scoped to `Some(BROKER_LAUNCH_NO_PTY)` only (CR-03.1) — its classification genuinely differs between
  the two broker spawn arms (required on NoPty, not-applicable on the PTY/legacy arm). A spawn-arm-
  agnostic function could not classify this row correctly for both arms.
- **Fix:** Gave the function the SAME `(entries: &[LayerRegistryEntry], spawn_arm: WindowsTokenArm)`
  signature as its sibling `required_layers_for_broker`, sharing the `broker_role` classifier.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs`
- **Commit:** `a5c06abb`

### Rule 3 (necessary infrastructure the plan's file list didn't name)

**3. `crates/nono-cli/src/exec_strategy_windows/launch.rs` touched (not in this plan's frontmatter `files_modified`)**
- **Rationale:** Task 1's own `<action>` text explicitly instructed "Wire the new env var's
  comma-joined value into the broker's spawn environment at the same call site that already sets
  `NONO_BROKER_REQUIRED_LAYERS`" — this is only reachable via `launch.rs`. Without this edit the wire
  contract would be defined but never actually sent, making Task 2/3's consumer-side work
  unreachable from a real launch.
- **Scope respected:** Only the two existing broker-spawn env-pair-building blocks were touched (the
  production `spawn_windows_child` call site and the `write_deny_low_il_broker_no_pty_...` live-run
  test harness that documents itself as mirroring production) — no other launch-path logic changed.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/launch.rs`
- **Commits:** `a5c06abb`

**4. `NONO_SESSION_ID` threaded to the broker (not named in this plan's interfaces text)**
- **Rationale:** D-15 requires the broker's own receipt be "stitched by session id" with `nono.exe`'s
  own `DirectCli` receipt for `broker.exe` itself — without a real session id passed across the
  process boundary, the two receipts cannot be correlated at all, only invented per-broker-process
  identifiers that would never match. `crates/nono/src/supervisor/aipc_sdk.rs` already documents
  `NONO_SESSION_ID` as an existing house convention for exactly this purpose ("audit-correlation only;
  accept empty"), and `spawn_windows_child` already has a `session_id: Option<&str>` parameter in
  scope at the broker-spawn call site — reused rather than inventing a new channel.
- **Files modified:** `crates/nono-cli/src/exec_strategy_windows/launch.rs`,
  `crates/nono-shell-broker/src/main.rs`
- **Commits:** `a5c06abb`, `70ebe6ee`

### Scope decision (documented, not a deviation from a stated instruction)

**5. Only 3 of 5 `TerminateProcess` branches are receipt-instrumented**
- The plan's `<interfaces>` text says "`SessionOutcome::Refused` on each `TerminateProcess`
  fail-closed branch." `run()` has 5 such branches. The 3 instrumented ones (required-layers-absent,
  gate-fail, resume-fail) are the branches inside the documented "Plan 117-11: the broker's own
  attestation gate" section — the D-03 write point this plan's interfaces block names explicitly. The
  2 NOT instrumented (`OpenProcessToken` failure, `apply_low_il_label_to_token` failure) occur BEFORE
  the wire contract is even read, structurally analogous to `CreateProcessW`/`CreateProcessAsUserW`
  itself failing (also not instrumented) — no census-relevant data (env vars, probe results) has been
  gathered yet at that point, and instrumenting them would require restructuring the label-application
  block ahead of this plan's stated file scope. Named here as a residual, not silently assumed solved.

## TDD Gate Compliance

Task 1 was tagged `tdd="true"`. As in Plans 118-03/118-04, the executor committed the new production
code (`BrokerRole`, `broker_role`, `required_not_applicable_for_broker`) together with its tests in
one commit (`a5c06abb`) rather than separate RED/GREEN commits. No `test(...)` commit precedes a
`feat(...)` commit for Task 1's refactor in the literal two-commit sense.

Mitigating context, matching the prior two plans' precedent: `required_layers_for_broker`'s refactor
through the shared `broker_role` classifier was verified to be BEHAVIOR-IDENTICAL by the pre-existing
`broker_wire_contract_tests` suite (which independently re-derives the expected filter via its own
inline formula) passing unchanged, and every new assertion carries a real perturbation proof performed
live during execution — the partition test's set-equality check (verified it actually distinguishes
overlap/gap from a mere count), and Task 2's sentinel negative control (failed as expected, then
reverted). Flagging the literal RED/GREEN gate miss per the TDD compliance instruction rather than
silently claiming full compliance.

## Verification

- `cargo test -p nono-sandbox-cli --bin nono exec_strategy::attestation` — **49 passed**, 0 failed
  (47 pre-existing + 2 new: `required_not_applicable_for_broker_returns_the_11_row_complement_for_the_nopty_arm`,
  `broker_wire_contract_partitions_all_13_layer_ids_with_zero_overlap_or_gap`).
- `cargo test -p nono-shell-broker` — **44 passed**, 0 failed (34 pre-existing + 8 new
  `broker_receipt_tests` + 2 new `broker_wire_contract_names` integration tests, all in one `cargo
  test` invocation since the integration test file is picked up automatically).
- `cargo test -p nono-shell-broker --test broker_wire_contract_names` — **2 passed**, 0 failed,
  isolated re-run confirmed.
- `cargo check -p nono-shell-broker` — exit 0, confirms `serde_json` resolvable as a dependency.
- `grep -rc "NONO_BROKER_NOT_APPLICABLE_LAYERS" crates/nono-cli/src` — totals **2** (the
  `attestation.rs` const definition + a `launch.rs` doc comment at the spawn-environment call site).
- `cargo fmt --all -- --check` — clean (after `cargo fmt --all` auto-applied formatting).
- `cargo clippy -p nono-shell-broker --all-targets -- -D warnings -D clippy::unwrap_used` — clean
  (added `#[allow(clippy::unwrap_used)]` on the new `broker_receipt_tests` module, matching this
  file's own established per-test-module pattern; CLAUDE.md permits this exception in test modules).
- `cargo clippy -p nono-shell-broker -p nono-sandbox-cli --tests -- -D warnings -D
  clippy::unwrap_used` — clean (the `--bin nono`-only invocation still hits the SAME 10 pre-existing
  `receipt_sink.rs` dead_code errors documented as expected in `118-05-SUMMARY.md` — "not yet called
  by production code" state, unrelated to this plan's files; `--tests` compiles the `#[cfg(test)]`
  module that exercises every symbol and is fully clean).
- **Diligence pass beyond the plan's own gate:** `cargo test -p nono-sandbox-cli --bin nono
  --no-fail-fast` (background run, ~129s): **1704 passed, 12 failed** — the failing names are EXACTLY
  the documented known-good baseline set (`audit_session::tests::discover_sessions_...`,
  6× `config::tests::*`, `exec_strategy::labels_guard::tests::non_owned_path_with_a_foreign_label_is_exempt_not_a_coverage_gap`,
  `profile_cmd::tests::test_init_allowed_when_pack_has_same_short_name`, 3× `protected_paths::tests::*`)
  — zero regressions.

## Cross-Target Clippy Inventory (for Plan 118-10)

| File | Gate(s) present | What this plan changed there |
|------|------------------|-------------------------------|
| `crates/nono-cli/src/exec_strategy_windows/attestation.rs` | No real `#[cfg(target_os)]` gate in this file itself; the whole `exec_strategy_windows` module tree is `#[cfg(target_os = "windows")] #[path = ...]`-included from `main.rs` | `BROKER_NOT_APPLICABLE_LAYERS_ENV_VAR`, `BrokerRole`, `broker_role`, `required_not_applicable_for_broker`, 2 new tests |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` | Same tree — Windows-only via `#[path]` inclusion | Two broker-spawn env-pair-building blocks (production + one live-run test harness) |
| `crates/nono-shell-broker/src/main.rs` | `#[cfg(windows)] mod broker { .. }` (whole-file gate; the module is empty/stub on non-Windows) | `broker_required_row_status`, `broker_census`, `broker_receipt_sink_dir`, `ensure_broker_receipt_sink_guarded`, `BrokerReceiptRecord`, `BrokerReceiptWriter`, `record_broker_receipt`, `run()`'s D-03 write-point wiring, `broker_receipt_tests` module |
| `crates/nono-shell-broker/tests/broker_wire_contract_names.rs` | No `#[cfg(target_os)]` gate — pure source-text scanning, compiles and runs on every host (reads the source tree's files, not the compiled target) | New file |

Neither local cross-target gate (`cross clippy --target x86_64-unknown-linux-gnu`, `cargo-zigbuild
clippy --target x86_64-apple-darwin`) was run by this plan — deferred to Plan 118-10 per the phase's
own verification strategy (`118-VALIDATION.md`: "Before `/gsd:verify-work`: ... both cross-target
clippy gates").

## Self-Check: PASSED

```
FOUND: crates/nono-cli/src/exec_strategy_windows/attestation.rs
FOUND: crates/nono-cli/src/exec_strategy_windows/launch.rs
FOUND: crates/nono-shell-broker/src/main.rs
FOUND: crates/nono-shell-broker/Cargo.toml
FOUND: crates/nono-shell-broker/tests/broker_wire_contract_names.rs
FOUND commit: a5c06abb
FOUND commit: 70ebe6ee
FOUND commit: a74ed575
```

## Threat Flags

None. This plan's changes are exactly the surface its own `<threat_model>` names
(T-118-18/T-118-19/T-118-20/T-118-21): the version-skewed wire-contract fail-safe (mitigated —
unrecognized/absent rows fail toward `Unconfirmed`, proven by
`unrecognized_row_classifies_unconfirmed_never_dropped`), the broker's hand-rolled sink writer
diverging from `receipt_sink.rs`'s shape (accepted per D-15, mitigated by the field-name-match test),
the CREATE_SUSPENDED read-before-write race (mitigated — the receipt write happens before
`ResumeThread`, matching the D-03 ordering guarantee the other two producers use), and the residual
per-`LayerId` wire-reachability gap Task 3's name-match test cannot see by design (accepted, covered
at runtime by the fail-safe and, for the real end-to-end wire, by Plan 118-10's manual checkpoint). No
new network endpoint, auth path, or schema change at a trust boundary was introduced — the
`NONO_SESSION_ID` env var reused an EXISTING house convention rather than inventing a new one, and the
receipt sink directory is the SAME one Plan 118-05 already guards.

## Known Stubs

None. `broker_census`, `BrokerReceiptWriter`, `ensure_broker_receipt_sink_guarded`, and
`record_broker_receipt` are complete, functioning implementations, live-wired into `run()`'s real
control flow (not deferred to a later plan, unlike 118-03/118-04's `build_enforcement_receipt`/
`build_daemon_receipt`, which were assembly functions proven correct but not yet call-site-wired).
The two uninstrumented `TerminateProcess` branches (Deviation 5) are a documented, reasoned scope
boundary, not an unfinished stub.
