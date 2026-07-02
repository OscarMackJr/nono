# ADR-100: CI Pipeline Reconcile — Upstream #1245/#1251 Adopt-vs-Adapt Disposition

**Status:** Accepted
**Phase:** 100 — Release Reconcile (Leapfrog 0.66.1 + PyPI Blocker)
**Date:** 2026-07-02
**Authors:** Phase 100 execution

---

## Context

Upstream `nolabs-ai/nono` shipped two related CI commits in the `v0.65.1..v0.66.0` window:

- `ebd94275` (#1245, "ci: idempotent publish-crates + cross-compile check on release
  PRs") — two independent hunks in one commit:
  1. Wraps each of `release.yml`'s three `cargo publish -p {nono,nono-proxy,nono-cli}
     --allow-dirty --token ...` calls in an `if cargo search <pkg> --limit 1 | grep -q
     "\"$VERSION\""` idempotency guard, so re-running `publish-crates` after a partial
     failure does not error on an already-published crate.
  2. Adds a new `cross-compile:` job to `ci.yml` — a 4-target matrix
     (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
     `aarch64-unknown-linux-gnu`) that builds `nono-cli` and, on Linux legs, verifies the
     release binary does not link `libdbus` (portability check). Gated
     `if: ${{ startsWith(github.event.pull_request.title, 'chore: release v') &&
     needs.changes.outputs.run_code_jobs == 'true' }}` — a `release-please`-style bot PR
     title convention.
- `84b5e7ce` (#1251, "ci: fix mapping err in compile step") — a same-day, one-line
  follow-up fixing #1245's own `if:` expression: converts the double-braced
  `if: ${{ EXPR }}` boolean-context form to the quoted-string `if: "EXPR"` form.

This ADR settles whether the fork adopts both hunks verbatim, adapts them, or skips
them, per this milestone's D-05/D-06 invariants (see `.planning/PROJECT.md` §Key
Decisions v3.4 and `.planning/phases/100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker/100-02-PLAN.md`).

---

## Options Considered

### Option A: Adopt verbatim

**Description:** Cherry-pick `ebd94275` and `84b5e7ce` unmodified, including the
`cross-compile` job's original `chore: release v...` PR-title trigger.

**Rejected.** The fork does not use a `release-please`-style bot that opens
`chore: release v...`-titled PRs — the fork's release model is tag-push
(`push: tags: 'v*.*.*'`) plus manual `workflow_dispatch` with a `tag` input directly on
`release.yml` (see `release.yml:3-12`). No PR with that exact title convention will ever
exist on this fork. Verbatim adoption would add a `cross-compile:` job whose `if:`
condition is permanently false — a silent false-assurance gap: the job appears to exist
in `ci.yml`, giving the impression of a cross-compile pre-flight gate, but it can never
fire, so a real `aarch64-unknown-linux-gnu` cross-compile regression would reach a
release tag without ever being caught by this job.

### Option B: Adapt — reconcile both hunks against the fork's actual trigger model

**Description:** Adopt hunk 1 (idempotent `publish-crates`) as a clean 1:1 port using the
fork's existing two-line `VERSION="${{ env.RELEASE_TAG }}"` / `VERSION="${VERSION#v}"`
stripping convention (already used at `release.yml:122-123`, `:130-131`) rather than
upstream's single-line `VERSION="${RELEASE_TAG#v}"`. Adopt hunk 2's job body verbatim
(matrix, checkout/toolchain/cross-install/build/libdbus-check steps) but rewrite the
`if:` trigger to `github.event_name == 'workflow_dispatch'` — a condition this fork's
event model actually produces — and apply `84b5e7ce`'s pre-corrected quoted-string `if:`
form inline from the start, rather than reproducing the double-braced bug and then
re-fixing it in a follow-up commit.

**Taken.**

---

## Decision

**Adapt (Option B).**

1. **`publish-crates` idempotency guard — clean 1:1 ADAPT, no structural conflict.**
   The fork's `publish-crates` job (`release.yml:483-519` pre-change) publishes the same
   3 crates in the same dependency order (`nono` → `nono-proxy` → `nono-cli`) with the
   same `--allow-dirty --token` invocation upstream's hunk targets. Each of the 3 `run:`
   steps now derives `VERSION` via the fork's existing two-line convention, then guards
   the `cargo publish` call with `if cargo search <crate> --limit 1 | grep -q
   "\"$VERSION\""` / `else` / `fi`, echoing a skip message on the already-published
   branch. The existing `sleep 30` indexing waits (nono-proxy, nono-cli steps) and the
   job's `needs: release` / stable-release `if:` gate (`release.yml:488`) are unmodified.

2. **`cross-compile` job — trigger rewritten to `workflow_dispatch`, job body adopted
   verbatim.** The job is inserted into `ci.yml` directly after `audit:` and before
   `docs-checks:`, matching upstream's own placement. `workflow_dispatch:` (no inputs) is
   added to `ci.yml`'s top `on:` block alongside the existing `pull_request`/`push`
   triggers, so an operator can manually fire the full CI matrix — including this job —
   before pushing a release tag, matching the milestone's prepare-only/operator-gated
   release posture. The `if:` condition is written directly as
   `if: "github.event_name == 'workflow_dispatch' && needs.changes.outputs.run_code_jobs
   == 'true'"` — the pre-corrected quoted-string form from `84b5e7ce`, applied from the
   start rather than shipping the double-braced form and fixing it a day later. `needs:
   changes` is preserved: for a non-`pull_request`/non-`push` event (the `changes` job's
   `*` case, `ci.yml:53-57`), `base_sha`/`head_sha` stay empty, the diff-based override
   never executes, and `run_code_jobs` stays at its initialized `true` value
   (`ci.yml:39`) — so a `workflow_dispatch` run gets `run_code_jobs=true` automatically,
   with no `changes` job edit required.

---

## Consequences

- **`cargo search` idempotency check is `[VERIFIED: cargo search nono --limit 1,
  2026-07-01]` MEDIUM-durability.** It is a live, unauthenticated read against the
  crates.io registry search API, functionally confirmed working at research time. If
  crates.io deprecates the search API, the fix is a mechanical swap to `cargo info
  <pkg>@<version>` (or an equivalent registry query) — not a removal of the idempotency
  guard itself.
- **The `release-readiness` verify-dark gate and the signed-MSI sign-before-harvest build
  order (D-06 invariants) are unaffected by either hunk.** Neither change touches the
  `build` job's Windows signing/WiX-harvest sequence (`release.yml:24-` onward) or any
  `verify-dark.ps1`-discovered gate; both edits are confined to `publish-crates` (crates.io
  publication, downstream of signing/harvest) and to `ci.yml` (a pre-release pull-request/
  push/manual-dispatch CI matrix, entirely separate from the tag-triggered `release.yml`
  pipeline).
- **The trigger that replaces the dead `chore: release v...` PR-title condition is
  `workflow_dispatch`** — an operator-invocable manual dispatch on `ci.yml`, distinct
  from `release.yml`'s own `workflow_dispatch` (which takes a `tag` input and performs
  the actual release build). This gives an operator a genuine pre-tag cross-compile
  sanity check without depending on a bot-authored PR title this fork will never
  produce.
- **No changes to the fork's Windows CI legs.** `cross-compile`'s matrix is Linux/macOS
  only (mirroring upstream); the fork's existing Windows-specific CI jobs are untouched.
- **Future upstream sync guidance:** if upstream further modifies the `cross-compile`
  job's trigger or matrix, treat the fork's `workflow_dispatch`-based trigger as a
  deliberate, permanent fork-divergence — do not re-adopt a bot-PR-title trigger in a
  later sync without first confirming the fork has adopted a `release-please`-style
  automation (unlikely, given the manual tag-push/`workflow_dispatch` model this fork
  has used since inception).

---

## References

- Upstream commit: `ebd94275` — `ci: idempotent publish-crates + cross-compile check on
  release PRs (#1245)`
- Upstream commit: `84b5e7ce` — `ci: fix mapping err in compile step (#1251)`
- This plan: `.planning/phases/100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker/100-02-PLAN.md`
- Research: `.planning/phases/100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker/100-RESEARCH.md`
  §Common Pitfalls 3 (dead PR-title trigger), 4 (pre-corrected `if:` form), 6 (`cargo
  search` durability)
- ADR-98: `proj/ADR-98-network-intent-disposition.md` — Status/Context/Decision/
  Consequences shape and naming/format precedent for this milestone's ADRs
- Fork release pipeline: `.github/workflows/release.yml` (`RELEASE_TAG`, `publish-crates`
  job)
- Fork CI pipeline: `.github/workflows/ci.yml` (`changes` job, `audit`/`cross-compile`/
  `docs-checks` job ordering)
