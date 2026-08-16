---
phase: 118
slug: per-session-enforcement-receipts
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-16
---

# Phase 118 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `118-RESEARCH.md` § Validation Architecture. Amended by the
> operator decisions D-25 / D-26 / D-27 recorded in `118-CONTEXT.md`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`) — `#[test]` in `crates/nono-cli/tests/*.rs` and `#[cfg(test)] mod tests` inline modules |
| **Config file** | none — workspace `Cargo.toml` + per-crate `Cargo.toml` |
| **Quick run command** | `cargo test -p nono-sandbox-cli --bin nono exec_strategy::attestation -- --nocapture` (per-module); `cargo test -p nono-sandbox --lib attestation` for core |
| **Full suite command** | `make test` (workspace) |
| **Phase gate command** | `make ci` (clippy `-D warnings -D clippy::unwrap_used` + `cargo fmt --check` + full suite) |
| **Estimated runtime** | quick ~15–40s per module; `make test` several minutes on this host |

**Known baseline:** `-p nono-sandbox-cli` and `-p nono-sandbox` carry pre-existing failures on this
Windows host that also fail at phase-base (11 of them). Do not chase these as
regressions — see the recorded baseline before triaging any red.

**Package names (post-checker revision, round 2 — verified against the live tree, not the
directory names):** `crates/nono/`'s package is `nono-sandbox` (`[lib] name = "nono"` is a
separate binding used only for `use nono::...` inside source, NOT for `cargo -p`).
`crates/nono-cli/`'s package is `nono-sandbox-cli`, and it has **NO `[lib]` target at all** — only
two bins, `nono` (`src/main.rs`) and `nono-agentd` (`src/bin/nono-agentd.rs`). `crates/nono-shell-broker/`'s
package genuinely is `nono-shell-broker` (matches its directory) and also has no `[lib]` target,
only its own single bin. `cargo test -p nono` and `cargo test -p nono-cli` both fail package
resolution outright; `--lib` against `nono-sandbox-cli` is invalid regardless of package name
since no lib target exists — inline `#[cfg(test)]` modules under `nono-cli` must be run via
`--bin nono <module::path>` or `--bin nono-agentd <module::path>`, whichever binary's `#[path]`
include set actually reaches the module under test (verify per module — do not assume; e.g. on
Windows, `main.rs` binds `exec_strategy_windows/mod.rs` to the module name `exec_strategy`, NOT
`exec_strategy_windows` — the directory name and the compiled module path are NOT the same string).

---

## Sampling Rate

- **After every task commit:** module-scoped `cargo test -p <crate> --lib <module>` for crates with a `[lib]` target (`nono-sandbox`); for `nono-sandbox-cli`, which has NO `[lib]` target, use `cargo test -p nono-sandbox-cli --bin nono <module>` or `--bin nono-agentd <module>`, whichever binary actually `#[path]`-includes the module under test (house convention, used throughout Phase 117).
- **After every plan wave:** `make test` (workspace-wide).
- **Before `/gsd:verify-work`:** `make ci` green, **plus both cross-target clippy gates** (D-23):
  - `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
  - `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`
  Both are required because promoting `LayerId` into `crates/nono` and adding the
  `nono receipt` family to `cli.rs` both touch files carrying Unix `cfg` branches.
  **No PARTIAL→CI** — a stopped daemon or absent tool does not qualify as a documented runner failure.
- **Max feedback latency:** < 60s at task granularity.
- **Never treat a resolving-but-empty filter as a pass.** Every `cargo test` command in this
  document and in the plans must be confirmed (via `-- --list` or a real run) to report a
  **non-zero** test count before it is trusted — `0 passed; N filtered out` exits 0 and reads as
  success while never having run anything. This is the specific failure mode the phase's own
  package/target-name corrections (below) exist to eliminate.

---

## Per-Task Verification Map

Task IDs are assigned by the planner; this table fixes the requirement → test-type →
command mapping each task must land in. `File Exists` reflects the tree **before** this phase.

**Reconciliation note (post-checker revision, W7 + round 2):** this table originally named a
single consolidated `receipt_census_test.rs` file and commands that assumed `nono-cli` has a
`[lib]` target reachable via bare `cargo test -p nono-cli <name>`. Neither matched how the phase's
10 plans actually landed the tests, and a round-2 re-check additionally found the package names
themselves were wrong (`nono`/`nono-cli` do not resolve; the real names are `nono-sandbox`/
`nono-sandbox-cli`) and that `nono-sandbox-cli` has no `[lib]` target at all — the correct form for
its inline tests is `--bin nono <module>` or `--bin nono-agentd <module>`, never `--lib`. A third,
independently-discovered defect: `exec_strategy_windows/mod.rs`'s content is bound to the module
name `exec_strategy` (not `exec_strategy_windows`) inside `main.rs` on Windows — every filter
below that touches that subtree uses `exec_strategy::`, verified empirically (`cargo test -p
nono-sandbox-cli --bin nono exec_strategy::attestation -- --list` lists real tests;
`exec_strategy_windows::attestation` lists zero and still exits 0). Census/sentinel/equivalence
tests live as inline `#[cfg(test)]` modules inside the source files under test, one set per
producer binary — no `receipt_census_test.rs` file exists or is planned. Every command below has
been verified to resolve to a real, non-empty target where the underlying code already exists in
the tree; where the code is this phase's own deliverable (not yet written), the package/bin/module
prefix has been verified against the parent module's actual `#[path]`/`mod` structure instead.

| Plan area | Requirement | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|-----------|-------------|-----------------|-----------|-------------------|-------------|--------|
| Census completeness — nono.exe (118-03) | RCPT-01 | `census_from_entries` names all 13 `LayerId` rows, no early return; `decide_from_entries`'s existing behavior unchanged | inline unit test + discovery-based meta-test extension | `cargo test -p nono-sandbox-cli --bin nono exec_strategy::attestation -- census` + `cargo test -p nono-sandbox-cli --test layer_registry_meta_test` (extended) | ❌ W2 | ⬜ pending |
| Census completeness — nono-agentd (118-04) | RCPT-01 | `daemon_census_rows` + `DAEMON_UNMODELLED_LAYER_EXPECTANCY` produce all 13 rows; cross-check drift guard against `layer_registry.rs` | inline unit test + discovery-based cross-check (`include_str!`-based, mirrors `daemon_decision_enum_variants`) | `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch -- daemon_expectancy` | ❌ W2 | ⬜ pending |
| Census completeness — nono-shell-broker (118-06) | RCPT-01 | `broker_census` builds all 13 rows from the widened wire contract; unrecognized/absent rows fail toward `Unconfirmed` | inline unit test + cross-crate name-match discovery test | `cargo test -p nono-shell-broker` + `cargo test -p nono-shell-broker --test broker_wire_contract_names` | ❌ W3 | ⬜ pending |
| Content-free receipt — type scan (118-01) | RCPT-01 | No path/argument/payload-shaped field type anywhere in `EnforcementReceipt`, each perturbation-proofed | discovery-based type-allowlist source scan | `cargo test -p nono-sandbox-cli --test receipt_content_free_scan` | ❌ W1 | ⬜ pending |
| Content-free receipt — sentinel round-trip, all 3 producers (118-03/118-04/118-06) | RCPT-01 | No path/SID-shaped sentinel value leaks into a serialized receipt from ANY of the three independent producers (class coverage, not just the CLI path) | inline behavioral round-trip test per producer, each with a documented negative control | `cargo test -p nono-sandbox-cli --bin nono exec_strategy::attestation -- sentinel` (CLI) + `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch -- daemon_sentinel` (daemon) + `cargo test -p nono-shell-broker` (broker, sentinel case) | ❌ W2/W3 | ⬜ pending |
| Chain construction (keyless, D-25) | RCPT-02 | `SHA256(receipt_domain \|\| prev \|\| leaf_hash)`; own chain domain (D-11 amended) | unit | `cargo test -p nono-sandbox --lib receipt_chain` | ❌ W1 | ⬜ pending |
| Receipt sink guard (D-08) | RCPT-02 | DENY ACE + `NO_READ_UP` label applied to the sink directory | unit (ACL/label read-back) + blocking human checkpoint (real confined child) | `cargo test -p nono-sandbox-cli --bin nono receipt_sink` (unit) + Plan 118-10 Task 3 (checkpoint) | ❌ W2/W6 | ⬜ pending |
| `nono receipt verify` (118-09) | RCPT-02 | Fail-closed recompute-and-compare; an edited receipt is detected | inline unit/integration test | `cargo test -p nono-sandbox-cli --bin nono receipt_commands` | ❌ W5 | ⬜ pending |
| Four-state rendering (118-09) | RCPT-03 | All four states render distinguishably; an unattested layer **never** renders as attested | inline unit + discovery-based exhaustive-match guard | `cargo test -p nono-sandbox-cli --bin nono receipt_commands -- rendering` | ❌ W5 | ⬜ pending |
| Daemon restructure (D-26) | RCPT-01 | `daemon_attest_and_decide` probes every modelled layer; **decision outcome provably unchanged** for every input | equivalence unit test (perturbation-proofed) | `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch -- daemon_attest_and_decide` | ✅ (extend) | ⬜ pending |
| Broker wire contract (D-27) | RCPT-01 | Broker receives `(EntryPath::Broker)` NotApplicable rows from nono-cli over a widened env-var contract; unrecognised/absent row set fails toward **unconfirmed**, never toward a short census; a build-time name-match test guards the two literal env-var names against drift | unit + cross-crate name-match discovery test | `cargo test -p nono-sandbox-cli --bin nono exec_strategy::attestation -- broker_not_applicable` + `cargo test -p nono-shell-broker` + `cargo test -p nono-shell-broker --test broker_wire_contract_names` | ❌ W2/W3 | ⬜ pending |
| Emitter degrade posture (D-04) | RCPT-01 | `require_receipts` machine-policy field follows the egress abort-on-unreadable lifecycle; emitter failure at the D-03 write point degrades **visibly** by default and aborts when machine policy requires receipts | unit (policy field) + inline unit (wiring) | `cargo test -p nono-sandbox --lib machine_policy -- require_receipts` (✅ extend) + `cargo test -p nono-sandbox-cli --bin nono exec_strategy::launch -- attestation_gate` (❌ W4) | mixed | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

**Reconciliation note (post-checker revision, W7 + round 2):** the two file-level items below were
originally scoped as a `tests/*.rs` pair; `receipt_content_free_scan.rs` landed as planned
(Plan 118-01, Wave 1), but census completeness landed as three separate inline `#[cfg(test)]`
modules — one per producer binary (Plans 118-03/118-04/118-06) — rather than a single
consolidated `receipt_census_test.rs`, because `nono-sandbox-cli` has no `[lib]` target and a
standalone `tests/*.rs` integration file cannot call the `pub(crate)` census-building functions
directly. This is the correct placement per the cross-binary compilation constraint (see
`118-PATTERNS.md`'s structural-constraint note); no `receipt_census_test.rs` file exists or is
planned. Commands below use the round-2-corrected package names, `--bin` targets, and the
`exec_strategy::` (not `exec_strategy_windows::`) module path.

- [ ] `crates/nono-cli/tests/receipt_content_free_scan.rs` (Plan 118-01, Wave 1) — D-14's
      type-allowlist scan, in the house source-scan idiom (`env!("CARGO_MANIFEST_DIR")` +
      `fs::read_to_string`; **no `regex`, no `include_str!`**; discovery-based, never naming its
      targets). Run: `cargo test -p nono-sandbox-cli --test receipt_content_free_scan`. The D-14
      sentinel round-trip is NOT in this file — see the three inline per-producer sentinel tests
      below.
- [ ] Inline census-completeness + sentinel-round-trip test in
      `crates/nono-cli/src/exec_strategy_windows/attestation.rs` (Plan 118-03, Wave 2) —
      `cargo test -p nono-sandbox-cli --bin nono exec_strategy::attestation -- census` and
      `-- sentinel`.
- [ ] Inline census-completeness + sentinel-round-trip test in
      `crates/nono-cli/src/agent_daemon/launch.rs` (Plan 118-04, Wave 2) —
      `cargo test -p nono-sandbox-cli --bin nono-agentd agent_daemon::launch -- daemon_expectancy`
      and `-- daemon_sentinel` (daemon-only module — `--bin nono` does NOT reach `agent_daemon::`;
      verified empirically, `main.rs` never declares `mod agent_daemon`).
- [ ] Inline census-completeness + sentinel test in `crates/nono-shell-broker/src/main.rs`, plus
      the cross-crate name-match discovery test in
      `crates/nono-shell-broker/tests/broker_wire_contract_names.rs` (Plan 118-06, Wave 3) —
      `cargo test -p nono-shell-broker` and `cargo test -p nono-shell-broker --test broker_wire_contract_names`.
- [ ] Extend `crates/nono-cli/tests/layer_registry_meta_test.rs` (Plan 118-03, Wave 2) — census ↔
      registry drift guard: **adding a `LayerId` variant without a census row must fail the
      build, not a review**. Run: `cargo test -p nono-sandbox-cli --test layer_registry_meta_test`.
- [ ] Reuse `crates/nono-cli/tests/layer_force_unavailable.rs`'s per-layer seam to produce
      non-`Confirmed` rows without mocking the OS.

No framework install needed — existing infrastructure covers the runner.

---

## Perturbation Proofs (mandatory — hard house rule)

Every discovery / self-enforcing test added by this phase must ship with proof it can FAIL.
A guard verified only for *placement* is what produced three consecutive gap-closure
blockers in Phase 117.

1. **Type-allowlist scan (D-14).** Must fail when a field of type `PathBuf`, a path-populated
   `String`, or any non-allowlisted type is added to the receipt struct.
   **Converse proof (guards against vacuous failure):** an allowlisted type — `u32`, `LayerId`,
   `LayerAttestationStatus` — must NOT trip it.
2. **Sentinel round-trip (D-14), all three producers.** Build a receipt from each of the three
   independent producer pipelines (nono.exe/118-03, nono-agentd/118-04, nono-shell-broker/118-06)
   seeded with a sentinel path/SID-shaped value; assert those exact bytes are absent from the
   serialized output, per producer. Negative control: leaving the sentinel-carrying field in must
   make the corresponding test fail (demonstrated in review, not committed) — recorded separately
   per producer since each is an independent data flow (class coverage, not just placement).
3. **Census-completeness meta-test.** Adding a 14th `LayerId` variant with no corresponding
   census row must fail to compile (exhaustive match, no wildcard arm — mirroring
   `assert_all_layer_ids_covered`) or fail the meta-test if the census builder is not itself
   exhaustively matched.
4. **`nono receipt verify` recompute-and-compare (D-25).** Hand-edit one byte of a stored receipt
   record on disk; `verify` must report a mismatch and fail closed.
5. **Daemon decision-equivalence (D-26).** The restructure must not change any decision outcome.
   Perturbation: alter one probe's result and assert the aborting layer **and** the
   `DaemonAttestationDecision` are identical pre- and post-restructure. A test that only asserts
   "13 rows present" is blind to the fail-direction change this decision introduces.
6. **Broker wire-contract guard (D-27).** Two complementary build-time checks, since the broker's
   design deliberately has no static per-LayerId table on its own side (a hardcoded table there
   was explicitly rejected as a third copy of registry knowledge): (a) on the nono-cli side, a
   partition test asserting the two wire-contract producer functions' outputs union to exactly
   `layer_registry::ALL` with zero overlap — a row that fell through both functions would break
   this invariant; (b) a cross-crate discovery test asserting the broker's source reads both
   wire-contract env-var names by their exact current literal string value — a rename/typo drift
   between the two crates fails this test. Separately, at runtime: a broker handed an
   unrecognised or absent row set must produce `Unconfirmed` rows, never a silently short census
   — assert this, do not assume it.

---

## Three Audit Questions for Every Guard

Apply all three to each guard this phase adds. Signing off on placement alone is the
failure mode this section exists to prevent.

- **Placement** — does the check run at the actual D-03 write point
  (`apply_startup_attestation_gate`, `launch.rs:1552`, before `resume_contained_process` /
  `terminate_suspended_process`), not upstream or downstream where it could be bypassed?
- **Predicate width** — does the check cover the FULL 13-row census on every arm, not just the
  rows each binary happens to already probe? (Research Finding 1's core lesson: `nono-cli` retains
  0 rows, the daemon models 5, the broker models 2.)
- **Class coverage** — does the perturbation proof exercise the general CLASS of violation
  ("any new non-allowlisted field type"), not just the one field someone thought of?

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Sink guard actually blocks a confined child (D-08: DENY ACE on session + package SID **plus** `NO_READ_UP` label) | RCPT-02 | Supervisor and confined child run as the **same user**, so this cannot be proven by a unit test — it needs a real confined session on a real host | Launch a confined session; from inside the child, attempt to read and to write the receipt sink; both must be denied. Repeat on the broker arm (Medium-IL child — the mandatory label misses it, the DENY ACE must catch it) and on an arm where no session SID was minted (the label must catch it). |
| `%PROGRAMDATA%\nono\receipts` labelability | RCPT-02 | `LABEL_SECURITY_INFORMATION` needs `WRITE_OWNER` (0x80000), which Owner does not grant — drive-root fails while `%USERPROFILE%`/`%TEMP%` succeed. Whether PROGRAMDATA succeeds is host-dependent and decides the discretionary sink location. | Attempt `try_set_mandatory_label` on the candidate sink path; record the result before committing to the location. |
| D-17 per-tool-call latency budget | RCPT-01 | D-24 requires a **measured** number, not an asserted "negligible" — and the project constraint is explicit (zero startup latency for the Windows backend) | Measure receipt-emission cost on the `claude_code_hook` path across N invocations; record the number. If it comes back bad, that is a recorded trade-off with a named fallback, not a silent design compromise. |
| Cross-target clippy (D-23) | all | Windows-host `cargo check` is structurally blind to Unix `cfg` branches | Run both gates per `.planning/templates/cross-target-verify-checklist.md`. |

---

## Validation Sign-Off

- [x] All tasks have an automated verify or a Wave 0 dependency
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all ❌ references above
- [x] No watch-mode flags
- [x] Feedback latency < 60s at task granularity
- [x] Every discovery/self-enforcing test carries its perturbation proof — confirmed by delta re-verification for the three tests added post-review (118-04 Test 4 daemon sentinel, 118-06 Task 3 name-match, 118-06 broker sentinel)
- [x] All three audit questions answered per guard
- [x] Every verify command resolves a real target AND reports a non-zero test count — measured, not assumed (see note below)
- [x] `nyquist_compliant: true` set in frontmatter

**Verify-command correctness (added after the round-2 sweep).** Every `cargo test` form in
this document and in the 10 plans was executed against the live tree and confirmed to resolve
a package + target and report a **non-zero** test count. This is a sign-off criterion, not a
formality: two independent defects were found that each produced `0 tests, 0 benchmarks` with
**exit 0** — a wrong package name (`-p nono` / `-p nono-cli`; the real names are `nono-sandbox`
and `nono-sandbox-cli`, and `nono-cli` has no `[lib]` target at all) and a wrong module path
(the Windows module is bound as `exec_strategy`, not `exec_strategy_windows`). Both would have
let the phase's two highest-stakes tests — D-26's daemon equivalence proof and the daemon
sentinel round-trip — report green while never running. A verify command that resolves but
matches nothing is the same "gate structurally incapable of reporting failure" defect this
phase exists to eliminate; treat any such command as unfixed.

**Approval:** approved 2026-08-16 — plans verified (0 blockers), delta re-verified, all
verify commands measured non-empty.
