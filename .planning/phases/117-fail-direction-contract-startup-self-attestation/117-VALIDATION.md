---
phase: 117
slug: fail-direction-contract-startup-self-attestation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-09
---

# Phase 117 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `117-RESEARCH.md` § Validation Architecture, and updated (checker pass 2,
> 2026-08-09 — Warning-9) from the finished 12-plan set's actual `<automated>` verify commands.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`). Windows-only assertions live in `#[cfg(all(test, target_os = "windows"))]` modules; cross-process behaviour lives in `crates/nono-cli/tests/*.rs` integration tests that spawn `env!("CARGO_BIN_EXE_nono")` |
| **Config file** | none — discovery is Cargo-native (`tests/*.rs` implicit `[[test]]`, in-crate `#[test]` fns) |
| **Quick run command** | `cargo test -p nono-cli --lib exec_strategy_windows -- --test-threads=1` |
| **Full suite command** | `make test` (workspace) — phase gate is `make ci` (clippy + fmt + tests) |
| **Estimated runtime** | quick ~30s; `make ci` several minutes |

**`--test-threads=1` is not optional for the Windows unit modules.** CLAUDE.md's env-var save/restore
rule exists because Rust runs unit tests in-process and in parallel; the layer-force-unavailable seam is
process-global state by construction, so parallel execution would make these tests flaky against each other.

**Known-baseline caveat:** `-p nono-cli` and `-p nono` have pre-existing failures on this Windows host
(11 at last count, recorded in project memory). Compare against the phase-base commit before treating any
red as a Phase 117 regression.

---

## Sampling Rate

- **After every task commit:** the `#[cfg(all(test, target_os = "windows"))]` unit module for the file
  just touched (fast, single-file scope — matches the existing convention throughout `exec_strategy_windows/`).
- **After every plan wave:** `cargo test -p nono-cli --features layer-fault-injection` (full crate,
  with the D-30 seam feature ON so the forced-unavailable tests actually execute) **plus**
  `cargo test -p nono --lib` (library-side diagnostic-code / error additions).
- **Before `/gsd:verify-work`:** `make ci` green, **plus** both local cross-target clippy gates
  (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) for the full workspace — per CLAUDE.md this is
  a MUST, no PARTIAL→CI.
- **Max feedback latency:** 60 seconds for the per-task quick run.

**Feature-flag sampling hazard (D-30).** The forced-unavailable tests only run when the
`layer-fault-injection` feature is enabled (the concrete feature name, landed by Plan 04). A
default-feature `cargo test` will pass while every CINT-03 test is compiled out — which reads as
green-by-absence, the exact Phase 115 V-01 failure class. Any wave that adds a forced-unavailable
test MUST run the featured command, and the phase gate must prove the featured suite ran.

---

## Per-Task Verification Map

*Populated from each PLAN.md's actual `<acceptance_criteria>`/verify content (checker pass 2 —
Warning-9 fix). One row per task across the finished 12-plan set.*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-T1 | 117-01 | 1 | CINT-01 | T-117-07 | D-10 in/out derivation + Blocker-1 note recorded in module doc comment | source-scan (manual read, no runtime test) | `grep -c "SC4-3" crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` | ❌ W1 | ⬜ pending |
| 01-T2 | 117-01 | 1 | CINT-01 | T-117-21 | `LayerId`/registry populated; `AppContainerProfile` expectancy excludes `EntryPath::DirectCli` | unit (build + grep) | `cargo build -p nono-cli` | ❌ W1 | ⬜ pending |
| 01-T3 | 117-01 | 1 | CINT-01 | T-117-06 | Exhaustive drift-guard match, no wildcard; module wired | unit | `cargo clippy -p nono-cli -- -D warnings -D clippy::unwrap_used` | ❌ W1 | ⬜ pending |
| 02-T1 | 117-02 | 1 | CINT-02 | — | `NonoDiagnosticCode::LayerAttestationFailed` + `NonoError` variant, exhaustive match preserved | unit | `cargo test -p nono --lib diagnostic_code` | ❌ W1 | ⬜ pending |
| 02-T2 | 117-02 | 1 | CINT-02 | T-117-08, T-117-09 | `RequiredLayersPolicy` is `pub` (Blocker-2), excluded from `is_unconfigured()` | unit | `cargo test -p nono --lib machine_policy` | ❌ W1 | ⬜ pending |
| 03-T1 | 117-03 | 2 | CINT-01 | T-117-10 | SPEC authored: 13 rows, D-12/D-20/D-24/D-15 sections, 5 discrepancy rows with D-16 evidence | source-scan (manual read) | `grep -c "^| SC4-" proj/SPEC-windows-fail-direction-contract.md` | ❌ W2 | ⬜ pending |
| 03-T2 | 117-03 | 2 | CINT-01 | T-117-06 | Registry call sites exist; SPEC cannot silently diverge from registry (D-01) | unit / source-scan | `cargo test -p nono-cli --test layer_registry_selfcheck` | ❌ W2 | ⬜ pending |
| 04-T1 | 117-04 | 2 | CINT-03 | T-117-01 | `layer-fault-injection` feature added, default-off | unit (build) | `cargo build -p nono-cli --features layer-fault-injection` | ❌ W2 | ⬜ pending |
| 04-T2 | 117-04 | 2 | CINT-03 | T-117-01, T-117-11 | WFP toggle migrated off `NONO_TEST_HARNESS` (SC4-4 closed) | unit + grep | `grep -c "NONO_TEST_HARNESS" crates/nono-cli/src/exec_strategy_windows/mod.rs` (expect 0) | ❌ W2 | ⬜ pending |
| 05-T1 | 117-05 | 2 | CINT-02 | — | `LayerAttestationStatus` + `ProcessHandle` platform-neutral (Warning-6) | unit (build) | `cargo build -p nono` | ❌ W2 | ⬜ pending |
| 05-T2 | 117-05 | 2 | CINT-02 | T-117-02, T-117-12 | Four raw probes over `ProcessHandle`, `Err` on OS-call failure, never accept child self-report (D-19) | unit | `cargo test -p nono --lib attestation -- --test-threads=1` | ❌ W2 | ⬜ pending |
| 06-T1 | 117-06 | 3 | CINT-03 | T-117-01, T-117-13 | Restricted-token/label force-unavailable hooks, compiled-out by default | unit (featured) | `cargo test -p nono-cli --features layer-fault-injection --lib exec_strategy_windows::restricted_token exec_strategy_windows::labels_guard -- --test-threads=1` | ❌ W3 | ⬜ pending |
| 06-T2 | 117-06 | 3 | CINT-03 | T-117-01, T-117-13 | DACL/Job-Object force-unavailable hooks | unit (featured) | `cargo test -p nono-cli --features layer-fault-injection --lib exec_strategy_windows::dacl_guard exec_strategy_windows::launch -- --test-threads=1` | ❌ W3 | ⬜ pending |
| 07-T1 | 117-07 | 3 | CINT-03 | T-117-01 | New `[features]` block in `nono-shell-broker/Cargo.toml` | unit (build) | `cargo build -p nono-shell-broker --features layer-fault-injection` | ❌ W3 | ⬜ pending |
| 07-T2 | 117-07 | 3 | CINT-03 | T-117-01, T-117-14 | AppContainer force-unavailable hooks (broker + daemon) | unit (build, both crates) | `cargo build -p nono-cli --features layer-fault-injection` | ❌ W3 | ⬜ pending |
| 08-T1 | 117-08 | 3 | CINT-02 | T-117-21 | `attest_and_decide()`: 4-state classification, `EntryPath::DirectCli` never probes `AppContainerProfile` (Blocker-1) | unit (featured) | `cargo test -p nono-cli --lib exec_strategy_windows::attestation -- --test-threads=1` | ❌ W3 | ⬜ pending |
| 08-T2 | 117-08 | 3 | CINT-02 | T-117-15 | `required_layers_for_broker()` filters on `EntryPath::Broker` only; module wired | unit (build + grep) | `grep -c "BROKER_REQUIRED_LAYERS_ENV_VAR" crates/nono-cli/src/exec_strategy_windows/attestation.rs` | ❌ W3 | ⬜ pending |
| 09-T1 | 117-09 | 3 | CINT-02 | T-117-17 | `LayerAttestationDowngraded` event + `downgraded_layers` field; Blocker-3 follow-up named in doc comment | unit | `cargo test -p nono-cli --lib telemetry -- --test-threads=1` | ❌ W3 | ⬜ pending |
| 09-T2 | 117-09 | 3 | CINT-02 | T-117-04, T-117-22 | Coarse banner, no `LayerId` names, not suppressible by `--silent` (Warning-8) | unit + grep | `grep -c "LayerId" crates/nono-cli/src/output.rs` (expect 0) | ❌ W3 | ⬜ pending |
| 10-T1 | 117-10 | 4 | CINT-02 | T-117-05, T-117-18, T-117-21 | D-21 gate in `launch.rs`; WFP-preconfirmed threaded via `network_enforcement` param (Warning-5); SC4-2 comment fixed | integration (featured) | `cargo test -p nono-cli --features layer-fault-injection --lib exec_strategy_windows::launch -- --test-threads=1` | ❌ W4 | ⬜ pending |
| 10-T2 | 117-10 | 4 | CINT-02 | T-117-05 | D-21 gate in `agent_daemon/launch.rs`, same idiom, independent implementation | integration (featured) | `cargo test -p nono-cli --features layer-fault-injection --lib agent_daemon::launch -- --test-threads=1` | ❌ W4 | ⬜ pending |
| 11-T1 | 117-11 | 4 | CINT-02 | T-117-05, T-117-02, T-117-21, T-117-23 | Broker's own D-21 gate; genuinely attests `AppContainerProfile` (Blocker-1 closed); env-var save/restore in tests (Warning-10) | integration (featured) | `cargo test -p nono-shell-broker --features layer-fault-injection -- --test-threads=1` | ❌ W4 | ⬜ pending |
| 12-T1 | 117-12 | 5 | CINT-03 | T-117-01 | Forced-unavailable test per automatable `LayerId` row, asserting the row's exact `ContractOutcome` | integration (featured) | `cargo test -p nono-cli --features layer-fault-injection --test layer_force_unavailable -- --test-threads=1` | ❌ W5 | ⬜ pending |
| 12-T2 | 117-12 | 5 | CINT-03 | T-117-19, T-117-20, T-117-17 | D-32 discovery meta-test; D-31 loud-gap list; ETW/AppLog assumption named (Blocker-3) | unit (featured, source-scan) | `cargo test -p nono-cli --features layer-fault-injection --test layer_registry_meta_test -- --test-threads=1` | ❌ W5 | ⬜ pending |
| 12-T3 | 117-12 | 5 | CINT-01, CINT-02, CINT-03 | — | Both cross-target clippy gates green; D-24 latency recorded; SC4 rows all closed; full phase gate | command (manual invocation + `make ci`) | `make ci` | ❌ W5 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**D-32 meta-test is the keystone.** It is the only check that fails when a *future* layer is added without a
test. It must DISCOVER rows from the registry, never assert against a hand-written list of names — a test
that names its targets is blind by construction (Phase 115 V-01).

---

## Early-Wave Requirements (Wave 1-2)

*(Renamed from "Wave 0 Requirements" — checker pass 2, Warning-9: the phase has no Wave 0; the
registry lands in Wave 1 and its self-check test lands in Wave 2, per the finished 12-plan wave
assignment, not a pre-Wave-1 scaffold.)*

Every automated command in the table above targets a test that does not exist until its plan lands.
Waves 1-2 land the prerequisites nothing else can be tested without:

- [ ] The layer registry itself (`crates/nono-cli`, D-02) — Plan 117-01, Wave 1
- [ ] `NonoDiagnosticCode::LayerAttestationFailed` + `RequiredLayersPolicy` (`pub`, Blocker-2) — Plan 117-02, Wave 1
- [ ] The registry self-check + SPEC drift-check test — Plan 117-03, Wave 2 (not Wave 0)
- [ ] The `layer-fault-injection` Cargo feature, modelled on the shipped `test-trust-overrides`
      precedent (`Cargo.toml:44`) per RESEARCH §E — Plan 117-04, Wave 2
- [ ] The shared `crates/nono` attestation primitives (`LayerAttestationStatus`, `ProcessHandle`,
      four raw probes) — Plan 117-05, Wave 2
- [ ] Resolution of RESEARCH Open Question 2 (`FirewallRulesNetworkBackend` reachability) — resolved
      in Plan 117-01 Task 1: reachable, becomes its own `LayerId::FirewallRulesEgress` row

*Framework install: none needed — Cargo test runner is already in use across the workspace.*

---

## Manual-Only Verifications

Per D-31, each of these is a **named, loud gap** recorded in the contract — never a silent `#[ignore]`.
Split into two categories (checker pass 2, Blocker-3): `LayerId`-scoped rows lacking an automatable
forced-unavailable test, and cross-cutting security assumptions that are not tied to one registry row.

### LayerId-scoped (Plan 117-12 Task 2's `MANUALLY_VERIFIED` list)

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| WFP forced-unavailable end-to-end under a live `nono-wfp-service` | CINT-02, CINT-03 | Requires the elevated service installed and running; per-SID WFP is daemon-path only (`nono agent launch`), not direct `nono run` | Install/start `nono-wfp-service` as admin, run a confined daemon session from a **non-elevated** shell with the WFP layer forced unavailable, assert the contracted outcome (abort vs. degrade) matches that registry row |
| Minifilter *absence* row | CINT-01, CINT-03 | **Structurally untestable — no driver exists.** ADR-65 stands | Not a skip: record as an explicit "structurally absent / not applicable" entry citing ADR-65, following Phase 116 D-08's `structurally-blocked` row form |
| Daemon-path (`nono-agentd`) forced-unavailable | CINT-02, CINT-03 | Plan 117-12 Task 2 re-checks this against what Plans 06/07/10 actually shipped; if the daemon-side hook/gate runs cleanly in ordinary `cargo test`, it is REMOVED from this list and moved to the automated map (`12-T1` row above) | If still host-gated after Plan 117-12 lands: needs a live/elevated `nono-agentd` process — steps to be filled in by that plan's own finding |
| Broker-arm confinement attestation | CINT-02 | The real Low-IL child is spawned *inside* `nono-shell-broker`, not at nono-cli's spawn site (RESEARCH §Summary-2). Broker runs fail GLE=87 under git-bash/MSYS | Run from **PowerShell**, not git-bash. A dev-layout install skips the broker trust gate; an unsigned Program Files install cannot spawn the broker at all (fail-secure) |

### Cross-cutting security assumptions (Plan 117-12 Task 2's `MANUAL_SECURITY_ASSUMPTIONS` list — Blocker-3 fix)

| Assumption | Requirement | Why Manual | Test Instructions |
|------------|-------------|------------|--------------------|
| ETW / Application Event Log is unreadable by a Low-IL/AppContainer confined child | CINT-02 (D-28) | Plan 117-09 adds `SecurityEvent.downgraded_layers` (specific `LayerId` names) on this documented-but-unverified assumption; verifying it requires either a live Windows Event Log ACL inspection or a real low-IL child attempting the read — not safely assertable from memory without live verification, and not worth risking a wrong "safe" claim baked into an automated test | Run `wevtutil gl Application` (or `EvtGetChannelConfigProperty`/`EvtChannelConfigPropertyAccess`) and inspect the returned `channelAccess` SDDL string for any ACE granting generic-read (`0x1`) to `S-1-1-0` (Everyone) or any low-integrity/AppContainer-reachable SID. If found, `SecurityEvent.downgraded_layers` must move to coarse wording (matching the banner) and this becomes a tracked SC4-6 discrepancy, not merely a manual note. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or an Early-Wave dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Early-Wave items (Wave 1-2) cover all ❌ MISSING references above
- [ ] No watch-mode flags
- [ ] Featured suite (`--features layer-fault-injection`) proven to have RUN, not merely passed — see feature-flag hazard above
- [ ] Both cross-target clippy gates run locally against the full workspace (no PARTIAL→CI)
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
