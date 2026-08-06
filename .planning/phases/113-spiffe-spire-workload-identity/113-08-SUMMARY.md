---
phase: 113-spiffe-spire-workload-identity
plan: 08
subsystem: proxy-network-security
tags: [spiffe, spire, adr, divergence-ledger, dependency-audit, cross-target-clippy, maturin, napi, nono-py, nono-ts]

# Dependency graph
requires:
  - phase: 113-spiffe-spire-workload-identity
    provides: "Plans 113-01 through 113-07's landed SPIFFE/SPIRE feature surface (spiffe.rs/auth.rs, config/schema types, oauth2/credential SPIFFE-assertion machinery, async RouteStore/CredentialStore::load, D-03 fail-closed guard, reverse-proxy handlers, test suite + D-07 skip mechanism + spire.yml CI lane) — this plan is the phase's evidence-and-record close-out, not a feature plan"
provides:
  - "proj/ADR-113-spiffe-disposition.md: binding disposition record (ADAPT-DOWN, D-01 positive proof; D-05 dependency review a/b/c with a corrected 9-new-crate count; D-08/SC3 ADR-86 re-confirmation with a named concept-leak caveat; OD-1 permanent scope boundary vs unabsorbed b1ecbc02; D-07 skip-reality + untested-path residuals)"
  - "108-DIVERGENCE-LEDGER.md: SPIFFE Carry-Forward Note (Phase 113, D-02) covering both the dropped tls_intercept hunks and the newly-identified b1ecbc02 divergence"
  - "../nono-py fixed and rebuilt green (3 files: proxy.rs, policy.rs, undo.rs — a genuinely larger break surface than the plan's single-field anticipation)"
  - "../nono-ts confirmed structurally immune, napi build re-run and green"
  - "Both mandatory cross-target clippy gates re-run fresh, GREEN, 0 errors"
  - "NET-02 marked Complete in REQUIREMENTS.md; Phase 113 marked complete in ROADMAP.md (checklist + all 8 plan boxes + Progress table row)"
affects: [114]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-05 dependency review distinguishes 'genuinely new to Cargo.lock' (9 packages, verified via git show on the exact commit that added the dependency) from 'already present, merely shared' (10 packages including jni/rustls-platform-verifier, verified via git log -S tracing them to a pre-existing sigstore/Trusted-Signing dependency stack unrelated to spiffe) — correcting 113-RESEARCH.md's undifferentiated '19 new crates' figure with real command evidence rather than inheriting it"
    - "JNI/Android cfg-gate proof run per-target (three cargo tree --target invocations) rather than trusting a single cargo tree -i jni, which requires a target to resolve at all and is ambiguous across two co-installed jni versions in this tree"

key-files:
  created:
    - proj/ADR-113-spiffe-disposition.md
  modified:
    - .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md
    - ../nono-py/src/proxy.rs
    - ../nono-py/src/policy.rs
    - ../nono-py/src/undo.rs

key-decisions:
  - "D-05(a) correction: only 9 packages (arc-swap, hyper-timeout, prost, prost-derive, prost-types, spiffe, tokio-stream, tonic, tonic-prost) are genuinely NEW to Cargo.lock as a direct result of this absorb — confirmed via git show on the exact commit (8e78f487) that added the spiffe dependency. The other 10 crates 113-RESEARCH.md's D-05 attributed to spiffe's pull (rustls-platform-verifier, jni, jni-macros, simd_cesu8, futures, pin-project(-internal), itertools, rand_pcg, jsonschema-regex, simdutf8) were already present in Cargo.lock before this phase, pulled by the fork's pre-existing sigstore/Trusted-Signing dependency stack (reqwest -> rustls-platform-verifier -> jni), not by spiffe at all — confirmed directly: `cargo tree -p spiffe -e features --target all | grep -i \"rustls-platform-verifier|jni\"` returns zero output. spiffe reaches the Workload API via tonic/hyper gRPC only."
  - "D-05(c) JNI cfg-gate proof: cargo tree --target <triple> -e features -p nono-sandbox-proxy | grep -i jni returns zero output on all 3 shipped targets (windows-msvc, linux-gnu, apple-darwin), run fresh this session. cargo tree -i jni is ambiguous (two jni versions, 0.21.1 and 0.22.4, both pre-existing) and requires --target all to resolve at all on this host's default target (which shows nothing without it) -- used --target all only as a secondary inspection to trace the pull path, not as the primary proof."
  - "cargo audit: exit 0, 6 pre-existing allowed advisory warnings (async-std, fxhash, paste, rustls-pemfile, anyhow, event-listener), none tracing through spiffe/tonic/prost. Zero vulnerabilities."
  - "The ADR corrects ROADMAP SC1's stale framing: SC1 asks the ADR to weigh 'the 545-deletion rewrite of the fork's divergent tls_intercept/reverse.rs' as one unit, but the fork has no tls_intercept module at all. The reverse.rs half is live and real; the tls_intercept half is superseded entirely by D-01. Recorded as a correction, not silently inherited."
  - "The nono-py break surface was genuinely larger than this plan's single anticipated fix (spiffe: None, after aws_auth: None, in proxy.rs). Direct build-fix-build discovered: (1) a SECOND exhaustive RustRouteConfig{..} literal in policy.rs's From<PolicyRouteConfig> impl, not named in 113-RESEARCH.md's D-09 finding or this plan's <interfaces> block; (2) three non-exhaustive-match compile errors (E0004) in proxy.rs's audit_event_to_py_dict for the four new core-library enum variants (SpiffeJwtBearer, SpiffeOAuthAssertion, SpiffeJwt, SpiffeUnsupportedPath); (3) a missing spiffe_context field (E0063) in undo.rs's NetworkAuditEvent{..} reconstruction literal inside set_network_events. All fixed; the reverse-direction (dict->Rust) string match arms for the 4 new enum variants were also added for round-trip symmetry with the newly-emitting forward direction (Rule 1 — a bug this plan's own emit-side fix would otherwise introduce). spiffe_context itself round-trips as None (no dict encoding of the structured SpiffeAuditContext payload exists in nono-py yet) — recorded as a named residual in the commit message and here, not fabricated."
  - "Pre-existing, unrelated cargo fmt --all --check non-compliance discovered in nono-py's override.rs (large diff, unrelated file, never touched by this fix) and an unrelated import-order line in proxy.rs (top-of-file, not touched by this fix) — confirmed via line-number cross-check that none of the fmt diff touches this plan's own edited lines. Out of scope per the Scope Boundary rule; not fixed, not silently claimed clean either."
  - "Full cargo test --workspace --no-fail-fast re-run fresh (not inherited from Plan 113-07's close): 4 target binaries failed (--bin nono, audit_attestation, env_vars, resl_nix_async_signal_safety), 17 named failing tests total, name-for-name identical to a subset of 111-04-VERIFICATION-NOTES.md's documented 24/27-name known-flake baseline. Zero new failure names. nono-sandbox-proxy's own suite (242 tests) and spiffe_integration.rs (5 tests) both 100% green as part of the same full-workspace sweep, not run in isolation."
  - "D-07 skip report re-run fresh, independently of Plan 113-07's own report, and produced the identical count: 4 SKIP in spiffe_integration.rs, 2 SKIP in spiffe_run.rs, 6 of 8 total."

requirements-completed: [NET-02]

# Metrics
duration: ~3h15min
completed: 2026-08-06
---

# Phase 113 Plan 08: ADR-113 Disposition + D-02 Ledger Note + D-05 Dependency Review + Binding Rebuilds + Cross-Target Gates Summary

**Closed Phase 113 with `proj/ADR-113-spiffe-disposition.md` discharging every proof obligation D-01/D-05/D-08/OD-1 placed on it by cited command output (not prose) — including a genuine correction to `113-RESEARCH.md`'s dependency-count claim (9 new crates, not 19, with the difference traced to a pre-existing sigstore dependency chain unrelated to `spiffe`) discovered by actually running the commands rather than inheriting the earlier static analysis; fixed a break surface in `../nono-py` three times larger than this plan's own anticipated single-field fix; and re-ran both mandatory cross-target clippy gates, `cargo fmt --all --check`, and the full workspace test suite fresh, confirming zero regressions against the documented baseline.**

## Performance

- **Duration:** ~3h15min (dominated by the ~30min full-workspace `cargo test --workspace --no-fail-fast` sweep, matching the documented `env_vars.rs` slow-test precedent from `111-04-VERIFICATION-NOTES.md`)
- **Started:** 2026-08-06 (session start)
- **Completed:** 2026-08-06
- **Tasks:** 3 completed
- **Files modified:** 6 across 2 repos (4 in `nono`, 3 in `../nono-py`)

## Accomplishments

- **Task 1:** `proj/ADR-113-spiffe-disposition.md` written and force-added past the `.gitignore:16` bare `proj/` landmine (`git add -f`), confirmed present in the commit via `git show --stat HEAD` (returns the file, not silently dropped). Discharges D-01's positive, grep-backed proof with a per-path enumeration table (reverse-proxy dispatch, CONNECT's pre-existing superset guard refined for audit precision, `handle_forward_http`'s genuinely-new D-03 guard) contrasted explicitly against `112-OAUTH-CAPTURE-DISPOSITION.md`'s SEC-02 case. `108-DIVERGENCE-LEDGER.md` gained the "SPIFFE Carry-Forward Note (Phase 113, D-02)" mirroring the SEC-09 Carry-Forward Note's exact shape, covering both the dropped ~478-line `tls_intercept` hunks and the newly-identified, previously-unledgered `b1ecbc02` divergence.
- **Task 2:** `cargo audit` re-run with `spiffe` live: exit 0, 6 pre-existing advisory warnings, zero vulnerabilities, none tracing through the new dependency tree. The 4 D-05(c) `cargo tree --target <triple>` JNI checks re-run fresh: zero output on all 3 shipped targets. Direct investigation (`git show`/`git log -S` on `Cargo.lock`) found `113-RESEARCH.md`'s "19 new crates" figure conflated genuinely-new packages with already-present ones pulled by an unrelated pre-existing dependency (the fork's sigstore/Trusted-Signing stack) — corrected to 9 genuinely-new packages, with the distinction proven by command output, not asserted. `../nono-py`'s break surface, investigated by actually running `cargo build --lib` rather than trusting the plan's single anticipated fix: two exhaustive struct literals (not one) plus three non-exhaustive-match compile errors on the new core-library enum variants plus a fourth struct-literal field — all fixed across `proxy.rs`/`policy.rs`/`undo.rs`. `maturin build` succeeded; `napi build --platform --release` in `../nono-ts` succeeded (confirmed no-op, zero `nono_proxy` references).
- **Task 3:** ADR force-add-committed and verified present in the commit. Both cross-target clippy gates (`cross clippy` linux-gnu, `cargo-zigbuild clippy` apple-darwin) re-run fresh over the fully-merged Wave 1-4 tree: GREEN, 0 errors, matching Plan 113-04's close (the phase's own dead_code watch-item, resolved at that plan, held with zero regressions through this final gate). `cargo fmt --all --check`: GREEN. `cargo test --workspace --no-fail-fast`: 4 target binaries failed, 17 named tests, a name-for-name strict subset of the documented 24/27-name known-flake baseline (`111-04-VERIFICATION-NOTES.md`) — zero new failure names. D-07 skip report re-run independently: 4 SKIP (`spiffe_integration.rs`) + 2 SKIP (`spiffe_run.rs`) = 6 of 8, identical to Plan 113-07's own count. `NET-02` flipped to Complete in `REQUIREMENTS.md` (both the requirement line and its traceability-table row); Phase 113's checklist box, all 8 plan checkboxes, and a new Progress-table row added by hand in `ROADMAP.md` (per the documented `roadmap.update-plan-progress` quirk).

## Task Commits

Each task was committed atomically, DCO-signed, in the repo(s) it touched:

1. **Task 1: ADR-113 + D-02 ledger carry-forward note** (nono repo) - `dbf735c5` (docs)
2. **Task 2: nono-py break-surface fix (3 files) + Cargo.lock update** (nono-py repo) - `485821f` (fix)
3. **Task 3: REQUIREMENTS.md/ROADMAP.md reconciliation + fresh gate re-run record** (nono repo) - `94fbaff3` (docs)

Task 2 produced no source changes in the `nono` repo itself — its evidence (cargo audit, cargo tree JNI checks, `maturin`/`napi` build confirmation) is cited in the ADR (Task 1's commit) and this SUMMARY rather than committed separately in `nono`.

**Plan metadata:** this SUMMARY's own commit is the final step.

## Files Created/Modified

- `proj/ADR-113-spiffe-disposition.md` (nono) - the phase's binding disposition record
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` (nono) - SPIFFE Carry-Forward Note
- `.planning/REQUIREMENTS.md` (nono) - NET-02 flipped to Complete
- `.planning/ROADMAP.md` (nono) - Phase 113 checklist, all 8 plan boxes, Progress-table row
- `src/proxy.rs` (nono-py) - `RustRouteConfig{..}` `spiffe: None,` field + 3 new match arms in `audit_event_to_py_dict`
- `src/policy.rs` (nono-py) - a second `RustRouteConfig{..}` literal's `spiffe: None,` field (`From<PolicyRouteConfig>`)
- `src/undo.rs` (nono-py) - `NetworkAuditEvent{..}` `spiffe_context: None,` field + 4 reverse-direction string match arms in `set_network_events`

## Decisions Made

See `key-decisions` in frontmatter for the full list with rationale. In brief: corrected `113-RESEARCH.md`'s D-05 "19 new crates" figure to a verified 9-genuinely-new/10-pre-existing-and-shared split; proved the JNI/Android cfg-gate closed per-target rather than via the ambiguous `cargo tree -i jni` alone; corrected ROADMAP SC1's stale `tls_intercept`/`reverse.rs` framing in the ADR; discovered and fixed a `../nono-py` break surface roughly 3x the size this plan's own `<interfaces>` block anticipated, adding round-trip symmetry for the new enum variants as a direct consequence (Rule 1); left an unrelated, pre-existing `nono-py` fmt/edition non-compliance untouched (out of scope, confirmed by line-number cross-check against this fix's own edits).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `../nono-py`'s break surface was larger than the plan's single anticipated fix**
- **Found during:** Task 2, running `cargo build --lib` in `../nono-py` after the plan's literal `spiffe: None,` fix
- **Issue:** The plan's `<interfaces>` block named exactly one break site (`src/proxy.rs`'s `RustRouteConfig{..}` literal). The real build surfaced 4 additional compile errors: a second exhaustive `RustRouteConfig{..}` literal in `src/policy.rs`'s `From<PolicyRouteConfig>` impl (E0063); three non-exhaustive-match errors (E0004) in `src/proxy.rs`'s `audit_event_to_py_dict` for the 4 new core-library enum variants this phase's Plan 113-01 added; and a missing `spiffe_context` field (E0063) in `src/undo.rs`'s `NetworkAuditEvent{..}` reconstruction literal.
- **Fix:** Fixed all 4 sites. For round-trip symmetry, also added the reverse-direction (Python-dict-string -> Rust-enum) match arms in `undo.rs::set_network_events` for the same 4 new variants — since `audit_event_to_py_dict` now emits those strings, leaving the reverse parser unable to accept them back would have been a genuine round-trip bug introduced by this very fix, not a pre-existing gap.
- **Files modified:** `../nono-py/src/proxy.rs`, `../nono-py/src/policy.rs`, `../nono-py/src/undo.rs`
- **Verification:** `cargo build --lib` exits 0; `maturin build` succeeds (wheel built); `cargo clippy -- -D warnings` exits 0.
- **Committed in:** `485821f` (nono-py repo)

**2. [Rule 2 - documentation correctness] `SpiffeAuditContext`'s dict round-trip has no encoding — recorded, not fabricated**
- **Found during:** Task 2, while fixing `undo.rs`'s `NetworkAuditEvent{..}` literal
- **Issue:** The structured `SpiffeAuditContext` payload (`workload_spiffe_id`, `trust_domain`, delegation chain, etc.) has no dict encoding anywhere in `nono-py`'s `audit_event_to_py_dict`/`set_network_events` pair — only the enum-tag strings are round-tripped. A SPIFFE-authenticated audit event's rich context is silently dropped on any dict round-trip today.
- **Fix:** Did not fabricate an encoding (out of this plan's narrow scope and Rule 4 territory — a new dict-schema decision). Set `spiffe_context: None,` with an inline comment naming the gap explicitly, so a future reader does not mistake the `None` for "this event never had SPIFFE context."
- **Files modified:** `../nono-py/src/undo.rs`
- **Verification:** Comment-only addition alongside the required field; `cargo build --lib` exits 0.
- **Committed in:** `485821f` (nono-py repo)

---

**Total deviations:** 2 (1 auto-fixed bug covering a larger-than-anticipated break surface, 1 documentation-correctness note recording a real, unfixed gap rather than papering over it). No scope creep — both stayed within the sibling-repo binding-rebuild obligation this plan's own `files_modified` scope already covers.
**Impact on plan:** Moderate on file-count (3 files in `../nono-py`, not 1), none on the security guarantee — the fix is purely additive struct-literal/match-arm completion required for the build to compile at all; no behavior changed for any pre-existing (non-SPIFFE) route or audit event.

## Issues Encountered

**Pre-existing `cargo fmt --all --check` non-compliance in `../nono-py`, unrelated to this fix, confirmed out of scope.** Running `cargo fmt --all -- --check` on the full `nono-py` repo surfaces a large diff, entirely in `src/override.rs` (a file this fix never touched) plus one unrelated import-ordering line at the top of `src/proxy.rs` and several pre-existing let-chain edition-mismatch errors in `policy.rs`/`undo.rs` at line numbers far from this fix's own edits. Cross-checked directly: none of the fmt diff's line numbers overlap this fix's own edited lines (`proxy.rs` ~48-95, `policy.rs` ~755, `undo.rs` ~528-624). Left untouched per the Scope Boundary rule ("Only auto-fix issues DIRECTLY caused by the current task's changes"); `cargo build --lib` and `cargo clippy -- -D warnings` (the repo's own stated CI gates per its `CLAUDE.md`) both pass clean.

**Full-workspace `cargo test --workspace --no-fail-fast` took ~27 minutes**, dominated by `env_vars.rs`'s known-slow test file (matches `111-04-VERIFICATION-NOTES.md`'s documented ~45-minute precedent for the same file; this run was faster, likely due to warm caches). Run via a background job with periodic polling rather than a single blocking foreground call, to avoid the harness's 600s auto-promotion behavior documented as having silently killed a prior attempt in `111-04-VERIFICATION-NOTES.md`.

## User Setup Required

None — no external service configuration required. The D-07 skip report (6 of 8 SPIFFE tests skipping) reflects the absence of a local SPIRE agent on this Windows dev host, a standing, documented, non-actionable fact for this host — not something requiring user setup. The live path is closed by `.github/workflows/spire.yml`'s CI lane or by a reviewer running `bash scripts/spire-test.sh` on Linux/macOS with a live `SPIRE_AGENT_SOCKET`.

## Known Residuals (carried forward, not silently closed)

These are named explicitly in `proj/ADR-113-spiffe-disposition.md` and repeated here per this plan's own instruction to report honestly:

1. **6 of 8 SPIFFE tests SKIP locally** (no local SPIRE agent) — compensated by the `spire.yml` CI lane, not locally exercised on this host. "Tests passed" on this host must never be read as "the live SPIFFE path works."
2. **`handle_spiffe_assertion_credential`'s (OAuth2 jwt-bearer exchange path's) successful-forward path has no test coverage anywhere, local or CI.** `SpiffeAssertionTokenCache::new` performs a live initial token exchange with no test-only bypass constructor; proving this path would require simultaneous live SPIRE + live OAuth2 IdP dependencies, or a production-code change Plan 113-07 explicitly declined to force. By contrast, `handle_spiffe_route`'s direct-bearer-injection path IS proven end-to-end (CI-only) via `spiffe_run.rs`.
3. **`SpiffeAuditContext`'s structured payload has no dict encoding in `../nono-py`** — a SPIFFE-authenticated audit event's rich context (workload SPIFFE ID, trust domain, delegation chain) is silently lost on any Python-side dict round-trip today, though the enum-tag strings (`auth_mechanism`, `injection_mode`, `denial_category`) do round-trip correctly. Named here as a residual for whichever future plan touches `nono-py`'s audit dict schema.
4. **Pre-existing `cargo fmt --all --check` non-compliance in `../nono-py`'s `src/override.rs`** and a top-of-file import-order line in `src/proxy.rs`, confirmed unrelated to this plan's own edits (out of scope, not fixed).

None of these prevent NET-02 or Phase 113's stated Success Criteria from being satisfied — SC1-SC4 are all satisfied per the evidence in this SUMMARY and the ADR — but per this plan's own mandate, they must not be smoothed over.

## Verification Results

- `test -f proj/ADR-113-spiffe-disposition.md` — **PASS**.
- `git show --stat HEAD` (commit `dbf735c5`) — **CONFIRMS** `proj/ADR-113-spiffe-disposition.md` present (Landmine L2 avoided via `git add -f`).
- `grep -c "b1ecbc02" proj/ADR-113-spiffe-disposition.md` — returns `>= 1`.
- `grep -c "SPIFFE Carry-Forward Note" .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — returns `1`.
- `grep -c "b1ecbc02" .planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — returns `>= 1`.
- `cargo audit` — **GREEN**, exit 0, 6 pre-existing advisories, 0 vulnerabilities.
- `cargo tree --target {x86_64-pc-windows-msvc,x86_64-unknown-linux-gnu,x86_64-apple-darwin} -e features -p nono-sandbox-proxy | grep -i jni` — **zero output on all 3**.
- `maturin build` (`../nono-py`) — **GREEN**, wheel built.
- `napi build --platform --release` (`../nono-ts`) — **GREEN**, confirmed no-op.
- `cross clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` — **GREEN**, 0 errors.
- `cargo-zigbuild clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` (`SDKROOT` unset) — **GREEN**, 0 errors.
- `cargo fmt --all --check` (nono repo) — **GREEN**.
- `cargo test --workspace --no-fail-fast` (nono repo) — RED (exit 101, expected) — 4 target binaries / 17 named tests failed, strict subset of the documented 24/27-name baseline, zero new names.
- `cargo test -p nono-sandbox-proxy --test spiffe_integration -- --nocapture | grep -c '^SKIP\['` — `4`.
- `cargo test -p nono-sandbox-cli --test spiffe_run -- --nocapture | grep -c '^SKIP\['` — `2`.

## Next Phase Readiness

- Phase 113 is closed: `proj/ADR-113-spiffe-disposition.md` exists, is committed, and discharges every proof obligation with cited command output. `108-DIVERGENCE-LEDGER.md` carries the D-02 note. `NET-02` is Complete in `REQUIREMENTS.md`; Phase 113 is complete in `ROADMAP.md`.
- Both sibling bindings (`../nono-py`, `../nono-ts`) rebuild green against the full Phase 113 diff.
- **For Phase 114 (OAuth Capture Absorb):** shares `oauth2.rs`/`credential.rs`/`tls_intercept` surface with this phase per `ROADMAP.md`'s stated Phase 114 dependency. That phase's planner should read this ADR's OD-1 section (the `b1ecbc02` scope boundary) before touching `credential.rs`'s SPIFFE-assertion-route machinery, to avoid silently restructuring symbols this phase built.
- **Named residuals carried forward** (see "Known Residuals" above): the untested `handle_spiffe_assertion_credential` successful-forward path, and `nono-py`'s missing `SpiffeAuditContext` dict encoding, are both open items for whichever future plan next touches this surface.
- No blockers.

---
*Phase: 113-spiffe-spire-workload-identity*
*Completed: 2026-08-06*

## Self-Check: PASSED

All claimed created/modified files confirmed present on disk: `proj/ADR-113-spiffe-disposition.md`,
`.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md`, `.planning/REQUIREMENTS.md`,
`.planning/ROADMAP.md` (nono repo); `src/proxy.rs`, `src/policy.rs`, `src/undo.rs` (nono-py repo).
All 3 claimed commit hashes confirmed present in their respective repos' `git log --oneline --all`:
`dbf735c5`/`94fbaff3` (nono), `485821f` (nono-py).
