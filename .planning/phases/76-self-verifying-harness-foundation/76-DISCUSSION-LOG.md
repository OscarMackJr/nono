# Phase 76: Self-Verifying Harness Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-16
**Phase:** 76-self-verifying-harness-foundation
**Areas discussed:** Verdict contract, Gate registration, SKIP precondition detection, Verdict persistence + reuse

---

## Verdict contract

| Option | Description | Selected |
|--------|-------------|----------|
| Typed JSON + 3-way exit | `{gate,verdict,reason,detail,timestamp}`; exit 0=PASS, 2=FAIL, 3=SKIP, 1/4+ reserved for harness error | ✓ |
| JSON + binary exit | Same JSON, exit 0 (PASS/SKIP) vs non-zero (FAIL); loses at-a-glance SKIP signal | |
| Verdict enum only, no detail | `{gate,verdict}` + exit code; loses the WHY behind FAIL/SKIP | |

**User's choice:** Typed JSON + 3-way exit
**Notes:** Distinct FAIL vs SKIP exit codes satisfy criteria #4 ("no ambiguity about why the skip occurred"). Noted that the Phase 81 aggregator maps `PASS_WITH_SKIPS` → exit 0, consistent with the per-gate scheme.

---

## Gate registration

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-discovered gate files | `scripts/gates/<name>.ps1` with `Test-Precondition`+`Invoke-Gate`; runner globs dir | ✓ |
| Central dispatch table | Single file with name→scriptblock map; every phase edits core (merge contention) | |
| Manifest + scripts | `gates.json` listing name→path+preconditions; more machinery than ~5 gates warrant | |

**User's choice:** Auto-discovered gate files
**Notes:** Phases 77-80 ADD a file, never edit the core runner. `--gate` and `--all` both derive from the glob.

---

## SKIP precondition detection

| Option | Description | Selected |
|--------|-------------|----------|
| Probe returns reason or null | `Test-Precondition` → `$null` (run) or string reason (SKIP); runner checks BEFORE `Invoke-Gate` | ✓ |
| Gate self-reports SKIP | No hook; `Invoke-Gate` decides — mixes precondition with assertion logic, easy false PASS | |
| Centralized precondition registry | Central map of gate→required service/exe — reintroduces a central file every phase edits | |

**User's choice:** Probe returns reason or null
**Notes:** SKIP = expected host-absence; FAIL = precondition met but assertion failed; crash in Invoke-Gate → harness-error exit, never silent.

---

## Verdict persistence + reuse

| Option | Description | Selected |
|--------|-------------|----------|
| Write file + re-run (persistence) | Each gate writes `.nono-runtime/verdicts/<gate>.json`; `--all` re-runs + reads dir | ✓ |
| In-memory only on --all | No per-gate file; nothing to inspect after a single `--gate` run | |
| Append to one log | JSONL append; must dedup re-runs + parse ordering — fragile | |
| Net-new runner, gates may shell out (reuse) | Fresh `verify-dark.ps1`; `Invoke-Gate` may shell to cargo/msiexec and map exit code | ✓ |
| Extend windows-test-harness.ps1 | Couples host-gated UAT to CI suite; bloats one file; regression risk | |
| Decide during planning | Defers a foundational contract decision the user wanted locked | |

**User's choice:** Write file + re-run **and** Net-new runner, gates may shell out
**Notes:** `.nono-runtime/verdicts/<gate>.json` (gitignored — confirm). Phase 78's classify gate becomes a thin wrapper over `cargo test --bin nono-agentd -- classify`, mapping `$LASTEXITCODE` to PASS/FAIL. Keep separate from `windows-test-harness.ps1` (different audience: CI build/smoke vs host-gated UAT replacement).

---

## Claude's Discretion

- Exact field naming/casing within the verdict JSON beyond the agreed keys.
- The precise self-check assertions inside `harness-self-check`'s `Invoke-Gate` (must exercise emit + persist + round-trip).
- Whether the runner accepts `--all` explicitly or treats "no `--gate`" as the all-run (Phase 81 formalizes).

## Deferred Ideas

- Feature gate bodies (copilot-e2e, wfp-egress-isolation, clean-host-install, classify) — owned by Phases 77-80.
- The `--all` aggregator + `overall`/`PASS_WITH_SKIPS` rollup logic — formalized in Phase 81 (DARK-02).
- Reusing `Invoke-LoggedCargo` log-tee'ing inside gates — optional ergonomics, not a Phase 76 deliverable.
