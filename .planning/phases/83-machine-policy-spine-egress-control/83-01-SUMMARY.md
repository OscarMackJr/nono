---
phase: 83-machine-policy-spine-egress-control
plan: "01"
subsystem: nono-core-library
tags: [machine-policy, registry, egress-control, fail-secure, windows, serde]
depends_on: []
dependency_graph:
  requires: []
  provides:
    - MachineEgressPolicy platform-neutral serde type (crates/nono)
    - read_machine_egress_policy() fail-secure Windows reader + non-Windows stub
    - NonoError::PolicyLoadFailed { reason } variant
    - sc4_dns_component_matrix EGRESS-03 test contract
  affects:
    - crates/nono/src/lib.rs (re-exports)
    - crates/nono/src/error.rs (new error variant)
    - crates/nono/src/net_filter.rs (new test)
    - Cargo.lock (winreg 0.56.0 locked)
tech_stack:
  added:
    - winreg 0.56.0 (Windows-only, operator-approved 2026-06-18)
  patterns:
    - fail-secure absent/unreadable/malformed taxonomy (D-07)
    - 64-bit registry view via KEY_WOW64_64KEY (D-09)
    - N×REG_SZ ADMX list subkey enumeration (D-13 Option A)
    - platform-neutral serde type with cfg-gated reader (Pitfall 5)
key_files:
  created:
    - crates/nono/src/machine_policy.rs
  modified:
    - crates/nono/src/error.rs
    - crates/nono/src/lib.rs
    - crates/nono/src/net_filter.rs
    - crates/nono/Cargo.toml
    - Cargo.lock
decisions:
  - "D-10: winreg 0.56 added Windows-only under [target.'cfg(windows)'.dependencies]; operator-verified provenance 2026-06-18"
  - "D-07: absent→Ok(None) / present-but-broken→Err(PolicyLoadFailed); implemented via raw_os_error()==2 check"
  - "D-09: KEY_WOW64_64KEY on every open_subkey_with_flags call in the reader"
  - "D-13: Option A (enumerate subkey N×REG_SZ via enum_values()) matches shipped Phase-82 ADMX <list> shape"
  - "D-14: existing leading-dot ends_with+len> form retained; sc4_dns_component_matrix codifies the EGRESS-03 contract"
metrics:
  duration: "7m"
  completed_date: "2026-06-18"
  tasks_completed: 3
  files_changed: 6
---

# Phase 83 Plan 01: Machine Policy Spine + Egress Control Foundation Summary

**One-liner:** Platform-neutral `MachineEgressPolicy` serde type + fail-secure `winreg` HKLM reader (absent→`Ok(None)` / unreadable-or-malformed→`Err(PolicyLoadFailed)`) + named EGRESS-03 DNS-component test matrix codifying D-14.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Verify winreg provenance (operator gate) | N/A — operator-approved pre-execution | — |
| 2 | MachineEgressPolicy type + PolicyLoadFailed + winreg reader | `524f7684` | Cargo.toml, error.rs, lib.rs, machine_policy.rs, Cargo.lock |
| 3 | SC-4 DNS-component reject-matrix test | `929a6c5f` | net_filter.rs |

## What Was Built

### Task 1: winreg Provenance Gate (Operator-Approved)

Operator confirmed before execution: `winreg` @ crates.io resolves to `github.com/gentoo90/winreg-rs`, latest published `0.56.0` (2026-03-14), 179,777,376 all-time downloads, established version history (0.53→0.54→0.55→0.56), MIT license. Dependency add was unblocked.

### Task 2: MachineEgressPolicy + PolicyLoadFailed + winreg Reader

**`crates/nono/Cargo.toml`** — `winreg = "0.56"` added **only** under `[target.'cfg(target_os = "windows")'.dependencies]` with an operator-approval comment (D-10; Pitfall 5 prevention).

**`crates/nono/src/error.rs`** — New `NonoError::PolicyLoadFailed { reason: String }` struct variant with `#[error("Machine policy load failed: {reason}")]` and a doc comment explicitly stating the fail-secure contract: once the HKLM key exists, ANY read/parse error aborts (D-07).

**`crates/nono/src/machine_policy.rs`** (new file) — Core deliverable:
- `MachineEgressPolicy` struct: `Debug + Clone + PartialEq + Eq + Serialize + Deserialize + Default`; fields `allowed_suffixes`, `allowed_hosts`, `preset_tokens` (all `Vec<String>`); no Windows types (Pitfall 5). `raw_allowlist()` method returns suffixes+hosts flat list; preset token expansion is CLI-layer concern (Plan 03).
- Windows reader (`#[cfg(target_os = "windows")]`): opens `HKLM\SOFTWARE\Policies\nono` with `KEY_READ | KEY_WOW64_64KEY` (D-09); maps `raw_os_error()==2` to `Ok(None)` (absent D-07); all other errors → `Err(PolicyLoadFailed)` (unreadable D-07). `read_list_subkey()` enumerates N×REG_SZ values from `AllowedSuffixes\` and `AllowedHosts\` subkeys matching the ADMX `<list>` shape (D-13 Option A); wrong REG type → `Err` with reason (malformed D-07). No `.unwrap_or`/`.ok()` anywhere in production path (Pitfall 3).
- Non-Windows stub (`#[cfg(not(target_os = "windows"))]`): returns `Ok(None)` unconditionally.
- `windows_reader` submodule makes `read_list_subkey` and `parse_policy` `pub(super)` for test access without exposing them publicly.
- 9 tests: empty policy, serde round-trip, `raw_allowlist` ordering, non-Windows stub contract, `PolicyLoadFailed` display/match/propagate, Windows HKCU integration (REG_SZ enumeration + wrong-type abort).

**`crates/nono/src/lib.rs`** — `pub mod machine_policy;` + `pub use machine_policy::{read_machine_egress_policy, MachineEgressPolicy};`

### Task 3: sc4_dns_component_matrix (EGRESS-03)

Added `sc4_dns_component_matrix` test to the existing `#[cfg(test)] mod tests` in `net_filter.rs`. Uses `HostFilter::new_strict(&["*.anthropic.com".to_string()])` and `public_ip()` helper. Asserts all four EGRESS-03 cases:
- `api.anthropic.com` → allowed (legitimate subdomain)
- `anthropic.com` → denied (bare domain — wildcard must not match parent)
- `evilanthropic.com` → denied (no leading-dot boundary)
- `anthropic.com.evil.com` → denied (suffix injection)

Existing leading-dot `ends_with` + `len >` form in `check_host` already passes all four cases (D-14 verified, no matcher rebuild needed).

## Deviations from Plan

None — plan executed exactly as written. The winreg API required one deviation fix during implementation:

**[Rule 1 - Bug] Fixed winreg API call: `String::from_reg_value` needs `FromRegValue` trait in scope**
- **Found during:** Task 2 compilation (RED phase)
- **Issue:** `String::from_reg_value(&val)` failed to compile — `FromRegValue` trait not in scope
- **Fix:** Added `use winreg::types::FromRegValue;` inside `reg_value_to_string()`
- **Files modified:** `crates/nono/src/machine_policy.rs`
- Resolved inline; no additional commit needed.

**[Rule 1 - Bug] Fixed winreg 0.56 `RegValue.bytes` field type: `Cow<'_, [u8]>` not `Vec<u8>`**
- **Found during:** Task 2 compilation (Windows integration test)
- **Issue:** Test constructed `RegValue { bytes: vec![...], ... }` but winreg 0.56 changed the type to `Cow<'_, [u8]>`
- **Fix:** Changed to `std::borrow::Cow::Owned(vec![...])`
- **Files modified:** `crates/nono/src/machine_policy.rs` (test module only)
- Resolved inline.

## Verification Results

```
cargo test -p nono machine_policy
  9 passed; 0 failed  (serde, PolicyLoadFailed, non-Windows stub, Windows HKCU integration)

cargo test -p nono net_filter::tests::sc4_dns_component_matrix
  1 passed; 0 failed

grep -n winreg crates/nono/Cargo.toml
  Lines 64-67: under [target.'cfg(target_os = "windows")'.dependencies] ONLY
```

## Known Stubs

None — all deliverables are fully wired. The `preset_tokens` field carries raw tokens; expansion is intentionally deferred to Plan 03 (CLI layer owns `policy.json`).

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: registry-read | crates/nono/src/machine_policy.rs | New HKLM read surface — all D-07 mitigations applied (absent/unreadable/malformed taxonomy; KEY_WOW64_64KEY; no unwrap_or on read path) |

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| `crates/nono/src/machine_policy.rs` exists | FOUND |
| `crates/nono/src/error.rs` contains `PolicyLoadFailed` | FOUND (line 252) |
| `crates/nono/src/lib.rs` contains `pub use machine_policy::` | FOUND (line 82) |
| `crates/nono/src/net_filter.rs` contains `sc4_dns_component_matrix` | FOUND (line 492) |
| Commit `524f7684` (Task 2) exists | FOUND |
| Commit `929a6c5f` (Task 3) exists | FOUND |
| `winreg` only under `[target.'cfg(windows)'.dependencies]` | VERIFIED |
| All 9 machine_policy tests pass | PASSED |
| sc4_dns_component_matrix test passes | PASSED |
