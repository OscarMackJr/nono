# Phase 96: Cross-Target Toolchain - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-26
**Phase:** 96-cross-target-toolchain
**Areas discussed:** Linux-gnu mechanism, apple-darwin disposition, Drift-fix scope, Doc home + retirement

---

## Linux-gnu mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| cross clippy | `cross clippy --target x86_64-unknown-linux-gnu`; cross+Docker already installed, image ships gnu-gcc, zero new host install | ✓ |
| Native gnu-gcc linker | Keep literal `cargo clippy --target` string, install MinGW/gnu cross-gcc on host | |
| WSL2 native build | Run gate inside WSL2 Ubuntu with native Linux toolchain | |

**User's choice:** cross clippy
**Notes:** Scout confirmed cross 0.2.5 + Docker 29.5.3 present and both rustup std targets already added.

### Command-string reconciliation (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| cross is the command | Document the `cross clippy ...` form as canonical; it discharges SC#2's `cargo clippy` contract by running cargo clippy inside the container; update CLAUDE.md + checklist | ✓ |
| Keep cargo string, cross as runner | Preserve literal `cargo clippy --target` string as contract; document cross only as wrapper (string not independently runnable on host) | |

**User's choice:** cross is the command

---

## apple-darwin disposition

| Option | Description | Selected |
|--------|-------------|----------|
| Time-boxed attempt, then blocker | Evaluate one viable path (cargo-zigbuild) bounded; on failure write XTGT-03(b) hard-blocker + PARTIAL→CI | ✓ |
| Go straight to hard-blocker | Treat SDK-from-Windows as known-infeasible; skip attempt, write rationale now | |
| Full osxcross stand-up | Complete osxcross toolchain so literal apple-darwin clippy runs natively | |

**User's choice:** Time-boxed attempt, then blocker

### Stop condition (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| One path, one plan-wave | Try exactly ONE approach (cargo-zigbuild), cap at single plan/wave; no clean exit → hard-blocker | ✓ |
| Any SDK-acquisition need = immediate stop | Stop the moment any path needs the proprietary SDK on the host (licensing) | |
| Researcher decides the bound | Leave cutoff to researcher feasibility findings | |

**User's choice:** One path, one plan-wave (note: the SDK-acquisition licensing stop was folded in as a co-equal stop condition — see CONTEXT D-04)

---

## Drift-fix scope

| Option | Description | Selected |
|--------|-------------|----------|
| Fix all surfaced drift | Fix everything the gate reports incl. upstream-inherited; gate must exit 0; no #[allow] silencing | ✓ |
| No-new-since-baseline bound | Mirror Phase 95 D-04; fix only sync-introduced drift, defer pre-existing (conflicts with SC#2 exit-0) | |
| Fix all, but escape-hatch large finds | Default fix-all, pause for large/structural clusters as a scope decision | |

**User's choice:** Fix all surfaced drift

---

## Doc home + retirement

### Doc home

| Option | Description | Selected |
|--------|-------------|----------|
| Checklist template | Setup + invocation in cross-target-verify-checklist.md (already owns setup + PARTIAL sections) + CLAUDE.md pointer | ✓ |
| CLAUDE.md Coding Standards | Full setup in CLAUDE.md bullet, checklist references it | |
| New docs/ file | Dedicated docs/cross-target-toolchain.md referenced from both | |

**User's choice:** Checklist template

### Retirement meaning (XTGT-04)

| Option | Description | Selected |
|--------|-------------|----------|
| Per-gate, evidence-based | linux-gnu → local-required; apple-darwin follows its outcome; decision tree Q2/Q3 rewritten accordingly | ✓ |
| Retire both gates' default now | Both default to local-run-required (contradicts reality if darwin blocks) | |
| Keep PARTIAL default, add 'prefer local' note | Leave default intact, add preference note (arguably doesn't satisfy XTGT-04) | |

**User's choice:** Per-gate, evidence-based

---

## Claude's Discretion

- Exact `cross` image tag to pin, plan/wave decomposition, precise cargo-zigbuild invocation, and
  whether to wire the linux-gnu gate into a `make` target — planner/researcher discretion within the
  locked decisions.

## Deferred Ideas

- Crate leapfrog ≥0.65.0 / release pipeline / runbook → Phase 97.
- Cross-target verification of `nono-py` / `nono-ts` binding repos → out of scope (separate repos).
- Dedicated `make` target / CI parity job for the linux-gnu gate → possible planner discretion, not locked.
