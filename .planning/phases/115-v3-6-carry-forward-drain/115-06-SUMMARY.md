---
phase: 115-v3-6-carry-forward-drain
plan: 06
subsystem: api
tags: [nono-py, pyo3, spiffe, oauth-capture, endpoint-policy, cross-repo, maturin]

# Dependency graph
requires:
  - phase: 115-v3-6-carry-forward-drain (plan 05)
    provides: "../nono-py compiling again with a serde-driven NetworkAuditDenialCategory codec (same-file precondition: both plans edit proxy.rs)"
provides:
  - "8 new PyO3 wrapper types in ../nono-py/src/proxy.rs mirroring nono-proxy's config shapes: SpiffeAuthConfig, CaptureResponseFieldKind, CaptureResponseField, CaptureConfig, EndpointPolicyDecision, EndpointPolicyRule, EndpointPolicyDefault, EndpointPolicy"
  - "RouteConfig::new exposes spiffe/capture/endpoint_policy — a Python embedder can configure all three confirmed-live-enforced mechanisms, closing DRAIN-05/NEW-07"
  - "oauth2/aws_auth stay hardcoded None with an in-code comment citing D-13/D-14, replacing silence with a recorded decision"
  - "D-16 allowlist test (proxy::tests::route_config_leaves_no_field_silently_none_outside_d15_allowlist), verified load-bearing by an inline break/revert transcript"
  - "All 8 new types registered on the Python module surface (lib.rs m.add_class, __init__.py imports/__all__, _nono_py.pyi stubs) — Rule 2 deviation beyond the plan's stated files_modified scope, required for the plan's own truths to hold"
affects: [118 (receipts work), any future nono-py profile-schema surface for endpoint_policy — currently the ONLY configuration surface for that mechanism, Rust or Python, per 115-RESEARCH.md Q4]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "inner: RustX wrapper idiom extended to 6 new struct types (SpiffeAuthConfig, CaptureResponseField, CaptureConfig, EndpointPolicyRule, EndpointPolicyDefault, EndpointPolicy) and 2 new C-like enum wrappers (CaptureResponseFieldKind, EndpointPolicyDecision) following this crate's existing CapabilitySource/InjectMode precedents exactly — no new idiom invented"
    - "#[pyclass(frozen, from_py_object)] (not skip_from_py_object) required for any wrapper struct that is itself used as a constructor parameter elsewhere (e.g. CaptureResponseField inside CaptureConfig::new, SpiffeAuthConfig/CaptureConfig/EndpointPolicy inside RouteConfig::new) — skip_from_py_object blocks PyO3's Vec<T>/Option<T> FromPyObject derive needed for that usage"
    - "Enumerated-assertion allowlist test (D-16): true generic field reflection isn't available in stable Rust, so the allowlist mechanism is an explicit per-field assert! list with a named const allowlist for the D-15 exclusions, not a loop over struct fields"

key-files:
  created: []
  modified:
    - ../nono-py/src/proxy.rs
    - ../nono-py/src/lib.rs
    - ../nono-py/python/nono_py/__init__.py
    - ../nono-py/python/nono_py/_nono_py.pyi
    - .planning/phases/115-v3-6-carry-forward-drain/deferred-items.md

key-decisions:
  - "D-15 confirmed at source before implementing: 115-RESEARCH.md Q4's endpoint_policy liveness finding (compiled at route-load in route.rs, enforced via an exhaustive match in reverse.rs, 3 dedicated tests in nono-proxy/src/config.rs) re-verified by symbol against crates/nono-proxy/src/config.rs and 115-RESEARCH.md before exposing it — not taken on faith"
  - "Registered all 8 new types on the Python module surface (lib.rs/__init__.py/_nono_py.pyi) beyond the plan's stated files_modified: [../nono-py/src/proxy.rs] scope. Rule 2 (auto-add missing critical functionality): without m.add_class() registration, PyO3 types compile but are not importable from Python, which would make the plan's own truth ('A Python embedder can set capture and spiffe on a RouteConfig') false despite a green cargo build -p nono-py."
  - "oauth2/aws_auth exclusion recorded as an in-code comment directly above the two hardcoded-None struct-literal fields in RouteConfig::new, citing D-13/D-14 (Plan 115-03), per the plan's exact required wording"
  - "D-16's allowlist mechanism is an explicit enumerated assert! list (not a generic field-reflection loop, which stable Rust cannot express over a heterogeneous struct) — matches the plan's own text ('carry an explicit allowlist for the D-15 exclusions')"

patterns-established:
  - "When a downstream binding crate's #[pyclass] wrapper struct will itself be passed as a constructor parameter to another wrapper (composition), it needs #[pyclass(frozen, from_py_object)] explicitly, not skip_from_py_object — this crate's newest PyO3 (0.28.3) deprecated the implicit derive that older wrapper structs (SupportInfo) rely on, so new composable wrappers should always be explicit"

requirements-completed: [DRAIN-05]

# Metrics
duration: ~50min
completed: 2026-08-08
---

# Phase 115 Plan 06: nono-py RouteConfig spiffe/capture/endpoint_policy Exposure Summary

**Added 8 new PyO3 wrapper types mirroring nono-proxy's config shapes and wired `spiffe`/`capture`/`endpoint_policy` into `RouteConfig::new` (all three confirmed live-enforced, endpoint_policy's liveness re-verified by symbol at source), replacing silent hardcoded-`None` with either real wiring or a recorded D-13/D-14 exclusion comment for `oauth2`/`aws_auth`, pinned by a load-bearing D-16 allowlist test and a real `maturin build`.**

## Performance

- **Duration:** ~50 min
- **Completed:** 2026-08-08T22:40:00Z
- **Tasks:** 2 (plus 1 Rule 2 deviation task: Python module-surface registration)
- **Files modified:** 5 (4 in `../nono-py`, 1 in `nono` — `deferred-items.md`)

## Accomplishments

- Closed DRAIN-05/NEW-07: `RouteConfig::new` in `../nono-py` no longer hardcodes `spiffe`/`capture`/`endpoint_policy` to `None`. A Python embedder can now configure all three — SPIFFE workload-identity auth (Phase 113), declarative OAuth capture (Phase 114), and explicit L7 endpoint policy — matching what a Rust caller has always been able to do.
- Verified `endpoint_policy` liveness at source before exposing it, per the plan's explicit precondition: re-grepped `crates/nono-proxy/src/route.rs` (`CompiledEndpointPolicy::compile` at route-load), `crates/nono-proxy/src/reverse.rs` (the exhaustive `Allow`/`Deny`/`Approve` match, `Approve` fails closed with no backend wired), and `crates/nono-proxy/src/config.rs`'s 3 dedicated tests (`endpoint_policy_deny_rule_is_enforced`, `endpoint_policy_legacy_route_preserves_deny_semantics`, `endpoint_policy_approve_without_backend_is_recognized`). Confirms 115-RESEARCH.md Q4's finding still holds. Also confirms the notable related finding: `endpoint_policy` has zero exposure at the `nono-cli` profile-schema level (`CustomCredentialDef` has no such field, `nono-profile.schema.json` has zero hits) — this plan's Python exposure is the *first* configuration surface for the mechanism outside test code, Rust or Python.
- `aws_auth`/`oauth2` stay hardcoded `None`, now with an in-code comment directly above the struct-literal fields citing D-13/D-14 (Plan 115-03's validation rejections) — a recorded decision, not silence, per D-15's explicit requirement.
- Added the D-16 allowlist test (`proxy::tests::route_config_leaves_no_field_silently_none_outside_d15_allowlist`): constructs a `RouteConfig` through the Python-facing `#[new]` with every parameter supplied and asserts every `Option<T>` field on the resulting `RustRouteConfig` is `Some(...)` except the named `["oauth2", "aws_auth"]` allowlist.
- **Verified the test is load-bearing, not decorative**, per the plan's explicit instruction: temporarily changed `tls_ca,` to `tls_ca: None, // SCRATCH` in `RouteConfig::new`'s struct literal (a field outside the allowlist), re-ran `cargo test -p nono-py --lib route_config_leaves_no_field`, confirmed it FAILED with `thread '...' panicked at src\proxy.rs:1246:9: tls_ca`, then reverted. Full transcript below.
- `maturin build` produced a real wheel (`nono_sandbox-0.70.0-cp312-cp312-win_amd64.whl`) against the new 8-type/3-new-parameter constructor shape — the sixth consecutive occurrence of this project's "only a real wheel build catches binding struct drift" lesson, this time confirming the new shape compiles end-to-end through maturin, not just `cargo build`.
- `cargo test -p nono-py --lib` (71 passed, up from 115-05's 70), `cargo build --workspace --all-targets` (nono repo, 0 errors), and `cargo fmt --all -- --check` (nono repo, clean) all pass.

## Task Commits

**../nono-py repo** (`44-broker-ffi-lockstep` branch):
1. **Task 1: New PyO3 wrapper types (contracts)** - `69f7148` (feat)
2. **Task 2: Wire RouteConfig::new + D-16 allowlist test** - `5d348c3` (feat)
3. **Rule 2 deviation: register new types on the Python module surface** - `8a45a1b` (fix)

_All three commits carry the DCO sign-off line._

## Files Created/Modified

- `../nono-py/src/proxy.rs` (nono-py repo) - 8 new `#[pyclass]` wrapper types (`SpiffeAuthConfig`, `CaptureResponseFieldKind`, `CaptureResponseField`, `CaptureConfig`, `EndpointPolicyDecision`, `EndpointPolicyRule`, `EndpointPolicyDefault`, `EndpointPolicy`); `RouteConfig::new` wired to accept `spiffe`/`capture`/`endpoint_policy` parameters and map them into `RustRouteConfig`; 3 new getters on `RouteConfig`; the D-13/D-14 comment above `oauth2: None, aws_auth: None,`; the D-16 allowlist test in a new `#[cfg(test)] mod tests` block
- `../nono-py/src/lib.rs` (nono-py repo) - `m.add_class::<proxy::*>()` registrations for all 8 new types (Rule 2 deviation)
- `../nono-py/python/nono_py/__init__.py` (nono-py repo) - imports and `__all__` entries for all 8 new types (Rule 2 deviation)
- `../nono-py/python/nono_py/_nono_py.pyi` (nono-py repo) - type stubs for all 8 new classes plus `RouteConfig`'s 3 new constructor params/getters (Rule 2 deviation, per this crate's own CLAUDE.md: "must stay in sync")
- `.planning/phases/115-v3-6-carry-forward-drain/deferred-items.md` (nono repo) - logged 2 pre-existing, out-of-scope findings surfaced while working in `../nono-py` (see Issues Encountered)

## Decisions Made

- **`#[pyclass(frozen, from_py_object)]` on all 6 new struct wrappers, not `skip_from_py_object`**: the plan's interface note cited `CapabilitySource`'s `#[pyclass(frozen, skip_from_py_object)]` idiom as precedent, but `CapabilitySource` is never itself used as a constructor argument elsewhere. Every new type here IS composed into another constructor (e.g. `Vec<CaptureResponseField>` into `CaptureConfig::new`, `SpiffeAuthConfig`/`CaptureConfig`/`EndpointPolicy` into `RouteConfig::new`), which requires PyO3's `FromPyObject` derive — `skip_from_py_object` blocks that and fails to compile (`E0277: the trait bound Vec<proxy::EndpointPolicyRule>: pyo3::FromPyObject is not satisfied`). Corrected to the explicit `from_py_object` form (matching `RouteConfig`/`ExternalProxyConfig`'s own existing idiom) rather than the deprecated bare `#[pyclass(frozen)]` implicit-derive form, which this crate's pinned PyO3 0.28.3 flags with a deprecation warning.
- **Registered new types on the Python module surface beyond the plan's stated `files_modified` scope** (Rule 2 — see Deviations below).
- **D-16 test placed in a new `#[cfg(test)] mod tests` block in `proxy.rs`** (not extending 115-05's `undo.rs` block) — the prior-wave context flagged this choice explicitly ("if your D-16 allowlist test belongs in proxy.rs, add a new mod tests there"). `proxy.rs` is where `RouteConfig::new` lives and where the test needs private-field access (`.inner`) to `RouteConfig`, so colocation is natural; `undo.rs`'s existing test module exercises an unrelated concern (the denial-category codec).
- **Getters added for `spiffe`/`capture`/`endpoint_policy` on `RouteConfig`** even though the plan's acceptance criteria didn't explicitly require them — every other field on `RouteConfig` has a getter (established idiom), and a Python embedder configuring these fields will reasonably expect to read them back. Not treated as a deviation (follows existing idiom exactly, no new API surface pattern).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Registered the 8 new PyO3 types on the Python module surface**
- **Found during:** Task 2, before writing the SUMMARY — re-read the plan's own truths ("A Python embedder can set capture and spiffe on a RouteConfig") against what `cargo build -p nono-py` alone actually proves.
- **Issue:** The plan's `files_modified` frontmatter scopes only `../nono-py/src/proxy.rs`. A `#[pyclass]` type that compiles in Rust is NOT automatically importable from Python — PyO3 requires an explicit `m.add_class::<T>()` call inside the `#[pymodule]` function (`lib.rs`). Without this, all 8 new types (and therefore `RouteConfig.spiffe`/`.capture`/`.endpoint_policy`) would be entirely unreachable from Python despite a green Rust build — the exact "compiles but doesn't work for the actual caller" gap this plan exists to close.
- **Fix:** Added 8 `m.add_class::<proxy::*>()` calls to `lib.rs`'s `_nono_py` module function; added imports + `__all__` entries to `python/nono_py/__init__.py`; added type stubs for all 8 classes plus `RouteConfig`'s 3 new constructor params/getters to `python/nono_py/_nono_py.pyi`, per this crate's own `CLAUDE.md` ("`_nono_py.pyi` ... is the source of truth for IDE autocompletion and mypy ... must stay in sync").
- **Files modified:** `../nono-py/src/lib.rs`, `../nono-py/python/nono_py/__init__.py`, `../nono-py/python/nono_py/_nono_py.pyi`
- **Verification:** `cargo build -p nono-py` (clean), `cargo test -p nono-py --lib` (71 passed), `maturin build` (wheel built against the fully-registered shape).
- **Committed in:** `8a45a1b` (`../nono-py` repo, separate commit after the two plan-scoped task commits)

**2. [Rule 1 - Bug, self-caught] Fixed a compile error from `skip_from_py_object` on composable wrapper types**
- **Found during:** Task 1, first `cargo build -p nono-py` after adding the 8 new types.
- **Issue:** Following the plan's `CapabilitySource` precedent literally (`#[pyclass(frozen, skip_from_py_object)]`) on all 6 new struct wrappers produced 10 compile errors — `skip_from_py_object` blocks PyO3's `FromPyObject` derive, but 4 of the 6 new types (`CaptureResponseField`, `EndpointPolicyRule`, `EndpointPolicyDefault`, and later `SpiffeAuthConfig`/`CaptureConfig`/`EndpointPolicy` via `RouteConfig::new`) are themselves used as constructor parameters elsewhere, which requires that derive.
- **Fix:** Changed all 6 to `#[pyclass(frozen, from_py_object)]` (explicit form, matching `RouteConfig`/`ExternalProxyConfig`'s existing precedent in the same file, avoiding a deprecation warning from the implicit bare-`frozen` form).
- **Files modified:** `../nono-py/src/proxy.rs`
- **Verification:** `cargo build -p nono-py` clean, 0 errors, 0 warnings.
- **Committed in:** `69f7148` (folded into Task 1's commit — caught and fixed before the first commit was made, not a separate follow-up commit)

**3. [Rule 1 - Bug, self-caught] Fixed a self-introduced rustfmt violation in the D-16 test**
- **Found during:** Task 2, running `cargo fmt --all -- --check` per the plan-level verification gate.
- **Issue:** Two `assert!` calls in the new D-16 test exceeded the line-length limit on a single line.
- **Fix:** Reformatted the two `assert!` calls to rustfmt's multi-line form.
- **Files modified:** `../nono-py/src/proxy.rs`
- **Verification:** `cargo fmt --all -- --check` shows zero diff in `proxy.rs` (confirmed no other file this plan touched has a self-introduced diff either — see Issues Encountered for the ~85 pre-existing, unrelated diffs elsewhere in the crate).
- **Committed in:** `5d348c3` (folded into Task 2's commit)

---

**Total deviations:** 3 (1 Rule 2 missing-critical, 2 Rule 1 self-caught bugs, all fixed before their respective task commits)
**Impact on plan:** The Rule 2 deviation is necessary for the plan's own success criteria to be true in practice, not just in `cargo build` terms — no scope creep beyond what "a Python embedder can set capture and spiffe on a RouteConfig" requires. Both Rule 1 fixes were caught and corrected within the same task, before any commit landed with the defect.

## Issues Encountered

- **D-16 load-bearing verification transcript** (per the plan's explicit instruction to record this, not just assert it): with `tls_ca,` temporarily changed to `tls_ca: None, // SCRATCH: D-16 load-bearing check, reverted after test run` in `RouteConfig::new`'s struct literal —
  ```
  running 1 test
  test proxy::tests::route_config_leaves_no_field_silently_none_outside_d15_allowlist ... FAILED

  ---- proxy::tests::route_config_leaves_no_field_silently_none_outside_d15_allowlist stdout ----
  thread 'proxy::tests::route_config_leaves_no_field_silently_none_outside_d15_allowlist' (99848) panicked at src\proxy.rs:1246:9:
  tls_ca

  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 70 filtered out; finished in 0.04s
  ```
  Reverted the scratch change; `cargo test -p nono-py --lib` returned to 71/71 passing, confirming the test is genuinely load-bearing and not a vacuous pass.
- **`../nono-py/python/nono_py/_nono_py.pyi` had pre-existing drift from the Rust API before this plan touched it**: it documented `tls_client_cert`/`tls_client_key` `RouteConfig` parameters that do not exist on `RouteConfig::new` in `src/proxy.rs` (nor on `nono_proxy::config::RouteConfig`). Not introduced by this plan — left untouched per the scope-boundary rule, this plan's new params were inserted correctly relative to the real signature (after `tls_ca`) but sit adjacent to the two stale entries in the stub. Logged to `deferred-items.md`.
- **`cargo fmt --all -- --check` in `../nono-py` shows ~85 pre-existing diffs** across `src/override.rs`, `src/override_trust.rs`, `src/windows_confined_run.rs`, `src/lib.rs` (mod-declaration ordering + an unrelated line-wrap), and `src/undo.rs` (2 spots) — verified none touch a line this plan authored (confirmed via `git diff --stat`: those four files show 0 insertions/deletions from this plan; `src/lib.rs`'s diffs land at lines 14/22/781, nowhere near this plan's `m.add_class` insertion around line 739-750). Likely a `rustfmt` toolchain/edition mismatch on this dev host versus the crate's pinned `rust-version = 1.95` / `edition = "2024"`. Not fixed here (out of scope, large surface). Logged to `deferred-items.md`.
- `cargo clippy -p nono-py --all-targets -- -D warnings` was not re-run this plan (per 115-VALIDATION.md, cross-target clippy gates are not required for this phase, and the pre-existing `override.rs` clippy debt from 115-05 remains logged in `deferred-items.md`, unrelated to this plan's files).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `../nono-py`'s `RouteConfig` now exposes every confirmed-live-enforced `nono-proxy` route mechanism to Python embedders (`spiffe`, `capture`, `endpoint_policy`, alongside the pre-existing `credential_key`/`inject_mode`/`endpoint_rules`/`tls_ca`), closing the DRAIN-05/NEW-07 gap at the class level: the D-16 test makes any future silent-`None` field addition a loud, must-acknowledge test failure rather than a repeat of this defect.
- `endpoint_policy` remains unexposed at the `nono-cli` profile-schema level (Rust or Python profile JSON) — this plan's Python binding exposure is the mechanism's first real-world configuration surface, but a Rust CLI user still cannot set it via a profile. Noted as out-of-scope for this phase (115-RESEARCH.md Q4), not a regression.
- Deferred, not blocking: `_nono_py.pyi`'s pre-existing `tls_client_cert`/`tls_client_key` stale stub entries, and the crate-wide `rustfmt` toolchain mismatch — both logged in `deferred-items.md`.
- Phase 115's DRAIN-01 through DRAIN-06 findings are now all closed by this plan and its five predecessors (per the phase's requirement list); this was the final plan in wave 3.

---
*Phase: 115-v3-6-carry-forward-drain*
*Completed: 2026-08-08*
