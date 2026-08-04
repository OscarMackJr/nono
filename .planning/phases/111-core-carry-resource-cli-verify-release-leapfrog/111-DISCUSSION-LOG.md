# Phase 111: Core Carry + Resource CLI + Fork-Invariant Verify + Release Leapfrog - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-04
**Phase:** 111-core-carry-resource-cli-verify-release-leapfrog
**Areas discussed:** ResourceLimits placement (CORE-02), Stale Unix help text, RLS-14 release posture + binding repos, VERIFY-01 coverage boundary

**Discussion shape:** The operator selected all four gray areas, then declined the per-area
question rounds and asked for a recommendation instead ("what do you recommend?"). Recommendations
were presented with reasoning and accepted verbatim ("write these into CONTEXT.md as locked
decisions"). The options below are therefore the alternatives Claude weighed and presented, with
the recommended-and-accepted option marked — not options the operator clicked through.

---

## Area 1 — Where `ResourceLimits` lives (CORE-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Adapt — keep CLI-side, align surface only | Reject the core-module absorb; align only flag names/help/semantics. Preserves ADR-86 cleanly; leaves a standing divergence every future sync re-encounters. | ✓ |
| Adopt — take the core module (ADR-86 crossing) | Absorb `resource` into the policy-free core as Phase 86 did for audit/diagnostics. Reduces future sync friction; costs a deliberate boundary change. | |
| Defer — own phase alongside 112/113 | Land CORE-01/VERIFY-01/RLS-14 now, route the module question to its own ADR-gated phase. | |

**User's choice:** Adapt (accepted the recommendation)
**Notes:** Decisive evidence gathered during discussion: the fork's implementation is materially
*more complete* than upstream's — all four limits are kernel-enforced on three platforms
(Job Object / cgroup v2 / `RLIMIT_AS`+`RLIMIT_NPROC`), whereas upstream's core module carries no
enforcement at all. Adopting would relocate a data type and force restructuring of working,
tested, security-critical code for cosmetic convergence, against a requirement that explicitly
forbids new enforcement and regressions. Claude flagged this as the one recommendation where the
opposite case is genuinely arguable; the operator did not flip it.

---

## Area 2 — Stale Unix help text vs real enforcement

| Option | Description | Selected |
|--------|-------------|----------|
| Fix it — in scope for "align semantics" | Correct the false claim that Unix limits are unenforced. A doc fix is not new enforcement. | ✓ |
| Leave it — "no new enforcement" fences it out | Treat any touch of the resource surface as out of bounds. | |

**User's choice:** Fix it (accepted the recommendation)
**Notes:** `cli.rs` ~L2781-2815 states that `--memory`/`--timeout`/`--max-processes` are "accepted
with a warning pending cross-platform follow-up" on Linux/macOS. Verified false during discussion:
`supervisor_macos.rs` enforces `RLIMIT_AS`/`RLIMIT_NPROC` and `supervisor_linux.rs` carries a
cgroup v2 module. A user could reasonably conclude their limits are not applied. Same defect class
as Phase 110 checkpoint defects 1 and 2 (a predicate or doc that stopped tracking its feature).
Constraint attached: the rewrite must be driven by reading the three backends, respecting the
macOS `RLIMIT_AS` = address-space-not-RSS caveat the source itself documents.

---

## Area 3 — RLS-14 release posture + binding repos

| Option | Description | Selected |
|--------|-------------|----------|
| Prepare-only + bump both binding repos in-phase | Matches v3.3/v3.4; keeps `0.70.0` from entangling with the unresolved v0.66.1 tag decision. Binding repos bumped with `maturin`/`napi` rebuild. | ✓ |
| Prepare-only, binding repos deferred | Smaller phase; risks the v3.4-class struct-drift failure that only a binding build catches. | |
| Actually cut/push the release | Rejected — v3.5's tag push is still blocked on the Azure 403. | |

**User's choice:** Prepare-only, both binding repos in-phase (accepted the recommendation)
**Notes:** Specific reason beyond precedent: pushing `0.70.0` while v3.5's `v0.66.1` remains
blocked on the Azure Trusted Signing 403 would entangle two release decisions that must stay
separate. Binding-repo inclusion rests on v3.4's durable lesson that sibling-repo drift is caught
only by `maturin build`/`napi build`, never by `cargo build --workspace`.

---

## Area 4 — VERIFY-01 coverage boundary

| Option | Description | Selected |
|--------|-------------|----------|
| Cover the combined 108-111 surface | Re-certify Phase 110's surface too, since `110-08` predates four library-behaviour fixes. ~10 min of gate time. | ✓ |
| Scope strictly to Phase 111's own changes | Smaller, faster; leaves a certification artifact describing a tree that no longer exists. | |

**User's choice:** Combined surface (accepted the recommendation)
**Notes:** Two grounds. (1) `110-08` certified against a tree superseded by `7c7a189c`,
`ea26b5b2`, `6d7ef719`, `4aec1944` — all landed 2026-08-04. (2) More pointedly, `has_port_rules()`
shipped *broken* straight through that certification, so the gate demonstrably failed to catch a
real defect inside its own declared scope. Re-running is cheap relative to that demonstrated miss.

---

## Cross-cutting decisions captured

- **ADR-111 required** for the CORE-02 disposition — matches the project pattern of gating
  contentious absorbs behind a standalone ADR (`proj/ADR-108-deny-domain-posture.md`, proposed
  ADR-113 for SPIFFE). Without it the next UPST audit re-litigates the same commit.
- **Fork flag names frozen** — `--memory`/`--max-processes`/`--cpu-percent`/`--timeout` are shipped
  public CLI surface. Upstream differences get an alias, never a rename. Precedent: Phase 110-01's
  `windows_low_il_broker`/`windows_interpreters` back-compat handling.
- **Standing-divergence entry** in the Phase 108 ledger, mirroring how the tool-sandbox subsystem
  was routed to v3.7 rather than silently dropped.

## Claude's Discretion

- Plan/wave decomposition and task ordering.
- Whether CORE-01's two carries (#1378, #1424) land in one plan or two.
- Exact ADR-111 section structure.
- Which fork-invariant assertions to encode beyond the two clippy gates and two binding builds.
- Handling of the documented pre-existing test-failure baseline (11 `--bin nono` failures, plus 13
  more surfaced by `--no-fail-fast` in 110-08) — do not chase as regressions, do not fabricate GREEN.

## Deferred Ideas

- **Adopting upstream's core `resource` module** — rejected for this milestone; if a future
  milestone wants library-level resource types for binding consumers, that is its own ADR-gated
  phase, not a drive-by during a sync.
- **Unix enforcement parity beyond what exists** — out of bounds (CORE-02 forbids new enforcement).
  Gaps found while correcting the help text go to `deferred-items.md`.
- **`20260611-msi-vcredist-prereq`** (todo, reviewed not folded) — MSI VC++ redistributable
  prerequisite; clean-host install territory (v3.5 Phase 106). Keyword-only match, score 0.6.
- **`20260611-poc-cert-broker-clean-host`** (todo, reviewed not folded) — POC cert / broker spawn
  on a clean host; belongs with v3.5's clean-host UAT, blocked on the Azure 403.
