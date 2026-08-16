---
phase: 118
slug: per-session-enforcement-receipts
status: draft
nyquist_compliant: false
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
| **Quick run command** | `cargo test -p nono-cli --lib exec_strategy_windows::attestation -- --nocapture` (per-module); `cargo test -p nono --lib attestation` for core |
| **Full suite command** | `make test` (workspace) |
| **Phase gate command** | `make ci` (clippy `-D warnings -D clippy::unwrap_used` + `cargo fmt --check` + full suite) |
| **Estimated runtime** | quick ~15–40s per module; `make test` several minutes on this host |

**Known baseline:** `-p nono-cli` and `-p nono` carry pre-existing failures on this
Windows host that also fail at phase-base (11 of them). Do not chase these as
regressions — see the recorded baseline before triaging any red.

---

## Sampling Rate

- **After every task commit:** module-scoped `cargo test -p <crate> --lib <module>` for whatever was touched (house convention, used throughout Phase 117).
- **After every plan wave:** `make test` (workspace-wide).
- **Before `/gsd:verify-work`:** `make ci` green, **plus both cross-target clippy gates** (D-23):
  - `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
  - `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`
  Both are required because promoting `LayerId` into `crates/nono` and adding the
  `nono receipt` family to `cli.rs` both touch files carrying Unix `cfg` branches.
  **No PARTIAL→CI** — a stopped daemon or absent tool does not qualify as a documented runner failure.
- **Max feedback latency:** < 60s at task granularity.

---

## Per-Task Verification Map

Task IDs are assigned by the planner; this table fixes the requirement → test-type →
command mapping each task must land in. `File Exists` reflects the tree **before** this phase.

| Plan area | Requirement | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|-----------|-------------|-----------------|-----------|-------------------|-------------|--------|
| Census completeness (all 3 binaries) | RCPT-01 | A receipt from a DirectCli / Broker / Daemon session names all 13 `LayerId` rows; none silently dropped | unit + discovery-based meta-test | `cargo test -p nono-cli --test layer_registry_meta_test` (extend) + new `cargo test -p nono-cli --test receipt_census_test` | ❌ W0 | ⬜ pending |
| Content-free receipt | RCPT-01 | No path / argument / payload bytes anywhere in a serialized receipt | type-allowlist source scan **+** sentinel round-trip (both, each perturbation-proofed) | new `cargo test -p nono-cli --test receipt_content_free_scan` | ❌ W0 | ⬜ pending |
| Chain construction (keyless, D-25) | RCPT-02 | `SHA256(receipt_domain \|\| prev \|\| leaf_hash)`; own chain domain (D-11 amended) | unit | `cargo test -p nono --lib receipt_chain` | ❌ W0 | ⬜ pending |
| `nono receipt verify` | RCPT-02 | Fail-closed recompute-and-compare; an edited receipt is detected | integration | `cargo test -p nono-cli receipt_verify` | ❌ W0 | ⬜ pending |
| Four-state rendering | RCPT-03 | All four states render distinguishably; an unattested layer **never** renders as attested | unit + discovery-based | new `cargo test -p nono-cli receipt_vocabulary_rendering` | ❌ W0 | ⬜ pending |
| Daemon restructure (D-26) | RCPT-01 | `daemon_attest_and_decide` probes every modelled layer; **decision outcome provably unchanged** for every input | equivalence unit test (perturbation-proofed) | `cargo test -p nono-cli --lib agent_daemon::launch` | ✅ (extend) | ⬜ pending |
| Broker wire contract (D-27) | RCPT-01 | Broker receives `(EntryPath::Broker)` NotApplicable rows from nono-cli; unrecognised/absent row set fails toward **unconfirmed**, never toward a short census | unit + cross-binary contract test | `cargo test -p nono-shell-broker` + `cargo test -p nono-cli receipt_broker_wire` | ❌ W0 | ⬜ pending |
| Emitter degrade posture (D-04) | RCPT-01 | Emitter failure degrades **visibly** (`TelemetryDegraded` + operator banner), never silently; `HKLM\SOFTWARE\Policies\nono` can make it fail-closed | unit | `cargo test -p nono --lib machine_policy` (extend) | ✅ (extend) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono-cli/tests/receipt_content_free_scan.rs` — D-14's type-allowlist scan **and** sentinel round-trip, in the house source-scan idiom (`env!("CARGO_MANIFEST_DIR")` + `fs::read_to_string`; **no `regex`, no `include_str!`**; discovery-based, never naming its targets)
- [ ] `crates/nono-cli/tests/receipt_census_test.rs` — 13-row completeness across all three entry paths
- [ ] Extend `crates/nono-cli/tests/layer_registry_meta_test.rs` — census ↔ registry drift guard: **adding a `LayerId` variant without a census row must fail the build, not a review**
- [ ] Reuse `crates/nono-cli/tests/layer_force_unavailable.rs`'s per-layer seam to produce non-`Confirmed` rows without mocking the OS

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
2. **Sentinel round-trip (D-14).** Build a receipt from a session seeded with sentinel
   path / argument / env values; assert those exact bytes are absent from the serialized output.
   Negative control: leaving a path field in must make the test fail (demonstrated in review, not committed).
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
6. **Broker wire-contract guard (D-27).** Adding an expectancy cell to `layer_registry.rs` for
   `(EntryPath::Broker, …)` without it reaching the broker over the wire must fail the
   cross-check. Separately: a broker handed an unrecognised or absent row set must produce
   `Unconfirmed` rows, never a silently short census — assert this, do not assume it.

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

- [ ] All tasks have an automated verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all ❌ references above
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s at task granularity
- [ ] Every discovery/self-enforcing test carries its perturbation proof
- [ ] All three audit questions answered per guard
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
