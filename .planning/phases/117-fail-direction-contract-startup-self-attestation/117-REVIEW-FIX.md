---
phase: 117-fail-direction-contract-startup-self-attestation
fixed_at: 2026-08-10T00:00:00Z
review_path: .planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md
iteration: 1
findings_in_scope: 22
fixed: 17
partial: 3
skipped: 2
status: partial
---

# Phase 117: Code Review Fix Report

**Source review:** `.planning/phases/117-fail-direction-contract-startup-self-attestation/117-REVIEW.md`
**Iteration:** 1
**Scope:** `critical_warning` — all 10 Critical + all 12 Warning findings.

**Summary:**

- Findings in scope: 22
- Fixed: 17
- Partial (mitigated, honestly recorded, open item remains): 3
- Skipped: 2
- Extra defect found during the pass and fixed: 1 (NEW-01)

Every Critical was addressed. The phase's central claim — "nono can no longer
report *enforcing* while a confinement layer is silently inert" — now holds on
the paths the review said it did not: on a default Windows build
`AttestationDecision::Proceed` is reachable, legitimate `BrokerLaunchNoPty`
launches are no longer refused, and an unconfined Medium-IL `cmd.exe` no
longer confirms `MandatoryIntegrityLabel`.

## Verification gate (run against the final tree)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo check --workspace --all-targets` | clean |
| `cargo check --workspace --all-targets --features layer-fault-injection` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` | clean |
| `cargo clippy --workspace --all-targets --features layer-fault-injection -- …` | clean |
| `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) | clean — `Finished dev profile in 3m 53s` |
| `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` | clean — exit 0, zero errors. Three independent runs (`41m 21s` and `62m 00s` cold, `1m 52s` incremental on the exact final tree). Docker engine up, pinned `cross` image; no PARTIAL→CI fallback needed. |

Per-fix test runs are cited under each finding. All work was done in an
isolated git worktree on branch `gsd-reviewfix/117-1506`.

---

## Fixed

### CR-01: `MandatoryIntegrityLabel` probe was vacuous and measured the wrong object

**Commit:** `ca0ce589`
**Files:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` (row `probe`),
`.../attestation.rs` (`classify_live_probe`), `.../launch.rs` (gate test)

The row's `probe` is now `ProbeKind::ConfiguredOnly` and
`MandatoryIntegrityLabel` is grouped into `classify_live_probe`'s defensive
fail-secure fallback, so a future registry/dispatch drift lands on
`Unconfirmed` rather than a false `Confirmed`. The genuine token-RID
observation moved to `nono-shell-broker`, which observes the actual confined
grandchild (CR-03).

**Non-vacuity evidence:**
`exec_strategy::attestation::registry_tests::mandatory_integrity_label_cannot_confirm_from_an_unconfined_medium_il_token`
classifies the real registry row against the test runner's own unconfined
Medium-IL process. Counterfactual run: transiently restoring the
`LiveTokenOrJobQuery` dispatch made it fail with
`left: Confirmed, right: Confirmed` — i.e. the pre-fix code did confirm an
unconfined process. Reverted after measurement.

### CR-02: `JobObjectContainment` accepted membership in *any* job

**Commit:** `6db5a33a`
**Files:** `crates/nono/src/attestation.rs` (`probe_in_job`, new `JobHandle`),
`crates/nono-cli/src/exec_strategy_windows/attestation.rs` (`AttestationInput::containment_job`),
`.../launch.rs` (gate + call site passes `containment.job`),
`crates/nono-cli/src/agent_daemon/launch.rs` (passes `job_raw_owned`)

`probe_in_job` now takes a mandatory job handle and **rejects a
null/invalid one fail-closed** rather than silently restoring the ANY-job
query — the vacuous form is no longer reachable by accident.

**Non-vacuity evidence:**
`nono::attestation::windows_probe_tests::in_job_probe_distinguishes_the_assigned_job_from_another_job`
spawns a real `CREATE_SUSPENDED` child, assigns it to job A, and asserts
`true` for A and `false` for a second live job B. Counterfactual run:
restoring the null-job form made the second assertion fail with
`left: Some(true), right: Some(false)` — confirming this host is a nested-job
host on which the old probe could never return a negative. Reverted.
Two gate-level tests were added on both mirrors
(`live_child_not_in_the_containment_job_aborts_naming_job_object_containment`,
`real_appcontainer_process_outside_the_named_job_aborts_on_job_containment`).

### CR-03: broker resume gate nested inside `if is_app_container`, ignoring all but one layer

**Commit:** `78127b24`
**Files:** `crates/nono-shell-broker/src/main.rs`,
`crates/nono-cli/src/exec_strategy_windows/{attestation.rs,layer_registry.rs,launch.rs}`

All three sub-defects:

1. **Class coverage.** The gate is hoisted out of `if is_app_container`. The
   legacy/PTY arm had no suspend window at all (it spawned without
   `CREATE_SUSPENDED` and never resumed), so it now spawns
   `CREATE_SUSPENDED`, runs the same gate, and resumes.
2. **Coverage.** `app_container_resume_gate` → `broker_resume_gate`: names
   outside `BROKER_ATTESTABLE_LAYERS` refuse resume instead of being
   ignored, and `MandatoryIntegrityLabel` is genuinely attested by re-reading
   the child token's integrity RID (`<= SECURITY_MANDATORY_LOW_RID`).
3. **Fail-open default.** `unwrap_or_default()` is gone; a missing or empty
   `NONO_BROKER_REQUIRED_LAYERS` refuses resume.

The registry's `AppContainerProfile` `(Broker, …)` cell is scoped to the
`BrokerLaunchNoPty` spawn shape, because the PTY shape creates no
AppContainer — the old unqualified cell made the wire contract demand an
unsatisfiable layer, which the broker papered over by ignoring it.

**Non-vacuity evidence** (`cargo test -p nono-shell-broker`, 9/9 passing) —
every one asserts a DENY:
`mandatory_integrity_label_is_attested_and_can_fail` (Medium-IL RID `0x2000`
refuses; failed probe refuses), `unattestable_required_layer_refuses_resume`,
`empty_required_layers_refuses_resume`,
`app_container_sid_must_match_the_expected_per_run_value` (foreign
`S-1-15-2-9-9-9` refuses; no expected value refuses),
`app_container_required_on_a_non_app_container_shape_refuses_resume`.
CLI-side: `pty_broker_arm_is_not_asked_to_attest_app_container_profile` and
`every_emitted_broker_required_layer_is_attestable_by_the_broker` (4/4).

> ⚠ **Requires human verification.** Adding `CREATE_SUSPENDED` to the legacy
> ConPTY broker spawn could not be exercised on this host — interactive PTY
> launches need a real console, and per project memory broker spawns fail
> `GLE=87` under git-bash/MSYS. The mechanism is byte-identical in shape to
> the AppContainer arm that already ships it. Someone should run an
> interactive `nono shell` from a real PowerShell console before this ships.

### CR-04: `WfpEgressFilters` aborted every non-WFP-backed `BrokerLaunchNoPty` launch

**Commit:** `ef8b96a9` (folded into CR-09 — the same threading fixes both)
**Files:** `crates/nono-cli/src/exec_strategy_windows/{layer_registry.rs,attestation.rs,mod.rs}`

Applicability now follows the **resolved network backend**, not the token
arm. `AppliedLayers::{wfp_egress_filters,firewall_rules_egress}` are
tri-state (`None` = this backend is not part of the launch's composition), so
a `FirewallRules`-backed or unrestricted launch classifies the WFP row
`NotApplicable` instead of `Unconfirmed`. Expectancy widened to all five
`DirectCli` arms + `Daemon`, since the backend decides, not the arm.

**Non-vacuity evidence:**
`network_row_for_an_unselected_backend_is_not_applicable` asserts BOTH
directions: `NotApplicable` + `Proceed` when the backend was not selected,
and `Abort{WfpEgressFilters, Unconfirmed}` when it WAS selected and the
enforcing component did not confirm.

### CR-06: daemon attested `DaclAncestorReadAttrs` although it never grants read-attributes

**Commit:** `38c20af9`
**Files:** `crates/nono-cli/src/agent_daemon/launch.rs`, plus the registry
expectancy change in `ef8b96a9`

`DaclAncestorReadAttrs` is dropped from the daemon's set and from its
`(Daemon, None)` expectancy cell. The remaining DACL rows are attested from
the caller's actual apply result rather than a hardcoded `vec![]`.

**Non-vacuity evidence:**
`real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable`
drives the SAME real AppContainer-confined, job-assigned suspended child four
ways: `Proceed`, then `Abort{DaclPackageSidGrant}` when the DACL guard is
reported unapplied, `Abort{WfpEgressFilters}` when scoping was required and
filters are missing, `Abort{AppContainerProfile}` against a foreign package
SID. None of those three denials was reachable before.

### CR-08: four Windows block-net tests failing or passing vacuously

**Commit:** `76432e05`
**Files:** `crates/nono-cli/tests/env_vars.rs`

All four tests (and their now-unused-without-the-feature helper) are gated on
`all(target_os = "windows", feature = "layer-fault-injection")`, and CR-10
adds the make target + CI job that runs that build — the review is explicit
that gating without a runner silently drops the coverage. The four stale
`NONO_TEST_HARNESS` comments and `.env` calls are removed (Plan 04 deleted
that runtime gate).

**Evidence:** `cargo check -p nono-sandbox-cli --test env_vars` is
warning-free both with and without the feature.

### CR-09: `AttestationDecision::Proceed` unreachable; downgrade signal fired on every launch

**Commit:** `ef8b96a9`
**Files:** `crates/nono-cli/src/exec_strategy_windows/{layer_registry.rs,attestation.rs,launch.rs,mod.rs}`

New `layer_registry::{LayerApplication, AppliedLayers}` vocabulary. The
caller states, per launch, which layers it applied; `PreparedWindowsLaunch`
derives it on demand from its own guards so it cannot drift, and
`spawn_windows_child` records the broker-Authenticode row where that decision
is actually made. A `ConfiguredOnly` row that WAS applied is the expected
baseline and no longer counts as a downgrade; a row that was NOT applied
classifies `Unconfirmed` and hits its own outcome. Defaults are fail-secure
(unreported ⇒ `NotApplied`).

**Non-vacuity evidence:**
- `unapplied_configured_only_row_aborts` — the same row, same arm, reported
  unapplied, returns `Abort{Unconfirmed}`. Structurally unreachable before.
- `applied_configured_only_row_proceeds_without_a_downgrade` — `Proceed`.
- At the real gate with a real job-contained suspended child:
  `launch_missing_an_expected_configured_only_layer_is_refused` flips exactly
  one applied flag and turns `Ok` into
  `LayerAttestationFailed{MandatoryIntegrityLabel, Unconfirmed}`;
  `fully_applied_launch_passes_the_gate` is the paired positive.

### CR-10: CINT-03 suite and D-32 drift gate behind a feature nothing enabled

**Commits:** `1f65a892`, `95568e6a`
**Files:** `crates/nono-cli/tests/layer_registry_meta_test.rs`, `Makefile`,
`.github/workflows/ci.yml`

`layer_registry_meta_test.rs` drops the feature gate entirely (it reads
source text; it touches no seam). `make test-layer-fault-injection` and a
`Windows Layer Fault Injection` CI job run
`cargo test -p nono-sandbox-cli --features layer-fault-injection` and the
broker equivalent.

**Evidence:** `cargo test -p nono-sandbox-cli --test layer_registry_meta_test`
now runs 5 tests in a **default** build; it previously compiled to zero.

A follow-up commit adds `-- --test-threads=1` to both runners: every
force-unavailable seam is a process-global `AtomicBool`, and under the
default parallel harness
`dacl_guard::tests::ancestor_traverse_grants_owned_ancestors_and_reverts_on_drop`
fails with "forced unavailable by test seam" (12/12 pass serially). Without
this the new CI job would have been red on its first run.

### WR-01: probes accepted *any* value, not the expected one

**Commits:** `78127b24` (broker), `38c20af9` (daemon), `5831b1bc` (CLI core)
**Files:** `crates/nono-shell-broker/src/main.rs`,
`crates/nono-cli/src/agent_daemon/launch.rs`,
`crates/nono-cli/src/exec_strategy_windows/{attestation.rs,launch.rs}`

All three cores compare against the expected value the caller already holds:
the broker and daemon compare the probed AppContainer SID to the per-run
package SID; the CLI core requires this launch's synthetic session SID to be
among the child token's restricting SIDs. The CLI core's own
`AppContainerProfile` dispatch needs no change — that row has no `DirectCli`
expectancy cell at all (Blocker-1), so it never reaches the live probe there.

**Non-vacuity evidence:**
`restricted_token_requires_this_launch_own_session_sid` (unrestricted token +
synthetic expected SID ⇒ `Unconfirmed`; no expected value ⇒ `Unconfirmed`),
`app_container_sid_must_match_the_expected_per_run_value` (broker),
`foreign_sid_decision` arm of the daemon test above.

### WR-02: daemon silently dropped `WfpEgressFilters`; its captured local was unused

**Commit:** `38c20af9`

`network_scoping_required` is now a parameter, alongside whether the
filter-add succeeded. The row is `NotApplicable` when scoping was not
requested, `Abort` when it was and the filters are not installed.

**Non-vacuity evidence:** the `unscoped_decision` arm of
`real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable`
asserts `Abort{WfpEgressFilters}`.

### WR-03: `host_gated_rows_are_loud` could not fail

**Commit:** `dfbb5cae`
**Files:** `crates/nono-cli/tests/layer_registry_meta_test.rs`

`manual_verification_section()` slices the SPEC at the first `## ` heading
containing "manual"/"host-gated"; both loudness tests search only that tail.

**Non-vacuity evidence:**
`manual_verification_section_excludes_the_registry_table` asserts the section
is a strict suffix and does not contain `## Layer registry` — it fails if the
helper ever regresses to returning the whole document.

### WR-05: byte-identical duplicate DACL test

**Commit:** `b7c2a836`
**Files:** `crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs`,
`crates/nono-cli/tests/layer_force_unavailable.rs`,
`crates/nono-cli/tests/layer_registry_meta_test.rs`, SPEC

The shared `AppliedDaclGrantsGuard` seam now reports the layer it actually
implements (`DaclPackageSidGrant` — CR-05 established it only ever grants the
package SID), and there is one correctly-labelled test instead of two
identical mislabelled ones. `DaclSessionSidGrant` gains an honest
`MANUALLY_VERIFIED` entry and SPEC row whose verification step is a **code
read**, not a run.

**Evidence:** `every_registry_row_has_a_test`, `host_gated_rows_are_loud`,
`spec_matches_registry`, `registry_call_sites_exist` pass; `dacl_guard`'s 12
seam tests pass with `--features layer-fault-injection -- --test-threads=1`.

### WR-07: daemon abort path relied on `KILL_ON_JOB_CLOSE`

**Commit:** `38c20af9`

`TerminateProcess(process_handle_raw, 1)` is called explicitly before
`cleanup_failed_agent`, matching the adjacent steps 6/6.5/6.6 idiom. The
job-close cascade cannot terminate the child when the abort reason IS
"`JobObjectContainment` unconfirmed" — by hypothesis the process is not in
that job — and the same `Drop` closed the process handle, orphaning a
suspended process.

### WR-08: `DaemonAttestationDecision::Proceed` never constructed

**Commit:** `38c20af9`

It is now the outcome of a launch where every modelled layer holds.

**Non-vacuity evidence:** the `Proceed` assertion in
`real_appcontainer_job_process_proceeds_and_the_negatives_are_reachable`.

### WR-09: expectancy under-modelled the DACL rows

**Commit:** `ef8b96a9`

`DACL_PACKAGE_SID_SCOPED_EXPECTANCY` covers all five `DirectCli` arms +
`Daemon`, matching `execution_runtime.rs` setting `package_sid: Some(..)`
unconditionally. Three genuinely-applied layers no longer classify
`NotApplicable` on four arms of five.

### WR-10: SPEC self-contradictions and stale deferrals

**Commit:** `d25edee2`
**File:** `proj/SPEC-windows-fail-direction-contract.md` (committed with
`git add -f`; `proj/` is gitignored)

1. The "Structural constraints" paragraph is reconciled with the table and
   with `broker_expected_rows_are_abort_only`'s `probe != NotApplicable`
   scoping, stating the `MinifilterAbsence` carve-out and its reason.
2. SC4-2 moved to **RESOLVED IN THIS PHASE by Plan 117-10**.
3. SC4-4 moved to **RESOLVED IN THIS PHASE by Plan 117-04**; the Finding
   column is kept verbatim as the historical record.

The same commit rewrites the registry table for every registry change in this
pass, updates the downgrade-banner section for what now counts as a
downgrade, updates the latency table for the renamed broker gate test, and
adds an `RF-01..RF-14` "Review-fix pass" section carrying the two open
operator decisions into the contract document itself.

**Evidence:** `spec_matches_registry`, `registry_call_sites_exist`,
`host_gated_rows_are_loud`, `security_assumptions_are_loud` all pass.

### WR-11: unvalidated `session_id` joined into a filesystem path

**Commit:** `df6dc004`
**File:** `crates/nono-cli/src/output.rs`

`session_id_is_safe_path_component` is an allow-list (ASCII alphanumeric plus
`-`/`_`, non-empty, ≤128 chars), so separators, `..`, drive prefixes, ADS
colons, trailing dots/spaces and non-ASCII homoglyphs cannot appear at all.
Rejection returns `None`, which means "no dedup marker" ⇒ MORE banner
visibility, never less.

**Non-vacuity evidence:** `rejects_traversal_and_separator_shapes` drives 13
hostile shapes and asserts both that the validator rejects them and that the
marker-path helper returns `None` rather than an escaped path.

---

## Partial — mitigated, open item recorded

### CR-05: `DaclSessionSidGrant` has no enforcing call site (PARTIAL)

**Commit:** `ef8b96a9` (registry), `d25edee2` (SPEC), `b7c2a836` (test/seam label)

**What was fixed:** the registry no longer makes a false claim. The row's
expectancy is emptied (so `classify_row` returns `NotApplicable` on every
arm), its stale `call_sites` citations are cleared (they pointed at the
PACKAGE-SID guard), the seam reports `DaclPackageSidGrant`, and the SPEC + a
`MANUALLY_VERIFIED` entry record the discrepancy with a code-read
verification step. The `LayerId` variant is retained because Phase 118
receipts share this vocabulary.

**What is NOT fixed — needs an operator decision:** whether the
`WriteRestricted` arm SHOULD grant `config.session_sid` write on the writable
grant set, restoring the mechanism `mod.rs`'s own `_applied_dacls` comment
still describes ("so confined writes under WRITE_RESTRICTED pass the
restricting-SID double check"). The grantee was changed to the package SID in
Plan 62-12.

- **If YES:** construct a second `AppliedDaclGrantsGuard` with
  `config.session_sid` in `prepare_live_windows_launch`, restore the row's
  `(DirectCli, WriteRestricted)` expectancy and its call sites, and set
  `AppliedLayers::dacl_session_sid_grant` accordingly. Note this WIDENS DACLs
  on user-owned paths with a SID that appears in exactly one token, and it
  changes what a `WriteRestricted`-arm child can write. It needs a live
  `WriteRestricted` run (a profile with `windows_low_il_broker = false`,
  non-PTY) to validate, which this host could not perform.
- **If NO:** delete the `LayerId::DaclSessionSidGrant` variant outright in a
  follow-up, once Phase 118's receipt vocabulary is settled.

I did not guess: making a DACL-widening change on the strength of a code
comment, in a security-critical enforcement pipeline I cannot exercise, is
exactly the class of wrong-guess-in-a-security-gate the brief says to avoid.

### CR-07: `HKLM\…\RequiredLayers` is a documented no-op (PARTIAL)

**Commit:** `cef3b3ba`
**File:** `crates/nono/src/machine_policy.rs`

**What was fixed:** the control is no longer *silently* ignored.
`parse_policy` detects a configured `RequiredLayers` sub-key and emits a
`RequiredLayersNotEnforced` warning naming every layer it found and stating
plainly that the build does not enforce them (degrade-not-abort on an
unreadable sub-key, mirroring `parse_telemetry_config`, per D-26). The
`RequiredLayersPolicy` doc comment carries a NOT-YET-ENFORCED section
distinguishing what exists (`attest_and_decide`'s tighten-only union and its
fail-closed unrecognized-name rejection) from what is missing.

**What is NOT fixed — needs an operator decision:** enforcement itself.

- **Decision 1 — carrier.** Where is the already-read machine policy carried
  to the Windows launch path? `ExecConfig` (touches every construction site,
  including the daemon's), or a new parameter on
  `execute_direct`/`execute_supervised` (narrower, but a fifth thing to
  thread through `spawn_windows_child`)? The daemon's Phase 83 D-04 "SOLE
  read" invariant means it must NOT be re-read at the gate.
- **Decision 2 — blast radius.** Enabling it means a fleet registry key can
  refuse launches, including escalating a `FailOpen` row like
  `MinifilterAbsence` to `Abort`. That is the point of the control, but it is
  an availability decision an operator should make deliberately.
- **Decision 3 — CLI flag.** The doc comment promises "a local CLI flag can
  only add to (never remove from) this set", but no `--required-layers` flag
  exists. Ship it with the reader, or drop that sentence.

I would implement it as: thread `machine_egress_policy.required_layers.required`
through a new `machine_required_layers: &[String]` field on `ExecConfig`,
populate `AttestationInput::machine_required_layers` from it at the gate, and
add `--required-layers` as a repeatable flag feeding
`required_layers_override`. That is a ~4-file change, but it is gated on
Decisions 1–3.

### WR-12: three duplicated decision cores with no shared conformance test (PARTIAL)

**Commits:** `78127b24`, `38c20af9`

**What was fixed:** two of the three divergences the review named are gone —
the broker now fail-closes on unrecognized required-layer names exactly as
`attest_and_decide` does, and the daemon models `WfpEgressFilters` instead of
ignoring it. Of the three prescribed conformance assertions, (b) landed:
`every_emitted_broker_required_layer_is_attestable_by_the_broker` mirrors
`BROKER_ATTESTABLE_LAYERS` in the CLI crate and fails the build if a new
Broker-expected `Abort` row is added without teaching the broker to probe it.
Assertion (c) is moot — the daemon no longer has a hardcoded downgraded list
to compare against the registry.

**What is NOT fixed:** assertion (a) — nothing asserts that the broker's
literal `"NONO_BROKER_REQUIRED_LAYERS"` equals
`BROKER_REQUIRED_LAYERS_ENV_VAR`. A rename on either side still silently
disconnects the wire contract. Doing this properly needs either a shared
`build.rs`-generated constant or a test that reads the broker's source text;
both are judgement calls about where the shared-constant seam should live,
and `nono-cli` is binary-only so a normal `pub const` import is impossible.
The failure is at least no longer silent in the permissive direction — the
broker now fail-closes on an absent variable (CR-03.3), so a rename produces
a hard refusal rather than an unattested launch.

---

## Skipped

### WR-04: `every_registry_row_has_a_test` accepts a function *name*; `MANUALLY_VERIFIED` is unbounded

**Reason:** not attempted — this needs a design decision about how much
strength the meta-test should have, and the strongest option the review
suggests ("run the tests and assert their outcome rather than grepping for
their names") is a test-harness architecture change, not a fix.

**What is needed:** decide between (a) assert the test function exists AND
the row's env-var bridge name appears in `command_runtime.rs` — cheap, closes
the empty-stub hole partially; (b) cap `MANUALLY_VERIFIED` at a declared
maximum so the allow-list cannot keep growing silently; (c) have the meta-test
shell out and assert outcomes — strongest, slowest, and needs the
`layer-fault-injection` feature the meta-test deliberately no longer requires
(CR-10). Note that CR-10 and WR-05 already reduced the blast radius: the gate
now actually executes, and the one duplicate-by-name row is gone.

### WR-06: stale call-site citations; self-check only verifies file existence

**Reason:** not attempted — the correct fix (cite stable symbol names,
`"mod.rs::prepare_live_windows_launch/AppliedLabelsGuard"`, and have the
self-check assert the file contains that symbol) is a mechanical rewrite of
all 13 rows' `call_sites` **plus** the SPEC's call-site column **plus**
`registry_call_sites_exist`. Doing half of it would leave the registry citing
two incompatible citation formats — worse drift than the line numbers.

Partially mitigated in passing: `DaclSessionSidGrant`'s citations were the
most wrong of the set (they named the package-SID guard) and are now cleared
(CR-05). The `// TODO(117-12)` acknowledging unclosed line drift remains.

**What is needed:** a decision on the citation format (symbol-only, or
`file.rs::symbol` with the line as a non-asserted hint), then one mechanical
pass over `layer_registry.rs`, the SPEC table and the self-check.

---

## Extra defect found during this pass (not in REVIEW.md)

### NEW-01: a Phase 117 unit-test fixture deleted `crates/nono-cli/` on every run

**Commit:** `2d09e154`
**Files:** `crates/nono-cli/src/exec_strategy_windows/network.rs`,
`.../launch.rs`

Phase 117 Plan 10 added
`attestation_gate_tests::firewall_rules_guard()`, which constructs
`NetworkEnforcementGuard::FirewallRules { staged_dir: PathBuf::from("."), .. }`.
`NetworkEnforcementGuard::Drop` calls `cleanup_network_enforcement_staging`,
which ran an unconditional `std::fs::remove_dir_all` on that field — so
dropping the fixture ran `remove_dir_all(".")` with cargo's test CWD, the
`crates/nono-cli` package root. **The whole 200-file package tree was deleted
three separate times during this fix session**, once mid-commit.

Two changes: `cleanup_network_enforcement_staging` now refuses any path that
is not a strict subdirectory of `%TEMP%/nono-net-block`, compared by path
components (`Path::starts_with`, not string `starts_with` — CLAUDE.md footgun
#1), and logs the refusal; the fixture points at a never-created path under
that root. A `Drop` impl must never be able to turn a mis-constructed guard
into an arbitrary recursive delete.

**Non-vacuity evidence:** running `cargo test --bin nono exec_strategy::`
before the change left 200 deleted files in `git status`; after it, zero
(verified repeatedly for the rest of the session).

---

## Open verification

- **CR-03's `CREATE_SUSPENDED` change to the legacy ConPTY broker spawn** is
  unexercised on this host (see CR-03 above).
- **Baseline test failures.** `broker_launch_assigns_child_to_job_object` and
  `write_deny_low_il_broker_no_pty_prevents_child_write_to_medium_il_file`
  fail in the fix worktree because `nono-shell-broker.exe` was not pre-built
  into that worktree's target dir. These are environment preconditions, not
  regressions — they assert their own precondition explicitly. The known
  11 pre-existing `nono-sandbox-cli` Windows failures were not re-swept
  end-to-end (a full `--bin nono` sweep is the operation that triggered the
  NEW-01 tree deletion, and was avoided after that was diagnosed); targeted
  runs of every test module touched by this pass are green.

---

_Fixed: 2026-08-10_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
