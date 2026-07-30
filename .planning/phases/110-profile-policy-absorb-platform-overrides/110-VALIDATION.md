---
phase: 110
slug: profile-policy-absorb-platform-overrides
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-30
---

# Phase 110 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `110-RESEARCH.md` §"Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`); `proptest` already used in `nono-cli` |
| **Config file** | none — standard `#[cfg(test)] mod tests` per source file, matching every file this phase touches |
| **Quick run command** | `cargo test -p nono -p nono-cli --lib` |
| **Full suite command** | `make ci` (clippy + fmt + tests, per CLAUDE.md) |
| **Estimated runtime** | quick ~60–90s; `make ci` several minutes |

**Cross-target gates (D-12, MANDATORY — not optional for this phase):**
```
cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used
cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used
```
Both are local-runnable on this dev host per `.planning/templates/cross-target-verify-checklist.md`.
This phase touches `crates/nono/src/sandbox/macos.rs`, `crates/nono/src/sandbox/linux.rs`, and
`crates/nono-cli/src/exec_strategy/supervisor_linux.rs` — all in cfg-gated-Unix scope.
**No PARTIAL→CI fallback** absent a documented runner failure.

---

## Sampling Rate

- **After every task commit:** `cargo test -p nono -p nono-cli --lib`
- **After every plan wave:** `make ci`
- **Before `/gsd:verify-work`:** `make ci` green **AND** both cross-target clippy gates green
- **Max feedback latency:** ~90 seconds (quick run)

---

## Per-Requirement Verification Map

Task-level rows are filled in by the plans (see `## Wave 0 Requirements` for the gaps plans must close).

| Req | Behavior proven | Threat Ref | Test Type | Automated Command | Test Exists? |
|-----|-----------------|------------|-----------|-------------------|--------------|
| PROF-01 | `platform_overrides` parses; applies current-OS block only; survives `extends` resolution; `windows_low_il_broker`/`windows_interpreters` accepted in BOTH forms with new-form-wins precedence | override silently widening scope (EoP) | unit | `cargo test -p nono-cli profile::mod::tests -- platform_overrides` | ✅ port upstream's `platform_overrides_*` suite from `ae1c513e`/`719975cf` (~10 tests, directly reusable) |
| PROF-02a | `$VAR` expands from process env in `filesystem.allow`/`read`/`write` | input validation (V5) | unit | `cargo test -p nono-cli capability_ext::tests -- expand_env_var` | ✅ port `test_profile_fs_allow_expands_env_var` from `2cbaa9a0` |
| PROF-02b | `@git:*` tokens expand in top-level filesystem lists on all 3 platforms; non-Unix cfg fallback compiles and returns pass-through | hostile per-repo `.git/config` path injection (Tampering) — `local`/`worktree` scopes MUST stay excluded | unit | `cargo test -p nono-cli -- dynamic_tokens` (module name = planner's choice) | ✅ port `dynamic_providers.rs`'s existing ~35-test suite wholesale, **including** `git_read_paths_excludes_per_repo_local_config_overrides` — do not modify it |
| PROF-03a | `merge_port_ranges` correctness (empty / no-overlap / overlap / adjacent / contained / unsorted / 3-way) | — | unit | `cargo test -p nono capability::tests -- merge_port_ranges` | ✅ port upstream's 6 tests verbatim |
| PROF-03b | macOS emits per-port Seatbelt rules; errors above the cumulative 16,384 limit | Seatbelt compiler SIGILL (local DoS) | unit | `cargo test -p nono sandbox::macos::tests -- port_range` | ✅ port upstream's 6 macOS tests |
| PROF-03c | Linux Landlock adds `NetPort` rules per range, no cap (full 1–65535) | — | unit | `cargo test -p nono sandbox::linux::tests -- port_range` | ❌ **Wave 0 gap** — upstream extended the ruleset loop but added no dedicated `linux.rs` unit test |
| PROF-03d | Windows filter-spec construction emits a `FWP_MATCH_RANGE` condition per range with **no per-port unroll** and no macOS-style cap | `valueLow`/`valueHigh` inversion widening the matched span (EoP) | unit | `cargo test` against the `nono-wfp-service` spec-builder test module | ❌ **Wave 0 gap** — fork-original code (D-07), no upstream test to port; follow the existing `localhost_ports: vec![8080]` fixture pattern at `nono-wfp-service.rs:1829-1936` |
| PROF-03e | Live kernel `FwpmFilterAdd0` accepts a range filter **and it is actually enforced** | — | manual / host-gated | none automatable from this dev host | N/A — requires Administrator + live `nono-wfp-service`; record as `checkpoint:human-verify` |
| PROF-03f | Discrete-`Vec<u16>` back-compat: existing profiles using discrete port lists still work (SC3) | silent regression of existing enforcement | unit | existing `localhost_ports` tests must remain green, unmodified | ✅ existing coverage — **must not be edited** to accommodate ranges |
| PROF-04 | `bun`/`mise` presets are schema-valid **and resolvable** — loaded and resolved *by name*, not merely present in JSON (D-11) | — | unit | `cargo test -p nono-cli -- manifest_roundtrip` + a new "resolve `bun-dev`/`mise-dev` by name" test | ❌ **Wave 0 gap** — `test_schema_validates_builtin_profiles_in_policy_json` proves schema conformance only, not resolvability |

---

## Wave 0 Requirements

Test infrastructure itself is fully present — no test-runner or fixture install needed. These are
missing-coverage gaps the plans must close:

- [ ] `crates/nono/src/sandbox/linux.rs` — no existing `#[test]` coverage for the new port-range Landlock rules; add alongside the ported macOS tests (PROF-03c)
- [ ] `crates/nono-cli/src/bin/nono-wfp-service.rs` — new `PortCondition::RemoteRange`/`LocalRange` construction needs fresh unit tests; fork-original, no upstream precedent (PROF-03d)
- [ ] `crates/nono-cli/src/profile/mod.rs` (or a schema-test module) — explicit schema-validation-**plus-resolution** tests for `platform_overrides` / `open_port_range` / `listen_port_range`, beyond the built-in-profiles iteration test. Required because `nono-profile.schema.json` is `additionalProperties: false` — a new serde field that is not added to the JSON schema parses but fails validation
- [ ] `crates/nono-cli/src/profile/mod.rs` — "resolve `bun-dev`/`mise-dev` by name" tests, distinct from schema-shape validation (PROF-04, D-11)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live WFP kernel accepts and enforces a remote-port-**range** filter | PROF-03 / SC3 (kernel arm) | Requires Administrator + a live `nono-wfp-service`; `FwpmFilterAdd0` cannot be exercised from a non-elevated dev-host unit test | Per the established WFP-01 dark-gate procedure: admin shell + fresh `nono-wfp-service`, non-elevated daemon (`runas /trustlevel:0x20000`), then confirm a connect inside the declared range succeeds and one outside it is blocked. Record as `checkpoint:human-verify`. |
| Bindings rebuild after `capability.rs` field addition (D-13) | PROF-03 | Sibling repos, separate toolchains — not reachable from `cargo test` in this workspace | `maturin build` in `../nono-py`; `napi build --platform --release` in `../nono-ts`. Phase 109 hit `E0063` here; research reports the new field is additive on a `Default`-deriving struct, so drift is expected to be low — **verify, don't assume**. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all 4 MISSING references above
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s for the quick run
- [ ] Both cross-target clippy gates green (D-12) — recorded, not assumed
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
