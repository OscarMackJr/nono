# Phase 94: UPST10 Divergence Audit - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-25
**Phase:** 94-upst10-divergence-audit
**Areas discussed:** Carve-out re-touch check, Upstream remote relocation, Inspection depth default, Release-readiness foreshadow

---

## Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| Carve-out re-touch check | Grep window for the 3 recorded fork-divergence carve-out files | ✓ |
| Upstream remote relocation | Repoint `upstream`, decide legacy fate, Future Cycles trigger | ✓ |
| Inspection depth default | Uniform actual-diff vs risk-tiered (Phase 85 precedent) | ✓ |
| Release-readiness foreshadow | Flag release/version commits for Phase 97 | ✓ |

**User's choice:** All four selected.
**Notes:** Methodology itself is locked by the Phase 85 precedent; these four were
presented as the genuinely open, fork-specific decisions.

---

## Carve-out Re-touch Check

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated section + grep | Standalone section; `git log <window> -- <file>` per carve-out; flag hits + record clean | ✓ |
| Per-cluster annotation only | Annotate inline only when a cluster touches a carve-out file | |
| Defer to Phase 95 | Let git surface the conflict during the actual cherry-picks | |

**User's choice:** Dedicated section + grep (D-04, D-05).
**Notes:** Carve-outs = CR-02 (`audit.rs` records_verified), CR-01 (`bindings/c/src/`
clear_last_call_state), Cluster F (proxy route/connect/reverse/server/proxy_runtime +
absent tls_intercept/ + EffectiveProxySettings). Hits flagged "expected conflict —
preserve fork expression"; zero-hit recorded as clean.

---

## Upstream Remote Relocation

| Option | Description | Selected |
|--------|-------------|----------|
| Replace + keep legacy | `upstream` → nolabs-ai; rename old → `upstream-legacy` | ✓ |
| Replace, drop legacy | `upstream` → nolabs-ai; remove always-further entirely | |
| Add as new remote | Keep `upstream` as-is; add `nolabs` as a separate remote | |

**User's choice:** Replace + keep legacy (D-06).
**Notes:** Continuity is byte-clean (v0.64.0 SHA identical), so legacy is provenance-only
insurance. PROJECT.md `## Upstream Parity Process` references updated to nolabs-ai (D-07).

### Future Cycles trigger (sub-question)

| Option | Description | Selected |
|--------|-------------|----------|
| Next release tag | Trigger = nolabs-ai ships any `v*` past v0.65.1 | ✓ |
| Drift-count threshold | Trigger = ≥ N unabsorbed substantive commits | |
| Time cadence | Trigger = fixed interval | |

**User's choice:** Next release tag (D-08).

---

## Inspection Depth Default

| Option | Description | Selected |
|--------|-------------|----------|
| Uniform actual-diff | `git show` every substantive commit + re-export scan | ✓ |
| Risk-tiered (Phase 85) | actual-diff only for shared-surface/FFI/boundary/fork-divergent | |
| Let the researcher decide | Per-cluster depth chosen from drift output | |

**User's choice:** Uniform actual-diff (D-09).
**Notes:** Window is far smaller than UPST9's 90 commits, so full-diff cost is low and it
removes the additive-looking-but-actually-boundary risk.

---

## Release-Readiness Foreshadow

| Option | Description | Selected |
|--------|-------------|----------|
| Flag in won't-sync row | One-line cross-ref noting version-floor metadata for Phase 97 | ✓ |
| Dedicated release section | Standalone enumeration of all Cargo.toml/CHANGELOG/dep commits | |
| Keep audit sync-only | No release foreshadowing at all | |

**User's choice:** Flag in won't-sync row (D-10).
**Notes:** Release/version commits go in won't-sync as usual (fork leapfrog convention);
a one-line cross-ref gives Phase 97 the ≥0.65.0 floor without a new section.

---

## Claude's Discretion

- Cluster naming/theme labels and internal ledger section ordering (follow Phase 85 shape).
- The exact set of empirical spot-check files beyond the mandatory D-04 carve-out paths.

## Deferred Ideas

- Standalone "Release Readiness" section — rejected in favor of D-10 one-line cross-ref.
- Drift-count / time-based sync triggers — rejected in favor of next-`v*`-tag (D-08).
- Absorption / cherry-pick → Phase 95; cross-target toolchain → Phase 96; crate leapfrog +
  release pipeline + runbook → Phase 97.
