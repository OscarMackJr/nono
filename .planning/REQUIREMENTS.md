# Requirements: nono v3.4 — UPST11 Upstream Sync to v0.66.0 + Release-Reconcile

**Defined:** 2026-06-30
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms. The fork stays current with upstream without regressing its Windows security model — and turns the v3.3 prepare-only pipeline into a genuinely operator-pushable `0.66.1` release.

**Scope:** two pillars — (1) absorb the `nolabs-ai/nono` `v0.65.1..v0.66.0` upstream window (the 19 PRs bundled by release-cut PR [#1293](https://github.com/nolabs-ai/nono/pull/1293)); (2) reconcile the prepare-only release pipeline and leapfrog the crate to `0.66.1` so an operator push is one step away.

> **Architecture invariants:**
> - **Parity gap = `v0.65.1 → v0.66.0`.** Fork upstream high-water mark is `v0.65.1` (UPST10/v3.3). Upstream PR #1293 is a release-cut only; the real content is the 19 PRs it references. A full per-PR divergence ledger is pre-authored in quick task `260629-toe` (`.planning/quick/260629-toe-v066-parity/PLAN.md`) — it is the audit input for Phase 98.
> - **Version collision resolved at `0.66.1`.** The fork is already at crate `0.66.0` (leapfrogged in v3.3); upstream has now also shipped `0.66.0`. The fork's next release bumps to **`0.66.1`** — strictly above upstream's `0.66.0`, collision-free. The fork MUST NOT publish `0.66.0`.
> - **#1225 (introduce `NetworkIntent`, remove `ProxyOnly`) is the highest-conflict item** — the fork has deep `NetworkMode::ProxyOnly` usage across `capability.rs`, `manifest_convert.rs`, `sandbox/linux.rs` and no `NetworkIntent`. The adopt-vs-fork-divergence call is **deferred to the Phase 98 audit (ADR)**, as v3.1/v3.3 did for high-conflict refactors.
> - **Drain-then-sync shape** preserved (mirrors v2.5/v2.6/v3.1/v3.3): the divergence audit is the first phase's deliverable, not pre-roadmap research.
> - **Cross-target clippy is MUST** for the cfg-gated Unix edits this sync touches (#1225, #1207, #1213, #1249); local `cross` (linux-gnu) + `cargo-zigbuild` (apple-darwin) gates must be GREEN — no PARTIAL→CI (retired in v3.3 Phase 96). `make ci` (clippy+fmt+tests), not clippy-only.
> - **Release scope = PREPARE ONLY.** The pipeline is reconciled, dry-run, and gated GREEN locally; the actual `git push` of tags + the live registry publish remain a **manual operator step outside this milestone** (repo forced PUBLIC pending Microsoft minifilter-altitude approval — no `build_notes/`/`.gsd/` staged before any push).
> - **Cross-repo:** `nono-py` (PyPI) at `../nono-py`, `nono-ts` (npm) at `../nono-ts`. A version bump touches all 5 workspace `Cargo.toml` + internal path-dep `version` pins + both binding manifests.

## v1 Requirements

### UPST11 — Upstream Sync (UPST11)

- [x] **UPST11-01**: A DIVERGENCE-LEDGER for the `nolabs-ai/nono` `v0.65.1..v0.66.0` window classifies every commit into will-sync / fork-preserve / won't-sync / split clusters, with a `windows-touch` flag per commit and a per-cell ADR-review verdict (continue/escalate); the #1225 `NetworkIntent`-vs-`ProxyOnly` disposition is settled with an ADR (full-sync-adopt OR fork-divergence carve-out, with rationale).
- [ ] **UPST11-02**: All will-sync feature/fix clusters are absorbed into the fork (cherry-pick with `-x` or manual replay, each commit DCO-signed) without regressing the Windows security model or the policy-free-library boundary — Cluster A full-sync-adopt (ADR-98): 72bcfd66 (#1225 NetworkIntent) + d457ecc3 (#1263 contradictory-flag guard); (tool-sandbox Cluster B: won't-sync — fork lacks tool-sandbox/; carry-forward); proxy (#983 Cluster C split, #1127 Cluster C, #1243 Cluster C), sandbox (#1207 Cluster D), tests (#1213 out-of-filter: tests/ subdir only), per their audited Phase 98 dispositions.
- [ ] **UPST11-03**: The dependency, CI, and documentation clusters are absorbed or reconciled — `sigstore-trust-root` 0.8.0→0.9.0 (#1229 Cluster F, sigstore-rs cascade checked), `criterion` 0.5.1→0.8.2 (#1232) (N/A — nono-cli/Cargo.toml only; out of drift-filter; reconcile separately), CI compile-step mapping fix (#1251) (N/A — CI yaml only; reconcile in Phase 100), proxy docs (#1247 activation) (N/A — crates/nono-cli/data/ dir; out of filter), proxy docs/X-Nono-Token fix (#1246 Cluster G), and the `always-further`→`nolabs-ai` org-rename (#1235 Cluster E) verified N/A-or-applied — with `Cargo.lock` regenerated and the workspace building clean.

**PR→cluster mapping (D-02 reconciliation — full traceability):**

| PR | SHA | Cluster | Disposition | Notes |
|----|-----|---------|-------------|-------|
| PR #1225 | 72bcfd66 | A | IN SCOPE full-sync-adopt | NetworkIntent |
| PR #1263 | d457ecc3 | A | IN SCOPE companion | contradictory-flag guard |
| PR #983 | cdeeb5b9 | C | IN SCOPE split | HTTP/2 pool |
| PR #1127 | 46bcfbb9 | C | IN SCOPE apply | endpoint wiring |
| PR #1243 | 08ca19a8 | C | IN SCOPE apply | wildcard route |
| PR #1207 | 5b8e94da | D | IN SCOPE will-sync | 9P warning |
| PR #1235 | c808f000 | E | IN SCOPE will-sync | org-ref migration |
| PR #1229 | 2e64798d | F | IN SCOPE will-sync | sigstore-trust-root |
| PR #1246 | a4d68189 | G | IN SCOPE will-sync | proxy docs/token fix |
| PR #1268 | 691e0f4f | B | WON'T-SYNC | fork lacks tool-sandbox/ |
| PR #1271 | 7011bc85 | B | WON'T-SYNC | fork lacks tool-sandbox/ |
| PR #1253 | d2252225 | B | WON'T-SYNC | fork lacks tool-sandbox/ |
| PR #1249 | 853d5236 | B | WON'T-SYNC | fork lacks tool-sandbox/ |
| PR #1213 | 30cfee67 | Noise | OUT-OF-FILTER | tests/ subdir only |
| PR #1232 | 5441f4eb | Noise | OUT-OF-FILTER | nono-cli/Cargo.toml only |
| PR #1251 | 84b5e7ce | Noise | OUT-OF-FILTER | CI yaml only |
| PR #1247 | 8aee0e77 | Noise | OUT-OF-FILTER | data/ dir only |
| PR #1293 | d817ed53 | H | WON'T-SYNC | release metadata; Phase 100 leapfrog 0.66.1 |
- [ ] **UPST11-04**: Fork-divergent invariants are explicitly preserved and verified post-sync — local cross-target clippy is GREEN on both Unix gates (`cross clippy` linux-gnu + direct-binary `cargo-zigbuild clippy` apple-darwin, `-D warnings -D clippy::unwrap_used`, no PARTIAL→CI), `make ci` (clippy + fmt + tests) is clean on the dev host, and a code-review + verifier pass confirm no Windows-backend (AppContainer/WFP/broker) or ADR-86 boundary regression.

### Release Reconcile (RLS)

- [ ] **RLS-10**: All 5 workspace crates (`nono`, `nono-cli`, `nono-proxy`, `nono-shell-broker`, `nono-ffi`) plus the `nono-py` / `nono-ts` bindings are version-bumped to **`0.66.1`** (the minimal collision-free bump above upstream's own `0.66.0`), with internal path-dep `version` pins consistent across every `Cargo.toml` and both binding manifests (`Cargo.lock` regenerated; workspace builds clean).
- [ ] **RLS-11**: The upstream CI changes are reconciled against the fork's prepare-only release pipeline — #1245 (idempotent `publish-crates` + cross-compile check on release PRs) and #1251 (compile-step mapping fix) are adopted or adapted without breaking the existing `release-readiness` verify-dark gate or the signed-MSI build order.
- [ ] **RLS-12**: The carried-forward `nono-py` `RouteConfig` PyPI blocker is closed — the missing `endpoint_policy` field is added (`src/policy.rs:743` + `src/proxy.rs:206`) so the `nono-py` wheel builds and `twine check` / maturin validation passes for publish.
- [ ] **RLS-13**: The release is **one-step-push ready at `0.66.1`** — `scripts/release-dry-run.ps1` plus the `release-readiness` gate are re-run GREEN, and `RELEASE-RUNBOOK.md` is updated for the `0.66.1` tag, embedding the PUBLIC-repo pre-push checklist (no `build_notes/`/`.gsd/` staged; crate version `0.66.1` > upstream `0.66.0` confirmed). The actual tag push + registry publish remain operator-gated.

## v2 / Future Requirements

Tracked, not in this roadmap.

- **FUT-01**: Live multi-registry publish executed in-milestone (this milestone is PREPARE-ONLY; the actual push/publish is operator-gated).
- **FUT-02**: Azure Trusted Signing distribution — replace the POC/self-signed cert with publicly-trusted code signing (clean-host broker trust out of the box).
- **FUT-03**: Drain the remaining host-gated distribution todos (POC-cert broker on clean host, MSI VC++ x64 runtime prereq).
- **FUT-04**: Native cross-target clippy run in a hosted CI matrix as the *enforcing* gate (complementary to the local toolchain).
- **FUT-05**: Next upstream-sync cadence past `nolabs-ai/nono` v0.66.0 (anything beyond this window is a future UPST cycle).

## Out of Scope

Explicit exclusions, documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Actually pushing tags / live registry publish | Release scope = PREPARE ONLY; the push + publish are an operator-gated manual step (repo PUBLIC pending Microsoft minifilter altitude). |
| Publishing crate `0.66.0` from the fork | Collides with upstream's own `0.66.0`; the fork bumps to `0.66.1` (RLS-10). |
| Going-private / un-ignoring `build_notes/`/`.gsd/` | Repo MUST stay PUBLIC until Microsoft approves the minifilter altitude; a prior go-private commit was cancelled. |
| New upstream features beyond the `v0.65.1..v0.66.0` window | Sync window is bounded; anything past `v0.66.0` is a future UPST cycle (FUT-05). |
| Publicly-trusted (Azure Trusted Signing) code signing | Deferred (FUT-02); this milestone signs with the existing pipeline cert. |
| Regressing the Windows security model or policy-free boundary to ease a sync | Fork-preserve invariants win any conflict with upstream (UPST11-04). |

## Traceability

Populated by roadmap creation 2026-06-30. Phase numbering continues from Phase 97 → Phase 98+.

| Requirement | Phase | Status |
|-------------|-------|--------|
| UPST11-01 | Phase 98 | Complete |
| UPST11-02 | Phase 99 | Pending |
| UPST11-03 | Phase 99 | Pending |
| UPST11-04 | Phase 99 | Pending |
| RLS-10 | Phase 100 | Pending |
| RLS-11 | Phase 100 | Pending |
| RLS-12 | Phase 100 | Pending |
| RLS-13 | Phase 100 | Pending |
