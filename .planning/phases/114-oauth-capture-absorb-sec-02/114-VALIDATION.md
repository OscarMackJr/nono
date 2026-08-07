---
phase: 114
slug: oauth-capture-absorb-sec-02
status: revised
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-06
---

# Phase 114 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `114-RESEARCH.md` § "Validation Architecture" (line 677).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`); `cross test` for Linux-gated paths |
| **Config file** | none — standard `#[cfg(test)] mod tests` per crate (matches every other `nono-proxy` / `nono-cli` module) |
| **Quick run command** | `cargo test -p nono-proxy` |
| **Full suite command** | `make ci` (clippy + fmt + tests, per CLAUDE.md) |
| **Estimated runtime** | ~60–120 seconds for `-p nono-proxy`; `make ci` several minutes |

**SC4 gates (not `cargo test` — separate mandatory gates):**
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used`
- `cargo fmt --all --check`

See `.planning/templates/cross-target-verify-checklist.md` — single source of truth. PARTIAL→CI is
**not** an acceptable outcome for this phase (CONTEXT.md D-09).

> ⚠ **Do not run a `-p nono-sandbox-cli --tests` full 36-binary sweep on this host** — it stalls
> (~25 min, recorded in prior-milestone notes). Scope test runs to `-p nono-proxy` / `-p nono-cli`
> plus targeted filters.

---

## Sampling Rate

- **After every task commit:** `cargo test -p nono-proxy` (or `-p nono-cli` when the task touches
  the profile/schema surface)
- **After every plan wave:** `make ci`
- **Before `/gsd:verify-work`:** full suite green **+** both cross-target clippy gates green **+**
  both binding rebuilds (`../nono-py`, `../nono-ts`) green
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

> Task IDs are assigned by `gsd-planner`. The rows below are the **required behaviors** — the
> planner MUST map each to at least one concrete task, and the plan-checker verifies coverage.
> All test files are ❌ (Wave 0) — none of these tests exist yet.

| Behavior | Decision Ref | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|----------|--------------|------------|-----------------|-----------|-------------------|-------------|--------|
| Buffer cap exceeded → response DENIED, never released unrewritten | SC2 / D-05 | Info Disclosure + DoS | Fail closed; count **actual bytes read**, not just `Content-Length` | unit | `cargo test -p nono-proxy capture_buffer_cap_exceeded_denies_response` | ❌ W0 | ⬜ pending |
| Configured `response_fields` path rewritten to phantom | SC2 / D-07 | Info Disclosure | Real token never reaches client | unit | `cargo test -p nono-proxy capture_rewrites_configured_fields` | ❌ W0 | ⬜ pending |
| Unconfigured but token-shaped field left unrewritten → fail closed | SC2 / D-07 | Info Disclosure | Reject rather than forward | unit | `cargo test -p nono-proxy capture_fails_closed_on_unrewritten_token_field` | ❌ W0 | ⬜ pending |
| `Content-Encoding` present → denied **for any status code**, not 2xx-only | Pitfall 2 (`3c59c62e` lesson) | Info Disclosure | Unconditional rejection | unit | `cargo test -p nono-proxy capture_denies_content_encoded_response` | ❌ W0 | ⬜ pending |
| **All three** `reverse.rs` relay sites apply the rewrite | D-06 / WR-13 | Info Disclosure | Prove each site **independently** — not one shared assertion | integration | 3 discrete tests, one per site (static_cred, `handle_spiffe_route`, `handle_spiffe_assertion_credential`) | ❌ W0 | ⬜ pending |
| CONNECT to a capture-declared route upstream is blocked | D-06 | Elevation of Privilege | Regression test — `is_route_upstream` already blocks this; **no new guard needed** | unit | `cargo test -p nono-proxy connect_denies_capture_declared_route_upstream` | ❌ W0 | ⬜ pending |
| `handle_forward_http` denies capture-declared route upstream at request time | D-06 (**NEW guard**) | Elevation of Privilege | The one genuinely new guard; mirrors `d03_forward_http_denies_spiffe_declared_route_upstream` | unit | `cargo test -p nono-proxy forward_http_denies_capture_declared_route_upstream` | ❌ W0 | ⬜ pending |
| Real tokens in-memory only, `Zeroizing`-wrapped, no disk persistence | D-08 | Info Disclosure | Assert no persist path / no file I/O in store | unit | `cargo test -p nono-proxy capture_store_holds_only_in_memory` | ❌ W0 | ⬜ pending |
| Phantom resolves only for admitted consumers | Threat table | Elevation of Privilege | Per-phantom consumer scoping | unit | `cargo test -p nono-proxy capture_phantom_rejects_unadmitted_consumer` | ❌ W0 | ⬜ pending |
| Minted phantom resolves to the real token for an admitted consumer, on a real production request-dispatch path (not just capture.rs's own unit tests) | D-02r / D-06 (revision fix) | Info Disclosure | Mint-to-resolve loop closed end-to-end — resolve_capture_request_body() wired into handle_reverse_proxy | integration | `cargo test -p nono-proxy capture_egress_resolves_admitted_phantom_in_request_body` | ❌ W0 | ⬜ pending |
| Non-admitted or unknown phantom in a request_nonce_fields path is left unchanged (never resolved to a real token) | D-02r / D-06 (revision fix) | Elevation of Privilege | Fail closed — no fallthrough to a real token for a consumer the phantom was not admitted to | integration | `cargo test -p nono-proxy capture_egress_fails_closed_for_unadmitted_consumer_in_request_body` | ❌ W0 | ⬜ pending |
| Capture-provider profile deserializes **AND** passes `validate_against_schema()` | D-13 | Input Validation | Round-trip; mirrors `test_schema_validates_spiffe_custom_credential` (`profile/mod.rs:7396`) | unit | `cargo test -p nono-cli test_schema_validates_capture_custom_credential` | ❌ W0 | ⬜ pending |
| No policy/enforcement logic lands in `crates/nono/src/` | SC3 / ADR-86 | — | Any new audit context type is **pure data**, no branching | review + grep | grep-verified acceptance criterion in the plan | N/A | ⬜ pending |
| `../nono-py` + `../nono-ts` rebuild green after `RouteConfig` gains a field | D-14 | — | **Only building catches struct drift** — report real results | build gate | `maturin build` in `../nono-py`; `napi build --platform --release` in `../nono-ts` | N/A | ⬜ pending |
| Both cross-target clippy gates GREEN locally | SC4 / D-09 | — | No PARTIAL→CI fallback | manual gate | see Test Infrastructure above | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono-proxy/src/capture.rs` (or equivalent new module) — `#[cfg(test)] mod tests` for
      phantom mint / resolve / admit, mirroring `oauth_capture/mod.rs`'s test shape (CITED from
      upstream) adapted to the fork's config types
- [ ] Buffer-and-rewrite helper tests in `reverse.rs` — minimum 4 cases: cap-exceeded,
      content-encoded-rejected, configured-field-rewritten, unconfigured-token-field-fails-closed
- [ ] `server.rs` D-06 guard tests — mirror `d03_forward_http_denies_spiffe_declared_route_upstream`
      and `d03_connect_denies_spiffe_declared_route_upstream` (Plan 113-05) exactly, substituted
      for capture
- [ ] `crates/nono-cli/src/profile/mod.rs` schema round-trip test — mirror
      `test_schema_validates_spiffe_custom_credential` (`mod.rs:7396-7415`)
- [ ] `resolve_capture_request_body` tests in `reverse.rs` (Plan 114-06 Task 3, revision fix) —
      2 cases minimum: admitted-phantom-resolves-in-request-body,
      unadmitted-or-unknown-phantom-left-unchanged. Proves `CapturePhantomStore::resolve()` has a
      real production call site, not only `capture.rs`'s own unit tests (the plan-checker's SC2
      half-feature blocker).

**Framework install:** none. `cargo test` is already fully configured; D-07's portable logic uses
only crates `nono-proxy` already depends on (`base64`, `getrandom`, `url`, `serde_json`) — **no new
dependency is anticipated**. A new dependency would itself be a planning deviation worth flagging.

---

## Manual-Only Verifications

| Behavior | Ref | Why Manual | Test Instructions |
|----------|-----|------------|-------------------|
| Cross-target clippy (linux-gnu + apple-darwin) | SC4 | Requires Docker `cross` + `cargo-zigbuild` toolchains; not expressible as a `cargo test` | Run both commands from Test Infrastructure. Follow `.planning/templates/cross-target-verify-checklist.md`. A stopped daemon or absent tool does **NOT** qualify for the PARTIAL→CI fallback. |
| Sibling-repo binding rebuilds | D-14 | Cross-repo build in `../nono-py` / `../nono-ts`; requires `workflow.use_worktrees=false` (already set) | `maturin build` in `../nono-py`, `napi build --platform --release` in `../nono-ts`. Report **real** results — static inspection has under-predicted the break set 5 phases running (Phase 113 predicted 1 file, got 3). |
| ADR-86 / ADR-111 boundary non-regression | SC3 / D-09 | Architectural judgment, not a runtime assertion | Confirm no policy or enforcement logic landed in `crates/nono/src/`. Back it with a literal grep-verified acceptance criterion, not prose (CONTEXT.md `<specifics>`). |
| Baseline test-failure diff | SC4 | Host has documented pre-existing failures | Diff the workspace suite against the documented inherited failing baseline (4 `nono-cli` + 1 `nono` lib) — do **not** chase these as regressions. |

**Host-gated tests:** if any capture test ends up host-gated, reuse Phase 113's D-07 loud-skip
convention — `eprintln!("SKIP[{}]: reason", module_path!())` plus a `grep -c '^SKIP\['` assertion.
It already exists in this repo; do not invent a second convention.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [x] Each of the 3 `reverse.rs` relay sites proven **independently** (WR-13 lesson)
- [x] The mint-to-resolve loop is closed — `CapturePhantomStore::resolve()` has a grep-provable
      production call site (Plan 114-06 Task 3), proven by 2 independent tests (admitted resolves,
      non-admitted fails closed) — revision fix for the plan-checker's SC2 half-feature blocker
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** revised and re-approved 2026-08-07 (plan-checker iteration 1 blocker addressed —
see `114-06-PLAN.md` Task 3, `114-11-PLAN.md` Task 1's corrected ADR-114 spec)
