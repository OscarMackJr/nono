# Phase 100: Release Reconcile — Leapfrog 0.66.1 + Pipeline + PyPI Blocker - Context

**Gathered:** 2026-07-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the post-Phase-99 tree **one-step-push-ready at crate version `0.66.1`**. Four
tracked concerns (RLS-10..13):

1. **RLS-10** — version-bump every workspace member + both binding repos to `0.66.1`
   (collision-free floor above upstream `0.66.0`); keep internal path-dep `version` pins
   consistent; regenerate `Cargo.lock`; `make build` clean.
2. **RLS-11** — reconcile upstream CI #1245 (idempotent `publish-crates` + cross-compile
   check on release PRs) and #1251 (compile-step mapping fix) into the fork's prepare-only
   pipeline **without** breaking the `release-readiness` verify-dark gate or the signed-MSI
   build order.
3. **RLS-12** — close the carried-forward `nono-py` `RouteConfig` PyPI blocker so the wheel
   builds and `twine check` / maturin validation passes.
4. **RLS-13** — re-run `scripts/release-dry-run.ps1` + the `release-readiness` gate GREEN at
   `0.66.1`; update `RELEASE-RUNBOOK.md` for the `0.66.1` tag with the PUBLIC-repo pre-push
   checklist. Actual tag push + registry publish stay **operator-gated**.

**Plus (folded this phase):** the two host-gated clean-host install UAT items (see Folded
Todos) — the release phase is where clean-host distribution verification belongs.

**This phase clarifies HOW to execute the above.** New product capabilities belong elsewhere.

</domain>

<decisions>
## Implementation Decisions

### Cross-repo binding scope (RLS-10 + RLS-12)
- **D-01: Both bindings fully in-phase.** Execute the `0.66.1` version bump in **both**
  separate binding repos (`C:/Users/OMack/nono-py`, `C:/Users/OMack/nono-ts`) AND the
  `nono-py` `endpoint_policy` fix, each as **separate DCO-signed commits in their own git
  repos** this phase. These are distinct repositories from the `nono` workspace — plan for
  cross-repo commits, not a single-repo diff.
- **D-02:** Binding **publish** (PyPI wheel, npm package) stays **operator-gated** — same
  prepare-only posture as the crate registry publish. Phase 100 makes them build/validate
  clean and version-consistent; it does not push.

### Version-bump membership (RLS-10)
- **D-03: All members lockstep to `0.66.1`.** RLS-10's enumeration ("5 crates") is **stale** —
  the workspace actually has 7 members. Bump **every** member currently carrying `0.66.0`:
  `crates/nono`, `crates/nono-cli`, `crates/nono-proxy`, `crates/nono-shell-broker`,
  `bindings/c` (nono-ffi), **`crates/nono-fltmgr-client`** (omitted from RLS-10), plus
  `tools/sign-fixture` if it carries a version. No intentional version skew.
- **D-04:** Keep **all internal path-dep `version` pins consistent** across every `Cargo.toml`
  (each member hard-codes `version = "..."` — there is **no** `version.workspace = true`
  inheritance, so every file is edited individually). Regenerate `Cargo.lock`; `make build`
  must pass clean. Publish set remains 3 crates (nono → nono-proxy → nono-cli) per the v3.3
  WR-02 decision — bumping non-published members is for path-dep consistency, not publish.

### CI reconciliation (RLS-11)
- **D-05: Adapt + short ADR.** Cherry-pick/adapt only the applicable hunks of #1245 and #1251;
  **preserve** the fork's prepare-only posture, the signed-MSI build order (sign-before-harvest),
  and the `release-readiness` verify-dark gate. Record the adopt-vs-adapt disposition in a
  **brief ADR** (mirrors the #1225 / ADR-98 pattern) so the divergence is auditable.
- **D-06:** The `release-readiness` verify-dark gate and the signed-MSI order are **invariants** —
  any CI change that would break them is rejected, not worked around.

### PyPI blocker depth (RLS-12)
- **D-07: Minimal `endpoint_policy: None` stub + tracked follow-up.** Add `endpoint_policy: None`
  at the two `nono-py` `RouteConfig` initializers so the wheel builds and `twine`/maturin
  validation passes (RLS-12 literal). This unblocks PyPI now; Python users cannot yet *set*
  endpoint policy — document that limitation.
- **D-08:** The "fully thread `endpoint_policy` through `nono-py` `RouteConfig`" work is
  reserved as a **named future phase** (see Deferred Ideas) — not attempted in Phase 100.

### Release posture (RLS-13)
- **D-09:** Phase 100 = **PREPARE ONLY**. `release-dry-run.ps1` + `release-readiness` re-run
  GREEN at `0.66.1`; `RELEASE-RUNBOOK.md` updated for the `0.66.1` tag with the PUBLIC-repo
  pre-push checklist (no `build_notes/`/`.gsd/` staged; `0.66.1` > upstream `0.66.0` confirmed).
  **Repo stays PUBLIC** (go-private cancelled 2026-07-01 — push no longer altitude-gated, but
  still operator-gated). The actual tag push + registry publish are the single remaining
  operator step, outside this phase.

### Claude's Discretion
- Wave/plan breakdown is the planner's call. A natural shape: (1) workspace version bump +
  path-dep pins + Cargo.lock + build; (2) CI reconcile + ADR; (3) nono-py PyPI stub + nono-py/
  nono-ts version bumps (cross-repo); (4) release-dry-run + release-readiness re-green + runbook
  update; (5) host-gated clean-host UAT (folded, may SKIP_HOST_UNAVAILABLE).
- ADR filename/location is the planner's call (convention: `proj/ADR-NN-*.md`).
- Whether `RELEASE-RUNBOOK.md` is updated in place at its archived v3.3 path or brought forward
  is the planner's call (note the current path in canonical refs).

### Folded Todos
- **`20260611-msi-vcredist-prereq.md`** — clean-Win11-host machine-MSI install with no VC++
  runtime. **Code fix DONE** (`+crt-static` wired, commit `a517284b`; binaries confirmed static;
  service-start non-fatal via `Vital="no"`). **Remaining = host-gated UAT only:** boot a clean
  Win11 VM, confirm the machine MSI installs with no `1603`/rollback and `nono.exe` launches
  (no `0xC0000135`). Host-gated — may SKIP_HOST_UNAVAILABLE if no clean VM is available.
- **`20260611-poc-cert-broker-clean-host.md`** — supervised path (`nono run --profile
  claude-code`) must spawn the broker on a clean host with no manual cert-trust step.
  **Signing-pipeline fix DONE** (release.yml migrated to Azure Trusted Signing, commit
  `20cd68d9`). **Remaining = cut a trusted-signed release + clean-host UAT.** ⚠️ **Depends on
  the active Azure Trusted Signing go-live thread** — signing currently works but the *verify*
  gate fails (`UnknownError`, issuer "Microsoft Enterprise ID Verified Policy AOC CA 01"); a
  genuinely clean-host-trusted MSI is blocked until that is resolved. Host-gated + externally
  blocked — plan as a gated/deferred verification, not a hard Phase-100 exit criterion.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements + roadmap
- `.planning/REQUIREMENTS.md` — RLS-10 / RLS-11 / RLS-12 / RLS-13 (lines ~51-54); note RLS-10's
  crate enumeration is stale (see D-03).
- `.planning/ROADMAP.md` §"Phase 100" — goal + 4 success criteria.

### Release pipeline (RLS-11 / RLS-13)
- `.github/workflows/release.yml` — the fork's release pipeline (prepare-only; Azure Trusted
  Signing; sign-before-harvest MSI order; `publish-crates` 3-crate set). Target of #1245/#1251
  reconcile.
- `scripts/release-dry-run.ps1` — 3-registry dry-run orchestrator; re-run GREEN at 0.66.1.
- `scripts/gates/release-readiness.ps1` — auto-discovered verify-dark release-readiness gate;
  policy violations = FAIL verdict, infra failures throw (v3.3 97-04 decision). Must stay GREEN.
- `.planning/milestones/v3.3-phases/97-release-engineering-leapfrog-pipeline-runbook/RELEASE-RUNBOOK.md`
  — current runbook location (archived v3.3 path); update for the 0.66.1 tag (D-09).

### Version bump surface (RLS-10)
- `Cargo.toml` (workspace) — members list (7 members); `[workspace.package]` (note `repository`
  still points at `always-further/nono` — flag for the planner).
- `crates/*/Cargo.toml`, `bindings/c/Cargo.toml`, `tools/sign-fixture/Cargo.toml` — per-crate
  hard-coded `version` (no workspace inheritance) + internal path-dep pins.
- `C:/Users/OMack/nono-py/Cargo.toml` + `pyproject.toml`; `C:/Users/OMack/nono-ts/package.json`
  — binding manifests (all at 0.66.0). **Separate git repos.**

### PyPI blocker (RLS-12)
- `C:/Users/OMack/nono-py/src/proxy.rs` — `RouteConfig` struct (~line 169) + initializer.
- `C:/Users/OMack/nono-py/src/policy.rs` — `PolicyRouteConfig` → `RustRouteConfig` `From` impl
  (~line 741). Add `endpoint_policy: None` at both construction sites.

### Fork invariants + process
- `CLAUDE.md` — cross-target clippy MUST/NEVER, `--workspace --all-targets` gate, DCO sign-off,
  `make ci` = clippy + fmt + tests.
- `.planning/templates/cross-target-verify-checklist.md` — the two local cross-target clippy gates.
- `proj/ADR-98-network-intent-disposition.md` — the adopt-vs-adapt ADR precedent (pattern for D-05).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- The v3.3 release pipeline (release.yml + release-dry-run.ps1 + release-readiness.ps1) already
  exists and passed at 0.66.0 — Phase 100 re-greens it at 0.66.1, not builds it from scratch.
- Azure Trusted Signing already wired into release.yml (commit `20cd68d9`).

### Established Patterns / Constraints
- **Workspace = 7 members**, each with a hard-coded `version` string (no `version.workspace = true`).
  Version + path-dep pin changes touch every relevant `Cargo.toml` individually
  (see `[[project_workspace_crates]]`).
- **Publish set = 3 crates** (nono → nono-proxy → nono-cli); `nono-shell-broker` is
  `publish = false`; bumping non-published members is for path-dep consistency only (v3.3 97-04).
- **`--workspace --all-targets` is the real local gate** (a `--bin nono` gate hides nono-ffi
  exhaustive-match breaks). `make ci` = clippy + fmt + tests must be clean.
- **Cross-target clippy both-gates GREEN** (Docker `cross` linux-gnu + zig `cargo-zigbuild`
  apple-darwin) — required if any cfg-gated Unix code is touched; version bumps alone usually
  don't, but Cargo.lock regen can pull dep changes — verify.

### Integration Points / Sequencing
- **`nono-py`/`nono-ts` depend on `nono`** — the binding version bump + PyPI validation happen
  in separate repos; a published wheel needs `nono 0.66.1` resolvable. maturin bundles the Rust
  source, so local path/source must be at 0.66.1 before validating the wheel.
- **`cargo publish --dry-run` for downstream crates** exits 101 until `nono` is published
  (they resolve from the live index at package time) — only `nono` dry-runs standalone (v3.3 97-03).
  Plan the dry-run expectations accordingly.
- Folded clean-host UAT items are **host-gated** (need a clean Win11 VM); the poc-cert one is
  additionally **externally blocked** on the Azure Trusted Signing verify-gate fix.

</code_context>

<specifics>
## Specific Ideas

- Version is **exactly `0.66.1`** — not `0.67.x`. Minimal collision-free bump above upstream's
  own `0.66.0` (Phase 98 ledger anchor). Do not skip ahead.
- Preserve fork identity: any org-ref touched during the bump keeps `OscarMackJr/nono` (Cluster E
  precedent). Flag — but don't silently rewrite — the workspace `repository = always-further/nono`
  field; whether to correct it to the fork is a planner decision within RLS-10 reconcile.
- The ADR (D-05) should explicitly state which #1245/#1251 hunks were taken vs dropped and why
  (prepare-only + signed-MSI-order preservation), like ADR-98 named its deviations.

</specifics>

<deferred>
## Deferred Ideas

- **Named future phase — nono-py `endpoint_policy` binding completeness.** Fully thread
  `endpoint_policy` through `nono-py`'s `RouteConfig` so Python callers can set endpoint routing
  end-to-end (beyond the D-07 `None` stub). Reserve as an explicit future phase; not Phase 100.
- **Actual tag push + registry publish (PyPI/npm/crates.io).** The single remaining operator-gated
  step after Phase 100 makes everything push-ready. Outside this phase by design (D-09).
- **Correcting `[workspace.package] repository` to the fork** — if the planner decides it's out
  of RLS-10 scope, capture as a small follow-up rather than expanding Phase 100.

</deferred>

---

*Phase: 100-release-reconcile-leapfrog-0-66-1-pipeline-pypi-blocker*
*Context gathered: 2026-07-01*
