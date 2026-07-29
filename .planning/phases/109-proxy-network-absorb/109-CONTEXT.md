# Phase 109: Proxy/Network Absorb - Context

**Gathered:** 2026-07-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Absorb the v0.67–v0.69 proxy/network features that map to **NET-01** and **NET-03** into the fork's
proxy, without regressing its fork-divergent TLS-interception + default-deny allowlist model, and
rebuild both language bindings.

**NET-02 (SPIFFE/SPIRE #1272) is NO LONGER IN THIS PHASE** — moved to the new **Phase 113** on
2026-07-29 (see D-01). Do not plan it here.

**Scope: 5 commits.**

| Commit | Req | Ins | Del | Files | Disposition |
|---|---|---|---|---|---|
| `3b207eeb` `deny_domain` (#1374) | NET-01 | 311 | 26 | 15 | **ADAPT** — settled by `proj/ADR-108-deny-domain-posture.md` |
| `1619275c` profile-declared `no_proxy` (#1415) | NET-03 | 2065 | 99 | 17 | adopt w/ validation intact (D-05) |
| `726ac1f1` `HTTP_PROXY` forward-proxy (#1335) | NET-03 | 815 | 5 | 1 | adopt |
| `23d93fc9` sibling-route cross-deny (#1437) | NET-03 | 87 | 0 | 1 | adopt |
| `6fb7ecbf` SigV4 encoded-URI (#1430) | NET-03 | 16 | 4 | 1 | adopt |

~3.3k insertions total. Work-list authority: `108-DIVERGENCE-LEDGER.md` §"NET Cluster — Per-Commit
Table". The **6 other NET-cluster commits** (`c344efb0`, `4192bfa5`, `261bbd68`, `3672ea10`,
`8255a27a`, `7d23bba6`) map to no v3.6 requirement and belong to **Phase 112** — not this phase.

</domain>

<decisions>
## Implementation Decisions

### Scope & Sequencing

- **D-01: NET-02 (SPIFFE/SPIRE, `c831dade`) is SPLIT OUT into a new Phase 113.** Measured at **4354
  insertions / 545 deletions / 33 files** — 57% of the original Phase 109 by volume. It is a
  *refactor of fork-divergent code*, not an addition: `reverse.rs` +545, `tls_intercept/handle.rs`
  +286, `route.rs` +348, `credential.rs` +243, `oauth2.rs` +290. It also crosses into the core
  library, expands `Cargo.lock` by ~633 lines, and touches `supervisor_linux.rs` (mandatory
  cross-target clippy). Bundling it meant one conflict in the SPIFFE refactor would block four
  unrelated small fixes. **Roadmap amendment applied 2026-07-29** — Phase 113 exists, NET-02
  retargeted.
- **D-02: SPIFFE gets its own standalone ADR** (`proj/ADR-113-spiffe-disposition.md`), mirroring the
  ADR-98/ADR-108 precedent. It is this milestone's second #1225-class item. **Not this phase's
  work** — recorded here only so Phase 109 planning does not absorb it by accident.
- **D-03: Phase 109 lands BEFORE Phase 112; Phase 112 rebases onto it.** Phase 112's `8255a27a`
  (async `load_with_diagnostics` refactor) touches `credential.rs`, `oauth2.rs`, `server.rs`,
  `tls_intercept` — the same files this phase edits, and `server.rs` is where both large absorbs
  land. A mechanical async refactor rebases onto changed feature code more easily than feature work
  rebases onto a refactor. Phase 113 also rebases onto 109 (declared in its ROADMAP `Depends on`).

### `deny_domain` (NET-01) — mechanism, not posture

> The **posture** is already settled and MUST NOT be re-opened: **ADAPT — deny layer only, never an
> allowlist substitute; a deny-only profile must still fail closed.** See
> `proj/ADR-108-deny-domain-posture.md` (D-10/D-11/D-12 of `108-CONTEXT.md`). These decisions cover
> only *how* to implement it.

- **D-04: The fail-closed guard mirrors the fork's existing `validate_block_net_conflicts` pattern.**
  A **CLI-side** validator, called from **both** entry points — `crates/nono-cli/src/command_runtime.rs`
  (cf. line 149) and `crates/nono-cli/src/launch_runtime.rs` (cf. line 361) — with
  `crates/nono-cli/src/network_policy.rs::build_proxy_config()` (cf. line 291) as the construction
  point. Rationale: reuses the fork's own proven fail-closed idiom, keeps the library policy-free per
  ADR-86/ADR-108, and covers both paths. **A guard on only one entry point is a bypass, not a guard.**
- **D-05 (posture consequence): a deny-only profile — `deny_domain` with no `allow_domain` — is a
  HARD ERROR at parse time**, with a message that names the fork's deliberate divergence from
  upstream. Rationale: such a profile almost certainly comes from a user expecting upstream's
  allow-all-except semantics; silently applying default-deny would present as a broken proxy with no
  explanation. An explicit, guiding error is both more honest and more debuggable than silent
  default-deny or a warning that scrolls away.

### `no_proxy` (#1415) — a bypass surface, not a convenience feature

- **D-06: `no_proxy` punches holes in the proxy. Its validation logic is NON-NEGOTIABLE and absorbs
  as an inseparable unit with the feature.** The bulk of the 2065 lines is guard rails:
  `validate_no_proxy_entry` (rejects URL credentials + path entries), `validate_profile_no_proxy`,
  `validate_proxy_launch_no_proxy_conflicts`, `validate_expanded_proxy_no_proxy_conflicts`,
  `no_proxy_entry_overlaps_host_pattern`, `bare_single_label_suffix_overlaps_host`, plus tests
  named `rejects_allow_domain_overlap`, `rejects_inherited_no_proxy_allow_domain_overlap`,
  `rejects_group_expanded_no_proxy_overlap`.
  **Without them, a `no_proxy` entry overlapping `allow_domain` silently defeats filtering — that
  would be a genuine security regression in a default-deny tool.** No executor may simplify, defer,
  or partially absorb these validators. If the feature is trimmed, the validators are not what gets
  trimmed.
- **D-07: `#1415` REPLACES the fork's existing `no_proxy_hosts` logic — adopt the replacement, then
  prove the fork's current behavior survived.** Upstream swaps
  `let mut no_proxy_parts = vec!["localhost", "127.0.0.1"]` for a `push_no_proxy_entry` pipeline and
  adds a `canonical_no_proxy_hosts` field (`crates/nono-proxy/src/server.rs`, cf. line 53). It also
  start-up-filters parent-shell `NO_PROXY`/`no_proxy`. **Required verification: `localhost` and
  `127.0.0.1` still reach the child's `no_proxy` env var as they do today.** Name this as a test, not
  an assumption — this is a refactor of live fork behavior, not an additive change.

### ADR-86 boundary (informational — feeds Phase 113, not this phase)

- **D-08: SPIFFE's core-library touches were inspected and ADR-86 appears INTACT.**
  `crates/nono/src/undo/types.rs` (+45) adds only auth-method enum variants (`SpiffeJwtBearer`,
  `SpiffeOAuthAssertion`, `SpiffeJwt`) and serde structs `SpiffeAuditContext` /
  `SpiffeDelegationContext` recording *what happened* (trust domain, SVID type, delegation chain).
  `crates/nono/src/audit.rs` (+2) is two `spiffe_context: None,` initializers. **No enforcement, no
  policy evaluation** — consistent with CLAUDE.md's boundary table, which already places the audit
  module in the core library as "observability primitives, not security policy."
  **Caveat for ADR-113:** it does introduce SPIFFE *vocabulary* into the policy-free library — a mild
  concept-leak, though audit records inherently carry domain terms. ADR-113 should confirm this
  reading against the full diff rather than inherit it.

### Standing Rules (carried, not re-litigated)

- **D-09: Rebuild BOTH bindings after any `nono-proxy` struct change** — `maturin build` in
  `../nono-py` and `napi build --platform --release` in `../nono-ts`. Binding struct-drift is caught
  **only** by actually building; static inspection misses it (durable lesson from v3.4, where
  `nono-py` was silently missing both `endpoint_policy` and `enable_h2`). This is SC4.
- **D-10: Ledger dispositions outrank the `260727-jkn` parity map.** The map has now been empirically
  wrong three times (see D-11). Where they disagree, the ledger wins.
- **D-11: Verify feature presence by behavior, never by identifier name.** See `<specifics>`.
- **D-12: Cross-target clippy** (`cross` linux-gnu + `cargo-zigbuild` apple-darwin, both GREEN
  locally, **no PARTIAL→CI**) is mandatory if any cfg-gated Unix code is touched. `make ci`
  (clippy+fmt+tests), not clippy alone. Per the ledger, this phase's 5 commits are proxy-local, so
  the trigger may not fire — **confirm rather than assume**.

### Claude's Discretion

- Plan/wave decomposition within the 5 commits, provided `1619275c` and `726ac1f1` (the two large
  ones) are not bundled into a single plan with the three small fixes.
- Exact wording of the D-05 parse-time error, provided it names the fork's divergence from upstream.
- Whether `6fb7ecbf` (16 ins) and `23d93fc9` (87 ins) share a plan.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Settled decisions this phase implements
- `proj/ADR-108-deny-domain-posture.md` — **the `deny_domain` posture is settled here (ADAPT).** Read before planning NET-01. Do not re-open.
- `.planning/phases/108-upst12-divergence-audit/108-CONTEXT.md` — D-10/D-11/D-12 (deny_domain), D-04/D-06 corrected measurement blocks.

### Work-list authority
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` §"NET Cluster — Per-Commit Table" — per-commit dispositions, windows-touch, security-relevant, re-export scans. **Authoritative over the parity map.**
- `.planning/phases/108-upst12-divergence-audit/108-VERIFICATION.md` — confirms the ledger was independently re-derived from live git.

### Fork invariants
- `proj/ADR-86-library-boundary-convergence.md` — the policy-free library boundary.
- `CLAUDE.md` — Library-vs-CLI boundary table; cross-target clippy MUST/NEVER; path-security and permission-scope rules; DCO sign-off.
- `.planning/templates/cross-target-verify-checklist.md` — single source of truth for both cross-target gates.

### Code surfaces this phase edits
- `crates/nono-cli/src/network_policy.rs` — `build_proxy_config()` (cf. :291); the D-04 construction point.
- `crates/nono-cli/src/command_runtime.rs` (cf. :149) + `crates/nono-cli/src/launch_runtime.rs` (cf. :361) — the two entry points calling `validate_block_net_conflicts`; the pattern D-04 mirrors.
- `crates/nono/src/net_filter.rs` — `HostFilter`, `deny_hosts`, `DENY_HOSTS`, `strict`, `FilterResult::Deny`.
- `crates/nono-proxy/src/server.rs` — `no_proxy_hosts` (cf. :53), the D-07 collision site; also where `726ac1f1` lands.
- `crates/nono-proxy/src/{config.rs,route.rs,filter.rs}` — touched by `#1415`.
- `crates/nono-proxy/src/tls_intercept/handle.rs` — touched by `23d93fc9`.

### Scope source (historical, now superseded on three counts)
- `.planning/quick/260727-jkn-map-macos-0-69-parity-gap-phases-for-the/260727-jkn-PLAN.md` — **treat with caution** (D-10/D-11).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`validate_block_net_conflicts`** — the fork's own fail-closed validator, already wired into both `command_runtime.rs` and `launch_runtime.rs`. D-04's guard copies this shape rather than inventing one.
- **`HostFilter` already has a deny mechanism** — `deny_hosts`, a hardcoded cloud-metadata `DENY_HOSTS`, and a `FilterResult::Deny` arm. `#1374` extends a live surface (adds `deny_suffixes` + `with_denied_hosts`), it does not introduce one.
- **`network_policy.rs::build_proxy_config()`** — the existing resolved-policy → `ProxyConfig` construction point.

### Established Patterns
- **Fail-closed validators live CLI-side**, called from both runtimes, returning a `Result` with an operator-legible message.
- **`with_*` builders take caller-supplied data**, keeping policy out of the library — the reason ADR-108 concluded ADR-86 is not breached.
- **Four dispositions** (adopt / adapt / skip / split) with `split` load-bearing.

### Integration Points
- `deny_domain` composes with `allow_domain` inside `HostFilter::check_host()` — deny evaluated **before** the allowlist.
- `no_proxy` interacts with `allow_domain` via the D-06 overlap validators — the security-critical seam of this phase.
- Both bindings (`../nono-py`, `../nono-ts`) consume `nono-proxy` structs and must be rebuilt (D-09, SC4).
- Phase 112 and Phase 113 both rebase onto this phase's `nono-proxy` changes (D-03).

</code_context>

<specifics>
## Specific Ideas

- **The parity map has now been empirically wrong three times, always the same way — matching
  identifier names mistaken for feature presence.** (1) Phase 108's D-06 error: a `*tool-sandbox*`
  substring glob matched `docs/cli/features/tool-sandbox.mdx`. (2) `#1415` marked "already present"
  because the fork has a `no_proxy_hosts` field — which builds the child's `no_proxy` **env var**,
  an unrelated mechanism; the profile layer and schema have **zero** hits. (3) `#1335` marked
  "already present" because `HTTP_PROXY` appears in `server.rs` — but those hits **set the env var
  pointing the child at the proxy**, the opposite direction from serving forward-proxy requests; the
  fork has zero hits for `classify_request_target` / `handle_forward_http` / absolute-form handling.
  **Rule for this phase and every future sync: confirm a feature by locating the behavior — the
  function that implements it, the schema key that declares it — never by grepping its name.**
- D-07's verification (`localhost`/`127.0.0.1` still reaching the child's `no_proxy` env var) should
  be an actual test, not a code-reading. It is the one place this phase can silently regress live
  fork behavior.
- `#1335` adds a forward-proxy **serving** path that accepts absolute-form requests. It ships 8 tests
  including `forward_http_denied_host_returns_403_and_audits` and
  `forward_https_absolute_form_is_rejected_with_connect_guidance` — preserve those; they encode the
  security contract.

</specifics>

<deferred>
## Deferred Ideas

- **NET-02 SPIFFE/SPIRE → Phase 113** (roadmap amendment applied 2026-07-29). Needs its own ADR
  (D-02) covering the ADR-86 crossing, the 545-deletion rewrite of fork-divergent code, and the
  ~633-line dependency-surface expansion.
- **The 6 unmapped NET-cluster commits → Phase 112** (`c344efb0`, `4192bfa5`, `261bbd68`,
  `3672ea10`, `8255a27a`, `7d23bba6`). Not in scope here even though they are proxy-adjacent.
- **Gray areas raised but not discussed** (available if planning needs them): whether the 6
  Phase-112 NET commits should fold back into 109 now that SPIFFE has left; how SC4's binding
  rebuild is verified when `nono-proxy` struct changes land across three phases (109/112/113);
  whether `#1335`'s forward-proxy serving path warrants its own threat review; and the **v3.4 WR-03
  open product-decision** on `validate_block_net_conflicts` vs strict-filter semantics — still open,
  and D-04's guard now sits directly beside it.

### Reviewed Todos (not folded)
- `20260611-msi-vcredist-prereq.md` and `20260611-poc-cert-broker-clean-host.md` — host-gated v3.5 distribution items owned by v3.5 Phase 106; keyword-only matches, unrelated to a proxy absorb.

</deferred>

---

*Phase: 109-proxy-network-absorb*
*Context gathered: 2026-07-29*
