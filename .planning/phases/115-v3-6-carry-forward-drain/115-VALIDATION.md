---
phase: 115
slug: v3-6-carry-forward-drain
status: ready
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-08
verified: 2026-08-08
---

# Phase 115 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `115-RESEARCH.md` § "Validation Architecture" (symbol-verified against tree `157b2c6d`).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`), workspace-native. `../nono-py` gets its **first** Rust-level `#[cfg(test)]` block this phase. |
| **Config file** | none — standard `cargo test` |
| **Quick run command** | `cargo test -p nono-sandbox-cli --lib profile::` (profile/merge/validation work) · `cargo test -p nono-sandbox-proxy --lib` (denial spine) · `cargo test -p nono-sandbox --lib` (core enum) · `cargo test -p nono-py --lib` (binding) |
| **Full suite command** | `cargo test --workspace` + `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used` + `maturin build` (run from `../nono-py`) |
| **Estimated runtime** | ~90–180s workspace tests; `maturin build` adds ~2–4 min cold |

**Cross-target clippy gates are NOT required for this phase.** No in-scope file contains
`#[cfg(target_os = "linux"/"macos")]`, lives under `exec_strategy/`, or lives under
`bindings/c/src/` (RESEARCH Q9, direct grep). Do not budget a cross-target-verify task.
Native Windows-host clippy + fmt is the correct gate set here — this is an evidence-based
exemption from CLAUDE.md's MUST, not a PARTIAL→CI fallback.

---

## Sampling Rate

- **After every task commit:** the crate-scoped quick-run command for the file(s) touched
- **After every plan wave:** `cargo build --workspace --all-targets` + the affected crates' `--lib` tests + `cargo fmt --all -- --check`; at the wave touching `../nono-py`, additionally `maturin build`
- **Before `/gsd:verify-work`:** full suite green, including `maturin build` exit 0
- **Max feedback latency:** ~180 seconds (workspace test run)

**Mandatory extra gate for this phase:** `/gsd:code-review` is required before phase close.
Milestone invariant — Phase 112's review gate caught 4 Critical fail-open defects that all 8
executor self-checks passed over, and Phase 114 repeated it twice. **Executor self-check is
not security evidence.**

---

## Per-Task Verification Map

Task IDs are assigned by the planner; this table binds each requirement to its proof
mechanism so plans can be checked against it (Dimension 8).

| Req | Decisions | Proof mechanism | Test Type | Automated Command | File Exists |
|-----|-----------|-----------------|-----------|-------------------|-------------|
| DRAIN-01 | D-01, D-02, D-03, D-05 | Override redefining only `upstream` inherits **every** other field incl. `inject_mode`/`inject_header`/`spiffe`; test is exhaustive over all merged fields and fails when a new field gains no merge arm. `upstream`'s child-wins asymmetry asserted with an "intended, not overlooked" comment (D-02). | unit | `cargo test -p nono-sandbox-cli --lib profile::` | ❌ W0 — extend the NEW-02 regression test (`profile/mod.rs`, locate by symbol) |
| DRAIN-01 | D-01 | Struct-shape change compiles across all 38 `CustomCredentialDef` literal sites (2 production, 36 `#[cfg(test)]`) + schema updated | compile | `cargo build --workspace --all-targets` | ✅ existing |
| DRAIN-01 | D-04 | `HookConfig` gaining an optional/defaulted field **fails to compile** | compile-time guard | `cargo build --workspace` + documented counterexample note (see "Compile-Guard Bite Proof") | ❌ W0 |
| DRAIN-02 | D-10, D-11, D-12 | Every category the encoder can emit decodes without raising, driven from the **enum's own iteration**, not a second hand-written list | unit (binding crate, Rust) | `cargo test -p nono-py --lib` **then** `cd ../nono-py && maturin build` | ❌ W0 — first Rust test in that crate |
| DRAIN-02 | D-10 | Serde `rename_all = "snake_case"` output is byte-identical to every current hand-written string (incl. `ConnectBypassesL7` → `connect_bypasses_l7`) | unit | `cargo test -p nono-sandbox --lib` | ⚠ RESEARCH already verified this empirically; pin it as a committed test |
| DRAIN-03 | D-06 | Omitting the denial category at a `log_denied` call site **fails to compile** | compile-time guard | `cargo build --workspace --all-targets` + documented counterexample note | ❌ W0 |
| DRAIN-03 | D-07, D-08 | `connect.rs` (HTTPS `deny_domain` point) and `external.rs` host-check emit `HostDenied`; `external.rs` proxy-reject site emits `ExternalProxyRejected` — a consumer filtering on `HostDenied` sees **all six** dispatch paths | unit | `cargo test -p nono-sandbox-proxy --lib` | ⚠ pattern exists in `server.rs` denial-category assertions; 3 new assertions needed |
| DRAIN-03 | D-09 | `InterceptHandshakeFailed` removed; no denial variant remains with zero production constructors | compile + audit | `cargo build --workspace --all-targets`; ledger-safety precondition **already discharged** by RESEARCH Q1 (zero hits repo-wide incl. fixtures/goldens) | ✅ precondition satisfied |
| DRAIN-04 | D-13 | A profile declaring `aws_auth` on **any** route is rejected at profile-validation time, not accepted-then-501'd | unit | `cargo test -p nono-sandbox-cli --lib profile::` | ❌ W0 — new rejection test; existing `aws_auth`-bearing validate-OK tests **will break, and that is the intended signal** |
| DRAIN-04 | D-14 | Plain OAuth2 `client_credentials` (no `client_assertion`) rejected at validation. **Evidence discharged:** RESEARCH Q2 = UNWIRED — such routes enter none of `CredentialStore`'s three maps and silently reach upstream with **zero token injection** (not a 501). Rejection applies. | unit | `cargo test -p nono-sandbox-cli --lib profile::` | ❌ W0 — new rejection test |
| DRAIN-05 | D-15 | Python `RouteConfig::new` accepts `spiffe`, `capture`, and `endpoint_policy` (confirmed live: compiled at route-load, enforced by exhaustive match in `reverse.rs`, 3 dedicated tests). `aws_auth`/`oauth2` stay `None` **as a recorded decision in code**, not an omission. | unit (binding crate) | `cargo test -p nono-py --lib` + `maturin build` | ❌ W0 — new PyO3 `#[pyclass]` wrappers required |
| DRAIN-05 | D-16 | No `RustRouteConfig` field is unconditionally `None`, enforced by a test with a **named allowlist** — a new silently-`None` field fails until someone adds it to the allowlist (a visible decision) | unit (binding crate) | `cargo test -p nono-py --lib` | ❌ W0 |
| DRAIN-06 | D-17 | A `deny_domain`-blocked SPIFFE route is denied **before** any JWT-SVID is minted — assert the **absence of the mint**, not merely the 403 | see "Open Design Decision" below | `cargo test -p nono-sandbox-proxy` | ⚠ `spiffe_integration.rs` exists with the Phase 113 D-03 pattern to mirror |

*Status legend: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Compile-Guard Bite Proof (mandatory for D-01/D-04/D-06/D-11)

Four of this phase's fixes are proven by **compile failure**, not by a runtime assertion.
A guard that exists but does not bite is worse than no guard — it manufactures false
confidence. Every compile-time guard in this phase MUST ship with a documented
"how do we know it bites?" artifact:

- A commented-out counterexample immediately adjacent to the guard, showing the exact
  edit that must fail to compile (e.g. `// Adding `pub timeout: Option<u64>` to HookConfig
  below must break this destructuring — verified 2026-08-08`), **or**
- A `compile_fail` doctest where the crate structure permits it.

An executor claiming "the guard is in place" without one of these has not validated
anything. This mirrors the phase's own governing rule: *make it unrepresentable, not
tested-for* — but the unrepresentability itself still needs evidence.

---

## Wave 0 Requirements

- [ ] Extended NEW-02 regression test in `crates/nono-cli/src/profile/mod.rs` — exhaustive over every merged `CustomCredentialDef` field including the two new `Option` fields and `spiffe` (closes ACC-04) — DRAIN-01
- [ ] Compile-guard + counterexample note for `HookConfig` (D-04) — DRAIN-01
- [ ] Compile-guard + counterexample note for `log_denied`'s required category (D-06) — DRAIN-03
- [ ] 3 new `denial_category` assertions for the previously-default call sites — DRAIN-03
- [ ] New `validate_custom_credential` rejection tests: `aws_auth` (D-13) and plain `client_credentials` (D-14) — DRAIN-04
- [ ] First `#[cfg(test)] mod tests` block in `../nono-py` — self-enumerating round-trip test driven from the encoder's own output (D-10/D-11/D-12) — DRAIN-02
- [ ] Allowlist test asserting no `RustRouteConfig` field is unconditionally `None` (D-16) — DRAIN-05
- [ ] DRAIN-06 regression test — **mechanism pending the planner's decision below**

---

## Open Design Decision — DRAIN-06 test mechanism

RESEARCH flagged this as the one unresolved design call. "Assert no SPIRE mint occurred"
has three candidate mechanisms:

| Option | Mechanism | Cost | Host-gated? |
|--------|-----------|------|-------------|
| (a) | Test double / spy on `SpiffeJwtSource::connect` | high — new seam in production code | no |
| (b) | Integration test pointing `workload_api_socket` at a non-existent path, asserting absence of a connection-attempt | medium | partially |
| **(c)** | **Structural**: the deny returns before `managed_auth.acquire()` is reachable at all — provable by control-flow, pinned by a self-enforcing source assertion | **low** | **no** |

**Recommendation: (c).** It matches D-17's own mechanism (hoist the check), needs no SPIRE
agent, and is consistent with this phase's "unrepresentable over tested-for" through-line.
**The planner must state the choice explicitly in the plan** — leaving it to the executor
is how a mechanism decision becomes an undocumented deviation.

If any test does end up host-gated, reuse Phase 113's loud-skip convention
(`eprintln!("SKIP[{}]: reason", module_path!())` + `grep -c '^SKIP\['`). Do not invent a
second convention.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `maturin build` wheel produces an importable module with the new `RouteConfig` params | DRAIN-02, DRAIN-05 | Only a real wheel build catches binding struct drift — this is the **5th consecutive** occurrence of that lesson (Phase 114 D-14) | From `../nono-py`: `maturin build` → exit 0. `cargo test -p nono-py` alone is NOT sufficient evidence. |
| Divergence-ledger entry for D-13 | DRAIN-04 | Documentation act, not code | Record in `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`: a future UPST absorb of real SigV4 must **un-reject** `aws_auth`. |

---

## Validation Sign-Off

- [x] All tasks have an automated verify command or a Wave 0 dependency
- [x] Sampling continuity: no 3 consecutive tasks without an automated verify
- [x] Wave 0 covers every ❌ reference above
- [x] Every compile-time guard (D-01/D-04/D-06/D-11) ships its bite-proof artifact
      *(D-01: no `#[derive(Default)]` on `CustomCredentialDef` + live add-field/E0063/revert cycle;
      D-04, D-06, D-11: live-verified counterexample steps in 115-01 T3 / 115-02 T1 / 115-02 T3)*
- [x] DRAIN-06 test mechanism chosen explicitly in the plan, not deferred to the executor
      *(115-04 locks option (c) — structural)*
- [x] `maturin build` runs from `../nono-py` at the binding wave (`use_worktrees: false` confirmed already set)
      *(retiered out of the per-task verify for latency; remains a hard acceptance criterion and
      wave-merge gate on both 115-05 and 115-06)*
- [x] No watch-mode flags
- [ ] `/gsd:code-review` scheduled before phase close — **open, run at phase close**
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-08 — plan-checker `VERIFICATION PASSED` (iteration 2, 0 blockers).
Remaining open item is `/gsd:code-review`, which by definition runs after execution.
