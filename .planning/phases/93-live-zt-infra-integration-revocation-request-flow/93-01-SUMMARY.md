---
phase: 93-live-zt-infra-integration-revocation-request-flow
plan: "01"
subsystem: security
tags: [zt-infra, ecdsa, hklm, winreg, pyo3, override, revocation]

requires:
  - phase: 92-signed-policy-overrides-nono-cli-python-bilateral-handshake
    provides: "verify_override_impl with test-injected pubkey_der; OverrideErrorKind enum; NonoOverrideError PyO3 exception"

provides:
  - "fail-secure HKLM trust-root reader (override_trust.rs) sourcing KMS pubkeys + ARN allowlist from HKLM\\SOFTWARE\\Policies\\nono\\Override"
  - "LiveRevoked and LiveUnavailable OverrideErrorKind variants (D-02) with 1:1 as_str() -> EventID codes"
  - "Process-wide VK_CACHE (LazyLock<Mutex<HashMap<String,Vec<u8>>>>) keyed by key_id (D-06)"
  - "verify_override_production() entry point sourcing trust from policy, closing VFY-03a [BLOCKING-93] seam"
  - "PyO3 read_override_trust_py registration in _nono_py module"
  - "winreg 0.56 promoted to direct dep in nono-py Cargo.toml (target-gated Windows)"

affects:
  - "93-02 through 93-06 plans (Wave 2-3 live arm and CLI audit-emit)"
  - "plans that call verify_override — now have a policy-sourced production path"

tech-stack:
  added:
    - "winreg 0.56 (promoted from transitive dep to direct dep in nono-py/Cargo.toml)"
  patterns:
    - "Fail-secure HKLM taxonomy: absent sub-key -> Ok(empty/fail-closed); wrong REG type -> Err; present-but-unreadable -> Err (never fall-through to permit)"
    - "TDD RED-GREEN cycle: test(93-01) commit f930123 (RED) then feat(93-01) commit 39cac71 (GREEN)"
    - "Forward-declaration discipline: production construction sites in later plans; #[allow(dead_code)] + removal-note for plan that adds call site"
    - "Backward-compat test-injection: verify_override_impl retains pubkey_der parameter for Phase 91/92 test vectors; production path uses policy-sourced lookup"

key-files:
  created:
    - "C:/Users/OMack/nono-py/src/override_trust.rs"
  modified:
    - "C:/Users/OMack/nono-py/src/override.rs"
    - "C:/Users/OMack/nono-py/src/lib.rs"
    - "C:/Users/OMack/nono-py/Cargo.toml"
    - "C:/Users/OMack/nono-py/src/windows_confined_run.rs"

key-decisions:
  - "Non-Windows stub for read_override_trust returns Ok(OverrideTrust::empty()) — same fail-closed shape as absent sub-key; downstream deny logic fires on empty ARN/pubkey sets (D-05)"
  - "winreg promoted from transitive to direct dep (same operator-approved 0.56) to make the dependency explicit in nono-py; no new external package introduced"
  - "VK_CACHE stores DER bytes (Vec<u8>) not VerificationKey — VerificationKey is not Clone; DER is cheap to re-parse on cache miss"
  - "verify_override_impl retains test-injected pubkey_der param for Phase 91/92 backward-compat; verify_override_production() is the new policy-sourced entry point"
  - "LiveRevoked/LiveUnavailable variants carry #[expect(dead_code)] citing Plan 02 as construction site — not suppressed wholesale, attribution tracked per-variant"
  - "Pre-existing clippy violations in windows_confined_run.rs (collapsible_if + too_many_arguments) fixed as Rule 1 deviations to unblock -D warnings gate"

patterns-established:
  - "Pattern: override_trust::read_override_trust() as authoritative trust root (HKLM spine, D-05)"
  - "Pattern: VK_CACHE lookup-then-policy chain — cache miss falls to HKLM read, absent key_id -> KeyNotAllowlisted (fail-closed)"

requirements-completed: [ZTL-01, VFY-01]

duration: "~90min"
completed: "2026-06-22"
---

# Phase 93 Plan 01: Live ZT-Infra Trust Foundation Summary

**Fail-secure HKLM trust-root reader + LiveRevoked/LiveUnavailable error kinds + per-key_id VK_CACHE, closing VFY-03a [BLOCKING-93] in nono-py ahead of Wave 2 live-arm Python**

## Performance

- **Duration:** ~90 min
- **Started:** 2026-06-22
- **Completed:** 2026-06-22
- **Tasks:** 3 (plus TDD RED commit as step 0)
- **Files modified:** 5

## Accomplishments

- `override_trust.rs` (new, 279 lines): fail-secure Windows HKLM reader mirroring `machine_policy.rs` taxonomy exactly (`KEY_READ | KEY_WOW64_64KEY`, absent-sub-key = `Ok(empty)`, wrong-type/unreadable = `Err`). Reads `Override\AllowedKeyArns\` (ADMX list of REG_SZ ARN values) and `Override\KmsPublicKeys\` (named values: key_id → base64-DER REG_SZ). Non-Windows stub returns `Ok(OverrideTrust::empty())` — same fail-closed shape.
- `OverrideErrorKind` extended with `LiveRevoked` (`as_str() == "LiveRevoked"` → EventID 10010 REVOKED) and `LiveUnavailable` (`as_str() == "LiveUnavailable"` → EventID 10008 REJECTED), providing the 1:1 kind→EventID contract for Wave 2 Python and CLI audit-emit.
- `VK_CACHE` (`LazyLock<Mutex<HashMap<String, Vec<u8>>>>`) added to `override.rs`; `lookup_pubkey_der_from_policy(key_id)` implements cache-miss → HKLM read → absent key_id = `Err(KeyNotAllowlisted)` (fail-closed, D-05).
- `verify_override_production()` is the new policy-sourced public entry point (Steps 0a-0c from policy, then delegates to `verify_override_impl`). Existing test vectors in Phase 91/92 call `verify_override_impl` with injected `pubkey_der` and are unaffected.
- `[BLOCKING-93]` doc-comment at the former VFY-03a seam updated to record closure.
- `lib.rs` registers `read_override_trust_py` in the `_nono_py` PyO3 module.
- `Cargo.toml` promotes `winreg = "0.56"` to a direct dep (target-gated Windows).

## Files Created/Modified

- `C:/Users/OMack/nono-py/src/override_trust.rs` (NEW) — 279 lines; `windows_reader` mod + `OverrideTrust` struct + non-Windows stub; PyO3 wrapper `read_override_trust_py`
- `C:/Users/OMack/nono-py/src/override.rs` (MODIFIED) — +241 lines net (Task 1 kinds + Task 3 VK_CACHE + lookup + require_arn_allowlist_from_policy + verify_override_production + 2 test modules)
- `C:/Users/OMack/nono-py/src/lib.rs` (MODIFIED) — added `mod override_trust` declaration + PyO3 function registration
- `C:/Users/OMack/nono-py/Cargo.toml` (MODIFIED) — `winreg = "0.56"` under `[target.'cfg(windows)'.dependencies]`
- `C:/Users/OMack/nono-py/src/windows_confined_run.rs` (MODIFIED) — pre-existing clippy fixes (collapsible_if, too_many_arguments) required to unblock `-D warnings` gate

## Commits (nono-py repo, branch `44-broker-ffi-lockstep`)

| Task | Type | Hash | Description |
|------|------|------|-------------|
| 1 RED | test | `f930123` | Add failing tests for LiveRevoked/LiveUnavailable (TDD RED gate) |
| 1 GREEN | feat | `39cac71` | Add LiveRevoked/LiveUnavailable error kinds (TDD GREEN gate) |
| 2 | feat | `82a5302` | Add fail-secure HKLM trust-root reader override_trust.rs (D-05/D-06) |
| 3 | feat | `8e19840` | Add VK_CACHE + policy-sourced pubkey lookup; close [BLOCKING-93] VFY-03a (D-06) |

## Decisions Made

1. **Non-Windows stub returns `Ok(OverrideTrust::empty())` not `Err`** — same fail-closed shape as absent HKLM sub-key; downstream ARN/pubkey checks deny on empty sets. Mirrored from `machine_policy.rs:533-536`.
2. **VK_CACHE stores DER bytes, not `VerificationKey`** — `VerificationKey` is not `Clone`; DER is cheap and trivially cached.
3. **`verify_override_impl` retains `pubkey_der` test-injection param** — Phase 91 CAF v0.1 byte-exact vectors and Phase 92 handshake tests continue to pass unchanged. `verify_override_production()` is the production path.
4. **`winreg 0.56` promoted from transitive to direct dep** — makes the dependency explicit; no new external package (same version already in the workspace tree via `crates/nono` path-dep).
5. **Forward-declaration `#[allow(dead_code)]` pattern** — `LiveRevoked`/`LiveUnavailable` carry `#[expect(dead_code, reason = "...Phase 93 Plan 02...")]`; `VK_CACHE`/`lookup_pubkey_der_from_policy`/`require_arn_allowlist_from_policy`/`verify_override_production` carry `#[allow(dead_code)]` with removal-note comments. Tests verify behavior now; production call sites arrive in Plans 02-04.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed pre-existing `collapsible_if` clippy error in `windows_confined_run.rs:244`**
- **Found during:** Task 2 (clippy gate)
- **Issue:** Nested `if let` blocks could be collapsed; blocked `-D warnings` gate
- **Fix:** Merged into single `if let ... && let ...` using let-chains
- **Files modified:** `C:/Users/OMack/nono-py/src/windows_confined_run.rs`
- **Commit:** `82a5302`

**2. [Rule 1 - Bug] Fixed pre-existing `too_many_arguments` clippy error on `confined_run` in `windows_confined_run.rs:367`**
- **Found during:** Task 2 (clippy gate)
- **Issue:** 8 parameters (py + 7 user-visible) exceeded clippy default; blocked `-D warnings` gate
- **Fix:** Added `#[allow(clippy::too_many_arguments)]` with comment: "PyO3 public API — signature is the Python caller's interface; restructuring would break all Python call sites"
- **Files modified:** `C:/Users/OMack/nono-py/src/windows_confined_run.rs`
- **Commit:** `82a5302`

**3. [Rule 1 - Bug] Fixed `#[cfg_attr(not(test), expect(dead_code))]` approach on VK_CACHE / lookup functions**
- **Found during:** Task 3 (clippy gate)
- **Issue:** `cfg_attr` + `expect(dead_code)` fired "unfulfilled lint expectation" because the functions call each other (none are dead relative to each other in non-test mode); required three iterations to converge
- **Fix:** Replaced with plain `#[allow(dead_code)]` + explicit comments identifying Phase 93 plan that adds each call site
- **Files modified:** `C:/Users/OMack/nono-py/src/override.rs`
- **Commit:** `8e19840`

**Total deviations:** 3 auto-fixed (2 pre-existing clippy bugs in unrelated file + 1 lint-attribute approach correction)
**Impact on plan:** None — all fixes unblocked the `-D warnings` gate; no scope or behavior change.

## Verification

- `cargo test -p nono-py --lib override::` — 66 tests pass (64 prior + 2 new Task 3 tests; TDD gate: RED `f930123` fails, GREEN `39cac71` passes)
- `cargo clippy -p nono-py -- -D warnings -D clippy::unwrap_used` — exits 0

## Cross-Target Note

Native clippy run on Windows host (primary target). `override_trust.rs` uses `#[cfg(target_os = "windows")]` / `#[cfg(not(target_os = "windows"))]` guards. The non-Windows stub is exercised by the native Windows build (the `not(windows)` branch is dead on this host). Cross-target verify (Linux/macOS) is PARTIAL — deferred to live CI per `cross-target-verify-checklist.md`. No unsafe code; no new cfg-gated Unix blocks introduced.

## Known Stubs

None — all items are forward-declarations with explicit test coverage. The `#[allow(dead_code)]` items have production call sites arriving in Plans 02-04 of this phase.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: trust-root-widening | `override_trust.rs` | Introduces the authoritative HKLM trust-root read path. D-05 correctly enforces that env vars cannot widen trust. Any future refactor that adds env-var fallback here would be a security regression. |

## Self-Check

- [x] `C:/Users/OMack/nono-py/src/override_trust.rs` exists (created in Task 2)
- [x] `C:/Users/OMack/nono-py/src/override.rs` contains `LiveUnavailable`, `VK_CACHE`, `lookup_pubkey_der_from_policy`, `verify_override_production`
- [x] Commits `f930123`, `39cac71`, `82a5302`, `8e19840` present in nono-py repo
- [x] 66 tests pass; clippy -D warnings exits 0

## Self-Check: PASSED
