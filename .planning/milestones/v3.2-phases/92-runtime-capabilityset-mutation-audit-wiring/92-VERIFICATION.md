---
phase: 92-runtime-capabilityset-mutation-audit-wiring
verified: 2026-06-22T14:30:00Z
status: human_needed
score: 10/11
overrides_applied: 0
human_verification:
  - test: "Run verify-dark.ps1 --gate override-01 end-to-end on a live host with nono.exe on PATH, Python + nono_py installed, and openssl present"
    expected: "Exit 0, verdict PASS, all three SCs pass (SC1/SC2/SC3); no SKIP_HOST_UNAVAILABLE due to .bat stub failure"
    why_human: "The gate's SC1 and SC3 use a .bat stub to capture args from Rust's std::process::Command spawn. The gate has a host-gated SKIP_HOST_UNAVAILABLE branch for SC1/SC3 when the .bat stub cannot execute via std::process::Command. The SUMMARY reports PASS but only after the stub injection; an automated verifier cannot confirm the stub executed on this host without running the gate live."
  - test: "Run python -m pytest tests/test_override_wiring.py -v in nono-py after maturin develop"
    expected: "11 tests pass (MUT-01..05, AUD-04, SC2 x5). No SKIP due to Windows-only mark being hit on a non-Windows CI host."
    why_human: "All 11 tests are marked Windows-only (sys.platform == 'win32') and require the compiled nono_py extension. Automated verification cannot re-run pytest in this context."
  - test: "Confirm end-to-end: run confined_run with a real override token against a real nono.exe (build from HEAD) on Windows 10/11, then inspect that the sandboxed child's filesystem access is limited to the override grant scope plus the base profile — not wider"
    expected: "The child process can access only the granted path(s) from the override scope. OS confinement (AppContainer/WFP) is still active — MUT-03 behavioral proof."
    why_human: "MUT-03 (OS confinement remains active beneath override expansion) is architecturally guaranteed by the additive-only --allow flag path, but behavioral proof requires an elevated Win11 host with nono-wfp-service running. Cannot be automated without the live stack per windows_supervised_needs_real_console memory."
---

# Phase 92: Runtime CapabilitySet Mutation + Audit Wiring — Verification Report

**Phase Goal:** Wire the Phase 91 offline override verifier into the `confined_run`/`confine` execution path so a verified override additively expands the `CapabilitySet` for exactly one invocation, every expansion emits `AuditEventPayload::PolicyOverrideApplied` into the `SecurityEventLayer` HMAC chain before the child spawns, and any failure to emit aborts before spawn (AUD-04 fail-closed).

**Verified:** 2026-06-22T14:30:00Z
**Status:** human_needed (all automated checks VERIFIED; 3 behavioral items require live host)
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `AuditEventPayload::PolicyOverrideApplied` variant exists in `audit.rs` with correct fields (jti, kms_key_id, zt_audit_hash Option skip-serializing-if-none, granted_path_hashes, expires_at) | VERIFIED | `crates/nono/src/audit.rs:116-130`; serde tag `"type":"policy_override_applied"` confirmed by inline test at line 1869; `zt_audit_hash` absent from JSON when None confirmed at line 1901 |
| 2 | EventID constants 10006-10010 and 5 new `SecurityEventType` PolicyOverride* variants exist; `event_id_for` and `severity_for` are exhaustive | VERIFIED | `crates/nono-cli/src/telemetry/event.rs:48-110`; all 5 constants at lines 48-56; all 5 variants at lines 81-90; exhaustive match at lines 98-110; `severity_for` extended at `mod.rs:170-181`; inline tests `override_event_ids_are_10006_through_10010` + serde roundtrip present |
| 3 | `OverrideGrant` has `zt_audit_hash` field and `#[getter]`; `verify_override_impl` populates it from `token.current_hash` (D-06 TOCTOU closure) | VERIFIED | `nono-py/src/override.rs:682-715`; field at line 688; getter at line 713; construction site at line 813 (`zt_audit_hash: token.current_hash.clone()`); `#[expect(dead_code)]` removed from `OutOfScope` at lines 67-71 |
| 4 | `sanitize_override_path` uses `Path::components()` / `Component::ParentDir` — never string `starts_with` (MUT-04) | VERIFIED | `nono-py/src/windows_confined_run.rs:273-294`; line 288 shows `if component == std::path::Component::ParentDir`; doc comment at line 271 explicitly states "never string starts_with (CLAUDE.md §Path Handling footgun; MUT-04)" |
| 5 | `append_override_args` appends `--allow` per scope path and `--override-audit <base64url-json>` with all 5 audit metadata fields; reads from verified grant only (D-06) | VERIFIED | `nono-py/src/windows_confined_run.rs:297-329`; calls `sanitize_override_path` per path at line 313; `serde_json::json!({zt_audit_hash, kms_key_id, jti, granted_paths, expires_at})` at lines 318-324; base64url-no-pad encode at line 325; `cmd.arg("--override-audit")` at line 327 |
| 6 | `probe_override_support` is called BEFORE `append_override_args` in both `confined_run` and `confine`; raises `NonoOverrideError` on version < 3.2; result cached by path (D-02 nono-py side) | VERIFIED | `confined_run` hook: `windows_confined_run.rs:397-406`; `confine` hook: `windows_confined_run.rs:505-515`; probe at line 401 precedes append at line 402 in both; `PROBE_CACHE: LazyLock<Mutex<HashMap<PathBuf, bool>>>` at line 35; `NonoOverrideError` raised at line 198 |
| 7 | MUT-05 regression: `confined_run`/`confine` called with `override_token=None` produce no `--override-audit` flag (byte-identical baseline) | VERIFIED | `nono-py/src/windows_confined_run.rs:396-407`; `if let Some(ref grant) = override_token` guard at line 397 — the entire block including `probe_override_support` and `append_override_args` is skipped when `override_token` is `None`; Rust unit test `test_no_override_token_no_override_audit_flag` in the test module |
| 8 | nono-cli accepts `--override-audit <base64-json>` as a hidden `SandboxArgs` field; `OverrideAuditMeta` struct exists with `serde(deny_unknown_fields)`; field threaded through `ExecutionFlags` with DECODE-ONCE decode in `prepare_run_launch_plan` | VERIFIED | `crates/nono-cli/src/cli.rs:1634-1648` (`OverrideAuditMeta` + `deny_unknown_fields`); `SandboxArgs.override_audit` at line 2032; `ExecutionFlags.override_audit` at `launch_runtime.rs:227`; base64url decode in `prepare_run_launch_plan` at line 420-436 |
| 9 | `SecurityEventLayer::emit_override_event` exists with `#[must_use]`; returns `Err` on poisoned mutex (AUD-04 fail-closed); `SECURITY_LAYER: OnceLock<SecurityEventLayer>` set by `init_tracing` and `init_daemon_telemetry` | VERIFIED | `crates/nono-cli/src/telemetry/mod.rs:321-365`; `map_err(|_| "mutex poisoned")` at line 328; `SECURITY_LAYER` static at line 48; `cli_bootstrap.rs:127` sets it; `agent_daemon/telemetry_init.rs:75` sets it; `#[must_use = "AUD-04: ..."]` at lines 319-320 |
| 10 | AUD-04 pre-spawn gate in `execute_sandboxed`: placed AFTER `start_proxy_runtime`, BEFORE `apply_pre_fork_sandbox`; returns `Err(NonoError::SandboxInit(...))` if emit fails | VERIFIED | `crates/nono-cli/src/execution_runtime.rs:246-288`; `start_proxy_runtime` at line 242; gate block at lines 259-288; `apply_pre_fork_sandbox` at line 305; error message "override audit emission failed — aborting before spawn (AUD-04)" at line 282 |
| 11 | VFY-01 seam: offline arm wired; live POST /actions AND-gate composition point documented in code with `[BLOCKING-93]` comment; VFY-01 is PARTIAL by locked decision D-03 | VERIFIED (PARTIAL by design) | `nono-py/src/windows_confined_run.rs:404-406` and lines 513-515 both contain `// VFY-01 PARTIAL [BLOCKING-93]: Phase 92 wires the offline verify arm. Phase 93 adds the live POST /actions AND-gate here before confined_run is called (D-03 composition seam; VFY-01 clause b).` — seam is present and fail-closed (Phase 93 work required to complete clause b) |

**Score:** 10/11 truths VERIFIED (Truth 11 is PARTIAL by locked design decision D-03, not a failure); all 10 non-deferred truths VERIFIED

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/nono/src/audit.rs` | `PolicyOverrideApplied` variant in `AuditEventPayload` | VERIFIED | Variant at line 116; all 5 fields correct; 3 unit tests |
| `crates/nono-cli/src/telemetry/event.rs` | EventIDs 10006-10010; 5 `SecurityEventType` variants; `event_id_for` exhaustive | VERIFIED | Constants at lines 48-56; variants at lines 81-90; exhaustive match confirmed |
| `crates/nono-cli/src/telemetry/mod.rs` | `SECURITY_LAYER` OnceLock; `emit_override_event` method; `severity_for` extended | VERIFIED | Static at line 48; method at lines 287-365; `severity_for` extended at lines 170-181 |
| `crates/nono-cli/src/cli.rs` | `OverrideAuditMeta` struct; `--override-audit` flag | VERIFIED | Struct at lines 1628-1648 with `deny_unknown_fields`; SandboxArgs field at line 2032 |
| `crates/nono-cli/src/launch_runtime.rs` | `ExecutionFlags.override_audit`; DECODE-ONCE decode | VERIFIED | Field at line 227; decode in `prepare_run_launch_plan` at lines 420-436 |
| `crates/nono-cli/src/cli_bootstrap.rs` | `SECURITY_LAYER.set()` in `init_tracing` | VERIFIED | Line 127 |
| `crates/nono-cli/src/execution_runtime.rs` | AUD-04 pre-spawn gate | VERIFIED | Lines 246-288; correct placement between proxy and sandbox |
| `nono-py/src/override.rs` | `zt_audit_hash` field + getter; `OutOfScope` `#[expect(dead_code)]` removed | VERIFIED | Field at line 688; getter at line 713; construction site at line 813; no `#[expect(dead_code)]` on `OutOfScope` (lines 64-71 show doc comment only) |
| `nono-py/src/windows_confined_run.rs` | `probe_override_support`; `sanitize_override_path`; `append_override_args`; signature extensions; `PROBE_CACHE`; VFY-01 seam comments | VERIFIED | All 3 helpers present; `override_token` params in both functions; `PROBE_CACHE` at line 35; seam comments at lines 404-406 and 513-515 |
| `scripts/gates/override-01.ps1` | DF-01 gate following `Test-Precondition`/`Invoke-Gate` contract; no `exit`; no `Persist-Verdict` | VERIFIED | File exists; `Test-Precondition` at line 77; `Invoke-Gate` at line 117; all `exit` occurrences are in comments or Python `sys.exit(0)` inside inline Python (not PowerShell `exit`); `Persist-Verdict` occurrences are comments only |
| `nono-py/tests/test_override_wiring.py` | 11 pytest tests; no `subprocess.Popen` mock; strategy-A comment | VERIFIED | 11 `def test_` functions confirmed; `grep subprocess.Popen` = 0; strategy comment at lines 8-22 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/nono-cli/src/telemetry/event.rs` | `crates/nono-cli/src/telemetry/mod.rs` | `SecurityEventType` used in `severity_for` | VERIFIED | `severity_for` at `mod.rs:167-181` uses all 5 new `PolicyOverride*` variants |
| `crates/nono-cli/src/cli_bootstrap.rs::init_tracing` | `SECURITY_LAYER` | `OnceLock::set` after `SecurityEventLayer::new` | VERIFIED | `cli_bootstrap.rs:127` calls `SECURITY_LAYER.set(security_layer.clone())` |
| `crates/nono-cli/src/execution_runtime.rs::execute_sandboxed` | `SECURITY_LAYER` | `SECURITY_LAYER.get().emit_override_event(...)` | VERIFIED | Lines 262-287 |
| `nono-py/src/windows_confined_run.rs::append_override_args` | `nono-py/src/override.rs::OverrideGrant` | reads `zt_audit_hash()`, `signer`, `jti`, `expires_at`, `scope_paths` | VERIFIED | Lines 308-327; `grant.zt_audit_hash()` at line 319 |
| `nono-py/src/windows_confined_run.rs::sanitize_override_path` | `std::path::Component::ParentDir` | component-wise ParentDir rejection | VERIFIED | Line 288 |
| `nono-py/src/windows_confined_run.rs::confined_run` | `probe_override_support` | called before `append_override_args` | VERIFIED | Lines 401-402; probe precedes append in both `confined_run` and `confine` |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `emit_override_event` | `pre_chain_bytes` | `jti`, `kms_key_id`, `zt_audit_hash` from `OverrideAuditMeta` (decoded from `--override-audit` flag) | Yes — reads live call parameters, not hardcoded | FLOWING |
| `AuditEventPayload::PolicyOverrideApplied` | All fields | Passed from nono-py via `OverrideAuditMeta` decoded from base64url JSON | Yes — data carrier, no logic | FLOWING |
| `sanitize_override_path` | `scope_paths` | `OverrideGrant.scope_paths` from verified token | Yes — token paths, not hardcoded | FLOWING |
| `OverrideGrant.zt_audit_hash` | `token.current_hash` | Populated at `verify_override_impl` line 813 | Yes — sourced from parsed override token | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `PolicyOverrideApplied` serializes with type tag | `cargo test -p nono --lib audit::` — inline test `policy_override_applied_serializes_with_type_tag` | 15 tests pass per SUMMARY (commit `6ecc67b7`); type tag test asserts `"type":"policy_override_applied"` | VERIFIED by commit evidence |
| `event_id_for(PolicyOverrideVerified) == 10007` | `cargo test -p nono-cli --lib telemetry::event` | Tests green per SUMMARY (commit `fa15bf91`) | VERIFIED by commit evidence |
| `emit_override_event` advances chain by 1 on fresh layer | `cargo test -p nono-cli --lib telemetry::` | Tests green per SUMMARY (commit `6480d897`); test `emit_override_event_advances_chain_by_one` confirmed in code | VERIFIED by commit evidence |
| AUD-04: poisoned mutex returns Err | `emit_override_event_err_on_poisoned_mutex` inline test | Test confirmed at `mod.rs:790-813` using `std::panic::catch_unwind` to poison mutex; asserts `Err` | VERIFIED by code inspection |
| `sanitize_override_path` rejects `..` | Rust unit test `test_sanitize_override_path_rejects_parent_dir_windows_absolute` | Test at `windows_confined_run.rs:696-709` using `Component::ParentDir` | VERIFIED by code inspection |
| `override-01.ps1` follows contract | `pwsh -File scripts/verify-dark.ps1 --gate override-01` | SUMMARY claims exit 0 / PASS; cannot re-execute without live Python+nono_py+openssl | SKIP — requires human execution |

---

### Requirements Coverage

| Requirement | Phase Plan | Description | Status | Evidence |
|-------------|-----------|-------------|--------|----------|
| MUT-01 | 92-02 | Verified override additively expands CapabilitySet for the invocation | COVERED | `append_override_args` appends `--allow` flags from `grant.scope_paths`; additive only; no deny removal |
| MUT-02 | 92-02 | Override is invocation-scoped; no global mutation | COVERED | `override_token: Option<PyRef<'_, OverrideGrant>>` parameter per-call; `PROBE_CACHE` caches probes (not grants); no shared grant state |
| MUT-03 | 92-02 | Override cannot remove/weaken deny rules or bypass OS confinement | COVERED (architectural) | `append_override_args` only adds `--allow` flags; OS enforcement (AppContainer+WFP) remains active underneath; deny rules in nono-cli profile are unaffected; human verification needed for behavioral proof |
| MUT-04 | 92-02 | Path/DNS matching uses component comparison (never string `starts_with`) | COVERED | `sanitize_override_path` uses `Path::components()` + `Component::ParentDir`; nono-cli's existing capability matching (`capability.rs` component-wise) handles the enforcement boundary |
| MUT-05 | 92-02 | No-override path byte-for-byte identical to pre-v3.2 behavior | COVERED | `if let Some(ref grant) = override_token` guard ensures no flags appended when `None`; Rust unit test `test_no_override_token_no_override_audit_flag` + pytest `test_override_regression_no_token` |
| AUD-01 | 92-01, 92-03 | Override lifecycle events emitted into SecurityEventLayer HMAC chain | COVERED | `emit_override_event` at `mod.rs:321-365`; advances `advance_chain`; 4 inline tests confirm chain advance |
| AUD-02 | 92-01, 92-03 | Override events embed ZT-Infra `audit.current_hash` (bi-directional link) | COVERED | `zt_audit_hash: Option<String>` in `PolicyOverrideApplied` and `OverrideAuditMeta`; sourced from `token.current_hash` via `OverrideGrant.zt_audit_hash()` (D-06) |
| AUD-03 | 92-01 | EventIDs 10006-10010; redaction (no raw secrets; paths as hashes) | COVERED | EventID constants 10006-10010 in `event.rs`; `granted_path_hashes` (not raw paths) in `PolicyOverrideApplied`; `path_hash`, `path_category`, `host` all `None` in override events per `emit_override_event:352-357` |
| AUD-04 | 92-02, 92-03 | No silent privilege escalation — no-audit = no-apply | COVERED | Bilateral: nono-py side (`probe_override_support` raises `NonoOverrideError` before any flag if nono.exe < 3.2); nono-cli side (AUD-04 gate at `execution_runtime.rs:246-288` returns `Err` before spawn if `emit_override_event` fails); `#[must_use = "AUD-04: ..."]` enforces caller handling |
| VFY-01 | 92-02 (PARTIAL) | Two-key AND gate: KMS sig AND live ZT-Infra POST /actions | PARTIAL [BLOCKING-93] | Offline arm wired (Phase 91+92); seam for live arm present in both `confined_run` and `confine` with `// VFY-01 PARTIAL [BLOCKING-93]` comments; live POST /actions AND-gate is Phase 93 scope per locked decision D-03 |
| DF-01 | 92-04 | `verify-dark.ps1 --gate OVERRIDE-01` gate with machine-readable verdict | COVERED (human verification needed for live run) | `scripts/gates/override-01.ps1` exists; follows `Test-Precondition`/`Invoke-Gate` contract; SC1/SC2/SC3 assertions present; no `exit` or `Persist-Verdict` calls in PowerShell code; SUMMARY reports exit 0 PASS |

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/nono-cli/src/telemetry/mod.rs` | 318 | `#[allow(dead_code)]` on `emit_override_event` | INFO | Intentional: method is used in `execution_runtime.rs` (nono binary) but Rust dead_code lint fires for nono-agentd multi-binary artifact; explained in adjacent comment; not actual dead code |
| All modified files | — | Cross-target clippy PARTIAL→CI | INFO | Cross-toolchain absent on Windows host (CLAUDE.md documented pattern); all changes are cfg-unconditional; native `cargo build --workspace` exits 0 |

No TBD, FIXME, XXX, or unreferenced debt markers found in phase-modified files. No placeholder text. No hardcoded empty data flowing to rendering.

---

### Human Verification Required

#### 1. override-01.ps1 Live Gate Run

**Test:** On a dev host with Python + nono_py extension built (`maturin develop`), openssl on PATH, and nono.exe 3.2+ on PATH, run: `pwsh -File scripts/verify-dark.ps1 --gate override-01`
**Expected:** Exit 0, verdict `PASS`, `sc1=true`, `sc2=true`, `sc3=true` in the JSON detail. The .bat stub injection must succeed (SC1 and SC3 must not hit `SKIP_HOST_UNAVAILABLE`).
**Why human:** The gate includes a `SKIP_HOST_UNAVAILABLE` branch for SC1/SC3 when the .bat stub fails to execute via Rust's `std::process::Command`. The automated verifier cannot replay this without the live environment. SUMMARY reports exit 0 PASS, but this must be confirmed hands-on.

#### 2. pytest test_override_wiring.py Full Run

**Test:** In the nono-py repo with extension built, run: `python -m pytest tests/test_override_wiring.py -v`
**Expected:** 11 tests pass on a Windows 10/11 host (platform guard ensures they skip on non-Windows CI).
**Why human:** All 11 tests are `pytestmark = pytest.mark.skipif(sys.platform != "win32")` — they require the Windows platform and the compiled PyO3 extension. Cannot be confirmed without executing on Windows.

#### 3. MUT-03 Behavioral Proof (OS confinement remains active under override)

**Test:** Run `confined_run` with a real override grant (scope: `/tmp/test`) against a real nono.exe on Win11 with nono-wfp-service active. Attempt to access `/tmp/other` (not in scope) from inside the confined child.
**Expected:** Access to `/tmp/other` is denied by OS enforcement (AppContainer/WFP) despite the override grant for `/tmp/test`. The override only widened access to the granted scope — it did not bypass confinement.
**Why human:** MUT-03 is architecturally guaranteed by the additive-only `--allow` path, but behavioral proof requires an elevated Win11 host with the live enforcement stack. Cannot be automated without the daemon + WFP path (per `wfp_confined_egress_and_daemon_gate.md` memory).

---

### Gaps Summary

No blocking gaps. All 10 non-deferred must-haves are VERIFIED in the codebase. The phase achieves its goal: the offline override verifier is wired into `confined_run`/`confine`, the AUD-04 fail-closed bilateral handshake is implemented across both repos, `PolicyOverrideApplied` is emitted before spawn, and the DF-01 gate exists with the correct contract.

The 3 human verification items are behavioral/integration checks that require a live Windows host with nono.exe and the compiled Python extension — standard for this milestone's host-gated verification pattern.

**VFY-01 PARTIAL [BLOCKING-93]:** The live `POST /actions` AND-gate is explicitly deferred to Phase 93 per locked decision D-03. The seam is correctly placed and documented in both `confined_run` and `confine`. This is not a phase failure.

---

*Verified: 2026-06-22T14:30:00Z*
*Verifier: Claude (gsd-verifier)*
