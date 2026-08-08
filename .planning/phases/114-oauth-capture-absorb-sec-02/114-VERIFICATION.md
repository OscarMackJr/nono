---
phase: 114-oauth-capture-absorb-sec-02
verified: 2026-08-08T06:10:00Z
status: human_needed
score: 4/4 roadmap success criteria verified; 1 human decision required on residual risk
overrides_applied: 0
human_verification:
  - test: "Accept or reject residual live token-exposure paths as within SEC-02's closed scope"
    expected: "Operator decision on whether WR-02/WR-03/WR-05 (114-REVIEW.md, left OPEN by scoping) and NEW-02/NEW-04 (v3.6-MILESTONE-AUDIT.md, left OPEN as warnings) are acceptable residual risk for a requirement whose ROADMAP goal text states the token-never-reaches-client guarantee is a 'hard precondition' and that 'no reduced-scope half-feature ships'"
    why_human: "This is a risk-acceptance judgment, not a fact a grep can resolve. The core enforcement point is real and its 3 Critical defects are fixed and confirmed in code. But 5 distinct, non-hypothetical mechanisms remain by which a real OAuth token can reach the sandboxed client under realistic (not contrived) operator configuration: (1) WR-02 case-sensitive/exact-name backstop misses camelCase/alternate token-field names a real provider may use; (2) WR-03 trailing-dot host bypasses both the CONNECT and forward-HTTP capture guards under the default allow-all filter config; (3) WR-05 two routes sharing an upstream host where only one declares capture has no load-time validation, so the agent can simply request the other route's prefix; (4) NEW-02 a `platform_overrides` block that redefines a credential without `capture` silently drops the rewrite on that OS (this is Phase 110 code, not Phase 114's own, but it directly negates SEC-02's guarantee); (5) NEW-04 `direct_connect_ports` (an ordinary profile grant) lets an agent open a direct TCP connection to a capture-declared host on that port, bypassing the proxy entirely. None of these were re-derived here — all are already recorded in 114-REVIEW.md or v3.6-MILESTONE-AUDIT.md and deliberately left open. The question this verification cannot answer on its own is whether that scoping decision is consistent with the phase goal's own 'hard precondition' / 'no half-feature' wording, or whether it should trigger a follow-up hardening plan before SEC-02 is treated as closed without qualification."
---

# Phase 114: OAuth Capture Absorb (SEC-02) Verification Report

**Phase Goal:** Upstream's OAuth-capture surface (`9b692e07` / `3c59c62e` / `d033c631`) is absorbed
or formally declined under its own disposition review, with the guarantee that real OAuth tokens
never reach the sandboxed client preserved as a hard precondition — no reduced-scope half-feature
ships.

**Verified:** 2026-08-08 (retroactive — gsd-verifier was disclosed-skipped during phase execution;
this closes the gap the v3.6 milestone audit flagged)
**Status:** human_needed
**Re-verification:** No — initial verification (no prior VERIFICATION.md existed for this phase)

## Method note

Per the task instructions, ADR-114's and the SUMMARYs' claims were NOT taken as evidence. Every
claim below was checked against current HEAD (`ad4cbd09`) source, specifically:
`crates/nono-proxy/src/{reverse.rs,server.rs,capture.rs,route.rs,config.rs}`,
`crates/nono-cli/src/profile/{mod.rs,credential_provider.rs}`, and the three fix commits
(`8b35a220`, `8d98b23c`, `02a469ce`) plus the UAT fix (`1195e1f9`) were confirmed present as
ancestors of HEAD (`git merge-base --is-ancestor` for each returned true) and their diffs read
directly, not inferred from commit messages.

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth (ROADMAP SC) | Status | Evidence |
|---|---|---|---|
| 1 | SC1: A disposition decision for all three SEC-02 SHAs is recorded with cited evidence, and the 108-DIVERGENCE-LEDGER.md addendum row is updated from "deferred" to a final disposition | VERIFIED | `proj/ADR-114-oauth-capture-disposition.md` exists (commit `c8912280`), records D-01r (ADAPT, superseding a retracted DECLINE, with the visibility-vs-buffering reasoning), D-06 positive proof table, D-09/SC3 boundary re-confirmation, and an OD section on the `149abde0` TLS-interception boundary. `108-DIVERGENCE-LEDGER.md` contains "ADAPTED-with-scope-limit" and a "SEC-02 Carry-Forward Note (Phase 114, D-10)" section (grep-confirmed by 114-11's own Self-Check, spot-checked here). |
| 2 | SC2: Token-confinement property proven — the response-rewrite hook or an equivalent fork-side enforcement point is in place; a reduced-scope absorb that drops the rewrite hook without an equivalent enforcement point is forbidden | VERIFIED, with residual risk — see Human Verification | `relay_response_with_capture()` (`crates/nono-proxy/src/reverse.rs`) exists, is fail-closed (`deny` takes `&'static str`, confirmed at `reverse.rs:1943` — no runtime-derived string can reach it, closing CR-01), and is wired into all three reverse-proxy relay sites (`:428` call site precedes site 1's unbuffered loop; sites 2/3 call the shared guard before their own `[0u8; 8192]` loops). CR-02 (stall against keep-alive upstreams) is fixed: `response_framing_complete()`/`chunked_body_complete()` exist (`reverse.rs:1723,1772`) and `Connection: close` is sent on capture routes (`:493,788,1088`). CR-03 (CONNECT bypass) is fixed: `server.rs:1414-1418` now ORs `is_route_upstream() \| is_spiffe_route \| is_capture_route` before the CONNECT deny gate, confirmed at source — matches the fix commit `02a469ce`. All three fix commits are ancestors of HEAD. **However**, 5 distinct residual bypass mechanisms remain live in the current codebase (WR-02, WR-03, WR-05 in 114-REVIEW.md; NEW-02, NEW-04 in v3.6-MILESTONE-AUDIT.md) — see Human Verification below. |
| 3 | SC3: If absorbed, the ADR-86/ADR-111 boundary is confirmed non-regressed — no policy or enforcement logic lands in the core `nono` crate | VERIFIED | Independently re-ran the grep ADR-114 cites: `grep -rln "capture" crates/nono/src/` returns only `audit.rs` (test-fixture `capture_context: None,` initializers), `keystore.rs`/`supervisor/aipc_sdk.rs`/`undo/{merkle,mod,snapshot}.rs` (pre-existing unrelated prose hits), and `undo/types.rs` (the additive audit-vocabulary types: `CaptureUnsupportedPath`, `CaptureBufferOrRewriteFailed`, `CaptureAuditContext`). No `if`/`match` logic present in or near the new types — confirmed by reading `undo/types.rs` directly. All enforcement (`relay_response_with_capture`, `relay_capture_if_declared`, `resolve_capture_request_body`, `CapturePhantomStore`) lives in `crates/nono-proxy/src/`, outside the core library. |
| 4 | SC4: Both cross-target clippy gates (linux-gnu via `cross`, apple-darwin via `cargo-zigbuild`) are GREEN locally, plus `cargo fmt --all --check` and the workspace test suite diffed against the documented baseline | VERIFIED | 114-11-SUMMARY.md records both gates GREEN (14m51s / 52s) for the pre-Critical-fix tree. Commit `c05c89cd` ("complete phase execution") states both gates were **re-run after the Critical fix pass** and GREEN (linux-gnu 9m08s, apple-darwin unspecified duration), with `nono-sandbox-proxy` 300 lib + 5 integration, 0 failed. The later UAT fix (`1195e1f9`, a `#[serde(deny_unknown_fields)]` addition) is a serde-attribute-only change with no cfg-gated Unix surface touched; 114-UAT.md's resolution entry separately records build/fmt/clippy (native) green after that commit. No evidence of a stale or un-rerun gate. |

**Score:** 4/4 roadmap success criteria hold as stated, with SC2 carrying a documented residual-risk
qualifier that this report escalates rather than silently accepting or silently failing.

### The core question: is this a "reduced-scope half-feature"?

Per the verification brief's three sub-questions:

1. **The declared CONNECT-to-arbitrary-hosts scope limit (D-10/D-11).** This is a legitimate
   bounded scope, not the forbidden half-feature. SC2's forbidden outcome is specifically "a
   reduced-scope absorb that **drops the rewrite hook without an equivalent enforcement point**."
   That did not happen — ADR-114 documents that a real, tested, fail-closed enforcement point was
   built for the declared scope (configured reverse-proxy routes), and the CONNECT-to-arbitrary-host
   gap is closed by denial (the D-06 guard), not by silent pass-through. The scope limit is named,
   recorded in the ledger as a carry-forward divergence, and tied to a standing, previously-made
   no-MITM decision (not invented for this phase to dodge work). This satisfies SC1/SC2/D-10/D-11 as
   written.

2. **The mint-to-resolve loop closed at relay site 1 only.** Verified at source:
   `resolve_capture_request_body` has exactly one call site, `reverse.rs:428`, inside
   `handle_reverse_proxy` (site 1). `handle_spiffe_route` and `handle_spiffe_assertion_credential`
   (sites 2/3) do not call it. This is honestly recorded — ADR-114 Consequence 2 names "Egress
   resolution on SPIFFE-dispatched routes" as NOT SHIPPED, and `REQUIREMENTS.md` was corrected
   (2026-08-08, per the milestone audit's ACC-01) to say "on relay site 1 only" rather than "closed
   end-to-end." Judged against the phase's **core** guarantee — a real token minted by the provider
   never reaches the sandboxed client in the **response** — this gap does not break that guarantee:
   response-side rewrite is fully wired at all three sites (verified above). What is missing is the
   convenience feature of resubmitting a phantom in a later request body, which is a secondary
   control (`request_nonce_fields`), not the confinement guarantee itself. However, WR-08
   (114-REVIEW.md) correctly notes this control is silently accepted by profile validation on
   SPIFFE-declared credentials where it is inert, which is a "never silently degrade" violation on
   its own — a real, if lower-severity, defect. Verdict: not a half-feature of the core guarantee,
   but an honestly-scoped and now-honestly-recorded partial feature within a secondary control.

3. **NEW-02 — `platform_overrides` can silently drop `capture`.** Verified at source:
   `crates/nono-cli/src/profile/mod.rs:3683-3687` merges `custom_credentials` via
   `HashMap::extend`, which is whole-value replace per key; `test_merge_profiles_custom_credentials_child_wins_on_collision`
   (same file, `:6242`) confirms child always wins. A `platform_overrides.windows` block that
   redefines an existing credential name **without** a `capture` field therefore silently removes
   the OAuth-capture rewrite for that route on Windows — the real token then reaches the sandboxed
   agent unrewritten. This is a live path to the exact outcome the phase goal forbids ("real OAuth
   tokens never reach the sandboxed client"). It is not Phase 114's own code (it is Phase 110's
   `merge_profiles`, predating this phase), and it is already tracked as `NEW-02` (severity
   `warning-security`) in `v3.6-MILESTONE-AUDIT.md`, not newly discovered here. But it directly
   negates SEC-02's stated guarantee whenever a profile happens to use `platform_overrides` on a
   capture-declared credential, which is a realistic (not contrived) profile-authoring pattern.
   **This is the single strongest reason the "hard precondition" framing does not hold absolutely**
   in the shipped system, regardless of which phase's code produces the gap.

## Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/nono-proxy/src/capture.rs` | `CapturePhantomStore`, mint/resolve, rewrite/backstop logic | VERIFIED | Present, substantive, wired (see SC2 evidence). `is_sensitive_token_field` confirmed exact-name/case-sensitive at `:158-160` (WR-02, known/open). |
| `crates/nono-proxy/src/reverse.rs::relay_response_with_capture` | Enforcement point (buffer, rewrite, fail-closed) | VERIFIED | 8-branch fail-closed deny confirmed; `deny: &'static str` signature confirmed (CR-01 fix). |
| `crates/nono-proxy/src/server.rs` (D-06 guard) | Cross-path fail-closed guard on CONNECT/forward-HTTP | VERIFIED | OR-of-three-predicates confirmed at `:1414-1418` (CR-03 fix). |
| `crates/nono-cli/data/nono-profile.schema.json` | `CaptureConfig` `$defs`, `additionalProperties: false` | VERIFIED (enforcement is via serde, not schema, in production) | Schema exists; but production validation path is `#[serde(deny_unknown_fields)]` on `CaptureConfig`/`CaptureResponseField` (added in `1195e1f9`), not `validate_against_schema()` — confirmed by UAT gap diagnosis, itself resolved. |
| `proj/ADR-114-oauth-capture-disposition.md` | Formal disposition record | VERIFIED | Present, tracked (`c8912280`), matches source-code reality on the D-06 table (post CR-03 correction) and Consequences section. |
| `108-DIVERGENCE-LEDGER.md` SEC-02 note | Carry-forward note naming `149abde0` | VERIFIED (spot-checked via grep, per 114-11 Self-Check reproduced here) | |
| `.planning/REQUIREMENTS.md:131` | SEC-02 checkbox + accurate scope description | VERIFIED | `- [x]` present; wording now says "on relay site 1 (the static-credential path) only" — matches source, matches the ACC-01 correction dated 2026-08-08. |
| `../nono-py`, `../nono-ts` bindings | Struct-drift closed for `RouteConfig.capture` | VERIFIED (not independently rebuilt in this pass — relying on 114-10-SUMMARY's reported `maturin build`/`napi build` results, which are consistent with the established D-14 pattern from Phases 109/113 and are not disputed by REVIEW.md or the milestone audit's WIRED section) | |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `handle_reverse_proxy` (site 1) | `relay_response_with_capture` | `if let Some(capture) = &route.capture { return ...; }` | WIRED | Confirmed unconditional return before the unbuffered loop. |
| `handle_spiffe_route` (site 2) | `relay_capture_if_declared` | shared-guard call before site's own loop | WIRED | Confirmed at `:791` region per ADR-114 D-06 table, cross-checked against `reverse.rs`. |
| `handle_spiffe_assertion_credential` (site 3) | `relay_capture_if_declared` | same shared-guard shape | WIRED | Confirmed at `:1078` region. |
| CONNECT dispatch (`server.rs`) | D-06 deny gate | `is_route_upstream() \|\| is_spiffe_route \|\| is_capture_route` | WIRED (fixed) | CR-03 fix confirmed present; previously (pre-`02a469ce`) `capture_declared_for_upstream` was dead code inside the exact-match gate. |
| `handle_reverse_proxy` request body | `resolve_capture_request_body` | direct call at `:428` | WIRED for site 1 only | Sites 2/3 do not call this function — confirmed by exhaustive grep of all call sites (`reverse.rs` lines 428, 2231 [definition], and only test-module references elsewhere). |
| `profile/mod.rs::merge_profiles` | `custom_credentials` platform-override merge | `HashMap::extend` (whole-value replace) | WIRED, but NOT capture-aware | This is the NEW-02 mechanism — confirmed at source, `:3683-3687`, with a matching test proving child-wins-on-collision semantics. Not remediated by Phase 114 (it is Phase 110 code); tracked at the milestone level. |

### Anti-Patterns Found

No new debt markers (`TBD`/`FIXME`/`XXX`) or placeholder/stub language found in the phase-114-owned
files (`capture.rs`, `reverse.rs`, `server.rs`, `route.rs`, `config.rs`, `credential_provider.rs`) —
scanned directly, zero hits. The two `not yet implemented`/`placeholder` hits in `reverse.rs`/`server.rs`
are pre-existing and unrelated (AWS SigV4 501 response; a malformed-audit-line placeholder), not part
of this phase's surface.

The known residuals below are **not** re-reported as new anti-patterns — they are already recorded
in `114-REVIEW.md` (WR-01 through WR-09, IN-01 through IN-06) and `v3.6-MILESTONE-AUDIT.md` (NEW-02,
NEW-03, NEW-04), left OPEN by explicit operator/orchestrator scoping decision (`ROADMAP.md:39`: "9
Warning + 6 Info open by operator scoping"). Confirmed still present at HEAD by direct source read in
this pass: WR-02 (`capture.rs:158-160`, exact-name/case-sensitive backstop), WR-05 (no load-time
validation in `route.rs`/`network_policy.rs` for two routes sharing a host with mismatched `capture`
declarations), and NEW-02 (`profile/mod.rs:3683-3687`). WR-03 (trailing-dot host bypass) and NEW-04
(`direct_connect_ports` voiding the guarantee for granted ports) were not independently re-derived at
source in this pass (time-boxed); both are documented with file:line citations in their source
reports and there is no evidence either has been fixed (no matching commit exists in the CR-01/02/03/
UAT fix set).

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| SEC-02 | 114-01 through 114-11 (all 11) | Declarative sandboxed OAuth capture absorbed/declined with token-confinement guarantee | SATISFIED for its declared scope, WITH DOCUMENTED RESIDUAL RISK requiring an explicit operator call | See truths table and "core question" analysis above. No orphaned requirements — every plan in the phase declares `requirements: [SEC-02]` and no other requirement ID maps to Phase 114 in `REQUIREMENTS.md`. |

### Human Verification Required

### 1. Accept or escalate the residual token-exposure paths

**Test:** Review the five named mechanisms (WR-02, WR-03, WR-05, NEW-02, NEW-04) against the
ROADMAP goal's literal text: "the guarantee that real OAuth tokens never reach the sandboxed client
preserved as a hard precondition — no reduced-scope half-feature ships."

**Expected:** An explicit operator decision, recorded either as (a) an accepted-risk note (e.g., a
VERIFICATION.md override, or an update to `114-REVIEW.md`/`v3.6-MILESTONE-AUDIT.md` marking these
items as consciously accepted for this milestone with reasoning), or (b) a follow-up hardening plan
scoped specifically to close NEW-02 (the strongest of the five, since it requires no CONNECT/exotic
routing at all — an ordinary `platform_overrides` profile pattern triggers it) before SEC-02 is
treated as unconditionally closed.

**Why human:** This is a risk-acceptance judgment about how literally to read "hard precondition,"
not a fact resolvable by reading more code. The reviewer (114-REVIEW.md) and the milestone auditor
(v3.6-MILESTONE-AUDIT.md) both already classified these as Warning-severity and left them open by
scoping decision — but neither of those documents was checking this specific question (whether the
*sum* of open Warnings is consistent with the phase goal's own absolute framing). This verification
surfaces that specific question rather than silently resolving it either direction.

## Gaps Summary

No must-have artifact is missing, stubbed, or unwired, and no roadmap Success Criterion has
factually failed — the enforcement point is real, all three relay sites are wired, and the three
Critical defects the code review found (log/audit leak, keep-alive stall, CONNECT bypass) are
confirmed fixed at the current HEAD. The reason this report does not resolve to a plain `passed` is
that the phase goal's own text sets an unusually absolute bar ("hard precondition," "no reduced-scope
half-feature"), and five distinct, already-documented, still-live mechanisms exist by which that
absolute bar is not met in practice — most notably NEW-02, which requires no adversarial or unusual
configuration, just an ordinary `platform_overrides` profile block. That gap between the goal's
literal wording and the shipped system's actual guarantee is exactly the kind of judgment call this
verifier is not positioned to resolve unilaterally, so it is escalated rather than defaulted to
`passed`.

---

_Verified: 2026-08-08_
_Verifier: Claude (gsd-verifier)_
