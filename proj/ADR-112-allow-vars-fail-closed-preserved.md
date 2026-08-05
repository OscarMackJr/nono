# ADR-112: `allow_vars` Empty/Omitted Semantics — Preserve the Fork's Fail-Closed Default

**Status:** Accepted
**Phase:** 112 — Security + Residual Sync
**Date:** 2026-08-05
**Authors:** Phase 112 execution

---

## Context

Upstream `nolabs-ai/nono` commit `a5a441c25769ecec663aed1d7038ab2b8814d43f` (#1204, "fix(profile):
empty allow_vars no longer strips all env vars") re-typed `EnvironmentConfig.allow_vars` from
`Vec<String>` to `Option<Vec<String>>` and changed the runtime resolution in
`crates/nono-cli/src/profile_runtime.rs` from `.map(...)` to `.and_then(...)`. The practical effect,
confirmed live via `git show a5a441c2 -- crates/nono-cli/src/profile_runtime.rs
crates/nono-cli/src/profile/mod.rs` immediately before writing this ADR:

- **Upstream (post-`a5a441c2`):** `allow_vars` **absent** from the profile's `environment` block
  (the JSON key not written at all) resolves to `None` — no allow-list filter is activated, and
  the sandboxed child inherits **every** parent environment variable. Only an **explicit**
  `"allow_vars": []` resolves to `Some(vec![])`, which activates the filter and blocks everything.
  Upstream's own commit message frames this as a bug fix: their prior behavior collapsed both
  "absent" and "explicit empty" to `Some([])`, silently stripping `PATH`/`HOME` from any profile
  that set `deny_vars` or `set_vars` without also writing `allow_vars`.

- **This fork (current, unchanged by this plan):**
  `crates/nono-cli/src/profile_runtime.rs:862-872` resolves `allowed_env_vars` via
  `profile.environment.as_ref().map(|env_config| { ... env_config.allow_vars.clone() })` — a
  `.map`, not upstream's `.and_then`. Because `allow_vars` is a plain `Vec<String>`
  (`#[serde(default)]`, not `Option<Vec<String>>`), **both** "key absent from JSON" and "explicit
  `[]`" deserialize to an empty `Vec`, and both cases produce the **same** result:
  `Some(env_config.allow_vars.clone())` — i.e. `Some(vec![])`, which activates the filter and
  strips every inherited variable except nono-injected credentials. This is deliberate: the
  in-code comment at `profile_runtime.rs:840-846` traces it to **Plan 34-08a Task 3 (D-20 manual
  replay of upstream `1b412a7`) and Task 5 (D-20 replay of v0.52.0 `780965d7`)** — a prior phase
  that hand-replayed an *earlier* upstream fail-closed fix for the exact same class of bug upstream
  is now, in `a5a441c2`, reversing again. The fork's own regression-guard test
  `empty_allow_vars_fails_closed` (`profile_runtime.rs:989`) asserts exactly this: "an
  `EnvironmentConfig` with empty `allow_vars` MUST surface as `Some(vec![])` (strip-all /
  fail-closed) rather than `None` (no filter / inherit-all)."

Upstream and the fork are not merely stylistically different — they encode **opposite security
defaults** for the identical input shape (`environment: { deny_vars: [...] }`, no `allow_vars`
key):

| Input: `environment` block present, `allow_vars` key omitted | Upstream (post-`a5a441c2`) | This fork (unchanged) |
|---|---|---|
| Resolved `allowed_env_vars` | `None` | `Some(vec![])` |
| Effect on sandboxed child | Inherits **every** parent env var | Inherits **zero** parent env vars (except nono-injected credentials) |

This is the exact scenario `112-CONTEXT.md`'s D-05 names as the one contingent ADR-112 escalation
trigger: "any **adopt** that loosens an existing fork guard." `112-RESEARCH.md` and
`112-DISPOSITION-TABLE.md` (Wave 1 reality-check, both dated 2026-08-05) independently confirm this
finding and flag SEC-08 for exactly this escalation.

---

## Decision

**PRESERVE, do not adopt.**

The fork's `allow_vars: Vec<String>` field and its always-`Some(...)`-when-`environment`-is-Some
resolution in `profile_runtime.rs:862-872` stay **exactly as they are**. This ADR accompanies
**zero source-code change** — `git diff -- crates/nono-cli/src/profile_runtime.rs` is empty for
this plan.

Upstream's `a5a441c2` is **not** ported, in any form (neither verbatim nor adapted), because there
is no way to adopt its `Option<Vec<String>>`-with-`None`-means-allow-all semantic without
reopening the exact env-var-leak class the fork's own prior Phase-34 fix (`D-20` replay of
upstream `1b412a7`/`780965d7`) was written to close. A profile author who sets `env_credentials`
and `deny_vars` but forgets to write `allow_vars` is a realistic, expected authoring pattern — not
an edge case — and the fork's position is that this pattern must fail closed (strip all inherited
vars, keep only nono-injected credentials), not fail open (inherit everything, including
`ANTHROPIC_API_KEY`-style ambient secrets the profile never intended to pass through).

`empty_allow_vars_fails_closed` (`crates/nono-cli/src/profile_runtime.rs:989`) remains the durable
regression guard against future accidental drift toward upstream's default — including any future
sync that re-encounters `a5a441c2` or a descendant commit and is tempted to treat "all target files
present" as sufficient grounds for `adopt` (the exact anti-pattern `112-RESEARCH.md` names: "file
presence is necessary, not sufficient, evidence for `adopt`").

---

## Why this is not a compromise position

Upstream's commit message is not wrong on its own terms — `Some([])`-collapses-"absent"-and-
"explicit-empty" *is* a source of confusion, and a hypothetical profile author who explicitly
writes `"allow_vars": []` meaning "I want the allow-list mechanism engaged with zero entries" and a
profile author who simply never mentions `allow_vars` are expressing different intents that
upstream's `Option`-based redesign distinguishes cleanly.

But this fork made a considered, previously-litigated choice (D-20, Plan 34-08a) to treat that
ambiguity as unacceptable in the *security-conservative* direction: when in doubt about whether a
profile author meant to filter environment variables, **strip them**, per `CLAUDE.md`'s
non-negotiable "When in doubt, choose the more restrictive option." Adopting upstream's
redesign — even with a differently-named field or a translation shim — would mean any profile that
merely omits `allow_vars` silently switches from "deny inherited env vars by default" to "allow
inherited env vars by default." That is a behavior change with no compensating control, on a
security-relevant default, that would ship silently to every existing fork user's profiles that
rely on (or are unaware they rely on) the current fail-closed behavior.

---

## Consequences

1. **Standing divergence, not a deferral.** Unlike SEC-09's `DEFERRED -> v3.7` treatment (tool
   sandbox is simply absent from the fork today), this is a **permanent** divergence: the fork
   will never adopt upstream's `Option<Vec<String>>` reinterpretation of `allow_vars`, regardless
   of future upstream syncs, unless a future ADR explicitly supersedes this one with new evidence.

2. **Future upstream syncs must re-affirm, not re-litigate.** Any future sync encountering
   `a5a441c2` or a descendant commit touching `EnvironmentConfig.allow_vars` semantics must cite
   this ADR and confirm `empty_allow_vars_fails_closed` still passes, rather than treat the fork's
   non-adoption as an unabsorbed gap to close.

3. **No user-visible change.** This ADR's decision produces no code diff and no behavior change.
   Profiles authored against the fork's documented semantics (omitted or explicit-empty
   `allow_vars` both strip all inherited vars) continue to behave identically before and after
   Phase 112.

4. **The regression guard is the enforcement mechanism.** `empty_allow_vars_fails_closed` is not
   merely a test — it is the mechanism by which this ADR's decision self-enforces against future
   drift, including well-intentioned refactors that might otherwise "simplify" the `.map` closure
   toward upstream's `.and_then` shape without realizing the semantic consequence.

---

## References

- Upstream commit: `a5a441c25769ecec663aed1d7038ab2b8814d43f` — `fix(profile): empty allow_vars no
  longer strips all env vars (#1204)`
- `crates/nono-cli/src/profile_runtime.rs:840-872` — the fork's existing fail-closed
  `allowed_env_vars` resolution this ADR preserves unchanged, with its in-code provenance comment
  citing Plan 34-08a Task 3/5 (D-20 manual replay of upstream `1b412a7`/`780965d7`)
- `crates/nono-cli/src/profile_runtime.rs:989` — `empty_allow_vars_fails_closed`, the durable
  regression guard this ADR cites as the enforcement mechanism
- `.planning/phases/112-security-residual-sync/112-CONTEXT.md` — D-05, the contingent
  ADR-112-escalation trigger this ADR fires under ("any adopt that loosens an existing fork guard")
- `.planning/phases/112-security-residual-sync/112-RESEARCH.md` — the SEC-08 disposition row and
  "State of the Art" table entry contrasting the fork's vs. upstream's default
- `.planning/phases/112-security-residual-sync/112-DISPOSITION-TABLE.md` — the Wave 1 live-re-run
  confirmation of the same finding, dated 2026-08-05
- `proj/ADR-111-resource-limits-boundary.md` — the structural precedent (Status/Phase/Date/Authors
  header, Context/Decision/Consequences shape) this document mirrors
- `proj/ADR-108-deny-domain-posture.md` — the ADAPT/PRESERVE-vs-adopt precedent this ADR extends to
  a "reject entirely, zero code change" outcome
- `./CLAUDE.md` § "Security Considerations" — "SECURITY IS NON-NEGOTIABLE... When in doubt, choose
  the more restrictive option," the governing principle this decision applies
