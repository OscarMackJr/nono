# Requirements: nono v3.3 — UPST10 Upstream Sync (v0.64→v0.65.1) + First Real Release

**Defined:** 2026-06-25
**Core Value:** Windows security must be as structurally impossible and feature-complete as Unix platforms. The fork stays current with upstream without regressing its Windows security model — and is, for the first time, genuinely *releasable*: a gated, signed, multi-registry pipeline prepared GREEN for a one-step operator push.

**Scope:** three pillars — (1) absorb the `nolabs-ai/nono` `v0.64.0..v0.65.1` upstream window; (2) make the workspace + bindings publishable (crate leapfrog ≥ `0.65.0`) with the release pipeline built, signed, dry-run, and gated GREEN; (3) stand up a local cross C toolchain to retire the cross-target clippy PARTIAL→CI debt.

> **Architecture invariants:**
> - **Release scope = PREPARE ONLY.** The pipeline is built, signed, dry-run, and gated GREEN locally; the actual `git push` of tags + the live registry publish remain a **manual operator step outside this milestone**. This preserves the LOCAL-ONLY posture (repo forced PUBLIC pending Microsoft minifilter-altitude approval — no `build_notes/`/`.gsd/` staged before any push).
> - **Drain-then-sync shape** preserved (mirrors v2.5/v2.6/v3.1): the divergence audit is the first phase's deliverable, not pre-roadmap research.
> - **Upstream relocated:** canonical upstream is now `nolabs-ai/nono` (was `always-further/nono`), latest tag `v0.65.1` (2026-06-23). Fork high-water mark is `v0.64.0` (UPST9/v3.1) → sync window `v0.64.0..v0.65.1` (v0.64.1, v0.65.0, v0.65.1).
> - **Cross-repo:** `nono-py` (PyPI) at `../nono-py`, `nono-ts` (npm) at `../nono-ts`. A version bump touches all 5 workspace `Cargo.toml` + internal path-dep `version` pins + both binding manifests.

## v1 Requirements

### UPST10 — Upstream Sync (UPST10)

- [x] **UPST10-01**: A DIVERGENCE-LEDGER for the `nolabs-ai/nono` `v0.64.0..v0.65.1` window classifies every commit into will-sync / fork-preserve / won't-sync / split clusters, with a `windows-touch` flag per commit and a per-cell ADR-review verdict (continue/escalate).
- [x] **UPST10-02**: All will-sync clusters are absorbed into the fork (cherry-pick with `-x` or manual replay), each commit DCO-signed, without regressing the Windows security model.
- [x] **UPST10-03**: Fork-divergent invariants are explicitly preserved and verified post-sync — the Windows backend (AppContainer/WFP/broker), the ADR-86 audit/diagnostics library-boundary carve-out, and the `exec_strategy_windows/` denial-rendering fork — with workspace `make build` + `make test` green on the dev host.
- [x] **UPST10-04**: The upstream relocation (`always-further/nono` → `nolabs-ai/nono`) is recorded — the git `upstream` remote and the PROJECT.md `## Upstream Parity Process` references point at the new canonical source; a Future Cycles stub notes the next sync trigger.

### Release Engineering — First Real Release (RLS)

- [x] **RLS-05**: All 5 workspace crates (`nono`, `nono-cli`, `nono-proxy`, `nono-shell-broker`, `nono-ffi`) plus the `nono-py` / `nono-ts` bindings are version-bumped to a leapfrogged ≥ `0.65.0` release version, with internal path-dep `version` pins consistent across every `Cargo.toml` and both binding manifests (`Cargo.lock` regenerated; workspace builds clean).
- [x] **RLS-06**: The release pipeline builds and signs all release artifacts reproducibly from a single tag — workspace binaries, signed Windows machine + user MSIs (payload signed before WiX harvest, admin-extract verify gate), `nono-py` wheels, and `nono-ts` native packages.
- [x] **RLS-07**: The pipeline runs a **dry-run** publish to crates.io (`cargo publish --dry-run` across the dependency-ordered workspace), PyPI (`twine check` / maturin build validation), and npm (`npm publish --dry-run`) that validates packaging + metadata WITHOUT pushing, and is gated GREEN.
- [x] **RLS-08**: `release.yml` produces (or dry-run-validates) a GitHub Release carrying the signed MSI + binary assets, with no `0s startup_failure` and all required build legs green.
- [x] **RLS-09**: The release is **one-step-push ready** — a documented operator runbook plus a green release-readiness gate confirm the only remaining action is the manual `git push` of tags + `cargo/twine/npm publish`, and the runbook embeds the PUBLIC-repo pre-push checklist (no `build_notes/`/`.gsd/` staged; crate leapfrog ≥ `0.65.0` confirmed).

### Cross-Target Toolchain (XTGT)

- [x] **XTGT-01**: A local cross C toolchain (cross/Docker or equivalent) is installed on the dev host and documented (setup steps + invocation) so cfg-gated Unix code compiles locally.
- [x] **XTGT-02**: `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings -D clippy::unwrap_used` runs locally and passes; any drift surfaced in cfg-gated Unix code is fixed in-milestone.
- [x] **XTGT-03**: `cargo clippy --workspace --target x86_64-apple-darwin -- -D warnings -D clippy::unwrap_used` runs locally and passes — OR a documented hard blocker (osxcross/macOS-SDK infeasibility from the Windows host) is recorded and apple-darwin stays explicitly PARTIAL→CI with rationale.
- [x] **XTGT-04**: The CLAUDE.md cross-target verification protocol + `.planning/templates/cross-target-verify-checklist.md` are updated to reflect local-runnable status, retiring the PARTIAL→CI *default* for the gate(s) now provably runnable on the host.

## v2 / Future Requirements

Tracked, not in this roadmap.

- **FUT-01**: Live multi-registry publish executed in-milestone (this milestone is PREPARE-ONLY; the actual push/publish is operator-gated).
- **FUT-02**: Azure Trusted Signing distribution — replace the POC/self-signed cert with publicly-trusted code signing (clean-host broker trust out of the box).
- **FUT-03**: Drain the remaining host-gated distribution todos (POC-cert broker on clean host, MSI VC++ x64 runtime prereq).
- **FUT-04**: Native cross-target clippy run in a hosted CI matrix as the *enforcing* gate (complementary to the local toolchain).

## Out of Scope

Explicit exclusions, documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Actually pushing tags / live registry publish | Release scope = PREPARE ONLY; the push + publish are an operator-gated manual step (repo PUBLIC pending Microsoft minifilter altitude). |
| Going-private / un-ignoring `build_notes/`/`.gsd/` | Repo MUST stay PUBLIC until Microsoft approves the minifilter altitude; a prior go-private commit was cancelled. |
| New upstream features beyond the `v0.64.0..v0.65.1` window | Sync window is bounded; anything past `v0.65.1` is a future UPST cycle. |
| Publicly-trusted (Azure Trusted Signing) code signing | Deferred (FUT-02); this milestone signs with the existing pipeline cert. |
| Regressing the Windows security model to ease a sync | Fork-preserve invariants win any conflict with upstream (UPST10-03). |
| Hosted-CI cross-target run as the decisive gate | This milestone makes the gate *local-runnable* (XTGT); CI enforcement is complementary future work (FUT-04). |

## Traceability

Populated by roadmap creation 2026-06-25. Phase numbering continues from Phase 93 → Phase 94+.

| Requirement | Phase | Status |
|-------------|-------|--------|
| UPST10-01 | Phase 94 | Complete |
| UPST10-04 | Phase 94 | Complete |
| UPST10-02 | Phase 95 | Complete |
| UPST10-03 | Phase 95 | Complete |
| XTGT-01 | Phase 96 | Complete |
| XTGT-02 | Phase 96 | Complete |
| XTGT-03 | Phase 96 | Complete |
| XTGT-04 | Phase 96 | Complete |
| RLS-05 | Phase 97 | Complete |
| RLS-06 | Phase 97 | Complete |
| RLS-07 | Phase 97 | Complete |
| RLS-08 | Phase 97 | Complete |
| RLS-09 | Phase 97 | Complete |
