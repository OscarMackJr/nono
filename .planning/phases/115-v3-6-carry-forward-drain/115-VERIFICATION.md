---
phase: 115-v3-6-carry-forward-drain
verified: 2026-08-09T07:56:07Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "SC5 (second half) — a `deny_domain`-blocked SPIFFE route is denied before any JWT-SVID is minted (finding V-01, BLOCKER)"
  gaps_remaining: []
  regressions: []
  closure_commits:
    - "d63c4c71 fix(115): deny before mint on BOTH SPIFFE dispatch paths (DRAIN-06 class closure)"
    - "3da34e2c docs(115): record DRAIN-06 gap closure"
gaps: []
deferred: []
---

# Phase 115: v3.6 Carry-Forward Drain — Verification Report

**Phase Goal:** The six findings the v3.6 audit carried forward are closed at the class level, not the symptom level — so the denial/audit spine the receipt work builds on is clean before it is built on, and neither hand-maintained-list drift (DRAIN-02/03) nor removal-by-silence (DRAIN-01) can silently recur.
**Verified:** 2026-08-09T07:56:07Z (re-verification) — supersedes the 2026-08-08T23:23:26Z initial pass
**Status:** passed
**Re-verification:** Yes — after gap closure. Previous: `gaps_found`, 4/5.
**HEAD at verification:** `3da34e2c`; `../nono-py` @ `d5ed3ab` (unchanged since the initial pass — working tree clean)

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `platform_overrides.<os>` credential redefinition inherits `inject_mode`/`inject_header`; NEW-02 test asserts `spiffe` (ACC-04) | VERIFIED (regression-checked) | `profile/mod.rs:982` — `pub inject_mode: Option<InjectMode>`; `:3657-3658` — `child.inject_mode.or(base.inject_mode)` / `child.inject_header.or(base.inject_header)`. Full evidence in the initial pass; re-checked at HEAD, unchanged. |
| 2 | Every denial category nono-py's encoder can emit decodes without raising, enforced by a test driven from the encoder's own output | VERIFIED (regression-checked) | Both directions are the same serde derive on the same type: `../nono-py/src/proxy.rs:109` `serde_json::to_value`, `../nono-py/src/undo.rs:33` `serde_json::from_value`. Ruling below retained. `../nono-py` untouched by the gap closure. |
| 3 | No production `log_denied` site relies on a defaulted denial category; connect.rs `deny_domain` HTTPS point carries a real category; no variant has zero production constructors | VERIFIED (regression-checked) | `audit.rs:198-202` — `category: NetworkAuditDenialCategory` is the required 3rd positional parameter; omitting it is E0061. `InterceptHandshakeFailed` re-swept at HEAD: 3 hits in `crates/`, **all three are prose in doc comments documenting the removal** (`undo/types.rs:243,645,655`) — zero variant declarations, zero constructors. Not a regression. |
| 4 | A profile declaring both `aws_auth` and `capture` is rejected at validation, not 501'd at request time | VERIFIED (regression-checked) | `profile/mod.rs:1174` — `if cred.aws_auth.is_some()` is the first statement of `validate_custom_credential`, unconditional. |
| 5 | Python embedder can set `capture` and `spiffe` on a `RouteConfig`, AND a `deny_domain`-blocked SPIFFE route is denied before any JWT-SVID is minted | **VERIFIED** (was FAILED) | **First half** unchanged and intact — `../nono-py/src/proxy.rs:739-740` threads `spiffe`/`capture` to `RustRouteConfig`; live Python execution in the initial pass returned `True` for both. **Second half now holds for BOTH SPIFFE dispatch paths** — see the DRAIN-06 closure verification below. |

**Score:** 5/5 truths verified

## Verification of the DRAIN-06 gap closure (the sole prior gap)

The executor's claims in `115-GAP-CLOSURE.md` were treated as self-report and re-derived independently. All three requested lines of attack were run.

### (a) Ordering at source — CONFIRMED

Read directly at `crates/nono-proxy/src/reverse.rs`, `handle_spiffe_assertion_credential` (fn at `:906`):

| Step | Line | Note |
|---|---|---|
| `parse_request_line` | 916 | |
| session-token auth gate → 407 + `AuthenticationFailed` | 920-940 | closes at 940 |
| `upstream_url` → `parse_upstream_url` | 944-954 | |
| **`ctx.filter.check_host(&upstream_host, upstream_port)`** | **966** | |
| **`NetworkAuditDenialCategory::HostDenied` + 403 + `return Ok(())`** | **971-983** | |
| `spiffe_assertion.cache.get_or_refresh()` (the mint) | **990** | |

Mint now strictly follows check and deny. The sibling path `handle_spiffe_route` retains the same shape (`check_host` @655, `HostDenied` @663, `managed_auth.acquire()` @675).

**Predicate width checked per the Phase 114 CR-03 lesson** (verifying a guard's placement is not verifying its predicate width): the new `check_host` call at `:966` sits at 4-space top-level statement indentation with the preceding `if ctx.require_auth {` block closed at `:940` — it is **unconditional**, not nested inside any host/port narrowing gate. Arguments are `&upstream_host, upstream_port`, byte-identical to the other two `check_host` sites in the file (`:356`, `:655`).

### (b) The self-enumerating test — interrogated, not accepted

`crates/nono-proxy/tests/spiffe_integration.rs::d17_spiffe_dispatch_host_check_precedes_mint_structurally`. Because this is the load-bearing artifact for the *class*, I re-implemented its algorithm faithfully in a scratch harness and mutation-tested it against copies of `reverse.rs` — no edits to the real source tree. The harness reproduces the real test exactly on the unmutated file (discovers exactly `["handle_spiffe_route", "handle_spiffe_assertion_credential"]`, PASS), and my computed byte offsets match the panic messages the executor reported verbatim (2628/1657 and 3255/2358), independently corroborating that the reported reverts were genuinely run.

| # | Mutation | Expected | Result |
|---|---|---|---|
| M0 | unmutated | pass | PASS |
| M1 | revert the **assertion-path** hoist (mint before check) | fail | FAIL — names `handle_spiffe_assertion_credential` and its own `.get_or_refresh(` |
| M2 | revert the **direct-path** hoist | fail | FAIL — names `handle_spiffe_route` and its own `managed_auth.acquire(` |
| M3 | rename the assertion path's mint primitive (`get_or_refresh` → `mint_now`) | fail (coverage loss) | FAIL — `KNOWN_MINTING_DISPATCH_FNS` subset assertion fires |
| M4 | delete the assertion path's check+deny block entirely | fail (panic, not skip) | FAIL — "mints but has no `ctx.filter.check_host(`" |
| M5 | break discovery (all lines indented → no column-0 `async fn`) | fail | FAIL — the explicit anti-vacuity assertion fires |
| M6 | inject an early column-0 `}` truncating the assertion fn body | fail (coverage loss) | FAIL — subset assertion fires |
| M7 | swap the assertion path's `HostDenied` for another category | fail | FAIL — "has a host-check, but no `HostDenied` deny-emission branch" |
| M8 | add a **future third** minting dispatch fn, mint before check | fail | FAIL — names `handle_spiffe_third_path`; **no test edit was needed** |
| M9 | add a future minting dispatch fn with **no** check at all | fail (panic, not skip) | FAIL — panics, does not skip |
| M13 | move the mint into a new helper fn called by the assertion path, mint-first | (documented residual) | **FAIL — caught anyway**, on two counts |

**The specific worst-case failure mode named in the task — "silently discovers zero minting functions and passes" — is closed on three independent levels**, verified by M3/M5/M6: (1) an explicit `assert!(!fn_starts.is_empty(), ...)` on the discovery scan itself; (2) `KNOWN_MINTING_DISPATCH_FNS` asserted a subset of discovered minting fns; (3) a length assertion. Any of the three fires before a vacuous pass is reachable. The subset assertion **is** doing what is claimed — M3 and M6 are precisely the two silent-coverage-loss shapes and both are converted into loud failures.

**Load-bearing for BOTH paths independently: CONFIRMED** (M1, M2) — each revert fails naming the reverted function and that function's own mint primitive, which is the property the old single-function test could not have had.

**Two claims in the closure record are OVER-stated in the *pessimistic* direction** (noted for accuracy, not as defects): the "call-syntax matching, not a call graph" residual says a mint behind a new helper "would not be seen". M13 shows it *is* seen — the helper is itself enumerated as a top-level `async fn` and panics for having no check, and the known-fn subset assertion fires as well. The residual only bites if the helper lives in a **different file**.

**One real, narrow bypass found — recorded as V-04 (warning, not a gap):** `strip_line_comments` removes only **whole-line** `//` comments. Trailing comments, `/* */` block comments, and string literals are **not** stripped. I confirmed by mutation (M10b/M11b/M12b) that placing the literal text `ctx.filter.check_host(` and `NetworkAuditDenialCategory::HostDenied` in any of those three positions *before* the mint makes an inverted function pass. **This does not affect the present-tense verdict:** I swept all 6 occurrences of the two guard tokens in `reverse.rs`'s production section and **every one is real code** (`:356`, `:655`, `:966` and `:364`, `:663`, `:974`) — zero occurrences in any comment or string. The concern is forward-looking: the function's own doc comment invites authors to "name the real call syntax freely" in ordering comments, which is safe only for the whole-line form actually used in this file.

### (c) No deny surface narrowed — confirmed independently of the line-count argument

Rather than trusting the `comm`/28-vs-28 argument, I read the commit diff (`git show d63c4c71 -- crates/nono-proxy/src/reverse.rs`) directly:

- The deny block (`check_host` call, `if !check.result.is_allowed()`, `reason`, `warn!`, `send_error(403)`, `log_denied(... HostDenied ...)`, `return Ok(())`) has **zero `-` and zero `+` lines** in the diff. It was not touched.
- The only code hunks are the `-`/`+` pair for the `get_or_refresh` match block, whose removed and added text is **identical** (the 503 `ManagedCredentialUnavailable` fail-closed arm included). Every other hunk is comment-only.
- The predicate, its two arguments, the `HostDenied` category, the 403 status and the audit `EventContext` are unchanged.
- The **widening** claim is corroborated by the diff shape: the exchange's `return Ok(())` 503 arm previously preceded the check, so a `deny_domain`-blocked upstream whose exchange also failed produced no `HostDenied` record at all. It now does. Strictly more requests reach the check; zero previously-denied requests are now allowed.

**Comment corrections:** both formerly-false comments were re-read at HEAD and are now true. `handle_spiffe_assertion_credential`'s doc comment (`:884-901`) replaces "Mirrors `handle_spiffe_route`'s structure exactly" with a shared-ORDER claim plus four enumerated deliberate differences; `handle_spiffe_route`'s D-17 comment (`:644-654`) replaces the false "every other `HostDenied` site … already uses" with a statement that is true now and explicitly records that it was not true when first written. Both were made true rather than softened, as the project record requires.

### Ruling on SC2 in light of WR-01 (retained from the initial pass)

**WR-01's factual claim is CORRECT** and is carried forward unchanged as a residual, not a gap. `assert_all_variants_covered` is an exhaustive `match` (E0004 on a new variant), but nothing binds `ALL`'s contents or length to the enum's variant count, and the companion test compares `ALL.len()` against a hardcoded `10`. Adding a variant + its guard arm while leaving `ALL` untouched compiles and passes.

**SC2 remains VERIFIED, not partial,** on mechanism: post-fix, encode and decode are the **same serde derive on the same type**, so for any unit variant the round trip is symmetric by construction. There is no longer a second vocabulary that *can* drift; the test is a proof artifact rather than the enforcement. The one scenario `ALL` drift would hide is a **non-unit** variant, where `to_value` yields a non-string and the encoder returns `Err` — narrow, and not silent to a Python caller. The `ALL` doc comment's "identical guarantee" sentence is factually wrong today and should not be relied on by Phase 117/118.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/nono-cli/src/profile/mod.rs` | `Option<T>` fields, `.or(base)` merge, HookConfig guard, exhaustive NEW-02 test, aws/oauth2 rejection | VERIFIED | HookConfig guard is an exhaustive destructure with no `..` — E0027 on a new field is a language certainty. |
| `crates/nono-cli/src/network_policy.rs` | production consumer resolves via `unwrap_or_default`/`unwrap_or_else(default_inject_header)` | VERIFIED | `:228-238`, reuses the existing private helper. |
| `crates/nono-cli/data/nono-profile.schema.json` | `oneOf`+null shape, no `"default"` key | VERIFIED | |
| `crates/nono-proxy/src/audit.rs` | required `category` param; `EventContext` without `denial_category` | VERIFIED | `:198-202`. |
| `crates/nono/src/undo/types.rs` | enum without `InterceptHandshakeFailed`, `ALL` const + guard | VERIFIED (guard weaker than documented — WR-01) | Re-swept: 3 remaining textual hits are all doc-comment prose about the removal. |
| `crates/nono-proxy/src/reverse.rs` | host-check-before-mint ordering in SPIFFE dispatch | **VERIFIED** (was PARTIAL) | Correct in **both** `handle_spiffe_route` (`:655`/`:663` before `:675`) and `handle_spiffe_assertion_credential` (`:966`/`:974` before `:990`). Pure move; deny block untouched. |
| `crates/nono-proxy/tests/spiffe_integration.rs` | structural no-mint-on-deny regression | **VERIFIED** (was PARTIAL) | Self-enumerating by behaviour; survived 11 adversarial mutations incl. all three vacuity shapes. |
| `crates/nono-proxy/src/{config,route}.rs` (CR-01/WR-03 fix) | boundary validation invoked from `RouteStore::load` | VERIFIED | `route.validate()?` first in the per-route loop (`route.rs:199`). |
| `../nono-py/src/proxy.rs` | serde encoder, 8 wrapper types, wired `RouteConfig::new`, D-16 allowlist test | VERIFIED | Untouched by the closure; `:109` encoder, `:739-740` wiring intact. |
| `../nono-py/src/undo.rs` | serde decoder + first `#[cfg(test)] mod tests` | VERIFIED | `:33`. |
| `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` | un-reject carry-forward note | VERIFIED | |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `merge_custom_credential_def` | `CustomCredentialDef` | `.or(base)` on all 14 optional arms | WIRED | `:3657-3658` |
| `network_policy.rs` | `nono_proxy::config::RouteConfig` | `unwrap_or_default` / `unwrap_or_else(default_inject_header)` | WIRED | `:232-238` |
| `connect.rs` / `external.rs` | `audit::log_denied` | positional `NetworkAuditDenialCategory` | WIRED | `connect.rs:84` HostDenied (unconditional predicate); `external.rs:136`/`:201` |
| `../nono-py` test module | `NetworkAuditDenialCategory::ALL` | round-trip over every core variant | WIRED | |
| `RouteStore::load` | `RouteConfig::validate` | `route.validate()?` first in loop | WIRED | |
| `RouteConfig::new` (PyO3) | `RustRouteConfig` | `.map(\|x\| x.inner)` | WIRED | Confirmed from live Python in the initial pass |
| `handle_spiffe_route` | `managed_auth.acquire()` | host-check gate runs first | WIRED | `:655` / `:663` before `:675` |
| **`handle_spiffe_assertion_credential`** | **`cache.get_or_refresh()`** | **host-check gate runs first** | **WIRED** (was NOT_WIRED) | `:966` / `:974` before `:990`; unconditional predicate |
| `d17_..._structurally` test | both SPIFFE dispatch fns | behaviour-based discovery over `reverse.rs` production section | WIRED | Load-bearing proven independently for each path (M1, M2) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `RouteConfig` (PyO3) | `inner.spiffe` / `inner.capture` | Python-constructed wrapper → `.inner` | Yes — live Python returned `True` for both | FLOWING |
| `denial_category` dict key | `event.denial_category` | serde `to_value` on core enum | Yes — all 10 variants round-trip | FLOWING |
| `RouteConfig::validate` | `self.inject_header` / `self.spiffe` / `self.capture` | `RouteStore::load` per-route loop | Yes — rejects hostile literals at load | FLOWING |
| D-17 test subjects | `minting_fns` | discovery over `include_str!("../src/reverse.rs")` | Yes — discovers 2 real fns; **empty-discovery is asserted against on 3 levels** | FLOWING |
| `NetworkAuditDenialCategory::ALL` | 10 hand-written entries | hand-written const | Yes today; not structurally bound to the enum | STATIC (WR-01) |

### Behavioral Spot-Checks (run by this verifier)

| Behavior | Command | Result | Status |
|---|---|---|---|
| D-17 structural ordering, both paths | `cargo test -p nono-sandbox-proxy --test spiffe_integration` | 6 passed, 0 failed | PASS |
| Full proxy suite (no regressions from the hoist) | `cargo test -p nono-sandbox-proxy` | 307 lib + 6 integration passed, 0 failed | PASS |
| D-17 test adversarial mutation battery | 11 mutations via an independent re-implementation | 11/11 detected; 0 false passes | PASS |
| Guard-token occurrence audit (comment/string bypass) | scan of all 6 occurrences in `reverse.rs` production | all 6 are real code, 0 in comments or strings | PASS |
| Mint primitives outside `reverse.rs` | workspace grep of `MINT_MARKERS` in `crates/nono-proxy/src/` | only the primitive **definitions** (`auth.rs:47` inside `ManagedUpstreamAuth::acquire`, `oauth2.rs`) and one doc comment — **no dispatch path outside `reverse.rs`** | PASS |
| Revert residue / working tree | `git status --porcelain`; `grep -c "TEMPORARY REVERT"` | clean (only this untracked report); 0 residue in both files | PASS |
| Debt markers in changed files | `grep -n "TBD\|FIXME\|XXX"` on both changed files | 0 hits | PASS |

Regression-sanity for the four previously-passed truths was re-run at HEAD (SC1 `:982`/`:3657-3658`; SC2 `../nono-py` `proxy.rs:109`/`undo.rs:33`; SC3 `audit.rs:198-202` + `InterceptHandshakeFailed` sweep; SC4 `profile/mod.rs:1174`). No regressions.

### Probe Execution

No probes declared in any PLAN/SUMMARY/gap-closure record and no `scripts/*/tests/probe-*.sh` exist in this repo. Step 7c: SKIPPED (no probes in scope).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| DRAIN-01 | 115-01 | `platform_overrides` credential inherits `inject_mode`/`inject_header`; ACC-04 `spiffe` assertion added | SATISFIED | SC1 |
| DRAIN-02 | 115-02, 115-05 | nono-py round-trips every category its encoder emits, pinned by an exhaustiveness test over the encoder's own output | SATISFIED | SC2 + ruling |
| DRAIN-03 | 115-02 | The three `EventContext::default()` denial sites emit a real category; zero-constructor variants removed or wired | SATISFIED | SC3 |
| DRAIN-04 | 115-03 | `aws_auth`+`capture` route rejected at config-validation, not 501'd | SATISFIED | SC4 |
| DRAIN-05 | 115-06 | Python embedder can configure `capture` and `spiffe` on `RouteConfig` | SATISFIED | Live Python run |
| DRAIN-06 | 115-04 + gap closure `d63c4c71` | A SPIFFE route blocked by `deny_domain` is denied **before** a JWT-SVID is minted | **SATISFIED** (was BLOCKED) | Both dispatch paths hoisted; ordering pinned by a discovery-based class test that survived 11 mutations |

No ORPHANED requirements: REQUIREMENTS.md maps exactly DRAIN-01…DRAIN-06 to Phase 115, and every ID appears in a PLAN `requirements:` field.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| — | — | `TBD` / `FIXME` / `XXX` | none | Zero hits across all changed source/schema files, including the two touched by the gap closure. Debt-marker gate: PASS. |
| `crates/nono-cli/data/nono-profile.schema.json`, `profile/mod.rs` | various | "not yet implemented" | Info | Documents a *deliberately rejected* mechanism backed by a real fail-closed rejection — not a stub. |
| `crates/nono-cli/src/profile/mod.rs` | 1238-1240, 1373-1409, 1420-1458 | unreachable validation behind a tautological guard | Warning (operator-deferred, WR-02) | Confirmed still present. |
| `crates/nono-proxy/src/reverse.rs` | 325, 391, 711, 995; `external.rs:125` | denial branches with no audit record | Warning (operator-deferred, WR-06) | Confirmed still present at HEAD (line numbers shifted by the hoist). |

### Findings deliberately left OPEN by operator decision (scoped out — NOT gaps, recorded for visibility)

| ID | Summary | Verifier confirmation at HEAD |
|---|---|---|
| WR-01 | `ALL`'s drift guard is weaker than its doc comment claims; D-11's "identical guarantee" rationale is false | CONFIRMED FACTUALLY CORRECT. Ruled non-blocking for SC2 — see ruling. |
| WR-02 | ~55 lines of now-unreachable validation behind a tautological guard | Confirmed present |
| WR-04 | `_nono_py.pyi` declares two nonexistent `RouteConfig` params | Confirmed |
| WR-05 | `PartialEq` semantics shift affects `nono profile diff` | Confirmed |
| WR-06 | 5 deny branches emit no audit record | Confirmed present |
| IN-01 … IN-08 | Info findings | Not re-litigated |

### Findings raised by verification

| ID | Severity | Status | Finding |
|---|---|---|---|
| V-01 | ~~BLOCKER~~ | **CLOSED** by `d63c4c71` | `handle_spiffe_assertion_credential` minted a JWT-SVID and presented it to the IdP token endpoint before its `deny_domain` host check. Re-verified closed at source, load-bearing proven independently for both paths, no deny surface narrowed. |
| V-02 | Warning | OPEN | The D-16 allowlist test in `../nono-py/src/proxy.rs` over-states its future-proofing: its per-field `assert!` list is hand-written, so a *new* field hardcoded `None` and absent from the list passes silently. The **present-tense** must-have is TRUE and verified; only the future-proofing claim is over-stated. |
| V-03 | Warning | OPEN | `115-VALIDATION.md`'s cross-target-clippy exemption premise ("no in-scope file contains `#[cfg(target_os = …)]`") is false for the final file set, but its **conclusion holds** — no changed line sits inside a Unix-cfg branch, and the two gap-closure files contain no `#[cfg(target_os = …)]` at all. Re-word rather than re-litigate. |
| V-04 | Warning | **NEW** | The D-17 test's `strip_line_comments` removes only **whole-line** `//` comments. Trailing comments, `/* */` block comments, and string literals are not stripped, so the literal text `ctx.filter.check_host(` or `NetworkAuditDenialCategory::HostDenied` appearing in any of those positions before a mint would make an inverted function pass (verified by mutation M10b/M11b/M12b). **No such occurrence exists today** — all 6 guard-token occurrences in `reverse.rs` production are real code — so the present-tense property is unaffected. The risk is that the function's own doc comment invites authors to "name the real call syntax freely" in ordering comments, which is safe only for the whole-line form. Cheapest hardening: also drop `/* … */` spans and everything after an unquoted `//`, or match on a token-position basis. Same over-claim shape as WR-01 and V-02. |

### Human Verification Required

None. All plans were `autonomous: true`; no PLAN contains a `<verify><human-check>` block; no checkpoint tasks exist; the gap closure introduced none. Every criterion was resolvable programmatically — including a live Python execution of the SC5 embedder path (initial pass) and an 11-mutation adversarial battery against the DRAIN-06 class test (this pass).

### Gaps Summary

None. All six carried-forward findings are closed, and closed structurally rather than by patching symptoms — which is what this phase's governing rule demanded:

- **DRAIN-01** moved the defect into the type system (`Option<T>` + no `#[derive(Default)]`, so a missed site is E0063).
- **DRAIN-02** removed the second vocabulary entirely rather than adding the two missing arms — encoder and decoder are now the same serde derive, which is why WR-01's real `ALL` drift does not reopen the class.
- **DRAIN-03** made the denial category a required parameter and deleted the field that permitted a silent default — stronger than the SC's literal wording.
- **DRAIN-04** rejects at the first statement of validation, unconditionally.
- **DRAIN-05** is wired end-to-end and confirmed from a real Python interpreter.
- **DRAIN-06**, the prior gap, is now closed at the class level and not the symptom level: the hoist was applied to the *second* SPIFFE dispatch path as a pure move with the deny block untouched and the predicate unchanged, the deny surface strictly widened rather than narrowed, both formerly-false comments were made true rather than softened, and — critically — the regression net was rebuilt to **discover its own subjects by behaviour**. That last point is what converts a symptom fix into a class fix: a future third dispatch path is covered with no edit to the test (proven by M8), a minting path with no host-check panics rather than being skipped (M4, M9), and the three ways the test could have silently degraded to a vacuous pass are each asserted against (M3, M5, M6).
- The code review's CR-01/WR-03 remediation is real, first-in-loop, and load-bearing.

Four warnings remain open and visible (V-02, V-03, V-04, plus the operator-deferred WR-01/WR-02/WR-04/WR-05/WR-06 and IN-01…IN-08). **Three of them — WR-01, V-02, V-04 — are the same recurring shape: a guard whose in-source documentation claims a stronger future-proofing property than the guard's mechanism actually delivers.** Each is factually harmless today and each is a documentation-versus-mechanism mismatch rather than a defect. Phase 117/118 should not build on any of those three doc claims without re-reading the mechanism.

---

_Verified: 2026-08-09T07:56:07Z (re-verification; supersedes 2026-08-08T23:23:26Z)_
_Verifier: Claude (gsd-verifier)_
