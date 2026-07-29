# Phase 109: Proxy/Network Absorb - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-29
**Phase:** 109-proxy-network-absorb
**Areas discussed:** phase sizing, #1415 true scope, deny_domain guard placement, Phase 109/112 ordering

All 4 offered areas were selected; every question resolved to the recommended option.

---

## Scouting findings that reframed the discussion

The `260727-jkn` parity map marked two of Phase 109's commits "already present — verify only." Both
were wrong, in the same way, and the same way Phase 108's D-06 error was wrong: **identifier-name
matches mistaken for feature presence.**

| Item | Map said | Measured reality |
|---|---|---|
| `#1415` `no_proxy` (`1619275c`) | present, verify only | Fork's `no_proxy_hosts` builds the **child's env var** — different mechanism. Profile layer: 0 hits. Schema: 0 hits. Upstream: 2065 ins / 17 files. **Absent.** |
| `#1335` `HTTP_PROXY` (`726ac1f1`) | present, verify only | Fork's `HTTP_PROXY` hits **point the child at the proxy** — opposite direction. Zero hits for `classify_request_target` / `handle_forward_http` / absolute-form. **Absent.** |

My own first scouting grep repeated the trap: `grep "no_proxy"` matched `nono_proxy`. Caught and
re-run with word boundaries before it reached any decision.

Sizing measured across all 6 original commits:

| Commit | Req | Ins | Del | Files |
|---|---|---|---|---|
| `c831dade` SPIFFE | NET-02 | 4354 | 545 | 33 |
| `1619275c` no_proxy | NET-03 | 2065 | 99 | 17 |
| `726ac1f1` HTTP_PROXY | NET-03 | 815 | 5 | 1 |
| `3b207eeb` deny_domain | NET-01 | 311 | 26 | 15 |
| `23d93fc9` sibling-route | NET-03 | 87 | 0 | 1 |
| `6fb7ecbf` SigV4 | NET-03 | 16 | 4 | 1 |

---

## Phase Sizing

### Q1 — How to split ~7.6k insertions

| Option | Description | Selected |
|--------|-------------|----------|
| Split NET-02 (SPIFFE) into its own phase | 109 keeps NET-01 + NET-03 (~3.3k, 5 commits, proxy-local); SPIFFE gets its own phase | ✓ |
| Keep all 6, split into waves | One phase, small-to-large wave ordering | |
| Defer NET-02 to v3.7 entirely | Drop SPIFFE from v3.6 | |
| Decide after an ADR on SPIFFE | Settle disposition first, size after | |

**User's choice:** Split NET-02 into its own phase → **Phase 113** created.
**Notes:** SPIFFE is 57% of the phase by volume and is a *refactor of fork-divergent code* (545 deletions across `reverse.rs`, `tls_intercept/handle.rs`, `route.rs`, `credential.rs`, `oauth2.rs`), not an addition. Bundling meant one SPIFFE conflict would block four unrelated small fixes.

### Q2 — Does SPIFFE warrant its own ADR?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — second #1225-class item | Crosses ADR-86, rewrites divergent code, expands dependency surface | ✓ |
| No — ledger already dispositioned it adopt | Handle surprises as execution deviations | |

**User's choice:** Yes — `proj/ADR-113-spiffe-disposition.md`, deferred to Phase 113.

### Q3 — SPIFFE's core-library touches and ADR-86

| Option | Description | Selected |
|--------|-------------|----------|
| Inspect the actual diff before deciding | Same discipline that caught `e6d26871` in Phase 108 | ✓ |
| Adapt — keep all SPIFFE logic CLI-side | Pre-commit to a policy-free core | |
| Adopt as-is, note the divergence | v3.1 Phase 86 precedent | |

**User's choice:** Inspect first. **Inspection performed during the discussion:** `undo/types.rs` (+45) adds only auth-method enum variants and serde audit structs (`SpiffeAuditContext`, `SpiffeDelegationContext`); `audit.rs` (+2) is two `spiffe_context: None,` initializers. No enforcement, no policy evaluation → **ADR-86 appears intact**, consistent with CLAUDE.md already placing the audit module in the core library as an observability primitive. Caveat recorded for ADR-113: SPIFFE *vocabulary* now enters the policy-free library.

### Q4 — Ordering vs Phase 112

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 109 first; 112 rebases onto it | Mechanical async refactor rebases more easily than feature work does | ✓ |
| Move `8255a27a` into Phase 109 | Avoids the rebase; grows 109 | |
| Phase 112 first | Features land on final-shape code; inverts the dependency spine | |

**User's choice:** Phase 109 first.

---

## `#1415` no_proxy — true scope

Inspection showed the bulk of the 2065 lines is **validation**, not feature: `validate_no_proxy_entry`,
`validate_profile_no_proxy`, `validate_proxy_launch_no_proxy_conflicts`,
`validate_expanded_proxy_no_proxy_conflicts`, `no_proxy_entry_overlaps_host_pattern`,
`bare_single_label_suffix_overlaps_host`, plus tests explicitly named for rejecting `allow_domain`
overlap. It also **replaces** the fork's existing `no_proxy_parts` construction and start-up-filters
parent-shell `NO_PROXY`.

### Q1 — Stance on adding a bypass path to a default-deny tool

| Option | Description | Selected |
|--------|-------------|----------|
| Adopt with validation intact, non-negotiable | Feature + validators absorb as an inseparable unit | ✓ |
| Adopt + fork-specific hard denial on overlap | Stricter than upstream; diverges for identical config | |
| Defer #1415 — don't add a bypass surface | Smallest security surface; leaves a parity gap | |

**User's choice:** Adopt with validation intact.
**Notes:** without the overlap validators, a `no_proxy` entry overlapping `allow_domain` silently defeats filtering — a genuine security regression. Recorded so no executor trims the validators as "extra."

### Q2 — Collision with the fork's existing `no_proxy_hosts`

| Option | Description | Selected |
|--------|-------------|----------|
| Adopt upstream's replacement, verify fork behavior preserved | Take the new pipeline; test that `localhost`/`127.0.0.1` still reach the child env var | ✓ |
| Preserve fork logic, layer upstream's on top | Lower regression risk, higher future merge friction | |
| Flag for the executor to decide with the diff in hand | Defer to execution | |

**User's choice:** Adopt the replacement, prove the behavior survived.

---

## `deny_domain` guard placement

Posture was **not** re-opened — ADR-108 settles it as ADAPT. These questions covered mechanism only.

### Q1 — Where the fail-closed guard lives

| Option | Description | Selected |
|--------|-------------|----------|
| Mirror `validate_block_net_conflicts` | CLI-side validator called from both `command_runtime.rs` and `launch_runtime.rs`; `network_policy.rs::build_proxy_config()` as construction point | ✓ |
| Guard inside `nono-proxy`'s `HostFilter` construction | Harder to bypass; pushes policy toward the library boundary | |
| Both — CLI validator + library assertion | Defense in depth; more code | |

**User's choice:** Mirror the existing fork pattern.
**Notes:** the fork already calls `validate_block_net_conflicts` from both runtimes — a guard on only one entry point would be a bypass.

### Q2 — What a deny-only profile does

| Option | Description | Selected |
|--------|-------------|----------|
| Hard error at parse time with guidance | Names the fork's divergence from upstream explicitly | ✓ |
| Silently retain default-deny | Maximally fail-closed; presents as a broken proxy | |
| Warn loudly, retain default-deny | Middle ground; warning may scroll away | |

**User's choice:** Hard error at parse time.

---

## Roadmap amendment applied during discussion

Operator chose to apply the Phase 113 split immediately rather than defer, so the planner sizes
Phase 109 against real scope. `ROADMAP.md`: range 108-112 → 108-113, new checklist row, new Phase 113
detail section with 4 success criteria, Phase 109 requirements reduced to NET-01 + NET-03, SC2 struck
through with a pointer. `REQUIREMENTS.md`: NET-02 retargeted to Phase 113 with the measurement
rationale recorded inline.

## Claude's Discretion

- Plan/wave decomposition within the 5 commits, provided the two large ones aren't bundled with the three small fixes.
- Wording of the deny-only parse-time error, provided it names the fork's divergence.
- Whether `6fb7ecbf` and `23d93fc9` share a plan.

## Deferred Ideas

- NET-02 SPIFFE/SPIRE → Phase 113, ADR-gated.
- The 6 unmapped NET-cluster commits → Phase 112.
- Not discussed, available if planning needs them: folding the 6 Phase-112 NET commits back into 109 now that SPIFFE has left; how SC4's binding rebuild is verified across three phases; a threat review for `#1335`'s forward-proxy serving path; and the still-open v3.4 **WR-03** product decision on `validate_block_net_conflicts` vs strict-filter semantics, which now sits directly beside the new deny_domain guard.
- Reviewed todos, not folded: `20260611-msi-vcredist-prereq.md`, `20260611-poc-cert-broker-clean-host.md` (v3.5 Phase 106, keyword-only matches).
