---
quick_id: 260604-nue
slug: proceed-during-signing-cert-wait
status: complete
date: 2026-06-04
verdict: YES — Phase 54 is safe to start now; isolate code-bearing Phase 55+ from the v0.58.0 release scope
---

# Can I proceed (e.g. Phase 54) while waiting ~days for the Azure signing cert?

## Verdict

**Yes — start Phase 54 now.** It is the ideal "wait productively" task. The Azure Trusted Signing
cert blocks **only** Phase 61's *tag/release* step (D-02); it does not block any other work.

## Why Phase 54 is safe and independent (grounded in ROADMAP.md)

- Phase 54 = **UPST7 Audit**, output is `DIVERGENCE-LEDGER.md` (upstream `v0.57.0..v0.59.0`,
  per-cluster dispositions, ADR review, re-export cross-check). **Pure analysis/docs — no code,
  no built-artifact change, no release-pipeline touch.**
- ROADMAP dependency line, verbatim: *"Depends on: Phase 53 (release ships first; audit **may
  proceed concurrently** but must not block drain)."* Phase 53 already shipped (v0.57.5). So 54 is
  explicitly unblocked and concurrency-safe.
- The phase dir already exists: `.planning/phases/54-upst7-audit/`.
- Bonus: Phase 54 is a **HARD GATE** — its dispositions are the input to Phases 55/56/58/59. Doing
  it during the cert wait removes the critical-path blocker for the whole UPST7 chain.

## The one thing to protect: the v0.58.0 release scope

`main` is already bumped to **version 0.58.0** and staged for tagging the moment the cert lands
(Public-Trust profile fix → smoke test green → merge PR #5 → `git tag -a v0.58.0`). **Anything
that lands on `main` before that tag becomes part of the signed v2.9 release** (release.yml builds
from the tagged commit).

| Work | Lands on main safely before the tag? |
|------|--------------------------------------|
| **Phase 54** (audit — `.planning/` docs only) | **Yes.** Docs don't change built binaries; release.yml builds from `crates/`, not `.planning/`. Commit straight to main. |
| **Phase 55+** (cherry-picks — real code) | **Hold off / branch.** These change the shipped binaries. Don't merge to main until *after* v0.58.0 is tagged — or consciously decide to widen the v2.9 release scope. |

**Recommendation:** keep the signed v2.9 release scoped to what Phases 60/62 delivered. When the
cert arrives, tag **v0.58.0 first**, then merge Phase 55+ code for the next version (v0.59.0 /
next milestone). Do Phase 55+ on a feature branch in the meantime.

## Don't disturb the staged release plumbing while waiting

- Leave **PR #5** (`chore/trusted-signing-release-wiring`) open and unmerged — it's ready.
- Leave the temporary **`trusted-signing-smoke.yml`** workflow on main (remove it after v0.58.0
  ships). Phase 54 work won't touch either.
- The cert-side remaining step is unchanged: create a **Public Trust** cert profile (current
  `NonoCertProfile` is private/enterprise → `UnknownError`), repoint `TRUSTED_SIGNING_PROFILE`,
  re-run the smoke test for `Status: Valid`.

## Concrete next move (during the wait)

```
/gsd:discuss-phase 54     # optional context pass, then:
/gsd:plan-phase 54
/gsd:execute-phase 54     # audit/docs — fine to land on main
```

## Self-Check: PASSED
- Verdict grounded in ROADMAP.md (Phase 54 "may proceed concurrently"; 54→55→56/58/59 dep chain;
  54 = audit/doc output). Confirmed `main` at version 0.58.0, phase-54 dir exists, PR #5 + smoke
  workflow staged. No code changed (advisory record only).
