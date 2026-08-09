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
> Derived from `117-RESEARCH.md` § Validation Architecture.

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
- **After every plan wave:** `cargo test -p nono-cli --features <fault-injection-feature>` (full crate,
  with the D-30 seam feature ON so the forced-unavailable tests actually execute) **plus**
  `cargo test -p nono --lib` (library-side diagnostic-code / error additions).
- **Before `/gsd:verify-work`:** `make ci` green, **plus** both local cross-target clippy gates
  (`cross` linux-gnu + `cargo-zigbuild` apple-darwin) for every file flagged cross-target in
  `117-RESEARCH.md` §I — per CLAUDE.md this is a MUST, no PARTIAL→CI.
- **Max feedback latency:** 60 seconds for the per-task quick run.

**Feature-flag sampling hazard (D-30).** The forced-unavailable tests only run when the fault-injection
feature is enabled. A default-feature `cargo test` will pass while every CINT-03 test is compiled out —
which reads as green-by-absence, the exact Phase 115 V-01 failure class. Any wave that adds a
forced-unavailable test MUST run the featured command, and the phase gate must prove the featured suite ran.

---

## Per-Task Verification Map

*Populated by the planner — one row per task, filled from each PLAN.md's `<automated>` verify blocks.*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| — | — | — | CINT-01 | — | Registry compiles; every row's cited call site resolves to a real current symbol | unit | `cargo test -p nono-cli registry_call_sites_exist` | ❌ W0 | ⬜ pending |
| — | — | — | CINT-01 | — | SPEC and registry cannot silently diverge (D-01) | unit / source-scan | `cargo test -p nono-cli spec_matches_registry` | ❌ W0 | ⬜ pending |
| — | — | — | CINT-02 | — | A forced-unavailable layer aborts or visibly downgrades — never a silent "enforcing" claim | integration | `cargo test -p nono-cli --features <fi> attestation_downgrades_on_forced_unavailable` | ❌ W0 | ⬜ pending |
| — | — | — | CINT-02 | — | Downgrade reaches all three D-27 channels (banner, diagnostic code, telemetry event) | unit ×3 | `cargo test -p nono-cli downgrade_reaches_` | ❌ W0 | ⬜ pending |
| — | — | — | CINT-02 | — | D-28 holds: layer-specific detail never lands on a channel the confined child can read | unit | `cargo test -p nono-cli child_visible_downgrade_is_coarse` | ❌ W0 | ⬜ pending |
| — | — | — | CINT-03 | — | Every registry row has a forced-unavailable test (discovery-based, D-32) | unit | `cargo test -p nono-cli every_registry_row_has_a_test` | ❌ W0 | ⬜ pending |
| — | — | — | CINT-03 | — | Host-gated rows are a loud named gap, not a silent skip (D-31) | unit | `cargo test -p nono-cli host_gated_rows_are_loud` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**D-32 meta-test is the keystone.** It is the only check that fails when a *future* layer is added without a
test. It must DISCOVER rows from the registry, never assert against a hand-written list of names — a test
that names its targets is blind by construction (Phase 115 V-01).

---

## Wave 0 Requirements

Every automated command above targets a test that does not exist yet. Wave 0 must land:

- [ ] The layer registry itself (`crates/nono-cli`, D-02) — nothing else is testable until rows exist
- [ ] The fault-injection feature in `crates/nono-cli/Cargo.toml` `[features]`, modelled on the shipped
      `test-trust-overrides` precedent (`Cargo.toml:44`) per RESEARCH §E
- [ ] A test module or `tests/*.rs` file for the registry self-checks, following the house
      `CARGO_MANIFEST_DIR` / `include_str!` source-scan pattern (RESEARCH §H)
- [ ] Resolution of RESEARCH Open Question 2 (`FirewallRulesNetworkBackend` reachability) — it determines
      whether the network layer is one contract row or two

*Framework install: none needed — Cargo test runner is already in use across the workspace.*

---

## Manual-Only Verifications

Per D-31, each of these is a **named, loud gap** recorded in the contract — never a silent `#[ignore]`.

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| WFP forced-unavailable end-to-end under a live `nono-wfp-service` | CINT-02, CINT-03 | Requires the elevated service installed and running; per-SID WFP is daemon-path only (`nono agent launch`), not direct `nono run` | Install/start `nono-wfp-service` as admin, run a confined daemon session from a **non-elevated** shell with the WFP layer forced unavailable, assert the contracted outcome (abort vs. downgrade) matches that registry row |
| Minifilter *absence* row | CINT-01, CINT-03 | **Structurally untestable — no driver exists.** ADR-65 stands | Not a skip: record as an explicit "structurally absent / not applicable" entry citing ADR-65, following Phase 116 D-08's `structurally-blocked` row form |
| Daemon-path (`nono-agentd`) forced-unavailable | CINT-02, CINT-03 | **UNRESOLVED** — `agent_daemon/launch.rs` is deliberately independent of `exec_strategy_windows/`; whether its test harness can drive forced-unavailable without admin/live-daemon is unverified | Wave 0 must determine this. If it needs a live daemon, promote to this table with steps; if not, move it to the automated map |
| Broker-arm confinement attestation | CINT-02 | The real Low-IL child is spawned *inside* `nono-shell-broker`, not at nono-cli's spawn site (RESEARCH §Summary-2). Broker runs fail GLE=87 under git-bash/MSYS | Run from **PowerShell**, not git-bash. A dev-layout install skips the broker trust gate; an unsigned Program Files install cannot spawn the broker at all (fail-secure) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all ❌ MISSING references above
- [ ] No watch-mode flags
- [ ] Featured suite (`--features <fi>`) proven to have RUN, not merely passed — see feature-flag hazard above
- [ ] Both cross-target clippy gates run locally for every cross-target-flagged file (no PARTIAL→CI)
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
