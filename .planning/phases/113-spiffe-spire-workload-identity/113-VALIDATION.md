---
phase: 113
slug: spiffe-spire-workload-identity
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-06
---

# Phase 113 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `113-RESEARCH.md` § "Validation Architecture". The planner fills the
> Per-Task Verification Map once plans exist; the executor flips Status columns.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`); `#[tokio::test]` for async |
| **Config file** | none — standard `Cargo.toml` test targets. New CI lane: `.github/workflows/spire.yml` (Wave 0) |
| **Quick run command** | `cargo test -p nono-sandbox-proxy --lib --test spiffe_integration -- --nocapture` |
| **Full suite command** | `cargo test --workspace --no-fail-fast` (diff against documented pre-existing failing baseline) |
| **Estimated runtime** | ~10 s quick (fail-closed subset needs no SPIRE); full workspace suite is minutes |

> **Host translation (`make` is NOT installed on this dev host):** every `make ci` /
> `make test-spiffe` reference in CLAUDE.md and `113-CONTEXT.md` must be run as its
> constituent commands — `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`,
> `cargo fmt --all --check`, `cargo test --workspace`, and `bash scripts/spire-test.sh`.
>
> **Scope the test commands narrowly.** A `-p nono-sandbox-cli --tests` sweep across all 36
> binaries STALLS (~25 min) on this host.

---

## Sampling Rate

- **After every task commit:** `cargo test -p nono-sandbox-proxy --lib --test spiffe_integration -- --nocapture`
- **After every plan wave:** both cross-target clippy gates + `cargo fmt --all --check` + `cargo test --workspace --no-fail-fast`
  - `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used`
  - `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset)
- **Before `/gsd:verify-work`:** full suite green + both cross-target gates GREEN locally (**no PARTIAL→CI** — D-12)
- **Max feedback latency:** ~10 s for the per-task quick command

**Phase gate caveat:** `spire.yml`'s new Linux CI lane is the *only* place the live-agent
tests genuinely execute end-to-end. No SPIRE server/agent exists on this Windows dev host, so
local verification cannot close D-06's loop by itself — the live half is closed by CI or not
at all, and that fact must be stated in the verification artifact (see D-07 below).

---

## Per-Task Verification Map

*To be filled by `gsd-planner` from the generated PLAN.md task IDs. Seed rows below come from
`113-RESEARCH.md` § "Phase Requirements → Test Map" and are the minimum coverage the map must
preserve.*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | — | NET-02 (SC1) | — | `proj/ADR-113-spiffe-disposition.md` exists and settles adopt/adapt/defer | manual (doc review) | `test -f proj/ADR-113-spiffe-disposition.md` | ❌ this phase creates it | ⬜ pending |
| TBD | TBD | — | NET-02 (SC2) | — | SPIFFE config round-trips through the profile schema | unit | `cargo test -p nono-sandbox-cli --lib -- profile::tests::test_validate_custom_credential_spiffe` | ❌ W0 — port from diff | ⬜ pending |
| TBD | TBD | — | NET-02 (SC2) | — | `RouteConfig.spiffe` serde round-trip | unit | `cargo test -p nono-sandbox-proxy --lib -- config::tests::test_spiffe_jwt_config_roundtrip` | ❌ W0 — port from diff | ⬜ pending |
| TBD | TBD | — | NET-02 (SC2/D-03) | T-fail-open | SPIFFE route with no live agent fails closed, never forwards unauthenticated | integration | `cargo test -p nono-sandbox-proxy --test spiffe_integration -- test_spiffe_jwt_fails_closed_on_missing_socket --nocapture` | ❌ W0 — port from diff (runs everywhere, no SPIRE) | ⬜ pending |
| TBD | TBD | — | NET-02 (D-03) | T-silent-unauth | A SPIFFE-declared route reached via CONNECT / forward-HTTP / external-proxy chain fails closed **at request time** | unit/integration | **new test — no upstream equivalent, must be authored** | ❌ W0 — fork-original | ⬜ pending |
| TBD | TBD | — | NET-02 (SC3) | — | ADR-86 boundary non-regressed: core-library additions are audit/telemetry data only | unit | `cargo test -p nono-sandbox --lib -- audit::tests` (extend with `spiffe_context: None,` fixtures) | ✓ existing, extend | ⬜ pending |
| TBD | TBD | — | NET-02 (SC4) | — | Cross-target clippy GREEN on both gates | build/lint | `cross clippy … x86_64-unknown-linux-gnu` + `cargo-zigbuild clippy … x86_64-apple-darwin`, both `-D warnings -D clippy::unwrap_used` | ✓ existing gate infra | ⬜ pending |
| TBD | TBD | — | NET-02 (SC4) | — | Bindings build green after the `nono-proxy` struct change | build | `maturin build` in `../nono-py` (after the `spiffe: None,` fix at `src/proxy.rs:218`); `napi build --platform --release` in `../nono-ts` (expected no-op) | ✓ existing gate infra | ⬜ pending |
| TBD | TBD | — | NET-02 (D-05b) | T-supply-chain | Dependency audit clean after `spiffe 0.16` lands | build/lint | `cargo audit` (cargo-audit 0.22.1 confirmed installed) | ✓ existing | ⬜ pending |
| TBD | TBD | — | NET-02 (D-05c) | T-supply-chain | JNI/Android path proven **not linked** on all three shipped targets | build | `cargo tree --target {x86_64-pc-windows-msvc,x86_64-unknown-linux-gnu,x86_64-apple-darwin} -e features -p nono-sandbox-proxy \| grep -i jni` → expect **zero output on all three**; plus `cargo tree -i jni` | ✓ existing | ⬜ pending |
| TBD | TBD | — | NET-02 (D-01) | — | No fork route type left silently unauthenticated by dropping the `tls_intercept` hunks | grep assertion | `grep -rn 'tls_intercept' crates/nono-proxy/src/ \| wc -l` → `0`, plus the positive per-route-path enumeration required by D-01 | ✓ existing tree | ⬜ pending |
| TBD | TBD | — | NET-02 (D-07) | — | Every skipped SPIRE-gated test is **named, counted, and surfaced** in the verification artifact | new mechanism | `cargo test … -- --nocapture 2>&1 \| grep -c '^SKIP\['` + named list in the verification report | ❌ W0 — this phase designs it | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/nono-proxy/tests/spiffe_integration.rs` — port from `c831dade`. 1 of 4 tests runs unconditionally (fail-closed, no SPIRE); 3 are `SPIRE_AGENT_SOCKET`-gated.
- [ ] `crates/nono-cli/tests/spiffe_run.rs` — port from `c831dade`. **100% live-agent-gated** (both tests early-`return` without `SPIRE_AGENT_SOCKET`); mock server is a pure-Rust `std::net::TcpListener`, **not** `scripts/spiffe-mock-server.py` — VERIFIED, so the local `cross` container's missing python3 does not affect it.
- [ ] **NEW, fork-original test** asserting D-03's fail-closed behavior on the CONNECT / forward-HTTP / external-proxy paths. Upstream has no equivalent because upstream *has* a `tls_intercept` implementation on those paths; this fork does not.
- [ ] The D-07 `SKIP[...]` marker convention + its grep-count verification step. **No existing fork infra to extend** — closest precedent is `socket_access_run.rs`'s ad-hoc un-aggregated `eprintln!`, which is precisely the failure this targets.
- [ ] `.github/workflows/spire.yml` CI lane (SPIRE 1.9.6) + `scripts/spire-test.sh` + `testdata/spire/{agent,server}.conf` — the only place live-agent tests actually execute.

---

## D-07 — Loud Skip Reporting (HARD ACCEPTANCE CRITERION)

A SPIFFE test that skips must be **named, counted, and surfaced** in the phase verification
artifact. "Tests passed" must never be mistakable for "the live path works."

**Why this is non-negotiable:** `crates/nono-cli/tests/socket_access_run.rs` reported `ok` in
~0.01 s across all its tests while exercising nothing (no python3 in the `cross` container), and
that nearly became evidence in Phase 112's verification.

**Mechanism (minimum, generalizes beyond SPIFFE):**
1. Every skip-capable test emits a structurally greppable stderr marker on skip —
   `eprintln!("SKIP[{}]: SPIRE_AGENT_SOCKET not set", module_path!())`, or a shared
   `test_skip!(reason)` macro (no shared test-helper module exists in `nono-proxy/tests/` today —
   this would be a new, minimal addition).
2. Verification runs `cargo test … -- --nocapture 2>&1 | grep -c '^SKIP\['` and records the count
   **and the names** in `human_verification_truths` / the verification report.
3. Because it is a plain stderr string convention rather than a new framework, any future
   host-gated test (GPU, python3, SPIRE) adopts the same prefix and is counted the same way.

**Expected phase-verification wording** (illustrative, not a template to copy blindly):
> "N SPIFFE tests SKIPPED (no local SPIRE agent): `<names>` — compensated by the new `spire.yml`
> CI lane, not locally exercised."

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `proj/ADR-113-spiffe-disposition.md` settles adopt-vs-adapt-vs-defer and discharges its proof obligations (D-01 positive proof, D-05 a/b/c, D-08/SC3 ADR-86 reading re-confirmed against the full diff, and the named SPIFFE-only scope boundary vs. `b1ecbc02`) | NET-02 / SC1 | Document-quality judgment; no assertion can prove an argument is sound | Read the ADR against `proj/ADR-111-resource-limits-boundary.md`'s shape. Confirm each proof obligation is discharged with a command + its output, not prose. |
| Live SPIFFE/SPIRE end-to-end path | NET-02 / SC2, D-06 | No SPIRE server/agent on this Windows dev host | Closed by `spire.yml`'s Linux CI lane, or by a reviewer running `bash scripts/spire-test.sh` on a Linux host with `SPIRE_AGENT_SOCKET` set. Must be reported per D-07 if not exercised. |
| ADR file actually committed | NET-02 / SC1 | `.gitignore:16`'s bare `proj/` pattern silently drops **new** files under `proj/` from `git add`/`git status` (already-tracked ADRs unaffected) — empirically reproduced during research | `git add -f proj/ADR-113-spiffe-disposition.md`, then `git show --stat HEAD` to confirm it is actually in the commit. Do **not** adopt upstream's `.gitignore` hunk. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without an automated verify
- [ ] Wave 0 covers all MISSING references above
- [ ] No watch-mode flags
- [ ] Feedback latency < 30 s for the per-task quick command
- [ ] D-07 skip count + names recorded in the verification artifact
- [ ] Both cross-target clippy gates GREEN **locally** (no PARTIAL→CI)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
