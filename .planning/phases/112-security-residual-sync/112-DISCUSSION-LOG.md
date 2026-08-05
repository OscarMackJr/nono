# Phase 112: Security + Residual Sync - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-05
**Phase:** 112-security-residual-sync
**Areas discussed:** SEC-09 guard relaxation posture, Scope of the 8 unnamed cluster commits (dissolved on evidence), Security-review placement & plan shape, RES-01/RES-02 absorb-vs-skip lean, Heavyweight handling, Disposition recording, Linux verification depth

---

## Gray area selection

All four presented areas were selected: SEC-09 guard relaxation posture, Scope of the 8 unnamed
cluster commits, Security-review placement & plan shape, RES-01/RES-02 absorb-vs-skip lean.

---

## Scope of the 8 unnamed cluster commits — DISSOLVED BEFORE ASKING

This area was selected by the user but **never put to a decision**, because grounding it against
the ledger showed it was already resolved.

The `security-residual-and-misc` cluster is 18 commits. The ledger's Rollup Support Notes
(§ line ~536) itemize every one: 10 D-18-named anchors + `f6f027511f` (security-relevant, hand-
checked) + 7 residual (`f050643479`, `7fe0c8283810`, `0158d52f0a`, `762eb05bd3d` registry/
update-check; `503045801a` PTY-teardown; `4cc0af2c52`, `9840a16f35` test-only). Tally 11 + 7 = 18 ✓.

Every SHA already has a named requirement home (SEC-01..09, RES-01, RES-02). There was no orphan
set to adjudicate, so no question was asked. Recorded in CONTEXT.md as **D-08** (a verified fact,
not a choice).

---

## SEC-09 guard relaxation posture

**Pre-question finding that reframed the area:** `git show f6f02751` plus live greps established
that the guard upstream removes is gated on `tool_sandbox_runtime.is_some()` /
`tool_sandbox_initial_shim`, that this block is **absent from the fork's
`crates/nono-cli/src/execution_runtime.rs`**, that the fork has **no `command_policies`**, and that
tool-sandbox is the v3.7 standing divergence. The area was originally framed as "how much do you
want to loosen a security guard" — the correct framing is "there is no guard here to loosen."
Options were rewritten accordingly before being presented.

| Option | Description | Selected |
|--------|-------------|----------|
| Won't-sync + v3.7 revisit obligation | Record as won't-sync (target subsystem absent), the ledger's existing 4th disposition, AND add an explicit carry-forward note to the v3.7 tool-sandbox work-list so the guard question is consciously decided when tool-sandbox lands | ✓ |
| Won't-sync, no v3.7 obligation | Record and close SEC-09; guard question resurfaces in v3.7 with no recorded prior reasoning | |
| Absorb the non-tool-sandbox remainder only | Port only the unrelated clippy/format cosmetics in `profile/mod.rs`, skip the guard removal (mirrors 111-01's selective port) | |

**User's choice:** Won't-sync + v3.7 revisit obligation.
**Notes:** The deciding factor was preventing silent inheritance of upstream's relaxed posture. Without
a recorded obligation, absorbing tool-sandbox in v3.7 would bring the relaxed guard along by
default with no one having decided that. Same dangling-TODO failure mode ADR-111 was written to
close. Captured as **D-01**; the carry-forward note is the deliverable, not a code change.

---

## Security-review placement & plan shape

| Option | Description | Selected |
|--------|-------------|----------|
| Reality-check pass first, then absorb | Wave 1 diff-verifies each SEC-01..09 target against the live fork and assigns disposition BEFORE any absorb task is planned | ✓ |
| Dedicated review plan at the end | Absorb across waves, then one final fork-invariant review plan (closest literal read of the Phase 87 precedent) | |
| Review folded into each absorb plan | Each SEC plan carries its own review section | |

**User's choice:** Reality-check pass first, then absorb.
**Notes:** Chosen on the direct evidence of SEC-09 being phantom work — the trailing-review option
would not have surfaced that until an executor was already mid-plan. This is a deliberate departure
from the literal Phase 87 "review at the end" reading; ROADMAP SC1 only requires the review be
*distinct from a release-cut phase*, which a front gate satisfies. Captured as **D-02**.

---

## RES-01 / RES-02 absorb-vs-skip lean

| Option | Description | Selected |
|--------|-------------|----------|
| Skip-biased, absorb only on proven fork applicability | Default skip-with-reasoning; absorb only where a diff proves applicability to the fork's divergent implementation | ✓ |
| Absorb-biased, skip only on proven conflict | Maximize upstream parity, shrink future sync debt | |
| Split: absorb RES-02 test-only, skip-bias the rest | Treat the 2 test-only commits as low-risk parity wins | |

**User's choice:** Skip-biased, absorb only on proven fork applicability.
**Notes:** Both residuals sit on heavily fork-divergent surfaces — RES-01 against the v3.0 HKLM
machine-policy spine and divergent update behavior, RES-02 against the v2.7–v2.13 Windows PTY/
broker rework. ROADMAP SC2 already accepts skip as first-class; the constraint retained is that
skips must be *recorded with reasoning*, never silently dropped. Captured as **D-03**.

---

## Heavyweight handling (raised by Claude after diffstat measurement)

Not among the four originally-presented areas. Raised after measuring all 11 SEC targets: SEC-01
(+1,873), SEC-02a (+4,425) and SEC-07 (+1,927) account for ~8,200 of ~9,700 total insertions —
larger than Phases 109 and 110 combined. SEC-07 alone adds a 696-line `proxy_command.rs` with no
fork equivalent.

| Option | Description | Selected |
|--------|-------------|----------|
| Keep in 112, one plan each, own waves | Phase stays whole; heavyweights get dedicated plans, six small items batched; expect 8–10 plans | ✓ |
| Split heavyweights into a follow-on phase | Move SEC-01/02a/07 to a new phase; requires ROADMAP amendment and re-mapping three requirement IDs | |
| Let the reality-check pass decide | Defer the split to Wave 1's disposition output | |

**User's choice:** Keep in 112, one plan each, own waves.
**Notes:** Accepts a large phase rather than amending the roadmap. Captured as **D-04**, with the
explicit caveat that D-02's reality-check pass runs first — if the heavyweights collapse to
won't-sync or heavy adapt-down, the planner may consolidate. The diffstat is an upper bound on
effort, not a target.

---

## Disposition recording

| Option | Description | Selected |
|--------|-------------|----------|
| Standing-divergence addendum in the 108 ledger | Extend the mechanism Phase 111-03 established for `e6d26871`/`34c2c975`; ledger is the canonical work-list | ✓ |
| New ADR-112 + ledger cross-reference | Full ADR mirroring ADR-108/ADR-111 shape | |
| Both — ledger table + ADR for load-bearing calls | Per-commit table for all 18 plus a short ADR for the judgment calls | |

**User's choice:** Standing-divergence addendum in the 108 ledger.
**Notes:** Cheapest option consistent with the precedent set two phases earlier, and the ledger is
where a future UPST sync looks first. Captured as **D-05** with a contingent escalation clause: if
the reality-check surfaces a disposition carrying genuine security judgment — in particular any
*adopt* that loosens an existing fork guard — that single decision should be escalated to an ADR.
Also noted that the ledger self-declares "Ledger closed," and Phase 111 already set the precedent
for an explicit locked exception appended after closure.

---

## Linux verification depth

| Option | Description | Selected |
|--------|-------------|----------|
| Cross-target clippy + `cross test`, no live-kernel UAT | Both mandatory gates per D-10 plus `cross test --target x86_64-unknown-linux-gnu` on affected modules; phase stays autonomous | ✓ |
| Add a live-kernel checkpoint for SEC-05/SEC-06 | Human-run live-kernel verification like Phase 110's PROF-03e | |
| Defer Linux-only enforcement to a Linux-parity phase | Route SEC-03/05/06 elsewhere | |

**User's choice:** Cross-target clippy + `cross test`, no live-kernel UAT.
**Notes:** Keeps the phase autonomous end-to-end on a Windows host. Precedent is Phase 111-01,
which proved a Windows-uncompilable test passed via `cross test` (`ok. 1 passed`). Recorded
alongside the SEC-05 starting-state finding: the fork **already grants `AccessFs::Refer` generally**
(`crates/nono/src/sandbox/linux.rs:365`), so the reality-check must establish whether the
execute-restriction layer specifically lacks it rather than assuming the whole 89-line change is
new. Captured as **D-06**.

---

## Claude's Discretion

- Exact wave count and plan-to-requirement grouping beyond D-04's constraints.
- Whether the six batched SEC items land in one plan or two, and their ordering.
- Concrete format of the reality-check disposition table, provided all 18 SHAs appear with an
  explicit disposition and cited evidence.
- Whether SEC-02's three commits are absorbed as one unit or staged (`9b692e07` is +4,425;
  `d033c631` is +4).

## Deferred Ideas

- Tool-sandbox subsystem absorb (PR #1105) — owns the v3.7 milestone; SEC-09's carry-forward note
  is filed against it, not executed here.
- Splitting the heavyweights into a follow-on phase — considered and rejected (D-04); revisit only
  if the reality-check pass shows they do not collapse.
- ADR-112 — contingent escalation only, per D-05.
- Live-kernel UAT for SEC-05/SEC-06 — rejected for this phase (D-06); candidate for a future phase
  with live-Linux verification set up.
- Fixing `profile_cmd.rs`'s shared-fixture flake (fixed `%TEMP%` dir → `tempfile::TempDir`) —
  recorded in Phase 111's UAT/verification addendum.
- Phase 108 prose arithmetic defects (NET says 5 `none` rows vs 6 actual; PROF says 4 vs 3;
  errors cancel) — flagged for whoever revisits Plans 108-03/108-04.

## Todos Reviewed, Not Folded

- `20260611-msi-vcredist-prereq.md` (score 0.60) — generic keyword match only; v3.5 release/UAT infra.
- `20260611-poc-cert-broker-clean-host.md` (score 0.60) — generic keyword match only; v3.5 release/UAT infra.
