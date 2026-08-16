# Phase 118: Per-Session Enforcement Receipts - Pattern Map

**Mapped:** 2026-08-16
**Files analyzed:** 12 (new) + 6 (modified)
**Analogs found:** 18 / 18

## Structural constraint that governs every mapping below

`nono-cli` has **no `[lib]` target** — only two `[[bin]]` targets, confirmed in
`crates/nono-cli/Cargo.toml:29-35` (`nono` → `src/main.rs`, `nono-agentd` →
`src/bin/nono-agentd.rs`). `nono-agentd.rs`'s `#[path]` include set is exactly three files
(confirmed by direct read, `crates/nono-cli/src/bin/nono-agentd.rs:42,50,57`):

```
#[path = "../agent_daemon/mod.rs"]
#[path = "../telemetry/mod.rs"]
#[path = "../agent_daemon/telemetry_init.rs"]
```

No `exec_strategy_windows/` module. This means any `.rs` file under `exec_strategy_windows/`
compiles into the `nono` binary **only**. Any symbol placed in a shared `#[path]`-included file
(e.g. `telemetry/mod.rs`) that is called by only one of the two binaries becomes a
`-D warnings`-fatal `dead_code` lint on the other, and per
`crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs`'s module header
(lines 1-59, read in full), **neither `#[expect(dead_code)]` nor a Cargo feature fixes this** —
both were empirically tried and rejected in Phase 117 Plan 17. The only real fix demonstrated in
this codebase is moving the code to the module tree the other binary's `#[path]` set never
touches, or promoting it to `crates/nono` (core), which both binaries link normally (not via
`#[path]`). `nono-cli`'s own `telemetry/mod.rs::advance_and_emit` (line 509) currently carries an
`#[allow(dead_code)]` with an 8-line comment justifying it as a documented, reviewed exception —
do not copy that exception as a general license; it required an explicit written argument.

This is **the concrete mechanism behind D-12** (receipt type + `LayerId` promoted to
`crates/nono`): it is the only placement reachable uniformly from `nono.exe`,
`nono-agentd.exe`, AND `nono-shell-broker.exe` (which depends only on `nono` core, confirmed in
`crates/nono-shell-broker/Cargo.toml` — no `nono-cli` dependency, cannot import
`layer_registry.rs`).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/nono/src/receipt.rs` (NEW) — `EnforcementReceipt` + `LayerId` promoted | model / core type | transform (census → record) | `crates/nono/src/attestation.rs` (`LayerAttestationStatus`) | exact |
| `crates/nono/src/receipt_chain.rs` (NEW) — chain domain + advance fn | model / core primitive | event-driven (append-only chain) | `crates/nono/src/audit.rs` (`hash_chain`, `CHAIN_DOMAIN_ALPHA`) — **keyless per D-25**, not `telemetry/mod.rs`'s HMAC | exact (post D-25) |
| `crates/nono/src/sandbox/windows.rs` (MODIFIED) — new `deny_sid_on_path`-style fn | utility (Win32 FFI wrapper) | file-I/O (ACL mutation) | same file, `edit_dacl_for_sid` (private) + `grant_sid_write_on_path` (public wrapper) | exact — same primitive, new `ACCESS_MODE` |
| `crates/nono/src/machine_policy.rs` (MODIFIED) — `require_receipts` field | config | request-response (registry read) | same file, `RequiredLayersPolicy` (lines 135-230) | exact |
| `crates/nono-cli/src/exec_strategy_windows/attestation.rs` (MODIFIED) — census pass alongside `decide_from_entries` | service (policy/decision) | CRUD-like classification loop | same file, `decide_from_entries` (:503-642) / `classify_row` (:405-497) | exact (extend, don't replace) |
| `crates/nono-cli/src/exec_strategy_windows/launch.rs` (MODIFIED) — receipt write in `apply_startup_attestation_gate` | controller (gate) | request-response (write-before-resume) | same file, `apply_startup_attestation_gate` (:1552-1675) | exact |
| `crates/nono-cli/src/receipt_sink.rs` (NEW) — sink path + ACL/label apply | utility | file-I/O | `crates/nono/src/sandbox/windows.rs` grant-ACE family + `try_set_mandatory_label` | role-match (new orchestration, existing primitives) |
| `crates/nono-cli/src/cli.rs` (MODIFIED) — `ReceiptCommands` | route/CLI definitions | request-response | same file, `AuditCommands` (:3497-3592) | exact |
| `crates/nono-cli/src/receipt_commands.rs` (NEW) — list/show/verify impls | controller (command dispatch) | CRUD (list/show) + request-response (verify) | `crates/nono-cli/src/audit_commands.rs` (`run_audit`, `cmd_list`/`cmd_show`/`cmd_verify`) | exact |
| `crates/nono-cli/src/app_runtime.rs` (MODIFIED) — dispatch wiring | route | request-response | same file, `Commands::Audit(args) => run_command_with_update(...)` (:109-110) | exact |
| `crates/nono-cli/src/agent_daemon/launch.rs` (MODIFIED) — `daemon_attest_and_decide` restructured to collect-all-then-decide; daemon-local expectancy table | service (fail-direction decision) | CRUD-like classification, restructured | same file, `daemon_attest_and_decide` (:1417-1500) + `daemon_decision_enum_variants` cross-check test (:2450-2570) | exact |
| `crates/nono-shell-broker/src/main.rs` (MODIFIED) — extended wire contract consumption, own receipt write | service (gate) + writer | request-response | same file, `broker_resume_gate` (:369-453), `BROKER_ATTESTABLE_LAYERS` (:321) | exact |
| `crates/nono-cli/src/exec_strategy_windows/attestation.rs` (MODIFIED, second hunk) — widen `BROKER_REQUIRED_LAYERS_ENV_VAR` wire payload / new sibling env var | service (wire contract producer) | event-driven (env-var handoff across process spawn) | same file, `required_layers_for_broker` (:718+) and `BROKER_REQUIRED_LAYERS_ENV_VAR` (:117) | exact |
| `crates/nono-cli/tests/receipt_content_free_scan.rs` (NEW) | test (source-scan) | transform (parse, no runtime OS) | `crates/nono-cli/tests/layer_registry_selfcheck.rs` | exact (idiom), new mechanism (field-TYPE parsing, not `fn`/variant matching) |
| `crates/nono-cli/tests/receipt_census_test.rs` (NEW) | test (meta/coverage) | transform | `crates/nono-cli/tests/layer_registry_meta_test.rs` | exact |
| `crates/nono-cli/tests/receipt_sentinel_roundtrip.rs` (NEW) | test (behavioral, seam-based) | transform | `crates/nono-cli/tests/layer_force_unavailable.rs` (seam pattern) | role-match |
| `crates/nono/src/receipt_chain.rs` unit tests | test | transform | `crates/nono/src/audit.rs` inline `#[cfg(test)]` module (`hash_chain`/`AuditVerifyArgs` recompute-compare coverage) | exact |
| daemon/broker expectancy cross-check test (NEW, likely appended to `agent_daemon/launch.rs`'s `#[cfg(test)] mod tests`) | test (discovery cross-check) | transform | same file, `daemon_decision_enum_variants` / `every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence` (:2450-2574) | exact |

---

## Pattern Assignments

### `crates/nono/src/receipt.rs` (NEW) — `EnforcementReceipt` type + promoted `LayerId`

**Analog:** `crates/nono/src/attestation.rs`

**Module-doc framing to copy verbatim in spirit** (`crates/nono/src/attestation.rs:1-34`):
```rust
//! Startup self-attestation primitives (CINT-02).
//!
//! This module is a policy-free status vocabulary plus raw OS probes over an
//! external process handle. It carries no decision about which layers are
//! required, what a missing layer should do, or how to render a downgraded
//! claim — that policy lives entirely in `crates/nono-cli` (D-02, ADR-86;
//! wired up by Plan 08). Everything here is mechanical observation.
//!
//! # D-04: shared vocabulary
//!
//! [`LayerAttestationStatus`] is the same type Phase 118's per-session
//! receipts consume when attesting "which layers were confirmed active for
//! this process". One type, not two lists that can diverge.
//!
//! # D-19: the supervisor attests, the confined process never does
//!
//! Every probe function in this module takes an already-open
//! [`ProcessHandle`] for a *target* process and performs an OS query against
//! it from outside. No function in this module accepts a claim, struct, or
//! string supplied BY the process being probed...
```
`receipt.rs`'s own module doc should state the same invariant explicitly (D-19 carried forward):
no receipt field may ever be populated from a claim made by the confined process.

**The exact four-state enum to carry verbatim (D-13)** (`crates/nono/src/attestation.rs:66-113`):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerAttestationStatus {
    Confirmed,
    EstablishedNotIndependentlyObservable,
    Unconfirmed,
    NotApplicable,
}
```
Already `pub` and re-exported at `crates/nono/src/lib.rs:69`:
```rust
pub use attestation::{LayerAttestationStatus, ProcessHandle};
```
Add the receipt type's re-export the same way — a flat re-export at `lib.rs`, not a nested path.

**`LayerId` today (to be promoted) — currently `pub(crate)`, 13 variants**
(`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:208-252`):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayerId {
    RestrictedToken,
    MandatoryIntegrityLabel,
    AppContainerProfile,
    DaclSessionSidGrant,
    DaclPackageSidGrant,
    DaclAncestorTraverse,
    DaclAncestorReadAttrs,
    WfpEgressFilters,
    FirewallRulesEgress,
    MinifilterAbsence,
    JobObjectContainment,
    BrokerAuthenticodeTrustGate,
    InterpreterCoverageGate,
}
```
The registry's own doc comment (line 204-207) already anticipates this move: *"D-11:
platform-neutral declaration (no `#[cfg]` here) so this type compiles and is usable (docs, drift
tests, **Phase 118's receipt type**) on every host."* Promoting it means: (a) move the enum body to
`crates/nono/src/receipt.rs` (or a dedicated `layer_id.rs` beside it) as `pub`, (b)
`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs` re-exports/re-uses it (`pub(crate)
use nono::LayerId;` or similar), keeping every `ArmExpectancy`/`REGISTRY_ENTRIES` row and the
`&'static str`-decoupled `WindowsTokenArm` comparison (`token_arm_matches`,
`attestation.rs:220-226`) unchanged — that decoupling precedent is explicitly why `LayerId` itself
does NOT need a similar string-based decoupling from `crates/nono-cli`.

**ADR-86 boundary argument to write down explicitly (D-12 requires this, not an assumption):**
cite `proj/ADR-86-library-boundary-convergence.md`'s Cluster A/B split (audit logic relocated
core-ward vs. diagnostic UX staying CLI-side) as the direct precedent — `LayerId` +
`LayerAttestationStatus` are the identity/status *vocabulary* (Cluster-A-shaped, core-eligible);
`ArmExpectancy`, `EntryPath`, `ProbeKind`, `REGISTRY_ENTRIES`, and every enforcing call site stay
CLI-side (Cluster-B-shaped, policy).

---

### `crates/nono/src/receipt_chain.rs` (NEW) — chain domain + advance function

**Analog (per D-25, NOT the HMAC telemetry chain):** `crates/nono/src/audit.rs`

**The exact keyless construction to mirror** (`crates/nono/src/audit.rs:657-669`, verified in-tree):
```rust
pub const CHAIN_DOMAIN_ALPHA: &[u8] = b"nono.audit.chain.alpha\n";
// ...
/// Hash one alpha rolling-chain link.
#[must_use]
pub fn hash_chain(previous: Option<&ContentHash>, leaf_hash: &ContentHash) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(CHAIN_DOMAIN_ALPHA);
    if let Some(prev) = previous {
        hasher.update(prev.as_bytes());
    } else {
        hasher.update([0u8; 32]);
    }
    hasher.update(leaf_hash.as_bytes());
    ContentHash::from_bytes(hasher.finalize().into())
}
```
Companion leaf-hash function (`crates/nono/src/audit.rs:648-655`):
```rust
#[must_use]
pub fn hash_event(event_bytes: &[u8]) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(EVENT_DOMAIN_ALPHA);
    hasher.update(event_bytes);
    ContentHash::from_bytes(hasher.finalize().into())
}
```
A **new, third domain constant** is required per D-11 (unchanged by D-25): `RECEIPT_CHAIN_DOMAIN`
/ `RECEIPT_EVENT_DOMAIN`, distinct from both `CHAIN_DOMAIN_ALPHA` and
`TELEMETRY_CHAIN_DOMAIN`/`TELEMETRY_EVENT_DOMAIN`, so a receipt-only sink can be verified without
needing telemetry events to fill in intermediate heads.

**`ContentHash` type to reuse** (`crates/nono/src/undo/types.rs:14-25`):
```rust
pub struct ContentHash([u8; 32]);
impl ContentHash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self { ... }
    pub fn as_bytes(&self) -> &[u8; 32] { ... }
}
```

**Mutex/advance-under-lock discipline to retain (D-11's WR-21/WR-09, primitive-independent) —
analog: `crates/nono-cli/src/telemetry/mod.rs`:**
```rust
// Source: crates/nono-cli/src/telemetry/mod.rs:509-525 (in-tree)
pub(crate) fn advance_and_emit<R>(
    &self,
    build_event_bytes: impl FnOnce(&str) -> Vec<u8>,
    emit: impl FnOnce(&str, &str, bool) -> R,
) -> Result<R, &'static str> {
    let mut inner = self.inner.lock().map_err(|_| "mutex poisoned")?;
    let event_bytes = build_event_bytes(&inner.session_id);
    advance_chain(&mut inner.chain, &event_bytes);
    let chain_head = chain_head_hex(&inner.chain.head);
    // The MutexGuard (`inner`) stays alive across this call — WR-21: mutex
    // held across the FULL build+advance+emit sequence, not just build+advance.
    Ok(emit(&inner.session_id, &chain_head, inner.config.enabled))
}
```
**What to copy:** the mutex-held-across-build+advance+emit shape and the single-accessor
discipline (chain fields private, `WR-09`). **What NOT to copy:** `ChainState.key` (ephemeral
`OsRng` key, zeroized on `Drop`, `telemetry/mod.rs:87-103`) — per D-25, the receipt chain has no
key at all; do not add a `key` field to the receipt's chain state.

**`AuditVerifyArgs`'s fail-closed recompute-and-compare doc to mirror for `nono receipt verify`**
(`crates/nono-cli/src/cli.rs:3505-3510`):
> "re-reads `audit-events.ndjson`, recomputes the per-event leaf hash, the hash-chain head, and
> the Merkle root... then fail-closes if any commitment... does not match."

---

### `crates/nono/src/sandbox/windows.rs` (MODIFIED) — new deny-ACE + reused mandatory label

**Analog: same file**, mandatory-label half is a pure copy (no new code needed beyond a call site);
DACL-deny half extends the existing `edit_dacl_for_sid` primitive with a new `ACCESS_MODE`.

**Mandatory label — reuse as-is** (`crates/nono/src/sandbox/windows.rs:1045`):
```rust
pub fn try_set_mandatory_label(path: &Path, mask: u32) -> Result<()> {
    // SDDL: "S:(ML;;<mask-hex>;;;LW)" via ConvertStringSecurityDescriptorToSecurityDescriptorW
    // + GetSecurityDescriptorSacl + SetNamedSecurityInfoW(LABEL_SECURITY_INFORMATION).
    // Fail-closed: any non-zero return -> NonoError::LabelApplyFailed.
}
```
For D-08's `NO_READ_UP` requirement on the sink, build the mask the same way
`label_mask_for_access_mode` does (`windows.rs:1004-1025`) — pass
`SYSTEM_MANDATORY_LABEL_NO_READ_UP | SYSTEM_MANDATORY_LABEL_NO_EXECUTE_UP` explicitly (the `Write`
arm's mask), since the receipt sink needs "no read-up" without needing "no write-up" (the
supervisor itself must still write receipts).

**The DACL-mutation primitive already generalized over `ACCESS_MODE` — extend, don't duplicate**
(`crates/nono/src/sandbox/windows.rs:1659-1663`):
```rust
fn edit_dacl_for_sid(
    path: &Path,
    sid: &str,
    access_mask: u32,
    access_mode: windows_sys::Win32::Security::Authorization::ACCESS_MODE,
    inheritance: u32,
) -> Result<()> {
    // ... GetNamedSecurityInfoW(DACL_SECURITY_INFORMATION) -> SetEntriesInAclW(SET_ACCESS/...)
    //     -> SetNamedSecurityInfoW, fail-closed at every step (NonoError::DaclApplyFailed).
}
```
Every existing caller passes `SET_ACCESS` (grant). **No `DENY_ACCESS` caller exists today** —
confirmed by reading all four public wrappers (`grant_sid_write_on_path:1800`,
`grant_sid_traverse_on_path:1846`, `grant_sid_read_on_path:1887`,
`grant_sid_read_attributes_on_path:1943`), every one importing only `SET_ACCESS`. D-08's DENY ACE
is genuinely new code, but it is a **thin sibling wrapper** — `deny_sid_on_path(path, sid,
access_mask)` calling `edit_dacl_for_sid(path, sid, access_mask, DENY_ACCESS, NO_INHERITANCE)` —
not a new ACL code path. Example grant wrapper to copy the doc/error shape from
(`crates/nono/src/sandbox/windows.rs:1800-1810`):
```rust
pub fn grant_sid_write_on_path(path: &Path, sid: &str, inheritable: bool) -> Result<()> {
    use windows_sys::Win32::Security::Authorization::SET_ACCESS;
    use windows_sys::Win32::Security::{CONTAINER_INHERIT_ACE, NO_INHERITANCE, OBJECT_INHERIT_ACE};
    let inheritance = if inheritable {
        OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE
    } else {
        NO_INHERITANCE
    };
    edit_dacl_for_sid(path, sid, SESSION_SID_WRITE_MASK, SET_ACCESS, inheritance)
}
```
**WRITE_OWNER gotcha (documented, must carry into the sink's own ACL-apply error path):**
`path_is_owned_by_current_user` (`windows.rs:1256`) exists precisely because
`SetNamedSecurityInfoW(..., DACL_SECURITY_INFORMATION, ...)` needs `WRITE_DAC`/`WRITE_OWNER` the
caller does not always implicitly hold (drive roots fail, `%USERPROFILE%`/`%TEMP%` succeed — see
MEMORY `feedback_windows_mandatory_label_write_owner.md`). Any sink-directory creation logic
should call this check (or accept its `NonoError::DaclApplyFailed`/`LabelApplyFailed` hint text
verbatim) rather than re-deriving the WRITE_OWNER story.

---

### `crates/nono/src/machine_policy.rs` (MODIFIED) — `require_receipts` (D-04)

**Analog: same file**, `RequiredLayersPolicy` (lines 135-230).

**The abort-vs-degrade split to extend, not reinvent** (`crates/nono/src/machine_policy.rs:7-19`):
```rust
//! # Fail-Secure Taxonomy (D-07)
//!
//! | Registry state | Return value |
//! | Key **absent** | `Ok(None)` — fall through to per-user config |
//! | Key **present but unconfigured** | `Ok(None)` — not enforcing (CR-02) |
//! | Key **present WITH ≥1 egress entry** | `Ok(Some(policy))` — enforcement active |
//! | Key **present but unreadable** | `Err(NonoError::PolicyLoadFailed)` |
//! | Key **present but malformed** | `Err(NonoError::PolicyLoadFailed)` |
//!
//! Once the HKLM key exists, any read or parse failure aborts. It is
//! never permissible to fall through ... when the key is present but
//! unreadable — that would be a fail-open vulnerability.
```
**`RequiredLayersPolicy`'s degrade-not-abort precedent — D-04's literal model**
(`crates/nono/src/machine_policy.rs:140-160`):
> "This file has two established, opposite-direction precedents for a present-but-broken registry
> value: the egress fields **abort** the whole policy read (D-07); `telemetry` instead
> **degrades** to `TelemetryConfig::default()` plus a stderr warning (D-14)... `required_layers`
> follows the telemetry (degrade-not-abort) lifecycle."

D-04's "require receipts fleet-wide" field should follow the **egress** (abort-on-unreadable)
lifecycle instead — the CONTEXT.md wording is explicit that a machine policy CAN make emitter
failure fail-closed, which is the strict end of this spectrum, not the degrade end. Model the new
field's read function after `read_required_layers` (`machine_policy.rs:734-757`, degrade template)
but flip its present-but-unreadable arm to `Err(...)` per the egress template
(`machine_policy.rs:7-19`) — write down explicitly which of the two precedents was chosen and why,
matching the file's own established documentation discipline.

---

### `crates/nono-cli/src/exec_strategy_windows/attestation.rs` (MODIFIED) — the census pass

**Analog: same file.** This is the core of Finding 1 — do not treat `attest_and_decide`'s return
value as the census.

**`RowVerdict` — already pure, already computed per row, currently discarded**
(`crates/nono-cli/src/exec_strategy_windows/attestation.rs:365-403`):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RowVerdict {
    status: LayerAttestationStatus,
    partially_established: bool,
}
```

**`classify_row` — the exact function to call once per row, non-short-circuiting**
(`crates/nono-cli/src/exec_strategy_windows/attestation.rs:405-420`, full body already read):
```rust
fn classify_row(
    entry: &layer_registry::LayerRegistryEntry,
    input: &AttestationInput,
) -> RowVerdict {
    let expected = entry
        .expectancy
        .iter()
        .find(|arm| {
            arm.entry_path == input.entry_path && token_arm_matches(input.token_arm, arm.token_arm)
        })
        .map(|arm| arm.expected)
        .unwrap_or(false);
    if !expected {
        return RowVerdict::plain(LayerAttestationStatus::NotApplicable);
    }
    match entry.probe { /* LiveTokenOrJobQuery | ConfirmedByEnforcingComponentReport |
                            ConfiguredOnly | NotApplicable */ }
}
```
`classify_row` has **no OS side effects beyond read-only probes** (Assumption A3, verified by
inspection of every match arm) — safe to call a second time in a new non-early-returning loop
alongside `decide_from_entries`'s existing loop, per Pitfall 4's guidance: keep the DECISION
function's control flow byte-identical; add a SEPARATE pure census-building pass.

**`decide_from_entries`'s current early-return shape (what a census pass must NOT inherit)**
(`crates/nono-cli/src/exec_strategy_windows/attestation.rs:503-527`, the discard point):
```rust
fn decide_from_entries(
    entries: &[layer_registry::LayerRegistryEntry],
    input: &AttestationInput,
    required_names: &HashSet<String>,
) -> AttestationDecision {
    let mut downgraded: Vec<layer_registry::LayerId> = Vec::new();
    for entry in entries {
        let verdict = classify_row(entry, input);
        let status = verdict.status;
        // ... every branch below either `continue`s (verdict discarded) or
        // `return AttestationDecision::Abort { .. }` (loop terminates,
        // every subsequent row's verdict never computed).
        if status == LayerAttestationStatus::Confirmed && !verdict.partially_established {
            continue;
        }
        // ...
    }
    if downgraded.is_empty() { AttestationDecision::Proceed }
    else { AttestationDecision::ProceedDowngraded { downgraded } }
}
```
**Pattern to build:** a sibling function (e.g. `census_from_entries`) with the identical `for entry
in entries { let verdict = classify_row(entry, input); ... }` shape but NO `return` inside the
loop — accumulate `Vec<(LayerId, LayerAttestationStatus, bool)>` for all 13 rows unconditionally,
call it once, alongside the untouched `decide_from_entries` call, from
`apply_startup_attestation_gate`.

---

### `crates/nono-cli/src/agent_daemon/launch.rs` (MODIFIED) — collect-all-then-decide (D-26)

**Analog: same file**, the 5-of-13 early-return chain to restructure.

**Exact shape being replaced** (`crates/nono-cli/src/agent_daemon/launch.rs:1417-1500`, full body
read):
```rust
fn daemon_attest_and_decide(
    process: HANDLE,
    job: HANDLE,
    expected_package_sid: &str,
    dacl_guard_applied: bool,
    network_scoping_required: bool,
    wfp_filters_installed: bool,
    ancestor_traverse_applied: bool,
) -> DaemonAttestationDecision {
    use nono::attestation::{probe_app_container_sid, probe_in_job, LayerAttestationStatus};

    let app_container_confirmed = match probe_app_container_sid(process) {
        Ok(Some(sid)) => sid.eq_ignore_ascii_case(expected_package_sid),
        Ok(None) | Err(_) => false,
    };
    if !app_container_confirmed {
        return DaemonAttestationDecision::Abort {
            layer: "AppContainerProfile",
            status: LayerAttestationStatus::Unconfirmed,
        };                                              // <-- early return #1 (line ~1435)
    }

    let job_confirmed = matches!(probe_in_job(process, job), Ok(true));
    if !job_confirmed {
        return DaemonAttestationDecision::Abort { layer: "JobObjectContainment", .. }; // #2 (~1446)
    }

    if network_scoping_required && !wfp_filters_installed {
        return DaemonAttestationDecision::Abort { layer: "WfpEgressFilters", .. }; // #3 (~1455)
    }

    if !dacl_guard_applied {
        return DaemonAttestationDecision::Abort { layer: "DaclPackageSidGrant", .. }; // #4 (~1470)
    }

    if !ancestor_traverse_applied {
        tracing::warn!(/* ... proceeds, this row is under-granting not under-confining */);
    }

    DaemonAttestationDecision::Proceed  // only reachable if ALL FOUR aborts above didn't fire
}
```
**Restructure to (per D-26, mechanical, not a new state):** probe every one of the 5 modelled
layers unconditionally first (into local `bool`/`LayerAttestationStatus` values, same as the CLI's
`classify_row` shape), THEN apply the identical abort-precedence decision at the end — so
`DaemonAttestationDecision`'s outcome for every existing test input is provably unchanged (D-26's
"provably unchanged" requirement), while a parallel census-building pass can also see the state of
`JobObjectContainment`/`WfpEgressFilters`/etc. even when `AppContainerProfile` would have aborted
under the old code. The daemon-local expectancy table for the 8 unmodelled `LayerId`s (per Finding
1c/Open Question 3, RESEARCH's recommended resolution) should be a **hardcoded, drift-guarded
`NotApplicable` set**, not a widened `#[path]` include — see the cross-check test pattern below.

**`DaemonAttestationDecision`'s own doc comment (already documents the two-state constraint —
copy this framing, do not violate it):**
```rust
// NOTE for reviewers: `DaemonAttestationDecision` has exactly two
// variants (`Proceed` / `Abort`) and `daemon_decision_enum_variants`
// pins that list, so there is no `ProceedDowngraded` to classify into.
```
D-16/D-37 point 2 requires the daemon stay two-state; the census (four-state, per-row) is separate
from the decision (two-state, aggregate) — do not merge them into one return type.

---

### `crates/nono-shell-broker/src/main.rs` (MODIFIED) — extended wire contract (D-27)

**Analog: same file**, `BROKER_ATTESTABLE_LAYERS` + `broker_resume_gate`.

**Current narrow knowledge to extend, not replace** (`crates/nono-shell-broker/src/main.rs:321`):
```rust
const BROKER_ATTESTABLE_LAYERS: &[&str] = &["AppContainerProfile", "MandatoryIntegrityLabel"];
```

**`broker_resume_gate`'s fail-closed shape to model the new `(EntryPath::Broker, ...)`
`NotApplicable` consumption on** (`crates/nono-shell-broker/src/main.rs:369-453`, full body read):
```rust
fn broker_resume_gate(
    required_layers_raw: &str,
    app_container_probe: Option<NonoResult<Option<String>>>,
    expected_app_container_sid: Option<&str>,
    integrity_probe: NonoResult<u32>,
) -> std::result::Result<(), String> {
    let required_layers: Vec<&str> = required_layers_raw
        .split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    if required_layers.is_empty() {
        return Err("... refusing to resume an unattested child (fail-closed)".to_string());
    }
    for name in &required_layers {
        if !BROKER_ATTESTABLE_LAYERS.contains(name) {
            return Err(format!("required layer {name:?} cannot be attested ..."));
        }
    }
    // ... AppContainerProfile arm (fail-closed on None/mismatch/probe-err)
    // ... MandatoryIntegrityLabel arm (fail-closed on RID above LOW, or probe-err)
    Ok(())
}
```
D-27's extension: a **second** env var (or a widened value on the existing
`BROKER_REQUIRED_LAYERS_ENV_VAR`) that names the `(EntryPath::Broker, ...)` `NotApplicable` rows,
so the broker's own receipt-census pass can classify all 13 rows without a third hardcoded copy of
registry knowledge. The compatibility guard: a broker handed an unrecognized or absent row set
must fail toward `Unconfirmed`, mirroring `broker_resume_gate`'s existing "unrecognized required
layer -> fail-closed refusal" arm (lines 387-393) rather than silently omitting rows.

---

### `crates/nono-cli/src/exec_strategy_windows/attestation.rs` (MODIFIED, wire-contract producer half)

**Analog: same file**, `BROKER_REQUIRED_LAYERS_ENV_VAR` + `required_layers_for_broker`.

**The existing env-var contract to extend** (`crates/nono-cli/src/exec_strategy_windows/attestation.rs:104-117`):
```rust
/// The wire contract nono-cli uses to tell `nono-shell-broker.exe` which
/// layers it must itself confirm active before resuming its own suspended
/// grandchild ...
/// Value shape: a comma-separated list of `LayerId` `Debug`-format names
/// (e.g. `"AppContainerProfile"`), naming exactly the registry rows whose
/// `(EntryPath::Broker, ...)` expectancy is `expected: true` AND whose
/// `outcome` is `ContractOutcome::Abort` — see [`required_layers_for_broker`].
pub(crate) const BROKER_REQUIRED_LAYERS_ENV_VAR: &str = "NONO_BROKER_REQUIRED_LAYERS";
```
D-27's widening keeps this env var's *meaning* unchanged (still names the must-be-`Confirmed`-
or-terminate rows) and adds a sibling channel naming the `NotApplicable` rows for the receipt
census — same producer function (`required_layers_for_broker`, `:718+`), same "the CLI is the
single source of truth, the broker never invents policy" framing (D-02's wording, quoted verbatim
in the module doc at line 115-116).

---

### `crates/nono-cli/src/exec_strategy_windows/launch.rs` (MODIFIED) — the D-03 write point

**Analog: same file.** No launch-path restructuring — the receipt write is a fourth thing to do at
branches that already exist.

**The call site position (verified, matches CONTEXT.md's line citations exactly)**
(`crates/nono-cli/src/exec_strategy_windows/launch.rs:2565-2586`, per RESEARCH.md's verified read):
```rust
let wfp_preconfirmed = derive_wfp_preconfirmed(network_enforcement);
if let Err(err) = apply_startup_attestation_gate(
    process.raw(), containment.job, layer_registry::EntryPath::DirectCli,
    Some(arm), wfp_preconfirmed, applied_layers, config.session_sid.as_deref(), session_id,
) {
    terminate_suspended_process(process.raw(), &format!("startup self-attestation failed: {err}"));
    return Err(err);       // <-- "refused" outcome (D-02): receipt still gets written
}
resume_contained_process(process.raw(), thread.raw())?;   // <-- "ran" outcome: write completes BEFORE this
```

**The three existing branches inside the gate itself — the receipt write slots into each**
(`crates/nono-cli/src/exec_strategy_windows/launch.rs:1552-1675`, full body read):
```rust
fn apply_startup_attestation_gate(/* ... */) -> Result<()> {
    let input = attestation::AttestationInput { /* ... */ };
    match attestation::attest_and_decide(input)? {
        attestation::AttestationDecision::Proceed => Ok(()),                    // branch 1: ran, clean
        attestation::AttestationDecision::Abort { layer, status } => {          // branch 2: refused
            Err(NonoError::LayerAttestationFailed {
                layer: format!("{layer:?}"), reason: format!("{status:?}"),
            })
        }
        attestation::AttestationDecision::ProceedDowngraded { downgraded } => { // branch 3: ran, downgraded
            // ... existing D-27 banner / audit-event emission already happens HERE
            Ok(())
        }
    }
}
```
The receipt write (built from the parallel census pass, see the `attestation.rs` mapping above)
belongs at the top of `apply_startup_attestation_gate`, computed once and then branched into
`ran`/`refused` alongside the existing `Ok`/`Err` return, so both outcomes reuse one census build.

---

### `crates/nono-cli/src/cli.rs` (MODIFIED) — `ReceiptCommands` (D-10)

**Analog: same file**, `AuditCommands`.

**The exact shape to mirror** (`crates/nono-cli/src/cli.rs:3497-3592`, full body read):
```rust
#[derive(Subcommand, Debug)]
pub enum AuditCommands {
    /// List all sandboxed sessions
    List(AuditListArgs),
    /// Show audit details for a session
    Show(AuditShowArgs),
    /// Verify the cryptographic integrity of an audit session
    Verify(AuditVerifyArgs),
    /// Remove old audit sessions
    Cleanup(AuditCleanupArgs),
}

#[derive(Parser, Debug)]
#[command(disable_help_flag = true)]
pub struct AuditVerifyArgs {
    /// Session ID to verify (e.g., 20260214-143022-12345)
    pub session_id: String,
    #[arg(long, value_name = "PATH")]
    pub public_key_file: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
    #[arg(long, short = 'h', action = clap::ArgAction::Help, help_heading = "OPTIONS")]
    pub help: Option<bool>,
}
```
`ReceiptCommands` mirrors this with `List | Show | Verify` (Cleanup optional per the discretion
item on retention). `ReceiptVerifyArgs` drops `public_key_file` (no asymmetric verification per
D-09) but keeps `session_id` + `json` + `help`.

**Confirmed cross-platform, no `#[cfg(target_os)]` gate on the command surface** (RESEARCH.md's
verified read of `cli.rs:3490-3592`) — `nono receipt` should follow the identical pattern: the
clap definitions compile on every target; the underlying data is empty/absent on non-Windows per
D-18. This is exactly why D-23 flags `cli.rs` edits as being in the cross-target clippy blast
radius — plan for both `cross clippy --target x86_64-unknown-linux-gnu` and `cargo-zigbuild clippy
--target x86_64-apple-darwin` to run clean on the new enum/structs even though their *behavior* is
Windows-only.

**Top-level `Commands` entry point to mirror** (`crates/nono-cli/src/cli.rs:907`):
```rust
    Audit(AuditArgs),
```

---

### `crates/nono-cli/src/receipt_commands.rs` (NEW) + dispatch wiring

**Analog:** `crates/nono-cli/src/audit_commands.rs` + `crates/nono-cli/src/app_runtime.rs`

**Dispatch module shape to copy** (`crates/nono-cli/src/audit_commands.rs:1-36`):
```rust
//! Audit subcommand implementations
use crate::cli::{AuditArgs, AuditCleanupArgs, AuditCommands, AuditListArgs, AuditShowArgs, AuditVerifyArgs};
// ...
pub fn run_audit(args: AuditArgs) -> Result<()> {
    match args.command {
        AuditCommands::List(args) => cmd_list(args),
        AuditCommands::Show(args) => cmd_show(args),
        AuditCommands::Verify(args) => cmd_verify(args),
        AuditCommands::Cleanup(args) => cmd_cleanup(args),
    }
}
```

**Top-level dispatch wiring** (`crates/nono-cli/src/app_runtime.rs:109-110`):
```rust
Commands::Audit(args) => {
    run_command_with_update(update_handle, silent, || audit_commands::run_audit(args))
}
```
`ReceiptCommands` gets an identical `Commands::Receipt(args) => run_command_with_update(...,
|| receipt_commands::run_receipt(args))` arm, in the same match block.

---

## Shared Patterns

### The house source-scan idiom (D-14's type-allowlist scan and census-completeness meta-test)

**Source:** `crates/nono-cli/tests/layer_registry_selfcheck.rs`, `layer_registry_meta_test.rs`
**Apply to:** every new discovery-based test in this phase

```rust
// Source: crates/nono-cli/tests/layer_registry_selfcheck.rs:34-65 (in-tree, house pattern)
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap_or_else(|| panic!("CARGO_MANIFEST_DIR {} has no parent", manifest_dir().display()))
        .parent()
        .unwrap_or_else(|| panic!(/* ... */))
        .to_path_buf()
}
fn read_layer_registry() -> String {
    let path = manifest_dir().join("src").join("exec_strategy_windows").join("layer_registry.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}
```
Rules confirmed by reading the module docs of all three test files
(`layer_registry_selfcheck.rs:1-28`, `layer_registry_meta_test.rs:1-36`,
`layer_force_unavailable.rs:1-43`):
- **No `regex` crate.** No `include_str!` for the file being scanned fresh (that would compile the
  scanned text into the test binary, defeating "discovers, never assumes"); `include_str!` IS used
  legitimately for cross-referencing a SECOND file's text within a `#[cfg(test)]` module
  (`agent_daemon/launch.rs`'s daemon/CLI enum cross-check does this — see below), a different use
  case from the primary "read your own scan target fresh" rule.
- Every test is discovery-based: it parses the target's current text on every run rather than
  hardcoding the 13 `LayerId` names.
- Word-boundary-exact symbol matching (`content_defines_symbol`,
  `layer_registry_selfcheck.rs:311-341`) — handles PascalCase↔snake_case, `Type::method`
  qualification, and explicitly rejects doc-comment mentions and prefix-preserving renames (its
  own negative tests: `content_defines_symbol_rejects_a_doc_comment_mention`,
  `content_defines_symbol_rejects_a_prefix_preserving_rename`,
  `content_defines_symbol_distinguishes_same_named_methods_in_different_impls`).

**D-14's type-allowlist scan is the one genuinely NEW mechanism** — no existing scan parses struct
FIELD TYPES; every existing scan matches `fn` definitions or enum variant names
(`layer_registry_selfcheck.rs`'s `call_sites_regions`/`content_defines_symbol`,
`layer_registry_meta_test.rs`'s `MANUALLY_VERIFIED` allow-list check). Budget this as new,
regex-free, line-by-line parsing logic on the `pub field_name: Type,` shape within the receipt
struct's brace-delimited body, located the same way `call_sites_regions` locates other structural
regions.

### The daemon/CLI decision-shape cross-check (reusable for D-27's broker wire-contract guard too)

**Source:** `crates/nono-cli/src/agent_daemon/launch.rs:2450-2574`
**Apply to:** the daemon-local expectancy table (Finding 1c) AND the broker's widened wire
contract (D-27)

```rust
// Source: crates/nono-cli/src/agent_daemon/launch.rs:2539-2574 (in-tree)
#[test]
fn every_daemon_variant_is_in_the_cli_variant_set_or_a_documented_divergence() {
    let daemon_src = include_str!("launch.rs");
    let daemon_variants = parse_enum_variant_names(
        daemon_enum_segment(daemon_src), "enum DaemonAttestationDecision {",
    );
    let cli_src = include_str!("../exec_strategy_windows/attestation.rs");
    let cli_variants = parse_enum_variant_names(cli_src, "enum AttestationDecision {");
    let documented_divergence: &[&str] = &["ProceedDowngraded"];
    for daemon_variant in &daemon_variants {
        assert!(cli_variants.contains(daemon_variant), "... must stay a subset ...");
    }
    for cli_variant in &cli_variants {
        let is_mirrored = daemon_variants.contains(cli_variant);
        let is_documented = documented_divergence.contains(&cli_variant.as_str());
        assert!(is_mirrored || is_documented, "... undocumented divergence ...");
    }
}
```
This is a **cross-FILE, `include_str!`-based** discovery test (a legitimate second use of
`include_str!`, distinct from the "never for the file being freshly scanned" rule above — this
reads a SIBLING file's source as a compile-time text embed, not a module/link dependency;
`nono-agentd` never `#[path]`-includes `exec_strategy_windows/` in a real build, so this remains
`#[cfg(test)]`-only text, not a compilation dependency). Reuse this exact shape for: (1) keeping
the daemon's local 8-row `NotApplicable` expectancy table honest against
`layer_registry.rs`'s `(EntryPath::Daemon, None)` cells, and (2) D-27's broker wire-contract
compatibility guard (source-scan both `nono-cli`'s producer and `nono-shell-broker`'s consumer to
confirm the row-name sets agree).

### Producing non-`Confirmed` census rows without mocking the OS

**Source:** `crates/nono-cli/tests/layer_force_unavailable.rs`
**Apply to:** `receipt_sentinel_roundtrip.rs` and any test needing a receipt with `Unconfirmed`/
`NotApplicable` rows

The file documents its own constraint precisely (`layer_force_unavailable.rs:7-20`): `nono-cli`
has no `[lib]` target, so `tests/*.rs` cannot call `pub(crate)` setters directly — only via
subprocess spawn (`env!("CARGO_BIN_EXE_nono")`) reading a `pub(crate)`-only force-unavailable seam
(static + setter + getter) gated behind `#[cfg(feature = "layer-fault-injection")]` (compiled out
of release builds, D-30). New receipt-focused tests needing a non-`Confirmed` row should reuse
this seam rather than inventing a new fault-injection mechanism.

### Machine-policy abort-vs-degrade split

**Source:** `crates/nono/src/machine_policy.rs`
**Apply to:** the new `require_receipts` field (D-04)

See the `machine_policy.rs` pattern assignment above — two established, opposite-direction
precedents already coexist in this one file (egress fields abort; `required_layers`/`telemetry`
degrade). D-04 must explicitly choose one (egress/abort-on-unreadable, matching "can make it
fail-closed") and document the choice inline the same way the file already documents both existing
precedents, rather than leaving the choice implicit.

---

## No Analog Found

None. Every file this phase touches or creates has a strong in-tree analog — this is a brownfield
phase building alongside Phase 117's freshly-hardened attestation machinery, and the research
(`118-RESEARCH.md` Finding 1/2) already confirmed every needed primitive (HMAC/keyless chaining,
JSON/ndjson serialization, clap subcommands, Win32 ACL/label calls) exists in-tree with a usage
precedent. The two items genuinely lacking a direct analog are noted inline above rather than
listed as "no analog": (1) the type-FIELD-parsing half of D-14's source scan (existing scans parse
`fn`/variant names, not struct field types); (2) the DENY-ACE half of D-08 (existing
`edit_dacl_for_sid` callers all pass `SET_ACCESS`) — both are small, mechanical extensions of an
existing pattern, not new subsystems.

## Metadata

**Analog search scope:** `crates/nono/src/` (attestation.rs, audit.rs, machine_policy.rs,
sandbox/windows.rs, undo/types.rs, lib.rs), `crates/nono-cli/src/` (exec_strategy_windows/
attestation.rs + launch.rs + attestation_downgrade_event.rs, agent_daemon/launch.rs,
telemetry/mod.rs, cli.rs, audit_commands.rs, app_runtime.rs, bin/nono-agentd.rs),
`crates/nono-shell-broker/src/main.rs` + `Cargo.toml`, `crates/nono-cli/tests/` (
layer_registry_selfcheck.rs, layer_registry_meta_test.rs, layer_force_unavailable.rs),
`crates/nono-cli/Cargo.toml`.
**Files scanned:** 18 read/grepped directly (all symbol- and line-verified against the live tree,
no inference from summaries).
**Pattern extraction date:** 2026-08-16
