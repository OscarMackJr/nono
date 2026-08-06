---
phase: 113
slug: spiffe-spire-workload-identity
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-08-06
---

# Phase 113 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `113-RESEARCH.md` § "Validation Architecture". Reconciled against the finished
> 8-plan set (113-01 through 113-08) — every Task ID / Plan / Wave below is a real value read from
> the landed PLAN.md files, not a placeholder. The executor flips Status columns during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner (`cargo test`); `#[tokio::test]` for async |
| **Config file** | none — standard `Cargo.toml` test targets. New CI lane: `.github/workflows/spire.yml` (Plan 113-07) |
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

*Reconciled against the 8 landed PLAN.md files (113-01 through 113-08). Every row's Automated
Command was cross-checked against the owning task's own `<verify>`/`<acceptance_criteria>` text;
no mismatches requiring a plan fix were found — where the plan's command is broader than
`113-RESEARCH.md`'s originally-suggested filter (e.g. a whole-file `--lib` run instead of a single
test name), the plan's actual command is recorded below as the binding one, since it is a superset
that still exercises the narrower behavior.*

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 113-08-T1 | 113-08 | 5 | NET-02 (SC1) | T-113-21 | `proj/ADR-113-spiffe-disposition.md` exists and settles adopt/adapt/defer with a positive, grep-backed D-01 proof | manual (doc review) + grep | `test -f proj/ADR-113-spiffe-disposition.md` | ❌ this phase creates it | ⬜ pending |
| 113-02-T3 | 113-02 | 1 | NET-02 (SC2) | — | SPIFFE config round-trips through the profile schema (mutual exclusion + validation) | unit | `cargo test -p nono-sandbox-cli --lib -- profile::tests::test_validate_custom_credential_spiffe` | ❌ W1 — port from diff | ⬜ pending |
| 113-02-T1 | 113-02 | 1 | NET-02 (SC2) | — | `RouteConfig.spiffe` serde round-trip (present value + absent-by-default) | unit | `cargo test -p nono-sandbox-proxy --lib -- config::tests::test_spiffe_jwt_config_roundtrip` (sibling: `config::tests::test_spiffe_absent_by_default`, same task) | ❌ W1 — **fixed by this revision**: both tests are now explicitly ported in the task's `<action>`/`<interfaces>`, not implied by build success (BLOCKER 1 closure) | ⬜ pending |
| 113-01-T3 | 113-01 | 1 | NET-02 (SC2/D-03) | T-113-03 | `SpiffeJwtSource::connect()` fails closed at the module level with no live agent | unit | `cargo test -p nono-sandbox-proxy --lib -- spiffe::tests auth::tests` (covers `test_jwt_source_fails_closed_on_missing_socket`) | ✓ landed this task | ⬜ pending |
| 113-07-T1 | 113-07 | 4 | NET-02 (SC2/D-03) | T-113-03 | SPIFFE route with no live agent fails closed at proxy-integration level, never forwards unauthenticated | integration | `cargo test -p nono-sandbox-proxy --test spiffe_integration -- test_spiffe_jwt_fails_closed_on_missing_socket --nocapture` | ❌ W4 — port from diff (runs everywhere, no SPIRE) | ⬜ pending |
| 113-05-T2 | 113-05 | 3 | NET-02 (D-03) | T-113-13 | A SPIFFE-declared route reached via CONNECT / forward-HTTP / external-proxy chain is denied at request time (unit-level guard) | unit | `cargo test -p nono-sandbox-proxy --lib` (new test in `server.rs`'s own `mod tests`, per Task 2's acceptance criteria) | ❌ W3 — fork-original, no upstream equivalent | ⬜ pending |
| 113-07-T1 | 113-07 | 4 | NET-02 (D-03) | T-113-13 | Same guard, exercised end-to-end via a real `nono_proxy::server::start` call (fork-original integration test, no upstream equivalent) | integration | `cargo test -p nono-sandbox-proxy --test spiffe_integration -- --nocapture` | ❌ W4 — fork-original | ⬜ pending |
| 113-01-T1 | 113-01 | 1 | NET-02 (SC3) | — | ADR-86 boundary non-regressed: core-library additions are audit/telemetry data only | unit | `cargo test -p nono-sandbox --lib -- undo:: audit::` | ✓ existing, extended with `spiffe_context: None,` fixtures | ⬜ pending |
| 113-08-T3 | 113-08 | 5 | NET-02 (SC4) | T-113-22 | Cross-target clippy GREEN on both gates | build/lint | `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` + `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` | ✓ existing gate infra | ⬜ pending |
| 113-08-T2 | 113-08 | 5 | NET-02 (SC4) | — | Bindings build green after the `nono-proxy` struct change | build | `maturin build` (in `../nono-py`, after the `spiffe: None,` fix at `src/proxy.rs:218`); `napi build --platform --release` (in `../nono-ts`, expected no-op) | ✓ existing gate infra | ⬜ pending |
| 113-08-T2 | 113-08 | 5 | NET-02 (D-05b) | T-113-20 / T-113-SC | Dependency audit clean after `spiffe 0.16` lands | build/lint | `cargo audit` (cargo-audit 0.22.1 confirmed installed) | ✓ existing | ⬜ pending |
| 113-08-T2 | 113-08 | 5 | NET-02 (D-05c) | T-113-20 / T-113-SC | JNI/Android path proven **not linked** on all three shipped targets | build | `cargo tree --target {x86_64-pc-windows-msvc,x86_64-unknown-linux-gnu,x86_64-apple-darwin} -e features -p nono-sandbox-proxy \| grep -i jni` → expect **zero output on all three**; plus `cargo tree -i jni` | ✓ existing | ⬜ pending |
| 113-08-T1 | 113-08 | 5 | NET-02 (D-01) | T-113-13 | No fork route type left silently unauthenticated by dropping the `tls_intercept` hunks | grep assertion + doc | `grep -rn 'tls_intercept' crates/nono-proxy/src/ \| wc -l` → `0`, plus the positive per-route-path enumeration in ADR-113 (reverse-proxy via `handle_spiffe_route`/`handle_spiffe_assertion_credential`; CONNECT/forward-HTTP/external-proxy via the D-03 guard) | ✓ existing tree (module already absent) | ⬜ pending |
| 113-07-T1 + 113-07-T2 | 113-07 | 4 | NET-02 (D-07) | T-113-19 | Every skip-capable SPIRE-gated test emits a structurally greppable `SKIP[...]` marker on skip (mechanism, not report) | grep assertion | `grep -c "SKIP\[" crates/nono-proxy/tests/spiffe_integration.rs` (≥3) and `grep -c "SKIP\[" crates/nono-cli/tests/spiffe_run.rs` (=2) | ❌ W4 — this phase designs and lands it | ⬜ pending |
| 113-08-T3 | 113-08 | 5 | NET-02 (D-07) | T-113-19 | Every skipped SPIRE-gated test is **named, counted, and surfaced** in the phase's verification artifact (report, not just mechanism) | new mechanism / recorded confirmation | `cargo test -p nono-sandbox-proxy --test spiffe_integration -- --nocapture 2>&1 \| grep -c '^SKIP\['` + same for `-p nono-sandbox-cli --test spiffe_run`; both counts and the full named list recorded in the SUMMARY per `113-VALIDATION.md`'s wording template | ❌ W5 — this task produces the report | ⬜ pending |
| 113-04-T1 + 113-03-T2 + 113-05-T1 | 113-04 / 113-03 / 113-05 | 2 / 2 / 3 | NET-02 (D-04) | T-113-11 | `RouteStore::load`/`CredentialStore::load` async conversion: a `spiffe`-declared route with an unreachable socket fails the WHOLE call closed; non-SPIFFE profiles are behavior-unchanged; both production `.await` call sites in `server.rs` land | unit + build | `cargo test -p nono-sandbox-proxy --lib -- route:: credential::` (113-04-T1/113-03-T2) + `cargo test -p nono-sandbox-proxy --lib` (113-05-T1, full-suite regression check) | ❌ W2/W3 — D-04's actual blast radius (1 production site + 7 test sites, all in `nono-proxy`, per 113-RESEARCH.md's Blast-Radius Analysis) | ⬜ pending |
| 113-03-T2 + 113-04-T1 | 113-03 / 113-04 | 2 | NET-02 (OD-1) | — | The general (declined) `oauth2_routes`/`OAuth2Route`/`get_oauth2()`/`lookup_all_by_upstream`/`has_intercept_route`/`requires_managed_credential` machinery is provably absent — the SPIFFE-only slice boundary is grep-verifiable, not merely asserted | grep assertion | `grep -c "oauth2_routes\|struct OAuth2Route\|fn get_oauth2" crates/nono-proxy/src/credential.rs` (=0) and `grep -c "lookup_all_by_upstream\|has_intercept_route\|requires_managed_credential" crates/nono-proxy/src/route.rs` (=0) | ❌ W2 — grep gate authored alongside the build-loop it constrains | ⬜ pending |
| 113-04-T2 | 113-04 | 2 | NET-02 (OD-2) | — | `route.rs`'s misattributed "belongs to tls_intercept" comment is corrected to cite the real ancestor (`b1ecbc02`) | grep assertion | `grep -c "belong to the upstream tls_intercept module" crates/nono-proxy/src/route.rs` (=0) and `grep -c "b1ecbc02" crates/nono-proxy/src/route.rs` (≥1) | ✓ existing comment, corrected in place | ⬜ pending |
| 113-01-T3 + 113-02-T3 | 113-01 / 113-02 | 1 | NET-02 (Landmine L1) | — | Both ported `&& let` let-chains (`spiffe.rs::check_nbf`, `proxy_runtime.rs`'s host-collection loop) are rewritten to nested `if let` before `cargo fmt --all --check` | grep assertion + fmt gate | `grep -c "&& let" crates/nono-proxy/src/spiffe.rs` (=0) and `grep -c "&& let" crates/nono-cli/src/proxy_runtime.rs` (=0), plus `cargo fmt --all --check` | ❌ W1 — fixed at the point each let-chain is ported | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/nono-proxy/tests/spiffe_integration.rs` — port from `c831dade`. 1 of 4 tests runs unconditionally (fail-closed, no SPIRE); 3 are `SPIRE_AGENT_SOCKET`-gated. **Covered by Plan 113-07, Task 1.**
- [x] `crates/nono-cli/tests/spiffe_run.rs` — port from `c831dade`. **100% live-agent-gated** (both tests early-`return` without `SPIRE_AGENT_SOCKET`); mock server is a pure-Rust `std::net::TcpListener`, **not** `scripts/spiffe-mock-server.py` — VERIFIED, so the local `cross` container's missing python3 does not affect it. **Covered by Plan 113-07, Task 2.**
- [x] **NEW, fork-original test** asserting D-03's fail-closed behavior on the CONNECT / forward-HTTP / external-proxy paths. Upstream has no equivalent because upstream *has* a `tls_intercept` implementation on those paths; this fork does not. **Covered at the unit level by Plan 113-05, Task 2 (the guard itself) and end-to-end by Plan 113-07, Task 1 (the new fork-original integration test).**
- [x] The D-07 `SKIP[...]` marker convention + its grep-count verification step. **No existing fork infra to extend** — closest precedent is `socket_access_run.rs`'s ad-hoc un-aggregated `eprintln!`, which is precisely the failure this targets. **Marker convention covered by Plan 113-07, Tasks 1 and 2; the counted/named verification report covered by Plan 113-08, Task 3.**
- [x] `.github/workflows/spire.yml` CI lane (SPIRE 1.9.6) + `scripts/spire-test.sh` + `testdata/spire/{agent,server}.conf` — the only place live-agent tests actually execute. **Covered by Plan 113-07, Task 3.**

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
   this would be a new, minimal addition). **Lands in Plan 113-07, Tasks 1/2.**
2. Verification runs `cargo test … -- --nocapture 2>&1 | grep -c "^SKIP\["` and records the count
   **and the names** in `human_verification_truths` / the verification report. **Lands in Plan
   113-08, Task 3.**
3. Because it is a plain stderr string convention rather than a new framework, any future
   host-gated test (GPU, python3, SPIRE) adopts the same prefix and is counted the same way.

**Expected phase-verification wording** (illustrative, not a template to copy blindly):
> "N SPIFFE tests SKIPPED (no local SPIRE agent): `<names>` — compensated by the new `spire.yml`
> CI lane, not locally exercised."

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `proj/ADR-113-spiffe-disposition.md` settles adopt-vs-adapt-vs-defer and discharges its proof obligations (D-01 positive proof, D-05 a/b/c, D-08/SC3 ADR-86 reading re-confirmed against the full diff, and the named SPIFFE-only scope boundary vs. `b1ecbc02`) | NET-02 / SC1 | Document-quality judgment; no assertion can prove an argument is sound | Read the ADR against `proj/ADR-111-resource-limits-boundary.md`'s shape. Confirm each proof obligation is discharged with a command + its output, not prose. Covered by Plan 113-08, Task 1. |
| Live SPIFFE/SPIRE end-to-end path | NET-02 / SC2, D-06 | No SPIRE server/agent on this Windows dev host | Closed by `spire.yml`'s Linux CI lane (Plan 113-07, Task 3), or by a reviewer running `bash scripts/spire-test.sh` on a Linux host with `SPIRE_AGENT_SOCKET` set. Must be reported per D-07 if not exercised (Plan 113-08, Task 3). |
| ADR file actually committed | NET-02 / SC1 | `.gitignore:16`'s bare `proj/` pattern silently drops **new** files under `proj/` from `git add`/`git status` (already-tracked ADRs unaffected) — empirically reproduced during research | `git add -f proj/ADR-113-spiffe-disposition.md`, then `git show --stat HEAD` to confirm it is actually in the commit. Do **not** adopt upstream's `.gitignore` hunk. Covered by Plan 113-08, Task 3. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or a Wave 0 dependency — confirmed against all 21 tasks
  across the 8 landed plans (113-01: 3, 113-02: 3, 113-03: 2, 113-04: 2, 113-05: 3, 113-06: 2,
  113-07: 3, 113-08: 3); every one carries an `<automated>` command in its own `<verify>` block.
- [x] Sampling continuity: no 3 consecutive tasks without an automated verify — trivially true,
  since 0 of 21 tasks lack one.
- [x] Wave 0 covers all MISSING references above — see the ticked Wave 0 Requirements section,
  each item now names its covering task.
- [x] No watch-mode flags
- [x] Feedback latency < 30 s for the per-task quick command (`cargo test -p nono-sandbox-proxy
  --lib --test spiffe_integration -- --nocapture`, ~10s per Test Infrastructure table above)
- [ ] D-07 skip count + names recorded in the verification artifact — *verified at execution, not
  plan time; mechanism designed and landed in Plan 113-07 (Tasks 1/2), the report is produced in
  Plan 113-08 (Task 3)*
- [ ] Both cross-target clippy gates GREEN **locally** (no PARTIAL→CI) — *verified at execution,
  not plan time; gated by Plan 113-08, Task 3*
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
