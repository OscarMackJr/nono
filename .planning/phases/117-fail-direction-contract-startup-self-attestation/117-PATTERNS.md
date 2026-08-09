# Phase 117: Fail-Direction Contract + Startup Self-Attestation - Pattern Map

**Mapped:** 2026-08-09
**Files analyzed:** 12 new/modified (per CONTEXT.md + RESEARCH.md's corrected file list)
**Analogs found:** 10 / 12 (2 explicitly "no analog — new pattern")

All line numbers below were re-verified by symbol against the current working tree on
2026-08-09 (not copied from CONTEXT.md/RESEARCH.md without re-checking). Corrections to
those two docs are called out inline where found.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` (NEW) | model/registry (enum + per-variant metadata) | transform (static data → matrix lookup) | `crates/nono/src/undo/types.rs:291-345` (`NetworkAuditDenialCategory::ALL` + drift-guard match) | **exact** — closest thing in the tree to "enum whose variants are exhaustively enumerable and drift-guarded" |
| Attestation module (NEW, e.g. `exec_strategy_windows/attestation.rs`) | service (probe + decide) | event-driven (probes live child state at one gate point) | `exec_strategy_windows/launch.rs:2134-2145` (existing gate-and-terminate idiom) + `crates/nono/src/machine_policy.rs:150-183`/`696-705` (platform-neutral status type, cfg-gated population) | role-match (gate idiom is exact; the "attest and classify into 4 states" shape has no full precedent) |
| Fault-injection seam (NEW, Cargo-feature-gated) | config/test-seam | request-response (compiled-out switch) | `crates/nono-cli/src/trust_cmd.rs:415-433`, `trust_keystore.rs` (11 sites) + `Cargo.toml:43-44` (`test-trust-overrides`) | **exact** |
| `proj/SPEC-windows-fail-direction-contract.md` (NEW) | doc/config artifact | n/a | No analog — new pattern (see below) | none |
| Drift-check + meta-test (NEW test) | test | transform (source-scan / enum-iterate) | `crates/nono-cli/tests/resl_supervisor_drain.rs` (full file) | **exact** |
| Per-layer forced-unavailable tests (NEW) | test | event-driven | `crates/nono-cli/src/exec_strategy_windows/launch.rs` `#[cfg(all(test, target_os = "windows"))]` modules (e.g. `apply_process_handle_to_containment_invalid_job_returns_err`, `:4212`) | role-match |
| `exec_strategy_windows/launch.rs` (MODIFIED — D-21 gate insertion) | controller/gate | request-response | itself, `:2134-2145` (existing adjacent gates: job-assign, resource-limits) | **exact** — insert between `:2141` and `:2145` |
| `crates/nono-shell-broker/src/main.rs` (MODIFIED — 2nd gate) | controller/gate | request-response | `main.rs:615-644` (broker's own existing `OpenProcessToken` + `apply_low_il_label_to_token` gate before its `ResumeThread` at `:645-660`) — proves the pattern is already live in this exact spot | **exact** |
| `crates/nono-cli/src/agent_daemon/launch.rs` (MODIFIED — 3rd gate) | controller/gate | request-response | itself — WFP gate ("step 6.5", `:698-735`) and DACL gate ("step 6.6", `:755-799`) immediately before `ResumeThread(thread_handle_raw)` (`:848`) | **exact**, same idiom, independent implementation (module doc `:26-30` explicitly disclaims sharing code with `exec_strategy_windows/`) |
| `crates/nono-cli/src/output.rs` (MODIFIED — downgraded claim) | view/render | transform | `print_banner` (`:32-46`, confirmed exact) for placement; `format_scope_status` (`:492-500`, `#[cfg(target_os = "linux")]`) for the 3/4-state vocabulary shape | role-match — banner has zero existing claim text to correct; `format_scope_status` is Linux-only, needs a Windows analog written fresh |
| `crates/nono/src/diagnostic/codes.rs` + `crates/nono/src/error.rs` (MODIFIED — new code/variant) | model/error | transform | `error.rs:207-235` (`LabelApplyFailed`/`DaclApplyFailed` variant shape) + `error.rs:383-448` (`diagnostic_code()` exhaustive match, no wildcard) | **exact** |
| `crates/nono/src/machine_policy.rs` (MODIFIED — `required_layers` field) | model/config | CRUD (registry read → parse → fail-open/closed) | `TelemetryConfig` sub-struct (`:95-131`) + `parse_telemetry_config` (`:511-580`, degrade-not-abort) vs. the egress-fields' abort discipline (`is_unconfigured`, `:238`) | **exact** |
| `crates/nono-cli/src/telemetry/` (MODIFIED — new event) | event emitter | pub-sub / event-driven | `SecurityEventType`/`event_id_for` (`event.rs:67-111`) + `SecurityEvent` struct (`event.rs:253-274`) + `SecurityEventLayer::emit_override_event` (`mod.rs:218-321+`) | **exact** |
| `crates/nono-cli/Cargo.toml` (MODIFIED — `[features]`) | config | n/a | itself, `:37-44` (`test-trust-overrides` precedent) | **exact** |

## Pattern Assignments

### `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` (NEW — registry enum)

**Analog:** `crates/nono/src/undo/types.rs:291-345` (`NetworkAuditDenialCategory`)

This is the single strongest analog in the whole tree for D-01's registry shape, and it directly
contradicts RESEARCH.md's framing that "no layer enum exists today... D-01's registry is
genuinely new construction, not a refactor." The *registry* (per-row metadata: expectancy
matrix, outcome, call site) is new. But the **enum + exhaustive-enumeration + drift-guard**
*shape* has a shipped, recent (Phase 115), heavily-commented precedent in the same workspace
that a plan should copy near-verbatim.

**`const ALL` idiom** (`crates/nono/src/undo/types.rs:291-345`):
```rust
impl NetworkAuditDenialCategory {
    /// Every variant of this enum, in declaration order.
    ///
    /// D-11 (Phase 115-02, DRAIN-02): self-enumerating so the DRAIN-02
    /// round-trip test ... does not need its own hand-written variant
    /// list — a hand-maintained list drifting from its source of truth is
    /// exactly the class DRAIN-02/NEW-06 exist to close.
    ///
    /// **D-11 dependency choice, decided here:** the zero-new-dependency
    /// fallback (`const ALL` + `assert_all_variants_covered` below), not
    /// `strum::EnumIter`. `strum`/`strum_macros` are completely absent from
    /// this workspace today ...
    pub const ALL: &'static [NetworkAuditDenialCategory] = &[
        NetworkAuditDenialCategory::AuthenticationFailed,
        NetworkAuditDenialCategory::EndpointPolicy,
        // ... one entry per variant, declaration order ...
        NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed,
    ];
}
```

**Compile-time drift guard** (same file, `:347-377`) — this is the mechanism D-32's meta-test
needs at the *enum* level (source-scan covers the *test-exists* half):
```rust
// IMPORTANT: match is exhaustive (no wildcard arm) so the compiler forces
// handling of every current and future NetworkAuditDenialCategory variant
#[cfg_attr(not(test), allow(dead_code))]
fn assert_all_variants_covered(category: &NetworkAuditDenialCategory) {
    match category {
        NetworkAuditDenialCategory::AuthenticationFailed
        | NetworkAuditDenialCategory::EndpointPolicy
        | /* ... */
        | NetworkAuditDenialCategory::CaptureBufferOrRewriteFailed => {}
    }
}
```
**Copy this shape for `LayerId`:** `pub enum LayerId { .. }` + `pub const ALL: &'static [LayerId] = &[...]` +
a `#[cfg_attr(not(test), allow(dead_code))] fn assert_all_layer_ids_covered(id: &LayerId)` with an
exhaustive no-wildcard match. Adding a layer without updating `ALL` or the guard match fails the
build — exactly D-01's "registry wins, drift fails a test" requirement, and it needs zero new
workspace dependency (the file's own doc comment already argued down `strum`).

**Secondary analog for per-row metadata (not enumeration):** `NonoDiagnosticCode`
(`crates/nono/src/diagnostic/codes.rs:17-38`) — flat `#[non_exhaustive]` enum, no `ALL` const, no
per-variant payload. Confirms RESEARCH's point that `codes.rs` is the *wrong* model for D-01 (no
enumeration idiom there) — use `undo/types.rs` instead, not `codes.rs`, despite CONTEXT.md's own
suggestion to "look hard at `codes.rs`."

**Unwrap ban note:** neither analog uses `.unwrap()`; `assert_all_variants_covered`'s exhaustive
match is itself the compile-time enforcement mechanism (no runtime panic path at all).

---

### Attestation module (NEW)

**No single analog** — RESEARCH.md's "no probe queries the suspended child's token" finding is
confirmed live (see below); this is assembled from three partial analogs, not one file to copy
wholesale.

**Gate-and-terminate idiom to imitate** (`crates/nono-cli/src/exec_strategy_windows/launch.rs:2134-2145`,
re-verified — line numbers match RESEARCH exactly):
```rust
if let Err(err) = apply_process_handle_to_containment(containment, process.raw()) {
    terminate_suspended_process(process.raw(), "AssignProcessToJobObject failed");
    return Err(err);
}
// Phase 16 RESL-01/02/04: apply the four resource-limit info classes BEFORE
// ResumeThread so the child never runs with an uncapped Job Object. Any
// failure here is fail-closed — terminate the suspended child and propagate.
if let Err(err) = apply_resource_limits(containment, limits) {
    terminate_suspended_process(process.raw(), "apply_resource_limits failed");
    return Err(err);
}
resume_contained_process(process.raw(), thread.raw())?;
```
The D-21 attestation call slots in as a third gate of this identical shape, between the
`apply_resource_limits` block and `resume_contained_process`. `terminate_suspended_process`
(`launch.rs:395-402`) wraps `TerminateProcess` — reuse it, don't reinvent.

**Probes already imported (production, top-level) at `mod.rs:42-48`**, confirmed exact against
RESEARCH.md:
```rust
use windows_sys::Win32::Security::{
    CreateWellKnownSid, DuplicateTokenEx, GetTokenInformation, SecurityImpersonation,
    SetTokenInformation, TokenElevation, TokenIntegrityLevel, TokenPrimary, WinLowLabelSid,
    SECURITY_MAX_SID_SIZE, TOKEN_ADJUST_DEFAULT, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE,
    TOKEN_ELEVATION, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
};
```
**Correction, re-confirmed:** `IsProcessInJob` is imported *inside* the
`#[cfg(all(test, target_os = "windows"))]` broker-dispatch test module (`mod.rs:58-65` comment:
"consumed only by Plan 31-03's `broker_dispatch_tests`... the import lives inside that test
module"), not at top level — promoting it to a production import is new plumbing, not a reuse, as
RESEARCH already flagged. `TokenAppContainerSid` (needed for the AppContainer probe) has zero
hits anywhere in the crate — genuinely new FFI surface, not present today.

**Platform-neutral status type + `cfg(windows)`-only population — a REAL analog exists,
correcting CONTEXT.md's "if none exists, say so" fallback:**
`crates/nono/src/machine_policy.rs:150-183` (`MachineEgressPolicy`) is a data type that compiles
on every host (`Vec<String>` / plain sub-struct fields only, doc comment `:137-139`: "intentionally
contains only `Vec<String>` fields so that it compiles on every platform") and is populated only
on Windows:
```rust
// crates/nono/src/machine_policy.rs:696-705
#[cfg(target_os = "windows")]
pub fn read_machine_egress_policy() -> Result<Option<MachineEgressPolicy>> {
    windows_reader::read_machine_egress_policy_impl()
}

/// Non-Windows stub: returns `Ok(None)` (no HKLM registry on Linux/macOS).
#[cfg(not(target_os = "windows"))]
pub fn read_machine_egress_policy() -> Result<Option<MachineEgressPolicy>> {
    Ok(None)
}
```
**Use this exact split for `LayerId`/the 4-state attestation-status type and its reader:** the type
itself lives outside any `cfg`, the populate/probe function is `#[cfg(target_os = "windows")]`
with a `#[cfg(not(target_os = "windows"))]` stub returning an empty/neutral value — matching D-11
literally. This is a stronger, more concrete precedent than the researcher's "genuinely new
pattern, say so" fallback; report it as found, not absent.

---

### Fault-injection seam (NEW, Cargo-feature-gated) — D-29/D-30

**Analog:** `crates/nono-cli/src/trust_cmd.rs:415-433`, `Cargo.toml:37-44`

**`[features]` block to extend** (`crates/nono-cli/Cargo.toml:37-44`, confirmed exact):
```toml
[features]
default = ["system-keyring"]
# Enables OS system keyring access (macOS Keychain / Linux Secret Service).
# Disable this for headless/container environments without libdbus.
# Controls keyring for nono-cli, nono, and nono-proxy in one flag.
system-keyring = ["dep:keyring", "nono/system-keyring", "nono-proxy/system-keyring"]
# Enables test-only trust root overrides used by the integration harness.
test-trust-overrides = []
```
Add e.g. `layer-fault-injection = []` alongside `test-trust-overrides`, default-off, same crate.

**Compiled-out + production-fallback pairing** (`trust_cmd.rs:411-433`, confirmed exact):
```rust
// Test-only: allow overriding Fulcio + Rekor URLs via env vars so
// hermetic integration tests can inject httpmock servers without
// modifying the production binary. Only compiled in test builds
// (feature = "test-trust-overrides").
#[cfg(feature = "test-trust-overrides")]
let context = {
    let fulcio_url = std::env::var("NONO_TEST_FULCIO_URL")
        .unwrap_or_else(|_| "https://fulcio.sigstore.dev".to_string());
    // ... build a test SigningConfig ...
    sigstore_sign::SigningContext::with_config(config)
};
#[cfg(not(feature = "test-trust-overrides"))]
let context = sigstore_sign::SigningContext::production();
```
Copy this `#[cfg(feature = "...")] let x = { .. }; #[cfg(not(feature = "..."))] let x = production();`
shape for each per-layer force-unavailable hook: the feature-on arm reads an env-var-or-flag
override, the feature-off arm is the real probe/apply call with **no branch to disable it** —
this is what makes it structurally absent from release binaries, unlike the WFP toggle below.

**The superseded runtime-gated toggle** (D-29's precedent, D-30's flagged inconsistency) —
confirmed exact against RESEARCH/CONTEXT citations:
- Storage: `static WINDOWS_WFP_TEST_FORCE_READY: AtomicBool = AtomicBool::new(false);` (`exec_strategy_windows/mod.rs:558`)
- Setter with runtime env-var gate (`mod.rs:584-597`):
```rust
pub(crate) fn set_windows_wfp_test_force_ready(force_ready: bool) {
    if force_ready && std::env::var_os("NONO_TEST_HARNESS").is_none() {
        tracing::warn!(
            "--dangerous-force-wfp-ready has no effect outside the test harness \
             (NONO_TEST_HARNESS env var not set)"
        );
        return;
    }
    WINDOWS_WFP_TEST_FORCE_READY.store(force_ready, Ordering::Relaxed);
}
```
- CLI flag (`cli.rs:2206-2214`, hidden): `#[arg(long, hide = true, help_heading = "OPTIONS")] pub dangerous_force_wfp_ready: bool,`
- Wiring (`command_runtime.rs:86-93`):
```rust
#[cfg(target_os = "windows")]
if run_args.sandbox.dangerous_force_wfp_ready {
    exec_strategy::set_windows_wfp_test_force_ready(true);
}
```
This is *always compiled into every release binary*; only the env var stands between an attacker
with process-spawn control and flipping a confinement layer off — exactly D-30's concern. Per
CONTEXT.md D-30, the plan must explicitly choose: migrate this to the `#[cfg(feature =
"layer-fault-injection")]` shape above, or record it as a named SC4 discrepancy row (D-13/D-15)
with a successor. Do not leave it addressed by neither.

---

### Self-enforcing source-scan test (D-32 house pattern)

**Analog:** `crates/nono-cli/tests/resl_supervisor_drain.rs` (full file, 133 lines — read in one pass)

**Correction to CONTEXT.md:** the house pattern is `env!("CARGO_MANIFEST_DIR")` +
`std::fs::read_to_string` (fresh at test-run time), **not** `include_str!` (a compile-time embed;
zero hits for `include_str!` in `crates/nono-cli/tests/*.rs`).

End-to-end shape:
```rust
const START_SENTINEL: &str = "WR-15-SUPERVISOR-EXITS-START";
const END_SENTINEL: &str = "WR-15-SUPERVISOR-EXITS-END";

fn read_exec_strategy() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest_dir)
        .join("src")
        .join("exec_strategy.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn sentinel_region(src: &str) -> String {
    let start = src.find(START_SENTINEL).unwrap_or_else(|| panic!(/* named, actionable message */));
    let end = src.find(END_SENTINEL).unwrap_or_else(|| panic!(/* ... */));
    assert!(end > start, "{END_SENTINEL} must appear after {START_SENTINEL} in source order");
    src[start..end]
        .lines()
        .map(|line| match line.find("//") { Some(idx) => &line[..idx], None => line })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_supervisor_loop_exit_drains_network_notifications() {
    let src = read_exec_strategy();
    let region = sentinel_region(&src);
    let bare = region.matches("return Ok((").count();
    assert_eq!(bare, 0, "<named, actionable failure message citing the exact file>");
    // Guards against a vacuously-true empty region:
    let drained = region.matches("drained_return!").count();
    assert!(drained >= 4, "<named count check>");
}
```
This is D-32's model for the meta-test: read the registry source + the test file(s) via
`CARGO_MANIFEST_DIR`, and assert every `LayerId::ALL` entry name appears as a substring of some
`#[test] fn` name discovered in the test file text. It works on the Windows dev host even for
content whose *subject* requires a different OS (this file scans Linux supervisor code from a
Windows-hosted `cargo test` run) — the same property D-31/D-32 need for Windows-only layer rows.

**Unwrap ban note:** every fallible call here is `.unwrap_or_else(|e| panic!(...))`, never bare
`.unwrap()` — tests are exempt from `-D clippy::unwrap_used` per CLAUDE.md, but this file still
follows the named-message discipline; copy that habit, not bare `.unwrap()`.

---

### RAII guard / Drop-order discipline (D-22 abort mechanics)

**Analog:** `PreparedWindowsLaunch` (`exec_strategy_windows/mod.rs:296-333`) + `AppliedLabelsGuard` (`labels_guard.rs:57-192`)

**Struct + field-ordering comments** (`mod.rs:296-333`, confirmed exact against RESEARCH's
line range and its correction):
```rust
struct PreparedWindowsLaunch {
    // Phase 21: applied-labels guard reverts mandatory-label ACEs on drop.
    // Declared BEFORE _network_enforcement so Rust's reverse-of-declaration
    // drop order reverts labels first, then tears down network enforcement.
    _applied_labels: labels_guard::AppliedLabelsGuard,
    _applied_dacls: Option<dacl_guard::AppliedDaclGrantsGuard>,
    _applied_ancestor_traverse: Option<dacl_guard::AppliedAncestorTraverseGuard>,
    _applied_ancestor_read_attrs: Option<dacl_guard::AppliedAncestorReadAttributesGuard>,
    _network_enforcement: Option<NetworkEnforcementGuard>,
    launch_program: PathBuf,
}
```
**Load-bearing correction (re-verified against RESEARCH §D and the Rust reference):** these
comments assert "reverse-of-declaration drop order" for *struct fields*. **Rust struct fields
drop in forward declaration order**, not reverse — the reverse-of-declaration (LIFO) rule applies
to local stack bindings, not struct fields. The code's actual behavior is still correct today
because the fields happen to already be declared labels→dacls→ancestors→network, which is the
order the authors want — forward-order drop reproduces it. **Do not copy the comment's stated
mechanism into new code; if a plan adds a field to this or a similar guard struct, order it
forward (top-to-bottom = drop order), and correct or remove the "reverse-of-declaration" wording
if touching this file** (candidate small SC4 fix per D-15).

**Individual guard shape** (`labels_guard.rs:52-59`, `:188-192`):
```rust
/// RAII guard that reverts applied mandatory labels when dropped.
#[derive(Debug, Default)]
pub(crate) struct AppliedLabelsGuard {
    entries: Vec<AppliedLabel>,
}
// ...
impl Drop for AppliedLabelsGuard {
    fn drop(&mut self) {
        self.revert_all();
    }
}
```
No `.unwrap()`/`.expect()` anywhere in the guard or its `Drop` impl — reverts are best-effort and
logged, never panicking (`clippy::unwrap_used` applies to this non-test file).

---

### Adding a `NonoDiagnosticCode` + wiring `NonoError::diagnostic_code()`/`remediation()`

**Analog:** `LabelApplyFailed` / `DaclApplyFailed` (`crates/nono/src/error.rs:200-235`)

**Full edit-site set for one new error variant + diagnostic code** (confirmed exact, all four
sites re-read):

1. New `NonoError` variant (`error.rs`, model on `:207-214`):
```rust
#[error("Failed to apply integrity label to {path}: {hint} (HRESULT: 0x{hresult:08X})")]
LabelApplyFailed {
    path: PathBuf,
    hresult: u32,
    hint: String,
},
```
2. New `NonoDiagnosticCode` variant (`crates/nono/src/diagnostic/codes.rs:20-38`) — the enum is
   `#[non_exhaustive]` so additive from outside the crate, but the match below is NOT:
```rust
#[non_exhaustive]
pub enum NonoDiagnosticCode {
    SandboxDeniedPath,
    // ... add e.g. LayerAttestationFailed here ...
    Other,
}
```
3. `diagnostic_code()` match arm — **exhaustive, no wildcard, no `#[cfg]` needed for a
   cross-platform variant** (`error.rs:383-448`, this exact style already used for the analog):
```rust
Self::LabelApplyFailed { .. }
| Self::DaclApplyFailed { .. }
| Self::PolicyLoadFailed { .. }
| Self::TelemetryUnavailable { .. }
| Self::TelemetryConfigInvalid { .. } => NonoDiagnosticCode::ConfigurationError,
```
Note the one `#[cfg(target_os = "linux")]`-gated arm at the tail (`error.rs:445-446`) —
`Self::Landlock(_) | Self::LandlockPath(_) => NonoDiagnosticCode::SandboxDeniedPath`. A new
Windows-only variant does **not** need its own `cfg` arm if `LayerAttestationFailed` itself is a
plain (non-cfg) variant only ever *constructed* on Windows — the match arm can be unconditional
since the enum variant exists on every platform (per D-11's own platform-neutral-type framing).
4. Optional `remediation()` arm (`error.rs:452-479`) — most variants fall through to `_ => None`;
   only add an arm if a concrete `NonoRemediation` applies.

**ADR-86 carve-out confirmed still standing** (per CLAUDE.md's boundary table): the *code/variant*
belongs in `crates/nono` (library), the *message formatting/rendering* for Windows denial output
stays CLI-side in `nono-cli`.

---

### Adding a structured event to `SecurityEventLayer`

**Analog:** `TelemetryDegraded` (EventID 10005) end-to-end, `crates/nono-cli/src/telemetry/{event,mod}.rs`

**1. New `EVENT_ID_*` constant** (`event.rs:38-56`):
```rust
pub const EVENT_ID_TELEMETRY_DEGRADED: u32 = 10005;
```
**2. New `SecurityEventType` variant** (`event.rs:67-90`) + **3. `event_id_for` match arm**
(`event.rs:97-111`, exhaustive):
```rust
pub enum SecurityEventType {
    // ...
    TelemetryDegraded,
}
pub fn event_id_for(t: &SecurityEventType) -> u32 {
    match t {
        // ...
        SecurityEventType::TelemetryDegraded => EVENT_ID_TELEMETRY_DEGRADED,
    }
}
```
**4. `SecurityEvent` struct** (`event.rs:251-274`) — reused as-is unless the new event needs a
field none of the existing ones carry:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SecurityEvent {
    pub event_type: SecurityEventType,
    pub agent_pid: u32,
    #[serde(skip_serializing_if = "Option::is_none")] pub path_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub path_category: Option<PathCategory>,
    #[serde(skip_serializing_if = "Option::is_none")] pub host: Option<String>,
    pub session_id: String,
    pub chain_head: String,
    pub timestamp_unix_ms: u64,
}
```
**5. Construction → `advance_chain` → emission** — `SecurityEventLayer::emit_override_event`
(`mod.rs:287-321+`) is the closest end-to-end direct-call example (bypasses the tracing-intercept
`on_event` path, called directly from `execution_runtime.rs`), doc comment states the pattern
explicitly:
```rust
/// # AUD-04 contract
/// Callers MUST treat `Err` as FATAL and abort before spawning the sandboxed child.
/// ...
/// # Telemetry-disabled behaviour (D-14 degrade-not-abort)
/// When `inner.config.enabled` is `false`, the HMAC chain still advances
/// (for sequence correctness and audit ordering) but no ETW/AppLog emit occurs.
#[must_use = "AUD-04: Err means the audit record was not committed — callers MUST \
              return Err before spawning (never silently proceed)"]
pub fn emit_override_event(&self, event_type: &SecurityEventType, jti: &str, kms_key_id: &str, /* ... */) -> Result<String, &'static str> { /* advances chain, returns chain_head hex */ }
```
An attestation-downgrade event method should follow this exact `#[must_use = "..."]` +
Ok(chain_head)/Err(&'static str) shape, called directly (not via `tracing`) from the D-21 gate
point, since that point already has direct access to `SECURITY_LAYER.get()` the same way
`execute_sandboxed` does.

**Open design question, unresolved by any analog (flag for planner, matches RESEARCH Open
Question 4):** no existing `SecurityEvent` field carries a free-text "which layer" detail; D-28
requires that detail to be confined-process-unreadable. Verify ETW/Application-Log ACL readability
before deciding whether to add a field to this struct or route layer specifics through a different
channel.

---

### Adding a field to `machine_policy.rs` (D-26 `required_layers` policy)

**Analog:** `TelemetryConfig` sub-struct extension pattern

**Sub-struct + doc-documented degrade discipline** (`machine_policy.rs:95-131`):
```rust
/// # Degrade-not-abort (D-14)
///
/// Malformed telemetry registry values fall back to this struct's `Default`
/// and surface a `TelemetryConfigInvalid` warning to stderr. They do NOT
/// return `Err` — contrast with `PolicyLoadFailed` for egress (D-07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryConfig {
    #[serde(default = "default_telemetry_enabled")] pub enabled: bool,
    #[serde(default = "default_telemetry_channel")] pub channel: String,
    #[serde(default = "default_telemetry_min_severity")] pub min_severity: TelemetrySeverity,
}
```
**Nested into the parent + explicit `is_unconfigured` exclusion** (`machine_policy.rs:171-183`):
```rust
/// Telemetry sub-section (Phase 84 D-12).
/// ...
/// **INVARIANT:** This field MUST NOT be counted in [`Self::is_unconfigured`].
/// A telemetry-only HKLM write must not flip the daemon to strict deny-all
/// egress (84-PATTERNS.md invariant 3 / CR-02).
#[serde(default)]
pub telemetry: TelemetryConfig,
```
**Read-with-degrade lifecycle** (`machine_policy.rs:511-580`, `parse_telemetry_config`) — absent
subkey → defaults (fine); present-but-unreadable → `eprintln!` warning + defaults (degrade, no
abort); malformed individual value → per-field warning + that field's own default. This is the
`TelemetryConfig`-side (degrade) half of the abort-vs-degrade split. Contrast with the *egress*
fields' own abort behavior (present-but-unreadable key at `read_machine_egress_policy_impl`
aborts — cited by `error.rs`'s `PolicyLoadFailed` variant and `machine_policy.rs:19`'s module doc).

**Decision the plan must make explicitly (per D-26):** does `required_layers` behave like
telemetry (degrade, excluded from `is_unconfigured`) or like egress (abort on
present-but-unreadable)? RESEARCH recommends telemetry-shaped (should NOT count toward
`is_unconfigured`, an admin who only sets required-layers shouldn't flip strict-deny-all egress) —
but D-26 itself says the CLI flag "can only tighten, never loosen" what machine policy sets, which
is closer to the egress fields' abort-on-malformed posture than to telemetry's silent-default. Cite
this tension in the plan rather than picking one precedent without stating why.

---

### Platform-neutral type with `cfg(windows)` population (D-11)

**Analog found — this is NOT "no analog exists."** `crates/nono/src/machine_policy.rs:150-183` +
`:690-705` (`MachineEgressPolicy` / `read_machine_egress_policy`), shown in full under the
Attestation-module section above. Reuse that exact split for the `LayerId` enum's supporting types
(attestation-status enum, any per-arm-expectancy struct) and for the registry's *population*
function if any part of it needs live Windows data — the enum/struct declarations themselves carry
no `cfg`, only the function(s) that populate/probe them are `#[cfg(target_os = "windows")]` with a
`#[cfg(not(target_os = "windows"))]` stub.

---

## Shared Patterns

### Fail-closed vs. degrade-not-abort vocabulary
**Source:** `crates/nono/src/machine_policy.rs:19` (module doc), `:96-100`, `:513-514`
**Apply to:** the attestation module's abort/degrade decision (D-25/D-26), the registry's
four-value outcome vocabulary (D-05/D-06)
The module already names this split in prose ("D-07 egress aborts, D-14 telemetry degrades") —
reuse the vocabulary rather than inventing new terms for what is structurally the same choice.

### Gate-before-`ResumeThread`, terminate-on-Err
**Source:** `exec_strategy_windows/launch.rs:2134-2145`, `nono-shell-broker/src/main.rs:615-660`,
`agent_daemon/launch.rs` steps 6.5/6.6 (`:698-799`) before `:848`
**Apply to:** all three D-23 gate insertion points (nono-cli direct spawn, broker's own child
spawn, daemon spawn) — same idiom, independently implemented in three files; do not assume one
shared helper exists to call from all three (RESEARCH §D confirms `agent_daemon/launch.rs`
explicitly does not depend on `exec_strategy_windows/`).

### `#[cfg(feature = "...")]` / `#[cfg(not(feature = "..."))]` paired arms
**Source:** `crates/nono-cli/Cargo.toml:43-44`, `trust_cmd.rs:415-433`, `trust_keystore.rs` (11 sites)
**Apply to:** the D-30 fault-injection seam in `nono-cli`. **`crates/nono-shell-broker` has NO
`[features]` block at all today** (confirmed by reading `Cargo.toml` list in RESEARCH §E) — if the
broker's own gate (site 2 of 3) needs a force-unavailable hook, a brand-new `[features]` section
must be added to that crate, not an extension of an existing one.

### `env!("CARGO_MANIFEST_DIR")` + `std::fs::read_to_string` source-scan tests
**Source:** `crates/nono-cli/tests/resl_supervisor_drain.rs` (full file)
**Apply to:** D-32's meta-test and the SPEC drift-check test, if the SPEC is drift-checked rather
than generated (Claude's discretion, D-01). Not `include_str!` — that is compile-time and has zero
precedent in this test directory.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `proj/SPEC-windows-fail-direction-contract.md` | doc/config artifact | n/a | No existing `proj/SPEC-*.md` (non-ADR) document was located to model section layout on; `proj/` today holds `ADR-*.md` and `DESIGN-*.md` only (per CLAUDE.md's References list). D-03 explicitly allows the plan to invent this shape; use CONTEXT.md's required-sections list (D-15 discrepancies, D-12 Unix boundary, D-20 mid-session limit, D-24 latency budget) as the actual spec, not a code analog. |
| `output.rs`'s Windows-side 3/4-state downgrade-claim text | view/render | transform | `format_scope_status` (`output.rs:492-500`) is the right *shape* to imitate (a `match` over a tuple of booleans/states returning a short phrase) but is `#[cfg(target_os = "linux")]`-only content — there is no existing Windows-side "claim status" renderer to copy verbatim; write one fresh, modeled on this function's shape and on `scope_status_color` (`:502-509`) for the accompanying color mapping. |

## Corrections to CONTEXT.md / RESEARCH.md (symbol-verified, not file-presence)

- **D-01's registry enum shape does have a strong analog**, contra RESEARCH's "no layer enum
  exists today... genuinely new construction, not a refactor": `NetworkAuditDenialCategory`
  (`crates/nono/src/undo/types.rs:291-345`) ships the exact `const ALL` + exhaustive-match
  drift-guard idiom D-32 needs, added as recently as Phase 115. CONTEXT.md's own suggestion to
  "look hard at `codes.rs`" is the *weaker* of the two candidates — `codes.rs` has no enumeration
  idiom at all.
- **D-11's "platform-neutral type, `cfg(windows)` population" has a real analog**, contra the
  prompt's fallback instruction to say so if none exists: `MachineEgressPolicy` +
  `read_machine_egress_policy()` (`crates/nono/src/machine_policy.rs:150-183`, `:696-705`) is
  exactly this shape, doc-commented as deliberate ("intentionally contains only `Vec<String>`
  fields so that it compiles on every platform").
- **`IsProcessInJob` is not a top-level production import**, contra CONTEXT.md's canonical_refs
  framing that probes are "already imported in `exec_strategy_windows/mod.rs`" — it is imported
  inside a `#[cfg(all(test, target_os = "windows"))]` test module only (`mod.rs:58-65`), matching
  RESEARCH's own correction. `TokenIntegrityLevel`/`TokenElevation`/`GetTokenInformation`/
  `OpenProcessToken`, by contrast, ARE genuine top-level production imports (`mod.rs:42-48`,
  `:67-69`) — re-verified exact.
- **`crates/nono-shell-broker` has no `[features]` block** — confirmed by reading the file
  structure implied in RESEARCH §E; any broker-side fault-injection hook is new plumbing for that
  crate, not an extension.

## Metadata

**Analog search scope:** `crates/nono/src/{undo,diagnostic,error.rs,machine_policy.rs}`,
`crates/nono-cli/src/{exec_strategy_windows,telemetry,output.rs,cli.rs,command_runtime.rs,
trust_cmd.rs,trust_keystore.rs}`, `crates/nono-cli/tests/`, `crates/nono-shell-broker/src/main.rs`,
`crates/nono-cli/src/agent_daemon/launch.rs`, all workspace `Cargo.toml` `[features]` blocks.
**Files scanned (read or targeted-read):** 18
**Pattern extraction date:** 2026-08-09
